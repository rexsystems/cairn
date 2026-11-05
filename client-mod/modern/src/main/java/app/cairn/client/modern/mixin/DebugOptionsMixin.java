package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.gui.components.DebugScreenOverlay.class)
public abstract class DebugOptionsMixin {
    @Inject(method = "showDebugScreen", at = @At("HEAD"), cancellable = true)
    private void cairn$debug(CallbackInfoReturnable<Boolean> ci) { if (CairnFabric.core() != null && CairnFabric.core().hideVanillaF3()) ci.setReturnValue(false); }
}
