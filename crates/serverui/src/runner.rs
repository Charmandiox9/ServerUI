use crate::sampler::SampleSource;
use crossterm::event::{self, Event};
use serverui_core::{
    application::{Action, App, Effect},
    domain::ProviderMode,
    infrastructure::InfrastructureRequest,
};
use serverui_tui::input::action_for_app;
use std::{
    io,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};
const RESULT_POLL: Duration = Duration::from_millis(50);
trait EventSource {
    fn now(&self) -> Duration;
    fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>>;
}
pub struct TerminalEvents {
    start: Instant,
    input: Receiver<io::Result<Event>>,
}
impl TerminalEvents {
    pub fn new() -> io::Result<Self> {
        let (output, input) = mpsc::sync_channel(64);
        // Crossterm's Unix timed poll can busy-spin in its sub-millisecond tail.
        // One blocking reader preserves key order and SIGWINCH without polling.
        // It is process-owned; dropping the receiver disconnects queued sends,
        // and process exit releases a read already blocked in the OS.
        thread::Builder::new()
            .name("serverui-input".into())
            .spawn(move || {
                loop {
                    let event = event::read();
                    let failed = event.is_err();
                    if output.send(event).is_err() || failed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            start: Instant::now(),
            input,
        })
    }
}
impl EventSource for TerminalEvents {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }
    fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        match self.input.recv_timeout(timeout) {
            Ok(event) => event.map(Some),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("El lector de teclado termino"))
            }
        }
    }
}
pub fn run(
    source: &mut impl SampleSource,
    events: &mut TerminalEvents,
    interval: Duration,
    mode: ProviderMode,
    render: impl FnMut(&App) -> io::Result<()>,
) -> io::Result<App> {
    run_loop(source, events, interval, mode, render)
}
fn run_loop(
    source: &mut impl SampleSource,
    events: &mut impl EventSource,
    interval: Duration,
    mode: ProviderMode,
    mut render: impl FnMut(&App) -> io::Result<()>,
) -> io::Result<App> {
    if interval.is_zero() {
        return Err(io::Error::other("el intervalo debe ser positivo"));
    }
    let mut app = App::new(mode);
    apply(&mut app, Action::Refresh, source);
    let mut deadline = events.now().saturating_add(interval);
    let mut dirty = true;
    while !app.quit {
        let before = app.clone();
        let now = events.now();
        if now >= deadline {
            request_snapshot(&mut app, source);
            deadline = now.saturating_add(interval);
        }
        if let Some((request, reply)) = source.service_take() {
            app.services.receive(request, reply);
        }
        if let Some(progress) = source.package_progress() {
            app.packages.progress = Some(progress);
        }
        if let Some(reply) = source.package_take() {
            app.packages.receive(reply);
        }
        if let Some((request, reply)) = source.infrastructure_take() {
            app.infrastructure.receive(request, reply);
            if let Some(request) = app.infrastructure_on_entry() {
                request_infrastructure(&mut app, source, request);
            }
        }
        if let Some(reply) = source.take() {
            app.receive(reply);
        }
        dirty |= app != before;
        if dirty {
            render(&app)?;
            dirty = false;
        }
        let timeout = deadline.saturating_sub(events.now()).min(RESULT_POLL);
        match events.next(timeout)? {
            Some(Event::Key(key)) => {
                if let Some(action) = action_for_app(key, &app) {
                    dirty |= apply(&mut app, action, source);
                }
            }
            Some(Event::Resize(_, _)) => dirty = true,
            _ => {}
        }
    }
    Ok(app)
}
fn apply(app: &mut App, action: Action, source: &mut impl SampleSource) -> bool {
    let before = app.clone();
    match app.update(action) {
        Some(Effect::QuerySnapshot) => request_snapshot(app, source),
        Some(Effect::Service(request)) => match source.service_request(request.clone()) {
            Ok(true) => {
                app.services.busy = true;
                app.services.message = None;
            }
            Ok(false) => {}
            Err(error) => app.services.receive(request, Err(error)),
        },
        Some(Effect::Package(request)) => match source.package_request(request.clone()) {
            Ok(true) => app.packages.started(request),
            Ok(false) => {}
            Err(error) => app.packages.receive(Err(error)),
        },
        Some(Effect::Infrastructure(request)) => request_infrastructure(app, source, request),
        None => {}
    }
    *app != before
}
fn request_infrastructure(
    app: &mut App,
    source: &mut impl SampleSource,
    request: InfrastructureRequest,
) {
    match source.infrastructure_request(request.clone()) {
        Ok(true) => app.infrastructure.started(request),
        Ok(false) => {}
        Err(error) => app.infrastructure.receive(request, Err(error)),
    }
}
fn request_snapshot(app: &mut App, source: &mut impl SampleSource) {
    match source.request() {
        Ok(true) => app.refreshing = true,
        Ok(false) => {}
        Err(error) => app.receive(Err(error)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serverui_core::{
        application::{DataStatus, Section},
        domain::{ProviderError, Sample, SystemProvider},
    };
    use serverui_providers::{MockProvider, Scenario};
    use std::collections::VecDeque;
    struct Events {
        now: Duration,
        queue: VecDeque<(Duration, Option<Event>)>,
    }
    impl EventSource for Events {
        fn now(&self) -> Duration {
            self.now
        }
        fn next(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            let Some((delay, event)) = self.queue.front_mut() else {
                return Err(io::Error::other("exhausted"));
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
    struct Source {
        provider: MockProvider,
        reply: Option<Sample>,
        requests: usize,
    }
    impl Source {
        fn new(scenario: Scenario) -> Self {
            Self {
                provider: MockProvider::new(scenario),
                reply: None,
                requests: 0,
            }
        }
    }
    impl SampleSource for Source {
        fn request(&mut self) -> Result<bool, ProviderError> {
            if self.reply.is_some() {
                return Ok(false);
            }
            self.requests += 1;
            self.reply = Some(self.provider.snapshot());
            Ok(true)
        }
        fn take(&mut self) -> Option<Sample> {
            self.reply.take()
        }
    }
    fn key(code: KeyCode) -> Option<Event> {
        Some(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
    }
    #[test]
    fn service_mutations_are_once_only_and_metrics_continue() -> io::Result<()> {
        use serverui_core::services::*;
        struct ServicesSource {
            metrics: Source,
            provider: serverui_providers::MockServices,
            reply: Option<(ServiceRequest, ServiceResult)>,
            mutations: usize,
        }
        impl SampleSource for ServicesSource {
            fn request(&mut self) -> Result<bool, ProviderError> {
                self.metrics.request()
            }
            fn take(&mut self) -> Option<Sample> {
                self.metrics.take()
            }
            fn service_request(&mut self, request: ServiceRequest) -> Result<bool, ProviderError> {
                if self.reply.is_some() {
                    return Ok(false);
                }
                self.mutations += usize::from(matches!(request, ServiceRequest::Execute(..)));
                let result = self.provider.perform(&request);
                self.reply = Some((request, result));
                Ok(true)
            }
            fn service_take(&mut self) -> Option<(ServiceRequest, ServiceResult)> {
                self.reply.take()
            }
        }
        let mut source = ServicesSource {
            metrics: Source::new(Scenario::Normal),
            provider: serverui_providers::MockServices::new(Scenario::Normal),
            reply: None,
            mutations: 0,
        };
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::ZERO, key(KeyCode::Char('3'))),
                (Duration::ZERO, key(KeyCode::Char('d'))),
                (Duration::ZERO, key(KeyCode::Esc)),
                (Duration::ZERO, key(KeyCode::Char('y'))),
                (Duration::ZERO, key(KeyCode::Char('a'))),
                (Duration::ZERO, key(KeyCode::Char('y'))),
                (Duration::ZERO, key(KeyCode::Char('y'))),
                (Duration::from_millis(210), key(KeyCode::Char('q'))),
            ]),
        };
        let app = run_loop(
            &mut source,
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Demo,
            |_| Ok(()),
        )?;
        assert_eq!(source.mutations, 1);
        assert_eq!(app.services.history.len(), 1);
        assert_eq!(source.metrics.requests, 3);
        assert!(app.quit);
        Ok(())
    }
    #[test]
    fn tick_resize_navigation_and_idle_timeout() -> io::Result<()> {
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::ZERO, key(KeyCode::Tab)),
                (Duration::ZERO, Some(Event::Resize(40, 12))),
                (Duration::from_millis(110), key(KeyCode::Char('q'))),
            ]),
        };
        let mut frames = Vec::new();
        let mut source = Source::new(Scenario::Normal);
        let app = run_loop(
            &mut source,
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Demo,
            |a| {
                frames.push(a.clone());
                Ok(())
            },
        )?;
        assert!(app.quit);
        assert_eq!(app.section, Section::Processes);
        assert_eq!(source.requests, 2);
        assert_eq!(frames.len(), 4);
        assert_ne!(frames[0].snapshot, frames[3].snapshot);
        Ok(())
    }
    #[test]
    fn busy_keyboard_does_not_starve_refresh() -> io::Result<()> {
        let mut queue = VecDeque::new();
        for _ in 0..20 {
            queue.push_back((Duration::from_millis(25), key(KeyCode::Right)));
        }
        queue.push_back((Duration::ZERO, key(KeyCode::Char('q'))));
        let mut events = Events {
            now: Duration::ZERO,
            queue,
        };
        let mut source = Source::new(Scenario::Normal);
        let app = run_loop(
            &mut source,
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Demo,
            |_| Ok(()),
        )?;
        assert_eq!(app.snapshot.as_ref().map(|s| s.uptime_seconds), Some(86405));
        Ok(())
    }
    #[test]
    fn unchanged_empty_data_does_not_redraw() -> io::Result<()> {
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::from([(Duration::from_millis(350), key(KeyCode::Char('q')))]),
        };
        let mut frames = 0;
        run_loop(
            &mut Source::new(Scenario::Empty),
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Demo,
            |_| {
                frames += 1;
                Ok(())
            },
        )?;
        assert_eq!(frames, 1);
        Ok(())
    }
    #[test]
    fn slow_source_never_blocks_navigation_or_exit() -> io::Result<()> {
        struct Slow {
            requests: usize,
        }
        impl SampleSource for Slow {
            fn request(&mut self) -> Result<bool, ProviderError> {
                self.requests += 1;
                Ok(self.requests == 1)
            }
            fn take(&mut self) -> Option<Sample> {
                None
            }
        }
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::from_millis(300), key(KeyCode::Tab)),
                (Duration::ZERO, Some(Event::Resize(40, 12))),
                (Duration::ZERO, key(KeyCode::Char('q'))),
            ]),
        };
        let mut source = Slow { requests: 0 };
        let mut rendered = 0;
        let app = run_loop(
            &mut source,
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Linux,
            |_| {
                rendered += 1;
                Ok(())
            },
        )?;
        assert!(app.quit && app.refreshing);
        assert_eq!(app.section, Section::Processes);
        assert_eq!(app.status, DataStatus::Loading);
        assert_eq!(rendered, 3);
        Ok(())
    }
    #[test]
    fn errors_remain_navigable_and_retry_recovers() -> io::Result<()> {
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::from([
                (Duration::from_millis(110), key(KeyCode::Tab)),
                (Duration::ZERO, key(KeyCode::Char('r'))),
                (Duration::ZERO, key(KeyCode::Char('q'))),
            ]),
        };
        let mut seen = false;
        let app = run_loop(
            &mut Source::new(Scenario::RecoverableError),
            &mut events,
            Duration::from_millis(100),
            ProviderMode::Demo,
            |a| {
                seen |= matches!(a.status, DataStatus::Error(_));
                Ok(())
            },
        )?;
        assert!(seen);
        assert_eq!(app.status, DataStatus::Ready);
        Ok(())
    }
    #[test]
    fn errors_from_render_or_events_propagate() {
        let mut events = Events {
            now: Duration::ZERO,
            queue: VecDeque::new(),
        };
        let mut source = Source::new(Scenario::Normal);
        assert!(
            run_loop(
                &mut source,
                &mut events,
                Duration::from_secs(1),
                ProviderMode::Demo,
                |_| Err(io::Error::other("draw"))
            )
            .is_err()
        );
        assert!(
            run_loop(
                &mut source,
                &mut events,
                Duration::from_secs(1),
                ProviderMode::Demo,
                |_| Ok(())
            )
            .is_err()
        );
    }
}
