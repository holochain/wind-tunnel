use influxdb::WriteQuery;

/// A trait for reporting metrics.
pub trait Report {
    type Error;

    /// Report a [`WriteQuery`] metric.
    fn report(&mut self, metric: WriteQuery) -> Result<(), Self::Error>;
}
