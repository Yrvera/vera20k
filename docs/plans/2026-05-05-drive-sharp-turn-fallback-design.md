# Drive Sharp-Turn Fallback Design

## Goal

When pathfinding produces a turn too sharp for any TurnTrack curve, drive the
vehicle one cell straight ahead in its current facing instead of stopping to
rotate in place — matching gamemd.exe `Process_Movement`'s `track_index =
facing * 9` substitute.

## Architecture Context

The drive-track system runs at every cell crossing for vehicles using the
Drive locomotor. Two callers consult `drive_track::select_drive_track` to
pick a curve:

- **Initial track init** at
  [movement_step.rs:91](../../src/sim/movement/movement_step.rs#L91), called
  from `configure_motion_after_transition` after a cell crossing. Fires when
  the new path step requires a facing change.
- **Mid-track chain attempt** at
  [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701),
  called when a unit's current track reaches its `chain_index` and a
  follow-on curve is needed for the next cell.

`select_drive_track` indexes the 8×8 `TURN_TRACKS` matrix as
`from_dir * 8 + to_dir` and returns the entry's `RawTrack` reference. When
the entry's `normal_track == 0` (the sentinel for "turn too sharp for a
smooth curve"), `select_drive_track` returns `None`. Both callers currently
react to `None`:

- Initial init falls through to in-place rotation
  (`facing_target = Some(new_face)`) — stop-rotate-go.
- Chain attempt skips the chain (current track finishes out).

The binary's behavior at `0x4b4023` differs only at the initial-init site:
when the looked-up `normal_track == 0`, it substitutes `track_index = facing
* 9` (the diagonal of the 8×8 matrix → straight-ahead in current facing),
shifts the path queue, and drives the unit one cell forward. The chain
branch in Process_Drive_Track has no such substitute and just refuses, which
already matches our chain caller. See
[DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md)
for the full assembly walk.

## Impact Analysis

**Files touched:**
- [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs) —
  add `fallback_drive_track` and (if not present) `direction_to_cell_delta`.
- [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) —
  modify `configure_motion_after_transition`, replace the `None` arm with
  the fallback path.
- [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) —
  add unit tests for the new helper.
- One integration-test site (existing test file or a new one) for end-to-end
  fallback behavior.

**Files NOT touched:**
- `select_drive_track` itself — contract preserved.
- [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701)
  (chain caller) — current `None` handling already matches Q3.
- Path representation / pathfinding — no changes to `MovementTarget.path`
  or `path_grid`.
- Speed ramp / arrival detection — `final_goal` and `loco.dest`-equivalent
  state stay valid through fallback steps.

**Determinism risk:** none. Substitute is a pure function of facing.
**Replay risk:** state hash will differ from pre-fix runs (vehicles now move
where they previously stopped). Replay snapshots taken before this lands
won't replay correctly after — expected.

**Hot path:** `configure_motion_after_transition` runs once per vehicle per
cell crossing. Adds at most one TURN_TRACKS lookup + one
direction_to_cell_delta lookup on the fallback branch. Effectively free.

## Chosen Approach

**Approach B from brainstorm: caller-side fallback at movement_step.rs.**

`select_drive_track` keeps its contract (`Option<DriveTrackSelection>`).
The caller at the initial-init site detects `None`, calls a new tiny helper
`fallback_drive_track(current_facing)` to look up the substitute entry, and
calls `begin_drive_track` with the substitute's `raw_track_index` + `flags`
+ current_dir's cell delta. The chain caller is unchanged.

This mirrors the binary's structure (Process_Movement substitutes;
Process_Drive_Track does not) and produces the smallest diff. Two
alternatives were considered and rejected:

- **Approach A** (substitute inside `select_drive_track` with a
  `TrackContext` enum) — adds a context param solely to gate behavior at
  one decision point. New pattern for no reason.
- **Approach C** (separate `fallback_drive_track` + caller-side wiring,
  same as B but more pieces) — dominated by B; B already extracts the
  helper but keeps the caller logic minimal.

## Tiny-Detail Ledger

Constraint set carried through to `/write-plan` and implementation. Each
item must have a home in the implementation; if any is dropped, that's a
parity drift, not a tradeoff.

| # | Detail | Source | Where it lives in the design |
|---|---|---|---|
| 1 | Substitute formula: `turn_index = quantized_facing * 9` (= `from_dir*8 + from_dir`) | `[GHIDRA 0x4b402e]` | Inside `fallback_drive_track`. |
| 2 | Quantization is `facing_to_dir(loco.facing)` — same +16 rounding as elsewhere | `[GHIDRA EBX preload at 0x4b3ff5..]` | Reuses existing `facing_to_dir` in drive_track.rs. |
| 3 | Trigger reads byte +0x00 (`normal_track`). Process_Movement never reads `short_track` at this site; `loco.is_reversed` is force-cleared at `0x4b4019` | `[GHIDRA 0x4b4019..0x4b4023]` | Caller calls `select_drive_track(_, _, false)` — the `use_short=false` already matches the binary's clear. |
| 4 | After fallback, binary sets `loco.head_to = NullCoord (0,0,0)`; track motion computed from points relative to current world position | `[GHIDRA 0x4b4685+]` | Rust's `head_offset` is cell-relative, not world-coord. The Rust analog is "head_offset = current_dir's cell delta" so the substitute track's transformed points land on the cur_dir cell. Caller passes `(cur_dx, cur_dy)` to `begin_drive_track`. |
| 5 | Substitute entry's `target_facing` equals current_dir → no rotation | `[binary table read 0x7e7b28]` | Caller sets `facing_target = None` after track_initiated (existing branch at line 103-104 handles this). |
| 6 | All 8 substitute entries have `flags & 8 == 0` → no cell-crossing validation | `[binary table read]` | Implicit — Rust doesn't do beyond-cell validation in this code path anyway. |
| 7 | Path queue shifted left by 1 (impossible step permanently consumed). No Find_Path call. `loco.dest`/`final_goal` not touched | `[GHIDRA 0x4b4607, 0x4b4612]` | Already handled — `target.next_index += 1` at line 80 runs *before* select_drive_track is called. `final_goal` is on `MovementTarget` and not modified anywhere in this code path. |
| 8 | Determinism: pure function of facing | derives from item 1 | No RNG, no clock, no global state read. |
| 9 | Chain-time fallback: NONE. Chain refusal is final; current track finishes, next tick re-enters Process_Movement | `[GHIDRA Process_Drive_Track chain branch]` | Chain caller at movement_tick.rs:701 unchanged. Existing `if let Some(sel) = ...` already does the right thing on None. |
| 10 | `loco.point_index` reset to 0 (not `entry_index`). Substitute tracks (1, 2) have `entry_index == 0` so this is a no-op | `[GHIDRA 0x4b4659]` | `begin_drive_track` initializes `point_index` to whatever the existing convention is. Tracks 1 and 2 both have `entry_index = 0`, so the binary's "= 0" and Rust's "= entry_index" agree by accident. **Confirm during implementation.** |
| 11 | Successive fallbacks compound: if path[next] AND path[next+1] are both unreachable from current_dir, the unit drives 2 cells off-route. Recovery is at higher layers | derives from item 7 | Tested explicitly in integration test #4. No design accommodation needed; the natural per-tick re-entry gives the same compounding. |
| 12 | Rust's `select_drive_track(use_short: bool)` parameter has no analog in Process_Movement at the substitute site | `[GHIDRA Q1]` | Out of scope. Caller passes `false`, which matches the binary's clear. Whether `use_short=true` paths in Rust were ever correctly hooked up is a *separate* investigation. |

Item 4 deserves emphasis: the binary's "head_to = NullCoord" is **not**
equivalent to "head_offset = (0, 0)" in Rust. The Rust value is a
cell-relative offset; the binary's is a world-coord destination sentinel.
The semantically equivalent Rust value is "head_offset for the cell-cur_dir
takes us to" — i.e., the cur_dir cell delta encoded the same way the path
delta is encoded. This is the load-bearing translation and the test in
strategy item #3 confirms it.

## Design

### Components

**1. `drive_track::fallback_drive_track(current_facing: u8) -> DriveTrackSelection`**

Pure function. Quantizes facing to direction index 0-7 (via existing
`facing_to_dir`), looks up `TURN_TRACKS[from_dir * 9]`, packages it into a
`DriveTrackSelection` and returns. Infallible — all 8 substitute entries are
non-null in the binary table (entries 0, 9, 18, 27, 36, 45, 54, 63 with
`normal_track ∈ {1, 2}`).

**2. `drive_track::direction_to_cell_delta(dir: usize) -> (i32, i32)`** (if
not already present)

Returns `(dx, dy)` for direction index 0-7 using the standard map convention
(N = (0, -1), NE = (1, -1), E = (1, 0), …, NW = (-1, -1)). Implementation
checks first whether an existing helper covers this — likely something near
`cell_delta_to_lepton_dir` in [util/lepton.rs](../../src/util/lepton.rs).
If absent, add a small `const DIRECTION_DELTAS: [(i32, i32); 8] = [...]`.

**3. Modified `configure_motion_after_transition`**

The current `None` arm:

```rust
let track_initiated = if uses_drive_tracks && new_face != *facing {
    if let Some(sel) = drive_track::select_drive_track(*facing, new_face, false) {
        *drive_track =
            drive_track::begin_drive_track(sel.raw_track_index, sel.flags, ndx, ndy);
        drive_track.is_some()
    } else {
        false                     // <— current behavior: stop-rotate-go
    }
} else {
    *drive_track = None;
    false
};
```

Becomes (sketch — exact wording in `/write-plan`):

```rust
let (track_initiated, mdx, mdy) = if uses_drive_tracks && new_face != *facing {
    let (sel, dx, dy) = match drive_track::select_drive_track(*facing, new_face, false) {
        Some(sel) => (sel, ndx, ndy),
        None => {
            let sub = drive_track::fallback_drive_track(*facing);
            let (cdx, cdy) = drive_track::direction_to_cell_delta(
                drive_track::facing_to_dir(*facing)
            );
            (sub, cdx, cdy)
        }
    };
    let new_track =
        drive_track::begin_drive_track(sel.raw_track_index, sel.flags, dx, dy);
    *drive_track = new_track;
    (drive_track.is_some(), dx, dy)
} else {
    *drive_track = None;
    (false, ndx, ndy)
};
```

The lower block at lines 126-131 (`move_dir_x/y/len` for vehicles) then
uses `(mdx, mdy)` instead of `(ndx, ndy)`:

```rust
} else {
    let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(mdx, mdy);
    target.move_dir_x = d_x;
    target.move_dir_y = d_y;
    target.move_dir_len = d_len;
}
```

The infantry block at lines 111-125 is **unaffected** — infantry don't use
drive tracks, so `uses_drive_tracks` is false, the `track_initiated` flag
stays false, and `mdx, mdy` stay equal to `ndx, ndy`.

### Interfaces / Contracts

- `select_drive_track`: unchanged. Returns `None` on impossible turns.
- `fallback_drive_track`: pure, infallible, returns `DriveTrackSelection`.
- `direction_to_cell_delta`: pure, infallible, returns `(i32, i32)`.
- `begin_drive_track`: unchanged. Caller chooses `(dx, dy)`.

### Data Flow

1. Tick movement reaches a cell crossing.
2. `configure_motion_after_transition` runs.
3. `target.next_index += 1` (path queue shift — matches binary at `0x4b4607`).
4. `(ndx, ndy) = path[next_index] - current_cell`.
5. `new_face = facing_from_delta(ndx, ndy)`.
6. If Drive AND facing changes:
   - Try `select_drive_track`. Some → `(sel, mdx, mdy) = (sel, ndx, ndy)`.
     None → `(sel, mdx, mdy) = (fallback_drive_track(facing), cur_dx, cur_dy)`.
   - `begin_drive_track(sel.raw_track_index, sel.flags, mdx, mdy)`.
7. If track initiated → `facing_target = None`.
8. `move_dir_x/y/len` computed from `(mdx, mdy)` (the actual move
   direction, not necessarily the path's requested direction).

### Error Handling

No new error paths. `fallback_drive_track` is infallible by table-data
invariant; if violated (corrupt table edit), panic via array indexing.
Same failure mode as existing `TURN_TRACKS` access — no new fallibility
to plumb.

### Testing Strategy

**Unit tests (drive_track_tests.rs):**

1. `fallback_drive_track_returns_substitute_for_each_direction` — for the
   8 cardinal/diagonal facings (0, 32, 64, …, 224), verify the returned
   selection has the expected `raw_track_index`, `target_facing`, and
   `flags` from the binary table:

   | facing | from_dir | raw_track | target_facing | flags |
   |---:|---:|---:|---:|---:|
   | 0   | 0 | 1 | 0x00 | 0 |
   | 32  | 1 | 2 | 0x20 | 0 |
   | 64  | 2 | 1 | 0x40 | 3 |
   | 96  | 3 | 2 | 0x60 | 4 |
   | 128 | 4 | 1 | 0x80 | 4 |
   | 160 | 5 | 2 | 0xA0 | 1 |
   | 192 | 6 | 1 | 0xC0 | 1 |
   | 224 | 7 | 2 | 0xE0 | 2 |

2. `fallback_drive_track_quantization_with_rounding` — for off-axis facings
   (16, 48, 80, 112, 144, 176, 208, 240), verify the +16 rounding lands on
   the correct substitute entry. (Facing 16 should round to NE → entry 9.)

3. `direction_to_cell_delta_all_eight` — exhaustive table check for
   directions 0-7.

**Integration tests (existing or new test file under sim/movement):**

4. `vehicle_facing_n_path_step_s_takes_straight_track_n` — vehicle at
   (10, 10) facing N, path = [(10, 10), (10, 11)] (180° impossible),
   step once. Assert: `drive_track.is_some()`, `raw_track_index == 1`,
   `transform_flags == 0`. After enough ticks for the track to complete,
   assert unit's cell is (10, 9) — one cell *north*, opposite the path
   direction.

5. `successive_fallbacks_compound` — vehicle facing N, path = [(10, 10),
   (10, 11), (10, 12)] (two impossible 180° steps). Step until both tracks
   complete. Assert unit ends at (10, 8) — two cells north — confirming
   compounding behavior of ledger item 11.

6. `fallback_does_not_modify_final_goal` — set up a fallback scenario,
   step through it, assert `target.final_goal` is unchanged. (Speed ramp
   reads `final_goal`, so this is the regression guard for ledger item 7.)

7. `chain_attempt_with_impossible_target_does_not_substitute` — vehicle
   mid-track, chain target's facing combo has `normal_track == 0`. Step
   such that the chain attempt fires. Assert `drive_track.raw_track_index`
   is still the original track's index (chain refused, current track
   continues) — the regression guard for ledger item 9.

8. **Existing test stays:** `select_drive_track_null_track_returns_none`
   ([drive_track_tests.rs:232](../../src/sim/movement/drive_track_tests.rs#L232))
   — `select_drive_track`'s contract is unchanged.

### Determinism

- Substitute is pure (facing in → table lookup out).
- No RNG, no time, no global state.
- State hash post-fix will differ from pre-fix (vehicles now move where
  they previously stopped). Pre-fix replay snapshots will not replay
  correctly post-fix — expected and acceptable.

## Architectural Decisions

**Patterns followed:**
- "Caller calls module function, dispatches on Option" — same pattern as
  every other locomotor decision in the sim.
- Tiny pure helpers in `drive_track.rs` — same pattern as `facing_to_dir`,
  `transform_track_point`, etc.
- No new types or enums.

**Patterns NOT followed (intentionally):**
- The binary nests the substitute *inside* the track-selection table
  lookup. Rust splits it: `select_drive_track` returns Option, caller
  applies fallback. This split is cleaner because the chain caller (which
  must NOT fallback) shares the same lookup function.

**Tech debt introduced:** none.

**Tech debt left in place (out of scope):**
- `select_drive_track`'s `use_short: bool` parameter has no binary analog
  at the Process_Movement site. Whether to remove or repurpose deserves
  its own investigation — flagged in RE report §5.3.
- The chain caller at [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701)
  uses `entity.facing` instead of the current track's `target_facing` per
  binary behavior — separate parity gap, separate fix (RE report §4.4).
- The "crush-override" path at `0x4b3ff9..0x4b400c` also reaches
  `cur_dir*9` track_index. Different trigger, same destination. Implement
  separately when crush-into-buildings parity becomes load-bearing
  (RE report §2.3).

## Alternatives Considered

**Approach A: substitute inside `select_drive_track` with a `TrackContext`
enum.** Centralizes the fallback decision but adds a new enum and a third
parameter to the function purely to gate behavior at a single decision
point. Anti-pattern: "new pattern for no reason." Rejected.

**Approach C: separate `fallback_drive_track` helper + caller-side
wiring (same as B but with the helper as a top-level concern).** Dominated
by B — B already extracts the helper for testability while keeping the
caller logic localized. C added no architectural value over B. Rejected.
