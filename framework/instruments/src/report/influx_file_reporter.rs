use crate::report::influx_reporter_base::InfluxReporterBase;
use crate::report::{ReportCollector, ReportMetric};
use crate::OperationRecord;

use influxdb::{Query, WriteQuery};

use std::fmt::Debug;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use wind_tunnel_core::prelude::DelegatedShutdownListener;

/// Write metrics to disk in the InfluxDB line protocol format.
/// Metrics can then be sent to InfluxDB by Telegraf.
///
/// This is the recommended reporter to use when running distributed tests.
pub struct InfluxFileReportCollector {
    inner: InfluxReporterBase,
}

impl InfluxFileReportCollector {
    pub fn new(
        runtime: &tokio::runtime::Handle,
        shutdown_listener: DelegatedShutdownListener,
        dir: PathBuf,
        run_id: String,
        scenario_name: String,
    ) -> Self {
        let flush_complete = Arc::new(AtomicBool::new(false));
        let (join_handle, writer) = start_metrics_file_write_task(
            runtime,
            shutdown_listener,
            dir,
            scenario_name.clone(),
            flush_complete.clone(),
        );

        Self {
            inner: InfluxReporterBase::new(
                run_id,
                scenario_name,
                join_handle,
                writer,
                flush_complete,
            ),
        }
    }
}

impl ReportCollector for InfluxFileReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.inner.add_operation(operation_record);
    }

    fn add_custom(&mut self, metric: ReportMetric) {
        self.inner.add_custom(metric);
    }

    fn finalize(&self) {
        self.inner.finalize();
    }
}

fn start_metrics_file_write_task(
    runtime: &tokio::runtime::Handle,
    mut shutdown_listener: DelegatedShutdownListener,
    dir: PathBuf,
    scenario_name: String,
    flush_complete: Arc<AtomicBool>,
) -> (JoinHandle<()>, UnboundedSender<WriteQuery>) {
    let (writer, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let join_handle = runtime.spawn(async move {
        if !dir.exists() {
            tokio::fs::create_dir_all(&dir).await.unwrap();
        }

        let out_path = dir.join(format!(
            "{}-{}.influx",
            scenario_name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        log::debug!("Influx file reporter starting, using file {:?}", out_path);
        let mut file = File::options()
            .create_new(true)
            .write(true)
            .open(out_path)
            .await
            .unwrap();

        // Listen and write metrics until shutdown
        loop {
            select! {
                _ = shutdown_listener.wait_for_shutdown() => {
                    log::debug!("Shutting down influx file reporter");
                    break;
                }
                query = receiver.recv() => {
                    if let Some(query) = query {
                        write_query(&mut file, query).await.unwrap()
                    } else {
                        break;
                    }
                }
            }
        }

        log::debug!("Draining any remaining metrics before shutting down...");
        let mut drain_count = 0;

        // Drain remaining metrics before shutting down
        while let Ok(query) = receiver.try_recv() {
            write_query(&mut file, query).await.unwrap();
            drain_count += 1;

            if drain_count % 1000 == 0 {
                log::debug!("Drained {} remaining metrics", drain_count);
            }
        }

        // Ensure everything that's buffered has been written to disk.
        file.flush().await.unwrap();

        log::debug!("Drained {} remaining metrics", drain_count);

        // Signal the 'finalize' method that the write task has finished.
        flush_complete.store(true, Ordering::Relaxed);
    });

    (join_handle, writer)
}

#[inline]
async fn write_query<W>(writer: &mut W, query: WriteQuery) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Debug,
{
    let query_str = query.build()?.get();
    writer.write_all(query_str.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    Ok(())
}
