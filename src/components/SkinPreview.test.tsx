// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";

const renders = vi.fn();
const applied: string[] = [];
let releaseSkin: (() => void) | null = null;
let failSkin: (() => void) | null = null;
const drewModel = () => renders.mock.calls.some(([objects]) => objects > 4);

vi.mock("three", async (importOriginal) => {
  const three = await importOriginal<typeof import("three")>();
  class FakeRenderer {
    outputColorSpace = "";
    toneMapping = 0;
    setPixelRatio() {}
    setSize() {}
    dispose() {}
    render(scene: import("three").Object3D) {
      // Counts what was on screen: empty scene, two lights and the root group are 4 objects.
      let objects = 0;
      scene.traverse(() => objects++);
      renders(objects);
    }
  }
  return { ...three, WebGLRenderer: FakeRenderer };
});
vi.mock("../lib/skin-rendering", async () => {
  const THREE = await import("three");
  return {
    loadModel: async () => ({ scene: new THREE.Group(), animations: [] }),
    clonePlayer: (s: import("three").Object3D) => s.clone(),
    createTransparentTexture: () => new THREE.Texture(),
    loadSkinTexture: (src: string) =>
      new Promise((resolve, reject) => {
        releaseSkin = () => resolve({ src });
        failSkin = () => reject(new Error("blocked"));
      }),
    applySkinTexture: (_: unknown, tex: { src: string }) => applied.push(tex.src),
    applyCapeTexture: () => {},
  };
});
vi.mock("../api", () => ({ api: { texture: async () => "data:image/png;base64,c2tpbg==" } }));
vi.mock("../store", () => ({ useStore: () => ({ settings: { reduceEffects: false } }) }));
vi.mock("../assets/models/classic-player.gltf?url", () => ({ default: "classic.gltf" }));
vi.mock("../assets/models/slim-player.gltf?url", () => ({ default: "slim.gltf" }));

import { SkinPreview } from "./SkinPreview";

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", class { observe() {} disconnect() {} });
  vi.stubGlobal("IntersectionObserver", class { constructor(private cb: (e: { isIntersecting: boolean }[]) => void) {} observe() { this.cb([{ isIntersecting: true }]); } disconnect() {} });
});
afterEach(() => {
  cleanup();
  renders.mockReset();
  applied.length = 0;
  releaseSkin = failSkin = null;
  vi.unstubAllGlobals();
});

describe("SkinPreview", () => {
  it("draws the model before the skin arrives and redraws once it is applied, with motion off", async () => {
    render(<SkinPreview skinUrl="http://textures.minecraft.net/texture/abc" reducedMotion />);
    await waitFor(() => expect(releaseSkin).not.toBeNull());
    await waitFor(() => expect(drewModel()).toBe(true));
    const before = renders.mock.calls.length;
    releaseSkin!();
    await waitFor(() => expect(applied).toEqual(["data:image/png;base64,c2tpbg=="]));
    await waitFor(() => expect(renders.mock.calls.length).toBeGreaterThan(before));
  });

  it("still shows the model when the skin can't be loaded", async () => {
    render(<SkinPreview skinUrl="http://textures.minecraft.net/texture/gone" reducedMotion />);
    await waitFor(() => expect(failSkin).not.toBeNull());
    failSkin!();
    await waitFor(() => expect(drewModel()).toBe(true));
    expect(applied).toEqual([]);
  });
});
