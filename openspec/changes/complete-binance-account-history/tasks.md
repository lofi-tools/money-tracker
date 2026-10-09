- [x] Implement Spot, Funding and Simple Earn account history fetching and current balance snapshots
- [x] Map transactions and account metadata using exact amounts
- [x] Add reconciliation, cached account test, and ignored live test
- [x] Run formatting, tests, and spec validation (11 client tests and 28 CLI tests passed; live tests excluded)
- [x] Add typed Snafu errors for credentials, signing, requests, responses, cache I/O, provider context and reconciliation; verify safe source chains
- [ ] Finish real-account reconciliation: credentials loaded successfully through direnv using BINANCE_API_KEY_ID/BINANCE_API_KEY_SECRET; backfill fetched and cached historical responses, then Convert tradeFlow returned HTTP 429 / -1003 after retries

- [x] Reuse Nexo’s persistent import-marker pattern for the Binance test DB; atomically save independent observations and verify offline reuse, rebuild and retained discrepancies

- [x] Reproduce and handle universal-transfer total=0 responses without rows; constrain requests to the documented six-month retention and verify cache replay plus malformed-page errors

- [x] Add ignored cached Binance balance display, request/import progress logs and the Nix binance command; document recovery from API history retention gaps
