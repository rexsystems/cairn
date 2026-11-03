package app.cairn.client;

import java.io.*;
import java.nio.file.*;
import java.text.SimpleDateFormat;
import java.util.*;

public final class CairnClient {
    public interface Canvas {
        void rect(int x, int y, int width, int height, int color);
        void text(String text, int x, int y, int color);
        void textScaled(String text, int x, int y, int color, float scale, boolean shadow);
        int textWidth(String text);
    }
    public enum Module {
        FPS("FPS", "Frame counter", "HUD", true, true),
        COORDINATES("Coordinates", "Your position in the world", "HUD", true, true),
        PING("Ping", "Server latency", "HUD", true, true),
        KEYSTROKES("Keystrokes", "Live movement and mouse keys", "Controls", true, true),
        CPS("Click counter", "Left and right clicks per second", "Controls", true, false),
        ARMOR("Armor", "Equipped armor durability", "HUD", true, false),
        EFFECTS("Effects", "Active potion effects", "HUD", true, false),
        CLOCK("Clock", "Local time", "HUD", true, false),
        SPEED("Speed", "Horizontal blocks per second", "HUD", true, false),
        SPRINT("Toggle sprint", "Sprint without holding the key", "Controls", false, false),
        ZOOM("Zoom", "Hold your zoom key", "Visuals", false, true),
        CROSSHAIR("Crosshair", "Color, gap, size and center dot", "Visuals", true, false),
        DIRECTION("Direction", "Compass heading", "HUD", true, false),
        BIOME("Biome", "Current biome", "HUD", true, false),
        HELD_ITEM("Held item", "Item name and remaining durability", "HUD", true, false),
        FOOD("Food", "Hunger and saturation", "HUD", true, false),
        WORLD_TIME("World time", "Time of day in Minecraft", "HUD", true, false),
        SESSION("Session", "Time in this game session", "HUD", true, false),
        MEMORY("Memory", "Java heap usage", "HUD", true, false),
        SERVER("Server", "Current server or local world", "HUD", true, false);
        public final String title, description, category;
        public final boolean hud, defaultEnabled;
        Module(String title, String description, String category, boolean hud, boolean enabled) {
            this.title = title; this.description = description; this.category = category; this.hud = hud; this.defaultEnabled = enabled;
        }
    }
    public enum Setting {
        SCALE("Scale", 100, 50, 250, 10), WIDTH("Panel width", 150, 80, 320, 20),
        OPACITY("Background opacity", 80, 0, 100, 10), BACKGROUND("Background", 1, 0, 1, 1),
        SHADOW("Text shadow", 0, 0, 1, 1), BORDER("Accent edge", 1, 0, 1, 1),
        ALIGN("Text alignment", 0, 0, 2, 1),
        TEXT_COLOR("Text color", 0xfff1f0e8, 0, 0, 0), BG_COLOR("Background color", 0xff1c211c, 0, 0, 0),
        ACCENT_COLOR("Accent / pressed color", 0xffc6db82, 0, 0, 0),
        KEY_SIZE("Key size", 22, 16, 36, 2), KEY_GAP("Key spacing", 3, 1, 8, 1),
        MOUSE_KEYS("Mouse buttons", 1, 0, 1, 1), SPACE_KEY("Space bar", 1, 0, 1, 1),
        KEY_CPS("CPS on mouse keys", 1, 0, 1, 1),
        ZOOM_FACTOR("Zoom strength", 4, 2, 10, 1), SMOOTH_ZOOM("Smooth zoom", 1, 0, 1, 1),
        CROSS_SIZE("Arm length", 4, 2, 12, 1), CROSS_GAP("Center gap", 2, 0, 8, 1),
        CROSS_THICKNESS("Thickness", 1, 1, 4, 1), CROSS_DOT("Center dot", 1, 0, 1, 1),
        CROSS_HIDE_VANILLA("Replace vanilla crosshair", 1, 0, 1, 1),
        CLOCK_SECONDS("Show seconds", 0, 0, 1, 1), CLOCK_24H("24 hour clock", 1, 0, 1, 1),
        SPRINT_AUTO("Auto sprint", 0, 0, 1, 1), SPRINT_RESET("Reset when stopped", 0, 0, 1, 1);
        public final String label;
        public final int initial, min, max, step;
        Setting(String label, int initial, int min, int max, int step) { this.label = label; this.initial = initial; this.min = min; this.max = max; this.step = step; }
        public boolean color() { return step == 0; }
        public boolean toggle() { return !color() && max == 1; }
    }
    static final Module[] MODULES = Module.values();
    static final Setting[] SETTINGS = Setting.values();
    static final int ACCENT = 0xffc6db82, TEXT = 0xfff1f0e8, MUTED = 0xffb4b8aa;
    static final class State {
        boolean enabled;
        final int[] settings = new int[SETTINGS.length];
        int x, y;
        String value = "", clipped = "";
        String clippedSource;
        int clippedWidth = -1;
        int textWidth;
        int pressedTextColor;
    }
    private final State[] states = new State[MODULES.length];
    private final Properties config = new Properties();
    private final Path path;
    private final Deque<Long> leftClicks = new ArrayDeque<Long>(), rightClicks = new ArrayDeque<Long>();
    private final SimpleDateFormat[] clocks = { new SimpleDateFormat("h:mm a"), new SimpleDateFormat("h:mm:ss a"), new SimpleDateFormat("HH:mm"), new SimpleDateFormat("HH:mm:ss") };
    private final Date date = new Date();
    private final CairnMenu menu;
    private final long sessionStart = System.nanoTime();
    private long nextCounters, nextClock;
    private int leftCps, rightCps;
    private String leftCpsLabel = "0 CPS", rightCpsLabel = "0 CPS";
    private boolean forward, back, left, right, space, mouseLeft, mouseRight;
    private Module selected = Module.FPS, dragging;
    private boolean editing, resizing, snap = true, editorTools = true;
    private int dragX, dragY, initialWidth, initialScale;
    private int viewportWidth = 960, viewportHeight = 540;
    private long zoomTime;
    private float zoom = 1;
    private String saveError;

    public CairnClient(Path gameDir) {
        path = gameDir.resolve("config/cairn-client.properties");
        if (Files.exists(path)) try (InputStream stream = Files.newInputStream(path)) { config.load(stream); }
        catch (IOException e) { saveError = e.getMessage(); }
        for (Module module : MODULES) {
            State state = new State(); states[module.ordinal()] = state;
            state.enabled = Boolean.parseBoolean(config.getProperty(module.name() + ".enabled", String.valueOf(module.defaultEnabled)));
            for (Setting setting : SETTINGS) {
                int value = number(module.name() + "." + setting.name(), setting.initial);
                state.settings[setting.ordinal()] = setting.color() ? value | 0xff000000 : clamp(value, setting.min, setting.max);
            }
            state.pressedTextColor = readable(state.settings[Setting.ACCENT_COLOR.ordinal()]);
            state.x = number(module.name() + ".x", module == Module.CPS ? 110 : 12);
            state.y = number(module.name() + ".y", module == Module.CPS ? 112 : module.ordinal() * 28 + 12);
            if (module == Module.CROSSHAIR && !Boolean.parseBoolean(config.getProperty("CROSSHAIR.placed", "false"))) { state.x = -1; state.y = -1; }
        }
        menu = new CairnMenu(this);
    }
    static int clamp(int value, int min, int max) { return Math.max(min, Math.min(max, value)); }
    static int readable(int color) {
        double red = (color >> 16 & 255) / 255.0, green = (color >> 8 & 255) / 255.0, blue = (color & 255) / 255.0;
        red = red <= .04045 ? red / 12.92 : Math.pow((red + .055) / 1.055, 2.4);
        green = green <= .04045 ? green / 12.92 : Math.pow((green + .055) / 1.055, 2.4);
        blue = blue <= .04045 ? blue / 12.92 : Math.pow((blue + .055) / 1.055, 2.4);
        return .2126 * red + .7152 * green + .0722 * blue > .179 ? 0xff000000 : 0xffffffff;
    }
    private int number(String key, int fallback) { try { return Integer.parseInt(config.getProperty(key)); } catch (RuntimeException e) { return fallback; } }
    State state(Module module) { return states[module.ordinal()]; }
    public boolean enabled(Module module) { return state(module).enabled; }
    public int setting(Module module, Setting setting) { return state(module).settings[setting.ordinal()]; }
    public void setting(Module module, Setting setting, int value) {
        state(module).settings[setting.ordinal()] = setting.color() ? value | 0xff000000 : clamp(value, setting.min, setting.max);
        state(module).clippedWidth = -1;
        if (setting == Setting.ACCENT_COLOR) state(module).pressedTextColor = readable(state(module).settings[setting.ordinal()]);
        if (module == Module.CLOCK) nextClock = 0;
    }
    public void toggle(Module module) { state(module).enabled = !enabled(module); save(); }
    public void value(Module module, String value) { state(module).value = value == null ? "" : value; }
    public void input(boolean forward, boolean back, boolean left, boolean right, boolean space, boolean mouseLeft, boolean mouseRight) {
        this.forward = forward; this.back = back; this.left = left; this.right = right; this.space = space; this.mouseLeft = mouseLeft; this.mouseRight = mouseRight;
    }
    public void click(boolean right) {
        Deque<Long> queue = right ? rightClicks : leftClicks;
        clicks(queue);
        if (queue.size() < 4096) queue.addLast(System.nanoTime());
    }
    private int clicks(Deque<Long> queue) {
        long oldest = System.nanoTime() - 1_000_000_000L;
        while (!queue.isEmpty() && queue.peekFirst() < oldest) queue.removeFirst();
        return queue.size();
    }
    private void counters() {
        long now = System.nanoTime();
        if (now >= nextCounters) {
            leftCps = clicks(leftClicks); rightCps = clicks(rightClicks);
            leftCpsLabel = leftCps + " CPS"; rightCpsLabel = rightCps + " CPS";
            value(Module.CPS, leftCps + " / " + rightCps + " CPS");
            nextCounters = now + 100_000_000L;
        }
        if (now >= nextClock) {
            date.setTime(System.currentTimeMillis());
            if (enabled(Module.CLOCK)) value(Module.CLOCK, clocks[setting(Module.CLOCK, Setting.CLOCK_24H) * 2 + setting(Module.CLOCK, Setting.CLOCK_SECONDS)].format(date));
            long seconds = (now - sessionStart) / 1_000_000_000L;
            if (enabled(Module.SESSION)) value(Module.SESSION, seconds / 3600 + "h " + seconds / 60 % 60 + "m " + seconds % 60 + "s");
            Runtime runtime = Runtime.getRuntime();
            if (enabled(Module.MEMORY)) value(Module.MEMORY, (runtime.totalMemory() - runtime.freeMemory()) / 1048576 + " / " + runtime.maxMemory() / 1048576 + " MB");
            nextClock = now + 1_000_000_000L;
        }
    }
    public float zoomFactor(boolean held) {
        long now = System.nanoTime();
        float target = held && enabled(Module.ZOOM) ? setting(Module.ZOOM, Setting.ZOOM_FACTOR) : 1;
        if (setting(Module.ZOOM, Setting.SMOOTH_ZOOM) == 0 || zoomTime == 0) zoom = target;
        else {
            double delta = Math.min(.1, (now - zoomTime) / 1_000_000_000.0);
            zoom += (target - zoom) * (float) (1 - Math.exp(-18 * delta));
            if (Math.abs(target - zoom) < .001f) zoom = target;
        }
        zoomTime = now;
        return zoom;
    }
    public boolean editing() { return editing; }
    public void enterEditor() { editing = true; editorTools = true; menu.blur(); }
    public void leaveEditor() { dragging = null; editing = false; save(); }
    public boolean escape() {
        if (editing) { leaveEditor(); return true; }
        return menu.escape();
    }
    public void key(int direction, boolean activate) { menu.key(direction, activate); }
    public void tab() {
        if (!editing) { menu.tab(); return; }
        for (int i = 1; i <= MODULES.length; i++) {
            Module candidate = MODULES[(selected.ordinal() + i) % MODULES.length];
            if (candidate.hud && enabled(candidate)) { selected = candidate; return; }
        }
    }
    public void toggleEditorTools() { editorTools = !editorTools; }
    public boolean typing() { return menu.typing(); }
    public void type(char character) { menu.type(character); }
    public void backspace() { menu.backspace(); }
    public void focusSearch() { menu.focusSearch(); }
    public String query() { return menu.query(); }
    public List<Module> filteredModules() { return menu.filtered(); }
    Module selected() { return selected; }
    void select(Module module) { selected = module; }
    void preset(int preset) {
        for (Module module : MODULES) {
            State state = state(module);
            state.enabled = preset == 0 ? module == Module.FPS || module == Module.COORDINATES : preset == 1 ? module.defaultEnabled || module == Module.CPS || module == Module.ARMOR || module == Module.DIRECTION : module.defaultEnabled || module == Module.BIOME || module == Module.WORLD_TIME || module == Module.SESSION || module == Module.FOOD;
        }
        save();
    }
    void reset(Module module) {
        State state = state(module);
        for (Setting setting : SETTINGS) state.settings[setting.ordinal()] = setting.initial;
        state.pressedTextColor = readable(state.settings[Setting.ACCENT_COLOR.ordinal()]);
        state.x = module == Module.CPS ? 110 : 12; state.y = module == Module.CPS ? 112 : module.ordinal() * 28 + 12;
        if (module == Module.CROSSHAIR) { state.x = state.y = -1; config.setProperty("CROSSHAIR.placed", "false"); }
        state.clippedWidth = -1;
        save();
    }
    static String fit(Canvas canvas, String text, int width) {
        if (canvas.textWidth(text) <= width) return text;
        while (!text.isEmpty() && canvas.textWidth(text + "...") > width) text = text.substring(0, text.length() - 1);
        return text.isEmpty() ? "" : text + "...";
    }
    private float scale(Module module) { return setting(module, Setting.SCALE) / 100f; }
    private int widgetWidth(Module module) {
        if (module == Module.CROSSHAIR) return 2 * (Math.round(setting(module, Setting.CROSS_SIZE) * scale(module)) + Math.round(setting(module, Setting.CROSS_GAP) * scale(module))) + Math.max(1, Math.round(setting(module, Setting.CROSS_THICKNESS) * scale(module)));
        if (module == Module.KEYSTROKES) return 3 * Math.round(setting(module, Setting.KEY_SIZE) * scale(module)) + 2 * Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale(module)));
        return Math.round(setting(module, Setting.WIDTH) * scale(module));
    }
    private int widgetHeight(Module module) {
        if (module == Module.CROSSHAIR) return widgetWidth(module);
        if (module != Module.KEYSTROKES) return Math.round(22 * scale(module));
        int size = Math.round(setting(module, Setting.KEY_SIZE) * scale(module)), gap = Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale(module)));
        int height = size * 2 + gap;
        if (setting(module, Setting.MOUSE_KEYS) == 1) height += Math.round((setting(module, Setting.KEY_SIZE) - 4) * scale(module)) + gap;
        if (setting(module, Setting.SPACE_KEY) == 1) height += Math.round(12 * scale(module)) + gap;
        return height;
    }
    private int x(Module module, int width) { return clamp(module == Module.CROSSHAIR && state(module).x < 0 ? (width - widgetWidth(module)) / 2 : state(module).x, 0, Math.max(0, width - widgetWidth(module))); }
    private int y(Module module, int height) { return clamp(module == Module.CROSSHAIR && state(module).y < 0 ? (height - widgetHeight(module)) / 2 : state(module).y, 0, Math.max(0, height - widgetHeight(module))); }
    private int color(Module module, Setting setting) { return setting(module, setting); }
    private int background(Module module) { return (setting(module, Setting.OPACITY) * 255 / 100 << 24) | (color(module, Setting.BG_COLOR) & 0xffffff); }
    private void keyBox(Canvas canvas, int x, int y, int w, int h, boolean pressed, String label, Module module, float scale) {
        w = Math.min(w, viewportWidth - x); h = Math.min(h, viewportHeight - y);
        if (w <= 0 || h <= 0) return;
        int bg = pressed ? color(module, Setting.ACCENT_COLOR) : background(module);
        canvas.rect(x, y, w, h, bg);
        if (h < Math.round(9 * scale)) return;
        int textColor = pressed ? state(module).pressedTextColor : color(module, Setting.TEXT_COLOR);
        if (label.endsWith(" CPS") && canvas.textWidth(label) * scale > w - 4) label = label.substring(0, label.length() - 4);
        label = fit(canvas, label, (int) ((w - 4) / scale));
        canvas.textScaled(label, x + Math.max(1, (w - Math.round(canvas.textWidth(label) * scale)) / 2), y + Math.max(0, (h - Math.round(9 * scale)) / 2), textColor, scale, false);
    }
    private void keystrokes(Canvas canvas, int x, int y, Module module, float scale) {
        int size = Math.round(setting(module, Setting.KEY_SIZE) * scale), gap = Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale));
        int total = size * 3 + gap * 2;
        keyBox(canvas, x + size + gap, y, size, size, forward, "W", module, scale);
        int row = y + size + gap;
        keyBox(canvas, x, row, size, size, left, "A", module, scale);
        keyBox(canvas, x + size + gap, row, size, size, back, "S", module, scale);
        keyBox(canvas, x + (size + gap) * 2, row, size, size, right, "D", module, scale);
        row += size + gap;
        if (setting(module, Setting.MOUSE_KEYS) == 1) {
            int h = Math.round((setting(module, Setting.KEY_SIZE) - 4) * scale), half = (total - gap) / 2;
            keyBox(canvas, x, row, half, h, mouseLeft, setting(module, Setting.KEY_CPS) == 1 ? leftCpsLabel : "LMB", module, scale);
            keyBox(canvas, x + half + gap, row, total - half - gap, h, mouseRight, setting(module, Setting.KEY_CPS) == 1 ? rightCpsLabel : "RMB", module, scale);
            row += h + gap;
        }
        if (setting(module, Setting.SPACE_KEY) == 1) keyBox(canvas, x, row, total, Math.round(12 * scale), space, "SPACE", module, scale);
    }
    private void crosshair(Canvas canvas, int x, int y, float scale) {
        Module module = Module.CROSSHAIR;
        int size = Math.round(setting(module, Setting.CROSS_SIZE) * scale), gap = Math.round(setting(module, Setting.CROSS_GAP) * scale), t = Math.max(1, Math.round(setting(module, Setting.CROSS_THICKNESS) * scale)), c = color(module, Setting.ACCENT_COLOR);
        int cx = x + size + gap + t / 2, cy = y + size + gap + t / 2;
        canvas.rect(cx - gap - t / 2 - size, cy - t / 2, size, t, c); canvas.rect(cx + gap + (t + 1) / 2, cy - t / 2, size, t, c);
        canvas.rect(cx - t / 2, cy - gap - t / 2 - size, t, size, c); canvas.rect(cx - t / 2, cy + gap + (t + 1) / 2, t, size, c);
        if (setting(module, Setting.CROSS_DOT) == 1) canvas.rect(cx - t / 2, cy - t / 2, t, t, c);
    }
    public void renderHud(Canvas canvas, int width, int height) {
        viewportWidth = width; viewportHeight = height;
        counters();
        for (Module module : MODULES) {
            if (!enabled(module) || !module.hud) continue;
            int x = x(module, width), y = y(module, height);
            float scale = Math.min(scale(module), Math.min(width / (float) Math.max(1, widgetWidth(module)) * scale(module), height / (float) Math.max(1, widgetHeight(module)) * scale(module)));
            int w = Math.min(width - x, widgetWidth(module)), h = Math.min(height - y, widgetHeight(module));
            if (module == Module.CROSSHAIR) crosshair(canvas, x, y, scale);
            else if (module == Module.KEYSTROKES) keystrokes(canvas, x, y, module, scale);
            else {
                if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, w, h, background(module));
                if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, w), h, color(module, Setting.ACCENT_COLOR));
                State state = state(module);
                String source = state.value.isEmpty() ? module.title : state.value;
                int available = (int) ((w - 12 * scale) / scale);
                if (state.clippedWidth != available || !source.equals(state.clippedSource)) {
                    state.clipped = fit(canvas, source, available); state.clippedSource = source; state.clippedWidth = available; state.textWidth = canvas.textWidth(state.clipped);
                }
                int textX = setting(module, Setting.ALIGN) == 0 ? x + Math.round(6 * scale) : setting(module, Setting.ALIGN) == 1 ? x + Math.max(0, (w - Math.round(state.textWidth * scale)) / 2) : x + Math.max(0, w - Math.round((state.textWidth + 6) * scale));
                canvas.textScaled(state.clipped, textX, y + Math.round(6 * scale), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
            }
            if (editing && module == selected) {
                canvas.rect(x, y, w, 1, ACCENT); canvas.rect(x, y + h - 1, w, 1, ACCENT);
                canvas.rect(x, y, 1, h, ACCENT); canvas.rect(x + w - 1, y, 1, h, ACCENT);
                canvas.rect(x + w - 6, y + h - 6, 6, 6, ACCENT);
            }
        }
    }
    public void renderMenu(Canvas canvas, int width, int height, int mouseX, int mouseY) {
        if (editing) {
            renderHud(canvas, width, height);
            if (!editorTools) return;
            canvas.rect(0, 0, width, 30, 0xf21c211c);
            canvas.text(fit(canvas, "HUD / Drag: move. Corner: resize. G: snap " + (snap ? "ON" : "OFF") + ". H: hide tools. Esc: modules", width - 24), 12, 11, TEXT);
            if (dragging != null) {
                int x = x(dragging, width), y = y(dragging, height);
                canvas.rect(x, 30, 1, Math.max(0, height - 30), 0x554a6042);
                canvas.rect(0, y, width, 1, 0x554a6042);
            }
            canvas.rect(0, height - 26, width, 26, 0xf21c211c);
            canvas.text(fit(canvas, selected.title + " / " + setting(selected, Setting.SCALE) + "% / Tab: select / Arrows: move / +/-: size / Enter: settings", width - 24), 12, height - 17, TEXT);
        } else menu.render(canvas, width, height, mouseX, mouseY);
    }
    public void editorKey(int dx, int dy, int scale) {
        if (scale != 0) setting(selected, Setting.SCALE, setting(selected, Setting.SCALE) + scale * 10);
        else {
            state(selected).x = x(selected, viewportWidth) + dx * (snap ? 4 : 1);
            state(selected).y = y(selected, viewportHeight) + dy * (snap ? 4 : 1);
            if (selected == Module.CROSSHAIR) config.setProperty("CROSSHAIR.placed", "true");
        }
        save();
    }
    public void snap() { snap = !snap; }
    public void editorSettings() { editing = false; menu.openSettings(selected); }
    public void mouseDown(int width, int height, int mouseX, int mouseY) {
        if (!editing) { menu.mouseDown(width, height, mouseX, mouseY); return; }
        if (editorTools && (mouseY < 30 || mouseY >= height - 26)) return;
        for (int i = MODULES.length - 1; i >= 0; i--) {
            Module module = MODULES[i];
            if (!enabled(module) || !module.hud) continue;
            int x = x(module, width), y = y(module, height), w = widgetWidth(module), h = widgetHeight(module);
            if (mouseX >= x && mouseX < x + w && mouseY >= y && mouseY < y + h) {
                selected = dragging = module; dragX = mouseX - x; dragY = mouseY - y;
                resizing = mouseX >= x + w - 8 && mouseY >= y + h - 8;
                initialWidth = w; initialScale = setting(module, Setting.SCALE);
                return;
            }
        }
    }
    public void drag(int width, int height, int mouseX, int mouseY) {
        if (dragging == null) return;
        if (resizing) {
            int newWidth = mouseX - x(dragging, width);
            setting(dragging, Setting.SCALE, Math.round(newWidth * initialScale / (float) Math.max(1, initialWidth) / 5) * 5);
        } else {
            int x = mouseX - dragX, y = mouseY - dragY;
            if (snap) { x = Math.round(x / 4f) * 4; y = Math.round(y / 4f) * 4; }
            state(dragging).x = clamp(x, 0, Math.max(0, width - widgetWidth(dragging)));
            int top = editorTools ? 30 : 0, bottom = editorTools ? 26 : 0;
            state(dragging).y = clamp(y, top, Math.max(top, height - bottom - widgetHeight(dragging)));
            if (dragging == Module.CROSSHAIR) config.setProperty("CROSSHAIR.placed", "true");
        }
    }
    public void mouseUp() { if (dragging != null) { dragging = null; save(); } }
    public void wheel(int amount) { menu.wheel(amount); }
    public String saveError() { return saveError; }
    public void save() {
        for (Module module : MODULES) {
            State state = state(module);
            config.setProperty(module.name() + ".enabled", String.valueOf(state.enabled));
            config.setProperty(module.name() + ".x", String.valueOf(state.x)); config.setProperty(module.name() + ".y", String.valueOf(state.y));
            for (Setting setting : SETTINGS) config.setProperty(module.name() + "." + setting.name(), String.valueOf(state.settings[setting.ordinal()]));
        }
        try {
            Files.createDirectories(path.getParent());
            Path temporary = Files.createTempFile(path.getParent(), "cairn-client-", ".tmp");
            try {
                try (OutputStream stream = Files.newOutputStream(temporary)) { config.store(stream, "Cairn Client"); }
                try { Files.move(temporary, path, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE); }
                catch (AtomicMoveNotSupportedException e) { Files.move(temporary, path, StandardCopyOption.REPLACE_EXISTING); }
            } finally { Files.deleteIfExists(temporary); }
            saveError = null;
        } catch (IOException e) { saveError = e.getMessage(); System.err.println("[Cairn Client] Could not save settings: " + e); }
    }
}
