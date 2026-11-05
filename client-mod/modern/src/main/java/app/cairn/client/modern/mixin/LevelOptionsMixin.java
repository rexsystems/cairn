package app.cairn.client.modern.mixin;
import app.cairn.client.CairnClient;
import app.cairn.client.modern.CairnFabric;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(Level.class)
public abstract class LevelOptionsMixin {
    @Inject(method = "getOverworldClockTime", at = @At("RETURN"), cancellable = true)
    private void cairn$getOverworldClockTime(CallbackInfoReturnable<Long> ci) {
        CairnClient core = CairnFabric.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.timeOverride() >= 0) ci.setReturnValue((ci.getReturnValue() / 24000) * 24000 + core.timeOverride());
    }
    @Inject(method = "getDefaultClockTime", at = @At("RETURN"), cancellable = true)
    private void cairn$getDefaultClockTime(CallbackInfoReturnable<Long> ci) {
        CairnClient core = CairnFabric.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.timeOverride() >= 0) ci.setReturnValue((ci.getReturnValue() / 24000) * 24000 + core.timeOverride());
    }
    @Inject(method = "getRainLevel", at = @At("RETURN"), cancellable = true)
    private void cairn$getRainLevel(float delta, CallbackInfoReturnable<Float> ci) {
        CairnClient core = CairnFabric.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.weatherOverride() >= 0) ci.setReturnValue(core.weatherOverride() >= 1 ? 1f : 0f);
    }
    @Inject(method = "getThunderLevel", at = @At("RETURN"), cancellable = true)
    private void cairn$getThunderLevel(float delta, CallbackInfoReturnable<Float> ci) {
        CairnClient core = CairnFabric.core();
        if ((Object) this instanceof net.minecraft.client.multiplayer.ClientLevel && core != null && core.weatherOverride() >= 0) ci.setReturnValue(core.weatherOverride() == 2 ? 1f : 0f);
    }
}
