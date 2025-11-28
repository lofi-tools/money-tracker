# Design: CLI and XDG Integration

## CLI Structure
We will use `clap` with the `derive` feature for type-safe argument parsing.

```rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Optional custom config path
    #[arg(short, long)]
    config: Option<PathBuf>,
}
```

## XDG Integration
we use the dirs crate to manage standard directories across OSes.

```rust

```

## Environment Variables
We will check for environment variables to override default paths.
- `APP_CACHE_DIR`: Overrides the XDG cache directory.
- `APP_DATA_DIR`: Overrides the data directory (which defaults to cache dir for now).

Priority order:
1. CLI Arguments (if applicable for specific paths)
2. Environment Variables
3. XDG Defaults

## Module Structure
- `bin/cli/src/cli.rs`: Contains the `clap` struct definitions.
- `bin/cli/src/main.rs`: Entry point, initializes CLI and XDG, then proceeds with application logic.
