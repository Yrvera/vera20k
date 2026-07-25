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

Generated source locks preserve the exact source hashes used by an import.

## Files

- `registry.v2.json` — generated normalized 336-row registry and baseline.
- `topology.v2.json` — reviewed service crosswalk, node annotations, edges,
  loops, coupled sets, and legacy slice aliases.
- `source-lock.v2.json` — generated source hashes and baseline Rust SHA.
- `schemas/*.schema.json` — data-format contracts.
- `target/system-map-v2/system-map.v2.json` — generated merged map.
- `target/system-map-v2/SYSTEM_MAP_V2.md` — generated human operating view.

Only `topology.v2.json` contains hand-curated topology. Registry/status and
freshness views are derived.

## Bootstrap Scope

The first v2 snapshot imports all 336 canonical systems and crosswalks all 41
core services. Its reviewed execution layer is deliberately smaller: 13
annotated load-bearing systems, 53 typed edges, and 12 ordered stock
player-visible loops. The initial edge balance is 44 routing, 8 native, and 1
Rust edge; it exposes how much native/Rust pairing still has to be added.

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
python -m tools.system_map import
python -m tools.system_map check --require-sources
python -m tools.system_map render --output target/system-map-v2
python -m tools.system_map render --check --output target/system-map-v2
python -m tools.system_map show GSI-07.15
python -m tools.system_map loop LOOP-004-HARVEST-CREDIT
python -m tools.system_map owners --limit 20
python -m tools.system_map stale
```

`owners` is a navigation view over mapped load-bearing systems. It is not a
gap scan and never treats absence from the bootstrap graph as low priority.
Generated reports bind the exact registry, source lock, and topology bytes by
SHA-256. `render --check` performs no writes and fails if either generated view
is missing or stale.

## Identity Rules

- Canonical system IDs are exactly `GSI-NN.NN` from the inventory.
- Implementation slices use `SLICE-YYYYMMDD-SLUG`.
- Never invent suffixed systems such as `GSI-04.03A`.
- Historical pseudo-GSI names may appear only in
  `legacy_slice_aliases`, where they are mapped to a `SLICE-*` identity and
  one or more real canonical systems.
- Loop IDs use `LOOP-NNN-SLUG`.
- Edge IDs use `EDGE-NNNN-SLUG`.

## Relationship Planes

Every edge declares one plane:

- `native` — verified gamemd relationship; requires cited evidence.
- `rust` — observed current-Rust relationship; requires a source commit.
- `oracle` — comparator/capture relationship.
- `routing` — navigation only and never parity evidence.

Native and Rust edges are intentionally separate. A native edge does not imply
Rust implements or orders the connection correctly.

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
