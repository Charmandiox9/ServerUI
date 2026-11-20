//! Metrics and service adapters. Demo never initializes real Linux providers.
mod infrastructure;
mod mock;
mod packages;
mod services;
#[cfg(target_os = "linux")]
pub use infrastructure::LinuxInfrastructure;
pub use infrastructure::MockInfrastructure;
pub use mock::{MockProvider, Scenario};
#[cfg(target_os = "linux")]
pub use packages::LinuxPackages;
pub use packages::MockPackages;
#[cfg(target_os = "linux")]
pub use services::LinuxServices;
pub use services::MockServices;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod linux_processes;
#[cfg(target_os = "linux")]
pub use linux::LinuxProvider;
