import { useEffect, useMemo, useState } from "react";
import { api, type CosmeticsMe, type CosmeticsOverview } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, Loading } from "../components/ui";
import { CapePreview } from "../components/CapePreview";
import { badgeChoices, capeChoices, checkCape, clampFrameMs, customCapeNote } from "../lib/cosmetics";

interface Draft { name: string; bytes: Uint8Array; src: string; frames: number; frameMs: number; error?: string }

/** Capes and badges other Cairn Client players see, from the server set in Settings. */
export function Cosmetics() {
  const { accounts, toast, go } = useStore();
  const active = accounts.find((a) => a.active);
  const [view, setView] = useState<CosmeticsOverview | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [customSrc, setCustomSrc] = useState<string | null>(null);

  const load = () => {
    setFailed(null);
    api.cosmeticsOverview(active?.id).then(setView).catch((e) => setFailed(String(e)));
  };
  useEffect(load, [active?.id]);

  const me = view?.me ?? null;
  const customHash = me?.customCape?.hash;
  useEffect(() => {
    setCustomSrc(null);
    if (customHash) api.cosmeticsCapeImage(customHash).then(setCustomSrc).catch(() => setCustomSrc(null));
  }, [customHash]);
  useEffect(() => () => { if (draft) URL.revokeObjectURL(draft.src); }, [draft]);

  const capes = useMemo(() => capeChoices(me), [me]);
  const badges = useMemo(() => badgeChoices(me), [me]);

  const apply = async (run: () => Promise<CosmeticsMe>, done?: string) => {
    setBusy(true);
    try {
      const next = await run();
      setView((v) => (v ? { ...v, me: next } : v));
      if (done) toast(done);
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };
  const equip = (cape: string | null, badge: string | null) => apply(() => api.cosmeticsEquip(active?.id, cape, badge));

  const pick = async (file: File | undefined) => {
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const src = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      const check = checkCape(img.naturalWidth, img.naturalHeight, bytes.length);
      setDraft({ name: file.name, bytes, src, frames: check.ok ? check.frames : 1, frameMs: 100, error: check.ok ? undefined : check.error });
    };
    img.onerror = () => setDraft({ name: file.name, bytes, src, frames: 1, frameMs: 100, error: "That file is not a PNG image." });
    img.src = src;
  };
  const upload = async () => {
    if (!draft || draft.error) return;
    await apply(async () => {
      await api.cosmeticsUpload(active?.id, draft.bytes, draft.frameMs);
      return api.cosmeticsEquip(active?.id, "custom", me?.equipped.badge ?? null);
    }, "Cape uploaded");
    setDraft(null);
  };

  if (failed) return <ErrorState title="Couldn't load cosmetics" error={failed} onRetry={load} />;
  if (!view) return <Loading label="Loading cosmetics" />;
  if (!view.url) {
    return (
      <EmptyState title="No cosmetics server" icon="crown" action={<button className="btn" onClick={() => go({ name: "settings" })}>Open Settings</button>}>
        Capes, badges and the online count come from a server you host. Add its address in Settings.
      </EmptyState>
    );
  }
  if (!view.stats) return <ErrorState title="The cosmetics server is not answering" error={view.error ?? view.url} onRetry={load} />;

  const note = me && customCapeNote(me);
  const equipped = me?.equipped ?? { cape: null, badge: null };
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>Cosmetics</h1>
          <p className="stat-line" aria-label="Players">
            <span><b>{view.stats.online}</b> online now</span>
            <span><b>{view.stats.peak}</b> peak</span>
            <span><b>{view.stats.total}</b> players</span>
          </p>
        </div>
        <button className="btn btn-quiet" onClick={load} disabled={busy}><Icon name="update" /> Refresh</button>
      </div>

      {!me && <p className="notice">{view.error ?? "Sign in to an account to wear cosmetics."} Everyone's cosmetics still show in game.</p>}
      {me?.role === "banned" && <p className="notice">This account is banned from cosmetics on this server.</p>}

      <section className="panel stack" aria-labelledby="c-capes">
        <h2 id="c-capes">Capes</h2>
        <div className="cosmetic-grid">
          <button className="cosmetic" aria-pressed={!equipped.cape} disabled={!me || busy} onClick={() => equip(null, equipped.badge)}>
            <span className="cosmetic-art empty">None</span>
            <span>No cape</span>
          </button>
          {me?.customCape && (
            <button className="cosmetic" aria-pressed={equipped.cape === "custom"} disabled={busy || me.customCape.status === "rejected"} onClick={() => equip("custom", equipped.badge)}>
              {customSrc ? <CapePreview src={customSrc} frames={me.customCape.frames} frameMs={me.customCape.frameMs} label="Your cape" /> : <span className="cosmetic-art empty">…</span>}
              <span>Your cape</span>
              {me.customCape.frames > 1 && <span className="chip">Animated</span>}
            </button>
          )}
          {capes.map((c) => (
            <button key={c.id} className="cosmetic" aria-pressed={equipped.cape === c.id} disabled={!me || c.locked || busy} onClick={() => equip(c.id, equipped.badge)}>
              <img className="cosmetic-art" src={`/cosmetics/cape-${c.id}.png`} alt="" draggable={false} />
              <span>{c.name}</span>
              {c.frames > 1 && <span className="chip chip-moss">Animated</span>}
              {c.locked && <span className="chip">{c.role === "staff" ? "Staff only" : "Locked"}</span>}
            </button>
          ))}
        </div>
      </section>

      <section className="panel stack" aria-labelledby="c-badges">
        <h2 id="c-badges">Nametag badge</h2>
        <div className="cosmetic-grid small">
          <button className="cosmetic" aria-pressed={!equipped.badge} disabled={!me || busy} onClick={() => equip(equipped.cape, null)}>
            <span className="cosmetic-art empty">None</span>
            <span>No badge</span>
          </button>
          {badges.map((b) => (
            <button key={b.id} className="cosmetic" aria-pressed={equipped.badge === b.id} disabled={!me || b.locked || busy} onClick={() => equip(equipped.cape, b.id)}>
              <img className="cosmetic-art" src={`/cosmetics/badge-${b.id}.png`} alt="" draggable={false} />
              <span>{b.name}</span>
              {b.locked && <span className="chip">{b.role === "staff" ? "Staff only" : "Locked"}</span>}
            </button>
          ))}
        </div>
      </section>

      {me && (
        <section className="panel stack" aria-labelledby="c-custom">
          <h2 id="c-custom">Upload your own cape</h2>
          <p className="muted">
            A PNG laid out like a Minecraft cape: 64×32, or any multiple of 64 wide. For an animated cape, stack frames
            under each other (up to 32). {me.moderation === "approval" ? "Server staff approve uploads before others see them." : ""}
          </p>
          {note && <p className="notice">{note}</p>}
          <div className="row" style={{ gap: "var(--space-4)", alignItems: "flex-start", flexWrap: "wrap" }}>
            {draft && !draft.error && <CapePreview src={draft.src} frames={draft.frames} frameMs={draft.frameMs} size={96} label="Preview" />}
            <div className="stack" style={{ flex: 1, minWidth: 220 }}>
              <label className="btn" style={{ alignSelf: "flex-start" }}>
                <Icon name="image" /> Choose PNG…
                <input type="file" accept="image/png" hidden onChange={(e) => { void pick(e.target.files?.[0]); e.target.value = ""; }} />
              </label>
              {draft && <small className="muted">{draft.name}{!draft.error && ` · ${draft.frames} frame${draft.frames === 1 ? "" : "s"}`}</small>}
              {draft?.error && <p className="notice error">{draft.error}</p>}
              {draft && !draft.error && draft.frames > 1 && (
                <label className="field">
                  <span>Frame time (ms)</span>
                  <input className="input" type="number" min={50} max={1000} step={10} value={draft.frameMs}
                    onChange={(e) => setDraft({ ...draft, frameMs: clampFrameMs(Number(e.target.value)) })} />
                </label>
              )}
              <div className="row" style={{ gap: "var(--space-2)" }}>
                <button className="btn" disabled={!draft || !!draft.error || busy} onClick={() => void upload()}><Icon name="upload" /> Upload and wear</button>
                {me.customCape && (
                  <button className="btn btn-quiet" disabled={busy} onClick={() => void apply(() => api.cosmeticsRemoveCape(active?.id), "Cape removed")}>
                    <Icon name="trash" /> Remove my cape
                  </button>
                )}
              </div>
            </div>
          </div>
        </section>
      )}
      <small className="muted">Server: {view.url}</small>
    </div>
  );
}
