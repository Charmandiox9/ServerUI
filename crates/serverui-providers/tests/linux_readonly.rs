#![cfg(target_os = "linux")]

use serverui_core::domain::{Snapshot, SystemProvider};
use serverui_providers::LinuxProvider;
use std::{
    collections::HashMap,
    error::Error,
    fs, io,
    process::{Child, Command},
    thread,
    time::Duration,
};

fn sample(provider: &mut LinuxProvider) -> Result<Snapshot, Box<dyn Error>> {
    provider
        .snapshot()?
        .ok_or_else(|| io::Error::other("missing Linux sample").into())
}
fn meminfo() -> Result<HashMap<String, u64>, Box<dyn Error>> {
    fs::read_to_string("/proc/meminfo")?
        .lines()
        .filter_map(|line| {
            let mut p = line.split_whitespace();
            Some((
                p.next()?.trim_end_matches(':').to_owned(),
                p.next()?.parse::<u64>(),
            ))
        })
        .map(|(k, v)| v.map(|v| (k, v * 1024)).map_err(Into::into))
        .collect()
}
fn cpu_times() -> Result<(u64, u64), Box<dyn Error>> {
    let stat = fs::read_to_string("/proc/stat")?;
    let line = stat
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("/proc/stat empty"))?;
    let times = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()?;
    if times.len() != 8 {
        return Err(io::Error::other("/proc/stat CPU fields").into());
    }
    Ok((times.iter().sum(), times[3] + times[4]))
}

#[test]
fn live_metrics_match_independent_proc_reads() -> Result<(), Box<dyn Error>> {
    let mut provider = LinuxProvider::new()?;
    let cpu_before = cpu_times()?;
    let first = sample(&mut provider)?;
    assert_eq!(first.cpu_usage, None, "initial CPU needs a baseline");
    let own_id = std::process::id();
    assert!(first.processes.iter().any(|p| p.id.pid == own_id));
    thread::sleep(Duration::from_millis(500));
    let second = sample(&mut provider)?;
    let cpu_after = cpu_times()?;
    let memory = meminfo()?;
    assert_eq!(second.memory_total, memory["MemTotal"]);
    let expected_used = memory["MemTotal"].saturating_sub(memory["MemAvailable"]);
    assert!(
        second.memory_used.abs_diff(expected_used)
            < (second.memory_total / 20).max(64 * 1024 * 1024)
    );
    assert_eq!(second.swap_total, memory["SwapTotal"]);
    assert!(
        second
            .swap_used
            .abs_diff(memory["SwapTotal"].saturating_sub(memory["SwapFree"]))
            < 16 * 1024 * 1024
    );
    let uptime = fs::read_to_string("/proc/uptime")?
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::other("uptime"))?
        .parse::<f64>()?;
    assert!((second.uptime_seconds as f64 - uptime).abs() < 2.0);
    let total = cpu_after.0 - cpu_before.0;
    let idle = cpu_after.1 - cpu_before.1;
    let expected_cpu = (total.saturating_sub(idle)) as f64 / total.max(1) as f64 * 10000.0;
    let cpu = second
        .cpu_usage
        .ok_or_else(|| io::Error::other("CPU not warmed"))?;
    assert!(cpu <= 10000);
    assert!(
        (cpu as f64 - expected_cpu).abs() < 1000.0,
        "CPU {cpu} vs independent {expected_cpu}"
    );
    let own = second
        .processes
        .iter()
        .find(|p| p.id.pid == own_id)
        .ok_or_else(|| io::Error::other("own process missing"))?;
    assert!(own.cpu_usage.is_some());
    assert!(own.memory_bytes > 0);
    assert!(own.name.chars().count() <= 128);
    println!(
        "Linux verified: CPU {:.2}% vs /proc {:.2}%, RAM {} bytes, {} processes, uptime {} s",
        cpu as f64 / 100.0,
        expected_cpu / 100.0,
        second.memory_total,
        second.process_count,
        second.uptime_seconds
    );
    Ok(())
}

#[test]
fn disk_capacity_matches_coreutils_stat() -> Result<(), Box<dyn Error>> {
    let mut provider = LinuxProvider::new()?;
    let s = sample(&mut provider)?;
    for disk in &s.disks {
        let Some(space) = disk.space else {
            continue;
        };
        let out = Command::new("stat")
            .args(["-f", "-c", "%S %b %a", "--", &disk.mount])
            .output()?;
        assert!(out.status.success());
        let fields = String::from_utf8(out.stdout)?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<Vec<u64>, _>>()?;
        assert_eq!(space.total_bytes, fields[0] * fields[1]);
        // Concurrent filesystem activity may consume blocks between reads.
        assert!(space.available_bytes.abs_diff(fields[0] * fields[2]) < 64 * 1024 * 1024);
        assert!(space.available_bytes <= space.total_bytes);
    }
    println!(
        "Compared {} mount(s) with stat -f; pseudo/network FS may be excluded",
        s.disks.len()
    );
    Ok(())
}

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn disappearing_process_is_removed_and_fds_do_not_accumulate() -> Result<(), Box<dyn Error>> {
    // External tools are confined to this test fixture, never the provider.
    let mut child = Fixture(Command::new("sleep").arg("5").spawn()?);
    let mut provider = LinuxProvider::new()?;
    let first = sample(&mut provider)?;
    let id = child.0.id();
    assert!(first.processes.iter().any(|p| p.id.pid == id));
    child.0.kill()?;
    child.0.wait()?;
    let before = fs::read_dir("/proc/self/fd")?.count();
    for _ in 0..3 {
        thread::sleep(Duration::from_millis(250));
        let next = sample(&mut provider)?;
        assert!(!next.processes.iter().any(|p| p.id.pid == id));
    }
    let after = fs::read_dir("/proc/self/fd")?.count();
    assert!(after <= before + 8, "FDs {before} -> {after}");
    Ok(())
}
