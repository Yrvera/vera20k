# InfantryClass::Can_Enter_Cell vtable +0x1AC -- Ghidra Research Report

**Address(es):** `0x0051BF90` (`InfantryClass::Can_Enter_Cell`, vtable `+0x1AC`), contrast-only `0x0073F0A0` (`UnitClass::Can_Enter_Cell`), shared bridge sub-check `0x004D9C60`
**Investigation Mode:** exhaustive-slice, downgraded to partial because no live Ghidra MCP tool was exposed in this subagent session
**Claimed Scope:** InfantryClass A* `Can_Enter_Cell` binding and the deltas from UnitClass needed for pathing/collision classifier parity: bridge/tube interaction, layer/occupancy selection, infantry occupant/building return-code differences, and Rust-facing split requirements.
**Non-Scope:** Full UnitClass re-documentation, full walk-locomotor movement tick, full low-bridge TubeClass producer audit, full gate/building mission runtime trace, and full concrete C decompilation listing.
**Confidence:** Medium-high overall. High where existing reports cite live Ghidra decompile/memory evidence; medium for subcell terminal return-code detail because available prior reports verify the fields and layer pattern but do not preserve the full `0x0051BF90` terminal branch.
**Active in YR:** Yes for the InfantryClass vtable binding, A* dispatch, bridge sub-check, low-bridge/tube paths, and normal infantry pathfinding. Conditional for stock-data-specific building flags and unusual `path_height > Level + 4` cases.

## 1. Overview

`InfantryClass` overrides the A* cell-entry slot at vtable `+0x1AC` with `0x0051BF90`, while inheriting the same `+0x1B0` bridge sub-check (`CheckBridgeTraversal @ 0x004D9C60`) used by `UnitClass`. The function is not a vehicle clone: it shares the bridge/tube prologue shape, then diverges in infantry-specific height shortcut, building/garrison handling, hostile-cell weapon-range gating, and infantry subcell availability.

The most implementation-relevant result is that Rust should not keep a single "UnitClass-style" classifier for all ground movers. Infantry needs its own classifier policy layered over the shared bridge/tube machinery: same `+0x1B0` bridge validator, same ground/bridge list and bitfield split, but different object/building returns and subcell acceptance.

## 2. Class Layout / Key Offsets

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `InfantryClass` vtable base `0x007EB058` | Primary vtable for infantry instances | constructor write `0x517ACC`, `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` | Yes |
| vtable `+0x1AC`, address `0x007EB204` | A* `Can_Enter_Cell` entry, bound to `0x0051BF90` | vtable memory read in `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` | Yes |
| vtable `+0x1B0`, address `0x007EB208` | Shared `CheckBridgeTraversal @ 0x004D9C60` | same vtable read; `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` | Yes |
| `CellClass+0xE4/+0xE8` | Ground / bridge object-list heads | Infantry same pattern at `0x51BFC4 / 0x51C2B0` per bridge offsets report | Yes |
| `CellClass+0x124/+0x128` | Ground / bridge occupancy bitfields; low byte includes infantry subcells and bit 5 non-infantry unit | `INFANTRY_SUBCELL_POSITIONING.md`; bridge offsets report | Yes |
| `CellClass+0x116` | Tube index for low bridge/tube logic | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | Yes |
| `CellClass+0xEC` | `LandType`; low bridge predicate requires `10` | `CellClass::IsLowBridgeCell @ 0x00484AB0` | Yes |
| `CellClass+0x11B` | signed terrain level | bridge traversal docs | Yes |
| `CellClass+0x140 & 0x100` | structural bridge cell flag | bridge traversal docs | Yes |
| `CellClass+0x140 & 0x200` | bridgehead/transition flag | bridge traversal docs | Yes |
| `BuildingType+0x16B7` | gate/garrison-style building flag in infantry branch | `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`, `0x0051C4EB..0x0051C549` | Conditional |
| `BuildingType+0x16BF/+0x16C0/+0x1701` | laser/firestorm/invisible-style building flags in infantry branch; prior field names vary | same building report, `0x0051C498..0x0051C4E6` | Conditional |

## 3. Core Logic

### 3.1 Binding and shared prologue

Verified binary fact: `InfantryClass` vtable `+0x1AC` is `0x0051BF90`, and `+0x1B0` is `0x004D9C60`. A* dispatch calls vtable `+0x1AC`; the function itself calls through `+0x1B0` for bridge traversal. Active in YR: Yes.

The infantry function starts with the same key prologue shape as `UnitClass`:

- pre-vtable bridge/list decision from `cell+0x140 & 0x100` and `abs(path_height - cell.Level) >= 2`;
- ground occupancy snapshot from `cell+0x124`;
- `CellClass::GetTubeAtCell`;
- direction `8` tube-entry special case;
- tube direction exclusion checks;
- `CheckBridgeTraversal @ 0x004D9C60`;
- post-vtable bridge occupancy re-snapshot from `cell+0x128` if `path_height == cell.Level + 4`;
- object-list scan from `cell+0xE4` or `cell+0xE8` according to the selected layer byte.

Evidence: `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` section 3 and `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` table row "InfantryClass::Can_Enter_Cell same pattern" (`0x51BFC4 / 0x51C2B0`). Active in YR: Yes.

### 3.2 Bridge interaction delta vs UnitClass

No separate infantry bridge sub-check was found. Infantry uses the same `CheckBridgeTraversal @ 0x004D9C60` as UnitClass through vtable `+0x1B0`. Therefore diff-0/diff-1/diff-4 legality, bridgehead requirement, signed `Level`, parent fallback, direction `-1` seed mode, and `path_height`/bridge-list side effects are shared.

Infantry-specific verified delta: after the shared bridge sub-check, `0x0051BF90` has an early acceptance:

```text
if path_height - cell.Level > 4: return 0
```

Evidence: `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` cites `0x51C055..0x51C062`. Active in YR: Conditional; normal flat/ordinary bridge pathing usually does not exceed deck height by more than four levels, but the branch is live in the function.

Implementation implication: keep shared bridge validation, but add the infantry-only high-path shortcut before ordinary occupancy/object hard blocking. Do not add this shortcut to UnitClass/vehicles.

### 3.3 Tube / low bridge interaction delta vs UnitClass

Infantry's direction-8 tube case is the same slot-family mechanism as UnitClass but with one documented small difference: the degenerate-tube rejection compares tube exit to tube entry (`tube+0x28 == tube+0x24`) instead of UnitClass's zero-endpoint check described in older Unit docs.

Evidence: `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` section 3.2. Active in YR: Yes for low bridge/tube-aware infantry pathing.

Low bridge identity is not overlay-only. `CellClass::IsLowBridgeCell @ 0x00484AB0` requires a valid `cell+0x116` tube index and `LandType == 10`; `GetTubeAtCell @ 0x00484F20` only bounds-checks the tube index. Infantry has its own tube movement routine (`InfantryClass::AI @ 0x0051BF00` calls `FUN_0051B350` when `+0x684` is non-negative), and low-bridge click/action handling is live (`InfantryClass::What_Action_OnCell @ 0x0051F900`). Active in YR: Yes.

Implementation implication: Rust infantry pathing must not use only bridge overlay/road state for low bridges. It needs the same tube-backed cell identity and direction-8 entry semantics used by the Unit path, with the infantry degenerate-tube rule preserved.

### 3.4 Subcell / occupant return-code findings

Verified field and helper facts:

- infantry subcells are represented by occupancy bits in `cell+0x124` ground and `cell+0x128` bridge;
- bits 0..4 are subcell bits, bit 5 (`0x20`) means non-infantry unit present, bit 6 (`0x40`) means building present;
- the functional placement subcells are 2, 3, and 4; indices 0 and 1 are not available for normal placement;
- `IsSubCellFree @ 0x00481130` returns false for subcell 0 or 1, then checks `1 << subcell` in `+0x124` or `+0x128`;
- `PlaceInfantryInCell @ 0x00481180` fails immediately on bit 5, treats bit 6 through a garrison-capability check, and selects an available functional subcell with direction-biased fallback.

Evidence: `INFANTRY_SUBCELL_POSITIONING.md` sections "Occupancy Byte Bit Field", "Occupancy Check", and "Placement Function". Active in YR: Yes.

Verified `0x0051BF90` layer fact: Infantry `Can_Enter_Cell` uses the same ground/bridge object-list and occupancy-bit split as UnitClass (`0x51BFC4 / 0x51C2B0`), so subcell availability must be checked on the selected occupancy-bit layer, not always ground. Active in YR: Yes.

Available reports do not preserve the complete terminal `0x0051BF90` subcell branch that maps every full/partially-full subcell pattern to a final return code. The safe implementation-facing conclusion is narrower: if an infantry mover has a free functional subcell on the occupancy-bit layer and the selected object list has no blocking object, the classifier can return code 0; if all functional subcells are occupied or bit 5/bit 6 creates a real blocker, it must continue to object/building/owner classification instead of treating the cell as ordinary clear. This is inference from verified subcell helpers plus the verified CEC layer pattern, not a newly decompiled terminal branch.

### 3.5 Object/building return-code deltas vs UnitClass

Infantry's building policy is not UnitClass's vehicle building policy.

Verified negative deltas:

- no `NumberImpassableRows` helper call (`0x00458A00`) in `0x0051BF90`;
- no `DynamicVectorClass::Contains @ 0x0065AD50` contact-vector row branch;
- no vehicle `HasBib` east-neighbor relaxation (`BuildingType+0x1570`) in infantry;
- no vehicle `UnitRepair` / `Bunker` row-helper branch (`+0x16A9/+0x16AB`) in infantry.

Evidence: `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`. Active in YR: No for infantry.

Verified positive infantry branch:

- `BuildingType+0x16B7` calls `BuildingClass::CanGarrison @ 0x004525F0`;
- if `CanGarrison` is false, allied building upgrades to at least code `3`;
- enemy building requires infantry action/fire capability or returns `7`; if able to act, upgrades to at least code `5`;
- generic enemy/hostile-cell handling later requires `TechnoClass::GetWeaponRange(this, -1) >= 1`; otherwise return `7`, else code `5`.

Evidence: `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`, assembly `0x0051C4EB..0x0051C549`, bridge hierarchy report section 3.6. Active in YR: Yes/Conditional by building and weapon data.

Available per-class report states Infantry returns `0/1/2/3/5/6/7`; it does not list code `4` for InfantryClass. Treat code `4` as not verified for infantry unless a future live decompile finds an infantry-specific friendly wall/overlay production site. UnitClass can produce code `4`; do not import that branch into infantry without evidence.

## 4. INI Keys

No new INI reader was identified in this subagent session. Relevant data gates from cited binary reports:

| Source | Key / family | Relevance | Active in YR |
|---|---|---|---|
| `rulesmd.ini` / `rules.ini` | low bridge overlay families `LOBRDG*`, `LOBRDGE*`, `LOBRDB*`, `LOBRDGB*` | Visible/art/damage identity; not sufficient for low-bridge pathing | Yes |
| final cell attributes | `LandType == 10` | Required by `IsLowBridgeCell`; live movement predicate | Yes |
| map `[Tubes]` | explicit tube records | Direction-8 / tube movement path; separate from auto low bridge tubes | Conditional by map |
| building type flags | `Gate=`, `LaserFence=`, `FirestormWall=`, garrison-like fields | Building-object branch choices in infantry CEC | Conditional |
| weapon/range data | infantry weapon range via `TechnoClass::GetWeaponRange(this,-1)` | hostile occupied-cell gate; unarmed/civilian/engineer cases can hard-block | Yes |

## 5. Integration Points

| Integration point | Verified shape | Evidence | Active in YR |
|---|---|---|---|
| A* neighbor expansion | dispatches mover vtable `+0x1AC`; Infantry binds to `0x0051BF90` | `AStar_main_loop @ 0x00429F54`, hierarchy report | Yes |
| Bridge sub-check | Infantry forwards CEC arg4/current-parent to `CheckBridgeTraversal @ 0x004D9C60` | `RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`, `0x0051C0E6` | Yes |
| Runtime walk calls | walk locomotion calls the same five-argument CEC shape | `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`; cited in building report | Yes |
| Low bridge tube movement | Infantry AI calls tube movement when `+0x684` is non-negative | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`, `0x0051BF00`, `FUN_0051B350` | Yes |
| Subcell assignment after movement | WalkLocomotion calls `PlaceInfantryInCell @ 0x00481180` through `0x0075C240` | `INFANTRY_SUBCELL_POSITIONING.md` | Yes |

## 6. Current Rust Implementation Status

Read-only scan only; no Rust files were modified.

| Surface | Current shape | Delta / risk |
|---|---|---|
| `src/sim/pathfinding/cell_entry.rs` | unified `CellEntryResult` codes and shared classifier; code names now mostly match verified Unit return table | Needs an explicit infantry policy split so vehicle-only building exceptions and code-4/friendly-wall assumptions are not applied to InfantryClass without proof. |
| `cell_entry.rs::check_terrain_with_layers` | infantry can clear if a subcell is available and selected object list has no blocker | Directionally correct, but the terminal `0x0051BF90` subcell return-code branch remains only partially verified in available docs. |
| `cell_entry.rs::decide_live_vehicle_building_entry` | already gates row/contact helper to `EntityCategory::Unit` | Matches the infantry negative facts from `0x0051BF90`; keep this boundary. |
| `src/sim/pathfinding/core.rs` | shared `check_bridge_traversal`, nullable parent, direction `-1`, split `CanEnterLayerContext` | Good shared bridge base; add infantry-only `path_height > Level + 4` behavior at the A*/CEC layer, not inside vehicle logic. |
| `core.rs::AStarOptions::is_infantry` | infantry goal height always ground level | Correct for low bridges and many normal cases, but does not by itself implement the Infantry CEC shortcut or object policy split. |
| `src/sim/movement/movement_occupancy.rs` | separates `DeferredCellCheck::Infantry` and `Vehicle`, but calls same `classify_occupied_cell_with_layers` | Needs to thread infantry-specific building/weapon-range and subcell branch semantics through the runtime classifier. |
| low bridge/tube model | no full `TubeClass`/per-cell tube identity equivalent in pathing | Foundational gap for both Unit and Infantry; for Infantry also affects `What_Action_OnCell` and `FUN_0051B350` equivalent. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Infantry vtable `+0x1AC` binding | verified | `0x007EB204 -> 0x0051BF90` in hierarchy report | none |
| Infantry vtable `+0x1B0` binding | verified | `0x007EB208 -> 0x004D9C60` | none |
| Shared bridge traversal semantics | verified by cited docs | `0x004D9C60`, bridge traversal reports | no need to redo here |
| Infantry high-path shortcut | verified by cited decompile | `0x51C055..0x51C062` | exact normal-game trigger frequency |
| Infantry tube direction-8 degenerate check | touched-not-exhausted | hierarchy report section 3.2 | live re-decompile should confirm exact comparison and return site |
| Infantry subcell bitfield/layer selection | verified for fields and layer pattern | `INFANTRY_SUBCELL_POSITIONING.md`; `0x51BFC4 / 0x51C2B0` | complete terminal return-code branch in `0x0051BF90` |
| Infantry building deltas vs Unit | verified | `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md` | exact gate mission runtime scenario |
| Infantry code `4` production | deferred | no available report lists it for Infantry | future live decompile if implementation wants wall parity |
| Rust classifier split | touched-not-exhausted | source scan | implementation design outside this report |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is `0x0051BF90` the InfantryClass A* CEC entry? -> Yes, vtable +0x1AC binds to it.` (evidence: `0x007EB204`, hierarchy report)
- `[RESOLVED] OQ-2 -- Does Infantry use the same bridge sub-check as Unit? -> Yes, vtable +0x1B0 binds to `0x004D9C60`.` (evidence: `0x007EB208`)
- `[RESOLVED] OQ-3 -- Is there an infantry-specific bridge/height shortcut? -> Yes, `path_height - cell.Level > 4` returns 0.` (evidence: `0x51C055..0x51C062`)
- `[RESOLVED] OQ-4 -- Are low bridge/tube paths live for infantry? -> Yes, Infantry AI and click-action paths use TubeClass and low-bridge predicates.` (evidence: `0x0051BF00`, `FUN_0051B350`, `0x0051F900`, low bridge report)
- `[RESOLVED] OQ-5 -- Are subcell bits layer-separated ground/bridge? -> Yes, `+0x124` and `+0x128` are ground/bridge occupancy bitfields and Infantry CEC follows the same layer pattern as Unit.` (evidence: subcell report; bridge offsets report)
- `[RESOLVED] OQ-6 -- Does Infantry use Unit's NumberImpassableRows/HasBib/contact branches? -> No direct equivalent in `0x0051BF90`.` (evidence: infantry building report)
- `[RESOLVED] OQ-7 -- Does Infantry have building/garrison-specific return codes? -> Yes; `CanGarrison` false can yield allied code 3, enemy code 5, or hard 7 if unable to act.` (evidence: `0x0051C4EB..0x0051C549`)
- `[DEFERRED] OQ-8 -- What is the exact terminal subcell-full return-code ladder inside `0x0051BF90`?` (category: `needs-runtime-debugger`; reason: no live Ghidra MCP was exposed and prior reports do not preserve this exact branch; next-step-if-pursued: live decompile `0x0051BF90` around post-object-loop occupancy-bit resolution)
- `[DEFERRED] OQ-9 -- Does Infantry ever produce code 4 from wall/overlay handling?` (category: `needs-runtime-debugger`; reason: available hierarchy report lists Infantry codes `0/1/2/3/5/6/7` only; next-step-if-pursued: live-decompile overlay branch in `0x0051BF90`)
- `[DEFERRED] OQ-10 -- Exact normal-YR trigger frequency for `path_height > Level + 4`.` (category: `requires-different-system-context`; reason: branch is binary-verified but scenario frequency requires runtime trace; next-step-if-pursued: trace bridge collapse/elevation anomalies for infantry A*)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Infantry uses same `CheckBridgeTraversal` as Unit but adds a CEC-local `path_height - cell.Level > 4 -> code 0` shortcut. | `0x007EB208 -> 0x004D9C60`; `0x51C055..0x51C062` | missing | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/cell_entry.rs` | Add infantry-only acceptance after shared bridge traversal, before generic hard-block classification. | Infantry A* over an abnormal bridge/elevation fixture accepts a cell where path height is more than four levels above ground while vehicle A* rejects or continues normal checks. Proposed test: `infantry_can_enter_cell_accepts_path_height_above_deck_but_vehicle_does_not`. | Do not put this shortcut into shared Unit/vehicle bridge traversal. |
| Infantry CEC must keep subcell availability on the post-bridge occupancy-bit layer separate from selected object-list layer. | `0x51BFC4 / 0x51C2B0`; `cell+0x124/+0x128` reports | partial | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/occupancy.rs` | For infantry, free functional subcell on `occupancy_bits_layer` can clear only if selected object-list layer has no blocker; full subcells or bit-5/bit-6 blockers flow into classifier. | Same `(rx,ry)` has bridge-layer infantry subcells occupied and ground subcells free; an infantry on bridge cannot treat the cell as ground-clear. Proposed test: `infantry_subcell_availability_uses_bridge_occupancy_bits_layer`. | Do not collapse object-list layer and subcell-bit layer into one movement layer at bridgeheads. |
| Infantry does not use UnitClass's vehicle-only building row/contact/HasBib exceptions. | no `0x00458A00`, no `0x0065AD50`, no `+0x1570` read in `0x0051BF90` | mostly matched by existing vehicle-only helper; keep boundary | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs` | Keep `NumberImpassableRows`, radio/contact, UnitRepair/Bunker, and HasBib relaxations gated to Unit/vehicle movement. | Infantry pathing into a refinery bib/repair-depot row remains classified by infantry building policy while vehicle-specific tests may skip. Proposed test: `infantry_can_enter_cell_does_not_use_vehicle_building_row_relaxation`. | Do not move `decide_live_vehicle_building_entry` into a shared ground-mover rule. |
| Infantry building branch uses `CanGarrison` / ownership / action capability, with allied false-garrison code 3 and enemy armed code 5 or unarmed code 7. | `0x0051C4EB..0x0051C549`; `0x004525F0`; GetWeaponRange gate | missing/unchecked | `src/sim/pathfinding/cell_entry.rs`, building rules/state surfaces, combat weapon surfaces | Add infantry-specific building occupant classifier separate from vehicle building policy. | Armed infantry against enemy garrison-style building returns code 5; unarmed/civilian/engineer returns 7; allied blocked garrison returns code 3. Proposed test: `infantry_building_garrison_branch_returns_3_5_or_7_by_owner_and_weapon`. | Do not reuse UnitClass's UnitRepair/Bunker/HasBib branch for infantry garrison/gate behavior. |
| Infantry low bridge/tube entry is tube-backed and has an Infantry CEC degenerate-tube rule. | `0x00484AB0`, `0x00484F20`, `0x0051BF00`, `FUN_0051B350`, hierarchy section 3.2 | missing foundational tube model | `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/tube_movement.rs`, future infantry tube surface | Model valid tube index + `LandType==10`; direction-8 entry must use TubeClass exit/entry semantics, not overlay-road passability. | Infantry crossing a low bridge cell with `LandType=10` and valid tube succeeds; same overlay without valid tube index does not get low-bridge tube behavior. Proposed test: `infantry_low_bridge_can_enter_cell_requires_tube_index_and_landtype_10`. | Do not implement low bridges as ordinary road overlays for infantry. |

### Negative Facts / Do Not Do

- Do not treat vtable `+0x1B0` as the A* entry. Evidence: Infantry `+0x1AC = 0x0051BF90`, `+0x1B0 = 0x004D9C60`. Active in YR: Yes.
- Do not add Infantry's `path_height > Level + 4` shortcut to UnitClass or shared `CheckBridgeTraversal`. Evidence: shortcut cited only in `0x0051BF90`; UnitClass baseline lacks it. Active in YR: Conditional.
- Do not apply vehicle `NumberImpassableRows`, RadioClass contact-vector, UnitRepair/Bunker, or HasBib relaxation to infantry. Evidence: absent from `0x0051BF90`; present in UnitClass reports. Active in YR: No for infantry.
- Do not assume Infantry produces code 4 until the `0x0051BF90` overlay branch is live-decompiled. Evidence: existing hierarchy report lists Infantry return set as `0/1/2/3/5/6/7`, not `4`. Active in YR: Unverified.
- Do not treat low bridge overlay `Land=Road` as the movement predicate. Evidence: `IsLowBridgeCell @ 0x00484AB0` requires valid `+0x116` tube index and `LandType == 10`. Active in YR: Yes.

### Remaining Uncertainty

- Exact `0x0051BF90` terminal subcell-full return-code ladder remains unresolved in this session because no live Ghidra MCP tool was available and prior reports do not preserve that branch.
- Exact Infantry overlay/wall code `4` production is unresolved; current evidence says do not assume it.
- Exact stock gameplay frequency for the high-path shortcut requires a runtime trace.
- Exact producer that writes infantry active tube index `+0x684` for low bridges remains outside this target; the consumer path is verified.

### Stale Docs / Follow-up Docs

- `docs/research/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`: replace "How does `InfantryClass::Can_Enter_Cell` differ from `UnitClass::Can_Enter_Cell`? (Infantry may have different occupancy semantics via sub-cells.)" with "Infantry binds vtable `+0x1AC` to `0x0051BF90`, shares `CheckBridgeTraversal @ 0x004D9C60`, adds `path_height - cell.Level > 4 -> code 0`, uses layer-separated subcell bits, and omits UnitClass vehicle-only building row/contact/HasBib exceptions; exact terminal subcell-full ladder still needs live-decompile confirmation."
- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`: any wording that implies code 4 or UnitClass vehicle building exceptions are shared by Infantry should be replaced with "InfantryClass has a separate `0x0051BF90` object/building policy; do not infer infantry return-code production from UnitClass without Infantry decompile evidence."
- `src/sim/pathfinding/cell_entry.rs` comments already warn that vehicle row/contact branches are not infantry rules; keep that boundary when adding any future classifier split.

## Sources

- `docs/research/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`
- `docs/research/INFANTRY_SUBCELL_POSITIONING.md`
- `docs/research/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `docs/research/RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`
- `docs/research/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- `docs/research/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- Rust read-only scan: `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_occupancy.rs`
