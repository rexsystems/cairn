import { afterEach, describe, expect, it, vi } from "vitest";
import type { JoinPlan } from "../api";

const api = {
  installModpack: vi.fn(async () => ({ id: "pack-m" })),
  curseforgeInstallModpack: vi.fn(async () => ({ id: "pack-c" })),
  importShareCode: vi.fn(async () => ({ instance: { id: "rebuilt" } })),
  createInstance: vi.fn(async () => ({ id: "made" })),
  joinRecord: vi.fn(async () => []),
};
vi.mock("../api", () => ({ api }));

const { describeJoin, joinNeedsDownload, runJoin } = await import("./join");

const invite = { address: "fun-cat.joinmc.link", mc: "1.21.1", loader: "fabric", lv: "0.16.9", fp: "abc", pack: null };
const plan = (step: JoinPlan["step"]): JoinPlan => ({ invite, step });

describe("runJoin", () => {
  afterEach(() => vi.clearAllMocks());

  it("starts a matching instance straight into the server", async () => {
    const play = vi.fn(async () => {});
    const id = await runJoin(plan({ kind: "launch", instance: { id: "mine", name: "Mine" } }), { secret: "s" }, play);
    expect(id).toBe("mine");
    expect(play).toHaveBeenCalledWith("mine", { server: "fun-cat.joinmc.link" });
    expect(api.joinRecord).toHaveBeenCalledWith({ secret: "s" }, "mine");
    expect(joinNeedsDownload(plan({ kind: "launch", instance: { id: "mine", name: "Mine" } }))).toBe(false);
  });

  it("installs the host's modpack from the right site", async () => {
    const play = vi.fn(async () => {});
    await runJoin(plan({ kind: "installPack", pack: { source: "c", project: "123", version: "456" } }), { secret: "s" }, play);
    expect(api.curseforgeInstallModpack).toHaveBeenCalledWith("123", "456");
    await runJoin(plan({ kind: "installPack", pack: { source: "m", project: "AANobbMI", version: "v1" } }), { secret: "s" }, play);
    expect(api.installModpack).toHaveBeenCalledWith("AANobbMI", "v1");
    expect(play).toHaveBeenLastCalledWith("pack-m", { server: "fun-cat.joinmc.link" });
  });

  it("rebuilds from the invite code and creates plain instances", async () => {
    const play = vi.fn(async () => {});
    await runJoin(plan({ kind: "rebuild" }), { code: "CAIRN1:x@fun-cat.joinmc.link" }, play);
    expect(api.importShareCode).toHaveBeenCalledWith("CAIRN1:x@fun-cat.joinmc.link");
    await runJoin(plan({ kind: "create" }), { secret: "s" }, play);
    expect(api.createInstance).toHaveBeenCalledWith(expect.objectContaining({ mcVersion: "1.21.1", loader: "fabric", loaderVersion: "0.16.9" }));
  });

  it("won't guess mods it doesn't know", async () => {
    const play = vi.fn(async () => {});
    await expect(runJoin(plan({ kind: "needCode" }), { secret: "s" }, play)).rejects.toThrow(/invite code/);
    await expect(runJoin(plan({ kind: "rebuild" }), { secret: "s" }, play)).rejects.toThrow(/missing/);
    expect(play).not.toHaveBeenCalled();
    expect(describeJoin(plan({ kind: "needCode" }))).toMatch(/Ask your friend for the invite code/);
  });
});
