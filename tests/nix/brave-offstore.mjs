// Installed Brave 1.96.61 regression: a valid local CRX is rejected as unverified.
// This uses a disposable profile, never the user's browser data or site grants.
import { chromium } from "@playwright/test";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";
const executablePath = process.env.NEXO_BRAVE_EXECUTABLE;
assert(executablePath, "Set NEXO_BRAVE_EXECUTABLE to installed Brave");
const release = resolve("tests/nix/fixtures/offstore");
const { extension_id: id, version } = JSON.parse(
  readFileSync(join(release, "release.json")),
);
const profile = mkdtempSync(join(tmpdir(), "nexo-brave-offstore-"));
let context;
try {
  mkdirSync(join(profile, "External Extensions"));
  writeFileSync(
    join(profile, "External Extensions", `${id}.json`),
    JSON.stringify({
      external_crx: join(release, "nexo-capture.crx"),
      external_version: version,
    }),
  );
  context = await chromium.launchPersistentContext(profile, {
    executablePath,
    headless: true,
    // Chromium skips its external provider when disable-default-apps is set.
    ignoreDefaultArgs: ["--disable-extensions", "--disable-default-apps"],
    args: [
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-brave-update",
    ],
  });
  let entry;
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    try {
      entry = JSON.parse(
        readFileSync(join(profile, "Default", "Secure Preferences")),
      ).extensions?.settings?.[id];
      if (entry?.disable_reasons?.includes(256)) break;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.equal(
    entry?.manifest.version,
    version,
    "Brave must have read the CRX",
  );
  assert.deepEqual(
    entry.disable_reasons,
    [256],
    "Expected DISABLE_NOT_VERIFIED; reassess deployment if Brave changes",
  );
  assert(
    !context
      .serviceWorkers()
      .some((w) => w.url().startsWith(`chrome-extension://${id}/`)),
  );
  console.log(
    "Confirmed installed Brave rejects local CRX with DISABLE_NOT_VERIFIED (256)",
  );
} finally {
  if (context) await context.close();
  rmSync(profile, { recursive: true, force: true });
}
