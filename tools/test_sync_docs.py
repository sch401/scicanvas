import contextlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import sync_docs


class SyncDocsTests(unittest.TestCase):
    @contextlib.contextmanager
    def checkout(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            for source, _ in sync_docs.COPIES:
                path = root / source
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(f"原始文档：{source}\r\n".encode())
            with mock.patch.object(sync_docs, "ROOT", root), mock.patch.object(sync_docs.subprocess, "run") as run:
                yield root, Path(temporary) / "desktop", run

    def test_default_reuses_generator_and_keeps_export_opt_in(self):
        with self.checkout() as (root, desktop, run):
            sync_docs.sync()
            run.assert_called_once_with(
                [sys.executable, str(root / "docs/parity/build_parity.py")], cwd=root, check=True,
            )
            self.assertFalse(desktop.exists())

    def test_export_preserves_bytes_aliases_and_unrelated_files(self):
        with self.checkout() as (root, desktop, run):
            desktop.mkdir()
            (desktop / "personal.md").write_text("保留", encoding="utf-8")
            sync_docs.sync(desktop_dir=desktop)
            for source, name in sync_docs.COPIES:
                self.assertEqual((desktop / name).read_bytes(), (root / source).read_bytes())
            self.assertEqual((desktop / "personal.md").read_text(encoding="utf-8"), "保留")
            run.assert_called_once()

    def test_check_reports_missing_and_stale_exports_without_writing(self):
        with self.checkout() as (root, desktop, run):
            with self.assertRaises(SystemExit):
                sync_docs.sync(check=True, desktop_dir=desktop)
            self.assertFalse(desktop.exists())
            run.assert_called_once_with(
                [sys.executable, str(root / "docs/parity/build_parity.py"), "--check"], cwd=root, check=True,
            )
            sync_docs.sync(desktop_dir=desktop)
            sync_docs.sync(check=True, desktop_dir=desktop)
            stale = desktop / "SciCanvas 对照清单.html"
            stale.write_bytes(b"older copy")
            before = {path.name: path.read_bytes() for path in desktop.iterdir()}
            with self.assertRaises(SystemExit):
                sync_docs.sync(check=True, desktop_dir=desktop)
            self.assertEqual(before, {path.name: path.read_bytes() for path in desktop.iterdir()})

    def test_generator_failure_stops_before_export(self):
        with self.checkout() as (_, desktop, run):
            run.side_effect = subprocess.CalledProcessError(1, "build_parity")
            with self.assertRaises(subprocess.CalledProcessError):
                sync_docs.sync(desktop_dir=desktop)
            self.assertFalse(desktop.exists())

    def test_check_entry_works_from_another_working_directory(self):
        script = sync_docs.ROOT / "tools/sync_docs.py"
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run(
                [sys.executable, str(script), "--check"], cwd=temporary, capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
