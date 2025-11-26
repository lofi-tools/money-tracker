// #![feature(map_try_insert)] // for try_insert in models::AssetPrice
// use crate::adapters::binance::BinanceSvc;
// use crate::adapters::coingecko;
use crate::adapters::nexo::NexoSvc;
use crate::models::Db;
use adapters::coingecko::CoinGeckoSvc;
use models::traits::IsProvider;

pub mod adapters {
    pub mod binance;
    pub mod coingecko;
    pub mod nexo;
}
mod models;

// TODO - estimate value, epy
// TODO - binance: other assets
// TODO - AAX ??
// TODO - NEXO ??
// TODO -

// GOALS - total value estimate
// GOALS - earn per year estimate (per asset breakdown of epy)
// GOALS - idle assets
// GOALS - movable assets (how much and when ??)
// GOALS - plot principal, interest, income
// GOALS -

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv_override().ok();
    // std::env::set_var("RUST_BACKTRACE", "1");

    let mut db = Db::new();

    let providers: Vec<Box<dyn IsProvider>> = vec![
        // Box::new(BinanceSvc::new()?)
        Box::new(NexoSvc::new()?),
    ];
    for provider in providers {
        // let positions = provider.fetch_positions().await?;
        // for position in positions {
        //     dbg!(position);
        // }
        let transactions = provider.fetch_transactions().await?;
    }

    let coingecko_svc = CoinGeckoSvc::new()?;
    let prices = coingecko_svc.fetch_current_prices().await?;
    dbg!(prices);

    // bc.list_staking_positions().await?;
    // data::fetch_assets();
    // data::fetch_binance().await?;

    // println!("PRODUCTS:");
    // for _product in PRODUCTS.read().unwrap().map.iter() {
    //     // println!("{}", product.1);
    // }
    // println!("POSITIONS:");
    // for _pos in POSITIONS.read().unwrap().by_id.iter() {
    //     // println!("{}", pos.1);
    // }

    // data::positions_groupby_currency();

    // coingecko::CurrentPriceReq::fetch_prices().await?;

    Ok(())
}

pub mod polars {
    use polars::{frame::row::Row, prelude::*};

    pub trait ToRow {
        fn to_row(&self) -> Row;
        fn schema() -> Schema;
    }
    pub trait VecExt {
        fn to_rows(&self) -> Vec<Row>;
    }
    impl<T: ToRow> VecExt for Vec<T> {
        fn to_rows(&self) -> Vec<Row> {
            self.iter().map(|t| t.to_row().clone()).collect()
        }
    }

    #[derive(Debug)]
    struct Employee {
        name: String,
        age: u32,
        salary: f64,
    }
    impl ToRow for Employee {
        fn to_row(&self) -> Row {
            Row::new(vec![
                AnyValue::String(&self.name),
                AnyValue::UInt32(self.age),
                AnyValue::Float64(self.salary),
            ])
        }
        fn schema() -> Schema {
            let mut schema = Schema::new();
            schema.with_column("name".into(), DataType::String);
            schema.with_column("age".into(), DataType::UInt32);
            schema.with_column("salary".into(), DataType::Float64);
            schema
        }
    }

    #[test]
    pub fn ext_test_polars() -> anyhow::Result<()> {
        let employees = vec![
            Employee {
                name: "Alice".to_string(),
                age: 30,
                salary: 50000.0,
            },
            Employee {
                name: "Bob".to_string(),
                age: 35,
                salary: 60000.0,
            },
            Employee {
                name: "Charlie".to_string(),
                age: 40,
                salary: 70000.0,
            },
        ];

        let df =
            DataFrame::from_rows_iter_and_schema(employees.to_rows().iter(), &Employee::schema())?;
        dbg!(df);

        Ok(())
    }
}
