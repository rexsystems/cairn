package app.cairn.client;

import java.nio.file.*;
import java.util.*;

public final class HudBenchmark {
    private static volatile long sink;
    private static final class Canvas implements CairnClient.Canvas {
        public void rect(int x, int y, int w, int h, int color) { sink += x + y + w + h + color; }
        public void text(String text, int x, int y, int color) { sink += text.length() + x + y + color; }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) { sink += text.length() + x + y + color + (int) scale; }
        public int textWidth(String text) { return text.length() * 6; }
    }
    public static void main(String[] args) throws Exception {
        Path directory = Files.createTempDirectory("cairn-benchmark-");
        try {
            CairnClient client = new CairnClient(directory);
            Canvas canvas = new Canvas();
            StringBuilder report = new StringBuilder("Cairn HUD core CPU benchmark. Counting canvas, no Minecraft, font renderer or GPU; these numbers do not measure in-game FPS.\n");
            for (int scenario = 0; scenario < 2; scenario++) {
                if (scenario == 1) for (CairnClient.Module module : CairnClient.Module.values()) if (!client.enabled(module)) client.toggle(module);
                for (int i = 0; i < 20000; i++) client.renderHud(canvas, 960, 540);
                long[] samples = new long[100];
                for (int batch = 0; batch < samples.length; batch++) {
                    long start = System.nanoTime();
                    for (int i = 0; i < 1000; i++) client.renderHud(canvas, 960, 540);
                    samples[batch] = (System.nanoTime() - start) / 1000;
                }
                Arrays.sort(samples);
                report.append(scenario == 0 ? "Default HUD" : "All modules enabled").append(": median ").append(samples[50]).append(" ns/frame, p95 batch average ").append(samples[95]).append(" ns/frame\n");
            }
            System.out.print(report);
            Path output = Paths.get("client-mod/common/build/reports/hud-benchmark.txt");
            Files.createDirectories(output.getParent()); Files.write(output, report.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8));
        } finally {
            try (java.util.stream.Stream<Path> paths = Files.walk(directory)) { paths.sorted(Comparator.reverseOrder()).forEach(path -> { try { Files.delete(path); } catch (Exception e) { throw new RuntimeException(e); } }); }
        }
    }
}
