// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { CrashAnalysis } from "../api";

const analyzeCrash = vi.fn<(id: string, log?: string) => Promise<CrashAnalysis>>();
const applyFix = vi.fn(async () => "Memory set to 6 GB");
const toast = vi.fn();

vi.mock("../api", () => ({ api: { analyzeCrash: (id: string, log?: string) => analyzeCrash(id, log), applyFix: () => applyFix() } }));
vi.mock("../store", () => ({ useStore: () => ({ toast, go: vi.fn() }) }));

const { CrashHelp } = await import("./CrashHelp");

const memory: CrashAnalysis = {
  source: "crash-reports/crash-1.txt",
  issues: [{ key: "crash-memory", severity: "error", title: "Minecraft ran out of memory", detail: "The game used all 4 GB it was given.", files: [], fixes: [{ label: "Use 6 GB", type: "setMemory", mb: 6144 } as never] }],
};

describe("CrashHelp", () => {
  afterEach(() => { cleanup(); analyzeCrash.mockReset(); applyFix.mockClear(); });

  it("shows the cause and applies its fix", async () => {
    analyzeCrash.mockResolvedValue(memory);
    render(<CrashHelp instanceId="pack" log="/x/crash-1.txt" onClose={() => {}} />);
    expect(await screen.findByText("Minecraft ran out of memory")).toBeTruthy();
    expect(analyzeCrash).toHaveBeenCalledWith("pack", "/x/crash-1.txt");
    fireEvent.click(screen.getByText("Use 6 GB"));
    await waitFor(() => expect(applyFix).toHaveBeenCalledTimes(1));
    expect(toast).toHaveBeenCalledWith("Memory set to 6 GB");
    await waitFor(() => expect(analyzeCrash).toHaveBeenCalledTimes(2));
  });

  it("says so when nothing is recognised", async () => {
    analyzeCrash.mockResolvedValue({ source: ".cairn/logs/launch.log", issues: [] });
    render(<CrashHelp instanceId="pack" onClose={() => {}} />);
    expect(await screen.findByText("No known cause in .cairn/logs/launch.log")).toBeTruthy();
  });

  it("handles instances that never ran", async () => {
    analyzeCrash.mockResolvedValue({ source: null, issues: [] });
    render(<CrashHelp instanceId="pack" onClose={() => {}} />);
    expect(await screen.findByText("No crash or session log yet.")).toBeTruthy();
  });
});
