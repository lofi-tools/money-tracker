//! Read-only Kraken Spot account acquisition. No order or transfer endpoints.
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use ring::{digest, hmac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, sync::LazyLock, time::Duration};
use tokio::sync::Mutex;

// Serialize private calls across clients: nonces must arrive in increasing order.
static PRIVATE_GATE: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

fn decimal<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    match Value::deserialize(d)? {
        Value::String(v) => Ok(v),
        Value::Number(v) => Ok(v.to_string()),
        _ => Err(serde::de::Error::custom(
            "expected decimal string or number",
        )),
    }
}
fn zero() -> String {
    "0".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AssetInfo {
    pub altname: String,
    pub decimals: u8,
    pub aclass: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ledger {
    pub refid: String,
    #[serde(deserialize_with = "decimal")]
    pub time: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub subtype: String,
    pub aclass: String,
    pub asset: String,
    pub amount: String,
    pub fee: String,
    pub balance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    #[serde(deserialize_with = "decimal")]
    pub balance: String,
    #[serde(default = "zero", deserialize_with = "decimal")]
    pub credit: String,
    #[serde(default = "zero", deserialize_with = "decimal")]
    pub credit_used: String,
    #[serde(default = "zero", deserialize_with = "decimal")]
    pub hold_trade: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    /// Local account binding; labels/credentials are not economic identity.
    pub account_id: String,
    pub credential_fingerprint: String,
    pub history_cutoff: DateTime<Utc>,
    pub observed_at: DateTime<Utc>,
    pub assets: BTreeMap<String, AssetInfo>,
    pub ledger: BTreeMap<String, Ledger>,
    pub balances: BTreeMap<String, Balance>,
}

pub fn fingerprint(bytes: &[u8]) -> String {
    digest::digest(&digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Atomic completed JSON files; interrupted writes do not masquerade as caches.
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("cache path has no parent")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(file.as_file_mut(), value)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub fn signature(path: &str, nonce: &str, body: &str, secret: &[u8]) -> String {
    let hash = digest::digest(&digest::SHA256, format!("{nonce}{body}").as_bytes());
    let mut message = path.as_bytes().to_vec();
    message.extend_from_slice(hash.as_ref());
    STANDARD.encode(hmac::sign(&hmac::Key::new(hmac::HMAC_SHA512, secret), &message).as_ref())
}

pub struct Client {
    http: reqwest::Client,
    base: String,
    key: String,
    secret: Vec<u8>,
    pace: Duration,
}

impl Client {
    pub fn from_env() -> Result<Self> {
        Self::new(
            "https://api.kraken.com",
            std::env::var("KRAKEN_API_KEY_ID")
                .context("KRAKEN_API_KEY_ID is required for a fresh fetch")?,
            &std::env::var("KRAKEN_API_SECRET")
                .context("KRAKEN_API_SECRET is required for a fresh fetch")?,
        )
    }

    fn new(base: &str, key: String, secret: &str) -> Result<Self> {
        ensure!(!key.is_empty(), "empty Kraken API key");
        let secret = STANDARD
            .decode(secret)
            .context("invalid Kraken secret encoding")?;
        ensure!(!secret.is_empty(), "empty Kraken secret");
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            base: base.into(),
            key,
            secret,
            pace: Duration::from_secs(4),
        })
    }

    fn envelope(value: Value) -> Result<Value> {
        let errors = value
            .get("error")
            .and_then(Value::as_array)
            .context("missing Kraken error array")?;
        // Do not echo untrusted API responses or credentials into logs.
        if !errors.is_empty() {
            bail!("Kraken API rejected request; verify permissions, nonce and endpoint coverage");
        }
        value
            .get("result")
            .cloned()
            .context("missing Kraken result")
    }

    async fn private(&self, endpoint: &str, params: &[(&str, String)]) -> Result<Value> {
        let path = format!("/0/private/{endpoint}");
        let mut last = PRIVATE_GATE.lock().await;
        for attempt in 0..5 {
            tokio::time::sleep(self.pace).await;
            let clock = u64::try_from(Utc::now().timestamp_millis())?;
            *last = clock.max(last.checked_add(1).context("Kraken nonce exhausted")?);
            let nonce = last.to_string();
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            form.append_pair("nonce", &nonce);
            for (name, value) in params {
                form.append_pair(name, value);
            }
            let body = form.finish();
            let response = self
                .http
                .post(format!("{}{path}", self.base))
                .header("API-Key", &self.key)
                .header("API-Sign", signature(&path, &nonce, &body, &self.secret))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("Kraken {endpoint} transport failed"))?;
            let status = response.status();
            let retry = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempt == 4 {
                    bail!("Kraken {endpoint} throttled after five attempts");
                }
                tokio::time::sleep(Duration::from_secs(retry.unwrap_or(4 << attempt))).await;
                continue;
            }
            ensure!(
                status.is_success(),
                "Kraken {endpoint} HTTP {}",
                status.as_u16()
            );
            let value: Value = response.json().await.context("invalid Kraken JSON")?;
            let errors = value
                .get("error")
                .and_then(Value::as_array)
                .context("missing Kraken error array")?;
            if errors.iter().any(|e| {
                e.as_str().is_some_and(|s| {
                    s.starts_with("EAPI:Rate limit exceeded") || s.starts_with("EService:Throttled")
                })
            }) {
                if attempt == 4 {
                    bail!("Kraken {endpoint} API throttled after five attempts");
                }
                let until = errors
                    .iter()
                    .filter_map(|e| {
                        e.as_str()?
                            .strip_prefix("EService:Throttled: ")?
                            .parse::<i64>()
                            .ok()
                    })
                    .max();
                let wait = until
                    .map(|t| (t - Utc::now().timestamp()).max(0) as u64)
                    .unwrap_or(4 << attempt);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            return Self::envelope(value).with_context(|| format!("Kraken {endpoint}"));
        }
        unreachable!()
    }

    pub async fn fetch_snapshot(
        &self,
        account_id: &str,
        cutoff: DateTime<Utc>,
        cache: &Path,
    ) -> Result<Snapshot> {
        ensure!(
            cutoff.timestamp_subsec_nanos() == 0,
            "Kraken history cutoff must use whole UTC seconds"
        );
        let response = self
            .http
            .get(format!("{}/0/public/Assets", self.base))
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Kraken Assets transport failed"))?;
        ensure!(response.status().is_success(), "Kraken Assets HTTP failure");
        let assets = serde_json::from_value(Self::envelope(response.json().await?)?)?;
        let mut ledger = BTreeMap::new();
        let mut offset = 0usize;
        let mut expected = None;
        loop {
            let params = [
                ("type", "all".into()),
                ("start", "0".into()),
                ("end", cutoff.timestamp().to_string()),
                ("ofs", offset.to_string()),
            ];
            let cache_key = fingerprint(&serde_json::to_vec(&(&self.key, "Ledgers", &params))?);
            let path = cache.join(format!("{cache_key}.json"));
            let result: Value = match std::fs::read(&path) {
                Ok(bytes) => {
                    serde_json::from_slice(&bytes).context("invalid cached Kraken ledger page")?
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("Fetching Kraken ledger offset {offset}");
                    let value = self.private("Ledgers", &params).await?;
                    // Validate before marking a response reusable.
                    let _: Page = serde_json::from_value(value.clone())
                        .context("invalid Kraken ledger page")?;
                    write_json(&path, &value)?;
                    value
                }
                Err(e) => return Err(e.into()),
            };
            let page: Page = serde_json::from_value(result)?;
            append_page(&mut ledger, &mut offset, &mut expected, page)?;
            if offset == expected.unwrap() {
                break;
            }
        }
        let balances = serde_json::from_value(self.private("BalanceEx", &[]).await?)?;
        Ok(Snapshot {
            version: 1,
            account_id: account_id.into(),
            credential_fingerprint: fingerprint(self.key.as_bytes()),
            history_cutoff: cutoff,
            observed_at: Utc::now(),
            assets,
            ledger,
            balances,
        })
    }
}

#[derive(Deserialize)]
struct Page {
    ledger: BTreeMap<String, Ledger>,
    count: usize,
}

fn append_page(
    ledger: &mut BTreeMap<String, Ledger>,
    offset: &mut usize,
    expected: &mut Option<usize>,
    page: Page,
) -> Result<()> {
    ensure!(
        expected.is_none_or(|n| n == page.count),
        "Kraken history count changed during pagination"
    );
    *expected = Some(page.count);
    ensure!(page.ledger.len() <= 50, "oversized Kraken ledger page");
    ensure!(
        !page.ledger.is_empty() || *offset == page.count,
        "Kraken ledger pagination stopped early"
    );
    for (id, row) in page.ledger {
        ensure!(
            !id.is_empty() && !ledger.contains_key(&id),
            "empty/repeated Kraken ledger ID during pagination"
        );
        ledger.insert(id, row);
        *offset += 1;
    }
    ensure!(
        *offset <= page.count,
        "Kraken ledger count is smaller than returned history"
    );
    Ok(())
}

/// A cache is bound to a local account and a key fingerprint, never a secret.
/// Complete snapshots replay without credentials. A pending cutoff resumes pages.
pub async fn cached_snapshot(
    path: &Path,
    account: &str,
    offline: bool,
    refresh: bool,
) -> Result<Snapshot> {
    ensure!(!(offline && refresh), "cannot refresh Kraken while offline");
    if !refresh {
        match std::fs::read(path) {
            Ok(bytes) => {
                let snapshot: Snapshot =
                    serde_json::from_slice(&bytes).context("invalid Kraken snapshot cache")?;
                ensure!(
                    snapshot.account_id == account,
                    "Kraken snapshot belongs to a different local account"
                );
                eprintln!("Reusing frozen Kraken snapshot {}", path.display());
                return Ok(snapshot);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    ensure!(
        !offline,
        "Kraken snapshot cache missing; fetch once before offline replay"
    );
    let client = Client::from_env()?;
    let pending_path = path.with_extension("pending.json");
    let key_hash = fingerprint(client.key.as_bytes());
    let cutoff = if !refresh && pending_path.exists() {
        let (bound_account, bound_key, cutoff): (String, String, DateTime<Utc>) =
            serde_json::from_slice(&std::fs::read(&pending_path)?)?;
        ensure!(
            bound_account == account && bound_key == key_hash,
            "Kraken pending cache identity conflict; use an explicit refresh with the correct account/key"
        );
        cutoff
    } else {
        DateTime::from_timestamp(Utc::now().timestamp(), 0).context("invalid Kraken cutoff")?
    };
    write_json(&pending_path, &(account, key_hash, cutoff))?;
    let snapshot = client
        .fetch_snapshot(account, cutoff, &path.with_extension("pages"))
        .await?;
    write_json(path, &snapshot)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn server(
        responses: Vec<(u16, Value)>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut requests = vec![];
            for (status, response) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = vec![];
                let header_end;
                loop {
                    let mut buffer = [0; 4096];
                    let n = stream.read(&mut buffer).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = index + 4;
                        break;
                    }
                }
                let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + length {
                    let mut buffer = [0; 4096];
                    let n = stream.read(&mut buffer).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let body = serde_json::to_string(&response).unwrap();
                stream.write_all(format!("HTTP/1.1 {status} test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nRetry-After: 0\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
            requests
        });
        (base, task)
    }

    #[tokio::test]
    async fn signed_paginated_acquisition_caches_pages_but_fetches_balances_independently()
    -> Result<()> {
        let envelope = |result: Value| serde_json::json!({"error":[],"result":result});
        let assets = envelope(
            serde_json::json!({"XXBT":{"altname":"XBT","decimals":8,"aclass":"currency"}}),
        );
        let rows = |start: usize, end: usize| {
            (start..end).map(|i| (format!("L{i}"),serde_json::json!({"refid":"r","time":1704067200.1234,"type":"deposit","aclass":"currency","asset":"XXBT","amount":"1","fee":"0","balance":(i+1).to_string()}))).collect::<serde_json::Map<_,_>>()
        };
        let balances = envelope(serde_json::json!({"XXBT":{"balance":"51","hold_trade":"2"}}));
        let (base, task) = server(vec![
            (200, assets.clone()),
            (
                200,
                envelope(serde_json::json!({"ledger":rows(0,50),"count":51})),
            ),
            (
                200,
                envelope(serde_json::json!({"ledger":rows(50,51),"count":51})),
            ),
            (200, balances.clone()),
            (200, assets),
            (200, balances),
        ])
        .await;
        let mut client = Client::new(&base, "test-key".into(), &STANDARD.encode(b"test-secret"))?;
        client.pace = Duration::ZERO;
        let dir = tempfile::tempdir()?;
        let cutoff = "2024-01-02T00:00:00Z".parse()?;
        let first = client.fetch_snapshot("local", cutoff, dir.path()).await?;
        let second = client.fetch_snapshot("local", cutoff, dir.path()).await?;
        assert_eq!(first.ledger.len(), 51);
        assert_eq!(first.ledger, second.ledger);
        assert_eq!(first.balances["XXBT"].balance, "51");
        let requests = task.await?;
        assert!(requests[1].ends_with("type=all&start=0&end=1704153600&ofs=0"));
        assert!(requests[2].ends_with("type=all&start=0&end=1704153600&ofs=50"));
        assert!(requests[3].starts_with("POST /0/private/BalanceEx "));
        assert!(requests[5].starts_with("POST /0/private/BalanceEx "));
        let mut last = 0u64;
        for request in requests.iter().filter(|r| r.starts_with("POST")) {
            let (headers, body) = request.split_once("\r\n\r\n").unwrap();
            let path = headers.split_whitespace().nth(1).unwrap();
            let nonce = url::form_urlencoded::parse(body.as_bytes())
                .find(|(name, _)| name == "nonce")
                .unwrap()
                .1
                .into_owned();
            let n: u64 = nonce.parse()?;
            assert!(n > last);
            last = n;
            let expected = signature(path, &nonce, body, b"test-secret");
            assert!(
                headers
                    .lines()
                    .any(|line| line.to_ascii_lowercase().starts_with("api-sign: ")
                        && line.split_once(": ").unwrap().1 == expected)
            );
            assert!(!body.contains("test-secret"));
        }
        Ok(())
    }

    #[tokio::test]
    async fn http_throttle_retries_and_offline_complete_snapshot_needs_no_client() -> Result<()> {
        let (base, task) = server(vec![
            (429, serde_json::json!({})),
            (200, serde_json::json!({"error":[],"result":{}})),
        ])
        .await;
        let mut client = Client::new(&base, "test-key".into(), &STANDARD.encode(b"test-secret"))?;
        client.pace = Duration::ZERO;
        client.private("BalanceEx", &[]).await?;
        assert_eq!(task.await?.len(), 2);
        let snapshot = Snapshot {
            version: 1,
            account_id: "a".into(),
            credential_fingerprint: "fake".into(),
            history_cutoff: Utc::now(),
            observed_at: Utc::now(),
            assets: BTreeMap::new(),
            ledger: BTreeMap::new(),
            balances: BTreeMap::new(),
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("snapshot.json");
        write_json(&path, &snapshot)?;
        assert_eq!(
            cached_snapshot(&path, "a", true, false).await?.account_id,
            "a"
        );
        assert!(cached_snapshot(&path, "b", true, false).await.is_err());
        assert!(cached_snapshot(&path, "a", true, true).await.is_err());
        std::fs::write(&path, b"broken")?;
        assert!(cached_snapshot(&path, "a", true, false).await.is_err());
        Ok(())
    }
    #[test]
    fn signature_matches_official_vector() -> Result<()> {
        let key = STANDARD.decode("kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==")?;
        assert_eq!(
            signature(
                "/0/private/AddOrder",
                "1616492376594",
                "nonce=1616492376594&ordertype=limit&pair=XBTUSD&price=37500&type=buy&volume=1.25",
                &key
            ),
            "4/dpxb3iT4tp/ZCVEwSnEsLxx0bqyhLpdfOpc6fn7OR8+UClSV5n9E6aSS8MPtnRfp32bAb0nmbRn6H8ndwLUQ=="
        );
        Ok(())
    }
    #[test]
    fn exact_numeric_balances_and_api_errors() -> Result<()> {
        let b: Balance =
            serde_json::from_str(r#"{"balance":12345678901234567890.123456789,"hold_trade":"1"}"#)?;
        assert_eq!(b.balance, "12345678901234567890.123456789");
        assert!(
            Client::envelope(serde_json::json!({"error":["private secret response"],"result":{}}))
                .unwrap_err()
                .to_string()
                .find("secret")
                .is_none()
        );
        assert!(Client::envelope(serde_json::json!({"result":{}})).is_err());
        Ok(())
    }
    #[test]
    fn pagination_rejects_repeats_truncation_and_count_changes() -> Result<()> {
        let row: Ledger = serde_json::from_value(
            serde_json::json!({"refid":"x","time":1,"type":"deposit","aclass":"currency","asset":"XXBT","amount":"1","fee":"0","balance":"1"}),
        )?;
        let mut rows = BTreeMap::new();
        let mut offset = 0;
        let mut count = None;
        append_page(
            &mut rows,
            &mut offset,
            &mut count,
            Page {
                ledger: BTreeMap::from([("id".into(), row.clone())]),
                count: 2,
            },
        )?;
        assert!(
            append_page(
                &mut rows,
                &mut offset,
                &mut count,
                Page {
                    ledger: BTreeMap::from([("id".into(), row.clone())]),
                    count: 2
                }
            )
            .is_err()
        );
        assert!(
            append_page(
                &mut rows,
                &mut offset,
                &mut count,
                Page {
                    ledger: BTreeMap::new(),
                    count: 2
                }
            )
            .is_err()
        );
        assert!(
            append_page(
                &mut rows,
                &mut offset,
                &mut count,
                Page {
                    ledger: BTreeMap::from([("other".into(), row)]),
                    count: 3
                }
            )
            .is_err()
        );
        Ok(())
    }
}
