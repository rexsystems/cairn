package app.cairn.client.mapped;
import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import net.minecraft.client.Minecraft;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
final class HudTelemetry {
    static void update(CairnClient client, Minecraft mc) {
        if (client.enabled(Module.LIGHT_LEVEL)) client.lightLevel(mc.level.getBrightness(net.minecraft.world.level.LightLayer.BLOCK, mc.player.blockPosition()), mc.level.getBrightness(net.minecraft.world.level.LightLayer.SKY, mc.player.blockPosition()));
        if (client.enabled(Module.PACK_DISPLAY)) client.packNames(String.join(", ", mc.getResourcePackRepository().getSelectedIds()));
        if (client.enabled(Module.HORSES)) {
            net.minecraft.world.entity.Entity vehicle = mc.player.getVehicle();
            if (vehicle instanceof net.minecraft.world.entity.LivingEntity) {
                net.minecraft.world.entity.LivingEntity mount = (net.minecraft.world.entity.LivingEntity) vehicle;
                client.horse(mount.getHealth(), mount.getMaxHealth(), mount.getAttribute(net.minecraft.world.entity.ai.attributes.Attributes.JUMP_STRENGTH) == null ? 0 : mount.getAttributeValue(net.minecraft.world.entity.ai.attributes.Attributes.JUMP_STRENGTH));
            } else client.clearHorse();
        }
        if (client.enabled(Module.ELYTRA)) { ItemStack chest = mc.player.getItemBySlot(net.minecraft.world.entity.EquipmentSlot.CHEST); client.elytra(chest.getItem() != Items.ELYTRA ? -1 : chest.getMaxDamage() - chest.getDamageValue(), mc.player.isFallFlying()); }
        if (client.enabled(Module.SHIELD)) { ItemStack main = mc.player.getMainHandItem(), off = mc.player.getOffhandItem(); ItemStack shield = off.getItem() == Items.SHIELD ? off : main; client.shield(shield.getItem() != Items.SHIELD ? -1 : shield.getMaxDamage() - shield.getDamageValue()); }
        if (client.enabled(Module.TOTEM)) {
            int count = 0;
            for (int i = 0; i < 36; i++) { ItemStack stack = mc.player.getInventory().getItem(i); if (stack.getItem() == Items.TOTEM_OF_UNDYING) count += stack.getCount(); }
            if (mc.player.getOffhandItem().getItem() == Items.TOTEM_OF_UNDYING) count += mc.player.getOffhandItem().getCount();
            client.totems(count);
        }
        if (client.enabled(Module.ATTACK_INDICATOR)) client.attackCooldown(mc.player.getAttackStrengthScale(0));
        if (client.enabled(Module.TNT_TIMER) || client.enabled(Module.ITEM_INFO) || client.enabled(Module.ITEM_DESPAWN)) {
            int fuse = -1, age = 0; double nearest = 36; String name = null;
            for (net.minecraft.world.entity.Entity entity : mc.level.entitiesForRendering()) {
                double distance = mc.player.distanceToSqr(entity);
                if (entity instanceof net.minecraft.world.entity.item.PrimedTnt && distance <= 1024) { int remaining = ((net.minecraft.world.entity.item.PrimedTnt) entity).getFuse(); if (fuse < 0 || remaining < fuse) fuse = remaining; }
                if (entity instanceof net.minecraft.world.entity.item.ItemEntity && distance < nearest) { net.minecraft.world.entity.item.ItemEntity item = (net.minecraft.world.entity.item.ItemEntity) entity; nearest = distance; name = item.getItem().getHoverName().getString(); age = item.getAge(); }
            }
            client.tntFuse(fuse); client.itemEntity(name, age);
        }
        if (client.enabled(Module.TEAM_TRACKER)) {
            java.util.List<String> teammates = new java.util.ArrayList<String>();
            if (mc.player.getTeam() != null) for (net.minecraft.world.entity.player.Player other : mc.level.players()) if (other != mc.player && mc.player.getTeam().equals(other.getTeam()) && mc.player.distanceToSqr(other) <= 4096) teammates.add(other.getName().getString() + " · " + Math.round(mc.player.distanceTo(other)) + "m");
            client.teamTracker(teammates);
        }
        ScoreboardTelemetry.update(client, mc);
    }
}
