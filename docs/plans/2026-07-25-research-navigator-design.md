# Unified Research Navigator Design

## Goal

Provide one bounded MCP and CLI navigation entry point that combines research
evidence with System Map v2 routing without merging their stores or overstating
either source's authority.

## Architecture Context

The research index is a SQLite-backed evidence-navigation tool. Its existing
`research_brief` composition combines document validation, document-oriented
system mapping, implementation handoffs, and exact-anchor graph results
(`tools/research_index/research_index/brief.py`). The MCP server owns index
freshness checks and exposes the library through bounded text or structured JSON
(`tools/research_index/mcp_server.py`).

System Map v2 is a separate canonical JSON topology. Its CLI currently loads and
validates the registry, source lock, and topology through a private loader, then
builds a live report containing systems, edges, loops, services, diagnostics,
and Git-derived freshness (`tools/system_map/cli.py`,
`tools/system_map/report.py`). It deliberately treats topology as navigation,
not parity proof (`AGENTS.md`, “System Map v2 Workflow”).

The navigator belongs at the tooling boundary above both stores. It must reuse
the research-index brief and a new public, read-only System Map API rather than
copying validation logic or importing the System Map CLI's private loader.

## Impact Analysis

Expected changes:

- `tools/system_map/api.py`: public validated report loading and deterministic
  candidate lookup.
- `tools/research_index/research_index/navigator.py`: pure composition of an
  existing research brief with a supplied System Map report.
- `tools/research_index/research_index/navigator_formatting.py`: bounded
  navigator text.
- `tools/research_index/navigate.py`: CLI fallback.
- `tools/research_index/mcp_server.py`: `research_navigate` MCP façade.
- Focused tests and tool documentation.

The change does not touch `src/`, simulation state, game data, System Map
canonical JSON, or the research database schema. Existing MCP and CLI contracts
remain compatible. The main risks are misleading lexical matches, unbounded
handoffs, duplicated loading/freshness work, and coupling the research package
to System Map filesystem internals.

## Chosen Approach

Build a thin façade with separate internals:

1. System Map exposes a public read-only loader that applies the same canonical
   validation as its CLI and fails on validation errors.
2. A deterministic candidate matcher ranks exact canonical IDs first, followed
   by exact textual identity and bounded token matches across system names,
   families, services, and loop metadata.
3. The research navigator accepts the prebuilt report, calls the existing
   research brief once, and returns both truth domains in distinct fields.
4. Exact `GSI-*` or `LOOP-*` input selects that object. Natural language returns
   ranked candidates with match reasons and never labels the highest lexical
   candidate as verified ownership.
5. MCP and CLI are thin adapters over the same composition and formatter.

This provides the requested single front door while preserving independent
evidence, freshness, and authority semantics.

## Player-Experience Detail Ledger

This tooling change has no direct runtime player-visible behavior. The relevant
experience is whether an implementation agent is routed toward complete,
evidence-grounded ordinary-skirmish work.

- `MILESTONE-BLOCKING`: A lexical match must not be presented as verified system
  ownership or parity evidence. Candidate results retain match reasons and
  System Map status/freshness, while the research result retains its own
  citations and validation (`AGENTS.md`, “Research Index Workflow” and “System
  Map v2 Workflow”).
- `MILESTONE-BLOCKING`: Exact canonical system and loop IDs must resolve
  deterministically or return an explicit not-found error. Silent fallback from
  a mistyped canonical ID could route work to the wrong loop
  (`tools/system_map/model.py`, canonical identity rules in `AGENTS.md`).
- `COMPOUNDING`: Zero document matches, zero topology candidates, ambiguous
  candidates, `UNMAPPED` systems, stale surfaces, and validation warnings must
  remain visible. Suppressing them would compound weak research into later
  implementation work (`tools/research_index/research_index/formatting.py`,
  `tools/system_map/report.py`).
- `COMPOUNDING`: Text output must remain bounded while JSON preserves the
  structured result. This prevents oversized/truncated agent handoffs without
  hiding machine-readable detail (`tools/research_index/research_index/handoff.py`,
  `tools/research_index/research_index/formatting.py`).
- `COMPOUNDING`: Canonical System Map validation errors must fail the navigator
  rather than returning partial topology. Warnings remain reportable and
  non-fatal (`tools/system_map/validation.py`, `tools/system_map/model.py`).
- `EXACTIFICATION-RESIDUAL`: Broad natural-language ranking cannot prove semantic
  intent. Trigger: short or generic queries. Frequency: occasional during broad
  exploration. Effect: multiple candidates may need inspection. Downstream
  risk is bounded by explicit candidate labeling and exact-ID refinement.
- `EXACTIFICATION-RESIDUAL`: The currently attached MCP process may need restart
  before the new tool is discoverable. This affects tool availability, not
  stored evidence or gameplay.

## Design

### Components

`tools.system_map.api`

- `load_report(repo, require_sources=True, ci=False)` loads canonical files,
  validates them, raises on errors, and builds live freshness.
- `find_candidates(report, query, limit)` returns deterministic candidate
  systems and loops with scores and match reasons.
- Exact lookup helpers use existing `show_system` and loop report structures.

`research_index.navigator`

- `research_navigate(...)` composes an existing `research_brief` result with the
  supplied System Map report.
- It keeps `research` and `system_map` as separate top-level fields.
- Optional `system_id` and `loop_id` select exact canonical objects.
- Natural-language candidates are still returned for discovery, but never
  replace exact selections.

Adapters:

- MCP calls the existing index freshness guard once, loads live System Map state,
  and returns bounded text or JSON. The MCP adapter runs the synchronous
  filesystem/SQLite/Git request on a worker thread because FastMCP otherwise
  executes synchronous tools on the Windows stdio event-loop thread.
- CLI locates the workspace/index through existing conventions and exposes the
  same parameters and output formats.

### Interfaces / Contracts

Proposed MCP/CLI inputs:

- required natural-language `query`
- optional research `system`, `source_kind`, and binary `anchors`
- optional exact `system_id` and `loop_id`
- bounded `limit`
- `format="text" | "json"`

Structured output:

- `query`
- `research`: unchanged research-brief structure
- `system_map`:
  - validation/freshness summary
  - exact selected system or loop, when requested
  - ranked `system_candidates`
  - ranked `loop_candidates`
  - explicit warnings

Compatibility rules:

- Existing research and System Map commands remain unchanged.
- Candidate arrays are deterministically sorted by score, canonical ID, then
  display name.
- Canonical-looking IDs that do not exist are errors, not fuzzy queries.
- Text limits candidate/detail sections; JSON retains the complete bounded
  result selected by `limit`.

### Data Flow

1. MCP/CLI ensures the research index is fresh.
2. MCP/CLI loads and validates the current System Map report.
3. The navigator builds the existing research brief.
4. Exact IDs are resolved.
5. Query terms are ranked against System Map systems and loops.
6. The formatter presents independent evidence and routing sections plus
   ambiguity/freshness warnings.

### Error Handling

- Missing or invalid canonical System Map files raise `SystemMapError`.
- Unknown explicit IDs raise a navigator input error naming the missing ID.
- Empty queries are rejected.
- Research zero matches are a successful, explicit empty result.
- Topology zero matches are a successful, explicit empty result.
- MCP returns ordinary tool errors for invalid input or canonical-data failures;
  CLI prints the error and exits non-zero.

### Testing Strategy

- Unit-test exact system and loop resolution.
- Unit-test ambiguous lexical ranking and stable ordering.
- Unit-test explicit zero matches and canonical-looking unknown IDs.
- Unit-test propagation of `UNMAPPED`, stale, and diagnostic warnings.
- Unit-test bounded text and JSON structure.
- Extend MCP tests for argument normalization, freshness ownership, text/JSON,
  and System Map failure propagation.
- Run the real CLI against canonical repository data.
- Exercise the new MCP callable directly in-process; note that the already
  attached desktop MCP process requires restart for tool discovery.

## Architectural Decisions

The design follows the existing split between library functions, thin CLI
adapters, and MCP format selection. It adds a public System Map API because
reusing a private CLI loader would be hidden coupling.

System Map data is not copied into SQLite. This avoids schema migration,
conflicting freshness lifecycles, and the false implication that topology is
research evidence. No gameplay or determinism debt is introduced.

Real MCP stdio validation exposed two integration constraints that the façade
owns: already-fresh index inspection must not serialize on the publication
lock, and Git probes must be non-interactive and time-bounded. Freshness now
uses double-checked locking only for stale rebuilds; System Map Git probes use
`DEVNULL` stdin plus explicit timeouts.

## Alternatives Considered

- **MCP-only wrapper:** fewer files, but it would lack the documented CLI
  fallback, be harder to test, and likely depend on private System Map CLI
  internals.
- **Combined persistent index:** could support one global ranking model, but
  would merge evidence and navigation semantics, duplicate freshness state, and
  create migration and authority risks disproportionate to the feature.
- **Exact IDs only:** maximally deterministic, but too weak as a discovery
  navigator. Exact IDs remain supported as the safe refinement path.
