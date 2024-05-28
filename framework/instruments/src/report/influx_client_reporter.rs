use crate::report::{ReportCollector, ReportMetric};
use crate::OperationRecord;
use anyhow::Context;
use influxdb::{Client, WriteQuery};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::report::influx_reporter_base::InfluxReporterBase;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use wind_tunnel_core::prelude::DelegatedShutdownListener;

/// Write metrics directly to InfluxDB using the InfluxDB client.
///
/// Using this reporter takes more resources from the current process but requires less
/// infrastructure to run because Telegraf is not needed. This is useful if you need to work
/// with InfluxDB locally. When running distributed it is recommended to use the
/// [InfluxFileReportCollector](crate::report::InfluxFileReportCollector).
pub struct InfluxClientReportCollector {
    inner: InfluxReporterBase,
}

impl InfluxClientReportCollector {
    pub fn new(
        runtime: &Runtime,
        shutdown_listener: DelegatedShutdownListener,
        run_id: String,
        scenario_name: String,
    ) -> anyhow::Result<Self> {
        let client = Client::new(
            std::env::var("INFLUX_HOST").context(
                "Cannot configure metrics reporter without environment variable `INFLUX_HOST`",
            )?,
            std::env::var("INFLUX_BUCKET").context(
                "Cannot configure metrics reporter without environment variable `INFLUX_BUCKET`",
            )?,
        )
        .with_token(std::env::var("INFLUX_TOKEN").context(
            "Cannot configure metrics reporter without environment variable `INFLUX_TOKEN`",
        )?);

        let flush_complete = Arc::new(AtomicBool::new(false));
        let (join_handle, writer) =
            start_metrics_write_task(runtime, shutdown_listener, client, flush_complete.clone());

        Ok(Self {
            inner: InfluxReporterBase::new(
                run_id,
                scenario_name,
                join_handle,
                writer,
                flush_complete,
            ),
        })
    }
}

impl ReportCollector for InfluxClientReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.inner.add_operation(operation_record);
    }

    fn add_custom(&mut self, metric: ReportMetric) {
        self.inner.add_custom(metric);
    }

    fn finalize(&self) {
        self.inner.finalize();
    }
}

fn start_metrics_write_task(
    runtime: &Runtime,
    mut shutdown_listener: DelegatedShutdownListener,
    client: Client,
    flush_complete: Arc<AtomicBool>,
) -> (JoinHandle<()>, UnboundedSender<WriteQuery>) {
    let (writer, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let join_handle = runtime.spawn(async move {
        loop {
            select! {
                _ = shutdown_listener.wait_for_shutdown() => {
                    log::debug!("Shutting down metrics reporter");
                    break;
                }
                query = receiver.recv() => {
                    if let Some(query) = query {
                        if let Err(e) = client.query(query).await {
                            log::warn!("Failed to send metric to InfluxDB: {}", e);
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        log::debug!("Draining any remaining metrics before shutting down...");
        let mut drain_count = 0;

        // Drain remaining metrics before shutting down
        while let Ok(query) = receiver.try_recv() {
            if let Err(e) = client.query(query).await {
                log::warn!("Failed to send metric to InfluxDB: {}", e);
            }
            drain_count += 1;

            if drain_count % 1000 == 0 {
                log::debug!("Drained {} remaining metrics", drain_count);
            }
        }

        log::debug!("Drained {} remaining metrics", drain_count);

        flush_complete.store(true, Ordering::Relaxed);
    });

    (join_handle, writer)
}
