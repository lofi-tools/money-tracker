import { test, expect } from "@playwright/test";
import { resolve } from "node:path";
const nav = '<nav><a href="/transactions">Transactions</a></nav>';
const card = (amount: string, due: string, basis = "") =>
  `<article><span>Initial balance</span> <b>${amount} ETH</b><p>Interest rate</p><p>Up to $ 4,000,000 6.25% ${basis}</p><p>Interest Terms 1 month</p><p>Interest earned in NEXO (Simple interest) 3.456 NEXO ACTIVE</p><p>Due date ${due}</p><label><input type="checkbox" checked>Automatic renewal</label><span>With interest</span><button onclick="window.mutations++">Exchange term</button></article>`;
async function fixture(page: any, html: string, path = "/fixed-terms/eth") {
  await page.route("https://platform.nexo.com/**", (route: any) =>
    route.fulfill({ contentType: "text/html", body: html }),
  );
  await page.addInitScript(() => {
    (window as any).mutations = 0;
    (window as any).messages = [];
    Object.defineProperty(window, "chrome", {
      value: {
        runtime: {
          id: "test-extension",
          sendMessage: async (m: any) => {
            if (m.type === "state") return { running: false };
            (window as any).messages.push(m);
            return { message: "Synthetic test only" };
          },
          onMessage: {
            addListener: (fn: any) => {
              (window as any).reader = fn;
            },
          },
        },
      },
    });
  });
  await page.goto(`https://platform.nexo.com${path}`);
  await page.addScriptTag({
    path: resolve("apps/nexo-capture-extension/src/page.js"),
  });
}
async function observe(page: any, type = "observe", extra = {}) {
  return page.evaluate(
    ({ type, extra }: any) =>
      new Promise((resolve) =>
        (window as any).reader(
          { type, ...extra },
          { id: "test-extension" },
          resolve,
        ),
      ),
    { type, extra },
  );
}
test("UK terms preserve precision, unknown start/basis and separate identical pots without setting changes", async ({
  page,
}) => {
  await fixture(
    page,
    `${nav}<h1>Fixed-term Savings ETH</h1><p>Active terms 2</p>${card("40.00000000", "2 Nov 2026")}${card("40.00000000", "3 Nov 2026", "APY")}<form>Create a term<label><input type="checkbox">Automatic renew</label><button>Create term</button></form>`,
  );
  const state: any = await observe(page);
  expect(state.terms).toHaveLength(2);
  expect(state.reported_active_count).toBe(2);
  expect(state.terms[0].principal).toEqual({
    asset: "ETH",
    amount: "40.00000000",
    precision: "displayed",
    exactness: "unknown",
  });
  expect(state.terms[0].start).toBeNull();
  expect(state.terms[0].rate.basis).toBe("unknown");
  expect(state.terms[1].rate.basis).toBe("apy");
  expect(state.terms[0].auto_renew).toBe(true);
  expect(state.terms[0].maturity).toEqual({
    value: "2026-11-02",
    precision: "date",
    timezone: null,
  });
  expect(state.terms[0].provider_term_id).toBeNull();
  expect(await page.evaluate(() => (window as any).mutations)).toBe(0);
  // Closed-shadow control only accepts trusted user activation, not page-dispatched events.
  await page.locator("#money-tracker-nexo-export").dispatchEvent("click");
  expect(await page.evaluate(() => (window as any).messages.length)).toBe(0);
  await page.locator("#money-tracker-nexo-export").click();
  expect(await page.evaluate(() => (window as any).messages[0]?.type)).toBe(
    "start",
  );
  await page.screenshot({
    path: "/private/tmp/nexo-extension-capture-review.png",
  });
});
test("settings are read-only and no absent checkbox becomes false", async ({
  page,
}) => {
  await fixture(
    page,
    `${nav}<h1>Settings</h1><section><h2>Daily interest</h2><h3>Payout type</h3><label><input type="radio" name="payout">Interest payout in the same currency</label><label><input type="radio" name="payout" checked>Interest payout in NEXO Tokens</label><h3>Fixed-term Savings renewal</h3><label><input type="checkbox" checked>Renewal with interest</label></section>`,
    "/profile/settings",
  );
  const state: any = await observe(page);
  expect(state.settings.payout_asset).toBe("NEXO");
  expect(state.settings.renew_with_interest).toBe(true);
  expect(await page.locator("input[type=checkbox]").isChecked()).toBe(true);
  expect(await page.evaluate(() => (window as any).messages.length)).toBe(0);
});
test("signed-out and unrecognized term layouts never produce a valid empty list", async ({
  page,
}) => {
  await fixture(page, '<h1>Log in</h1><input type="password">');
  expect(await page.locator("#money-tracker-nexo-export").isVisible()).toBe(
    false,
  );
  expect(await observe(page)).toEqual({ authenticated: false });
});
