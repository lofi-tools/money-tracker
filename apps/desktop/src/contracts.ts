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
}
export type Selection =
  { kind: "overview" | "dust" } | { kind: "asset" | "provider"; id: string };
export interface Issue {
  code: string;
  withheld: string[];
}
export interface QuantityValue {
  asset_id: string;
  units: string;
  scale: number;
}
export interface Entry {
  selection: Selection;
  name: string;
  ticker: string | null;
  summary: Summary;
  issues: Issue[];
  quantities: [string, QuantityValue][];
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
export type Startup =
  | { status: "ready"; data: Snapshot; settings: Settings }
  | {
      status: "missing";
      database: { destination: string; legacy_database: string | null };
      settings: Settings;
    };
export interface Node {
  id: string;
  name: string;
  parent: string | null;
  children: string[];
  summary: Summary;
  issues: Issue[];
  liability: boolean;
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
  rate: {
    pinned: boolean;
    effective_at: string;
    updated_at: string;
    evidence_id: string | null;
    rate: { fraction: string; basis: string } | null;
  } | null;
  price: {
    as_of: string;
    updated_at: string;
    pinned: boolean;
    evidence_id: string | null;
  } | null;
}
export interface Details {
  name: string;
  summary: Summary;
  issues: Issue[];
  original_name?: string | null;
  source_reference?: string | null;
  holdings: Holding[];
}
export const defaultSettings: Settings = {
  reporting_currency: "USD",
  theme: "system",
  privacy_mode: "normal",
  privacy_scale: "0.1",
  dust_threshold: "1",
  refresh_interval_seconds: 86400,
  refresh_on_open: false,
};
