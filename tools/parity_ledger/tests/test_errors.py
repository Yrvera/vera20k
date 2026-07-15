"""Tests for stable ledger diagnostics."""

import unittest

from tools.parity_ledger.errors import Diagnostic, ExitCode, LedgerError


class DiagnosticTests(unittest.TestCase):
    def test_diagnostics_sort_independently_of_discovery_order(self) -> None:
        values = [
            Diagnostic("Z", record_id="b"),
            Diagnostic("A", record_id="c"),
            Diagnostic("A", record_id="a"),
        ]
        error = LedgerError(ExitCode.VALIDATION_FAILED, list(reversed(values)))
        self.assertEqual(error.diagnostics, tuple(sorted(values)))


if __name__ == "__main__":
    unittest.main()
