import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { InstanceCard } from "../components/InstanceCard";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, Select } from "../components/ui";
import { loaderLabel } from "../format";
import { ImportFromApp } from "./ImportFromApp";
import { PasteShareCode } from "./PasteShareCode";

type Sort = "recent" | "name" | "created";

export function Library() {
  const { instances, instancesError, refreshInstances, go, setShowCreate, toast } = useStore();
  const [q, setQ] = useState("");
  const [loader, setLoader] = useState<string>("all");
  const [sort, setSort] = useState<Sort>("recent");
  const [importing, setImporting] = useState(false);
  const [importApp, setImportApp] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [pasting, setPasting] = useState(false);

  const list = useMemo(() => {
    let l = [...(instances ?? [])];
    if (q.trim()) l = l.filter((i) => i.name.toLowerCase().includes(q.trim().toLowerCase()) || i.mcVersion.includes(q.trim()));
    if (loader !== "all") l = l.filter((i) => i.loader === loader);
    l.sort((a, b) =>
      sort === "name" ? a.name.localeCompare(b.name) : sort === "created" ? b.created - a.created : (b.lastPlayed ?? b.created) - (a.lastPlayed ?? a.created),
    );
    return l;
  }, [instances, q, loader, sort]);

  const loadersPresent = Array.from(new Set((instances ?? []).map((i) => i.loader)));

  const importPack = async () => {
    const file = await open({
      multiple: false,
      filters: [
        { name: "Modpack", extensions: ["mrpack", "zip"] },
        { name: "Modrinth modpack", extensions: ["mrpack"] },
        { name: "CurseForge modpack", extensions: ["zip"] },
      ],
    });
    if (!file) return;
    setImporting(true);
    try {
      const inst = await api.importModpack(file as string);
      await refreshInstances();
      toast(`Imported ${inst.name}`);
      go({ name: "instance", id: inst.id });
    } catch (e) {
      toast(String(e), true);
    } finally {
      setImporting(false);
    }
  };

  const restoreBackup = async () => {
    const file = await open({ multiple: false, filters: [{ name: "Cairn backup", extensions: ["zip"] }] });
    if (!file) return;
    setRestoring(true);
    try {
      const inst = await api.restoreInstanceBackup(file as string);
      await refreshInstances();
      toast(`Restored ${inst.name}`);
      go({ name: "instance", id: inst.id });
    } catch (e) {
      toast(String(e), true);
    } finally {
      setRestoring(false);
    }
  };

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Library</h1>
          <p className="muted">{instances ? `${instances.length} ${instances.length === 1 ? "instance" : "instances"}` : "Reading instances"}</p>
        </div>
        <div className="row">
          <button className="btn" onClick={() => setImportApp(true)}><Icon name="download" />Import from app</button>
          <button className="btn" onClick={restoreBackup} disabled={restoring}><Icon name="upload" />{restoring ? "Restoring..." : "Restore backup"}</button>
          <button className="btn" onClick={() => setPasting(true)}><Icon name="copy" />Paste share code</button>
          <button className="btn" onClick={importPack} disabled={importing}><Icon name="upload" />{importing ? "Importing..." : "Import pack"}</button>
          <button className="btn btn-play" style={{ minHeight: 40, fontSize: 15 }} onClick={() => setShowCreate(true)}><Icon name="plus" />New instance</button>
        </div>
      </div>

      {importApp && <ImportFromApp onClose={() => setImportApp(false)} />}
      {pasting && <PasteShareCode onClose={() => setPasting(false)} />}

      {instances && instances.length > 0 && (
        <div className="row" style={{ gap: 12 }}>
          <label className="search">
            <span className="sr-only">Filter instances</span>
            <Icon name="search" size={16} />
            <input className="input" placeholder="Filter by name or version" value={q} onChange={(e) => setQ(e.target.value)} />
          </label>
          {loadersPresent.length > 1 && (
            <div className="segmented" role="group" aria-label="Loader">
              {["all", ...loadersPresent].map((l) => (
                <button key={l} aria-pressed={loader === l} onClick={() => setLoader(l)}>{l === "all" ? "All" : loaderLabel[l]}</button>
              ))}
            </div>
          )}
          <div className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8, flex: "0 0 auto" }}>
            <span className="small muted" style={{ fontWeight: 700 }}>Sort</span>
            <Select
              style={{ width: 170 }}
              aria-label="Sort instances"
              value={sort}
              onChange={(v) => setSort(v as Sort)}
              options={[
                { value: "recent", label: "Last played" },
                { value: "name", label: "Name" },
                { value: "created", label: "Newest first" },
              ]}
            />
          </div>
        </div>
      )}

      {instancesError && <ErrorState title="Your instances could not be read" error={instancesError} onRetry={refreshInstances} />}
      {instances === null && !instancesError && (
        <div className="grid" aria-label="Loading instances">{[0, 1, 2, 3].map((i) => <div key={i} className="skeleton" style={{ minHeight: 130 }} />)}</div>
      )}
      {instances && instances.length === 0 && (
        <EmptyState icon="library" title="No instances yet" action={
          <div className="row" style={{ justifyContent: "center" }}>
            <button className="btn" onClick={() => setImportApp(true)}>Import from app</button>
            <button className="btn" onClick={() => go({ name: "discover", projectType: "modpack" })}>Browse modpacks</button>
            <button className="btn btn-play" style={{ minHeight: 40, fontSize: 15 }} onClick={() => setShowCreate(true)}>Create an instance</button>
          </div>
        }>
          Create one from a Minecraft version and loader, install a modpack from Discover, import an .mrpack, or pull an instance from Modrinth App, RexLauncher or Prism Launcher.
        </EmptyState>
      )}
      {instances && instances.length > 0 && list.length === 0 && (
        <EmptyState icon="search" title="Nothing matches that filter" action={<button className="btn" onClick={() => { setQ(""); setLoader("all"); }}>Clear filters</button>} />
      )}
      {list.length > 0 && (
        <div className="grid">
          {list.map((i) => <InstanceCard key={i.id} inst={i} />)}
        </div>
      )}
    </>
  );
}
