import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Startup,
  Selection,
  Tree,
  Details,
  Settings,
  ManualEdit,
  RefreshTrigger,
  RefreshUpdate,
} from "./contracts";
export const reviewMode = import.meta.env.MODE === "review";
// One typed transport boundary. A browser preview never substitutes synthetic money.
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (reviewMode)
    return import("./review-api").then(({ reviewCall }) =>
      reviewCall<T>(command, args),
    );
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
  /** `null` means an equivalent refresh was already running and coalesced. */
  refresh: (trigger: RefreshTrigger) =>
    call<RefreshUpdate | null>("refresh", { trigger }),
  manualEdit: (edit: ManualEdit) => call<Startup>("manual_edit", { edit }),
  undoAction: (id: string) => call<Startup>("undo_action", { id }),
  quit: () => call<void>("quit"),
};
/** Scheduled refresh passes arrive as events; the browser preview has none. */
export function onRefresh(
  handler: (update: RefreshUpdate) => void,
): () => void {
  if (reviewMode || !isTauri()) return () => {};
  let stop: (() => void) | undefined;
  let cancelled = false;
  void import("@tauri-apps/api/event").then(async ({ listen }) => {
    const unlisten = await listen<RefreshUpdate>("refresh", (event) =>
      handler(event.payload),
    );
    if (cancelled) unlisten();
    else stop = unlisten;
  });
  return () => {
    cancelled = true;
    stop?.();
  };
}

export function onBrowserCapture(
  callback: (result: Startup) => void,
): () => void {
  let stop: (() => void) | undefined;
  let cancelled = false;
  if (!reviewMode && isTauri())
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const unlisten = await listen<Startup>("browser-capture", (event) =>
        callback(event.payload),
      );
      if (cancelled) unlisten();
      else stop = unlisten;
    });
  return () => {
    cancelled = true;
    stop?.();
  };
}
