# Drive Sharp-Turn Fallback Design

## Goal

When pathfinding produces a turn too sharp for any precomputed curve, drive
vehicles should mirror gamemd.exe: drive forward in current facing for one
cell, consume the impossible path step, and accept the route drift — instead
of the current stop-rotate-go behavior. Also fix the adjacent chain-caller
bug that uses `entity.facing` (mid-turn) instead of the active track's
post-turn `target_facing`.

## Architecture Context

Drive vehicles use a precomputed-curve system rooted at
[src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs):

- `TURN_TRACKS[72]` table indexed `from_dir * 8 + to_dir` (each dir is
  the facing quantized to 8 compass points). Each entry points to a
  `RawTrack` (1–15) of subpixel motion points and a `target_facing`.
- When pathfinding produces a turn ≥ 135° (same-hemisphere flip),
  `TURN_TRACKS[idx].normal_track == 0` — no smooth curve exists.
- Today, [`select_drive_track`](../../src/sim/movement/drive_track.rs#L3447)
  returns `None` for null tracks. Two callers handle that:
  - **Initial selection** at
    [movement_step.rs:90-101](../../src/sim/movement/movement_step.rs#L90):
    falls into `track_initiated = false`; for vehicles with `rot > 0`,
    line 108 sets `facing_target = Some(new_face)` →
    `handle_vehicle_rotation` does an in-place rotate before any forward
    motion. **Stop-rotate-go.**
  - **Chain attempt** at
    [movement_tick.rs:701-717](../../src/sim/movement/movement_tick.rs#L701):
    silently refuses; current track finishes, next tick re-enters
    `Process_Movement`-equivalent.

In gamemd.exe, the initial-selection site has a fallback (Process_Movement
at `0x4b402e`): substitute `track_index = cur_dir * 9` (the diagonal of
the 8×8 matrix → straight-ahead RawTrack 1 or 2 transformed for current
direction), shift the path queue left by one, and drive forward without
rotating. Chain has no fallback there — refusal matches current Rust.

The chain caller in Rust currently reads
[`entity.facing`](../../src/sim/movement/movement_tick.rs#L690) for the
chain "from-dir." The binary instead reads byte +0x04 of the **current**
TurnTrack entry (the active track's post-turn `target_facing`). During a
turn, `entity.facing` is mid-rotation, not the post-turn facing — so the
chain selection picks a different track than the binary.

## Impact Analysis

**Touched files:**

- `src/sim/movement/drive_track.rs` — add `build_sharp_turn_fallback`,
  add `target_facing: u8` field to `DriveTrackState`, populate in
  `begin_drive_track`.
- `src/sim/movement/movement_step.rs` — fallback branch in
  `configure_motion_after_transition`. May need a small
  `dir_to_cell_delta(facing) -> (i32, i32)` helper.
- `src/sim/movement/movement_tick.rs` — chain `cur_face` reads from
  `entity.drive_track.as_ref().map(|t| t.target_facing)` instead of
  `entity.facing`.
- `src/sim/movement/drive_track_tests.rs` — unit tests for
  `build_sharp_turn_fallback`.
- `src/sim/movement/movement_tests.rs` (or new
  `sharp_turn_fallback_tests.rs`) — integration tests.
- Possibly `src/util/fixed_math.rs` — host the new
  `dir_to_cell_delta` helper next to `facing_from_delta_int`.

**Depends on what we're changing:**

- Anything that serializes or hashes `DriveTrackState` will widen by one
  byte. The planned snapshot-serialization pass
  ([project_snapshot_serialization.md](../../../.claude/projects/<claude-project>/memory/project_snapshot_serialization.md))
  will pick this up automatically once derives roll out.
- Sim state-hash for vehicles with active drive tracks will change.
  Pre-existing replays of sharp-turn paths will need to be regenerated
  — flag in commit.

**Risk areas:**

- Forgetting to bump `next_index` would silently keep the impossible step
  in the queue and re-trigger the fallback every tick on the same step.
  Mitigation: integration test that asserts queue advances.
- Forgetting that `move_dir_*` is informational while a drive track is
  active — but stale values could leak into later code if a track exits
  early. Mitigation: set `move_dir_*` to substitute deltas in the
  fallback branch for symmetry with the surrounding code.
- The chain `target_facing` change is in the chain hot path — a
  regression here could subtly break ordinary curve chaining. Mitigation:
  integration test for mid-turn chain selection that compares against
  pre-change behavior on a non-sharp-turn path (should be unchanged where
  `entity.facing == target_facing`, which it usually is by the time a
  turn finishes).

## Chosen Approach

**Approach 2 (caller-side substitute helper).**

Keep `select_drive_track` as a pure table lookup that returns `None` on
null tracks. Add `build_sharp_turn_fallback` for synthesizing the
substitute. The initial-selection caller in
`configure_motion_after_transition` calls the fallback only when its
locomotor is `Drive` and `select_drive_track` returned `None`. The chain
caller continues calling `select_drive_track` directly, so its `None`
silently refuses — matching the binary.

Why over Approach 1 (substitute inside `select_drive_track` with an
`allow_fallback: bool`): the chain caller would inherit fallback by
default and silently grow a behavior the binary doesn't have if the flag
ever flipped. Approach 2 makes the chain site naturally correct.

Why over Approach 3 (`SelectionResult` enum return): the explicitness gain
is real but small relative to the churn of updating every caller; the
chain caller still has to translate one variant to another, which
is the same foot-gun.

## Tiny-Detail Ledger

Sourced from
[DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md).
Verified against binary directly: substitute site
disassembled at `0x4b3ff5..0x4b405b`; chain block decompiled in
`Process_Drive_Track` at `0x4b0f20`.

- Trigger: only `TURN_TRACKS[from_dir*8 + to_dir].normal_track == 0`.
  `use_short` is cleared to 0 immediately before this read at the
  binary site — irrelevant. `[GHIDRA 0x4b4019-0x4b4023]`
- Substitute TurnTrack index: `cur_dir * 9` (entries 0, 9, 18, 27, 36,
  45, 54, 63). Cardinal `cur_dir` → RawTrack 1 (23 pts); diagonal →
  RawTrack 2 (31 pts). `[doc: §5.2]`
- Substitute transform flags (low 3 bits) per cur_dir: `N=0, NE=0,
  E=3, SE=4, S=4, SW=1, W=1, NW=2`. Already implemented in
  `transform_track_point`. `[doc: §5.2]`
- Path queue shift: `path[1..24]` → `path[0..23]`, slot 23 sentinel =
  -1. Rust equivalent: bump `target.next_index` by one extra — the
  impossible step is permanently dropped. `[GHIDRA 0x4b4607]`
- `loco.dest` (ultimate destination) **not touched** — Rust has no
  per-tick `loco.dest` mirror; final dest lives in `target.path[last]`,
  which is unchanged. `[doc: §3.4]`
- `loco.head_to` ← NullCoord(0,0,0). Rust track motion is computed
  purely from current world pos + transformed track points; no
  `head_to` field exists. `[doc: §3.4]`
- `loco.point_index` ← 0. `begin_drive_track` already starts at
  `entry_index = 0` for RawTracks 1 and 2. `[doc: §3.4]`
- `is_reversed` / use_short ← 0. We never pass `use_short=true` from
  Rust. `[GHIDRA 0x4b4019]`
- `flags & 8` (cell-crossing validation bit) is 0 for **every**
  substitute entry → cell-beyond passability/cliff/occupancy check
  skipped this tick. Rust's `configure_motion_after_transition` doesn't
  pre-validate either; per-tick validation in `process_cell_crossings`
  on subsequent ticks. Match. `[doc: §3.1 table]`
- No Find_Path / repath call. Recovery is FootClass/AI-level. `[doc: §3.4]`
- Successive impossible steps: each next tick re-triggers the fallback,
  consuming another path step. The unit can drive forward repeatedly
  until path empties. `[doc: §5.1]`
- Crush/blocked override (binary path #1, sets `loco+0x64 = 1`) ALSO
  produces `cur_dir*9` but is a **distinct trigger** — explicitly out
  of scope here. `[doc: §2.3]`
- Chain branch: when chain's `select_drive_track` returns
  `normal_track == 0`, **no substitute** — current track finishes,
  control returns to Process_Movement next tick. Current Rust matches.
  `[doc: §4]`
- Chain `cur_face` in Rust reads `entity.facing` but binary uses the
  **current track's `target_facing`** (post-turn facing). Adjacent fix
  in scope. `[doc: §4.4]`

## Design

### Components

**`drive_track.rs`:**

```rust
/// Synthesize the substitute selection used when pathfinding requests a
/// turn too sharp for any precomputed curve. Returns the cur_dir*9 entry
/// — RawTrack 1 (cardinals) or 2 (diagonals), transform-rotated to the
/// unit's current facing. The unit drives forward in current_facing for
/// one cell; the caller consumes the impossible path step and skips
/// pre-validation of the cell-beyond.
pub fn build_sharp_turn_fallback(current_facing: u8) -> Option<DriveTrackSelection>;
```

`DriveTrackState` gains:

```rust
pub target_facing: u8,
```

Populated in `begin_drive_track` from the `DriveTrackSelection`
that started the track.

**`util/fixed_math.rs`** (or co-located helper):

```rust
/// Convert a 0-255 facing to its quantized 8-direction cell delta.
/// N → (0, -1), NE → (1, -1), E → (1, 0), ..., NW → (-1, -1).
pub fn dir_to_cell_delta(facing: u8) -> (i32, i32);
```

### Interfaces / Contracts

`select_drive_track` — unchanged signature, unchanged behavior. Pure
table lookup; returns `None` on null tracks or missing point data.

`build_sharp_turn_fallback(current_facing)` — pure function. Returns
`Some` for any quantized cur_dir whose corresponding RawTrack 1 or 2
has loaded point data (always, in normal operation). Returns `None`
defensively if track data isn't available.

`DriveTrackState.target_facing` — read-only after construction; equal
to the `target_facing` of the TurnTrack entry that produced this
track. Used by the chain caller to determine the post-turn
"from-dir" for chain-target track selection.

### Data Flow

**Initial selection** (`configure_motion_after_transition`,
`movement_step.rs:69`):

1. Compute `new_face` from `path[next_index]` direction.
2. If locomotor is `Drive` AND `new_face != *facing`:
   - Call `select_drive_track(*facing, new_face, false)`.
   - If `Some(sel)`: existing behavior — `begin_drive_track` with
     path-derived `(ndx, ndy)`.
   - If `None`: call `build_sharp_turn_fallback(*facing)`.
     - If `Some(fb)`:
       - `(cdx, cdy) = dir_to_cell_delta(*facing)`.
       - `begin_drive_track(fb.raw_track_index, fb.flags, cdx, cdy)`.
       - On success: `target.next_index += 1` (drop impossible step);
         set `move_dir_*` from `(cdx, cdy)`; `track_initiated = true`.
       - On failure (defensive): `track_initiated = false`.
     - If `None` (defensive, never expected): `track_initiated = false`.
3. Else (no direction change or non-Drive): existing behavior unchanged.
4. Existing branch at line 103 uses `track_initiated` to decide whether
   to set `facing_target`.

**Chain caller** (`movement_tick.rs:676`, `DriveTrackChainReady` arm):

1. `cur_face = entity.drive_track.as_ref().map(|t|
   t.target_facing).unwrap_or(entity.facing);`
2. Rest of the chain block (next-cell walkability check,
   `select_drive_track`, `begin_drive_track`) unchanged.
3. `select_drive_track` returning `None` continues to silently refuse
   — current track finishes, next tick re-enters initial-selection
   path which may then trigger the regular fallback.

### Error Handling

- `build_sharp_turn_fallback` returns `Option`; `None` falls through to
  the existing rotate-then-drive path. Defensive only.
- `begin_drive_track` already returns `Option`; existing handling
  preserved.
- No new panics, no `unwrap`, no expect-style asserts in the hot path.

### Testing Strategy

**Unit tests** (`drive_track_tests.rs`):

- `build_sharp_turn_fallback` for each of the 8 quantized cardinal /
  diagonal `current_facing` values: cardinals → RawTrack 1; diagonals
  → RawTrack 2; `flags` matches the per-direction transform table;
  `target_facing` matches the quantized `current_facing`.
- `build_sharp_turn_fallback` with non-quantized facings (e.g. 17,
  240): same selection as the nearest quantized direction (rounds via
  existing `facing_to_dir`).
- `dir_to_cell_delta` for all 8 directions returns the expected unit
  deltas.

**Integration tests**:

- 180° impossible turn: vehicle facing N, path step requests S.
  Expect: no `facing_target` set, no in-place rotation tick, drive
  track initiated, `next_index` advanced past the impossible step,
  unit ends one cell N of starting cell on the next cell crossing.
- 135° impossible turn: E → SW. Same shape.
- Successive fallbacks: 180° then 180° — both consumed across two
  cell crossings, no rotation, two cells drift in original facing.
- Chain `cur_face` correctness: vehicle starts a turn whose current
  track has `target_facing = NE` while mid-turn `entity.facing = N`.
  Path's next-next cell requires E from the post-turn facing. Verify
  the chain attempt picks the track for `NE → E`, not for `N → E`.
- Determinism: state-hash unchanged when no sharp turns occur in any
  unit's path; deterministic across runs when they do.

## Architectural Decisions

- **Pattern followed:** table lookups in `drive_track.rs`; per-tick
  policy in `movement_step.rs` / `movement_tick.rs`. Same layering as
  the existing code.
- **Pattern deviated:** none.
- **Tech debt introduced:** trivial — one byte added to
  `DriveTrackState`.
- **Determinism:** pure table-driven, no RNG, no float. Safe for
  lockstep. Snapshot/state-hash widens by one byte for active drive
  tracks; pre-existing replays of sharp-turn paths must be regenerated.

## Alternatives Considered

- **Approach 1 — substitute inside `select_drive_track`:** would
  require an `allow_fallback: bool` parameter so the chain caller can
  opt out. Rejected because the chain-caller default is a foot-gun: a
  silent fallback the binary doesn't have if the flag is forgotten.
- **Approach 3 — `SelectionResult` enum return:** more explicit at the
  type level but the chain caller still has to translate `Fallback →
  Refuse`, which is the same foot-gun renamed. Rejected for churn cost
  not justified by the small explicitness gain.

## Out of Scope (deferred follow-ups)

- **Crush/blocked override path** at `0x4b3ff9`: separate trigger that
  also produces `cur_dir*9` but sets `loco+0x64 = 1`. Distinct system,
  needs its own RE pass on what `loco+0x64` drives downstream.
- **`use_short` upstream decision:** RE doc §5.3 — when does the binary
  set `is_reversed = 1` to flip per-step lookups to `short_track`? Not
  load-bearing while Rust passes `use_short=false` everywhere.
- **`RawTrack[normal_track].chain_index != 0` second-clause refusal**
  at the chain branch: per RE doc §7, in practice never independently
  triggers (defensive code in the binary). Worth verifying if chain
  ever appears to refuse unexpectedly.
