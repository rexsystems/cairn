package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import net.minecraft.client.Camera;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(Camera.class)
public abstract class ZoomMixin {
    @Inject(method = "calculateFov", at = @At("RETURN"), cancellable = true)
    private void cairn$zoom(float delta, CallbackInfoReturnable<Float> ci) { ci.setReturnValue(ci.getReturnValue() / CairnFabric.zoomFactor()); }
}
