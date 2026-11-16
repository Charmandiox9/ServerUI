"""Measure a prebuilt Linux demo in its own PTY, without changing the host.

Usage: python3 scripts/benchmark_demo.py /path/to/serverui --seconds 600
Measurements include startup, first sample, help response, CPU time and RSS.
Pass --real to exercise LinuxProvider instead of the isolated demo. Output is JSON.
"""

import argparse
import fcntl
import json
import os
import platform
import pty
import select
import struct
import subprocess
import termios
import time


def measure(binary, seconds, real=False):
    master, slave = pty.openpty()
    child = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        before = termios.tcgetattr(slave)
        started = time.monotonic()
        child = subprocess.Popen(
            [binary, *([] if real else ["--demo"]), "--ascii", "--no-color"],
            stdin=slave, stdout=slave, stderr=slave,
            env=dict(os.environ, TERM="xterm-256color"), start_new_session=True,
        )

        def drain(timeout=0.05):
            ready, _, _ = select.select([master], [], [], timeout)
            return os.read(master, 65536) if ready else b""

        output = bytearray()

        def wait_for(marker, timeout=5):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if marker in output:
                    end = output.index(marker) + len(marker)
                    del output[:end]
                    return
                output.extend(drain())
                if child.poll() is not None:
                    raise AssertionError(f"demo exited: {bytes(output)!r}")
            raise AssertionError(f"frame not observed: {marker!r}")

        def cpu_seconds():
            with open(f"/proc/{child.pid}/stat", encoding="ascii") as stream:
                fields = stream.read().rsplit(")", 1)[1].split()
            return (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")

        def rss_bytes():
            with open(f"/proc/{child.pid}/status", encoding="ascii") as stream:
                for line in stream:
                    if line.startswith("VmRSS:"):
                        return int(line.split()[1]) * 1024
            raise AssertionError("VmRSS unavailable")

        wait_for(b"LINUX" if real else b"DEMO")
        first_frame_ms = (time.monotonic() - started) * 1000
        wait_for(b"RAM:")
        first_sample_ms = (time.monotonic() - started) * 1000
        response_started = time.monotonic()
        os.write(master, b"?")
        wait_for(b"Ayuda")
        help_ms = (time.monotonic() - response_started) * 1000
        os.write(master, b"?")
        baseline_cpu = cpu_seconds()
        baseline_time = time.monotonic()
        samples = []
        next_sample = baseline_time
        while time.monotonic() - baseline_time < seconds:
            drain()
            if child.poll() is not None:
                raise AssertionError("demo exited during measurement")
            now = time.monotonic()
            if now >= next_sample:
                samples.append((round(now - baseline_time, 2), rss_bytes()))
                next_sample = now + 1
        elapsed = time.monotonic() - baseline_time
        cpu_percent = (cpu_seconds() - baseline_cpu) / elapsed * 100
        os.write(master, b"q")
        deadline = time.monotonic() + 5
        while child.poll() is None and time.monotonic() < deadline:
            drain()
        assert child.poll() == 0, "demo did not exit cleanly"
        assert termios.tcgetattr(slave) == before, "raw mode not restored"
        result = {
            "platform": platform.platform(), "cpu": platform.machine(),
            "logical_cpus": os.cpu_count(), "terminal": "xterm-256color PTY 80x24",
            "refresh_ms": 1000, "duration_seconds": round(elapsed, 2),
            "mode": "linux" if real else "demo",
            "first_sample_ms": round(first_sample_ms, 3),
            "first_frame_ms": round(first_frame_ms, 3), "help_response_ms": round(help_ms, 3),
            "cpu_percent_one_core": round(cpu_percent, 4),
            "rss_min_bytes": min(value for _, value in samples),
            "rss_max_bytes": max(value for _, value in samples),
            "rss_final_bytes": samples[-1][1],
            "rss_at_60_seconds_bytes": next((value for second, value in samples if second >= 60), None),
        }
        print(json.dumps(result, indent=2))
        assert result["rss_max_bytes"] < 30_000_000, "RSS target exceeded"
        assert first_frame_ms < 1000, "startup target exceeded"
        assert help_ms < 100, "response target exceeded"
        assert cpu_percent < 1, "idle CPU target exceeded"
    finally:
        if child is not None:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=5)
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=os.path.abspath)
    parser.add_argument("--seconds", type=int, default=600)
    parser.add_argument("--real", action="store_true", help="Measure LinuxProvider read-only mode")
    args = parser.parse_args()
    if args.seconds < 1:
        parser.error("--seconds must be positive")
    measure(args.binary, args.seconds, args.real)
