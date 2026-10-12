import { For, Show, createSignal, createUniqueId, splitProps } from "solid-js";
import type { JSX } from "solid-js";
import { Tabs } from "@ark-ui/solid/tabs";
import { Editable } from "@ark-ui/solid/editable";
import { css } from "../../styled-system/css";
import { button } from "../../styled-system/recipes";
import { Icon } from "./icons";
export type ButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  intent?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
};
export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["intent", "size", "class"]);
  return (
    <button
      type="button"
      {...rest}
      class={[button({ intent: local.intent, size: local.size }), local.class]
        .filter(Boolean)
        .join(" ")}
    />
  );
}
export function IconButton(props: ButtonProps & { label: string }) {
  const [local, rest] = splitProps(props, ["label", "class"]);
  return (
    <Button
      {...rest}
      aria-label={local.label}
      class={[css({ borderRadius: "sm", minWidth: "32px" }), local.class]
        .filter(Boolean)
        .join(" ")}
    />
  );
}
export function PinButton(props: {
  pinned: boolean;
  onChange: (pinned: boolean) => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <IconButton
      label={props.label}
      aria-pressed={props.pinned}
      disabled={props.disabled}
      intent={props.pinned ? "secondary" : "ghost"}
      size="sm"
      onClick={() => props.onChange(!props.pinned)}
    >
      <Icon name="pin" />
    </IconButton>
  );
}
const inputClass = css({
  minHeight: "32px",
  border: "1px solid",
  borderColor: "border",
  borderRadius: "sm",
  px: "3",
  py: "2",
  bg: "subtle",
  color: "text",
  fontSize: "sm",
  width: "100%",
  _disabled: { opacity: 0.5 },
  _invalid: { borderColor: "danger" },
});
const srOnly = css({
  position: "absolute",
  width: "1px",
  height: "1px",
  overflow: "hidden",
  clipPath: "inset(50%)",
  whiteSpace: "nowrap",
});
export function TextField(
  props: JSX.InputHTMLAttributes<HTMLInputElement> & {
    label: string;
    error?: string;
    prefix?: string;
  },
) {
  const id = createUniqueId();
  const [local, rest] = splitProps(props, [
    "label",
    "error",
    "class",
    "id",
    "prefix",
  ]);
  return (
    <div class={css({ display: "grid", gap: "2" })}>
      <label
        for={local.id ?? id}
        class={css({ fontSize: "sm", fontWeight: "500" })}
      >
        {local.label}
      </label>
      <div
        class={css({ position: "relative" })}
        style={{
          "--mt-input-prefix-width": `${(local.prefix?.length ?? 0) * 0.75 + 1.125}rem`,
        }}
      >
        <Show when={local.prefix}>
          <span
            aria-hidden="true"
            class={css({
              position: "absolute",
              left: "3",
              top: "50%",
              transform: "translateY(-50%)",
              pointerEvents: "none",
              color: "muted",
              fontSize: "sm",
            })}
          >
            {local.prefix}
          </span>
        </Show>
        <input
          {...rest}
          id={local.id ?? id}
          aria-invalid={local.error ? true : rest["aria-invalid"]}
          aria-describedby={
            [rest["aria-describedby"], local.error ? `${id}-error` : undefined]
              .filter(Boolean)
              .join(" ") || undefined
          }
          class={[
            inputClass,
            local.prefix && css({ pl: "var(--mt-input-prefix-width)" }),
            local.class,
          ]
            .filter(Boolean)
            .join(" ")}
        />
      </div>
      <Show when={local.error}>
        <span
          id={`${id}-error`}
          class={css({ fontSize: "xs", color: "danger" })}
        >
          {local.error}
        </span>
      </Show>
    </div>
  );
}
export function SearchField(props: {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  placeholder?: string;
}) {
  const id = createUniqueId();
  return (
    <div class={css({ position: "relative" })}>
      <label for={id} class={srOnly}>
        {props.label ?? "Search assets or providers"}
      </label>
      <span
        class={css({
          position: "absolute",
          left: "9px",
          top: "8px",
          color: "muted",
          pointerEvents: "none",
        })}
      >
        <Icon name="search" />
      </span>
      <input
        id={id}
        type="search"
        value={props.value}
        onInput={(e) => props.onChange(e.currentTarget.value)}
        placeholder={props.placeholder ?? "Search assets or providers"}
        class={[inputClass, css({ pl: "9" })].join(" ")}
      />
    </div>
  );
}
export function GroupingTabs(props: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  items: readonly { value: string; label: string }[];
  children?: JSX.Element;
}) {
  return (
    <Tabs.Root
      value={props.value}
      onValueChange={(e) => props.onChange(e.value)}
      class={css({ display: "grid", gap: "3" })}
    >
      <Tabs.List
        aria-label={props.label}
        class={css({
          display: "flex",
          gap: "1",
          p: "1",
          bg: "subtle",
          borderRadius: "md",
        })}
      >
        <For each={props.items}>
          {(item) => (
            <Tabs.Trigger
              value={item.value}
              class={css({
                flex: "1",
                px: "3",
                py: "2",
                borderRadius: "sm",
                fontSize: "sm",
                fontWeight: "700",
                color: "muted",
                cursor: "pointer",
                _selected: {
                  bg: "accent",
                  color: "onAccent",
                  boxShadow: "0 1px 3px #00000014",
                },
                _focusVisible: {
                  outline: "2px solid",
                  outlineColor: "accentText",
                  outlineOffset: "2px",
                },
              })}
            >
              {item.label}
            </Tabs.Trigger>
          )}
        </For>
      </Tabs.List>
      <For each={props.items}>
        {(item) => (
          <Tabs.Content value={item.value}>
            {props.value === item.value ? props.children : null}
          </Tabs.Content>
        )}
      </For>
    </Tabs.Root>
  );
}
/** Controlled draft/edit state prevents invalid Enter/blur commits. */
export function InlineEdit(props: {
  label: string;
  value: string;
  onCommit: (value: string) => void;
  validate?: (value: string) => string | undefined;
  pinned?: boolean;
  onPinChange?: (pinned: boolean) => void;
  suffix?: string;
  disabled?: boolean;
}) {
  const [editing, setEditing] = createSignal(false);
  const [draft, setDraft] = createSignal(props.value);
  const [error, setError] = createSignal<string>();
  const id = createUniqueId();
  const commit = (value: string) => {
    const problem = props.validate?.(value);
    if (problem) {
      setError(problem);
      setEditing(true);
      return;
    }
    setError(undefined);
    props.onCommit(value);
    setEditing(false);
  };
  return (
    <div class={css({ display: "grid", gap: "1" })}>
      <div
        class={css({
          display: "flex",
          alignItems: "center",
          justifyContent: "flex-end",
          gap: "1",
        })}
      >
        <Editable.Root
          class={css({
            display: "inline-flex",
            alignItems: "center",
            gap: "1",
          })}
          value={editing() ? draft() : props.value}
          edit={editing()}
          disabled={props.disabled}
          translations={{ input: props.label }}
          activationMode="none"
          submitMode="enter"
          invalid={!!error()}
          onEditChange={(e) => {
            if (e.edit) {
              setDraft(props.value);
            }
            setError(undefined);
            setEditing(e.edit);
          }}
          onValueChange={(e) => {
            setDraft(e.value);
            setError(props.validate?.(e.value));
          }}
          // Closing the controlled Ark editor can expose the old source prop
          // before its commit callback; the owned validated draft is authoritative.
          onValueCommit={() => commit(draft())}
        >
          <Editable.Label class={srOnly}>{props.label}</Editable.Label>
          <Editable.Area>
            <Editable.Preview
              class={css({
                display: "inline-block",
                minWidth: "42px",
                textAlign: "right",
                fontSize: "sm",
                fontVariantNumeric: "tabular-nums",
              })}
            />
            <Editable.Input
              aria-describedby={error() ? id : undefined}
              class={[
                inputClass,
                css({
                  width: "90px",
                  minHeight: "32px",
                  textAlign: "right",
                  px: "2",
                }),
              ].join(" ")}
              onKeyDown={(e) => {
                if (e.key === "Enter" && props.validate?.(draft())) {
                  e.preventDefault();
                  e.stopPropagation();
                  setError(props.validate(draft()));
                }
              }}
            />
          </Editable.Area>
          <Editable.Control
            class={css({ display: "inline-flex", gap: "1", ml: "1" })}
          >
            <Editable.EditTrigger
              class={button({ intent: "ghost", size: "sm" })}
              aria-label={`Edit ${props.label}`}
            >
              <Icon name="edit" />
            </Editable.EditTrigger>
            <Editable.SubmitTrigger
              class={button({ intent: "ghost", size: "sm" })}
              aria-label={`Save ${props.label}`}
              disabled={!!error()}
            >
              <Icon name="check" />
            </Editable.SubmitTrigger>
            <Editable.CancelTrigger
              class={button({ intent: "ghost", size: "sm" })}
              aria-label={`Cancel ${props.label}`}
            >
              <Icon name="close" />
            </Editable.CancelTrigger>
          </Editable.Control>
        </Editable.Root>
        <Show when={props.suffix}>
          <span class={css({ fontSize: "xs", color: "muted" })}>
            {props.suffix}
          </span>
        </Show>
        <Show when={props.onPinChange}>
          <PinButton
            label={`Pin ${props.label}`}
            pinned={props.pinned ?? false}
            disabled={props.disabled}
            onChange={(v) => props.onPinChange?.(v)}
          />
        </Show>
      </div>
      <Show when={error()}>
        <span
          id={id}
          role="alert"
          class={css({ fontSize: "xs", color: "danger", textAlign: "right" })}
        >
          {error()}
        </span>
      </Show>
    </div>
  );
}
