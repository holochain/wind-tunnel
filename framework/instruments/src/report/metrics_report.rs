use std::time::SystemTime;
use crate::metrics::{create_operation_duration_metric, OperationDurationMetric};
use crate::report::ReportCollector;
use crate::OperationRecord;
use anyhow::Context;
use influxive_core::Metric;
use influxive_writer::InfluxiveWriter;

pub struct MetricsReportCollector {
    pub writer: InfluxiveWriter,
}

impl MetricsReportCollector {
    pub fn new() -> anyhow::Result<Self> {
        let writer = InfluxiveWriter::with_token_auth(
            influxive_writer::InfluxiveWriterConfig::default(),
            std::env::var("INFLUX_HOST").context(
                "Cannot configure metrics reporter without environment variable `INFLUX_HOST`",
            )?,
            std::env::var("INFLUX_BUCKET").context(
                "Cannot configure metrics reporter without environment variable `INFLUX_BUCKET`",
            )?,
            std::env::var("INFLUX_TOKEN").context(
                "Cannot configure metrics reporter without environment variable `INFLUX_TOKEN`",
            )?,
        );

        Ok(Self {
            writer,
        })
    }
}

impl ReportCollector for MetricsReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        let mut metric = Metric::new(
            SystemTime::now(),
            "operation_duration",
        )
            .with_field("value", operation_record
                .elapsed
                .expect("OperationRecord must have an elapsed time")
                .as_micros() as f64
                / 1000.0)
            .with_tag("operation_id", operation_record.operation_id.to_string())
            .with_tag("is_error", operation_record.is_error.to_string());
        for (k, v) in &operation_record.attr {
            metric  = metric.with_tag(k.to_string(), v.to_string());
        }

        self.writer.write_metric(metric);
    }

    fn finalize(&self) {
        // Not required for metrics currently.
    }
}
