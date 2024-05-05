use self::models2::{Asset, AssetId, ExternalAssetId, Position, PositionId, Product, ProviderId};
use self::traits::{IsProvider, Issuer3};
use crate::adapters::binance2::models2::ProductId;
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
    fn provider_id(&self) -> models2::ProviderId {
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

// pub type BinanceAssetId = ExternalId<Asset, String, BinanceSvc>;

pub mod models2 {
    use super::traits::Issuer3;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ProviderId {
        id: String,
    }
    impl From<&str> for ProviderId {
        fn from(name: &str) -> Self {
            ProviderId {
                id: name.to_string(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct PositionId {
        id: String,
    }
    impl<S: AsRef<str>> From<S> for PositionId {
        fn from(name: S) -> Self {
            PositionId {
                id: name.as_ref().to_string(),
            }
        }
    }

    pub struct AllPositions {
        pub positions: HashMap<PositionId, Position>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct AssetId {
        id: String,
    }
    impl<S: AsRef<str>> From<S> for AssetId {
        fn from(name: S) -> Self {
            AssetId {
                id: name.as_ref().to_string(),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Asset {
        pub id: AssetId,
        pub external_ids: HashMap<ProviderId, ExternalAssetId>,
    }
    impl Asset {
        pub fn new(id: &str) -> Self {
            Asset {
                id: AssetId::from(id),
                external_ids: HashMap::new(),
            }
        }
        pub fn with_ext_id(mut self, ext_id: ExternalAssetId) -> Self {
            self.external_ids.insert(ext_id.issuer_id(), ext_id);
            self
        }
        pub fn merge(&mut self, other: &Self) -> &mut Self {
            self.external_ids.extend(other.external_ids.clone());
            self
        }
    }

    #[derive(Debug, Clone)]
    pub struct ExternalAssetId {
        pub id: String,
        pub issuer_name: String,
        _asset: std::marker::PhantomData<Asset>,
    }
    impl ExternalAssetId {
        pub fn new<Issuer: Issuer3>(id: &str) -> Self {
            ExternalAssetId {
                id: id.to_string(),
                issuer_name: Issuer::name().to_string(),
                _asset: std::marker::PhantomData,
            }
        }
        pub fn issuer_id(&self) -> ProviderId {
            ProviderId::from(self.issuer_name.as_str())
        }
    }
    impl PartialEq for ExternalAssetId {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Eq for ExternalAssetId {}
    impl std::hash::Hash for ExternalAssetId {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.issuer_name.hash(state);
            self.id.hash(state);
        }
    }

    pub struct AllAssets {
        pub by_id: HashMap<AssetId, Asset>,
        // by_external_id: HashMap<ExternalAssetId, Asset>, // TODO use sqlite indexes instead of re-creating a database
    }
    impl AllAssets {
        pub fn new() -> Self {
            AllAssets {
                by_id: HashMap::new(),
                // by_external_id: HashMap::new(),
            }
        }
        pub fn upsert(&mut self, new: Asset) {
            let merge_into_by_id = self
                .by_id
                .get_mut(&new.id)
                .map(|existing| existing.merge(&new))
                .cloned()
                .unwrap_or(new.clone());
            self.by_id.insert(new.id, merge_into_by_id);
        }
        pub fn get(&self, id: &AssetId) -> Option<&Asset> {
            self.by_id.get(id)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Product {
        pub id: ProductId,
        pub asset_id: AssetId,
        pub apy: f64,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct ProductId {
        id: String,
        _type: std::marker::PhantomData<Product>,
    }
    impl<S: AsRef<str>> From<S> for ProductId {
        fn from(name: S) -> Self {
            ProductId {
                id: name.as_ref().to_string(),
                _type: std::marker::PhantomData,
            }
        }
    }

    pub struct AllProducts {
        pub products: HashMap<ProductId, Product>,
    }
    impl AllProducts {
        pub fn new() -> Self {
            AllProducts {
                products: HashMap::new(),
            }
        }
        pub fn insert(&mut self, product: Product) {
            self.products.insert(product.id.clone(), product);
        }
        pub fn get(&self, id: &ProductId) -> Option<&Product> {
            self.products.get(id)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Position {
        pub id: PositionId,
        pub product_id: ProductId,
        pub amount: f64,
        pub start_date: DateTime<Utc>,
        pub end_date: DateTime<Utc>,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::adapters::binance2::BinanceSvc;

        pub struct Provider2 {}
        impl Issuer3 for Provider2 {
            fn name() -> &'static str {
                "provider_2"
            }
        }

        #[test]
        fn test_upsert() -> anyhow::Result<()> {
            let mut assets = AllAssets::new();
            assets.upsert(
                Asset::new("asset_1")
                    .with_ext_id(ExternalAssetId::new::<BinanceSvc>("binance:asset_1")),
            );
            assets.upsert(
                Asset::new("asset_1").with_ext_id(ExternalAssetId::new::<Provider2>(
                    "some_other_name_for_same_asset",
                )),
            );

            // TODO assert there's 1 asset with 2 ext ids

            let asset = assets.get(&AssetId::from("asset_1")).unwrap();
            assert_eq!(asset.external_ids.len(), 2);

            Ok(())
        }
    }
}

pub mod traits {
    use std::future::Future;

    use super::models2::{Position, ProviderId};

    pub trait Issuer2: std::fmt::Debug {
        fn name(&self) -> &'static str;
    }

    // not object-safe
    pub trait Issuer3 {
        fn name() -> &'static str;
    }

    #[async_trait::async_trait] // makes async trait also object-safe
    pub trait IsProvider {
        // TODO load assets, attach external_ids

        fn provider_id(&self) -> ProviderId;

        // fn list_products(&self) -> anyhow::Result<AllProducts>;
        async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>>;
    }
}
