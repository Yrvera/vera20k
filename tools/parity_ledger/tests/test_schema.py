"""Tests for strict persisted-document contracts."""

from copy import deepcopy
from pathlib import Path
import re
import unittest

from tools.parity_ledger import IMPORTER_NAME, IMPORTER_VERSION
from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.jsonio import canonical_json_bytes, load_json_strict
from tools.parity_ledger.model import (
    Assignment,
    AssignmentState,
    ImplementationState,
    LedgerReport,
    Obligation,
    ObligationKind,
    ObligationSetDocument,
    OracleState,
    ParityVerdict,
    QueueState,
    RegressionState,
    SourceClaims,
    SourceFileLock,
    SourceLockDocument,
    SourceRef,
    SourceRole,
    SourceState,
    Tracking,
)
from tools.parity_ledger.schema import (
    assert_schema_document_parity,
    decode_obligation_set,
    decode_evidence_set,
    decode_ledger,
    decode_source_lock,
)


class SchemaTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = SourceRef(
            "docs/plans/source.md",
            "L7",
            "source",
            "1" * 64,
            Tracking.IGNORED_LOCAL,
            "adapter",
            1,
        )
        self.obligation = Obligation(
            "miner:L7",
            "miner",
            ObligationKind.PARITY_GAP,
            "A precise gap",
            self.source,
            SourceClaims(),
            Assignment(None),
        )
        self.obligations = ObligationSetDocument(
            "bootstrap",
            "2" * 64,
            (self.obligation,),
            (),
            (),
        ).to_document()

    def test_valid_documents_round_trip(self) -> None:
        decoded = decode_obligation_set(self.obligations)
        self.assertEqual(decoded.to_document(), self.obligations)
        lock = SourceLockDocument(
            "bootstrap",
            IMPORTER_NAME,
            IMPORTER_VERSION,
            "2" * 64,
            (
                SourceFileLock(
                    "source",
                    "miner",
                    SourceRole.INVENTORY,
                    "docs/plans/source.md",
                    "1" * 64,
                    Tracking.IGNORED_LOCAL,
                    "adapter",
                    1,
                ),
            ),
        ).to_document()
        self.assertEqual(decode_source_lock(lock).to_document(), lock)

    def test_rejects_unknown_forbidden_and_namespace_fields(self) -> None:
        unknown = deepcopy(self.obligations)
        unknown["obligations"][0]["surprise"] = True
        forbidden = deepcopy(self.obligations)
        forbidden["obligations"][0]["source_claims"]["Done"] = False
        namespace = deepcopy(self.obligations)
        namespace["obligations"][0]["system"] = "shell"
        for value in (unknown, forbidden, namespace):
            with self.subTest(value=value), self.assertRaises(LedgerError):
                decode_obligation_set(value)

    def test_rejects_bool_integer_upper_hash_and_unsorted_array(self) -> None:
        bool_version = deepcopy(self.obligations)
        bool_version["schema_version"] = True
        upper_hash = deepcopy(self.obligations)
        upper_hash["obligations"][0]["source"]["sha256"] = "A" * 64
        unsorted = deepcopy(self.obligations)
        second = deepcopy(unsorted["obligations"][0])
        second["id"] = "miner:A1"
        second["source"]["local_id"] = "A1"
        unsorted["obligations"].append(second)
        for value in (bool_version, upper_hash, unsorted):
            with self.subTest(value=value), self.assertRaises(LedgerError):
                decode_obligation_set(value)

    def test_rejects_non_namespaced_disposition_source_id(self) -> None:
        malformed = deepcopy(self.obligations)
        malformed["dispositions"] = [
            {
                "kind": "retired_non_gap",
                "source": self.source.to_document(),
                "source_id": "L7",
                "targets": [],
            }
        ]
        with self.assertRaises(LedgerError):
            decode_obligation_set(malformed)

    def test_portable_schemas_match_runtime_top_level(self) -> None:
        assert_schema_document_parity(Path("parity/schemas"))
        for path in Path("parity/schemas").glob("*.json"):
            raw = path.read_bytes()
            self.assertEqual(raw, canonical_json_bytes(load_json_strict(raw)), path.name)

    def test_portable_path_pattern_rejects_runtime_unsafe_paths(self) -> None:
        schema = load_json_strict(Path("parity/schemas/source-lock.v1.schema.json").read_bytes())
        pattern = schema["$defs"]["source"]["properties"]["path"]["pattern"]
        self.assertIsNotNone(re.fullmatch(pattern, "src/sim/tick.rs"))
        for unsafe in (
            "a/",
            "NUL",
            "docs/a.",
            "docs/a ",
            "docs/\x01x",
            "x" * 256,
        ):
            with self.subTest(path=unsafe):
                self.assertIsNone(re.fullmatch(pattern, unsafe))

    def test_evidence_kind_rejects_irrelevant_or_mismatched_fields(self) -> None:
        document = {
            "corpus_digest": "2" * 64,
            "evidence": [
                {
                    "artifact": None,
                    "check": {"commit": "a" * 40, "type": "git_ancestor"},
                    "coverage": None,
                    "id": "evidence:git:test",
                    "kind": "git_scoped",
                    "obligations": ["miner:L7"],
                    "provenance": None,
                    "schema_version": 1,
                }
            ],
            "schema_version": 1,
            "source_set": "bootstrap",
        }
        decode_evidence_set(document)
        irrelevant = deepcopy(document)
        irrelevant["evidence"][0]["coverage"] = {"domain": "all", "mode": "exhaustive"}
        mismatched = deepcopy(document)
        mismatched["evidence"][0]["kind"] = "implementation_anchor"
        unscoped_candidate = deepcopy(document)
        unscoped_candidate["evidence"][0]["kind"] = "git_candidate"
        for value in (irrelevant, mismatched, unscoped_candidate):
            with self.subTest(value=value), self.assertRaises(LedgerError):
                decode_evidence_set(value)

    def test_ledger_coverage_state_is_closed(self) -> None:
        counts = {
            "assignment_state": {item.value: 0 for item in AssignmentState},
            "implementation_state": {item.value: 0 for item in ImplementationState},
            "oracle_state": {item.value: 0 for item in OracleState},
            "parity_verdict": {item.value: 0 for item in ParityVerdict},
            "queue_state": {item.value: 0 for item in QueueState},
            "regression_state": {item.value: 0 for item in RegressionState},
            "source_state": {item.value: 0 for item in SourceState},
            "system": {},
            "total": 0,
        }
        valid = LedgerReport("2" * 64, "BOOTSTRAP_PROVISIONAL", counts, (), (), ()).to_document()
        decode_ledger(valid)
        invalid = deepcopy(valid)
        invalid["coverage_state"] = "CERTIFIED"
        with self.assertRaises(LedgerError):
            decode_ledger(invalid)
        false_counts = deepcopy(valid)
        false_counts["counts"]["parity_verdict"]["VERIFIED"] = 999
        with self.assertRaises(LedgerError):
            decode_ledger(false_counts)


if __name__ == "__main__":
    unittest.main()
