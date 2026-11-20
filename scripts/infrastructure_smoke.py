"""PTY checks for Stage 4 demo and disposable Linux adapter fixtures."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile

from apt_smoke import terminal

ENTRY = (b"6", "Docker local")
ENGINES = (Path("/usr/bin/docker"), Path("/usr/bin/podman"))
FIXTURE = """#!/usr/bin/python3
import os
from pathlib import Path
import sys

engine = Path(sys.argv[0]).name
state = Path(os.environ["SERVERUI_ENGINE_FIXTURE"])
args = sys.argv[1:]
if engine == "docker":
    assert args[:3] == ["--host", "unix:///var/run/docker.sock", "container"]
    assert "DOCKER_HOST" not in os.environ and "DOCKER_CONTEXT" not in os.environ
    args = args[3:]
else:
    assert args[:2] == ["--remote=false", "container"]
    assert "CONTAINER_HOST" not in os.environ and "CONTAINER_CONNECTION" not in os.environ
    args = args[2:]
container_id = ("a" if engine == "docker" else "b") * 64
operation, *options = args
status = state / engine
if operation == "list":
    field = "{{.State}}" if engine == "docker" else "{{.Status}}"
    if options == ["--all", "--no-trunc", "--filter", "id=" + container_id, "--format", "{{.ID}}"]:
        if not (state / "missing").exists():
            print(container_id)
    else:
        assert options == ["--all", "--no-trunc", "--format", "{{.ID}}|{{.Names}}|" + field]
        print(container_id, engine + "-fixture", status.read_text(), sep="|")
elif operation == "logs":
    assert options == ["--tail", "80", "--", container_id]
    print(engine + " fixture stdout")
    print(engine + " fixture stderr", file=sys.stderr)
else:
    assert operation in ("start", "stop", "restart")
    flags = [] if operation == "start" else ["--time", "10"]
    assert options == flags + ["--", container_id]
    with (state / "operations").open("a") as journal:
        print(engine, operation, container_id, file=journal)
    if (state / "deny").exists():
        sys.exit(1)
    status.write_text("exited" if operation == "stop" else "running")
"""


def demo(binary):
    def exercise(send, resize):
        send(b"", "demo-web  running")
        send(b"d", "Detener (Docker)")
        send(b"n", "demo-web  running")
        send(b"d", "Detener (Docker)")
        resize(36, 10)
        send(b"", "minimo 40x12")
        send(b"y", "minimo 40x12")
        resize(80, 24)
        send(b"", "demo-web  running")
        send(b"d", "Detener (Docker)")
        send(b"yy", "demo-web  exited")
        send(b"\r", "logs simulados")
        send(b"\x1b", "demo-web  exited")
        send(b"e", "demo-job  exited")
        send(b"a", "Iniciar (Podman)")
        send(b"y", "demo-job  running")
        send(b"7", "192.0.2.10/24")
        send(b"b", "vda-demo")
        send(b"m", "MiB disponibles")

    terminal(binary, True, exercise, ENTRY)
    print("PASS infrastructure demo: engines, visible confirmation, logs, inventory, terminal restore")


def sandbox(binary):
    if not (
        Path("/.dockerenv").is_file() and os.getuid() == 0
        and os.environ.get("SERVERUI_INFRA_SANDBOX") == "disposable"
        and all(not path.exists() and not path.is_symlink() for path in ENGINES)
        and not Path("/var/run/docker.sock").exists()
    ):
        raise SystemExit("Use the disposable Ubuntu recipe in docs/ACCEPTANCE_STAGE_4.md")

    with tempfile.TemporaryDirectory(prefix="serverui-infra-") as temporary:
        root = Path(temporary)
        (root / "docker").write_text("running", encoding="ascii")
        (root / "podman").write_text("exited", encoding="ascii")
        journal = root / "operations"
        deny = root / "deny"
        missing = root / "missing"
        os.environ.update(
            SERVERUI_ENGINE_FIXTURE=temporary, DOCKER_HOST="tcp://invalid.example:2375",
            DOCKER_CONTEXT="fixture-remote", CONTAINER_HOST="ssh://invalid.example",
            CONTAINER_CONNECTION="fixture-remote",
        )
        created = []
        try:
            for path in ENGINES:
                with path.open("x", encoding="ascii") as executable:
                    created.append(path)
                    executable.write(FIXTURE)
                path.chmod(0o755)

            def exercise(send, _resize):
                send(b"", "docker-fixture  running")
                send(b"\r", "docker fixture stderr")
                send(b"", "docker fixture stdout")
                send(b"\x1b", "docker-fixture  running")
                send(b"d", "Detener (Docker)")
                send(b"n", "docker-fixture  running")
                assert not journal.exists()
                missing.touch()
                send(b"d", "Detener (Docker)")
                send(b"y", "El ID ya no esta disponible")
                missing.unlink()
                assert not journal.exists()
                deny.touch()
                send(b"d", "Detener (Docker)")
                send(b"y", "Docker rechazo la operacion")
                deny.unlink()
                assert (root / "docker").read_text() == "running"
                send(b"d", "Detener (Docker)")
                send(b"yy", "docker-fixture  exited")
                send(b"R", "Reiniciar (Docker)")
                send(b"y", "docker-fixture  running")
                send(b"e", "podman-fixture  exited")
                send(b"a", "Iniciar (Podman)")
                send(b"y", "podman-fixture  running")
                send(b"7", "Interfaces y direcciones IP")
                if any(Path(path).is_file() for path in ("/usr/sbin/ip", "/usr/bin/ip", "/sbin/ip", "/bin/ip")):
                    send(b"", "127.0.0.1/8")
                else:
                    send(b"", "IP no disponible")
                send(b"b", "Dispositivos de bloque")
                devices = subprocess.check_output(
                    ["/usr/bin/lsblk", "-n", "-l", "-b", "-o", "KNAME,TYPE,SIZE,RO"], text=True,
                ).splitlines()
                if devices:
                    send(b"", devices[0].split()[0])
                send(b"m", "Montajes y capacidad")

            terminal(binary, False, exercise, ENTRY)
            assert journal.read_text().splitlines() == [
                f"docker stop {'a' * 64}", f"docker stop {'a' * 64}",
                f"docker restart {'a' * 64}", f"podman start {'b' * 64}",
            ]
        finally:
            for path in created:
                path.unlink()
    print("PASS Stage 4 Linux: exact argv, stale target, logs, denial, once-only actions, live inventory")


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[2] != "--sandbox"):
        raise SystemExit("usage: infrastructure_smoke.py SERVERUI [--sandbox]")
    binary = str(Path(sys.argv[1]).resolve())
    if len(sys.argv) == 3:
        sandbox(binary)
    else:
        demo(binary)
