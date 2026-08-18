# Research Index Freshness and System Map v2 Integration Design

## Goal

Make research-index freshness automatic and observable for MCP callers, and
establish the already-designed System Map v2 implementation as tracked,
validated project source without merging their separate truth domains.

The user approved this combined direction on 2026-07-25 after reviewing the
recommended sequence: finish research-index freshness automation, then track
and integrate System Map v2 cleanly.

## Architecture Context

The research index has three existing layers:

1. `research_index/` owns corpus discovery, chunking, SQLite storage,
   retrieval, validation, graph queries, handoff assembly, and formatting.
2. Thin CLI shims expose those library contracts.
3. `mcp_server.py` exposes the same contracts through FastMCP.

Rebuilds are already database-atomic: `rebuild_database` writes a temporary
SQLite file and replaces the live database after the connection closes
([source: `tools/research_index/research_index/database.py:50-72`]). However,
the current index has no generation manifest, indexed-root record, workspace
identity, tool-format version, or current corpus snapshot. MCP tools read the
database without a freshness gate and rely on the caller to invoke
`research_validate` or `research_reindex`
([source: `tools/research_index/mcp_server.py`; design:
`docs/plans/2026-05-26-research-index-mcp-server-design.md`]).

`iter_indexable_files` silently skips missing roots
([source: `tools/research_index/research_index/metadata.py:48-64`]), and
`rebuild_index` replaces the database even when discovery returns no
documents ([source:
`tools/research_index/research_index/indexing.py:20-50`]). A misspelled,
missing, outside-workspace, or intentionally partial root can therefore
replace the shared database without a safety signal. Validation compares
checksums only for indexed documents and reads link existence recorded at
index time, so it cannot discover newly added corpus files or a link target
that changed independently
([source: `tools/research_index/research_index/validation.py:12-69`,
`tools/research_index/research_index/validation.py:86-115`]).

System Map v2 is a separate standard-library Python tool and canonical JSON
data surface:

- `system_map/registry.v2.json` normalizes all 336 canonical GSI systems.
- `system_map/topology.v2.json` stores reviewed services, typed edges, ordered
  loops, Rust surfaces, and routing annotations.
- `system_map/source-lock.v2.json` binds the ignored research inputs and
  historical Rust baseline by hash.
- `tools/system_map/` imports, validates, queries, renders, and computes
  Git-aware Rust freshness.

Its approved design explicitly keeps engine topology separate from research
retrieval and parity obligation tracking
([design: `docs/plans/2026-07-25-system-map-v2-design.md`;
source: `system_map/README.md`]). The current local implementation passes 31
unit tests and live `check --require-sources` validation with 336 systems, 41
services, 53 edges, 12 loops, zero errors, and the 12 documented broad-stage
warnings [runtime: 2026-07-25 baseline].

## Impact Analysis

### Research-index changes

- Add a lifecycle module owning root validation, corpus snapshots, sidecar
  generation metadata, health inspection, and ensure-fresh orchestration.
- Extend the rebuild path to reject unsafe roots and empty replacement,
  write unique temporary databases, and publish metadata only after the
  database swap succeeds.
- Add a `research_health` CLI and MCP tool.
- Gate every read-only MCP tool through synchronous ensure-fresh behavior.
- Extend validation with current unindexed-file detection and live link
  target checks.
- Add bounded health formatting and tests covering CLI, library, and MCP
  behavior.

### System Map changes

- Track the existing `system_map/` canonical data, schemas, README, and source
  lock.
- Track the existing `tools/system_map/` library, CLI, and tests.
- Track the approved System Map design.
- Preserve generated render output under ignored `target/system-map-v2/`.
- Preserve ignored research inputs outside Git; `check --ci` validates tracked
  canonical state, while `check --require-sources` additionally validates
  locally available ignored source documents.

### Compatibility and blast radius

- Existing research-index CLI arguments, MCP tool names, dictionary fields,
  and SQLite schema remain compatible.
- The sidecar manifest is generated state under
  `tools/research_index/.cache/`; it is not committed.
- A legacy database without a sidecar is retained until a successful
  automatic rebuild replaces it.
- System Map does not import the research-index Python package, and the
  research index does not import System Map. Cross-links remain documentation
  and canonical citation/GSI identifiers, preventing hidden runtime coupling.
- No Rust, gameplay, INI, asset, simulation determinism, Cargo, Ghidra, merge,
  or remote state is touched.

## Chosen Approach

### Sidecar generation manifest plus synchronous MCP refresh

Use a small JSON sidecar next to the SQLite database rather than migrating the
345 MB live database. The manifest records:

- format and tool version;
- a signature of the schema and index-builder source inputs;
- absolute workspace identity;
- normalized repo-relative indexed roots;
- generation and build time;
- indexed document/chunk counts;
- the published database byte size and nanosecond modification time;
- a deterministic per-file snapshot of path, byte size, and nanosecond
  modification time.

Health inspection enumerates the stored roots, compares the current stat
snapshot with the indexed snapshot, verifies the database has its required
tables, and reports added, changed, removed, or missing-root state. It also
compares the current SQLite file identity with the one recorded after
publication, so an old sidecar cannot certify a separately replaced database.
The snapshot walk measured about 0.33 seconds for the then-current 3,037-file
corpus using a PowerShell baseline; MCP pays it once per top-level tool call,
not once per nested brief component [runtime: 2026-07-25].

Every MCP read tool synchronously calls `ensure_fresh`. A process-local lock
coalesces server threads, and a standard-library advisory file lock serializes
refresh publication across MCP and CLI processes. If the database is missing,
legacy, stale, or bound to another workspace, the server performs one full
atomic rebuild before answering. Rebuild failure leaves the previous database
untouched and fails the tool call with an actionable error rather than
silently serving stale evidence.

When a valid manifest exists, its stored root list is authoritative for
automatic refresh. An explicit `research_reindex(roots=[...])` intentionally
changes that list after every supplied root passes safety checks; subsequent
MCP reads preserve that explicit scope. When no valid manifest exists, the
server uses `DEFAULT_ROOTS`. `research_health` always displays the effective
roots so a deliberately focused index cannot masquerade as the default full
corpus.

Explicit CLI reads remain non-mutating. `research_health --refresh` provides
the same ensure-fresh behavior for CLI fallback workflows; plain
`research_health` only inspects.

### Safe full rebuild, not incremental mutation

Validate every selected root before reading:

- it must exist;
- it must resolve inside the selected workspace;
- it must be a file or directory containing at least one indexable document;
- discovery across all roots must yield at least one chunked document.

Write each rebuild to a unique sibling temporary database, so simultaneous
processes cannot delete or share one fixed `.tmp` path. Publish the database
with `os.replace`, stat the published file, then atomically publish the
manifest containing that file identity. Build or database-swap failure leaves
the previous database untouched. If manifest publication fails after a
successful database swap, the new database remains valid but uncertified; the
old or missing sidecar cannot match its file identity, so the next health
check rebuilds and republishes rather than serving it as fresh.

Full rebuilding is intentionally retained for this tranche because document
references and graph edges are corpus-wide. Incremental mutation would need
careful reverse-reference repair and a larger migration/test surface.

### Track System Map without merging responsibilities

Commit the reviewed System Map sources as their own coherent change. Preserve
its standard-library-only implementation, schema/version contracts, strict
source locks, deterministic renderer, and separate native/Rust/oracle/routing
planes. Research-index README cross-links the engine map; System Map README
explains the reciprocal boundary.

## Player-Experience Detail Ledger

For this developer tooling, the relevant experience is whether parity work is
routed through current evidence and the correct complete player loop.

- `COMPOUNDING` — A newly added or removed research document must invalidate
  the generation even though it has no existing SQLite row. Otherwise an
  agent can duplicate research or miss a correction. Owner: lifecycle corpus
  snapshot. Test: add/delete a temporary corpus file and inspect/refresh.
  [source: current validation only selects `documents` rows]
- `COMPOUNDING` — A link target changed independently of its source document
  must be checked against the current workspace, not stored
  `links.exists_flag`. Owner: live validation. Test: create/delete a target
  after indexing without touching the source. [source:
  `validation.py:86-115`]
- `COMPOUNDING` — Missing, outside-workspace, and empty roots must never
  replace the last good database. Owner: lifecycle root validator and atomic
  rebuild. Test: each invalid root leaves existing database bytes unchanged.
  [source: `metadata.py:48-64`; `indexing.py:20-50`]
- `COMPOUNDING` — An MCP query must not silently use a database generated for
  another workspace or older tool generation. Owner: manifest workspace and
  version checks plus MCP ensure-fresh. Test: legacy/mismatched manifests
  trigger exactly one rebuild before a query.
- `COMPOUNDING` — An explicit custom root scope must remain stable across
  later automatic checks and must be visible to the caller. Owner: effective
  roots stored in the manifest. Test: a focused rebuild followed by an MCP
  read preserves the focused roots; deleting one makes health fail closed.
- `MILESTONE-BLOCKING` — Existing research-index queries, JSON shapes,
  citations, filters, exact anchors, and the 34 reliability tests must remain
  compatible. Owner: additive lifecycle boundary and regression suite.
- `MILESTONE-BLOCKING` — System Map must retain exactly 336 canonical
  systems, 41 services, 53 reviewed edges, 12 ordered loops, source hashes,
  ID rules, and separate relationship planes. Owner: existing schemas,
  validator, source lock, and 31-test suite. [source:
  `system_map/README.md`; runtime baseline]
- `COMPOUNDING` — System Map generated views must remain derived and
  deterministic; generated output must not be committed as hand-maintained
  truth. Owner: `render`/`render --check`; output remains under `target/`.
- `EXACTIFICATION-RESIDUAL` — Any corpus change causes a full rebuild.
  Trigger: add/edit/delete indexed content. Frequency: normally after a
  research batch or INI change. Effect: the first later MCP lookup can take
  single-digit to tens of seconds. Downstream risk: bounded because later
  queries are fresh and the old database survives failures.
- `EXACTIFICATION-RESIDUAL` — Stat snapshots can theoretically miss content
  rewritten with identical byte size and preserved nanosecond mtime. Trigger:
  an unusual copy/restore tool that deliberately preserves both. Frequency:
  uncommon in the normal editor/agent workflow. Effect: automatic freshness
  could remain apparently current. Downstream risk: bounded by checksum-based
  `research_validate` before high-stakes work; exact content hashing on every
  query remains a future option if this occurs.
- `EXACTIFICATION-RESIDUAL` — An already-running MCP process cannot load newly
  cherry-picked Python code into itself. Trigger: integrating the feature
  while the server is live. Frequency: once per tool upgrade. Effect: old
  tools remain exposed until the client restarts the MCP server. Downstream
  risk: `research_health` reports tool generation after restart, and the
  integration handoff must call this out.

## Design

### Components

1. **`research_index/lifecycle.py`**
   - root normalization and workspace confinement;
   - current corpus stat snapshot;
   - sidecar load/write;
   - database shape inspection;
   - bounded health diff;
   - ensure-fresh orchestration.

2. **`research_index/locking.py`**
   - cross-process advisory publication lock;
   - bounded timeout surfaced as a lifecycle error.

3. **`research_index/indexing.py` and `database.py`**
   - safe validated rebuild;
   - unique temporary database;
   - structured rebuild result used to write manifest;
   - compatible one-line CLI/MCP summary.

4. **`research_index/validation.py`**
   - current full-scope unindexed files;
   - live local-link resolution;
   - accurate uncapped counts with display limits applied afterward.

5. **`research_index/formatting.py` and `health.py`**
   - compact health text;
   - JSON detail;
   - explicit inspect versus refresh exit semantics.

6. **`mcp_server.py`**
   - one process-local refresh lock around the cross-process lifecycle lock;
   - ensure-fresh before every read tool;
   - explicit `research_health(refresh=False)`.

7. **System Map canonical source**
   - existing `system_map/` data/schemas/README;
   - existing `tools/system_map/` standard-library implementation/tests;
   - no generated target artifacts.

### Interfaces / Contracts

```text
python tools/research_index/health.py
python tools/research_index/health.py --refresh
research_health(refresh=false)
```

Health JSON is additive and includes `ready`, `fresh`, `workspace`,
`db_path`, `format_version`, `tool_version`, `generation`, roots, counts,
bounded file-diff rows, and actionable reasons.

`rebuild_index` continues to return the legacy summary string to current
callers. New lifecycle orchestration may use an internal structured rebuild
record rather than parsing that string.

Existing MCP read tools preserve their public signatures and response shapes;
automatic refresh is a precondition, not embedded result data.

`research_validate` through MCP therefore validates the freshly ensured
database and current corpus/link quality. Callers that specifically need to
inspect whether a rebuild is pending use `research_health(refresh=false)`.
The CLI validator remains non-mutating and can still report an existing stale
database.

System Map commands and schemas remain exactly those documented in
`system_map/README.md`.

### Data Flow

```text
MCP read call
  -> inspect stored manifest + current root snapshot + database shape
  -> fresh: execute existing query
  -> stale: validate roots -> build unique temp DB -> atomic DB swap
           -> atomic manifest swap -> execute existing query

System Map ignored research inputs
  -> import/source lock -> tracked registry + reviewed topology
  -> validate + current Git freshness
  -> ignored deterministic target/ render
```

### Error Handling

- Health inspection never creates an absent SQLite database.
- Invalid roots raise a typed lifecycle error before any temporary database is
  created.
- Rebuild exceptions remove only their unique temporary artifact.
- Failed automatic refresh surfaces an MCP error and retains the last good
  database when building or swapping fails.
- Manifest-publication failure leaves a valid replacement database
  uncertified; database identity mismatch prevents it from being treated as
  fresh until a later successful refresh.
- Malformed/missing manifests are stale, not fatal, when the corpus roots can
  rebuild them.
- System Map retains its existing structured diagnostics and nonzero exit
  codes.

### Testing Strategy

- Unit tests for root confinement, missing/empty roots, manifest parsing,
  database shape, and deterministic snapshots.
- Temporary-corpus tests for fresh, added, changed, removed, legacy, and
  workspace-mismatch states.
- Custom-root persistence and effective-root visibility tests.
- Database-identity mismatch test proving an old sidecar cannot certify a
  replaced SQLite file.
- Atomic-failure test proving last-good database bytes remain.
- Validation tests for live link appearance/disappearance and unindexed files.
- Subprocess tests for `health.py` inspect/refresh and exit codes.
- MCP wrapper tests proving read calls refresh once and `research_health`
  remains explicit.
- Existing 34 research-index tests remain green.
- Existing 31 System Map tests remain green.
- `python -m tools.system_map check --ci`.
- `python -m tools.system_map check --require-sources` with the locked local
  source documents.
- Deterministic `render` followed by `render --check`.
- Production-shaped research-index rebuild into ignored worktree cache,
  health inspection, CLI query, and actual stdio MCP call.
- No Cargo.

## Architectural Decisions

- Follow the existing shared-library/thin-shim research-index boundary.
- Keep lifecycle state outside SQLite to avoid a database migration and make
  old-database detection explicit.
- Prefer synchronous fail-closed MCP refresh over serving stale results with a
  warning.
- Retain full atomic rebuild until an incremental graph/reference repair
  contract is designed separately.
- Preserve System Map as dependency navigation, not evidence retrieval or
  completion status.
- Track canonical inputs and source locks; ignore generated views.
- Introduce no third-party Python dependency.

## Alternatives Considered

### Caller-managed validation only

Keep `research_validate` and rely on agents to call `research_reindex`.
Rejected because new files are invisible to indexed validation and the user
explicitly requested automatic maintenance.

### Embed lifecycle metadata in SQLite

Add metadata and per-file snapshot tables. This offers one-file coupling but
forces a migration of the large live database and makes legacy inspection
harder. Rejected for this tranche; a sidecar provides the required generation
contract without changing query storage.

### Background refresh while serving the old database

Return stale results immediately and rebuild in another thread. Rejected
because downstream agents can act on results known to be stale, and process
shutdown complicates completion guarantees.

### Incremental document updates

Mutate changed document rows and repair references/graph edges in place.
Deferred because reverse document links and extracted-term graph
relationships make correctness broader than the apparent file-local update.

### Merge System Map into the research index

Rejected by the approved System Map design. Document retrieval, execution
topology, and parity evidence have different truth and freshness semantics.

## Approval Record

The user explicitly selected both recommended next steps for implementation in
this session.

The first formal design review returned `REVISE` for unspecified custom-root
persistence and overstated two-file atomicity. The design now makes stored
roots authoritative, records database file identity, and treats a
post-database manifest failure as an uncertified generation. The second formal
review returned `APPROVE`; implementation may proceed.
