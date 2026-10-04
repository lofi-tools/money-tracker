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
pub struct Employee {
    pub name: String,
    pub age: u32,
    pub salary: f64,
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
