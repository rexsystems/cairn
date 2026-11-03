package app.cairn.client.mapped.mixin;

import app.cairn.client.mapped.CairnFabricMapped;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.blaze3d.vertex.VertexConsumer;
import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.client.renderer.entity.EntityRenderDispatcher;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Draws the Cairn badge left of a player's name; the name itself stays vanilla. */
@Mixin(EntityRenderer.class)
abstract class CosmeticsMixin {
    @Shadow @Final protected EntityRenderDispatcher entityRenderDispatcher;
    @Shadow public abstract net.minecraft.client.gui.Font getFont();

    @Inject(method = "renderNameTag", at = @At("HEAD"))
    private void cairn$badge(Entity entity, Component name, PoseStack pose, MultiBufferSource buffers, int light, CallbackInfo info) {
        if (entityRenderDispatcher.distanceToSqr(entity) > 4096) return;
        // PlayerRenderer draws the below-name score through this same method; skip that pass so the badge stays on the name (already lifted by vanilla).
        if (!name.equals(entity.getDisplayName())) return;
        ResourceLocation icon = CairnFabricMapped.badge(entity);
        if (icon == null) return;
        // Mirrors vanilla's name tag transform so the badge lines up with the text.
        float x0 = -getFont().width(name) / 2f - 11, x1 = x0 + 9, y0 = ("deadmau5".equals(name.getString()) ? -10 : 0) - .5f, y1 = y0 + 9;
        pose.pushPose();
        pose.translate(0, entity.getBbHeight() + 0.5F, 0);
        pose.mulPose(entityRenderDispatcher.cameraOrientation());
        pose.scale(-.025F, -.025F, .025F);
        com.mojang.math.Matrix4f matrix = pose.last().pose();
        VertexConsumer buffer = buffers.getBuffer(RenderType.text(icon));
        buffer.vertex(matrix, x0, y0, 0).color(255, 255, 255, 255).uv(0, 0).uv2(light).endVertex();
        buffer.vertex(matrix, x0, y1, 0).color(255, 255, 255, 255).uv(0, 1).uv2(light).endVertex();
        buffer.vertex(matrix, x1, y1, 0).color(255, 255, 255, 255).uv(1, 1).uv2(light).endVertex();
        buffer.vertex(matrix, x1, y0, 0).color(255, 255, 255, 255).uv(1, 0).uv2(light).endVertex();
        pose.popPose();
    }
}
