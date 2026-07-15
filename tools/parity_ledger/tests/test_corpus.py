"""Tests for corpus digests, source-lock commit ordering, and deterministic writes."""

from dataclasses import replace
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools.parity_ledger.corpus import (
    import_source_set,
    load_tracked_corpus,
    validate_cross_records,
    write_import,
)
from tools.parity_ledger.errors import Diagnostic, ExitCode, FailureCode, LedgerError
from tools.parity_ledger.jsonio import StagedWrite, canonical_json_bytes
from tools.parity_ledger.model import (
    EvidenceDeclaration,
    EvidenceKind,
    Relation,
    RelationKind,
    TestDeclaredCheck,
)
from tools.parity_ledger.tests.corpus_fixture import make_repo


def _generated_bytes(repo: Path) -> dict[str, bytes]:
    return {
        path.relative_to(repo).as_posix(): path.read_bytes()
        for path in sorted((repo / "parity").rglob("*.json"))
    }


class CorpusTests(unittest.TestCase):
    def test_import_is_byte_deterministic_and_loads(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            first = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            write_import(repo, first)
            before = _generated_bytes(repo)
            second = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            write_import(repo, second)
            self.assertEqual(before, _generated_bytes(repo))
            corpus = load_tracked_corpus(repo)
            self.assertEqual(len(corpus.obligation_set.obligations), 277)
            self.assertEqual(corpus.source_lock.corpus_digest, first.digest)

    def test_raw_source_change_changes_lock_and_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            first = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            core = repo / "docs/plans/2026-05-29-core-engine-substrate-todo.md"
            core.write_bytes(core.read_bytes().replace(b"- [ ] One alpha", b"- [x] One alpha"))
            second = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            self.assertNotEqual(first.digest, second.digest)
            first_lock = next(item for item in first.sources if item.source_id == "core-todo")
            second_lock = next(item for item in second.sources if item.source_id == "core-todo")
            self.assertNotEqual(first_lock.sha256, second_lock.sha256)
            self.assertEqual(
                [item.id for item in first.obligations if item.system == "core"],
                [item.id for item in second.obligations if item.system == "core"],
            )

    def test_interrupted_source_lock_commit_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            write_import(repo, import_source_set(repo, "bootstrap", derive_workspace_evidence=False))
            core = repo / "docs/plans/2026-05-29-core-engine-substrate-todo.md"
            core.write_bytes(core.read_bytes().replace(b"- [ ] One alpha", b"- [x] One alpha"))
            changed = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            original_commit = StagedWrite.commit
            calls = 0

            def fail_third(staged: StagedWrite) -> None:
                nonlocal calls
                calls += 1
                if calls == 3:
                    raise LedgerError(
                        ExitCode.WORKSPACE_FAILED,
                        [Diagnostic(FailureCode.OUTPUT_IO_FAILED.value, message="injected", fatal=True)],
                    )
                original_commit(staged)

            with mock.patch.object(StagedWrite, "commit", fail_third):
                with self.assertRaises(LedgerError):
                    write_import(repo, changed)
            with self.assertRaises(LedgerError) as caught:
                load_tracked_corpus(repo)
            self.assertEqual(caught.exception.diagnostics[0].code, FailureCode.CORPUS_DIGEST_MISMATCH.value)

    def test_cross_validation_rejects_provenance_and_metadata_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            bundle = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            disposition = bundle.dispositions[0]
            bad_disposition = replace(
                disposition,
                source=replace(disposition.source, sha256="f" * 64),
            )
            bad_lock = replace(bundle.sources[0], system="forged")
            obligation = bundle.obligations[0]
            bad_obligation = replace(
                obligation,
                source=replace(obligation.source, local_id="forged"),
            )
            variants = (
                replace(bundle, dispositions=(bad_disposition, *bundle.dispositions[1:])),
                replace(bundle, sources=(bad_lock, *bundle.sources[1:])),
                replace(bundle, diagnostics=(*bundle.diagnostics, Diagnostic("FORGED", fatal=True))),
                replace(bundle, obligations=(bad_obligation, *bundle.obligations[1:])),
            )
            for variant in variants:
                with self.subTest(variant=variant):
                    self.assertTrue(any(item.fatal for item in validate_cross_records(variant)))

    def test_cross_validation_rejects_cross_system_assignment_and_relation_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            bundle = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            miner = next(
                item
                for item in bundle.obligations
                if item.system == "miner" and item.assignment.primary is not None
            )
            shell = next(
                item
                for item in bundle.obligations
                if item.system == "shell" and item.assignment.primary is not None
            )
            assert miner.assignment.primary is not None
            assert shell.assignment.primary is not None

            cross_system_assignment = replace(
                miner,
                assignment=replace(
                    miner.assignment,
                    primary=replace(
                        miner.assignment.primary,
                        source=shell.assignment.primary.source,
                    ),
                ),
            )
            wrong_namespace = replace(
                miner,
                assignment=replace(
                    miner.assignment,
                    primary=replace(miner.assignment.primary, workstream="shell:WS-1"),
                ),
            )
            cross_system_relation = replace(
                miner,
                relations=(
                    Relation(RelationKind.RELATED, shell.id, shell.assignment.primary.source),
                ),
            )

            for changed, expected_field in (
                (cross_system_assignment, "assignment.source.source_key"),
                (wrong_namespace, "assignment.workstream"),
                (cross_system_relation, "relations.source.source_key"),
            ):
                obligations = tuple(
                    changed if item.id == miner.id else item
                    for item in bundle.obligations
                )
                diagnostics = validate_cross_records(replace(bundle, obligations=obligations))
                with self.subTest(field=expected_field):
                    self.assertTrue(
                        any(item.fatal and item.field == expected_field for item in diagnostics)
                    )

    def test_cross_validation_requires_complete_anchor_and_scoped_test_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            bundle = import_source_set(repo, "bootstrap", derive_workspace_evidence=False)
            anchor = next(
                item
                for item in bundle.evidence
                if item.kind is EvidenceKind.IMPLEMENTATION_ANCHOR
            )
            without_anchor = replace(
                bundle,
                evidence=tuple(item for item in bundle.evidence if item.id != anchor.id),
            )
            self.assertTrue(
                any(item.fatal for item in validate_cross_records(without_anchor))
            )

            obligation_id = bundle.obligations[0].id
            orphan = EvidenceDeclaration(
                "evidence:test:orphan",
                (obligation_id,),
                EvidenceKind.REGRESSION_DECLARATION,
                None,
                None,
                None,
                TestDeclaredCheck("src/fake.rs", "fake", "a" * 40),
            )
            with_orphan = replace(
                bundle,
                evidence=tuple(sorted((*bundle.evidence, orphan), key=lambda item: item.id)),
            )
            self.assertTrue(
                any(item.fatal for item in validate_cross_records(with_orphan))
            )

    def test_loader_rejects_forged_importer_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            write_import(repo, import_source_set(repo, "bootstrap", derive_workspace_evidence=False))
            corpus = load_tracked_corpus(repo)
            forged = replace(corpus.source_lock, importer="forged-ledger", importer_version=999)
            (repo / "parity/sources/bootstrap.json").write_bytes(
                canonical_json_bytes(forged.to_document())
            )
            with self.assertRaises(LedgerError) as caught:
                load_tracked_corpus(repo)
            self.assertEqual(
                caught.exception.diagnostics[0].code,
                FailureCode.CORPUS_DIGEST_MISMATCH.value,
            )


if __name__ == "__main__":
    unittest.main()
