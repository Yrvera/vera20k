# Two-Stream RNG Fix — Synthesized Design (2026-05-29)

> DESIGN + PLAN ONLY. No `.rs` files are modified by this document.
> Authority order for every claim below: binary → Ghidra (live) → `docs/research/` → `ini/`.
> Source ground truth: `docs/research/SUBSTRATE_PARITY_LEDGER_20260529.md` + the dual-stream
> routing/init packet captured this session (`decompile_function 0x0052FC20`,
> `disassemble_function` of each routed consumer — cited per row below).

## 1. The parity hole (one sentence)

gamemd runs **two** independent `RandomClass` instances — the scenario stream
(`Scenario->Random` @ `Scen+0x218`) and the main/global stream (`g_MainRng` @ `0x00886B88`) —
**seeded byte-identically** from one `g_RngSeed` at init but advanced independently because
different consumers draw from different streams; our engine has **one** `Simulation.rng: SimRng`
(`src/sim/world/mod.rs:289`), so all consumers share one cursor and diverge from gamemd from
tick 1.

## 2. Chosen approach: two named fields + intent-named typed accessors

**Winner: `typed-accessor`, grafted with the seeding/identity proofs from `two-fields` and the
dual-stream-per-function rule from `stream-enum`.**

`Simulation` holds **two named `SimRng` fields**; consumers never name a *stream*, they call an
**intent-named accessor** that owns the stream choice. This is the strongest defense against the
dominant failure mode (silent misroute — wrong stream still compiles, still produces plausible
random values, only desyncs vs gamemd) and the cheapest to extend.

### Why this shape over the alternatives

| Criterion | two-fields (raw `&mut self.scenario_rng`) | stream-enum (`DualRng` + `RngStream` arg) | **typed-accessor (chosen)** |
|---|---|---|---|
| Parity-correctness | routes correctly if each site picks right | routes correctly | same |
| Misroute-resistance | **weak** — each site re-decides the stream by name; a copy-paste picks the wrong field silently | medium — wrong enum can be passed far from the routing decision | **strongest** — site names *intent*; stream is decided in exactly one place per intent; auditable as one line |
| Determinism/lockstep | identical (same `SimRng`) | identical | identical |
| Leaf-code churn | low (leaf sigs stay `&mut SimRng`) | **high** (enum threaded into every leaf signature) OR Option-A accessors (then ≈ this design) | low (leaf sigs stay `&mut SimRng`; accessor returns `&mut SimRng`) |
| Dual-stream-per-fn (HouseClass/TechnoClass::Fire) | needs both fields exposed | native (per-draw enum) | needs both accessors callable from one fn — supported |
| Clarity / audit anchor | field name | enum at call site | **accessor name = routing record + grep anchor** |

The enum approach's strength (centralizing routing) is achieved here by the accessor name without
threading an enum through leaf signatures; the two-fields approach's strength (zero leaf churn,
trivial seeding proof) is preserved because accessors return `&mut SimRng` and drop into every
existing borrow site unchanged.

### 2.1 Data structure (`src/sim/world/mod.rs`, replacing line 289)

```rust
/// Scenario RNG — gamemd `Scenario->Random` (Scen+0x218). Drives in-object-tick
/// sim draws: scatter, sub-cell placement, smudge/destruction, particles,
/// wall/overlay damage, bridge collapse/repair, ore growth/spread, TIBTRE,
/// anim scorch/50-50, miner-dock jitter. MUST be serialized + hashed (never
/// #[serde(skip)]) or a divergence here hides from desync detection.
pub(crate) scenario_rng: SimRng,
/// Main/global RNG — gamemd `g_MainRng` (0x00886B88). Drives presentation/weapon
/// helpers: weapon spread, warhead detonate, sound variant, EBolt/laser, building
/// missile, HouseClass AI/superpower gate. No verified sim/ consumer routes here
/// TODAY (those layers are not yet ported); seeded + hashed regardless so it is
/// already in lockstep when they land. MUST be serialized + hashed.
pub(crate) main_rng: SimRng,
```

- The single `pub rng: SimRng` field is **removed entirely**, not renamed. This forces the
  compiler to flag every borrow site so nothing compiles until it has been explicitly routed —
  the migration becomes mechanically exhaustive instead of best-effort.
- Both fields are `pub(crate)` (not `pub`) so external crates/layers cannot bypass the accessors.
- `SimRng` itself is **unchanged** — math, mask, rejection sampling, and the `index_b=0x67` seed
  are already byte-identical to gamemd (`INIT_TABLE_1`/`INIT_TABLE_2` verified vs `read_memory
  0x00839644`/`0x00839694`; raw sequence pinned by `test_gamemd_raw_sequence_seed_one`,
  `src/sim/rng.rs:218`).

### 2.2 Accessor API (intent-named; one routing decision per intent)

```rust
impl Simulation {
    // --- Scenario stream (PROVEN scenario unless flagged in §3 YELLOW) ---
    pub(crate) fn scatter_rng(&mut self)      -> &mut SimRng { &mut self.scenario_rng } // bump displacement, idle/forced scatter, passenger unload exit, sell-eject
    pub(crate) fn subcell_rng(&mut self)      -> &mut SimRng { &mut self.scenario_rng } // infantry sub-cell rotation, paradrop sub-cell
    pub(crate) fn smudge_rng(&mut self)       -> &mut SimRng { &mut self.scenario_rng } // destruction smudge/survivor/debris, smudge type pick
    pub(crate) fn wall_damage_rng(&mut self)  -> &mut SimRng { &mut self.scenario_rng } // overlay/wall damage roll
    pub(crate) fn bridge_rng(&mut self)       -> &mut SimRng { &mut self.scenario_rng } // bridge collapse/repair/debris/explosion
    pub(crate) fn ore_rng(&mut self)          -> &mut SimRng { &mut self.scenario_rng } // ore growth/spread queue + direction + variant, TIBTRE
    pub(crate) fn anim_rng(&mut self)         -> &mut SimRng { &mut self.scenario_rng } // building damage-fire type/start-frame
    pub(crate) fn particle_rng(&mut self)     -> &mut SimRng { &mut self.scenario_rng } // particle/smoke/gas/fire lifetime/offset/dir/insert
    pub(crate) fn superweapon_rng(&mut self)  -> &mut SimRng { &mut self.scenario_rng } // lightning-storm scatter/bolt (PROVEN scenario — §3)
    pub(crate) fn miner_jitter_rng(&mut self) -> &mut SimRng { &mut self.scenario_rng } // dock-entry retry + unload-deploy frame jitter

    // --- Main stream (PROVEN main; no sim/ consumer wired yet — for future use) ---
    pub(crate) fn weapon_spread_rng(&mut self) -> &mut SimRng { &mut self.main_rng } // projectile spread X/Y, warhead detonate scatter
    pub(crate) fn house_ai_rng(&mut self)      -> &mut SimRng { &mut self.main_rng } // HouseClass superpower/AI gate roll
    // sound / EBolt / laser / building-missile draws live in audio/render/app layers; add
    // a main-stream accessor at each as it is ported. NEVER default them to scenario_rng.
}
```

**Keep accessors distinct even when several return the same stream today.** The intent name is the
per-consumer routing record and the audit/grep anchor; collapsing them to one `scenario_rng()`
would erase which consumer maps to which gamemd authority class and make a future main-routed
consumer easy to misfile. Each accessor gets a one-line comment naming its gamemd authority
class — the routing table below *is* the spec.

**Borrow scope:** an accessor returns `&mut SimRng` borrowing only the one field, so the
borrow checker permits passing `sim.scatter_rng()` into a subsystem while the rest of `sim` is
otherwise borrowed exactly as `&mut sim.rng` was before. Functions that already take
`rng: &mut SimRng` keep their signature; only the **caller** changes (`&mut sim.rng` →
`sim.bridge_rng()`, etc.). The `BridgeSpawnContext`-style struct that stores `rng: &'a mut SimRng`
keeps its field type; only the construction site changes.

## 3. Verified routing table (every call site in the ground-truth packet)

Confidence is per the live-disassembly evidence captured this session. **Default verdict is DRIFT
until PROVEN** (CLAUDE.md burden of proof); rows marked YELLOW are LIKELY-by-family and must be
upgraded by a direct `disassemble_function` before implementation ships.

| Rust call site(s) | Accessor | Stream | Confidence | gamemd authority (cited evidence) |
|---|---|---|---|---|
| `world/mod.rs:1437` (ground-movement dispatch → bump_crush) | `scatter_rng()`/`subcell_rng()` | scenario | PROVEN | `UnitClass__Scatter 0x00743a50` / `InfantryClass__Scatter 0x0051D0D0` read `[0x00a8b230]+0x218` (`disassemble_function 0x00743DC5`, `0x0051D2AC`) |
| `bump_crush.rs:383` (`next_range_u32(4)` sub-cell rotation) | `subcell_rng()` | scenario | **PROVEN** | `CellClass__PlaceInfantryInCell` `0x00481180`: at `0x0048138a` `MOV EDX,[0x00a8b230]` → `LEA ECX,[EDX+0x218]` → `PUSH 3; PUSH 0; CALL 0x0065c7e0` = `RandomRanged(0,3)` (4 outcomes, matches `next_range_u32(4)`) from `Scen+0x218` (`disassemble_function 0x0048139A`, 2026-05-29) |
| `bump_crush.rs:740` (`next_range_u32(8)` bump displacement) | `scatter_rng()` | scenario | PROVEN | `UnitClass__Scatter 0x00743a50` |
| `scatter.rs:123` (`next_range_u32(8)`) | `scatter_rng()` | scenario | PROVEN | `UnitClass__Scatter 0x00743a50` (note: `tick_idle_scatter` caller is commented out — reached only via other scatter callers) |
| `passenger.rs:887`, `:1042` (`next_u32() % 8` unload exit) | `scatter_rng()` | scenario | PROVEN (family) | scatter/exit family, `Scen+0x218`. **Modulo-bias preserved — see §7 R4** |
| `aircraft/drop_payload.rs:184` (→ `allocate_sub_cell_with_preference`) | `subcell_rng()` | scenario | **PROVEN** | same `CellClass__PlaceInfantryInCell` `Scen+0x218` path as `bump_crush.rs:383` (`disassemble_function 0x0048139A`, 2026-05-29) |
| `miner/miner_dock_sequence.rs:83`, `:1018` (jitter) | `miner_jitter_rng()` | scenario | PROVEN (family) | in-object-tick (`MissionClass`) sim draw, `Scen+0x218` |
| `production/production_sell.rs:394` (`next_range_u32_inclusive(0,4)-2` eject dir) | `scatter_rng()` | scenario | PROVEN | `BuildingClass__SpawnSurvivors 0x00442d90` eject/facing reads `Scen+0x218` (`disassemble_function 0x00442d90`) |
| `world/mod.rs:1008` (→ `damage_wall_overlay`) + `overlay_grid.rs:359` (`next_range_u32(strength)`) | `wall_damage_rng()` | scenario | PROVEN | `CellClass__DestroyOverlay 0x00480cb0` reads `Scen+0x218` (`disassemble_function 0x00480cb0`) — **contradicts old ledger (g_MainRng/YELLOW)** |
| `combat/smudge_dispatch.rs:38,212,239,240,241,297` | `smudge_rng()` | scenario | PROVEN | `BuildingClass__DestructionEffects 0x004415f0` (W-2/H-2 discards `0x004417d3`/`0x00441805`, roll `0x00441819`) + `SpawnSurvivors 0x00442d90` — **contradicts old ledger** |
| `smudge_grid.rs:285` (`next_range_u32(filtered.len())`) | `smudge_rng()` | scenario | PROVEN (family) | DestructionEffects/AnimClass smudge-placement family, `Scen+0x218` |
| `bridge_state/mod.rs:1335`; `walker.rs:412/420`; `world_orders.rs:379`; `bridge_orchestrator.rs:1177,1185,1195,1203,1227,1228,1261,1262,1265,1294-1295,1419` | `bridge_rng()` | scenario | PROVEN | `CellClass__BlowUpBridge 0x0047dd70` (95% gate `0x0047de54`, jitter `0x0047dec6/df04`, metallic `0x0047df43/df91`, delay `0x0047dfe1`, index `0x0047e004`) — **contradicts old ledger** |
| `ore_growth.rs:363,435,525,559,583,693,863,1192,1274,1463,1499` | `ore_rng()` | scenario | PROVEN | `TiberiumClass__GrowthProcessor 0x00722f00` raw `Next` at `0x00722f6f`/`0x00723044`, `[0x00a8b230]+0x218` — **contradicts ledger §2.1** |
| `terrain_spawn.rs:78,382,587` (TIBTRE) | `ore_rng()` | scenario | PROVEN | `TerrainClass::AI 0x0071C730` raw `Next` at `0x0071C755`, `Scen+0x218` (`disassemble_function 0x0071C730`) |
| `superweapon/lightning_storm.rs:190,191,202,203,213` | `superweapon_rng()` | scenario | **PROVEN** | `LightningStorm__GroundStrike 0x0053a300` (Scen+0x218 at `0x0053a345`/`a47a` `Next`, `0x0053a62c`/`a665` `RandomRanged`) + `LightningStorm__Process 0x0053a6c0` (Scen+0x218 at `0x0053a9ab`/`a9c3` strike-offset `RandomRanged`); **zero g_MainRng reads** (`disassemble_function 0x0053a300`, `0x0053a6c0`, 2026-05-29) |
| `app_building_anim.rs:258,263` (damage-fire type/start-frame) | `anim_rng()` | scenario | PROVEN | `AnimClass__AI 0x00423ac0` / `AnimClass__Start 0x00424f00` read `Scen+0x218`. **NOTE: app layer, not sim/ — see §6** |
| `particles/spawn.rs:96,99,229`; `smoke.rs:88,89,178,213`; `gas.rs:86,87,198`; `fire.rs:65,116` | `particle_rng()` | scenario | PROVEN | `ParticleClass__Constructor 0x0062b5e0` (velocity `0x0062b7e7`, lifetime `0x0062b842/b870`, color `0x0062bac0`) `Scen+0x218` — **strongly contradicts ledger §2.2** |
| `world/mod.rs:1864` (DORMANT idle-scatter, commented out) | `scatter_rng()` (when reactivated) | scenario | PROVEN | `InfantryClass__Scatter`; **currently draws nothing — advances no stream** |

**Main-stream consumers (no current sim/ call site).** Weapon projectile spread (`TechnoClass::Fire
0x004690b0`, X/Y at `0x004690f0`/`0x00469121`, `MOV ECX,0x886B88`), warhead detonate scatter
(LIKELY), sound variant (`SoundEvent__SelectNextSample` etc.), EBolt/Tesla (`EBolt__DrawRecursiveBolt`),
laser (`LaserDrawClass__Draw`), building-missile (`BuildingClass__Mission_Missile`), HouseClass
superpower/AI gate (`HouseClass__Update 0x004F887D/0x004F8895`). These live in `combat/`/`audio/`/
render/app layers and are routed via `weapon_spread_rng()`/`house_ai_rng()`/future accessors when
ported. Until then `main_rng` is seeded + hashed but never advanced — **correct** (gamemd advances
`g_MainRng` only via these consumers), but it means main-stream parity is **untested by gameplay
today**; do not claim the hole is fully closed (§7 R3).

**Dual-stream-in-one-function rule.** `HouseClass::Update` (`0x004F8440`) and `TechnoClass::Fire`
(`0x004690b0`) each draw from **both** streams in one function: cell-state roll `0x004F88FA` =
scenario, AI/superpower gate `0x004F887D/0x004F8895` = main; weapon spread = main, on-fire anim
spawn `0x00469d29…` = scenario. When these are ported, the stream choice is **per-draw, not
per-function** — the function must hold `&mut self`/both accessors and call the correct accessor at
each site. A single `&mut SimRng` param would silently re-merge the streams. Pre-register both in
the implementation plan.

## 4. Seeding — both streams byte-identical including index_b=0x67

gamemd (`decompile_function 0x0052FC20`, `Init_Random_Number_System`): `Random__Seed(g_RngSeed)` →
copy 253 dwords (index_a + index_b + 250 table words) into `Scen+0x218` → `Random__Seed(g_RngSeed)`
**again with the same seed** → copy 253 dwords into `g_MainRng`. Both images are byte-identical
because `Random__Seed 0x0065C6D0` is a pure deterministic function of the seed (it sets `index_a=0`,
`index_b=0x67`, fills 250 words, `disabled=0`).

Rust equivalent in `with_seed` (`mod.rs:451`, replacing `rng: SimRng::new(seed)`):

```rust
scenario_rng: SimRng::new(seed),
main_rng:     SimRng::new(seed),
```

`SimRng::new(seed)` already reproduces the full seed image (`index_a=0`, `index_b=RNG_INDEX_B_SEED=0x67`,
250 words from `INIT_TABLE_1/2`, `disabled=0` — `src/sim/rng.rs:25-34,65-88`). Two `new(seed)` calls
with the same seed are identical by construction — the same proof gamemd's two `Seed` calls give.
**No new seeding code is needed.** `DEFAULT_SIM_SEED` (`mod.rs:75`) is unchanged; both streams
inherit it.

Add a debug assertion in `with_seed`: `debug_assert_eq!(scenario_rng.state(), main_rng.state())`,
to lock the byte-identity invariant at construction. The lockstep guarantee for MP is unchanged:
shared negotiated `g_RngSeed` in → identical dual-stream state out (the entropy block in
`0x0052FC20` is skipped for network/replay; deterministic seed used as-is).

The 253-dword clone copies index_a/index_b/table but **not** the offset-0 `disabled` byte (which
`Seed` already set to 0). We re-run `reseed` rather than memcpy, so this is irrelevant to us — but
note it so a future `disabled`-toggling feature (RNG suspend) keeps both clones in lockstep.

## 5. Snapshot + hash — both streams, or a desync hides

**`snapshot.rs`:** no per-field edit — it serializes the whole `Simulation` via bincode
(`GameSnapshotRef.sim`). Because both `scenario_rng` and `main_rng` are plain (non-`#[serde(skip)]`)
fields, bincode persists both automatically. **Two required actions:**
1. **Audit:** neither field may carry `#[serde(skip)]` (comment on each field states this; mirror it
   in any future refactor).
2. **Bump `SNAPSHOT_VERSION`** (one field → two changes the bincode layout; bincode is positional,
   so an old one-`rng` blob read into the two-field struct corrupts everything after it). Old saves
   must be **rejected cleanly**, not mis-deserialized. This is an accepted save/replay-format break.

**`world_hash.rs:39`:** replace the single hash with **both**, in a **fixed, documented order**:

```rust
self.scenario_rng.hash_state(&mut hasher);
self.main_rng.hash_state(&mut hasher);
```

Order is part of the hash contract and must never change. If only one stream were hashed, a
divergence in the other produces identical hashes on two desynced clients — the desync detector
goes **blind exactly where this fix matters most**. Both, always.

**Replay-seed site (`app_sim_tick.rs:260`, `seed: sim.rng.state()`):** this no longer compiles. It
was already wrong — `state()` is a mid-stream fingerprint, not the seed. Fix: store the construction
seed on `Simulation` (e.g. `pub(crate) seed: u64`, set in `with_seed`) and record **that** in the
replay header. The replay header carries one negotiated `g_RngSeed`; both streams derive from it.
This is a latent existing bug the two-stream change surfaces; fixing it is in scope (it won't
compile otherwise).

## 6. Interaction with tick-rate + logic-vector fixes (NOT in scope — ordering caveat only)

These are separate, parallel efforts. Do not solve them here; observe the ordering risks:

- **logic-vector** (`src/sim/world/logic_vector.rs`, untracked WIP from a parallel session;
  dispatched from `world/mod.rs:553`) defines LogicClass active-object ordering. RNG draws happen
  *inside* per-object bodies dispatched in logic order. The two streams are **order-independent**,
  but the **scenario stream's observed value sequence depends on logic order** — if logic order
  changes, the scenario cursor's trajectory changes even though per-consumer routing is unchanged.
  The `:553` dispatch borrow becomes `sim.<intent>_rng()`. **Merge risk:** both changes touch
  `:553`/`logic_vector.rs`. **Order:** land logic-vector first (it defines the path/order), then
  route its borrow to scenario; or coordinate the two diffs. Do not judge logic-vector correctness
  here.

- **tick-rate** changes how many `advance_tick` calls elapse per wall-second — affects how often
  per-tick draws (ore-growth batch, scatter) fire, shifting **both** cursors' positions over time.
  No stream-routing interaction. **Tests must pin draw counts per *tick*, not per wall-clock ms**,
  to survive both fixes.

- **`app_building_anim.rs:258,263` is in the app layer, not `sim/`.** Routing it to `anim_rng()`
  (scenario) is correct per `AnimClass`, but if the tick-rate/logic-vector refactor relocates these
  draws into the sim tick or changes *when* in the tick they fire, the scenario stream's draw order
  shifts and parity breaks. Whoever moves these must re-validate scenario-stream order. Do not
  reorder draws as part of this change.

- **Shared:** all three fixes alter `state_hash` output (logic order, tick count, dual RNG).
  Re-baseline any golden-hash fixtures **once, after all three land** — not between, or you
  re-baseline twice.

**Recommended merge order:** land this two-stream split **first** (smallest, most local, no draw
reordering), then rebase tick-rate/logic-vector onto a correct two-stream baseline to validate
draw-ordering against.

## 7. Test strategy — prove each stream reproduces gamemd's sequence

1. **Seed-equality invariant (unit).** After `with_seed(s)`: `scenario_rng.state() ==
   main_rng.state()` and both `index_b == 0x67`, across seeds incl. 0, 1, `u32::MAX`-derived. Proves §4.
2. **Independence (unit).** Draw N from `scatter_rng()` only; assert `main_rng.state()` unchanged and
   equal to a fresh `SimRng::new(s)`; `scenario_rng.state()` changed. Symmetric for `weapon_spread_rng()`.
3. **gamemd raw-sequence pin per stream (unit).** Extend `test_gamemd_raw_sequence_seed_one`
   (`rng.rs:218`): both streams independently produce `0x78B76ED5, 0x275D74AE, 0xDA63B931` from
   seed 1 — proves the clone is exact.
4. **Routing regression (unit) — the central guard.** One test per accessor: draw once through it,
   assert *only* the intended stream advanced. Catches a future edit silently re-pointing e.g.
   `bridge_rng()` at `main_rng`. This is the guard against the dominant (silent-misroute) failure.
5. **Ground-truth value parity (Ghidra, highest value — REQUIRED, not optional).** For ≥1 scenario
   consumer (e.g. wall-damage `RandomRanged(0,strength)` or bridge variant `RandomRanged(0,3)`) and
   ≥1 main consumer (weapon spread), capture gamemd's emitted **ranged** sequence for a fixed
   post-init seed via `emulate_function 0x0065C7E0` (`Random__RandomRanged`) and assert the Rust
   stream matches value-for-value. Without this, tests prove internal consistency, **not gamemd
   parity** — this is the test that satisfies the burden-of-proof bar.
6. **Hash coverage (unit).** Draw from `main_rng` only → assert `state_hash()` changes. Same for
   `scenario_rng`. Proves §5 — neither stream is silently excluded from the hash.
7. **Snapshot round-trip.** Advance each stream a *different* number of draws, save → load, assert
   both streams restore to identical state (proves the serde swap persists both and independently).
8. **Determinism regression (compile-fix).** Test helpers that do `sim.rng = SimRng::new(seed)`
   (`combat_tests`, `world_tests`, `bridge_orchestrator`, `miner_tests`) break; add a `pub(crate)
   fn reseed_both(&mut self, seed)` helper and update each, preserving single-stream test intent.

## 8. YELLOW handling (adjusted 2026-05-29 — Y1/Y2 closed by live Ghidra; none block this change)

**Resolved this session (LIKELY → PROVEN):**

- **Y1 — sub-cell routing: PROVEN scenario.** `CellClass__PlaceInfantryInCell 0x00481180`, at
  `0x0048138a`: `MOV EDX,[0x00a8b230]` (Scenario ptr) → `LEA ECX,[EDX+0x218]` → `PUSH 3; PUSH 0;
  CALL 0x0065c7e0` = `RandomRanged(0,3)` (4 outcomes — matches Rust `next_range_u32(4)`).
  `bump_crush.rs:383` + `drop_payload.rs:184` route `subcell_rng()→scenario` confirmed
  (`disassemble_function 0x0048139A`). No longer a gate.
- **Y2 — lightning-storm routing: PROVEN scenario.** `LightningStorm__GroundStrike 0x0053a300`
  and `LightningStorm__Process 0x0053a6c0` draw exclusively from `Scen+0x218` (six sites: `Next`
  `0x0053a345`/`a47a`, `RandomRanged` `0x0053a62c`/`a665`/`a9ab`/`a9c3`); **zero g_MainRng
  reads.** `lightning_storm.rs:190-213` route `superweapon_rng()→scenario` confirmed
  (`disassemble_function 0x0053a300`, `0x0053a6c0`). No longer a gate.

**Reclassified — NOT blockers for this change (these consumers are not yet ported):**

- **Y3 — warhead detonate scatter (FUTURE, not this change).** A *main-stream* consumer that does
  not exist in the port yet; `main_rng` stays unadvanced until warhead RNG lands. Routing it is the
  porting task's responsibility, not this one. When ported: verify the Detonate AoE draw count and
  confirm it reads `g_MainRng 0x886B88` (co-located with the PROVEN `TechnoClass::Fire` spread).
  Tracked under §3 "Main-stream consumers", removed from this change's gate list.
- **Y4 — `passenger.rs:887/:1042` `% 8` modulo bias (SEPARATE pre-existing DRIFT).** Not a routing
  question and not introduced by this change. This change **preserves the existing draw method
  verbatim** (changing `% 8` → `RandomRanged(0,7)` would shift the stream's draw count/values and
  must not be bundled). Spin off as its own fix task; out of scope here.
- **Y5 — main-stream completeness (inherent scope boundary, not a defect).** Correct by design:
  gamemd advances `g_MainRng` only via weapon/audio/AI consumers, none of which are ported, so
  `main_rng` being seeded+hashed-but-unadvanced *is* the faithful state. This change is necessary
  and **sufficient for every RNG consumer that exists in the port today**; main-stream end-to-end
  gameplay parity arrives with those consumers. Stated honestly; not a reason to delay this change.

**Net: zero remaining pre-ship YELLOW gates for the two-stream split.** Every consumer currently in
`sim/` is PROVEN-routed. Y3 is a future port task; Y4 is a separate bug; Y5 is an accurate scope
statement.

## 9. Files touched (implementation phase — for the later plan, NOT this run)

`src/sim/world/mod.rs` (two fields + accessors + `with_seed` + `seed` field + the ~7 dispatch
borrows at 553/1008/1437/1831/1843/1906/1919/1932/1944), `src/sim/world/world_hash.rs:39` (hash
both), `src/sim/snapshot.rs` (SNAPSHOT_VERSION bump only), `src/app_sim_tick.rs:260` (replay seed
fix), and the accessor-swap at every call site in §3 plus the `reseed_both` test helper. **No change
to `src/sim/rng.rs`** (`SimRng` is already gamemd-correct).
