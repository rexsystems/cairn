// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { InstanceView } from "../../api";
const api = {
  performanceAdvice: vi.fn(async () => null),
  mcVersions: vi.fn(async () => []),
  loaderVersions: vi.fn(async () => []),
  updateInstance: vi.fn(async (inst: InstanceView) => inst),
};
vi.mock("../../api", () => ({ api }));
vi.mock("../../store", () => ({ useStore: () => ({ toast: vi.fn(), settings: {}, requestInstall: vi.fn() }) }));
const { OptionsTab } = await import("./OptionsTab");
const inst = { id: "test", name: "Existing", loader: "fabric", loaderVersion: "0.20.1", mcVersion: "26.3", jvmArgs: "", created: 0, playSeconds: 0 } as InstanceView;
afterEach(() => { cleanup(); vi.clearAllMocks(); });

it("saves the single Cairn toggle without changing the existing instance settings", async () => {
  const saved = vi.fn();
  render(<OptionsTab inst={inst} onSaved={saved} />);
  fireEvent.click(screen.getByRole("switch", { name: "Cairn Client" }));
  fireEvent.click(screen.getByText("Save options"));
  await waitFor(() => expect(api.updateInstance).toHaveBeenCalledWith({ ...inst, useCairn: true }));
  expect(saved).toHaveBeenCalledWith({ ...inst, useCairn: true });
});
it("allows turning Cairn off on an existing instance", async () => {
  render(<OptionsTab inst={{ ...inst, useCairn: true }} onSaved={vi.fn()} />);
  fireEvent.click(screen.getByRole("switch", { name: "Cairn Client" }));
  fireEvent.click(screen.getByText("Save options"));
  await waitFor(() => expect(api.updateInstance).toHaveBeenCalledWith({ ...inst, useCairn: false }));
});
it.each(["1.8", "1.8.8", "1.8.9"])("offers Cairn for Forge %s", (mcVersion) => {
  render(<OptionsTab inst={{ ...inst, loader: "forge", mcVersion }} onSaved={vi.fn()} />);
  expect((screen.getByRole("switch", { name: "Cairn Client" }) as HTMLInputElement).disabled).toBe(false);
});
it("does not offer an incompatible build, but allows disabling an old selection", () => {
  render(<OptionsTab inst={{ ...inst, mcVersion: "1.19.4" }} onSaved={vi.fn()} />);
  expect((screen.getByRole("switch", { name: "Cairn Client" }) as HTMLInputElement).disabled).toBe(true);
  cleanup();
  render(<OptionsTab inst={{ ...inst, mcVersion: "1.19.4", useCairn: true }} onSaved={vi.fn()} />);
  expect((screen.getByRole("switch", { name: "Cairn Client" }) as HTMLInputElement).disabled).toBe(false);
});
