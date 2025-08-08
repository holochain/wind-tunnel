use std::fs::File;
use std::io::{self, Write};

use influxlp_tools::LineProtocol;

use crate::aggregator::Report;

/// A [`Report`] implementation utility for writing InfluxDB metrics in line protocol format.
pub struct InfluxFileReporter<W>
where
    W: Write,
{
    writer: W,
}

impl<W> InfluxFileReporter<W>
where
    W: Write,
{
    /// Creates a new [`InfluxFileReporter`] with the specified [`Write`]r.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl InfluxFileReporter<File> {
    /// Creates a new [`InfluxWriter`] from a file at the specified path.
    pub fn from_file<P>(path: P) -> Result<Self, io::Error>
    where
        P: AsRef<std::path::Path>,
    {
        let file = std::fs::File::create(path)?;
        Ok(Self::new(file))
    }
}

impl<W> Report for InfluxFileReporter<W>
where
    W: Write,
{
    type Error = io::Error;

    fn report(&mut self, metric: LineProtocol) -> Result<(), Self::Error> {
        debug!("Writing metric: {metric}");
        writeln!(self.writer, "{metric}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_write_metrics() {
        let mut buffer = Vec::new();
        let metrics = vec![
            LineProtocol::new("test.metric")
                .add_field("value", 42i64)
                .with_timestamp(1622547800i64 * 1_000_000_000),
            LineProtocol::new("test.metric")
                .add_field("value", -1i64)
                .with_timestamp(1622547801i64 * 1_000_000_000),
        ];

        let mut reporter = InfluxFileReporter::new(&mut buffer);
        for metric in metrics {
            reporter.report(metric).unwrap();
        }

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("test.metric value=42i 1622547800000000000"));
        assert!(output.contains("test.metric value=-1i 1622547801000000000"));
    }

    #[test]
    fn test_should_write_to_file() {
        let tempfile = tempfile::NamedTempFile::new().unwrap();
        let metrics = vec![
            LineProtocol::new("test.metric")
                .add_field("value", 42i64)
                .with_timestamp(1622547800i64 * 1_000_000_000),
            LineProtocol::new("test.metric")
                .add_field("value", -1i64)
                .with_timestamp(1622547801i64 * 1_000_000_000),
        ];

        let mut reporter = InfluxFileReporter::from_file(tempfile.path()).unwrap();
        for metric in metrics {
            reporter.report(metric).unwrap();
        }
        let content = std::fs::read_to_string(tempfile.path()).unwrap();
        assert!(content.contains("test.metric value=42i 1622547800000000000"));
        assert!(content.contains("test.metric value=-1i 1622547801000000000"));
    }
}
