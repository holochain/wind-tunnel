use itertools::Itertools;
use wind_tunnel_summary_model::RunSummary;

pub fn latest_run_summaries_by_name_and_config(
    summary_runs: Vec<RunSummary>,
) -> Vec<(String, String, RunSummary)> {
    summary_runs
        .into_iter()
        .into_group_map_by(|summary| (summary.scenario_name.clone(), summary.fingerprint()))
        .into_iter()
        .map(|((name, fingerprint), mut summaries)| {
            summaries.sort_by_key(|summary| summary.started_at);

            // Safe to unwrap because there must have been at least one element
            (name, fingerprint, summaries.last().unwrap().clone())
        })
        .collect::<Vec<_>>()
}
