use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use opentelemetry::KeyValue;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::data::{
    AggregatedMetrics, Gauge, Histogram, MetricData, ResourceMetrics, Sum,
};
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;

use crate::line_protocol::{self, FieldValue};

/// A custom OpenTelemetry MetricExporter that writes aggregated metrics to
/// InfluxDB line protocol files.
pub struct InfluxLineProtocolExporter {
    run_id: String,
    scenario_name: String,
    file: Mutex<std::io::BufWriter<std::fs::File>>,
    is_shutdown: AtomicBool,
}

impl InfluxLineProtocolExporter {
    pub fn new(dir: PathBuf, run_id: String, scenario_name: String) -> anyhow::Result<Self> {
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }

        let out_path = dir.join(format!(
            "{}-{}.influx",
            scenario_name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        log::debug!("Influx line protocol exporter starting, using file {out_path:?}");

        let file = std::fs::File::options()
            .create_new(true)
            .write(true)
            .open(out_path)?;

        Ok(Self {
            run_id,
            scenario_name,
            file: Mutex::new(std::io::BufWriter::with_capacity(1024 * 1024, file)),
            is_shutdown: AtomicBool::new(false),
        })
    }

    fn write_metrics(&self, metrics: &ResourceMetrics) -> anyhow::Result<()> {
        let mut file = self.file.lock().unwrap();
        let file = &mut *file;
        let mut buf = String::with_capacity(512);

        for scope_metrics in metrics.scope_metrics() {
            for metric in scope_metrics.metrics() {
                let name = metric.name();
                match metric.data() {
                    AggregatedMetrics::F64(data) => {
                        self.write_metric_data(file, &mut buf, name, data)?;
                    }
                    AggregatedMetrics::U64(data) => {
                        self.write_metric_data_u64(file, &mut buf, name, data)?;
                    }
                    AggregatedMetrics::I64(data) => {
                        self.write_metric_data_i64(file, &mut buf, name, data)?;
                    }
                }
            }
        }

        file.flush()?;
        Ok(())
    }

    fn write_metric_data(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        data: &MetricData<f64>,
    ) -> anyhow::Result<()> {
        match data {
            MetricData::Histogram(h) => self.write_histogram(file, buf, name, h),
            MetricData::Gauge(g) => self.write_gauge_f64(file, buf, name, g),
            MetricData::Sum(s) => self.write_sum_f64(file, buf, name, s),
            MetricData::ExponentialHistogram(_) => Ok(()),
        }
    }

    fn write_metric_data_u64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        data: &MetricData<u64>,
    ) -> anyhow::Result<()> {
        match data {
            MetricData::Gauge(g) => self.write_gauge_u64(file, buf, name, g),
            MetricData::Sum(s) => self.write_sum_u64(file, buf, name, s),
            _ => Ok(()),
        }
    }

    fn write_metric_data_i64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        data: &MetricData<i64>,
    ) -> anyhow::Result<()> {
        match data {
            MetricData::Gauge(g) => self.write_gauge_i64(file, buf, name, g),
            MetricData::Sum(s) => self.write_sum_i64(file, buf, name, s),
            _ => Ok(()),
        }
    }

    fn write_histogram(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        histogram: &Histogram<f64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(histogram.time());

        for dp in histogram.data_points() {
            let mut tags = self.base_tags();
            for kv in dp.attributes() {
                tags.push((kv.key.as_str().to_string(), kv_value_to_string(kv)));
            }
            let tag_refs: Vec<(&str, &str)> =
                tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

            let mut fields: Vec<(&str, FieldValue)> = vec![
                ("count", FieldValue::UnsignedInteger(dp.count())),
                ("sum", FieldValue::Float(dp.sum())),
            ];

            if let Some(min) = dp.min() {
                fields.push(("min", FieldValue::Float(min)));
            }
            if let Some(max) = dp.max() {
                fields.push(("max", FieldValue::Float(max)));
            }

            // Compute mean
            if dp.count() > 0 {
                fields.push(("mean", FieldValue::Float(dp.sum() / dp.count() as f64)));
            }

            // Write bucket counts as fields: bucket_<bound>=<count>
            let bounds: Vec<f64> = dp.bounds().collect();
            let bucket_counts: Vec<u64> = dp.bucket_counts().collect();
            // We need owned strings for the field names
            let bucket_field_names: Vec<String> = bounds
                .iter()
                .map(|b| format!("bucket_{b}"))
                .chain(std::iter::once("bucket_inf".to_string()))
                .collect();
            for (field_name, &count) in bucket_field_names.iter().zip(bucket_counts.iter()) {
                fields.push((field_name.as_str(), FieldValue::UnsignedInteger(count)));
            }

            buf.clear();
            line_protocol::write_line_protocol(buf, name, &tag_refs, &fields, timestamp_ns)?;
            writeln!(file, "{buf}")?;
        }
        Ok(())
    }

    fn write_gauge_f64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        gauge: &Gauge<f64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(gauge.time());
        for dp in gauge.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value()),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_gauge_u64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        gauge: &Gauge<u64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(gauge.time());
        for dp in gauge.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value() as f64),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_gauge_i64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        gauge: &Gauge<i64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(gauge.time());
        for dp in gauge.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value() as f64),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_sum_f64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        sum: &Sum<f64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(sum.time());
        for dp in sum.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value()),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_sum_u64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        sum: &Sum<u64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(sum.time());
        for dp in sum.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value() as f64),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_sum_i64(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        sum: &Sum<i64>,
    ) -> anyhow::Result<()> {
        let timestamp_ns = system_time_to_nanos(sum.time());
        for dp in sum.data_points() {
            self.write_single_value_point(
                file,
                buf,
                name,
                dp.attributes(),
                FieldValue::Float(dp.value() as f64),
                timestamp_ns,
            )?;
        }
        Ok(())
    }

    fn write_single_value_point<'a>(
        &self,
        file: &mut impl Write,
        buf: &mut String,
        name: &str,
        attributes: impl Iterator<Item = &'a KeyValue>,
        value: FieldValue,
        timestamp_ns: u128,
    ) -> anyhow::Result<()> {
        let mut tags = self.base_tags();
        for kv in attributes {
            tags.push((kv.key.as_str().to_string(), kv_value_to_string(kv)));
        }
        let tag_refs: Vec<(&str, &str)> =
            tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        buf.clear();
        line_protocol::write_line_protocol(
            buf,
            name,
            &tag_refs,
            &[("value", value)],
            timestamp_ns,
        )?;
        writeln!(file, "{buf}")?;
        Ok(())
    }

    fn base_tags(&self) -> Vec<(String, String)> {
        vec![
            ("run_id".to_string(), self.run_id.clone()),
            ("scenario_name".to_string(), self.scenario_name.clone()),
        ]
    }
}

impl PushMetricExporter for InfluxLineProtocolExporter {
    fn export(
        &self,
        metrics: &ResourceMetrics,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let result = if self.is_shutdown.load(Ordering::Relaxed) {
            Err(OTelSdkError::AlreadyShutdown)
        } else {
            self.write_metrics(metrics)
                .map_err(|e| OTelSdkError::InternalFailure(format!("Failed to write metrics: {e}")))
        };
        std::future::ready(result)
    }

    fn force_flush(&self) -> OTelSdkResult {
        let mut file = self.file.lock().unwrap();
        file.flush()
            .map_err(|e| OTelSdkError::InternalFailure(format!("Failed to flush: {e}")))
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        self.is_shutdown.store(true, Ordering::Relaxed);
        self.force_flush()
    }

    fn temporality(&self) -> Temporality {
        Temporality::Delta
    }
}

fn system_time_to_nanos(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn kv_value_to_string(kv: &KeyValue) -> String {
    kv.value.as_str().to_string()
}
