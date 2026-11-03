package app.cairn.client.mapped;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.renderer.PanoramaRenderer;

final class TitleBackground {
    private static final PanoramaRenderer PANORAMA = new PanoramaRenderer(new net.minecraft.client.renderer.CubeMap(net.minecraft.resources.ResourceLocation.withDefaultNamespace("textures/gui/title/background/panorama")));
    static void render(GuiGraphics graphics, int width, int height, float delta) { PANORAMA.render(graphics, width, height, delta, 1); }
}
