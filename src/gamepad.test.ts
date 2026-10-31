// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { startGamepadNav } from "./gamepad";

const pads: (Gamepad | null)[] = [];

describe("gamepad navigation", () => {
  afterEach(() => { vi.restoreAllMocks(); pads.length = 0; });

  const setup = () => {
    Object.defineProperty(navigator, "getGamepads", { configurable: true, value: () => pads });
    return vi.spyOn(window, "requestAnimationFrame").mockImplementation(() => 1);
  };

  it("does not poll without a connected pad", () => {
    const raf = setup();
    const stop = startGamepadNav(() => true);
    expect(raf).not.toHaveBeenCalled();
    stop();
  });

  it("starts polling when a pad connects", () => {
    const raf = setup();
    const stop = startGamepadNav(() => true);
    pads.push({ axes: [0, 0], buttons: [] } as unknown as Gamepad);
    window.dispatchEvent(new Event("gamepadconnected"));
    expect(raf).toHaveBeenCalledTimes(1);
    stop();
  });

  it("stays idle while disabled", () => {
    const raf = setup();
    pads.push({ axes: [0, 0], buttons: [] } as unknown as Gamepad);
    const stop = startGamepadNav(() => false);
    window.dispatchEvent(new Event("gamepadconnected"));
    expect(raf).not.toHaveBeenCalled();
    stop();
  });
});
