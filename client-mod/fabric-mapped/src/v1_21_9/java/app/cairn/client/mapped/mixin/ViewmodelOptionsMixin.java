package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.renderer.ItemInHandRenderer.class)
public abstract class ViewmodelOptionsMixin {
    @Inject(method = "renderHandsWithItems", at = @At("HEAD"))
    private void cairn$push(float delta, com.mojang.blaze3d.vertex.PoseStack pose, net.minecraft.client.renderer.SubmitNodeCollector collector, net.minecraft.client.player.LocalPlayer player, int light, CallbackInfo ci) {
        pose.pushPose(); float scale = CairnFabricMapped.core() == null ? 1 : CairnFabricMapped.core().viewmodelScale(); pose.scale(scale, scale, scale);
    }
    @Inject(method = "renderHandsWithItems", at = @At("RETURN"))
    private void cairn$pop(float delta, com.mojang.blaze3d.vertex.PoseStack pose, net.minecraft.client.renderer.SubmitNodeCollector collector, net.minecraft.client.player.LocalPlayer player, int light, CallbackInfo ci) { pose.popPose(); }
}
