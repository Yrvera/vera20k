"""Tests for graph target validation and canonical dependency cycles."""

from dataclasses import replace
import unittest

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.graph import validate_graph
from tools.parity_ledger.model import (
    Assignment,
    Obligation,
    ObligationKind,
    SourceClaims,
    SourceRef,
    Tracking,
)


SOURCE = SourceRef("docs/a.md", "A", "source", "0" * 64, Tracking.TRACKED, "adapter", 1)


def obligation(identifier: str, dependencies: tuple[str, ...] = ()) -> Obligation:
    return Obligation(
        identifier,
        identifier.split(":", 1)[0],
        ObligationKind.PARITY_GAP,
        identifier,
        SOURCE,
        SourceClaims(),
        Assignment(None),
        dependencies=dependencies,
    )


class GraphTests(unittest.TestCase):
    def test_valid_graph(self) -> None:
        validate_graph((obligation("core:a"), obligation("core:b", ("core:a",))), ())

    def test_unresolved_and_cycle_are_stable(self) -> None:
        with self.assertRaises(LedgerError) as unresolved:
            validate_graph((obligation("core:a", ("core:missing",)),), ())
        self.assertEqual(unresolved.exception.diagnostics[0].code, "UNRESOLVED_DEPENDENCY")
        cycle = (
            obligation("core:c", ("core:a",)),
            obligation("core:a", ("core:b",)),
            obligation("core:b", ("core:c",)),
        )
        with self.assertRaises(LedgerError) as caught:
            validate_graph(cycle, ())
        self.assertEqual(caught.exception.diagnostics[0].message, "core:a -> core:b -> core:c -> core:a")


if __name__ == "__main__":
    unittest.main()
