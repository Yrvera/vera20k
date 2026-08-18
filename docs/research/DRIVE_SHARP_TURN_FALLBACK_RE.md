# Drive Sharp-Turn Fallback — Focused RE Report

**Address(es):** `0x4b4023` (Process_Movement substitute site), `0x4b1d30` ish (Process_Drive_Track chain branch)
**Confidence:** High (every claim verified against decompiled assembly)
**Active in YR:** Yes — core movement; runs every tick for every drive vehicle.

**Scope:** Three parity-blocking questions for `/brainstorm drive sharp-turn
fallback`. Not a general decompilation pass — the broader Process_Movement and
Process_Drive_Track functions are already covered in
`DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` and
`PROCESS_DRIVE_TRACK_DECOMPILATION.md`. This report locks the three details
those reports left ambiguous.

**Source gap-scan:**
[2026-05-05c-gap-scan-drive_track-deep.md D2 §1](../ra2-rust-game/docs/gap-scans/2026-05-05c-gap-scan-drive_track-deep.md).

---

> ## ⚠ CORRECTION 2026-07-29 — read before using Q2
>
> **Q2's observation is right; its framing is misleading and has caused a real bug.**
>
> Q2 answers "**Path queue shifted left by 1** (impossible step permanently consumed)",
> which reads as though the shift were a consequence *of the substitute*. It is not. The
> shift at `0x4b4607` sits in the **shared no-cell-crossing tail** that every track with
> `flags & 8 == 0` reaches — substitute or not — so it is the **ordinary one-node
> consumption of any non-cell-crossing step**.
>
> Verified via `disassemble_bytes 0x004b4016 len 64`: the substitute store at `0x004b4031`
> (`track_index = cur_dir*9`) **falls through** to `0x004b4034`, which is exactly where the
> normal path lands via the `JNZ` at `0x004b402c`. The two paths **converge before** the
> `flags & 8` test at `0x004b403a` and the `JZ 0x004b45f6` at `0x004b4045`. §3.1 and §3.2
> below are therefore describing shared tail behaviour, not substitute-specific behaviour.
>
> **Consequence, and why this matters:** the Rust port read Q2 as licence to consume an
> extra path node in the sharp-turn fallback, *on top of* its ordinary per-step advance —
> so it dropped two nodes where gamemd drops one. That put the vehicle one waypoint
> off-route on every ≥135° turn and produced the non-adjacent path step that a second
> invented gate then cancelled move orders over. Both were retired in `a093e9ee`.
>
> **The accurate statement is: the substitute consumes exactly one node, like any straight
> step — it never consumes a second.**

## 1. Question summary

| # | Question | Answer | Confidence |
|---|---------|--------|-----------|
| Q1 | Does the `lea ecx,[ebx+ebx*8]` fallback at `0x4b4023` fire on `short_track==0` (use_short path) or only on `normal_track==0`? | **Only `normal_track==0`.** Process_Movement reads byte +0x00 unconditionally at this site. The `use_short` flag (`loco+0x60`) is *cleared to 0* immediately before the lookup at `0x4b4019`. | HIGH |
| Q2 | After the substitute, does Process_Movement advance the path index, leave it, requeue, or call Find_Path? Is `loco->dest` adjusted to the straight-ahead cell or left pointing at the original sharp-turn target? | **Path queue shifted left by 1** (impossible step permanently consumed). **`loco->dest` NOT touched.** `loco->head_to` set to NullCoord. `loco->point_index` reset to 0. `loco->is_reversed` cleared. No Find_Path call. | HIGH |
| Q3 | In Process_Drive_Track's chain branch, is there a `facing*9` substitute when the chained-into entry has `normal_track==0`? | **No substitute.** Chain is gated `if (normal_track != 0 && raw_track.chain_index != 0)`. If either fails, the entire chain block is skipped — current track finishes, next tick re-enters Process_Movement which may then trigger the regular fallback. | HIGH |

---

## 2. Q1 — Substitute trigger condition (Process_Movement)

**Site:** `0x4b4023`, the second-to-last instruction in the track-selection
phase of `DriveLocomotionClass::Process_Movement`.

### 2.1 Exact assembly

```
; ESI = next_entry (path-requested direction, possibly forced to EBX earlier
;       by the crush check at 0x4b3ff9..0x4b400a)
; EBX = current_direction (quantized to 0-7)
004b4016: LEA EAX, [ESI + EBX*8]                     ; track_index = next_entry + cur_dir*8
004b4019: MOV byte ptr [EBP + 0x60], 0x0             ; loco.is_reversed = 0    *** clears use_short ***
004b401d: MOV dword ptr [EBP + 0x58], EAX            ; loco.track_index = track_index
004b4020: LEA EAX, [EAX + EAX*2]                     ; EAX *= 3
004b4023: MOV CL, byte ptr [EAX*4 + 0x7e7b28]        ; CL = TURN_TRACKS[track_index].normal_track
                                                     ; (offset +0x00, never +0x01 short_track)
004b402a: TEST CL, CL
004b402c: JNZ 0x004b4034                             ; if normal_track != 0 → continue
004b402e: LEA ECX, [EBX + EBX*8]                     ; ECX = cur_dir * 9
004b4031: MOV dword ptr [EBP + 0x58], ECX            ; loco.track_index = cur_dir * 9 (FALLBACK)
004b4034: MOV EAX, dword ptr [EBP + 0x58]            ; reload (post-fallback) track_index
004b4037: LEA EDX, [EAX + EAX*2]                     ; EDX *= 3
004b403a: TEST byte ptr [EDX*4 + 0x7e7b30], 0x8      ; flags & 8 (cell-crossing bit)
004b4042: MOV EAX, dword ptr [EBP + 0xc]
004b4045: JZ  0x004b45f6                             ; bit unset → no-cell-crossing path
                                                     ; bit set   → cell-crossing path at 0x4b404b
```

### 2.2 The `use_short` story

Process_Movement does **not** read `short_track` (+0x01) at this site. It
unconditionally reads `normal_track` (+0x00), and explicitly clears
`loco.is_reversed` (the use_short flag, +0x60) one instruction earlier.

`is_reversed`/use_short *is* read in **Process_Drive_Track** (the active-track
stepping function) at multiple sites:

```c
// Process_Drive_Track top, ~line 460 of decompiled output:
if (*(char *)(param_1 + 0x60) == '\0') {
    cVar5 = *pcStack_74;              // [+0x00] = normal_track
} else {
    cVar5 = (&DAT_007e7b29)[iVar7];   // 0x7e7b29 = base+1 = [+0x01] = short_track
}
```

So `use_short` is a runtime flag that influences **per-step lookups during
active stepping**, but Process_Movement (the per-cell-transition reroute) sees
it cleared by 0x4b4019 and always indexes `normal_track`.

### 2.3 Tiny-detail catch — there's a SECOND override path

The `lea ecx,[ebx+ebx*8]` at `0x4b402e` is **not the only route to track_index
= cur_dir*9** in Process_Movement. The crush check earlier at
`0x4b3ff5..0x4b400c` also forces this:

```
004b3ff5: PUSH 0x0
004b3ff7: MOV  ECX, EDI
004b3ff9: CALL 0x47eba0                    ; some "is blocked / can crush" check
004b3ffe: TEST EAX, EAX
004b4000: JNZ  0x4b400a                    ; non-zero → ESI = EBX
004b4002: MOV  AL, byte ptr [ESP + 0x13]   ; load local flag
004b4006: TEST AL, AL
004b4008: JZ   0x4b4012                    ; zero → skip override
004b400a: MOV  ESI, EBX                    ; ESI = current_dir
004b400c: MOV  byte ptr [EBP + 0x64], 0x1  ; loco+0x64 = 1 ("crush" or override flag)
```

When this fires, ESI gets forced to EBX (current_dir), so the LEA at 0x4b4016
computes `EBX + EBX*8 = cur_dir*9` — same destination as the
normal_track==0 fallback, but **for a different reason** (crush/blocked).

For the brainstorm: the engine has **two distinct triggers** that both
end with `track_index = cur_dir * 9`:

1. **Crush/block override** (0x4b3ff9 helper returns nonzero, OR local flag
   [ESP+0x13] is set) — sets `loco+0x64 = 1` (a "crush armed" or similar flag)
   along with the override.
2. **Sharp-turn fallback** (TURN_TRACKS[track_index].normal_track == 0) —
   pure substitute, no extra flag set.

The Rust gap analysis only flagged path #2. Path #1 is a separate parity gap
that uses the same `cur_dir*9` mechanic but should not be conflated. Calling
this out so the brainstorm doesn't accidentally fold them together.

---

## 3. Q2 — Post-substitute control flow (Process_Movement)

After 0x4b4031 sets `loco.track_index = cur_dir*9`, control flows to 0x4b4034.

### 3.1 Cell-crossing flag for substitute entries

At 0x4b403a, `flags & 8` is tested on the **substituted** track_index. For
the eight `cur_dir*9` entries (TURN_TRACKS indices 0, 9, 18, 27, 36, 45, 54,
63), the flag bytes (read from binary memory at `0x7e7b28`):

| cur_dir | TT idx | normal_track | target_facing | flags | flags & 8 |
|---:|---:|---:|---:|---:|:---:|
| 0 (N)  | 0  | 1 | 0x00 | 0x00 | 0 |
| 1 (NE) | 9  | 2 | 0x20 | 0x00 | 0 |
| 2 (E)  | 18 | 1 | 0x40 | 0x03 | 0 |
| 3 (SE) | 27 | 2 | 0x60 | 0x04 | 0 |
| 4 (S)  | 36 | 1 | 0x80 | 0x04 | 0 |
| 5 (SW) | 45 | 2 | 0xA0 | 0x01 | 0 |
| 6 (W)  | 54 | 1 | 0xC0 | 0x01 | 0 |
| 7 (NW) | 63 | 2 | 0xE0 | 0x02 | 0 |

**Every substitute entry has `flags & 8 == 0`.** So the JZ at 0x4b4045
**always** taken post-substitute → control jumps to `0x4b45f6` (the
no-cell-crossing path). **The cell-crossing validation at 0x4b404b never runs
for fallback paths.** The cell beyond the next is never validated — the unit
just commits to driving forward.

### 3.2 The no-cell-crossing tail (0x4b45f6 onwards)

```
; Path queue shift (consume one path step)
004b45f6: LEA   ESI, [EAX + 0x5e4]      ; ESI = techno + 0x5e4 = path_queue[1]
004b45fc: LEA   EDI, [EAX + 0x5e0]      ; EDI = techno + 0x5e0 = path_queue[0]
004b4602: MOV   ECX, 0x17               ; 23 dwords
004b4607: REP MOVSD                      ; path_queue[1..24] → path_queue[0..23]
004b4609: OR    EDI, 0xFFFFFFFF
004b460c: MOV   ECX, [EBP + 0xC]         ; ECX = techno
004b460f: LEA   ESI, [EBP + 0x40]        ; ESI = loco+0x40 = head_to
004b4612: MOV   [ECX + 0x63c], EDI        ; path_queue[23] = -1 (sentinel)

; Compute target screen-cell (for some other use; written at techno+0x558)
004b4618..0x4b4649  (CDQ + AND 0xff + ADD + SAR 8 dance — signed
                     /256 with rounding-toward-zero correction)

; Bridge transition flag clear
004b464f: MOV   ECX, [EBP + 0xC]
004b4652: MOV   byte [ECX + 0x68a], 0    ; bridge_transition_flag = 0

; Reset point_index — track restarts from beginning
004b4659: MOV   [EBP + 0x5c], 0          ; loco.point_index = 0

; Set head_to = NullCoord (with skip-if-already-null guard)
004b4660: MOV   EAX, [0x008a0790]        ; NullCoord.X = 0
004b4665: MOV   ECX, [ESI]               ; ECX = head_to.X
004b4667: CMP   ECX, EAX
004b4669: JNZ   0x4b4685                 ; if differ → write
004b466b: MOV   EDX, [ESI + 0x4]
004b466e: MOV   ECX, [0x008a0794]
004b4674: CMP   EDX, ECX
004b4676: JNZ   0x4b4685
004b4678: MOV   ECX, [ESI + 0x8]
004b467b: MOV   EDX, [0x008a0798]
004b4681: CMP   ECX, EDX
004b4683: JZ    0x4b46a3                 ; all 3 match → skip the write block
004b4685: MOV   EDX, ESI                 ; write head_to = NullCoord
004b4687: MOV   [EDX], EAX                ; head_to.X = 0
... (head_to.Y = 0, head_to.Z = 0)
```

### 3.3 Tiny-detail summary table

| State | Modified by post-substitute path? | New value |
|---|---:|---|
| `loco.track_index` (+0x58) | yes (at 0x4b4031) | `cur_dir * 9` |
| `loco.is_reversed` / use_short (+0x60) | yes (at 0x4b4019, before substitute) | 0 |
| `loco.point_index` (+0x5c) | yes (at 0x4b4659) | 0 |
| `loco.head_to` (+0x40..0x48) | yes (at 0x4b4685+) | NullCoord (0,0,0) |
| `loco.dest` (+0x30..0x38) | **NO** | unchanged from Set_Destination |
| `loco.is_on_track` (+0x63) | **NO** in this branch | unchanged |
| `techno.path_queue[0..22]` (+0x5e0..0x63c) | yes (at 0x4b4607) | shifted left by 1 |
| `techno.path_queue[23]` (+0x63c) | yes (at 0x4b4612) | -1 |
| `techno.+0x558` (some screen coord) | yes (at 0x4b4649) | computed cell-coord pair |
| `techno.+0x68a` (bridge transition) | yes (at 0x4b4652) | 0 |
| `techno.+0x68b` (bridge transition flag set) | NO (only set in cell-crossing branch at 0x4b45ed) | unchanged |

### 3.4 Tiny-detail catches

- **`head_to` becomes NullCoord, NOT the cur_dir cell.** This is the same
  treatment legitimate straight tracks (entries 0, 9 — NE→NE same-dir) get.
  The unit's per-tick position during the track is computed *purely* from the
  transformed track points relative to the unit's *current* world position
  (`techno+0x9c..0xa4`), not from head_to. head_to=NullCoord signals "no
  external destination cell — track points define the motion".
- **`loco.dest` is never updated.** `loco.dest` was set by
  `DriveLocomotionClass::Set_Destination` at the original move order time and
  represents the *ultimate* destination, not the next-cell waypoint. The
  fallback substitute is a per-cell-transition decision that doesn't change
  the ultimate goal. Any speed-ramping / arrival check that reads `loco.dest`
  continues to see the same final destination — meaning the unit's
  deceleration profile is computed against the original final goal even when
  it's currently driving away from the next-step path cell.
- **`point_index` resets to 0** (not to the substituted track's
  `entry_index`). For RawTrack 1 (cur_dir=0/2/4/6) and RawTrack 2
  (cur_dir=1/3/5/7), `entry_index` is 0 anyway — so this is a no-op. But it's
  still worth noting: the binary writes 0, not `entry_index`. Any Rust
  implementation that uses `entry_index` for the point_index will agree
  *only* because these specific tracks happen to have `entry_index == 0`.
- **No Find_Path call.** The fallback does not invoke pathfinding to repair
  the route. The path is permanently shortened by one step; the unit accepts
  drifting off-route and relies on whatever drives the path-replanning higher
  up (FootClass-level or AI-level) to recover later.
- **Path consumption is permanent.** If the second path step is ALSO
  unreachable from the new position, the next tick triggers the fallback
  again — consuming another path step. **A 180° impossible-turn path with
  successive unreachable steps will see the unit drive forward in current
  facing repeatedly until the path is exhausted.**
- **`techno+0x558` write.** The compute at 0x4b4618..0x4b4649 stores a
  16:16 cell coord at `techno+0x558`. The CDQ/AND-0xff/ADD/SAR-8 sequence is
  the standard "signed lepton → cell with rounding toward zero" idiom — it
  divides by 256, biasing negative values up. This field is read elsewhere
  (probably the rendering layer for the next-cell destination overlay or a
  similar UI/AI cue). Worth flagging for a future scan, but not load-bearing
  for the fallback's parity.
- **No vtable Mission/Guard call here.** The substitute path doesn't invoke
  any mission state change — the SetMission(Guard)/Scatter_Force on track
  arrival is in Process_Drive_Track, not here.

### 3.5 Implication for parity

The Rust caller at
[`movement_step.rs:91`](../ra2-rust-game/src/sim/movement/movement_step.rs#L91)
currently passes `(ndx, ndy)` to `begin_drive_track` where (ndx, ndy) is the
**path's requested next direction**. For the substitute case, the binary's
behavior is equivalent to passing `(0, 0)` — head_to is NullCoord — and using
the cur_dir*9 entry's transform flags (which orient RawTrack 1/2 to
`current_dir`). The Rust port has to reconcile: either (a) substitute also
inside `select_drive_track` and let the caller still pass (ndx, ndy) but use
them only for the path step (which is then discarded), or (b) propagate the
substitute up so the caller knows to override (ndx, ndy) to (0, 0). Brainstorm
will decide.

---

## 4. Q3 — Chain-time fallback (Process_Drive_Track)

**Site:** Process_Drive_Track main loop, the chain attempt branch around the
`(*pcStack_bc != '\0')` test.

### 4.1 The chain gate

```c
// loop body, near the bottom of the inner do-while:
if ((((uStack_a0 != 8) && (uStack_a0 != 0xffffffff)) && (uStack_f0._3_1_ != '\0')) &&
    ((*(int *)(&DAT_007e7a2c + iStack_c4) == *(int *)(param_1 + 0x5c) &&  // chain_index == point_index
     (*(int *)(param_1 + 0x5c) != 0)))) {                                  // point_index != 0

    // Compute chain-target TurnTrack index
    uStack_9c       = (uint)CONCAT21(garbage, pcStack_74[4]) << 8;          // pcStack_74[+4] = current target_facing
    iStack_c0       = uStack_a0 + ((uStack_9c >> 0xc) + 1 >> 1 & 7) * 8;    // = next_path_dir + quantized_target_dir * 8
    pcStack_bc      = &g_DriveTrackIndex_Table + iStack_c0 * 0xc;           // ptr to TurnTrack[iStack_c0]

    if ((*pcStack_bc != '\0') &&                                            // normal_track != 0
        (*(int *)(&DAT_007e7a30 + *pcStack_bc * 0x10) != 0)) {              // RawTrack[normal_track].chain_index != 0

        // ... Can_Enter_Cell dispatch + chain-success path ...
    }
}
*(int *)(param_1 + 0x5c) = *(int *)(param_1 + 0x5c) + 1;   // advance point_index
```

### 4.2 Behavior when `normal_track == 0`

If `*pcStack_bc == '\0'` (`normal_track == 0` for the chain-target TurnTrack
entry), the entire `(*pcStack_bc != '\0' && ...)` body is skipped. **No
substitute, no Can_Enter_Cell call, no track replacement, no path-queue
shift.** Control falls through to `point_index += 1` and the loop continues
on the **current** track. The current track will eventually hit its end
(point_index reaches `points_count`), at which point the normal end-of-track
handling fires and control returns to Process_Movement on the next tick.

The new tick's Process_Movement will then either:
- Find a normal track for the new current-facing → next-path-step turn, OR
- Hit the same `normal_track == 0` condition at 0x4b4023 → trigger the
  Process_Movement fallback (Q1/Q2).

### 4.3 Tiny-detail catches

- **The chain *also* depends on `RawTrack[normal_track].chain_index != 0`**
  (second clause). Even if `normal_track != 0`, if the corresponding raw
  track has `chain_index == 0`, chain is refused. This is a separate
  refusal mode from the `normal_track==0` case.
- **Chain quantization uses `target_facing` of the CURRENT track, not
  current `loco.facing`.** `pcStack_74[+4]` is byte +0x04 of the current
  TurnTrack entry, which is `target_facing`. The chain target is computed
  from "where this turn will leave us" + "where the path wants to go next".
  This is subtle: a chain attempt during a 45° turn sees the *post-turn*
  facing, not the *current mid-turn* facing.
- **Quantization formula:** `((target_facing >> 4) + 1) >> 1 & 7`. For
  target_facing 0x00 → 0 (N); 0x20 → 1 (NE); 0x40 → 2 (E); ...; 0xE0 → 7
  (NW). Same +16 rounding the rest of the engine uses. No off-by-one here.
- **Chain success uses `is_reversed = 0`** (cleared explicitly at the chain
  success site, mirrored to `param_1 + 0x60 = 0`). So a chain into a new
  track always uses `normal_track`, never `short_track` — same as
  Process_Movement.
- **`point_index` reset on chain success: `chain_index - 1`, not 0.** Per
  PROCESS_DRIVE_TRACK_DECOMPILATION.md §6b and confirmed in this
  decompilation: `*(int *)(param_1 + 0x5c) = *(int *)(&DAT_007e7a30 +
  iStack_c4) + -1`. This is **D4 §1** of the gap-scan, separate from this
  question — noting here only because it's adjacent in the same code block.

### 4.4 Implication for parity

The Rust chain attempt at
[`movement_tick.rs:701`](../ra2-rust-game/src/sim/movement/movement_tick.rs#L701)
calls `select_drive_track(cur_face, next_face, false)` — but uses
`entity.facing` as `cur_face`. This is **wrong** vs the binary, which uses
the current track's `target_facing` (the post-turn facing). However, that's
a separate bug from this question; the answer here is just: **no fallback
substitute at chain time, refusal is a refusal, current track finishes
out**. The Rust side should mirror this — when chain `select_drive_track`
returns None, do nothing (current implementation already matches: leaves
the current track running).

---

## 5. Cross-cutting findings

These came up while answering the three questions and are worth flagging so
they don't get re-discovered next session.

### 5.1 The substitute does not "approximate" — it commits

Once the substitute fires, the path step is gone. There is no
"opportunistic resume" — the unit doesn't try to rotate into the original
direction once it gains room. It drives in current_dir for one full cell.
If the next path step is also unreachable, fallback fires again, and the
unit walks farther off-route. The recovery mechanism is at a higher layer
(FootClass / AI replanning), not in Process_Movement.

### 5.2 The substitute's track is RawTrack 1 or 2, transform-rotated

| cur_dir | TT idx | normal_track | RawTrack | Transform flags (low 3 bits) |
|---:|---:|---:|---|---|
| 0 (N)  | 0  | 1 | straight (23 pts) | 0 — no transform |
| 1 (NE) | 9  | 2 | straight diagonal (31 pts) | 0 — no transform |
| 2 (E)  | 18 | 1 | straight | 3 = bit0+bit1 (mirror+rotate) |
| 3 (SE) | 27 | 2 | straight diagonal | 4 = bit2 |
| 4 (S)  | 36 | 1 | straight | 4 = bit2 (180° flip) |
| 5 (SW) | 45 | 2 | straight diagonal | 1 = bit0 |
| 6 (W)  | 54 | 1 | straight | 1 = bit0 |
| 7 (NW) | 63 | 2 | straight diagonal | 2 = bit1 |

So the engine reuses RawTracks 1 (cardinal) and 2 (diagonal) for all 8
directions via the lower-3-bit transform flag. The Rust side already has
these transforms working
([`drive_track.rs::transform_track_point`](../ra2-rust-game/src/sim/movement/drive_track.rs)).

### 5.3 Rust's `select_drive_track(use_short: bool)` parameter mismatch

`select_drive_track` in Rust takes `use_short: bool` and lets the caller
pick between `normal_track` and `short_track` at selection time. The binary
does NOT do this at the equivalent site — Process_Movement always reads
`normal_track`, and `use_short` (loco+0x60) is consulted only inside the
already-active track's stepping (in Process_Drive_Track). Whether this
matters for parity depends on what triggers `use_short` to be set in the
binary in the first place (a pre-Process_Movement decision). Out of scope
for this report; flag for a future investigation if `use_short=true` paths
ever start showing parity drift.

### 5.4 NullCoord global

`g_NullCoord_Drive_X/Y/Z` at `0x008a0790/94/98` are initialized to 0,0,0
exactly once by `DriveLocomotionClass::InitNullCoords` at 0x4af4e0 (one
writer found via byte-pattern search for `A3 90 07 8A 00`). They are never
reassigned at runtime — they're effectively a 12-byte zero constant. Used
by `head_to == NullCoord` checks throughout the drive code as a "no
destination" sentinel.

---

## 6. Verification spot-checks

Per the verification pass requirement, two findings re-checked:

1. **Q1 verification:** The 0x4b4023 instruction byte sequence in the
   disassembly output is `8A 0C 85 28 7B 7E 00` (MOV CL, byte ptr
   [EAX*4 + 0x7e7b28]). Byte +0x00 of TurnTrack entries is `normal_track`
   per PROCESS_DRIVE_TRACK_DECOMPILATION.md §field-table and confirmed by
   reading the table at 0x7e7b28 directly (entry 0: `01 00 00 00 ...` =
   normal_track=1 matches Rust TURN_TRACKS[0]). ✓

2. **Q2 path-queue shift verification:** The `REP MOVSD` at 0x4b4607 with
   ECX=0x17 (=23) and ESI/EDI pointing to techno+0x5e4 / techno+0x5e0
   shifts 23 dwords. path_queue is 24 dwords (+0x5e0 to +0x63c, per
   DRIVE_LOCOMOTION_HELPERS report — 24 entries × 4 bytes = 96 bytes,
   ending exactly at +0x63c which is then set to -1 at 0x4b4612). 23
   dwords copied + last entry set to -1 = full shift-left-by-1. ✓

---

## 7. Open questions (out of scope for this report)

- **What flag controls Process_Drive_Track's `use_short`?** Q1 leaves the
  question of when the binary actually elects use_short=1 unanswered — that
  decision is upstream of Process_Movement. Likely candidates: a per-unit
  flag set by AI/scripting, or a speed-class derived flag. Worth a follow-up
  RE if `short_track` parity ever becomes load-bearing.
- **What is `loco+0x64`?** Set to 1 by the crush-override path at
  0x4b400c, set to 0 elsewhere. Likely "this fallback was due to a
  crushable obstacle" but unverified. Distinct from the sharp-turn fallback
  but reaches the same `cur_dir*9` track_index.
- **What is `techno+0x558`?** Written by 0x4b4649 in the no-cell-crossing
  tail. Looks like a 16:16 cell coord, possibly used by the AI/render layer.
  Not consumed by drive logic.
- **Is the chain's `RawTrack.chain_index != 0` second clause ever hit
  independently of `normal_track != 0`?** I.e., are there any TurnTrack
  entries where `normal_track != 0` but the corresponding RawTrack's
  `chain_index == 0`? Looking at Rust RAW_TRACKS: tracks 1, 2, 7-10 have
  `chain_index = -1` (which the binary stores as 0xFFFF... — but the binary
  test `!= 0` would PASS for -1 because -1 is not zero in unsigned). So
  this second clause likely only fails for RawTrack 0, which `normal_track`
  already excludes. Defensive code, not a separate refusal mode in
  practice. Worth verifying if the chain ever appears to refuse
  unexpectedly.

---

## Sources

**Binary functions decompiled / inspected:**
- `0x4b2630` Process_Movement (track-selection phase 0x4b3ff5..0x4b4047
  + tail 0x4b45f6..0x4b46cb) — disassembly via Ghidra MCP
- `0x4b0f20` Process_Drive_Track (chain branch + use_short check) —
  full decompilation via Ghidra MCP
- `0x4af4e0` InitNullCoords (1 instruction sequence, identifies global
  layout) — xref + assembly
- `0x47eba0` (helper called at 0x4b3ff9 in the crush-override pre-check) —
  noted but not decompiled (out of scope)

**Memory inspected:**
- TurnTrack table at `0x7e7b28..0x7e7e88` (864 bytes, all 72 entries —
  cross-referenced against Rust `TURN_TRACKS` constant)
- NullCoord global at `0x008a0790..0x008a0798`

**Existing reports referenced:**
- DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md — Phase 8 track selection prose
  (high-level, doesn't differentiate Q1/Q2/Q3)
- PROCESS_DRIVE_TRACK_DECOMPILATION.md — chain branch sketch (matched live
  decompilation)
- DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md — field-offset table
- DRIVE_TRACK_TABLES_DEEP_DECODE.md — TurnTrack & RawTrack layouts

**Rust source cited:**
- [src/sim/movement/drive_track.rs:3447-3491](../ra2-rust-game/src/sim/movement/drive_track.rs#L3447) — `select_drive_track`
- [src/sim/movement/movement_step.rs:91-101](../ra2-rust-game/src/sim/movement/movement_step.rs#L91) — initial track-init caller
- [src/sim/movement/movement_tick.rs:701-717](../ra2-rust-game/src/sim/movement/movement_tick.rs#L701) — chain caller
