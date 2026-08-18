# Two-Stream RNG — Implementation Plan (2026-05-29)

> Implements `docs/research/TWO_STREAM_RNG_DESIGN_20260529.md`.
> Authority order: binary → Ghidra (live) → `docs/research/` → `ini/`.
> Goal: replace the single `Simulation.rng: SimRng` with **two** byte-identically-seeded
> independent streams (`scenario_rng` = gamemd `Scenario->Random` @ `Scen+0x218`; `main_rng` =
> gamemd `g_MainRng` @ `0x00886B88`), route every consumer to the proven stream via intent-named
> accessors, serialize + hash BOTH streams, and prove each stream reproduces gamemd's sequence.
>
> `SimRng` itself is **not modified** — its seeding math, `index_b=0x67` start, and ranged/rejection
> draws are already byte-identical to gamemd (`src/sim/rng.rs`; `INIT_TABLE_1/2` confirmed vs
> `read_memory 0x00839644`/`0x00839694`; raw sequence pinned by `test_gamemd_raw_sequence_seed_one`).
>
> Tasks are ordered so the build stays green between them. The single field is removed only in
> Task 2 (forcing the compiler to flag every borrow site); Tasks 3–7 route each flagged site; Tasks
> 8–10 do hash/snapshot/replay; Task 11 fixes test helpers; Task 12 adds the determinism + parity
> tests. **Between Task 2 and Task 11 the crate will not fully compile** — that is intentional and
> the routing tasks are the mechanical fix. Run `cargo check` only at the Task-11 boundary and after.

---

## Task 1 — Add the two RNG fields + intent-named accessors (NO field removal yet)

**File:** `src/sim/world/mod.rs`

**1a. Add fields** beside the existing `pub rng: SimRng` (line 289). Do NOT remove `rng` yet —
keep the struct compiling so the accessors can be added and reviewed in isolation:

```rust
/// Scenario RNG — gamemd `Scenario->Random` (Scen+0x218). Drives in-object-tick
/// sim draws: scatter, sub-cell placement, smudge/destruction, particles,
/// wall/overlay damage, bridge collapse/repair, ore growth/spread, TIBTRE,
/// anim scorch/50-50, miner-dock jitter. MUST be serialized + hashed (never
/// #[serde(skip)]) or a divergence here hides from desync detection.
pub(crate) scenario_rng: SimRng,
/// Main/global RNG — gamemd `g_MainRng` (0x00886B88). Drives presentation/weapon
/// helpers (weapon spread, warhead detonate, sound variant, EBolt/laser, building
/// missile, HouseClass AI/superpower gate). No verified sim/ consumer routes here
/// today; seeded + hashed regardless so it is already in lockstep when those land.
/// MUST be serialized + hashed.
pub(crate) main_rng: SimRng,
/// Construction seed — recorded so the replay header carries the negotiated
/// g_RngSeed (not a mid-stream fingerprint). Both streams derive from it.
pub(crate) seed: u64,
```

**1b. Seed all three in `with_seed`** (line 451+). Add to the struct literal (line 459 region),
keeping the existing `rng: SimRng::new(seed)` for now:

```rust
rng: SimRng::new(seed),
scenario_rng: SimRng::new(seed),
main_rng: SimRng::new(seed),
seed,
```

Per design §4: two `SimRng::new(seed)` calls with the same seed are byte-identical by
construction (this is the same proof gamemd's two `Random__Seed(g_RngSeed)` calls give —
`decompile_function 0x0052FC20`). At the end of `with_seed`, add:

```rust
debug_assert_eq!(out.scenario_rng.state(), out.main_rng.state());
debug_assert_eq!(out.scenario_rng.state(), out.rng.state());
```

(restructure `with_seed` to bind the value to `let out = Self { … };` then return `out`, if it
currently returns the literal directly).

**1c. Add the accessor block** in `impl Simulation` (near `with_seed`):

```rust
// --- Scenario stream (gamemd Scenario->Random @ Scen+0x218) ---
pub(crate) fn scatter_rng(&mut self)      -> &mut SimRng { &mut self.scenario_rng } // bump displacement, idle/forced scatter, passenger unload exit, sell-eject
pub(crate) fn subcell_rng(&mut self)      -> &mut SimRng { &mut self.scenario_rng } // infantry sub-cell rotation, paradrop sub-cell
pub(crate) fn smudge_rng(&mut self)       -> &mut SimRng { &mut self.scenario_rng } // destruction smudge/survivor/debris, smudge type pick
pub(crate) fn wall_damage_rng(&mut self)  -> &mut SimRng { &mut self.scenario_rng } // overlay/wall damage roll
pub(crate) fn bridge_rng(&mut self)       -> &mut SimRng { &mut self.scenario_rng } // bridge collapse/repair/debris/explosion
pub(crate) fn ore_rng(&mut self)          -> &mut SimRng { &mut self.scenario_rng } // ore growth/spread queue + direction + variant, TIBTRE
pub(crate) fn anim_rng(&mut self)         -> &mut SimRng { &mut self.scenario_rng } // building damage-fire type/start-frame
pub(crate) fn particle_rng(&mut self)     -> &mut SimRng { &mut self.scenario_rng } // particle/smoke/gas/fire lifetime/offset/dir/insert
pub(crate) fn superweapon_rng(&mut self)  -> &mut SimRng { &mut self.scenario_rng } // lightning-storm scatter/bolt (YELLOW Y2)
pub(crate) fn miner_jitter_rng(&mut self) -> &mut SimRng { &mut self.scenario_rng } // dock-entry retry + unload-deploy frame jitter

// --- Main stream (gamemd g_MainRng @ 0x00886B88); no sim/ consumer wired yet ---
pub(crate) fn weapon_spread_rng(&mut self) -> &mut SimRng { &mut self.main_rng } // projectile spread X/Y, warhead detonate scatter
pub(crate) fn house_ai_rng(&mut self)      -> &mut SimRng { &mut self.main_rng } // HouseClass superpower/AI gate roll

/// Test/replay helper — reseed BOTH streams from one seed (mirrors the dual
/// Seed+clone in gamemd Init_Random_Number_System). Replaces test code that
/// did `sim.rng = SimRng::new(seed)`.
pub(crate) fn reseed_both(&mut self, seed: u64) {
    self.scenario_rng = SimRng::new(seed);
    self.main_rng = SimRng::new(seed);
    self.seed = seed;
}
```

Keep accessors distinct even though several return the same stream today (design §2.2): the
intent name is the per-consumer routing record and the grep/audit anchor.

**Verify:** `cargo check` is green (both old `rng` and new fields/accessors coexist; nothing routed
yet, no warnings beyond unused-accessor which `pub(crate)` + later use will clear). Confirm the
accessor list covers every consumer family in the routing table (Task 3–7).

---

## Task 2 — Remove the single `rng` field (forces exhaustive routing)

**File:** `src/sim/world/mod.rs`

- Delete `pub rng: SimRng` (line 289) and its doc comment.
- Delete `rng: SimRng::new(seed),` from the `with_seed` literal and the `debug_assert_eq!` line that
  compares against `self.rng`.

After this the crate WILL NOT COMPILE — every `self.rng` / `sim.rng` borrow is now an error. This
is the design's deliberate mechanism (§2.1) making the migration exhaustive. Tasks 3–11 clear the
errors. Do not run `cargo check` for green until Task 11; use `cargo check 2>&1` to enumerate the
remaining `no field rng` errors as a live checklist.

---

## Task 3 — Route the `world/mod.rs` dispatch borrows

**File:** `src/sim/world/mod.rs` — replace each `&mut self.rng` / `Some(&mut self.rng)` at the
dispatch sites with the proven accessor (design §3):

| Line | Current | Replace with | Consumer / stream |
|---|---|---|---|
| 553 | `rng: Some(&mut self.rng),` (ReduceTiberiumContext) | `rng: Some(self.ore_rng()),` | ore growth/spread — scenario (`TiberiumClass__GrowthProcessor 0x00722f00`) |
| 1008 | `&mut self.rng,` (→ `damage_wall_overlay`) | `self.wall_damage_rng(),` | wall/overlay damage — scenario (`CellClass__DestroyOverlay 0x00480cb0`) |
| 1437 | `&mut self.rng,` (→ ground-movement `tick_movement_with_grids`) | `self.scatter_rng(),` | bump/scatter + sub-cell — scenario (`UnitClass__Scatter 0x00743a50`) |
| 1831 | `&mut self.rng,` (drain pending smudge) | `self.smudge_rng(),` | destruction smudge — scenario (`BuildingClass__DestructionEffects 0x004415f0`) |
| 1843 | `&mut self.rng,` (drain combat smudge) | `self.smudge_rng(),` | same |
| 1906 | `&mut self.rng,` (tick_native_growth_driver) | `self.ore_rng(),` | ore — scenario |
| 1919 | `&mut self.rng,` (tick_native_spread_driver) | `self.ore_rng(),` | ore — scenario |
| 1932 | `&mut self.rng,` (legacy `tick_ore_growth`) | `self.ore_rng(),` | ore — scenario |
| 1944 | `&mut self.rng,` (TerrainSpawnContext::new) | `self.ore_rng(),` | TIBTRE — scenario (`TerrainClass::AI 0x0071C730`) |
| 1869 | `//     &mut self.rng,` (DORMANT idle-scatter, commented) | `//     self.scatter_rng(),` | update the comment to keep the dormant block routable when re-enabled |

**Borrow-checker note (design §2.2):** the accessor returns `&mut SimRng` borrowing only the one
field, so each call drops in where `&mut self.rng` was, including inside the larger struct-literal
borrows (ReduceTiberiumContext at 545, TerrainSpawnContext at 1940). Confirm none of these sites
simultaneously needs a *different* RNG accessor in the same expression (none does — each is
single-stream).

**Verify:** `cargo check 2>&1` shows the `mod.rs:553/1008/1437/1831/1843/1906/1919/1932/1944` errors
cleared; remaining errors are now only in the leaf-borrow files (Tasks 4–7).

---

## Task 4 — Route the bump/scatter/passenger/sell consumers (scenario)

Leaf signatures stay `rng: &mut SimRng`; only the borrow at the `sim.rng` site changes. The
`bump_crush.rs:383/740` and `scatter.rs:123` draws are reached via the `mod.rs:1437` dispatch
(already routed in Task 3) — no edit needed there. Edit the direct-`sim.rng` sites:

| File:line | Current | Replace with | Stream / evidence |
|---|---|---|---|
| `src/sim/passenger.rs:887` | `sim.rng.next_u32()` | `sim.scatter_rng().next_u32()` | scenario (scatter/exit family). **Preserve `% 8` modulo (Y4)** |
| `src/sim/passenger.rs:1042` | `sim.rng.next_u32()` | `sim.scatter_rng().next_u32()` | scenario. **Preserve `% 8` (Y4)** |
| `src/sim/production/production_sell.rs:394` | `sim.rng.next_range_u32_inclusive(0, 4)` | `sim.scatter_rng().next_range_u32_inclusive(0, 4)` | scenario (`BuildingClass__SpawnSurvivors 0x00442d90`) |
| `src/sim/aircraft/drop_payload.rs:184` | `&mut sim.rng,` (→ `allocate_sub_cell_with_preference`) | `sim.subcell_rng(),` | scenario PROVEN (`CellClass__PlaceInfantryInCell` `Scen+0x218`, `disassemble_function 0x0048139A`) |

**Do NOT** change `% 8` to `next_range_u32(8)` at the passenger sites — that is a *separate*
pre-existing DRIFT (design §8 Y4); changing it here would shift the stream's draw count/values.
Surface it separately.

**Verify:** `cargo check 2>&1` — passenger/production_sell/drop_payload errors cleared.

---

## Task 5 — Route miner-dock jitter + bridge consumers (scenario)

**File:** `src/sim/miner/miner_dock_sequence.rs`

| Line | Current | Replace with |
|---|---|---|
| 82 | `sim`\n`.rng`\n`.next_range_u32_inclusive(0, ENTER_RETRY_JITTER_MAX_FRAMES)` | `sim.miner_jitter_rng().next_range_u32_inclusive(0, ENTER_RETRY_JITTER_MAX_FRAMES)` |
| 1017 | `sim`\n`.rng`\n`.next_range_u32_inclusive(0, MISSION_DEPLOY_UNLOAD_JITTER_MAX_FRAMES)` | `sim.miner_jitter_rng().next_range_u32_inclusive(0, MISSION_DEPLOY_UNLOAD_JITTER_MAX_FRAMES)` |

**File:** `src/sim/world/bridge_orchestrator.rs` — all bridge draws are scenario
(`CellClass__BlowUpBridge 0x0047dd70`):

| Line | Current | Replace with |
|---|---|---|
| 180 | `rng: &mut sim.rng,` (BridgePresentationContext) | `rng: &mut sim.scenario_rng,` **(direct field, NOT `bridge_rng()` — see borrow note)** |
| 1178 | `sim`\n`.rng` (95% outer gate, `next_range_u32_inclusive`) | `sim.bridge_rng()` |
| 1185 | `bridge_jittered_subcells(&mut sim.rng)` | `bridge_jittered_subcells(sim.bridge_rng())` |
| 1196 | `sim`\n`.rng` (metallic 50% gate) | `sim.bridge_rng()` |
| 1203 | `sim.rng.next_range_u32(metallic_count)` | `sim.bridge_rng().next_range_u32(metallic_count)` |
| 1227 | `sim.rng.next_range_u32_inclusive(1, 5)` | `sim.bridge_rng().next_range_u32_inclusive(1, 5)` |
| 1228 | `sim.rng.next_range_u32(explosion_count)` | `sim.bridge_rng().next_range_u32(explosion_count)` |
| 1395 | `let rng = &mut sim.rng;` (per-path strength gate, design row `:1419`) | `let rng = &mut sim.scenario_rng;` **(direct field, NOT `bridge_rng()` — see borrow note)** |

**BORROW NOTE (reviewer-confirmed blocker — sites 180 & 1395 ONLY).** Lines 180 and 1395 sit
inside live `sim.bridge_state.as_mut()` borrows (180 also co-borrows `sim.world_effects`,
`sim.bridge_explosions`, `sim.effect_frame_counts`, `sim.bridge_anim_sounds`). The `bridge_rng()`
*method* borrows all of `sim` → E0499. Use the **direct disjoint field** `&mut sim.scenario_rng`
at these two sites (same form as `world_orders.rs:379`). The other bridge sites
(1178/1185/1196/1203/1227/1228) sit AFTER the context block closes with no live `bs`, so the
`bridge_rng()` accessor is fine there. Verified by reviewer pass; no other accessor-vs-live-borrow
conflict exists anywhere in the migration.

The `1261/1262/1264` sites operate on `presentation.rng` (already routed via line 180's context
construction) — no edit. `walker.rs:412/420` and `bridge_state/mod.rs:1335` take `rng: &mut SimRng`
params fed from the context/dispatch — no edit. `world_orders.rs:379`:

**File:** `src/sim/world/world_orders.rs:379`
- `bs.repair_bridge_from_engineer_scan(&scan, &mut self.rng, terrain)` → `… self.bridge_rng(), terrain)`
  (need to check `bs`/`self` split-borrow: `bs` is `self.bridge_state.as_mut()` taken earlier in the
  fn — if `bs` holds a mutable borrow of `self` that conflicts with `self.bridge_rng()`, restructure
  to take the `&mut SimRng` before borrowing `bs`, or pass `&mut self.scenario_rng` directly here
  since `bridge_state` and `scenario_rng` are disjoint fields. Prefer the disjoint-field direct
  borrow if `bs` is a long-lived `&mut self.bridge_state` borrow.)

**Verify:** `cargo check 2>&1` — miner/bridge errors cleared. Pay attention to any split-borrow
error at `world_orders.rs:379` and resolve with the disjoint-field form.

---

## Task 6 — Route ore-growth, terrain-spawn, smudge, overlay leaf-driven sites

These are all reached via dispatch sites already routed in Task 3 (`ore_rng()`, `smudge_rng()`,
`wall_damage_rng()`). The leaf functions in the files below take `rng: &mut SimRng` and need NO
edit:
- `src/sim/ore_growth.rs` (363, 435, 525, 559, 583, 693, 863, 1192, 1274, 1463, 1499) — leaf params.
- `src/sim/terrain_spawn.rs` (78, 325, 343, 382, 587) — fed by `TerrainSpawnContext.rng` (routed at
  `mod.rs:1944`).
- `src/sim/overlay_grid.rs` (330, 343, 359, 381) — `damage_wall_overlay`/`damage_wall_recursive`
  take `rng: &mut SimRng` (routed at `mod.rs:1008`).
- `src/sim/combat/smudge_dispatch.rs` (38, 128, 212, 239, 240, 241, 297, 359) — take `rng: &mut
  SimRng` (routed at `mod.rs:1831/1843`).
- `src/sim/tiberium/mod.rs:113` — forwards `ctx.rng.as_deref_mut()` (ctx routed at `mod.rs:553`).
- `src/sim/smudge_grid.rs:285` — confirm its caller passes a routed `smudge_rng()`; if it has its own
  `sim.rng` borrow, route to `smudge_rng()`.

**Action:** Verify by `cargo check 2>&1` that NO `no field rng` errors remain in these files. If any
file still references `sim.rng` directly (e.g. `smudge_grid.rs` has a direct borrow not covered by a
context), route it to the matching scenario accessor (`smudge_rng()` / `ore_rng()`).

**Verify:** `cargo check 2>&1` — ore/terrain/overlay/smudge/tiberium errors cleared.

---

## Task 7 — Route particle, superweapon, and app-layer (anim) consumers

**File:** `src/sim/particles/{spawn,smoke,gas,fire}.rs` — these borrow `sim.rng` directly at the
dispatch (the `tick_particle_systems(sim, …)` entry, `system_ai.rs:81`, calls into them with
`&mut self`). Route every direct `sim.rng` / `&mut sim.rng` borrow in these files to
`sim.particle_rng()`:
- `smoke.rs:53,54,77,88,89,100,282,377`
- `gas.rs:75,86,87,98,288,333,367`
- `fire.rs:191,210,266,269,296,302,331,361`
- `spawn.rs:96,99,229` — these are inside `fn spawn_particle(…, rng: &mut SimRng)` leaf params; the
  caller in `spawn.rs`/`system_ai.rs` that supplies `&mut sim.rng` is the routing site — route that
  caller to `sim.particle_rng()`. (Confirm via `cargo check` which exact lines error.)

Leaf helpers like `make_child(spec, pt, rng)`, `symmetric_offset(r, rng)`, `tick_particle(p, pt,
frame, rng)`, `make_particle(…)` keep `rng: &mut SimRng` — only the `sim.rng` borrow at their call
site changes to `sim.particle_rng()`.

**File:** `src/sim/superweapon/lightning_storm.rs` (190, 191, 202, 203, 213) — route each
`sim.rng.…` to `sim.superweapon_rng().…`. (PROVEN scenario: `LightningStorm__GroundStrike
0x0053a300` + `LightningStorm__Process 0x0053a6c0` draw only from `Scen+0x218`, zero g_MainRng —
`disassemble_function 0x0053a300`/`0x0053a6c0`, 2026-05-29.)

**File:** `src/app_building_anim.rs:183` — `&mut sim.rng,` (→ `create_damage_fire_slot_anims`) →
`sim.anim_rng(),`. (App layer, design §6 caveat: scenario per `AnimClass`, but if a tick-rate/
logic-vector refactor relocates these draws, scenario draw-order must be re-validated.)

**Verify:** `cargo check 2>&1` — particle/superweapon/app_building_anim errors cleared. Only the
hash/snapshot/replay and test-helper errors should remain.

---

## Task 8 — Hash BOTH streams in the world hash

**File:** `src/sim/world/world_hash.rs:39`

Replace:
```rust
self.rng.hash_state(&mut hasher);
```
with (fixed, documented order — scenario first, then main):
```rust
// Hash BOTH RNG streams in a fixed order. Order is part of the hash contract
// and must never change. Hashing only one stream would let a divergence in the
// other produce identical hashes on two desynced clients (desync detector goes
// blind exactly where the two-stream split matters).
self.scenario_rng.hash_state(&mut hasher);
self.main_rng.hash_state(&mut hasher);
```

Update the doc comment at `world_hash.rs:31` ("Hashes tick, RNG, …") to say "both RNG streams".

**Verify:** `cargo check 2>&1` — `world_hash.rs:39` error cleared.

---

## Task 9 — Snapshot: bump version (serde handles both fields automatically)

**File:** `src/sim/snapshot.rs:16`

- Bump `const SNAPSHOT_VERSION: u32 = 11;` → `12`. Rationale (design §5): one field → two changes
  the bincode positional layout; an old single-`rng` blob read into the two-field struct would
  mis-deserialize everything after it. Old saves/replays must be rejected cleanly (the existing
  `version != SNAPSHOT_VERSION` guards at lines 109/122 already do this). Accepted save/replay-format
  break.
- **No per-field serde edit.** `scenario_rng`/`main_rng` are plain (non-`#[serde(skip)]`) fields, so
  bincode persists both automatically (the whole `Simulation` is serialized via
  `GameSnapshotRef.sim`). The `seed: u64` field is also persisted automatically.
- **Audit:** confirm neither new field carries `#[serde(skip)]` (the field doc comments state this;
  mirror in any future refactor).

**Verify:** `cargo check 2>&1` — no new errors; snapshot version guards unchanged.

---

## Task 10 — Replay seed site (latent bug the split surfaces)

**File:** `src/app_sim_tick.rs:260`

Current `seed: sim.rng.state(),` no longer compiles (field gone) AND was already wrong —
`state()` is a mid-stream fingerprint, not the construction seed (design §5). Replace with the
recorded construction seed:
```rust
seed: sim.seed,
```
The replay header carries one negotiated `g_RngSeed`; both streams derive from it at load via
`with_seed(header.seed)`. (Confirm the replay-load path reconstructs the sim with
`Simulation::with_seed(header.seed)` or equivalent so both streams reseed identically — if it uses a
different reconstruction, route it through `with_seed`/`reseed_both`.)

**Verify:** `cargo check 2>&1` — `app_sim_tick.rs:260` error cleared.

---

## Task 11 — Fix test helpers that did `sim.rng = SimRng::new(seed)` (restores green build)

Replace every `sim.rng = SimRng::new(seed)` with `sim.reseed_both(seed)` (Task 1c helper), and every
`sim.rng.state()` read with the appropriate stream's `state()` (`sim.scenario_rng.state()` for the
sim consumers under test — all current test consumers are scenario-routed). Sites (from grep):

- `src/sim/combat/combat_tests.rs:1779` — `sim.rng = …new(seed)` → `sim.reseed_both(seed)`.
- `src/sim/world/world_tests.rs:1521, 1561, 1667` — `= …new(seed)` → `reseed_both`; `:1701`
  `sim.rng.state()` → `sim.scenario_rng.state()`.
- `src/sim/world/bridge_orchestrator.rs:1826, 1869, 1906` — `= …new(seed)` → `reseed_both`;
  `1830/1857/1930/1931/1942` `sim.rng[.state()]` reads → `sim.scenario_rng[.state()]` (bridge is
  scenario-routed). `:1930` `sim.rng = …new(7)` → `sim.reseed_both(7)`.
- `src/sim/miner/miner_tests.rs:4088, 4108` — `sim.rng.state()` → `sim.scenario_rng.state()`
  (miner jitter is scenario-routed).
- `src/sim/production/production_sell.rs:1165, 1181, 1239, 1250, 1269, 1286, 1310, 1325` —
  `sim.rng.state()` / `sim.rng.clone()` reads → `sim.scenario_rng.…` (sell-eject is scenario-routed).

Preserve each test's single-stream intent: a test asserting "consumed N draws" or "consumed nothing"
must read the SAME stream the routed consumer now uses (scenario for all current tests).

**Verify:** `cargo check` is GREEN. `cargo test -p <crate> --lib sim::` passes (no behavioral change
expected — every consumer that previously drew from `rng` now draws from `scenario_rng`, which is
seeded identically, so existing single-stream tests still pass).

---

## Task 12 — Determinism + gamemd-parity tests (the burden-of-proof gate)

**File:** `src/sim/rng.rs` (`mod tests`) and/or a new `src/sim/world/rng_routing_tests.rs` module
(declare in `world/mod.rs`). Add (design §7):

1. **Seed-equality invariant.** After `Simulation::with_seed(s)`: `scenario_rng.state() ==
   main_rng.state()`; both have `index_b == 0x67` (add a `pub(crate) fn index_b(&self) -> i32` test
   accessor on `SimRng` if not exposable, or assert via the first-draw sequence). Seeds: 0, 1,
   `DEFAULT_SIM_SEED`, `u32::MAX as u64`. Proves §4.
2. **Independence.** Draw N from `scatter_rng()` only; assert `main_rng.state()` == fresh
   `SimRng::new(s).state()` (unchanged) and `scenario_rng.state()` != fresh. Symmetric: draw from
   `weapon_spread_rng()` only, assert scenario unchanged.
3. **Per-stream gamemd raw-sequence pin.** Both `scenario_rng` and `main_rng` from seed 1 produce
   `0x78B76ED5, 0x275D74AE, 0xDA63B931` independently (extends `test_gamemd_raw_sequence_seed_one`).
   Proves the clone is exact.
4. **Routing regression (central guard).** One test per accessor: draw once through it, assert ONLY
   the intended field advanced (the other stream's `state()` unchanged). Catches a future edit
   silently re-pointing e.g. `bridge_rng()` at `main_rng`. This is the guard against the dominant
   silent-misroute failure (design §2).
5. **Ground-truth value parity (Ghidra — REQUIRED).** For ≥1 scenario consumer (wall-damage
   `RandomRanged(0,strength)` or bridge variant `RandomRanged(0,3)`) and ≥1 main consumer (weapon
   spread), capture gamemd's emitted RANGED sequence for a fixed post-init seed via
   `emulate_function 0x0065C7E0` (`Random__RandomRanged`) and assert the Rust stream matches
   value-for-value. Without this the tests prove internal consistency, not gamemd parity. (This is a
   research+test step; capture the emulated values during plan execution and bake them as expected
   constants with an inline `emulate_function 0x0065C7E0` citation.)
6. **Hash coverage.** Draw from `main_rng` only → assert `state_hash()` changes; same for
   `scenario_rng`. Proves §5 (neither stream silently excluded from the hash).
7. **Snapshot round-trip.** Advance scenario and main a DIFFERENT number of draws, save → load,
   assert both restore to identical state (proves serde persists both independently). Use the
   existing snapshot save/load test harness.
8. **Determinism end-to-end.** Two sims `with_seed(s)`, advance K ticks with identical inputs,
   assert equal `state_hash()` every tick (existing determinism test should still pass; add a variant
   that asserts both streams' `state()` match between the two sims).

**Verify:** `cargo test` green; tests 1–8 present and passing. Test 5's expected constants carry the
`emulate_function 0x0065C7E0` citation.

---

## YELLOW status (adjusted 2026-05-29 — design §8): zero remaining pre-ship gates

- **Y1 — CLOSED (PROVEN scenario).** `CellClass__PlaceInfantryInCell 0x00481180` at `0x0048138a`
  draws `RandomRanged(0,3)` from `Scen+0x218` (`disassemble_function 0x0048139A`). Covers
  `bump_crush.rs:383` (via `mod.rs:1437`) + `drop_payload.rs:184`. No gate.
- **Y2 — CLOSED (PROVEN scenario).** `LightningStorm__GroundStrike 0x0053a300` +
  `LightningStorm__Process 0x0053a6c0` draw only from `Scen+0x218`, zero g_MainRng
  (`disassemble_function 0x0053a300`/`0x0053a6c0`). Covers `lightning_storm.rs:190-213`. No gate.
- **Y3 — FUTURE, not this change.** Warhead detonate scatter is a main-stream consumer not yet
  ported; `main_rng` stays unadvanced until it lands. Verify its draw count + `g_MainRng 0x886B88`
  read when warhead RNG is ported. Not a blocker for the two-stream split.
- **Y4 — SEPARATE bug, out of scope.** `passenger.rs:887/1042` `% 8` modulo bias is a pre-existing
  DRIFT unrelated to routing. This plan preserves the draw method verbatim; spin Y4 off as its own
  task (do NOT change `% 8` here — it shifts the stream's draw count).
- **Y5 — scope statement, not a defect.** No main consumer exists in the port, so `main_rng`
  seeded+hashed-but-unadvanced is the faithful state. This change is sufficient for every RNG
  consumer in `sim/` today; main-stream gameplay parity arrives with the weapon/audio/AI port.

**Result: every consumer in scope is PROVEN-routed. No pre-implementation Ghidra gate remains.**

## Merge-order caveat (design §6)

Land this two-stream split FIRST (smallest, most local, no draw reordering), THEN rebase
tick-rate / logic-vector (`world/mod.rs:553`, `logic_vector.rs`) onto a correct two-stream baseline.
Re-baseline golden-hash fixtures ONCE after all three land. Do not reorder draws as part of this
change.

## Files touched (summary)

- `src/sim/world/mod.rs` — two fields + `seed` + accessors + `with_seed` + `reseed_both`; route
  dispatch borrows 553/1008/1437/1831/1843/1906/1919/1932/1944 (+1869 comment).
- `src/sim/world/world_hash.rs` — hash both streams.
- `src/sim/snapshot.rs` — `SNAPSHOT_VERSION` 11→12.
- `src/app_sim_tick.rs` — replay seed `sim.seed`.
- `src/app_building_anim.rs` — `anim_rng()`.
- `src/sim/passenger.rs`, `production/production_sell.rs`, `aircraft/drop_payload.rs`,
  `miner/miner_dock_sequence.rs`, `world/bridge_orchestrator.rs`, `world/world_orders.rs`,
  `superweapon/lightning_storm.rs`, `particles/{spawn,smoke,gas,fire}.rs`, `smudge_grid.rs` (if
  direct borrow) — route to the matching scenario accessor.
- Test helpers: `combat_tests.rs`, `world_tests.rs`, `bridge_orchestrator.rs` (tests),
  `miner_tests.rs`, `production_sell.rs` (tests) — `reseed_both` + per-stream `state()`.
- New routing/determinism tests (Task 12).
- **No change to `src/sim/rng.rs`** except adding tests (and an optional `index_b()` test accessor).

---

## Plan Review (2026-05-29, reviewer pass — codebase verified, no code edited)

**Verdict: ISSUES — one blocking build-breaker (Task 5), rest PASS.**

### Verified correct against current code
- Struct `Simulation` @ `world/mod.rs:266`; `pub rng: SimRng` @ **line 289** (doc comment is "Single
  explicit deterministic PRNG stream…", not the text Task 1a quotes — cosmetic, the field/line are right).
- `with_seed` @ `world/mod.rs:451` returns the struct literal directly (`Self { … }`); `rng: SimRng::new(seed)`
  @ line 459. Task 1b's "restructure to `let out = …`" is REQUIRED for the debug_assert — correctly flagged.
- All 10 `mod.rs` dispatch borrows confirmed at the exact cited lines: 553, 1008, 1437, 1831, 1843, 1869
  (commented), 1906, 1919, 1932, 1944. Routing assignments are sound.
- Leaf/context sites confirmed: `passenger.rs:887,1042` (`% 8` present — preserve note correct),
  `production_sell.rs:394`, `drop_payload.rs:184`, `miner_dock_sequence.rs:82,1017`,
  `world_orders.rs:379`, `world_hash.rs:39` (+ doc @ 31), `app_sim_tick.rs:260`, `app_building_anim.rs:183`,
  `lightning_storm.rs:190,191,202,203,213`, particles `smoke/gas/fire.rs` (all cited lines exist),
  `bridge_orchestrator.rs:180,1178,1185,1196,1203,1227,1228,1395` (1178/1196 are multi-line `sim\n.rng` — correct).
- `terrain_spawn.rs` and `tiberium/mod.rs` use `ctx.rng` (context-routed) — Task 6 "no edit" is correct.
- **Call-site coverage is complete.** Repo-wide `.rng` = 84 occurrences across 19 files; every non-test
  consumer is assigned a stream. No missed call site.
- SimRng (`rng.rs`) derives `Clone + Serialize + Deserialize`; `state/new/next_u32/next_range_u32/
  next_range_u32_inclusive/hash_state` all exist; `index_b` private (Task 12 accessor genuinely needed);
  `RNG_INDEX_B_SEED = 0x67`. No `#[serde(skip)]` — Task 9 (auto-persist both fields, bump 11→12) is correct;
  `SNAPSHOT_VERSION = 11` @ line 16, guards @ 109/122 confirmed.
- No pre-existing `seed` field on `Simulation` — adding it is safe (no collision).
- world_orders.rs:379 split-borrow: `bs = self.bridge_state.as_mut()` held live across the call. Plan's fix
  (`&mut self.scenario_rng` direct disjoint field, NOT the `bridge_rng()` method) is CORRECT and necessary.

### BLOCKING (Task 5 — will not compile as written)
- **`bridge_orchestrator.rs:180`** — `rng: &mut sim.rng` sits in a `BridgePresentationContext` literal that
  simultaneously borrows `sim.world_effects`, `sim.bridge_explosions`, `sim.effect_frame_counts`,
  `sim.bridge_anim_sounds`, AND a live `bs = sim.bridge_state.as_mut()`. The plan says replace with
  `rng: sim.bridge_rng()`, but the accessor method borrows ALL of `sim` and conflicts with those 5 live
  borrows → E0499. **Fix: use `rng: &mut sim.scenario_rng` (direct disjoint field), same form the plan
  already prescribes for world_orders.rs:379.**
- **`bridge_orchestrator.rs:1395`** — `let rng = &mut sim.rng;` is held with a live
  `bridge_state = sim.bridge_state.as_mut()` (line 1391). `sim.bridge_rng()` conflicts → E0499.
  **Fix: `let rng = &mut sim.scenario_rng;`.**
- These two are the ONLY accessor-vs-live-borrow conflicts. Verified the accessor IS safe at all other
  routed sites (particles: `sys` is a separate `&mut ParticleSystem` param, not from `sim`;
  lightning_storm/app_building_anim/passenger/production_sell/drop_payload: no conflicting live `sim` borrow).

### Non-blocking notes
- Task 5's prose already half-anticipates this ("pass `&mut self.scenario_rng` directly … if `bs` is a
  long-lived borrow") but only for world_orders.rs:379; the same reasoning must be applied to bridge sites
  180 and 1395. The other bridge sites (1178/1185/1196/1203/1227/1228) sit AFTER the context block closes
  and have no live `bs`, so the `bridge_rng()` accessor is fine there.
- Mid-sequence build state matches the plan's stated contract (red between Task 2 and Task 11) — intentional,
  acceptable.
- YELLOW carryover (Y1 sub-cell, Y2 lightning-storm, Y3 warhead) is honestly flagged and routes are gated
  on pre-ship Ghidra verification — not resolved in this plan but correctly surfaced, not silently shipped.
