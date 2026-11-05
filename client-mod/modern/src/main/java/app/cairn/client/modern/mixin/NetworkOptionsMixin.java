package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.multiplayer.ClientPacketListener.class)
public abstract class NetworkOptionsMixin {
    @Inject(method = "handleSetTime", at = @At("TAIL"))
    private void cairn$time(net.minecraft.network.protocol.game.ClientboundSetTimePacket packet, CallbackInfo ci) { if (CairnFabric.core() != null) CairnFabric.core().serverTime(packet.gameTime()); }
    @Inject(method = "handleEntityEvent", at = @At("TAIL"))
    private void cairn$event(net.minecraft.network.protocol.game.ClientboundEntityEventPacket packet, CallbackInfo ci) {
        net.minecraft.client.Minecraft mc = net.minecraft.client.Minecraft.getInstance();
        if (CairnFabric.core() == null || mc.level == null) return;
        net.minecraft.world.entity.Entity entity = packet.getEntity(mc.level);
        if (entity == null) return;
        if (packet.getEventId() == 35 && entity == mc.player) CairnFabric.core().totemPop();
        
    }
    @Inject(method = "handleDamageEvent", at = @At("TAIL"))
    private void cairn$damage(net.minecraft.network.protocol.game.ClientboundDamageEventPacket packet, CallbackInfo ci) {
        if (CairnFabric.core() != null && net.minecraft.client.Minecraft.getInstance().player != null) CairnFabric.core().entityHurt(packet.entityId(), packet.entityId() == net.minecraft.client.Minecraft.getInstance().player.getId());
    }
}
