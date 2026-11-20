use serverui_core::domain::ProviderError;
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

pub struct ActionWorker<Request, Reply> {
    requests: SyncSender<Request>,
    results: Receiver<(Request, Result<Reply, ProviderError>)>,
    pending: Option<Request>,
    cancelled: Arc<AtomicBool>,
    name: &'static str,
}
fn stopped(name: &str) -> ProviderError {
    ProviderError::Unavailable(format!("{name} termino; reinicia ServerUI"))
}
impl<Request: Clone + Send + 'static, Reply: Send + 'static> ActionWorker<Request, Reply> {
    pub fn spawn(
        name: &'static str,
        factory: impl FnOnce() -> Box<dyn FnMut(&Request) -> Result<Reply, ProviderError>>
        + Send
        + 'static,
    ) -> io::Result<Self> {
        let (requests, input) = mpsc::sync_channel::<Request>(1);
        let (output, results) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancelled);
        thread::Builder::new().name(name.into()).spawn(move || {
            let mut factory = Some(factory);
            let mut provider = None;
            while let Ok(request) = input.recv() {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let result = catch_unwind(AssertUnwindSafe(|| {
                    if let Some(factory) = factory.take() {
                        provider = Some(factory());
                    }
                    provider.as_mut().ok_or_else(|| stopped(name))?(&request)
                }));
                let fatal = result.is_err();
                let reply = result.unwrap_or_else(|_| Err(stopped(name)));
                if stop.load(Ordering::Acquire)
                    || output.try_send((request, reply)).is_err()
                    || fatal
                {
                    break;
                }
            }
        })?;
        Ok(Self {
            requests,
            results,
            pending: None,
            cancelled,
            name,
        })
    }
    pub fn request(&mut self, request: Request) -> Result<bool, ProviderError> {
        if self.pending.is_some() {
            return Ok(false);
        }
        match self.requests.try_send(request.clone()) {
            Ok(()) => {
                self.pending = Some(request);
                Ok(true)
            }
            Err(TrySendError::Full(_)) => Ok(false),
            Err(TrySendError::Disconnected(_)) => Err(stopped(self.name)),
        }
    }
    pub fn take(&mut self) -> Option<(Request, Result<Reply, ProviderError>)> {
        match self.results.try_recv() {
            Ok(result) => {
                self.pending = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => self
                .pending
                .take()
                .map(|request| (request, Err(stopped(self.name)))),
        }
    }
}
impl<Request, Reply> Drop for ActionWorker<Request, Reply> {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serverui_core::services::*;
    #[test]
    fn service_worker_is_lazy_bounded_and_drop_never_joins() -> io::Result<()> {
        struct Blocked {
            entered: mpsc::Sender<()>,
            release: Receiver<()>,
        }
        impl ServiceProvider for Blocked {
            fn perform(&mut self, _: &ServiceRequest) -> ServiceResult {
                let _ = self.entered.send(());
                let _ = self.release.recv();
                Ok(ServiceData::Executed)
            }
        }
        let (entered, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let mut worker = ActionWorker::spawn("serverui-services", move || {
            let mut provider = Blocked {
                entered,
                release: wait,
            };
            Box::new(move |request| provider.perform(request))
        })?;
        assert_eq!(ready.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(worker.request(ServiceRequest::List), Ok(true));
        ready.recv().map_err(io::Error::other)?;
        for _ in 0..1000 {
            assert_eq!(worker.request(ServiceRequest::List), Ok(false));
            assert_eq!(worker.take(), None);
        }
        drop(worker);
        release.send(()).map_err(io::Error::other)?;
        Ok(())
    }
    #[test]
    fn panic_is_structured_and_does_not_escape_worker() -> io::Result<()> {
        let mut worker =
            ActionWorker::<ServiceRequest, ServiceData>::spawn("serverui-services", || {
                panic!("fixture")
            })?;
        worker
            .request(ServiceRequest::List)
            .map_err(io::Error::other)?;
        let (_, result) = worker.results.recv().map_err(io::Error::other)?;
        assert!(result.is_err());
        Ok(())
    }
}
