package app.cairn.client.mapped;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.renderer.PanoramaRenderer;

final class TitleBackground {
    private static final PanoramaRenderer PANORAMA = new PanoramaRenderer(TitleScreen.CUBE_MAP);
    static void render(GuiGraphics graphics, int width, int height, float delta) { PANORAMA.render(delta, 1); }
}
