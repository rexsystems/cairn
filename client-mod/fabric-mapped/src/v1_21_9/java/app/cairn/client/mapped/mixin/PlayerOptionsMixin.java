package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.player.LocalPlayer.class)
public abstract class PlayerOptionsMixin {
    @Inject(method = "drop(Z)Z", at = @At("HEAD"), cancellable = true)
    private void cairn$drop(CallbackInfoReturnable<Boolean> ci) { if (CairnFabricMapped.core() != null && CairnFabricMapped.core().preventDrop()) ci.setReturnValue(false); }
}
