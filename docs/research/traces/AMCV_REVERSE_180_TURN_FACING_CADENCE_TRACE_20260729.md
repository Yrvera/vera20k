# AMCV 180° Reverse — Turn/Facing Cadence Retrace — 2026-07-29

**Scenario:** stock YR `[AMCV]` at the exact center of flat clear Temperate cell `(50,50)`,
ground Z, body facing east `0x40`. One ordinary player Move to the exact center of `(45,50)`
— five cells due west, open ground, no blockers. A full `0x40 → 0xC0` reversal.

**Status:** **RED / NOT PARITY — and the failure is directional, not cosmetic.** Native
rotates the hull in place for 25 binary frames and does not move a lepton until the body
faces west exactly; it never selects a curve for this turn and never drops a path node.
Current Rust has no facing-alignment gate at all: it takes the "sharp turn" substitute
keyed on the **body facing** instead of the path-step direction, so the first visible
motion is the MCV driving one cell **east — away from the ordered destination** — while
consuming a westward path node, with the hull facing frozen at `0x40` the whole time.

**Verdict tally:** **PASS 10 · FAIL 9 · UNCHECKED 2 · NOT-IMPLEMENTED 1** (22 bounded
stages). A PASS certifies only the named row.

**Frequency clause.** This is not an MCV-only path. The Rust branch fires for *any* Drive
unit whose first ordered path step is ≥135° from its current body facing — every retreat
order, every "come back the way you came", every unit told to reverse out of a firefight,
plus the very common MCV case of misclicking the deploy spot and re-clicking behind the
unit. In ordinary skirmish that is dozens of times per match per player, and it is
maximally visible on AMCV because `[AMCV]` (`ini/rulesmd.ini:6969..7010`) has **no
`Turret=`** — the entire body silhouette carries the heading, with no turret to absorb the
error.

## Scope, freshness, and evidence discipline

- Investigation only. No Rust, INI, or asset was edited; no Cargo command was run. Ghidra
  access was read-only — no renames, comments, type writes, or `save_program`. This report
  is the sole written artifact.
- Tree read at `ce096b3f`. Current source is authoritative over
  `AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md` (used for navigation only). Two of that
  trace's FAILs are now stale in the direction it did *not* predict: `handle_vehicle_rotation`
  has since been migrated off millisecond integration onto the binary-frame `FacingClass`
  (`movement_step.rs:287..347`), but for Drive units that function is now **dead code on
  this path** — see stage 4.
- Every address, offset, and table byte below names the tool call that produced it this
  session. Local Ghidra labels were treated as hints and re-bound from vtable slot bytes,
  instruction operands, and function bodies.
- **Anchor drift versus the task brief:** the brief cited TurnTrack at `0x007E7B40` and
  RawTrack metadata at `0x007E7A68`. Both are wrong. Verified from instruction operands and
  contiguity: TurnTrack base = **`0x007E7B28`**, RawTrack base = **`0x007E7A28`**
  (16 records × 16 bytes = 256 bytes, ending exactly at the TurnTrack base).
- The literal native per-frame facing series is derived in closed form from two freshly
  decompiled bodies; it was **not executed** against an oracle and is recorded `UNCHECKED`.
  No value was promoted from static plausibility.

## Retail inputs

- `ini/rulesmd.ini:6969..7010`: `[AMCV]`, `Image=MCV`, `Speed=4`, `ROT=5`,
  `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` (Drive), `MovementZone=Normal`,
  `Crusher=yes`, `DeploysInto=GACNST`. **No `Turret=` key** — non-turret draw path.
- Path for the fixture is the trivial straight westward row
  `(50,50)→(49,50)→(48,50)→(47,50)→(46,50)→(45,50)`; every step direction is octant `6` (W).
  No smoothing pass can alter a single-direction row.

## Stage 1 — ROT semantics and where it is applied

`TechnoTypeClass+0x71c` is the rules `ROT=` field. The hull facing is `TechnoClass+0x388`,
a 24-byte `FacingClass` whose ROT is set to the rules value by `UnitClass::Constructor`
(established in `BODY_FACING_DRIVE_LOCOMOTOR_ROT_GHIDRA_REPORT.md`; re-bound below through
the locomotor rather than taken on trust).

Fresh `disassemble_function 0x004B0EF0` — the whole body is three loads and a call:

```text
004b0f01 MOV ECX, dword ptr [EDX + 0x8]   ; linked object
004b0f04 ADD ECX, 0x388                   ; hull PrimaryFacing
004b0f0a CALL 0x004c9220                  ; RateTimer::Set (gradual setter)
```

Fresh `decompile_function 0x004c93d0` (the interpolator read) reduces to exactly:

```text
rot   = short @ +0x14           ; rot < 1  -> return current
start = int   @ +0x08           ; -1 = never started
dur   = int   @ +0x10
diff  = (short)current - (short)prev
step  = abs(diff) / rot         ; step < 1 -> return current
rem   = dur - (frame - start)   ; clamped at 0
result = current - (diff / step) * rem
```

So **ROT is a per-binary-frame angular delta in 16-bit facing units, stored as
`rot_byte << 8`**. `ROT=5` ⇒ `1280` units/frame. Turn duration is
`abs(delta_16) / (ROT << 8)` = `delta_8 / ROT`, integer-truncated:

| Turn | delta | frames at ROT=5 |
|---|---|---|
| 45° | `0x20` | 6 |
| 90° | `0x40` | 12 |
| **180°** | **`0x80`** | **25** |

Rust `src/sim/movement/facing_class.rs:75..140` implements this formula field-for-field
(clamp `>0x7E → 0x7F`, `<<8`, `duration = abs(diff)/rot`, `current - (diff/step)*remaining`,
`step<1` snap). It is a faithful port. **PASS** on the primitive.

**Cadence.** The prompt's concern — that facing might inherit the acceleration path's ×3
tick multiplier — does **not** hold. `binary_frame = (total_sim_ms · 15) / 1000`
(`src/app_sim_tick.rs:601`), i.e. one frame per three 22 ms sim ticks, and the drive-track
budget is gated identically by `drive_track_native_frame_count`
(`movement_step.rs:47..58`). Facing advances once per native frame, not 45 times a second.
The acceleration fraction is the outlier: `update_drive_speed_fraction` is called
un-gated inside the per-tick body (`movement_tick.rs:1246`), so the speed fraction still
mutates three times per native frame — that prior finding stands at HEAD.

What *does* drift is the absolute clock. At stock `GameSpeed=1` gamemd caps near 62.5 logic
frames/s, so the 25-frame reversal takes **≈0.40 s**. Rust's `binary_frame` advances at
≈21/s in real time (`NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md:150..231`,
re-read this session), so the same 25 frames would take **≈1.19 s** — about **3.0× slower**.
This is the engine-wide calibration gap, not a facing-specific one.

## Stage 2 — TurnTrack / RawTrack selection for the `0x40 → 0xC0` pair

Table identity was re-bound from instruction operands, not labels
(`search_instructions operand_pattern="7e7b"`):

```text
004b4023  MOV  CL,  byte ptr [EAX*0x4 + 0x7e7b28]   ; TurnTrack[idx].normal_selector
004b403a  TEST byte ptr [EDX*0x4 + 0x7e7b30], 0x8   ; TurnTrack[idx].flags bit 3
```

The `*0x4` scale with an index already tripled gives a **12-byte stride**: `+0x00` normal
RawTrack selector, `+0x01` short selector, `+0x04` target facing, `+0x08` flags. Base
**`0x007E7B28`**.

The index formula is read straight off the path queue, *not* off the body facing
(`decompile_function 0x004B2630`):

```c
uVar18 = *(uint *)(techno + 0x5e0);        // path queue [0] — current step octant
uVar19 = *(uint *)(techno + 0x5e4);        // path queue [1] — next step octant
iVar5  = uVar19 + uVar18 * 8;              // TurnTrack index
*(param_1 + 0x60) = 0;                     // short selector cleared
*(param_1 + 0x58) = iVar5;
if (TurnTrack[iVar5].normal_selector == 0)
    *(param_1 + 0x58) = uVar18 * 9;        // fallback: straight track for the CURRENT step
```

Literal bytes, `read_memory` this session:

| Record | Address | Bytes | normal | short | target facing | flags |
|---|---|---|---:|---:|---|---:|
| 18 (`cur=E,next=E`) | `0x007E7C00` | `01 00 00 00 · 40 00 00 00 · 03 00 00 00` | 1 | 0 | `0x40` | 3 |
| **22 (`cur=E,next=W`)** | **`0x007E7C30`** | **`00 00 00 00 · c0 00 00 00 · 00 00 00 00`** | **0** | **0** | `0xC0` | 0 |
| 54 (`cur=W,next=W`) | `0x007E7DB0` | `01 00 00 00 · c0 00 00 00 · 01 00 00 00` | 1 | 0 | `0xC0` | 1 |

(`read_memory 0x007E7C00 len 24`, `read_memory 0x007E7C18 len 72`, `read_memory 0x007E7DA4 len 36`.)

**The 180° pair has no curve.** Every `≥3`-octant record in the sampled rows (21, 22, 23,
24 and the `0x40..0x60`/`0x80`/`0xA0` family at `read_memory 0x007E7B28 len 96`) carries
normal selector `0`. Only ±0, ±1, ±2 octants have real tracks, and only those carry the
curve bit `0x8`.

RawTrack metadata (`read_memory 0x007E7A28 len 64`), 16-byte records
`(point pointer, chain index, entry index, cell-cross index)`:

- `RawTrack[0]` — `ptr 0, chain 0, entry 0xC0, cross 0` — the null/sentinel row selector `0` points at.
- `RawTrack[1]` — `ptr 0x007E6258, chain -1, entry 0, cross -1` — the straight track.
- `RawTrack[3]` — `ptr 0x007E64F8, chain 37, entry 12, cross 22` — a real curve, for contrast.

**In this fixture native never consults record 22 at all.** The path is uniformly westward,
so `uVar18 = uVar19 = 6` and the only record ever indexed is **54** (`= 6·9`), whose normal
selector is `1`, target facing `0xC0`, flags `1` (bit 3 clear ⇒ single-node, non-curve).
Rust's `TURN_TRACKS`/`RAW_TRACKS` constants
(`src/sim/movement/drive_track.rs:321..364`, `566..588`, `707..740`) reproduce every one of
these rows byte-for-byte. **Table data: PASS. Index operands: FAIL** (stage 3).

## Stage 3 — Turn in place, not an arc: the facing gate

Before any track is selected, `Process_Movement` compares the hull facing against the
current path step and bails out if they differ by **anything at all**
(`disassemble_bytes 0x004b33f0 len 80`, `disassemble_bytes 0x004b343b len 48`):

```text
004b3410  ADD    ECX, 0x388            ; hull PrimaryFacing
004b3416  CALL   0x004c93d0            ; FacingClass::Current()
004b341b  MOV    AX,  word ptr [EAX]
004b341e  MOV    ECX, EBX
004b3420  SHL    ECX, 0xd              ; desired = step_octant << 13
004b3423  SUB    AX,  CX
004b3426  MOVSX  EAX, AX               ; -> abs(diff) in ESI, 0 in EAX
004b3437  CMP    EAX, ESI
004b3439  JGE    0x004b345b            ; 0 >= abs(diff)  -> aligned, proceed
; else:
004b344c  CALL   dword ptr [ECX + 0x4c]  ; Do_Turn(desired)
004b3452  MOV    AL,  0x1
004b3458  RET    0xc                     ; return WITHOUT moving, WITHOUT consuming a node
```

`octant << 13` is the 16-bit facing space: `2<<13 = 0x4000` (E), `6<<13 = 0xC000` (W) —
consistent with facing bytes `0x40`/`0xC0`. Slot `+0x4c` was resolved from the vtable bytes,
not the label: `read_memory 0x007E7EB0 len 128` gives `+0x40 → 0x004B0500` (Process),
**`+0x4C → 0x004B0EF0`** (Do_Turn), `+0x70 → 0x004B0C40` (Force_Track) — the last matching
the independently established Drive `Force_Track` binding, which confirms `0x007E7EB0` is
the Drive `ILocomotion` vtable.

`decompile_function 0x004B0500` confirms the frame-0 entry: with no forced track
(`+0x54 == -1`) the dispatcher goes straight to `Process_Movement`.

**So the native answer is unambiguous: a 180° reversal is a pure rotate-in-place at the
origin cell.** No neighbouring cell is entered, no track is installed (`+0x58` stays `-1`),
the head-to coordinate is never written, so the turn cannot be blocked by terrain the path
search did not consider. Active in YR: yes — this is the ordinary Drive path, no
`SpecialFlags` gate, no TS-only branch. Note the gate demands *exact* equality, so it also
runs the last frame of every ordinary turn.

**Rust has no equivalent gate.** `handle_vehicle_rotation` — the only in-place rotation
Rust owns — is unreachable for Drive units on this path: `issue_move_command`
(`src/sim/movement/movement_commands.rs:610..624`) sets `facing_target = None` on *every*
Drive branch, whether a track started or not. **NOT-IMPLEMENTED.**

## Stage 4 — What current Rust actually does (exact source reduction)

`Command::Move` → `issue_move_command` (`movement_commands.rs:494..628`):

1. `new_facing = facing_from_delta(-1, 0) = 0xC0`; `initial_step_delta = (-1, 0)`.
2. `select_drive_track(entity.facing = 0x40, 0xC0, false)` (`drive_track.rs:3466..3509`)
   quantizes **both arguments from facings**: `from_dir = 2`, `to_dir = 6`,
   `turn_index = 2·8 + 6 = 22`. `TURN_TRACKS[22].normal_track == 0` → `None`.
3. `build_sharp_turn_fallback(entity.facing = 0x40)` (`drive_track.rs:3520..3543`):
   `cur_dir = 2` → `turn_index = 18` → RawTrack `1`, flags `3`, target facing `0x40`.
4. `dir_to_cell_delta(0x40)` = `DIRECTION_DELTAS[2]` = **`(+1, 0)` — east**
   (`src/util/direction.rs:12..21`).
5. `begin_drive_track(1, 3, +1, 0, 0x40)` → `head_offset = (384, 128)`,
   `point_index = RAW_TRACKS[1].entry_index = 0`.
6. `movement.next_index += 1` — **the first westward path node is discarded.**
7. `facing_target = None`.

The same four-line block is duplicated at `movement_step.rs:123..156`,
`movement_step.rs:816..837`, `movement_step.rs:910..931`, and `movement_tick.rs:585..614`;
all five sites key the fallback on `entity.facing`.

`TRACK1_POINTS` (`drive_track.rs:843..`) is straight north — `x = 0`, `y = 245, 234, 223 …`,
`facing = 0` — and `transform_track_point` with flags `3` (`drive_track.rs:44..62`) applies
swap then negate-x: `tf = -(-0 - 0x40) = 0x40`, `(x, y) → (-y, 0)`. So the transformed
series is `sub_x = 384 - y`, `sub_y = 128`, **facing constant `0x40`**.

Literal current-Rust body-facing series through the "reversal", one entry per native frame:

```text
frame 0..N :  0x40, 0x40, 0x40, 0x40, 0x40, …   (never changes)
```

Literal current-Rust position series (sub-cell x, cell `(50,50)` frame of reference):

```text
139, 150, 161, 172, 183, 194, 205, 216, 227, 238, 249, 260 → cell jump into (51,50) …
```

— i.e. it starts by teleporting 11 leptons **east** of the cell center and keeps going east.
After the track finishes, `configure_motion_after_transition`
(`movement_step.rs:112..191`) recomputes `next_face = 0xC0` from the *new* cell, hits
`TURN_TRACKS[22].normal_track == 0` again, takes the fallback again, drives east again, and
discards another node. When the path row is exhausted the auto-repath at
`movement_tick.rs:251..258` builds a fresh westward segment from the new cell — and the
body facing is still `0x40`, so the cycle repeats. Whether any higher layer ever breaks the
loop (map edge, blocker → `try_repath_after_block`) is outside this bounded reduction and
is recorded **UNCHECKED**; the first-visible-motion result is not.

There is no test guarding the correct behaviour — the existing suite pins the *wrong*
quantity: `sharp_turn_fallback_produces_valid_track_for_all_8_dirs`
(`drive_track_tests.rs:454..480`) asserts precisely that the substitute drives forward
along `dir_to_cell_delta(current_facing)`.

## Stage 5 — The native per-frame facing series

Native installs `Do_Turn(0xC000)` on the first Process call. `RateTimer::Set @ 0x004c9220`
snapshots the live value into `prev`, writes `current = 0xC000`, `start = g_CurrentFrameCounter`,
`duration = abs(diff)/rot`. With `prev = 0x4000`, `diff = (short)(0xC000 - 0x4000) = -32768`
(exactly 180° resolves **counter-clockwise**, i.e. E → NE → N → NW → W, through north),
`rot = 1280`, `step = 32768/1280 = 25`, `per_step = -32768/25 = -1310`.

Closed-form reduction of the freshly decompiled `Current()` body, `value = 0xC000 + 1310·remaining`:

```text
frame  +0  +1  +2  +3  +4  +5  +6  +7  +8  +9 +10 +11 +12
byte  3F  3A  35  30  2B  26  21  1C  16  11  0C  07  02
frame +13 +14 +15 +16 +17 +18 +19 +20 +21 +22 +23 +24 +25
byte  FD  F8  F3  EE  E8  E3  DE  D9  D4  CF  CA  C5  C0
```

Position is unchanged for all 26 samples. On frame +25 the gate passes, and only then does
`Process_Movement` fall through to `iVar5 = 6 + 6·8 = 54` → RawTrack 1 → the westward drive.

This series is an exact algebraic reduction of `0x004c9220` + `0x004c93d0` for this one
fixture, but it was **not executed** against an emulator or a live capture, so it is
recorded **UNCHECKED** rather than promoted. (Note the truncating `per_step` means frame +0
reads `0x3F`, not `0x40` — a real one-byte artefact of the native formula, faithfully
reproduced by Rust's `facing_class.rs` if that path were reachable.)

## Stage 6 — Fresh track cursor

`search_instructions function=DriveLocomotionClass__Process_Movement operand_pattern="0x5c"`
returns exactly one write:

```text
004b4659  MOV  dword ptr [EBP + 0x5c], 0x0
```

Native unconditionally zeroes the point cursor at `Drive+0x5C` after selecting a track.
Rust `begin_drive_track_with_head_offset` (`drive_track.rs:3642..3663`) sets
`point_index: meta.entry_index`. The prior FAIL therefore **still stands at HEAD** — no
commit in `git log -- src/sim/movement/drive_track.rs` addresses it. It is *not* observable
in this fixture, because both the native fallback (RawTrack 1) and the Rust fallback
(RawTrack 1) have `entry_index = 0`; it bites on every real curve (`entry_index` 11, 12, 15,
16 on RawTracks 3–6).

## Stage 7 — Render handoff

AMCV has no `Turret=`, so `src/app_instances/units.rs:263..300` takes the non-turret branch
and emits one composite sprite keyed on `canonical_unit_facing(entity.facing)`.
`UNIT_FACING_STEP = 1`, `UNIT_FACING_BUCKETS = 256` (`src/render/unit_atlas.rs:33..50`) —
the body voxel is pre-rendered at **all 256 discrete facings**, so
`canonical_unit_facing` is the identity function and **no quantization happens at render
time**. Every 1/256-turn step the sim produces is a distinct rendered pose.

That is the right call for fidelity, and it is exactly why this drift is so visible: render
quantization cannot mask a facing error. The player sees the hull frozen at east for the
entire manoeuvre where gamemd shows a 26-pose sweep through north. Whether gamemd itself
caches 256 body facings for voxel bodies was not checked this session — **UNCHECKED**, and
irrelevant to the verdict, since the sim-side series is what differs.

## Stage verdicts

| # | Stage | Verdict | Bounded result |
|---:|---|---|---|
| 1 | Retail `[AMCV]` bindings (Drive CLSID, `Speed=4`, `ROT=5`, no `Turret=`) | PASS | Exact stock inputs agree. |
| 2 | Hull facing identity = `TechnoClass+0x388`, turned by locomotor slot `+0x4C` | PASS | Re-bound from vtable bytes and `0x004B0EF0` body. |
| 3 | ROT units/scaling (`rot_byte << 8`, 16-bit facing space) | PASS | Rust `facing_class.rs` matches fresh `0x004c93d0` decompile. |
| 4 | Turn duration formula (`delta_8 / ROT`; 180° = 25 frames at ROT=5) | PASS | Identical integer reduction both sides. |
| 5 | Facing applied per binary frame, not per sim tick | PASS | `binary_frame` = 1 per 3 sim ticks; no ×3 multiplier on facing. |
| 6 | Stock wall-clock duration of the 180° turn | FAIL | ≈0.40 s native vs ≈1.19 s Rust (~3.0× slow) — engine-wide calibration gap. |
| 7 | Drive speed-fraction cadence | FAIL | `update_drive_speed_fraction` still mutates 3× per native frame (`movement_tick.rs:1246`). |
| 8 | TurnTrack base/stride/record layout | PASS | `0x007E7B28`, 12-byte stride, verified from `004b4023`/`004b403a` operands. |
| 9 | RawTrack base/stride/field order | PASS | `0x007E7A28`, 16-byte stride; brief's `0x007E7A68` is wrong. |
| 10 | TurnTrack[22] content (the `0x40→0xC0` pair) | PASS | Binary `00 00 00 00 · c0 · 00` = Rust `{0, 0, 0xC0, 0}`. |
| 11 | TurnTrack[18]/[54] and RawTrack[1] content | PASS | Byte-for-byte agreement with Rust constants. |
| 12 | Track-index operands (path-step octants vs body facing) | FAIL | Native uses `path[0]`/`path[1]`; Rust uses `entity.facing`/next-cell facing. |
| 13 | Facing-alignment gate before track selection | NOT-IMPLEMENTED | Rust clears `facing_target` on every Drive branch; no gate exists. |
| 14 | Turn-in-place vs arc for the reversal | FAIL | Native rotates in place at the origin cell; Rust translates a full cell. |
| 15 | Direction of first visible motion | FAIL | Native: none for 25 frames, then west. Rust: immediately **east**. |
| 16 | Sharp-turn fallback direction (`cur_dir·9`) | FAIL | Native `cur_dir` = path-step octant (W); Rust = body facing (E). |
| 17 | Path-node consumption during the sharp turn | FAIL | Native consumes nothing; Rust does `next_index += 1` at five call sites. |
| 18 | Fresh track cursor (`0` vs `entry_index`) | FAIL | `004b4659` writes 0; Rust seeds `meta.entry_index`. Not observable in this fixture. |
| 19 | Per-frame body-facing series through the reversal | FAIL | Rust series is constant `0x40`; native sweeps `0x3F…0xC0` over 26 samples. |
| 20 | Literal native facing series | UNCHECKED | Derived in closed form from `0x004c9220`/`0x004c93d0`; not executed. |
| 21 | Render facing → voxel key, 256 buckets | PASS | Identity mapping; quantization cannot mask the drift. |
| — | Whether Rust ever reaches `(45,50)` | UNCHECKED | Auto-repath re-arms with the same east-facing body; loop exit not bounded here. |

## Top root findings

1. **The fallback is keyed on the wrong quantity.** Native's `uVar18` in `cur_dir · 9` is
   the *path-step octant*; Rust passes the *body facing*. Every one of the five Rust call
   sites repeats it. For a reversal the two are 180° apart, which is why the unit drives
   exactly backwards. Fixing this one substitution turns a directional bug into a merely
   cosmetic one.
2. **The facing gate is the missing mechanism, not the fallback.** Native reaches the
   fallback only when consecutive *path steps* differ by ≥3 octants — a genuinely sharp
   corner in the route. A unit that merely *starts* misaligned never gets there: the gate at
   `004b3410..004b3439` rotates it in place first. Rust has no gate, so it routes ordinary
   misalignment into the sharp-corner substitute.
3. **Path nodes are being destroyed.** `next_index += 1` on the fallback has no native
   counterpart anywhere in `Process_Movement`; the native turn branch returns `1` before any
   node is touched. This is the mechanism that lets the error compound cell after cell
   instead of self-correcting.
4. **The in-place rotation Rust does own is dead code for Drive.** The frame-based
   `FacingClass` migration in `handle_vehicle_rotation` is correct and matches the binary —
   and `issue_move_command` unconditionally clears `facing_target` for Drive units, so it
   never runs. Correct code carrying cost with no player benefit.
5. **Render will not hide any of it.** 256 body facing buckets, identity mapping. The frozen
   hull and the wrong travel direction are both fully exposed.

## Smallest decisive follow-up

Add one headless fixture: AMCV at `(50,50)` facing `0x40`, Move to `(45,50)`, assert that
after the first native frame the entity's cell is still `(50,50)` and that within 25 native
frames its facing reaches `0xC0` having passed through `0x00`. That single assertion fails
today for three independent reasons (no gate, wrong fallback direction, node consumption)
and passes only when all three are native. It needs no oracle and no new tables — the
TurnTrack/RawTrack constants are already byte-correct.

## Fresh read-only Ghidra calls

Program `gamemd.exe`, project `testProsjekt`. No mutations of any kind.

- `decompile_function 0x004B2630` — full `Process_Movement` body.
- `decompile_function 0x004B0500` — Process dispatch; confirms frame-0 entry.
- `decompile_function 0x004c93d0` — `FacingClass::Current` interpolation formula.
- `disassemble_function 0x004B0EF0` — `Do_Turn` → `+0x388`, `RateTimer::Set`.
- `disassemble_bytes 0x004b33f0 len 80` and `0x004b343b len 48` — the facing gate and its early return.
- `search_instructions operand_pattern="7e7b"` — TurnTrack base/stride from live operands.
- `search_instructions function=DriveLocomotionClass__Process_Movement operand_pattern="0x5c"` — the single cursor-0 write at `004b4659`.
- `read_memory 0x007E7B28 len 96`, `0x007E7C00 len 24`, `0x007E7C18 len 72`, `0x007E7DA4 len 36` — TurnTrack rows 0–7, 18–19, 20–25, 53–55.
- `read_memory 0x007E7A28 len 64` — RawTrack rows 0–3.
- `read_memory 0x007E7EB0 len 128` — Drive `ILocomotion` vtable; slots `+0x40`, `+0x4C`, `+0x70`.
- `list_globals name_substring="g_DriveTrack"` — label survey (hints only; every address re-derived above).
