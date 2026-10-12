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
  await expect(nav.getByRole("button", { name: "Change history" })).toHaveCount(
    0,
  );
  for (let pass = 0; pass < 2; pass++) {
    await nav.getByRole("button", { name: /US USD/ }).click();
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
    await nav.getByRole("button", { name: "OV Overview" }).click();
    await expect(
      page
        .getByRole("main")
        .getByRole("heading", { name: "Overview", exact: true }),
    ).toBeInViewport();
    if (!pass) await page.reload();
  }
  expect(errors).toEqual([]);
});
