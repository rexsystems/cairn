// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { CairnFriend, CairnFriendList, SocialSettings } from "../api";

const settings: SocialSettings = { share: "full", appearOffline: false, requests: "everyone", dnd: false };
const bob: CairnFriend = { uuid: "b".repeat(32), name: "Bob", since: 1, status: "online", activity: { version: "1.21.11", mode: "multiplayer", server: "mc.example.net" } };
const zed: CairnFriend = { uuid: "c".repeat(32), name: "Zed", since: 1, status: "offline", activity: null };
const list = (): CairnFriendList => ({ friends: [bob, zed], incoming: [{ uuid: "f".repeat(32), name: "Finn" }], outgoing: [], blocked: [], settings });
const cairnFriends = vi.fn(async () => list());
const cairnFriendAction = vi.fn(async (..._a: unknown[]) => ({ ...list(), reply: { status: "sent", name: "Alex" } }));
const cairnSocialSettings = vi.fn(async (patch: Partial<SocialSettings>) => ({ ...settings, ...patch }));
const toast = vi.fn();

vi.mock("../api", () => ({ api: { cairnFriends: () => cairnFriends(), cairnFriendAction: (...a: unknown[]) => cairnFriendAction(...a), cairnSocialSettings: (p: Partial<SocialSettings>) => cairnSocialSettings(p) } }));
vi.mock("../store", () => ({ useStore: () => ({ toast }) }));

const { CairnFriends, describeActivity } = await import("./CairnFriends");

describe("CairnFriends", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); });

  it("describes what friends are doing from the mod's presence only", () => {
    expect(describeActivity(bob)).toBe("Multiplayer 1.21.11 on mc.example.net");
    expect(describeActivity({ ...bob, status: "dnd", activity: { ...bob.activity!, server: null } })).toBe("Do not disturb · Multiplayer 1.21.11");
    expect(describeActivity({ ...bob, activity: null })).toBe("Online in Cairn Client");
    expect(describeActivity(zed)).toBe("Offline");
  });

  it("lists friends and requests and sends actions", async () => {
    render(<CairnFriends />);
    expect(await screen.findByText("Bob")).toBeTruthy();
    expect(screen.getByText("Multiplayer 1.21.11 on mc.example.net")).toBeTruthy();
    expect(screen.getByText("Offline")).toBeTruthy();
    fireEvent.click(screen.getByText("Accept"));
    await waitFor(() => expect(cairnFriendAction).toHaveBeenCalledWith("accept", "f".repeat(32)));
    fireEvent.change(screen.getByLabelText("Minecraft name or UUID"), { target: { value: " Alex " } });
    fireEvent.click(screen.getByText("Add friend"));
    await waitFor(() => expect(cairnFriendAction).toHaveBeenCalledWith("add", "Alex"));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("Friend request sent to Alex."));
  });

  it("syncs privacy and DND to the server", async () => {
    render(<CairnFriends />);
    fireEvent.click(await screen.findByLabelText("Appear offline"));
    await waitFor(() => expect(cairnSocialSettings).toHaveBeenCalledWith({ appearOffline: true }));
    fireEvent.click(screen.getByLabelText("Do not disturb (mutes friend toasts in game)"));
    await waitFor(() => expect(cairnSocialSettings).toHaveBeenCalledWith({ dnd: true }));
    fireEvent.click(screen.getByText("Game only"));
    await waitFor(() => expect(cairnSocialSettings).toHaveBeenCalledWith({ share: "game" }));
  });

  it("explains when there is no Cairn server", async () => {
    cairnFriends.mockRejectedValueOnce("Set a cosmetics server in Settings first.");
    render(<CairnFriends />);
    expect(await screen.findByText("Set a cosmetics server in Settings first.")).toBeTruthy();
  });
});
