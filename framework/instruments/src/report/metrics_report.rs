use crate::report::ReportCollector;
use crate::OperationRecord;
use anyhow::Context;
use influxdb::{Client, InfluxDbWriteable, Timestamp, WriteQuery};
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use wind_tunnel_core::prelude::DelegatedShutdownListener;

pub struct MetricsReportCollector {
    pub writer: UnboundedSender<influxdb::WriteQuery>,
}

impl MetricsReportCollector {
    pub fn new(
        runtime: &Runtime,
        shutdown_listener: DelegatedShutdownListener,
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

        let writer = start_metrics_write_task(runtime, shutdown_listener, client);

        Ok(Self { writer })
    }
}

impl ReportCollector for MetricsReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        let mut query = Timestamp::Nanoseconds(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .into_query("wt.instruments.operation_duration")
        .add_field(
            "value",
            operation_record
                .elapsed
                .expect("OperationRecord must have an elapsed time")
                .as_micros() as f64
                / 1000.0,
        )
        .add_tag("operation_id", operation_record.operation_id.to_string())
        .add_tag("is_error", operation_record.is_error.to_string());

        for (k, v) in &operation_record.attr {
            query = query.add_tag(k, v.to_string());
        }

        self.writer.send(query).unwrap();
    }

    fn finalize(&self) {
        // Not required for metrics currently.
    }
}

fn start_metrics_write_task(
    runtime: &Runtime,
    mut shutdown_listener: DelegatedShutdownListener,
    client: Client,
) -> UnboundedSender<WriteQuery> {
    let (writer, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    runtime.spawn(async move {
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

        log::trace!("Draining any remaining metrics before shutting down...");
        let mut drain_count = 0;

        // Drain remaining metrics before shutting down
        while let Ok(query) = receiver.try_recv() {
            if let Err(e) = client.query(query).await {
                log::warn!("Failed to send metric to InfluxDB: {}", e);
            }
            drain_count += 1;
        }

        log::debug!("Drained {} remaining metrics", drain_count);
    });
    writer
}
