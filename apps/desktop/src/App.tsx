import {
  For,
  Show,
  batch,
  createEffect,
  createMemo,
  createSignal,
  onMount,
} from "solid-js";
import {
  ThemeRoot,
  PrivacyProvider,
  Button,
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
} from "@money-tracker/design-system";
import type {
  AllocationRow,
  CurrencyFigure,
  Rational,
} from "@money-tracker/design-system";
import { api, reviewMode } from "./api";
import { defaultSettings } from "./contracts";
import type {
  Startup,
  Settings,
  Snapshot,
  Selection,
  Summary,
  Figure,
  MoneyValue,
  Tree,
  Details,
  Issue,
  Entry,
} from "./contracts";
const figure = (f: Figure<MoneyValue>): CurrencyFigure =>
  f.status === "known" || f.status === "partial"
    ? { status: f.status, value: f.value.amount }
    : { status: f.status };
const key = (s: Selection) => JSON.stringify(s);
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
  const [panel, setPanel] = createSignal<Details>();
  const [options, setOptions] = createSignal(false);
  const [draft, setDraft] = createSignal<Settings>(defaultSettings);
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
    setPanel(undefined);
    setSelection(s);
  }
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
  async function openDetails(id: string, trigger: HTMLButtonElement) {
    const snapshot = data();
    if (!snapshot) return;
    const request = ++detailRequest;
    returnFocus = trigger;
    setOptions(false);
    try {
      const d = await api.details(snapshot.snapshot_id, selection(), id);
      if (request === detailRequest) setPanel(d);
    } catch {
      if (request === detailRequest) failure();
    }
  }
  function entryWarnings(e: Entry, trigger: HTMLElement) {
    detailRequest++;
    returnFocus = trigger;
    setOptions(false);
    setPanel({
      name: e.name,
      summary: e.summary,
      issues: e.issues,
      holdings: [],
    });
  }
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
        rate: <Apy figure={n.summary.current_apy} />,
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
      setOptions(false);
    } catch {
      props.setSettings(old);
      failure();
    } finally {
      setBusy(false);
    }
  }
  function openOptions(trigger: HTMLElement) {
    detailRequest++;
    setPanel(undefined);
    returnFocus = trigger;
    setDraft({ ...props.settings });
    setOptions(true);
  }
  const money = (f: Figure<MoneyValue>) => (
    <MoneyFigure
      figure={figure(f)}
      currency={props.settings.reporting_currency}
    />
  );
  return (
    <div class={`desktop ${panel() || options() ? "panel-open" : ""}`}>
      <header class="app-header">
        <h1>Money Tracker</h1>
        <div class="tools">
          <label class="muted">
            Privacy{" "}
            <select
              aria-label="Privacy mode"
              value={props.settings.privacy_mode}
              disabled={busy() || !data()}
              onChange={(e) =>
                void save({
                  ...props.settings,
                  privacy_mode: e.currentTarget
                    .value as Settings["privacy_mode"],
                })
              }
            >
              <option value="normal">Normal</option>
              <option value="hidden">Hidden</option>
              <option value="scaled">Scaled</option>
            </select>
          </label>
          <Button
            disabled={!data()}
            onClick={(e) => openOptions(e.currentTarget)}
          >
            Settings
          </Button>
          <Button
            intent="ghost"
            disabled={reviewMode}
            onClick={() => void api.quit().catch(failure)}
          >
            Quit
          </Button>
        </div>
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
              <NavigationEntry
                name="Overview"
                selected={selection().kind === "overview"}
                onSelect={() => select({ kind: "overview" })}
              />
              <For each={entries()}>
                {(entry) => (
                  <NavigationEntry
                    name={entry.name}
                    secondary={entry.ticker ?? undefined}
                    selected={key(entry.selection) === key(selection())}
                    onSelect={() => select(entry.selection)}
                    value={money(entry.summary.current_value)}
                    rate={<Apy figure={entry.summary.current_apy} />}
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
                  selected={selection().kind === "dust"}
                  onSelect={() => select({ kind: "dust" })}
                  value={money(data()!.dust.summary.current_value)}
                  rate={<Apy figure={data()!.dust.summary.current_apy} />}
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
            </nav>
            <main class="main-pane">
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
                    <StateMessage kind="loading" title="Reading allocation…" />
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
                      onDetails={(id, trigger) => void openDetails(id, trigger)}
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
              <p class="footnote">
                Local snapshot · {new Date(data()!.cutoff).toLocaleString()}
                <br />
                Current-rate annual estimate; principal and available prices
                held constant. Includes evidenced borrowing costs. Additional
                fees and historical reward-currency performance are not
                estimated. Unknown inputs remain unknown. Provider refresh
                follows in a later stage.
              </p>
            </main>
          </div>
        </Show>
      </Show>
      <DetailsPanel
        open={!!panel() || options()}
        onOpenChange={(open) => {
          if (!open) {
            detailRequest++;
            setPanel(undefined);
            setOptions(false);
          }
        }}
        title={options() ? "Settings" : (panel()?.name ?? "Details")}
        returnFocus={() => returnFocus}
      >
        <Show
          when={options()}
          fallback={
            <Show when={panel()}>
              {(d) => (
                <>
                  <Warnings issues={d().issues} />
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
                      <p class="source">
                        Original account: {d().original_name}
                      </p>
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
                          <Badge tone="danger">
                            Liability · borrowed amount
                          </Badge>
                        </Show>
                        <Quantity
                          units={holding.quantity.units}
                          scale={holding.quantity.scale}
                          asset={holding.quantity.asset_id}
                        />
                        <p>Current value: {money(holding.value)}</p>
                        <p>
                          Earnings per year: {money(holding.annual_earnings)}
                        </p>
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
                      </section>
                    )}
                  </For>
                </>
              )}
            </Show>
          }
        >
          <form
            class="settings"
            onSubmit={(e) => {
              e.preventDefault();
              void save(draft());
            }}
          >
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
              label="Privacy scale (fraction)"
              disabled={reviewMode}
              value={draft().privacy_scale}
              onInput={(e) =>
                setDraft({ ...draft(), privacy_scale: e.currentTarget.value })
              }
            />
            <TextField
              label="Dust threshold in reporting currency"
              disabled={reviewMode}
              value={draft().dust_threshold}
              onInput={(e) =>
                setDraft({ ...draft(), dust_threshold: e.currentTarget.value })
              }
            />
            <p class="muted">
              Refresh preferences are saved for the later scheduler; this stage
              only reads cached data.
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
                  refresh_interval_seconds: Number(e.currentTarget.value),
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
                Refresh on open (when scheduler is implemented)
              </span>
            </label>
            <Button type="submit" disabled={busy()}>
              Save settings
            </Button>
          </form>
        </Show>
      </DetailsPanel>
    </div>
  );
}
