use crate::models::traits::{IsProvider, Issuer3};
use crate::models::{
    AssetId, ExternalAssetId, Position, PositionId, Product, ProductId, ProviderId, Transaction,
};
use binance_client::BinanceClient;

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
    // pub fn new_asset(local_asset_id: &str, binance_asset_id: &str) -> AssetId {
    //     AssetId::new(local_asset_id)
    //         .with_ext_id(ExternalAssetId::new::<BinanceSvc>(binance_asset_id))
    // }
}
impl BinanceSvc {
    // TODO move this to Provider trait

    // pub fn fetch_assets(&self) -> anyhow::Result<Vec<AssetId>> {
    //     Ok(vec![BinanceSvc::new_asset("ETH", "ethereum")])
    // }
    pub async fn fetch_products(&self) -> anyhow::Result<Vec<Product>> {
        let binance_products = self.client.list_staking_products().await?;

        let products = binance_products
            .into_iter()
            .map(|sp| Product {
                id: ProductId::from(&sp.project_id),
                asset_id: AssetId::from_binance(&sp.detail.asset),
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
    async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        todo!()
    }
}
impl Issuer3 for BinanceSvc {
    fn name() -> &'static str {
        PROVIDER_ID_BINANCE
    }
}

impl AssetId {
    fn from_binance(binance_asset: &str) -> Self {
        match binance_asset {
            "ethereum" => AssetId::Eth,
            _ => AssetId::unknown(binance_asset),
        }
    }
}

pub mod old {
    // use crate::models::{ExternalId, ProductId};
    // use binance_client::payloads::StakingPositionResp;

    // const PROVIDER_ID_BINANCE: &str = "binance";

    // pub struct Binance;
    // impl Binance {
    //     fn _product_id(id: &str) -> ProductId {
    //         ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id))
    //     }
    // }

    // mod transform {
    //     use crate::adapters::binance::{self, PROVIDER_ID_BINANCE};
    //     use crate::data::ASSETS;
    //     use crate::models::{
    //         AssetId, ExternalId, ExternalIdVal, Position, PositionId, Product, ProductId,
    //     };
    //     use binance_client::payloads::StakingPositionResp;

    //     fn product_id(id: impl Into<ExternalIdVal>) -> ProductId {
    //         ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse resp to those types
    //     }
    //     fn position_id(id: impl Into<ExternalIdVal>) -> PositionId {
    //         PositionId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse biannce resp to this type
    //     }

    //     impl From<&StakingPositionResp> for Position {
    //         fn from(pos: &StakingPositionResp) -> Self {
    //             // TODO match product
    //             // let matched_product = MUT_PRODUCTS.read().
    //             // let product = Product::from(pos);

    //             Position {
    //                 id: position_id(pos.position_id),
    //                 product_id: Product::from(pos).id,
    //                 amount: pos.amount.parse::<f64>().unwrap(),
    //                 start_date: pos.purchaseTime,
    //                 end_date: pos.interestEndDate,
    //             }
    //         }
    //     }
    //     impl From<&binance::StakingPositionResp> for Product {
    //         fn from(pos: &binance::StakingPositionResp) -> Self {
    //             let matched_asset = ASSETS.get(&AssetId(pos.asset.clone())).unwrap(); // TODO report errors somewhere

    //             Product {
    //                 id: product_id(&pos.product_id),
    //                 asset_id: matched_asset.id.clone(),
    //                 apy: (pos.apy.parse()).unwrap(),
    //             }
    //         }
    //     }
    // }

    // // #[derive(thiserror::Error, Debug)]
    // // pub enum BinanceErr {
    // //     #[error("failed deserializing resp body: {0}")]
    // //     DeserResp(reqwest::Error),
    // //     #[error("API error response: {0}")]
    // //     ApiErrResp(BinanceApiErr),
    // //     #[error("reqwest err: {0}")]
    // //     ReqwestErr(reqwest::Error),
    // // }
}
