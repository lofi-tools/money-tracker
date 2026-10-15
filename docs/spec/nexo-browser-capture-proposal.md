# Proposed Nexo browser capture

Status: **approved design, implementation pending**, approved 15 October 2026. Owned by the existing account-specific Nexo capture task in [master todo](../todo/master-todo.md). This extends [current inferred allocations](nexo-current-allocations.md), retaining the [transaction matcher's ambiguity rules](../findings/nexo-transaction-inference.md). No extension, importer, account capture or database change is delivered by this document.

## Findings and unresolved evidence

A Manifest V3 extension can use a site-specific [content script](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts) to place a floating control on Nexo pages. The user's revised workflow requires persistent site access granted at setup, rather than toolbar-only temporary access. Its isolated world can read the DOM without depending on Nexo's internal JavaScript objects. [Native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) provides a browser-to-local-application connection through a registered native host; content scripts pass messages through the extension service worker. These capabilities support the proposed workflow, but neither the extension nor a tracker bridge exists yet.

Nexo's [fixed-term guide](https://support.nexo.com/article/how-to-earn-interest-with-fixed-term-yield-us) points to Active terms and account rate breakdowns. It distinguishes term renewal from a separate setting for reinvesting interest, and documents that a payout preference can change within a term. These are reasons to capture scoped current settings and accrued rewards by currency separately. The guide is US-labelled: its rates, defaults, eligibility and UI availability are **not evidence for this UK account**. Capture only what the actual account displays.

No logged-in UK web DOM was inspected in this investigation. The actual web origin, selectors, whether terms expose durable IDs, principal precision, start/due timezones, APR/APY labels and complete-list counts are unverified. The non-US public savings guide did not expose its useful body to the web reader. No documented retail savings API was established; that is a research limitation, not proof that one does not exist.

## Revised workflow: one-click account collection and ingestion

This supersedes the earlier manual per-view capture/download proposal, following the user's correction on 14 October.

1. During installation, pair the extension with the local tracker, select the existing portfolio/Nexo account and confirm its database. Grant access only to the verified Nexo website/account origins required by the actual UI. Hostnames and authenticated selectors are still to be verified; do not grant all subdomains by assumption.
2. While the user is browsing Nexo and visibly signed in, show a compact floating button over the page. Hover/focus displays **Export Nexo data**; the same control works with keyboard activation. Hide it on logout and unsupported origins. It must not cover Nexo transaction controls.
3. Clicking that button starts one export run. The extension automatically navigates the account's fixed-term list, every term detail, savings-rate breakdown and payout/renewal settings, including read-only pagination, list filters and lazy-loaded rows. No repeated user capture clicks or mandatory JSON download/import steps.
4. The floating control displays progress, counts, partial-coverage warnings and Cancel. Use a deterministic navigation state machine tied to this tab/account/run. Pause on login expiry, account switching, user interference or unfamiliar layout; never claim a complete capture after a skipped/failed view. Restore the initial account view when safe after completion/cancellation.
5. Send the captured JSON directly through the local bridge. The tracker validates and ingests the original snapshot plus uniquely supported subscription evidence into its database. Ambiguous matches/conflicts are stored unresolved and visible in the tracker. The click authorizes this ingestion into the already paired account; no extra confirmation for each term or a mandatory review screen before every run.
6. Show the tracker acknowledgement: imported/new/updated/unresolved counts, coverage and any pinned conflicts. A transport receipt alone is not DB success. Retain the unsent/failed run in session memory for an explicit Retry; never silently send it to another database/account. An optional local JSON download can be added later, but it is not the primary flow.

Recommend an unpacked development extension under a future `apps/nexo-capture-extension/`, a small native bridge packaged with the tracker, and shared Rust ingestion/matching. Store publication and scheduled background collection remain later decisions. This proposal does not install or run either component.

### Read-only navigation contract

The driver may open verified list/detail/settings views, switch read-only tabs or filters, page and scroll. Every action has a specific expected destination/DOM state, bounded waits and a visited-view record. A row ordinal may guide navigation during one run but cannot become term identity or justify deduplication. Repeated identical rows remain separate observations unless identity evidence supports merging.

Never press Create/Confirm term, accept agreements, toggle renewal/payout/reinvestment, submit edits, transfer, trade or withdraw. Settings are read from their current values. Stop rather than guess when a view cannot be distinguished from an edit/confirmation flow. There is no cookie/token scraping, network interception, hidden endpoint discovery or direct Nexo API calling in the proposed implementation. Normal page navigation may cause Nexo's own authenticated requests.

Completeness requires all applicable assets/pages/details and stable list counts/filter context to agree; explicit coverage evidence distinguishes partial from complete. Terms can renew while the run is collecting them, so preserve per-field observation time, recheck the list at the end and mark inconsistent runs partial. Global current payout preference and per-term/accrued payout facts retain separate scopes.

### Extension permissions and lifetime

Propose `scripting`, `storage`, `nativeMessaging`, plus optional host permissions granted once for the verified Nexo web origins and dynamically registered content scripts. Site access is needed to display the floating button without a toolbar gesture. No `<all_urls>`, cookies, webRequest, debugger, remote code, externally-connectable webpage API or downloads permission in the primary flow. This corrects the earlier temporary-access-only design. See [Chrome's permission declaration guidance](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions).

Use [session storage](https://developer.chrome.com/docs/extensions/reference/api/storage) for one working run, restricted to trusted extension contexts; clear after committed ingestion, cancellation/discard or expiry. No browser sync. Only display/connection preferences persist. The service worker checks sender extension/tab/origin, active-run token, message type and bounds before forwarding. A page-dispatched synthetic event cannot trigger collection/native messages; require a real activation of the extension's control. Unknown/unsolicited messages fail closed.

### Local bridge and database ownership

Recommend **native messaging**, rather than a localhost HTTP server: the installed host is restricted to the paired extension ID and uses JSON messages over stdio. This avoids adding a web-accessible ingestion endpoint. Chrome starts the native host on connection; per-browser host registration and a stable extension ID are installation requirements. Only the extension service worker connects to it, not page code. The bridge is a proposed local tracker component, not a Nexo service. [Chrome native messaging reference](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)

Pair once to an existing local portfolio/account through tracker UI. Use an opaque local connection ID; browser payloads must not select arbitrary DB paths or execute commands. The tracker owns database resolution via [shared startup configuration](database-startup.md), including the standard `~/.data/money-tracker/portfolio.duckdb` on macOS/Linux and development override. A browser-launched process may not inherit the developer shell's environment; resolve the paired/configured destination explicitly and show it during setup. Never create a second empty database on a connection error.

The native host forwards a bounded JSON envelope (connection ID, capture ID/hash, schema and payload) into an owner-only local inbox using atomic file replacement. The installed desktop tracker can be launched if needed, then its existing database worker discovers/ingests this inbox. The bridge does not concurrently open DuckDB alongside the desktop. Add a local result acknowledgement keyed by capture ID; delete/transient-clear inbox data only after a durable result. This proposed inbox/watcher/acknowledgement path does not exist today. It must handle duplicates, crash recovery, stale connection IDs and app/busy errors without duplicate effects. Keep source snapshots as immutable application evidence, not arbitrary writable browser files. Impose application limits below browser transport limits and return only compact status, never account credentials.

Wire the bridge to the shared Rust importer, not a second financial mapper. Validate exact schema, account binding, source origin, precision, hash and resource limits; reject malformed captures atomically. Valid partial captures may be ingested with partial coverage. Persist unresolved facts even when no transaction binding is possible. Keep existing manual pins/precedence and report conflicts without silently overwriting them. No direct ledger balance edits or inferred financial transactions.

## Proposed JSON contract

Versioned, bounded UTF-8 JSON; financial decimal values are strings. Preserve source text plus normalized value, precision and uncertainty. Missing values are `null`, never zero/false. Keep actual dates separate from calculated expected dates. Date-only observations stay date-only; never silently turn local midnight into a UTC timestamp.

Illustrative **synthetic** observation (a design example, not account evidence):

```json
{
  "schema": "money-tracker.nexo-terms",
  "version": 1,
  "capture_id": "synthetic-session-001",
  "capture_started_at": "2026-10-14T12:00:00Z",
  "capture_finished_at": "2026-10-14T12:02:00Z",
  "extractor_version": "proposed-v1",
  "account_hint": { "local_label": "My Nexo", "jurisdiction": "GB", "source": "user_entered" },
  "coverage": { "status": "partial", "reported_active_count": null, "views_captured": 1 },
  "account_settings": { "payout_asset": null, "renew_with_interest": null },
  "terms": [{
    "observation_id": "synthetic-observation-001",
    "provider_term_id": null,
    "observed_at": "2026-10-14T12:01:00Z",
    "status": "active",
    "principal": { "asset": "ETH", "amount": "45.00", "precision": "displayed", "exactness": "unknown" },
    "rate": { "percent": "6.25", "basis": "apy", "raw_label": "6.25% APY", "scope": "term" },
    "payout": { "asset": "NEXO", "scope": "term", "historical_continuity": "unknown" },
    "start": { "value": "2026-10-03", "precision": "date", "timezone": null },
    "maturity": { "value": "2026-11-03", "precision": "date", "timezone": null },
    "duration_months": 1,
    "auto_renew": true,
    "renew_with_interest": null,
    "accrued_rewards": [],
    "evidence": { "view": "term_detail", "fields": { "principal": "45.00 ETH", "start": "3 Oct 2026", "maturity": "3 Nov 2026", "auto_renew": "checked" } },
    "warnings": ["Principal may be rounded; transaction linkage unresolved"]
  }],
  "warnings": ["Only one detail view captured; account coverage is partial"]
}
```

The final schema must also carry the verified origin, sanitized page path without query/hash, locale, browser timezone (context only), per-setting observation time and field provenance. User-entered jurisdiction/account labels are hints, not provider identity. Never capture account email, names, cookies, authentication tokens, whole HTML, screenshots or unrelated dashboard content by default. Extract only bounded relevant labels/values; render all page strings as text and treat them as untrusted data.

Principal means original deposited principal, not a term balance including accrued interest. If the UI does not distinguish them, retain a labelled balance observation and leave principal unknown. Record full displayed rate and its explicit basis (`apy`, `apr`, or `unknown`); do not add tier/payout bonuses twice, promote an offer maximum to a held rate, or infer a compounding frequency. A global payout selection cannot override contrary per-term evidence. Current payout settings do not establish past payout history. Capture renewal-with-interest separately from per-term automatic renewal; absent toggles stay unknown. Any visible in-kind/NEXO accrued rewards remain separate quantities, never new ownership movements.

## Matching and automatic local ingestion

The extension captures observations; **matching belongs in shared Rust application code**, not the page parser. The paired existing Nexo account determines the destination; validate the JSON envelope/schema/limits and hash the original bytes. The tracker automatically applies unique supported matches and records every unresolved candidate/reason. Manual resolution remains available afterwards; it is not a prerequisite for exporting.

| Evidence | Proposed outcome |
| --- | --- |
| Provider term ID already bound to a local subscription and consistent facts | Reuse that binding; conflicting principal/dates require review. A new external ID alone cannot bind a CSV event because the CSV lacks it. |
| Exact asset and explicitly full-precision original principal, plus source start date/time identifying exactly one feasible lock | Offer a unique match after one-to-one competition across all observations and candidate events. A date-only start matches a timezone-qualified interval; unknown timezone or rounded amount blocks automatic linkage. |
| Same asset/principal but multiple feasible locks, missing start, uncertain precision, conflicting evidence or several captures competing for one lock | Unresolved, with candidate IDs and reason. No FIFO, nearest date, expected maturity, APY or expected-yield tiebreaker. |
| Captured current term with no supported CSV link | Retain as an unlinked provider observation, not a new asset. It can be shown in an unresolved evidence panel. |

A provider ID stabilizes repeated captures only if its semantics are established; DOM row order, a generated element ID and a hash of rounded values are not provider IDs. Repeated no-ID observations are possibly duplicates until a unique identity is established; never deduplicate distinct identical pots by their values.

Current equal-principal cohorts preserve candidate openings but ambiguous releases. A uniquely identified capture may establish one still-active opening; do not assign identities to remaining released/open members by elimination without enough evidence. Any proposed cohort resolution must compete globally and preserve an audit link to the original ambiguity.

Older CSV ownership and newer active-term observations can disagree. Capture does not silently repair financial history. Compare principals only against ownership at a compatible cutoff; if coverage/timing is incompatible or current locked principal exceeds trusted ownership, retain evidence and withhold the affected partition rather than clamping or adding assets. Partial-list absence never closes a term. Even a complete list does not establish an exact unlock/closure event or date.

The proposed importer appends dated immutable capture evidence and uniquely supported bindings; user-resolved bindings retain manual audit/Undo. Existing `Subscription` already supports maturity/renewal but only timestamp dates; date-only evidence therefore needs a precision-preserving capture record instead of invented UTC times. `RateEvidence` can hold a matched subscription's current annual rate, beginning at observation time unless an actual effective date is shown. Existing pins and manual precedence remain intact; conflicts are surfaced, not silently overwritten. Unknown basis stays captured evidence without manufacturing a usable APY. Keep captured account rate, expected annotated rate and historical observed estimate distinct. No backdating today's rate across the term, product-wide rate broadcast, synthetic rewards or duplicate subscription positions.

## Implementation stages and acceptance

These are implementation/commit stages of the same one-click workflow, not requests for the user to operate intermediate manual capture/import steps.

1. **Verify account UI and navigation:** inspect current UK term/list/rate/settings views; establish origin, authenticated markers, safe navigation, ID stability, locale/precision and complete coverage. Save redacted/synthetic DOM fixtures; never commit account data. Report web-only limitations where facts are unavailable rather than claim mobile-only information is supported.
2. **Extension collection:** floating authenticated-page button, read-only automated traversal, progress/Cancel, session collection and precise JSON. Test login expiry, account switching, user navigation, timeouts, layout changes, pagination/virtualization, renewal during collection, duplicate identical pots, unknown rate basis and absent toggles. Assert that no setting/financial action occurs and that unrelated origins cannot start/continue a run.
3. **Bridge and shared ingestion:** stable extension ID, explicit pairing/install registration, native host, owner-only inbox and desktop worker ingestion. Add global one-to-one matching, candidate reasons and durable acknowledgements. Synthetic end-to-end tests cover app start, missing/wrong DB, malformed payload, bad origin/token, restart after commit-before-ack, duplicate retries, partial coverage, pins and ambiguous cohorts. Do not ingest account email/tokens or expose a generic command/SQL endpoint.
4. **Integrated account trial:** one click browses all readable term/settings views, sends JSON and ingests uniquely supported facts. Unknowns appear in the tracker. Date-only evidence remains date-only; actual maturity labels require captured precision. Captured principal never adds to original ownership. Compare protected financial tables before/after, show capture coverage/ingestion counts and failures, and present results before committing.

Approval would authorize implementing the floating-button, automatic read-only collection and local ingestion flow above. It would not authorize changing Nexo settings, creating/renewing terms, performing financial actions, API interception or publication. No implementation is included in this proposal update.
