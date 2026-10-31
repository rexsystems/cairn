import type { CompatReport, Fix, InstallPlan, PlanItem } from "../api";

/** CurseForge files whose authors turned off third-party downloads have no direct URL. */
export const isManual = (item: PlanItem) => !item.candidate.url && !!item.candidate.downloadPage;

export interface PlanSummary {
  requested: PlanItem[];
  dependencies: PlanItem[];
  updates: PlanItem[];
  manual: PlanItem[];
  /** True when the plan does more than the user clicked, so it is worth showing first. */
  needsReview: boolean;
}

export function summarizePlan(plan: InstallPlan): PlanSummary {
  const requested = plan.items.filter((i) => i.reason.type === "requested");
  const dependencies = plan.items.filter((i) => i.reason.type === "dependencyOf");
  const updates = plan.items.filter((i) => i.reason.type === "update" || i.replaces.length > 0);
  const manual = plan.items.filter(isManual);
  const needsReview = plan.problems.length > 0 || dependencies.length > 0 || updates.length > 0 || manual.length > 0 || plan.items.length === 0;
  return { requested, dependencies, updates, manual, needsReview };
}

export function installedMessage(installed: { title: string }[], pending: number, target: string): string {
  const parts: string[] = [];
  if (installed.length === 1) parts.push(`Installed ${installed[0].title} into ${target}`);
  else if (installed.length > 1) parts.push(`Installed ${installed[0].title} and ${installed.length - 1} more into ${target}`);
  if (pending > 0) parts.push(`${pending} ${pending === 1 ? "file needs" : "files need"} a manual download`);
  return parts.join(". ") || "Nothing to install";
}

/** Blocking problems stop a launch until the user fixes them or chooses to launch anyway. */
export const blocksLaunch = (r: CompatReport) => r.pending.length > 0 || r.issues.some((i) => i.severity === "error");

/** Fixes the backend applies by itself; the rest need the UI (a search page or the downloads dialog). */
export const runsInBackend = (f: Fix) => f.type !== "searchMod" && f.type !== "manualDownloads";
