# MCV Deploy Facing Turn — In-Place Rotation Trace

**Scenario:** Allied AMCV at cell (50, 50), facing East (0x40). Player presses Deploy.
AMCV must pivot in place to South (0x80) before GACNST spawns.

**Mechanic:** AMCV deploy facing turn and gated unit-to-building swap.

**Traced against:** commit 103aee0 ("sim: MCV deploy facing turn + ConstructionYard undeploy gating")

**gamemd evidence:** `UnitClass__Deploy @ 0x007393C0` (decompiled this session),
`UnitClass__Mission_Deploy_Building @ 0x0073D630` (decompiled this session),
`Deploy_facing_calculator @ 0x00465D70` (decompiled this session),
`AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md` (pre-existing, verified)

**YR-active:** Yes — stock `[AMCV] DeploysInto=GACNST` drives this path;
no TS-legacy gating applies.

---

## Pipeline Stages

### Stage 1 — DeployMcv command dispatch

**Question:** Does the D keypress reach `deploy_mcv()` with the correct entity_id?

**Rust:** `app_input.rs:897` — `Command::DeployMcv { entity_id }` is pushed when entity
has `deploys_into.is_some()`. `world_commands.rs:477` dispatches to `deploy_mcv()`.

**gamemd:** `Mission_Deploy_Building` state 0 → state 1 → calls `UnitClass::Deploy()` every tick
while stopped. The player D keypress sets the unit into `Mission_Deploy_Building`.

**Verdict: PASS** — command is correctly dispatched when `deploys_into` is set. The
structural difference (one-shot command vs per-tick mission) is the root of later failures.

---

### Stage 2 — DeployFacing source: GACNST type, not AMCV

**Question:** Does Rust read deploy facing from the `DeploysInto` building type (GACNST),
not from AMCV itself?

**Rust:** `world_spawn.rs:636` — `let yard_obj = rules.object(&yard_type)?;` then
`yard_obj.deploy_facing` at line 648. `object_type.rs:1019-1022` parses `[GACNST]
DeployFacing` with `(v.clamp(0,7) as u8) << 5`, defaulting to `0x80` when absent.

**gamemd:** `UnitClass__Deploy @ 0x007395BA-C6` loads `unit_type+0x404` (GACNST pointer),
passes to `Deploy_facing_calculator @ 0x00465D70` which returns `*(param+0xEDC)`.
`BuildingTypeClass` constructor (`0x0045DEEC`) sets `+0xEDC = 0x80` by default.
Stock `rulesmd.ini` has no `[GACNST] DeployFacing=` line → default 0x80.

**Numerical equality:**
- gamemd: `target = 0x80` (verified from `0x0045DEEC`, `rulesmd.ini:11622`)
- Rust: `deploy_facing = 0x80` (default; test `parse_construction_yard_and_deploy_facing` confirms)

**Verdict: PASS** — identical source, identical value 0x80.

---

### Stage 3 — Initial facing comparison gate

**Question:** Does Rust correctly detect that AMCV (facing 0x40) does not match
deploy target (0x80) and refuse to spawn the ConYard?

**Rust:** `world_spawn.rs:711` — `if source_facing != deploy_facing { … return true; }`
Compares stored `u8` facing directly.

**gamemd:** `UnitClass__Deploy` at `0x007395D2-EB`:
```c
puVar7 = (uint *)RateTimer__Current();
if (((*puVar7 >> 7) + 1 >> 1 & 0xff) != uVar6) { … return 1; }
```
Reads 16-bit interpolated facing from locomotor's `FacingClass`, rounds to 8-bit:
`((current_16bit >> 7) + 1) >> 1 & 0xFF`.

**Numerical equality:** For a stopped unit at facing 0x40 (= 16-bit 0x4000), gamemd rounds:
`((0x4000 >> 7) + 1) >> 1 & 0xFF = (0x80 + 1) >> 1 & 0xFF = 0x40`. Matches stored u8.
For a unit stopped exactly at a facing byte boundary, the formulas agree.

**Verdict: PASS** (for the stopped-unit case). The comparison correctly blocks deploy when
AMCV is at 0x40 and target is 0x80.

*Note: Mid-rotation rounding semantics differ (gamemd uses 16-bit interpolated value;
Rust uses snapped u8). This gap is masked by the FAIL in Stage 4 but would matter
if gradual rotation were implemented.*

---

### Stage 4 — In-place rotation via ROT: CRITICAL FAIL

**Question:** Does the MCV rotate gradually toward 0x80 at its ROT=5 rate?
Are ticks counted? Is the animation visible?

**gamemd mechanism:** `UnitClass__Deploy` at `0x007395EF-65F`, when facing mismatch:
```c
// Call ILocomotion::Set_Desired_Heading (vtable+0x4C) with target << 8
(**(code **)(*piStack_38 + 0x4c))(…, (uint)uVar6 << 8);
// Set mission state
(**(code **)(param_1->vtable + 0x274))(3);
// Return 1 (still in progress)
return 1;
```
`Set_Desired_Heading` calls `RateTimer__Set`, which sets timer duration =
`|delta| / rate` binary frames. For ROT=5 in gamemd: rate is read as the unit's
ROT field passed into the FacingClass. Duration = `|0x80 - 0x40| / 5` = 64 / 5 = 12.8
→ 13 binary frames at 15Hz = ~867ms = ~26 sim ticks at 30Hz.

`Mission_Deploy_Building` state 1 is called **every tick** while the locomotor is stopped.
It checks `ILocomotion[4]` (Is_Moving) — while turning, this returns non-zero, so
state 1 re-calls `UnitClass::Deploy()` each tick until facing matches.

**Rust code at `world_spawn.rs:711-717`:**
```rust
if source_facing != deploy_facing {
    if let Some(entity) = self.entities.get_mut(stable_id) {
        entity.facing_target = Some(deploy_facing);
        entity.facing = deploy_facing;   // ← INSTANT SNAP — BUG
        entity.movement_target = None;
    }
    return true;
}
```

**Two bugs:**

**Bug 4a — Instant snap (FAIL):** `entity.facing = deploy_facing` at line 714 sets the
facing to 0x80 immediately in the same command processing tick. The MCV visually
jumps from East to South with no intermediate frames. The player sees a teleport,
not a rotation. gamemd plays ~26 ticks of smooth rotation.

**Bug 4b — No per-tick retry (NOT-IMPLEMENTED / FAIL):** After setting `facing_target`,
there is no mechanism to call `deploy_mcv()` again when `facing_target` clears. The
`facing_target` system in `movement_step.rs:190` drives rotation (and `movement_tick.rs:582`
invokes it) — but only in the context of `movement_target` path following. There is no
MCV-specific state that tracks "this unit is turning to deploy" and re-calls deploy when
the turn completes. The ConYard never spawns unless the player re-issues the deploy command.

**Test evidence:** `deploy_mcv_waits_for_target_building_deploy_facing` (deploy_tests.rs:404)
asserts `entity.facing == 0x80` after command — confirming the snap bug is present
and passing as "correct" in the test suite. The test should assert `entity.facing == 0x40`
(unchanged) and `entity.facing_target == Some(0x80)`.

**gamemd ticks calculation:**
- ROT=5, delta=64 facing units, FacingClass rate=5: duration = 64/5 = ~13 binary frames
- At 15 binary fps → 13 frames = ~867ms → at 30 Hz sim = ~26 sim ticks
- Rust `rot_to_facing_delta(5, 33ms)` = ceil(5*256*15*33 / 360000) = ceil(1.76) = 2 per tick
- If Rust used gradual rotation: ceil(64/2) = 32 ticks → different from gamemd's ~26
- **Tick count UNCHECKED** because gradual rotation is not implemented at all

**Verdict: FAIL (Bug 4a: instant snap) + NOT-IMPLEMENTED (Bug 4b: no per-tick retry)**

---

### Stage 5 — No position delta during rotation

**Question:** Does the unit remain stationary (no lepton displacement) while rotating?

**Rust:** `world_spawn.rs:715` clears `movement_target = None`. The `handle_vehicle_rotation`
function in `movement_step.rs:217-218` skips lepton advancement while rotating:
```rust
// Skip lepton advancement — still rotating in place.
RotationResult::StillRotating { … }
```

**gamemd:** `UnitClass::Deploy` when facing mismatch does NOT call any movement function —
it only calls `Set_Desired_Heading` and returns 1. The locomotor's `Process` method handles
the body turn without translating position.

**Verdict: PASS** — position stays fixed (vacuously, since rotation is instant anyway).

---

### Stage 6 — Transformation gate: ConYard not spawned until facing matches

**Question:** Does the ConYard spawn only after the facing matches?

**Rust:** `world_spawn.rs:711` — gate exists, returns early if facing mismatch.
After the instant snap (Bug 4a), `source_facing == deploy_facing` on the *same* call,
so `deploy_mcv()` proceeds to despawn AMCV and spawn GACNST immediately.

**gamemd:** ConYard is created only after `((*puVar7 >> 7) + 1 >> 1 & 0xff) == uVar6`
passes. This happens after the locomotor has completed the rotation (all 13 binary frames).

**Result:** The gate exists in Rust code but is bypassed by the instant snap. The ConYard
spawns on the first tick after the deploy command, with no rotation delay.

**Verdict: FAIL** — ConYard spawns ~26 ticks too early (rotation delay omitted entirely).

---

### Stage 7 — Rotation direction (clockwise vs counter-clockwise)

**Question:** Does rotation use the shortest arc?

**Rust:** `turret.rs:18-28` `shortest_rotation(current, target)` — diff > 128 wraps CCW,
diff < -128 wraps CW. `0x80 - 0x40 = 64 <= 128` → positive → clockwise.
`movement_step.rs:212-215` — positive diff → `facing.wrapping_add(max_delta)` (clockwise).

**gamemd:** `RateTimer__Set` at `0x004c9220` — computes delta = desired - saved; sign-correct
interpolation; same shortest-arc convention.

**Verdict: PASS** (formula matches; moot due to instant snap, but the underlying
`shortest_rotation` logic is correct).

---

### Stage 8 — ROT value reading and formula

**Question:** Is `[AMCV] ROT=5` correctly read and used for rotation speed?

**Rust:** `rot_to_facing_delta(rot=5, tick_ms=33)` = `ceil(5*256*15*33/360000)` = 2
facing units per 33ms tick. Formula: converts ROT degrees/frame at 15fps to facing units/tick.

**gamemd:** FacingClass rate = ROT value (5). Timer duration = `|delta| / rate` frames.
For delta=64, rate=5: ~13 binary frames at 15fps. Per frame: 64/13 ≈ 5 facing units/frame.

**Numerical comparison:**
- gamemd: ~5 facing units per 15Hz frame = ~5/2 = ~2.5 facing units per 30Hz sim tick
- Rust: 2 facing units per 30Hz sim tick (div_ceil rounds up to 2, not 2.5)
- **Difference: ~25% slower rotation in Rust than gamemd**

The total rotation time differs: gamemd ~867ms (13 frames) vs Rust ~1067ms (32 ticks at 2/tick).

**Verdict: FAIL** (if rotation were implemented) — formula is different. Rust uses
degree-based conversion while gamemd uses `|delta|/rate` frame count. The tick count
would be wrong even if the snap bug were fixed.

---

### Stage 9 — CannotDeployHere sound NOT fired on valid placement

**Question:** Is `SimSoundEvent::CannotDeployHere` suppressed for valid flat grass?

**Rust:** `world_spawn.rs:693-696` — sound only emitted on structure-occupied cell.
`world_spawn.rs:700-706` — sound only emitted on build-blocked terrain. Neither fires
on clear flat grass.

**gamemd:** `UnitClass::Deploy @ 0x00739502` — `VoxClass__PlayEVA` with
`EVA_CannotDeployHere` only on placement failure, not on facing mismatch or valid ground.

**Verdict: PASS** — correct suppression on valid terrain.

---

### Stage 10 — Deploy sound when ConYard appears

**Question:** Is the AMCV's `DeploySound=PlaceBuilding` played when the ConYard spawns?

**Rust:** `world_spawn.rs:720-740` — no sound event emitted on successful MCV deploy.
`[AMCV] DeploySound=PlaceBuilding` is parsed (`rulesmd.ini:6995`) but not used here.
The `SimSoundEvent::EntityDeployed` exists (world/mod.rs:118) but is only used for
infantry deploy-fire.

**gamemd:** `UnitClass::Deploy @ 0x007393C0` at the `VocClass__PlayAt` call near the end
(line after `vtable+0xF8`): plays the type's `VocSound` (deploy sound) at the MCV's
position after successful placement.

**Verdict: FAIL / NOT-IMPLEMENTED** — no deploy sound on MCV → ConYard conversion.

---

### Stage 11 — ConYard idle anim start frame and tick

**Question:** Does GACNST appear with its idle anim starting at frame 0 on the same tick?

**Rust:** `world_spawn.rs:732-738` — `ge.building_up = Some(BuildingUp { elapsed_ticks: 0,
total_ticks: 30 })`. The ConYard spawns with a build-up animation rather than idle anim.

**gamemd:** `UnitClass::Deploy` calls `vtable+0x1E8` with mission 0x12 (Construction) on the
new BuildingClass before placing it. `BuildingClass::Constructor` initializes its own facing
and animation state via `RateTimer__Set(0x4000)`. The ConYard enters `Mission_Construct`
which plays the build-up/deploy anim sequence.

The 30-tick total is not verified against gamemd's construction timer. The gamemd timer
for the ConYard build-up is data-driven from the building type and animation frame rate.

**Verdict: UNCHECKED** — build-up animation is present in Rust but the duration (30 ticks)
is not verified against gamemd's mission construction timer.

---

## Summary Table

| Stage | Description | Verdict |
|---|---|---|
| 1 | DeployMcv command dispatch | PASS |
| 2 | DeployFacing source: GACNST type, default 0x80 | PASS |
| 3 | Facing comparison gate blocks deploy at 0x40 | PASS |
| 4a | In-place rotation — gradual, no instant snap | FAIL |
| 4b | Per-tick deploy retry after rotation completes | NOT-IMPLEMENTED |
| 5 | No position delta during rotation | PASS |
| 6 | ConYard spawn gated on facing match | FAIL |
| 7 | Shortest-arc rotation direction | PASS |
| 8 | ROT=5 to facing-delta formula | FAIL |
| 9 | CannotDeployHere suppressed on valid placement | PASS |
| 10 | Deploy sound on ConYard appearance | NOT-IMPLEMENTED |
| 11 | ConYard build-up anim start frame | UNCHECKED |

**PASS: 6 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 2**

---

## Top Failures (Player-Visible)

### 1. Instant facing snap (Stage 4a) — FAIL
**Player sees:** MCV sprite teleports from East to South facing in one frame instead of
smoothly rotating over ~867ms. The VXL hull direction pops instantly.
**File:line:** `src/sim/world/world_spawn.rs:714`
**gamemd evidence:** `UnitClass__Deploy @ 0x007395EF`: calls `Set_Desired_Heading` (vtable+0x4C)
with `target << 8`, then returns 1 — rotation happens over multiple frames via FacingClass.

### 2. ConYard spawns ~26 ticks too early (Stage 6) — FAIL
**Player sees:** ConYard appears instantly on deploy command with no rotation animation,
instead of appearing after ~867ms of rotation. Build timing is wrong.
**File:line:** `src/sim/world/world_spawn.rs:711-718` (snap + return true bypasses gate on
next call; but see Bug 4b — in practice ConYard spawns on same tick as facing snap)
**gamemd evidence:** `Mission_Deploy_Building @ 0x0073DDCB`: calls `UnitClass::Deploy()` in
state 1 every tick until `param_1[0x24]` (IsDeployed) is set.

### 3. No per-tick deploy retry after rotation (Stage 4b) — NOT-IMPLEMENTED
**Player sees:** If facing snap were fixed, player deploys MCV, it rotates, but ConYard
never appears because no system re-calls `deploy_mcv()` when `facing_target` clears.
**File:line:** No file — the mechanism does not exist. Closest: `src/sim/world/mod.rs` advance_tick
has no MCV deploy retry phase.
**gamemd evidence:** `Mission_Deploy_Building` state machine runs every tick; state 1 calls
`UnitClass::Deploy()` each tick until deployment succeeds.

### 4. ROT formula produces wrong tick count (Stage 8) — FAIL
**Player sees:** MCV rotates at wrong speed (32 sim ticks vs gamemd's ~26). ~230ms delay
difference. Visible on any non-instant MCV deploy.
**File:line:** `src/sim/movement/turret.rs:32-43` (`rot_to_facing_delta`)
**gamemd evidence:** `RateTimer__Set @ 0x004c9220`: duration = `|desired - saved| / rate`
frames. For AMCV: |0x80 - 0x40| = 64, ROT=5 → 13 binary frames at 15Hz.

### 5. No MCV deploy sound (Stage 10) — NOT-IMPLEMENTED
**Player sees/hears:** No `PlaceBuilding` sound when GACNST appears. In gamemd, `VocClass__PlayAt`
fires the type's deploy sound after `vtable+0xF8` (MarkDeployComplete).
**File:line:** `src/sim/world/world_spawn.rs:720-740` — no sound event
**gamemd evidence:** `UnitClass__Deploy @ 0x007393C0` near end: `if (*(int *)(iVar11 + 0x56c) != -1)`
plays deploy voc. `[AMCV] DeploySound=PlaceBuilding` (`rulesmd.ini:6995`).

---

## Adjacent Findings (not traced this run)

- **Test suite false-positive:** `deploy_mcv_waits_for_target_building_deploy_facing`
  (deploy_tests.rs:404) asserts `entity.facing == 0x80` after command. This passes due to
  the snap bug. The test should assert facing remains 0x40 during turn and spawns GACNST
  only after ~32 ticks (or ~26 with correct ROT formula).

- **`rot_to_facing_delta` formula mismatch:** This function is also used for turret rotation
  in `turret.rs`. The degree-to-facing-unit formula may produce incorrect speeds for other
  units with ROT≠0. Separate investigation needed.

- **MCV state machine missing:** There is no `McvState` component on GameEntity tracking
  "awaiting deploy after turn." The correct fix requires either: (a) a new `GameEntity` field
  to persist the "deploy pending" intent, or (b) a per-tick system that checks
  `facing_target.is_none() && <was_deploying>` and calls `deploy_mcv()`. Design deferred.

---

## Sources

- `UnitClass__Deploy @ 0x007393C0` — decompiled this session (Ghidra MCP)
- `UnitClass__Mission_Deploy_Building @ 0x0073D630` — decompiled this session (Ghidra MCP)
- `Deploy_facing_calculator @ 0x00465D70` — decompiled this session (Ghidra MCP)
- `UnitClass__Mission_Deploy @ 0x006AFF60` — decompiled this session (identified as slave miner path, not AMCV)
- `docs/research/AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md` — pre-existing verified research
- `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md` — pre-existing verified research
- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` — pre-existing verified research
- `docs/research/TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` — RateTimer/FacingClass algorithm
- `ini/rulesmd.ini:6969-6995` — `[AMCV]` section (ROT=5, DeploySound=PlaceBuilding)
- `ini/rulesmd.ini:11622-11631` — `[GACNST]` section (no DeployFacing)
- Rust code examined: `src/sim/world/world_spawn.rs`, `src/sim/deploy_tests.rs`,
  `src/sim/movement/movement_step.rs`, `src/sim/movement/turret.rs`,
  `src/sim/movement/movement_tick.rs`, `src/sim/game_entity.rs`, `src/rules/object_type.rs`
