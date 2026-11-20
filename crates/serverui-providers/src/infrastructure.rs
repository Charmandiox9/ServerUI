use crate::Scenario;
use serverui_core::{domain::ProviderError, infrastructure::*};

fn unavailable(message: impl Into<String>) -> ProviderError {
    ProviderError::Unavailable(message.into())
}

pub struct MockInfrastructure {
    scenario: Scenario,
    running: [bool; 2],
}

impl MockInfrastructure {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            running: [true, false],
        }
    }
}

impl InfrastructureProvider for MockInfrastructure {
    fn perform(&mut self, request: &InfrastructureRequest) -> InfrastructureResult {
        if self.scenario == Scenario::RecoverableError {
            self.scenario = Scenario::Normal;
            return Err(unavailable(
                "Fallo de infraestructura simulado; r: reintentar",
            ));
        }
        let empty = self.scenario == Scenario::Empty;
        let (engine, id) = match request {
            InfrastructureRequest::Inventory => {
                return Ok(InfrastructureData::Inventory(Inventory {
                    interfaces: Ok(if empty {
                        vec![]
                    } else {
                        vec![NetworkInterface {
                            name: "eth0-demo".into(),
                            state: "UP".into(),
                            addresses: Some(vec!["192.0.2.10/24".into(), "2001:db8::10/64".into()]),
                        }]
                    }),
                    devices: Ok(if empty {
                        vec![]
                    } else {
                        vec![BlockDevice {
                            name: "vda-demo".into(),
                            kind: "disk".into(),
                            bytes: 64 * 1024 * 1024 * 1024,
                            read_only: false,
                        }]
                    }),
                }));
            }
            InfrastructureRequest::List(engine) => (*engine, None),
            InfrastructureRequest::Logs(engine, id) => (*engine, Some(id)),
            InfrastructureRequest::Change(change) => (change.engine, Some(&change.container.id)),
        };
        let index = usize::from(engine == Engine::Podman);
        let fixture = if index == 0 {
            "a".repeat(64)
        } else {
            "b".repeat(64)
        };
        let fixture_id =
            ContainerId::parse(&fixture).ok_or_else(|| unavailable("ID de demo invalido"))?;
        if id.is_some_and(|id| empty || *id != fixture_id) {
            return Err(unavailable("Contenedor simulado no disponible"));
        }
        match request {
            InfrastructureRequest::List(_) => Ok(InfrastructureData::Containers(
                if empty {
                    vec![]
                } else {
                    vec![Container {
                        id: fixture_id,
                        name: if index == 0 { "demo-web" } else { "demo-job" }.into(),
                        state: if self.running[index] {
                            "running"
                        } else {
                            "exited"
                        }
                        .into(),
                    }]
                },
                false,
            )),
            InfrastructureRequest::Logs(..) => Ok(InfrastructureData::Logs(vec![
                "DEMO: logs simulados del contenedor".into(),
            ])),
            InfrastructureRequest::Change(change) => {
                self.running[index] = change.operation != ContainerOperation::Stop;
                Ok(InfrastructureData::Changed)
            }
            InfrastructureRequest::Inventory => Err(unavailable("Consulta de demo invalida")),
        }
    }
}

#[cfg(target_os = "linux")]
mod native;
#[cfg(target_os = "linux")]
pub use native::LinuxInfrastructure;
