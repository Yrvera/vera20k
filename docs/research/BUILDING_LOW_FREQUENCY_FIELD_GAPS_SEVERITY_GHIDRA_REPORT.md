# Building Low-Frequency Universal Field Gaps Severity - Ghidra Research Report

**Address(es):** `0x0044F820`, `0x0045FE50`, `0x004C9C70`, `0x0045E880`, `0x0043B740`, `0x00442C40`, `0x005F3900`, `0x004FB0E0`, `0x00440580`, `0x00449A50`, `0x00445F80`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** severity sizing for already-identified small universal building gaps: invalid `[Structures]` owner rows, sparse `AddOccupy1..8` / `RemoveOccupy1..8` parsing, factory-created building HP timing, and closely adjacent low-frequency field gaps from the three parent reports.  
**Non-Scope:** full placement/pathfinding footprint mismatch, selection/rendering parity, and implementing fixes.  
**Confidence:** High for invalid-owner skip, Add/Remove parse cardinality, stock INI sparsity audit, and factory-created building initial HP; Medium for current Rust comparison.  
**Active in YR:** Conditional overall. The binary paths are live in YR, but several discrepancies only trigger on malformed/custom content or currently unobserved lifecycle windows.

## 1. Overview

This report does not reopen the larger building-foundation mismatch. It sizes the smaller universal-field gaps that came out of the owner, strength/armor/sight, and foundation/occupy reports.

The result is mostly low severity: invalid owner rows require malformed/custom maps and sparse Add/Remove numbering is parsed differently but does not occur in audited stock `art.ini`/`artmd.ini`. The formerly unresolved factory-created current-HP question is now closed: `BuildingClass::Constructor` assigns the type and then calls `BuildingClass::Init_Managers @ 0x00442C40`, which copies `Type.Strength` to both current and visual/estimated HP before the factory receives the object.

## 2. Discrepancy Severity Table

| Discrepancy | Active in YR | Stock YR exposure | Current Rust behavior | Severity | Player-visible reasoning |
|---|---|---|---|---|---|
| Invalid `[Structures]` owner rows skipped by gamemd | Conditional: active map-load path, only when owner lookup fails | Custom/malformed maps; retail map audit found no nonstandard invalid owner in the simple loose-map sample, and `Neutral`/`Special` are standard house types in `rulesmd.ini` | Rust parses the row and interns any owner string, then only skips owned-count increment if no `HouseState` exists | Low | A bad map can spawn extra buildings in Rust that gamemd would omit, but standard content should not hit this except if map/session house creation differs |
| Nonzero `g_GameMode` local-player `[Structures]` gate | Conditional: `0x0044F820` proceeds only when `g_GameMode == 0 || house != g_PlayerPtr` | Unknown for normal skirmish/campaign loading semantics; parent report flagged it but did not map modes | No equivalent found in map spawn path | Unknown | Could suppress player-owned preplaced structures in some mode, but session-mode mapping is outside this sizing pass |
| Sparse `AddOccupyN` / `RemoveOccupyN` keys | Yes for parser; effect conditional on `CanHideThings` | No stock hit in audited `ini/artmd.ini` or `ini/art.ini`: 200 YR and 125 base sections with add/remove keys had zero gaps before their max index | Rust stops at first missing key, so `AddOccupy1` + `AddOccupy3` would drop slot 3 | Low | Only malformed/custom art can differ; stock YR numbering is dense |
| Missing/invalid individual Add/Remove value defaults | Yes for parser; effect conditional on `CanHideThings` | Stock entries are ordinary dense pairs in audited files | Rust silently ignores malformed pairs and stops only on absent key; binary stores sentinel `(0xFFFF,0xFFFF)` for each missing/malformed slot independently | Low | Custom malformed values can leave later valid slots active in gamemd but not Rust; stock files do not rely on this |
| `CanHideThings` gate absent from Rust hidden-occupy behavior | Conditional: default true, binary gates hidden height/add/remove writes | Low for this narrow add/remove gap: no audited stock section with Add/Remove also had `CanHideThings=false` | Current search found no parsed/effective Rust `CanHideThings`/`OccupyHeight` field | Low within this narrow gap; larger footprint mismatch is out of scope | For add/remove specifically stock does not combine modifiers with false gate; custom art could apply modifiers in Rust when gamemd would skip |
| Factory-created building current HP timing | Yes: production creates a `BuildingClass` object before placement | Stock production path is active for every constructed building | Rust stores only a ready type ID, then spawns at placement with current=max=`Strength` | Match for initial HP | gamemd's inherited `ObjectClass` constructor first writes `0xFF`, but `BuildingClass::Init_Managers` overwrites both HP fields with `Type.Strength` before construction returns; ordinary undamaged player-built buildings therefore enter placement at full HP in both engines |
| Scenario/map health token edge cases | Yes: map `[Structures]` path | Stock maps commonly use 256/full health; malformed over-256 is clamped by gamemd | Rust clamps parse to 256 and scales by `Strength` | Low | Parent report found broad match for normal 0..256 health; over-256 custom rows cap similarly |
| Same-tick owner index freshness after spawn | Rust-side only | No direct gamemd discrepancy proven | Rust `EntityStore` owner index rebuild is tick-based, not insert-time; owned counts update immediately if house exists | Low/Unknown | Only visible if a same-tick Rust system queries owner index before rebuild; not a gamemd field mismatch by itself |

## 3. Load-Bearing Verified Facts

1. `BuildingClass::ReadFromINI @ 0x0044F820` skips a `[Structures]` row before allocation if `HouseClass::FindByName` returns `-1` or the resolved `g_HouseClass_Array[index]` pointer is null. Active in YR: Conditional, standard map load path with valid-house gate.
2. `0x0044F820` also gates rows with `g_GameMode == 0 || resolved_house != g_PlayerPtr`. Active in YR: Conditional; exact stock mode mapping remains deferred.
3. `BuildingTypeClass_ReadINI_Water @ 0x00461425..0x004614E8` formats and reads exactly numbered `AddOccupy%d` and `RemoveOccupy%d` slots for indices 1..8; each slot begins with sentinel `(0xFFFF,0xFFFF)` before the INI pair read. Active in YR: Yes for parsing; runtime effect conditional on `CanHideThings`.
4. The same parser reads `CanHideThings` at `0x0046140F..0x0046141F` into `BuildingTypeClass+0x1766`; parent report verified constructor default true and hidden occupancy writers gate on that byte. Active in YR: Conditional; default true.
5. `FactoryClass::StartProduction @ 0x004C9C70` creates a production object through vtable `+0x8C`; the building target `0x0045E880` allocates `0x720` and calls `BuildingClass::Constructor @ 0x0043B740`. `ObjectClass::Constructor @ 0x005F3900` initially writes `0xFF` to `Health +0x6C` and visual/estimated health `+0x70`, but this is only the inherited-constructor default. After assigning `Type` at `+0x520`, `BuildingClass::Constructor` calls `BuildingClass::Init_Managers @ 0x00442C40`; its instructions at `0x00442C7B` and `0x00442C7E` copy `Type+0xA0` into `+0x6C` and `+0x70`. `FactoryClass::StartProduction` therefore receives a full-HP building. `BuildingClass::Mission_Construction @ 0x00449A50`, `GrandOpening`, `Unlimbo`, and `OnConstructionComplete` do not replace that ordinary undamaged value. Active in YR: Yes. Evidence: active `gamemd.exe` Ghidra `batch_decompile(0x0043B740,0x00442C40,0x005F3900,0x00449A50)` and function-scoped `search_instructions` for `+0x6C/+0x70`, 2026-07-25.

## 4. Stock Data Audit

| Audit | Result | Evidence | Active in YR impact |
|---|---:|---|---|
| `ini/artmd.ini` sections with any Add/Remove key | 200 | PowerShell parser over `ini/artmd.ini` | Zero sparse numbering cases before max index |
| `ini/art.ini` sections with any Add/Remove key | 125 | PowerShell parser over `ini/art.ini` | Zero sparse numbering cases before max index |
| Add/Remove sections with `CanHideThings=false` | 0 in both audited files | PowerShell parser over `ini/artmd.ini` and `ini/art.ini` | Missing Rust gate is not exposed by stock Add/Remove data |
| Loose retail map files in local install with `[Structures]` | 51 of 54 loose `.mmx/.yro/.map` files matched by simple raw-text scanner | Local install scan | Only `Neutral` was seen as a "missing from [Houses]" owner in 5 files with no parsed `[Houses]`; `Neutral` is a standard house type in `rulesmd.ini`, so this is not evidence of malformed stock invalid owners |

The map audit is a coarse loose-file scan, not an archive extraction of every MIX-packed map. It is enough to size the invalid-owner discrepancy as custom/malformed-content risk rather than a proven stock-map bug.

## 5. Current Rust Comparison

| Area | Rust evidence | Severity note |
|---|---|---|
| Map structures parse owner blindly | `src/map/entities.rs` parses token 0 into `MapEntity.owner`; `src/sim/world/world_spawn.rs` interns it before spawn | Low custom-map extra-spawn risk |
| Missing house does not prevent entity spawn | `increment_owned_count` only updates if a `HouseState` exists; spawn itself has already inserted the entity | Low unless malformed map relies on gamemd skipping |
| Add/Remove parser stops early | `src/rules/art_data.rs` loops `1..=8` but `break`s on first missing key for both Add and Remove | Low for stock; custom sparse art differs |
| Production buildings are not objects until placement | `production_queue.rs` pushes ready building type IDs; `production_placement.rs` later calls `spawn_object` | Unknown vs gamemd's pre-created factory object |
| Production placement spawns full HP | `world_spawn.rs` sets `Health { current: obj.strength.max(1), max: obj.strength.max(1) }` | Unknown/frequent if gamemd visible construction HP differs |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Invalid owner skip | verified | `0x0044F820` decompile; parent owner report | none |
| `g_GameMode` local-player structure gate | touched-not-exhausted | `0x0044F820` decompile | map/session mode mapping |
| Add/Remove independent 1..8 parser | verified | `0x00461425..0x004614E8` assembly context and parent foundation report | none |
| Add/Remove stock sparsity | verified for local INI files | PowerShell audit of `ini/artmd.ini`, `ini/art.ini` | none for those files |
| `CanHideThings` stock add/remove overlap | verified for local INI files | PowerShell audit | none for those files |
| Factory create object path | verified | `0x004C9C70`, `0x0045E880`, `0x0043B740`, `0x00442C40`, `0x005F3900` | none for initial HP |
| Production placement / unlimbo / construction mission / construction complete HP trace | verified for ordinary undamaged factory placement | `0x004FB0E0`, `0x00440580`, `0x00449A50`, `0x00447780`, `0x00445F80` | combat, repair, and scripted damage remain separate lifecycle systems |
| Rust comparison | verified read-only | files listed above | no edits made |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Are invalid owner rows active in YR? -> Conditional: active map load path skips invalid owners; requires invalid/missing house name.` Evidence: `0x0044F820`.
- `[RESOLVED] OQ-2 - Does stock art use sparse Add/Remove numbering? -> No in audited local `artmd.ini`/`art.ini`.` Evidence: 200 YR and 125 base sections with zero sparse cases.
- `[RESOLVED] OQ-3 - Does binary parse sparse Add/Remove independently? -> Yes, fixed 1..8 loop with sentinel per slot.` Evidence: `0x00461425..0x004614E8`.
- `[RESOLVED] OQ-4 - Does stock Add/Remove combine with `CanHideThings=false`? -> No in audited local art files.` Evidence: local INI audit.
- `[RESOLVED] OQ-5 - What exact HP does an ordinary undamaged factory-created building carry into placement/build-up? -> Full `Type.Strength`; `BuildingClass::Init_Managers` overwrites the inherited `0xFF` defaults at `+0x6C/+0x70` before constructor return.` Evidence: active `gamemd.exe` `BuildingClass::Constructor @ 0x0043B740` and `BuildingClass::Init_Managers @ 0x00442C40`, especially `0x00442C7B/0x00442C7E`; construction-mission path `0x00449A50` has no HP write.

## Sources

- Ghidra decompilation/assembly: `0x0044F820`, `0x0045FE50`, `0x0046140F..0x00461541`, `0x004C9C70`, `0x0045E880`, `0x0043B740`, `0x00442C40`, `0x006F2B40`, `0x005F3900`, `0x005F5C60`, `0x004FB0E0`, `0x00440580`, `0x00449A50`, `0x00447780`, `0x00445F80`.
- Parent reports: `BUILDING_OWNER_HOUSE_BINDING_PARITY_GHIDRA_REPORT.md`, `BUILDING_STRENGTH_ARMOR_SIGHT_INIT_PARITY_GHIDRA_REPORT.md`, `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`.
- INI files: `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust files read-only: `src/map/entities.rs`, `src/map/houses.rs`, `src/rules/art_data.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_queue.rs`, `src/sim/production/production_placement.rs`.
