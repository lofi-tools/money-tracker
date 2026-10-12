import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { readFileSync } from "node:fs";
// Actual serialized Rust Session output, regenerated before this suite. Never shipped in app.
const fixture = JSON.parse(readFileSync(".cache/desktop-fixture.json", "utf8"));
async function install(page: Page, missing = false) {
  await page.addInitScript(
    ({ fixture, missing }) => {
      let state = structuredClone(fixture.startup);
      const applied: Record<string, unknown>[] = [];
      const selectionKey = (s: { kind: string; id?: string }) =>
        JSON.stringify(
          s.id === undefined ? { kind: s.kind } : { kind: s.kind, id: s.id },
        );
      Object.defineProperty(window, "isTauri", { value: true });
      Object.defineProperty(window, "__edits", { value: applied });
      const targetOf = (edit: Record<string, any>) => {
        switch (edit.kind) {
          case "rate":
          case "rate_pin":
            return { kind: "rate", scope: edit.scope };
          case "price":
          case "price_pin":
            return { kind: "price", asset: edit.asset, quote: edit.quote };
          case "dust":
            return { kind: "dust", provider: edit.provider, asset: edit.asset };
          case "spam":
            return { kind: "spam", asset: edit.asset };
          case "subscription_name":
            return { kind: "subscription", id: edit.id };
          default:
            return { kind: "account", id: edit.account ?? "local group" };
        }
      };
      const withAction = (target: Record<string, unknown>) => {
        state.data = {
          ...state.data,
          edits: {
            ...state.data.edits,
            actions: [
              {
                id: "manual-action-1",
                at: new Date().toISOString(),
                undone: false,
                target,
                undo_of: null,
                revision: 1,
                before: null,
                after: null,
              },
              ...state.data.edits.actions,
            ],
            local_state: [[target, { revision: 1, value: null }]],
          },
        };
      };
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
              // A refresh pass returns the new snapshot plus typed per-source outcomes.
              case "refresh": {
                const at = new Date().toISOString();
                state.data = {
                  ...state.data,
                  refresh: [
                    {
                      source: "prices",
                      label: "Current prices (USD)",
                      ready: true,
                      prerequisite: null,
                      last_attempt: at,
                      last_success: at,
                      next_due: new Date(Date.now() + 86_400_000).toISOString(),
                      last_error: null,
                    },
                    {
                      source: "provider:NEXO",
                      label: "Nexo export",
                      ready: false,
                      prerequisite:
                        "No readable Nexo export is configured; refreshing Nexo re-reads a CSV export and cannot fetch live balances.",
                      last_attempt: at,
                      last_success: null,
                      next_due: null,
                      last_error: null,
                    },
                  ],
                };
                return {
                  startup: copy(state),
                  report: {
                    at,
                    trigger: args.trigger,
                    interval_seconds: 86_400,
                    in_flight: false,
                    entries: [
                      {
                        source: "prices",
                        label: "Current prices (USD)",
                        outcome: {
                          status: "updated",
                          detail: "2 exact current prices",
                        },
                      },
                      {
                        source: "provider:NEXO",
                        label: "Nexo export",
                        outcome: {
                          status: "failed",
                          code: "acquisition_failed",
                          message:
                            "the source could not be reached or refused the request from this app",
                        },
                      },
                    ],
                  },
                };
              }
              // Browser previews echo the request into the ordered action log.
              case "manual_edit":
                applied.push(args.edit);
                withAction(targetOf(args.edit));
                return copy(state);
              case "undo_action":
                state.data = {
                  ...state.data,
                  edits: {
                    ...state.data.edits,
                    actions: state.data.edits.actions.map((action: any) =>
                      action.id === args.id
                        ? { ...action, undone: true }
                        : action,
                    ),
                  },
                };
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
  await page.getByRole("button", { name: /US USD/ }).click();
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
  await page.getByRole("button", { name: /MY MYSTERY/ }).click();
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
  await page.getByRole("button", { name: /EX example/ }).click();
  await expect(
    page.getByRole("button", { name: "My wallet", exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("table")
      .getByRole("button", { name: "example", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Privacy mode").selectOption("hidden");
  await page.getByRole("button", { name: "Save settings" }).click();
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await expect(
    page.getByRole("dialog").getByLabel("Quantity hidden").first(),
  ).toBeVisible();
  expect(await page.locator("body").innerText()).not.toContain(
    "9007199254740993",
  );
  expect(await page.getByRole("dialog").innerText()).not.toContain("$1,001");
  await page.getByRole("button", { name: "Close details" }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Privacy mode").selectOption("scaled");
  await page.getByRole("button", { name: "Save settings" }).click();
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
  await expect(page.getByRole("button", { name: /EX example/ })).toHaveCount(0);
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
test("647px window contains independent panes and keeps Settings reachable", async ({
  page,
}) => {
  await page.setViewportSize({ width: 647, height: 724 });
  await install(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Quit", exact: true }),
  ).toHaveCount(0);
  await expect(page.getByLabel("Privacy mode")).toHaveCount(0);
  await page.getByRole("button", { name: /US USD/ }).click();
  const nav = page.getByRole("navigation", { name: "Portfolio navigation" });
  const settings = nav.getByRole("button", { name: "Settings", exact: true });
  await expect(settings).toBeVisible();
  const bounds = await settings.boundingBox();
  expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(724);
  expect(
    await page.evaluate(
      () =>
        document.documentElement.scrollHeight <= window.innerHeight &&
        document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  const panes = await page
    .locator(".sidebar-list,.main-pane")
    .evaluateAll((nodes) =>
      nodes.map((node) => ({
        client: node.clientHeight,
        scroll: node.scrollHeight,
        overflow: getComputedStyle(node).overflowY,
      })),
    );
  expect(panes.every((p) => p.overflow === "auto" && p.client > 0)).toBe(true);
  expect(panes[1].scroll).toBeGreaterThan(panes[1].client);
  await page.locator(".main-pane").evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  await expect(settings).toBeVisible();
  await settings.click();
  await expect(settings).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("main").getByRole("heading", { name: "Settings" }),
  ).toBeVisible();
  await expect(
    page.getByRole("main").getByRole("table", { name: "Current allocations" }),
  ).toHaveCount(0);
  await expect(page.getByLabel("Privacy mode")).toBeVisible();
  await expect(page.getByLabel("Privacy scale (fraction)")).toHaveCount(0);
  await page.getByLabel("Privacy mode").selectOption({ label: "Scaled down" });
  await expect(page.getByLabel("Privacy scale (fraction)")).toBeVisible();
  await page.getByLabel("Privacy mode").selectOption("hidden");
  await expect(page.getByLabel("Privacy scale (fraction)")).toHaveCount(0);
  await page.getByRole("button", { name: "Save settings" }).click();
  await expect(page.getByLabel("Quantity hidden").first()).toBeVisible();
  await page.screenshot({
    path: ".cache/desktop-647-review.png",
    fullPage: true,
  });
});

test("refresh reports each source and retains cached figures", async ({
  page,
}) => {
  await install(page);
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Portfolio navigation" });
  const usd = nav.getByRole("button", { name: /US USD/ });
  const before = await usd.innerText();
  await nav.getByRole("button", { name: "Settings", exact: true }).click();
  const status = page.getByRole("region", { name: "Refresh status" });
  await status.getByRole("button", { name: "Refresh now" }).click();
  // Failures name their source and carry redacted text, never provider detail.
  const notice = page.getByRole("status");
  await expect(notice).toContainText("Nexo export: refresh failed");
  await expect(notice).toContainText("2 exact current prices");
  await expect(status).toContainText("Current prices (USD)");
  await expect(status).toContainText("No readable Nexo export is configured");
  expect(await usd.innerText()).toBe(before);
  await page.screenshot({ path: ".cache/desktop-refresh-review.png" });
});

test("manual rate, price, dust and spam edits commit typed requests", async ({
  page,
}) => {
  await install(page);
  await page.goto("/");
  await page.getByRole("button", { name: /US USD/ }).click();
  await expect(page.getByRole("table")).toBeVisible();
  // Inline APY editing lives on leaf holding rows with a dated, pinned override.
  const rateEdit = page
    .getByRole("button", { name: "Edit Annual rate" })
    .first();
  await expect(rateEdit).toBeVisible();
  await rateEdit.click();
  const rateInput = page.getByRole("textbox", { name: "Annual rate" }).first();
  await rateInput.fill("6.5");
  await rateInput.press("Enter");
  await expect
    .poll(() => page.evaluate(() => (window as any).__edits.length))
    .toBe(1);
  const sent = await page.evaluate(() => (window as any).__edits[0]);
  expect(sent.kind).toBe("rate");
  expect(sent.rate).toMatchObject({
    fraction: "0.065",
    basis: "apy",
    compounds_per_year: null,
    reward_asset: null,
  });
  expect(sent.effective_at).toMatch(/^\d{4}-\d{2}-\d{2}T00:00:00Z$/);
  expect(sent.pinned).toBe(false);
  expect(sent.clear).toBe(false);
  // The effective date is editable and defaults to today.
  const effective = page.getByLabel("Effective from").first();
  await expect(effective).toHaveValue(new Date().toISOString().slice(0, 10));
  // Prices are edited in the reporting currency alongside their pin; the
  // reporting asset itself has no quote and therefore no price override.
  await page.getByRole("button", { name: "MY MYSTERY Unknown" }).click();
  const priceEdit = page
    .getByRole("button", { name: "Edit Current price" })
    .first();
  await priceEdit.click();
  const priceInput = page
    .getByRole("textbox", { name: "Current price" })
    .first();
  await priceInput.fill("1.25");
  await priceInput.press("Enter");
  await expect
    .poll(() => page.evaluate(() => (window as any).__edits.length))
    .toBe(2);
  expect(await page.evaluate(() => (window as any).__edits[1])).toMatchObject({
    kind: "price",
    price: "1.25",
  });
  await page.getByRole("button", { name: "My wallet", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  // Dust, spam and subscription naming are scoped manual controls.
  const dust = page.getByRole("group", { name: "Dust classification" }).first();
  await expect(dust).toBeVisible();
  await dust.getByRole("button", { name: "Significant" }).click();
  const spam = page.getByRole("group", { name: "Spam exclusion" }).first();
  await expect(spam).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => (window as any).__edits.length))
    .toBe(3);
  expect(await page.evaluate(() => (window as any).__edits[2])).toMatchObject({
    kind: "dust",
    mode: "significant",
  });
  await spam.getByRole("textbox").fill("unwanted airdrop");
  await spam.getByRole("button", { name: "Exclude as spam" }).click();
  await expect
    .poll(() => page.evaluate(() => (window as any).__edits.length))
    .toBe(4);
  expect(await page.evaluate(() => (window as any).__edits[3])).toMatchObject({
    kind: "spam",
    excluded: true,
    reason: "unwanted airdrop",
  });
  await page.getByRole("button", { name: "Close details" }).click();
  // The action log exposes the change with a working undo link.
  await page.getByRole("button", { name: "Change history" }).click();
  await expect(
    page.getByRole("heading", { name: "Change history" }),
  ).toBeVisible();
  await expect(
    page.getByText("Manual rate", { exact: false }).first(),
  ).toBeVisible();
  const undo = page.getByRole("button", { name: "Undo" }).first();
  await expect(undo).toBeVisible();
  await undo.click();
  await expect(page.getByText("Undone").first()).toBeVisible();
});
