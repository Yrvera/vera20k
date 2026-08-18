# Drive Sharp-Turn Fallback Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make Drive vehicles drive forward through impossible-turn path
steps (matching gamemd.exe) instead of stop-rotate-go, and fix the chain
caller's `cur_face` source so it uses the active track's post-turn
`target_facing` instead of the live (mid-rotation) `entity.facing`.

**Architecture:** Pure additive change inside `sim/`. `drive_track.rs`
gets a new fallback synthesizer + one byte on `DriveTrackState`;
`movement_step.rs` adds a fallback branch in
`configure_motion_after_transition`; `movement_tick.rs` rewires the
chain caller's `cur_face` source. No new sim→render coupling, no INI
parsing, no RNG, no float in sim logic.

**Design Doc:** [docs/plans/2026-05-06-drive-sharp-turn-fallback-design.md](2026-05-06-drive-sharp-turn-fallback-design.md)

---

## Grounding Summary

- **Docs:** [DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md)
  answers all three load-bearing questions at HIGH confidence,
  verified-from-binary. Background context in
  [DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md),
  [PROCESS_DRIVE_TRACK_DECOMPILATION.md](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md),
  [DRIVE_TRACK_TABLES_DEEP_DECODE.md](../../../ra2-rust-game-docs/DRIVE_TRACK_TABLES_DEEP_DECODE.md).
- **Ghidra:** disassembled `0x4b3ff5..0x4b405b` (Process_Movement
  substitute site) and decompiled `0x4b0f20` (Process_Drive_Track,
  chain branch) directly during brainstorm. Both match the RE doc
  instruction-for-instruction. Substitute is `LEA ECX, [EBX+EBX*8]` at
  `0x4b402e`; chain quantization reads `pcStack_74[4]` (the current
  TurnTrack entry's `target_facing` byte) at `0x4b0f20`'s chain block.
- **Repo pattern mirrored:** `select_drive_track` at
  [drive_track.rs:3447](../../src/sim/movement/drive_track.rs#L3447) —
  pure table lookup that returns `Option<DriveTrackSelection>`. The
  new `build_sharp_turn_fallback` follows the same shape (pure,
  Option-returning, no side effects).
- **INI keys:** none. Drive track data is binary-extracted (`TURN_TRACKS`
  and `RAW_TRACKS` constants in `drive_track.rs`); the fallback uses
  the same tables. `rules(md).ini` is not consulted.
- **Unknown:** none load-bearing for this work. Three deferred items
  (crush/blocked override at `0x4b3ff9`, upstream `use_short` decision,
  chain `RawTrack.chain_index != 0` second clause) are documented in
  the design doc's Out of Scope section.
- **Git state:** recent commits in `src/sim/movement/` (since
  2026-05-05) are `interp_sub_step` smoothness work and parachute
  descent — neither touches the fallback site or the chain branch.
  Design premise still holds.

## Key Technical Decisions

- **Field on `DriveTrackState`: `target_facing: u8`** (vs.
  `turn_track_index: u16`). One byte, lookup-free at the chain hot
  path. **Confidence:** high — both options are valid; this is
  cheaper. **Source:** repo pattern (`DriveTrackSelection.target_facing`
  already exists — just propagate it into state).
- **`build_sharp_turn_fallback` returns `Option<DriveTrackSelection>`.**
  Defensive against unloaded track data; matches `select_drive_track`
  semantics so callers can chain `.or_else()` if needed.
  **Confidence:** high. **Source:** repo pattern (`select_drive_track`).
- **`dir_to_cell_delta` lives in `util/fixed_math.rs`** next to
  `facing_from_delta_int`. **Confidence:** high — same algorithmic
  family. **Source:** repo pattern.
- **Path-step consumption: `target.next_index += 1` in the fallback
  branch only.** Mirrors the binary's path-queue shift at `0x4b4607`.
  **Confidence:** high — verified-from-binary. **Source:**
  DRIVE_SHARP_TURN_FALLBACK_RE.md §3.2; Ghidra `REP MOVSD` with
  ECX=0x17.
- **Defensive fall-through to current rotate-then-drive if either
  `build_sharp_turn_fallback` or `begin_drive_track` returns `None`.**
  Never expected; preserves prior behavior on unexpected nulls.
  **Confidence:** high. **Source:** existing pattern in
  `configure_motion_after_transition`.

## Open Questions

### Resolved During Planning

- **Does `DriveTrackState` already store `target_facing`?** No — read
  drive_track.rs:138-165. Needs a new field. **Source:** direct read.
- **Does a `dir_to_cell_delta` helper already exist?** No — only the
  inverse `facing_from_delta_int` at fixed_math.rs:280. **Source:**
  grep across `src/util/`.
- **Is the chain branch reachable when `entity.drive_track == None`?**
  No — `DriveTrackChainReady` is returned only inside
  `if let Some(track_state)` at movement_step.rs:250. The chain caller's
  `unwrap_or(entity.facing)` is purely defensive. **Source:** direct
  read of movement_step.rs:250-280.

### Deferred to Implementation

None. All required facts are sourced.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/util/fixed_math.rs` | Add `dir_to_cell_delta(facing) -> (i32, i32)` |
| Modify | `src/sim/movement/drive_track.rs` | Add `target_facing: u8` to `DriveTrackState`; thread it through `begin_drive_track`; add `build_sharp_turn_fallback` helper |
| Modify | `src/sim/movement/movement_step.rs` | Fallback branch in `configure_motion_after_transition`; pass `target_facing` from `DriveTrackSelection` into `begin_drive_track` at the existing call site |
| Modify | `src/sim/movement/movement_tick.rs` | Chain `cur_face` reads `entity.drive_track.target_facing`; pass `target_facing` into `begin_drive_track` at chain success |
| Modify | `src/sim/movement/drive_track_tests.rs` | Unit tests for `build_sharp_turn_fallback`; update existing `begin_drive_track` test sites for new signature |
| Modify | `src/sim/movement/movement_tests.rs` | Integration tests for sharp-turn fallback + chain `cur_face` correctness |

## Interface Changes

- **`DriveTrackState`** — add `pub target_facing: u8`. Affects
  serde-derived snapshot serialization (one byte wider) and any
  state-hash that walks `DriveTrackState`. No external consumers
  outside `sim/movement/`.
- **`begin_drive_track`** — signature gains `target_facing: u8`
  parameter (4th positional after `head_dy`). Two call sites in
  `sim/movement/`: movement_step.rs:91 and movement_tick.rs:707. Tests
  in drive_track_tests.rs need updating too.
- **`build_sharp_turn_fallback(current_facing: u8) ->
  Option<DriveTrackSelection>`** — new public function in
  `drive_track.rs`. One caller (movement_step.rs).
- **`dir_to_cell_delta(facing: u8) -> (i32, i32)`** — new public
  function in `util/fixed_math.rs`. One caller (movement_step.rs).

## Sim Checklist

- [x] All math uses `fixed`-point or integer — no f32/f64 in game
  logic. `dir_to_cell_delta` is a pure integer-table lookup.
  `build_sharp_turn_fallback` is a pure table lookup.
- [x] New state included in deterministic state hash —
  `DriveTrackState` is already hashed (it's part of `GameEntity`); the
  new `target_facing: u8` automatically participates.
- [x] No dependencies on render/ui/sidebar/audio/net — all changes
  inside `sim/movement/` and `util/`.
- [x] Tick ordering impact noted — none. The fallback runs inside the
  existing ground-movement tick phase; no new phase, no new ordering
  edge.
- [x] BTreeMap iteration order considered — no change to entity
  iteration; per-entity state only.

## Risk Areas

- **Forgetting `target.next_index += 1`** would silently keep the
  impossible step in the queue and re-trigger the fallback every tick
  on the same step. **Mitigation:** integration test that asserts
  `next_index` advances by 2 across the fallback cell crossing.
- **Stale `move_dir_*` after fallback** — drive tracks own motion
  while active, but stale `move_dir_*` could leak into later code if a
  track exits early. **Mitigation:** set `move_dir_*` to substitute
  deltas in the fallback branch for symmetry with the surrounding
  code.
- **Chain `cur_face` regression** — this is in the chain hot path; a
  bug here would subtly break ordinary curve chaining.
  **Mitigation:** integration test covers both the parity bug fix
  (mid-turn `entity.facing != target_facing`) and the existing
  passing case (post-turn, equal). Both must pass.
- **State hash / replay break** — vehicles in active drive tracks now
  carry one extra byte in their snapshot. Pre-existing replays of
  sharp-turn paths will hash differently. **Mitigation:** flag in
  commit message; user re-records any bookmarked replays.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Sharp-turn fallback substitutes `cur_dir*9` instead of stop-rotate-go | Every vehicle that gets a 135°+ path step (very high frequency in real play) currently stops and rotates in place — visibly different from gamemd.exe which drives forward. | Integration test in Task 7 + visual verification by issuing a 180° move in-game and comparing to gamemd.exe. |
| Task 4 | `target.next_index += 1` consumes the impossible step | Without consumption, fallback re-triggers every tick on the same step → vehicle drives forward forever in current facing. Catastrophically visible. | Unit + integration assertion that `next_index` advances. |
| Task 5 | Chain `cur_face` uses active track's `target_facing` | Mid-turn chain selection picks a different curve than the binary → visible facing jitter / wrong-curve mid-turn. Frequency: every chained turn (common in path-heavy combat). | Integration test in Task 7 with `entity.facing != track.target_facing`. |
| Task 4 | Substitute uses RawTrack 1 (cardinals) or 2 (diagonals) with correct transform flags | Wrong transform → vehicle drives in the wrong direction during fallback. Visible immediately every fallback. | Unit test in Task 6 covering all 8 cur_dirs. |
| Task 4 | No cell-beyond pre-validation in fallback branch | Binary skips it (`flags & 8 == 0`). Adding it would refuse fallback in cases where binary would proceed → divergence. | Integration test asserts fallback fires regardless of next-cell walkability inspection. |

---

## Tasks

### Task 1: Add `dir_to_cell_delta` helper

**Why:** `configure_motion_after_transition`'s fallback branch needs
to compute the unit cell delta for "drive forward in current facing."
This is the inverse of `facing_from_delta_int`, restricted to the 8
quantized cardinal/diagonal directions. Lives in `util/fixed_math.rs`
next to its inverse.

**Files:**
- Modify: `src/util/fixed_math.rs`

**Pattern:** Pure integer helper, sibling to `facing_from_delta_int`.

**Step 1: Write the helper**

Add immediately after `facing_from_delta_int_u16` (around
fixed_math.rs:312):

```rust
/// Inverse of `facing_from_delta_int` for quantized 8-direction facings.
///
/// Quantizes the input facing to the nearest of 8 compass points
/// (32 facing units apart) and returns the unit cell delta for that
/// direction. Used by movement code that needs to step one cell in
/// the unit's current facing without a path waypoint.
///
/// Iso-grid convention (matches `facing_from_delta_int` and the
/// canonical `DIR_DELTAS` already used in
/// `src/sim/pathfinding/path_smooth.rs:31-40`):
///   N (0)   → ( 0, -1)
///   NE (32) → ( 1, -1)
///   E (64)  → ( 1,  0)
///   SE (96) → ( 1,  1)
///   S (128) → ( 0,  1)
///   SW (160)→ (-1,  1)
///   W (192) → (-1,  0)
///   NW (224)→ (-1, -1)
pub fn dir_to_cell_delta(facing: u8) -> (i32, i32) {
    // Add half-step (16) for nearest-direction rounding, then divide
    // by 32 to get a 0-7 direction index. Wrapping handles the 240+16
    // overflow (→ 0 = N) the same way `facing_to_dir` does in
    // drive_track.rs.
    let dir = (facing.wrapping_add(16) / 32) & 0x07;
    match dir {
        0 => (0, -1),  // N
        1 => (1, -1),  // NE
        2 => (1, 0),   // E
        3 => (1, 1),   // SE
        4 => (0, 1),   // S
        5 => (-1, 1),  // SW
        6 => (-1, 0),  // W
        7 => (-1, -1), // NW
        _ => unreachable!(),
    }
}
```

> **Note for executor:** the same 8-tuple is already defined as
> `DIR_DELTAS` in
> [src/sim/pathfinding/path_smooth.rs:31-40](../../src/sim/pathfinding/path_smooth.rs#L31-40).
> If you want a single source of truth, lift that constant into
> `util/fixed_math.rs` as `pub const DIR_DELTAS: [(i32, i32); 8]` and
> have `dir_to_cell_delta` index into it
> (`DIR_DELTAS[((facing.wrapping_add(16) / 32) & 7) as usize]`); update
> `path_smooth.rs` to import the shared one. This is optional cleanup —
> the inline match above is correct on its own.

**Step 2: Add tests**

Append to the `tests` module at the bottom of `fixed_math.rs`:

```rust
#[test]
fn dir_to_cell_delta_quantized_directions() {
    assert_eq!(dir_to_cell_delta(0),   (0, -1));   // N
    assert_eq!(dir_to_cell_delta(32),  (1, -1));   // NE
    assert_eq!(dir_to_cell_delta(64),  (1, 0));    // E
    assert_eq!(dir_to_cell_delta(96),  (1, 1));    // SE
    assert_eq!(dir_to_cell_delta(128), (0, 1));    // S
    assert_eq!(dir_to_cell_delta(160), (-1, 1));   // SW
    assert_eq!(dir_to_cell_delta(192), (-1, 0));   // W
    assert_eq!(dir_to_cell_delta(224), (-1, -1));  // NW
}

#[test]
fn dir_to_cell_delta_rounds_to_nearest() {
    // 16 is the boundary, rounds up to NE — same direction
    // `facing_to_dir` rounds to.
    assert_eq!(dir_to_cell_delta(16), (1, -1));
    // 17 → NE
    assert_eq!(dir_to_cell_delta(17), (1, -1));
    // 240 wraps: (240 + 16) % 256 = 0 → N
    assert_eq!(dir_to_cell_delta(240), (0, -1));
    // 248 wraps: (248 + 16) % 256 = 8 → 8/32 = 0 → N
    assert_eq!(dir_to_cell_delta(248), (0, -1));
}

#[test]
fn dir_to_cell_delta_round_trips_through_facing_from_delta() {
    // For each quantized facing, dir_to_cell_delta then
    // facing_from_delta_int should land in the same direction bucket
    // (within ±16 of the original, since facing_from_delta_int is
    // continuous and we're sampling integer deltas).
    for &f in &[0u8, 32, 64, 96, 128, 160, 192, 224] {
        let (dx, dy) = dir_to_cell_delta(f);
        let recovered = facing_from_delta_int(dx, dy);
        let diff = (recovered as i16 - f as i16).rem_euclid(256);
        let dist = diff.min(256 - diff);
        assert!(
            dist <= 4,
            "facing {} → ({},{}) → {} (dist {})",
            f, dx, dy, recovered, dist
        );
    }
}
```

**Step 3: Verify**

Run: `cargo test --lib dir_to_cell_delta`
Expected: 3 tests pass.

**Step 4: Commit**

Message: `util: add dir_to_cell_delta helper for facing→cell delta`

---

### Task 2: Add `target_facing` field to `DriveTrackState`

**Why:** The chain caller at movement_tick.rs:690 needs to read the
active track's post-turn facing (per binary at `pcStack_74[4]`).
Adding it as a field is cheaper than looking it up via stored
`turn_track_index` at the hot path. Must be done before the new
fallback synthesizer (Task 4) and before the chain caller fix (Task 5).

**Files:**
- Modify: `src/sim/movement/drive_track.rs`

**Pattern:** Additive field on existing serde-derived struct.

**Step 1: Add the field**

In [drive_track.rs:138-165](../../src/sim/movement/drive_track.rs#L138),
add after `cell_offset_y`:

```rust
    /// Lepton Y offset applied after a mid-track cell transition.
    pub cell_offset_y: i32,
    /// Post-turn facing (TURN_TRACKS[turn_track_index].target_facing).
    /// Read by the chain caller as the "from-dir" for chain-target
    /// track selection — using the live entity facing would pick the
    /// wrong curve mid-turn.
    pub target_facing: u8,
}
```

**Step 2: Thread through `begin_drive_track`**

Update the signature and body at
[drive_track.rs:3566-3587](../../src/sim/movement/drive_track.rs#L3566):

```rust
pub fn begin_drive_track(
    raw_track_index: u8,
    transform_flags: u8,
    head_dx: i32,
    head_dy: i32,
    target_facing: u8,
) -> Option<DriveTrackState> {
    let meta = RAW_TRACKS.get(raw_track_index as usize)?;
    let points = raw_track_points(raw_track_index);
    if points.is_empty() {
        return None;
    }
    Some(DriveTrackState {
        raw_track_index,
        point_index: meta.entry_index,
        residual: 0,
        transform_flags: transform_flags & 0x07,
        head_offset_x: head_dx * 256 + 128,
        head_offset_y: head_dy * 256 + 128,
        cell_offset_x: 0,
        cell_offset_y: 0,
        target_facing,
    })
}
```

**Step 3: Update doc comment**

Adjust the `///` block above `begin_drive_track` to mention the new
parameter:

```rust
/// `target_facing`: post-turn facing (TURN_TRACKS[idx].target_facing
/// from the selection that produced this track). Stored on the state
/// for the chain caller to read.
pub fn begin_drive_track(
```

**Step 4: Update existing call sites**

The compiler will flag two:

- [movement_step.rs:91-93](../../src/sim/movement/movement_step.rs#L91):
  pass `sel.target_facing` as the 5th argument.
  ```rust
  *drive_track =
      drive_track::begin_drive_track(sel.raw_track_index, sel.flags, ndx, ndy, sel.target_facing);
  ```
- [movement_tick.rs:706-712](../../src/sim/movement/movement_tick.rs#L706):
  same — pass `sel.target_facing`.
  ```rust
  if let Some(new_track) =
      super::drive_track::begin_drive_track(
          sel.raw_track_index,
          sel.flags,
          chain_dx,
          chain_dy,
          sel.target_facing,
      )
  ```

**Step 5: Update test call sites**

Search `drive_track_tests.rs` for `begin_drive_track(`:

```
grep -n "begin_drive_track(" src/sim/movement/drive_track_tests.rs
```

For each call, append a 5th argument. For tracks that are tied to a
specific TurnTrack (e.g. `track_3_begin_starts_at_entry_12`), use the
matching `TURN_TRACKS[idx].target_facing`. For raw construction tests
that don't care about facing semantics, pass `0` (the default
behavior is unchanged for those tests).

If a test calls `begin_drive_track(3, 0, 1, -1)` — that's an N→NE
turn, which is `TURN_TRACKS[1]` with `target_facing = 0x20`. Update
to `begin_drive_track(3, 0, 1, -1, 0x20)`.

**Step 6: Verify compile**

Run: `cargo build`
Expected: clean build, no errors.

**Step 7: Verify tests pass**

Run: `cargo test --lib drive_track`
Expected: all existing drive_track tests still pass (the new field is
additive and call sites carry a sensible value).

**Step 8: Commit**

Message: `drive_track: store target_facing on DriveTrackState`

---

### Task 3: Add `build_sharp_turn_fallback` helper

**Why:** Synthesizes the substitute selection for sharp turns.
Mirrors the binary's `LEA ECX, [EBX+EBX*8]` at `0x4b402e`: looks up
`TURN_TRACKS[cur_dir * 9]` (entries 0/9/18/27/36/45/54/63 — the
diagonal of the 8×8 matrix, all of which point to RawTrack 1 or 2
with the right transform flags).

**Files:**
- Modify: `src/sim/movement/drive_track.rs`

**Pattern:** Pure table lookup, mirrors `select_drive_track`.

**Step 1: Add the helper**

Insert immediately after `select_drive_track` at
[drive_track.rs:3491](../../src/sim/movement/drive_track.rs#L3491):

```rust
/// Synthesize the substitute selection used when pathfinding produces
/// a turn too sharp for any precomputed curve (`select_drive_track`
/// returned `None`). Returns the `cur_dir * 9` TURN_TRACKS entry —
/// RawTrack 1 (cardinals) or 2 (diagonals) transformed for the unit's
/// current facing. The unit drives forward in `current_facing` for
/// one cell; the caller is responsible for consuming the impossible
/// path step.
///
/// Returns `None` defensively if RawTrack 1 or 2 point data isn't
/// loaded (never expected in normal operation — these are foundational
/// tracks).
pub fn build_sharp_turn_fallback(current_facing: u8) -> Option<DriveTrackSelection> {
    let cur_dir = facing_to_dir(current_facing);
    let turn_index = cur_dir * FACING_DIRECTIONS + cur_dir; // cur_dir * 9
    let turn_track = TURN_TRACKS.get(turn_index)?;
    if turn_track.normal_track == 0 {
        return None; // structurally impossible for cur_dir*9 entries; defensive
    }
    let raw_meta = RAW_TRACKS.get(turn_track.normal_track as usize)?;
    let points = raw_track_points(turn_track.normal_track);
    if points.is_empty() {
        return None;
    }
    Some(DriveTrackSelection {
        turn_track_index: turn_index,
        raw_track_index: turn_track.normal_track,
        entry_index: raw_meta.entry_index,
        chain_index: raw_meta.chain_index,
        cell_cross_index: raw_meta.cell_cross_index,
        points_count: raw_meta.points_count,
        target_facing: turn_track.target_facing,
        flags: turn_track.flags,
    })
}
```

**Step 2: Verify compile**

Run: `cargo build`
Expected: clean.

**Step 3: Commit (no tests yet — added in Task 6)**

Message: `drive_track: add build_sharp_turn_fallback helper`

---

### Task 4: Wire fallback into `configure_motion_after_transition`

**Why:** The single behavioral change that flips drive vehicles from
stop-rotate-go to drive-forward-and-consume on impossible turns. Must
come after Task 1 (`dir_to_cell_delta`), Task 2 (`begin_drive_track`
signature), and Task 3 (`build_sharp_turn_fallback`).

**Files:**
- Modify: `src/sim/movement/movement_step.rs`

**Pattern:** Existing if-let-Some / else fall-through pattern in the
same function — the fallback branch is added between the existing
`Some(sel)` and the `else { false }`.

**Step 1: Replace the `track_initiated` block**

Current code at
[movement_step.rs:90-101](../../src/sim/movement/movement_step.rs#L90):

```rust
        let track_initiated = if uses_drive_tracks && new_face != *facing {
            if let Some(sel) = drive_track::select_drive_track(*facing, new_face, false) {
                *drive_track =
                    drive_track::begin_drive_track(sel.raw_track_index, sel.flags, ndx, ndy, sel.target_facing);
                drive_track.is_some()
            } else {
                false
            }
        } else {
            *drive_track = None;
            false
        };
```

Replace with:

```rust
        let mut substituted_delta: Option<(i32, i32)> = None;
        let track_initiated = if uses_drive_tracks && new_face != *facing {
            if let Some(sel) = drive_track::select_drive_track(*facing, new_face, false) {
                *drive_track =
                    drive_track::begin_drive_track(sel.raw_track_index, sel.flags, ndx, ndy, sel.target_facing);
                drive_track.is_some()
            } else if let Some(fb) = drive_track::build_sharp_turn_fallback(*facing) {
                // Sharp-turn fallback: no precomputed curve exists for this
                // turn angle. Mirrors gamemd.exe's `cur_dir*9` substitute —
                // drive forward in current facing for one cell and consume
                // the impossible path step. The path queue's "what to do
                // next" is permanently dropped by one entry; recovery is
                // handled by higher-level repath, not here.
                let (cdx, cdy) = crate::util::fixed_math::dir_to_cell_delta(*facing);
                *drive_track = drive_track::begin_drive_track(
                    fb.raw_track_index,
                    fb.flags,
                    cdx,
                    cdy,
                    fb.target_facing,
                );
                if drive_track.is_some() {
                    target.next_index += 1; // drop the impossible path step
                    substituted_delta = Some((cdx, cdy));
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            *drive_track = None;
            false
        };
```

**Step 2: Override `move_dir_*` for the substitute branch**

The existing move_dir computation at lines 126-131 uses the original
`(ndx, ndy)`. For the substitute, override with the substitute deltas
so move_dir reflects actual motion direction.

Current (around movement_step.rs:126):

```rust
        } else {
            let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(ndx, ndy);
            target.move_dir_x = d_x;
            target.move_dir_y = d_y;
            target.move_dir_len = d_len;
        }
```

Replace with:

```rust
        } else {
            let (eff_dx, eff_dy) = substituted_delta.unwrap_or((ndx, ndy));
            let (d_x, d_y, d_len) = crate::util::lepton::cell_delta_to_lepton_dir(eff_dx, eff_dy);
            target.move_dir_x = d_x;
            target.move_dir_y = d_y;
            target.move_dir_len = d_len;
        }
```

**Step 3: Verify compile**

Run: `cargo build`
Expected: clean.

**Step 4: Verify existing tests still pass**

Run: `cargo test --lib movement`
Expected: all existing movement tests still pass. The new branch
fires only when `select_drive_track` returns `None` for a vehicle
with a direction change — non-sharp paths are unaffected.

**Step 5: Commit**

Message: `movement: drive sharp-turn fallback substitutes cur_dir*9 forward step`

---

### Task 5: Fix chain caller's `cur_face` source

**Why:** The chain attempt's "from-dir" should be the post-turn
facing (binary reads `pcStack_74[4]` = current TurnTrack's
`target_facing`), not the live (mid-rotation) `entity.facing`. Must
come after Task 2 (so `target_facing` is on the state).

**Files:**
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Single-line read change at the chain caller's `cur_face`
binding.

**Step 1: Replace the binding**

Current code at
[movement_tick.rs:690](../../src/sim/movement/movement_tick.rs#L690):

```rust
                            let next_face = super::facing_from_delta(ndx, ndy);
                            let cur_face = entity.facing;
```

Replace with:

```rust
                            let next_face = super::facing_from_delta(ndx, ndy);
                            // Use the active track's post-turn facing as the
                            // chain "from-dir" — by the time the chain attempt
                            // fires (at chain_index of the current track), the
                            // unit's logical destination facing is already set
                            // even though entity.facing is mid-rotation. The
                            // unwrap_or is defensive: DriveTrackChainReady is
                            // only produced inside an active track.
                            let cur_face = entity
                                .drive_track
                                .as_ref()
                                .map(|t| t.target_facing)
                                .unwrap_or(entity.facing);
```

**Step 2: Verify compile**

Run: `cargo build`
Expected: clean.

**Step 3: Verify existing tests still pass**

Run: `cargo test --lib movement`
Expected: existing tests pass. The change only affects mid-turn
chain attempts where `entity.facing != target_facing`; tests written
against the prior (incorrect) behavior may need updates if any
explicitly assert chain-target track based on `entity.facing`. None
known at plan-write time; the compiler will tell us.

**Step 4: Commit**

Message: `movement: chain caller reads target_facing from active drive track`

---

### Task 6: Unit tests for `build_sharp_turn_fallback`

**Why:** Lock down the substitute synthesizer for all 8 quantized
cur_dirs and confirm transform-flag tables match the binary.

**Files:**
- Modify: `src/sim/movement/drive_track_tests.rs`

**Pattern:** Existing `select_drive_track_*` test style.

**Step 1: Add tests**

Append to `drive_track_tests.rs` after the existing
`select_drive_track_*` tests:

```rust
// ---------------------------------------------------------------------------
// build_sharp_turn_fallback tests
// ---------------------------------------------------------------------------

#[test]
fn build_sharp_turn_fallback_cardinals_use_raw_track_1() {
    // Cardinal cur_dirs (N, E, S, W) at facings 0, 64, 128, 192.
    // Each maps to TURN_TRACKS[cur_dir * 9] with normal_track = 1.
    for facing in [0u8, 64, 128, 192] {
        let fb = build_sharp_turn_fallback(facing).unwrap_or_else(|| {
            panic!("fallback should exist for cardinal facing {}", facing)
        });
        assert_eq!(
            fb.raw_track_index, 1,
            "cardinal facing {} should use RawTrack 1 (straight)",
            facing
        );
    }
}

#[test]
fn build_sharp_turn_fallback_diagonals_use_raw_track_2() {
    // Diagonal cur_dirs (NE, SE, SW, NW) at facings 32, 96, 160, 224.
    for facing in [32u8, 96, 160, 224] {
        let fb = build_sharp_turn_fallback(facing).unwrap_or_else(|| {
            panic!("fallback should exist for diagonal facing {}", facing)
        });
        assert_eq!(
            fb.raw_track_index, 2,
            "diagonal facing {} should use RawTrack 2 (straight diagonal)",
            facing
        );
    }
}

#[test]
fn build_sharp_turn_fallback_transform_flags_match_binary() {
    // Verified-from-binary (DRIVE_SHARP_TURN_FALLBACK_RE.md §5.2):
    //   cur_dir | TT idx | flags (low 3 bits)
    //   0 (N)   | 0      | 0
    //   1 (NE)  | 9      | 0
    //   2 (E)   | 18     | 3
    //   3 (SE)  | 27     | 4
    //   4 (S)   | 36     | 4
    //   5 (SW)  | 45     | 1
    //   6 (W)   | 54     | 1
    //   7 (NW)  | 63     | 2
    let cases: &[(u8, u8)] = &[
        (0, 0),     // N
        (32, 0),    // NE
        (64, 3),    // E
        (96, 4),    // SE
        (128, 4),   // S
        (160, 1),   // SW
        (192, 1),   // W
        (224, 2),   // NW
    ];
    for &(facing, expected_low3) in cases {
        let fb = build_sharp_turn_fallback(facing).unwrap();
        assert_eq!(
            fb.flags & 0x07,
            expected_low3,
            "facing {} should have transform flags low3 = {}",
            facing,
            expected_low3
        );
    }
}

#[test]
fn build_sharp_turn_fallback_target_facing_matches_quantized_cur_dir() {
    // The substitute is "drive forward in current_facing", so target_facing
    // equals current_facing quantized to 8 directions.
    let cases: &[(u8, u8)] = &[
        (0, 0x00),
        (32, 0x20),
        (64, 0x40),
        (96, 0x60),
        (128, 0x80),
        (160, 0xA0),
        (192, 0xC0),
        (224, 0xE0),
    ];
    for &(facing, expected_target) in cases {
        let fb = build_sharp_turn_fallback(facing).unwrap();
        assert_eq!(
            fb.target_facing, expected_target,
            "facing {} substitute should have target_facing 0x{:02X}",
            facing, expected_target
        );
    }
}

#[test]
fn build_sharp_turn_fallback_rounds_to_nearest_dir() {
    // Non-quantized facings round to the nearest 8-direction bucket.
    // 17 → NE (1), same as facing 32.
    let fb_17 = build_sharp_turn_fallback(17).unwrap();
    let fb_32 = build_sharp_turn_fallback(32).unwrap();
    assert_eq!(fb_17.raw_track_index, fb_32.raw_track_index);
    assert_eq!(fb_17.flags, fb_32.flags);
    assert_eq!(fb_17.target_facing, fb_32.target_facing);
}
```

**Step 2: Verify**

Run: `cargo test --lib build_sharp_turn_fallback`
Expected: 5 tests pass.

**Step 3: Commit**

Message: `drive_track: tests for build_sharp_turn_fallback`

---

### Task 7: Integration test — sharp-turn fallback consumes path step

**Why:** End-to-end coverage that the fallback fires through
`configure_motion_after_transition`, that `next_index` advances, and
that no `facing_target` is set (no stop-rotate-go).

**Files:**
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing movement integration tests in this file.

**Step 1: Inspect existing test scaffolding**

Open `movement_tests.rs` and find the helper that builds a
`MovementTarget` + `Position` + `LocomotorState` for a Drive vehicle
at a given cell with a given facing. Reuse it. If no such helper
exists, add tests using the same scaffolding pattern as the closest
existing movement integration test (search for `MovementTarget {`
constructions).

**Step 2: Add the 180° impossible turn test**

Add a new `#[test]` function:

```rust
#[test]
fn sharp_turn_fallback_180_consumes_path_step_no_rotate() {
    // Setup: Drive vehicle at cell (10,10), facing N (0).
    // Path constructed so that after the first cell crossing, the
    // *next* path step's required facing is 180° from current.
    //
    // Per `facing_from_delta_int` (the codebase's iso-grid quantizer):
    //   delta (0,+1) → facing 128 (S) — opposite of N (0).
    //
    // After cell crossing into the next cell, configure_motion_after_transition
    // sees new_face = facing_from_delta(0, 1) = 128 (S), current
    // facing = 0 (N). |128 - 0| = 128 facing units = 180° → too sharp.
    // select_drive_track returns None → fallback fires.
    //
    // Expectation:
    //   - drive_track is Some (substitute track active)
    //   - facing_target is None (no stop-rotate-go)
    //   - target.next_index advanced past the impossible step
    //
    // (Exact scaffolding mirrors the closest existing integration test.
    // The important assertions are the three above.)

    // [scaffolding to construct a vehicle at (10,10) facing 0,
    //  path [(10,10),(10,11),(10,12)], rot > 0]
    // [advance one tick worth of cell crossing into (10,11)]
    // [call configure_motion_after_transition or run the tick that
    //  invokes it]

    // Replace placeholder construction with the actual helper from
    // this file. The pattern below is illustrative; adapt to the
    // local helper signatures.
    //
    // let mut target = build_target(...);
    // let mut position = build_position(10, 10);
    // let mut facing: u8 = 0;
    // let mut facing_target: Option<u8> = None;
    // let mut drive_track: Option<DriveTrackState> = None;
    // let locomotor = Some(LocomotorState { kind: LocomotorKind::Drive, .. });
    //
    // configure_motion_after_transition(
    //     &mut target, &locomotor, &mut drive_track,
    //     &mut facing, &mut facing_target,
    //     EntityCategory::Vehicle, /* rot */ 50,
    //     (10, 11), (CELL_CENTER_LEPTON, CELL_CENTER_LEPTON),
    // );
    //
    // assert!(drive_track.is_some(), "fallback track should be active");
    // assert!(facing_target.is_none(), "no stop-rotate-go");
    // assert_eq!(target.next_index, /* impossible step + cell-crossing increment */);
}
```

**Note for executor:** the placeholder block above is intentional —
the existing test helpers in `movement_tests.rs` aren't fully
characterized at plan-write time. Read 2-3 of the closest existing
integration tests in this file, then write the test using the same
helpers. The three assertions (drive_track is Some, facing_target is
None, next_index advanced past the impossible step) are the
load-bearing parts.

**Step 3: Add the chain `cur_face` correctness test**

Add another `#[test]` function:

```rust
#[test]
fn chain_cur_face_uses_track_target_facing_not_entity_facing() {
    // Setup: vehicle mid-turn — current drive track has
    // target_facing = NE (0x20), entity.facing = N (0x00) (haven't
    // rotated to target yet).
    // Path: next cell is in the direction that requires E (0x40)
    // from the post-turn facing.
    //
    // Chain quantization (binary at 0x4b0f20 chain block):
    //   from-dir = quantize(target_facing) = 1 (NE)
    //   to-dir   = quantize(next_face) = 2 (E)
    //   TURN_TRACKS[from*8 + to] = TURN_TRACKS[10] (NE→E turn)
    //
    // Vs. wrong (pre-fix) behavior:
    //   from-dir = quantize(entity.facing) = 0 (N)
    //   to-dir   = quantize(next_face) = 2 (E)
    //   TURN_TRACKS[2] (N→E turn — entirely different curve)
    //
    // Expectation: the chain selection picks TURN_TRACKS[10], not
    // TURN_TRACKS[2]. Easiest assertion: check that the active drive
    // track's raw_track_index matches what TURN_TRACKS[10].normal_track
    // produces, not TURN_TRACKS[2].normal_track.

    // [scaffolding: build vehicle with active drive_track having
    //  target_facing = 0x20, entity.facing = 0x00, set up
    //  DriveTrackChainReady arm to fire]
    // [run the tick / invoke the chain branch]

    // assert_eq!(
    //     entity.drive_track.unwrap().raw_track_index,
    //     TURN_TRACKS[10].normal_track,
    //     "chain should pick NE→E curve, not N→E"
    // );
}
```

**Note for executor:** same scaffolding caveat as Task 7 Step 2.

**Step 4: Verify**

Run: `cargo test --lib sharp_turn_fallback chain_cur_face`
Expected: 2 tests pass.

**Step 5: Commit**

Message: `movement: integration tests for sharp-turn fallback + chain cur_face`

---

### Task 8: Successive-fallback integration test

**Why:** Confirms that two impossible turns in a row both fire the
fallback and both consume their path step (a 180°-180° path doesn't
produce infinite re-trigger or stuck state).

**Files:**
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Same scaffolding as Task 7.

**Step 1: Add the test**

```rust
#[test]
fn successive_sharp_turn_fallbacks_consume_steps_independently() {
    // Setup: vehicle at (10,10), facing N. Path constructed so two
    // back-to-back path steps both produce 135°+ turn angles relative
    // to current facing → fallback fires twice.
    //
    // Tick 1: cross into next cell, fallback substitutes forward step,
    //   next_index advances by 2 (one for cell crossing, one for
    //   impossible-step consumption).
    // Tick 2: substitute track completes, next cell crossing fires
    //   configure_motion_after_transition again, fallback fires
    //   again, next_index advances by another 2.
    //
    // Expectation: after two cell crossings, vehicle has driven
    // forward in current_facing twice (no rotation), next_index has
    // advanced past four path entries total.

    // [scaffolding follows the same pattern as Task 7]
}
```

**Step 2: Verify**

Run: `cargo test --lib successive_sharp_turn_fallbacks`
Expected: 1 test passes.

**Step 3: Commit**

Message: `movement: integration test for successive sharp-turn fallbacks`

---

### Task 9: Verification against gamemd.exe

**Why:** Confirm the implementation matches original engine behavior
in a real game scenario, not just unit tests.

**Verify:**

1. **Disassembly cross-check (already done at design time, re-verify
   after implementation lands):** the substitute index `cur_dir * 9`
   matches `LEA ECX, [EBX + EBX*8]` at `0x4b402e`. The transform
   flags table in `build_sharp_turn_fallback`'s test (Task 6 Step 1)
   matches the binary's `TURN_TRACKS[i*9].flags` for i in 0..8. Done
   via Ghidra MCP `disassemble_bytes` at `0x4b3ff5..0x4b405b`.
2. **In-game observation:** issue a Grizzly Tank a move order that
   forces a 180° turn in confined space (e.g., dead-end corridor in a
   skirmish map). Compare side-by-side with gamemd.exe. The Rust
   build should now drive forward and consume the impossible step
   (matching gamemd.exe), not stop and rotate in place (current
   behavior).
3. **Path-drift behavior:** issue a long path that would require a
   sharp turn near the start. The vehicle should commit forward
   briefly, then the higher-level repath should kick in and route
   correctly. Match gamemd.exe's recovery cadence (one or two cells
   off-route before repath).
4. **Chain mid-turn correctness:** issue a Grizzly a move order with
   two consecutive turns. Visually, the chain should produce one
   continuous smooth motion — not the visual jitter / wrong-curve
   that the old `entity.facing` source might have caused.

**Expected:** behavior is indistinguishable from gamemd.exe in all four
checks.

**No commit** — verification only. Document any drift or surprises in
a follow-up `/disparity-scan drive sharp-turn` if needed.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-drive-sharp-turn-fallback-design.md](2026-05-06-drive-sharp-turn-fallback-design.md)
- **Primary RE doc:** [ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md](../../../ra2-rust-game-docs/DRIVE_SHARP_TURN_FALLBACK_RE.md) — HIGH confidence, verified-from-binary
- **Background docs:**
  - [DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md)
  - [PROCESS_DRIVE_TRACK_DECOMPILATION.md](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md)
  - [DRIVE_TRACK_TABLES_DEEP_DECODE.md](../../../ra2-rust-game-docs/DRIVE_TRACK_TABLES_DEEP_DECODE.md)
  - [DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md)
- **Source gap-scan:** [docs/gap-scans/2026-05-05c-gap-scan-drive_track-deep.md](../gap-scans/2026-05-05c-gap-scan-drive_track-deep.md) §D2.1
- **gamemd.exe addresses (in this report only — do not put in Rust comments):**
  - `0x4b2630` — `DriveLocomotionClass::Process_Movement`
  - `0x4b3ff5..0x4b405b` — track-selection phase + fallback
  - `0x4b4019` — clears `loco.is_reversed` (use_short)
  - `0x4b4023` — reads `TURN_TRACKS[idx].normal_track` byte
  - `0x4b402e` — `LEA ECX, [EBX + EBX*8]` substitute (cur_dir*9)
  - `0x4b403a` — `flags & 8` cell-crossing test
  - `0x4b4045` — JZ to `0x4b45f6` no-cell-crossing tail
  - `0x4b4607` — `REP MOVSD` path-queue shift
  - `0x4b0f20` — `Process_Drive_Track` chain branch
  - TURN_TRACKS table at `0x7e7b28..0x7e7e88`
- **INI keys:** none. Drive track data is binary-extracted; no
  `rules(md).ini` consumption.
- **Related code:**
  - [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs)
  - [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs)
  - [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs)
  - [src/util/fixed_math.rs](../../src/util/fixed_math.rs)
- **Prior commits touching this code (for context):**
  - `4ebf932` — drive_track end-to-end smoothness test
  - `0dc54e8` — interp_sub_step helper
  - `bc5e285` — DriveTrackAdvance next-step peek fields
