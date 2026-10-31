import { useCallback, useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, type InstanceView } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { ErrorState, InstanceIcon, Loading, Modal, PopoverMenu } from "../components/ui";
import { PlayButton } from "../components/PlayButton";
import { InstanceCover } from "../components/InstanceCover";
import { loaderLabel, playTime, relativeTime } from "../format";
import { HorizonModsTab } from "./instance/HorizonModsTab";
import { ExportDialog } from "./instance/ExportDialog";
import { ShareDialog } from "./instance/ShareDialog";
import { ContentTab } from "./instance/ContentTab";
import { ServersTab } from "./instance/ServersTab";
import { WorldsTab } from "./instance/WorldsTab";
import { ScreenshotsTab } from "./instance/ScreenshotsTab";
import { LogsTab } from "./instance/LogsTab";
import { ActivityTab } from "./instance/ActivityTab";
import { OptionsTab } from "./instance/OptionsTab";

const BASE_TABS = [
  { id: "content", label: "Content" },
  { id: "worlds", label: "Worlds" },
  { id: "servers", label: "Servers" },
  { id: "screenshots", label: "Screenshots" },
  { id: "logs", label: "Logs" },
  { id: "activity", label: "Activity" },
  { id: "options", label: "Options" },
];

export function InstancePage({ id, initialTab }: { id: string; initialTab?: string }) {
  const { refreshInstances, go, toast, games } = useStore();
  const [inst, setInst] = useState<InstanceView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState(initialTab ?? "content");
  const [tabSeeded, setTabSeeded] = useState(!!initialTab);
  const [menu, setMenu] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [sharing, setSharing] = useState(false);
  const menuBtnRef = useRef<HTMLButtonElement>(null);

  const load = useCallback(() => {
    api.getInstance(id).then((i) => { setInst(i); setError(null); }).catch((e) => setError(String(e)));
  }, [id]);
  useEffect(load, [load, games[id]?.state]);
  useEffect(() => {
    if (!inst || tabSeeded) return;
    if (inst.useHorizon) setTab("horizon-mods");
    setTabSeeded(true);
  }, [inst, tabSeeded]);
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
            <button role="menuitem" className="menu-item" onClick={() => { setMenu(false); setSharing(true); }}><Icon name="copy" />Share</button>
            <button role="menuitem" className="menu-item" onClick={() => { setMenu(false); setExporting(true); }}><Icon name="download" />Export as .mrpack</button>
            <button role="menuitem" className="menu-item" onClick={async () => {
              setMenu(false);
              const dest = await save({ defaultPath: `${inst.name}-backup.zip`, filters: [{ name: "Cairn backup", extensions: ["zip"] }] });
              if (!dest) return;
              try { await api.backupInstance(inst.id, dest as string); toast("Backup saved"); } catch (e) { toast(String(e), true); }
            }}><Icon name="download" />Backup instance</button>
            <button role="menuitem" className="menu-item" onClick={async () => { setMenu(false); try { await api.installInstance(inst.id); toast("Game files are up to date"); } catch (e) { toast(String(e), true); } }}><Icon name="update" />Repair game files</button>
            <button role="menuitem" className="menu-item" onClick={async () => { setMenu(false); try { toast(await api.createShortcut(inst.id)); } catch (e) { toast(String(e), true); } }}><Icon name="external" />Create desktop shortcut</button>
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
          {tab === "activity" && <ActivityTab inst={inst} />}
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
      {sharing && <ShareDialog inst={inst} onExport={() => { setSharing(false); setExporting(true); }} onClose={() => setSharing(false)} />}
      {exporting && <ExportDialog inst={inst} onClose={() => setExporting(false)} />}
    </>
  );
}
