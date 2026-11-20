use crate::domain::ProviderError;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Engine {
    #[default]
    Docker,
    Podman,
}

impl Engine {
    pub fn label(self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Docker => Self::Podman,
            Self::Podman => Self::Docker,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerId(String);

impl ContainerId {
    pub fn parse(value: &str) -> Option<Self> {
        (value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub id: ContainerId,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerOperation {
    Start,
    Stop,
    Restart,
}

impl ContainerOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "Iniciar",
            Self::Stop => "Detener",
            Self::Restart => "Reiniciar",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerChange {
    pub engine: Engine,
    pub container: Container,
    pub operation: ContainerOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfrastructureRequest {
    List(Engine),
    Logs(Engine, ContainerId),
    Change(ContainerChange),
    Inventory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInterface {
    pub name: String,
    pub state: String,
    pub addresses: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDevice {
    pub name: String,
    pub kind: String,
    pub bytes: u64,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub interfaces: Result<Vec<NetworkInterface>, ProviderError>,
    pub devices: Result<Vec<BlockDevice>, ProviderError>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InventoryView {
    #[default]
    Network,
    Devices,
    Mounts,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InfrastructureData {
    Containers(Vec<Container>, bool),
    Logs(Vec<String>),
    Changed,
    Inventory(Inventory),
}

pub type InfrastructureResult = Result<InfrastructureData, ProviderError>;

pub trait InfrastructureProvider: Send {
    fn perform(&mut self, request: &InfrastructureRequest) -> InfrastructureResult;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Infrastructure {
    pub engine: Engine,
    pub rows: Arc<Vec<Container>>,
    pub selected: usize,
    pub listed: bool,
    pub limited: bool,
    pub logs: Option<(ContainerId, Arc<Vec<String>>)>,
    pub confirmation: Option<ContainerChange>,
    pub container_message: Option<String>,
    pub inventory: Option<Arc<Inventory>>,
    pub inventory_error: Option<String>,
    pub view: InventoryView,
    pub container_scroll: usize,
    pub inventory_scroll: usize,
    pub busy: Option<InfrastructureRequest>,
    discard_logs: bool,
}

impl Infrastructure {
    pub fn started(&mut self, request: InfrastructureRequest) {
        match request {
            InfrastructureRequest::Inventory => self.inventory_error = None,
            InfrastructureRequest::List(_) => {
                if self.listed {
                    self.container_message = None;
                }
                self.container_scroll = 0;
            }
            InfrastructureRequest::Logs(..) => {
                self.container_scroll = 0;
            }
            InfrastructureRequest::Change(_) => self.container_message = None,
        }
        self.discard_logs = false;
        self.busy = Some(request);
    }

    pub fn receive(&mut self, request: InfrastructureRequest, reply: InfrastructureResult) {
        self.busy = None;
        match (request, reply) {
            (InfrastructureRequest::List(_), Ok(InfrastructureData::Containers(rows, limited))) => {
                let selected = self.rows.get(self.selected);
                self.selected = selected
                    .and_then(|old| rows.iter().position(|new| new.id == old.id))
                    .unwrap_or(0);
                self.rows = Arc::new(rows);
                self.limited = limited;
                self.listed = true;
            }
            (InfrastructureRequest::List(_), Err(error)) => {
                self.rows = Arc::default();
                self.selected = 0;
                self.limited = false;
                self.listed = true;
                self.container_message = Some(error.to_string());
            }
            (InfrastructureRequest::Logs(_, id), Ok(InfrastructureData::Logs(lines))) => {
                if !self.discard_logs {
                    self.logs = Some((id, Arc::new(lines)));
                    self.container_message = None;
                }
            }
            (InfrastructureRequest::Change(_), Ok(InfrastructureData::Changed)) => {
                self.listed = false;
                self.container_message = Some("Operacion completada".into());
            }
            (InfrastructureRequest::Inventory, Ok(InfrastructureData::Inventory(inventory))) => {
                self.inventory = Some(Arc::new(inventory));
                self.inventory_scroll = 0;
            }
            (InfrastructureRequest::Inventory, Err(error)) => {
                self.inventory_error = Some(error.to_string());
            }
            (_, Err(error)) => self.container_message = Some(error.to_string()),
            _ => {}
        }
    }

    pub fn close_container_detail(&mut self) {
        self.discard_logs = matches!(self.busy, Some(InfrastructureRequest::Logs(..)));
        self.logs = None;
        self.confirmation = None;
        self.container_scroll = 0;
    }

    pub fn switch_engine(&mut self) {
        self.engine = self.engine.other();
        self.rows = Arc::default();
        self.selected = 0;
        self.listed = false;
        self.limited = false;
        self.container_message = None;
        self.close_container_detail();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{Action, App, Effect, Section};

    #[test]
    fn rejects_short_ids_names_and_options() {
        for id in ["latest", "a1b2", "--all", &"x".repeat(64), &"A".repeat(64)] {
            assert!(ContainerId::parse(id).is_none());
        }
        assert!(ContainerId::parse(&"01abcdef".repeat(8)).is_some());
    }

    #[test]
    fn container_confirmation_pins_engine_and_id_and_is_required_once() {
        let mut app = App::default();
        let request = InfrastructureRequest::List(Engine::Docker);
        assert_eq!(
            app.update(Action::Select(Section::Containers)),
            Some(Effect::Infrastructure(request.clone()))
        );
        let id = ContainerId::parse(&"a".repeat(64)).unwrap_or_else(|| panic!("fixture"));
        let container = Container {
            id,
            name: "web".into(),
            state: "running".into(),
        };
        app.infrastructure.receive(
            request,
            Ok(InfrastructureData::Containers(
                vec![container.clone()],
                false,
            )),
        );
        let stop = Action::ContainerPropose(ContainerOperation::Stop);
        assert_eq!(app.update(Action::ContainerConfirm), None);
        assert_eq!(app.update(stop), None);
        assert_eq!(app.update(Action::SwitchEngine), None);
        assert_eq!(app.update(Action::Close), None);
        assert_eq!(app.update(Action::ContainerConfirm), None);
        app.update(stop);
        assert_eq!(
            app.update(Action::ContainerConfirm),
            Some(Effect::Infrastructure(InfrastructureRequest::Change(
                ContainerChange {
                    engine: Engine::Docker,
                    container,
                    operation: ContainerOperation::Stop
                }
            )))
        );
        assert_eq!(app.update(Action::ContainerConfirm), None);
        assert_eq!(
            app.update(Action::SwitchEngine),
            Some(Effect::Infrastructure(InfrastructureRequest::List(
                Engine::Podman
            )))
        );
        assert!(app.infrastructure.rows.is_empty());
    }
}
