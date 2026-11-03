package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.client.KeyboardHandler;
import net.minecraft.client.input.KeyEvent;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(KeyboardHandler.class)
public abstract class KeyboardMixin {
    @Inject(method = "keyPress", at = @At("HEAD"), cancellable = true)
    private void cairn$key(long window, int action, KeyEvent event, CallbackInfo ci) { if (CairnFabricMapped.keyPress(event.key(), action)) ci.cancel(); }
}
