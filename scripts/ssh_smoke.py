"""Ubuntu container fixture: SSH over private loopback, no published ports.

Runs a real SSH client/server and the demo as an unprivileged remote user.
Root is used only to provision the disposable container's sshd. No credentials
or server state are copied from the host. Requires OpenSSH and the fixture user
created by Dockerfile.ubuntu-smoke. This is not an application daemon.
"""

import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

from pty_smoke import smoke


def run(binary):
    # StrictModes rejects an authorized key below world-writable /tmp.
    with tempfile.TemporaryDirectory(prefix="serverui-ssh-", dir="/run/sshd") as directory:
        # The sshd child changes identity; public key traversal must be possible.
        os.chmod(directory, 0o755)
        folder = Path(directory)
        client_key = str(folder / "client_key")
        host_key = str(folder / "host_key")
        for key in [client_key, host_key]:
            subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key], check=True)
        config = folder / "sshd_config"
        config.write_text(
            "Port 2222\nListenAddress 127.0.0.1\n"
            f"HostKey {host_key}\nPidFile {folder / 'pid'}\n"
            f"AuthorizedKeysFile {client_key}.pub\n"
            "PasswordAuthentication no\nKbdInteractiveAuthentication no\n"
            "PermitRootLogin no\nUsePAM no\nAllowUsers serverui-test\n"
            "StrictModes yes\nLogLevel ERROR\n",
            encoding="ascii",
        )
        server = subprocess.Popen(["/usr/sbin/sshd", "-D", "-e", "-f", str(config)])
        try:
            deadline = time.monotonic() + 5
            while True:
                if server.poll() is not None:
                    raise AssertionError("fixture sshd exited")
                try:
                    with socket.create_connection(("127.0.0.1", 2222), timeout=0.2):
                        break
                except OSError:
                    if time.monotonic() > deadline:
                        raise AssertionError("fixture sshd not ready")
                    time.sleep(0.05)
            args = [
                "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=accept-new",
                "-o", f"UserKnownHostsFile={folder / 'known_hosts'}",
                "-o", "LogLevel=ERROR", "-i", client_key,
                "-p", "2222", "serverui-test@127.0.0.1",
            ]
            identity = subprocess.check_output(["ssh", *args, "id", "-u"], text=True).strip()
            assert identity.isdigit() and identity != "0", "demo user must not be root"
            print(f"Remote demo UID: {identity}", flush=True)
            smoke("/usr/bin/ssh", ["-tt", *args, binary, "--demo", "--ascii", "--no-color"], keys=b"q")
            smoke("/usr/bin/ssh", ["-tt", *args, binary, "--demo", "--demo-scenario", "error"], keys=b"\x03")
        finally:
            server.terminate()
            server.wait(timeout=5)


if __name__ == "__main__":
    run(os.path.abspath(sys.argv[1]))
