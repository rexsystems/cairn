package app.cairn.client;

import java.util.*;
import app.cairn.client.CairnClient.Module;
import app.cairn.client.CairnClient.Setting;
import app.cairn.client.CairnClient.Canvas;
import app.cairn.client.CairnClient.Bind;
import static app.cairn.client.CairnClient.*;

final class CairnMenu {
    private static final int ACCENT = 0xffff858c, TEXT = 0xffeeeeee, MUTED = 0xffaeb5bf;
    private static final int PANEL = 0xff101318, SURFACE = 0xff191e25, EDGE = 0xff2a3039, PRIMARY = 0xffbf303d;
    private static final String[] CATEGORIES = { "All", "HUD", "Controls", "Visuals", "Enabled" };
    private final CairnClient client;
    private final List<Control> controls = new ArrayList<Control>();
    private final List<Module> modules = new ArrayList<Module>();
    private String query = "", category = "All", hex = "";
    private boolean searchFocus, hexFocus, hexSelected, details, cosmetics, friends, friendFocus, friendSettings;
    private String friendField = "";
    private int friendsScroll;
    private Setting colorSetting;
    private String colorError;
    private int listScroll, settingsScroll, cosmeticsScroll, focus = -1, mouseX, panelX, panelWidth, listCapacity = 1, listColumns = 1;
    private String cacheKey;
    private Slider activeSlider;
    private boolean sliderChanged;
    private float hue, saturation, brightness;
    private int draftRgb, draftAlpha = 100, colorDrag, colorX, colorY, colorWidth, colorHeight;
    private boolean brightnessAxis;
    private final Map<Integer, Integer> colorControls = new HashMap<Integer, Integer>();
    private static final class Slider {
        final Module module; final Setting setting;
        int x, width;
        Slider(Module module, Setting setting, int x, int width) { this.module = module; this.setting = setting; this.x = x; this.width = width; }
    }
    private static final class Control {
        final int x, y, w, h;
        final Runnable action;
        final Slider slider;
        Control(int x, int y, int w, int h, Runnable action) { this(x, y, w, h, action, null); }
        Control(int x, int y, int w, int h, Runnable action, Slider slider) { this.x = x; this.y = y; this.w = w; this.h = h; this.action = action; this.slider = slider; }
    }
    CairnMenu(CairnClient client) {
        this.client = client;
        // Reopening the menu lands on the page the player last used.
        String page = client.option("menu.page", "modules");
        cosmetics = page.equals("cosmetics");
        friends = page.equals("friends");
    }
    boolean typing() { return searchFocus || hexFocus || friendFocus; }
    String query() { return query; }
    void focusSearch() { searchFocus = true; hexFocus = friendFocus = false; details = false; colorSetting = null; }
    void blur() { mouseUp(); searchFocus = hexFocus = friendFocus = false; }
    void type(char c) {
        if (friendFocus) {
            if ((Character.isLetterOrDigit(c) && c < 128 || c == '_' || c == '-') && friendField.length() < 36) friendField += c;
            return;
        }
        if (hexFocus && Character.digit(c, 16) >= 0) {
            if (hexSelected) { hex = ""; hexSelected = false; }
            if (hex.length() < 8) hex += Character.toUpperCase(c);
            readHex();
            colorError = null;
        }
        else if (searchFocus && c >= 32 && c != 127 && query.length() < 64) { query += c; listScroll = 0; }
    }
    void backspace() {
        if (friendFocus) { if (!friendField.isEmpty()) friendField = friendField.substring(0, friendField.length() - 1); return; }
        if (hexFocus && hexSelected) { hex = ""; hexSelected = false; }
        else if (hexFocus && !hex.isEmpty()) hex = hex.substring(0, hex.length() - 1);
        else if (searchFocus && !query.isEmpty()) { query = query.substring(0, query.length() - 1); listScroll = 0; }
        if (hexFocus) readHex();
    }
    boolean escape() {
        mouseUp();
        if (typing()) { blur(); return true; }
        if (colorSetting != null) { colorSetting = null; return true; }
        if (details) { details = false; settingsScroll = 0; return true; }
        if (cosmetics || friends) { page("modules"); return true; }
        return false;
    }
    private void page(String name) {
        cosmetics = name.equals("cosmetics"); friends = name.equals("friends");
        if (!name.equals(client.option("menu.page", "modules"))) client.setOption("menu.page", name);
    }
    void openCosmetics() { page("cosmetics"); details = false; colorSetting = null; cosmeticsScroll = 0; blur(); focus = -1; }
    void openFriends() { page("friends"); details = false; colorSetting = null; friendsScroll = 0; blur(); focus = -1; client.friends.wantList(); }
    void openSettings(Module module) { client.select(module); details = true; page("modules"); colorSetting = null; settingsScroll = 0; blur(); focus = -1; }
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
        mouseUp();
        if (colorSetting != null && colorControls.containsKey(focus)) {
            int kind = colorControls.get(focus);
            if (activate && kind == 1) brightnessAxis = !brightnessAxis;
            else if (direction != 0) {
                if (kind == 1 && brightnessAxis) brightness = Math.max(0, Math.min(1, brightness + direction / 100f));
                else if (kind == 1) saturation = Math.max(0, Math.min(1, saturation + direction / 100f));
                else if (kind == 2) hue = Math.max(0, Math.min(1, hue + direction / 360f));
                else draftAlpha = clamp(draftAlpha + direction, 0, 100);
                syncHex();
            }
            return;
        }
        if (focus >= 0 && focus < controls.size() && controls.get(focus).slider != null) {
            Slider slider = controls.get(focus).slider;
            if (direction != 0) {
                client.setting(slider.module, slider.setting, client.setting(slider.module, slider.setting) + direction * slider.setting.step);
                client.save();
            }
            return;
        }
        if (activate) {
            if (hexFocus) { applyColor(); return; }
            if (friendFocus) { addFriend(); return; }
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
        mouseUp();
        if (colorSetting != null) return;
        if (cosmetics) { cosmeticsScroll = Math.max(0, cosmeticsScroll + amount); return; }
        if (friends) { friendsScroll = Math.max(0, friendsScroll + amount); return; }
        if (details || panelWidth >= 620 && mouseX >= panelX + panelWidth - 216) settingsScroll = Math.max(0, settingsScroll + amount);
        else listScroll = Math.max(0, listScroll + amount);
    }
    void mouseDown(int width, int height, int x, int y) {
        mouseUp();
        mouseX = x; lastMouseY = y;
        blur(); focus = -1;
        for (int i = controls.size() - 1; i >= 0; i--) {
            Control control = controls.get(i);
            if (x >= control.x && x < control.x + control.w && y >= control.y && y < control.y + control.h) {
                if (control.slider != null) { activeSlider = control.slider; focus = i; drag(x, y); }
                else { if (colorControls.containsKey(i)) focus = i; control.action.run(); }
                return;
            }
        }
    }
    void drag(int x, int y) {
        if (colorDrag != 0) {
            float horizontal = clamp(x - colorX, 0, colorWidth) / (float) colorWidth;
            if (colorDrag == 1) { saturation = horizontal; brightness = 1 - clamp(y - colorY, 0, colorHeight) / (float) colorHeight; }
            else if (colorDrag == 2) hue = horizontal;
            else draftAlpha = Math.round(horizontal * 100);
            syncHex(); return;
        }
        if (activeSlider == null) return;
        Setting setting = activeSlider.setting;
        float fraction = clamp(x - activeSlider.x, 0, activeSlider.width) / (float) activeSlider.width;
        int value = clamp(setting.min + Math.round(fraction * (setting.max - setting.min) / setting.step) * setting.step, setting.min, setting.max);
        if (value != client.setting(activeSlider.module, setting)) {
            client.setting(activeSlider.module, setting, value); sliderChanged = true;
        }
    }
    void mouseUp() {
        activeSlider = null; colorDrag = 0;
        if (sliderChanged) { sliderChanged = false; client.save(); }
    }
    private void button(Canvas canvas, String label, int x, int y, int w, int h, int bg, int fg, Runnable action) {
        int index = controls.size();
        boolean hover = mouseX >= x && mouseX < x + w && lastMouseY >= y && lastMouseY < y + h;
        boolean accent = bg == 0xff3c4835;
        rounded(canvas, x, y, w, h, 3, hover ? 0xff3a424e : bg == PRIMARY ? PRIMARY : accent ? 0xff44252c : SURFACE);
        canvas.text(fit(canvas, label, w - 12), x + 6, y + Math.max(0, (h - 9) / 2), hover && fg == MUTED ? TEXT : fg);
        controls.add(new Control(x, y, w, h, action));
        if (focus == index) {
            canvas.rect(x, y, w, 1, ACCENT); canvas.rect(x, y + h - 1, w, 1, ACCENT);
            canvas.rect(x, y, 1, h, ACCENT); canvas.rect(x + w - 1, y, 1, h, ACCENT);
        }
    }
    static void rounded(Canvas canvas, int x, int y, int w, int h, int radius, int color) {
        int r = Math.min(radius, Math.min(w, h) / 2);
        if (w <= 0 || h <= 0) return;
        canvas.rect(x + r, y, w - r * 2, h, color);
        canvas.rect(x, y + r, w, h - r * 2, color);
        for (int row = 0; row < r; row++) {
            int inset = r - (int) Math.sqrt(r * r - (r - row - 1) * (r - row - 1));
            canvas.rect(x + inset, y + row, w - inset * 2, 1, color);
            canvas.rect(x + inset, y + h - row - 1, w - inset * 2, 1, color);
        }
    }
    private void navigation(Canvas canvas, String label, int x, int y, int w, boolean selected, Runnable action) {
        int index = controls.size();
        boolean hover = mouseX >= x && mouseX < x + w && lastMouseY >= y && lastMouseY < y + 24;
        rounded(canvas, x, y, w, 24, 3, selected ? PRIMARY : hover ? 0xff303741 : SURFACE);
        canvas.text(fit(canvas, label, w - 16), x + 8, y + 8, selected ? TEXT : MUTED);
        controls.add(new Control(x, y, w, 24, action));
        if (focus == index) canvas.rect(x + 3, y + 21, w - 6, 2, ACCENT);
    }
    void openModules() { page("modules"); details = false; colorSetting = null; blur(); focus = -1; }
    void editorControls(Canvas canvas, int width, int height, int mx, int my) {
        controls.clear(); colorControls.clear(); mouseX = mx; lastMouseY = my;
        int w = Math.min(190, width - 24), x = (width - w) / 2, y = Math.max(42, height / 2 - 30);
        canvas.text("CAIRN", (width - canvas.textWidth("CAIRN")) / 2, y - 20, TEXT);
        button(canvas, "Cairn Settings", x, y, w, 26, PRIMARY, TEXT, () -> client.openModules());
        int half = (w - 6) / 2;
        button(canvas, "Cosmetics", x, y + 32, half, 24, SURFACE, TEXT, () -> client.openCosmetics());
        button(canvas, "Friends", x + half + 6, y + 32, half, 24, SURFACE, TEXT, () -> client.openFriends());
        for (final Module module : MODULES) {
            if (!module.hud || !client.enabled(module)) continue;
            int[] b = client.bounds(module);
            int bx = clamp(b[0] + b[2] + 2, 0, width - 26), by = clamp(b[1], 0, height - 38);
            rounded(canvas, bx, by, 12, 12, 2, SURFACE); gear(canvas, bx + 1, by, MUTED);
            controls.add(new Control(bx, by, 12, 12, () -> { client.select(module); client.editorSettings(); }));
            button(canvas, "-", bx + 14, by, 12, 12, SURFACE, ACCENT, () -> client.toggle(module));
        }
    }
    boolean editorClick(int x, int y) {
        for (Control control : controls) if (x >= control.x && x < control.x + control.w && y >= control.y && y < control.y + control.h) { control.action.run(); return true; }
        return false;
    }
    void clearControls() { mouseUp(); controls.clear(); }
    private int lastMouseY;
    void render(Canvas canvas, int width, int height, int mx, int my) {
        controls.clear(); colorControls.clear(); mouseX = mx; lastMouseY = my;
        int w = Math.min(680, width - 24), x = (width - w) / 2, top = Math.max(12, (height - 440) / 2), bottom = height - top;
        panelX = x; panelWidth = w;
        canvas.rect(0, 0, width, height, 0x9906090d);
        canvas.textScaled("CAIRN", x + 2, top + 4, TEXT, 1.25f, false);
        int enabled = 0; for (Module module : MODULES) if (client.enabled(module)) enabled++;
        canvas.text(enabled + " / " + MODULES.length + " enabled", x + 68, top + 7, MUTED);
        button(canvas, "Edit HUD", x + w - 112, top, 80, 22, SURFACE, TEXT, () -> client.enterEditor());
        button(canvas, "X", x + w - 26, top, 26, 22, SURFACE, ACCENT, () -> client.requestMenuClose());
        if (client.keysManaged() && w >= 500) bindButton(canvas, Bind.MENU, x + w - 268, top, 150, 22);
        int navY = top + 28, navWidth = Math.min(102, (w - 8) / 3);
        navigation(canvas, "Modules", x, navY, navWidth, !friends && !cosmetics, this::openModules);
        navigation(canvas, friendsLabel(), x + navWidth + 4, navY, navWidth, friends, this::openFriends);
        navigation(canvas, "Cosmetics", x + (navWidth + 4) * 2, navY, navWidth, cosmetics, this::openCosmetics);
        int bodyTop = navY + 32;
        rounded(canvas, x, bodyTop, w, bottom - bodyTop, 5, PANEL);
        int pageTop = bodyTop + 10;
        if (cosmetics && colorSetting == null) { cosmeticsPage(canvas, x + 12, pageTop, w - 24, bottom - pageTop - 12); return; }
        if (friends && colorSetting == null) { friendsPage(canvas, x + 12, pageTop, w - 24, bottom - pageTop - 12); return; }
        if (colorSetting != null) { colorPicker(canvas, x + 12, pageTop + 1, w - 24, bottom - pageTop - 13); return; }
        if (details) {
            button(canvas, "Back to modules", x + 12, pageTop, 116, 22, 0xff303a2d, ACCENT, () -> { details = false; settingsScroll = 0; });
            settings(canvas, x + 12, pageTop + 28, w - 24, bottom - pageTop - 40);
            return;
        }
        boolean wide = w >= 540;
        int searchY = pageTop, searchWidth = wide ? 174 : w - 74;
        int searchX = wide ? x + w - 232 : x + 12;
        button(canvas, (searchFocus ? "> " : "Search: ") + (query.isEmpty() ? "Find a module..." : query) + (searchFocus ? "_" : ""), searchX, searchY, searchWidth, 22, searchFocus ? 0xff3c4835 : SURFACE, TEXT, () -> focusSearch());
        button(canvas, "Clear", x + w - 54, searchY, 42, 22, SURFACE, MUTED, () -> { query = ""; listScroll = 0; });
        int tabsY = wide ? searchY : searchY + 28;
        int tabWidth = (wide ? w - 250 : w - 24) / CATEGORIES.length;
        for (int i = 0; i < CATEGORIES.length; i++) {
            final String name = CATEGORIES[i];
            button(canvas, name, x + 12 + i * tabWidth, tabsY, tabWidth - 3, 22, name.equals(category) ? 0xff3c4835 : 0xff292f27, name.equals(category) ? ACCENT : MUTED, () -> { category = name; listScroll = settingsScroll = 0; details = false; });
        }
        int contentTop = tabsY + 30, contentBottom = bottom - 42;
        if (colorSetting != null) { colorPicker(canvas, x + 12, contentTop, w - 24, contentBottom - contentTop); }
        else if (details) {
            button(canvas, "Back to modules", x + 12, contentTop, 116, 22, 0xff303a2d, ACCENT, () -> { details = false; settingsScroll = 0; });
            settings(canvas, x + 12, contentTop + 28, w - 24, contentBottom - contentTop - 28);
        } else {
            moduleList(canvas, x + 12, contentTop, w - 24, contentBottom - contentTop);
        }
        if (colorSetting != null || details) return;
        int footerY = bottom - 30, presetW = w < 330 ? Math.max(36, (w - 168) / 3) : Math.max(42, Math.min(68, (w - 40) / 4));
        canvas.text("Preset", x + 12, footerY + 7, MUTED);
        String[] names = { "Minimal", "Combat", "Explore" };
        for (int i = 0; i < 3; i++) {
            final int preset = i;
            button(canvas, names[i], x + 54 + i * (presetW + 3), footerY, presetW, 22, 0xff303a2d, TEXT, () -> client.preset(preset));
        }
        if (client.keysManaged() && w < 500) {
            int bindWidth = Math.min(150, w - 66 - 3 * (presetW + 3));
            bindButton(canvas, Bind.MENU, x + w - 12 - bindWidth, footerY, bindWidth, 22);
        }
        if (client.saveError() != null) canvas.text(fit(canvas, "Could not save settings. Check the log.", w - 24), x + 12, contentBottom - 10, 0xffffb9a4);
    }
    private String friendsLabel() {
        int pending = client.friends.incoming().size();
        return pending > 0 ? "Friends " + pending : "Friends";
    }
    private void addFriend() {
        client.friends.add(friendField);
        friendField = ""; friendFocus = false;
    }
    private static final String[] SHARE = { "full", "game", "status" }, SHARE_LABELS = { "Share: game + server", "Share: game, hide server", "Share: online only" };
    private void friendsPage(Canvas canvas, int x, int y, int w, int h) {
        final Friends social = client.friends;
        social.wantList();
        button(canvas, "Back to modules", x, y, 116, 22, 0xff303a2d, ACCENT, () -> page("modules"));
        boolean dnd = social.dnd();
        button(canvas, dnd ? "DND: ON" : "DND: OFF", x + w - 150, y, 86, 22, dnd ? 0xff4a3a2a : 0xff303a2d, dnd ? 0xfff2bc71 : MUTED, () -> social.dnd(!social.dnd()));
        button(canvas, "Refresh", x + w - 58, y, 58, 22, 0xff303a2d, MUTED, () -> { social.wantList(); client.cosmetics.refresh(); });
        canvas.text(fit(canvas, social.message(), Math.max(0, w - 278)), x + 124, y + 7, social.available() ? TEXT : MUTED);
        int row = y + 28;
        String hint = friendField.isEmpty() && !friendFocus ? "Add a friend by Minecraft name..." : friendField + (friendFocus ? "_" : "");
        // Short screens fold the privacy and toast rows away so the list keeps room.
        boolean compact = h < 300, settings = !compact || friendSettings;
        int fieldW = w - 66 - (compact ? 70 : 0);
        button(canvas, hint, x, row, fieldW, 24, friendFocus ? 0xff3c4835 : 0xff141b16, friendField.isEmpty() && !friendFocus ? MUTED : TEXT, () -> friendFocus = true);
        button(canvas, "Add", x + fieldW + 6, row, 60, 24, 0xff3c4835, ACCENT, this::addFriend);
        if (compact) button(canvas, friendSettings ? "List" : "Privacy", x + w - 64, row, 64, 24, 0xff303a2d, friendSettings ? ACCENT : TEXT, () -> { friendSettings = !friendSettings; friendsScroll = 0; });
        row += 30;
        if (settings) row = friendSettings(canvas, social, x, row, w, dnd);
        if (compact && friendSettings) return;
        friendRows(canvas, social, x, row, w, y + h - row);
    }
    private int friendSettings(Canvas canvas, final Friends social, int x, int row, int w, boolean dnd) {
        int third = (w - 8) / 3;
        int share = Math.max(0, Arrays.asList(SHARE).indexOf(social.share()));
        button(canvas, SHARE_LABELS[share], x, row, third, 22, 0xff303a2d, share == 0 ? TEXT : ACCENT, () -> social.share(SHARE[(share + 1) % SHARE.length]));
        button(canvas, social.appearOffline() ? "Appear offline: ON" : "Appear offline: OFF", x + third + 4, row, third, 22, 0xff303a2d, social.appearOffline() ? ACCENT : MUTED, () -> social.appearOffline(!social.appearOffline()));
        boolean open = !"none".equals(social.requests());
        button(canvas, open ? "Requests: everyone" : "Requests: no one", x + 2 * (third + 4), row, w - 2 * (third + 4), 22, 0xff303a2d, open ? MUTED : ACCENT, () -> social.requests(open ? "none" : "everyone"));
        row += 28;
        canvas.text("Toasts", x, row + 7, MUTED);
        int toastW = (w - 48) / Friends.TOASTS.length;
        for (int i = 0; i < Friends.TOASTS.length; i++) {
            final String type = Friends.TOASTS[i];
            boolean on = social.toast(type);
            button(canvas, Friends.TOAST_LABELS[i], x + 48 + i * toastW, row, toastW - 4, 22, 0xff303a2d, on && !dnd ? ACCENT : MUTED, () -> social.toast(type, !social.toast(type)));
        }
        return row + 30;
    }
    private void friendRows(Canvas canvas, final Friends social, int x, int row, int w, int height) {
        List<Object> rows = new ArrayList<Object>();
        if (!social.incoming().isEmpty()) { rows.add("Friend requests"); rows.addAll(social.incoming()); }
        rows.add(social.friends().isEmpty() ? "No friends yet. Add someone who plays with Cairn Client." : "Friends");
        rows.addAll(social.friends());
        if (!social.outgoing().isEmpty()) { rows.add("Sent requests"); for (Friends.Person p : social.outgoing()) rows.add(new Object[] { "out", p }); }
        if (!social.blocked().isEmpty()) { rows.add("Blocked"); for (Friends.Person p : social.blocked()) rows.add(new Object[] { "blocked", p }); }
        int rowH = 26, capacity = Math.max(1, height / rowH);
        friendsScroll = clamp(friendsScroll, 0, Math.max(0, rows.size() - capacity));
        for (int i = friendsScroll; i < Math.min(rows.size(), friendsScroll + capacity); i++) {
            int cy = row + (i - friendsScroll) * rowH;
            Object item = rows.get(i);
            if (item instanceof String) { canvas.text(fit(canvas, (String) item, w), x, cy + 9, TEXT); continue; }
            canvas.rect(x, cy, w, rowH - 3, 0xff2a3229);
            if (item instanceof Friends.Friend) friendRow(canvas, social, (Friends.Friend) item, x, cy, w);
            else if (item instanceof Friends.Person) {
                final Friends.Person p = (Friends.Person) item;
                canvas.text(fit(canvas, p.name + " wants to be friends", w - 190), x + 8, cy + 7, TEXT);
                button(canvas, "Accept", x + w - 178, cy + 2, 60, 19, 0xff3c4835, ACCENT, () -> social.accept(p));
                button(canvas, "Decline", x + w - 114, cy + 2, 60, 19, 0xff303a2d, MUTED, () -> social.decline(p));
                button(canvas, "Block", x + w - 50, cy + 2, 48, 19, 0xff303a2d, MUTED, () -> social.block(p.uuid, p.name));
            } else {
                Object[] pair = (Object[]) item;
                final Friends.Person p = (Friends.Person) pair[1];
                boolean out = pair[0].equals("out");
                canvas.text(fit(canvas, p.name.isEmpty() ? p.uuid : p.name, w - 80), x + 8, cy + 7, MUTED);
                if (out) button(canvas, "Cancel", x + w - 62, cy + 2, 60, 19, 0xff303a2d, MUTED, () -> social.cancel(p));
                else button(canvas, "Unblock", x + w - 62, cy + 2, 60, 19, 0xff303a2d, MUTED, () -> social.unblock(p));
            }
        }
        if (rows.size() > capacity) {
            int bar = Math.max(6, height * capacity / rows.size()), position = friendsScroll * Math.max(0, height - bar) / Math.max(1, rows.size() - capacity);
            canvas.rect(x + w + 2, row + position, 2, bar, ACCENT);
        }
    }
    private void friendRow(Canvas canvas, Friends social, Friends.Friend f, int x, int cy, int w) {
        int dot = !f.online() ? 0xff6b7067 : "dnd".equals(f.status) ? 0xfff2bc71 : ACCENT;
        canvas.rect(x + 8, cy + 8, 6, 6, dot);
        boolean join = social.canJoin(f);
        int buttons = (join ? 48 : 0) + 112;
        String name = fit(canvas, f.name, Math.max(0, Math.min(110, w - buttons - 30)));
        canvas.text(name, x + 20, cy + 7, f.online() ? TEXT : MUTED);
        int activityX = x + 28 + Math.max(canvas.textWidth(name), 0);
        canvas.text(fit(canvas, f.activity(), Math.max(0, x + w - buttons - activityX - 6)), activityX, cy + 7, MUTED);
        int bx = x + w - buttons;
        if (join) { button(canvas, "Join", bx, cy + 2, 44, 19, 0xff3c4835, ACCENT, () -> social.join(f)); bx += 48; }
        button(canvas, "Remove", bx, cy + 2, 58, 19, 0xff303a2d, MUTED, () -> social.remove(f));
        button(canvas, "Block", bx + 62, cy + 2, 48, 19, 0xff303a2d, MUTED, () -> social.block(f.uuid, f.name));
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
            rounded(canvas, cx, cy, cardW, cardH, 3, selected ? 0xff44252c : hover ? 0xff303741 : SURFACE);
            if (selected) { canvas.rect(cx, cy, cardW, 2, ACCENT); canvas.rect(cx, cy + cardH - 2, cardW, 2, ACCENT); }
            canvas.rect(cx + 12, cy + 5, 32, 50, 0xff090c10);
            String texture = id.equals("none") ? null : cosmetics.previewTexture(id, now);
            // The cape's outer face sits at (1,1) and is 10x16 in the 64x32 layout.
            if (texture != null) canvas.image(texture, cx + 13, cy + 6, 30, 48, 1 / 64f, 1 / 32f, 11 / 64f, 17 / 32f);
            canvas.text(fit(canvas, capeName(cosmetics, id), cardW - 6), cx + 3, cy + 59, selected ? ACCENT : TEXT);
            String note = !cosmetics.signedIn() ? "Preview" : selected ? "Worn" : id.equals("custom") && "pending".equals(cosmetics.customStatus()) ? "In review" : "";
            if (!note.isEmpty()) canvas.text(fit(canvas, note, cardW - 6), cx + 3, cy + 71, MUTED);
            if (cosmetics.signedIn() && !cosmetics.busy()) controls.add(new Control(cx, cy, cardW, cardH, () -> cosmetics.equip(id.equals("none") ? null : id, cosmetics.equippedBadge())));
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
            rounded(canvas, bx, by, cell, cell, 3, selected ? 0xff44252c : SURFACE);
            if (selected) canvas.rect(bx, by + cell - 2, cell, 2, ACCENT);
            String texture = cosmetics.badgeTexture(id);
            if (texture != null) canvas.image(texture, bx + 4, by + 4, 20, 20, 0, 0, 1, 1);
            else if (id.equals("none")) canvas.text("-", bx + 11, by + 10, MUTED);
            if (cosmetics.signedIn() && !cosmetics.busy()) controls.add(new Control(bx, by, cell, cell, () -> cosmetics.equip(cosmetics.equippedCape(), id)));
        }
        canvas.text(fit(canvas, "Upload your own cape, animated strips too, in the Cairn launcher.", w), x, y + h - 10, MUTED);
    }
    private void moduleList(Canvas canvas, int x, int y, int w, int height) {
        List<Module> list = filtered();
        if (list.isEmpty()) { canvas.text(fit(canvas, "No modules match. Try another search.", w - 12), x + 6, y + 10, MUTED); return; }
        int columns = w >= 540 ? 4 : w >= 400 ? 3 : 2;
        int cellWidth = (w - (columns - 1) * 7) / columns;
        int fittedHeight = height >= 280 && height < 360 ? (height - 16) / 3 : height >= 160 && height < 216 ? (height - 8) / 2 : height;
        int cardHeight = Math.min(102, Math.max(48, fittedHeight));
        int rowHeight = cardHeight + 8;
        int rows = (list.size() + columns - 1) / columns, capacity = Math.max(1, (height + 8) / rowHeight);
        listCapacity = capacity; listColumns = columns;
        listScroll = clamp(listScroll, 0, Math.max(0, rows - capacity));
        if (list.indexOf(client.selected()) < 0) client.select(list.get(0));
        int selectedRow = list.indexOf(client.selected()) / columns;
        if (focus == -1 && !typing() && selectedRow < listScroll) client.select(list.get(Math.min(list.size() - 1, listScroll * columns)));
        for (int i = listScroll * columns; i < Math.min(list.size(), (listScroll + capacity) * columns); i++) {
            final Module module = list.get(i);
            int cx = x + i % columns * (cellWidth + 7), cy = y + (i / columns - listScroll) * rowHeight;
            boolean hover = mouseX >= cx && mouseX < cx + cellWidth && lastMouseY >= cy && lastMouseY < cy + cardHeight;
            rounded(canvas, cx, cy, cellWidth, cardHeight, 4, hover ? 0xff4a5361 : EDGE);
            rounded(canvas, cx + 1, cy + 1, cellWidth - 2, cardHeight - 2, 3, hover ? 0xff202730 : 0xff14181e);
            canvas.text(fit(canvas, module.title, cellWidth - 16), cx + 8, cy + 8, client.enabled(module) ? TEXT : MUTED);
            int index = controls.size();
            controls.add(new Control(cx, cy, cellWidth, cardHeight - 26, () -> openSettings(module)));
            if (focus == index) canvas.rect(cx + 4, cy + 1, cellWidth - 8, 2, ACCENT);
            if (cardHeight >= 72) preview(canvas, module, cx + 8, cy + 24, cellWidth - 16, cardHeight - 54);
            int buttonY = cy + cardHeight - 25;
            int gearIndex = controls.size();
            button(canvas, "", cx + 6, buttonY, 22, 19, SURFACE, MUTED, () -> openSettings(module));
            gear(canvas, cx + 12, buttonY + 4, MUTED);
            if (focus == gearIndex || mouseX >= cx + 6 && mouseX < cx + 28 && lastMouseY >= buttonY && lastMouseY < buttonY + 19) canvas.text("Settings", cx + 8, buttonY - 12, TEXT);
            int toggleIndex = controls.size();
            boolean on = client.enabled(module);
            int tx = cx + 32, tw = cellWidth - 38;
            boolean toggleHover = mouseX >= tx && mouseX < tx + tw && lastMouseY >= buttonY && lastMouseY < buttonY + 19;
            rounded(canvas, tx, buttonY, tw, 19, 3, on ? toggleHover ? 0xff2a5a39 : 0xff1d432b : toggleHover ? 0xff3a424e : SURFACE);
            String label = fit(canvas, on ? "Enabled" : "Disabled", tw - 8);
            canvas.text(label, tx + (tw - canvas.textWidth(label)) / 2, buttonY + 5, on ? 0xffa4e3b4 : MUTED);
            controls.add(new Control(tx, buttonY, tw, 19, () -> client.toggle(module)));
            if (focus == toggleIndex) canvas.rect(tx + 3, buttonY + 16, tw - 6, 2, ACCENT);
        }
        if (rows > capacity) {
            int barHeight = Math.max(6, height * capacity / rows), position = listScroll * Math.max(0, height - barHeight) / Math.max(1, rows - capacity);
            canvas.rect(x + w + 2, y + position, 2, barHeight, ACCENT);
        }
    }
    private void gear(Canvas canvas, int x, int y, int color) {
        canvas.rect(x + 3, y, 4, 12, color);
        canvas.rect(x, y + 4, 10, 4, color);
        canvas.rect(x + 1, y + 2, 8, 8, color);
        canvas.rect(x + 3, y + 4, 4, 4, SURFACE);
    }
    private void preview(Canvas canvas, Module module, int x, int y, int w, int h) {
        int cx = x + w / 2, cy = y + h / 2;
        int color = client.enabled(module) ? 0xffdce1e7 : 0xff858e9b;
        if (module == Module.KEYSTROKES) {
            int size = Math.min(14, Math.max(8, h / 2 - 1));
            for (int row = 0; row < 2; row++) for (int col = 0; col < 3; col++) {
                if (row == 0 && col != 1) continue;
                int kx = cx + (col - 1) * (size + 2) - size / 2, ky = cy + (row - 1) * (size + 2);
                rounded(canvas, kx, ky, size, size, 2, color);
            }
            return;
        }
        if (module == Module.CROSSHAIR) {
            canvas.rect(cx - 14, cy - 1, 10, 2, color); canvas.rect(cx + 4, cy - 1, 10, 2, color);
            canvas.rect(cx - 1, cy - 14, 2, 10, color); canvas.rect(cx - 1, cy + 4, 2, 10, color);
            return;
        }
        if (module == Module.CPS) {
            for (int i = 0; i < 2; i++) {
                int mx = cx - 22 + i * 24;
                rounded(canvas, mx, cy - 12, 20, 25, 5, color);
                canvas.rect(mx + 9, cy - 11, 2, 12, 0xff14181e);
                canvas.rect(mx + 2, cy - 1, 16, 2, 0xff14181e);
            }
            return;
        }
        if (module == Module.ARMOR) {
            for (int i = -1; i <= 1; i++) {
                int ax = cx + i * 20 - 8;
                canvas.rect(ax, cy - 8, 5, 8, color); canvas.rect(ax + 11, cy - 8, 5, 8, color);
                canvas.rect(ax + 4, cy - 5, 8, 18, color); canvas.rect(ax + 3, cy - 7, 10, 5, color);
                canvas.rect(ax + 6, cy - 7, 4, 3, 0xff14181e);
            }
            return;
        }
        if (module == Module.PING) {
            for (int i = 0; i < 4; i++) rounded(canvas, cx - 20 + i * 11, cy + 13 - i * 7, 7, 4 + i * 7, 1, color);
            return;
        }
        if (module == Module.CLOCK || module == Module.SESSION || module == Module.WORLD_TIME || module == Module.ZOOM) {
            int r = Math.min(14, Math.max(7, h / 2 - 2));
            ring(canvas, cx, cy, r, color);
            if (module == Module.ZOOM) {
                for (int i = 0; i < 10; i++) canvas.rect(cx + r - 2 + i, cy + r - 2 + i, 3, 3, color);
            } else if (module == Module.WORLD_TIME) {
                canvas.rect(cx - 2, cy - r - 6, 4, 4, color); canvas.rect(cx - 2, cy + r + 3, 4, 4, color);
                canvas.rect(cx - r - 6, cy - 2, 4, 4, color); canvas.rect(cx + r + 3, cy - 2, 4, 4, color);
            } else {
                canvas.rect(cx - 1, cy - r + 4, 2, r - 2, color); canvas.rect(cx, cy - 1, r - 4, 2, color);
                if (module == Module.SESSION) canvas.rect(cx - 4, cy - r - 5, 8, 3, color);
            }
            return;
        }
        if (module == Module.MEMORY || module == Module.SERVER) {
            for (int i = 0; i < 3; i++) {
                rounded(canvas, cx - 24, cy - 15 + i * 12, 48, 9, 2, color);
                canvas.rect(cx - 20, cy - 12 + i * 12, 3, 3, 0xff14181e);
            }
            return;
        }
        if (module == Module.BIOME) {
            canvas.rect(cx - 3, cy, 6, 17, color);
            for (int i = 0; i < 3; i++) canvas.rect(cx - 8 - i * 4, cy - 18 + i * 9, 16 + i * 8, 10, color);
            return;
        }
        if (module == Module.HELD_ITEM) {
            for (int i = 0; i < 25; i++) canvas.rect(cx - 14 + i, cy + 13 - i, 4, 4, color);
            for (int i = 0; i < 14; i++) canvas.rect(cx - 13 + i, cy - 2 + i, 3, 3, color);
            return;
        }
        String value;
        switch (module) {
            case FPS: value = "FPS"; break;
            case COORDINATES: value = "X Y Z"; break;
            case PING: value = "ms"; break;
            case ARMOR: value = "[|||]"; break;
            case EFFECTS: value = "+"; break;
            case CLOCK: value = "12:00"; break;
            case SPEED: value = ">>"; break;
            case SPRINT: value = ">>>"; break;
            case ZOOM: value = "4x"; break;
            case DIRECTION: value = "N"; break;
            case BIOME: value = "Biome"; break;
            case HELD_ITEM: value = "Item"; break;
            case FOOD: value = "Food"; break;
            case WORLD_TIME: value = "Day"; break;
            case SESSION: value = "00:00"; break;
            case MEMORY: value = "RAM"; break;
            case SERVER: value = "Server"; break;
            default: value = module.title;
        }
        float scale = Math.min(2.4f, Math.min(w / (float) Math.max(1, canvas.textWidth(value)), h / 11f));
        canvas.textScaled(value, cx - Math.round(canvas.textWidth(value) * scale) / 2, cy - Math.round(9 * scale) / 2, color, scale, false);
    }
    private void ring(Canvas canvas, int cx, int cy, int radius, int color) {
        for (int row = -radius; row <= radius; row++) {
            int outer = (int) Math.sqrt(radius * radius - row * row);
            int inner = Math.abs(row) >= radius - 2 ? 0 : (int) Math.sqrt((radius - 2) * (radius - 2) - row * row);
            if (inner == 0) canvas.rect(cx - outer, cy + row, outer * 2 + 1, 1, color);
            else {
                canvas.rect(cx - outer, cy + row, Math.max(1, outer - inner), 1, color);
                canvas.rect(cx + inner, cy + row, Math.max(1, outer - inner), 1, color);
            }
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
        else if (module.hud) list.addAll(Arrays.asList(Setting.AUTO_WIDTH, Setting.WIDTH, Setting.HEIGHT, Setting.PADDING_X, Setting.PADDING_Y, Setting.OPACITY, Setting.TEXT_ALPHA, Setting.ACCENT_ALPHA, Setting.BACKGROUND, Setting.BORDER, Setting.SHADOW, Setting.ALIGN));
        if (module == Module.CLOCK) list.addAll(Arrays.asList(Setting.CLOCK_SECONDS, Setting.CLOCK_24H));
        if (module == Module.ZOOM) list.addAll(Arrays.asList(Setting.ZOOM_FACTOR, Setting.SMOOTH_ZOOM, Setting.SCROLL_ZOOM, Setting.ZOOM_CINEMATIC));
        if (module == Module.SPRINT) list.addAll(Arrays.asList(Setting.SPRINT_AUTO, Setting.SPRINT_RESET));
        return list;
    }
    private void settings(Canvas canvas, int x, int y, int w, int height) {
        Module module = client.selected();
        boolean compact = height < 180;
        boolean side = !compact && w >= 500;
        int fullWidth = w;
        if (side) w -= Math.max(196, w / 3) + 16;
        canvas.text(fit(canvas, module.title + " settings", compact ? w - 80 : w), x, y + 2, TEXT);
        button(canvas, client.enabled(module) ? "Enabled" : "Disabled", x, y + 18, w, 22, 0xff303a2d, client.enabled(module) ? ACCENT : MUTED, () -> client.toggle(module));
        int previewHeight = compact || side ? 0 : height >= 240 ? 88 : 44;
        int px = side ? x + w + 16 : compact ? x + w - 72 : x + 8, py = side ? y + 20 : compact ? y : y + 64;
        int pw = side ? fullWidth - w - 16 : compact ? 72 : w - 16, ph = side ? height - 44 : compact ? 14 : previewHeight - 24;
        if (side) {
            canvas.text("Mod preview", px, y + 2, TEXT);
            float inset = module == Module.ZOOM ? (1 - 1f / client.setting(module, Setting.ZOOM_FACTOR)) / 2 : 0;
            canvas.image("preview/world", px, py, pw, ph, inset, inset, 1 - inset, 1 - inset);
            canvas.rect(px, py, pw, ph, 0x3306090d);
        }
        if (!compact && !side) {
            rounded(canvas, x, y + 44, w, previewHeight, 3, 0xff090c10);
            canvas.text("Preview", x + 8, y + 50, MUTED);
        }
        if (module.hud) client.renderPreview(canvas, module, px, py, pw, ph);
        else preview(canvas, module, px, py, pw, ph);
        List<Setting> options = options(module);
        int rowHeight = 28, first = y + (compact || side ? 40 : 50 + previewHeight), reserved = compact || side ? 66 : 76 + previewHeight;
        if (module == Module.ZOOM && client.keysManaged()) { bindButton(canvas, Bind.ZOOM, x, first, w, 22); first += rowHeight; reserved += rowHeight; }
        int capacity = Math.max(1, (height - reserved) / rowHeight);
        settingsScroll = clamp(settingsScroll, 0, Math.max(0, options.size() - capacity));
        for (int i = settingsScroll; i < Math.min(options.size(), settingsScroll + capacity); i++) {
            final Setting setting = options.get(i);
            int cy = first + (i - settingsScroll) * rowHeight, value = client.setting(module, setting);
            int sliderWidth = Math.min(180, Math.max(112, w / 2));
            boolean numeric = !setting.color() && !setting.toggle() && setting != Setting.ALIGN;
            canvas.text(fit(canvas, setting.label, w - (numeric ? sliderWidth + 8 : 85)), x, cy + 7, MUTED);
            if (setting.color()) {
                canvas.rect(x + w - 76, cy + 6, 12, 12, value);
                button(canvas, "Change", x + w - 59, cy, 59, 23, 0xff303a2d, TEXT, () -> { colorSetting = setting; draftAlpha = client.setting(module, CairnClient.alphaSetting(setting)); setDraft(client.setting(module, setting)); hexFocus = false; colorError = null; focus = 0; brightnessAxis = false; });
            } else if (setting.toggle()) {
                settingToggle(canvas, value == 1, x + w - 64, cy, () -> { client.setting(module, setting, value == 1 ? 0 : 1); client.save(); });
            } else if (setting == Setting.ALIGN) button(canvas, new String[] { "Left", "Center", "Right" }[value], x + w - 78, cy, 78, 23, SURFACE, TEXT, () -> { client.setting(module, setting, (value + 1) % 3); client.save(); });
            else settingSlider(canvas, module, setting, x + w - sliderWidth, cy, sliderWidth);
        }
        int foot = y + Math.max(48, height - 24);
        if (!options.isEmpty()) {
            button(canvas, "Up", x, foot, 31, 22, 0xff303a2d, MUTED, () -> settingsScroll = Math.max(0, settingsScroll - 1));
            button(canvas, "Down", x + 35, foot, 43, 22, 0xff303a2d, MUTED, () -> settingsScroll++);
        }
        button(canvas, "Reset", x + w - 57, foot, 57, 22, 0xff303a2d, MUTED, () -> client.reset(module));
    }
    private void settingSlider(Canvas canvas, Module module, Setting setting, int x, int y, int width) {
        int index = controls.size(), value = client.setting(module, setting);
        String label = String.valueOf(value) + (setting == Setting.SCALE || setting == Setting.OPACITY || setting == Setting.TEXT_ALPHA || setting == Setting.ACCENT_ALPHA ? "%" : "");
        canvas.text(label, x, y + 7, TEXT);
        int trackX = x + 42, trackWidth = width - 50;
        Slider slider = new Slider(module, setting, trackX, trackWidth);
        if (activeSlider != null && activeSlider.module == module && activeSlider.setting == setting) { activeSlider.x = trackX; activeSlider.width = trackWidth; }
        int progress = Math.round((value - setting.min) / (float) (setting.max - setting.min) * trackWidth);
        rounded(canvas, trackX, y + 10, trackWidth, 4, 2, 0xff626c79);
        if (progress > 0) rounded(canvas, trackX, y + 10, progress, 4, 2, PRIMARY);
        rounded(canvas, trackX + progress - 4, y + 7, 8, 10, 4, ACCENT);
        if (focus == index) canvas.rect(x, y + 22, width, 1, ACCENT);
        controls.add(new Control(x, y, width, 23, null, slider));
    }
    private void settingToggle(Canvas canvas, boolean enabled, int x, int y, Runnable action) {
        int index = controls.size();
        boolean hover = mouseX >= x && mouseX < x + 64 && lastMouseY >= y && lastMouseY < y + 23;
        rounded(canvas, x, y, 64, 23, 3, hover ? 0xff303741 : SURFACE);
        canvas.text(enabled ? "ON" : "OFF", x + 4, y + 7, TEXT);
        rounded(canvas, x + 32, y + 5, 28, 14, 7, enabled ? 0xff285d39 : 0xff414750);
        rounded(canvas, x + (enabled ? 47 : 35), y + 7, 10, 10, 5, enabled ? 0xffa4e3b4 : 0xffdce1e7);
        if (focus == index) canvas.rect(x + 2, y + 21, 60, 2, ACCENT);
        controls.add(new Control(x, y, 64, 23, action));
    }
    private void applyColor() {
        if (!readHex()) { colorError = "Use RRGGBB or RRGGBBAA."; return; }
        client.setting(client.selected(), colorSetting, draftRgb);
        client.setting(client.selected(), CairnClient.alphaSetting(colorSetting), draftAlpha);
        client.save(); colorSetting = null; hexFocus = false; colorError = null; focus = -1; controls.clear(); colorControls.clear();
    }
    private void setDraft(int rgb) {
        draftRgb = rgb & 0xffffff;
        float[] hsv = java.awt.Color.RGBtoHSB(draftRgb >> 16 & 255, draftRgb >> 8 & 255, draftRgb & 255, null);
        hue = hsv[0]; saturation = hsv[1]; brightness = hsv[2]; syncHex();
    }
    private void syncHex() {
        draftRgb = java.awt.Color.HSBtoRGB(hue, saturation, brightness) & 0xffffff;
        hex = String.format(Locale.ROOT, "%06X%02X", draftRgb, Math.round(draftAlpha * 255 / 100f));
        hexSelected = false;
    }
    private boolean readHex() {
        if (hex.length() != 6 && hex.length() != 8) return false;
        try {
            int rgb = (int) Long.parseLong(hex.substring(0, 6), 16);
            float[] hsv = java.awt.Color.RGBtoHSB(rgb >> 16 & 255, rgb >> 8 & 255, rgb & 255, null);
            draftRgb = rgb; hue = hsv[0]; saturation = hsv[1]; brightness = hsv[2];
            if (hex.length() == 8) draftAlpha = Math.round(Integer.parseInt(hex.substring(6), 16) * 100 / 255f);
            return true;
        } catch (NumberFormatException e) { return false; }
    }
    private void checker(Canvas canvas, int x, int y, int w, int h) {
        for (int row = 0; row < h; row += 8) for (int col = 0; col < w; col += 8)
            canvas.rect(x + col, y + row, Math.min(8, w - col), Math.min(8, h - row), ((row / 8 + col / 8) & 1) == 0 ? 0xff757b84 : 0xffaeb5bf);
    }
    private void colorControl(int kind, int x, int y, int w, int h) {
        colorControls.put(controls.size(), kind);
        if (colorDrag == kind) { colorX = x; colorY = y; colorWidth = w - 1; colorHeight = h - 1; }
        controls.add(new Control(x, y, w, h, () -> { colorDrag = kind; colorX = x; colorY = y; colorWidth = w - 1; colorHeight = h - 1; drag(mouseX, lastMouseY); }));
    }
    private void colorPicker(Canvas canvas, int x, int y, int w, int h) {
        canvas.text(fit(canvas, colorError == null ? client.selected().title + " / " + colorSetting.label : colorError, w), x, y + 2, TEXT);
        int pw = Math.min(256, w), ph = Math.max(20, Math.min(144, h - 104)), py = y + 18;
        canvas.rect(x, py, pw, ph, java.awt.Color.HSBtoRGB(hue, 1, 1));
        for (int i = 0; i < 64; i++) {
            int left = i * pw / 64, right = (i + 1) * pw / 64;
            canvas.rect(x + left, py, right - left, ph, ((255 - i * 255 / 63) << 24) | 0xffffff);
        }
        for (int i = 0; i < 64; i++) {
            int top = i * ph / 64, bottom = (i + 1) * ph / 64;
            if (bottom > top) canvas.rect(x, py + top, pw, bottom - top, i * 255 / 63 << 24);
        }
        int sx = x + Math.round(saturation * (pw - 1)), sy = py + Math.round((1 - brightness) * (ph - 1));
        canvas.rect(clamp(sx - 3, x, x + pw - 7), clamp(sy - 3, py, py + ph - 7), 7, 7, 0xff000000);
        canvas.rect(clamp(sx - 2, x + 1, x + pw - 6), clamp(sy - 2, py + 1, py + ph - 6), 5, 5, TEXT);
        colorControl(1, x, py, pw, ph);
        int hy = py + ph + 6, ay = hy + 16, hx = x + 42, hw = pw - 42, ax = x + 74, aw = pw - 74;
        canvas.text("Hue", x, hy + 1, MUTED);
        canvas.text("Alpha " + draftAlpha + "%", x, ay + 1, MUTED);
        checker(canvas, ax, ay, aw, 10);
        for (int i = 0; i < 64; i++) {
            int left = i * hw / 64, right = (i + 1) * hw / 64;
            canvas.rect(hx + left, hy, right - left, 10, java.awt.Color.HSBtoRGB(i / 63f, 1, 1));
            left = i * aw / 64; right = (i + 1) * aw / 64;
            canvas.rect(ax + left, ay, right - left, 10, (i * 255 / 63 << 24) | draftRgb);
        }
        int hueMarker = hx + Math.round(hue * (hw - 4)), alphaMarker = ax + Math.round(draftAlpha / 100f * (aw - 4));
        canvas.rect(hueMarker, hy - 1, 4, 12, 0xff000000); canvas.rect(hueMarker + 1, hy, 2, 10, TEXT);
        canvas.rect(alphaMarker, ay - 1, 4, 12, 0xff000000); canvas.rect(alphaMarker + 1, ay, 2, 10, TEXT);
        colorControl(2, hx, hy, hw, 10); colorControl(3, ax, ay, aw, 10);
        for (Map.Entry<Integer, Integer> field : colorControls.entrySet()) if (field.getKey() == focus) {
            Control c = controls.get(focus); canvas.rect(c.x, c.y + c.h - 1, c.w, 1, ACCENT);
        }
        int hexY = ay + 16, actionY = hexY + 26;
        button(canvas, "#" + hex + (hexFocus ? "_" : ""), x, hexY, Math.min(pw, 150), 20, SURFACE, TEXT, () -> { hexFocus = true; hexSelected = true; });
        checker(canvas, x + 158, hexY, pw - 158, 20);
        canvas.rect(x + 158, hexY, pw - 158, 20, (draftAlpha * 255 / 100 << 24) | draftRgb);
        button(canvas, "Apply color", x, actionY, 90, 22, PRIMARY, TEXT, () -> applyColor());
        button(canvas, "Cancel", x + 98, actionY, 65, 22, SURFACE, TEXT, () -> { colorSetting = null; hexFocus = false; });
        if (w >= 500) {
            int px = x + pw + 20, previewWidth = w - pw - 20;
            canvas.text("Live preview", px, py, TEXT);
            checker(canvas, px, py + 18, previewWidth, Math.min(120, ph));
            client.renderColorPreview(canvas, client.selected(), colorSetting, draftRgb, draftAlpha, px + 8, py + 26, previewWidth - 16, Math.min(104, ph - 16));
        }
        if (h >= 300) canvas.text(fit(canvas, colorError != null ? colorError : "Drag to pick. Arrows adjust; Enter switches saturation/brightness.", w), x, actionY + 32, MUTED);
    }
}
