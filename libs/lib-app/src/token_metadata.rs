//! Explicit, held-token metadata acquisition; offline reads use core's DB cache.
use anyhow::{Result, ensure};
use base64::Engine;
use chrono::Utc;
use lib_core::{
    AssetId, PortfolioId, Store,
    token_metadata::{TokenMetadata, valid_contract},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Duration};

/// Presentation only: exact IDs stay in DTO identities and all financial operations.
pub fn short_asset_label(id: &str) -> String {
    let Some(rest) = id.strip_prefix("eip155:") else {
        return id.to_owned();
    };
    let (chain, asset) = rest.split_once('/').unwrap_or((rest, "unknown"));
    let network = match chain {
        "1" => "Ethereum",
        "137" => "Polygon",
        "56" => "BNB Chain",
        "42161" => "Arbitrum",
        "324" => "zkSync",
        _ => "Unknown chain",
    };
    if let Some(contract) = asset.strip_prefix("erc20:").filter(|s| valid_contract(s)) {
        format!(
            "Unknown token · {network} · {}…{}",
            &contract[..6],
            &contract[38..]
        )
    } else {
        format!("{network} asset")
    }
}
pub fn display_identity(store: &Store, id: &AssetId) -> Result<(String, Option<String>, bool)> {
    let display = store.asset_display(id)?;
    let fetched = store.token_metadata(id)?;
    let useful =
        |s: Option<String>| s.filter(|s| !s.starts_with("eip155:") && !s.trim().is_empty());
    let native =
        id.0.starts_with("eip155:")
            .then(|| crate::asset_groups::representation(&id.0))
            .flatten()
            .map(|r| (r.name, r.ticker));
    let name = useful(display.as_ref().and_then(|d| d.name.clone()))
        .or_else(|| useful(fetched.as_ref().and_then(|m| m.name.clone())))
        .or_else(|| native.map(|(n, _)| n.to_owned()));
    let ticker = useful(display.as_ref().and_then(|d| d.ticker.clone()))
        .or_else(|| useful(fetched.as_ref().and_then(|m| m.ticker.clone())))
        .or_else(|| native.map(|(_, t)| t.to_owned()));
    let unresolved = id.0.starts_with("eip155:") && name.is_none() && ticker.is_none();
    Ok((
        name.or(ticker.clone())
            .unwrap_or_else(|| short_asset_label(&id.0)),
        ticker,
        unresolved,
    ))
}
fn text(v: &Value) -> Option<String> {
    v.as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
fn decimal(v: &Value) -> Option<u8> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_u64().and_then(|n| u8::try_from(n).ok()))
}
pub fn safe_icon(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let url = reqwest::Url::parse(s).ok()?;
    (s.len() <= 2048
        && url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "coin-images.coingecko.com" | "assets.coingecko.com" | "raw.githubusercontent.com"
            )
        ))
    .then(|| s.to_owned())
}
fn apply_blockscout(m: &mut TokenMetadata, v: &Value) -> Result<()> {
    ensure!(
        v["address_hash"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(&m.contract))
            && v["type"] == "ERC-20",
        "explorer token identity mismatch"
    );
    m.name = text(&v["name"]).or(m.name.take());
    m.ticker = text(&v["symbol"]).or(m.ticker.take());
    m.icon_url = safe_icon(&v["icon_url"]).or(m.icon_url.take());
    m.decimals = decimal(&v["decimals"]).or(m.decimals);
    Ok(())
}
fn apply_coingecko(m: &mut TokenMetadata, v: &Value) -> Result<()> {
    let platform = if m.chain_id == 137 {
        "polygon-pos"
    } else {
        "ethereum"
    };
    ensure!(
        v["platforms"][platform]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(&m.contract)),
        "market token identity mismatch"
    );
    m.coingecko_id = text(&v["id"]);
    m.name = m.name.take().or_else(|| text(&v["name"]));
    m.ticker = m.ticker.take().or_else(|| text(&v["symbol"]));
    m.icon_url = m
        .icon_url
        .take()
        .or_else(|| safe_icon(&v["image"]["small"]));
    if let Some(d) = decimal(&v["detail_platforms"][platform]["decimal_place"]) {
        if m.decimals.is_some_and(|old| old != d) {
            m.warnings
                .push("Explorer and market decimals disagree".into());
        }
        m.decimals = m.decimals.or(Some(d));
    }
    Ok(())
}
fn rpc_text(v: &Value) -> Option<String> {
    let hex = v["result"].as_str()?.strip_prefix("0x")?;
    if hex.len() > 1024 || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect::<Option<_>>()?;
    let raw = if bytes.len() == 32 {
        &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(32)]
    } else {
        if bytes.len() < 64
            || bytes[..31].iter().any(|b| *b != 0)
            || bytes[31] != 32
            || bytes[32..63].iter().any(|b| *b != 0)
        {
            return None;
        }
        let n = bytes[63] as usize;
        bytes.get(64..64 + n)?
    };
    text(&json!(std::str::from_utf8(raw).ok()?))
}
async fn response(request: reqwest::RequestBuilder) -> Result<Value> {
    let mut r = request
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("metadata transport failed"))?;
    ensure!(
        r.status().is_success(),
        "metadata HTTP {}",
        r.status().as_u16()
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = r
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("metadata response failed"))?
    {
        ensure!(
            bytes.len() + chunk.len() <= 2_000_000,
            "metadata response too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
/// No wallet requests, ticker matching, price writes, or ledger writes.
pub async fn refresh_held_ethereum(store: &Store, portfolio: &PortfolioId) -> Result<usize> {
    refresh_held_network(store, portfolio, 1).await
}
/// Polygon acquisition is limited to reviewed mainstream exact contracts; no ticker guesses.
pub async fn refresh_held_network(
    store: &Store,
    portfolio: &PortfolioId,
    chain_id: u64,
) -> Result<usize> {
    ensure!(matches!(chain_id, 1 | 137), "unsupported metadata network");
    let platform = if chain_id == 137 {
        "polygon-pos"
    } else {
        "ethereum"
    };
    let explorer_host = if chain_id == 137 {
        "polygon.blockscout.com"
    } else {
        "eth.blockscout.com"
    };
    let rpc = if chain_id == 137 {
        "https://polygon.drpc.org"
    } else {
        "https://eth.drpc.org"
    };
    let prefix = format!("eip155:{chain_id}/erc20:");
    ensure!(
        store.portfolios()?.iter().any(|p| &p.id == portfolio),
        "unknown portfolio"
    );
    let positions: std::collections::BTreeMap<_, _> = store
        .position_metadata()?
        .into_iter()
        .map(|(p, a, _, _)| (p.0, a.0))
        .collect();
    let ids: BTreeSet<_> = store
        .portfolio_balances_at(portfolio, Utc::now())?
        .into_iter()
        .filter(|b| b.amount != lib_core::Units::from(0))
        .filter_map(|b| positions.get(&b.position_id.0).cloned())
        .filter(|id| {
            id.strip_prefix(&prefix).is_some_and(valid_contract)
                && (chain_id == 1 || crate::asset_groups::representation(id).is_some())
        })
        .collect();
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let cg = coingecko_client::CoingeckoClient::new()?;
    for raw_id in &ids {
        let id = &AssetId::str(raw_id);
        let contract = raw_id.strip_prefix(&prefix).unwrap().to_owned();
        let mut m = store.token_metadata(id)?.unwrap_or(TokenMetadata {
            asset_id: id.clone(),
            chain_id,
            contract,
            name: None,
            ticker: None,
            icon_url: None,
            icon_data: None,
            decimals: None,
            rpc_decimals: None,
            coingecko_id: None,
            fetched_at: None,
            sources: Default::default(),
            warnings: vec![],
        });
        let mut fetched = false;
        m.warnings.clear();
        m.rpc_decimals = None;
        let explorer = format!("https://{explorer_host}/api/v2/tokens/{}", m.contract);
        match response(http.get(&explorer)).await {
            Ok(v) => match apply_blockscout(&mut m, &v) {
                Ok(()) => {
                    fetched = true;
                    m.sources
                        .insert(explorer, json!({"fetched_at":Utc::now(),"response":v}));
                }
                Err(_) => m
                    .warnings
                    .push("Explorer identity mismatch; response ignored".into()),
            },
            Err(_) => m
                .warnings
                .push("Explorer metadata unavailable; retained cache".into()),
        }
        let path = format!("coins/{platform}/contract/{}", m.contract);
        match response(cg.http_client.get(format!("{}{}", cg.base_url, path))).await {
            Ok(v) => match apply_coingecko(&mut m, &v) {
                Ok(()) => {
                    fetched = true;
                    m.sources
                        .insert(format!("https://api.coingecko.com/api/v3/{path}"), json!({"fetched_at":Utc::now(),"response":{"id":v["id"],"name":v["name"],"symbol":v["symbol"],"platforms":v["platforms"],"detail_platforms":v["detail_platforms"],"image":v["image"]}}));
                }
                Err(_) => m
                    .warnings
                    .push("Market identity mismatch; response ignored".into()),
            },
            Err(_) => m
                .warnings
                .push("CoinGecko metadata unavailable; retained cache".into()),
        }
        let chain = response(
            http.post(rpc)
                .json(&json!({"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]})),
        )
        .await;
        if chain.as_ref().is_ok_and(|v| {
            v["result"] == format!("0x{chain_id:x}")
                && v["id"] == 1
                && v["jsonrpc"] == "2.0"
                && v["error"].is_null()
        }) {
            let block = response(
                http.post(rpc)
                    .json(&json!({"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]})),
            )
            .await;
            if let Ok(block) = block {
                if let Some(tag) = block["result"]
                    .as_str()
                    .filter(|s| s.starts_with("0x") && u64::from_str_radix(&s[2..], 16).is_ok())
                {
                    let mut evidence =
                        json!({"fetched_at":Utc::now(), "chain_id":chain_id, "block":tag});
                    let mut observed = false;
                    for (field, selector) in [
                        ("decimals", "0x313ce567"),
                        ("name", "0x06fdde03"),
                        ("ticker", "0x95d89b41"),
                    ] {
                        if let Ok(value) = response(http.post(rpc).json(&json!({"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{"to":m.contract,"data":selector},tag]}))).await {
                            if value["id"] != 1 || value["jsonrpc"] != "2.0" || !value["error"].is_null() { continue; }
                            observed = true;
                            if field == "decimals" {
                                m.rpc_decimals = value["result"].as_str().and_then(|s| s.strip_prefix("0x")).filter(|s| s.len() == 64).and_then(|s| u8::from_str_radix(s,16).ok());
                            } else {
                                let parsed = rpc_text(&value);
                                if field == "name" { m.name = m.name.take().or(parsed); }
                                else { m.ticker = m.ticker.take().or(parsed); }
                            }
                            evidence[field] = value;
                        }
                    }
                    if observed {
                        fetched = true;
                        m.sources.insert(rpc.into(), evidence);
                    }
                }
            }
        }
        let scale = store.asset_scale(id)?;
        if m.rpc_decimals.is_none() {
            m.warnings
                .push("Current RPC decimals unavailable; ledger scale preserved".into());
        }
        if m.rpc_decimals.is_some_and(|d| d != scale) || m.decimals.is_some_and(|d| d != scale) {
            m.warnings.push(
                "Metadata decimals conflict with protected ledger scale; amounts unchanged".into(),
            );
        }
        if m.ticker.is_none() {
            m.warnings.push("Token ticker missing".into());
        }
        if m.name.is_none() {
            m.warnings.push("Token name missing".into());
        }
        if let Some(url) = &m.icon_url {
            // No redirects, strict host allowlist, bounded PNG/JPEG only. SVG is never embedded.
            if let Ok(mut r) = http.get(url).send().await {
                if r.status().is_success() {
                    let mut bytes = vec![];
                    while let Ok(Some(chunk)) = r.chunk().await {
                        if bytes.len() + chunk.len() > 128_000 {
                            bytes.clear();
                            break;
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                        Some("image/png")
                    } else if bytes.starts_with(&[255, 216, 255]) {
                        Some("image/jpeg")
                    } else {
                        None
                    };
                    if let Some(mime) = mime {
                        m.icon_data = Some(format!(
                            "data:{mime};base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        ));
                    }
                }
            }
        }
        if m.icon_data.is_none() {
            m.warnings
                .push("Token icon unavailable in offline cache".into());
        }
        if fetched {
            m.fetched_at = Some(Utc::now());
        }
        store.save_token_metadata(&m)?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Ok(ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metadata() -> TokenMetadata {
        TokenMetadata {
            asset_id: AssetId::str("eip155:1/erc20:0x0cf0ee63788a0849fe5297f3407f701e122cc023"),
            chain_id: 1,
            contract: "0x0cf0ee63788a0849fe5297f3407f701e122cc023".into(),
            name: None,
            ticker: None,
            icon_url: None,
            icon_data: None,
            decimals: None,
            rpc_decimals: None,
            coingecko_id: None,
            fetched_at: None,
            sources: Default::default(),
            warnings: vec![],
        }
    }
    #[test]
    fn exact_identity_required_and_missing_is_not_zero() {
        let mut m = metadata();
        let contract = m.contract.clone();
        assert!(
            apply_blockscout(
                &mut m,
                &json!({"address_hash":contract,"type":"ERC-20","decimals":null})
            )
            .is_ok()
        );
        assert_eq!(m.decimals, None);
        assert!(
            apply_coingecko(
                &mut m,
                &json!({"symbol":"SAME","platforms":{"ethereum":"0xwrong"}})
            )
            .is_err()
        );
        assert_eq!(m.coingecko_id, None);
        let contract = m.contract.clone();
        apply_coingecko(&mut m, &json!({"id":"known","platforms":{"ethereum":contract},"detail_platforms":{"ethereum":{"decimal_place":0}}})).unwrap();
        assert_eq!(m.decimals, Some(0));
    }
    #[test]
    fn polygon_market_metadata_requires_polygon_contract_identity() {
        let mut m = metadata();
        m.chain_id = 137;
        let contract = m.contract.clone();
        assert!(
            apply_coingecko(
                &mut m,
                &json!({"id":"usd-coin","platforms":{"ethereum":contract}})
            )
            .is_err()
        );
        apply_coingecko(&mut m, &json!({"id":"usd-coin","platforms":{"polygon-pos":contract},"detail_platforms":{"polygon-pos":{"decimal_place":6}}})).unwrap();
        assert_eq!(m.coingecko_id.as_deref(), Some("usd-coin"));
        assert_eq!(m.decimals, Some(6));
    }
    #[test]
    fn icons_reject_active_and_untrusted_urls() {
        for url in [
            "javascript:alert(1)",
            "data:image/svg+xml,x",
            "https://localhost/x",
            "https://coin-images.coingecko.com.evil/x",
            "https://user:pass@assets.coingecko.com/x",
        ] {
            assert!(safe_icon(&json!(url)).is_none());
        }
        assert!(
            safe_icon(&json!(
                "https://coin-images.coingecko.com/coins/images/1/small/a.png"
            ))
            .is_some()
        );
        assert!(rpc_text(&json!({"result":"0x"})).is_none());
        assert_eq!(
            rpc_text(&json!({"result":format!("0x{}{}", "414243", "00".repeat(29))})),
            Some("ABC".into())
        );
    }
    #[test]
    fn polygon_cache_preserves_chain_identity_and_scale() -> Result<()> {
        let store = Store::in_memory()?;
        let mut m = metadata();
        m.chain_id = 137;
        m.asset_id = AssetId::str(&format!("eip155:137/erc20:{}", m.contract));
        store.save_asset_scale(&m.asset_id, 6)?;
        m.decimals = Some(6);
        store.save_token_metadata(&m)?;
        assert_eq!(store.token_metadata(&m.asset_id)?, Some(m.clone()));
        assert_eq!(store.asset_scale(&m.asset_id)?, 6);
        m.chain_id = 1;
        assert!(store.save_token_metadata(&m).is_err());
        Ok(())
    }
    #[test]
    fn offline_cache_preserves_manual_display_and_protected_scale() -> Result<()> {
        let store = Store::in_memory()?;
        let mut m = metadata();
        store.save_asset_scale(&m.asset_id, 8)?;
        let display = lib_core::ui_metadata::AssetDisplay {
            id: m.asset_id.clone(),
            name: Some("My token".into()),
            ticker: Some("LOCAL".into()),
            chain: Some("1".into()),
            contract: Some(m.contract.clone()),
            unresolved_label: None,
            evidence: "manual".into(),
        };
        store.save_asset_display(&display)?;
        m.decimals = Some(18);
        m.rpc_decimals = Some(18);
        store.save_token_metadata(&m)?;
        assert_eq!(store.token_metadata(&m.asset_id)?, Some(m.clone()));
        assert_eq!(store.asset_scale(&m.asset_id)?, 8);
        assert_eq!(store.asset_display(&m.asset_id)?, Some(display));
        m.contract = "0x1111111111111111111111111111111111111111".into();
        assert!(store.save_token_metadata(&m).is_err());
        Ok(())
    }
}
