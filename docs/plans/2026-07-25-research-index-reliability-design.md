# Research Index Reliability Tranche Design

## Goal

Make research-index handoffs honest, relevant, bounded, and freshness-aware without changing the SQLite schema or removing existing CLI/MCP result fields.

## Architecture Context

The research index is a local Python tool with three layers:

1. `research_index/` owns SQLite retrieval, docgraph queries, handoff assembly,
   validation, and text formatting.
2. Thin CLI scripts under `tools/research_index/` parse arguments, call the
   library, print text or JSON, and select an exit code.
3. `mcp_server.py` calls the same library functions and exposes their text or
   JSON serialization through FastMCP.

`database.search` currently turns every token into an FTS `OR` expression,
silently falls back to `LIKE`, and treats a blank query as `LIKE '%%'`
([source: `tools/research_index/research_index/database.py:168`,
`tools/research_index/research_index/database.py:363`]). Handoff assembly
amplifies that broad retrieval by appending generic handoff vocabulary to the
query, while its relevance predicate treats a query with no “specific” token
as matching every candidate
([source: `tools/research_index/research_index/handoff.py:353`,
`tools/research_index/research_index/handoff.py:378`]).

The same handoff expands title and heading words into implementation graph
lookups, so generic terms such as `HIGH` and `SUPERSEDED` can pull unrelated
Rust paths into a focused result
([source: `tools/research_index/research_index/handoff.py:385`,
`tools/research_index/research_index/handoff.py:401`]). Extracted Rust paths
are returned without consulting the current workspace
([source: `tools/research_index/research_index/graph.py:270`,
`tools/research_index/research_index/handoff.py:458`]).

Validation defines validity solely as the absence of file/checksum/link errors,
so an explicitly scoped query that selects zero documents is reported as valid
([source: `tools/research_index/research_index/validation.py:12`,
`tools/research_index/research_index/validation.py:39`]). The brief CLI then
uses that value as its success exit condition
([source: `tools/research_index/brief.py:29`]).

Finally, the text handoff prints every assembled subsection up to the caller's
per-section limit, including long snippets, while JSON contains the full nested
implementation graphs
([source: `tools/research_index/research_index/formatting.py:127`]).

## Impact Analysis

Primary changes:

- `database.py`: shared query token analysis, meaningful-hit coverage, and a
  blank-query guard.
- `handoff.py`: filter broad FTS rows by query coverage and expose an explicit
  match status.
- `touchpoints.py`: restrict implementation graph expansion to query-related
  evidence terms, compute unique supporting-document counts, and annotate
  current Rust-path existence.
- `graph.py`: optionally annotate Rust-path existence for implementation graph
  callers without removing existing fields.
- `validation.py`: distinguish a matched, clean scope from an empty scope.
- `formatting.py`: render compact, omission-aware handoff text and explicit
  no-match/freshness signals.
- CLI/MCP shims: pass the workspace where needed; CLIs return nonzero for an
  empty requested map, handoff, validation, or brief.
- `.gitignore`: track the research-index source and tests while continuing to
  ignore only generated `.cache/` state and the repository-wide bytecode
  patterns.
- Tests and README: cover actual CLI/MCP contracts and document detailed JSON
  versus bounded text.

Compatibility risks:

- Existing dictionary keys and detailed JSON rows remain. New fields are
  additive.
- Search remains broad `OR` retrieval for callers that want discovery. The
  stricter coverage gate applies to implementation handoff assembly, where
  unrelated results are unsafe.
- CLI exit status changes only for an explicitly empty result that previously
  looked successful.
- No database migration or reindex is required.
- No Rust, simulation, determinism, asset, or gameplay code is touched.
- This commit establishes the formerly local/ignored tool as tracked source;
  integration must preserve the target checkout's ignored cache database.

## Chosen Approach

Use one shared query-analysis contract in the library, then make each
workflow's success semantics explicit.

Search will keep compatibility with broad discovery but will return no rows for
blank input and add match-coverage metadata to each result. Handoff assembly
will retrieve a wider candidate pool, retain only rows meeting a deterministic
minimum informative-term coverage, and expand implementation graphs only from
query-related extracted anchors. A handoff will report `matched=false` unless
it has relevant evidence or a relevant handoff section.

Workspace freshness will be a read-time annotation, not index-time state:
Rust touchpoints gain an additive `exists` field when a workspace is supplied,
existing paths sort first, and missing paths produce a warning. This avoids a
schema change and makes results correct after source moves without reindexing.
Merged `doc_count` values will be derived from the unique supporting-document
set instead of summing the same document through multiple implementation terms.

Human-readable handoffs will use per-section display caps, shorter snippets,
and explicit “omitted” counts. Detailed JSON remains available for callers that
need the complete structured bundle.

Validation will add `scope_matched`; `valid` will be false when no document was
selected. Map and handoff gain equivalent additive match signals. CLI exit
codes will follow those signals, while MCP text/JSON will expose them for the
caller.

## Player-Experience Detail Ledger

For this developer tool, the relevant “experience” is whether parity work is
routed to trustworthy evidence and current implementation surfaces.

- `COMPOUNDING` — A zero-document validation must not certify the index scope.
  Baseline reproduction: CLI and MCP both returned `documents_checked=0`,
  `valid=true`, and exit 0 for a generated miss. This can falsely authorize
  parity work. [runtime: 2026-07-25 baseline `validate.py` and
  `research_validate`]
- `COMPOUNDING` — A missing lowercase query must not inherit generic
  implementation-handoff sections. Baseline reproduction emitted an unrelated
  8,417-character handoff because the lowercase token set bypassed relevance
  filtering. [source: `tools/research_index/research_index/handoff.py:353-382`;
  runtime: 2026-07-25 generated-miss CLI]
- `COMPOUNDING` — A focused query must not expand generic title/heading tokens
  into unrelated Rust systems. The stock miner query returned bridge paths
  through the term `HIGH`. [source:
  `tools/research_index/research_index/handoff.py:385-427`; runtime: 2026-07-25
  MCP `research_handoff`]
- `COMPOUNDING` — Returned Rust paths must state whether they exist now. The
  live index contains 674 distinct Rust paths, 174 of which are absent from the
  workspace. They may be stale or planned, so the tool must report existence
  rather than infer a cause. [runtime: 2026-07-25 read-only SQLite/workspace
  comparison]
- `COMPOUNDING` — Touchpoint support counts must count unique documents, not
  repeated term-graph routes. The stock miner handoff reports 11 documents for
  `src/sim/miner/miner_system.rs` while its structured result contains only 8
  unique documents. Inflated support can make a weak owner look authoritative.
  [source: `tools/research_index/research_index/handoff.py:458-493`; runtime:
  2026-07-25 stock handoff JSON]
- `COMPOUNDING` — Default MCP text must remain below ordinary response
  truncation pressure while retaining citations and omission counts. The stock
  miner handoff measured about 18k characters in text and 146k in JSON.
  [runtime: 2026-07-25 CLI/MCP stock workflow]
- `MILESTONE-BLOCKING` — Existing exact symbol/address, source/system filters,
  citations, JSON keys, and broad search discovery must keep working; otherwise
  the reliability fix would make normal research navigation less useful.
  [source: `AGENTS.md` Research Index Workflow; current unit tests]
- `EXACTIFICATION-RESIDUAL` — Detailed JSON remains large because compatibility
  preserves nested rows. It is explicit opt-in output; compact default text is
  the production path. Trigger: callers request JSON. Frequency: occasional
  automation/debugging. Effect: possible transport truncation. Downstream risk:
  bounded because no field is silently removed and callers can lower `limit`.
- `EXACTIFICATION-RESIDUAL` — Path existence does not prove semantic freshness.
  A present file may have moved symbols or changed responsibilities. Trigger:
  research prose cites an existing but outdated owner. Frequency and effect are
  corpus-dependent. Downstream risk remains visible in citations and still
  requires direct Rust reads per `AGENTS.md`.

## Design

### Components

`database.py` will expose normalized informative query terms plus per-row match
coverage. Corpus-generic workflow words such as “research”, “implementation”,
“handoff”, and “Rust” will not satisfy handoff relevance by themselves.
Specific anchors (addresses, `::`/underscore identifiers, paths, and INI-like
keys) remain informative. Hex addresses compare by numeric digits so the
documented CLI form `0x73e5e0` still matches indexed `0x0073e5e0` evidence.

`handoff.py` will use those terms to:

- filter evidence and explicit handoff candidates;
- require one informative hit for a one-term query and two for a multi-term
  query;
- set `matched`.

`touchpoints.py` will derive graph terms from the query plus extracted evidence
anchors that overlap the query, never arbitrary title/status vocabulary. It
will merge Rust paths with a unique supporting-document count and annotate
them with existence state. Keeping this cohesive concern separate prevents
`handoff.py` from growing beyond the project's module-size guideline.

`graph.py` will accept an optional workspace and add `exists` to returned Rust
paths when supplied. Existing library callers that omit the workspace retain
their prior shape.

`validation.py` and `system_map.py` will expose additive scope-match fields.

`formatting.py` will own display budgets. It will never silently omit rows:
each capped section names how many entries were not displayed and directs
callers to JSON or a narrower query.

### Interfaces / Contracts

- Existing positional parameters remain valid; new workspace parameters are
  optional trailing keyword arguments.
- Hex-address lookup is case-insensitive and ignores leading zero padding.
- Existing result keys remain; `matched`, `scope_matched`,
  `matched_terms`, `query_coverage`, and `exists` are additive.
- `research_search("")` returns no results and an actionable MCP hint.
- Scoped validation with zero documents returns `scope_matched=false`,
  `valid=false`.
- Handoff with no relevant evidence/handoff returns `matched=false`, no
  authority cluster pollution, no unrelated Rust touchpoints, and warnings.
- Missing Rust paths are not deleted from results; they are marked
  `exists=false`, sorted behind current paths, and summarized in warnings.
- `doc_count` equals the number of unique paths in `documents`, including when
  the same document is reached through several implementation terms.
- Text handoffs remain citation-bearing and bounded; JSON remains detailed.

### Data Flow

1. CLI/MCP receives query, filters, limit, and workspace.
2. Broad FTS retrieval produces candidate rows plus normalized query coverage.
3. Handoff relevance gate rejects candidates below the informative-hit
   threshold.
4. Query-related anchors drive implementation graph lookups.
5. Direct and graph Rust paths merge, gain workspace existence state, and sort
   deterministically.
6. Structured result records match/freshness state.
7. Formatter emits bounded text; JSON serializer emits the compatible detailed
   structure.

### Error Handling

Blank handoff/search input is a normal no-match result, not a database-wide
query. Missing workspace files produce freshness annotations, not exceptions.
SQLite errors continue to follow current behavior; this tranche does not hide
database corruption or missing-database failures.

CLI commands use exit 1 for a requested empty map/handoff/validation/brief and
retain argparse's exit 2 for invalid invocation. MCP cannot return a process
status, so its structured/text match fields carry the same signal.

### Testing Strategy

- Preserve and run all existing `unittest` coverage.
- Add temporary-database unit tests for blank queries, lowercase misses,
  meaningful multi-term matches, generic-handoff rejection, query-related graph
  terms, unique touchpoint support counts, and present/missing Rust-path
  annotations.
- Add validation and map zero-scope tests.
- Add formatter tests proving citations remain, omissions are explicit, and a
  synthetic large handoff stays within a fixed display budget.
- Add subprocess CLI tests for zero-match exit status and real text.
- Extend live MCP smoke tests to verify empty validation/handoff signals.
- Rebuild a worktree-local index, then reproduce the stock CLI and MCP-shaped
  library workflows. Do not use Cargo.

## Architectural Decisions

The design follows the existing shared-library/thin-shim boundary. Query
meaning, freshness, and match status live in library results; CLI/MCP wrappers
only transport them.

It deliberately does not add embeddings, another search backend, a database
schema migration, or Git-history freshness. Those would broaden the tranche
and make compatibility/testing harder before basic truthfulness is fixed.

The query gate is applied to handoffs, not general search. This preserves broad
exploration while preventing weak lexical coincidence from being presented as
implementation guidance.

## Alternatives Considered

### 1. Shim-only warnings and exit codes

Add no-match checks in each CLI/MCP wrapper and cap printed strings. This is
small but leaves polluted handoff structures, stale touchpoints, and divergent
CLI/MCP behavior. Rejected because callers using JSON would still receive
misleading guidance.

### 2. Shared library truth and bounded presentation (chosen)

Add deterministic query coverage, match status, live path annotations, and
bounded text at the existing ownership points. This fixes the causes without a
schema migration and keeps detailed JSON compatibility.

### 3. Reindex with Git fingerprints and semantic retrieval

Persist Rust file hashes/symbol ownership and add embeddings or semantic
ranking. This could eventually detect deeper semantic staleness, but requires a
schema/data migration, reindex policy, new dependencies, and much broader
evaluation. Rejected for this tranche.

## Approval Record

Autonomous adversarial review asked:

- Could stricter matching hide useful broad discovery? Resolved by leaving
  `research_search` broad and applying the coverage gate only to implementation
  handoffs.
- Could path checks mislabel planned work as stale? Resolved by exposing the
  factual name `exists` and warning “missing or planned”, without deleting it or
  asserting why.
- Could compact text break automation? Existing JSON remains detailed and all
  structured keys are preserved; text omissions are explicit.
- Could zero-match invalidation break full-corpus validation? A non-empty,
  issue-free corpus remains valid. Only an empty selected scope changes.
- Could the tranche require a reindex or database migration? No schema changes
  are planned.
- Could repeated implementation terms inflate confidence? Resolved by deriving
  `doc_count` from the unique document set after merge.

Decision: self-approved for formal design review. The remaining residuals are
bounded and do not block implementation.
