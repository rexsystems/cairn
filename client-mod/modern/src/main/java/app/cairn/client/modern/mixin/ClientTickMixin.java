package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Minecraft.class)
public abstract class ClientTickMixin {
    @Inject(method = "tick", at = @At("TAIL"))
    private void cairn$tick(CallbackInfo ci) { CairnFabric.tick(); }
}
