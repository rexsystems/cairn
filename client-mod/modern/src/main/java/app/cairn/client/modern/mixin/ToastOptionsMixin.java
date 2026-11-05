package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.gui.components.toasts.ToastManager.class)
public abstract class ToastOptionsMixin {
    @Inject(method = "addToast", at = @At("HEAD"), cancellable = true)
    private void cairn$mute(CallbackInfo ci) { if (CairnFabric.core() != null && CairnFabric.core().muteToasts()) ci.cancel(); }
}
