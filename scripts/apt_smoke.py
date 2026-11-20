"""APT demo PTY and opt-in mutation smoke in a disposable Ubuntu container."""

import fcntl
import hashlib
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

from service_smoke import Screen

FIXTURE = "serverui-apt-fixture"
PKEXEC = Path("/usr/bin/pkexec")


def terminal(binary, demo, exercise, entry=(b"5", "Recomendados APT"), step_timeout=12):
    master, slave = pty.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        before = termios.tcgetattr(slave)
        args = [binary, "--ascii", "--no-color"]
        if demo:
            args.append("--demo")
        child = subprocess.Popen(
            args, stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
            env=dict(os.environ, TERM="xterm-256color", LC_ALL="C"),
        )
        screen = Screen()
        captured = bytearray()

        def pump():
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                captured.extend(chunk)
                screen.feed(chunk)

        def send(keys, expected):
            if keys:
                os.write(master, keys)
            deadline = time.monotonic() + step_timeout
            while time.monotonic() < deadline and child.poll() is None:
                pump()
                if expected in screen.text():
                    return
            raise AssertionError(f"Expected {expected!r}:\n{screen.text()}")

        def resize(width, height):
            nonlocal screen
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
            screen = Screen(width=width, height=height)
            child.send_signal(signal.SIGWINCH)

        try:
            send(b"", "RAM:")
            send(*entry)
            exercise(send, resize)
            os.write(master, b"q")
            deadline = time.monotonic() + 5
            while child.poll() is None and time.monotonic() < deadline:
                pump()
            pump()
            assert child.poll() == 0, "ServerUI did not exit"
            assert termios.tcgetattr(slave) == before
            assert b"\x1b[?1049l" in captured and b"\x1b[?25h" in captured
        finally:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=5)
    finally:
        os.close(master)
        os.close(slave)


def demo(binary):
    def exercise(send, _resize):
        send(b"", "demo-utils")
        send(b"/demo-editor\r", "Cache APT: demo-editor")
        send(b"i", "Simulacion APT.")
        send(b"n", "> demo-editor")
        send(b"i", "Simulacion APT.")
        send(b"yy", "APT simulado: operacion completada")
        send(b"b", "Simulacion APT.")
        send(b"y", "APT simulado: operacion completada")
        send(b"\x1b", "> demo-editor")
        send(b"/\x15no-existe\r", "Sin coincidencias")
        send(b"/\x15\r", "demo-utils")

    terminal(binary, True, exercise)
    print("PASS APT demo: recommendations, search, empty, cancel, apply, undo, terminal restore")


def checked(args):
    result = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
    if result.returncode:
        raise AssertionError(result.stdout)
    return result.stdout


def installed_version():
    query = subprocess.run(
        ["/usr/bin/dpkg-query", "-W", "--showformat=${Status} ${Version}", FIXTURE],
        text=True, capture_output=True, timeout=5,
    )
    prefix = "install ok installed "
    return query.stdout[len(prefix):] if query.stdout.startswith(prefix) else None


def publish_fixture(repo, source, version):
    root = repo / "build"
    (root / "DEBIAN").mkdir(parents=True, exist_ok=True)
    control = (
        f"Package: {FIXTURE}\nVersion: {version}\nArchitecture: all\n"
        "Maintainer: ServerUI smoke <fixture@example.invalid>\n"
        "Description: Disposable Stage 3 acceptance fixture\n"
    )
    (root / "DEBIAN/control").write_text(control, encoding="ascii")
    postinst = root / "DEBIAN/postinst"
    postinst.write_text("#!/bin/sh\nsleep 1\n", encoding="ascii")
    postinst.chmod(0o755)
    deb = repo / f"{FIXTURE}_{version}_all.deb"
    checked(["/usr/bin/dpkg-deb", "--build", "--root-owner-group", str(root), str(deb)])
    data = deb.read_bytes()
    (repo / "Packages").write_text(
        f"{control}Filename: ./{deb.name}\nSize: {len(data)}\nSHA256: {hashlib.sha256(data).hexdigest()}\n",
        encoding="ascii",
    )
    checked([
        "/usr/bin/apt-get", "-o", f"Dir::Etc::sourcelist={source}",
        "-o", "Dir::Etc::sourceparts=-", "-o", "APT::Get::List-Cleanup=false", "update",
    ])


def sandbox(binary):
    if not (
        Path("/.dockerenv").is_file() and os.getuid() == 0
        and os.environ.get("SERVERUI_APT_SANDBOX") == "disposable"
        and not PKEXEC.exists() and not PKEXEC.is_symlink()
    ):
        raise SystemExit("Use the disposable Ubuntu Docker recipe in docs/ACCEPTANCE_STAGE_3.md")
    assert installed_version() is None
    with tempfile.TemporaryDirectory(prefix="serverui-apt-") as temporary:
        repo = Path(temporary)
        repo.chmod(0o755)
        source = Path("/etc/apt/sources.list.d/serverui-acceptance.list")
        decision = repo / "deny"
        with source.open("x", encoding="ascii") as handle:
            handle.write(f"deb [trusted=yes] file:{repo} ./\n")
        # Fake only the authorization boundary in this disposable container.
        # APT, dpkg, progress FD and the application are the real executables.
        with PKEXEC.open("x", encoding="ascii") as handle:
            handle.write(
                '#!/bin/sh\n[ "$1" = --disable-internal-agent ] && '
                '[ "$2" = /usr/bin/apt-get ] || exit 127\n'
                f"[ ! -e '{decision}' ] || exit 126\n"
                'shift\nexec "$@"\n'
            )
        PKEXEC.chmod(0o755)
        try:
            publish_fixture(repo, source, "1")

            def exercise(send, _resize):
                send(f"/{FIXTURE}\r".encode(), f"> {FIXTURE}")
                send(b"i", "Simulacion APT.")
                send(b"n", f"> {FIXTURE}")
                assert installed_version() is None
                decision.touch()
                send(b"i", "Simulacion APT.")
                send(b"y", "autorizacion cancelada")
                decision.unlink()
                assert installed_version() is None
                send(b"\x1b", f"> {FIXTURE}")

                with open("/var/lib/dpkg/lock-frontend", "w", encoding="ascii") as lock:
                    fcntl.lockf(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    send(b"i", "Simulacion APT.")
                    send(b"y", "APT fallo (100)")
                assert installed_version() is None
                send(b"\x1b", f"> {FIXTURE}")
                send(b"i", "Simulacion APT.")
                send(b"y", "Progreso APT:")
                send(b"1", "CPU:")
                send(b"5", "APT: operacion completada")
                assert installed_version() == "1"

                send(b"b", "Simulacion APT.")
                send(b"y", "APT: operacion completada")
                assert installed_version() is None
                send(b"\x1b", f"> {FIXTURE}")
                send(b"i", "Simulacion APT.")
                send(b"y", "APT: operacion completada")
                assert installed_version() == "1"
                publish_fixture(repo, source, "2")
                send(b"\x1b", f"> {FIXTURE}")
                send(b"u", "Simulacion APT.")
                send(b"y", "APT: operacion completada")
                assert installed_version() == "2"
                send(b"\x1b", f"> {FIXTURE}")
                send(b"d", "Simulacion APT.")
                send(b"y", "APT: operacion completada")
                assert installed_version() is None

            terminal(binary, False, exercise)
        finally:
            PKEXEC.unlink()
            source.unlink()
    print("PASS Ubuntu APT: cancel, deny, lock failure, install, progress, undo, upgrade, remove")


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[2] != "--sandbox"):
        raise SystemExit("usage: apt_smoke.py SERVERUI [--sandbox]")
    binary = str(Path(sys.argv[1]).resolve())
    if len(sys.argv) == 3:
        sandbox(binary)
    else:
        demo(binary)
