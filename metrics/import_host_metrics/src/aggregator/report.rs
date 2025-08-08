use influxlp_tools::LineProtocol;

/// A trait for reporting metrics.
pub trait Report {
    type Error;

    /// Report a [`LineProtocol`] metric.
    fn report(&mut self, metric: LineProtocol) -> Result<(), Self::Error>;
}
