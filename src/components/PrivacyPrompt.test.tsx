// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { Settings } from "../api";

const saveSettings = vi.fn(async (s: Settings) => s);
const setSettings = vi.fn();
let settings: Partial<Settings> | null = null;

vi.mock("../api", () => ({ api: { saveSettings: (s: Settings) => saveSettings(s) } }));
vi.mock("../store", () => ({ useStore: () => ({ settings, setSettings, toast: vi.fn() }) }));

const { PrivacyPrompt } = await import("./PrivacyPrompt");

describe("PrivacyPrompt", () => {
  beforeEach(() => {
    settings = { telemetryEnabled: false, crashReportsEnabled: false, privacyPromptAnswered: false };
    saveSettings.mockClear();
    setSettings.mockClear();
  });
  afterEach(cleanup);

  it("stays hidden once answered", () => {
    settings = { ...settings, privacyPromptAnswered: true };
    render(<PrivacyPrompt />);
    expect(screen.queryByText("Help improve Cairn?")).toBeNull();
  });

  it("saves both off when declined", async () => {
    render(<PrivacyPrompt />);
    fireEvent.click(screen.getByText("No thanks"));
    await waitFor(() => expect(saveSettings).toHaveBeenCalledTimes(1));
    expect(saveSettings.mock.calls[0][0]).toMatchObject({ telemetryEnabled: false, crashReportsEnabled: false, privacyPromptAnswered: true });
  });

  it("saves only what was switched on", async () => {
    render(<PrivacyPrompt />);
    fireEvent.click(screen.getByRole("switch", { name: /Crash and error reports/ }));
    fireEvent.click(screen.getByText("Save choice"));
    await waitFor(() => expect(saveSettings).toHaveBeenCalledTimes(1));
    expect(saveSettings.mock.calls[0][0]).toMatchObject({ telemetryEnabled: false, crashReportsEnabled: true, privacyPromptAnswered: true });
    expect(setSettings).toHaveBeenCalled();
  });
});
