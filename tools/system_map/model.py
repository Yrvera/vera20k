"""Shared constants, diagnostics, and identifiers for System Map v2."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re


SCHEMA_VERSION = 2

INVENTORY_PATH = Path(
    "docs/research/GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md"
)
STATUS_MATRIX_PATH = Path(
    "docs/research/GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md"
)
CORE_SERVICES_PATH = Path("docs/research/CORE_ENGINE_SERVICES_MAP.md")
DATA_DIR = Path("system_map")
REGISTRY_PATH = DATA_DIR / "registry.v2.json"
TOPOLOGY_PATH = DATA_DIR / "topology.v2.json"
SOURCE_LOCK_PATH = DATA_DIR / "source-lock.v2.json"
MECHANISMS_PATH = DATA_DIR / "mechanisms.v1.json"

CANONICAL_ID_RE = re.compile(r"^GSI-\d{2}\.\d{2}$")
FAMILY_ID_RE = re.compile(r"^GSI-\d{2}$")
LEGACY_PSEUDO_GSI_RE = re.compile(r"^GSI-\d{2}\.\d{2}[A-Z]$")
SLICE_ID_RE = re.compile(r"^SLICE-\d{8}-[A-Z0-9-]+$")
LOOP_ID_RE = re.compile(r"^LOOP-\d{3}-[A-Z0-9-]+$")
EDGE_ID_RE = re.compile(r"^EDGE-\d{4}-[A-Z0-9-]+$")
MECHANISM_ID_RE = re.compile(r"^MBLK-\d{3}-[A-Z0-9-]+$")
MECHANISM_EDGE_ID_RE = re.compile(r"^MBEDGE-\d{4}-[A-Z0-9-]+$")
COMMIT_RE = re.compile(r"^[0-9a-fA-F]{40}$")
ADDRESS_RE = re.compile(r"^0x[0-9A-Fa-f]{4,8}$")

ID_POLICY_PATTERNS = {
    "canonical_system_pattern": r"^GSI-[0-9]{2}\.[0-9]{2}$",
    "slice_pattern": r"^SLICE-[0-9]{8}-[A-Z0-9-]+$",
    "loop_pattern": r"^LOOP-[0-9]{3}-[A-Z0-9-]+$",
    "edge_pattern": r"^EDGE-[0-9]{4}-[A-Z0-9-]+$",
}

ACTIVITY_VALUES = frozenset(
    {
        "STOCK_ACTIVE",
        "MODE_ACTIVE",
        "CONTENT_CONDITIONAL",
        "COMPILED_INACTIVE",
        "UNKNOWN",
        "GROUP_NODE",
    }
)
INVENTORY_VALUES = frozenset(
    {"DISCOVERED", "BOUNDED", "EXHAUSTIVE_SLICE", "GROUP_NODE"}
)
NATIVE_VALUES = frozenset(
    {"UNCHECKED", "ANCHORED", "CONTRACTED", "NATIVE_ORACLE", "N/A"}
)
RUST_VALUES = frozenset(
    {
        "ABSENT",
        "SCAFFOLD",
        "PARTIAL",
        "PRESENT",
        "COMPLETE_FOR_CONTRACT",
        "N/A",
    }
)
PARITY_VALUES = frozenset(
    {"UNCHECKED", "DRIFT", "TRACE_MATCHED", "VERIFIED", "N/A"}
)

EDGE_PLANES = frozenset({"native", "rust", "oracle", "routing"})
EDGE_KINDS = frozenset(
    {
        "requires",
        "owns_state",
        "owns_algorithm",
        "reads",
        "writes",
        "ordered_before",
        "emits",
        "emits_to",
        "consumes",
        "lifecycle_handoff",
        "handoff_to",
        "renders",
        "presents",
        "plays_audio",
        "drives",
        "gated_by",
        "loop_requires",
    }
)
RUST_COVERAGE_VALUES = frozenset({"representative", "exhaustive"})
ORACLE_STATUS_VALUES = frozenset(
    {
        "BLOCKED",
        "BLOCKED_EXTERNAL",
        "TRACE_MATCHED",
        "UNCHECKED",
        "UNVERIFIED",
        "VERIFIED",
    }
)
ORACLE_VERIFICATION_METHOD_VALUES = frozenset(
    {"exhaustive_proof", "native_executable"}
)
SERVICE_ROLE_VALUES = frozenset(
    {
        "consumes",
        "drives",
        "handoff",
        "orders",
        "owns_algorithm",
        "owns_iteration",
        "owns_lifecycle",
        "owns_state",
        "presents",
        "provides",
    }
)


@dataclass(frozen=True, order=True)
class Diagnostic:
    """One deterministic validation or freshness diagnostic."""

    severity: str
    code: str
    message: str
    record_id: str = ""
    field: str = ""
    path: str = ""

    def to_document(self) -> dict[str, str]:
        return {
            "code": self.code,
            "field": self.field,
            "message": self.message,
            "path": self.path,
            "record_id": self.record_id,
            "severity": self.severity,
        }


class SystemMapError(Exception):
    """Process-boundary error with structured diagnostics."""

    def __init__(self, diagnostics: list[Diagnostic], exit_code: int = 2):
        super().__init__("; ".join(item.message for item in diagnostics))
        self.diagnostics = diagnostics
        self.exit_code = exit_code


def canonical_system_id(value: object) -> str | None:
    """Return a canonical GSI ID, or ``None`` for invalid values."""

    if isinstance(value, str) and CANONICAL_ID_RE.fullmatch(value):
        return value
    return None


def family_for(system_id: str) -> str:
    """Return ``GSI-NN`` for a canonical row ID."""

    return system_id[:6]


def loop_stage_ids(loop: dict) -> tuple[list[object], list[int | None]]:
    """Normalize both supported loop-stage encodings."""

    stages = loop.get("ordered_stages", loop.get("stages", []))
    if not isinstance(stages, list):
        return [], []
    ids: list[object] = []
    orders: list[int | None] = []
    for stage in stages:
        if isinstance(stage, str):
            ids.append(stage)
            orders.append(None)
        elif isinstance(stage, dict):
            ids.append(
                stage.get("system", stage.get("system_id", stage.get("id")))
            )
            order = stage.get("order")
            orders.append(order if isinstance(order, int) else None)
        else:
            ids.append(None)
            orders.append(None)
    return ids, orders
