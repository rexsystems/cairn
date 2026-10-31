import { useCallback, useEffect, useState } from "react";
import { api, fileSrc, type InstanceView, type World } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { EmptyState, ErrorState, Loading } from "../../components/ui";
import { bytes, relativeTime } from "../../format";

export function WorldsTab({ inst }: { inst: InstanceView }) {
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
