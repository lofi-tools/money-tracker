import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test("catalog covers exported components with live selection, details and privacy", async ({
  page,
}) => {
  const errors: string[] = [];
  const requests: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("request", (r) => requests.push(r.url()));
  await page.goto("/");
  await expect(
    page.getByRole("heading", {
      name: "Design system catalog & portfolio showcase",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Allocation table & detail panel" }),
  ).toBeVisible();
  await page
    .getByRole("group", { name: "Catalog chart range", exact: true })
    .getByRole("button", { name: "Year", exact: true })
    .click();
  await expect(
    page
      .getByRole("group", { name: "Catalog chart range", exact: true })
      .getByRole("button", { name: "Year", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(
    page.getByRole("img", { name: "Synthetic year trend", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("group", { name: "Example card pages" })
    .getByRole("button", { name: "Page 3" })
    .click();
  await expect(
    page
      .getByRole("group", { name: "Example card pages" })
      .getByRole("button", { name: "Page 3" }),
  ).toHaveAttribute("aria-pressed", "true");
  const trigger = page.getByRole("button", {
    name: "Open sample details",
    exact: true,
  });
  await trigger.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page
    .getByRole("button", { name: "Close details", exact: true })
    .click();
  await expect(trigger).toBeFocused();
  await page.getByLabel("Catalog privacy").selectOption("hidden");
  expect(await page.locator("body").innerText()).not.toContain(
    "9007199254740993",
  );
  await expect(page.getByLabel("Amount hidden").first()).toBeVisible();
  await expect(page.getByText("5%", { exact: true }).first()).toBeVisible();
  expect(errors).toEqual([]);
  expect(
    requests.every(
      (url) =>
        url.startsWith("http://127.0.0.1:4173/") || url.startsWith("data:"),
    ),
  ).toBe(true);
});
for (const theme of ["dark", "light"])
  test(`catalog ${theme} theme, contrast and full-page visual review`, async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByLabel("Catalog theme").selectOption(theme);
    await expect(page.locator("[data-theme]")).toHaveAttribute(
      "data-theme",
      theme,
    );
    const result = await new AxeBuilder({ page }).analyze();
    expect(result.violations).toEqual([]);
    await page.screenshot({
      path: `.cache/design-system-review/catalog-${theme}.png`,
      fullPage: true,
    });
  });
test("catalog narrow layout has no page overflow and respects reduced motion", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page
    .getByRole("button", { name: "Open details panel", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Close details" }),
  ).toBeFocused();
  await page.getByRole("button", { name: "Close details" }).click();
  await page.screenshot({
    path: ".cache/design-system-review/catalog-narrow.png",
    fullPage: true,
  });
});
