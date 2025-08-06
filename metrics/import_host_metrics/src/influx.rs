use std::fs::File;
use std::io::{self, Write};

use influxdb::{Query, WriteQuery};

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
    type Error = InfluxWriteError;

    fn report(&mut self, metric: WriteQuery) -> Result<(), Self::Error> {
        debug!("Writing metric: {}", metric.build()?.get());
        writeln!(self.writer, "{}", metric.build()?.get())?;
        Ok(())
    }
}

/// Error type for InfluxDB write operations.
#[derive(Debug, thiserror::Error)]
pub enum InfluxWriteError {
    #[error("An error occurred while writing to the output writer.")]
    Io(#[from] io::Error),
    #[error("An error occurred while building the InfluxDB query.")]
    Influx(#[from] influxdb::Error),
}

#[cfg(test)]
mod tests {

    use influxdb::Timestamp;

    use super::*;

    #[test]
    fn test_write_metrics() {
        let mut buffer = Vec::new();
        let metrics = vec![
            WriteQuery::new(Timestamp::Seconds(1622547800), "test.metric")
                .add_field("value", influxdb::Type::UnsignedInteger(42)),
            WriteQuery::new(Timestamp::Seconds(1622547801), "test.metric")
                .add_field("value", influxdb::Type::SignedInteger(-1)),
        ];

        let mut reporter = InfluxFileReporter::new(&mut buffer);
        for metric in metrics {
            reporter.report(metric).unwrap();
        }

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("test.metric value=42i 1622547800"));
        assert!(output.contains("test.metric value=-1i 1622547801"));
    }

    #[test]
    fn test_should_write_to_file() {
        let tempfile = tempfile::NamedTempFile::new().unwrap();
        let metrics = vec![
            WriteQuery::new(Timestamp::Seconds(1622547800), "test.metric")
                .add_field("value", influxdb::Type::UnsignedInteger(42)),
            WriteQuery::new(Timestamp::Seconds(1622547801), "test.metric")
                .add_field("value", influxdb::Type::SignedInteger(-1)),
        ];

        let mut reporter = InfluxFileReporter::from_file(tempfile.path()).unwrap();
        for metric in metrics {
            reporter.report(metric).unwrap();
        }
        let content = std::fs::read_to_string(tempfile.path()).unwrap();
        assert!(content.contains("test.metric value=42i 1622547800"));
        assert!(content.contains("test.metric value=-1i 1622547801"));
    }
}
