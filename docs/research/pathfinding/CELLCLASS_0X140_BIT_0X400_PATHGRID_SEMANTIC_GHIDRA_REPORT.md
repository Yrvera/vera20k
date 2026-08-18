# CellClass+0x140 Bit 0x400 Pathgrid Semantic - Ghidra Research Report

**Address(es):** `0x0047C620`, `0x0047E040`, `0x0047E470`, `0x00565C10`, `0x00574200`, `0x00574EAD`, `0x006E2390`, `0x00429830`, `0x0073F0A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** semantic meaning, verified readers/writers, YR activity, and pathgrid/placement/CellRect/A* relevance of `CellClass+0x140` bit `0x400`, starting from deferred OQ-6 in `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`.  
**Non-Scope:** complete taxonomy of all `CellClass+0x140` flags, bridge damage formulas beyond this bit, exact visual bridge overlay frame selection, and full runtime zone invalidation parity.  
**Confidence:** High for the scoped readers/writers and negative A*/placement boundary; Medium for the human-readable semantic name because the bit is part of a multi-bit bridge-state encoding rather than a standalone named field.  
**Active in YR:** Yes for bridge overlay mark, bridge damage/repair, building placement, and bridge-destruction fallback. Conditional for the Ion impact-Z helper: the checked reader is reached through trigger action case `0x2A`, not ordinary/sidebar superweapon firing.

## 1. Overview

`CellClass+0x140 bit 0x400` is a bridge-state flag in the same field family as bridge structural bit `0x100`. The most precise scoped name is `BridgeInactiveOrEndpointSearch` / "non-structural bridge endpoint/fallback marker": it is set by `CellClass::SetBridgeDirection_*` when the bridge state argument is zero, and cleared on normal live bridge marking where `state=1`.

The bit is not the A* bridge-approach pathgrid multiplier. A* edge cost uses `CellClass+0x140 & 0x40000`, while this target bit is `0x400`. For Rust, the main direct implication is placement/buildability and bridge impact/damage selection, not ordinary unit A* passability.

## 2. Class Layout / Key Offsets

| Offset / bit | Verified meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `CellClass+0x140` | Cell flags bitfield containing bridge, placement, and pathing-adjacent state | Yes | All scoped functions read/write this field |
| `+0x140 & 0x100` | Structural/live bridge surface bit used by placement, A*, bridge-Z and damage logic | Yes | `0x0047C620`, `0x00429830`, `0x0073F0A0`, `0x006E2390` |
| `+0x140 & 0x400` | Bridge inactive/fallback endpoint marker; set when `SetBridgeDirection_*` receives state 0; blocks building placement terrain fallback and participates in bridge damage/impact-Z fallback | Yes | `0x0047E040`, `0x0047E470`, `0x0047C620`, `0x00574200`, `0x00574EAD`, `0x006E2390` |
| `+0x140 & 0x800` | Bridge orientation/fallback direction selector read with `0x400` in bridge destruction fallback | Yes | `0x00574200`, `0x00574EAD`; written by `SetBridgeDirection_*` when `direction==0` |
| `+0x140 & 0x40000` | A* bridge approach cost multiplier flag, separate from target bit `0x400` | Yes | `AStar_compute_edge_cost @ 0x00429830` |

## 3. Core Logic

### 3.1 Writer: `CellClass::SetBridgeDirection_NESW @ 0x0047E040`

Active in YR: Yes. Called from bridge overlay marking, map resize refresh, and high bridge damage/repair paths.

The function computes:

- `state = param_3 & 1`
- `bit_0x400_value = (param_3 == 0) << 10`
- `bit_0x100_value = state << 8`
- `bit_0x200_value = state << 9`
- `bit_0x1000_value = state << 12`
- `bit_0x10000_value = state << 16`
- `bit_0x800_value = (direction == 0) << 11`

For the anchor cell, it writes:

`Flags = Flags & 0xFFFEE07F | bit_0x100 | bit_0x200 | bit_0x1000 | bit_0x10000 | bit_0x400 | (state << 7) | bit_0x800`.

For forward/opposite neighbor cells, similar masks write/clear `0x100`, `0x200`, `0x400`, `0x800`, `0x1000`, and `0x10000`, while preserving or clearing other bridge bits according to cell role.

When `param_3 == 0`, the function also sets `field_0x11e = 0` and calls `CellClass__BlowUpBridge` for affected cells. When `param_3 != 0`, it clears `0x400` and sets live bridge bits. Evidence: decompile `0x0047E040`.

### 3.2 Writer: `CellClass::SetBridgeDirection_NWSE @ 0x0047E470`

Active in YR: Yes. Called from high bridge overlay mark, low bridge damage/repair paths, and map resize refresh.

`0x0047E470` is behavior-identical to `0x0047E040` for this bit. It uses the same `param_3 == 0` expression to set `0x400`, and the same live-state `param_3 & 1` bits to clear `0x400` while setting structural bridge bits. Evidence: decompile `0x0047E470`.

### 3.3 Map-load activity: `OverlayClass::Mark @ 0x005FC570`

Active in YR: Yes. This is used when bridge overlays are marked during map loading and in the map editor.

Bridge overlay IDs call the bridge-direction writers with `state=1`:

- `0x18` -> `SetBridgeDirection_NESW(cell, dir=0, state=1)`
- `0x19` -> `SetBridgeDirection_NESW(cell, dir=6, state=1)`
- `0xED` -> `SetBridgeDirection_NWSE(cell, dir=0, state=1)`
- `0xEE` -> `SetBridgeDirection_NWSE(cell, dir=6, state=1)`

Because `state=1`, map-load bridge marking clears `0x400` on normal live bridge cells. Evidence: `0x005FC570` decompile and xrefs at `0x005FC5FE`, `0x005FC60A`, `0x005FC62C`.

### 3.4 Preservation/refresh writer: `MapClass::Resize @ 0x00565C10`

Active in YR: Conditional. This is a map resize/save-load/editor style path, not ordinary per-tick skirmish movement.

`MapClass::Resize` copies `CellClass+0x140` bit-by-bit into a temporary buffer and restores it bit-by-bit after reallocating cells. The copy/restore includes `0x400` specifically at assembly `0x0056615C` and `0x0056696E`. Later, a refresh loop calls `SetBridgeDirection_*` with `state=1` for cells that retained bit `0x80` but lost their bridge anchor pointer. Evidence: decompile `0x00565C10`; assembly contexts `0x0056615C`, `0x0056696E`.

### 3.5 Reader: building placement `0x0047C620`

Active in YR: Yes. This is active for ready-building preview, placement execution/type validation, and BuildingClass wrapper paths.

`Cell_passability_building_placement` rejects cells with `Flags & 0x400` in two places:

- Laser-fence/tiberium-overlay special branch: `Flags & 0x100` rejects, then `Flags & 0x400` rejects, then `SlopeIndex == 0` accepts.
- Ordinary terrain fallback with `speedType == -1`: requires no `0x100`, no `0x400`, and `SlopeIndex == 0` before using `Buildable=` or the naval iso-tile range branch.

This means `0x400` is directly build-blocking for the `0x0047C620` terrain fallback. Evidence: decompile `0x0047C620`; existing placement report OQ-6.

### 3.6 Reader: bridge destruction fallback `0x00574200` / `0x00574EAD`

Active in YR: Yes. These are runtime-called high/low bridge destruction fallback paths, including BridgeRepairHut death and BombClass detonation paths.

Both low and high bridge versions:

- Search for a nearby cell whose flags satisfy `flags & 0x500` (`0x100 | 0x400`).
- Return if neither `0x100` nor `0x400` is found.
- If `0x400` is present without `0x100`, enter a pure-`0x400` fallback branch that walks perpendicular cells up to 4 steps while cells continue to have `0x400`, then offsets two more cells to recover an anchor for bridge damage application.
- Use `flags & 0x800` to select the fallback direction.

Evidence: decompile `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574200`; decompile `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574EAD`; direct `TEST [ECX+0x140],0x400` assembly at `0x005742E4` and `0x00574F00`.

### 3.7 Reader: Ion Cannon impact-Z `0x006E2390`

Active in YR: Conditional. The helper is live through `TriggerAction::Execute` case `0x2A`; it should not be described as ordinary/sidebar Ion Cannon superweapon activity.

`FUN_006E2390` computes target cell ground Z, then adds bridge height if either `Flags & 0x100` or `Flags & 0x400` is set:

`impact_z = ground_z + (flags & (0x100 | 0x400) ? DAT_00B0E6D4 : 0)`.

The resulting impact Z is then fed through anim/damage setup. Evidence: decompile `0x006E2390`; prior `SUPERWEAPON_IMPACT_Z_BRIDGE_AOE_GHIDRA_REPORT.md`.

### 3.8 Negative A* reader check

Active in YR: Yes for A*, but No for this bit as an A* cost flag in the checked functions.

`AStar_compute_edge_cost @ 0x00429830` reads:

- `dest_cell+0x140 & 0x40000` for the 4x bridge approach cost multiplier.
- `dest_cell+0x140 & 0x800` and side cells' `0x100` for diagonal bridge cost shaping.

It does not use `0x400` in the decompiled body. `UnitClass::Can_Enter_Cell @ 0x0073F0A0` reads `0x100` for bridge layer/list selection and does not show a `0x400` gate in the inspected decompile. Evidence: decompile `0x00429830`, `0x0073F0A0`.

## 4. INI Keys

No INI key directly names or sets `CellClass+0x140 bit 0x400`.

| Input | Relationship | Active in YR |
|---|---|---|
| Bridge overlay IDs `0x18`, `0x19`, `0xED`, `0xEE` | Map overlays trigger `OverlayClass::Mark`, which calls `SetBridgeDirection_*` with `state=1` and clears `0x400` on live bridge cells | Yes |
| Building placement `Buildable=` and speed table | Only reached after `0x400` has not rejected the cell in `0x0047C620` | Yes |

## 5. Integration Points

| Integration point | Bit role | Active in YR | Evidence |
|---|---|---|---|
| Ready building placement / CellRect-like foundation validators | Per-cell rejection through `0x0047C620`; type validators walk foundation cells and call it | Yes | `0x0047C620`, prior `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md` |
| PathGrid / A* edge cost | Not this bit; A* uses `0x40000` for bridge approach cost | Yes for A*, No for `0x400` | `0x00429830` |
| Unit movement `Can_Enter_Cell` | Checked body uses `0x100` bridge state, not `0x400` | Yes for movement, No for this bit in checked body | `0x0073F0A0` |
| Bridge destruction fallback | Fallback anchor-search marker, especially `0x400` without `0x100` | Yes | `0x00574200`, `0x00574EAD` |
| Trigger-action impact bridge Z | Case `0x2A` Ion impact-Z helper treats `0x400` like `0x100` for bridge-height impact Z | Conditional | `0x006E2390`; caller through `TriggerAction::Execute` |
| Map resize/cell copy | Bit is preserved by generic bit-copy and may be cleared by refresh re-mark | Conditional | `0x00565C10` |

## 6. Current Rust Implementation Status

Rust comparison is read-only and implementation-facing only:

| Rust area | Current status vs scoped finding | Evidence |
|---|---|---|
| `src/map/bridge_facts.rs` / `src/sim/world/bridge_orchestrator.rs` | Current Rust now has explicit `BRIDGE_FLAG_DESTROYED_OR_RAMP = 0x400` and bridge fallback users; older notes that this bit is wholly absent are stale | Rust scan after 2026-05-22 audit |
| `src/sim/pathfinding/core.rs::PathGrid` | Uses `bridge_walkable`, `bridge_structural`, and `transition`; no raw `0x400` semantic is modeled as an A* cost flag | Rust scan and Codegraph `PathGrid` search |
| `src/sim/production/production_placement.rs` | Placement rejects bridge deck/bridge walkable and canonical ramps, but does not explicitly distinguish binary `0x400` bridge-inactive fallback marker | Rust scan |
| `src/app_sim_tick.rs::rebuild_dynamic_path_grid` | Rebuilds `PathGrid` from terrain plus `BridgeRuntimeState`; this is the right surface for bridge-state changes, but `0x400` should not be added as an A* `0x40000` equivalent | Rust scan |
| `src/sim/pathfinding/zone_*` | Zone rebuild consumes `PathGrid`; no direct `0x400` parity should be inferred without the sibling zone investigations | Rust scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior OQ-6 in placement report | verified/extended | `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`; `0x0047C620` | none for placement effect |
| `SetBridgeDirection_NESW @ 0x0047E040` | verified | decompile | none for bit `0x400` writes |
| `SetBridgeDirection_NWSE @ 0x0047E470` | verified | decompile | none for bit `0x400` writes |
| `OverlayClass::Mark @ 0x005FC570` bridge calls | verified | decompile, xrefs | none for bridge overlay IDs in scope |
| `MapClass::Resize @ 0x00565C10` bit-copy | verified | decompile; assembly `0x0056615C`, `0x0056696E` | ordinary skirmish reachability of resize path not expanded |
| `Cell_passability_building_placement @ 0x0047C620` | verified | decompile | none for bit `0x400` effect |
| Bridge destruction fallback high/low | verified | decompile `0x00574200`, `0x00574EAD`; asm `0x005742E4`, `0x00574F00` | exact bridge-damage endpoint visuals out of scope |
| Ion Cannon impact-Z reader | verified | decompile `0x006E2390`; prior SW report | other superweapon readers not exhaustively expanded |
| A* edge cost relationship | verified negative | decompile `0x00429830` | none for target bit; other A* flags out of scope |
| UnitClass movement relationship | touched-not-exhausted | decompile `0x0073F0A0` | full unit movement body is large; no `0x400` branch observed in scoped pass |
| CellRect validators | verified through prior validator reports | `0x00716150`/`0x0045EE70` reports | no separate direct rectangle-wide `0x400` reader found in this slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What exact slice is claimed? -> Only CellClass+0x140 & 0x400 semantic, readers/writers, and pathgrid/placement/A* relevance.` (evidence: user scope; Active in YR: Yes)
- `[RESOLVED] OQ-2 - Does 0x0047C620 read bit 0x400? -> Yes; it rejects laser-fence/tiberium overlay fallback and ordinary terrain fallback cells with this bit.` (evidence: `0x0047C620`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - Who writes bit 0x400? -> SetBridgeDirection_NESW/NWSE set it when param_3==0 and clear it when state=1; map resize preserves it by bit-copy.` (evidence: `0x0047E040`, `0x0047E470`, `0x00565C10`; Active in YR: Yes / Conditional for resize)
- `[RESOLVED] OQ-4 - Is map-load bridge marking a writer? -> Yes, but it calls SetBridgeDirection_* with state=1, which clears 0x400 on normal live bridge cells.` (evidence: `0x005FC570`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - Is the bit a pathgrid/A* cost flag? -> No in the checked A* cost function; 0x00429830 uses 0x40000, not 0x400.` (evidence: `0x00429830`; Active in YR: Yes for A*, No for this bit)
- `[RESOLVED] OQ-6 - Does UnitClass A* movement directly read this bit? -> No direct 0x400 gate was observed in the inspected UnitClass::Can_Enter_Cell; the bridge layer gate uses 0x100.` (evidence: `0x0073F0A0`; Active in YR: Yes for movement, No for this bit in checked body)
- `[RESOLVED] OQ-7 - Does this bit feed building placement or CellRect validators? -> Yes indirectly for foundation/CellRect validators that call 0x0047C620 per cell; no separate rectangle-wide direct read found.` (evidence: `0x0047C620`; prior validator reports; Active in YR: Yes)
- `[RESOLVED] OQ-8 - Does the bit have non-placement readers? -> Yes, bridge destruction fallback and Ion Cannon impact-Z read it.` (evidence: `0x00574200`, `0x00574EAD`, `0x006E2390`; Active in YR: Yes)
- `[RESOLVED] OQ-9 - Are there relevant INI keys? -> No direct key; bridge overlay IDs and placement tables reach the bit only through code paths.` (evidence: `OverlayClass::Mark`, `0x0047C620`; Active in YR: Yes)
- `[RESOLVED] OQ-10 - Null/invalid cell behavior? -> Out-of-bounds fallback uses sentinel DAT_00ABDC50; bridge destruction fallback returns when neither 0x100 nor 0x400 is found.` (evidence: `0x00574200`, `0x00574EAD`; Active in YR: Yes)
- `[RESOLVED] OQ-11 - Zero value edge case? -> If bit 0x400 is zero, placement can proceed to other checks; Ion Cannon does not add bridge height unless 0x100 is also set.` (evidence: `0x0047C620`, `0x006E2390`; Active in YR: Yes)
- `[RESOLVED] OQ-12 - Max/other flags edge case? -> 0x400 is tested independently from 0x100 in placement and Ion Cannon; bridge destruction often tests combined mask 0x500.` (evidence: `0x0047C620`, `0x00574200`, `0x006E2390`; Active in YR: Yes)
- `[DEFERRED] OQ-13 - Exhaustive whole-binary xref census for every loaded-register & 0x400 form.` (category: bounded-cost-too-high; reason: scoped readers/writers were verified from target, bridge writer, direct byte-pattern hits, and relevant pathing functions; next-step-if-pursued: run a dedicated cell-flag global xref audit)
- `[DEFERRED] OQ-14 - Exact human-readable Westwood/YRPP name for this flag.` (category: requires-different-system-context; reason: no reliable ground-truth symbol name was available; next-step-if-pursued: compare save/map cell flag serialization and bridge state machine labels)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x400` blocks `0x0047C620` terrain fallback placement just like `0x100`, before `Buildable=`/naval checks | `0x0047C620` | partial: Rust has bridge/ramp placement rejections but no explicit bridge-inactive marker parity | `src/sim/production/production_placement.rs::cell_placeable`; bridge state data feeding placement | Placement must reject cells represented by binary `0x400` even if they are not currently bridge-walkable | On a destroyed/inactive bridge endpoint marker cell with otherwise buildable land, ready-building preview/commit rejects placement; proposed test: `placement_rejects_bridge_inactive_marker_0x400_cells` | Do not rely only on current `bridge_walkable`; `0x400` can be meaningful when `0x100` is absent |
| `0x400` is not A* `0x40000`; A* cost uses `0x40000` for bridge approach, while `0x400` did not appear as a cost/passability gate in checked A* functions | `0x00429830`, `0x0073F0A0` | none observed if Rust keeps separate bridge transition/pathgrid concepts; unchecked for every bridge state mapping | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/terrain_cost.rs`, zone rebuild surfaces | Keep target bit out of A* cost multipliers unless a separate movement reader is proven | A fixture with a cell tagged only as the binary `0x400` marker does not receive the bridge-approach cost multiplier and is not treated as `0x40000`; proposed test: `astar_does_not_treat_cell_flag_0x400_as_bridge_approach_cost` | Do not conflate `0x400` with `0x40000`; the names look similar but drive different systems |
| `SetBridgeDirection_*` sets `0x400` only when bridge state argument is zero and clears it on live bridge mark (`state=1`) | `0x0047E040`, `0x0047E470`, `0x005FC570` | unchecked: Rust bridge runtime state may not preserve an explicit inactive-marker bit | `src/sim/bridge_state.rs`; `src/map/bridge_facts.rs`; `src/app_sim_tick.rs::rebuild_dynamic_path_grid` | Bridge state transitions should expose a placement/render/damage marker equivalent without feeding it into ordinary A* as structural walkability | Destroying/repairing a bridge toggles the inactive marker separately from structural `bridge_walkable`, and placement reacts after rebuild; proposed test: `bridge_state_toggles_inactive_marker_separately_from_walkable` | Do not erase the marker during dynamic rebuild just because the deck is not walkable |

### Negative Facts / Do Not Do

- Do not implement `0x400` as the A* bridge approach cost bit. Evidence: A* cost reads `0x40000` at `0x00429830`; Active in YR: Yes.
- Do not treat normal live bridge map-load cells as `0x400` cells. Evidence: `OverlayClass::Mark @ 0x005FC570` calls `SetBridgeDirection_*` with `state=1`, which clears `0x400`; Active in YR: Yes.
- Do not let `PathGrid::bridge_walkable` alone decide building placement on bridge-damaged/inactive marker cells. Evidence: `0x0047C620` checks `0x400` independently of `0x100`; Active in YR: Yes.
- Do not assume `0x400` is TS-dead just because it is bridge-related. Evidence: runtime bridge destruction fallback and Ion Cannon impact-Z read it in active YR paths; Active in YR: Yes.
- Do not collapse `0x400` and `0x100` into one Rust enum state without preserving the "0x400 without 0x100" branch. Evidence: destruction fallback has a dedicated pure-`0x400` branch; Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md` OQ-6 replacement wording:
  "`CellClass+0x140 bit 0x400` is a bridge-state inactive/fallback endpoint marker, written by `CellClass::SetBridgeDirection_NESW/NWSE` when their state argument is zero and cleared by normal bridge overlay marking with state `1`. It blocks `0x0047C620` terrain fallback placement and is read by bridge destruction fallback and Ion Cannon impact-Z logic. It is not the A* bridge approach cost flag; A* uses `0x40000`."

## 10. Remaining Uncertainty

- The exact original symbolic name for `0x400` remains unknown; the report uses a behavior-derived name.
- A whole-binary scalar xref census was not completed; this report verifies the target placement reader, bridge-state writers, direct bridge destruction tests, Ion Cannon reader, and checked A* movement negative boundary.
- Whether every Rust bridge runtime transition currently preserves enough information to model pure-`0x400` marker cells remains an implementation audit item.

## Sources

- Ghidra decompile: `Cell_passability_building_placement @ 0x0047C620`
- Ghidra decompile: `CellClass::SetBridgeDirection_NESW @ 0x0047E040`
- Ghidra decompile: `CellClass::SetBridgeDirection_NWSE @ 0x0047E470`
- Ghidra xrefs: SetBridgeDirection callers from `OverlayClass::Mark`, `MapClass::Resize`, high/low bridge update and damage state functions
- Ghidra decompile: `OverlayClass::Mark @ 0x005FC570`
- Ghidra decompile: `MapClass::Resize @ 0x00565C10`
- Ghidra assembly context: `0x0056615C`, `0x0056696E`, `0x005742E4`, `0x00574F00`
- Ghidra decompile: `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574200`
- Ghidra decompile: `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574EAD`
- Ghidra decompile: `FUN_006E2390`
- Ghidra decompile: `AStar_compute_edge_cost @ 0x00429830`
- Ghidra decompile: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`
- Existing docs: `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`, `SUPERWEAPON_IMPACT_Z_BRIDGE_AOE_GHIDRA_REPORT.md`, `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`, `BUILDING_CAN_ENTER_CELL_PLACEMENT_PASSABILITY_BODY_GHIDRA_REPORT.md`, `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/production/production_placement.rs`, `src/app_sim_tick.rs`
