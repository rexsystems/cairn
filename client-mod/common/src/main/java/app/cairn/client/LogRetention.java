package app.cairn.client;

import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.*;

final class LogRetention {
    static int prune(Path directory, int keep, int days, long now) throws IOException {
        if (!Files.isDirectory(directory, LinkOption.NOFOLLOW_LINKS)) return 0;
        final Map<Path, Long> modified = new HashMap<Path, Long>();
        try (DirectoryStream<Path> files = Files.newDirectoryStream(directory)) {
            for (Path file : files) {
                if (!file.getFileName().toString().matches("[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]+\\.log\\.gz")) continue;
                BasicFileAttributes attributes = Files.readAttributes(file, BasicFileAttributes.class, LinkOption.NOFOLLOW_LINKS);
                if (attributes.isRegularFile()) modified.put(file, attributes.lastModifiedTime().toMillis());
            }
        }
        List<Path> files = new ArrayList<Path>(modified.keySet());
        Collections.sort(files, (a, b) -> Long.compare(modified.get(b), modified.get(a)));
        long cutoff = now - Math.max(1, days) * 86_400_000L;
        int removed = 0;
        for (int i = Math.max(1, keep); i < files.size(); i++) {
            Path file = files.get(i);
            BasicFileAttributes current = Files.readAttributes(file, BasicFileAttributes.class, LinkOption.NOFOLLOW_LINKS);
            if (current.isRegularFile() && current.lastModifiedTime().toMillis() < cutoff && Files.deleteIfExists(file)) removed++;
        }
        return removed;
    }
}
