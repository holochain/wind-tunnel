use crate::analyze::standard_timing_stats;
use crate::model::StandardTimingsStats;
use crate::query;
use anyhow::Context;
use polars::frame::DataFrame;
use polars::prelude::{DataType, IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

/// Return the highest installed-cell count and enabled percentage recorded while enabling apps.
fn cell_enable_metrics(frame: &DataFrame) -> anyhow::Result<(u32, f64)> {
    let metrics = frame
        .clone()
        .lazy()
        .select([
            col("cells_total")
                .cast(DataType::UInt32)
                .max()
                .alias("cells_total"),
            col("enabled_pct")
                .cast(DataType::Float64)
                .max()
                .alias("enabled_pct"),
        ])
        .collect()
        .context("Aggregate cell enable metrics")?;

    let cells_total = metrics.column("cells_total")?.u32()?.get(0).unwrap_or(0);
    let max_enabled_pct = metrics.column("enabled_pct")?.f64()?.get(0).unwrap_or(0.0);

    Ok((cells_total, max_enabled_pct))
}

fn timing_for_phase(
    frame: &DataFrame,
    phase: &str,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    let filtered = frame
        .clone()
        .lazy()
        .filter(col("phase").eq(lit(phase)))
        .collect()
        .with_context(|| format!("Filter conductor startup phase {phase}"))?;

    if filtered.is_empty() {
        return Ok(None);
    }

    standard_timing_stats(filtered, "value", "10s", None)
        .with_context(|| format!("Timing stats for conductor startup phase {phase}"))
        .map(Some)
}

fn startup_timings(frame: &DataFrame) -> anyhow::Result<ConductorStartupTimings> {
    Ok(ConductorStartupTimings {
        initial: timing_for_phase(frame, "initial")?
            .context("Conductor startup data has no initial phase")?,
        post_install: timing_for_phase(frame, "post_install")?
            .context("Conductor startup data has no post_install phase")?,
        periodic: timing_for_phase(frame, "periodic")?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConductorStartupTimings {
    /// Initial startup time in seconds with no cells installed.
    initial: StandardTimingsStats,
    /// Startup time in seconds after all cells are installed but remain disabled.
    post_install: StandardTimingsStats,
    /// Startup time in seconds while progressively enabling cells.
    ///
    /// This is absent when `WT_RESTART_INTERVAL` disables periodic restarts or the run ends before
    /// the first configured restart.
    periodic: Option<StandardTimingsStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConductorStartupSummary {
    /// Conductor startup time in seconds for each lifecycle phase.
    ///
    /// Phases: `initial` is a start with an empty conductor (the baseline), `post_install` is a
    /// restart with all cells installed but disabled (with lazy cell loading this should stay
    /// close to the baseline), and `periodic` are restarts taken while a growing share of cells
    /// is enabled. Comparing the phases shows how installed and enabled cell counts affect
    /// startup time.
    startup_timing: ConductorStartupTimings,
    /// Time in seconds to enable one previously disabled app (cell) via the admin interface.
    ///
    /// Enables happen sequentially, so the trend over the run shows whether enabling gets more
    /// expensive as the number of already-enabled cells grows.
    cell_enable_timing: StandardTimingsStats,
    /// Number of cells installed on each conductor during setup.
    cells_total: u32,
    /// Highest percentage of cells (0-100) that had been enabled by the end of the run.
    ///
    /// Below 100 the run ended before every cell was enabled, so timings at high enabled
    /// percentages are missing rather than fast.
    max_enabled_pct: f64,
}

pub(crate) async fn summarize_conductor_startup(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<ConductorStartupSummary> {
    assert_eq!(summary.scenario_name, "conductor_startup");

    let startup = query::query_metrics_fields(
        &client,
        &summary,
        "wt.custom.conductor_startup",
        &["value", "cells_total", "cells_enabled", "enabled_pct"],
        &["agent", "phase"],
        None,
    )
    .await
    .context("Load conductor startup data")?;

    let cell_enable = query::query_metrics_fields(
        &client,
        &summary,
        "wt.custom.cell_enable",
        &["value", "cells_total", "cells_enabled", "enabled_pct"],
        &["agent"],
        None,
    )
    .await
    .context("Load cell enable data")?;

    let (cells_total, max_enabled_pct) = cell_enable_metrics(&cell_enable)?;

    Ok(ConductorStartupSummary {
        startup_timing: startup_timings(&startup)?,
        cell_enable_timing: standard_timing_stats(cell_enable, "value", "10s", None)
            .context("Timing stats for cell enable")?,
        cells_total,
        max_enabled_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::{NamedFrom, Series, TimeUnit};

    #[test]
    fn cell_enable_metrics_uses_numeric_column_maxima() -> anyhow::Result<()> {
        let frame = DataFrame::new(vec![
            Series::new("cells_total".into(), [10_i64, 10, 10]).into(),
            Series::new("enabled_pct".into(), [10.0_f64, 50.0, 100.0]).into(),
        ])?;

        let (cells_total, max_enabled_pct) = cell_enable_metrics(&frame)?;

        assert_eq!(cells_total, 10);
        assert_eq!(max_enabled_pct, 100.0);
        Ok(())
    }

    #[test]
    fn startup_timings_preserve_phase_specific_results() -> anyhow::Result<()> {
        let time = Series::new(
            "time".into(),
            [0_i64, 1_000_000_000, 2_000_000_000, 3_000_000_000],
        )
        .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;
        let frame = DataFrame::new(vec![
            time.into(),
            Series::new(
                "phase".into(),
                ["initial", "post_install", "periodic", "periodic"],
            )
            .into(),
            Series::new("value".into(), [1.0_f64, 2.0, 3.0, 5.0]).into(),
        ])?;

        let timings = startup_timings(&frame)?;

        assert_eq!(timings.initial.mean, 1.0);
        assert_eq!(timings.post_install.mean, 2.0);
        assert_eq!(timings.periodic.expect("periodic timing").mean, 4.0);
        Ok(())
    }
}
