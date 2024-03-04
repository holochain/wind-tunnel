use crate::shutdown::DelegatedShutdownListener;
use sysinfo::{Pid, ProcessRefreshKind, System};

/// Monitor the resource usage of the wind-tunnel process and report high usage.
///
/// Note that this won't stop the test proceeding, it will just log a warning to the user know that
/// their test might be affected by high resource usage.
///
/// The CPU usage for the process is collected every [sysinfo::MINIMUM_CPU_UPDATE_INTERVAL] and checked.
/// If it is above 10% with respect to the number of cores then a warning is logged.
pub(crate) fn start_monitor(mut shutdown_listener: DelegatedShutdownListener) {
    std::thread::Builder::new()
        .name("monitor".to_string())
        .spawn(move || {
            let this_process_pid = Pid::from_u32(std::process::id());
            let mut sys = System::new();

            sys.refresh_cpu();
            let cpu_count = sys.cpus().len();

            loop {
                if shutdown_listener.should_shutdown() {
                    break;
                }

                sys.refresh_process_specifics(this_process_pid, ProcessRefreshKind::new().with_cpu());

                let process = sys.process(this_process_pid).expect("Failed to get process info");

                let usage = (process.cpu_usage() / (cpu_count * 100) as f32) * 100.0;
                if usage > 10.0 {
                    log::warn!("High CPU usage detected. Wind tunnel is using {:.2}% of the CPU, with {} available cores", usage, cpu_count);
                }

                std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            }
        })
        .expect("Failed to start monitor thread");
}
