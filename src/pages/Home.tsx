import { useEffect, useState } from "react";
import { api, fileSrc, type InstanceView, type SearchHit } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, InstanceIcon, Loading } from "../components/ui";
import { PlayButton } from "../components/PlayButton";
import { InstanceCard } from "../components/InstanceCard";
import { compactNumber, loaderLabel, playTime } from "../format";

interface HeroImage { src: string; credit?: string }

async function heroImageFor(inst: InstanceView, fallback: SearchHit | undefined): Promise<HeroImage | null> {
  try {
    if (inst.cover) return { src: fileSrc(inst.cover)!, credit: "Your latest screenshot" };
  } catch { /* fall through to other sources */ }
  if (inst.pack) {
    try {
      const p = await api.project(inst.pack.projectId);
      const g = p.gallery?.find((x: any) => x.featured) ?? p.gallery?.[0];
      if (g) return { src: g.url, credit: `Image from ${p.title} on Modrinth` };
    } catch { /* ignore */ }
  }
  if (fallback?.featured_gallery) return { src: fallback.featured_gallery, credit: `Image from ${fallback.title} on Modrinth` };
  return null;
}

/** A pack image that differs per instance and avoids the first card in the modpack row below. */
function pickArt(id: string, packs: SearchHit[] | null): SearchHit | undefined {
  const withArt = (packs ?? []).slice(0, 7).filter((p) => p.featured_gallery);
  if (withArt.length < 2) return withArt[0];
  const h = [...id].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);
  return withArt[1 + (h % (withArt.length - 1))];
}

function Hero({ inst, packs }: { inst: InstanceView; packs: SearchHit[] | null }) {
  const { go } = useStore();
  const [img, setImg] = useState<HeroImage | null | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    heroImageFor(inst, pickArt(inst.id, packs)).then((i) => alive && setImg(i));
    return () => { alive = false; };
  }, [inst.id, packs]);
  return (
    <section className="hero" aria-labelledby="hero-title">
      <div className="hero-media">{img && <img src={img.src} alt="" />}</div>
      {img?.credit && <span className="hero-credit">{img.credit}</span>}
      <div className="hero-body">
        <InstanceIcon name={inst.name} src={inst.iconPath} large />
        <div style={{ flex: 1, minWidth: 220 }}>
          <div className="hero-kicker">{inst.lastPlayed ? "Jump back in" : "Ready when you are"}</div>
          <h1 id="hero-title" className="hero-title">{inst.name}</h1>
          <div className="hero-meta">
            <span>Minecraft {inst.mcVersion}</span>
            <span>{loaderLabel[inst.loader]}{inst.loaderVersion ? ` ${inst.loaderVersion}` : ""}</span>
            {inst.modCount > 0 && <span>{inst.modCount} mods</span>}
            <span>{inst.playSeconds >= 60 ? `Played ${playTime(inst.playSeconds)}` : "Not played yet"}</span>
          </div>
        </div>
        <div className="row">
          <button className="btn" style={{ background: "rgba(20,19,18,0.6)", color: "#f4eee5", borderColor: "#857a6c" }} onClick={() => go({ name: "instance", id: inst.id })}>
            Open instance
          </button>
          <PlayButton id={inst.id} />
        </div>
      </div>
    </section>
  );
}

export function Home() {
  const { instances, instancesError, refreshInstances, go, setShowCreate, accounts } = useStore();
  const [packs, setPacks] = useState<SearchHit[] | null>(null);
  const [packsError, setPacksError] = useState<string | null>(null);

  const loadPacks = () => {
    setPacksError(null);
    api.search({ query: "", facets: [["project_type:modpack"]], index: "follows", offset: 0, limit: 7 })
      .then((r) => setPacks(r.hits))
      .catch((e) => setPacksError(String(e)));
  };
  useEffect(loadPacks, []);

  const sorted = instances ? [...instances].sort((a, b) => (b.lastPlayed ?? b.created) - (a.lastPlayed ?? a.created)) : [];
  const top = sorted[0];
  const hasAccount = accounts.length > 0;

  return (
    <>
      {instances === null && !instancesError && <div className="skeleton" style={{ minHeight: 300 }} aria-label="Loading instances" />}
      {instancesError && <ErrorState title="Your instances could not be read" error={instancesError} onRetry={refreshInstances} />}
      {instances && top && <Hero inst={top} packs={packs} />}
      {instances && !top && (
        <section className="hero hero-empty" aria-labelledby="welcome">
          <div className="hero-body">
            <div style={{ flex: 1, minWidth: 240 }}>
              <h1 id="welcome" className="hero-title">Build your first stack.</h1>
              <p className="muted" style={{ marginTop: 10, maxWidth: 560 }}>
                An instance is one Minecraft setup: a game version, a mod loader and its own mods, worlds and settings.
                {!hasAccount && " Add an offline name in Accounts to play right away, or sign in with Microsoft for online servers."}
              </p>
            </div>
            <div className="row">
              <button className="btn" onClick={() => go({ name: "discover", projectType: "modpack" })}><Icon name="discover" />Browse modpacks</button>
              <button className="btn btn-play" onClick={() => setShowCreate(true)}><Icon name="plus" />Create an instance</button>
            </div>
          </div>
        </section>
      )}

      {sorted.length > 1 && (
        <section className="stack" aria-labelledby="recent-h">
          <div className="page-head">
            <h2 id="recent-h">Your instances</h2>
            <button className="btn btn-quiet" onClick={() => go({ name: "library" })}>Open library</button>
          </div>
          <div className="recent-row">
            {sorted.slice(1, 9).map((i) => <InstanceCard key={i.id} inst={i} />)}
          </div>
        </section>
      )}

      <section className="stack" aria-labelledby="packs-h">
        <div className="page-head">
          <div>
            <h2 id="packs-h">Most-followed modpacks on Modrinth</h2>
            <p className="muted small">Live from the Modrinth API. Installing one creates a new instance.</p>
          </div>
          <button className="btn btn-quiet" onClick={() => go({ name: "discover", projectType: "modpack" })}>Browse all modpacks</button>
        </div>
        {packs === null && !packsError && <Loading label="Loading modpacks from Modrinth" />}
        {packsError && <ErrorState title="Modrinth could not be reached" error={packsError} onRetry={loadPacks} />}
        {packs && packs.length === 0 && <EmptyState title="No modpacks returned">Modrinth answered with an empty list. Try again later.</EmptyState>}
        {packs && packs.length > 0 && (
          <div className="pack-row">
            {packs.slice(0, 7).map((p) => (
              <button key={p.project_id} className="card pack-card" onClick={() => go({ name: "discover", projectType: "modpack", projectId: p.project_id })}>
                <div className="cover">{(p.featured_gallery || p.gallery?.[0]) && <img src={p.featured_gallery || p.gallery![0]} alt="" loading="lazy" />}</div>
                <div className="body">
                  {p.icon_url ? <img src={p.icon_url} alt="" /> : <span style={{ width: 40 }} />}
                  <div style={{ minWidth: 0 }}>
                    <h3>{p.title}</h3>
                    <p className="small muted" style={{ display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{p.description}</p>
                    <p className="small muted" style={{ marginTop: 6 }}>{compactNumber(p.downloads)} downloads, {compactNumber(p.follows)} followers</p>
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </section>
    </>
  );
}
