import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type Loader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

export interface PackSource { projectId: string; versionId: string; title: string; versionNumber: string }

export interface LaunchProfile {
  id: string;
  name: string;
  jvmArgs: string;
  memoryMb?: number | null;
  javaPath?: string | null;
  disableShaders: boolean;
}

export interface Instance {
  id: string;
  name: string;
  mcVersion: string;
  loader: Loader;
  loaderVersion?: string | null;
  icon?: string | null;
  memoryMb?: number | null;
  jvmArgs: string;
  javaPath?: string | null;
  width?: number | null;
  height?: number | null;
  created: number;
  lastPlayed?: number | null;
  playSeconds: number;
  group?: string | null;
  pack?: PackSource | null;
  launchVersion?: string | null;
  launchProfiles?: LaunchProfile[];
  activeLaunchProfile?: string | null;
}

export interface InstanceView extends Instance { path: string; iconPath?: string | null; modCount: number; cover?: string | null }

export interface Settings {
  theme: "dark" | "light" | "system";
  defaultMemoryMb: number;
  defaultJvmArgs: string;
  defaultWidth: number;
  defaultHeight: number;
  javaOverrides: Record<string, string>;
  autoInstallJava: boolean;
  concurrentDownloads: number;
  hideOnLaunch: boolean;
  reduceMotion: boolean;
  isolateGameEnv: boolean;
  gameEnv: Record<string, string>;
  discordRpc: boolean;
  gamepadNav: boolean;
  telemetryEnabled: boolean;
}

export interface AppInfo { version: string; dataDir: string; clientId: string; clientIdSource: string; authFlow: string; os: string }

export type ContentKind = "mods" | "resourcepacks" | "shaderpacks" | "datapacks";
export interface ContentItem {
  fileName: string; kind: ContentKind; enabled: boolean; title: string; version?: string | null;
  authors?: string | null; description?: string | null; icon?: string | null; projectId?: string | null; size: number;
}
export interface ContentHit {
  instanceId: string; instanceName: string; kind: ContentKind; fileName: string; title: string; projectId?: string | null;
}
export interface UpdateInfo {
  kind: ContentKind; fileName: string; title: string; icon?: string | null; projectId: string;
  currentVersion: string; newVersion: string; newVersionId: string;
}
export interface World { folder: string; icon?: string | null; lastPlayed: number; size: number }
export interface ServerEntry { name: string; ip: string; icon?: string | null; acceptTextures?: number | null }
export interface FileEntry { name: string; path: string; modified: number; size: number }
export interface GameState { instanceId: string; state: "preparing" | "running" | "exited"; pid?: number | null; started: number; exitCode?: number | null }
export interface LogLine { instanceId: string; line: string; stream: string }
export interface TaskEvent { id: string; title: string; stage: string; done: number; total: number; state: "running" | "done" | "error"; error?: string | null }
export interface McVersion { id: string; type: string; url: string; releaseTime: string }
export interface LoaderVersion { version: string; stable: boolean }
export interface Account {
  id: string;
  kind: "microsoft" | "offline";
  username: string;
  uuid: string;
  skinUrl?: string | null;
  capeUrl?: string | null;
  skinModel?: string;
  added: number;
  active: boolean;
  needsSignIn: boolean;
}
export interface CapeOption { id: string; alias: string; url: string; active: boolean }
export interface SkinOption { id: string; url: string; model: string; active: boolean }
export interface AccountCosmetics { account: Account; capes: CapeOption[]; skins: SkinOption[] }
export interface JavaInstall { path: string; version: string; major: number; managed: boolean }

export interface SearchHit {
  project_id: string; slug: string; title: string; description: string; author: string; icon_url?: string | null;
  downloads: number; follows: number; project_type: string; categories: string[]; display_categories?: string[];
  versions: string[]; gallery?: string[]; featured_gallery?: string | null; date_modified: string; color?: number | null;
}
export interface SearchResult { hits: SearchHit[]; total_hits: number; offset: number; limit: number }

export interface ModrinthAppProfile {
  key: string;
  name: string;
  mcVersion: string;
  loader: string;
  loaderVersion?: string | null;
  path: string;
  iconPath?: string | null;
  sourceApp: string;
  created: number;
  lastPlayed?: number | null;
  playSeconds: number;
  memoryMb?: number | null;
  packProjectId?: string | null;
  packVersionId?: string | null;
}

export interface DepInfo {
  projectId?: string | null;
  versionId?: string | null;
  dependencyType: string;
  title?: string | null;
}

export interface LaunchOpts {
  world?: string | null;
  server?: string | null;
  profileId?: string | null;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),

  listInstances: () => invoke<InstanceView[]>("list_instances"),
  getInstance: (id: string) => invoke<InstanceView>("get_instance", { id }),
  createInstance: (inst: Partial<Instance>) => invoke<InstanceView>("create_instance", { inst: { jvmArgs: "", created: 0, playSeconds: 0, launchProfiles: [], ...inst } }),
  updateInstance: (inst: Instance) => invoke<InstanceView>("update_instance", { inst }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  duplicateInstance: (id: string) => invoke<InstanceView>("duplicate_instance", { id }),
  setInstanceIcon: (id: string, path: string | null) => invoke<InstanceView>("set_instance_icon", { id, path }),
  importMrpack: (path: string) => invoke<InstanceView>("import_mrpack", { path }),
  exportMrpack: (id: string, dest: string, extra: string[]) => invoke<string>("export_mrpack", { id, dest, extra }),
  listModrinthAppProfiles: () => invoke<ModrinthAppProfile[]>("list_modrinth_app_profiles"),
  importModrinthAppProfile: (key: string) => invoke<InstanceView>("import_modrinth_app_profile", { key }),
  backupInstance: (id: string, dest: string) => invoke<string>("backup_instance", { id, dest }),
  restoreInstanceBackup: (path: string) => invoke<InstanceView>("restore_instance_backup", { path }),
  searchLibraryContent: (query: string) => invoke<ContentHit[]>("search_library_content", { query }),

  listContent: (id: string, kind: ContentKind) => invoke<ContentItem[]>("list_content", { id, kind }),
  toggleContent: (id: string, kind: ContentKind, file: string) => invoke<string>("toggle_content", { id, kind, file }),
  removeContent: (id: string, kind: ContentKind, file: string) => invoke<void>("remove_content", { id, kind, file }),
  addContentFiles: (id: string, kind: ContentKind, files: string[]) => invoke<string[]>("add_content_files", { id, kind, files }),
  installedProjects: (id: string) => invoke<string[]>("installed_projects", { id }),
  checkUpdates: (id: string) => invoke<UpdateInfo[]>("check_updates", { id }),
  applyUpdates: (id: string, updates: UpdateInfo[]) => invoke<number>("apply_updates", { id, updates }),
  listWorlds: (id: string) => invoke<World[]>("list_worlds", { id }),
  listServers: (id: string) => invoke<ServerEntry[]>("list_servers", { id }),
  saveServers: (id: string, serversList: ServerEntry[]) => invoke<void>("save_servers", { id, serversList }),
  listScreenshots: (id: string) => invoke<FileEntry[]>("list_screenshots", { id }),
  listLogFiles: (id: string) => invoke<FileEntry[]>("list_log_files", { id }),
  readInstanceFile: (id: string, path: string) => invoke<string>("read_instance_file", { id, path }),

  launch: (id: string, opts?: LaunchOpts | string) => {
    const o: LaunchOpts = typeof opts === "string" ? { world: opts } : (opts ?? {});
    return invoke<void>("launch_instance", {
      id,
      world: o.world ?? null,
      server: o.server ?? null,
      profileId: o.profileId ?? null,
    });
  },
  kill: (id: string) => invoke<boolean>("kill_instance", { id }),
  gameStates: () => invoke<GameState[]>("game_states"),
  gameLog: (id: string) => invoke<string[]>("game_log", { id }),
  installInstance: (id: string) => invoke<void>("install_instance", { id }),

  mcVersions: () => invoke<McVersion[]>("mc_versions"),
  /** Minecraft Wiki artwork for a game version (cover when no screenshot / pack art). */
  mcVersionArt: (version: string) => invoke<string | null>("mc_version_art", { version }),
  loaderVersions: (loader: Loader, mc: string) => invoke<LoaderVersion[]>("loader_versions", { loader, mc }),

  search: (params: { query: string; facets: string[][]; index: string; offset: number; limit: number }) => invoke<SearchResult>("modrinth_search", { params }),
  project: (id: string) => invoke<any>("modrinth_project", { id }),
  versions: (id: string, loaders: string[], gameVersions: string[]) => invoke<any[]>("modrinth_versions", { id, loaders, gameVersions }),
  tags: () => invoke<{ categories: any[]; loaders: any[]; gameVersions: any[] }>("modrinth_tags"),
  install: (instanceId: string, projectId: string, versionId?: string | null, includeOptional = false) =>
    invoke<{ title: string; fileName: string }[]>("modrinth_install", { instanceId, projectId, versionId: versionId ?? null, includeOptional }),
  versionDeps: (versionId: string) => invoke<DepInfo[]>("modrinth_version_deps", { versionId }),
  installModpack: (projectId: string, versionId?: string | null) => invoke<InstanceView>("modrinth_install_modpack", { projectId, versionId: versionId ?? null }),

  listAccounts: () => invoke<Account[]>("list_accounts"),
  setActiveAccount: (id: string) => invoke<Account[]>("set_active_account", { id }),
  removeAccount: (id: string) => invoke<Account[]>("remove_account", { id }),
  addOfflineAccount: (name: string) => invoke<Account[]>("add_offline_account", { name }),
  accountCosmetics: (id: string) => invoke<AccountCosmetics>("account_cosmetics", { id }),
  setAccountCape: (id: string, capeId: string | null) => invoke<AccountCosmetics>("set_account_cape", { id, capeId }),
  setAccountSkin: (id: string, path: string, model: string) => invoke<AccountCosmetics>("set_account_skin", { id, path, model }),
  setAccountSkinFromHistory: (id: string, skinId: string) => invoke<AccountCosmetics>("set_account_skin_from_history", { id, skinId }),
  msaLogin: () => invoke<Account[] | null>("msa_login"),
  msaCancel: () => invoke<void>("msa_cancel"),

  listJava: () => invoke<JavaInstall[]>("list_java"),
  installJava: (major: number) => invoke<JavaInstall>("install_java", { major }),
  openDataPath: (path: string) => invoke<void>("open_data_path", { path }),

  checkForUpdate: async () => {
    try {
      return await check();
    } catch {
      return null;
    }
  },
  relaunch,
};

export function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => cb(e.payload));
}

export const fileSrc = (p?: string | null) => (p ? (p.startsWith("http") ? p : convertFileSrc(p)) : undefined);