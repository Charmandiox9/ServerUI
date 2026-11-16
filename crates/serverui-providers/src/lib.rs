//! Read-only adapters. Demo never initializes the Linux provider.
mod mock;
pub use mock::{MockProvider, Scenario};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxProvider;
