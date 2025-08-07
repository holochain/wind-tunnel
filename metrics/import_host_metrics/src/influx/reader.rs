use std::io::{BufRead as _, Read};

use influxlp_tools::LineProtocol;

pub struct InfluxReader;

impl InfluxReader {
    /// Reads InfluxDB line protocol from a reader and returns a vector of [`LineProtocol`].
    pub fn read<R>(reader: R) -> Result<Vec<LineProtocol>, InfluxReadError>
    where
        R: Read,
    {
        let mut results = Vec::new();
        for line in std::io::BufReader::new(reader).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match influxlp_tools::LineProtocol::parse_line(&line) {
                Ok(parsed_line) => results.push(parsed_line),
                Err(e) => return Err(InfluxReadError::Parse(e)),
            }
        }

        Ok(results)
    }

    /// Reads InfluxDB line protocol from a file at the specified path and returns a vector of [`LineProtocol`].
    pub fn read_from_file<P>(path: P) -> Result<Vec<LineProtocol>, InfluxReadError>
    where
        P: AsRef<std::path::Path>,
    {
        let file = std::fs::File::open(path)?;
        Self::read(file)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InfluxReadError {
    #[error("An error occurred while reading the input: {0}")]
    Io(#[from] std::io::Error),
    #[error("An error occurred while parsing the InfluxDB line protocol: {0}")]
    Parse(#[from] influxlp_tools::error::LineProtocolError),
}

#[cfg(test)]
mod tests {}
