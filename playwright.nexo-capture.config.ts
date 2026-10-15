import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests/nexo-capture",
  workers: 1,
  timeout: 15000,
  reporter: "list",
  use: { viewport: { width: 1000, height: 800 } },
});
