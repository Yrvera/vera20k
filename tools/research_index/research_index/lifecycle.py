"""Research-index generation metadata, health inspection, and safe refresh."""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile
from typing import Iterable

from .indexing import DEFAULT_ROOTS, rebuild_index_result
from .locking import IndexLockTimeout, index_lock
from .metadata import iter_indexable_files


FORMAT_VERSION = 1
TOOL_VERSION = "2.0.0"
DEFAULT_HEALTH_LIMIT = 40
PACKAGE_ROOT = Path(__file__).resolve().parent
TOOL_ROOT = PACKAGE_ROOT.parent
BUILDER_INPUTS = (
    TOOL_ROOT / "schema.sql",
    PACKAGE_ROOT / "chunking.py",
    PACKAGE_ROOT / "database.py",
    PACKAGE_ROOT / "indexing.py",
    PACKAGE_ROOT / "metadata.py",
)
REQUIRED_TABLES = frozenset(
    {
        "chunks",
        "chunks_fts",
        "documents",
        "edges",
    }
)


class IndexLifecycleError(RuntimeError):
    """A safe-refresh precondition or publication failure."""


def manifest_path(db_path: Path) -> Path:
    return db_path.with_suffix(db_path.suffix + ".meta.json")


def normalize_roots(
    workspace: Path,
    roots: Iterable[str | Path],
) -> tuple[list[Path], list[str], list[Path]]:
    """Resolve roots inside the workspace and reject empty or unsafe scopes."""
    workspace = workspace.resolve()
    resolved_roots: list[Path] = []
    relative_roots: list[str] = []
    indexable_files: set[Path] = set()
    seen: set[Path] = set()

    for value in roots:
        candidate = Path(value)
        if not candidate.is_absolute():
            candidate = workspace / candidate
        resolved = candidate.resolve()
        try:
            relative = resolved.relative_to(workspace)
        except ValueError as exc:
            raise IndexLifecycleError(
                f"index root escapes workspace: {candidate}"
            ) from exc
        if resolved in seen:
            continue
        if not resolved.exists():
            raise IndexLifecycleError(f"index root does not exist: {relative}")
        if not (resolved.is_file() or resolved.is_dir()):
            raise IndexLifecycleError(
                f"index root is not a file or directory: {relative}"
            )

        files = iter_indexable_files([resolved])
        if not files:
            raise IndexLifecycleError(
                f"index root has no indexable files: {relative}"
            )
        for path in files:
            try:
                resolved_file = path.resolve()
                resolved_file.relative_to(workspace)
            except ValueError as exc:
                raise IndexLifecycleError(
                    f"indexable file escapes workspace: {path}"
                ) from exc
            indexable_files.add(resolved_file)

        seen.add(resolved)
        resolved_roots.append(resolved)
        relative_roots.append(relative.as_posix() or ".")

    if not resolved_roots:
        raise IndexLifecycleError("at least one index root is required")
    return resolved_roots, relative_roots, sorted(indexable_files)


def corpus_snapshot(
    workspace: Path,
    files: Iterable[Path],
) -> dict[str, dict[str, int]]:
    """Return deterministic stat identity for every indexed candidate."""
    workspace = workspace.resolve()
    snapshot: dict[str, dict[str, int]] = {}
    for resolved in files:
        try:
            relative = resolved.relative_to(workspace).as_posix()
        except ValueError as exc:
            raise IndexLifecycleError(
                f"indexable file escapes workspace: {path}"
            ) from exc
        stat = resolved.stat()
        snapshot[relative] = {
            "mtime_ns": stat.st_mtime_ns,
            "size": stat.st_size,
        }
    return dict(sorted(snapshot.items()))


def inspect_index(
    db_path: Path,
    workspace: Path,
    roots: Iterable[str | Path] | None = None,
    *,
    limit: int = DEFAULT_HEALTH_LIMIT,
) -> dict:
    """Inspect current database, manifest, and corpus without modifying them."""
    db_path = db_path.resolve()
    workspace = workspace.resolve()
    metadata, manifest_reasons = _load_manifest(manifest_path(db_path))
    root_values = _effective_root_values(workspace, metadata, roots)
    reasons = list(manifest_reasons)

    try:
        current_builder_signature = _builder_signature()
    except OSError as exc:
        current_builder_signature = None
        reasons.append(f"index builder inputs are unreadable: {exc}")

    try:
        _, root_labels, discovered = normalize_roots(workspace, root_values)
        current_files = corpus_snapshot(workspace, discovered)
    except (IndexLifecycleError, OSError) as exc:
        root_labels = [str(value).replace("\\", "/") for value in root_values]
        current_files = {}
        reasons.append(str(exc))

    database = _inspect_database(db_path)
    reasons.extend(database.pop("reasons"))

    stored_files = metadata.get("files", {}) if metadata else {}
    if not isinstance(stored_files, dict):
        stored_files = {}
        reasons.append("manifest files snapshot is invalid")

    added = sorted(set(current_files) - set(stored_files))
    removed = sorted(set(stored_files) - set(current_files))
    changed = sorted(
        path
        for path in set(current_files) & set(stored_files)
        if current_files[path] != stored_files[path]
    )

    if metadata is not None:
        if metadata.get("format_version") != FORMAT_VERSION:
            reasons.append("manifest format version differs")
        if metadata.get("tool_version") != TOOL_VERSION:
            reasons.append("manifest tool version differs")
        if (
            metadata.get("builder_signature")
            != current_builder_signature
        ):
            reasons.append("index builder signature differs")
        if not _same_path(metadata.get("workspace"), workspace):
            reasons.append("manifest workspace differs")
        if metadata.get("roots") != root_labels:
            reasons.append("manifest roots differ")
        if metadata.get("generation") != _generation(
            workspace,
            root_labels,
            current_files,
            current_builder_signature,
        ):
            reasons.append("generation fingerprint differs")
        if metadata.get("database") != database.get("identity"):
            reasons.append("database identity differs")
        if metadata.get("document_count") != database.get("document_count"):
            reasons.append("database document count differs")
        if metadata.get("chunk_count") != database.get("chunk_count"):
            reasons.append("database chunk count differs")

    if added:
        reasons.append(f"{len(added)} unindexed file(s) added")
    if changed:
        reasons.append(f"{len(changed)} indexed file(s) changed")
    if removed:
        reasons.append(f"{len(removed)} indexed file(s) removed")

    reasons = _dedupe(reasons)
    return {
        "ready": database["ready"],
        "fresh": database["ready"] and metadata is not None and not reasons,
        "workspace": str(workspace),
        "db_path": str(db_path),
        "manifest_path": str(manifest_path(db_path)),
        "format_version": (
            metadata.get("format_version") if metadata else None
        ),
        "tool_version": metadata.get("tool_version") if metadata else None,
        "builder_signature": current_builder_signature,
        "indexed_builder_signature": (
            metadata.get("builder_signature") if metadata else None
        ),
        "generation": metadata.get("generation") if metadata else None,
        "built_at_utc": metadata.get("built_at_utc") if metadata else None,
        "roots": root_labels,
        "document_count": database.get("document_count", 0),
        "chunk_count": database.get("chunk_count", 0),
        "current_file_count": len(current_files),
        "changes": {
            "added": added[:limit],
            "changed": changed[:limit],
            "removed": removed[:limit],
            "counts": {
                "added": len(added),
                "changed": len(changed),
                "removed": len(removed),
            },
        },
        "reasons": reasons,
    }


def refresh_index(
    db_path: Path,
    workspace: Path,
    roots: Iterable[str | Path] | None = None,
    *,
    limit: int = DEFAULT_HEALTH_LIMIT,
) -> dict:
    """Safely rebuild and publish a fresh database generation."""
    db_path = db_path.resolve()
    workspace = workspace.resolve()
    try:
        with index_lock(db_path):
            return _refresh_index_unlocked(
                db_path,
                workspace,
                roots=roots,
                limit=limit,
            )
    except IndexLockTimeout as exc:
        raise IndexLifecycleError(str(exc)) from exc


def _refresh_index_unlocked(
    db_path: Path,
    workspace: Path,
    roots: Iterable[str | Path] | None = None,
    *,
    limit: int = DEFAULT_HEALTH_LIMIT,
) -> dict:
    """Rebuild while the caller holds the generation publication lock."""
    metadata, _ = _load_manifest(manifest_path(db_path))
    root_values = _effective_root_values(workspace, metadata, roots)

    try:
        builder_before = _builder_signature()
        root_paths, root_labels, discovered = normalize_roots(
            workspace,
            root_values,
        )
        before = corpus_snapshot(workspace, discovered)
        rebuilt = rebuild_index_result(workspace, root_paths, db_path)
        _, after_root_labels, after_discovered = normalize_roots(
            workspace,
            root_labels,
        )
        after = corpus_snapshot(workspace, after_discovered)
        builder_after = _builder_signature()
    except IndexLifecycleError:
        raise
    except (OSError, sqlite3.Error, ValueError) as exc:
        raise IndexLifecycleError(f"research-index rebuild failed: {exc}") from exc

    if (
        root_labels != after_root_labels
        or before != after
        or builder_before != builder_after
    ):
        raise IndexLifecycleError(
            "research corpus or builder changed during rebuild; "
            "generation was not certified"
        )

    database_identity = _database_identity(db_path)
    metadata = {
        "format_version": FORMAT_VERSION,
        "tool_version": TOOL_VERSION,
        "builder_signature": builder_after,
        "workspace": str(workspace),
        "roots": root_labels,
        "generation": _generation(
            workspace,
            root_labels,
            after,
            builder_after,
        ),
        "built_at_utc": datetime.now(timezone.utc)
        .isoformat(timespec="seconds")
        .replace("+00:00", "Z"),
        "document_count": rebuilt.document_count,
        "chunk_count": rebuilt.chunk_count,
        "database": database_identity,
        "files": after,
    }
    try:
        _atomic_write_json(manifest_path(db_path), metadata)
    except OSError as exc:
        raise IndexLifecycleError(
            f"database rebuilt but generation manifest was not published: {exc}"
        ) from exc

    health = inspect_index(
        db_path,
        workspace,
        roots=root_labels,
        limit=limit,
    )
    if not health["fresh"]:
        raise IndexLifecycleError(
            "rebuilt research index did not pass freshness inspection: "
            + "; ".join(health["reasons"])
        )
    health["refreshed"] = True
    health["summary"] = rebuilt.summary()
    return health


def ensure_fresh(
    db_path: Path,
    workspace: Path,
    roots: Iterable[str | Path] | None = None,
    *,
    limit: int = DEFAULT_HEALTH_LIMIT,
) -> dict:
    """Return fresh health, rebuilding synchronously only when required."""
    db_path = db_path.resolve()
    workspace = workspace.resolve()
    health = inspect_index(
        db_path,
        workspace,
        roots=roots,
        limit=limit,
    )
    if health["fresh"]:
        health["refreshed"] = False
        return health

    try:
        with index_lock(db_path):
            # Another process may have published the required generation while
            # this caller waited. Recheck under the publication lock before
            # performing the expensive rebuild.
            health = inspect_index(
                db_path,
                workspace,
                roots=roots,
                limit=limit,
            )
            if health["fresh"]:
                health["refreshed"] = False
                return health
            return _refresh_index_unlocked(
                db_path,
                workspace,
                roots=roots,
                limit=limit,
            )
    except IndexLockTimeout as exc:
        raise IndexLifecycleError(str(exc)) from exc


def effective_root_labels(
    db_path: Path,
    workspace: Path,
) -> list[str]:
    """Return stored explicit roots, or defaults for a legacy generation."""
    metadata, _ = _load_manifest(manifest_path(db_path.resolve()))
    return [
        str(value).replace("\\", "/")
        for value in _effective_root_values(
            workspace.resolve(),
            metadata,
            None,
        )
    ]


def _effective_root_values(
    workspace: Path,
    metadata: dict | None,
    roots: Iterable[str | Path] | None,
) -> list[str | Path]:
    if roots is not None:
        return list(roots)
    if metadata is not None and _same_path(
        metadata.get("workspace"),
        workspace,
    ):
        stored = metadata.get("roots")
        if (
            isinstance(stored, list)
            and stored
            and all(isinstance(value, str) for value in stored)
        ):
            return list(stored)
    return list(DEFAULT_ROOTS)


def _load_manifest(path: Path) -> tuple[dict | None, list[str]]:
    if not path.is_file():
        return None, ["generation manifest is missing"]
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None, ["generation manifest is unreadable"]
    if not isinstance(value, dict):
        return None, ["generation manifest root is invalid"]
    return value, []


def _inspect_database(db_path: Path) -> dict:
    if not db_path.is_file():
        return {
            "ready": False,
            "document_count": 0,
            "chunk_count": 0,
            "identity": None,
            "reasons": ["research database is missing"],
        }
    try:
        uri = db_path.as_uri() + "?mode=ro"
        conn = sqlite3.connect(uri, uri=True)
        try:
            tables = {
                row[0]
                for row in conn.execute(
                    "SELECT name FROM sqlite_master WHERE type IN ('table', 'view')"
                )
            }
            missing = sorted(REQUIRED_TABLES - tables)
            if missing:
                return {
                    "ready": False,
                    "document_count": 0,
                    "chunk_count": 0,
                    "identity": _database_identity(db_path),
                    "reasons": [
                        "research database lacks required tables: "
                        + ", ".join(missing)
                    ],
                }
            document_count = int(
                conn.execute("SELECT COUNT(*) FROM documents").fetchone()[0]
            )
            chunk_count = int(
                conn.execute("SELECT COUNT(*) FROM chunks").fetchone()[0]
            )
        finally:
            conn.close()
    except (OSError, sqlite3.Error) as exc:
        return {
            "ready": False,
            "document_count": 0,
            "chunk_count": 0,
            "identity": None,
            "reasons": [f"research database is unreadable: {exc}"],
        }
    return {
        "ready": True,
        "document_count": document_count,
        "chunk_count": chunk_count,
        "identity": _database_identity(db_path),
        "reasons": [],
    }


def _database_identity(db_path: Path) -> dict[str, int]:
    stat = db_path.stat()
    return {
        "mtime_ns": stat.st_mtime_ns,
        "size": stat.st_size,
    }


def _generation(
    workspace: Path,
    roots: list[str],
    files: dict[str, dict[str, int]],
    builder_signature: str | None,
) -> str:
    payload = {
        "builder_signature": builder_signature,
        "files": files,
        "format_version": FORMAT_VERSION,
        "roots": roots,
        "tool_version": TOOL_VERSION,
        "workspace": str(workspace.resolve()),
    }
    encoded = json.dumps(
        payload,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _builder_signature() -> str:
    digest = hashlib.sha256()
    for path in BUILDER_INPUTS:
        digest.update(path.relative_to(TOOL_ROOT).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _atomic_write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f"{path.name}.",
        suffix=".tmp",
        dir=path.parent,
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(
            descriptor,
            "w",
            encoding="utf-8",
            newline="\n",
        ) as handle:
            json.dump(
                value,
                handle,
                ensure_ascii=False,
                separators=(",", ":"),
                sort_keys=True,
            )
            handle.write("\n")
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def _same_path(value: object, path: Path) -> bool:
    if not isinstance(value, str):
        return False
    return os.path.normcase(os.path.abspath(value)) == os.path.normcase(
        os.path.abspath(path)
    )


def _dedupe(values: list[str]) -> list[str]:
    return list(dict.fromkeys(values))
