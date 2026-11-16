use serverui_core::domain::{
    DiskInfo, DiskSpace, ProcessId, ProcessInfo, ProviderError, Sample, Snapshot, SystemProvider,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scenario {
    #[default]
    Normal,
    Empty,
    RecoverableError,
}
#[derive(Debug, Default)]
pub struct MockProvider {
    scenario: Scenario,
    step: u64,
}
impl MockProvider {
    pub fn new(scenario: Scenario) -> Self {
        Self { scenario, step: 0 }
    }
}
impl SystemProvider for MockProvider {
    fn snapshot(&mut self) -> Sample {
        let step = self.step;
        self.step = self.step.saturating_add(1);
        match self.scenario {
            Scenario::Empty => return Ok(None),
            Scenario::RecoverableError if step == 1 => {
                return Err(ProviderError::Unavailable(
                    "Fallo simulado; r: reintentar".into(),
                ));
            }
            _ => {}
        }
        const MIB: u64 = 1024 * 1024;
        const GIB: u64 = 1024 * MIB;
        let processes = [
            "serverui", "sshd", "postgres", "nginx", "systemd", "worker", "backup", "cron",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let n = i as u64;
            ProcessInfo {
                id: ProcessId {
                    pid: 100 + i as u32,
                    start_time: 1_700_000_000 + n,
                },
                parent_pid: Some(1),
                name: name.into(),
                status: if i == 5 { "Running" } else { "Sleeping" }.into(),
                cpu_usage: Some(((step % 6 + n) * 500) as u32),
                memory_bytes: (32 + n * 24) * MIB,
                virtual_memory_bytes: (128 + n * 64) * MIB,
                run_time_seconds: 3600_u64.saturating_add(step),
            }
        })
        .collect::<Vec<_>>();
        Ok(Some(Snapshot {
            cpu_usage: Some([1200, 2800, 3600, 1900, 800, 2200][(step % 6) as usize]),
            cpu_count: 4,
            memory_used: 2 * GIB + (step % 8) * 64 * MIB,
            memory_total: 8 * GIB,
            swap_used: 0,
            swap_total: 2 * GIB,
            uptime_seconds: 86_400_u64.saturating_add(step),
            disks: vec![DiskInfo {
                mount: "/ (demo)".into(),
                space: Some(DiskSpace {
                    total_bytes: 64 * GIB,
                    available_bytes: 46 * GIB,
                }),
            }],
            disk_warning: None,
            process_count: processes.len(),
            processes,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serverui_core::application::{App, DataStatus};
    #[test]
    fn samples_repeat_and_process_ids_are_stable() {
        let mut a = MockProvider::default();
        let mut b = MockProvider::default();
        let mut last_ids = None;
        for _ in 0..100 {
            let s = a.snapshot();
            assert_eq!(s, b.snapshot());
            let Ok(Some(s)) = s else { panic!("sample") };
            assert!(s.cpu_usage.is_some_and(|v| v <= 10000));
            assert!(s.memory_used <= s.memory_total);
            let ids = s.processes.iter().map(|p| p.id).collect::<Vec<_>>();
            if let Some(before) = last_ids {
                assert_eq!(ids, before);
            }
            last_ids = Some(ids);
        }
    }
    #[test]
    fn empty_has_no_metrics() {
        assert_eq!(MockProvider::new(Scenario::Empty).snapshot(), Ok(None));
    }
    #[test]
    fn error_retains_last_sample_and_recovers() {
        let mut p = MockProvider::new(Scenario::RecoverableError);
        let mut a = App::default();
        a.receive(p.snapshot());
        let before = a.snapshot.clone();
        a.receive(p.snapshot());
        assert!(matches!(a.status, DataStatus::Error(_)));
        assert_eq!(a.snapshot, before);
        a.receive(p.snapshot());
        assert_eq!(a.status, DataStatus::Ready);
        assert_ne!(a.snapshot, before);
    }
}
