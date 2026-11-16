//! Bounded process metrics from stat; no arguments, environment or per-PID FDs.
use serverui_core::domain::{ProcessId, ProcessInfo};
use std::{
    collections::{BinaryHeap, HashMap},
    fs::{self, File},
    io::{self, Read},
    path::Path,
    time::Duration,
};

pub(crate) const MAX_PROCESSES: usize = 10_000;
const STAT_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Parameters {
    ticks: u64,
    page: u64,
}
impl Parameters {
    pub(crate) fn load() -> io::Result<Self> {
        let bytes = fs::read("/proc/self/auxv")?;
        Self::parse(&bytes).ok_or_else(|| io::Error::other("AT_CLKTCK/AT_PAGESZ no disponibles"))
    }
    fn parse(bytes: &[u8]) -> Option<Self> {
        const WORD: usize = size_of::<usize>();
        let mut ticks = None;
        let mut page = None;
        if !bytes.len().is_multiple_of(WORD * 2) {
            return None;
        }
        for entry in bytes.as_chunks::<{ WORD * 2 }>().0 {
            let kind = usize::from_ne_bytes(entry[..WORD].try_into().ok()?);
            let value = usize::from_ne_bytes(entry[WORD..].try_into().ok()?) as u64;
            match kind {
                0 => break,
                6 => page = Some(value),
                17 => ticks = Some(value),
                _ => {}
            }
        }
        let (ticks, page) = (ticks?, page?);
        (ticks > 0 && ticks <= 1_000_000 && page.is_power_of_two() && page <= (1 << 30))
            .then_some(Self { ticks, page })
    }
}

#[derive(Debug, PartialEq)]
struct Stat {
    pid: u32,
    name: String,
    state: u8,
    parent: u32,
    ticks: u64,
    start: u64,
    virtual_bytes: u64,
    rss_pages: u64,
}
fn number(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0_u64, |value, c| {
        c.is_ascii_digit().then_some(())?;
        value.checked_mul(10)?.checked_add(u64::from(c - b'0'))
    })
}
fn parse_stat(bytes: &[u8], pid: u32) -> Option<Stat> {
    if bytes.len() >= STAT_BYTES {
        return None;
    }
    let open = bytes.iter().position(|b| *b == b'(')?;
    let close = bytes.iter().rposition(|b| *b == b')')?;
    if close <= open {
        return None;
    }
    let prefix = std::str::from_utf8(bytes.get(..open)?).ok()?.trim();
    if prefix.parse::<u32>().ok()? != pid {
        return None;
    }
    let mut fields = bytes
        .get(close + 1..)?
        .split(u8::is_ascii_whitespace)
        .filter(|s| !s.is_empty());
    let state = fields.next()?;
    if state.len() != 1 || !state[0].is_ascii_alphabetic() {
        return None;
    }
    let parent = u32::try_from(number(fields.next()?)?).ok()?;
    let user = number(fields.nth(9)?)?; // field 14, after parent (field 4)
    let system = number(fields.next()?)?;
    let start = number(fields.nth(6)?)?; // field 22
    let virtual_bytes = number(fields.next()?)?;
    let rss = fields.next()?;
    let rss_pages = if rss.starts_with(b"-") {
        number(&rss[1..])?;
        0
    } else {
        number(rss)?
    };
    Some(Stat {
        pid,
        name: String::from_utf8_lossy(bytes.get(open + 1..close)?)
            .chars()
            .take(128)
            .collect(),
        state: state[0],
        parent,
        ticks: user.checked_add(system)?,
        start,
        virtual_bytes,
        rss_pages,
    })
}

fn cpu(ticks: u64, old_ticks: u64, elapsed: Duration, frequency: u64) -> Option<u32> {
    if elapsed.is_zero() || frequency == 0 {
        return None;
    }
    let delta = u128::from(ticks.checked_sub(old_ticks)?);
    let value = delta * 1_000_000_000 * 10_000 / (u128::from(frequency) * elapsed.as_nanos());
    Some(value.min(u128::from(u32::MAX)) as u32)
}
fn state_label(state: u8) -> &'static str {
    match state {
        b'R' => "Running",
        b'S' => "Sleeping",
        b'D' => "UninterruptibleDiskSleep",
        b'Z' => "Zombie",
        b'T' => "Stopped",
        b't' => "Tracing",
        b'X' | b'x' => "Dead",
        b'I' => "Idle",
        b'K' => "Wakekill",
        b'W' => "Waking",
        b'P' => "Parked",
        _ => "Unknown",
    }
}
pub(crate) struct Processes {
    parameters: Parameters,
    boot: u64,
    baseline: HashMap<(u32, u64), u64>,
    last: Option<Duration>,
}
impl Processes {
    pub(crate) fn new(boot: u64) -> io::Result<Self> {
        Ok(Self {
            parameters: Parameters::load()?,
            boot,
            baseline: HashMap::new(),
            last: None,
        })
    }
    pub(crate) fn collect(
        &mut self,
        now: Duration,
        uptime: u64,
    ) -> io::Result<(Vec<ProcessInfo>, usize)> {
        self.collect_from(Path::new("/proc"), now, uptime)
    }
    fn collect_from(
        &mut self,
        root: &Path,
        now: Duration,
        uptime: u64,
    ) -> io::Result<(Vec<ProcessInfo>, usize)> {
        // Retain lowest PIDs without an unbounded allocation on large hosts.
        let mut pids = BinaryHeap::new();
        let mut count = 0_usize;
        for entry in fs::read_dir(root)? {
            let Ok(entry) = entry else {
                continue;
            };
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
                .filter(|pid| *pid > 0)
            else {
                continue;
            };
            count = count.saturating_add(1);
            if pids.len() < MAX_PROCESSES {
                pids.push(pid);
            } else if pids.peek().is_some_and(|last| pid < *last) {
                pids.pop();
                pids.push(pid);
            }
        }
        let elapsed = self.last.map(|last| now.saturating_sub(last));
        let mut next = HashMap::with_capacity(pids.len());
        let mut output = Vec::with_capacity(pids.len());
        let mut buffer = [0_u8; STAT_BYTES];
        for pid in pids.into_sorted_vec() {
            let path = root.join(pid.to_string()).join("stat");
            let Ok(mut file) = File::open(path) else {
                continue;
            };
            // proc stat fits in one seq_file read. A partial/oversized record is
            // skipped; never follow it with reads of arguments or environment.
            let Ok(size) = file.read(&mut buffer) else {
                continue;
            };
            let Some(stat) = parse_stat(&buffer[..size], pid) else {
                continue;
            };
            let key = (pid, stat.start);
            let cpu_usage = elapsed.and_then(|elapsed| {
                cpu(
                    stat.ticks,
                    *self.baseline.get(&key)?,
                    elapsed,
                    self.parameters.ticks,
                )
            });
            next.insert(key, stat.ticks);
            let start_seconds = stat.start / self.parameters.ticks;
            output.push(ProcessInfo {
                id: ProcessId {
                    pid,
                    start_time: self.boot.saturating_add(start_seconds),
                },
                parent_pid: Some(stat.parent),
                name: stat.name,
                status: state_label(stat.state).into(),
                cpu_usage,
                memory_bytes: stat.rss_pages.saturating_mul(self.parameters.page),
                virtual_memory_bytes: stat.virtual_bytes,
                run_time_seconds: uptime.saturating_sub(start_seconds),
            });
        }
        self.baseline = next;
        self.last = Some(now);
        if count <= MAX_PROCESSES {
            count = output.len();
        }
        Ok((output, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_invalidate_baselines_for_pid_reuse_and_disappearance() -> io::Result<()> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "serverui-proc-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root)?;
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        fs::create_dir(root.join("42"))?;
        let stat = root.join("42/stat");
        fs::write(&stat, record(b"fixture", 987, 75))?;
        let mut sampler = Processes {
            parameters: Parameters {
                ticks: 100,
                page: 4096,
            },
            boot: 1000,
            baseline: HashMap::new(),
            last: None,
        };
        let (rows, count) = sampler.collect_from(&root, Duration::ZERO, 50)?;
        assert_eq!(count, 1);
        assert_eq!(rows[0].cpu_usage, None);
        assert_eq!(rows[0].memory_bytes, 3 * 4096);
        assert_eq!(rows[0].id.start_time, 1009);
        fs::write(&stat, record(b"fixture", 987, 175))?;
        let (rows, _) = sampler.collect_from(&root, Duration::from_secs(1), 51)?;
        assert_eq!(rows[0].cpu_usage, Some(10000));
        // Reuse within the same Unix second still invalidates internal CPU.
        fs::write(&stat, record(b"replacement", 988, 1))?;
        let (rows, _) = sampler.collect_from(&root, Duration::from_secs(2), 52)?;
        assert_eq!(rows[0].cpu_usage, None);
        fs::remove_file(&stat)?;
        assert!(
            sampler
                .collect_from(&root, Duration::from_secs(3), 53)?
                .0
                .is_empty()
        );
        assert!(sampler.baseline.is_empty());
        fs::write(&stat, b"42 (truncated) S")?;
        assert!(
            sampler
                .collect_from(&root, Duration::from_secs(4), 54)?
                .0
                .is_empty()
        );
        Ok(())
    }
    fn record(name: &[u8], start: u64, user: u64) -> Vec<u8> {
        let mut bytes = b"42 (".to_vec();
        bytes.extend(name);
        bytes.extend(
            format!(") S 1 0 0 0 0 0 0 0 0 0 {user} 25 0 0 0 0 0 0 {start} 8192 3 0 0\n")
                .as_bytes(),
        );
        bytes
    }
    #[test]
    fn stat_parser_handles_comm_boundaries_non_utf8_and_field_units() {
        let stat =
            parse_stat(&record(b"a ) (b\n\xff", 987, 75), 42).unwrap_or_else(|| panic!("fixture"));
        assert_eq!(stat.parent, 1);
        assert_eq!(stat.ticks, 100);
        assert_eq!(stat.start, 987);
        assert_eq!(stat.virtual_bytes, 8192);
        assert_eq!(stat.rss_pages, 3);
        assert!(stat.name.starts_with("a ) (b\n"));
        assert!(parse_stat(&record(b"x", 1, 1), 43).is_none());
        assert!(parse_stat(b"42 (x) S 1", 42).is_none());
        assert!(parse_stat(&vec![b'x'; STAT_BYTES], 42).is_none());
        assert!(number(b"18446744073709551616").is_none());
    }
    #[test]
    fn cpu_is_per_core_and_rejects_invalid_baselines() {
        assert_eq!(cpu(250, 0, Duration::from_secs(1), 100), Some(25000));
        assert_eq!(cpu(0, 0, Duration::from_secs(1), 100), Some(0));
        assert_eq!(cpu(10, 20, Duration::from_secs(1), 100), None);
        assert_eq!(cpu(10, 0, Duration::ZERO, 100), None);
    }
    #[test]
    fn auxv_uses_native_word_size_and_validates_parameters() {
        fn entry(key: usize, value: usize, data: &mut Vec<u8>) {
            data.extend(key.to_ne_bytes());
            data.extend(value.to_ne_bytes());
        }
        let mut data = Vec::new();
        entry(6, 4096, &mut data);
        entry(17, 100, &mut data);
        entry(0, 0, &mut data);
        let parameters = Parameters::parse(&data).unwrap_or_else(|| panic!("auxv"));
        assert_eq!(parameters.page, 4096);
        assert_eq!(parameters.ticks, 100);
        assert!(Parameters::parse(&data[..data.len() - 1]).is_none());
        assert!(Parameters::parse(&[]).is_none());
    }
}
