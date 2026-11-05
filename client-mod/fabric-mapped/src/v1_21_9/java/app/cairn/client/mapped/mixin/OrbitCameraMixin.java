package app.cairn.client.mapped.mixin;
import app.cairn.client.mapped.CairnFabricMapped;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(net.minecraft.client.Camera.class)
public abstract class OrbitCameraMixin {
    @Shadow private float eyeHeight;
    @Shadow private float eyeHeightOld;
    @Shadow protected abstract void setRotation(float yaw, float pitch);
    @Shadow protected abstract void setPosition(double x, double y, double z);
    @Shadow protected abstract void move(float x, float y, float z);
    @Shadow private float getMaxZoom(float distance) { throw new AssertionError(); }
    @Inject(method = "setup", at = @At("RETURN"))
    private void cairn$orbit(net.minecraft.world.level.Level level, net.minecraft.world.entity.Entity entity, boolean detached, boolean mirrored, float delta, CallbackInfo ci) {
        net.minecraft.client.Minecraft mc = net.minecraft.client.Minecraft.getInstance();
        if (!CairnFabricMapped.orbitActive() || mc.player == null || mc.getCameraEntity() != mc.player) return;
        setRotation(CairnFabricMapped.orbitYaw(), CairnFabricMapped.orbitPitch());
        setPosition(mc.player.xo + (mc.player.getX() - mc.player.xo) * delta, mc.player.yo + (mc.player.getY() - mc.player.yo) * delta + eyeHeightOld + (eyeHeight - eyeHeightOld) * delta, mc.player.zo + (mc.player.getZ() - mc.player.zo) * delta);
        move(-getMaxZoom(4), 0, 0);
    }
}
