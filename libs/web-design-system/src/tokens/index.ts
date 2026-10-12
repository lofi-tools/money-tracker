import { defineTokens, defineSemanticTokens } from "@pandacss/dev";
export const tokens = defineTokens({
  fonts: {
    body: {
      value:
        'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    },
  },
  fontSizes: {
    xs: { value: "0.75rem" },
    sm: { value: "0.875rem" },
    md: { value: "1rem" },
    lg: { value: "1.25rem" },
    xl: { value: "1.75rem" },
    display: { value: "2.75rem" },
  },
  spacing: {
    1: { value: "0.25rem" },
    2: { value: "0.5rem" },
    3: { value: "0.75rem" },
    4: { value: "1rem" },
    5: { value: "1.25rem" },
    6: { value: "1.5rem" },
    8: { value: "2rem" },
    9: { value: "2.25rem" },
    10: { value: "2.5rem" },
    12: { value: "3rem" },
  },
  radii: {
    sm: { value: "0.375rem" },
    md: { value: "0.625rem" },
    lg: { value: "0.875rem" },
  },
});
export const semanticTokens = defineSemanticTokens({
  colors: {
    canvas: { value: { base: "#f7f9fc", _dark: "#10141b" } },
    surface: { value: { base: "#ffffff", _dark: "#171e28" } },
    subtle: { value: { base: "#eef2f7", _dark: "#202936" } },
    hover: { value: { base: "#e5ebf2", _dark: "#283443" } },
    text: { value: { base: "#1b2735", _dark: "#edf2f7" } },
    muted: { value: { base: "#596577", _dark: "#adb8c5" } },
    border: { value: { base: "#cdd6e1", _dark: "#364357" } },
    accent: { value: { base: "#00796e", _dark: "#7ce2ce" } },
    onAccent: { value: { base: "#ffffff", _dark: "#082822" } },
    accentSubtle: { value: { base: "#e1f4ef", _dark: "#173b36" } },
    warning: { value: { base: "#85581a", _dark: "#f7cf82" } },
    warningSurface: { value: { base: "#fff7e6", _dark: "#292014" } },
    danger: { value: { base: "#b32e39", _dark: "#ffacb4" } },
    dangerSurface: { value: { base: "#fff0f2", _dark: "#34171f" } },
  },
});
