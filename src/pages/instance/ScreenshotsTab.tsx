import { useCallback, useEffect, useState } from "react";
import { api, fileSrc, type FileEntry, type InstanceView } from "../../api";
import { EmptyState, ErrorState, Loading } from "../../components/ui";

export function ScreenshotsTab({ inst }: { inst: InstanceView }) {
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
