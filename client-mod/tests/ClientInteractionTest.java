package app.cairn.client;

import java.nio.file.*;
import java.util.*;
import app.cairn.client.CairnClient.Module;
import app.cairn.client.CairnClient.Setting;
import app.cairn.client.CairnClient.Bind;

public final class ClientInteractionTest {
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    private static final class Label {
        final String text; final int x, y;
        Label(String text, int x, int y) { this.text = text; this.x = x; this.y = y; }
    }
    private static final class Canvas implements CairnClient.Canvas {
        final int width, height;
        int measures;
        final List<Label> labels = new ArrayList<Label>();
        final List<Integer> colors = new ArrayList<Integer>();
        int icons;
        Canvas(int width, int height) { this.width = width; this.height = height; }
        public void rect(int x, int y, int w, int h, int color) { check(x >= 0 && y >= 0 && w >= 0 && h >= 0 && x + w <= width && y + h <= height, "Rectangle outside " + width + "x" + height + ": " + x + "," + y + "," + w + "," + h); colors.add(color); }
        public void icon(Object icon, int x, int y, int size) { icons++; check(x >= 0 && y >= 0 && x + size <= width && y + size <= height, "Icon outside viewport"); }
        public int textWidth(String text) { measures++; return text.length() * 6; }
        public void text(String text, int x, int y, int color) { textScaled(text, x, y, color, 1, false); }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) {
            check(x >= 0 && y >= 0 && x + Math.round(text.length() * 6 * scale) <= width && y + Math.round(9 * scale) <= height, "Text outside viewport: " + text);
            labels.add(new Label(text, x, y));
        }
        void render(CairnClient client) { labels.clear(); client.renderMenu(this, width, height, 0, 0); }
        void click(CairnClient client, String label) {
            for (Label item : labels) if (item.text.equals(label)) { client.mouseDown(width, height, item.x + 1, item.y + 1); render(client); return; }
            throw new AssertionError("Control not visible: " + label);
        }
        void clickAfter(CairnClient client, String label, String preceding) {
            boolean seen = false;
            for (Label item : labels) {
                if (seen && item.text.equals(label)) { client.mouseDown(width, height, item.x + 1, item.y + 1); render(client); return; }
                if (item.text.equals(preceding)) seen = true;
            }
            throw new AssertionError("Control not visible: " + label + " after " + preceding);
        }
    }
    private static boolean overlaps(int[] a, int[] b) { return a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]; }
    private static void mouseButtonsAndCards(Path parent) throws Exception {
        for (int[] buttons : new int[][] { { 0, 1 }, { 1, 3 } }) {
            CairnClient client = new CairnClient(Files.createTempDirectory(parent, "mouse-"));
            for (int i = 0; i < 2; i++) client.mouseClick(buttons[0], buttons[0], buttons[1]);
            for (int i = 0; i < 3; i++) client.mouseClick(buttons[1], buttons[0], buttons[1]);
            client.mouseClick(42, buttons[0], buttons[1]);
            client.setting(Module.KEYSTROKES, Setting.KEY_SIZE, 36);
            Canvas hud = new Canvas(960, 540);
            client.renderHud(hud, 960, 540);
            check(client.state(Module.CPS).value.equals("2 / 3 CPS"), "Native mouse codes preserve independent left/right counters");
            Label left = null, right = null;
            for (Label label : hud.labels) {
                if (label.text.equals("2 CPS")) left = label;
                if (label.text.equals("3 CPS")) right = label;
            }
            check(left != null && right != null && left.x < right.x, "CPS labels stay under the matching mouse button");
        }
        for (int[] size : new int[][] { { 320, 240 }, { 427, 240 }, { 640, 360 }, { 960, 540 } }) {
            CairnClient client = new CairnClient(Files.createTempDirectory(parent, "cards-"));
            Canvas canvas = new Canvas(size[0], size[1]);
            canvas.render(client);
            canvas.clickAfter(client, "Enabled", "FPS");
            check(!client.enabled(Module.FPS), "The first card's switch disables FPS");
            canvas.clickAfter(client, "Disabled", "FPS");
            check(client.enabled(Module.FPS), "The switch enables FPS again");
            canvas.click(client, "FPS");
            check(canvas.labels.stream().anyMatch(label -> label.text.startsWith("Scale")), "Clicking the card opens settings instead of silently selecting it");
            canvas.click(client, "X");
            check(client.consumeMenuClose() && !client.consumeMenuClose(), "Close is a one-shot action for the game adapter");
        }
    }
    private static void layoutAndKeys(Path parent) throws Exception {
        Path directory = Files.createTempDirectory(parent, "layout-");
        Files.createDirectories(directory.resolve("config"));
        // Positions older builds wrote for every module, plus one the player really moved.
        Files.write(directory.resolve("config/cairn-client.properties"), Arrays.asList("FPS.x=12", "FPS.y=12", "COORDINATES.x=12", "COORDINATES.y=40", "CPS.x=110", "CPS.y=112", "PING.x=300", "PING.y=200"));
        CairnClient client = new CairnClient(directory);
        check(!client.state(Module.FPS).placed && !client.state(Module.CPS).placed && client.state(Module.PING).placed, "Legacy default slots count as unplaced");
        for (String preset : new String[] { "Minimal", "Combat", "Explore", "All" }) {
            if (preset.equals("All")) { for (Module module : Module.values()) if (!client.enabled(module)) client.toggle(module); }
            else client.preset(preset.equals("Minimal") ? 0 : preset.equals("Combat") ? 1 : 2);
            for (int[] size : new int[][] { { 427, 240 }, { 640, 360 }, { 960, 540 } }) {
                client.value(Module.FPS, "144 FPS"); client.value(Module.COORDINATES, "XYZ 1234 64 -5678");
                client.renderHud(new Canvas(size[0], size[1]), size[0], size[1]);
                List<int[]> boxes = new ArrayList<int[]>();
                for (Module module : Module.values()) if (client.enabled(module) && module.hud && module != Module.CROSSHAIR) boxes.add(client.bounds(module));
                // A screen too small for everything may clamp; the common sizes must not overlap.
                if (size[1] >= 360 || !preset.equals("All")) for (int i = 0; i < boxes.size(); i++) for (int j = i + 1; j < boxes.size(); j++) check(!overlaps(boxes.get(i), boxes.get(j)), preset + " preset overlaps at " + size[0] + "x" + size[1]);
            }
        }
        int[] fps = client.bounds(Module.FPS);
        client.value(Module.FPS, "9 FPS"); client.renderHud(new Canvas(960, 540), 960, 540);
        int narrow = client.bounds(Module.FPS)[2];
        client.value(Module.FPS, "1440 FPS"); client.renderHud(new Canvas(960, 540), 960, 540);
        check(narrow < fps[2] + 30 && client.bounds(Module.FPS)[2] > narrow, "Panels fit their text");
        client.setting(Module.FPS, Setting.AUTO_WIDTH, 0);
        check(client.bounds(Module.FPS)[2] == 150, "Fixed width still available");
        client.save();
        check(!new String(Files.readAllBytes(directory.resolve("config/cairn-client.properties")), "UTF-8").contains("FPS.x="), "Unplaced widgets do not pin a position");

        check(!client.keysManaged(), "Forge keeps vanilla key bindings");
        client.manageKeys(344, "Right Shift", 67, "C");
        Canvas canvas = new Canvas(800, 600); canvas.render(client);
        canvas.click(client, "Menu key: Right Shift");
        check(client.binding(), "Clicking a key button waits for a key");
        client.bind(82, "R");
        check(!client.binding() && client.keyCode(Bind.MENU) == 82, "Pressed key becomes the menu key");
        client.select(Module.ZOOM); client.enterEditor(); client.editorSettings(); canvas.render(client);
        canvas.click(client, "Zoom key: C");
        check(client.escape() && !client.binding() && client.keyCode(Bind.ZOOM) == 67, "Escape cancels rebinding");
        CairnClient restored = new CairnClient(directory); restored.manageKeys(344, "Right Shift", 67, "C");
        check(restored.keyCode(Bind.MENU) == 82 && restored.keyName(Bind.MENU).equals("R"), "Key bindings persist");

        if (!client.enabled(Module.ZOOM)) client.toggle(Module.ZOOM);
        client.setting(Module.ZOOM, Setting.SMOOTH_ZOOM, 0);
        check(client.zoomFactor(true) == 4, "Zoom uses its strength");
        check(client.zoomScroll(true, 1) && client.zoomFactor(true) == 5, "Scrolling while zoomed zooms further");
        check(!client.zoomScroll(false, 1), "Scrolling without zoom keeps the hotbar");
        for (int i = 0; i < 40; i++) client.zoomScroll(true, -1);
        check(client.zoomFactor(true) == 1, "Scroll zoom never zooms out past normal");
        client.zoomFactor(false);
        check(client.zoomFactor(true) == 4, "Releasing zoom forgets the scroll level");
        client.setting(Module.ZOOM, Setting.ZOOM_CINEMATIC, 0);
        check(!client.smoothCamera(true, false), "Cinematic zoom can be turned off");
        client.setting(Module.ZOOM, Setting.ZOOM_CINEMATIC, 1);
        check(client.smoothCamera(true, false) && client.smoothCamera(true, true) && !client.smoothCamera(false, true), "Cinematic zoom restores the player's camera");
        check(client.smoothCamera(true, true) && client.smoothCamera(false, true), "Cinematic zoom keeps an enabled camera");
        check(client.menuKey(82) && !client.menuKey(344), "Menu opens on the bound key");
    }
    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-interaction-test-");
        try {
            previewsAndOverlay(directory);
            CairnClient client = new CairnClient(directory);
            client.focusSearch();
            for (char c : "keySTROKES".toCharArray()) client.type(c);
            check(client.filteredModules().equals(Collections.singletonList(Module.KEYSTROKES)), "Search is case insensitive");
            client.backspace(); check(client.query().equals("keySTROKE"), "Backspace edits search");
            check(client.escape() && !client.typing(), "Escape leaves search focus");
            Canvas canvas = new Canvas(800, 600);
            canvas.render(client); canvas.click(client, "Clear");
            check(client.filteredModules().size() == Module.values().length, "Clear restores modules");
            client.select(Module.KEYSTROKES); client.enterEditor(); client.editorSettings();
            canvas.render(client); canvas.click(client, "+");
            check(client.setting(Module.KEYSTROKES, Setting.SCALE) == 110, "Scale stepper updates selected widget");
            canvas.click(client, "Change");
            String field = null;
            for (Label label : canvas.labels) if (label.text.startsWith("#")) field = label.text;
            canvas.click(client, field);
            for (int i = 0; i < 6; i++) client.backspace();
            for (char c : "12ab34".toCharArray()) client.type(c);
            canvas.render(client); canvas.click(client, "Apply color");
            check(client.setting(Module.KEYSTROKES, Setting.TEXT_COLOR) == 0xff12ab34, "Arbitrary hex color applied");
            CairnClient restored = new CairnClient(directory);
            check(restored.setting(Module.KEYSTROKES, Setting.TEXT_COLOR) == 0xff12ab34, "Color persists");
            check(restored.setting(Module.KEYSTROKES, Setting.SCALE) == 110, "Size persists independently");
            check(restored.setting(Module.FPS, Setting.SCALE) == 100, "Other widgets remain unchanged");
            canvas.click(client, "Back to modules"); canvas.click(client, "Minimal");
            check(client.enabled(Module.FPS) && !client.enabled(Module.KEYSTROKES), "Preset changes actual enabled modules");
            client.toggle(Module.COORDINATES);
            client.select(Module.FPS); client.state(Module.FPS).x = 20; client.state(Module.FPS).y = 40; client.state(Module.FPS).placed = true;
            client.enterEditor(); client.mouseDown(800, 600, 30, 50); client.drag(800, 600, 60, 100); client.mouseUp();
            check(client.state(Module.FPS).x == 52 && client.state(Module.FPS).y == 92, "Drag uses grid snapping");
            int[] fps = client.bounds(Module.FPS);
            client.mouseDown(800, 600, fps[0] + fps[2] - 2, fps[1] + fps[3] - 2); client.drag(800, 600, fps[0] + fps[2] * 3 / 2, 140); client.mouseUp();
            check(client.setting(Module.FPS, Setting.SCALE) == 150, "Corner drag resizes widget");
            client.leaveEditor();
            for (int width : new int[] {320, 400, 640, 1280}) for (int height : new int[] {240, 360, 720}) {
                Canvas viewport = new Canvas(width, height);
                for (Module module : Module.values()) {
                    client.select(module); client.enterEditor(); client.editorSettings();
                    for (int page = 0; page < 20; page++) { viewport.render(client); viewport.click(client, "Down"); }
                    client.escape();
                }
            }
            client.setting(Module.FPS, Setting.SCALE, 999); check(client.setting(Module.FPS, Setting.SCALE) == 250, "Scale bounded");
            client.setting(Module.FPS, Setting.OPACITY, -200); check(client.setting(Module.FPS, Setting.OPACITY) == 0, "Opacity bounded");
            check(CairnClient.readable(0xff000000) == 0xffffffff && CairnClient.readable(0xffffffff) == 0xff000000, "Pressed labels retain contrast");
            if (!client.enabled(Module.KEYSTROKES)) client.toggle(Module.KEYSTROKES);
            client.setting(Module.KEYSTROKES, Setting.SCALE, 250);
            client.setting(Module.KEYSTROKES, Setting.KEY_SIZE, 36);
            client.setting(Module.KEYSTROKES, Setting.KEY_GAP, 8);
            client.renderHud(new Canvas(320, 240), 320, 240);
            client.toggle(Module.KEYSTROKES);
            client.value(Module.FPS, "60 FPS");
            Canvas hud = new Canvas(800, 600); client.renderHud(hud, 800, 600); int measurements = hud.measures;
            client.renderHud(hud, 800, 600); check(hud.measures == measurements, "Stable HUD text does not repeat layout measurement");
            layoutAndKeys(directory);
            mouseButtonsAndCards(directory);
            check(client.saveError() == null, "No settings errors");
            System.out.println("PASS: search, per-widget colors and sizes, presets, snapping, resize handles, all settings pages at 12 viewport sizes, contrast, stable HUD layout caching, overlap-free presets, legacy position migration, key rebinding and scroll zoom.");
        } finally {
            try (java.util.stream.Stream<Path> paths = Files.walk(directory)) { paths.sorted(Comparator.reverseOrder()).forEach(path -> { try { Files.delete(path); } catch (Exception e) { throw new RuntimeException(e); } }); }
        }
    }
    private static void previewsAndOverlay(Path parent) throws Exception {
        CairnClient client = new CairnClient(Files.createTempDirectory(parent, "previews-"));
        check(client.setting(Module.FPS, Setting.BORDER) == 0, "New HUDs have no colored edge");
        client.value(Module.FPS, "Preview FPS");
        Canvas preview = new Canvas(400, 300);
        client.setting(Module.FPS, Setting.BG_COLOR, 0x123456);
        client.setting(Module.FPS, Setting.OPACITY, 50);
        client.renderPreview(preview, Module.FPS, 10, 10, 160, 100);
        check(preview.colors.contains(0x7f123456), "Preview uses custom background and opacity");
        client.setting(Module.FPS, Setting.BACKGROUND, 0); preview.colors.clear();
        client.renderPreview(preview, Module.FPS, 10, 10, 160, 100);
        check(preview.colors.isEmpty(), "Preview reacts immediately to background being disabled");
        client.rows(Module.ARMOR, Arrays.asList(new CairnClient.HudRow(new Object(), "363"), new CairnClient.HudRow(new Object(), "528")));
        client.renderPreview(preview, Module.ARMOR, 10, 10, 160, 100);
        check(preview.icons == 2 && preview.labels.stream().anyMatch(label -> label.text.equals("528")), "Row previews render item icons and durability");
        client.enterEditor(); preview.render(client); preview.click(client, "Cairn Settings");
        check(!client.editing() && preview.labels.stream().anyMatch(label -> label.text.equals("Modules")), "Editor opens the module gallery through Cairn Settings");
        for (int[] size : new int[][] {{320, 240}, {640, 360}, {960, 540}}) {
            Canvas canvas = new Canvas(size[0], size[1]);
            CairnTitle title = new CairnTitle(); final int[] calls = {0};
            for (String name : new String[] {"Singleplayer", "Multiplayer", "Cairn Settings", "Cosmetics", "Friends", "Minecraft settings", "Quit game"}) title.add(name, () -> calls[0]++, true);
            title.render(canvas, size[0], size[1], 0, 0);
            for (Label label : canvas.labels) if (label.text.equals("Singleplayer")) title.click(label.x + 1, label.y + 1);
            title.key(false, false); title.key(true, false);
            check(calls[0] == 2, "Title actions work with mouse and keyboard");
        }
        client.cosmetics.equip("ember", "classic");
        check(client.cosmetics.status().startsWith("No cosmetics server"), "Selection preserves the missing-server diagnosis");
    }
}
