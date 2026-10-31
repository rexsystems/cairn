// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { InstanceView, PlayitStatus, ServerInfo } from "../../api";

const handlers = new Map<string, (p: unknown) => void>();
const api = {
  serverInfo: vi.fn<(id: string) => Promise<ServerInfo>>(),
  serverLog: vi.fn(async () => [] as string[]),
  serverAcceptEula: vi.fn(async () => {}),
  serverStart: vi.fn(async () => {}),
  serverCommand: vi.fn(async () => {}),
  playitStatus: vi.fn(async (): Promise<PlayitStatus> => ({ supported: true, installed: false, linked: false, running: false })),
  playitLink: vi.fn(),
};
const toast = vi.fn();

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn(async () => {}) }));
vi.mock("../../api", () => ({
  api,
  on: (event: string, cb: (p: unknown) => void) => {
    handlers.set(event, cb);
    return Promise.resolve(() => handlers.delete(event));
  },
}));
vi.mock("../../store", () => ({ useStore: () => ({ toast }) }));

const { HostTab } = await import("./HostTab");

const inst = { id: "pack", name: "Pack" } as InstanceView;
const base: ServerInfo = {
  dir: "/data/servers/pack",
  config: { port: 25565, memoryMb: 2048, maxPlayers: 8, onlineMode: true, motd: "A Cairn server", lan: false },
  prepared: true,
  outdated: false,
  eula: false,
  state: "stopped",
  lanAddress: "192.168.1.20",
  clientOnly: ["Sodium"],
};

describe("HostTab", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); handlers.clear(); });

  it("won't start until the player accepts the EULA themselves", async () => {
    api.serverInfo.mockResolvedValueOnce(base).mockResolvedValue({ ...base, eula: true });
    render(<HostTab inst={inst} />);
    const accept = (await screen.findByText("Accept")).closest("button")!;
    expect(screen.queryByText("Start server")).toBeNull();
    expect(accept.disabled).toBe(true);
    fireEvent.click(screen.getByLabelText("I have read and accept the Minecraft EULA"));
    fireEvent.click(accept);
    await waitFor(() => expect(api.serverAcceptEula).toHaveBeenCalledWith("pack"));
    fireEvent.click(await screen.findByText("Start server"));
    await waitFor(() => expect(api.serverStart).toHaveBeenCalledWith("pack"));
    expect(screen.getByText("1 client-only mod left out")).toBeTruthy();
  });

  it("shows live log lines and sends console commands", async () => {
    api.serverInfo.mockResolvedValue({ ...base, eula: true, state: "running" });
    render(<HostTab inst={inst} />);
    await screen.findByText("Running");
    act(() => handlers.get("server-log")!({ instanceId: "pack", lines: ["Done (2.0s)! For help, type \"help\""] }));
    act(() => handlers.get("server-log")!({ instanceId: "other", lines: ["not ours"] }));
    expect(screen.getByText(/Done \(2\.0s\)/)).toBeTruthy();
    expect(screen.queryByText(/not ours/)).toBeNull();
    fireEvent.change(screen.getByLabelText("Server command"), { target: { value: "/op Steve" } });
    fireEvent.click(screen.getByText("Send"));
    await waitFor(() => expect(api.serverCommand).toHaveBeenCalledWith("pack", "/op Steve"));
  });

  it("only offers LAN addresses once network access is on", async () => {
    api.serverInfo.mockResolvedValue({ ...base, eula: true });
    render(<HostTab inst={inst} />);
    expect(await screen.findByText(/only listens on this computer/)).toBeTruthy();
    expect(screen.queryByText(/192\.168\.1\.20/)).toBeNull();
    expect(await screen.findByText("Link playit.gg")).toBeTruthy();
    expect(api.playitLink).not.toHaveBeenCalled();
  });
});
