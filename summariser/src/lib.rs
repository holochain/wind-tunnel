use crate::model::SummaryOutput;
use anyhow::Context;
use futures::future::BoxFuture;
use futures::FutureExt;
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

pub fn execute_report_for_run_summary(
    client: influxdb::Client,
    summary: RunSummary,
) -> Option<BoxFuture<'static, anyhow::Result<SummaryOutput>>> {
    let name = &summary.scenario_name;

    let client = client.clone();
    match name.as_str() {
        "app_install" => Some(
            async move {
                summarize_app_install(client.clone(), summary.clone())
                    .await
                    .context("App install summary")
            }
            .boxed(),
        ),
        "dht_sync_lag" => Some(
            async move {
                summarize_dht_sync_lag(client.clone(), summary.clone())
                    .await
                    .context("DHT sync lag summary")
            }
            .boxed(),
        ),
        "first_call" => Some(
            async move {
                summarize_first_call(client.clone(), summary.clone())
                    .await
                    .context("First call summary")
            }
            .boxed(),
        ),
        "local_signals" => Some(
            async move {
                summarize_local_signals(client.clone(), summary.clone())
                    .await
                    .context("Local signals summary")
            }
            .boxed(),
        ),
        "remote_call_rate" => Some(
            async move {
                summarize_remote_call_rate(client.clone(), summary.clone())
                    .await
                    .context("Remote call rate summary")
            }
            .boxed(),
        ),
        "single_write_many_read" => Some(
            async move {
                summarize_single_write_many_read(client.clone(), summary.clone())
                    .await
                    .context("Single write many read summary")
            }
            .boxed(),
        ),
        "two_party_countersigning" => Some(
            async move {
                summarize_countersigning_two_party(client.clone(), summary.clone())
                    .await
                    .context("Countersigning, two party, summary")
            }
            .boxed(),
        ),
        "validation_receipts" => Some(
            async move {
                summarize_validation_receipts(client.clone(), summary.clone())
                    .await
                    .context("Validation receipts summary")
            }
            .boxed(),
        ),
        "write_query" => Some(
            async move {
                summarize_write_query(client.clone(), summary.clone())
                    .await
                    .context("Write query summary")
            }
            .boxed(),
        ),
        "write_read" => Some(
            async move {
                summarize_write_read(client.clone(), summary.clone())
                    .await
                    .context("Write read summary")
            }
            .boxed(),
        ),
        "write_validated" => Some(
            async move {
                summarize_write_validated(client.clone(), summary.clone())
                    .await
                    .context("Write validated summary")
            }
            .boxed(),
        ),
        "zome_call_single_value" => Some(
            async move {
                summarize_zome_call_single_value(client.clone(), summary.clone())
                    .await
                    .context("Zome call single value summary")
            }
            .boxed(),
        ),
        "write_get_agent_activity" => Some(
            async move {
                summarize_write_get_agent_activity(client.clone(), summary.clone())
                    .await
                    .context("Agent activity summary")
            }
            .boxed(),
        ),
        "write_validated_must_get_agent_activity" => Some(
            async move {
                summarize_write_validated_must_get_agent_activity(client.clone(), summary.clone())
                    .await
                    .context("Write validated must_get_agent_activity summary")
            }
            .boxed(),
        ),
        _ => {
            log::warn!("No report for scenario: {}", name);
            None
        }
    }
}
