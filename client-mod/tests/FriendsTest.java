package app.cairn.client;

import com.sun.net.httpserver.HttpServer;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;

public final class FriendsTest {
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    private static final class Canvas implements CairnClient.Canvas {
        final int width, height;
        final List<String> labels = new ArrayList<String>();
        final Map<String, int[]> positions = new HashMap<String, int[]>();
        Canvas(int width, int height) { this.width = width; this.height = height; }
        public void rect(int x, int y, int w, int h, int color) { check(x >= 0 && y >= 0 && x + w <= width && y + h <= height, "Rectangle outside viewport: " + x + "," + y + "," + w + "," + h + " in " + width + "x" + height); }
        public int textWidth(String text) { return text.length() * 6; }
        public void text(String text, int x, int y, int color) { textScaled(text, x, y, color, 1, false); }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) {
            check(x >= 0 && y >= 0 && x + Math.round(text.length() * 6 * scale) <= width && y + 9 <= height, "Text outside viewport: " + text + " at " + x + "," + y + " in " + width + "x" + height);
            labels.add(text);
            positions.put(text, new int[] { x, y });
        }
        public void image(String texture, int x, int y, int w, int h, float u0, float v0, float u1, float v1) { check(x >= 0 && y >= 0 && x + w <= width && y + h <= height, "Image outside viewport"); }
    }
    private static void waitFor(Callable<Boolean> condition, String message) throws Exception {
        long end = System.currentTimeMillis() + 8000;
        while (System.currentTimeMillis() < end) { if (condition.call()) return; Thread.sleep(20); }
        throw new AssertionError(message);
    }
    private static final String LIST = "{\"friends\":["
        + "{\"uuid\":\"b\",\"name\":\"zed\",\"status\":\"offline\",\"activity\":null},"
        + "{\"uuid\":\"c\",\"name\":\"Bob\",\"status\":\"online\",\"activity\":{\"version\":\"1.21.11\",\"mode\":\"multiplayer\",\"server\":\"mc.example.net\"}},"
        + "{\"uuid\":\"d\",\"name\":\"amy\",\"status\":\"dnd\",\"activity\":{\"version\":\"1.8.9\",\"mode\":\"multiplayer\",\"server\":null}},"
        + "{\"uuid\":\"e\",\"name\":\"Eve\",\"status\":\"online\",\"activity\":{\"version\":\"26.3\",\"mode\":\"multiplayer\",\"server\":\"evil\\u0007host\"}}],"
        + "\"incoming\":[{\"uuid\":\"f\",\"name\":\"Finn\"}],\"outgoing\":[{\"uuid\":\"g\",\"name\":\"Gus\"}],\"blocked\":[{\"uuid\":\"h\",\"name\":\"Hal\"}],"
        + "\"settings\":{\"share\":\"game\",\"appearOffline\":true,\"requests\":\"none\",\"dnd\":false}}";

    private static void units(Path parent) throws Exception {
        System.clearProperty("cairn.cosmetics.url");
        CairnClient client = new CairnClient(Files.createTempDirectory(parent, "units-"));
        Friends friends = client.friends;
        friends.apply(Json.object(Json.parse(LIST)));
        List<Friends.Friend> list = friends.friends();
        check(list.get(0).name.equals("amy") && list.get(1).name.equals("Bob") && list.get(3).name.equals("zed"), "Online friends first, then by name");
        check(list.get(1).activity().equals("Multiplayer 1.21.11 on mc.example.net"), "Activity shows the server");
        check(list.get(0).activity().equals("Do not disturb / Multiplayer 1.8.9"), "A hidden server is just multiplayer");
        check(list.get(2).server == null, "Control characters in a server address are dropped");
        check(list.get(3).activity().equals("Offline"), "Offline friends say so");
        check(friends.incoming().size() == 1 && friends.outgoing().size() == 1 && friends.blocked().size() == 1, "Requests and blocks parsed");
        check(friends.share().equals("game") && friends.appearOffline() && friends.requests().equals("none"), "Settings parsed");

        check(friends.activityJson().equals("{}"), "No activity before the game reports one");
        friends.activity("1.21.11", true, " Play.Example.net ");
        check(friends.activityJson().equals("{\"activity\":{\"version\":\"1.21.11\",\"mode\":\"multiplayer\",\"server\":\"Play.Example.net\"}}"), "Multiplayer activity carries the address");
        friends.activity("1.21.11", true, null);
        check(friends.activityJson().contains("\"singleplayer\"") && friends.activityJson().contains("\"server\":null"), "Singleplayer has no server");
        friends.activity("1.21.11", false, "ignored");
        check(friends.activityJson().contains("\"menu\""), "Menus have no server");

        check(!friends.canJoin(list.get(1)), "No Join without an adapter handler");
        final List<String> joined = new ArrayList<String>();
        friends.joinHandler(joined::add);
        check(friends.canJoin(list.get(1)) && !friends.canJoin(list.get(0)), "Join needs a visible server");
        friends.join(list.get(1));
        check(joined.equals(Collections.singletonList("mc.example.net")), "Join connects to the friend's server");
        friends.join(list.get(0));
        check(joined.size() == 1, "A hidden address cannot be joined");
        friends.joinHandler(null);
        friends.join(list.get(1));
        check(joined.size() == 1, "Entering a world disables joining even from an existing friend row");

        client.manageKeys(258, "Tab", 67, "C");
        check(client.menuKey(258) && !client.menuCloseKey(258, 258), "Tab can open the menu without closing it during navigation");
        client.focusSearch();
        check(client.typing(), "Search takes text focus");
        client.tab();
        check(!client.typing(), "Tab leaves a text field and advances to controls");
        client.manageKeys(344, "Right Shift", 67, "C");
        check(client.menuCloseKey(344, 258), "Other menu keys still close the menu");

        for (int[] size : new int[][] { { 320, 240 }, { 427, 240 }, { 480, 270 }, { 570, 320 }, { 640, 360 }, { 683, 384 }, { 800, 450 }, { 854, 480 }, { 960, 540 }, { 1280, 720 }, { 1600, 900 }, { 1920, 1080 } }) {
            CairnClient navigation = new CairnClient(Files.createTempDirectory(parent, "navigation-"));
            navigation.manageKeys(344, "Right Shift", 67, "C");
            Canvas canvas = new Canvas(size[0], size[1]);
            navigation.renderMenu(canvas, size[0], size[1], 0, 0);
            check(canvas.positions.containsKey("Friends") && canvas.positions.containsKey("Cosmetics"), "Both pages are reachable from modules at " + size[0] + "x" + size[1]);
            check(canvas.labels.stream().anyMatch(l -> l.startsWith("Menu key")), "Menu rebinding remains reachable at every viewport");
            int[] at = canvas.positions.get("Friends");
            navigation.mouseDown(size[0], size[1], at[0], at[1]);
            Canvas opened = new Canvas(size[0], size[1]);
            navigation.renderMenu(opened, size[0], size[1], 0, 0);
            check(opened.labels.contains("Back to modules") && opened.labels.stream().anyMatch(l -> l.startsWith("Add a friend")), "Friends button opens the page");
        }

        String events = "[{\"type\":\"online\",\"from\":{\"name\":\"Bob\"}},{\"type\":\"server\",\"from\":{\"name\":\"Bob\"},\"server\":\"mc.example.net\"},"
            + "{\"type\":\"request\",\"from\":{\"name\":\"Finn\"}},{\"type\":\"accepted\",\"from\":{\"name\":\"Gus\"}},{\"type\":\"server\",\"from\":{\"name\":\"X\"},\"server\":\"bad host\"},{\"type\":\"spam\"}]";
        check(friends.handle(Json.list(Json.parse(events))), "Events change the list");
        check(friends.toasts().size() == 3, "At most three toasts at once");
        check(friends.toasts().get(2).text.equals("accepted your friend request"), "Newest toast kept");
        friends.toast("request", false);
        Canvas wait = new Canvas(640, 360);
        for (Friends.Toast t : friends.toasts()) t.shown = 1;
        friends.render(wait, 640, 360);
        check(friends.toasts().isEmpty(), "Toasts expire");
        friends.handle(Json.list(Json.parse("[{\"type\":\"request\",\"from\":{\"name\":\"Finn\"}},{\"type\":\"online\",\"from\":{\"name\":\"Bob\"}}]")));
        check(friends.toasts().size() == 1 && friends.toasts().get(0).title.equals("Bob"), "A muted toast type is skipped");
        friends.dnd(true);
        check(friends.toasts().isEmpty(), "Turning on DND clears toasts");
        check(friends.handle(Json.list(Json.parse("[{\"type\":\"online\",\"from\":{\"name\":\"Bob\"}}]"))) && friends.toasts().isEmpty(), "DND suppresses toasts but still refreshes the list");
        friends.dnd(false);
        friends.handle(Json.list(Json.parse("[{\"type\":\"settings\",\"settings\":{\"share\":\"full\",\"appearOffline\":false,\"requests\":\"everyone\",\"dnd\":true}},{\"type\":\"online\",\"from\":{\"name\":\"Bob\"}}]")));
        check(friends.dnd() && friends.toasts().isEmpty(), "DND turned on from the launcher applies before the next toast");
        friends.handle(Json.list(Json.parse("[{\"type\":\"settings\",\"settings\":{\"dnd\":false}}]")));
        check(!friends.dnd(), "DND turned off from the launcher");
        check(friends.handle(Json.list(Json.parse("[{\"type\":\"presence\",\"from\":{\"name\":\"Bob\"}}]"))) && friends.toasts().isEmpty(), "A privacy change refreshes the list silently");
        friends.toast("request", true);

        friends.handle(Json.list(Json.parse("[{\"type\":\"request\",\"from\":{\"name\":\"" + new String(new char[60]).replace('\0', 'W') + "\"}},{\"type\":\"online\",\"from\":{\"name\":\"Bob\"}}]")));
        for (int[] size : new int[][] { { 60, 40 }, { 200, 50 }, { 320, 240 }, { 640, 360 }, { 1920, 1080 } }) friends.render(new Canvas(size[0], size[1]), size[0], size[1]);

        // The menu reopens on the Friends tab, and the page fits every viewport.
        client.setOption("menu.page", "friends");
        Path dir = Files.createTempDirectory(parent, "page-");
        CairnClient paged = new CairnClient(dir);
        paged.setOption("menu.page", "friends");
        paged = new CairnClient(dir);
        paged.friends.apply(Json.object(Json.parse(LIST)));
        StringBuilder many = new StringBuilder("{\"friends\":[");
        for (int i = 0; i < 40; i++) many.append(i == 0 ? "" : ",").append("{\"uuid\":\"u" + i + "\",\"name\":\"Player_" + i + "\",\"status\":\"online\",\"activity\":{\"version\":\"1.21.11\",\"mode\":\"multiplayer\",\"server\":\"a-very-long-server-address-number-" + i + ".example.net\"}}");
        many.append("],\"incoming\":[{\"uuid\":\"f\",\"name\":\"Finn\"}],\"outgoing\":[],\"blocked\":[]}");
        for (String body : new String[] { LIST, many.toString() }) {
            paged.friends.apply(Json.object(Json.parse(body)));
            for (int[] size : new int[][] { { 320, 240 }, { 427, 240 }, { 480, 270 }, { 570, 320 }, { 640, 360 }, { 683, 384 }, { 800, 450 }, { 854, 480 }, { 960, 540 }, { 1280, 720 }, { 1600, 900 }, { 1920, 1080 } }) {
                Canvas canvas = new Canvas(size[0], size[1]);
                paged.renderMenu(canvas, size[0], size[1], 0, 0);
                check(canvas.labels.contains("Back to modules") && canvas.labels.stream().anyMatch(l -> l.startsWith("Add a friend")), "Friends page opens at " + size[0] + "x" + size[1]);
                check(canvas.labels.contains("Friend requests") && canvas.labels.stream().anyMatch(l -> l.startsWith("Finn")), "Requests listed at " + size[0] + "x" + size[1]);
                for (int i = 0; i < 60; i++) paged.wheel(1);
                paged.renderMenu(canvas, size[0], size[1], 0, 0);
                for (int i = 0; i < 60; i++) paged.wheel(-1);
            }
        }
        check(paged.escape(), "Escape leaves the Friends page");
        Canvas modules = new Canvas(640, 360);
        paged.renderMenu(modules, 640, 360, 0, 0);
        check(!modules.labels.contains("Back to modules") && modules.labels.contains("Friends 1"), "The header shows pending requests");
        check(new CairnClient(dir).option("menu.page", "").equals("modules"), "Leaving the page is remembered too");
    }

    private static void server(Path parent) throws Exception {
        final String self = "5627dd98e6be3c21b8a8e92344183641";
        final Map<String, String> bodies = new ConcurrentHashMap<String, String>();
        final List<String> heartbeats = new CopyOnWriteArrayList<String>();
        final BlockingQueue<String> pending = new LinkedBlockingQueue<String>();
        final AtomicInteger cursor = new AtomicInteger(7);
        final String[] dnd = { "false" };
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.setExecutor(Executors.newCachedThreadPool(task -> { Thread t = new Thread(task); t.setDaemon(true); return t; }));
        server.createContext("/", exchange -> {
            String method = exchange.getRequestMethod(), path = exchange.getRequestURI().getPath(), query = exchange.getRequestURI().getQuery();
            String body = new String(Cosmetics.readAll(exchange.getRequestBody(), 1 << 16), StandardCharsets.UTF_8);
            bodies.put(method + " " + path, body);
            String reply;
            try {
                if (path.equals("/v1/auth/start")) reply = "{\"serverId\":\"0123456789abcdef0123456789abcdef01234567\"}";
                else if (path.equals("/v1/auth/finish")) reply = "{\"token\":\"t\",\"expiresIn\":3600,\"uuid\":\"" + self + "\",\"name\":\"Steve\",\"role\":\"player\"}";
                else if (path.equals("/v1/me")) reply = "{\"role\":\"player\",\"equipped\":{},\"unlocked\":{\"capes\":[],\"badges\":[]}}";
                else if (path.equals("/v1/heartbeat")) { heartbeats.add(body); reply = "{\"online\":1}"; }
                else if (path.equals("/v1/friends")) reply = LIST.replace("\"dnd\":false", "\"dnd\":" + dnd[0]);
                else if (path.equals("/v1/friends/requests")) reply = "{\"status\":\"sent\",\"uuid\":\"x\",\"name\":\"Alex\"}";
                else if (path.equals("/v1/me/social")) { dnd[0] = String.valueOf(body.contains("\"dnd\":true")); reply = "{\"share\":\"full\",\"appearOffline\":false,\"requests\":\"everyone\",\"dnd\":" + dnd[0] + "}"; }
                else if (path.equals("/v1/events")) {
                    if (query == null) reply = "{\"events\":[],\"cursor\":" + cursor.get() + "}";
                    else {
                        check(query.startsWith("after=" + cursor.get() + "&wait="), "The poll continues from the cursor: " + query);
                        String event = pending.poll(2, TimeUnit.SECONDS);
                        reply = event == null ? "{\"events\":[],\"cursor\":" + cursor.get() + "}" : "{\"events\":[" + event + "],\"cursor\":" + cursor.incrementAndGet() + "}";
                    }
                }
                else reply = "{}";
            } catch (InterruptedException e) { reply = "{}"; }
            byte[] bytes = reply.getBytes(StandardCharsets.UTF_8);
            exchange.sendResponseHeaders(200, bytes.length);
            try (OutputStream out = exchange.getResponseBody()) { out.write(bytes); }
        });
        server.start();
        try {
            System.setProperty("cairn.cosmetics.url", "http://127.0.0.1:" + server.getAddress().getPort());
            final CairnClient client = new CairnClient(Files.createTempDirectory(parent, "server-"));
            final Friends friends = client.friends;
            friends.activity("26.3", true, "mc.example.net");
            client.cosmetics.account("Steve", "5627dd98-e6be-3c21-b8a8-e92344183641", "0");
            waitFor(() -> heartbeats.stream().anyMatch(b -> b.contains("\"server\":\"mc.example.net\"")), "The heartbeat carries the activity");
            waitFor(() -> friends.loaded() && friends.friends().size() == 4, "The list loads after sign-in");
            friends.activity("26.3", false, null);
            waitFor(() -> heartbeats.stream().anyMatch(b -> b.contains("\"menu\"")), "An activity change is sent soon");

            Thread.sleep(300);
            pending.add("{\"type\":\"online\",\"from\":{\"uuid\":\"c\",\"name\":\"Bob\"}}");
            waitFor(() -> friends.toasts().size() == 1, "A long-polled event becomes a toast");
            friends.dnd(true);
            waitFor(() -> "{\"dnd\":true}".equals(bodies.get("PUT /v1/me/social")), "DND syncs to the server");
            pending.add("{\"type\":\"request\",\"from\":{\"uuid\":\"f\",\"name\":\"Finn\"}}");
            waitFor(() -> pending.isEmpty() && cursor.get() == 9, "Second event delivered");
            Thread.sleep(200);
            check(friends.toasts().isEmpty(), "No toasts while DND is on");

            friends.add("bad name!");
            check(friends.message().startsWith("Enter a Minecraft name"), "Names are checked locally");
            friends.add("Alex");
            waitFor(() -> "{\"target\":\"Alex\"}".equals(bodies.get("POST /v1/friends/requests")) && friends.message().equals("Friend request sent to Alex."), "Requests send the target");
        } finally {
            System.clearProperty("cairn.cosmetics.url");
            server.stop(0);
        }
    }

    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-friends-test-");
        units(directory);
        server(directory);
        System.out.println("Friends tests passed.");
        System.exit(0);
    }
}
