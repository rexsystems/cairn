package app.cairn.client.modern;
import app.cairn.client.CairnClient;
import net.minecraft.client.Minecraft;
final class ScoreboardTelemetry {
    static void update(CairnClient client, Minecraft mc) {
        if (!client.enabled(CairnClient.Module.SCOREBOARD)) return;
        net.minecraft.world.scores.Scoreboard board = mc.level.getScoreboard();
        net.minecraft.world.scores.Objective objective = board.getDisplayObjective(net.minecraft.world.scores.DisplaySlot.SIDEBAR);
        java.util.List<String> lines = new java.util.ArrayList<String>();
        if (objective != null) {
            lines.add(objective.getDisplayName().getString());
            java.util.List<net.minecraft.world.scores.PlayerScoreEntry> scores = new java.util.ArrayList<net.minecraft.world.scores.PlayerScoreEntry>(board.listPlayerScores(objective));
            java.util.Collections.sort(scores, java.util.Comparator.comparingInt(net.minecraft.world.scores.PlayerScoreEntry::value).reversed().thenComparing(net.minecraft.world.scores.PlayerScoreEntry::owner, String.CASE_INSENSITIVE_ORDER));
            for (net.minecraft.world.scores.PlayerScoreEntry score : scores) {
                String owner = score.owner(); if (owner.startsWith("#")) continue;
                lines.add(net.minecraft.world.scores.PlayerTeam.formatNameForTeam(board.getPlayersTeam(owner), net.minecraft.network.chat.Component.literal(owner)).getString() + " " + score.value());
                if (lines.size() == 16) break;
            }
        }
        client.scoreboardLines(lines);
    }
}
