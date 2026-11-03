package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.Hud;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Hud.class)
public abstract class HudMixin {
    @Inject(method = "extractRenderState", at = @At("TAIL"))
    private void cairn$hud(GuiGraphicsExtractor graphics, DeltaTracker delta, CallbackInfo ci) { CairnFabric.hud(graphics); }
    @Inject(method = "extractCrosshair", at = @At("HEAD"), cancellable = true)
    private void cairn$crosshair(CallbackInfo ci) { if (CairnFabric.hideCrosshair()) ci.cancel(); }
}
