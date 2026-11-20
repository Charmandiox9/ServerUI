use crate::domain::{ProcessId, ProcessInfo, ProviderError, ProviderMode, Sample, Snapshot};
use crate::infrastructure::{
    ContainerChange, ContainerOperation, Infrastructure, InfrastructureRequest, InventoryView,
};
use crate::packages::{PackageOperation, PackageRequest, Packages};
use crate::services::{ServiceOperation, ServiceRequest, Services};
use std::{cmp::Ordering, sync::Arc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Dashboard,
    Processes,
    Services,
    Settings,
    Packages,
    Containers,
    Inventory,
}
impl Section {
    pub const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Processes,
        Self::Services,
        Self::Settings,
        Self::Packages,
        Self::Containers,
        Self::Inventory,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Dashboard => 0,
            Self::Processes => 1,
            Self::Services => 2,
            Self::Settings => 3,
            Self::Packages => 4,
            Self::Containers => 5,
            Self::Inventory => 6,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Processes => "Procesos",
            Self::Services => "Servicios",
            Self::Settings => "Ajustes",
            Self::Packages => "Paquetes",
            Self::Containers => "Contenedores",
            Self::Inventory => "Red y discos",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DataStatus {
    #[default]
    Loading,
    Ready,
    Empty,
    Error(ProviderError),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Pid,
    Name,
}
impl ProcessSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "RAM",
            Self::Pid => "PID",
            Self::Name => "Nombre",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Pid,
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Next,
    Previous,
    Select(Section),
    ToggleHelp,
    Close,
    Refresh,
    Quit,
    NextRow,
    PreviousRow,
    FirstRow,
    LastRow,
    Inspect,
    CycleSort,
    ReverseSort,
    StartFilter,
    FinishFilter,
    FilterInput(char),
    FilterBackspace,
    ClearFilter,
    ServicePropose(ServiceOperation),
    ServiceConfirm,
    ServiceHistory,
    PackagePropose(PackageOperation),
    PackageConfirm,
    PackageUndo,
    ContainerPropose(ContainerOperation),
    ContainerConfirm,
    SwitchEngine,
    InventoryView(InventoryView),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    QuerySnapshot,
    Service(ServiceRequest),
    Package(PackageRequest),
    Infrastructure(InfrastructureRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub services: Services,
    pub packages: Packages,
    pub infrastructure: Infrastructure,
    pub mode: ProviderMode,
    pub section: Section,
    pub help: bool,
    pub quit: bool,
    pub refreshing: bool,
    pub status: DataStatus,
    pub snapshot: Option<Arc<Snapshot>>,
    pub filter: String,
    pub editing_filter: bool,
    pub sort: ProcessSort,
    pub descending: bool,
    pub selected: Option<ProcessId>,
    pub inspecting: Option<ProcessId>,
    pub notice: Option<String>,
    pub dashboard_scroll: usize,
}
impl Default for App {
    fn default() -> Self {
        Self::new(ProviderMode::Demo)
    }
}
impl App {
    pub fn new(mode: ProviderMode) -> Self {
        Self {
            services: Services::default(),
            packages: Packages::default(),
            infrastructure: Infrastructure::default(),
            mode,
            section: Section::Dashboard,
            help: false,
            quit: false,
            refreshing: false,
            status: DataStatus::Loading,
            snapshot: None,
            filter: String::new(),
            editing_filter: false,
            sort: ProcessSort::Cpu,
            descending: true,
            selected: None,
            inspecting: None,
            notice: None,
            dashboard_scroll: 0,
        }
    }

    pub fn update(&mut self, action: Action) -> Option<Effect> {
        if self.section == Section::Containers && !self.help && action != Action::Quit {
            return self.update_containers(action);
        }
        if self.section == Section::Inventory && !self.help {
            match action {
                Action::InventoryView(view) => {
                    self.infrastructure.view = view;
                    self.infrastructure.inventory_scroll = 0;
                    return None;
                }
                Action::NextRow | Action::PreviousRow => {
                    let delta = if action == Action::NextRow { 1 } else { -1 };
                    let state = &mut self.infrastructure;
                    let maximum = match state.view {
                        InventoryView::Mounts => {
                            self.snapshot.as_ref().map_or(0, |s| s.disks.len())
                        }
                        InventoryView::Network => state
                            .inventory
                            .as_ref()
                            .and_then(|s| s.interfaces.as_ref().ok())
                            .map_or(0, Vec::len),
                        InventoryView::Devices => state
                            .inventory
                            .as_ref()
                            .and_then(|s| s.devices.as_ref().ok())
                            .map_or(0, Vec::len),
                    }
                    .saturating_sub(1);
                    state.inventory_scroll = state
                        .inventory_scroll
                        .saturating_add_signed(delta)
                        .min(maximum);
                    return None;
                }
                Action::Refresh => {
                    if self.infrastructure.view == InventoryView::Mounts {
                        return Some(Effect::QuerySnapshot);
                    }
                    return self
                        .infrastructure
                        .busy
                        .is_none()
                        .then_some(Effect::Infrastructure(InfrastructureRequest::Inventory));
                }
                _ => {}
            }
        }
        if action == Action::Quit && matches!(self.packages.busy, Some(PackageRequest::Apply(_))) {
            self.help = false;
            self.select_section(Section::Packages);
            return None;
        }
        if self.section == Section::Packages && !self.help && action != Action::Quit {
            let packages = &mut self.packages;
            let request = match action {
                Action::Close => {
                    packages.close();
                    return None;
                }
                Action::NextRow | Action::PreviousRow => {
                    packages.select(if action == Action::NextRow { 1 } else { -1 });
                    return None;
                }
                Action::PackageConfirm => {
                    return packages
                        .confirmation
                        .take()
                        .map(PackageRequest::Apply)
                        .map(Effect::Package);
                }
                _ if packages.confirmation.is_some() => return None,
                Action::Refresh | Action::FinishFilter if packages.busy.is_none() => {
                    packages.search()
                }
                Action::PackagePropose(operation)
                    if packages.busy.is_none()
                        && packages.report.is_none()
                        && !packages.editing =>
                {
                    packages.preview(operation)
                }
                Action::PackageUndo => packages
                    .report
                    .as_ref()
                    .and_then(|report| report.undo_install.clone())
                    .map(|name| PackageRequest::Preview(name, PackageOperation::Remove)),
                Action::StartFilter if packages.busy.is_none() => {
                    packages.close();
                    packages.editing = true;
                    return None;
                }
                Action::FilterInput(c) if packages.editing => {
                    if (c.is_ascii_lowercase()
                        || c.is_ascii_digit()
                        || matches!(c, '.' | '+' | '-'))
                        && packages.query.len() < 128
                    {
                        packages.query.push(c);
                    }
                    return None;
                }
                Action::FilterBackspace if packages.editing => {
                    packages.query.pop();
                    return None;
                }
                Action::ClearFilter if packages.editing => {
                    packages.query.clear();
                    return None;
                }
                Action::Refresh => return None,
                _ => return self.update_general(action),
            };
            return request.map(Effect::Package);
        }
        if self.section == Section::Services && !self.help && action != Action::Quit {
            if self.services.confirmation.is_some() {
                return match action {
                    Action::ServiceConfirm => self.services.confirm().map(Effect::Service),
                    Action::NextRow => {
                        self.services.log_scroll =
                            self.services.log_scroll.saturating_add(1).min(256);
                        None
                    }
                    Action::PreviousRow => {
                        self.services.log_scroll = self.services.log_scroll.saturating_sub(1);
                        None
                    }
                    Action::Close => {
                        self.services.confirmation = None;
                        None
                    }
                    _ => None,
                };
            }
            let mut handled = true;
            let effect = match action {
                Action::Refresh if !self.services.busy && !self.services.show_history => {
                    Some(Effect::Service(match &self.services.logs {
                        Some((unit, _)) => ServiceRequest::Logs(unit.clone()),
                        None => ServiceRequest::List,
                    }))
                }
                Action::ServicePropose(operation) => {
                    self.services.propose(operation);
                    None
                }
                Action::ServiceHistory => {
                    self.services.show_history = !self.services.show_history;
                    self.services.log_scroll = 0;
                    None
                }
                Action::ServiceConfirm => None,
                Action::NextRow => {
                    self.services.select(1);
                    None
                }
                Action::PreviousRow => {
                    self.services.select(-1);
                    None
                }
                Action::FirstRow => {
                    self.services.select(isize::MIN);
                    None
                }
                Action::LastRow => {
                    self.services.select(isize::MAX);
                    None
                }
                Action::Inspect
                    if !self.services.busy
                        && self.services.logs.is_none()
                        && !self.services.show_history =>
                {
                    self.services
                        .selected
                        .clone()
                        .map(ServiceRequest::Logs)
                        .map(Effect::Service)
                }
                Action::Close => {
                    self.services.show_history = false;
                    self.services.logs = None;
                    self.services.editing = false;
                    None
                }
                Action::StartFilter
                    if self.services.logs.is_none() && !self.services.show_history =>
                {
                    self.services.editing = true;
                    None
                }
                Action::FinishFilter => {
                    self.services.editing = false;
                    None
                }
                Action::FilterInput(c)
                    if !c.is_control() && self.services.filter.chars().count() < 64 =>
                {
                    self.services.filter.push(c);
                    self.services.reconcile();
                    None
                }
                Action::FilterBackspace => {
                    self.services.filter.pop();
                    self.services.reconcile();
                    None
                }
                Action::ClearFilter => {
                    self.services.filter.clear();
                    self.services.reconcile();
                    None
                }
                _ => {
                    handled = false;
                    None
                }
            };
            if handled {
                return effect;
            }
        }
        self.update_general(action)
    }

    fn update_containers(&mut self, action: Action) -> Option<Effect> {
        let state = &mut self.infrastructure;
        if state.confirmation.is_some() {
            return match action {
                Action::Close => {
                    state.close_container_detail();
                    None
                }
                Action::ContainerConfirm => state
                    .confirmation
                    .take()
                    .map(InfrastructureRequest::Change)
                    .map(Effect::Infrastructure),
                _ => None,
            };
        }
        let request = match action {
            Action::Close => {
                state.close_container_detail();
                return None;
            }
            Action::NextRow | Action::PreviousRow => {
                let delta = if action == Action::NextRow { 1 } else { -1 };
                if let Some((_, logs)) = &state.logs {
                    state.container_scroll = state
                        .container_scroll
                        .saturating_add_signed(delta)
                        .min(logs.len().saturating_sub(1));
                } else {
                    state.selected = state
                        .selected
                        .saturating_add_signed(delta)
                        .min(state.rows.len().saturating_sub(1));
                }
                return None;
            }
            Action::SwitchEngine if state.busy.is_none() => {
                state.switch_engine();
                Some(InfrastructureRequest::List(state.engine))
            }
            Action::Refresh if state.busy.is_none() => Some(match &state.logs {
                Some((id, _)) => InfrastructureRequest::Logs(state.engine, id.clone()),
                None => InfrastructureRequest::List(state.engine),
            }),
            Action::Inspect if state.busy.is_none() && state.logs.is_none() => state
                .rows
                .get(state.selected)
                .map(|row| InfrastructureRequest::Logs(state.engine, row.id.clone())),
            Action::ContainerPropose(operation) if state.busy.is_none() && state.logs.is_none() => {
                state.confirmation =
                    state
                        .rows
                        .get(state.selected)
                        .cloned()
                        .map(|container| ContainerChange {
                            engine: state.engine,
                            container,
                            operation,
                        });
                return None;
            }
            Action::Refresh => return None,
            _ => return self.update_general(action),
        };
        request.map(Effect::Infrastructure)
    }

    pub fn infrastructure_on_entry(&self) -> Option<InfrastructureRequest> {
        let state = &self.infrastructure;
        if self.help || state.busy.is_some() {
            return None;
        }
        match self.section {
            Section::Containers if !state.listed => Some(InfrastructureRequest::List(state.engine)),
            Section::Inventory if state.inventory.is_none() && state.inventory_error.is_none() => {
                Some(InfrastructureRequest::Inventory)
            }
            _ => None,
        }
    }

    fn update_general(&mut self, action: Action) -> Option<Effect> {
        let count = Section::ALL.len();
        match action {
            Action::Quit => self.quit = true,
            Action::Refresh => return Some(Effect::QuerySnapshot),
            Action::ToggleHelp => self.help = !self.help,
            Action::Close => {
                if self.help {
                    self.help = false;
                } else if self.editing_filter {
                    self.editing_filter = false;
                } else {
                    self.inspecting = None;
                    self.notice = None;
                }
            }
            _ if self.help => {}
            Action::Next => self.select_section(Section::ALL[(self.section.index() + 1) % count]),
            Action::Previous => {
                self.select_section(Section::ALL[(self.section.index() + count - 1) % count])
            }
            Action::Select(section) => self.select_section(section),
            Action::NextRow if self.section == Section::Dashboard => {
                self.dashboard_scroll = self.dashboard_scroll.saturating_add(1).min(
                    self.snapshot
                        .as_ref()
                        .map_or(0, |s| s.disks.len().saturating_sub(1)),
                )
            }
            Action::PreviousRow if self.section == Section::Dashboard => {
                self.dashboard_scroll = self.dashboard_scroll.saturating_sub(1)
            }
            Action::FirstRow if self.section == Section::Dashboard => self.dashboard_scroll = 0,
            Action::LastRow if self.section == Section::Dashboard => {
                self.dashboard_scroll = self
                    .snapshot
                    .as_ref()
                    .map_or(0, |s| s.disks.len().saturating_sub(1))
            }
            _ if self.section != Section::Processes => {}
            Action::NextRow => self.move_selection(1),
            Action::PreviousRow => self.move_selection(-1),
            Action::FirstRow => self.selected = self.visible_processes().first().map(|p| p.id),
            Action::LastRow => self.selected = self.visible_processes().last().map(|p| p.id),
            Action::Inspect => {
                self.inspecting = self.selected;
                self.notice = None;
            }
            Action::CycleSort => {
                self.sort = self.sort.next();
                self.descending = matches!(self.sort, ProcessSort::Cpu | ProcessSort::Memory);
                self.reconcile_selection();
            }
            Action::ReverseSort => {
                self.descending = !self.descending;
            }
            Action::StartFilter => {
                self.editing_filter = true;
                self.inspecting = None;
                self.notice = None;
            }
            Action::FinishFilter => self.editing_filter = false,
            Action::FilterInput(c) if !c.is_control() && self.filter.chars().count() < 64 => {
                self.filter.push(c);
                self.reconcile_selection();
            }
            Action::FilterBackspace => {
                self.filter.pop();
                self.reconcile_selection();
            }
            Action::ClearFilter => {
                self.filter.clear();
                self.reconcile_selection();
            }
            Action::FilterInput(_)
            | Action::ServicePropose(_)
            | Action::ServiceConfirm
            | Action::ServiceHistory
            | Action::PackageConfirm
            | Action::PackageUndo
            | Action::PackagePropose(_)
            | Action::ContainerPropose(_)
            | Action::ContainerConfirm
            | Action::SwitchEngine
            | Action::InventoryView(_) => {}
        }
        if self.section == Section::Services
            && !self.services.loaded
            && !self.services.busy
            && !self.help
            && matches!(action, Action::Select(_) | Action::Next | Action::Previous)
        {
            return Some(Effect::Service(ServiceRequest::List));
        }
        if self.section == Section::Packages
            && !self.packages.searched
            && self.packages.busy.is_none()
            && self.packages.message.is_none()
            && matches!(action, Action::Select(_) | Action::Next | Action::Previous)
        {
            return Some(Effect::Package(PackageRequest::Recommendations));
        }
        if matches!(
            action,
            Action::Select(_)
                | Action::Next
                | Action::Previous
                | Action::ToggleHelp
                | Action::Close
        ) {
            return self.infrastructure_on_entry().map(Effect::Infrastructure);
        }
        None
    }

    fn select_section(&mut self, section: Section) {
        self.services.editing = false;
        self.services.confirmation = None;
        if self.section == Section::Packages && section != Section::Packages {
            self.packages.close();
        }
        if self.section == Section::Containers && section != Section::Containers {
            self.infrastructure.close_container_detail();
        }
        self.section = section;
        self.editing_filter = false;
        self.inspecting = None;
        self.notice = None;
    }

    fn move_selection(&mut self, delta: isize) {
        let rows = self.visible_processes();
        if rows.is_empty() {
            self.selected = None;
            return;
        }
        let index = rows
            .iter()
            .position(|p| Some(p.id) == self.selected)
            .unwrap_or(0);
        let next = index.saturating_add_signed(delta).min(rows.len() - 1);
        self.selected = Some(rows[next].id);
    }

    fn reconcile_selection(&mut self) {
        let rows = self.visible_processes();
        if !rows.iter().any(|p| Some(p.id) == self.selected) {
            self.selected = rows.first().map(|p| p.id);
        }
    }

    pub fn visible_processes(&self) -> Vec<&ProcessInfo> {
        let filter = self.filter.to_lowercase();
        let mut rows: Vec<_> = self
            .snapshot
            .iter()
            .flat_map(|s| s.processes.iter())
            .filter(|p| {
                filter.is_empty()
                    || p.name.to_lowercase().contains(&filter)
                    || p.id.pid.to_string().contains(&filter)
            })
            .collect();
        rows.sort_by(|a, b| {
            // Unknown CPU always sorts last in either direction.
            if self.sort == ProcessSort::Cpu {
                match (a.cpu_usage, b.cpu_usage) {
                    (None, Some(_)) => return Ordering::Greater,
                    (Some(_), None) => return Ordering::Less,
                    _ => {}
                }
            }
            let order = match self.sort {
                ProcessSort::Cpu => a.cpu_usage.cmp(&b.cpu_usage),
                ProcessSort::Memory => a.memory_bytes.cmp(&b.memory_bytes),
                ProcessSort::Pid => a.id.pid.cmp(&b.id.pid),
                ProcessSort::Name => a.name.cmp(&b.name),
            };
            let order = if self.descending {
                order.reverse()
            } else {
                order
            };
            order.then_with(|| a.id.cmp(&b.id))
        });
        rows
    }

    pub fn inspected_process(&self) -> Option<&ProcessInfo> {
        self.snapshot
            .as_ref()?
            .processes
            .iter()
            .find(|p| Some(p.id) == self.inspecting)
    }

    pub fn receive(&mut self, result: Sample) {
        self.refreshing = false;
        self.status = match result {
            Ok(Some(snapshot)) => {
                self.snapshot = Some(Arc::new(snapshot));
                self.dashboard_scroll = self.dashboard_scroll.min(
                    self.snapshot
                        .as_ref()
                        .map_or(0, |s| s.disks.len().saturating_sub(1)),
                );
                if self.inspecting.is_some() && self.inspected_process().is_none() {
                    self.inspecting = None;
                    self.notice = Some("El proceso inspeccionado ya no existe".into());
                }
                self.reconcile_selection();
                DataStatus::Ready
            }
            Ok(None) => {
                self.snapshot = None;
                self.selected = None;
                self.inspecting = None;
                DataStatus::Empty
            }
            Err(error) => DataStatus::Error(error),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_navigation_confirmation_and_logs_are_pure_effects() {
        use crate::services::{ServiceData, ServiceInfo, UnitName};
        let mut app = App::default();
        assert_eq!(
            app.update(Action::Select(Section::Services)),
            Some(Effect::Service(ServiceRequest::List))
        );
        let unit = UnitName::parse("fixture.service").unwrap_or_else(|e| panic!("{e}"));
        app.services.receive(
            ServiceRequest::List,
            Ok(ServiceData::List {
                units: vec![ServiceInfo {
                    unit: unit.clone(),
                    load: "loaded".into(),
                    active: "active".into(),
                    sub: "running".into(),
                    description: "Fixture".into(),
                }],
                limited: false,
            }),
        );
        assert_eq!(
            app.update(Action::ServicePropose(ServiceOperation::Stop)),
            None
        );
        app.update(Action::Next);
        assert_eq!(app.section, Section::Services);
        app.update(Action::Close);
        assert_eq!(app.update(Action::ServiceConfirm), None);
        app.update(Action::ServicePropose(ServiceOperation::Restart));
        assert_eq!(
            app.update(Action::ServiceConfirm),
            Some(Effect::Service(ServiceRequest::Execute(
                unit.clone(),
                ServiceOperation::Restart
            )))
        );
        assert_eq!(app.update(Action::ServiceConfirm), None);
        assert_eq!(
            app.update(Action::Inspect),
            Some(Effect::Service(ServiceRequest::Logs(unit)))
        );
        app.services.busy = true;
        app.update(Action::ServicePropose(ServiceOperation::Stop));
        assert_eq!(app.services.confirmation, None);
        app.update(Action::Quit);
        assert!(app.quit);
    }
    fn sample() -> Snapshot {
        Snapshot {
            processes: [
                (10, "alpha", Some(2000), 200),
                (20, "beta", Some(2000), 100),
                (30, "ALPHA", None, 300),
            ]
            .into_iter()
            .map(|(pid, name, cpu_usage, memory_bytes)| ProcessInfo {
                id: ProcessId { pid, start_time: 1 },
                parent_pid: None,
                name: name.into(),
                status: "Running".into(),
                cpu_usage,
                memory_bytes,
                virtual_memory_bytes: memory_bytes,
                run_time_seconds: 1,
            })
            .collect(),
            process_count: 3,
            ..Snapshot::default()
        }
    }
    fn app() -> App {
        let mut app = App::default();
        app.receive(Ok(Some(sample())));
        app.update(Action::Select(Section::Processes));
        app
    }
    #[test]
    fn navigation_and_help() {
        let mut a = App::default();
        a.update(Action::Previous);
        assert_eq!(a.section, Section::Inventory);
        a.update(Action::Next);
        assert_eq!(a.section, Section::Dashboard);
        a.update(Action::ToggleHelp);
        assert_eq!(a.update(Action::Refresh), Some(Effect::QuerySnapshot));
        a.update(Action::Next);
        assert_eq!(a.section, Section::Dashboard);
        a.update(Action::Close);
        a.update(Action::Select(Section::Processes));
        assert_eq!(a.section, Section::Processes);
        a.update(Action::ToggleHelp);
        a.update(Action::Quit);
        assert!(a.quit);
    }
    #[test]
    fn filter_name_pid_clear_and_limit() {
        let mut a = app();
        for c in "alpha".chars() {
            a.update(Action::FilterInput(c));
        }
        assert_eq!(
            a.visible_processes()
                .iter()
                .map(|p| p.id.pid)
                .collect::<Vec<_>>(),
            [10, 30]
        );
        a.update(Action::ClearFilter);
        a.update(Action::FilterInput('2'));
        assert_eq!(a.visible_processes()[0].id.pid, 20);
        a.update(Action::FilterBackspace);
        assert_eq!(a.visible_processes().len(), 3);
        for _ in 0..100 {
            a.update(Action::FilterInput('界'));
        }
        assert_eq!(a.filter.chars().count(), 64);
        a.update(Action::FilterInput('\n'));
        assert!(!a.filter.contains('\n'));
    }
    #[test]
    fn stable_sort_and_unknown_cpu_last() {
        let mut a = app();
        assert_eq!(
            a.visible_processes()
                .iter()
                .map(|p| p.id.pid)
                .collect::<Vec<_>>(),
            [10, 20, 30]
        );
        a.update(Action::ReverseSort);
        assert_eq!(a.visible_processes().last().map(|p| p.id.pid), Some(30));
        a.update(Action::CycleSort);
        assert_eq!(a.visible_processes()[0].id.pid, 30);
        a.update(Action::CycleSort);
        assert_eq!(a.visible_processes()[0].id.pid, 10);
    }
    #[test]
    fn identity_survives_sort_but_reused_pid_closes_inspection() {
        let mut a = app();
        a.update(Action::NextRow);
        let id = a.selected;
        a.update(Action::Inspect);
        a.update(Action::CycleSort);
        assert_eq!(a.selected, id);
        let mut s = sample();
        s.processes
            .iter_mut()
            .filter(|p| Some(p.id) == id)
            .for_each(|p| p.id.start_time = 2);
        a.receive(Ok(Some(s)));
        assert_eq!(a.inspecting, None);
        assert!(a.notice.is_some());
    }
    #[test]
    fn selection_reconciles_removal_and_empty_filter() {
        let mut a = app();
        a.update(Action::LastRow);
        assert_eq!(a.selected.map(|id| id.pid), Some(30));
        let mut s = sample();
        s.processes.retain(|p| p.id.pid != 30);
        a.receive(Ok(Some(s)));
        assert_eq!(a.selected.map(|id| id.pid), Some(10));
        a.update(Action::FilterInput('!'));
        assert_eq!(a.selected, None);
        a.update(Action::NextRow);
        a.update(Action::Inspect);
        assert_eq!(a.inspecting, None);
    }
    #[test]
    fn failed_sample_retains_data_without_copying_on_input() {
        let mut a = app();
        let before = a.snapshot.clone();
        a.update(Action::NextRow);
        assert!(Arc::ptr_eq(
            a.snapshot.as_ref().unwrap_or_else(|| panic!("snapshot")),
            before.as_ref().unwrap_or_else(|| panic!("snapshot"))
        ));
        a.receive(Err(ProviderError::Unavailable("test".into())));
        assert!(matches!(a.status, DataStatus::Error(_)));
        assert_eq!(a.snapshot, before);
        a.receive(Ok(None));
        assert_eq!(a.status, DataStatus::Empty);
        assert_eq!(a.snapshot, None);
    }

    #[test]
    fn dashboard_mount_scroll_is_bounded_across_refresh() {
        use crate::domain::DiskInfo;
        let mut a = App::default();
        let mut s = sample();
        s.disks = (0..3)
            .map(|i| DiskInfo {
                mount: format!("/disk{i}"),
                space: None,
            })
            .collect();
        a.receive(Ok(Some(s.clone())));
        a.update(Action::LastRow);
        assert_eq!(a.dashboard_scroll, 2);
        a.update(Action::NextRow);
        assert_eq!(a.dashboard_scroll, 2);
        a.update(Action::PreviousRow);
        assert_eq!(a.dashboard_scroll, 1);
        s.disks.clear();
        a.receive(Ok(Some(s)));
        assert_eq!(a.dashboard_scroll, 0);
    }

    #[test]
    fn selection_survives_reordering_a_full_snapshot() {
        let mut s = sample();
        let template = s.processes[0].clone();
        s.processes = (1..=10000)
            .map(|pid| {
                let mut p = template.clone();
                p.id.pid = pid;
                p
            })
            .collect();
        s.process_count = s.processes.len();
        let mut a = App::default();
        a.receive(Ok(Some(s.clone())));
        a.update(Action::Select(Section::Processes));
        a.update(Action::LastRow);
        let selected = a.selected;
        s.processes.reverse();
        s.processes[0].cpu_usage = Some(9999);
        a.receive(Ok(Some(s)));
        assert_eq!(a.selected, selected);
        assert_eq!(a.visible_processes().len(), 10000);
        assert_eq!(a.visible_processes()[0].id.pid, 10000);
    }
}
