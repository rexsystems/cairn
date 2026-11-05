package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.multiplayer.MultiPlayerGameMode.class)
public abstract class CombatOptionsMixin {
    @Inject(method = "handleInventoryMouseClick", at = @At("HEAD"), cancellable = true)
    private void cairn$inventoryDrop(int container, int slot, int button, net.minecraft.world.inventory.ClickType type, net.minecraft.world.entity.player.Player player, CallbackInfo ci) {
        if (CairnFabricMapped.core() != null && CairnFabricMapped.core().preventDrop() && (type == net.minecraft.world.inventory.ClickType.THROW || slot == -999 && type == net.minecraft.world.inventory.ClickType.PICKUP)) ci.cancel();
    }
    @Inject(method = "attack", at = @At("TAIL"))
    private void cairn$attack(net.minecraft.world.entity.player.Player player, net.minecraft.world.entity.Entity target, CallbackInfo ci) { if (CairnFabricMapped.core() != null) CairnFabricMapped.core().attacked(target.getId()); }
}
