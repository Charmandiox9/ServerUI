use serverui_core::domain::{ProviderError, Snapshot, SystemProvider};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scenario {
    #[default]
    Normal,
    Empty,
    /// First query succeeds, second fails, later queries recover.
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
    fn snapshot(&mut self) -> Result<Option<Snapshot>, ProviderError> {
        let step = self.step;
        self.step = self.step.saturating_add(1);
        match self.scenario {
            Scenario::Empty => return Ok(None),
            Scenario::RecoverableError if step == 1 => {
                return Err(ProviderError::Unavailable(
                    "Fallo simulado; pulsa r para reintentar".into(),
                ));
            }
            _ => {}
        }
        const GIB: u64 = 1024 * 1024 * 1024;
        Ok(Some(Snapshot {
            cpu_percent: [12, 28, 36, 19, 8, 22][(step % 6) as usize],
            memory_used: 2 * GIB + (step % 8) * 64 * 1024 * 1024,
            memory_total: 8 * GIB,
            swap_used: 0,
            swap_total: 2 * GIB,
            disk_used: 18 * GIB,
            disk_total: 64 * GIB,
            uptime_seconds: 86_400_u64.saturating_add(step),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serverui_core::application::{App, DataStatus};

    #[test]
    fn samples_are_repeatable_and_bounded() {
        let mut first = MockProvider::default();
        let mut second = MockProvider::default();
        for _ in 0..100 {
            let sample = first.snapshot();
            assert_eq!(sample, second.snapshot());
            let Ok(Some(sample)) = sample else {
                panic!("expected demo sample")
            };
            assert!(sample.cpu_percent <= 100);
            assert!(sample.memory_used <= sample.memory_total);
        }
    }

    #[test]
    fn empty_scenario_has_no_metrics() {
        assert_eq!(MockProvider::new(Scenario::Empty).snapshot(), Ok(None));
    }

    #[test]
    fn error_retains_last_sample_and_retry_recovers() {
        let mut provider = MockProvider::new(Scenario::RecoverableError);
        let mut app = App::default();
        app.receive(provider.snapshot());
        let previous = app.snapshot.clone();
        app.receive(provider.snapshot());
        assert!(matches!(app.status, DataStatus::Error(_)));
        assert_eq!(app.snapshot, previous);
        app.receive(provider.snapshot());
        assert_eq!(app.status, DataStatus::Ready);
        assert_ne!(app.snapshot, previous);
    }
}
