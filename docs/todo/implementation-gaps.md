# Implementation planning notes

Delivery status and priority are owned by the [master todo list](master-todo.md). These notes retain design alternatives from earlier proposals; they are not a second checklist. Roadmap numbers from the earlier export-based plan have been removed.

The [source-grounded data model proposal](../spec/data-model-proposal.md) now develops these alternatives into recommended boundaries and phases. It supersedes these short notes where more specific; no proposed schema changes have been implemented by the review.

- **Account metadata:** persist/retrieve provider catalogue, account instances, products, position relationships and dates alongside the existing ledger; preserve manual/imported observations and provenance.
- **Normal application sync:** expose Binance import/reconciliation beyond ignored tests and Nix fixture commands; report account coverage, cutoff, unsupported operations, and unresolved discrepancies. API-only history cannot establish completeness outside endpoint retention.
- **Nexo input management:** remove the dated default export path; select/import multiple original exports and preserve hash/stable-ID semantics. Consider a reader/content boundary in `nexo-csv` and managed storage for uploaded sources. Earlier proposals suggested a `FileStore` save/load/exists abstraction, restricted file permissions, and a setup import command. Choose the interface to fit the actual application workflow before implementing it.
- **Automated Nexo input:** decide whether environment/secret-manager content input is useful. Earlier proposals conflicted between plain `NEXO_CSV_CONTENT` and base64 input, and between `--file` and `--import` setup flags; none is a current contract. If selected, define one format, precedence, and sensitive-data handling. Encryption at rest remains an optional future storage design.
- **Exact amounts:** replace Nexo floating-point ingestion, maintain asset scales across imports, and test overflow and realistic high-precision holdings against the `u64` domain / signed `BIGINT` storage boundary.
- **CLI configuration:** implement configuration-file loading or remove the unused `--config-path` option; define precedence and parsing tests when selected.

Completed foundations: shared core library, integer transaction effects, DuckDB ledger and price persistence, atomic content-hash imports, historical prices and net worth, explicit asset/liability/external roles, ordered idempotent startup migrations, and frozen Binance statement/Earn reconciliation. Their acceptance requirements live in [specifications](../README.md). Do not schedule these again from old unchecked proposal tasks.
