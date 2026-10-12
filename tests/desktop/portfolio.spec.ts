import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { readFileSync } from "node:fs";
// Actual serialized Rust Session output, regenerated before this suite. Never shipped in app.
const fixture = JSON.parse(readFileSync(".cache/desktop-fixture.json", "utf8"));
async function install(page: Page, missing = false) {
  await page.addInitScript(
    ({ fixture, missing }) => {
      let state = structuredClone(fixture.startup);
      const selectionKey = (s: { kind: string; id?: string }) =>
        JSON.stringify(
          s.id === undefined ? { kind: s.kind } : { kind: s.kind, id: s.id },
        );
      Object.defineProperty(window, "isTauri", { value: true });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          invoke: async (command: string, args: Record<string, any>) => {
            const copy = (x: unknown) => structuredClone(x);
            switch (command) {
              case "startup":
                return missing
                  ? {
                      status: "missing",
                      settings: state.settings,
                      database: {
                        destination: "/temporary/portfolio.duckdb",
                        legacy_database: "/old/portfolio.duckdb",
                      },
                    }
                  : copy(state);
              case "start_new":
                state.data = {
                  ...state.data,
                  holding_count: 0,
                  assets: [],
                  providers: [],
                  dust_count: 0,
                };
                return copy(state);
              case "copy_database":
                return copy(state);
              case "allocation":
                return copy(fixture.trees[selectionKey(args.selection)]);
              case "details":
                return copy(
                  fixture.details[
                    `${selectionKey(args.selection)}:${args.nodeId}`
                  ],
                );
              case "save_settings":
                state.settings = args.settings;
                return copy(state);
              case "quit":
                return;
              default:
                throw Error("Unexpected desktop command");
            }
          },
        },
      });
    },
    { fixture, missing },
  );
}

test("real Rust DTO: Overview, scoped allocation, liability and exact unknown holdings", async ({
  page,
}) => {
  await install(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Partial subtotal").first()).toBeVisible();
  await page.getByRole("button", { name: /USD.*APY unknown/ }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(
    page.getByText("Liability · borrowed amount").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(
    page.getByRole("dialog").getByText("MYSTERY", { exact: false }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Close details" }).click();
  await expect(
    page.getByRole("button", { name: "My wallet", exact: true }),
  ).toBeFocused();
  await page.getByRole("button", { name: /MYSTERY.*APY unknown/ }).click();
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await expect(
    page
      .getByRole("dialog")
      .getByText("9007199254740993.000000000000000001 MYSTERY", {
        exact: true,
      }),
  ).toBeVisible();
});
test("group/search, privacy across details, persisted settings and accessibility", async ({
  page,
}) => {
  await install(page);
  await page.goto("/");
  await page.getByRole("tab", { name: "By provider" }).click();
  await page.getByRole("button", { name: /example.*APY unknown/ }).click();
  await expect(
    page.getByRole("button", { name: "My wallet", exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("table")
      .getByRole("button", { name: "example", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await page.getByLabel("Privacy mode").selectOption("hidden");
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await expect(
    page.getByRole("dialog").getByLabel("Quantity hidden").first(),
  ).toBeVisible();
  expect(await page.locator("body").innerText()).not.toContain(
    "9007199254740993",
  );
  expect(await page.getByRole("dialog").innerText()).not.toContain("$1,001");
  await page.getByRole("button", { name: "Close details" }).click();
  await page.getByLabel("Privacy mode").selectOption("scaled");
  await expect(
    page.getByText(
      "Scaled amounts for privacy. Percentage rates remain unchanged.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Theme", { exact: true }).selectOption("dark");
  await page.getByRole("button", { name: "Save settings" }).click();
  await expect(page.locator('[data-theme="dark"]')).toBeVisible();
  await page.getByLabel("Search assets or providers").fill("does not exist");
  await expect(
    page.getByRole("button", { name: /example.*APY unknown/ }),
  ).toHaveCount(0);
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
  await page.screenshot({ path: ".cache/desktop-review.png", fullPage: true });
});
test("missing DB chooser is explicit; browser alone never displays fixture money", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("alert")).toContainText("Local data unavailable");
  await expect(
    page.getByRole("heading", { name: "Overview", exact: true }),
  ).toHaveCount(0);
  await install(page, true);
  await page.reload();
  await expect(page.getByText("Choose your local portfolio")).toBeVisible();
  await page.getByRole("button", { name: "Start new portfolio" }).click();
  await expect(page.getByText("No imported holdings yet")).toBeVisible();
});
