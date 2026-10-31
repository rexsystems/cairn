import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, on, type PlayitStatus, type PlayitTunnel } from "../../api";
import { Icon } from "../../components/Icon";
import { useStore } from "../../store";

const TUNNELS_PAGE = "https://playit.gg/account/tunnels";

/** Optional playit.gg tunnel. Nothing downloads or runs until the player asks for it. */
export function PlayitPanel({ port }: { port: number }) {
  const { toast } = useStore();
  const [status, setStatus] = useState<PlayitStatus | null>(null);
  const [linking, setLinking] = useState<string | null>(null);
  const [tunnels, setTunnels] = useState<PlayitTunnel[] | null>(null);
  const [busy, setBusy] = useState(false);

  const loadTunnels = () => api.playitTunnels().then(setTunnels).catch((e) => toast(String(e), true));

  useEffect(() => {
    api.playitStatus().then((s) => {
      setStatus(s);
      if (s.linked) loadTunnels();
    }).catch(() => {});
    const offClaim = on<string>("playit-claim", setLinking);
    const offState = on<boolean>("playit-state", (running) => setStatus((s) => (s ? { ...s, running } : s)));
    return () => {
      offClaim.then((f) => f());
      offState.then((f) => f());
    };
  }, []);

  const act = async (fn: () => Promise<PlayitStatus>) => {
    setBusy(true);
    try {
      setStatus(await fn());
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };

  const link = () =>
    act(async () => {
      try {
        const s = await api.playitLink();
        toast("playit.gg is linked.");
        loadTunnels();
        return s;
      } finally {
        setLinking(null);
      }
    });

  if (!status) return null;

  return (
    <section className="card stack" aria-labelledby="playit-title">
      <h3 id="playit-title">Play with friends over the internet</h3>
      {!status.supported ? (
        <p className="small muted">playit.gg doesn't make an agent for this system, so tunnels aren't available here.</p>
      ) : !status.linked ? (
        <>
          <p className="small">
            playit.gg gives your server a public address without touching your router. Cairn downloads playit's official agent, checks its fingerprint, and asks you to approve this computer on playit.gg.
          </p>
          {linking ? (
            <div className="stack">
              <p className="small" role="status">Approve this computer in the playit.gg page that just opened. Waiting...</p>
              <div className="row">
                <button className="btn btn-sm" onClick={() => openUrl(linking)}><Icon name="external" size={14} />Open the page again</button>
                <button className="btn btn-quiet btn-sm" onClick={() => api.playitCancelLink()}>Cancel</button>
              </div>
            </div>
          ) : (
            <div><button className="btn" disabled={busy} onClick={link}><Icon name="globe" />Link playit.gg</button></div>
          )}
        </>
      ) : (
        <>
          <div className="row">
            {status.running ? (
              <button className="btn btn-sm" disabled={busy} onClick={() => act(api.playitStop)}><Icon name="stop" size={14} />Stop tunnel</button>
            ) : (
              <button className="btn btn-sm" disabled={busy} onClick={() => act(api.playitStart)}><Icon name="play" size={14} />Start tunnel</button>
            )}
            <button className="btn btn-quiet btn-sm" onClick={loadTunnels}><Icon name="update" size={14} />Refresh</button>
            <button className="btn btn-quiet btn-sm" onClick={() => openUrl(TUNNELS_PAGE)}><Icon name="external" size={14} />Manage on playit.gg</button>
            <button className="btn btn-quiet btn-sm" disabled={busy} onClick={() => act(api.playitUnlink)}>Unlink</button>
          </div>
          {tunnels && tunnels.length > 0 ? (
            <ul className="playit-tunnels">
              {tunnels.map((t) => (
                <li key={t.name + t.address} className="row">
                  <code className="mono">{t.address || "Waiting for an address"}</code>
                  <span className="small muted">{t.kind}</span>
                  {t.problem && <span className="small host-problem">Disabled: {t.problem}</span>}
                  {t.address && <button className="btn btn-quiet btn-sm" aria-label={`Copy ${t.address}`} onClick={() => navigator.clipboard.writeText(t.address).then(() => toast("Address copied."))}><Icon name="copy" size={14} /></button>}
                </li>
              ))}
            </ul>
          ) : (
            <p className="small muted">
              No tunnels yet. On playit.gg, add a Minecraft Java tunnel that points to <code className="mono">127.0.0.1:{port}</code>, then press Refresh.
            </p>
          )}
          <p className="small muted">The tunnel only runs while you keep it on, and stops when Cairn closes.</p>
        </>
      )}
    </section>
  );
}
