package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.Minecraft.class)
public abstract class InteractionOptionsMixin {
    @Inject(method = "startAttack", at = @At("HEAD"), cancellable = true)
    private void cairn$attack(CallbackInfo ci) { if (CairnFabricMapped.orbitActive()) ci.cancel(); }
    @Inject(method = {"continueAttack", "startUseItem"}, at = @At("HEAD"), cancellable = true)
    private void cairn$interact(CallbackInfo ci) { if (CairnFabricMapped.orbitActive()) ci.cancel(); }
}
