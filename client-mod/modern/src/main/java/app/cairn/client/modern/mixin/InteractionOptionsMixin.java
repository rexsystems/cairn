package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.Minecraft.class)
public abstract class InteractionOptionsMixin {
    @Inject(method = "startAttack", at = @At("HEAD"), cancellable = true)
    private void cairn$attack(CallbackInfoReturnable<Boolean> ci) { if (CairnFabric.orbitActive()) ci.setReturnValue(false); }
    @Inject(method = {"continueAttack", "startUseItem"}, at = @At("HEAD"), cancellable = true)
    private void cairn$interact(CallbackInfo ci) { if (CairnFabric.orbitActive()) ci.cancel(); }
}
