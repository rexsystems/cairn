package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.client.MouseHandler;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(MouseHandler.class)
public abstract class MouseMixin {
    @Inject(method = "onPress", at = @At("HEAD"))
    private void cairn$click(long window, int button, int action, int modifiers, CallbackInfo ci) { if (action == 1) CairnFabricMapped.click(button); }
    // While zoomed, the wheel changes zoom instead of the hotbar slot.
    @Inject(method = "onScroll", at = @At("HEAD"), cancellable = true)
    private void cairn$scroll(long window, double horizontal, double vertical, CallbackInfo ci) { if (CairnFabricMapped.scroll(vertical)) ci.cancel(); }
}
