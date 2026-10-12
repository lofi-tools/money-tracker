import { defineTokens, defineSemanticTokens } from "@pandacss/dev";
export const tokens = defineTokens({
  fonts: {
    body: { value: '"Manrope Variable", Manrope, system-ui, sans-serif' },
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
    sm: { value: "12px" },
    md: { value: "16px" },
    lg: { value: "20px" },
    panel: { value: "26px" },
    frame: { value: "34px" },
    pill: { value: "999px" },
  },
  shadows: {
    panel: { value: "0 24px 60px #00000038" },
    tile: { value: "0 10px 24px #00000024" },
  },
});
export const semanticTokens = defineSemanticTokens({
  colors: {
    canvas: { value: { base: "#deded5", _dark: "#6e6e6a" } },
    surface: { value: { base: "#f8f8f2", _dark: "#191918" } },
    inset: { value: { base: "#f0f0e9", _dark: "#141413" } },
    subtle: { value: { base: "#e7e9e0", _dark: "#232321" } },
    hover: { value: { base: "#dbe2d6", _dark: "#2c2c2a" } },
    text: { value: { base: "#20221e", _dark: "#f4f4f1" } },
    muted: { value: { base: "#5b6055", _dark: "#a3a39e" } },
    border: { value: { base: "#bfc5b7", _dark: "#ffffff18" } },
    accent: { value: { base: "#b7d3bf", _dark: "#b7d3bf" } },
    accentText: { value: { base: "#35583f", _dark: "#b7d3bf" } },
    onAccent: { value: { base: "#22332a", _dark: "#22332a" } },
    accentSubtle: { value: { base: "#dcebdd", _dark: "#2b342d" } },
    warning: { value: { base: "#80551b", _dark: "#edc78d" } },
    warningSurface: { value: { base: "#f3e7cd", _dark: "#3a3022" } },
    danger: { value: { base: "#9e3942", _dark: "#edacaf" } },
    dangerSurface: { value: { base: "#f5e0df", _dark: "#3b2627" } },
  },
});
