//! Read-only EVM discovery through Blockscout v2 and independent pinned JSON-RPC.
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Utc};
use lib_core::Units;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

pub fn fingerprint(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(f.as_file_mut(), value)?;
    f.as_file().sync_all()?;
    f.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub const TRANSFER: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

pub fn address(s: &str) -> Result<String> {
    ensure!(
        s.len() == 42 && s.starts_with("0x") && s[2..].bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid EVM address"
    );
    Ok(s.to_ascii_lowercase())
}
pub fn hash(s: &str) -> Result<String> {
    ensure!(
        s.len() == 66 && s.starts_with("0x") && s[2..].bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid EVM hash"
    );
    Ok(s.to_ascii_lowercase())
}
pub fn hex(s: &str) -> Result<Units> {
    let digits = s.strip_prefix("0x").context("missing hexadecimal prefix")?;
    ensure!(
        !digits.is_empty() && digits.len() <= 64 && digits.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid EVM uint256"
    );
    BigUint::parse_bytes(digits.as_bytes(), 16)
        .context("invalid hex quantity")?
        .to_string()
        .parse()
}
pub fn integer(v: &Value) -> Result<u64> {
    match v {
        Value::String(s) if s.starts_with("0x") => Ok(hex(s)?.to_string().parse()?),
        Value::String(s) => Ok(s.parse()?),
        _ => v.as_u64().context("missing integer"),
    }
}
pub fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .with_context(|| format!("missing EVM field {k}"))
}
pub fn peer(v: &Value) -> Result<String> {
    address(
        v.as_str()
            .or_else(|| v["hash"].as_str())
            .context("missing peer address")?,
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub scale: u8,
    pub balance: Units,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u8,
    pub chain: u64,
    pub address: String,
    pub block: u64,
    pub block_hash: String,
    pub timestamp: DateTime<Utc>,
    pub native_balance: Units,
    pub tokens: BTreeMap<String, Token>,
    pub transactions: BTreeMap<String, Value>,
    pub receipts: BTreeMap<String, Value>,
    #[serde(default)]
    pub transaction_outcomes: BTreeMap<String, bool>,
    pub internals: Vec<Value>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub unresolved_tokens: BTreeMap<String, String>,
    #[serde(default)]
    pub token_evidence: BTreeMap<String, Value>,
}

/// Network defaults reflect the chain names present in the Airtable exports.
/// BNB uses Etherscan V2 with chain access, or an explicit compatible indexer.
pub fn defaults(chain: u64) -> Result<(&'static str, Option<&'static str>)> {
    Ok(match chain {
        1 => ("https://eth.drpc.org", Some("https://eth.blockscout.com")),
        137 => (
            "https://polygon.drpc.org",
            Some("https://polygon.blockscout.com"),
        ),
        56 => ("https://bsc-rpc.publicnode.com", None),
        42161 => (
            "https://arb1.arbitrum.io/rpc",
            Some("https://arbitrum.blockscout.com"),
        ),
        324 => (
            "https://mainnet.era.zksync.io",
            Some("https://zksync.blockscout.com"),
        ),
        _ => bail!("unsupported EVM chain; choose 1, 137, 56, 42161 or 324"),
    })
}

struct Client {
    http: reqwest::Client,
    rpc: String,
    indexer: String,
    key: Option<String>,
    blockscout_key: Option<String>,
}
impl Client {
    fn new(rpc: &str, indexer: &str) -> Result<Self> {
        for endpoint in [rpc, indexer] {
            let u = reqwest::Url::parse(endpoint)?;
            ensure!(
                u.scheme() == "https"
                    || (u.scheme() == "http"
                        && matches!(u.host_str(), Some("localhost" | "127.0.0.1"))),
                "EVM endpoints require HTTPS or localhost"
            );
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            rpc: rpc.into(),
            indexer: indexer.trim_end_matches('/').into(),
            blockscout_key: if reqwest::Url::parse(indexer)?.host_str()
                == Some("api.blockscout.com")
            {
                Some(std::env::var("BLOCKSCOUT_API_KEY").context("Blockscout universal API requires BLOCKSCOUT_API_KEY; alternatively select Etherscan V2 with ETHERSCAN_API_KEY")?)
            } else {
                None
            },
            key: if indexer.trim_end_matches('/') == "https://api.etherscan.io/v2/api" {
                Some(std::env::var("ETHERSCAN_API_KEY").context("Etherscan history requires ETHERSCAN_API_KEY with selected chain access or a compatible --indexer override")?)
            } else {
                None
            },
        })
    }
    async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        for attempt in 0..5 {
            if !self.rpc.starts_with("http://127.0.0.1")
                && !self.rpc.starts_with("http://localhost")
            {
                tokio::time::sleep(Duration::from_millis(350)).await;
            }
            let response = self
                .http
                .post(&self.rpc)
                .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("EVM RPC transport failed"))?;
            if response.status().as_u16() == 429 {
                let delay = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1 << attempt);
                tokio::time::sleep(Duration::from_secs(delay.min(60))).await;
                continue;
            }
            ensure!(response.status().is_success(), "EVM RPC HTTP failure");
            let value: Value = response
                .json()
                .await
                .map_err(|_| anyhow::anyhow!("invalid EVM RPC JSON"))?;
            ensure!(
                value["id"] == 1 && value["jsonrpc"] == "2.0",
                "invalid EVM RPC envelope"
            );
            if let Some(error) = value.get("error").filter(|v| !v.is_null()) {
                let message = error["message"].as_str().unwrap_or("").to_ascii_lowercase();
                if message.contains("rate limit") || message.contains("too many requests") {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                // Do not echo provider text or URLs which can contain credentials.
                bail!(
                    "EVM RPC rejected {method} (code {}); check contract execution/archive support",
                    error["code"].as_i64().unwrap_or(0)
                );
            }
            return value
                .get("result")
                .filter(|v| !v.is_null())
                .cloned()
                .with_context(|| format!("missing EVM RPC result for {method}"));
        }
        bail!("EVM RPC throttle retries exhausted")
    }
    async fn etherscan_pages(
        &self,
        wallet: &str,
        stream: &str,
        chain: u64,
        block: u64,
    ) -> Result<Vec<Value>> {
        // Available to users with an appropriate Etherscan plan. BNB access is
        // not assumed to be free; no credential or request URL is logged.
        if stream == "tokens" {
            return Ok(vec![]);
        }
        let action = match stream {
            "transactions" => "txlist",
            "token-transfers" => "tokentx",
            "internal-transactions" => "txlistinternal",
            _ => bail!("unsupported Etherscan stream"),
        };
        let mut all = Vec::new();
        for page in 1..=10 {
            let mut response = None;
            for attempt in 0..5 {
                tokio::time::sleep(Duration::from_millis(550)).await;
                let r = self
                    .http
                    .get(&self.indexer)
                    .query(&[
                        ("chainid", chain.to_string()),
                        ("module", "account".into()),
                        ("action", action.into()),
                        ("address", wallet.into()),
                        ("startblock", "0".into()),
                        ("endblock", block.to_string()),
                        ("page", page.to_string()),
                        ("offset", "1000".into()),
                        ("sort", "asc".into()),
                        ("apikey", self.key.clone().context("missing Etherscan key")?),
                    ])
                    .send()
                    .await
                    .map_err(|_| anyhow::anyhow!("Etherscan transport failed"))?;
                if r.status().as_u16() == 429 {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                ensure!(r.status().is_success(), "Etherscan HTTP failure");
                let v: Value = r
                    .json()
                    .await
                    .map_err(|_| anyhow::anyhow!("invalid Etherscan JSON"))?;
                if v["result"]
                    .as_str()
                    .is_some_and(|s| s.to_ascii_lowercase().contains("rate limit"))
                {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                if v["status"] == "0"
                    && (v["message"] == "No transactions found"
                        || v["result"] == "No transactions found")
                {
                    return Ok(all);
                }
                ensure!(
                    v["status"] == "1",
                    "Etherscan rejected request; check key, chain access and plan"
                );
                response = Some(v);
                break;
            }
            let v = response.context("Etherscan throttle retries exhausted")?;
            let rows = v["result"].as_array().context("missing Etherscan rows")?;
            for r in rows {
                let mut row = r.clone();
                row["block_number"] = r["blockNumber"].clone();
                if stream == "transactions" {
                    row["status"] = match r["isError"].as_str() {
                        Some("0") => json!("ok"),
                        Some("1") => json!("error"),
                        _ => Value::Null,
                    };
                }
                if stream != "transactions" {
                    row["transaction_hash"] = r["hash"].clone();
                }
                if stream == "token-transfers" {
                    row["token"] = json!({"type":"ERC-20","address_hash":r["contractAddress"],"symbol":r["tokenSymbol"],"decimals":r["tokenDecimal"]});
                }
                if stream == "internal-transactions" {
                    row["index"] = r["traceId"].clone();
                    row["success"] = json!(r["isError"] == "0");
                    row["created_contract"] = r["contractAddress"].clone();
                }
                all.push(row);
            }
            if rows.len() < 1000 {
                return Ok(all);
            }
        }
        bail!("Etherscan history reaches 10000-row bound; split-range backfill required")
    }
    async fn pages(&self, wallet: &str, stream: &str) -> Result<Vec<Value>> {
        let mut all = Vec::new();
        let mut params = BTreeMap::<String, String>::new();
        if stream == "tokens" {
            params.insert("type".into(), "ERC-20".into());
        }
        let mut cursors = BTreeSet::new();
        for _ in 0..100_000 {
            ensure!(
                cursors.insert(serde_json::to_string(&params)?),
                "repeated EVM indexer cursor"
            );
            let mut response = None;
            for attempt in 0..5 {
                if self.blockscout_key.is_some() {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                let mut request = self
                    .http
                    .get(format!(
                        "{}/api/v2/addresses/{wallet}/{stream}",
                        self.indexer
                    ))
                    .query(&params);
                if let Some(key) = &self.blockscout_key {
                    request = request.query(&[("apikey", key)]);
                }
                let r = request
                    .send()
                    .await
                    .map_err(|_| anyhow::anyhow!("EVM indexer transport failed"))?;
                if r.status().as_u16() == 429 {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                ensure!(
                    r.status().is_success(),
                    "EVM indexer HTTP {}; check compatible API v2 endpoint",
                    r.status().as_u16()
                );
                response = Some(
                    r.json::<Value>()
                        .await
                        .map_err(|_| anyhow::anyhow!("invalid EVM indexer JSON"))?,
                );
                break;
            }
            let v = response.context("EVM indexer throttle retries exhausted")?;
            let items = v["items"].as_array().context("missing EVM indexer items")?;
            all.extend(items.iter().cloned());
            match v.get("next_page_params") {
                Some(Value::Null) => return Ok(all),
                Some(Value::Object(next)) => {
                    ensure!(
                        !items.is_empty() && !next.is_empty(),
                        "nonadvancing EVM indexer page"
                    );
                    params = next
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                v.as_str()
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| v.to_string()),
                            )
                        })
                        .collect();
                    if stream == "tokens" {
                        params.insert("type".into(), "ERC-20".into());
                    }
                }
                _ => bail!("missing EVM pagination completeness marker"),
            }
        }
        bail!("EVM page bound exceeded")
    }
    async fn call(&self, contract: &str, data: String, tag: &str) -> Result<Value> {
        self.rpc("eth_call", json!([{"to":contract,"data":data},tag]))
            .await
    }
    async fn resolve_token(
        &self,
        contract: &str,
        symbol: &str,
        metadata_scale: Option<u8>,
        tag: &str,
        receipts: &BTreeMap<String, Value>,
        wallet: &str,
        warnings: &mut Vec<String>,
        evidence: &mut BTreeMap<String, Value>,
    ) -> Result<Token> {
        let decimals = self.call(contract, "0x313ce567".into(), tag).await?;
        let mut proof = json!({"cutoff":tag,"decimals_response":decimals});
        let response = match self
            .call(contract, format!("0x70a08231{:0>64}", &wallet[2..]), tag)
            .await
        {
            Ok(response) => {
                proof["balance_response"] = response.clone();
                response
            }
            Err(error) => {
                proof["balance_error"] = json!(format!("{error:#}"));
                evidence.insert(contract.into(), proof);
                return Err(error.context("balanceOf failed at frozen cutoff"));
            }
        };
        let result: Result<Token> = async {
            let scale = if decimals.as_str() == Some("0x") {
                let first = receipts.values().filter(|r| r["logs"].as_array().is_some_and(|logs| logs.iter().any(|log| log["address"].as_str().is_some_and(|c| c.eq_ignore_ascii_case(contract)))))
                    .map(|r| integer(&r["blockNumber"])).collect::<Result<Vec<_>>>()?.into_iter().min();
                let historical = if let Some(block) = first {
                    let historical_tag = format!("0x{block:x}");
                    let value = self.call(contract, "0x313ce567".into(), &historical_tag).await?;
                    let header = self.rpc("eth_getBlockByNumber", json!([historical_tag,false])).await?;
                    ensure!(integer(&header["number"])? == block, "historical decimals header mismatch");
                    ensure!(receipts.values().filter(|r| integer(&r["blockNumber"]).ok() == Some(block)).all(|r| r["blockHash"] == header["hash"]), "historical decimals block reorg");
                    proof["historical_decimals"] = json!({"block":block,"block_hash":header["hash"],"response":value});
                    if value.as_str() == Some("0x") { None } else {
                        Some(hex(value.as_str().context("invalid historical decimals")?)?.to_string().parse::<u8>().context("invalid historical token scale")?)
                    }
                } else { None };
                if let Some(scale) = historical {
                    if let Some(metadata_scale) = metadata_scale { ensure!(scale == metadata_scale, "historical/indexer decimals conflict"); }
                    warnings.push(format!("Token {contract} omits decimals at cutoff; using evidenced historical RPC scale"));
                    scale
                } else {
                    let scale = metadata_scale.context("token decimals unavailable from current/historical RPC and indexer; raw units remain unresolved")?;
                    warnings.push(format!("Token {contract} omits decimals(); using indexer metadata"));
                    scale
                }
            } else {
                hex(decimals.as_str().context("invalid token decimals")?)?.to_string().parse::<u8>().context("invalid token scale")?
            };

            let balance = if response.as_str() == Some("0x") {
                let code = self.rpc("eth_getCode", json!([contract,tag])).await?;
                proof["code_response"] = code.clone();
                ensure!(code == "0x", "token balanceOf() returned no data for a live contract");
                warnings.push(format!("Token {contract} has no deployed code at cutoff; zero observable balance does not erase historical ledger holdings"));
                0.into()
            } else { hex(response.as_str().context("invalid token balance")?)? };
            Ok(Token {symbol:symbol.into(),scale,balance})
        }.await;
        evidence.insert(contract.into(), proof);
        result
    }
    async fn refine(&self, s: &mut Snapshot) -> Result<()> {
        ensure!(
            integer(&self.rpc("eth_chainId", json!([])).await?)? == s.chain,
            "RPC chain ID mismatch"
        );
        let tag = format!("0x{:x}", s.block);
        ensure!(
            self.rpc("eth_getBlockByNumber", json!([tag, false]))
                .await?["hash"]
                == s.block_hash,
            "cached EVM cutoff is no longer canonical"
        );
        let pending = s.unresolved_tokens.keys().cloned().collect::<Vec<_>>();
        s.warnings
            .retain(|w| !pending.iter().any(|contract| w.contains(contract)));
        for contract in pending {
            let result = self
                .resolve_token(
                    &contract,
                    "unknown",
                    None,
                    &tag,
                    &s.receipts,
                    &s.address,
                    &mut s.warnings,
                    &mut s.token_evidence,
                )
                .await;
            match result {
                Ok(token) => {
                    s.tokens.insert(contract.clone(), token);
                    s.unresolved_tokens.remove(&contract);
                }
                Err(error) => {
                    s.unresolved_tokens.insert(contract, format!("{error:#}"));
                }
            }
        }
        ensure!(
            self.rpc("eth_getBlockByNumber", json!([tag, false]))
                .await?["hash"]
                == s.block_hash,
            "cached EVM cutoff changed while resolving metadata"
        );
        for (contract, proof) in &s.token_evidence {
            if proof["historical_decimals"]["response"]
                .as_str()
                .is_some_and(|value| value != "0x")
            {
                s.warnings.retain(|warning| {
                    warning != &format!("Token {contract} omits decimals(); using indexer metadata")
                });
            }
        }
        s.warnings.sort();
        s.warnings.dedup();
        Ok(())
    }
    async fn fetch(&self, chain: u64, wallet: &str, confirmations: u64) -> Result<Snapshot> {
        ensure!(
            integer(&self.rpc("eth_chainId", json!([])).await?)? == chain,
            "RPC chain ID mismatch"
        );
        let head = integer(&self.rpc("eth_blockNumber", json!([])).await?)?;
        let block = head
            .checked_sub(confirmations)
            .context("confirmation depth exceeds head")?;
        let tag = format!("0x{block:x}");
        let header = self
            .rpc("eth_getBlockByNumber", json!([tag, false]))
            .await?;
        ensure!(
            integer(&header["number"])? == block,
            "RPC cutoff header number mismatch"
        );
        let block_hash = hash(text(&header, "hash")?)?;
        let timestamp = DateTime::from_timestamp(integer(&header["timestamp"])?.try_into()?, 0)
            .context("invalid block timestamp")?;
        eprintln!("Discovering EVM chain {chain} through block {block}");
        let (normal, transfers, internals, inventory) = if self.key.is_some() {
            (
                self.etherscan_pages(wallet, "transactions", chain, block)
                    .await?,
                self.etherscan_pages(wallet, "token-transfers", chain, block)
                    .await?,
                self.etherscan_pages(wallet, "internal-transactions", chain, block)
                    .await?,
                vec![],
            )
        } else {
            (
                self.pages(wallet, "transactions").await?,
                self.pages(wallet, "token-transfers").await?,
                self.pages(wallet, "internal-transactions").await?,
                self.pages(wallet, "tokens").await?,
            )
        };
        let mut hashes = BTreeSet::new();
        let mut transaction_outcomes = BTreeMap::new();
        let mut contracts = BTreeMap::new();
        let mut warnings=vec!["ERC-20 discovery depends on indexer completeness; NFTs and protocol underlying exposures are not included".into()];
        if self.key.is_some() {
            warnings.push("Etherscan discovery uses transfer history; no independent current token inventory endpoint is included".into());
        }
        for row in &normal {
            if row["block_number"].is_null() {
                continue;
            } // pending, never booked
            let n = integer(&row["block_number"])?;
            if n <= block {
                let h = hash(text(row, "hash")?)?;
                match row["status"].as_str() {
                    Some("ok") => {
                        transaction_outcomes.insert(h.clone(), true);
                    }
                    Some("error") => {
                        transaction_outcomes.insert(h.clone(), false);
                    }
                    _ => {}
                }
                ensure!(
                    hashes.insert(hash(text(row, "hash")?)?),
                    "duplicate normal transaction in indexer pages"
                );
            }
        }
        let mut bounded_internals = Vec::new();
        for row in internals {
            if integer(&row["block_number"])? <= block {
                bounded_internals.push(row);
            }
        }
        let internals = bounded_internals;
        for row in internals.iter().chain(transfers.iter()) {
            if integer(&row["block_number"])? > block {
                continue;
            }
            hashes.insert(hash(text(row, "transaction_hash")?)?);
        }
        for row in transfers.iter().chain(inventory.iter()) {
            let token = &row["token"];
            if token["type"] != "ERC-20" {
                continue;
            }
            contracts.insert(
                address(text(token, "address_hash")?)?,
                (
                    token["symbol"].as_str().unwrap_or("unknown").to_owned(),
                    token
                        .get("decimals")
                        .and_then(|v| integer(v).ok())
                        .and_then(|n| u8::try_from(n).ok()),
                ),
            );
        }
        let mut transactions = BTreeMap::new();
        let mut receipts = BTreeMap::new();
        eprintln!(
            "Fetching {} EVM transactions/receipts and {} discovered contracts",
            hashes.len(),
            contracts.len()
        );
        for h in hashes {
            let tx = self.rpc("eth_getTransactionByHash", json!([h])).await?;
            let receipt = self.rpc("eth_getTransactionReceipt", json!([h])).await?;
            ensure!(
                hash(text(&tx, "hash")?)? == h && hash(text(&receipt, "transactionHash")?)? == h,
                "EVM transaction identity mismatch"
            );
            let n = integer(&receipt["blockNumber"])?;
            ensure!(
                n <= block && integer(&tx["blockNumber"])? == n,
                "EVM receipt outside cutoff"
            );
            let canonical = self
                .rpc("eth_getBlockByNumber", json!([format!("0x{n:x}"), false]))
                .await?;
            ensure!(
                integer(&canonical["number"])? == n,
                "RPC transaction header number mismatch"
            );
            ensure!(
                hash(text(&receipt, "blockHash")?)? == hash(text(&canonical, "hash")?)?
                    && tx["blockHash"] == receipt["blockHash"],
                "EVM noncanonical receipt/reorg"
            );
            let mut tx = tx;
            tx["timestamp"] = canonical["timestamp"].clone();
            // Receipt log indices provide native identities even when a token
            // transfer indexer does not expose a stable event identifier.
            for log in receipt["logs"].as_array().context("missing receipt logs")? {
                let topics = log["topics"].as_array().context("missing log topics")?;
                if topics.len() == 3
                    && topics[0] == TRANSFER
                    && (topic_address(&topics[1])? == wallet
                        || topic_address(&topics[2])? == wallet)
                {
                    contracts
                        .entry(address(text(log, "address")?)?)
                        .or_insert_with(|| ("unknown".into(), None));
                }
            }
            transactions.insert(h.clone(), tx);
            receipts.insert(h, receipt);
        }
        let native_balance = hex(self
            .rpc("eth_getBalance", json!([wallet, tag]))
            .await?
            .as_str()
            .context("invalid native balance")?)?;
        let mut tokens = BTreeMap::new();
        let mut unresolved_tokens = BTreeMap::new();
        let mut token_evidence = BTreeMap::new();
        for (contract, (symbol, metadata_scale)) in contracts {
            let result = self
                .resolve_token(
                    &contract,
                    &symbol,
                    metadata_scale,
                    &tag,
                    &receipts,
                    wallet,
                    &mut warnings,
                    &mut token_evidence,
                )
                .await;
            match result {
                Ok(token) => {
                    tokens.insert(contract, token);
                }
                Err(error) => {
                    unresolved_tokens.insert(contract, format!("{error:#}"));
                }
            }
        }
        ensure!(
            self.rpc("eth_getBlockByNumber", json!([tag, false]))
                .await?["hash"]
                == header["hash"],
            "EVM cutoff changed during acquisition; refresh required"
        );
        if chain == 324 {
            warnings.push("zkSync Era only; legacy zkSync Lite is not an EVM chain".into());
        }
        Ok(Snapshot {
            version: 1,
            chain,
            address: wallet.into(),
            block,
            block_hash,
            timestamp,
            native_balance,
            tokens,
            transactions,
            receipts,
            transaction_outcomes,
            internals,
            warnings,
            unresolved_tokens,
            token_evidence,
        })
    }
}
pub fn topic_address(value: &Value) -> Result<String> {
    let s = value.as_str().context("invalid address topic")?;
    ensure!(
        s.len() == 66 && s.starts_with("0x") && s[2..26].bytes().all(|b| b == b'0'),
        "invalid address topic"
    );
    address(&format!("0x{}", &s[26..]))
}
/// Resolve metadata against the original frozen cutoff; never advance balances/history.
pub async fn resolve_cached_tokens(
    path: &Path,
    expected_account: &str,
    rpc: Option<&str>,
) -> Result<()> {
    let mut s: Snapshot = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(
        s.version == 1 && expected_account == format!("eip155:{}:{}", s.chain, s.address),
        "EVM resolution cache/account identity conflict"
    );
    address(&s.address)?;
    let (default_rpc, _) = defaults(s.chain)?;
    // RPC-only resolution does not need an indexer credential.
    Client::new(rpc.unwrap_or(default_rpc), "https://eth.blockscout.com")?
        .refine(&mut s)
        .await?;
    write_json(path, &s)
}
fn discovery_endpoint(chain: u64, requested: Option<&str>, default: Option<&str>) -> String {
    if let Some(requested) = requested {
        return requested.into();
    }
    if matches!(chain, 137 | 42161) {
        if std::env::var_os("ETHERSCAN_API_KEY").is_some() {
            return "https://api.etherscan.io/v2/api".into();
        }
        if std::env::var_os("BLOCKSCOUT_API_KEY").is_some() {
            return format!("https://api.blockscout.com/{chain}");
        }
    }
    default.unwrap_or("https://api.etherscan.io/v2/api").into()
}
pub async fn cached(
    path: &Path,
    chain: u64,
    wallet: &str,
    rpc: Option<&str>,
    indexer: Option<&str>,
    confirmations: u64,
    offline: bool,
    refresh: bool,
) -> Result<Snapshot> {
    let wallet = address(wallet)?;
    ensure!(!(offline && refresh), "offline EVM refresh is invalid");
    if !refresh && path.exists() {
        let s: Snapshot = serde_json::from_slice(&std::fs::read(path)?)?;
        ensure!(
            s.version == 1 && s.chain == chain && s.address == wallet,
            "EVM cache identity conflict"
        );
        return Ok(s);
    }
    ensure!(!offline, "EVM snapshot missing offline");
    let (default_rpc, default_indexer) = defaults(chain)?;
    let client = Client::new(
        rpc.unwrap_or(default_rpc),
        &discovery_endpoint(chain, indexer, default_indexer),
    )?;
    let s = client.fetch(chain, &wallet, confirmations).await?;
    write_json(path, &s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_uint256_and_locator_validation() -> Result<()> {
        assert_eq!(
            hex(&format!("0x{}", "f".repeat(64)))?.to_string(),
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"
        );
        assert!(hex("0x-1").is_err());
        assert!(hex(&format!("0x{}", "f".repeat(65))).is_err());
        assert!(address("not-a-wallet").is_err());
        assert!(defaults(280).is_err());
        assert!(defaults(56)?.1.is_none());
        Ok(())
    }
    #[tokio::test]
    async fn frozen_cache_is_bound_to_chain_and_address() -> Result<()> {
        let wallet = format!("0x{}", "a".repeat(40));
        let s = Snapshot {
            version: 1,
            chain: 1,
            address: wallet.clone(),
            block: 10,
            block_hash: format!("0x{}", "b".repeat(64)),
            timestamp: Utc::now(),
            native_balance: 0.into(),
            tokens: BTreeMap::new(),
            transactions: BTreeMap::new(),
            receipts: BTreeMap::new(),
            transaction_outcomes: BTreeMap::new(),
            internals: vec![],
            warnings: vec![],
            unresolved_tokens: BTreeMap::new(),
            token_evidence: BTreeMap::new(),
        };
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("s.json");
        write_json(&path, &s)?;
        assert_eq!(
            cached(&path, 1, &wallet, None, None, 64, true, false)
                .await?
                .block,
            10
        );
        assert!(
            cached(&path, 137, &wallet, None, None, 64, true, false)
                .await
                .is_err()
        );
        assert!(
            cached(&path, 1, &wallet, None, None, 64, true, true)
                .await
                .is_err()
        );
        std::fs::write(&path, b"invalid")?;
        assert!(
            cached(&path, 1, &wallet, None, None, 64, true, false)
                .await
                .is_err()
        );
        Ok(())
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    async fn rpc_fixture(
        handler: impl Fn(Value) -> Value + Send + 'static,
    ) -> Result<(String, tokio::task::JoinHandle<()>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let (end, length) = loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(p) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..p]);
                        let length = header
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length: ")
                                    .and_then(|s| s.parse::<usize>().ok())
                            })
                            .unwrap();
                        break (p + 4, length);
                    }
                };
                while bytes.len() < end + length {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                let request: Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
                let result = handler(request);
                let body = if let Some(error) = result.get("_rpc_error") {
                    json!({"jsonrpc":"2.0","id":1,"error":error})
                } else {
                    json!({"jsonrpc":"2.0","id":1,"result":result})
                }
                .to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        Ok((base, task))
    }
    #[tokio::test]
    async fn rpc_throttle_retries_and_errors_do_not_echo_secrets() -> Result<()> {
        let mut count = 0;
        let calls = Arc::new(Mutex::new(0));
        let seen = calls.clone();
        let (base, server) = rpc_fixture(move |_| {
            let mut count = seen.lock().unwrap();
            *count += 1;
            if *count == 1 {
                json!({"_rpc_error":{"code":-32005,"message":"too many requests"}})
            } else {
                json!("0x1")
            }
        })
        .await?;
        let client = Client::new(&base, &base)?;
        assert_eq!(client.rpc("eth_chainId", json!([])).await?, "0x1");
        count += *calls.lock().unwrap();
        assert_eq!(count, 2);
        server.abort();
        let (base, server) = rpc_fixture(
            |_| json!({"_rpc_error":{"code":3,"message":"execution reverted; secret-key-value"}}),
        )
        .await?;
        let client = Client::new(&base, &base)?;
        let error = client
            .rpc("eth_call", json!([]))
            .await
            .unwrap_err()
            .to_string();
        server.abort();
        assert!(error.contains("code 3"));
        assert!(!error.contains("secret-key-value"));
        Ok(())
    }
    #[tokio::test]
    async fn historical_zero_scale_requires_canonical_evidence_and_no_code_for_zero_balance()
    -> Result<()> {
        let contract = format!("0x{}", "c".repeat(40));
        let wallet = format!("0x{}", "a".repeat(40));
        let header_hash = format!("0x{}", "d".repeat(64));
        let receipts = BTreeMap::from([(
            "tx".into(),
            json!({"blockNumber":"0x5","blockHash":header_hash,"logs":[{"address":contract}]}),
        )]);
        for (code, canonical, expect_success) in [
            ("0x", true, true),
            ("0x6000", true, false),
            ("0x", false, false),
        ] {
            let h = header_hash.clone();
            let (base,server)=rpc_fixture(move |r| match r["method"].as_str().unwrap() {
                "eth_call" if r["params"][1]=="0x5"=>json!("0x0"),
                "eth_call"=>json!("0x"),
                "eth_getBlockByNumber"=>json!({"number":"0x5","hash":if canonical {h.clone()} else {format!("0x{}","e".repeat(64))}}),
                "eth_getCode"=>json!(code),
                _=>panic!("unexpected RPC"),
            }).await?;
            let client = Client::new(&base, &base)?;
            let mut warnings = vec![];
            let mut evidence = BTreeMap::new();
            let result = client
                .resolve_token(
                    &contract,
                    "retired",
                    None,
                    "0xa",
                    &receipts,
                    &wallet,
                    &mut warnings,
                    &mut evidence,
                )
                .await;
            server.abort();
            assert_eq!(result.is_ok(), expect_success);
            if let Ok(token) = result {
                assert_eq!(token.scale, 0);
                assert_eq!(token.balance, 0);
                assert_eq!(evidence[&contract]["historical_decimals"]["block"], 5);
                assert!(warnings.iter().any(|w| w.contains("does not erase")));
            }
        }
        Ok(())
    }
    #[tokio::test]
    async fn reverting_balance_getter_retains_method_specific_evidence() -> Result<()> {
        let (base, server) = rpc_fixture(|r| {
            if r["params"][0]["data"] == "0x313ce567" {
                json!("0x12")
            } else {
                json!({"_rpc_error":{"code":3,"message":"execution reverted"}})
            }
        })
        .await?;
        let client = Client::new(&base, &base)?;
        let contract = format!("0x{}", "c".repeat(40));
        let wallet = format!("0x{}", "a".repeat(40));
        let mut evidence = BTreeMap::new();
        let error = client
            .resolve_token(
                &contract,
                "unknown",
                None,
                "0xa",
                &BTreeMap::new(),
                &wallet,
                &mut vec![],
                &mut evidence,
            )
            .await
            .unwrap_err()
            .to_string();
        server.abort();
        assert!(error.contains("balanceOf"));
        assert_eq!(evidence[&contract]["decimals_response"], "0x12");
        assert!(
            evidence[&contract]["balance_error"]
                .as_str()
                .unwrap()
                .contains("code 3")
        );
        Ok(())
    }
    #[tokio::test]
    async fn missing_optional_decimals_stays_unresolved_with_raw_balance_evidence() -> Result<()> {
        let (base, server) = rpc_fixture(|r| {
            assert_eq!(r["params"][1], "0xa");
            if r["params"][0]["data"] == "0x313ce567" {
                json!("0x")
            } else {
                json!("0x309")
            }
        })
        .await?;
        let client = Client::new(&base, &base)?;
        let contract = format!("0x{}", "c".repeat(40));
        let wallet = format!("0x{}", "a".repeat(40));
        let mut evidence = BTreeMap::new();
        let result = client
            .resolve_token(
                &contract,
                "promo",
                None,
                "0xa",
                &BTreeMap::new(),
                &wallet,
                &mut vec![],
                &mut evidence,
            )
            .await;
        server.abort();
        assert!(result.is_err());
        assert_eq!(evidence[&contract]["balance_response"], "0x309");
        Ok(())
    }
    #[tokio::test]
    async fn universal_blockscout_key_is_attached_to_chain_scoped_pages() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            assert!(request.contains("/137/api/v2/addresses/wallet/transactions?"));
            assert!(request.contains("apikey=fixture-only"));
            let body = json!({"items":[],"next_page_params":null}).to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        });
        let mut client = Client::new(&base, &format!("{base}/137"))?;
        client.blockscout_key = Some("fixture-only".into());
        assert!(client.pages("wallet", "transactions").await?.is_empty());
        server.await?;
        Ok(())
    }
    #[tokio::test]
    async fn etherscan_distinguishes_empty_history_from_denial_and_keeps_trace_ids() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let server = tokio::spawn(async move {
            for (status, body) in [
                (
                    200,
                    json!({"status":"0","message":"No transactions found","result":[]}),
                ),
                (
                    200,
                    json!({"status":"0","message":"NOTOK","result":"Invalid API Key"}),
                ),
                (
                    200,
                    json!({"status":"1","result":[{"hash":"hash","blockNumber":"10","traceId":"0_2","isError":"0","contractAddress":""}]}),
                ),
                (403, json!({"status":"1","result":[]})),
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0; 8192];
                let n = stream.read(&mut request).await.unwrap();
                let request = String::from_utf8_lossy(&request[..n]);
                assert!(request.contains("chainid=56"));
                assert!(request.contains("endblock=10"));
                assert!(request.contains("apikey=test-only"));
                let body = body.to_string();
                stream.write_all(format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let mut client = Client::new(&base, &base)?;
        client.key = Some("test-only".into());
        assert!(
            client
                .etherscan_pages("wallet", "transactions", 56, 10)
                .await?
                .is_empty()
        );
        let denied = client
            .etherscan_pages("wallet", "transactions", 56, 10)
            .await
            .unwrap_err()
            .to_string();
        assert!(denied.contains("rejected"));
        assert!(!denied.contains("test-only"));
        let rows = client
            .etherscan_pages("wallet", "internal-transactions", 56, 10)
            .await?;
        assert_eq!(rows[0]["index"], "0_2");
        assert_eq!(rows[0]["success"], true);
        assert!(
            client
                .etherscan_pages("wallet", "transactions", 56, 10)
                .await
                .is_err()
        );
        server.await?;
        Ok(())
    }
    #[tokio::test]
    async fn discovery_paginates_and_independent_calls_share_a_fixed_block() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let wallet = format!("0x{}", "a".repeat(40));
        let contract = format!("0x{}", "c".repeat(40));
        let calls = Arc::new(Mutex::new(Vec::<Value>::new()));
        let seen = calls.clone();
        let w = wallet.clone();
        let c = contract.clone();
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let end;
                loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(p) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        end = p + 4;
                        break;
                    }
                }
                let header = String::from_utf8(bytes[..end].to_vec()).unwrap();
                let length = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < end + length {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                let path = header.split_whitespace().nth(1).unwrap();
                let body = if header.starts_with("POST") {
                    let request: Value = serde_json::from_slice(&bytes[end..]).unwrap();
                    seen.lock().unwrap().push(request.clone());
                    let result = match request["method"].as_str().unwrap() {
                        "eth_chainId" => json!("0x1"),
                        "eth_blockNumber" => json!("0x4a"),
                        "eth_getBlockByNumber" => {
                            json!({"number":"0xa","hash":format!("0x{}","d".repeat(64)),"timestamp":"0x6553f100"})
                        }
                        "eth_getBalance" => json!("0x0"),
                        "eth_call" if request["params"][0]["data"] == "0x313ce567" => json!("0x12"),
                        "eth_call" => json!(format!("0x1{}", "0".repeat(40))),
                        _ => panic!("unexpected mock RPC method"),
                    };
                    json!({"id":1,"jsonrpc":"2.0","result":result})
                } else if path.contains("/tokens") {
                    if path.contains("items_count") {
                        json!({"items":[],"next_page_params":null})
                    } else {
                        json!({"items":[{"token":{"address_hash":c,"symbol":"MOCK","type":"ERC-20"}}],"next_page_params":{"items_count":1}})
                    }
                } else {
                    assert!(path.contains(&w));
                    json!({"items":[],"next_page_params":null})
                };
                let text = body.to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",text.len()).as_bytes()).await.unwrap();
            }
        });
        let client = Client::new(&base, &base)?;
        let result = client.fetch(1, &wallet, 64).await;
        server.abort();
        let s = result?;
        assert_eq!(s.block, 10);
        assert_eq!(s.tokens.len(), 1);
        assert_eq!(s.tokens[&contract].scale, 18);
        assert!(s.tokens[&contract].balance.to_string().len() > 39);
        for call in calls.lock().unwrap().iter() {
            match call["method"].as_str().unwrap() {
                "eth_call" | "eth_getBalance" => assert_eq!(call["params"][1], "0xa"),
                "eth_getBlockByNumber" => assert_eq!(call["params"][0], "0xa"),
                _ => (),
            }
        }
        Ok(())
    }
}
