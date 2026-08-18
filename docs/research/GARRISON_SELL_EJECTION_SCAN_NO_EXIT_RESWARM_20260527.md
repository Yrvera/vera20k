# Garrison Sell Ejection Scan / No-Exit Path -- Reswarm Report

**Address(es):** `BuildingClass::SellBuilding @ 0x00457DE0`, `SpawnUnitsWithParachute @ 0x004585C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `SellBuilding` garrison ejection edge-cell scan order, first-occupant `Can_Enter_Cell` call contract, chosen-coordinate reuse, reverse occupant-vector order, direct successful-ejection handoff boundary, and no-exit split for player sell versus destruction/red-HP callers.
**Non-Scope:** captured sell availability, ownership transfer/revert, fire/weapon/kills, render/`OccupantAnim`, bunker/passenger systems outside `CanBeOccupied` building garrison ejection.
**Confidence:** High for scan order, caller argument split, first-occupant predicate use, reverse order, coordinate reuse, and null no-exit removal. Medium for exact equivalence of current Rust's stand-in terrain predicate because `InfantryClass::Can_Enter_Cell` internals are outside this slice.
**Active in YR:** Yes. Standard YR/RA2 data contains many `CanBeOccupied=yes` civilian buildings, and active player-sell, destruction, and red-HP callers reach `0x00457DE0`.

## 0. Working Notes Gate

**Target question:** Verify `BuildingClass::SellBuilding` garrison ejection scan/no-exit mechanics and compare only to `src/sim/production/production_sell.rs`.

**Non-goals:** Do not re-study captured sell preservation, owner timing, fire, render, bunker, generic transport unload, or full infantry scatter internals.

**Evidence needed to mark COMPLETE:** fresh read-only Ghidra decompile plus assembly context for `0x00457DE0`, caller argument sites, and `0x004585C0`; current Rust scan of `production_sell.rs`; final handoff with concrete test proposals.

**Stop conditions:** Stop once the scan order, first-occupant predicate contract, caller-dependent no-exit behavior, reverse order, coordinate reuse, and current Rust deltas are proven. Defer exact `InfantryClass::Can_Enter_Cell` body and full Scatter internals.

## 1. Overview

`SellBuilding` selects at most one exit cell before the occupant loop. Active in YR: Yes. It scans deterministic foundation-edge candidates, using only garrison occupant slot 0 for the `Can_Enter_Cell`-style virtual call, then converts the chosen cell once and reuses that coordinate for all occupants in reverse vector order.

No-exit behavior is caller-argument dependent. Active in YR: Yes. Player sell passes `push 1; push 0` and falls back to the southeast inside-foundation coordinate. Destruction and red-HP callers pass `push 0; push 0`; if no edge cell is accepted, `SellBuilding` calls `SpawnUnitsWithParachute(0)`, whose null branch destroys/removes occupants in reverse order and does not parachute, scatter, queue mission, or consume RNG.

## 2. Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `SellBuilding` resets `Building+0x69C` and gates on occupant count before scanning. | Fresh decompile `0x00457DE0`; assembly context `0x00457DEB` write and `0x00457DF5` vtable `+0x408` call. | Yes |
| The scan uses the first occupant pointer from `Building+0x688`; later occupants are not probed while selecting the exit cell. | Fresh decompile at `0x00457DE0`; assembly context `0x00457E61` loads `+0x688`, `0x00457E75` loads `[EDX]`, and `0x00457E91..0x00457E93` calls that occupant's vtable. | Yes |
| Candidate predicate call is first occupant vtable `+0x1AC` with arguments equivalent to `Can_Enter_Cell(CellClass*, -1, -1, 0, 1)`, and return `0` accepts. | Assembly context `0x00457E77..0x00457E93`: pushes `1`, `0`, `-1`, `-1`, gets `CellClass`, pushes it, sets `ECX=EDI`, calls `[EBP+0x1AC]`; `0x00457E99..0x00457E9B` accepts on zero. | Yes |
| Edge scan order is east/right column, south/bottom row, north/top row, west/left column. | Fresh decompile `0x00457E35..0x00458060`; assembly contexts around `0x00457E9D`, `0x00457EEF`, `0x00457FAF`, `0x0045802D`. | Yes |
| For a 2x2 at `(ox,oy)`, the native scan tests `(ox+2,oy+2)`, `(ox+2,oy+1)`, `(ox+2,oy)`, `(ox+2,oy-1)`, then bottom row westward, then top row eastward from `(ox,oy-1)`, then left row southward from `(ox-1,oy)`. SE/NE/SW are duplicate tests; NW outside corner is skipped. | Same scan ranges plus branch bounds at `0x00457EB0..0x00457EB8`, `0x00457FA6..0x00457FAD`, `0x0045801E..0x00458027`. | Yes |
| No explicit `SellBuilding` map-edge clamp was found before `MapClass::Get_CellClass`; candidates are handed to the map/cell predicate. | Fresh decompile and assembly pass `0x00457E35..0x00458060` contains candidate arithmetic, `MapClass__Get_CellClass`, and predicate calls, with no explicit nonnegative/bounds guard in the helper. | Yes |
| A chosen cell is converted once, then reused for all occupants. | Assembly context `0x00458060..0x00458078` converts chosen cell before loop; decompile stores the returned coordinate before iterating `Building+0x694`. | Yes |
| Occupants iterate reverse vector order, `count - 1` down to `0`. | Fresh decompile loop; assembly evidence from prior plus fresh context around `0x00458060..0x0045819D` and `0x004585D9..0x004585FA` for fallback order. | Yes |
| Successful `Unlimbo` is followed by vtable `+0x3C8(0)` and direct scatter handoff vtable `+0x174` using a building coordinate; later mission `0xF` is gated by the first stack argument. | Fresh decompile `0x004580BD..0x00458138`; assembly context `0x00458132..0x00458138` shows gated `+0x1E8(0xF,0)` block. | Yes, with `0xF` conditional |
| Individual failed `Unlimbo(chosen_coord,0)` destroys/removes that occupant via vtable `+0xF8`; the loop continues. | Fresh decompile `0x004580BD..0x004580C5`, failure branch to `+0xF8`. | Yes |
| Player sell passes `push 1; push 0` into `SellBuilding`; destruction and red-HP pass `push 0; push 0`. | Caller assembly contexts: `0x0044A5C4..0x0044A5CA`, `0x00442635..0x0044263B`, `0x00458223..0x00458229`; extra direct caller `0x0044D898..0x0044D89C` also passes zero/zero. | Yes for named callers |
| If all edge scans fail, second argument controls fallback: nonzero uses inside-foundation `(origin+W-1, origin+H-1)`, zero calls `SpawnUnitsWithParachute(0)`. | Fresh decompile `0x00458140..0x00458190`; assembly context `0x00458140..0x00458184`. | Yes |
| `SpawnUnitsWithParachute(0)` null branch destroys/removes occupants in reverse order and then clears the garrison vector; no parachute visuals/state are created in that branch. | Fresh decompile `0x004585C0`; assembly context `0x004585CD..0x004585FA`, cleanup `0x00458704..0x00458722`. | Yes for zero-argument callers |

## 3. INI Keys

| Key | Stock source/default | Role in this slice | Binary evidence | Active in YR |
|---|---|---|---|---|
| `CanBeOccupied` | commented default near `ini/rules.ini:3103`; many stock civilian structures set `CanBeOccupied=yes`, e.g. `ini/rules.ini:10813` onward | Makes the garrison vector active and gates destruction caller through `BuildingType+0x157B`. | Caller context `0x0044262B..0x0044263B` reads `+0x157B` before `SellBuilding`. | Yes |
| `MaxNumberOccupants` | examples at `ini/rules.ini:10814` and many adjacent civilian entries | Capacity/data for the occupant vector; not part of exit scan order. | Runtime loop reads vector count `Building+0x694`; key parser chain not re-opened in this slice. | Yes |
| `Foundation` | object type data | Source of `W/H` through building type foundation helpers. | Fresh decompile and assembly context `0x00457E35..0x00457E48` calls width/height helpers. | Yes |

## 4. Current Rust Status

| Rust surface | Current behavior | Match status |
|---|---|---|
| `src/sim/production/production_sell.rs:256` `garrison_sellbuilding_exit_cells` | Implements east, south, north, west scan with duplicate SE/NE/SW corners and skipped NW for normal in-bounds foundations. | Mostly matches in-bounds scan order. Drift/unchecked at map edges because `push_nonnegative_cell` prefilters negative cells, while native helper has no proven explicit clamp before `MapClass::Get_CellClass`. |
| `src/sim/production/production_sell.rs:291` `garrison_first_occupant_can_enter_cell` and `:330` `choose_garrison_exit_cell` | Uses only passenger slot 0 to probe cells. | Matches first-occupant selection contract; predicate body is a stand-in, not exact `InfantryClass::Can_Enter_Cell`. |
| `src/sim/production/production_sell.rs:300` `garrison_infantry_can_enter_cell` | Uses Rust `check_terrain` with ground movement and occupancy. | Partial/mismatch risk: native call is vtable `+0x1AC(CellClass*,-1,-1,0,1)`, so exact terrain/subcell/bridge/reservation semantics remain unchecked here. |
| `src/sim/production/production_sell.rs:471` `eject_garrison_passengers_at_edges` | Chooses one exit cell once, uses mode split for player sell versus destruction, iterates passengers in reverse, and removes passengers on destruction no-exit. | Matches the verified high-level contract for coordinate reuse, reverse order, and no-exit mode split. |
| `src/sim/production/production_sell.rs:420` `place_garrison_passenger_at_cell` | Directly mutates entity placement and returns `true` if the entity exists. | Partial: does not model native `Unlimbo(chosen_coord,0)` failure; native kills only an occupant whose `Unlimbo` fails after a coordinate exists. |
| `src/sim/production/production_sell.rs:359` `sellbuilding_direct_scatter_handoff` | Approximate direct scatter/destination logic with `next_range_u32_inclusive(0,4)`, no exact mission queue surface. | Out-of-scope for full scatter; still a partial stand-in for the successful post-`Unlimbo` handoff. |
| Tests at `src/sim/production/production_sell.rs:1000`, `:1029`, `:1222`, `:1300` | Cover scan order, slot-0 probing, player-sell inside fallback, and destruction no-exit removal. | Good coverage for this slice, except map-edge negative-candidate parity and exact native predicate/Unlimbo failure remain uncovered. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::SellBuilding @ 0x00457DE0` | verified | Fresh decompile; assembly contexts `0x00457DEB`, `0x00457E77..0x00457E93`, `0x00458140..0x00458184` | none for scoped ejection helper |
| Scan edge order and duplicate/skipped corners | verified | Fresh decompile `0x00457E35..0x00458060`; assembly contexts at scan loop branches | none |
| First occupant predicate call contract | verified | Assembly `0x00457E61..0x00457E93`, `0x00457E99..0x00457E9B` | exact vtable target body deferred |
| Caller argument split | verified | Assembly caller contexts `0x0044A5C4..0x0044A5CA`, `0x00442635..0x0044263B`, `0x00458223..0x00458229` | identity of extra `0x0044D89C` caller not needed |
| Player sell inside-foundation fallback | verified | Fresh decompile `0x00458140..0x0045817B`; caller `0x0044A5C4..0x0044A5CA` | runtime fixture not captured |
| Destruction/red-HP no-exit removal | verified | Caller contexts and `SpawnUnitsWithParachute(0)` null branch `0x004585CD..0x004585FA` | subclass internals of vtable `+0xF8` deferred |
| Successful post-`Unlimbo` direct handoff boundary | touched-not-exhausted | Fresh decompile `0x004580BD..0x00458138`; prior scatter report | full Scatter internals out-of-scope |
| Current Rust comparison | verified | `production_sell.rs` line scan and codegraph symbol search | no Rust edited |

## 6. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Is this path active in standard YR? -> Yes, via stock `CanBeOccupied=yes` civilian data and active sell/destruction/red-HP callers.` (evidence: `ini/rules.ini:10813`, `0x0044A5CA`, `0x0044263B`, `0x00458229`)
- `[RESOLVED] OQ-02 -- Which occupant is used for exit probing? -> only slot 0 from `Building+0x688`.` (evidence: `0x00457E61..0x00457E93`)
- `[RESOLVED] OQ-03 -- What arguments does the probe use? -> `CellClass*`, `-1`, `-1`, `0`, `1`; return zero accepts.` (evidence: `0x00457E77..0x00457E99`)
- `[RESOLVED] OQ-04 -- What is exact scan order? -> east column, south row, north row, west column; duplicate SE/NE/SW, skipped NW outside corner.` (evidence: `0x00457E35..0x00458060`)
- `[RESOLVED] OQ-05 -- Is one exit reused for all occupants? -> Yes; chosen cell is converted before the reverse loop.` (evidence: `0x00458060..0x00458078`, decompile loop)
- `[RESOLVED] OQ-06 -- What is occupant order? -> reverse vector order, high index to low.` (evidence: decompile `0x00457DE0`; null branch assembly `0x004585D9..0x004585FA`)
- `[RESOLVED] OQ-07 -- What happens when all edge probes fail for player sell? -> fallback to inside-foundation `(origin+W-1,origin+H-1)`.` (evidence: caller `0x0044A5C4..0x0044A5CA`; branch `0x00458140..0x0045817B`)
- `[RESOLVED] OQ-08 -- What happens when all edge probes fail for destruction/red-HP? -> `SpawnUnitsWithParachute(0)` null branch removes occupants, no parachute/scatter/RNG.` (evidence: callers `0x00442635..0x0044263B`, `0x00458223..0x00458229`; branch `0x004585CD..0x004585FA`)
- `[RESOLVED] OQ-09 -- Does current Rust match the major ejection skeleton? -> Mostly: scan order in-bounds, first occupant, one exit, reverse loop, mode split are present.` (evidence: `production_sell.rs:256`, `:291`, `:330`, `:471`, `:534`, `:597`)
- `[RESOLVED] OQ-10 -- What Rust drift remains in this slice? -> exact `Can_Enter_Cell` body, map-edge negative candidate handling, and native `Unlimbo` failure are not modeled exactly.` (evidence: `production_sell.rs:250`, `:300`, `:420`)
- `[DEFERRED] OQ-11 -- Exact `InfantryClass::Can_Enter_Cell` body for vtable `+0x1AC`.` (category: `out-of-scope`; reason: this slot only needed the caller contract and current Rust surface; next-step-if-pursued: audit `0x0051BF90`/cell-entry reports)
- `[DEFERRED] OQ-12 -- Full InfantryClass Scatter internals after successful direct handoff.` (category: `out-of-scope`; reason: separate scatter reports cover it and this target only asked for direct handoff boundary; next-step-if-pursued: verify against `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Edge scan is fixed east/south/north/west, probes only slot 0 with `Can_Enter_Cell(CellClass*,-1,-1,0,1)==0`, and has no proven pre-`Get_CellClass` clamp. | Fresh decompile `0x00457E35..0x00458060`; assembly `0x00457E61..0x00457E99` | in-bounds order matches; map-edge prefilter and predicate body are not exact | `production_sell.rs::garrison_sellbuilding_exit_cells`, `choose_garrison_exit_cell`, `garrison_infantry_can_enter_cell` | Preserve current in-bounds order, but do not claim map-edge or predicate exactness until native `Can_Enter_Cell` semantics are bound. | 2x2 building at `(0,0)` with blocked/invalid edge cells should be compared to native: negative candidates must be passed/rejected equivalently, not silently omitted unless proven equivalent. Proposed test: `garrison_sellbuilding_scan_order_map_edge_matches_native_predicate` | Do not replace the scan with sorted survivor positions or per-occupant probing. |
| One chosen coordinate is reused for all occupants, processed high-index to low-index; individual failed `Unlimbo` destroys that occupant. | Fresh decompile `0x00458060..0x0045819D`; failure branch `0x004580BD..0x004580C5` | one-exit/LIFO match; Rust `place_garrison_passenger_at_cell` does not expose native `Unlimbo` failure | `production_sell.rs::eject_garrison_passengers_at_edges`, `place_garrison_passenger_at_cell` | Keep one-coordinate LIFO behavior; add/keep a future placement failure path that removes only occupants whose native-equivalent unlimbo fails after a coordinate exists. | Occupant stack at accepted edge cell where later `Unlimbo` would fail should remove that occupant without selecting another edge cell. Proposed test: `garrison_sellbuilding_failed_unlimbo_removes_only_failed_occupant` | Do not advance to the next edge cell to avoid stacking/failure. |
| No-exit is caller-mode dependent: player sell uses inside-foundation fallback; destruction/red-HP use `SpawnUnitsWithParachute(0)` null removal. | Caller contexts `0x0044A5C4..0x0044A5CA`, `0x00442635..0x0044263B`, `0x00458223..0x00458229`; branch `0x00458140..0x00458190`; null branch `0x004585CD..0x004585FA` | Rust mode split matches current verified contract | `production_sell.rs::eject_garrison_occupants`, `eject_destruction_garrison`, `eject_red_hp_garrison` | Preserve separate `PlayerSell` vs destruction/red-HP modes; ensure no parachute state/RNG/scatter is attached on zero-arg null removal. | Fully blocked exits: player sell keeps occupant at `(origin+W-1,origin+H-1)`; destruction marks occupants removed with unchanged RNG. Proposed tests: `garrison_player_sell_no_exit_uses_inside_foundation_fallback`, `garrison_destruction_no_exit_removes_without_rng_or_scatter` | Do not make `SpawnUnitsWithParachute(0)` universal for player sell and do not add parachute visuals for zero-arg fallback. |

## 8. Negative Facts / Do Not Do

- Do not scan every occupant for a usable exit cell. Evidence: slot-0 load/call at `0x00457E61..0x00457E93`. Active in YR: Yes.
- Do not sort perimeter cells or remove duplicate corner probes. Evidence: scan loop ranges `0x00457E35..0x00458060`. Active in YR: Yes.
- Do not add the northwest outside-corner probe for the native scan. Evidence: third/fourth loops increment from `(ox-1,oy-1)` before `MapClass::Get_CellClass` at `0x00457FAF` and `0x0045802D`. Active in YR: Yes.
- Do not treat player-sell no-exit as `SpawnUnitsWithParachute(0)`. Evidence: player sell caller `0x0044A5C4..0x0044A5CA` passes nonzero second arg; branch `0x00458148..0x0045817B` builds inside-foundation coordinate. Active in YR: No for normal player sell.
- Do not implement `SpawnUnitsWithParachute(0)` as actual parachuting. Evidence: null branch `0x004585CD..0x004585FA` only calls occupant `+0xF8` then vector cleanup. Active in YR: No for zero-arg fallback.

## 9. Remaining Uncertainty

- Exact native body/return ladder of first occupant vtable `+0x1AC` remains out-of-scope; current Rust terrain predicate is only a stand-in.
- Native `MapClass::Get_CellClass` handling for negative/off-map candidates was not drained; because `SellBuilding` itself does not clamp, Rust's `push_nonnegative_cell` is an unproven equivalence at map edges.
- Exact placement-failure conditions behind occupant `Unlimbo(chosen_coord,0)` are deferred; current Rust placement path does not model that virtual's failure.
- The extra zero/zero direct caller at `0x0044D89C` was observed but not identified; it has the same no-exit contract as destruction/red-HP for this slice.

## 10. Stale Docs / Follow-up Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`: replace "Medium-high. Handoff-critical facts are backed by recent Ghidra reports ... no callable Ghidra MCP tool" with "High for the SellBuilding scan/no-exit slice after fresh read-only Ghidra MCP verification on 2026-05-27; see `GARRISON_SELL_EJECTION_SCAN_NO_EXIT_RESWARM_20260527.md` for current primary evidence."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`: replace "The branch is reached from live YR sell/destruction callers when ... all four exit-edge scans fail" with "The zero-argument `SpawnUnitsWithParachute(0)` branch is reached from live destruction/red-HP and other zero-second-argument callers when all edge scans fail; normal player sell passes a nonzero second argument and uses the inside-foundation fallback instead."

## Sources

- Fresh read-only Ghidra decompile: `0x00457DE0`, `0x004585C0`.
- Fresh read-only Ghidra assembly contexts: `0x00457DEB`, `0x00457E35..0x00458060`, `0x00457E77..0x00457E99`, `0x00458140..0x00458184`, caller sites `0x0044A5C4..0x0044A5CA`, `0x00442635..0x0044263B`, `0x00458223..0x00458229`, `0x0044D898..0x0044D89C`, fallback branch `0x004585CD..0x004585FA`, cleanup `0x00458704..0x00458722`.
- Prior reports read: `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`, `GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`, `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`.

## Status

COMPLETE.
