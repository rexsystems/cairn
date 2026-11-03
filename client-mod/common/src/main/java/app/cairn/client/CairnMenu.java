package app.cairn.client;

import java.util.*;
import app.cairn.client.CairnClient.Module;
import app.cairn.client.CairnClient.Setting;
import app.cairn.client.CairnClient.Canvas;
import app.cairn.client.CairnClient.Bind;
import static app.cairn.client.CairnClient.*;

final class CairnMenu {
    private static final String[] CATEGORIES = { "All", "HUD", "Controls", "Visuals", "Enabled" };
    private static final int[] SWATCHES = { 0xc6db82, 0xf1f0e8, 0x171c18, 0xf2bc71, 0xf28c82, 0x82c8d8, 0xa9b4ef, 0xe8a7ce };
    private final CairnClient client;
    private final List<Control> controls = new ArrayList<Control>();
    private final List<Module> modules = new ArrayList<Module>();
    private String query = "", category = "All", hex = "";
    private boolean searchFocus, hexFocus, hexSelected, details, cosmetics;
    private Setting colorSetting;
    private String colorError;
    private int listScroll, settingsScroll, cosmeticsScroll, focus = -1, mouseX, panelX, panelWidth, listCapacity = 1, listColumns = 1;
    private String cacheKey;
    private static final class Control {
        final int x, y, w, h;
        final Runnable action;
        Control(int x, int y, int w, int h, Runnable action) { this.x = x; this.y = y; this.w = w; this.h = h; this.action = action; }
    }
    CairnMenu(CairnClient client) { this.client = client; }
    boolean typing() { return searchFocus || hexFocus; }
    String query() { return query; }
    void focusSearch() { searchFocus = true; hexFocus = false; details = false; colorSetting = null; }
    void blur() { searchFocus = hexFocus = false; }
    void type(char c) {
        if (hexFocus && Character.digit(c, 16) >= 0) {
            if (hexSelected) { hex = ""; hexSelected = false; }
            if (hex.length() < 6) hex += Character.toUpperCase(c);
            colorError = null;
        }
        else if (searchFocus && c >= 32 && c != 127 && query.length() < 64) { query += c; listScroll = 0; }
    }
    void backspace() {
        if (hexFocus && hexSelected) { hex = ""; hexSelected = false; }
        else if (hexFocus && !hex.isEmpty()) hex = hex.substring(0, hex.length() - 1);
        else if (searchFocus && !query.isEmpty()) { query = query.substring(0, query.length() - 1); listScroll = 0; }
    }
    boolean escape() {
        if (typing()) { blur(); return true; }
        if (colorSetting != null) { colorSetting = null; return true; }
        if (details) { details = false; settingsScroll = 0; return true; }
        if (cosmetics) { cosmetics = false; return true; }
        return false;
    }
    void openCosmetics() { cosmetics = true; details = false; colorSetting = null; cosmeticsScroll = 0; blur(); focus = -1; }
    void openSettings(Module module) { client.select(module); details = true; cosmetics = false; colorSetting = null; settingsScroll = 0; blur(); focus = -1; }
    List<Module> filtered() {
        String key = query + "|" + category;
        if (category.equals("Enabled")) for (Module module : MODULES) key += client.enabled(module) ? "1" : "0";
        if (!key.equals(cacheKey)) {
            modules.clear();
            String q = query.toLowerCase(Locale.ROOT).trim();
            for (Module module : MODULES) {
                if (!category.equals("All") && !(category.equals("Enabled") ? client.enabled(module) : category.equals(module.category))) continue;
                if ((module.title + " " + module.description + " " + module.category).toLowerCase(Locale.ROOT).contains(q)) modules.add(module);
            }
            cacheKey = key;
        }
        return Collections.unmodifiableList(modules);
    }
    void key(int direction, boolean activate) {
        if (activate) {
            if (hexFocus) { applyColor(); return; }
            if (colorSetting != null && focus < 0) { applyColor(); return; }
            if (searchFocus) { searchFocus = false; return; }
            if (focus >= 0 && focus < controls.size()) controls.get(focus).action.run();
            else if (!filtered().isEmpty()) client.toggle(client.selected());
            return;
        }
        blur(); focus = -1;
        List<Module> list = filtered();
        if (list.isEmpty()) return;
        int index = Math.max(0, list.indexOf(client.selected()));
        index = Math.floorMod(index + direction, list.size());
        client.select(list.get(index));
        int row = index / listColumns;
        if (row < listScroll) listScroll = row;
        if (row >= listScroll + listCapacity) listScroll = row - listCapacity + 1;
        settingsScroll = 0;
    }
    void tab() { blur(); if (!controls.isEmpty()) focus = (focus + 1) % controls.size(); }
    void wheel(int amount) {
        if (colorSetting != null) return;
        if (cosmetics) { cosmeticsScroll = Math.max(0, cosmeticsScroll + amount); return; }
        if (details || panelWidth >= 620 && mouseX >= panelX + panelWidth - 216) settingsScroll = Math.max(0, settingsScroll + amount);
        else listScroll = Math.max(0, listScroll + amount);
    }
    void mouseDown(int width, int height, int x, int y) {
        blur(); focus = -1;
        for (int i = controls.size() - 1; i >= 0; i--) {
            Control control = controls.get(i);
            if (x >= control.x && x < control.x + control.w && y >= control.y && y < control.y + control.h) { control.action.run(); return; }
        }
    }
    private void button(Canvas canvas, String label, int x, int y, int w, int h, int bg, int fg, Runnable action) {
        int index = controls.size();
        boolean hover = mouseX >= x && mouseX < x + w && lastMouseY >= y && lastMouseY < y + h;
        canvas.rect(x, y, w, h, hover ? 0xff46513d : bg);
        canvas.text(fit(canvas, label, w - 12), x + 6, y + Math.max(0, (h - 9) / 2), hover && fg == MUTED ? TEXT : fg);
        controls.add(new Control(x, y, w, h, action));
        if (focus == index) {
            canvas.rect(x, y, w, 1, ACCENT); canvas.rect(x, y + h - 1, w, 1, ACCENT);
            canvas.rect(x, y, 1, h, ACCENT); canvas.rect(x + w - 1, y, 1, h, ACCENT);
        }
    }
    private int lastMouseY;
    void render(Canvas canvas, int width, int height, int mx, int my) {
        controls.clear(); mouseX = mx; lastMouseY = my;
        int w = Math.min(760, width - 24), x = (width - w) / 2, top = 12, bottom = height - 12;
        panelX = x; panelWidth = w;
        canvas.rect(0, 0, width, height, 0xd9101411);
        canvas.rect(x, top, w, bottom - top, 0xff20261f);
        canvas.rect(x, top, 3, 38, ACCENT);
        canvas.textScaled("CAIRN", x + 14, top + 6, TEXT, 1.5f, false);
        int enabled = 0; for (Module module : MODULES) if (client.enabled(module)) enabled++;
        canvas.text(MODULES.length + " modules / " + enabled + " enabled", x + 14, top + 25, MUTED);
        button(canvas, "Edit HUD", x + w - 94, top + 7, 80, 26, 0xff3c4835, ACCENT, () -> client.enterEditor());
        if (client.keysManaged() && w >= 400) bindButton(canvas, Bind.MENU, x + w - 248, top + 7, 150, 26);
        boolean headerCosmetics = w >= 480 || w < 400 && w >= 330;
        int cosmeticsX = client.keysManaged() && w >= 400 ? x + w - 334 : x + w - 180;
        if (headerCosmetics) button(canvas, "Cosmetics", cosmeticsX, top + 7, 80, 26, cosmetics ? 0xff3c4835 : 0xff303a2d, cosmetics ? ACCENT : TEXT, () -> { if (cosmetics) cosmetics = false; else openCosmetics(); });
        if (cosmetics && colorSetting == null) { cosmeticsPage(canvas, x + 12, top + 47, w - 24, bottom - top - 59); return; }
        if (colorSetting != null) { colorPicker(canvas, x + 12, top + 48, w - 24, bottom - top - 60); return; }
        if (details) {
            button(canvas, "Back to modules", x + 12, top + 47, 116, 22, 0xff303a2d, ACCENT, () -> { details = false; settingsScroll = 0; });
            settings(canvas, x + 12, top + 75, w - 24, bottom - top - 87);
            return;
        }
        int searchY = top + 46;
        button(canvas, (searchFocus ? "> " : "Search: ") + (query.isEmpty() ? "Find a module..." : query) + (searchFocus ? "_" : ""), x + 12, searchY, w - 74, 25, searchFocus ? 0xff3c4835 : 0xff141b16, TEXT, () -> focusSearch());
        button(canvas, "Clear", x + w - 58, searchY, 46, 25, 0xff2c342b, MUTED, () -> { query = ""; listScroll = 0; });
        int tabsY = searchY + 31, tabWidth = (w - 24) / CATEGORIES.length;
        for (int i = 0; i < CATEGORIES.length; i++) {
            final String name = CATEGORIES[i];
            button(canvas, name, x + 12 + i * tabWidth, tabsY, tabWidth - 3, 22, name.equals(category) ? 0xff3c4835 : 0xff292f27, name.equals(category) ? ACCENT : MUTED, () -> { category = name; listScroll = settingsScroll = 0; details = false; });
        }
        int contentTop = tabsY + 30, contentBottom = bottom - (colorSetting != null || details ? 12 : 43);
        if (colorSetting != null) { colorPicker(canvas, x + 12, contentTop, w - 24, contentBottom - contentTop); }
        else if (details) {
            button(canvas, "Back to modules", x + 12, contentTop, 116, 22, 0xff303a2d, ACCENT, () -> { details = false; settingsScroll = 0; });
            settings(canvas, x + 12, contentTop + 28, w - 24, contentBottom - contentTop - 28);
        } else {
            int listW = w >= 620 ? w - 238 : w - 24;
            moduleList(canvas, x + 12, contentTop, listW, contentBottom - contentTop);
            if (w >= 620) settings(canvas, x + w - 212, contentTop, 200, contentBottom - contentTop);
        }
        if (colorSetting != null || details) return;
        int footerY = bottom - 32, presetW = w < 330 ? Math.max(36, (w - 150) / 3) : Math.max(42, Math.min(78, (w - 40) / 4));
        canvas.text("Preset", x + 12, footerY + 7, MUTED);
        String[] names = { "Minimal", "Combat", "Explore" };
        for (int i = 0; i < 3; i++) {
            final int preset = i;
            button(canvas, names[i], x + 54 + i * (presetW + 3), footerY, presetW, 22, 0xff303a2d, TEXT, () -> client.preset(preset));
        }
        int presetsEnd = x + 54 + 3 * (presetW + 3);
        int footerButton = w < 330 ? 70 : 82;
        if (!headerCosmetics && x + w - 12 - footerButton >= presetsEnd) button(canvas, "Cosmetics", x + w - 12 - footerButton, footerY, footerButton, 22, 0xff3c4835, ACCENT, () -> openCosmetics());
        if (client.saveError() != null) canvas.text(fit(canvas, "Could not save settings. Check the log.", w - 24), x + 12, contentBottom - 10, 0xffffb9a4);
    }
    private static String capeName(Cosmetics cosmetics, String id) {
        if (id.equals("none")) return "None";
        if (id.equals("custom")) return "Your cape";
        for (Cosmetics.Item item : cosmetics.capes()) if (item.id.equals(id)) return item.name;
        return id;
    }
    private void cosmeticsPage(Canvas canvas, int x, int y, int w, int h) {
        final Cosmetics cosmetics = client.cosmetics;
        cosmetics.wantStats();
        button(canvas, "Back to modules", x, y, 116, 22, 0xff303a2d, ACCENT, () -> this.cosmetics = false);
        boolean wide = w >= 380;
        if (wide) {
            button(canvas, "Refresh", x + w - 58, y, 58, 22, 0xff303a2d, MUTED, () -> cosmetics.refresh());
            button(canvas, cosmetics.visible() ? "Shown" : "Hidden", x + w - 122, y, 60, 22, 0xff303a2d, cosmetics.visible() ? ACCENT : MUTED, () -> cosmetics.visible(!cosmetics.visible()));
            String stats = cosmetics.online() >= 0 ? cosmetics.online() + " online / " + cosmetics.peak() + " peak / " + cosmetics.total() + " players" : cosmetics.enabled() ? "Counting players..." : "";
            canvas.text(fit(canvas, stats, w - 254), x + 124, y + 7, MUTED);
        }
        canvas.text(fit(canvas, cosmetics.status(), w), x, y + 30, cosmetics.signedIn() ? TEXT : MUTED);
        List<String> capes = new ArrayList<String>();
        capes.add("none");
        for (Cosmetics.Item item : cosmetics.capes()) if (cosmetics.unlocked(item, true)) capes.add(item.id);
        if (cosmetics.customHash() != null && !"rejected".equals(cosmetics.customStatus())) capes.add("custom");
        String worn = cosmetics.equippedCape() == null ? "none" : cosmetics.equippedCape();
        int cardW = 56, cardH = 84, gap = 6, columns = Math.max(1, (w + gap) / (cardW + gap));
        int gridTop = y + 58, badgeBlock = 62, rowsVisible = Math.max(1, (h - 58 - badgeBlock - 14) / (cardH + gap));
        int rows = (capes.size() + columns - 1) / columns;
        cosmeticsScroll = clamp(cosmeticsScroll, 0, Math.max(0, rows - rowsVisible));
        canvas.text(rows > rowsVisible ? "Capes (scroll for more)" : "Capes", x, y + 46, TEXT);
        long now = System.currentTimeMillis();
        for (int i = cosmeticsScroll * columns; i < Math.min(capes.size(), (cosmeticsScroll + rowsVisible) * columns); i++) {
            final String id = capes.get(i);
            int cx = x + i % columns * (cardW + gap), cy = gridTop + (i / columns - cosmeticsScroll) * (cardH + gap);
            boolean selected = id.equals(worn), hover = mouseX >= cx && mouseX < cx + cardW && lastMouseY >= cy && lastMouseY < cy + cardH;
            canvas.rect(cx, cy, cardW, cardH, selected ? 0xff35412f : hover ? 0xff313a2f : 0xff2a3229);
            if (selected) { canvas.rect(cx, cy, cardW, 2, ACCENT); canvas.rect(cx, cy + cardH - 2, cardW, 2, ACCENT); }
            canvas.rect(cx + 12, cy + 5, 32, 50, 0xff171c18);
            String texture = id.equals("none") ? null : cosmetics.previewTexture(id, now);
            // The cape's outer face sits at (1,1) and is 10x16 in the 64x32 layout.
            if (texture != null) canvas.image(texture, cx + 13, cy + 6, 30, 48, 1 / 64f, 1 / 32f, 11 / 64f, 17 / 32f);
            canvas.text(fit(canvas, capeName(cosmetics, id), cardW - 6), cx + 3, cy + 59, selected ? ACCENT : TEXT);
            String note = selected ? "Worn" : id.equals("custom") && "pending".equals(cosmetics.customStatus()) ? "In review" : "";
            if (!note.isEmpty()) canvas.text(fit(canvas, note, cardW - 6), cx + 3, cy + 71, MUTED);
            controls.add(new Control(cx, cy, cardW, cardH, () -> cosmetics.equip(id.equals("none") ? null : id, cosmetics.equippedBadge())));
        }
        int badgeY = gridTop + rowsVisible * (cardH + gap) + 2;
        if (badgeY + 44 > y + h - 12) return;
        canvas.text(fit(canvas, "Logo badge next to your name", w), x, badgeY, TEXT);
        String badge = cosmetics.equippedBadge() == null ? "classic" : cosmetics.equippedBadge();
        List<String> badges = new ArrayList<String>();
        badges.add("none");
        for (Cosmetics.Item item : cosmetics.badges()) if (cosmetics.unlocked(item, false)) badges.add(item.id);
        int cell = 28, perRow = Math.max(1, (w + 4) / (cell + 4));
        for (int i = 0; i < Math.min(badges.size(), perRow); i++) {
            final String id = badges.get(i);
            int bx = x + i * (cell + 4), by = badgeY + 14;
            boolean selected = id.equals(badge);
            canvas.rect(bx, by, cell, cell, selected ? 0xff35412f : 0xff2a3229);
            if (selected) canvas.rect(bx, by + cell - 2, cell, 2, ACCENT);
            String texture = cosmetics.badgeTexture(id);
            if (texture != null) canvas.image(texture, bx + 4, by + 4, 20, 20, 0, 0, 1, 1);
            else if (id.equals("none")) canvas.text("-", bx + 11, by + 10, MUTED);
            controls.add(new Control(bx, by, cell, cell, () -> cosmetics.equip(cosmetics.equippedCape(), id)));
        }
        canvas.text(fit(canvas, "Upload your own cape, animated strips too, in the Cairn launcher.", w), x, y + h - 10, MUTED);
    }
    private void moduleList(Canvas canvas, int x, int y, int w, int height) {
        List<Module> list = filtered();
        if (list.isEmpty()) { canvas.text(fit(canvas, "No modules match. Try another search.", w - 12), x + 6, y + 10, MUTED); return; }
        int columns = w >= 330 ? 2 : 1, cellWidth = (w - (columns - 1) * 8) / columns, rowHeight = 51;
        int rows = (list.size() + columns - 1) / columns, capacity = Math.max(1, height / rowHeight);
        listCapacity = capacity; listColumns = columns;
        listScroll = clamp(listScroll, 0, Math.max(0, rows - capacity));
        if (list.indexOf(client.selected()) < 0) client.select(list.get(0));
        int selectedRow = list.indexOf(client.selected()) / columns;
        if (focus == -1 && !typing() && selectedRow < listScroll) client.select(list.get(Math.min(list.size() - 1, listScroll * columns)));
        for (int i = listScroll * columns; i < Math.min(list.size(), (listScroll + capacity) * columns); i++) {
            final Module module = list.get(i);
            int cx = x + i % columns * (cellWidth + 8), cy = y + (i / columns - listScroll) * rowHeight;
            boolean selected = module == client.selected();
            canvas.rect(cx, cy, cellWidth, 44, selected ? 0xff35412f : 0xff2a3229);
            if (selected) canvas.rect(cx, cy, 2, 44, ACCENT);
            canvas.text(fit(canvas, module.title, cellWidth - 52), cx + 9, cy + 7, TEXT);
            canvas.text(fit(canvas, module.description, cellWidth - 69), cx + 9, cy + 24, MUTED);
            controls.add(new Control(cx, cy, cellWidth - 43, 44, () -> { client.select(module); settingsScroll = 0; }));
            if (focus == controls.size() - 1) { canvas.rect(cx, cy, cellWidth - 43, 1, ACCENT); canvas.rect(cx, cy + 43, cellWidth - 43, 1, ACCENT); }
            button(canvas, client.enabled(module) ? "ON" : "OFF", cx + cellWidth - 39, cy + 4, 35, 17, selected ? 0xff35412f : 0xff2a3229, client.enabled(module) ? ACCENT : MUTED, () -> client.toggle(module));
            button(canvas, "Settings", cx + cellWidth - 59, cy + 23, 55, 17, selected ? 0xff35412f : 0xff2a3229, MUTED, () -> openSettings(module));
        }
        if (rows > capacity) {
            int barHeight = Math.max(6, height * capacity / rows), position = listScroll * Math.max(0, height - barHeight) / Math.max(1, rows - capacity);
            canvas.rect(x + w + 2, y + position, 2, barHeight, ACCENT);
        }
    }
    private void bindButton(Canvas canvas, Bind bind, int x, int y, int w, int h) {
        boolean waiting = client.listening() == bind;
        button(canvas, waiting ? "Press a key..." : bind.label + ": " + client.keyName(bind), x, y, w, h, waiting ? 0xff3c4835 : 0xff303a2d, waiting ? ACCENT : TEXT, () -> client.listen(bind));
    }
    private List<Setting> options(Module module) {
        List<Setting> list = new ArrayList<Setting>();
        if (module == Module.CROSSHAIR) { list.addAll(Arrays.asList(Setting.SCALE, Setting.ACCENT_COLOR, Setting.CROSS_SIZE, Setting.CROSS_GAP, Setting.CROSS_THICKNESS, Setting.CROSS_DOT, Setting.CROSS_HIDE_VANILLA)); return list; }
        if (module.hud) list.addAll(Arrays.asList(Setting.SCALE, Setting.TEXT_COLOR, Setting.BG_COLOR, Setting.ACCENT_COLOR));
        if (module == Module.KEYSTROKES) list.addAll(Arrays.asList(Setting.KEY_SIZE, Setting.KEY_GAP, Setting.MOUSE_KEYS, Setting.SPACE_KEY, Setting.KEY_CPS, Setting.OPACITY));
        else if (module.hud) list.addAll(Arrays.asList(Setting.AUTO_WIDTH, Setting.WIDTH, Setting.OPACITY, Setting.BACKGROUND, Setting.BORDER, Setting.SHADOW, Setting.ALIGN));
        if (module == Module.CLOCK) list.addAll(Arrays.asList(Setting.CLOCK_SECONDS, Setting.CLOCK_24H));
        if (module == Module.ZOOM) list.addAll(Arrays.asList(Setting.ZOOM_FACTOR, Setting.SMOOTH_ZOOM, Setting.SCROLL_ZOOM, Setting.ZOOM_CINEMATIC));
        if (module == Module.SPRINT) list.addAll(Arrays.asList(Setting.SPRINT_AUTO, Setting.SPRINT_RESET));
        return list;
    }
    private void settings(Canvas canvas, int x, int y, int w, int height) {
        Module module = client.selected();
        canvas.text(fit(canvas, module.title + " settings", w), x, y + 2, TEXT);
        button(canvas, client.enabled(module) ? "Enabled" : "Disabled", x, y + 18, w, 22, 0xff303a2d, client.enabled(module) ? ACCENT : MUTED, () -> client.toggle(module));
        List<Setting> options = options(module);
        int rowHeight = 28, first = y + 40, reserved = 66;
        if (module == Module.ZOOM && client.keysManaged()) { bindButton(canvas, Bind.ZOOM, x, first, w, 22); first += rowHeight; reserved += rowHeight; }
        int capacity = Math.max(1, (height - reserved) / rowHeight);
        settingsScroll = clamp(settingsScroll, 0, Math.max(0, options.size() - capacity));
        for (int i = settingsScroll; i < Math.min(options.size(), settingsScroll + capacity); i++) {
            final Setting setting = options.get(i);
            int cy = first + (i - settingsScroll) * rowHeight, value = client.setting(module, setting);
            canvas.text(fit(canvas, setting.label, w - 85), x, cy + 7, MUTED);
            if (setting.color()) {
                canvas.rect(x + w - 76, cy + 6, 12, 12, value);
                button(canvas, "Change", x + w - 59, cy, 59, 23, 0xff303a2d, TEXT, () -> { colorSetting = setting; hex = String.format(Locale.ROOT, "%06X", client.setting(module, setting) & 0xffffff); hexFocus = false; colorError = null; });
            } else if (setting.toggle()) {
                button(canvas, value == 1 ? "ON" : "OFF", x + w - 54, cy, 54, 23, 0xff303a2d, value == 1 ? ACCENT : MUTED, () -> { client.setting(module, setting, value == 1 ? 0 : 1); client.save(); });
            } else {
                button(canvas, "-", x + w - 78, cy, 20, 23, 0xff303a2d, TEXT, () -> { client.setting(module, setting, client.setting(module, setting) - setting.step); client.save(); });
                String label = setting == Setting.ALIGN ? new String[] { "Left", "Center", "Right" }[value] : String.valueOf(value) + (setting == Setting.SCALE || setting == Setting.OPACITY ? "%" : "");
                canvas.text(fit(canvas, label, 34), x + w - 55, cy + 7, TEXT);
                button(canvas, "+", x + w - 20, cy, 20, 23, 0xff303a2d, TEXT, () -> { client.setting(module, setting, client.setting(module, setting) + setting.step); client.save(); });
            }
        }
        int foot = y + Math.max(48, height - 24);
        if (!options.isEmpty()) {
            button(canvas, "Up", x, foot, 31, 22, 0xff303a2d, MUTED, () -> settingsScroll = Math.max(0, settingsScroll - 1));
            button(canvas, "Down", x + 35, foot, 43, 22, 0xff303a2d, MUTED, () -> settingsScroll++);
        }
        button(canvas, "Reset", x + w - 57, foot, 57, 22, 0xff303a2d, MUTED, () -> client.reset(module));
    }
    private void applyColor() {
        if (hex.length() != 6) { colorError = "Enter six hex digits."; return; }
        try { client.setting(client.selected(), colorSetting, (int) Long.parseLong(hex, 16)); client.save(); colorSetting = null; hexFocus = false; colorError = null; }
        catch (NumberFormatException e) { colorError = "Use 0-9 and A-F."; }
    }
    private void colorPicker(Canvas canvas, int x, int y, int w, int h) {
        canvas.text(fit(canvas, client.selected().title + " / " + colorSetting.label, w), x, y + 2, TEXT);
        int swatchW = Math.min(30, (w - 28) / 8);
        for (int i = 0; i < SWATCHES.length; i++) {
            final int color = SWATCHES[i];
            button(canvas, "", x + i * (swatchW + 4), y + 22, swatchW, 24, 0xff000000 | color, TEXT, () -> { hex = String.format(Locale.ROOT, "%06X", color); });
        }
        button(canvas, "#" + hex + (hexFocus ? "_" : ""), x, y + 58, Math.min(w, 150), 25, hexFocus ? 0xff3c4835 : 0xff141b16, TEXT, () -> { hexFocus = true; hexSelected = true; });
        int actionY = h >= 140 ? y + 112 : y + 88;
        if (h >= 140) canvas.text(fit(canvas, colorError == null ? "Click the field and enter any RGB hex color." : colorError, w), x, y + 92, colorError == null ? MUTED : 0xffffb9a4);
        button(canvas, "Apply color", x, actionY, 90, 24, 0xff3c4835, ACCENT, () -> applyColor());
        button(canvas, "Cancel", x + 98, actionY, 65, 24, 0xff303a2d, TEXT, () -> { colorSetting = null; hexFocus = false; });
    }
}
