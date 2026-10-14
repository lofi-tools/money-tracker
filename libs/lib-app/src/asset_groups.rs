//! Reviewed display groups. No asset equivalence, quantity conversion or price substitution.
#[derive(Clone, Copy)]
pub struct Representation {
    pub underlying: &'static str,
    pub name: &'static str,
    pub ticker: &'static str,
    pub source: &'static str,
}
pub fn representation(id: &str) -> Option<Representation> {
    let (underlying, name, ticker, source) = match id {
        "USD" | "USDT" | "USDC" | "USDX" | "xUSD" => (
            "USD",
            id_label(id),
            id_ticker(id),
            "legacy provider asset IDs; Nexo CSV currency mapping",
        ),
        "EUR" | "EURX" => (
            "EUR",
            "Euro",
            if id == "EURX" { "EURX" } else { "EUR" },
            "Nexo CSV currency mapping",
        ),
        "GBP" | "GBPX" => (
            "GBP",
            "Pound sterling",
            if id == "GBPX" { "GBPX" } else { "GBP" },
            "Nexo CSV currency mapping",
        ),
        "ETH" => ("ETH", "Ether", "ETH", "provider native asset ID"),
        "BNB" => ("BNB", "BNB", "BNB", "provider native asset ID"),
        "POL" | "MATIC" => (
            "POL",
            "Polygon",
            if id == "MATIC" { "MATIC" } else { "POL" },
            "reviewed Polygon native asset migration",
        ),
        "eip155:1/native" => (
            "ETH",
            "Ether · Ethereum",
            "ETH",
            "https://ethereum.org/developers/docs/apis/json-rpc/",
        ),
        "eip155:42161/native" => (
            "ETH",
            "Ether · Arbitrum",
            "ETH",
            "https://docs.arbitrum.io/",
        ),
        "eip155:324/native" => ("ETH", "Ether · zkSync", "ETH", "https://docs.zksync.io/"),
        "eip155:137/native" => (
            "POL",
            "POL · Polygon",
            "POL",
            "https://docs.polygon.technology/",
        ),
        "eip155:56/native" => (
            "BNB",
            "BNB · BNB Chain",
            "BNB",
            "https://docs.bnbchain.org/",
        ),
        "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => (
            "USD",
            "USD Coin · Ethereum",
            "USDC",
            "https://developers.circle.com/stablecoins/usdc-contract-addresses",
        ),
        "eip155:137/erc20:0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => (
            "USD",
            "USD Coin · Polygon",
            "USDC",
            "https://developers.circle.com/stablecoins/usdc-contract-addresses",
        ),
        "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831" => (
            "USD",
            "USD Coin · Arbitrum",
            "USDC",
            "https://developers.circle.com/stablecoins/usdc-contract-addresses",
        ),
        "eip155:324/erc20:0x1d17cbcf0d6d143135ae902365d2e5e2a16538d4" => (
            "USD",
            "USD Coin · zkSync",
            "USDC",
            "https://developers.circle.com/stablecoins/usdc-contract-addresses",
        ),
        "eip155:1/erc20:0xdac17f958d2ee523a2206206994597c13d831ec7" => (
            "USD",
            "Tether USD · Ethereum",
            "USDT",
            "https://tether.to/en/supported-protocols/",
        ),
        "eip155:1/erc20:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => (
            "ETH",
            "Wrapped Ether · Ethereum",
            "WETH",
            "https://developers.uniswap.org/docs/protocols/v3/deployments/v3-ethereum-deployments",
        ),
        "eip155:42161/erc20:0x82af49447d8a07e3bd95bd0d56f35241523fbab1" => (
            "ETH",
            "Wrapped Ether · Arbitrum",
            "WETH",
            "https://developers.uniswap.org/docs/protocols/v3/deployments/v3-arbitrum-deployments",
        ),
        _ => return None,
    };
    Some(Representation {
        underlying,
        name,
        ticker,
        source,
    })
}
fn id_label(id: &str) -> &'static str {
    match id {
        "USDT" => "Tether USD",
        "USDC" => "USD Coin",
        "USDX" => "Nexo USDX",
        "xUSD" => "Nexo USD credit",
        _ => "US dollar",
    }
}
fn id_ticker(id: &str) -> &'static str {
    match id {
        "USDT" => "USDT",
        "USDC" => "USDC",
        "USDX" => "USDX",
        "xUSD" => "xUSD",
        _ => "USD",
    }
}
pub fn group_label(id: &str) -> &str {
    id
}

/// Provider-qualified quantity classification, independent of pricing/grouping.
/// Nexo FiatX counts in currency quantities by the user's display convention;
/// this does not change ledger identity or assume market-price parity.
pub fn is_direct(provider: &str, id: &str) -> bool {
    if provider.eq_ignore_ascii_case("nexo") {
        if matches!(id, "EURX" | "GBPX" | "USDX") {
            return true;
        }
        if id == "USD" {
            return false;
        }
    }
    matches!(
        id,
        "USD"
            | "EUR"
            | "GBP"
            | "ETH"
            | "BNB"
            | "POL"
            | "MATIC"
            | "eip155:1/native"
            | "eip155:42161/native"
            | "eip155:324/native"
            | "eip155:137/native"
            | "eip155:56/native"
    )
}
