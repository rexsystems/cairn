package app.cairn.client;

import java.util.ArrayList;
import java.util.List;

/** Shared title controls; vanilla supplies the world, multiplayer, settings and quit actions. */
public final class CairnTitle {
    private static final class Action {
        final String label; final Runnable run; final boolean enabled;
        int x, y, w, h;
        Action(String label, Runnable run, boolean enabled) { this.label = label; this.run = run; this.enabled = enabled; }
    }
    private final List<Action> actions = new ArrayList<Action>();
    private int focus = -1;
    public void add(String label, Runnable run, boolean enabled) { actions.add(new Action(label, run, enabled)); }
    public void render(CairnClient.Canvas canvas, int width, int height, int mx, int my) {
        canvas.rect(0, 0, width, height, 0xb80a0d12);
        int w = Math.min(210, width - 32), h = Math.max(16, Math.min(28, (height - 96) / Math.max(1, actions.size()) - 5));
        int total = actions.size() * (h + 5), y = Math.max(62, (height - total) / 2 + 24), x = (width - w) / 2;
        int logoY = Math.max(8, y - 52);
        canvas.image("badge/classic", width / 2 - 12, logoY, 24, 24, 0, 0, 1, 1);
        String title = "CAIRN CLIENT";
        canvas.text(title, (width - canvas.textWidth(title)) / 2, logoY + 30, 0xffeeeeee);
        for (int i = 0; i < actions.size(); i++) {
            Action action = actions.get(i);
            action.x = x; action.y = y; action.w = w; action.h = h;
            boolean hover = mx >= x && mx < x + w && my >= y && my < y + h;
            int color = action.label.equals("Quit game") ? 0xff44252c : 0xee10151b;
            CairnMenu.rounded(canvas, x, y, w, h, 3, action.enabled && (hover || focus == i) ? 0xff363e49 : color);
            if (focus == i) canvas.rect(x, y + h - 2, w, 2, 0xffff858c);
            canvas.text(action.label, x + (w - canvas.textWidth(action.label)) / 2, y + (h - 8) / 2, action.enabled ? 0xffeeeeee : 0xffaeb5bf);
            y += h + 5;
        }
        String footer = "Cairn / Right Shift: HUD and modules";
        if (width >= 280) canvas.text(footer, (width - canvas.textWidth(footer)) / 2, height - 13, 0xffaeb5bf);
    }
    public void click(int x, int y) {
        for (Action action : actions) if (action.enabled && x >= action.x && x < action.x + action.w && y >= action.y && y < action.y + action.h) { action.run.run(); return; }
    }
    public void key(boolean activate, boolean backwards) {
        if (activate) { if (focus >= 0 && actions.get(focus).enabled) actions.get(focus).run.run(); return; }
        for (int i = 0; i < actions.size(); i++) {
            focus = focus < 0 ? backwards ? actions.size() - 1 : 0 : Math.floorMod(focus + (backwards ? -1 : 1), actions.size());
            if (actions.get(focus).enabled) return;
        }
    }
}
