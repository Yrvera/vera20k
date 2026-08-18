# LogicClass tick order + RNG streams — VERA20k substrate near-completeness audit

*Date: 2026-06-04. Branch: factory-house-substrate-p1p2. Method: 17-agent read-only workflow (7 audit lanes → adversarial per-lane verify → synthesize → completeness critic → finalize). Default verdict on any unproven divergence is DRIFT. No code or docs were edited during the audit.*

*Scope: the object-substrate near-completion frontier — LogicVector/ObjectSubstrate, the lifecycle chokepoints, the no-op object-AI shell, the `advance_tick` phase ladder vs gamemd's `LogicClass::PerTickUpdate`, the live-order/keys_sorted consumers, RNG stream routing + draw order + count, and save/load/hash of active order + RNG.*

> **Line-citation note (re-verified in the finalize pass):** the deferred-delete drain call sites are `flush_pending_delete()` at `mod.rs:1903` (end-of-tick / P9, inside `run_late_region` which is *defined above* `advance_tick`), `:1954` (command boundary), and `:2477` (end-of-P5). `uninit` body begins `mod.rs:1184`; `flush_pending_delete` body begins `:1236`. Verified via `Grep "flush_pending_delete()"` → hits `1903,1954,2477` and `Grep "fn (uninit|flush_pending_delete)"` → `1184,1236`. The "3 in-`advance_tick` + 1 app-layer (`app_sim_tick.rs:316`) = 4 total vs gamemd's 1" count is correct. (Note: some cited line numbers elsewhere in this doc are approximate/`~` where the agent did not re-pin them; re-grep before relying on an exact line.)

---

## 0. STATUS MATRIX (all required areas)

| # | Area | STATUS | One-line basis |
|---|---|---|---|
| **A1** | `LogicVector` type invariants (push/remove/snapshot/serde) | **MATCH** | tail-append, order-preserving retain, verbatim clone, transparent Vec — all bit-identically tested (`logic_vector.rs:24-36,80-124`) |
| **A2** | `LogicVector` membership *scope* (entities vs anims/bullets/particles) | **DRIFT** | Rust holds only the 4 entity categories; gamemd appends anims/bullets to the SAME live vector (`ANIMCLASS:43-56`) |
| **A3** | `ObjectSubstrate` consolidation (counters/occupancy/store/queue) | **PARTIAL** | substrate owns the data; lifecycle methods still physically on `Simulation` (`substrate.rs:48-77` vs `mod.rs:1184-1298`) |
| **B1** | `register_live_object` / `unregister_live_object` | **PARTIAL** | flag-guard + tail-append + compacting-remove MATCH; reveal gate-chain (`+0x234`/`+0x90`/Mark-PUT) not enforced (`mod.rs:774-811`) |
| **B2** | `reveal` / `conceal` | **PARTIAL** | thin membership delegates; no Mark-PUT-failure revert, no inline occupancy, no Deselect-first ordering (`mod.rs:816-824`) |
| **B3** | `unlimbo` / `create_limbo` | **PARTIAL** | insert→reveal→count→occupancy order MATCH; omits Sight+3 fog reveal, Added_To_Game, deploy-fire, Reveal-failure retry (`world_spawn.rs:539-568`) |
| **B4** | `uninit` + `flush_pending_delete` (two-phase death) | **DRIFT** | detach/conceal/enqueue MATCH, but **4 drain points** vs gamemd's single end-of-`Main_Tick` drain (`mod.rs:1184-1298,1903,1954,2477`; `app_sim_tick.rs:316`) |
| **B5** | Presence FSM (Limbo\|InCell\|Dying) + derived asserts | **PARTIAL** | Limbo/InCell shadow InLimbo; `Dying` is Rust-only, un-derivable; assert coupled to the multi-drain (`game_entity.rs:177-187,510-516`) |
| **C1** | `object_ai_stage` + `for_each_live_object` walk primitive | **MATCH** (walk) / **MISSING** (behavior) | count-reload + compacting-skip bit-identically tested; only the no-op shell consumes it (`mod.rs:942-949`, `techno_ai.rs:45-114`) |
| **C2** | `live_object_order_snapshot` consumer routing | **PARTIAL** | every behavior-bearing pass uses the point-in-time snapshot, not the live walk (`mod.rs:924`) |
| **D1** | `advance_tick` phase order vs `PerTickUpdate` ladder | **DRIFT** | object loop split into P1/P5/P6; pre-object globals run post-object; Factory authoritative-stepping but at wrong slot, House not in tail (§1 matrix) |
| **D2** | Map-load active-order seeding | **DRIFT** | reveals in `MapEntity` slice order, not gamemd's section sequence (`world_spawn.rs:47`) |
| **E1** | Frame-counter commit timing (LATE) | **MATCH** | single late write-site at `mod.rs:1921-1923`; `execute_tick` provably never written early (exhaustive enumeration) |
| **F1** | Pre-object global rungs present at correct slot | **DRIFT** / **MISSING** | ore/SW/AI/effects run post-object; bridge-shroud-120, bombs-global, RadSite, EMP, wave-splash, crate-regen absent (§1 matrix) |
| **G1** | Live-order consumers (movement/combat/retal/miner/passenger) | **PARTIAL** | snapshot-based, blind to same-pass membership change (`mod.rs:2027,2072,2284,2484`, `miner_system.rs:106`, `passenger.rs:355`) |
| **G2** | Shared-resource contention consumers (repairs/docks/producer) | **DRIFT** | resolved in stable-id order, not live order (`production_sell.rs:779-817`, `aircraft_dock.rs:288`, `production_tech.rs:569`) |
| **G3** | Capture/C4/bridge-repair multi-actor consumers | **DRIFT** | stable-id winner-on-shared-target; bridge `key_idx += 2` is a stable-id surrogate (`world_orders.rs:187,455,275,420`) |
| **G4** | Order-agnostic consumers (power/vision/deploy/turret/gate/AoE) | **MATCH** | per-owner/per-entity isolated or total-order tiebreak (verified bodies) |
| **H1** | Scenario/main/mapgen stream identity + seeding | **MATCH** | both gameplay streams seed identically; mapgen zeroed-unseeded (`mod.rs:519-521,618-624`) |
| **H2** | Per-accessor *instance* routing (11 scenario + 2 main) | **MATCH** | every binary-verified consumer routes to the correct instance (incl. `random_assignment`→Scen, binary-confirmed) |
| **H3** | `random_assignment` path *behavior* | **DRIFT** | no random color draw; MP uses network callback not RNG; different draw order (`skirmish_launch.rs:291-306` vs `0x0069B8C0`) |
| **H4** | Direct `&mut self.scenario_rng` tick callsites | **MATCH** | all verified Scen consumers (movement bump/scatter, smudge, ore, TIBTRE) |
| **H5** | `mapgen_rng` usage (bridge-repair walker variant) | **MATCH** | correct instance, binary-confirmed (`disassemble_function 0x00598030`); RMG seeding deferred |
| **I1** | Cross-system scenario-cursor ORDER within a tick | **DRIFT** | ore/spread/TIBTRE draw at P7 vs gamemd's pre-object orders 9-10 — shifts the shared cursor |
| **I2** | Particle draw COUNT + variant | **DRIFT** | `next_range_u32` (mask-reject) where gamemd uses raw `Next % n` — wrong count + value |
| **I3** | Smudge/wall draw COUNT + variant | **DRIFT** | anim 50/50 raw-high-bit; wall exclusive-range + wrong boundary (`smudge_dispatch.rs:212`, `overlay_grid.rs:366-367`) |
| **I4** | Ore growth/spread COUNT + variant | **PARTIAL** | native driver variant-correct; legacy reservoir path injects phantom draws (`ore_growth.rs:1463`) |
| **I5** | Scatter/bump/sub-cell COUNT + variant | **PARTIAL** | sub-cell exact; `scatter_blocker` 8-way draw has no gamemd correspondence (`bump_crush.rs:740`) |
| **I6** | Bridge collapse/damage draw order + count + stream | **PARTIAL** (MATCH stream) | damage gate/Ion-bypass exact; debris per-cell count needs reconcile vs doc §4 |
| **I7** | Algorithm-variant family correctness (`rng.rs`) | **PARTIAL** | `_inclusive` (RandomRanged) and `_scaled` (mapgen) used correctly except at particle/anim/wall sites |
| **I8** | Passenger/garrison ejection draws | **UNCHECKED** | eject paths not traced for count/order this audit |
| **I9** | Sound-variant RNG draws | **MISSING**/**UNCHECKED** | sim emits events without a variant draw; gamemd's pick may be lockstep-relevant |
| **J1** | `state_hash` field coverage + fold order | **MATCH** | all substrate order/identity fields folded in fixed order; logic order hashed separately (`world_hash.rs:63-100`) |
| **J2** | `snapshot.rs` round-trip; pending_delete skip; verbatim vector | **MATCH** | transparent-Vec serialize; queue empty at tick-boundary saves (`snapshot.rs`, `substrate.rs:59-76`) |
| **J3** | RNG state round-trip (all 4 SimRng fields, 3 streams) | **MATCH** | no serde-skip on any field; 3-stream round-trip tested (`rng.rs:15-21`, `rng_routing_tests.rs:257,339`) |
| **J4** | Desync detection / MP checksum vs gamemd | **UNCHECKED** | the only doc evidence for "gamemd has no live full-state checksum" is the discredited `+0xD64` mislabel; the negative is now unproven (`mod.rs:2630`) |

---

## 1. advance_tick ladder vs PerTickUpdate — full rung map

Native ladder authoritative source: `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0` §Verified Binary Order (30 rungs) + `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS` §2.

| Native rung | Rust phase | Verdict |
|---|---|---|
| 1 per-tick counter setup | committed LATE `mod.rs:1921-1923` | PARTIAL (correct frame visibility, tail commit) |
| 2-4 scenario dispatch/timer/flag-clear | none | MISSING |
| 5 ShroudGrow precursor | none | N/A — `ShroudGrow=no` in YR |
| 6 RecalcBridgeShroud every 120 frames | none | **MISSING (gameplay-visible)** |
| 7 FogOfWar regrowth | none | N/A — TS-legacy, `FogOfWar=no` |
| 8 terrain-morph + Z-adjust | none | MISSING (SW-ambient conditional) |
| 9 Tiberium GrowthDriver | P7 `mod.rs:~2584` | **DRIFT** (pre-object→post-everything; RNG-shifting) |
| 10 Tiberium SpreadDriver | P7 `mod.rs:~2597` | **DRIFT** (slot wrong; growth<spread preserved) |
| 11 BombClass::UpdateAll | distributed P5 `mod.rs:~2273` | **DRIFT** (no global bomb list) |
| 12 30-frame batch helper | none | MISSING (semantics unknown) |
| 13 TeamClass AI | P8 `mod.rs:~1894` (late) | **DRIFT** — and NOT a TeamClass list (`ai::tick_ai` occupies a different role) |
| 14 DiskLaser reverse loop | none | MISSING |
| 15 effect-TTL reaper | P9 `mod.rs:~1935` | DRIFT (tail vs pre-object) |
| 16 LaserDraw UpdateAllAI | none | MISSING/UNCHECKED (render-side) |
| 17 LightningStorm::Process | P4.5 `mod.rs:~2242` | **DRIFT** — runs far earlier (pre-combat); effect fused with per-house SW charge |
| 18 RadSite reverse loop | none | MISSING |
| 19 Z-cache batch | none | MISSING |
| 20 EMPulse UpdateAll | none | MISSING |
| **21 main live-object vector** | split P1 `:2027` / P5 `:2284` / P6 `:2484` | **DRIFT** — central LARGE-MIGRATION |
| 22 conditional click-anim | none | N/A — gated off in skirmish |
| 23 WaveClass splash | none | MISSING |
| 24 AlphaShape purge | none | MISSING (render-side) |
| 25 crate-regen timers | none | **MISSING (gameplay-visible)** |
| 26 g_Tactical scroll | none in sim | N/A — app-layer |
| **27 FactoryClass array** | P7 `mod.rs:2512` (`step_all`) | **DRIFT** (authoritative-stepping but pre-object/pre-combat in Rust, tail in gamemd) |
| **28 HouseClass array** | split P4 `:~2231`/P7/P8 `:~1894`/P8.5 `:~1924` | **DRIFT** — no unified tail; defeat-after-AI inverted |
| 29 last-ref refocus | none | N/A — UI/camera |
| 30 free scratch vector | RAII | N/A |

Docs: `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0`, `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS`, `PERTICKUPDATE_FULL_ORDERING_LADDER`, `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI`, `ADVANCE_TICK_PHASE_PARTITION_NATIVE_SPINE`.

---

## 2. Cross-lane contradictions resolved

1. **Bridge/wall RNG stream identity.** First-pass flagged bridge collapse as a possible stream DRIFT citing `PER_FRAME §2.2/§3.1` (lists it under g_MainRng). **RESOLVED to MATCH:** the bridge/wall-specific docs (`APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW §2` = `Scenario+0x218`; `BRIDGE_RNG_CALL_ORDER §3.2`; `WALL_DAMAGE_RNG_CLASSIFICATION`) prove both are Scen->Random. Rust's `bridge_rng()`/`wall_damage_rng()` → scenario routing is **correct** (`mod.rs:582-587`); `PER_FRAME §2.2/§3.1` is the stale party.
2. **`random_assignment_rng` — routing vs behavior.** The *instance* is binary-confirmed Scen->Random (`0x0069B8C0`), so routing is correct (H2 MATCH); the real disparity is *behavior* (H3 DRIFT: no color draw, MP network-callback, draw order).
3. **Mind-control revert classification.** Reclassified from NEEDS-RESEARCH up to **[SMALL-IMPL]** because the gamemd behavior is binary-verified (`0x004DE5D0`, `CaptureManagerClass::FreeAll()` before `ObjectClass::UnInit`) and the Rust gap is confirmed (`uninit` clears only radio+bunker); only the controller↔controllee link representation is the open design piece.
4. **`economy.credits == house.credits` assert.** It **does** exist (`debug_assert_economy_shadow mod.rs:991-999` → `debug_assert_production_shadow mod.rs:1022`). That candidate gap is **removed**.
5. **Pending-delete drain count.** Authoritative: **3 inside `advance_tick` (`:1954`, `:2477`, `:1903`) + 1 app-layer (`app_sim_tick.rs:316`) = 4 total** vs gamemd's 1.
6. **`step_all` exists and is authoritative.** `FactoryRegistry::step_all` (`factory.rs:679`) is called live every tick at `mod.rs:2512`, charging each armed factory's per-step cost against the **real** wallet (`house.credits`) in `insertion_seq` temporal order (`factory.rs:684-733`, load/store `:722-726`). This is the P5a authority work (commit `dc7a34d9`). **Consequence:** any earlier "factory registry is still DERIVED shadow / no whole-registry stepping" claim is wrong — the registry is **authoritative-stepping**; the remaining DRIFT (rung 27, D1) is purely the *slot* and the `(owner,category)`-map-then-`insertion_seq`-sort *order* vs the native global FactoryClass-array insertion order (gap 20). J1 determinism rationale is VERIFIABLE: `insertion_seq` is a strictly-monotonic enqueue counter (no ties), and the hash fold uses that same order (`world_hash.rs:285`).
7. **Accessor count is 11 scenario + 2 main.** scenario block `mod.rs:573-605` (`scatter, subcell, smudge, wall_damage, bridge, ore, anim, particle, superweapon, miner_jitter, random_assignment`); main block `:608-613` (`weapon_spread, house_ai`). `mapgen_rng` has no accessor (consumed via `&mut self.mapgen_rng`).

---

## 3. Consolidated stale-docs list

Every doc below has accurate gamemd-behavior content unless noted; staleness is mostly Rust-side line drift or a superseded routing/label claim.

| Doc | What is stale |
|---|---|
| `LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES` | `:50` "No Rust equivalent was found" (now `for_each_live_object`+`in_logic_vector`+`LogicVector`); `:48` `advance_tick @ :1508` (now `:1980`); high-risk-pass row lists air/teleport/tunnel/rocket/homing/droppod/movement/combat as plain sorted snapshots — they now consume `live_order` (partially stale) |
| `LOGICCLASS_PERTICKUPDATE_SCHEDULER` | OQ-LCPU-014 `:146` + §9 `:154-156` "no separate active-list membership bit/list" — now exists |
| `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN` | `:159` `reveal/register_live_object @ :680` (now `:816`/`:774`); `:160` `live_object_order_snapshot @ :745` (now `:924`) |
| `TECHNOCLASS_AI_MIGRATION_BOUNDARY` | §3.3/§6 `advance_tick @ :1508`, movement `:1534`, combat `:1732` (now `:1980`+) |
| `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0` | §6/§9/Sources stale line refs (now `:774-811`, `:942`, `:1980`, `game_entity.rs:177/:244`) |
| `SLICE6_DEFERRED_DELETE_DYING_WINDOW` | §6 "port frees on `uninit` immediately (no deferred queue/Dying window)" — now exists (`substrate.rs:69-76`); §10 single-drain contract is the contract Rust **violates** (flag the violation) |
| `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0` | §Current Rust Order line refs; "Frame counter visibility" labels late-commit "DRIFT risk" — should read MATCH (`:1921-1923`) |
| `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS` | §4/§7 stale layout line refs |
| `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI` | stale Rust line refs; any "factory registry is DERIVED shadow / oracle-clone only" claim now wrong — `step_all` charges the real wallet (`mod.rs:2512`, `factory.rs:722-726`) |
| `RNG_SYSTEM_GHIDRA_REPORT §3.1` | line 169 lists LightningStorm under g_MainRng; line 381 "everything else uses g_MainRng" — WRONG; lightning+ore+smudge+particles are Scen->Random (`0x0053A300`, `0x0062B5E0`) |
| `PER_FRAME_RNG_CONSUMPTION_ORDER §1/§2.2/§3.1` | assigns particles/wall/bridge-collapse/ore/lightning/smudge to g_MainRng — WRONG/STALE; all Scen->Random |
| `SAME_TICK_SPAWNED_OBJECT_BEHAVIOR §4` (`:113`) | "Rust miner processing still uses `keys_sorted()`" — now `live_object_order_snapshot()` (`miner_system.rs:106`) |
| `LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY` (`:84-95`) | old miner `keys_sorted` claim — now live-order at `:106` |
| `SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION` | `:109-116` stale line refs; §6/§8 "Rust serialization unchecked" — order IS serialized verbatim + tested |
| `ORE_TIBERIUM_RNG_CLASSIFICATION §3.1/§3.2/§6` | marks `terrain_spawn.rs` `next_range_u32(1_000_000)` + "immediate spawn" RED — now raw `next_u32()` (`:78`) + two-phase midpoint spawn (`:159-200`); stale line cites |
| `SYNC_CHECKSUM_MAINTICK_OBJECT_SUM §3.1` + addr table | labels `FUN_006d6170 @ +0xD64/0xD68` an "8-byte state_hash from ScenarioClass" — WRONG: it is the g_Tactical camera **scroll (x,y)** pair (re-verified `decompile_function 0x006D6170`, caller only `Main_Tick`) |
| `DESYNC_DETECTION_MAINTICK_COMPARE §2.1` | inherits the same `+0xD64` "state_hash" mislabel; with that discredited, the "no live MP hash compare" negative is now UNCHECKED |

Stale Rust **comments** (cosmetic): `game_entity.rs:509` ("after which the slot is freed in the same call" — false post-deferral); `factory.rs:97` ("In P2/P3 it is DERIVED shadow" — false, registry steps authoritatively via `step_all`).

---

## 4. Full remaining-gap list (de-duplicated)

### Structure / membership
1. **LogicVector membership scope** omits anims/bullets/particles that gamemd appends to the same live vector. (A2) — **[LARGE-MIGRATION]**
2. **ObjectSubstrate lifecycle consolidation** — reveal/conceal/unlimbo/uninit physically on `Simulation`, not the substrate. (A3) — **[LARGE-MIGRATION]**
3. **Map-load active-order section sequence** — slice-order reveal vs native section sequence. (D2) — **[LARGE-MIGRATION]** (gated on map parser exposing section order)
4. LogicVector capacity/grow-failure path absent. Unreachable in practice. (A1) — **[TEST-ONLY]**

### Lifecycle gate-chain (merged B1/B2/B3)
5. **Reveal gate-chain + placement-failure revert** — register/reveal/unlimbo register unconditionally; gamemd gates on `+0x234` type-flag, `+0x90` IsAlive, Mark-PUT success, reverting `InLimbo=1` on failure. Player-visible: factory exit onto a fully-blocked cell stays registered-but-unplaced instead of limbo-retry. — **[SMALL-IMPL]** (gate+revert) + **[NEEDS-RESEARCH]** (Rust type→`+0x234` eligibility map)
6. reveal/conceal do not encapsulate occupancy add/remove. — **[SMALL-IMPL]**
7. **Unlimbo omits TechnoClass::Unlimbo extras** — Sight+3 fog reveal inline, Added_To_Game, deploy-fire; same-tick vision timing vs native (deferred to P3). — **[NEEDS-RESEARCH]**

### Pending-delete + Presence
8. **Pending-delete multi-drain cadence** — 3 drains in `advance_tick` + 1 app-layer vs gamemd's single end-of-`Main_Tick` drain. Root cause = vision/power/production/AI raw-store consumers have no dying-gate. — **[LARGE-MIGRATION]**
9. **Mind-control revert on controller death** — `uninit` clears only radio+bunker; gamemd's `FootClass::UnInit` calls `CaptureManagerClass::FreeAll()` first. — **[SMALL-IMPL]** (only controller↔controllee link model is open; `mind_controlled` is a lone bool, `game_entity.rs:355`)
10. App-layer anim-end uninit+drain staging vs gamemd end-of-tick destroy. — **[NEEDS-RESEARCH]**
11. **Presence conflates InLimbo + IsAlive** — `derived_presence` can't reconstruct `Dying`; the per-tick assert only passes because all corpses are force-drained first. Fix: `derived_presence` consults `dying`. — **[SMALL-IMPL]** + **[TEST-ONLY]**

### Object-AI absorption (merged)
12. **No `TechnoClass::AI_Update` owner** — mission dispatch, cloak/gattling/spawn-slave/target-validation/weapon-reload/EMP/temporal/health-smoothing scattered or absent. — **[LARGE-MIGRATION]**
13. **No `FootClass::AI` body** — tib self-heal, veteran promote, movement counter, rank/falling anim, IPiggyback swap, enter-transport, team-AI, idle scatter not unified; locomotor `Process` runs as global P1/P2 sweeps, not after per-object mission dispatch. — **[LARGE-MIGRATION]**
14. **No UnitClass post-Foot order** — turret rotation is a post-combat global sweep, not `Fire_At_Target`→`Facing_Update` inside the object body. — **[LARGE-MIGRATION]**
15. **Live-walk contract built but unexercised** — only the no-op `object_ai_stage` uses `for_each_live_object`; movement/combat/retaliation/miner/passenger all use the point-in-time snapshot, blind to same-pass register/unregister. — **[LARGE-MIGRATION]**

### Phase order
16. **Ore growth/spread/TIBTRE at P7** (post-combat) vs gamemd pre-object orders 9-10 — RNG-stream-shifting. — **[LARGE-MIGRATION]**; TIBTRE interleave+midpoint — **[NEEDS-RESEARCH]**
17. **Factory production at wrong slot** — authoritative `step_all` (`mod.rs:2512`) charges per-house credit at P7 but gamemd's FactoryClass array runs in the post-object tail (rung 27). Reorder vs miners/combat-ore-reduction shifts the shared near-broke credit pool. — **[LARGE-MIGRATION]** *(SLOT/ORDER gap, not authority — wallet already real)*
18. **Defeat-after-AI inversion** — AI at `~1894` then defeat at `~1924`; a house that should be defeated this tick can still apply AI commands. — **[SMALL-IMPL]**
19. **No unified per-house tail** (SW-ready/defeat/AI-manage in HouseClass::Update internal order). — **[LARGE-MIGRATION]** (gated on AI rewrite)
20. **Multi-factory same-frame completion order** — `step_all` sorts by `insertion_seq`; gamemd walks the native global FactoryClass array in array-insertion order. Equivalent only if those orders match under all add/remove sequences (unproven). — **[NEEDS-RESEARCH]**
21. **LightningStorm pre-combat + fused with per-house SW charge** vs gamemd rung-17 pre-object + per-house charge in HouseClass tail. — **[SMALL-IMPL]**
22. **Absent pre-object global rungs** — bridge-shroud-recalc-120 (6), BombClass::UpdateAll global (11), DiskLaser (14), RadSite (18), Z-cache (19), EMPulse (20), WaveClass splash (23), crate-regen (25). Bridge-shroud-120 + crate-regen are the gameplay-visible ones. — **[NEEDS-RESEARCH]** per rung

### Live-order / contention consumers
23. **Capture/C4/bridge-repair multi-actor-on-one-target** uses stable-id, not live-vector order; bridge-repair `key_idx += 2` is a stable-id surrogate. — **[SMALL-IMPL]** per pass
24. **Repairs shared per-owner wallet** resolved in stable-id order (`tick_repairs` iterates `.values()`, `production_sell.rs:781`). — **[SMALL-IMPL]**
25. **Dock/pad claim tiebreak** stable-id (`aircraft_dock.rs:288`; building service-slot). — **[SMALL-IMPL]** each
26. **Producer-building selection base order** stable-id (`production_tech.rs:569`) + `find_helipad_for_aircraft` first-match (`production_spawn.rs:731`). — **[NEEDS-RESEARCH]**
27. **`tick_unloading` multi-transport eject + cell-claim** stable-id (`passenger.rs:913-940`). — **[SMALL-IMPL]**
28. **`slave_miner.rs:136` cross-master resource contention** order-sensitive (keys_sorted). — **[SMALL-IMPL]**
29. bunker_install ClearWait cross-scatter on shared footprint (keys_sorted, `bunker_install.rs:71`). — **[SMALL-IMPL]** (assumed; not re-verified beyond the keys_sorted basis)
30. **No global AnimClass live-pool + first-AI guard** — anims embedded per-entity (`animation.rs:396/534/561`). — **[LARGE-MIGRATION]**

### RNG behavior
31. **`random_assignment` SP color+order + MP network-callback** — no random color draw; MP uses `vtable+0x6c/+0x70` (zero RNG); different order. Offsets the scenario cursor before tick 0. — **[SMALL-IMPL]** (SP) + **[NEEDS-RESEARCH]**/**[LARGE-MIGRATION]** (MP)
32. **Particle draws use `next_range_u32` (mask-reject)** where gamemd uses raw `Next % n` — wrong COUNT + value. Sites: `particles/{spawn:96/99/229, fire:65/116, smoke:88/89/178/213, gas:86/87/198}`. — **[SMALL-IMPL]** per site
33. **Legacy ore reservoir-sample** (`ore_growth.rs:1463`) injects phantom Scen draws; live when native tiberium classes aren't loaded. — **[LARGE-MIGRATION]**
34. **Anim scorch/crater 50/50 raw-high-bit** (`smudge_dispatch.rs:212`) — gamemd uses `RandomRanged(0,0x7FFFFFFE)` normalized + `0x7FFFFFFF` rejection. Differs in COUNT. — **[SMALL-IMPL]**
35. **Wall damage exclusive-range + wrong boundary** (`overlay_grid.rs:366-367`) — `next_range_u32(strength)` (`[0,S-1]`) + `roll > damage` vs gamemd inclusive `[0,S]` + `roll >= damage` no-op. — **[SMALL-IMPL]**
36. **`scatter_blocker` 8-way draw** (`bump_crush.rs:740`) has no gamemd correspondence. — **[SMALL-IMPL]** wiring + **[NEEDS-RESEARCH]** per-class draw set
37. Idle scatter dormant draw (`scatter.rs:123`) wrong if re-enabled (commented out at `mod.rs:2494-2502`). — **[NEEDS-RESEARCH]**
38. **Bridge per-cell debris draw COUNT** — Rust up to 6 draws/cell vs `BRIDGE_COLLAPSE_CHAIN_MECHANISM §4`'s 4. — **[NEEDS-RESEARCH]**
39. Warhead-detonate is dual-stream (`0x004690B0`): spread→main, debris/anim/scorch→scenario; future detonate impl must split-route. — **[SMALL-IMPL]** guard-rail
40. Verify `flat_tiberium_variant_ids` returns exactly 12. — **[TEST-ONLY]**
41. Native growth reinsert spread-feed draw count (`ore_growth.rs:583`). — **[TEST-ONLY]**
42. `miner_jitter_rng` instance not pinned to a per-system live-disasm doc. — **[NEEDS-RESEARCH]** (low-risk)
43. Passenger/garrison ejection draws not traced (SellBuilding makes NO `%8` draw; post-Unlimbo Scatter may draw `RandomRanged(0,4)`). — **[NEEDS-RESEARCH]**
44. Sound-variant RNG pick — confirm whether gamemd's pick is lockstep-relevant. — **[NEEDS-RESEARCH]**
45. MP/host seed handshake unimplemented (net layer). — **[NEEDS-RESEARCH]** now / **[LARGE-MIGRATION]** when net exists
46. RMG seeding of `mapgen_rng` from `MapSeed+0x74` deferred. — **[NEEDS-RESEARCH]**/**[LARGE-MIGRATION]** when RMG lands
47. Bridge-walker draw-primitive parity (`walker.rs:415` `next_range_u32(4)` vs gamemd `Random__Next + ftol` limit 3). — **[NEEDS-RESEARCH]**
48. Guard test: no live gameplay site uses `_scaled` (multiply-high). — **[TEST-ONLY]**
49. SimRng-vs-RandomClass full-input-space equivalence pinned only at seed=1. — **[TEST-ONLY]**

### Save/load/hash
50. No `debug_assert!(pending_delete.is_empty())` in `GameSnapshot::save`; correct today only by call-site discipline. — **[TEST-ONLY]**
51. No golden-preimage regression pinning the whole `state_hash` fold order + `SimRng` serde layout. — **[TEST-ONLY]**
52. No round-trip test for a non-empty `pending_delete` at serialize time. — **[TEST-ONLY]**
53. Net-layer command-stall must gate `advance_tick` (not hash-compare-then-abort) when net is built. — **[NEEDS-RESEARCH]**
54. Patch the two docs' `+0xD64` "state_hash" mislabel (camera scroll). — **[NEEDS-RESEARCH]** (doc-correction; facts in hand)
55. **Verify gamemd has no live full-state checksum compare in `Main_Tick`** — J4 MATCH lost its evidence when `+0xD64` was discredited; need a fresh `decompile_function` of `Main_Tick`. — **[NEEDS-RESEARCH]**

---

## 5. Gaps by bucket

**[SMALL-IMPL]** — 5, 6, 9, 11, 18, 21, 23, 24, 25, 27, 28, 29, 31a, 32, 34, 35, 36a, 39.
**[LARGE-MIGRATION]** — 1, 2, 3, 8, 12, 13, 14, 15, 16, 17, 19, 30, 33, 31c, 45/46 (net/RMG when those land).
**[NEEDS-RESEARCH]** — 5b, 7, 10, 16b, 20, 22 (per rung), 26, 31b, 36b, 37, 38, 42, 43, 44, 47, 53, 54, 55.
**[TEST-ONLY]** — 4, 40, 41, 48, 49, 50, 51, 52, + the test half of 11.

---

## 6. Next 5 slices (ranked by downstream impact × risk)

> Ranking reasons from the north star: a lockstep-correct, indistinguishable-from-gamemd client. Slices that **shift the shared scenario RNG cursor or the hashed live order** desync *every* match deterministically (highest impact, must-fix-first); the cheap unblocked [SMALL-IMPL]s go before the spine-touching [LARGE-MIGRATION]s gated on the object-AI decision.

### SLICE 1 — Particle RNG raw-modulo conversion (gap 32) · [SMALL-IMPL]
**What:** add one raw signed-abs-modulo helper (`abs(next_u32()) % n`) and replace every `next_range_u32(n)` at particle lifetime/jitter/offset/insert sites (`particles/{spawn:96/99/229, fire:65/116, smoke:88/89/178/213, gas:86/87/198}`).
**Why rank 1:** highest impact-per-risk. Particles spawn on nearly every explosion; each `next_range_u32` rejection advances the **shared scenario cursor** a variable number of times, desyncing every later scenario consumer in the tick — a guaranteed lockstep break that fires constantly. Mechanical per-site swap, no spine touch; `PARTICLE_RNG_CLASSIFICATION` is current.
**Dependencies:** none. The helper it adds is reused by SLICE 2.

### SLICE 2 — Smudge 50/50 + wall-damage RNG variant fix (gaps 34, 35) · [SMALL-IMPL]
**What:** replace anim scorch/crater raw-high-bit (`smudge_dispatch.rs:212`) with `RandomRanged(0,0x7FFFFFFE)` normalized + `0x7FFFFFFF` rejection (accept `<0x40000000`); switch wall damage (`overlay_grid.rs:366-367`) to `next_range_u32_inclusive(0,strength)` and return when `roll >= damage`.
**Why rank 2:** same cursor-shift class, fires on common events (every scorch/crater anim, every wall hit below strength). Bounded two-site fix with boundary algebra already worked out. Slightly lower frequency than particles.
**Dependencies:** shares the raw-modulo helper from SLICE 1 (sequence SLICE 1 first).

### SLICE 3 — `random_assignment` SP color + draw-order parity (gap 31a) · [SMALL-IMPL]
**What:** in `resolve_random_assignments` (`skirmish_launch.rs:291-306`) add the random **color** draw (`RandomRanged(0,7)` with collision-retry) per human node and AI slot, in gamemd's node/slot order (all humans country→color, then all AI), matching `0x0069B8C0`.
**Why rank 3:** offsets the scenario cursor **before tick 0**, desyncing the entire match from the first tick; fires every skirmish. Moderate risk: draw order/grouping must match exactly (collision-retry loop count is part of cursor advance). Instance routing already correct. MP-callback branch out of scope (net not built) — SP-only.
**Dependencies:** none (SP path).

### SLICE 4 — Decouple Presence FSM from drain timing (gaps 11, 18) · [SMALL-IMPL]
**What:** make `derived_presence` (`game_entity.rs:510-516`) return `Dying` when `dying` is set, so `debug_assert_presence_consistent` no longer depends on all corpses being force-drained first. Bundle the cheap **defeat-before-AI** reorder (gap 18): move `check_defeat` (`mod.rs:~1924`) before the AI-manage step.
**Why rank 4:** pure invariant-hardening that **unblocks** SLICE 5 — today the Presence assert would false-fire the moment you reduce the drain count. Low risk; defeat reorder is a verified [SMALL-IMPL] DRIFT with a player-visible effect (a dead house acting).
**Dependencies:** none; prerequisite-enabler for SLICE 5.

### SLICE 5 — Add dying-gates to raw-store consumers, then collapse to one drain (gap 8) · [LARGE-MIGRATION]
**What:** add a `dying`-gate to every raw-store consumer that currently relies on the early drains (vision P3, power P4, production P7, AI P8, particles P5.5, retaliation), then remove the command-boundary (`:1954`) and end-of-P5 (`:2477`) drains, leaving the single end-of-tick drain (`:1903`) to match gamemd's `ProcessPendingDelete`.
**Why rank 5:** highest *structural* impact (restores gamemd's Dying-window visibility to mid-tick systems — kill-credit, last-attacker, power/vision counting) but highest *risk*: touches the spine and every raw-store consumer; the current drains are masking real consumer bugs (in-code comments at `:1946-1953` and `:2470-2476` admit this). Must come **after** SLICE 4. Gateway to the eventual object-AI absorption (gaps 12-17) but stops short of it.
**Dependencies:** SLICE 4 (Presence must derive `Dying` first).

*(Deliberately NOT top-5: object-AI absorption / live-walk consumer migration / ore-reslot / factory tail-reslot (gaps 12-17) — the deepest LARGE-MIGRATIONs, gated on the authoritative-projectile decision and `feedback_no_ai_yet`, premature before SLICE 5 normalizes the death window.)*

---

## 7. Acceptance tests per slice

**SLICE 1 (particle RNG) — hash-NOT-neutral by design.** Fixture sim that spawns a known particle burst (railgun MaxEC=80); assert the scenario cursor advances by exactly `count` raw draws (one per spawn), not the variable rejection count. Assert produced lifetimes equal `abs(Next) % MaxEC` for seed=1 against a hand-computed table. Assert `state_hash` changes from the pre-fix baseline (cursor moved) then is stable across re-runs. Regression: fail if any `particles/*` site reintroduces `next_range_u32`.

**SLICE 2 (smudge/wall) — hash-NOT-neutral.** Wall: assert no-op at `roll == damage`, draw-consumed-then-no-damage; assert advance only when `roll < damage`; pin inclusive range `[0,strength]`. Smudge: assert scorch chosen when masked `< 0x40000000`, and a masked `0x7FFFFFFF` triggers a redraw (extra cursor advance). Compare cursor after a fixed scorch+crater+wall-hit sequence against a hand-derived count.

**SLICE 3 (random_assignment) — hash-NOT-neutral (pre-tick-0 offset).** Given a fixed config (N humans, M AI, all "random"), assert the scenario cursor advances by exactly `2*(N+M)` draws (country+color) in gamemd node/slot order, retries counted. Assert resulting (country,color) tuples match a seed=1 golden table cross-checked against `0x0069B8C0`. Assert tick-0 `state_hash` differs from baseline.

**SLICE 4 (Presence + defeat) — hash-neutral for Presence (debug-assert + unhashed serde-skip field only).** Assert `state_hash` byte-identical before/after across a fixture run. Unit test: construct a `Dying` entity surviving to the assert; assert `derived_presence()` returns `Dying` and the assert passes. Defeat reorder: assert a house whose last building dies this tick issues no AI command the same tick. Verify hash-neutrality only if no house's defeat status changes on the boundary tick; if it does, this is a correctness fix and the hash *should* change — pin the new value.

**SLICE 5 (dying-gates + single drain) — hash-NOT-neutral (intended).** Per raw-store consumer: a corpse uninit'd this tick must be **excluded** by the dying-gate from vision/power/production/AI/particle/retaliation counts. Critical kill-credit + last-attacker tests: an instant-hit kill of B before B's turn must make B's `last_attacker` and any retaliation reflect the Dying window exactly as gamemd (`SLICE6 §8`). After collapsing to one drain, assert membership + presence asserts still pass and `state_hash` is deterministic across re-runs; compare a multi-death tick's hash against a reference trace if one exists.

---

## 8. DO NOT REWRITE YET (and why)

1. **The no-op `object_ai_stage` / `techno_ai_shell` (`techno_ai.rs:45-114`).** Keep it a strict no-op until absorption is *sequenced*. The shell, the `for_each_live_object` walk, the S1 shadow, and the P2 factory trace are correct, bit-identically tested scaffolding. Filling an arm now — before SLICE 5 normalizes the death window and before the authoritative-projectile decision — would absorb behavior into a per-object pass with the wrong death-visibility semantics and have to be re-done.

2. **The subsystem-phase split (movement P1 / combat P5 / retaliation P6 over snapshots).** The central [LARGE-MIGRATION] (gaps 12-17). Leave it until (a) SLICE 5 has added dying-gates and collapsed the drains, and (b) the authoritative-bullet/`BulletClass` decision is made (it governs instant-hit death-before-fire ordering). The snapshots are behavior-neutral *today* precisely because nothing registers/unregisters mid-pass and re-reads.

3. **Hashed-order contracts:** the `state_hash` fold order (`world_hash.rs:63-100`), the verbatim `LogicVector` serialize (`logic_vector.rs:62-74`), the `SimRng` serde layout (`rng.rs:15-21`), and the factory `insertion_seq` sweep+fold order (`factory.rs:686-691`, `world_hash.rs:285`). Do not reorder or "clean up" — they are the lockstep/replay contract; any reorder silently invalidates every existing save and reference hash. Lock them with a golden-preimage test (gap 51) *before* anyone touches them.

4. **The `_scaled` (multiply-high) vs `_inclusive` (mask-reject) RNG split (`rng.rs:173/190`).** `_scaled` is correctly reserved for unseeded mapgen / bridge-repair-walker; `_inclusive` is the `RandomRanged` match. Do not unify — they consume different draw counts. (SLICES 1-2 fix wrong-family *call sites*, not the helpers.)

5. **The map-load slice-order reveal (`world_spawn.rs:47`).** Do not "fix" by guessing a section order — the native sequence (gap 3) is gated on the map parser preserving/exposing section order; faking it trades one wrong order for another. Spec the parser side first.

6. **The command-region synchronous drain (`mod.rs:1954`).** Part of the multi-drain DRIFT (gap 8), but removing it before the raw-store dying-gates exist (SLICE 5) re-introduces the "vision/power count a just-removed object" bug it was added to paper over (comment at `:1946-1953`). It comes out *with* SLICE 5.

7. **The authoritative factory sweep order (`step_all` at `mod.rs:2512`, sweeping `insertion_seq` order).** The registry is **authoritative-stepping** — `step_all` charges the real `house.credits` per step (`factory.rs:722-726`, commit `dc7a34d9`). Remaining DRIFT is the *slot* (gap 17) and *order key* (gap 20). Do not move the sweep to the gamemd tail until the per-house-tail/AI rewrite is scheduled — it is RNG-cursor-shifting and gated on `feedback_no_ai_yet`. The hash fold keys off this sweep order (`world_hash.rs:285`), so reordering invalidates saves (see #3).

---

*Net assessment: the substrate's data shapes, walk primitive, frame-counter timing, RNG instance routing, factory authoritative-stepping, and save/load/hash are at or near MATCH and bit-identically tested. The near-completeness frontier is (a) **draw-count/variant RNG bugs** that shift the shared scenario cursor every match (SLICES 1-3, cheap, must-fix), (b) the **deferred-death cadence** that hides corpses earlier than gamemd (SLICES 4-5, the gateway migration), and (c) the still-empty **object-AI absorption** + phase-order normalization — including the factory *slot* reslot now that the factory *authority* is already real (the deep LARGE-MIGRATIONs, correctly deferred). Two doc families (`PER_FRAME`/`RNG_SYSTEM` stream tables; `SYNC_CHECKSUM`/`DESYNC` `+0xD64` "state_hash" mislabel) actively misroute future work and should be patched regardless of code slices; the latter also leaves J4 (no-live-MP-checksum) UNCHECKED until a fresh `Main_Tick` decompile re-establishes the negative.*
