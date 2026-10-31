import { useEffect, useState } from "react";
import { api, type ModrinthAppProfile } from "../api";
import { useStore } from "../store";
import { EmptyState, ErrorState, InstanceIcon, Loading, Modal } from "../components/ui";
import { loaderLabel, relativeTime } from "../format";

export function ImportFromApp({ onClose }: { onClose: () => void }) {
  const { refreshInstances, go, toast } = useStore();
  const [profiles, setProfiles] = useState<ModrinthAppProfile[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);

  const load = () => {
    setError(null);
    setProfiles(null);
    api.listModrinthAppProfiles()
      .then((list) => {
        setProfiles(list);
        setSelected(list[0]?.key ?? null);
      })
      .catch((e) => setError(String(e)));
  };

  useEffect(load, []);

  const submit = async (key = selected) => {
    if (!key || importing) return;
    setSelected(key);
    setImporting(true);
    try {
      const inst = await api.importModrinthAppProfile(key);
      await refreshInstances();
      onClose();
      toast(`Imported ${inst.name}`);
      go({ name: "instance", id: inst.id });
    } catch (e) {
      toast(String(e), true);
    } finally {
      setImporting(false);
    }
  };

  const pick = profiles?.find((p) => p.key === selected);

  return (
    <Modal
      wide
      title="Import from another launcher"
      onClose={onClose}
      footer={
        <>
          <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
          <button
            className="btn btn-play"
            style={{ minHeight: 42, fontSize: 15 }}
            onClick={() => void submit()}
            disabled={!pick || importing}
          >
            {importing ? "Importing..." : pick ? `Import ${pick.name}` : "Import"}
          </button>
        </>
      }
    >
      <p className="muted" style={{ marginTop: 0 }}>
        Copies an instance from Modrinth App, RexLauncher, Prism Launcher, PolyMC or MultiMC on this computer. Your original stays where it is.
      </p>

      {error && <ErrorState title="Could not read those launchers" error={error} onRetry={load} />}
      {profiles === null && !error && <Loading label="Looking for instances" />}
      {profiles && profiles.length === 0 && (
        <EmptyState icon="library" title="Nothing to import">
          Install one of those launchers first, or point it at a folder that has instances.
        </EmptyState>
      )}
      {profiles && profiles.length > 0 && (
        <div className="card import-list" role="listbox" aria-label="Instances to import">
          {profiles.map((p) => {
            const active = p.key === selected;
            return (
              <button
                key={p.key}
                type="button"
                role="option"
                aria-selected={active}
                className={`import-row${active ? " is-selected" : ""}`}
                onClick={() => setSelected(p.key)}
                onDoubleClick={() => void submit(p.key)}
              >
                <InstanceIcon name={p.name} />
                <span className="import-row-main">
                  <span className="import-row-name">{p.name}</span>
                  <span className="small muted">
                    {loaderLabel[p.loader] ?? p.loader} {p.mcVersion}
                    {" · "}
                    {p.sourceApp}
                    {" · "}
                    {p.lastPlayed ? relativeTime(p.lastPlayed) : "never played"}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      )}
    </Modal>
  );
}
