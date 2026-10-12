import { render } from "solid-js/web";
import { Catalog } from "./Catalog";
import { PortfolioShowcase } from "./PortfolioShowcase";
import "@money-tracker/design-system/fonts.css";
import "@money-tracker/design-system/styles.css";
import "./catalog.css";
render(
  () =>
    location.pathname.startsWith("/portfolio") ? (
      <PortfolioShowcase />
    ) : (
      <Catalog />
    ),
  document.getElementById("root")!,
);
