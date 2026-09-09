import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AppEntry = {
  id: string;
  name: string;
  publisher?: string | null;
  version?: string | null;
  size_bytes?: number | null;
  size_display?: string | null;
  install_date?: string | null;
  install_location?: string | null;
  source_label: string;
  icon_path?: string | null;
  icon_color?: string | null;
  /** False for entries with no real removal path — never shown. Defaults true for older backends. */
  can_uninstall?: boolean;
  /** True when a real extracted icon PNG is already cached. */
  has_icon?: boolean;
};

export type AppDetails = {
  id: string;
  name: string;
  publisher?: string | null;
  version?: string | null;
  size_bytes?: number | null;
  size_display?: string | null;
  install_date?: string | null;
  install_location?: string | null;
  uninstall_string?: string | null;
  quiet_uninstall_string?: string | null;
  source_label: string;
  is_system: boolean;
  registry_keys: string[];
  metadata: Record<string, string>;
  icon_path?: string | null;
  icon_color?: string | null;
};

export type UninstallPayload = { ids: string[]; force: boolean; silent?: boolean };

export type UninstallProgressEvent = {
  seq: number;
  current: number;
  total: number;
  app_name: string;
  status: "processing" | "done" | "error" | "completed";
  log: string;
};

export type UninstallResultDto = {
  id: string;
  name: string;
  success: boolean;
  error?: string | null;
};

export type SystemStatsDto = {
  cpu: number;
  ram_used: number;
  ram_total: number;
  ram_pct: number;
  swap_used: number;
  swap_total: number;
  disks: { label: string; used: number; total: number; pct: number }[];
  gpu?: { name: string; usage: number; vram_used: number; vram_total: number } | null;
  battery?: { percent: number; charging: boolean } | null;
  uptime_secs: number;
  process_count: number;
};

export type LeftoverDto = {
  id: string;
  artifact_type: string;
  path: string;
  size_bytes?: number | null;
  size_display?: string | null;
  confidence: number;
  safety: string;
  description?: string | null;
};

export type AppResourceDto = {
  is_running: boolean;
  pid?: number | null;
  process_count: number;
  cpu: number;
  memory_bytes: number;
  memory_display?: string | null;
  gpu: number;
  vram_bytes: number;
  exe_path?: string | null;
};

export type VideoEntryDto = {
  id: string;
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  size_display: string;
  drive: string;
};

export type DevModuleDto = {
  id: string;
  path: string;
  name: string;
  kind: string;
  language: string;
  size_bytes: number;
  size_display: string;
  file_count: number;
  drive: string;
};

export async function scanApplications(): Promise<AppEntry[]> {
  return invoke<AppEntry[]>("scan_applications");
}

export async function getAppDetails(id: string): Promise<AppDetails> {
  return invoke<AppDetails>("get_app_details", { id });
}

export async function getSystemStats(): Promise<SystemStatsDto> {
  return invoke<SystemStatsDto>("get_system_stats");
}

export async function analyzeLeftovers(id: string, refresh = false): Promise<LeftoverDto[]> {
  return invoke<LeftoverDto[]>("analyze_leftovers", { id, refresh });
}

export type CleanLeftoverItem = {
  path: string;
  artifact_type: string;
  safety: string;
};

export type CleanLeftoverResult = {
  path: string;
  deleted: boolean;
  reason: string;
};

export async function cleanLeftoverArtifacts(
  items: CleanLeftoverItem[],
  force: boolean,
): Promise<CleanLeftoverResult[]> {
  return invoke<CleanLeftoverResult[]>("clean_leftover_artifacts", { items, force });
}

export async function uninstallApplications(payload: UninstallPayload): Promise<UninstallResultDto[]> {
  return invoke<UninstallResultDto[]>("uninstall_applications", { payload });
}

const iconCache = new Map<string, string | null>();

export async function getAppIcon(id: string): Promise<string | null> {
  if (iconCache.has(id)) return iconCache.get(id) ?? null;
  const v = await invoke<string | null>("get_app_icon", { id });
  // Only cache hits; misses may appear after a rescan
  if (v) iconCache.set(id, v);
  return v;
}

export function clearIconCache() {
  iconCache.clear();
}

export async function getAppResources(): Promise<Record<string, AppResourceDto>> {
  return invoke<Record<string, AppResourceDto>>("get_app_resources");
}

export async function getAppResource(id: string): Promise<AppResourceDto | null> {
  return invoke<AppResourceDto | null>("get_app_resource", { id });
}

export async function scanVideos(refresh = false): Promise<VideoEntryDto[]> {
  return invoke<VideoEntryDto[]>("scan_videos", { refresh });
}
export async function deleteVideos(paths: string[]): Promise<string[]> {
  return invoke<string[]>("delete_videos", { paths });
}
export async function scanDevModules(refresh = false): Promise<DevModuleDto[]> {
  return invoke<DevModuleDto[]>("scan_dev_modules", { refresh });
}
export async function cleanDevModules(paths: string[]): Promise<string[]> {
  return invoke<string[]>("clean_dev_modules", { paths });
}
export async function cleanAllDevModules(): Promise<string[]> {
  return invoke<string[]>("clean_all_dev_modules");
}

export function onUninstallProgress(cb: (e: UninstallProgressEvent) => void): Promise<UnlistenFn> {
  return listen<UninstallProgressEvent>("uninstall-progress", (event) => cb(event.payload));
}

/**
 * Heartbeat-aware listener with retry. Resolves with unlisten fn, or rejects if backend dies.
 * Frontend should show warning if no event for 5s (Audit 2 §2.2).
 */
export async function onUninstallProgressWithHeartbeat(
  cb: (e: UninstallProgressEvent) => void,
  onHeartbeatLost?: () => void
): Promise<UnlistenFn> {
  let lastEvent = Date.now();
  const wrapped = (e: UninstallProgressEvent) => {
    lastEvent = Date.now();
    cb(e);
  };
  const unlisten = await listen<UninstallProgressEvent>("uninstall-progress", (event) => wrapped(event.payload));
  // Silent/MSI uninstallers can go quiet for minutes; warn at 30s, not 5s.
  const heartbeat = window.setInterval(() => {
    if (Date.now() - lastEvent > 30000) {
      onHeartbeatLost?.();
    }
  }, 5000);
  const originalUnlisten = unlisten;
  return () => {
    window.clearInterval(heartbeat);
    originalUnlisten();
  };
}
