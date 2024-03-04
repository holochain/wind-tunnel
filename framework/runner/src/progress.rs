use std::cmp::min;
use std::fmt::Write;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use tokio::time::Instant;
use crate::shutdown::DelegatedShutdownListener;

/// Displays a progress bar while the test is running to show the user how long is left.
pub fn start_progress(planned_runtime: Duration, mut shutdown_listener: DelegatedShutdownListener) {
    std::thread::Builder::new().name("progress".to_string()).spawn(move || {
        let start_time = Instant::now();
        let pb = ProgressBar::new(planned_runtime.as_secs());
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{wide_bar:.cyan/blue}] [{elapsed_precise} / {planned_runtime}]")
            .expect("Failed to set progress style")
            .with_key("planned_runtime", {
                let hours = planned_runtime.as_secs() / 3600;
                let minutes = (planned_runtime.as_secs() % 3600) / 60;
                let seconds = planned_runtime.as_secs() % 60;
                move |_state: &ProgressState, w: &mut dyn Write| write!(w, "{:02}:{:02}:{:02}", hours, minutes, seconds).expect("Could not write planned_runtime")
            })
            .progress_chars("#>-"));

        loop {
            if shutdown_listener.should_shutdown() {
                log::trace!("Progress thread shutting down");
                pb.finish_and_clear();
                break;
            }

            let new = min(start_time.elapsed().as_secs(), planned_runtime.as_secs());
            pb.set_position(new);
            std::thread::sleep(Duration::from_secs(1));
        }
    }).expect("Failed to start progress thread");
}
