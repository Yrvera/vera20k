# Building Foundation / Occupy Modifiers — Ghidra Research Report

**Address(es):** `0x0045FE50`, `0x0045EC20`, `0x00441F60`, `0x005683C0`, `0x005687F0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** `Foundation=`, `AddOccupy1..8`, `RemoveOccupy1..8`, how the building type stores them, how placed buildings expose base foundation cells versus hidden occupancy modifiers, and current Rust comparison for those same surfaces.  
**Non-Scope:** full placement validator, full A* cost model, full selection/bracket rendering, and runtime debugger validation of `CellClass+0x100` consumers beyond the traced entry/exit writers.  
**Confidence:** High for parsing/storage and writer paths; Medium for the consumer map because the wider passability chain was touched, not exhausted.  
**Active in YR:** Yes; these paths are reached by `BuildingTypeClass::ReadINI`, building unlimbo/place, and `TechnoClass` cell enter/exit for standard building objects. Add/remove effects are conditional on `CanHideThings`.

## 1. Overview

`Foundation=` selects a fixed foundation enum/table entry, not a free-form shape. `AddOccupy*` and `RemoveOccupy*` are not merged into the normal foundation cell list used by `BuildingClass::Place_OccupyMap`; they separately adjust the hidden multi-cell occupancy counter written during `TechnoClass` enter/exit when the building type has `CanHideThings` enabled.

For player-visible parity, this means some systems must keep using the base foundation rectangle/list, while path/blocking hidden-occupancy behavior must apply `AddOccupy*/RemoveOccupy*`.

## 2. Class Layout / Key Offsets

| Offset | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingTypeClass+0xDFC` | pointer to foundation cell list table for selected `Foundation=` | `0x0046152C..0x00461541`; returned by `0x0045EC20` | Yes |
| `BuildingTypeClass+0xED4` | pointer to foundation exit-cell table for selected `Foundation=` | `0x00461547..0x0046156A` | Yes |
| `BuildingTypeClass+0xEF0` | `Foundation=` enum id | `0x00461225..0x00461257`; width/height readers `0x0045EC90/0x0045ECA0` | Yes |
| `BuildingTypeClass+0xEF8` | hidden occupancy height; writer uses `max(value - 1, 1)` | `0x005684FE..0x00568693`, mirrored at `0x0056892E..0x00568B0C` | Conditional: only if `CanHideThings` |
| `BuildingTypeClass+0x1624..0x1660` | eight `(x,y)` `AddOccupy%d` pairs, sentinel `0xFFFF,0xFFFF` | parse loop `0x00461425..0x00461486`; constructor `0x0045E49B` | Conditional: only if `CanHideThings` |
| `BuildingTypeClass+0x1664..0x16A0` | eight `(x,y)` `RemoveOccupy%d` pairs, sentinel `0xFFFF,0xFFFF` | parse loop `0x0046148A..0x004614D5`; constructor `0x0045E49B` | Conditional: only if `CanHideThings` |
| `BuildingTypeClass+0x1766` | `CanHideThings=` gate, default true | constructor sets `1`; read at `0x0046140F`; string `0x0081A640` | Yes, default true |
| `CellClass+0x100` | hidden occupancy counter adjusted by height/add/remove writers | `0x005685E9`, `0x0056871C`, `0x005687AA`, `0x00568A7B`, `0x00568BC8` | Conditional: only for building hidden occupancy |

## 3. Core Logic

### INI parse and precedence

`BuildingTypeClass::ReadINI @ 0x0045FE50` first runs ordinary building/rules-section reads, then switches to the art/image section at `BuildingTypeClass+0x1F8` (`0x004610DE`). The `Foundation` parse then does two reads:

1. Read `Foundation` from the art/image section with current `+0xEF0` as default, and store the result (`0x00461225..0x00461248`).
2. Read `Foundation` from the earlier object/rules section using the art result as default; if the resolver returns nonzero, overwrite `+0xEF0` (`0x0046123C..0x00461257`).

Active in YR: Yes. Evidence: this is the live `BuildingTypeClass::ReadINI` path called after `TechnoTypeClass::ReadINI`; no TS-only gate was found on these key reads.

### Add/Remove parse

The parser loops exactly eight numbered slots, using keys formatted from `AddOccupy%d` and `RemoveOccupy%d`, with index values `1..8`. Missing or malformed reads use the default pair `(0xFFFF,0xFFFF)`, not a zero offset. The constructor also initializes all eight add and remove pairs to the same sentinel.

Active in YR: Yes for parsing; effect conditional on `CanHideThings`. Evidence: `0x00461425..0x004614E8`, constructor `0x0045E49B`, strings `0x0081A634`/`0x0081A624`.

### Base foundation list

After parsing modifiers, the code sets `+0xDFC` directly to `0x0089C900 + foundation_id * 120` and `+0xED4` to `0x0089D368 + foundation_id * 120`. `BuildingTypeClass` vtable slot `+0x90` at `0x0045EC20` returns `+0xDFC` or a sentinel list if null. No `AddOccupy` or `RemoveOccupy` pair participates in that pointer calculation.

Active in YR: Yes. Evidence: `0x0046152C..0x00461541`, `0x0045EC20`.

### Normal building placement/ownership occupancy

`BuildingClass::Place_OccupyMap @ 0x00441F60` calls the building object's foundation-list vtable (`ObjectClass` path resolves to type vtable `+0x90` at `0x005F5B90`), walks each `(short dx, short dy)` until `(0x7FFF,0x7FFF)`, and marks those cells: overlay type `0xEF`, cell owner/type pointer only on the origin cell, attribute recalc, zone assignment, dirty rect update. It uses the base foundation list; no add/remove fields are read here.

Active in YR: Yes. Evidence: `0x00441F60`, `0x005F5B90`.

### Hidden multi-cell occupancy writers

`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` and `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0` are the add/remove-modifier writers. They require:

- passed object pointer non-null,
- `object->Type` reports the “multi-cell contents” capability byte at `type+0x235`,
- object kind is building (`WhatAmI()==2` in this function’s type system),
- building type `CanHideThings` at `+0x1766` is true before height/add/remove counter work.

For enter, the function first adds the object to every base foundation cell content list and recalculates cell attributes. If the object is a building and `CanHideThings` is true, it increments `CellClass+0x100` for diagonal hidden-occupancy cells derived from every base foundation cell and `max(OccupyHeight - 1, 1)`, with a 0x200-byte local duplicate mask. It then loops eight add/remove slots: each valid `AddOccupy` increments `Cell+0x100` at `origin + offset`; each valid `RemoveOccupy` decrements `Cell+0x100` if the counter is nonzero.

For exit, the function removes the object from base foundation cell content lists and reverses the same hidden occupancy effects: diagonal hidden cells decrement if nonzero, valid `AddOccupy` cells decrement if nonzero. `RemoveOccupy` does not need an exit-side increment because it only canceled an enter-side increment.

Active in YR: Conditional. Standard building objects enter these functions, and `CanHideThings` defaults true, but any type with `CanHideThings=no` skips the hidden occupancy height/add/remove block. Evidence: `0x005683C0`, `0x005687F0`, constructor default `+0x1766=1`, read at `0x0046140F`.

## 4. INI Keys

| Key | Source section | Binary behavior | Rust status |
|---|---|---|---|
| `Foundation=` | art/image section, then object/rules section can override if non-default | Fixed table enum in `+0xEF0`; art read first, nonzero rules read can override | MATCH: `ruleset.rs:1742..1760` preserves non-default rules foundation, otherwise uses art |
| `AddOccupy1..8=` | art/image section | Eight numbered `(x,y)` pairs; sentinel when absent | MATCH: parser visits all `1..8` slots independently via `continue` on missing key (`art_data.rs:1171..1187`); corrected 2026-05-29 from PARTIAL — stale line numbers and wrong early-exit claim (STALE_LINE_NUMBERS + INFERENCE_HARDENED) |
| `RemoveOccupy1..8=` | art/image section | Eight numbered `(x,y)` pairs; sentinel when absent | MATCH: same `parse_numbered_cell_offsets` helper, visits all 8 slots; corrected 2026-05-29 from PARTIAL (STALE_LINE_NUMBERS + INFERENCE_HARDENED) |
| `CanHideThings=` | rules/object section (corrected 2026-05-29: was "art/image section"; binary reads at `param_1+0x1766` using `iVar21` — the rules-section CCINIClass pointer — at ReadINI line 706 after the art-section switch at line 632; no art-section read of this key exists. In practice most types omit it from rules.ini so the constructor default of `1` (true) applies. Source: decompile_function `0x0045FE50`, line 706 — MISLEADING) | Gates hidden occupancy height/add/remove writes; default true | UNKNOWN: not in current Rust comparison scope as a parsed/effective gate |
| `OccupyHeight=` | art/image section | Hidden occupancy diagonal depth uses `max(OccupyHeight - 1, 1)` | UNKNOWN for exact hidden occupancy; Rust uses footprint blockers, not this counter |

Retail examples checked: `ini/artmd.ini:1763..1795` gives `[GAREFN] Foundation=4x3`, `CanHideThings=True`, `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1`; `[NAREFN]` has `Foundation=4x3`, `CanHideThings=true`, and eight `RemoveOccupy` pairs at `ini/artmd.ini:1706..1760`.

## 5. Integration Points

| System | Uses base foundation or modified hidden occupancy? | Evidence | Active in YR |
|---|---|---|---|
| Building placement/owner cell marking | Base foundation list only | `BuildingClass::Place_OccupyMap @ 0x00441F60`; `+0xDFC` from `0x00461541` | Yes |
| Cell content lists scanned by unit passability | Base foundation content list, then passability logic sees objects in cells | enter/exit `CellClass__AddContent/RemoveContent` in `0x005683C0/0x005687F0`; `UnitClass::Can_Enter_Cell @ 0x0073F0A0` scans cell content lists | Yes |
| Hidden occupancy/can-hide counter | Modified by height + `AddOccupy` + `RemoveOccupy` | `Cell+0x100` writes in `0x005683C0/0x005687F0` | Conditional |
| Foundation width/height geometry | Base foundation dimensions only | `0x0045EC90`, `0x0045ECA0`, `0x00458E00` | Yes |
| Foundation exit table | Base foundation enum only | `+0xED4 = 0x0089D368 + id * 120` at `0x00461547..0x0046156A` | Yes |
| Full placement/path validation | touched-not-exhausted | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` touched; full validator deferred | Yes |

## 6. Current Rust Implementation Status

| Area | Status | Evidence | Notes |
|---|---|---|---|
| Fixed `Foundation=` table | MATCH | `src/rules/foundation.rs:17`, `src/rules/object_type.rs:850..853` | Existing table behavior matches prior parser-table evidence. |
| Foundation art/rules precedence | MATCH | `src/rules/ruleset.rs:1842..1864` (corrected 2026-05-29: was line `1742..1760` — stale line numbers; logic verified current at 1842..1864. Root-cause: STALE_LINE_NUMBERS) | Preserves rules non-default over art, otherwise art supplies foundation. |
| Add/remove parsing cardinality | MATCH | `src/rules/art_data.rs:1171..1187` (corrected 2026-05-29: was `art_data.rs:388..420` — stale line numbers; prior claim "stops at first missing key" was WRONG. Current Rust `parse_numbered_cell_offsets` uses `continue` on missing keys, visiting all 8 slots independently, matching binary. Structural difference remains: binary stores all 8 slots with 0xFFFF for missing; Rust uses a Vec with only valid entries. Observable behavior is identical since 0xFFFF pairs are ignored. Root-cause: STALE_LINE_NUMBERS + INFERENCE_HARDENED on prior parse behavior) |
| Add/remove stored on object types | MATCH | `src/rules/ruleset.rs:1892..1896` (corrected 2026-05-29: was `1792..1796` — stale line numbers; actual merge verified at 1892..1896. Root-cause: STALE_LINE_NUMBERS) | Art modifiers are merged onto buildings. |
| “Actual occupied footprint” helper | MISMATCH | `src/sim/production/production_tech.rs:576..613` | Rust constructs one adjusted cell set by adding/removing modifiers from the rectangle. Binary keeps base foundation list separate and applies modifiers only to hidden occupancy counters. |
| App path grid blockers | MISMATCH | `src/sim/pathfinding/core.rs:1468..1488`, `src/app_init.rs:689..703`, `src/app_sim_tick.rs:811..815` | Rust blocks movement with adjusted footprint; binary base content lists and hidden occupancy are distinct. |
| Placement / spawn footprint checks | MISMATCH | `src/sim/world/world_spawn.rs:242..247`, `430..435` | Rust uses adjusted footprint for placement occupancy; binary `Place_OccupyMap` uses base foundation list. |
| C4 target footprint | UNKNOWN/POTENTIAL MISMATCH | `src/sim/world/world_orders.rs:575..580` | Binary C4 consumer not traced in this slot. If it uses normal building cells, Rust’s adjusted set may differ. |
| Selection/pips/brackets/minimap | MATCH for base dimensions only, UNKNOWN for per-cell hit coverage | `src/app_entity_pick.rs:347..360`, `src/app_selection_brackets.rs:197..200`, `src/app_ui_overlays.rs:95..100`, `src/render/minimap.rs:319` | These use foundation dimensions, not adjusted footprint. Exact building click hit was not checked against binary here. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Foundation=` string xrefs | verified | `0x0081A734`, xrefs `0x0046122C/0x0046123D` | none |
| Art-vs-rules `Foundation=` precedence | verified | `0x004610DE`, `0x00461225..0x00461257` | none |
| `AddOccupy%d` / `RemoveOccupy%d` parse loops | verified | `0x0081A634`, `0x0081A624`, `0x00461425..0x004614E8`; loop terminates on `iVar21 < 8` (all 8 slots always checked) — confirmed via decompile_function `0x0045FE50` lines 710-734 | none |
| Default sentinels for add/remove pairs | verified | constructor `0x0045E49B` | none |
| Base foundation pointer assignment | verified | `0x0046152C..0x00461541`, `0x0045EC20` | none |
| `Place_OccupyMap` normal cell marking | verified | `0x00441F60`, `0x005F5B90` | none for this slice |
| Hidden occupancy enter writer | verified | `0x005683C0` | exact downstream consumers of `Cell+0x100` deferred |
| Hidden occupancy exit writer | verified | `0x005687F0` | exact downstream consumers of `Cell+0x100` deferred |
| `UnitClass::Can_Enter_Cell` consumer path | touched-not-exhausted | `0x0073F0A0` | full passability chain and `Cell+0x100` reader attribution |
| Current Rust parser/merge | verified | listed Rust file lines | none |
| Current Rust placement/path consumers | touched-not-exhausted | listed Rust file lines | every consumer of `building_footprint_cells` not exhaustively classified |

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-1 — Is Foundation read from art or rules? -> Both: art/image section first, then object/rules section may override when the resolver returns nonzero.` (evidence: `0x004610DE`, `0x00461225..0x00461257`)
- `[RESOLVED] OQ-2 — Are AddOccupy/RemoveOccupy free-form or numbered? -> Numbered `%d` keys, indices 1..8.` (evidence: `0x00461425..0x004614E8`, strings `0x0081A634/0x0081A624`)
- `[RESOLVED] OQ-3 — Do modifiers change `BuildingTypeClass+0xDFC` foundation list? -> No; `+0xDFC` is assigned from `foundation_id * 120 + 0x0089C900` after parsing.` (evidence: `0x0046152C..0x00461541`)
- `[RESOLVED] OQ-4 — Does `Place_OccupyMap` use modified footprint? -> No; it walks vtable foundation list and does not read add/remove offsets.` (evidence: `0x00441F60`, `0x005F5B90`)
- `[RESOLVED] OQ-5 — Where do modifiers apply? -> Enter/exit multi-cell writers adjust `CellClass+0x100` hidden occupancy when `CanHideThings` is true.` (evidence: `0x005683C0`, `0x005687F0`)
- `[RESOLVED] OQ-6 — Is this active in stock YR? -> Yes for building types with standard enter/exit; conditional on `CanHideThings`, default true, and retail GAREFN/NAREFN set it true.` (evidence: constructor default, `0x0046140F`, `ini/artmd.ini:1752`, `ini/artmd.ini:1790`)
- `[RESOLVED] OQ-7 — Does Rust parse and merge modifiers? -> Yes, but sparse numbered keys differ because Rust breaks on first missing key.` (evidence: `src/rules/art_data.rs:388..420`, `src/rules/ruleset.rs:1792..1796`)
- `[RESOLVED] OQ-8 — Does Rust keep base foundation and hidden occupancy separate? -> No; `building_footprint_cells` produces one adjusted footprint used by multiple systems.` (evidence: `src/sim/production/production_tech.rs:576..613`)
- `[DEFERRED] OQ-9 — Which exact passability sub-branch consumes `Cell+0x100`?` (category: `requires-different-system-context`; reason: `UnitClass::Can_Enter_Cell` was touched but full dataflow through cell flags/passability helpers exceeds this narrow slot; next-step-if-pursued: trace `CellClass+0x100` readers and helper vtable `+0x1B0` in a passability-focused investigation)
- `[DEFERRED] OQ-10 — Do selection click tests ever use modified hidden occupancy?` (category: `requires-different-system-context`; reason: this slot verified placement/enter writers, not Tactical/UI selection scan paths; next-step-if-pursued: trace building click/selection from screen cell to object lookup)

## Sources

- Ghidra: `0x0045FE50`, `0x00461225..0x00461575`, `0x0045EC20`, `0x0045EC90`, `0x0045ECA0`, `0x00441F60`, `0x005F5B90`, `0x005683C0`, `0x005687F0`, `0x0073F0A0`.
- INI: `ini/artmd.ini` `[NAREFN]` lines around `1706..1760`; `[GAREFN]` lines around `1763..1795`.
- Rust: `src/rules/art_data.rs`, `src/rules/ruleset.rs`, `src/rules/foundation.rs`, `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/sim/world/world_spawn.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`, `src/sim/world/world_orders.rs`.
