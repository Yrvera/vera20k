"""Bootstrap import orchestration, semantic digests, and tracked corpus loading."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from . import IMPORTER_NAME, IMPORTER_VERSION, SCHEMA_VERSION
from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .evidence import derive_evidence, derive_path_evidence
from .graph import validate_graph
from .importers import import_core_checklist, import_miner, import_scheduler_checklist, import_shell
from .jsonio import canonical_json_bytes, sha256_bytes, stage_atomic_bytes
from .model import (
    Disposition,
    EvidenceDeclaration,
    EvidenceKind,
    EvidenceSetDocument,
    GitAncestorCheck,
    Obligation,
    ObligationSetDocument,
    SourceFileLock,
    SourceLockDocument,
    SourceRole,
    TestDeclaredCheck,
)
from .schema import decode_evidence_set, decode_obligation_set, decode_source_lock, load_canonical
from .source_sets import SOURCE_SETS, SourceConfig


@dataclass(frozen=True)
class ImportBundle:
    source_set: str
    sources: tuple[SourceFileLock, ...]
    obligations: tuple[Obligation, ...]
    dispositions: tuple[Disposition, ...]
    diagnostics: tuple[Diagnostic, ...]
    evidence: tuple[EvidenceDeclaration, ...]
    digest: str

    def documents(self) -> tuple[SourceLockDocument, ObligationSetDocument, EvidenceSetDocument]:
        return (
            SourceLockDocument(
                self.source_set,
                IMPORTER_NAME,
                IMPORTER_VERSION,
                self.digest,
                self.sources,
            ),
            ObligationSetDocument(
                self.source_set,
                self.digest,
                self.obligations,
                self.dispositions,
                self.diagnostics,
            ),
            EvidenceSetDocument(self.source_set, self.digest, self.evidence),
        )


@dataclass(frozen=True)
class Corpus:
    source_lock: SourceLockDocument
    obligation_set: ObligationSetDocument
    evidence_set: EvidenceSetDocument


def _error(code: FailureCode, message: str, *, exit_code: ExitCode = ExitCode.VALIDATION_FAILED) -> None:
    raise LedgerError(exit_code, [Diagnostic(code.value, message=message, fatal=True)])


def _source_path(repo: Path, config: SourceConfig) -> Path:
    return repo / Path(*config.path.split("/"))


def _read_required(repo: Path, config: SourceConfig) -> bytes:
    path = _source_path(repo, config)
    try:
        return path.read_bytes()
    except OSError as exc:
        raise LedgerError(
            ExitCode.REQUIRED_SOURCE_FAILED,
            [
                Diagnostic(
                    FailureCode.SOURCE_UNAVAILABLE.value,
                    source_path=config.path,
                    message=str(exc),
                    fatal=True,
                )
            ],
        ) from exc


def digest_payload(
    source_set: str,
    sources: tuple[SourceFileLock, ...],
    obligations: tuple[Obligation, ...],
    dispositions: tuple[Disposition, ...],
    diagnostics: tuple[Diagnostic, ...],
    evidence: tuple[EvidenceDeclaration, ...],
) -> dict[str, object]:
    return {
        "diagnostics": [item.to_document() for item in sorted(diagnostics)],
        "dispositions": [item.to_document() for item in sorted(dispositions, key=lambda item: item.source_id)],
        "evidence": [item.to_document() for item in sorted(evidence, key=lambda item: item.id)],
        "importer": IMPORTER_NAME,
        "importer_version": IMPORTER_VERSION,
        "obligations": [item.to_document() for item in sorted(obligations, key=lambda item: item.id)],
        "schema_version": SCHEMA_VERSION,
        "source_set": source_set,
        "sources": [item.to_document() for item in sorted(sources, key=lambda item: item.source_id)],
    }


def corpus_digest(payload: dict[str, object]) -> str:
    return sha256_bytes(canonical_json_bytes(payload))


def _locks(configs: tuple[SourceConfig, ...], raw: dict[str, bytes]) -> tuple[SourceFileLock, ...]:
    return tuple(
        sorted(
            (
                SourceFileLock(
                    config.source_id,
                    config.system,
                    config.role,
                    config.path,
                    sha256_bytes(raw[config.source_id]),
                    config.tracking,
                    config.adapter,
                    config.declared_count,
                )
                for config in configs
            ),
            key=lambda item: item.source_id,
        )
    )


def _existing_evidence(repo: Path, source_set: str) -> tuple[EvidenceDeclaration, ...]:
    path = repo / "parity" / "evidence" / f"{source_set}.json"
    if not path.exists():
        return ()
    document = load_canonical(path, decode_evidence_set)
    if document.source_set != source_set:
        _error(FailureCode.SCHEMA_INVALID, "existing evidence source_set differs")
    return document.evidence


def import_source_set(
    repo: Path,
    source_set: str,
    *,
    derive_workspace_evidence: bool = True,
) -> ImportBundle:
    if source_set not in SOURCE_SETS:
        _error(FailureCode.SCHEMA_INVALID, f"unknown source set {source_set!r}")
    configs = SOURCE_SETS[source_set]
    raw = {config.source_id: _read_required(repo, config) for config in configs}
    by_id = {config.source_id: config for config in configs}
    core = import_core_checklist(raw["core-todo"], by_id["core-todo"])
    scheduler = import_scheduler_checklist(raw["scheduler-roadmap"], by_id["scheduler-roadmap"])
    miner, miner_diagnostics = import_miner(
        raw["miner-scan"],
        raw["miner-roadmap"],
        by_id["miner-scan"],
        by_id["miner-roadmap"],
    )
    shell, dispositions, shell_diagnostics = import_shell(
        raw["shell-scan"],
        raw["shell-roadmap"],
        by_id["shell-scan"],
        by_id["shell-roadmap"],
    )
    obligations = tuple(sorted((*core, *scheduler, *miner, *shell), key=lambda item: item.id))
    sources = _locks(configs, raw)
    existing_evidence = _existing_evidence(repo, source_set)
    manual_evidence = tuple(
        item
        for item in existing_evidence
        if item.kind in {EvidenceKind.GAMEMD_VECTOR, EvidenceKind.BRIDGE_TRACE}
    )
    if derive_workspace_evidence:
        generated_evidence, evidence_diagnostics = derive_evidence(repo, obligations)
    else:
        generated_evidence = derive_path_evidence(obligations)
        evidence_diagnostics = ()
    evidence = tuple(sorted((*generated_evidence, *manual_evidence), key=lambda item: item.id))
    diagnostics = tuple(sorted((*miner_diagnostics, *shell_diagnostics, *evidence_diagnostics)))
    preliminary = ImportBundle(source_set, sources, obligations, dispositions, diagnostics, evidence, "")
    cross = validate_cross_records(preliminary)
    if cross:
        raise LedgerError(ExitCode.VALIDATION_FAILED, cross)
    validate_graph(obligations, dispositions)
    digest = corpus_digest(
        digest_payload(source_set, sources, obligations, dispositions, diagnostics, evidence)
    )
    return ImportBundle(source_set, sources, obligations, dispositions, diagnostics, evidence, digest)


def _all_source_refs(obligation: Obligation):
    yield obligation.source
    if obligation.assignment.primary is not None:
        yield obligation.assignment.primary.source
    for mention in obligation.assignment.related:
        yield mention.source
    for relation in obligation.relations:
        yield relation.source


def _assignment_mentions(obligation: Obligation):
    if obligation.assignment.primary is not None:
        yield obligation.assignment.primary
    yield from obligation.assignment.related


def validate_cross_records(bundle: ImportBundle) -> list[Diagnostic]:
    diagnostics: list[Diagnostic] = []
    obligation_ids = [item.id for item in bundle.obligations]
    disposition_ids = [item.source_id for item in bundle.dispositions]
    evidence_ids = [item.id for item in bundle.evidence]
    for values, code, label in (
        (obligation_ids, FailureCode.DUPLICATE_OBLIGATION, "obligation"),
        (disposition_ids, FailureCode.SCHEMA_INVALID, "disposition"),
        (evidence_ids, FailureCode.EVIDENCE_INVALID, "evidence"),
    ):
        duplicates = sorted({value for value in values if values.count(value) > 1})
        for duplicate in duplicates:
            diagnostics.append(Diagnostic(code.value, record_id=duplicate, message=f"duplicate {label}", fatal=True))
    active = set(obligation_ids)
    for disposition in bundle.dispositions:
        if disposition.source_id in active:
            diagnostics.append(
                Diagnostic(
                    FailureCode.DUPLICATE_OBLIGATION.value,
                    record_id=disposition.source_id,
                    message="active obligation also has a disposition",
                    fatal=True,
                )
            )
        for target in disposition.targets:
            if target not in active:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.UNRESOLVED_RELATION.value,
                        record_id=disposition.source_id,
                        field="targets",
                        message=f"missing disposition target {target}",
                        fatal=True,
                    )
                )
    locks = {item.source_id: item for item in bundle.sources}
    if len(locks) != len(bundle.sources):
        diagnostics.append(Diagnostic(FailureCode.SCHEMA_INVALID.value, message="duplicate source locks", fatal=True))
    expected_configs = {item.source_id: item for item in SOURCE_SETS.get(bundle.source_set, ())}
    if set(locks) != set(expected_configs):
        diagnostics.append(
            Diagnostic(
                FailureCode.SCHEMA_INVALID.value,
                field="sources",
                message=f"source-lock IDs differ: {sorted(locks)}",
                fatal=True,
            )
        )
    for source_id in sorted(set(locks) & set(expected_configs)):
        lock = locks[source_id]
        config = expected_configs[source_id]
        actual_metadata = (
            lock.system,
            lock.role,
            lock.path,
            lock.tracking,
            lock.adapter,
            lock.declared_count,
        )
        expected_metadata = (
            config.system,
            config.role,
            config.path,
            config.tracking,
            config.adapter,
            config.declared_count,
        )
        if actual_metadata != expected_metadata:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=source_id,
                    field="sources",
                    message="source-lock metadata differs from source-set configuration",
                    fatal=True,
                )
            )
    for obligation in bundle.obligations:
        expected_local_id = obligation.id.split(":", 1)[1]
        if obligation.source.local_id != expected_local_id:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=obligation.id,
                    field="source.local_id",
                    message=f"inventory local_id must be {expected_local_id}",
                    fatal=True,
                )
            )
        inventory_lock = locks.get(obligation.source.source_key)
        if inventory_lock is not None and (
            inventory_lock.system != obligation.system
            or inventory_lock.role is not SourceRole.INVENTORY
        ):
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=obligation.id,
                    field="source.source_key",
                    message="obligation inventory source must match its system",
                    fatal=True,
                )
            )
        for mention in _assignment_mentions(obligation):
            if not mention.workstream.startswith(f"{obligation.system}:"):
                diagnostics.append(
                    Diagnostic(
                        FailureCode.SCHEMA_INVALID.value,
                        record_id=obligation.id,
                        field="assignment.workstream",
                        message="assignment workstream must match the obligation system namespace",
                        fatal=True,
                    )
                )
            lock = locks.get(mention.source.source_key)
            if lock is not None and lock.system != obligation.system:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.SCHEMA_INVALID.value,
                        record_id=obligation.id,
                        field="assignment.source.source_key",
                        message="assignment source must match the obligation system",
                        fatal=True,
                    )
                )
        for relation in obligation.relations:
            lock = locks.get(relation.source.source_key)
            if lock is not None and lock.system != obligation.system:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.SCHEMA_INVALID.value,
                        record_id=obligation.id,
                        field="relations.source.source_key",
                        message="relation source must match the obligation system",
                        fatal=True,
                    )
                )
        for reference in _all_source_refs(obligation):
            lock = locks.get(reference.source_key)
            if lock is None:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.SCHEMA_INVALID.value,
                        record_id=obligation.id,
                        field="source_key",
                        message=f"missing source lock {reference.source_key}",
                        fatal=True,
                    )
                )
                continue
            expected = (lock.path, lock.sha256, lock.tracking, lock.adapter, IMPORTER_VERSION)
            actual = (
                reference.path,
                reference.sha256,
                reference.tracking,
                reference.importer,
                reference.importer_version,
            )
            if actual != expected:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.SCHEMA_INVALID.value,
                        record_id=obligation.id,
                        field="source",
                        message=f"source reference differs from lock {reference.source_key}",
                        fatal=True,
                    )
                )
    for disposition in bundle.dispositions:
        reference = disposition.source
        expected_local_id = disposition.source_id.split(":", 1)[1]
        if reference.local_id != expected_local_id:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=disposition.source_id,
                    field="source.local_id",
                    message=f"disposition local_id must be {expected_local_id}",
                    fatal=True,
                )
            )
        lock = locks.get(reference.source_key)
        if lock is None:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=disposition.source_id,
                    field="source_key",
                    message=f"missing source lock {reference.source_key}",
                    fatal=True,
                )
            )
            continue
        source_system = disposition.source_id.split(":", 1)[0]
        if lock.system != source_system or lock.role is not SourceRole.INVENTORY:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=disposition.source_id,
                    field="source.source_key",
                    message="disposition inventory source must match its system",
                    fatal=True,
                )
            )
        expected = (lock.path, lock.sha256, lock.tracking, lock.adapter, IMPORTER_VERSION)
        actual = (
            reference.path,
            reference.sha256,
            reference.tracking,
            reference.importer,
            reference.importer_version,
        )
        if actual != expected:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=disposition.source_id,
                    field="source",
                    message=f"disposition source differs from lock {reference.source_key}",
                    fatal=True,
                )
            )
    for diagnostic in bundle.diagnostics:
        if diagnostic.fatal:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    record_id=diagnostic.record_id,
                    field="diagnostics",
                    message=f"persisted diagnostic {diagnostic.code} cannot be fatal",
                    fatal=True,
                )
            )
    for evidence in bundle.evidence:
        for obligation_id in evidence.obligations:
            if obligation_id not in active:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.EVIDENCE_INVALID.value,
                        record_id=evidence.id,
                        field="obligations",
                        message=f"unknown obligation {obligation_id}",
                        fatal=True,
                    )
                )
    expected_path_evidence = derive_path_evidence(bundle.obligations)
    actual_path_evidence = tuple(
        item
        for item in bundle.evidence
        if item.kind is EvidenceKind.IMPLEMENTATION_ANCHOR
    )
    if actual_path_evidence != expected_path_evidence:
        diagnostics.append(
            Diagnostic(
                FailureCode.EVIDENCE_INVALID.value,
                field="evidence",
                message="implementation-anchor declarations are not the complete generated set",
                fatal=True,
            )
        )
    scoped_commits = {
        (obligation_id, evidence.check.commit)
        for evidence in bundle.evidence
        if evidence.kind is EvidenceKind.GIT_SCOPED
        and isinstance(evidence.check, GitAncestorCheck)
        for obligation_id in evidence.obligations
    }
    for evidence in bundle.evidence:
        if not isinstance(evidence.check, TestDeclaredCheck):
            continue
        for obligation_id in evidence.obligations:
            if (obligation_id, evidence.check.commit) not in scoped_commits:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.EVIDENCE_INVALID.value,
                        record_id=evidence.id,
                        field=obligation_id,
                        message="regression declaration lacks a matching scoped Git commit",
                        fatal=True,
                    )
                )
    expected_counts = {"core": 32, "scheduler": 17, "miner": 139, "shell": 89}
    actual_counts = {
        system: sum(item.system == system for item in bundle.obligations)
        for system in expected_counts
    }
    if actual_counts != expected_counts or len(bundle.obligations) != 277:
        diagnostics.append(
            Diagnostic(
                FailureCode.SCHEMA_INVALID.value,
                field="obligations",
                message=f"bootstrap counts differ: {actual_counts}, total={len(bundle.obligations)}",
                fatal=True,
            )
        )
    if bundle.source_set == "bootstrap":
        expected_unassigned = {
            "miner:L7",
            "miner:L34",
            "miner:L35",
            "miner:L43",
            "miner:M32",
            "shell:H1",
            "shell:H19",
        }
        actual_unassigned = {
            item.id for item in bundle.obligations if item.assignment.primary is None
        }
        if actual_unassigned != expected_unassigned:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    field="assignment",
                    message=f"bootstrap unassigned IDs differ: {sorted(actual_unassigned)}",
                    fatal=True,
                )
            )
        expected_dispositions = {"shell:L3", "shell:L25", "shell:L28"}
        if set(disposition_ids) != expected_dispositions:
            diagnostics.append(
                Diagnostic(
                    FailureCode.SCHEMA_INVALID.value,
                    field="dispositions",
                    message=f"bootstrap dispositions differ: {sorted(disposition_ids)}",
                    fatal=True,
                )
            )
    return sorted(diagnostics)


def _paths(repo: Path, source_set: str) -> tuple[Path, Path, Path]:
    return (
        repo / "parity" / "sources" / f"{source_set}.json",
        repo / "parity" / "obligations" / f"{source_set}.json",
        repo / "parity" / "evidence" / f"{source_set}.json",
    )


def write_import(repo: Path, bundle: ImportBundle) -> None:
    source_document, obligation_document, evidence_document = bundle.documents()
    source_value = source_document.to_document()
    obligation_value = obligation_document.to_document()
    evidence_value = evidence_document.to_document()
    decode_source_lock(source_value)
    decode_obligation_set(obligation_value)
    decode_evidence_set(evidence_value)
    source_path, obligation_path, evidence_path = _paths(repo, bundle.source_set)
    staged = []
    try:
        for path, payload in (
            (obligation_path, canonical_json_bytes(obligation_value)),
            (evidence_path, canonical_json_bytes(evidence_value)),
            (source_path, canonical_json_bytes(source_value)),
        ):
            staged.append(stage_atomic_bytes(path, payload))
        for item in staged:
            item.commit()
    finally:
        for item in staged:
            item.cleanup()


def load_tracked_corpus(repo: Path, source_set: str = "bootstrap") -> Corpus:
    source_path, obligation_path, evidence_path = _paths(repo, source_set)
    try:
        source_lock = load_canonical(source_path, decode_source_lock)
        obligation_set = load_canonical(obligation_path, decode_obligation_set)
        evidence_set = load_canonical(evidence_path, decode_evidence_set)
    except FileNotFoundError as exc:
        _error(FailureCode.SCHEMA_INVALID, f"tracked corpus file missing: {exc.filename}")
    documents = (source_lock, obligation_set, evidence_set)
    if source_lock.importer != IMPORTER_NAME or source_lock.importer_version != IMPORTER_VERSION:
        _error(
            FailureCode.CORPUS_DIGEST_MISMATCH,
            "source-lock importer identity/version differs from this ledger implementation",
        )
    if {item.source_set for item in documents} != {source_set}:
        _error(FailureCode.CORPUS_DIGEST_MISMATCH, "tracked source_set values differ")
    digests = {item.corpus_digest for item in documents}
    if len(digests) != 1:
        _error(FailureCode.CORPUS_DIGEST_MISMATCH, "tracked corpus digest markers differ")
    recomputed = corpus_digest(
        digest_payload(
            source_set,
            source_lock.sources,
            obligation_set.obligations,
            obligation_set.dispositions,
            obligation_set.diagnostics,
            evidence_set.evidence,
        )
    )
    if recomputed != source_lock.corpus_digest:
        _error(FailureCode.CORPUS_DIGEST_MISMATCH, "tracked semantic corpus digest differs")
    bundle = ImportBundle(
        source_set,
        source_lock.sources,
        obligation_set.obligations,
        obligation_set.dispositions,
        obligation_set.diagnostics,
        evidence_set.evidence,
        recomputed,
    )
    cross = validate_cross_records(bundle)
    if cross:
        raise LedgerError(ExitCode.VALIDATION_FAILED, cross)
    validate_graph(bundle.obligations, bundle.dispositions)
    return Corpus(source_lock, obligation_set, evidence_set)
