use crate::analyze::standard_timing_stats;
use crate::model::{StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use serde::Serialize;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize)]
struct AppInstallSummary {
    first_install_time: f64,
    install_app_timing: StandardTimingsStats,
}

pub(crate) async fn summarize_app_install(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "app_install");

    let frame = query::query_instrument_data(client.clone(), &summary, "admin_install_app")
        .await
        .context("Load instrument data")?;

    let first = frame
        .column("value")?
        .get(0)
        .context("First")?
        .try_extract::<f64>()?;

    SummaryOutput::new(
        summary,
        AppInstallSummary {
            first_install_time: first,
            install_app_timing: standard_timing_stats(frame, "value", "10s", Some(1))
                .context("Standard timing stats")?,
        },
    )
}
