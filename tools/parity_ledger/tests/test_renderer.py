"""Tests for deterministic report ordering, counts, and Markdown projection."""

from pathlib import Path
import tempfile
import unittest

from tools.parity_ledger.corpus import import_source_set, load_tracked_corpus, write_import
from tools.parity_ledger.model import ParityVerdict, RegressionState
from tools.parity_ledger.renderer import build_report, render_json, render_markdown
from tools.parity_ledger.schema import decode_ledger
from tools.parity_ledger.tests.corpus_fixture import make_repo


class RendererTests(unittest.TestCase):
    def test_report_is_deterministic_and_never_overclaims(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            write_import(repo, import_source_set(repo, "bootstrap", derive_workspace_evidence=False))
            corpus = load_tracked_corpus(repo)
            first = build_report(repo, corpus)
            second = build_report(repo, corpus)
            self.assertEqual(render_json(first), render_json(second))
            self.assertEqual(len(first.rows), 277)
            self.assertEqual(first.counts["system"], {"core": 32, "miner": 139, "scheduler": 17, "shell": 89})
            self.assertEqual(first.counts["parity_verdict"][ParityVerdict.VERIFIED.value], 0)
            self.assertEqual(first.counts["regression_state"][RegressionState.PASS.value], 0)
            self.assertEqual(first.counts["regression_state"][RegressionState.FAIL.value], 0)
            decode_ledger(first.to_document())
            markdown = render_markdown(first).decode("utf-8")
            self.assertIn("## Unassigned Obligations", markdown)
            self.assertIn("`miner:M32`", markdown)
            self.assertNotIn("%", markdown)
            self.assertNotIn(str(repo), markdown)


if __name__ == "__main__":
    unittest.main()
