import { For, Show, createEffect, on, onCleanup } from "solid-js";
import type { JSX } from "solid-js";
import { Dialog } from "@ark-ui/solid/dialog";
import { css } from "../../styled-system/css";
import { button } from "../../styled-system/recipes";
import { Icon } from "./icons";
import { WarningButton } from "./finance";
export function NavigationEntry(props: {
  name: string;
  secondary?: JSX.Element;
  value?: JSX.Element;
  rate?: JSX.Element;
  selected?: boolean;
  onSelect: () => void;
  icon?: JSX.Element;
  warning?: {
    label: string;
    onClick: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
  };
}) {
  return (
    <div
      class={css({
        display: "flex",
        alignItems: "center",
        gap: "1",
        borderRadius: "md",
        _hover: { bg: "subtle" },
      })}
    >
      <button
        type="button"
        aria-current={props.selected ? "page" : undefined}
        onClick={props.onSelect}
        class={css({
          display: "grid",
          gridTemplateColumns: "32px 1fr auto",
          alignItems: "center",
          gap: "3",
          p: "3",
          width: "100%",
          textAlign: "left",
          borderRadius: "md",
          cursor: "pointer",
          _currentPage: { bg: "accentSubtle", color: "accentText" },
          _focusVisible: { outline: "2px solid", outlineColor: "accentText" },
        })}
      >
        <span
          class={css({
            display: "grid",
            placeItems: "center",
            bg: "accent",
            borderRadius: "sm",
            width: "32px",
            height: "32px",
            fontSize: "xs",
            fontWeight: "800",
            color: "onAccent",
          })}
        >
          {props.icon ?? props.name.slice(0, 2).toUpperCase()}
        </span>
        <span class={css({ display: "grid", gap: "1", minWidth: 0 })}>
          <span
            class={css({
              fontSize: "sm",
              fontWeight: "700",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            })}
          >
            {props.name}
          </span>
          <Show when={props.secondary}>
            <span class={css({ fontSize: "xs", color: "muted" })}>
              {props.secondary}
            </span>
          </Show>
        </span>
        <span class={css({ display: "grid", gap: "1", textAlign: "right" })}>
          <span class={css({ fontSize: "sm", fontWeight: "700" })}>
            {props.value}
          </span>
          <Show when={props.rate}>
            <span class={css({ fontSize: "xs", color: "muted" })}>
              {props.rate}
            </span>
          </Show>
        </span>
      </button>
      <Show when={props.warning}>
        <WarningButton
          label={props.warning!.label}
          onClick={props.warning!.onClick}
        />
      </Show>
    </div>
  );
}
export interface AllocationRow {
  id: string;
  name: string;
  depth: number;
  hasChildren?: boolean;
  expanded?: boolean;
  secondary?: JSX.Element;
  value: JSX.Element;
  rate: JSX.Element;
  annual: JSX.Element;
  liability?: boolean;
  warning?: {
    label: string;
    onClick: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
  };
}
/** Visible rows are supplied by the host; expansion never changes economic totals. */
export function AllocationTable(props: {
  rows: readonly AllocationRow[];
  onExpand: (id: string) => void;
  onDetails: (id: string, trigger: HTMLButtonElement) => void;
  caption: string;
}) {
  return (
    <div
      class={css({
        overflowX: "auto",
        border: "1px solid",
        borderColor: "border",
        borderRadius: "panel",
        bg: "inset",
      })}
    >
      <table
        class={css({
          width: "100%",
          borderCollapse: "collapse",
          fontSize: "sm",
          minWidth: "600px",
        })}
      >
        <caption
          class={css({
            textAlign: "left",
            px: "5",
            py: "4",
            fontWeight: "700",
            borderBottom: "1px solid",
            borderColor: "border",
          })}
        >
          {props.caption}
        </caption>
        <thead>
          <tr
            class={css({
              color: "muted",
              fontSize: "xs",
              textAlign: "right",
              bg: "subtle",
            })}
          >
            <th
              scope="col"
              class={css({
                textAlign: "left",
                px: "5",
                py: "3",
                fontWeight: "500",
                width: "42%",
              })}
            >
              Allocation
            </th>
            <th
              scope="col"
              class={css({ px: "4", py: "3", fontWeight: "500" })}
            >
              Current value
            </th>
            <th
              scope="col"
              class={css({ px: "4", py: "3", fontWeight: "500" })}
            >
              Current rate
            </th>
            <th
              scope="col"
              class={css({ px: "5", py: "3", fontWeight: "500" })}
            >
              Per year
            </th>
          </tr>
        </thead>
        <tbody>
          <For each={props.rows}>
            {(row) => (
              <tr
                class={css({
                  borderBottom: "1px solid",
                  borderColor: "border",
                  _last: { borderBottom: 0 },
                  _hover: { bg: "subtle" },
                })}
              >
                <th
                  scope="row"
                  class={css({
                    px: "3",
                    py: "3",
                    fontWeight: "400",
                    textAlign: "left",
                  })}
                >
                  <div
                    style={{
                      "padding-left": `${Math.max(0, row.depth) * 16}px`,
                    }}
                    class={css({
                      display: "flex",
                      alignItems: "center",
                      gap: "2",
                    })}
                  >
                    <Show
                      when={row.hasChildren}
                      fallback={
                        <span class={css({ width: "32px", flexShrink: 0 })} />
                      }
                    >
                      <button
                        type="button"
                        aria-label={`${row.expanded ? "Collapse" : "Expand"} ${row.name}`}
                        aria-expanded={!!row.expanded}
                        onClick={() => props.onExpand(row.id)}
                        class={[
                          button({ intent: "ghost", size: "sm" }),
                          css({ px: "2", flexShrink: 0 }),
                        ].join(" ")}
                      >
                        <span
                          style={{
                            transform: row.expanded ? "rotate(90deg)" : "none",
                          }}
                        >
                          <Icon name="arrow" />
                        </span>
                      </button>
                    </Show>
                    <div class={css({ display: "grid", gap: "1" })}>
                      <button
                        type="button"
                        onClick={(e) =>
                          props.onDetails(row.id, e.currentTarget)
                        }
                        class={css({
                          textAlign: "left",
                          fontWeight: "700",
                          cursor: "pointer",
                          color: "text",
                          _hover: { color: "accentText" },
                          borderRadius: "sm",
                          _focusVisible: {
                            outline: "2px solid",
                            outlineColor: "accentText",
                            outlineOffset: "3px",
                          },
                        })}
                      >
                        {row.name}
                      </button>
                      <Show when={row.secondary}>
                        <span class={css({ color: "muted", fontSize: "xs" })}>
                          {row.secondary}
                        </span>
                      </Show>
                      <Show when={row.liability}>
                        <span class={css({ color: "danger", fontSize: "xs" })}>
                          Liability · borrowed amount
                        </span>
                      </Show>
                    </div>
                    <Show when={row.warning}>
                      <WarningButton
                        label={row.warning!.label}
                        onClick={row.warning!.onClick}
                      />
                    </Show>
                  </div>
                </th>
                <td
                  class={css({
                    textAlign: "right",
                    px: "4",
                    py: "3",
                    fontWeight: "500",
                  })}
                >
                  {row.value}
                </td>
                <td class={css({ textAlign: "right", px: "4", py: "3" })}>
                  {row.rate}
                </td>
                <td class={css({ textAlign: "right", px: "5", py: "3" })}>
                  {row.annual}
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </div>
  );
}
/** Nonmodal, controlled, mounted under ThemeRoot; does not hide the allocation table. */
export function DetailsPanel(props: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  returnFocus?: () => HTMLElement | undefined;
  children: JSX.Element;
}) {
  let close: HTMLButtonElement | undefined;
  let frame: number | undefined;
  createEffect(
    on(
      () => props.open,
      (open, previous) => {
        if (frame !== undefined) cancelAnimationFrame(frame);
        if (open || previous)
          frame = requestAnimationFrame(() => {
            const target = open ? close : props.returnFocus?.();
            if (target?.isConnected) target.focus({ preventScroll: true });
          });
      },
    ),
  );
  onCleanup(() => {
    if (frame !== undefined) cancelAnimationFrame(frame);
  });

  return (
    <Dialog.Root
      open={props.open}
      onOpenChange={(e) => props.onOpenChange(e.open)}
      modal={false}
      preventScroll={false}
      closeOnInteractOutside={false}
      restoreFocus
      finalFocusEl={() => props.returnFocus?.() ?? null}
      initialFocusEl={() => close ?? null}
      lazyMount
      unmountOnExit
    >
      <Dialog.Positioner
        class={css({
          position: "fixed",
          top: 0,
          right: 0,
          bottom: 0,
          zIndex: 40,
          pointerEvents: "none",
        })}
      >
        <Dialog.Content
          class={css({
            width: "min(26rem, calc(100vw - 1rem))",
            height: "100%",
            bg: "surface",
            color: "text",
            borderLeft: "1px solid",
            borderTopLeftRadius: "panel",
            borderBottomLeftRadius: "panel",
            borderColor: "border",
            boxShadow: "-8px 0 48px #00000038",
            pointerEvents: "auto",
            overflowY: "auto",
            animation: "mt-panel-in 180ms ease-out",
            _closed: { display: "none" },
          })}
        >
          <div
            class={css({
              display: "flex",
              justifyContent: "space-between",
              alignItems: "start",
              gap: "3",
              p: "6",
              borderBottom: "1px solid",
              borderColor: "border",
            })}
          >
            <div>
              <Dialog.Title
                class={css({ fontSize: "lg", fontWeight: "700", mb: "2" })}
              >
                {props.title}
              </Dialog.Title>
              <Dialog.Description
                class={css({ fontSize: "sm", color: "muted" })}
              >
                {props.description ?? "Source details and available evidence"}
              </Dialog.Description>
            </div>
            <Dialog.CloseTrigger
              ref={close}
              class={button({ intent: "ghost", size: "sm" })}
              aria-label="Close details"
            >
              <Icon name="close" />
            </Dialog.CloseTrigger>
          </div>
          <div class={css({ p: "6", display: "grid", gap: "5" })}>
            {props.children}
          </div>
        </Dialog.Content>
      </Dialog.Positioner>
    </Dialog.Root>
  );
}
export function StateMessage(props: {
  kind: "empty" | "loading" | "offline" | "error";
  title: string;
  description?: string;
  action?: JSX.Element;
}) {
  return (
    <div
      role={props.kind === "error" ? "alert" : "status"}
      aria-live={props.kind === "error" ? "assertive" : "polite"}
      class={css({
        border: "1px solid",
        borderColor: "border",
        borderRadius: "lg",
        p: "5",
        bg: "surface",
        display: "grid",
        gap: "2",
      })}
    >
      <strong class={css({ fontSize: "sm" })}>{props.title}</strong>
      <Show when={props.description}>
        <p class={css({ fontSize: "sm", color: "muted" })}>
          {props.description}
        </p>
      </Show>
      {props.action}
    </div>
  );
}
