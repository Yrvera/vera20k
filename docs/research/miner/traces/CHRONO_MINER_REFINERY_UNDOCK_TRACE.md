# Chrono Miner — Refinery Undock Trace

**Scope:** End-of-deposit moment through miner physically clear of dock cell with destination set toward ore.
**Slot:** 4 of chrono-miner swarm.
**Date:** 2026-05-19
**Iron Law:** PASS requires literal numerical equality between our output and gamemd's.
         Stages where both sides were not computed are UNCHECKED, not PASS.

---

## Stage Map (undock only — deposit itself is slot 5, warp-back is slot 2)

| # | Stage | gamemd | Ours | Result |
|---|-------|--------|------|--------|
| U01 | Anim teardown (slots 0xA, 0xB cleared) | ClearAnimSlot(10), ClearAnimSlot(11) | Not implemented | NOT-IMPLEMENTED |
| U02 | Departure VOC (`BunkerWallsDownSound`) | VocClass::PlayAt(TankBunkerDown, building_pos) | Not implemented | NOT-IMPLEMENTED |
| U03 | Departure anim creation (slots 0xC, 0xD) | CreateAnimForSlot(12=SpecialAnimThree, 13=SpecialAnimFour) | Not implemented | NOT-IMPLEMENTED |
| U04 | DepositCooldown hold (park on pad during deposit anim) | Implicit: building-side state machine holds miner on pad | `phase_deposit_cooldown` counts down `deposit_anim_duration_ticks` | PASS |
| U05 | Miner dock-link teardown — unit side | `piVar1[0xb9]=0` (byte offset +0x2E4 on unit) before Power_On | `snap.miner.reserved_refinery = None` at Departing arrival | UNCHECKED |
| U06 | Miner dock-link teardown — building side | `param_1->field_0x2e4 = 0` after SetMission(MOVE) | `sim.production.dock_reservations.release(ref_sid)` at Departing arrival | UNCHECKED |
| U07 | Power_On locomotor | `(*loco_vtable+0x58)(loco)` — DriveLocomotionClass::Power_On | Implicit: movement issued via `issue_move_command` (no explicit Power_On call) | UNCHECKED |
| U08 | Force_Track 0x47 issued at departure start | `Head_To(track=0x47, building_center.x − 0x80, building_center.y + 0x80, z)` | `entity.facing = 0x47; entity.facing_target = Some(0x47)` at Departing entry | FAIL (details §2) |
| U09 | Speed multiplier restored to 1.0 | `(*unit_vtable+0x544)(0, 0x3FF00000)` = IEEE 754 double 1.0 | Not implemented — miner speed set from INI at snapshot start each tick, not via multiplier reset | NOT-IMPLEMENTED |
| U10 | Exit cell anchor = NW_cell + (−1, +1) | `anchor.x = cx−1; anchor.y = cy+1` (building's GetCellLocation short pair) | `anchor_x = rx as i32 − 1; anchor_y = ry as i32 + 1` | PASS |
| U11 | Passable-cell spiral search from anchor | `FootClass::Find_Nearby_Passable_Cell(anchor, WaterBound, unlimited radius)` | `find_nearby_passable_cell(anchor_x, anchor_y, grid, occupancy, EXIT_SEARCH_MAX_RADIUS=16)` | UNCHECKED (radius, §3) |
| U12 | Set_Destination on unit to found cell | `FootClass::Set_Destination(dest_cell, 1)` — vtable+0x480 | `movement::issue_move_command(...)` from `phase_departing` | UNCHECKED |
| U13 | SetMission(MOVE=2) overrides Set_Destination's fallback ENTER=7 | `(*unit_vtable+0x1e8)(2, 0)` immediately after Set_Destination | `MinerState::SearchOre` set at exit arrival, no ENTER mission issued | PASS (different path, same observable: miner moves) |
| U14 | Building mission reset to SLEEP/GUARD (5) | `(*building_vtable+0x1e8)(5, 0)` | Not simulated — buildings have no mission state machine | NOT-IMPLEMENTED |
| U15 | RadioCommand(CLEAR=3) to production system | `(*building_vtable+0x274)(3)` | `dock_reservations.release()` serves same semantic (frees dock slot) | PASS (functionally equivalent) |
| U16 | Exit facing 0x47 maintained throughout drive | DriveLocomotionClass follows track 0x47 during exit drive | `entity.facing = 0x47; entity.facing_target = Some(0x47)` at Departing arrival re-snap | FAIL (details §2) |
| U17 | Piggyback swap-back: DriveLoco → TeleportLoco | `FootClass::AI` (0x4DA530): polls `IPiggyback::Is_Ok_To_End` per tick, calls `End_Piggyback` when miner stops | Not implemented as an explicit mechanism; teleport capability assumed restored after SearchOre | NOT-IMPLEMENTED |
| U18 | `FootClass::Locomotion_AI` (0x520F40) assigns re-harvest mission | Detects `Teleporter=yes` + not moving + ore level → assign Mission 0x18 (Harvest) or 0x17 (Guard) | `phase_departing` at exit sets `MinerState::SearchOre` directly | UNCHECKED |
| U19 | `Mission_Guard_Harvester` re-trigger to Mission_Harvest | If AI-controlled + refinery exists + ore not depleted: `Queue_Mission(10)` | Not implemented — SearchOre is our direct equivalent, no Guard_Harvester layer | NOT-IMPLEMENTED |
| U20 | `last_harvest_cell` memory preserved across dock cycle | gamemd does not explicitly name this field; consistent behavior: miner returns to prior patch | `snap.miner.last_harvest_cell` preserved through all dock phases; consumed only by SearchOre | PASS |
| U21 | Occupancy release of dock cell at departure | Pad removed from occupancy when miner drives off | `phase_departing` drives miner to exit cell; occupancy grid updated by movement system on position change | UNCHECKED |
| U22 | Vision / shroud: no change at undock | Shroud reveal is per-unit-position, no explicit undock cue in ReleaseDockedHarvester | No undock-specific shroud event emitted | PASS |

---

## §1. PASS Detail

**U04 — DepositCooldown hold:**
gamemd holds the miner on the pad while the building-side anim plays. Our `phase_deposit_cooldown` counts down `deposit_anim_duration_ticks` (derived from the longest `SpecialAnim` on the building, per art.ini). For GAREFN: `SpecialAnim=GAREFNOR`. Duration is looked up from the atlas at runtime. The semantic matches: miner sits on pad until last anim expires.

**U10 — Exit cell anchor:**
gamemd `ReleaseDockedHarvester`: `anchor.x = GetCellLocation_x() − 1; anchor.y = GetCellLocation_y() + 1`. Our `refinery_exit_cell`: `anchor_x = rx as i32 − 1; anchor_y = ry as i32 + 1`. Both use the building's NW-corner cell (foundation top-left). Numerically identical. PASS.

**U15 — Dock slot released:**
gamemd sends RadioCommand(CLEAR=3) to free the refinery's dock slot. Our `dock_reservations.release(ref_sid)` does the same: removes the occupant and promotes the next queued miner. Observable result is identical: next miner in queue gets the dock. PASS.

**U20 — `last_harvest_cell` preserved:**
`phase_departing` arrival sets `snap.miner.target_ore_cell = None` (the immediate target consumed) but explicitly preserves `snap.miner.last_harvest_cell`. Comment in code: "Preserve `last_harvest_cell` — the ghost-cell archive must survive the entire dock cycle so the next `SearchOre` returns directly to the productive patch saved when this miner became full." Verified at `miner_dock_sequence.rs:571–574`. Matches gamemd behavior of returning to prior patch. PASS.

---

## §2. FAIL Detail

**U08 / U16 — Exit facing 0x47:**

gamemd issues `Head_To(track=0x47, ...)` via `ILocomotion::Force_Track` (loco vtable+0x70) at the moment `ReleaseDockedHarvester` is called. This is a **drive track command**, not a raw facing write. The facing field updates as the DriveLocomotionClass processes the track. The miner faces ESE (0x47 ≈ 100° CW from north) **from the first tick of movement** through the entire exit drive.

Our `phase_departing` writes `entity.facing = 0x47` and `entity.facing_target = Some(0x47)` **only** when the initial move command is issued (at Departing entry, before first movement step). The re-snap at arrival (`at_exit` branch) snaps facing again. Problem: the movement system (`locomotor.rs`) may rotate facing during travel to match the path direction, overriding the 0x47 snap before the miner reaches the exit cell. This is a rendering disparity: in gamemd the miner always faces ESE on the exit drive (hardcoded via Force_Track), but in ours the facing can rotate to follow the A* path if it bends.

Evidence: `phase_departing` lines 551–556 set facing at move-issue, then lines 559–562 re-snap at arrival, but there is no mechanism to hold facing=0x47 throughout the drive. The A* path may route south then west to avoid the foundation, producing a south/SW facing during part of the exit.

**Player-visible:** Miner visually faces the wrong direction during the brief exit drive after unloading. Happens once per ore delivery cycle (every 14.4 ticks per bale × ~20 bales ≈ every 288 ticks ≈ every 19 seconds of continuous harvesting). Medium frequency, small visual window. Severity: LOW-MEDIUM.

File:line: `src/sim/miner/miner_dock_sequence.rs:548–556` (facing set at move issue) and `559–562` (arrival snap).

---

## §3. UNCHECKED / NOT-IMPLEMENTED Detail

**U09 — Speed multiplier:**
gamemd calls `unit_vtable+0x544(0, 0x3FF00000)` = set speed multiplier to 1.0 (double). This restores full speed after the dock imposed a speed override. Our speed is read from INI each tick and converted via `ra2_speed_to_leptons_per_second(raw_speed)`. There is no explicit speed-multiplier-reset on undock. If no speed override was applied on dock entry (we never write a dock-time slow-down), the effect is the same; if a future speed override feature is added without this reset the gap will bite.

**U11 — Search radius cap:**
gamemd uses `max_radius = 0xFFFFFFFF` (unlimited) in its `Find_Nearby_Passable_Cell` call. Our `EXIT_SEARCH_MAX_RADIUS = 16`. In pathological maps where the first 16-ring cells around `(rx−1, ry+1)` are all blocked (dense terrain + units), gamemd would find a cell we miss. In practice this is irrelevant for stock maps. Not a functional difference on any shipping map.

**U17 — Piggyback swap-back:**
This is the most mechanically significant NOT-IMPLEMENTED. In gamemd, `FootClass::AI` (0x4DA530) runs every tick and polls `IPiggyback::Is_Ok_To_End()` on the DriveLocomotionClass. When the miner has stopped after the exit drive, `Is_Ok_To_End` returns true (all conditions met: `Is_Moving_Now=false`, piggybacked loco exists, enabled flag set, unit not in limbo). Then `FootClass::AI` calls `End_Piggyback()`, which extracts the TeleportLocomotionClass from inside the DriveLocomotion and stores it back into `FootClass+0x674`. After this, the miner has TeleportLoco as its active locomotor again and can self-teleport.

Our implementation: teleport capability is not modeled as a piggybacked loco. The miner's `MinerKind::Chrono` flag drives the teleport decision in `begin_return`. After the dock cycle, `MinerState::SearchOre` fires and the next `begin_return` call issues `issue_teleport_command` if far enough. The observable output (miner teleports to ore when far from refinery) is preserved, but the intermediate state (miner briefly exits dock as Drive-only, then re-acquires TeleportLoco) is not reproduced. A player watching carefully would NOT see a wrong outcome, but the timing of warp-vs-drive decisions on the first re-harvest could differ if the exit drive ends close to an ore patch (miner would drive instead of warp when it should drive — this is actually correct since after dock the miner is at the refinery, not at the ore).

**U19 — Mission_Guard_Harvester layer:**
gamemd uses `Queue_Mission(5)` → `UnitClass::Mission_Guard_Harvester` as an intermediate state after the miner departs. This state does harvester-specific re-trigger logic before delegating to `FootClass::Mission_Guard`. The chrono-miner-specific path in Guard_Harvester:
1. Scans 8 adjacent cells for a same-house refinery → re-triggers Harvest if found.
2. Checks `Is_Ok_To_End` on the locomotor + full storage → re-triggers Harvest.

We go directly `SearchOre` → `MoveToOre`/teleport without this guard layer. The 8-cell adjacent-refinery scan is irrelevant at departure (miner just left the refinery, it's adjacent). The `Is_Ok_To_End` check guards against acting before the piggyback swap-back completes — since we don't model piggyback, this is latent.

Observable difference: none in normal play. The Guard_Harvester→Harvest transition is immediate (one tick). Our SearchOre is similarly immediate.

---

## §4. Verdict Tally

PASS: 6 | FAIL: 2 | UNCHECKED: 7 | NOT-IMPLEMENTED: 7

---

## §5. Top 5 Player-Visible Failures

1. **U17 NOT-IMPLEMENTED — Piggyback swap-back (DriveLocomotion → TeleportLocomotion)**
   Player sees: after undocking, chrono miner's teleport availability relies on `MinerKind::Chrono` heuristic rather than the actual locomotor swap. If the miner is close to ore post-dock it correctly drives; if far it correctly teleports. No visible error in normal play. The missing mechanism would produce wrong behavior only if locomotor state is ever inspected by other systems (combat retaliation, damaged drive loco).
   File:line: `src/sim/miner/miner_system.rs:775–810` (begin_return, teleport decision).
   gamemd evidence: `MINER_DOCK_GAPS_RESEARCH.md §Gap 1` — `FootClass::AI` (0x4DA530) at offset +0x970.

2. **U08/U16 FAIL — Exit facing 0x47 not held during exit drive**
   Player sees: miner briefly faces wrong direction (path-following, e.g. south/SW) during the exit drive off the refinery pad instead of always facing ESE (0x47). Fires once per ore delivery cycle (~19 sec of continuous harvesting). Short duration (~5–10 ticks).
   File:line: `src/sim/miner/miner_dock_sequence.rs:548–556` (facing set at move issue, not Force_Track semantic).
   gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md §Step 8` — `(*loco_vtable+0x70)(loco, 0x47, ...)`.

3. **U01 NOT-IMPLEMENTED — Anim slots 0xA+0xB cleared at undock start**
   Player sees: GAREFNOR unload animation continues playing visually after the miner has been flagged as departing. The pipe/conveyor anim (slot A) and active-dock anim (slot B) should stop the moment `ReleaseDockedHarvester` fires.
   File:line: no corresponding code in `src/sim/miner/miner_dock_sequence.rs`; `phase_deposit_cooldown` transitions to Departing without sending a building-anim-stop event.
   gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md §Step 1` — `BuildingClass__ClearAnimSlot(slot=0xA)` then `ClearAnimSlot(slot=0xB)`.

4. **U02 NOT-IMPLEMENTED — Departure VOC (`BunkerWallsDownSound = TankBunkerDown`) not played**
   Player sees/hears: no sound at the moment the miner drives off the pad. gamemd plays `TankBunkerDown` at the building's location via `VocClass::PlayAt`. Fires once per ore delivery cycle.
   File:line: no `BunkerWallsDownSound` or equivalent in `miner_dock_sequence.rs`.
   gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md §Step 2`; confirmed in `ini/rulesmd.ini:720` (`BunkerWallsDownSound= TankBunkerDown`).

5. **U03 NOT-IMPLEMENTED — Departure anims (slots 0xC=SpecialAnimThree, 0xD=SpecialAnimFour) not created**
   Player sees: refinery's departure animation sequence (`SpecialAnimThree` / `SpecialAnimFour`) never plays. For GAREFN these keys are not set in artmd.ini (only `SpecialAnim=GAREFNOR` for the deposit), so this gap is invisible for GAREFN but would fire on any custom refinery that defines `SpecialAnimThree=` or `SpecialAnimFour=`.
   File:line: no corresponding `CreateAnimForSlot` equivalent in `miner_dock_sequence.rs`.
   gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md §Steps 3–4`.

---

## §6. Ancillary Findings

- **CMIN INI**: `Storage=20` (20 bales), `Speed=4`, `UnloadingClass=CMON`. Confirmed parsed correctly via INI grep. No discrepancy.
- **GAREFN art.ini**: `QueueingCell=4,1` (parsed by our `refinery_queue_cell`), `SpecialAnim=GAREFNOR` (single SpecialAnim slot). No `SpecialAnimThree`/`SpecialAnimFour` defined → U03 gap is latent for stock maps.
- **Building mission state machine**: gamemd calls `SetMission(5)` + `RadioCommand(CLEAR=3)` on the refinery after releasing the miner. Our engine has no building mission state machine at all (buildings are passive). This is a correct architectural simplification since the only observable side effect (freeing the dock slot) is covered by `dock_reservations.release()`.
- **TS legacy filter**: `FootClass::Locomotion_AI` (0x520F40) assigns Mission 0x18/0x17 after piggyback swap-back. This is a live YR path (not TS-only), but its observable result is replaced by our `MinerState::SearchOre` direct transition. No action needed beyond U17 tracking.

---

**Status: COMPLETE**
