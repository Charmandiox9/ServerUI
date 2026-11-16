//! Typed service operations and terminal-independent interaction state.
use crate::domain::ProviderError;
use std::{collections::VecDeque, sync::Arc};

pub const MAX_SERVICES: usize = 2048;
pub const MAX_LOG_LINES: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnitName(String);
impl UnitName {
    pub fn parse(value: &str) -> Result<Self, ProviderError> {
        if value.len() > 256
            || !value.ends_with(".service")
            || value.len() <= 8
            || !value.as_bytes()[0].is_ascii_alphanumeric()
            || !value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._@:-".contains(&c))
            || value.ends_with("@.service")
        {
            return Err(ProviderError::Unavailable(
                "Nombre de unidad no admitido".into(),
            ));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOperation {
    Start,
    Stop,
    Restart,
    Enable,
}
impl ServiceOperation {
    pub fn argument(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Enable => "enable",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "Iniciar",
            Self::Stop => "Detener",
            Self::Restart => "Reiniciar",
            Self::Enable => "Habilitar al arranque",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub unit: UnitName,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRequest {
    List,
    Logs(UnitName),
    Execute(UnitName, ServiceOperation),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceData {
    List {
        units: Vec<ServiceInfo>,
        limited: bool,
    },
    Logs(Vec<String>),
    Executed,
}
pub type ServiceResult = Result<ServiceData, ProviderError>;
pub trait ServiceProvider: Send {
    fn perform(&mut self, request: &ServiceRequest) -> ServiceResult;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Services {
    pub units: Arc<Vec<ServiceInfo>>,
    pub loaded: bool,
    pub limited: bool,
    pub selected: Option<UnitName>,
    pub filter: String,
    pub editing: bool,
    pub busy: bool,
    pub confirmation: Option<(UnitName, ServiceOperation)>,
    pub logs: Option<(UnitName, Arc<Vec<String>>)>,
    pub log_scroll: usize,
    pub show_history: bool,
    pub message: Option<String>,
    pub history: VecDeque<String>,
}
impl Services {
    pub fn visible(&self) -> Vec<&ServiceInfo> {
        let filter = self.filter.to_ascii_lowercase();
        self.units
            .iter()
            .filter(|s| s.unit.as_str().to_ascii_lowercase().contains(&filter))
            .collect()
    }
    pub fn select(&mut self, delta: isize) {
        if self.show_history {
            self.log_scroll = self
                .log_scroll
                .saturating_add_signed(delta)
                .min(self.history.len().saturating_sub(1));
            return;
        }
        if let Some((_, logs)) = &self.logs {
            self.log_scroll = self
                .log_scroll
                .saturating_add_signed(delta)
                .min(logs.len().saturating_sub(1));
            return;
        }
        let rows = self.visible();
        let index = rows
            .iter()
            .position(|s| Some(&s.unit) == self.selected.as_ref())
            .unwrap_or(0);
        self.selected = rows
            .get(
                index
                    .saturating_add_signed(delta)
                    .min(rows.len().saturating_sub(1)),
            )
            .map(|s| s.unit.clone());
    }
    pub fn reconcile(&mut self) {
        let rows = self.visible();
        if !rows.iter().any(|s| Some(&s.unit) == self.selected.as_ref()) {
            self.selected = rows.first().map(|s| s.unit.clone());
        }
    }
    pub fn propose(&mut self, operation: ServiceOperation) {
        if self.busy || self.logs.is_some() || self.show_history {
            return;
        }
        if let Some(unit) = self
            .selected
            .clone()
            .filter(|u| self.units.iter().any(|s| &s.unit == u))
        {
            self.log_scroll = 0;
            self.confirmation = Some((unit, operation));
        }
    }
    pub fn confirm(&mut self) -> Option<ServiceRequest> {
        if self.busy {
            return None;
        }
        let (unit, operation) = self.confirmation.take()?;
        self.units
            .iter()
            .any(|s| s.unit == unit)
            .then_some(ServiceRequest::Execute(unit, operation))
    }
    pub fn receive(&mut self, request: ServiceRequest, result: ServiceResult) {
        self.busy = false;
        self.message = None;
        let mutation = matches!(request, ServiceRequest::Execute(..));
        match result {
            Ok(ServiceData::List { mut units, limited }) => {
                let limited = limited || units.len() > MAX_SERVICES;
                units.truncate(MAX_SERVICES);
                units.sort_by(|a, b| a.unit.cmp(&b.unit));
                units.dedup_by(|a, b| a.unit == b.unit);
                self.units = Arc::new(units);
                self.loaded = true;
                self.limited = limited;
                self.reconcile();
                // Refresh invalidates any previously proposed authorization.
                self.confirmation = None;
            }
            Ok(ServiceData::Logs(mut lines)) => {
                lines.truncate(MAX_LOG_LINES);
                if let ServiceRequest::Logs(ref unit) = request {
                    self.logs = Some((unit.clone(), Arc::new(lines)));
                    self.log_scroll = 0;
                }
            }
            Ok(ServiceData::Executed) => {
                self.message = Some("Comando completado; r: comprobar estado".into())
            }
            Err(error) => self.message = Some(error.to_string()),
        }
        if mutation {
            if self.history.len() == 16 {
                self.history.pop_front();
            }
            if let ServiceRequest::Execute(unit, operation) = request {
                self.history.push_back(format!(
                    "{} {}: {}",
                    operation.label(),
                    unit.as_str(),
                    self.message.as_deref().unwrap_or("Sin resultado")
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit() -> UnitName {
        UnitName::parse("fixture.service").unwrap_or_else(|e| panic!("{e}"))
    }
    fn state() -> Services {
        let mut s = Services::default();
        s.receive(
            ServiceRequest::List,
            Ok(ServiceData::List {
                units: vec![ServiceInfo {
                    unit: unit(),
                    load: "loaded".into(),
                    active: "active".into(),
                    sub: "running".into(),
                    description: "fixture".into(),
                }],
                limited: false,
            }),
        );
        s
    }
    #[test]
    fn unit_names_reject_shell_options_patterns_and_controls() {
        for bad in [
            "--all.service",
            "../a.service",
            "a*.service",
            "a;id.service",
            "a\n.service",
            "a@.service",
            "a\\x20.service",
            "a%20.service",
            ".service",
        ] {
            assert!(UnitName::parse(bad).is_err(), "{bad}");
        }
        assert!(UnitName::parse("worker@instance-1.service").is_ok());
        assert!(UnitName::parse(&format!("{}.service", "a".repeat(256))).is_err());
    }
    #[test]
    fn only_explicit_confirmation_emits_one_exact_operation() {
        let mut s = state();
        assert_eq!(s.confirm(), None);
        s.propose(ServiceOperation::Stop);
        s.confirmation = None;
        assert_eq!(s.confirm(), None);
        s.propose(ServiceOperation::Restart);
        s.selected = None; // The captured target cannot change after proposal.
        assert_eq!(
            s.confirm(),
            Some(ServiceRequest::Execute(unit(), ServiceOperation::Restart))
        );
        assert_eq!(s.confirm(), None);
        s.busy = true;
        s.propose(ServiceOperation::Start);
        assert_eq!(s.confirm(), None);
    }
    #[test]
    fn disappearing_units_invalidate_confirmation_and_history_is_bounded() {
        let mut s = state();
        s.propose(ServiceOperation::Stop);
        s.receive(
            ServiceRequest::List,
            Ok(ServiceData::List {
                units: vec![],
                limited: false,
            }),
        );
        assert_eq!(s.confirm(), None);
        for _ in 0..30 {
            s.receive(
                ServiceRequest::Execute(unit(), ServiceOperation::Start),
                Err(ProviderError::Unavailable("denegado".into())),
            );
        }
        assert_eq!(s.history.len(), 16);
        assert!(s.message.as_ref().is_some_and(|m| m.contains("denegado")));
        s.receive(
            ServiceRequest::Logs(unit()),
            Ok(ServiceData::Logs(vec!["line".into(); 200])),
        );
        assert_eq!(s.logs.as_ref().map(|(_, v)| v.len()), Some(MAX_LOG_LINES));
        s.select(isize::MAX);
        assert_eq!(s.log_scroll, MAX_LOG_LINES - 1);
    }
}
