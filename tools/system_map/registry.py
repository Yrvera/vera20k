"""Import the ignored Markdown inventory into a normalized GSI baseline."""

from __future__ import annotations

from pathlib import Path
import re

from .jsonio import (
    atomic_write_bytes,
    canonical_json_bytes,
    load_json_strict,
    sha256_bytes,
    sha256_file,
)
from .model import (
    ACTIVITY_VALUES,
    CORE_SERVICES_PATH,
    DATA_DIR,
    Diagnostic,
    INVENTORY_PATH,
    INVENTORY_VALUES,
    NATIVE_VALUES,
    PARITY_VALUES,
    REGISTRY_PATH,
    RUST_VALUES,
    SCHEMA_VERSION,
    SOURCE_LOCK_PATH,
    STATUS_MATRIX_PATH,
    SystemMapError,
    canonical_system_id,
    family_for,
)


_FAMILY_HEADING_RE = re.compile(r"^###\s+(GSI-\d{2})\b(.*)$")
_RUST_SNAPSHOT_RE = re.compile(
    r"^\*\*Rust snapshot:\*\*\s*`([0-9a-fA-F]{7,64})`",
    re.MULTILINE,
)
_SERVICE_SLUG_RE = re.compile(r"^`([^`]+)`(?:\s+\([^)]*\))?$")


def find_repo_root(start: Path | None = None) -> Path:
    """Find the repository root without assuming the current directory."""

    current = (start or Path.cwd()).resolve()
    if current.is_file():
        current = current.parent
    for candidate in (current, *current.parents):
        if (candidate / "Cargo.toml").is_file() and (candidate / ".git").exists():
            return candidate
    raise SystemMapError(
        [
            Diagnostic(
                "error",
                "REPOSITORY_NOT_FOUND",
                f"could not find repository root from {current.as_posix()}",
            )
        ],
        exit_code=3,
    )


def read_utf8(path: Path) -> str:
    try:
        return path.read_bytes().decode("utf-8-sig", errors="strict")
    except FileNotFoundError as exc:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "MISSING_SOURCE",
                    f"source does not exist: {path.as_posix()}",
                    path=path.as_posix(),
                )
            ]
        ) from exc
    except UnicodeDecodeError as exc:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "INVALID_UTF8",
                    f"source is not strict UTF-8: {exc}",
                    path=path.as_posix(),
                )
            ]
        ) from exc


def split_markdown_row(line: str) -> list[str] | None:
    """Split a pipe table row while respecting code spans and escaped pipes."""

    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        return None
    cells: list[str] = []
    current: list[str] = []
    in_code = False
    escaped = False
    for char in stripped[1:-1]:
        if escaped:
            current.append(char)
            escaped = False
            continue
        if char == "\\":
            escaped = True
            current.append(char)
            continue
        if char == "`":
            in_code = not in_code
            current.append(char)
            continue
        if char == "|" and not in_code:
            cells.append("".join(current).strip())
            current = []
            continue
        current.append(char)
    if escaped:
        current.append("\\")
    cells.append("".join(current).strip())
    return cells


def _family_heading(line: str) -> tuple[str, str] | None:
    match = _FAMILY_HEADING_RE.match(line.strip())
    if not match:
        return None
    family_id = match.group(1)
    remainder = re.sub(r"^[\s\u2013\u2014-]+", "", match.group(2)).strip()
    return family_id, remainder


def parse_inventory(text: str, source_path: str) -> tuple[dict, dict[str, dict]]:
    """Parse family metadata and every canonical inventory row."""

    families: dict[str, dict] = {}
    systems: dict[str, dict] = {}
    current_family: str | None = None
    diagnostics: list[Diagnostic] = []

    for line_number, line in enumerate(text.splitlines(), 1):
        heading = _family_heading(line)
        if heading:
            current_family, family_name = heading
            if current_family in families:
                diagnostics.append(
                    Diagnostic(
                        "error",
                        "DUPLICATE_FAMILY",
                        f"duplicate family heading {current_family}",
                        record_id=current_family,
                        path=f"{source_path}:{line_number}",
                    )
                )
            else:
                families[current_family] = {
                    "name": family_name,
                    "systems": [],
                }
            continue

        cells = split_markdown_row(line)
        if not cells or not cells or canonical_system_id(cells[0]) is None:
            continue
        if len(cells) != 3:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "INVALID_INVENTORY_ROW",
                    f"expected 3 cells, found {len(cells)}",
                    record_id=cells[0],
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        system_id, name, discovery_scope = cells
        expected_family = family_for(system_id)
        if current_family != expected_family:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "INVENTORY_FAMILY_MISMATCH",
                    f"{system_id} appears under {current_family}, expected {expected_family}",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        if system_id in systems:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "DUPLICATE_SYSTEM_ID",
                    f"duplicate inventory ID {system_id}",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        if not name or not discovery_scope:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "EMPTY_INVENTORY_FIELD",
                    "system name and discovery scope must be non-empty",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        systems[system_id] = {
            "discovery_scope": discovery_scope,
            "family": expected_family,
            "family_name": families[expected_family]["name"],
            "name": name,
            "source_line": line_number,
        }
        families[expected_family]["systems"].append(system_id)

    if diagnostics:
        raise SystemMapError(diagnostics)
    if not systems:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "EMPTY_INVENTORY",
                    "no canonical GSI rows were parsed",
                    path=source_path,
                )
            ]
        )
    for family in families.values():
        family["systems"].sort()
    return dict(sorted(families.items())), dict(sorted(systems.items()))


def parse_status_matrix(
    text: str, source_path: str
) -> tuple[str, dict[str, dict]]:
    """Parse the row-level status matrix as a historical baseline."""

    snapshot_match = _RUST_SNAPSHOT_RE.search(text)
    if not snapshot_match:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "MISSING_RUST_SNAPSHOT",
                    "status matrix lacks a Rust snapshot commit",
                    path=source_path,
                )
            ]
        )
    snapshot = snapshot_match.group(1).lower()
    rows: dict[str, dict] = {}
    current_family: str | None = None
    diagnostics: list[Diagnostic] = []

    for line_number, line in enumerate(text.splitlines(), 1):
        heading = _family_heading(line)
        if heading:
            current_family = heading[0]
            continue
        cells = split_markdown_row(line)
        if not cells or canonical_system_id(cells[0]) is None:
            continue
        if len(cells) != 8:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "INVALID_STATUS_ROW",
                    f"expected 8 cells, found {len(cells)}",
                    record_id=cells[0],
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        (
            system_id,
            name,
            activity,
            inventory_evidence,
            native_evidence,
            rust_implementation,
            parity,
            basis,
        ) = cells
        if system_id in rows:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "DUPLICATE_STATUS_ID",
                    f"duplicate status ID {system_id}",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
            continue
        expected_family = family_for(system_id)
        if current_family != expected_family:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "STATUS_FAMILY_MISMATCH",
                    f"{system_id} appears under {current_family}, expected {expected_family}",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
        checks = (
            ("activity", activity, ACTIVITY_VALUES),
            ("inventory_evidence", inventory_evidence, INVENTORY_VALUES),
            ("native_evidence", native_evidence, NATIVE_VALUES),
            ("rust_implementation", rust_implementation, RUST_VALUES),
            ("parity", parity, PARITY_VALUES),
        )
        for field, value, allowed in checks:
            if value not in allowed:
                diagnostics.append(
                    Diagnostic(
                        "error",
                        "INVALID_BASELINE_VALUE",
                        f"{field} has unsupported value {value!r}",
                        record_id=system_id,
                        field=field,
                        path=f"{source_path}:{line_number}",
                    )
                )
        is_group = activity == "GROUP_NODE"
        if is_group:
            if (
                inventory_evidence != "GROUP_NODE"
                or native_evidence != "N/A"
                or rust_implementation != "N/A"
                or parity != "N/A"
            ):
                diagnostics.append(
                    Diagnostic(
                        "error",
                        "INVALID_GROUP_STATUS",
                        "GROUP_NODE rows require GROUP_NODE/N/A/N/A/N/A axes",
                        record_id=system_id,
                        path=f"{source_path}:{line_number}",
                    )
                )
        elif "N/A" in {native_evidence, rust_implementation, parity}:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "ATOMIC_STATUS_NA",
                    "atomic rows cannot use N/A status values",
                    record_id=system_id,
                    path=f"{source_path}:{line_number}",
                )
            )
        rows[system_id] = {
            "activity": activity,
            "basis": basis,
            "inventory_evidence": inventory_evidence,
            "native_evidence": native_evidence,
            "parity": parity,
            "rust_implementation": rust_implementation,
            "source_line": line_number,
        }

    if diagnostics:
        raise SystemMapError(diagnostics)
    return snapshot, dict(sorted(rows.items()))


def parse_core_service_slugs(text: str) -> list[str]:
    """Parse the 41 service catalog slugs without importing adjacency prose."""

    in_catalog = False
    slugs: set[str] = set()
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("## ") and "Catalog" in stripped:
            in_catalog = True
            continue
        if in_catalog and stripped.startswith("## ") and "Catalog" not in stripped:
            break
        if not in_catalog:
            continue
        cells = split_markdown_row(line)
        if not cells:
            continue
        match = _SERVICE_SLUG_RE.fullmatch(cells[0])
        if match:
            slugs.add(match.group(1))
    if not slugs:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "EMPTY_SERVICE_CATALOG",
                    "no core service slugs were parsed",
                    path=CORE_SERVICES_PATH.as_posix(),
                )
            ]
        )
    return sorted(slugs)


def build_registry(
    repo: Path,
    *,
    inventory_path: Path = INVENTORY_PATH,
    status_path: Path = STATUS_MATRIX_PATH,
    core_services_path: Path = CORE_SERVICES_PATH,
) -> tuple[dict, dict]:
    """Build normalized registry and source-lock documents in memory."""

    inventory_abs = repo / inventory_path
    status_abs = repo / status_path
    core_abs = repo / core_services_path
    inventory_text = read_utf8(inventory_abs)
    status_text = read_utf8(status_abs)
    core_text = read_utf8(core_abs)

    families, inventory_rows = parse_inventory(
        inventory_text, inventory_path.as_posix()
    )
    baseline_snapshot, status_rows = parse_status_matrix(
        status_text, status_path.as_posix()
    )
    inventory_ids = set(inventory_rows)
    status_ids = set(status_rows)
    diagnostics: list[Diagnostic] = []
    for system_id in sorted(inventory_ids - status_ids):
        diagnostics.append(
            Diagnostic(
                "error",
                "STATUS_ROW_MISSING",
                "inventory row is missing from status matrix",
                record_id=system_id,
            )
        )
    for system_id in sorted(status_ids - inventory_ids):
        diagnostics.append(
            Diagnostic(
                "error",
                "STATUS_ROW_EXTRA",
                "status row is absent from inventory",
                record_id=system_id,
            )
        )
    for system_id in sorted(inventory_ids & status_ids):
        if inventory_rows[system_id]["name"] != _status_name(
            status_text, status_rows[system_id]["source_line"]
        ):
            diagnostics.append(
                Diagnostic(
                    "error",
                    "SYSTEM_NAME_MISMATCH",
                    "inventory and status-matrix names differ",
                    record_id=system_id,
                )
            )
    if diagnostics:
        raise SystemMapError(diagnostics)

    systems = {}
    for system_id, inventory in inventory_rows.items():
        baseline = dict(status_rows[system_id])
        baseline.pop("source_line", None)
        system = dict(inventory)
        system["baseline_status"] = baseline
        systems[system_id] = system

    service_slugs = parse_core_service_slugs(core_text)
    registry = {
        "baseline_rust_snapshot": baseline_snapshot,
        "families": families,
        "schema_version": SCHEMA_VERSION,
        "service_catalog": service_slugs,
        "systems": systems,
    }
    registry_bytes = canonical_json_bytes(registry)
    source_lock = {
        "baseline_rust_snapshot": baseline_snapshot,
        "registry_sha256": sha256_bytes(registry_bytes),
        "schema_version": SCHEMA_VERSION,
        "sources": {
            "core_services": {
                "path": core_services_path.as_posix(),
                "sha256": sha256_file(core_abs),
            },
            "inventory": {
                "path": inventory_path.as_posix(),
                "sha256": sha256_file(inventory_abs),
            },
            "status_matrix": {
                "path": status_path.as_posix(),
                "sha256": sha256_file(status_abs),
            },
        },
    }
    return registry, source_lock


def _status_name(text: str, line_number: int) -> str:
    lines = text.splitlines()
    if line_number < 1 or line_number > len(lines):
        return ""
    cells = split_markdown_row(lines[line_number - 1])
    return cells[1] if cells and len(cells) >= 2 else ""


def write_import(
    repo: Path,
    registry: dict,
    source_lock: dict,
    *,
    output_dir: Path = DATA_DIR,
) -> None:
    """Persist both imported documents atomically and deterministically."""

    registry_path = repo / output_dir / REGISTRY_PATH.name
    lock_path = repo / output_dir / SOURCE_LOCK_PATH.name
    atomic_write_bytes(registry_path, canonical_json_bytes(registry))
    atomic_write_bytes(lock_path, canonical_json_bytes(source_lock))


def load_registry(repo: Path) -> dict:
    value = load_json_strict(repo / REGISTRY_PATH)
    if not isinstance(value, dict):
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "REGISTRY_NOT_OBJECT",
                    "registry root must be an object",
                    path=REGISTRY_PATH.as_posix(),
                )
            ]
        )
    return value


def load_source_lock(repo: Path) -> dict:
    value = load_json_strict(repo / SOURCE_LOCK_PATH)
    if not isinstance(value, dict):
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "SOURCE_LOCK_NOT_OBJECT",
                    "source lock root must be an object",
                    path=SOURCE_LOCK_PATH.as_posix(),
                )
            ]
        )
    return value
