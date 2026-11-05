package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.gui.components.toasts.ToastComponent.class)
public abstract class ToastOptionsMixin {
    @Inject(method = "addToast", at = @At("HEAD"), cancellable = true)
    private void cairn$mute(CallbackInfo ci) { if (CairnFabricMapped.core() != null && CairnFabricMapped.core().muteToasts()) ci.cancel(); }
}
