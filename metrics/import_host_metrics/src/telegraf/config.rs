use std::path::PathBuf;

/// Provides the configuration parameters for telegraf to report aggregated host metrics.
///
/// It then allows the user to write the configuration to a Writer.
#[derive(Debug, Default)]
pub struct TelegrafConfig {
    pub bucket: String,
    pub influxdb_token: String,
    pub influxdb_url: String,
    pub metrics_file_path: PathBuf,
    pub organization: String,
}

impl TelegrafConfig {
    /// Builds a [`TelegrafConfig`] with the specified bucket.
    pub fn bucket(mut self, bucket: String) -> Self {
        self.bucket = bucket;
        self
    }

    /// Builds a [`TelegrafConfig`] with the specified token.
    pub fn influxdb_token(mut self, token: String) -> Self {
        self.influxdb_token = token;
        self
    }

    /// Builds a [`TelegrafConfig`] with the specified InfluxDB URL.
    pub fn influxdb_url(mut self, url: String) -> Self {
        self.influxdb_url = url;
        self
    }

    /// Builds a [`TelegrafConfig`] with the specified metrics file path.
    pub fn metrics_file_path(mut self, path: PathBuf) -> Self {
        self.metrics_file_path = path;
        self
    }

    /// Builds a [`TelegrafConfig`] with the specified organization.
    pub fn organization(mut self, organization: String) -> Self {
        self.organization = organization;
        self
    }

    /// Writes the configuration to a writer.
    pub fn write<W>(&self, mut writer: W) -> std::io::Result<()>
    where
        W: std::io::Write,
    {
        writeln!(
            writer,
            r#"
[[outputs.influxdb_v2]]
  ## The URLs of the InfluxDB cluster nodes.
  urls = ["{influxdb_url}"]
  ## Token for authentication
  token = "{influxdb_token}"
  ## Organization is the name of the organization you wish to write to
  organization = "{influxdb_organization}"
  ## Destination bucket to write into
  bucket = "{influxdb_bucket}"

[[inputs.file]]
  ## Files to parse each interval. Accept standard unix glob matching rules,
  ## as well as ** to match recursive files and directories.
  files = ["{metrics_file_path}"]
  ## Data format to consume.
  data_format = "influx"
  ## Character encoding to use when interpreting the file contents.  Invalid
  ## characters are replaced using the unicode replacement character.  When set
  ## to the empty string the encoding will be automatically determined.
  character_encoding = "utf-8"
"#,
            influxdb_url = self.influxdb_url,
            influxdb_token = self.influxdb_token,
            influxdb_organization = self.organization,
            influxdb_bucket = self.bucket,
            metrics_file_path = self
                .metrics_file_path
                .as_path()
                .to_string_lossy()
                .replace('\\', "\\\\"), // escape backslashes for Windows
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegraf_config_builder() {
        let config = TelegrafConfig::default()
            .influxdb_url("http://localhost:8086".to_string())
            .influxdb_token("my-token".to_string())
            .organization("my-org".to_string())
            .bucket("my-bucket".to_string())
            .metrics_file_path(PathBuf::from("/path/to/metrics.influx"));

        assert_eq!(config.influxdb_url, "http://localhost:8086");
        assert_eq!(config.influxdb_token, "my-token");
        assert_eq!(config.organization, "my-org");
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(
            config.metrics_file_path,
            PathBuf::from("/path/to/metrics.influx")
        );
    }

    #[test]
    fn test_telegraf_config_write() {
        let config = TelegrafConfig::default()
            .influxdb_url("http://localhost:8086".to_string())
            .influxdb_token("my-token".to_string())
            .organization("my-org".to_string())
            .bucket("my-bucket".to_string())
            .metrics_file_path(PathBuf::from("/path/to/metrics.influx"));

        let temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        config
            .write(temp.as_file())
            .expect("Failed to write config");

        let content = std::fs::read_to_string(temp.path()).expect("Failed to read temp file");
        assert!(content.contains("urls = [\"http://localhost:8086\"]"));
        assert!(content.contains("token = \"my-token\""));
        assert!(content.contains("organization = \"my-org\""));
        assert!(content.contains("bucket = \"my-bucket\""));
        assert!(content.contains("files = [\"/path/to/metrics.influx\"]"));
    }
}
