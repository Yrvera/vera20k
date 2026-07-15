"""Stable diagnostics and process exit codes for the parity ledger."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, IntEnum


class ExitCode(IntEnum):
    OK = 0
    INVALID_ARGUMENT = 2
    VALIDATION_FAILED = 10
    REQUIRED_SOURCE_FAILED = 11
    WORKSPACE_FAILED = 12
    INTERNAL_ERROR = 70


class FailureCode(str, Enum):
    UNSUPPORTED_SCHEMA = "UNSUPPORTED_SCHEMA"
    SCHEMA_INVALID = "SCHEMA_INVALID"
    NONCANONICAL_JSON = "NONCANONICAL_JSON"
    UNSAFE_PATH = "UNSAFE_PATH"
    SOURCE_MALFORMED = "SOURCE_MALFORMED"
    SOURCE_UNAVAILABLE = "SOURCE_UNAVAILABLE"
    SOURCE_STALE = "SOURCE_STALE"
    DECLARED_COUNT_MISMATCH = "DECLARED_COUNT_MISMATCH"
    UNNUMBERED_CONFIRMED_ITEMS = "UNNUMBERED_CONFIRMED_ITEMS"
    STALE_ROADMAP_REFERENCE = "STALE_ROADMAP_REFERENCE"
    DUPLICATE_OBLIGATION = "DUPLICATE_OBLIGATION"
    DUPLICATE_ASSIGNMENT = "DUPLICATE_ASSIGNMENT"
    UNRESOLVED_RELATION = "UNRESOLVED_RELATION"
    UNRESOLVED_DEPENDENCY = "UNRESOLVED_DEPENDENCY"
    DEPENDENCY_CYCLE = "DEPENDENCY_CYCLE"
    EVIDENCE_INVALID = "EVIDENCE_INVALID"
    CURRENT_ANCHOR_MISSING = "CURRENT_ANCHOR_MISSING"
    GIT_FAILED = "GIT_FAILED"
    CORPUS_DIGEST_MISMATCH = "CORPUS_DIGEST_MISMATCH"
    OUTPUT_IO_FAILED = "OUTPUT_IO_FAILED"
    INTERNAL_ERROR = "INTERNAL_ERROR"


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    source_path: str = ""
    record_id: str = ""
    field: str = ""
    message: str = ""
    fatal: bool = False

    def to_document(self) -> dict[str, object]:
        return {
            "code": self.code,
            "fatal": self.fatal,
            "field": self.field,
            "message": self.message,
            "record_id": self.record_id,
            "source_path": self.source_path,
        }


class LedgerError(Exception):
    """Expected ledger failure with deterministic diagnostics."""

    def __init__(self, exit_code: ExitCode, diagnostics: list[Diagnostic]) -> None:
        super().__init__(diagnostics[0].message if diagnostics else exit_code.name)
        self.exit_code = exit_code
        self.diagnostics = tuple(sorted(diagnostics))
