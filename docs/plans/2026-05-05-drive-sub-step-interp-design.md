# Drive Sub-Step Visual Interpolation Design

## Goal

Make moving vehicles drift smoothly between discrete drive-track steps by writing
an interpolated lepton sub-cell position to `Position.sub_x/sub_y` every tick,
matching gamemd.exe's `Process_Drive_Track` Phase 7 (RESIDUAL).

## Architecture Context

**Binary** ([PROCESS_DRIVE_TRACK_DECOMPILATION.md §7](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md)):
After the step loop exits with `budget <= 7`, `Process_Drive_Track` computes a
fractional visual position by scaling the next step's transformed delta by
`budget * (1/7)` and applies it via `EnterCell` (`vtable+0x1B4` =
`TechnoClass__Set_Coords_With_Cloak` at `0x4DB810`). `EnterCell` writes the
position into the techno's canonical pos field via `ObjectClass::Set_Raw_Coords`,
so in the binary the sub-step lepton position IS the canonical sim position.
A safety check restricts the interpolated cell to `{saved_cell, full_step_cell}`
or to the `budget > 3` trust window — cell occupancy never updates mid-step.

**Rust** ([drive_track.rs:3599-3678](../../src/sim/movement/drive_track.rs#L3599-L3678)):
`advance_drive_track` walks the step loop while `budget >= 7`, stores the
leftover at `state.residual`, and returns the *last consumed step's* coords.
There is no fractional offset on the returned position. Render reads
`Position.sub_x/sub_y` indirectly via the cached `screen_x/screen_y`.

**Position layout** ([components.rs:31-60](../../src/sim/components.rs#L31-L60)):
`Position` already carries lepton sub-cell precision (`sub_x: SimFixed`,
`sub_y: SimFixed`, range `[0, 256)`) plus cached `screen_x/screen_y: f32`
marked `#[serde(skip, default)]` — already excluded from the deterministic
state hash. `refresh_screen_coords()` is the single recompute site, called
from every place that mutates world position.

**Bridge coupling** ([movement_bridge.rs:11-15](../../src/sim/movement/movement_bridge.rs#L11-L15)):
The existing TODO is about explicit bridge-layer state (FootClass+0x79), not
about sub-step bridge detection. The binary's Phase 7 only re-runs ramp
detection on the cell-transition arm of `EnterCell`, which the safety check
restricts to `interp_cell == full_step_cell`. Sub-step bridge transition is
just an early run of the same logic at the moment of crossing — independent
of the open TODO.

## Impact Analysis

**Files modified:**

- [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs) — extend
  `advance_drive_track` to expose the next step's transformed delta and both
  candidate endpoint cells; add a sub-step interp helper.
- [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) —
  call the sub-step interp at the end of each tick after the discrete-step
  walk, write the result through `Position` and `refresh_screen_coords`.
- [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) —
  unit tests for the interp math, the safety-cell gate, the early-exit cases.
- Other movement/combat tests that pin per-tick positions on moving vehicles
  will need expectation updates (one-time churn).

**No render changes.** `screen_x/y` cascade through `refresh_screen_coords`.

**Determinism:** sub-step pos is fixed-point (`SimFixed`), no f64. Sim state
hash now reflects sub-step lepton drift — replays remain deterministic, but
hash divergence localization sees per-tick churn for moving vehicles. Acceptable.

**Risk areas:**

- Combat range tests may shift threshold edges by ≤ 1 cell on certain ticks.
  Documented as L12 in the parity ledger; not probed before implementation.
- Test pin churn — one-time cost, mostly in `movement_tests.rs` and
  combat-with-moving-vehicle scenarios.
- Fixed-point rounding direction must be pinned and documented.

## Chosen Approach

**Approach C — sim-side sub-cell write (binary architecture, framed as exercising
existing sub-cell precision more often).**

Picked because:
1. The binary's Phase 7 writes the canonical pos every tick, so combat /
   vision / fire-from-position naturally see sub-step lepton positions.
   Render-side interp would silently drift these.
2. `Position` already stores lepton sub-cell coords. We're stepping an
   existing field at sub-tick granularity, not introducing a new concept.
3. `screen_x/screen_y` cascades for free through `refresh_screen_coords`.
   No render changes.
4. Sidesteps the L12 question (does sub-step pos have observable combat
   effects in YR?) — sim-side is correct regardless of the answer.

Trade vs render-side: ~2x more code and one-time test churn. No determinism bets.

## Tiny-Detail Ledger

Items the implementation must preserve. Each cites a source.

- **L1.** Step cost = 7 budget units. Loop runs while `budget >= 7`, so residual
  ∈ `{0..=6}`. `[doc: PROCESS_DRIVE_TRACK_DECOMPILATION.md §7]`
- **L2.** Interp factor = `budget / 7`, applied to the *next* step's transformed
  delta. Constant `1/7` lives at `DAT_007e7fa8` (f64 in binary). Our
  fixed-point form is `delta * residual / 7` with truncate-toward-zero rounding.
  `[doc §7 lines 806-808]`
- **L3.** Delta is computed by `Transform_Track_Coords` on the current
  (next-to-be-consumed) track step, applying the same transform_flags the
  next consumed step would use. `[doc §7 lines 797-808]`
- **L4.** Safety gate: use interp coords iff `interp_cell ∈ {saved_cell,
  full_step_cell}` **OR** `budget > 3`. Else fall back to `full_step_cell`
  coords. `[doc §7 lines 815-827]`
- **L5.** Early exit: if `residual < 1` or `track_index < 0`, no interp this
  tick (visual stays at last step's coords). `[doc §7 lines 771-772]`
- **L6.** Track-end sentinel skip: if `step.x == 0 && step.y == 0 &&
  point_index != 0`, no interp this tick. `[doc §7 lines 789-791]`
- **L7. DEFERRED — DOCUMENTED PARITY DRIFT.** Binary calls `EnterCell`
  (`Set_Coords_With_Cloak` at `0x4DB810`) every interp tick, which re-evaluates
  cloak (`DoCloak(0)/DoCloak(1)` bookend). Our cloak system has no sub-step
  re-eval hook today. Visible effect: a cloaked Mirage's mid-step shimmer
  cadence won't match. Trigger frequency: every tick of every cloaked moving
  vehicle. Player-visibility: very subtle. Below the parity bar; revisit if
  symptoms surface. `[ghidra 0x4DB810]`
- **L8.** Cell occupancy and `Position.rx/ry/z` never update mid-step —
  guaranteed by L4. The discrete step boundary is the only point where rx/ry
  can change. `[doc §7]`
- **L9. DEFERRED — DOCUMENTED PARITY DRIFT.** When `interp_cell !=
  current_cell` (only possible when `interp_cell == full_step_cell`, per L4),
  the binary runs ramp detection: `dst_height == src_height - 4 && dst.flag &
  0x100 → on_bridge = 1`; `src.flag & 0x100 → on_bridge = 0`. Our
  `movement_bridge` already runs equivalent height-based detection at cell
  crossings — but on the discrete step boundary, not on the visual sub-step.
  Visible effect: bridge-layer transitions snap on the discrete step, not on
  the visual sub-step crest. Trigger frequency: only when a vehicle crests a
  ramp mid-tick (rare). Player-visibility: very subtle one-tick snap. Below
  the parity bar; revisit if symptoms surface. `[doc §7 lines 850-863]`
- **L10.** `facing_lock` suppression: same-cell `EnterCell` path saves
  `facing_lock`, sets it to 0, calls EnterCell, restores. Different-cell path
  keeps `facing_lock` untouched. We have no `facing_lock` field today; this
  detail is moot until/unless one is introduced. Tracked but not actionable
  in this design. `[doc §7 lines 839-843]`
- **L11.** Fixed-point determinism: `(delta_dx, delta_dy)` is `(i32, i32)` in
  leptons (output of `transform_track_point`); `residual` is `i32` in
  `0..=6`. Compute `delta * residual / 7` as integer division (truncate
  toward zero) — matches f64 → ftol behavior closely enough; deterministic
  by construction. `[implementation choice]`
- **L12. UNKNOWN — NOT PROBED.** Whether sub-step lepton pos shifts of ≤7
  leptons (one step's worth) per axis change targeting / fire-from / vision
  outcomes in YR. Approach C makes this question moot — we get it right by
  default. `[ledger entry]`

## Design

### Components

**1. `advance_drive_track` return-shape extension.** New fields on
`DriveTrackAdvance` (or a sibling struct) carry the data sub-step interp
needs:

- `next_step_delta: (i32, i32)` — the transformed (dx, dy) the *next* step
  would apply if budget reached 7 again. Zero if no next step exists (track
  end / sentinel hit / `point_index < 0`).
- `next_step_endpoint_offset: (i32, i32)` — the cumulative offset into
  `cell_offset_x/y` if the next step were consumed; used to determine
  `full_step_cell`.
- `residual: i32` — already on `DriveTrackState` but exposed in the return
  for ergonomics.

Internally, `advance_drive_track` stops early as today; before returning,
it peeks at `points[point_index + 1]` (when present, not at sentinel, not
finished) and runs `transform_track_point` on it to populate
`next_step_delta`. No second loop; one-shot peek.

**2. Sub-step interp helper** — module-private fn in
[`drive_track.rs`](../../src/sim/movement/drive_track.rs):

```rust
fn interp_sub_step(
    saved_sub_x: SimFixed,
    saved_sub_y: SimFixed,
    next_delta_x: i32,
    next_delta_y: i32,
    residual: i32,
    saved_cell: (u16, u16),
    full_step_cell: (u16, u16),
) -> Option<InterpResult>
```

Returns `None` for L5/L6 early exits. Otherwise returns the interpolated
sub-cell position plus a flag describing whether the interp landed in
`saved_cell`, `full_step_cell`, or required the `budget > 3` trust window
(L4). Falls back to `full_step_cell` coords when L4 forbids interp.

Math (L2, L11): `interp_dx = next_delta_x * residual / 7` using integer
division (truncate toward zero); same for dy. Apply to `saved_sub_x +
interp_dx` (lepton-space). Cell membership is determined by floor-division
of the resulting (saved_sub + interp_d) against 256, mirroring the existing
cell-jump check at [drive_track.rs:3625-3634](../../src/sim/movement/drive_track.rs#L3625-L3634).

**3. Caller wiring** in [`movement_step.rs`](../../src/sim/movement/movement_step.rs):
After the existing per-tick step walk, before returning, call
`interp_sub_step` and write the result through:

- `position.sub_x`, `position.sub_y` (the interp output)
- `position.refresh_screen_coords()` (cascades to render)

`position.rx/ry/z` are never written by the interp path — guaranteed by L4
+ L8.

### Interfaces / Contracts

- `advance_drive_track` signature gets new fields on its return type. All
  existing callers must be updated; non-drive locomotors are unaffected.
- The interp helper is `pub(crate)` to allow tests.
- `Position` is unchanged structurally. The contract that
  `refresh_screen_coords()` is called after any sub-cell mutation is
  preserved.

### Data Flow

```
World::advance_tick
  → ground movement (existing)
    → movement_tick.rs → drive locomotor branch
      → advance_drive_track  (consumes whole steps, stores residual)
      → interp_sub_step      (NEW — peeks next step, applies fractional offset)
      → write position.sub_x/sub_y
      → position.refresh_screen_coords()  (cascades to render)
```

Sub-step interp runs inside the sim tick. Cell occupancy / pathfinding /
state hash all see the sub-step position from the same tick onward. Render
sees it via `screen_x/y` next frame.

### Error Handling

Library-internal — no I/O, no fallible ops. The helper returns `Option` for
L5/L6 early exits; the L4 fallback is an enum branch in the result, not an
error. No new `thiserror` variants needed.

### Testing Strategy

Unit tests in [`drive_track_tests.rs`](../../src/sim/movement/drive_track_tests.rs):

- **Interp math (L2, L11):** `(delta_x, delta_y, residual) → (interp_dx,
  interp_dy)` for each `residual ∈ {0..=6}`. Verify truncate-toward-zero
  rounding for negative deltas.
- **Early exits (L5, L6):** `residual == 0`, `track_index < 0`, sentinel
  step `(0, 0)` at `point_index != 0` — all return `None`.
- **Safety gate normal case (L4 main):** interp lands in `saved_cell` →
  use interp. Interp lands in `full_step_cell` → use interp.
- **Safety gate budget>3 trust window (L4 alt):** interp lands in some
  third cell, `residual > 3` → use interp anyway. Same case with
  `residual <= 3` → fall back to `full_step_cell` coords.
- **Cell-boundary edge:** interp result's `sub_x` exactly at 0 or 255 —
  verify cell membership classification matches the existing cell-jump
  logic.
- **Determinism:** the same `(state, speed, dt)` produces the same
  `(sub_x, sub_y)` across runs (existing test pattern).

Integration: a single end-to-end test that advances a Mirage Tank for N
ticks and asserts `screen_x` changes monotonically rather than in 7-lepton
jumps.

## Architectural Decisions

**Patterns followed:**
- Sub-cell precision in `Position` — already established, just exercised more.
- `refresh_screen_coords` cascade — already established, no new render path.
- `SimFixed` / integer math in sim — CLAUDE.md "All simulation math uses
  fixed-point". Interp uses integer leptons + integer division.
- `#[serde(skip, default)]` for render-only fields — already established.
- Determinism via fixed-point — preserved; no f32/f64 in sim.

**Patterns deviated from:** none.

**Tech debt introduced:**
- L7 (cloak re-eval per sub-step) deferred. Will need a hook into cloak
  state if symptoms surface. Documented in the ledger.
- L9 (bridge sub-step transition) deferred. Will piggy-back on
  `movement_bridge` once `FootClass+0x79` (the open TODO at
  [movement_bridge.rs:11](../../src/sim/movement/movement_bridge.rs#L11)) is
  resolved. Documented in the ledger.
- L12 unprobed. Approach C makes this safe; if it ever matters we get the
  right answer for free.

## Alternatives Considered

**Approach A — sim-side, framed as porting Phase 7.** Same code as Approach C;
different framing. Rejected the framing only — not the content. C is "we
already have sub-cell precision, just step it smoothly", which reads cleaner
in our codebase.

**Approach B — render-side smoothing.** Sim stores discrete step pos;
`build_instances.rs` interpolates at draw time using `residual` + delta + a
1/7 factor in f32. Smaller patch, zero determinism risk, zero sim test
churn. Rejected because it bets that sub-step lepton position has no
observable combat / vision / fire-from effects in YR (L12), and gives up
L7 (cloak re-eval) and L9 (bridge sub-step transition) by construction.
Approach C carries them all (L7/L9 as documented drifts) at ~2x effort.
The recommendation called the trade-off explicitly; user picked C.
