import {
  For,
  Match,
  Show,
  Switch,
  batch,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";
import {
  ThemeRoot,
  PrivacyProvider,
  Button,
  Icon,
  SearchField,
  GroupingTabs,
  NavigationEntry,
  SummaryValue,
  MoneyFigure,
  Quantity,
  Rate,
  Badge,
  WarningButton,
  AllocationTable,
  DetailsPanel,
  StateMessage,
  TextField,
  InlineEdit,
} from "@money-tracker/design-system";
import type {
  AllocationRow,
  CurrencyFigure,
  Rational,
} from "@money-tracker/design-system";
import { api, onRefresh, reviewMode } from "./api";
import { defaultSettings } from "./contracts";
import type {
  Startup,
  Settings,
  RefreshReport,
  RefreshUpdate,
  Snapshot,
  Selection,
  Summary,
  Figure,
  MoneyValue,
  Holding,
  Tree,
  Details,
  Issue,
  Entry,
  LocalTarget,
  LocalValue,
  EditSlot,
  ManualEdit,
  ManualRate,
  ManualPrice,
  AnnualRate,
  NodeAccount,
} from "./contracts";
const figure = (f: Figure<MoneyValue>): CurrencyFigure =>
  f.status === "known" || f.status === "partial"
    ? { status: f.status, value: f.value.amount }
    : { status: f.status };
const key = (s: Selection) => JSON.stringify(s);
/** Exact percent/fraction conversion: binary floats never reach stored rates. */
function percentToFraction(text: string): string {
  const [whole = "", fraction = ""] = text.trim().split(".");
  const digits = whole + fraction;
  const decimals = fraction.length + 2;
  if (digits.length <= decimals) return `0.${digits.padStart(decimals, "0")}`;
  return `${digits.slice(0, -decimals)}.${digits.slice(-decimals)}`;
}
function fractionToPercent(text: string): string {
  const [whole = "0", fraction = ""] = text.trim().split(".");
  const decimals = fraction.length - 2;
  const digits = (whole + fraction).replace(/^0+/, "") || "0";
  if (decimals <= 0) return digits;
  return `${digits.slice(0, -decimals)}.${digits.slice(-decimals)}`;
}
const percentPattern = /^\d*(\.\d+)?$/;
const validatePercent = (value: string) =>
  value.trim() === "" || percentPattern.test(value.trim())
    ? undefined
    : "Use a nonnegative percentage, for example 4.5.";
const validateDecimal = (value: string) =>
  percentPattern.test(value.trim())
    ? undefined
    : "Use a nonnegative amount, for example 1234.56.";
const today = () => new Date().toISOString().slice(0, 10);
const atMidnight = (day: string) => `${day}T00:00:00Z`;
/** Manual rates and prices are stored as annual percentages by default. */
const apy = (text: string): AnnualRate => ({
  fraction: percentToFraction(text),
  basis: "apy",
  compounds_per_year: null,
  reward_asset: null,
});
const manualRate = (slot?: EditSlot | null): ManualRate | undefined =>
  slot?.value?.kind === "rates" ? slot.value.value.at(-1) : undefined;
const manualPrice = (slot?: EditSlot | null): ManualPrice | undefined =>
  slot?.value?.kind === "prices" ? slot.value.value.at(-1) : undefined;
const rateScope = (slot?: EditSlot | null) =>
  slot?.target.kind === "rate" ? slot.target.scope : undefined;
const priceTarget = (slot?: EditSlot | null) =>
  slot?.target.kind === "price" ? slot.target : undefined;
const spamState = (slot?: EditSlot | null) =>
  slot?.value?.kind === "spam"
    ? slot.value.value
    : { excluded: false, reason: "" };
const targetKey = (target: LocalTarget) => JSON.stringify(target);
/** Refresh outcomes carry counts and static text only, never amounts. */
function refreshNotice(report: RefreshReport): string {
  const parts = report.entries.flatMap((entry) => {
    if (entry.outcome.status === "updated")
      return [`${entry.label}: updated (${entry.outcome.detail})`];
    if (entry.outcome.status === "failed")
      return [`${entry.label}: refresh failed — ${entry.outcome.message}`];
    return [];
  });
  return parts.length
    ? parts.join(" · ")
    : "Refresh finished; cached figures are unchanged.";
}
const missing = (value: string | null) =>
  value === null ? "Never" : new Date(value).toLocaleString();

function Apy(props: { figure: Figure<Rational> }) {
  return (
    <Show
      when={
        props.figure.status === "known" || props.figure.status === "partial"
      }
      fallback={
        <span class="muted">
          {props.figure.status === "not_applicable"
            ? "APY not applicable"
            : "APY unknown"}
        </span>
      }
    >
      <Rate value={(props.figure as { value: Rational }).value} />
      <Show when={props.figure.status === "partial"}>
        <span class="muted"> (est.)</span>
      </Show>
    </Show>
  );
}
// Deliberately display codes/withheld fields rather than provider diagnostic messages:
// a mismatch message may embed original private quantities even in hidden mode.
function Warnings(props: { issues: Issue[] }) {
  return (
    <Show
      when={props.issues.length}
      fallback={<p class="muted">No unresolved coverage warnings.</p>}
    >
      <ul class="warnings">
        <For each={props.issues}>
          {(issue) => (
            <li>
              <strong>{issue.code.replaceAll("_", " ")}</strong>
              <Show when={issue.withheld.length}>
                <div class="muted">
                  Unavailable or partial:{" "}
                  {issue.withheld.map((x) => x.replaceAll("_", " ")).join(", ")}
                </div>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}
export function App() {
  const [settings, setSettings] = createSignal<Settings>(defaultSettings);
  return (
    <ThemeRoot mode={settings().theme} class="desktop-stage">
      <PrivacyProvider
        value={{
          mode: settings().privacy_mode,
          scale: settings().privacy_scale,
        }}
      >
        <Body settings={settings()} setSettings={setSettings} />
      </PrivacyProvider>
    </ThemeRoot>
  );
}
function Body(props: {
  settings: Settings;
  setSettings: (s: Settings) => void;
}) {
  const [startup, setStartup] = createSignal<Startup>();
  const [data, setData] = createSignal<Snapshot>();
  const [error, setError] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [group, setGroup] = createSignal("asset");
  const [search, setSearch] = createSignal("");
  const [selection, setSelection] = createSignal<Selection>({
    kind: "overview",
  });
  const [tree, setTree] = createSignal<Tree>();
  const [treeBusy, setTreeBusy] = createSignal(false);
  const [expanded, setExpanded] = createSignal(new Set<string>());
  const [panelNode, setPanelNode] = createSignal<{ id: string }>();
  const [panelWarnings, setPanelWarnings] = createSignal<Entry>();
  const [panel, setPanel] = createSignal<Details>();
  const [options, setOptions] = createSignal<"" | "settings" | "history">("");
  const [draft, setDraft] = createSignal<Settings>(defaultSettings);
  const [notice, setNotice] = createSignal("");
  let returnFocus: HTMLElement | undefined;
  let treeRequest = 0;
  let detailRequest = 0;
  const failure = () =>
    setError(
      "Unable to read or save local data. Check configuration and close any other database writer, then retry. Financial details are omitted from this message.",
    );
  function accept(result: Startup) {
    if (result.status === "ready" && result.data.contract_version !== 1)
      throw new Error("Unsupported contract");
    batch(() => {
      props.setSettings(result.settings);
      setStartup(result);
      if (result.status === "ready") setData(result.data);
    });
  }
  async function start(action: () => Promise<Startup | null>) {
    setBusy(true);
    setError("");
    try {
      const r = await action();
      if (r) accept(r);
    } catch {
      failure();
    } finally {
      setBusy(false);
    }
  }
  onMount(() => void start(api.startup));
  // Scheduled refresh runs announce their result; the stored snapshot is adopted.
  onCleanup(
    onRefresh((update: RefreshUpdate) => {
      accept(update.startup);
      if (update.report.entries.some((e) => e.outcome.status !== "not_due"))
        setNotice(refreshNotice(update.report));
    }),
  );
  /** Explicit refresh from Settings; a concurrent pass coalesces into one run. */
  async function refreshNow() {
    if (busy() || reviewMode) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const update = await api.refresh("manual");
      if (!update) {
        setNotice("A refresh is already running; its result will appear here.");
        return;
      }
      accept(update.startup);
      setNotice(refreshNotice(update.report));
    } catch {
      setError(
        "Unable to refresh. Existing cached figures are retained; check your connection and provider configuration.",
      );
    } finally {
      setBusy(false);
    }
  }
  const entries = createMemo(() => {
    const list = group() === "asset" ? data()?.assets : data()?.providers;
    const text = search().toLocaleLowerCase();
    return (list ?? []).filter((e) =>
      `${e.name} ${e.ticker ?? ""}`.toLocaleLowerCase().includes(text),
    );
  });
  const selectedEntry = createMemo(() =>
    [
      ...(data()?.assets ?? []),
      ...(data()?.providers ?? []),
      ...(data() ? [data()!.dust] : []),
    ].find((e) => key(e.selection) === key(selection())),
  );
  const summary = () =>
    selection().kind === "overview"
      ? data()!.overview
      : (selectedEntry()?.summary ?? data()!.overview);
  function select(s: Selection) {
    detailRequest++;
    setPanelNode(undefined);
    setPanelWarnings(undefined);
    setPanel(undefined);
    setOptions("");
    setSelection(s);
  }
  // Details follow the snapshot, so a committed edit refreshes them in place.
  createEffect(() => {
    const snapshot = data();
    const node = panelNode();
    if (!snapshot || !node || panelWarnings()) return;
    const request = ++detailRequest;
    void api
      .details(snapshot.snapshot_id, selection(), node.id)
      .then((d) => {
        if (request === detailRequest) setPanel(d);
      })
      .catch(() => {
        if (request === detailRequest) failure();
      });
  });
  createEffect(() => {
    const snapshot = data();
    const chosen = selection();
    const request = ++treeRequest;
    detailRequest++;
    setPanel(undefined);
    setTree(undefined);
    if (!snapshot || chosen.kind === "overview") {
      setTreeBusy(false);
      return;
    }
    setTreeBusy(true);
    void api
      .allocation(snapshot.snapshot_id, chosen)
      .then((t) => {
        if (request !== treeRequest) return;
        batch(() => {
          setTree(t); // Open roots, exposing one level beneath the selected account hierarchy.
          setExpanded(new Set(t.initially_expanded));
          setTreeBusy(false);
        });
      })
      .catch(() => {
        if (request === treeRequest) {
          setTreeBusy(false);
          failure();
        }
      });
  });
  function openDetails(id: string, trigger: HTMLButtonElement) {
    if (!data()) return;
    returnFocus = trigger;
    setOptions("");
    setPanelWarnings(undefined);
    setPanel(undefined);
    setPanelNode({ id });
  }
  function entryWarnings(e: Entry, trigger: HTMLElement) {
    detailRequest++;
    returnFocus = trigger;
    setOptions("");
    setPanelNode(undefined);
    setPanel({
      name: e.name,
      summary: e.summary,
      issues: e.issues,
      holdings: [],
    });
    setPanelWarnings(e);
  }
  /** One typed manual edit or Undo; a rejected change never alters figures. */
  async function applyEdit(request: ManualEdit) {
    if (busy() || reviewMode) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      accept(await api.manualEdit(request));
    } catch {
      setNotice(
        "Unable to save this change. Check the value, effective date and selection, then retry.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function undoAction(id: string) {
    if (busy() || reviewMode) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      accept(await api.undoAction(id));
    } catch {
      setNotice(
        "This change can no longer be undone because newer manual changes exist.",
      );
    } finally {
      setBusy(false);
    }
  }
  const openHistory = (trigger: HTMLElement) => {
    detailRequest++;
    setPanelNode(undefined);
    setPanelWarnings(undefined);
    setPanel(undefined);
    returnFocus = trigger;
    setOptions("history");
  };
  const openOptions = (trigger: HTMLElement) => {
    detailRequest++;
    setPanelNode(undefined);
    setPanelWarnings(undefined);
    setPanel(undefined);
    returnFocus = trigger;
    setDraft({ ...props.settings });
    setOptions("settings");
  };
  const rows = createMemo<AllocationRow[]>(() => {
    const t = tree();
    if (!t) return [];
    const nodes = new Map(t.nodes.map((n) => [n.id, n]));
    const result: AllocationRow[] = [];
    function walk(id: string, depth: number) {
      const n = nodes.get(id);
      if (!n) return;
      result.push({
        id: n.id,
        name: n.name,
        depth,
        hasChildren: n.children.length > 0,
        expanded: expanded().has(id),
        liability: n.liability,
        value: (
          <MoneyFigure
            figure={figure(
              n.liability ? n.summary.liabilities : n.summary.current_value,
            )}
            currency={props.settings.reporting_currency}
          />
        ),
        rate: (
          <>
            {/* Narrow windows show the derived rate only; manual row editing
                happens in the details panel below 900px. */}
            <span class="narrow-only">
              <Apy
                figure={
                  n.children.length
                    ? (n.summary.estimated_current_apy ?? n.summary.current_apy)
                    : n.summary.current_apy
                }
              />
            </span>
            <Show when={n.edits?.rate}>
              <span class="wide-only">
                <RateEditor
                  slot={n.edits!.rate!}
                  fallback={
                    n.children.length
                      ? (n.summary.estimated_current_apy ??
                        n.summary.current_apy)
                      : n.summary.current_apy
                  }
                  onEdit={(request) => void applyEdit(request)}
                />
              </span>
            </Show>
          </>
        ),
        // Leaf rows carry their own price override; rollups display derived figures.
        actions: n.edits?.price ? (
          <span class="wide-only">
            <PriceEditor
              slot={n.edits.price}
              selected={n.edits.selected_price}
              currency={props.settings.reporting_currency}
              onEdit={(request) => void applyEdit(request)}
            />
          </span>
        ) : undefined,
        annual: (
          <MoneyFigure
            figure={figure(n.summary.net_annual_earnings)}
            currency={props.settings.reporting_currency}
          />
        ),
        warning: n.issues.length
          ? {
              label: `Warnings for ${n.name}`,
              onClick: (e) => void openDetails(n.id, e.currentTarget),
            }
          : undefined,
      });
      if (expanded().has(id))
        n.children.forEach((child) => walk(child, depth + 1));
    }
    t.roots.forEach((id) => walk(id, 0));
    return result;
  });
  function expand(id: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }
  async function save(settings: Settings) {
    if (busy()) return;
    const old = props.settings;
    props.setSettings({ ...old, privacy_mode: settings.privacy_mode });
    setBusy(true);
    setError("");
    try {
      accept(await api.saveSettings(settings));
      setOptions("");
    } catch {
      props.setSettings(old);
      failure();
    } finally {
      setBusy(false);
    }
  }
  const panelData = createMemo<Details | undefined>(() => {
    const warnings = panelWarnings();
    if (warnings)
      return {
        name: warnings.name,
        summary: warnings.summary,
        issues: warnings.issues,
        holdings: [],
      };
    return panel();
  });
  const money = (f: Figure<MoneyValue>) => (
    <MoneyFigure
      figure={figure(f)}
      currency={props.settings.reporting_currency}
    />
  );
  return (
    <div class={`desktop ${panel() ? "panel-open" : ""}`}>
      <header class="app-header">
        <h1>Money Tracker</h1>
      </header>
      <Show when={reviewMode}>
        <aside class="banner" aria-label="Review notice">
          Local portfolio preview · appearance changes are temporary ·
          desktop-only actions are unavailable
        </aside>
      </Show>
      <Show when={props.settings.privacy_mode === "scaled"}>
        <aside class="banner" aria-label="Privacy notice">
          Scaled amounts for privacy. Percentage rates remain unchanged.
        </aside>
      </Show>
      <Show when={error()}>
        <StateMessage
          kind="error"
          title="Local data unavailable"
          description={error()}
          action={
            <Button disabled={busy()} onClick={() => void start(api.startup)}>
              Retry
            </Button>
          }
        />
      </Show>
      <Show
        when={startup()}
        fallback={
          <div class="startup">
            <StateMessage kind="loading" title="Opening local portfolio…" />
          </div>
        }
      >
        <Show
          when={data()}
          fallback={
            <div class="startup">
              <StateMessage
                kind="empty"
                title="Choose your local portfolio"
                description="Copy an existing imported Money Tracker database, or explicitly start an empty one."
              />
              <p class="source">
                Destination:{" "}
                {startup()?.status === "missing"
                  ? (startup() as Extract<Startup, { status: "missing" }>)
                      .database.destination
                  : ""}
              </p>
              <Show
                when={
                  startup()?.status === "missing" &&
                  (startup() as Extract<Startup, { status: "missing" }>)
                    .database.legacy_database
                }
              >
                <p class="source">
                  A previous database was found at{" "}
                  {
                    (startup() as Extract<Startup, { status: "missing" }>)
                      .database.legacy_database
                  }
                  . Choose it in the file picker to copy it.
                </p>
              </Show>
              <div class="tools">
                <Button
                  disabled={busy()}
                  onClick={() => void start(api.copyDatabase)}
                >
                  Copy existing database…
                </Button>
                <Button
                  intent="secondary"
                  disabled={busy()}
                  onClick={() => void start(api.startNew)}
                >
                  Start new portfolio
                </Button>
              </div>
            </div>
          }
        >
          <div class="workspace">
            <nav class="sidebar" aria-label="Portfolio navigation">
              <div class="sidebar-controls">
                <NavigationEntry
                  name="Overview"
                  selected={!options() && selection().kind === "overview"}
                  onSelect={() => select({ kind: "overview" })}
                />
                <GroupingTabs
                  value={group()}
                  onChange={(value) => {
                    setGroup(value);
                    setSearch("");
                    select({ kind: "overview" });
                  }}
                  label="Group portfolio"
                  items={[
                    { value: "asset", label: "By asset" },
                    { value: "provider", label: "By provider" },
                  ]}
                />
                <SearchField
                  value={search()}
                  onChange={setSearch}
                  label="Search assets or providers"
                />
              </div>
              <div
                class="sidebar-list"
                role="region"
                tabindex="0"
                aria-label="Assets and providers"
              >
                <For each={entries()}>
                  {(entry) => (
                    <NavigationEntry
                      name={entry.name}
                      secondary={entry.ticker ?? undefined}
                      selected={
                        !options() && key(entry.selection) === key(selection())
                      }
                      onSelect={() => select(entry.selection)}
                      value={
                        <span class="nav-figures">
                          {money(entry.summary.current_value)}
                          <For each={entry.quantities}>
                            {([role, q]) => (
                              <span class="nav-quantity">
                                <Quantity
                                  units={q.units}
                                  scale={q.scale}
                                  asset={q.asset_id}
                                />
                                {role === "liability" ? " borrowed" : ""}
                              </span>
                            )}
                          </For>
                        </span>
                      }
                      rate={(() => {
                        const rate =
                          entry.summary.estimated_current_apy ??
                          entry.summary.current_apy;
                        return rate.status === "known" ||
                          rate.status === "partial" ? (
                          <Apy figure={rate} />
                        ) : undefined;
                      })()}
                      warning={
                        entry.issues.length
                          ? {
                              label: `Warnings for ${entry.name}`,
                              onClick: (e) =>
                                entryWarnings(entry, e.currentTarget),
                            }
                          : undefined
                      }
                    />
                  )}
                </For>
                <Show when={data()!.dust_count > 0}>
                  <NavigationEntry
                    name="Dust"
                    selected={!options() && selection().kind === "dust"}
                    onSelect={() => select({ kind: "dust" })}
                    value={money(data()!.dust.summary.current_value)}
                    rate={(() => {
                      const rate =
                        data()!.dust.summary.estimated_current_apy ??
                        data()!.dust.summary.current_apy;
                      return rate.status === "known" ||
                        rate.status === "partial" ? (
                        <Apy figure={rate} />
                      ) : undefined;
                    })()}
                    warning={
                      data()!.dust.issues.length
                        ? {
                            label: "Warnings for Dust",
                            onClick: (e) =>
                              entryWarnings(data()!.dust, e.currentTarget),
                          }
                        : undefined
                    }
                  />
                </Show>
              </div>
              <div class="sidebar-footer">
                <Button
                  intent="ghost"
                  aria-current={options() === "settings" ? "page" : undefined}
                  disabled={!data()}
                  onClick={(e) => openOptions(e.currentTarget)}
                >
                  <Icon name="gear" />
                  Settings
                </Button>
                <Button
                  intent="ghost"
                  aria-current={options() === "history" ? "page" : undefined}
                  disabled={!data()}
                  onClick={(e) => openHistory(e.currentTarget)}
                >
                  <Icon name="clock" />
                  Change history
                </Button>
              </div>
            </nav>
            <main class="main-pane" tabindex="0" aria-label="Portfolio content">
              <Show when={notice()}>
                <div role="status" class="banner">
                  {notice()}
                </div>
              </Show>
              <Switch>
                <Match when={options() === "settings"}>
                  <section aria-label="Settings">
                    <h2>Settings</h2>
                    <form
                      class="settings"
                      onSubmit={(e) => {
                        e.preventDefault();
                        void save(draft());
                      }}
                    >
                      <div class="privacy-settings-row">
                        <label>
                          Privacy
                          <select
                            aria-label="Privacy mode"
                            value={draft().privacy_mode}
                            onChange={(e) =>
                              setDraft({
                                ...draft(),
                                privacy_mode: e.currentTarget
                                  .value as Settings["privacy_mode"],
                              })
                            }
                          >
                            <option value="normal">Normal</option>
                            <option value="hidden">Hidden</option>
                            <option value="scaled">Scaled down</option>
                          </select>
                        </label>
                        <Show when={draft().privacy_mode === "scaled"}>
                          <TextField
                            label="Privacy scale (fraction)"
                            disabled={reviewMode}
                            value={draft().privacy_scale}
                            onInput={(e) =>
                              setDraft({
                                ...draft(),
                                privacy_scale: e.currentTarget.value,
                              })
                            }
                          />
                        </Show>
                      </div>
                      <label>
                        Theme
                        <select
                          aria-label="Theme"
                          value={draft().theme}
                          onChange={(e) =>
                            setDraft({
                              ...draft(),
                              theme: e.currentTarget.value as Settings["theme"],
                            })
                          }
                        >
                          <option value="system">System</option>
                          <option value="light">Light</option>
                          <option value="dark">Dark</option>
                        </select>
                      </label>
                      <TextField
                        label="Reporting currency"
                        disabled={reviewMode}
                        value={draft().reporting_currency}
                        onInput={(e) =>
                          setDraft({
                            ...draft(),
                            reporting_currency: e.currentTarget.value,
                          })
                        }
                      />

                      <TextField
                        label="Dust threshold in reporting currency"
                        prefix={
                          draft().reporting_currency === "USD"
                            ? "$"
                            : draft().reporting_currency
                        }
                        disabled={reviewMode}
                        value={draft().dust_threshold}
                        onInput={(e) =>
                          setDraft({
                            ...draft(),
                            dust_threshold: e.currentTarget.value,
                          })
                        }
                      />
                      <p class="muted">
                        The app refreshes the configured sources below at this
                        interval while it is open; closing it ends all refresh
                        work. Refresh on open is separate and off by default,
                        but a missed interval triggers one catch-up refresh.
                      </p>
                      <TextField
                        label="Refresh interval (seconds)"
                        disabled={reviewMode}
                        type="number"
                        min="1"
                        step="1"
                        value={draft().refresh_interval_seconds}
                        onInput={(e) =>
                          setDraft({
                            ...draft(),
                            refresh_interval_seconds: Number(
                              e.currentTarget.value,
                            ),
                          })
                        }
                      />
                      <label>
                        <span>
                          <input
                            type="checkbox"
                            disabled={reviewMode}
                            checked={draft().refresh_on_open}
                            onChange={(e) =>
                              setDraft({
                                ...draft(),
                                refresh_on_open: e.currentTarget.checked,
                              })
                            }
                          />{" "}
                          Refresh on open
                        </span>
                      </label>
                      <Button type="submit" disabled={busy()}>
                        Save settings
                      </Button>
                    </form>
                    <section aria-label="Refresh status">
                      <h3>Refresh status</h3>
                      <Button
                        disabled={reviewMode || busy()}
                        onClick={() => void refreshNow()}
                      >
                        Refresh now
                      </Button>
                      <Show
                        when={data()?.refresh?.length}
                        fallback={
                          <p class="muted">
                            No refresh source is configured for this portfolio
                            yet.
                          </p>
                        }
                      >
                        <table class="refresh-status">
                          <thead>
                            <tr>
                              <th>Source</th>
                              <th>Last success</th>
                              <th>Next refresh</th>
                              <th>Status</th>
                            </tr>
                          </thead>
                          <tbody>
                            <For each={data()?.refresh ?? []}>
                              {(source) => (
                                <tr>
                                  <td>{source.label}</td>
                                  <td>{missing(source.last_success)}</td>
                                  <td>
                                    {source.next_due === null
                                      ? "Not scheduled"
                                      : missing(source.next_due)}
                                  </td>
                                  <td>
                                    {source.prerequisite ??
                                      source.last_error ??
                                      (source.ready ? "Ready" : "Unavailable")}
                                  </td>
                                </tr>
                              )}
                            </For>
                          </tbody>
                        </table>
                      </Show>
                    </section>
                  </section>
                </Match>
                <Match when={options() === "history"}>
                  <section aria-label="Change history">
                    <h2>Change history</h2>
                    <HistoryPane
                      data={data()!}
                      onUndo={(id) => void undoAction(id)}
                    />
                  </section>
                </Match>
                <Match when={true}>
                  <div class="selection-summary">
                    <h2>
                      {selection().kind === "overview"
                        ? "Overview"
                        : (selectedEntry()?.name ?? "Allocation")}
                    </h2>
                    <Show when={selectedEntry()?.issues.length}>
                      <WarningButton
                        label="Selected allocation warnings"
                        onClick={(e) =>
                          entryWarnings(selectedEntry()!, e.currentTarget)
                        }
                      />
                    </Show>
                  </div>
                  <div class="summaries">
                    <SummaryValue
                      label="Current value"
                      figure={figure(summary().current_value)}
                      currency={props.settings.reporting_currency}
                      primary
                    />
                    <SummaryValue
                      label="Currently earning · per year"
                      figure={figure(summary().net_annual_earnings)}
                      currency={props.settings.reporting_currency}
                    />
                  </div>
                  <Show when={selection().kind !== "overview"}>
                    <div class="quantity-list">
                      <For each={selectedEntry()?.quantities}>
                        {([role, q]) => (
                          <span>
                            <Quantity
                              units={q.units}
                              scale={q.scale}
                              asset={q.asset_id}
                            />{" "}
                            {role === "liability" ? "borrowed" : ""}
                          </span>
                        )}
                      </For>
                    </div>
                    <Show
                      when={!treeBusy()}
                      fallback={
                        <StateMessage
                          kind="loading"
                          title="Reading allocation…"
                        />
                      }
                    >
                      <Show
                        when={rows().length}
                        fallback={
                          <StateMessage
                            kind="empty"
                            title="No current allocations"
                          />
                        }
                      >
                        <AllocationTable
                          rows={rows()}
                          onExpand={expand}
                          onDetails={(id, trigger) =>
                            void openDetails(id, trigger)
                          }
                          caption="Current allocations"
                        />
                      </Show>
                    </Show>
                  </Show>
                  <Show when={data()!.holding_count === 0}>
                    <StateMessage
                      kind="empty"
                      title="No imported holdings yet"
                      description="Import using the CLI. Account connections and CSV imports in the app will follow later."
                    />
                  </Show>
                </Match>
              </Switch>
            </main>
          </div>
        </Show>
      </Show>
      <DetailsPanel
        open={!!panelData()}
        onOpenChange={(open) => {
          if (!open) {
            detailRequest++;
            setPanelNode(undefined);
            setPanelWarnings(undefined);
            setPanel(undefined);
            setOptions("");
            queueMicrotask(() => returnFocus?.focus());
          }
        }}
        title={panelData()?.name ?? "Details"}
        returnFocus={() => returnFocus}
      >
        <Show when={panelData()}>
          {(d) => (
            <>
              <Warnings issues={d().issues} />
              <Show
                when={d().summary.estimated_current_apy?.status === "partial"}
              >
                <p class="muted">
                  Estimated aggregate APY assumes missing investment rates are
                  zero. Individual rates remain unknown.
                </p>
              </Show>
              <div class="detail-block">
                <p>Gross assets: {money(d().summary.gross_assets)}</p>
                <p>Liabilities: {money(d().summary.liabilities)}</p>
                <p>
                  Gross earnings per year:{" "}
                  {money(d().summary.gross_annual_earnings)}
                </p>
                <p>
                  Borrowing costs per year:{" "}
                  {money(d().summary.annual_borrowing_costs)}
                </p>
                <p>
                  Net earnings per year:{" "}
                  {money(d().summary.net_annual_earnings)}
                </p>
                <Apy figure={d().summary.current_apy} />
                <Show when={d().original_name}>
                  <p class="source">Original account: {d().original_name}</p>
                </Show>
                <Show when={d().source_reference}>
                  <p class="source">
                    Provider reference: {d().source_reference}
                  </p>
                </Show>
              </div>
              <For each={d().holdings}>
                {(holding) => (
                  <section class="detail-block">
                    <strong>{holding.label}</strong>
                    <Show when={holding.role === "liability"}>
                      <Badge tone="danger">Liability · borrowed amount</Badge>
                    </Show>
                    <Quantity
                      units={holding.quantity.units}
                      scale={holding.quantity.scale}
                      asset={holding.quantity.asset_id}
                    />
                    <p>Current value: {money(holding.value)}</p>
                    <p>Earnings per year: {money(holding.annual_earnings)}</p>
                    <p>
                      Borrowing costs per year:{" "}
                      {money(holding.annual_borrowing_cost)}
                    </p>
                    <p class="muted">
                      {holding.verification_current
                        ? "Independent balance evidence at this cutoff"
                        : "Ledger balance · independent verification unavailable at this cutoff"}
                    </p>
                    <Show when={holding.subscription}>
                      {(s) => (
                        <>
                          <p>Lock: {s().lock_status}</p>
                          <p>Maturity: {s().maturity ?? "Unknown"}</p>
                          <p>
                            Auto-renew:{" "}
                            {s().auto_renew === null
                              ? "Unknown"
                              : s().auto_renew
                                ? "On"
                                : "Off"}
                          </p>
                        </>
                      )}
                    </Show>
                    <Show when={holding.rate}>
                      {(r) => (
                        <p class="muted">
                          Rate evidence updated {r().updated_at} · effective{" "}
                          {r().effective_at} ·{" "}
                          {r().pinned
                            ? "pinned manual override"
                            : r().evidence_id
                              ? "source evidence"
                              : "manual override"}
                        </p>
                      )}
                    </Show>
                    <Show when={holding.price}>
                      {(p) => (
                        <p class="muted">
                          Price as of {p().as_of} ·{" "}
                          {p().pinned
                            ? "pinned manual override"
                            : p().evidence_id
                              ? "source evidence"
                              : "manual override"}
                        </p>
                      )}
                    </Show>
                    <Warnings issues={holding.issues} />
                    <div class="detail-edits">
                      <Show when={holding.edits.rate}>
                        <RateEditor
                          slot={holding.edits.rate!}
                          fallback={apyFigure(holding)}
                          onEdit={(request) => void applyEdit(request)}
                        />
                      </Show>
                      <Show when={holding.edits.price}>
                        <PriceEditor
                          slot={holding.edits.price!}
                          selected={holding.price}
                          currency={props.settings.reporting_currency}
                          hidden={props.settings.privacy_mode === "hidden"}
                          onEdit={(request) => void applyEdit(request)}
                        />
                      </Show>
                      <Show when={holding.edits.dust}>
                        <DustControls
                          provider={holding.provider_id}
                          asset={holding.quantity.asset_id}
                          onEdit={(request) => void applyEdit(request)}
                        />
                      </Show>
                      <SpamControls
                        slot={holding.edits.spam}
                        asset={holding.quantity.asset_id}
                        onEdit={(request) => void applyEdit(request)}
                      />
                      <Show when={holding.edits.name}>
                        <NameEditor
                          slot={holding.edits.name!}
                          onEdit={(request) => void applyEdit(request)}
                        />
                      </Show>
                    </div>
                  </section>
                )}
              </For>
              <Show when={d().account}>
                {(account) => (
                  <AccountControls
                    account={account()}
                    onEdit={(request) => void applyEdit(request)}
                  />
                )}
              </Show>
            </>
          )}
        </Show>
      </DetailsPanel>
    </div>
  );
}

const rational = (fraction: string): Rational => {
  const [whole = "0", scale = ""] = fraction.split(".");
  const digits = (whole + scale).replace("-", "").replace(/^0+/, "") || "0";
  const numerator = BigInt(digits) * (fraction.startsWith("-") ? -1n : 1n);
  return {
    numerator: numerator.toString(),
    denominator: (10n ** BigInt(scale.length)).toString(),
  };
};
const apyFigure = (holding: Holding): Figure<Rational> =>
  holding.rate?.rate
    ? { status: "known", value: rational(holding.rate.rate.fraction) }
    : { status: "unknown" };
const subscriptionId = (slot: EditSlot) =>
  slot.target.kind === "subscription" ? slot.target.id : "";
/** Inline APY editing with a dated, pinned manual override and a source reset. */
function RateEditor(props: {
  slot: EditSlot;
  fallback: Figure<Rational>;
  onEdit: (request: ManualEdit) => void;
}) {
  const scope = rateScope(props.slot)!;
  const [day, setDay] = createSignal(today());
  const latest = () => manualRate(props.slot);
  const pinned = () => latest()?.pinned ?? false;
  const value = () => {
    const rate = latest()?.rate;
    return rate ? fractionToPercent(rate.fraction) : "";
  };
  return (
    <span class="row-editor">
      <Apy figure={props.fallback} />
      <InlineEdit
        label="Annual rate"
        value={value()}
        suffix="%"
        pinned={pinned()}
        disabled={reviewMode}
        validate={validatePercent}
        onPinChange={(next) =>
          props.onEdit({ kind: "rate_pin", scope, pinned: next })
        }
        onCommit={(text) =>
          props.onEdit({
            kind: "rate",
            scope,
            rate: text.trim() ? apy(text) : null,
            effective_at: atMidnight(day()),
            pinned: pinned(),
            clear: false,
          })
        }
      />
      <label class="date-field">
        <span>Effective from</span>
        <input
          type="date"
          value={day()}
          max={today()}
          disabled={reviewMode}
          onChange={(e) => setDay(e.currentTarget.value)}
        />
      </label>
      <Show when={props.slot.value?.kind === "rates"}>
        <Button
          intent="ghost"
          size="sm"
          disabled={reviewMode}
          onClick={() =>
            props.onEdit({
              kind: "rate",
              scope,
              rate: null,
              effective_at: atMidnight(day()),
              pinned: false,
              clear: true,
            })
          }
        >
          Reset to source
        </Button>
      </Show>
    </span>
  );
}
/** Inline price editing in the reporting currency, dated and pinnable. */
function PriceEditor(props: {
  slot: EditSlot;
  selected?: { price: string } | null;
  currency: string;
  /** Hidden privacy mode masks monetary figures, including editable quotes. */
  hidden?: boolean;
  onEdit: (request: ManualEdit) => void;
}) {
  const target = priceTarget(props.slot)!;
  if (props.hidden)
    return (
      <span class="muted">
        Price editing is unavailable while Hidden privacy mode is on
      </span>
    );
  const [day, setDay] = createSignal(today());
  const latest = () => manualPrice(props.slot);
  const pinned = () => latest()?.pinned ?? false;
  const value = () => latest()?.price ?? props.selected?.price ?? "";
  const reset = () =>
    props.onEdit({
      kind: "price",
      asset: target.asset,
      quote: target.quote,
      price: null,
      effective_at: atMidnight(day()),
      pinned: false,
      clear: true,
    });
  return (
    <span class="row-editor">
      <span class="muted">{props.currency} price</span>
      <InlineEdit
        label="Current price"
        value={value()}
        pinned={pinned()}
        disabled={reviewMode}
        validate={validateDecimal}
        onPinChange={(next) =>
          props.onEdit({
            kind: "price_pin",
            asset: target.asset,
            quote: target.quote,
            pinned: next,
          })
        }
        onCommit={(text) =>
          props.onEdit({
            kind: "price",
            asset: target.asset,
            quote: target.quote,
            price: text.trim() || null,
            effective_at: atMidnight(day()),
            pinned: pinned(),
            clear: false,
          })
        }
      />
      <label class="date-field">
        <span>Effective from</span>
        <input
          type="date"
          value={day()}
          max={today()}
          disabled={reviewMode}
          onChange={(e) => setDay(e.currentTarget.value)}
        />
      </label>
      <Show when={props.slot.value?.kind === "prices"}>
        <Button intent="ghost" size="sm" disabled={reviewMode} onClick={reset}>
          Reset to source
        </Button>
      </Show>
    </span>
  );
}
/** Manual dust marking with an explicit reset to auto classification. */
function DustControls(props: {
  provider: string;
  asset: string;
  onEdit: (request: ManualEdit) => void;
}) {
  const items = [
    { value: "dust" as const, label: "Dust" },
    { value: "significant" as const, label: "Significant" },
    { value: "auto" as const, label: "Reset to auto" },
  ];
  return (
    <div class="edit-row" role="group" aria-label="Dust classification">
      <span class="muted">Dust classification</span>
      <For each={items}>
        {(item) => (
          <Button
            size="sm"
            intent="ghost"
            disabled={reviewMode}
            onClick={() =>
              props.onEdit({
                kind: "dust",
                provider: props.provider,
                asset: props.asset,
                mode: item.value,
              })
            }
          >
            {item.label}
          </Button>
        )}
      </For>
    </div>
  );
}
/** Portfolio-wide spam exclusion with a required reason and a restore control. */
function SpamControls(props: {
  slot: EditSlot;
  asset: string;
  onEdit: (request: ManualEdit) => void;
}) {
  const state = () => spamState(props.slot);
  const [reason, setReason] = createSignal(state().reason);
  return (
    <div class="edit-row" role="group" aria-label="Spam exclusion">
      <Show
        when={state().excluded}
        fallback={
          <>
            <span class="muted">Spam exclusion</span>
            <TextField
              label="Reason for excluding this asset"
              value={reason()}
              disabled={reviewMode}
              onInput={(e) => setReason(e.currentTarget.value)}
            />
            <Button
              size="sm"
              disabled={reviewMode || !reason().trim()}
              onClick={() =>
                props.onEdit({
                  kind: "spam",
                  asset: props.asset,
                  excluded: true,
                  reason: reason(),
                })
              }
            >
              Exclude as spam
            </Button>
          </>
        }
      >
        <span class="muted">Excluded as spam · {state().reason}</span>
        <Button
          size="sm"
          disabled={reviewMode}
          onClick={() =>
            props.onEdit({
              kind: "spam",
              asset: props.asset,
              excluded: false,
              reason: "",
            })
          }
        >
          Restore to tracked
        </Button>
      </Show>
    </div>
  );
}
/** Local subscription display name; the provider name stays available. */
function NameEditor(props: {
  slot: EditSlot;
  onEdit: (request: ManualEdit) => void;
}) {
  const current = () =>
    props.slot.value?.kind === "subscription_name"
      ? props.slot.value.value
      : "";
  return (
    <div class="edit-row">
      <span class="muted">Subscription name</span>
      <InlineEdit
        label="Subscription name"
        value={current()}
        disabled={reviewMode}
        validate={(value) => (value.trim() ? undefined : "Enter a name.")}
        onCommit={(value) =>
          props.onEdit({
            kind: "subscription_name",
            id: subscriptionId(props.slot),
            name: value.trim(),
          })
        }
      />
      <Show when={props.slot.value}>
        <Button
          size="sm"
          intent="ghost"
          disabled={reviewMode}
          onClick={() =>
            props.onEdit({
              kind: "subscription_name",
              id: subscriptionId(props.slot),
              name: null,
            })
          }
        >
          Use provider name
        </Button>
      </Show>
    </div>
  );
}
/** Local names, reparenting and group creation inside one provider. */
function AccountControls(props: {
  account: NodeAccount;
  onEdit: (request: ManualEdit) => void;
}) {
  // A local choice wins until it matches the current provider default again.
  const [chosen, setChosen] = createSignal<string | null>(null);
  const parent = () => chosen() ?? props.account.parent ?? "";
  const [groupName, setGroupName] = createSignal("");
  return (
    <section class="detail-block" aria-label="Local organization">
      <strong>Local organization</strong>
      <InlineEdit
        label="Local account name"
        value={props.account.local_label ?? props.account.source_label}
        disabled={reviewMode}
        validate={(value) => (value.trim() ? undefined : "Enter a name.")}
        onCommit={(value) =>
          props.onEdit({
            kind: "account_name",
            account: props.account.id,
            name: value.trim(),
          })
        }
      />
      <Button
        size="sm"
        intent="ghost"
        disabled={reviewMode}
        onClick={() =>
          props.onEdit({
            kind: "account_name",
            account: props.account.id,
            name: null,
          })
        }
      >
        Use source name
      </Button>
      <label class="date-field">
        <span>Move to…</span>
        <select
          aria-label="Move account to"
          value={parent()}
          disabled={reviewMode}
          onChange={(e) => {
            const value = e.currentTarget.value;
            setChosen(value || (props.account.parent ?? ""));
            props.onEdit({
              kind: "move",
              account: props.account.id,
              parent: value || null,
              source_default: false,
            });
          }}
        >
          <For each={props.account.parent_options}>
            {(option) => (
              <option value={option.id ?? ""}>
                {option.group ? "Group · " : ""}
                {option.name}
              </option>
            )}
          </For>
        </select>
      </label>
      <Button
        size="sm"
        intent="ghost"
        disabled={reviewMode}
        onClick={() => {
          setChosen(props.account.parent ?? "");
          props.onEdit({
            kind: "move",
            account: props.account.id,
            parent: null,
            source_default: true,
          });
        }}
      >
        Reset to provider default
      </Button>
      <label class="date-field">
        <span>New group under this node</span>
        <input
          type="text"
          aria-label="New group name"
          value={groupName()}
          disabled={reviewMode}
          onInput={(e) => setGroupName(e.currentTarget.value)}
        />
      </label>
      <Button
        size="sm"
        disabled={reviewMode || !groupName().trim()}
        onClick={() => {
          props.onEdit({
            kind: "group",
            provider: props.account.provider_id,
            name: groupName().trim(),
            parent: props.account.id,
          });
          setGroupName("");
        }}
      >
        Create group
      </Button>
    </section>
  );
}
function describeValue(value: LocalValue | null): string {
  switch (value?.kind) {
    case "account":
      return `${value.value.local_label ?? "source name"} · ${
        value.value.parent_is_custom
          ? (value.value.parent ?? "top level")
          : "provider default"
      }`;
    case "subscription_name":
      return value.value;
    case "rates":
      return value.value.length
        ? `${fractionToPercent(value.value.at(-1)!.rate?.fraction ?? "0")}%${
            value.value.at(-1)!.pinned ? " · pinned" : ""
          }`
        : "no manual rate";
    case "prices":
      return value.value.length
        ? `${value.value.at(-1)!.price}${value.value.at(-1)!.pinned ? " · pinned" : ""}`
        : "no manual price";
    case "spam":
      return value.value.excluded
        ? `excluded · ${value.value.reason}`
        : "restored to tracked";
    case "dust":
      return value.value;
    default:
      return "no local value";
  }
}
function describeTarget(
  target: LocalTarget,
  names: Map<string, string>,
  entries: Map<string, string>,
): string {
  const label = (id: string, map: Map<string, string>) => map.get(id) ?? id;
  switch (target.kind) {
    case "account":
      return `Local name or grouping · ${label(target.id, names)}`;
    case "subscription":
      return `Subscription name · ${target.id}`;
    case "rate": {
      const scope = target.scope;
      const where =
        scope.kind === "subscription"
          ? `subscription ${scope.id}`
          : scope.kind === "product"
            ? `product ${scope.id}`
            : scope.kind === "borrowing_position"
              ? `position ${scope.id}`
              : scope.kind === "provider_asset"
                ? `${label(scope.provider, entries)} · ${scope.asset}`
                : scope.kind === "account_asset"
                  ? `${label(scope.account, names)} · ${scope.asset}`
                  : label(scope.id, entries);
      return `Manual rate · ${where}`;
    }
    case "price":
      return `Manual price · ${target.asset} in ${target.quote}`;
    case "spam":
      return `Spam decision · ${target.asset}`;
    case "dust":
      return `Dust classification · ${label(target.provider, entries)} / ${target.asset}`;
  }
}
/** Ordered action log with prior/resulting state and working undo links. */
function HistoryPane(props: { data: Snapshot; onUndo: (id: string) => void }) {
  const revisions = new Map(
    props.data.edits.local_state.map(([target, record]) => [
      targetKey(target),
      record.revision,
    ]),
  );
  const names = new Map(
    props.data.edits.accounts.map((a) => [a.account.id, a.account.label]),
  );
  const entries = new Map(
    props.data.assets
      .concat(props.data.providers)
      .map((entry) => [JSON.stringify(entry.selection), entry.name] as const),
  );
  // Reactive: a committed edit or Undo replaces the log inside the same view.
  const actions = createMemo(() => props.data.edits.actions);
  return (
    <Show
      when={actions().length}
      fallback={
        <StateMessage
          kind="empty"
          title="No local changes yet"
          description="Manual rates, prices, names, grouping and dust or spam decisions are listed here with their prior and resulting state."
        />
      }
    >
      <ul class="history">
        <For each={actions()}>
          {(action) => (
            <li>
              <div class="history-head">
                <strong>{describeTarget(action.target, names, entries)}</strong>
                <time datetime={action.at}>
                  {action.at.replace("T", " ").slice(0, 16)}
                </time>
              </div>
              <div class="muted">
                {describeValue(action.before)} → {describeValue(action.after)}
              </div>
              <div class="history-actions">
                <Show when={action.undone}>
                  <Badge tone="warning">Undone</Badge>
                </Show>
                <Show when={action.undo_of}>
                  <Badge tone="accent">Compensating change</Badge>
                </Show>
                <Show
                  when={
                    !action.undone &&
                    !action.undo_of &&
                    revisions.get(targetKey(action.target)) === action.revision
                  }
                >
                  <Button
                    size="sm"
                    disabled={reviewMode}
                    onClick={() => props.onUndo(action.id)}
                  >
                    Undo
                  </Button>
                </Show>
              </div>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}
