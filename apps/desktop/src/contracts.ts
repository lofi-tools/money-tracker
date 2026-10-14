// Version 1 desktop wire contract mirrors src-tauri/src/lib.rs and lib-app DTOs.
// Monetary values, quantities and rates never cross IPC as JS numbers.
import type { Rational } from "@money-tracker/design-system";
export type Figure<T> =
  | { status: "known"; value: T }
  | { status: "partial"; value: T }
  | { status: "unknown" }
  | { status: "not_applicable" };
export interface MoneyValue {
  asset_id: string;
  amount: Rational;
}
export interface Summary {
  calculation_basis: string;
  current_value: Figure<MoneyValue>;
  gross_assets: Figure<MoneyValue>;
  liabilities: Figure<MoneyValue>;
  gross_annual_earnings: Figure<MoneyValue>;
  annual_borrowing_costs: Figure<MoneyValue>;
  net_annual_earnings: Figure<MoneyValue>;
  current_apy: Figure<Rational>;
  estimated_current_apy?: Figure<Rational>;
}
export type Selection =
  | { kind: "overview" | "dust" }
  | { kind: "asset" | "provider" | "exposure"; id: string };
export interface Issue {
  code: string;
  withheld: string[];
}
export interface QuantityValue {
  asset_id: string;
  units: string;
  scale: number;
}
export interface TokenMetadata {
  asset_id: string;
  chain_id: number;
  contract: string;
  name: string | null;
  ticker: string | null;
  icon_url: string | null;
  icon_data: string | null;
  decimals: number | null;
  rpc_decimals: number | null;
  coingecko_id: string | null;
  fetched_at: string | null;
  sources: Record<string, unknown>;
  warnings: string[];
}
export interface Entry {
  direct_quantities?: QuantityValue[] | null;
  alternatives?: {
    name: string;
    ticker: string;
    provider: string;
    role: string;
    quantity: QuantityValue;
    value: Figure<MoneyValue>;
  }[];
  metadata_unresolved?: boolean;
  token_metadata?: TokenMetadata | null;
  estimated_current_value?: Figure<MoneyValue> | null;
  selection: Selection;
  name: string;
  ticker: string | null;
  summary: Summary;
  issues: Issue[];
  quantities: [string, QuantityValue][];
}
export interface Settings {
  reporting_currency: string;
  theme: "system" | "light" | "dark";
  privacy_mode: "normal" | "hidden" | "scaled";
  privacy_scale: string;
  dust_threshold: string;
  refresh_interval_seconds: number;
  refresh_on_open: boolean;
}
export type RefreshTrigger = "launch" | "scheduled" | "manual";
/** One configured refresh source with its persisted schedule and freshness. */
export interface RefreshStatus {
  source: string;
  label: string;
  ready: boolean;
  prerequisite: string | null;
  last_attempt: string | null;
  last_success: string | null;
  next_due: string | null;
  last_error: string | null;
}
/** There is no amount or provider text in a refresh outcome, by design. */
export type RefreshOutcome =
  | { status: "updated"; detail: string }
  | { status: "unchanged" }
  | { status: "failed"; code: string; message: string }
  | { status: "blocked"; reason: string }
  | { status: "scheduled" }
  | { status: "not_due" };
export interface RefreshEntry {
  source: string;
  label: string;
  outcome: RefreshOutcome;
}
export interface RefreshReport {
  at: string;
  trigger: RefreshTrigger;
  interval_seconds: number;
  in_flight: boolean;
  entries: RefreshEntry[];
}
/** One refresh pass; `startup` carries the snapshot and schedule it produced. */
export interface RefreshUpdate {
  startup: Startup;
  report: RefreshReport;
}
export type Startup =
  | { status: "ready"; data: Snapshot; settings: Settings }
  | {
      status: "missing";
      database: { destination: string; legacy_database: string | null };
      settings: Settings;
    };
/** Manual overrides are dated and pinned; source evidence is never replaced. */
export interface AnnualRate {
  fraction: string;
  basis: "apr" | "apy";
  compounds_per_year: number | null;
  reward_asset: string | null;
}
export interface ManualRate {
  effective_at: string;
  updated_at: string;
  rate: AnnualRate | null;
  pinned: boolean;
}
export interface ManualPrice {
  effective_at: string;
  updated_at: string;
  price: string;
  pinned: boolean;
}
export interface AccountCustomization {
  local_label: string | null;
  parent_is_custom: boolean;
  parent: string | null;
}
export type LocalValue =
  | { kind: "account"; value: AccountCustomization }
  | { kind: "subscription_name"; value: string }
  | { kind: "rates"; value: ManualRate[] }
  | { kind: "prices"; value: ManualPrice[] }
  | { kind: "spam"; value: { excluded: boolean; reason: string } }
  | { kind: "dust"; value: "auto" | "dust" | "significant" };
export type RateScope =
  | { kind: "subscription"; id: string }
  | { kind: "product"; id: string }
  | { kind: "account_asset"; id: { account: string; asset: string } }
  | { kind: "provider"; id: string }
  | { kind: "asset"; id: string }
  | { kind: "provider_asset"; id: { provider: string; asset: string } }
  | { kind: "borrowing_position"; id: string };
export type LocalTarget =
  | { kind: "account"; id: string }
  | { kind: "subscription"; id: string }
  | { kind: "rate"; scope: RateScope }
  | { kind: "price"; asset: string; quote: string }
  | { kind: "spam"; asset: string }
  | { kind: "dust"; provider: string; asset: string };
export interface LocalRecord {
  revision: number;
  value: LocalValue | null;
}
/** One editable local target with the revision the host must send back. */
export interface EditSlot {
  target: LocalTarget;
  revision: number;
  value: LocalValue | null;
}
export interface SelectedPrice {
  as_of: string;
  updated_at: string;
  pinned: boolean;
  evidence_id: string | null;
  price: string;
}
export interface NodeEdits {
  rate?: EditSlot;
  price?: EditSlot;
  selected_price?: SelectedPrice;
}
export interface SelectedRate {
  pinned: boolean;
  effective_at: string;
  updated_at: string;
  evidence_id: string | null;
  rate: AnnualRate | null;
}
export interface HoldingEdits {
  rate: EditSlot | null;
  price: EditSlot | null;
  dust: EditSlot | null;
  spam: EditSlot;
  name: EditSlot | null;
}
export interface AccountOption {
  id: string | null;
  name: string;
  group: boolean;
}
export interface NodeAccount {
  wallet_software?: string | null;
  id: string;
  provider_id: string;
  parent: string | null;
  group: boolean;
  source_label: string;
  local_label: string | null;
  slot: EditSlot;
  parent_options: AccountOption[];
}
export interface AccountStructure {
  account_id: string;
  kind: "source" | "group";
  source_parent: string | null;
  mapping_version: number;
}
export interface AccountJurisdiction {
  country_code: string;
  declared_at: string;
}
export interface UiAccount {
  account: {
    id: string;
    provider_id: string;
    label: string;
    source_reference: string | null;
    jurisdiction?: AccountJurisdiction | null;
  };
  structure: AccountStructure;
  parent: string | null;
}
export interface ActionEntry {
  id: string;
  at: string;
  undone: boolean;
  target: LocalTarget;
  undo_of: string | null;
  revision: number;
  before: LocalValue | null;
  after: LocalValue | null;
}
export interface EditContext {
  accounts: UiAccount[];
  local_state: [LocalTarget, LocalRecord][];
  actions: ActionEntry[];
}
export interface Snapshot {
  contract_version: 1;
  snapshot_id: number;
  cutoff: string;
  overview: Summary;
  assets: Entry[];
  providers: Entry[];
  dust: Entry;
  holding_count: number;
  dust_count: number;
  edits: EditContext;
  /** Configured refresh sources with stored schedule, freshness and failures. */
  refresh: RefreshStatus[];
}
export type AllocationTarget =
  | { kind: "provider"; id: string }
  | { kind: "account"; id: string }
  | { kind: "product"; account: string; product: string }
  | { kind: "subscription"; id: string }
  | { kind: "holding"; id: string };
export interface InferredAllocation {
  maturity?: string | null;
  expected_maturity?: string | null;
  id: string;
  account: string;
  kind: string;
  quantity: QuantityValue;
  opened_at: string | null;
  as_of: string;
  source_hash: string;
  count: number;
  observed_apy: string | null;
  observed_from: string | null;
  expected_apy: Rational | null;
  expected_rate_note: string | null;
}
export interface Node {
  inferred_allocation?: InferredAllocation | null;
  id: string;
  target: AllocationTarget;
  name: string;
  parent: string | null;
  children: string[];
  summary: Summary;
  issues: Issue[];
  liability: boolean;
  edits?: NodeEdits;
}
export interface Tree {
  initially_expanded: string[];
  selection: Selection;
  roots: string[];
  nodes: Node[];
}
export interface Holding {
  label: string;
  role: string;
  provider_id: string;
  quantity: QuantityValue;
  value: Figure<MoneyValue>;
  annual_earnings: Figure<MoneyValue>;
  annual_borrowing_cost: Figure<MoneyValue>;
  verification_current: boolean;
  issues: Issue[];
  subscription: {
    lock_status: string;
    maturity: string | null;
    auto_renew: boolean | null;
    source_reference: string;
  } | null;
  rate: SelectedRate | null;
  price: SelectedPrice | null;
  edits: HoldingEdits;
}
export interface Details {
  inferred_allocation?: InferredAllocation | null;
  jurisdiction?: AccountJurisdiction | null;
  name: string;
  summary: Summary;
  issues: Issue[];
  original_name?: string | null;
  source_reference?: string | null;
  holdings: Holding[];
  account?: NodeAccount | null;
}
/** Typed manual edit requests; Rust validates scope, dates and revisions. */
export type ManualEdit =
  | { kind: "wallet_software"; account: string; software: string | null }
  | {
      kind: "rate";
      scope: RateScope;
      rate: AnnualRate | null;
      effective_at: string;
      pinned: boolean;
      clear: boolean;
    }
  | { kind: "rate_pin"; scope: RateScope; pinned: boolean }
  | {
      kind: "price";
      asset: string;
      quote: string;
      price: string | null;
      effective_at: string;
      pinned: boolean;
      clear: boolean;
    }
  | { kind: "price_pin"; asset: string; quote: string; pinned: boolean }
  | { kind: "account_name"; account: string; name: string | null }
  | { kind: "subscription_name"; id: string; name: string | null }
  | { kind: "group"; provider: string; name: string; parent: string | null }
  | {
      kind: "move";
      account: string;
      parent: string | null;
      source_default: boolean;
    }
  | {
      kind: "dust";
      provider: string;
      asset: string;
      mode: "auto" | "dust" | "significant";
    }
  | { kind: "spam"; asset: string; excluded: boolean; reason: string };
export const defaultSettings: Settings = {
  reporting_currency: "USD",
  theme: "system",
  privacy_mode: "normal",
  privacy_scale: "0.1",
  dust_threshold: "1",
  refresh_interval_seconds: 86400,
  refresh_on_open: false,
};
