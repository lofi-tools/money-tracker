//! Explicit corrected replay into an existing database; backup before invoking.
use anyhow::{Result, ensure};
use lib_app::adapters::nexo::NexoSvc;
use lib_core::Store;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(args.len() == 2, "usage: nexo-replay EXISTING_DB CSV");
    ensure!(
        std::path::Path::new(&args[0]).is_file(),
        "database must already exist"
    );
    let store = Store::open(&args[0])?;
    let result =
        NexoSvc::from_path(&args[1]).import_into_with_progress(&store, |done, total| {
            println!("Mapped {done}/{total} rows; commit pending.");
        })?;
    println!("Nexo replay: {result:?}; stable source IDs retained.");
    Ok(())
}
