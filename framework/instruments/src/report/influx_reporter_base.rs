use crate::report::{ReportCollector, ReportMetric};
use crate::OperationRecord;

use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
use influxive_core::DataType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

pub(crate) struct InfluxReporterBase {
    join_handle: JoinHandle<()>,
    writer: UnboundedSender<WriteQuery>,
    flush_complete: Arc<AtomicBool>,
}

impl InfluxReporterBase {
    pub fn new(
        join_handle: JoinHandle<()>,
        writer: UnboundedSender<WriteQuery>,
        flush_complete: Arc<AtomicBool>,
    ) -> Self {
        Self {
            join_handle,
            writer,
            flush_complete,
        }
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

    fn crash_if_reporting_task_finished(&self) {
        if self.join_handle.is_finished() {
            panic!("Reporter cannot be used because the write task has finished");
        }
    }
}

impl ReportCollector for InfluxReporterBase {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.crash_if_reporting_task_finished();

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
                .as_micros() as f64 // TODO use as_secs_f64 and let influx handle ms
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
        self.crash_if_reporting_task_finished();

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
        while !self.flush_complete.load(Ordering::Relaxed) {
            if notify_timer.elapsed().as_secs() > 10 {
                log::warn!(
                    "Still waiting for metrics to flush after {} seconds.",
                    wait_started.elapsed().as_secs()
                );
                notify_timer = std::time::Instant::now();
            }

            // If the write task has exited then there's no point trying to wait for it to finish
            // any longer.
            if self.join_handle.is_finished() {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        log::debug!(
            "Metrics flushed after {} seconds",
            wait_started.elapsed().as_secs()
        );
    }
}

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
