# Bridges Tier 2 — Phase F Orchestrator Design

## Goal

Wire combat damage events end-to-end through the new bridge state machine + a
new overlay-direct walker, replacing the legacy single-shot `apply_damage` flow,
so that bridges are destructible in gamemd-parity-correct fashion: 4-path
dispatcher with per-path RNG gate, IonCannon-only retry, body/bridgehead
state-machine drivers (already shipped), `DestroyBridge_*` walker drivers (new),
and the full BlowUpBridge cascade (kill ground, DropIn deck, debris spawn, rim
refresh, zone rebuild).

## Architecture Context

**Current state on `dev` (2026-05-07):**

- `BridgeRuntimeState::body_cell_advance_state` and `bridgehead_advance_state`
  are shipped (Phase B+C, commits 20b8fdc and ce99b29) but DORMANT — no
  caller. They model `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`.
- Legacy `BridgeRuntimeState::apply_damage` ([bridge_state.rs:552](../../src/sim/bridge_state.rs#L552))
  is the active path: per-group HP that flips every cell to Destroyed when HP
  hits 0. No state machine, no warhead handling, no proper cascade.
- 3 combat sites push raw `BridgeDamageEvent { rx, ry, damage }` into the
  legacy flow ([combat/mod.rs:798, 1476, 1511](../../src/sim/combat/mod.rs)).
- Cascade lives in `Simulation::resolve_bridge_state_changes`
  ([world/mod.rs:773](../../src/sim/world/mod.rs#L773)) — currently
  spawns explosions and despawns/snaps on-bridge entities. Has parity bugs:
  despawns deck entities when destination not walkable (should DropIn);
  spawns wrong debris shape (should be 50% MetallicDebris + always
  BridgeExplosion).

**Verification (this session):** dispatch architecture in `Apply_area_damage @
0x00489280` is materially different from the existing call-chain summary.
Verified live and documented at
`ra2-rust-game-docs/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`. Four
sequential paths, each with independent RNG gate; direct-overlay paths NOT
mutually exclusive with state-machine paths at the dispatcher level; Finding
3 reframes the shipped state-machine drivers as **late-stage progression** on
already-transitioned overlays — primary damage on a fresh bridge goes through
the walker (`DestroyBridge_*`).

## Impact Analysis

**Files modified:**
- `src/sim/bridge_state.rs` → `src/sim/bridge_state/mod.rs` (directory split)
- `src/sim/bridge_state/walker.rs` (NEW)
- `src/sim/world/mod.rs` (orchestrator + cascade dispatch removed; thin
  forward to `bridge_orchestrator`)
- `src/sim/world/bridge_orchestrator.rs` (NEW)
- `src/sim/combat/mod.rs` (extend `BridgeDamageEvent`, update 3 emit sites)
- `src/rules/ruleset.rs` (`resolve_bridge_warheads`, accessors)
- `src/app/app_init_helpers.rs` (call `resolve_bridge_warheads` at sim init,
  pre-intern `metallic_debris`)
- `src/sim/snapshot/world_hash.rs` (drop `group_hitpoints` /
  `strength_per_group` if hashed)

**Risk areas:**
- Determinism: 4 RNG draws per event in fixed dispatch order. Wrong order →
  desync. Mitigated by snapshot/hash regression tests + RNG draw-count parity
  test.
- DropIn correction changes existing entity-survival semantics on bridge
  collapse. Existing tests asserting despawn-when-unwalkable will need
  updating to assert snap-and-survive.
- Cascade ordering: ground kill must precede deck DropIn must precede debris.
  Wrong order produces visible glitches (entities surviving on cell where
  ground unit just died, etc).
- `update_adjacent_bridges` rim refresh — may be active or stub depending on
  whether renderer queries neighbor `damage_state`. Implementation pass
  resolves.
- Multiple tests (existing 6 world_tests bridge cases) require migration to
  new `BridgeDamageEvent` shape.

**Dependencies:** Phase B+C (state-machine drivers, anchor walker, span
walker) all shipped. Tasks 13.5 / 15.5 (overlay-write branch of UpdateRamp)
remain deferred — does NOT block Phase F since the walker covers the raw-
overlay path independently.

## Chosen Approach

**Approach 2 — World layer (`world/bridge_orchestrator.rs`) owns the 4-path
dispatcher + cascade consumers.** State-machine drivers and walker drivers
stay as methods on `BridgeRuntimeState`.

Rationale:
1. Cascade consumers (kill ground, DropIn, debris, rim, zones) are already
   world-layer code. Putting orchestrator there too means one function owns
   the whole event-to-cascade flow — no `Vec<StateOutcome>` carrier crossing
   layers.
2. Pattern consistency: `apply_wall_damage_events` already does
   cell-event-with-cascade in the world layer. Same shape.
3. Determinism boundary: 4 path-gate RNG draws + cascade RNG draws all live
   in one file/function — easier to read, test, audit for parity.

Rejected alternatives in §"Alternatives Considered" below.

## Tiny-Detail Ledger

Each item must be preserved in implementation; cited source.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| 1 | Outer gate: `SpecialFlags & 0x8000` (DestroyableBridges) | `[GHIDRA 0x00489280]` + verification doc §1 | orchestrator early-return on `!bridge_state.is_destroyable()` |
| 2 | Outer gate: `warhead+0x144` (Wall=) | `[GHIDRA 0x00489280]` + HIGH §11.6 | combat-side gate (3 emit sites already check `warhead.wall`) |
| 3 | 4 sequential paths in fixed order: HighSM → LowSM → LowDirect → HighDirect | verification doc §1 + `[GHIDRA 0x00489280 LAB_00489f27..LAB_0048a214]` | orchestrator `for path in [HighSM, LowSM, LowDirect, HighDirect]` |
| 4 | Per-path independent RNG gate: `RandomRanged(1, BridgeStrength) < damage` | `[GHIDRA 0x00489280]` | orchestrator inner: `rng.next_range_u32_inclusive(1, bridge_strength)` |
| 5 | IonCannon (`Rules+0xFF0`) bypasses RNG gate | `[GHIDRA 0x00489280]` | `if !ctx.is_ion_cannon { rng draw + gate }` |
| 6 | Retry loop: 3 retries IF IonCannon AND state-machine path. Direct-overlay paths single-shot | verification doc Finding 1 | `max_attempts = if ctx.is_ion_cannon && path.is_state_machine() { 4 } else { 1 }` |
| 7 | Z-height range gate: `[level-2, level+1]` tile-step units of cell.level. State-machine paths only, gated by `flags & 0x100` | verification doc Finding 2 | `path_matches_cell` for state-machine paths checks impact_z range |
| 8 | `ApplyDamageToCell` internal dispatch: overlay-direct first, state-machine second | verification doc Finding 3 | orchestrator `path_matches_cell(HighSM)` requires `overlay_byte ∉ [0xCD..0xE6]` |
| 9 | State-machine HIGH match: `flags & 0x100` + anchor.+0x44 ∈ {0x18, 0x19} OR bridgehead tile-class match | `[GHIDRA 0x00489f27..0x00489f77]` | `path_matches_cell(HighSM)` |
| 10 | State-machine LOW match: anchor.+0x44 ∈ {0xED, 0xEE} OR low-bridgehead tile-class | `[GHIDRA 0x0048a0a5]` | `path_matches_cell(LowSM)` |
| 11 | Direct-overlay LOW match: `cell.OverlayIndex ∈ [0x4A..0x63]` | `[GHIDRA 0x0048a214]` | `path_matches_cell(LowDirect)` |
| 12 | Direct-overlay HIGH match: `cell.OverlayIndex ∈ [0xCD..0xE6]` | `[GHIDRA 0x0048a214]` | `path_matches_cell(HighDirect)` |
| 13 | Body-cell driver state machine (Healthy/Damaged/PartialA/PartialB/Destroyed) | shipped, [bridge_state.rs:610](../../src/sim/bridge_state.rs#L610) | already in driver |
| 14 | Bridgehead-cell driver (Healthy → Damaged absorbed; Damaged → Destroyed with 3-cell row) | shipped, [bridge_state.rs:790](../../src/sim/bridge_state.rs#L790) | already in driver |
| 15 | Walker NS/EW classification by overlay byte | `[GHIDRA 0x0057CCF0]` | `destroy_bridge_high` dispatches to NS/EW walker |
| 16 | Walker iterates segments along bridge axis with overlay transitions | HIGH §4 + `[GHIDRA 0x0057CF60 / 0x0057D530]` | `destroy_bridge_walker_ns_high` / `_ew_high` |
| 17 | Walker emits BlowUpBridge per walked cell | HIGH §11.4 | walker fills `set_bridge_direction.actions` with `BlowUpBridge` |
| 18 | BlowUpBridge step 1: kill `+0xE4` ground occupants via `ReceiveDamage(damage=0, C4Warhead, force_kill=1)` | HIGH §11.4 | cascade `kill_ground_occupants_at(rx, ry, c4_id)` |
| 19 | BlowUpBridge step 2: `+0xE8` bridge-deck → DropIn (snap to ground, clear OnBridge, NO damage, NO despawn) | HIGH §12.7 | cascade `drop_in_bridge_deck_entities` (CORRECTION) |
| 20 | NO drown / fall damage / EVA / "BridgeDestroyed" sound | HIGH §12.9 | cascade does NOT despawn deck entities |
| 21 | BlowUpBridge step 4 debris: 95% outer gate per cell | HIGH §11.4 | `spawn_bridge_debris`: `rng.next_range_u32(20) != 0` |
| 22 | Debris: 2 jitter draws (consumed for RNG order parity) | HIGH §11.4 | `_jitter_x = rng.next_range_u32(0xFFFF); _jitter_y = ...` |
| 23 | Debris: 50% MetallicDebris (no delay) gated by `voxel_max > 0` | HIGH §11.4 | `if rng.next_range_u32(2) == 0 && voxel_max > 0 && metallic_count > 0 { spawn }` |
| 24 | Debris: 1 always BridgeExplosion (delay 1-5 frames) | HIGH §11.4 | `delay_frames = rng.next_range_u32_inclusive(1, 5)` |
| 25 | UpdateAdjacentBridges_High × 2 perpendiculars (rim refresh) | HIGH §11.9 | cascade `update_adjacent_bridges` (stub or active per renderer query) |
| 26 | NotifyBridgeSpanCollapse / TriggerEvent 31 (no-op on skirmish) | HIGH §11.3 + §12.6 | cascade `notify_bridge_span_collapse` (stub hook) |
| 27 | InvalidateBridgeZones + UpdateBridgeZonesHelper on collapse | HIGH §12.8 | cascade `if zones_dirty { rebuild_zone_grid }` |
| 28 | NO eager pathfinding invalidation; stale paths fail emergently | HIGH §12.8 | already correct (`is_bridge_walkable` checks `damage_state`) |
| 29 | Sub-tick order: ground kill → deck DropIn → debris → rim → trigger 31 → zone rebuild | HIGH §11.4 + §12.8 | cascade ordering in orchestrator |
| 30 | Per-cell HP, NOT per-group | shipped drivers operate on cells | delete `group_hitpoints` + `strength_per_group` fields |
| 31 | RandomRanged is the lockstep RNG (Westwood mask-and-retry, inclusive) | HIGH §12.10 | `SimRng::next_range_u32_inclusive` (already shipped) |

## Design

### Components

```
src/sim/combat/mod.rs
  ├─ BridgeDamageEvent (extended: warhead_ref, is_ion_cannon, impact_z)
  └─ 3 emit sites updated

src/rules/ruleset.rs
  ├─ ion_cannon_warhead_id: Option<InternedId> (NEW)
  ├─ c4_warhead_id: Option<InternedId>         (NEW)
  ├─ resolve_bridge_warheads()                 (NEW)
  ├─ ion_cannon_warhead_id() accessor          (NEW)
  └─ c4_warhead_id() accessor                  (NEW)

src/sim/bridge_state/                          (DIRECTORY — was bridge_state.rs)
  ├─ mod.rs
  │   ├─ DELETED: apply_damage(), group_hitpoints, strength_per_group
  │   ├─ ADDED:   bridge_strength() getter, is_destroyable() getter,
  │   │           path_matches_cell(), DispatchPath enum
  │   └─ KEPT:    body_cell_advance_state, bridgehead_advance_state,
  │               anchor_span ops, cell ops
  └─ walker.rs                                 (NEW)
      ├─ destroy_bridge_high(rx, ry, terrain) -> StateOutcome
      ├─ destroy_bridge_low(rx, ry, terrain)  -> StateOutcome
      ├─ destroy_bridge_walker_ns_high()      (axis walker)
      ├─ destroy_bridge_walker_ew_high()      (axis walker)
      └─ apply_bridge_destruction_*_high()    (per-cell helper)

src/sim/world/
  ├─ mod.rs
  │   ├─ Simulation: + metallic_debris: Vec<InternedId>
  │   ├─ DELETED: spawn_bridge_explosions, apply_bridge_damage_events,
  │   │           resolve_bridge_state_changes
  │   └─ Single call site forwards to bridge_orchestrator
  └─ bridge_orchestrator.rs                    (NEW)
      ├─ apply_bridge_damage_events()         (entry — replaces both old fns)
      ├─ kill_ground_occupants_at()
      ├─ drop_in_bridge_deck_entities()       (CORRECTION — no despawn)
      ├─ spawn_bridge_debris()                (CORRECTION — 50% MD + always BE)
      ├─ update_adjacent_bridges()
      └─ notify_bridge_span_collapse()        (no-op stub hook)
```

### Interfaces / Contracts

**Combat boundary** (3 emit sites in `combat/mod.rs`):
```rust
let wh_id = interner.intern(&warhead.id);
bridge_damage_events.push(BridgeDamageEvent {
    rx: target_rx, ry: target_ry, damage: damage_u16,
    warhead_ref: wh_id,
    is_ion_cannon: wh_id == rules.ion_cannon_warhead_id(),
    impact_z: target_z,
});
```

**Orchestrator** (`world/bridge_orchestrator.rs`):
```rust
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> Vec<u64>  // returns despawned entity IDs (typically empty after DropIn correction)
```

**State-machine drivers** (already shipped):
```rust
impl BridgeRuntimeState {
    pub fn body_cell_advance_state(
        &mut self, rx: u16, ry: u16, is_high: bool,
    ) -> StateOutcome;

    pub fn bridgehead_advance_state(
        &mut self, rx: u16, ry: u16, is_high: bool,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome;
}
```

**New walker drivers**:
```rust
impl BridgeRuntimeState {
    pub fn destroy_bridge_high(
        &mut self, rx: u16, ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome;

    pub fn destroy_bridge_low(
        &mut self, rx: u16, ry: u16,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome;
}
```

**New classifier**:
```rust
impl BridgeRuntimeState {
    pub(crate) fn path_matches_cell(
        &self, path: DispatchPath,
        rx: u16, ry: u16,
        ctx: &BridgeDamageContext,
        terrain: &ResolvedTerrainGrid,
    ) -> bool;
}
```

**Invariants**:
- `path_matches_cell(HighStateMachine, ...)` returns `false` when
  `overlay_byte ∈ [0xCD..0xE6]` (raw body overlay routes to walker per
  Finding 3).
- `path_matches_cell(state_machine_paths, ...)` checks Z-height range gate
  per Finding 2.
- Drivers return `StateOutcome::NoChange` for any guard violation; no errors.

### Data Flow

State-machine collapse path (transitioned-overlay anchor cell):
```
combat emit → orchestrator
  ├─ outer gate (destroyable_flag)
  ├─ for path in [HighSM, LowSM, LowDirect, HighDirect]:
  │   ├─ path_matches_cell? → continue if no
  │   ├─ if !is_ion_cannon: RNG draw + gate
  │   ├─ retries = 4 if (is_ion_cannon && path.is_state_machine()) else 1
  │   └─ for retry in 0..retries:
  │       ├─ outcome = dispatch_driver(path, rx, ry, terrain)
  │       └─ if outcome != NoChange: push, break
  ├─ aggregate Collapsed outcomes:
  │   ├─ destroyed_set ∪= outcome.destroyed_cells ∪ BlowUpBridge cells
  │   ├─ blow_up_cells ∪= cells with CellAction::BlowUpBridge
  │   ├─ rim_cells ∪= outcome.adjacent_bridges_dirty
  │   └─ any_zones_dirty |= outcome.zones_dirty
  ├─ Step 1: kill_ground_occupants_at(blow_up_cells)
  ├─ Step 2: drop_in_bridge_deck_entities(destroyed_set)  // no despawn
  ├─ Step 3: spawn_bridge_debris(destroyed_set)
  ├─ Step 4: update_adjacent_bridges(rim_cells)
  ├─ Step 5: notify_bridge_span_collapse(destroyed_set)
  └─ Step 6: if any_zones_dirty: sim.rebuild_zone_grid()
```

Walker collapse path (raw-overlay body cell): identical orchestrator flow,
but `path_matches_cell(HighStateMachine)` returns false (overlay still raw),
`path_matches_cell(HighDirect)` matches, walker driver fires, produces
`Collapsed` with the full walked span in `destroyed_cells` and BlowUpBridge
actions for each. Cascade is uniform.

### Error Handling

No `Result` types. Defensive `NoChange` for guard violations in drivers.
`bridge_state.is_none()` or `terrain.is_none()` early-return empty Vec.
`RuleSet::ion_cannon_warhead_id()` panics if `resolve_bridge_warheads()` not
called — sim-init contract violation.

### Testing Strategy

- **Unit (drivers):** `path_matches_cell` × 4 paths × match/non-match ×
  Z-gate-pass/fail (16 cases); walker NS/EW classification; walker overlay
  transitions; walker endpoint detection.
- **Unit (orchestrator):** 4-path dispatcher with handcrafted cells; IonCannon
  retry only on state-machine paths; non-IonCannon RNG gate; multi-path
  mutual exclusion via overlay-byte invariant.
- **Integration (`world_tests.rs`):** migrate existing 6 fixtures; new tests
  for ground-occupant kill, DropIn-no-despawn correction, debris RNG draw
  count, walker full-span collapse.
- **Determinism:** snapshot round-trip; world hash regression; explicit RNG
  draw-count parity test (assert exactly N draws per cascade per cell).

### Determinism Considerations

- 4 path-gate RNG draws per event in fixed dispatch order (HighSM → LowSM →
  LowDirect → HighDirect). Each path that matches draws once.
- IonCannon retry does NOT draw additional RNG.
- Cascade RNG draws happen in `BTreeSet`-sorted destroyed-cell order. Per
  cell: 5–7 draws (95% gate + 2 jitter + metallic 50% + optional metallic
  slot + explosion delay + explosion slot).
- All math integer — `u16`/`i32`/`u32`. No `f32`/`f64`.
- Single sim-tick callsite at [world/mod.rs:1337](../../src/sim/world/mod.rs#L1337);
  no inter-tick state crossing.

## Architectural Decisions

**Patterns followed:** world-layer orchestrator (matches
`apply_wall_damage_events`); driver methods on `BridgeRuntimeState`;
deterministic RNG via `SimRng`; module split when files exceed ~600 LOC.

**Patterns deviated:** master plan's `apply_state_outcome` helper dissolved
(driver mutations apply directly via `update_ramp_perpendicular`; cascade
processes `StateOutcome::Collapsed` payload). `BridgeStateChange` deleted —
`StateOutcome` flows end-to-end. New scope: walker drivers (was stubbed in
master plan).

**Tech debt introduced:**
- `update_adjacent_bridges` stub-or-active pending renderer-layer query
  check — implementation pass resolves.
- `notify_bridge_span_collapse` no-op stub for trigger system — wired as a
  hook for future campaign support.
- Both stubs documented in source + cited to HIGH §11.3 / §11.9.

## Alternatives Considered

**Approach 1 — `BridgeRuntimeState` owns the orchestrator.** Rejected because
cascade consumers are already in world layer; putting orchestrator there too
eliminates the `Vec<StateOutcome>` carrier crossing layers, and matches the
existing `apply_wall_damage_events` pattern.

**Approach 3 — free function in `sim::bridge_state`.** Rejected as a
hybrid that introduces a new pattern (free function orchestrator) for
marginal benefit over Approach 1.

**Phase F-minimal scope (skip rim refresh + trigger 31 hooks).** Rejected
during brainstorm — cuts were convenience-disguised parity drift; rim
refresh has unverified renderer-impact, trigger 31 is verified-no-op-on-
skirmish-only-but-hook-cost-is-trivial. Both stay in scope.

**Reuse legacy `apply_damage` + `BridgeStateChange` carrier.** Rejected —
incompatible with state-machine driver semantics; legacy path uses per-group
HP which has no anchor-span concept.
