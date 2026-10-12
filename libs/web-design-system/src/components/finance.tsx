import { Show } from "solid-js";
import type { JSX } from "solid-js";
import { css } from "../../styled-system/css";
import { badge } from "../../styled-system/recipes";
import { formatMoney, formatQuantity, formatRate } from "../formatting/index";
import type { ExactValue } from "../formatting/index";
import { usePrivacy } from "./theme";
import { IconButton } from "./controls";
import { Icon } from "./icons";
export function Badge(props: {
  tone?: "neutral" | "accent" | "warning" | "danger";
  children: JSX.Element;
}) {
  return <span class={badge({ tone: props.tone })}>{props.children}</span>;
}
export function WarningButton(props: {
  label: string;
  onClick: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
}) {
  return (
    <IconButton
      label={props.label}
      intent="ghost"
      size="sm"
      class={css({ color: "warning" })}
      onClick={props.onClick}
    >
      <Icon name="warning" />
    </IconButton>
  );
}
export function Money(props: {
  value: ExactValue;
  currency?: string;
  negative?: boolean;
  class?: string;
}) {
  const privacy = usePrivacy();
  return (
    <span
      class={[
        css({ fontVariantNumeric: "tabular-nums", whiteSpace: "nowrap" }),
        props.negative ? css({ color: "danger" }) : "",
        props.class,
      ]
        .filter(Boolean)
        .join(" ")}
      aria-label={privacy().mode === "hidden" ? "Amount hidden" : undefined}
    >
      {formatMoney(props.value, props.currency ?? "USD", privacy())}
    </span>
  );
}
export function Quantity(props: {
  units: string;
  scale: number;
  asset?: string;
  class?: string;
}) {
  const privacy = usePrivacy();
  return (
    <span
      class={[
        css({ fontVariantNumeric: "tabular-nums", overflowWrap: "anywhere" }),
        props.class,
      ]
        .filter(Boolean)
        .join(" ")}
      aria-label={privacy().mode === "hidden" ? "Quantity hidden" : undefined}
    >
      {formatQuantity(props.units, props.scale, privacy())}
      <Show when={props.asset}>
        <span> {props.asset}</span>
      </Show>
    </span>
  );
}
export function Rate(props: { value: ExactValue; class?: string }) {
  return (
    <span
      class={[
        css({ fontVariantNumeric: "tabular-nums", whiteSpace: "nowrap" }),
        props.class,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {formatRate(props.value)}
    </span>
  );
}
export type CurrencyFigure =
  | { status: "known" | "partial"; value: ExactValue }
  | { status: "unknown" | "not_applicable" };
export function MoneyFigure(props: {
  figure: CurrencyFigure;
  currency?: string;
  label?: string;
}) {
  return (
    <span
      class={css({
        display: "inline-flex",
        gap: "2",
        alignItems: "center",
        flexWrap: "wrap",
      })}
    >
      <Show
        when={
          props.figure.status === "known" || props.figure.status === "partial"
        }
        fallback={
          <span class={css({ color: "muted", fontSize: "sm" })}>
            {props.figure.status === "not_applicable"
              ? "Not applicable"
              : "Unknown"}
          </span>
        }
      >
        <Money
          value={(props.figure as { value: ExactValue }).value}
          currency={props.currency}
        />
      </Show>
      <Show when={props.figure.status === "partial"}>
        <Badge tone="warning">Partial subtotal</Badge>
      </Show>
    </span>
  );
}
export function SummaryValue(props: {
  label: string;
  figure: CurrencyFigure;
  currency?: string;
  description?: string;
  primary?: boolean;
}) {
  return (
    <div class={css({ display: "grid", gap: "2" })}>
      <span class={css({ fontSize: "sm", color: "muted", fontWeight: "500" })}>
        {props.label}
      </span>
      <div
        class={
          props.primary
            ? css({
                fontSize: "display",
                fontWeight: "600",
                letterSpacing: "-0.04em",
              })
            : css({
                fontSize: "xl",
                fontWeight: "600",
                letterSpacing: "-0.025em",
              })
        }
      >
        <MoneyFigure figure={props.figure} currency={props.currency} />
      </div>
      <Show when={props.description}>
        <span class={css({ fontSize: "xs", color: "muted" })}>
          {props.description}
        </span>
      </Show>
    </div>
  );
}
