import { render } from "solid-js/web";
import { For, Show, createMemo, createSignal } from "solid-js";
import {
  ThemeRoot,
  PrivacyProvider,
  Badge,
  Button,
  Icon,
  GroupingTabs,
  SearchField,
  TextField,
  NavigationEntry,
  SummaryValue,
  AllocationTable,
  DetailsPanel,
  Money,
  MoneyFigure,
  Quantity,
  Rate,
  InlineEdit,
  StateMessage,
  WarningButton,
} from "@money-tracker/design-system";
import type {
  ThemeMode,
  Privacy,
  AllocationRow,
} from "@money-tracker/design-system";
import "@money-tracker/design-system/styles.css";
import fixture from "../../../docs/spec/fixtures/current-portfolio.json";
import "./showcase.css";

function App() {
  const [theme, setTheme] = createSignal<ThemeMode>("system");
  const [privacy, setPrivacy] = createSignal<Privacy["mode"]>("normal");
  const [scale, setScale] = createSignal("0.1");
  function Body() {
    const [group, setGroup] = createSignal("asset");
    const [search, setSearch] = createSignal("");
    const [selection, setSelection] = createSignal("overview");
    const [expanded, setExpanded] = createSignal<string[]>(["nexo"]);
    const [detail, setDetail] = createSignal<string>();
    let returnFocus: HTMLButtonElement | undefined;
    const [apy, setApy] = createSignal("5");
    const [pinned, setPinned] = createSignal(false);
    const [undo, setUndo] = createSignal<{ apy: string; pinned: boolean }>();
    const edit = (value: string) => {
      setUndo({ apy: apy(), pinned: pinned() });
      setApy(value);
    };
    const pin = (value: boolean) => {
      setUndo({ apy: apy(), pinned: pinned() });
      setPinned(value);
    };
    const open = (id: string, trigger: HTMLButtonElement) => {
      returnFocus = trigger;
      setDetail(id);
    };
    const navigate = (value: string) => {
      setSelection(value);
      setDetail(undefined);
    };
    const navs = createMemo(() =>
      group() === "asset"
        ? [
            {
              id: "eth",
              name: "Ethereum",
              ticker: "ETH",
              value: "1000",
              rate: "0.04",
              units: "100000000000000000000",
              scale: 18,
            },
          ]
        : [
            {
              id: "nexo",
              name: "Nexo",
              ticker: "Provider",
              value: "750",
              rate: { numerator: "4", denominator: "75" },
              units: undefined,
              scale: 18,
            },
            {
              id: "kraken",
              name: "Kraken",
              ticker: "Provider",
              value: "250",
              rate: "0",
              units: undefined,
              scale: 18,
            },
          ],
    );
    const filtered = createMemo(() =>
      navs().filter((n) =>
        (n.name + " " + n.ticker)
          .toLowerCase()
          .includes(search().toLowerCase()),
      ),
    );
    const expand = (id: string) =>
      setExpanded((ids) =>
        ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id],
      );
    const rows = createMemo<AllocationRow[]>(() => {
      const nexo = selection() !== "kraken";
      const kraken = selection() !== "nexo";
      const list: AllocationRow[] = [];
      if (nexo) {
        list.push({
          id: "nexo",
          name: "Nexo",
          depth: 0,
          hasChildren: true,
          expanded: expanded().includes("nexo"),
          secondary: "Synthetic provider account",
          value: <Money value="750" />,
          rate: <Rate value={{ numerator: "4", denominator: "75" }} />,
          annual: <Money value="40" />,
        });
        if (expanded().includes("nexo")) {
          list.push({
            id: "product",
            name: "Fixed-term earn",
            depth: 1,
            hasChildren: true,
            expanded: expanded().includes("product"),
            secondary: "Two subscriptions · separate rates",
            value: <Money value="750" />,
            rate: <Rate value={{ numerator: "4", denominator: "75" }} />,
            annual: <Money value="40" />,
          });
          if (expanded().includes("product"))
            list.push(
              {
                id: "term-one",
                name: "ETH fixed term · October",
                depth: 2,
                secondary: (
                  <Quantity
                    units="50000000000000000000"
                    scale={18}
                    asset="ETH"
                  />
                ),
                value: <Money value="500" />,
                rate: <Rate value="0.05" />,
                annual: <Money value="25" />,
              },
              {
                id: "term-two",
                name: "ETH fixed term · November",
                depth: 2,
                secondary: (
                  <Quantity
                    units="25000000000000000000"
                    scale={18}
                    asset="ETH"
                  />
                ),
                value: <Money value="250" />,
                rate: <Rate value="0.06" />,
                annual: <Money value="15" />,
              },
            );
        }
      }
      if (kraken)
        list.push({
          id: "kraken",
          name: "Kraken spot",
          depth: 0,
          secondary: (
            <Quantity units="25000000000000000000" scale={18} asset="ETH" />
          ),
          value: <Money value="250" />,
          rate: <Rate value="0" />,
          annual: <Money value="0" />,
        });
      return list;
    });
    const title = createMemo(() =>
      selection() === "overview"
        ? "Overview"
        : (navs().find((n) => n.id === selection())?.name ?? "Ethereum"),
    );
    const stateRows: AllocationRow[] = [
      {
        id: "unknown-rate",
        name: "Rate not provided",
        depth: 0,
        value: <Money value="100" />,
        rate: <span class="muted">Unknown</span>,
        annual: <MoneyFigure figure={{ status: "unknown" }} />,
        warning: {
          label: "Explain missing rate",
          onClick: (e) => open("warning", e.currentTarget),
        },
      },
      {
        id: "unpriced",
        name: "Unpriced holding",
        depth: 0,
        secondary: <Quantity units="1" scale={18} asset="TOKEN" />,
        value: <MoneyFigure figure={{ status: "unknown" }} />,
        rate: <Rate value="0.05" />,
        annual: <MoneyFigure figure={{ status: "unknown" }} />,
        warning: {
          label: "Explain missing price",
          onClick: (e) => open("warning", e.currentTarget),
        },
      },
      {
        id: "debt",
        name: "Credit line",
        depth: 0,
        liability: true,
        value: <Money value="100" />,
        rate: <Rate value="0.1" />,
        annual: <Money value="-10" negative />,
      },
    ];
    return (
      <>
        <header class="topbar">
          <div class="brand">
            <span class="brand-mark">
              <Icon name="wallet" size={22} />
            </span>
            <div>
              <strong>Money Tracker</strong>
              <span class="brand-subtitle">
                Design system · synthetic showcase
              </span>
            </div>
          </div>
          <div class="toolbar">
            <label>
              Theme
              <select
                aria-label="Theme"
                value={theme()}
                onChange={(e) => setTheme(e.currentTarget.value as ThemeMode)}
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </label>
            <label>
              Privacy
              <select
                aria-label="Privacy"
                value={privacy()}
                onChange={(e) =>
                  setPrivacy(e.currentTarget.value as Privacy["mode"])
                }
              >
                <option value="normal">Normal</option>
                <option value="hidden">Hide amounts</option>
                <option value="scaled">Scale amounts</option>
              </select>
            </label>
            <Show when={privacy() === "scaled"}>
              <label>
                Display scale
                <select
                  aria-label="Display scale"
                  value={scale()}
                  onChange={(e) => setScale(e.currentTarget.value)}
                >
                  <option value="0.1">10%</option>
                  <option value="0.25">25%</option>
                  <option value="0.01">1%</option>
                </select>
              </label>
            </Show>
          </div>
        </header>
        <div class="synthetic-note">
          <Badge tone="accent">Synthetic data</Badge>
          <span>
            No accounts connected. No transactions or settings are saved.
          </span>
          <Show when={privacy() === "scaled"}>
            <strong>Amounts are scaled; rates are unchanged.</strong>
          </Show>
        </div>
        <main class="showcase-shell" data-panel-open={!!detail()}>
          <aside class="sidebar" aria-label="Portfolio navigation">
            <div class="nav-heading">PORTFOLIO</div>
            <NavigationEntry
              name="Overview"
              selected={selection() === "overview"}
              onSelect={() => navigate("overview")}
              icon={<Icon name="wallet" />}
            />
            <div class="nav-divider" />
            <GroupingTabs
              label="Navigation grouping"
              value={group()}
              onChange={(v) => {
                setGroup(v);
                setSearch("");
                navigate("overview");
              }}
              items={[
                { value: "asset", label: "By asset" },
                { value: "provider", label: "By provider" },
              ]}
            >
              <SearchField value={search()} onChange={setSearch} />
              <nav aria-label={group() === "asset" ? "Assets" : "Providers"}>
                <For each={filtered()}>
                  {(n) => (
                    <NavigationEntry
                      name={n.name}
                      selected={selection() === n.id}
                      onSelect={() => navigate(n.id)}
                      secondary={
                        n.units ? (
                          <Quantity
                            units={n.units}
                            scale={n.scale}
                            asset={n.ticker}
                          />
                        ) : (
                          n.ticker
                        )
                      }
                      value={<Money value={n.value} />}
                      rate={<Rate value={n.rate} />}
                    />
                  )}
                </For>
                <Show when={filtered().length === 0}>
                  <p class="empty-search">No matching assets or providers.</p>
                </Show>
              </nav>
            </GroupingTabs>
            <div class="sidebar-footer">
              <span class="status-dot" />
              Isolated component preview
            </div>
          </aside>
          <section class="main-pane">
            <div class="section-header">
              <div>
                <p class="eyebrow">CURRENT HOLDINGS · DEMONSTRATION</p>
                <h1>{title()}</h1>
                <p class="muted">
                  A calm, clear view of balances and current earnings.
                </p>
              </div>
              <Badge>Example fixture</Badge>
            </div>
            <section
              class="summary-card"
              aria-label="Synthetic portfolio summary"
            >
              <SummaryValue
                primary
                label="Current value"
                figure={{
                  status: "known",
                  value:
                    selection() === "nexo"
                      ? "750"
                      : selection() === "kraken"
                        ? "250"
                        : fixture.overview.current_value.value.amount,
                }}
                description="USD · fixed example prices"
              />
              <SummaryValue
                label="Currently earning / year"
                figure={{
                  status: "known",
                  value:
                    selection() === "kraken"
                      ? "0"
                      : fixture.overview.net_annual_earnings.value.amount,
                }}
                description="Current immediate rate. Historical reward adjustment unavailable."
              />
            </section>
            <Show when={selection() !== "overview"}>
              <div class="section-label">
                <h2>Allocation</h2>
                <span class="muted">
                  Name opens details · arrow expands children
                </span>
              </div>
              <AllocationTable
                caption={`${title()} allocation · synthetic fixture`}
                rows={rows()}
                onExpand={expand}
                onDetails={open}
              />
            </Show>
            <section class="specimens">
              <div class="section-label">
                <div>
                  <p class="eyebrow">COMPONENT SPECIMENS</p>
                  <h2>States, controls & details</h2>
                </div>
              </div>
              <p class="muted small">
                These examples are outside the fixture totals. Field edits only
                demonstrate component state.
              </p>
              <div class="specimen-grid">
                <article class="specimen-card">
                  <h3>Inline rate & pin</h3>
                  <p class="muted small">
                    A subscription-specific rate, with validation and a local
                    demo Undo.
                  </p>
                  <InlineEdit
                    label="Demo subscription APY"
                    value={apy()}
                    suffix="%"
                    onCommit={edit}
                    validate={(v) =>
                      /^\d+(\.\d+)?$/.test(v)
                        ? undefined
                        : "Enter a nonnegative decimal percentage"
                    }
                    pinned={pinned()}
                    onPinChange={pin}
                  />
                  <TextField
                    label="Read-only example name"
                    value="Synthetic account"
                    readOnly
                    aria-describedby="name-help"
                  />
                  <p id="name-help" class="muted small">
                    Synthetic names are not saved.
                  </p>
                  <div class="specimen-actions">
                    <Button
                      size="sm"
                      disabled={!undo()}
                      onClick={() => {
                        const old = undo();
                        if (old) {
                          setApy(old.apy);
                          setPinned(old.pinned);
                          setUndo(undefined);
                        }
                      }}
                    >
                      <Icon name="undo" />
                      Undo demo edit
                    </Button>
                    <Show when={pinned()}>
                      <Badge tone="accent">Pinned</Badge>
                    </Show>
                  </div>
                </article>
                <article class="specimen-card">
                  <h3>Partial & unknown</h3>
                  <div class="figure-line">
                    <span>Known earnings subtotal</span>
                    <MoneyFigure figure={{ status: "partial", value: "25" }} />
                  </div>
                  <div class="figure-line">
                    <span>Overall APY</span>
                    <span class="muted">Unknown</span>
                  </div>
                  <div class="figure-line">
                    <span>Zero earnings</span>
                    <Money value="0" />
                  </div>
                  <div class="figure-line">
                    <span>Not applicable</span>
                    <MoneyFigure figure={{ status: "not_applicable" }} />
                  </div>
                  <WarningButton
                    label="Explain partial earnings"
                    onClick={(e) => open("warning", e.currentTarget)}
                  />
                </article>
              </div>
              <AllocationTable
                caption="Missing-data and liability specimens · excluded from fixture totals"
                rows={stateRows}
                onExpand={() => {}}
                onDetails={open}
              />
              <div class="state-grid">
                <StateMessage
                  kind="empty"
                  title="No holdings yet"
                  description="A new portfolio can start empty."
                />
                <StateMessage
                  kind="offline"
                  title="Viewing saved data"
                  description="Offline data retains its source timestamps."
                />
                <StateMessage
                  kind="loading"
                  title="Reading local data…"
                  description="Loading does not replace figures with zero."
                />
                <StateMessage
                  kind="error"
                  title="Source could not be read"
                  description="Keep previous data and show the affected source."
                  action={<Button size="sm">View source details</Button>}
                />
              </div>
              <div class="button-specimen">
                <Button intent="primary">Primary action</Button>
                <Button>Secondary action</Button>
                <Button intent="ghost">Quiet action</Button>
                <Button intent="danger">Destructive action</Button>
                <Button disabled>Disabled action</Button>
              </div>
            </section>
            <footer class="page-footer">
              SolidJS · Panda CSS · Ark UI{" "}
              <span>Stage 5 · component package only</span>
            </footer>
          </section>
        </main>
        <DetailsPanel
          open={!!detail()}
          onOpenChange={(v) => {
            if (!v) setDetail(undefined);
          }}
          title={
            detail() === "warning"
              ? "Missing-data warning"
              : detail() === "term-one"
                ? "ETH fixed term · October"
                : detail() === "term-two"
                  ? "ETH fixed term · November"
                  : "Allocation details"
          }
          description="Synthetic source metadata · read-only preview"
          returnFocus={() => returnFocus}
        >
          <Show
            when={detail() === "warning"}
            fallback={
              <>
                <Show
                  when={detail() === "term-one" || detail() === "term-two"}
                  fallback={
                    <StateMessage
                      kind="empty"
                      title="No subscription metadata in this specimen"
                      description="This selection does not establish a fixed-term subscription. The connected app will supply its source details."
                    />
                  }
                >
                  <Badge tone="accent">Locked · example term</Badge>
                  <dl class="detail-list">
                    <div>
                      <dt>Provider name</dt>
                      <dd>Nexo</dd>
                    </div>
                    <div>
                      <dt>Original reference</dt>
                      <dd>
                        {detail() === "term-one"
                          ? "synthetic:term-october"
                          : "synthetic:term-november"}
                      </dd>
                    </div>
                    <div>
                      <dt>Current quantity</dt>
                      <dd>
                        <Quantity
                          units={
                            detail() === "term-one"
                              ? "50000000000000000000"
                              : "25000000000000000000"
                          }
                          scale={18}
                          asset="ETH"
                        />
                      </dd>
                    </div>
                    <div>
                      <dt>Current value</dt>
                      <dd>
                        <Money
                          value={detail() === "term-one" ? "500" : "250"}
                        />
                      </dd>
                    </div>
                    <div>
                      <dt>Annual rate</dt>
                      <dd>
                        <Rate
                          value={detail() === "term-one" ? "0.05" : "0.06"}
                        />
                      </dd>
                    </div>
                    <div>
                      <dt>Maturity</dt>
                      <dd>
                        {detail() === "term-one"
                          ? "25 October 2026"
                          : "25 November 2026"}
                      </dd>
                    </div>
                    <div>
                      <dt>Auto-renew</dt>
                      <dd>On</dd>
                    </div>
                  </dl>
                  <p class="muted small">
                    Full exact quantities respect the selected privacy mode.
                    This panel keeps the table visible.
                  </p>
                </Show>
              </>
            }
          >
            <Badge tone="warning">Partial coverage</Badge>
            <h3>What is missing</h3>
            <p>
              Some allocations have no evidenced rate or exact quote. Unknown
              values are not zero.
            </p>
            <h3>What is withheld</h3>
            <p>
              Overall APY is unknown until all rates and prices are available.
              Known annual earnings can be shown as a labelled partial subtotal.
            </p>
            <h3>Historical figures</h3>
            <p>
              Balance verification does not establish complete history.
              Historical earnings require separate reconciled coverage.
            </p>
            <p class="muted small">
              Manual edits can resolve missing rates or prices; they do not
              clear unrelated reconciliation issues.
            </p>
          </Show>
        </DetailsPanel>
      </>
    );
  }
  return (
    <ThemeRoot mode={theme()} class="showcase-root">
      <PrivacyProvider value={{ mode: privacy(), scale: scale() }}>
        <Body />
      </PrivacyProvider>
    </ThemeRoot>
  );
}
render(() => <App />, document.getElementById("root")!);
