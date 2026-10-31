import { useEffect, useState } from "react";
import { api, type InstanceView, type PerformanceAdvice, type LaunchProfile, type LoaderVersion, type McVersion } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { Select, Switch } from "../../components/ui";
import { loaderLabel } from "../../format";

export function OptionsTab({ inst, onSaved }: { inst: InstanceView; onSaved: (i: InstanceView) => void }) {
  const { toast, settings, requestInstall } = useStore();
  const [advice, setAdvice] = useState<PerformanceAdvice | null>(null);
  useEffect(() => { api.performanceAdvice(inst.id).then(setAdvice).catch(() => setAdvice(null)); }, [inst.id, inst.loader, inst.mcVersion]);
  const [form, setForm] = useState(inst);
  const [versions, setVersions] = useState<McVersion[]>([]);
  const [lvs, setLvs] = useState<LoaderVersion[] | null>(null);
  const [saving, setSaving] = useState(false);
  useEffect(() => { api.mcVersions().then((v) => setVersions(v.filter((x) => x.type === "release" || x.id === inst.mcVersion))).catch(() => {}); }, [inst.mcVersion]);
  useEffect(() => {
    if (form.loader === "vanilla") { setLvs([]); return; }
    setLvs(null);
    api.loaderVersions(form.loader, form.mcVersion).then(setLvs).catch(() => setLvs([]));
  }, [form.loader, form.mcVersion]);
  const set = <K extends keyof InstanceView>(k: K, v: InstanceView[K]) => setForm((f) => ({ ...f, [k]: v }));
  const dirty = JSON.stringify(form) !== JSON.stringify(inst);
  const save = async () => {
    setSaving(true);
    try { const i = await api.updateInstance(form); onSaved(i); setForm(i); toast("Saved instance options"); } catch (e) { toast(String(e), true); } finally { setSaving(false); }
  };
  return (
    <div className="settings-grid">
      <section className="panel settings-section">
        <header><h2>General</h2></header>
        <label className="field"><span>Name</span><input className="input" value={form.name} onChange={(e) => set("name", e.target.value)} /></label>
        <div className="field">
          <span>Minecraft version</span>
          <Select
            aria-label="Minecraft version"
            value={form.mcVersion}
            onChange={(v) => {
              setForm((f) => ({
                ...f,
                mcVersion: v,
                loaderVersion: null,
                useHorizon: f.loader === "vanilla" && v === "26.3" ? f.useHorizon : false,
              }));
            }}
            options={[
              ...(!versions.some((v) => v.id === form.mcVersion) ? [{ value: form.mcVersion, label: form.mcVersion }] : []),
              ...versions.map((v) => ({ value: v.id, label: v.id })),
            ]}
          />
          <small>Changing versions can break worlds and mods. Back up first.</small>
        </div>
        <div className="field">
          <span>Loader</span>
          <Select
            aria-label="Loader"
            value={form.loader}
            onChange={(v) => {
              const loader = v as InstanceView["loader"];
              setForm((f) => ({
                ...f,
                loader,
                loaderVersion: null,
                useHorizon: loader === "vanilla" && f.mcVersion === "26.3" ? f.useHorizon : false,
              }));
            }}
            options={Object.entries(loaderLabel).map(([k, v]) => ({ value: k, label: v }))}
          />
        </div>
        {form.loader !== "vanilla" && (
          <div className="field">
            <span>{loaderLabel[form.loader]} version</span>
            <Select
              aria-label={`${loaderLabel[form.loader]} version`}
              value={form.loaderVersion ?? ""}
              onChange={(v) => set("loaderVersion", v || null)}
              options={[
                { value: "", label: "Latest stable" },
                ...(lvs ?? []).slice(0, 60).map((v) => ({ value: v.version, label: `${v.version}${v.stable ? "" : " (beta)"}` })),
              ]}
            />
            {lvs && lvs.length === 0 && <small className="field-error">No {loaderLabel[form.loader]} build for {form.mcVersion}.</small>}
          </div>
        )}
        {(() => {
          const horizonOk = form.loader === "vanilla" && form.mcVersion === "26.3";
          return (
            <div className="field">
              <Switch
                checked={!!form.useHorizon && horizonOk}
                onChange={(v) => set("useHorizon", v)}
                label="Play with Horizon"
                disabled={!horizonOk}
              />
              <small>
                {horizonOk
                  ? "Rust client instead of Java. Worlds stay here. Online-mode servers may not work yet."
                  : "Horizon needs vanilla Minecraft 26.3."}
              </small>
            </div>
          );
        })()}
      </section>
      {advice && advice.pack.length > 0 && (
        <section className="panel settings-section">
          <header><h2>Performance mods</h2><p className="small muted">Sodium or Embeddium, Lithium, FerriteCore, ModernFix, Entity Culling and friends from Modrinth. Mods already installed or without a build for {inst.mcVersion} are skipped, and you see the full list before anything is downloaded.</p></header>
          <div className="row">
            <button className="btn" onClick={() => requestInstall({ instanceId: inst.id, source: "modrinth", roots: advice.pack, title: "Performance mods", review: true })}><Icon name="download" />Add performance mods</button>
          </div>
        </section>
      )}
      <section className="panel settings-section">
        <header><h2>Java and memory</h2><p className="small muted">Leave Java empty and Cairn picks or downloads the right version.</p></header>
        {advice && (
          <div className="field">
            <span>Memory preset</span>
            <div className="segmented" role="group" aria-label="Memory preset">
              {advice.memory.presets.map((p) => (
                <button key={p.id} aria-pressed={form.memoryMb === p.mb} onClick={() => set("memoryMb", p.mb)}>{p.label} · {(p.mb / 1024).toFixed(1)} GB</button>
              ))}
            </div>
            {advice.memory.totalMb > 0 && (
              <p className="small muted">This computer has {(advice.memory.totalMb / 1024).toFixed(1)} GB. Up to {(advice.memory.maxSafeMb / 1024).toFixed(1)} GB leaves enough for the system; {(advice.memory.recommendedMb / 1024).toFixed(1)} GB suits this instance.</p>
            )}
          </div>
        )}
        <label className="field">
          <span>Memory: {((form.memoryMb ?? settings?.defaultMemoryMb ?? 4096) / 1024).toFixed(1)} GB</span>
          <input type="range" min={1024} max={Math.max(16384, advice?.memory.totalMb ?? 0)} step={512} value={form.memoryMb ?? settings?.defaultMemoryMb ?? 4096} onChange={(e) => set("memoryMb", Number(e.target.value))} style={{ accentColor: "var(--moss)", minHeight: 32 }} />
        </label>
        <label className="field"><span>Java executable</span><input className="input mono" placeholder="Automatic" value={form.javaPath ?? ""} onChange={(e) => set("javaPath", e.target.value)} /></label>
        {advice && (form.memoryMb ?? settings?.defaultMemoryMb ?? 4096) > advice.memory.maxSafeMb && advice.memory.totalMb > 0 && (
          <p className="notice warn">More memory than this computer can spare makes the game and the system swap, which causes stutters. Try {(advice.memory.recommendedMb / 1024).toFixed(1)} GB.</p>
        )}
        {advice && (
          <div className="field">
            <span>JVM tuning</span>
            <div className="segmented" role="group" aria-label="JVM tuning">
              {advice.jvm.map((p) => (
                <button key={p.id} title={p.detail} aria-pressed={form.jvmArgs.trim().split(/\s+/).join(" ") === p.args} onClick={() => set("jvmArgs", p.args)}>{p.label}</button>
              ))}
            </div>
          </div>
        )}
        <label className="field"><span>Extra JVM arguments</span><input className="input mono" placeholder="-XX:+UseG1GC" value={form.jvmArgs} onChange={(e) => set("jvmArgs", e.target.value)} /></label>
        <div className="row">
          <label className="field" style={{ flex: 1 }}><span>Window width</span><input className="input" type="number" min={320} placeholder={String(settings?.defaultWidth ?? 1280)} value={form.width ?? ""} onChange={(e) => set("width", e.target.value ? Number(e.target.value) : null)} /></label>
          <label className="field" style={{ flex: 1 }}><span>Window height</span><input className="input" type="number" min={240} placeholder={String(settings?.defaultHeight ?? 720)} value={form.height ?? ""} onChange={(e) => set("height", e.target.value ? Number(e.target.value) : null)} /></label>
        </div>
      </section>
      <section className="panel settings-section" style={{ gridColumn: "1 / -1" }}>
        <header>
          <h2>Launch profiles</h2>
          <p className="small muted">Named JVM setups. “Shaders off” hides the shaderpacks folder for that run.</p>
        </header>
        <div className="field" style={{ maxWidth: 360 }}>
          <span>Active profile</span>
          <Select
            aria-label="Active launch profile"
            value={form.activeLaunchProfile ?? ""}
            onChange={(v) => set("activeLaunchProfile", v || null)}
            options={[
              { value: "", label: "Default (instance settings)" },
              ...(form.launchProfiles ?? []).map((p) => ({ value: p.id, label: p.name })),
            ]}
          />
        </div>
        <div className="stack" style={{ gap: 10 }}>
          {(form.launchProfiles ?? []).map((p, i) => (
            <div key={p.id} className="panel" style={{ padding: 12 }}>
              <div className="row" style={{ gap: 8, alignItems: "flex-end" }}>
                <label className="field" style={{ flex: 1 }}><span>Name</span><input className="input" value={p.name} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, name: e.target.value };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <label className="field" style={{ flex: 1 }}><span>JVM args</span><input className="input mono" value={p.jvmArgs} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, jvmArgs: e.target.value };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <label className="field" style={{ width: 140 }}><span>Memory MB</span><input className="input" type="number" min={1024} placeholder="Default" value={p.memoryMb ?? ""} onChange={(e) => {
                  const launchProfiles = [...(form.launchProfiles ?? [])];
                  launchProfiles[i] = { ...p, memoryMb: e.target.value ? Number(e.target.value) : null };
                  set("launchProfiles", launchProfiles);
                }} /></label>
                <button className="btn btn-quiet btn-icon" aria-label={`Remove ${p.name}`} onClick={() => {
                  const launchProfiles = (form.launchProfiles ?? []).filter((x) => x.id !== p.id);
                  set("launchProfiles", launchProfiles);
                  if (form.activeLaunchProfile === p.id) set("activeLaunchProfile", null);
                }}><Icon name="trash" /></button>
              </div>
              <Switch checked={p.disableShaders} onChange={(v) => {
                const launchProfiles = [...(form.launchProfiles ?? [])];
                launchProfiles[i] = { ...p, disableShaders: v };
                set("launchProfiles", launchProfiles);
              }} label="Shaders off for this profile" />
            </div>
          ))}
        </div>
        <div className="row">
          <button className="btn btn-sm" onClick={() => {
            const id = crypto.randomUUID();
            const next: LaunchProfile = { id, name: "Shaders off", jvmArgs: "", memoryMb: null, javaPath: null, disableShaders: true };
            set("launchProfiles", [...(form.launchProfiles ?? []), next]);
            set("activeLaunchProfile", id);
          }}><Icon name="plus" size={14} />Add “Shaders off”</button>
          <button className="btn btn-sm" onClick={() => {
            const id = crypto.randomUUID();
            set("launchProfiles", [...(form.launchProfiles ?? []), { id, name: "Custom JVM", jvmArgs: "", memoryMb: null, javaPath: null, disableShaders: false }]);
          }}><Icon name="plus" size={14} />Add profile</button>
        </div>
      </section>
      <div className="row" style={{ gridColumn: "1 / -1", justifyContent: "flex-end" }}>
        <button className="btn btn-quiet" disabled={!dirty} onClick={() => setForm(inst)}>Discard changes</button>
        <button className="btn" disabled={!dirty || saving} onClick={save}><Icon name="check" />{saving ? "Saving..." : "Save options"}</button>
      </div>
    </div>
  );
}
