use crate::models::traits::{IsProvider, Issuer3};
use crate::models::{
    Asset, AssetId, ExternalAssetId, Position, PositionId, Product, ProductId, ProviderId,
};
use binance_client::BinanceClient;
use typed_ids::Issuer;

const PROVIDER_ID_BINANCE: &str = "binance";

pub struct BinanceSvc {
    pub client: BinanceClient,
}
impl BinanceSvc {
    pub fn new() -> anyhow::Result<Self> {
        Ok(BinanceSvc {
            client: BinanceClient::new()?,
        })
    }
    pub fn new_asset(local_asset_id: &str, binance_asset_id: &str) -> Asset {
        Asset::new(local_asset_id).with_ext_id(ExternalAssetId::new::<BinanceSvc>(binance_asset_id))
    }
}
impl BinanceSvc {
    // TODO move this to Provider trait

    pub fn fetch_assets(&self) -> anyhow::Result<Vec<Asset>> {
        Ok(vec![BinanceSvc::new_asset("ETH", "ethereum")])
    }
    pub async fn fetch_products(&self) -> anyhow::Result<Vec<Product>> {
        let binance_products = self.client.list_staking_products().await?;

        let products = binance_products
            .into_iter()
            .map(|sp| Product {
                id: ProductId::from(&sp.project_id),
                asset_id: AssetId::from(sp.detail.asset), // TODO match to local asset ??
                apy: sp.detail.apy,
            })
            .collect();
        Ok(products)
    }

    pub async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>> {
        let binance_positions = self.client.list_staking_positions().await?;
        // TODO also simple flex and simple lock positions

        let positions = binance_positions
            .into_iter()
            // TODO impl TryFrom<BinanceModel> for each model
            .map(|sp| {
                Ok(Position {
                    id: PositionId::from(&(*sp.position_id).to_string()),
                    product_id: ProductId::from(&sp.product_id),
                    amount: sp.amount,
                    start_date: sp.purchase_time,
                    end_date: sp.interest_end_date,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(positions)
    }
}
#[async_trait::async_trait]
impl IsProvider for BinanceSvc {
    fn provider_id(&self) -> ProviderId {
        ProviderId::from(PROVIDER_ID_BINANCE)
    }
    async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>> {
        self.fetch_positions().await
    }
}
impl Issuer for BinanceSvc {
    fn issuer_id() -> &'static str {
        PROVIDER_ID_BINANCE
    }
}
impl Issuer3 for BinanceSvc {
    fn name() -> &'static str {
        PROVIDER_ID_BINANCE
    }
}
