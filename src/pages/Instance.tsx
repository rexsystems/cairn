import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, fileSrc, on, type ContentItem, type ContentKind, type FileEntry, type HorizonModItem, type InstanceView, type LaunchProfile, type LoaderVersion, type LogLine, type McVersion, type ServerEntry, type UpdateInfo, type World } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, InstanceIcon, Loading, Modal, PopoverMenu, Select, Switch } from "../components/ui";
import { PlayButton } from "../components/PlayButton";
import { InstanceCover } from "../components/InstanceCover";
import { bytes, loaderLabel, playTime, relativeTime } from "../format";

const BASE_TABS = [
  { id: "content", label: "Content" },
  { id: "worlds", label: "Worlds" },
  { id: "servers", label: "Servers" },
  { id: "screenshots", label: "Screenshots" },
  { id: "logs", label: "Logs" },
  { id: "options", label: "Options" },
];

const KIND_LABEL: Record<ContentKind, string> = { mods: "Mods", resourcepacks: "Resource packs", shaderpacks: "Shaders", datapacks: "Datapacks" };
const KIND_TYPE: Record<ContentKind, string> = { mods: "mod", resourcepacks: "resourcepack", shaderpacks: "shader", datapacks: "datapack" };
const KIND_FOLDER: Record<ContentKind, string> = { mods: "mods", resourcepacks: "resourcepacks", shaderpacks: "shaderpacks", datapacks: "datapacks" };

export function InstancePage({ id, initialTab }: { id: string; initialTab?: string }) {
  const { refreshInstances, go, toast, games } = useStore();
  const [inst, setInst] = useState<InstanceView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState(initialTab ?? "content");
  const [menu, setMenu] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [exporting, setExporting] = useState(false);
  const menuBtnRef = useRef<HTMLButtonElement>(null);

  const load = useCallback(() => {
    api.getInstance(id).then((i) => { setInst(i); setError(null); }).catch((e) => setError(String(e)));
  }, [id]);
  useEffect(load, [load, games[id]?.state]);
  useEffect(() => {
    if (inst && !inst.useHorizon && tab === "horizon-mods") setTab("content");
  }, [inst, tab]);

  if (error) return <ErrorState title="This instance could not be opened" error={error} onRetry={load} />;
  if (!inst) return <Loading label="Opening instance" />;

  const changeIcon = async () => {
    const file = await open({ multiple: false, filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] });
    if (!file) return;
    try { setInst(await api.setInstanceIcon(inst.id, file as string)); refreshInstances(); } catch (e) { toast(String(e), true); }
  };
  const duplicate = async () => {
    setMenu(false);
    try { const c = await api.duplicateInstance(inst.id); await refreshInstances(); toast(`Created ${c.name}`); go({ name: "instance", id: c.id }); } catch (e) { toast(String(e), true); }
  };

  const running = games[inst.id] && games[inst.id].state !== "exited";
  const tabs = inst.useHorizon
    ? [{ id: "horizon-mods", label: "Horizon mods" }, ...BASE_TABS]
    : BASE_TABS;

  return (
    <>
      <button className="btn btn-quiet btn-sm" style={{ alignSelf: "flex-start" }} onClick={() => go({ name: "library" })}><Icon name="back" size={14} />Library</button>
      <header className="inst-head inst-banner">
        <InstanceCover inst={inst} className={`inst-cover tone-${inst.loader}`} />
        <div className="inst-banner-scrim" aria-hidden="true" />
        <button className="btn btn-quiet" style={{ padding: 0, height: "auto", border: 0 }} onClick={changeIcon} aria-label="Change instance icon" title="Change icon">
          <InstanceIcon name={inst.name} src={inst.iconPath} large />
        </button>
        <div className="inst-head-main">
          <h1>{inst.name}</h1>
          <div className="row">
            <span className="chip">Minecraft {inst.mcVersion}</span>
            <span className="chip">{loaderLabel[inst.loader]}{inst.loaderVersion ? ` ${inst.loaderVersion}` : ""}</span>
            {inst.useHorizon && <span className="chip">Horizon</span>}
            {inst.pack && <span className="chip">From {inst.pack.title} {inst.pack.versionNumber}</span>}
            {running && <span className="chip chip-moss">{games[inst.id].state === "running" ? "Running" : "Starting"}</span>}
          </div>
          <div className="stat-line">
            <span>Play time <b>{inst.playSeconds >= 60 ? playTime(inst.playSeconds) : "none yet"}</b></span>
            <span>Last played <b>{relativeTime(inst.lastPlayed)}</b></span>
          </div>
        </div>
        <div className="inst-head-actions">
          <button className="btn btn-icon" aria-label="Open instance folder" title="Open folder" onClick={() => api.openDataPath(inst.path).catch((e) => toast(String(e), true))}><Icon name="folder" /></button>
          <button ref={menuBtnRef} className="btn btn-icon" aria-label="More actions" aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu(!menu)}><Icon name="more" /></button>
          <PopoverMenu open={menu} onClose={() => setMenu(false)} anchorRef={menuBtnRef}>
            <button role="menuitem" className="menu-item" onClick={duplicate}><Icon name="copy" />Duplicate</button>
            <button role="menuitem" className="menu-item" onClick={() => { setMenu(false); setExporting(true); }}><Icon name="download" />Export as .mrpack</button>
            <button role="menuitem" className="menu-item" onClick={async () => {
              setMenu(false);
              const dest = await save({ defaultPath: `${inst.name}-backup.zip`, filters: [{ name: "Cairn backup", extensions: ["zip"] }] });
              if (!dest) return;
              try { await api.backupInstance(inst.id, dest as string); toast("Backup saved"); } catch (e) { toast(String(e), true); }
            }}><Icon name="download" />Backup instance</button>
            <button role="menuitem" className="menu-item" onClick={async () => { setMenu(false); try { await api.installInstance(inst.id); toast("Game files are up to date"); } catch (e) { toast(String(e), true); } }}><Icon name="update" />Repair game files</button>
            <hr className="divider" style={{ margin: "6px 0" }} />
            <button role="menuitem" className="menu-item" style={{ color: "var(--ember)" }} onClick={() => { setMenu(false); setConfirmDelete(true); }}><Icon name="trash" />Delete instance</button>
          </PopoverMenu>
          <PlayButton id={inst.id} />
        </div>
      </header>

      <div>
        <div className="tabs" role="tablist" aria-label="Instance sections">
          {tabs.map((t) => (
            <button key={t.id} role="tab" id={`tab-${t.id}`} aria-selected={tab === t.id} aria-controls={`panel-${t.id}`} className="tab" onClick={() => setTab(t.id)}>{t.label}</button>
          ))}
        </div>
        <div role="tabpanel" id={`panel-${tab}`} aria-labelledby={`tab-${tab}`} style={{ paddingTop: 20 }}>
          {tab === "horizon-mods" && inst.useHorizon && <HorizonModsTab inst={inst} />}
          {tab === "content" && <ContentTab inst={inst} onChange={() => { load(); refreshInstances(); }} />}
          {tab === "worlds" && <WorldsTab inst={inst} />}
          {tab === "servers" && <ServersTab inst={inst} />}
          {tab === "screenshots" && <ScreenshotsTab inst={inst} />}
          {tab === "logs" && <LogsTab inst={inst} />}
          {tab === "options" && <OptionsTab inst={inst} onSaved={(i) => { setInst(i); refreshInstances(); }} />}
        </div>
      </div>

      {confirmDelete && (
        <Modal title={`Delete ${inst.name}?`} onClose={() => setConfirmDelete(false)} footer={
          <>
            <button className="btn btn-quiet" onClick={() => setConfirmDelete(false)}>Keep it</button>
            <button className="btn btn-danger" onClick={async () => {
              try { await api.deleteInstance(inst.id); await refreshInstances(); toast(`Deleted ${inst.name}`); go({ name: "library" }); } catch (e) { toast(String(e), true); }
            }}><Icon name="trash" />Delete forever</button>
          </>
        }>
          <p>This removes the instance folder, including its worlds, screenshots and mods. Shared game files stay for other instances.</p>
        </Modal>
      )}
      {exporting && <ExportDialog inst={inst} onClose={() => setExporting(false)} />}
    </>
  );
}

function HorizonModsTab({ inst }: { inst: InstanceView }) {
  const { toast } = useStore();
  const [items, setItems] = useState<HorizonModItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(() => {
    setError(null);
    api.listHorizonMods(inst.id).then(setItems).catch((e) => setError(String(e)));
  }, [inst.id]);
  useEffect(() => { setItems(null); load(); }, [load]);

  const addFiles = async () => {
    const files = await open({ multiple: true, filters: [{ name: "Horizon mod", extensions: ["hmod"] }] });
    if (!files) return;
    try {
      const added = await api.addHorizonModFiles(inst.id, Array.isArray(files) ? files : [files]);
      toast(`Added ${added.length} ${added.length === 1 ? "mod" : "mods"}`);
      load();
    } catch (e) { toast(String(e), true); }
  };
  const installZoom = async () => {
    setBusy(true);
    try {
      const m = await api.installCairnZoomMod(inst.id);
      toast(`Installed ${m.name} (hold C in-game)`);
      load();
    } catch (e) { toast(String(e), true); } finally { setBusy(false); }
  };

  if (error) return <ErrorState title="Horizon mods could not be listed" error={error} onRetry={load} />;
  if (!items) return <Loading label="Reading Horizon mods" />;

  const hasZoom = items.some((m) => m.id === "cairn_zoom");

  return (
    <div className="stack" style={{ gap: 16 }}>
      <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
        <button className="btn" onClick={addFiles}><Icon name="plus" />Add .hmod</button>
        {!hasZoom && (
          <button className="btn btn-quiet" onClick={installZoom} disabled={busy}>
            <Icon name="download" />{busy ? "Installing…" : "Install Cairn Zoom"}
          </button>
        )}
        <button className="btn btn-quiet btn-icon" aria-label="Open horizon-mods folder" title="Open folder" onClick={() => api.openDataPath(`${inst.path}/horizon-mods`).catch((e) => toast(String(e), true))}><Icon name="folder" /></button>
      </div>
      <p className="small muted">Horizon runs `.hmod` packages (Rust → WASM), not Fabric/Forge jars. Drop files here or use Add. Enable a mod, then Play.</p>
      {items.length === 0 ? (
        <EmptyState icon="box" title="No Horizon mods yet">
          Install Cairn Zoom to try hold-to-zoom, or add any `.hmod` from playhorizon.dev.
        </EmptyState>
      ) : (
        <div className="card content-list">
          {items.map((m) => (
            <div key={m.fileName} className="content-row">
              <div className="content-icon" style={{ display: "grid", placeItems: "center" }} aria-hidden="true"><Icon name="box" /></div>
              <div className="content-main">
                <div className="content-title">{m.name}{m.version ? ` ${m.version}` : ""}</div>
                <div className="content-sub">{m.description || m.id}{m.capabilities.length ? ` · ${m.capabilities.join(", ")}` : ""} · {bytes(m.size)}</div>
              </div>
              <div className="row" style={{ flexWrap: "nowrap", gap: 4, alignItems: "center" }}>
                <Switch
                  checked={m.enabled}
                  onChange={async () => {
                    try { await api.toggleHorizonMod(inst.id, m.id); load(); } catch (e) { toast(String(e), true); }
                  }}
                  label={m.enabled ? "On" : "Off"}
                />
                <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Remove ${m.name}`} onClick={async () => {
                  try { await api.removeHorizonMod(inst.id, m.fileName); toast(`Removed ${m.name}`); load(); } catch (e) { toast(String(e), true); }
                }}><Icon name="trash" size={16} /></button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ExportDialog({ inst, onClose }: { inst: InstanceView; onClose: () => void }) {
  const { toast } = useStore();
  const [config, setConfig] = useState(true);
  const [options, setOptions] = useState(false);
  const [busy, setBusy] = useState(false);
  const run = async () => {
    const dest = await save({ defaultPath: `${inst.name}.mrpack`, filters: [{ name: "Modrinth modpack", extensions: ["mrpack"] }] });
    if (!dest) return;
    setBusy(true);
    try {
      const extra = [...(config ? ["config"] : []), ...(options ? ["options.txt"] : [])];
      const p = await api.exportMrpack(inst.id, dest, extra);
      toast(`Saved ${p}`);
      onClose();
    } catch (e) { toast(String(e), true); } finally { setBusy(false); }
  };
  return (
    <Modal title="Export as .mrpack" onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
        <button className="btn" onClick={run} disabled={busy}><Icon name="download" />{busy ? "Exporting..." : "Choose where to save"}</button>
      </>
    }>
      <p className="muted">Files that came from Modrinth are linked by URL and hash, so the pack stays small. Everything else is packed as an override.</p>
      <label className="check"><input type="checkbox" checked={config} onChange={(e) => setConfig(e.target.checked)} />Include the config folder</label>
      <label className="check"><input type="checkbox" checked={options} onChange={(e) => setOptions(e.target.checked)} />Include options.txt (your keybinds and video settings)</label>
    </Modal>
  );
}

function ContentTab({ inst, onChange }: { inst: InstanceView; onChange: () => void }) {
  const { go, toast } = useStore();
  const [kind, setKind] = useState<ContentKind>(inst.loader === "vanilla" ? "resourcepacks" : "mods");
  const [items, setItems] = useState<ContentItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [updates, setUpdates] = useState<UpdateInfo[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  const load = useCallback(() => {
    setError(null);
    api.listContent(inst.id, kind).then(setItems).catch((e) => setError(String(e)));
  }, [inst.id, kind]);
  useEffect(() => { setItems(null); load(); }, [load]);

  const kinds: ContentKind[] = inst.loader === "vanilla"
    ? ["resourcepacks", "shaderpacks", "datapacks"]
    : ["mods", "resourcepacks", "shaderpacks", "datapacks"];
  const filtered = (items ?? []).filter((i) => !q.trim() || i.title.toLowerCase().includes(q.toLowerCase()) || i.fileName.toLowerCase().includes(q.toLowerCase()));
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
        <button className="btn" onClick={check} disabled={checking}><Icon name="update" />{checking ? "Checking..." : "Check for updates"}</button>
        <button className="btn" onClick={addFiles}><Icon name="upload" />Add files</button>
        <button className="btn" onClick={() => go({ name: "discover", instanceId: inst.id, projectType: KIND_TYPE[kind] })}><Icon name="plus" />Find {KIND_LABEL[kind].toLowerCase()}</button>
      </div>

      {updates && updates.length > 0 && (
        <div className="notice" role="status">
          <Icon name="update" />
          <div style={{ flex: 1 }}>
            <b>{updates.length} {updates.length === 1 ? "update" : "updates"} from Modrinth</b>
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
      {filtered.length > 0 && (
        <div className="card content-list">
          {filtered.map((it) => {
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
      <p className="small muted">Drag and drop {kind === "mods" ? ".jar" : ".zip"} files here, or use Add files.</p>
    </div>
  );
}

function ServersTab({ inst }: { inst: InstanceView }) {
  const { play, toast } = useStore();
  const [servers, setServers] = useState<ServerEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [ip, setIp] = useState("");
  const load = useCallback(() => {
    setError(null);
    api.listServers(inst.id).then(setServers).catch((e) => setError(String(e)));
  }, [inst.id]);
  useEffect(load, [load]);
  const persist = async (next: ServerEntry[]) => {
    try {
      await api.saveServers(inst.id, next);
      setServers(next);
    } catch (e) {
      toast(String(e), true);
    }
  };
  const add = async () => {
    if (!ip.trim()) return toast("Enter a server address", true);
    await persist([...(servers ?? []), { name: name.trim() || ip.trim(), ip: ip.trim() }]);
    setName("");
    setIp("");
  };
  if (error) return <ErrorState title="Servers could not be read" error={error} onRetry={load} />;
  if (!servers) return <Loading label="Reading servers.dat" />;
  return (
    <div className="stack" style={{ gap: 16 }}>
      <div className="row" style={{ alignItems: "flex-end" }}>
        <label className="field" style={{ flex: "1 1 160px" }}><span>Name</span><input className="input" value={name} onChange={(e) => setName(e.target.value)} placeholder="My server" /></label>
        <label className="field" style={{ flex: "1 1 220px" }}><span>Address</span><input className="input" value={ip} onChange={(e) => setIp(e.target.value)} placeholder="play.example.net" onKeyDown={(e) => e.key === "Enter" && add()} /></label>
        <button className="btn" onClick={add}><Icon name="plus" />Add</button>
        <button className="btn btn-icon" aria-label="Open servers folder" title="Open instance folder" onClick={() => api.openDataPath(inst.path).catch((e) => toast(String(e), true))}><Icon name="folder" /></button>
      </div>
      {servers.length === 0 && <EmptyState icon="globe" title="No servers yet">Add an address, or join once in-game and it will show up after you reopen this tab.</EmptyState>}
      {servers.length > 0 && (
        <div className="card content-list">
          {servers.map((s, i) => (
            <div key={`${s.ip}-${i}`} className="content-row">
              <div className="content-icon" style={{ display: "grid", placeItems: "center" }} aria-hidden="true"><Icon name="globe" /></div>
              <div className="content-main">
                <div className="content-title">{s.name || s.ip}</div>
                <div className="content-sub mono">{s.ip}</div>
              </div>
              <div className="row" style={{ flexWrap: "nowrap", gap: 4 }}>
                <button className="btn btn-sm btn-play" onClick={() => play(inst.id, { server: s.ip })} title="Quick Play (Minecraft 1.20+)"><Icon name="play" size={14} />Play</button>
                <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Remove ${s.name}`} onClick={() => persist(servers.filter((_, j) => j !== i))}><Icon name="trash" size={16} /></button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function WorldsTab({ inst }: { inst: InstanceView }) {
  const { play, toast } = useStore();
  const [worlds, setWorlds] = useState<World[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => { api.listWorlds(inst.id).then(setWorlds).catch((e) => setError(String(e))); }, [inst.id]);
  useEffect(load, [load]);
  if (error) return <ErrorState title="Worlds could not be listed" error={error} onRetry={load} />;
  if (!worlds) return <Loading label="Reading worlds" />;
  if (!worlds.length) return <EmptyState icon="globe" title="No worlds yet">Worlds you create in this instance show up here, and you can jump straight into one.</EmptyState>;
  return (
    <div className="worlds">
      {worlds.map((w) => (
        <div key={w.folder} className="card world">
          {w.icon ? <img src={fileSrc(w.icon)} alt="" /> : <div className="ph" aria-hidden="true" />}
          <div className="world-body">
            <div style={{ flex: 1, minWidth: 0 }}>
              <div className="world-name" title={w.folder}>{w.folder}</div>
              <div className="small muted">{relativeTime(w.lastPlayed)}, {bytes(w.size)}</div>
            </div>
            <button className="btn btn-icon btn-sm" aria-label={`Open ${w.folder} folder`} onClick={() => api.openDataPath(`${inst.path}/saves/${w.folder}`).catch((e) => toast(String(e), true))}><Icon name="folder" size={14} /></button>
            <button className="btn btn-sm" onClick={() => play(inst.id, { world: w.folder })} title="Starts the game straight into this world (Minecraft 1.20 and newer)"><Icon name="play" size={14} />Play</button>
          </div>
        </div>
      ))}
    </div>
  );
}

function ScreenshotsTab({ inst }: { inst: InstanceView }) {
  const [shots, setShots] = useState<FileEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<FileEntry | null>(null);
  const load = useCallback(() => { api.listScreenshots(inst.id).then(setShots).catch((e) => setError(String(e))); }, [inst.id]);
  useEffect(load, [load]);
  useEffect(() => {
    if (!view) return;
    const k = (e: KeyboardEvent) => e.key === "Escape" && setView(null);
    document.addEventListener("keydown", k);
    return () => document.removeEventListener("keydown", k);
  }, [view]);
  if (error) return <ErrorState title="Screenshots could not be listed" error={error} onRetry={load} />;
  if (!shots) return <Loading label="Reading screenshots" />;
  if (!shots.length) return <EmptyState icon="image" title="No screenshots yet">Press F2 in game. Screenshots from this instance appear here.</EmptyState>;
  return (
    <>
      <div className="shots">
        {shots.map((s) => (
          <button key={s.path} className="shot" onClick={() => setView(s)} aria-label={`View ${s.name}`}>
            <img src={fileSrc(s.path)} alt="" loading="lazy" />
          </button>
        ))}
      </div>
      {view && (
        <div className="lightbox" role="dialog" aria-modal="true" aria-label={view.name} onClick={() => setView(null)}>
          <img src={fileSrc(view.path)} alt={view.name} />
          <button className="btn" style={{ position: "fixed", top: 16, right: 16 }} onClick={() => setView(null)} autoFocus>Close</button>
        </div>
      )}
    </>
  );
}

function classify(line: string, stream: string) {
  if (line.startsWith("[Cairn]")) return "launcher";
  if (/\/(ERROR|FATAL)\]|Exception|^\s+at /.test(line) || stream === "stderr" && /error/i.test(line)) return "error";
  if (/\/WARN\]/.test(line)) return "warn";
  return "";
}

function LogsTab({ inst }: { inst: InstanceView }) {
  const { games, toast } = useStore();
  const [source, setSource] = useState<string>("live");
  const [lines, setLines] = useState<{ t: string; s: string }[]>([]);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [fileText, setFileText] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.gameLog(inst.id).then((l) => setLines(l.map((t) => ({ t, s: "stdout" }))));
    api.listLogFiles(inst.id).then(setFiles).catch(() => setFiles([]));
    const un = on<LogLine>("game-log", (l) => {
      if (l.instanceId === inst.id) setLines((prev) => (prev.length > 8000 ? [...prev.slice(-6000), { t: l.line, s: l.stream }] : [...prev, { t: l.line, s: l.stream }]));
    });
    return () => { un.then((u) => u()); };
  }, [inst.id]);

  useEffect(() => {
    if (source === "live") { setFileText(null); return; }
    api.readInstanceFile(inst.id, source).then(setFileText).catch((e) => setFileText(String(e)));
  }, [source, inst.id]);

  const shown = useMemo(() => {
    const base = source === "live" ? lines : (fileText ?? "").split("\n").map((t) => ({ t, s: "stdout" }));
    const f = filter.trim().toLowerCase();
    return f ? base.filter((l) => l.t.toLowerCase().includes(f)) : base;
  }, [lines, fileText, source, filter]);

  useEffect(() => { if (follow && box.current) box.current.scrollTop = box.current.scrollHeight; }, [shown, follow]);

  const g = games[inst.id];
  return (
    <div className="stack">
      <div className="row" style={{ gap: 12 }}>
        <div className="field" style={{ flex: "0 1 320px" }}>
          <span className="sr-only">Log source</span>
          <Select
            aria-label="Log source"
            value={source}
            onChange={setSource}
            options={[
              { value: "live", label: `Current session${g && g.state !== "exited" ? " (running)" : ""}` },
              ...files.map((f) => ({ value: f.path, label: f.name })),
            ]}
          />
        </div>
        <label className="search" style={{ maxWidth: 320 }}>
          <span className="sr-only">Filter log lines</span>
          <Icon name="search" size={16} />
          <input className="input" placeholder="Filter lines" value={filter} onChange={(e) => setFilter(e.target.value)} />
        </label>
        <label className="check"><input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />Follow output</label>
        <span className="spacer" />
        <button className="btn" onClick={() => navigator.clipboard.writeText(shown.map((l) => l.t).join("\n")).then(() => toast("Copied log to clipboard"))}><Icon name="copy" />Copy</button>
        {source !== "live" && (
          <button className="btn" onClick={() => {
            const folder = source.includes("crash-reports") ? `${inst.path}/crash-reports` : source.includes("/logs/") ? `${inst.path}/logs` : `${inst.path}/.cairn/logs`;
            api.openDataPath(folder).catch((e) => toast(String(e), true));
          }}><Icon name="folder" />Open folder</button>
        )}
      </div>
      {files.some((f) => f.name.startsWith("Crash report:")) && (
        <div className="notice" role="status">
          <Icon name="warn" />
          <div style={{ flex: 1 }}>
            <b>Crash reports on disk</b>
            <p className="small muted">Pick a “Crash report” in the source list, then copy it or open the folder.</p>
          </div>
          <button className="btn btn-sm" onClick={() => {
            const crash = files.find((f) => f.name.startsWith("Crash report:"));
            if (crash) setSource(crash.path);
          }}>Show latest</button>
          <button className="btn btn-sm" onClick={() => api.openDataPath(`${inst.path}/crash-reports`).catch((e) => toast(String(e), true))}><Icon name="folder" size={14} />Open crashes</button>
        </div>
      )}
      <div className="log-view" ref={box} tabIndex={0} aria-label="Game log" role="log">
        {shown.length === 0 ? (
          <span className="muted">{source === "live" ? "Nothing logged yet. Press Play and the game output streams here." : fileText === null ? "Reading file..." : "This file is empty."}</span>
        ) : shown.map((l, i) => <div key={i} className={`log-line ${classify(l.t, l.s)}`}>{l.t}</div>)}
      </div>
    </div>
  );
}

function OptionsTab({ inst, onSaved }: { inst: InstanceView; onSaved: (i: InstanceView) => void }) {
  const { toast, settings } = useStore();
  const [form, setForm] = useState(inst);
  const [versions, setVersions] = useState<McVersion[]>([]);
  const [lvs, setLvs] = useState<LoaderVersion[] | null>(null);
  const [saving, setSaving] = useState(false);
  useEffect(() => { api.mcVersions().then((v) => setVersions(v.filter((x) => x.type === "release" || x.id === inst.mcVersion))).catch(() => {}); }, [inst.mcVersion]);
  useEffect(() => {
    if (form.loader === "vanilla") { setLvs([]); return; }
    setLvs(null);
    api.loaderVersions(form.loader, form.mcVersion).then(setLvs).catch(() => setLvs([]));
  }, [form.loader, form.mcVersion]);
  const set = <K extends keyof InstanceView>(k: K, v: InstanceView[K]) => setForm((f) => ({ ...f, [k]: v }));
  const dirty = JSON.stringify(form) !== JSON.stringify(inst);
  const save = async () => {
    setSaving(true);
    try { const i = await api.updateInstance(form); onSaved(i); setForm(i); toast("Saved instance options"); } catch (e) { toast(String(e), true); } finally { setSaving(false); }
  };
  return (
    <div className="settings-grid">
      <section className="panel settings-section">
        <header><h2>General</h2></header>
        <label className="field"><span>Name</span><input className="input" value={form.name} onChange={(e) => set("name", e.target.value)} /></label>
        <div className="field">
          <span>Minecraft version</span>
          <Select
            aria-label="Minecraft version"
            value={form.mcVersion}
            onChange={(v) => {
              setForm((f) => ({
                ...f,
                mcVersion: v,
                loaderVersion: null,
                useHorizon: f.loader === "vanilla" && v === "26.3" ? f.useHorizon : false,
              }));
            }}
            options={[
              ...(!versions.some((v) => v.id === form.mcVersion) ? [{ value: form.mcVersion, label: form.mcVersion }] : []),
              ...versions.map((v) => ({ value: v.id, label: v.id })),
            ]}
          />
          <small>Changing versions can break worlds and mods. Back up first.</small>
        </div>
        <div className="field">
          <span>Loader</span>
          <Select
            aria-label="Loader"
            value={form.loader}
            onChange={(v) => {
              const loader = v as InstanceView["loader"];
              setForm((f) => ({
                ...f,
                loader,
                loaderVersion: null,
                useHorizon: loader === "vanilla" && f.mcVersion === "26.3" ? f.useHorizon : false,
              }));
            }}
            options={Object.entries(loaderLabel).map(([k, v]) => ({ value: k, label: v }))}
          />
        </div>
        {form.loader !== "vanilla" && (
          <div className="field">
            <span>{loaderLabel[form.loader]} version</span>
            <Select
              aria-label={`${loaderLabel[form.loader]} version`}
              value={form.loaderVersion ?? ""}
              onChange={(v) => set("loaderVersion", v || null)}
              options={[
                { value: "", label: "Latest stable" },
                ...(lvs ?? []).slice(0, 60).map((v) => ({ value: v.version, label: `${v.version}${v.stable ? "" : " (beta)"}` })),
              ]}
            />
            {lvs && lvs.length === 0 && <small className="field-error">No {loaderLabel[form.loader]} build for {form.mcVersion}.</small>}
          </div>
        )}
        {(() => {
          const horizonOk = form.loader === "vanilla" && form.mcVersion === "26.3";
          return (
            <div className="field">
              <Switch
                checked={!!form.useHorizon && horizonOk}
                onChange={(v) => set("useHorizon", v)}
                label="Play with Horizon"
                disabled={!horizonOk}
              />
              <small>
                {horizonOk
                  ? "Rust client instead of Java. Worlds stay here. Online-mode servers may not work yet."
                  : "Horizon needs vanilla Minecraft 26.3."}
              </small>
            </div>
          );
        })()}
      </section>
      <section className="panel settings-section">
        <header><h2>Java and memory</h2><p className="small muted">Leave Java empty and Cairn picks or downloads the right version.</p></header>
        <label className="field">
          <span>Memory: {((form.memoryMb ?? settings?.defaultMemoryMb ?? 4096) / 1024).toFixed(1)} GB</span>
          <input type="range" min={1024} max={16384} step={512} value={form.memoryMb ?? settings?.defaultMemoryMb ?? 4096} onChange={(e) => set("memoryMb", Number(e.target.value))} style={{ accentColor: "var(--moss)", minHeight: 32 }} />
        </label>
        <label className="field"><span>Java executable</span><input className="input mono" placeholder="Automatic" value={form.javaPath ?? ""} onChange={(e) => set("javaPath", e.target.value)} /></label>
        <label className="field"><span>Extra JVM arguments</span><input className="input mono" placeholder="-XX:+UseG1GC" value={form.jvmArgs} onChange={(e) => set("jvmArgs", e.target.value)} /></label>
        <div className="row">
          <label className="field" style={{ flex: 1 }}><span>Window width</span><input className="input" type="number" min={320} placeholder={String(settings?.defaultWidth ?? 1280)} value={form.width ?? ""} onChange={(e) => set("width", e.target.value ? Number(e.target.value) : null)} /></label>
          <label className="field" style={{ flex: 1 }}><span>Window height</span><input className="input" type="number" min={240} placeholder={String(settings?.defaultHeight ?? 720)} value={form.height ?? ""} onChange={(e) => set("height", e.target.value ? Number(e.target.value) : null)} /></label>
        </div>
      </section>
      <section className="panel settings-section" style={{ gridColumn: "1 / -1" }}>
        <header>
          <h2>Launch profiles</h2>
          <p className="small muted">Named JVM setups. “Shaders off” hides the shaderpacks folder for that run.</p>
        </header>
        <div className="field" style={{ maxWidth: 360 }}>
          <span>Active profile</span>
          <Select
            aria-label="Active launch profile"
            value={form.activeLaunchProfile ?? ""}
            onChange={(v) => set("activeLaunchProfile", v || null)}
            options={[
              { value: "", label: "Default (instance settings)" },
              ...(form.launchProfiles ?? []).map((p) => ({ value: p.id, label: p.name })),
            ]}
          />
        </div>
        <div className="stack" style={{ gap: 10 }}>
          {(form.launchProfiles ?? []).map((p, i) => (
            <div key={p.id} className="panel" style={{ padding: 12 }}>
              <div className="row" style={{ gap: 8, alignItems: "flex-end" }}>
                <label className="field" style={{ flex: 1 }}><span>Name</span><input className="input" value={p.name} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, name: e.target.value };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <label className="field" style={{ flex: 1 }}><span>JVM args</span><input className="input mono" value={p.jvmArgs} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, jvmArgs: e.target.value };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <label className="field" style={{ width: 140 }}><span>Memory MB</span><input className="input" type="number" min={1024} placeholder="Default" value={p.memoryMb ?? ""} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, memoryMb: e.target.value ? Number(e.target.value) : null };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <button className="btn btn-quiet btn-icon" aria-label={`Remove ${p.name}`} onClick={() => {
                  const launchProfiles = (form.launchProfiles ?? []).filter((x) => x.id !== p.id);
                  set("launchProfiles", launchProfiles);
                  if (form.activeLaunchProfile === p.id) set("activeLaunchProfile", null);
                }}><Icon name="trash" /></button>
              </div>
              <Switch checked={p.disableShaders} onChange={(v) => {
                const launchProfiles = [...(form.launchProfiles ?? [])];
                launchProfiles[i] = { ...p, disableShaders: v };
                set("launchProfiles", launchProfiles);
              }} label="Shaders off for this profile" />
            </div>
          ))}
        </div>
        <div className="row">
          <button className="btn btn-sm" onClick={() => {
            const id = crypto.randomUUID();
            const next: LaunchProfile = { id, name: "Shaders off", jvmArgs: "", memoryMb: null, javaPath: null, disableShaders: true };
            set("launchProfiles", [...(form.launchProfiles ?? []), next]);
            set("activeLaunchProfile", id);
          }}><Icon name="plus" size={14} />Add “Shaders off”</button>
          <button className="btn btn-sm" onClick={() => {
            const id = crypto.randomUUID();
            set("launchProfiles", [...(form.launchProfiles ?? []), { id, name: "Custom JVM", jvmArgs: "", memoryMb: null, javaPath: null, disableShaders: false }]);
          }}><Icon name="plus" size={14} />Add profile</button>
        </div>
      </section>
      <div className="row" style={{ gridColumn: "1 / -1", justifyContent: "flex-end" }}>
        <button className="btn btn-quiet" disabled={!dirty} onClick={() => setForm(inst)}>Discard changes</button>
        <button className="btn" disabled={!dirty || saving} onClick={save}><Icon name="check" />{saving ? "Saving..." : "Save options"}</button>
      </div>
    </div>
  );
}
