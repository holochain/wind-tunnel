use influxdb::integrations::serde_integration::DatabaseQueryResult;
use polars::prelude::*;
use std::io::Write;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("No series in result: {result:?}")]
    NoSeriesInResult { result: serde_json::Value },
}

pub(crate) fn load_from_response(response: DatabaseQueryResult) -> anyhow::Result<DataFrame> {
    let result = &response.results[0];

    let series = result
        .as_object()
        .and_then(|o| o.get("series"))
        .and_then(|s| s.as_array());
    if series.is_none() {
        return Err(LoadError::NoSeriesInResult {
            result: result.clone(),
        }
        .into());
    }

    let select_series = series.unwrap().first();
    if select_series.is_none() {
        anyhow::bail!("No series in result: {:?}", result);
    }

    let columns = select_series
        .unwrap()
        .as_object()
        .and_then(|o| o.get("columns"))
        .and_then(|c| c.as_array());
    if columns.is_none() {
        anyhow::bail!("No columns in series: {:?}", select_series);
    }
    let columns = columns.unwrap();

    let values = select_series
        .unwrap()
        .as_object()
        .and_then(|o| o.get("values"))
        .and_then(|v| v.as_array());
    if values.is_none() {
        anyhow::bail!("No values in series: {:?}", select_series);
    }
    let values = values.unwrap();

    // Convert to the polars format, an array of objects with each field named per object
    let mut content: Vec<serde_json::Value> = Vec::with_capacity(values.len());

    for value in values {
        let values = value.as_array().unwrap();
        let mut obj = serde_json::Map::<String, serde_json::Value>::new();
        for (column, value) in columns.iter().zip(values.iter()) {
            obj.insert(column.as_str().unwrap().to_string(), value.clone());
        }
        content.push(serde_json::Value::Object(obj));
    }

    let mut f = tempfile::tempfile()?;
    f.write_all(serde_json::to_string(&content)?.as_bytes())?;

    let mut frame = JsonReader::new(f).finish()?;
    frame = frame
        .clone()
        .lazy()
        .with_column(
            col("time")
                .str()
                .to_datetime(
                    None,
                    None,
                    StrptimeOptions {
                        format: Some("%Y-%m-%dT%H:%M:%S.%9fZ".into()),
                        strict: false, // Sometime date-times come back with a different precision from InfluxDB
                        ..Default::default()
                    },
                    lit("raise"),
                )
                .alias("time"),
        )
        .collect()?;

    Ok(frame)
}
