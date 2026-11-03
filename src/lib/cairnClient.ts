import type { Loader } from "../api";

/** Game versions with a bundled Cairn Client jar; keep in sync with src-tauri/src/cairn_client.rs. */
export const CAIRN_CLIENT_BUILDS: Partial<Record<Loader, string[]>> = {
  forge: ["1.8", "1.8.8", "1.8.9"],
  fabric: ["1.16.5", "1.20.1", "1.20.4", "1.21.1", "1.21.11", "26.3"],
};

export const CAIRN_CLIENT_SUMMARY = "Forge 1.8 / 1.8.8 / 1.8.9 and Fabric 1.16.5 / 1.20.1 / 1.20.4 / 1.21.1 / 1.21.11 / 26.3";

export function cairnClientSupported(loader: Loader, mcVersion: string, useHorizon = false): boolean {
  return !useHorizon && (CAIRN_CLIENT_BUILDS[loader]?.includes(mcVersion) ?? false);
}
