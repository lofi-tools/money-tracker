// Test the built wrapper itself, never add --load-extension in this harness.
import { chromium } from '@playwright/test';
import { mkdtempSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
const executablePath = process.env.NEXO_BRAVE_EXECUTABLE;
assert(executablePath, 'Set NEXO_BRAVE_EXECUTABLE to the wrapped App executable');
const manifest = JSON.parse(readFileSync('apps/nexo-capture-extension/manifest.json'));
const id = [...createHash('sha256').update(Buffer.from(manifest.key, 'base64')).digest('hex').slice(0,32)].map(c => String.fromCharCode(97 + parseInt(c,16))).join('');
const profile = mkdtempSync(join(tmpdir(), 'nexo-brave-wrapper-'));
let context;
try {
  context = await chromium.launchPersistentContext(profile, {
    executablePath, headless: true, ignoreDefaultArgs: ['--disable-extensions'],
    args: ['--no-first-run', '--no-default-browser-check', '--disable-brave-update'],
  });
  const worker = context.serviceWorkers().find(w => w.url().startsWith(`chrome-extension://${id}/`)) ??
    await context.waitForEvent('serviceworker', { predicate: w => w.url().startsWith(`chrome-extension://${id}/`), timeout: 15000 });
  assert.equal(await worker.evaluate(() => chrome.runtime.id), id);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${id}/options.html`);
  assert.match(await page.locator('h1').innerText(), /Nexo capture/);
  await page.goto('brave://version');
  const text = await page.locator('body').innerText();
  assert.match(text, /--load-extension=\/nix\/store\//);
  assert(text.includes(profile), 'The wrapper must preserve the test profile argument');
  console.log(`Wrapped Brave loaded ${id} automatically and preserved caller arguments`);
} finally {
  if (context) await context.close();
  rmSync(profile, { recursive: true, force: true });
}
