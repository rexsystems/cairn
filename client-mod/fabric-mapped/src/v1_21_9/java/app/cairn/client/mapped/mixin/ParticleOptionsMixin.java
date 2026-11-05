package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.particle.ParticleEngine.class)
public abstract class ParticleOptionsMixin {
    @Inject(method = "add", at = @At("HEAD"), cancellable = true)
    private void cairn$density(CallbackInfo ci) { if (CairnFabricMapped.core() != null && java.util.concurrent.ThreadLocalRandom.current().nextFloat() >= CairnFabricMapped.core().particleRate()) ci.cancel(); }
}
