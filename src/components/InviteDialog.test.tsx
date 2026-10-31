// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { DiscordStatus, InviteTarget } from "../api";

const inviteTargets = vi.fn<(id: string) => Promise<InviteTarget[]>>();
const inviteEnable = vi.fn(async () => ({ code: "CAIRN1:abc@fun-cat.joinmc.link", notes: [] as string[] }));
const inviteDisable = vi.fn(async () => {});
let status: DiscordStatus = { enabled: true, connected: true, joinable: [] };

vi.mock("../api", () => ({
  api: {
    inviteTargets: (id: string) => inviteTargets(id),
    inviteEnable: (...a: unknown[]) => inviteEnable(...(a as [])),
    inviteDisable: (...a: unknown[]) => inviteDisable(...(a as [])),
    discordStatus: async () => status,
  },
}));
vi.mock("../store", () => ({ useStore: () => ({ toast: vi.fn(), settings: { discordRpc: true } }) }));

const { InviteDialog } = await import("./InviteDialog");

const targets: InviteTarget[] = [
  { kind: "playit", label: "playit.gg tunnel \"mc\"", address: "fun-cat.joinmc.link", max: 10 },
  { kind: "lan", label: "Your local network", address: "192.168.1.5:25565", max: 10 },
];

describe("InviteDialog", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); status = { enabled: true, connected: true, joinable: [] }; });

  it("builds the invite code without touching Discord until the host opts in", async () => {
    inviteTargets.mockResolvedValue(targets);
    render(<InviteDialog id="a" name="A" onClose={() => {}} />);
    await waitFor(() => expect(inviteEnable).toHaveBeenCalledWith("a", "fun-cat.joinmc.link", 10, false));
    fireEvent.click(await screen.findByRole("switch"));
    await waitFor(() => expect(inviteEnable).toHaveBeenLastCalledWith("a", "fun-cat.joinmc.link", 10, true));
    fireEvent.click(screen.getByText("192.168.1.5:25565"));
    await waitFor(() => expect(inviteEnable).toHaveBeenLastCalledWith("a", "192.168.1.5:25565", 10, true));
    fireEvent.click(screen.getByRole("switch"));
    await waitFor(() => expect(inviteDisable).toHaveBeenCalledWith("a"));
  });

  it("explains what to start when there is nothing to invite to", async () => {
    inviteTargets.mockResolvedValue([]);
    render(<InviteDialog id="a" name="A" onClose={() => {}} />);
    expect(await screen.findByText(/Nothing to invite to yet/)).toBeTruthy();
    expect(inviteEnable).not.toHaveBeenCalled();
  });

  it("keeps the Discord switch off while Discord isn't running", async () => {
    status = { enabled: true, connected: false, joinable: [] };
    inviteTargets.mockResolvedValue(targets);
    render(<InviteDialog id="a" name="A" onClose={() => {}} />);
    expect(await screen.findByText(/Open the Discord desktop app/)).toBeTruthy();
    expect((screen.getByRole("switch") as HTMLInputElement).disabled).toBe(true);
  });
});
