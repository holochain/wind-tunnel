use crate::aggregator::HostMetricsAggregator;
use crate::model::SummaryOutput;
use crate::{analyze, query};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriteValidatedMustGetAgentActivitySummary {
    error_count: usize,
}

pub(crate) async fn summarize_write_validated_must_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "write_validated_must_get_agent_activity");

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        WriteValidatedMustGetAgentActivitySummary {
            error_count: query::zome_call_error_count(client, &summary).await?,
        },
        host_metrics,
    )
}
