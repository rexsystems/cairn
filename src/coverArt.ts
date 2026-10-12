import { api, fileSrc, type InstanceView } from "./api";

/** Screenshot → Modrinth pack gallery → Minecraft Wiki art for that version → CSS tone. */
export async function resolveCover(inst: InstanceView): Promise<string | null> {
  if (inst.cover) {
    const src = fileSrc(inst.cover);
    if (src) return src;
  }
  if (inst.pack?.projectId) {
    try {
      const p = await api.project(inst.pack.projectId);
      const g = p.gallery?.find((x: { featured?: boolean; url?: string }) => x.featured) ?? p.gallery?.[0];
      if (g && typeof g === "object" && g.url) return g.url;
      if (typeof g === "string") return g;
    } catch { /* fall through */ }
  }
  if (inst.mcVersion) {
    try {
      const art = await api.mcVersionArt(inst.mcVersion);
      if (art) return art;
    } catch { /* fall through */ }
  }
  return null;
}
