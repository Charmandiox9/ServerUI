"""Linux-only PTY acceptance test. Standard library, no root or host changes.

Run after building serverui and the terminal_probe example. The slave PTY is
kept open to compare its actual termios state before and after each child.
"""

import errno
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import time


def smoke(binary, arguments, keys=None, expected_code=0):
    master, slave = pty.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        before = termios.tcgetattr(slave)
        env = dict(os.environ, TERM="xterm-256color")
        child = subprocess.Popen(
            [binary, *arguments], stdin=slave, stdout=slave, stderr=slave, env=env,
            start_new_session=True,
        )
        output = bytearray()
        deadline = time.monotonic() + 15
        sent = False
        resized = False
        try:
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.05)
                if ready:
                    try:
                        output.extend(os.read(master, 65536))
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                if keys is not None and not sent and b"DEMO" in output:
                    os.write(master, b"\t?")
                    # Crossterm also checks dimensions on draw; signal resize
                    # so the OS event path is exercised on the child process.
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 12, 40, 0, 0))
                    child.send_signal(signal.SIGWINCH)
                    sent = True
                elif sent and not resized:
                    os.write(master, keys)
                    resized = True
                if child.poll() is not None and not ready:
                    break
            if child.poll() is None:
                raise AssertionError(f"child did not exit: {arguments}; sent={sent}, resized={resized}, head={bytes(output[:1500])!r}, tail={bytes(output[-300:])!r}")
            assert child.returncode == expected_code, (arguments, child.returncode, output[-1000:])
            assert termios.tcgetattr(slave) == before, f"raw mode not restored: {arguments}"
            assert b"\x1b[?1049h" in output, "alternate screen not entered"
            assert b"\x1b[?1049l" in output, "alternate screen not restored"
            assert b"\x1b[?25h" in output, "cursor not restored"
            if keys is not None:
                assert sent, "demo frame not observed"
            if "panic" in arguments:
                assert output.index(b"\x1b[?1049l") < output.index(b"injected PTY panic"), "panic printed before restoring screen"
            print(f"PASS {os.path.basename(binary)} {' '.join(arguments)}")
        finally:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=5)
    finally:
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    application = os.path.abspath(sys.argv[1])
    probe = os.path.abspath(sys.argv[2])
    smoke(application, ["--demo", "--ascii", "--no-color", "--refresh-ms", "100"], keys=b"q")
    smoke(application, ["--demo", "--demo-scenario", "empty"], keys=b"\x03")
    smoke(application, ["--demo", "--demo-scenario", "error", "--refresh-ms", "100"], keys=b"q")
    smoke(probe, ["normal"])
    smoke(probe, ["error"], expected_code=1)
    smoke(probe, ["panic"], expected_code=101)
