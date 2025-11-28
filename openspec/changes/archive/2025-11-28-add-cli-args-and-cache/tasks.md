# Tasks: Add CLI Args and Cache

- [x] Add `clap` (with `derive` feature) and `dirs` dependencies to `bin/cli/Cargo.toml`.
- [x] Create `bin/cli/src/cli.rs` and define the `Args` struct.
- [x] Refactor `bin/cli/src/main.rs` to use `cli::Args` and `dirs`.
- [x] Implement logic to check `APP_CACHE_DIR` and `APP_DATA_DIR` environment variables and override paths accordingly.
- [x] Verify the application compiles and runs with `--help`.
