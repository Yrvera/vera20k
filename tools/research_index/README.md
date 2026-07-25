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
  validate.py
  research_index/
    chunking.py
    database.py
    formatting.py
    metadata.py
    ranking.py
    touchpoints.py
```

Generated state is written to:

```text
tools/research_index/.cache/research.db
```

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
Git-aware Rust-surface freshness:

```powershell
python -m tools.system_map owners --limit 20
python -m tools.system_map show GSI-07.15
python -m tools.system_map loop LOOP-004-HARVEST-CREDIT
```

Neither map is parity proof. Research-index results navigate evidence; System
Map v2 navigates owners and connections.

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

Validation failures mean the index is stale or a cited local link is broken.
Rebuild with `python tools/research_index/index.py` after intentional doc edits.
An explicitly scoped validation that matches zero documents is also invalid:
validating nothing must not certify a research scope.

## MCP Server

The repo-local `.mcp.json` launches:

```powershell
python tools/research_index/mcp_server.py
```

It exposes `research_search`, `research_related`, `research_graph`,
`research_map`, `research_handoff`, `research_validate`, `research_brief`, and
`research_reindex`. Text is the compact default; request JSON only when a
caller needs the full structured rows.

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
- Add an MCP wrapper only after CLI outputs are stable.
