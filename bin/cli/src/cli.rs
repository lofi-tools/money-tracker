use clap::Parser;
use std::path::PathBuf;

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
}

pub struct Config {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_var_parsing() {
        unsafe {
            // Clear any existing environment variables that might interfere
            env::remove_var("APP_CACHE_DIR");
            env::remove_var("APP_DATA_DIR");

            // Set test environment variables
            env::set_var("APP_CACHE_DIR", "/tmp/test_cache_from_env");
            env::set_var("APP_DATA_DIR", "/tmp/test_data_from_env");

            // Simulate parsing arguments without providing them on the command line
            // clap will automatically pick up environment variables for fields marked with `env = "..."`
            let args = Args::parse_from(vec!["cli_app_name"]); // "cli_app_name" is the program name

            // Assert that the environment variables were correctly picked up
            assert_eq!(
                args.cache_dir,
                Some(PathBuf::from("/tmp/test_cache_from_env"))
            );
            assert_eq!(
                args.data_dir,
                Some(PathBuf::from("/tmp/test_data_from_env"))
            );

            // Clean up environment variables after the test
            env::remove_var("APP_CACHE_DIR");
            env::remove_var("APP_DATA_DIR");
        }
    }

    #[test]
    fn test_cli_args_override_env_vars() {
        unsafe {
            // Set environment variables
            env::set_var("APP_CACHE_DIR", "/tmp/env_cache");
            env::set_var("APP_DATA_DIR", "/tmp/env_data");

            // Simulate parsing with command-line arguments that should override env vars
            let args = Args::parse_from(vec![
                "cli_app_name",
                "--cache-dir",
                "/tmp/cli_cache",
                "--data-dir",
                "/tmp/cli_data",
            ]);

            // Assert that command-line arguments take precedence
            assert_eq!(args.cache_dir, Some(PathBuf::from("/tmp/cli_cache")));
            assert_eq!(args.data_dir, Some(PathBuf::from("/tmp/cli_data")));

            // Clean up environment variables
            env::remove_var("APP_CACHE_DIR");
            env::remove_var("APP_DATA_DIR");
        }
    }
}
