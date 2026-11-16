use crate::domain::{ProviderError, Snapshot};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Next,
    Previous,
    Select(Section),
    ToggleHelp,
    CloseHelp,
    Refresh,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    QuerySnapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct App {
    pub section: Section,
    pub help: bool,
    pub quit: bool,
    pub status: DataStatus,
    pub snapshot: Option<Snapshot>,
}

impl App {
    pub fn update(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::Quit => self.quit = true,
            Action::ToggleHelp => self.help = !self.help,
            Action::CloseHelp => self.help = false,
            Action::Refresh => return Some(Effect::QuerySnapshot),
            _ if self.help => {}
            Action::Next => self.section = Section::ALL[(self.section.index() + 1) % 4],
            Action::Previous => self.section = Section::ALL[(self.section.index() + 3) % 4],
            Action::Select(section) => self.section = section,
        }
        None
    }

    pub fn receive(&mut self, result: Result<Option<Snapshot>, ProviderError>) {
        self.status = match result {
            Ok(Some(snapshot)) => {
                self.snapshot = Some(snapshot);
                DataStatus::Ready
            }
            Ok(None) => {
                self.snapshot = None;
                DataStatus::Empty
            }
            // Keep the last good sample visible and explicitly mark it stale.
            Err(error) => DataStatus::Error(error),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps_in_both_directions() {
        let mut app = App::default();
        app.update(Action::Previous);
        assert_eq!(app.section, Section::Settings);
        app.update(Action::Next);
        assert_eq!(app.section, Section::Dashboard);
        for section in Section::ALL {
            app.update(Action::Select(section));
            assert_eq!(app.section, section);
        }
    }

    #[test]
    fn help_captures_navigation_but_allows_exit() {
        let mut app = App::default();
        app.update(Action::ToggleHelp);
        app.update(Action::Next);
        assert_eq!(app.section, Section::Dashboard);
        app.update(Action::CloseHelp);
        app.update(Action::Next);
        assert_eq!(app.section, Section::Processes);
        app.update(Action::ToggleHelp);
        app.update(Action::Quit);
        assert!(app.quit);
    }

    #[test]
    fn refresh_is_a_typed_effect_and_error_is_explicit() {
        let mut app = App::default();
        assert_eq!(app.update(Action::Refresh), Some(Effect::QuerySnapshot));
        app.receive(Ok(None));
        assert_eq!(app.status, DataStatus::Empty);
        app.receive(Err(ProviderError::Unavailable("demo".into())));
        assert!(matches!(app.status, DataStatus::Error(_)));
        assert!(!app.quit);
    }
}
