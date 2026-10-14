import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
// Actual synthetic Rust Session output; never load private provider data in tests.
const fixture = JSON.parse(readFileSync(".cache/desktop-fixture.json", "utf8"));
test("review navigation works after reload, fills the viewport and keeps history inside Settings", async ({
  page,
}) => {
  const legacy = structuredClone(fixture);
  delete legacy.startup.data.edits;
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/__portfolio_review", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(legacy),
    }),
  );
  await page.goto("/");
  const frame = page.locator(".desktop");
  await expect(frame).toBeVisible();
  expect(await frame.boundingBox()).toEqual({
    x: 0,
    y: 0,
    width: 968,
    height: 724,
  });
  await expect(frame).toHaveCSS("border-radius", "0px");
  const nav = page.getByRole("navigation", { name: "Portfolio navigation" });
  const search = page.locator(".app-header").getByRole("searchbox");
  await expect(search).toBeVisible();
  await expect(nav.getByRole("searchbox")).toHaveCount(0);
  await search.fill("not-an-asset");
  await expect(nav.getByRole("button", { name: /^USD/ })).toHaveCount(0);
  await search.fill("");
  const card = page.locator(".overview-card");
  await expect(card).toContainText("$901");
  const cardBounds = await card.boundingBox();
  const navBounds = await nav.boundingBox();
  expect(cardBounds!.x + cardBounds!.width).toBeGreaterThan(
    navBounds!.x + navBounds!.width,
  );
  expect(cardBounds!.x + cardBounds!.width).toBeLessThan(
    navBounds!.x + navBounds!.width + 20,
  );

  await expect(nav.getByRole("button", { name: "Change history" })).toHaveCount(
    0,
  );
  for (let pass = 0; pass < 2; pass++) {
    await nav.getByRole("button", { name: /^USD/ }).click();
    await expect(
      page.getByRole("main").getByRole("heading", { name: "USD", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("table", { name: "Current allocations" }),
    ).toBeVisible();
    await page.locator(".main-pane").evaluate((node) => {
      node.scrollTop = node.scrollHeight;
    });
    await nav.getByRole("button", { name: "Settings", exact: true }).click();
    await expect(
      page
        .getByRole("main")
        .getByRole("heading", { name: "Settings", exact: true }),
    ).toBeInViewport();
    await expect(page.getByLabel("Privacy mode")).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Change history", exact: true }),
    ).toHaveCount(1);
    await expect(
      page.getByText("Change history unavailable", { exact: true }),
    ).toHaveCount(1);
    await nav.getByRole("button", { name: /^OV Overview/ }).click();
    await expect(
      page
        .getByRole("main")
        .getByRole("heading", { name: "Overview", exact: true }),
    ).toBeInViewport();
    if (!pass) await page.reload();
  }
  expect(errors).toEqual([]);
});

test("main asset SVG icons and cached token labels remain usable offline", async ({
  page,
}) => {
  const data = structuredClone(fixture);
  const asset = data.startup.data.assets.find(
    (e: any) => e.selection.id === "USD",
  )!;
  const id = "eip155:1/erc20:0x1111111111111111111111111111111111111111";
  const original = JSON.stringify({
    kind: asset.selection.kind,
    id: asset.selection.id,
  });
  const selected = { kind: "asset", id };
  data.trees[JSON.stringify(selected)] = structuredClone(data.trees[original]);
  data.trees[JSON.stringify(selected)].selection = selected;
  data.startup.data.assets.push({
    ...asset,
    selection: selected,
    name: "Cached token",
    ticker: "CACHE",
    token_metadata: {
      asset_id: id,
      chain_id: 1,
      contract: id.split(":").at(-1),
      name: "Cached token",
      ticker: "CACHE",
      icon_url: "https://example.invalid/icon.svg",
      icon_data: "data:image/svg+xml,<svg onload='alert(1)'/>",
      decimals: 18,
      rpc_decimals: null,
      coingecko_id: null,
      fetched_at: "2026-10-13T12:00:00Z",
      sources: {},
      warnings: ["Current RPC decimals unavailable; ledger scale preserved"],
    },
  });
  data.startup.data.assets.push({
    ...asset,
    selection: {
      kind: "asset",
      id: "eip155:1/erc20:0x2222222222222222222222222222222222222222",
    },
    name: "Unknown token",
    ticker: null,
    metadata_unresolved: true,
  });
  const solSelection = { kind: "asset", id: "SOL" };
  data.startup.data.assets.push({
    ...asset,
    selection: solSelection,
    name: "SOL",
    ticker: "SOL",
    direct_quantities: [{ asset_id: "SOL", units: "35259", scale: 3 }],
  });
  const external: string[] = [];
  page.on("request", (req) => {
    if (
      !req.url().startsWith("http://127.0.0.1:") &&
      !req.url().startsWith("data:")
    )
      external.push(req.url());
  });
  await page.route("**/__portfolio_review", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(data),
    }),
  );
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Portfolio navigation" });
  await expect(nav.getByRole("button", { name: /Unknown token/ })).toHaveCount(
    0,
  );
  const mainAsset = nav.getByRole("button", { name: /^USD/ }).first();
  await expect(
    mainAsset.locator("..").locator(".currency-quantity-glyph"),
  ).toHaveText("$");
  await expect(
    mainAsset.locator("..").locator(".nav-entry-icon img"),
  ).toHaveAttribute("src", /^(data:image\/svg\+xml,|.*\/usd\.svg)/);
  const solNav = nav.getByRole("button", { name: /^SOL/ }).first();
  await expect(
    solNav.locator("..").locator(".nav-entry-icon img"),
  ).toHaveAttribute("src", /^(data:image\/svg\+xml,|.*\/sol\.svg)/);
  await expect(nav.locator(".others-dust-entry img")).toHaveAttribute(
    "src",
    /others|data:image/,
  );
  await nav.getByRole("button", { name: /^Cached token CACHE/ }).click();
  await page.getByText("Token information · cached metadata").click();
  await expect(page.locator(".token-metadata")).toContainText(
    "RPC decimals: Unavailable",
  );
  await expect(page.locator(".token-metadata")).toContainText(
    "Unverified / unavailable",
  );
  await expect(
    nav.getByRole("button", { name: /Cached token CACHE/ }).locator("img"),
  ).toHaveCount(0);
  expect(await page.getByRole("main").innerText()).not.toContain(id);
  expect(external).toEqual([]);
});

test("direct quantities replace representation counts and wrapped holdings expand separately", async ({
  page,
}) => {
  const data = structuredClone(fixture);
  const original = data.startup.data.assets[0];
  const wrapped =
    "eip155:42161/erc20:0x82af49447d8a07e3bd95bd0d56f35241523fbab1";
  const eth = {
    ...original,
    summary: {
      ...original.summary,
      estimated_current_apy: {
        status: "partial",
        value: { numerator: "477", denominator: "10000" },
        evidence: [],
        issues: [],
      },
    },
    name: "ETH",
    ticker: "ETH",
    selection: { kind: "exposure", id: "ETH" },
    direct_quantities: [{ asset_id: "ETH", units: "21422025648", scale: 8 }],
    alternatives: [
      {
        name: "Wrapped Ether · Arbitrum · EVM wallets",
        ticker: "WETH",
        provider: "evm",
        role: "asset",
        quantity: {
          asset_id: wrapped,
          units: "2000000000000000000",
          scale: 18,
        },
        value: {
          status: "known",
          value: {
            asset_id: "USD",
            amount: { numerator: "4020", denominator: "1" },
          },
          evidence: [],
        },
      },
    ],
  };
  data.startup.data.assets = [eth];
  await page.route("**/__portfolio_review", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(data),
    }),
  );
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Portfolio navigation" });
  await expect(nav.getByRole("button", { name: /^ETH / })).toHaveAccessibleName(
    /214\.22 ETH/,
  );
  await expect(nav.getByText(/representations?/)).toHaveCount(0);
  await expect(nav.locator(".nav-apy")).toContainText("~4.77%");
  await expect(nav.locator(".nav-apy")).not.toContainText("est.");
  await expect(nav.locator(".nav-apy svg path")).toHaveCount(6);
  await expect(nav.locator(".nav-native-quantity").first()).toHaveCSS(
    "white-space",
    "nowrap",
  );
  const symbol = nav.locator(".crypto-quantity-symbol").first();
  await expect(symbol).toBeVisible();
  const dimensions = await symbol.evaluate((el) => {
    const icon = el.getBoundingClientRect();
    const amount = el
      .parentElement!.querySelector("span")!
      .getBoundingClientRect();
    return {
      height: icon.height,
      amountHeight: amount.height,
      top: icon.top,
      amountTop: amount.top,
      bottom: icon.bottom,
      amountBottom: amount.bottom,
    };
  });
  expect(dimensions.height).toBe(dimensions.amountHeight);
  expect(dimensions.top).toBe(dimensions.amountTop);
  expect(dimensions.bottom).toBe(dimensions.amountBottom);
  await expect(symbol).toHaveAttribute("stroke", "currentColor");
  const warning = nav.getByRole("button", {
    name: "Warnings for ETH",
    exact: true,
  });
  const title = nav
    .locator(".nav-entry-title > span")
    .filter({ hasText: /^ETH$/ });
  const titleBounds = await title.boundingBox();
  const warningBounds = await warning.boundingBox();
  expect(warningBounds!.x).toBeGreaterThanOrEqual(
    titleBounds!.x + titleBounds!.width,
  );
  expect(
    Math.abs(
      warningBounds!.y +
        warningBounds!.height / 2 -
        (titleBounds!.y + titleBounds!.height / 2),
    ),
  ).toBeLessThan(3);
  expect(warningBounds!.height).toBe(16);
  const toggle = nav.getByRole("button", {
    name: "Alternative representations of ETH",
  });
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  const list = nav.getByRole("list", { name: "ETH alternative holdings" });
  await expect(list).toContainText("Wrapped Ether · Arbitrum");
  await expect(list).toContainText("2 WETH");
  await expect(
    page
      .getByRole("main")
      .getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  await toggle.click();
  await expect(list).toHaveCount(0);
  await warning.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page
      .getByRole("main")
      .getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Close details" }).click();
});

test("partial values wrap inside their column without crowding the title", async ({
  page,
}) => {
  await page.setViewportSize({ width: 687, height: 724 });
  const data = structuredClone(fixture);
  const entry = data.startup.data.assets.find((e: any) => e.name === "USD")!;
  entry.estimated_current_value = {
    status: "partial",
    value: {
      asset_id: "USD",
      amount: { numerator: "-1234567", denominator: "1" },
    },
    evidence: [],
    issues: [],
  };
  data.startup.data.assets = [entry];
  await page.route("**/__portfolio_review", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(data),
    }),
  );
  await page.goto("/");
  const row = page
    .getByRole("navigation")
    .getByRole("button", { name: /^USD / })
    .locator("..");
  await expect(row.getByText("Partial", { exact: true })).toBeVisible();
  await expect(row).not.toContainText("Partial subtotal");
  const title = await row.locator(".nav-entry-title").boundingBox();
  const value = await row.locator(".nav-entry-value").boundingBox();
  const bounds = await row.boundingBox();
  expect(title!.x + title!.width).toBeLessThanOrEqual(value!.x);
  expect(value!.x + value!.width).toBeLessThanOrEqual(
    bounds!.x + bounds!.width,
  );
  const badge = await row.getByText("Partial", { exact: true }).boundingBox();
  expect(badge!.x).toBeGreaterThanOrEqual(value!.x);
  expect(badge!.x + badge!.width).toBeLessThanOrEqual(value!.x + value!.width);
  const currency = await row
    .locator(".nav-native-quantity")
    .first()
    .boundingBox();
  expect(currency!.x + currency!.width).toBeLessThanOrEqual(
    bounds!.x + bounds!.width,
  );
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    687,
  );
});
