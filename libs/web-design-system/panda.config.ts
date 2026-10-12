import { defineConfig, defineRecipe } from "@pandacss/dev";
import { tokens, semanticTokens } from "./src/tokens/index";
const focus = {
  outline: "2px solid",
  outlineColor: "accentText",
  outlineOffset: "3px",
};
export default defineConfig({
  presets: ["@pandacss/preset-base"],
  preflight: true,
  prefix: "mt",
  include: ["./src/**/*.{ts,tsx}"],
  outdir: "styled-system",
  jsxFramework: "solid",
  conditions: {
    extend: {
      dark: "[data-theme=dark] &",
      currentPage: "&[aria-current=page]",
    },
  },
  theme: {
    extend: {
      tokens,
      semanticTokens,
      keyframes: {
        "mt-panel-in": {
          from: { transform: "translateX(100%)" },
          to: { transform: "translateX(0)" },
        },
      },
      recipes: {
        button: defineRecipe({
          className: "button",
          base: {
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            gap: "2",
            borderRadius: "pill",
            fontWeight: "700",
            cursor: "pointer",
            whiteSpace: "nowrap",
            border: "1px solid",
            borderColor: "transparent",
            transition: "box-shadow 180ms, transform 180ms",
            _focusVisible: focus,
            _disabled: { opacity: 0.5, cursor: "not-allowed" },
          },
          variants: {
            intent: {
              primary: {
                bg: "accent",
                color: "onAccent",
                _hover: { filter: "brightness(0.94)" },
              },
              secondary: {
                bg: "subtle",
                color: "text",
                borderColor: "border",
                _hover: { bg: "subtle" },
              },
              ghost: {
                color: "muted",
                bg: "transparent",
                _hover: { bg: "subtle", color: "text" },
              },
              danger: {
                bg: "dangerSurface",
                color: "danger",
                borderColor: "danger",
              },
            },
            size: {
              sm: { minHeight: "32px", px: "2", fontSize: "xs" },
              md: { minHeight: "40px", px: "3", fontSize: "sm" },
            },
          },
          defaultVariants: { intent: "secondary", size: "md" },
        }),
        badge: defineRecipe({
          className: "badge",
          base: {
            display: "inline-flex",
            alignItems: "center",
            gap: "1",
            borderRadius: "pill",
            px: "2",
            py: "1",
            fontSize: "xs",
            fontWeight: "600",
          },
          variants: {
            tone: {
              neutral: { color: "muted", bg: "subtle" },
              accent: { color: "accentText", bg: "accentSubtle" },
              warning: { color: "warning", bg: "warningSurface" },
              danger: { color: "danger", bg: "dangerSurface" },
            },
          },
          defaultVariants: { tone: "neutral" },
        }),
      },
    },
  },
  staticCss: { recipes: "*" },
  globalCss: {
    ":root": { fontFamily: "body", WebkitFontSmoothing: "antialiased" },
    body: { margin: 0 },
    "[data-theme]": {
      color: "text",
      bg: "canvas",
      fontFamily: "body",
      colorScheme: "light",
    },
    "[data-theme=dark]": { colorScheme: "dark" },
    "::selection": { bg: "accent", color: "onAccent" },
    "button, input, select": { fontFamily: "inherit" },
    "button, a, input, select": { _focusVisible: focus },
    "@media (prefers-reduced-motion: reduce)": {
      "*, *::before, *::after": {
        animationDuration: "0.01ms !important",
        transitionDuration: "0.01ms !important",
        scrollBehavior: "auto !important",
      },
    },
  },
});
