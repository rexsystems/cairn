package app.cairn.client.modern.mixin;
import app.cairn.client.modern.CairnFabric;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.gui.components.BossHealthOverlay.class)
public abstract class BossOptionsMixin {
    @Shadow @Final private java.util.Map<java.util.UUID, net.minecraft.client.gui.components.LerpingBossEvent> events;
    @Inject(method = "extractRenderState", at = @At("HEAD"))
    private void cairn$boss(CallbackInfo ci) {
        if (CairnFabric.core() == null) return;
        if (events.isEmpty()) CairnFabric.core().bossBar(null, 0);
        else { net.minecraft.client.gui.components.LerpingBossEvent event = events.values().iterator().next(); CairnFabric.core().bossBar(event.getName().getString(), event.getProgress()); }
    }
}
