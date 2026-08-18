# research_index MCP Server Design

## Goal

Expose the eight `tools/research_index/*.py` CLIs as MCP tools in the same first-class
slot as Grep, Read, and codegraph, so that research lookups stop losing to the
grep reflex mid-session.

## Architecture Context

`tools/research_index/research_index/` is the load-bearing module. The eight CLI
scripts at `tools/research_index/*.py` are uniform 30–50 line shims with no
business logic — each one parses args, calls a single library function, and
prints either the JSON result or its `format_*` companion.

Library entry points (all return JSON-serializable dicts/lists; all paired with
a pure `format_*` text renderer in `research_index/formatting.py`):

| CLI | Library function | Return shape |
|---|---|---|
| `search.py` | `database.search(db, query, limit, system, source_kind)` | `list[dict]` |
| `related.py` | `database.related_by_document` / `related_by_term` | `list[dict]` |
| `graph.py` | `graph.{document_graph, backlinks, evidence_view, implementation_view}` | `dict` |
| `map.py` | `system_map.system_map` | `dict` |
| `handoff.py` | `handoff.parity_handoff` | `dict` |
| `brief.py` | `brief.research_brief` | `dict` |
| `validate.py` | `validation.validate_index` | `dict` |
| `index.py` | `metadata.iter_indexable_files` + `chunking.chunk_file` + `database.rebuild_database` | — |

`DEFAULT_DB` is self-resolving: `Path(__file__).resolve().parents[1] / ".cache" /
"research.db"`. Workspace root is needed only by `brief` and `validate`, and only
for resolving link targets during validation.

Existing `.mcp.json` already registers `ghidra-mcp` with the same shape we'll
mirror: `{"command": "python", "args": [...]}`. Template for the server itself
is `<local>/Documents/ghidra-mcp/bridge_mcp_ghidra.py` — FastMCP + PEP 723
inline deps header. Approved Anthropic Python MCP SDK only.

## Impact Analysis

- **New file:** `tools/research_index/mcp_server.py` (~250–400 lines, mostly
  tool decorators).
- **Modified:** `.mcp.json` adds one entry for `research-index`.
- **Small library refactor (Phase 2):** add
  `research_index.indexing.rebuild_index(workspace: Path, roots: list[Path]) -> str`
  (extracted from `index.py:main`, returns the existing 1-line summary).
  Both `index.py` and the MCP `research_reindex` tool then call it. Motivated
  by `research_reindex` (Phase 2) — `index.py:main` does 15 lines of
  orchestration (iterate roots → chunk each file → build `(relpath, meta,
  chunks)` tuples → call `rebuild_database`), and duplicating that loop in
  the MCP tool body would diverge over time. Not needed for Phase 1; lands
  with the rest of the Phase 2 work.
- **Unchanged in Phase 1:** all 8 CLI scripts and the entire
  `research_index/` library. Phase 1 is purely additive (new MCP server
  file + `.mcp.json` entry).
- **Risk: tight coupling to library internals.** Mitigated by importing only
  the documented top-level entry points listed above; the schema and ranking
  modules stay out of the import surface.
- **Risk: stale-index drift.** Mitigated by exposing `research_validate` and
  `research_reindex` as first-class tools rather than hiding freshness checks
  in every search call. Operator (or agent on operator's behalf) controls
  rebuild cadence.
- **No determinism / sim impact.** Pure tooling layer.

## Chosen Approach

**Library-import FastMCP server** at `tools/research_index/mcp_server.py`,
exposing 8 tools that map 1:1 to the existing CLIs. Each tool dispatches to
one library function, optionally JSON-serializes, and either pretty-formats
through `formatting.format_*` or returns an empty-results hint.

Rejected alternatives:

- **Subprocess wrapper:** pays ~200ms Python cold-start per call (2–4 seconds
  of overhead across a typical session). The library is already organized
  for direct import; the CLI-exactness argument fails because the CLIs
  themselves are trivial shims.
- **Async / job-handle reindex:** reindex of 1700 docs runs in single-digit
  to tens of seconds — well inside MCP tool timeouts. Async state + polling
  buys nothing for a tool called rarely.
- **Auto-freshness check per call:** clutters agent context with warnings,
  costs a docs/ mtime walk every call. `research_validate` exists for this.

## Tiny-Detail Ledger

Tooling, not gameplay parity — the heavy parity ledger doesn't apply. Real
footguns:

- **Python 3.10+** required (library uses `Literal`, structural typing). PEP
  723 header pins this. [source: `bridge_mcp_ghidra.py` line 2]
- **`sys.stdout.reconfigure(encoding="utf-8", errors="replace")`** at startup,
  matching every CLI. Research docs contain non-ASCII (em-dashes, code-page
  strings from Ghidra reports). [source: `search.py:27`,
  `related.py:26`, `graph.py:27`, `map.py:29`, etc.]
- **`sys.path.insert(0, str(Path(__file__).resolve().parent))`** before any
  `research_index.*` import. Otherwise the import resolves only when launched
  from a specific cwd. [source: how the existing CLIs implicitly resolve
  this via being run from their own dir]
- **Workspace = `Path(__file__).resolve().parents[2]`** for `brief` and
  `validate` calls. Do not trust `Path.cwd()`. [source: `brief.py:36`,
  `validate.py:36`]
- **`DEFAULT_DB` resolves itself** from
  `research_index/database.py:15-16` — no workspace param needed for DB.
- **`anchors` normalization:** CLI takes repeated `--anchor X` flags,
  library takes `list[str]`. MCP exposes `anchors: list[str] | None`;
  normalize `None` → `[]` before calling `research_brief`. [source:
  `brief.py:18`]
- **Empty-result hint format must not include the query as a path-like
  token** — the hint is purely advisory, not a citation. Generated in the
  MCP wrapper, never in the library.
- **`research_reindex` returns the 1-line summary** produced by the new
  `rebuild_index` helper (`"indexed documents=N chunks=M db=<path>"`) — agent
  needs confirmation the rebuild ran.
- **CLI default limits override library defaults in two places.** MCP tool
  defaults must match the CLI, not the library:
  - `database.search` library default is 10; `search.py` CLI passes 20.
    `research_search` defaults `limit=20`.
  - `graph.backlinks` library default is 20; `graph.py` CLI passes 12 across
    all four modes. `research_graph` defaults `limit=12` for all modes.
  - All other tools: library default = CLI default, no override needed.
- **Empty-result hint scope is tool-specific.** Only `research_search` and
  `research_related` return `list[dict]` where `if not result` is the right
  emptiness check. The dict-returning tools (`graph`, `map`, `handoff`,
  `brief`, `validate`) always return a populated outer dict; emptiness lives
  in inner fields (`documents: []`, `evidence: []`). Their `format_*`
  renderers already display "0 documents matched" headers — don't bolt an
  extra hint on top.
- **`research_brief` is a fan-out (heavy aggregator).** Internally invokes
  `validate_index` + `system_map` + `parity_handoff` + (`evidence_view` +
  `implementation_view`) × N anchors. 5+ SQLite connections per call.
  Docstring must signal "heavy — use `research_search` or `research_handoff`
  first for cheap lookups." Parallels `codegraph_context` in role.
- **Library has no `print()` calls.** Confirmed by grep across
  `research_index/`. Safe for stdio MCP transport — only the CLI shims print.
- **`research_index/__init__.py` is empty.** No import-time side effects.
- **SQLite concurrent read/write is safe.** `rebuild_database` writes to a
  `.tmp` file then `os.replace`s atomically (`database.py:48`). Concurrent
  `research_search` during `research_reindex` sees the old DB until the swap.
  No additional locking needed.
- **No tool-name collisions:** `research_*` vs `codegraph_*`. Confirmed by
  prefix.
- **`.mcp.json` uses relative path** `"tools/research_index/mcp_server.py"` —
  Claude Code launches from repo root (consistent with how
  `python tools/research_index/search.py` is invoked throughout CLAUDE.md).

## Design

### Components

```
tools/research_index/
  mcp_server.py            ← NEW; FastMCP server, 8 @mcp.tool() functions
  search.py / related.py / graph.py / map.py / handoff.py
  brief.py / validate.py / index.py                ← unchanged
  research_index/                                   ← unchanged library
  .cache/research.db                                ← unchanged
.mcp.json                                           ← adds "research-index" entry
```

### Interfaces / Contracts

All 8 tools accept `format: Literal["text", "json"] = "text"` and return a
string (text content block). Param names mirror the CLI flags for muscle-memory
continuity.

```python
research_search(
    query: str,
    limit: int = 20,
    system: str | None = None,
    source: str | None = None,
    format: str = "text",
) -> str

research_related(
    target: str,
    by: Literal["doc", "term"] = "doc",
    limit: int = 20,
    format: str = "text",
) -> str

research_graph(
    mode: Literal["doc", "backlinks", "evidence", "implementation"],
    target: str,
    limit: int = 12,
    format: str = "text",
) -> str

research_map(
    topic: str | None = None,
    system: str | None = None,
    source: str | None = None,
    status: str | None = None,
    limit: int = 80,
    format: str = "text",
) -> str

research_handoff(
    query: str,
    system: str | None = None,
    source: str | None = None,
    limit: int = 8,
    format: str = "text",
) -> str

research_brief(
    query: str,
    anchors: list[str] | None = None,
    system: str | None = None,
    source: str | None = None,
    limit: int = 8,
    format: str = "text",
) -> str

research_validate(
    topic: str | None = None,
    system: str | None = None,
    source: str | None = None,
    status: str | None = None,
    limit: int = 40,
    format: str = "text",
) -> str

research_reindex(
    roots: list[str] | None = None,
) -> str
```

Tool docstrings are the discovery surface — written so an agent picks the
right tool by intent, not by name. Draft set:

- `research_search` — "FTS hits with snippets and citations for a phrase or
  symbol. Use when looking up where a concept is documented."
- `research_related` — "Docs sharing extracted evidence (symbols, addresses,
  INI keys, Rust paths) with a source doc (`by='doc'`) or an exact term
  (`by='term'`)."
- `research_graph` — "Navigate docgraph edges. `mode='doc'` shows outgoing,
  `'backlinks'` shows incoming, `'evidence'` resolves a term to docs,
  `'implementation'` resolves a term to Rust paths."
- `research_map` — "Inventory of docs for a system or topic, grouped by
  subsystem / source / status. Use to survey what's already researched."
- `research_handoff` — "Implementation-handoff bundle for a mechanism or
  symbol: handoff sections + top evidence + Rust touchpoints."
- `research_brief` — "Compact pre-implementation planning bundle.
  `anchors` lets you pin exact symbols or addresses."
- `research_validate` — "Check the index against current files; report
  stale chunks and broken local markdown links."
- `research_reindex` — "Rebuild the FTS index from disk. `roots` optional;
  defaults to docs/research, docs/plans, ini."

### Data Flow

```
agent invokes research_<tool>(args)
  ↓
FastMCP marshals args
  ↓
tool function:
  - normalize None defaults (anchors → [], roots → DEFAULT_ROOTS)
  - call library function with (DEFAULT_DB, args...)
  - check for empty result → return hint string
  - if format == "json": return json.dumps(result, indent=2)
  - else: return format_<tool>(result)
  ↓
FastMCP wraps string in TextContent
  ↓
agent reads result, follows up with another tool or composes response
```

### Error Handling

- **Empty results:** wrapper detects empty list/dict and returns a one-line
  hint suggesting the next tool to try. Never throws.
- **Bad target (e.g., `research_graph mode='doc'` with a doc path that
  doesn't exist):** library returns an empty/error result dict. Wrapper
  detects, returns descriptive message including the target.
- **DB missing or unreadable:** wrapper catches `sqlite3.OperationalError`
  at the top of each tool, returns `"Index not built. Run research_reindex."`
- **Library exceptions:** logged via Python logging (matches ghidra-mcp
  pattern), re-raised as a clean text response so the agent sees the failure
  rather than a transport-level error.
- **No silent failures** — every code path returns a meaningful string.

### Testing Strategy

Extend the existing test file at
`tools/research_index/tests/test_research_index.py` rather than creating a
new test layout.

- **Smoke test per tool:** invoke each tool's wrapper function with a
  known-good query against the live `.cache/research.db`, assert non-empty
  text and an expected substring (e.g., `research_search "BridgeRepairHut"`
  must contain `docs/research/bridges`).
- **Empty-result test (search, related only):** call with a guaranteed-miss
  query, assert hint string is returned (not blank).
- **JSON format test:** call each tool with `format="json"`, parse with
  `json.loads`, assert dict/list shape.
- **No new library tests needed** — the library itself is already tested.
- **Manual integration test:** add `research-index` to `.mcp.json`, restart
  Claude Code, invoke each tool by name, verify outputs match the matching
  `python tools/research_index/<tool>.py` invocation.

## Phasing

The work splits cleanly into two phases. Phase 1 is the proof-of-pattern gate
that flushes out unknowns on the smallest viable surface; Phase 2 is
mechanical replication once Phase 1 has locked the shape.

### Phase 1 — Prove the pattern (gate on one tool)

Smallest end-to-end slice that exercises every piece of the design once.
Deliberately excludes the `rebuild_index` library refactor — that's
motivated by `research_reindex` (Phase 2) and is not needed to validate any
of the four Phase 1 unknowns below.

- New file `tools/research_index/mcp_server.py`:
  - PEP 723 header (`requires-python >=3.10`, `dependencies = ["mcp>=1.2.0,<2"]`)
  - `sys.path` setup so `from research_index.database import ...` resolves
  - `sys.stdout.reconfigure(encoding="utf-8", errors="replace")`
  - `mcp = FastMCP("research-index")`
  - `if __name__ == "__main__": mcp.run()`
- **One tool only — `research_search`.** Most-used surface, validates the
  empty-result-hint code path, exercises the `format: "text" | "json"` param.
- `.mcp.json` entry for `research-index`.
- Manual integration test: restart Claude Code, invoke `research_search`
  against the live `.cache/research.db`, confirm output substring-matches
  the equivalent `python tools/research_index/search.py "<query>"`.

**Phase 1 exit criteria:** `research_search` works end-to-end through MCP,
including empty-result hint and JSON format. No further tools added in this
phase.

**Unknowns Phase 1 resolves** (all verified 2026-05-27 via end-to-end MCP
round-trip after Claude Code restart):

- Does FastMCP accept `Literal["text", "json"]`, `str | None`, default-None
  parameters in `@mcp.tool()` signatures? **YES.** Schema came through with
  `Literal` as a proper enum (`"enum": ["text", "json"]`) and `str | None`
  as `anyOf [string, null]`. No fallback needed.
- Does `.mcp.json` accept the relative path
  `"tools/research_index/mcp_server.py"`? **YES.** Claude Code resolved
  the relative path against the repo root on session restart; the tool
  appeared as `mcp__research-index__research_search` without any absolute-path
  fallback.
- Does `sys.stdout.reconfigure(...)` interfere with FastMCP's ownership of
  stdout for the stdio transport? **NO.** Em-dashes (`—`) and section signs
  (`§`) embedded in research-doc output rendered correctly through MCP. The
  `if hasattr(sys.stdout, "reconfigure"): ...` block is justified and
  necessary — without it the Windows default encoding garbles non-ASCII.
- Does the `format` kwarg (Python builtin shadow) round-trip cleanly through
  MCP JSON? **YES.** No rename needed; `format="json"` round-tripped
  correctly with proper JSON output.

All four resolved as the design predicted — no Phase 1 fallbacks exercised.
Phase 2 can rely on this pattern unchanged.

**Additional Phase 1 finding:** The empty-result test query in the plan must
be generated at runtime, not hard-coded. A hard-coded "guaranteed miss"
string baked into the plan doc gets indexed (because `docs/plans/` is
indexed), turning the test into a self-reference hit. The current plan's
Task 4 Step 3 Row B reflects this fix.

### Phase 2 — Replicate the pattern across remaining tools

Mechanical once Phase 1 is green. New plan to be written against this phase
only after Phase 1 lands.

- **Library refactor:** extract
  `research_index.indexing.rebuild_index(workspace, roots, db_path) -> str`
  from `index.py:main`; convert `index.py` to a shim that calls the new
  helper. Needed before `research_reindex` so the orchestration loop has a
  single owner.
- 7 additional `@mcp.tool()` functions: `research_related`, `research_graph`,
  `research_map`, `research_handoff`, `research_brief`, `research_validate`,
  `research_reindex`. Each is a copy of the Phase 1 pattern with a different
  library dispatch and (where applicable) empty-result hint.
- Tool docstrings tuned for discovery (the 8 one-liners from the
  Interfaces section of this doc).
- Smoke tests added to
  `tools/research_index/tests/test_research_index.py` — one assertion per
  tool against the live DB.

**Phase 2 exit criteria:** all 8 tools usable through MCP; smoke tests pass;
each tool's output matches its CLI counterpart's output for the same query.

## Architectural Decisions

**Patterns followed:**

- FastMCP decorator-per-tool pattern from ghidra-mcp (`@mcp.tool()`).
- PEP 723 inline-dependency header (`# /// script ... # ///`) so the server
  is launchable via `python` or `uv run` without separate venv setup.
- `.mcp.json` registration mirrors the existing `ghidra-mcp` entry exactly.
- Tool naming: `research_<verb>` parallels `codegraph_<verb>` — same shape
  in the catalog, agent reaches for the right family by prefix.
- Library-import-not-subprocess pattern matches how the CLIs already use the
  library; no new abstraction introduced.

**Patterns deviated from / not applicable:**

- ghidra-mcp dynamically loads tools from an HTTP `/mcp/schema` endpoint at
  runtime. Not applicable here — tool set is static and known at write time.
  Use plain `@mcp.tool()` decorators instead of dynamic registration.
- ghidra-mcp uses per-endpoint timeout overrides. Not needed: every
  research_index call is a fast SQLite query except `research_reindex`,
  which still completes well inside default timeouts.

**Tech debt introduced:**

- Two surfaces for the same functionality (CLI + MCP). Mitigation: both
  delegate to the same library, so the library remains the single source
  of truth. Cost of duplication is one thin wrapper file.
- Coupling between MCP server and library entry-point signatures. Mitigation:
  signatures are stable per the README's own "MCP only after CLI outputs
  are stable" gate. If a library function adds a param, the MCP tool gets
  the new param too.

## Alternatives Considered

- **Subprocess wrapper** — rejected for ~200ms per-call overhead and no
  ergonomic gain.
- **Async reindex with job handle** — rejected as premature; reindex
  completes in seconds.
- **Auto-freshness check on every search** — rejected as context noise
  and per-call filesystem cost.
- **Four separate `research_graph_*` tools** — rejected; the four modes
  answer the same question ("docgraph adjacency"), splitting them
  quadruples catalog surface for no clarity gain.
- **Two separate `research_related_*` tools (by-doc, by-term)** — rejected;
  same argument, single tool with `by` enum is cleaner.
- **Top-level `tools/mcp/` directory grouping all future MCP servers** —
  deferred; abstraction has cost and there's only one new server to write.
  Revisit if a second MCP server appears in this repo.
