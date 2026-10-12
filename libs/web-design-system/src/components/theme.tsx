import {
  createContext,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  useContext,
} from "solid-js";
import type { Accessor, JSX } from "solid-js";
import type { Privacy } from "../formatting/index";
export type ThemeMode = "system" | "light" | "dark";
const PrivacyContext = createContext<Accessor<Privacy>>(
  () => ({ mode: "normal" }) as Privacy,
);
export function PrivacyProvider(props: {
  value: Privacy;
  children: JSX.Element;
}) {
  return (
    <PrivacyContext.Provider value={() => props.value}>
      {props.children}
    </PrivacyContext.Provider>
  );
}
export function usePrivacy() {
  return useContext(PrivacyContext) ?? (() => ({ mode: "normal" }) as Privacy);
}
/** Host owns preference persistence. Portals must mount inside this theme root. */
export function ThemeRoot(props: {
  mode?: ThemeMode;
  children: JSX.Element;
  class?: string;
}) {
  const [system, setSystem] = createSignal<"light" | "dark">("light");
  onMount(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystem(mq.matches ? "dark" : "light");
    update();
    mq.addEventListener("change", update);
    onCleanup(() => mq.removeEventListener("change", update));
  });
  const theme = createMemo(() =>
    !props.mode || props.mode === "system" ? system() : props.mode,
  );
  return (
    <div data-theme={theme()} class={props.class}>
      {props.children}
    </div>
  );
}
