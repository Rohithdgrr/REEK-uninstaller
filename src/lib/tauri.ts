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
  current: number;
  total: number;
  app_name: string;
  status: "processing" | "done" | "error";
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
  size_display?: string | null;
  confidence: number;
  safety: string;
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

export async function scanApplications(): Promise<AppEntry[]> {
  return invoke<AppEntry[]>("scan_applications");
}

export async function getAppDetails(id: string): Promise<AppDetails> {
  return invoke<AppDetails>("get_app_details", { id });
}

export async function getSystemStats(): Promise<SystemStatsDto> {
  return invoke<SystemStatsDto>("get_system_stats");
}

export async function analyzeLeftovers(id: string): Promise<LeftoverDto[]> {
  return invoke<LeftoverDto[]>("analyze_leftovers", { id });
}

export async function uninstallApplications(payload: UninstallPayload): Promise<UninstallResultDto[]> {
  return invoke<UninstallResultDto[]>("uninstall_applications", { payload });
}

export async function getAppIcon(id: string): Promise<string | null> {
  return invoke<string | null>("get_app_icon", { id });
}

export async function getAppResources(): Promise<Record<string, AppResourceDto>> {
  return invoke<Record<string, AppResourceDto>>("get_app_resources");
}

export async function getAppResource(id: string): Promise<AppResourceDto | null> {
  return invoke<AppResourceDto | null>("get_app_resource", { id });
}

export function onUninstallProgress(cb: (e: UninstallProgressEvent) => void): Promise<UnlistenFn> {
  return listen<UninstallProgressEvent>("uninstall-progress", (event) => cb(event.payload));
}
