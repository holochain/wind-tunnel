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

    drop(result_obj);

    // Stream ALL series to the tempfile, merging tag values into each row.
    // This handles both single-series responses (no GROUP BY) and multi-series responses
    // (GROUP BY tag), where each series has its own `tags` map that must be merged into rows.
    let mut f = tempfile::tempfile()?;
    {
        let mut writer = std::io::BufWriter::new(&mut f);
        writer.write_all(b"[")?;
        let mut wrote_row = false;

        for series_val in series_arr {
            let mut series_obj = match series_val {
                serde_json::Value::Object(m) => m,
                other => anyhow::bail!("Expected series element to be an object, got: {other:?}"),
            };

            // Extract tags (present when GROUP BY tag is used)
            let tags: serde_json::Map<String, serde_json::Value> = series_obj
                .remove("tags")
                .and_then(|v| match v {
                    serde_json::Value::Object(m) => Some(m),
                    _ => None,
                })
                .unwrap_or_default();

            let columns: Vec<String> = match series_obj.remove("columns") {
                Some(serde_json::Value::Array(cols)) => cols
                    .into_iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    })
                    .collect(),
                _ => anyhow::bail!("No columns in series for table '{table}'"),
            };

            let values: Vec<serde_json::Value> = match series_obj.remove("values") {
                Some(serde_json::Value::Array(vals)) => vals,
                _ => anyhow::bail!("No values in series for table '{table}'"),
            };
            drop(series_obj);

            for row in values {
                let row_arr = match row {
                    serde_json::Value::Array(arr) => arr,
                    other => anyhow::bail!("Expected row to be an array, got: {:?}", other),
                };
                if row_arr.len() != columns.len() {
                    anyhow::bail!(
                        "Row length {} does not match column count {}",
                        row_arr.len(),
                        columns.len()
                    );
                }
                if wrote_row {
                    writer.write_all(b",")?;
                }
                // Build row object: tag columns first, then value columns
                let mut obj: serde_json::Map<String, serde_json::Value> = tags.clone();
                obj.extend(
                    columns
                        .iter()
                        .zip(row_arr)
                        .map(|(col, val)| (col.clone(), val)),
                );
                serde_json::to_writer(&mut writer, &serde_json::Value::Object(obj))?;
                wrote_row = true;
            }
        }
        writer.write_all(b"]")?;
        writer.flush()?;
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
