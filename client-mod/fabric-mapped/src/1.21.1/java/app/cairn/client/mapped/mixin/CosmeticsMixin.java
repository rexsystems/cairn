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
    private void cairn$badge(Entity entity, Component name, PoseStack pose, MultiBufferSource buffers, int light, float partialTick, CallbackInfo info) {
        if (entityRenderDispatcher.distanceToSqr(entity) > 4096) return;
        // PlayerRenderer draws the below-name score through this same method; skip that pass so the badge stays on the name (already lifted by vanilla).
        if (!name.equals(entity.getDisplayName())) return;
        ResourceLocation icon = CairnFabricMapped.badge(entity);
        if (icon == null) return;
        // Mirrors vanilla's name tag transform so the badge lines up with the text.
        float x0 = -getFont().width(name) / 2f - 11, x1 = x0 + 9, y0 = ("deadmau5".equals(name.getString()) ? -10 : 0) - .5f, y1 = y0 + 9;
        net.minecraft.world.phys.Vec3 at = entity.getAttachments().getNullable(net.minecraft.world.entity.EntityAttachment.NAME_TAG, 0, entity.getViewYRot(partialTick));
        if (at == null) return;
        pose.pushPose();
        pose.translate(at.x, at.y + .5, at.z);
        pose.mulPose(entityRenderDispatcher.cameraOrientation());
        pose.scale(.025F, -.025F, .025F);
        org.joml.Matrix4f matrix = pose.last().pose();
        VertexConsumer buffer = buffers.getBuffer(RenderType.text(icon));
        buffer.addVertex(matrix, x0, y0, 0).setColor(-1).setUv(0, 0).setLight(light);
        buffer.addVertex(matrix, x0, y1, 0).setColor(-1).setUv(0, 1).setLight(light);
        buffer.addVertex(matrix, x1, y1, 0).setColor(-1).setUv(1, 1).setLight(light);
        buffer.addVertex(matrix, x1, y0, 0).setColor(-1).setUv(1, 0).setLight(light);
        pose.popPose();
    }
}
