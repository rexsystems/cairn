package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.client.gui.Gui;
import com.mojang.blaze3d.vertex.PoseStack;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Gui.class)
public abstract class HudMixin {
    @Inject(method = "render", at = @At("TAIL"))
    private void cairn$hud(PoseStack graphics, float delta, CallbackInfo ci) { CairnFabricMapped.hud(graphics); }
    @Inject(method = "renderCrosshair", at = @At("HEAD"), cancellable = true)
    private void cairn$crosshair(CallbackInfo ci) { if (CairnFabricMapped.hideCrosshair()) ci.cancel(); }
}
