# Lockstep Determinism Integrity Bundle — Fix Plan

Status: RESEARCH / PLAN ONLY. No `src/` edits made. Verified against current code
2026-07-19. Cited file:line refs re-checked (several drifted from the scan).

Scope: four determinism gaps that break lockstep and/or save-reload faithfulness.
Authority: repo analysis only (no Ghidra needed). Golden-hash / SNAPSHOT_VERSION
ceremony notes included per gap; only **one session** may re-baseline goldens or
bump `SNAPSHOT_VERSION`.

Key constants:
- State hash: `src/sim/world/world_hash.rs` → `Simulation::state_hash()` (line 63).
- Harness golden: `GLOBAL_HARNESS_FINAL_HASH = 6318917868157055929`
  (`src/sim/world/global_parity_harness_tests.rs:102`), plus `FINAL_STREAM_STATES`
  and `POSITION_FINGERPRINT` in the same file.
- Snapshot format: `SNAPSHOT_VERSION = 27` (`src/sim/snapshot.rs:76`, asserted by
  `snapshot_version_is_27` at :432).

---

## GAP 1 — Unhashed authoritative GameEntity fields (+ two Simulation aggregates)

### Current-state evidence
`hash_entities` (`world_hash.rs:524-776`) folds ~48 per-entity fields. Cross-referencing
the full `GameEntity` struct (`src/sim/game_entity.rs:196-524`) against that fold, the
following authoritative, gameplay-affecting fields are **NOT hashed**. Each "mutated at"
cite is a non-test runtime write proving the field changes during a match.

**Correction to the scan:** `bunker_link` (w_hash:643), `bunker_occupant` (:642),
`c4_plant` (:639) ARE already hashed — the scan was stale on those. `mind_controlled`
is genuinely unhashed.

#### Must-hash (19 gameplay-critical entity fields)

| # | Field | Type | Why authoritative | Runtime mutation |
|---|-------|------|-------------------|------------------|
| 1 | `veterancy` | `u16` | elite/vet damage, ROF, sight, self-heal | `infantry.rs:275,282,321` |
| 2 | `repairing` | `bool` | building repair drains credits + heals HP each tick | `production_sell.rs:762,765,833,985` |
| 3 | `last_attacker_id` | `Option<u64>` | retaliation target selection | `combat/mod.rs:1861,1883` |
| 4 | `dying` | `bool` | `is_active()` gate — excludes from vision/power/production/movement/targeting | `combat/mod.rs:1007,1024`, `aircraft/mod.rs:651,799`, `iron_curtain.rs:57`, `world/mod.rs:1358` |
| 5 | `sub_cell` | `Option<u8>` | infantry occupancy sub-slot + fire origin | `occupancy.rs:310`, `drop_payload.rs:213,316`, `production_sell.rs:899,933` |
| 6 | `blocked_scatter_timer` | `u8` | countdown → when a blocked unit scatters | movement (blocked path) |
| 7 | `garrison_original_owner` | `Option<InternedId>` | ownership revert on garrison empty | `passenger.rs:1469,2144`, `production_sell.rs:1013` |
| 8 | `mind_controlled` | `bool` | mind-control gate (targeting/ownership) | `passenger.rs:1781` |
| 9 | `order_intent` | `Option<OrderIntent>` | AttackMove/Guard/Unloading persistent order, ticked by `world_orders` | order systems |
| 10 | `slave_harvester` | `Option<SlaveHarvester>` | slave-miner harvest AI (economy) | slave_miner system |
| 11 | `teleport_state` | `Option<TeleportState>` | chrono warp-out/in FSM | teleport_movement |
| 12 | `rocket_state` | `Option<RocketState>` | V3/Dreadnought rocket flight FSM | rocket_movement |
| 13 | `droppod_state` | `Option<DropPodState>` | drop-pod descent FSM | droppod_movement |
| 14 | `parachute_state` | `Option<ParachuteDescentState>` | paradrop descent FSM | parachute_descent |
| 15 | `dock_state` | `Option<DockState>` | repair-depot docking FSM | building_dock |
| 16 | `aircraft_ammo` | `Option<AircraftAmmo>` | ammo count gates attack-run vs RTB | aircraft_dock |
| 17 | `aircraft_mission` | `Option<AircraftMission>` | aircraft behavior FSM (attack/guard/RTB/idle) | aircraft/mod.rs |
| 18 | `building_up` | `Option<BuildingUp>` | construction anim; `elapsed_ticks` advances in sim, gates operational | `world/mod.rs:1852` |
| 19 | `building_down` | `Option<BuildingDown>` | undeploy anim; completion **spawns the MCV** | `world/mod.rs:1875-1876` |

#### 20th entity field — dormant, hash for completeness
- `tunnel_state: Option<TunnelState>` — TS-legacy subterranean, dormant in YR
  (`feedback_no_tunnel_subterranean`). Never set in stock YR, so it will not shift the
  golden, but the fold is a cheap `Option` presence tag and closes the audit hole if a
  future path ever sets it. Include a `Some=>{1u8; ...}` tag.

#### Simulation-level aggregates (NOT in hash_entities, also unhashed)
- `trigger_runtime: TriggerRuntime` (`world/mod.rs:491`) — global/local map vars, disabled
  triggers, fired one-shots, elapsed scenario ticks. Authoritative map-trigger state,
  serialized but **not** folded into `state_hash`. **Must hash** via a new
  `hash_trigger_runtime` fold.
- `ai_players: Vec<AiPlayerState>` (`world/mod.rs:379`) — per-AI-owner state. Authoritative
  when AI runs; currently AI is out of scope (`feedback_no_ai_yet`) so the Vec is typically
  empty (len 0 folds harmlessly). **Hash for completeness** (`len()` + per-element fold);
  guard against future silent divergence.

#### Fields that must NOT be hashed (render / debug / app-local caches — flag, do not fold)
- `position.screen_x/screen_y` (`#[serde(skip)]`, render cache) — already excluded.
- `selected` — app-layer local selection; documented non-authoritative (game_entity.rs:236).
- `in_logic_vector`, `presence` (`#[serde(skip)]`) — represented via the hashed logic
  order (`world_hash.rs:82-86`) / non-authoritative shadow; keep excluded.
- `debug_log` (`#[serde(skip)]`) — debug inspector only.
- `building_anim_overlays`, `animation`, `voxel_animation`, `harvest_overlay` — presentation
  animations driven by **wall-clock `elapsed_ms`** (not sim ticks) → render caches, keep out.
  (Flag: if any sim logic ever reads `animation.frame` for a fire/gameplay decision, promote
  it — today the fire gate lives in `attack_target.pending_infantry_fire`, which IS hashed.)
- `display_type_override` — documented "visual-only" VXL swap during unload (render).
- `zfudge_bridge` — documented "Render-only depth bias".
- `is_voxel` — render model selector, constant per type (redundant with `type_ref`).

#### Spawn-constant, type-derived (redundant with hashed `type_ref`; LOW, optional)
`crushable`, `deployed_crushable`, `omni_crusher`, `omni_crush_resistant`,
`immune_to_radiation`, `too_big_to_fit_under_bridge` — all set once from rules at spawn
(`world_spawn.rs:164-170`); only test code mutates them. Provably a function of `type_ref`
(hashed), so safe to omit. Include only as defense-in-depth if desired; not required.

### Exact fix
Append the new folds at the **end** of the per-entity body in `hash_entities`, in the fixed
order below, right after `entity.damage_particle_live_until.hash(hasher);` (w_hash:774).
Iteration already walks `self.substrate.entities.values()` (BTreeMap = ascending stable_id),
so entity order is deterministic; only the **field order within the body** is new contract.
Follow this file's existing idioms: `Option` → `0u8`/`1u8` presence tag then value; enums →
`(x as u8/u16)`; fixed-point → `.to_bits()`; render-only floats excluded.

```rust
// --- Determinism Bundle Gap 1: previously-unhashed authoritative fields ---
entity.veterancy.hash(hasher);
entity.repairing.hash(hasher);
entity.last_attacker_id.hash(hasher);         // Option<u64>: derives Hash
entity.dying.hash(hasher);
entity.sub_cell.hash(hasher);                  // Option<u8>
entity.blocked_scatter_timer.hash(hasher);
entity.garrison_original_owner.hash(hasher);   // Option<InternedId>
entity.mind_controlled.hash(hasher);
entity.order_intent.hash(hasher);              // enum derives Eq; add #[derive(Hash)] or fold explicitly
entity.tunnel_state.is_some().hash(hasher);    // dormant TS-legacy: tag only, or full fold if Hash added

// State machines: derive Hash on each (SimFixed via .to_bits(); exclude render-only floats,
// following the HomingState precedent at w_hash:738). Then:
match &entity.slave_harvester { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.teleport_state  { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.rocket_state    { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.droppod_state   { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.parachute_state { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.dock_state      { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.aircraft_ammo   { Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.aircraft_mission{ Some(s)=>{1u8.hash(hasher); s.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.building_up     { Some(b)=>{1u8.hash(hasher); b.elapsed_ticks.hash(hasher); b.total_ticks.hash(hasher);} None=>0u8.hash(hasher) }
match &entity.building_down   { Some(b)=>{1u8.hash(hasher); b.elapsed_ticks.hash(hasher); b.total_ticks.hash(hasher); b.spawn_type.hash(hasher); b.spawn_owner.hash(hasher); b.spawn_rx.hash(hasher); b.spawn_ry.hash(hasher); b.spawn_z.hash(hasher); b.was_selected.hash(hasher);} None=>0u8.hash(hasher) }
```

Prerequisite derives (add `#[derive(Hash)]` or manual Hash where a field is `SimFixed`/`f32`):
`OrderIntent` (all int), `SlaveHarvester`, `TeleportState`, `RocketState`, `DropPodState`,
`ParachuteDescentState`, `DockState`, `AircraftAmmo`, `AircraftMission`, `TunnelState`.
For any struct carrying render-only `f32`, follow `HomingState`'s manual-Hash-excluding-float
pattern; fold `SimFixed` via `.to_bits()`. (A quick per-struct grep for `f32`/`SimFixed`
tells you which need manual impls.)

Two new Simulation-level folds, called from `state_hash()` after `hash_anims`:
```rust
fn hash_trigger_runtime(&self, hasher) { /* elapsed ticks, global vars, local vars,
    disabled set (sorted), fired one-shots (sorted) — all in deterministic key order */ }
fn hash_ai_players(&self, hasher) { self.ai_players.len().hash(hasher);
    for a in &self.ai_players { /* explicit fields */ } }
```

### Risk / blast radius
- Adds ~30 lines to `hash_entities` + two small folds + up to 10 `#[derive(Hash)]`. No
  serialization change (all fields already in serde), so **no SNAPSHOT_VERSION bump** for
  Gap 1.
- **Every fold addition shifts `state_hash`** even where the added fields are at default
  values on harness entities → `GLOBAL_HARNESS_FINAL_HASH`, `FINAL_STREAM_STATES` (unlikely —
  no new RNG routing, so this should stay put; if it moves it's a bug), and
  `POSITION_FINGERPRINT` (should NOT move) must be re-checked. Expect only the **total-hash**
  baseline (`GLOBAL_HARNESS_FINAL_HASH`) to move; re-baseline it once with the documented
  reason "Gap-1 hashed 19 entity fields + trigger_runtime + ai_players".
- Land ALL of Gap 1 in **one commit** so there is exactly one re-baseline event.

### Effect on goldens
- `GLOBAL_HARNESS_FINAL_HASH`: **shifts → re-baseline once.**
- `SNAPSHOT_VERSION`: unchanged.
- One session only.

### Acceptance tests (add to `world_hash.rs` test modules, mirroring existing ones)
- Per-field: two sims identical except `veterancy` (100 vs 200) → `state_hash` differs.
  Repeat for `repairing`, `dying`, `mind_controlled`, `sub_cell`, `blocked_scatter_timer`,
  `garrison_original_owner`, `last_attacker_id`, `order_intent`, and each state-machine Option
  present-vs-absent + one inner-field mutation.
- `trigger_runtime`: flip a global variable → hash differs.
- Regression: empty-store sims still hash equal (`empty_particle_store_hashes_consistently`
  style).

---

## GAP 2 — Particle serde/hash split

### Current-state evidence
- `particle_systems: ParticleSystemStore` is `#[serde(skip)]` (`world/mod.rs:495-496`) →
  a save **drops all live particle systems**; on load it is restored empty
  (`construct` seeds `ParticleSystemStore::new()`).
- But it **is hashed** every tick (`world_hash.rs:100` → `hash_particle_systems`
  132-169). So an immediate post-load `state_hash` differs from the pre-save hash, and a
  reloaded client draws fewer future `particle_rng` values than a continuous client.
- Damage status: gas particle **damage application is DEFERRED (Task C6, not implemented)** —
  `gas.rs:15-17, 123-124, 134-137` explicitly stub the damage-to-cell-occupants hook.
  So particles deal **no** damage today. Their authoritative effect right now is **RNG-draw
  cadence**: ticking systems consume `particle_rng` (= `scenario_rng`, `world/mod.rs:663-665`)
  for lifetime/offset/direction/insert, and `scenario_rng` IS hashed and drives all downstream
  sim draws. C6 will add direct damage on top.

### Decision: option (a) — **make `particle_systems` serialize** (do NOT stop hashing it)
Rationale: even pre-C6 the live particle population changes future `scenario_rng` draw counts,
so it is lockstep-authoritative; dropping it from the hash (option b) would blind the desync
detector exactly where the stream split matters, and C6 will make it damage-authoritative.
Keeping the hash and fixing serde is the parity-correct direction.

### Exact fix
1. Remove `#[serde(skip)]` from `particle_systems` (`world/mod.rs:495`); add
   `#[serde(default)]` so pre-existing saves still load (restored empty).
2. Derive `Serialize, Deserialize` on the three particle types that currently lack them:
   `ParticleSystem` (`particles/mod.rs:38`), `Particle` (:68), `ParticleSystemStore` (:119).
   `SparkRuntimeState` (:58) already derives them.
   - Nested types to verify/annotate: `glam::IVec3` (needs glam `serde` feature — confirm in
     Cargo.toml, else store as `[i32;3]`), `SimFixed` (already serde), `NativeF32Bits` /
     `NativeF64Bits` (already serde per the spark test).
3. Bump `SNAPSHOT_VERSION` 27 → 28 (`snapshot.rs:76`) and update the `snapshot_version_is_27`
   test (:432) to `_is_28`, with a one-line changelog note (already the file's convention).
4. Fold order in `hash_particle_systems` is unchanged, so the **hash pre-image does not move** —
   Gap 2 does NOT re-baseline `GLOBAL_HARNESS_FINAL_HASH`.

### Risk / blast radius
- Serialization surface grows to the whole particle store; `ParticleSystemStore` iteration is
  already stable-id ordered (BTreeMap-style, per `hash_particle_systems`), so round-trip order
  is deterministic. Verify `ParticleSystemStore`'s internal container preserves id order after
  deserialize (it must, to keep the hash equal pre/post save).
- Watch fields with `#[serde(skip)]` inside `ParticleSystem`/`Particle` that the hash reads —
  none currently, but any future skip on a hashed field re-introduces this exact bug.

### Effect on goldens
- `SNAPSHOT_VERSION`: **27 → 28** (one session).
- `GLOBAL_HARNESS_FINAL_HASH`: unchanged.

### Acceptance tests
- Save/reload round-trip with ≥1 live particle system + particles (incl. a `Some(spark)`):
  `state_hash()` before save == `state_hash()` after load (today it differs). Assert the
  restored store has the same system/particle counts and coords.
- Old-format save (particles absent) still deserializes (empty store) — `#[serde(default)]`.

---

## GAP 3 — Client-side wall auto-fill (out of sim pipeline)

### Current-state evidence
- `fill_wall_between_endpoints` (`app_commands.rs:385-468`, called from
  `place_ready_building_at_cursor` at :330-332) mutates **`state.overlays`** (app layer) —
  it walks `state.overlays` for existing same-type walls and pushes free `OverlayEntry`s,
  then calls `compute_wall_connectivity`. This happens **app-side, after** the
  `Command::PlaceReadyBuilding` is merely *queued* (:297-306).
- The sim, by contrast, spawns a wall as a real **entity** via `spawn_object`
  (`production_placement.rs:206`, `place_ready_building`), which registers occupancy.
- Consequence: auto-filled cells have **no sim entity/occupancy** — they don't block
  movement, can't be damaged/destroyed, and each client computes its own fill from its own
  app-layer `state.overlays`, so two lockstep clients can disagree → desync + wrong occupancy.

### Exact fix
Move the auto-fill into the deterministic sim, inside `place_ready_building`
(`production_placement.rs`), gated on `obj.wall` (the same predicate the app uses,
`app_commands.rs:324-329`):
1. After the placed wall entity spawns, run the 4-cardinal walk **against sim state**: for each
   direction, scan outward reading the **entity store / sim occupancy** (reuse
   `structure_occupies_cell`, already used app-side at `app_commands.rs:413`) for an existing
   same-type wall endpoint; stop at a blocker or map edge.
2. For each intermediate cell between the placed wall and the found endpoint, **spawn a free
   wall entity** in the sim (no queue/credit consumption — matches today's "free fill"),
   registering occupancy like any wall. Determinism is automatic (runs once, in-sim, from
   sim state that both clients share).
3. Delete `fill_wall_between_endpoints` from `app_commands.rs` and its call site
   (:330-332). The app-layer `state.overlays` wall visuals should then be **derived from sim
   wall entities** (the existing sim→app overlay sync path), not injected app-side.

Design note: if walls are intended to live as sim **overlay-grid** cells rather than entities,
register them in `sim.overlay_grid` instead of spawning entities — but do it in the sim, and
hash whatever representation is chosen (`overlay_grid` is already hashed at
`world_hash.rs:430`). Either way the invariant is: **wall auto-fill is a sim mutation, never an
app-only one.** Recommend matching the existing wall-placement representation (entities via
`spawn_object`) for consistency.

### Risk / blast radius
- New sim code spawns N wall entities per placement; must respect the same placement/occupancy
  validation the single-wall path uses. Wall entities are cheap but N can be large on long
  runs — cap by the same "stop at blocker/edge" walk (already bounded by map size 0..511).
- Touches the sim→app overlay sync (walls now originate in sim). Verify the renderer still
  shows filled walls (it should, once they're real entities/overlays the existing sync draws).

### Effect on goldens
- Changes sim state on wall placement → would shift `GLOBAL_HARNESS_FINAL_HASH` **only if** the
  harness places walls (it does not) → no expected re-baseline. If a wall-exercising fixture
  exists, re-baseline it once.
- `SNAPSHOT_VERSION`: unchanged (wall entities already serialize).

### Acceptance tests
- Two independently-constructed sims apply the same `PlaceReadyBuilding(wall)` command between
  the same endpoints → identical `state_hash` AND identical set of occupied wall cells.
- Filled cells block movement / are damageable (occupancy registered), unlike today.

---

## GAP 4 — Mutable local-player identity

### Current-state evidence
- `preferred_local_owner` (`app_commands.rs:642-699`) is a **heuristic recomputed every call**:
  owner-of-selected-unit → `local_owner_override` → most-structures ranking → map houses →
  any playable owner. `resolve_owner` (:30-34) **rewrites** `local_owner_override` on every
  build/place/sell/SW action, and `cycle_local_owner` (:620-636) mutates it on demand.
- The launch session DOES seed an identity: `app_transitions.rs:266`
  `state.local_owner_override = result.initial_local_owner;`, sourced from
  `app_init.rs:696-727,1161` (`initial_local_owner`, field declared at `app_init.rs:131`).
- Problem: because `preferred_local_owner` tries **owner-of-selected-unit first**, selecting an
  ally's (or any playable house's) unit silently repoints the "local player," and every
  code path that calls `resolve_owner`/`preferred_local_owner` (production, placement, sell,
  repair, SW launch, rally) then acts as a **different house mid-match** — non-deterministic
  under lockstep and not a fixed launch-session slot.

### Exact fix
Pin local identity to a **match-scoped, immutable slot** set once at launch:
1. Add `AppState.local_player_owner: Option<String>` (or reuse an interned `local_house_id`),
   set exactly once in `app_transitions.rs:266` from `initial_local_owner`, and **never
   written again** during a match.
2. Replace the heuristic body of `preferred_local_owner` with: return
   `state.local_player_owner` verbatim (fall back to the old heuristic ONLY when it is `None`,
   i.e. dev/sandbox with no launch session). Remove the "owner of selected unit" and
   "most-structures" branches from the normal path — selection must never change identity.
3. Remove `local_owner_override` rewrites from `resolve_owner` (:30-34) and the other call
   sites; `resolve_owner` should just read the pinned slot.
4. Keep `cycle_local_owner` (:620-636) as an **explicit debug-only** path, gated behind the
   existing sandbox/debug flag (e.g. `state.sandbox_full_visibility` or a dedicated debug gate)
   so it cannot fire in a real skirmish/MP match.

### Risk / blast radius
- App-layer only — **no sim state, no hash impact, no SNAPSHOT_VERSION change.** This is a
  lockstep-correctness fix (each client must issue commands as its own fixed house).
- ~27 files reference `local_owner_override`/`preferred_local_owner` (grep list). Most are
  read-only consumers that keep working once the source is pinned; the writers to audit are
  `resolve_owner`, `cycle_local_owner`, and any other assignment to `local_owner_override`.
  Do a full grep for `local_owner_override =` before ripping it out.
- Verify sandbox/dev flows that relied on selection-follows-owner still work behind the debug
  gate.

### Effect on goldens
- None (`GLOBAL_HARNESS_FINAL_HASH`, `SNAPSHOT_VERSION` unchanged).

### Acceptance tests
- Two-client (or two-AppState) test: same launch session → both resolve the SAME, STABLE local
  owner across selecting an ally's unit, building, and selling — identity does not move.
- Debug cycle-owner is inert unless the debug gate is on.

---

## Recommended implementation order

1. **GAP 1 (hashing) FIRST — before any net work.** One commit, one documented re-baseline of
   `GLOBAL_HARNESS_FINAL_HASH`. Rationale: net/lockstep desync detection is worthless while the
   hash omits ~20 authoritative fields; landing this first means later gaps are validated by a
   complete hash. No SNAPSHOT_VERSION change.
2. **GAP 2 (particle serialize).** Independent of Gap 1's re-baseline; bumps
   `SNAPSHOT_VERSION` 27→28. Sequence after Gap 1 so the two golden/version ceremonies don't
   collide in one session.
3. **GAP 3 (wall auto-fill → sim).** Behavior/occupancy + lockstep fix; no expected golden
   move. After Gap 1 so the new sim state is hash-covered.
4. **GAP 4 (pin local owner).** App-layer only, zero hash/version impact; do alongside the net
   work it unblocks. Lowest sim risk, can land last or in parallel.

Golden/version discipline: Gaps 1 and 2 each touch the goldens/version ceremony — one session,
sequentially, not in parallel with other golden-touching work.
