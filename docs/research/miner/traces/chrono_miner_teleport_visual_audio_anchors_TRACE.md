# Chrono Miner Return Teleport Visual/Audio Anchors Trace

Date: 2026-05-20
Scenario: Standard YR `CMIN` return teleport to a standard `GAREFN` on a flat map.
Concrete fixture for numeric comparison: CMIN departs cell `(80,80,0)`, GAREFN NW anchor
cell `(10,10)`.

Scope: chrono-out effect cell, chrono-in effect cell, ChronoOutSound/ChronoInSound cells,
ordering relative to teleport state, and whether anchors use depart cell, refinery center,
refinery pad, or refinery queue cell.

Hard constraints honored: Ghidra MCP read-only only; no Rust/INI/repo-doc edits; this is
the only file written by this run.

## Sources Checked

- INI: `ini/rulesmd.ini` and `ini/artmd.ini`.
- Rust: `src/sim/miner/miner_system.rs`, `src/sim/movement/teleport_movement.rs`,
  `src/sim/miner/miner_dock_sequence.rs`, `src/app_sim_tick.rs`, `src/audio/events.rs`.
- Existing research used as pointers: `NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md`,
  `MISSION_HARVEST_STATE2_TOOFAR_PATHFIND_BRANCH_GHIDRA_REPORT.md`,
  `CHRONO_WARP_VISUAL_RENDERING.md`.
- Fresh read-only Ghidra decompilation:
  - `UnitClass__Mission_Harvest` at `0x0073E5E0`.
  - `TeleportLocomotionClass__StateMachineTick` at `0x007192F0`.
  - `TechnoClass__Set_Destination` at `0x00741970`.

## Active YR Binding

`CMIN` is the active standard Allied Chrono Miner, not a TS legacy path:

- `rulesmd.ini [CMIN]`: `Harvester=yes`, `Teleporter=yes`,
  `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` (Teleport locomotor),
  `Dock=NAREFN,GAREFN`, `ChronoInSound=ChronoMinerTeleport`,
  `ChronoOutSound=ChronoMinerTeleport`.
- `artmd.ini [GAREFN]`: `Foundation=4x3`, `QueueingCell=4,1`,
  `RemoveOccupy1=3,1`.
- Ghidra `UnitClass__Mission_Harvest` reads the unit type Teleporter byte and, in state 2
  return, reads `BuildingTypeClass+0x1618/+0x161C` for the target seed. Existing
  read-INI research verifies those fields are `QueueingCell`.

## Pipeline

`Mission_Harvest` state 2 return -> far chrono-return branch -> `Set_Destination` to
queue cell -> next TeleportLoco `StateMachineTick` -> departure `WarpOut` effect ->
ChronoOutSound -> destination set/mark -> ChronoInSound -> arrival `WarpOut` effect.

Our pipeline:

`tick_miners` return branch -> `spawn_warp_effects` immediately -> `issue_teleport_command`
to pad cell -> next `tick_teleport_movement` relocates to pad.

## Stage Results

### Stage 1 - Return Teleport Target Anchor

gamemd:

- GAREFN anchor `(10,10)`.
- `QueueingCell=4,1`.
- State 2 return target seed = `(10+4, 10+1)` = `(14,11)`.
- Clean flat-map passability keeps ring-0 candidate, so landing/return target = `(14,11)`.

Our code:

- `try_issue_chrono_return_teleport` calls `refinery_pad_for_sid`.
- `refinery_pad_cell` fallback for 4x3 returns `(rx + width - 1, ry + height / 2)` =
  `(13,11)`, the refinery pad/RemoveOccupy cell.

Verdict: FAIL. Our return teleport anchor is the pad `(13,11)`; gamemd uses the queue cell
`(14,11)`.

### Stage 2 - Chrono-Out Visual Anchor

gamemd:

- `TeleportLocomotionClass__StateMachineTick` constructs `Rules+0x33C` (`WarpOut`) at
  the current unit location before unmarking/moving.
- Fixture departure cell: `(80,80)`.

Our code:

- `spawn_warp_effects` pushes the first `WorldEffect` at `depart=(snap.rx,snap.ry,z)`.
- Fixture departure cell: `(80,80)`.

Verdict: PASS for cell anchor. Pixel-center equality is UNCHECKED because this trace did
not render and compare the SHP centering pixels against gamemd.

### Stage 3 - Chrono-Out Sound Anchor

gamemd:

- After unmarking the source but before destination set/mark, `StateMachineTick` plays
  `TypeClass+0x578` fallback `Rules+0x21C` at the unit's current departure coordinates.
- CMIN resolves to `ChronoMinerTeleport`.
- Fixture sound cell: `(80,80)`.

Our code:

- `spawn_warp_effects` resolves per-unit `chrono_out_sound` and emits
  `SimSoundEvent::ChronoTeleport { rx: depart.0, ry: depart.1 }`.
- Fixture sound cell: `(80,80)`, sound id `ChronoMinerTeleport`.

Verdict: PASS for cell and sound id.

### Stage 4 - Chrono-In Visual Anchor

gamemd:

- After destination set/mark, `StateMachineTick` constructs the second `Rules+0x33C`
  `WarpOut` at the new unit location.
- Fixture arrival cell: `(14,11)`.

Our code:

- `spawn_warp_effects` pushes the second `WorldEffect` at `arrive=(pad.0,pad.1,z)`.
- Fixture arrival cell: `(13,11)`.

Verdict: FAIL. Player sees the inbound flash one cell west of gamemd.

### Stage 5 - Chrono-In Sound Anchor

gamemd:

- After destination mark and before the arrival anim constructor, `StateMachineTick` plays
  `TypeClass+0x574` fallback `Rules+0x218` at destination coordinates.
- CMIN resolves to `ChronoMinerTeleport`.
- Fixture sound cell: `(14,11)`.

Our code:

- `spawn_warp_effects` resolves per-unit `chrono_in_sound` and emits
  `SimSoundEvent::ChronoTeleport { rx: arrive.0, ry: arrive.1 }`.
- Fixture sound cell: `(13,11)`, sound id `ChronoMinerTeleport`.

Verdict: FAIL. Sound id matches, but the spatial source is one cell west of gamemd.

### Stage 6 - Ordering Relative to Teleport State

gamemd:

- `Mission_Harvest` sets the destination in state 2.
- The actual visual/sound events occur inside the next `StateMachineTick`, in the same
  invocation that moves/marks the unit at destination.

Our code:

- `spawn_warp_effects` runs before `issue_teleport_command`.
- In full `Simulation::advance_tick`, miner logic runs in production phase 7, after
  `tick_teleport_movement` has already run in phase 2. The visual/sound events are therefore
  emitted on tick `T`, while relocation happens on tick `T+1`.

Verdict: FAIL. Effects and sounds are one sim tick early relative to the position snap.

### Stage 7 - Intra-Tick Event Order

gamemd observed order in `StateMachineTick` phase 0:

1. Departure `WarpOut` anim at `(80,80)`.
2. Set `BeingWarped`.
3. Unmark source.
4. ChronoOutSound at `(80,80)`.
5. Set destination / mark at `(14,11)`.
6. ChronoInSound at `(14,11)`.
7. Arrival `WarpOut` anim at `(14,11)`.

Our order in `spawn_warp_effects`:

1. Departure `WorldEffect` at `(80,80)`.
2. Arrival `WorldEffect` at `(13,11)`.
3. ChronoOutSound at `(80,80)`.
4. ChronoInSound at `(13,11)`.
5. Relocation later in `tick_teleport_movement`.

Verdict: FAIL. Arrival effect is queued before ChronoOutSound and before relocation; gamemd
queues arrival effect after destination mark and after ChronoInSound.

### Stage 8 - Semicolon Secondary Effect

gamemd data:

- `rulesmd.ini [General]`: `WarpOut=WARPOUT;WAKE2`.
- Existing verified rendering research records `Rules+0x33C` as the self-teleport effect
  slot, with the semicolon secondary anim participating in the visual.

Our code:

- `rules.general.warp_out.name` carries only the primary effect name used by
  `WorldEffect`; `spawn_warp_effects` emits one SHP name per endpoint.

Verdict: NOT-IMPLEMENTED for the secondary `WAKE2` endpoint effect. The anchor would be the
same endpoint cell as the primary effect; the visual component is absent.

## Verdict Tally

PASS: 3 | FAIL: 5 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Top Player-Visible Findings

1. Stage 1 - Arrival/teleport anchor: our CMIN snaps to refinery pad `(13,11)` instead of
   gamemd queue cell `(14,11)`. Our code: `src/sim/miner/miner_system.rs:862-875`,
   `src/sim/miner/miner_dock_sequence.rs:95-109`. gamemd evidence:
   `UnitClass__Mission_Harvest 0x0073E5E0`, `QueueingCell=4,1`.
2. Stage 4 - Chrono-in visual anchor: inbound flash appears at `(13,11)` instead of
   `(14,11)`. Our code: `src/sim/miner/miner_system.rs:899-914`. gamemd evidence:
   `TeleportLocomotionClass__StateMachineTick 0x007192F0`, second `Rules+0x33C`
   constructor after destination mark.
3. Stage 5 - Chrono-in sound anchor: spatial `ChronoMinerTeleport` plays at `(13,11)`
   instead of `(14,11)`. Our code: `src/sim/miner/miner_system.rs:931-938`,
   `src/app_sim_tick.rs:382-388`. gamemd evidence: `StateMachineTick 0x007192F0`,
   `TypeClass+0x574` sound at destination.
4. Stage 6 - Timing: our visuals/sounds are emitted one sim tick before relocation.
   Our code/order: `src/sim/world/mod.rs:1093-1098` before `src/sim/world/mod.rs:1444-1445`;
   `spawn_warp_effects` runs in miner phase. gamemd evidence: `StateMachineTick 0x007192F0`
   emits effects/sounds and relocates in the same invocation.
5. Stage 8 - Secondary visual: `WAKE2` component of `WarpOut=WARPOUT;WAKE2` is absent.
   Our code: `src/sim/miner/miner_system.rs:889-914`. gamemd/INI evidence:
   `rulesmd.ini [General] WarpOut=WARPOUT;WAKE2`, `Rules+0x33C` used by
   `StateMachineTick 0x007192F0`.

## Adjacent Findings

- CMIN chrono delay / BeingWarped opacity is outside this run's anchor scope. The fresh
  `StateMachineTick` decompile sets `BeingWarped` before movement and uses the distance timer;
  this should get a separate focused trace because older docs conflict on harvester delay.
- Radio `Receive_Radio` case `0x0E` uses hardcoded `(NW+3,NW+1)` for the later dock/pad
  interaction. That is adjacent to, but distinct from, the far-return warp landing anchor
  from `QueueingCell=4,1`.

Status: COMPLETE
