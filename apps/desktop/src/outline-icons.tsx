// Audited Lucide paths; original SVGs and license are in assets/outline.
import { For } from "solid-js";
const paths = {
  growth: [
    "M12 16v5",
    "M16 14.639V21",
    "M20 10.656V21",
    "m22 3-8.646 8.646a.5.5 0 0 1-.708 0L9.354 8.354a.5.5 0 0 0-.707 0L2 15",
    "M4 18.463V21",
    "M8 14.656V21",
  ],
  USD: ["M12 2L12 22", "M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"],
  EUR: [
    "M4 10h12",
    "M4 14h9",
    "M19 6a7.7 7.7 0 0 0-5.2-2A7.9 7.9 0 0 0 6 12c0 4.4 3.5 8 7.8 8 2 0 3.8-.8 5.2-2",
  ],
  GBP: ["M18 7c0-5.333-8-5.333-8 0", "M10 7v14", "M6 21h12", "M6 13h10"],
} as const;
export function OutlineIcon(props: {
  name: keyof typeof paths;
  size?: number;
}) {
  return (
    <svg
      aria-hidden="true"
      width={props.size ?? 14}
      height={props.size ?? 14}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <For each={paths[props.name]}>{(d) => <path d={d} />}</For>
    </svg>
  );
}
