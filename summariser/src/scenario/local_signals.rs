use crate::analyze::{standard_ratio_stats, standard_timing_stats};
use crate::model::{StandardRatioStats, StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalSignalsSummary {
    send: StandardTimingsStats,
    recv: StandardTimingsStats,
    success_ratio: StandardRatioStats,
}

pub(crate) async fn summarize_local_signals(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "local_signals");

    let send_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.signal_batch_send")
            .await
            .context("Load send data")?;

    let recv_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.signal_batch_recv")
            .await
            .context("Load recv data")?;

    let success_ratio =
        query::query_custom_data(client.clone(), &summary, "wt.custom.signal_success_ratio")
            .await
            .context("Load success ratio")?;

    SummaryOutput::new(
        summary,
        LocalSignalsSummary {
            send: standard_timing_stats(send_frame, "value", None).context("Send timing stats")?,
            recv: standard_timing_stats(recv_frame, "value", None).context("Recv timing stats")?,
            success_ratio: standard_ratio_stats(success_ratio, "value")
                .context("Success ratio stats")?,
        },
    )
}
