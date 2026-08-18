# Pathfinder 0x0042ACF0 Object +0x678 Gate - Ghidra Research Report

**Address(es):** `0x0042ACF0` primary; supporting `0x00429A90`, `0x0042C900`, `0x004CBBA0`, `0x0070EFE0`, `0x00712170`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the peer-object eligibility gate around `vtable+0x84 -> TypeClass + 0x678` inside `PathfinderClass::UpdateBridgePassability`, its live A* caller path, and the Rust implication for temporary `CellClass+0x140 & 0x40000` bridge-cost marker generation from nearby moving objects.
**Non-Scope:** full `UpdateBridgePassability` geometry, 5x5 fallback details, tube path offset table, `AStar_compute_edge_cost` entity soft-block chain, and general `TechnoTypeClass` layout beyond the `Speed=` field.
**Confidence:** High for the scoped gate and live caller path.
**Active in YR:** Yes, conditional on A* running with `PathfinderClass+0x3C != 0`.

## Target Question

What exactly is the `+0x678` field read in the peer-path owner/rank comparison inside `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, and how should Rust use that fact when generating temporary `0x40000` bridge-cost markers from nearby moving objects?

## Non-Goals

- Do not re-investigate the settled `0x40000` cost multiplier in `AStar_compute_edge_cost`.
- Do not re-derive the full 24-entry peer path propagation geometry.
- Do not model or patch Rust behavior in this report.
- Do not treat the decompiler's ambiguous local names as struct names without assembly support.

## Evidence Needed To Mark COMPLETE

- Decompile and assembly evidence that `0x0042ACF0` compares `+0x678` on type pointers returned by object virtual slot `+0x84`.
- Decompile and assembly evidence that `TechnoTypeClass+0x678` is the parsed/scaled `Speed=` field.
- Decompile and xref/assembly evidence that `0x0042ACF0` is live from the A* path.
- Rust scan sufficient to name the affected pathfinding and movement surfaces.

## Stop Conditions

- Stop once the `+0x678` semantic is resolved to a named parsed field and the comparison polarity is verified.
- Stop if the only unresolved items require runtime debugger observation rather than static decompile/assembly.
- Stop before writing Rust, INI, or in-repo docs.

## 1. Overview

The scoped `+0x678` read is not an `ObjectClass` runtime owner/rank field. `PathfinderClass::UpdateBridgePassability` first calls virtual slot `+0x84` on the searching object and each peer object, then compares `TypeClass+0x678`; supporting `TechnoTypeClass::ReadINI` and `TechnoClass::GetTypeSpeed` prove that field is the per-type movement `Speed=` value after clamping/scaling.

Player-visible implication: normal urgency peer-path `0x40000` marking is only allowed from slower peer types when the searching unit's type speed is strictly greater. Equal-speed peers and same-type peers do not contribute peer-path markers under the normal gate; urgency `2` bypasses this speed/type/playfield gate.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Meaning | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `vtable+0x84` | Object/Techno runtime object | virtual method | Returns the object's type pointer for this pathfinding code | `0x0042AE01`, `0x0042AE44` calls; `TechnoClass::GetTypeSpeed @ 0x0070EFE0` calls same slot before reading `+0x678` | Yes |
| `+0x678` | `TechnoTypeClass` | int | Parsed/scaled `Speed=` value, clamped to `0..255`; `Speed=-1` leaves prior/default value | parser write `0x0071464C..0x00714699`; reader `0x0070EFE0` | Yes |
| `+0x3C` | `PathfinderClass` | int | Search urgency / retry mode; `2` bypasses the speed/type/playfield gate | `0x0042AE4E`; set from `AStar_pathfind_search` param at `0x0042C900` | Yes when nonzero |
| `+0x5E0` | peer object | int[?] direction queue | Peer path direction queue base, terminator `-1` | `0x0042AE88`, `0x0042AE90`, `0x0042AFB2` | Yes |
| `+0x558` | peer object | packed cell coord | Peer path propagation starts from this current/reference coord | `0x0042AE33` | Yes |
| `CellClass+0x140 bit 0x40000` | cell | bit | Temporary cost marker toggled by this function and consumed by A* cost | prior report plus local `0x0042AF93..0x0042AFAE` masked write | Yes |

## 3. Core Logic

Normal peer-path eligibility in `0x0042ACF0` is:

1. Resolve the searching object's type once through `searcher->vtable+0x84`; assembly stores this in `ESI` after `0x0042AE01`.
2. For each peer object of `WhatAmI()==1` or `WhatAmI()==0xF`, resolve the peer type through `peer->vtable+0x84` at `0x0042AE44`.
3. If `PathfinderClass+0x3C == 2`, bypass the normal gate and inspect the peer path.
4. Otherwise skip the peer when the type pointers are equal (`CMP ESI,EAX; JZ skip`).
5. Otherwise load `searcher_type+0x678` into `EDX`, load `peer_type+0x678` into `ECX`, compare, and skip on `searcher_speed <= peer_speed` (`JLE skip`).
6. If the speed/type gate passed, also require `MapClass::Is_Cell_In_Playfield(..., 1)` to return nonzero.
7. Eligible peer paths then start at the peer's path queue base (`object+0x5E0`) and stop after 24 entries or `-1`.

Tiny details that matter:

- The comparison is strict. Equal `Speed=` values do not mark peer paths under urgency `0/1`.
- The same type pointer always skips under urgency `0/1`, even if two instances are different objects.
- The compared field belongs to the type pointer, not the peer object itself.
- The comparison is signed integer assembly (`CMP EDX,ECX; JLE`), but parser evidence constrains normal `Speed=` stores to `0..255` after conversion unless `Speed=-1` preserves an existing/default value.
- Urgency `2` bypasses all three normal checks in this block: same-type skip, speed ordering, and playfield validation.
- The function then toggles cost markers; it does not change cell walkability or occupancy.

## 4. INI Keys

| INI key | Owner | Binary storage | Default/read behavior | Effect here | Active in YR |
|---|---|---|---|---|---|
| `Speed=` | `TechnoTypeClass` | `+0x678` | `ReadInt(..., default=-1)`; `-1` skips the store; otherwise clamp input to max `100`, coerce nonpositive to `0`, scale by `*256/100`, clamp to `255` | Higher scaled type speed can inspect and mark slower peer paths | Yes |

No INI key directly configures the `0x40000` marker or the `PathfinderClass+0x3C` gate in this slice.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Run_AStar @ 0x004CBBA0` | Calls `AStar_pathfind_search` with global Pathfinder `0x0087E8B8` and passes the urgency argument through | call assembly at `0x004CBC31`; decompile shows `AStar_pathfind_search(..., param_5)` | Yes |
| `AStar_pathfind_search @ 0x0042C900` | Stores caller urgency into `PathfinderClass+0x3C`, then calls `AStar_main_loop` | decompile `*(uint *)(param_1+0x3C)=param_8`; caller `FootClass__Run_AStar` | Yes |
| `AStar_main_loop @ 0x00429A90` | Calls `UpdateBridgePassability` before search and on success/failure cleanup when `+0x3C != 0` | xrefs `0x00429C1A`, `0x0042A42D`, `0x0042A44C`; assembly tests `Pathfinder+0x3C` before calls | Yes, conditional on nonzero urgency |
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | Selects nearby peer objects and toggles temporary `0x40000` markers | primary decompile/assembly | Yes |
| `TechnoClass::GetTypeSpeed @ 0x0070EFE0` | Independent reader proving `Type+0x678` is type speed | decompile and assembly `CALL [vtable+0x84]`, `MOV EAX,[EAX+0x678]` | Yes |
| `TechnoTypeClass::ReadINI @ 0x00712170` | Parser proving `Speed=` writes `Type+0x678` | xref from string `0x0081D9CC`; assembly `0x0071464C..0x00714699` | Yes |

## 6. Current Rust Implementation Status

Rust has entity soft-block costs and urgency in A*, but not this temporary peer-path `0x40000` marker layer:

- `src/sim/pathfinding/core.rs` has `AStarOptions::entity_block_map` and `urgency`, plus code-2 friendly mover cost handling at lines 847-864 and 1607-1645. This is the separate `AStar_compute_edge_cost` entity cost path, not the `0x40000` marker generator.
- `src/sim/movement/bump_crush.rs` builds `entity_block_map` from current and next cells only at lines 101-106 and 185-199. It does not scan a peer direction queue up to 24 entries or apply a type-speed ordering gate.
- `src/sim/movement/movement_path.rs` passes `entity_block_map` and `urgency` into flat/layered A* at lines 142-183 and 236-248. There is no search-scoped cost overlay equivalent to `CellClass+0x140 & 0x40000`.
- `PathGrid` remains static/layered terrain data. That is correct as a base structure, but the temporary marker must not be baked into it.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0042ACF0` `+0x678` peer gate | verified | decompile plus assembly `0x0042AE01..0x0042AE66` | none |
| Same-type skip polarity | verified | `0x0042AE54..0x0042AE56` | none |
| Strict speed ordering polarity | verified | `0x0042AE58..0x0042AE66` | none |
| Urgency `2` bypass | verified | `0x0042AE4E..0x0042AE52` | none |
| `TechnoTypeClass+0x678 = Speed=` | verified | parser assembly `0x0071464C..0x00714699`; reader `0x0070EFE0` | none |
| Live A* caller path | verified | xrefs/callers `0x004CBC31 -> 0x0042C900 -> 0x00429A90 -> 0x0042ACF0` | none |
| Rust pathfinding surfaces | verified | `core.rs`, `bump_crush.rs`, `movement_path.rs` scan | exact implementation design remains future work |
| Full 5x5 fallback | deferred | prior report covers it | out of scope for this field-gate slice |
| Full tube direction-8 propagation | deferred | prior report covers high level | out of scope for this field-gate slice |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is `+0x678` read from the object or from a type pointer? -> Type pointer returned by virtual slot `+0x84`; not direct object storage.` (evidence: `0x0042AE01`, `0x0042AE44`, `0x0042AE58`; Active in YR: Yes)
- `[RESOLVED] OQ-002 - What field is `TechnoTypeClass+0x678`? -> Parsed/scaled `Speed=` field.` (evidence: `0x0071464C..0x00714699`, `0x0070EFE0`; Active in YR: Yes)
- `[RESOLVED] OQ-003 - What is the comparison polarity? -> Searcher type speed must be strictly greater than peer type speed; `JLE` skips equal/slower searchers.` (evidence: `0x0042AE58..0x0042AE66`; Active in YR: Yes)
- `[RESOLVED] OQ-004 - Does same type skip before speed compare? -> Yes for urgency `0/1`; `CMP ESI,EAX; JZ skip`.` (evidence: `0x0042AE54..0x0042AE56`; Active in YR: Yes)
- `[RESOLVED] OQ-005 - Does urgency bypass the gate? -> `PathfinderClass+0x3C == 2` jumps directly to peer path inspection.` (evidence: `0x0042AE4E..0x0042AE52`; Active in YR: Conditional)
- `[RESOLVED] OQ-006 - Is `0x0042ACF0` live in standard A*? -> Yes, called by `AStar_main_loop` when `Pathfinder+0x3C != 0`.` (evidence: xrefs `0x00429C1A`, `0x0042A42D`, `0x0042A44C`; Active in YR: Conditional)
- `[RESOLVED] OQ-007 - Who sets `Pathfinder+0x3C`? -> `AStar_pathfind_search` stores its last argument into `+0x3C`.` (evidence: `0x0042C900` decompile; Active in YR: Yes)
- `[RESOLVED] OQ-008 - What is the live caller from movement? -> `FootClass::Run_AStar` calls `AStar_pathfind_search` at `0x004CBC31` and passes its urgency argument.` (evidence: `0x004CBBA0` decompile; `0x004CBC31` assembly; Active in YR: Yes)
- `[RESOLVED] OQ-009 - Is the playfield check part of the normal speed/type gate? -> Yes, after strict speed pass and before peer path inspection; urgency `2` bypasses it.` (evidence: `0x0042AE68..0x0042AE7B`; Active in YR: Yes)
- `[RESOLVED] OQ-010 - What Rust surface currently approximates nearby movers? -> `entity_block_map` in `core.rs`/`bump_crush.rs`, based on current/next cells, not peer path marker generation.` (evidence: source scan; Active in YR: Rust status only)
- `[DEFERRED] OQ-011 - Exact default constructor value for `TechnoType+0x678` before `Speed=` parsing.` (category: out-of-scope; reason: parser and reader prove the semantic needed for this gate; default constructor audit belongs to a TechnoType layout pass; next-step-if-pursued: decompile `TechnoTypeClass` constructor and compare stock unit defaults)
- `[DEFERRED] OQ-012 - Whether any modded negative non--1 `Speed=` value can preserve unusual prior storage.` (category: out-of-scope; reason: normal YR stock content uses nonnegative Speed values; next-step-if-pursued: test parser with negative custom INI or inspect default field initialization)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal peer-path `0x40000` marking only considers peers whose `TechnoType.Speed` is strictly lower than the searching unit's `TechnoType.Speed`, and skips same type entirely. | `0x0042AE44..0x0042AE66`; `0x0071464C..0x00714699`; `0x0070EFE0` | missing | `src/sim/pathfinding/core.rs`; movement path snapshot/type data passed from `src/sim/movement/*` | Add a search-scoped peer marker generator that can compare searching type speed against peer type speed before marking peer paths. | A faster Grizzly-style type near a slower Rhino-style peer gets peer-path 4x markers; equal-speed/same-type peers do not. Proposed test: `astar_bridge_peer_markers_require_strict_type_speed_priority`. | Do not compare house owner, veterancy, runtime current speed, or object id; this gate is type pointer identity plus `TechnoType.Speed`. |
| `Pathfinder+0x3C == 2` bypasses same-type, speed, and playfield checks for peer path marking. | `0x0042AE4E..0x0042AE52` | missing | `src/sim/pathfinding/core.rs`; path request urgency plumbing from movement blocked/repath code | Urgency route-around mode should mark eligible peer paths without enforcing the strict speed/type gate. | Same-type moving peer that is skipped at urgency 1 contributes peer markers at urgency 2. Proposed test: `astar_bridge_peer_markers_urgency2_bypasses_speed_priority_gate`. | Do not apply the speed gate unconditionally across all urgency modes. |
| Peer-path marking is a temporary per-search `0x40000` cost overlay; Rust currently has entity soft-block maps and static `PathGrid` but no equivalent overlay. | live caller path `0x004CBC31 -> 0x0042C900 -> 0x00429A90 -> 0x0042ACF0`; Rust scan `core.rs`, `bump_crush.rs`, `movement_path.rs` | missing | `src/sim/pathfinding/core.rs`; `src/sim/movement/bump_crush.rs`; `src/sim/movement/movement_path.rs` | Build/search with a transient cost-marker set derived from nearby moving peer path queues, max 24 entries, cleaned after the search. | Two consecutive A* calls over the same base grid leave `PathGrid` unchanged while the active search sees 4x costs on marked peer path cells. Proposed test: `astar_bridge_peer_marker_overlay_is_search_scoped_and_restored`. | Do not bake this into occupancy, hard blocks, terrain cost grids, bridge runtime state, or permanent `PathGrid` flags. |

## Negative Facts / Do Not Do

- Do not call the scoped field `ObjectClass+0x678`; the verified reads are `TypeClass+0x678` after `object->vtable+0x84`.
- Do not interpret the gate as owner, alliance, veterancy rank, current speed, or movement-progress ordering.
- Do not use `>=`; equality skips under normal urgency because the branch is `JLE skip`.
- Do not mark same-type peer paths under urgency `0/1`.
- Do not model `0x40000` peer markers as occupancy, hard passability, static `PathGrid`, or bridge-damage state.

## Stale Docs / Follow-up Docs

- `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` lines that say the exact semantic name of `+0x678` is not proven should be replaced with:
  "`object->vtable+0x84` returns the object's `TechnoTypeClass*`; the field read at `+0x678` is the parsed/scaled `Speed=` value. The normal peer-path marker gate skips same-type peers and requires `searcher.TechnoType.Speed > peer.TechnoType.Speed`; urgency `2` bypasses this gate."

No other stale wording was found inside this slice beyond prior reports that already warn not to model the marker as static terrain or occupancy.

## Remaining Uncertainty

- Constructor/default value for `TechnoType+0x678` before `Speed=` parsing was not re-opened; stock `Speed=` parser/read semantics are sufficient for this slice.
- This report did not audit every possible caller of the secondary xref at `0x00504DAE`; the normal `FootClass::Run_AStar` path and `AStar_main_loop` liveness are verified.

## Sources

- Ghidra decompile: `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`
- Ghidra assembly context: `0x0042AE01`, `0x0042AE44`, `0x0042AE4E`, `0x0042AE54`, `0x0042AE58`, `0x0042AE5E`, `0x0042AE64`, `0x0042AE66`
- Ghidra assembly context: marker write sequence `0x0042AF93..0x0042AFAE`
- Ghidra decompile/assembly: `AStar_main_loop @ 0x00429A90`, calls at `0x00429C1A`, `0x0042A42D`, `0x0042A44C`
- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`
- Ghidra decompile/assembly: `FootClass::Run_AStar @ 0x004CBBA0`, call at `0x004CBC31`
- Ghidra decompile/assembly: `TechnoClass::GetTypeSpeed @ 0x0070EFE0`, read at `0x0070EFEC`
- Ghidra decompile/assembly/xref: `TechnoTypeClass::ReadINI @ 0x00712170`; `Speed` string `0x0081D9CC`; parser/write `0x0071464C..0x00714699`
- Prior docs: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, `timing/movement-speed-turn-rate.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/movement/movement_path.rs`
