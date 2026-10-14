//! Read-only counts of current-at-export allocations and usable observed rates.
use anyhow::Result;
use lib_app::adapters::nexo::allocations::open_allocations;
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: nexo-current-allocations CSV"))?;
    let rows = nexo_csv::NexoCsv::from_file(path)?;
    let cutoff = rows
        .iter()
        .map(|r| r.date_time_utc)
        .max()
        .ok_or_else(|| anyhow::anyhow!("empty CSV"))?;
    let parts = open_allocations(
        &rows,
        &lib_core::AccountId::from("legacy:nexo"),
        "diagnostic",
        cutoff,
    )?;
    println!(
        "Open allocation rows: {}; represented fixed terms: {}; unique: {}; unresolved groups: {}; usable observed predecessor APYs: {}",
        parts.len(),
        parts.iter().map(|p| p.count).sum::<usize>(),
        parts.iter().filter(|p| p.kind == "fixed").count(),
        parts
            .iter()
            .filter(|p| p.kind == "unresolved_fixed")
            .count(),
        parts.iter().filter(|p| p.observed_apy.is_some()).count()
    );
    Ok(())
}
