use crate::analyze::standard_timing_stats;
use crate::model::{StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirstCallSummary {
    zome_call_timing: StandardTimingsStats,
}

pub(crate) async fn summarize_first_call(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "first_call");

    let frame = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load instrument data")?;

    SummaryOutput::new(
        summary,
        FirstCallSummary {
            zome_call_timing: standard_timing_stats(frame, "value", "10s", None)
                .context("Standard timing stats")?,
        },
    )
}
