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
        /** Draws part of a cosmetics texture key; adapters without image support skip it. */
        default void image(String texture, int x, int y, int width, int height, float u0, float v0, float u1, float v1) { }
        default void icon(Object icon, int x, int y, int size) { if (icon instanceof String) image((String) icon, x, y, size, size, 0, 0, 1, 1); }
    }
    public static final class HudRow {
        public final Object icon;
        public final String label;
        public HudRow(Object icon, String label) { this.icon = icon; this.label = label; }
    }
    public static String effectIcon(String description) {
        String[] parts = description.split("\\.", 3);
        return parts.length == 3 ? "effect/" + parts[1] + "/" + parts[2] : null;
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
        SHADOW("Text shadow", 0, 0, 1, 1), BORDER("Accent edge", 0, 0, 1, 1),
        ALIGN("Text alignment", 0, 0, 2, 1),
        TEXT_COLOR("Text color", 0xffeeeeee, 0, 0, 0), BG_COLOR("Background color", 0xff15181e, 0, 0, 0),
        ACCENT_COLOR("Accent / pressed color", 0xffc6db82, 0, 0, 0),
        KEY_SIZE("Key size", 22, 16, 36, 2), KEY_GAP("Key spacing", 3, 1, 8, 1),
        MOUSE_KEYS("Mouse buttons", 1, 0, 1, 1), SPACE_KEY("Space bar", 1, 0, 1, 1),
        KEY_CPS("CPS on mouse keys", 1, 0, 1, 1),
        ZOOM_FACTOR("Zoom strength", 4, 2, 10, 1), SMOOTH_ZOOM("Smooth zoom", 1, 0, 1, 1),
        SCROLL_ZOOM("Scroll to adjust", 1, 0, 1, 1), ZOOM_CINEMATIC("Cinematic camera", 1, 0, 1, 1),
        AUTO_WIDTH("Fit width to text", 1, 0, 1, 1),
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
    /** Keys the adapter lets players rebind from the Cairn menu. */
    public enum Bind { MENU("Menu key"), ZOOM("Zoom key");
        public final String label;
        Bind(String label) { this.label = label; }
    }
    static final Module[] MODULES = Module.values();
    static final Setting[] SETTINGS = Setting.values();
    static final int TEXT_HEIGHT = 18, MARGIN = 4, GAP = 2;
    static final int ACCENT = 0xffc6db82, TEXT = 0xfff1f0e8, MUTED = 0xffb4b8aa;
    static final class State {
        boolean enabled;
        final int[] settings = new int[SETTINGS.length];
        int x, y, flowX, flowY, contentWidth;
        boolean placed;
        String value = "", clipped = "";
        String clippedSource, measured;
        int clippedWidth = -1;
        int textWidth;
        int pressedTextColor;
        List<HudRow> rows = Collections.emptyList();
    }
    private final State[] states = new State[MODULES.length];
    private final Properties config = new Properties();
    private final Path path;
    private final Deque<Long> leftClicks = new ArrayDeque<Long>(), rightClicks = new ArrayDeque<Long>();
    private final SimpleDateFormat[] clocks = { new SimpleDateFormat("h:mm a"), new SimpleDateFormat("h:mm:ss a"), new SimpleDateFormat("HH:mm"), new SimpleDateFormat("HH:mm:ss") };
    private final Date date = new Date();
    private final CairnMenu menu;
    public final Cosmetics cosmetics;
    public final Friends friends;
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
    private float zoom = 1, scrollZoom = 1;
    private final int[] keyCodes = new int[Bind.values().length];
    private final String[] keyNames = new String[Bind.values().length];
    private boolean keysManaged, menuCloseRequested;
    private Bind listening;
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
            if (!config.containsKey("hud.style.version")) {
                if (state.settings[Setting.ACCENT_COLOR.ordinal()] == 0xffc6db82) state.settings[Setting.BORDER.ordinal()] = 0;
                if (state.settings[Setting.BG_COLOR.ordinal()] == 0xff1c211c) state.settings[Setting.BG_COLOR.ordinal()] = Setting.BG_COLOR.initial;
                if (state.settings[Setting.TEXT_COLOR.ordinal()] == 0xfff1f0e8) state.settings[Setting.TEXT_COLOR.ordinal()] = Setting.TEXT_COLOR.initial;
            }
            state.pressedTextColor = readable(state.settings[Setting.ACCENT_COLOR.ordinal()]);
            state.x = number(module.name() + ".x", -1);
            state.y = number(module.name() + ".y", -1);
            state.placed = state.x >= 0 && state.y >= 0 && !legacyDefault(module, state.x, state.y);
            if (module == Module.CROSSHAIR) state.placed = Boolean.parseBoolean(config.getProperty("CROSSHAIR.placed", "false"));
        }
        cosmetics = new Cosmetics(this);
        config.setProperty("hud.style.version", "2");
        friends = new Friends(this, cosmetics);
        menu = new CairnMenu(this);
    }
    String option(String key, String fallback) { return config.getProperty(key, fallback); }
    void setOption(String key, String value) {
        if (value == null) config.remove(key); else config.setProperty(key, value);
        save();
    }
    // Builds before the flow layout saved these fixed slots for every module, so they mean "never moved".
    private static boolean legacyDefault(Module module, int x, int y) {
        return module == Module.CPS ? x == 110 && y == 112 : x == 12 && y == module.ordinal() * 28 + 12;
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
    public void rows(Module module, List<HudRow> rows) {
        state(module).rows = Collections.unmodifiableList(new ArrayList<HudRow>(rows.subList(0, Math.min(8, rows.size()))));
        state(module).measured = null;
    }
    public void input(boolean forward, boolean back, boolean left, boolean right, boolean space, boolean mouseLeft, boolean mouseRight) {
        this.forward = forward; this.back = back; this.left = left; this.right = right; this.space = space; this.mouseLeft = mouseLeft; this.mouseRight = mouseRight;
    }
    public void click(boolean right) {
        Deque<Long> queue = right ? rightClicks : leftClicks;
        clicks(queue);
        if (queue.size() < 4096) queue.addLast(System.nanoTime());
    }
    public void mouseClick(int button, int leftButton, int rightButton) {
        if (button == leftButton || button == rightButton) click(button == rightButton);
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
        if (!held) scrollZoom = 1;
        float target = held && enabled(Module.ZOOM) ? setting(Module.ZOOM, Setting.ZOOM_FACTOR) * scrollZoom : 1;
        if (setting(Module.ZOOM, Setting.SMOOTH_ZOOM) == 0 || zoomTime == 0) zoom = target;
        else {
            double delta = Math.min(.1, (now - zoomTime) / 1_000_000_000.0);
            zoom += (target - zoom) * (float) (1 - Math.exp(-18 * delta));
            if (Math.abs(target - zoom) < .001f) zoom = target;
        }
        zoomTime = now;
        return zoom;
    }
    /** Returns true when the wheel adjusted an active zoom, so the adapter can keep it off the hotbar. */
    public boolean zoomScroll(boolean held, double amount) {
        if (!held || amount == 0 || !enabled(Module.ZOOM) || setting(Module.ZOOM, Setting.SCROLL_ZOOM) == 0) return false;
        float base = setting(Module.ZOOM, Setting.ZOOM_FACTOR);
        scrollZoom = Math.max(1 / base, Math.min(50 / base, scrollZoom * (amount > 0 ? 1.25f : .8f)));
        return true;
    }
    public boolean cinematicZoom() { return enabled(Module.ZOOM) && setting(Module.ZOOM, Setting.ZOOM_CINEMATIC) == 1; }
    /** Smooth-camera value for this tick; the player's own choice comes back once zoom ends. */
    public boolean smoothCamera(boolean zooming, boolean current) {
        boolean wanted = zooming && cinematicZoom();
        if (wanted && !cinematicActive) { cinematicPrevious = current; cinematicActive = true; }
        else if (!wanted && cinematicActive) { cinematicActive = false; return cinematicPrevious; }
        return cinematicActive || current;
    }
    public boolean menuKey(int code) { return keysManaged && listening == null && code == keyCode(Bind.MENU); }
    public boolean menuCloseKey(int code, int tabCode) { return !typing() && code != tabCode && menuKey(code); }
    /** Adapters that read raw key codes call this once; Forge keeps vanilla's own Controls screen instead. */
    public void manageKeys(int menuCode, String menuName, int zoomCode, String zoomName) {
        keysManaged = true;
        keyCodes[Bind.MENU.ordinal()] = number("key.MENU", menuCode); keyNames[Bind.MENU.ordinal()] = config.getProperty("key.MENU.name", menuName);
        keyCodes[Bind.ZOOM.ordinal()] = number("key.ZOOM", zoomCode); keyNames[Bind.ZOOM.ordinal()] = config.getProperty("key.ZOOM.name", zoomName);
    }
    private boolean cinematicActive, cinematicPrevious;
    public boolean keysManaged() { return keysManaged; }
    public int keyCode(Bind bind) { return keyCodes[bind.ordinal()]; }
    public String keyName(Bind bind) { return keyNames[bind.ordinal()]; }
    public boolean binding() { return listening != null; }
    Bind listening() { return listening; }
    void listen(Bind bind) { listening = bind; menu.blur(); }
    /** Assigns the pressed key to the bind waiting for input; a negative code cancels. */
    public void bind(int code, String name) {
        if (listening == null) return;
        if (code >= 0) {
            keyCodes[listening.ordinal()] = code; keyNames[listening.ordinal()] = name == null || name.isEmpty() ? "Key " + code : name;
            config.setProperty("key." + listening.name(), String.valueOf(code)); config.setProperty("key." + listening.name() + ".name", keyNames[listening.ordinal()]);
            save();
        }
        listening = null;
    }
    public boolean editing() { return editing; }
    public void requestMenuClose() { menuCloseRequested = true; }
    public boolean consumeMenuClose() { boolean close = menuCloseRequested; menuCloseRequested = false; return close; }
    public void enterEditor() { editing = true; editorTools = true; menu.blur(); menu.clearControls(); }
    public void openModules() { editing = false; dragging = null; menu.openModules(); }
    public void openCosmetics() { editing = false; dragging = null; menu.openCosmetics(); }
    public void openFriends() { editing = false; dragging = null; menu.openFriends(); }
    public void leaveEditor() { dragging = null; editing = false; menuCloseRequested = false; save(); }
    public boolean escape() {
        if (listening != null) { listening = null; return true; }
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
        state.x = state.y = -1; state.placed = false;
        if (module == Module.CROSSHAIR) config.setProperty("CROSSHAIR.placed", "false");
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
        if (setting(module, Setting.AUTO_WIDTH) == 1) return Math.round(autoWidth(state(module).contentWidth) * scale(module));
        return Math.round(setting(module, Setting.WIDTH) * scale(module));
    }
    // Rounded up to whole steps so a ticking counter does not make the panel wobble every frame.
    static int autoWidth(int textWidth) { return Math.min(Setting.WIDTH.max, (Math.max(24, textWidth) + 12 + 5) / 6 * 6); }
    private int widgetHeight(Module module) {
        if (module == Module.CROSSHAIR) return widgetWidth(module);
        if (!state(module).rows.isEmpty()) return Math.round(state(module).rows.size() * 20 * scale(module));
        if (module != Module.KEYSTROKES) return Math.round(TEXT_HEIGHT * scale(module));
        int size = Math.round(setting(module, Setting.KEY_SIZE) * scale(module)), gap = Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale(module)));
        int height = size * 2 + gap;
        if (setting(module, Setting.MOUSE_KEYS) == 1) height += Math.round((setting(module, Setting.KEY_SIZE) - 4) * scale(module)) + gap;
        if (setting(module, Setting.SPACE_KEY) == 1) height += Math.round(12 * scale(module)) + gap;
        return height;
    }
    private int x(Module module, int width) {
        State state = state(module);
        int x = state.placed ? state.x : module == Module.CROSSHAIR ? (width - widgetWidth(module)) / 2 : state.flowX;
        return clamp(x, 0, Math.max(0, width - widgetWidth(module)));
    }
    private int y(Module module, int height) {
        State state = state(module);
        int y = state.placed ? state.y : module == Module.CROSSHAIR ? (height - widgetHeight(module)) / 2 : state.flowY;
        return clamp(y, 0, Math.max(0, height - widgetHeight(module)));
    }
    int[] bounds(Module module) { return new int[] { x(module, viewportWidth), y(module, viewportHeight), widgetWidth(module), widgetHeight(module) }; }
    private void place(Module module, int x, int y) {
        State state = state(module); state.x = x; state.y = y; state.placed = true;
        if (module == Module.CROSSHAIR) config.setProperty("CROSSHAIR.placed", "true");
    }
    /** Widgets the player never moved stack top-left in module order and wrap into columns, so presets never overlap. */
    private void layout(Canvas canvas, int width, int height) {
        int columnX = MARGIN, cursor = MARGIN, columnWidth = 0;
        for (Module module : MODULES) {
            if (!enabled(module) || !module.hud || module == Module.CROSSHAIR) continue;
            State state = state(module);
            if (module != Module.KEYSTROKES) {
                String source = state.value.isEmpty() ? module.title : state.value;
                if (!source.equals(state.measured)) { state.contentWidth = canvas.textWidth(source); state.measured = source; }
                if (!state.rows.isEmpty()) {
                    state.contentWidth = 0;
                    for (HudRow row : state.rows) state.contentWidth = Math.max(state.contentWidth, canvas.textWidth(row.label) + 22);
                }
            }
            if (state.placed) continue;
            int w = widgetWidth(module), h = widgetHeight(module);
            if (cursor > MARGIN && cursor + h > height - MARGIN) { columnX += columnWidth + GAP; cursor = MARGIN; columnWidth = 0; }
            state.flowX = columnX; state.flowY = cursor;
            cursor += h + GAP; columnWidth = Math.max(columnWidth, w);
        }
    }
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
    void renderPreview(Canvas canvas, Module module, int x, int y, int width, int height) {
        float scale = scale(module);
        String value = state(module).value.isEmpty() ? module.title : state(module).value;
        int contentWidth = canvas.textWidth(value);
        if (!state(module).rows.isEmpty()) { contentWidth = 0; for (HudRow row : state(module).rows) contentWidth = Math.max(contentWidth, canvas.textWidth(row.label) + 22); }
        int w = module == Module.CROSSHAIR || module == Module.KEYSTROKES ? widgetWidth(module)
            : Math.round((setting(module, Setting.AUTO_WIDTH) == 1 ? autoWidth(contentWidth) : setting(module, Setting.WIDTH)) * scale);
        int h = widgetHeight(module);
        float fit = Math.min(1, Math.min(width / (float) Math.max(1, w), height / (float) Math.max(1, h)));
        scale *= fit; w = Math.round(w * fit); h = Math.round(h * fit);
        x += (width - w) / 2; y += (height - h) / 2;
        if (module == Module.CROSSHAIR) crosshair(canvas, x, y, scale);
        else if (module == Module.KEYSTROKES) keystrokes(canvas, x, y, module, scale);
        else if (!state(module).rows.isEmpty()) renderRows(canvas, module, x, y, w, h, scale);
        else {
            if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, w, h, background(module));
            if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, w), h, color(module, Setting.ACCENT_COLOR));
            String label = fit(canvas, value, Math.max(0, (int) (w / scale) - 12));
            int align = setting(module, Setting.ALIGN);
            int tx = align == 0 ? Math.round(6 * scale) : align == 1 ? (w - Math.round(canvas.textWidth(label) * scale)) / 2 : w - Math.round((canvas.textWidth(label) + 6) * scale);
            canvas.textScaled(label, x + Math.max(0, tx), y + Math.max(0, (h - Math.round(8 * scale)) / 2), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
        }
    }
    private void renderRows(Canvas canvas, Module module, int x, int y, int width, int height, float scale) {
        if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, width, height, background(module));
        if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, width), height, color(module, Setting.ACCENT_COLOR));
        int rowHeight = Math.max(1, Math.round(20 * scale)), iconSize = Math.max(1, Math.round(16 * scale));
        for (HudRow row : state(module).rows) {
            if (y + rowHeight > viewportHeight || height < rowHeight) break;
            canvas.icon(row.icon, x + Math.round(3 * scale), y + Math.round(2 * scale), iconSize);
            String label = fit(canvas, row.label, Math.max(0, (int) (width / scale) - 28));
            canvas.textScaled(label, x + Math.round(23 * scale), y + Math.round(6 * scale), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
            y += rowHeight; height -= rowHeight;
        }
    }
    public void renderHud(Canvas canvas, int width, int height) {
        viewportWidth = width; viewportHeight = height;
        counters();
        layout(canvas, width, height);
        for (Module module : MODULES) {
            if (!enabled(module) || !module.hud) continue;
            int x = x(module, width), y = y(module, height);
            float scale = Math.min(scale(module), Math.min(width / (float) Math.max(1, widgetWidth(module)) * scale(module), height / (float) Math.max(1, widgetHeight(module)) * scale(module)));
            int w = Math.min(width - x, widgetWidth(module)), h = Math.min(height - y, widgetHeight(module));
            if (module == Module.CROSSHAIR) crosshair(canvas, x, y, scale);
            else if (module == Module.KEYSTROKES) keystrokes(canvas, x, y, module, scale);
            else if (!state(module).rows.isEmpty()) renderRows(canvas, module, x, y, w, h, scale);
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
                canvas.textScaled(state.clipped, textX, y + Math.max(0, (h - Math.round(8 * scale)) / 2), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
            }
            if (editing && module == selected) {
                canvas.rect(x, y, w, 1, ACCENT); canvas.rect(x, y + h - 1, w, 1, ACCENT);
                canvas.rect(x, y, 1, h, ACCENT); canvas.rect(x + w - 1, y, 1, h, ACCENT);
                canvas.rect(x + w - 6, y + h - 6, 6, 6, ACCENT);
            }
        }
        friends.render(canvas, width, height);
    }
    public void renderMenu(Canvas canvas, int width, int height, int mouseX, int mouseY) {
        if (editing) {
            renderHud(canvas, width, height);
            if (!editorTools) return;
            if (dragging != null) {
                int x = x(dragging, width), y = y(dragging, height);
                canvas.rect(x, 30, 1, Math.max(0, height - 30), 0x554a6042);
                canvas.rect(0, y, width, 1, 0x554a6042);
            }
            canvas.rect(0, height - 26, width, 26, 0xf21c211c);
            canvas.text(fit(canvas, selected.title + " / " + setting(selected, Setting.SCALE) + "% / Tab: select / Arrows: move / +/-: size / Enter: settings", width - 24), 12, height - 17, TEXT);
            menu.editorControls(canvas, width, height, mouseX, mouseY);
        } else { menu.render(canvas, width, height, mouseX, mouseY); friends.render(canvas, width, height); }
    }
    public void editorKey(int dx, int dy, int scale) {
        if (scale != 0) setting(selected, Setting.SCALE, setting(selected, Setting.SCALE) + scale * 10);
        else {
            place(selected, x(selected, viewportWidth) + dx * (snap ? 4 : 1), y(selected, viewportHeight) + dy * (snap ? 4 : 1));
        }
        save();
    }
    public void snap() { snap = !snap; }
    public void editorSettings() { editing = false; menu.openSettings(selected); }
    public void mouseDown(int width, int height, int mouseX, int mouseY) {
        if (!editing) { menu.mouseDown(width, height, mouseX, mouseY); return; }
        if (editorTools && menu.editorClick(mouseX, mouseY)) return;
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
            int top = 0, bottom = editorTools ? 26 : 0;
            place(dragging, clamp(x, 0, Math.max(0, width - widgetWidth(dragging))), clamp(y, top, Math.max(top, height - bottom - widgetHeight(dragging))));
        }
    }
    public void mouseUp() { if (dragging != null) { dragging = null; save(); } }
    public void wheel(int amount) { menu.wheel(amount); }
    public String saveError() { return saveError; }
    public void save() {
        for (Module module : MODULES) {
            State state = state(module);
            config.setProperty(module.name() + ".enabled", String.valueOf(state.enabled));
            if (state.placed) { config.setProperty(module.name() + ".x", String.valueOf(state.x)); config.setProperty(module.name() + ".y", String.valueOf(state.y)); }
            else { config.remove(module.name() + ".x"); config.remove(module.name() + ".y"); }
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
