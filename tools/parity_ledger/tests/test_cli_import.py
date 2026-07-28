"""Tests for import CLI exits and canonical summaries."""

from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools.parity_ledger.cli import main
from tools.parity_ledger.jsonio import load_json_strict
from tools.parity_ledger.tests.corpus_fixture import make_repo


class ImportCliTests(unittest.TestCase):
    def test_import_success_and_missing_source_exit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            stdout = StringIO()
            with redirect_stdout(stdout):
                with mock.patch(
                    "tools.parity_ledger.cli.import_source_set",
                    side_effect=lambda path, name: __import__(
                        "tools.parity_ledger.corpus", fromlist=["import_source_set"]
                    ).import_source_set(path, name, derive_workspace_evidence=False),
                ):
                    self.assertEqual(main(["import", "--source-set", "bootstrap"], repo=repo), 0)
            summary = load_json_strict(stdout.getvalue())
            self.assertEqual(summary["obligations"], 277)
            missing = repo / "docs/plans/2026-05-29-core-engine-substrate-todo.md"
            missing.unlink()
            stderr = StringIO()
            with redirect_stderr(stderr):
                with mock.patch(
                    "tools.parity_ledger.cli.import_source_set",
                    side_effect=lambda path, name: __import__(
                        "tools.parity_ledger.corpus", fromlist=["import_source_set"]
                    ).import_source_set(path, name, derive_workspace_evidence=False),
                ):
                    self.assertEqual(main(["import", "--source-set", "bootstrap"], repo=repo), 11)
            error = load_json_strict(stderr.getvalue())
            self.assertEqual(error["exit_code"], 11)

    def test_unknown_source_set_is_argparse_exit_two(self) -> None:
        with self.assertRaises(SystemExit) as caught:
            main(["import", "--source-set", "unknown"])
        self.assertEqual(caught.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
