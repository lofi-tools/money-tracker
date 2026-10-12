//! Export a read-only UI review snapshot from a validated temporary DB copy.
use anyhow::{Context, Result, ensure};
use lib_app::{
    config::{DATABASE_FILENAME, ResolvedConfig},
    portfolio::Selection,
    startup::OpenAction,
};
use money_tracker_desktop::{Session, Startup};
use serde_json::json;
use std::path::PathBuf;
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2 || (args.len() == 3 && args[2] == "--refresh-prices"),
        "usage: export_preview SOURCE.duckdb OUTPUT.json [--refresh-prices]"
    );
    let mut resolved = ResolvedConfig::system(None, None)?;
    let temporary = tempfile::tempdir()?;
    resolved.database = temporary.path().join(DATABASE_FILENAME);
    resolved.config_file = temporary.path().join("preview-config.json");
    let mut session = Session::new(resolved);
    let mut ready = session
        .open(OpenAction::CopyExisting(PathBuf::from(&args[0])))
        .await?;
    if args.len() == 3 {
        ready = session.refresh_prices().await?;
    }
    let Startup::Ready { data, .. } = &ready else {
        anyhow::bail!("preview database missing")
    };
    let mut trees = serde_json::Map::new();
    let mut details = serde_json::Map::new();
    for selection in data
        .assets
        .iter()
        .chain(&data.providers)
        .map(|e| e.selection.clone())
        .chain([Selection::Dust])
    {
        let tree = session.tree(data.snapshot_id, selection.clone())?;
        for node in &tree.nodes {
            details.insert(
                format!("{}:{}", serde_json::to_string(&selection)?, node.node.id),
                serde_json::to_value(session.details(
                    data.snapshot_id,
                    selection.clone(),
                    &node.node.id,
                )?)?,
            );
        }
        trees.insert(
            serde_json::to_string(&selection)?,
            serde_json::to_value(tree)?,
        );
    }
    session.close().await?;
    let output = PathBuf::from(&args[1]);
    std::fs::create_dir_all(output.parent().context("output directory required")?)?;
    std::fs::write(
        output,
        serde_json::to_vec(&json!({"startup":ready,"trees":trees,"details":details}))?,
    )?;
    println!("Local preview snapshot ready; source database preserved.");
    Ok(())
}
