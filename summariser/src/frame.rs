use influxdb::integrations::serde_integration::DatabaseQueryResult;
use polars::prelude::*;
use std::io::{Seek, SeekFrom, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("No series in table {table}: {result:?}")]
    NoSeriesInResult {
        table: String,
        result: serde_json::Value,
    },
}

pub(crate) fn load_from_response(
    table: &str,
    response: DatabaseQueryResult,
) -> anyhow::Result<DataFrame> {
    // Consume `response` to take ownership of the inner data. This avoids the need to
    // clone values when converting to the Polars JSON format, and allows each row to be
    // freed as it is written rather than accumulating a duplicate copy in memory.
    let result = response
        .results
        .into_iter()
        .next()
        .unwrap_or(serde_json::Value::Null);

    let mut result_obj = match result {
        serde_json::Value::Object(m) => m,
        other => {
            return Err(LoadError::NoSeriesInResult {
                table: table.to_string(),
                result: other,
            }
            .into());
        }
    };

    let series_arr = match result_obj.remove("series") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => {
            return Err(LoadError::NoSeriesInResult {
                table: table.to_string(),
                result: serde_json::Value::Object(result_obj),
            }
            .into());
        }
    };

    // result_obj no longer needed
    drop(result_obj);

    let mut series_obj = match series_arr.into_iter().next() {
        Some(serde_json::Value::Object(m)) => m,
        _ => anyhow::bail!("No series or series is not an object"),
    };

    let columns: Vec<String> = match series_obj.remove("columns") {
        Some(serde_json::Value::Array(cols)) => cols
            .into_iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            })
            .collect(),
        _ => anyhow::bail!("No columns in series"),
    };

    let values: Vec<serde_json::Value> = match series_obj.remove("values") {
        Some(serde_json::Value::Array(vals)) => vals,
        _ => anyhow::bail!("No values in series"),
    };
    // series_obj no longer needed
    drop(series_obj);

    // Stream rows directly to the tempfile one at a time, consuming each row as it is written.
    // This avoids building an intermediate Vec<serde_json::Value> (which would clone all data)
    // and avoids serde_json::to_string (which materialises the entire JSON as a String in memory
    // before writing). Peak memory here is one row at a time rather than two full copies.
    let mut f = tempfile::tempfile()?;
    {
        let mut writer = std::io::BufWriter::new(&mut f);
        writer.write_all(b"[")?;
        let n = values.len();
        for (i, row) in values.into_iter().enumerate() {
            if let serde_json::Value::Array(row_arr) = row {
                let obj: serde_json::Map<String, serde_json::Value> = columns
                    .iter()
                    .zip(row_arr.into_iter())
                    .map(|(col, val)| (col.clone(), val))
                    .collect();
                serde_json::to_writer(&mut writer, &serde_json::Value::Object(obj))?;
            }
            if i + 1 < n {
                writer.write_all(b",")?;
            }
        }
        writer.write_all(b"]")?;
    }

    f.seek(SeekFrom::Start(0))?;
    let mut frame = JsonReader::new(f).finish()?;
    frame = parse_time_column(frame)?;

    Ok(frame)
}

pub(crate) fn parse_time_column(frame: DataFrame) -> anyhow::Result<DataFrame> {
    Ok(frame
        .clone()
        .lazy()
        .with_column(
            col("time")
                .str()
                .to_datetime(
                    Some(TimeUnit::Nanoseconds),
                    None,
                    StrptimeOptions {
                        format: None,
                        strict: false, // Sometime date-times come back with a different precision from InfluxDB
                        ..Default::default()
                    },
                    lit("raise"),
                )
                .fill_null(col("time").str().to_datetime(
                    Some(TimeUnit::Nanoseconds),
                    None,
                    StrptimeOptions {
                        format: None,
                        strict: false, // Sometime date-times come back with a different precision from InfluxDB
                        ..Default::default()
                    },
                    lit("raise"),
                ))
                .alias("time"),
        )
        .collect()?)
}
