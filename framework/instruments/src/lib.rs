use crate::report::{MetricKind, ReportCollector};
use dashmap::DashMap;
use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Gauge, Histogram, Meter, MeterProvider};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

mod line_protocol;
mod otel_exporter;
mod report;

pub mod prelude {
    pub use crate::report::{MetricKind, ReportCollector, ReportMetric};
    pub use crate::{OperationRecord, ReportConfig, Reporter, report_operation};
}

/// Default histogram bucket boundaries for operation durations (in seconds).
///
/// Finer granularity in the 1ms–500ms range where most operations land.
const DEFAULT_HISTOGRAM_BOUNDARIES: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

#[derive(Debug)]
pub struct ReportConfig {
    pub dir: Option<PathBuf>,
    pub run_id: String,
    pub scenario_name: String,
    pub enable_in_memory: bool,
    pub enable_in_memory_with_custom_metrics: bool,
    pub enable_influx_file: bool,
    pub metrics_interval: Duration,
}

impl ReportConfig {
    pub fn new(run_id: String, scenario_name: String) -> Self {
        ReportConfig {
            dir: None,
            run_id,
            scenario_name,
            enable_in_memory: false,
            enable_in_memory_with_custom_metrics: false,
            enable_influx_file: false,
            metrics_interval: Duration::from_secs(10),
        }
    }

    pub fn enable_in_memory(mut self) -> Self {
        self.enable_in_memory = true;
        self
    }

    pub fn enable_in_memory_with_custom_metrics(mut self) -> Self {
        self.enable_in_memory_with_custom_metrics = true;
        self
    }

    pub fn enable_influx_file(mut self, dir: PathBuf) -> Self {
        self.dir = Some(dir);
        self.enable_influx_file = true;
        self
    }

    pub fn with_metrics_interval(mut self, interval: Duration) -> Self {
        self.metrics_interval = interval;
        self
    }

    pub fn init_reporter(self) -> anyhow::Result<Reporter> {
        let mut collectors: Vec<RwLock<Box<dyn ReportCollector + Send + Sync>>> = Vec::new();

        if self.enable_in_memory {
            collectors.push(RwLock::new(Box::new(report::InMemoryReporter::new())));
        }
        if self.enable_in_memory_with_custom_metrics {
            collectors.push(RwLock::new(Box::new(
                report::InMemoryWithCustomMetricsReporter::new(),
            )));
        }

        // Set up OTel meter provider if influx file reporting is enabled
        let meter_provider = if self.enable_influx_file {
            let exporter = otel_exporter::InfluxLineProtocolExporter::new(
                self.dir.unwrap(),
                self.run_id.clone(),
                self.scenario_name.clone(),
            )?;

            let reader = PeriodicReader::builder(exporter)
                .with_interval(self.metrics_interval)
                .build();

            let provider = SdkMeterProvider::builder().with_reader(reader).build();

            Some(provider)
        } else {
            None
        };

        let meter = meter_provider
            .as_ref()
            .map(|p| p.meter("wind_tunnel_instruments"));

        // Pre-create the operation duration histogram
        let operation_histogram = meter.as_ref().map(|m| {
            m.f64_histogram("wt.instruments.operation_duration")
                .with_boundaries(DEFAULT_HISTOGRAM_BOUNDARIES.to_vec())
                .build()
        });

        Ok(Reporter {
            collectors,
            meter_provider,
            meter,
            operation_histogram,
            gauges: DashMap::new(),
            counters: DashMap::new(),
        })
    }
}

/// The central metrics reporter for Wind Tunnel.
///
/// Records operation timings and custom metrics. When influx-file mode is enabled,
/// metrics are aggregated by OpenTelemetry and periodically flushed to InfluxDB
/// line protocol files.
pub struct Reporter {
    /// Legacy in-memory collectors for local dev
    collectors: Vec<RwLock<Box<dyn ReportCollector + Send + Sync>>>,
    /// OTel meter provider (manages the periodic reader + exporter)
    meter_provider: Option<SdkMeterProvider>,
    /// OTel meter for creating instruments
    meter: Option<Meter>,
    /// Pre-created histogram for operation durations
    operation_histogram: Option<Histogram<f64>>,
    /// Lazily-created OTel gauges for custom gauge metrics
    gauges: DashMap<String, Gauge<f64>>,
    /// Lazily-created OTel counters for custom counter metrics
    counters: DashMap<String, Counter<f64>>,
}

impl Reporter {
    fn add_operation(&self, operation_record: &OperationRecord) {
        // Record to OTel histogram
        if let Some(histogram) = &self.operation_histogram {
            let elapsed = operation_record
                .elapsed
                .expect("OperationRecord must have an elapsed time")
                .as_secs_f64();

            let mut attrs = vec![
                KeyValue::new("operation_id", operation_record.operation_id.clone()),
                KeyValue::new("is_error", operation_record.is_error.to_string()),
            ];
            for (k, v) in &operation_record.attr {
                attrs.push(KeyValue::new(k.clone(), v.clone()));
            }
            histogram.record(elapsed, &attrs);
        }

        // Also forward to in-memory collectors
        for collector in &self.collectors {
            collector.write().add_operation(operation_record);
        }
    }

    pub fn add_custom(&self, metric: report::ReportMetric) {
        // Record to OTel instruments
        if let Some(meter) = &self.meter {
            let attrs: Vec<KeyValue> = metric
                .tags
                .iter()
                .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
                .collect();

            let full_name = format!("wt.custom.{}", metric.name);

            match metric.kind {
                MetricKind::Gauge => {
                    let gauge = self
                        .gauges
                        .entry(full_name.clone())
                        .or_insert_with(|| meter.f64_gauge(full_name).build());
                    gauge.record(metric.value, &attrs);
                }
                MetricKind::Counter => {
                    let counter = self
                        .counters
                        .entry(full_name.clone())
                        .or_insert_with(|| meter.f64_counter(full_name).build());
                    counter.add(metric.value, &attrs);
                }
            }
        }

        // Also forward to in-memory collectors
        for collector in &self.collectors {
            collector.write().add_custom(metric.clone());
        }
    }

    pub fn finalize(&self) {
        // Shutdown the OTel meter provider — this triggers a final flush
        if let Some(provider) = &self.meter_provider
            && let Err(e) = provider.shutdown()
        {
            log::warn!("Failed to shutdown meter provider: {e}");
        }

        // Finalize in-memory collectors
        for collector in &self.collectors {
            collector.write().finalize();
        }
    }
}

impl std::fmt::Debug for Reporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reporter").finish()
    }
}

#[derive(Clone)]
pub struct OperationRecord {
    /// The ID of the operation, application specific value
    operation_id: String,
    /// The instant when the operation started
    started: std::time::Instant,
    /// Extra attributes to be reported
    attr: HashMap<String, String>,
    /// Elapsed time of the operation
    pub(crate) elapsed: Option<std::time::Duration>,
    /// Whether the operation failed
    pub(crate) is_error: bool,
}

impl std::fmt::Debug for OperationRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationRecord")
            .field("operation_id", &self.operation_id)
            .field("attr", &self.attr)
            .field("elapsed", &self.elapsed)
            .field("is_error", &self.is_error)
            .finish()
    }
}

impl OperationRecord {
    pub fn new(operation_id: String) -> Self {
        Self {
            operation_id,
            started: std::time::Instant::now(),
            attr: HashMap::new(),
            elapsed: None,
            is_error: false,
        }
    }

    pub fn add_attr(&mut self, key: &str, value: String) {
        self.attr.insert(key.to_string(), value);
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.elapsed
    }

    fn finish(&mut self) {
        self.elapsed = Some(self.started.elapsed());
    }

    fn set_error(&mut self, is_error: bool) {
        self.is_error = is_error;
    }
}

pub fn report_operation<T, E>(
    reporter: Arc<Reporter>,
    mut operation_record: OperationRecord,
    response: &Result<T, E>,
) {
    operation_record.finish();
    operation_record.set_error(response.is_err());
    reporter.add_operation(&operation_record);
}
