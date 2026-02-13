use crate::aggregator::HostMetricsAggregator;
use crate::model::SummaryOutput;
use anyhow::Context;
use futures::FutureExt;
use futures::future::BoxFuture;
use scenario::*;
use wind_tunnel_summary_model::RunSummary;

mod aggregator;
mod analyze;
mod frame;
pub mod model;
mod partition;
mod query;
pub mod scenario;

#[cfg(any(feature = "test_data", feature = "query_test_data"))]
pub mod test_data;

macro_rules! execute_report_with_host_metrics {
    ($client:ident, $summary:ident, $report_fn:ident) => {
        async move {
            let (host_metrics, scenario) = futures::join!(
                {
                    let client = $client.clone();
                    let summary = $summary.clone();

                    async move {
                        HostMetricsAggregator::new(&client, &summary)
                            .try_aggregate()
                            .await
                    }
                },
                $report_fn($client.clone(), $summary.clone())
            );

            SummaryOutput::new(
                $summary,
                scenario.context(stringify!($report_fn))?,
                host_metrics,
            )
        }
        .boxed()
    };
}

pub fn execute_report_for_run_summary(
    client: influxdb::Client,
    summary: RunSummary,
) -> Option<BoxFuture<'static, anyhow::Result<SummaryOutput>>> {
    let name = &summary.scenario_name;

    let client = client.clone();

    match name.as_str() {
        "app_install" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_app_install
        )),
        "dht_sync_lag" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_dht_sync_lag
        )),
        "first_call" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_first_call
        )),
        "full_arc_create_validated_zero_arc_read" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_full_arc_create_validated_zero_arc_read
        )),
        "local_signals" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_local_signals
        )),
        "mixed_arc_get_agent_activity" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_mixed_arc_get_agent_activity
        )),
        "mixed_arc_must_get_agent_activity" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_mixed_arc_must_get_agent_activity
        )),
        "remote_call_rate" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_remote_call_rate
        )),
        "remote_signals" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_remote_signals
        )),
        "single_write_many_read" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_single_write_many_read
        )),
        "two_party_countersigning" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_countersigning_two_party
        )),
        "validation_receipts" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_validation_receipts
        )),
        "write_query" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_query
        )),
        "write_read" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_read
        )),
        "write_validated" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_validated
        )),
        "zome_call_single_value" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_zome_call_single_value
        )),
        "write_get_agent_activity" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_get_agent_activity
        )),
        "write_get_agent_activity_volatile" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_get_agent_activity_volatile
        )),
        "write_validated_must_get_agent_activity" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_write_validated_must_get_agent_activity
        )),
        "zero_arc_create_data" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_zero_arc_create_data
        )),
        "zero_arc_create_data_validated" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_zero_arc_create_data_validated
        )),
        "zero_arc_create_and_read" => Some(execute_report_with_host_metrics!(
            client,
            summary,
            summarize_zero_arc_create_and_read
        )),
        _ => {
            log::warn!("No report for scenario: {name}");
            None
        }
    }
}
