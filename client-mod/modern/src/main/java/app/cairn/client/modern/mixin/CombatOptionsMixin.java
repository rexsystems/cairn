package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.multiplayer.MultiPlayerGameMode.class)
public abstract class CombatOptionsMixin {
    @Inject(method = "handleContainerInput", at = @At("HEAD"), cancellable = true)
    private void cairn$inventoryDrop(int container, int slot, int button, net.minecraft.world.inventory.ContainerInput type, net.minecraft.world.entity.player.Player player, CallbackInfo ci) {
        if (CairnFabric.core() != null && CairnFabric.core().preventDrop() && (type == net.minecraft.world.inventory.ContainerInput.THROW || slot == -999 && type == net.minecraft.world.inventory.ContainerInput.PICKUP)) ci.cancel();
    }
    @Inject(method = "dropItem", at = @At("HEAD"), cancellable = true)
    private void cairn$drop(CallbackInfo ci) { if (CairnFabric.core() != null && CairnFabric.core().preventDrop()) ci.cancel(); }
    @Inject(method = "attack", at = @At("TAIL"))
    private void cairn$attack(net.minecraft.world.entity.player.Player player, net.minecraft.world.entity.Entity target, CallbackInfo ci) { if (CairnFabric.core() != null) CairnFabric.core().attacked(target.getId()); }
}
