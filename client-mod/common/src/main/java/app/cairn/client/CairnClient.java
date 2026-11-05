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
        SERVER("Server", "Current server or local world", "HUD", true, false),
        BLOCK_INDICATOR("Block indicator", "The block you are looking at", "HUD", true, false),
        REACH("Reach", "Distance to the block or entity under the crosshair", "HUD", true, false),
        TPS("TPS", "Server tick rate estimated from time packets", "HUD", true, false),
        COMBO("Combo", "Hits landed before the combo resets", "HUD", true, false),
        DEATH_INFO("Death info", "Last death message and coordinates", "HUD", true, false),
        HORSES("Horses", "Mount jump strength and health", "HUD", true, false),
        BOSS_BAR("Boss bar", "Active boss bar name and progress", "HUD", true, false),
        HEARTS("Hearts", "Health and absorption", "HUD", true, false),
        ARMOR_POINTS("Armor points", "Armor defense points", "HUD", true, false),
        ATTACK_INDICATOR("Attack indicator", "Attack cooldown progress", "HUD", true, false),
        ELYTRA("Elytra", "Chest elytra durability while flying", "HUD", true, false),
        SHIELD("Shield", "Offhand or main-hand shield durability", "HUD", true, false),
        TOTEM("Totem", "Totems in inventory and last pop", "HUD", true, false),
        LIGHT_LEVEL("Light level", "Block and sky light at your feet", "HUD", true, false),
        SCOREBOARD("Scoreboard", "Sidebar scoreboard lines", "HUD", true, false),
        TNT_TIMER("TNT timer", "Fuse of nearby primed TNT", "HUD", true, false),
        ITEM_INFO("Item info", "Nearest dropped item name and age", "HUD", true, false),
        ITEM_DESPAWN("Item despawn", "Seconds until nearby item entities despawn", "HUD", true, false),
        PACK_DISPLAY("Pack display", "Active resource pack names", "HUD", true, false),
        INVENTORY("Inventory", "Your inventory items in a nine-column grid", "HUD", true, false),
        SYSTEM_RESOURCES("System resources", "Heap plus available processors", "HUD", true, false),
        SATURATION("Saturation", "Food saturation only", "HUD", true, false),
        UHC_OVERLAY("UHC overlay", "Health with absorption called out for UHC", "HUD", true, false),
        TEAM_TRACKER("Team tracker", "Scoreboard teammates nearby", "HUD", true, false),
        BLOCK_OVERLAY("Block overlay", "Colored outline on the targeted block", "Visuals", false, false),
        BRIGHTNESS("Brightness", "Raises gamma while enabled", "Visuals", false, false),
        FOV_CHANGER("FOV changer", "Multiplies field of view", "Visuals", false, false),
        HITBOXES("Hitboxes", "Draw entity collision boxes", "Visuals", false, false),
        HIT_INDICATOR("Hit indicator", "Flash when you deal damage", "Visuals", false, false),
        TIME_CHANGER("Time changer", "Client-only time of day override", "Visuals", false, false),
        WEATHER_CHANGER("Weather changer", "Client-only clear or rain override", "Visuals", false, false),
        NICK_HIDER("Nick hider", "Replace your name in the HUD with a cover", "Visuals", false, false),
        DROP_PREVENTION("Drop prevention", "Block the drop-item key while enabled", "Controls", false, false),
        AUTOHIDE_HUD("Autohide HUD", "Hide Cairn HUD widgets while inventory is open", "HUD", false, false),
        PERSPECTIVE("Perspective", "Cycle first/third person with a Cairn bind", "Visuals", false, false),
        SNAPLOOK("Snaplook", "Hold to look behind, release to restore", "Visuals", false, false),
        AUTO_PERSPECTIVE("Auto perspective", "Third person while flying with elytra", "Visuals", false, false),
        VIEWMODEL("ViewModel", "Scale the held-item viewmodel", "Visuals", false, false),
        PARTICLES("Particles", "Reduce or keep vanilla particle density", "Visuals", false, false),
        NAMETAGS("Nametags", "Show or hide other players' nametags", "Visuals", false, false),
        RECONNECT("Reconnect", "Reconnect to the last multiplayer server", "Controls", false, false),
        TOAST_CONTROL("Toast control", "Mute vanilla recipe and advancement toasts", "Controls", false, false),
        UI_SCALING("UI scaling", "Override GUI scale while in-game menus open", "Controls", false, false),
        CULL_LOGS("Cull logs", "Remove old archived Minecraft logs", "Controls", false, false),
        CUSTOM_F3("Custom F3", "Hide the vanilla debug HUD while Cairn is active", "Controls", false, false),
        CHAT_TIMESTAMPS("Chat timestamps", "Prefix chat lines with the local time", "Controls", false, false),
        WAYPOINTS("Waypoints", "Show last death coordinates as a waypoint", "HUD", true, false),
        MOB_OVERLAY("Mob overlay", "Show mob health above nearby mobs", "Visuals", false, false),
        LOOT_BEAMS("Loot beams", "Beam markers on nearby item entities", "Visuals", false, false),
        TITLE_TWEAKER("Title tweaker", "Use the Cairn title screen instead of vanilla", "Controls", false, true),
        FREELOOK("Freelook", "Hold to orbit the camera without turning your player", "Visuals", false, false);
        public final String title, description, category;
        public final boolean hud, defaultEnabled;
        Module(String title, String description, String category, boolean hud, boolean enabled) {
            this.title = title; this.description = description; this.category = category; this.hud = hud; this.defaultEnabled = enabled;
        }
    }
    public enum Setting {
        SCALE("Scale", 100, 50, 250, 10), WIDTH("Panel width", 150, 80, 320, 20),
        HEIGHT("Panel height", 18, 8, 120, 1), PADDING_X("Horizontal padding", 6, 0, 24, 1), PADDING_Y("Vertical padding", 2, 0, 24, 1),
        OPACITY("Background opacity", 80, 0, 100, 1), BACKGROUND("Background", 1, 0, 1, 1),
        TEXT_ALPHA("Text opacity", 100, 0, 100, 1), ACCENT_ALPHA("Accent opacity", 100, 0, 100, 1),
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
        SPRINT_AUTO("Auto sprint", 0, 0, 1, 1), SPRINT_RESET("Reset when stopped", 0, 0, 1, 1),
        FOV_FACTOR("FOV percent", 100, 30, 200, 5), BRIGHTNESS_GAMMA("Gamma percent", 150, 100, 300, 10),
        OVERLAY_COLOR("Overlay color", 0xffffffff, 0, 0, 0), TIME_OF_DAY("Time of day", 6000, 0, 23999, 500),
        WEATHER_MODE("Weather", 0, 0, 2, 1), VIEWMODEL_SCALE("ViewModel scale", 100, 50, 150, 5),
        PARTICLE_RATE("Particle percent", 100, 0, 100, 10), UI_SCALE("GUI scale", 2, 1, 4, 1),
        NICK_COVER("Cover name length", 8, 3, 16, 1), COMBO_TIMEOUT("Combo timeout ms", 2000, 500, 5000, 100),
        AUTOHIDE_INVENTORY("Hide in inventories", 1, 0, 1, 1), SHOW_NAMETAGS("Show nametags", 1, 0, 1, 1),
        ARMOR_INVERT("Boots first", 0, 0, 1, 1), INVENTORY_HOTBAR("Include hotbar", 1, 0, 1, 1),
        SLOT_BACKGROUND("Slot backgrounds", 1, 0, 1, 1),
        LOG_DAYS("Keep logs for days", 3, 1, 30, 1), MIN_LOGS("Minimum archives to keep", 3, 1, 50, 1);
        public final String label;
        public final int initial, min, max, step;
        Setting(String label, int initial, int min, int max, int step) { this.label = label; this.initial = initial; this.min = min; this.max = max; this.step = step; }
        public boolean color() { return step == 0; }
        public boolean toggle() { return !color() && max == 1; }
    }
    /** Keys the adapter lets players rebind from the Cairn menu. */
    public enum Bind { MENU("Menu key"), ZOOM("Zoom key"), PERSPECTIVE("Perspective"), SNAPLOOK("Snaplook"), RECONNECT("Reconnect"), FREELOOK("Freelook");
        public final String label;
        Bind(String label) { this.label = label; }
    }
    static final Module[] MODULES = Module.values();
    static final Setting[] SETTINGS = Setting.values();
    static final int MARGIN = 4, GAP = 2;
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
        state(module).rows = Collections.unmodifiableList(new ArrayList<HudRow>(rows.subList(0, Math.min(module == Module.SCOREBOARD ? 16 : 8, rows.size()))));
        state(module).measured = null;
    }
    private List<HudRow> inventory = Collections.emptyList();
    /** Slots 0–8 are the hotbar; slots 9–35 are the three inventory rows. */
    public void inventory(List<HudRow> slots) {
        if (slots.size() != 36) throw new IllegalArgumentException("Expected 36 inventory slots");
        inventory = Collections.unmodifiableList(new ArrayList<HudRow>(slots));
    }
    public void blockTarget(Object icon, String name) {
        boolean present = name != null && !name.isEmpty();
        rows(Module.BLOCK_INDICATOR, present ? Collections.singletonList(new HudRow(icon, name)) : Collections.<HudRow>emptyList());
        value(Module.BLOCK_INDICATOR, present ? name : "No block");
    }
    /** Distance in blocks to the current hit; negative clears to "No target". */
    public void reach(double blocks) {
        value(Module.REACH, blocks < 0 ? "No target" : String.format(Locale.ROOT, "%.2f blocks", blocks));
    }
    private long lastServerTime, lastServerPacket; private double estimatedTps = -1;
    public void serverTime(long gameTime) { serverTime(gameTime, System.nanoTime()); }
    void serverTime(long gameTime, long now) {
        if (lastServerPacket != 0 && gameTime > lastServerTime && now > lastServerPacket) {
            double sample = Math.min(20, (gameTime - lastServerTime) * 1_000_000_000.0 / (now - lastServerPacket));
            estimatedTps = estimatedTps < 0 ? sample : estimatedTps * .75 + sample * .25;
        }
        lastServerTime = gameTime; lastServerPacket = now;
    }
    private int combo; private long lastComboHit; private int totemPops; private long hitFlashUntil;
    private String lastDeath = "No death yet", deathWaypoint = "";
    private boolean snaplookHeld, reconnectRequested, perspectivePressed, inventoryOpen;
    private Double brightnessPrevious;
    private long nextMaintenance;
    private static final java.util.concurrent.ExecutorService MAINTENANCE = java.util.concurrent.Executors.newSingleThreadExecutor(task -> { Thread thread = new Thread(task, "Cairn log cleanup"); thread.setDaemon(true); return thread; });
    public void inventoryOpen(boolean open) { inventoryOpen = open; }
    /** Updates timed HUD values; TPS comes only from server time packets. */
    public void clientTick() {
        long now = System.nanoTime();
        if (enabled(Module.CULL_LOGS) && now >= nextMaintenance) {
            nextMaintenance = now + 3_600_000_000_000L;
            final int keep = setting(Module.CULL_LOGS, Setting.MIN_LOGS), days = setting(Module.CULL_LOGS, Setting.LOG_DAYS);
            MAINTENANCE.execute(() -> {
                try { LogRetention.prune(path.getParent().getParent().resolve("logs"), keep, days, System.currentTimeMillis()); }
                catch (IOException failure) { System.err.println("[Cairn Client] Could not clean archived logs: " + failure.getMessage()); }
            });
        }
        if (enabled(Module.TPS)) value(Module.TPS, estimatedTps < 0 || now - lastServerPacket > 15_000_000_000L ? "Waiting for server ticks" : String.format(Locale.ROOT, "%.1f TPS", estimatedTps));
        if (enabled(Module.DEATH_INFO)) value(Module.DEATH_INFO, lastDeath);
        if (enabled(Module.SYSTEM_RESOURCES)) {
            Runtime runtime = Runtime.getRuntime();
            value(Module.SYSTEM_RESOURCES, (runtime.totalMemory() - runtime.freeMemory()) / 1048576 + " MB / " + Runtime.getRuntime().availableProcessors() + " threads");
        }
        if (enabled(Module.COMBO) && combo > 0 && now - lastComboHit > setting(Module.COMBO, Setting.COMBO_TIMEOUT) * 1_000_000L) {
            combo = 0; value(Module.COMBO, "0 combo");
        }
        if (enabled(Module.COMBO) && combo == 0) value(Module.COMBO, "0 combo");
        if (enabled(Module.WAYPOINTS)) value(Module.WAYPOINTS, deathWaypoint.isEmpty() ? "No waypoint" : deathWaypoint);
    }
    public void comboHit() {
        long now = System.nanoTime();
        if (now - lastComboHit > setting(Module.COMBO, Setting.COMBO_TIMEOUT) * 1_000_000L) combo = 0;
        combo++; lastComboHit = now; hitFlashUntil = now + 250_000_000L;
        if (enabled(Module.COMBO)) value(Module.COMBO, combo + " combo");
    }
    public void comboReset() { combo = 0; lastComboHit = 0; if (enabled(Module.COMBO)) value(Module.COMBO, "0 combo"); }
    private int attackedEntity = -1;
    private long attackedAt;
    private boolean wasDead;
    public void attacked(int entity) { attackedEntity = entity; attackedAt = System.nanoTime(); }
    public void entityHurt(int entity, boolean self) {
        if (self) { comboReset(); attackedEntity = -1; }
        else if (entity == attackedEntity && System.nanoTime() - attackedAt < 2_000_000_000L) { comboHit(); attackedEntity = -1; }
    }
    public void life(boolean dead, int x, int y, int z) {
        if (dead && !wasDead) death(null, x, y, z);
        wasDead = dead;
    }
    public void leftWorld() {
        lastServerPacket = 0; estimatedTps = -1; attackedEntity = -1; wasDead = false;
        inventory = Collections.emptyList();
    }
    public boolean hitFlash() { return enabled(Module.HIT_INDICATOR) && System.nanoTime() < hitFlashUntil; }
    public void death(String message, int x, int y, int z) {
        lastDeath = message == null || message.isEmpty() ? "Died at " + x + " " + y + " " + z : message;
        deathWaypoint = "Death " + x + " " + y + " " + z;
        if (enabled(Module.DEATH_INFO)) value(Module.DEATH_INFO, lastDeath);
        if (enabled(Module.WAYPOINTS)) value(Module.WAYPOINTS, deathWaypoint);
    }
    public void totemPop() { totemPops++; if (enabled(Module.TOTEM)) value(Module.TOTEM, state(Module.TOTEM).value.replaceAll(" · \\d+ pops$", "") + " · " + totemPops + " pops"); }
    public void hearts(float health, float max, float absorption) {
        if (enabled(Module.HEARTS)) value(Module.HEARTS, String.format(Locale.ROOT, "%.1f / %.1f HP%s", health, max, absorption > 0 ? String.format(Locale.ROOT, " +%.1f abs", absorption) : ""));
        if (enabled(Module.UHC_OVERLAY)) value(Module.UHC_OVERLAY, String.format(Locale.ROOT, "%.1f HP%s", health, absorption > 0 ? String.format(Locale.ROOT, " | %.1f abs", absorption) : ""));
    }
    public void armorPoints(int points) { if (enabled(Module.ARMOR_POINTS)) value(Module.ARMOR_POINTS, points + " armor"); }
    public void attackCooldown(float progress) { if (enabled(Module.ATTACK_INDICATOR)) value(Module.ATTACK_INDICATOR, progress >= 1 ? "Ready" : String.format(Locale.ROOT, "%d%%", Math.round(progress * 100))); }
    public void horse(float health, float max, double jump) {
        if (enabled(Module.HORSES)) value(Module.HORSES, String.format(Locale.ROOT, "Mount %.1f/%.1f%s", health, max, jump > 0 ? String.format(Locale.ROOT, " · jump %.2f", jump) : ""));
    }
    public void clearHorse() { if (enabled(Module.HORSES)) value(Module.HORSES, "Not mounted"); }
    public void bossBar(String name, float progress) {
        if (!enabled(Module.BOSS_BAR)) return;
        value(Module.BOSS_BAR, name == null || name.isEmpty() ? "No boss bar" : String.format(Locale.ROOT, "%s · %d%%", name, Math.round(Math.max(0, Math.min(1, progress)) * 100)));
    }
    public void lightLevel(int block, int sky) { if (enabled(Module.LIGHT_LEVEL)) value(Module.LIGHT_LEVEL, "Block " + block + " / Sky " + sky); }
    public void scoreboardLines(List<String> lines) {
        if (!enabled(Module.SCOREBOARD)) return;
        if (lines == null || lines.isEmpty()) { value(Module.SCOREBOARD, "No sidebar"); rows(Module.SCOREBOARD, Collections.<HudRow>emptyList()); return; }
        List<HudRow> out = new ArrayList<HudRow>();
        for (int i = 0; i < Math.min(16, lines.size()); i++) out.add(new HudRow(null, lines.get(i)));
        rows(Module.SCOREBOARD, out); value(Module.SCOREBOARD, lines.get(0));
    }
    public void tntFuse(int ticksLeft) { if (enabled(Module.TNT_TIMER)) value(Module.TNT_TIMER, ticksLeft < 0 ? "No TNT" : String.format(Locale.ROOT, "%.2fs", ticksLeft / 20.0)); }
    public void itemEntity(String name, int ageTicks) {
        if (enabled(Module.ITEM_INFO)) value(Module.ITEM_INFO, name == null ? "No item" : name + " · " + ageTicks / 20 + "s");
        if (enabled(Module.ITEM_DESPAWN)) value(Module.ITEM_DESPAWN, name == null ? "No item" : Math.max(0, (6000 - ageTicks) / 20) + "s left");
    }
    public void packNames(String names) { if (enabled(Module.PACK_DISPLAY)) value(Module.PACK_DISPLAY, names == null || names.isEmpty() ? "Default pack" : names); }
    public void inventorySlots(int emptyHotbar, int emptyInv) { if (enabled(Module.INVENTORY)) value(Module.INVENTORY, "Hotbar " + emptyHotbar + " free · Inv " + emptyInv + " free"); }
    public void saturation(float saturation) { if (enabled(Module.SATURATION)) value(Module.SATURATION, String.format(Locale.ROOT, "%.1f sat", saturation)); }
    public void elytra(int remaining, boolean flying) { if (enabled(Module.ELYTRA)) value(Module.ELYTRA, remaining < 0 ? "No elytra" : (flying ? "Flying · " : "") + remaining + " dur"); }
    public void shield(int remaining) { if (enabled(Module.SHIELD)) value(Module.SHIELD, remaining < 0 ? "No shield" : remaining + " dur"); }
    public void totems(int count) { if (enabled(Module.TOTEM)) value(Module.TOTEM, count + " totem" + (count == 1 ? "" : "s") + (totemPops > 0 ? " · " + totemPops + " pops" : "")); }
    public void teamTracker(List<String> teammates) {
        if (!enabled(Module.TEAM_TRACKER)) return;
        if (teammates == null || teammates.isEmpty()) { value(Module.TEAM_TRACKER, "No teammates"); rows(Module.TEAM_TRACKER, Collections.<HudRow>emptyList()); return; }
        List<HudRow> out = new ArrayList<HudRow>();
        for (int i = 0; i < Math.min(8, teammates.size()); i++) out.add(new HudRow(null, teammates.get(i)));
        rows(Module.TEAM_TRACKER, out); value(Module.TEAM_TRACKER, teammates.size() + " nearby");
    }
    public float fovFactor() { return enabled(Module.FOV_CHANGER) ? setting(Module.FOV_CHANGER, Setting.FOV_FACTOR) / 100f : 1f; }
    public float brightnessGamma(float current) {
        if (!enabled(Module.BRIGHTNESS)) {
            if (brightnessPrevious != null) { float restore = brightnessPrevious.floatValue(); brightnessPrevious = null; return restore; }
            return current;
        }
        if (brightnessPrevious == null) brightnessPrevious = Double.valueOf(current);
        return setting(Module.BRIGHTNESS, Setting.BRIGHTNESS_GAMMA) / 100f;
    }
    public int overlayColor() { return color(Module.BLOCK_OVERLAY, Setting.OVERLAY_COLOR); }
    public boolean drawBlockOverlay() { return enabled(Module.BLOCK_OVERLAY); }
    public boolean drawHitboxes() { return enabled(Module.HITBOXES); }
    public boolean drawMobOverlay() { return enabled(Module.MOB_OVERLAY); }
    public boolean drawLootBeams() { return enabled(Module.LOOT_BEAMS); }
    public boolean hideNametags() { return enabled(Module.NAMETAGS) && setting(Module.NAMETAGS, Setting.SHOW_NAMETAGS) == 0; }
    public boolean preventDrop() { return enabled(Module.DROP_PREVENTION); }
    public boolean muteToasts() { return enabled(Module.TOAST_CONTROL); }
    public boolean cullLogs() { return enabled(Module.CULL_LOGS); }
    public boolean hideVanillaF3() { return enabled(Module.CUSTOM_F3); }
    public boolean chatTimestamps() { return enabled(Module.CHAT_TIMESTAMPS); }
    public boolean cairnTitle() { return enabled(Module.TITLE_TWEAKER); }
    public boolean autohideHud(boolean inventoryOpen) { return enabled(Module.AUTOHIDE_HUD) && inventoryOpen && setting(Module.AUTOHIDE_HUD, Setting.AUTOHIDE_INVENTORY) == 1; }
    public int timeOverride() { return enabled(Module.TIME_CHANGER) ? setting(Module.TIME_CHANGER, Setting.TIME_OF_DAY) : -1; }
    /** 0 clear, 1 rain, 2 thunder; -1 means leave vanilla alone. */
    public int weatherOverride() { return enabled(Module.WEATHER_CHANGER) ? setting(Module.WEATHER_CHANGER, Setting.WEATHER_MODE) : -1; }
    public float viewmodelScale() { return enabled(Module.VIEWMODEL) ? setting(Module.VIEWMODEL, Setting.VIEWMODEL_SCALE) / 100f : 1f; }
    public float particleRate() { return enabled(Module.PARTICLES) ? setting(Module.PARTICLES, Setting.PARTICLE_RATE) / 100f : 1f; }
    public int uiScaleOverride() { return enabled(Module.UI_SCALING) ? setting(Module.UI_SCALING, Setting.UI_SCALE) : -1; }
    public String coverName(String real) {
        if (!enabled(Module.NICK_HIDER) || real == null) return real;
        int n = setting(Module.NICK_HIDER, Setting.NICK_COVER); StringBuilder b = new StringBuilder(n); for (int i = 0; i < n; i++) b.append('*'); return b.toString();
    }
    public boolean consumeReconnect() { boolean v = reconnectRequested; reconnectRequested = false; return v; }
    public void requestReconnect() { if (enabled(Module.RECONNECT)) reconnectRequested = true; }
    public boolean snaplook(boolean keyDown) {
        if (!enabled(Module.SNAPLOOK)) { snaplookHeld = false; return false; }
        snaplookHeld = keyDown; return snaplookHeld;
    }
    public boolean perspectivePressed(boolean keyDown) {
        if (!enabled(Module.PERSPECTIVE)) { perspectivePressed = false; return false; }
        boolean edge = keyDown && !perspectivePressed; perspectivePressed = keyDown; return edge;
    }
    public boolean autoPerspective(boolean elytraFlying) { return enabled(Module.AUTO_PERSPECTIVE) && elytraFlying; }
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
    public boolean menuCloseKey(int code, int tabCode, int shiftCode) { return code != tabCode && menuKey(code) && (!typing() || code == shiftCode); }
    /** Adapters that read raw key codes call this once; Forge keeps vanilla's own Controls screen instead. */
    public void manageKeys(int menuCode, String menuName, int zoomCode, String zoomName) {
        keysManaged = true;
        keyCodes[Bind.MENU.ordinal()] = number("key.MENU", menuCode); keyNames[Bind.MENU.ordinal()] = config.getProperty("key.MENU.name", menuName);
        keyCodes[Bind.ZOOM.ordinal()] = number("key.ZOOM", zoomCode); keyNames[Bind.ZOOM.ordinal()] = config.getProperty("key.ZOOM.name", zoomName);
        keyCodes[Bind.FREELOOK.ordinal()] = number("key.FREELOOK", 342); keyNames[Bind.FREELOOK.ordinal()] = config.getProperty("key.FREELOOK.name", "Left Alt");
        keyCodes[Bind.PERSPECTIVE.ordinal()] = number("key.PERSPECTIVE", 295); keyNames[Bind.PERSPECTIVE.ordinal()] = config.getProperty("key.PERSPECTIVE.name", "F6");
        keyCodes[Bind.SNAPLOOK.ordinal()] = number("key.SNAPLOOK", 71); keyNames[Bind.SNAPLOOK.ordinal()] = config.getProperty("key.SNAPLOOK.name", "G");
        keyCodes[Bind.RECONNECT.ordinal()] = number("key.RECONNECT", 82); keyNames[Bind.RECONNECT.ordinal()] = config.getProperty("key.RECONNECT.name", "R");
    }
    public void defaultBind(Bind bind, int code, String name) {
        keyCodes[bind.ordinal()] = number("key." + bind.name(), code);
        keyNames[bind.ordinal()] = config.getProperty("key." + bind.name() + ".name", name);
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
    public void leaveEditor() { menu.mouseUp(); dragging = null; editing = false; menuCloseRequested = false; save(); }
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
        if (module == Module.INVENTORY) return Math.round((9 * 18 + 2 * setting(module, Setting.PADDING_X)) * scale(module));
        if (module == Module.CROSSHAIR) return 2 * (Math.round(setting(module, Setting.CROSS_SIZE) * scale(module)) + Math.round(setting(module, Setting.CROSS_GAP) * scale(module))) + Math.max(1, Math.round(setting(module, Setting.CROSS_THICKNESS) * scale(module)));
        if (module == Module.KEYSTROKES) return 3 * Math.round(setting(module, Setting.KEY_SIZE) * scale(module)) + 2 * Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale(module)));
        if (setting(module, Setting.AUTO_WIDTH) == 1) return Math.round(autoWidth(state(module).contentWidth, setting(module, Setting.PADDING_X)) * scale(module));
        return Math.round(setting(module, Setting.WIDTH) * scale(module));
    }
    // Rounded up to whole steps so a ticking counter does not make the panel wobble every frame.
    static int autoWidth(int textWidth) { return autoWidth(textWidth, 6); }
    private static int autoWidth(int textWidth, int padding) { return Math.min(Setting.WIDTH.max, (Math.max(24, textWidth) + 2 * padding + 5) / 6 * 6); }
    private int rowHeight(Module module) { return Math.max(setting(module, Setting.HEIGHT), 16 + 2 * setting(module, Setting.PADDING_Y)); }
    private int widgetHeight(Module module) {
        if (module == Module.INVENTORY) return Math.round(((setting(module, Setting.INVENTORY_HOTBAR) == 1 ? 4 : 3) * 18 + 2 * setting(module, Setting.PADDING_Y)) * scale(module));
        if (module == Module.CROSSHAIR) return widgetWidth(module);
        if (!state(module).rows.isEmpty()) return Math.round(state(module).rows.size() * rowHeight(module) * scale(module));
        if (module != Module.KEYSTROKES) return Math.round(Math.max(setting(module, Setting.HEIGHT), 8 + 2 * setting(module, Setting.PADDING_Y)) * scale(module));
        int size = Math.round(setting(module, Setting.KEY_SIZE) * scale(module)), gap = Math.max(1, Math.round(setting(module, Setting.KEY_GAP) * scale(module)));
        int height = size * 2 + gap;
        if (setting(module, Setting.MOUSE_KEYS) == 1) height += Math.round((setting(module, Setting.KEY_SIZE) - 4) * scale(module)) + gap;
        if (setting(module, Setting.SPACE_KEY) == 1) height += Math.round(12 * scale(module)) + gap;
        return height;
    }
    private int x(Module module, int width) {
        State state = state(module);
        int x = module == Module.CROSSHAIR ? width / 2 - widgetWidth(module) / 2 : state.placed ? state.x : state.flowX;
        return clamp(x, 0, Math.max(0, width - widgetWidth(module)));
    }
    private int y(Module module, int height) {
        State state = state(module);
        int y = module == Module.CROSSHAIR ? height / 2 - widgetHeight(module) / 2 : state.placed ? state.y : state.flowY;
        return clamp(y, 0, Math.max(0, height - widgetHeight(module)));
    }
    int[] bounds(Module module) { return new int[] { x(module, viewportWidth), y(module, viewportHeight), widgetWidth(module), widgetHeight(module) }; }
    private void place(Module module, int x, int y) {
        State state = state(module); state.x = x; state.y = y; state.placed = true;
        if (module == Module.CROSSHAIR) config.setProperty("CROSSHAIR.placed", "true");
    }
    /** Widgets the player never moved stack top-left in module order and wrap into columns, so presets never overlap. */
    private long packedSignature;
    private final int[] packedX = new int[MODULES.length], packedY = new int[MODULES.length];
    private void layout(Canvas canvas, int width, int height) {
        int columnX = MARGIN, cursor = MARGIN, columnWidth = 0;
        List<Module> flowing = new ArrayList<Module>();
        boolean overflow = false;
        for (Module module : MODULES) {
            if (!enabled(module) || !module.hud || module == Module.CROSSHAIR) continue;
            State state = state(module);
            if (module != Module.KEYSTROKES) {
                String source = state.value.isEmpty() ? module.title : state.value;
                if (!source.equals(state.measured)) { state.contentWidth = canvas.textWidth(source); state.measured = source; }
                if (!state.rows.isEmpty()) {
                    state.contentWidth = 0;
                    for (HudRow row : state.rows) state.contentWidth = Math.max(state.contentWidth, canvas.textWidth(row.label) + 20);
                }
            }
            if (state.placed) continue;
            flowing.add(module);
            int w = widgetWidth(module), h = widgetHeight(module);
            if (cursor > MARGIN && cursor + h > height - MARGIN) { columnX += columnWidth + GAP; cursor = MARGIN; columnWidth = 0; }
            state.flowX = columnX; state.flowY = cursor;
            overflow |= columnX + w > width - MARGIN;
            cursor += h + GAP; columnWidth = Math.max(columnWidth, w);
        }
        for (Module module : flowing) for (Module pinned : MODULES) {
            if (!enabled(pinned) || !pinned.hud || pinned == Module.CROSSHAIR || !state(pinned).placed) continue;
            int px = x(pinned, width), py = y(pinned, height);
            State s = state(module);
            overflow |= s.flowX < px + widgetWidth(pinned) && px < s.flowX + widgetWidth(module) && s.flowY < py + widgetHeight(pinned) && py < s.flowY + widgetHeight(module);
        }
        if (overflow) {
            long signature = width * 31L + height;
            for (Module module : MODULES) if (enabled(module) && module.hud && module != Module.CROSSHAIR) {
                signature = signature * 31 + module.ordinal(); signature = signature * 31 + widgetWidth(module); signature = signature * 31 + widgetHeight(module);
                signature = signature * 31 + (state(module).placed ? x(module, width) + 1 : 0); signature = signature * 31 + (state(module).placed ? y(module, height) + 1 : 0);
            }
            if (packedSignature == signature) {
                for (Module module : flowing) { state(module).flowX = packedX[module.ordinal()]; state(module).flowY = packedY[module.ordinal()]; }
                return;
            }
            Collections.sort(flowing, (a, b) -> Integer.compare(widgetWidth(b), widgetWidth(a)));
            List<int[]> occupied = new ArrayList<int[]>();
            for (Module pinned : MODULES) if (enabled(pinned) && pinned.hud && pinned != Module.CROSSHAIR && state(pinned).placed) occupied.add(new int[] {x(pinned, width), y(pinned, height), widgetWidth(pinned), widgetHeight(pinned)});
            for (Module module : flowing) {
                int w = widgetWidth(module), h = widgetHeight(module);
                TreeSet<Integer> xs = new TreeSet<Integer>(), ys = new TreeSet<Integer>();
                xs.add(MARGIN); ys.add(MARGIN);
                for (int[] box : occupied) { xs.add(box[0] + box[2] + GAP); ys.add(box[1] + box[3] + GAP); }
                boolean placed = false;
                for (int px : xs) {
                    if (px + w > width - MARGIN) continue;
                    for (int py : ys) {
                        if (py + h > height - MARGIN) continue;
                        boolean free = true;
                        for (int[] box : occupied) if (px < box[0] + box[2] + GAP && box[0] < px + w + GAP && py < box[1] + box[3] + GAP && box[1] < py + h + GAP) { free = false; break; }
                        if (free) { state(module).flowX = px; state(module).flowY = py; occupied.add(new int[] {px, py, w, h}); placed = true; break; }
                    }
                    if (placed) break;
                }
            }
            packedSignature = signature;
            for (Module module : flowing) { packedX[module.ordinal()] = state(module).flowX; packedY[module.ordinal()] = state(module).flowY; }
        }
    }
    private Module previewModule;
    private Setting previewColor;
    private int previewRgb, previewAlpha;
    static Setting alphaSetting(Setting color) { return color == Setting.BG_COLOR || color == Setting.OVERLAY_COLOR ? Setting.OPACITY : color == Setting.TEXT_COLOR ? Setting.TEXT_ALPHA : Setting.ACCENT_ALPHA; }
    private int color(Module module, Setting setting) {
        int rgb = setting(module, setting), alpha = setting(module, alphaSetting(setting));
        if (module == previewModule && setting == previewColor) { rgb = previewRgb; alpha = previewAlpha; }
        return (alpha * 255 / 100 << 24) | (rgb & 0xffffff);
    }
    private int background(Module module) { return color(module, Setting.BG_COLOR); }
    private void widgetText(Canvas canvas, String value, int x, int y, int color, float scale, boolean shadow) {
        if ((color >>> 24) != 0) canvas.textScaled(value, x, y, color, scale, shadow);
    }
    void renderColorPreview(Canvas canvas, Module module, Setting color, int rgb, int alpha, int x, int y, int w, int h) {
        previewModule = module; previewColor = color; previewRgb = rgb; previewAlpha = alpha;
        try { renderPreview(canvas, module, x, y, w, h); }
        finally { previewModule = null; previewColor = null; }
    }
    private void keyBox(Canvas canvas, int x, int y, int w, int h, boolean pressed, String label, Module module, float scale) {
        w = Math.min(w, viewportWidth - x); h = Math.min(h, viewportHeight - y);
        if (w <= 0 || h <= 0) return;
        int bg = pressed ? color(module, Setting.ACCENT_COLOR) : background(module);
        canvas.rect(x, y, w, h, bg);
        if (h < Math.round(9 * scale)) return;
        int pressedText = previewModule == module && previewColor == Setting.ACCENT_COLOR ? readable(bg) : state(module).pressedTextColor;
        int textColor = pressed ? (color(module, Setting.TEXT_COLOR) & 0xff000000) | (pressedText & 0xffffff) : color(module, Setting.TEXT_COLOR);
        if (label.endsWith(" CPS") && canvas.textWidth(label) * scale > w - 4) label = label.substring(0, label.length() - 4);
        label = fit(canvas, label, (int) ((w - 4) / scale));
        widgetText(canvas, label, x + Math.max(1, (w - Math.round(canvas.textWidth(label) * scale)) / 2), y + Math.max(0, (h - Math.round(9 * scale)) / 2), textColor, scale, false);
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
        List<HudRow> original = state(module).rows;
        String originalValue = state(module).value;
        if (original.isEmpty()) state(module).rows = previewRows(module);
        if (originalValue.isEmpty()) state(module).value = previewValue(module);
        try { renderPreviewContent(canvas, module, x, y, width, height); }
        finally { state(module).rows = original; state(module).value = originalValue; }
    }
    private static List<HudRow> previewRows(Module module) {
        if (module == Module.ARMOR) return Arrays.asList(new HudRow("sample/helmet", "363 / 363"), new HudRow("sample/chest", "528 / 528"), new HudRow("sample/legs", "495 / 495"), new HudRow("sample/boots", "429 / 429"));
        if (module == Module.EFFECTS) return Arrays.asList(new HudRow("sample/potion", "Speed II · 1:30"), new HudRow("sample/potion", "Strength · 0:45"));
        if (module == Module.BLOCK_INDICATOR || module == Module.HELD_ITEM) return Collections.singletonList(new HudRow("sample/block", "Grass Block"));
        if (module == Module.SCOREBOARD) return Arrays.asList(new HudRow(null, "Cairn"), new HudRow(null, "Score: 120"), new HudRow(null, "Players: 8"));
        if (module == Module.TEAM_TRACKER) return Arrays.asList(new HudRow(null, "Alex · 12m"), new HudRow(null, "Steve · 24m"));
        return Collections.emptyList();
    }
    private static String previewValue(Module module) {
        switch (module) {
            case FPS: return "144 FPS";
            case COORDINATES: return "XYZ 128 64 -256";
            case PING: return "42 ms";
            case CPS: return "6 / 4 CPS";
            case SPEED: return "4.3 blocks/s";
            case HEARTS: return "20 / 20 HP";
            case FOOD: return "Food 20 / Sat 5";
            case REACH: return "2.8 blocks";
            case TPS: return "20.0 TPS";
            case COMBO: return "3 combo";
            case DEATH_INFO: return "Died at 128 64 -256";
            case HORSES: return "Mount 26/30 · jump 0.70";
            case BOSS_BAR: return "Wither · 65%";
            case ARMOR_POINTS: return "20 armor";
            case ATTACK_INDICATOR: return "75%";
            case ELYTRA: return "Flying · 432 dur";
            case SHIELD: return "336 dur";
            case TOTEM: return "2 totems";
            case LIGHT_LEVEL: return "Block 7 / Sky 15";
            case TNT_TIMER: return "2.50s";
            case ITEM_INFO: return "Diamond · 12s";
            case ITEM_DESPAWN: return "288s left";
            case PACK_DISPLAY: return "Example resource pack";
            case SYSTEM_RESOURCES: return "2048 MB / 8 threads";
            case SATURATION: return "5.0 sat";
            case UHC_OVERLAY: return "20 HP | 4 abs";
            case WAYPOINTS: return "Death 128 64 -256";
            case CLOCK: case WORLD_TIME: return "12:30";
            case SESSION: return "01:23:45";
            case BIOME: return "Plains";
            case SERVER: return "play.example.com";
            case DIRECTION: return "North";
            default: return module.title;
        }
    }
    private void renderPreviewContent(Canvas canvas, Module module, int x, int y, int width, int height) {
        float scale = scale(module);
        String value = state(module).value.isEmpty() ? module.title : state(module).value;
        int contentWidth = canvas.textWidth(value);
        if (!state(module).rows.isEmpty()) { contentWidth = 0; for (HudRow row : state(module).rows) contentWidth = Math.max(contentWidth, canvas.textWidth(row.label) + 20); }
        int w = module == Module.CROSSHAIR || module == Module.KEYSTROKES || module == Module.INVENTORY ? widgetWidth(module)
            : Math.round((setting(module, Setting.AUTO_WIDTH) == 1 ? autoWidth(contentWidth, setting(module, Setting.PADDING_X)) : setting(module, Setting.WIDTH)) * scale);
        int h = widgetHeight(module);
        float fit = Math.min(1, Math.min(width / (float) Math.max(1, w), height / (float) Math.max(1, h)));
        scale *= fit; w = Math.round(w * fit); h = Math.round(h * fit);
        x += (width - w) / 2; y += (height - h) / 2;
        if (module == Module.INVENTORY) renderInventory(canvas, x, y, w, h, scale, true);
        else if (module == Module.CROSSHAIR) crosshair(canvas, x, y, scale);
        else if (module == Module.KEYSTROKES) keystrokes(canvas, x, y, module, scale);
        else if (!state(module).rows.isEmpty()) renderRows(canvas, module, x, y, w, h, scale);
        else {
            if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, w, h, background(module));
            if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, w), h, color(module, Setting.ACCENT_COLOR));
            int padding = setting(module, Setting.PADDING_X);
            String label = fit(canvas, value, Math.max(0, (int) (w / scale) - 2 * padding));
            int align = setting(module, Setting.ALIGN);
            int tx = align == 0 ? Math.round(padding * scale) : align == 1 ? (w - Math.round(canvas.textWidth(label) * scale)) / 2 : w - Math.round((canvas.textWidth(label) + padding) * scale);
            widgetText(canvas, label, x + Math.max(0, tx), y + Math.max(0, (h - Math.round(8 * scale)) / 2), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
        }
    }
    private void renderRows(Canvas canvas, Module module, int x, int y, int width, int height, float scale) {
        if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, width, height, background(module));
        if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, width), height, color(module, Setting.ACCENT_COLOR));
        int rowHeight = Math.max(1, Math.round(rowHeight(module) * scale)), iconSize = Math.max(1, Math.round(16 * scale));
        int padding = setting(module, Setting.PADDING_X);
        List<HudRow> rows = state(module).rows;
        for (int i = 0; i < rows.size(); i++) {
            HudRow row = rows.get(module == Module.ARMOR && setting(module, Setting.ARMOR_INVERT) == 1 ? rows.size() - 1 - i : i);
            if (y + rowHeight > viewportHeight || height < rowHeight) break;
            drawIcon(canvas, row.icon, x + Math.round(padding * scale), y + Math.max(0, (rowHeight - iconSize) / 2), iconSize);
            String label = fit(canvas, row.label, Math.max(0, (int) (width / scale) - 20 - 2 * padding));
            widgetText(canvas, label, x + Math.round((padding + 20) * scale), y + Math.max(0, (rowHeight - Math.round(8 * scale)) / 2), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
            y += rowHeight; height -= rowHeight;
        }
    }
    private void drawIcon(Canvas canvas, Object icon, int x, int y, int size) {
        if (icon instanceof String && ((String) icon).startsWith("sample/")) ModuleIcons.sample(canvas, ((String) icon).substring(7), x, y, size);
        else if (icon != null) canvas.icon(icon, x, y, size);
    }
    private void renderInventory(Canvas canvas, int x, int y, int w, int h, float scale, boolean preview) {
        Module module = Module.INVENTORY;
        if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, w, h, background(module));
        int rows = setting(module, Setting.INVENTORY_HOTBAR) == 1 ? 4 : 3;
        for (int i = 0; i < rows * 9; i++) {
            int sx = x + Math.round((setting(module, Setting.PADDING_X) + i % 9 * 18) * scale);
            int sy = y + Math.round((setting(module, Setting.PADDING_Y) + i / 9 * 18) * scale);
            int size = Math.max(1, Math.round(16 * scale));
            if (sx + size > viewportWidth || sy + size > viewportHeight || sx + size > x + w || sy + size > y + h) continue;
            if (setting(module, Setting.SLOT_BACKGROUND) == 1) canvas.rect(sx, sy, size, size, (color(module, Setting.BG_COLOR) & 0xff000000) | 0x30343b);
            int slot = i < 27 ? i + 9 : i - 27;
            HudRow item = inventory.isEmpty() ? null : inventory.get(slot);
            if (item == null && preview && inventory.isEmpty() && i % 4 == 0) item = new HudRow("sample/block", "64");
            if (item == null || item.icon == null) continue;
            drawIcon(canvas, item.icon, sx, sy, size);
            if (!item.label.isEmpty() && !item.label.equals("1")) {
                float textScale = scale * .75f;
                widgetText(canvas, item.label, sx + Math.max(0, size - Math.round(canvas.textWidth(item.label) * textScale)), sy + Math.max(0, size - Math.round(9 * textScale)), color(module, Setting.TEXT_COLOR), textScale, true);
            }
        }
    }
    public void renderHud(Canvas canvas, int width, int height) {
        viewportWidth = width; viewportHeight = height;
        counters();
        if (!editing && autohideHud(inventoryOpen)) return;
        layout(canvas, width, height);
        for (Module module : MODULES) {
            if (!enabled(module) || !module.hud) continue;
            int x = x(module, width), y = y(module, height);
            float scale = Math.min(scale(module), Math.min(width / (float) Math.max(1, widgetWidth(module)) * scale(module), height / (float) Math.max(1, widgetHeight(module)) * scale(module)));
            int w = Math.min(width - x, widgetWidth(module)), h = Math.min(height - y, widgetHeight(module));
            if (module == Module.INVENTORY) renderInventory(canvas, x, y, w, h, scale, false);
            else if (module == Module.CROSSHAIR) crosshair(canvas, x, y, scale);
            else if (module == Module.KEYSTROKES) keystrokes(canvas, x, y, module, scale);
            else if (!state(module).rows.isEmpty()) renderRows(canvas, module, x, y, w, h, scale);
            else {
                if (setting(module, Setting.BACKGROUND) == 1) canvas.rect(x, y, w, h, background(module));
                if (setting(module, Setting.BORDER) == 1) canvas.rect(x, y, Math.min(2, w), h, color(module, Setting.ACCENT_COLOR));
                State state = state(module);
                String source = state.value.isEmpty() ? module.title : state.value;
                int padding = setting(module, Setting.PADDING_X);
                int available = (int) (w / scale) - 2 * padding;
                if (state.clippedWidth != available || !source.equals(state.clippedSource)) {
                    state.clipped = fit(canvas, source, available); state.clippedSource = source; state.clippedWidth = available; state.textWidth = canvas.textWidth(state.clipped);
                }
                int textX = setting(module, Setting.ALIGN) == 0 ? x + Math.round(padding * scale) : setting(module, Setting.ALIGN) == 1 ? x + Math.max(0, (w - Math.round(state.textWidth * scale)) / 2) : x + Math.max(0, w - Math.round((state.textWidth + padding) * scale));
                widgetText(canvas, state.clipped, textX, y + Math.max(0, (h - Math.round(8 * scale)) / 2), color(module, Setting.TEXT_COLOR), scale, setting(module, Setting.SHADOW) == 1);
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
        viewportWidth = width; viewportHeight = height;
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
            if (selected == Module.CROSSHAIR) return;
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
            if (!enabled(module) || !module.hud || module == Module.CROSSHAIR) continue;
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
        if (!editing) { menu.drag(mouseX, mouseY); return; }
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
    public void mouseUp() { menu.mouseUp(); if (dragging != null) { dragging = null; save(); } }
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
