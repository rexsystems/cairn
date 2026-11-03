import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import java.nio.file.*;
import java.util.*;

public final class ClientCoreTest {
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    private static final class Canvas implements CairnClient.Canvas {
        final int width, height;
        Canvas(int width, int height) { this.width = width; this.height = height; }
        public void rect(int x, int y, int w, int h, int color) { check(x >= 0 && y >= 0 && x + w <= width && y + h <= height && w >= 0 && h >= 0, "Rectangle overflow at " + width + "x" + height); }
        public int textWidth(String text) { return text.length() * 6; }
        public void text(String text, int x, int y, int color) { check(x >= 0 && y >= 0 && x + textWidth(text) <= width && y + 8 <= height, "Text overflow: " + text + " at " + width + "x" + height); }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) { check(x >= 0 && y >= 0 && x + Math.round(textWidth(text) * scale) <= width && y + Math.round(8 * scale) <= height, "Scaled text overflow: " + text); }
    }
    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-core-test-");
        try {
            Path userMod = directory.resolve("user-mod.jar");
            Files.write(userMod, new byte[] {1, 2, 3});
            CairnClient client = new CairnClient(directory);
            check(client.enabled(Module.FPS), "FPS defaults on");
            client.toggle(Module.FPS);
            check(!new CairnClient(directory).enabled(Module.FPS), "Module choice persists");
            client.toggle(Module.FPS);
            for (Module module : Module.values()) if (!client.enabled(module)) client.toggle(module);
            client.value(Module.COORDINATES, "XYZ -123456789 255 123456789 with an extremely long label");
            for (int width : new int[] {320, 400, 640, 1280}) {
                for (int height : new int[] {240, 360, 720}) {
                    Canvas canvas = new Canvas(width, height);
                    for (int i = 0; i < Module.values().length; i++) {
                        client.key(1, false);
                        client.renderMenu(canvas, width, height, 0, 0);
                    }
                    client.renderHud(canvas, width, height);
                    client.enterEditor();
                    client.renderMenu(canvas, width, height, 0, 0);
                    client.mouseDown(width, height, 15, 15);
                    client.drag(width, height, width + 1000, height + 1000);
                    client.mouseUp();
                    client.renderHud(canvas, width, height);
                    client.leaveEditor();
                }
            }
            client.key(0, true);
            client.wheel(1000);
            client.renderMenu(new Canvas(320, 240), 320, 240, 0, 0);
            client.click(false); client.click(true);
            check(client.saveError() == null, "Settings save succeeds");
            check(Arrays.equals(Files.readAllBytes(userMod), new byte[] {1, 2, 3}), "User mod stays unchanged");
            System.out.println("PASS: configuration persistence, all module controls, keyboard selection, scrolling, HUD dragging and overflow at 12 viewport sizes; user files preserved.");
        } finally {
            try (java.util.stream.Stream<Path> paths = Files.walk(directory)) {
                paths.sorted(Comparator.reverseOrder()).forEach(path -> { try { Files.delete(path); } catch (Exception e) { throw new RuntimeException(e); } });
            }
        }
    }
}
