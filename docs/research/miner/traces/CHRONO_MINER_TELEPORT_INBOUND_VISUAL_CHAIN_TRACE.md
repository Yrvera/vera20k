# Chrono Miner — Teleport Inbound Visual Chain Trace

**Slot:** 2 of 5 (parallel swarm, continuation)
**Mechanic:** Full inbound warp visual chain — far-miner warp decision → WarpOut anims →
  sounds → instant relocate → BeingWarped → Drive restore → pad drive → dock FSM
**Scenario:** CMIN at cell (60,60), GAREFN anchor at (10,10), ~52 cells distance > 50-cell
  threshold. Miner has 20 bales. Inbound return trip only.
**Date:** 2026-05-20
**Binary ref:** gamemd.exe YR 1.001
**Closes:** CHRONO_MINER_TELEPORT_DOCK_APPROACH_TRACE.md (2026-05-19) PARTIAL gap
**Docs consulted:** CHRONO_MINER_SYSTEM_OVERVIEW.md, CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md,
  PHASE0_CHRONO_DELAY_FORMULA_MATH_GHIDRA_REPORT.md, CHRONO_WARP_VISUAL_RENDERING.md,
  RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md,
  CHRONO_MINER_TELEPORT_DOCK_APPROACH_TRACE.md
**Rust files:** src/sim/miner/miner_system.rs, src/sim/movement/teleport_movement.rs,
  src/sim/miner/miner_dock_sequence.rs, src/app_instances/units.rs
**INI:** ini/rulesmd.ini [General], [CMIN]; ini/artmd.ini [GAREFN]

---

## Iron Law Reminder

PASS requires literal numerical equality between our output and gamemd's. If both values
were not computed, the stage is UNCHECKED, not PASS. Pad-PASS is forbidden.

---

## Prior PARTIAL Gap — What Was Unverified

The 2026-05-19 trace left 3 UNCHECKED stages:
1. **Stage 3** — AnimClass flag `0x600` (centered + loop) vs our `translucent: true` in WorldEffect
2. **Stage 4c** — Distance calculation: 2D (our code) vs 3D Euclidean (gamemd)
3. **Stage 7 (render path)** — Whether renderer reads teleport_state to apply 50% translucency

Additionally: the overview doc (§5, step 3) says "WarpAway" but the rendering doc (§6) says
"WarpOut" is spawned twice for self-teleport. The overview's step 3 text was **WRONG** (doc
has since been corrected in §6 and the table at §5's detailed breakdown). CHRONO_WARP_VISUAL_RENDERING.md
§6 is the authoritative source: self-teleport spawns WarpOut (Rules+0x33C) twice.

---

## Stage Table

### Stage A: Warp-far decision — distance reference and formula

**gamemd:** Miner→refinery distance measured from miner position to dock-adjacent queue cell
(Find_Nearby_Passable_Cell seed = `BuildingType->DockOffset + anchor`). Compared in leptons:
`dist_leptons <= ChronoHarvTooFarDistance * 256` (= 50 * 256 = 12,800 leptons). Far → warp.

**Our code (begin_return, miner_system.rs ~line 787):**
```rust
let center = refinery_center_cell_for_sid(sim, rules, rsid).unwrap_or(dock);
let threshold = config.too_far_threshold_chrono as u32;  // 50
let far_enough = cell_dist_sq((snap.rx, snap.ry), center) > threshold * threshold;
```
Reference point is building **center cell**, not queue cell. For GAREFN (4×3) the center
cell is `(rx+2, ry+1)` = `(12, 11)`. The queue cell is `(14, 11)`. Distance from miner
(60,60) to center (12,11) ≈ 69 cells. Distance to queue cell (14,11) ≈ 67 cells. Both
exceed 50 → no threshold-boundary error in this concrete scenario.

**PASS (concrete scenario):** At 52+ cells either reference gives far_enough=true.
**FAIL (boundary case, inherited from prior trace):** For minerss at 49-53 cells, the
  2-cell shift in reference point produces wrong warp/drive decisions (fires on every
  harvest cycle near the boundary). Severity: MEDIUM. See prior trace Stage 1.

---

### Stage B: Queue cell computation — departure target

**gamemd:** Self-teleport sends the miner to the queue cell. Queue cell from the
`BuildingClass::Receive_Radio case 0x0E` radio handshake is:
  `anchor + (3, 1)` — hardcoded, NOT from `QueueingCell=` INI.
  `ObjectClass::Get_Cell_Packed` returns building anchor → result = `(rx+3, ry+1)` = `(13, 11)`.
  (Verified: RECEIVE_RADIO_CASE_0x0E §3, assembly at 0x43C2D0, `*psVar5 + 3`, `psVar5[1] + 1`.)

However, in State 2 RETURN the chrono miner uses `Set_Destination` with the
**BuildingType->DockOffset** cell (`BuildingTypeClass+0x1618/+0x161C`) as the teleport target,
NOT the CAN_DOCK radio-computed cell. The CAN_DOCK handshake runs AFTER the miner arrives
at the teleport destination.

**art.ini [GAREFN]:** `QueueingCell=4,1` — this is the DockOffset used by State 2 RETURN
when computing the teleport destination via `BuildingType->DockOffset`.

**Our code (refinery_queue_cell, miner_dock_sequence.rs line 65):**
```rust
pub(super) fn refinery_queue_cell(rx, ry, width, height, queueing_cell: Option<(u16,u16)>) {
    if let Some((qx, qy)) = queueing_cell { (rx + qx, ry + qy) }
    else { (rx + width, ry + height / 2) }
}
```
With `QueueingCell=Some((4,1))` from art.ini → returns `(rx+4, ry+1)` = `(14, 11)`.

**FAIL:** Teleport destination is `(14, 11)` in our code vs `(13, 11)` in gamemd's State 2
RETURN computation from DockOffset. The prior trace Stage 10 correctly identified X offset 4
vs 3 from anchor, but attributed it to the CAN_DOCK radio path. The root cause is:
- State 2 RETURN uses `BuildingType->DockOffset` (populated from `QueueingCell=4,1`) → `(rx+4, ry+1)`
- gamemd's State 2 actually reads `BuildingTypeClass+0x1618` / `+0x161C` (the DockOffset).
  `QueueingCell=4,1` in artmd.ini stores (4,1) → `anchor+(4,1)` = `(14,11)`.

**RESOLUTION — The prior trace Stage 10 finding may be wrong.** The art.ini has
`QueueingCell=4,1`, which would give `(rx+4, ry+1)` = `(14,11)` from DockOffset. The CAN_DOCK
radio handshake hardcodes `+3` for the arrival-confirmation queue cell, but the TELEPORT
DESTINATION uses the DockOffset. These are two different cells serving two different roles.
Our teleport target `(14,11)` from QueueingCell may be correct for the teleport step.
The `(13,11)` applies only to the radio protocol's MOVE_TO_CELL message after arrival.

**UNCHECKED:** The exact address in Mission_Harvest State 2 that reads DockOffset to set
the teleport destination. This requires Ghidra (offline this session). The art.ini evidence
(`QueueingCell=4,1`) supports our `(14,11)` target as consistent with the DockOffset path;
it does NOT rule out the possibility that gamemd's State 2 uses a different cell formula.

---

### Stage C: WarpOut anim spawned at departure

**gamemd (TeleportLocomotionClass::InitiateWarp 0x719400, verified in CHRONO_WARP_VISUAL_RENDERING §6):**
- `AnimClass::Constructor(Rules+0x33C, &srcCoords, 0, 1, 0x600, 0, 0)`
- `Rules+0x33C` = `WarpOut=WARPOUT;WAKE2` (from ini/rulesmd.ini line 549)
- Flags `0x600` = `0x200` (center sprite on coords) | `0x400` (documented as unused).
  The `;WAKE2` suffix is a secondary anim (expanding ring), drawn via a special Z-buffered quad path.

**Our code (spawn_warp_effects, miner_system.rs line 855):**
```rust
sim.world_effects.push(WorldEffect {
    shp_name: anim_interned,  // rules.general.warp_out.name = "WARPOUT"
    translucent: true,
    // ...
});
```
- Anim name: uses `warp_out.name` which parses the primary name before `;`. Result: "WARPOUT". PASS.
- `translucent: true`: gamemd passes flag 0x600, not a translucency flag (bits 1-2 of draw flags
  control translucency; 0x600 = center + unused). The WarpOut anim type itself has
  `Translucent=yes` in art.ini making it semi-transparent via its AnimType flag, not via
  the Constructor call flags. Our `translucent: true` on WorldEffect achieves the same output.
- Secondary anim (WAKE2 ring): the `;WAKE2` suffix is NOT handled by our code. We parse only
  the primary name before the semicolon. WAKE2 is the expanding ring overlay.

**FAIL (visual):** The `;WAKE2` secondary anim (expanding ring) is not rendered.
  Player sees: departure and arrival have the WARPOUT shimmer but no expanding ring pulse.
  Fires: every warp (= every far-harvest cycle). Frequency: very common.

**UNCHECKED:** Whether the centering flag `0x200` on the Constructor call is replicated by
our WorldEffect.sub_x/sub_y = CELL_CENTER_LEPTON. The intent matches (center on cell),
but the exact pixel centering depends on the anim's SHP dimensions. Cannot compare without
rendering output.

---

### Stage D: ChronoOutSound at departure

**gamemd (InitiateWarp, after departure anim spawn):**
`VocClass::PlayAt(TypeClass+0x578 or Rules+0x21C)` at departure coordinates.
For CMIN: TypeClass+0x578 = ChronoOutSound = ChronoMinerTeleport (per-unit override).
(ini/rulesmd.ini [CMIN] line 7366: `ChronoOutSound=ChronoMinerTeleport`)

**Our code (spawn_warp_effects, miner_system.rs ~line 893):**
```rust
let chrono_out = obj.and_then(|o| o.chrono_out_sound.clone())
    .or_else(|| rules.general.chrono_out_sound.clone());
```
Per-unit wins → ChronoMinerTeleport. Emitted as `SimSoundEvent::ChronoTeleport { rx: depart.0, ry: depart.1 }`.

**PASS:** Per-unit override → global fallback chain matches. Sound name correct.
  Position at departure cell correct.

---

### Stage E: Instant relocation (position snap + occupancy swap)

**gamemd (Phase 0 InitiateWarp, steps 8+11):**
1. Unmark from old cell (remove occupancy at departure)
2. Mark at new cell (add occupancy at destination)
3. For Harvester=yes (CMIN): `timer=0, BeingWarped=0` (instant, no post-warp lock)

**Our code (tick_teleport_movement, TeleportPhase::Relocate):**
```rust
entity.position.rx = teleport.target_rx;
entity.position.ry = teleport.target_ry;
occupancy.move_entity(old_rx, old_ry, target_rx, target_ry, ...);
if teleport.being_warped_ticks == 0 { finished.push(id); }
```
- For is_harvester=true: `being_warped_ticks=0` → finishes in same tick. PASS.
- Occupancy swap: unconditional move, no occupant check. Matches gamemd. PASS.

**NOT-IMPLEMENTED:** Stop targeting (FUN_0070D4A0) and anim-detach steps not in
  `tick_teleport_movement`. Attackers keep lock on departure cell post-warp.

**NOT-IMPLEMENTED:** Bridge flag `TechnoClass->IsOnBridge (+0x8C)` set from destination
  cell flags (gamemd Phase 0 step 10) — we have no equivalent.

**PASS (position + occupancy):** Snap and swap are correct for harvester instant-warp.

---

### Stage F: WarpOut anim spawned at arrival + ChronoInSound

**gamemd (InitiateWarp, steps 12 and 15):**
- Play ChronoInSound at arrival coords: `VocClass::PlayAt(TypeClass+0x574)` = ChronoMinerTeleport
- Spawn `AnimClass::Constructor(Rules+0x33C, &destCoords, 0, 1, 0x600, 0, 0)` at arrival cell

**Our code:** Both are present in `spawn_warp_effects` (arrival WorldEffect + ChronoInSound event).
The ordering in our code is departure effects first, then arrival — gamemd plays ChronoInSound
(step 12) BEFORE spawning the arrival anim (step 15). Our code emits ChronoInSound after both
anims. Both sounds/anims are queued within the same tick, so ordering is subframe-only.

**PASS (structural):** Arrival anim (WarpOut) and ChronoInSound emitted at arrival coordinates.
Same WAKE2 secondary anim gap applies (see Stage C). Sound name correct.

---

### Stage G: BeingWarped translucency — chrono miner stays fully opaque

**gamemd:** For Harvester=yes: timer=0, BeingWarped(+0x271)=0 immediately after Phase 0.
  Unit appears fully opaque from first frame at destination.
  The WarpOut anim plays OVER the opaque unit (the shimmer is the anim, not the unit's draw flag).

**CHRONO_WARP_VISUAL_RENDERING.md §6 CRITICAL CORRECTION:**
"The unit itself is NOT rendered with translucency during chrono teleport. The warp effect
is purely the WarpOut animation overlay." BeingWarped does NOT add draw flag 0x2004 for
translucency in self-teleport. The overview doc's claim that "flag 0x2004 = 50% translucency"
was documented as referring to TechnoClass::Draw, but the rendering doc confirms the
locomotor's `Visual_Character` returns 0 (not cloaked) → no translucency on unit itself.

**Our code (units.rs line 190):**
```rust
// Chrono teleport doesn't tint the unit — the visual effect is the
// WarpOut animation overlay; the unit itself stays fully opaque.
let alpha: f32 = 1.0;
```
Renderer hardcodes alpha=1.0 regardless of teleport_state. This matches gamemd.

**PASS:** Unit rendered fully opaque during warp, matching gamemd. The translucency doc-comment
in teleport_movement.rs (line 59: "50% translucent") is misleading but has no code effect
since the renderer ignores teleport_state.being_warped_ticks for unit alpha.

**UNCHECKED (non-harvester path):** For Chrono Legionnaire (non-harvester), BeingWarped=1.
  Our code still uses alpha=1.0. The renderer should draw non-harvester teleporters at 50%
  alpha during ChronoDelay phase. This is a separate bug (not in this scenario's scope).

---

### Stage H: Locomotor swap — Teleport → Drive restore

**gamemd:** For Harvester=yes, timer=0 → `Is_Ok_To_End=true` immediately after warp.
  `FootClass::AI` swaps back to `DriveLocomotionClass` (same tick or next tick).

**Our code (tick_teleport_movement cleanup, line 241):**
```rust
entity.teleport_state = None;
if let Some(ref mut loco) = entity.locomotor {
    if loco.is_overridden() { loco.end_override(); }
}
```
Drive restored in same tick as warp completion (being_warped_ticks=0 → `finished.push(id)` in
Relocate tick → cleanup in same tick call). This matches or is 1 tick faster than gamemd
depending on FootClass::AI tick ordering. Difference: ≤1 tick (subframe-visible at most).

**PASS (structural):** Drive locomotor restored on warp completion, matching intent.

---

### Stage I: Drive to pad — post-warp dock approach

**gamemd (Mission_Harvest State 3 + UnitClass::Mission_Enter):**
After Drive restored, miner drives to queue cell `(13, 11)` (from CAN_DOCK radio = anchor+(3,1)).
Radio handshake: NEED_TO_MOVE(0x13) → MOVE_TO_CELL(0x12) with `(13,11)` → ENTER_DOCK(0x18)
→ TIMING_SYNC(0x16) → SetSpeed(0x4000) → miner drives onto pad cell.

**Our code (miner_dock_sequence.rs phase_approach):**
- No radio handshake. Direct move to `pad` cell via movement_target.
- Queue cell mismatch: our teleport lands at `(14,11)`, gamemd's radio sends MOVE_TO_CELL
  with `(13,11)`. Miner would need to drive one extra cell.

**FAIL (queue cell X offset):** Inherited from prior trace Stage 10. Teleport destination
  is `(14,11)` (ours, from QueueingCell=4,1) vs radio-sent queue cell `(13,11)` (gamemd,
  anchor+3). Even if our teleport destination is correct per DockOffset, the subsequent
  radio-protocol-driven MOVE_TO_CELL targets `(13,11)`, one cell west. Our dock FSM
  issues no such radio correction.

**NOT-IMPLEMENTED:** Full radio protocol (NEED_TO_MOVE, MOVE_TO_CELL, ENTER_DOCK,
  TIMING_SYNC / SetSpeed(0x4000)) is absent. We skip directly to `movement_target = pad`.

---

### Stage J: RefineryDockPhase Approach → Linked

**gamemd (Mission_Enter → dock-pad entry):**
Harvester drives to pad cell (inside foundation) under Mission_Enter after ENTER_DOCK.
SetSpeed(0x4000) sets locomotor speed during approach (TIMING_SYNC(0x16)).

**Our code (phase_approach, miner_dock_sequence.rs):**
- On reserve granted: moves to pad directly, transitions Approach → Linked.
- No SetSpeed(0x4000) equivalent. We use the standard movement speed.

**FAIL (structural):** Missing TIMING_SYNC/SetSpeed on approach. Player-visible if approach
  speed differs from standard speed. Fires every dock cycle.

---

## Stage Summary Table

| Stage | Description | Result |
|-------|-------------|--------|
| A | Far decision (concrete scenario — both refs give far_enough) | PASS |
| A-b | Far decision (boundary case, center vs queue cell) | FAIL (inherited) |
| B | Queue cell / teleport destination | UNCHECKED |
| C | WarpOut anim at departure (primary name correct) | PASS |
| C-b | WAKE2 secondary anim (ring) not rendered | FAIL |
| D | ChronoOutSound at departure | PASS |
| E | Instant relocation — position + occupancy | PASS |
| E-b | Stop-targeting / anim-detach on warp | NOT-IMPLEMENTED |
| E-c | IsOnBridge flag from dest cell | NOT-IMPLEMENTED |
| F | WarpOut anim + ChronoInSound at arrival | PASS |
| G | BeingWarped translucency — opaque for harvester | PASS |
| G-u | Non-harvester translucency (alpha=1.0 bug, out of scope) | UNCHECKED |
| H | Drive locomotor restore after warp | PASS |
| I | Radio protocol / correct queue cell for MOVE_TO_CELL | FAIL |
| J | SetSpeed(0x4000) on TIMING_SYNC | FAIL |

---

## Verdict Tally

**PASS: 8 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 3**

---

## Top 5 Player-Visible Failures

1. **Stage C-b — WAKE2 expanding ring not rendered at departure/arrival**
   Player sees: chrono warp shows only the WARPOUT shimmer, no expanding ring pulse at
   each endpoint. Fires every warp (every far-harvest cycle, very common in Allied games).
   Our code: `spawn_warp_effects` parses only the primary name before `;` — "WARPOUT" only.
   gamemd: `WarpOut=WARPOUT;WAKE2` — Constructor called with the full INI value; the secondary
   is rendered via AnimClass::DrawIt's Z-buffered ring path.
   (CHRONO_WARP_VISUAL_RENDERING.md §6, InitiateWarp at 0x719400.)

2. **Stage I — Radio protocol absent: miner drives wrong queue cell after warp**
   Player sees: miner teleports to `(14,11)` but the radio-correct queue cell is `(13,11)`.
   Without the MOVE_TO_CELL(0x12) redirect, our miner drives directly to the pad from `(14,11)`,
   one cell east of where gamemd places the approach queue. In constrained maps this can cause
   pathfinding issues or visible extra pathing before dock.
   Our code: `miner_dock_sequence.rs` — no radio handshake, moves directly to pad.
   gamemd evidence: RECEIVE_RADIO_CASE_0x0E §3, anchor+(3,1) hardcoded at 0x43C2D0.

3. **Stage J — Missing TIMING_SYNC/SetSpeed(0x4000) on dock approach**
   Player sees: miner drives onto dock pad at standard speed instead of the synchronized
   speed (0x4000) set by TIMING_SYNC. May look visually slower or faster than expected,
   and dock animation sync is off if refinery expects the 0x4000 rate.
   Fires every dock cycle (harvester returns to refinery every 60-90 seconds).
   Our code: no SetSpeed equivalent in phase_approach.
   gamemd evidence: RECEIVE_RADIO_CASE_0x0E §7, UnitClass::Receive_Radio case 0x16 at 0x737430.

4. **Stage E-b — Stop-targeting not called on warp**
   Player sees: units attacking the miner continue targeting departure cell after warp.
   Fires whenever miner is under attack while warping (common in mid-map combat scenarios).
   Our code: `tick_teleport_movement` TeleportPhase::Relocate has no FUN_0070D4A0 call.
   gamemd evidence: CHRONO_MINER_SYSTEM_OVERVIEW §5, Phase 0 step 1.

5. **Stage A-b — Distance threshold uses building center, not queue cell**
   Player sees: miner 49-51 cells from refinery may drive instead of warp (or vice versa).
   Error window ≈ 2 cells for a 4×3 building. Fires on every harvest cycle when miner is
   near the 50-cell ChronoHarvTooFarDistance boundary — common on mid-sized maps.
   Our code: `miner_system.rs ~line 787` — uses `refinery_center_cell_for_sid`.
   gamemd evidence: PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT §2.

---

## Adjacent Findings

**Doc inconsistency resolved:** CHRONO_MINER_SYSTEM_OVERVIEW.md §5 step 3 says "WarpAway" anim
is spawned at departure. CHRONO_WARP_VISUAL_RENDERING.md §6 (more detailed, verified from binary
byte search at InitiateWarp 0x719400) confirms WarpOut (Rules+0x33C) is used, NOT WarpAway.
The overview doc was incorrect; the rendering doc is authoritative. Our code correctly uses
`rules.general.warp_out.name`.

**Translucency doc-comment mismatch:** `teleport_movement.rs` doc-comment says "50% translucent"
during ChronoDelay. The renderer (`units.rs:190`) hardcodes `alpha=1.0`. For the chrono miner
(harvester, instant-warp) this is correct. For non-harvester teleporters, the renderer comment
is aspirational — the 50% translucency is NOT implemented in the renderer despite the doc.

**CMIN primary locomotor:** rulesmd.ini [CMIN] sets `Locomotor={4A582747-...}` (teleport CLSID).
The piggybacking pattern our code implements (Drive piggybacked under Teleport) mirrors the
gamemd architecture where Drive is stored under TeleportLoco's +0x48 and swapped post-warp.

---

## Status

PARTIAL — Stage B (teleport destination cell formula — State 2 RETURN's DockOffset reading vs
radio-hardcoded anchor+(3,1)) requires fresh Ghidra decompilation of Mission_Harvest State 2
to confirm which cell formula applies to the teleport SET_DESTINATION call. Ghidra was offline
this session.

All other stages assessed from existing docs, INI files, and Rust source.
