# Trace: Chrono Miner Full-Cargo Return Walk vs Teleport

**Scenario:** Loaded Allied Chrono Miner (`CMIN`) at cell `(87,187)` returning to standard Allied Ore Refinery (`GAREFN`) at top-left cell `(85,180)` on flat passable terrain.

**Scope:** One movement branch only: full-cargo return path-walk vs teleport to refinery. Adjacent dock unload, post-dump exit, multi-miner queueing, and far-return behavior are out of scope.

**Date:** 2026-05-20

**Write scope:** This report only. No Rust, INI, in-repo docs, or other files were modified.

> **Repo-status supersession 2026-05-25:** Any table rows below that report
> current Rust's close/far return threshold as hardcoded `2` cells are stale.
> Current Rust reads `ChronoHarvTooFarDistance`. Use this trace for the gamemd
> near-return walk-vs-teleport behavior, not as current repo-status evidence.

## Sources Checked

- Current Rust:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
  - `src/sim/movement/teleport_movement.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/miner/mod.rs`
- INI:
  - `ini/rulesmd.ini`
  - `ini/artmd.ini`
- Prior traces/research used as pointers:
  - `chrono_miner_full_cargo_return_teleport_to_refinery_pad_TRACE.md`
  - `CHRONO_MINER_TOO_FAR_THRESHOLD_BRANCH_TRACE.md`
  - `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- Read-only live Ghidra:
  - `UnitClass__Mission_Harvest`
  - `BuildingClass__Receive_Radio`
  - `TeleportLocomotionClass__HeadToCoord`

Ghidra use was read-only only.

## Direct Answer

For a loaded `CMIN` at `(87,187)` returning to `GAREFN` at `(85,180)`, standard YR path-walks toward the refinery. It does not teleport.

The binary computes the return-branch distance from the miner coordinate to the refinery object's coordinate, which for this scenario is the building origin/top-left `(85,180)`. On flat terrain:

```text
dx = (87 - 85) * 256 = 512
dy = (187 - 180) * 256 = 1792
distance = floor(sqrt(512^2 + 1792^2)) = 1863 leptons
threshold = ChronoHarvTooFarDistance * 256 = 50 * 256 = 12800 leptons
1863 <= 12800 => close branch => radio/refinery path-walk
```

Current Rust teleports. It compares the miner to the CAN_DOCK dock cell `(88,181)` with a hardcoded `2`-cell threshold:

```text
cell_dist_sq((87,187), (88,181)) = 1^2 + 6^2 = 37
threshold^2 = 2^2 = 4
37 > 4 => teleport branch
```

The teleport destination is not the gamemd CAN_DOCK target. Current Rust stages at art `QueueingCell=4,1`, so for `GAREFN(85,180)` it targets `(89,181)`. The gamemd close branch's `BuildingClass__Receive_Radio` CAN_DOCK path computes `(85+3,180+1) = (88,181)`.

## Active YR Evidence

`CMIN` and `GAREFN` are active standard YR data:

- `rulesmd.ini:7351` `[CMIN]`
- `rulesmd.ini:7374` `Storage=20`
- `rulesmd.ini:7396` `Teleporter=yes`
- `rulesmd.ini:7398` teleport locomotor `{4A582747-9839-11d1-B709-00A024DDAFD1}`
- `rulesmd.ini:11722` `[GAREFN]`
- `rulesmd.ini:11726` `DockUnload=yes`
- `rulesmd.ini:11727` `Refinery=yes`
- `rulesmd.ini:11729` `NumberOfDocks=1`
- `rulesmd.ini:294` `ChronoHarvTooFarDistance=50`
- `artmd.ini:1766` `Foundation=4x3`
- `artmd.ini:1773` `QueueingCell=4,1`

The relevant binary path is active in YR, not dormant TS legacy: `UnitClass__Mission_Harvest` gates on the unit type's live harvester and teleporter bytes, and `CMIN` has both through retail YR INI. `BuildingClass__Receive_Radio` case `0x0E` gates on live refinery DockUnload/related type bytes, and `GAREFN` has the standard refinery flags.

## Pipeline

Loaded `CMIN` harvest state -> return-to-refinery state -> find/refine dock target -> decide close path-walk vs far teleport -> either radio CAN_DOCK + walk to `(88,181)` or Rust chrono warp to staging -> dock admission.

## Stage Results

| Stage | Compared Output | Rust Output | gamemd Output | Verdict |
|---|---|---:|---:|---|
| 1. Rules identity | Active CMIN/GAREFN return inputs | `CMIN`, `Storage=20`, chrono miner; `GAREFN` refinery | Same active YR INI inputs | PASS |
| 2. Full-cargo return entry | Transition from full load into return branch | Loaded scenario can enter `ReturnToRefinery`; exact tick not measured | Full storage enters `Mission_Harvest` state 2; exact tick not measured here | UNCHECKED |
| 3. Threshold source | Chrono close/far threshold | Hardcoded `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2` | `ChronoHarvTooFarDistance=50` read at `RulesClass+0xD7C` | FAIL |
| 4. Distance/reference value | Branch comparison value | `cell_dist_sq((87,187),(88,181)) = 37`; threshold squared `4` | `distance=1863`; threshold `12800` | FAIL |
| 5. Branch/action | Walk vs teleport | Teleport branch starts | Close radio/dock branch starts | FAIL |
| 6. Immediate target | First concrete movement target | Teleport staging `(89,181)` | CAN_DOCK path target `(88,181)` | FAIL |
| 7. Player-visible movement mode | What the player sees first | Chrono warp out/in, then follow-up movement needed | Normal path-walk from `(87,187)` toward refinery | FAIL |
| 8. Warp sound/effects | Audiovisual output at return start | Two chrono warp effects/sounds emitted by `spawn_warp_effects` | No teleport locomotor armed for this close branch | FAIL |
| 9. Exact walk timing | Tick count to refinery-side target | Not measured for this exact map/grid | Not measured from drive tracks in this trace | UNCHECKED |
| 10. Dock admission timing | Exact radio/dock tick order after arrival | Not measured for this exact path | Not measured beyond branch and CAN_DOCK target | UNCHECKED |

## Rust Evidence

`src/sim/miner/miner_system.rs:36-39` defines the current chrono inbound return gate:

```text
CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2
```

`src/sim/miner/miner_system.rs:852-891` implements `try_issue_chrono_return_teleport`. It:

1. Requires `MinerKind::Chrono`.
2. Computes staging with `chrono_return_staging_cell_for_sid`.
3. Compares `cell_dist_sq((snap.rx, snap.ry), dock) > threshold * threshold`.
4. On true, calls `spawn_warp_effects`.
5. Calls `issue_teleport_command(..., is_harvester=true)`.

`src/sim/miner/miner_system.rs:1030-1062` computes the return staging cell from `refinery_queue_cell`, which uses art `QueueingCell` when present. For `GAREFN(85,180)`, `QueueingCell=4,1` gives `(89,181)`.

`src/sim/miner/miner_system.rs:1064-1072` computes the regular dock cell through `refinery_can_dock_queue_cell`, which is `(rx+3, ry+1)`. For this scenario, `dock=(88,181)`.

`src/sim/movement/teleport_movement.rs:103-150` sets `teleport_state`, clears existing ground movement, and for `is_harvester=true` uses `being_warped_ticks=0`. That makes this a visible immediate chrono relocation path, not a normal drive-in return.

## gamemd Evidence

`UnitClass__Mission_Harvest` state 2 is the active full-harvester return branch. In the teleporter path, it:

1. Calls the unit's docking-bay lookup.
2. Calls the dock/refinery object's coordinate virtual.
3. Calls the unit coordinate virtual.
4. Computes 3D Euclidean distance in leptons.
5. Floors through `Math__ftol`.
6. Compares `distance <= RulesClass+0xD7C * 0x100`.
7. If true, sends radio code `2` to the refinery and advances to the next return/dock state.

For this scenario, `1863 <= 12800`, so the close radio/dock branch is taken.

`BuildingClass__Receive_Radio` case `0x0E`, on the DockUnload/refinery path, computes the returned cell for this branch as building map cell plus `(3,1)`. For `GAREFN(85,180)`, that is `(88,181)`.

`TeleportLocomotionClass__HeadToCoord` is active in standard YR and arms the teleport state machine when a teleport destination is assigned. For this concrete close-return branch, `UnitClass__Mission_Harvest` does not assign such a destination; it takes the radio/dock path instead.

## Findings

### F01 - Rust teleports when gamemd path-walks

**Stage:** 5/7

**Player-visible difference:** The miner warps away from `(87,187)` and reappears near the refinery. In standard YR, this miner is close under the 50-cell chrono-harvester threshold and drives toward the refinery.

**Our code:** `src/sim/miner/miner_system.rs:852-891`, `src/sim/movement/teleport_movement.rs:103-150`

**gamemd evidence:** `UnitClass__Mission_Harvest` state 2 compares `distance <= RulesClass+0xD7C * 0x100`; `1863 <= 12800` takes the radio/dock drive branch.

### F02 - Rust uses a hardcoded 2-cell branch threshold instead of `ChronoHarvTooFarDistance=50`

**Stage:** 3/4

**Player-visible difference:** Normal nearby ore returns become chrono warps, making CMIN return much faster and showing warp effects that should not occur.

**Our code:** `src/sim/miner/miner_system.rs:36-39`, `src/sim/miner/miner_system.rs:867-870`

**gamemd evidence:** `UnitClass__Mission_Harvest` state 2 reads `RulesClass+0xD7C`; retail `rulesmd.ini:294` sets `ChronoHarvTooFarDistance=50`.

### F03 - Rust warp destination is `(89,181)`, but gamemd close-branch target is `(88,181)`

**Stage:** 6

**Player-visible difference:** After the incorrect warp, the miner appears one cell east of the CAN_DOCK target and must correct its position before dock admission. The original close branch walks directly toward `(88,181)`.

**Our code:** `src/sim/miner/miner_system.rs:1030-1062`, `src/sim/miner/miner_dock_sequence.rs:70-89`

**gamemd evidence:** `BuildingClass__Receive_Radio` case `0x0E` computes `building_cell + (3,1)` for the DockUnload CAN_DOCK target.

### F04 - Rust emits spurious chrono audiovisuals

**Stage:** 8

**Player-visible difference:** The player hears/sees Chrono Miner teleport effects on a return that should be a mundane drive-in approach.

**Our code:** `src/sim/miner/miner_system.rs:878-891`, `src/sim/miner/miner_system.rs:894-924`, `src/sim/movement/teleport_movement.rs:103-150`

**gamemd evidence:** The close branch in `UnitClass__Mission_Harvest` sends radio code `2` and does not arm `TeleportLocomotionClass__HeadToCoord`.

## Adjacent Findings

- Prior reports that say current Rust measures this branch from the refinery center are stale for this code snapshot. Current Rust measures the threshold from the CAN_DOCK dock cell `(rx+3,ry+1)`.
- Far-return behavior beyond `ChronoHarvTooFarDistance=50` is live in standard YR and uses the teleport locomotor, but that is not this scenario.
- The difference between `QueueingCell=4,1` and the CAN_DOCK hardcoded `(3,1)` matters here because Rust incorrectly enters its warp-staging path. The broader queue/fallback semantics are adjacent and not traced further in this slot.

## Verdict Tally

PASS: 1 | FAIL: 6 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Status

COMPLETE.
