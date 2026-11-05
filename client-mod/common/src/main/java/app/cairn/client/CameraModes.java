package app.cairn.client;

/** Restores a temporary perspective only while Cairn still owns that choice. */
public final class CameraModes<T> {
    private T previous, applied;
    private int mode;
    private boolean interrupted;
    public T update(T current, boolean snap, boolean automatic, T rear, T front) {
        int wanted = snap ? 2 : automatic ? 1 : 0;
        if (mode != 0 && !current.equals(applied)) { mode = 0; previous = null; interrupted = true; }
        if (wanted == 0) interrupted = false;
        if (mode != wanted) {
            if (mode != 0 && current.equals(applied)) current = previous;
            mode = 0; previous = null;
            if (wanted != 0 && !interrupted) {
                previous = current; mode = wanted; applied = wanted == 2 ? front : rear;
                current = applied;
            }
        }
        return current;
    }
}
