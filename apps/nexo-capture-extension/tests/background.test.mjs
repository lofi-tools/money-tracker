import { test } from "node:test";
import assert from "node:assert/strict";
const event = () => ({
  listeners: [],
  addListener(fn) {
    this.listeners.push(fn);
  },
});
function chromeFixture() {
  let url = "https://platform.nexo.com/";
  let modal = false;
  let posted;
  const calls = [];
  const session = {};
  const message = event();
  const disconnect = event();
  const port = {
    onMessage: message,
    onDisconnect: disconnect,
    disconnect() {},
    postMessage(value) {
      posted = value;
      queueMicrotask(() =>
        message.listeners.forEach((fn) =>
          fn({
            status: "committed",
            receipt: { matched: 0, unresolved: 1, conflicts: 0 },
          }),
        ),
      );
    },
  };
  const term = {
    observation_id: "one",
    provider_term_id: null,
    observed_at: new Date().toISOString(),
    status: "active",
    principal: {
      asset: "ETH",
      amount: "40.00000000",
      precision: "displayed",
      exactness: "unknown",
    },
    rate: null,
    start: null,
    maturity: null,
    payout: null,
    auto_renew: null,
    renew_with_interest: null,
    duration_months: null,
    accrued_rewards: [],
    evidence: {},
    warnings: [],
  };
  const chrome = {
    runtime: {
      id: "test-extension",
      onInstalled: event(),
      onStartup: event(),
      onMessage: event(),
      connectNative: () => port,
    },
    permissions: { contains: async () => true, onRemoved: event() },
    scripting: {
      unregisterContentScripts: async () => {},
      registerContentScripts: async () => {},
    },
    storage: {
      local: { get: async () => ({ connectionId: "a".repeat(64) }) },
      session: {
        setAccessLevel: async () => {},
        set: async (value) => Object.assign(session, value),
        get: async () => session,
        remove: async (key) => delete session[key],
      },
    },
    tabs: {
      get: async () => ({ url }),
      update: async (_id, value) => {
        calls.push(value.url);
        url = value.url;
        modal = false;
      },
      sendMessage: async (_id, m) => {
        if (m.type === "progress") return {};
        if (m.type === "breakdown") {
          modal = true;
          return { ok: true };
        }
        if (m.type === "fixed") {
          url = "https://platform.nexo.com/fixed-terms/eth";
          return { ok: true };
        }
        const path = new URL(url).pathname;
        return {
          authenticated: true,
          path,
          assets: ["ETH"],
          savings_breakdown: modal,
          has_fixed_action: modal,
          reported_active_count: 1,
          paginated: false,
          terms: [{ ...term, observed_at: new Date().toISOString() }],
          settings: path.includes("settings")
            ? { renew_with_interest: true }
            : null,
        };
      },
    },
  };
  return {
    chrome,
    calls,
    session,
    get posted() {
      return posted;
    },
  };
}
test("one trusted extension request navigates account views, sends JSON once and waits for committed ack", async () => {
  const f = chromeFixture();
  globalThis.chrome = f.chrome;
  await import("../src/background.js?flow");
  const result = await new Promise((resolve) =>
    f.chrome.runtime.onMessage.listeners[0](
      { type: "start" },
      {
        id: "test-extension",
        frameId: 0,
        url: "https://platform.nexo.com/",
        tab: { id: 7, url: "https://platform.nexo.com/" },
      },
      resolve,
    ),
  );
  assert.match(result.message, /0 matched, 1 unresolved/);
  assert.ok(f.calls.includes("https://platform.nexo.com/profile/settings"));
  assert.equal(f.posted.connection_id, "a".repeat(64));
  const capture = JSON.parse(f.posted.payload);
  assert.equal(capture.coverage.status, "partial");
  assert.equal(capture.terms.length, 1);
  assert.equal(f.session.pending, undefined);
  assert.equal(f.calls.at(-1), "https://platform.nexo.com/");
});
test("unrelated website messages cannot start an export", async () => {
  const f = chromeFixture();
  globalThis.chrome = f.chrome;
  await import("../src/background.js?origin");
  let replied = false;
  f.chrome.runtime.onMessage.listeners[0](
    { type: "start" },
    {
      id: "test-extension",
      frameId: 0,
      url: "https://evil.example/",
      tab: { id: 7, url: "https://evil.example/" },
    },
    () => {
      replied = true;
    },
  );
  assert.equal(replied, false);
  assert.equal(f.posted, undefined);
});
