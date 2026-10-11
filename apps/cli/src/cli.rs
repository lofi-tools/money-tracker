use clap::{Parser, Subcommand};
use std::path::PathBuf;

static APP_NAME: &str = "money-tracker";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
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

    /// Use local inputs/caches without contacting provider or price APIs
    #[arg(long)]
    pub offline: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Discover/import EVM wallet quantities and reconcile against pinned RPC balances
    Evm {
        /// Public wallet address; stored in account metadata (never needs a seed/key)
        #[arg(long, required_unless_present_any=["account", "list"], conflicts_with="account")]
        address: Option<String>,
        /// Select an existing wallet account from the portfolio DB
        #[arg(long)]
        account: Option<String>,
        /// List saved wallet accounts without network acquisition
        #[arg(long)]
        list: bool,
        #[arg(long, value_delimiter = ',', default_value = "1,137,56,42161,324")]
        chain: Vec<u64>,
        #[arg(long, default_value=lib_core::DEFAULT_PORTFOLIO)]
        portfolio: String,
        /// Register wallet identities without contacting providers
        #[arg(long)]
        register_only: bool,
        /// Recheck unresolved metadata at the frozen snapshot block, without refetching history
        #[arg(long)]
        resolve_tokens: bool,
        /// Apply the reviewed Ethereum SafePromo spam exclusion to this saved account
        #[arg(long, requires = "account", conflicts_with = "restore_token")]
        exclude_safe_promo: bool,
        /// Restore tracking for this exact ERC20 contract (retains review history)
        #[arg(long, requires = "account")]
        restore_token: Option<String>,
        /// JSON-RPC override (select one chain when overriding endpoints)
        #[arg(long)]
        rpc: Option<String>,
        /// Compatible Blockscout v2 base URL; BNB otherwise uses Etherscan V2
        #[arg(long)]
        indexer: Option<String>,
        #[arg(long, default_value_t = 64)]
        confirmations: u64,
        #[arg(long)]
        snapshot: Option<PathBuf>,
        #[arg(long)]
        refresh: bool,
    },
    /// Import/replay a Kraken account and print exact reconciled balances
    Kraken {
        /// Stable local account ID; reuse for this account even if API keys change
        #[arg(long)]
        account: String,
        #[arg(long, default_value = lib_core::DEFAULT_PORTFOLIO)]
        portfolio: String,
        /// Frozen JSON snapshot; defaults to an account-specific cache path
        #[arg(long)]
        snapshot: Option<PathBuf>,
        /// Explicitly replace the frozen snapshot with a fresh API fetch
        #[arg(long)]
        refresh: bool,
    },
}

pub struct Config {
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
    pub nexo_csv_path: PathBuf,
}
impl Config {
    pub fn from_env(args: Args) -> anyhow::Result<Self> {
        let resolved = lib_app::config::ResolvedConfig::system(args.config_path, args.data_dir)?;
        if !resolved.database.try_exists()? {
            if let Some(legacy) = &resolved.legacy_database {
                anyhow::ensure!(
                    !legacy.try_exists()?,
                    "previous database exists at {}; select it explicitly with --data-dir or copy it to {} before continuing",
                    legacy.display(),
                    resolved.database.display()
                );
            }
        }
        Ok(Self {
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

            data_dir: resolved
                .database
                .parent()
                .expect("resolved database has a parent")
                .to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    static STD_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::cache_dir().unwrap());

    #[test]
    fn config_file_loads_and_env_and_flags_take_precedence() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        lib_app::config::AppConfig {
            data_dir: Some(dir.path().join("configured")),
            ..Default::default()
        }
        .save(&path)
        .unwrap();
        temp_env::with_var("APP_DATA_DIR", None::<&str>, || {
            let args = Args::parse_from([APP_NAME, "--config-path", path.to_str().unwrap()]);
            assert_eq!(
                Config::from_env(args).unwrap().data_dir,
                dir.path().join("configured")
            );
        });
        temp_env::with_var(
            "APP_DATA_DIR",
            Some(dir.path().join("env").to_str().unwrap()),
            || {
                let args = Args::parse_from([APP_NAME, "--config-path", path.to_str().unwrap()]);
                assert_eq!(
                    Config::from_env(args).unwrap().data_dir,
                    dir.path().join("env")
                );
                let flag = dir.path().join("flag");
                let args = Args::parse_from([
                    APP_NAME,
                    "--config-path",
                    path.to_str().unwrap(),
                    "--data-dir",
                    flag.to_str().unwrap(),
                ]);
                assert_eq!(Config::from_env(args).unwrap().data_dir, flag);
            },
        );
        let args = Args::parse_from([
            APP_NAME,
            "--config-path",
            dir.path().join("missing.json").to_str().unwrap(),
        ]);
        assert!(Config::from_env(args).is_err());
    }

    #[test]
    fn kraken_command_requires_account_and_accepts_offline_replay() {
        assert!(Args::try_parse_from([APP_NAME, "kraken"]).is_err());
        let args = Args::try_parse_from([
            APP_NAME,
            "--offline",
            "kraken",
            "--account",
            "kraken:alice",
            "--snapshot",
            "frozen.json",
        ])
        .unwrap();
        assert!(args.offline);
        assert!(
            matches!(args.command, Some(Command::Kraken { account, portfolio, refresh: false, .. }) if account == "kraken:alice" && portfolio == lib_core::DEFAULT_PORTFOLIO)
        );
    }
    #[test]
    fn evm_requires_locator_but_saved_accounts_and_list_need_no_address() {
        assert!(Args::try_parse_from([APP_NAME, "evm"]).is_err());
        assert!(Args::try_parse_from([APP_NAME, "evm", "--list"]).is_ok());
        let a = Args::try_parse_from([APP_NAME, "--offline", "evm", "--account", "saved-wallet"])
            .unwrap();
        assert!(a.offline);
        assert!(
            matches!(a.command, Some(Command::Evm { address: None, account: Some(id), .. }) if id == "saved-wallet")
        );
        assert!(
            Args::try_parse_from([
                APP_NAME,
                "evm",
                "--account",
                "saved",
                "--address",
                "address"
            ])
            .is_err()
        );
    }
    static STD_DATA_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        lib_app::config::Locations::system()
            .unwrap()
            .default_data_dir
    });

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
                let config = Config::from_env(args).unwrap();
                assert_eq!(config.cache_dir, STD_CACHE_DIR.join(APP_NAME));
                assert_eq!(config.data_dir, *STD_DATA_DIR);
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
                let config = Config::from_env(args).unwrap();
                assert_eq!(config.cache_dir, PathBuf::from("/tmp/cli_cache"));
                assert_eq!(config.data_dir, *STD_DATA_DIR);

                let args = Args::parse_from(vec![APP_NAME, "--data-dir", "/tmp/cli_data"]);
                let config = Config::from_env(args).unwrap();
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

                let config = Config::from_env(args).unwrap();
                assert_eq!(config.cache_dir, PathBuf::from("/tmp/cli_cache"));
                assert_eq!(config.data_dir, PathBuf::from("/tmp/cli_data"));
            },
        );
    }

    #[test]
    fn nexo_csv_defaults_to_repo_cache_independent_of_data_dir() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let args = Args::parse_from([APP_NAME, "--data-dir", ".cache/data"]);
        let config = Config::from_env(args).unwrap();
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
        let config = Config::from_env(args).unwrap();
        assert_eq!(config.nexo_csv_path, PathBuf::from("exports/nexo.csv"));
    }
}
