# Connecting Kraken: user instructions and acquisition strategy

Checked against official Kraken documentation on 2026-10-11. **Decision: use manual read-only API keys for now; defer OAuth.** CSV ingestion is also future work. See [the provider contract](../spec/kraken.md) for precise mappings, supported products, reconciliation and tests.

## Connection options

| Question | Answer |
| --- | --- |
| Does Kraken offer OAuth? | Yes: Kraken Connect/Fast API. It is not implemented in this app. |
| Can we use a localhost redirect? | Not established by the official documentation reviewed. Do not promise this yet. |
| What works in this release? | A user-created read-only Spot API key and private key, supplied through environment variables. |
| Can history be fetched automatically? | Yes, the ledger API supports paginated historical queries. We request all available entries from epoch start through a fixed cutoff. |
| Is a manual export required? | Normally no for supported, reconciling Spot accounts. Exports can help investigate missing history or unsupported records, but this release cannot ingest Kraken CSVs. |

**Later OAuth:** [Kraken Connect/Fast API](https://docs-legacy.kraken.com/api/docs/oauth/kraken-connect/) lets users sign in and authorize an app to create API keys, avoiding manual copying. Kraken's [developer index](https://docs.kraken.com/llms.txt) lists OAuth as requiring B2B partner approval: Kraken must approve/register our app, separately from each user's consent. Eligibility, fees and approval timing are unconfirmed. Public desktop clients are documented, but localhost redirects and PKCE remain unverified. Track registration and callback verification in [the later OAuth task](../todo/master-todo.md#3-third-implement-kraken); manual keys remain sufficient for current local tracking.

## User setup for the current CLI

1. Sign in to [Kraken Pro](https://pro.kraken.com/). Open the profile menu → Settings → API → Create API key. Name it `Money Tracker`.
2. Enable **Query funds** and **Query ledger entries** only. Export Data is unnecessary for the implemented connector. Do not enable trading, funding actions or Earn actions.
3. Leave Query Start/End Date/Time restrictions unset to allow lifetime backfill. A date-restricted key can hide older activity. If using an IP whitelist or expiration, ensure it permits the computer running the app.
4. Generate the key and save the API key and private key in your password manager. These steps and settings follow [Kraken's API-key instructions](https://support.kraken.com/articles/360000919966-how-to-create-an-api-key).
5. API-key action 2FA requiring an OTP on each request is unsupported by our client. Account-login 2FA can remain enabled; it is separate from this API-key setting.

Provide `KRAKEN_API_KEY` (API/public key) and `KRAKEN_API_SECRET` (the base64 private key) in the command's environment, preferably using password-manager injection. The app currently has no credential-entry UI or keychain integration and does not read these from a configuration file. Keep actual credentials out of source files, committed `.env` files, command arguments and diagnostic output.

Run from the repository root:

```sh
# First acquisition; the two credential environment variables must already be set:
cargo run -p cli -- --data-dir ./data --cache-dir .cache kraken --account kraken:primary
# Replay the frozen evidence without credentials or network acquisition:
cargo run -p cli -- --offline --data-dir ./data --cache-dir .cache kraken --account kraken:primary
# Explicitly obtain a newer snapshot:
cargo run -p cli -- --data-dir ./data --cache-dir .cache kraken --account kraken:primary --refresh
```

Choose a distinct, stable local `--account` ID for each real Kraken account. Keep that ID when rotating keys for the same account. It is an app identity, not a remote Kraken wallet selector. The current connector reads the authenticated user's default wallet. Global flags go before `kraken`; `--refresh` goes after it. Existing complete caches replay by default, so running the first command again does not automatically refresh.

The command prints exact owned balances and independently reconciles them. A mismatch exits unsuccessfully and retains the snapshot for investigation. First check key date restrictions, unsupported products and activity during acquisition; an explicit refresh on a quiet account can resolve timing differences. Do not invent transactions to force agreement. Authentication failures require checking the key pair, permissions and key settings; cached offline replay does not require valid credentials.

## History coverage and manual exports

The [ledger endpoint](https://docs.kraken.com/api-reference/account-data/get-ledgers-info) accepts start/end bounds and returns paginated entries. No short retention window is stated on that reference. We request `type=all`, `start=0`, a fixed end and every 50-row offset page. This is a backfill of **available default-wallet ledger history**, not a guarantee of every product's complete lifetime activity. Key date restrictions, remote wallets, regional variants and unsupported products must still be investigated. Reconciliation proves a supported current quantity comparison; it does not by itself prove every historical event or earnings classification is complete.

We use ledger entries as the monetary source, rather than adding trade history again. [Kraken's tax API guidance](https://support.kraken.com/in/articles/how-to-use-our-api-for-taxes) explains why ledger coverage matters, including instant buy/sell activity absent from ordinary trade history. Margin, derivatives, credit and unsupported asset/ledger types are explicitly outside our present mapper; an export alone does not make these supported.

To obtain investigation evidence manually, sign in to Kraken Web → profile/name → **Documents** → Exports → **New export** → **Ledger**. Choose a date range covering first activity through the acquisition cutoff, retain all assets/types and all available fields, choose CSV and Generate. Download when ready; generated exports are available for 14 days. Use [Kraken's document instructions](https://support.kraken.com/articles/download-request-documents), which also link the Kraken Pro equivalent. Keep a balance export separately if useful for a dated comparison.

This release's `--snapshot` takes our frozen JSON snapshot, **not** a Kraken CSV, ZIP or statement PDF. Keep manual files locally for investigation and the future CSV recovery importer. A future importer must preserve native ledger IDs and reconcile against independent balances so overlap with API history never duplicates effects.

Kraken also offers [API-generated exports](https://support.kraken.com/ca/articles/360025484891-how-to-export-your-account-history-via-api) using AddExport, ExportStatus and RetrieveExport. That gives us an automated recovery option before asking users to download files. It requires Export Data in addition to ledger access and is not implemented here.

## Strategy used and next connection work

Implemented acquisition is local first: read credentials from the process environment, fetch Assets metadata, page signed Ledgers requests through a fixed cutoff, then independently fetch BalanceEx. Serialize private requests with monotonic nonces, pace calls and retry bounded throttle failures. Validated pages and an interrupted-fetch cutoff support resumable acquisition. A complete frozen snapshot supports replay with no credentials or network; it stores a public-key fingerprint and financial evidence, never the secret or signatures. Financial caches still contain private account data.

The adapter preserves exact decimals, account-scopes native identities, records actual fees once and checks ledger-derived quantities against independent observations. Unsupported mappings fail visibly. Acquisition and mapping responsibilities are separated in [the client](../../libs/kraken-client/src/lib.rs) and [the adapter](../../apps/cli/src/adapters/kraken.rs). Synthetic/mock tests establish these contracts; no live private-account verification is claimed.

Proposed next steps belong in [the Kraken todo section](../todo/master-todo.md#3-third-implement-kraken): preflight permissions/date restrictions with GetApiKeyInfo; preferred cursor paging; API export recovery plus manual CSV ingestion; remote wallet selection; and live fixture coverage. For OAuth, first obtain partner approval and confirm desktop callback/PKCE rules. Our intended desktop design is a public client with credentials retained locally; a future cloud confidential client keeps its secret on the server. Both should feed the same acquisition/mapping pipeline and request minimal access. OAuth is an onboarding improvement, not a dependency of free local tracking.
