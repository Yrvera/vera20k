# Trace: Chrono Miner Full-Cargo Return Teleport To Refinery Pad

**Scenario:** A full Allied Chrono Miner (`CMIN`) at ore cell `(87,187)` returns to its owning Allied Ore Refinery (`GAREFN`). The concrete fixture uses the refinery anchor from the current Rust test geometry: `GAREFN` at top-left cell `(85,180)`.

**Question:** Does `gamemd.exe` teleport the full CMIN back to the refinery pad, what cell is used as the return target/arrival anchor, and does it ever path-walk instead?

**Date:** 2026-05-20

**Write scope:** This report only. No Rust, INI, or in-repo docs were modified.

> **Repo-status supersession 2026-05-25:** This trace predates the current Rust
> `ChronoHarvTooFarDistance` close/far split. Treat any current-Rust finding
> about a hardcoded 2-cell return threshold or direct pad teleport as stale
> unless reverified against current source. Keep its binary destination
> research only where it agrees with newer close/far and fallback reports.

## Sources Checked

- Rust:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
  - `src/sim/movement/teleport_movement.rs`
  - `src/sim/miner/miner_tests.rs`
- INI:
  - `ini/rulesmd.ini`
  - `ini/artmd.ini`
- Existing research:
  - `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
  - `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`
  - `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
  - `NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md`
  - prior traces under `ra2-rust-game-docs/traces/`
- Live read-only Ghidra decompilation:
  - `UnitClass__Mission_Harvest @ 0x0073E5E0`
  - `FootClass__Find_Docking_Bay @ 0x004DF040`
  - `BuildingClass__Receive_Radio @ 0x0043C2D0`
  - `TechnoClass__Set_Destination @ 0x00741970`
  - `TeleportLocomotionClass__HeadToCoord @ 0x00718100`
  - `TeleportLocomotionClass__InitiateWarp @ 0x00719400`

Ghidra use was read-only only.

## Pipeline

Full `CMIN` cargo -> `UnitClass::Mission_Harvest` state 2 / Rust `begin_return` -> choose owning `GAREFN` -> decide chrono return branch -> either radio/dock path-walk or immediate teleport -> arrive at refinery-side target cell -> enter dock sequence.

## Concrete Values

- `CMIN` is active standard YR data:
  - `rulesmd.ini:7351` `[CMIN]`
  - `rulesmd.ini:7374` `Storage=20`
  - `rulesmd.ini:7396` `Teleporter=yes`
  - `rulesmd.ini:7398` teleport locomotor `{4A582747-9839-11d1-B709-00A024DDAFD1}`
- `GAREFN` is active standard YR data:
  - `rulesmd.ini:11722` `[GAREFN]`
  - `rulesmd.ini:11726` `DockUnload=yes`
  - `rulesmd.ini:11727` `Refinery=yes`
  - `rulesmd.ini:11729` `NumberOfDocks=1`
  - `artmd.ini:1766` `Foundation=4x3`
  - `artmd.ini:1773` `QueueingCell=4,1`
- `ChronoHarvTooFarDistance=50`:
  - `rulesmd.ini:294`
  - Ghidra: `UnitClass__Mission_Harvest` state 2 reads `RulesClass+0xD7C` and compares against `* 0x100`.
- Scenario geometry:
  - Miner cell: `(87,187)`.
  - Refinery top-left anchor: `(85,180)`.
  - gamemd state-2 distance reference: refinery object coord/top-left anchor.
  - gamemd CAN_DOCK move-to target: anchor + `(3,1)` = `(88,181)`.
  - Rust center reference: `(85 + 4/2, 180 + 3/2)` = `(87,181)`.
  - Rust pad target: `(85 + 4 - 1, 180 + 3/2)` = `(88,181)`.

## Stage Table

| Stage | Output Compared | Our Output | gamemd Output | Verdict |
|---|---|---:|---:|---|
| 1. Scenario rules data | CMIN capacity / flags, GAREFN dock flags | `Storage=20`, chrono miner, GAREFN dockable | Same INI values feed standard YR; live decompile gates on harvester/teleporter and DockUnload/Refinery flags | PASS |
| 2. Full cargo trigger | Full cargo threshold | Assumed full: 20/20 bales for this scenario | Assumed full: storage ratio reaches 1.0 before state 2 | UNCHECKED |
| 3. Return distance reference | Reference point for branch | Refinery center `(87,181)` | Refinery top-left/object coord `(85,180)` | FAIL |
| 4. Return distance number | Distance vs chrono threshold | `cell_dist_sq((87,187),(87,181)) = 36`; `2^2 = 4`; far=true | `sqrt((2*256)^2 + (7*256)^2) = floor(sqrt(3473408)) = 1863`; `50*256 = 12800`; far=false | FAIL |
| 5. Branch/action | Teleport vs path-walk | Calls `spawn_warp_effects` and `issue_teleport_command` to `(88,181)` | Sends radio/HELLO path, advances state 2->3, then Mission_Enter path-walks; no teleport issued | FAIL |
| 6. Return target/arrival cell | Cell used for immediate approach target | Teleport arrival target `(88,181)` | CAN_DOCK `MOVE_TO_CELL` target `(88,181)` | PASS |
| 7. Path walking | Whether miner path-walks in this scenario | No; position snaps by teleport to `(88,181)` on teleport tick | Yes; the original path-walks because 1863 <= 12800 | FAIL |
| 8. Warp visuals/sounds | Chrono warp effects at return start | Two warp effects and ChronoTeleport sounds are emitted | None on this branch; teleport locomotor is not armed | FAIL |
| 9. Exact walking duration | Tick count from `(87,187)` to `(88,181)` | 1 relocation tick in teleport path | Not computed from live movement tracks in this trace | UNCHECKED |
| 10. Dock admission after arrival | Exact tick/order after reaching `(88,181)` | Transitions to Dock/Approach after teleport guard clears | Mission_Enter/CAN_DOCK sequence reaches dock path after walking | UNCHECKED |

## gamemd Evidence

### `UnitClass__Mission_Harvest @ 0x0073E5E0`

State 2 is the active full-harvester return branch. The function reads the unit type teleporter byte (`Type+0xCD4`) into `cVar1`. For a teleporter harvester with a found dock:

1. It calls the dock object's coordinate function and the unit's coordinate function.
2. It computes 3D Euclidean distance in leptons and floors through `Math__ftol`.
3. For teleporter harvesters, it compares:

```text
distance <= RulesClass+0xD7C * 0x100
```

For this scenario:

```text
dx = (87 - 85) * 256 = 512
dy = (187 - 180) * 256 = 1792
dz = 0
distance = floor(sqrt(512^2 + 1792^2)) = floor(sqrt(3,473,408)) = 1863
threshold = 50 * 256 = 12,800
1863 <= 12,800 => true
```

So the live YR path takes the within-distance branch, calls radio code `2` to the refinery, sets mission-harvest substate `3` on acceptance, and later enters Mission_Enter. It does not enter the fallback warp-target path for this scenario.

The fallback warp path is still live in standard YR, but only when the distance exceeds `ChronoHarvTooFarDistance` or reservation fallback conditions require it. That adjacent far-return behavior is not this scenario.

### `BuildingClass__Receive_Radio @ 0x0043C2D0`

The active refinery `CAN_DOCK` case (`0x0E`) gates on standard refinery flags including `DockUnload=yes` / related type bytes. For DockUnload refineries it calls the building cell-coordinate function, then constructs:

```text
target_cell = building_anchor + (3,1)
```

For `GAREFN` at `(85,180)`, the dock approach target is:

```text
(85 + 3, 180 + 1) = (88,181)
```

This is not the `QueueingCell=4,1` value; `QueueingCell` remains relevant to other fallback/queue contexts, not this accepted CAN_DOCK move-to target.

### `TeleportLocomotionClass__HeadToCoord @ 0x00718100`

This function arms the teleport locomotor when a teleport destination is assigned. It is active in YR and used by `CMIN` in far-return/self-teleport cases, but the state-2 branch above does not assign a teleport destination for `(87,187)` -> `GAREFN(85,180)`.

### `TeleportLocomotionClass__InitiateWarp @ 0x00719400`

The live teleport path includes the harvester special case: when the object is a `UnitClass` harvester, it sets the chrono timer duration to `0` and clears `BeingWarped`. This confirms the teleport machinery is active for CMIN in standard YR, but it does not fire for this close-return scenario.

## Rust Evidence

### Return Branch

`src/sim/miner/miner_system.rs:40`

```text
const CHRONO_INBOUND_WARP_THRESHOLD_CELLS: u32 = 2;
```

`src/sim/miner/miner_system.rs:844-869`:

- `try_issue_chrono_return_teleport` handles chrono return.
- It uses `refinery_center_cell_for_sid` as the distance reference.
- It compares `cell_dist_sq(...) > threshold * threshold`.
- On true, it calls `spawn_warp_effects` and `issue_teleport_command`.

For this scenario:

```text
center = (87,181)
cell_dist_sq((87,187),(87,181)) = 36
threshold^2 = 2^2 = 4
36 > 4 => true
```

So Rust teleports.

### Rust Arrival Cell

`src/sim/miner/miner_system.rs:862` calls:

```text
refinery_pad_for_sid(...).unwrap_or(dock)
```

`src/sim/miner/miner_dock_sequence.rs:95-108` returns the no-offset refinery pad cell:

```text
(rx + width - 1, ry + height / 2)
```

For `GAREFN(85,180)` with `4x3` foundation:

```text
(85 + 3, 180 + 1) = (88,181)
```

Rust's immediate teleport target equals gamemd's accepted CAN_DOCK walk target numerically, but the movement mode and timing are wrong.

## Findings

### F01 - Return branch teleports when gamemd path-walks

**Stage:** 4/5/7

**Player-visible difference:** The miner instantly warps from `(87,187)` to `(88,181)` with chrono effects and sound. In standard YR, this same miner path-walks to the refinery because it is only 1863 leptons from the refinery origin, below the 12800-lepton threshold.

**Our code:** `src/sim/miner/miner_system.rs:40`, `src/sim/miner/miner_system.rs:844-869`, `src/sim/movement/teleport_movement.rs:103-156`

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0`, state 2: teleporter branch compares `distance <= RulesClass+0xD7C * 0x100`; for this scenario `1863 <= 12800`, so the path-walk/radio branch is taken.

### F02 - Return threshold constant is 2 cells instead of `ChronoHarvTooFarDistance=50`

**Stage:** 4

**Player-visible difference:** CMINs warp home from ordinary nearby ore cells instead of driving in, making returns faster and producing warp audiovisuals that should not happen.

**Our code:** `src/sim/miner/miner_system.rs:40`

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0` reads `RulesClass+0xD7C`; retail `rulesmd.ini:294` sets `ChronoHarvTooFarDistance=50`.

### F03 - Return distance uses refinery center instead of gamemd object/origin coordinate

**Stage:** 3/4

**Player-visible difference:** The warp/drive boundary is shifted around each refinery. Near the real 50-cell threshold, some miners choose the wrong return mode even if the threshold constant is fixed.

**Our code:** `src/sim/miner/miner_system.rs:856`, `src/sim/miner/miner_system.rs:1033-1047`

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0` compares unit coordinates against the dock object's `vtable+0x48` coordinates before the threshold test.

### F04 - Spurious chrono return audiovisuals

**Stage:** 8

**Player-visible difference:** The player sees/hears a Chrono Miner teleport at `(87,187)` and `(88,181)`. Standard YR should show a normal drive-in approach for this geometry.

**Our code:** `src/sim/miner/miner_system.rs:871-924`, `src/sim/movement/teleport_movement.rs:158-248`

**gamemd evidence:** Same `UnitClass__Mission_Harvest @ 0x0073E5E0` branch; `TeleportLocomotionClass__HeadToCoord @ 0x00718100` is never armed for this scenario.

## Direct Answer

For a full `CMIN` at `(87,187)` returning to its owning `GAREFN` at `(85,180)`, `gamemd.exe` does **not** teleport it back to the refinery pad. It takes the within-threshold radio/dock branch and path-walks.

The accepted refinery approach target is `(88,181)`, computed as `GAREFN` anchor `(85,180)` plus hardcoded `(3,1)` in `BuildingClass::Receive_Radio` case `0x0E`.

Rust currently also targets `(88,181)`, but reaches it by immediate teleport because it uses a hardcoded 2-cell chrono return threshold against the refinery center. The target cell matches; the movement mode, timing, and audiovisual output do not.

## Adjacent Findings

- Far-return CMIN behavior where distance exceeds `ChronoHarvTooFarDistance=50` is live and uses the teleport locomotor, but that is not this scenario.
- Queueing/fallback behavior involving `QueueingCell=4,1` is adjacent. It was not traced here except to distinguish it from the accepted CAN_DOCK target `(anchor+3, anchor+1)`.
- Dock unload, bale deposit cadence, exit path, targeting detachment on actual chrono warp, and post-dump behavior are out of scope for this trace.

## Verdict Tally

PASS: 2 | FAIL: 5 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0
