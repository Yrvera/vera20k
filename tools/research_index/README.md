# VERA20k Research Index

Local evidence index for VERA20k research docs. This is a small, repo-specific
retrieval tool for turning research files into cited implementation evidence.

V1 is deliberately simple:

- SQLite FTS5 full-text search.
- Markdown/INI chunks with file and line citations.
- Source-kind and status inference.
- Address, symbol, INI key, Rust path, and markdown link extraction.
- Related-document lookup by shared extracted terms.
- Deterministic docgraph edges for document navigation.
- CLI and FastMCP entry points over the same library contracts.
- No embeddings or chat UI.

## Layout

```text
tools/research_index/
  schema.sql
  index.py
  search.py
  related.py
  graph.py
  handoff.py
  map.py
  brief.py
  navigate.py
  validate.py
  health.py
  research_index/
    chunking.py
    database.py
    formatting.py
    lifecycle.py
    locking.py
    metadata.py
    navigator.py
    navigator_formatting.py
    ranking.py
    touchpoints.py
```

Generated state is written to:

```text
tools/research_index/.cache/research.db
tools/research_index/.cache/research.db.meta.json
```

The sidecar manifest records the exact indexed roots, corpus file identities,
index-builder source signature, tool format, and published database identity.
Both files are generated and ignored by Git.

## Build The Index

```powershell
python tools/research_index/index.py
```

Default indexed roots:

- `docs/research`
- `docs/plans`
- `ini`

You can override roots:

```powershell
python tools/research_index/index.py docs/research/bridges
```

Explicit root overrides are persisted in the generation manifest, so automatic
MCP refreshes keep the same scope. Roots must stay inside the workspace and
contain at least one indexable Markdown or INI file; an unsafe or empty request
cannot replace a valid database.

Rebuilds use unique sibling temporary databases plus atomic replacement. A
cross-process lock serializes publication, and the new generation is certified
only when the corpus is unchanged across the rebuild. Already-fresh reads use a
lock-free inspection path; stale reads recheck under the publication lock before
rebuilding, so multiple MCP sessions do not serialize ordinary lookups.

## Freshness Health

Inspect the database, manifest, and current corpus without changing anything:

```powershell
python tools/research_index/health.py
python tools/research_index/health.py --json
```

Synchronously rebuild only when stale:

```powershell
python tools/research_index/health.py --refresh
```

The health command reports added, changed, and removed files with bounded text
output. It exits nonzero when inspection is stale or not ready. Freshness uses
file size plus nanosecond modification time; an unusual same-size edit that
also preserves the exact timestamp requires an explicit reindex.

## Search

```powershell
python tools/research_index/search.py "BridgeRepairHut C4"
python tools/research_index/search.py --system bridges "Can_Enter_Cell"
python tools/research_index/search.py --source ghidra "0x00574000"
python tools/research_index/search.py --json "low bridge TubeClass height"
```

Results include:

- path
- heading
- line range
- source kind
- status
- ranked snippet

## Related Docs

Find docs sharing symbols, addresses, INI keys, or Rust paths with a source doc:

```powershell
python tools/research_index/related.py docs/research/bridges/00-system-models/BRIDGE_SYSTEM.md
```

Find docs related to an exact term:

```powershell
python tools/research_index/related.py --term BridgeRepairHut
```

## Docgraph

The index also builds deterministic graph edges from extracted evidence:

- `references_doc`
- `mentions_symbol`
- `mentions_address`
- `mentions_ini_key`
- `mentions_rust_path`
- `belongs_to_system`
- `belongs_to_subsystem`
- `has_source_kind`
- `has_status`

Graph commands:

```powershell
python tools/research_index/graph.py doc docs/research/bridges/00-system-models/BRIDGE_SYSTEM.md
python tools/research_index/graph.py backlinks docs/research/bridges/00-system-models/BRIDGE_SYSTEM.md
python tools/research_index/graph.py evidence BridgeRepairHut
python tools/research_index/graph.py implementation BridgeRepairHut
python tools/research_index/graph.py evidence 0x00574000 --json
```

`evidence` prioritizes research-document relationships. `implementation`
prioritizes Rust paths mentioned by the matching docs. Implementation views
also report `exists=yes|no` against the selected workspace; existence is a
freshness clue, not proof that the file still owns the cited behavior.
Hex-address lookups ignore case and leading zero padding, so `0x73e5e0` and
`0x0073E5E0` resolve the same extracted address.

## Parity Handoff

Build an implementation-oriented handoff before changing Rust:

```powershell
python tools/research_index/handoff.py "Can_Enter_Cell zone passability"
python tools/research_index/handoff.py "chrono miner refinery unload implementation handoff"
python tools/research_index/handoff.py --system bridges "collapse"
```

The handoff view combines:

- explicit implementation-handoff sections when present;
- top verified evidence chunks with file and line citations;
- Rust touchpoints from extracted graph terms, including supporting doc citations;
- warnings when evidence, handoff sections, or Rust touchpoints are missing.

Handoffs require meaningful query-term coverage before presenting evidence as
implementation guidance. Generic corpus words such as `research`, `handoff`,
and `Rust` cannot make an unrelated document match by themselves. Default text
is a bounded summary with explicit omission counts; `--json` retains the
detailed structured bundle and can be much larger.

## System / Topic Map

List the research inventory for a system or topic:

```powershell
python tools/research_index/map.py --system bridges
python tools/research_index/map.py --system bridges collapse
python tools/research_index/map.py --system bridges collapse --source ghidra
```

The map view groups matching documents by subsystem/source/status, lists the
matching docs, and surfaces implementation-handoff plus contradiction,
supersession, stale, and uncertainty sections when those headings or phrases are
present.

This document/topic map is distinct from the engine execution topology under
`system_map/`. Use the standalone System Map v2 CLI for canonical GSI systems,
typed dependency/authority edges, service crosswalks, production loops, and
semantic mechanism blocks with Git-aware Rust-surface freshness:

```powershell
python -m tools.system_map owners --limit 20
python -m tools.system_map show GSI-07.15
python -m tools.system_map loop LOOP-004-HARVEST-CREDIT
python -m tools.system_map mechanism MBLK-004-POWERED-RADAR-GATE
```

Neither map is parity proof. Research-index results navigate evidence; System
Map v2 navigates owners and connections.

## Unified Navigator

Use the navigator when a topic needs both cited research evidence and
dependency-aware routing:

```powershell
python tools/research_index/navigate.py "power outage recovery"
python tools/research_index/navigate.py "GSI-09.07"
python tools/research_index/navigate.py "MBLK-004-POWERED-RADAR-GATE"
python tools/research_index/navigate.py "harvest credit" --system-id GSI-07.39 --loop-id LOOP-004-HARVEST-CREDIT
python tools/research_index/navigate.py "radar recovery" --mechanism-id MBLK-004-POWERED-RADAR-GATE
python tools/research_index/navigate.py --json "bridge collapse"
```

The navigator is a thin façade, not a combined store. Its `research` field is
the existing evidence brief; its `system_map` field contains live topology,
freshness, exact selections, and ranked candidates. Natural-language candidates
are always labelled as navigation candidates rather than verified owners,
parity evidence, or completion claims.

Exact `GSI-*`, `LOOP-*`, and `MBLK-*` queries select their canonical object.
An exact mechanism query uses the block's bounded `research_query` plus its
verified native anchors to seed the research brief, while preserving the
original outer query and disclosing the effective seed. An explicit
`--mechanism-id` alongside natural language selects the mechanism without
replacing that natural-language research query. Unknown exact IDs fail instead
of falling back to fuzzy results. Zero matches return
`matched=false`; the CLI exits nonzero when neither domain matched or when
matched research fails live validation. Text output is bounded, while JSON
retains the structured bundle within fixed limits of 20 rows per section, eight
anchors, and 40 diagnostics.

## Pre-Implementation Brief

Build a compact planning bundle before editing Rust:

```powershell
python tools/research_index/brief.py --system miner "Mission_Harvest State 2 chrono miner return teleport drive" --anchor 0x73e5e0
```

The brief combines topic-map results, document validation, implementation
handoff candidates, Rust touchpoints, top evidence, and optional exact
symbol/address anchors.

## Validate Indexed Docs

Check that indexed docs still exist, match their indexed checksum, and have no
missing local markdown links:

```powershell
python tools/research_index/validate.py --system bridges collapse
python tools/research_index/validate.py --system miner "Mission_Harvest State 2"
```

The CLI validation command is deliberately non-mutating and detects changed,
missing, and newly added files plus local links against the live filesystem.
Rebuild with `python tools/research_index/index.py` or
`python tools/research_index/health.py --refresh` after intentional edits. An
explicitly scoped validation that matches zero documents is also invalid:
validating nothing must not certify a research scope.

## MCP Server

The repo-local `.mcp.json` launches:

```powershell
python tools/research_index/mcp_server.py
```

It exposes `research_search`, `research_related`, `research_graph`,
`research_map`, `research_handoff`, `research_validate`, `research_brief`,
`research_navigate`, `research_reindex`, and `research_health`. Text is the
compact default; request JSON only when a caller needs the full structured
rows.

Every evidence-reading MCP tool checks freshness before opening the index. If
the corpus changed, the call waits for one synchronous, locked rebuild and then
continues against the certified generation. Failures are surfaced instead of
serving stale evidence. `research_health(refresh=false)` is the non-mutating way
to inspect pending changes; `research_validate` refreshes first and then checks
the requested scope and live local links. `research_navigate` also validates
canonical System Map sources and recomputes live Git freshness on every call.
Restart an MCP process that predates the tool before expecting it to appear in
tool discovery.

## Ranking Model

V1 uses a conservative evidence preference:

1. Ghidra reports
2. traces
3. implementation contracts
4. system model syntheses
5. Rust audits
6. plans
7. unknown notes

The ranking is a retrieval hint only. `gamemd.exe` evidence and verified research
remain the source of truth.

## Next Steps

- Add explicit YAML frontmatter to high-value docs for `status`,
  `supersedes`, and `superseded_by`.
- Add a `contract.py` command that emits implementation-checklist drafts from
  cited chunks.
- Add optional semantic retrieval after FTS and metadata ranking are trusted.
