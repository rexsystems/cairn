// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ShareManifest, SharedImport } from "../api";

const readShareCode = vi.fn<(code: string) => Promise<ShareManifest>>();
const importShareCode = vi.fn<(code: string) => Promise<SharedImport>>();
const toast = vi.fn();
const go = vi.fn();

vi.mock("../api", () => ({ api: { readShareCode: (c: string) => readShareCode(c), importShareCode: (c: string) => importShareCode(c) } }));
vi.mock("../store", () => ({ useStore: () => ({ toast, go, refreshInstances: vi.fn(async () => {}) }) }));

const { PasteShareCode } = await import("./PasteShareCode");

const manifest: ShareManifest = { name: "Friends", mc: "1.21.1", loader: "fabric", lv: "0.16.9", items: [{ s: "m", p: "a", v: "1" }, { s: "c", p: "2", v: "3" }] };

describe("PasteShareCode", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); });

  it("previews the code, then creates the instance", async () => {
    readShareCode.mockResolvedValue(manifest);
    importShareCode.mockResolvedValue({ instance: { id: "friends", name: "Friends" } as SharedImport["instance"], installed: 2, pending: [], failed: [] });
    const onClose = vi.fn();
    render(<PasteShareCode onClose={onClose} />);
    const create = screen.getByText("Create instance").closest("button")!;
    expect(create.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Share code"), { target: { value: "CAIRN1:abc" } });
    expect(await screen.findByText("Friends")).toBeTruthy();
    expect(screen.getByText(/1 from CurseForge/)).toBeTruthy();
    fireEvent.click(create);
    await waitFor(() => expect(go).toHaveBeenCalledWith({ name: "instance", id: "friends" }));
    expect(toast).toHaveBeenCalledWith("Created Friends with 2 files", false);
    expect(onClose).toHaveBeenCalled();
  });

  it("explains a bad code and keeps Create off", async () => {
    readShareCode.mockRejectedValue("This is not a Cairn share code. They start with CAIRN1:");
    render(<PasteShareCode onClose={() => {}} />);
    fireEvent.change(screen.getByLabelText("Share code"), { target: { value: "hello" } });
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("Create instance").closest("button")!.disabled).toBe(true);
  });
});
