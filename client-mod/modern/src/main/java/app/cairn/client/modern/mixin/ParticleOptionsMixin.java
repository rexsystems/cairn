package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.particle.ParticleEngine.class)
public abstract class ParticleOptionsMixin {
    @Inject(method = "add", at = @At("HEAD"), cancellable = true)
    private void cairn$density(CallbackInfo ci) { if (CairnFabric.core() != null && java.util.concurrent.ThreadLocalRandom.current().nextFloat() >= CairnFabric.core().particleRate()) ci.cancel(); }
}
