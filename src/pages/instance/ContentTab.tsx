import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, fileSrc, type CompatReport, type ContentItem, type ContentKind, type InstanceView, type UpdateInfo } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { EmptyState, ErrorState, Loading } from "../../components/ui";
import { CompatIssues } from "../../components/CompatIssues";
import { ManualDownloads } from "../../components/ManualDownloads";
import { bytes } from "../../format";
import { KIND_FOLDER, KIND_LABEL, KIND_TYPE } from "./kinds";

const CONTENT_PAGE = 60;

export function ContentTab({ inst, onChange }: { inst: InstanceView; onChange: () => void }) {
  const { go, toast } = useStore();
  const [kind, setKind] = useState<ContentKind>(inst.loader === "vanilla" || inst.useHorizon ? "resourcepacks" : "mods");
  const [items, setItems] = useState<ContentItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [visible, setVisible] = useState(CONTENT_PAGE);
  const [updates, setUpdates] = useState<UpdateInfo[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [report, setReport] = useState<CompatReport | null>(null);
  const [scanning, setScanning] = useState(false);

  const load = useCallback(() => {
    setError(null);
    api.listContent(inst.id, kind).then(setItems).catch((e) => setError(String(e)));
  }, [inst.id, kind]);
  useEffect(() => { setItems(null); setVisible(CONTENT_PAGE); load(); }, [load]);
  useEffect(() => { setVisible(CONTENT_PAGE); }, [q]);

  const kinds: ContentKind[] = inst.useHorizon
    ? ["resourcepacks", "shaderpacks", "datapacks"]
    : inst.loader === "vanilla"
      ? ["resourcepacks", "shaderpacks", "datapacks"]
      : ["mods", "resourcepacks", "shaderpacks", "datapacks"];
  const filtered = (items ?? []).filter((i) => !q.trim() || i.title.toLowerCase().includes(q.toLowerCase()) || i.fileName.toLowerCase().includes(q.toLowerCase()));
  const shown = filtered.slice(0, visible);
  const updatesFor = new Map((updates ?? []).map((u) => [u.fileName, u]));

  const toggle = async (it: ContentItem) => {
    try { await api.toggleContent(inst.id, kind, it.fileName); load(); } catch (e) { toast(String(e), true); }
  };
  const remove = async (it: ContentItem) => {
    try { await api.removeContent(inst.id, kind, it.fileName); toast(`Removed ${it.title}`); load(); onChange(); } catch (e) { toast(String(e), true); }
  };
  const addPaths = async (files: string[]) => {
    if (!files.length) return;
    try { const added = await api.addContentFiles(inst.id, kind, files); toast(`Added ${added.length} ${added.length === 1 ? "file" : "files"}`); load(); onChange(); } catch (e) { toast(String(e), true); }
  };
  const addFiles = async () => {
    const files = await open({ multiple: true, filters: [{ name: kind === "mods" ? "Mod jar" : "Pack zip", extensions: kind === "mods" ? ["jar"] : ["zip", "jar"] }] });
    if (!files) return;
    await addPaths(Array.isArray(files) ? files : [files]);
  };
  const scan = useCallback(async (quiet = false) => {
    setScanning(true);
    try {
      const r = await api.checkCompat(inst.id);
      setReport(r);
      if (!quiet) toast(r.issues.length ? `${r.issues.length} ${r.issues.length === 1 ? "problem" : "problems"} found` : `All ${r.mods} mods look compatible`);
    } catch (e) { if (!quiet) toast(String(e), true); } finally { setScanning(false); }
  }, [inst.id, toast]);
  const afterFix = () => { load(); onChange(); void scan(true); };
  const check = async () => {
    setChecking(true);
    try {
      const u = await api.checkUpdates(inst.id);
      setUpdates(u);
      load();
      toast(u.length ? `${u.length} ${u.length === 1 ? "update" : "updates"} available` : "Everything is up to date");
    } catch (e) { toast(String(e), true); } finally { setChecking(false); }
  };
  const applyAll = async (list: UpdateInfo[]) => {
    setUpdating(true);
    try {
      const n = await api.applyUpdates(inst.id, list);
      toast(`Updated ${n} ${n === 1 ? "file" : "files"}`);
      setUpdates((u) => (u ?? []).filter((x) => !list.includes(x)));
      load();
      if (report) void scan(true);
    } catch (e) { toast(String(e), true); } finally { setUpdating(false); }
  };

  return (
    <div
      className="stack"
      style={{ gap: 16, outline: dragOver ? "2px dashed var(--accent)" : undefined, outlineOffset: 4, borderRadius: 10 }}
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        const paths = Array.from(e.dataTransfer.files).map((f) => (f as File & { path?: string }).path).filter(Boolean) as string[];
        if (paths.length) void addPaths(paths);
        else toast("Drop files from disk (jar/zip). Browser-only drops have no path.", true);
      }}
    >
      <div className="row" style={{ gap: 12 }}>
        <div className="segmented" role="group" aria-label="Content type">
          {kinds.map((k) => <button key={k} aria-pressed={kind === k} onClick={() => setKind(k)}>{KIND_LABEL[k]}</button>)}
        </div>
        <label className="search" style={{ maxWidth: 240, minWidth: 160 }}>
          <span className="sr-only">Filter installed content</span>
          <Icon name="search" size={16} />
          <input className="input" placeholder={`Filter ${KIND_LABEL[kind].toLowerCase()}`} value={q} onChange={(e) => setQ(e.target.value)} />
        </label>
        <span className="spacer" />
        <button className="btn btn-icon" aria-label={`Open ${KIND_LABEL[kind]} folder`} title="Open folder" onClick={() => api.openDataPath(`${inst.path}/${KIND_FOLDER[kind]}`).catch((e) => toast(String(e), true))}><Icon name="folder" /></button>
        {kind === "mods" && <button className="btn" onClick={() => scan()} disabled={scanning}><Icon name="check" />{scanning ? "Checking..." : "Check mods"}</button>}
        <button className="btn" onClick={check} disabled={checking}><Icon name="update" />{checking ? "Checking..." : "Check for updates"}</button>
        <button className="btn" onClick={addFiles}><Icon name="upload" />Add files</button>
        <button className="btn" onClick={() => go({ name: "discover", instanceId: inst.id, projectType: KIND_TYPE[kind] })}><Icon name="plus" />Find {KIND_LABEL[kind].toLowerCase()}</button>
      </div>

      <ManualDownloads instanceId={inst.id} onChange={afterFix} />
      {kind === "mods" && report && report.issues.filter((i) => i.key !== "manual-downloads").length > 0 && (
        <CompatIssues instanceId={inst.id} issues={report.issues.filter((i) => i.key !== "manual-downloads")} onFixed={afterFix} />
      )}
      {updates && updates.length > 0 && (
        <div className="notice" role="status">
          <Icon name="update" />
          <div style={{ flex: 1 }}>
            <b>{updates.length} {updates.length === 1 ? "update" : "updates"} available</b>
            <p className="small muted">{updates.slice(0, 4).map((u) => `${u.title} ${u.newVersion}`).join(", ")}{updates.length > 4 ? ` and ${updates.length - 4} more` : ""}</p>
          </div>
          <button className="btn" onClick={() => applyAll(updates)} disabled={updating}>{updating ? "Updating..." : "Update all"}</button>
        </div>
      )}

      {error && <ErrorState title={`${KIND_LABEL[kind]} could not be listed`} error={error} onRetry={load} />}
      {!error && items === null && <Loading label={`Reading ${KIND_LABEL[kind].toLowerCase()}`} />}
      {items && items.length === 0 && (
        <EmptyState title={`No ${KIND_LABEL[kind].toLowerCase()} installed`} action={
          <button className="btn" onClick={() => go({ name: "discover", instanceId: inst.id, projectType: KIND_TYPE[kind] })}><Icon name="discover" />Find {KIND_LABEL[kind].toLowerCase()} on Modrinth</button>
        }>
          Install from Modrinth, add files, or drag jars/zips onto this panel.
        </EmptyState>
      )}
      {items && items.length > 0 && filtered.length === 0 && <EmptyState icon="search" title="Nothing matches that filter" />}
      {shown.length > 0 && (
        <div className="card content-list">
          {shown.map((it) => {
            const up = updatesFor.get(it.fileName);
            return (
              <div key={it.fileName} className={`content-row ${it.enabled ? "" : "disabled"}`}>
                {it.icon ? <img className="content-icon" src={fileSrc(it.icon)} alt="" /> : <div className="content-icon" aria-hidden="true" />}
                <div className="content-main">
                  <div className="content-title">
                    {it.title}
                    {!it.enabled && <span className="chip" style={{ marginLeft: 8 }}>Disabled</span>}
                    {up && <span className="chip chip-moss" style={{ marginLeft: 8 }}>Update: {up.newVersion}</span>}
                  </div>
                  <div className="content-sub">{[it.version, it.authors && `by ${it.authors}`, it.fileName].filter(Boolean).join("  ·  ")}</div>
                </div>
                <div className="content-extra small muted">{bytes(it.size)}</div>
                <div className="row" style={{ flexWrap: "nowrap", gap: 4 }}>
                  {up && <button className="btn btn-sm" onClick={() => applyAll([up])} disabled={updating} aria-label={`Update ${it.title}`}><Icon name="download" size={14} /></button>}
                  <label className="switch" title={it.enabled ? "Disable" : "Enable"}>
                    <input type="checkbox" role="switch" checked={it.enabled} onChange={() => toggle(it)} aria-label={`${it.enabled ? "Disable" : "Enable"} ${it.title}`} />
                    <span className="track" aria-hidden="true" />
                  </label>
                  <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Remove ${it.title}`} title="Remove" onClick={() => remove(it)}><Icon name="trash" size={16} /></button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      {filtered.length > shown.length && (
        <button className="btn" onClick={() => setVisible((n) => n + CONTENT_PAGE)}>
          Show more ({shown.length} of {filtered.length})
        </button>
      )}
      <p className="small muted">Drag and drop {kind === "mods" ? ".jar" : ".zip"} files here, or use Add files.</p>
    </div>
  );
}
