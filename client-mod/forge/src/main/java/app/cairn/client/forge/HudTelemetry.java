package app.cairn.client.forge;

import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import net.minecraft.client.Minecraft;

final class HudTelemetry {
    static void update(CairnClient client, Minecraft mc) {
        if (client.enabled(Module.LIGHT_LEVEL)) client.lightLevel(mc.theWorld.getLightFor(net.minecraft.world.EnumSkyBlock.BLOCK, mc.thePlayer.getPosition()), mc.theWorld.getLightFor(net.minecraft.world.EnumSkyBlock.SKY, mc.thePlayer.getPosition()));
        if (client.enabled(Module.PACK_DISPLAY)) client.packNames(String.join(", ", mc.gameSettings.resourcePacks));
        if (client.enabled(Module.BOSS_BAR)) client.bossBar(net.minecraft.entity.boss.BossStatus.statusBarTime > 0 ? net.minecraft.entity.boss.BossStatus.bossName : null, net.minecraft.entity.boss.BossStatus.healthScale);
        if (client.enabled(Module.HORSES)) {
            net.minecraft.entity.Entity vehicle = mc.thePlayer.ridingEntity;
            if (vehicle instanceof net.minecraft.entity.EntityLivingBase) {
                net.minecraft.entity.EntityLivingBase mount = (net.minecraft.entity.EntityLivingBase) vehicle;
                client.horse(mount.getHealth(), mount.getMaxHealth(), mount instanceof net.minecraft.entity.passive.EntityHorse ? ((net.minecraft.entity.passive.EntityHorse) mount).getHorseJumpStrength() : 0);
            } else client.clearHorse();
        }
        if (client.enabled(Module.TNT_TIMER) || client.enabled(Module.ITEM_INFO) || client.enabled(Module.ITEM_DESPAWN)) {
            int fuse = -1, age = 0; double nearest = 36; String name = null;
            for (Object object : mc.theWorld.loadedEntityList) {
                net.minecraft.entity.Entity entity = (net.minecraft.entity.Entity) object;
                double distance = mc.thePlayer.getDistanceSqToEntity(entity);
                if (entity instanceof net.minecraft.entity.item.EntityTNTPrimed && distance <= 1024) { int remaining = ((net.minecraft.entity.item.EntityTNTPrimed) entity).fuse; if (fuse < 0 || remaining < fuse) fuse = remaining; }
                if (entity instanceof net.minecraft.entity.item.EntityItem && distance < nearest) { net.minecraft.entity.item.EntityItem item = (net.minecraft.entity.item.EntityItem) entity; nearest = distance; name = item.getEntityItem().getDisplayName(); age = item.getAge(); }
            }
            client.tntFuse(fuse); client.itemEntity(name, age);
        }
        if (client.enabled(Module.TEAM_TRACKER)) {
            java.util.List<String> teammates = new java.util.ArrayList<String>();
            if (mc.thePlayer.getTeam() != null) for (Object object : mc.theWorld.playerEntities) {
                net.minecraft.entity.player.EntityPlayer other = (net.minecraft.entity.player.EntityPlayer) object;
                if (other != mc.thePlayer && mc.thePlayer.getTeam().equals(other.getTeam()) && mc.thePlayer.getDistanceSqToEntity(other) <= 4096) teammates.add(other.getGameProfile().getName() + " · " + Math.round(mc.thePlayer.getDistanceToEntity(other)) + "m");
            }
            client.teamTracker(teammates);
        }
        if (client.enabled(Module.SCOREBOARD)) {
            net.minecraft.scoreboard.Scoreboard board = mc.theWorld.getScoreboard();
            net.minecraft.scoreboard.ScoreObjective objective = board.getObjectiveInDisplaySlot(1);
            java.util.List<String> lines = new java.util.ArrayList<String>();
            if (objective != null) {
                lines.add(objective.getDisplayName());
                java.util.List<net.minecraft.scoreboard.Score> scores = new java.util.ArrayList<net.minecraft.scoreboard.Score>(board.getSortedScores(objective));
                java.util.Collections.reverse(scores);
                for (net.minecraft.scoreboard.Score score : scores) {
                    if (score.getPlayerName().startsWith("#")) continue;
                    lines.add(net.minecraft.scoreboard.ScorePlayerTeam.formatPlayerName(board.getPlayersTeam(score.getPlayerName()), score.getPlayerName()) + " " + score.getScorePoints());
                    if (lines.size() == 16) break;
                }
            }
            client.scoreboardLines(lines);
        }
    }
}
