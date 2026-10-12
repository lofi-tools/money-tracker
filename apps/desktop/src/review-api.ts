import type { Details, Selection, Settings, Startup, Tree } from "./contracts";
interface ReviewSnapshot {
  startup: Extract<Startup, { status: "ready" }>;
  trees: Record<string, Tree>;
  details: Record<string, Details>;
}
const selectionKey = (s: Selection) =>
  JSON.stringify("id" in s ? { kind: s.kind, id: s.id } : { kind: s.kind });
let snapshot: Promise<ReviewSnapshot> | undefined;
function load() {
  return (snapshot ??= fetch("/__portfolio_review", { cache: "no-store" })
    .then(async (response) => {
      if (!response.ok) throw Error("Review snapshot unavailable");
      return (await response.json()) as ReviewSnapshot;
    })
    .catch((error) => {
      snapshot = undefined;
      throw error;
    }));
}
export async function reviewCall<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  const data = await load();
  let result: unknown;
  switch (command) {
    case "startup":
      result = data.startup;
      break;
    case "allocation":
      if (args.snapshotId !== data.startup.data.snapshot_id)
        throw Error("Stale snapshot");
      result = data.trees[selectionKey(args.selection as Selection)];
      break;
    case "details":
      if (args.snapshotId !== data.startup.data.snapshot_id)
        throw Error("Stale snapshot");
      result =
        data.details[
          `${selectionKey(args.selection as Selection)}:${args.nodeId}`
        ];
      break;
    case "save_settings": {
      const settings = args.settings as Settings;
      const current = data.startup.settings;
      if (
        settings.reporting_currency !== current.reporting_currency ||
        settings.dust_threshold !== current.dust_threshold ||
        settings.privacy_scale !== current.privacy_scale ||
        settings.refresh_interval_seconds !==
          current.refresh_interval_seconds ||
        settings.refresh_on_open !== current.refresh_on_open
      )
        throw Error("Review supports appearance changes only");
      data.startup = {
        ...data.startup,
        settings: {
          ...current,
          theme: settings.theme,
          privacy_mode: settings.privacy_mode,
        },
      };
      result = data.startup;
      break;
    }
    default:
      throw Error("Desktop-only action unavailable in review");
  }
  if (result === undefined) throw Error("Review selection unavailable");
  return structuredClone(result) as T;
}
