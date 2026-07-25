# System Map v2 Design

## Goal

Turn the existing broad GSI taxonomy into a typed, freshness-aware execution
graph that can guide dependency-first parity work without becoming a
hand-maintained completion ledger.

The user approved this direction on 2026-07-25 after reviewing the proposed
machine-readable graph, player-loop catalog, service crosswalk, freshness
tracking, and separate slice-ID namespace.

## Architecture Context

The project already has four useful but disconnected views:

- `GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md` defines 336
  canonical GSI candidates across 18 families.
- `GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md` provides a
  conservative five-axis status baseline, but its Rust snapshot is commit
  `a97ce88454d2ab938e6f8892dcac861845302c09`.
- `CORE_ENGINE_SERVICES_MAP.md` defines 41 native service substrates and a
  service-level dependency graph.
- `CURRENT_ARCHITECTURE.md` and current Rust identify implementation owners,
  but do not cross-link them to canonical GSI rows.

`tools/research_index` indexes documents and evidence terms. Its existing
`system_map` means "documents belonging to a topic"; it is not an engine
topology. `tools/parity_ledger` tracks active obligations and dispositions;
using it as the engine map would wrongly mix navigation with completion
semantics.

System Map v2 therefore lives in its own `system_map/` data surface and
`tools/system_map/` standard-library CLI. It may cite research-index evidence,
but does not replace either existing tool.

## Impact Analysis

### Added surfaces

- `system_map/registry.v2.json`: generated normalization of all inventory and
  status-matrix rows.
- `system_map/topology.v2.json`: reviewed annotations, service crosswalk,
  typed/planed edges, player loops, Rust surfaces, and legacy slice aliases.
- `system_map/source-lock.v2.json`: source hashes, baseline Rust SHA, and
  normalized registry digest.
- `system_map/schemas/*.json`: format contracts.
- `tools/system_map/`: importer, validator, query layer, freshness analysis,
  deterministic renderer, and unit tests.
- `target/system-map-v2/`: generated merged JSON and Markdown.
- `docs/research/SYSTEM_MAP_V2.md`: optional generated navigation view with a
  prominent non-authority banner.

### Existing surfaces updated

- `AGENTS.md`: add the System Map v2 navigation workflow and commands.
- `tools/research_index/README.md`: distinguish the document map from the
  engine map and link to the new CLI.

### Non-scope

- No Rust gameplay, INI, asset, Ghidra, or Oracle mutation.
- No claim that the 336-row inventory is exhaustive.
- No mass decomposition of all 77 group nodes.
- No hand-edited parity percentage or completion declaration.
- No automatic work selection based on missing code.

## Chosen Approach

Use a normalized registry plus a small curated topology source.

The importer derives the full 336-node registry from the existing inventory and
status matrix. Human judgment is limited to relationships that cannot be
reliably inferred: service crosswalks, typed GSI edges, exact Rust surfaces,
native anchors, loop stages, and legacy-ID corrections.

The merged map is generated deterministically. All matrix fields are named
`baseline_*`; they are never presented as current truth. Current Git evidence
can prove that a mapped Rust surface changed and is therefore stale. An
unchanged representative path remains `UNRESOLVED`; only explicitly exhaustive
coverage may become `FRESH`.

## Tiny-Detail Ledger

- Preserve all 336 canonical IDs and their exact inventory names.
  [inventory: `GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md`
  Stable-ID candidate system registry]
- Preserve the five independent matrix axes; never collapse them into one
  progress value.
  [matrix: `GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md`
  Controlled status model]
- Record the matrix Rust snapshot SHA and source hashes. A newer checkout makes
  the baseline globally old; it does not silently rewrite statuses.
- Reject pseudo-GSI suffixes such as `GSI-04.03A`. Use `SLICE-*` IDs and map
  them back to one or more canonical GSI rows.
- Every edge carries `plane = native | rust | oracle | routing`. A native edge
  never implies the Rust edge matches.
- Edge direction is fixed by kind:
  - `requires`: owner to prerequisite;
  - `loop_requires`: loop owner to another stage required to close one named
    player-visible loop, without asserting a causal prerequisite;
  - `owns_state`: authority to consumer, with named state;
  - `ordered_before`: earlier to later, with named context;
  - `emits_to`/`handoff_to`: producer to consumer;
  - `renders`/`plays_audio`: gameplay producer to presentation consumer.
- Native-plane claims require a cited verified document/address. Rust-plane
  claims require an observed commit and exact path/symbol. Routing edges are
  explicitly non-evidence.
- Detect cycles only in `requires`. Cycles in read/write/event relationships
  are legal. A required cycle must be declared as a reviewed coupled set.
- Every loop uses canonical GSI stages with unique contiguous order, a stock
  fixture, player-visible result, native/Rust entry fields, and an honest
  oracle gate.
- Loop-level citations establish the bootstrap trace. Before implementation,
  each load-bearing stage/transition must still be paired against native and
  production-Rust evidence; the map does not infer that pairing.
- Unknown anchors, callers, or oracles remain `UNCHECKED`; do not invent them.
- `TRACE_MATCHED` and `VERIFIED` require a reproducible verification record and
  existing repository-relative proof artifacts locked by SHA-256.
- Rendered JSON and Markdown sort IDs, edges, services, and loops
  deterministically and contain no wall-clock timestamp.
- Generated reports record SHA-256 for registry, source-lock, and topology
  inputs plus a stable generator version; `render --check` detects stale output
  without writing.
- Dirty or changed mapped Rust paths can only downgrade freshness.
- Rust-edge evidence is attributed to both endpoints for freshness, so a changed
  file cannot leave a reviewed Rust relationship apparently current.
- Representative path coverage can never prove `FRESH`.
- Generated owner views rank only mapped, active, load-bearing relationships;
  absence from the bootstrap graph is `UNMAPPED`, never low priority.
- The map stores no task ownership, branch/worktree state, or parity completion
  status.

## Design

### Components

1. **Registry importer**
   - Parses the 336 inventory rows and 336 matrix rows.
   - Verifies exact ID/name equality and controlled statuses.
   - Writes normalized baseline records and source hashes.

2. **Topology source**
   - Cross-links all 41 core-service slugs to canonical GSI rows.
   - Stores reviewed node annotations, typed edges, loop definitions, and
     legacy slice aliases.
   - Starts with the load-bearing spine and approximately 10–12 stock loops.

3. **Validator**
   - Enforces the executable structural contract represented by the published
     schemas, plus IDs, paths, evidence citations, edge endpoints, planes, loop
     order, address syntax, source locks, and `requires` cycles.
   - Treats known legacy suffix IDs as aliases and rejects new unknown ones.

4. **Freshness analyzer**
   - Compares topology Rust surfaces with their `observed_at_commit`, current
     `HEAD`, and dirty paths.
   - Reports `STALE`, `UNRESOLVED`, or `FRESH` under the strict coverage rule.

5. **Query and renderer**
   - Shows one GSI node, loop, stale set, or owner-candidate view.
   - Produces merged JSON plus a concise Markdown operating view.

### Interfaces / Contracts

Primary commands:

```text
python -m tools.system_map import
python -m tools.system_map check --ci
python -m tools.system_map check --require-sources
python -m tools.system_map render --output target/system-map-v2
python -m tools.system_map render --check --output target/system-map-v2
python -m tools.system_map show GSI-07.15 --json
python -m tools.system_map loop LOOP-004-HARVEST-CREDIT --json
python -m tools.system_map owners --limit 20
python -m tools.system_map stale
```

Canonical data is accepted only after `check --require-sources` succeeds.
Generated output carries the source-lock hashes and current Git comparison.

### Data Flow

```text
inventory Markdown ─┐
status matrix ───────┼─ import/validate ─ registry.v2.json
core-services map ──┘                  └─ source-lock.v2.json

registry.v2.json + topology.v2.json + current Git state
    └─ build/query/freshness
        ├─ target/system-map-v2/system-map.v2.json
        ├─ target/system-map-v2/SYSTEM_MAP_V2.md
        └─ optional docs/research/SYSTEM_MAP_V2.md
```

### Error Handling

The CLI exits non-zero with stable diagnostic codes for malformed source rows,
ID mismatches, dangling edges, illegal pseudo-GSI IDs, missing paths/evidence,
invalid loop order, stale source locks, and unacknowledged prerequisite cycles.
Warnings never upgrade evidence or freshness.

### Testing Strategy

- Unit fixtures for inventory and matrix parsing.
- Exact set/name reconciliation tests.
- Edge-plane and kind validation tests.
- Dependency-cycle and coupled-set tests.
- Loop order and endpoint tests.
- Pseudo-GSI rejection and legacy-alias tests.
- Freshness tests using temporary Git repositories.
- Deterministic render golden-in-memory comparison.
- Live-corpus test for 336 rows, 18 families, 77 group nodes, and 41
  cross-walked services.

## Architectural Decisions

- Keep engine topology separate from research document retrieval and parity
  obligation tracking.
- Use JSON and Python standard library only.
- Keep derived baseline status explicit and stale-aware.
- Make native, Rust, Oracle, and routing planes distinct.
- Enrich the graph incrementally as real closed loops are traced.
- Store durable canonical inputs under `system_map/`; generated views go under
  `target/` and ignored docs.

## Alternatives Considered

### Extend `research_index.system_map`

Rejected because that API groups documents by topic. Overloading it with GSI
execution topology would make both meanings ambiguous.

### Extend `parity_ledger`

Rejected because obligation/disposition semantics encourage treating a
navigation graph as completion truth.

### Hand-edit a larger Markdown matrix

Rejected because it would age immediately, be difficult to validate, and repeat
the stale-status problem System Map v2 is intended to solve.
