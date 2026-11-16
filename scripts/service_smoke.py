"""Linux PTY regression for Stage 2 DEMO; never calls real service tools.

Checks the rendered terminal state rather than searching differential ANSI bytes.
The small decoder handles the cursor/erase sequences emitted by Crossterm here.
"""
import codecs
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time


class Screen:
    def __init__(self, width=80, height=24):
        self.width, self.height = width, height
        self.cells = [[" "] * width for _ in range(height)]
        self.x = self.y = 0
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            if self.pending.startswith("\x1b["):
                match = re.match(r"\x1b\[([0-9;?]*)([@-~])", self.pending)
                if not match:
                    return
                args, code = match.groups()
                values = [int(p or 0) for p in args.lstrip("?").split(";")]
                n = values[0] or 1
                if code in "Hf":
                    self.y = min(self.height - 1, n - 1)
                    self.x = min(self.width - 1, (values[1] or 1) - 1 if len(values) > 1 else 0)
                elif code == "A":
                    self.y = max(0, self.y - n)
                elif code == "B":
                    self.y = min(self.height - 1, self.y + n)
                elif code == "C":
                    self.x = min(self.width - 1, self.x + n)
                elif code == "D":
                    self.x = max(0, self.x - n)
                elif code == "G":
                    self.x = min(self.width - 1, n - 1)
                elif code == "J" and values[0] in (2, 3):
                    self.cells = [[" "] * self.width for _ in range(self.height)]
                elif code == "K":
                    start = 0 if values[0] in (1, 2) else self.x
                    end = self.width if values[0] in (0, 2) else self.x + 1
                    self.cells[self.y][start:end] = [" "] * (end - start)
                self.pending = self.pending[len(match.group(0)):]
            elif self.pending.startswith("\x1b"):
                if len(self.pending) < 2:
                    return
                self.pending = self.pending[2:]
            else:
                char, self.pending = self.pending[0], self.pending[1:]
                if char == "\r":
                    self.x = 0
                elif char == "\n":
                    self.y = min(self.height - 1, self.y + 1)
                elif char >= " ":
                    self.cells[self.y][self.x] = char
                    self.x = min(self.width - 1, self.x + 1)

    def text(self):
        return "\n".join("".join(row) for row in self.cells)


def smoke(binary):
    master, slave = pty.openpty()
    child = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        before = termios.tcgetattr(slave)
        child = subprocess.Popen([binary, "--demo", "--ascii", "--no-color"],
                                 stdin=slave, stdout=slave, stderr=slave,
                                 start_new_session=True, env=dict(os.environ, TERM="xterm-256color"))
        screen = Screen()
        captured = bytearray()

        def wait_for(text):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if text in screen.text():
                    return
                readable, _, _ = select.select([master], [], [], 0.05)
                if readable:
                    data = os.read(master, 65536)
                    captured.extend(data)
                    screen.feed(data)
                if child.poll() is not None:
                    break
            raise AssertionError(f"Expected {text!r}:\n{screen.text()}")

        def send(keys, expected):
            os.write(master, keys)
            wait_for(expected)

        wait_for("RAM:")
        send(b"3", "cron.service")
        send(b"d", "Detener?")
        send(b"\x1b", "a:iniciar")
        send(b"h", "Sin acciones ejecutadas")
        send(b"\x1b", "a:iniciar")
        send(b"a", "Iniciar?")
        send(b"y", "Comando completado")
        send(b"h", "Iniciar cron.service")
        send(b"\x1b", "a:iniciar")
        send(b"\r", "DEMO Sin acceso al journal del host")
        send(b"\x1b", "a:iniciar")
        send(b"/ssh\r", "ssh.service")
        send(b"?", "Ayuda")
        send(b"?", "ssh.service")
        send(b"1", "RAM:")
        os.write(master, b"q")
        deadline = time.monotonic() + 5
        while child.poll() is None and time.monotonic() < deadline:
            readable, _, _ = select.select([master], [], [], 0.05)
            if readable:
                captured.extend(os.read(master, 65536))
        assert child.poll() == 0, "Demo did not exit cleanly"
        assert termios.tcgetattr(slave) == before, "Terminal modes not restored"
        assert b"\x1b[?1049l" in captured, "Alternate screen not restored"
        assert b"\x1b[?25h" in captured, "Cursor not restored"
        print("PASS Stage 2 demo: cancel, confirm once, history, logs, filter, help, dashboard, cleanup")
    finally:
        if child is not None:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=5)
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    smoke(os.path.abspath(sys.argv[1]))
