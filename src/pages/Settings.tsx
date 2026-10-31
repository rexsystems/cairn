import { useEffect, useMemo, useState } from "react";
import { api, type JavaInstall, type Settings } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { Loading, Switch } from "../components/ui";

const JAVA_MAJORS = [8, 17, 21, 25];

function envToText(env: Record<string, string>): string {
  return Object.entries(env)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k}=${v}`)
    .join("\n");
}

function textToEnv(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim();
    if (!key) continue;
    out[key] = line.slice(eq + 1);
  }
  return out;
}

export function SettingsPage() {
  const { settings, setSettings, info, toast } = useStore();
  const [form, setForm] = useState<Settings | null>(settings);
  const [envText, setEnvText] = useState(() => envToText(settings?.gameEnv ?? {}));
  const [java, setJava] = useState<JavaInstall[] | null>(null);
  const [javaError, setJavaError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updateNote, setUpdateNote] = useState<string | null>(null);

  useEffect(() => {
    if (settings && !form) {
      setForm(settings);
      setEnvText(envToText(settings.gameEnv ?? {}));
    }
  }, [settings, form]);
  const loadJava = () => { setJavaError(null); api.listJava().then(setJava).catch((e) => setJavaError(String(e))); };
  useEffect(loadJava, []);

  const parsedEnv = useMemo(() => textToEnv(envText), [envText]);
  const formWithEnv = form ? { ...form, gameEnv: parsedEnv } : null;
  const dirty = !!formWithEnv && settings && (
    formWithEnv.theme !== settings.theme
    || formWithEnv.defaultMemoryMb !== settings.defaultMemoryMb
    || formWithEnv.defaultJvmArgs !== settings.defaultJvmArgs
    || formWithEnv.defaultWidth !== settings.defaultWidth
    || formWithEnv.defaultHeight !== settings.defaultHeight
    || formWithEnv.autoInstallJava !== settings.autoInstallJava
    || formWithEnv.concurrentDownloads !== settings.concurrentDownloads
    || formWithEnv.hideOnLaunch !== settings.hideOnLaunch
    || formWithEnv.reduceMotion !== settings.reduceMotion
    || formWithEnv.reduceEffects !== settings.reduceEffects
    || formWithEnv.isolateGameEnv !== settings.isolateGameEnv
    || formWithEnv.discordRpc !== settings.discordRpc
    || formWithEnv.discordShowInstance !== settings.discordShowInstance
    || formWithEnv.gamepadNav !== settings.gamepadNav
    || formWithEnv.telemetryEnabled !== settings.telemetryEnabled
    || formWithEnv.crashReportsEnabled !== settings.crashReportsEnabled
    || (formWithEnv.curseforgeApiKey ?? "") !== (settings.curseforgeApiKey ?? "")
    || JSON.stringify(formWithEnv.gameEnv) !== JSON.stringify(settings.gameEnv ?? {})
  );

  if (!form || !formWithEnv) return <Loading label="Reading settings" />;
  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => setForm({ ...form, [k]: v });

  const persist = async (next: Settings, quiet = false) => {
    setSaving(true);
    try {
      const s = await api.saveSettings(next);
      setSettings(s);
      setForm(s);
      setEnvText(envToText(s.gameEnv ?? {}));
      if (!quiet) toast("Settings saved");
    } catch (e) {
      toast(String(e), true);
    } finally {
      setSaving(false);
    }
  };
  const instant = <K extends keyof Settings>(k: K, v: Settings[K]) => {
    const next = { ...form, [k]: v };
    setForm(next);
    if (settings) persist({ ...settings, [k]: v }, true);
  };

  const installJava = async (major: number) => {
    setInstalling(major);
    try { const j = await api.installJava(major); toast(`Installed Java ${j.major}`); loadJava(); } catch (e) { toast(String(e), true); } finally { setInstalling(null); }
  };

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Settings</h1>
          <p className="muted">Defaults for new instances, Java, game environment and appearance.</p>
        </div>
        <div className="row">
          <button className="btn btn-quiet" disabled={!dirty} onClick={() => { setForm(settings); setEnvText(envToText(settings?.gameEnv ?? {})); }}>Discard</button>
          <button className="btn" disabled={!dirty || saving} onClick={() => persist(formWithEnv)}><Icon name="check" />{saving ? "Saving..." : "Save changes"}</button>
        </div>
      </div>

      <div className="settings-grid">
        <section className="panel settings-section" aria-labelledby="s-look">
          <header><h2 id="s-look">Appearance</h2></header>
          <div className="field">
            <span>Theme</span>
            <div className="segmented" role="group" aria-label="Theme">
              {(["dark", "light", "system"] as const).map((t) => (
                <button key={t} aria-pressed={form.theme === t} onClick={() => instant("theme", t)}>{t === "system" ? "Match system" : t[0].toUpperCase() + t.slice(1)}</button>
              ))}
            </div>
          </div>
          <Switch checked={form.reduceMotion} onChange={(v) => instant("reduceMotion", v)} label="Reduce motion" />
          <Switch checked={form.reduceEffects} onChange={(v) => instant("reduceEffects", v)} label="Lighter graphics" />
          <small className="muted">Turns off shadows and animations and draws the 3D skin at lower quality. Try it if Cairn feels slow.</small>
          <Switch checked={form.hideOnLaunch} onChange={(v) => set("hideOnLaunch", v)} label="Hide Cairn while the game runs" />
          <Switch checked={form.gamepadNav} onChange={(v) => instant("gamepadNav", v)} label="Gamepad navigation in the launcher" />
          <Switch checked={form.discordRpc} onChange={(v) => set("discordRpc", v)} label="Discord Rich Presence while playing" />
          <Switch checked={form.discordShowInstance} disabled={!form.discordRpc} onChange={(v) => set("discordShowInstance", v)} label="Show the instance name in Discord" />
          <small className="muted">Shows the Minecraft version, loader and play time. Updates as soon as you save, even mid-game.</small>
          <Switch
            checked={form.privacyPromptAnswered && form.telemetryEnabled}
            onChange={(v) => setForm({ ...form, telemetryEnabled: v, privacyPromptAnswered: true })}
            label="Share anonymous usage (opens, launches, installs)"
          />
          <Switch
            checked={form.privacyPromptAnswered && form.crashReportsEnabled}
            onChange={(v) => setForm({ ...form, crashReportsEnabled: v, privacyPromptAnswered: true })}
            label="Send crash and error reports"
          />
          <small className="muted">Both are off unless you turn them on. No account names, worlds or files are sent.</small>
          <label className="field">
            <span>CurseForge API key</span>
            <input
              className="input mono"
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={form.curseforgeApiKey ?? ""}
              placeholder="$2a$10$…"
              onChange={(e) => set("curseforgeApiKey", e.target.value)}
              onBlur={() => {
                if (!settings) return;
                if ((form.curseforgeApiKey ?? "") !== (settings.curseforgeApiKey ?? "")) {
                  void persist({ ...form, curseforgeApiKey: form.curseforgeApiKey ?? "" }, true);
                }
              }}
            />
          </label>
          <small className="muted">Used to import CurseForge modpack zips. Official builds include a key; one set here takes priority. Get your own at console.curseforge.com.</small>
        </section>

        <section className="panel settings-section" aria-labelledby="s-game">
          <header><h2 id="s-game">Game defaults</h2><p className="small muted">Instances use these unless they set their own.</p></header>
          <label className="field">
            <span>Memory: {(form.defaultMemoryMb / 1024).toFixed(1)} GB</span>
            <input type="range" min={1024} max={16384} step={512} value={form.defaultMemoryMb} onChange={(e) => set("defaultMemoryMb", Number(e.target.value))} style={{ accentColor: "var(--moss)", minHeight: 32 }} />
          </label>
          <label className="field"><span>JVM arguments</span><input className="input mono" value={form.defaultJvmArgs} placeholder="-XX:+UseG1GC" onChange={(e) => set("defaultJvmArgs", e.target.value)} /></label>
          <div className="row">
            <label className="field" style={{ flex: 1 }}><span>Window width</span><input className="input" type="number" min={320} value={form.defaultWidth} onChange={(e) => set("defaultWidth", Number(e.target.value))} /></label>
            <label className="field" style={{ flex: 1 }}><span>Window height</span><input className="input" type="number" min={240} value={form.defaultHeight} onChange={(e) => set("defaultHeight", Number(e.target.value))} /></label>
          </div>
          <label className="field">
            <span>Parallel downloads: {form.concurrentDownloads}</span>
            <input type="range" min={1} max={64} value={form.concurrentDownloads} onChange={(e) => set("concurrentDownloads", Number(e.target.value))} style={{ accentColor: "var(--moss)", minHeight: 32 }} />
          </label>
        </section>

        <section className="panel settings-section" aria-labelledby="s-env" style={{ gridColumn: "1 / -1" }}>
          <header>
            <h2 id="s-env">Game environment</h2>
            <p className="small muted">Minecraft gets its own process environment so launcher fixes (Wayland, WebKit) do not leak into the game.</p>
          </header>
          <Switch
            checked={form.isolateGameEnv}
            onChange={(v) => set("isolateGameEnv", v)}
            label="Strip launcher GTK and WebKit variables from the game"
          />
          <label className="field">
            <span>Extra environment variables</span>
            <textarea
              className="input"
              value={envText}
              spellCheck={false}
              placeholder={"# one per line, KEY=value\n# empty value removes the variable\n# MESA_GL_VERSION_OVERRIDE=4.5"}
              onChange={(e) => setEnvText(e.target.value)}
              rows={6}
            />
            <small>Applied after isolation. Use an empty value (`NAME=`) to unset a variable the game would otherwise inherit.</small>
          </label>
        </section>

        <section className="panel settings-section" aria-labelledby="s-java" style={{ gridColumn: "1 / -1" }}>
          <header><h2 id="s-java">Java</h2><p className="small muted">Each Minecraft version needs a specific Java. Cairn picks a match and downloads Eclipse Temurin from Adoptium when none is found.</p></header>
          <Switch checked={form.autoInstallJava} onChange={(v) => set("autoInstallJava", v)} label="Download Java automatically when needed" />
          {javaError && <p className="field-error">Could not scan for Java: {javaError}</p>}
          {!java && !javaError && <Loading label="Looking for Java" />}
          {java && java.length === 0 && <p className="muted">No Java found on this computer yet.</p>}
          {java && java.length > 0 && (
            <div className="card content-list">
              {java.map((j) => (
                <div key={j.path} className="content-row">
                  <div className="content-icon" style={{ display: "grid", placeItems: "center" }} aria-hidden="true"><Icon name="cup" /></div>
                  <div className="content-main">
                    <div className="content-title">Java {j.major} {j.managed && <span className="chip" style={{ marginLeft: 6 }}>Managed by Cairn</span>}</div>
                    <div className="content-sub mono">{j.version}  ·  {j.path}</div>
                  </div>
                </div>
              ))}
            </div>
          )}
          <div className="row">
            <span className="small muted">Install Temurin:</span>
            {JAVA_MAJORS.map((m) => (
              <button key={m} className="btn btn-sm" disabled={installing !== null} onClick={() => installJava(m)}>
                <Icon name="download" size={14} />{installing === m ? `Installing Java ${m}...` : `Java ${m}`}
              </button>
            ))}
          </div>
        </section>

        <section className="panel settings-section" aria-labelledby="s-about">
          <header><h2 id="s-about">About Cairn</h2></header>
          {info && (
            <dl className="kv">
              <dt>Version</dt><dd>{info.version}</dd>
              <dt>System</dt><dd>{info.os}</dd>
              <dt>Data folder</dt><dd className="mono">{info.dataDir}</dd>
            </dl>
          )}
          <div className="field">
            <span>Update channel</span>
            <div className="segmented" role="group" aria-label="Update channel">
              {(["stable", "beta"] as const).map((c) => (
                <button key={c} aria-pressed={(form.updateChannel ?? "stable") === c} onClick={() => instant("updateChannel", c)}>{c === "stable" ? "Stable" : "Beta"}</button>
              ))}
            </div>
            <small className="muted">Beta gets every new build first and may have bugs.</small>
          </div>
          <div className="row">
            {info && <button className="btn btn-sm" onClick={() => api.openDataPath(info.dataDir).catch((e) => toast(String(e), true))}><Icon name="folder" size={14} />Open data folder</button>}
            <button
              className="btn btn-sm"
              disabled={updateBusy}
              onClick={async () => {
                setUpdateBusy(true);
                setUpdateNote(null);
                try {
                  const u = await api.checkForUpdate();
                  if (!u) {
                    setUpdateNote("You are on the latest build, or updates are not configured yet.");
                  } else {
                    setUpdateNote(`Update ${u.version} is available. Downloading…`);
                    await u.downloadAndInstall();
                    toast("Update installed. Restarting…");
                    await api.relaunch();
                  }
                } catch (e) {
                  setUpdateNote(String(e));
                } finally {
                  setUpdateBusy(false);
                }
              }}
            >
              <Icon name="update" size={14} />{updateBusy ? "Checking…" : "Check for updates"}
            </button>
          </div>
          {updateNote && <p className="small muted">{updateNote}</p>}
          <p className="small muted">Cairn is not affiliated with Mojang, Microsoft or Modrinth. Minecraft is a trademark of Mojang AB. Mod data comes from the Modrinth API.</p>
        </section>
      </div>
    </>
  );
}
