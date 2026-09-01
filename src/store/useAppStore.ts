import { create } from "zustand";
import type { AppEntry, UninstallProgressEvent, UninstallResultDto } from "../lib/tauri";

export type View = "splash" | "dashboard" | "progress" | "results";
export type SortKey = "name" | "date" | "size" | "resources";
export type SortDir = "asc" | "desc";

type State = {
  view: View;
  apps: AppEntry[];
  loading: boolean;
  search: string;
  sortKey: SortKey;
  sortDir: SortDir;
  selected: Set<string>;
  force: boolean;
  showConfirm: boolean;
  progress: UninstallProgressEvent | null;
  logs: string[];
  results: UninstallResultDto[];
  error: string | null;
};

type Actions = {
  setView: (v: View) => void;
  setApps: (a: AppEntry[]) => void;
  setLoading: (b: boolean) => void;
  setSearch: (s: string) => void;
  setSort: (k: SortKey) => void;
  toggleSelect: (id: string) => void;
  toggleSelectAll: (ids: string[]) => void;
  clearSelection: () => void;
  setForce: (b: boolean) => void;
  setShowConfirm: (b: boolean) => void;
  pushLog: (s: string) => void;
  setProgress: (p: UninstallProgressEvent | null) => void;
  setResults: (r: UninstallResultDto[]) => void;
  setError: (e: string | null) => void;
  resetLogs: () => void;
};

export const useAppStore = create<State & Actions>((set) => ({
  view: "splash",
  apps: [],
  loading: true,
  search: "",
  sortKey: "size",
  sortDir: "desc",
  selected: new Set<string>(),
  force: false,
  showConfirm: false,
  progress: null,
  logs: [],
  results: [],
  error: null,

  setView: (view) => set({ view }),
  setApps: (apps) => set({ apps }),
  setLoading: (loading) => set({ loading }),
  setSearch: (search) => set({ search }),
  setSort: (key) =>
    set((s) => {
      if (s.sortKey === key) return { sortDir: s.sortDir === "asc" ? "desc" : "asc" };
      // size/resources default to heaviest-first (desc), others asc
      const defDir: SortDir = key === "size" || key === "resources" ? "desc" : "asc";
      return { sortKey: key, sortDir: defDir };
    }),
  toggleSelect: (id) =>
    set((s) => {
      const n = new Set(s.selected);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return { selected: n };
    }),
  toggleSelectAll: (ids) =>
    set((s) => {
      const all = ids.every((id) => s.selected.has(id));
      if (all) {
        const n = new Set(s.selected);
        ids.forEach((id) => n.delete(id));
        return { selected: n };
      } else {
        const n = new Set(s.selected);
        ids.forEach((id) => n.add(id));
        return { selected: n };
      }
    }),
  clearSelection: () => set({ selected: new Set() }),
  setForce: (force) => set({ force }),
  setShowConfirm: (showConfirm) => set({ showConfirm }),
  pushLog: (line) => set((s) => ({ logs: [...s.logs, line] })),
  setProgress: (progress) => set({ progress }),
  setResults: (results) => set({ results }),
  setError: (error) => set({ error }),
  resetLogs: () => set({ logs: [], progress: null, results: [] }),
}));
