import { For, Show, createUniqueId } from "solid-js";
import type { JSX } from "solid-js";
import { css } from "../../styled-system/css";
import { Icon } from "./icons";
export function Surface(props: {
  children: JSX.Element;
  tone?: "surface" | "inset" | "glow";
  class?: string;
}) {
  return (
    <section
      class={[
        css({
          border: "1px solid",
          borderColor: "border",
          borderRadius: "panel",
          p: "6",
          bg: "surface",
          boxShadow: "panel",
        }),
        props.tone === "inset" ? css({ bg: "inset" }) : "",
        props.tone === "glow"
          ? css({
              backgroundImage:
                "radial-gradient(120% 95% at 82% 8%, #a7d1b64d, transparent 58%)",
            })
          : "",
        props.class,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {props.children}
    </section>
  );
}
export function IconTile(props: {
  children: JSX.Element;
  size?: "sm" | "md" | "lg";
}) {
  return (
    <span
      class={css({
        display: "inline-grid",
        placeItems: "center",
        bg: "accent",
        color: "onAccent",
        borderRadius: "md",
        flexShrink: 0,
        fontWeight: "800",
        transition: "transform 180ms, box-shadow 180ms",
        _hover: { transform: "translateY(-2px)", boxShadow: "tile" },
      })}
      style={{
        width:
          props.size === "sm" ? "40px" : props.size === "lg" ? "62px" : "52px",
        height:
          props.size === "sm" ? "40px" : props.size === "lg" ? "62px" : "52px",
      }}
    >
      {props.children}
    </span>
  );
}
export function ScreenHeader(props: {
  title: string;
  subtitle?: string;
  icon?: JSX.Element;
  action?: JSX.Element;
}) {
  return (
    <div
      class={css({
        display: "flex",
        gap: "3",
        alignItems: "center",
        justifyContent: "space-between",
      })}
    >
      <Show when={props.icon}>{props.icon}</Show>
      <div class={css({ flex: 1 })}>
        <h3
          class={css({
            fontSize: "lg",
            fontWeight: "700",
            letterSpacing: "-0.02em",
          })}
        >
          {props.title}
        </h3>
        <Show when={props.subtitle}>
          <p class={css({ fontSize: "xs", color: "muted", mt: "1" })}>
            {props.subtitle}
          </p>
        </Show>
      </div>
      {props.action}
    </div>
  );
}
export function ChipGroup(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  items: readonly { value: string; label: string }[];
}) {
  return (
    <div
      role="group"
      aria-label={props.label}
      class={css({ display: "flex", gap: "2", flexWrap: "wrap" })}
    >
      <For each={props.items}>
        {(item) => (
          <button
            type="button"
            aria-pressed={props.value === item.value}
            onClick={() => props.onChange(item.value)}
            class={css({
              border: "1px solid",
              borderColor: "border",
              borderRadius: "pill",
              px: "5",
              py: "3",
              fontSize: "xs",
              fontWeight: "700",
              color: "muted",
              cursor: "pointer",
              _hover: { color: "text", bg: "hover" },
              _pressed: {
                bg: "accent",
                color: "onAccent",
                borderColor: "accent",
              },
            })}
          >
            {item.label}
          </button>
        )}
      </For>
    </div>
  );
}
export function StatRow(props: { label: string; children: JSX.Element }) {
  return (
    <div
      class={css({
        display: "flex",
        justifyContent: "space-between",
        alignItems: "baseline",
        gap: "4",
        py: "2",
        fontSize: "sm",
      })}
    >
      <span class={css({ color: "muted", fontWeight: "600" })}>
        {props.label}
      </span>
      <span class={css({ fontWeight: "800" })}>{props.children}</span>
    </div>
  );
}
export function ListRow(props: {
  label: string;
  description?: string;
  icon: JSX.Element;
  value: JSX.Element;
}) {
  return (
    <div
      class={css({
        display: "flex",
        alignItems: "center",
        gap: "3",
        p: "3",
        bg: "subtle",
        borderRadius: "lg",
      })}
    >
      {props.icon}
      <div class={css({ flex: 1, minWidth: 0 })}>
        <strong
          class={css({ display: "block", fontSize: "sm", fontWeight: "700" })}
        >
          {props.label}
        </strong>
        <Show when={props.description}>
          <span class={css({ fontSize: "xs", color: "muted" })}>
            {props.description}
          </span>
        </Show>
      </div>
      <span class={css({ fontSize: "sm", fontWeight: "700" })}>
        {props.value}
      </span>
    </div>
  );
}
export function TimePill(props: { children: JSX.Element }) {
  return (
    <span
      class={css({
        display: "inline-flex",
        alignItems: "center",
        gap: "3",
        bg: "subtle",
        border: "1px solid",
        borderColor: "border",
        borderRadius: "md",
        p: "3",
        fontSize: "xs",
        fontWeight: "700",
      })}
    >
      <span
        class={css({
          color: "accentText",
          display: "grid",
          placeItems: "center",
        })}
      >
        <Icon name="clock" size={22} />
      </span>
      {props.children}
    </span>
  );
}
/** SVG geometry is presentation input, not portfolio data or a performance calculation. */
export function TrendChart(props: {
  label: string;
  path: string;
  marker?: { x: number; y: number; label: string };
}) {
  const id = createUniqueId();
  return (
    <div class={css({ position: "relative", height: "220px" })}>
      <svg
        role="img"
        aria-label={props.label}
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        class={css({ width: "100%", height: "100%", overflow: "visible" })}
      >
        <defs>
          <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#b7d3bf" stop-opacity="0.3" />
            <stop offset="100%" stop-color="#b7d3bf" stop-opacity="0" />
          </linearGradient>
        </defs>
        <path d={`${props.path} L100 100 L0 100 Z`} fill={`url(#${id})`} />
        <path
          d={props.path}
          fill="none"
          stroke="var(--mt-colors-accent-text)"
          stroke-width="2.2"
          vector-effect="non-scaling-stroke"
          stroke-linecap="round"
        />
      </svg>
      <Show when={props.marker}>
        {(marker) => (
          <span
            class={css({
              position: "absolute",
              transform: "translate(-50%, -50%)",
            })}
            style={{ left: `${marker().x}%`, top: `${marker().y}%` }}
          >
            <span
              class={css({
                display: "block",
                width: "12px",
                height: "12px",
                borderRadius: "pill",
                bg: "accentText",
                boxShadow: "0 0 0 4px var(--mt-colors-inset)",
              })}
            />
            <span
              class={css({
                position: "absolute",
                bottom: "24px",
                left: "50%",
                transform: "translateX(-50%)",
                bg: "hover",
                border: "1px solid",
                borderColor: "border",
                borderRadius: "sm",
                px: "3",
                py: "2",
                whiteSpace: "nowrap",
                fontSize: "xs",
                fontWeight: "700",
              })}
            >
              {marker().label}
            </span>
          </span>
        )}
      </Show>
    </div>
  );
}
export function PaginationDots(props: {
  label: string;
  count: number;
  selected: number;
  onChange: (index: number) => void;
}) {
  return (
    <div
      role="group"
      aria-label={props.label}
      class={css({ display: "flex", gap: "1", justifyContent: "center" })}
    >
      <For each={Array.from({ length: props.count }, (_, i) => i)}>
        {(i) => (
          <button
            type="button"
            aria-label={`Page ${i + 1}`}
            aria-pressed={props.selected === i}
            onClick={() => props.onChange(i)}
            class={css({
              display: "grid",
              placeItems: "center",
              minWidth: "28px",
              height: "32px",
              cursor: "pointer",
            })}
          >
            <span
              class={css({ height: "6px", borderRadius: "pill", bg: "muted" })}
              style={{
                width: props.selected === i ? "18px" : "6px",
                background:
                  props.selected === i
                    ? "var(--mt-colors-accent-text)"
                    : undefined,
              }}
            />
          </button>
        )}
      </For>
    </div>
  );
}
