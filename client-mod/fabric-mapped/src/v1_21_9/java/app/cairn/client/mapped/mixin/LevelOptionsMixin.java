package app.cairn.client.mapped.mixin;
import app.cairn.client.CairnClient;
import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(Level.class)
public abstract class LevelOptionsMixin {
    @Inject(method = "getDayTime", at = @At("RETURN"), cancellable = true)
    private void cairn$getDayTime(CallbackInfoReturnable<Long> ci) {
        CairnClient core = CairnFabricMapped.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.timeOverride() >= 0) ci.setReturnValue((ci.getReturnValue() / 24000) * 24000 + core.timeOverride());
    }
    @Inject(method = "getRainLevel", at = @At("RETURN"), cancellable = true)
    private void cairn$getRainLevel(float delta, CallbackInfoReturnable<Float> ci) {
        CairnClient core = CairnFabricMapped.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.weatherOverride() >= 0) ci.setReturnValue(core.weatherOverride() >= 1 ? 1f : 0f);
    }
    @Inject(method = "getThunderLevel", at = @At("RETURN"), cancellable = true)
    private void cairn$getThunderLevel(float delta, CallbackInfoReturnable<Float> ci) {
        CairnClient core = CairnFabricMapped.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.weatherOverride() >= 0) ci.setReturnValue(core.weatherOverride() == 2 ? 1f : 0f);
    }
}
