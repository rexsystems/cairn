import catalogJson from "./cosmeticsCatalog.json";
import type { CosmeticsMe } from "../api";

export interface CatalogCape { id: string; name: string; frames: number; frameMs: number; role: string | null }
export interface CatalogBadge { id: string; name: string; role: string | null }
export const catalog = catalogJson as { version: number; capes: CatalogCape[]; badges: CatalogBadge[] };

// Same limits as cosmetics-server/src/cape.rs, so a bad file is explained before it is uploaded.
export const MAX_UPLOAD_BYTES = 1024 * 1024;
const MAX_FRAMES = 32;
const MAX_WIDTH = 1024;
const MAX_PIXELS = 4 * 1024 * 1024;
export const FRAME_MS_MIN = 50;
export const FRAME_MS_MAX = 1000;

export type CapeCheck = { ok: true; frames: number } | { ok: false; error: string };

/** Capes are 64×32 per frame (or any multiple of 64 wide); animated ones stack frames vertically. */
export function checkCape(width: number, height: number, bytes: number): CapeCheck {
  if (bytes > MAX_UPLOAD_BYTES) return { ok: false, error: "Capes must be under 1 MB." };
  if (width < 64 || width % 64 !== 0 || width > MAX_WIDTH) return { ok: false, error: "Cape width must be 64, 128, 256, 512 or 1024 pixels." };
  const frameHeight = width / 2;
  if (height === 0 || height % frameHeight !== 0) return { ok: false, error: `Each frame must be ${width}×${frameHeight}; animated capes stack frames vertically.` };
  const frames = height / frameHeight;
  if (frames > MAX_FRAMES) return { ok: false, error: `Animated capes can have at most ${MAX_FRAMES} frames.` };
  if (width * height > MAX_PIXELS) return { ok: false, error: "The cape has too many pixels; use a smaller width or fewer frames." };
  return { ok: true, frames };
}

export function frameAt(elapsedMs: number, frames: number, frameMs: number): number {
  if (frames <= 1 || frameMs <= 0) return 0;
  return Math.floor(elapsedMs / frameMs) % frames;
}

export function clampFrameMs(ms: number): number {
  if (!Number.isFinite(ms)) return 100;
  return Math.min(FRAME_MS_MAX, Math.max(FRAME_MS_MIN, Math.round(ms)));
}

/** Catalog capes first (locked ones last), then the player's own upload if the server shows it. */
export function capeChoices(me: CosmeticsMe | null) {
  const unlocked = new Set(me?.unlocked.capes ?? catalog.capes.filter((c) => !c.role).map((c) => c.id));
  return catalog.capes
    .map((c) => ({ ...c, locked: !unlocked.has(c.id) }))
    .sort((a, b) => Number(a.locked) - Number(b.locked));
}

export function badgeChoices(me: CosmeticsMe | null) {
  const unlocked = new Set(me?.unlocked.badges ?? catalog.badges.filter((b) => !b.role).map((b) => b.id));
  return catalog.badges.map((b) => ({ ...b, locked: !unlocked.has(b.id) })).sort((a, b) => Number(a.locked) - Number(b.locked));
}

export function customCapeNote(me: CosmeticsMe): string | null {
  const c = me.customCape;
  if (!c) return null;
  if (c.status === "rejected") return "Your cape was rejected by the server staff. Upload a different one.";
  if (c.status === "pending" && !c.visible) return "Waiting for staff approval. Only you can see it until then.";
  return null;
}
