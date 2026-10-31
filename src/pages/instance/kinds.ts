import type { ContentKind } from "../../api";

export const KIND_LABEL: Record<ContentKind, string> = { mods: "Mods", resourcepacks: "Resource packs", shaderpacks: "Shaders", datapacks: "Datapacks" };
export const KIND_TYPE: Record<ContentKind, string> = { mods: "mod", resourcepacks: "resourcepack", shaderpacks: "shader", datapacks: "datapack" };
export const KIND_FOLDER: Record<ContentKind, string> = { mods: "mods", resourcepacks: "resourcepacks", shaderpacks: "shaderpacks", datapacks: "datapacks" };
