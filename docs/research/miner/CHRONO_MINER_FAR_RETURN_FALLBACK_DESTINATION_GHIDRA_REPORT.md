# Chrono Miner Far Return Fallback Destination - Ghidra Research Report

**Address(es):** `0x0073E5E0` (`UnitClass__Mission_Harvest`), `0x0056DC20` (`FootClass__Find_Nearby_Passable_Cell`), `0x004DF040` (`FootClass__Find_Docking_Bay`), `0x00461506` / `0x00464938` / `0x004649B7` (`BuildingTypeClass_ReadINI_Water` field readers)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** only the `CMIN` state-2 far/fallback return destination chosen after the normal close radio dock path is not used.  
**Non-Scope:** post-arrival radio sequence, dock-pivot timing, post-unload exit, and exact rendering/audio of the teleport.  
**Confidence:** High  
**Active in YR:** Yes. `rulesmd.ini` defines `[CMIN]` with `Harvester=yes`, `Teleporter=yes`, and `Dock=NAREFN,GAREFN`; stock YR `[General] ChronoHarvTooFarDistance=50`; stock `GAREFN`/`NAREFN` have `QueueingCell=4,1`.

> **Repo-status supersession 2026-05-25:** Section 6's older note that
> current Rust still uses a hardcoded 2-cell chrono inbound threshold is stale.
> Current Rust parses `ChronoHarvTooFarDistance` into
> `MinerConfig::too_far_threshold_chrono` and uses a strict-greater 3D lepton
> distance split in `src/sim/miner/miner_system.rs`. Keep this report's binary
> fallback-destination facts, but do not use the old line-number/status note as
> current implementation evidence.

## 1. Overview

When a loaded Chrono Miner is in `Mission_Harvest` state 2 and the normal close radio path does not fire, gamemd performs a fallback dock search and assigns a destination near the refinery. The destination seed is the refinery top-left cell plus `BuildingTypeClass+0x1618/+0x161C`, which are populated from art.ini `QueueingCell=`, then `FootClass__Find_Nearby_Passable_Cell` selects the actual passable cell before `Set_Destination`.

This is not the radio `CAN_DOCK` hardcoded target `anchor+(3,1)`, and it is not the `NumberOfDocks` / `DockingOffset%d` pad array.

## 2. Key Offsets And Fields

| Offset | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `UnitClass+0x0BC` (`param_1[0x2F]`) | unit | `Mission_Harvest` internal state; state 2 is return-to-refinery | `0x0073E5E0` switch | Yes; active harvester mission state |
| `UnitClass+0x6C4` (`param_1[0x1B1]`) | unit | `UnitTypeClass*` | state-2 reads type flags and Dock list | Yes |
| `UnitTypeClass+0x3E8` | unit type | dock-list struct used by `Find_Docking_Bay` | `0x0073EC39` adds `0x3E8` before vtable `+0x528` call | Yes; `[CMIN] Dock=NAREFN,GAREFN` |
| `TechnoTypeClass+0xCD4` | unit type | `Teleporter=yes` gate (`cVar1`) | `0x0073E5E0` state-2 teleporter branch | Yes; `[CMIN] Teleporter=yes` |
| `RulesClass+0xD7C` | rules | `ChronoHarvTooFarDistance` in cells, multiplied by `0x100` leptons | `0x0073EE40` read, `0x0073EE46` shift-left 8 | Yes; `rulesmd.ini:294=50` |
| `BuildingClass+0x09C/+0x0A0/+0x0A4` | building | world coordinate; top-left cell derived by signed lepton-to-cell conversion | `0x0073ECDF` onward | Yes |
| `BuildingClass+0x520` | building | `BuildingTypeClass*` | `0x0073ECE5` | Yes |
| `BuildingTypeClass+0x1618/+0x161C` | building type | `QueueingCell` X/Y cell offsets | `0x00461520` / `0x00461523` writes after string `QueueingCell` | Yes; `artmd.ini:1716`, `1773` |
| `BuildingTypeClass+0x1780` | building type | `NumberOfDocks`, not used in this destination branch | `0x00464938` reader only; no state-2 read | Yes as data, No for this branch |
| `BuildingTypeClass+0x1788` | building type | `DockingOffset%d` array pointer, not used in this destination branch | `0x004649B7` reader only; no state-2 read | Yes for other buildings, No for this branch |

## 3. Core Logic

### State-2 normal path gate

`UnitClass__Mission_Harvest @ 0x0073E5E0` first calls the dock search through vtable slot `+0x528` with arg3 `0`:

```
dock = Find_Docking_Bay(UnitType + 0x3E8, 0, 0)
```

For `CMIN`, the code computes 3D lepton distance from the unit to the refinery object coordinate and compares it to `RulesClass+0xD7C * 0x100`. If the distance is within threshold, it sends radio message `2` to the refinery and, when accepted, sets state `3`. That close path does not read `QueueingCell`.

**Active in YR:** Yes. Evidence: `0x0073EE40` reads `RulesClass+0xD7C`; `0x0073EE55` pushes radio message `2`; `[CMIN] Teleporter=yes` in `rulesmd.ini:7396`.

### Fallback trigger

If the close path is not taken, the code increments `g_MapEditorMode`, calls the same vtable dock search with arg3 `1`, then decrements `g_MapEditorMode`:

```
dock = Find_Docking_Bay(UnitType + 0x3E8, 0, 1)
```

For `CMIN`, any non-null fallback dock proceeds into destination selection because the branch condition is `(distance > 0x300 || TeleporterFlag != 0)`. Since `CMIN` has `Teleporter=yes`, the `0x300` condition is irrelevant for CMIN after a fallback dock is found.

**Active in YR:** Yes. Evidence: `0x0073EC1F` reads `g_MapEditorMode`; `0x0073EC25` pushes arg3 `1`; `0x0073ECD0` compares distance with `0x300`; `0x0073ECD7` tests the teleporter flag.

### Destination formula

For the selected fallback dock building:

```
anchor_x = signed_floor_div_256(building.coord_x)
anchor_y = signed_floor_div_256(building.coord_y)
seed_x = anchor_x + *(short *)(building_type + 0x1618)
seed_y = anchor_y + *(short *)(building_type + 0x161C)
actual_cell = Find_Nearby_Passable_Cell(seed, size=2, zone=-1, flags...)
if actual_cell == NullCell:
    Set_Destination(NULL, 1)
else:
    Set_Destination(MapClass::Get_CellClass(actual_cell), 1)
```

For stock `GAREFN`/`NAREFN`, `QueueingCell=4,1`, so the seed is `building anchor + (4,1)`. The actual destination can be the seed itself or another passable cell selected by `Find_Nearby_Passable_Cell`.

**Active in YR:** Yes. Evidence: `0x0073ED25` reads `+0x1618`; `0x0073ED34` reads `+0x161C`; `0x0073ED75` calls `0x0056DC20`; `0x0073EDB5` calls vtable `+0x480` with the cell object.

### What this excludes

The far/fallback destination does not use `anchor+(3,1)`. That hardcoded coordinate belongs to the refinery radio `CAN_DOCK` path and is a different stage.

The far/fallback destination does not use `NumberOfDocks` or `DockingOffset%d`. `BuildingTypeClass_ReadINI_Water` reads those fields at `+0x1780/+0x1788`, but `Mission_Harvest` state 2 does not read them.

**Active in YR:** Yes for the exclusion in this branch. Evidence: no `+0x1780/+0x1788` read in the `0x0073EC1F`-`0x0073EDB5` fallback block; `+0x1618/+0x161C` are the fields read.

## 4. INI Keys

| INI path | Value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `rulesmd.ini [General] ChronoHarvTooFarDistance` | `50` | threshold for close radio path; farther CMINs fall through to fallback | `rulesmd.ini:294`; binary read at `0x0073EE40` | Yes |
| `rulesmd.ini [CMIN] Dock` | `NAREFN,GAREFN` | populates dock-list used at `UnitType+0x3E8` | `rulesmd.ini:7361`; state-2 `+0x3E8` search | Yes |
| `rulesmd.ini [CMIN] Harvester` | `yes` | enables harvester mission path | `rulesmd.ini:7364` | Yes |
| `rulesmd.ini [CMIN] Teleporter` | `yes` | selects chrono threshold and makes fallback destination branch unconditional once a dock is found | `rulesmd.ini:7396`; binary teleporter test | Yes |
| `artmd.ini [GAREFN] QueueingCell` | `4,1` | seed offset for fallback destination | `artmd.ini:1773`; binary field writes at `0x00461520`/`0x00461523` | Yes |
| `artmd.ini [NAREFN] QueueingCell` | `4,1` | same for Soviet refinery | `artmd.ini:1716`; binary field writes at `0x00461520`/`0x00461523` | Yes |
| `rulesmd.ini [GAREFN]/[NAREFN] NumberOfDocks` | `1` | not used by this branch | `rulesmd.ini:11729`, `12521`; field reader `0x00464938` | No for this branch |

## 5. Integration Points

`FootClass__Find_Docking_Bay @ 0x004DF040` iterates the unit type's dock-list and calls the per-type evaluator through vtable slot `+0x52C`. Prior reports verified arg3 `1` skips the reservation/contact-list check but still requires the building to be a valid dock candidate. In this slice, that means the fallback can choose a refinery even when the normal reserved/free close path is unavailable.

`FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` is called after the `QueueingCell` seed is built. It collects passable candidates around the seed and returns one packed cell, or `NullCell` if none is found. State 2 immediately converts the returned packed cell to a `CellClass*` and calls vtable `+0x480`.

`TechnoClass__Set_Destination @ 0x00741970` / `FootClass__Set_Destination_Internal @ 0x004D94B0` then assign the destination object. For a teleporter targeting an empty/passable cell, the active teleport locomotor is what produces the inbound warp, as documented in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`.

**Active in YR:** Yes. Evidence: all three calls are on the standard `CMIN` harvester mission path; no TS-only flag gates were found in this block.

## 6. Current Rust Implementation Status

Rust now has separate helpers for the two coordinate concepts:

- `src/sim/miner/miner_dock_sequence.rs:70` computes `refinery_queue_cell` from `QueueingCell`.
- `src/sim/miner/miner_dock_sequence.rs:88` computes the radio `CAN_DOCK` queue cell as `anchor+(3,1)`.
- `src/sim/miner/miner_system.rs` computes the chrono far-return staging cell from `QueueingCell` and passable-cell search.

Repo-status update 2026-05-25: the prior "2-cell chrono inbound warp threshold" mismatch is fixed in current Rust. Current Rust reads `ChronoHarvTooFarDistance` and performs the close/far split with 3D lepton distance and strict `>` fallback semantics. The remaining implementation risk for this slice is exact `FootClass__Find_Nearby_Passable_Cell` fallback search behavior, including radius, candidate cap, direct/indirect classification, and frame-modulo selection; see `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass__Mission_Harvest` state 2 fallback destination | verified | `0x0073EC1F`-`0x0073EDB5` | none for destination formula |
| `QueueingCell` field mapping | verified | `0x00461506` string xref; writes `+0x1618/+0x161C` | none |
| `NumberOfDocks` / `DockingOffset%d` exclusion | verified | readers at `0x00464938`/`0x004649B7`; no state-2 reads | none for this branch |
| `Find_Nearby_Passable_Cell` use before destination | verified | call at `0x0073ED75`; function `0x0056DC20` | exact candidate order already covered elsewhere, not re-expanded |
| radio `anchor+(3,1)` distinction | verified by cross-doc | `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`; no use in fallback block | no further work in this slot |
| exact post-arrival docking sequence | deferred | out of scope | slot 4 / dock-arrival investigation |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does far/fallback use `QueueingCell=(4,1)`? Yes. It reads `BuildingTypeClass+0x1618/+0x161C`, which are loaded from `QueueingCell`, then seeds `Find_Nearby_Passable_Cell`. Evidence: `0x0073ED25`, `0x0073ED34`, `0x00461520`.  
[RESOLVED] OQ-2 - Does far/fallback use radio `anchor+(3,1)`? No. That formula is not in the fallback block; the fallback block reads `+0x1618/+0x161C`. Evidence: `0x0073ECDF`-`0x0073EDB5`; radio formula documented separately.  
[RESOLVED] OQ-3 - Does far/fallback use `NumberOfDocks`/`DockingOffset%d`? No. Those fields are read from INI but not used in state-2 fallback. Evidence: `0x00464938`, `0x004649B7`, and state-2 fallback reads only `+0x1618/+0x161C`.  
[RESOLVED] OQ-4 - Is the passable-cell search before `Set_Destination`? Yes. `0x0073ED75` calls `FootClass__Find_Nearby_Passable_Cell`; only after a non-null result does `0x0073EDB5` call vtable `+0x480`.  
[DEFERRED] OQ-5 - Exact radio/dock messages after the miner appears at the fallback cell. Category: out-of-scope. Next step: dock-arrival/link timing slot.

## Sources

- Ghidra: `UnitClass__Mission_Harvest @ 0x0073E5E0`
- Ghidra: `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`
- Ghidra: `FootClass__Find_Docking_Bay @ 0x004DF040`
- Ghidra: `BuildingClass__CanDock @ 0x00457CE0`
- Ghidra: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`
- `docs/research/FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md`
- `docs/research/NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md`
- `docs/research/traces/CHRONO_MINER_TELEPORT_INBOUND_VISUAL_CHAIN_TRACE.md`
- `docs/research/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`
