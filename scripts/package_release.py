"""Package a prebuilt release binary and documentation using standard Python.

CI builds and tests before invoking this script. Archives carry a UTC epoch
timestamp and fixed file modes so packaging the same inputs is deterministic.
The workspace manifest is the only source of the release version.
"""

import argparse
import gzip
import hashlib
import io
from pathlib import Path
import tarfile
import tomllib
import zipfile


def package(binary, target, output):
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    windows = target == "x86_64-pc-windows-msvc"
    files = [
        ("serverui.exe" if windows else "serverui", binary.read_bytes(), 0o755),
        ("README.md", (root / "README.md").read_bytes(), 0o644),
        ("CHANGELOG.md", (root / "CHANGELOG.md").read_bytes(), 0o644),
    ]
    install_guide = root / "docs" / "INSTALL_UBUNTU.md"
    if not windows and install_guide.is_file():
        files.append(("docs/INSTALL_UBUNTU.md", install_guide.read_bytes(), 0o644))
    output.mkdir(parents=True, exist_ok=True)
    suffix = ".zip" if windows else ".tar.gz"
    archive = output / f"serverui-v{version}-{target}{suffix}"
    if windows:
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
            for name, data, mode in files:
                entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                entry.create_system = 3
                entry.external_attr = (0o100000 | mode) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                bundle.writestr(entry, data)
    else:
        with archive.open("wb") as stream:
            with gzip.GzipFile(filename="", mode="wb", fileobj=stream, mtime=0, compresslevel=9) as zipped:
                with tarfile.open(fileobj=zipped, mode="w") as bundle:
                    for name, data, mode in files:
                        entry = tarfile.TarInfo(name)
                        entry.size = len(data)
                        entry.mode = mode
                        entry.mtime = 0
                        bundle.addfile(entry, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    checksum = output / f"{archive.name}.sha256"
    # Windows' default CRLF translation makes GNU sha256sum include CR in
    # the filename. Release manifests must be byte-for-byte POSIX text.
    checksum.write_text(f"{digest}  {archive.name}\n", encoding="ascii", newline="\n")
    print(f"{digest}  {archive.name}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--target", choices=["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"], required=True)
    parser.add_argument("--output", type=Path, default=Path("target/release-assets"))
    arguments = parser.parse_args()
    package(arguments.binary, arguments.target, arguments.output)
