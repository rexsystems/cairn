import { useEffect, useState } from "react";
import { api, type SharePreview } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { Modal } from "../components/ui";
import { loaderLabel } from "../format";
import { runJoin } from "../lib/join";

/** Rebuilds an instance from a share code after showing what it contains. Invite codes (code@server) can also join. */
export function PasteShareCode({ onClose }: { onClose: () => void }) {
  const { toast, go, refreshInstances, play } = useStore();
  const [code, setCode] = useState("");
  const [manifest, setManifest] = useState<SharePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setManifest(null);
    setError(null);
    if (!code.trim()) return;
    let live = true;
    api.readShareCode(code).then((m) => live && setManifest(m)).catch((e) => live && setError(String(e)));
    return () => { live = false; };
  }, [code]);

  const create = async () => {
    setBusy(true);
    try {
      const r = await api.importShareCode(code);
      await refreshInstances();
      const notes = [
        r.pending.length ? `${r.pending.length} need a manual download` : "",
        ...r.failed,
      ].filter(Boolean);
      toast(`Created ${r.instance.name} with ${r.installed} files${notes.length ? `. ${notes.join(". ")}` : ""}`, r.failed.length > 0);
      go({ name: "instance", id: r.instance.id });
      onClose();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };

  // Uses an instance that already matches when there is one, so joining twice doesn't make copies.
  const createAndJoin = async () => {
    setBusy(true);
    try {
      const from = { code: code.trim() };
      const id = await runJoin(await api.joinPlan(from), from, play);
      await refreshInstances();
      go({ name: "instance", id });
      onClose();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };

  const fromCf = manifest?.items.filter((i) => i.s === "c").length ?? 0;
  return (
    <Modal title="Paste share code" onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
        <button className={manifest?.address ? "btn btn-quiet" : "btn"} onClick={create} disabled={!manifest || busy}><Icon name="plus" />{busy ? "Creating..." : "Create instance"}</button>
        {manifest?.address && <button className="btn btn-play" onClick={createAndJoin} disabled={busy}><Icon name="play" />Create and join</button>}
      </>
    }>
      <label className="field">
        <span>Share code</span>
        <textarea className="input mono share-code" value={code} onChange={(e) => setCode(e.target.value)} placeholder="CAIRN1:..." data-autofocus />
      </label>
      {error && <div className="notice error" role="alert"><Icon name="warn" />{error}</div>}
      {manifest && (
        <div className="card" style={{ padding: 14 }}>
          <b>{manifest.name}</b>
          <p className="small muted">
            Minecraft {manifest.mc}, {loaderLabel[manifest.loader] ?? manifest.loader}{manifest.lv ? ` ${manifest.lv}` : ""}, {manifest.items.length === 1 ? "1 file" : `${manifest.items.length} files`}
          </p>
          {manifest.address && <p className="small">Invite to <code className="mono">{manifest.address}</code></p>}
          {fromCf > 0 && <p className="small muted">{fromCf} from CurseForge. Some of those may need a download in your browser.</p>}
        </div>
      )}
    </Modal>
  );
}
