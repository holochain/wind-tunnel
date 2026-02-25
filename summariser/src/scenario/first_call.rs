use crate::analyze::standard_timing_stats;
use crate::model::StandardTimingsStats;
use crate::query;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FirstCallSummary {
    /// First zome call to a newly installed app, in seconds.
    ///
    /// Note that the app is uninstalled and re-installed between each call - so that each call is
    /// supposed to measure a cold start of the WASM and force it to be compiled and loaded.
    zome_call_timing: StandardTimingsStats,
}

pub(crate) async fn summarize_first_call(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<FirstCallSummary> {
    assert_eq!(summary.scenario_name, "first_call");

    let frame = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load instrument data")?;

    Ok(FirstCallSummary {
        zome_call_timing: standard_timing_stats(frame, "value", "10s", None)
            .context("Standard timing stats")?,
    })
}
