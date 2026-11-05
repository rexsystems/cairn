package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;
@Mixin(net.minecraft.client.renderer.LightTexture.class)
public abstract class BrightnessOptionsMixin {
    @Redirect(method = "updateLightTexture", at = @At(value = "FIELD", target = "Lnet/minecraft/client/Options;gamma:D"))
    private double cairn$gamma(net.minecraft.client.Options options) {
        return CairnFabricMapped.core() != null && CairnFabricMapped.core().enabled(app.cairn.client.CairnClient.Module.BRIGHTNESS) ? CairnFabricMapped.core().setting(app.cairn.client.CairnClient.Module.BRIGHTNESS, app.cairn.client.CairnClient.Setting.BRIGHTNESS_GAMMA) / 100.0 : options.gamma;
    }
}
