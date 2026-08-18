# AMCV Mid-Route Repath / Blocked-Cell Retrace — 2026-07-29

**Scenario:** stock YR `[AMCV]`, allied owner, exact center of flat clear Temperate `(50,50)`, ground Z, body facing east `0x40`; ordinary player Move to the exact clear-cell center `(60,50)`. The route is committed and the MCV is in motion. When the MCV reaches roughly `(53,50)`, an **allied intact `[GAWALL]`** is placed at `(57,50)` — a cell already inside the committed path queue, four cells ahead, not adjacent.

**Status:** **RED / NOT PARITY.** The two headline verdicts from `MCV_WALL_MID_ROUTE_REPATH_TRACE.md` (2026-05-27) both survive and one gets worse. The facing snap is re-confirmed FAIL and is now bounded by hard evidence: native does not merely "turn gradually", it **refuses to consume the new path node at all until body facing exactly equals the new direction**, so the MCV stops dead and pivots in place — Rust snaps the body 45° and keeps driving. Two new FAILs land underneath: Rust rolls a partially-consumed cell transition backwards to the cell center on every block, and commit `543ba0b9`'s deferred-repath retry is the **inverse** of native's failure policy on the same branch (native drops the destination; Rust re-arms an unthrottled per-tick retry). Same-tick repath, queue disposal, and the code-7 outcome for this exact cell do agree.

**Verdict tally:** **PASS 8 · FAIL 7 · UNCHECKED 3 · NOT-IMPLEMENTED 2** (20 bounded rows). A PASS certifies only its named row.

---

## Scope, freshness, and evidence discipline

- Investigation only. No Rust, INI, or asset was edited; no Cargo command was run; Ghidra access was read-only (no rename, no comment, no `save_program`). This report is the sole written artifact.
- Tree read at `ce096b3f`. Current source is authoritative over the 2026-05-27 trace, which is treated as navigation only. Where that trace's claim survived re-derivation it is marked as re-confirmed; where it was wrong it is corrected explicitly (§"Corrections to the prior trace").
- Native identity is not taken from labels. The UnitClass vtable is anchored through `get_xrefs_to 0x00738970` → single data reference at `0x007f60f4`, which fixes the vtable base at `0x007F5C70`; slot `+0x1AC` at `0x007F5E1C` reads `0x0073F0A0` (`read_memory 0x007F5E1C`), matching `get_function_by_address 0x0073F0A0` = `UnitClass__Can_Enter_Cell`. Every other slot below is read from that same table.
- **The decompiler's prototype for `DriveLocomotionClass::Process_Movement` is wrong and the prior trace inherited the error.** The function ends `RET 0xc` (`disassemble_bytes 0x004b3418..0x004b34df`, `0x004b3bef..0x004b3c8f`), i.e. three stack arguments, while Ghidra declares two. All argument-dependent claims below are read from assembly, not from the decompile.
- Literal per-tick lepton, facing, detour-cell and frame series were not executed against an oracle and stay `UNCHECKED`. Nothing is promoted from static plausibility.

## Retail inputs

- `ini/rulesmd.ini:6969..7010` — `[AMCV]`: `Speed=4`, `ROT=5`, `Crusher=yes`, `MovementZone=Normal`, `Size=6`, `Locomotor={4A582741-…}` (Drive). **No `Primary=` and no `Secondary=`** — the AMCV is weaponless. It has no turret, so body facing is the only visible orientation.
- `ini/rulesmd.ini:12022..12036` — `[GAWALL]`: `Wall=yes`, `Strength=300`, `Armor=concrete`, `Selectable=no`. `ini/rulesmd.ini:1736..1741` lists `3=GAWALL` in `[OverlayTypes]`.
- `ini/rulesmd.ini:58` — `CloseEnough=2.25`; `ini/rulesmd.ini:3106..3107` — `PathDelay=.01`, `BlockagePathDelay=60`.
- Rust constants: `src/sim/world/mod.rs:665..667` — `close_enough = 576` leptons (2.25 × 256), `path_delay_ticks = 9`, `blockage_path_delay_ticks = 60`.

---

## Stage 1 — Detection: when does native notice?

**Native trigger is a lazy, single-cell check performed at node adoption, not a queue revalidation and not an occupancy event.**

`DriveLocomotionClass__Process_Movement @ 0x004B2630` (`get_function_by_address 0x004B2630`, body `0x004b2630-0x004b4766`) is called only from `DriveLocomotionClass__Process @ 0x004B0500` at `0x004b0647` and `0x004b0a79` (`get_xrefs_to 0x004B2630`), and recursively from itself. It runs when the locomotor needs a new head-to, i.e. when the previous drive track has been consumed and the body is at rest at the current cell center.

Its only forward-looking passability call on the committed route is one `Can_Enter_Cell` on the **immediate next path cell**, issued through the owner vtable at `0x004b34c0` (`CALL dword ptr [EAX + 0x1ac]`, `disassemble_bytes 0x004b3418..0x004b34df`). The unit unmarks itself from the object list around the call (`CALL [EDX+0x124]` with `0` before and `1` after at `0x004b34ae` / `0x004b34d1`) so it cannot self-block. There is no scan of the remaining 22 queue entries, and nothing in the wall-placement path notifies a locomotor. The path queue at `foot+0x5E0` stores direction octants only, so a stale queue is only ever discovered one cell at a time.

Consequence for this fixture: the wall lands four cells ahead and native is indifferent to it for many frames. Detection fires on the tick the MCV, standing at `(56,50)`, tries to adopt the node into `(57,50)`.

Two secondary lookaheads exist and both are one cell further out:

- A **second-cell overlay precheck** in the track-selection block (decompiled body around `LAB_004b3f7d`): if the cell one step beyond the next carries an overlay whose type byte `+0x22D` is set — or `+0x2A8` is set for a mover whose TechnoType enum at `+0x5B4` equals `0xC` — the locomotor sets its own byte `+0x64` and **forces the straight track instead of a curve**. This is what stops a vehicle from swinging a two-cell curve into a wall it is about to be denied.
- A **curve-chain recheck**: when the selected track carries flag bit 3 (two-node curve), native issues a second `Can_Enter_Cell` on the cell two steps out at `0x004b4120` and dispatches the full 0–7 result again.

**Rust's primary trigger has the same shape.** `process_cell_crossings` (`src/sim/movement/movement_step.rs:1031`) evaluates only `target.path[target.next_index]`: `evaluate_runtime_can_enter_cell` at `movement_step.rs:1092..1102` and the terrain predicate at `movement_step.rs:1142..1178`. No event bus, no queue revalidation. Rust also has the curve-chain analogue — `AdvanceResult::DriveTrackChainReady` re-evaluates the cell at `next_index + 1` (`src/sim/movement/movement_tick.rs:1533..1596`).

Rust has **no** analogue of the second-cell overlay precheck. Nothing in `movement_tick.rs` or `drive_track.rs` inspects the overlay of the cell beyond the next in order to demote a curve to a straight track.

### Wall visibility timing

Native reads live `CellClass` state inside `Can_Enter_Cell`; the moment the wall overlay is planted it is visible to the next call. Rust's placement→passability path still lands outside the sim-tick loop: the per-tick loop closes at `src/app_sim_tick.rs:1368`, and `inject_placed_wall_overlays` + `rebuild_dynamic_path_grid` run after it at `src/app_sim_tick.rs:1382..1390`. Every sim tick in the frame that places the wall — and every later tick in that same frame — still reads the pre-wall `PathGrid`. **For this fixture the lag is invisible** (the wall is four cells ahead of the adoption point), but the mechanism is not native and does become visible when the wall lands on the cell being adopted this tick.

---

## Stage 2 — Result-code taxonomy for an allied intact GAWALL

`UnitClass__Can_Enter_Cell @ 0x0073F0A0` (`decompile_function 0x0073F0A0`) reaches the wall overlay slice only when the overlay type's byte `+0x2A8` is set. Inside it:

```text
if (ovt+0x22D == 0 || (TechnoType+0xD28 /*Crusher*/ == 0 && !HasWeaponAbility()))
   && (ovt+0x2A8 == 0 || TechnoType+0x5B4 != 0xC):
        if (!vtbl+0x2AC())                       -> return 7
        w = vtbl+0x3F8()                          // primary weapon
        if (!w.Warhead.Wall && (!w.Warhead+0x147 || ovt+0x9C != 6)) -> return 7
        if (!Is_Ally) { code = max(code, 5); break }
else:
        if (!Is_Ally) break                       // enemy crushable wall: code unchanged
code = max(code, 4)                               // allied wall
```

Slot `+0x2AC` on the UnitClass vtable is at `0x007F5F1C` and reads `0x00701120` (`read_memory 0x007F5F1C`). `decompile_function 0x00701120` is a five-line predicate: it calls vtable `+0x3F4` and returns 1 only if the returned weapon struct is non-null **and** its first field (the `WeaponTypeClass*`) is non-null. **`[AMCV]` declares no `Primary=` and no `Secondary=` (`ini/rulesmd.ini:6969..7010`), so this predicate is false and native returns `7` before ownership is ever consulted.**

So the answer to "does the caller branch differently for allied vs enemy?" is: **it would, but not for this unit.** A weaponless mover short-circuits to `7` on both allied and enemy walls; only an armed mover whose primary warhead has `Wall=true` reaches the `Is_Ally ? 4 : 5` split. The `+0x22D` value for GAWALL is `UNCHECKED`; if it is set, an armed-or-Crusher mover takes the else-branch and an allied GAWALL yields `4` instead. Either way the AMCV does not enter and does not crush: `Process_Movement`'s crusher downgrade of codes 4/5 additionally requires the cell's overlay index to be exactly `0` (`MOV EAX,[ESI+0x44]; TEST EAX,EAX; JNZ` at `0x004b3516..0x004b351b`, `disassemble_bytes 0x004b34de..0x004b356f`), and `3=GAWALL` in `[OverlayTypes]` is not that index.

Note also `0x004b34d7`: `CMP dword ptr [ESP+0x18], 0x7 / JGE` — the `TechnoType+0xC94` override that can force a result to `0` is only reachable for results **below** 7, so it cannot rescue this cell.

**Rust does not model the taxonomy at the runtime step.** `evaluate_runtime_can_enter_cell` (`src/sim/movement/movement_occupancy.rs:127..218`) returns a layer context and a bridge-traversal bool — no code. The terrain decision at `movement_step.rs:1142..1178` is a plain `bool`. A `CellEntryResult` enum with the full 0–7 mapping does exist (`src/sim/pathfinding/cell_entry.rs:47..80`), but `CellEntryResult::FriendlyWall` has **no producer anywhere in the tree** — it appears only in its own definition, one unit test (`cell_entry.rs:812`), and two match arms that treat it identically to `Impassable` (`movement_occupancy.rs:873`, `movement_tick.rs:834`). Wall overlays reach the mover as `overlay_blocks = true` (`src/map/resolved_terrain.rs:1531..1555`) → non-walkable `PathGrid` cell → hard block, with no ownership, no wall-damage-state, and no warhead consultation.

For **this** fixture Rust and native agree that `(57,50)` is rejected. The mechanism does not agree, and it would diverge immediately for an armed mover (a Grizzly against the same allied wall is native `4`, Rust hard-block).

---

## Stage 3 — Stop versus finishing the current cell transition

**Native never rolls back.** The check is issued at node adoption, with the body at rest at the current cell center after the previous track completed. On code 7 with the normal (first-pass) argument, `0x004b4519..0x004b4552` writes `-1` into the path queue head, stamps `techno+0x640 = g_CurrentFrameCounter`, zeroes `techno+0x648`, and recurses. Nothing about the body position changes. On the second pass the aim point `this+0x40/0x44/0x48` is reset to the null coordinate and `Set_Destination(NULL, true)` is called; still no positional edit.

**Rust rolls a partially-consumed cell transition backwards.** `process_cell_crossings` first tests whether the accumulated leptons have crossed the boundary (`movement_step.rs:1077..1089`, `sub_x >= LEPTONS_PER_CELL`), and only then evaluates passability. On the block it executes, at `movement_step.rs:1197..1202`:

```rust
position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
*drive_track_state = None;
target.movement_delay = 0;
```

`sub_x` has just reached or passed 256 and is written back to 128 — a rearward relocation of roughly half a cell inside the current cell, plus an unconditional re-centering of the perpendicular axis. The file's own comment at `movement_step.rs:1301..1305` documents that exactly this kind of forced re-centering produces "a visible position jump" and avoids it on the *success* path; the blocked path still does it. The MCV therefore drives up to the wall's edge and then slides backwards to the middle of `(56,50)`.

`*drive_track_state = None` additionally destroys the in-flight `DriveTrackState` (its `point_index`, `target_facing`, and residual) rather than letting the track complete. That is discarded sim state on a path every ground unit takes; it is a determinism concern because the amount of track discarded depends on where in the track the boundary test happened to fire.

---

## Stage 4 — Repath timing, queue disposal, and the deferred-repath mechanism

### The native sequence, in one call

Both `Process_Movement` callsites push `(out_ptr, 1, 0)` — `PUSH EBX(=0); PUSH 0x1; PUSH EAX; MOV ECX,EDI; CALL` at `0x004b0647` and `0x004b0a79` (`get_assembly_context 004b0647,004b0a79`). With `RET 0xc` and a `0x4C` frame plus four pops, the arguments sit at `[ESP+0x60]`, `[ESP+0x64]`, `[ESP+0x68]`; the code-7 branches test `[ESP+0x64]`, i.e. the literal `1`. So the **first** evaluation each tick always takes the "may repath" arm:

1. `0x004b4521` — `foot+0x5E0 = 0xFFFFFFFF`: the entire committed path queue is **discarded**, not spliced.
2. `0x004b452b..0x004b4548` — `techno+0x640 = g_CurrentFrameCounter`, `+0x644 = <cell>`, `+0x648 = 0`. The path-delay timer is deliberately zeroed so the retry cannot be throttled.
3. `0x004b4544..0x004b4552` — `PUSH 0; PUSH 0; PUSH <arg0>; MOV ECX,EBP; CALL 0x004b2630`: re-enter **with argument 1 set to 0**.
4. The re-entry sees an empty queue, falls into the no-path branch, passes the (now zero) delay gate, and calls `FootClass__Find_Path(dest_cell, 0, 0)`.
5. Before the call it stamps a fresh delay: `FLD double [RulesClass+0x1760]; FMUL [0x007e27f8]; CALL ftol` → `techno+0x648` (`disassemble_bytes 0x004b2843..0x004b288f`). With `PathDelay=.01` (`ini/rulesmd.ini:3106`) this is the 9-frame throttle. `BlockagePathDelay` is the separate `RulesClass+0x1768` timer used by the code-2 friendly-block branch.
6. On success the new queue head is adopted in the same call.

So: **detection, queue discard, Find_Path and new-node adoption all happen inside one tick, in one call.** Consecutive code-7 blocks are never throttled, because step 2 zeroes the timer that step 5 sets.

### Rust

`handle_blocked_tick` (`src/sim/movement/movement_blocked.rs:33`) is entered with `movement_delay` already forced to 0 by the caller (`movement_step.rs:1202`), takes `urgency = 2` because the code-7 caller passes `skip_grace_period = true` (`movement_step.rs:1226`), and calls `try_repath_after_block` in the same tick. `try_repath_after_block` (`src/sim/movement/movement_path.rs:385..499`) replaces `target.path` wholesale and sets `next_index = 1` — same "discard, do not splice" disposal — and deliberately does not set `movement_delay` on success (`movement_path.rs:487..489`). The new path begins consuming the next tick, and native likewise does not advance the body this tick (see Stage 5). Those rows agree.

### Is a deferred/retry repath a gamemd mechanism?

**No. It is VERA-internal, and commit `543ba0b9` moved it further from native, not closer.**

The Rust branch `543ba0b9` patched is `process_pending_drive_arrivals` (`src/sim/movement/movement_tick.rs:434`), reached when a drive mover's track ended away from its owner destination and `defer_drive_arrival_clear` armed `pending_arrival_clear` (`src/sim/movement/navcom.rs:181..188`, reached from `finalize_finished_entities` at `movement_tick.rs:1963..1979`). Its native counterpart is precisely the no-path process-entry branch above: destination live, path queue empty, delay gate passed, `Find_Path` issued.

Native's behaviour **when that `Find_Path` fails** is to give up on the destination, not to retry it:

- If the leaf predicate at owner vtable `+0x2CC` is false → `(**(vtbl + 0x480))(0, 1)` and return. Slot `+0x480` reads `0x00741970` from `0x007f60f0` (`read_memory 0x007f60ec`), which `get_function_by_address 0x00741970` resolves to `TechnoClass__Set_Destination`. Calling it with a null target **clears the destination**.
- Otherwise it falls into the CloseEnough / mission-2-or-0xB slice, and from there either `Set_Destination(NULL, true)` (no tether) or `FootClass__Stop_Moving()` + `(**(vtbl + 0x484))(0, 1)` (tethered; `0x007f60f4` → `0x00738970` = `UnitClass__OnArrival`), or scatters the cell and re-enters.

There is no branch anywhere in `Process_Movement` that re-arms a flag so the same failed repath is attempted again next tick with no state change. The one native retry that does exist — the code-7 recursion — is bounded to a **single** re-entry inside the same tick and is followed by a destination clear if the retry also fails.

`543ba0b9` adds three `entity.navigation.pending_arrival_clear = true;` re-arms (`movement_tick.rs:494`, `538`, `544`) on the no-locomotor, pathfinding-failure and short-path cases. That is an unbounded per-tick retry with no `PathDelay` stamp and no give-up rule — an invented gate of exactly the kind `ENGINE.md` forbids in `sim/`. The commit message already says so ("Treat as VERA-internal with the gamemd equivalent UNCHECKED until traced"); this trace resolves the UNCHECKED to **DRIFT**.

The commit did fix a real, silent, player-visible stall (a live destination with no path and no retry — the unit simply stops), so a straight revert would be worse. The native-shaped replacement is the branch above: on repath failure, evaluate the `+0x2CC` predicate, otherwise run the CloseEnough/scatter slice, and end with `set_destination_internal_null` — plus the `PathDelay` stamp before the `Find_Path` attempt so a persistent failure is throttled to once per 9 frames instead of once per tick.

### CloseEnough placement and metric

Native applies the CloseEnough abort **only after `Find_Path` has failed**, and compares a 3D Euclidean distance (`CoordStruct__Distance3D`) against `RulesClass+0x1718`. Rust applies it at `movement_blocked.rs:87..105` — **before the `movement_delay` gate and before any repath attempt** — and compares a Manhattan sum, `(|dx| + |dy|) * 256`, against 576. Any ground unit blocked within 2.25 cells of its destination therefore abandons the order in Rust without attempting to route around; native repaths first. The metric also diverges in both directions (a `(2,2)` offset is 1024 Manhattan versus 724 Euclidean). This fixture does not trip it — the block fires at `(56,50)`, four cells from `(60,50)` — but the same call site serves every blocked unit in the game.

---

## Stage 5 — Drive-track continuity and body facing

**This is the row the 2026-05-27 trace got directionally right and materially understated.**

Before native will consume a path node it enforces an **exact** facing match. From `disassemble_bytes 0x004b3418..0x004b34df`:

```text
004b341b  MOV   AX, word ptr [EAX]     ; current body facing (16-bit)
004b341e  MOV   ECX, EBX               ; EBX = next path direction octant
004b3420  SHL   ECX, 0xd               ; desired facing = octant << 13
004b3423  SUB   AX, CX
004b3426  MOVSX EAX, AX
...                                     ; ESI = |delta|, EAX = 0
004b3437  CMP   EAX, ESI
004b3439  JGE   0x004b345b             ; continue only when |delta| == 0
004b343b  ...
004b344c  CALL  dword ptr [ECX + 0x4c] ; FacingClass set-desired
004b3452  MOV   AL, 0x1
004b3458  RET   0xc                    ; return WITHOUT consuming the node
```

For this fixture the MCV is facing east (`0x4000`, octant 2 << 13) and the post-repath first step is north-east (octant 1 → `0x2000`). The delta is `0x2000` — 45°, non-zero — so native sets the desired facing, returns, and **the MCV does not move at all** until `ROT=5` has rotated the body through the full 45° and the gate reads zero. Only then is the node consumed and a track selected, and because facing already equals the new direction the track is a straight one. Native's turn here is a rotate-in-place with a full stop, not a curve.

Rust does the opposite. `try_repath_after_block` writes the new heading directly at `src/sim/movement/movement_path.rs:497`:

```rust
*facing = facing_from_delta(dx, dy);
```

There is no facing gate anywhere in `process_cell_crossings`, and `entity.facing_target` is not set. The body snaps 45° within the repath tick and the mover starts consuming the new path on the next tick. Because `entity.facing` was already overwritten, the subsequent `select_drive_track(cur_face, next_face, …)` sees `cur_face == next_face` and picks a straight track — so the turn curve native would have produced from the *prior* heading is suppressed as well.

Player-visible result on a turretless, `ROT=5`, `Speed=4`, `Size=6` vehicle: native shows *stop → visible pivot → resume*; Rust shows *instant body rotation with no pause*. `Crusher=yes` changes nothing here — this cell is code 7 (Stage 2).

---

## Stage 6 — Arrival

The repathed route still terminates at the requested cell: `try_repath_after_block` runs the goal through `resolve_requested_move_goal` and only rewrites `target.final_goal` when the goal itself is blocked (`movement_path.rs:420..440`); `(60,50)` is clear, so the requested exact center survives the detour.

The arrival clear itself is the `26ef9d2a` shape and matches the native contract on the rows re-read here. `finish_drive_navigation` (`src/sim/movement/navcom.rs:115..148`) compares `nav_com` against the current cell and, on a match for a live object, runs `finish_drive_arrival` in the same tick: `foot_stop_moving` (owner destination pair only), drive-runtime reset, and the queue advance gated on the current mission being `Move` (`navcom.rs:156..173`). Native's equivalent gate is visible at the head of `Process_Movement`: when the object is not moving and the path queue head is `-1`, it nulls the aim point, calls `GetCurrentMission` through owner vtable `+0x184`, returns immediately unless the result is `2`, and only then calls owner vtable `+0x484` (`0x00738970` = `UnitClass__OnArrival`). The dying-object skip (`navcom.rs:120..127`) mirrors the native liveness gate.

The residual already recorded in the source stands: `NavTargetRef::Cell` carries no layer, so the native height comparison in the arrival match is not modelled (`navcom.rs:128..131`). It cannot fire on this flat-ground fixture.

The literal arrival tick, the exact detour cell series, and the final facing were not executed and remain `UNCHECKED`.

---

## Stage verdict table

| # | Bounded row | Verdict |
|---|-------------|---------|
| 1 | Wall placement → passability visibility timing (`app_sim_tick.rs:1368`, `1382..1390` vs live `CellClass`) | FAIL |
| 2 | Primary detection trigger: lazy, immediate-next-cell only, no event, no queue revalidation | PASS |
| 3 | Second-cell overlay precheck that demotes a curve to a straight track (`LAB_004b3f7d` region) | NOT-IMPLEMENTED |
| 4 | Curve-chain second-cell recheck (`0x004b4120` ↔ `movement_tick.rs:1533..1596`) | PASS |
| 5 | Result for allied intact GAWALL vs weaponless AMCV: native `7`, Rust hard-block — this cell rejected by both | PASS |
| 6 | Allied/enemy split and crusher downgrade mechanism (`4`/`5`, overlay-index-0 gate) | NOT-IMPLEMENTED |
| 7 | Stop vs finish current cell: Rust re-centres `sub_x`/`sub_y` backwards on block | FAIL |
| 8 | In-flight drive-track teardown (`*drive_track_state = None`) leaving partial-track state | FAIL |
| 9 | Repath happens in the detection tick; no node consumed that tick | PASS |
| 10 | Old queue discarded wholesale, not spliced (`foot+0x5E0 = -1` ↔ `target.path` replace, `next_index = 1`) | PASS |
| 11 | `PathDelay` throttle behaviour on the code-7 path (zeroed before retry, restamped after) | PASS |
| 12 | `543ba0b9` deferred-repath retry vs native failure policy (`Set_Destination(NULL,true)`) | FAIL |
| 13 | CloseEnough abort placement (pre-repath vs post-Find_Path) and metric (Manhattan vs 3D Euclidean) | FAIL |
| 14 | Facing gate before node adoption — rotate in place at `ROT`, node not consumed | FAIL |
| 15 | New first track selected from pre-repath facing (Rust overwrites facing first) | FAIL |
| 16 | Literal detour side and cell series around `(57,50)` | UNCHECKED |
| 17 | Post-repath smoothing output vs native `Path_smooth_corners` / `Path_optimize_straight_segments` | UNCHECKED |
| 18 | Repathed route still arrives at the requested exact center `(60,50)` | PASS |
| 19 | Same-tick NavCom clear + Move-gated queue advance at arrival (`26ef9d2a`) | PASS |
| 20 | Literal per-tick lepton / facing / frame series for the whole run | UNCHECKED |

---

## Corrections to the prior trace

`MCV_WALL_MID_ROUTE_REPATH_TRACE.md` (2026-05-27) carried three claims that do not survive re-derivation:

1. **"Code-7 handler … call `vtable+0x480 (StopMission)(0,1)`"** — slot `+0x480` is `TechnoClass__Set_Destination @ 0x00741970`, not a stop-mission call, and on the first pass each tick that branch is **not taken at all**: the first pass discards the queue and repaths. The destination-clearing branch is the *second*-pass give-up.
2. **Stage 8, "Stop duration: 1 tick … net delta 0 ticks vs gamemd"** — native's stop is not one tick. The facing gate holds the MCV stationary for the entire `ROT=5` rotation through the heading change before it will consume the first new node. Rust's stop is one tick. The delta is the whole pivot.
3. **Stage 15's remedy, "should set `facing_target` … and let the drive-track selection handle the turn"** — the native mechanism is not a turn curve, it is a rotate-in-place with the node held un-consumed. Implementing a curve would still be a DRIFT.

Stage 3/4/17's PathGrid-lag DRIFT and Stage 12's smoothing UNCHECKED both stand.

---

## Top root findings

1. **No facing gate before node adoption (rows 14, 15).** Native refuses to consume the first post-repath node until the body has physically rotated to the new octant; Rust snaps the body and drives. On the AMCV — no turret, `ROT=5`, `Size=6` — this is the single most visible defect in the fixture. *Frequency:* every repath that changes heading, for every Drive mover. Ordinary skirmish hits this many times per minute (miners re-routing around parked units in the base, tank columns bumping, a wall going up in front of anything). An MCV specifically moves only a handful of times per match, but it is the case a player is most likely to be watching closely, and the fix is shared with every vehicle.
2. **`543ba0b9`'s retry is the inverse of native's failure policy (row 12).** Native clears the destination on a failed process-entry repath; Rust re-arms an unthrottled per-tick retry. It replaced a silent permanent stall with a silent permanent retry loop, and it is a VERA-invented gate in `sim/`. *Frequency:* fires whenever a drive mover's track ends away from its owner destination and the rebuild fails — most often a miner or MCV whose target cell got built over.
3. **Blocked-tick backward re-centering (rows 7, 8).** Rust advances leptons past the cell boundary, discovers the block, and writes both sub-axes back to the cell center while dropping the active drive track. *Frequency:* every hard block for every ground mover — the highest-frequency row in this report.
4. **CloseEnough abort runs before the repath and uses the wrong metric (row 13).** A unit blocked within 2.25 cells of its destination cancels its order in Rust without attempting to route around; native repaths first and only gives up if `Find_Path` fails. *Frequency:* common — short final approaches are exactly where blockage clusters.
5. **The 0–7 taxonomy is decorative (row 6).** `CellEntryResult::FriendlyWall` has no producer; wall ownership, wall damage state, and the mover's warhead `Wall=` flag are never consulted at runtime. This fixture happens to agree with native (`7` both sides) only because the AMCV is weaponless.

## Smallest decisive follow-up

One headless unit test that drives a `[AMCV]`-shaped Drive mover east from `(50,50)` toward `(60,50)` on a flat clear grid, flips `(57,50)` to non-walkable at the tick the mover reaches `(53,50)`, and asserts three things on the repath tick: (a) `entity.facing` is still `0x40` — the body has not rotated; (b) `position.sub_x` is unchanged rather than re-centred; (c) `target.next_index` is still pointing at the un-consumed first new node. All three currently fail, all three are single-assertion, and they pin rows 7, 14 and 15 — the three highest-frequency findings — without needing an oracle capture. The literal native pivot duration (`ROT=5` over 45°) is the one number that still needs an emulation or live capture before it can be asserted.
