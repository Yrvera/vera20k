# research_index MCP Server — Phase 2 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Code blocks are complete and ready to paste — do not summarize or interpret.

**Goal:** Replicate the Phase 1 pattern across the remaining 7 tools
(`research_related`, `research_graph`, `research_map`, `research_handoff`,
`research_validate`, `research_brief`, `research_reindex`), preceded by the
`rebuild_index` library refactor that `research_reindex` depends on, followed
by smoke tests and a manual end-to-end integration test in a restarted
Claude Code session.

**Architecture:** All seven new tools follow the Phase 1 body shape (see
[tools/research_index/mcp_server.py:47-84](tools/research_index/mcp_server.py:47)).
Phase 1 froze every design unknown — `Literal[...]`, `format` kwarg,
`str | None`, relative `.mcp.json` path, and `stdout.reconfigure` all
verified. Phase 2 is mechanical replication.

**Design Doc:** [docs/plans/2026-05-26-research-index-mcp-server-design.md](docs/plans/2026-05-26-research-index-mcp-server-design.md)

**Phase 1 Plan (pattern source):** [docs/plans/2026-05-26-research-index-mcp-server-phase-1-plan.md](docs/plans/2026-05-26-research-index-mcp-server-phase-1-plan.md)

---

## Grounding Summary

This is tooling/infrastructure work, not gameplay parity work. The RA2-specific
grounding sections of the write-plan skill (docs/research/, Ghidra MCP
verification, INI keys, gamemd.exe addresses) are **not applicable**. Grounding
comes from three sources, all already validated:

- **Phase 1 success** — [tools/research_index/mcp_server.py](tools/research_index/mcp_server.py)
  is live, the `research_search` tool round-trips through MCP end-to-end, all
  four design unknowns resolved positively. The body shape (`library_fn` →
  empty-result hint (list returns only) → JSON branch → `format_*` text
  branch) is the template every Phase 2 tool mirrors.
- **Library signatures** verified live by reading every entry point in
  [tools/research_index/research_index/database.py:168,238,253](tools/research_index/research_index/database.py),
  [graph.py:29,79,103,107](tools/research_index/research_index/graph.py),
  [system_map.py:24](tools/research_index/research_index/system_map.py),
  [handoff.py:21](tools/research_index/research_index/handoff.py),
  [brief.py:13](tools/research_index/research_index/brief.py),
  [validation.py:12](tools/research_index/research_index/validation.py).
  All return JSON-serializable dicts/lists. `__init__.py` is empty. No
  `print()` calls in the library.
- **CLI shapes** verified by reading every `tools/research_index/*.py` shim
  ([related.py](tools/research_index/related.py),
  [graph.py](tools/research_index/graph.py),
  [map.py](tools/research_index/map.py),
  [handoff.py](tools/research_index/handoff.py),
  [brief.py](tools/research_index/brief.py),
  [validate.py](tools/research_index/validate.py),
  [index.py](tools/research_index/index.py)). MCP param names mirror CLI flags
  for muscle-memory continuity.

**Local-only deliverable note (load-bearing):** Per
[`project_local_only_paths` memory](<local>/.claude/projects/<claude-project>/memory/project_local_only_paths.md),
all Phase 2 paths are gitignored — `tools/research_index/` (entire directory,
including its `tests/` and `research_index/` library), `.mcp.json`,
and `docs/plans/*` (anything in `docs/` except `_config.yml` and `index.md`).
**No `git add` / `git commit` steps appear in this plan.** Edits land
locally; `git status` will not show them tracked; `git restore` will fail.
This is intentional and matches Phase 1's actual behavior (the Phase 1
plan's commit commands were refused by `git add` on the gitignored paths
with the standard `"The following paths are ignored by one of your
.gitignore files... Use -f if you really want to add them"` message — not
silent no-ops, but harmless to the deliverable).

**What's still unknown after grounding:** Nothing material. Three minor
deferred items:

- Whether `@mcp.tool()`-decorated functions are callable in-process from the
  smoke tests. The decorator registers the tool with FastMCP but returns the
  function unchanged for direct calls; the smoke tests rely on this. If
  FastMCP wraps the function in something non-callable, fall back to invoking
  the underlying library functions and only assert that the *server module*
  imports cleanly.
- Whether `.cache/research.db` exists at the time the smoke tests run.
  Mitigated by `setUpClass` skip-if-missing guard.
- The exact `format_*` rendering of empty inner fields for `graph`, `map`,
  `handoff`, `brief`, `validate` — they already render "0 documents matched"
  style headers per design tiny-detail ledger, so smoke tests assert
  non-empty *string* rather than non-empty *content*.

---

## Key Technical Decisions

- **`rebuild_index` lives in new `research_index/indexing.py`, not in
  `database.py`** — **Confidence:** high
  - **Source:** Keeps the orchestration loop (iterate roots → chunk → build
    tuples → `rebuild_database`) separate from the database-writing
    primitive. Database module already owns the SQL primitive; the
    orchestration is a layer up. Same separation the existing
    `chunking.py` / `metadata.py` / `database.py` split already implies.
- **`indexing.rebuild_index(workspace, roots, db_path) -> str`** — **Confidence:** high
  - **Source:** Direct extraction from `index.py:main:24-45`. Returns the
    existing 1-line summary so both CLI and MCP get the same confirmation
    string. Tiny-detail ledger requirement (design line 111-113).
- **`DEFAULT_ROOTS` moves to `indexing.py`, re-imported by `index.py`** —
  **Confidence:** high
  - **Source:** Both the CLI and the new `research_reindex` need the same
    constant. Owning it next to `rebuild_index` keeps the shim trivial.
- **Smoke tests skip when `.cache/research.db` missing** — **Confidence:** high
  - **Source:** Design Testing Strategy explicitly uses the live DB
    ("invoke each tool's wrapper function with a known-good query against
    the live `.cache/research.db`"). A skip-if-missing guard keeps the
    rest of the test file runnable in a clean checkout.
- **Empty-result hint applies ONLY to `research_search` and
  `research_related`** — **Confidence:** high
  - **Source:** Design tiny-detail ledger line 121-127. Dict-returning tools
    always return a populated outer dict; their `format_*` renderers already
    show "0 documents matched" headers. Adding a hint on top would be
    duplicate / inconsistent.
- **CLI default limit override applies in exactly two places** —
  **Confidence:** high
  - **Source:** Design tiny-detail ledger line 114-120. Library
    `database.search` default 10 → CLI/MCP 20 (already in Phase 1).
    Library `graph.backlinks` default 20 → CLI/MCP 12 across all four graph
    modes. Every other library default already matches the CLI; no other
    overrides needed.
- **`research_brief` docstring leads with "heavy"** — **Confidence:** high
  - **Source:** Design tiny-detail ledger line 128-132. Fan-out aggregator
    runs `validate_index` + `system_map` + `parity_handoff` + N×
    (`evidence_view` + `implementation_view`). Docstring must steer agents
    to `research_search` or `research_handoff` first.
- **Empty-result test queries generated at runtime via `secrets.token_hex(16)`**
  — **Confidence:** high
  - **Source:** Phase 1 finding (design line 357-361 and Phase 1 plan Task 4
    Step 3 Row B). A hard-coded miss string would get indexed (because
    `docs/plans/` is indexed) and self-poison the test on the next reindex.
- **No `git add` / `git commit` steps anywhere in this plan** —
  **Confidence:** high
  - **Source:** [`project_local_only_paths` memory](<local>/.claude/projects/<claude-project>/memory/project_local_only_paths.md);
    all Phase 2 paths are gitignored.

## Open Questions

### Resolved During Planning

- *Where does `rebuild_index` live?* → New file
  `tools/research_index/research_index/indexing.py`.
- *Does `index.py` shrink to a one-import shim or keep arg parsing?* → Keeps
  its `argparse` block (positional `roots`, `--db`); only the body changes
  to call `rebuild_index(...)` and print the returned string.
- *How is the empty-results hint phrased for `research_related`?* → Mirrors
  the `research_search` hint in
  [tools/research_index/mcp_server.py:70-80](tools/research_index/mcp_server.py:70):
  "No related docs for {target!r}{filter_suffix}. Try ..."
- *Should graph dispatch be one tool with `mode` enum or four tools?* →
  One tool with `mode: Literal["doc", "backlinks", "evidence",
  "implementation"]`. Design line 426-428 rejected the four-tool variant.
- *How does `research_reindex` learn the workspace?* → Uses the
  module-level `WORKSPACE = _SERVER_DIR.parents[1]` already established in
  Phase 1 ([mcp_server.py:42](tools/research_index/mcp_server.py:42)).

### Deferred to Implementation

- **In-process callability of `@mcp.tool()` functions** — verify at Task 9
  Step 2. Expected: the decorator returns the function unchanged; direct
  calls work. Fallback if not: the smoke tests assert library output
  equivalence instead, plus a single "module imports cleanly" check.
- **Live `.cache/research.db` availability during tests** — `setUpClass`
  raises `unittest.SkipTest` if missing. Implementation-time only — no
  upstream change.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `tools/research_index/research_index/indexing.py` | `rebuild_index(workspace, roots, db_path) -> str` + `DEFAULT_ROOTS` |
| Modify | `tools/research_index/index.py` | Shim calling `rebuild_index` (replaces inline loop in `main`) |
| Modify | `tools/research_index/mcp_server.py` | Add 7 `@mcp.tool()` functions; +imports |
| Modify | `tools/research_index/tests/test_research_index.py` | Add `MCPServerSmokeTests` class |

No changes to `.mcp.json` — Phase 1 already wired the server entry.

## Interface Changes

- **New library symbol:** `research_index.indexing.rebuild_index(workspace:
  Path, roots: list[Path], db_path: Path) -> str` and
  `research_index.indexing.DEFAULT_ROOTS: tuple[str, ...]`. Consumed by
  `index.py` (CLI) and the new `research_reindex` MCP tool.
- **No breaking changes** to existing library entry points, CLIs, or DB
  schema. `index.py`'s CLI signature (`roots... --db`) is preserved.
- **MCP tool surface expands from 1 → 8.** Tool name prefix
  `mcp__research-index__research_*`. Catalog parallels
  `mcp__codegraph__codegraph_*` (intentional family naming).

## Sim Checklist

**N/A** — Phase 2 touches no Rust code, no `sim/`, no game state, no
determinism surface. Pure tooling.

## Risk Areas

- **Library refactor (Task 1)** — `index.py` is the only existing consumer
  of the orchestration loop. After Task 1, run `python tools/research_index/index.py`
  end-to-end and confirm the indexed document count matches the pre-refactor
  count. If the count changes, the extraction has a bug (most likely a
  `relpath` or `workspace` capture mismatch).
- **Tool name collisions** — None expected (`research_*` prefix vs
  `codegraph_*`, design line 140-141). Confirmed at Phase 1.
- **Empty-result hint scope drift** — Easy to forget which tools take the
  hint. Each task's Step 2 says explicitly "no empty-result hint" or "yes,
  empty-result hint" before showing code.
- **`research_reindex` race with concurrent `research_search`** — Mitigated
  by `rebuild_database` writing to a `.tmp` file and atomic
  [database.py:48](tools/research_index/research_index/database.py:48)
  `os.replace`. No locking added.

## Parity-Critical Items

**N/A** — Phase 2 is pure tooling. No player-observable behavior, no
gamemd.exe parity stakes, no draw composition, no input/audio/render path.
The "parity bar" does not apply.

---

## Tasks

### Task 1: Extract `rebuild_index` library helper

**Why:** `research_reindex` (Task 8) needs a stable callable entry point that
returns the 1-line summary. Without this extraction, the MCP tool would
duplicate the 15-line orchestration loop from `index.py:main`, and the two
copies would drift over time. Lands first so every later task can assume the
helper exists.

**Files:**
- Create: `tools/research_index/research_index/indexing.py`
- Modify: `tools/research_index/index.py`

**Pattern:** New module sibling to existing
[chunking.py](tools/research_index/research_index/chunking.py),
[metadata.py](tools/research_index/research_index/metadata.py),
[database.py](tools/research_index/research_index/database.py). Plain
`from __future__ import annotations` header, no class wrapping needed —
single top-level function.

**Step 1: Create the new module**

Create file `tools/research_index/research_index/indexing.py` with this
exact content:

```python
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
```

**Step 2: Convert `index.py` to a shim**

Replace the entire contents of `tools/research_index/index.py` with:

```python
#!/usr/bin/env python3
"""Build the local VERA20k research index (CLI shim over indexing.rebuild_index)."""

from __future__ import annotations

import argparse
from pathlib import Path

from research_index.database import DEFAULT_DB
from research_index.indexing import DEFAULT_ROOTS, rebuild_index


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the VERA20k research SQLite FTS index.")
    parser.add_argument("roots", nargs="*", default=list(DEFAULT_ROOTS), help="Roots to index")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Output SQLite database path")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace = Path.cwd()
    roots = [workspace / root for root in args.roots]
    db_path = Path(args.db)
    print(rebuild_index(workspace, roots, db_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

**Step 3: Verify the import chain resolves**

From the repo root, run:

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.indexing import DEFAULT_ROOTS, rebuild_index; print('OK', DEFAULT_ROOTS)"
```

Expected output: `OK ('docs/research', 'docs/plans', 'ini')`.

If `ImportError` appears, re-check that Step 1 saved to
`tools/research_index/research_index/indexing.py` (note the doubled
directory — the inner `research_index/` is the package, the outer is the
tool directory).

**Step 4: Run the CLI end-to-end and confirm output unchanged**

From the repo root:

```powershell
python tools/research_index/index.py
```

Expected: prints `indexed documents=N chunks=M db=<path>` exactly once
(no other stdout). Record `N` and `M`. If the script raises an exception,
the refactor has a bug.

**Step 5: Compare document count to pre-refactor baseline**

Run a quick search to confirm the DB is queryable:

```powershell
python tools/research_index/search.py "BridgeRepairHut" --limit 3
```

Expected: at least one hit under `docs/research/bridges/`. If empty, the
rebuild silently dropped documents — check `iter_indexable_files(roots)`
walked the expected roots (the most common bug is forgetting to join
`workspace / root`, which is preserved verbatim from the old code).

---

### Task 2: Add `research_related` MCP tool

**Why:** Mirrors the Phase 1 `research_search` shape on a list-returning
library function with two dispatch branches (`by="doc"` vs `by="term"`).
Second most-used surface after search; this is also the only other tool
that needs the empty-result hint.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1's `research_search` at
[mcp_server.py:47-84](tools/research_index/mcp_server.py:47). Empty-result
hint **YES**.

**Step 1: Add library imports**

In `tools/research_index/mcp_server.py`, find the existing import block:

```python
from research_index.database import DEFAULT_DB, search
from research_index.formatting import format_search_results
```

Replace it with:

```python
from research_index.database import (
    DEFAULT_DB,
    related_by_document,
    related_by_term,
    search,
)
from research_index.formatting import (
    format_related_results,
    format_search_results,
)
```

**Step 2: Register the tool**

In the same file, append the following block immediately after the
`research_search` function (after the closing `return format_search_results(rows)`
line, before the `if __name__ == "__main__":` block):

```python
@mcp.tool()
def research_related(
    target: str,
    by: Literal["doc", "term"] = "doc",
    limit: int = 20,
    format: Literal["text", "json"] = "text",
) -> str:
    """Docs sharing extracted evidence (symbols, addresses, INI keys, Rust paths) with a source.

    Use when you have one doc and want others that cite the same symbols,
    or when you want every doc that mentions an exact term. For free-text
    search use research_search; for graph adjacency by edge kind use
    research_graph.

    Args:
        target: Document path (when by="doc") or exact extracted term
            (when by="term"). Document paths must be repo-relative
            (e.g. "docs/research/bridges/BRIDGE_C4_GHIDRA_REPORT.md").
        by: "doc" to find docs related to a document; "term" to find docs
            referencing an exact extracted term.
        limit: Max results (default 20, matches related.py CLI).
        format: "text" for formatted output, "json" for structured.
    """
    if by == "term":
        rows = related_by_term(DEFAULT_DB, target, limit)
    else:
        rows = related_by_document(DEFAULT_DB, target, limit)

    if not rows:
        return (
            f"No related docs for {target!r} (by={by!r}). "
            f"Confirm the path or term spelling; try research_search for "
            f"free-text lookup or research_graph for adjacency."
        )

    if format == "json":
        return json.dumps(rows, indent=2)
    return format_related_results(rows)
```

**Step 3: Re-verify the server imports cleanly**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`. If the import fails, the most likely cause is a typo in
the new imports; re-read Step 1.

**Step 4: Sanity-check the library wiring**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB, related_by_term; from research_index.formatting import format_related_results; print(format_related_results(related_by_term(DEFAULT_DB, 'BridgeRepairHut', 3)))"
```

Expected: a non-empty block. If empty, the term may not be in the index;
fall back to a term you know exists (e.g. confirm via
`python tools/research_index/search.py "BridgeRepairHut"` first).

---

### Task 3: Add `research_graph` MCP tool

**Why:** Single tool with `mode` enum dispatching to four library functions.
Validates that `Literal[four values]` works in a real schema (Phase 1 only
exercised the two-value `Literal["text", "json"]`). Returns `dict`; no
empty-result hint per design tiny-detail ledger.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1's `research_search` body shape, minus the
empty-result hint, plus a mode-dispatch ladder. Mirrors
[graph.py:32-44](tools/research_index/graph.py:32) CLI dispatch logic
verbatim.

**Step 1: Extend imports**

In `tools/research_index/mcp_server.py`, find the imports added in Task 2.
Add these two import groups beneath them:

```python
from research_index.graph import (
    backlinks,
    document_graph,
    evidence_view,
    implementation_view,
)
from research_index.formatting import (
    format_backlinks,
    format_document_graph,
    format_graph_view,
)
```

Note: `format_graph_view` covers both `evidence` and `implementation` modes
(verified at [graph.py:42,44](tools/research_index/graph.py:42) — both call
`format_graph_view`). Do **not** create a separate formatter import for
implementation; it does not exist.

**Step 2: Register the tool**

Append after `research_related`:

```python
@mcp.tool()
def research_graph(
    mode: Literal["doc", "backlinks", "evidence", "implementation"],
    target: str,
    limit: int = 12,
    format: Literal["text", "json"] = "text",
) -> str:
    """Navigate the research docgraph.

    Four modes that resolve different edge kinds:
    - "doc": outgoing edges from a document (what it cites).
    - "backlinks": incoming edges to a document (who cites it).
    - "evidence": resolve an exact term to docs that cite it as evidence.
    - "implementation": resolve an exact term to Rust paths cited near it.

    Args:
        mode: One of "doc", "backlinks", "evidence", "implementation".
        target: Repo-relative document path for "doc"/"backlinks"; exact
            term for "evidence"/"implementation".
        limit: Max rows per section (default 12, matches graph.py CLI;
            note: library default for backlinks is 20, MCP overrides to 12
            for parity with the CLI).
        format: "text" for formatted output, "json" for structured.
    """
    if mode == "doc":
        result = document_graph(DEFAULT_DB, target, limit)
        text = format_document_graph(result)
    elif mode == "backlinks":
        result = backlinks(DEFAULT_DB, target, limit)
        text = format_backlinks(result)
    elif mode == "evidence":
        result = evidence_view(DEFAULT_DB, target, limit)
        text = format_graph_view(result)
    else:  # implementation
        result = implementation_view(DEFAULT_DB, target, limit)
        text = format_graph_view(result)

    if format == "json":
        return json.dumps(result, indent=2)
    return text
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check one library call**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.graph import evidence_view; from research_index.formatting import format_graph_view; print(format_graph_view(evidence_view(DEFAULT_DB, 'BridgeRepairHut', 3)))"
```

Expected: a non-empty block beginning with a header. If empty, the term
may not be indexed; confirm via search first.

---

### Task 4: Add `research_map` MCP tool

**Why:** Inventory tool for surveying what's already researched in a system
or topic. Five optional filter params (topic, system, source, status,
limit). Returns `dict`; no empty-result hint — `format_system_map` already
handles emptiness with a "0 documents matched" header.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1 body shape minus empty-result hint. Mirrors
[map.py:32-41](tools/research_index/map.py:32) verbatim.

**Step 1: Extend imports**

In `tools/research_index/mcp_server.py`, extend the existing
`from research_index.formatting import (...)` block to include
`format_system_map`, and add a new import line for `system_map`:

```python
from research_index.formatting import (
    format_backlinks,
    format_document_graph,
    format_graph_view,
    format_related_results,
    format_search_results,
    format_system_map,
)
from research_index.system_map import system_map
```

(Replace the existing `format_*` block with the expanded one above to
keep imports sorted; do not duplicate the imports already present.)

**Step 2: Register the tool**

Append after `research_graph`:

```python
@mcp.tool()
def research_map(
    topic: str | None = None,
    system: str | None = None,
    source: str | None = None,
    status: str | None = None,
    limit: int = 80,
    format: Literal["text", "json"] = "text",
) -> str:
    """Inventory of docs for a system or topic, grouped by subsystem/source/status.

    Use to survey what is already researched before starting an investigation;
    returns groups, document counts, handoff sections, and uncertainty
    signals. For free-text lookup use research_search; for the
    implementation-handoff bundle use research_handoff.

    Args:
        topic: Optional topic phrase matched in paths, titles, headings, or
            chunks. Omit to list every doc in the (optionally filtered)
            scope.
        system: Filter by inferred system (e.g. "bridges", "miner", "chrono").
        source: Filter by source kind (e.g. "ghidra", "trace", "synthesis").
        status: Filter by status (e.g. "verified", "stale", "unknown").
        limit: Max rows per section (default 80, matches map.py CLI).
        format: "text" for formatted output, "json" for structured.
    """
    result = system_map(
        DEFAULT_DB,
        system=system,
        topic=topic,
        source_kind=source,
        status=status,
        limit=limit,
    )
    if format == "json":
        return json.dumps(result, indent=2)
    return format_system_map(result)
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check the library wiring**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.system_map import system_map; from research_index.formatting import format_system_map; print(format_system_map(system_map(DEFAULT_DB, system='bridges', limit=5)))"
```

Expected: a non-empty block with at least one group section.

---

### Task 5: Add `research_handoff` MCP tool

**Why:** Implementation-handoff bundle for a mechanism or symbol — returns
handoff sections + top evidence + Rust touchpoints in one call. Returns
`dict`; no empty-result hint — `format_parity_handoff` renders emptiness.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1 body shape minus empty-result hint. Mirrors
[handoff.py:31-33](tools/research_index/handoff.py:31) verbatim.

**Step 1: Extend imports**

Add to the existing `formatting` import group (preserve sorted order):

```python
from research_index.formatting import (
    format_backlinks,
    format_document_graph,
    format_graph_view,
    format_parity_handoff,
    format_related_results,
    format_search_results,
    format_system_map,
)
```

Add a new import line:

```python
from research_index.handoff import parity_handoff
```

**Step 2: Register the tool**

Append after `research_map`:

```python
@mcp.tool()
def research_handoff(
    query: str,
    system: str | None = None,
    source: str | None = None,
    limit: int = 8,
    format: Literal["text", "json"] = "text",
) -> str:
    """Implementation-handoff bundle for a mechanism or symbol.

    Returns the handoff sections, top evidence rows, and Rust touchpoints
    for a query in one call. Use when planning an implementation and you
    want the "what does gamemd do, what Rust does it map to" view. For a
    bigger pre-implementation aggregate (validation + map + handoff +
    anchors) use research_brief.

    Args:
        query: Mechanism, symbol, doc topic, or implementation question.
        system: Filter evidence and handoff sections by inferred system.
        source: Filter evidence and handoff sections by source kind.
        limit: Max rows per section (default 8, matches handoff.py CLI).
        format: "text" for formatted output, "json" for structured.
    """
    result = parity_handoff(
        DEFAULT_DB,
        query,
        limit=limit,
        system=system,
        source_kind=source,
    )
    if format == "json":
        return json.dumps(result, indent=2)
    return format_parity_handoff(result)
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check the library wiring**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.handoff import parity_handoff; from research_index.formatting import format_parity_handoff; print(format_parity_handoff(parity_handoff(DEFAULT_DB, 'BridgeRepairHut C4', limit=4)))"
```

Expected: a non-empty block including the "Implementation handoff
candidates:" or "Rust touchpoints:" header.

---

### Task 6: Add `research_validate` MCP tool

**Why:** Stale-index and broken-link check. Uses the module-level
`WORKSPACE` constant (the design's tiny-detail ledger calls this out:
do not trust `Path.cwd()`). Returns `dict`; no empty-result hint —
`format_validation` renders zero-issue states.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1 body shape minus empty-result hint, plus a
`WORKSPACE` arg passed to the library function. Mirrors
[validate.py:33-43](tools/research_index/validate.py:33) verbatim.

**Step 1: Extend imports**

Add to the formatting group:

```python
from research_index.formatting import (
    format_backlinks,
    format_document_graph,
    format_graph_view,
    format_parity_handoff,
    format_related_results,
    format_search_results,
    format_system_map,
    format_validation,
)
```

Add a new import line:

```python
from research_index.validation import validate_index
```

**Step 2: Register the tool**

Append after `research_handoff`:

```python
@mcp.tool()
def research_validate(
    topic: str | None = None,
    system: str | None = None,
    source: str | None = None,
    status: str | None = None,
    limit: int = 40,
    format: Literal["text", "json"] = "text",
) -> str:
    """Check the index against current files; report stale chunks and broken links.

    Returns missing files, checksum mismatches (docs changed since
    indexing), missing markdown link targets, and stale/unknown-status
    docs. Use before relying on the index for a high-stakes lookup, or
    after a doc rewrite to confirm the index is fresh.

    Args:
        topic: Optional topic phrase to validate within.
        system: Filter by inferred system.
        source: Filter by source kind.
        status: Filter by status.
        limit: Max issue rows per section (default 40, matches validate.py
            CLI).
        format: "text" for formatted output, "json" for structured.
    """
    result = validate_index(
        DEFAULT_DB,
        WORKSPACE,
        system=system,
        topic=topic,
        source_kind=source,
        status=status,
        limit=limit,
    )
    if format == "json":
        return json.dumps(result, indent=2)
    return format_validation(result)
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check the library wiring (uses repo root as workspace)**

```powershell
python -c "import sys; from pathlib import Path; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.validation import validate_index; from research_index.formatting import format_validation; print(format_validation(validate_index(DEFAULT_DB, Path.cwd(), system='bridges', limit=5)))"
```

Expected: a non-empty block. Validation may report missing files /
checksum mismatches if any docs have changed since the last
`python tools/research_index/index.py` run — that is correct behavior,
not a tool bug.

---

### Task 7: Add `research_brief` MCP tool

**Why:** Heavy aggregator — internally invokes `validate_index` +
`system_map` + `parity_handoff` + N × (`evidence_view` +
`implementation_view`). Docstring must signal "use search or handoff first
for cheap lookups." Comes after the lighter tools so an executor reading
the file top-to-bottom sees the cheap ones first.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1 body shape minus empty-result hint, plus
`WORKSPACE` and `anchors` normalization. Mirrors
[brief.py:34-43](tools/research_index/brief.py:34) verbatim.

**Step 1: Extend imports**

Add to the formatting group:

```python
from research_index.formatting import (
    format_backlinks,
    format_document_graph,
    format_graph_view,
    format_parity_handoff,
    format_related_results,
    format_research_brief,
    format_search_results,
    format_system_map,
    format_validation,
)
```

Add a new import line:

```python
from research_index.brief import research_brief as _research_brief_lib
```

The `as _research_brief_lib` rename avoids a name clash with the MCP tool
function declared below (which is decorated as `@mcp.tool()` and exported
under the name `research_brief`).

**Step 2: Register the tool**

Append after `research_validate`:

```python
@mcp.tool()
def research_brief(
    query: str,
    anchors: list[str] | None = None,
    system: str | None = None,
    source: str | None = None,
    limit: int = 8,
    format: Literal["text", "json"] = "text",
) -> str:
    """Compact pre-implementation planning bundle. HEAVY — prefer cheaper tools first.

    Aggregates validation + system map + parity handoff + per-anchor
    evidence/implementation views in one call. Issues 5+ SQLite
    connections per invocation. Use only when assembling a full
    pre-implementation context block. For a quick lookup use
    research_search; for an implementation-only handoff use
    research_handoff; for a system inventory use research_map.

    Args:
        query: System topic, mechanism, function, or implementation
            question.
        anchors: Optional exact symbols or addresses to pin (each gets a
            per-anchor evidence + implementation view appended). Defaults
            to an empty list when omitted.
        system: Filter by inferred system.
        source: Filter by source kind.
        limit: Max rows per section (default 8, matches brief.py CLI).
        format: "text" for formatted output, "json" for structured.
    """
    anchors_list = anchors if anchors else []
    result = _research_brief_lib(
        DEFAULT_DB,
        WORKSPACE,
        query,
        system=system,
        source_kind=source,
        anchors=anchors_list,
        limit=limit,
    )
    if format == "json":
        return json.dumps(result, indent=2)
    return format_research_brief(result)
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check the library wiring**

```powershell
python -c "import sys; from pathlib import Path; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.brief import research_brief; from research_index.formatting import format_research_brief; print(format_research_brief(research_brief(DEFAULT_DB, Path.cwd(), 'BridgeRepairHut', limit=4)))"
```

Expected: a non-empty block headed "Pre-implementation brief:". May take
1-3 seconds because of the fan-out — this is normal.

---

### Task 8: Add `research_reindex` MCP tool

**Why:** The only mutating tool. Wraps `indexing.rebuild_index` from
Task 1. Comes last so all the read tools land first — if `research_reindex`
breaks the DB partway through, the read tools were already smoke-tested
against the old (working) DB.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Phase 1 skeleton plus a small input-normalization block.
No empty-result hint (always returns the summary string). Mirrors
[index.py:24-45](tools/research_index/index.py:24) (post-Task-1 shim
form) verbatim.

**Step 1: Extend imports**

Add a new import line:

```python
from research_index.indexing import DEFAULT_ROOTS, rebuild_index
```

**Step 2: Register the tool**

Append after `research_brief`:

```python
@mcp.tool()
def research_reindex(
    roots: list[str] | None = None,
) -> str:
    """Rebuild the FTS index from disk.

    Walks the given roots (markdown + ini files), chunks them, and writes
    a fresh SQLite DB atomically (tmp file + os.replace). Concurrent
    research_search calls during a reindex see the previous DB until the
    swap. Use after large doc rewrites, after pulling new research, or
    after research_validate reports widespread checksum mismatches.

    Args:
        roots: Optional repo-relative paths to walk. Defaults to
            ("docs/research", "docs/plans", "ini") when omitted.

    Returns:
        One-line summary: ``indexed documents=N chunks=M db=<path>``.
    """
    root_strs = roots if roots else list(DEFAULT_ROOTS)
    root_paths = [WORKSPACE / root for root in root_strs]
    return rebuild_index(WORKSPACE, root_paths, DEFAULT_DB)
```

**Step 3: Re-verify imports**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`.

**Step 4: Sanity-check by calling the underlying helper directly**

This sanity check exercises `rebuild_index` from the workspace path the
MCP tool will use:

```powershell
python -c "import sys; from pathlib import Path; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB; from research_index.indexing import DEFAULT_ROOTS, rebuild_index; workspace = Path.cwd(); roots = [workspace / r for r in DEFAULT_ROOTS]; print(rebuild_index(workspace, roots, DEFAULT_DB))"
```

Expected: prints `indexed documents=N chunks=M db=<path>` with the same
counts you recorded in Task 1 Step 4. If the count differs significantly,
something has changed in `docs/` since Task 1 — that's expected if other
work has been happening in parallel.

---

### Task 9: Add smoke tests in `tests/test_research_index.py`

**Why:** One assertion per tool against the live `.cache/research.db`,
plus empty-result hint tests for the two list-returning tools. Catches
regressions if any MCP tool's library dispatch or formatter wiring
breaks. Skip-if-missing guard keeps the file runnable in a clean
checkout.

**Files:**
- Modify: `tools/research_index/tests/test_research_index.py`

**Pattern:** Existing test classes in the file use `tempfile.TemporaryDirectory`
+ inline DB builds. The MCP smoke tests are different — they use the live
DB. New class isolates that pattern.

**Step 1: Add module-level imports (top of the file, after existing imports)**

Find the end of the existing import block in `test_research_index.py`
(the last existing line is
`from research_index.validation import validate_index`). Add this single
new line immediately after:

```python
import json
import secrets
```

Do **not** import `mcp_server` at module scope. The `mcp_server` module
imports `from mcp.server.fastmcp import FastMCP`, which would cascade an
`ImportError` to the entire test file if the optional `mcp` package
isn't installed in the active env — taking down all 12 existing tests
alongside the new ones. Defer the import to `setUpClass` (Step 2) so the
new smoke tests skip gracefully without breaking the rest.

**Step 2: Append the new test class**

Append after the existing `_row` helper function (the last function in
the file before `if __name__ == "__main__":`):

```python
class MCPServerSmokeTests(unittest.TestCase):
    """Smoke tests for the FastMCP tool wrappers in mcp_server.

    Use the live .cache/research.db rather than an inline tempdir build,
    because the design's testing strategy specifies it (production-shaped
    coverage) and several tools (research_brief, research_validate)
    exercise the workspace argument against the real repo. Skipped when
    the live DB is missing.
    """

    mcp_server = None  # populated by setUpClass when imports succeed

    @classmethod
    def setUpClass(cls) -> None:
        from research_index.database import DEFAULT_DB

        if not DEFAULT_DB.exists():
            raise unittest.SkipTest(
                f"Live research index not built at {DEFAULT_DB}; "
                f"run `python tools/research_index/index.py` first."
            )

        try:
            import mcp_server  # noqa: E402  (TOOL_ROOT is already on sys.path)
        except ImportError as exc:
            raise unittest.SkipTest(f"mcp_server not importable: {exc}") from exc

        cls.mcp_server = mcp_server

    def test_research_search_returns_text(self) -> None:
        out = self.mcp_server.research_search(query="BridgeRepairHut", limit=3)
        self.assertIn("docs/research/bridges", out)

    def test_research_search_json_round_trips(self) -> None:
        out = self.mcp_server.research_search(query="BridgeRepairHut", limit=3, format="json")
        rows = json.loads(out)
        self.assertIsInstance(rows, list)

    def test_research_search_empty_result_returns_hint(self) -> None:
        # Runtime-generated query guarantees no index entry. A hard-coded
        # miss string would be indexed when docs/plans/ is reindexed,
        # turning this assertion into a self-reference hit (Phase 1
        # finding).
        miss_query = secrets.token_hex(16)
        out = self.mcp_server.research_search(query=miss_query)
        self.assertTrue(out.startswith("No results for"))

    def test_research_related_returns_text(self) -> None:
        out = self.mcp_server.research_related(target="BridgeRepairHut", by="term", limit=3)
        self.assertNotEqual(out.strip(), "")

    def test_research_related_empty_result_returns_hint(self) -> None:
        miss_term = secrets.token_hex(16)
        out = self.mcp_server.research_related(target=miss_term, by="term")
        self.assertTrue(out.startswith("No related docs for"))

    def test_research_graph_doc_mode_returns_text(self) -> None:
        out = self.mcp_server.research_graph(
            mode="evidence",
            target="BridgeRepairHut",
            limit=3,
        )
        self.assertNotEqual(out.strip(), "")

    def test_research_graph_json_round_trips(self) -> None:
        out = self.mcp_server.research_graph(
            mode="evidence",
            target="BridgeRepairHut",
            limit=3,
            format="json",
        )
        result = json.loads(out)
        self.assertIsInstance(result, dict)

    def test_research_map_returns_text(self) -> None:
        out = self.mcp_server.research_map(system="bridges", limit=5)
        self.assertNotEqual(out.strip(), "")

    def test_research_handoff_returns_text(self) -> None:
        out = self.mcp_server.research_handoff(query="BridgeRepairHut", limit=3)
        self.assertNotEqual(out.strip(), "")

    def test_research_validate_returns_text(self) -> None:
        out = self.mcp_server.research_validate(system="bridges", limit=5)
        self.assertNotEqual(out.strip(), "")

    def test_research_brief_returns_text(self) -> None:
        out = self.mcp_server.research_brief(query="BridgeRepairHut", limit=3)
        self.assertNotEqual(out.strip(), "")

    def test_research_brief_with_anchors_normalizes_none(self) -> None:
        # anchors=None should normalize to [] without exception.
        out = self.mcp_server.research_brief(query="BridgeRepairHut", anchors=None, limit=3)
        self.assertNotEqual(out.strip(), "")
```

**Step 3: Run the test file**

From the repo root:

```powershell
python -m unittest tools.research_index.tests.test_research_index -v
```

Expected outcomes (one of three, all acceptable):

- **All 12 existing tests pass + all 12 `MCPServerSmokeTests` pass.** This
  is the green-path outcome when `.cache/research.db` exists and `mcp`
  is installed.
- **All 12 existing tests pass + `MCPServerSmokeTests` skipped with
  "Live research index not built at ..."** — DB missing. Run
  `python tools/research_index/index.py`, then re-run.
- **All 12 existing tests pass + `MCPServerSmokeTests` skipped with
  "mcp_server not importable: ..."** — the `mcp` Python package isn't
  installed in this env. The existing tests are unaffected because
  `import mcp_server` only happens inside `setUpClass`.

If any new test **fails** (not skips), the corresponding MCP tool body
has a bug — re-read the matching task above and re-check the library
call signature.

**Step 4: Confirm `@mcp.tool()` callability assumption**

This is the design's one remaining unknown. The smoke tests call
`self.mcp_server.research_search(...)` etc. directly — that only works
if FastMCP's `@mcp.tool()` decorator returns the original function
unchanged (the standard FastMCP behavior).

If any smoke test fails with `TypeError: 'Tool' object is not callable`
(or similar), the decorator wraps the function in a non-callable object
in your installed FastMCP version. Fallback: call the underlying
library functions directly from the smoke tests instead of the MCP
wrappers, and add a single `test_mcp_server_imports_cleanly` that just
asserts `mcp_server.mcp is not None` to keep some coverage of the
server module itself.

If all smoke tests pass on the first run, this fallback is not needed
and Phase 1's implicit assumption (callable wrappers) holds for Phase 2
too.

---

### Task 10: Manual integration test in a restarted Claude Code session

**Why:** This is the actual Phase 2 acceptance gate. Until the seven new
tools round-trip through a real Claude Code MCP session, they are not
proven. Phase 1 already verified that `.mcp.json` relative paths,
`Literal`, `format` kwarg, and `stdout.reconfigure` all work — but each
new tool's docstring, schema, and dispatch path still needs to round-trip
through MCP at least once.

**Files:** (none modified in this task; verification + bug-fix-as-needed)

**Pattern:** N/A — manual checklist driven by user.

**Step 1: User restarts Claude Code**

The current Claude Code session caches the MCP server's tool list at
startup. Adding new `@mcp.tool()` decorators to `mcp_server.py` will not
appear in the running session. Ask the user to restart Claude Code
(`/exit` then relaunch). Confirm restart before continuing.

**Step 2: Verify all 8 tools are discoverable**

In the new session, the tools should appear as
`mcp__research-index__research_search`,
`mcp__research-index__research_related`,
`mcp__research-index__research_graph`,
`mcp__research-index__research_map`,
`mcp__research-index__research_handoff`,
`mcp__research-index__research_brief`,
`mcp__research-index__research_validate`,
`mcp__research-index__research_reindex`. Confirm visibility, or attempt
to call one and observe the tool list.

**Step 3: Round-trip each new tool against the live DB**

Invoke each tool with a known-good query and verify the output is
non-empty and substring-matches the CLI counterpart. Run both calls and
diff visually:

| # | MCP call | CLI counterpart |
|---|----------|-----------------|
| A | `research_related(target="BridgeRepairHut", by="term", limit=3)` | `python tools/research_index/related.py BridgeRepairHut --term --limit 3` |
| B | `research_graph(mode="evidence", target="BridgeRepairHut", limit=3)` | `python tools/research_index/graph.py evidence BridgeRepairHut --limit 3` |
| C | `research_map(system="bridges", limit=5)` | `python tools/research_index/map.py --system bridges --limit 5` |
| D | `research_handoff(query="BridgeRepairHut", limit=3)` | `python tools/research_index/handoff.py BridgeRepairHut --limit 3` |
| E | `research_validate(system="bridges", limit=5)` | `python tools/research_index/validate.py --system bridges --limit 5` |
| F | `research_brief(query="BridgeRepairHut", limit=3)` | `python tools/research_index/brief.py BridgeRepairHut --limit 3` |
| G | `research_reindex()` | `python tools/research_index/index.py` |

Pass criterion per row: the MCP output substring-matches the CLI output
(allowing differences in trailing newlines / wrapping). For row G, both
should print the same `indexed documents=N chunks=M db=<path>` line; the
counts must be identical (they index the same DB from the same workspace).

**Step 4: Confirm empty-result hints**

Generate a fresh random miss string at runtime (do NOT hard-code in this
plan — that would self-index next reindex):

- Run `research_search` with `query=<random 32-char hex>` (Claude can
  generate it with `secrets.token_hex(16)`). Verify the hint starts with
  `"No results for"`.
- Run `research_related` with `target=<random 32-char hex>, by="term"`.
  Verify the hint starts with `"No related docs for"`.

**Step 5: Confirm `Literal["doc", "backlinks", "evidence", "implementation"]`
schema works for `research_graph`**

This is the only design unknown not exercised by Phase 1 (Phase 1 only
used the two-value `Literal["text", "json"]`). Try each mode at least
once:

- `research_graph(mode="doc", target="docs/research/bridges/00-system-models/BRIDGE_SYSTEM.md", limit=3)`
- `research_graph(mode="backlinks", target="docs/research/bridges/00-system-models/BRIDGE_SYSTEM.md", limit=3)`
- `research_graph(mode="evidence", target="BridgeRepairHut", limit=3)`
- `research_graph(mode="implementation", target="BridgeRepairHut", limit=3)`

If any call errors with a schema validation message, FastMCP rejected the
four-value `Literal`. Fallback: change the param to `mode: str` with a
manual validation block at the top of `research_graph`:

```python
if mode not in ("doc", "backlinks", "evidence", "implementation"):
    return f"Invalid mode {mode!r}; expected doc, backlinks, evidence, or implementation."
```

Restart Claude Code and re-test. Record the fallback (if applied) in this
plan's "Phase 2 Exit Criteria" section below.

**Step 6: No commit step**

All Phase 2 deliverables are gitignored (see
[`project_local_only_paths` memory](<local>/.claude/projects/<claude-project>/memory/project_local_only_paths.md)).
The work stays in the local checkout. Do not attempt `git add` or
`git commit` — `git add` will refuse with "paths are ignored", or
silently no-op, depending on whether `-f` is passed. Neither is intended.

---

## Phase 2 Exit Criteria

Phase 2 is **done** when ALL of the following are true:

1. `python tools/research_index/index.py` runs end-to-end and prints the
   1-line summary (Task 1 Step 4 passes).
2. `python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"`
   succeeds after every task (Task 2-8 Step 3 each pass).
3. `python -m unittest tools.research_index.tests.test_research_index -v`
   passes all 14+ tests (12 existing + new MCPServerSmokeTests) — or
   `MCPServerSmokeTests` is skipped with the "Live research index not
   built" message (Task 9 Step 3 passes).
4. After Claude Code restart, all 8 tools appear as
   `mcp__research-index__research_*` and each MCP call in Task 10 Step 3
   substring-matches its CLI counterpart.
5. Empty-result hints work for `research_search` and `research_related`
   (Task 10 Step 4 passes).
6. `research_graph` mode dispatch works for all four `Literal` values
   (Task 10 Step 5 passes), OR the fallback `mode: str` + manual
   validation is recorded here:

   > _Fallback applied:_ (fill in YES + reason if `Literal[four values]`
   > rejected, otherwise leave blank)

When all six are true, Phase 2 ships. Both surfaces (CLI + MCP) now front
the research index; future work consumes whichever is closer to hand.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-26-research-index-mcp-server-design.md](docs/plans/2026-05-26-research-index-mcp-server-design.md)
- **Phase 1 plan (pattern source):** [docs/plans/2026-05-26-research-index-mcp-server-phase-1-plan.md](docs/plans/2026-05-26-research-index-mcp-server-phase-1-plan.md)
- **Phase 1 deliverable (body shape):** [tools/research_index/mcp_server.py:47-84](tools/research_index/mcp_server.py:47)
- **Library entry points exercised:**
  - [research_index/database.py:168](tools/research_index/research_index/database.py) — `search`
  - [research_index/database.py:238,253](tools/research_index/research_index/database.py) — `related_by_document`, `related_by_term`
  - [research_index/graph.py:29,79,103,107](tools/research_index/research_index/graph.py) — `document_graph`, `backlinks`, `evidence_view`, `implementation_view`
  - [research_index/system_map.py:24](tools/research_index/research_index/system_map.py) — `system_map`
  - [research_index/handoff.py:21](tools/research_index/research_index/handoff.py) — `parity_handoff`
  - [research_index/brief.py:13](tools/research_index/research_index/brief.py) — `research_brief`
  - [research_index/validation.py:12](tools/research_index/research_index/validation.py) — `validate_index`
  - [research_index/database.py:27](tools/research_index/research_index/database.py) — `rebuild_database` (consumed via new `indexing.rebuild_index`)
- **CLI counterparts (signature parity):**
  - [related.py](tools/research_index/related.py), [graph.py](tools/research_index/graph.py),
    [map.py](tools/research_index/map.py), [handoff.py](tools/research_index/handoff.py),
    [brief.py](tools/research_index/brief.py), [validate.py](tools/research_index/validate.py),
    [index.py](tools/research_index/index.py)
- **Local-only-paths constraint:** [`project_local_only_paths` memory](<local>/.claude/projects/<claude-project>/memory/project_local_only_paths.md)
- **`.gitignore` entries (verified):** lines 22-25 (`/docs/*`,
  `/docs/research/`, `!/docs/_config.yml`, `!/docs/index.md`), plus
  `.mcp.json` and `tools/research_index/`.
