// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { DiscordEvent, JoinPlan } from "../api";

let emit: (ev: DiscordEvent) => void = () => {};
let pending: string | null = null;
const joinPlan = vi.fn<(from: { secret?: string; code?: string }) => Promise<JoinPlan>>();
const discordReply = vi.fn(async () => []);
const installModpack = vi.fn(async () => ({ id: "pack" }));
const play = vi.fn(async () => {});
const go = vi.fn();
const toast = vi.fn();

vi.mock("../api", () => ({
  api: {
    takeDiscordJoin: async () => { const s = pending; pending = null; return s; },
    joinPlan: (from: { secret?: string }) => joinPlan(from),
    discordReply: (...a: unknown[]) => discordReply(...(a as [])),
    installModpack: (...a: unknown[]) => installModpack(...(a as [])),
    joinRecord: async () => [],
  },
  on: (_: string, cb: (ev: DiscordEvent) => void) => { emit = cb; return Promise.resolve(() => {}); },
}));
vi.mock("../store", () => ({ useStore: () => ({ play, go, toast, refreshInstances: vi.fn(async () => {}) }) }));

const { DiscordJoin } = await import("./DiscordJoin");

const invite = { address: "fun-cat.joinmc.link", mc: "1.21.1", loader: "fabric", lv: null, fp: "abc", pack: null };

describe("DiscordJoin", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); pending = null; });

  it("goes straight in when a Join click matches an instance, even one from before the window opened", async () => {
    pending = "c1;fun-cat.joinmc.link;1.21.1;fabric;;abc;";
    joinPlan.mockResolvedValue({ invite, step: { kind: "launch", instance: { id: "mine", name: "Mine" } } });
    render(<DiscordJoin />);
    await waitFor(() => expect(play).toHaveBeenCalledWith("mine", { server: "fun-cat.joinmc.link" }));
    expect(joinPlan).toHaveBeenCalledWith({ secret: "c1;fun-cat.joinmc.link;1.21.1;fabric;;abc;" });
    expect(go).toHaveBeenCalledWith({ name: "instance", id: "mine" });
  });

  it("asks before downloading a modpack", async () => {
    joinPlan.mockResolvedValue({ invite, step: { kind: "installPack", pack: { source: "m", project: "p", version: "v" } } });
    render(<DiscordJoin />);
    pending = "secret";
    emit({ kind: "join", secret: "secret" });
    const btn = await screen.findByText("Set up and join");
    expect(installModpack).not.toHaveBeenCalled();
    fireEvent.click(btn);
    await waitFor(() => expect(play).toHaveBeenCalledWith("pack", { server: "fun-cat.joinmc.link" }));
  });

  it("acts on a join once even though it is both queued and sent", async () => {
    joinPlan.mockResolvedValue({ invite, step: { kind: "launch", instance: { id: "mine", name: "Mine" } } });
    render(<DiscordJoin />);
    await waitFor(() => expect(joinPlan).not.toHaveBeenCalled());
    pending = "s";
    emit({ kind: "join", secret: "s" });
    emit({ kind: "join", secret: "s" });
    await waitFor(() => expect(play).toHaveBeenCalledTimes(1));
  });

  it("shows ask-to-join requests with Accept and Decline", async () => {
    render(<DiscordJoin />);
    emit({ kind: "joinRequest", userId: "42", username: "steve", avatarUrl: null });
    emit({ kind: "joinRequest", userId: "43", username: "alex", avatarUrl: null });
    expect(await screen.findByText("steve")).toBeTruthy();
    fireEvent.click(screen.getAllByText("Accept")[0]);
    await waitFor(() => expect(discordReply).toHaveBeenCalledWith("42", true, "steve", null));
    expect(screen.queryByText("steve")).toBeNull();
    fireEvent.click(screen.getByText("Decline"));
    await waitFor(() => expect(discordReply).toHaveBeenCalledWith("43", false, "alex", null));
    expect(toast).toHaveBeenCalledWith("steve can join now");
  });
});
