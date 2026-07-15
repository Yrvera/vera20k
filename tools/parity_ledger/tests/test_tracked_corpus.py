"""CI-safe assertions over only the tracked bootstrap corpus."""

from pathlib import Path
import unittest

from tools.parity_ledger.corpus import load_tracked_corpus
from tools.parity_ledger.jsonio import canonical_json_bytes, load_json_strict
from tools.parity_ledger.model import RegressionState
from tools.parity_ledger.renderer import build_report, render_markdown
from tools.parity_ledger.workspace import find_repo_root


class TrackedCorpusTests(unittest.TestCase):
    def test_bootstrap_truth_and_declaration_ceiling(self) -> None:
        repo = find_repo_root(Path(__file__))
        corpus = load_tracked_corpus(repo)
        self.assertEqual(len(corpus.obligation_set.obligations), 277)
        counts = {}
        for obligation in corpus.obligation_set.obligations:
            counts[obligation.system] = counts.get(obligation.system, 0) + 1
        self.assertEqual(counts, {"core": 32, "miner": 139, "scheduler": 17, "shell": 89})
        unassigned = {
            item.id for item in corpus.obligation_set.obligations if item.assignment.primary is None
        }
        self.assertEqual(
            unassigned,
            {"miner:L7", "miner:L34", "miner:L35", "miner:L43", "miner:M32", "shell:H1", "shell:H19"},
        )
        self.assertEqual(
            {item.source_id for item in corpus.obligation_set.dispositions},
            {"shell:L3", "shell:L25", "shell:L28"},
        )
        report = build_report(repo, corpus, source_mode="ci")
        self.assertEqual(report.counts["parity_verdict"]["VERIFIED"], 0)
        self.assertEqual(report.counts["regression_state"][RegressionState.PASS.value], 0)
        self.assertEqual(report.counts["regression_state"][RegressionState.FAIL.value], 0)
        self.assertNotIn("%", render_markdown(report).decode("utf-8"))

    def test_tracked_json_is_canonical(self) -> None:
        repo = find_repo_root(Path(__file__))
        for relative in (
            "parity/sources/bootstrap.json",
            "parity/obligations/bootstrap.json",
            "parity/evidence/bootstrap.json",
        ):
            raw = (repo / relative).read_bytes()
            self.assertEqual(raw, canonical_json_bytes(load_json_strict(raw)), relative)


if __name__ == "__main__":
    unittest.main()
