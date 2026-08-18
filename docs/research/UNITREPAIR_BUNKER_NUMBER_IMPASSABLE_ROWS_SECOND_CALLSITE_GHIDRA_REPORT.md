# UnitRepair/Bunker NumberImpassableRows Second Callsite - Ghidra Research Report

**Address(es):** `0x0073F0A0` (`UnitClass::Can_Enter_Cell`), focused callsite `0x0073F76D`, helper `0x00458A00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Reconcile the second `NumberImpassableRows` helper callsite for stock UnitRepair buildings (`GADEPT`, `NADEPT`, `YADEPT`, `CAOUTP`) and `NATBNK`, including which same-building cells are skipped vs. remain blockers and where this belongs in Rust.  
**Non-Scope:** The first radio/contact callsite at `0x0073F5A2`, full downstream `Can_Enter_Cell` return-code matrix, bunker enter/eject/destruction lifecycle beyond the already-verified `+0x2E4` occupancy claim, and static placement occupancy.  
**Confidence:** High for the callsite, helper polarity, stock INI/art consequences, and Rust-surface delta; Medium for `YADEPT` art image inheritance because the binary image resolution path was not re-traced in this slice.  
**Active in YR:** Yes, conditionally: the branch is live in YR `UnitClass::Can_Enter_Cell` when the current cell-list object is a building whose `BuildingTypeClass+0x16A9` (`UnitRepair`) or `+0x16AB` (`Bunker`) flag is set and `Look_up_building_in_cell(candidate_cell)` returns that same building.

## 1. Overview

The second direct caller of helper `0x00458A00` is the UnitRepair/Bunker branch inside `UnitClass::Can_Enter_Cell`. It does not precompute a permanent passability mask; while scanning a candidate cell's object list, it may ignore the current building occupant for that candidate cell when the helper returns false.

For stock repair depots and CAOUTP, `NumberImpassableRows` protects west-side foundation columns and skips the remaining same-building foundation columns. For stock `NATBNK`, `NumberImpassableRows=0` makes an empty bunker skipped on all same-foundation cells, but an occupied bunker returns true before row math because `BuildingClass+0x2E4` is nonzero.

## 2. Class Layout / Key Offsets

| Class / object | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16A9` | `UnitRepair=yes` flag tested by the second callsite | parser reads `UnitRepair` at `0x0046090D` and stores at `0x00460929`; callsite reads `0x0073F74B` | Yes for `GADEPT`, `NADEPT`, `YADEPT`, `CAOUTP` stock INI |
| `BuildingTypeClass` | `+0x16AB` | `Bunker=yes` flag tested by the second callsite and helper | parser reads `Bunker` at `0x00460941` and stores at `0x00460954`; callsite reads `0x0073F755`; helper reads `0x00458A30` | Yes for `NATBNK` stock INI |
| `BuildingTypeClass` | `+0x1620` | `NumberImpassableRows` integer; `-1` sentinel means helper true for all same-building cells | parser reads at `0x0046013A` and stores at `0x00460147`; helper reads `0x00458A20`, `0x00458A67` | Yes for scoped stock buildings |
| `BuildingClass` | `+0x2E4` (`param_1[0xB9]`) | Bunker contained-unit pointer, used only for the Bunker fast-true override | helper read `0x00458A3C`; prior report verifies writer `0x00459301` | Conditional: nonzero after tank-bunker install |
| `CellClass` | `+0x24` | Candidate cell X coordinate used in strict row comparison | helper read `0x00458A51` | Yes |

## 3. Core Logic

Focused branch in `UnitClass::Can_Enter_Cell`:

1. The object-list scan has reached a building object.
2. If neither `BuildingType+0x16A9` (`UnitRepair`) nor `+0x16AB` (`Bunker`) is set, the branch is bypassed.
3. `Look_up_building_in_cell(candidate_cell)` must equal the current building object; otherwise the row helper is not called.
4. The callsite passes `ECX = current building`, stack argument `candidate CellClass*`, then tests `AL`.
5. If helper returns false, `JZ 0x0073FA87` advances to the next object-list occupant; this building does not block this candidate cell at this branch.
6. If helper returns true, execution continues at `0x0073F77A` into the normal building blocker path; true is not itself the final passability code.

Helper polarity, restated only for this callsite:

```text
if Look_up_building_in_cell(candidate_cell) != building:
    return false
if building.Type.NumberImpassableRows == -1:
    return true
if building.Type.Bunker && building.bunker_occupant_ptr != null:
    return true
return candidate_cell.x < building.origin_cell.x + building.Type.NumberImpassableRows
```

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The branch is an OR gate: `UnitRepair` true is enough; otherwise `Bunker` true is enough. | `0x0073F74B..0x0073F75D` | High | Yes, conditional on stock INI flags |
| The helper is called only after the candidate cell's primary building lookup equals the current building. | `0x0073F75F..0x0073F768` | High | Yes |
| Helper false skips the current building occupant, not the whole cell. | `0x0073F772` `TEST AL,AL`; `0x0073F774` `JZ 0x0073FA87`; loop advances via object `+0x30` | High | Yes |
| Helper true keeps the building in later normal blocking logic; it does not immediately return "blocked". | fall-through `0x0073F77A`; later building logic in `UnitClass::Can_Enter_Cell` | High | Yes |
| The row comparison is by X column from game west, strict `<`, independent of Y once the same-building candidate cell gate passed. | helper reads `cell+0x24`, origin via vtable `+0x1B8`, compare at `0x00458A51..0x00458A72`; INI comments say "counts from game west" | High | Yes |
| Empty `NATBNK` with `NumberImpassableRows=0` skips every same-foundation cell; occupied `NATBNK` remains a normal blocker because the Bunker `+0x2E4` fast-true fires before row math. | helper `0x00458A30..0x00458A48`; prior writer `0x00459301`; `rulesmd.ini:13732,13751`; `artmd.ini:5022` | High | Conditional: occupancy state |

## 4. INI Keys

| Section | Key(s) | Stock YR value | Same-building cells affected | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `[GADEPT]` | `UnitRepair`, `NumberImpassableRows` | `yes`, `1` | 3x3 foundation: west column remains; two east columns are skipped | `rulesmd.ini:11895,11913`; `artmd.ini:3838..3842` | Yes |
| `[NADEPT]` | `UnitRepair`, `NumberImpassableRows`, `NumberOfDocks` | `yes`, `1`, `1` | 4x3 foundation: west column remains; three east columns are skipped | `rulesmd.ini:12683,12699,12701`; `artmd.ini:12010..12015` | Yes |
| `[YADEPT]` | `UnitRepair`, `NumberImpassableRows`, `Image` | `yes`, `1`, `GADEPT` | Uses `GADEPT` image/art in stock INI; expected 3x3 foundation: west column remains; two east columns are skipped | `rulesmd.ini:13424..13427,13438,13456`; `artmd.ini:3838..3842`; no `[YADEPT]` art section found by scoped scan | Conditional: type is stock but TechLevel 15 |
| `[CAOUTP]` | `UnitRepair`, `NumberImpassableRows`, `NumberOfDocks` | `yes`, `3`, `1` | 4x3 foundation: west three columns remain; eastmost column is skipped | `rulesmd.ini:13870,13886,13899,13901`; `artmd.ini:3875..3880` | Yes, when the tech outpost is present on a map |
| `[NATBNK]` | `Bunker`, `NumberImpassableRows`, `NumberOfDocks` | `yes`, `0`, `1` | 2x2 foundation: empty skips both columns; occupied blocks both columns via fast-true | `rulesmd.ini:13722,13732,13750,13751`; `artmd.ini:5019..5022`; helper `0x00458A30..0x00458A48` | Conditional: empty vs occupied |

Cell notation below uses origin `(ox, oy)` and stock foundation cells only:

| Building | Remain blockers when this branch reaches helper true | Skipped when helper false | Active in YR |
|---|---|---|---|
| `GADEPT` | `(ox, oy..oy+2)` | `(ox+1..ox+2, oy..oy+2)` | Yes |
| `NADEPT` | `(ox, oy..oy+2)` | `(ox+1..ox+3, oy..oy+2)` | Yes |
| `YADEPT` | `(ox, oy..oy+2)` via `Image=GADEPT` | `(ox+1..ox+2, oy..oy+2)` | Conditional: stock type not normally buildable (`TechLevel=15`) |
| `CAOUTP` | `(ox..ox+2, oy..oy+2)` | `(ox+3, oy..oy+2)` | Yes, if present/captured on a map |
| empty `NATBNK` | none from this building occupant on its 2x2 foundation | `(ox..ox+1, oy..oy+1)` | Conditional: `BuildingClass+0x2E4 == 0` |
| occupied `NATBNK` | `(ox..ox+1, oy..oy+1)` remains in normal building blocker path | none, because helper fast-true suppresses the skip | Conditional: `BuildingClass+0x2E4 != 0` |

## 5. Integration Points

This is live `UnitClass::Can_Enter_Cell` object-list behavior. The object list pointer comes from the candidate `CellClass` (`cell+0xE4` or `cell+0xE8` selected earlier in the function), and the loop advances through object `+0x30` at `0x0073FA87`.

The branch is not part of placement validation, not a one-time static path-grid transformation, and not a terrain passability flag. Static `PathGrid` may still serve as a coarse terrain/foundation layer, but this rule needs a runtime override at the same decision level as entity/object blockers: "for this candidate cell and this encountered building occupant, skip it or keep it."

## 6. Current Rust Implementation Status

| Rust area | Status | Evidence | Delta |
|---|---|---|---|
| `ObjectType::unit_repair`, `ObjectType::bunker`, `ObjectType::number_impassable_rows` | present | `src/rules/object_type.rs:644..696`, `:1071..1096` | Field comment still says "top, Y-axis"; binary says X/game-west |
| Static `PathGrid::block_building_movement_cells` | present and used at map load/rebuild | `src/app_init.rs:695..702`, `src/app_sim_tick.rs:813..814`, `src/sim/pathfinding/core.rs:1469..1484` | Too static for this callsite; cannot express candidate-specific skip or empty-vs-occupied bunker state |
| `building_movement_blocking_cells_for_state` | helper can express X-column and bunker-occupied semantics when configured | `src/sim/production/production_tech.rs:699..722` | The default wrapper activates rows globally; `bump_crush` currently calls with `number_rows_active=false` |
| Runtime entity block set | present | `src/sim/movement/bump_crush.rs:140..161` | It always treats structure cells through entity blockers with `number_rows_active=false`; no UnitRepair/Bunker branch gate equivalent |
| Bunker occupant state | present | `src/sim/game_entity.rs:264..270`, `src/sim/world/world_hash.rs:442` | Useful state exists, but static `PathGrid` cannot consume it correctly for this callsite |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Can_Enter_Cell` branch at `0x0073F76D` | verified | Ghidra decompile `0x0073F0A0`; assembly `0x0073F74B..0x0073F774` | none for this slice |
| Helper `0x00458A00` polarity needed by second callsite | verified | Ghidra decompile `0x00458A00`; xrefs only `0x0073F5A2`, `0x0073F76D` | full first callsite non-scope |
| `UnitRepair` parser field | verified | parser `0x0046090D`, store `0x00460929`; callsite read `0x0073F74B` | none |
| `Bunker` parser field | verified | parser `0x00460941`, store `0x00460954`; callsite read `0x0073F755`; helper read `0x00458A30` | none |
| `NumberImpassableRows` parser field | verified | parser `0x0046013A`, store `0x00460147`; helper reads `+0x1620` | none |
| `NATBNK` `BuildingClass+0x2E4` occupancy claim | verified-by-prior-report, spot-checked helper read | `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`; helper `0x00458A3C`; writer `0x00459301` from prior report | exact clear/eject writer remains out of scope |
| Stock repair depot / CAOUTP / NATBNK cell consequences | verified | stock INI/art lines plus helper strict X comparison | none for listed stock foundations |
| `YADEPT` image/art inheritance | touched-not-exhausted | `rulesmd.ini:13427 Image=GADEPT`; `artmd.ini` has `[GADEPT]`, no `[YADEPT]` in scoped scan | binary image-resolution path not re-traced |
| Full downstream building blocker return codes after helper true | deferred | non-scope; branch falls through to existing `Can_Enter_Cell` logic | separate full `Can_Enter_Cell` matrix if needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the target callsite live in YR? -> Yes, it is inside live `UnitClass::Can_Enter_Cell` and gated by parsed stock BuildingType flags.` (evidence: `0x0073F0A0`, `0x0073F74B..0x0073F774`, parser `0x0046090D`, `0x00460941`)
- `[RESOLVED] OQ-2 - Which flags reach the second helper callsite? -> `UnitRepair` at `+0x16A9` OR `Bunker` at `+0x16AB`.` (evidence: `0x0073F74B..0x0073F75D`)
- `[RESOLVED] OQ-3 - Does the callsite require same-building candidate cells? -> Yes, it calls `Look_up_building_in_cell(candidate_cell)` and compares the result to the current building before helper call.` (evidence: `0x0073F75F..0x0073F768`)
- `[RESOLVED] OQ-4 - What does helper false do here? -> It jumps to the next object-list occupant, skipping this building as a blocker for the candidate cell.` (evidence: `0x0073F772..0x0073F774`, loop continuation `0x0073FA87`)
- `[RESOLVED] OQ-5 - Which stock UnitRepair cells remain blockers? -> Cells with `x < origin_x + NumberImpassableRows`; for rows 1, only west column; for CAOUTP rows 3, west three columns.` (evidence: helper `0x00458A51..0x00458A72`; stock INI/art lines)
- `[RESOLVED] OQ-6 - Which stock UnitRepair cells are skipped? -> Same-building foundation cells at `x >= origin_x + NumberImpassableRows`.` (evidence: helper false branch `0x0073F774`; strict X comparison)
- `[RESOLVED] OQ-7 - Does `NATBNK` use the same row math? -> Empty yes (`rows=0`, so all foundation cells skip); occupied no, because Bunker `+0x2E4` fast-true returns before row math.` (evidence: helper `0x00458A30..0x00458A48`; `rulesmd.ini:13751`; prior writer `0x00459301`)
- `[RESOLVED] OQ-8 - Should this be implemented as static `PathGrid` blocking? -> No, the binary behavior is candidate-cell/object-list runtime skip/keep logic in `Can_Enter_Cell`.` (evidence: callsite branch and `0x0073FA87` loop behavior)
- `[RESOLVED] OQ-9 - Does current Rust have fields needed for a bunker occupancy check? -> Yes, `GameEntity::bunker_occupant` exists and is hashed.` (evidence: `src/sim/game_entity.rs:264..270`; `src/sim/world/world_hash.rs:442`)
- `[RESOLVED] OQ-10 - Is current Rust already activating the second-callsite row logic at runtime? -> Not in the scanned entity blocker path; `bump_crush` passes `number_rows_active=false`.` (evidence: `src/sim/movement/bump_crush.rs:153..161`)
- `[DEFERRED] OQ-11 - What exact functions clear `BuildingClass+0x2E4` during bunker exit/destruction/sell?` (category: out-of-scope; reason: the occupied/non-occupied branch is already established by prior writer and helper read; next-step-if-pursued: run a bunker lifecycle clear-writer slice)
- `[DEFERRED] OQ-12 - What are all final return codes after helper true for every ownership/weapon/crush combination?` (category: out-of-scope; reason: the target only needs skip-vs-keep for the second row callsite; next-step-if-pursued: full `UnitClass::Can_Enter_Cell` building-blocker matrix)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| For `UnitRepair=yes` same-building cells, only X columns with `candidate_x < origin_x + NumberImpassableRows` remain building blockers; columns at or east of the limit skip that building occupant. | `0x0073F74B..0x0073F774`; helper `0x00458A51..0x00458A72`; stock depot/CAOUTP INI/art lines | missing/incomplete: runtime entity blockers do not activate this branch; static grid applies rows too broadly | `src/sim/movement/bump_crush.rs`, movement cell-entry/A* blocker construction, any future `can_enter_cell` equivalent | Add a runtime UnitRepair-gated same-building occupant skip/keep decision; `GADEPT/YADEPT` block west column only, `NADEPT` west column only, `CAOUTP` west three columns | Test proposal: `unitrepair_second_callsite_skips_depot_east_columns_but_blocks_west_column` | Do not implement as a permanent PathGrid carve-out; helper false skips one building occupant only |
| Empty `NATBNK` (`Bunker=yes`, `NumberImpassableRows=0`, no contained unit) skips the bunker occupant on all same-foundation cells. | `0x0073F755..0x0073F774`; helper row math `0x00458A51..0x00458A72`; `rulesmd.ini:13732,13751`; `artmd.ini:5022` | partial state exists (`bunker_occupant`), but entity blocker path calls row logic with `number_rows_active=false`; static grid cannot reflect empty/occupied | `src/sim/game_entity.rs` state consumers; `src/sim/movement/bump_crush.rs`; pathfinding/cell-entry layer | When a stock bunker is empty, same-building 2x2 foundation cells should not be blocked by that bunker occupant via this branch | Test proposal: `empty_natbnk_second_callsite_skips_all_foundation_cells` | Do not treat `NumberImpassableRows=0` as "all impassable"; in helper it means false for ordinary foundation X values |
| Occupied `NATBNK` (`BuildingClass+0x2E4 != 0`) returns true before row math and remains in the normal building blocker path on all 2x2 foundation cells. | helper `0x00458A30..0x00458A48`; prior verified writer `0x00459301`; `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md` | state field exists, but static PathGrid and current entity block usage do not model the live fast-true distinction | `src/sim/game_entity.rs::bunker_occupant` consumers; `src/sim/movement/bump_crush.rs`; bunker entry/exit lifecycle integration | Occupying a bunker must suppress the empty-bunker skip; the building remains a blocker/cost participant like other buildings | Test proposal: `occupied_natbnk_second_callsite_blocks_foundation_cells` | Do not base occupied state on generic passengers unless intentionally mapped to the verified bunker contained-unit pointer semantics |

### Stale Docs / Follow-up Docs

- No standalone research-doc replacement is required by this slice.
- Source-comment replacement needed when code edits are allowed: `src/rules/object_type.rs:692` should say `Number of same-building west-side X columns that remain impassable for the live NumberImpassableRows helper`, replacing the current top/Y-axis wording.

## 10. Negative Facts / Do Not Do

| Negative fact | Evidence | Active in YR |
|---|---|---|
| Do not put this solely in static `PathGrid`; the binary decision is made inside live `UnitClass::Can_Enter_Cell` while iterating object-list occupants. | `0x0073F772..0x0073F774`, `0x0073FA87` | Yes |
| Do not apply `NumberImpassableRows` to all buildings globally; the helper has only two direct xrefs and this one is gated by `UnitRepair`/`Bunker`. | Ghidra xrefs to `0x00458A00`: only `0x0073F5A2`, `0x0073F76D`; second gate `0x0073F74B..0x0073F75D` | Yes |
| Do not interpret "Rows" as Y-axis/top rows; the helper compares candidate X against origin X plus the configured value. | `0x00458A51..0x00458A72`; INI comment says "counts from game west" | Yes |
| Do not treat helper true as an immediate hard-block return; true only prevents the skip and falls through to normal building logic. | fall-through `0x0073F77A` | Yes |
| Do not treat occupied and empty `NATBNK` the same; occupied bunkers fast-true via `BuildingClass+0x2E4`, empty bunkers use `rows=0` and skip. | helper `0x00458A30..0x00458A48`; prior writer `0x00459301`; `rulesmd.ini:13751` | Conditional |

## 11. Remaining Uncertainty

- Exact `BuildingClass+0x2E4` clear/eject/destruction writers are not re-traced here; the prior report already verifies the install writer and helper read needed for occupied-vs-empty `NATBNK`.
- `YADEPT` art inheritance is inferred from stock `Image=GADEPT` plus absence of a scoped `[YADEPT]` art section; the binary image-resolution path was not re-traced.
- Full final return-code outcomes after helper true are outside this slice; this report only claims skip-vs-keep at the second helper callsite.

## Sources

- Ghidra: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`; focused assembly `0x0073F74B..0x0073F774`, loop continuation `0x0073FA87`.
- Ghidra: helper `0x00458A00`; direct xrefs `0x0073F5A2`, `0x0073F76D`.
- Ghidra: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; `NumberImpassableRows` read/store `0x0046013A/0x00460147`; `UnitRepair` read/store `0x0046090D/0x00460929`; `Bunker` read/store `0x00460941/0x00460954`.
- Prior reports: `NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`.
- INI/art: `ini/rulesmd.ini` scoped sections `[GADEPT]`, `[NADEPT]`, `[YADEPT]`, `[CAOUTP]`, `[NATBNK]`; `ini/artmd.ini` `[GADEPT]`, `[NADEPT]`, `[CAOUTP]`, `[NATBNK]`.
- Rust scan only: `src/rules/object_type.rs`, `src/sim/pathfinding/core.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`, `src/sim/production/production_tech.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_hash.rs`.
