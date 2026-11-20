use crate::domain::ProviderError;
use std::sync::Arc;

pub const MAX_RESULTS: usize = 100;
pub const MAX_PLAN_LINES: usize = 100;
pub const MAX_LOG_LINES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageName(String);

impl PackageName {
    pub fn parse(value: &str) -> Option<Self> {
        let valid = (2..=128).contains(&value.len())
            && value.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
            && value.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'+' | b'-')
            });
        valid.then(|| Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageOperation {
    Install,
    Upgrade,
    Remove,
}

impl PackageOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "Instalar",
            Self::Upgrade => "Actualizar",
            Self::Remove => "Eliminar",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: PackageName,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagePlan {
    pub name: PackageName,
    pub operation: PackageOperation,
    pub changes: Arc<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageRequest {
    Recommendations,
    Search(PackageName),
    Preview(PackageName, PackageOperation),
    Apply(PackagePlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProgress {
    pub percent: Option<u8>,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReport {
    pub status: String,
    pub lines: Vec<String>,
    pub undo_install: Option<PackageName>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PackageData {
    Search(Vec<PackageInfo>, bool),
    Preview(PackagePlan),
    Applied(PackageReport),
}

pub type PackageResult = Result<PackageData, ProviderError>;

pub trait PackageProvider: Send {
    fn perform(
        &mut self,
        request: &PackageRequest,
        progress: &mut dyn FnMut(PackageProgress),
    ) -> PackageResult;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Packages {
    pub query: String,
    pub active_query: Option<PackageName>,
    pub editing: bool,
    pub rows: Arc<Vec<PackageInfo>>,
    pub selected: usize,
    pub limited: bool,
    pub searched: bool,
    pub busy: Option<PackageRequest>,
    pub confirmation: Option<PackagePlan>,
    pub progress: Option<PackageProgress>,
    pub report: Option<Arc<PackageReport>>,
    pub message: Option<String>,
    pub scroll: usize,
    discard_preview: bool,
}

impl Packages {
    pub fn search(&mut self) -> Option<PackageRequest> {
        self.editing = false;
        if self.query.is_empty() {
            return Some(PackageRequest::Recommendations);
        }
        let Some(query) = PackageName::parse(&self.query) else {
            self.message = Some("Escribe al menos 2 caracteres de un nombre APT".into());
            return None;
        };
        Some(PackageRequest::Search(query))
    }

    pub fn preview(&self, operation: PackageOperation) -> Option<PackageRequest> {
        let row = self.rows.get(self.selected)?;
        Some(PackageRequest::Preview(row.name.clone(), operation))
    }

    pub fn started(&mut self, request: PackageRequest) {
        if matches!(
            request,
            PackageRequest::Recommendations | PackageRequest::Search(_)
        ) {
            self.active_query = match &request {
                PackageRequest::Search(query) => Some(query.clone()),
                _ => None,
            };
            self.rows = Arc::default();
            self.selected = 0;
            self.limited = false;
            self.searched = false;
        }
        self.busy = Some(request);
        self.discard_preview = false;
        self.message = None;
        self.progress = None;
        self.report = None;
        self.scroll = 0;
    }

    pub fn receive(&mut self, result: PackageResult) {
        self.busy = None;
        self.progress = None;
        match result {
            Ok(PackageData::Search(rows, limited)) => {
                self.rows = Arc::new(rows);
                self.selected = 0;
                self.limited = limited;
                self.searched = true;
            }
            Ok(PackageData::Preview(_)) if self.discard_preview => {}
            Ok(PackageData::Preview(plan)) if plan.changes.is_empty() => {
                self.message = Some("APT no preve cambios para ese paquete".into());
            }
            Ok(PackageData::Preview(plan)) => self.confirmation = Some(plan),
            Ok(PackageData::Applied(report)) => self.report = Some(Arc::new(report)),
            Err(error) => self.message = Some(error.to_string()),
        }
    }

    pub fn select(&mut self, delta: isize) {
        let detail_lines = self
            .confirmation
            .as_ref()
            .map(|plan| plan.changes.len())
            .or_else(|| self.report.as_ref().map(|report| report.lines.len()));
        if let Some(lines) = detail_lines {
            self.scroll = self
                .scroll
                .saturating_add_signed(delta)
                .min(lines.saturating_sub(1));
        } else {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(self.rows.len().saturating_sub(1));
        }
    }

    pub fn close(&mut self) {
        self.discard_preview = matches!(self.busy, Some(PackageRequest::Preview(..)));
        self.confirmation = None;
        self.editing = false;
        self.report = None;
        self.scroll = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{Action, App, Effect, Section},
        domain::ProviderMode,
    };

    #[test]
    fn recommendations_load_on_first_entry_and_return_after_clearing_search() {
        let mut app = App::default();
        let open = Action::Select(Section::Packages);
        let recommendations = PackageRequest::Recommendations;
        assert_eq!(
            app.update(open),
            Some(Effect::Package(recommendations.clone()))
        );
        app.packages.started(recommendations.clone());
        assert_eq!(app.update(open), None);
        app.packages.receive(Ok(PackageData::Search(vec![], false)));
        assert_eq!(app.update(open), None);

        app.update(Action::StartFilter);
        for c in "git".chars() {
            app.update(Action::FilterInput(c));
        }
        let Some(Effect::Package(search @ PackageRequest::Search(_))) =
            app.update(Action::FinishFilter)
        else {
            panic!("expected a package search");
        };
        app.packages.started(search);
        app.packages.receive(Ok(PackageData::Search(vec![], false)));
        assert_eq!(
            app.packages.active_query.as_ref().map(PackageName::as_str),
            Some("git")
        );
        app.update(Action::StartFilter);
        app.update(Action::ClearFilter);
        assert_eq!(
            app.update(Action::FinishFilter),
            Some(Effect::Package(recommendations))
        );
    }

    #[test]
    fn package_names_reject_apt_expressions_paths_options_and_controls() {
        for invalid in [
            "",
            "a",
            "-apt",
            ".apt",
            "a*",
            "a?",
            "a:amd64",
            "a=1",
            "a/b",
            "a b",
            "apt\n",
            "Apt",
            "$(apt)",
            "apt;other",
        ] {
            assert!(PackageName::parse(invalid).is_none(), "{invalid:?}");
        }
        assert!(PackageName::parse("libc++-dev").is_some());
        assert!(PackageName::parse(&"a".repeat(129)).is_none());
    }

    #[test]
    fn preview_is_cancelable_and_only_one_confirmation_applies_the_exact_plan() {
        let name = PackageName::parse("demo-web").unwrap_or_else(|| panic!("fixture"));
        let mut app = App::new(ProviderMode::Demo);
        app.update(Action::Select(Section::Packages));
        app.packages.rows = Arc::new(vec![PackageInfo {
            name: name.clone(),
            description: String::new(),
        }]);
        let plan = PackagePlan {
            name: name.clone(),
            operation: PackageOperation::Remove,
            changes: Arc::new(vec!["Remv demo-web".into()]),
        };
        let preview = PackageRequest::Preview(name, PackageOperation::Remove);
        let proposed = Action::PackagePropose(PackageOperation::Remove);
        assert_eq!(app.update(proposed), Some(Effect::Package(preview.clone())));
        app.packages.started(preview.clone());
        app.update(Action::Close);
        app.packages.receive(Ok(PackageData::Preview(plan.clone())));
        assert_eq!(app.update(Action::PackageConfirm), None);
        app.packages.started(preview);
        app.packages.receive(Ok(PackageData::Preview(plan.clone())));
        assert_eq!(app.update(Action::Next), None);
        assert_eq!(app.section, Section::Packages);
        assert_eq!(
            app.update(Action::PackageConfirm),
            Some(Effect::Package(PackageRequest::Apply(plan.clone())))
        );
        assert_eq!(app.update(Action::PackageConfirm), None);
        app.packages.started(PackageRequest::Apply(plan));
        app.update(Action::Select(Section::Dashboard));
        assert_eq!(app.section, Section::Dashboard);
        app.update(Action::Quit);
        assert!(!app.quit);
        assert_eq!(app.section, Section::Packages);
    }
}
