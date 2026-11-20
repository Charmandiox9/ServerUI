"""Check the actual local container engines on a disposable GitHub-hosted VM."""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from uuid import uuid4

from apt_smoke import terminal

DOCKER = ("/usr/bin/docker", "--host", "unix:///var/run/docker.sock")
PODMAN = ("/usr/bin/podman", "--remote=false")
IMAGE = (
    "docker.io/library/ubuntu:22.04@"
    "sha256:9abcaffc92a5279fff3a6a41635c0800ea33e528425fb389658806476e7defde"
)
LABEL = "org.serverui.test-fixture"
NAME_PREFIX = "serverui-ci-"


def client(engine, *arguments, timeout=25):
    result = subprocess.run(
        (*engine, *arguments), capture_output=True, text=True,
        stdin=subprocess.DEVNULL, timeout=timeout,
    )
    if result.returncode:
        raise RuntimeError(
            f"{Path(engine[0]).name} {arguments[0]} failed ({result.returncode}): {result.stderr.strip()}"
        )
    return result


def inspect(engine, container_id):
    info, = json.loads(client(engine, "container", "inspect", "--", container_id).stdout)
    if info["Id"] != container_id:
        raise AssertionError("Container identity changed")
    return info


def remove_fixture(engine, container_id, token):
    labels = inspect(engine, container_id)["Config"]["Labels"]
    if labels.get(LABEL) != token:
        raise AssertionError("Container fixture label changed")
    client(engine, "container", "rm", "--force", "--", container_id)


def create_fixture(engine, stack):
    token = uuid4().hex[:12]
    name = f"{NAME_PREFIX}{token}"
    client(engine, "image", "pull", IMAGE, timeout=120)
    result = client(
        engine, "container", "create",
        "--name", name, "--label", f"{LABEL}={token}", "--network", "none",
        "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
        "--pull", "never", IMAGE,
        "sh", "-c", 'trap "exit 0" TERM INT; printf "%s\\n" "$1"; printf "%s\\n" "$2" >&2; sleep 600 & wait',
        "fixture", f"stdout-{token}", f"stderr-{token}",
    )
    container_id = result.stdout.strip()
    if not re.fullmatch(r"[a-f0-9]{64}", container_id):
        raise AssertionError("Engine did not return a complete container ID")
    stack.callback(remove_fixture, engine, container_id, token)
    return container_id, name, token


def verify_cycle(engine, label, fixture, send):
    container_id, name, token = fixture

    def wait_for_row():
        template = "{{.State}}" if engine == DOCKER else "{{.Status}}"
        status = client(
            engine, "container", "list", "--all", "--no-trunc", "--filter",
            f"id={container_id}", "--format", template,
        ).stdout.strip()
        if not status or "\n" in status:
            raise AssertionError("Expected exactly one state for the fixture")
        send(b"", f"> {name}  {status.split()[0]} ")

    def operation(key, prompt):
        send(key, f"{prompt} ({label})")
        send(b"", container_id)
        send(b"y", "Operacion completada")
        wait_for_row()

    wait_for_row()
    before = inspect(engine, container_id)["State"]
    assert not before["Running"]
    send(b"a", f"Iniciar ({label})")
    send(b"n", f"> {name} ")
    after = inspect(engine, container_id)["State"]
    assert (after["Status"], after["StartedAt"]) == (before["Status"], before["StartedAt"])

    operation(b"a", "Iniciar")
    started = inspect(engine, container_id)["State"]
    assert started["Running"]
    markers = (f"stdout-{token}", f"stderr-{token}")
    deadline = time.monotonic() + 5
    while True:
        captured = client(engine, "container", "logs", "--tail", "80", "--", container_id)
        if all(marker in captured.stdout + captured.stderr for marker in markers):
            break
        if time.monotonic() >= deadline:
            raise AssertionError(f"{label} did not emit the fixture logs")
        time.sleep(0.05)
    send(b"\r", markers[0])
    send(b"", markers[1])
    send(b"\x1b", f"> {name} ")

    operation(b"R", "Reiniciar")
    restarted = inspect(engine, container_id)["State"]
    assert restarted["Running"] and restarted["StartedAt"] != started["StartedAt"]

    operation(b"d", "Detener")
    stopped = inspect(engine, container_id)["State"]
    assert not stopped["Running"] and stopped["Status"] in ("exited", "stopped")
    print(f"PASS {label}: cancel, start, both log streams, restart, stop")


def smoke(binary):
    if not (
        os.environ.get("GITHUB_ACTIONS") == "true"
        and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
        and os.geteuid() != 0
        and not Path("/.dockerenv").exists()
    ):
        raise SystemExit("Run only on a disposable GitHub-hosted Ubuntu VM as a normal user")
    for key in ("DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_TLS", "DOCKER_TLS_VERIFY",
                "CONTAINER_HOST", "CONTAINER_CONNECTION"):
        os.environ.pop(key, None)
    with ExitStack() as stack:
        docker = create_fixture(DOCKER, stack)
        podman = create_fixture(PODMAN, stack)

        def exercise(send, _resize):
            verify_cycle(DOCKER, "Docker", docker, send)
            send(b"e", "Podman local")
            verify_cycle(PODMAN, "Podman", podman, send)

        terminal(binary, False, exercise, (b"6", "Docker local"), step_timeout=35)
    print("PASS disposable Docker and Podman fixtures were removed")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: engine_vm_smoke.py SERVERUI")
    smoke(str(Path(sys.argv[1]).resolve()))
