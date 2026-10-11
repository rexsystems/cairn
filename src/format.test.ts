import { describe, expect, it } from "vitest";
import { buildFacets, bytes, compactNumber, playTime, relativeTime } from "./format";

describe("format", () => {
  it("compacts numbers", () => {
    expect(compactNumber(999)).toBe("999");
    expect(compactNumber(1500)).toBe("1.5k");
    expect(compactNumber(25_000)).toBe("25k");
    expect(compactNumber(3_200_000)).toBe("3.2M");
  });
  it("formats bytes", () => {
    expect(bytes(512)).toBe("512 B");
    expect(bytes(2048)).toBe("2 KB");
    expect(bytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });
  it("formats play time", () => {
    expect(playTime(0)).toBe("not played yet");
    expect(playTime(90)).toBe("1 min");
    expect(playTime(3600 * 2 + 60 * 5)).toBe("2 h 5 min");
  });
  it("formats relative time", () => {
    const now = 1_000_000_000_000;
    expect(relativeTime(null, now)).toBe("never");
    expect(relativeTime(now - 10_000, now)).toBe("just now");
    expect(relativeTime(now - 3 * 3600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 26 * 3600_000, now)).toBe("yesterday");
  });
  it("builds Modrinth facets", () => {
    expect(buildFacets({ projectType: "mod", gameVersion: "1.21.1", loader: "fabric" })).toEqual([
      ["project_type:mod"], ["versions:1.21.1"], ["categories:fabric"],
    ]);
    expect(buildFacets({ projectType: "mod", loader: "quilt" })[1]).toEqual(["categories:quilt", "categories:fabric"]);
    expect(buildFacets({ projectType: "resourcepack", loader: "fabric", categories: ["16x"] })).toEqual([
      ["project_type:resourcepack"], ["categories:16x"],
    ]);
  });
});
