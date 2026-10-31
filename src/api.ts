import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
  /** Launch Horizon (Rust client) instead of Java. Vanilla 26.3 only. */
  useHorizon?: boolean;
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
  discordShowInstance: boolean;
  discordAppId?: string;
  updateChannel: UpdateChannel;
  gamepadNav: boolean;
  telemetryEnabled: boolean;
  crashReportsEnabled: boolean;
  privacyPromptAnswered: boolean;
  curseforgeApiKey?: string;
}

export interface AppUpdate { version: string; currentVersion: string; notes: string | null; channel: string }
export type UpdateChannel = "stable" | "beta";
export interface AppInfo { version: string; dataDir: string; clientId: string; clientIdSource: string; authFlow: string; os: string }

export type ContentKind = "mods" | "resourcepacks" | "shaderpacks" | "datapacks";
export interface ContentItem {
  fileName: string; kind: ContentKind; enabled: boolean; title: string; version?: string | null;
  authors?: string | null; description?: string | null; icon?: string | null; projectId?: string | null; size: number;
}
export interface HorizonModItem {
  fileName: string;
  id: string;
  name: string;
  version?: string | null;
  description?: string | null;
  authors?: string | null;
  enabled: boolean;
  size: number;
  capabilities: string[];
}
export interface BundledHorizonMod {
  id: string;
  name: string;
  description: string;
  fileName: string;
}
export interface ContentHit {
  instanceId: string; instanceName: string; kind: ContentKind; fileName: string; title: string; projectId?: string | null;
}
export interface UpdateInfo {
  kind: ContentKind; fileName: string; title: string; icon?: string | null; projectId: string;
  currentVersion: string; newVersion: string; newVersionId: string;
  source: ContentSource;
}

export type ContentSource = "modrinth" | "curseforge";

export interface InstallRoot { projectId: string; versionId?: string | null; soft?: boolean }
export interface MemoryPreset { id: string; label: string; mb: number }
export interface JvmPreset { id: string; label: string; detail: string; args: string }
export interface PerformanceAdvice {
  memory: { totalMb: number; maxSafeMb: number; recommendedMb: number; presets: MemoryPreset[] };
  jvm: JvmPreset[];
  currentJvm: string | null;
  pack: InstallRoot[];
}
export interface PlanCandidate {
  source: ContentSource;
  project: { id: string; slug: string; title: string; projectType: string; icon?: string | null; pageUrl?: string | null };
  versionId: string;
  versionNumber: string;
  fileName: string;
  url?: string | null;
  downloadPage?: string | null;
  size?: number | null;
}
export type PlanReason = { type: "requested" } | { type: "update"; from: string } | { type: "dependencyOf"; title: string };
export interface PlanItem { candidate: PlanCandidate; reason: PlanReason; replaces: string[]; optional: boolean }
export interface PlanNote { title: string; message: string }
export interface InstallPlan { items: PlanItem[]; skipped: PlanNote[]; problems: PlanNote[] }
export interface PendingDownload {
  projectId: string; fileId: string; title: string; versionNumber: string; fileName: string; folder: string;
  downloadPage: string; size?: number | null; icon?: string | null; replaces: string[];
}
export interface InstallResult { installed: { title: string; fileName: string }[]; pending: PendingDownload[] }
export interface PendingView { items: PendingDownload[]; watching: boolean; downloadsDir?: string | null }

export type Fix =
  | { type: "installMod"; modId: string; projectId?: string | null; source?: string | null }
  | { type: "enableMod"; file: string }
  | { type: "disableMod"; file: string }
  | { type: "setMemory"; mb: number }
  | { type: "useManagedJava" }
  | { type: "searchMod"; query: string }
  | { type: "manualDownloads" };
export type FixAction = Fix & { label: string };
export interface CompatIssue { key: string; severity: "error" | "warning"; title: string; detail: string; files: string[]; fixes: FixAction[] }
export interface CompatReport { issues: CompatIssue[]; pending: PendingDownload[]; mods: number }
export interface ManualPickup { instanceId: string; titles: string[]; remaining: number }
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
  /** Set on CurseForge hits only. */
  source?: ContentSource; page_url?: string | null; distribution?: boolean;
}
export interface CurseForgeSearch { query: string; projectType: string; gameVersion: string; loader: string; sort: string; offset: number; limit: number }
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
  /** Modrinth `.mrpack` or CurseForge modpack `.zip` (format sniffed). */
  importModpack: (path: string) => invoke<InstanceView>("import_modpack", { path }),
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

  listHorizonMods: (id: string) => invoke<HorizonModItem[]>("list_horizon_mods", { id }),
  toggleHorizonMod: (id: string, modId: string) => invoke<boolean>("toggle_horizon_mod", { id, modId }),
  removeHorizonMod: (id: string, file: string) => invoke<void>("remove_horizon_mod", { id, file }),
  addHorizonModFiles: (id: string, files: string[]) => invoke<HorizonModItem[]>("add_horizon_mod_files", { id, files }),
  listBundledHorizonMods: () => invoke<BundledHorizonMod[]>("list_bundled_horizon_mods"),
  installBundledHorizonMod: (id: string, sampleId: string) => invoke<HorizonModItem>("install_bundled_horizon_mod", { id, sampleId }),
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
  /** Uploads to mclo.gs; returns the public link. */
  shareLog: (content: string) => invoke<string>("share_log", { content }),
  createShortcut: (id: string) => invoke<string>("create_shortcut", { id }),
  takeLaunchRequest: () => invoke<string | null>("take_launch_request"),
  installInstance: (id: string) => invoke<void>("install_instance", { id }),

  mcVersions: () => invoke<McVersion[]>("mc_versions"),
  /** Minecraft Wiki artwork for a game version (cover when no screenshot / pack art). */
  mcVersionArt: (version: string) => invoke<string | null>("mc_version_art", { version }),
  loaderVersions: (loader: Loader, mc: string) => invoke<LoaderVersion[]>("loader_versions", { loader, mc }),

  search: (params: { query: string; facets: string[][]; index: string; offset: number; limit: number }) => invoke<SearchResult>("modrinth_search", { params }),
  project: (id: string) => invoke<any>("modrinth_project", { id }),
  versions: (id: string, loaders: string[], gameVersions: string[]) => invoke<any[]>("modrinth_versions", { id, loaders, gameVersions }),
  tags: () => invoke<{ categories: any[]; loaders: any[]; gameVersions: any[] }>("modrinth_tags"),
  versionDeps: (versionId: string) => invoke<DepInfo[]>("modrinth_version_deps", { versionId }),
  installModpack: (projectId: string, versionId?: string | null) => invoke<InstanceView>("modrinth_install_modpack", { projectId, versionId: versionId ?? null }),

  /** Resolves required dependencies and conflicts without touching the instance. */
  planInstall: (instanceId: string, source: ContentSource, roots: InstallRoot[], includeOptional = false) =>
    invoke<InstallPlan>("plan_install", { instanceId, source, roots, includeOptional }),
  installContent: (instanceId: string, source: ContentSource, roots: InstallRoot[], includeOptional = false, allowProblems = false) =>
    invoke<InstallResult>("install_content", { instanceId, source, roots, includeOptional, allowProblems }),
  performanceAdvice: (id: string) => invoke<PerformanceAdvice>("performance_advice", { id }),
  checkCompat: (id: string) => invoke<CompatReport>("check_compat", { id }),
  applyFix: (id: string, fix: Fix) => invoke<string>("apply_fix", { id, fix }),
  pendingDownloads: (id: string) => invoke<PendingView>("pending_downloads", { id }),
  openManualDownload: (id: string, fileId?: string | null) => invoke<void>("open_manual_download", { id, fileId: fileId ?? null }),
  importManualDownload: (id: string, path: string) => invoke<string>("import_manual_download", { id, path }),
  dismissManualDownload: (id: string, fileId: string) => invoke<void>("dismiss_manual_download", { id, fileId }),

  curseforgeAvailable: () => invoke<boolean>("curseforge_available"),
  curseforgeSearch: (params: CurseForgeSearch) => invoke<SearchResult>("curseforge_search", { params }),
  curseforgeProject: (id: string) => invoke<any>("curseforge_project", { id }),
  curseforgeFiles: (id: string, gameVersion?: string | null, loader?: string | null) =>
    invoke<any[]>("curseforge_files", { id, gameVersion: gameVersion ?? null, loader: loader ?? null }),
  curseforgeInstallModpack: (projectId: string, fileId?: string | null) =>
    invoke<InstanceView>("curseforge_install_modpack", { projectId, fileId: fileId ?? null }),

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

  /** Checks the channel chosen in Settings. Null when up to date or the check failed. */
  checkForUpdate: async (): Promise<(AppUpdate & { downloadAndInstall: () => Promise<void> }) | null> => {
    try {
      const u = await invoke<AppUpdate | null>("app_update_check");
      return u && { ...u, downloadAndInstall: () => invoke<void>("app_update_install") };
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