use crate::models::{ListProducts, Product, ProductId};
use anyhow::anyhow;
// use std::time::Duration;
use derive_more::From;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;

#[derive(Debug, From)]
enum DbCmd {
    // Get { key: String },
    // Set { key: String, val: String },
    GetProduct(CmdGetProduct),
    InsertProduct(CmdInsertProduct),
}
#[derive(Debug)]
pub struct CmdGetProduct {
    product_id: ProductId,
    tx_resp: oneshot::Sender<Result<Product, anyhow::Error>>,
}
#[derive(Debug)]
pub struct CmdInsertProduct {
    product: Product,
    // tx_resp: oneshot::Sender<Result<(), anyhow::Error>>,
}

pub struct DB {
    products: ListProducts,
}
impl DB {
    pub fn new() -> Self {
        DB {
            products: ListProducts::new(),
        }
    }
    pub fn spawn() -> DbRef {
        let (tx_req, mut rx_resp) = mpsc::channel(32);
        let manager = tokio::spawn(async move {
            let mut db = Self::new();
            // Loop on receiving messages
            while let Some(cmd) = rx_resp.recv().await {
                match cmd {
                    DbCmd::GetProduct(cmd) => db.get_product(cmd).await,
                    DbCmd::InsertProduct(cmd) => db.insert_product(cmd).await,
                }
            }
        });
        DbRef { tx_req }
    }
    async fn get_product<'a>(&'a self, cmd: CmdGetProduct) {
        let got = self
            .products
            .get(&cmd.product_id)
            .cloned()
            .ok_or(anyhow!("not found"));
        cmd.tx_resp.send(got);
        // tokio::time::sleep(Duration::from_millis(400)).await;
    }
    async fn insert_product(&mut self, cmd: CmdInsertProduct) {
        self.products.insert(cmd.product);
    }
}

pub struct DbRef {
    tx_req: Sender<DbCmd>,
}
impl DbRef {
    pub async fn get_product(&self, product_id: ProductId) -> Result<Product, anyhow::Error> {
        let (tx_resp, rx_resp) = oneshot::channel();
        self.tx_req
            .send(DbCmd::GetProduct(CmdGetProduct {
                product_id,
                tx_resp,
            }))
            .await?;
        let read_res = rx_resp.await?;
        read_res
    }
    pub async fn insert_product(&self, product: &Product) -> Result<(), anyhow::Error> {
        Ok(self
            .tx_req
            .send(
                CmdInsertProduct {
                    product: product.clone(),
                }
                .into(),
            )
            .await?)
    }
}
