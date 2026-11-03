import { describe, expect, it } from "vitest";
import { cairnClientSupported } from "./cairnClient";

describe("Cairn Client builds", () => {
  it("needs the matching loader and no Horizon", () => {
    expect(cairnClientSupported("fabric", "1.21.11")).toBe(true);
    expect(cairnClientSupported("forge", "1.21.11")).toBe(false);
    expect(cairnClientSupported("forge", "1.8.9")).toBe(true);
    expect(cairnClientSupported("fabric", "1.21.11", true)).toBe(false);
  });
});
