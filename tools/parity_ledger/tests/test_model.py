"""Tests for deterministic v1 model projection."""

import unittest

from tools.parity_ledger.model import (
    Assignment,
    AssignmentMention,
    AssignmentRole,
    SourceRef,
    Tracking,
)


class ModelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = SourceRef(
            "docs/plans/roadmap.md",
            "W1",
            "roadmap",
            "0" * 64,
            Tracking.IGNORED_LOCAL,
            "roadmap-adapter",
            1,
        )

    def test_related_assignments_are_sorted(self) -> None:
        assignment = Assignment(
            None,
            (
                AssignmentMention("W9", AssignmentRole.RESEARCH_GATE, self.source),
                AssignmentMention("W2", AssignmentRole.PARENT, self.source),
            ),
        )
        self.assertEqual(
            [item["workstream"] for item in assignment.to_document()["related"]],
            ["W2", "W9"],
        )


if __name__ == "__main__":
    unittest.main()
