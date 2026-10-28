import { useEffect, useMemo, useState } from "react";
import { api, type Loader, type LoaderVersion, type McVersion } from "../api";
import { useStore } from "../store";
import { Modal, Select, Switch } from "../components/ui";
import { loaderLabel } from "../format";

const LOADERS: { id: Loader; note: string }[] = [
  { id: "vanilla", note: "No mods" },
  { id: "fabric", note: "Lightweight" },
  { id: "quilt", note: "Runs Fabric mods" },
  { id: "forge", note: "Classic packs" },
  { id: "neoforge", note: "Modern Forge" },
];

export function CreateInstance({ onClose }: { onClose: () => void }) {
  const { refreshInstances, go, toast, settings } = useStore();
  const [versions, setVersions] = useState<McVersion[] | null>(null);
  const [versionsError, setVersionsError] = useState<string | null>(null);
  const [snapshots, setSnapshots] = useState(false);
  const [mc, setMc] = useState("");
  const [loader, setLoader] = useState<Loader>("fabric");
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersion[] | null>(null);
  const [loaderError, setLoaderError] = useState<string | null>(null);
  const [loaderVersion, setLoaderVersion] = useState("");
  const [name, setName] = useState("");
  const [memory, setMemory] = useState(settings?.defaultMemoryMb ?? 4096);
  const [useHorizon, setUseHorizon] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const horizonOk = loader === "vanilla" && mc === "26.3";
  useEffect(() => {
    if (!horizonOk) setUseHorizon(false);
  }, [horizonOk]);

  useEffect(() => {
    api.mcVersions().then((v) => {
      setVersions(v);
      setMc(v.find((x) => x.type === "release")?.id ?? v[0]?.id ?? "");
    }).catch((e) => setVersionsError(String(e)));
  }, []);

  const shown = useMemo(() => (versions ?? []).filter((v) => (snapshots ? v.type === "release" || v.type === "snapshot" : v.type === "release")), [versions, snapshots]);

  useEffect(() => {
    if (!mc || loader === "vanilla") { setLoaderVersions(null); setLoaderError(null); return; }
    let alive = true;
    setLoaderVersions(null);
    setLoaderError(null);
    setLoaderVersion("");
    api.loaderVersions(loader, mc).then((v) => {
      if (!alive) return;
      setLoaderVersions(v);
      setLoaderVersion((v.find((x) => x.stable) ?? v[0])?.version ?? "");
    }).catch((e) => alive && setLoaderError(String(e)));
    return () => { alive = false; };
  }, [mc, loader]);

  const noLoaderBuild = loader !== "vanilla" && loaderVersions !== null && loaderVersions.length === 0;
  const placeholderName = mc ? `${loaderLabel[loader]} ${mc}` : "My instance";

  const submit = async () => {
    setError(null);
    if (!mc) return setError("Pick a Minecraft version.");
    if (noLoaderBuild) return setError(`${loaderLabel[loader]} has no build for Minecraft ${mc}. Pick another version or loader.`);
    setSaving(true);
    try {
      const inst = await api.createInstance({
        name: name.trim() || placeholderName,
        mcVersion: mc,
        loader,
        loaderVersion: loader === "vanilla" ? null : loaderVersion || null,
        memoryMb: memory,
        useHorizon: horizonOk && useHorizon,
      });
      await refreshInstances();
      onClose();
      go({ name: "instance", id: inst.id });
      toast(`Created ${inst.name}. Downloading game files in the background.`);
      api.installInstance(inst.id).then(refreshInstances).catch((e) => toast(String(e), true));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal title="New instance" onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
        <button className="btn btn-play" style={{ minHeight: 42, fontSize: 15 }} onClick={submit} disabled={saving || !mc}>{saving ? "Creating..." : "Create instance"}</button>
      </>
    }>
      <label className="field">
        <span>Name</span>
        <input className="input" data-autofocus value={name} placeholder={placeholderName} maxLength={60} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && submit()} />
      </label>

      <div className="field" role="radiogroup" aria-label="Mod loader">
        <span>Mod loader</span>
        <div className="loader-pick">
          {LOADERS.map((l) => (
            <button key={l.id} type="button" role="radio" aria-checked={loader === l.id} className="loader-opt" onClick={() => setLoader(l.id)}>
              <span>{loaderLabel[l.id]}</span>
              <span className="small muted" style={{ fontWeight: 400 }}>{l.note}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="row" style={{ alignItems: "flex-end", gap: 16 }}>
        <div className="field" style={{ flex: "1 1 200px" }}>
          <span>Minecraft version</span>
          {versionsError ? <span className="field-error">Could not load versions: {versionsError}</span> : (
            <Select
              aria-label="Minecraft version"
              value={mc}
              onChange={setMc}
              disabled={!versions}
              placeholder="Loading versions..."
              options={shown.map((v) => ({ value: v.id, label: `${v.id}${v.type === "snapshot" ? " (snapshot)" : ""}` }))}
            />
          )}
        </div>
        <Switch checked={snapshots} onChange={setSnapshots} label="Show snapshots" />
      </div>

      {loader !== "vanilla" && (
        <div className="field">
          <span>{loaderLabel[loader]} version</span>
          {loaderError && <span className="field-error">Could not load {loaderLabel[loader]} versions: {loaderError}</span>}
          {!loaderError && (
            <Select
              aria-label={`${loaderLabel[loader]} version`}
              value={loaderVersion}
              onChange={setLoaderVersion}
              disabled={!loaderVersions || noLoaderBuild}
              placeholder={!loaderVersions ? `Loading ${loaderLabel[loader]} versions...` : noLoaderBuild ? `No build for ${mc}` : "Choose version"}
              options={(loaderVersions ?? []).slice(0, 80).map((v) => ({ value: v.version, label: `${v.version}${v.stable ? "" : " (beta)"}` }))}
            />
          )}
          {noLoaderBuild && <small>{loaderLabel[loader]} does not support Minecraft {mc} yet.</small>}
        </div>
      )}

      <label className="field">
        <span>Memory: {(memory / 1024).toFixed(1)} GB</span>
        <input type="range" min={1024} max={16384} step={512} value={memory} onChange={(e) => setMemory(Number(e.target.value))} style={{ accentColor: "var(--moss)", minHeight: 32 }} />
        <small>Most modpacks run well with 4 to 6 GB. You can change this later.</small>
      </label>

      {horizonOk && (
        <div className="field">
          <Switch checked={useHorizon} onChange={setUseHorizon} label="Play with Horizon" />
          <small>Uses the Rust client instead of Java. Worlds stay in this instance. Online-mode servers may not work yet.</small>
        </div>
      )}
      {error && <p className="field-error" role="alert">{error}</p>}
    </Modal>
  );
}
