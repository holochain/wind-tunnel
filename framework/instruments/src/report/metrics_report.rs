use crate::report::{ReportCollector, ReportMetric};
use crate::OperationRecord;
use anyhow::Context;
use influxdb::{Client, InfluxDbWriteable, Timestamp, WriteQuery};
use influxive_core::DataType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use wind_tunnel_core::prelude::DelegatedShutdownListener;

trait DataTypeExt {
    fn into_type(self) -> influxdb::Type;
}

impl DataTypeExt for DataType {
    fn into_type(self) -> influxdb::Type {
        match self {
            DataType::Bool(b) => influxdb::Type::Boolean(b),
            DataType::F64(f) => influxdb::Type::Float(f),
            DataType::I64(i) => influxdb::Type::SignedInteger(i),
            DataType::U64(u) => influxdb::Type::UnsignedInteger(u),
            DataType::String(s) => influxdb::Type::Text(s.into_string()),
        }
    }
}

pub struct MetricsReportCollector {
    pub writer: UnboundedSender<WriteQuery>,
    pub flush_complete: Arc<AtomicBool>,
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

        let flush_complete = Arc::new(AtomicBool::new(false));
        let writer =
            start_metrics_write_task(runtime, shutdown_listener, client, flush_complete.clone());

        Ok(Self {
            writer,
            flush_complete,
        })
    }

    fn try_send(&self, query: WriteQuery) {
        if let Err(e) = self.writer.send(query) {
            if self.flush_complete.load(Ordering::Relaxed) {
                log::info!(
                    "Failed to record metric because the write task has finished: {}",
                    e
                );
            } else {
                log::warn!("Failed to record metric: {}", e);
            }
        }
    }
}

impl ReportCollector for MetricsReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        let mut query = Timestamp::Nanoseconds(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
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

        self.try_send(query);
    }

    fn add_custom(&mut self, metric: ReportMetric) {
        let metric = metric.into_inner();

        let mut query = Timestamp::Nanoseconds(
            metric
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime before UNIX_EPOCH")
                .as_nanos(),
        )
        .into_query(metric.name.into_string());

        for (k, v) in metric.fields {
            query = query.add_field(k.into_string(), v.into_type());
        }

        for (k, v) in metric.tags {
            query = query.add_tag(k.into_string(), v.into_type());
        }

        self.try_send(query);
    }

    fn finalize(&self) {
        let wait_started = std::time::Instant::now();
        let mut notify_timer = std::time::Instant::now();
        while !self
            .flush_complete
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            if notify_timer.elapsed().as_secs() > 10 {
                log::warn!(
                    "Still waiting for metrics to flush after {} seconds.",
                    wait_started.elapsed().as_secs()
                );
                notify_timer = std::time::Instant::now();
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        log::debug!(
            "Metrics flushed after {} seconds",
            wait_started.elapsed().as_secs()
        );
    }
}

fn start_metrics_write_task(
    runtime: &Runtime,
    mut shutdown_listener: DelegatedShutdownListener,
    client: Client,
    flush_complete: Arc<AtomicBool>,
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

    writer
}
