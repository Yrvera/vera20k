# AMCV game-start first-move Drive retrace — 2026-07-29

**Scenario:** stock YR `[AMCV]`, normal/unveteran owner, no crate or house speed bonus, at
rest at the exact centre of flat clear Temperate cell `(40,40)`, ground Z, body facing north
`0x00`. One ordinary player Move to the exact centre of `(46,40)` — six cells due east, open
ground, no blockers. This is the first thing every skirmish player does after the match loads.

**Status:** **RED / NOT PARITY, and the 2026-07-20 diagnosis was pointed at the wrong
mechanism.** For this fixture active `gamemd.exe` does **not** drive a turn curve at all: the
new-cell adoption path gates on an exact body-facing match and pivots the MCV in place at
`ROT` before any cell is adopted, then selects the *straight* east track (TurnTrack 18 →
RawTrack 1, fresh cursor 0). Current Rust skips the pivot entirely and launches the MCV
straight into the north→east curve (TurnTrack 2 → RawTrack 4, cursor 11). One of the seven
2026-07-20 FAIL rows — the arrival NavCom clear — is now **PASS**, fixed by `26ef9d2a`. The
other six remain FAIL, two of them for different reasons than previously written down.

**Verdict tally:** **PASS 5 · FAIL 10 · UNCHECKED 4 · NOT-IMPLEMENTED 1** (20 bounded rows).
A PASS certifies only the named row, never the system.

---

## Scope, freshness, and evidence discipline

- Investigation only. No Rust, INI, or asset file was edited; no Cargo command was run; Ghidra
  access was strictly read-only (`decompile_function`, `batch_decompile`, `read_memory`,
  `get_function_xrefs`, `get_function_callees`, `get_assembly_context`,
  `search_functions_enhanced`, `get_current_program_info` only — no rename, comment, type
  write, or `save_program`). This report is the sole written artifact.
- Tree at `ce096b3f`. Current source is authoritative over every prior trace. The four
  movement commits since 2026-07-20 (`543ba0b9`, `26ef9d2a`, `4324d33b`, `6f6ec58e`) and
  `932fc5e8` were read from `git show` before adjudicating.
- **This is a retrace.** `AMCV_OPEN_GROUND_DRIVE_RETRACE_20260720.md` (PARTIAL, PASS 3 / FAIL 7
  / UNCHECKED 3) is re-adjudicated row by row in §"Re-adjudication" below. Generic open-ground
  Drive findings established one day earlier in
  `MTNK_OPEN_GROUND_DRIVE_FEEL_RETRACE_20260728.md` (startup ownership location, cell-handoff
  tick termination, sentinel-free completion, wall-clock frame-rate calibration) are **cited,
  not re-derived**; the budget here went to what is MCV-specific.
- Program: `gamemd.exe` in project `testProsjekt`, image base `0x00400000`, 10036 functions
  (verified via `get_current_program_info(program="gamemd.exe")`).
- Local Ghidra labels were treated as navigation hints only. Two of them are demonstrably
  misleading and are corrected below (`RateTimer__Current`, `Apply_Track_Delta`).
- The literal native per-frame coordinate/facing/cursor/residual series through arrival was not
  executed against an oracle and is recorded as UNCHECKED, not inferred.

### Native identity — re-verified from bytes, not labels

- `read_memory(0x007E7EAC,132)`: the dword immediately below the ILocomotion vtable is
  `0x007FFDE8`. `read_memory(0x007FFDE8,24)`: COL `+0x0C` → `0x00820248`.
  `read_memory(0x00820248,48)`: the TypeDescriptor name bytes spell
  `.?AVDriveLocomotionClass@@`. The same 132-byte read decodes vtable slot `+0x40` →
  **`Process @ 0x004B0500`** (dword index 17 from the vtable start) and slot `+0x4C` →
  **`Do_Turn @ 0x004B0EF0`**. Drive identity and the two anchors are byte-proven.
- Fresh read-only decompiles this session: `0x004B0500` (Drive `Process`), `0x004B2630`
  (`Process_Movement`), `0x004B0F20` (`Process_Drive_Track`), `0x004D94B0`
  (`FootClass::Set_Destination_Internal`), `0x004DB1A0` (`FootClass::GetCurrentSpeed`) — all
  four prior-doc addresses **confirmed correct**. Also `0x004B0EF0`, `0x004B0AD0`, `0x004C93D0`,
  `0x00476FC0`.

#### Two label corrections (record for future sessions)

- **`RateTimer__Current @ 0x004C93D0` is `FacingClass::Current`.** Its body interpolates from a
  stored previous facing toward a target over a frame-counter timer at a per-object rate (the
  `ROT` slot), returning the instantaneous facing (verified via
  `decompile_function 0x004C93D0`). The "RateTimer" name is stale and hides the fact that this
  is the facing read used by the movement facing gate.
- **`DriveLocomotionClass__Apply_Track_Delta @ 0x004B0AD0` is not a movement stepper.** It reads
  the RawTrack `+0x0C` index, transforms that point, and calls owner vtable `+0xF0`/`+0xF4`
  (occupation mark/unmark) — it is the *occupation handoff* helper (verified via
  `decompile_function 0x004B0AD0`). The 2026-07-20 claim that "a newly selected track is
  processed in the same native frame" by `Process_Drive_Track` mis-attributed this call; see
  the correction under row 13.

---

## Retail inputs

`ini/rulesmd.ini:6969-7010` — `[AMCV]`:

| Key | Value | Line |
|---|---|---|
| `Image` | `MCV` | 6972 |
| `Speed` | `4` | 6980 |
| `ROT` | `5` | 6986 |
| `Crusher` | `yes` | 6988 |
| `Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` (Drive) | 6998 |
| `Weight` | `3.5` | 6999 |
| `MovementZone` | `Normal` | 7000 |
| `Size` | `6` | 7006 |

Absent and therefore load-bearing by default: **no `Turret=`** (the body voxel is the entire
visual — nothing masks body-facing drift), **no `SpeedType=`**, **no `Accelerates=`** (so
`Accelerates` is true and the acceleration ramp is live; the `Accelerates=false` line at
`ini/rulesmd.ini:6962` belongs to the *preceding* section, not to `[AMCV]`), no
`AccelerationFactor=`, `DeaccelerationFactor=`, or `SlowdownDistance=` override.

`ini/artmd.ini:773-776` — `[MCV] Voxel=yes`, `Remapable=yes`, `Cameo=MCVICON`. Voxel/HVA render
keying, single body sprite.

### Effective `SpeedType` — derived from the binary, not guessed

`CCINIClass::ReadSpeedType @ 0x00476FC0` returns its third argument unchanged when the key is
absent (verified via `decompile_function 0x00476FC0`). The caller is `UnitTypeClass::ReadINI`;
the default is computed from `Crusher` immediately before the call
(verified via `get_assembly_context 0x00747700`):

```asm
007476de  MOV  DL, byte ptr [EDI + 0xd28]   ; Crusher
007476e4  NEG  DL
007476e6  SBB  EDX, EDX
007476e8  ADD  EDX, 0x2                     ; Crusher ? 1 : 2
007476eb  MOV  dword ptr [EDI + 0x67c], EDX ; TechnoTypeClass+0x67C = SpeedType
007476f1  MOV  EAX, dword ptr [EDI + 0x67c] ; ... passed as the ReadSpeedType default
00747700  CALL 0x00476fc0
```

`[AMCV] Crusher=yes` ⇒ **effective `SpeedType` = 1 = Track**. `+0x67C` then indexes the terrain
multiplier table as `table[SpeedType + LandType*9]` inside `Process_Movement` (verified via
`decompile_function 0x004B2630`, the `g_SpeedType_LandType_Table` read that produces the Drive
target speed fraction). On flat clear Temperate this is the unmodified full-speed row, so
`SpeedType` changes nothing for *this* fixture — but see row 3: the Rust default is
unconditional.

`Speed=4` → base integer `floor(4*256/100) = 10` per native speed frame on both sides; Rust
preserves it as `10*15 = 150` leptons/s (`src/util/fixed_math.rs:370-378`).

---

## Native pipeline for this exact fixture

```text
Move command -> TechnoClass::Set_Destination -> FootClass::Set_Destination_Internal @0x004D94B0
  (clear NavCom_Aux, write NavCom, resolve target coords, dispatch Head_To_Coord on the
   active locomotor, reset retry state)
-> Drive Process @0x004B0500 (no active track: TrackNumber == -1)
-> Process_Movement @0x004B2630, new-cell adoption at LAB_004b3298:
     path head octant = 2 (E)
     FACING GATE: FacingClass::Current @0x004C93D0 vs (octant << 13) = 0x4000
       mismatch -> Drive::Do_Turn @0x004B0EF0 (set desired facing) and RETURN,
       path node NOT consumed, no track selected, Drive+0x4C residual forced to 0
     ... repeats every logic frame until the body has pivoted north -> east ...
     once facing == 0x40: Can_Enter_Cell, terrain table -> Drive target fraction (+0x50),
       next-node octant = 2, selector = next + cur*8 = 18
-> TurnTrack 18 -> {normal=1, short=0, dir=0x40, flags=3}; flags bit 3 clear => straight
   track, path queue shifted by ONE node
-> LAB_004b460c: Drive+0x5C (point cursor) := 0, head-to := next cell centre,
   Apply_Track_Delta @0x004B0AD0 (occupation marking only)
-> back in Process @0x004B0500: Process_Drive_Track(param_2 = 0) runs in the SAME frame
   WITH full fresh speed
-> per frame: accel ramp -> GetCurrentSpeed -> budget = speed + residual, strict > 7,
   7 per point, read point[cursor] then tail-increment
-> RawTrack 1 = 23 points + a (0,0) sentinel at index 23; the sentinel ends the track,
   clears head-to, sets TrackNumber -1 and cursor 0
-> arrival: NavCom cell match -> FootClass::Stop_Moving, path head -1, OnArrival only
   when GetCurrentMission == 2 (Move)
```

### Verified table bytes

- TurnTrack control table base `0x007E7B28`, 12-byte entries
  (`decompile_function 0x004B2630`: `selector*0xc` indexing of
  `g_DriveTrackIndex_Table` / `g_DriveTrackFlags_Table`).
  `read_memory(0x007E7B28,48)` decodes entries 0–3; `read_memory(0x007E7C00,12)` decodes
  entry 18:

  | selector | normal | short | dir | flags | meaning |
  |---:|---:|---:|---:|---:|---|
  | 0 (N→N) | 1 | 0 | `0x00` | 0 | straight north |
  | 1 (N→NE) | 3 | 7 | `0x20` | 8 | curve |
  | 2 (N→E) | 4 | 9 | `0x40` | 8 | curve |
  | 3 (N→SE) | 0 | 0 | `0x60` | 0 | no track → straight fallback |
  | **18 (E→E)** | **1** | 0 | `0x40` | **3** | **straight east — this fixture** |

- RawTrack table base `0x007E7A28`, 16-byte entries (`read_memory(0x007E7A28,96)`):

  | raw | points | `+0x04` | `+0x08` | `+0x0C` |
  |---:|---|---:|---:|---:|
  | 1 | `0x007E6258` | −1 | **0** | −1 |
  | 3 | `0x007E64F8` | 37 | 12 | 22 |
  | 4 | `0x007E6790` | 26 | **11** | 19 |

  **What those fields actually are** (verified via `decompile_function 0x004B0F20`):
  `+0x04` is the *chain* trigger cursor; `+0x08` is the cursor a track is **entered at when it
  is chained into mid-flight** — the chain branch writes `cursor := newRaw[+0x08] - 1` and the
  tail increment then lands on it; `+0x0C` is the **occupation-handoff** cursor consumed by
  `Apply_Track_Delta @ 0x004B0AD0` (owner vtable `+0xF0`/`+0xF4` mark/unmark), not a
  "cell-cross" index. A *fresh* track never reads `+0x08`: `Process_Movement` writes cursor `0`
  unconditionally at `LAB_004b460c`.

- RawTrack 1 points at `0x007E6258` (`read_memory(0x007E6258,180)` and
  `read_memory(0x007E635C,36)`): `(0,245,0), (0,234,0), (0,223,0) … (0,3,0)` — 23 points,
  y stepping by −11, then **`(0,0,0)` at index 23** as the end-of-track sentinel. Every heading
  byte on this track is `0x00` in raw frame (the transform supplies the east rotation).

---

## Stage-by-stage

### 1–3. Rules, locomotor, SpeedType

Rust reads `Speed=4`, `ROT=5`, Drive CLSID, `MovementZone=Normal`, `Crusher=yes`, and — because
`[AMCV]` sets no acceleration keys — the defaults `Accelerates=true`,
`AccelerationFactor=0.03`, `DeaccelerationFactor=0.002`, `SlowdownDistance=500`
(`src/rules/object_type.rs:921-933`). Those match the native type-field defaults consumed at
`+0x2F8`/`+0x300`/`+0x308`/`+0xDBD` in `0x004B0F20`.

`SpeedType` is where the mechanisms part. Rust is
`section.get("SpeedType").map(SpeedType::from_ini).unwrap_or_default()`
(`src/rules/object_type.rs:1071-1074`) with `impl Default for SpeedType { Self::Track }`
(`src/rules/locomotor_type.rs:149-152`) — an **unconditional** Track default. Native is
`Crusher ? Track : Wheel`. For `[AMCV]` the two agree (Crusher=yes → Track), so the AMCV value
is PASS; the rule is FAIL. **Frequency:** 54 stock Drive sections omit `SpeedType=`, and 31 of
them are not Crushers — including `[FV]` IFV, `[YTNK]` Gattling Tank, `[DRON]` Terror Drone,
`[FTRK]` Flak Track, `[CAOS]` Chaos Drone. Every one of those is Wheel in gamemd and Track in
Rust, which changes their terrain multiplier and pathfinding cost on rough/beach/ice cells in
ordinary play.

### 4–6. Command acceptance, destination ownership, path

`src/sim/world/world_commands.rs:146-175,254-284` accepts the living owned unit, queues the Move
mission, clears incompatible order/attack/dock intent, and dispatches path creation. The literal
native command-frame ordering and mission-byte writes were not captured → UNCHECKED.

Destination ownership is native-shaped: `movement_commands.rs:545-577` calls
`navcom::set_destination_internal_cell`, which sets owner `nav_com=(46,40)` and the Drive
destination/head-to (`src/sim/movement/navcom.rs:60-74,128-134`). That is exactly the
`FootClass::Set_Destination_Internal @ 0x004D94B0` shape re-verified this session (clear aux,
write NavCom, resolve coords, dispatch `Head_To_Coord` on the active locomotor at `+0x19D`,
reset retry). PASS.

The static empty-grid Rust path is `(40,40) … (46,40)` in six east steps. The native path queue
for this fixture was not executed → UNCHECKED.

### 7. The initial turn — the headline break

**Native rotates in place before it moves.** At `LAB_004b3298` in `Process_Movement`, after the
playfield and crush checks and *before* `Can_Enter_Cell`, the adoption path reads the owner's
instantaneous facing through `FacingClass::Current @ 0x004C93D0`, subtracts `octant << 13`
(octant 2 → `0x4000`, i.e. facing byte `0x40` east), and on **any** nonzero difference calls
Drive vtable `+0x4C` = `Do_Turn @ 0x004B0EF0` (which just sets the desired facing) and returns
**without consuming the path node and without selecting a track** (verified via
`decompile_function 0x004B2630`, `decompile_function 0x004B0EF0`,
`decompile_function 0x004C93D0`). While that repeats, `Process_Drive_Track` takes its early
return at the top (`TrackNumber == -1` and no head-to) and forces `Drive+0x4C = 0`, so no speed
fraction accumulates and no residual builds. A stock MCV therefore **pivots north→east on the
spot at `ROT=5`, then starts to translate**.

Current Rust has no such gate. `movement_commands.rs:566` writes
`drive.turn.first_movement_allowed = false` and that field is **never read anywhere in `src/`**
— it is dead state. When a drive track starts, `movement_commands.rs:611-613` explicitly sets
`entity_mut.facing_target = None`, handing all facing authority to the curve. The MCV begins
translating and rotating in the same frame. **NOT-IMPLEMENTED.**

**Player visibility:** maximal. This is the first second of every match, on a turretless unit
whose body voxel is the whole silhouette, with the camera centred on it and nothing else moving.

### 8. First track selection — wrong inputs, wrong track

Native selector is `next_node_octant + current_node_octant * 8`, both taken from the **path
queue** (`Process_Movement`: `iVar5 = uVar19 + uVar18 * 8` writing `Drive+0x58`), evaluated only
after the facing gate has already aligned the body to the current node. For a straight east
path that is `2 + 2*8 = 18` → RawTrack 1, flags 3, straight.

Rust selector is `facing_to_dir(current_body_facing) * 8 + facing_to_dir(next_facing)` where
`current_facing` is the **body facing** and `next_facing` is the direction of the **first path
step** (`src/sim/movement/drive_track.rs:3465-3500`, called from
`movement_commands.rs:585-591`). For this fixture that is `0*8 + 2 = 2` → RawTrack 4, flags 8,
the north→east curve.

So Rust drives a 90° turn curve where native drives a straight line. **FAIL.** Note the
selector *formula* is the same shape on both sides; only the two inputs differ. The prior
traces' claim that "facing pair `0 → 0x40` selects TurnTrack 2 / RawTrack 4" describes the
selector correctly but attaches it to the wrong situation: entry 2 is what a unit already
*travelling north* uses when its next node turns east.

### 9. Fresh cursor

`begin_drive_track_with_head_offset` sets `point_index: meta.entry_index`
(`drive_track.rs:3649-3664`) — RawTrack `+0x08`. Native writes cursor `0` on every fresh track
(`Process_Movement`, `LAB_004b460c`) and only ever uses `+0x08` on a mid-flight chain-in.
**FAIL** on the mechanism. Numerically, RawTrack 1's `+0x08` is `0`, so had Rust selected the
right track the value would have coincided; on the RawTrack 4 it actually selects, it starts 11
points into a 38-point curve.

### 10. Point read/increment order — a permanent one-point lead

Native: `iVar8 = *(int *)(param_1 + 0x5c); uStack_c8 -= 7;` reads point[cursor] at the top of
the do-loop and `*(int *)(param_1 + 0x5c) += 1;` increments at the bottom
(`decompile_function 0x004B0F20`). Rust: `state.point_index += 1;` then reads
(`drive_track.rs:3746-3748`). With a fresh cursor of 0, native's first paid point is index 0 and
Rust's is index 1 — on RawTrack 1 that is exactly **11 leptons of permanent lead** for the same
budget. **FAIL.**

Budget arithmetic itself matches: native `uStack_c8 = (retry ? 0 : speed) + Drive+0x4C`, strict
`if (7 < uStack_c8)`, cost 7 per point, leftover stored back to `Drive+0x4C` at the end
(`0x004B0F20` lines around the `do…while (7 < uStack_c8)` loop). Rust is
`budget = fresh_budget + *residual_budget`, `while budget > TRACK_STEP_COST`, `budget -= 7`,
`*residual_budget = budget` (`drive_track.rs:3742-3779`) and the retry path adds zero fresh
speed. Row 12 PASS.

### 11. Track length and the paid sentinel

RawTrack 1 has 23 real points plus a `(0,0)` sentinel at index 23. Native **pays** that sentinel
(it enters the do-loop, subtracts 7, then detects `x==0 && y==0 && cursor!=0` and runs the
end-of-track block). Rust's metadata is `points_count: 23`, `last_index = 22`, and the loop
condition `state.point_index < last_index` stops at 22 — the sentinel is never reached and the
completion is synthetic (`drive_track.rs:3737,3746,3781`).

Nominal full-cell cost is therefore **24 × 7 = 168** native versus **22 × 7 = 154** Rust,
about **8.3 % cheaper per cell**. **FAIL.** (Rust's `cell_jump` break can end a pass earlier
still; the exact per-cell paid count under that interaction was not executed.)

### 12. Budget/residual arithmetic

PASS — see row 10 above. Residual sub-point interpolation is also structurally the same idea:
native scales the delta by `residual * 0.14285715` in float
(`CoordStruct__ScaleByFactor`, `0x004B0F20`) and commits under a cell-validity guard; Rust
truncates `delta * residual / 7` in integers. The rounding difference is sub-lepton and was not
promoted to a separate row.

### 13. Same-frame processing of a newly created track — prior claim corrected

`Process @ 0x004B0500`, no-track branch: `Process_Movement(&out, 1, 0)`, and if the out-flag is
clear and the owner is alive it sets the retry argument to **0** and calls
`Process_Drive_Track(0)` — i.e. the fresh track *is* stepped in the same native frame and **with
full fresh speed**, not with a zero budget. The 2026-07-20 table's `F0/F1/F2` rows ("centered
while integer fresh budget remains 0") rest on the mis-attribution of
`Apply_Track_Delta @ 0x004B0AD0` as the stepper and should not be reused.

Rust instead creates the path, the Drive runtime, and the track inside **command dispatch**
(`movement_commands.rs:545-631`) rather than inside a Drive process pass. **FAIL** — this is the
generic startup-ownership row already established as
`MTNK_OPEN_GROUND_DRIVE_FEEL_RETRACE_20260728.md` stage 5; cited, not re-derived.

### 14. Acceleration cadence — unchanged since 2026-07-20

Native updates the speed fraction exactly once per `Process_Drive_Track` invocation, inside the
`Drive+0x58 < 0x40` block: accelerate by `AccelerationFactor` (`type+0x308`, flat, clamped to
the Drive target `+0x50`), decelerate by `base_speed * DeaccelerationFactor` (`type+0x300`), and
inside `SlowdownDistance` (`type+0x2F8`) brake by the same product with a hard floor of
`0.3` (`decompile_function 0x004B0F20`). Rust's `update_drive_speed_fraction`
(`src/sim/movement/drive_locomotion.rs:98-133`) implements exactly those three arms with the same
constants — but its only movement-tick call site, `movement_tick.rs:1246`, is **not** behind the
15 Hz gate. That gate, `drive_track_native_frame_count`, exists solely in
`movement_step.rs:47-58,757` and controls the *point budget*, not the fraction. With
`SIM_TICK_HZ = 45` and `DRIVE_TRACK_NATIVE_FRAME_HZ = 15`, the fraction advances **three times
per native frame**. **FAIL, unchanged.**

Concretely: `Accelerates=true`, `AccelerationFactor=0.03`, so the ramp from rest to full takes
34 updates. Rust takes 34 sim ticks; native takes 34 Drive frames. The MCV reaches cruising
speed roughly 3× sooner in Rust. Note the strict `> 7` gate means nothing moves at all until
`floor(10 * fraction) + residual > 7`, so the whole opening acceleration is player-visible as a
different launch profile, not just a different top-speed arrival.

### 15. Braking cadence

Same function, same three constants, same call site → the `SlowdownDistance=500` /
`10 * 0.002 = 0.02` decrement / `0.3` floor is applied three times per native frame.
**FAIL, unchanged.** Visible as the MCV stopping shorter and harder on arrival at `(46,40)`.

### 16. Cell handoff authority

Rust derives a cell crossing from the transformed coordinate leaving `[0,256)` and **breaks the
point loop**, ending the entity's step (`drive_track.rs:3750-3766`). Native has no such rule:
RawTrack `+0x0C` is an explicit occupation mark/unmark cursor consumed by `Apply_Track_Delta`,
and for **RawTrack 1 it is `-1`** — the straight east track has *no* occupation-handoff point at
all; the cell change is owned by the `(0,0)` sentinel plus the next `Process_Movement` adoption.
**FAIL.** The 2026-07-20 statement "RawTrack 4 cell-cross index is 19" names a real byte but the
wrong role, and the wrong track for this fixture.

### 17. Arrival NavCom clear — now PASS (fixed by `26ef9d2a`)

Native end-of-track block (`0x004B0F20`, the `(0,0)`-sentinel branch): clears head-to, sets
`TrackNumber = -1` and cursor `0`, compares the owner cell against the NavCom target, and — under
the liveness gate `+0x90` alive / `+0x81` / `+0x8D` — calls `FootClass::Stop_Moving`, sets the
path head to `-1`, and fires the `OnArrival` virtual **only** when `GetCurrentMission == 2`
(Move). Nothing is deferred.

Current Rust matches that shape: `finalize_finished_entities` (`movement_tick.rs:1963-2013`)
calls `navcom::finish_drive_navigation` (`navcom.rs:115-148`), which on a NavCom cell match runs
`finish_drive_arrival` (`navcom.rs:156-173`) — `foot_stop_moving`, drive-runtime reset, and a
`MissionType::Move`-gated pop of `nav_queue[0]` into `set_destination_internal_cell` — and skips
the clear entirely for a dying object. The deferred pass survives only as the process-entry
fallback for NavCom-set-but-no-track states. **PASS** for this row: the clear is same-tick with
the track finish, and the track can only finish on a 15 Hz-aligned tick, so the mission layer no
longer sees a stale NavCom for an extra dispatch.

### 18. Render handoff

`[AMCV]` has no `Turret=`, so `src/app_instances/units.rs:255-289` takes the non-turret branch
and emits a **single composite voxel sprite keyed directly on `entity.facing`** — the value the
drive track writes each budget pass. Screen coordinates come straight from sim
`screen_x/screen_y` with no interpolation (`units.rs:195-197`). There is no turret sprite to
absorb body-facing error and no smoothing layer between sim and screen, so rows 7–11 are
presented to the player at full byte-facing resolution. **FAIL.**

### 19–20. Literal series and wall-clock pace

The complete native per-frame coordinate/facing/cursor/residual/NavCom series through `(46,40)`,
the literal arrival frame, and the native pivot duration at `ROT=5` were not captured →
**UNCHECKED**. Stock wall-clock pace at `GameSpeed=1` is the generic frame-rate calibration
question owned by `E2_STATIC_WALL_WALK_FEEL_RETRACE_20260728.md` and
`MTNK_OPEN_GROUND_DRIVE_FEEL_RETRACE_20260728.md`; it was not re-derived here →
**UNCHECKED** for this fixture.

---

## Stage verdict table

| # | Stage | Verdict | Bounded result |
|---:|---|---|---|
| 1 | Retail `[AMCV]` / Drive / `[MCV]` art bindings | PASS | Speed 4, ROT 5, Drive CLSID, Normal, Crusher, no Turret, Voxel — all agree. |
| 2 | AMCV effective `SpeedType` value | PASS | Native `Crusher ? Track : Wheel` → Track; Rust default Track. Same value. |
| 3 | `SpeedType` default *rule* | FAIL | Rust default is unconditional Track; native branches on `Crusher`. 31 stock non-Crusher Drive units affected. |
| 4 | Command → Move mission acceptance | UNCHECKED | Rust path identified; native command-frame writes not captured. |
| 5 | Destination ownership / `Head_To_Coord` handoff | PASS | Owner NavCom `(46,40)` + Drive destination/head-to, native-shaped. |
| 6 | Flat path cells | UNCHECKED | Rust static expectation is six east steps; native queue not executed. |
| 7 | Initial in-place pivot before cell adoption | **NOT-IMPLEMENTED** | Native facing gate → `Do_Turn`, node not consumed. Rust has no gate; `first_movement_allowed` is dead state. |
| 8 | First track selection inputs | **FAIL** | Rust body-facing × first-step → selector 2 / RawTrack 4 (curve). Native path-node × next-node → selector 18 / RawTrack 1 (straight). |
| 9 | Fresh track cursor | **FAIL** | Rust uses RawTrack `+0x08`; native writes `0` and reads `+0x08` only on mid-flight chain-in. |
| 10 | Point read/increment order | **FAIL** | Rust pre-increments; native reads then post-increments. 11-lepton permanent lead. |
| 11 | Track length / paid `(0,0)` sentinel | **FAIL** | 154 vs 168 budget per straight cell (~8.3 % cheap). |
| 12 | Budget/residual arithmetic | PASS | `speed + residual`, strict `>7`, cost 7, leftover carried, retry adds zero fresh speed. |
| 13 | Startup ownership / same-frame first step | **FAIL** | Rust builds track in command dispatch; native in Drive `Process`, then steps it same-frame with full fresh speed. (Generic; MTNK 20260728 stage 5.) |
| 14 | Acceleration cadence | **FAIL** | Fraction updates every 45 Hz tick; native once per Drive frame. 3× fast ramp. |
| 15 | Braking cadence | **FAIL** | Same threshold/decrement/floor, applied 3× per native frame. |
| 16 | Cell handoff authority | **FAIL** | Rust coordinate crossing ends the tick; native `+0x0C` occupation handoff is `-1` on this track. |
| 17 | Arrival NavCom clear | **PASS** | `26ef9d2a` landed the same-tick `Stop_Moving` + Move-gated queue advance + dying-object skip. |
| 18 | Render handoff | **FAIL** | Turretless voxel keyed on `entity.facing`, raw sim screen coords — nothing masks rows 7–11. |
| 19 | Literal native per-frame series | UNCHECKED | Not executed against an oracle. |
| 20 | Stock wall-clock pace | UNCHECKED | Owned by the E2/MTNK frame-rate calibration; not re-derived. |

**Tally: PASS 5 · FAIL 10 · UNCHECKED 4 · NOT-IMPLEMENTED 1.**

---

## Re-adjudication of the seven 2026-07-20 FAIL rows

| 2026-07-20 row | Now | Why |
|---|---|---|
| Fresh-track cursor (metadata 11 vs native 0) | **still FAIL** | Mechanism unchanged at HEAD (`drive_track.rs:3656`). Corrected: `+0x08` is the mid-flight chain-in cursor, and for *this* fixture native selects RawTrack 1 (`+0x08 = 0`) — Rust selects RawTrack 4 and starts at 11. |
| Initial 90° turn | **still FAIL, reframed** | Not "curve begins at the wrong point": native does not use a curve here at all. It pivots in place at `ROT` through the `Process_Movement` facing gate first. Rust has no equivalent → row 7 NOT-IMPLEMENTED, row 8 FAIL. |
| Acceleration cadence (45 Hz vs native frame) | **still FAIL** | `update_drive_speed_fraction` at `movement_tick.rs:1246` is still outside the 15 Hz `drive_track_native_frame_count` gate. |
| Point/residual progression + cell-cross index | **still FAIL, reframed** | Read/increment order (row 10) and the unpaid sentinel (row 11) are the concrete drifts. "RawTrack 4 cell-cross index 19" is a real byte with the wrong role — `+0x0C` is occupation mark/unmark, and it is `-1` on the straight track. |
| Braking cadence | **still FAIL** | Same call site, same 45 Hz cadence. Threshold/decrement/floor values themselves agree. |
| Arrival NavCom clear | **now PASS** | Fixed by `26ef9d2a`; verified against the `0x004B0F20` end-of-track block this session. |
| Render handoff | **still FAIL** | Turretless single-sprite voxel keyed on `entity.facing`, raw sim screen coords. |

`543ba0b9`, `4324d33b`, and `6f6ec58e` touch deferred-repath retry and the tube-step gate; none
of them is on this fixture's path (open ground, no tube cell, no failed repath). `932fc5e8`
changes the miner's outbound command shape, not the player Move path.

---

## Top root findings

1. **The MCV does not pivot before it moves.** Every skirmish opens with the player ordering the
   MCV somewhere; on any order that is not already dead ahead, native shows a stationary body
   rotation at `ROT=5` and *then* forward motion, while Rust shows an immediate curved slide.
   Because `[AMCV]` has no turret, the body voxel is the entire visual — there is nothing to
   hide it behind. This is the single most-seen locomotion difference in the game.
2. **The wrong track is selected because the selector reads the wrong two inputs.** Native's
   pair is (current path node direction, next path node direction); Rust's is (body facing,
   first step direction). Fixing the facing gate would make most of the curve-track cursor and
   entry-index arguments moot for straight moves, because straight moves would stop selecting
   curves at all.
3. **Acceleration and braking run three times too often.** The constants and the three arms are
   right; only the cadence is wrong, and the fix is one gate. On an `Accelerates=true` unit with
   `Speed=4` this is the difference between a heavy MCV that leans into its start and one that
   snaps to cruising speed.
4. **A straight cell costs 154 budget in Rust and 168 in gamemd**, from the unpaid `(0,0)`
   sentinel plus the pre-increment. The MCV is both permanently 11 leptons ahead and about 8 %
   fast per cell, compounding across the six-cell move.
5. **The arrival handoff is now correct** — worth stating plainly, because it was the seventh
   FAIL row and `26ef9d2a` closed it against the same binary block re-read here.

## Smallest decisive follow-up

Implement and check the facing gate first, alone: on drive-track creation, if the body facing is
not exactly `first_step_octant << 5`, set a facing target and consume **no** path node and
create **no** track this tick. Then re-run this fixture. That one change converts rows 7 and 8
and removes the RawTrack-4 curve from every straight-line move, which is the precondition for
rows 9–11 mattering at all. The remaining literal-series UNCHECKEDs need a controlled native
capture at `0x004B0F20` entry/exit logging owner coordinate/facing, `Drive+0x58` selector,
`Drive+0x5C` cursor, `Drive+0x4C` residual, owner speed fraction, and NavCom each frame from
command acceptance through arrival — compared against the equivalent Rust fixture.
