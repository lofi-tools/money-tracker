import { For, Show, createSignal } from "solid-js";
import type { JSX } from "solid-js";
import {
  ThemeRoot,
  PrivacyProvider,
  Button,
  IconButton,
  Icon,
  IconTile,
  Surface,
  ScreenHeader,
  ChipGroup,
  StatRow,
  ListRow,
  TimePill,
  TrendChart,
  PaginationDots,
  TextField,
  SearchField,
  PinButton,
  InlineEdit,
  GroupingTabs,
  NavigationEntry,
  AllocationTable,
  DetailsPanel,
  StateMessage,
  Badge,
  WarningButton,
  Money,
  Quantity,
  Rate,
  MoneyFigure,
  SummaryValue,
} from "@money-tracker/design-system";
import type {
  ThemeMode,
  Privacy,
  AllocationRow,
} from "@money-tracker/design-system";
import { usePrivacy } from "@money-tracker/design-system";
function Spec(props: {
  title: string;
  kind: string;
  note?: string;
  wide?: boolean;
  children: JSX.Element;
}) {
  return (
    <Surface class={`spec ${props.wide ? "wide" : ""}`}>
      <header class="spec-head">
        <h3>{props.title}</h3>
        <Badge tone="accent">{props.kind}</Badge>
      </header>
      <div class="spec-stage">{props.children}</div>
      <Show when={props.note}>
        <p class="spec-note">{props.note}</p>
      </Show>
    </Surface>
  );
}
export function Catalog() {
  const [theme, setTheme] = createSignal<ThemeMode>("dark");
  const [privacy, setPrivacy] = createSignal<Privacy["mode"]>("normal");
  return (
    <ThemeRoot mode={theme()} class="catalog-root">
      <PrivacyProvider value={{ mode: privacy(), scale: "0.1" }}>
        <CatalogBody
          theme={theme()}
          setTheme={setTheme}
          privacy={privacy()}
          setPrivacy={setPrivacy}
        />
      </PrivacyProvider>
    </ThemeRoot>
  );
}
function CatalogBody(props: {
  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
  privacy: Privacy["mode"];
  setPrivacy: (privacy: Privacy["mode"]) => void;
}) {
  const privacy = usePrivacy();
  const [range, setRange] = createSignal("month");
  const [segment, setSegment] = createSignal("income");
  const [page, setPage] = createSignal(0);
  const [search, setSearch] = createSignal("");
  const [name, setName] = createSignal("My wallet");
  const [rate, setRate] = createSignal("5");
  const [pin, setPin] = createSignal(false);
  const [expanded, setExpanded] = createSignal(true);
  const [nav, setNav] = createSignal("overview");
  const [details, setDetails] = createSignal(false);
  let trigger: HTMLElement | undefined;
  const open = (element: HTMLButtonElement) => {
    trigger = element;
    setDetails(true);
  };
  const ranges = [
    { value: "week", label: "Week" },
    { value: "month", label: "Month" },
    { value: "year", label: "Year" },
    { value: "all", label: "All time" },
  ];
  const chart = () =>
    range() === "week"
      ? "M0 80 C12 80 16 50 25 62 S42 35 55 46 S82 15 100 20"
      : range() === "year"
        ? "M0 85 C20 86 20 65 35 60 S60 75 73 40 S88 25 100 8"
        : "M0 82 C12 84 18 55 28 65 S43 82 54 52 S70 67 78 31 S90 39 100 16";
  const rows = (): AllocationRow[] => [
    {
      id: "provider",
      name: "Example provider",
      depth: 0,
      hasChildren: true,
      expanded: expanded(),
      value: <Money value="1000" />,
      rate: <Rate value="0.04" />,
      annual: <Money value="40" />,
    },
    ...(expanded()
      ? [
          {
            id: "term",
            name: "Fixed term · October",
            depth: 1,
            value: <Money value="500" />,
            rate: <Rate value="0.05" />,
            annual: <Money value="25" />,
            secondary: "Locked · matures 31 October · auto-renew on",
          },
          {
            id: "loan",
            name: "Credit line",
            depth: 1,
            liability: true,
            value: <Money value="100" />,
            rate: <Rate value="0.08" />,
            annual: <Money value="-8" />,
            warning: {
              label: "Synthetic borrowing warning",
              onClick: (e: MouseEvent & { currentTarget: HTMLButtonElement }) =>
                open(e.currentTarget),
            },
          },
        ]
      : []),
  ];
  const colors = [
    ["Canvas", "canvas", "#6e6e6a"],
    ["Surface", "surface", "#191918"],
    ["Inset", "inset", "#141413"],
    ["Surface 2", "subtle", "#232321"],
    ["Surface 3", "hover", "#2c2c2a"],
    ["Sage", "accent", "#b7d3bf"],
    ["Ink", "text", "#f4f4f1"],
    ["Muted", "muted", "#a3a39e"],
  ];
  return (
    <main class="catalog-page">
      <header class="catalog-hero">
        <div>
          <span class="hero-tag">Money Tracker · Sage UI</span>
          <h1>
            Design system catalog
            <br />
            &amp; portfolio showcase
          </h1>
          <p>
            Warm charcoal. Soft sage. Every public component, its states and a
            composed financial interface.
          </p>
          <Badge tone="accent">Synthetic data · no accounts connected</Badge>
        </div>
        <div class="hero-actions">
          <nav aria-label="Catalog sections">
            <a href="#tokens">01 · Tokens</a>
            <a href="#components">02 · Components</a>
            <a href="#dashboard">03 · Dashboard</a>
            <a href="/portfolio">Portfolio flow ↗</a>
          </nav>
          <div class="catalog-options">
            <label>
              Theme
              <select
                aria-label="Catalog theme"
                value={props.theme}
                onChange={(e) =>
                  props.setTheme(e.currentTarget.value as ThemeMode)
                }
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="system">System</option>
              </select>
            </label>
            <label>
              Privacy
              <select
                aria-label="Catalog privacy"
                value={props.privacy}
                onChange={(e) =>
                  props.setPrivacy(e.currentTarget.value as Privacy["mode"])
                }
              >
                <option value="normal">Normal</option>
                <option value="hidden">Hidden</option>
                <option value="scaled">Scaled · 10%</option>
              </select>
            </label>
          </div>
        </div>
      </header>
      <section id="tokens" aria-labelledby="tokens-title">
        <div class="catalog-section-heading">
          <div>
            <span>01 · Foundations</span>
            <h2 id="tokens-title">Design tokens</h2>
          </div>
          <p>
            Manrope, rounded geometry, layered surfaces and a quiet sage glow.
          </p>
        </div>
        <div class="token-grid">
          <Surface class="color-card">
            <h3>Color</h3>
            <p class="spec-note">
              Theme-aware semantic tokens · dark reference values shown
            </p>
            <div class="swatch-grid">
              <For each={colors}>
                {([label, token, hex]) => (
                  <div>
                    <div
                      class="swatch"
                      style={{ background: `var(--mt-colors-${token})` }}
                    />
                    <strong>{label}</strong>
                    <code>{hex.toUpperCase()}</code>
                  </div>
                )}
              </For>
            </div>
          </Surface>
          <Surface>
            <h3>Typography</h3>
            <p class="spec-note">
              Manrope Variable · locally bundled · 400–800
            </p>
            <div class="type-sample display">
              <Money value="8926.07" />
              <small>Display / 800</small>
            </div>
            <div class="type-sample title">
              Your balance<small>Title / 700</small>
            </div>
            <div class="type-sample body">
              Current allocations<small>Body / 500</small>
            </div>
            <div class="type-sample caption">
              Source evidence and details<small>Caption / 500</small>
            </div>
            <div class="micro-label">Portfolio · current snapshot</div>
          </Surface>
          <Surface>
            <h3>Radius &amp; shape</h3>
            <p class="spec-note">
              Controls 4 · tiles 6 · cards 8 · panels 10 · frames 12
            </p>
            <div class="radius-row">
              <For
                each={[
                  ["Tile", "6px"],
                  ["Card", "8px"],
                  ["Panel", "10px"],
                  ["Frame", "12px"],
                ]}
              >
                {([label, radius]) => (
                  <div>
                    <div
                      class="radius-box"
                      style={{ "border-radius": radius }}
                    />
                    <span>{label}</span>
                  </div>
                )}
              </For>
            </div>
          </Surface>
          <Surface tone="glow">
            <h3>Elevation &amp; glow</h3>
            <p class="spec-note">
              Soft radial sage light; warm, subtle shadows.
            </p>
            <div class="glow-stage">
              <IconTile size="lg">
                <Icon name="leaf" size={30} />
              </IconTile>
              <span>Calm focus. Clear hierarchy.</span>
            </div>
          </Surface>
        </div>
      </section>
      <section id="components" aria-labelledby="components-title">
        <div class="catalog-section-heading">
          <div>
            <span>02 · Reusable pieces</span>
            <h2 id="components-title">Component library</h2>
          </div>
          <p>
            Live controls, financial coverage states and accessible
            interactions. All figures are examples.
          </p>
        </div>
        <div class="catalog-spec-grid">
          <Spec title="Buttons & icon actions" kind="Atoms">
            <div class="wrap">
              <Button intent="primary">Primary action</Button>
              <Button>Secondary</Button>
              <Button intent="ghost">Ghost</Button>
              <Button intent="danger">Danger</Button>
              <Button disabled>Disabled</Button>
              <Button size="sm">Small</Button>
              <IconButton
                label="Open sample details"
                onClick={(e) => open(e.currentTarget)}
              >
                <Icon name="more" />
              </IconButton>
              <PinButton
                label="Pin catalog rate"
                pinned={pin()}
                onChange={setPin}
              />
            </div>
          </Spec>
          <Spec title="Screen header & icon tiles" kind="Molecules">
            <ScreenHeader
              title="Your accounts"
              subtitle="All providers in one place"
              icon={
                <IconTile>
                  <Icon name="wallet" size={24} />
                </IconTile>
              }
              action={
                <IconButton
                  label="Account options"
                  onClick={(e) => open(e.currentTarget)}
                  intent="ghost"
                >
                  <Icon name="sliders" size={22} />
                </IconButton>
              }
            />
            <div class="wrap">
              <IconTile size="sm">
                <Icon name="card" />
              </IconTile>
              <IconTile>
                <Icon name="leaf" size={24} />
              </IconTile>
              <IconTile size="lg">
                <Icon name="home" size={28} />
              </IconTile>
              <TimePill>12 October 2026 · 10:00 UTC</TimePill>
            </div>
          </Spec>
          <Spec title="Inputs & search" kind="Forms">
            <TextField
              label="Local account name"
              value={name()}
              onInput={(e) => setName(e.currentTarget.value)}
            />
            <TextField
              label="Invalid input example"
              value="not a rate"
              error="Enter an exact decimal rate."
            />
            <TextField
              label="Disabled field"
              value="Managed by provider"
              disabled
            />
            <TextField
              label="Read-only field"
              value="Original provider reference"
              readOnly
            />
            <SearchField
              label="Catalog search example"
              value={search()}
              onChange={setSearch}
            />
            <p class="spec-note">Search draft: {search() || "empty"}</p>
          </Spec>
          <Spec
            title="Inline rate editor & pin"
            kind="Forms"
            note="Changes only this demo field. Does not calculate or save portfolio earnings."
          >
            <InlineEdit
              label="Catalog APY"
              value={rate()}
              suffix="%"
              pinned={pin()}
              onPinChange={setPin}
              onCommit={setRate}
              validate={(value) =>
                /^\d+(\.\d+)?$/.test(value)
                  ? undefined
                  : "Use a nonnegative decimal."
              }
            />
            <StatRow label="Current demo annotation">{rate()}%</StatRow>
            <Badge tone={pin() ? "accent" : "neutral"}>
              {pin() ? "Pinned" : "Latest update wins"}
            </Badge>
          </Spec>
          <Spec title="Range chips" kind="Selection">
            <ChipGroup
              label="Catalog chart range"
              value={range()}
              onChange={setRange}
              items={ranges}
            />
            <p class="spec-note">
              Selected: {range()}. Buttons expose their pressed state.
            </p>
          </Spec>
          <Spec title="Segmented control" kind="Selection">
            <GroupingTabs
              label="Catalog earnings view"
              value={segment()}
              onChange={setSegment}
              items={[
                { value: "income", label: "Money in" },
                { value: "outgoing", label: "Money out" },
              ]}
            />
            <StatRow
              label={
                segment() === "income" ? "Received example" : "Paid example"
              }
            >
              <Money value={segment() === "income" ? "18567.30" : "6240.18"} />
            </StatRow>
          </Spec>
          <Spec title="Badges & coverage warnings" kind="Status">
            <div class="wrap">
              <Badge>Neutral</Badge>
              <Badge tone="accent">Source evidence</Badge>
              <Badge tone="warning">Partial</Badge>
              <Badge tone="danger">Liability</Badge>
              <WarningButton
                label="Explain coverage warning"
                onClick={(e) => open(e.currentTarget)}
              />
            </div>
            <p class="spec-note">
              Icons complement text; color is never the only status cue.
            </p>
          </Spec>
          <Spec title="Financial formatting" kind="Exact values">
            <StatRow label="Significant amount">
              <Money value="12345.67" />
            </StatRow>
            <StatRow label="Small amount">
              <Money value="12.34" />
            </StatRow>
            <StatRow label="Tiny nonzero">
              <Money value="0.004" />
            </StatRow>
            <StatRow label="Negative cost">
              <Money value="-25.05" negative />
            </StatRow>
            <Quantity
              units="9007199254740993000000000000000001"
              scale={18}
              asset="EXAMPLE"
            />
            <Quantity units="1" scale={18} asset="ETH" />
            <div class="wrap">
              <Rate value="0.05" />
              <Rate value="0" />
              <Rate value="-0.00001" />
            </div>
          </Spec>
          <Spec title="Known, partial & unavailable" kind="Financial states">
            <StatRow label="Known zero">
              <MoneyFigure figure={{ status: "known", value: "0" }} />
            </StatRow>
            <StatRow label="Partial subtotal">
              <MoneyFigure figure={{ status: "partial", value: "125.5" }} />
            </StatRow>
            <StatRow label="Unknown">
              <MoneyFigure figure={{ status: "unknown" }} />
            </StatRow>
            <StatRow label="Not applicable">
              <MoneyFigure figure={{ status: "not_applicable" }} />
            </StatRow>
          </Spec>
          <Spec title="Summary value" kind="Molecule">
            <SummaryValue
              label="Current value · synthetic"
              figure={{ status: "known", value: "8926.07" }}
              primary
            />
            <SummaryValue
              label="Currently earning · per year"
              figure={{ status: "partial", value: "356.8" }}
              description="Known portion only. Unknown rates remain unavailable."
            />
          </Spec>
          <Spec
            title="Transaction row"
            kind="Molecule"
            note="Presentation only. The app’s transaction-history feature remains later work."
          >
            <ListRow
              label="Staking reward"
              description="Example provider · received"
              icon={
                <IconTile size="sm">
                  <Icon name="leaf" />
                </IconTile>
              }
              value={<Money value="23.5" />}
            />
            <ListRow
              label="Network fee"
              description="Example blockchain · paid"
              icon={
                <IconTile size="sm">
                  <Icon name="wallet" />
                </IconTile>
              }
              value={<Money value="-0.35" />}
            />
          </Spec>
          <Spec
            title="Card & pagination"
            kind="Composition"
            note="Fictional card specimen. No bank integration or card data."
          >
            <Surface tone="glow" class="bank-card">
              <div class="bank-card-mark">
                <Icon name="card" size={30} />
              </div>
              <strong class="card-number">
                •••• &nbsp; •••• &nbsp; •••• &nbsp; 4242
              </strong>
              <div class="card-footer">
                <div>
                  <span>Cardholder</span>
                  <strong>Alex Example</strong>
                </div>
                <div>
                  <span>Valid through</span>
                  <strong>12 / 30</strong>
                </div>
                <Icon name="leaf" />
              </div>
            </Surface>
            <PaginationDots
              label="Example card pages"
              count={3}
              selected={page()}
              onChange={setPage}
            />
          </Spec>
          <Spec
            title="Line chart & marker"
            kind="Visualization"
            wide
            note="Synthetic SVG geometry, not historical portfolio returns. No financial interpolation or number animation."
          >
            <TrendChart
              label={`Synthetic ${range()} trend`}
              path={chart()}
              marker={
                range() === "week"
                  ? { x: 55, y: 46, label: "Example point" }
                  : range() === "year"
                    ? { x: 35, y: 60, label: "Example point" }
                    : {
                        x: 54,
                        y: 52,
                        label:
                          range() === "month" ? "12 October" : "Example point",
                      }
              }
            />
            <ChipGroup
              label="Specimen chart range"
              value={range()}
              onChange={setRange}
              items={ranges}
            />
          </Spec>
          <Spec title="Sage glow panel" kind="Composition">
            <Surface tone="glow">
              <ScreenHeader
                title="Upcoming"
                icon={
                  <IconTile size="sm">
                    <Icon name="clock" />
                  </IconTile>
                }
              />
              <StatRow label="Fixed term matures">31 October</StatRow>
              <StatRow label="Renewal deadline">29 October</StatRow>
              <StatRow label="Available at maturity">
                <Money value="500" />
              </StatRow>
            </Surface>
          </Spec>
          <Spec title="Navigation entries" kind="Portfolio">
            <NavigationEntry
              name="Overview"
              selected={nav() === "overview"}
              onSelect={() => setNav("overview")}
              icon={<Icon name="home" />}
            />
            <NavigationEntry
              name="Ethereum"
              secondary="ETH"
              selected={nav() === "eth"}
              onSelect={() => setNav("eth")}
              value={<Money value="1000" />}
              rate={<Rate value="0.04" />}
              warning={{
                label: "Ethereum sample warning",
                onClick: (e) => open(e.currentTarget),
              }}
            />
            <NavigationEntry
              name="Provider"
              selected={nav() === "provider"}
              onSelect={() => setNav("provider")}
              value={<MoneyFigure figure={{ status: "unknown" }} />}
            />
          </Spec>
          <Spec title="Allocation table & detail panel" kind="Portfolio" wide>
            <AllocationTable
              caption="Synthetic allocation specimen"
              rows={rows()}
              onExpand={() => setExpanded(!expanded())}
              onDetails={(_, element) => open(element)}
            />
            <Button onClick={(e) => open(e.currentTarget)}>
              Open details panel
            </Button>
          </Spec>
          <Spec title="Lifecycle & empty states" kind="Status" wide>
            <div class="catalog-state-grid">
              <StateMessage
                kind="empty"
                title="No holdings yet"
                description="Connect or import accounts in a future stage."
              />
              <StateMessage kind="loading" title="Reading local data…" />
              <StateMessage
                kind="offline"
                title="Cached portfolio"
                description="Existing figures remain available offline."
              />
              <StateMessage
                kind="error"
                title="Coverage needs review"
                description="Financial diagnostics are omitted in privacy mode."
                action={
                  <Button onClick={(e) => open(e.currentTarget)}>
                    Review details
                  </Button>
                }
              />
            </div>
          </Spec>
          <Spec title="Navigation icons" kind="Atoms">
            <div class="wrap">
              <For
                each={
                  [
                    "home",
                    "wallet",
                    "leaf",
                    "user",
                    "clock",
                    "sliders",
                    "more",
                    "up",
                    "down",
                    "card",
                    "arrow",
                    "pin",
                    "search",
                    "warning",
                    "close",
                    "edit",
                    "undo",
                    "eye",
                    "check",
                  ] as const
                }
              >
                {(icon) => (
                  <div class="icon-specimen">
                    <Icon name={icon} size={22} />
                    <span>{icon}</span>
                  </div>
                )}
              </For>
            </div>
          </Spec>
        </div>
      </section>
      <section id="dashboard" aria-labelledby="dashboard-title">
        <div class="catalog-section-heading">
          <div>
            <span>03 · In context</span>
            <h2 id="dashboard-title">Composed money dashboard</h2>
          </div>
          <p>
            A synthetic arrangement of the same exported components. It is not
            the portfolio app’s feature roadmap.
          </p>
        </div>
        <Surface class="composed-dashboard">
          <ScreenHeader
            title="Money Tracker"
            subtitle="Your money, clearly."
            icon={
              <IconTile>
                <Icon name="leaf" size={26} />
              </IconTile>
            }
            action={<Badge tone="accent">Synthetic data</Badge>}
          />
          <div class="dashboard-grid">
            <Surface tone="inset" class="dashboard-balance">
              <ScreenHeader
                title="Your balance"
                action={
                  <IconButton
                    label="Balance example details"
                    onClick={(e) => open(e.currentTarget)}
                    intent="ghost"
                  >
                    <Icon name="more" />
                  </IconButton>
                }
              />
              <SummaryValue
                label="Combined value"
                figure={{ status: "known", value: "8926.07" }}
                primary
              />
              <ChipGroup
                label="Dashboard range"
                value={range()}
                onChange={setRange}
                items={ranges}
              />
              <TrendChart label="Synthetic balance curve" path={chart()} />
              <StatRow label="Annual earnings">
                <Money value="356.8" />
              </StatRow>
            </Surface>
            <Surface tone="glow">
              <ScreenHeader
                title="Currently earning"
                subtitle="Current-rate illustration"
              />
              <SummaryValue
                label="Per year"
                figure={{ status: "known", value: "356.8" }}
                primary
              />
              <StatRow label="Effective current APY">
                <Rate value="0.04" />
              </StatRow>
              <StatRow label="Gross earnings">
                <Money value="364.8" />
              </StatRow>
              <StatRow label="Borrowing costs">
                <Money value="8" />
              </StatRow>
              <p class="spec-note">
                All displayed figures are synthetic examples, excluded from your
                portfolio.
              </p>
            </Surface>
            <Surface tone="inset">
              <ScreenHeader
                title="Recent activity"
                subtitle="Presentation-only history specimen"
              />
              <div class="dashboard-list">
                <ListRow
                  label="Reward received"
                  description="Example earn product"
                  icon={
                    <IconTile size="sm">
                      <Icon name="leaf" />
                    </IconTile>
                  }
                  value={<Money value="23.5" />}
                />
                <ListRow
                  label="Network fee"
                  description="Example wallet"
                  icon={
                    <IconTile size="sm">
                      <Icon name="wallet" />
                    </IconTile>
                  }
                  value={<Money value="-0.35" />}
                />
              </div>
            </Surface>
            <Surface tone="glow">
              <ScreenHeader title="Upcoming" />
              <StatRow label="Fixed term matures">31 October</StatRow>
              <StatRow label="Unlocked value">
                <Money value="500" />
              </StatRow>
              <TimePill>Stop renewing from today</TimePill>
            </Surface>
          </div>
          <footer class="dashboard-nav">
            <For each={["home", "wallet", "leaf", "user"] as const}>
              {(icon) => (
                <IconButton
                  label={`Sample ${icon} navigation`}
                  intent="ghost"
                  onClick={(e) => open(e.currentTarget)}
                >
                  <Icon name={icon} size={22} />
                </IconButton>
              )}
            </For>
          </footer>
        </Surface>
      </section>
      <footer class="catalog-footer">
        SolidJS · Panda CSS · Ark UI ·{" "}
        <a href="/portfolio">Explore the full portfolio interaction flow ↗</a>
        <span>
          {privacy().mode === "normal"
            ? "Synthetic figures"
            : privacy().mode === "hidden"
              ? "Amounts hidden"
              : "Figures scaled to 10%"}
        </span>
      </footer>
      <DetailsPanel
        open={details()}
        onOpenChange={setDetails}
        title="Example source details"
        description="Synthetic evidence and financial states"
        returnFocus={() => trigger}
      >
        <Badge tone="warning">Example warning</Badge>
        <p>
          This is a component specimen. No provider, database or account is
          connected.
        </p>
        <SummaryValue
          label="Known annual subtotal"
          figure={{ status: "partial", value: "25" }}
        />
        <Quantity units="250000000000000000" scale={18} asset="ETH" />
        <StatRow label="Rate">
          <Rate value="0.05" />
        </StatRow>
        <StatRow label="Lock status">Locked</StatRow>
        <StatRow label="Maturity">31 October</StatRow>
        <StatRow label="Auto-renew">On</StatRow>
        <p class="spec-note">
          Unknown prices, missing rates and unreconciled history remain explicit
          in the real app.
        </p>
      </DetailsPanel>
    </main>
  );
}
