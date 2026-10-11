//! Shared local configuration. No provider credentials belong in this file.
use crate::dto::ExactDecimal;
use anyhow::{Context, Result, ensure};
use lib_core::{AssetId, PortfolioId, Units};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub const DATABASE_FILENAME: &str = "portfolio.duckdb";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
    #[default]
    Normal,
    Hidden,
    Scaled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub reporting_currency: AssetId,
    pub theme: Theme,
    pub privacy_mode: PrivacyMode,
    pub privacy_scale: ExactDecimal,
    pub dust_threshold: ExactDecimal,
    pub refresh_interval_seconds: u64,
    pub refresh_on_open: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            reporting_currency: AssetId::str("USD"),
            theme: Theme::System,
            privacy_mode: PrivacyMode::Normal,
            privacy_scale: "0.1".parse().unwrap(),
            dust_threshold: "1".parse().unwrap(),
            refresh_interval_seconds: 86400,
            refresh_on_open: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.reporting_currency.0.trim().is_empty(),
            "reporting currency is empty"
        );
        // Compare exact decimal values at their own scale, never through floats.
        let scale = self
            .privacy_scale
            .as_str()
            .split_once('.')
            .map_or(0, |(_, f)| f.len()) as u8;
        let value = Units::from_decimal(self.privacy_scale.as_str(), scale)?;
        ensure!(
            value > Units::default() && value < Units::from_decimal("1", scale)?,
            "privacy scale must be greater than zero and less than one"
        );
        ensure!(
            !self.dust_threshold.as_str().starts_with('-'),
            "dust threshold must be nonnegative"
        );
        ensure!(
            self.refresh_interval_seconds > 0,
            "refresh interval must be positive"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub data_dir: Option<PathBuf>,
    pub active_portfolio: Option<PortfolioId>,
    pub settings: Settings,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            data_dir: None,
            active_portfolio: None,
            settings: Settings::default(),
        }
    }
}
impl AppConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "unsupported configuration version {}",
            self.version
        );
        ensure!(
            self.data_dir
                .as_ref()
                .is_none_or(|p| !p.as_os_str().is_empty()),
            "data directory is empty"
        );
        ensure!(
            self.active_portfolio
                .as_ref()
                .is_none_or(|p| !p.0.trim().is_empty()),
            "active portfolio is empty"
        );
        self.settings.validate()
    }
    pub fn load(path: &Path, required: bool) -> Result<Self> {
        let config: Self = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("invalid config {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => Self::default(),
            Err(e) => return Err(e).with_context(|| format!("reading config {}", path.display())),
        };
        config.validate()?;
        Ok(config)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&serde_json::to_vec_pretty(self)?)?;
        temp.write_all(b"\n")?;
        temp.as_file().sync_all()?;
        temp.persist(path)
            .map_err(|e| e.error)
            .context("installing local configuration")?;
        Ok(())
    }
}

/// Injectable OS locations keep path tests independent of the developer's environment.
#[derive(Clone, Debug)]
pub struct Locations {
    pub default_data_dir: PathBuf,
    pub config_file: PathBuf,
    pub legacy_database: Option<PathBuf>,
}
impl Locations {
    pub fn for_platform(
        home: &Path,
        config_dir: &Path,
        local_data_dir: &Path,
        legacy_data_dir: &Path,
        windows: bool,
    ) -> Self {
        Self {
            default_data_dir: if windows {
                local_data_dir.join("money-tracker")
            } else {
                home.join(".data/money-tracker")
            },
            config_file: config_dir.join("money-tracker/config.json"),
            legacy_database: Some(
                legacy_data_dir
                    .join("money-tracker")
                    .join(DATABASE_FILENAME),
            ),
        }
    }
    pub fn system() -> Result<Self> {
        Ok(Self::for_platform(
            &dirs::home_dir().context("home directory unavailable")?,
            &dirs::config_dir().context("config directory unavailable")?,
            &dirs::data_local_dir().context("local data directory unavailable")?,
            &dirs::data_dir().context("legacy data directory unavailable")?,
            cfg!(windows),
        ))
    }
}
#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub database: PathBuf,
    pub config_file: PathBuf,
    pub config: AppConfig,
    pub legacy_database: Option<PathBuf>,
}
impl ResolvedConfig {
    pub fn resolve(
        locations: Locations,
        config_path: Option<PathBuf>,
        explicit_data_dir: Option<PathBuf>,
        env_data_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let required = config_path.is_some();
        let config_file = config_path.unwrap_or(locations.config_file);
        let config = AppConfig::load(&config_file, required)?;
        let configured = explicit_data_dir
            .or(env_data_dir)
            .or_else(|| config.data_dir.clone());
        let data_dir = match configured {
            Some(p) => {
                ensure!(!p.as_os_str().is_empty(), "data directory is empty");
                p
            }
            None => locations.default_data_dir,
        };
        // Relative paths are deliberately relative to the process working directory;
        // '~' is not expanded. Document this rather than guessing at path syntax.
        let database = data_dir.join(DATABASE_FILENAME);
        let legacy_database = locations.legacy_database.filter(|p| p != &database);
        Ok(Self {
            database,
            config_file,
            config,
            legacy_database,
        })
    }
    pub fn system(
        config_path: Option<PathBuf>,
        explicit_data_dir: Option<PathBuf>,
    ) -> Result<Self> {
        Self::resolve(
            Locations::system()?,
            config_path,
            explicit_data_dir,
            std::env::var_os("APP_DATA_DIR").map(PathBuf::from),
        )
    }
}
