package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(net.minecraft.client.OptionInstance.class)
public abstract class BrightnessOptionsMixin {
    @Inject(method = "get", at = @At("RETURN"), cancellable = true)
    private void cairn$gamma(CallbackInfoReturnable<Object> ci) {
        net.minecraft.client.Minecraft mc = net.minecraft.client.Minecraft.getInstance();
        if (CairnFabricMapped.core() != null && mc.options != null && (Object) this == mc.options.gamma() && CairnFabricMapped.core().enabled(app.cairn.client.CairnClient.Module.BRIGHTNESS)) ci.setReturnValue(Double.valueOf(CairnFabricMapped.core().setting(app.cairn.client.CairnClient.Module.BRIGHTNESS, app.cairn.client.CairnClient.Setting.BRIGHTNESS_GAMMA) / 100.0));
    }
}
