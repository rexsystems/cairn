import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type BundledHorizonMod, type HorizonModItem, type InstanceView } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { EmptyState, ErrorState, Loading, Switch } from "../../components/ui";
import { bytes } from "../../format";

export function HorizonModsTab({ inst }: { inst: InstanceView }) {
  const { toast } = useStore();
  const [items, setItems] = useState<HorizonModItem[] | null>(null);
  const [bundled, setBundled] = useState<BundledHorizonMod[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(() => {
    setError(null);
    Promise.all([api.listHorizonMods(inst.id), api.listBundledHorizonMods()])
      .then(([list, samples]) => { setItems(list); setBundled(samples); })
      .catch((e) => setError(String(e)));
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
  const installSample = async (sampleId: string) => {
    setBusyId(sampleId);
    try {
      const m = await api.installBundledHorizonMod(inst.id, sampleId);
      toast(`Installed ${m.name}`);
      load();
    } catch (e) { toast(String(e), true); } finally { setBusyId(null); }
  };

  if (error) return <ErrorState title="Horizon mods could not be listed" error={error} onRetry={load} />;
  if (!items) return <Loading label="Reading Horizon mods" />;

  const installedIds = new Set(items.map((m) => m.id));
  const available = bundled.filter((b) => !installedIds.has(b.id));

  return (
    <div className="stack" style={{ gap: 16 }}>
      <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
        <button className="btn" onClick={addFiles}><Icon name="plus" />Add .hmod</button>
        <button className="btn btn-quiet btn-icon" aria-label="Open horizon-mods folder" title="Open folder" onClick={() => api.openDataPath(`${inst.path}/horizon-mods`).catch((e) => toast(String(e), true))}><Icon name="folder" /></button>
      </div>
      <p className="small muted">
        Horizon runs `.hmod` (Rust → WASM), not Fabric/Forge jars. Rebind mod keys in-game under Options → Mods (not Controls).
        Voice chat and full world maps need APIs Horizon does not expose yet.
      </p>
      {available.length > 0 && (
        <div className="stack" style={{ gap: 8 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Cairn samples</h3>
          <div className="card content-list">
            {available.map((b) => (
              <div key={b.id} className="content-row">
                <div className="content-icon" style={{ display: "grid", placeItems: "center" }} aria-hidden="true"><Icon name="download" /></div>
                <div className="content-main">
                  <div className="content-title">{b.name}</div>
                  <div className="content-sub">{b.description}</div>
                </div>
                <button className="btn btn-sm" disabled={!!busyId} onClick={() => installSample(b.id)}>
                  {busyId === b.id ? "…" : "Install"}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
      {items.length === 0 ? (
        <EmptyState icon="box" title="No Horizon mods yet">
          Install a Cairn sample above, or add a `.hmod` from playhorizon.dev.
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
