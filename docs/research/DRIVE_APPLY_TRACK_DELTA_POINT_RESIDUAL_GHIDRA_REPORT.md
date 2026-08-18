# Drive Apply_Track_Delta Point Residual - Ghidra Research Report

**Address(es):** `0x004B0AD0` primary; callers/context `0x004B0C40`, `0x004B0F20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `DriveLocomotionClass::Apply_Track_Delta` endpoint/jump marking behavior, its `Force_Track` and `Process_Drive_Track` callers, and what this proves about point-index and residual ownership.
**Non-Scope:** general A* pathfinding, NavCom lifecycle, full speed ramp formulas, all collision case handling, and full `Force_Track` caller enumeration.
**Confidence:** High for `Apply_Track_Delta` local behavior and `Process_Drive_Track` residual ownership; Medium for exact object-field names because older docs mix object-base and ILocomotion-pointer offsets.
**Active in YR:** Yes for normal Drive locomotor stepping; Conditional for `Force_Track` caller use because the function is active but direct scenario reachability depends on caller context.

## Working Notes Gate

- Target question: Does `Apply_Track_Delta @ 0x004B0AD0` own point-index or residual consumption, or only apply/mark track-coordinate deltas?
- Non-goals: Do not investigate general pathfinding, NavCom, speed ramp formulas beyond residual-budget ownership, or unrelated `Can_Enter_Cell` cases.
- Evidence needed to mark COMPLETE: decompile plus assembly context for `0x004B0AD0`, caller evidence from `Force_Track` and `Process_Drive_Track`, Rust touchpoint scan, and a handoff item.
- Stop conditions: stop once the primary function and the two relevant caller shapes are resolved, all open questions are resolved/deferred, and no Rust/code edits are made.

## 1. Overview

`Apply_Track_Delta` is not the per-tick drive-track stepper. It is a marking helper: when a Drive locomotor has an active normal track and the current point index is before the raw track's `jump_index`, it marks/removes an extra transformed track point, then always applies the supplied coordinate with the requested mark mode.

The per-tick point budget is owned by `Process_Drive_Track @ 0x004B0F20`: it adds current speed only on the normal call, consumes one point per 7 budget units, increments `point_index`, stores the leftover at Drive object `+0x4C`, and uses that residual only for interpolation. Active in YR: Yes; DriveLocomotion `Process @ 0x004B0500` reaches `Process_Drive_Track` for standard Drive units, and prior trace docs confirm standard YR ground vehicles use this path.

## 2. Class Layout / Key Offsets

Offsets below are relative to the DriveLocomotion object base unless noted. `Force_Track` is dispatched through the ILocomotion pointer (`object_base + 4`), so its decompile offsets are often 4 bytes lower than `Process_Drive_Track`/`Apply_Track_Delta`.

| Offset | Field | Verified use | Active in YR |
|---:|---|---|---|
| `+0x0C` | owner `FootClass*` | `Apply_Track_Delta` calls owner vtable `+0x1D0`, `+0xF0`, `+0xF4` through this pointer | Yes |
| `+0x40..+0x48` | `head_to` coord | `Process_Drive_Track` and `Apply_Track_Delta` transform track-local points relative to the active head coordinate | Yes |
| `+0x4C` | residual movement budget | `Process_Drive_Track` clears it on guard failure, reads it into budget, writes leftover after the loop | Yes |
| `+0x50` | double current speed fraction/value | `Process_Drive_Track` passes it to owner `SetSpeed`; `Force_Track` writes double `1.0` through ILocomotion offset `+0x4C/+0x50` | Yes |
| `+0x58` | TurnTrack index | `Apply_Track_Delta` reads it to choose TurnTrack normal raw-track byte; `Process_Drive_Track` owns normal stepping | Yes |
| `+0x5C` | point index | `Apply_Track_Delta` only compares it to raw track `jump_index`; `Process_Drive_Track` increments/resets it | Yes |
| `+0x60` | short/reversed selector | `Apply_Track_Delta` requires this byte to be zero before applying the extra transformed point | Yes |
| `+0x63` | active/head-to flag | `Process_Drive_Track` and `Force_Track` set/clear it; `Apply_Track_Delta` does not write it | Yes |

## 3. Core Logic

### 3.1 `Apply_Track_Delta @ 0x004B0AD0`

Verified decompile and assembly context: function starts at `0x004B0AD0`; null-coordinate checks at `0x004B0ADF..0x004B0AFE`; TurnTrack normal raw byte loaded at `0x004B0B22`; raw track pointer/jump index read at `0x004B0B35..0x004B0B41`; point-index compare at `0x004B0B57..0x004B0B5C`; transform call at `0x004B0B7E..0x004B0B80`; mark/remove calls at `0x004B0BAF`, `0x004B0BBF`, `0x004B0BFA`, `0x004B0C0E`, `0x004B0C2E`.

Pseudocode, preserving branch shape:

```text
Apply_Track_Delta(object_base, coord*, mode):
  if coord == NullCoord:
      return

  if use_short_track == 0
     and track_index != -1
     and TurnTrack[track_index].normal_track != 0:
      raw = TurnTrack[track_index].normal_track
      jump = RawTrack[raw].jump_index       // table +0x0C
      if jump > -1 and point_index < jump:
          point = RawTrack[raw].points[jump]
          transformed = Transform_Track_Coords(point)
          owner.GetBodyFacing()
          if mode == 0:
              owner.MarkRemove(transformed)
              owner.MarkRemove(coord)
              return
          if mode == 1 or mode == 3:
              owner.MarkPut(transformed)

  if mode == 0:
      owner.MarkRemove(coord)
  else if mode == 1 or mode == 3:
      owner.MarkPut(coord)
```

Material details:

- Active in YR: Yes. Evidence: `Force_Track @ 0x004B0C40` and `Process_Drive_Track @ 0x004B0F20` call this helper on DriveLocomotion paths; standard Drive units execute `Process_Drive_Track`.
- Null coordinate is a full early return. It does not clear state, residual, track validity, or point index.
- The extra transformed-point path is gated by `use_short_track == 0`, `track_index != -1`, `normal_track != 0`, `jump_index > -1`, and `point_index < jump_index`.
- `Apply_Track_Delta` does not write `point_index`, `track_index`, `residual`, `head_to`, `destination`, or `is_on_track`.
- Mode `0` removes both the transformed jump coordinate and the supplied coordinate, then returns early. Modes `1` and `3` put the transformed coordinate if gated, then put the supplied coordinate. Other modes do nothing except the null/gate checks.

### 3.2 `Force_Track @ 0x004B0C40` caller shape

Verified decompile and assembly context: entry writes the supplied track to ILocomotion-relative `+0x54` and point index to `+0x58` at `0x004B0C53..0x004B0C56`, equivalent to object-base `+0x58/+0x5C`. On accepted target it calls `Apply_Track_Delta` at `0x004B0D3A`, writes destination, resets speed/residual-related words at `0x004B0D52..0x004B0D59`, and returns at `0x004B0D67`.

Active in YR: Conditional. The vtable slot and function are active, and `BuildingClass::ReleaseDockedHarvester` has a verified conditional path that calls slot `+0x70`; stock CMIN/HARV refinery unload does not always take that path according to `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`.

Important offset-base fact:

- `Force_Track` receives the ILocomotion pointer, so `param+0x54` is object-base `+0x58` (`track_index`) and `param+0x58` is object-base `+0x5C` (`point_index`).
- It passes `this - 4` (`object_base`) into `Apply_Track_Delta`, so `Apply_Track_Delta` uses object-base offsets directly.

### 3.3 `Process_Drive_Track @ 0x004B0F20` residual and point stepping

Verified decompile and assembly context: guard clears residual at function start; speed is read through owner vtable `+0x538`; budget is `((retry ? 0 : speed) + residual)`; each consumed point costs `7`; leftover is written to object-base `+0x4C`; interpolation scales by `residual * 1/7`.

Core facts:

- Active in YR: Yes. Standard DriveLocomotion `Process` calls this function when a track is active; prior traces (`MCV_DRIVE_10_CELLS_STRAIGHT_FLAT_GRASS_TRACE.md`, `DRIVE_TRACK_LOOKAHEAD_RUNTIME_TUPLE_TRACE.md`) identify it as the live ground vehicle runtime path.
- Retry/same-tick call does not add speed again. Nonzero `param_2` masks speed contribution to zero and uses only stored residual.
- The loop condition is `while budget > 7`, not `>= 7`; the function subtracts 7 before processing the point body, then increments `point_index` at the end of the loop body.
- Residual interpolation after the loop does not increment `point_index` and does not call the facing-update site.
- Chain-success branch sets the new `track_index`, sets `point_index = RawTrack[next_raw].entry_index - 1`, temporarily clears/reinstalls `head_to`, then calls `Apply_Track_Delta(next_cell, 1)` after the target cell is accepted.

## 4. INI Keys

No INI key is read by `Apply_Track_Delta`. Drive speed and acceleration keys feed `Process_Drive_Track` through TechnoType/current-speed state, but full speed-ramp decoding is out of scope for this slot.

| Key / source | Default relevance | Effect in this slice | Active in YR |
|---|---|---|---|
| Unit `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` | Drive locomotor assignment | Reaches DriveLocomotion `Process`/`Process_Drive_Track` for normal ground vehicles | Yes |
| Unit `Speed` / acceleration fields | Not read by `Apply_Track_Delta` | Indirectly affect budget before `Apply_Track_Delta` callsites; exact formulas deferred | Yes for Drive movement, deferred for this helper |

## 5. Integration Points

| Caller / callee | Evidence | Role | Active in YR |
|---|---|---|---|
| `Force_Track @ 0x004B0C40 -> Apply_Track_Delta` | decompile plus assembly `0x004B0D31..0x004B0D3A` | initial forced-track mark/placement after accepted forced target | Conditional |
| `Process_Drive_Track @ 0x004B0F20 -> Apply_Track_Delta` | decompile chain-success branch | marks accepted follow-on chain cell after Can_Enter_Cell success | Yes |
| `Transform_Track_Coords @ 0x004B4780` | decompile call at `0x004B0B7E..0x004B0B80` | transforms raw `jump_index` point before extra mark/remove | Yes |
| owner vtable `+0x1D0` | assembly `0x004B0B93`, `0x004B0BDE` | body-facing/read side-effect before mark calls | Yes |
| owner vtable `+0xF0` / `+0xF4` | assembly `0x004B0BFA`, `0x004B0C2E`, `0x004B0BAF`, `0x004B0BBF`, `0x004B0C0E` | put/remove occupation bits for transformed point and supplied coordinate | Yes |

## 6. Current Rust Implementation Status

Rust currently has the data structures needed to model both a standalone `DriveTrackState` and DriveLocomotion-owned state:

- `src/sim/movement/drive_track.rs` has `DriveTrackState { raw_track_index, point_index, residual, transform_flags, ... }`, `begin_drive_track`, `begin_drive_track_with_head_offset`, `advance_drive_track`, and `interp_sub_step`.
- `src/sim/components.rs` has `DriveLocomotionRuntime { track_index, point_index, track_valid, is_reversed, current_speed_fraction, residual_budget, ... }`, but `residual_budget` is not the active budget used by `advance_drive_track`.
- `src/sim/movement/movement_step.rs` calls `advance_drive_track(track_state, effective_speed, dt)` and stores residual inside `DriveTrackState`, not `DriveLocomotionRuntime`.
- `src/sim/movement/movement_tick.rs` chain handling creates a fresh track with `begin_drive_track(...)`; it does not set the chained point index to `RawTrack.entry_index - 1` after a binary-style chain success, and it does not call an `Apply_Track_Delta`-equivalent mark helper for the accepted next cell.

Current Rust is close on "7-budget residual per active track" but the ownership differs: gamemd stores residual and point index on the DriveLocomotion object, while Rust stores them primarily in `entity.drive_track`. That matters if destination/head-to/track state is cleared or chained independently of `MovementTarget`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Apply_Track_Delta @ 0x004B0AD0` null coord branch | verified | decompile; assembly `0x004B0ADF..0x004B0AFE` | none |
| `Apply_Track_Delta` extra transformed jump mark/remove | verified | decompile; assembly `0x004B0B22..0x004B0B80`, `0x004B0BAF..0x004B0BFA` | exact owner vtable method names can be audited separately |
| `Apply_Track_Delta` point/residual non-ownership | verified | no writes to `+0x4C/+0x58/+0x5C` in decompile; assembly range `0x004B0AD0..0x004B0C3B` | none |
| `Force_Track` object-base offset correction | verified | decompile; assembly `0x004B0C53..0x004B0C56`, `0x004B0D3A` | full caller enumeration deferred |
| `Process_Drive_Track` residual budget ownership | verified | decompile `0x004B0F20`; prior report `PROCESS_DRIVE_TRACK_DECOMPILATION.md` lines 193-200, 763-819 | exact speed ramp constants deferred to speed slot |
| Rust `DriveTrackState` residual | verified | `src/sim/movement/drive_track.rs` search output | implementation audit/fix remains |
| Rust `DriveLocomotionRuntime.residual_budget` | verified | `src/sim/components.rs` search output | currently not active owner for stepping |
| Full `Can_Enter_Cell` switch cases | deferred | out of scope | separate collision/repath investigation |
| Full `Force_Track` caller set | deferred | vtable dispatch and conditional reports | separate caller-scan investigation if a symptom requires it |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `Apply_Track_Delta` the per-tick point/budget stepper? -> No; it marks/removes a transformed jump point plus supplied coord and does not consume budget or increment point index.` (evidence: `0x004B0AD0` decompile; assembly `0x004B0B57..0x004B0C2E`)
- `[RESOLVED] OQ-2 - What gates the extra transformed point? -> non-null coord, `use_short_track == 0`, `track_index != -1`, nonzero normal raw track, `jump_index > -1`, and `point_index < jump_index`.` (evidence: `0x004B0B04..0x004B0B5C`)
- `[RESOLVED] OQ-3 - Which modes mark/remove? -> mode 0 calls owner `+0xF4`; modes 1 and 3 call owner `+0xF0`; other modes do not mark after the gate.` (evidence: `0x004B0BAF`, `0x004B0BBF`, `0x004B0BFA`, `0x004B0C0E`, `0x004B0C2E`)
- `[RESOLVED] OQ-4 - Does null coordinate clear state? -> No, it returns before state writes.` (evidence: `0x004B0ADF..0x004B0AFE`)
- `[RESOLVED] OQ-5 - Where is residual stored? -> object-base `+0x4C` in `Process_Drive_Track`, not inside `Apply_Track_Delta`.` (evidence: `0x004B0F20` decompile)
- `[RESOLVED] OQ-6 - Does residual interpolation update point_index? -> No, it runs after residual storage and returns without the loop's point-index increment or facing-update site.` (evidence: `0x004B0F20` residual branch; `PROCESS_DRIVE_TRACK_DECOMPILATION.md` lines 763-819)
- `[RESOLVED] OQ-7 - Does same-tick retry add speed again? -> No, retry masks speed contribution to zero and uses residual only.` (evidence: `0x004B0F20` decompile; `DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-8 - Why do docs disagree on offsets? -> `Force_Track` receives ILocomotion pointer and calls `Apply_Track_Delta` with `this - 4`; some offsets are ILocomotion-relative, some object-base.` (evidence: `0x004B0C53..0x004B0C56`, `0x004B0D31..0x004B0D3A`)
- `[RESOLVED] OQ-9 - Is `Apply_Track_Delta` active in YR? -> Yes for DriveLocomotion paths; called from live Process_Drive_Track chain branch and from Force_Track when a forced caller reaches it.` (evidence: `0x004B0F20`, `0x004B0C40`)
- `[RESOLVED] OQ-10 - Does Rust store residual in the same owner? -> No; active stepping residual is in `DriveTrackState::residual`, while `DriveLocomotionRuntime::residual_budget` exists but is not consumed by `advance_drive_track`.` (evidence: `src/sim/movement/drive_track.rs`, `src/sim/components.rs`)
- `[RESOLVED] OQ-11 - Does Rust chain use binary `point_index = entry_index - 1`? -> Not in the scanned chain path; it begins a fresh track through `begin_drive_track`, which initializes to `entry_index`.` (evidence: `src/sim/movement/movement_tick.rs`, `src/sim/movement/drive_track.rs`)
- `[DEFERRED] OQ-12 - Exact speed ramp formulas feeding the budget` (category: `out-of-scope`; reason: separate speed-fraction swarm slot; next-step-if-pursued: investigate `Process_Drive_Track` speed setup before the budget line)
- `[DEFERRED] OQ-13 - Full `Force_Track` caller enumeration` (category: `bounded-cost-too-high`; reason: vtable slot dispatch has many indirect call shapes and is not needed to answer residual ownership; next-step-if-pursued: targeted vtable receiver scan for slot `+0x70`)
- `[DEFERRED] OQ-14 - Full collision/repath switch cases after chain lookahead` (category: `out-of-scope`; reason: this slot only needed accepted chain behavior and `Apply_Track_Delta`; next-step-if-pursued: separate `Can_Enter_Cell` switch audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Residual budget and point index are DriveLocomotion-owned fields; `Apply_Track_Delta` does not own or mutate them | `0x004B0F20` decompile; `0x004B0AD0` decompile/assembly range `0x004B0AD0..0x004B0C3B` | mismatch/partial: active residual is `DriveTrackState::residual`; `DriveLocomotionRuntime::residual_budget` exists but is unused | `src/sim/components.rs`, `src/sim/movement/drive_track.rs`, `src/sim/movement/movement_step.rs` | Drive track stepping should read/write `DriveLocomotionRuntime.residual_budget` and `point_index` as the canonical gamemd-owned state, with any `DriveTrackState` reduced to table/transform metadata or synchronized mirror | A Drive vehicle with residual 6 and speed budget 2 consumes one point next tick (`6 + 2 > 7`) and stores residual 1 on Drive runtime, not on a detached track state | `drive_runtime_residual_carries_across_track_ticks` | Do not keep two unsynchronized residual owners |
| Chain success sets the new track point index to `RawTrack[next_raw].entry_index - 1`, then `Apply_Track_Delta(next_cell, 1)` marks the accepted next cell | `0x004B1B3C` chain-key compare; `0x004B1B78..0x004B1B99` next track table checks; `0x004B0AD0` helper; `PROCESS_DRIVE_TRACK_DECOMPILATION.md` lines 649-715 | mismatch: Rust chain begins a new track via `begin_drive_track`, initializing to `entry_index`; no explicit `Apply_Track_Delta`-equivalent mark side-effect | `src/sim/movement/movement_tick.rs::handle_deferred_drive_track_chain`, `src/sim/movement/drive_track.rs` | Accepted chain should start at the pre-consume tail point (`entry_index - 1`) and then continue with the same tick budget semantics; occupancy/position side effect must match `Apply_Track_Delta` mark mode 1 | A curve reaching chain index with accepted follow-on track starts the new track at `entry_index - 1`, so the next consumed point is the binary tail point rather than skipping it | `drive_track_chain_success_starts_at_entry_minus_one` | Do not call `begin_drive_track` unchanged for chain-success state |
| `Apply_Track_Delta` extra transformed-point mark is conditional and mark-only; it does not advance visual progress | `0x004B0B04..0x004B0C2E` | missing/unchecked: Rust does coordinate/cell reservation but has no narrow helper mirroring the transformed jump-point mark/remove branch | `src/sim/movement/movement_occupancy.rs`, `src/sim/movement/movement_tick.rs`, possible `drive_track.rs` pure helper | Add a simulation-side equivalent only where occupancy/marking effects matter; keep it separate from point-budget stepping | For a cell-crossing raw track with `point_index < jump_index`, forced/chain mark mode 1 reserves/marks both the transformed jump coordinate and supplied target coordinate; for `point_index >= jump_index`, only supplied target is marked | `apply_track_delta_marks_jump_coord_before_jump_index` | Do not fold this into budget stepping or use it as a reason to mutate `point_index` |

### Negative Facts / Do Not Do

- Do not move per-point residual ownership into `Apply_Track_Delta`; it has no residual read/write and no point-index increment. Evidence: `0x004B0AD0` decompile and assembly `0x004B0AD0..0x004B0C3B`.
- Do not treat mode `0` and mode `1` as symmetric; mode `0` removes and returns after the transformed+supplied remove path, while mode `1/3` puts transformed and then falls through to put supplied. Evidence: `0x004B0BAF..0x004B0C2E`.
- Do not use raw track `jump_index` as a point count. `Apply_Track_Delta` reads it as a conditional endpoint for extra marking, and `Process_Drive_Track` separately walks point arrays/sentinels. Evidence: `0x004B0B35..0x004B0B5C`; `DRIVE_TRACK_TABLES_DEEP_DECODE.md`.
- Do not initialize accepted chain tracks at `entry_index` if matching the binary chain branch; it writes `entry_index - 1` before continuing. Evidence: `PROCESS_DRIVE_TRACK_DECOMPILATION.md` lines 649-715 and `0x004B1B78..0x004B1B99`.
- Do not compare offsets from `Force_Track` and `Process_Drive_Track` without normalizing the `this` base. Evidence: `Force_Track` writes ILocomotion-relative `+0x54/+0x58` at `0x004B0C53..0x004B0C56`, then calls `Apply_Track_Delta` with object-base pointer at `0x004B0D31..0x004B0D3A`.

### Stale Docs / Follow-up Docs

- `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` lines near the object layout say `+0x4C` is `current_speed` and later say the stepping residual is "likely within Process_Drive_Track's locals." Replacement wording:
  - **Replacement:** "Normalize offsets before reading this table: `Process_Drive_Track` and `Apply_Track_Delta` use the Drive object base, where `+0x4C` is the integer residual movement budget and `+0x50` is the double current speed. `Force_Track` is dispatched through the ILocomotion pointer (`object_base + 4`), so its `param+0x4C/+0x50` writes are object-base `+0x50/+0x54` halves of the double speed. The residual is not a local-only value; it is read/written in `Process_Drive_Track @ 0x004B0F20`."
- `docs/research/PROCESS_DRIVE_TRACK_DECOMPILATION.md` says the residual gate comment "`budget > 3` use full step coords instead"; the decompile shows the condition selects interpolated coords when `interp_cell` is current/full OR `budget > 3`, otherwise falls back to full. Replacement wording:
  - **Replacement:** "`budget > 3` is a trust window for using the interpolated coordinate even when cell classification is neither saved nor full; fallback to full-step coords happens when the interpolated cell is neither current nor full and `budget <= 3`."

## Sources

- Ghidra decompile: `DriveLocomotionClass__Apply_Track_Delta @ 0x004B0AD0`
- Ghidra assembly context: `0x004B0AD0`, `0x004B0B22`, `0x004B0B57`, `0x004B0B7E`, `0x004B0BAF`, `0x004B0BFA`, `0x004B0C0E`, `0x004B0C2E`
- Ghidra decompile: `DriveLocomotionClass__Force_Track @ 0x004B0C40`
- Ghidra assembly context: `0x004B0C53`, `0x004B0C56`, `0x004B0D3A`, `0x004B0D52`
- Ghidra decompile: `DriveLocomotionClass__Process_Drive_Track @ 0x004B0F20`
- Existing docs: `docs/research/PROCESS_DRIVE_TRACK_DECOMPILATION.md`, `docs/research/DRIVE_TRACK_TABLES_DEEP_DECODE.md`, `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`, `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`, `docs/research/CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
- Rust scan: `src/sim/movement/drive_track.rs`, `src/sim/components.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_tick.rs`

