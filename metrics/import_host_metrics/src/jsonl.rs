use std::io::BufRead as _;

/// A module for reading JSON Lines (jsonl) files.
#[derive(Debug, Default)]
pub struct JsonlReader {
    /// Whether to allow invalid entries during parsing.
    pub allow_invalid_entries: bool,
}

impl JsonlReader {
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

    use crate::run_scenario::RunScenario;

    use super::*;

    const JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_run_summary.jsonl");
    const MALFORMED_JSON_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/invalid_summary.jsonl");

    #[test]
    fn test_should_parse_jsonl() {
        let reader = fs::File::open(JSON_PATH).expect("Failed to open test file");
        let result: Result<Vec<RunScenario>, JsonlError> = JsonlReader::default().parse(reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_parse_jsonl_from_file() {
        let result: Result<Vec<RunScenario>, JsonlError> =
            JsonlReader::default().parse_from_file(JSON_PATH);
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_fail_parsing_on_invalid_jsonl() {
        let reader = fs::File::open(MALFORMED_JSON_PATH).expect("Failed to open test file");
        let result: Result<Vec<RunScenario>, JsonlError> = JsonlReader::default().parse(reader);
        assert!(result.is_err());
    }
}
