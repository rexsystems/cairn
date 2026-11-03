package app.cairn.client.mapped.mixin;

import app.cairn.client.mapped.CairnFabricMapped;
import net.minecraft.client.player.AbstractClientPlayer;
import net.minecraft.client.resources.PlayerSkin;
import net.minecraft.resources.ResourceLocation;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** A Cairn cape replaces the Mojang one; elytra keep their own texture. */
@Mixin(AbstractClientPlayer.class)
abstract class CapeMixin {
    @Inject(method = "getSkin", at = @At("RETURN"), cancellable = true)
    private void cairn$cape(CallbackInfoReturnable<PlayerSkin> info) {
        ResourceLocation cape = CairnFabricMapped.cape((AbstractClientPlayer) (Object) this);
        PlayerSkin skin = info.getReturnValue();
        if (cape != null && skin != null) info.setReturnValue(new PlayerSkin(skin.texture(), skin.textureUrl(), cape, skin.elytraTexture(), skin.model(), skin.secure()));
    }
}
