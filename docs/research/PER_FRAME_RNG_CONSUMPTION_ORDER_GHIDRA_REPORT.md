# Per-Frame RNG Consumption Order — Ghidra Research Report

**Date:** 2026-05-28  
**Target:** Per-frame RNG draw order across subsystems during one `LogicClass::PerTickUpdate` (`0x0055AFB0`) for determinism-critical lockstep design.  
**Investigation mode:** `/re-swarm` slot 3, read-only + doc assembly  
**Active in YR:** Yes — every multiplayer game relies on this order being identical across clients.  
**Confidence:** High for stream identity (which instance), phase ordering, and per-consumer draw counts where already verified. Medium for cross-consumer relative ordering within the main live-object vector (depends on entity iteration order).

---

## 0. Investigation Contract

**Target question:** Which subsystems draw from `g_MainRng` or `Scen->Random` during one `LogicClass::PerTickUpdate`, in what frame-phase order, how many draws per activation, and via which helper (`Random__Next` raw vs `Random__RandomRanged`)?

**Non-goals:** RNG algorithm/seeding (slot 2), sync checksum mechanics (slot 1), desync comparison handler (slot 4), AI internals, MP network handshake details.

**Evidence needed to mark COMPLETE:** Ordered ledger with stream identity and draw counts for all known consumers; active-in-YR classification for each; Rust `SimRng` comparison; at least one handoff item with test name and risk.

**Stop conditions:** Stop when all known RNG consumers are placed in the frame-phase skeleton from the anchor doc. Do not discover new consumers by brute-force Ghidra xref — cite existing per-system docs.

---

## 1. Overview — Stream Identity

Two streams are consumed during a normal YR gameplay frame. Both start from the same seed but diverge immediately:

- **`g_MainRng` @ `0x00886B88`** (static BSS) — draws for combat, damage, sound variant, effects, particles, ore growth, TIBTRE probability, building destruction smudges, wall damage, lightning storm, Tesla bolt, laser jitter, and building-missile jitter. This is the dominant game-logic stream.
- **`Scen->Random` @ `Scen+0x218`** (heap pointer `0x00A8B230`) — draws for infantry scatter direction, unit scatter direction, infantry sub-cell placement rotation, and the conditional `HouseClass::Update` roll (`RandomRanged(0,2)`). This stream is serialized with save-state.
- **`g_MapGenRng` @ `0x00ABE890`** — random-map generator only; never consumed during a live skirmish tick.

**Stream identity classification is CRITICAL for Rust.** Rust currently has a single `SimRng` for all draws. A two-stream split is required for faithful determinism: whichever callers gamemd routes to `Scen->Random` must draw from a separate stream in Rust.

Evidence: `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1/§3.2; verified via assembly at `HouseClass__Update 0x004F887D` (`MOV ECX, 0x886B88; CALL 0x0065C7E0`) for `g_MainRng`, and `InfantryClass__Scatter 0x0051D2AC` (`LEA ECX,[Scen+0x218]`) for `Scen->Random`. See also `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2/§3.3 and `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §2.

---

## 2. Frame-Phase Ordering Skeleton

The frame phase order is drawn from `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md` and `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md` (both verified against live Ghidra decompile of `0x0055AFB0`).

### 2.1 Pre-Object-Vector Phases (Orders 1–20 in anchor doc)

| Frame order | Binary range / evidence | Subsystem | RNG consumed? | Stream | Helper / amount | Active in YR |
|---:|---|---|---|---|---|---|
| 1–4 | `0x0055AFBD..0x0055B1D8` | Scenario cell-action timer loop | No direct RNG in the loop driver. Individual action handlers may draw — unverified. | — | — | Yes |
| 5 | `0x0055B200..0x0055B29A` | Rules-gated tiberium precursor `FUN_004ACAC0` | **Likely yes** inside callee (unresolved FUN_) | Unverified | — | Conditional on `Rules+0x17F0 != 0` |
| 6 | `0x0055B29A..0x0055B2AD` | `MapClass::RecalcBridgeShroudFlags` (every 120 frames) | No — deterministic cell scan | — | — | Yes, periodic |
| 7 | `0x0055B2B8..0x0055B33D` | TS fog branch `FUN_004ACBC0` | Unverified | — | — | **Conditional; off in standard YR** (requires `SpecialFlags & 0x1000`) |
| 8 | `0x0055B33D..0x0055B4D7` | Tiberium ambient transition helpers | Unverified | — | — | Conditional on counters |
| **9** | **`0x0055B4D7`** | **`TiberiumClass::GrowthDriver_AllTypes`** | **Yes** | **`g_MainRng`** | `Random__Next`, raw modulo, **one draw** for batch count per type when heap is non-empty and `GrowthPercentage > 0` | **Yes** |
| **10** | **`0x0055B4DC`** | **`TiberiumClass::SpreadDriver_AllTypes`** | **Yes** | **`g_MainRng` / `Scen->Random` (see §3.5)** | `RandomRanged(0,7)` once for direction, then optionally `RandomRanged(0,11)` for overlay variant per empty-flat placement | **Yes** |
| 11 | `0x0055B4E1` | `BombClass::UpdateAll` | Unverified (C4/bomb path) | — | — | Yes |
| 12 | `0x0055B4EB` | `FUN_0054E4D0` (30-frame scripted-action queue) | Unverified | — | — | Yes |
| 13–14 | `0x0055B4F5..0x0055B59F` | TeamClass temp-vector AI (copied-count, not live-vector) | Unverified; AI deferred project-wide | — | — | Yes |
| 15 | `0x0055B5A1..0x0055B5BC` | DiskLaserClass reverse loop `vtable+0x5C` | Unverified | — | — | Yes |
| 16 | `0x0055B5BE` | `FUN_005FF390` object/FX reaper | No direct RNG expected | — | — | Yes |
| 17 | `0x0055B5C3` | `LaserDrawClass::UpdateAllAI` | **Yes** — laser jitter via `g_MainRng` (`RadBeam__DrawAndTickAll`) | **`g_MainRng`** | `Random__Next` or `RandomRanged`, exact draw count per beam unverified in this slot | Yes |
| **18** | **`0x0055B5C8`** | **`LightningStorm::Process`** | **Yes** | **`g_MainRng`** | Multiple draws per bolt: direction, target selection. See `LIGHTNING_STORM` docs. Active when storm is running | Conditional on active storm |
| 19 | `0x0055B5CD..0x0055B5E8` | RadSiteClass reverse loop | Unverified | — | — | Yes |
| 20 | `0x0055B5EC..0x0055B5F6` | `FUN_00554D50` light drain + `EMPulseClass::UpdateAll` | Unverified (EMP subsystem) | — | — | Yes |

### 2.2 Main Live-Object Vector (Order 21)

Binary: `0x0055B5FB..0x0055B619`. Forward, live-count reload after each `vtable+0x5C` call. This is where the bulk of per-unit/per-structure per-tick RNG draws occur.

**Within-vector ordering is determined by the entity's position in `LogicClass+0x04/+0x10` array** (insertion order, typically creation order). This must match Rust's BTreeMap iteration order for lockstep determinism. The two do not currently match — Rust uses `BTreeMap<u64, GameEntity>` sorted by entity ID, while gamemd uses a flat dynamic vector in insertion order.

For each object in this vector, per-tick draws (all `g_MainRng` unless noted) include:

| Consumer | Stream | Helper | Draws | Active in YR | Evidence |
|---|---|---|---|---|---|
| `TechnoClass::ReceiveDamage` scatter/damage | Mixed: `FUN_0049F420` uses `Scen->Random`; other verified damage rolls use `g_MainRng` | Raw `Next` for `FUN_0049F420`; ranged helpers for damage rolls | Data-dependent: scatter X/Y jitter via `FUN_0049F420` consumes 1 raw Scenario draw for angle, plus separately routed warhead/damage draws | Yes | Direct assembly `0x0049F423..0x0049F43F`; `RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §4.4; `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| `BulletClass::Detonate` / `BulletDetonation` cluster scatter | Mixed: two conditional top-of-Detonate warhead-property rolls use `g_MainRng`; cluster radius and `FUN_0049F420` direction use `Scen->Random` | Scenario `RandomRanged(0x100,0x200)` followed by one raw Scenario draw for direction; separately routed conditional main draws | Data-dependent; each live cluster continuation consumes the Scenario radius+direction pair | Yes | Direct assembly `0x00469042..0x00469067`, `0x004690E8..0x0046912B`; `RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §4.4 |
| `BuildingClass::DestructionEffects` center smudge | `Scen->Random` | `RandomRanged(0,W-2)` discard, `RandomRanged(0,H-2)` discard, `RandomRanged(0,99)` roll | 1–3 depending on foundation size | Yes | `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2, assembly `0x004417E4..0x00441819` |
| `BuildingClass::SpawnSurvivors` per-cell smudge | `Scen->Random` (per `SMUDGE_RNG_CLASSIFICATION.md` §2, assembly `0x004432B0..0x004432BF`) | `RandomRanged(0,99)` + 1 raw draw (angle) per passable foundation cell | 0 or 2 per cell | Yes | `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.3, assembly `0x0044329B..0x004433EF` |
| `AnimClass::Start` scorch/crater 50/50 | `Scen->Random` (assembly `0x0042507A`) | `RandomRanged(0,0x7FFFFFFE)` | 1 per Scorch+Crater anim | Yes (when both flags set) | `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.1 |
| `CellClass::DestroyOverlay` wall damage | `g_MainRng` (needs stream verification — YELLOW) | `RandomRanged(0, Strength)` | 1 per wall hit below strength threshold | Yes | `WALL_DAMAGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3 |
| `ParticleClass::Constructor` lifetime | `g_MainRng` | `Random__Next` raw, abs modulo `MaxEC` | 1 per particle spawn | Yes | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.1, assembly `0x0062B842..0x0062B889` |
| `ParticleClass::AI_Fire` jitter | `g_MainRng` | `Random__Next` raw, `% 10 - 5` | 1 per fire particle per tick | Yes | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.4, assembly `0x0062CB41..0x0062CB51` |
| `ParticleSystemClass::AI_Smoke` periodic spawn offsets | `g_MainRng` | `Random__Next` twice (Y then X, `% SpawnRadius+1`) | 2 per spawn cadence tick | Yes (for `Spawns=yes` smoke) | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2, assembly `0x0062F0AC..0x0062F0D5` |
| `SpawnParticleWithInsert` fire insertion shuffle | `g_MainRng` | `Random__Next`, abs `% actual_range` | 1 per fire system AI tick | Yes | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.5, assembly `0x0062E590..0x0062E59D` |
| `ParticleClass::AI_Gas` random drift | `g_MainRng` | 1 gate draw (`&7`), 1 axis draw, 1 magnitude draw (`%3-1`) | 0 or 3 per gas particle per even tick | Yes | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.6, assembly `0x0062BD93..0x0062BDF1` |
| `ParticleClass::AI_Smoke` per-particle drift | `g_MainRng` | 1 gate draw (`&3`), 1 axis, 1 magnitude | 0 or 3 per smoke particle per odd tick | Yes | `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.6, assembly `0x0062C55E..0x0062C5B1` |
| `UnitClass::Scatter` direction jitter | **`Scen->Random`** | `RandomRanged(0,2)-1` (non-null threat coord); conditional `RandomRanged(1,4)` (tow-target gate) | 1 or 2 depending on state | Yes (conditional on scatter trigger) | `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2, assembly `0x00743DC5`, `0x00743D2B` |
| `InfantryClass::Scatter` direction | **`Scen->Random`** | `RandomRanged(0,4)` twice (two direction paths) | 1 per scatter invocation | Yes | `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.3, assembly `0x0051D2BA`, `0x0051D385` |
| `CellClass::PlaceInfantryInCell` sub-cell rotation | **`Scen->Random`** | `RandomRanged(0,3)` | 1 per infantry placement into quadrant 0 | Yes | `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.4, assembly `0x0048139A` |
| `FootClass::AI` idle scatter (every 64 frames when eligible) | **`Scen->Random`** (virtual Scatter call; dispatches to per-class) | `RandomRanged(0,4)` via InfantryClass or `RandomRanged(0,2)-1` via UnitClass | 1 per idle scatter trigger | Yes (dormant in current Rust) | `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.5, assembly `0x004DAE59` |
| `HouseClass::Update` cell-state roll | **`Scen->Random`** | `RandomRanged(0,2)` conditional | 1 when cell-state condition fires | Yes (frequency unverified in this slot) | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.2, assembly `0x004F88FA` |
| `TechnoClass::IncreaseGattlingStage` particle spawn | `g_MainRng` | Unverified count | Data-dependent | Yes | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| `TechnoClass::SpawnRadEruption` | `g_MainRng` | Unverified count | Data-dependent | Yes | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| `EBolt__DrawRecursiveBolt` / `EBolt__Init` | `g_MainRng` | Branching draws | Data-dependent | Yes | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| Sound variant selection (`SoundEvent__*`) | `g_MainRng` | `Random__Next` or `RandomRanged` | 1 per sound variant pick | Yes | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| `BuildingClass::Mission_Missile` launch jitter | `g_MainRng` | `RandomRanged` | Unverified count | Yes | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1 |
| `AnimClass::AI` bouncer/meteor ore deposit | `g_MainRng` (needs stream verification — YELLOW) | `RandomRanged(0,3)` variant, then `RandomRanged(0,2)` density | 2 per valid landing candidate | Conditional (METDEBRI/CRYSTAL content) | `ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.5, assembly `0x00424102..0x00424155` |
| Bridge collapse anim spawn | `g_MainRng` | `RandomRanged(0,0x7FFFFFFE)` × 2 (X/Y jitter), `RandomRanged(1,5)` delay, `RandomRanged(0,BridgeExplosions.count-1)` type | 4 × 3 perp-cells × up to 4 axial iterations = 48 max | Yes (on collapse) | `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` §4 |
| Bridge damage roll | `g_MainRng` | `RandomRanged(1, Rules.BridgeStrength)` | 1 per bridge damage event | Yes | `bridges/WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md` §3.5 |
| `TerrainClass::AI` TIBTRE probability roll | **`Scen->Random`** (assembly `0x0071C755` loads `Scen+0x218`) | `Random__Next` raw, abs/modulo 1,000,000, float compare | 1 per idle TIBTRE per tick | Yes | `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §3, assembly `0x0071C761` |

> **Note on TIBTRE inside the live-object vector:** `TerrainClass` objects are registered in the `LogicClass` live vector and tick via `vtable+0x5C`. Their probability draw happens here, not in the explicit `FUN_004ACAC0` pre-driver at order 5. Their midpoint-triggered `SpreadTiberium` calls run at the animation midpoint tick — potentially a different frame number entirely.

### 2.3 Post-Object-Vector Phases (Orders 22–29)

| Frame order | Evidence | Subsystem | RNG? | Notes |
|---:|---|---|---|---|
| 22 | `0x0055B61B..0x0055B649` | AnimClass pool (non-local/non-skirmish only, modes ≠ 0,5) | AnimClass::AI draws if active | **Skipped in standard local skirmish (modes 0 and 5)** |
| 23 | `0x0055B64B` | Wave splash forces tick | Unverified | Yes |
| 24 | `0x0055B650` | `AlphaShapeClass::PurgeDisabled` | No RNG | Yes |
| 25 | `0x0055B655` | `MapClass::UpdateCrateRegenTimers` | Possibly (crate regen placement) | Yes, conditional |
| 26 | `0x0055B65A` | `g_Tactical->vtable+0x5C` | Unverified | Yes |
| 27 | `0x0055B66A..0x0055B68B` | `g_FactoryClass_Array` forward loop (`FactoryClass::AI`) | Unverified per-factory | Yes |
| 28 | `0x0055B68F..0x0055B6B1` | `g_HouseClass_Array` forward loop (`HouseClass::Update` per house) | **Yes** — `Scen->Random` `RandomRanged(0,2)` conditional per house per tick | Yes | 
| 29 | `0x0055B6B3..0x0055B6CC` | Last-ref-object refocus | No sim RNG | Yes, conditional |

> `HouseClass::Update` is confirmed to draw from `Scen->Random` at `0x004F88FA` (`RandomRanged(0,2)`) — this is **after** the main live-object vector, at order 28. Evidence: `RNG_SYSTEM_GHIDRA_REPORT.md` §3.2 (assembly `0x004F88FA`).

### 2.4 The Mystery MP RNG Spend

`Main_Tick @ ~0x0055D...` contains a conditional `RandomRanged(0,2)` draw gated on `g_GameMode == 3 || g_GameMode == 4` (LAN/Internet) and cursor-cell state. This runs **outside** `LogicClass::PerTickUpdate` and appears to be a deliberate stream-alignment spend for MP only. It does not fire in SP/skirmish. Evidence: `RNG_SYSTEM_GHIDRA_REPORT.md` §5.4. Active in YR: Conditional (MP modes only).

---

## 3. Key Per-Consumer Verified Facts

### 3.1 Stream assignment summary

| Stream | Verified consumers (selected) | Evidence |
|---|---|---|
| `g_MainRng` | Growth/spread batch draw, wall damage, particles (all types), bridge collapse RNG, laser/EBolt/Tesla jitter, sound variant, building destruction smudges center roll, combat scatter/damage, warhead detonation | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1; per-system docs |
| `Scen->Random` | Infantry scatter direction, unit scatter direction, infantry sub-cell rotation, anim scorch/crater 50/50, survivor smudge roll+angle, building center smudge discard rolls, HouseClass cell-state roll, TIBTRE probability roll | Assembly-verified in `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §2, `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2–3.4, `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §3 |

**CRITICAL UNVERIFIED STREAM ASSIGNMENTS (YELLOW):**
- Building center smudge discards at `0x004417E4`: assembly at `0x00441810` shows the roll; stream identity (`g_MainRng` vs `Scen->Random`) for the discard calls was not traced to the load point in this slot. `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §2 shows the offset helper table references `Scen+0x218` for the survivor path but uses a different address for the center-destruction path — this difference needs direct binary verification.
- Wall damage `CellClass::DestroyOverlay` stream — cited as `g_MainRng` based on pattern but not assembly-confirmed in this slot.
- AnimClass bouncer ore stream — not assembly-confirmed in this slot.

### 3.2 Draw counts for high-frequency consumers

| Consumer | Draws per activation | Variable? |
|---|---|---|
| TiberiumClass GrowthProcessor per type | Exactly 1 (`Random__Next` abs modulo batch_clamped), plus no draw if gates fail | Fixed once per active type per growth interval |
| SpreadTiberium TIBTRE direction | Exactly 1 `RandomRanged(0,7)` | Fixed |
| SpreadTiberium TIBTRE empty-cell placement variant | Exactly 1 `RandomRanged(0,11)` | Only if valid empty flat cell found |
| TIBTRE probability roll | Exactly 1 `Random__Next` | Fixed per idle TIBTRE per tick |
| Building center smudge (3×3+ foundation) | 3 (2 discard + 1 roll) | 1 for 2×2 (no discards due to equal-bound no-draw) |
| Survivor smudge per passable cell | 2 (1 roll + 1 angle draw) | 0 if cell fails passability |
| InfantryClass::Scatter | 1 `RandomRanged(0,4)` | Fixed per invocation |
| UnitClass::Scatter (threat-coord path) | 1 `RandomRanged(0,2)-1` | Fixed per invocation |
| Particle constructor lifetime | 1 `Random__Next` raw | Fixed per particle spawn |
| Bridge collapse anim spawn | 4 per anim × 3 perp-cells × iterations (≤48 total) | Data-dependent on `BridgeExplosions.count` |
| Wall damage stage advance | 1 `RandomRanged(0, Strength)` | 1 when below-strength non-forced hit |

---

## 4. Rust `SimRng` Ordering vs Native

| Rust phase | `self.rng` draw site | Draws from | Native phase | Match? |
|---:|---|---|---|---|
| Phase 2: Ground movement | `tick_movement_with_grids` (`world/mod.rs:1286`) | Sub-cell rotation, scatter | Native order 21 (within-vector per object) | DRIFT — Rust movement runs before combat; native scatter is inside object `vtable+0x5C` in order 21 |
| Phase 4.5: Superweapons | via `tick_superweapons` (implicit, e.g. lightning) | `g_MainRng` equivalent | Native order 18 (LightningStorm) — BEFORE object vector | DRIFT — Rust superweapons run before combat; native LightningStorm runs at order 18 before order 21 objects |
| Phase 5: Combat + smudge drain | wall damage (`world/mod.rs:981`), smudge drain (`world/mod.rs:1680/1692`) | Wall RNG, smudge RNG | Native: wall and smudge are within object vector at order 21 | Partially matching |
| Phase 5.5: Particles | `tick_particle_systems` | Particle lifetime/jitter/spawn | Native: within-vector at order 21 | DRIFT — timing of particle ticks vs object vector unclear |
| Phase 7: Ore growth/spread | `tick_native_growth_driver/spread_driver` (`world/mod.rs:1755/1768`) | Growth batch draw, spread direction | **Native orders 9–10 — before bombs, teams, EMP, object vector** | **DRIFT: ore runs after combat/production in Rust; native runs before object vector** |
| Phase 7: TIBTRE spawning | `tick_terrain_spawners_stateful` (`world/mod.rs:1793`) | TIBTRE probability; midpoint direction/variant | Native: probability within order-21 `TerrainClass::AI`; direction/variant at midpoint tick | DRIFT — Rust TIBTRE spawner is post-ore; native is within-vector |

**Summary:** The single Rust `SimRng` stream does not match native ordering because: (a) it conflates `g_MainRng` and `Scen->Random` draws, and (b) the Rust phase ordering differs from native `PerTickUpdate` ordering in multiple places.

---

## 5. Implementation Handoff

### 5.1 Two-stream split

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|---|
| `InfantryClass::Scatter`, `HouseClass::Update`, `CellClass::PlaceInfantryInCell`, `UnitClass::Scatter` (direction), `AnimClass::Start` smudge 50/50, and survivor smudge rolls all draw from `Scen->Random` (`Scen+0x218`), not `g_MainRng`. | `RNG_SYSTEM_GHIDRA_REPORT.md` §3.2; `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.2–3.4; `SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §2; `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §3 | Single `SimRng` for all draws | `src/sim/world/mod.rs` `self.rng`; all call sites using `next_range_u32` for scatter/smudge/sub-cell | Add `scenario_rng` field separate from `sim_rng`; route scatter/sub-cell/survivor-smudge/house-cell-state draws through `scenario_rng` | Run 100-tick skirmish; check that `scenario_rng` state at tick 100 matches draws from TIBTRE probability, infantry scatter, and house cell-state roll replayed in native order; `sim_rng` state should differ | `test_dual_rng_stream_split_infantry_scatter_uses_scenario_stream` | Do not route TIBTRE probability or combat draws through `scenario_rng` — native uses `Scen->Random` for TIBTRE probability but `g_MainRng` for growth batch count |

### 5.2 Ore growth/spread ordering relative to object vector

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|---|
| `TiberiumClass::GrowthDriver_AllTypes` and `SpreadDriver_AllTypes` run at native orders 9–10, before bombs (order 11), teams (13–14), EMP (20), and the main live-object vector (order 21). Rust runs ore growth/spread in Phase 7, after combat (Phase 5), particles (5.5), retaliation (6), and production (Phase 7 start). | `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md` orders 9–10; `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md` §2 orders 9–10 | Ore growth/spread RNG draws at Rust Phase 7 instead of native pre-object-vector position | `src/sim/world/mod.rs::advance_tick` ore growth/spread section (`world/mod.rs:1748..1783`) | Move ore growth/spread RNG draws to the pre-object-vector section of the tick; this may require restructuring the phase order or isolating the RNG draws | On a tick where ore growth fires and a unit takes damage, the ore growth batch draw must precede all combat-phase draws | `test_ore_growth_rng_precedes_combat_phase_draws` | Moving ore growth/spread requires also moving any state they produce (density changes) that combat reads; do not split the RNG draw from the state mutation |

### 5.3 TIBTRE probability stream and draw position

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|---|
| TIBTRE probability rolls one `Random__Next` from `Scen->Random` (not `g_MainRng`) per idle terrain object per tick, inside the main live-object vector at order 21. Midpoint-triggered direction/placement draws from `Scen->Random` as well (`SpreadTiberium` assembly `0x00483823`). | `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §3 assembly `0x0071C761`; `ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.3; `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` assembly `0x00483823..0x00483839` shows `LEA ECX,[Scen+0x218]` | TIBTRE terrain spawner RNG draws are via `self.rng` (single stream, post-ore-growth phase) | `src/sim/terrain_spawn.rs`; `src/sim/world/mod.rs:1787..1803` | Route TIBTRE probability and midpoint direction/variant draws through `scenario_rng`; move terrain spawner tick to within-object-vector phase equivalent | A seeded run with 3 idle TIBTREs must produce probability draws interleaved with other object `vtable+0x5C` draws in creation order, not after ore growth | `test_tibtre_probability_draw_routes_through_scenario_rng` | Do not route TIBTRE to `sim_rng`; do not place TIBTRE tick after ore growth/spread |

---

## 6. Negative Facts / Do Not Do

1. **Do not use a single RNG stream for all draws.** gamemd has two independent streams (`g_MainRng` and `Scen->Random`) that diverge from tick 1. Even if the algorithm matches, single-stream will desync from gamemd in any game involving scatter, sub-cell placement, or infantry creation. Evidence: `RNG_SYSTEM_GHIDRA_REPORT.md` §3.1/§3.2; `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.3.

2. **Do not place ore growth/spread RNG after combat in the native frame skeleton.** Growth/spread are at native orders 9–10 (before object AI at order 21). Moving them later changes which RNG draws precede the main object vector's draws, cascading a stream mismatch across all subsequent consumers. Evidence: `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md` §2 orders 9–10.

3. **Do not use `RandomRanged` for TIBTRE probability.** gamemd uses `Random__Next` (raw, one draw) at `0x0071C761`. `RandomRanged` can consume more than one draw on rejection, shifting every subsequent consumer. Evidence: `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §3, assembly `0x0071C761`.

4. **Do not use `RandomRanged` for particle lifetime, fire jitter, or fire insertion shuffle.** All three use raw `Random__Next` modulo. `RandomRanged` may re-draw on rejection, causing cross-consumer ordering drift that compounds per particle spawned. Evidence: `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.1/§3.4/§3.5.

5. **Do not treat the AnimClass pool loop (native order 22) as active in standard local skirmish.** It is gated by `g_GameMode != 0 && g_GameMode != 5`; standard skirmish uses mode 0 or 5. Evidence: `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md` §2 order 22.

---

## 7. Remaining Uncertainty

- **YELLOW: Stream identity for building center smudge discard rolls** (`BuildingClass::DestructionEffects` at `0x004417E4`). The survivor path is confirmed `Scen->Random`; the center-destruction roll may also be but was not assembly-traced in this slot for the discard portion (distinct from the 50/50 roll). Verification: `decompile_function 0x004415F0` and trace the `ECX` value in the `RandomRanged` call at `~0x004417E7`.

- **YELLOW: Wall damage stream** (`CellClass::DestroyOverlay` `RandomRanged(0,Strength)`). `g_MainRng` assumed by pattern; needs assembly trace of `ECX` in `CALL 0x0065C7E0` at `0x00480D03` to confirm.

- **YELLOW: AnimClass bouncer ore deposit stream.** Assumed `g_MainRng` but the `ECX` source at `0x00424102` was not traced.

- **UNVERIFIED: Draw counts within per-object handlers** for `TechnoClass::IncreaseGattlingStage`, `TechnoClass::SpawnRadEruption`, `EBolt__Init`, `BuildingClass::Mission_Missile`. Listed as `g_MainRng` by confirmed xref but exact draw count per activation not documented.

- **UNVERIFIED: TeamClass AI draws** (orders 13–14). Deferred project-wide; stream identity and draw pattern unknown.

- **UNVERIFIED: BombClass::UpdateAll draws** (order 11). Draw pattern not documented in available reports.

- **UNVERIFIED: Main-vector object iteration order.** gamemd uses flat dynamic insertion-order vector; Rust uses `BTreeMap<u64>` sorted by entity ID. Iteration order diverges from gamemd for the same game state. This is the root cause of any within-vector ordering mismatch, even if all other ordering fixes are applied.

---

## 8. Stale Doc Replacement Wording

**`docs/research/RNG_SYSTEM_GHIDRA_REPORT.md` §5.4** — Existing wording: "This appears to be a deliberate RNG-eating call to keep two clients' streams aligned even when one client has a different cursor state." Replace clarification note with: "This conditional `RandomRanged(0,2)` draw in `Main_Tick` fires only for `g_GameMode == 3 || g_GameMode == 4` (MP modes). It runs *outside* `LogicClass::PerTickUpdate` and therefore does not appear in the per-frame ordered ledger for standard local skirmish. Its stream identity and exact trigger condition remain DEFERRED."

---

## 9. Sources

- `docs/research/LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md` — global subsystem order, verified Ghidra decompile of `0x0055AFB0`.
- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md` — full top-level ladder with binary ranges.
- `docs/research/RNG_SYSTEM_GHIDRA_REPORT.md` — three-instance layout, caller list, stream seeding.
- `docs/research/PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — particle RNG bounds/helpers.
- `docs/research/SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — smudge/anim 50/50 RNG.
- `docs/research/ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — TIBTRE/bouncer RNG.
- `docs/research/TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` — TIBTRE timing and stream identity.
- `docs/research/SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — scatter direction RNG and stream identity.
- `docs/research/TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md` — growth processor batch draw.
- `docs/research/WALL_DAMAGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — wall damage `RandomRanged(0,Strength)`.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` — bridge collapse per-anim RNG order.
- `src/sim/world/mod.rs::advance_tick` — current Rust phase ordering and `self.rng` draw sites.

## 10. Status

**PARTIAL.** The ordered ledger is complete for all known high-frequency consumers across the 30-step native ladder; stream identity is verified for all consumers where prior docs traced the `ECX` value. Remaining uncertainty covers 5 items (stream identity for 3 yellow consumers, 2 draw-count-only gaps). The core determinism-critical findings (two-stream split requirement, ore growth ordering, TIBTRE stream identity) are at HIGH confidence with inline evidence.
