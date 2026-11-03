package app.cairn.client;

import java.nio.file.*;
import java.util.*;
import app.cairn.client.CairnClient.Module;
import app.cairn.client.CairnClient.Setting;

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
        Canvas(int width, int height) { this.width = width; this.height = height; }
        public void rect(int x, int y, int w, int h, int color) { check(x >= 0 && y >= 0 && w >= 0 && h >= 0 && x + w <= width && y + h <= height, "Rectangle outside " + width + "x" + height + ": " + x + "," + y + "," + w + "," + h); }
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
    }
    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-interaction-test-");
        try {
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
            client.select(Module.FPS); client.state(Module.FPS).x = 20; client.state(Module.FPS).y = 40;
            client.enterEditor(); client.mouseDown(800, 600, 30, 50); client.drag(800, 600, 60, 100); client.mouseUp();
            check(client.state(Module.FPS).x == 52 && client.state(Module.FPS).y == 92, "Drag uses grid snapping");
            client.mouseDown(800, 600, 200, 112); client.drag(800, 600, 277, 140); client.mouseUp();
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
            check(client.saveError() == null, "No settings errors");
            System.out.println("PASS: search, per-widget colors and sizes, presets, snapping, resize handles, all settings pages at 12 viewport sizes, contrast and stable HUD layout caching.");
        } finally {
            try (java.util.stream.Stream<Path> paths = Files.walk(directory)) { paths.sorted(Comparator.reverseOrder()).forEach(path -> { try { Files.delete(path); } catch (Exception e) { throw new RuntimeException(e); } }); }
        }
    }
}
