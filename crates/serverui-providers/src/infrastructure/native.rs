use super::*;
use crate::services::native::{bounded_command, bounded_streams};
use std::{fs, io::Read, net::IpAddr, path::Path, process::Command, time::Duration};

const LIMIT: usize = 256;
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const ACTION_TIMEOUT: Duration = Duration::from_secs(25);

pub struct LinuxInfrastructure;

fn engine_command(engine: Engine) -> Command {
    match engine {
        Engine::Docker => {
            let mut command = Command::new("/usr/bin/docker");
            command
                .env_remove("DOCKER_CONTEXT")
                .env_remove("DOCKER_HOST")
                .env_remove("DOCKER_TLS")
                .env_remove("DOCKER_TLS_VERIFY")
                .args(["--host", "unix:///var/run/docker.sock", "container"]);
            command
        }
        Engine::Podman => {
            let mut command = Command::new("/usr/bin/podman");
            command
                .env_remove("CONTAINER_HOST")
                .env_remove("CONTAINER_CONNECTION")
                .args(["--remote=false", "container"]);
            command
        }
    }
}

fn container_command(request: &InfrastructureRequest) -> Option<(Engine, Command, Duration)> {
    let (engine, args, timeout) = match request {
        InfrastructureRequest::List(engine) => (
            *engine,
            vec![
                "list",
                "--all",
                "--no-trunc",
                "--format",
                match engine {
                    Engine::Docker => "{{.ID}}|{{.Names}}|{{.State}}",
                    // Podman 3.4 expands tabs through its table writer.
                    Engine::Podman => "{{.ID}}|{{.Names}}|{{.Status}}",
                },
            ],
            QUERY_TIMEOUT,
        ),
        InfrastructureRequest::Logs(engine, id) => (
            *engine,
            vec!["logs", "--tail", "80", "--", id.as_str()],
            QUERY_TIMEOUT,
        ),
        InfrastructureRequest::Change(change) => {
            let id = change.container.id.as_str();
            let args = match change.operation {
                ContainerOperation::Start => vec!["start", "--", id],
                ContainerOperation::Stop => vec!["stop", "--time", "10", "--", id],
                ContainerOperation::Restart => vec!["restart", "--time", "10", "--", id],
            };
            (change.engine, args, ACTION_TIMEOUT)
        }
        InfrastructureRequest::Inventory => return None,
    };
    let mut command = engine_command(engine);
    command.args(args);
    Some((engine, command, timeout))
}

fn verify_target(engine: Engine, id: &ContainerId) -> Result<(), ProviderError> {
    let mut command = engine_command(engine);
    let filter = format!("id={}", id.as_str());
    command.args([
        "list",
        "--all",
        "--no-trunc",
        "--filter",
        &filter,
        "--format",
        "{{.ID}}",
    ]);
    let error = format!(
        "{} no pudo comprobar el ID; revisa permisos y estado",
        engine.label()
    );
    let output = bounded_command(command, QUERY_TIMEOUT, &error)?;
    if String::from_utf8_lossy(&output).trim() != id.as_str() {
        return Err(unavailable("El ID ya no esta disponible; r: actualizar"));
    }
    Ok(())
}

fn parse_containers(text: &str) -> InfrastructureData {
    let mut rows = Vec::new();
    let mut limited = false;
    for line in text.lines() {
        if rows.len() == LIMIT {
            limited = true;
            break;
        }
        let mut fields = line.splitn(3, '|');
        let (Some(id), Some(name), Some(state)) = (
            fields.next().and_then(ContainerId::parse),
            fields.next(),
            fields.next(),
        ) else {
            limited = true;
            continue;
        };
        rows.push(Container {
            id,
            name: name.chars().take(128).collect(),
            state: state.chars().take(32).collect(),
        });
    }
    InfrastructureData::Containers(rows, limited)
}

fn addresses(text: &str) -> Result<Vec<NetworkInterface>, ProviderError> {
    let mut rows = Vec::new();
    for line in text.lines().take(LIMIT + 1) {
        if rows.len() == LIMIT {
            return Err(unavailable(
                "Inventario de red supera el limite de 256 interfaces",
            ));
        }
        let mut fields = line.split_whitespace();
        let (Some(name), Some(state)) = (fields.next(), fields.next()) else {
            return Err(unavailable("Formato de interfaces no reconocido"));
        };
        let addresses = fields
            .filter(|field| {
                field.split_once('/').is_some_and(|(address, prefix)| {
                    address
                        .parse::<IpAddr>()
                        .ok()
                        .zip(prefix.parse::<u8>().ok())
                        .is_some_and(|(ip, bits)| bits <= if ip.is_ipv4() { 32 } else { 128 })
                })
            })
            .take(64)
            .map(str::to_owned)
            .collect();
        rows.push(NetworkInterface {
            name: name.chars().take(64).collect(),
            state: state.chars().take(24).collect(),
            addresses: Some(addresses),
        });
    }
    Ok(rows)
}

fn devices(text: &str) -> Result<Vec<BlockDevice>, ProviderError> {
    let mut rows = Vec::new();
    for line in text.lines().take(LIMIT + 1) {
        if rows.len() == LIMIT {
            return Err(unavailable(
                "Inventario de discos supera el limite de 256 dispositivos",
            ));
        }
        let mut fields = line.split_whitespace();
        let (Some(name), Some(kind), Some(bytes), Some(read_only)) = (
            fields.next(),
            fields.next(),
            fields.next().and_then(|field| field.parse().ok()),
            fields.next().and_then(|field| match field {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            }),
        ) else {
            return Err(unavailable(
                "Formato de dispositivos de bloque no reconocido",
            ));
        };
        rows.push(BlockDevice {
            name: name.chars().take(64).collect(),
            kind: kind.chars().take(24).collect(),
            bytes,
            read_only,
        });
    }
    Ok(rows)
}

fn inventory_command(program: &str, args: &[&str], error: &str) -> Result<String, ProviderError> {
    let mut command = Command::new(program);
    command.args(args);
    bounded_command(command, QUERY_TIMEOUT, error)
        .map(|output| String::from_utf8_lossy(&output).into_owned())
}

fn network() -> Result<Vec<NetworkInterface>, ProviderError> {
    if let Some(ip) = ["/usr/sbin/ip", "/usr/bin/ip", "/sbin/ip", "/bin/ip"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
    {
        return inventory_command(
            ip,
            &["-br", "address", "show"],
            "ip no pudo consultar las interfaces",
        )
        .and_then(|output| addresses(&output));
    }
    let entries = fs::read_dir("/sys/class/net")
        .map_err(|_| unavailable("Inventario de interfaces no disponible"))?;
    let mut rows = Vec::new();
    for entry in entries.take(LIMIT + 1) {
        if rows.len() == LIMIT {
            return Err(unavailable(
                "Inventario de red supera el limite de 256 interfaces",
            ));
        }
        let entry = entry.map_err(|_| unavailable("No se pudo enumerar las interfaces"))?;
        let mut state = String::new();
        let state = fs::File::open(entry.path().join("operstate"))
            .and_then(|file| file.take(64).read_to_string(&mut state))
            .map_or_else(
                |_| "estado no disponible".into(),
                |_| state.trim().to_ascii_uppercase(),
            );
        rows.push(NetworkInterface {
            name: entry.file_name().to_string_lossy().into_owned(),
            state,
            addresses: None,
        });
    }
    rows.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

impl InfrastructureProvider for LinuxInfrastructure {
    fn perform(&mut self, request: &InfrastructureRequest) -> InfrastructureResult {
        let Some((engine, command, timeout)) = container_command(request) else {
            return Ok(InfrastructureData::Inventory(Inventory {
                interfaces: network(),
                devices: inventory_command(
                    "/usr/bin/lsblk",
                    &["-n", "-l", "-b", "-o", "KNAME,TYPE,SIZE,RO"],
                    "lsblk no pudo consultar los dispositivos",
                )
                .and_then(|output| devices(&output)),
            }));
        };
        if !Path::new(command.get_program()).is_file() {
            return Err(unavailable(format!(
                "{} no disponible; e: cambiar de motor",
                engine.label()
            )));
        }
        match request {
            InfrastructureRequest::Logs(_, id) => verify_target(engine, id)?,
            InfrastructureRequest::Change(change) => verify_target(engine, &change.container.id)?,
            _ => {}
        }
        let error = format!(
            "{} rechazo la operacion; comprueba el motor, permisos y estado",
            engine.label()
        );
        if matches!(request, InfrastructureRequest::Logs(..)) {
            let (stdout, stderr) = bounded_streams(command, timeout, &error)?;
            let lines = [&stdout, &stderr]
                .into_iter()
                .flat_map(|stream| {
                    String::from_utf8_lossy(stream)
                        .lines()
                        .rev()
                        .take(80)
                        .map(|line| line.chars().take(256).collect())
                        .collect::<Vec<String>>()
                        .into_iter()
                        .rev()
                })
                .collect();
            return Ok(InfrastructureData::Logs(lines));
        }
        let output = bounded_command(command, timeout, &error)?;
        let text = String::from_utf8_lossy(&output);
        Ok(match request {
            InfrastructureRequest::List(_) => parse_containers(&text),
            _ => InfrastructureData::Changed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_parsers_validate_ip_prefixes_device_fields_and_capacity()
    -> Result<(), ProviderError> {
        let rows = addresses("lo UNKNOWN 127.0.0.1/8 ::1/128 invalid/2 192.0.2.2/40")?;
        assert_eq!(
            rows[0].addresses,
            Some(vec!["127.0.0.1/8".to_owned(), "::1/128".to_owned()])
        );
        let rows = devices("sda disk 17179869184 0\nsr0 rom 4096 1")?;
        assert_eq!(rows[0].bytes, 17_179_869_184);
        assert!(rows[1].read_only);
        assert!(devices("sda disk unknown 1").is_err());
        Ok(())
    }

    #[test]
    fn list_rejects_abbreviated_container_ids() {
        let raw = format!("1234|short|exited\n{}|web|Up 2 seconds ago", "a".repeat(64));
        let InfrastructureData::Containers(rows, limited) = parse_containers(&raw) else {
            panic!("container list");
        };
        assert!(limited);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "web");
        assert_eq!(rows[0].state, "Up 2 seconds ago");
    }
}
