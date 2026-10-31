import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { externalHref, renderProjectBody } from "../lib/markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type InstanceView, type SearchHit, type SearchResult } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, Loading, Modal, Select } from "../components/ui";
import { buildFacets, compactNumber, loaderLabel, relativeTime } from "../format";

const TYPES = [
  { id: "mod", label: "Mods", blurb: "Change how the game plays" },
  { id: "resourcepack", label: "Resource packs", blurb: "Textures, sounds and UI" },
  { id: "datapack", label: "Datapacks", blurb: "Recipes, loot and world rules" },
  { id: "shader", label: "Shaders", blurb: "Lighting and atmosphere" },
  { id: "modpack", label: "Modpacks", blurb: "Full setups as new instances" },
] as const;
const SORTS = [
  { id: "relevance", label: "Relevance" },
  { id: "downloads", label: "Downloads" },
  { id: "follows", label: "Follows" },
  { id: "newest", label: "Newest" },
  { id: "updated", label: "Recently updated" },
];
const PAGE = 20;
const MOD_LOADERS = ["fabric", "quilt", "forge", "neoforge"];

interface Tag { name: string; project_type: string; header: string }

export function Discover({ instanceId, initialType, openProject }: { instanceId?: string; initialType?: string; openProject?: string }) {
  const { instances, go, back, toast, refreshInstances } = useStore();
  const [type, setType] = useState(initialType ?? "mod");
  const [targetId, setTargetId] = useState<string>(instanceId ?? "");
  const target = useMemo(() => instances?.find((i) => i.id === targetId) ?? null, [instances, targetId]);
  const fromInstance = useMemo(() => (instanceId ? instances?.find((i) => i.id === instanceId) ?? null : null), [instances, instanceId]);
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [sort, setSort] = useState("relevance");
  const [page, setPage] = useState(0);
  const [cats, setCats] = useState<string[]>([]);
  const [loader, setLoader] = useState("");
  const [gameVersion, setGameVersion] = useState("");
  const [tags, setTags] = useState<{ categories: Tag[]; gameVersions: { version: string; version_type: string }[] } | null>(null);
  const [result, setResult] = useState<SearchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [installing, setInstalling] = useState<string | null>(null);
  const [detail, setDetail] = useState<string | null>(openProject ?? null);
  const [typeCovers, setTypeCovers] = useState<Partial<Record<string, string>>>({});
  const top = useRef<HTMLDivElement>(null);

  useEffect(() => { api.tags().then((t) => setTags(t as never)).catch(() => setTags({ categories: [], gameVersions: [] })); }, []);
  useEffect(() => {
    let alive = true;
    (async () => {
      const entries = await Promise.all(TYPES.map(async (t) => {
        try {
          const r = await api.search({
            query: "",
            facets: [[`project_type:${t.id}`]],
            index: "downloads",
            offset: 0,
            limit: 8,
          });
          const hit = r.hits.find((h) => h.featured_gallery || h.gallery?.[0]) ?? r.hits[0];
          const src = hit?.featured_gallery || hit?.gallery?.[0] || hit?.icon_url || null;
          return src ? ([t.id, src] as const) : null;
        } catch {
          return null;
        }
      }));
      if (!alive) return;
      const next: Partial<Record<string, string>> = {};
      for (const e of entries) if (e) next[e[0]] = e[1];
      setTypeCovers(next);
    })();
    return () => { alive = false; };
  }, []);
  useEffect(() => { const t = setTimeout(() => setDebounced(query), 280); return () => clearTimeout(t); }, [query]);
  useEffect(() => { setPage(0); }, [debounced, sort, cats, loader, gameVersion, type, targetId]);
  useEffect(() => { setCats([]); }, [type]);

  // An instance target pins version and loader so every result fits it.
  const effVersion = target && type !== "modpack" ? target.mcVersion : gameVersion;
  const effLoader = target && type === "mod" ? target.loader : loader;

  const refreshInstalled = useCallback(() => {
    if (!targetId) { setInstalled(new Set()); return; }
    api.installedProjects(targetId).then((p) => setInstalled(new Set(p))).catch(() => setInstalled(new Set()));
  }, [targetId]);
  useEffect(refreshInstalled, [refreshInstalled]);

  const search = useCallback(() => {
    setLoading(true);
    setError(null);
    const facets = buildFacets({ projectType: type, gameVersion: effVersion || undefined, loader: effLoader && effLoader !== "vanilla" ? effLoader : undefined, categories: cats });
    api.search({ query: debounced, facets, index: sort, offset: page * PAGE, limit: PAGE })
      .then((r) => { setResult(r); if (page > 0) top.current?.scrollIntoView({ block: "start", behavior: "smooth" }); })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [type, effVersion, effLoader, cats, debounced, sort, page]);
  useEffect(search, [search]);

  const categoryTags = (tags?.categories ?? []).filter((c) => c.project_type === (type === "shader" ? "shader" : type) && c.header === "categories");
  const pages = result ? Math.ceil(result.total_hits / PAGE) : 0;
  const releaseVersions = (tags?.gameVersions ?? []).filter((v) => v.version_type === "release").slice(0, 60);
  const typeLabel = TYPES.find((t) => t.id === type)!.label.toLowerCase();
  const eligible = (instances ?? []).filter((i) => type !== "mod" || i.loader !== "vanilla");

  const goBack = () => {
    if (instanceId) go({ name: "instance", id: instanceId });
    else back();
  };

  const quickInstall = async (hit: SearchHit) => {
    if (type === "modpack") {
      setInstalling(hit.project_id);
      try {
        const inst = await api.installModpack(hit.project_id, null);
        await refreshInstances();
        toast(`Installed ${inst.name}`);
        go({ name: "instance", id: inst.id });
      } catch (e) {
        toast(String(e), true);
      } finally {
        setInstalling(null);
      }
      return;
    }
    if (!target) {
      toast("Pick an instance to install into first.", true);
      return;
    }
    setInstalling(hit.project_id);
    try {
      const done = await api.install(target.id, hit.project_id, null);
      toast(
        done.length > 1
          ? `Installed ${done[0].title} and ${done.length - 1} ${done.length === 2 ? "dependency" : "dependencies"}`
          : `Installed ${hit.title} into ${target.name}`,
      );
      refreshInstalled();
      refreshInstances();
    } catch (e) {
      toast(String(e), true);
    } finally {
      setInstalling(null);
    }
  };

  return (
    <>
      <div className="page-head" ref={top}>
        <div>
          {instanceId && (
            <button type="button" className="btn btn-quiet btn-sm discover-back" onClick={goBack}>
              <Icon name="back" size={14} />
              Back{fromInstance ? ` to ${fromInstance.name}` : ""}
            </button>
          )}
          <h1>Discover</h1>
          <p className="muted">Mods, packs and shaders from Modrinth{target ? `, filtered for ${target.name}` : ""}.</p>
        </div>
        {type !== "modpack" && (
          <div className="field" style={{ minWidth: 240 }}>
            <span>Install into</span>
            <Select
              aria-label="Install into"
              value={targetId}
              onChange={setTargetId}
              options={[
                { value: "", label: "Choose when installing" },
                ...(instances ?? []).map((i) => ({ value: i.id, label: `${i.name} (${loaderLabel[i.loader]} ${i.mcVersion})` })),
              ]}
            />
          </div>
        )}
      </div>

      <div className="type-cards" role="tablist" aria-label="Project type">
        {TYPES.map((t) => {
          const cover = typeCovers[t.id];
          const selected = type === t.id;
          return (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={selected}
              className={`type-card type-card-${t.id}${selected ? " is-active" : ""}`}
              onClick={() => setType(t.id)}
            >
              <span className="type-card-media" aria-hidden="true">
                {cover ? <img src={cover} alt="" loading="lazy" /> : <span className="type-card-fallback" />}
              </span>
              <span className="type-card-body">
                <span className="type-card-label">{t.label}</span>
                <span className="type-card-blurb">{t.blurb}</span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="discover">
        <aside className="filters" aria-label="Filters">
          {!target || type === "modpack" ? (
            <div className="filter-group collapsible">
              <h3>Minecraft version</h3>
              <Select
                aria-label="Minecraft version"
                value={gameVersion}
                onChange={setGameVersion}
                options={[
                  { value: "", label: "Any version" },
                  ...releaseVersions.map((v) => ({ value: v.version, label: v.version })),
                ]}
              />
            </div>
          ) : (
            <div className="notice" style={{ fontSize: 14 }}>
              <Icon name="check" />
              <span>Showing {typeLabel} for Minecraft {target.mcVersion}{type === "mod" ? ` on ${loaderLabel[target.loader]}` : ""}.</span>
            </div>
          )}
          {(type === "mod" || type === "modpack") && !(target && type === "mod") && (
            <div className="filter-group collapsible" role="radiogroup" aria-label="Loader">
              <h3>Loader</h3>
              {["", ...MOD_LOADERS].map((l) => (
                <label key={l || "any"} className="check"><input type="radio" name="loader" checked={loader === l} onChange={() => setLoader(l)} />{l ? loaderLabel[l] : "Any loader"}</label>
              ))}
            </div>
          )}
          {categoryTags.length > 0 && (
            <div className="filter-group collapsible">
              <h3>Categories</h3>
              {categoryTags.map((c) => (
                <label key={c.name} className="check">
                  <input type="checkbox" checked={cats.includes(c.name)} onChange={(e) => setCats((p) => (e.target.checked ? [...p, c.name] : p.filter((x) => x !== c.name)))} />
                  {c.name.replace(/-/g, " ").replace(/^./, (s) => s.toUpperCase())}
                </label>
              ))}
            </div>
          )}
        </aside>

        <div className="results">
          <div className="row" style={{ gap: 12 }}>
            <label className="search">
              <span className="sr-only">Search {typeLabel}</span>
              <Icon name="search" size={16} />
              <input className="input" type="search" placeholder={`Search ${typeLabel}`} value={query} onChange={(e) => setQuery(e.target.value)} />
            </label>
            <div className="field" style={{ flex: "0 0 180px" }}>
              <span className="sr-only">Sort by</span>
              <Select
                aria-label="Sort by"
                value={sort}
                onChange={setSort}
                options={SORTS.map((s) => ({ value: s.id, label: `Sort: ${s.label}` }))}
              />
            </div>
          </div>
          {result && !error && <p className="small muted" aria-live="polite">{compactNumber(result.total_hits)} results</p>}

          {error && <ErrorState title="Modrinth search failed" error={error} onRetry={search} />}
          {!error && loading && !result && <div className="stack">{[0, 1, 2, 3, 4].map((i) => <div key={i} className="skeleton" />)}</div>}
          {!error && result && result.hits.length === 0 && (
            <EmptyState icon="search" title={`No ${typeLabel} match`}>Try fewer filters or a different search.</EmptyState>
          )}
          {!error && result && result.hits.length > 0 && (
            <div className="stack" style={{ opacity: loading ? 0.55 : 1, transition: "opacity var(--t-fast)" }} aria-busy={loading}>
              {result.hits.map((h) => (
                <Result
                  key={h.project_id}
                  hit={h}
                  installed={installed.has(h.project_id)}
                  installing={installing === h.project_id}
                  canQuickInstall={type === "modpack" || !!target}
                  quickLabel={type === "modpack" ? "Install pack" : target ? `Install for ${target.mcVersion}` : "Install"}
                  onOpen={() => setDetail(h.project_id)}
                  onInstall={() => quickInstall(h)}
                />
              ))}
            </div>
          )}
          {pages > 1 && !error && (
            <nav className="row" aria-label="Pages" style={{ justifyContent: "center", marginTop: 8 }}>
              <button className="btn btn-sm" disabled={page === 0 || loading} onClick={() => setPage(page - 1)}>Previous</button>
              <span className="small muted">Page {page + 1} of {compactNumber(pages)}</span>
              <button className="btn btn-sm" disabled={page + 1 >= pages || loading} onClick={() => setPage(page + 1)}>Next</button>
            </nav>
          )}
        </div>
      </div>

      {detail && (
        <ProjectModal id={detail} target={target} eligible={eligible} installed={installed.has(detail)}
          onTarget={setTargetId} onClose={() => setDetail(null)} onInstalled={refreshInstalled} />
      )}
    </>
  );
}

function Result({
  hit,
  installed,
  installing,
  canQuickInstall,
  quickLabel,
  onOpen,
  onInstall,
}: {
  hit: SearchHit;
  installed: boolean;
  installing: boolean;
  canQuickInstall: boolean;
  quickLabel: string;
  onOpen: () => void;
  onInstall: () => void;
}) {
  return (
    <article className="card result" onClick={onOpen}>
      {hit.icon_url ? <img className="result-icon" src={hit.icon_url} alt="" loading="lazy" /> : <div className="result-icon" aria-hidden="true" />}
      <div style={{ minWidth: 0 }}>
        <div className="result-title">
          <h3><button className="linklike" onClick={(e) => { e.stopPropagation(); onOpen(); }}>{hit.title}</button></h3>
          <span className="small muted">by {hit.author}</span>
        </div>
        <p className="result-desc">{hit.description}</p>
        <div className="result-tags">
          {(hit.display_categories ?? hit.categories).slice(0, 5).map((c) => <span key={c} className="chip">{c.replace(/-/g, " ")}</span>)}
        </div>
      </div>
      <div className="result-meta">
        <span><b style={{ color: "var(--text)" }}>{compactNumber(hit.downloads)}</b> downloads</span>
        <span><b style={{ color: "var(--text)" }}>{compactNumber(hit.follows)}</b> follows</span>
        <span>Updated {relativeTime(Date.parse(hit.date_modified))}</span>
        {installed ? (
          <span className="chip chip-moss"><Icon name="check" size={12} />Installed</span>
        ) : (
          <button
            type="button"
            className="btn btn-sm btn-play"
            disabled={!canQuickInstall || installing}
            title={canQuickInstall ? quickLabel : "Pick an instance above first"}
            onClick={(e) => { e.stopPropagation(); onInstall(); }}
          >
            <Icon name="download" size={14} />
            {installing ? "Installing..." : "Install"}
          </button>
        )}
      </div>
    </article>
  );
}

interface Version { id: string; name: string; version_number: string; game_versions: string[]; loaders: string[]; version_type: string; date_published: string; downloads: number }

function ProjectModal({ id, target, eligible, installed, onTarget, onClose, onInstalled }: {
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
