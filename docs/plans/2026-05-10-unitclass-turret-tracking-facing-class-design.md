# UnitClass Turret Tracking — FacingClass Equivalent — Design

## Goal

Reimplement gamemd.exe's per-tick turret rotation, fire-decision codes, and Fire_At_Target → Facing_Update tick order using a Rust-native `FacingClass` value type and a `FireDecision` enum, achieving observable parity with the binary's turret behavior on UnitClass instances.

## Architecture Context

Today the engine handles turret tracking and fire-decision in three places:

- **`src/sim/movement/turret.rs`** — `tick_turret_rotation` is a per-tick step-clamp. Each tick it computes `max_delta = rot_to_facing_delta_u16(rot, tick_ms)` and rotates `entity.turret_facing: Option<u16>` toward a desired facing by at most `max_delta`. There is no concept of a per-rotation timer, no "rotation-in-progress" flag, no snap-on-tiny-rotation behavior.
- **`src/sim/combat/combat_fire_gate.rs`** — `collect_fire_blocked_entities` returns a `BTreeSet<u64>` of "can't fire this tick" IDs. The gate is boolean: there is no record of *why* a unit was blocked.
- **`src/sim/combat/mod.rs`** — the snapshot loop in `tick_combat_with_fog` walks each attacker, `continue`-ing past cooldown, friendly-fire, visibility, range, and a flat-tolerance turret-alignment check (`is_turret_aligned_u16` with a hardcoded 2048/65536 tolerance independent of ROT). On success it fires; on alignment failure it continues without recording the reason.

**Tick order in `World::advance_tick` Phase 5** (`src/sim/world/mod.rs:1146-1170`):

```
turret::tick_turret_rotation
   → tick_capture_orders
   → tick_order_intents_pre_combat
   → tick_attack_pursuit
   → tick_combat_with_fog
```

The comment at line 1148 says "turret rotation MUST run before combat so turrets are aligned when firing." This is the **opposite** of gamemd, which runs `Fire_At_Target` BEFORE `Facing_Update`. In the binary, the fire decision uses *last tick's* facing, so target-acquisition→fire takes one extra binary frame; our impl resolves it the same tick. Real, observable disparity.

**Facing convention.** `facing_toward_lepton` calls `facing_from_delta_int_u16` which uses `atan2(dx, -dy)` (`src/util/fixed_math.rs:308`). gamemd's `compute_facing_to_target` uses `atan2(dy, -dx)` with `Math::ftol` truncate-toward-zero (research doc §6). These do not produce the same u16 quantization for the same lepton geometry; the axis convention needs to be verified against a binary trace.

**GameEntity facing fields** (`src/sim/game_entity.rs`):
- `facing: u8` — body, written instantly by the locomotor, no smoothing.
- `facing_target: Option<u8>` — body destination during a multi-tick rotation.
- `turret_facing: Option<u16>` — turret/barrel, 16-bit, no separate destination/timer.

**Time base.** Sim ticks at variable `tick_ms` (test values: 22, 33, 100). gamemd ticks at fixed 15 Hz. Cooldowns are bridged via `rof_to_cooldown_ticks(rof_frames, tick_ms)` which converts gamemd frames → our ticks. There is currently no synthetic gamemd-frame counter on `World`.

## Impact Analysis

**New files:**
- `src/sim/movement/facing_class.rs` — `FacingClass` struct + tests.
- `src/sim/combat/fire_decision.rs` — `FireDecision` enum + post-fire dispatch helpers.

**Modified files (significant):**
- `src/sim/game_entity.rs` — replace `turret_facing: Option<u16>` with `barrel_facing: Option<FacingClass>`. Body fields (`facing: u8`, `facing_target: Option<u8>`) **remain unchanged in this round** (deferred follow-up).
- `src/sim/movement/turret.rs` — rewrite `tick_turret_rotation` around `FacingClass`. Delete `rot_to_facing_delta_u16` (subsumed by `FacingClass::set_rot`). Keep `facing_toward_lepton` with axis-convention verification.
- `src/sim/combat/mod.rs` — replace `is_turret_aligned_u16` tolerance check with `barrel.current(binary_frame) == desired`. Thread `FireDecision` through the snapshot loop's match arms. Gattling stage update keys on the decision variant.
- `src/sim/combat/combat_targeting.rs` — `AttackerSnapshot::turret_facing: Option<u16>` becomes `Option<FacingClass>` (Copy type, cheap snapshot).
- `src/sim/world/mod.rs` Phase 5 — flip order: `tick_combat_with_fog` runs **before** `tick_turret_rotation`. Add `binary_frame: u32` and `total_sim_ms: u64` fields on `World`, advanced at the top of each `advance_tick`.
- `src/util/fixed_math.rs` — verify `facing_from_delta_int_u16` axis convention against binary; fix if needed; add binary-trace ground-truth test.

**Blast radius (greppable):**
- `entity.turret_facing` reads: ~15 sites — combat alignment check, retaliation, render, snapshot/state-hash. All migrate to `entity.barrel_facing.as_ref().map(|f| f.current(binary_frame))`.
- `is_turret_aligned_u16` callers: 1 site in combat. Drops in favor of direct `current(frame) == desired` comparison.
- `tick_turret_rotation` signature: `tick_ms: u32` becomes `binary_frame: u32`. Every test that calls it adapts to the new signature.
- Tick-order flip: any test that asserts "set target, advance one tick, expect fired this tick" now expects fire on the tick AFTER alignment completes. Test fallout will be visible from the test suite.

**Snapshot / serialization.** `FacingClass` is 14 bytes (4×u16 + Option<u32> ≈ 14 with niche). Replaces `Option<u16>` (~3 bytes). Net ~11 bytes per turreted entity. At 20k entities (worst case, project scale target) and ~25% turreted: ~55 KB extra per snapshot. Negligible.

`World` gains `binary_frame: u32` (derived) and `total_sim_ms: u64` (authoritative). Both serialize.

**Determinism.** All `FacingClass` operations are integer (i16 wrapping_sub, u16 division, u32 saturating_sub). No new floats. The only float remains in `facing_from_delta_int_u16` (atan2 → quantize to u16), which is unchanged in nondeterminism profile. State hash trivially extends to `FacingClass` fields and the new `World` time fields.

**Risk areas.**
- Atan2 axis convention fix may regress existing tests if our current u16 facings happen to be self-consistent under the wrong convention. Mitigation: add binary-trace test FIRST, then change the formula, then run the suite.
- Tick-order flip may expose tests that hold incorrect assumptions about rotate-and-fire same-tick behavior. Mitigation: surface via test failures, audit each, update assertion timing.
- `binary_frame` advancement formula must be drift-free across long sessions. Use `binary_frame = (total_sim_ms * 15) / 1000` (integer division), not accumulator-based, to avoid rounding drift.

## Chosen Approach

A timer-based `FacingClass` value type that mirrors gamemd's interpolation algorithm, paired with a behavioral-subset `FireDecision` enum and a flipped tick order. ROT is stored in binary units; FacingClass time is in synthetic 15 Hz binary frames derived from accumulated `tick_ms`. Body smoothing is deferred as a documented follow-up.

This combination is the minimum design that captures the binary's observable turret behavior end-to-end:
- The 24-byte FacingClass primitive is the source of truth for "where in its rotation lifecycle is this turret right now" — patching the step-clamp model in place would require recomputing step_size/duration each tick anyway, which IS the FacingClass, just badly factored.
- Synthetic binary-frame time base means our `current(frame)` returns byte-exact-binary animated values at every binary-frame boundary, including the visible "stepped" rotation gamemd produces. Render can interpolate visually between binary frames if extra smoothness is wanted later — but as a render concern, never sim state.
- FireDecision codes are needed because gattling spin-up and force-fire dispatch key off the decision variant, not just a boolean fire/no-fire.
- Tick order flip preserves the binary's 1-tick acquisition-to-fire latency.

Body smoothing is deferred because the body refactor is structurally independent of the turret refactor (the same `FacingClass` type works for both) and including body in this round triples the call-site migration without changing the architecture. Player-visible drift from the deferral: vehicle bodies still snap-rotate where binary smooths over a few binary frames — noticeable on heavy-tank bodies during fast direction changes, but the turret (the thing the eye tracks during combat) is fixed.

## Tiny-Detail Ledger

Parity-relevant items the implementation must preserve. Each cites its source and identifies its home in the design.

| # | Item | Source | Home |
|---|---|---|---|
| 1 | ROT byte clamped at 0x7F (input > 126 → 127). | doc §2.7 / `0x4C9680` | `FacingClass::set_rot()` |
| 2 | ROT stored as `(byte << 8)`; INI ROT=5 → 1280 facing units per binary frame. | doc §2.7 | `FacingClass::set_rot()`, internal `rot_per_frame: u16` field |
| 3 | `step_size = abs(diff)/ROT < 1` snaps (no interp); rotation requests smaller than one frame's worth of ROT bypass interpolation. | doc §2.2 | `FacingClass::current()` early-return when `step_size == 0` |
| 4 | Wrap-around via signed short subtraction: going 0xFFE0 → 0x0010 traverses +0x30, not -0xFFD0. | doc §2.3 | `i16` cast on `current.wrapping_sub(prev)` for diff |
| 5 | `Set` snapshots animated value into `prev` BEFORE writing new target, so retargets don't stutter. | doc §2.4 | `FacingClass::set()` calls `self.current(frame)` first |
| 6 | `start_frame == -1` is "never started" sentinel, not a duration-0 case. | doc §9 #12 | `start_frame: Option<u32>` (Rust None ≡ -1) |
| 7 | CDTimer mystery field at `+0x0C` — uninit-stack write, never read by interpolation. | doc §2.5 | Omitted from struct |
| 8 | `compute_facing_to_target` uses `atan2(dy, -dx)` with `Math::ftol` truncate-toward-zero. | doc §6 | `facing_toward_lepton` axis-convention fix; verify `as i32` truncation matches |
| 9 | Tick order: Fire_At_Target → Facing_Update; fire decision uses LAST binary frame's facing. | doc §7 | Phase 5 reorder: `tick_combat_with_fog` → `tick_turret_rotation` |
| 10 | Body FacingClass keeps `ROT=3` (FacingClass-ctor default) regardless of rules.ini ROT. | doc §1.2 | **DEFERRED** — body smoothing is a separate follow-up; documented in §Alternatives Considered |
| 11 | Turret/barrel get `Type.ROT << 8` from rules.ini at spawn. | doc §1.1 | Spawn-time `barrel.set_rot(obj.turret_rot as u8)` |
| 12 | GetFireError code 5 collapses ~30 sub-conditions, all "no fire this tick". | doc §4.2 | `FireDecision::Generic` single variant |
| 13 | Force-fire (code 9) → `CanFireAt` + state lock, not direct fire. | doc §4.7 | `FireDecision::ForceFire` post-dispatch handler |
| 14 | Gattling stage update gates on err ∈ {0, 2, 3, 4}: spin-up; else decay. | doc §4.8 | `FireDecision::drives_gattling_spinup()` true for `{Fire, Facing, Cooldown, Generic}` |
| 15 | err==0\|2 + auto-deploy=true → mission 0x10 (IFV/Mirage Tank deploy auto-trigger). | doc §4.3 | **DEFERRED** — no IFV auto-deploy mission yet; flagged as known gap |
| 16 | Harvester turn-anim: 5-tick timer, 8-direction lookup `((facing >> 0xC) + 1) >> 1) & 7`. | doc §4.4 | **DEFERRED** — own follow-up; current Rust has a different turret-spin hack at `turret.rs:205` (also marked for cleanup) |
| 17 | TurretSpins: target advances `(rounded_8bit + 8) << 8` per binary frame, 32 ticks/rev, 256-step quantized. | doc §5.2 | **DEFERRED** — blocked on Floating Disk render. Design must accommodate: verify `FacingClass` with `rot_byte=100` correctly handles per-frame `set` calls with 2048-unit deltas (test included) |
| 18 | `field_0x4A0` per-tick "is rotating" latch — Facing_Update writes `CDTimerClass::Remaining(BarrelFacing)` into this byte. | doc §5.5 | `FacingClass::is_rotating(frame) -> bool`, computed on demand (not cached) |
| 19 | TurretAI 8-cell idle scan, every 8 binary frames, 1-cell radius, distinct from retaliation. | doc §3.1 | **PARTIAL** — current `tick_retaliation` is broader; explicit 8-cell scan flagged as separate follow-up |
| 20 | Idle no-target turret returns to body facing (Facing_Update Section A). | doc §5.1 | `tick_turret_rotation` calls `barrel.set(body_facing_u16, frame)` for targetless units |

**Deferred items with explicit prerequisites:** #10 (body smoothing), #15 (IFV auto-deploy), #16 (harvester turn-anim), #17 (TurretSpins / Floating Disk), #19 (TurretAI explicit scan). All other items have a concrete home in this design.

## Design

### Components

#### `FacingClass` — `src/sim/movement/facing_class.rs`

```rust
/// Timer-based 16-bit facing interpolator, mirroring gamemd's FacingClass primitive.
///
/// At any binary frame, the animated value is computed from `prev`, `current`,
/// `start_frame`, and `rot_per_frame` — there is no per-frame "advance" call.
/// Setting a new target snapshots the current animated value into `prev` so
/// rotations retarget smoothly without snap-back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FacingClass {
    /// Destination — where the rotation will end up. 16-bit DirStruct.
    current: u16,
    /// Where the current rotation began. Updated on `set` to the animated
    /// value at the moment of the new request (ledger #5).
    prev: u16,
    /// Binary frame when the rotation began. None = never started
    /// (binary's -1 sentinel; ledger #6).
    start_frame: Option<u32>,
    /// Total binary frames needed = abs(short_diff(current, prev)) / rot_per_frame,
    /// truncated. Ledger #3: when this would be 0, current() snaps.
    duration_frames: u16,
    /// Per-frame step in 16-bit facing units. Stored as (rot_byte << 8)
    /// (ledger #2). Zero means instant rotator (no interpolation).
    rot_per_frame: u16,
}

impl FacingClass {
    /// New FacingClass with given initial facing and ROT byte.
    pub fn new(initial: u16, rot_byte: u8) -> Self;

    /// Update the rate of turn. Ledger #1: clamps at 0x7F. Ledger #2: stores <<8.
    pub fn set_rot(&mut self, rot_byte: u8);

    /// Animated facing at the given binary frame. Pure function of state + frame.
    /// Ledger #3: snaps when step_size < 1. Ledger #4: wrap via signed short
    /// subtraction. Returns `current` when rot==0, when no rotation has been
    /// initiated, or when the rotation timer has expired.
    pub fn current(&self, binary_frame: u32) -> u16;

    /// Destination facing (where the rotation will end). Equivalent to the
    /// raw `current` field; named `destination` for callers that want the
    /// target rather than the animated value.
    pub fn destination(&self) -> u16 { self.current }

    /// Smooth setter — initiates a new rotation toward `new_target` from the
    /// current animated position. Ledger #5: snapshots `current(frame)` into
    /// `prev` before writing the new target. No-op when `new_target == current`.
    /// Returns true if anything changed.
    pub fn set(&mut self, new_target: u16, binary_frame: u32) -> bool;

    /// Snap setter — UpdateFacing semantics. Writes target to both current
    /// and prev, resets the timer. Used by spawn / locomotor takeoff /
    /// deploy paths that want no smoothing.
    pub fn snap(&mut self, new_target: u16, binary_frame: u32) -> bool;

    /// Whether a rotation is currently in progress. Ledger #18: equivalent
    /// to gamemd's CDTimerClass::Remaining check. Computed on demand.
    pub fn is_rotating(&self, binary_frame: u32) -> bool;
}
```

**Interpolation algorithm** (`current()` body, derived from research doc §2.1):

```rust
pub fn current(&self, binary_frame: u32) -> u16 {
    if self.rot_per_frame == 0 {
        return self.current;
    }
    let Some(start) = self.start_frame else {
        return self.current;
    };
    let elapsed = binary_frame.saturating_sub(start) as u32;
    if elapsed >= self.duration_frames as u32 {
        return self.current;
    }
    let remaining = self.duration_frames - elapsed as u16;

    // Ledger #4: signed short subtraction gives shortest signed delta.
    let diff: i16 = self.current.wrapping_sub(self.prev) as i16;

    // Ledger #3: step_size < 1 snaps.
    let step_size = diff.unsigned_abs() / self.rot_per_frame;
    if step_size < 1 {
        return self.current;
    }

    // animated = current - sign(diff) * rot_per_frame * remaining
    // (equivalent to: prev + sign(diff) * rot_per_frame * elapsed)
    let signed_step = diff.signum() as i32 * self.rot_per_frame as i32;
    let delta = signed_step * remaining as i32;
    (self.current as i32 - delta).rem_euclid(65536) as u16
}
```

**Set algorithm** (`set()` body, derived from research doc §2.4):

```rust
pub fn set(&mut self, new_target: u16, binary_frame: u32) -> bool {
    if new_target == self.current {
        return false;  // matches binary's `if (*this == *new_target) return 0`
    }
    if self.rot_per_frame > 0 {
        // Ledger #5: snapshot animated value into prev BEFORE writing new target.
        self.prev = self.current(binary_frame);
    } else {
        self.prev = self.current;
    }
    self.current = new_target;
    if self.rot_per_frame > 0 {
        let diff: i16 = self.current.wrapping_sub(self.prev) as i16;
        self.duration_frames = diff.unsigned_abs() / self.rot_per_frame;
        self.start_frame = Some(binary_frame);
    }
    true
}
```

#### `FireDecision` — `src/sim/combat/fire_decision.rs`

```rust
/// Outcome of a per-tick fire decision for one attacker. Subset of gamemd's
/// GetFireError codes covering only the variants that drive observably
/// different post-fire behavior. Ledger #12: code 5 (Generic) collapses
/// ~30 binary sub-reasons since they all map to "no fire this tick".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireDecision {
    Fire,           // code 0 — fire this tick
    Cooldown,       // code 3 — ROF timer hasn't expired
    Facing,         // code 2 — turret rotation in progress
    Range,          // code 6 — out of weapon range
    NoAmmo,         // code 1 — Ammo == 0
    CloakedTarget,  // code 8 — target cloaked, weapon doesn't see cloak
    ForceFire,      // code 9 — user-issued force-fire; needs CanFireAt
    Generic,        // code 5 — collapses ~30 sub-reasons, all "no fire"
}

impl FireDecision {
    /// Ledger #14: gattling spin-up gates on codes {0, 2, 3, 4}; else decay.
    /// Code 4 is unmapped in our enum; we approximate with Generic.
    pub fn drives_gattling_spinup(self) -> bool {
        matches!(self, Self::Fire | Self::Facing | Self::Cooldown | Self::Generic)
    }

    /// Whether this decision means "fire happens this tick" (only Fire and
    /// post-CanFireAt-passing ForceFire qualify).
    pub fn is_fire(self) -> bool {
        matches!(self, Self::Fire)
    }
}
```

#### GameEntity changes — `src/sim/game_entity.rs`

```rust
pub struct GameEntity {
    // ...
    /// Body facing (0–255, 0=N/64=E/128=S/192=W). Unchanged in this round.
    /// Future follow-up: replace with body_facing: FacingClass for ROT=3 smoothing.
    pub facing: u8,
    pub facing_target: Option<u8>,

    // OLD: pub turret_facing: Option<u16>,
    // NEW:
    /// Turret/barrel facing. None for turretless entities. Ledger #11:
    /// rot_byte = obj.turret_rot at spawn.
    pub barrel_facing: Option<FacingClass>,
    // ...
}

impl GameEntity {
    /// Convenience accessor for the legacy 8-bit form, used by sprite-frame
    /// selection and any caller that doesn't care about FacingClass internals.
    pub fn barrel_facing_u8(&self, binary_frame: u32) -> Option<u8> {
        self.barrel_facing.as_ref().map(|f| (f.current(binary_frame) >> 8) as u8)
    }
}
```

#### World time fields — `src/sim/world/mod.rs`

```rust
pub struct World {
    // ...existing...
    /// Total accumulated sim-tick milliseconds since world creation.
    /// Authoritative time source; binary_frame is derived from this.
    pub total_sim_ms: u64,
    /// Synthetic gamemd 15 Hz frame counter. Computed each tick as
    /// (total_sim_ms * 15 / 1000). Used by FacingClass methods.
    pub binary_frame: u32,
}
```

Computed at the top of `advance_tick`:

```rust
pub fn advance_tick(&mut self, ..., tick_ms: u32) {
    self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
    self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
    // ...rest of tick...
}
```

This formula is drift-free: every binary-frame boundary is exactly when `total_sim_ms` crosses a multiple of `1000/15 ≈ 66.67ms`. A 22ms tick sometimes advances `binary_frame` by 0 and sometimes by 1, with the long-run rate exactly 15 Hz.

### Interfaces / contracts

**`tick_turret_rotation` signature change:**
```rust
// OLD:
pub fn tick_turret_rotation(
    entities: &mut EntityStore,
    rules: &RuleSet,
    tick_ms: u32,
    interner: &StringInterner,
);

// NEW:
pub fn tick_turret_rotation(
    entities: &mut EntityStore,
    rules: &RuleSet,
    binary_frame: u32,
    interner: &StringInterner,
);
```

The function iterates entities with `barrel_facing.is_some()` and calls `barrel.set(desired, binary_frame)` where:
- `desired = facing_toward_lepton(self → target)` if entity has `attack_target`
- `desired = body_facing_to_turret(entity.facing)` otherwise (ledger #20)

`set()` is a no-op when the target hasn't changed since last call, so this is idempotent.

**Combat alignment check** (`combat/mod.rs:1494`):
```rust
// OLD:
if !is_turret_aligned_u16(turret_facing, desired) { continue; }

// NEW:
if barrel.current(world.binary_frame) != desired || barrel.is_rotating(world.binary_frame) {
    decision = FireDecision::Facing;
    // post-dispatch: gattling spin-up still drives because Facing ∈ spinup set
    continue;
}
```

**`AttackerSnapshot` change**:
```rust
// OLD:
pub turret_facing: Option<u16>,
// NEW:
pub barrel_facing: Option<FacingClass>,
```

`FacingClass` is `Copy` so the snapshot stays cheap.

### Data flow

Phase 5 of `advance_tick`, post-flip:

```
top of advance_tick:
   total_sim_ms += tick_ms
   binary_frame = (total_sim_ms * 15) / 1000

[Phase 1–4: commands, movement, vision, power]
   movement may write entity.facing (instant, body unchanged this round)

Phase 5:
   tick_capture_orders
   tick_order_intents_pre_combat
   tick_attack_pursuit
   tick_combat_with_fog                    ← reads barrel.current(binary_frame)
                                            ← drives FireDecision per attacker
                                            ← fires when decision == Fire
   tick_turret_rotation                    ← writes barrel.set(desired, binary_frame)
                                              for both attack_target and idle paths
```

Combat NEVER calls `barrel.set()` — only reads. This is cleaner than mirroring gamemd's redundant double-set (Fire_At_Target case 2 + Facing_Update Section A both call set; second is always a no-op). Behavioral output is identical because the rotation always starts the binary frame AFTER acquisition.

### Error handling

No new fallible operations. `FacingClass` methods are infallible:
- `current()` returns `self.current` for any out-of-range frame (saturates `binary_frame - start_frame`).
- `set()` and `snap()` return bool indicating change.
- `is_rotating()` returns false for unset/expired/zero-ROT cases.

`FireDecision` is a plain enum, no error states.

### Testing strategy

**Unit tests — `FacingClass` (~12 tests):**
- `set_rot` clamps at 0x7F and shifts <<8.
- ROT=0: instant rotator, `current()` always returns destination.
- Snap when `step_size < 1` (set ROT=0x7F (= 0x7F00 per frame), call `set(target = current+1)`, verify `current(frame)` returns new target immediately).
- Wrap shortest path: set destination 0x0010 from 0xFFE0, verify intermediate `current(frame)` values lie along +0x30 arc, not -0xFFD0.
- Set mid-rotation snapshots animated into prev: set, advance 5 frames, set new target, verify `prev == animated_at_frame_5`.
- `is_rotating` returns false after `start_frame + duration_frames`.
- TurretSpins (deferred-but-tested): `set_rot(100)`, repeatedly `set(target = ((current(f) >> 7 + 1) >> 1 + 8) << 8, f)`, advance 32 frames, verify barrel returned to start (full revolution).

**Unit tests — `FireDecision` (~3 tests):**
- `drives_gattling_spinup()` truth table matches ledger #14.
- `is_fire()` returns true only for `Fire`.

**Integration tests — combat (~5 tests):**
- 1-tick acquisition latency: spawn attacker + target, advance 1 tick, assert no fire (turret rotating). Advance until alignment completes, assert fire on the binary frame AFTER alignment.
- Mid-rotation retarget: target moves during rotation; barrel "snapshots and re-aims" smoothly, no facing snap-back.
- Slow-ROT vs fast-ROT alignment: ROT=2 turret takes proportionally more frames to align than ROT=10 turret — fixes the current 2048-flat-tolerance bug.
- FireDecision::Range: target out of range, decision = Range, no fire, no gattling spin-up.
- FireDecision::Facing: target in range but turret rotating, decision = Facing, no fire, gattling spins up (drives_gattling_spinup true).

**Binary ground-truth (separate follow-up, NOT blocking this design):**
- Trace 5–10 representative engagements from gamemd.exe (target offset → BarrelFacing.Current per binary frame) and assert byte-exact match. This is the ultimate parity verification for `facing_toward_lepton` axis convention and the FacingClass interpolation curve.

### Determinism considerations

- `FacingClass::current(frame)` is a pure function of state + frame; no hidden state.
- `set(target, frame)` reads `self.current(frame)` (deterministic) and writes deterministic state.
- `binary_frame` derived deterministically from `total_sim_ms` (integer division, no drift).
- All math is integer (i16, u16, u32 saturating). No new floats introduced.
- Iteration order in `tick_turret_rotation` follows `entities.keys_sorted()` (existing pattern).
- State hash extends to `FacingClass` fields and `total_sim_ms` / `binary_frame`. `FacingClass` derives `Hash`.

## Architectural Decisions

**Patterns followed:**
- Plain GameEntity fields (no ECS), serde-derived for snapshot.
- `tick_*` functions take `&mut EntityStore` and iterate `keys_sorted()` for determinism.
- Binary RE-driven primitive (`FacingClass`) lives in `sim/movement/`, alongside other locomotor state.
- Combat decision codes (`FireDecision`) live in `sim/combat/`, alongside the combat loop.
- New `World` time fields follow the existing pattern of `tick: u32` (already serialized).

**Patterns deviated from:**
- First sim "value type" with non-trivial method surface (~6 methods on `FacingClass`). Justified: faithfully reimplementing a binary primitive used in 3 places per entity. Six free functions threading `(prev, current, start, duration, rot)` would be worse.
- Combat introduces an enum-driven decision dispatch (`match decision { ... }`) where today it uses `continue`-based control flow. This is the only way to surface "why fire was blocked" to the gattling-stage and force-fire post-dispatch logic.

**Tech debt introduced:** None. Deferred items (#10, #15, #16, #17, #19) all have explicit prerequisites (body refactor, IFV deploy mission, harvester anim, Floating Disk render, explicit TurretAI scan) and are tracked in this design's ledger as known parity drift.

**Tech debt resolved:**
- `is_turret_aligned_u16` flat-tolerance bug — slow-ROT turrets considered aligned too early, fast-ROT too late.
- Per-tick step-clamp drift from binary's timer-based interpolation curve.
- Harvester turret-spin hack at `turret.rs:205` (`current_turret.wrapping_add(32768)`) — replaced with proper ledger #20 idle-return-to-body behavior; harvester turn-anim becomes a separate explicit follow-up (#16).
- Tick-order inversion (turret-before-combat): now matches binary.
- Atan2 axis convention discrepancy (`atan2(dx, -dy)` vs binary's `atan2(dy, -dx)`): verified and fixed.

## Alternatives Considered

1. **Patch step-clamp model in place** (rejected). Each binary behavior bolted on separately, no unifying source of truth, future-parity-rot risk. Net code volume similar to FacingClass introduction since the snap, wrap, and timer logic still has to live somewhere.
2. **FacingClass for turret only, body stays u8** (chosen for body scope). Defers body smoothing as a documented follow-up. The FacingClass type is body-ready; future body refactor is purely a migration of `entity.facing: u8` callers.
3. **Full 10-code FireDecision enum** (rejected). Codes 4 and 7 are unused in the binary; code 5's 30 sub-reasons all collapse to "no fire". Threading sub-reason complexity buys zero observable difference.
4. **Cache `is_rotating` on GameEntity** (rejected). Adds a dirty-flag invariant and serialized state for what becomes 2 inlined integer compares.
5. **Per-our-tick step conversion (no synthetic binary-frame counter)** (rejected for time base). Smoother at our tickrate but approximate parity: ~1% rotation-rate drift, snap-on-step<1 window varies with tickrate, and rotation duration drifts if `tick_ms` varies mid-rotation. The synthetic 15 Hz frame counter is the only way to get byte-exact-binary animated values at every binary-frame boundary, including the visible "stepped" rotation gamemd produces. Render layer can interpolate between binary frames visually if we want extra fluidity later — but as a render concern, never sim state.

## Deferred follow-ups (tracked, not in this round)

- **Body smoothing** (ledger #10): replace `entity.facing: u8` with `body_facing: FacingClass` (rot_byte=3 default). Migrates ~30 call sites. Independent refactor; FacingClass type already body-ready.
- **IFV / Mirage Tank auto-deploy** (ledger #15): err==0|2 + auto-deploy → mission 0x10. Blocked on the deploy-mission state machine for IFV / Mirage being implemented.
- **Harvester turn-anim** (ledger #16): 5-tick timer + 8-direction lookup. Blocked on harvester sprite-frame integration; current turret-spin hack at `turret.rs:205` removed by this round but harvester-specific turn anim is its own design.
- **TurretSpins / Floating Disk** (ledger #17): permaspin formula, only on `[DISK]`. Blocked on Floating Disk rendering. FacingClass design verified against the formula by test inclusion (ROT=100 + per-frame `set` with 2048-unit deltas).
- **Explicit TurretAI 8-cell scan** (ledger #19): every 8 binary frames, 1-cell-radius. Distinct from `tick_retaliation` (broader). Independent follow-up; current retaliation handles most of the player-visible behavior.
