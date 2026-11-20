use crate::Scenario;
use serverui_core::{domain::ProviderError, packages::*};
use std::sync::Arc;

pub struct MockPackages {
    scenario: Scenario,
    failed_search: bool,
    versions: [u8; 3],
}

impl MockPackages {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            failed_search: false,
            versions: [0, 1, 0],
        }
    }
}

const FIXTURES: [(&str, &str); 3] = [
    ("demo-editor", "Editor de texto simulado"),
    ("demo-web", "Servidor web simulado"),
    ("demo-utils", "Utilidades simuladas"),
];

fn unavailable(message: &str) -> ProviderError {
    ProviderError::Unavailable(message.into())
}

impl PackageProvider for MockPackages {
    fn perform(
        &mut self,
        request: &PackageRequest,
        progress: &mut dyn FnMut(PackageProgress),
    ) -> PackageResult {
        let (name, operation) = match request {
            PackageRequest::Recommendations | PackageRequest::Search(_) => {
                let query = match request {
                    PackageRequest::Search(query) => Some(query.as_str()),
                    _ => None,
                };
                if query.is_some()
                    && self.scenario == Scenario::RecoverableError
                    && !self.failed_search
                {
                    self.failed_search = true;
                    return Err(unavailable("Fallo APT simulado; r: reintentar"));
                }
                let rows = FIXTURES
                    .iter()
                    .filter_map(|(name, description)| {
                        if self.scenario == Scenario::Empty
                            || query.is_some_and(|query| !name.contains(query))
                        {
                            return None;
                        }
                        Some(PackageInfo {
                            name: PackageName::parse(name)?,
                            description: (*description).into(),
                        })
                    })
                    .collect();
                return Ok(PackageData::Search(rows, false));
            }
            PackageRequest::Preview(name, operation) => (name, *operation),
            PackageRequest::Apply(plan) => (&plan.name, plan.operation),
        };
        let index = FIXTURES
            .iter()
            .position(|(candidate, _)| *candidate == name.as_str())
            .ok_or_else(|| unavailable("Paquete simulado desconocido"))?;
        let old = self.versions[index];
        let new = match operation {
            PackageOperation::Install => 2,
            PackageOperation::Upgrade if old != 0 => 2,
            PackageOperation::Upgrade => {
                return Err(unavailable("El paquete simulado no esta instalado"));
            }
            PackageOperation::Remove => 0,
        };
        let changes = if old == new {
            vec![]
        } else {
            vec![format!(
                "{} {} ({old} -> {new})",
                operation.label(),
                name.as_str()
            )]
        };
        let plan = PackagePlan {
            name: name.clone(),
            operation,
            changes: Arc::new(changes),
        };
        if let PackageRequest::Apply(confirmed) = request {
            if confirmed != &plan || plan.changes.is_empty() {
                return Err(unavailable("Cambio de plan; vuelve a simular"));
            }
            progress(PackageProgress {
                percent: Some(75),
                line: "Aplicando cambios simulados".into(),
            });
            self.versions[index] = new;
            Ok(PackageData::Applied(PackageReport {
                status: "APT simulado: operacion completada".into(),
                lines: plan.changes.as_ref().clone(),
                undo_install: (operation == PackageOperation::Install && old == 0)
                    .then(|| name.clone()),
            }))
        } else {
            Ok(PackageData::Preview(plan))
        }
    }
}

#[cfg(target_os = "linux")]
mod native;
#[cfg(target_os = "linux")]
pub use native::LinuxPackages;
