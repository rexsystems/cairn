export function compactNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}k`;
  return String(n);
}

export function bytes(n: number): string {
  if (n >= 1 << 30) return `${(n / (1 << 30)).toFixed(1)} GB`;
  if (n >= 1 << 20) return `${(n / (1 << 20)).toFixed(1)} MB`;
  if (n >= 1 << 10) return `${Math.round(n / (1 << 10))} KB`;
  return `${n} B`;
}

export function playTime(seconds: number): string {
  if (seconds < 60) return seconds > 0 ? "under a minute" : "not played yet";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h === 0) return `${m} min`;
  return m ? `${h} h ${m} min` : `${h} h`;
}

export function relativeTime(ms: number | null | undefined, now = Date.now()): string {
  if (!ms) return "never";
  const s = Math.round((now - ms) / 1000);
  if (s < 45) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  const d = Math.round(h / 24);
  if (d === 1) return "yesterday";
  if (d < 30) return `${d} days ago`;
  return new Date(ms).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export const loaderLabel: Record<string, string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};

/** Modrinth search facets for an instance-scoped search. */
export function buildFacets(opts: { projectType: string; gameVersion?: string; loader?: string; categories?: string[] }): string[][] {
  const f: string[][] = [[`project_type:${opts.projectType}`]];
  if (opts.gameVersion) f.push([`versions:${opts.gameVersion}`]);
  if (opts.loader && opts.projectType === "mod") {
    f.push(opts.loader === "quilt" ? ["categories:quilt", "categories:fabric"] : [`categories:${opts.loader}`]);
  }
  // resourcepack / datapack / shader use game version only (loader filter is minecraft-side).
  for (const c of opts.categories ?? []) f.push([`categories:${c}`]);
  return f;
}

export function flowLabel(flow: string): string {
  if (flow === "live") return "Device code on login.live.com, then Xbox Live and Minecraft services";
  if (flow === "msal") return "Device code on the Microsoft identity platform, then Xbox Live and Minecraft services";
  return flow;
}
