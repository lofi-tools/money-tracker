use tokio::sync::mpsc::{self, Sender};

use crate::models::{ListProducts, Product, ProductId};

#[derive(Debug)]
enum Command {
    // Get { key: String },
    // Set { key: String, val: String },
    GetProduct { product_id: ProductId },
    InsertProduct { product: Product },
}

pub struct DB {
    products: ListProducts,
}
impl DB {
    pub fn spawn() -> DbRef {
        let (tx, mut rx) = mpsc::channel(32);
        let manager = tokio::spawn(async move {
            // Establish a connection to the server
            // let mut client = client::connect("127.0.0.1:6379").await.unwrap();

            // Start receiving messages
            while let Some(cmd) = rx.recv().await {
                use Command::*;

                match cmd {
                    GetProduct { product_id } => todo!(),
                    InsertProduct { product } => todo!(),
                    // Get { key } => {
                    //     client.get(&key).await;
                    // }
                    // Set { key, val } => {
                    //     client.set(&key, val).await;
                    // }
                }
            }
        });
        DbRef { tx } // TODO
    }
}

pub struct DbRef {
    // store channels, be Clone, have methods to ask to get/set
    tx: Sender<Command>,
}
