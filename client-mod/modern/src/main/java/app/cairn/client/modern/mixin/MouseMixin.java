package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import net.minecraft.client.MouseHandler;
import net.minecraft.client.input.MouseButtonInfo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(MouseHandler.class)
public abstract class MouseMixin {
    @org.spongepowered.asm.mixin.injection.Redirect(method = "turnPlayer", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/player/LocalPlayer;turn(DD)V"))
    private void cairn$orbit(net.minecraft.client.player.LocalPlayer player, double dx, double dy) { CairnFabric.turn(player, dx, dy); }
    @Inject(method = "onButton", at = @At("HEAD"))
    private void cairn$click(long window, MouseButtonInfo button, int action, CallbackInfo ci) { if (action == 1) CairnFabric.click(button.button()); }
    // While zoomed, the wheel changes zoom instead of the hotbar slot.
    @Inject(method = "onScroll", at = @At("HEAD"), cancellable = true)
    private void cairn$scroll(long window, double horizontal, double vertical, CallbackInfo ci) { if (CairnFabric.scroll(vertical)) ci.cancel(); }
}
