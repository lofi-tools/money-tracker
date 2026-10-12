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
    | "check"
    | "leaf"
    | "clock"
    | "home"
    | "user"
    | "sliders"
    | "more"
    | "up"
    | "down"
    | "card"
    | "gear";
  size?: number;
}): JSX.Element {
  const paths = {
    gear: "M10 3h4l.6 3 2 .9 2.7-1.1 2 3.4-2.2 1.9v2l2.2 1.9-2 3.4-2.7-1.1-2 .9-.6 3h-4l-.6-3-2-.9-2.7 1.1-2-3.4 2.2-1.9v-2L2.7 9.3l2-3.4 2.7 1.1 2-.9.6-3z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0",

    arrow: "m9 5 7 7-7 7",
    pin: "m16 3 5 5-4 1-3 4 1 3-3-1-4 3-1 4-5-5 4-1 3-4-1-3 3 1 4-3z",
    search: "m21 21-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
    warning: "M12 3 2 21h20L12 3z M12 9v5 M12 17h.01",
    close: "m6 6 12 12 M6 18 18 6",
    edit: "m16 3 5 5-13 13H3v-5L16 3z M14 5l5 5",
    undo: "M4 10h10a6 6 0 0 1 0 12 M4 10l6-6 M4 10l6 6",
    wallet: "M3 7h18v13H3V7z M3 7V4h14v3 M16 12h5v4h-5v-4z",
    eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0",
    leaf: "M6 21c0-9 5-14 14-15-1 9-6 14-14 15z M6 21c4-6 8-10 12-13",
    clock: "M20 12a8 8 0 1 1-16 0 8 8 0 0 1 16 0 M12 8v4l3 2",
    home: "M4 11l8-7 8 7 M6 10v10h12V10",
    user: "M15.5 8a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0 M5 20a7 7 0 0 1 14 0",
    sliders:
      "M4 7h3 M11 7h9 M4 17h9 M17 17h3 M11 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0 M17 17a2 2 0 1 1-4 0 2 2 0 0 1 4 0",
    more: "M12 5h.01 M12 12h.01 M12 19h.01",
    up: "M7 17 17 7 M10 7h7v7",
    down: "m7 7 10 10 M17 10v7h-7",
    card: "M3 6h18v13H3z M3 10h18 M6 15h3",
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
