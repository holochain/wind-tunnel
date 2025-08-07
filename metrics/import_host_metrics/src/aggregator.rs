mod report;

use influxlp_tools::LineProtocol;

pub use self::report::Report;
use super::run_scenario::RunScenario;

const RUN_ID: &str = "run_id";
const SCENARIO_NAME: &str = "scenario_name";

/// Aggregates host metrics by `run_id` and reports them using a [`Report`]er.
pub struct HostMetricsAggregator;

impl HostMetricsAggregator {
    /// Aggregates the given influx lines [`LineProtocol`] by `run_id` of run scenarios
    /// and reports them using the reporter.
    pub fn aggregate_by_scenario<R>(
        mut reporter: R,
        run_scenarios: &[RunScenario],
        metrics: &[LineProtocol],
    ) -> Result<(), R::Error>
    where
        R: Report,
    {
        // iterate over each run scenario
        let mut reports = 0;
        for scenario in run_scenarios {
            // get time range for the scenario
            let start_time = scenario.started_at;
            let end_time = start_time + scenario.run_duration;

            let metrics_in_range = Self::get_metrics_in_range(metrics, start_time, end_time);
            debug!(
                "Scenario '{run_id}' is in the time range [{start_time}, {end_time}] with {n_metrics} metrics",
                run_id = scenario.run_id,
                n_metrics = metrics_in_range.len()
            );
            for metric in metrics_in_range {
                // create a report for each metric
                let report = Self::create_report(scenario, metric);
                // report the metric
                reporter.report(report)?;
                reports += 1;
                debug!(
                    "Reported metric '{metric_name}' for run '{run_id}' in scenario '{scenario}'",
                    metric_name = metric.get_measurement_ref(),
                    run_id = scenario.run_id,
                    scenario = scenario.scenario_name
                );
            }
        }

        info!(
            "Aggregated {reports} metrics across {} scenarios",
            run_scenarios.len()
        );

        Ok(())
    }

    /// Get all the [`LineProtocol`] entries that fall within the given time range.
    fn get_metrics_in_range(
        metrics: &[LineProtocol],
        start_time_secs: u64,
        end_time_secs: u64,
    ) -> Vec<&LineProtocol> {
        // convert timestamps to nanoseconds
        let start_time = start_time_secs * 1_000_000_000; // Convert to nanoseconds
        let end_time = end_time_secs * 1_000_000_000; // Convert to nanoseconds
        metrics
            .iter()
            .filter(|m| {
                let timestamp = m.get_timestamp().unwrap_or_default() as u64;

                timestamp >= start_time && timestamp <= end_time
            })
            .collect()
    }

    /// Create a [`LineProtocol`] from its relative [`RunScenario`] `run_id` and `scenario_name`
    /// and the [`LineProtocol`] entry.
    fn create_report(run_scenario: &RunScenario, host_metrics: &LineProtocol) -> LineProtocol {
        host_metrics
            .clone()
            .add_tag(RUN_ID, run_scenario.run_id.to_string())
            .add_tag(SCENARIO_NAME, run_scenario.scenario_name.to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug, Default)]
    struct TestReporter {
        lines: Vec<String>,
    }

    impl Report for TestReporter {
        type Error = ();

        fn report(&mut self, metric: LineProtocol) -> Result<(), Self::Error> {
            self.lines.push(metric.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_should_aggregate_metrics() {
        let scenarios = vec![
            RunScenario {
                run_id: "test_run_1".to_string(),
                scenario_name: "test_scenario_1".to_string(),
                started_at: 1622547600,
                run_duration: 60,
            },
            RunScenario {
                run_id: "test_run_1".to_string(),
                scenario_name: "test_scenario_2".to_string(),
                started_at: 1622547800,
                run_duration: 60,
            },
            RunScenario {
                run_id: "test_run_2".to_string(),
                scenario_name: "test_scenario_2".to_string(),
                started_at: 1622547700,
                run_duration: 60,
            },
        ];
        let metrics = vec![test_metrics(1622547605), test_metrics(1622547705)];
        let reporter = TestReporter::default();
        HostMetricsAggregator::aggregate_by_scenario(reporter, &scenarios, &metrics)
            .expect("Failed to aggregate metrics");
    }

    #[test]
    fn test_should_get_metrics_in_range() {
        let metrics = vec![test_metrics(1622547600), test_metrics(1622547700)];

        let result = HostMetricsAggregator::get_metrics_in_range(&metrics, 1622547600, 1622547700);
        assert_eq!(result.len(), 2);

        let result = HostMetricsAggregator::get_metrics_in_range(&metrics, 1622547605, 1622547606);
        assert_eq!(result.len(), 0);

        let result = HostMetricsAggregator::get_metrics_in_range(&metrics, 1622547601, 1622547700);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_should_create_query_from_host_metrics() {
        let run_scenario = RunScenario {
            run_id: "test_run".to_string(),
            scenario_name: "test_scenario".to_string(),
            started_at: 1622547600,
            run_duration: 60,
        };
        let host_metrics = test_metrics(1622547600);

        let query = HostMetricsAggregator::create_report(&run_scenario, &host_metrics).to_string();

        assert!(query.contains("cpu"));
        assert!(query.contains("run_id=test_run"));
        assert!(query.contains("scenario_name=test_scenario"));
        assert!(query.contains("usage_guest=0"));
        assert!(query.contains("usage_guest_nice=0"));
        assert!(query.contains("usage_idle=95.90000000000146"));
    }

    fn test_metrics(timestamp_secs: i64) -> LineProtocol {
        let timestamp = timestamp_secs * 1_000_000_000; // Convert to nanoseconds
        let s = format!(
            r#"cpu,cpu=cpu0,host=msi-manjaro usage_user=2.699999999999818,usage_system=0.8999999999997499,usage_nice=0,usage_iowait=0.19999999999999574,usage_irq=0.20000000000003126,usage_softirq=0.10000000000005116,usage_guest_nice=0,usage_idle=95.90000000000146,usage_steal=0,usage_guest=0 {timestamp}"#
        );
        LineProtocol::parse_line(&s).expect("Failed to parse test metrics")
    }
}
