"""Explicitly authorized systemd USER acceptance; always cleans owned fixtures.

Requires service_acceptance_fixture.py prepare first. Refuses root, validates
fixture hashes, checks the user manager bus, and never targets existing services.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import termios
import time

import dbus

import service_acceptance_fixture as fixture
from service_smoke import Screen


def run(binary):
    uid, root, manifest, wants = fixture.paths()
    fixture.verified(root, manifest, wants, uid)
    env = dict(os.environ, TERM="xterm-256color", DBUS_SYSTEM_BUS_ADDRESS=f"unix:path=/run/user/{uid}/bus")
    def query(*args):
        return subprocess.run(["/usr/bin/systemctl", "--user", "--no-pager", *args], capture_output=True, text=True, timeout=10)
    user_group = query("show", "--property=ControlGroup", "--value").stdout.strip()
    redirected = subprocess.check_output(["/usr/bin/systemctl", "--system", "show", "--property=ControlGroup", "--value"], env=env, text=True, timeout=10).strip()
    if not user_group or user_group != redirected or f"user@{uid}.service" not in user_group:
        raise RuntimeError("Refusing to run: redirected manager is not this user's manager")
    # Keep inactive fixtures in list-units without starting any service. systemd
    # automatically releases these references when this bus connection closes.
    bus = dbus.bus.BusConnection(f"unix:path=/run/user/{uid}/bus")
    manager = dbus.Interface(bus.get_object("org.freedesktop.systemd1", "/org/freedesktop/systemd1"), "org.freedesktop.systemd1.Manager")
    for unit in fixture.UNITS:
        manager.RefUnit(unit)
    master, slave = pty.openpty()
    child = None
    cases = []
    report = {"scope": "real systemd user manager", "uid": uid, "cases": cases, "system_manager_tested": False}
    captured = bytearray()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        before = termios.tcgetattr(slave)
        child = subprocess.Popen([binary, "--ascii", "--no-color"], stdin=slave, stdout=slave, stderr=slave, start_new_session=True, env=env)
        screen = Screen(120,30)
        def wait_for(predicate, label, timeout=10):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if predicate(screen.text()): return
                ready, _, _ = select.select([master], [], [], .05)
                if ready:
                    data = os.read(master,65536)
                    captured.extend(data)
                    screen.feed(data)
                if child.poll() is not None: break
            raise AssertionError("Stage failed: " + label + "; terminal content withheld")
        def send(keys, text, label):
            os.write(master, keys)
            wait_for(lambda s: text in s,label)
        def select_unit(unit):
            if unit not in fixture.UNITS: raise RuntimeError("Unit is not allowlisted")
            os.write(master,("/\x15" + unit + "\r").encode())
            wait_for(lambda s: "/"+unit in s and "_ |" not in s,"select fixture")
        def propose(key,label,unit):
            send(key,label+"?","confirmation")
            assert unit in screen.text(), "Wrong unit in confirmation"
        def perform(key,label,unit,result="Comando completado"):
            propose(key,label,unit)
            send(b"y",result,label)
        def active(unit):
            return query("show","--property=ActiveState","--value",unit).stdout.strip()
        def main_pid(unit):
            return int(query("show","--property=MainPID","--value",unit).stdout.strip())
        normal, failing, slow = fixture.UNITS
        assert active(normal) == "inactive", "Fixture is not in its initial state"
        wait_for(lambda s: "RAM:" in s,"initial metrics")
        send(b"3"," cargados ","service list")
        select_unit(normal)
        propose(b"a","Iniciar",normal)
        send(b"n","a:iniciar","cancel")
        assert active(normal) == "inactive"
        send(b"h","Sin acciones ejecutadas","cancel history")
        send(b"\x1b","a:iniciar","return from history")
        cases.append("cancel_no_action")
        perform(b"a","Iniciar",normal)
        first_pid = main_pid(normal)
        assert active(normal) == "active" and first_pid > 0
        cases.append("start_active")
        perform(b"R","Reiniciar",normal)
        assert active(normal) == "active" and main_pid(normal) != first_pid
        cases.append("restart_new_pid")
        perform(b"d","Detener",normal)
        assert active(normal) == "inactive"
        cases.append("stop_inactive")
        fixture.main("lock-enable")
        try:
            perform(b"e","Habilitar al arranque",normal,"Comando rechazado")
            assert query("is-enabled",normal).stdout.strip() == "disabled"
            assert not any(wants.iterdir())
            cases.append("enable_write_denied")
        finally:
            fixture.main("unlock-enable")
        perform(b"e","Habilitar al arranque",normal)
        assert query("is-enabled",normal).stdout.strip() == "enabled"
        assert active(normal) == "inactive", "Enable unexpectedly started fixture"
        cases.append("enable_without_start")
        select_unit(failing)
        perform(b"a","Iniciar",failing,"Comando rechazado")
        assert query("show","--property=Result","--value",failing).stdout.strip() == "exit-code"
        cases.append("failed_start_controlled")
        select_unit(slow)
        propose(b"a","Iniciar",slow)
        send(b"y","Consultando...","slow request")
        send(b"?","Ayuda","help while worker busy")
        send(b"?","a:iniciar","close help")
        send(b"1","RAM:","dashboard while worker busy")
        send(b"3","a:iniciar","return to services")
        wait_for(lambda s: "Tiempo agotado" in s,"timeout result",timeout=8)
        deadline = time.monotonic()+10
        while active(slow) != "active" and time.monotonic() < deadline:
            time.sleep(.1)
        assert active(slow) == "active", "Job did not complete after client timeout"
        send(b"h","HISTORIAL DE ESTA SESION","action history")
        assert sum(slow in row for row in screen.text().splitlines()) == 1, "Unexpected repeated slow action"
        cases.append("timeout_ui_responsive_job_completed_once")
        os.write(master,b"q")
        deadline=time.monotonic()+5
        while child.poll() is None and time.monotonic() < deadline:
            ready,_,_=select.select([master],[],[],.05)
            if ready: captured.extend(os.read(master,65536))
        assert child.poll() == 0
        assert termios.tcgetattr(slave) == before
        assert b"\x1b[?1049l" in captured and b"\x1b[?25h" in captured
        report["terminal_restored"] = True
        report["result"] = "PASS"
    finally:
        if child is not None:
            if child.poll() is None: child.kill()
            child.wait(timeout=5)
        os.close(master); os.close(slave)
        fixture.main("cleanup")
        bus.close()
        assert not manifest.exists()
        assert all(not (root/name).exists() for name in fixture.FILES)
        report["fixtures_removed"] = True
        print(json.dumps(report))


if __name__ == "__main__":
    run(os.path.abspath(sys.argv[1]))
