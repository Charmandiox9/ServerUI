use super::*;
use crate::services::native::bounded_command;
use std::{
    collections::VecDeque,
    io::{self, Read},
    process::{Command, Stdio},
    sync::mpsc::{self, SyncSender},
    thread,
    time::{Duration, Instant},
};

const APT_GET: &str = "/usr/bin/apt-get";
const QUERY_TIMEOUT: Duration = Duration::from_secs(8);
const LINE_LIMIT: usize = 512;
const RECOMMENDED: &[&str] = &[
    "cloudflared",
    "curl",
    "git",
    "htop",
    "ncdu",
    "nginx",
    "nodejs",
    "ripgrep",
    "tmux",
    "unzip",
];

pub struct LinuxPackages;

impl PackageProvider for LinuxPackages {
    fn perform(
        &mut self,
        request: &PackageRequest,
        progress: &mut dyn FnMut(PackageProgress),
    ) -> PackageResult {
        match request {
            PackageRequest::Recommendations => {
                search(&format!("^({})$", RECOMMENDED.join("|")), RECOMMENDED)
            }
            PackageRequest::Search(query) => {
                let literal = query.as_str().replace('.', "[.]").replace('+', "[+]");
                search(&literal, &[])
            }
            PackageRequest::Preview(name, operation) => {
                preview(name, *operation).map(PackageData::Preview)
            }
            PackageRequest::Apply(plan) => {
                if plan.changes.is_empty() || preview(&plan.name, plan.operation)? != *plan {
                    return Err(unavailable(
                        "El plan APT cambio; vuelve a simular y confirmar",
                    ));
                }
                progress(PackageProgress {
                    percent: None,
                    line: "Esperando autorizacion del sistema para ejecutar APT".into(),
                });
                let mut process = Command::new("/usr/bin/pkexec");
                process
                    .args(["--disable-internal-agent", APT_GET])
                    .args(apt_args(&plan.name, plan.operation, false));
                execute(process, progress, undo_install(plan)).map(PackageData::Applied)
            }
        }
    }
}

fn undo_install(plan: &PackagePlan) -> Option<PackageName> {
    if plan.operation != PackageOperation::Install {
        return None;
    }
    let mut installs = plan
        .changes
        .iter()
        .filter_map(|line| line.strip_prefix("Inst "));
    let mut fields = installs.next()?.split_whitespace();
    let name = fields.next()?.split(':').next()?;
    let fresh = fields.next()?.starts_with('(');
    (installs.next().is_none() && name == plan.name.as_str() && fresh).then(|| plan.name.clone())
}

fn apt_args(name: &PackageName, operation: PackageOperation, simulate: bool) -> Vec<String> {
    let mut args: Vec<String> = [
        "-o",
        "APT::Color=0",
        "-o",
        "APT::Get::AutomaticRemove=false",
        "-o",
        "APT::Get::Purge=false",
        "-o",
        "APT::Get::AllowUnauthenticated=false",
        "-o",
        "DPkg::Lock::Timeout=0",
    ]
    .map(String::from)
    .to_vec();
    args.push(
        if simulate {
            "--simulate"
        } else {
            "--assume-yes"
        }
        .into(),
    );
    if !simulate {
        args.extend(
            [
                "-o",
                "APT::Status-Fd=1",
                "-o",
                "Dpkg::Use-Pty=0",
                "-o",
                "Dpkg::Options::=--force-confold",
                "-o",
                "Acquire::Retries=0",
                "-o",
                "Acquire::http::Timeout=30",
                "-o",
                "Acquire::https::Timeout=30",
            ]
            .map(String::from),
        );
    }
    if operation != PackageOperation::Remove {
        args.push("--no-remove".into());
    }
    if operation == PackageOperation::Upgrade {
        args.push("--only-upgrade".into());
    }
    args.push(
        if operation == PackageOperation::Remove {
            "remove"
        } else {
            "install"
        }
        .into(),
    );
    // The :native suffix also prevents a trailing '+' or '-' from becoming an action.
    args.extend(["--".into(), format!("{}:native", name.as_str())]);
    args
}

fn search(pattern: &str, exact_names: &[&str]) -> PackageResult {
    let mut process = Command::new("/usr/bin/apt-cache");
    process
        .env_remove("APT_CONFIG")
        .args(["--names-only", "search", "--", pattern]);
    let output = bounded_command(
        process,
        QUERY_TIMEOUT,
        "No se pudo consultar la cache local de APT",
    )?;
    let text = String::from_utf8_lossy(&output);
    let matches = text.lines().filter_map(|line| {
        let (name, description) = line.split_once(" - ")?;
        if !exact_names.is_empty() && !exact_names.contains(&name) {
            return None;
        }
        Some(PackageInfo {
            name: PackageName::parse(name)?,
            description: description.chars().take(256).collect(),
        })
    });
    let mut rows: Vec<_> = matches.take(MAX_RESULTS + 1).collect();
    let limited = rows.len() > MAX_RESULTS;
    rows.truncate(MAX_RESULTS);
    rows.sort_unstable_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    Ok(PackageData::Search(rows, limited))
}

fn preview(name: &PackageName, operation: PackageOperation) -> Result<PackagePlan, ProviderError> {
    let mut process = Command::new(APT_GET);
    process
        .env_remove("APT_CONFIG")
        .args(apt_args(name, operation, true));
    let output = bounded_command(
        process,
        QUERY_TIMEOUT,
        "APT no pudo simular. Comprueba el paquete, la cache y el estado de las dependencias",
    )?;
    parse_plan(name, operation, &String::from_utf8_lossy(&output))
}

fn parse_plan(
    name: &PackageName,
    operation: PackageOperation,
    output: &str,
) -> Result<PackagePlan, ProviderError> {
    let mut changes = Vec::new();
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("Remv")
                if operation == PackageOperation::Remove
                    && fields
                        .next()
                        .is_some_and(|p| p.split(':').next() == Some(name.as_str())) => {}
            Some("Inst" | "Conf") if operation != PackageOperation::Remove => {}
            Some("Remv" | "Purg" | "Inst" | "Conf") => {
                return Err(unavailable(
                    "La simulacion incluye cambios incompatibles con la operacion",
                ));
            }
            _ => continue,
        }
        if changes.len() == MAX_PLAN_LINES || line.len() > 256 {
            return Err(unavailable("El plan APT supera el limite de revision"));
        }
        changes.push(line.to_string());
    }
    Ok(PackagePlan {
        name: name.clone(),
        operation,
        changes: Arc::new(changes),
    })
}

fn drain(mut input: impl Read, output: SyncSender<String>) -> io::Result<()> {
    let mut chunk = [0; 4096];
    let mut line = Vec::new();
    loop {
        let count = input.read(&mut chunk)?;
        if count == 0 {
            if !line.is_empty() {
                let _ = output.send(String::from_utf8_lossy(&line).into_owned());
            }
            return Ok(());
        }
        for &byte in &chunk[..count] {
            if matches!(byte, b'\n' | b'\r') {
                if !line.is_empty()
                    && output
                        .send(String::from_utf8_lossy(&line).into_owned())
                        .is_err()
                {
                    return Ok(());
                }
                line.clear();
            } else if line.len() < LINE_LIMIT {
                line.push(byte);
            }
        }
    }
}

fn progress_line(line: &str) -> PackageProgress {
    let mut fields = line.splitn(4, ':');
    if matches!(fields.next(), Some("dlstatus" | "pmstatus")) {
        let _ = fields.next();
        let percent = fields
            .next()
            .and_then(|f| f.parse::<f64>().ok())
            .filter(|p| (0.0..=100.0).contains(p))
            .map(|p| p.round() as u8);
        if let Some(message) = fields.next() {
            return PackageProgress {
                percent,
                line: message.trim().to_string(),
            };
        }
    }
    PackageProgress {
        percent: None,
        line: line.into(),
    }
}

fn execute(
    mut process: Command,
    notify: &mut dyn FnMut(PackageProgress),
    undo_install: Option<PackageName>,
) -> Result<PackageReport, ProviderError> {
    let mut child = process
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| unavailable("pkexec o APT no disponible; comprueba la instalacion"))?;
    let (output, lines) = mpsc::sync_channel(16);
    let mut complete_output = true;
    for pipe in [
        child
            .stdout
            .take()
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let output = output.clone();
        complete_output &= thread::Builder::new()
            .name("serverui-apt-output".into())
            .spawn(move || {
                let _ = drain(pipe, output);
            })
            .is_ok();
    }
    drop(output);
    let mut tail = VecDeque::with_capacity(MAX_LOG_LINES);
    let mut finished = None;
    let mut percent = None;
    let mut output_closed = false;
    loop {
        if output_closed {
            thread::sleep(Duration::from_millis(100));
        } else {
            match lines.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    let mut progress = progress_line(&line);
                    percent = progress.percent.or(percent);
                    progress.percent = percent;
                    if tail.len() == MAX_LOG_LINES {
                        tail.pop_front();
                    }
                    tail.push_back(progress.line.clone());
                    notify(progress);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => output_closed = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        if finished.is_none()
            && let Some(status) = child.try_wait().map_err(|_| {
                unavailable("No se pudo consultar APT; verifica dpkg antes de reintentar")
            })?
        {
            finished = Some((status, Instant::now()));
        }
        if let Some((status, end)) = finished
            && (output_closed || end.elapsed() >= Duration::from_secs(1))
        {
            let mut message = match status.code() {
                Some(0) => "APT: operacion completada".to_string(),
                Some(126 | 127) => "APT: autorizacion cancelada o no concedida".to_string(),
                Some(code) => format!("APT fallo ({code}); revisa la salida y dpkg"),
                None => "APT interrumpido; comprueba dpkg antes de reintentar".to_string(),
            };
            if !complete_output || !output_closed {
                message.push_str(" (salida incompleta)");
            }
            return Ok(PackageReport {
                status: message,
                lines: tail.into(),
                undo_install: undo_install.filter(|_| status.success()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cascade_and_offer_undo_only_for_one_new_package() {
        let name = PackageName::parse("demo-web").unwrap_or_else(|| panic!("fixture"));
        assert!(
            parse_plan(
                &name,
                PackageOperation::Remove,
                "Remv demo-web [1]\nRemv dependent [1]"
            )
            .is_err()
        );
        assert!(parse_plan(&name, PackageOperation::Install, "Remv dependent [1]").is_err());
        assert!(parse_plan(&name, PackageOperation::Remove, "Purg demo-web [1]").is_err());
        let plan = |text| {
            parse_plan(&name, PackageOperation::Install, text)
                .unwrap_or_else(|error| panic!("{error}"))
        };
        assert_eq!(
            undo_install(&plan("Inst demo-web (1 index)\nConf demo-web (1 index)")),
            Some(name.clone())
        );
        assert_eq!(undo_install(&plan("Inst demo-web [1] (2 index)")), None);
        assert_eq!(
            undo_install(&plan("Inst demo-web (1 index)\nInst dependency (1 index)")),
            None
        );
    }

    #[test]
    fn fixed_arguments_keep_targets_separate_and_progress_parses_apt_percent() {
        let name = PackageName::parse("libc++").unwrap_or_else(|| panic!("fixture"));
        let args = apt_args(&name, PackageOperation::Upgrade, false);
        assert!(args.ends_with(&["install".into(), "--".into(), "libc++:native".into()]));
        assert!(args.iter().any(|arg| arg == "--only-upgrade"));
        assert!(args.iter().any(|arg| arg == "--no-remove"));
        assert_eq!(
            progress_line("pmstatus:dpkg-exec:19.7132:Running dpkg").percent,
            Some(20)
        );
        assert_eq!(
            progress_line("pmstatus:dpkg-exec:NaN:Invalid").percent,
            None
        );
    }
}
