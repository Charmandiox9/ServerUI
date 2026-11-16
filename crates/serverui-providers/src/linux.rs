use super::linux_processes::Processes;
use serverui_core::domain::{DiskInfo, DiskSpace, ProviderError, Sample, Snapshot, SystemProvider};
use std::time::{Duration, Instant};
use sysinfo::{DiskRefreshKind, Disks, MINIMUM_CPU_UPDATE_INTERVAL, System};

const MAX_DISKS: usize = 64;

pub struct LinuxProvider {
    system: System,
    disks: Option<Disks>,
    disk_data: Vec<DiskInfo>,
    disk_warning: Option<String>,
    last_disks: Option<Instant>,
    last_sample: Option<Instant>,
    cached: Option<Snapshot>,
    processes: Processes,
    origin: Instant,
}

fn unavailable(context: &str, error: impl std::fmt::Display) -> ProviderError {
    ProviderError::Unavailable(format!("{context}: {error}"))
}

/// Normalize untrusted floating-point library values once, at the adapter boundary.
fn usage(value: f32) -> Option<u32> {
    (value.is_finite() && value >= 0.0)
        .then(|| (value as f64 * 100.0).round().min(u32::MAX as f64) as u32)
}

impl LinuxProvider {
    pub fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            origin: Instant::now(),
            system: System::new().map_err(|e| unavailable("Inicializando Linux", e))?,
            disks: None,
            disk_data: Vec::new(),
            disk_warning: None,
            last_disks: None,
            last_sample: None,
            cached: None,
            processes: Processes::new(
                System::boot_time().map_err(|e| unavailable("Leyendo inicio Linux", e))?,
            )
            .map_err(|e| unavailable("Inicializando procesos", e))?,
        })
    }

    fn refresh_disks(&mut self, now: Instant) {
        if self
            .last_disks
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(5))
        {
            return;
        }
        self.last_disks = Some(now);
        if self.disks.is_none() {
            match Disks::new() {
                Ok(disks) => self.disks = Some(disks),
                Err(error) => {
                    self.disk_warning = Some(format!("Discos no disponibles: {error}"));
                    return;
                }
            }
        }
        let Some(disks) = self.disks.as_mut() else {
            return;
        };
        // Enumerate first; only collect storage for the bounded display list.
        disks.refresh_specifics(true, DiskRefreshKind::nothing());
        self.disk_warning =
            (disks.len() > MAX_DISKS).then(|| format!("Montajes limitados a {MAX_DISKS}"));
        self.disk_data = disks
            .iter_mut()
            .take(MAX_DISKS)
            .map(|disk| {
                let refreshed = disk.refresh_specifics(DiskRefreshKind::nothing().with_storage());
                DiskInfo {
                    mount: disk
                        .mount_point()
                        .to_string_lossy()
                        .chars()
                        .take(256)
                        .collect(),
                    space: (refreshed && disk.total_space() > 0).then(|| DiskSpace {
                        total_bytes: disk.total_space(),
                        available_bytes: disk.available_space().min(disk.total_space()),
                    }),
                }
            })
            .collect();
        self.disk_data.sort_by(|a, b| a.mount.cmp(&b.mount));
    }
}

impl SystemProvider for LinuxProvider {
    fn snapshot(&mut self) -> Sample {
        let now = Instant::now();
        if self
            .last_sample
            .is_some_and(|last| now.duration_since(last) < MINIMUM_CPU_UPDATE_INTERVAL)
        {
            return Ok(self.cached.clone());
        }
        let warmed = self.last_sample.is_some();
        self.system.refresh_memory();
        self.system.refresh_cpu_usage();
        if self.system.total_memory() == 0 || self.system.cpus().is_empty() {
            return Err(ProviderError::Unavailable(
                "No se pueden leer CPU/RAM desde /proc".into(),
            ));
        }
        let uptime = System::uptime().map_err(|e| unavailable("Leyendo uptime", e))?;
        let cpu_count = self.system.cpus().len().min(u32::MAX as usize) as u32;
        let (processes, process_count) = self
            .processes
            .collect(now.saturating_duration_since(self.origin), uptime)
            .map_err(|e| unavailable("Leyendo procesos", e))?;
        self.refresh_disks(now);
        let snapshot = Snapshot {
            cpu_usage: if warmed {
                usage(self.system.global_cpu_usage()).map(|v| v.min(10000))
            } else {
                None
            },
            cpu_count,
            memory_total: self.system.total_memory(),
            memory_used: self.system.used_memory().min(self.system.total_memory()),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap().min(self.system.total_swap()),
            uptime_seconds: uptime,
            disks: self.disk_data.clone(),
            disk_warning: self.disk_warning.clone(),
            process_count,
            processes,
        };
        self.last_sample = Some(now);
        self.cached = Some(snapshot.clone());
        Ok(Some(snapshot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_cpu_is_unavailable_and_multicore_usage_is_preserved() {
        assert_eq!(usage(f32::NAN), None);
        assert_eq!(usage(f32::INFINITY), None);
        assert_eq!(usage(-1.0), None);
        assert_eq!(usage(123.45), Some(12345));
        assert_eq!(usage(0.0), Some(0));
    }
}
