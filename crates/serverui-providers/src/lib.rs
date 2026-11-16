//! Metrics and service adapters. Demo never initializes real Linux providers.
mod mock;
mod services;
pub use mock::{MockProvider, Scenario};
#[cfg(target_os = "linux")]
pub use services::LinuxServices;
pub use services::MockServices;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod linux_processes;
#[cfg(target_os = "linux")]
pub use linux::LinuxProvider;
