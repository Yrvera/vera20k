# VERA20k System Map v2

System Map v2 is the dependency-aware navigation graph for parity work. It
connects the canonical GSI registry to native engine services, current Rust
surfaces, typed relationships, and ordered player-visible production loops.

It is **not** a completion ledger and does not certify parity.

## Sources

- `docs/research/GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md`
  supplies canonical GSI IDs, names, families, and discovery scope.
- `docs/research/GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md`
  supplies a historical five-axis baseline. Imported fields are named
  `baseline_*` because the matrix describes an older Rust commit.
- `docs/research/CORE_ENGINE_SERVICES_MAP.md` supplies the 41 service slugs.
- `topology.v2.json` supplies reviewed relationships that prose cannot safely
  infer.
- `mechanisms.v1.json` supplies reviewed semantic contracts that connect
  canonical systems, native anchors, Rust surfaces, and exact loop stages.

Generated source locks preserve the exact source hashes used by an import.

## Files

- `registry.v2.json` — generated normalized 336-row registry and baseline.
- `topology.v2.json` — reviewed service crosswalk, node annotations, edges,
  loops, coupled sets, and legacy slice aliases.
- `mechanisms.v1.json` — reviewed semantic mechanism blocks and typed
  mechanism-to-mechanism handoffs.
- `source-lock.v2.json` — generated source hashes and baseline Rust SHA.
- `schemas/*.schema.json` — data-format contracts.
- `target/system-map-v2/system-map.v2.json` — generated merged map.
- `target/system-map-v2/SYSTEM_MAP_V2.md` — generated human operating view.

`topology.v2.json` and `mechanisms.v1.json` are hand-curated navigation inputs.
Registry/status, loop-stage crosswalks, and freshness views are derived.

## Bootstrap Scope

The first v2 snapshot imports all 336 canonical systems and crosswalks all 41
core services. Its reviewed execution layer is deliberately smaller: 13
annotated load-bearing systems, 55 typed system edges, and 12 ordered stock
player-visible loops. Mechanism v1 deliberately starts with seven blocks from
the already researched power-outage loop: four native ordering edges plus two
cross-owner routing edges. Notification, power-bar, and radar presentation are
separate blocks so unverified renderer-relative order is never implied.
Stages 10–18 of that loop remain explicitly unmapped in the mechanism view.

That is a useful navigation spine, not a complete graph of `gamemd.exe`.
Unannotated systems are `UNMAPPED`, not unimportant. Loop stages currently cite
evidence at loop level; before implementation, the active owner must still pair
each load-bearing native transition with the corresponding production Rust
transition and record any newly verified detail.

The 12 current `GROUP_LOOP_STAGE` warnings are intentional audit markers for
broad inventory rows. Refine them when their loop becomes the active owner,
instead of bulk-decomposing the whole registry.

## Commands

```powershell
python tools/research_index/navigate.py "power outage recovery"
python -m tools.system_map import
python -m tools.system_map check --require-sources
python -m tools.system_map render --output target/system-map-v2
python -m tools.system_map render --check --output target/system-map-v2
python -m tools.system_map show GSI-07.15
python -m tools.system_map loop LOOP-004-HARVEST-CREDIT
python -m tools.system_map mechanism MBLK-004-POWERED-RADAR-GATE
python -m tools.system_map owners --limit 20
python -m tools.system_map stale
```

The research navigator is the preferred broad entry point when both evidence
and topology are needed. It calls the public read-only API in
`tools/system_map/api.py`, preserves System Map as an independent truth domain,
and labels natural-language matches as candidates. Use the standalone commands
for detailed inspection and topology maintenance. Git probes used by validation
and freshness are non-interactive and time-bounded so MCP stdio sessions cannot
inherit a prompt or hang indefinitely.

`owners` is a navigation view over mapped load-bearing systems. It is not a
gap scan and never treats absence from the bootstrap graph as low priority.
Generated reports bind the exact registry, source lock, topology, and mechanism
bytes by SHA-256. `render --check` performs no writes and fails if either
generated view is missing or stale.

## Identity Rules

- Canonical system IDs are exactly `GSI-NN.NN` from the inventory.
- Implementation slices use `SLICE-YYYYMMDD-SLUG`.
- Never invent suffixed systems such as `GSI-04.03A`.
- Historical pseudo-GSI names may appear only in
  `legacy_slice_aliases`, where they are mapped to a `SLICE-*` identity and
  one or more real canonical systems.
- Loop IDs use `LOOP-NNN-SLUG`.
- Edge IDs use `EDGE-NNNN-SLUG`.
- Mechanism block IDs use `MBLK-NNN-SLUG`.
- Mechanism edge IDs use `MBEDGE-NNNN-SLUG`.

## Why mechanisms do not duplicate Ghidra

Ghidra remains the native binary topology: functions, callers, control flow,
data references, and verified addresses. A mechanism block references that
evidence but answers a different question: which native and Rust components
together implement one behavioral contract, who owns each state transition,
what ordering is load-bearing, which systems consume the result, and which
player-visible loop stages it closes.

Do not import generic Ghidra call adjacency into `mechanisms.v1.json`. Add a
block only when an evidence-backed semantic boundary or handoff helps trace or
implement a real production loop.

Loop stage order is a player-journey route, not automatic proof that every
adjacent consumer runs earlier in the same frame. Use a loop `ordering_note`
when presentation or audio order is unresolved, and use cited `MBEDGE-*`
relationships for exact load-bearing order claims.

## Relationship Planes

Every edge declares one plane:

- `native` — verified gamemd relationship; requires cited evidence.
- `rust` — observed current-Rust relationship; requires a source commit.
- `oracle` — comparator/capture relationship.
- `routing` — navigation only and never parity evidence.

Native and Rust edges are intentionally separate. A native edge does not imply
Rust implements or orders the connection correctly.

The same planes apply to `MBEDGE-*` relationships. Mechanism edges stay in the
separate `mechanism_edges` report field and never inflate system-edge metrics.

Core-service `roles` describe the service substrate as a whole. They do not
assert that every listed GSI system has every role. Use typed edges for an exact
per-system authority, ordering, or handoff claim.

## Edge Direction

- `requires`: owner → causal/system prerequisite.
- `loop_requires`: loop owner → another system needed to close one named
  player-visible loop. The edge must name that loop; this is not a claim that
  the target is a prerequisite of the owner's mechanism.
- `owns_state`: authority → consumer; `state` is required.
- `ordered_before`: earlier → later; `context` is required.
- `emits_to` and `handoff_to`: producer → consumer.
- `renders` and `plays_audio`: gameplay producer → presentation consumer.
- `gated_by`: gated system → gate owner.

Only causal `requires` is required to be acyclic. `loop_requires` may point to
upstream fixture producers or downstream closure consumers and is excluded from
causal cycle detection. A reviewed mutually dependent causal slice must be
declared under `coupled_sets`. Read/write/event cycles are normal.

## Freshness Semantics

- `UNMAPPED`: no explicit Rust production surface is recorded.
- `MISSING`: a mapped Rust path no longer exists.
- `DIVERGED`: an observation commit exists but is not an ancestor of current
  `HEAD`.
- `STALE`: at least one mapped path changed after its observation commit, or is
  dirty now.
- `UNRESOLVED`: an observation commit is unavailable, or unchanged coverage is
  only representative and cannot prove freshness.
- `FRESH`: every relevant implementation surface is explicitly exhaustive and
  unchanged. This is intentionally difficult to earn.

Freshness is not parity. An unchanged implementation can still be wrong.
Rust-plane edge evidence participates in the freshness of both connected
systems, so changing a cited relationship file cannot leave the endpoints
apparently unchanged.

Direct mechanism Rust surfaces and Rust-plane mechanism-edge evidence likewise
participate in each connected block's freshness.

Positive loop verdicts are deliberately harder to forge: `TRACE_MATCHED` or
`VERIFIED` requires a reproducible command/result record plus one or more
existing repository-relative artifacts whose recorded SHA-256 matches their
content.

## Maintenance Rule

Enrich the graph while closing real production loops. Do not pause parity work
to annotate all 336 systems, and do not bulk-select work from missing fields.
Each completed slice should:

1. keep its canonical GSI owner(s);
2. add or correct affected typed edges;
3. update exact Rust surfaces and observation SHA;
4. update affected loop stages and oracle gates;
5. run `check --require-sources` and regenerate the views.

When the semantic contract itself changed, also update only the affected
mechanism blocks, edges, and loop memberships. Unmapped mechanism stages are
navigation residuals, not a completion score and not an invitation to bulk-fill
the graph.

Before expanding the canonical mechanism corpus beyond this LOOP-012 pilot,
split `tools/system_map/mechanism_validation.py` by shape, block, evidence, and
edge concerns. The pilot is covered, but growing the dual JSON-Schema/runtime
contract in one file would make drift harder to review.
