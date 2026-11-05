package app.cairn.client.mapped;
import app.cairn.client.CairnClient;
import net.minecraft.client.Minecraft;
final class ScoreboardTelemetry {
    static void update(CairnClient client, Minecraft mc) {
        if (!client.enabled(CairnClient.Module.SCOREBOARD)) return;
        net.minecraft.world.scores.Scoreboard board = mc.level.getScoreboard();
        net.minecraft.world.scores.Objective objective = board.getDisplayObjective(1);
        java.util.List<String> lines = new java.util.ArrayList<String>();
        if (objective != null) {
            lines.add(objective.getDisplayName().getString());
            java.util.List<net.minecraft.world.scores.Score> scores = new java.util.ArrayList<net.minecraft.world.scores.Score>(board.getPlayerScores(objective));
            java.util.Collections.sort(scores, java.util.Comparator.comparingInt(net.minecraft.world.scores.Score::getScore).reversed().thenComparing(net.minecraft.world.scores.Score::getOwner, String.CASE_INSENSITIVE_ORDER));
            for (net.minecraft.world.scores.Score score : scores) {
                String owner = score.getOwner(); if (owner.startsWith("#")) continue;
                lines.add(net.minecraft.world.scores.PlayerTeam.formatNameForTeam(board.getPlayersTeam(owner), new net.minecraft.network.chat.TextComponent(owner)).getString() + " " + score.getScore());
                if (lines.size() == 16) break;
            }
        }
        client.scoreboardLines(lines);
    }
}
