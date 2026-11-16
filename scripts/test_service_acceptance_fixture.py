"""Fixture safety tests: temporary files and mocked systemctl only."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import service_acceptance_fixture as fixture


class FixtureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="serverui-fixture-test-")
        self.base = Path(self.temp.name).resolve()
        self.assertEqual(self.base.parent, Path(tempfile.gettempdir()).resolve())
        self.root = self.base / "units"
        self.manifest = self.base / "manifest.json"
        self.wants = self.root / (fixture.TARGET + ".wants")
        uid = os.getuid() if hasattr(os, "getuid") else 0
        self.paths = patch.object(fixture, "paths", return_value=(uid,self.root,self.manifest,self.wants))
        self.commands = patch.object(fixture,"command")
        self.socket = patch.object(Path,"is_socket",return_value=True)
        self.failed = patch.object(fixture,"failed_units",return_value=[])
        self.failed.start()
        self.paths.start(); self.command = self.commands.start(); self.socket.start()

    def tearDown(self):
        self.wants.chmod(0o700) if self.wants.exists() else None
        self.socket.stop(); self.commands.stop(); self.paths.stop()
        self.failed.stop()
        self.temp.cleanup()

    def test_plan_does_not_create_files_or_run_commands(self):
        fixture.main("plan")
        self.assertFalse(self.root.exists())
        self.command.assert_not_called()

    def test_prepare_never_starts_services_and_cleanup_is_allowlisted(self):
        fixture.main("prepare")
        calls = [call.args[0] for call in self.command.call_args_list]
        self.assertEqual(calls,["daemon-reload","show"])
        self.assertEqual(len(list(self.root.glob("*.service"))),3)
        fixture.main("cleanup")
        self.assertFalse(self.manifest.exists())
        self.assertFalse(any(self.root.iterdir()))
        stop = next(c for c in self.command.call_args_list if c.args[0] == "stop")
        self.assertEqual(stop.args,("stop",*fixture.UNITS,fixture.TARGET))
        self.assertFalse(any(c.args[0] == "reset-failed" for c in self.command.call_args_list))

    def test_cleanup_resets_only_failed_allowlisted_units(self):
        fixture.main("prepare")
        self.command.reset_mock()
        with patch.object(fixture,"failed_units",return_value=[fixture.UNITS[1]]):
            fixture.main("cleanup")
        resets = [c.args for c in self.command.call_args_list if c.args[0] == "reset-failed"]
        self.assertEqual(resets,[("reset-failed",fixture.UNITS[1])])

    def test_existing_files_are_never_overwritten(self):
        self.root.mkdir()
        path = self.root / fixture.UNITS[0]
        path.write_text("existing")
        with self.assertRaises(RuntimeError): fixture.main("prepare")
        self.assertEqual(path.read_text(),"existing")
        self.command.assert_not_called()

    def test_modified_fixture_prevents_cleanup_and_stops(self):
        fixture.main("prepare")
        self.command.reset_mock()
        (self.root / fixture.UNITS[0]).write_text("changed")
        with self.assertRaises(RuntimeError): fixture.main("cleanup")
        self.command.assert_not_called()
        self.assertTrue(self.manifest.exists())

    def test_unexpected_file_prevents_cleanup(self):
        fixture.main("prepare")
        self.command.reset_mock()
        (self.wants / "unrelated").write_text("retain")
        with self.assertRaises(RuntimeError): fixture.main("cleanup")
        self.command.assert_not_called()
        self.assertTrue((self.wants / "unrelated").exists())

    def test_enable_permission_controls_only_empty_owned_directory(self):
        fixture.main("prepare")
        fixture.main("lock-enable")
        fixture.main("unlock-enable")
        (self.wants / "unrelated").write_text("retain")
        with self.assertRaises(RuntimeError): fixture.main("lock-enable")


if __name__ == "__main__":
    unittest.main()
