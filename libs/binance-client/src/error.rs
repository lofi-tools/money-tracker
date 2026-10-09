//! Private API failures omit response bodies, credential values and signed URLs.
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum BinanceError {
    #[snafu(display(
        "{variable} is missing from the process environment; export it before using Binance (.env files are not loaded automatically)"
    ))]
    MissingCredential { variable: &'static str },
    // VarError::NotUnicode contains the credential itself; never retain that source.
    #[snafu(display("{variable} must contain valid Unicode"))]
    InvalidCredential { variable: &'static str },
    #[snafu(display("{variable} is empty"))]
    EmptyCredential { variable: &'static str },
    #[snafu(display("cannot initialize Binance HTTP client"))]
    HttpClient { source: reqwest::Error },
    #[snafu(display("cannot build Binance request"))]
    BuildRequest { source: reqwest::Error },
    #[snafu(display("cannot sign Binance request: system clock is before the Unix epoch"))]
    Clock { source: std::time::SystemTimeError },
    #[snafu(display("cannot encode Binance signature or cache fingerprint"))]
    Hex { source: std::fmt::Error },
    #[snafu(display("Binance {endpoint}: cannot send request"))]
    Request {
        endpoint: String,
        source: reqwest::Error,
    },
    #[snafu(display("Binance {endpoint}: cannot read response"))]
    ReadResponse {
        endpoint: String,
        source: reqwest::Error,
    },
    #[snafu(display("Binance {endpoint}: invalid JSON response"))]
    DecodeResponse {
        endpoint: String,
        source: serde_json::Error,
    },
    #[snafu(display("Binance {endpoint}: HTTP {status}, API code {code:?}"))]
    Http {
        endpoint: String,
        status: u16,
        code: Option<i64>,
    },
    #[snafu(display("Binance {endpoint}: unsuccessful API response, code {code:?}"))]
    Api { endpoint: String, code: Option<i64> },
    #[snafu(display("Binance {endpoint}: {reason}"))]
    InvalidResponse {
        endpoint: String,
        reason: &'static str,
    },
    #[snafu(display("Binance {endpoint}: {reason}"))]
    Pagination {
        endpoint: String,
        reason: &'static str,
    },
    #[snafu(display("cannot read Binance history cache {}", path.display()))]
    CacheRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("invalid Binance history cache {}; remove it explicitly to refetch", path.display()))]
    CacheDecode {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[snafu(display("cannot write Binance history cache {}", path.display()))]
    CacheWrite {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("cannot encode Binance history cache {}", path.display()))]
    CacheEncode {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[snafu(display("cannot serialize Binance request metadata"))]
    EncodeMetadata { source: serde_json::Error },
}

pub(crate) fn validate_credential(
    variable: &'static str,
    value: Result<String, std::env::VarError>,
) -> Result<String, BinanceError> {
    let value = match value {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return MissingCredentialSnafu { variable }.fail(),
        Err(std::env::VarError::NotUnicode(_)) => {
            return InvalidCredentialSnafu { variable }.fail();
        }
    };
    snafu::ensure!(!value.trim().is_empty(), EmptyCredentialSnafu { variable });
    Ok(value)
}

pub(crate) fn http_client() -> Result<reqwest::Client, BinanceError> {
    reqwest::Client::builder()
        .build()
        .map_err(reqwest::Error::without_url)
        .context(HttpClientSnafu)
}

pub(crate) fn credential_with_alias(
    primary: &'static str,
    alias: &'static str,
    read: impl Fn(&str) -> Result<String, std::env::VarError>,
) -> Result<String, BinanceError> {
    match read(primary) {
        Err(std::env::VarError::NotPresent) => match read(alias) {
            Err(std::env::VarError::NotPresent) => {
                MissingCredentialSnafu { variable: primary }.fail()
            }
            value => validate_credential(alias, value),
        },
        value => validate_credential(primary, value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferred_credentials_and_legacy_aliases() {
        let primary = "BINANCE_API_KEY_ID";
        let alias = "BINANCE_API_KEY";
        assert_eq!(
            credential_with_alias(primary, alias, |name| Ok(name.into())).unwrap(),
            primary
        );
        assert_eq!(
            credential_with_alias(primary, alias, |name| {
                if name == primary {
                    Err(std::env::VarError::NotPresent)
                } else {
                    Ok("legacy-key".into())
                }
            })
            .unwrap(),
            "legacy-key"
        );
        assert!(matches!(
            credential_with_alias(primary, alias, |_| Err(std::env::VarError::NotPresent)),
            Err(BinanceError::MissingCredential {
                variable: "BINANCE_API_KEY_ID"
            })
        ));
        // An explicitly configured empty primary must fail rather than using another account.
        assert!(matches!(
            credential_with_alias(primary, alias, |name| Ok(if name == primary {
                ""
            } else {
                "legacy-key"
            }
            .into())),
            Err(BinanceError::EmptyCredential {
                variable: "BINANCE_API_KEY_ID"
            })
        ));
    }
    #[test]
    fn credentials_have_typed_errors_without_secret_values() {
        assert!(matches!(
            validate_credential("BINANCE_API_KEY", Err(std::env::VarError::NotPresent)),
            Err(BinanceError::MissingCredential {
                variable: "BINANCE_API_KEY"
            })
        ));
        assert!(matches!(
            validate_credential("BINANCE_SECRET_KEY", Ok("  ".into())),
            Err(BinanceError::EmptyCredential { .. })
        ));
        let error = validate_credential(
            "BINANCE_SECRET_KEY",
            Err(std::env::VarError::NotUnicode("secret-value".into())),
        )
        .unwrap_err();
        assert!(matches!(error, BinanceError::InvalidCredential { .. }));
        assert!(!format!("{error:?} {error}").contains("secret-value"));
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(
            validate_credential("BINANCE_API_KEY", Ok("valid-key".into())).unwrap(),
            "valid-key"
        );
    }
}
