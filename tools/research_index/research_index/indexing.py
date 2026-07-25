"""Build/rebuild the research-index SQLite FTS database.

Owns the orchestration loop that pairs file discovery + chunking + metadata
extraction with the SQL primitive in database.rebuild_database. Consumed by
the index.py CLI and the research_reindex MCP tool.
"""

from __future__ import annotations

from pathlib import Path

from .chunking import chunk_file
from .database import rebuild_database
from .metadata import document_metadata, iter_indexable_files


DEFAULT_ROOTS: tuple[str, ...] = ("docs/research", "docs/plans", "ini")


def rebuild_index(workspace: Path, roots: list[Path], db_path: Path) -> str:
    """Rebuild the FTS database from disk.

    Args:
        workspace: Repo root. Relpaths are computed against this; missing
            link validation reads files relative to it.
        roots: Absolute or workspace-relative paths to walk for indexable
            files (markdown, ini). Caller is responsible for joining with
            workspace if needed.
        db_path: Output SQLite database path. Parent directory will be
            created if missing. Rebuild is atomic (writes .tmp then
            os.replace).

    Returns:
        One-line summary string matching the legacy index.py output:
        ``indexed documents=N chunks=M db=<path>``
    """
    documents = []
    chunk_total = 0

    for path in iter_indexable_files(roots):
        relpath = path.relative_to(workspace).as_posix()
        meta = document_metadata(path, workspace)
        chunks = list(chunk_file(path))
        if not chunks:
            continue
        documents.append((relpath, meta, chunks))
        chunk_total += len(chunks)

    rebuild_database(db_path, workspace, documents)
    return f"indexed documents={len(documents)} chunks={chunk_total} db={db_path}"
