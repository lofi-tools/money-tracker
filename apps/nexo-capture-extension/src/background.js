const ORIGIN = "https://platform.nexo.com";
const HOST = "dev.money_tracker.nexo_capture";
const running = new Map();
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function configure() {
  if (!(await chrome.permissions.contains({ origins: [`${ORIGIN}/*`] })))
    return;
  await chrome.scripting
    .unregisterContentScripts({ ids: ["nexo-capture"] })
    .catch(() => {});
  await chrome.scripting.registerContentScripts([
    {
      id: "nexo-capture",
      matches: [`${ORIGIN}/*`],
      js: ["src/page.js"],
      runAt: "document_idle",
      persistAcrossSessions: true,
    },
  ]);
  await chrome.storage.session.setAccessLevel({
    accessLevel: "TRUSTED_CONTEXTS",
  });
}
chrome.runtime.onInstalled.addListener(configure);
chrome.runtime.onStartup.addListener(configure);
chrome.permissions.onRemoved.addListener(async () => {
  for (const run of running.values()) run.cancelled = true;
  if (!(await chrome.permissions.contains({ origins: [`${ORIGIN}/*`] })))
    await chrome.scripting
      .unregisterContentScripts({ ids: ["nexo-capture"] })
      .catch(() => {});
});
async function progress(tabId, message, active = true) {
  const run = running.get(tabId);
  if (run) run.message = message;
  await chrome.tabs
    .sendMessage(tabId, { type: "progress", message, running: active })
    .catch(() => {});
}
async function ready(tabId, run, predicate) {
  for (let i = 0; i < 60; i++) {
    if (run.cancelled)
      throw Error(run.failure ?? "Cancelled; unsent collection discarded");
    const tab = await chrome.tabs.get(tabId);
    if (new URL(tab.url).origin !== ORIGIN)
      throw Error("Left Nexo; export stopped");
    try {
      const view = await chrome.tabs.sendMessage(tabId, { type: "observe" });
      if (view?.error) throw Error(view.error);
      if (view?.authenticated && predicate(view)) return view;
    } catch {}
    await wait(250);
  }
  throw Error("Nexo view unavailable or signed out; export stopped");
}
async function navigate(tabId, run, path) {
  if (!["/", "/profile/settings"].includes(path))
    throw Error("Unsupported navigation");
  await chrome.tabs.update(tabId, { url: ORIGIN + path });
  return ready(tabId, run, (v) => v.path === path);
}
async function capture(tabId, initialUrl) {
  if (running.size) throw Error("An export is already running");
  const { connectionId } = await chrome.storage.local.get("connectionId");
  if (!/^[a-f0-9]{64}$/.test(connectionId ?? ""))
    throw Error("Set up the local connection from the extension toolbar first");
  const run = {
    cancelled: false,
    sending: false,
    interfered: false,
    message: "Starting…",
  };
  running.set(tabId, run);
  const captureId = crypto.randomUUID();
  let port;
  let nativeError = null;
  try {
    port = chrome.runtime.connectNative(HOST);
    port.onDisconnect.addListener(() => {
      nativeError =
        chrome.runtime.lastError?.message ?? "Native host disconnected";
      run.failure = "Native bridge unavailable; check Brave host setup";
      run.cancelled = true;
    });
    const started = new Date().toISOString();
    let view = await navigate(tabId, run, "/");
    const assets = [...new Set(view.assets)];
    if (!assets.length || assets.length > 100)
      throw Error("No reliable savings asset inventory; export stopped");
    const snapshot = {
      schema: "money-tracker.nexo-terms",
      version: 1,
      capture_id: captureId,
      capture_started_at: started,
      capture_finished_at: started,
      extractor_version: "brave-0.1.0",
      origin: ORIGIN,
      page_path: "/",
      locale: "en",
      browser_timezone: "unknown",
      account_hint: { source: "local_pairing" },
      coverage: {
        status: "partial",
        reported_active_count: null,
        views_captured: 0,
      },
      account_settings: { asset_rates: [] },
      terms: [],
      warnings: [],
    };
    let complete = true;
    let count = 0;
    for (let i = 0; i < assets.length; i++) {
      if (nativeError) throw Error("Native bridge unavailable; check setup");
      const asset = assets[i];
      await progress(tabId, `Reading ${asset} (${i + 1}/${assets.length})…`);
      view = await navigate(tabId, run, "/");
      await chrome.tabs.sendMessage(tabId, { type: "breakdown", asset });
      view = await ready(tabId, run, (v) => v.savings_breakdown);
      snapshot.coverage.views_captured++;
      if (view.asset_rate)
        snapshot.account_settings.asset_rates.push({
          asset,
          ...view.asset_rate,
        });
      if (!view.has_fixed_action) {
        snapshot.warnings.push(`${asset}: fixed terms view unavailable`);
        complete = false;
        continue;
      }
      const opened = await chrome.tabs.sendMessage(tabId, { type: "fixed" });
      if (opened?.error) throw Error(opened.error);
      view = await ready(
        tabId,
        run,
        (v) =>
          v.path.startsWith("/fixed-terms/") &&
          v.reported_active_count !== null &&
          (v.reported_active_count === 0 || v.terms.length > 0),
      );
      if (view.reported_active_count !== view.terms.length || view.paginated) {
        complete = false;
        snapshot.warnings.push(`${asset}: active list coverage incomplete`);
      }
      count += view.reported_active_count;
      snapshot.terms.push(
        ...view.terms.map((t, index) => ({
          ...t,
          observation_id: `${i}-${index}`,
        })),
      );
      snapshot.coverage.views_captured++;
    }
    await progress(tabId, "Reading payout and renewal settings…");
    view = await navigate(tabId, run, "/profile/settings");
    if (view.settings)
      snapshot.account_settings = {
        ...snapshot.account_settings,
        ...view.settings,
      };
    else {
      complete = false;
      snapshot.warnings.push("Account savings settings unavailable");
    }
    snapshot.coverage.views_captured++;
    // A final revisit/consistency pass is still pending.
    // First version conservatively reports partial: no durable term IDs/start precision
    // have been verified for this UK interface, so a full account snapshot is not claimed.
    complete = false;
    snapshot.warnings.push(
      "Capture is partial: stable term IDs, start dates and full list consistency are not established",
    );
    snapshot.coverage.status = complete ? "complete" : "partial";
    snapshot.coverage.reported_active_count = count;
    snapshot.capture_finished_at = new Date().toISOString();
    if (run.cancelled) throw Error("Cancelled");
    const payload = JSON.stringify(snapshot);
    if (new TextEncoder().encode(payload).length > 1_000_000)
      throw Error("Capture exceeds transport limit");
    await chrome.storage.session.set({
      pending: {
        connectionId,
        payload,
        tabId,
        expiresAt: Date.now() + 30 * 60_000,
      },
    });
    run.sending = true;
    await progress(
      tabId,
      "Sending to local tracker; waiting for DB acknowledgement…",
      false,
    );
    const result = await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () =>
          reject(
            Error(
              "Tracker acknowledgement timed out; Retry from extension toolbar",
            ),
          ),
        40_000,
      );
      port.onMessage.addListener((message) => {
        clearTimeout(timer);
        resolve(message);
      });
      port.onDisconnect.addListener(() => {
        clearTimeout(timer);
        reject(
          Error(
            "Native bridge disconnected; pending capture retained for Retry",
          ),
        );
      });
      port.postMessage({ version: 1, connection_id: connectionId, payload });
    });
    if (result.status === "committed") {
      await chrome.storage.session.remove("pending");
      const r = result.receipt;
      run.result = `Imported ${r.matched} matched, ${r.unresolved} unresolved, ${r.conflicts} conflicts. Coverage: partial.`;
      return run.result;
    }
    if (result.status === "pending") {
      run.result =
        "Received locally; DB ingestion is pending. Open the paired tracker, then Retry from extension setup.";
      return run.result;
    }
    throw Error(result.message ?? "Tracker rejected capture");
  } catch (error) {
    run.result = error.message;
    throw error;
  } finally {
    if (run.result)
      await chrome.storage.session.set({
        [`last:${tabId}`]: {
          message: run.result,
          expiresAt: Date.now() + 30 * 60_000,
        },
      });
    const wasSending = run.sending;
    running.delete(tabId);
    if (port) port.disconnect();
    if (!wasSending) {
      const { pending } = await chrome.storage.session.get("pending");
      if (pending && JSON.parse(pending.payload).capture_id === captureId)
        await chrome.storage.session.remove("pending");
    }
    const initial = new URL(initialUrl);
    if (initial.origin === ORIGIN && !run.interfered) {
      const tab = await chrome.tabs.get(tabId).catch(() => null);
      if (tab?.url?.startsWith(ORIGIN + "/"))
        await chrome.tabs.update(tabId, { url: initialUrl }).catch(() => {});
    }
  }
}
chrome.runtime.onMessage.addListener((m, sender, reply) => {
  if (sender.id !== chrome.runtime.id) return;
  if (m.type === "configure" && !sender.tab) {
    configure().then(() => reply({ ok: true }));
    return true;
  }
  if (m.type === "retry" && !sender.tab) {
    (async () => {
      const { pending } = await chrome.storage.session.get("pending");
      if (!pending || pending.expiresAt < Date.now()) {
        await chrome.storage.session.remove("pending");
        throw Error("No pending capture");
      }
      return chrome.runtime.sendNativeMessage(HOST, {
        version: 1,
        connection_id: pending.connectionId,
        payload: pending.payload,
      });
    })().then(
      async (r) => {
        if (r.status === "committed")
          await chrome.storage.session.remove("pending");
        reply(r);
      },
      () =>
        reply({
          status: "rejected",
          message: "Retry unavailable; check tracker and pairing",
        }),
    );
    return true;
  }
  if (
    !sender.tab ||
    sender.frameId !== 0 ||
    new URL(sender.url).origin !== ORIGIN
  )
    return;
  const tabId = sender.tab.id;
  if (m.type === "state") {
    const run = running.get(tabId);
    if (run) {
      reply({ running: !run.sending, message: run.message });
      return;
    }
    chrome.storage.session.get(`last:${tabId}`).then((data) => {
      const last = data[`last:${tabId}`];
      reply({
        running: false,
        message: last?.expiresAt > Date.now() ? last.message : "",
      });
    });
    return true;
  }
  if (m.type === "interference") {
    const run = running.get(tabId);
    if (run && !run.sending) {
      run.cancelled = true;
      run.interfered = true;
    }
    reply({ ok: true });
    return;
  }
  if (m.type === "cancel") {
    const run = running.get(tabId);
    if (run && !run.sending) run.cancelled = true;
    reply({ ok: true });
    return;
  }
  if (m.type === "start") {
    capture(tabId, sender.tab.url).then(
      (message) => {
        reply({ message });
        progress(tabId, message, false);
      },
      (error) => {
        const message = error.message;
        reply({ message });
        progress(tabId, message, false);
      },
    );
    return true;
  }
});
