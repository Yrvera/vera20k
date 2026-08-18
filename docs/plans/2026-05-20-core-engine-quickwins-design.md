# Core Engine Quick-Wins — Phantom Power Damage Removal + Dying-Window Dock Leak

## Goal
Remove two HIGH-priority player-visible disparities surfaced by the 2026-05-20 disparity-scan swarm: (1) phantom degradation damage that Rust applies but gamemd does not, and (2) the dock-reservation leak during a docked miner's death animation.

## Architecture Context

### Power system
`src/sim/power_system.rs` runs once per tick from the sim loop. Per-player `PowerState` tracks `total_output`, `total_drain`, `is_low_power`, `power_blackout_remaining` (spy/ForceShield), and `theoretical_total_power` (used by the sidebar power-bar fill curve). `recalculate_power_for_owner` sums Power= contributions (positive=health-scaled, negative=flat drain). The second-pass loop at L162-193 accumulates `degradation_accum_ms` per owner and, when it crosses `damage_delay_minutes * 60_000` ms, calls `apply_degradation_damage` to subtract 1 HP from every `Powered=yes` consuming building owned by that player.

### Dock reservations
`DockReservations` (defined at [miner_dock.rs:18](../../src/sim/miner/miner_dock.rs#L18)) is a `BTreeMap<refinery_sid, miner_sid>` for occupants + per-refinery FIFO queues. Used by:
- **Miner ↔ refinery**: `sim.production.dock_reservations`, ticked from `tick_miners` at [miner_system.rs:117](../../src/sim/miner/miner_system.rs#L117).
- **Unit ↔ repair depot**: `sim.production.depot_dock_reservations`, ticked from `tick_building_docks` at [building_dock.rs:76](../../src/sim/docking/building_dock.rs#L76).
- **Aircraft ↔ airfield pad**: separate `airfield_docks`, has its own `cleanup_dead`, not in scope (already separate codepath).

Both cleanup_dead callsites build `alive_sids` from `sim.entities.values()`/`keys_sorted()`. A miner with `entity.dying = true` but not yet despawned is still in the entity store → counts as alive → its dock reservation is not released until despawn (~6-10 ticks later when the death animation completes).

### gamemd reference (RADIO_CLASS_PROTOCOL §8.5)
`TechnoClass::Limbo_Tail_CallConceal @ 0x0065AA80` is called when an entity enters limbo (death, ChangeOwner, passenger-board). It calls `Broadcast_Radio_ToAll(0x03)` which sends BREAK to every radio contact. gamemd's refinery slot is therefore freed **on the same tick the miner enters limbo**, not when the death animation finishes.

## Impact Analysis

| File | Change |
|---|---|
| [src/sim/power_system.rs](../../src/sim/power_system.rs) | Remove `apply_degradation_damage` fn (L198-224); remove the second-pass damage block (L162-193); remove `degradation_accum_ms` field from `PowerState`; remove the `if !state.is_low_power { state.degradation_accum_ms = 0; }` reset at L157-159 |
| [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | Drop `degradation_accum_ms` from the per-player power state hash mixing |
| [src/sim/miner/miner_system.rs:116](../../src/sim/miner/miner_system.rs#L116) | Filter dying entities out of `alive_sids` |
| [src/sim/docking/building_dock.rs:75](../../src/sim/docking/building_dock.rs#L75) | Filter dying entities out of `alive` set |
| `power_system_tests.rs` / `miner_tests.rs` | Add regression tests (see Testing) |

**Touched code surface:** ~40 lines deleted, ~4 lines added across 4 files. No new modules, no new types, no new dependencies.

**Determinism:** state hash format changes (one `u32` field removed from PowerState hashing). Any pre-fix replay touching low-power will diverge. Acceptable — pre-fix behavior was incorrect.

**Backwards compatibility:** `damage_delay_minutes` field on `RulesGeneral` stays parsed (still in [General] DamageDelay= in rules.ini) so modder INIs don't break. The field becomes unused.

**INI parser:** unchanged. `DamageDelay=1` is still parsed; just nothing reads it.

**Risk areas:**
- The second-pass loop also resets `degradation_accum_ms = 0` on `!is_low_power` transitions. Removing the field makes that reset moot — verify no other code reads it. (grep confirmed: only 3 files reference it, all being changed.)
- Save-game format: `PowerState` is `serde::Serialize` — removing the field breaks deserialization of old saves. Use `#[serde(default)]` on the new struct, or accept the break. (Recommend: accept; saves from yesterday don't survive this kind of churn anyway.)
- World-hash change: any replay-fixture tests using a low-power scenario will need their golden hashes regenerated.

## Chosen Approach

### POWER G1 — A1: Surgical delete
Remove `apply_degradation_damage`, its caller block, and the `degradation_accum_ms` field. Keep the INI parser intact. Why this over alternatives:
- **vs. "also remove DamageDelay= INI parser"** (A2): orthogonal cleanup; could break custom INIs; not asked for. A1's narrower scope is correct here.

### RADIO G1 — B1: Pull model (filter dying from alive_sids)
Change the two cleanup_dead callsites to exclude entities with `entity.dying == true` when building the alive set. Why this over alternatives:
- **vs. push model** (B2: hook every `entity.dying = true` site to also call `dock_reservations.cancel`): `entity.dying = true` is set at 15+ sites across combat, aircraft, superweapons, production, world_orders, bridge_orchestrator, genetic_converter, etc. A push model creates tight coupling between every death trigger and the dock-reservation module, and future death sites would silently miss the cleanup.
- B1's 1-tick lag (cleanup fires next tick, not same tick) is below the observable threshold. gamemd's "same-tick BREAK broadcast" at 15 Hz is itself a 1-tick window from the queued-miner's perspective.
- B1 matches the existing pull-model `cleanup_dead` pattern already used by `airfield_docks`, `depot_dock_reservations`, and `dock_reservations`. No new architectural pattern introduced.

## Tiny-Detail Ledger

| # | Detail | Source | Where preserved in design |
|---|---|---|---|
| L1 | HouseClass+0x578C/+0x5794 (DamageDelay timer) written-but-never-read in binary | POWER_SYSTEM §7 L927-932 | Removing the Rust analog (`degradation_accum_ms`) closes the parity gap |
| L2 | Powered=yes building during low-power: IsOperational=false, zero HP damage | POWER_SYSTEM §7 | `is_building_powered()` returns false during low power (unchanged); damage application removed |
| L3 | `[General] DamageDelay=1` parsed but never consumed | POWER_SYSTEM §7 | INI parser unchanged; field becomes dead in Rust same as gamemd |
| L4 | SpyPowerSabotage blackout timer (separate live mechanism) | POWER_SYSTEM | `power_blackout_remaining` untouched — explicitly preserved |
| L5 | ConditionYellow damage-fire overlays tied to HP, not power | POWER_SYSTEM | Removing degradation damage means buildings stop randomly hitting ConditionYellow during low power — matches gamemd |
| L6 | `Limbo_Tail_CallConceal @ 0x0065AA80` broadcasts BREAK on entity entering limbo (death/ChangeOwner/passenger-board) | RADIO §8.5 L617-630, L773-775 | B1 releases dock when dying-set (≤1 tick lag), close-enough to gamemd same-tick semantics |
| L7 | Refinery accepts next miner within ~1 frame of previous death — no death-anim wait | inferred from L6 | B1: queued miner promoted on next tick (≤1 tick delay vs gamemd's ≤1 frame) |
| L8 | Fixture: HARV at dock HP=1, lethal V3 strike, second HARV queued. Expected: ≤1 tick promotion. Today (pre-fix): ~6-10 ticks. Post-fix: ≤1 tick. | derived from L6 | New regression test (see Testing) |
| L9 | Cleanup applies to both occupant slot AND queue entries | code: miner_dock.rs L111-115 | Filtering `alive_sids` to exclude dying covers both — existing `cleanup_dead` already iterates both |
| L10 | RADIO G2 false positive: refinery has no toggled "miner docked" anim; visual "unloading" is `display_type_override` on miner entity, dies with miner | code: game_entity.rs L236; doc RADIO §8.5 silent-swallow note L274 | Explicitly out of scope; documented to prevent re-discovery |

## Design

### POWER G1 components

**`PowerState` struct (power_system.rs)**

```rust
pub struct PowerState {
    pub total_output: i32,
    pub total_drain: i32,
    pub is_low_power: bool,
    #[serde(rename = "spy_blackout_remaining")]
    pub power_blackout_remaining: u32,
    // REMOVED: pub degradation_accum_ms: u32,
    pub was_low_power: bool,
    pub theoretical_total_power: i32,
}
```

**`tick_power_system` second pass (power_system.rs L155-193)** — fully deleted. The `if !state.is_low_power { state.degradation_accum_ms = 0; }` reset at L157-159 also goes (no longer meaningful).

**`apply_degradation_damage` fn (L198-224)** — deleted entirely.

**`world_hash.rs`** — find where `degradation_accum_ms` mixes into the state hash, drop that line.

### RADIO G1 components

**`miner_system.rs` L115-117**

Before:
```rust
let alive_sids: BTreeSet<u64> = sim.entities.values().map(|e| e.stable_id).collect();
sim.production.dock_reservations.cleanup_dead(&alive_sids);
```

After:
```rust
let alive_sids: BTreeSet<u64> = sim
    .entities
    .values()
    .filter(|e| !e.dying)
    .map(|e| e.stable_id)
    .collect();
sim.production.dock_reservations.cleanup_dead(&alive_sids);
```

**`building_dock.rs` L75-76**

Before:
```rust
let alive: BTreeSet<u64> = sim.entities.keys_sorted().iter().copied().collect();
sim.production.depot_dock_reservations.cleanup_dead(&alive);
```

After:
```rust
let alive: BTreeSet<u64> = sim
    .entities
    .values()
    .filter(|e| !e.dying)
    .map(|e| e.stable_id)
    .collect();
sim.production.depot_dock_reservations.cleanup_dead(&alive);
```

### Data flow (no change)
Both fixes preserve existing dataflow. POWER G1 removes a side effect; RADIO G1 narrows the input set to an existing function.

### Error handling
None to add. Both fixes are pure deletes / filter-narrowing — no new failure modes.

### Testing Strategy

**POWER G1 regression test** (in `power_system_tests.rs`):
- Setup: one player, one Power=-100 Powered=yes building, no power plants → low power.
- Tick for 10 simulated minutes worth of ticks.
- Assert: building HP is still at max_hp (no degradation damage applied).

**RADIO G1 regression test** (in `miner_tests.rs`):
- Setup: refinery + two miners. Miner A reserves the dock and enters Phase=Unloading. Miner B enters the queue.
- Set `miner_a.dying = true` (simulate combat death).
- Tick once.
- Assert: `dock_reservations.occupied[refinery_sid] == miner_b.stable_id` (promoted).
- Assert: miner B's phase advances to Approach/Linked on the next miner tick.

**Depot dock test** (in `building_dock`'s test module if present, else add):
- Same shape as above with a repair depot instead of a refinery.

**Determinism test impact**: any existing replay test that triggers low power needs golden-hash regeneration. Sweep `*replay*` and `*hash*` test files; expect minimal hits given how niche sustained-low-power-replay scenarios are.

## Architectural Decisions

- **Follows existing patterns:** B1 reuses the pull-model `cleanup_dead` that's already canonical in this codebase (aircraft pads, depot docks, miner docks).
- **No new tech debt:** A1 is pure deletion; B1 is a filter narrowing.
- **No new dependencies, no new modules, no new traits.**
- **Determinism preserved:** state hash format changes (one field removed); replays from before this fix diverge but new replays are deterministic.
- **gamemd alignment:** both fixes move Rust closer to gamemd's observable output (A1 zero damage matches gamemd's never-read DamageDelay; B1 ≤1-tick promotion approximates gamemd's same-tick BREAK).

## Alternatives Considered

- **A2 (remove DamageDelay= INI parser):** Rejected — orthogonal cleanup, risks modder INI breakage, not asked for.
- **B2 (push model: hook every dying-set site):** Rejected — would couple 15+ death triggers to dock_reservations, doesn't match existing pull-model architecture, easy to miss new sites in future. The 1-tick lag in B1 is well below observable threshold.
- **B3 (release dock inside `despawn_entity` directly):** Rejected as a hybrid — still has the death-animation delay since despawn fires after death anim. Worse than both B1 and B2.

## Out of Scope

- **RADIO G2 (refinery anim reset):** confirmed false positive. The "unloading" visual is on the miner (`display_type_override`), not the refinery; gamemd's BREAK silently swallows building anim resets per RADIO doc L274.
- **DamageDelay= INI parser:** stays as-is for modder compat.
- **Other disparity-scan findings:** POWER G2/G3/G4/G5/G6/G7, RADIO G3/G4/G5/G6, FACING/TIMER G1/G2, ILOCOMOTION G1/G2, COORDINATE G1/G2 — all deferred to separate brainstorms.
- **ChangeOwner / mind-control dock release:** distinct scenario (entity alive but ownership changed). Not in the dying-window class. Separate follow-up if needed.

## Verification

After implementation:
- All existing power_system_tests pass (excluding any explicit degradation-damage tests, which delete).
- New regression tests pass.
- `cargo check && cargo test --workspace` clean.
- Manual in-game check: build a power plant, build 4 Power=-100 buildings, watch HP stay full during low power.
- Manual in-game check: queue two miners on one refinery, kill the docked miner with a V3, verify second miner gets promoted immediately (no death-anim wait).
