use itertools::Itertools;
use sha3::Digest;
use wind_tunnel_summary_model::RunSummary;

pub fn latest_run_summaries_by_name_and_config(
    summary_runs: Vec<RunSummary>,
) -> Vec<(String, Vec<u8>, RunSummary)> {
    summary_runs
        .into_iter()
        .into_group_map_by(|summary| {
            let mut hasher = sha3::Sha3_256::new();
            if let Some(run_duration) = summary.run_duration {
                Digest::update(&mut hasher, run_duration.to_le_bytes());
            }
            summary
                .behaviours
                .iter()
                .sorted_by_key(|(k, _)| (*k).clone())
                .for_each(|(k, v)| {
                    Digest::update(&mut hasher, k.as_bytes());
                    Digest::update(&mut hasher, v.to_le_bytes());
                });
            summary
                .env
                .iter()
                .sorted_by_key(|(k, _)| (*k).clone())
                .for_each(|(k, v)| {
                    Digest::update(&mut hasher, k.as_bytes());
                    Digest::update(&mut hasher, v.as_bytes());
                });
            Digest::update(&mut hasher, summary.wind_tunnel_version.as_bytes());

            (
                summary.scenario_name.clone(),
                hasher.finalize()[..].to_vec(),
            )
        })
        .into_iter()
        .map(|((name, fingerprint), mut summaries)| {
            summaries.sort_by_key(|summary| summary.started_at);

            // Safe to unwrap because there must have been at least one element
            (name, fingerprint, summaries.last().unwrap().clone())
        })
        .collect::<Vec<_>>()
}
