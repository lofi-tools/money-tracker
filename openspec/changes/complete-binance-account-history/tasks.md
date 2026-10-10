- [x] Implement Spot, Funding and Simple Earn account history fetching and current balance snapshots
- [x] Map transactions and account metadata using exact amounts
- [x] Add reconciliation, cached account test, and ignored live test
- [x] Run formatting, tests, and spec validation (11 client tests and 28 CLI tests passed; live tests excluded)
- [x] Add typed Snafu errors for credentials, signing, requests, responses, cache I/O, provider context and reconciliation; verify safe source chains
- [ ] Finish real-account reconciliation: credentials loaded successfully through direnv using BINANCE_API_KEY_ID/BINANCE_API_KEY_SECRET; backfill fetched and cached historical responses, then Convert tradeFlow returned HTTP 429 / -1003 after retries

- [x] Reuse Nexo’s persistent import-marker pattern for the Binance test DB; atomically save independent observations and verify offline reuse, rebuild and retained discrepancies

- [x] Reproduce and handle universal-transfer total=0 responses without rows; constrain requests to the documented six-month retention and verify cache replay plus malformed-page errors

- [x] Add ignored cached Binance balance display, request/import progress logs and the Nix binance command; document recovery from API history retention gaps

- [x] Add shared request pacing, full Retry-After cooldowns, exponential fallback, bounded 429 retries, 418 handling and progress logs; document historical statement export steps

- [x] Load Binance CSV/ZIP from .cache/imports, cache decompressed CSV under .cache/test-data, and move Binance/Nexo test DBs there
- [x] Map statement movements, pair Spot/Funding transfers, preserve repeated rows, and deduplicate exact overlapping API/statement movements
- [x] Fetch/cache independent positions without full-history backfill and supplement omitted Earn records; add separate display and strict reconciliation tests
- [ ] Resolve the real fixture’s remaining 0.07192 SOL Flexible deficit using historical Earn evidence; every Spot, Funding and Locked position reconciles

- [x] Verify 20 Binance client, 34 CLI and 11 core tests; real ZIP/CSV and Nexo cached display replay; strict statement reconciliation retains the sole SOL discrepancy; OpenSpec validation and Nix syntax pass

- [x] Use .cache/api_responses/binance for history response JSON files; migrate existing local responses and preserve the old path for in-flight fetches

- [x] Share one HTTP transport and limiter across all client URLs/credentials in a process; track weighted history, consume usage headers/Spot limits, and learn lower effective budgets from throttling with gradual recovery

- [x] Verify weighted/adaptive limiter with virtual-time tests, mock 429/418 responses and cache bypass; 24 client and 34 CLI tests pass; OpenSpec validation passes
