import { useEffect, useState } from "react";
import { api, type ShareManifest } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { Modal } from "../components/ui";
import { loaderLabel } from "../format";

/** Rebuilds an instance from a share code after showing what it contains. */
export function PasteShareCode({ onClose }: { onClose: () => void }) {
  const { toast, go, refreshInstances } = useStore();
  const [code, setCode] = useState("");
  const [manifest, setManifest] = useState<ShareManifest | null>(null);
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

  const fromCf = manifest?.items.filter((i) => i.s === "c").length ?? 0;
  return (
    <Modal title="Paste share code" onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
        <button className="btn" onClick={create} disabled={!manifest || busy}><Icon name="plus" />{busy ? "Creating..." : "Create instance"}</button>
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
          {fromCf > 0 && <p className="small muted">{fromCf} from CurseForge. Some of those may need a download in your browser.</p>}
        </div>
      )}
    </Modal>
  );
}
