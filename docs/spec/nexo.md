# Nexo CSV ingestion

[CSV reading](../../libs/nexo-csv/src/lib.rs) owns parsing and exact-byte SHA-256 file identity. [The adapter](../../apps/cli/src/adapters/nexo.rs) owns transaction semantics, conversion to domain types, position registration, and import orchestration.

The adapter accepts a configurable CSV path via [CLI configuration](cli.md). It checks a completed file hash before parsing/mapping. Register assets, effects, and the hash atomically; skip identical successfully imported bytes even at a different path. Changed bytes reuse stable `NEXO:` transaction IDs to replace effects. A failed import records no completed hash and can be retried.

Movements use eight-decimal integer units. Current Nexo parsing still passes through floating point and rounding; exact parsing is an outstanding precision task. Register one aggregate `NEXO:<ticker>` asset position per ticker. The export cannot reliably distinguish individual product/wallet positions, so individual lookup is unsupported and products are not fabricated. Internal wallet transfers and term locks have no aggregate asset effect. Exchanges reduce input assets and increase output assets.

Credit mapping uses separate `NEXO-CREDIT:` liability positions, including the current USD proxy. Loan draw, mapped fees/interest, and duplicate merchant/FX draw handling must preserve signed debt without treating external counterparties as liabilities. The role migration upgrades legacy databases without reimport. Generic loan lifecycle, repayment coverage, collateral terms, and claim metadata still need fuller source-backed modeling; the existing liability fix does not imply every Nexo event type is supported.

## Valuation limitations

An incomplete export can produce negative aggregate assets if funding or transfers are omitted. Nexo USD/fiat proxies and export-implied NEXO prices are valuation assumptions documented in [historical valuation](historical-net-worth.md). No wallet/subscription detail or yield rate should be inferred from aggregate movements alone.

Managed file upload/storage, a setup command, environment content input, and automatic discovery were proposed but are not implemented. They are retained as [future design choices](../todo/implementation-gaps.md), rather than requirements describing current behavior.
