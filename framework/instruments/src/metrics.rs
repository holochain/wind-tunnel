use opentelemetry_api::global::meter_with_version;
use opentelemetry_api::metrics::{Histogram, Unit};

pub type OperationDurationMetric = Histogram<f64>;

pub fn create_operation_duration_metric() -> OperationDurationMetric {
    meter_with_version(
        "wt.operation",
        Some("1"),
        None::<&'static str>,
        None::<Vec<_>>,
    )
    .f64_histogram("wt.operation.duration")
    .with_unit(Unit::new("ms"))
    .with_description("Operation duration in milliseconds")
    .init()
}
