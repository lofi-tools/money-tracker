//! Raw private account snapshots. Amounts remain decimal strings until ledger mapping.
use crate::{BinanceClient, BinanceError, error::*, signing::RequestSigner};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};
use std::collections::{BTreeMap, HashSet};

pub const BINANCE_LAUNCH_MS: i64 = 1_500_249_600_000; // 2017-07-17 UTC
const DAY: i64 = 86_400_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSnapshot {
    pub version: u32,
    pub from: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    pub symbols: Vec<Symbol>,
    /// Endpoint name (and any required discriminator) -> complete raw records.
    pub history: BTreeMap<String, Vec<Value>>,
    pub balances: BTreeMap<String, Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
}

#[derive(Clone, Copy)]
enum Paging {
    Offset,
    Page,
    FiatPage,
    Split,
}

impl BinanceClient {
    /// Constructor for callers with an explicit credential source and mock HTTP servers.
    pub fn with_credentials(
        base_url: impl Into<String>,
        api_key: String,
        api_secret: String,
    ) -> Result<Self, BinanceError> {
        let api_key = validate_credential("BINANCE_API_KEY_ID", Ok(api_key))?;
        let api_secret = validate_credential("BINANCE_API_KEY_SECRET", Ok(api_secret))?;
        Ok(Self {
            http_client: http_client()?,
            base_url: base_url.into(),
            api_key,
            api_secret,
            history_cache: None,
            request_progress: None,
        })
    }

    /// Report completed requests using only the endpoint and cache-hit status.
    pub fn with_request_progress(
        mut self,
        on_progress: impl Fn(&str, bool) + Send + Sync + 'static,
    ) -> Self {
        self.request_progress = Some(Box::new(on_progress));
        self
    }

    /// Cache successful historical requests so an interrupted backfill can resume.
    /// Current balances always come from fresh API calls. Keys contain no secrets.
    pub fn with_history_cache(mut self, directory: impl Into<std::path::PathBuf>) -> Self {
        let fingerprint = hmac_sha256::HMAC::mac(&self.api_key, "binance-cache-v1");
        self.history_cache = Some(
            directory.into().join(
                fingerprint
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            ),
        );
        self
    }

    // Do not include signed URLs, API keys, or account response bodies in errors.
    pub(crate) async fn account_request(
        &self,
        path: &str,
        params: &[(String, String)],
        post: bool,
    ) -> Result<Value, BinanceError> {
        let historical = path.contains("/history/")
            || matches!(
                path,
                "/api/v3/myTrades"
                    | "/sapi/v1/capital/deposit/hisrec"
                    | "/sapi/v1/capital/withdraw/history"
                    | "/sapi/v1/asset/assetDividend"
                    | "/sapi/v1/convert/tradeFlow"
                    | "/sapi/v1/asset/dribblet"
                    | "/sapi/v1/asset/transfer"
                    | "/sapi/v1/fiat/orders"
                    | "/sapi/v1/fiat/payments"
            );
        let cache = if let Some(dir) = self.history_cache.as_ref().filter(|_| historical && !post) {
            let key = serde_json::to_string(&(path, params)).context(EncodeMetadataSnafu)?;
            let hash = hmac_sha256::HMAC::mac(key, "binance-request-v1");
            let name: String = hash.iter().map(|b| format!("{b:02x}")).collect();
            Some(dir.join(format!("{name}.json")))
        } else {
            None
        };
        let path_endpoint = path;
        if let Some(path) = &cache {
            match std::fs::read(path) {
                Ok(bytes) => {
                    let value =
                        serde_json::from_slice(&bytes).context(CacheDecodeSnafu { path })?;
                    if let Some(on_progress) = &self.request_progress {
                        on_progress(path_endpoint, true);
                    }
                    return Ok(value);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(source) => {
                    return Err(BinanceError::CacheRead {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
        let origin = self
            .base_url
            .trim_end_matches("/sapi/v1")
            .trim_end_matches('/');
        let url = format!("{origin}{path}");
        for attempt in 0..4 {
            let builder = if post {
                self.http_client.post(&url)
            } else {
                self.http_client.get(&url)
            };
            let response = builder
                .query(params)
                .query(&[("recvWindow", "10000")])
                .timeout(std::time::Duration::from_secs(30))
                .sign(self)?
                .send()
                .await
                .map_err(reqwest::Error::without_url)
                .context(RequestSnafu { endpoint: path })?;
            if response.status().as_u16() == 429 && attempt < 3 {
                let seconds = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(30)
                    .min(60);
                tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
                continue;
            }
            let value = decode_response(response, path).await?;
            if let Some(path) = &cache {
                // Generated cache paths always have the configured directory as parent.
                let directory = path.parent().context(InvalidResponseSnafu {
                    endpoint: "cache",
                    reason: "cache path has no parent",
                })?;
                std::fs::create_dir_all(directory).context(CacheWriteSnafu { path })?;
                let mut file =
                    tempfile::NamedTempFile::new_in(directory).context(CacheWriteSnafu { path })?;
                serde_json::to_writer(file.as_file_mut(), &value)
                    .context(CacheEncodeSnafu { path })?;
                file.as_file()
                    .sync_all()
                    .context(CacheWriteSnafu { path })?;
                file.persist(path)
                    .map_err(|e| e.error)
                    .context(CacheWriteSnafu { path })?;
            }
            if let Some(on_progress) = &self.request_progress {
                on_progress(path, false);
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            return Ok(value);
        }
        PaginationSnafu {
            endpoint: path,
            reason: "rate-limit retries exhausted",
        }
        .fail()
    }

    async fn pages(
        &self,
        path: &str,
        key: &str,
        params: &[(String, String)],
        limit: usize,
        paging: Paging,
    ) -> Result<Vec<Value>, BinanceError> {
        let mut records = Vec::new();
        let mut seen_pages = HashSet::new();
        let mut page = 1;
        loop {
            let mut query = params.to_vec();
            let limit_name = match paging {
                Paging::Page => "size",
                Paging::FiatPage => "rows",
                _ => "limit",
            };
            query.push((limit_name.into(), limit.to_string()));
            match paging {
                Paging::Offset => query.push(("offset".into(), records.len().to_string())),
                Paging::Page => query.push(("current".into(), page.to_string())),
                Paging::FiatPage => query.push(("page".into(), page.to_string())),
                Paging::Split => (),
            }
            let response = self.account_request(path, &query, false).await?;
            // Universal transfer history omits rows entirely when total is zero.
            // Restrict this exception to that endpoint and an explicit empty total;
            // otherwise missing records must remain an error, never lost history.
            if path == "/sapi/v1/asset/transfer"
                && key == "rows"
                && response.get("rows").is_none()
                && response.get("total").is_some_and(|v| v.as_u64() == Some(0))
                && response.get("moreData").and_then(Value::as_bool) != Some(true)
                && records.is_empty()
            {
                break;
            }
            let rows = if key.is_empty() {
                &response
            } else {
                &response[key]
            };
            let rows = rows.as_array().context(InvalidResponseSnafu {
                endpoint: path,
                reason: "missing records array",
            })?;
            ensure!(
                rows.is_empty()
                    || seen_pages.insert(serde_json::to_string(rows).context(EncodeMetadataSnafu)?),
                PaginationSnafu {
                    endpoint: path,
                    reason: "pagination did not advance"
                }
            );
            let count = rows.len();
            records.extend(rows.iter().cloned());
            let more = response
                .get("moreData")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let total = response.get("total").and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            });
            if matches!(paging, Paging::Split) {
                ensure!(
                    !(more || total.is_some_and(|n| n > count as u64)) || count >= limit,
                    PaginationSnafu {
                        endpoint: path,
                        reason: "response is truncated below requested limit"
                    }
                );
            }
            if matches!(paging, Paging::Split)
                || (!more && count < limit && total.is_none_or(|n| records.len() as u64 >= n))
            {
                break;
            }
            ensure!(
                count > 0,
                PaginationSnafu {
                    endpoint: path,
                    reason: "incomplete pagination"
                }
            );
            page += 1;
        }
        Ok(records)
    }

    async fn history(
        &self,
        path: &str,
        key: &str,
        extra: &[(String, String)],
        from: i64,
        to: i64,
        days: i64,
        limit: usize,
        paging: Paging,
    ) -> Result<Vec<Value>, BinanceError> {
        let mut pending = Vec::new();
        let mut start = from;
        while start <= to {
            let end = (start + days * DAY - 1).min(to);
            pending.push((start, end));
            start = end + 1;
        }
        let mut records = Vec::new();
        while let Some((start, end)) = pending.pop() {
            let mut params = extra.to_vec();
            let (start_name, end_name) = if matches!(paging, Paging::FiatPage) {
                ("beginTime", "endTime")
            } else {
                ("startTime", "endTime")
            };
            params.extend([
                (start_name.into(), start.to_string()),
                (end_name.into(), end.to_string()),
            ]);
            let rows = self.pages(path, key, &params, limit, paging).await?;
            if matches!(paging, Paging::Split) && rows.len() >= limit {
                ensure!(
                    start < end,
                    PaginationSnafu {
                        endpoint: path,
                        reason: "too many records at one millisecond; history is truncated"
                    }
                );
                let midpoint = start + (end - start) / 2;
                pending.extend([(start, midpoint), (midpoint + 1, end)]);
            } else {
                records.extend(rows);
            }
        }
        Ok(records)
    }

    /// All API-accessible Spot, Funding and Simple Earn history. Binance may retain
    /// less history than the account's lifetime; reconciliation must detect gaps.
    /// `additional_symbols` supplies delisted pairs absent from exchangeInfo.
    pub async fn fetch_account_snapshot(
        &self,
        additional_symbols: &[Symbol],
    ) -> Result<AccountSnapshot, BinanceError> {
        let from =
            DateTime::from_timestamp_millis(BINANCE_LAUNCH_MS).context(InvalidResponseSnafu {
                endpoint: "history",
                reason: "launch timestamp out of range",
            })?;
        let cutoff = Utc::now();
        let origin = self
            .base_url
            .trim_end_matches("/sapi/v1")
            .trim_end_matches('/');
        let response = self
            .http_client
            .get(format!("{origin}/api/v3/exchangeInfo"))
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context(RequestSnafu {
                endpoint: "/api/v3/exchangeInfo",
            })?;
        let info = decode_response(response, "/api/v3/exchangeInfo").await?;
        let mut symbols: Vec<Symbol> =
            serde_json::from_value(info["symbols"].clone()).context(DecodeResponseSnafu {
                endpoint: "/api/v3/exchangeInfo",
            })?;
        for symbol in additional_symbols {
            if !symbols.iter().any(|s| s.symbol == symbol.symbol) {
                symbols.push(symbol.clone());
            }
        }
        let mut history = BTreeMap::new();
        for (name, path, key, days, limit, paging) in [
            (
                "deposits",
                "/sapi/v1/capital/deposit/hisrec",
                "",
                89,
                1000,
                Paging::Offset,
            ),
            (
                "withdrawals",
                "/sapi/v1/capital/withdraw/history",
                "",
                89,
                1000,
                Paging::Offset,
            ),
            (
                "dividends",
                "/sapi/v1/asset/assetDividend",
                "rows",
                179,
                500,
                Paging::Split,
            ),
            (
                "convert",
                "/sapi/v1/convert/tradeFlow",
                "list",
                30,
                1000,
                Paging::Split,
            ),
            (
                "dust",
                "/sapi/v1/asset/dribblet",
                "userAssetDribblets",
                30,
                100,
                Paging::Split,
            ),
        ] {
            history.insert(
                name.into(),
                self.history(
                    path,
                    key,
                    &[],
                    BINANCE_LAUNCH_MS,
                    cutoff.timestamp_millis(),
                    days,
                    limit,
                    paging,
                )
                .await?,
            );
        }
        for kind in ["flexible", "locked"] {
            for record in ["subscriptionRecord", "redemptionRecord", "rewardsRecord"] {
                let path = format!("/sapi/v1/simple-earn/{kind}/history/{record}");
                let reward_types: &[&str] = if kind == "flexible" && record == "rewardsRecord" {
                    &["BONUS", "REALTIME", "REWARDS"]
                } else {
                    &[""]
                };
                for reward_type in reward_types {
                    let extra = if reward_type.is_empty() {
                        vec![]
                    } else {
                        vec![("type".into(), reward_type.to_string())]
                    };
                    history.insert(
                        format!("{kind}/{record}/{reward_type}"),
                        self.history(
                            &path,
                            "rows",
                            &extra,
                            BINANCE_LAUNCH_MS,
                            cutoff.timestamp_millis(),
                            30,
                            100,
                            Paging::Page,
                        )
                        .await?,
                    );
                }
            }
        }
        // This endpoint only exposes the last six calendar months. Requests
        // before that boundary are rejected rather than returning older history.
        let transfer_from = cutoff
            .checked_sub_months(chrono::Months::new(6))
            .context(InvalidResponseSnafu {
                endpoint: "/sapi/v1/asset/transfer",
                reason: "transfer history boundary overflow",
            })?
            .timestamp_millis()
            .max(BINANCE_LAUNCH_MS);
        for transfer in ["MAIN_FUNDING", "FUNDING_MAIN"] {
            history.insert(
                format!("transfer/{transfer}"),
                self.history(
                    "/sapi/v1/asset/transfer",
                    "rows",
                    &[("type".into(), transfer.into())],
                    transfer_from,
                    cutoff.timestamp_millis(),
                    30,
                    100,
                    Paging::Page,
                )
                .await?,
            );
        }
        for endpoint in ["orders", "payments"] {
            for transaction_type in ["0", "1"] {
                history.insert(
                    format!("fiat/{endpoint}/{transaction_type}"),
                    self.history(
                        &format!("/sapi/v1/fiat/{endpoint}"),
                        "data",
                        &[("transactionType".into(), transaction_type.into())],
                        BINANCE_LAUNCH_MS,
                        cutoff.timestamp_millis(),
                        30,
                        500,
                        Paging::FiatPage,
                    )
                    .await?,
                );
            }
        }
        for symbol in &symbols {
            let mut trades = Vec::new();
            let mut from_id = 0;
            loop {
                let response = self
                    .account_request(
                        "/api/v3/myTrades",
                        &[
                            ("symbol".into(), symbol.symbol.clone()),
                            ("fromId".into(), from_id.to_string()),
                            ("limit".into(), "1000".into()),
                        ],
                        false,
                    )
                    .await?;
                let rows = response.as_array().context(InvalidResponseSnafu {
                    endpoint: "/api/v3/myTrades",
                    reason: "expected trades array",
                })?;
                let count = rows.len();
                if let Some(last) = rows.last() {
                    let next = last["id"]
                        .as_u64()
                        .context(InvalidResponseSnafu {
                            endpoint: "/api/v3/myTrades",
                            reason: "missing trade id",
                        })?
                        .checked_add(1)
                        .context(InvalidResponseSnafu {
                            endpoint: "/api/v3/myTrades",
                            reason: "trade id overflow",
                        })?;
                    ensure!(
                        next > from_id,
                        PaginationSnafu {
                            endpoint: "/api/v3/myTrades",
                            reason: "pagination did not advance"
                        }
                    );
                    from_id = next;
                }
                trades.extend(
                    rows.iter()
                        .filter(|r| {
                            r["time"]
                                .as_i64()
                                .is_some_and(|t| t <= cutoff.timestamp_millis())
                        })
                        .cloned(),
                );
                if count < 1000 {
                    break;
                }
            }
            if !trades.is_empty() {
                history.insert(format!("trades/{}", symbol.symbol), trades);
            }
        }
        // Fetch these after history; reconcile only a quiet account. Concurrent activity
        // during collection is intentionally visible as a mismatch.
        let mut balances = BTreeMap::new();
        let spot = self.account_request("/api/v3/account", &[], false).await?;
        balances.insert(
            "spot".into(),
            spot["balances"]
                .as_array()
                .context(InvalidResponseSnafu {
                    endpoint: "/api/v3/account",
                    reason: "missing spot balances",
                })?
                .clone(),
        );
        let funding = self
            .account_request("/sapi/v1/asset/get-funding-asset", &[], true)
            .await?;
        balances.insert(
            "funding".into(),
            funding
                .as_array()
                .context(InvalidResponseSnafu {
                    endpoint: "/sapi/v1/asset/get-funding-asset",
                    reason: "missing funding balances",
                })?
                .clone(),
        );
        for kind in ["flexible", "locked"] {
            balances.insert(
                kind.into(),
                self.pages(
                    &format!("/sapi/v1/simple-earn/{kind}/position"),
                    "rows",
                    &[],
                    100,
                    Paging::Page,
                )
                .await?,
            );
        }
        Ok(AccountSnapshot {
            version: 1,
            from,
            fetched_at: cutoff,
            symbols,
            history,
            balances,
        })
    }
}

async fn decode_response(
    response: reqwest::Response,
    endpoint: &str,
) -> Result<Value, BinanceError> {
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(reqwest::Error::without_url)
        .context(ReadResponseSnafu { endpoint })?;
    let parsed = serde_json::from_slice::<Value>(&bytes);
    let code = parsed
        .as_ref()
        .ok()
        .and_then(|v| v.get("code"))
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        });
    ensure!(
        status.is_success(),
        HttpSnafu {
            endpoint,
            status: status.as_u16(),
            code
        }
    );
    let value = parsed.context(DecodeResponseSnafu { endpoint })?;
    ensure!(
        code.is_none_or(|c| c == 0) && value.get("success").and_then(Value::as_bool) != Some(false),
        ApiSnafu { endpoint, code }
    );
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };

    async fn server(responses: Vec<(u16, Value)>) -> (BinanceClient, JoinHandle<Vec<String>>) {
        raw_server(
            responses
                .into_iter()
                .map(|(status, body)| (status, body.to_string()))
                .collect(),
        )
        .await
    }

    async fn raw_server(responses: Vec<(u16, String)>) -> (BinanceClient, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut chunk = [0; 4096];
                    let count = stream.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&chunk[..count]);
                    if bytes.windows(4).any(|b| b == b"\r\n\r\n") {
                        break;
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                stream.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
            requests
        });
        (
            BinanceClient::with_credentials(
                format!("http://{address}"),
                "private-test-key".into(),
                "private-test-secret".into(),
            )
            .unwrap(),
            task,
        )
    }

    #[tokio::test]
    async fn pages_offset_and_page_with_string_total() -> anyhow::Result<()> {
        let (client, task) = server(vec![
            (200, serde_json::json!([{"id":1},{"id":2}])),
            (200, serde_json::json!([{"id":3}])),
        ])
        .await;
        let rows = client
            .pages("/deposits", "", &[], 2, Paging::Offset)
            .await?;
        assert_eq!(rows.len(), 3);
        let requests = task.await?;
        assert!(requests[0].contains("offset=0"));
        assert!(requests[1].contains("offset=2"));
        assert!(requests[0].contains("signature="));
        assert!(
            requests[0]
                .to_lowercase()
                .contains("x-mbx-apikey: private-test-key")
        );
        let (client, task) = server(vec![
            (200, serde_json::json!({"rows":[{"id":1}],"total":"2"})),
            (200, serde_json::json!({"rows":[{"id":2}],"total":"2"})),
        ])
        .await;
        assert_eq!(
            client
                .pages("/earn", "rows", &[], 100, Paging::Page)
                .await?
                .len(),
            2
        );
        let requests = task.await?;
        assert!(requests[1].contains("current=2"));
        assert!(requests[1].contains("size=100"));
        Ok(())
    }

    #[tokio::test]
    async fn transfer_empty_response_without_rows_is_cached_and_replayed() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let (client, task) = server(vec![(200, serde_json::json!({"total":0}))]).await;
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = events.clone();
        let client =
            client
                .with_history_cache(dir.path())
                .with_request_progress(move |endpoint, cached| {
                    captured.lock().unwrap().push((endpoint.to_owned(), cached));
                });
        for _ in 0..2 {
            assert!(
                client
                    .pages("/sapi/v1/asset/transfer", "rows", &[], 100, Paging::Page)
                    .await?
                    .is_empty()
            );
        }
        let requests = task.await?;
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains("size=100"));
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                ("/sapi/v1/asset/transfer".to_owned(), false),
                ("/sapi/v1/asset/transfer".to_owned(), true),
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn transfer_missing_rows_does_not_hide_nonempty_or_malformed_history()
    -> anyhow::Result<()> {
        for response in [
            serde_json::json!({"total":1}),
            serde_json::json!({}),
            serde_json::json!({"total":0,"rows":null}),
            serde_json::json!({"total":0,"moreData":true}),
        ] {
            let (client, task) = server(vec![(200, response)]).await;
            assert!(matches!(
                client
                    .pages("/sapi/v1/asset/transfer", "rows", &[], 100, Paging::Page)
                    .await,
                Err(BinanceError::InvalidResponse { .. })
            ));
            task.await?;
        }
        // The omission exception must not apply to other endpoints.
        let (client, task) = server(vec![(200, serde_json::json!({"total":0}))]).await;
        assert!(
            client
                .pages("/earn", "rows", &[], 100, Paging::Page)
                .await
                .is_err()
        );
        task.await?;
        Ok(())
    }

    #[tokio::test]
    async fn splits_saturated_history_without_overlapping_boundaries() -> anyhow::Result<()> {
        let (client, task) = server(vec![
            (
                200,
                serde_json::json!({"rows":[{"id":1},{"id":2}],"total":2}),
            ),
            (200, serde_json::json!({"rows":[{"id":2}],"total":1})),
            (200, serde_json::json!({"rows":[{"id":1}],"total":1})),
        ])
        .await;
        let rows = client
            .history("/dividends", "rows", &[], 0, 9, 1, 2, Paging::Split)
            .await?;
        assert_eq!(rows.len(), 2);
        let requests = task.await?;
        assert!(requests[0].contains("startTime=0&endTime=9"));
        assert!(requests[1].contains("startTime=5&endTime=9"));
        assert!(requests[2].contains("startTime=0&endTime=4"));
        Ok(())
    }

    #[tokio::test]
    async fn detects_repeated_pages_and_sanitizes_api_errors() -> anyhow::Result<()> {
        let page = serde_json::json!({"rows":[{"id":1}], "total":2});
        let (client, task) = server(vec![(200, page.clone()), (200, page)]).await;
        assert!(
            client
                .pages("/earn", "rows", &[], 1, Paging::Page)
                .await
                .unwrap_err()
                .to_string()
                .contains("did not advance")
        );
        task.await?;
        let (client, task) = server(vec![(
            401,
            serde_json::json!({"code":-2015,"msg":"private-test-key"}),
        )])
        .await;
        let error = client
            .account_request("/account", &[], false)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("-2015"));
        assert!(!error.contains("private-test-key"));
        assert!(!error.contains("signature="));
        task.await?;
        Ok(())
    }

    #[tokio::test]
    async fn history_cache_replays_but_balances_are_fetched_fresh() -> anyhow::Result<()> {
        let (client, task) = server(vec![
            (200, serde_json::json!([{"id":1}])),
            (200, serde_json::json!({"balances":[{"free":"1"}]})),
            (200, serde_json::json!({"balances":[{"free":"2"}]})),
        ])
        .await;
        let directory = tempfile::tempdir()?;
        let client = client.with_history_cache(directory.path());
        let first = client
            .account_request("/sapi/v1/capital/deposit/hisrec", &[], false)
            .await?;
        assert_eq!(
            first,
            client
                .account_request("/sapi/v1/capital/deposit/hisrec", &[], false)
                .await?
        );
        assert_ne!(
            client
                .account_request("/api/v3/account", &[], false)
                .await?,
            client
                .account_request("/api/v3/account", &[], false)
                .await?
        );
        assert_eq!(task.await?.len(), 3);
        Ok(())
    }
    #[tokio::test]
    async fn response_errors_are_typed_and_preserve_safe_sources() -> anyhow::Result<()> {
        use std::error::Error;
        let (client, task) = raw_server(vec![
            (503, "private response text".into()),
            (200, "not-json".into()),
            (200, r#"{"code":-2015,"msg":"private-test-key"}"#.into()),
            (200, r#"{"success":false}"#.into()),
        ])
        .await;
        let error = client
            .account_request("/account", &[], false)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            BinanceError::Http {
                status: 503,
                code: None,
                ..
            }
        ));
        assert!(!format!("{error:?}").contains("private response text"));
        let error = client
            .account_request("/account", &[], false)
            .await
            .unwrap_err();
        assert!(matches!(error, BinanceError::DecodeResponse { .. }));
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<serde_json::Error>()
                .is_some()
        );
        let error = client
            .account_request("/account", &[], false)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            BinanceError::Api {
                code: Some(-2015),
                ..
            }
        ));
        assert!(!format!("{error:?}").contains("private-test-key"));
        assert!(matches!(
            client
                .account_request("/account", &[], false)
                .await
                .unwrap_err(),
            BinanceError::Api { code: None, .. }
        ));
        task.await?;
        Ok(())
    }

    #[tokio::test]
    async fn corrupt_history_cache_has_typed_path_and_source() -> anyhow::Result<()> {
        let (client, task) = server(vec![(200, serde_json::json!([]))]).await;
        let directory = tempfile::tempdir()?;
        let client = client.with_history_cache(directory.path());
        client
            .account_request("/sapi/v1/capital/deposit/hisrec", &[], false)
            .await?;
        task.await?;
        let cache_dir = client.history_cache.as_ref().unwrap();
        let path = std::fs::read_dir(cache_dir)?.next().unwrap()?.path();
        std::fs::write(&path, "bad cache")?;
        let error = client
            .account_request("/sapi/v1/capital/deposit/hisrec", &[], false)
            .await
            .unwrap_err();
        assert!(matches!(error, BinanceError::CacheDecode { path: ref p, .. } if *p == path));
        Ok(())
    }
}
