use clap::Parser;
use std::path::PathBuf;

static APP_NAME: &str = "money-tracker";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Optional custom config path
    #[arg(short, long)]
    pub config_path: Option<PathBuf>,

    /// Cache directory
    #[arg(long, env = "APP_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Data directory
    #[arg(short, long, env = "APP_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Nexo CSV export (defaults to .cache/imports/nexo_transactions_05-10-2026_10-36-53.csv)
    #[arg(long)]
    pub nexo_csv: Option<PathBuf>,

    /// Extend price history back to this UTC date and print net worth (YYYY-MM-DD)
    #[arg(long)]
    pub historical_from: Option<String>,

    /// End of historical backfill (YYYY-MM-DD; defaults to now)
    #[arg(long)]
    pub historical_to: Option<String>,

    /// Import the CSV and query stored prices without contacting price providers
    #[arg(long)]
    pub offline: bool,
}

pub struct Config {
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
    pub nexo_csv_path: PathBuf,
}
impl Config {
    pub fn from_env(args: Args) -> Self {
        Self {
            nexo_csv_path: args.nexo_csv.unwrap_or_else(|| {
                PathBuf::from(".cache/imports/nexo_transactions_05-10-2026_10-36-53.csv")
            }),
            cache_dir: args
                .cache_dir
                .or_else(|| dirs::cache_dir().map(|p| p.join(APP_NAME)))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|p| p.join(".cache").join(APP_NAME))
                        .unwrap_or_else(|| PathBuf::from("./.cache").join(APP_NAME))
                }),

            data_dir: args
                .data_dir
                .or_else(|| dirs::data_dir().map(|p| p.join(APP_NAME)))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|p| p.join(".data").join(APP_NAME))
                        .unwrap_or_else(|| PathBuf::from("./.data").join(APP_NAME))
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    static STD_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::cache_dir().unwrap());
    static STD_DATA_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_dir().unwrap());

    #[test]
    fn test_env_var_parsing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        temp_env::with_vars(
            vec![
                ("APP_CACHE_DIR", Some("/tmp/test_cache_from_env")),
                ("APP_DATA_DIR", Some("/tmp/test_data_from_env")),
            ],
            || {
                // Simulate parsing arguments without providing them on the command line
                // clap will automatically pick up environment variables for fields marked with `env = "..."`
                let args = Args::parse_from(vec![APP_NAME]);

                // Assert that the environment variables were correctly picked up
                assert_eq!(
                    args.cache_dir,
                    Some(PathBuf::from("/tmp/test_cache_from_env"))
                );
                assert_eq!(
                    args.data_dir,
                    Some(PathBuf::from("/tmp/test_data_from_env"))
                );
            },
        );
    }

    #[test]
    fn test_cli_args_override_env_vars() {
        let _guard = ENV_MUTEX.lock().unwrap();
        temp_env::with_vars(
            vec![
                ("APP_CACHE_DIR", Some("/tmp/env_cache")),
                ("APP_DATA_DIR", Some("/tmp/env_data")),
            ],
            || {
                // Simulate parsing with command-line arguments that should override env vars
                let args = Args::parse_from(vec![
                    APP_NAME,
                    "--cache-dir",
                    "/tmp/cli_cache",
                    "--data-dir",
                    "/tmp/cli_data",
                ]);

                // Assert that command-line arguments take precedence
                assert_eq!(args.cache_dir, Some(PathBuf::from("/tmp/cli_cache")));
                assert_eq!(args.data_dir, Some(PathBuf::from("/tmp/cli_data")));
            },
        );
    }

    #[test]
    fn test_config_no_args_no_env() {
        let _guard = ENV_MUTEX.lock().unwrap(); // test reads env
        temp_env::with_vars(
            vec![
                ("APP_CACHE_DIR", None::<&str>),
                ("APP_DATA_DIR", None::<&str>),
            ],
            || {
                let args = Args::parse_from(vec![APP_NAME]);
                let config = Config::from_env(args);
                assert_eq!(config.cache_dir, STD_CACHE_DIR.join(APP_NAME));
                assert_eq!(config.data_dir, STD_DATA_DIR.join(APP_NAME));
            },
        );
    }

    #[test]
    fn test_config_override_cli_args() {
        let _guard = ENV_MUTEX.lock().unwrap(); // test reads env
        // Clear env so fallback assertions don't see the developer shell's values
        temp_env::with_vars(
            vec![
                ("APP_CACHE_DIR", None::<&str>),
                ("APP_DATA_DIR", None::<&str>),
            ],
            || {
                let args = Args::parse_from(vec![APP_NAME, "--cache-dir", "/tmp/cli_cache"]);
                let config = Config::from_env(args);
                assert_eq!(config.cache_dir, PathBuf::from("/tmp/cli_cache"));
                assert_eq!(config.data_dir, STD_DATA_DIR.join(APP_NAME));

                let args = Args::parse_from(vec![APP_NAME, "--data-dir", "/tmp/cli_data"]);
                let config = Config::from_env(args);
                assert_eq!(config.cache_dir, STD_CACHE_DIR.join(APP_NAME));
                assert_eq!(config.data_dir, PathBuf::from("/tmp/cli_data"));
            },
        );
    }

    #[test]
    fn test_config_override_env_vars() {
        let _guard = ENV_MUTEX.lock().unwrap();
        temp_env::with_vars(
            vec![
                ("APP_CACHE_DIR", Some("/tmp/env_cache")),
                ("APP_DATA_DIR", Some("/tmp/env_data")),
            ],
            || {
                // Simulate parsing with command-line arguments that should override env vars
                let args = Args::parse_from(vec![
                    APP_NAME,
                    "--cache-dir",
                    "/tmp/cli_cache",
                    "--data-dir",
                    "/tmp/cli_data",
                ]);

                // Assert that command-line arguments take precedence
                assert_eq!(args.cache_dir, Some(PathBuf::from("/tmp/cli_cache")));
                assert_eq!(args.data_dir, Some(PathBuf::from("/tmp/cli_data")));

                let config = Config::from_env(args);
                assert_eq!(config.cache_dir, PathBuf::from("/tmp/cli_cache"));
                assert_eq!(config.data_dir, PathBuf::from("/tmp/cli_data"));
            },
        );
    }

    #[test]
    fn nexo_csv_defaults_to_repo_cache_independent_of_data_dir() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let args = Args::parse_from([APP_NAME, "--data-dir", ".cache/data"]);
        let config = Config::from_env(args);
        assert_eq!(
            config.nexo_csv_path,
            PathBuf::from(".cache/imports/nexo_transactions_05-10-2026_10-36-53.csv")
        );

        let args = Args::parse_from([
            APP_NAME,
            "--data-dir",
            ".cache/data",
            "--nexo-csv",
            "exports/nexo.csv",
        ]);
        let config = Config::from_env(args);
        assert_eq!(config.nexo_csv_path, PathBuf::from("exports/nexo.csv"));
    }
}
