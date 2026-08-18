# research_index MCP Server — Phase 1 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Code blocks are complete and ready to paste — do not summarize or interpret.

**Goal:** Expose `research_search` as a working MCP tool end-to-end, validating
the FastMCP pattern + library-import wiring + `.mcp.json` registration on the
smallest viable surface before replicating across the remaining 7 tools in Phase 2.

**Architecture:** New file `tools/research_index/mcp_server.py` imports library
entry points directly from `research_index/` and exposes them via FastMCP
`@mcp.tool()` decorators. No library modifications in Phase 1 — the
`rebuild_index` extraction is a Phase 2 prerequisite (needed for
`research_reindex`) and is not required to validate any of the four Phase 1
unknowns.

**Design Doc:** [docs/plans/2026-05-26-research-index-mcp-server-design.md](docs/plans/2026-05-26-research-index-mcp-server-design.md)

---

## Grounding Summary

This is tooling/infrastructure work, not gameplay parity work. The RA2-specific
grounding sections of the write-plan skill (docs/research/, Ghidra MCP
verification, INI keys, gamemd.exe addresses) are **not applicable**. Grounding
for this plan comes from two sources, both already validated during the
brainstorm session:

- **`research_index/` library shape** verified by reading every entry-point
  function signature ([tools/research_index/research_index/database.py:168](tools/research_index/research_index/database.py),
  [tools/research_index/research_index/graph.py:29](tools/research_index/research_index/graph.py),
  [tools/research_index/research_index/system_map.py:24](tools/research_index/research_index/system_map.py),
  [tools/research_index/research_index/handoff.py:21](tools/research_index/research_index/handoff.py),
  [tools/research_index/research_index/brief.py:13](tools/research_index/research_index/brief.py),
  [tools/research_index/research_index/validation.py:12](tools/research_index/research_index/validation.py)).
  All entry points return JSON-serializable dicts/lists; library contains no
  `print()` calls; `__init__.py` is empty.
- **FastMCP pattern** from
  `<local>/Documents/ghidra-mcp/bridge_mcp_ghidra.py`: PEP 723 inline-deps
  header, `from mcp.server.fastmcp import FastMCP`, module-level
  `mcp = FastMCP("name")`, `@mcp.tool()` decorator on functions with typed
  parameters and `str` return, `mcp.run()` at `if __name__ == "__main__":`.
  Sync and async tools both work; type annotations accept `str | None = None`
  defaults.

**What's still unknown after grounding** (Phase 1's whole purpose is to resolve):

- Does FastMCP accept `Literal["text", "json"]` in a tool signature?
- Does `.mcp.json` accept a repo-relative path like
  `"tools/research_index/mcp_server.py"`, or does it require absolute
  (ghidra-mcp uses absolute)?
- Does `sys.stdout.reconfigure(encoding="utf-8")` conflict with FastMCP's
  ownership of stdout for the stdio transport?
- Does a kwarg named `format` (Python builtin shadow) round-trip cleanly
  through MCP, or does it need renaming to `output_format`?

---

## Key Technical Decisions

- **Library import, not subprocess** — **Confidence:** high
  - **Source:** Library is already cleanly modularized; every CLI is a 30-line
    shim. Direct import avoids ~200ms Python cold-start per call.
- **Server lives at `tools/research_index/mcp_server.py`** — **Confidence:** high
  - **Source:** Confirmed in design Q&A; sibling-to-CLIs minimizes `sys.path`
    setup (one line: insert `Path(__file__).parent`).
- **PEP 723 inline-deps header** — **Confidence:** high
  - **Source:** ghidra-mcp pattern at `bridge_mcp_ghidra.py:1-6`.
- **`Literal["text", "json"]` for format param** — **Confidence:** medium
  - **Source:** Pydantic supports it; FastMCP uses Pydantic under the hood;
    ghidra-mcp does not exercise this pattern so it's an inference. Fallback if
    it fails: plain `str` param with manual validation. **Phase 1 verifies.**
- **Relative path in `.mcp.json`** — **Confidence:** medium
  - **Source:** Claude Code launches MCP servers from the repo root; the
    existing `python tools/research_index/search.py` CLI invocation pattern in
    CLAUDE.md uses relative paths consistently. ghidra-mcp uses absolute, but
    that's because the script lives outside this repo. Fallback: switch to
    absolute path. **Phase 1 verifies.**
- **`stdout.reconfigure` before FastMCP import** — **Confidence:** medium
  - **Source:** Every CLI script does this safely; FastMCP stdio transport
    uses `sys.stdout.buffer` (binary) so the text-layer reconfigure should not
    collide. **Phase 1 verifies.**
- **`format` kwarg name (shadows builtin)** — **Confidence:** medium
  - **Source:** Python allows the shadow inside function scope; MCP transport
    doesn't care. Cosmetic concern only. Fallback: rename to `output_format`.
    **Phase 1 verifies.**

## Open Questions

### Resolved During Planning

- *Where does the server live?* → `tools/research_index/mcp_server.py`
  (confirmed by user in brainstorm Q&A).
- *How are reindex roots exposed?* → Optional `list[str]`, defaults to
  `["docs/research", "docs/plans", "ini"]` (confirmed by user). Phase 2
  detail; not exercised in Phase 1.

### Deferred to Implementation

- **FastMCP `Literal` support** — must verify by running the server. If FastMCP
  rejects `Literal["text", "json"]`, fall back to `str` + assertion check.
- **`.mcp.json` relative path resolution** — must verify by restarting Claude
  Code with the new entry. If the server fails to launch, switch to absolute
  path `tools/research_index/mcp_server.py`.
- **`stdout.reconfigure` + FastMCP stdio coexistence** — must verify the
  server doesn't emit garbled MCP frames during startup. If issue surfaces,
  remove the `reconfigure` line (research docs with non-ASCII may still render
  correctly if FastMCP wraps the JSON layer itself).
- **`format` kwarg compatibility** — verify the tool call works with
  `format="json"` as MCP param. If FastMCP errors, rename to `output_format`.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `tools/research_index/mcp_server.py` | FastMCP server, Phase 1 = skeleton + `research_search` tool only |
| Modify | `.mcp.json` | Add `"research-index"` server entry |

## Interface Changes

- **No library modifications.** Phase 1 imports existing entry points
  (`research_index.database.search`, `research_index.database.DEFAULT_DB`,
  `research_index.formatting.format_search_results`) without modifying any
  of them.
- **No breaking changes** to existing CLIs or the indexed DB.

## Sim Checklist

**N/A** — Phase 1 touches no Rust code, no `sim/`, no game state, no determinism
surface. It modifies Python tooling and one JSON config file.

## Risk Areas

- **MCP server fails to start** — surfaced by Phase 1's manual integration
  test (Task 4). All four "Deferred to Implementation" unknowns are checked
  here; resolution lands as Phase 1 deliverables.
- **Tight coupling to library internals** — mitigated by importing only
  documented top-level entry points (`search`, `format_search_results`,
  `DEFAULT_DB`). Phase 2 expands this surface; Phase 1 deliberately imports
  the minimum.

## Parity-Critical Items

**N/A** — This plan is pure tooling. No player-observable behavior, no
gamemd.exe parity stakes, no draw composition, no input/audio/render path
involved. The "parity bar" doesn't apply to a developer-facing MCP server.

---

## Tasks

### Task 1: Scaffold the MCP server (no tools yet)

**Why:** Land the minimum FastMCP skeleton — PEP 723 header, sys.path setup,
stdout reconfigure, `mcp = FastMCP("research-index")`, `mcp.run()`. No tools
registered. This separates "the server starts and accepts MCP frames" from
"the search tool works" — if the next task's tool registration breaks, we
know it's the tool not the server.

**Files:**
- Create: `tools/research_index/mcp_server.py`

**Pattern:** Mirrors `<local>/Documents/ghidra-mcp/bridge_mcp_ghidra.py:1-95`
for PEP 723 header, FastMCP import, and `if __name__ == "__main__"` block.

**Step 1: Create the server skeleton**

Create file `tools/research_index/mcp_server.py` with this exact content:

```python
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "mcp>=1.2.0,<2",
# ]
# ///
"""
research_index MCP Server — exposes the tools/research_index CLIs as MCP tools.

Library entry points are imported directly from research_index/. The CLI
scripts in this directory continue to work for shell invocations and CI.
"""

from __future__ import annotations

import json
import logging
import sys
from pathlib import Path
from typing import Literal

# Make the research_index package importable when launched by `python mcp_server.py`.
_SERVER_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(_SERVER_DIR))

# Reconfigure stdout to UTF-8 (matches the CLI shims; research docs contain non-ASCII).
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

from mcp.server.fastmcp import FastMCP

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("research-index-mcp")

# Repo root is two parents up from this file's directory.
WORKSPACE = _SERVER_DIR.parents[1]

mcp = FastMCP("research-index")


# (tools registered below in Task 2)


if __name__ == "__main__":
    logger.info(f"research-index MCP server starting (workspace={WORKSPACE})")
    mcp.run()
```

Note: `logging.basicConfig` writes to `sys.stderr`, not stdout — stdout is
reserved for MCP frames in stdio transport.

**Step 2: Verify the file imports cleanly**

From the repo root, run:

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected output: `OK`. If you see an `ImportError`, the most likely cause is
that the `mcp` package isn't installed in the current Python — install it
once with `pip install "mcp>=1.2.0,<2"` (or use `uv run` once dependencies
are pinned by the PEP 723 header).

**Step 3: Verify the server can start (stdio loop)**

From the repo root, run:

```powershell
python tools/research_index/mcp_server.py
```

Expected behavior: the server logs `research-index MCP server starting
(workspace=...)` to stderr and then blocks waiting on stdin for MCP frames.
Press Ctrl+C to exit. If it crashes immediately, re-read Step 1 and check the
import.

**Step 4: Commit**

```powershell
git add tools/research_index/mcp_server.py
git commit -m "feat: scaffold research-index MCP server"
```

---

### Task 2: Add `research_search` MCP tool

**Why:** First tool. Exercises the full library-call → empty-result-hint →
`format_*` text dispatch → JSON fallback path on the most-used surface. This
is the actual proof-of-pattern.

**Files:**
- Modify: `tools/research_index/mcp_server.py`

**Pattern:** Mirrors `bridge_mcp_ghidra.py:929` (`@mcp.tool()` decorator on a
sync function with typed params returning `str`).

**Step 1: Add library imports**

In `tools/research_index/mcp_server.py`, add these imports immediately after
the existing `from mcp.server.fastmcp import FastMCP` line:

```python
from research_index.database import DEFAULT_DB, search
from research_index.formatting import format_search_results
```

**Step 2: Register the tool**

In the same file, replace the line that reads
`# (tools registered below in Task 2)` with the following block:

```python
@mcp.tool()
def research_search(
    query: str,
    limit: int = 20,
    system: str | None = None,
    source: str | None = None,
    format: Literal["text", "json"] = "text",
) -> str:
    """Full-text search over docs/research/, docs/plans/, ini/ via SQLite FTS5.

    Use when looking up where a concept (function name, INI key, address,
    mechanism) is documented. For exploring docgraph adjacency or related
    docs, use research_related or research_graph instead (Phase 2).

    Args:
        query: FTS5 query string.
        limit: Max results (default 20, matches search.py CLI).
        system: Filter by inferred system (e.g. "bridges", "miner", "chrono").
        source: Filter by source kind (e.g. "ghidra", "trace", "synthesis").
        format: "text" for formatted output, "json" for structured.
    """
    rows = search(DEFAULT_DB, query, limit=limit, system=system, source_kind=source)

    if not rows:
        filter_suffix = ""
        if system or source:
            filter_suffix = f" (filters: system={system!r}, source={source!r})"
        return (
            f"No results for {query!r}{filter_suffix}. "
            f"Try a broader query, drop the filters, or use research_related "
            f"with by='term' once Phase 2 lands."
        )

    if format == "json":
        return json.dumps(rows, indent=2)
    return format_search_results(rows)
```

**Step 3: Re-verify the server imports cleanly**

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); import mcp_server; print('OK')"
```

Expected: `OK`. If `Literal` causes a Pydantic / FastMCP validation error at
import time, that's one of the Phase 1 unknowns surfacing. **Fallback:** change
the param to `format: str = "text"` and add this manual check at the top of
the function body:

```python
if format not in ("text", "json"):
    return f"Invalid format {format!r}; expected 'text' or 'json'."
```

Record the fallback in the task notes for Task 4.

**Step 4: Sanity-check the library call directly**

This is a fast pre-MCP test that verifies the library wiring without needing
to restart Claude Code:

```powershell
python -c "import sys; sys.path.insert(0, 'tools/research_index'); from research_index.database import DEFAULT_DB, search; from research_index.formatting import format_search_results; print(format_search_results(search(DEFAULT_DB, 'BridgeRepairHut', limit=3)))"
```

Expected: prints a search-results block including at least one path under
`docs/research/bridges/`. If the result is empty, the index may be stale —
run `python tools/research_index/index.py` and retry.

**Step 5: Commit**

```powershell
git add tools/research_index/mcp_server.py
git commit -m "feat: add research_search MCP tool"
```

---

### Task 3: Register `research-index` in `.mcp.json`

**Why:** Wire the new server into Claude Code's MCP launcher. Until this
lands, the tool is invisible to the agent.

**Files:**
- Modify: `.mcp.json`

**Pattern:** Mirrors the existing `ghidra-mcp` entry shape.

**Step 1: Replace `.mcp.json` contents**

Replace the entire contents of `.mcp.json` with:

```json
{
  "mcpServers": {
    "ghidra-mcp": {
      "command": "python",
      "args": [
        "<local>\\Downloads\\ghidra_12.0.4_PUBLIC_20260303\\ghidra_12.0.4_PUBLIC\\bridge_mcp_ghidra.py"
      ],
      "env": {
        "GHIDRA_SERVER_URL": "http://127.0.0.1:8089/"
      }
    },
    "research-index": {
      "command": "python",
      "args": [
        "tools/research_index/mcp_server.py"
      ]
    }
  }
}
```

**Step 2: Validate JSON**

```powershell
python -c "import json; json.load(open('.mcp.json'))"
```

Expected: no output (silent success). If a `JSONDecodeError` is raised, the
edit has a comma or brace issue — re-read Step 1.

**Step 3: Commit**

```powershell
git add .mcp.json
git commit -m "feat: register research-index MCP server in .mcp.json"
```

---

### Task 4: Manual integration test + resolve Phase 1 unknowns

**Why:** This is the actual Phase 1 gate. Until the MCP tool round-trips
successfully through a real Claude Code session, the four "Deferred to
Implementation" unknowns aren't resolved. Execute this with the user.

**Files:** (none modified in this task; this is a verification + bug-fix-as-needed task)

**Pattern:** N/A — manual checklist.

**Step 1: User restarts Claude Code**

The new MCP server is registered, but a running Claude Code session won't
pick it up. Ask the user to restart Claude Code (close + reopen, or
`/exit` + relaunch). Confirm restart before continuing.

**Step 2: Verify the tool is discoverable**

In the new session, the tool should appear in the available tools list as
`mcp__research-index__research_search` (or similar; MCP tool naming
convention prefixes server name). Ask the user to confirm visibility, or
attempt to call it and observe whether the tool is registered.

**Step 3: Run the four verification calls**

Invoke `research_search` four times, recording results for each:

| # | Call | Pass criterion |
|---|------|----------------|
| A | `research_search(query="BridgeRepairHut")` | Returns formatted text containing at least one `docs/research/bridges/` path. |
| B | `research_search(query=<random 32-char hex>)` | Returns the empty-result hint string (starts with "No results for"). Generate a fresh hex string at runtime; do NOT hard-code the test query in the plan doc, which would index itself and poison the test (this happened during Task 2 verification). |
| C | `research_search(query="BridgeRepairHut", format="json")` | Returns valid JSON. Parse with `json.loads` to confirm. |
| D | `research_search(query="C4", system="bridges", limit=3)` | Returns at most 3 results, all from bridges-system docs. |

**Step 4: Resolve each Phase 1 unknown**

For each of the four "Deferred to Implementation" questions, record the
verdict:

- **FastMCP `Literal` support:** Did the server start cleanly with
  `format: Literal["text", "json"]`? If YES → keep. If NO → the fallback
  from Task 2 Step 3 is already applied; note that change here.
- **`.mcp.json` relative path:** Did Claude Code launch the server from the
  relative path? Check by attempting a tool call (failure to launch = MCP
  surfaces "server not available"). If YES → keep. If NO → change `args[0]`
  to absolute `"<local>\\Documents\\ra2-rust-game\\tools\\research_index\\mcp_server.py"`
  and re-test (requires another Claude Code restart).
- **`stdout.reconfigure` + FastMCP coexistence:** Did the server emit garbled
  or partial MCP frames during startup? Symptom: tool calls fail with parse
  errors or the server crashes mid-response. If clean → keep. If garbled →
  remove the `if hasattr(sys.stdout, "reconfigure"): ...` block from
  `mcp_server.py`. Research docs with non-ASCII may still render OK because
  FastMCP wraps the JSON layer; verify with a known-utf8-heavy query like
  `research_search(query="—")` after removal.
- **`format` kwarg name:** Did calls C and D succeed? If YES → keep. If MCP
  rejected the kwarg name → rename param to `output_format` in `mcp_server.py`
  and re-test.

**Step 5: Update the design doc with Phase 1 verdicts**

Edit [docs/plans/2026-05-26-research-index-mcp-server-design.md](docs/plans/2026-05-26-research-index-mcp-server-design.md)
in the **Phasing → Phase 1 → "Unknowns Phase 1 resolves"** subsection. Add
a "Resolution" line under each unknown stating verified/changed/etc. For
unknowns that required a fallback, also update the affected section of the
design (e.g., if `format` was renamed, update the Interfaces / Contracts
block to reflect `output_format`).

**Step 6: Commit changes (if any)**

If Steps 4-5 produced any code or doc changes:

```powershell
git add tools/research_index/mcp_server.py .mcp.json docs/plans/2026-05-26-research-index-mcp-server-design.md
git commit -m "fix: resolve Phase 1 FastMCP/MCP-config unknowns"
```

If no code changes were needed (all four unknowns resolved as the design
predicted), commit only the doc update:

```powershell
git add docs/plans/2026-05-26-research-index-mcp-server-design.md
git commit -m "docs: record Phase 1 verification results"
```

---

## Phase 1 Exit Criteria

Phase 1 is **done** when ALL of the following are true:

1. `python tools/research_index/mcp_server.py` starts and blocks on stdin
   without crashing (Task 1 Step 3 passes).
2. The four verification calls in Task 4 Step 3 all produce the expected
   pass criteria.
3. The four Phase 1 unknowns are recorded with verdicts in the design doc
   (Task 4 Step 5 done).

When all three are true, Phase 1 ships. Phase 2 — library refactor + wiring
the remaining 7 tools + smoke tests — gets its own plan written against the
now-resolved Phase 1 patterns.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-26-research-index-mcp-server-design.md](docs/plans/2026-05-26-research-index-mcp-server-design.md)
- **FastMCP pattern source:** `<local>/Documents/ghidra-mcp/bridge_mcp_ghidra.py`
  (PEP 723 header at lines 1-6; `@mcp.tool()` example at line 929;
  `mcp.run()` at line 1749).
- **Library entry points exercised in Phase 1:**
  - [tools/research_index/research_index/database.py:168](tools/research_index/research_index/database.py) — `search(db_path, query, limit, system, source_kind)`
  - [tools/research_index/research_index/database.py:15-16](tools/research_index/research_index/database.py) — `DEFAULT_DB` resolution
  - [tools/research_index/research_index/formatting.py:6](tools/research_index/research_index/formatting.py) — `format_search_results(rows)`
- **Existing `.mcp.json` shape:** [.mcp.json](.mcp.json) (ghidra-mcp entry as the registration template).
