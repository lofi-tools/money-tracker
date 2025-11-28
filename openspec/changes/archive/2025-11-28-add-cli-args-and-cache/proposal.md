# Proposal: Add CLI Arguments and Cache Directory Support

## Summary
Introduce `clap` for command-line argument parsing and `xdg` (via `directories` or similar) for standardizing file paths. This change also involves refactoring the main entry point to use a `cli.rs` module and directing data storage to the cache directory for now.

## Motivation
- **Standardization**: Use standard CLI argument parsing.
- **Compliance**: Adhere to XDG base directory specifications for configuration and data.
- **Flexibility**: Allow users to control behavior via CLI flags.

## Proposed Changes
1.  Add `clap` and `dirs` dependencies to `bin/cli`.
2.  Create `bin/cli/src/cli.rs` to handle argument parsing.
3.  Update `bin/cli/src/main.rs` to use the new CLI module.
4.  Implement environment variable overrides (`APP_CACHE_DIR`, `APP_DATA_DIR`) for directory paths.
5.  Configure the application to use the standard cache directory (via `dirs`) or its override for data storage temporarily.
