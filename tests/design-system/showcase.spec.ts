import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import fs from "node:fs/promises";
test("public consumer loads styled variants, grouping keys and independent table/details controls", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/portfolio");
  await expect(
    page.getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Synthetic data", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Primary action", exact: true }),
  ).toHaveCSS("border-radius", "4px");
  const tab = page.getByRole("tab", { name: "By asset", exact: true });
  await tab.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "By provider", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "By asset", exact: true }).click();
  await page.getByRole("button", { name: /Ethereum/ }).click();
  const term = page.getByRole("button", {
    name: "ETH fixed term · October",
    exact: true,
  });
  await expect(term).toHaveCount(0);
  await page
    .getByRole("button", { name: "Expand Fixed-term earn", exact: true })
    .click();
  await expect(term).toBeVisible();
  await page
    .getByRole("button", { name: "Collapse Fixed-term earn", exact: true })
    .click();
  await expect(term).toHaveCount(0);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Expand Fixed-term earn", exact: true })
    .click();
  await term.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByRole("table", {
      name: "Ethereum allocation · synthetic fixture",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Collapse Fixed-term earn", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Close details", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(term).toBeFocused();
  const summary = page.getByRole("region", {
    name: "Synthetic portfolio summary",
  });
  const before = await summary.innerText();
  await page.getByRole("searchbox").fill("not-an-asset");
  await expect(
    page.getByText("No matching assets or providers."),
  ).toBeVisible();
  expect(await summary.innerText()).toBe(before);
  await expect(
    page.getByRole("textbox", { name: "Read-only example name", exact: true }),
  ).toHaveAccessibleDescription("Synthetic names are not saved.");
  expect(errors).toEqual([]);
  expect(
    requests.every(
      (url) =>
        url.startsWith("http://127.0.0.1:4173/") || url.startsWith("data:"),
    ),
  ).toBe(true);
});
test("inline validation, commit, cancel, pin and Undo are keyboard accessible", async ({
  page,
}) => {
  await page.goto("/portfolio");
  await page
    .getByRole("button", { name: "Edit Demo subscription APY", exact: true })
    .click();
  const input = page.getByRole("textbox", {
    name: "Demo subscription APY",
    exact: true,
  });
  await expect(input).toBeFocused();
  await input.fill("-2");
  await input.press("Enter");
  await expect(input).toBeVisible();
  await expect(
    page
      .getByRole("alert")
      .filter({ hasText: "Enter a nonnegative decimal percentage" }),
  ).toBeVisible();
  await input.fill("6.5");
  await input.press("Enter");
  await expect(input).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Undo demo edit", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Pin Demo subscription APY", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Pin Demo subscription APY",
      exact: true,
    }),
  ).toHaveAttribute("aria-pressed", "true");
  await page
    .getByRole("button", { name: "Undo demo edit", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Pin Demo subscription APY",
      exact: true,
    }),
  ).toHaveAttribute("aria-pressed", "false");
  await page
    .getByRole("button", { name: "Edit Demo subscription APY", exact: true })
    .click();
  await input.fill("-9");
  await page
    .getByRole("button", { name: "Cancel Demo subscription APY", exact: true })
    .click();
  await expect(input).toHaveCount(0);
  await expect(
    page
      .getByRole("alert")
      .filter({ hasText: "Enter a nonnegative decimal percentage" }),
  ).toHaveCount(0);
  await expect(page.getByText("6.5", { exact: true })).toBeVisible();
});
test("hidden/scaled modes cover visible text, attributes and details without masking percentages", async ({
  page,
}) => {
  await page.goto("/portfolio");
  await page.getByRole("button", { name: /Ethereum/ }).click();
  await page
    .getByRole("button", { name: "Expand Fixed-term earn", exact: true })
    .click();
  await page
    .getByRole("button", { name: "ETH fixed term · October", exact: true })
    .click();
  await page.getByLabel("Privacy", { exact: true }).selectOption("hidden");
  await expect(page.getByRole("dialog")).toContainText("••••");
  await expect(page.getByRole("dialog")).toContainText("5%");
  const visible = await page.locator("body").innerText();
  expect(visible).not.toMatch(/[$€¥]\s*\d|50 ETH|100 ETH/);
  const attributes = await page
    .locator("[aria-label], [title], [value]")
    .evaluateAll((els) =>
      els
        .flatMap((el) =>
          ["aria-label", "title", "value"].map((k) => el.getAttribute(k) ?? ""),
        )
        .join(" "),
    );
  expect(attributes).not.toMatch(
    /\$1,000|\$500|50 ETH|100 ETH|50000000000000000000/,
  );
  await page.getByLabel("Privacy", { exact: true }).selectOption("scaled");
  await expect(
    page.getByText("Amounts are scaled; rates are unchanged."),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).toContainText("$50.00");
  await expect(page.getByRole("dialog")).toContainText("5 ETH");
  await expect(page.getByRole("dialog")).not.toContainText("$500");
  await page.getByLabel("Display scale", { exact: true }).selectOption("0.25");
  await expect(page.getByRole("dialog")).toContainText("$125");
  await expect(page.getByRole("dialog")).toContainText("12.5 ETH");
});
for (const theme of ["light", "dark"] as const) {
  test(`${theme} theme, warning panel, contrast and reduced motion`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.goto("/portfolio");
    await expect(page.locator("[data-theme]")).toHaveAttribute(
      "data-theme",
      theme,
    );
    await page.emulateMedia({
      colorScheme: theme === "dark" ? "light" : "dark",
      reducedMotion: "reduce",
    });
    await expect(page.locator("[data-theme]")).toHaveAttribute(
      "data-theme",
      theme === "dark" ? "light" : "dark",
    );
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await expect(page.locator("[data-theme]")).toHaveAttribute(
      "data-theme",
      theme,
    );
    await page.getByRole("button", { name: /Ethereum/ }).click();
    await page
      .getByRole("button", { name: "Expand Fixed-term earn", exact: true })
      .click();
    await fs.mkdir(".cache/design-system-review", { recursive: true });
    await page.screenshot({
      path: `.cache/design-system-review/${theme}.png`,
      fullPage: true,
    });
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(results.violations).toEqual([]);
    await page
      .getByRole("button", { name: "Explain partial earnings", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText("What is withheld");
    const panel = page.getByRole("dialog");
    const duration = await panel.evaluate(
      (el) => getComputedStyle(el).animationDuration,
    );
    expect(parseFloat(duration)).toBeLessThanOrEqual(0.001);
    const panelResults = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(panelResults.violations).toEqual([]);
    await page
      .getByRole("button", { name: "Close details", exact: true })
      .click();
    await page
      .getByLabel("Theme", { exact: true })
      .selectOption(theme === "dark" ? "light" : "dark");
    await expect(page.locator("[data-theme]")).toHaveAttribute(
      "data-theme",
      theme === "dark" ? "light" : "dark",
    );
  });
}
test("narrow windows keep controls usable and table overflow contained", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/portfolio");
  await page.getByRole("button", { name: /Ethereum/ }).click();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page
    .getByRole("button", { name: "Expand Fixed-term earn", exact: true })
    .click();
  await page
    .getByRole("button", { name: "ETH fixed term · October", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Close details", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await fs.mkdir(".cache/design-system-review", { recursive: true });
  await page.screenshot({
    path: ".cache/design-system-review/narrow.png",
    fullPage: true,
  });
});
