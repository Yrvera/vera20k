"""Table-driven tests for conservative v1 parity and queue precedence."""

from itertools import product
import unittest

from tools.parity_ledger.model import (
    Assignment,
    AssignmentState,
    ImplementationState,
    Obligation,
    ObligationKind,
    OracleState,
    ParityVerdict,
    QueueState,
    RegressionState,
    SourceClaims,
    SourceRef,
    SourceState,
    Tracking,
)
from tools.parity_ledger.reducer import RowFacts, derive_parity, derive_queue


SOURCE = SourceRef("docs/a.md", "L1", "source", "0" * 64, Tracking.TRACKED, "adapter", 1)


def obligation(kind: ObligationKind = ObligationKind.PARITY_GAP) -> Obligation:
    return Obligation("miner:L1", "miner", kind, "gap", SOURCE, SourceClaims(), Assignment(None))


class ReducerTests(unittest.TestCase):
    def test_gap_drift_and_staleness_precedence(self) -> None:
        current = RowFacts(
            SourceState.CURRENT,
            AssignmentState.ASSIGNED,
            ImplementationState.NONE,
            RegressionState.NONE,
            OracleState.NONE,
        )
        self.assertEqual(derive_parity(obligation(), current), ParityVerdict.DRIFT)
        self.assertEqual(
            derive_parity(obligation(), RowFacts(**{**current.__dict__, "source_state": SourceState.STALE})),
            ParityVerdict.UNCHECKED,
        )
        self.assertEqual(
            derive_parity(obligation(), RowFacts(**{**current.__dict__, "source_state": SourceState.UNAVAILABLE})),
            ParityVerdict.DRIFT,
        )

    def test_queue_precedence(self) -> None:
        base = RowFacts(
            SourceState.CURRENT,
            AssignmentState.ASSIGNED,
            ImplementationState.LANDED,
            RegressionState.DECLARED,
            OracleState.NONE,
        )
        self.assertEqual(derive_queue(obligation(), base, ParityVerdict.UNVERIFIED), QueueState.NEEDS_REGRESSION)
        self.assertEqual(
            derive_queue(obligation(), RowFacts(**{**base.__dict__, "assignment_state": AssignmentState.UNASSIGNED}), ParityVerdict.UNVERIFIED),
            QueueState.NEEDS_ASSIGNMENT,
        )
        self.assertEqual(
            derive_queue(obligation(), RowFacts(**{**base.__dict__, "research_required": True}), ParityVerdict.UNVERIFIED),
            QueueState.NEEDS_RESEARCH,
        )
        passed = RowFacts(**{**base.__dict__, "regression_state": RegressionState.PASS})
        self.assertEqual(derive_queue(obligation(), passed, ParityVerdict.UNVERIFIED), QueueState.NEEDS_ORACLE)

    def test_v1_declaration_space_never_produces_reserved_results(self) -> None:
        for source, assigned, implementation, declared, oracle in product(
            SourceState,
            (AssignmentState.ASSIGNED, AssignmentState.UNASSIGNED),
            ImplementationState,
            (RegressionState.NONE, RegressionState.DECLARED),
            (OracleState.NONE, OracleState.INCOMPLETE, OracleState.SAMPLED),
        ):
            facts = RowFacts(source, assigned, implementation, declared, oracle, oracle_attempted=oracle is not OracleState.NONE)
            parity = derive_parity(obligation(), facts)
            queue = derive_queue(obligation(), facts, parity)
            self.assertNotEqual(parity, ParityVerdict.VERIFIED)
            self.assertNotEqual(facts.regression_state, RegressionState.PASS)
            self.assertNotEqual(facts.regression_state, RegressionState.FAIL)
            self.assertNotEqual(facts.oracle_state, OracleState.EXHAUSTIVE)
            self.assertIsInstance(queue, QueueState)

    def test_core_obligation_is_not_relabelled_as_confirmed_gap(self) -> None:
        facts = RowFacts(
            SourceState.CURRENT,
            AssignmentState.ASSIGNED,
            ImplementationState.NONE,
            RegressionState.NONE,
            OracleState.NONE,
        )
        self.assertEqual(derive_parity(obligation(ObligationKind.CORE_OBLIGATION), facts), ParityVerdict.UNCHECKED)


if __name__ == "__main__":
    unittest.main()
