package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.player.LocalPlayer.class)
public abstract class PlayerOptionsMixin {
    @Inject(method = "canDropItems", at = @At("HEAD"), cancellable = true)
    private void cairn$drop(CallbackInfoReturnable<Boolean> ci) { if (CairnFabric.core() != null && CairnFabric.core().preventDrop()) ci.setReturnValue(false); }
}
