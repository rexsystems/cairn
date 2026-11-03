package app.cairn.client;

import java.awt.image.BufferedImage;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import java.util.regex.Pattern;
import javax.imageio.ImageIO;

/**
 * Talks to a self-hosted Cairn cosmetics server and keeps what nearby players wear.
 * All network and image work runs on one daemon thread; render threads only read the maps.
 */
public final class Cosmetics {
    public static final String MOJANG_JOIN = "https://sessionserver.mojang.com/session/minecraft/join";
    private static final Pattern ID = Pattern.compile("[a-z0-9_-]{1,32}");
    private static final Pattern CAPE_URL = Pattern.compile("/v1/capes/([0-9a-f]{64})\\.png");
    private static final Pattern UUID_HEX = Pattern.compile("[0-9a-f]{32}");
    private static final long LOOK_TTL = 120_000, MISS_TTL = 300_000, HEARTBEAT = 60_000;
    private static final int MAX_BATCH = 100, MAX_FRAMES = 32;

    /** What one player wears, as the server reported it. */
    public static final class Look {
        public final String uuid, name, role, badge, cape, capeUrl;
        public final int frames, frameMs;
        Look(String uuid, String name, String role, String badge, String cape, String capeUrl, int frames, int frameMs) {
            this.uuid = uuid; this.name = name; this.role = role; this.badge = badge; this.cape = cape; this.capeUrl = capeUrl;
            this.frames = Math.max(1, Math.min(MAX_FRAMES, frames)); this.frameMs = Math.max(50, frameMs);
        }
        public boolean staff() { return "staff".equals(role) || "admin".equals(role); }
    }
    /** A catalog cape or badge bundled with the mod. */
    public static final class Item {
        public final String id, name, role;
        public final int frames, frameMs;
        Item(String id, String name, String role, int frames, int frameMs) { this.id = id; this.name = name; this.role = role; this.frames = frames; this.frameMs = frameMs; }
    }
    private static final class Entry {
        final Look look; final long expires;
        Entry(Look look, long expires) { this.look = look; this.expires = expires; }
    }
    static final class Reply {
        final int code; final String body;
        Reply(int code, String body) { this.code = code; this.body = body; }
        Map<String, Object> json() { try { return Json.object(Json.parse(body)); } catch (IllegalArgumentException e) { return Collections.emptyMap(); } }
        String error() { return Json.string(json().get("error"), "HTTP " + code); }
    }

    private final CairnClient client;
    private final String base, urlProblem;
    private final List<Item> capes = new ArrayList<Item>(), badges = new ArrayList<Item>();
    private final Map<String, Entry> looks = new ConcurrentHashMap<String, Entry>();
    private final Set<String> pending = Collections.newSetFromMap(new ConcurrentHashMap<String, Boolean>());
    private final Map<String, byte[]> textures = new ConcurrentHashMap<String, byte[]>();
    private final Map<String, Integer> capeFrames = new ConcurrentHashMap<String, Integer>();
    private final Set<String> loading = Collections.newSetFromMap(new ConcurrentHashMap<String, Boolean>());
    private ScheduledExecutorService worker;
    private volatile String name, uuid, accessToken, token, role = "player", status;
    private volatile String equippedCape, equippedBadge, customHash, customStatus, moderation;
    private volatile int customFrames = 1, customFrameMs = 100, online = -1, peak = -1, total = -1, failures;
    private volatile Set<String> unlockedCapes = Collections.emptySet(), unlockedBadges = Collections.emptySet();
    private volatile long lookupsAfter, nextLookup, nextStats, lastBeat;
    private volatile boolean busy;
    private final java.util.concurrent.atomic.AtomicBoolean beatQueued = new java.util.concurrent.atomic.AtomicBoolean();
    private long generation;

    Cosmetics(CairnClient client) {
        this.client = client;
        String raw = System.getProperty("cairn.cosmetics.url");
        if (blank(raw)) raw = System.getenv("CAIRN_COSMETICS_URL");
        if (blank(raw)) raw = client.option("cosmetics.url", "");
        String problem = null, url = null;
        if (!blank(raw)) {
            url = normalizeUrl(raw);
            if (url == null) problem = "Cosmetics server URL must be https:// (http:// only for localhost).";
        }
        base = url; urlProblem = problem;
        loadCatalog();
        status = base != null ? "Waiting for the game to sign in" : problem != null ? problem : "No cosmetics server set. Add one in the Cairn launcher settings.";
    }

    /** Accepts https anywhere and plain http only on loopback, so session tokens never cross a network unencrypted. */
    static String normalizeUrl(String raw) {
        try {
            URI uri = new URI(raw.trim());
            String scheme = uri.getScheme(), host = uri.getHost();
            if (host == null || uri.getUserInfo() != null || uri.getQuery() != null || uri.getFragment() != null) return null;
            boolean loopback = host.equals("localhost") || host.equals("127.0.0.1") || host.equals("[::1]") || host.equals("::1");
            if (!"https".equalsIgnoreCase(scheme) && !("http".equalsIgnoreCase(scheme) && loopback)) return null;
            String text = uri.toString();
            while (text.endsWith("/")) text = text.substring(0, text.length() - 1);
            return text;
        } catch (URISyntaxException e) { return null; }
    }
    private static boolean blank(String value) { return value == null || value.trim().isEmpty(); }
    static String compactUuid(String raw) {
        if (raw == null) return null;
        String value = raw.replace("-", "").toLowerCase(Locale.ROOT);
        return UUID_HEX.matcher(value).matches() ? value : null;
    }

    private void loadCatalog() {
        try (InputStream stream = Cosmetics.class.getResourceAsStream("/assets/cairn_client/cosmetics/catalog.json")) {
            if (stream == null) return;
            Map<String, Object> catalog = Json.object(Json.parse(new String(readAll(stream, 1 << 20), StandardCharsets.UTF_8)));
            for (Object raw : Json.list(catalog.get("capes"))) {
                Map<String, Object> cape = Json.object(raw);
                String id = Json.string(cape.get("id"), "");
                if (ID.matcher(id).matches()) capes.add(new Item(id, Json.string(cape.get("name"), id), Json.string(cape.get("role"), null), Json.integer(cape.get("frames"), 1), Json.integer(cape.get("frameMs"), 0)));
            }
            for (Object raw : Json.list(catalog.get("badges"))) {
                Map<String, Object> badge = Json.object(raw);
                String id = Json.string(badge.get("id"), "");
                if (ID.matcher(id).matches()) badges.add(new Item(id, Json.string(badge.get("name"), id), Json.string(badge.get("role"), null), 1, 0));
            }
        } catch (IOException | IllegalArgumentException e) { System.err.println("[Cairn Client] Could not read the cosmetics catalog: " + e); }
    }

    public boolean enabled() { return base != null; }
    public String server() { return base; }
    public String status() { return status; }
    public boolean signedIn() { return token != null; }
    String token() { return token; }
    String uuid() { return uuid; }
    /** Another part of the client got a 401; sign in again on the current session. */
    void expired() {
        if (token == null) return;
        token = null;
        final long session;
        synchronized (this) { session = generation; }
        run(() -> signIn(session), 0);
    }
    public boolean busy() { return busy; }
    public String role() { return role; }
    public int online() { return online; }
    public int peak() { return peak; }
    public int total() { return total; }
    public String equippedCape() { return equippedCape; }
    public String equippedBadge() { return equippedBadge; }
    public String customStatus() { return customStatus; }
    public String customHash() { return customHash; }
    public List<Item> capes() { return Collections.unmodifiableList(capes); }
    public List<Item> badges() { return Collections.unmodifiableList(badges); }
    public boolean visible() { return !"false".equals(client.option("cosmetics.show", "true")); }
    public void visible(boolean show) { client.setOption("cosmetics.show", String.valueOf(show)); }
    /** Without a session the player hasn't unlocked anything, but public items still preview. */
    public boolean unlocked(Item item, boolean cape) {
        if (token == null) return item.role == null;
        return (cape ? unlockedCapes : unlockedBadges).contains(item.id);
    }

    private synchronized ScheduledExecutorService worker() {
        if (worker == null) {
            worker = Executors.newSingleThreadScheduledExecutor(task -> {
                Thread thread = new Thread(task, "Cairn cosmetics");
                thread.setDaemon(true);
                return thread;
            });
        }
        return worker;
    }
    private void run(Runnable task, long delayMs) {
        if (base == null) return;
        background(task, delayMs);
    }
    private void background(Runnable task, long delayMs) {
        worker().schedule(() -> {
            try { task.run(); } catch (RuntimeException e) { System.err.println("[Cairn Client] Cosmetics task failed: " + e); }
        }, delayMs, TimeUnit.MILLISECONDS);
    }

    /** Adapters call this every tick; it only acts when the signed-in account changes. */
    public void account(String name, String rawUuid, String accessToken) {
        String id = compactUuid(rawUuid);
        if (base == null || name == null || id == null) return;
        if (name.equals(this.name) && id.equals(this.uuid) && Objects.equals(accessToken, this.accessToken)) return;
        this.name = name; this.uuid = id; this.accessToken = accessToken; token = null;
        final long session;
        synchronized (this) { session = ++generation; }
        run(() -> signIn(session), 0);
    }
    /** Retry now, for the menu's Refresh button. */
    public void refresh() {
        failures = 0; lookupsAfter = 0;
        looks.clear();
        final long session;
        synchronized (this) { session = ++generation; }
        if (name != null) run(() -> signIn(session), 0);
        run(this::stats, 0);
    }
    private boolean current(long session) { synchronized (this) { return session == generation; } }
    private long backoff() { return Math.min(600_000L, 5_000L << Math.min(7, failures)); }

    private void signIn(long session) {
        if (!current(session)) return;
        try {
            Reply start = post("/v1/auth/start", null, "{\"name\":" + Json.quote(name) + "}");
            if (start.code != 200) { fail(session, "Cosmetics sign-in failed: " + start.error(), start.code >= 500); return; }
            String serverId = Json.string(start.json().get("serverId"), "");
            // The game token only ever goes to Mojang; the cosmetics server just asks Mojang whether we joined.
            if (realToken(accessToken)) {
                String body = "{\"accessToken\":" + Json.quote(accessToken) + ",\"selectedProfile\":" + Json.quote(uuid) + ",\"serverId\":" + Json.quote(serverId) + "}";
                Reply joined = http("POST", MOJANG_JOIN, null, "application/json", body.getBytes(StandardCharsets.UTF_8), 1 << 16);
                if (joined.code != 204 && joined.code != 200) { fail(session, "Mojang refused the sign-in (HTTP " + joined.code + "). Cosmetics stay visible.", false); return; }
            }
            Reply finish = post("/v1/auth/finish", null, "{\"name\":" + Json.quote(name) + ",\"serverId\":" + Json.quote(serverId) + "}");
            if (finish.code != 200) {
                String reason = finish.code == 401 || finish.code == 403 ? "Offline accounts can see cosmetics but not wear them." : "Cosmetics sign-in failed: " + finish.error();
                fail(session, reason, finish.code >= 500 || finish.code == 429);
                return;
            }
            Map<String, Object> reply = finish.json();
            if (!current(session)) return;
            token = Json.string(reply.get("token"), null);
            role = Json.string(reply.get("role"), "player");
            failures = 0;
            status = "Signed in as " + name + ("player".equals(role) ? "" : " (" + role + ")");
            long expires = Math.max(300, Json.whole(reply.get("expiresIn"), 3600)) * 1000;
            run(() -> signIn(session), expires - 120_000);
            loadMe();
            heartbeat(session);
            client.friends.signedIn();
        } catch (IOException e) { fail(session, "Cosmetics server unreachable. Retrying in " + backoff() / 1000 + " s.", true); }
    }
    static boolean realToken(String token) {
        // Offline profiles carry placeholders like "0"; sending those to Mojang only earns a 403.
        return token != null && token.length() >= 32 && !token.matches("[0-9]+");
    }
    private void fail(long session, String reason, boolean retry) {
        if (!current(session)) return;
        status = reason;
        token = null;
        if (retry) { long delay = backoff(); failures++; run(() -> signIn(session), delay); }
    }
    private void heartbeat(long session) {
        if (!current(session) || token == null) return;
        if (beat(session) == 401) return;
        stats();
        run(() -> heartbeat(session), HEARTBEAT);
    }
    /** The only place presence comes from: what this game is doing, sent while it runs. */
    private int beat(long session) {
        String bearer = token;
        if (bearer == null) return 0;
        lastBeat = System.currentTimeMillis();
        try {
            Reply reply = post("/v1/heartbeat", bearer, client.friends.activityJson());
            if (reply.code == 401) { token = null; run(() -> signIn(session), 0); }
            else if (reply.code == 200) online = Json.integer(reply.json().get("online"), online);
            return reply.code;
        } catch (IOException e) { return 0; }
    }
    /** Activity changed: tell friends now instead of at the next minute, at most every 5 s. */
    void beatSoon() {
        if (base == null || token == null || !beatQueued.compareAndSet(false, true)) return;
        final long session;
        synchronized (this) { session = generation; }
        long wait = Math.max(0, lastBeat + 5000 - System.currentTimeMillis());
        run(() -> { beatQueued.set(false); if (current(session)) beat(session); }, wait);
    }
    private void stats() {
        try {
            Reply reply = http("GET", base + "/v1/stats", null, null, null, 1 << 14);
            if (reply.code != 200) return;
            Map<String, Object> stats = reply.json();
            online = Json.integer(stats.get("online"), online);
            peak = Json.integer(stats.get("peak"), peak);
            total = Json.integer(stats.get("total"), total);
        } catch (IOException ignored) { }
    }
    /** The menu asks for stats while open; this keeps them from going stale without a session. */
    public void wantStats() {
        long now = System.currentTimeMillis();
        if (base == null || now < nextStats) return;
        nextStats = now + HEARTBEAT;
        run(this::stats, 0);
    }
    private void loadMe() throws IOException {
        Reply reply = http("GET", base + "/v1/me", token, null, null, 1 << 16);
        if (reply.code != 200) return;
        Map<String, Object> me = reply.json();
        Map<String, Object> equipped = Json.object(me.get("equipped"));
        equippedCape = Json.string(equipped.get("cape"), null);
        equippedBadge = Json.string(equipped.get("badge"), null);
        Map<String, Object> unlocked = Json.object(me.get("unlocked"));
        unlockedCapes = ids(unlocked.get("capes"));
        unlockedBadges = ids(unlocked.get("badges"));
        moderation = Json.string(me.get("moderation"), "approval");
        Map<String, Object> custom = Json.object(me.get("customCape"));
        java.util.regex.Matcher url = CAPE_URL.matcher(Json.string(custom.get("url"), ""));
        customHash = url.matches() ? url.group(1) : null;
        customStatus = customHash == null ? null : Json.string(custom.get("status"), "pending");
        customFrames = Json.integer(custom.get("frames"), 1);
        customFrameMs = Json.integer(custom.get("frameMs"), 100);
        looks.remove(uuid);
    }
    private static Set<String> ids(Object raw) {
        Set<String> out = new HashSet<String>();
        for (Object value : Json.list(raw)) if (value instanceof String) out.add((String) value);
        return Collections.unmodifiableSet(out);
    }

    /** Equips a cape ("custom", a catalog id or null) and badge ("none", an id or null) in one request, as the server expects. */
    public void equip(String cape, String badge) {
        if (token == null) { status = "Sign in with a Microsoft account to change cosmetics."; return; }
        busy = true;
        status = "Saving...";
        final long session;
        synchronized (this) { session = generation; }
        run(() -> {
            try {
                String body = "{\"cape\":" + (cape == null ? "null" : Json.quote(cape)) + ",\"badge\":" + (badge == null ? "null" : Json.quote(badge)) + "}";
                Reply reply = http("PUT", base + "/v1/me/equipped", token, "application/json", body.getBytes(StandardCharsets.UTF_8), 1 << 14);
                if (reply.code == 200) { equippedCape = cape; equippedBadge = badge; looks.remove(uuid); status = "Saved. Other players see it within a minute."; loadMe(); }
                else if (reply.code == 401) { status = "Session expired, signing in again."; token = null; run(() -> signIn(session), 0); }
                else status = reply.error();
            } catch (IOException e) { status = "Could not reach the cosmetics server."; }
            busy = false;
        }, 0);
    }

    /** The cosmetics for one player, or null; unknown players are queued for the next batch lookup. */
    public Look look(String rawUuid) {
        if (base == null || !visible()) return null;
        String id = compactUuid(rawUuid);
        if (id == null) return null;
        Entry entry = looks.get(id);
        long now = System.currentTimeMillis();
        if (entry == null || entry.expires < now) pending.add(id);
        return entry == null ? null : entry.look;
    }
    /** Sends queued lookups; adapters call this once per tick and it rate-limits itself. */
    public void tick() {
        long now = System.currentTimeMillis();
        if (base == null || pending.isEmpty() || now < nextLookup || now < lookupsAfter) return;
        nextLookup = now + 1000;
        final List<String> batch = new ArrayList<String>();
        for (Iterator<String> it = pending.iterator(); it.hasNext() && batch.size() < MAX_BATCH; ) { batch.add(it.next()); it.remove(); }
        run(() -> lookup(batch), 0);
    }
    private void lookup(List<String> batch) {
        StringBuilder query = new StringBuilder();
        for (String id : batch) query.append(query.length() == 0 ? "" : ",").append(id);
        try {
            Reply reply = http("GET", base + "/v1/players?uuids=" + query, null, null, null, 1 << 20);
            if (reply.code != 200) throw new IOException("HTTP " + reply.code);
            long now = System.currentTimeMillis();
            Map<String, Object> players = Json.object(reply.json().get("players"));
            for (String id : batch) {
                Look look = parseLook(id, Json.object(players.get(id)));
                looks.put(id, new Entry(look, now + (look == null ? MISS_TTL : LOOK_TTL)));
            }
            lookupsAfter = 0;
        } catch (IOException e) {
            lookupsAfter = System.currentTimeMillis() + backoff();
            pending.addAll(batch);
        }
    }
    static Look parseLook(String uuid, Map<String, Object> player) {
        if (player.isEmpty()) return null;
        String badge = Json.string(player.get("badge"), "none");
        if (!ID.matcher(badge).matches()) badge = "none";
        Map<String, Object> cape = Json.object(player.get("cape"));
        String capeId = Json.string(cape.get("id"), null), url = null;
        if ("custom".equals(capeId)) {
            java.util.regex.Matcher match = CAPE_URL.matcher(Json.string(cape.get("url"), ""));
            if (match.matches()) { capeId = "custom-" + match.group(1); url = match.group(); }
            else capeId = null;
        } else if (capeId != null && !ID.matcher(capeId).matches()) capeId = null;
        return new Look(uuid, Json.string(player.get("name"), ""), Json.string(player.get("role"), "player"), badge, capeId, url,
            Json.integer(cape.get("frames"), 1), Json.integer(cape.get("frameMs"), 100));
    }

    /** Texture key of the cape frame to draw now, or null until its frames are loaded. */
    public String capeTexture(Look look) {
        if (look == null || look.cape == null) return null;
        return frameKey(look.cape, look.capeUrl, look.frames, look.frameMs, System.currentTimeMillis());
    }
    /** Same, for a catalog item or the player's own upload shown in the menu. */
    public String previewTexture(String cape, long now) {
        if (cape == null) return null;
        if (cape.equals("custom")) return customHash == null ? null : frameKey("custom-" + customHash, "/v1/capes/" + customHash + ".png", customFrames, customFrameMs, now);
        for (Item item : capes) if (item.id.equals(cape)) return frameKey(item.id, null, item.frames, item.frameMs, now);
        return null;
    }
    private String frameKey(String cape, String url, int frames, int frameMs, long now) {
        Integer loaded = capeFrames.get(cape);
        if (loaded == null) { loadCape(cape, url, frames); return null; }
        return "cape/" + cape + "/" + frame(now, loaded, frameMs);
    }
    /** Animated capes use wall-clock time so every client shows the same frame. */
    static int frame(long now, int frames, int frameMs) { return frames <= 1 ? 0 : (int) (now / Math.max(50, frameMs) % frames); }
    public String badgeTexture(Look look) { return look == null ? null : badgeTexture(look.badge); }
    public String badgeTexture(String badge) {
        if (badge == null || badge.equals("none") || !ID.matcher(badge).matches()) return null;
        String key = "badge/" + badge;
        if (textures.containsKey(key)) return key;
        if (loading.add(key)) {
            Runnable task = () -> {
                try (InputStream stream = Cosmetics.class.getResourceAsStream("/assets/cairn_client/cosmetics/badges/" + badge + ".png")) {
                    if (stream != null) textures.put(key, readAll(stream, 1 << 20));
                } catch (IOException ignored) { }
            };
            // Animated strips take a moment to decode, so even bundled ones stay off the render thread.
        background(task, 0);
        }
        return null;
    }
    /** PNG bytes for a texture key; adapters upload them once and keep their own handle. */
    public byte[] texture(String key) { return textures.get(key); }

    private void loadCape(String cape, String url, int frames) {
        if (!loading.add("cape/" + cape)) return;
        Runnable task = () -> {
            try {
                byte[] png;
                if (url != null) {
                    if (base == null) return;
                    png = download(base + url);
                } else {
                    try (InputStream stream = Cosmetics.class.getResourceAsStream("/assets/cairn_client/cosmetics/capes/" + cape + ".png")) {
                        if (stream == null) return;
                        png = readAll(stream, 4 << 20);
                    }
                }
                List<byte[]> split = split(png, frames);
                for (int i = 0; i < split.size(); i++) textures.put("cape/" + cape + "/" + i, split.get(i));
                if (!split.isEmpty()) capeFrames.put(cape, split.size());
            } catch (IOException e) {
                System.err.println("[Cairn Client] Could not load cape " + cape + ": " + e);
            }
        };
        // Animated strips take a moment to decode, so even bundled ones stay off the render thread.
        background(task, 0);
    }
    /** Cuts a vertical strip of 2:1 frames into separate PNGs, so every game version can upload them as plain textures. */
    static List<byte[]> split(byte[] png, int frames) throws IOException {
        BufferedImage image = ImageIO.read(new ByteArrayInputStream(png));
        if (image == null) throw new IOException("Not an image");
        int width = image.getWidth(), frameHeight = width / 2;
        if (width < 64 || width > 1024 || width % 64 != 0 || image.getHeight() % frameHeight != 0) throw new IOException("Unexpected cape size " + width + "x" + image.getHeight());
        int count = Math.max(1, Math.min(Math.min(MAX_FRAMES, frames), image.getHeight() / frameHeight));
        List<byte[]> out = new ArrayList<byte[]>();
        for (int i = 0; i < count; i++) {
            BufferedImage frame = new BufferedImage(width, frameHeight, BufferedImage.TYPE_INT_ARGB);
            frame.getGraphics().drawImage(image.getSubimage(0, i * frameHeight, width, frameHeight), 0, 0, null);
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            ImageIO.write(frame, "png", bytes);
            out.add(bytes.toByteArray());
        }
        return out;
    }
    private byte[] download(String url) throws IOException {
        HttpURLConnection connection = open("GET", url, null);
        try {
            if (connection.getResponseCode() != 200) throw new IOException("HTTP " + connection.getResponseCode());
            try (InputStream stream = connection.getInputStream()) { return readAll(stream, 2 << 20); }
        } finally { connection.disconnect(); }
    }

    private Reply post(String path, String bearer, String json) throws IOException {
        return http("POST", base + path, bearer, "application/json", json.getBytes(StandardCharsets.UTF_8), 1 << 16);
    }
    private static HttpURLConnection open(String method, String url, String bearer) throws IOException { return open(method, url, bearer, 8000); }
    private static HttpURLConnection open(String method, String url, String bearer, int readTimeout) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        // A redirect could carry the bearer token to another host, so none are followed.
        connection.setInstanceFollowRedirects(false);
        connection.setConnectTimeout(5000);
        connection.setReadTimeout(readTimeout);
        connection.setRequestMethod(method);
        connection.setRequestProperty("User-Agent", "CairnClient");
        connection.setRequestProperty("Accept", "application/json");
        if (bearer != null) connection.setRequestProperty("Authorization", "Bearer " + bearer);
        return connection;
    }
    static Reply http(String method, String url, String bearer, String type, byte[] body, int limit) throws IOException {
        return http(method, url, bearer, type, body, limit, 8000);
    }
    static Reply http(String method, String url, String bearer, String type, byte[] body, int limit, int readTimeout) throws IOException {
        HttpURLConnection connection = open(method, url, bearer, readTimeout);
        try {
            if (body != null) {
                connection.setDoOutput(true);
                connection.setRequestProperty("Content-Type", type);
                connection.setFixedLengthStreamingMode(body.length);
                try (OutputStream out = connection.getOutputStream()) { out.write(body); }
            }
            int code = connection.getResponseCode();
            InputStream stream = code >= 400 ? connection.getErrorStream() : connection.getInputStream();
            if (stream == null) return new Reply(code, "");
            try (InputStream in = stream) { return new Reply(code, new String(readAll(in, limit), StandardCharsets.UTF_8)); }
        } finally { connection.disconnect(); }
    }
    static byte[] readAll(InputStream stream, int limit) throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        for (int read; (read = stream.read(buffer)) > 0; ) {
            if (out.size() + read > limit) throw new IOException("Response larger than " + limit + " bytes");
            out.write(buffer, 0, read);
        }
        return out.toByteArray();
    }
}
