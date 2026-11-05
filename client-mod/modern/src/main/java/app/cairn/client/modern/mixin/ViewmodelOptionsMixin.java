package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.renderer.FirstPersonHandsAndItemsRenderer.class)
public abstract class ViewmodelOptionsMixin {
    @Inject(method = "submitHandsWithItems", at = @At("HEAD"))
    private void cairn$push(float delta, com.mojang.blaze3d.vertex.PoseStack pose, net.minecraft.client.renderer.SubmitNodeCollector collector, net.minecraft.client.renderer.state.level.PlayerRenderState player, net.minecraft.client.renderer.state.level.FirstPersonHandsAndItemsRenderState state, CallbackInfo ci) {
        pose.pushPose(); float scale = CairnFabric.core() == null ? 1 : CairnFabric.core().viewmodelScale(); pose.scale(scale, scale, scale);
    }
    @Inject(method = "submitHandsWithItems", at = @At("RETURN"))
    private void cairn$pop(float delta, com.mojang.blaze3d.vertex.PoseStack pose, net.minecraft.client.renderer.SubmitNodeCollector collector, net.minecraft.client.renderer.state.level.PlayerRenderState player, net.minecraft.client.renderer.state.level.FirstPersonHandsAndItemsRenderState state, CallbackInfo ci) { pose.popPose(); }
}
