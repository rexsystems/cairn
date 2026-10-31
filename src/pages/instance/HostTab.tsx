import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, on, type InstanceView, type ServerConfig, type ServerInfo, type ServerLogEvent, type ServerStateEvent } from "../../api";
import { Icon } from "../../components/Icon";
import { Switch } from "../../components/ui";
import { useStore } from "../../store";
import { PlayitPanel } from "./PlayitPanel";

const LOG_KEEP = 3000;
const EULA_URL = "https://aka.ms/MinecraftEULA";
const STATE_LABEL = { starting: "Starting", running: "Running", stopping: "Stopping", stopped: "Stopped" } as const;

/** Run a dedicated server built from this instance, on this computer. */
export function HostTab({ inst }: { inst: InstanceView }) {
  const { toast } = useStore();
  const [info, setInfo] = useState<ServerInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [form, setForm] = useState<ServerConfig | null>(null);
  const [lines, setLines] = useState<string[]>([]);
  const [consent, setConsent] = useState(false);
  const [cmd, setCmd] = useState("");
  const logRef = useRef<HTMLPreElement>(null);
  const id = inst.id;

  const refresh = () =>
    api.serverInfo(id).then((i) => {
      setInfo(i);
      setForm((f) => f ?? i.config);
    }).catch((e) => setError(String(e)));

  useEffect(() => {
    refresh();
    api.serverLog(id).then(setLines).catch(() => {});
    const offLog = on<ServerLogEvent>("server-log", (e) => {
      if (e.instanceId === id) setLines((l) => [...l, ...e.lines].slice(-LOG_KEEP));
    });
    const offState = on<ServerStateEvent>("server-state", (e) => {
      if (e.instanceId !== id) return;
      setInfo((i) => (i ? { ...i, state: e.state } : i));
      if (e.state === "stopped" && e.exitCode != null && e.exitCode !== 0) toast(`The ${inst.name} server stopped with code ${e.exitCode}.`, true);
    });
    return () => {
      offLog.then((f) => f());
      offState.then((f) => f());
    };
  }, [id]);

  useEffect(() => {
    const el = logRef.current;
    if (el && el.scrollHeight - el.scrollTop - el.clientHeight < 80) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const act = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
      refresh();
    }
  };

  if (error) return <div className="notice error" role="alert"><Icon name="warn" />{error}</div>;
  if (!info || !form) return <p className="muted">Loading...</p>;

  const live = info.state !== "stopped";
  const dirty = JSON.stringify(form) !== JSON.stringify(info.config);
  const set = <K extends keyof ServerConfig>(k: K, v: ServerConfig[K]) => setForm({ ...form, [k]: v });

  const setUp = () =>
    act(async () => {
      const r = await api.serverPrepare(id);
      const left = r.clientOnly.length ? ` Left out ${r.clientOnly.length} client-only mod${r.clientOnly.length === 1 ? "" : "s"}.` : "";
      toast(`Server ready.${left}`);
    });

  const save = () =>
    act(async () => {
      const saved = await api.serverSaveConfig(id, form);
      setForm(saved);
      toast(live ? "Saved. Restart the server to apply it." : "Saved.");
    });

  const send = async () => {
    const line = cmd.trim();
    if (!line) return;
    setCmd("");
    await api.serverCommand(id, line).catch((e) => toast(String(e), true));
  };

  return (
    <div className="stack host">
      <section className="card stack">
        <div className="row host-head">
          <div>
            <h3>Host a server</h3>
            <p className="small muted">A separate server folder built from this instance's version and mods. Your single-player worlds aren't touched.</p>
          </div>
          <span className={`host-state host-${info.state}`} role="status">{STATE_LABEL[info.state]}</span>
        </div>
        {info.outdated && (
          <div className="notice"><Icon name="warn" />The instance changed version or loader since the server was set up. Set it up again before starting.</div>
        )}
        <div className="row">
          {!live && info.prepared && info.eula && !info.outdated && (
            <button className="btn btn-play" disabled={busy || dirty} onClick={() => act(() => api.serverStart(id))}><Icon name="play" />Start server</button>
          )}
          {live && (
            <>
              <button className="btn btn-stop" disabled={info.state === "stopping"} onClick={() => act(() => api.serverStop(id))}><Icon name="stop" />Stop</button>
              {info.state === "stopping" && <button className="btn btn-danger btn-sm" onClick={() => act(() => api.serverStop(id, true))}>Force stop</button>}
            </>
          )}
          {!live && (
            <button className="btn" disabled={busy} onClick={setUp}>
              <Icon name={info.prepared ? "update" : "download"} />{info.prepared ? "Update server files" : "Set up server"}
            </button>
          )}
          {info.prepared && <button className="btn btn-quiet" onClick={() => api.openDataPath(info.dir).catch((e) => toast(String(e), true))}><Icon name="folder" />Open folder</button>}
        </div>
        {dirty && !live && info.prepared && <p className="small muted">Save the settings below before starting.</p>}
      </section>

      {info.prepared && !info.eula && (
        <section className="card stack" aria-labelledby="eula-title">
          <h3 id="eula-title">Minecraft EULA</h3>
          <p className="small">Mojang requires every server owner to accept the Minecraft End User License Agreement. Cairn won't accept it for you.</p>
          <button className="btn btn-quiet btn-sm host-link" onClick={() => openUrl(EULA_URL)}><Icon name="external" size={14} />Read the EULA</button>
          <label className="row">
            <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} />
            <span>I have read and accept the Minecraft EULA</span>
          </label>
          <div><button className="btn" disabled={!consent || busy} onClick={() => act(() => api.serverAcceptEula(id))}><Icon name="check" />Accept</button></div>
        </section>
      )}

      <section className="card stack" aria-labelledby="host-settings">
        <h3 id="host-settings">Settings</h3>
        <div className="host-grid">
          <label className="field"><span>Port</span><input className="input" type="number" min={1024} max={65535} value={form.port} onChange={(e) => set("port", Number(e.target.value))} /></label>
          <label className="field"><span>Memory (MB)</span><input className="input" type="number" min={512} step={512} value={form.memoryMb} onChange={(e) => set("memoryMb", Number(e.target.value))} /></label>
          <label className="field"><span>Max players</span><input className="input" type="number" min={1} max={200} value={form.maxPlayers} onChange={(e) => set("maxPlayers", Number(e.target.value))} /></label>
          <label className="field"><span>Message of the day</span><input className="input" maxLength={120} value={form.motd} onChange={(e) => set("motd", e.target.value)} /></label>
        </div>
        <Switch checked={form.onlineMode} onChange={(v) => set("onlineMode", v)} label="Check accounts with Mojang (online mode)" />
        {!form.onlineMode && <p className="small muted">Anyone can join with any name while this is off. Keep it on unless every player uses an offline account.</p>}
        <Switch checked={form.lan} onChange={(v) => set("lan", v)} label="Let devices on my network join" />
        <div><button className="btn btn-sm" disabled={!dirty || busy} onClick={save}>Save settings</button></div>
      </section>

      <section className="card stack" aria-labelledby="host-join">
        <h3 id="host-join">How to join</h3>
        <p className="small">On this computer: <code className="mono">localhost:{info.config.port}</code></p>
        {info.config.lan ? (
          <p className="small">
            On your network: <code className="mono">{info.lanAddress ?? "this computer's IP"}:{info.config.port}</code>. Your system may ask whether Java can accept connections.
          </p>
        ) : (
          <p className="small muted">The server only listens on this computer. Turn on network access above, or use a playit.gg tunnel below.</p>
        )}
        <p className="small muted">Cairn never opens ports on your router.</p>
      </section>

      <PlayitPanel port={info.config.port} />

      <section className="card stack" aria-labelledby="host-console">
        <h3 id="host-console">Console</h3>
        <pre ref={logRef} className="log-view host-log" aria-live="off">{lines.length ? lines.join("\n") : "The server log shows up here."}</pre>
        <form className="row" onSubmit={(e) => { e.preventDefault(); send(); }}>
          <input className="input mono host-cmd" aria-label="Server command" placeholder={live ? "say hello, op Steve, whitelist add Alex..." : "Start the server to send commands"} disabled={!live} value={cmd} onChange={(e) => setCmd(e.target.value)} />
          <button className="btn btn-sm" type="submit" disabled={!live || !cmd.trim()}>Send</button>
        </form>
      </section>

      {info.clientOnly.length > 0 && (
        <details className="card">
          <summary>{info.clientOnly.length} client-only mod{info.clientOnly.length === 1 ? "" : "s"} left out</summary>
          <ul className="small host-left-out">{info.clientOnly.map((m) => <li key={m}>{m}</li>)}</ul>
        </details>
      )}
    </div>
  );
}
