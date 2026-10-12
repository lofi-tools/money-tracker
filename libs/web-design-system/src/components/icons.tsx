import type { JSX } from "solid-js";
export function Icon(props: {
  name:
    | "arrow"
    | "pin"
    | "search"
    | "warning"
    | "close"
    | "edit"
    | "undo"
    | "wallet"
    | "eye"
    | "check";
  size?: number;
}): JSX.Element {
  const paths = {
    arrow: "m9 5 7 7-7 7",
    pin: "m16 3 5 5-4 1-3 4 1 3-3-1-4 3-1 4-5-5 4-1 3-4-1-3 3 1 4-3z",
    search: "m21 21-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
    warning: "M12 3 2 21h20L12 3z M12 9v5 M12 17h.01",
    close: "m6 6 12 12 M6 18 18 6",
    edit: "m16 3 5 5-13 13H3v-5L16 3z M14 5l5 5",
    undo: "M4 10h10a6 6 0 0 1 0 12 M4 10l6-6 M4 10l6 6",
    wallet: "M3 7h18v13H3V7z M3 7V4h14v3 M16 12h5v4h-5v-4z",
    eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0",
    check: "m5 12 4 4L19 6",
  };
  return (
    <svg
      aria-hidden="true"
      width={props.size ?? 16}
      height={props.size ?? 16}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d={paths[props.name]} />
    </svg>
  );
}
