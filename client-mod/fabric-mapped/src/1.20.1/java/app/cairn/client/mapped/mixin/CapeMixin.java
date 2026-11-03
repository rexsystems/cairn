package app.cairn.client.mapped.mixin;

import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.client.player.AbstractClientPlayer;
import net.minecraft.resources.ResourceLocation;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** A Cairn cape replaces the Mojang one; the cape layer only draws once isCapeLoaded is true. */
@Mixin(AbstractClientPlayer.class)
abstract class CapeMixin {
    @Inject(method = "getCloakTextureLocation", at = @At("HEAD"), cancellable = true)
    private void cairn$cape(CallbackInfoReturnable<ResourceLocation> info) {
        ResourceLocation cape = CairnFabricMapped.cape((AbstractClientPlayer) (Object) this);
        if (cape != null) info.setReturnValue(cape);
    }

    @Inject(method = "isCapeLoaded", at = @At("HEAD"), cancellable = true)
    private void cairn$loaded(CallbackInfoReturnable<Boolean> info) {
        if (CairnFabricMapped.cape((AbstractClientPlayer) (Object) this) != null) info.setReturnValue(true);
    }
}
