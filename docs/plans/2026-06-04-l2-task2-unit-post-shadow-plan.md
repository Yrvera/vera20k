# L2 Task 2 — Stand up the shadow `unit_post` host (Fire + Facing)

> **For Claude:** Execute task-by-task. Each task is self-contained. This slice
> adds a **read-only shadow** per-object Unit Fire+Facing host. It does **NOT**
> flip authority, does **NOT** bump `SNAPSHOT_VERSION`, and must change **zero**
> hashed state. The gate on every task: `cargo test -p vera20k combat` stays at
> **143 passed; 0 failed** and the per-tick `state_hash` is unmoved. In a debug
> build the new shadow asserts must not fire. If any assert fires or a hash moves,
> STOP — the shadow caught a real divergence; diagnose it, don't silence it.

**Goal:** Build `unit_post(sim, id, …)` — the per-object UnitClass Fire→Facing
step — as a standalone host wired in as a **debug-only shadow** inside Phase 5,
proving its fire/facing/cooldown output matches the legacy `tick_combat_with_fog`
+ `tick_turret_rotation` sweeps for Unit-category entities, with no authority flip.

**Architecture:** New `src/sim/world/unit_post.rs`. The host reuses the
already-landed `resolve_attacker_fire` (Task 1) for fire and the `turret.rs`
facing helpers for facing. It runs in Phase 5 (where combat+turret already run),
NOT in the early `object_ai_stage` (that stays a no-op; relocating fire ahead of
movement is the future S4 slice, explicitly out of scope here per invariant #3).

**Design Doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` (Slice L2
§4 Task 2 / 2a, §5, §7) + gating verdict
`docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md` (settled:
deferred-projectile → keep batched P4/P6 — no damage-apply threading needed here).

---

## Grounding Summary

- **The design doc (2026-06-02) is stale in three load-bearing ways** (verified
  against the live tree + `git log` this session — re-scoped with user sign-off):
  1. **The shell scaffold it assumed absent now EXISTS.** `src/sim/world/techno_ai.rs`
     has `object_ai_stage()` wired into `advance_tick` at `world/mod.rs:1788` —
     but it runs **early, before Phase-1 movement**, as an S0 **no-op** with an
     empty `EntityCategory::Unit => {}` arm in `techno_ai_shell(sim, id, category)`
     (`techno_ai.rs:103`). `substrate.rs`/`logic_vector.rs` own object order. The
     S1 shadow `unit_ai_shadow_step` (`techno_ai.rs:162`) only proves
     dispatch-before-locomotor for movement, not fire/facing.
     **Decision (user-confirmed): L2 does NOT touch `object_ai_stage`.** Wiring
     fire+facing into the early stage would move Unit firing ahead of movement —
     a per-tick firing-position reorder for every moving unit — which is the
     future S4 relocation, not L2. L2 keeps fire+facing in Phase 5 (invariant #3).
  2. **The landed Task-1 signature differs from the design's proposal.** Real:
     `resolve_attacker_fire(snap: &AttackerSnapshot, entities: &EntityStore,
     rules, interner: &mut StringInterner, fog, occupancy: &OccupancyGrid,
     overlay_grid, overlay_registry, terrain, binary_frame, tick_ms,
     out: &mut CombatEmit)` (`combat/mod.rs:1772-1784`). No `resource_nodes`, no
     `FireOutcome` return, `&EntityStore` immutable. The host calls THIS.
  3. **Every cited line number drifted** (see live map below).
- **Live surfaces (re-verified this session — re-Read before editing, they drift):**
  - `resolve_attacker_fire` @ `combat/mod.rs:1772-1784` (private `fn` — must widen
    to `pub(crate)`); `struct CombatEmit` (16 fields) @ `combat/mod.rs:1181-1203`
    (private — must widen to `pub(crate)`).
  - `struct AttackerSnapshot` (24 fields) @ `combat/combat_targeting.rs:54-84`
    (already `pub`).
  - Snapshot build is **inline** in `tick_combat_with_fog` Phase 1
    (`combat/mod.rs:1404-1549`); per-entity reads @ `:1449-1476`; the **cooldown /
    burst-delay decrement** is `attack.cooldown_ticks/burst_delay_ticks =
    …saturating_sub(1)` @ `:1421-1422`, inside the `get_mut` pre-pass, run for
    **all** attackers (with `attack_target`, not in-transport) in `keys_sorted()`
    id order, **before** snapshots, regardless of fire-block.
  - Phase 5 of `advance_tick`: `let logic_order = live_object_order_snapshot();`
    @ `world/mod.rs:2050`; `combat::tick_combat_with_fog(…)` @ `:2051-2067`
    (last arg `&logic_order`); `turret::tick_turret_rotation(&mut …entities,
    rules, self.binary_frame, &self.interner)` @ `:2068-2073` (AFTER combat);
    combat-emitted smudge drain (scenario_rng, emission order) @ `:2260-2273`;
    despawn→`unregister_live_object` @ `:2087-2090`; `flush_pending_delete()` @
    `:2288`.
  - `tick_turret_rotation` @ `turret.rs:82-87`: iterates `keys_sorted()`
    (id-ascending), skips `barrel_facing.is_none()`, computes
    `facing_toward_lepton(...)` toward the `attack_target` (Entity or Cell) else
    `body_facing_to_turret(entity.facing)`, then `barrel.set_rot(rot)` @ `:168`
    + `barrel.set(target_facing, binary_frame)` @ `:169`. **Category-agnostic.**
  - `SNAPSHOT_VERSION = 17` @ `sim/snapshot.rs:24` — **do NOT touch**.
  - Test harness `combat/combat_turret_facing_tests.rs` (helpers `spawn_turreted`
    `:42`, `spawn_target` `:49`, `rules_with_mtnk_rot` `:25`, `empty_height_map`
    `:18`, `use_test_interner` `:57`; keep-green tests @ `:62/:91/:149/:174`).
- **gamemd behavior the shadow pins (verified docs):** `UnitClass::AI @ 0x007360C0`
  runs `Fire_At_Target` (`0x007365E1`) **before** `Facing_Update` (`0x007365E8`)
  — fire reads previous-tick facing; a target order cannot rotate-and-fire the
  same tick (1-tick acquisition latency). Sources:
  `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` §4,
  `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` §7,
  `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md` (shell boundary).
- **RNG:** `combat/mod.rs` consumes **zero** RNG (grep-confirmed). The host must
  add none. (The damage-particle RNG is S4, not L2.)
- **INI:** none — pure plumbing + a shadow; no new game constants.
- **Unknown after grounding (→ Deferred / flagged for /review-plan):** (a) the
  exact `FacingClass` accessor for the *current rotation destination* (to assert
  facing equality) — `FacingClass` API at `movement/facing_class.rs` not fully
  read; (b) whether combat-path `interner.intern()` is **always** a lookup of a
  pre-interned string (it must be, or the debug shadow's pre-interning shifts
  interned-id assignment vs release — a determinism hazard; guarded by a
  len-unchanged assert).

## Key Technical Decisions

- **L2 host runs in Phase 5, NOT in `object_ai_stage`.** Preserves invariant #3
  (fire stays after movement; no phase reorder). The early `object_ai_stage`
  no-op is the future S4 home; `unit_post`'s signature is shaped so S4 moves it
  there with no re-plumbing. **Confidence:** high — **Source:** design §7 inv #3
  + user decision this session + live `world/mod.rs:1788` vs Phase 5 `:2051`.
- **Shadow runs read-only at the TOP of Phase 5 (before the legacy sweeps), into
  a scratch `CombatEmit` + a facing record; assertions run after the sweeps.**
  Pre-sweep state is identical to what legacy fire/facing read (nothing mutates
  HP/positions/barrels between Phase-5 start and legacy Phase-2, since the shadow
  is read-only). So the shadow's fire/facing computation equals the legacy's, and
  the post-sweep asserts compare scratch-Unit output vs `combat_result` filtered
  to Unit attackers (fire) and vs the post-turret barrel destination (facing).
  **Confidence:** medium → **flag for /review-plan.** **Source:** derived from the
  Phase-5 dataflow this session (`world/mod.rs:2050-2073`); the design's looser
  "scratch CombatEmit + debug_assert" realized concretely.
- **Extract a shared `build_attacker_snapshot` (pure refactor) so host and legacy
  build byte-identical snapshots.** The host must build a per-object
  `AttackerSnapshot` to call `resolve_attacker_fire`; duplicating the inline
  `:1449-1476` logic would risk drift. Factor it out; legacy Phase-1 calls it; the
  host calls it. The **cooldown decrement stays a separate explicit mutation** in
  legacy Phase-1 (the builder is pure-read). **Confidence:** medium → flag for
  /review-plan (the extraction shape). **Source:** `combat/mod.rs:1404-1549`.
- **Cooldown decrement: shadow does NOT mutate; reads the already-decremented
  value (legacy decremented it this tick for all attackers including Units).** The
  host's snapshot uses `cooldown_ticks`/`burst_delay_ticks` as-is post-legacy. A
  separate unit test proves per-object decrement (live order) == the id-order
  pre-pass (saturating_sub(1) is per-entity, order-independent), so the future
  flip is safe. **Confidence:** high — **Source:** `combat/mod.rs:1421-1422` +
  algebraic order-independence of per-entity `saturating_sub`.
- **Interner safety: the shadow must not grow the interner.** Combat-path
  `intern()` of warhead/weapon/anim ids must be a lookup of a pre-interned string;
  if the shadow ever adds a new id, debug vs release interned-id assignment
  diverges → hash divergence across build types. Guard: assert `interner.len()`
  unchanged across the shadow walk. **Confidence:** medium → flag for /review-plan
  (confirm all combat-path strings are pre-interned at rules load). **Source:**
  determinism analysis this session; `resolve_attacker_fire` interns at
  `combat/mod.rs` (warhead/weapon/report/occupant_anim).
- **No `dyn`/trait dispatch; the host is called from a `match category == Unit`
  filter.** **Confidence:** high — **Source:** design invariant #2.

## Open Questions

### Resolved During Planning
- *Does L2 use the now-landed `object_ai_stage`?* — No. It runs before movement;
  L2 must keep fire+facing in Phase 5 (inv #3). User-confirmed.
- *Is damage-apply threading needed?* — No. Task-0 verdict: deferred-projectile,
  keep batched P4/P6. The host emits `damage_events` into `out`; nothing applies
  them in shadow.
- *Can the shadow recompute fire post-sweep?* — No (state mutates). It must compute
  at Phase-5 start on pre-sweep state; asserts run after.

### Deferred to Implementation
- **Exact `FacingClass` rotation-destination accessor** for the facing-equality
  assert — read `movement/facing_class.rs` and use the existing destination/target
  getter; if none exists, compare `barrel.current(binary_frame)` *after the
  rotation settles* is wrong (mid-arc) — so a destination getter is required.
  Resolve by reading the type; flagged for /review-plan.
- **Whether `tick_turret_rotation` and `resolve_attacker_fire` need the host to
  pass `tick_ms`** — `resolve_attacker_fire` takes `tick_ms`; the host reads it
  from the Phase-5 `tick_ms` arg of `advance_tick`. Confirm it is in scope at the
  shadow call site.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/world/unit_post.rs` | `unit_post()` Fire→Facing host + `unit_post_shadow_step()` read-only variant + `L2_UNIT_POST_AUTHORITATIVE` const |
| Modify | `src/sim/combat/mod.rs` | Widen `resolve_attacker_fire` + `CombatEmit` to `pub(crate)`; extract `build_attacker_snapshot` (pure refactor) and call it from Phase 1 |
| Modify | `src/sim/world/mod.rs` | `mod unit_post;` decl; debug-only shadow walk + asserts inside Phase 5 |
| Modify | `src/sim/combat/combat_turret_facing_tests.rs` | New named acceptance tests (fire→facing order, fire-reads-prev-facing, turret reorder no-drift, snapshot-retired-others-unchanged, cooldown order-independent, no-RNG) |

## Interface Changes

- `resolve_attacker_fire` — visibility widened `fn` → `pub(crate) fn` (so
  `world/unit_post.rs` can call it). No signature change. Only intra-crate callers.
- `struct CombatEmit` — `struct` → `pub(crate) struct`; its 16 fields become
  `pub(crate)` (host pushes into them / reads them). No layout change.
- `build_attacker_snapshot` — **new** `pub(crate) fn` in `combat/mod.rs` (or
  `combat_targeting.rs`); called by legacy Phase-1 and by the host. Pure read.
- `unit_post`, `unit_post_shadow_step`, `L2_UNIT_POST_AUTHORITATIVE` — **new**,
  `pub(crate)` / module-private. Nothing external depends on them yet.
- `tick_combat_with_fog` / `tick_turret_rotation` public signatures — **unchanged**.

## Sim Checklist

- [x] No new f32/f64 — host reuses fixed-point combat/turret math unchanged.
- [x] No new hashed state — the shadow is read-only; scratch `CombatEmit` is a
      transient debug local, never stored/hashed; `SNAPSHOT_VERSION` untouched.
- [x] No dependency on render/ui/sidebar/audio/net — stays within `sim/`.
- [x] Tick ordering unchanged — shadow runs *inside* Phase 5; legacy sweeps stay
      authoritative; no phase moves; `object_ai_stage` untouched.
- [x] BTreeMap / live-order iteration — host walks `live_object_order_snapshot()`
      (point-in-time, NOT `for_each_live_object`), matching legacy combat's order
      source; Unit subset compared for equality.
- [x] RNG — host draws zero; interner-length-unchanged assert guards against a
      hidden determinism shift.

## Risk Areas

- **Determinism across build types (highest).** The debug shadow pre-interns and
  pre-reads; it must be provably side-effect-free (no interner growth, no entity
  mutation, no RNG). Mitigation: `interner.len()` unchanged assert; shadow takes
  read-only borrows except `&mut interner` (lookup-only); no `barrel.set`, no HP
  write, no cooldown write in shadow mode.
- **Snapshot drift between host and legacy.** Mitigated by the shared
  `build_attacker_snapshot` (single source) + the per-Unit snapshot-equality and
  damage-event-equality asserts.
- **Facing reorder (id-order → live-order) output-neutrality.** Genuine order
  change; pinned by `turret_sweep_retired_for_scoped_units_no_drift` and the
  per-tick facing-destination assert.
- **`combat/mod.rs` is hash-critical.** The only edit there is a visibility widen
  + a pure `build_attacker_snapshot` extraction — same bit-identical discipline as
  Task 1; the 143-test combat suite + per-tick hash are the gate.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2,5 | **Fire-before-Facing within one object pass** | gamemd `UnitClass::AI` runs `Fire_At_Target` then `Facing_Update`; fire reads previous-tick facing → 1-tick acquisition latency. A swap makes turrets rotate-and-fire same tick. | `unit_ai_fire_then_facing_update_order` + `unit_fire_reads_previous_tick_facing`; cites `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT` §4. |
| 4 | **Per-Unit fire emission order == legacy live-LOGIC order** | The post-combat `scenario_rng` smudge drain (`world/mod.rs:2260`) consumes `smudge_spawn_requests` in emission order; any reorder shifts the lockstep RNG cursor → desync. | Shadow asserts scratch Unit `damage_events`/`smudge_spawn_requests` order == `combat_result` filtered to Unit attackers; `smudge_emission_order_unchanged`. |
| 1 | **`build_attacker_snapshot` is byte-identical to the inline build** | A drifted field = wrong fire decision for every Unit. | `cargo test -p vera20k combat` stays 143/0 after the pure extraction. |
| 3 | **Cooldown/burst-delay decrement is order-independent** | Future flip moves the decrement from an id-order pre-pass to a per-object live-order step; if not order-independent the first-fire timing drifts. | `unit_cooldown_decrement_order_independent`. |
| 4 | **Facing reorder (id→live order) is output-neutral** | Removing the id-order turret sweep for Units must not change any barrel destination. | `turret_sweep_retired_for_scoped_units_no_drift` + per-tick facing-destination assert. |
| 4 | **Interner not mutated by the shadow** | Debug-only pre-interning that grows the interner shifts interned-id assignment vs release → cross-build hash divergence. | `debug_assert_eq!(interner.len() before==after)` around the shadow walk. |

---

## Tasks

### Task 1: Widen visibility + extract `build_attacker_snapshot` (pure refactor)

**Why:** The host needs to call `resolve_attacker_fire` and build an
`AttackerSnapshot` per object the *same* way legacy does. Make the entry points
reachable and the snapshot build a single shared source. Zero behavior change.

**Files:** Modify `src/sim/combat/mod.rs` (+ possibly `combat/combat_targeting.rs`).
Re-Read `:1181-1203`, `:1404-1549`, `:1772-1784` immediately before editing.

**Step 1: Widen `resolve_attacker_fire` and `CombatEmit` to `pub(crate)`.**
```rust
// combat/mod.rs ~:1181 — was `#[derive(Default)] struct CombatEmit {`
#[derive(Default)]
pub(crate) struct CombatEmit {
    pub(crate) damage_events: Vec<(u64, u16, u64, InternedId)>,
    pub(crate) remove_attack: Vec<u64>,
    pub(crate) retarget_events: Vec<(u64, u64)>,
    pub(crate) fire_events: Vec<SimFireEvent>,
    pub(crate) reveal_events: Vec<RevealEvent>,
    pub(crate) bridge_damage_events: Vec<BridgeDamageEvent>,
    pub(crate) wall_damage_events: Vec<WallDamageEvent>,
    pub(crate) terrain_damage_events: Vec<TerrainDamageEvent>,
    pub(crate) tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    pub(crate) explosion_effects: Vec<ExplosionEffect>,
    pub(crate) smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    pub(crate) burst_updates: Vec<(u64, u8, u8, u16)>,
    pub(crate) ammo_deduct: Vec<u64>,
    pub(crate) garrison_advance: Vec<u64>,
    pub(crate) pending_infantry_updates: Vec<(u64, Option<PendingInfantryFire>)>,
    pub(crate) animation_switches: Vec<(u64, SequenceKind)>,
}
```
```rust
// combat/mod.rs ~:1772 — was `fn resolve_attacker_fire(`
pub(crate) fn resolve_attacker_fire(
    // ...signature unchanged...
)
```
(Field doc-comments inside `CombatEmit` are preserved; only the leading `pub(crate)`
is added per field. Do NOT reorder fields.)

**Step 2: Extract the inline per-entity snapshot build into a shared function.**
Read `combat/mod.rs:1404-1549`. The Phase-1 loop currently: (a) `get_mut`s the
entity, (b) decrements `attack.cooldown_ticks`/`burst_delay_ticks` (`:1421-1422`),
(c) reads garrison cargo, (d) reads `:1449-1476` into a tuple, (e) destructures and
`snapshots.push(AttackerSnapshot { … })` (`:1523-1548`). Extract (d)+(e) — the
**pure read** that maps a `&GameEntity` (+ the already-decremented cooldown/burst
values + garrison snapshot) into an `AttackerSnapshot` — into:
```rust
/// Build the per-attacker fire snapshot from current entity state. PURE READ —
/// the caller has already decremented cooldown/burst-delay for this tick. Returns
/// the snapshot fields the fire decision (`resolve_attacker_fire`) consumes.
pub(crate) fn build_attacker_snapshot(
    entity: &GameEntity,
    target: TargetKind,
    cooldown_ticks: u16,
    burst_remaining: u8,
    burst_delay_ticks: u8,
    pending_infantry_fire: Option<PendingInfantryFire>,
    garrison: Option<GarrisonSnapshot>,
) -> AttackerSnapshot {
    AttackerSnapshot {
        stable_id: entity.stable_id,
        owner: entity.owner,
        category: entity.category,
        target,
        pos_rx: entity.position.rx,
        pos_ry: entity.position.ry,
        pos_z: entity.position.z,
        sub_x: entity.position.sub_x,
        sub_y: entity.position.sub_y,
        type_id: entity.type_ref,
        facing: entity.facing,
        veterancy: entity.veterancy,
        cooldown_ticks,
        animation_sequence: entity.animation.as_ref().map(|a| a.sequence),
        animation_frame: entity.animation.as_ref().map(|a| a.frame_index),
        is_prone: entity.infantry.as_ref().is_some_and(|i| i.is_prone),
        is_fully_deployed: entity.is_fully_deployed(),
        has_movement: entity.movement_target.is_some(),
        pending_infantry_fire,
        barrel_facing: entity.barrel_facing,
        burst_remaining,
        burst_delay_ticks,
        weapon_override: entity.weapon_override,
        garrison,
    }
}
```
> **Exact field reads MUST be copied verbatim from the live `:1449-1548` block** —
> the snippet above is the candidate; reconcile every field against the current
> source (e.g. `is_prone`, `is_fully_deployed()`, `weapon_override` accessors) so
> the extraction is bit-identical. The compiler + the 143-test gate are the oracle.
Then replace the inline tuple-build + `push` in Phase 1 with:
```rust
snapshots.push(build_attacker_snapshot(
    entity, attack_target, cooldown_ticks, burst_remaining,
    burst_delay_ticks, pending_infantry_fire, garrison,
));
```
Keep the cooldown decrement (`:1421-1422`) and garrison-cargo read exactly where
they are — only the field-read + push moves into the function.

**Step 3: Verify.** Pre-flight `tasklist | grep -iE "cargo|rustc"`; then
`cargo check -p vera20k -q` (exit 0) and `cargo test -p vera20k combat` —
**must stay `143 passed; 0 failed`** (pure refactor). Read the literal
`test result:` line.

**Step 4: Commit** — `git add src/sim/combat/mod.rs` (+ `combat_targeting.rs` if
touched) `&& git commit -m "sim/combat: extract build_attacker_snapshot + widen fire entry to pub(crate) (L2 Task 2 prep, no behavior change)"`

---

### Task 2: Create `unit_post.rs` — the Fire→Facing host (not yet wired)

**Why:** Define the per-object host (the future authoritative Unit-arm body) and a
read-only shadow variant, before wiring. Types/host first.

**Files:** Create `src/sim/world/unit_post.rs`; modify `src/sim/world/mod.rs`
(add `mod unit_post;`). Re-Read `turret.rs:82-170` and `combat/mod.rs:1772-1784`
first.

**Step 1: Module header + flag.**
```rust
//! Per-object UnitClass post-Foot step: Fire_At_Target → Facing_Update.
//!
//! Native order (gamemd `UnitClass::AI`): FIRE reads PREVIOUS-tick barrel facing,
//! then FACING_UPDATE rotates the barrel toward the target for next tick — so a
//! freshly-acquired target cannot rotate-and-fire the same tick (1-tick latency).
//!
//! L2 scope: SHADOW only. The legacy `tick_combat_with_fog` + `tick_turret_rotation`
//! sweeps stay authoritative; this host runs read-only in debug to prove agreement
//! before a later slice flips authority. Depends on `sim/combat` (fire body,
//! snapshot builder) and `sim/movement/turret` (facing helpers). Never depends on
//! render/ui/audio/net.

/// When true, `unit_post` is authoritative for Unit fire+facing (flips the legacy
/// sweeps off for Units). L2 leaves this FALSE — the flip is a later slice.
pub(crate) const L2_UNIT_POST_AUTHORITATIVE: bool = false;
```

**Step 2: The read-only shadow step (the only entry L2 actually runs).**
This computes one Unit's fire (into `out`) + desired facing (recorded), mutating
NOTHING except interning lookups. It builds the snapshot via the shared
`build_attacker_snapshot`, reading the already-decremented cooldown (legacy
decremented it this tick).
```rust
/// Read-only shadow of one Unit's Fire→Facing for the current tick. Emits fire
/// events into `out`; returns the desired barrel facing the Facing step would set
/// (None when the unit is not turreted). Does NOT mutate entities/occupancy/barrel
/// or decrement cooldown. Caller invokes once per Unit in live-LOGIC order.
pub(crate) fn unit_post_shadow_step(
    sim: &Simulation,
    interner: &mut StringInterner,
    id: u64,
    rules: &RuleSet,
    binary_frame: u32,
    tick_ms: u32,
    out: &mut CombatEmit,
) -> Option<u16> {
    let entity = sim.substrate.entities.get(id)?;
    // FIRE: only firing attackers (mirror the legacy snapshot inclusion test:
    // has attack_target, alive, not inside a transport, not fire-blocked).
    if let Some(attack) = entity.attack_target.as_ref() {
        if !entity.passenger_role.is_inside_transport() /* && !fire_blocked */ {
            // Cooldown already decremented by legacy Phase-1 this tick; read as-is.
            let snap = combat::build_attacker_snapshot(
                entity,
                attack.target,
                attack.cooldown_ticks,
                attack.burst_remaining,
                attack.burst_delay_ticks,
                attack.pending_infantry_fire,
                None, // Units are never garrison occupants
            );
            combat::resolve_attacker_fire(
                &snap,
                &sim.substrate.entities,
                rules,
                interner,
                Some(&sim.fog),
                &sim.substrate.occupancy,
                sim.overlay_grid.as_ref(),
                /* overlay_registry */ None, // see note: thread from advance_tick arg
                sim.resolved_terrain.as_ref(),
                binary_frame,
                tick_ms,
                out,
            );
        }
    }
    // FACING: reproduce tick_turret_rotation's per-entity desired facing.
    turret::desired_turret_facing(entity, &sim.substrate.entities)
}
```
> **Reconcile against live code (flagged for /review-plan):**
> - The **fire-inclusion predicate** must match legacy Phase-1 exactly, including
>   `fire_blocked` (legacy computes it via
>   `combat_fire_gate::collect_fire_blocked_entities`). Either compute the same
>   blocked-set once at the shadow-walk site and pass membership in, or call the
>   same gate helper. Do NOT approximate.
> - `overlay_registry` is an `advance_tick` parameter, not a `sim` field — thread
>   it into the shadow step (add a param) rather than passing `None`.
> - **`desired_turret_facing`** does not exist yet — Task 2 Step 3 extracts it
>   from `tick_turret_rotation` so host and sweep share one source.

**Step 3: Extract `desired_turret_facing` from `tick_turret_rotation` (pure
refactor of `turret.rs`).** Read `turret.rs:94-150`. Factor the per-entity
desired-facing computation (the `if let Some(ref attack) … facing_toward_lepton …
else body_facing_to_turret(entity.facing)` block) into:
```rust
/// The barrel facing `tick_turret_rotation` would drive this entity toward this
/// tick. None when the entity has no turret. Pure read.
pub(crate) fn desired_turret_facing(entity: &GameEntity, entities: &EntityStore) -> Option<u16> {
    entity.barrel_facing.as_ref()?; // None when not turreted
    let desired = if let Some(ref attack) = entity.attack_target {
        let target_pos = match attack.target {
            TargetKind::Entity(tid) => entities.get(tid)
                .map(|t| (t.position.rx, t.position.ry, t.position.sub_x, t.position.sub_y)),
            TargetKind::Cell(rx, ry) => Some((rx, ry, SimFixed::from_num(128), SimFixed::from_num(128))),
        };
        match target_pos {
            Some((trx, try_, tsx, tsy)) => facing_toward_lepton(
                entity.position.rx, entity.position.ry, entity.position.sub_x, entity.position.sub_y,
                trx, try_, tsx, tsy),
            None => body_facing_to_turret(entity.facing),
        }
    } else {
        body_facing_to_turret(entity.facing)
    };
    Some(desired)
}
```
Then `tick_turret_rotation`'s Phase-1 loop calls `desired_turret_facing(entity,
entities)` instead of the inline block. Verify combat tests stay green (it pins
turret behavior).

**Step 4: The future authoritative host (define now, unused while flag is false).**
Add `unit_post` (mutating: per-object cooldown decrement + `barrel.set`) gated so
L2 never runs it. Keep it minimal — it makes the Task-3 flip a small diff:
```rust
/// Authoritative per-object Unit Fire→Facing. UNUSED while
/// `L2_UNIT_POST_AUTHORITATIVE == false`; the later flip slice drives it.
#[allow(dead_code)]
pub(crate) fn unit_post(
    sim: &mut Simulation, id: u64, rules: &RuleSet,
    binary_frame: u32, tick_ms: u32, out: &mut CombatEmit,
) {
    // 1. FIRE — decrement cooldown/burst-delay for THIS object (Task 2a), build
    //    snapshot, resolve_attacker_fire into `out` (NOT applied here; P4 batch).
    // 2. FACING — desired = desired_turret_facing(entity, …); barrel.set_rot(rot);
    //    barrel.set(desired, binary_frame).
    // Bodies filled by the flip slice; left as a typed stub here to fix the seam.
    let _ = (sim, id, rules, binary_frame, tick_ms, out);
}
```

**Step 5: Add `mod unit_post;` to `world/mod.rs`** (near the other `mod` decls).

**Step 6: Verify.** `cargo check -p vera20k -q` (exit 0; `unit_post`/const may warn
dead-code — acceptable). `cargo test -p vera20k combat` stays 143/0 (the
`desired_turret_facing` extraction is the only behavior-touching change and is a
pure refactor).

**Step 7: Commit** — `git commit -m "sim/world: add unit_post Fire→Facing host (shadow scaffold) + extract desired_turret_facing (L2 Task 2)"`

---

### Task 3 (= design Task 2a): Cooldown decrement order-independence

**Why:** The future flip moves the cooldown/burst-delay decrement from the legacy
id-order pre-pass (`combat/mod.rs:1421-1422`) to a per-object live-order step
inside `unit_post`. Prove that reorder is output-neutral now.

**Files:** Modify `src/sim/combat/combat_turret_facing_tests.rs` (add the test).

**Step 1: Add the proof test.** `saturating_sub(1)` is per-entity with no
cross-entity dependency, so any visitation order yields identical results — pin it
empirically across two orders.
```rust
#[test]
fn unit_cooldown_decrement_order_independent() {
    // Two Units with distinct cooldown/burst-delay values. Decrement each by one
    // tick in id-ascending order, then (fresh copy) in reverse order; the
    // resulting (cooldown, burst_delay) per unit must be identical.
    let start = [(1u64, 7u16, 3u8), (2u64, 4u16, 0u8)];
    let dec = |v: &mut [(u64, u16, u8)], order: &[usize]| {
        for &i in order {
            v[i].1 = v[i].1.saturating_sub(1);
            v[i].2 = v[i].2.saturating_sub(1);
        }
    };
    let mut a = start.to_vec();
    let mut b = start.to_vec();
    dec(&mut a, &[0, 1]);
    dec(&mut b, &[1, 0]);
    assert_eq!(a, b, "per-entity cooldown decrement must be order-independent");
}
```
> Reconcile field names with the real `AttackTarget` accessors when the flip slice
> wires the live decrement; this test pins the algebraic property the flip relies
> on. (If preferred, build two `Simulation`s with the units in opposite live order
> and assert equal post-tick `cooldown_ticks` — a heavier but end-to-end version.)

**Step 2: Verify.** `cargo test -p vera20k combat` — 143 prior + this new test pass.

**Step 3: Commit** — `git commit -m "test(sim/combat): pin cooldown decrement order-independence (L2 Task 2a)"`

---

### Task 4: Wire the debug-only shadow walk into Phase 5 + assertions

**Why:** Run the host read-only every debug tick and prove per-Unit fire/facing
agreement with the legacy sweeps. This is the safety net that makes the later flip
trustworthy.

**Files:** Modify `src/sim/world/mod.rs` (Phase 5, `:2050-2073` region). Re-Read it
first — these numbers drift.

**Step 1: At the TOP of Phase 5 (immediately before
`combat::tick_combat_with_fog` at `:2051`), compute the shadow on pre-sweep state
— debug builds only.**
```rust
// --- L2 shadow: per-object Unit Fire→Facing, read-only, debug only. Computes on
//     pre-sweep state (identical to what the legacy sweeps read this tick), then
//     asserts agreement AFTER the sweeps. Proves the future per-object flip is
//     output-neutral. Never mutates hashed state.
#[cfg(debug_assertions)]
let l2_shadow = {
    let interner_len_before = self.interner.len();
    let mut scratch = combat::CombatEmit::default();
    let mut want_facing: Vec<(u64, u16)> = Vec::new();   // (id, desired) for Units
    let mut fired_units: Vec<u64> = Vec::new();          // Unit attacker ids, in walk order
    for &id in &logic_order {
        let is_unit = self.substrate.entities.get(id)
            .is_some_and(|e| e.category == EntityCategory::Unit);
        if !is_unit { continue; }
        let before = scratch.fire_events.len();
        let desired = crate::sim::world::unit_post::unit_post_shadow_step(
            self, &mut self.interner, id, rules, self.binary_frame, tick_ms,
            overlay_registry, &mut scratch,
        );
        if scratch.fire_events.len() != before { fired_units.push(id); }
        if let Some(d) = desired { want_facing.push((id, d)); }
    }
    debug_assert_eq!(self.interner.len(), interner_len_before,
        "L2 shadow must not grow the interner (determinism hazard)");
    (scratch, want_facing, fired_units)
};
```
> **Borrow note:** `unit_post_shadow_step` takes `&Simulation` + `&mut self.interner`
> — these are disjoint fields, but the loop above borrows `self` immutably (via the
> `&Simulation` arg) while also `&mut self.interner`. Resolve by splitting the
> borrow (e.g. destructure the needed fields before the loop, or pass the entity
> store / occupancy / fog / overlay refs explicitly instead of `&Simulation`). The
> compiler is the oracle here — flagged for /review-plan; the clean fix is likely
> to pass the concrete read-only refs rather than `&Simulation`.

**Step 2: AFTER the turret sweep (`:2073`), assert agreement — debug only.**
```rust
#[cfg(debug_assertions)]
{
    let (scratch, want_facing, _fired) = l2_shadow;
    // FIRE: scratch Unit damage_events must equal combat_result's, restricted to
    // Unit attackers, in the same order (emission-order / smudge-cursor critical).
    let legacy_unit_dmg: Vec<_> = combat_result.damage_events_for_units_only(); // helper or inline filter by attacker category
    debug_assert_eq!(scratch.damage_events, legacy_unit_dmg,
        "L2 shadow Unit fire damage_events diverged from the legacy sweep");
    // FACING: each Unit's desired facing must equal the destination the legacy
    // turret sweep set on its barrel.
    for (id, desired) in want_facing {
        if let Some(dest) = self.substrate.entities.get(id)
            .and_then(|e| e.barrel_facing.as_ref())
            .map(|b| b.destination()) // FacingClass destination accessor — verify name
        {
            debug_assert_eq!(dest, desired,
                "L2 shadow Unit {id} barrel destination diverged from turret sweep");
        }
    }
}
```
> **Reconcile (flagged for /review-plan):**
> - `combat_result` does NOT expose a Unit-only damage filter; either filter
>   `combat_result.damage_events` by looking up each `attacker_id`'s category
>   (note: a dead attacker may already be concealed — resolve by capturing the
>   firing-Unit id set during the shadow and filtering on `attacker_id ∈ fired`),
>   or compare against `scratch` differently. The robust comparison: the shadow
>   captured every Unit attacker's emitted events into `scratch`; assert the
>   subsequence of `combat_result.damage_events` whose `attacker_id` is a Unit
>   equals `scratch.damage_events`.
> - `FacingClass::destination()` — confirm the real accessor name in
>   `movement/facing_class.rs`. If the barrel is mid-rotation, compare the
>   *destination*, never the animated `current(frame)`.
> - Also assert `scratch.smudge_spawn_requests` Unit subset matches in order
>   (the RNG-cursor invariant, `smudge_emission_order_unchanged`).

**Step 3: Verify.** `cargo check -p vera20k -q`. Then run a debug skirmish/replay
test (`cargo test -p vera20k combat` and the sim integration tests run under
`debug_assertions`): the new asserts must not fire. If one fires, STOP — log the
id/tick and diagnose whether it is an intended-interleave difference (→ that's the
Task-3 flip's hash change, out of scope here) or a host bug (→ fix the host).

**Step 4: Commit** — `git commit -m "sim/world: debug shadow walk for unit_post Fire→Facing agreement (L2 Task 2)"`

---

### Task 5: Named acceptance tests

**Why:** Pin the slice-spec behaviors so the later flip has explicit goldens.

**Files:** Modify `src/sim/combat/combat_turret_facing_tests.rs`. Mirror the
existing harness (`spawn_turreted` `:42`, `spawn_target` `:49`,
`rules_with_mtnk_rot` `:25`, `advance_tick(&[], Some(&rules), &empty_height_map(),
None, None, 67)`).

**Step 1: `unit_ai_fire_then_facing_update_order`** — spawn a turreted Unit + a
side target, assign the target, advance one tick. Assert the fire decision read the
**previous-tick** barrel facing (no damage on the acquisition tick because the
barrel was not yet aligned) AND the barrel rotation only *began* this tick
(`barrel.destination()` now points at the target, but `current(frame)` has not
reached it). This is the Rust expression of `Fire_At_Target` before `Facing_Update`.

**Step 2: `unit_fire_reads_previous_tick_facing`** — descendant of the existing
`one_tick_acquisition_latency_first_tick_no_fire` (`:62`): on the acquisition tick a
Unit does not rotate-and-fire; assert target HP unchanged after one tick, fire on a
later tick once aligned. (Keep the original test green.)

**Step 3: `turret_sweep_retired_for_scoped_units_no_drift`** — a mixed
Unit + Aircraft turreted scene: run a tick; assert each entity's
`desired_turret_facing(...)` (live-order host) equals the barrel destination the
id-order `tick_turret_rotation` set — proving the order change is output-neutral.

**Step 4: `combat_snapshot_retired_for_units_other_categories_unchanged`** — a Unit
+ garrisoned Building + Aircraft attacker scene: assert the shadow's `scratch`
contains fire events **only** for the Unit, and Building/Aircraft fire in
`combat_result` is byte-identical to a baseline run without the shadow (the shadow
must not perturb non-Unit fire).

**Step 5: `smudge_emission_order_unchanged`** — a multi-Unit destruction scene:
assert the `scenario_rng` position after the combat smudge drain is identical with
and without the shadow walk (the shadow emits into scratch only; it must not touch
`scenario_rng`). **Step 6: `unit_post_consumes_no_rng`** — assert both
`scenario_rng` and main-rng positions are unchanged by `unit_post_shadow_step` for
a firing Unit.

**Step 7: Verify.** `cargo test -p vera20k combat` — all prior 143 + the new tests
pass; the four keep-green tests (`:62/:91/:149/:174`) stay green. **Step 8: Commit**
— `git commit -m "test(sim/combat): L2 unit_post acceptance + parity pins (Task 2)"`

---

### Task 6: Full-suite verification (separate bounded pass)

**Why:** Confirm no hashed state moved and the whole sim stays green.

**Step 1:** Pre-flight `tasklist | grep -iE "cargo|rustc"`; then
`cargo check -p vera20k -q` (exit 0).
**Step 2:** `cargo test -p vera20k combat` — read the literal `test result:` line;
expect `143 passed` + the new tests, `0 failed`.
**Step 3:** `cargo test -p vera20k` (full sim) — all previously-green tests stay
green; in particular replay/`state_hash` goldens are **unmoved** (the slice adds no
hashed state; the shadow is debug-only and read-only). Read the literal
`test result:` lines. Confirm `-p vera20k` (a wrong `-p` exits 101 without running).
**Step 4:** If any `state_hash` golden moved, STOP and revert — the shadow leaked a
mutation (interner growth, an accidental `barrel.set`, a cooldown write). Find and
remove the side effect; never re-baseline in this slice (no flip = no hash change).

---

## Out of scope (explicitly deferred — do NOT do here)

- **Task 3 (flip authority)** and **Task 4 (`SNAPSHOT_VERSION` bump + golden
  re-baseline)** of the design — those are a separate plan after this shadow is
  green. L2 Task 2 ships the shadow only.
- Wiring `unit_post` into the early `object_ai_stage` (future S4 relocation).
- Retiring the global sweeps for Infantry / Aircraft / Building (their slices).
- Adding the S4 damage-particle RNG, HarvestBrain / Anim-Ammo / Spawn ordering.
- Changing projectile/hitscan timing or AoE/single-target damage math.

## Sources & References

- **Design doc:** `docs/plans/2026-06-02-ai-shell-unitclass-core-plan.md` Slice L2
  §4 Task 2/2a, §5, §7, §8 (re-scoped this session against live state).
- **Gating verdict:** `docs/research/L2_FIRE_DAMAGE_TIMING_VERDICT_GHIDRA_REPORT.md`
  (deferred-projectile → keep batched P4/P6).
- **gamemd behavior:** `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` §4
  (`UnitClass::AI @ 0x007360C0`: `Fire_At_Target 0x007365E1` → `Facing_Update
  0x007365E8`); `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` §7
  (tick order); `TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md` §3.4 (first
  safe migration boundary = mobile Techno object-AI shell).
- **Live code (re-verified this session):** `combat/mod.rs` (`resolve_attacker_fire`
  `:1772-1784`, `CombatEmit` `:1181-1203`, snapshot build `:1404-1549`, cooldown
  decrement `:1421-1422`); `combat/combat_targeting.rs:54-84` (`AttackerSnapshot`);
  `world/mod.rs` (`object_ai_stage` call `:1788`, Phase-5 combat `:2051-2067`,
  turret `:2068-2073`, smudge drain `:2260-2273`, `flush_pending_delete` `:2288`,
  `live_object_order_snapshot` `:918`, `for_each_live_object` `:941`); `turret.rs`
  (`tick_turret_rotation` `:82`, `barrel.set` `:168-169`); `snapshot.rs:24`
  (`SNAPSHOT_VERSION = 17`); `world/techno_ai.rs` (`object_ai_stage` `:41`,
  `techno_ai_shell` `:103`); test harness `combat/combat_turret_facing_tests.rs`.
- **Related (NOT edited here):** `world/unit_post.rs` (NEW); the flip slice's
  `SNAPSHOT_VERSION`/golden site (future).
