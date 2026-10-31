import { describe, expect, it } from "vitest";
import type { CompatReport, InstallPlan, PlanItem } from "../api";
import { blocksLaunch, installedMessage, isManual, runsInBackend, summarizePlan } from "./install";

const item = (title: string, reason: PlanItem["reason"], extra: Partial<PlanItem["candidate"]> = {}, replaces: string[] = []): PlanItem => ({
  candidate: {
    source: "modrinth",
    project: { id: title, slug: title, title, projectType: "mod" },
    versionId: "v",
    versionNumber: "1.0",
    fileName: `${title}.jar`,
    url: "https://cdn.modrinth.com/x.jar",
    ...extra,
  },
  reason,
  replaces,
  optional: false,
});

describe("summarizePlan", () => {
  it("installs a lone mod without review", () => {
    const plan: InstallPlan = { items: [item("sodium", { type: "requested" })], skipped: [], problems: [] };
    expect(summarizePlan(plan).needsReview).toBe(false);
  });

  it("asks for review when dependencies, updates or manual files come along", () => {
    const deps: InstallPlan = { items: [item("a", { type: "requested" }), item("api", { type: "dependencyOf", title: "a" })], skipped: [], problems: [] };
    expect(summarizePlan(deps).dependencies).toHaveLength(1);
    expect(summarizePlan(deps).needsReview).toBe(true);

    const upd: InstallPlan = { items: [item("a", { type: "requested" }, {}, ["a-0.9.jar"])], skipped: [], problems: [] };
    expect(summarizePlan(upd).needsReview).toBe(true);

    const manual: InstallPlan = { items: [item("a", { type: "requested" }, { url: null, downloadPage: "https://www.curseforge.com/x" })], skipped: [], problems: [] };
    expect(summarizePlan(manual).manual).toHaveLength(1);
    expect(summarizePlan(manual).needsReview).toBe(true);
  });

  it("asks for review when there are problems or nothing to do", () => {
    expect(summarizePlan({ items: [], skipped: [], problems: [] }).needsReview).toBe(true);
    expect(summarizePlan({ items: [item("a", { type: "requested" })], skipped: [], problems: [{ title: "a", message: "conflicts" }] }).needsReview).toBe(true);
  });
});

describe("isManual", () => {
  it("needs both a missing url and a page", () => {
    expect(isManual(item("a", { type: "requested" }, { url: null, downloadPage: "https://www.curseforge.com/x" }))).toBe(true);
    expect(isManual(item("a", { type: "requested" }, { url: null, downloadPage: null }))).toBe(false);
    expect(isManual(item("a", { type: "requested" }))).toBe(false);
  });
});

describe("installedMessage", () => {
  it("names the first file and counts the rest", () => {
    expect(installedMessage([{ title: "Sodium" }], 0, "Pack")).toBe("Installed Sodium into Pack");
    expect(installedMessage([{ title: "Create" }, { title: "Flywheel" }], 1, "Pack")).toBe("Installed Create and 1 more into Pack. 1 file needs a manual download");
    expect(installedMessage([], 2, "Pack")).toBe("2 files need a manual download");
  });
});

describe("launch checks", () => {
  const report = (severity: "error" | "warning", pending = 0): CompatReport => ({
    issues: [{ key: "k", severity, title: "t", detail: "d", files: [], fixes: [] }],
    pending: Array.from({ length: pending }, (_, i) => ({ projectId: "1", fileId: String(i), title: "x", versionNumber: "1", fileName: "x.jar", folder: "mods", downloadPage: "", replaces: [] })),
    mods: 3,
  });
  it("blocks on errors and pending downloads only", () => {
    expect(blocksLaunch(report("warning"))).toBe(false);
    expect(blocksLaunch(report("error"))).toBe(true);
    expect(blocksLaunch(report("warning", 1))).toBe(true);
  });
  it("routes search and manual download fixes to the UI", () => {
    expect(runsInBackend({ type: "searchMod", query: "x" })).toBe(false);
    expect(runsInBackend({ type: "manualDownloads" })).toBe(false);
    expect(runsInBackend({ type: "disableMod", file: "a.jar" })).toBe(true);
  });
});
