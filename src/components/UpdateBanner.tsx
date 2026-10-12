import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { Icon } from "./Icon";
import { useStore } from "../store";

const DISMISS_KEY = "cairn.update.dismissed";

type Pending = { version: string; update: NonNullable<Awaited<ReturnType<typeof api.checkForUpdate>>> };

/** Top-of-window strip when a launcher update is available. Dismissible per version. */
export function UpdateBanner() {
  const { toast } = useStore();
  const [pending, setPending] = useState<Pending | null>(null);
  const [busy, setBusy] = useState(false);
  const checked = useRef(false);

  useEffect(() => {
    if (checked.current) return;
    checked.current = true;
    let alive = true;
    (async () => {
      const u = await api.checkForUpdate();
      if (!alive || !u) return;
      try {
        if (sessionStorage.getItem(DISMISS_KEY) === u.version) return;
      } catch { /* ignore */ }
      setPending({ version: u.version, update: u });
    })();
    return () => { alive = false; };
  }, []);

  if (!pending) return null;

  const dismiss = () => {
    try { sessionStorage.setItem(DISMISS_KEY, pending.version); } catch { /* ignore */ }
    setPending(null);
  };

  const install = async () => {
    setBusy(true);
    try {
      await pending.update.downloadAndInstall();
      toast("Update installed. Restarting…");
      await api.relaunch();
    } catch (e) {
      toast(String(e), true);
      setBusy(false);
    }
  };

  return (
    <div className="update-banner" role="status">
      <Icon name="update" size={16} />
      <p>
        Cairn <strong>{pending.version}</strong> is ready.
      </p>
      <div className="row" style={{ gap: 8, marginLeft: "auto" }}>
        <button type="button" className="btn btn-sm btn-quiet" disabled={busy} onClick={dismiss}>
          Later
        </button>
        <button type="button" className="btn btn-sm" disabled={busy} onClick={install}>
          <Icon name="download" size={14} />
          {busy ? "Updating…" : "Update now"}
        </button>
        <button type="button" className="btn btn-quiet btn-icon btn-sm" aria-label="Dismiss" disabled={busy} onClick={dismiss}>
          <Icon name="close" size={14} />
        </button>
      </div>
    </div>
  );
}
