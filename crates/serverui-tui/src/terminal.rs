use std::io::{self, Stdout};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

trait TerminalControl {
    fn raw(&mut self, enabled: bool) -> io::Result<()>;
    fn alternate(&mut self, enabled: bool) -> io::Result<()>;
    fn cursor(&mut self, visible: bool) -> io::Result<()>;
}

struct NativeControl;

impl TerminalControl for NativeControl {
    fn raw(&mut self, enabled: bool) -> io::Result<()> {
        if enabled {
            enable_raw_mode()
        } else {
            disable_raw_mode()
        }
    }

    fn alternate(&mut self, enabled: bool) -> io::Result<()> {
        if enabled {
            execute!(io::stdout(), EnterAlternateScreen)
        } else {
            execute!(io::stdout(), LeaveAlternateScreen)
        }
    }

    fn cursor(&mut self, visible: bool) -> io::Result<()> {
        if visible {
            execute!(io::stdout(), Show)
        } else {
            execute!(io::stdout(), Hide)
        }
    }
}

/// Track attempted acquisitions too: a terminal write can partly succeed.
struct Guard<C: TerminalControl> {
    control: C,
    raw: bool,
    alternate: bool,
    cursor: bool,
}

impl<C: TerminalControl> Guard<C> {
    fn enter(control: C) -> io::Result<Self> {
        let mut guard = Self {
            control,
            raw: false,
            alternate: false,
            cursor: false,
        };
        guard.raw = true;
        guard.control.raw(true)?;
        guard.alternate = true;
        guard.control.alternate(true)?;
        guard.cursor = true;
        guard.control.cursor(false)?;
        Ok(guard)
    }

    fn close(&mut self) -> io::Result<()> {
        let mut first_error = None;
        // Try every restoration even if an earlier one fails. Failed operations
        // retain their flags so Drop can retry them once.
        if self.cursor {
            match self.control.cursor(true) {
                Ok(()) => self.cursor = false,
                Err(error) => first_error = Some(error),
            }
        }
        if self.alternate {
            match self.control.alternate(false) {
                Ok(()) => self.alternate = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        if self.raw {
            match self.control.raw(false) {
                Ok(()) => self.raw = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl<C: TerminalControl> Drop for Guard<C> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Call once, after TTY validation, before acquiring terminal modes.
/// Restore before the original hook prints the panic report.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut control = NativeControl;
        let _ = control.cursor(true);
        let _ = control.alternate(false);
        let _ = control.raw(false);
        previous(info);
    }));
}

pub fn run(body: impl FnOnce(&mut AppTerminal) -> io::Result<()>) -> io::Result<()> {
    let mut guard = Guard::enter(NativeControl)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = body(&mut terminal);
    drop(terminal);
    let cleanup = guard.close();
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(io::Error::other(format!(
            "{error}; error restaurando terminal: {cleanup}"
        ))),
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    struct Fake {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail: Option<&'static str>,
    }

    impl Fake {
        fn call(&mut self, name: &'static str) -> io::Result<()> {
            self.calls.borrow_mut().push(name);
            if self.fail == Some(name) {
                Err(io::Error::other(name))
            } else {
                Ok(())
            }
        }
    }

    impl TerminalControl for Fake {
        fn raw(&mut self, on: bool) -> io::Result<()> {
            self.call(if on { "raw+" } else { "raw-" })
        }
        fn alternate(&mut self, on: bool) -> io::Result<()> {
            self.call(if on { "alt+" } else { "alt-" })
        }
        fn cursor(&mut self, on: bool) -> io::Result<()> {
            self.call(if on { "show" } else { "hide" })
        }
    }

    #[test]
    fn partial_initialization_is_rolled_back() {
        for (failure, expected) in [
            ("raw+", vec!["raw+", "raw-"]),
            ("alt+", vec!["raw+", "alt+", "alt-", "raw-"]),
            ("hide", vec!["raw+", "alt+", "hide", "show", "alt-", "raw-"]),
        ] {
            let calls = Rc::new(RefCell::new(Vec::new()));
            assert!(
                Guard::enter(Fake {
                    calls: calls.clone(),
                    fail: Some(failure)
                })
                .is_err()
            );
            assert_eq!(*calls.borrow(), expected);
        }
    }

    #[test]
    fn cleanup_attempts_all_modes_and_retries_failures() -> io::Result<()> {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut guard = Guard::enter(Fake {
            calls: calls.clone(),
            fail: Some("show"),
        })?;
        assert!(guard.close().is_err());
        drop(guard);
        assert_eq!(
            *calls.borrow(),
            ["raw+", "alt+", "hide", "show", "alt-", "raw-", "show"]
        );
        Ok(())
    }

    #[test]
    fn unwind_restores_modes() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = Guard::enter(Fake {
                calls: calls.clone(),
                fail: None,
            });
            panic!("injected panic");
        }));
        assert!(result.is_err());
        assert_eq!(
            *calls.borrow(),
            ["raw+", "alt+", "hide", "show", "alt-", "raw-"]
        );
    }

    #[test]
    fn successful_close_is_idempotent() -> io::Result<()> {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut guard = Guard::enter(Fake {
            calls: calls.clone(),
            fail: None,
        })?;
        guard.close()?;
        guard.close()?;
        drop(guard);
        assert_eq!(calls.borrow().len(), 6);
        Ok(())
    }
}
