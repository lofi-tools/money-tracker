/* Packaged isolated-world reader. No page evaluation, fetch, credentials or setting writes. */
(() => {
  if (globalThis.__moneyTrackerNexo) return;
  globalThis.__moneyTrackerNexo = true;
  const text = (el) =>
    (el?.innerText ?? el?.textContent ?? "").replace(/\s+/g, " ").trim();
  const visible = (el) =>
    el instanceof Element &&
    !!el.getClientRects().length &&
    getComputedStyle(el).visibility !== "hidden";
  const bodyText = () => text(document.body);
  function authenticated() {
    return (
      location.origin === "https://platform.nexo.com" &&
      !!document.querySelector(
        'a[href="/transactions"],a[href="https://platform.nexo.com/transactions"]',
      ) &&
      (/Portfolio Balance|Fixed-term Savings|Daily interest/.test(bodyText()) ||
        [...document.querySelectorAll("button")].some(
          (el) => visible(el) && text(el) === "Buy crypto",
        )) &&
      !document.querySelector('input[type="password"]')
    );
  }
  function date(raw) {
    const m =
      /\b(\d{1,2}) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) (\d{4})\b/.exec(
        raw,
      );
    if (!m) return null;
    const month =
      [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec",
      ].indexOf(m[2]) + 1;
    const value = `${m[3]}-${String(month).padStart(2, "0")}-${m[1].padStart(2, "0")}`;
    if (new Date(`${value}T00:00:00Z`).toISOString().slice(0, 10) !== value)
      return null;
    return { value, precision: "date", timezone: null };
  }
  function amount(s) {
    return /^(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?$/.test(s)
      ? s.replaceAll(",", "")
      : null;
  }
  function checkbox(root, label) {
    const controls = [
      ...root.querySelectorAll(
        'input[type="checkbox"],input[type="radio"],[role="checkbox"],[role="switch"],[role="radio"]',
      ),
    ];
    const matching = controls.filter((el) => {
      const labelledBy = (el.getAttribute("aria-labelledby") ?? "")
        .split(/\s+/)
        .filter(Boolean)
        .map((id) => text(document.getElementById(id)))
        .join(" ");
      const labelText =
        el.getAttribute("aria-label") || labelledBy || text(el.labels?.[0]);
      return label.test(labelText) || label.test(text(el.parentElement));
    });
    const found =
      matching.length === 1
        ? matching[0]
        : controls.length === 1
          ? controls[0]
          : null;
    if (!found) return null;
    if (found instanceof HTMLInputElement) return found.checked;
    return found.getAttribute("aria-checked") === "true"
      ? true
      : found.getAttribute("aria-checked") === "false"
        ? false
        : null;
  }
  function cards() {
    const found = new Set();
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_TEXT,
    );
    while (walker.nextNode()) {
      if (
        !/^\s*Initial balance(?:\s|$)/.test(
          walker.currentNode.textContent ?? "",
        )
      )
        continue;
      let el = walker.currentNode.parentElement;
      for (let i = 0; el && i < 10; i++, el = el.parentElement) {
        const value = text(el);
        if ((value.match(/Initial balance/g) ?? []).length > 1) break;
        if (
          visible(el) &&
          value.includes("Due date") &&
          value.includes("Automatic renewal")
        ) {
          found.add(el);
          break;
        }
      }
    }
    return [...found];
  }
  function terms() {
    return cards().map((el, index) => {
      const value = text(el);
      if (!/\bACTIVE\b/.test(value))
        throw Error("Active term status not established");
      if (value.length > 8000) throw Error("Unexpected term layout");
      const p = /Initial balance\s+([\d.,]+)\s+([A-Z][A-Za-z0-9]{1,12})\b/.exec(
        value,
      );
      const portion =
        value.split("Interest rate")[1]?.split("Interest Terms")[0] ?? "";
      const rates = [...portion.matchAll(/(\d+(?:\.\d+)?)\s*%/g)];
      const percent = rates.length === 1 ? rates[0][1] : null;
      const payout = /Interest earned in\s+([A-Z][A-Za-z0-9]{1,12})\b/.exec(
        value,
      );
      const due = date(value.split("Due date")[1] ?? "");
      const start = date(
        value.split(/Start date|Opened on/)[1]?.split("Due date")[0] ?? "",
      );
      const rawRate = percent
        ? `${percent}% ${/\bAPY\b/i.test(portion) ? "APY" : /\bAPR\b/i.test(portion) ? "APR" : "(basis not labelled)"}`
        : null;
      return {
        observation_id: `view-${index}`,
        provider_term_id: null,
        observed_at: new Date().toISOString(),
        status: "active",
        principal:
          p && amount(p[1])
            ? {
                asset: p[2].toUpperCase(),
                amount: amount(p[1]),
                precision: "displayed",
                exactness: "unknown",
              }
            : null,
        rate: percent
          ? {
              percent,
              basis: /\bAPY\b/i.test(portion)
                ? "apy"
                : /\bAPR\b/i.test(portion)
                  ? "apr"
                  : "unknown",
              raw_label: rawRate,
              scope: "term",
            }
          : null,
        payout: payout
          ? {
              asset: payout[1].toUpperCase(),
              scope: "term",
              historical_continuity: "unknown",
            }
          : null,
        start,
        maturity: due,
        duration_months:
          Number(/Interest Terms\s+(\d+)\s+months?/i.exec(value)?.[1]) || null,
        auto_renew: checkbox(el, /Automatic renewal/i),
        renew_with_interest: /Without interest/i.test(value)
          ? false
          : /With interest/i.test(value)
            ? true
            : null,
        accrued_rewards: (() => {
          const reward =
            /Interest earned in\s+([A-Z][A-Za-z0-9]{1,12})\s*\((?:Simple|Compound) interest\)\s*([\d.,]+)\s+([A-Z][A-Za-z0-9]{1,12})/.exec(
              value,
            );
          return reward &&
            amount(reward[2]) &&
            reward[1].toUpperCase() === reward[3].toUpperCase()
            ? [
                {
                  asset: reward[3].toUpperCase(),
                  amount: amount(reward[2]),
                  precision: "displayed",
                },
              ]
            : [];
        })(),
        evidence: {
          view: "active_terms",
          fields: {
            principal: p?.[0] ?? null,
            rate: portion.slice(0, 512),
            maturity: due?.value ?? null,
            calculation:
              /\((Simple interest|Compound interest)\)/i.exec(value)?.[1] ??
              null,
          },
        },
        warnings: [
          "Displayed principal precision not independently established",
          ...(!start ? ["Start date not displayed"] : []),
          ...(!percent ? ["Rate bands or missing rate: unresolved"] : []),
        ],
      };
    });
  }
  function dashboardAssets() {
    return [...document.querySelectorAll("tr")]
      .filter(visible)
      .filter((row) => /Earning up to/.test(text(row)))
      .map((row) => {
        const ticker = row.querySelector("img[alt]")?.getAttribute("alt");
        return ticker && /^[A-Z][A-Za-z0-9]{1,12}$/.test(ticker)
          ? ticker.toUpperCase()
          : null;
      })
      .filter(Boolean);
  }
  function openBreakdown(asset) {
    const rows = [...document.querySelectorAll("tr")].filter(
      (row) =>
        row.querySelector("img[alt]")?.getAttribute("alt")?.toUpperCase() ===
        asset,
    );
    if (rows.length !== 1) throw Error("Asset row not unique");
    const row = rows[0];
    const target = [...row.querySelectorAll("span,div,button")]
      .filter(visible)
      .filter((el) => /^Earning up to [\d.]+% per year$/.test(text(el)))
      .sort(
        (a, b) =>
          a.querySelectorAll("*").length - b.querySelectorAll("*").length,
      )[0];
    if (!target) throw Error("Savings breakdown action unavailable");
    target.click();
  }
  function fixedAction() {
    const buttons = [...document.querySelectorAll("button")]
      .filter(visible)
      .filter((el) => text(el) === "Fixed-term Savings");
    if (buttons.length !== 1)
      throw Error("Fixed-term navigation action not unique");
    buttons[0].click();
  }
  function settings() {
    const section = [...document.querySelectorAll("main,section,div")]
      .filter(visible)
      .filter(
        (el) =>
          text(el).includes("Payout type") &&
          text(el).includes("Fixed-term Savings renewal"),
      )
      .sort((a, b) => text(a).length - text(b).length)[0];
    if (!section) throw Error("Savings settings unavailable");
    const labelled = (label) => {
      const matches = [...section.querySelectorAll("label")].filter((el) =>
        label.test(text(el)),
      );
      return matches.length === 1
        ? checkbox(matches[0], label)
        : checkbox(section, label);
    };
    const nexo = labelled(/Interest payout in NEXO Tokens/);
    const kind = labelled(/Interest payout in the same currency/);
    return {
      observed_at: new Date().toISOString(),
      payout_asset: nexo === true && kind !== true ? "NEXO" : null,
      payout_in_kind: kind === true && nexo !== true ? true : null,
      renew_with_interest: labelled(/Renewal with interest/),
    };
  }
  function breakdownRates() {
    const roots = [...document.querySelectorAll("section,div")]
      .filter(visible)
      .filter(
        (el) =>
          text(el).includes("Interest breakdown") &&
          text(el).includes("Savings Wallet balance") &&
          text(el).includes("Flexible Savings"),
      )
      .sort((a, b) => text(a).length - text(b).length);
    const value = text(roots[0]);
    if (!value || value.length > 12000) return null;
    const flex =
      /Flexible Savings\s*\*?\s*(?:Flexible Savings\s*)?(\d+(?:\.\d+)?)%\s+([\d.,]+)\s+([A-Z][A-Za-z0-9]{1,12})/.exec(
        value,
      );
    const fixed = [
      ...value.matchAll(
        /Fixed for\s+(\d+)\s+months?\s+(\d+(?:\.\d+)?)%\s+([\d.,]+)\s+([A-Z][A-Za-z0-9]{1,12})/g,
      ),
    ].map((m) => ({
      duration_months: Number(m[1]),
      percent: m[2],
      balance: amount(m[3]),
      asset: m[4].toUpperCase(),
      basis: "unknown",
    }));
    return {
      observed_at: new Date().toISOString(),
      flexible: flex
        ? {
            percent: flex[1],
            balance: amount(flex[2]),
            asset: flex[3].toUpperCase(),
            basis: "unknown",
          }
        : null,
      fixed,
    };
  }
  function observe() {
    if (!authenticated()) return { authenticated: false };
    const value = bodyText();
    const list = location.pathname.startsWith("/fixed-terms/");
    const count = /Active terms\s+(\d+)/.exec(value);
    return {
      authenticated: true,
      path: location.pathname,
      assets: location.pathname === "/" ? dashboardAssets() : [],
      reported_active_count: count ? Number(count[1]) : null,
      terms: list ? terms() : [],
      has_fixed_action: [...document.querySelectorAll("button")].some(
        (el) => visible(el) && text(el) === "Fixed-term Savings",
      ),
      asset_rate: breakdownRates(),
      savings_breakdown: /Interest breakdown/.test(value),
      settings: location.pathname === "/profile/settings" ? settings() : null,
      paginated: !!document.querySelector(
        '[aria-label*="Next"],[aria-label*="next"],button[aria-label*="Load more"]',
      ),
    };
  }
  const host = document.createElement("div");
  host.id = "money-tracker-nexo-export";
  const shadow = host.attachShadow({ mode: "closed" });
  const style = document.createElement("style");
  style.textContent =
    ":host{position:fixed!important;right:20px!important;bottom:24px!important;z-index:2147483646!important;font:14px system-ui!important}button{border:1px solid #b5d4c0;background:#18271f;color:#dcebe1;border-radius:24px;padding:12px 16px;cursor:pointer;box-shadow:0 4px 18px #0005}button:hover,button:focus-visible{background:#304a3b;outline:2px solid #b5d4c0}p{max-width:300px;background:#18271f;color:#dcebe1;padding:8px;border-radius:8px;margin:6px 0}";
  shadow.append(style);
  const button = document.createElement("button");
  button.textContent = "Export Nexo data";
  button.title = "Export Nexo data";
  const cancel = document.createElement("button");
  cancel.textContent = "Cancel";
  cancel.hidden = true;
  const status = document.createElement("p");
  status.setAttribute("role", "status");
  status.hidden = true;
  shadow.append(button, cancel, status);
  document.documentElement.append(host);
  button.onclick = async (event) => {
    if (!event.isTrusted || !authenticated()) return;
    button.disabled = true;
    cancel.hidden = false;
    status.hidden = false;
    status.textContent = "Starting…";
    try {
      const r = await chrome.runtime.sendMessage({ type: "start" });
      status.textContent = r?.message ?? "Unable to start";
    } catch {
      status.textContent = "Extension disconnected; reload Nexo to retry";
    } finally {
      button.disabled = false;
      cancel.hidden = true;
    }
  };
  cancel.onclick = (event) => {
    if (event.isTrusted) chrome.runtime.sendMessage({ type: "cancel" });
  };
  const sync = () => {
    host.hidden = !authenticated();
  };
  new MutationObserver(sync).observe(document.body, {
    childList: true,
    subtree: true,
  });
  sync();
  let collecting = false;
  for (const type of ["click", "input", "keydown"])
    document.addEventListener(
      type,
      (event) => {
        if (
          collecting &&
          event.isTrusted &&
          event.target !== host &&
          !host.contains(event.target)
        )
          chrome.runtime.sendMessage({ type: "interference" }).catch(() => {});
      },
      true,
    );
  chrome.runtime
    .sendMessage({ type: "state" })
    .then((state) => {
      if (state?.message && !state.running) {
        status.hidden = false;
        status.textContent = state.message;
      }
      if (state?.running) {
        collecting = true;
        button.disabled = true;
        cancel.hidden = false;
        status.hidden = false;
        status.textContent = state.message;
      }
    })
    .catch(() => {});
  chrome.runtime.onMessage.addListener((m, sender, reply) => {
    if (sender.id !== chrome.runtime.id) return;
    try {
      if (m.type === "observe") reply(observe());
      else if (m.type === "breakdown") {
        openBreakdown(m.asset);
        reply({ ok: true });
      } else if (m.type === "fixed") {
        fixedAction();
        reply({ ok: true });
      } else if (m.type === "progress") {
        collecting = m.running;
        button.disabled = m.running;
        status.hidden = false;
        status.textContent = m.message;
        cancel.hidden = !m.running;
        reply({ ok: true });
      }
    } catch {
      reply({ error: "Unrecognized Nexo view; stopped safely" });
    }
  });
})();
