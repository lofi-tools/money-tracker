// Monochrome geometry adapted from the bundled, licensed coin SVGs.
// Original sources and licenses are in assets/coins; no background circles.
import { For, Show, createSignal, onMount, onCleanup } from "solid-js";
const glyphs: Record<string, string> = {
  USD: "$",
  EUR: "€",
  GBP: "£",
  BTC: "₿",
  JPY: "¥",
  CNY: "¥",
  INR: "₹",
  KRW: "₩",
  RUB: "₽",
  TRY: "₺",
};
const shapes: Record<string, { d: string; transform: string }[]> = {
  SOL: [
    {
      d: "M9.925 19.687a.59.59 0 01.415-.17h14.366a.29.29 0 01.207.497l-2.838 2.815a.59.59 0 01-.415.171H7.294a.291.291 0 01-.207-.498l2.838-2.815zm0-10.517A.59.59 0 0110.34 9h14.366c.261 0 .392.314.207.498l-2.838 2.815a.59.59 0 01-.415.17H7.294a.291.291 0 01-.207-.497L9.925 9.17zm12.15 5.225a.59.59 0 00-.415-.17H7.294a.291.291 0 00-.207.498l2.838 2.815c.11.109.26.17.415.17h14.366a.291.291 0 00.207-.498l-2.838-2.815z",
      transform: "",
    },
  ],
  ETH: [
    { d: "M16.498 4v8.87l7.497 3.35z", transform: "" },
    { d: "M16.498 4L9 16.22l7.498-3.35z", transform: "" },
    { d: "M16.498 21.968v6.027L24 17.616z", transform: "" },
    { d: "M16.498 27.995v-6.028L9 17.616z", transform: "" },
    { d: "M16.498 20.573l7.497-4.353-7.497-3.348z", transform: "" },
    { d: "M9 16.22l7.498 4.353v-7.701z", transform: "" },
  ],
  BNB: [
    {
      d: "M12.116 14.404L16 10.52l3.886 3.886 2.26-2.26L16 6l-6.144 6.144 2.26 2.26zM6 16l2.26-2.26L10.52 16l-2.26 2.26L6 16zm6.116 1.596L16 21.48l3.886-3.886 2.26 2.259L16 26l-6.144-6.144-.003-.003 2.263-2.257zM21.48 16l2.26-2.26L26 16l-2.26 2.26L21.48 16zm-3.188-.002h.002V16L16 18.294l-2.291-2.29-.004-.004.004-.003.401-.402.195-.195L16 13.706l2.293 2.293z",
      transform: "",
    },
  ],
  NEAR: [
    {
      d: "m421.61,108c-13,0-25.07,6.74-31.88,17.82l-73.37,108.93c-2.39,3.59-1.42,8.43,2.17,10.82,2.91,1.94,6.76,1.7,9.41-.58l72.22-62.64c1.2-1.08,3.05-.97,4.13.23.49.55.75,1.26.75,1.99v196.12c0,1.62-1.31,2.92-2.93,2.92-.87,0-1.69-.38-2.24-1.05L181.56,121.24c-7.11-8.39-17.55-13.23-28.54-13.24h-7.63c-20.65,0-37.39,16.74-37.39,37.39v276.22c0,20.65,16.74,37.39,37.39,37.39,13,0,25.07-6.74,31.88-17.82l73.37-108.93c2.39-3.59,1.42-8.43-2.17-10.82-2.91-1.94-6.76-1.7-9.41.58l-72.22,62.64c-1.2,1.08-3.05.97-4.13-.23-.49-.55-.75-1.26-.74-1.99v-196.17c0-1.62,1.31-2.92,2.93-2.92.86,0,1.69.38,2.24,1.05l218.28,261.37c7.11,8.39,17.55,13.23,28.54,13.24h7.63c20.65.01,37.4-16.72,37.42-37.37V145.39c0-20.65-16.74-37.39-37.39-37.39Z",
      transform: "translate(6 6) scale(.057) translate(-108 -108)",
    },
  ],
  NEXO: [
    {
      d: "M10.676 7.125l10.732 6.191v6.319L5 10.159l5.265-3.034a.42.42 0 01.42 0",
      transform: "",
    },
    { d: "M21.408 7l-5.467 3.16 5.467 3.156z", transform: "" },
    {
      d: "M21.408 7l5.264 3.036a.42.42 0 01.22.367v12.389l-5.484-3.157V7z",
      transform: "",
    },
    {
      d: "M26.881 22.792l-5.264 3.033a.44.44 0 01-.42 0l-10.732-6.19v-6.328l16.416 9.485z",
      transform: "",
    },
    {
      d: "M5 10.16v12.387a.42.42 0 00.22.368l5.265 3.036V13.307L5 10.16z",
      transform: "",
    },
    { d: "M10.476 25.95l5.465-3.158-5.465-3.157z", transform: "" },
  ],
  DOT: [
    {
      d: "M16.272 6.625c-3.707 0-6.736 3.012-6.736 6.736 0 .749.124 1.48.356 2.192a.95.95 0 001.194.589.95.95 0 00.588-1.194 4.745 4.745 0 01-.267-1.73c.071-2.512 2.103-4.58 4.616-4.704a4.86 4.86 0 015.115 4.847 4.862 4.862 0 01-4.58 4.848s-.945.053-1.408.125c-.232.035-.41.071-.535.089-.054.018-.107-.036-.09-.09l.161-.783.873-4.028a.934.934 0 00-.712-1.105.934.934 0 00-1.105.713s-2.103 9.802-2.121 9.909a.934.934 0 00.713 1.105.934.934 0 001.105-.713c.017-.107.303-1.408.303-1.408a2.367 2.367 0 011.996-1.854 21.43 21.43 0 011.051-.089 6.744 6.744 0 006.22-6.719c0-3.724-3.03-6.736-6.737-6.736zm.481 15.505a1.122 1.122 0 00-1.336.873c-.125.606.25 1.212.873 1.337a1.122 1.122 0 001.337-.874c.124-.623-.25-1.212-.874-1.336z",
      transform: "",
    },
  ],
  POL: [
    {
      d: "M21.092 12.693c-.369-.215-.848-.215-1.254 0l-2.879 1.654-1.955 1.078-2.879 1.653c-.369.216-.848.216-1.254 0l-2.288-1.294c-.369-.215-.627-.61-.627-1.042V12.19c0-.431.221-.826.627-1.042l2.25-1.258c.37-.216.85-.216 1.256 0l2.25 1.258c.37.216.628.611.628 1.042v1.654l1.955-1.115v-1.653a1.16 1.16 0 00-.627-1.042l-4.17-2.372c-.369-.216-.848-.216-1.254 0l-4.244 2.372A1.16 1.16 0 006 11.076v4.78c0 .432.221.827.627 1.043l4.244 2.372c.369.215.849.215 1.254 0l2.879-1.618 1.955-1.114 2.879-1.617c.369-.216.848-.216 1.254 0l2.251 1.258c.37.215.627.61.627 1.042v2.552c0 .431-.22.826-.627 1.042l-2.25 1.294c-.37.216-.85.216-1.255 0l-2.251-1.258c-.37-.216-.628-.611-.628-1.042v-1.654l-1.955 1.115v1.653c0 .431.221.827.627 1.042l4.244 2.372c.369.216.848.216 1.254 0l4.244-2.372c.369-.215.627-.61.627-1.042v-4.78a1.16 1.16 0 00-.627-1.042l-4.28-2.409z",
      transform: "",
    },
  ],
};
export function hasCurrencySymbol(asset: string) {
  return !!glyphs[asset] || !!shapes[asset];
}
export function CurrencySymbol(props: { asset: string }) {
  let svg: SVGSVGElement | undefined;
  let geometry: SVGGElement | undefined;
  const [opticalScale, setOpticalScale] = createSignal(1);
  const [inkBox, setInkBox] = createSignal({ x: 0, width: 32 });
  onMount(() => {
    if (!svg || !geometry) return;
    const bounds = geometry.getBBox();
    const source = new XMLSerializer()
      .serializeToString(svg)
      .replaceAll("currentColor", "#000");
    const url = URL.createObjectURL(
      new Blob([source], { type: "image/svg+xml" }),
    );
    const image = new Image();
    let disposed = false;
    onCleanup(() => {
      disposed = true;
      URL.revokeObjectURL(url);
    });
    image.onload = () => {
      URL.revokeObjectURL(url);
      if (disposed) return;
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 64;
      const context = canvas.getContext("2d");
      if (!context) return;
      context.drawImage(image, 0, 0, 64, 64);
      const pixels = context.getImageData(0, 0, 64, 64).data;
      let ink = 0;
      for (let i = 3; i < pixels.length; i += 4) ink += pixels[i] / 255;
      // Equalize painted area, with a cap so no symbol escapes its text line.
      const scale = Math.min(
        1.3,
        25.6 / Math.max(bounds.width, bounds.height),
        Math.sqrt((0.12 * 4096) / Math.max(ink, 1)),
      );
      const adjusted = Math.max(0.65, scale);
      setOpticalScale(adjusted);
      const strokePadding =
        props.asset === "ETH" || props.asset === "NEXO" ? 0.8 : 0;
      setInkBox({
        x: 16 + (bounds.x - 16 - strokePadding) * adjusted,
        width: (bounds.width + strokePadding * 2) * adjusted,
      });
    };
    image.src = url;
  });
  return (
    <Show
      when={glyphs[props.asset]}
      fallback={
        <svg
          ref={svg}
          class="crypto-quantity-symbol"
          data-optical-scale={opticalScale()}
          aria-hidden="true"
          viewBox={`${inkBox().x} 0 ${inkBox().width} 32`}
          style={{ width: `calc(1lh * ${inkBox().width / 32})` }}
          width="1em"
          height="1em"
          fill={
            props.asset === "ETH" || props.asset === "NEXO"
              ? "none"
              : "currentColor"
          }
          stroke={
            props.asset === "ETH" || props.asset === "NEXO"
              ? "currentColor"
              : "none"
          }
          stroke-width="1.6"
          stroke-linejoin="round"
        >
          <g
            ref={geometry}
            transform={`translate(16 16) scale(${opticalScale()}) translate(-16 -16)`}
          >
            <For each={shapes[props.asset] ?? []}>
              {(p) => <path d={p.d} transform={p.transform || undefined} />}
            </For>
          </g>
        </svg>
      }
    >
      {(glyph) => (
        <span class="currency-quantity-glyph" aria-hidden="true">
          {glyph()}
        </span>
      )}
    </Show>
  );
}
