use thiserror::Error;

/// A single sample. Sizes are bytes, CPU is an integer percentage.
/// No field is tied to an operating system API or terminal library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub cpu_percent: u8,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("{0}")]
    Unavailable(String),
}

/// Stage 0 implements only an immediate, bounded mock query.
/// Real I/O must be dispatched outside the UI thread in a later stage.
pub trait SystemProvider {
    fn snapshot(&mut self) -> Result<Option<Snapshot>, ProviderError>;
}
