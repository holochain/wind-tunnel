use std::path::PathBuf;
use itertools::Itertools;
use wind_tunnel_summary_model::load_summary_runs;

fn main() {
    let summary_runs = load_summary_runs(PathBuf::from("run_summary.jsonl")).expect("Failed to load run summaries");

    // Note that this is just a simple selection strategy. If we have run scenarios with more than
    // one configuration, we might want to select multiple summaries per scenario name.
    let latest_summaries = summary_runs.into_iter().into_group_map_by(|summary| summary.scenario_name.clone()).into_iter().map(|(_, mut summaries)| {
        summaries.sort_by_key(|summary| summary.started_at);

        // Safe to unwrap because there must have been at least one element
        summaries.last().unwrap().clone()
    }).collect::<Vec<_>>();

    for summary in latest_summaries {
        println!("{:?}", summary);
    }
}
