"""Regression tests for portable release archives/checksum manifests."""

import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

from package_release import package


class PackageReleaseTests(unittest.TestCase):
    def test_linux_manifest_has_posix_newlines_and_valid_checksum(self):
        # Run on Windows as well: its default text translation caused a literal
        # CR to become part of the filename when sha256sum read the manifest.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            binary = root / "serverui"
            binary.write_bytes(b"fixture binary, not executed")
            output = root / "packages"
            package(binary, "x86_64-unknown-linux-gnu", output)
            archive, = output.glob("*.tar.gz")
            manifest = archive.with_name(archive.name + ".sha256")
            data = manifest.read_bytes()
            self.assertNotIn(b"\r", data, "GNU sha256sum treats CR as part of the filename")
            digest, name = data.decode("ascii").rstrip("\n").split("  ", 1)
            self.assertEqual(name, archive.name)
            self.assertEqual(digest, hashlib.sha256(archive.read_bytes()).hexdigest())
            if shutil.which("sha256sum"):
                subprocess.run(["sha256sum", "-c", manifest.name], cwd=output, check=True)
            with tarfile.open(archive) as bundle:
                self.assertEqual(bundle.getmember("serverui").mode, 0o755)
                self.assertEqual(bundle.extractfile("serverui").read(), binary.read_bytes())


if __name__ == "__main__":
    unittest.main()
