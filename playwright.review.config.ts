import { defineConfig } from "@playwright/test";
export default defineConfig({
  outputDir: "./test-results/review",
  testDir: "./tests/review",
  workers: 1,
  timeout: 30000,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:4176",
    viewport: { width: 968, height: 724 },
    trace: "retain-on-failure",
  },
  webServer: {
    command:
      "pnpm --config.verifyDepsBeforeRun=false --filter @money-tracker/desktop dev:review --port 4176 --strictPort",
    url: "http://127.0.0.1:4176",
    reuseExistingServer: false,
  },
});
