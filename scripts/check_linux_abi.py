"""Check the linked GNU libc requirements of a Linux release with readelf."""

import argparse
import os
import re
import subprocess


def check(binary, maximum):
    info = subprocess.check_output(["readelf", "--version-info", binary], text=True, env=dict(os.environ, LC_ALL="C"))
    required = {tuple(map(int, version.split("."))) for version in re.findall(r"Name: GLIBC_([0-9.]+)", info)}
    if not required:
        raise SystemExit("No GNU libc version requirements found; inspect this binary manually")
    highest = max(required)
    allowed = tuple(map(int, maximum.split(".")))
    print(f"Required glibc: {'.'.join(map(str, highest))}; supported baseline: {maximum}")
    if highest > allowed:
        raise SystemExit("Binary requires a newer glibc than the supported baseline")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("--max-glibc", default="2.35")
    args = parser.parse_args()
    check(args.binary, args.max_glibc)
