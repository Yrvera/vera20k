# CABHUT No-Overlay Fallback Design

## Goal

Make `BridgeRepairHut` destruction match gamemd.exe when the hut-centered 5x5 scan finds no bridge overlay and the fallback must use bridge flags/ramp evidence.

## Architecture Context

The change belongs in `src/sim/world/bridge_orchestrator.rs`. CABHUT bridge destruction is deterministic simulation behavior reached from C4/demo-truck hut paths in `world_orders`, then routed through `dispatch_bridge_collapse_from_hut`.

Current flow:

- `dispatch_bridge_collapse_from_hut` builds the hut-centered 5x5 scan and chooses low/high bridge family.
- The overlay-first path finds a matching destroy overlay in the 5x5 and calls `run_hut_collapse_bounded`.
- The no-overlay path currently calls `find_hut_fallback_cells`, which returns a traced list of bridge-ish evidence cells.
- The dispatcher then applies `apply_hut_damage_to_cell` to each returned fallback cell until a collapse occurs.
- Final `StateOutcome`s flow through `apply_hut_bridge_outcomes`, which handles occupants, deck drops, debris, rim refresh, event hook, and zone refresh.

The binary fallback is not list-shaped. It picks one starter cell by `CellClass+0x140 & 0x500`, resolves one anchor, walks toward a ramp/endpoint, and performs bounded retry groups. The design should keep this as hut-specific orchestration and reuse existing bridge-state mutation primitives instead of moving it into render/audio/app layers.

## Impact Analysis

Primary files:

- `src/sim/world/bridge_orchestrator.rs`
- `src/sim/world/world_orders_bridge_repair_tests.rs`

Expected internal changes:

- Replace `find_hut_fallback_cells -> Vec<(u16,u16)>` with a small fallback plan/state object, or add a new plan path and remove the traced-list fallback.
- Retire `append_hut_fallback_trace` and `HUT_FALLBACK_TRACE_LIMIT`.
- Add helpers for starter lookup, raw flag retrieval, anchor resolution, pure `0x400` bridgehead walk, ramp probe, endpoint probe, and bounded retry execution.

Blast radius:

- Overlay-first CABHUT behavior must remain unchanged.
- Shared bridge collapse outcomes and final cascade should remain unchanged.
- Determinism is preserved if helper iteration uses fixed arrays and no hash iteration.
- No new sim dependencies on render/audio/ui.

Risk areas:

- Rust currently treats broad runtime bridge evidence as fallback evidence; the binary accepts only `flags & 0x500`.
- Runtime `BridgeRuntimeCell` does not directly store raw `CellClass+0x140`; raw flags should come from `ResolvedTerrainCell.bridge_flags()` unless a verified runtime equivalent is introduced.
- Existing tests are broad and may pass while starter/anchor details are wrong, so focused tests are required.

## Chosen Approach

Use a binary-shaped `HutFallbackPlan` inside `bridge_orchestrator`.

The plan builder:

1. Reads raw bridge flags at the hut cell.
2. If not accepted, searches direction indices `0..7`, distances `1..3`.
3. Stores the selected starter coordinate and flags.
4. Resolves the binary anchor from `0x100`, `0x80`, `0x400`, and `0x800`.
5. Produces either no-op or a ramp-walk execution plan.

The executor:

1. Walks from the anchor in the binary forward direction.
2. Finds the first bridge ramp tile.
3. Calls `apply_hut_damage_to_cell` up to 3 times on the ramp.
4. Walks back toward the endpoint.
5. Optionally calls `apply_hut_damage_to_cell` up to 3 times one cell beyond the endpoint.
6. Returns collected `StateOutcome`s to the existing `apply_hut_bridge_outcomes` cascade.

This keeps gamemd.exe's output-driving behavior local to the hut fallback while preserving the existing outcome/cascade architecture.

## Tiny-Detail Ledger

- The 5x5 overlay scan runs before fallback; fallback only runs if no matching overlay is found. Source: `BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`, `0x00574000`, `0x00574C20`.
- Overlay scan order is `dx = -2..2` outer, `dy = -2..2` inner. Source: same report.
- High overlay fast path accepts `0xCD..=0xE8`; low accepts `0x4A..=0x65`. Source: same report.
- Fallback starter acceptance is exactly `flags & 0x500 != 0`. Source: `0x0057409D..0x00574231`, `0x00574CBD..0x00574E51`.
- Hut input cell is tested before the 8-direction search. Source: same report.
- Direction search order is index `0..7`, matching `N, NE, E, SE, S, SW, W, NW`. Source: same report and `HUT_FALLBACK_DIRS`.
- Distance order is `1`, then `2`, then `3` for each direction before moving to the next direction. Source: same report.
- `0x80` alone does not qualify a starter. Source: same report.
- `0x800` alone does not qualify a starter. Source: same report.
- If no accepted starter is found, return with no damage and no final cascade. Source: early return at `0x00574244`.
- `0x100` plus `0x80` anchors at starter cell's own coordinate. Source: same report.
- `0x100` without `0x80` anchors at `cell+0x2C` coordinate. Source: same report.
- Pure `0x400` with `0x800` clear walks E, with `0x800` set walks S. Source: `0x00574255..0x0057431B`.
- Pure `0x400` then offsets two cells opposite from the first non-`0x400` cell: E walk offsets W; S walk offsets N. Source: same report.
- Four consecutive pure `0x400` continuation cells returns early with no damage. Source: same report.
- Ramp forward direction after anchor is `(flags & 0x800) ? W : N`. Source: same report.
- First ramp retry group is up to 3 `ApplyDamageToCell` calls, stopping early on success. Source: `0x00574350..0x005745CA`.
- Endpoint walk reverses direction after the first ramp. Source: same report.
- If endpoint relative tile difference is `-2`, skip the second retry group. Source: same report.
- Otherwise, second retry group is up to 3 calls one cell beyond endpoint in the original forward direction. Source: same report.
- No-ramp bounds exit still reaches zone rebuild, but does not call adjacent bridge update or tactical dirty. Source: same report.
- Post-ramp exits call adjacent bridge update, set tactical dirty, and rebuild zones. Source: same report.

## Design

### Components

`HutFallbackStarter`

- Internal struct containing `pos: (u16,u16)` and `flags: u32`.
- Created only by exact binary starter acceptance.

`HutFallbackPlan`

- Internal enum:
  - `NoOp`
  - `RampWalk { starter, anchor }`
- `NoOp` covers no accepted starter and pure `0x400` four-continuation early return.

Starter helpers:

- `hut_fallback_flags(sim, pos) -> u32`
- `find_hut_fallback_starter(sim, hut_center) -> Option<HutFallbackStarter>`

Anchor helpers:

- `resolve_hut_fallback_anchor(sim, starter) -> Option<(u16,u16)>`
- `resolve_pure_bridgehead_anchor(sim, starter) -> Option<(u16,u16)>`

Execution helpers:

- `run_hut_fallback_plan(bs, terrain, plan) -> Vec<StateOutcome>`
- `apply_hut_damage_retries(bs, terrain, pos) -> Vec<StateOutcome>`
- `is_hut_fallback_ramp_cell(terrain, pos) -> bool`
- `is_hut_fallback_endpoint_cell(terrain, family, pos) -> Option<i32>` or equivalent endpoint metadata probe.

### Interfaces / Contracts

`dispatch_bridge_collapse_from_hut` should keep its public signature and final cascade. Its internal no-overlay branch changes from "iterate fallback cells" to "build plan and execute plan".

`apply_hut_damage_to_cell` remains the mutation primitive for bridge/ramp damage. The new executor controls which cells receive it and how many retries happen.

Raw fallback starter evidence should use terrain bridge flags, not broad runtime bridge evidence. If a test-only runtime fixture needs starter flags, prefer seeding matching `ResolvedTerrainCell.bridge_facts.raw_flags` in the fixture rather than widening production evidence semantics.

### Data Flow

Overlay path:

1. Build 5x5 scan.
2. Find matching overlay seed.
3. Run bounded collapse.
4. Apply existing final cascade.

No-overlay fallback:

1. Build `HutFallbackStarter` from raw flags.
2. Resolve anchor.
3. Walk to first ramp.
4. Retry damage on ramp.
5. Walk to endpoint.
6. Optionally retry damage one cell beyond endpoint.
7. Feed outcomes into existing final cascade.

### Error Handling

No panics for map bounds, missing terrain, or missing bridge state. Missing data returns `NoOp` or empty outcomes.

Out-of-map coordinate steps should stop the current walk, matching binary sentinel behavior at the observable level: no arbitrary fallback to a different bridge cell.

### Testing Strategy

Add focused tests to `world_orders_bridge_repair_tests.rs`:

- `c4_on_cabhut_fallback_uses_first_flag_starter_not_trace`
- `c4_on_cabhut_fallback_rejects_0x80_only_starter`
- `c4_on_cabhut_fallback_rejects_0x800_only_starter`
- `c4_on_cabhut_pure_bridgehead_clear_0x800_offsets_west`
- `c4_on_cabhut_pure_bridgehead_set_0x800_offsets_north`
- `c4_on_cabhut_pure_bridgehead_four_continuations_noops`
- `c4_on_cabhut_no_overlay_fallback_does_not_trace_second_evidence_cell`
- Keep existing overlay-first terminal tests to prove no regression.

Where full ramp/endpoint fixture setup is too expensive for a world-order integration test, add private module tests in `bridge_orchestrator.rs` for pure plan construction and anchor resolution, plus one integration test that exercises final mutation.

## Architectural Decisions

- Keep this in `bridge_orchestrator` because it is a hut-specific dispatch algorithm, not a general bridge-state primitive.
- Reuse `StateOutcome` and `apply_hut_bridge_outcomes` so occupant/debris/zone/trigger ordering remains shared with the existing bridge collapse cascade.
- Do not add a new crate or ECS abstraction.
- Do not route through render/audio. Collapse sound remains a separate follow-up.
- Do not implement event `0x1F` as part of this design.

Known technical debt:

- Endpoint/ramp predicate fidelity depends on currently available terrain bridge facts. If those facts cannot express the binary `IsBridgeRampTile` / `IsLowBridgeEndpointTile` distinction well enough, implementation must either add a small terrain helper with cited binary semantics or stop for a narrower RE follow-up before approximating.

## Alternatives Considered

### Minimal Single-Starter Patch

Only change `find_hut_fallback_cells` to return one cell. Rejected because it leaves verified parity holes: pure `0x400` anchor math, ramp walk, endpoint tail retry, and side-effect distinctions.

### Move Fallback Into `BridgeRuntimeState`

Could be exact, but it would pull hut/terrain/orchestrator context into the bridge-state layer. Rejected because this path is specific to CABHUT destruction dispatch and the existing orchestrator already owns the final cascade ordering.

### Keep Trace But Add Guards

Rejected. The trace itself is the wrong shape; adding guards would preserve a misleading abstraction and make future parity fixes harder to reason about.
