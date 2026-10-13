//! Read-only aggregate diagnostic; no IDs, amounts or private row text printed.
use lib_app::adapters::nexo::term_inference::infer_terms;
use nexo_csv::NexoCsv;
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        anyhow::anyhow!("usage: nexo-term-inference CSV [nearby-seconds, default 0]")
    })?;
    let window = args
        .next()
        .map(|v| v.parse::<i64>())
        .transpose()?
        .unwrap_or(0);
    anyhow::ensure!(args.next().is_none(), "unexpected argument");
    let rows = NexoCsv::from_file(path)?;
    let report = infer_terms(&rows, window)?;
    let closed = report
        .terms
        .iter()
        .filter_map(|t| t.close.as_ref())
        .collect::<Vec<_>>();
    println!(
        "nearby window: {window} seconds; lock candidates: {}; inferred closed: {}; ambiguous locks: {}; unresolved unlocks: {}; unresolved rewards: {}",
        report.terms.len(),
        closed.len(),
        report.terms.iter().filter(|t| t.ambiguous).count(),
        report.unresolved_unlocks.len(),
        report.unresolved_rewards.len()
    );
    println!(
        "linked fixed rewards: {}; same-asset exact returns: {}; finite annual equivalents: {}",
        closed.iter().filter(|c| c.reward.is_some()).count(),
        closed
            .iter()
            .filter(|c| c.reward.as_ref().is_some_and(|r| r.period_return.is_some()))
            .count(),
        closed
            .iter()
            .filter(|c| c
                .reward
                .as_ref()
                .is_some_and(|r| r.equivalent_apy_estimate.is_some()))
            .count()
    );
    Ok(())
}
