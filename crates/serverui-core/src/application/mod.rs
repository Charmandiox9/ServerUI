use crate::domain::{ProcessId, ProcessInfo, ProviderError, ProviderMode, Sample, Snapshot};
use crate::services::{ServiceOperation, ServiceRequest, Services};
use std::{cmp::Ordering, sync::Arc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Dashboard,
    Processes,
    Services,
    Settings,
}
impl Section {
    pub const ALL: [Self; 4] = [
        Self::Dashboard,
        Self::Processes,
        Self::Services,
        Self::Settings,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Dashboard => 0,
            Self::Processes => 1,
            Self::Services => 2,
            Self::Settings => 3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Processes => "Procesos",
            Self::Services => "Servicios",
            Self::Settings => "Ajustes",
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
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    QuerySnapshot,
    Service(ServiceRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub services: Services,
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
            Action::Next => self.select_section(Section::ALL[(self.section.index() + 1) % 4]),
            Action::Previous => self.select_section(Section::ALL[(self.section.index() + 3) % 4]),
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
            | Action::ServiceHistory => {}
        }
        if self.section == Section::Services
            && !self.services.loaded
            && !self.services.busy
            && !self.help
            && matches!(action, Action::Select(_) | Action::Next | Action::Previous)
        {
            return Some(Effect::Service(ServiceRequest::List));
        }
        None
    }

    fn select_section(&mut self, section: Section) {
        self.services.editing = false;
        self.services.confirmation = None;
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
        assert_eq!(a.section, Section::Settings);
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
