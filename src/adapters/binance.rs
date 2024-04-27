use crate::models::{ExternalId, ProductId};
use binance_client::models::StakingPosition;

const PROVIDER_ID_BINANCE: &str = "binance";

pub struct Binance;
impl Binance {
    fn product_id(id: &str) -> ProductId {
        ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id))
    }
}

mod transform {
    use binance_client::models::StakingPosition;

    use crate::adapters::binance::{self, PROVIDER_ID_BINANCE};
    use crate::data::ASSETS;
    use crate::models::{
        AssetId, ExternalId, ExternalIdVal, Position, PositionId, Product, ProductId,
    };

    fn product_id(id: impl Into<ExternalIdVal>) -> ProductId {
        ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse resp to those types
    }
    fn position_id(id: impl Into<ExternalIdVal>) -> PositionId {
        PositionId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse biannce resp to this type
    }

    impl From<&StakingPosition> for Position {
        fn from(pos: &StakingPosition) -> Self {
            // TODO match product
            // let matched_product = MUT_PRODUCTS.read().
            // let product = Product::from(pos);

            Position {
                id: position_id(pos.positionId),
                product_id: Product::from(pos).id,
                amount: pos.amount.parse::<f64>().unwrap(),
                start_date: pos.purchaseTime,
                end_date: pos.interestEndDate,
            }
        }
    }
    impl From<&binance::StakingPosition> for Product {
        fn from(pos: &binance::StakingPosition) -> Self {
            let matched_asset = ASSETS.get(&AssetId(pos.asset.clone())).unwrap(); // TODO report errors somewhere

            Product {
                id: product_id(&pos.productId),
                asset_id: matched_asset.id.clone(),
                apy: (pos.apy.parse()).unwrap(),
            }
        }
    }
}

// #[derive(thiserror::Error, Debug)]
// pub enum BinanceErr {
//     #[error("failed deserializing resp body: {0}")]
//     DeserResp(reqwest::Error),
//     #[error("API error response: {0}")]
//     ApiErrResp(BinanceApiErr),
//     #[error("reqwest err: {0}")]
//     ReqwestErr(reqwest::Error),
// }
