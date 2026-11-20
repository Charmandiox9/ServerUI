use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serverui_core::{
    application::{Action, App, Section},
    infrastructure::{ContainerOperation, InventoryView},
    packages::PackageOperation,
    services::ServiceOperation,
};

pub fn action_for_app(key: KeyEvent, app: &App) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c' | 'C') => Some(Action::Quit),
            KeyCode::Char('u')
                if app.editing_filter || app.services.editing || app.packages.editing =>
            {
                Some(Action::ClearFilter)
            }
            _ => None,
        };
    }
    if key
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    if app.section == Section::Packages && app.packages.confirmation.is_some() && !app.help {
        return match key.code {
            KeyCode::Char('y') if key.kind == KeyEventKind::Press => Some(Action::PackageConfirm),
            KeyCode::Esc | KeyCode::Char('n') => Some(Action::Close),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NextRow),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::PreviousRow),
            _ => None,
        };
    }
    if app.section == Section::Containers && !app.help {
        if app.infrastructure.confirmation.is_some() {
            return match key.code {
                KeyCode::Char('y') if key.kind == KeyEventKind::Press => {
                    Some(Action::ContainerConfirm)
                }
                KeyCode::Esc | KeyCode::Char('n') => Some(Action::Close),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            };
        }
        let action = match key.code {
            KeyCode::Char('a') => Some(Action::ContainerPropose(ContainerOperation::Start)),
            KeyCode::Char('d') => Some(Action::ContainerPropose(ContainerOperation::Stop)),
            KeyCode::Char('R') => Some(Action::ContainerPropose(ContainerOperation::Restart)),
            KeyCode::Char('e') => Some(Action::SwitchEngine),
            _ => None,
        };
        if action.is_some() {
            return action;
        }
    }
    if app.section == Section::Inventory && !app.help {
        let view = match key.code {
            KeyCode::Char('n') => Some(InventoryView::Network),
            KeyCode::Char('b') => Some(InventoryView::Devices),
            KeyCode::Char('m') => Some(InventoryView::Mounts),
            _ => None,
        };
        if let Some(view) = view {
            return Some(Action::InventoryView(view));
        }
    }
    if app.section == Section::Services && app.services.confirmation.is_some() && !app.help {
        return match key.code {
            KeyCode::Char('y') if key.kind == KeyEventKind::Press => Some(Action::ServiceConfirm),
            KeyCode::Esc | KeyCode::Char('n') => Some(Action::Close),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NextRow),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::PreviousRow),
            _ => None,
        };
    }
    if (app.editing_filter
        || (app.section == Section::Services && app.services.editing)
        || (app.section == Section::Packages && app.packages.editing))
        && !app.help
    {
        return match key.code {
            KeyCode::Esc => Some(Action::Close),
            KeyCode::Enter => Some(Action::FinishFilter),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        };
    }
    if app.section == Section::Packages && !app.help {
        if key.code == KeyCode::Char('b') {
            return Some(Action::PackageUndo);
        }
        let operation = match key.code {
            KeyCode::Char('i') => Some(PackageOperation::Install),
            KeyCode::Char('u') => Some(PackageOperation::Upgrade),
            KeyCode::Char('d') => Some(PackageOperation::Remove),
            _ => None,
        };
        if let Some(operation) = operation {
            return Some(Action::PackagePropose(operation));
        }
    }
    if app.section == Section::Services && !app.help {
        if key.code == KeyCode::Char('h') {
            return Some(Action::ServiceHistory);
        }
        let operation = match key.code {
            KeyCode::Char('a') => Some(ServiceOperation::Start),
            KeyCode::Char('d') => Some(ServiceOperation::Stop),
            KeyCode::Char('R') => Some(ServiceOperation::Restart),
            KeyCode::Char('e') => Some(ServiceOperation::Enable),
            _ => None,
        };
        if let Some(operation) = operation {
            return Some(Action::ServicePropose(operation));
        }
    }
    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Esc => Some(Action::Close),
        KeyCode::BackTab | KeyCode::Left => Some(Action::Previous),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Previous),
        KeyCode::Tab | KeyCode::Right => Some(Action::Next),
        KeyCode::Char('1') => Some(Action::Select(Section::Dashboard)),
        KeyCode::Char('2') => Some(Action::Select(Section::Processes)),
        KeyCode::Char('3') => Some(Action::Select(Section::Services)),
        KeyCode::Char('4') => Some(Action::Select(Section::Settings)),
        KeyCode::Char('5') => Some(Action::Select(Section::Packages)),
        KeyCode::Char('6') => Some(Action::Select(Section::Containers)),
        KeyCode::Char('7') => Some(Action::Select(Section::Inventory)),
        KeyCode::Char('r') => Some(Action::Refresh),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::NextRow),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::PreviousRow),
        KeyCode::Home => Some(Action::FirstRow),
        KeyCode::End => Some(Action::LastRow),
        KeyCode::Enter => Some(Action::Inspect),
        KeyCode::Char('s') => Some(Action::CycleSort),
        KeyCode::Char('S') => Some(Action::ReverseSort),
        KeyCode::Char('/') => Some(Action::StartFilter),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    #[test]
    fn confirmation_captures_keys_and_rejects_repeat_and_alt() {
        let mut app = App {
            section: Section::Services,
            ..App::default()
        };
        let unit = serverui_core::services::UnitName::parse("fixture.service")
            .unwrap_or_else(|e| panic!("{e}"));
        app.services.confirmation = Some((unit, ServiceOperation::Stop));
        assert_eq!(
            action_for_app(key(KeyCode::Char('y')), &app),
            Some(Action::ServiceConfirm)
        );
        let mut repeated = key(KeyCode::Char('y'));
        repeated.kind = KeyEventKind::Repeat;
        assert_eq!(action_for_app(repeated, &app), None);
        for code in [
            KeyCode::Tab,
            KeyCode::Enter,
            KeyCode::Char('a'),
            KeyCode::Char('r'),
        ] {
            assert_eq!(action_for_app(key(code), &app), None);
        }
        assert_eq!(action_for_app(key(KeyCode::Esc), &app), Some(Action::Close));
        app.services.confirmation = None;
        app.services.editing = true;
        for c in ['a', 'd', 'R', 'e', 'y', 'q'] {
            assert_eq!(
                action_for_app(key(KeyCode::Char(c)), &app),
                Some(Action::FilterInput(c))
            );
        }
    }
    #[test]
    fn shortcuts_and_filter_capture() {
        let mut a = App::default();
        assert_eq!(
            action_for_app(key(KeyCode::Char('2')), &a),
            Some(Action::Select(Section::Processes))
        );
        a.section = Section::Processes;
        a.editing_filter = true;
        for c in ['q', 'r', '1', '?'] {
            assert_eq!(
                action_for_app(key(KeyCode::Char(c)), &a),
                Some(Action::FilterInput(c))
            );
        }
        assert_eq!(
            action_for_app(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), &a),
            Some(Action::Quit)
        );
        assert_eq!(
            action_for_app(key(KeyCode::Enter), &a),
            Some(Action::FinishFilter)
        );
        let mut release = key(KeyCode::Char('q'));
        release.kind = KeyEventKind::Release;
        assert_eq!(action_for_app(release, &a), None);
        assert_eq!(
            action_for_app(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT), &a),
            None
        );
    }
}
