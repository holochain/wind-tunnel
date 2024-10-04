use crate::model::SummaryOutput;
use wind_tunnel_summary_model::RunSummary;

pub(crate) async fn summarize_countersigning_two_party(
    _client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "two_party_countersigning");

    SummaryOutput::new(summary, serde_json::Value::Bool(true))
}
