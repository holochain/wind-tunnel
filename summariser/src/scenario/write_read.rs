use crate::analyze::{partition_into_map, standard_rate, standard_timing_stats};
use crate::model::{StandardRateStats, StandardTimingsStats};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteQuerySummary {
    /// Duration of `create_sample_entry` zome calls (seconds)
    write_timing: StandardTimingsStats,
    /// Rate of `create_sample_entry` zome calls per 10-second window
    write_rate: StandardRateStats,
    /// Duration of `get_sample_entry` zome calls (seconds)
    read_timing: StandardTimingsStats,
    /// Rate of `get_sample_entry` zome calls per 10-second window
    read_rate: StandardRateStats,
    /// Number of zome call errors observed during the run
    errors: usize,
}

pub(crate) async fn summarize_write_read(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteQuerySummary> {
    assert_eq!(summary.scenario_name, "write_read");

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let mut by_fn = partition_into_map(zome_calls, "fn_name")?;
    let create_zome_calls = by_fn
        .remove("create_sample_entry")
        .context("No create_sample_entry calls found")?;
    let get_zome_calls = by_fn
        .remove("get_sample_entry")
        .context("No get_sample_entry calls found")?;

    Ok(WriteQuerySummary {
        write_timing: standard_timing_stats(create_zome_calls.clone(), "value", "10s", None)
            .context("Write timing stats")?,
        write_rate: standard_rate(create_zome_calls, "value", "10s").context("Write rate")?,
        read_timing: standard_timing_stats(get_zome_calls.clone(), "value", "10s", None)
            .context("Read timing stats")?,
        read_rate: standard_rate(get_zome_calls, "value", "10s").context("Read rate")?,
        errors: zome_call_error_count(client.clone(), &summary)
            .await
            .context("Load zome call error data")?,
    })
}
