use thiserror::Error;

/// All sizes are bytes; CPU is hundredths of one percent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub cpu_usage: Option<u32>,
    pub cpu_count: u32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub uptime_seconds: u64,
    pub disks: Vec<DiskInfo>,
    pub disk_warning: Option<String>,
    pub processes: Vec<ProcessInfo>,
    pub process_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessId {
    pub pid: u32,
    /// Unix start time in seconds, paired with PID to detect reuse.
    pub start_time: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub id: ProcessId,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub status: String,
    /// 10000 is 100% of ONE logical CPU. None means no valid baseline.
    pub cpu_usage: Option<u32>,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub run_time_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskInfo {
    pub mount: String,
    pub space: Option<DiskSpace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpace {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProviderMode {
    #[default]
    Demo,
    Linux,
}

impl ProviderMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Demo => "DEMO",
            Self::Linux => "LINUX / SOLO LECTURA",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("{0}")]
    Unavailable(String),
}

pub type Sample = Result<Option<Snapshot>, ProviderError>;

/// The composition layer must call real adapters outside the UI thread.
pub trait SystemProvider {
    fn snapshot(&mut self) -> Sample;
}
