import { useCallback, useEffect, useState } from "react";
import { api, type InstanceView, type ServerEntry } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { EmptyState, ErrorState, Loading } from "../../components/ui";

export function ServersTab({ inst }: { inst: InstanceView }) {
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
