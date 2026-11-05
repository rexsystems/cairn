package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
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
        if (CairnFabricMapped.core() == null) return;
        if (events.isEmpty()) CairnFabricMapped.core().bossBar(null, 0);
        else { net.minecraft.client.gui.components.LerpingBossEvent event = events.values().iterator().next(); CairnFabricMapped.core().bossBar(event.getName().getString(), event.getProgress()); }
    }
}
