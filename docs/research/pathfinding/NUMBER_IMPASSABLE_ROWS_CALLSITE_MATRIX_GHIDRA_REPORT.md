# NumberImpassableRows Callsite Matrix - Ghidra Research Report

**Address(es):** `0x00458A00` helper, `0x0073F0A0` `UnitClass::Can_Enter_Cell`, `0x0046013A` INI read site  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** direct callers and branch context for helper `0x00458A00`, plus stock YR `NumberImpassableRows` users relevant to ordinary ground movement.  
**Non-Scope:** full `UnitClass::Can_Enter_Cell`, full building placement/occupancy system, runtime debugger validation, unrelated hidden-occupancy readers.  
**Confidence:** High for helper semantics and direct xref matrix; Medium for "likely active" stock-building classification where activation depends on radio/contact/dock state.  
**Active in YR:** Conditional. The helper is parsed and called by live YR unit passability, but it only affects cells when the encountered building reaches one of two gated `UnitClass::Can_Enter_Cell` branches.

## 1. Overview

`NumberImpassableRows` is not a standalone terrain/grid prepass in `gamemd.exe`. It is read into `BuildingTypeClass+0x1620`, then consumed by helper `0x00458A00` from exactly two direct callsites, both inside `UnitClass::Can_Enter_Cell`.

The helper answers whether the candidate cell is still in the building's impassable west-side row band. A false result makes the two callers skip the current building occupant and continue scanning the cell object list; a true result lets the normal building blocker handling continue.

## 2. Helper Semantics

| Fact | Active in YR | Evidence | Confidence |
|---|---|---|---|
| Direct xrefs to `0x00458A00` are only `0x0073F5A2` and `0x0073F76D`, both in `UnitClass::Can_Enter_Cell`. | Yes, conditional on the UnitClass object-list branch reaching those sites. | Ghidra `get_function_xrefs 0x00458A00`. | High |
| Helper arguments are `ECX = building object` and one stack argument `candidate CellClass*`; both callers use `ECX=ESI` and `PUSH EDI`. | Yes. This is the live calling convention for both sites. | Assembly `0x0073F59F..0x0073F5A2`, `0x0073F76A..0x0073F76D`; helper prologue `0x00458A02..0x00458A0A`. | High |
| Helper first checks `Look_up_building_in_cell(candidate_cell)` and returns false if the first ground building in that cell is not the same building object. | Yes. Prevents row logic from applying to unrelated building occupants. | `0x00458A08` call to `0x0047C520`, compare at `0x00458A0F`. | High |
| If `BuildingType+0x1620 == -1`, helper returns true. `-1` is therefore "all same-building cells remain impassable" for these branches. | Yes. This is the constructor/default sentinel used unless INI overrides it. | `0x00458A20..0x00458A2D`; default in `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`; field CSV. | High |
| If `BuildingType+0x16AB` is set and building instance `+0x2E4` is nonzero, helper returns true before applying row math. | Conditional. `+0x16AB` is read from INI key `Bunker`; instance `+0x2E4` state was not decoded in this slice. | `0x00458A30..0x00458A48`; `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads `Bunker` into `+0x16AB`. | Medium |
| Otherwise helper compares candidate cell X to `building_origin_x + NumberImpassableRows` and returns true only when candidate X is strictly less than that limit. | Yes, conditional on caller reaching helper. This is the "counts from game west" rule. | `0x00458A51` reads `cell+0x24`, `vtable+0x1B8` returns building origin coord, `0x00458A67..0x00458A72` adds rows and uses `SETG`. | High |
| A configured value `0` makes the strict row comparison false for ordinary same-foundation cells at or east of origin, unless the Bunker fast-true state above fires. | Conditional. Relevant to stock `NATBNK`; exact occupied-bunker state is outside this slot. | Helper comparison at `0x00458A67..0x00458A72`; `rulesmd.ini:13751`. | Medium |

## 3. Callsite Matrix

| Callsite | Branch context | Arguments | Gate before call | Effect when helper false | Active in YR |
|---|---|---|---|---|---|
| `0x0073F5A2` | Early object-list exception for a building already present in the moving unit's radio/contact vector. | `ECX=ESI` current object; `PUSH EDI` candidate cell. | Object pointer is in mover's `DynamicVectorClass` (`0x0065AD50`), object `WhatAmI()==6` building, and the object flag bit used at `[ESI+0x14] & 1` allowed a non-null Contains argument. | Jumps to next object (`0x0073FA87`), so the building does not block this candidate cell. | Conditional. This is the branch matching the INI comment's "RadioContact/Enter means I can drive on you assumption"; active for dock/enter/exit style contact states, not every random building obstacle. Evidence: `0x0073F57C..0x0073F5A9`, `0x0065AD50`. |
| `0x0073F76D` | Later default building branch for UnitRepair/Bunker-style buildings before invisible/gate/laser/bib/default blocker handling. | `ECX=ESI` current object; `PUSH EDI` candidate cell. | BuildingType `+0x16A9` (`UnitRepair`) OR `+0x16AB` (`Bunker`) is set; `Look_up_building_in_cell(candidate_cell) == current building`. | Jumps to next object (`0x0073FA87`), so the building does not block this candidate cell. | Conditional. Active for repair depots and bunkers when their building object is the primary building in the candidate cell. Evidence: `0x0073F74B..0x0073F774`; INI read offsets in `0x0045FE50`. |

No `InfantryClass::Can_Enter_Cell`, `BuildingClass` passability wrapper, placement validator, or locomotor COM passability function directly calls `0x00458A00` in this binary xref set. That is a negative finding limited to direct xrefs; indirect semantic equivalents were not searched as code clones.

## 4. INI Keys And Stock Structures

`NumberImpassableRows` is read by `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, at `0x0046013A` pushing string `0x0081AD6C`, and stored to `BuildingTypeClass+0x1620` at `0x00460147`. YR `rulesmd.ini` overrides base RA2.

| Building(s) | rulesmd value | Related stock flags | Likely active on ordinary ground movement | Active in YR |
|---|---:|---|---|---|
| `GAREFN`, `NAREFN` | 3 | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `Bib=yes` | Yes, conditional through radio/dock/contact branch for harvesters and refinery entry/exit cells. | Conditional; evidence `rulesmd.ini:11726..11764`, `12519..12524`, callsite `0x0073F5A2`. |
| `GAWEAP`, `NAWEAP`, `YAWEAP` | 1 | `WeaponsFactory=yes`, `Factory=UnitType`, `Bib=yes`, `ExitCoord=512,256,0` | Yes, conditional through factory unlimbo/exit contact states; INI comment explicitly cites tanks/hover getting stuck. | Conditional; evidence `rulesmd.ini:11775..11804`, `12565..12598`, `13309..13339`, callsite `0x0073F5A2`. |
| `GADEPT`, `NADEPT`, `YADEPT` | 1 | `UnitRepair=yes` | Yes for ordinary ground vehicles entering/using repair depots; this is the cleanest stock hit for the second callsite. | Conditional; evidence `rulesmd.ini:11895..11913`, `12683..12701`, `13438..13456`, callsite `0x0073F76D` gate `+0x16A9`. |
| `GAYARD`, `NAYARD`, `YAYARD` | 3 | `Naval=yes`, `WaterBound=yes`, `WeaponsFactory=yes`, `UnitRepair=yes` | Not ordinary ground movement; active for naval/water or possibly hover/amphibious interactions, outside this slot's "ordinary ground" emphasis. | Conditional; evidence `rulesmd.ini:11850..11880`, `12638..12668`, `13388..13418`. |
| `NATBNK` | 0 | `Bunker=yes`, `NumberOfDocks=1` | Conditional/uncertain for ordinary ground vehicles: second callsite can reach Bunker buildings, but helper has a Bunker state fast-true plus row `0` otherwise false. Instance field `+0x2E4` was not decoded here. | Conditional; evidence `rulesmd.ini:13732..13751`, helper `0x00458A30..0x00458A48`, callsite `0x0073F76D` gate `+0x16AB`. |
| `CAOUTP` | 3 | `UnitRepair=yes`, civilian tech outpost | Yes where present on maps and used as a repair building; less common than player-built depots. | Conditional; evidence `rulesmd.ini:13886..13901`, callsite `0x0073F76D`. |
| `YAREFN` | commented out | `UndeploysInto=SMIN` | No configured row effect in stock YR because the key is commented. | No for `NumberImpassableRows`; evidence `rulesmd.ini:13286` is commented. |

Base RA2 `rules.ini` also has `GAREFN`, `NAREFN`, `GAYARD`, `NAYARD`, `GADEPT`, `NADEPT`, and `CAOUTP`, but YR `rulesmd.ini` is authoritative for stock YR.

## 5. Integration Points

The helper only participates after `UnitClass::Can_Enter_Cell` has selected the object list (`cell+0xE4` or `cell+0xE8`) and is iterating occupants. It is not part of the earlier bridge, tube, shroud, locomotor, or overlay wall checks.

At both callsites, false means "ignore this building occupant for this candidate cell and continue the loop." True does not immediately return a code; it falls through into normal building logic that can later return or upgrade codes such as 0, 3, 5, 7 depending on mission, ownership, garrison, gate, laser fence, weapon/crush, and allied/enemy checks.

## 6. Current Rust Implementation Status

Rust currently parses `NumberImpassableRows` (`src/rules/object_type.rs:686`, `:1078`) and passes it into movement/path blocking (`src/app_init.rs:695`, `src/app_sim_tick.rs:813`, `src/sim/movement/bump_crush.rs:151`).

Rust also has a row-filter implementation in `src/sim/production/production_tech.rs:643` and `src/sim/pathfinding/core.rs:1465`. This report did not audit whether Rust's caller timing and contact-state gates match the two binary callsites; the binary finding here is that the row limit is a conditional per-occupant skip inside `Can_Enter_Cell`, not a universal placement-time cell erasure.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00458A00` helper body | verified | Ghidra decompile and assembly `0x00458A00..0x00458A76` | Decode instance field `BuildingClass+0x2E4` if bunker behavior needs runtime fidelity. |
| Direct xrefs to helper | verified | Ghidra `get_function_xrefs 0x00458A00` | None for direct calls. |
| `0x0073F5A2` radio/contact branch | verified | Assembly `0x0073F57C..0x0073F5A9`; `0x0065AD50` Contains decompile | Exact lifecycle that inserts/removes buildings from the radio/contact vector is out of scope. |
| `0x0073F76D` UnitRepair/Bunker branch | verified | Assembly `0x0073F74B..0x0073F774`; INI read offsets in `0x0045FE50` | Bunker occupied-state field `+0x2E4` deferred. |
| INI parser read | verified | `0x0046013A` string push, `0x00460147` store to `+0x1620` | None. |
| Stock YR structure set | verified | `rulesmd.ini` line scan listed in Sources | Activation marked conditional where runtime state gates are required. |
| Ordinary ground movement likelihood | touched-not-exhausted | Binary branch gates plus INI flags | Runtime scenario traces for each building class were not run. |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - How many direct helper callsites exist? -> Two, both in UnitClass::Can_Enter_Cell.` Evidence: Ghidra xrefs to `0x00458A00`.
- `[RESOLVED] OQ-2 - What arguments do callsites pass? -> Current object in ECX and candidate cell as the single stack argument.` Evidence: `0x0073F59F..0x0073F5A2`, `0x0073F76A..0x0073F76D`.
- `[RESOLVED] OQ-3 - What does helper return for -1? -> True/all same-building cells remain impassable for these branches.` Evidence: `0x00458A20..0x00458A2D`.
- `[RESOLVED] OQ-4 - What is the row comparison? -> `candidate_cell.x < building_origin_x + NumberImpassableRows`, strict less-than.` Evidence: `0x00458A51..0x00458A72`.
- `[RESOLVED] OQ-5 - Which stock YR sections set the key? -> GAREFN/NAREFN, GAWEAP/NAWEAP/YAWEAP, GAYARD/NAYARD/YAYARD, GADEPT/NADEPT/YADEPT, NATBNK, CAOUTP; YAREFN is commented.` Evidence: `rulesmd.ini` lines listed in Sources.
- `[DEFERRED] OQ-6 - What exactly is BuildingClass+0x2E4 in the Bunker fast-true helper path?` Category: out-of-scope; reason: not needed for non-bunker row matrix, but needed for exact NATBNK behavior.
- `[DEFERRED] OQ-7 - Which functions populate the radio/contact vector used by `0x0073F5A2` for every dock/factory scenario?` Category: requires-different-system-context; reason: callsite matrix is verified, but full radio protocol lifecycle is a separate trace.

## Sources

- Ghidra: `0x00458A00`, `0x0073F0A0`, `0x0073F5A2`, `0x0073F76D`, `0x0047C520`, `0x0065AD50`, `0x0045FE50`, `0x0046013A`.
- Existing docs: `BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`, `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `BUILDINGTYPECLASS_FIELDS.csv`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan only: `src/rules/object_type.rs`, `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/bump_crush.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`.
