use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serverui_core::application::{Action, App, Section};

pub fn action_for_app(key: KeyEvent, app: &App) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c' | 'C') => Some(Action::Quit),
            KeyCode::Char('u') if app.editing_filter => Some(Action::ClearFilter),
            _ => None,
        };
    }
    if key
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    if app.editing_filter && !app.help {
        return match key.code {
            KeyCode::Esc => Some(Action::Close),
            KeyCode::Enter => Some(Action::FinishFilter),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        };
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
