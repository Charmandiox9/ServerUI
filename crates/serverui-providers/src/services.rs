use serverui_core::{domain::ProviderError, services::*};

pub struct MockServices {
    units: Vec<ServiceInfo>,
    scenario: crate::Scenario,
    step: usize,
}
impl MockServices {
    pub fn new(scenario: crate::Scenario) -> Self {
        let units = ["cron.service", "nginx.service", "ssh.service"]
            .into_iter()
            .filter_map(|name| UnitName::parse(name).ok())
            .map(|unit| ServiceInfo {
                unit,
                load: "loaded".into(),
                active: "active".into(),
                sub: "running".into(),
                description: "Servicio simulado".into(),
            })
            .collect();
        Self {
            units,
            scenario,
            step: 0,
        }
    }
}
impl ServiceProvider for MockServices {
    fn perform(&mut self, request: &ServiceRequest) -> ServiceResult {
        self.step = self.step.saturating_add(1);
        if self.scenario == crate::Scenario::RecoverableError && self.step == 1 {
            return Err(ProviderError::Unavailable(
                "Permiso denegado simulado; r: reintentar".into(),
            ));
        }
        match request {
            ServiceRequest::List => Ok(ServiceData::List {
                units: if self.scenario == crate::Scenario::Empty {
                    vec![]
                } else {
                    self.units.clone()
                },
                limited: false,
            }),
            ServiceRequest::Logs(unit) => Ok(ServiceData::Logs(vec![
                format!("DEMO {}: inicio simulado", unit.as_str()),
                "DEMO Sin acceso al journal del host".into(),
            ])),
            ServiceRequest::Execute(unit, operation) => {
                let Some(service) = self.units.iter_mut().find(|s| &s.unit == unit) else {
                    return Err(ProviderError::Unavailable(
                        "Unidad simulada desaparecida".into(),
                    ));
                };
                match operation {
                    ServiceOperation::Start | ServiceOperation::Restart => {
                        service.active = "active".into();
                        service.sub = "running".into();
                    }
                    ServiceOperation::Stop => {
                        service.active = "inactive".into();
                        service.sub = "dead".into();
                    }
                    ServiceOperation::Enable => {}
                }
                Ok(ServiceData::Executed)
            }
        }
    }
}

// Compile and exercise the process boundary on Windows too, using only fixture
// commands in tests. The public real provider is exported only on Linux.
#[cfg(any(target_os = "linux", test))]
mod native {
    use super::*;
    use std::{
        io::Read,
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };
    const OUTPUT_LIMIT: u64 = 256 * 1024;
    const TIMEOUT: Duration = Duration::from_secs(5);

    fn failure(message: &str) -> ProviderError {
        ProviderError::Unavailable(message.into())
    }

    // Two readers drain independent pipes to avoid deadlock. Each reads at most
    // LIMIT+1 bytes and exits; only one command may be in flight in the worker.
    fn reader(
        input: impl Read + Send + 'static,
    ) -> std::io::Result<mpsc::Receiver<std::io::Result<Vec<u8>>>> {
        let (send, receive) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("serverui-command-output".into())
            .spawn(move || {
                let mut bytes = Vec::new();
                let result = input
                    .take(OUTPUT_LIMIT + 1)
                    .read_to_end(&mut bytes)
                    .map(|_| bytes);
                let _ = send.try_send(result);
            })?;
        Ok(receive)
    }
    pub(super) fn command(
        mut command: Command,
        timeout: Duration,
    ) -> Result<Vec<u8>, ProviderError> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("LC_ALL", "C")
            .env("SYSTEMD_COLORS", "0")
            .env("SYSTEMD_PAGER", "cat")
            .spawn()
            .map_err(|_| failure("Herramienta no disponible o sin permiso de ejecucion"))?;
        let setup = (|| {
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| failure("No se pudo leer la salida"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| failure("No se pudo leer el diagnostico"))?;
            let out = reader(stdout).map_err(|_| failure("No se pudo iniciar lector"))?;
            let err = reader(stderr).map_err(|_| failure("No se pudo iniciar lector"))?;
            Ok((out, err))
        })();
        let (out, err) = match setup {
            Ok(pair) => pair,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let deadline = Instant::now() + timeout;
        let mut output = None;
        let mut errors = None;
        let result = loop {
            for (receiver, slot) in [(&out, &mut output), (&err, &mut errors)] {
                if slot.is_none() {
                    match receiver.try_recv() {
                        Ok(Ok(bytes)) => *slot = Some(bytes),
                        Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                            *slot = Some(vec![0; OUTPUT_LIMIT as usize + 1]);
                        }
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                }
            }
            if output
                .as_ref()
                .is_some_and(|b| b.len() > OUTPUT_LIMIT as usize)
                || errors
                    .as_ref()
                    .is_some_and(|b| b.len() > OUTPUT_LIMIT as usize)
            {
                break Err(failure(
                    "Salida excede 256 KiB; resultado incierto si era una accion; comprobar estado",
                ));
            }
            match child.try_wait() {
                Ok(Some(status)) if output.is_some() && errors.is_some() => {
                    break if status.success() {
                        Ok(output.take().unwrap_or_default())
                    } else {
                        Err(failure(
                            "Comando rechazado o fallido: comprobar permisos, systemd y unidad; no se eleva privilegio",
                        ))
                    };
                }
                Err(_) => {
                    break Err(failure(
                        "Error al consultar comando; comprobar estado antes de reintentar",
                    ));
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                break Err(failure(
                    "Tiempo agotado; resultado incierto. Comprobar estado; no reintentar automaticamente",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        };
        if result.is_err() {
            let _ = child.kill();
        }
        // Only the service worker waits for its direct child, never the UI.
        let _ = child.wait();
        result
    }

    pub struct LinuxServices;
    fn spec(request: &ServiceRequest) -> (&'static str, Vec<String>) {
        match request {
            ServiceRequest::List => (
                "/usr/bin/systemctl",
                [
                    "--system",
                    "--no-pager",
                    "--no-legend",
                    "--plain",
                    "--full",
                    "--no-ask-password",
                    "list-units",
                    "--type=service",
                    "--all",
                ]
                .map(String::from)
                .to_vec(),
            ),
            ServiceRequest::Logs(unit) => (
                "/usr/bin/journalctl",
                vec![
                    "--system".into(),
                    "--no-pager".into(),
                    "--quiet".into(),
                    "--output=short-iso".into(),
                    "--lines=80".into(),
                    "--unit".into(),
                    unit.as_str().into(),
                ],
            ),
            ServiceRequest::Execute(unit, operation) => (
                "/usr/bin/systemctl",
                vec![
                    "--system".into(),
                    "--no-pager".into(),
                    "--no-ask-password".into(),
                    operation.argument().into(),
                    "--".into(),
                    unit.as_str().into(),
                ],
            ),
        }
    }
    fn parse_list(bytes: &[u8]) -> ServiceData {
        let text = String::from_utf8_lossy(bytes);
        let mut units = Vec::new();
        let mut limited = false;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut fields = line.split_whitespace();
            let Some(name) = fields.next() else {
                continue;
            };
            let Ok(unit) = UnitName::parse(name) else {
                limited = true;
                continue;
            };
            let (Some(load), Some(active), Some(sub)) =
                (fields.next(), fields.next(), fields.next())
            else {
                limited = true;
                continue;
            };
            if units.len() == MAX_SERVICES {
                limited = true;
                break;
            }
            units.push(ServiceInfo {
                unit,
                load: load.chars().take(32).collect(),
                active: active.chars().take(32).collect(),
                sub: sub.chars().take(32).collect(),
                description: fields
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(256)
                    .collect(),
            });
        }
        ServiceData::List { units, limited }
    }
    impl ServiceProvider for LinuxServices {
        fn perform(&mut self, request: &ServiceRequest) -> ServiceResult {
            let (program, arguments) = spec(request);
            let mut process = Command::new(program);
            process.args(arguments);
            let bytes = command(process, TIMEOUT)?;
            Ok(match request {
                ServiceRequest::List => parse_list(&bytes),
                ServiceRequest::Logs(_) => ServiceData::Logs(
                    String::from_utf8_lossy(&bytes)
                        .lines()
                        .take(MAX_LOG_LINES)
                        .map(|l| l.chars().take(256).collect())
                        .collect(),
                ),
                ServiceRequest::Execute(..) => ServiceData::Executed,
            })
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn arguments_are_fixed_and_unit_is_a_separate_argument() {
            let _provider = LinuxServices;
            let unit = UnitName::parse("demo@1.service").unwrap_or_else(|e| panic!("{e}"));
            for operation in [
                ServiceOperation::Start,
                ServiceOperation::Stop,
                ServiceOperation::Restart,
                ServiceOperation::Enable,
            ] {
                let (program, args) = spec(&ServiceRequest::Execute(unit.clone(), operation));
                assert_eq!(program, "/usr/bin/systemctl");
                assert_eq!(
                    args,
                    [
                        "--system",
                        "--no-pager",
                        "--no-ask-password",
                        operation.argument(),
                        "--",
                        "demo@1.service"
                    ]
                );
            }
        }
        #[test]
        fn parser_handles_failed_and_unsupported_units_and_limits() {
            let ServiceData::List { units, limited } = parse_list(b"ssh.service loaded active running OpenBSD server\nbad*.service loaded failed failed bad\nfixture.service loaded failed failed Fixture\n") else { panic!("list") };
            assert!(limited);
            assert_eq!(units.len(), 2);
            assert_eq!(units[1].active, "failed");
            let ServiceData::List { units, limited } = parse_list(
                "a.service loaded active running A\n"
                    .repeat(MAX_SERVICES + 1)
                    .as_bytes(),
            ) else {
                panic!("list")
            };
            assert_eq!(units.len(), MAX_SERVICES);
            assert!(limited);
        }
        #[test]
        fn command_runner_handles_success_denial_timeout_and_output_limit() -> std::io::Result<()> {
            // Spawn our Rust test executable: independent of Python startup,
            // site initialization and PATH differences on hosted Windows.
            fn fixture(scenario: &str) -> std::io::Result<Command> {
                let mut command = Command::new(std::env::current_exe()?);
                command
                    .args([
                        "--exact",
                        "services::native::tests::command_fixture",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("SERVERUI_COMMAND_FIXTURE", scenario);
                Ok(command)
            }
            let output = command(fixture("success")?, TIMEOUT).map_err(std::io::Error::other)?;
            assert!(String::from_utf8_lossy(&output).contains("SERVERUI_FIXTURE_OK"));
            assert!(command(fixture("denial")?, TIMEOUT).is_err());
            assert!(command(fixture("timeout")?, Duration::from_millis(20)).is_err());
            assert!(command(fixture("limit")?, TIMEOUT).is_err());
            Ok(())
        }
        #[test]
        #[ignore = "subprocess fixture, launched explicitly by the command boundary test"]
        fn command_fixture() -> std::io::Result<()> {
            match std::env::var("SERVERUI_COMMAND_FIXTURE").as_deref() {
                Ok("success") => println!("SERVERUI_FIXTURE_OK"),
                Ok("denial") => std::process::exit(7),
                Ok("timeout") => loop {
                    thread::park();
                },
                Ok("limit") => {
                    use std::io::Write;
                    std::io::stdout().write_all(&vec![b'x'; 300_000])?;
                }
                _ => return Err(std::io::Error::other("Fixture scenario missing")),
            }
            Ok(())
        }
    }
}
#[cfg(target_os = "linux")]
pub use native::LinuxServices;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mock_actions_change_only_simulated_state_and_errors_recover() {
        let mut provider = MockServices::new(crate::Scenario::RecoverableError);
        assert!(provider.perform(&ServiceRequest::List).is_err());
        let Ok(ServiceData::List { units, .. }) = provider.perform(&ServiceRequest::List) else {
            panic!("list")
        };
        let target = units[0].unit.clone();
        assert_eq!(
            provider.perform(&ServiceRequest::Execute(
                target.clone(),
                ServiceOperation::Stop
            )),
            Ok(ServiceData::Executed)
        );
        let Ok(ServiceData::List { units, .. }) = provider.perform(&ServiceRequest::List) else {
            panic!("list")
        };
        assert_eq!(units[0].active, "inactive");
        assert!(matches!(
            provider.perform(&ServiceRequest::Logs(target)),
            Ok(ServiceData::Logs(_))
        ));
    }
}
