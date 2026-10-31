import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, on, type ManualPickup, type PendingView } from "../api";
import { useStore } from "../store";
import { bytes } from "../format";
import { Icon } from "./Icon";
import { Modal } from "./ui";

/** Files CurseForge only serves from its website. Cairn watches Downloads and verifies each by hash. */
export function ManualDownloads({ instanceId, onChange, compact }: { instanceId: string; onChange?: () => void; compact?: boolean }) {
  const { toast } = useStore();
  const [view, setView] = useState<PendingView | null>(null);
  const load = useCallback(() => { api.pendingDownloads(instanceId).then(setView).catch(() => setView(null)); }, [instanceId]);
  useEffect(load, [load]);
  useEffect(() => {
    const sub = on<ManualPickup>("manual-download", (p) => {
      if (p.instanceId !== instanceId) return;
      load();
      onChange?.();
    });
    return () => { sub.then((u) => u()); };
  }, [instanceId, load, onChange]);

  if (!view || view.items.length === 0) return null;

  const openPage = async (fileId?: string) => {
    try { await api.openManualDownload(instanceId, fileId); load(); } catch (e) { toast(String(e), true); }
  };
  const pick = async () => {
    const f = await open({ multiple: false, filters: [{ name: "Mod or pack file", extensions: ["jar", "zip"] }] });
    if (!f || Array.isArray(f)) return;
    try { toast(`Added ${await api.importManualDownload(instanceId, f)}`); load(); onChange?.(); } catch (e) { toast(String(e), true); }
  };
  const dismiss = async (fileId: string) => {
    try { await api.dismissManualDownload(instanceId, fileId); load(); } catch (e) { toast(String(e), true); }
  };

  return (
    <div className="notice warn stack" role="status" style={{ gap: 10, flexDirection: "column", alignItems: "stretch" }}>
      <div className="row" style={{ gap: 10 }}>
        <Icon name="download" />
        <div style={{ flex: 1, minWidth: 200 }}>
          <b>{view.items.length} {view.items.length === 1 ? "file needs" : "files need"} a manual download</b>
          <p className="small muted">
            The authors only allow downloads on CurseForge. Click Download on each page; Cairn {view.watching ? "is watching" : "will watch"}{" "}
            {view.downloadsDir ? <code>{view.downloadsDir}</code> : "your Downloads folder"} and moves matching files in after checking their hash.
          </p>
        </div>
        <button className="btn" onClick={() => openPage()}><Icon name="external" />Open all pages</button>
        <button className="btn" onClick={pick}><Icon name="upload" />Choose file</button>
      </div>
      {!compact && (
        <div className="card content-list">
          {view.items.map((p) => (
            <div key={p.fileId} className="content-row">
              {p.icon ? <img className="content-icon" src={p.icon} alt="" /> : <div className="content-icon" aria-hidden="true" />}
              <div className="content-main">
                <div className="content-title">{p.title}</div>
                <div className="content-sub">{[p.fileName, p.size ? bytes(p.size) : null].filter(Boolean).join("  ·  ")}</div>
              </div>
              <div className="row" style={{ flexWrap: "nowrap", gap: 4 }}>
                <button className="btn btn-sm" onClick={() => openPage(p.fileId)}><Icon name="external" size={14} />Open page</button>
                <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Skip ${p.title}`} title="Skip this file" onClick={() => dismiss(p.fileId)}><Icon name="close" size={14} /></button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function ManualDownloadsDialog({ instanceId, onClose }: { instanceId: string; onClose: () => void }) {
  return (
    <Modal wide title="Manual downloads" onClose={onClose} footer={<button className="btn" onClick={onClose}>Done</button>}>
      <ManualDownloads instanceId={instanceId} />
      <p className="small muted" style={{ marginTop: 12 }}>Once every file is in place this list empties by itself.</p>
    </Modal>
  );
}
