import { describe, expect, it } from "vitest";
import { badgeChoices, capeChoices, checkCape, clampFrameMs, customCapeNote, frameAt } from "./cosmetics";
import type { CosmeticsMe } from "../api";

const me = (over: Partial<CosmeticsMe> = {}): CosmeticsMe => ({
  uuid: "u", name: "Steve", role: "player", equipped: { cape: null, badge: null }, customCape: null,
  unlocked: { capes: ["classic", "spectrum"], badges: ["moss"] }, moderation: "approval", ...over,
});

describe("checkCape", () => {
  it("accepts single frames and vertical strips", () => {
    expect(checkCape(64, 32, 1000)).toEqual({ ok: true, frames: 1 });
    expect(checkCape(128, 64 * 16, 1000)).toEqual({ ok: true, frames: 16 });
  });
  it("explains what is wrong, like the server would", () => {
    expect(checkCape(64, 64 * 33 / 2, 10)).toMatchObject({ ok: false, error: expect.stringContaining("32 frames") });
    expect(checkCape(100, 50, 10)).toMatchObject({ ok: false, error: expect.stringContaining("width") });
    expect(checkCape(64, 40, 10)).toMatchObject({ ok: false, error: expect.stringContaining("64×32") });
    expect(checkCape(64, 32, 2 * 1024 * 1024)).toMatchObject({ ok: false, error: expect.stringContaining("1 MB") });
    expect(checkCape(1024, 512 * 16, 10)).toMatchObject({ ok: false, error: expect.stringContaining("too many pixels") });
  });
});

describe("animation", () => {
  it("loops frames and keeps frame times in the server's range", () => {
    expect(frameAt(0, 16, 100)).toBe(0);
    expect(frameAt(1650, 16, 100)).toBe(0);
    expect(frameAt(250, 16, 100)).toBe(2);
    expect(frameAt(999, 1, 0)).toBe(0);
    expect(clampFrameMs(10)).toBe(50);
    expect(clampFrameMs(5000)).toBe(1000);
    expect(clampFrameMs(Number.NaN)).toBe(100);
  });
});

describe("choices", () => {
  it("locks what the server did not unlock and lists it last", () => {
    const capes = capeChoices(me());
    expect(capes.find((c) => c.id === "staff")?.locked).toBe(true);
    expect(capes[0].locked).toBe(false);
    expect(capes[capes.length - 1].locked).toBe(true);
    expect(badgeChoices(me()).filter((b) => !b.locked).map((b) => b.id)).toEqual(["moss"]);
  });
  it("shows the public catalog when signed out", () => {
    expect(capeChoices(null).find((c) => c.id === "staff")?.locked).toBe(true);
    expect(capeChoices(null).find((c) => c.id === "flow")?.locked).toBe(false);
  });
  it("tells the player why their upload is not visible yet", () => {
    const cape = { hash: "h", frames: 1, frameMs: 0, url: "/v1/capes/h.png" };
    expect(customCapeNote(me({ customCape: { ...cape, status: "pending", visible: false } }))).toContain("approval");
    expect(customCapeNote(me({ customCape: { ...cape, status: "pending", visible: true } }))).toBeNull();
    expect(customCapeNote(me({ customCape: { ...cape, status: "rejected", visible: false } }))).toContain("rejected");
  });
});
