use crate::action_worker::ActionWorker;
use serverui_core::domain::{ProviderError, Sample, SystemProvider};
use serverui_core::{
    packages::{PackageData, PackageProgress, PackageProvider, PackageRequest, PackageResult},
    services::{ServiceData, ServiceProvider, ServiceRequest, ServiceResult},
};
use std::{
    io,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
};

/// Nonblocking application boundary. At most one request can be in flight.
pub trait SampleSource {
    fn request(&mut self) -> Result<bool, ProviderError>;
    fn take(&mut self) -> Option<Sample>;
    fn service_request(&mut self, _: ServiceRequest) -> Result<bool, ProviderError> {
        Err(ProviderError::Unavailable(
            "Servicios no disponibles".into(),
        ))
    }
    fn service_take(&mut self) -> Option<(ServiceRequest, ServiceResult)> {
        None
    }
    fn package_request(&mut self, _: PackageRequest) -> Result<bool, ProviderError> {
        Err(ProviderError::Unavailable("APT no disponible".into()))
    }
    fn package_take(&mut self) -> Option<PackageResult> {
        None
    }
    fn package_progress(&mut self) -> Option<PackageProgress> {
        None
    }
}

pub type ServiceFactory = Box<dyn FnOnce() -> Box<dyn ServiceProvider> + Send>;
pub type PackageFactory = Box<dyn FnOnce() -> Box<dyn PackageProvider> + Send>;

pub struct Sampler {
    pub service_factory: Option<ServiceFactory>,
    services: Option<ActionWorker<ServiceRequest, ServiceData>>,
    pub package_factory: Option<PackageFactory>,
    packages: Option<ActionWorker<PackageRequest, PackageData>>,
    progress: Option<Receiver<PackageProgress>>,
    requests: SyncSender<()>,
    results: Receiver<Sample>,
    cancelled: Arc<AtomicBool>,
    pending: bool,
    disconnected: bool,
}

fn stopped() -> ProviderError {
    ProviderError::Unavailable("El recolector termino; reinicia ServerUI".into())
}

impl Sampler {
    pub fn spawn(
        factory: impl FnOnce() -> Result<Box<dyn SystemProvider + Send>, ProviderError> + Send + 'static,
    ) -> io::Result<Self> {
        let (requests, input) = mpsc::sync_channel(1);
        let (output, results) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancelled);
        thread::Builder::new()
            .name("serverui-sampler".into())
            .spawn(move || {
                let mut factory = Some(factory);
                let mut provider = None;
                while input.recv().is_ok() && !stop.load(Ordering::Acquire) {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        if let Some(factory) = factory.take() {
                            provider = Some(factory()?);
                        }
                        match provider.as_mut() {
                            Some(provider) => provider.snapshot(),
                            None => Err(stopped()),
                        }
                    }));
                    let (sample, fatal) = match result {
                        Ok(sample) => {
                            let fatal = provider.is_none();
                            (sample, fatal)
                        }
                        Err(_) => (
                            Err(ProviderError::Unavailable(
                                "Fallo interno del recolector; reinicia ServerUI".into(),
                            )),
                            true,
                        ),
                    };
                    if stop.load(Ordering::Acquire) || output.try_send(sample).is_err() || fatal {
                        break;
                    }
                }
            })?;
        Ok(Self {
            services: None,
            service_factory: None,
            package_factory: None,
            packages: None,
            progress: None,
            requests,
            results,
            cancelled,
            pending: false,
            disconnected: false,
        })
    }
}
impl SampleSource for Sampler {
    fn service_request(&mut self, request: ServiceRequest) -> Result<bool, ProviderError> {
        if self.services.is_none()
            && let Some(factory) = self.service_factory.take()
        {
            self.services = Some(
                ActionWorker::spawn("Worker de servicios", move || {
                    let mut provider = factory();
                    Box::new(move |request| provider.perform(request))
                })
                .map_err(|_| {
                    ProviderError::Unavailable("No se pudo iniciar worker de servicios".into())
                })?,
            );
        }
        self.services.as_mut().ok_or_else(stopped)?.request(request)
    }
    fn service_take(&mut self) -> Option<(ServiceRequest, ServiceResult)> {
        self.services.as_mut()?.take()
    }
    fn package_request(&mut self, request: PackageRequest) -> Result<bool, ProviderError> {
        if self.packages.is_none()
            && let Some(factory) = self.package_factory.take()
        {
            let (output, progress) = mpsc::sync_channel(1);
            self.packages = Some(
                ActionWorker::spawn("Worker de APT", move || {
                    let mut provider = factory();
                    Box::new(move |request| {
                        provider.perform(request, &mut |state| {
                            let _ = output.try_send(state);
                        })
                    })
                })
                .map_err(|_| {
                    ProviderError::Unavailable("No se pudo iniciar worker de APT".into())
                })?,
            );
            self.progress = Some(progress);
        }
        self.packages
            .as_mut()
            .ok_or_else(|| ProviderError::Unavailable("APT no disponible".into()))?
            .request(request)
    }
    fn package_take(&mut self) -> Option<PackageResult> {
        self.packages.as_mut()?.take().map(|(_, result)| result)
    }
    fn package_progress(&mut self) -> Option<PackageProgress> {
        self.progress.as_ref()?.try_recv().ok()
    }
    fn request(&mut self) -> Result<bool, ProviderError> {
        if self.pending {
            return Ok(false);
        }
        match self.requests.try_send(()) {
            Ok(()) => {
                self.pending = true;
                Ok(true)
            }
            Err(TrySendError::Full(_)) => Ok(false),
            Err(TrySendError::Disconnected(_)) => Err(stopped()),
        }
    }
    fn take(&mut self) -> Option<Sample> {
        match self.results.try_recv() {
            Ok(sample) => {
                self.pending = false;
                Some(sample)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) if !self.disconnected => {
                self.disconnected = true;
                let pending = self.pending;
                self.pending = false;
                pending.then(|| Err(stopped()))
            }
            Err(TryRecvError::Disconnected) => None,
        }
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    #[test]
    fn service_thread_and_provider_are_created_only_on_first_service_request() -> io::Result<()> {
        use serverui_core::services::ServiceProvider;
        let (created, receiver) = mpsc::channel();
        let mut sampler =
            Sampler::spawn(|| Ok(Box::new(serverui_providers::MockProvider::default())))?;
        sampler.service_factory = Some(Box::new(move || {
            let _ = created.send(());
            Box::new(serverui_providers::MockServices::new(
                serverui_providers::Scenario::Normal,
            )) as Box<dyn ServiceProvider>
        }));
        assert!(sampler.services.is_none());
        assert_eq!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty));
        sampler
            .service_request(ServiceRequest::List)
            .map_err(io::Error::other)?;
        receiver.recv().map_err(io::Error::other)?;
        assert!(sampler.services.is_some());
        Ok(())
    }
    struct Blocked {
        entered: mpsc::Sender<()>,
        release: Receiver<()>,
    }
    impl SystemProvider for Blocked {
        fn snapshot(&mut self) -> Sample {
            let _ = self.entered.send(());
            let _ = self.release.recv();
            Ok(None)
        }
    }
    #[test]
    fn bounded_requests_and_drop_do_not_wait_for_slow_provider() -> io::Result<()> {
        let (entered, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let mut sampler = Sampler::spawn(move || {
            Ok(Box::new(Blocked {
                entered,
                release: wait,
            }))
        })?;
        assert_eq!(sampler.request(), Ok(true));
        ready.recv().map_err(io::Error::other)?;
        for _ in 0..1000 {
            assert_eq!(sampler.request(), Ok(false));
            assert_eq!(sampler.take(), None);
        }
        // If Drop joined the worker, this test would deadlock before release.
        drop(sampler);
        release.send(()).map_err(io::Error::other)?;
        Ok(())
    }
    #[test]
    fn factory_is_lazy_and_cancelled_before_sampling() -> io::Result<()> {
        let (created, receiver) = mpsc::channel();
        let sampler = Sampler::spawn(move || {
            let _ = created.send(());
            Err(stopped())
        })?;
        drop(sampler);
        assert!(receiver.recv().is_err());
        Ok(())
    }
    #[test]
    fn factory_error_is_a_structured_result() -> io::Result<()> {
        let mut sampler = Sampler::spawn(|| Err(ProviderError::Unavailable("fixture".into())))?;
        assert_eq!(sampler.request(), Ok(true));
        // Wait on the result channel only in this unit test, never in the UI.
        let reply = sampler.results.recv().map_err(io::Error::other)?;
        assert_eq!(reply, Err(ProviderError::Unavailable("fixture".into())));
        Ok(())
    }

    #[test]
    fn background_panic_becomes_a_structured_error() -> io::Result<()> {
        let mut sampler = Sampler::spawn(|| panic!("fixture sampler panic"))?;
        assert_eq!(sampler.request(), Ok(true));
        let reply = sampler.results.recv().map_err(io::Error::other)?;
        assert!(
            matches!(reply, Err(ProviderError::Unavailable(message)) if message.contains("Fallo interno"))
        );
        Ok(())
    }
}
