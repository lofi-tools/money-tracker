import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Startup, Selection, Tree, Details, Settings } from "./contracts";
// One typed transport boundary. A browser preview never substitutes synthetic money.
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri())
    return Promise.reject(new Error("Desktop runtime unavailable"));
  return invoke<T>(command, args);
}
export const api = {
  startup: () => call<Startup>("startup"),
  startNew: () => call<Startup>("start_new"),
  copyDatabase: () => call<Startup | null>("copy_database"),
  allocation: (snapshotId: number, selection: Selection) =>
    call<Tree>("allocation", { snapshotId, selection }),
  details: (snapshotId: number, selection: Selection, nodeId: string) =>
    call<Details>("details", { snapshotId, selection, nodeId }),
  saveSettings: (settings: Settings) =>
    call<Startup>("save_settings", { settings }),
  quit: () => call<void>("quit"),
};
