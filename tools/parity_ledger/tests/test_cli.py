"""Integration tests for check modes and target-contained rendering."""

from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from tools.parity_ledger.cli import main
from tools.parity_ledger.corpus import import_source_set, write_import
from tools.parity_ledger.jsonio import load_json_strict
from tools.parity_ledger.source_sets import BOOTSTRAP_SOURCES
from tools.parity_ledger.tests.corpus_fixture import make_repo


class CliTests(unittest.TestCase):
    def _prepared(self, root: Path) -> Path:
        repo = make_repo(root)
        write_import(repo, import_source_set(repo, "bootstrap", derive_workspace_evidence=False))
        return repo

    def test_check_require_render_and_ci_without_ignored_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self._prepared(Path(directory))
            stdout = StringIO()
            with mock.patch(
                "tools.parity_ledger.cli.import_source_set",
                side_effect=lambda path, name: __import__(
                    "tools.parity_ledger.corpus", fromlist=["import_source_set"]
                ).import_source_set(path, name, derive_workspace_evidence=False),
            ):
                with redirect_stdout(stdout):
                    self.assertEqual(main(["check", "--require-sources"], repo=repo), 0)
            self.assertEqual(load_json_strict(stdout.getvalue())["counts"]["total"], 277)
            stdout = StringIO()
            with redirect_stdout(stdout):
                self.assertEqual(main(["render", "--output", "target/parity-ledger"], repo=repo), 0)
            self.assertTrue((repo / "target/parity-ledger/ledger.json").is_file())
            self.assertTrue((repo / "target/parity-ledger/summary.md").is_file())
            for config in BOOTSTRAP_SOURCES:
                (repo / Path(*config.path.split("/"))).unlink()
            stdout = StringIO()
            with redirect_stdout(stdout):
                self.assertEqual(main(["check", "--ci"], repo=repo), 0)
            self.assertEqual(load_json_strict(stdout.getvalue())["counts"]["source_state"]["UNAVAILABLE"], 277)

    def test_render_rejects_tracked_output_and_check_modes_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self._prepared(Path(directory))
            stderr = StringIO()
            with redirect_stderr(stderr):
                self.assertEqual(main(["render", "--output", "parity"], repo=repo), 2)
            self.assertFalse((repo / "parity/ledger.json").exists())
            stderr = StringIO()
            with redirect_stderr(stderr):
                self.assertEqual(main(["render", "--output", "target/../parity"], repo=repo), 2)
            with self.assertRaises(SystemExit) as caught:
                main(["check", "--ci", "--require-sources"], repo=repo)
            self.assertEqual(caught.exception.code, 2)

    def test_require_sources_rejects_stale_normalized_import(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self._prepared(Path(directory))
            stderr = StringIO()
            with mock.patch(
                "tools.parity_ledger.cli.import_source_set",
                return_value=SimpleNamespace(digest="f" * 64),
            ):
                with redirect_stderr(stderr):
                    self.assertEqual(main(["check", "--require-sources"], repo=repo), 11)
            error = load_json_strict(stderr.getvalue())
            self.assertEqual(error["diagnostics"][0]["code"], "SOURCE_STALE")

    def test_render_rejects_symlink_escape_from_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self._prepared(Path(directory))
            (repo / "target").mkdir()
            link = repo / "target/link"
            try:
                link.symlink_to(repo / "parity", target_is_directory=True)
            except OSError as exc:
                self.skipTest(f"directory symlink unavailable: {exc}")
            stderr = StringIO()
            with redirect_stderr(stderr):
                self.assertEqual(main(["render", "--output", "target/link"], repo=repo), 2)
            self.assertFalse((repo / "parity/ledger.json").exists())


if __name__ == "__main__":
    unittest.main()
