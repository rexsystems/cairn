import { useCallback, useEffect, useMemo, useState } from "react";
import { externalHref, renderProjectBody } from "../../lib/markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type InstanceView } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { EmptyState, ErrorState, Loading, Modal, Select } from "../../components/ui";
import { compactNumber, loaderLabel, relativeTime } from "../../format";

interface Version { id: string; name: string; version_number: string; game_versions: string[]; loaders: string[]; version_type: string; date_published: string; downloads: number }

export function ProjectModal({ id, target, eligible, installed, onTarget, onClose, onInstalled }: {
  id: string; target: InstanceView | null; eligible: InstanceView[]; installed: boolean;
  onTarget: (id: string) => void; onClose: () => void; onInstalled: () => void;
}) {
  const { toast, refreshInstances, go } = useStore();
  const [project, setProject] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [versions, setVersions] = useState<Version[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [tab, setTab] = useState<"about" | "versions">("about");
  const [includeOptional, setIncludeOptional] = useState(false);
  const [deps, setDeps] = useState<{ required: string[]; optional: string[] } | null>(null);

  const load = useCallback(() => { setError(null); api.project(id).then(setProject).catch((e) => setError(String(e))); }, [id]);
  useEffect(load, [load]);
  const isPack = project?.project_type === "modpack";
  useEffect(() => {
    if (!project) return;
    setVersions(null);
    setDeps(null);
    const loaders = target && !isPack && project.project_type === "mod" ? (target.loader === "quilt" ? ["quilt", "fabric"] : [target.loader]) : [];
    const gv = target && !isPack ? [target.mcVersion] : [];
    api.versions(project.id, loaders, gv).then((v) => {
      const list = v as Version[];
      setVersions(list);
      const first = list.find((x) => x.version_type === "release") ?? list[0];
      if (first && project.project_type === "mod") {
        api.versionDeps(first.id).then((d) => {
          setDeps({
            required: d.filter((x) => x.dependencyType === "required").map((x) => x.title || x.projectId || "dependency"),
            optional: d.filter((x) => x.dependencyType === "optional").map((x) => x.title || x.projectId || "dependency"),
          });
        }).catch(() => setDeps(null));
      }
    }).catch(() => setVersions([]));
  }, [project, target, isPack]);

  const body = useMemo(() => renderProjectBody(project?.body), [project]);

  const install = async (versionId?: string) => {
    if (isPack) {
      setBusy(versionId ?? "latest");
      try {
        const inst = await api.installModpack(project.id, versionId);
        await refreshInstances();
        toast(`Installed ${inst.name}`);
        onClose();
        go({ name: "instance", id: inst.id });
      } catch (e) { toast(String(e), true); } finally { setBusy(null); }
      return;
    }
    if (!target) { toast("Pick an instance to install into first.", true); return; }
    setBusy(versionId ?? "latest");
    try {
      const done = await api.install(target.id, project.id, versionId, includeOptional);
      toast(done.length > 1 ? `Installed ${done[0].title} and ${done.length - 1} ${done.length === 2 ? "dependency" : "dependencies"}` : `Installed ${project.title} into ${target.name}`);
      onInstalled();
      refreshInstances();
    } catch (e) { toast(String(e), true); } finally { setBusy(null); }
  };

  const gallery: { url: string; title?: string }[] = project?.gallery ?? [];
  const noMatch = !isPack && target && versions && versions.length === 0;

  return (
    <Modal wide onClose={onClose} title={project ? project.title : "Loading project"}>
      {error && <ErrorState title="This project could not be loaded" error={error} onRetry={load} />}
      {!error && !project && <Loading label="Fetching from Modrinth" />}
      {project && (
        <div className="stack" style={{ gap: 20 }}>
          <div className="project-head">
            {project.icon_url ? <img src={project.icon_url} alt="" /> : <div className="result-icon" style={{ width: 88, height: 88 }} aria-hidden="true" />}
            <div style={{ flex: 1, minWidth: 0 }} className="stack">
              <p>{project.description}</p>
              <div className="row small muted">
                <span><b style={{ color: "var(--text)" }}>{compactNumber(project.downloads)}</b> downloads</span>
                <span><b style={{ color: "var(--text)" }}>{compactNumber(project.followers)}</b> follows</span>
                <span>{project.license?.id ?? "No license listed"}</span>
                <button className="btn btn-quiet btn-sm" onClick={() => openUrl(`https://modrinth.com/${project.project_type}/${project.slug}`)}><Icon name="external" size={14} />Modrinth page</button>
              </div>
            </div>
          </div>

          <div className="panel stack" style={{ gap: 12 }}>
            {isPack ? (
              <div className="row">
                <p style={{ flex: 1, minWidth: 200 }}>Installs as a new instance with everything the pack needs.</p>
                <button className="btn btn-play" style={{ minHeight: 42, fontSize: 15 }} disabled={!!busy} onClick={() => install()}><Icon name="download" />{busy ? "Installing..." : "Install modpack"}</button>
              </div>
            ) : (
              <div className="row" style={{ alignItems: "flex-end" }}>
                <div className="field" style={{ flex: "1 1 240px" }}>
                  <span>Install into</span>
                  <Select
                    aria-label="Install into"
                    value={target?.id ?? ""}
                    onChange={onTarget}
                    options={[
                      { value: "", label: "Pick an instance" },
                      ...eligible.map((i) => ({ value: i.id, label: `${i.name} (${loaderLabel[i.loader]} ${i.mcVersion})` })),
                    ]}
                  />
                </div>
                {installed && target ? (
                  <span className="chip chip-moss" style={{ minHeight: 40 }}><Icon name="check" size={14} />Installed in {target.name}</span>
                ) : (
                  <button className="btn btn-play" style={{ minHeight: 42, fontSize: 15 }} disabled={!target || !!busy || !!noMatch || (target && !versions)} onClick={() => install((versions?.find((v) => v.version_type === "release") ?? versions?.[0])?.id)}>
                    <Icon name="download" />{busy ? "Installing..." : "Install"}
                  </button>
                )}
              </div>
            )}
            {noMatch && <p className="field-error">No version of {project.title} supports {loaderLabel[target!.loader]} {target!.mcVersion}.</p>}
            {!isPack && project.project_type === "mod" && (
              <div className="stack" style={{ gap: 8 }}>
                <small className="muted">Required dependencies install automatically{deps?.required.length ? `: ${deps.required.slice(0, 4).join(", ")}${deps.required.length > 4 ? "…" : ""}` : "."}</small>
                {deps && deps.optional.length > 0 && (
                  <label className="check">
                    <input type="checkbox" checked={includeOptional} onChange={(e) => setIncludeOptional(e.target.checked)} />
                    Also install optional ({deps.optional.slice(0, 3).join(", ")}{deps.optional.length > 3 ? "…" : ""})
                  </label>
                )}
              </div>
            )}
            {!isPack && !target && <small className="muted">Pick an instance first.</small>}
          </div>

          {gallery.length > 0 && (
            <div className="gallery" aria-label="Gallery">
              {gallery.map((g) => <img key={g.url} src={g.url} alt={g.title ?? ""} loading="lazy" />)}
            </div>
          )}

          <div className="tabs" role="tablist">
            <button role="tab" className="tab" aria-selected={tab === "about"} onClick={() => setTab("about")}>About</button>
            <button role="tab" className="tab" aria-selected={tab === "versions"} onClick={() => setTab("versions")}>Versions{versions ? <span className="count">{versions.length}</span> : null}</button>
          </div>
          {tab === "about" && (
            <div className="prose" onClick={(e) => {
              const a = (e.target as HTMLElement).closest("a");
              if (!a) return;
              e.preventDefault();
              const href = externalHref(a.href);
              if (href) openUrl(href);
            }} dangerouslySetInnerHTML={{ __html: body || "<p>No description.</p>" }} />
          )}
          {tab === "versions" && (
            <div>
              {!versions && <Loading label="Listing versions" />}
              {versions && versions.length === 0 && <EmptyState title="No matching versions" />}
              {versions?.slice(0, 50).map((v) => (
                <div key={v.id} className="version-row">
                  <div style={{ minWidth: 0 }}>
                    <div className="content-title">{v.name || v.version_number} {v.version_type !== "release" && <span className="chip chip-ember">{v.version_type}</span>}</div>
                    <div className="content-sub">{v.loaders.map((l) => loaderLabel[l] ?? l).join(", ")}  ·  {v.game_versions.slice(-4).join(", ")}{v.game_versions.length > 4 ? " and more" : ""}  ·  {relativeTime(Date.parse(v.date_published))}</div>
                  </div>
                  <button className="btn btn-sm" disabled={!!busy || (!isPack && !target)} onClick={() => install(v.id)}><Icon name="download" size={14} />{busy === v.id ? "Installing..." : "Install"}</button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
