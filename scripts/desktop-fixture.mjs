import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
mkdirSync(".cache", { recursive: true });
const result = spawnSync(
  "cargo",
  [
    "test",
    "--locked",
    "-p",
    "money-tracker-desktop",
    "--features",
    "custom-protocol",
    "--test",
    "portfolio",
    "real_wire_fixture",
  ],
  {
    stdio: "inherit",
    env: {
      ...process.env,
      UPDATE_DESKTOP_FIXTURE: resolve(".cache/desktop-fixture.json"),
    },
  },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
