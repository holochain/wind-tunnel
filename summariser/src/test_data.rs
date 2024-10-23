use anyhow::Context;
use influxdb::{Query, ReadQuery};
use polars::frame::DataFrame;
use sha3::Digest;

#[cfg(feature = "test_data")]
pub fn insert_run_summary(summary: &wind_tunnel_summary_model::RunSummary) -> anyhow::Result<()> {
    let out_file = match open_output_path(
        "1_run_summaries",
        &file_name_from_run_summary(summary),
        false,
    )? {
        Some(f) => f,
        None => {
            log::info!("Not creating run summary file as it already exists");
            return Ok(());
        }
    };

    log::debug!("Writing run summary to {:?}", out_file);

    serde_json::to_writer_pretty(out_file, summary).context("Failed to write run summary")?;

    Ok(())
}

#[cfg(feature = "test_data")]
pub fn insert_query_result(query: &ReadQuery, frame: &mut DataFrame) -> anyhow::Result<()> {
    use polars::io::SerWriter;
    use polars::prelude::JsonFormat;

    let file_name = file_name_from_query(query)?;
    let out_file = match open_output_path("2_query_results", &file_name, false)? {
        Some(f) => f,
        None => {
            log::info!(
                "Not creating query result file as it already exists for query {:?}: {:?}",
                query,
                file_name
            );
            return Ok(());
        }
    };

    log::debug!("Writing query result to {:?}", out_file);

    polars::io::json::JsonWriter::new(out_file)
        .with_json_format(JsonFormat::Json)
        .finish(frame)?;

    Ok(())
}

// TODO should provide some way to detect which files are no longer being used as the test data is
//      updated. We need to be deleting data that is no longer needed.
#[cfg(feature = "query_test_data")]
pub fn load_query_result(query: &ReadQuery) -> anyhow::Result<DataFrame> {
    use polars::io::SerReader;

    let mut in_file = open_input_path("2_query_results", file_name_from_query(query)?)
        .with_context(|| format!("For query: {:?}", query))?;

    in_file.set_modified(std::time::SystemTime::now())?;

    polars::io::json::JsonReader::new(&mut in_file)
        .finish()
        .context("Failed to load query result")
}

#[cfg(any(feature = "test_data", feature = "query_test_data"))]
pub fn insert_summary_output(
    output: &crate::model::SummaryOutput,
    overwrite: bool,
) -> anyhow::Result<()> {
    let out_file = match open_output_path(
        "3_summary_outputs",
        &file_name_from_run_summary(&output.run_summary),
        overwrite,
    )? {
        Some(f) => f,
        None => {
            log::info!("Not creating summary output file as it already exists");
            return Ok(());
        }
    };

    log::debug!("Writing summary output to {:?}", out_file);

    serde_json::to_writer_pretty(out_file, output).context("Failed to write summary output")?;

    Ok(())
}

#[cfg(any(feature = "test_data", feature = "query_test_data"))]
fn open_output_path(
    stage: &str,
    file_name: &str,
    overwrite: bool,
) -> anyhow::Result<Option<std::fs::File>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(stage)
        .join(file_name);

    match std::fs::OpenOptions::new()
        .create_new(!overwrite)
        .write(true)
        .open(path)
    {
        Ok(f) => {
            // Truncate the file if we're overwriting
            if overwrite {
                f.set_len(0)?;
            }

            Ok(Some(f))
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // No need to error if this has already been created
            Ok(None)
        }
        Err(e) => Err(e).context("Failed to open file for writing"),
    }
}

#[cfg(feature = "query_test_data")]
fn open_input_path(stage: &str, file_name: String) -> anyhow::Result<std::fs::File> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(stage)
        .join(file_name);

    match std::fs::OpenOptions::new().read(true).open(&path) {
        Ok(f) => Ok(f),
        Err(e) => Err(e).with_context(|| format!("Failed to open input file: {:?}", path)),
    }
}

#[cfg(any(feature = "test_data", feature = "query_test_data"))]
fn file_name_from_run_summary(summary: &wind_tunnel_summary_model::RunSummary) -> String {
    format!("{}-{}.json", summary.scenario_name, summary.fingerprint())
}

fn file_name_from_query(query: &ReadQuery) -> anyhow::Result<String> {
    let query_string = query.clone().build()?.get();
    let mut hasher = sha3::Sha3_256::new();
    Digest::update(&mut hasher, query_string.as_bytes());

    Ok(format!("{:x}.json", hasher.finalize()))
}
