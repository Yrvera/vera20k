# Hover P2b — facing-directed steering (gamemd semantics, Rust-native)

Source contract: `docs/research/HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §3/§4/§6/§8.
Predecessors: P1 (keys + throttle math), P2a (throttle wired into speed ramp).

## Behavior contract being implemented

gamemd hover movement is a continuous integrator: the body facing steers toward the
current one-cell waypoint (snap when aligned, gradual at rules ROT while turning), and
the XY step each tick is `speed` along the **facing** (not the path vector). Forward
speed request drops to 0 while the needed turn exceeds 45° (0x2000), so hard turns brake
the unit; gentle turns produce facing-lagged curved paths. `HoverBoost` multiplies the
request when the next two queued path steps share a direction (clamped to 1.0 after).

## Rust-native translation (reuse, don't rebuild)

- **Facing**: reuse `entity.facing` (authoritative u8) + `entity.body_facing`
  (`FacingClass`, binary-frame, from M1) at rules ROT. No per-tick ROT deltas
  (forbidden per FRAME_BASIS negative-facts; FacingClass is the verified primitive).
- **XY step**: reuse the existing straight-line advancement + crossing machinery by
  redirecting `target.move_dir_{x,y}` to the facing unit vector
  (`cos_bam/sin_bam(facing16 − 0x4000)`, `move_dir_len = 1`) each tick. The advance
  steps `sub += move_dir · (speed·dt / len)` — identical math, new direction source.
  Occupancy / bridge / reservation code is untouched.
- **BAM mapping** (this repo's convention: facing 0=N, 64=E; atan2_bam 0=+x, 0x4000=+y):
  `facing16 = atan2_bam(dy, dx) + 0x4000`; `move-vector angle = facing16 − 0x4000`.
- **Turn-stall**: >45° from desired → request 0 (throttle brakes) AND hold position
  this tick. DISCLOSED APPROXIMATION: gamemd translates along the stale facing while
  braking during a >45° swing; our crossing loop is path-directed and cannot absorb a
  sideways cell exit, so we hold position during the hard-turn phase instead. Bounded
  drift (≤ the brake-decay tail, a few dozen leptons on sharp corners); strictly closer
  to gamemd than the previous stop-rotate-at-every-corner. UNVERIFIED-pending-trace.
- **Speed request** (`hover.rs`): 0 turning-hard; 0.5 within 255 leptons of the final
  goal (arrival slow-in, P2a) or within 255 leptons of the path start (departure
  slow-out, doc §3 step-start gate — per-PATH interpretation chosen: the per-STEP
  reading would cap cruise at 0.75·Speed, contradicting the verified post-boost clamp
  analysis; flagged UNVERIFIED-pending-trace); else 1.0. Boost mult when the next two
  path directions match; `target = min(mult·request, 1)`.
- **Drive-track suppression**: the repath/blocked-recovery path-creation site
  (`movement_tick.rs` ≈546) starts drive tracks with NO locomotor-kind gate — hover
  units get curve tables there today. Gate it on Drive (the command-issue site already
  is; `configure_motion_after_transition` already is).
- **Stop cleanup**: `finalize_finished_entities` clears `body_facing` (all movers) —
  the next order steers fresh; hover_throttle already zeroed (P2a).

## Files

- `src/sim/movement/hover.rs` — steering helpers + request/boost fns + unit tests.
- `src/sim/movement/movement_tick.rs` — hover steering branch (replaces
  handle_vehicle_rotation for hover), ramp-block request wiring, repath-site kind gate,
  finalize body_facing clear.
- `src/sim/movement/movement_tests.rs` — integration tests.

## Acceptance

1. Cardinal mapping unit tests: desired facing16 from deltas (N/E/S/W), facing→vector
   (64→(+1,0), 0→(0,−1), 128→(0,+1), 192→(−1,0)), turn-stall boundary (0x2000
   exclusive: 0x2000 not hard, 0x2001 hard — gamemd `> 0x2000`).
2. Straight east path: identical progression to P2a (aligned facing ⇒ unit vector ≡
   normalized cell vector), cells cross, throttle ramps.
3. 90° corner: throttle BRAKES during the swing (vs P2a freeze), facing converges to
   the new heading via body_facing at rules ROT, then movement resumes and the mover
   advances on the new axis. No position freeze with a frozen throttle; no
   sub-coordinate escape from the current cell during the swing.
4. Boost: request/mult unit test — two same-direction queued steps at request 0.5 →
   target 0.75; at request 1.0 → clamped 1.0.
5. Full lib suite: no new failures beyond the 2 known baseline ratchets.

## Explicitly deferred

- Translation-while-hard-turning (needs any-direction cell-crossing machinery).
- Step-start per-step vs per-path 0.5 interpretation (needs live gamemd speed trace).
- P3 vertical bob/damped-spring controller.
