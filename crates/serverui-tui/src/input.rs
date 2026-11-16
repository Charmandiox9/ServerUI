use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serverui_core::application::{Action, Section};

pub fn action_for(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c' | 'C') => Some(Action::Quit),
            _ => None,
        };
    }
    if key
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return None;
    }
    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Esc => Some(Action::CloseHelp),
        KeyCode::BackTab | KeyCode::Left => Some(Action::Previous),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Previous),
        KeyCode::Tab | KeyCode::Right => Some(Action::Next),
        KeyCode::Char('1') => Some(Action::Select(Section::Dashboard)),
        KeyCode::Char('2') => Some(Action::Select(Section::Processes)),
        KeyCode::Char('3') => Some(Action::Select(Section::Services)),
        KeyCode::Char('4') => Some(Action::Select(Section::Settings)),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_mapping_and_release_events() {
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)),
            Some(Action::Previous)
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE)),
            Some(Action::Select(Section::Settings))
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT)),
            None
        );
        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(action_for(release), None);
    }
}
