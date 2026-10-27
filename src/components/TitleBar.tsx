import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CairnMark } from "../assets/CairnMark";

/** Custom window chrome: drag strip + min/max/close that collapse to dots until hover. */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    let alive = true;
    const sync = async () => {
      try {
        const m = await win.isMaximized();
        if (alive) setMaximized(m);
      } catch { /* ignore */ }
    };
    void sync();
    let unlisten: (() => void) | undefined;
    void win.onResized(() => { void sync(); }).then((u) => { unlisten = u; });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  const win = () => getCurrentWindow();

  return (
    <header
      className="titlebar"
      data-tauri-drag-region
      onDoubleClick={() => void win().toggleMaximize()}
    >
      <div className="titlebar-brand" data-tauri-drag-region>
        <CairnMark className="titlebar-mark" size={18} />
        <span className="titlebar-name">cairn</span>
      </div>
      <div className="titlebar-spacer" data-tauri-drag-region />
      <div className="titlebar-controls" role="group" aria-label="Window">
        <button
          type="button"
          className="titlebar-btn titlebar-min"
          aria-label="Minimize"
          onClick={() => void win().minimize()}
        >
          <span className="titlebar-glyph" aria-hidden="true" />
        </button>
        <button
          type="button"
          className="titlebar-btn titlebar-max"
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={() => void win().toggleMaximize()}
        >
          <span className={`titlebar-glyph ${maximized ? "is-restore" : ""}`} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="titlebar-btn titlebar-close"
          aria-label="Close"
          onClick={() => void win().close()}
        >
          <span className="titlebar-glyph" aria-hidden="true" />
        </button>
      </div>
    </header>
  );
}
