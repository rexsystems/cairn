// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

vi.mock("../api", () => ({ api: { cosmeticsOverview: async () => ({
  url: "http://127.0.0.1:8787", stats: { online: 1, peak: 1, total: 1 },
  me: { uuid: "a".repeat(32), name: "Alex", role: "player", equipped: { cape: "ember", badge: "classic" },
    unlocked: { capes: ["ember"], badges: ["classic"] }, customCape: null, moderation: "approval" },
}) } }));
vi.mock("../store", () => ({ useStore: () => ({ accounts: [{ id: "a", active: true }], toast: vi.fn(), go: vi.fn() }) }));
const { Cosmetics } = await import("./Cosmetics");
afterEach(cleanup);

it("previews the catalog and directs equipping to Minecraft", async () => {
  render(<Cosmetics />);
  expect(await screen.findByText(/Choose your cape and badge in Minecraft/)).toBeTruthy();
  expect(screen.getByText("Capes")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /No cape|Ember|No badge|Gold|Upload and wear/ })).toBeNull();
  expect(screen.getByRole("button", { name: "Upload cape" })).toBeTruthy();
  expect(screen.getAllByText("Equipped")).toHaveLength(2);
});
