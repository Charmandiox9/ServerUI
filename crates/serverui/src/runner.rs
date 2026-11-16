use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event};
use serverui_core::{
    application::{Action, App, Effect},
    domain::SystemProvider,
};
use serverui_tui::input::action_for;

trait EventSource {
    fn now(&self) -> Duration;
    fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>>;
}

pub struct TerminalEvents {
    start: Instant,
}

impl Default for TerminalEvents {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl EventSource for TerminalEvents {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        if event::poll(timeout)? {
            event::read().map(Some)
        } else {
            Ok(None)
        }
    }
}

// The only caller outside this module uses TerminalEvents; the private trait
// keeps fake event clocks local to tests without exposing a public framework.
pub fn run(
    provider: &mut impl SystemProvider,
    events: &mut TerminalEvents,
    interval: Duration,
    render: impl FnMut(&App) -> io::Result<()>,
) -> io::Result<App> {
    run_loop(provider, events, interval, render)
}

fn run_loop(
    provider: &mut impl SystemProvider,
    events: &mut impl EventSource,
    interval: Duration,
    mut render: impl FnMut(&App) -> io::Result<()>,
) -> io::Result<App> {
    if interval.is_zero() {
        return Err(io::Error::other("el intervalo debe ser positivo"));
    }
    let mut app = App::default();
    apply(&mut app, Action::Refresh, provider);
    let mut deadline = events.now().saturating_add(interval);
    let mut dirty = true;
    while !app.quit {
        // Check time before every input: a busy keyboard cannot starve refresh.
        let now = events.now();
        if now >= deadline {
            dirty |= apply(&mut app, Action::Refresh, provider);
            deadline = now.saturating_add(interval);
        }
        if dirty {
            render(&app)?;
            dirty = false;
        }
        let timeout = deadline.saturating_sub(events.now());
        match events.next(timeout)? {
            Some(Event::Key(key)) => {
                if let Some(action) = action_for(key) {
                    dirty |= apply(&mut app, action, provider);
                }
            }
            Some(Event::Resize(_, _)) => dirty = true,
            _ => {}
        }
    }
    Ok(app)
}

fn apply(app: &mut App, action: Action, provider: &mut impl SystemProvider) -> bool {
    let before = app.clone();
    if let Some(Effect::QuerySnapshot) = app.update(action) {
        app.receive(provider.snapshot());
    }
    *app != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serverui_core::application::{DataStatus, Section};
    use serverui_providers::{MockProvider, Scenario};
    use std::collections::VecDeque;

    struct FakeEvents {
        now: Duration,
        queue: VecDeque<(Duration, Option<Event>)>,
    }

    impl EventSource for FakeEvents {
        fn now(&self) -> Duration {
            self.now
        }
        fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            let Some((delay, event)) = self.queue.front_mut() else {
                return Err(io::Error::other("test event source exhausted"));
            };
            if *delay > timeout {
                *delay -= timeout;
                self.now += timeout;
                return Ok(None);
            }
            self.now += *delay;
            let result = event.take();
            self.queue.pop_front();
            Ok(result)
        }
    }

    fn key(code: KeyCode) -> Option<Event> {
        Some(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
    }

    #[test]
    fn tick_navigation_and_resize_are_deterministic() -> io::Result<()> {
        let mut events = FakeEvents {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::ZERO, key(KeyCode::Tab)),
                (Duration::ZERO, Some(Event::Resize(40, 12))),
                (Duration::from_millis(110), key(KeyCode::Char('q'))),
            ]),
        };
        let mut frames = Vec::new();
        let app = run_loop(
            &mut MockProvider::default(),
            &mut events,
            Duration::from_millis(100),
            |app| {
                frames.push(app.clone());
                Ok(())
            },
        )?;
        assert!(app.quit);
        assert_eq!(app.section, Section::Processes);
        assert_eq!(frames.len(), 4);
        assert_ne!(frames[0].snapshot, frames[3].snapshot);
        Ok(())
    }

    #[test]
    fn busy_input_does_not_starve_refresh() -> io::Result<()> {
        let mut queue = VecDeque::new();
        for _ in 0..20 {
            queue.push_back((Duration::from_millis(25), key(KeyCode::Right)));
        }
        queue.push_back((Duration::ZERO, key(KeyCode::Char('q'))));
        let mut events = FakeEvents {
            now: Duration::ZERO,
            queue,
        };
        let app = run_loop(
            &mut MockProvider::default(),
            &mut events,
            Duration::from_millis(100),
            |_| Ok(()),
        )?;
        let Some(snapshot) = app.snapshot else {
            panic!("expected sample")
        };
        assert_eq!(snapshot.uptime_seconds, 86_405);
        Ok(())
    }

    #[test]
    fn unchanged_empty_data_does_not_redraw_on_ticks() -> io::Result<()> {
        let mut events = FakeEvents {
            now: Duration::ZERO,
            queue: VecDeque::from([(Duration::from_millis(350), key(KeyCode::Char('q')))]),
        };
        let mut frames = 0;
        run_loop(
            &mut MockProvider::new(Scenario::Empty),
            &mut events,
            Duration::from_millis(100),
            |_| {
                frames += 1;
                Ok(())
            },
        )?;
        assert_eq!(frames, 1);
        Ok(())
    }

    #[test]
    fn errors_remain_navigable_and_explicit_retry_recovers() -> io::Result<()> {
        let mut events = FakeEvents {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::from_millis(110), key(KeyCode::Tab)),
                (Duration::ZERO, key(KeyCode::Char('r'))),
                (Duration::ZERO, key(KeyCode::Char('q'))),
            ]),
        };
        let mut seen_error = false;
        let app = run_loop(
            &mut MockProvider::new(Scenario::RecoverableError),
            &mut events,
            Duration::from_millis(100),
            |app| {
                seen_error |= matches!(app.status, DataStatus::Error(_));
                Ok(())
            },
        )?;
        assert!(seen_error);
        assert_eq!(app.status, DataStatus::Ready);
        assert_eq!(app.section, Section::Processes);
        Ok(())
    }

    #[test]
    fn render_and_event_errors_propagate() {
        let mut events = FakeEvents {
            now: Duration::ZERO,
            queue: VecDeque::new(),
        };
        let mut provider = MockProvider::default();
        assert!(
            run_loop(&mut provider, &mut events, Duration::from_secs(1), |_| Err(
                io::Error::other("draw failed")
            ))
            .is_err()
        );
        assert!(
            run_loop(&mut provider, &mut events, Duration::from_secs(1), |_| Ok(
                ()
            ))
            .is_err()
        );
    }
}
