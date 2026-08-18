# Drive Sharp-Turn Fallback Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** When pathfinding produces a turn too sharp for any TurnTrack curve,
drive the vehicle one cell straight ahead in current facing — matching
gamemd.exe `Process_Movement`'s `track_index = facing * 9` substitute at
`0x4b4023`.

**Architecture:** Caller-side fallback at
[`movement_step.rs::configure_motion_after_transition`](../../src/sim/movement/movement_step.rs#L69).
`select_drive_track` keeps its `Option<DriveTrackSelection>` contract (still
returns `None` on impossible turns); the initial-track-init caller detects
`None`, calls a new `fallback_drive_track(facing)` helper for the substitute
entry, and threads the chosen `(mdx, mdy)` direction through to
`begin_drive_track` and `move_dir`. Mirrors the binary's structure
(Process_Movement substitutes; Process_Drive_Track's chain branch does not).

**Design Doc:** [docs/plans/2026-05-05-drive-sharp-turn-fallback-design.md](2026-05-05-drive-sharp-turn-fallback-design.md)

---

## Grounding Summary

- **Docs:**
  [DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md)
  (today, HIGH confidence — three Q&A: substitute formula, post-substitute
  path mechanics, no chain-time analog).
  [DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md)
  and
  [PROCESS_DRIVE_TRACK_DECOMPILATION.md](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md)
  as background.
- **Ghidra verification (HIGH confidence, today):**
  - `0x4b4023` — substitute fires when byte +0x00 (`normal_track`) of the
    looked-up TurnTrack entry is 0; `loco.is_reversed`/use_short cleared at
    `0x4b4019` so `short_track` is never read at this site.
  - `0x4b4607` — path queue shifted left by 1 after substitute. `loco.dest`
    NOT modified.
  - `0x4b4685+` — `loco.head_to` set to NullCoord (binary's "no destination"
    sentinel).
  - Process_Drive_Track chain branch — no `facing*9` analog; refusal is
    final, current track finishes out.
- **Repo pattern:** New helpers mirror existing private helpers in
  [drive_track.rs](../../src/sim/movement/drive_track.rs) (`facing_to_dir`,
  `transform_track_point`). Direction-delta table mirrors `DIR_DELTAS` in
  [path_smooth.rs:31](../../src/sim/pathfinding/path_smooth.rs#L31) — same
  ordering convention (0=N, 1=NE, …, 7=NW). We add a new
  `DIRECTION_DELTAS` rather than expose path_smooth's private table.
- **INI keys:** None. The fallback is hard-coded engine behavior, not
  data-driven.
- **Unknowns confirmed during planning:**
  - `facing_to_dir` exists at
    [drive_track.rs:3494](../../src/sim/movement/drive_track.rs#L3494),
    currently private — must be made `pub(super)` for the caller to use.
  - `begin_drive_track` initializes `point_index` from `RAW_TRACKS[i].entry_index`
    ([drive_track.rs:3579](../../src/sim/movement/drive_track.rs#L3579)).
    For substitute tracks (RawTrack 1 and 2), `entry_index == 0`, matching
    the binary's "= 0" write. Resolves design ledger item 10.
  - `cell_delta_to_lepton_dir` exists at
    [util/lepton.rs:124](../../src/util/lepton.rs#L124) — reusable for the
    `move_dir` computation post-fallback.
  - Existing test
    [`select_drive_track_null_track_returns_none`](../../src/sim/movement/drive_track_tests.rs#L232)
    will continue to pass — `select_drive_track`'s contract is unchanged.

## Key Technical Decisions

- **Caller-side fallback (Approach B from brainstorm)** — Confidence: high.
  Source: design doc Architectural Decisions section + RE report Q3 (chain
  caller must NOT fallback).
- **`fallback_drive_track(current_facing: u8) -> DriveTrackSelection`
  infallible helper** — Confidence: high. All 8 substitute entries (TT
  indices 0, 9, 18, 27, 36, 45, 54, 63) have non-zero `normal_track` per
  the extracted TURN_TRACKS table. Source:
  [drive_track.rs:174-622](../../src/sim/movement/drive_track.rs#L174) and
  binary table read at `0x7e7b28`.
- **`direction_to_cell_delta(dir: usize) -> (i32, i32)` via new const
  `DIRECTION_DELTAS` in drive_track.rs** — Confidence: high. Same
  convention as `path_smooth::DIR_DELTAS`. Local copy avoids
  cross-module-coupling refactor.
- **Caller passes `(cur_dx, cur_dy)` (NOT `(0, 0)`) to `begin_drive_track`
  on fallback** — Confidence: high. Source: design ledger item 4. Rust's
  `head_offset` is cell-relative; the binary's `head_to = NullCoord` is a
  world-coord sentinel. The semantically equivalent Rust value is "the
  cur_dir cell's delta from current cell" — which for cur_dir cell is
  exactly `direction_to_cell_delta(facing_to_dir(facing))`.
- **Make `facing_to_dir` `pub(super)` (movement-module-internal)** —
  Confidence: high. Required by caller; least-invasive visibility bump.

No low-confidence decisions. Plan ready for execution as-is.

## Open Questions

### Resolved during planning

- Where does `DIR_DELTAS` live: existing one is private at
  [path_smooth.rs:31](../../src/sim/pathfinding/path_smooth.rs#L31). We add
  a new `DIRECTION_DELTAS` in drive_track.rs to keep the module
  self-contained. Cross-module refactor is scope creep.
- `facing_to_dir` visibility: changed to `pub(super)` so movement_step.rs
  can call it.
- `begin_drive_track` `point_index` reset: matches binary by accident
  (entry_index == 0 for tracks 1 and 2). No code change needed.
- Chain-caller regression test (design test #7): **dropped from the plan.**
  Constructing a mid-track chain-with-impossible-target test reliably is
  brittle (requires precise tick counts to land on the chain_index point
  with a specific path layout). The chain caller at
  [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701) is
  not modified by this plan; correctness is preserved by leaving its code
  alone. Code review confirms.

### Deferred to implementation

None.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs) | Add `DIRECTION_DELTAS`, `direction_to_cell_delta`, `fallback_drive_track`. Change `facing_to_dir` visibility. |
| Modify | [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) | Replace None arm in `configure_motion_after_transition`. Thread `(mdx, mdy)` through to vehicle move_dir computation. |
| Modify | [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) | Unit tests for `fallback_drive_track` and `direction_to_cell_delta`. |
| Modify | [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) | End-to-end integration tests: fallback fires, successive fallbacks compound, `final_goal` preserved. |

## Interface Changes

- `drive_track::facing_to_dir`: visibility private → `pub(super)`. Additive
  (more visible). No callers exist outside drive_track.rs today; the
  caller in movement_step.rs is added in Task 4.
- `drive_track::fallback_drive_track(current_facing: u8) -> DriveTrackSelection`:
  NEW `pub` function. Infallible. No external dependencies.
- `drive_track::direction_to_cell_delta(dir: usize) -> (i32, i32)`:
  NEW `pub(super)` function. Infallible (panics on dir ≥ 8 — same
  contract as existing array indexing in the module).
- `drive_track::DIRECTION_DELTAS`: NEW module-private const. Not exposed.

No public API surface area changes that affect callers outside the
movement module.

## Sim Checklist

- [x] All math is integer (cell deltas) or existing fixed-point
  (`cell_delta_to_lepton_dir` returns `SimFixed`). No new f32/f64 usage.
- [x] No new state fields added. The fallback only writes existing
  `DriveTrackState` and `MovementTarget` fields, all already in the
  state-hash inputs.
- [x] No dependencies added on render/ui/sidebar/audio/net.
- [x] Tick ordering unchanged. Fallback runs at the same site as today's
  `None` handling (after path-step advance, before track init).
- [x] BTreeMap iteration order: not affected (per-entity logic).

## Risk Areas

- **Existing test `select_drive_track_null_track_returns_none`** at
  [drive_track_tests.rs:232](../../src/sim/movement/drive_track_tests.rs#L232).
  Must continue to PASS — `select_drive_track`'s contract is unchanged.
  Verified by Task 7.
- **State hash divergence vs pre-fix runs.** Vehicles that previously
  stopped will now move on impossible turns. Pre-fix replay snapshots
  will not replay correctly post-fix — expected and acceptable.
- **Successive fallbacks compound** (ledger item 11). A path with multiple
  consecutive impossible steps will drive the unit progressively off-route
  until path is exhausted. Matches binary; recovery is at higher layers.
  Tested explicitly in Task 5.
- **Accidental chain-caller breakage.** If a future change wires the
  fallback into [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701)
  the binary's chain-time refusal contract (Q3) breaks. The plan does not
  modify movement_tick.rs at all; reviewer must confirm.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 4 | Vehicle takes a straight step in current_dir on impossible turn (instead of stop-rotate-go) | **Trigger frequency: high — fires every time pathfinding produces a >135° turn that the smoother didn't merge.** Visible to the player as the vehicle no longer pausing in place. | Task 5 integration test #1: facing N + path step S → unit ends up 1 cell *north*, not stopped in place. |
| Task 4 | Successive fallbacks compound (drive forward repeatedly) | Pathological paths drive the unit off-route — same as gamemd.exe; relies on higher-layer replanning to recover. Trigger frequency: low (only when pathfinder hands out malformed paths). | Task 5 integration test #2: two impossible steps → unit ends up 2 cells north. |
| Task 4 | `MovementTarget.final_goal` preserved through fallback | Speed ramp at [movement_tick.rs:506-508](../../src/sim/movement/movement_tick.rs#L506) reads `final_goal` for distance-to-destination. The binary doesn't touch `loco.dest` at the fallback site (`0x4b4023`); deceleration must continue toward the original ultimate destination. | Task 6 integration test: assert `target.final_goal` unchanged before/after a fallback step. |
| Task 4 | Chain-time path **NOT** modified | Q3: chain branch in Process_Drive_Track has no `facing*9` analog; current track finishes out. Substituting at chain time would visually break multi-track turns. | Plan does not touch movement_tick.rs. Code review verifies. |
| Task 1, 2 | Substitute formula `TURN_TRACKS[from_dir * 9]` matches binary table | Every entry's `raw_track_index`, `target_facing`, `flags` must match the binary read at `0x7e7b28`. A wrong row drives the unit in the wrong direction. | Task 1/2 unit tests assert all 8 substitute entries match the table values from the RE report. |

---

## Tasks

### Task 1: Add `DIRECTION_DELTAS` const and `direction_to_cell_delta` helper

**Why:** Independent helper, needed by Task 4. Adding first lets unit tests
verify it in isolation before the caller depends on it.

**Files:**
- Modify: [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs)
  (add near `facing_to_dir`, around line 3490)
- Modify: [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs)
  (append at end of file)

**Pattern:** Mirrors `DIR_DELTAS` in
[path_smooth.rs:31](../../src/sim/pathfinding/path_smooth.rs#L31). Same
ordering convention.

**Step 1: Add the const and helper**

Insert in `drive_track.rs` immediately after the existing `facing_to_dir`
function (currently at line 3494, just before line 3500's
`/// Result of selecting a drive track for a facing change.`):

```rust
/// Direction-index → (cell_dx, cell_dy) lookup.
/// Order matches `facing_to_dir`: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW.
const DIRECTION_DELTAS: [(i32, i32); 8] = [
    (0, -1),  // 0 = N
    (1, -1),  // 1 = NE
    (1, 0),   // 2 = E
    (1, 1),   // 3 = SE
    (0, 1),   // 4 = S
    (-1, 1),  // 5 = SW
    (-1, 0),  // 6 = W
    (-1, -1), // 7 = NW
];

/// Returns the cell-coordinate delta for a quantized direction index 0-7.
pub(super) fn direction_to_cell_delta(dir: usize) -> (i32, i32) {
    DIRECTION_DELTAS[dir % FACING_DIRECTIONS]
}
```

**Step 2: Add unit test**

Append to [drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs)
(at the end of the file, after the existing tests):

```rust
#[test]
fn direction_to_cell_delta_all_eight() {
    assert_eq!(direction_to_cell_delta(0), (0, -1), "N");
    assert_eq!(direction_to_cell_delta(1), (1, -1), "NE");
    assert_eq!(direction_to_cell_delta(2), (1, 0), "E");
    assert_eq!(direction_to_cell_delta(3), (1, 1), "SE");
    assert_eq!(direction_to_cell_delta(4), (0, 1), "S");
    assert_eq!(direction_to_cell_delta(5), (-1, 1), "SW");
    assert_eq!(direction_to_cell_delta(6), (-1, 0), "W");
    assert_eq!(direction_to_cell_delta(7), (-1, -1), "NW");
}

#[test]
fn direction_to_cell_delta_wraps_modulo_8() {
    assert_eq!(direction_to_cell_delta(8), direction_to_cell_delta(0));
    assert_eq!(direction_to_cell_delta(15), direction_to_cell_delta(7));
}
```

**Step 3: Verify**

Run: `cargo test -p vera20k --lib direction_to_cell_delta -- --nocapture`
Expected: 2 tests pass.

**Step 4: Commit**

```
git add src/sim/movement/drive_track.rs src/sim/movement/drive_track_tests.rs
git commit -m "drive_track: add DIRECTION_DELTAS + direction_to_cell_delta helper"
```

---

### Task 2: Add `fallback_drive_track` helper

**Why:** Core lookup for the sharp-turn fallback. Independent of caller
changes; testable in isolation.

**Files:**
- Modify: [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs)
  (add immediately after `select_drive_track`, around line 3492)
- Modify: [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs)
  (append)

**Pattern:** Mirrors the structure of `select_drive_track` but is
infallible and uses the substitute formula `TURN_TRACKS[from_dir * 9]`
verified against gamemd.exe `0x4b4023`.

**Step 1: Add the helper**

Insert in `drive_track.rs` immediately after the closing `}` of
`select_drive_track` (around line 3491, before
`/// Quantize a 0-255 facing to a direction index 0-7`):

```rust
/// Returns the substitute drive-track selection for an impossible turn.
///
/// When `select_drive_track` returns `None` (the looked-up TurnTrack
/// entry has `normal_track == 0` — turn too sharp), the initial-track-init
/// caller should call this to obtain a "drive straight ahead in current
/// facing" substitute. The returned track is one of the 8 diagonal-self
/// entries `TURN_TRACKS[from_dir * 9]`, all of which are non-null.
///
/// Pure function of facing — same +16 quantization as `facing_to_dir`,
/// no global state or randomness.
pub fn fallback_drive_track(current_facing: u8) -> DriveTrackSelection {
    let from_dir = facing_to_dir(current_facing);
    let turn_index = from_dir * FACING_DIRECTIONS + from_dir;
    let turn_track = &TURN_TRACKS[turn_index];
    let raw_meta = &RAW_TRACKS[turn_track.normal_track as usize];
    DriveTrackSelection {
        turn_track_index: turn_index,
        raw_track_index: turn_track.normal_track,
        entry_index: raw_meta.entry_index,
        chain_index: raw_meta.chain_index,
        cell_cross_index: raw_meta.cell_cross_index,
        points_count: raw_meta.points_count,
        target_facing: turn_track.target_facing,
        flags: turn_track.flags,
    }
}
```

**Step 2: Add unit tests**

Append to drive_track_tests.rs:

```rust
#[test]
fn fallback_drive_track_returns_substitute_for_each_direction() {
    // Diagonal entries TURN_TRACKS[i*9] from binary table at 0x7e7b28.
    // (facing, expected_raw, expected_target_facing, expected_flags)
    let cases: &[(u8, u8, u8, u8)] = &[
        (0,   1, 0x00, 0),
        (32,  2, 0x20, 0),
        (64,  1, 0x40, 3),
        (96,  2, 0x60, 4),
        (128, 1, 0x80, 4),
        (160, 2, 0xA0, 1),
        (192, 1, 0xC0, 1),
        (224, 2, 0xE0, 2),
    ];
    for &(facing, expected_raw, expected_target, expected_flags) in cases {
        let sel = fallback_drive_track(facing);
        assert_eq!(sel.raw_track_index, expected_raw, "facing {facing} raw");
        assert_eq!(sel.target_facing, expected_target, "facing {facing} target");
        assert_eq!(sel.flags, expected_flags, "facing {facing} flags");
    }
}

#[test]
fn fallback_drive_track_quantization_with_rounding() {
    // facing_to_dir uses +16 rounding then divides by 32.
    //   facing 15 → (15+16)/32 = 0 → N
    //   facing 16 → (16+16)/32 = 1 → NE
    //   facing 47 → (47+16)/32 = 1 → NE
    //   facing 48 → (48+16)/32 = 2 → E
    //   facing 240 → (240+16)/32 = 8 → 0 (mod 8) → N
    assert_eq!(fallback_drive_track(15).raw_track_index,
               fallback_drive_track(0).raw_track_index, "15 → N");
    assert_eq!(fallback_drive_track(16).raw_track_index,
               fallback_drive_track(32).raw_track_index, "16 → NE");
    assert_eq!(fallback_drive_track(47).raw_track_index,
               fallback_drive_track(32).raw_track_index, "47 → NE");
    assert_eq!(fallback_drive_track(48).raw_track_index,
               fallback_drive_track(64).raw_track_index, "48 → E");
    assert_eq!(fallback_drive_track(240).raw_track_index,
               fallback_drive_track(0).raw_track_index, "240 → N (wraps)");
}

#[test]
fn fallback_drive_track_substitutes_have_nonzero_raw_track() {
    // The whole substitute family must have valid (non-null) raw tracks.
    // Defensive guard against future TURN_TRACKS edits.
    for facing in [0u8, 32, 64, 96, 128, 160, 192, 224] {
        let sel = fallback_drive_track(facing);
        assert_ne!(sel.raw_track_index, 0,
                   "facing {facing} substitute must not be null track");
    }
}
```

**Step 3: Verify**

Run: `cargo test -p vera20k --lib fallback_drive_track -- --nocapture`
Expected: 3 tests pass.

**Step 4: Commit**

```
git add src/sim/movement/drive_track.rs src/sim/movement/drive_track_tests.rs
git commit -m "drive_track: add fallback_drive_track substitute helper"
```

---

### Task 3: Make `facing_to_dir` visible to the movement module

**Why:** Task 4's caller change in movement_step.rs needs to call
`facing_to_dir`. Currently private to drive_track.rs. Smallest possible
visibility bump: `pub(super)` — visible to the movement module only, not
the whole crate.

**Files:**
- Modify: [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs#L3494)

**Step 1: Change visibility**

At [drive_track.rs:3494](../../src/sim/movement/drive_track.rs#L3494), change:

```rust
fn facing_to_dir(facing: u8) -> usize {
```

to:

```rust
pub(super) fn facing_to_dir(facing: u8) -> usize {
```

**Step 2: Verify build**

Run: `cargo check -p vera20k`
Expected: Clean build, no warnings about visibility.

**Step 3: Commit**

```
git add src/sim/movement/drive_track.rs
git commit -m "drive_track: expose facing_to_dir to movement module"
```

---

### Task 4: Caller-side fallback in `configure_motion_after_transition`

**Why:** The load-bearing change. Replaces the current `None` arm
(stop-rotate-go) with the substitute path, threading the chosen movement
direction through to `move_dir_x/y/len` so the unit accelerates in the
right direction.

**Files:**
- Modify:
  [src/sim/movement/movement_step.rs:90-131](../../src/sim/movement/movement_step.rs#L90)

**Pattern:** Caller dispatches on `Option`, applies fallback on `None`.
The same pattern as today, just replacing the no-op `false` arm with a
real substitute.

**Step 1: Replace the `None` arm and thread `(mdx, mdy)` through**

Replace the block currently at
[movement_step.rs:87-131](../../src/sim/movement/movement_step.rs#L87)
(from the `let uses_drive_tracks` line through the closing of the vehicle
`else` block at line 131) with:

```rust
        let uses_drive_tracks = locomotor
            .as_ref()
            .is_some_and(|l| matches!(l.kind, LocomotorKind::Drive));
        let (track_initiated, mdx, mdy) = if uses_drive_tracks && new_face != *facing {
            let (sel, dx, dy) = match drive_track::select_drive_track(*facing, new_face, false) {
                Some(sel) => (sel, ndx, ndy),
                None => {
                    // Sharp-turn fallback: requested turn has no smooth curve.
                    // Substitute a straight track in current facing — drive one
                    // cell forward, let the next tick re-evaluate from the new
                    // position. Path step is already consumed (next_index += 1
                    // above), matching the binary's path_queue shift.
                    let sub = drive_track::fallback_drive_track(*facing);
                    let (cdx, cdy) = drive_track::direction_to_cell_delta(
                        drive_track::facing_to_dir(*facing),
                    );
                    (sub, cdx, cdy)
                }
            };
            *drive_track =
                drive_track::begin_drive_track(sel.raw_track_index, sel.flags, dx, dy);
            (drive_track.is_some(), dx, dy)
        } else {
            *drive_track = None;
            (false, ndx, ndy)
        };

        if track_initiated {
            *facing_target = None;
        } else if category == EntityCategory::Infantry || mover_rot <= 0 {
            *facing = new_face;
        } else {
            *facing_target = Some(new_face);
        }

        if category == EntityCategory::Infantry {
            // Infantry: direction from current sub-cell toward next cell's subcell position.
            // Use the allocated subcell offset to maintain visual spread during movement,
            // matching the WalkLocomotionClass which walks to FindSubCellDest result.
            let (sc_x, sc_y) = locomotor
                .as_ref()
                .and_then(|l| l.subcell_dest)
                .unwrap_or((CELL_CENTER_LEPTON, CELL_CENTER_LEPTON));
            let dest_x = SimFixed::from_num(ndx * 256) + sc_x;
            let dest_y = SimFixed::from_num(ndy * 256) + sc_y;
            let dx = dest_x - current_sub.0;
            let dy = dest_y - current_sub.1;
            target.move_dir_x = dx;
            target.move_dir_y = dy;
            target.move_dir_len = fixed_distance(dx, dy);
        } else {
            let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(mdx, mdy);
            target.move_dir_x = d_x;
            target.move_dir_y = d_y;
            target.move_dir_len = d_len;
        }
```

Notes on the diff:
- The match on `select_drive_track` replaces the `if let Some(...) {} else { false }`.
- `(track_initiated, mdx, mdy)` is a tuple so the lower vehicle move_dir
  block can use the actual movement direction.
- The infantry path is unchanged (keeps `ndx, ndy`) — infantry don't use
  drive tracks; `uses_drive_tracks` is false; `mdx, mdy` would equal
  `ndx, ndy` in the else arm anyway. Keeping `ndx, ndy` in the infantry
  block is functionally equivalent and avoids touching the
  subcell-walk math.
- The vehicle `else` at the bottom now uses `mdx, mdy` instead of
  `ndx, ndy`.

**Step 2: Verify build**

Run: `cargo check -p vera20k`
Expected: Clean build.

Run: `cargo test -p vera20k --lib drive_track`
Expected: All existing drive_track tests still pass, including
`select_drive_track_null_track_returns_none`.

Run: `cargo test -p vera20k --lib movement`
Expected: All existing movement tests still pass.

**Step 3: Commit**

```
git add src/sim/movement/movement_step.rs
git commit -m "movement_step: apply sharp-turn fallback on impossible-turn path step"
```

---

### Task 5: Integration test — single fallback drives in current facing

**Why:** End-to-end regression guard for the headline behavior. Without
this, the unit-tests on `fallback_drive_track` could pass while the
caller wires it up wrong (e.g., passing `(ndx, ndy)` instead of
`(cur_dx, cur_dy)` and the unit ending up in the wrong cell).

**Files:**
- Modify:
  [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs)
  (append at end)

**Pattern:** Mirrors existing `test_tick_movement_*` integration tests in
the same file. Adds a small `make_drive_loco()` helper for tests that need
an actual Drive locomotor (the existing tests rely on
`locomotor = None` defaulting to no-track movement, which doesn't trigger
the fallback path).

**Step 1: Add `make_drive_loco` helper**

Append to movement_tests.rs (immediately before the new tests):

```rust
fn make_drive_loco() -> crate::sim::movement::locomotor::LocomotorState {
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    LocomotorState {
        kind: LocomotorKind::Drive,
        layer: MovementLayer::Ground,
        phase: GroundMovePhase::Idle,
        air_phase: AirMovePhase::Landed,
        speed_multiplier: SIM_ONE,
        speed_fraction: SIM_ONE,
        fly_current_speed: SIM_ZERO,
        altitude: SIM_ZERO,
        target_altitude: SIM_ZERO,
        climb_rate: SIM_ZERO,
        jumpjet_speed: SIM_ZERO,
        jumpjet_wobbles: 0.0,
        jumpjet_accel: SIM_ZERO,
        jumpjet_current_speed: SIM_ZERO,
        jumpjet_deviation: 0,
        jumpjet_crash_speed: SIM_ZERO,
        jumpjet_turn_rate: 0,
        balloon_hover: false,
        hover_attack: false,
        speed_type: SpeedType::Track,
        movement_zone: MovementZone::Normal,
        rot: 5, // non-zero so vehicle wouldn't instant-rotate without fallback
        override_state: None,
        air_progress: SIM_ZERO,
        infantry_wobble_phase: 0.0,
        subcell_dest: None,
    }
}
```

(If a `make_drive_loco`-equivalent already exists in this file or another
test file, reuse it instead.)

**Step 2: Add the integration test**

Append to movement_tests.rs:

```rust
#[test]
fn vehicle_facing_n_path_step_s_takes_sharp_turn_fallback_north() {
    // Vehicle facing N (facing=0) at (10, 10). Path step requests cell
    // (10, 11) — south, a 180° impossible turn. The binary substitutes
    // track_index = facing*9 = 0 (straight-N track) and the unit drives
    // one cell NORTH instead of stopping to rotate. We mirror that here.
    let mut entities = EntityStore::new();
    let path: Vec<(u16, u16)> = vec![(10, 10), (10, 11)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 10, 10);
    e.facing = 0; // N
    e.locomotor = Some(make_drive_loco());
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(2560), // fast enough to finish track
        move_dir_x: SIM_ZERO,
        move_dir_y: SimFixed::from_num(-256), // initially going N
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    // Run enough ticks for the substitute track to complete (~one cell).
    for _ in 0..16 {
        tick_movement(&mut entities, 100, &mut test_interner());
    }

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.position.rx, 10, "x unchanged");
    assert_eq!(
        entity.position.ry, 9,
        "moved one cell NORTH (opposite of impossible-turn path)"
    );
    assert_eq!(entity.facing, 0, "facing unchanged — no rotation queued");
}
```

**Step 3: Verify**

Run: `cargo test -p vera20k --lib vehicle_facing_n_path_step_s_takes_sharp_turn_fallback_north -- --nocapture`
Expected: PASS.

**Step 4: Commit**

```
git add src/sim/movement/movement_tests.rs
git commit -m "movement: integration test — sharp-turn fallback drives in current facing"
```

---

### Task 6: Integration test — successive fallbacks compound; final_goal preserved

**Why:** Two parity-critical guards from the design's testing strategy
(items 5 and 6). Successive fallbacks confirm ledger item 11; final_goal
preservation confirms ledger item 7 / D5 §1 interaction.

**Files:**
- Modify:
  [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs)
  (append after Task 5's test)

**Step 1: Add the successive-fallback test**

Append:

```rust
#[test]
fn successive_sharp_turn_fallbacks_compound() {
    // Two consecutive impossible-turn path steps. The unit should drive
    // current_dir for each — ending up two cells NORTH of its start,
    // having ignored both path requests.
    let mut entities = EntityStore::new();
    let path: Vec<(u16, u16)> = vec![(10, 10), (10, 11), (10, 12)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 10, 10);
    e.facing = 0; // N
    e.locomotor = Some(make_drive_loco());
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 3],
        next_index: 1,
        speed: SimFixed::from_num(2560),
        move_dir_x: SIM_ZERO,
        move_dir_y: SimFixed::from_num(-256),
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    // Run enough ticks for two substitute tracks to complete (~two cells).
    for _ in 0..32 {
        tick_movement(&mut entities, 100, &mut test_interner());
    }

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.position.rx, 10, "x unchanged");
    assert_eq!(
        entity.position.ry, 8,
        "moved two cells NORTH (compound fallback over two impossible steps)"
    );
}
```

**Step 2: Add the final_goal-preservation test**

Append:

```rust
#[test]
fn sharp_turn_fallback_does_not_modify_final_goal() {
    // Speed ramp at movement_tick.rs:506-508 reads MovementTarget.final_goal
    // for distance-to-destination. The binary doesn't touch loco.dest at
    // the fallback site (0x4b4023); deceleration must continue to brake
    // toward the original ultimate destination.
    let mut entities = EntityStore::new();
    let path: Vec<(u16, u16)> = vec![(10, 10), (10, 11)];
    let goal = (10u16, 20u16);
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 10, 10);
    e.facing = 0; // N
    e.locomotor = Some(make_drive_loco());
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(2560),
        final_goal: Some(goal),
        move_dir_x: SIM_ZERO,
        move_dir_y: SimFixed::from_num(-256),
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    // Tick through the fallback.
    for _ in 0..16 {
        tick_movement(&mut entities, 100, &mut test_interner());
    }

    let entity = entities.get(1).expect("entity exists");
    if let Some(target) = entity.movement_target.as_ref() {
        assert_eq!(
            target.final_goal,
            Some(goal),
            "final_goal must NOT change through a fallback step"
        );
    }
    // (The MovementTarget may or may not still exist depending on whether
    // path was consumed; what matters is that, while it exists, final_goal
    // is preserved. If path completes mid-test, the next assertion is moot.)
}
```

**Step 3: Verify**

Run:
```
cargo test -p vera20k --lib successive_sharp_turn_fallbacks_compound -- --nocapture
cargo test -p vera20k --lib sharp_turn_fallback_does_not_modify_final_goal -- --nocapture
```
Expected: both PASS.

**Step 4: Commit**

```
git add src/sim/movement/movement_tests.rs
git commit -m "movement: integration tests — successive fallbacks + final_goal preserved"
```

---

### Task 7: Full-suite regression and clippy verification

**Why:** Confirm no existing test broke and no new clippy warnings landed.

**Files:** None (verification only).

**Step 1: Full library tests**

Run: `cargo test -p vera20k --lib`
Expected: All tests pass. If a previously-passing test now fails, **stop
and investigate** — most likely the failing test made an assumption that
silently relied on the stop-rotate-go behavior (e.g., a test that placed
a vehicle on a path with an impossible turn and asserted it stayed in
place). If that's the case:
- Check whether the test was a bug-by-accident (was the vehicle supposed
  to be moving?) → fix the test.
- Or whether the test was intentionally pinning the broken behavior →
  rewrite the test to match the new (correct) behavior, with a comment
  pointing at this plan.

**Step 2: Clippy**

Run: `cargo clippy -p vera20k --lib --tests -- -D warnings`
Expected: clean. If new warnings appear in the changed files, fix them
in a follow-up commit before declaring the task complete.

**Step 3: Confirm parity-critical items in passing tests**

Cross-reference the Parity-Critical Items table with the test names:
- Task 5's `vehicle_facing_n_path_step_s_takes_sharp_turn_fallback_north`
  → headline behavior verified.
- Task 6's `successive_sharp_turn_fallbacks_compound` → ledger item 11.
- Task 6's `sharp_turn_fallback_does_not_modify_final_goal` → ledger
  item 7.
- Task 1/2's unit tests → substitute formula matches binary table.
- Code review of [movement_tick.rs:701](../../src/sim/movement/movement_tick.rs#L701)
  → unchanged (Q3 chain-time refusal preserved).

**Step 4: No commit needed unless clippy fixes were applied.**

---

## Sources & References

- **Design doc:**
  [docs/plans/2026-05-05-drive-sharp-turn-fallback-design.md](2026-05-05-drive-sharp-turn-fallback-design.md)
- **Brainstorm/RE follow-up:**
  [DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md)
  (HIGH confidence on Q1/Q2/Q3, all three answers verified in Ghidra)
- **Background reports:**
  [DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md),
  [PROCESS_DRIVE_TRACK_DECOMPILATION.md](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md),
  [DRIVE_TRACK_TABLES_DEEP_DECODE.md](../../../ra2-rust-game-docs/DRIVE_TRACK_TABLES_DEEP_DECODE.md)
- **Source gap-scan:**
  [docs/gap-scans/2026-05-05c-gap-scan-drive_track-deep.md D2 §1](../gap-scans/2026-05-05c-gap-scan-drive_track-deep.md)
- **gamemd.exe addresses (kept here, not in Rust comments per CLAUDE.md):**
  - `0x4b4019` — `MOV byte [EBP+0x60], 0x0` (clear `is_reversed`/use_short
    immediately before substitute lookup)
  - `0x4b4023` — `MOV CL, byte ptr [EAX*4 + 0x7e7b28]` (read normal_track,
    +0x00 byte of TurnTrack entry)
  - `0x4b402e` — `LEA ECX, [EBX+EBX*8]` (compute `cur_dir * 9`)
  - `0x4b4031` — `MOV [EBP+0x58], ECX` (store substituted track_index)
  - `0x4b4607` — `REP MOVSD` (path queue shift)
  - `0x4b4685+` — write head_to = NullCoord
  - `0x7e7b28` — TurnTrack table base
- **INI keys:** none.
- **Related Rust code:**
  - [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs)
    (TURN_TRACKS, RAW_TRACKS, select_drive_track, begin_drive_track, facing_to_dir)
  - [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs)
    (configure_motion_after_transition — the modification site)
  - [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs)
    (chain caller at line 701 — NOT modified)
  - [src/sim/pathfinding/path_smooth.rs:31](../../src/sim/pathfinding/path_smooth.rs#L31)
    (DIR_DELTAS — pattern that DIRECTION_DELTAS mirrors)
  - [src/util/lepton.rs:124](../../src/util/lepton.rs#L124)
    (cell_delta_to_lepton_dir — used post-fallback for move_dir)
- **Recent related commits on dev:**
  - 4ebf932 drive_track: end-to-end smoothness test for sub-step interp
  - 29479a3 movement_step: apply interp_sub_step on mid-track normal path
  - 6f07f8a movement: drive-track decel uses 2D Euclidean lepton distance
