// Exact, reviewed display-only icon mappings. Never select a contract icon by ticker.
const eth = new URL("./assets/coins/eth.svg", import.meta.url).href;
const nexo = new URL("./assets/coins/nexo.svg", import.meta.url).href;
const near = new URL("./assets/coins/near.svg", import.meta.url).href;
const bnb = new URL("./assets/coins/bnb.svg", import.meta.url).href;
const dot = new URL("./assets/coins/dot.svg", import.meta.url).href;
const pol = new URL("./assets/coins/matic.svg", import.meta.url).href;
const usdt = new URL("./assets/coins/usdt.svg", import.meta.url).href;
const usdc = new URL("./assets/coins/usdc.svg", import.meta.url).href;
const usd = new URL("./assets/coins/usd.svg", import.meta.url).href;
const eur = new URL("./assets/coins/eur.svg", import.meta.url).href;
const gbp = new URL("./assets/coins/gbp.svg", import.meta.url).href;
const btc = new URL("./assets/coins/btc.svg", import.meta.url).href;
const icons: Record<string, string> = {
  ETH: eth,
  NEXO: nexo,
  NEAR: near,
  BNB: bnb,
  DOT: dot,
  POL: pol,
  MATIC: pol,
  USDT: usdt,
  USDC: usdc,
  USD: usd,
  USDX: usd,
  xUSD: usd,
  EUR: eur,
  EURX: eur,
  GBP: gbp,
  GBPX: gbp,
  BTC: btc,
  "eip155:1/native": eth,
  "eip155:42161/native": eth,
  "eip155:324/native": eth,
  "eip155:56/native": bnb,
  "eip155:137/native": pol,
  "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": usdc,
  "eip155:137/erc20:0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": usdc,
  "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831": usdc,
  "eip155:324/erc20:0x1d17cbcf0d6d143135ae902365d2e5e2a16538d4": usdc,
  "eip155:1/erc20:0xdac17f958d2ee523a2206206994597c13d831ec7": usdt,
  "eip155:1/erc20:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2": eth,
  "eip155:42161/erc20:0x82af49447d8a07e3bd95bd0d56f35241523fbab1": eth,
};
const reviewedTickers: Record<string, string> = {
  "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": "USDC",
  "eip155:137/erc20:0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": "USDC",
  "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831": "USDC",
  "eip155:324/erc20:0x1d17cbcf0d6d143135ae902365d2e5e2a16538d4": "USDC",
  "eip155:1/erc20:0xdac17f958d2ee523a2206206994597c13d831ec7": "USDT",
  "eip155:1/erc20:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2": "WETH",
  "eip155:42161/erc20:0x82af49447d8a07e3bd95bd0d56f35241523fbab1": "WETH",
};
export const assetIcon = (id: string) => icons[id];
export function shortAssetLabel(id: string): string {
  if (reviewedTickers[id]) return reviewedTickers[id];
  if (!id.startsWith("eip155:")) return id;
  const [chain, asset] = id.slice(7).split("/");
  const network: Record<string, string> = {
    "1": "Ethereum",
    "137": "Polygon",
    "56": "BNB Chain",
    "42161": "Arbitrum",
    "324": "zkSync",
  };
  const nativeTicker: Record<string, string> = {
    "1": "ETH",
    "42161": "ETH",
    "324": "ETH",
    "56": "BNB",
    "137": "POL",
  };
  if (asset === "native") return nativeTicker[chain] ?? "Native token";
  const contract = asset?.replace(/^erc20:/, "") ?? "";
  return `${network[chain] ?? "Unknown chain"} token ${contract.slice(0, 6)}…${contract.slice(-4)}`;
}
export function readableReference(reference: string): string {
  return reference
    .replace(/eip155:\d+\/(?:erc20:0x[a-fA-F0-9]{40}|native)/g, shortAssetLabel)
    .replace(
      /eip155:(\d+):(?:0x)?([a-fA-F0-9]{40})/g,
      (_, chain, address) =>
        `Chain ${chain} wallet 0x${address.slice(0, 4)}…${address.slice(-4)}`,
    );
}
