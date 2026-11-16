//! Development probe: aggregate timings only, no names, arguments or host writes.
#[cfg(target_os = "linux")]
#[path = "../src/linux_processes.rs"]
mod linux_processes;
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    use sysinfo::{DiskRefreshKind, Disks, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new()?;
    let mut disks = Disks::new()?;
    let origin = Instant::now();
    let mut fast = linux_processes::Processes::new(System::boot_time()?)?;
    let mut costs = [Duration::ZERO; 4];
    const SAMPLES: u32 = 30;
    for _ in 0..SAMPLES {
        let start = Instant::now();
        system.refresh_memory();
        system.refresh_cpu_usage();
        costs[0] += start.elapsed();
        let start = Instant::now();
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory(),
        );
        costs[1] += start.elapsed();
        let start = Instant::now();
        disks.refresh_specifics(true, DiskRefreshKind::nothing());
        for disk in disks.iter_mut().take(64) {
            disk.refresh_specifics(DiskRefreshKind::nothing().with_storage());
        }
        costs[2] += start.elapsed();
        let start = Instant::now();
        let _ = fast.collect(origin.elapsed(), System::uptime()?)?;
        costs[3] += start.elapsed();
    }
    println!(
        "samples={SAMPLES} processes={} mounts={}",
        system.processes().len(),
        disks.len()
    );
    for (name, cost) in ["system", "processes", "disks", "processes_stat"]
        .into_iter()
        .zip(costs)
    {
        println!(
            "{name}_mean_ms={:.4}",
            cost.as_secs_f64() * 1000.0 / f64::from(SAMPLES)
        );
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("sampling_cost requires Linux");
}
