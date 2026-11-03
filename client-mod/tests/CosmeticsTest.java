package app.cairn.client;

import com.sun.net.httpserver.HttpServer;
import java.awt.image.BufferedImage;
import java.io.*;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import javax.imageio.ImageIO;

public final class CosmeticsTest {
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    private static final class Canvas implements CairnClient.Canvas {
        final int width, height;
        final List<String> labels = new ArrayList<String>(), images = new ArrayList<String>();
        Canvas(int width, int height) { this.width = width; this.height = height; }
        public void rect(int x, int y, int w, int h, int color) { check(x >= 0 && y >= 0 && x + w <= width && y + h <= height, "Rectangle outside viewport: " + x + "," + y + "," + w + "," + h); }
        public int textWidth(String text) { return text.length() * 6; }
        public void text(String text, int x, int y, int color) { textScaled(text, x, y, color, 1, false); }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) {
            check(x >= 0 && y >= 0 && x + Math.round(text.length() * 6 * scale) <= width && y + 9 <= height, "Text outside viewport: " + text);
            labels.add(text);
        }
        public void image(String texture, int x, int y, int w, int h, float u0, float v0, float u1, float v1) {
            check(x >= 0 && y >= 0 && x + w <= width && y + h <= height, "Image outside viewport");
            images.add(texture);
        }
    }
    private static void waitFor(Callable<Boolean> condition, String message) throws Exception {
        long end = System.currentTimeMillis() + 5000;
        while (System.currentTimeMillis() < end) { if (condition.call()) return; Thread.sleep(20); }
        throw new AssertionError(message);
    }
    private static byte[] strip(int width, int frames) throws IOException {
        BufferedImage image = new BufferedImage(width, width / 2 * frames, BufferedImage.TYPE_INT_ARGB);
        for (int i = 0; i < frames; i++) image.setRGB(0, i * width / 2, 0xff000000 | i);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        ImageIO.write(image, "png", out);
        return out.toByteArray();
    }

    private static void units() throws Exception {
        check("https://cosmetics.example.com".equals(Cosmetics.normalizeUrl(" https://cosmetics.example.com/ ")), "https URL accepted and trimmed");
        check("http://127.0.0.1:8787".equals(Cosmetics.normalizeUrl("http://127.0.0.1:8787")), "Loopback http accepted");
        check(Cosmetics.normalizeUrl("http://cosmetics.example.com") == null, "Plain http to another host is refused");
        check(Cosmetics.normalizeUrl("https://user:pass@example.com") == null, "Credentials in the URL are refused");
        check(Cosmetics.normalizeUrl("file:///etc/passwd") == null, "Other schemes are refused");

        Map<String, Object> parsed = Json.object(Json.parse("{\"a\":[1,2.5,\"x\\\"\\u0041\"],\"b\":{\"c\":null,\"d\":true}}"));
        check(Json.list(parsed.get("a")).size() == 3 && "x\"A".equals(Json.list(parsed.get("a")).get(2)), "Arrays and escapes parse");
        check(Json.object(parsed.get("b")).get("d") == Boolean.TRUE, "Nested objects parse");
        StringBuilder deep = new StringBuilder();
        for (int i = 0; i < 40; i++) deep.append('[');
        try { Json.parse(deep.toString()); throw new AssertionError("Deep nesting must be refused"); } catch (IllegalArgumentException expected) { }
        check(Json.quote("a\"b\\\n").equals("\"a\\\"b\\\\\\u000a\""), "Strings are escaped");

        String hash = "ab12000000000000000000000000000000000000000000000000000000000000";
        Map<String, Object> custom = Json.object(Json.parse("{\"name\":\"Alex\",\"role\":\"staff\",\"badge\":\"gold\",\"cape\":{\"id\":\"custom\",\"url\":\"/v1/capes/" + hash + ".png\",\"frames\":4,\"frameMs\":120}}"));
        Cosmetics.Look look = Cosmetics.parseLook("u", custom);
        check(look.staff() && ("custom-" + hash).equals(look.cape) && look.frames == 4 && look.frameMs == 120, "Custom capes keep their hash and timing");
        Map<String, Object> hostile = Json.object(Json.parse("{\"badge\":\"../../x\",\"cape\":{\"id\":\"custom\",\"url\":\"https://evil.example/x.png\"}}"));
        Cosmetics.Look safe = Cosmetics.parseLook("u", hostile);
        check(safe.cape == null && "none".equals(safe.badge), "Server data can't point textures at other paths or hosts");
        check(Cosmetics.parseLook("u", Collections.<String, Object>emptyMap()) == null, "Unknown players have no look");

        check(Cosmetics.frame(0, 16, 100) == 0 && Cosmetics.frame(250, 16, 100) == 2 && Cosmetics.frame(1700, 16, 100) == 1, "Frames advance with wall-clock time");
        check(Cosmetics.frame(12345, 1, 0) == 0, "Static capes stay on frame 0");
        check(!Cosmetics.realToken("0") && !Cosmetics.realToken(null) && Cosmetics.realToken("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.signature-part"), "Offline placeholders never go to Mojang");

        List<byte[]> frames = Cosmetics.split(strip(64, 3), 3);
        check(frames.size() == 3, "Animated strips split into frames");
        BufferedImage second = ImageIO.read(new ByteArrayInputStream(frames.get(1)));
        check(second.getWidth() == 64 && second.getHeight() == 32 && (second.getRGB(0, 0) & 0xffffff) == 1, "Each frame is one 2:1 slice");
        check(Cosmetics.split(strip(64, 3), 1).size() == 1, "Frame count from the server caps the split");
        try { Cosmetics.split(strip(100, 1), 1); throw new AssertionError("Odd sizes must be refused"); } catch (IOException expected) { }
    }

    private static void offline(Path parent) throws Exception {
        System.clearProperty("cairn.cosmetics.url");
        CairnClient client = new CairnClient(Files.createTempDirectory(parent, "offline-"));
        check(!client.cosmetics.enabled() && client.cosmetics.look("069a79f444e94726a5befca90e38aaf5") == null, "No server means no lookups");
        client.cosmetics.account("Steve", "069a79f4-44e9-4726-a5be-fca90e38aaf5", "0");
        client.cosmetics.tick();
        Canvas canvas = new Canvas(640, 360);
        client.renderMenu(canvas, 640, 360, 0, 0);
        check(canvas.labels.contains("Cosmetics"), "The menu links to cosmetics");
        client.escape();
        for (int[] size : new int[][] { { 320, 240 }, { 480, 270 }, { 640, 360 }, { 1280, 720 } }) {
            CairnClient fresh = new CairnClient(Files.createTempDirectory(parent, "size-"));
            Canvas small = new Canvas(size[0], size[1]);
            fresh.renderMenu(small, size[0], size[1], 0, 0);
            check(small.labels.contains("Cosmetics"), "Cosmetics entry visible at " + size[0] + "x" + size[1]);
        }
        openCosmetics(client, canvas);
        check(canvas.labels.contains("Capes") && canvas.labels.contains("Pulse"), "Catalog previews show without a server");
        check(canvas.labels.stream().anyMatch(label -> label.startsWith("No cosmetics server set")), "Explains why equipping is off");
        waitFor(() -> { canvas.images.clear(); canvas.labels.clear(); client.renderMenu(canvas, 640, 360, 0, 0); return canvas.images.stream().anyMatch(key -> key.startsWith("cape/pulse/")); }, "Bundled capes render previews");
        check(client.escape() && !client.escape(), "Escape leaves the cosmetics page first");
    }
    private static void openCosmetics(CairnClient client, Canvas canvas) {
        canvas.labels.clear();
        client.renderMenu(canvas, canvas.width, canvas.height, 0, 0);
        // The header button sits left of the bind button; find it by scanning the header row.
        for (int x = 0; x < canvas.width; x += 4) {
            client.mouseDown(canvas.width, canvas.height, x, 26);
            canvas.labels.clear();
            client.renderMenu(canvas, canvas.width, canvas.height, 0, 0);
            if (canvas.labels.contains("Back to modules") && canvas.labels.contains("Capes")) return;
            client.escape();
        }
        throw new AssertionError("Could not open the cosmetics page");
    }

    private static void server(Path parent) throws Exception {
        final String self = "5627dd98e6be3c21b8a8e92344183641", other = "069a79f444e94726a5befca90e38aaf5";
        final List<String> calls = new CopyOnWriteArrayList<String>();
        final Map<String, String> bodies = new ConcurrentHashMap<String, String>();
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            String path = exchange.getRequestURI().getPath(), auth = exchange.getRequestHeaders().getFirst("Authorization");
            byte[] request = Cosmetics.readAll(exchange.getRequestBody(), 1 << 16);
            bodies.put(exchange.getRequestMethod() + " " + path, new String(request, StandardCharsets.UTF_8));
            calls.add(exchange.getRequestMethod() + " " + path + (auth == null ? "" : " auth=" + auth));
            String reply;
            int code = 200;
            if (path.equals("/v1/auth/start")) reply = "{\"serverId\":\"0123456789abcdef0123456789abcdef01234567\"}";
            else if (path.equals("/v1/auth/finish")) reply = "{\"token\":\"session-token\",\"expiresIn\":3600,\"uuid\":\"" + self + "\",\"name\":\"Steve\",\"role\":\"staff\"}";
            else if (path.equals("/v1/me")) reply = "{\"role\":\"staff\",\"equipped\":{\"cape\":\"pulse\",\"badge\":null},\"customCape\":null,\"unlocked\":{\"capes\":[\"classic\",\"pulse\",\"staff\"],\"badges\":[\"classic\",\"gold\"]},\"moderation\":\"approval\"}";
            else if (path.equals("/v1/heartbeat")) reply = "{\"online\":2,\"next\":60}";
            else if (path.equals("/v1/stats")) reply = "{\"online\":2,\"peak\":5,\"total\":9}";
            else if (path.equals("/v1/players")) reply = "{\"players\":{\"" + other + "\":{\"name\":\"Alex\",\"role\":\"player\",\"badge\":\"moss\",\"cape\":{\"id\":\"spectrum\",\"frames\":20,\"frameMs\":80}}}}";
            else if (path.equals("/v1/me/equipped")) reply = "{}";
            else { code = 404; reply = "{\"error\":\"Not found.\"}"; }
            byte[] bytes = reply.getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(code, bytes.length);
            try (OutputStream out = exchange.getResponseBody()) { out.write(bytes); }
        });
        server.start();
        try {
            System.setProperty("cairn.cosmetics.url", "http://127.0.0.1:" + server.getAddress().getPort() + "/");
            final CairnClient client = new CairnClient(Files.createTempDirectory(parent, "server-"));
            final Cosmetics cosmetics = client.cosmetics;
            check(cosmetics.enabled(), "System property enables cosmetics");
            cosmetics.account("Steve", "5627dd98-e6be-3c21-b8a8-e92344183641", "0");
            waitFor(() -> cosmetics.signedIn() && "pulse".equals(cosmetics.equippedCape()), "Signs in and loads the player's cosmetics");
            check("staff".equals(cosmetics.role()) && cosmetics.status().contains("staff"), "Role comes from the server");
            check(bodies.get("POST /v1/auth/finish").contains("\"serverId\":\"0123456789abcdef0123456789abcdef01234567\""), "Finish echoes the server id");
            for (String call : calls) check(!call.contains("auth=") || call.contains("auth=Bearer session-token"), "Only the session token is sent: " + call);
            check(calls.stream().anyMatch(call -> call.equals("GET /v1/me auth=Bearer session-token")), "Session token authorizes /v1/me");

            check(cosmetics.look(other) == null, "First lookup is queued");
            cosmetics.tick();
            waitFor(() -> cosmetics.look(other) != null, "Batch lookup fills the cache");
            check(calls.stream().anyMatch(call -> call.equals("GET /v1/players")), "Lookups are anonymous");
            Cosmetics.Look look = cosmetics.look(other);
            check("spectrum".equals(look.cape) && look.frames == 20 && "moss".equals(look.badge), "Look parsed");
            waitFor(() -> cosmetics.capeTexture(look) != null && cosmetics.texture(cosmetics.capeTexture(look)) != null, "Animated cape frames load");
            waitFor(() -> cosmetics.badgeTexture(look) != null, "Badge texture loads");

            Canvas canvas = new Canvas(960, 540);
            openCosmetics(client, canvas);
            waitFor(() -> { canvas.images.clear(); canvas.labels.clear(); client.renderMenu(canvas, 960, 540, 0, 0); return canvas.images.stream().anyMatch(key -> key.startsWith("cape/staff/")) && canvas.labels.contains("Worn"); }, "Unlocked staff cape and the worn one are listed");
            check(canvas.labels.stream().anyMatch(label -> label.startsWith("2 online / 5 peak / 9 players")), "Online stats shown");
            cosmetics.equip("staff", "gold");
            waitFor(() -> bodies.containsKey("PUT /v1/me/equipped") && !cosmetics.busy(), "Equip request sent");
            check(bodies.get("PUT /v1/me/equipped").equals("{\"cape\":\"staff\",\"badge\":\"gold\"}"), "Equip sends both slots");

            cosmetics.visible(false);
            check(cosmetics.look(other) == null, "Hiding cosmetics hides every look");
            cosmetics.visible(true);
        } finally {
            System.clearProperty("cairn.cosmetics.url");
            server.stop(0);
        }
    }

    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-cosmetics-test-");
        units();
        offline(directory);
        server(directory);
        System.out.println("Cosmetics tests passed.");
    }
}
