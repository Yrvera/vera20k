# CellRect::CheckOccupancy @ 0x00586780 - Full Blocker Tree

**Address(es):** `0x00586780` primary, helpers `0x0047C550`, `0x0047C520`, `0x00578390`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `CellRect::CheckOccupancy(rect, reservation_arg)` blocker tree, second-argument behavior, direct YR callsites needed to name the argument, playfield handling, and bridge/layer negative facts.  
**Non-Scope:** full `CellRect::CheckPassability`, full `Find_Nearby_Passable_Cell`, complete `CellClass+0xDC` writer lifecycle, complete class identity for RTTI `0x24`, and broad building placement.  
**Confidence:** Medium-High. The blocker tree and call contracts are inherited from recent Ghidra-backed reports; this slot had no live Ghidra MCP tool exposed, so no new decompiler read was possible.  
**Active in YR:** Yes. The function is called by live `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` and live AI/site helper `FUN_005060B0 @ 0x005060B0`.

## 1. Overview

`CellRect::CheckOccupancy` is a rectangle blocker validator, not a terrain passability validator. It walks each cell in `rect`, rejects a fixed set of object-list/cell-field/reservation blockers, then returns the result of a final four-corner playfield check.

The second argument is not a bridge layer. It is a reservation bit index: `-1` disables `CellClass+0xDC` filtering, while any other value checks `1 << (arg & 0x1F)` against every visited cell's `+0xDC`.

## 2. Class Layout / Key Offsets

| Offset / item | Verified behavior in this function | Active in YR | Evidence |
|---|---|---:|---|
| `CellRect` | Four 32-bit fields: `x`, `y`, `width`, `height`; low 16 bits are used as signed cell coords during lookup. | Yes | `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `0x005867AC..0x005867CE` |
| `CellClass+0x44` | Must be `-1`; any other value blocks. | Yes | validator report, `0x0058682D..0x00586832`; struct report names it `OverlayTypeIndex` |
| `CellClass+0x4C` | Must be zero; any nonzero value blocks. Human semantic name is disputed across docs, but the read/reject behavior is verified. | Yes | validator report, `0x00586833..0x00586838`; `CELLCLASS_STRUCT_GHIDRA_REPORT.md` calls it `ZoneType` |
| `CellClass+0x11C` | Must be zero; any nonzero slope/special byte blocks. | Yes | validator report, `0x0058683A..0x00586842`; struct report calls it `SlopeIndex` |
| `CellClass+0xDC` | Optional per-house/base placement reservation bitmask. Checked only when argument is not `-1`. | Conditional | validator report `0x00586787..0x0058679D`, `0x0058681F..0x0058682B`; reservation lifecycle report |
| `CellClass+0xE4` | Ground object-list head used by both helper scans in this function. | Yes | validator report; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` |
| `CellClass+0xE8` | Bridge/deck object-list head. Not read by this function. | No direct use | bridge object-list report; absent from validator body per validator report |
| `CellClass+0x124/+0x128` | Ground/bridge occupation bitfields. Not read by this function. | No direct use | validator report distinguishes `CheckPassability`; bridge object-list report |
| `CellClass+0x140` | Cell flags, including bridge structural flags. Not read by this function. | No direct use | validator report; bridge docs |

## 3. Core Logic

Verified pseudocode, preserving return polarity:

```text
bool CheckOccupancy(CellRect* rect, int reservation_arg):
    if reservation_arg == -1:
        reservation_mask = 0
    else:
        reservation_mask = 1 << (reservation_arg & 0x1F)

    for x in rect.x .. rect.x + rect.width - 1:
        for y in rect.y .. rect.y + rect.height - 1:
            cell = get_cell_or_dummy(x.low16_signed, y.low16_signed)

            if FUN_0047C550(cell, 0) != null:
                return false

            if (cell[0xDC] & reservation_mask) != 0:
                return false

            if cell[0x44] != -1:
                return false

            if cell[0x4C] != 0:
                return false

            if cell[0x11C] != 0:
                return false

            if Look_up_building_in_cell(cell) != null:
                return false

    return MapClass__IsRectInPlayfield(rect, 1)
```

Tiny details:

- The `-1` check is exact. Other negative values are not skipped; they alias through `arg & 0x1F`. Example: `-2` would target bit 30.
- The reservation mask is computed once before the rectangle scan, not per cell.
- `arg & 0x1F` means bit indices alias every 32, regardless of the normal YR house count.
- Zero or negative `width` / `height` skip the blocker scan and still run `MapClass__IsRectInPlayfield(rect, 1)`, which uses `x + width - 1` and `y + height - 1` for corner checks.
- Out-of-array or null cell lookups use the dummy cell during the scan, but the final playfield check is still what rejects out-of-play rectangles on the success path.
- Return semantics are `true/nonzero = clear and in playfield`, `false/zero = blocked or out of playfield`. Older prose that says the function returns nonzero on rejection is stale/inverted.

## 4. INI Keys

The function reads no INI keys directly.

| Key / data | Relationship to this function | Active in YR | Evidence |
|---|---|---:|---|
| `AIBaseSpacing=1` | Used by AI/site helper to expand/probe candidate rectangles before direct `CheckOccupancy(expanded_rect, HouseClass+0x30)` calls. | Conditional | `CELLCLASS_0XDC_RESERVATION_LIFECYCLE_GHIDRA_REPORT.md`; `ini/rulesmd.ini` / `ini/rules.ini` |
| `SpeedType=`, `MovementZone=`, land speed table | Not read here; these belong to `CheckPassability` / `CellClass::CheckCellPassability`. | No direct use | validator report |
| `Buildable=` | Not read here; building placement has separate predicates. | No direct use | validator report; placement docs |

## 5. Integration Points

| Caller / helper | What was verified | Active in YR | Evidence |
|---|---|---:|---|
| `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` | Calls `CheckOccupancy(rect, -1)` only when final occupancy flag is enabled; reservation filtering is skipped. | Yes | validator report xrefs `0x0056DE9D`, `0x0056E0B3`, `0x0056E2F6`, `0x0056E4F8`; caller matrix report |
| `FUN_005060B0 @ 0x005060B0` | Calls `CheckOccupancy(expanded_rect, HouseClass+0x30)`, so reservation bits are active and keyed by house index. | Conditional | validator report `0x005069C0..0x005069DB`; reservation lifecycle report |
| `FUN_0047C550 @ 0x0047C550` | First blocker helper; scans the ground object list for RTTI `0x24` when passed layer `0`. Exact class identity remains out-of-scope. | Yes | validator report; overlay/class docs note `0x24` object class |
| `Look_up_building_in_cell @ 0x0047C520` | Final per-cell object-list helper; scans `CellClass+0xE4` and returns first object whose `WhatAmI()==6`. | Yes | validator report; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` |
| `MapClass__IsRectInPlayfield @ 0x00578390` | Final success-path bounds test; checks all four corners using inclusive `width-1` / `height-1` extents. | Yes | validator report |

Bridge/layer interaction:

- There is no explicit bridge-layer argument in this function.
- The helper calls use the ground list (`+0xE4`) in the verified call shape; the bridge/deck list (`+0xE8`) is not scanned.
- `CellClass+0x124/+0x128` occupation bitfields are not read here. They belong to `CheckCellPassability` and `Can_Enter_Cell`.
- `CellClass+0x140` bridge structural flags are not read here, so bridge rejection is only indirect through the cell fields that this function actually tests (`+0x44`, `+0x4C`, `+0x11C`, object helpers, reservation mask).

## 6. Current Rust Implementation Status

| Rust area | Current shape | Delta against verified behavior |
|---|---|---|
| `C:/Users/enok/Documents/ra2-rust-game/src/sim/occupancy.rs` | `OccupancyGrid` stores dynamic entity occupants by `MovementLayer`, with ground/bridge filtering and object-list-like ordering. | Does not model `CellClass+0xDC` reservations, `+0x44/+0x4C/+0x11C` blocker fields, dummy-cell scan behavior, or final four-corner playfield semantics as a `CellRect::CheckOccupancy` surface. |
| `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs` | Has `CanEnterLayerContext` separating terrain, object-list, and occupancy-bit layers. | Useful for `Can_Enter_Cell`; it is not the same as `CheckOccupancy`, which does not read bridge occupation bitfields and scans only the verified helper object list. |
| `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_placement.rs` | Has placement checks for terrain/static blockers/bridges and structure overlap. | This is a separate building-placement predicate family; it does not yet represent `CheckOccupancy(rect, house_index)` with `+0xDC` reservation filtering. |
| `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_spawn.rs` and `production_spawn.rs` | Spawn logic uses current path/occupancy surfaces and bridge spawn layer state. | Nearby-cell final occupancy parity needs a distinct `CheckOccupancy(rect, -1)` equivalent that skips reservations but rejects the other cell/object blockers. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellRect::CheckOccupancy @ 0x00586780` full blocker order | verified-from-prior-binary-report | `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` | none for listed tree; this slot lacked live Ghidra MCP for a fresh re-read |
| Reservation argument `-1` vs non-`-1` | verified | validator report; reservation lifecycle report | writer lifecycle for `+0xDC` remains separate |
| `FUN_0047C550(cell,0)` effect | verified for blocker effect | validator report `0x0047C550` | exact class name behind RTTI `0x24` |
| `Look_up_building_in_cell @ 0x0047C520` | verified | validator report; occupancy ordering docs | none for ground-list `WhatAmI()==6` helper contract |
| `MapClass__IsRectInPlayfield @ 0x00578390` | verified for final call and corner semantics | validator report | internals of `Is_Cell_In_Playfield` not re-opened |
| Bridge/deck list `+0xE8` participation | verified negative fact | validator report absence; bridge object-list report | no further bridge object-list audit needed for this function |
| `CheckPassability` distinction | touched-not-exhausted | validator report | intentionally not re-investigated beyond negative distinction |
| Current Rust parity surfaces | touched-not-exhausted | codegraph + `rg` scan of relevant files | exact future API design deferred to implementation work |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `0x00586780` active in standard YR? -> Yes, live callers include `Find_Nearby_Passable_Cell` and `FUN_005060B0`.` (evidence: validator report xrefs; Active in YR: Yes)
- `[RESOLVED] OQ-2 - What is the return polarity? -> True/nonzero means clear and in playfield; false/zero means blocker or out-of-playfield.` (evidence: validator report final `MapClass__IsRectInPlayfield`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What does `reservation_arg == -1` do? -> It sets the reservation mask to zero and skips `Cell+0xDC` filtering.` (evidence: `0x00586787..0x0058679D`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - What does non-`-1` reservation arg do? -> It checks `1 << (arg & 0x1F)` against `CellClass+0xDC` on every scanned cell.` (evidence: `0x0058681F..0x0058682B`; Active in YR: Conditional)
- `[RESOLVED] OQ-5 - Does FNPC allow callers to choose the reservation layer? -> No; FNPC's final occupancy calls pass `-1`.` (evidence: caller matrix report; Active in YR: Yes)
- `[RESOLVED] OQ-6 - Does AI/site helper use reservation filtering? -> Yes; it passes `HouseClass+0x30`.` (evidence: `0x005069C0..0x005069DB`; Active in YR: Conditional)
- `[RESOLVED] OQ-7 - Does this function read bridge/deck object list `+0xE8`? -> No verified read; helper scans are ground-list oriented.` (evidence: validator report + bridge object-list report; Active in YR: No direct use)
- `[RESOLVED] OQ-8 - Does this function read `+0x124/+0x128` occupation bits? -> No; those belong to passability / Can_Enter_Cell.` (evidence: validator report; Active in YR: No direct use)
- `[RESOLVED] OQ-9 - Does it read bridge flags `+0x140`? -> No; any bridge effect is indirect through tested cell fields/helpers.` (evidence: validator report; Active in YR: No direct use)
- `[RESOLVED] OQ-10 - Does it include terrain speed, MovementZone, or LandType checks? -> No; that is `CheckPassability`/`CheckCellPassability`.` (evidence: validator report; Active in YR: No direct use)
- `[RESOLVED] OQ-11 - What happens for zero or negative dimensions? -> The scan loops skip, then final playfield check uses `width-1`/`height-1` corners.` (evidence: validator report; Active in YR: Conditional)
- `[RESOLVED] OQ-12 - Are out-of-bounds cells immediately rejected during scan? -> Not necessarily; scan can use dummy cell, then final playfield check rejects success-path out-of-play rectangles.` (evidence: validator report; Active in YR: Yes)
- `[RESOLVED] OQ-13 - Is `Cell+0xDC` GapGen? -> No for checked behavior; GapGen uses `+0x78` and sensors `+0x7C`.` (evidence: reservation lifecycle report; Active in YR: Yes)
- `[DEFERRED] OQ-14 - Exact class identity behind RTTI `0x24`.` (category: out-of-scope; reason: blocker effect is enough for this function; next-step-if-pursued: class RTTI enum audit)
- `[DEFERRED] OQ-15 - Complete writer/clearer lifecycle for `CellClass+0xDC`.` (category: requires-different-system-context; reason: read contract was settled by parent and lifecycle has its own report with unresolved setters; next-step-if-pursued: scenario/base-node/AI reservation writer audit)
- `[DEFERRED] OQ-16 - Exact semantic name of `CellClass+0x4C`.` (category: requires-different-system-context; reason: docs conflict between validator wording and struct `ZoneType`; next-step-if-pursued: audit all `+0x4C` writers/readers and update struct naming)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CheckOccupancy(rect, -1)` skips `CellClass+0xDC` but still rejects `FUN_0047C550`, `+0x44`, `+0x4C`, `+0x11C`, ground-list building lookup, and out-of-playfield rectangles. | validator report `0x00586780`; caller matrix report | Missing as a distinct validator | `src/sim/occupancy.rs`, nearby-cell/spawn helpers | Add a rectangle blocker predicate separate from dynamic layer occupancy and separate from passability. | Proposed test: `check_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers` | Do not model `-1` as "skip all occupancy"; it skips only the reservation bitmask. |
| Non-`-1` argument checks `1 << (arg & 0x1F)` against every cell's `+0xDC`. | `0x00586787..0x0058679D`, `0x0058681F..0x0058682B`; reservation lifecycle report | No per-cell per-house reservation map exists | future AI/base placement reservation surface | Represent reservation bits separately from entity occupancy and allow `-1` skip vs house-index filtering. | Proposed test: `check_occupancy_reservation_layer_minus_one_vs_house_index` | Do not merge `+0xDC` with GapGen, shroud, or `OccupancyGrid` occupants. |
| `Find_Nearby_Passable_Cell` final occupancy always calls `CheckOccupancy(rect, -1)`. | caller matrix report; validator report | Production spawn/nearby fallback does not expose this exact contract | `src/sim/production/production_spawn.rs`, future FNPC parity helper | Final occupancy in nearby search must reject object/cell blockers but ignore reservation-only blockers. | Proposed test: `find_nearby_passable_final_occupancy_ignores_reservation_layer` | Do not let AI/base reservation bits make normal FNPC fail. |
| AI/site helper calls `CheckOccupancy(expanded_rect, HouseClass+0x30)`. | `0x005069C0..0x005069DB`; reservation lifecycle report | AI/base placement reservation surface missing/unchecked | future AI/base building placement | Apply same-house reservation filtering for AI site selection, with `AIBaseSpacing` expansion in the caller logic. | Proposed test: `ai_base_spacing_reservation_blocks_same_house_expanded_rect` | Do not apply this behavior to player building placement unless its caller path is verified. |
| Bridge/deck object list and bridge occupation bitfields are not part of this function. | validator report absence; bridge object-list report | Existing Rust has rich layer occupancy; easy to overuse it here | `src/sim/pathfinding/cell_entry.rs`, `src/sim/occupancy.rs` | Keep `CheckOccupancy` narrower than `Can_Enter_Cell`: ground-list helper scans plus cell-field blockers, no `+0xE8/+0x128` scan. | Proposed test: `check_occupancy_does_not_treat_bridge_deck_occupant_as_ground_blocker` | Do not reuse layered movement entry logic wholesale for this validator. |

### Negative Facts / Do Not Do

- Do not call `CellClass+0xDC` GapGen. Active GapGen evidence writes `CellClass+0x78`; sensors use `+0x7C`.
- Do not implement `CheckOccupancy` as terrain passability. It reads no `SpeedType`, `MovementZone`, `LandType`, speed table, or zone id.
- Do not make FNPC final occupancy consult reservations; FNPC passes `-1`.
- Do not treat the second argument as bridge/ground layer selection. It only builds the `+0xDC` reservation mask.
- Do not scan `CellClass+0xE8` or `+0x128` for this function unless fresh binary evidence contradicts the current validator report.
- Do not use older inverted wording that says `CheckOccupancy` returns nonzero on rejection.
- Do not resolve the `+0x4C` naming conflict by guessing. The implementation-critical fact is only `nonzero blocks` for this function.

### Stale Docs / Follow-up Docs

- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: older wording says `CheckOccupancy` returns nonzero on rejection and calls `Cell+0xDC` `GapGenBitmask`. Replace with: `FNPC calls CellRect::CheckOccupancy(rect, -1) when final occupancy is enabled. The function returns true only if the rectangle has no verified cell/object blockers and is in the playfield; with -1 it skips CellClass+0xDC reservation filtering. CellClass+0xDC is a per-house reservation bitmask, not GapGen.`
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md`: replace `0xDC = GapGenBitmask` with the reservation lifecycle report wording.
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace `no other units blocking` summary with the full object/cell/playfield blocker list and note the reservation skip in FNPC.
- `BUILDING_SYSTEMS_GHIDRA_REPORT.md`: replace any GapGen `+0xDC` claim with GapGen `+0x78` / sensors `+0x7C`.

## 10. Remaining Uncertainty

- Exact concrete class name for RTTI `0x24` in `FUN_0047C550` remains deferred; the blocker effect is verified.
- `CellClass+0x4C` has a human-name conflict in prior docs. This report claims only the verified `nonzero blocks` behavior inside `CheckOccupancy`.
- Full writer/clearer lifecycle for `CellClass+0xDC` remains unresolved outside constructor/resize preservation and readers; do not implement speculative writers from stale docs.
- This subagent could not perform a fresh live Ghidra read because no Ghidra MCP tools were exposed in the session. The report relies on recent Ghidra-backed reports and local Rust/doc scans.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/CELLCLASS_0XDC_RESERVATION_LIFECYCLE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/CELLCLASS_STRUCT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/occupancy.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_placement.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_spawn.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_spawn.rs`
