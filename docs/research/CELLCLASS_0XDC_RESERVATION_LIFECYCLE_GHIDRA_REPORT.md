# CellClass+0xDC Reservation Lifecycle - Ghidra Research Report

## Scope

Target: `CELLCLASS_0XDC_RESERVATION_LIFECYCLE`.

Investigation mode: `/re-investigate` exhaustive slice, scoped to `CellClass+0xDC` reservation bitmask writers, clearers, and live standard-YR readers around AI/base placement, `HouseClass+0x30`, `AIBaseSpacing`, and GapGen-like naming.

Out of scope: dynamic entity occupancy, full CellRect validator taxonomy, broad CellClass field naming, and non-placement uses except where needed to disambiguate `+0xDC` from GapGen/shroud state.

Primary seed: `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` OQ-16.

## Executive Summary

`CellClass+0xDC` is a per-house bitmask read by live AI/base placement helpers in standard YR. The bit index is `HouseClass+0x30`; readers compute `1 << (house_index & 0x1F)` and test cells near or around candidate building placement areas.

The field is not the Gap Generator visibility bitmask in the checked code. Gap/shroud visual code writes `CellClass+0x78`, and sensor-array counts use `CellClass+0x7C` per-house short counters. Prior docs that name `+0xDC` as `GapGenBitmask` should be treated as stale for implementation.

Verified lifecycle in this slice:

- Clear/init: `CellClass__Constructor` initializes `param_1[0x37]` (`+0xDC`) to zero.
- Preserve/copy: `MapClass__Resize` copies old cell `+0xDC` through its temporary cell backup and restores it after resize.
- Readers: `CellRect__CheckOccupancy`, `FUN_005060B0`, `FUN_0050B760`, and `FUN_00486D90` read `+0xDC` masks.
- Set/clear writers in live AI/base placement and GapGen paths were not found in this pass. No `BuildingClass__Unlimbo` `+0xDC` writer was present in the checked binary range; that prior claim conflicts with direct evidence.

Overall confidence: Medium for reader contract and GapGen negative fact; Low/Medium for complete writer taxonomy because no setter was identified in this bounded pass.

## Material Findings

### F1 - Cell initialization clears `+0xDC`

Active in YR: Yes.

Evidence:

- `CellClass__Constructor @ 0x0047BC50` decompiles with `param_1[0x37] = 0`; `0x37 * 4 == 0xDC`.
- Raw disassembly candidate scan also found `0x0047BC76: mov dword ptr [esi + 0xdc], ebx` in the constructor path.

Behavior:

- Newly constructed map cells begin with no reservation bits set.

### F2 - `CellRect__CheckOccupancy` conditionally reads `+0xDC`

Active in YR: Conditional.

Evidence:

- Prior verified report `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` establishes `CellRect__CheckOccupancy @ 0x00586780`.
- Material instruction from parent is preserved: `CheckOccupancy(rect, -1)` skips `+0xDC`; otherwise it checks `1 << (arg & 0x1F)`.
- Raw instruction at `0x00586823` reads `mov ecx, dword ptr [esi+0xdc]`.

Behavior:

- `arg == -1` means do not consult reservation bits.
- Any other `arg` is interpreted as the bit index for `CellClass+0xDC`.

### F3 - AI/site helper `FUN_005060B0` reads same-house reservation bits

Active in YR: Conditional.

Evidence:

- `FUN_005060B0 @ 0x005060B0` computes `1 << (HouseClass+0x30 & 0x1F)`.
- It probes neighboring cells for `(cell+0xDC & house_mask) != 0`; raw instruction candidate `0x005065A8: test dword ptr [eax + 0xdc], ebx`.
- It later calls `CellRect__CheckOccupancy(expanded_rect, HouseClass+0x30)` at the verified callsite `0x005069DB`.
- Parent context already settled that `FUN_005060B0` is the AI/site helper caller using `HouseClass+0x30`.

Behavior:

- This helper does not treat `+0xDC` as dynamic occupancy.
- It uses same-house reservation proximity and reservation-free expanded rectangles while evaluating AI/base placement candidates.

### F4 - `FUN_0050B760` is a live expanded-area reservation predicate

Active in YR: Conditional.

Evidence:

- `FUN_0050B760 @ 0x0050B760` returns true immediately when `g_GameMode == 0`; otherwise it scans a rectangle around a building/candidate area.
- It reads `RulesClass+0x1460` (`AIBaseSpacing`) and computes mask `1 << (HouseClass+0x30 & 0x1F)`.
- Raw instruction `0x0050B800: test dword ptr [eax + 0xdc], ebp` tests each scanned cell.
- Raw call scan found callers at `0x00444FBA` inside `BuildingClass__ExitObject_Main @ 0x00443C60` and `0x00506A33` inside `FUN_005060B0`.

Behavior:

- In nonzero game mode, this predicate returns whether the scanned placement area overlaps a same-house `+0xDC` reservation bit.
- It is used by standard AI building/base-placement flow, including `BuildingClass__ExitObject_Main` for non-player building exits.

### F5 - `FUN_00486D90` reads a current-cell plus adjacent-cell reservation mask

Active in YR: Conditional.

Evidence:

- `FUN_00486D90 @ 0x00486D90` decompiles as:
  - `uVar2 = 1 << (param_2 & 0x1f)`.
  - If `(uVar2 & *(uint *)(param_1 + 0xdc)) == 0`, return `0xffffffff`.
  - Otherwise scan eight `g_DirectionOffsets` neighbors through `MapClass__Get_CellClass`.
  - For each neighbor with `(neighbor+0xDC & uVar2) != 0`, set the corresponding direction bit in the return value.
- Raw call scan found calls at `0x00456106` and `0x0045641D`, near building radar/sensor/gap-adjacent code.

Behavior:

- This helper summarizes connected same-house reservation adjacency from a current cell.
- The exact standard-YR call context remains less certain than the AI/base placement readers above.

### F6 - GapGen/shroud code does not use `+0xDC`

Active in YR: Yes, as a negative fact for the checked GapGen path.

Evidence:

- `BuildingClass__UpdateGapGenerator_Tick @ 0x00454DB0` calls `FUN_00487110` and `FUN_00487130`.
- `FUN_00487110 @ 0x00487110`: `*(uint *)(cell + 0x78) |= 1 << (house & 0x1f)`.
- `FUN_00487130 @ 0x00487130`: `*(uint *)(cell + 0x78) &= ~(1 << (house & 0x1f))`.
- Sensor helpers `CellClass__IncrementSensorCount @ 0x00487150`, `CellClass__DecrementSensorCount @ 0x00487160`, and `CellClass__SensorCountForHouse @ 0x004870D0` use `cell+0x7C + house*2`.

Behavior:

- Gap generator visibility and sensor counts are separate from `CellClass+0xDC`.
- Do not implement GapGen by writing the placement reservation field.

### F7 - `BuildingClass__Unlimbo` was not a verified `+0xDC` writer in this pass

Active in YR: No, for the prior claimed `+0xDC` write in the checked Unlimbo range.

Evidence:

- Raw scan of the `0x00440000..0x00447000` BuildingClass unlimbo/exit region found `+0x122` updates near `0x00440CD9/0x00440CE4` and `0x00445D11/0x00445D1C`, but no `+0xDC` access in `BuildingClass__Unlimbo @ 0x00440580`.
- The `+0xDC` direct reader in this region is the call to `FUN_0050B760` from `BuildingClass__ExitObject_Main @ 0x00444FBA`, not an Unlimbo writer.

Behavior:

- Existing doc wording that says `BuildingClass__Unlimbo` writes `cell+0xDC |= owner mask` should not be used for Rust implementation without a new contradiction-quality proof.

### F8 - Map resize preserves `+0xDC` but is not a reservation setter

Active in YR: Conditional.

Evidence:

- `MapClass__Resize @ 0x00565DF0` copies old cells into a temporary backup, including `puVar8[0x2d] = *(undefined4 *)(iVar13 + 0xdc)`.
- During restore it writes `*(undefined4 *)(puVar16 + 0xdc) = puVar8[0x2e]` as part of the relocated cell-field copy block.
- This function is resize/save-load/editor-adjacent and does not compute or set a new `HouseClass+0x30` reservation mask.

Behavior:

- Resize preserves existing reservation bits across cell reallocation.
- This is not evidence for the missing AI/base reservation setter lifecycle.

## INI / Rules Evidence

Active in YR: Yes.

Evidence:

- `ini/rulesmd.ini:3132` has `AIBaseSpacing=1`.
- `ini/rules.ini:2602` has `AIBaseSpacing=1`.
- `FUN_0050B760` and `FUN_005060B0` read `RulesClass+0x1460`, matching the prior `AIBaseSpacing` identification.

Behavior:

- Default YR AI base reservation scans use an `AIBaseSpacing` value of 1, subject to helper-specific extra spacing for selected building-type flags already covered by the parent report.

## Current Rust Surface

Observed current shape:

- `src/sim/occupancy.rs` models dynamic entity occupancy.
- `src/sim/movement/movement_reservation.rs` models movement destination commitment.
- `src/sim/production/production_placement.rs` exists, but AI/base placement surfaces are incomplete/future.

Interpretation:

- `CellClass+0xDC` should not be collapsed into dynamic entity occupancy.
- The eventual Rust model needs a separate per-cell, per-house AI/base placement reservation map or equivalent cell field if AI base building placement is implemented.
- Writer integration should stay gated until the missing setter lifecycle is verified.

## Implementation Handoff

1. Verified behavior: `CheckOccupancy(rect, -1)` ignores `CellClass+0xDC`, while non-`-1` checks `1 << (arg & 0x1F)` -> Rust delta: add a separate reservation-aware validation path, not dynamic occupancy reuse -> affected surface: future CellRect/placement validators and AI site selection -> acceptance scenario: a reserved cell blocks `CheckOccupancy(rect, house_index)` but not `CheckOccupancy(rect, -1)` -> proposed test name: `check_occupancy_reservation_layer_minus_one_vs_house_index` -> risk: conflating this with live `OccupancyGrid` will make Find_Nearby placement too strict.

2. Verified behavior: `FUN_005060B0` and `FUN_0050B760` read same-house reservation bits using `HouseClass+0x30` and `AIBaseSpacing` -> Rust delta: add a per-cell house-bit reservation map for future AI/base placement reads, with default `AIBaseSpacing=1` from INI -> affected surface: AI/base building placement, not player production placement unless that code is proven to call the same helpers -> acceptance scenario: AI candidate selection changes when the same-house reservation bit is present in the expanded scan area, but not for a different house bit -> proposed test name: `ai_base_reservation_reader_requires_same_house_bit` -> risk: implementing writers prematurely from stale docs may reserve the wrong cells and break AI expansion.

3. Verified behavior: GapGen writes `cell+0x78` and sensor counts use `cell+0x7C`, not `cell+0xDC` -> Rust delta: keep shroud/gap visibility state separate from AI/base reservation state -> affected surface: shroud, gap generator visuals, and future reservation map -> acceptance scenario: activating/deactivating a gap generator changes shroud/gap visibility state without mutating base reservation bits -> proposed test name: `gap_generator_does_not_write_base_reservation_bits` -> risk: reusing `+0xDC` for GapGen will feed visual state into building placement.

## Negative Facts / Do Not Do

- Do not name or implement `CellClass+0xDC` as `GapGenBitmask`; active GapGen evidence writes `cell+0x78` through `FUN_00487110/00487130`, and sensor counts use `cell+0x7C`.
- Do not use `OccupancyGrid` dynamic entity occupancy as the source for `+0xDC`; the verified readers use a per-house bitmask keyed by `HouseClass+0x30`, not object-layer occupancy.
- Do not make `Find_Nearby_Passable_Cell` reject `+0xDC` reservations; the already-settled caller passes `-1`, and `CheckOccupancy(rect, -1)` skips the mask.
- Do not copy the stale `BuildingClass__Unlimbo writes cell+0xDC` claim into Rust; the checked Unlimbo range had `+0x122` activity and no `+0xDC` writer.
- Do not use `AIBaseSpacing=1000`; both base and YR INIs set `AIBaseSpacing=1`.

## Remaining Uncertainty

- No direct set/clear writer for live `CellClass+0xDC` reservations was identified in the scoped AI/base placement or GapGen pass, beyond constructor clear and MapClass resize preservation. A follow-up should search serialization/loading, scenario/base-node import, AI planning, and any indirect bitset helpers.
- `FUN_00486D90` is a verified `+0xDC` adjacency reader, but its exact standard-YR call semantics from the `0x00456106` and `0x0045641D` callers remain unresolved.
- `MapClass__Resize` preserves `+0xDC`, but resize appears map-editor/save-load adjacent; this report does not claim it as a normal skirmish reservation writer.

## Stale Docs / Replacement Wording

`docs/research/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`

Replace wording that labels `Cell+0xDC` as `GapGenBitmask` with:

> `CellClass+0xDC` is a per-house reservation/base-placement bitmask consulted by AI/base placement readers when a non-`-1` layer/house index is passed. It is not the verified Gap Generator visibility bitmask; checked GapGen code writes `CellClass+0x78`, and sensor counts use `CellClass+0x7C`.

`docs/research/CELLCLASS_STRUCT_GHIDRA_REPORT.md`

Replace any `0xDC = GapGenBitmask` field label with:

> `0xDC = per-house AI/base placement reservation bitmask (read by CellRect__CheckOccupancy, FUN_005060B0, FUN_0050B760, FUN_00486D90; setter lifecycle still unverified). Gap/shroud visibility uses 0x78, not 0xDC.`

`docs/research/BUILDING_SYSTEMS_GHIDRA_REPORT.md`

Replace wording that says Gap generator updates `cell+0xDC` with:

> `BuildingClass__UpdateGapGenerator_Tick` updates per-house visibility through helpers at `0x00487110/0x00487130`, which write `CellClass+0x78`; sensor-array counts use `CellClass+0x7C`. It is not evidence for `CellClass+0xDC` reservation writes.

`docs/research/BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`

Replace wording that says `BuildingClass__Unlimbo` writes `cell+0xDC |= (1 << owner_idx)` with:

> This pass did not verify a `BuildingClass__Unlimbo` writer for `CellClass+0xDC`. The checked Unlimbo range shows `+0x122` updates and no `+0xDC` access; `+0xDC` is read later by AI/base placement helpers such as `FUN_0050B760` and `FUN_005060B0`.

## Sources

- `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`
- Ghidra decompilation: `CellClass__Constructor @ 0x0047BC50`
- Ghidra decompilation: `CellRect__CheckOccupancy @ 0x00586780` from prior verified report
- Ghidra decompilation: `FUN_005060B0 @ 0x005060B0`
- Ghidra decompilation: `FUN_0050B760 @ 0x0050B760`
- Ghidra decompilation: `FUN_00486D90 @ 0x00486D90`
- Ghidra decompilation: `BuildingClass__UpdateGapGenerator_Tick @ 0x00454DB0`
- Ghidra decompilation: `FUN_00487110 @ 0x00487110`, `FUN_00487130 @ 0x00487130`, `CellClass__IncrementSensorCount @ 0x00487150`, `CellClass__DecrementSensorCount @ 0x00487160`, `CellClass__SensorCountForHouse @ 0x004870D0`
- Ghidra decompilation: `MapClass__Resize @ 0x00565DF0`
- Repo INI files: `ini/rulesmd.ini`, `ini/rules.ini`
