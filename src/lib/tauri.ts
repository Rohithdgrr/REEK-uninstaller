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
};

export type UninstallPayload = { ids: string[]; force: boolean };

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

export async function scanApplications(): Promise<AppEntry[]> {
  return invoke<AppEntry[]>("scan_applications");
}

export async function uninstallApplications(payload: UninstallPayload): Promise<UninstallResultDto[]> {
  return invoke<UninstallResultDto[]>("uninstall_applications", { payload });
}

export function onUninstallProgress(cb: (e: UninstallProgressEvent) => void): Promise<UnlistenFn> {
  return listen<UninstallProgressEvent>("uninstall-progress", (event) => cb(event.payload));
}
