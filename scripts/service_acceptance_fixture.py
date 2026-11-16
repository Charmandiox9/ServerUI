"""Reviewable, unprivileged systemd USER fixtures; default action only prints a plan.

No sudo, polkit changes, system units, network configuration or existing-file
replacement. Prepare/cleanup require separate explicit invocation by the user.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

PREFIX = "serverui-acceptance"
UNITS = [PREFIX + suffix + ".service" for suffix in ("", "-fail", "-slow")]
TARGET = PREFIX + ".target"
FILES = {
    UNITS[0]: "[Unit]\nDescription=ServerUI disposable acceptance\n[Service]\nType=exec\nExecStart=/usr/bin/sleep infinity\n[Install]\nWantedBy=" + TARGET + "\n",
    UNITS[1]: "[Unit]\nDescription=ServerUI intentional failure\n[Service]\nType=oneshot\nExecStart=/usr/bin/false\n[Install]\nWantedBy=" + TARGET + "\n",
    UNITS[2]: "[Unit]\nDescription=ServerUI bounded slow start\n[Service]\nType=exec\nExecStartPre=/usr/bin/sleep 8\nExecStart=/usr/bin/sleep infinity\nTimeoutStartSec=15\n[Install]\nWantedBy=" + TARGET + "\n",
    TARGET: "[Unit]\nDescription=ServerUI isolated acceptance target\n",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def paths():
    uid = os.getuid()
    if uid == 0:
        raise RuntimeError("Run as a normal user; sudo/root is not supported")
    root = Path.home() / ".config/systemd/user"
    manifest = Path(f"/tmp/serverui-acceptance-{uid}.json")
    wants = root / (TARGET + ".wants")
    return uid, root, manifest, wants


def command(*args):
    # Explicit --user: never inherit a redirected SYSTEM bus for fixture setup.
    subprocess.run(["/usr/bin/systemctl", "--user", "--no-pager", *args], check=True, timeout=25)


def failed_units():
    result = subprocess.run(["/usr/bin/systemctl", "--user", "--no-pager", "--plain", "--no-legend", "list-units", "--failed", "--type=service"], capture_output=True, text=True, check=True, timeout=10)
    names = {line.split()[0] for line in result.stdout.splitlines() if line.split()}
    return [unit for unit in UNITS if unit in names]


def verified(root, manifest, wants, uid):
    if manifest.is_symlink() or manifest.stat().st_uid != uid:
        raise RuntimeError("Manifest ownership mismatch")
    record = json.loads(manifest.read_text())
    expected = {name: digest(text.encode()) for name, text in FILES.items()}
    if record != {"uid": uid, "root": str(root.resolve()), "files": expected}:
        raise RuntimeError("Manifest does not match the reviewed fixture")
    for name, checksum in expected.items():
        path = root / name
        if path.is_symlink() or path.stat().st_uid != uid or digest(path.read_bytes()) != checksum:
            raise RuntimeError("Fixture changed; refusing to alter " + name)
    if wants.is_symlink() or wants.stat().st_uid != uid:
        raise RuntimeError("Wants directory ownership mismatch")


def main(action):
    uid, root, manifest, wants = paths()
    if action == "plan":
        print(json.dumps({"scope": "systemd user manager only", "files": [str(root / n) for n in FILES],
                          "directory": str(wants), "manifest": str(manifest),
                          "prepare": "create new files, daemon-reload and load unit metadata; no service starts",
                          "cleanup": "verify hashes, stop/disable only these units, remove owned fixture files",
                          "limits": "journal records remain; no VM/snapshot or system-manager authorization test"}, indent=2))
        return
    runtime = Path(f"/run/user/{uid}/bus")
    if not runtime.is_socket():
        raise RuntimeError("No user systemd bus is available")
    if action == "prepare":
        if manifest.exists() or manifest.is_symlink() or wants.exists() or wants.is_symlink() or any((root / name).exists() or (root / name).is_symlink() for name in FILES):
            raise RuntimeError("Fixture path already exists; no files overwritten")
        root.mkdir(parents=True, exist_ok=True)
        created = []
        try:
            wants.mkdir(mode=0o700)
            for name, text in FILES.items():
                path = root / name
                with path.open("x", encoding="utf-8", newline="\n") as stream:
                    stream.write(text)
                created.append(path)
            record = {"uid": uid, "root": str(root.resolve()), "files": {n: digest(t.encode()) for n, t in FILES.items()}}
            with manifest.open("x", encoding="utf-8") as stream:
                json.dump(record, stream)
            manifest.chmod(0o600)
        except Exception:
            for path in created:
                path.unlink()
            if wants.is_dir() and not wants.is_symlink():
                wants.rmdir()
            raise
        command("daemon-reload")
        command("show", "--property=Id", "--property=LoadState", *UNITS)
        print("Prepared. No service started. Restore/cleanup with the cleanup action.")
        return
    verified(root, manifest, wants, uid)
    if action in ("lock-enable", "unlock-enable"):
        if any(wants.iterdir()):
            raise RuntimeError("Wants directory must be empty for the permission test")
        wants.chmod(0o500 if action == "lock-enable" else 0o700)
        return
    wants.chmod(0o700)
    for link in wants.iterdir():
        if link.name not in UNITS or not link.is_symlink() or link.resolve() != (root / link.name).resolve():
            raise RuntimeError("Unexpected file in fixture wants directory; cleanup stopped")
    command("stop", *UNITS, TARGET)
    command("disable", *UNITS)
    failed = failed_units()
    if failed:
        command("reset-failed", *failed)
    for name in FILES:
        (root / name).unlink()
    if wants.exists():
        wants.rmdir()
    manifest.unlink()
    command("daemon-reload")
    print("Removed the reviewed fixtures; existing services and journal untouched.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["plan", "prepare", "lock-enable", "unlock-enable", "cleanup"], default="plan", nargs="?")
    args = parser.parse_args()
    main(args.action)
