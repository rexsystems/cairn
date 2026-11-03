package app.cairn.client;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import java.util.function.Consumer;
import java.util.regex.Pattern;

/**
 * Cairn friends: the list, requests, privacy and toasts. Presence is fed only by this mod's heartbeat,
 * so the server never knows what a player does outside Cairn Client.
 */
public final class Friends {
    public static final String[] TOASTS = { "online", "server", "request", "accepted" };
    public static final String[] TOAST_LABELS = { "Came online", "Joined a server", "Friend requests", "Accepted" };
    private static final Pattern TARGET = Pattern.compile("[A-Za-z0-9_]{1,16}|[0-9a-fA-F-]{32,36}");
    private static final Pattern SERVER = Pattern.compile("[A-Za-z0-9._:\\[\\]-]{1,255}");
    private static final long TOAST_MS = 5000, LIST_EVERY = 30_000, WAIT_SECONDS = 15;
    private static final int MAX_TOASTS = 3;

    public static final class Friend {
        public final String uuid, name, status, version, mode, server;
        Friend(String uuid, String name, String status, String version, String mode, String server) {
            this.uuid = uuid; this.name = name; this.status = status; this.version = version; this.mode = mode; this.server = server;
        }
        public boolean online() { return !"offline".equals(status); }
        /** One line for the list, e.g. "Playing 1.21.11 on mc.example.net". */
        public String activity() {
            if (!online()) return "Offline";
            String prefix = "dnd".equals(status) ? "Do not disturb / " : "";
            if (mode == null) return prefix + "Online";
            String version = this.version == null ? "" : " " + this.version;
            if ("multiplayer".equals(mode)) return prefix + "Multiplayer" + version + (server == null ? "" : " on " + server);
            if ("singleplayer".equals(mode)) return prefix + "Singleplayer" + version;
            return prefix + "In the menus" + version;
        }
    }
    public static final class Person {
        public final String uuid, name;
        Person(String uuid, String name) { this.uuid = uuid; this.name = name; }
    }
    public static final class Toast {
        public final String title, text;
        long shown;
        Toast(String title, String text) { this.title = title; this.text = text; }
    }

    private final CairnClient client;
    private final Cosmetics cosmetics;
    private final Deque<Toast> toasts = new ConcurrentLinkedDeque<Toast>();
    private volatile List<Friend> friends = Collections.emptyList();
    private volatile List<Person> incoming = Collections.emptyList(), outgoing = Collections.emptyList(), blocked = Collections.emptyList();
    private volatile String share = "full", requests = "everyone", message, loadError, version, mode, server, sentActivity = "";
    private volatile boolean appearOffline, loaded, busy;
    private volatile long nextList, listAfter, messageAt;
    private volatile Consumer<String> join;
    private ExecutorService worker;
    private Thread events;

    Friends(CairnClient client, Cosmetics cosmetics) { this.client = client; this.cosmetics = cosmetics; }

    public List<Friend> friends() { return friends; }
    public List<Person> incoming() { return incoming; }
    public List<Person> outgoing() { return outgoing; }
    public List<Person> blocked() { return blocked; }
    public String share() { return share; }
    public String requests() { return requests; }
    public boolean appearOffline() { return appearOffline; }
    public boolean loaded() { return loaded; }
    public boolean busy() { return busy; }
    /** DND is cached locally so toasts stop at once, even before the server answers. */
    public boolean dnd() { return "true".equals(client.option("friends.dnd", "false")); }
    public boolean toast(String type) { return !"false".equals(client.option("friends.toast." + type, "true")); }
    public void toast(String type, boolean on) { client.setOption("friends.toast." + type, String.valueOf(on)); }
    /** Action results stay for a few seconds; after that the list's own state shows. */
    public String message() {
        if (message != null && System.currentTimeMillis() - messageAt < 8000) return message;
        if (loadError != null) return loadError;
        if (!cosmetics.enabled()) return "Friends need a Cairn server. Set one in the Cairn launcher.";
        if (!cosmetics.signedIn()) return cosmetics.status();
        return loaded ? friends.size() + (friends.size() == 1 ? " friend" : " friends") : "Loading friends...";
    }
    private void note(String text) { message = text; messageAt = System.currentTimeMillis(); }
    public boolean available() { return cosmetics.enabled() && cosmetics.signedIn(); }

    /** Adapters set this where the game can connect to a server from the menu. */
    public void joinHandler(Consumer<String> handler) { join = handler; }
    public boolean canJoin(Friend friend) { return join != null && friend.server != null && SERVER.matcher(friend.server).matches(); }
    public void join(Friend friend) { if (canJoin(friend)) join.accept(friend.server); }

    /** Adapters call this every tick with what the player is doing; only changes reach the server. */
    public void activity(String version, boolean inWorld, String server) {
        String mode = !inWorld ? "menu" : server == null || server.trim().isEmpty() ? "singleplayer" : "multiplayer";
        String address = "multiplayer".equals(mode) ? server.trim() : null;
        String key = version + "|" + mode + "|" + address;
        if (key.equals(sentActivity)) return;
        sentActivity = key; this.version = version; this.mode = mode; this.server = address;
        cosmetics.beatSoon();
    }
    /** The heartbeat body. The server decides what friends may see from the player's privacy settings. */
    String activityJson() {
        String version = this.version, mode = this.mode, server = this.server;
        if (version == null || mode == null) return "{}";
        return "{\"activity\":{\"version\":" + Json.quote(version) + ",\"mode\":" + Json.quote(mode) + ",\"server\":" + (server == null ? "null" : Json.quote(server)) + "}}";
    }

    private synchronized ExecutorService worker() {
        if (worker == null) worker = Executors.newSingleThreadExecutor(daemon("Cairn friends"));
        return worker;
    }
    private static ThreadFactory daemon(String name) {
        return task -> { Thread thread = new Thread(task, name); thread.setDaemon(true); return thread; };
    }
    private void submit(Runnable task) {
        if (!cosmetics.enabled()) return;
        worker().execute(() -> {
            try { task.run(); } catch (RuntimeException e) { System.err.println("[Cairn Client] Friends task failed: " + e); }
        });
    }

    /** Called after every sign-in; starts the notification poller once. */
    synchronized void signedIn() {
        nextList = 0; listAfter = 0;
        wantList();
        if (events == null) {
            events = daemon("Cairn friend events").newThread(this::pollEvents);
            events.start();
        }
    }
    /** The menu calls this each frame while the Friends tab is open. */
    public void wantList() {
        long now = System.currentTimeMillis();
        if (!available() || now < nextList || now < listAfter) return;
        nextList = now + LIST_EVERY;
        submit(this::load);
    }
    private void load() {
        String token = cosmetics.token();
        if (token == null) return;
        try {
            Cosmetics.Reply reply = Cosmetics.http("GET", cosmetics.server() + "/v1/friends", token, null, null, 1 << 20);
            if (reply.code == 401) { cosmetics.expired(); return; }
            if (reply.code != 200) { loadError = reply.error(); listAfter = System.currentTimeMillis() + 30_000; return; }
            apply(reply.json());
            loadError = null;
        } catch (IOException e) { loadError = "Friends server unreachable. Retrying soon."; listAfter = System.currentTimeMillis() + 30_000; }
    }
    void apply(Map<String, Object> body) {
        List<Friend> list = new ArrayList<Friend>();
        for (Object raw : Json.list(body.get("friends"))) {
            Map<String, Object> f = Json.object(raw);
            Map<String, Object> activity = Json.object(f.get("activity"));
            String server = Json.string(activity.get("server"), null);
            if (server != null && !SERVER.matcher(server).matches()) server = null;
            list.add(new Friend(Json.string(f.get("uuid"), ""), Json.string(f.get("name"), "?"), Json.string(f.get("status"), "offline"),
                Json.string(activity.get("version"), null), Json.string(activity.get("mode"), null), server));
        }
        // Online friends first, then by name, like every other friends list.
        Collections.sort(list, (a, b) -> a.online() != b.online() ? (a.online() ? -1 : 1) : a.name.compareToIgnoreCase(b.name));
        friends = Collections.unmodifiableList(list);
        incoming = people(body.get("incoming"));
        outgoing = people(body.get("outgoing"));
        blocked = people(body.get("blocked"));
        Map<String, Object> settings = Json.object(body.get("settings"));
        applySettings(settings);
        loaded = true;
    }
    private void applySettings(Map<String, Object> settings) {
        if (settings.isEmpty()) return;
        share = Json.string(settings.get("share"), share);
        requests = Json.string(settings.get("requests"), requests);
        appearOffline = settings.get("appearOffline") == Boolean.TRUE;
        boolean dnd = settings.get("dnd") == Boolean.TRUE;
        if (dnd != dnd()) client.setOption("friends.dnd", String.valueOf(dnd));
    }
    private static List<Person> people(Object raw) {
        List<Person> out = new ArrayList<Person>();
        for (Object item : Json.list(raw)) {
            Map<String, Object> p = Json.object(item);
            out.add(new Person(Json.string(p.get("uuid"), ""), Json.string(p.get("name"), "")));
        }
        return Collections.unmodifiableList(out);
    }

    public void add(String target) {
        String value = target == null ? "" : target.trim();
        if (!TARGET.matcher(value).matches()) { note("Enter a Minecraft name or UUID."); return; }
        act("POST", "/v1/friends/requests", "{\"target\":" + Json.quote(value) + "}", reply -> {
            String name = Json.string(reply.get("name"), value);
            return "accepted".equals(reply.get("status")) ? "You and " + name + " are now friends." : "Friend request sent to " + name + ".";
        });
    }
    public void accept(Person p) { act("POST", "/v1/friends/requests/" + p.uuid + "/accept", null, r -> "You and " + p.name + " are now friends."); }
    public void decline(Person p) { act("POST", "/v1/friends/requests/" + p.uuid + "/decline", null, r -> "Declined " + p.name + "."); }
    public void cancel(Person p) { act("POST", "/v1/friends/requests/" + p.uuid + "/cancel", null, r -> "Cancelled the request to " + p.name + "."); }
    public void remove(Friend f) { act("DELETE", "/v1/friends/" + f.uuid, null, r -> "Removed " + f.name + "."); }
    public void block(String uuid, String name) { act("PUT", "/v1/blocks/" + uuid, null, r -> "Blocked " + name + ". They can't see you or message you."); }
    public void unblock(Person p) { act("DELETE", "/v1/blocks/" + p.uuid, null, r -> "Unblocked " + p.name + "."); }
    public void dnd(boolean on) {
        client.setOption("friends.dnd", String.valueOf(on));
        if (on) toasts.clear();
        settings("{\"dnd\":" + on + "}", on ? "Do not disturb is on. Toasts are muted." : "Do not disturb is off.");
    }
    /** "full" shares the server, "game" hides it, "status" shares only online or offline. */
    public void share(String value) { share = value; settings("{\"share\":" + Json.quote(value) + "}", null); }
    public void appearOffline(boolean on) { appearOffline = on; settings("{\"appearOffline\":" + on + "}", on ? "Friends now see you as offline." : "Friends can see you online again."); }
    public void requests(String value) { requests = value; settings("{\"requests\":" + Json.quote(value) + "}", null); }
    private void settings(String json, String done) {
        act("PUT", "/v1/me/social", json, reply -> { applySettings(reply); return done; });
    }

    private interface Done { String apply(Map<String, Object> reply); }
    private void act(String method, String path, String json, Done done) {
        if (!available()) { note(cosmetics.enabled() ? "Sign in with a Microsoft account to use friends." : "Friends need a Cairn server."); return; }
        busy = true;
        submit(() -> {
            try {
                String token = cosmetics.token();
                byte[] body = json == null ? null : json.getBytes(StandardCharsets.UTF_8);
                Cosmetics.Reply reply = Cosmetics.http(method, cosmetics.server() + path, token, json == null ? null : "application/json", body, 1 << 16);
                if (reply.code == 401) { cosmetics.expired(); note("Session expired, signing in again."); }
                else if (reply.code >= 400) note(reply.error());
                else { String text = done.apply(reply.json()); if (text != null) note(text); }
                load();
            } catch (IOException e) { note("Friends server unreachable."); }
            finally { busy = false; }
        });
    }

    /** Long-polls notifications on its own thread, backing off while the server is unreachable. */
    private void pollEvents() {
        long cursor = -1;
        String owner = null;
        int failures = 0;
        while (true) {
            try {
                String token = cosmetics.token(), uuid = cosmetics.uuid();
                if (token == null) { cursor = -1; Thread.sleep(2000); continue; }
                if (!uuid.equals(owner)) { owner = uuid; cursor = -1; }
                String url = cosmetics.server() + "/v1/events" + (cursor < 0 ? "" : "?after=" + cursor + "&wait=" + WAIT_SECONDS);
                Cosmetics.Reply reply = Cosmetics.http("GET", url, token, null, null, 1 << 18, (int) (WAIT_SECONDS + 10) * 1000);
                if (reply.code == 401) { cosmetics.expired(); Thread.sleep(2000); continue; }
                if (reply.code != 200) throw new IOException("HTTP " + reply.code);
                Map<String, Object> body = reply.json();
                if (cursor >= 0 && handle(Json.list(body.get("events")))) { nextList = 0; submit(this::load); }
                cursor = Json.whole(body.get("cursor"), cursor < 0 ? 0 : cursor);
                failures = 0;
                long retry = Json.whole(body.get("retryAfter"), 0);
                if (retry > 0) Thread.sleep(Math.min(300, retry) * 1000);
            } catch (InterruptedException e) {
                return;
            } catch (IOException | RuntimeException e) {
                try { Thread.sleep(Math.min(300_000L, 5_000L << Math.min(6, failures++))); } catch (InterruptedException stop) { return; }
            }
        }
    }
    /** Queues toasts for new events; returns true when the list should be reloaded. */
    boolean handle(List<Object> events) {
        boolean changed = false;
        for (Object raw : events) {
            Map<String, Object> event = Json.object(raw);
            String type = Json.string(event.get("type"), ""), name = Json.string(Json.object(event.get("from")).get("name"), "A friend");
            String title, text;
            // DND or privacy changed from the launcher or another game; apply before toasting the rest.
            if (type.equals("settings")) { applySettings(Json.object(event.get("settings"))); continue; }
            if (type.equals("online")) { title = name; text = "is now online"; }
            else if (type.equals("server")) {
                String server = Json.string(event.get("server"), null);
                if (server == null || !SERVER.matcher(server).matches()) continue;
                title = name; text = "joined " + server;
            }
            else if (type.equals("request")) { title = "Friend request"; text = name + " wants to be friends"; }
            else if (type.equals("accepted")) { title = name; text = "accepted your friend request"; }
            else continue;
            changed = true;
            if (dnd() || !toast(type)) continue;
            toasts.addLast(new Toast(title, text));
            while (toasts.size() > MAX_TOASTS) toasts.pollFirst();
        }
        return changed;
    }
    public List<Toast> toasts() { return new ArrayList<Toast>(toasts); }

    /** Draws queued toasts in the top-right corner; both the HUD and the menu call this. */
    public void render(CairnClient.Canvas canvas, int width, int height) {
        if (toasts.isEmpty()) return;
        long now = System.currentTimeMillis();
        int w = Math.min(190, width - 8), y = 6;
        for (Iterator<Toast> it = toasts.iterator(); it.hasNext(); ) {
            Toast toast = it.next();
            if (toast.shown == 0) toast.shown = now;
            if (now - toast.shown > TOAST_MS) { it.remove(); continue; }
            if (y + 30 > height || w < 40) break;
            int x = width - w - 4;
            canvas.rect(x, y, w, 30, 0xf020261f);
            canvas.rect(x, y, 2, 30, CairnClient.ACCENT);
            canvas.text(CairnClient.fit(canvas, toast.title, w - 12), x + 8, y + 5, CairnClient.TEXT);
            canvas.text(CairnClient.fit(canvas, toast.text, w - 12), x + 8, y + 17, CairnClient.MUTED);
            y += 34;
        }
    }
}
