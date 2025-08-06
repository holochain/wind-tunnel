use std::io::BufRead as _;

/// A module for reading JSON Lines (jsonl) files.
#[derive(Debug, Default)]
pub struct JsonlReader {
    /// Whether to allow invalid entries during parsing.
    pub allow_invalid_entries: bool,
}

impl JsonlReader {
    /// Configures the reader to allow invalid entries.
    ///
    /// If set to `true`, the reader will skip lines that cannot be parsed into the specified type.
    pub fn allow_invalid_entries(mut self, allow: bool) -> Self {
        self.allow_invalid_entries = allow;
        self
    }

    /// Parses a JSON Lines file from the given reader into a vector of type `T`.
    pub fn parse<R, T>(&self, reader: R) -> Result<Vec<T>, JsonlError>
    where
        R: std::io::Read,
        T: serde::de::DeserializeOwned,
    {
        let mut results = Vec::new();
        for line in std::io::BufReader::new(reader).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(value) => results.push(value),
                Err(e) if self.allow_invalid_entries => {
                    trace!("Skipping invalid entry: {e}");
                    continue;
                }
                Err(e) => return Err(JsonlError::Serde(e)),
            }
        }
        Ok(results)
    }

    /// Parses a JSON Lines file from the specified path into a vector of type `T`.
    pub fn parse_from_file<P, T>(&self, path: P) -> Result<Vec<T>, JsonlError>
    where
        P: AsRef<std::path::Path>,
        T: serde::de::DeserializeOwned,
    {
        let file = std::fs::File::open(path)?;
        self.parse(file)
    }
}

/// An error type for [`JsonlReader::parse`].
#[derive(Debug, thiserror::Error)]
pub enum JsonlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde JSON error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {

    use std::fs;

    use crate::metrics::HostMetrics;

    use super::*;

    const JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_metrics.jsonl");
    const MALFORMED_JSON_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/malformed_host_metrics.jsonl"
    );
    const METRICS_WITH_ONE_INVALID_ENTRY: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/host_metrics_with_one_invalid_entry.jsonl"
    );

    #[test]
    fn test_should_parse_jsonl() {
        let reader = fs::File::open(JSON_PATH).expect("Failed to open test file");
        let result: Result<Vec<HostMetrics>, JsonlError> = JsonlReader::default().parse(reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_parse_jsonl_from_file() {
        let result: Result<Vec<HostMetrics>, JsonlError> =
            JsonlReader::default().parse_from_file(JSON_PATH);
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_allow_invalid_entries() {
        let reader =
            fs::File::open(METRICS_WITH_ONE_INVALID_ENTRY).expect("Failed to open test file");
        let result: Result<Vec<HostMetrics>, JsonlError> = JsonlReader::default()
            .allow_invalid_entries(true)
            .parse(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1); // Assuming there is one valid entry in the file
    }

    #[test]
    fn test_should_fail_parsing_on_invalid_jsonl() {
        let reader = fs::File::open(MALFORMED_JSON_PATH).expect("Failed to open test file");
        let result: Result<Vec<HostMetrics>, JsonlError> = JsonlReader::default().parse(reader);
        assert!(result.is_err());
    }
}
