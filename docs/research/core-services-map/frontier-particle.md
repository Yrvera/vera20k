# Core Service Profile — frontier-particle

**Slug:** `frontier-particle`
**Service:** ParticleSystemClass / ParticleClass — lightweight visual-effect particle systems (smoke plumes, gas/poison clouds, flamethrower fire streams, electrical sparks, railgun beam trails)
**Status:** FRONTIER (promoted from catalog stub D3 in `_frontier.md`). No prior substrate study under `core-services-map/`; this profile consolidates the deep, already-Ghidra-verified particle reports into the core-services map.
**Primary docs (existing, Ghidra-verified):**
- `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` — the end-to-end report: class layouts (type+instance, offsets), AI dispatch, INI keys, integration/callers, render, lifecycle, save/load (IPersistStream), wind tables, exhaustive detail + verification passes.
- `docs/research/PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` — every particle RNG call site classified (raw `Random__Next() % n` vs `RandomRanged`), bounds + consumption order, lockstep handoff.
- `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` — Spark/Railgun/Normalized timing, frame-selection, the two different BehavesLike enums.
- `docs/research/TICK_ANIMATION_VISIBLE_LEFTOVERS_GHIDRA_REPORT.md` §4 — particles are a separate frame-domain; Spark/Railgun no-op in current Rust.
- `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` §8 — `g_ExtraAnimationsEnabled` (0x00A8EB78) gate on Spark draw.
- `docs/research/TECHNOCLASS_AI_UPDATE_BODY_GHIDRA_REPORT.md` §4 — the S4b damage-particle (Spark) spawn truth table; Scen->Random binding re-confirmed.

**This profile:** edge/graph extract for the core-services map. Long content lives in the docs above.

**Evidence base / re-verification note:** No Ghidra instance was reachable this session
(`list_instances` → none; `connect_instance gamemd` → TCP refused; no Java/Ghidra process found;
no MCP bridge port listening). The addresses below were therefore **NOT re-run live this session**.
They are carried from the `[ghidra/verified]` reports listed above, which cite exact
`decompile_function` / assembly-line evidence inline. Treat every address as
**VERIFIED-VIA-CITED-DOC**, not VERIFIED-LIVE-THIS-SESSION. Re-run the representative
`ParticleSystemClass::AI @ 0x0062FD60` and the cross-service edges against the binary before any
implementation.

---

## Stub corrections (representative address + plug point)

The seed stub (D3 in `_frontier.md`) was largely correct on addresses; two items refined:

1. **Representative function — CONFIRMED.** Stub named `ParticleSystemClass__AI @ 0x0062FD60`
   (dispatch) as the representative fn. **Confirmed** by `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`
   §3.1 / Sources ("`0x0062fd60` — ParticleSystemClass::AI (dispatch)"), and it is the slot-23
   (`vt+0x5c`) AI head the spine dispatches over the live object vector. The per-type sub-AI
   addresses in the stub are confirmed with one correction: stub said `_Smoke @ 0x0062ED40`,
   `_Fire @ 0x0062F9A0`, `_Railgun @ 0x0062F230`, `SpawnParticle @ 0x0062E380` — all confirmed.
   Stub did **not** list `_Gas @ 0x0062E6D0` and `_Spark @ 0x0062E840`; both are added below
   (verified in the report Sources list). Note **two parallel AI dispatchers**: the *system* AI
   `0x0062FD60` and the *particle* AI dispatch `ParticleClass::AI_Dispatch @ 0x0062CE40` (each
   switches on its own BehavesLike, which use DIFFERENT enum orderings — see below).

2. **Plug point rung label — CORRECTED.** Stub said "PerTickUpdate object pass (rung N)". The
   verified spine (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`, 28 rungs A–AB) places the universal
   per-object AI fan-out at **Rung T (#20)**, driver `ObjectClass::AI @ 0x005F3E70` dispatched
   via `vt+0x5c` over the LogicClass live vector (`this=0x0087f778`, items `+0x04`, count `+0x10`).
   Rung T's row explicitly lists "**particles**" among the slot-23 polymorphic subclasses
   ("bullets/voxelanims/particles"). ("Rung N" was the older provisional letter; rung N (#14) in
   the verified ladder is the **Laser/draw-segment timer purge** `0x005FF390`, a *separate*
   secondary edge that the particle Spark path *fills* — see Used-by.) ParticleSystemClass is one
   of the Rung-T polymorphic subclasses; its slot-23 override is `ParticleSystemClass::AI @ 0x0062FD60`.

---

## Purpose

The **cosmetic particle-effect service** — the live `ParticleSystemClass` container and the
`ParticleClass` instances it owns, which produce the small drifting/animated effects layered over
the battlefield: building damage smoke, gas/poison/psychic clouds, flamethrower fire streams,
electrical sparks (damage, welding/repair), and railgun beam trails.

Two-layer architecture:
- **ParticleSystemClass** — a container/emitter that owns a `DynamicVectorClass<ParticleClass*>`,
  manages periodic spawning, and dispatches per-tick AI by its `BehavesLike` type. It is itself an
  `ObjectClass`-derived element registered in the global object vector and ticked by Rung T.
- **ParticleClass** — an individual particle (position, velocity, color, animation state,
  lifetime, damage counter). Particles are **NOT** in the global object vector — they are owned by
  their parent system and ticked from the system's AI, a **separate frame-domain** from the main
  object scheduler.

Each has a type class (`ParticleSystemTypeClass`, `ParticleTypeClass`) holding INI-parsed data.

It does NOT own: the armor/warhead damage kernel (gas-cloud damage delegates to
`damage-helpers`/`ReceiveDamage`), the RNG engine (`random-scenario`), the SHP/pixel rasteriser
(`frontier-blitter` via the tactical render pass), or the laser-line timer list it feeds
(`frontier-render-layer`'s LaserDraw/segment families). It *drives* those at its boundaries.

Conceptual core that is purely this service: **`ParticleSystemClass::AI` (per-tick emitter +
lifetime)** + the **five per-type system AIs** (Smoke/Gas/Fire/Spark/Railgun) + **`SpawnParticle`
family** (allocate + insert a `ParticleClass`) + **`ParticleClass::AI_Dispatch` + the per-type
particle AIs + movement dispatch** + the **two ReadINI parsers** (system/particle type data).

---

## Owns

- **The live particle-system object & its emitter state.** `ParticleSystemClass` (inherits
  `ObjectClass`, type 0x12 in the active vector). Key fields verified in
  `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §2.3: a `DynamicVectorClass<ParticleClass*>` of owned
  particles (count at `param_1[0x33]`), `is_active` (`+0x24`), `marked_for_deletion` (`+0x3E`),
  `Lifetime` (`param_1[0x3b]`, decremented each AI tick), `directionless` flag (`+0xF9`), system
  `BehavesLike` read at `param_1[0x2b]→type+0x2B4`.
- **The individual particle objects.** `ParticleClass` (inherits `ObjectClass`), size **0x138**
  (312 bytes, confirmed `operator_new(0x138)`): vtable `+0x000` (`0x7ef954`), CoordStruct Object
  Coords `+0x09C`, plus velocity/color/animation-state/lifetime/translucency (`+0x12F`)/damage-
  counter (`+0x12A`) fields. Particles are owned by the system's vector, NOT the global object list.
- **The system type data** `ParticleSystemTypeClass` (inherits ObjectTypeClass, ends ~`+0x310`).
  Verified offset table (§2.1): `HoldsWhat +0x294`, `Spawns +0x298`, `SpawnFrames +0x29C`,
  `Slowdown +0x2A0`, `ParticleCap +0x2A4` (default 50), `SpawnRadius +0x2A8`, `BehavesLike +0x2B4`,
  `Lifetime +0x2B8`, `SpawnDirection +0x2BC`, plus the Railgun doubles (`ParticlesPerCoord +0x2C8`,
  `SpiralDeltaPerCoord +0x2D0`, `SpiralRadius +0x2D8`, perturbation coeffs `+0x2E0/+0x2E8/+0x2F0`)
  and Spark fields (`SpawnSparkPercentage +0x2F8`, `SparkSpawnFrames +0x300`, `LightSize +0x304`,
  `LaserColor +0x308`, `Laser +0x30B`, `OneFrameLight +0x30C`).
- **The particle type data** `ParticleTypeClass` (inherits ObjectTypeClass, size **0x318**
  confirmed `operator_new(0x318)`). Verified §2.2: `NextParticleOffset +0x294`, Spark velocities
  `XVelocity/YVelocity/MinZVelocity/ZVelocityRange +0x2A0..+0x2AC`, `ColorSpeed +0x2B0`, ColorList
  vector `+0x2B8..+0x2C8`, `StartColor1/2 +0x2D4/+0x2D7`, `MaxDC +0x2DC` (frames between damage
  ticks), `MaxEC +0x2E0` (lifetime), `Warhead +0x2E4`, `Damage +0x2E8`, animation states
  `+0x2EC..+0x2FC`, `Translucency +0x2F4`, `WindEffect +0x2F8`, `Velocity +0x2FC`, `Deacc +0x300`,
  `Radius +0x304`, state-limit flags `+0x308..+0x30E`, `Normalized +0x30F`, `NextParticle +0x310`,
  `BehavesLike +0x314`.
- **The two DIFFERENT BehavesLike enums** (a known foot-gun — only Gas/Smoke swap):
  - System (`ParticleSystemTypeClass`, string table `0x00836ee0`): Smoke=0, Gas=1, Fire=2, Spark=3, Railgun=4.
  - Particle (`ParticleTypeClass`, string table `0x008370bc`): Gas=0, Smoke=1, Fire=2, Spark=3, Railgun=4.
- **The wind-drift lookup tables** (Gas vs Smoke use DIFFERENT tables; verified read):
  Gas DX `0x00836664` = `[0,2,2,1,0,-2,-2,-2]`, Gas DY `0x00836684` = `[-2,-2,0,2,2,2,0,-2]`;
  Smoke DX `0x008366a4` = `[0,2,2,2,0,-2,-2,-2]`, Smoke DY `0x008366c4` = `[-2,-2,0,2,2,2,0,-2]`
  (Smoke tables are `+0x40` from Gas). Indexed by `WindDirection` (FacingType 0–7).
- **The lifecycle: Construct → Active (registered in global object vector via Reveal) → mark for
  deletion (lifetime→0) → all particles die → Unregister + enter limbo (`DAT_00b0f698`) → session
  end/map clear destroys.** There is **NO pooling/reuse** — limbo systems persist to map clear
  (`FUN_006851f0`). Map-init also destroys a singleton system pointer `DAT_00a8ed78` (gas pool).
- **The IPersistStream Save/Load contract** (per-instance): in the PRIMARY vtable, not a secondary.
  PSC `Load 0x0062FF20` / `Save 0x00630090`; ParticleClass `Load 0x0062D7A0` / `Save 0x0062D810`;
  GetClassID PSC `0x006301A0` / Particle `0x0062D930`. Load registers the swizzle ID via
  `FUN_006CF2C0`, re-installs all 4 vtables, then `ObjectClass::Load`/`AbstractClass::Load`.

---

## Key functions & globals (addresses)

All VERIFIED-VIA-CITED-DOC (not re-run live this session — see evidence-base note above).

| Symbol | Address | Role |
|---|---|---|
| `ParticleSystemClass::AI` (dispatch) | 0x0062FD60 | **Representative fn** — slot-23 (`vt+0x5c`) per-tick AI head; switches on system BehavesLike → per-type AI, then decrements Lifetime, marks-for-deletion at 0, unregisters+limboes when active∧marked∧count==0 |
| `ParticleSystemClass::AI_Smoke` | 0x0062ED40 | Smoke system AI — periodic self-spawn (`frame % SpawnFrames == 0`), 2 raw RNG draws for offset |
| `ParticleSystemClass::AI_Gas` | 0x0062E6D0 | Gas system AI (no periodic self-spawn in verified slice) |
| `ParticleSystemClass::AI_Fire` | 0x0062F9A0 | Fire system AI — periodic spawn + fire-insertion shuffle |
| `ParticleSystemClass::AI_Spark` | 0x0062E840 | Spark system AI — `SpawnSparkPercentage` roll, `SparkSpawnFrames`, light creation |
| `ParticleSystemClass::AI_Railgun` | 0x0062F230 | Railgun system AI — front-loads path particles once (count 0 + not-deleted), spiral transform from start→target |
| `ParticleSystemClass::SpawnParticle` | 0x0062E380 | Spawn variant A (alloc 0x138 ParticleClass + insert into system vector) |
| `ParticleSystemClass::SpawnParticle` (var B) | 0x0062E430 | Spawn variant B |
| `ParticleSystemClass::SpawnParticleWithInsert` | 0x0062E4C0 | Spawn + reorder within recent tail (1 raw `abs(Next()) % actual_range` draw) |
| `ParticleSystemClass::Constructor` | 0x0062DC50 | Builds system; checks attached-object bridge flag (`cell+0x140 & 0x100`, layer 0xB); registers in object vector (`0x0062E0CF`) |
| `ParticleSystemClass::RemoveAllParticles` | 0x0062E650 | Reverse-walk destroy all owned particles |
| `ParticleClass::Constructor` | 0x0062B5E0 | Builds particle; lifetime RNG (Railgun: `abs(Next()) %10`; else `abs(Next()) % MaxEC`) `+ MaxEC` |
| `ParticleClass::AI_Dispatch` | 0x0062CE40 | Per-particle AI head — switches on particle BehavesLike (different enum), then decrements particle lifetime |
| `ParticleClass::AI_Gas` | 0x0062BD50 | Gas particle AI (bridge/height interaction; gas damage tick) |
| `ParticleClass::AI_Smoke` | 0x0062C540 | Smoke particle AI |
| `ParticleClass::AI_Fire` | 0x0062CB10 | Fire particle AI — 1 raw RNG jitter `Next()%10 - 5` (`-5..=4`) |
| `ParticleClass::MovementDispatch` | 0x0062D5E0 | Movement by BehavesLike — wind drift (Gas/Smoke), velocity/deacc; reads wind tables |
| `ParticleClass::Draw_It` | 0x0062CEC0 | Particle render: Spark/Railgun = single colored pixel to surface; Gas/Smoke/Fire = SHP via CC_Draw_Shape; all return layer 3 |
| `ParticleSystemTypeClass::ReadINI` | 0x006442D0 | Parses `[ParticleSystems]` keys; constructor `0x006440A0` |
| `ParticleTypeClass::ReadINI` | 0x00644F50 | Parses `[Particles]`/particle-type keys; constructor `0x00644BE0` |
| `RulesClass::ReadParticleSystems` | 0x00672A70 | Rules-load: instantiates the system type list (per stub) |
| **Scheduler / RNG infra (not owned; shared):** | | |
| `LogicClass::PerTickUpdate` | 0x0055AFB0 | The spine; Rung T loop dispatches PSC AI |
| `ObjectClass::AI` (base slot 23) | 0x005F3E70 | Rung T base `vt+0x5c` body; PSC overrides it |
| `ObjectClass::Reveal` | (base) | Construction-time scheduler join (PSC/Particle/Anim/VoxelAnim ctors call it) |
| `Random__Next` | 0x0065C780 | Raw RNG draw used by ALL particle RNG sites (NOT `RandomRanged 0x0065C7E0`) |
| `LaserDraw/segment timer list` | DAT_00ac167c / count DAT_00ac1688 | Rung N (#14) list the Spark draw path fills |

**Globals / fields:**
- Global active object vector `DAT_00a8e96c` / count `DAT_00a8e978` — PSC registers here (type 0x12).
- LogicClass live vector `0x0087f778` (items `+0x04`, count `+0x10`) — the AI scheduler PSC joins.
- Limbo vector `DAT_00b0f698` — dead systems park here until map clear (no reuse).
- Gas-pool singleton system pointer `DAT_00a8ed78` — destroyed at map clear.
- Global ParticleTypeClass array `DAT_00a83d9c` — `HoldsWhat` indexes into it.
- System BehavesLike strings `0x00836ee0`; particle BehavesLike strings `0x008370bc`.
- Wind tables `0x00836664`/`0x00836684` (gas), `0x008366a4`/`0x008366c4` (smoke).
- Spark-draw gate `g_ExtraAnimationsEnabled` `0x00A8EB78` (Smoke/Spark skip draw when 0 / fast-forward).

---

## Tick / render position

**Plug point: the per-tick spine, Rung T (#20) — the universal object-AI fan-out.**

- **Per-tick AI:** `LogicClass::PerTickUpdate @ 0x0055AFB0` Rung T walks the LogicClass live vector
  (`this=0x0087f778`) FORWARD, **count re-read each iteration**, dispatching `vt+0x5c` (slot 23) on
  every live object. For a particle SYSTEM this resolves to `ParticleSystemClass::AI @ 0x0062FD60`.
  Gate = vector count > 0 only; NOT mode-gated. (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung T
  row names "particles" among slot-23 subclasses; `_spine-rung-20.md`.)
- **Particles are a SEPARATE frame-domain.** Individual `ParticleClass` instances are NOT in the
  object vector. Each tick, the parent system's AI ticks its owned particles via
  `ParticleClass::AI_Dispatch @ 0x0062CE40`. **Order within the AI call (lockstep-relevant):** both
  dispatchers run behavior FIRST, THEN decrement lifetime — `ParticleSystemClass::AI` dispatches
  by type then `Lifetime--`/mark-for-deletion; `ParticleClass::AI_Dispatch` dispatches then
  particle-`lifetime--`. (`PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED` Ordering Notes;
  `TICK_ANIMATION_VISIBLE_LEFTOVERS` §4.)
- **Construction-time scheduler join:** `ParticleSystemClass::Constructor` calls `ObjectClass::Reveal`
  (the same machinery AnimClass/VoxelAnimClass/Particle ctors use), appending to the live vector —
  so a system spawned during an earlier object's Rung-T AI this tick can be same-pass eligible
  (count re-read). (`ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN` §3.4.)
- **Render:** particles draw in the tactical object render pass (`Tactical_ObjectRenderingLoop`
  under `TacticalClass_Draw @ 0x006D3D10`), all returning **layer 3** (top). Spark/Railgun =
  single colored pixels written directly to the surface (color interpolated from ColorList, Z/alpha
  checked); Gas/Smoke/Fire = SHP sprites via `CC_Draw_Shape` with translucency-driven draw flags.
  **Smoke (type 1) and Spark (type 3) skip rendering** when `g_ExtraAnimationsEnabled` (0x00A8EB78)
  == 0 / fast-forward — an optimization; Gas/Fire/Railgun always draw. Fog-of-war shroud cull
  (`SpecialFlags & 0x1000`) is TS-legacy, inactive in stock YR.
- **RNG (lockstep contract):** ALL particle RNG sites call **raw `Random__Next() @ 0x0065C780`**
  then apply `%`/`IDIV`/sign-normalize — they do **NOT** use the mask-and-reject `RandomRanged`
  (`0x0065C7E0`). These bind to **Scen->Random** (the synchronized lockstep stream `Scen+0x218`),
  NOT g_MainRng — explicitly corrected in `reference_rng_instance_routing_truth` (the stale
  `RNG_SYSTEM`/`PER_FRAME_RNG_CONSUMPTION_ORDER` docs wrongly listed particles under g_MainRng) and
  re-confirmed via `TechnoClass::AI_Update`'s damage-particle (Spark) spawn (`0x6FAE24`/`0x6FAEB3`)
  drawing `[0x00A8B230]+0x218`. Because particles tick inside the lockstep object-AI subtree,
  reordering particle AI or substituting `RandomRanged` for raw-modulo shifts every later RNG result
  → desync.

---

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence. (Evidence = cited verified docs; not re-run live.)

1. **abstract-object** (AbstractClass / ObjectClass) — lifecycle substrate, STRONGEST structural edge
   - via: both PSC and ParticleClass ARE `ObjectClass` subclasses. `ObjectClass::Reveal`
     (construction-time scheduler join), `ObjectClass::Load/AbstractClass::Load` (swizzle register),
     slot-23 (`vt+0x5c`) dispatch contract, the active-object-vector membership (type 0x12), the
     IPersistStream slots living in the AbstractClass-rooted primary vtable.
   - evidence: `OBJECTCLASS_GHIDRA_REPORT.md` §1 (ParticleClass/ParticleSystemClass listed as direct
     ObjectClass descendants); `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §10.4 (active-vector lifecycle),
     §11.6 (Save/Load via AbstractClass/ObjectClass).

2. **random-scenario** (RandomClass + ScenarioClass) — LOCKSTEP-CRITICAL
   - via: particle lifetime RNG in `ParticleClass::Constructor` (`abs(Next())%10` Railgun / `%MaxEC`
     else, `+MaxEC`); Smoke system periodic-spawn offsets (2 raw `Next()% (SpawnRadius+1)` in
     `AI_Smoke`); Fire particle jitter (`Next()%10 - 5` in `AI_Fire`); insertion shuffle
     (`abs(Next()) % actual_range` in `SpawnParticleWithInsert`). ALL via raw `Random__Next 0x0065C780`
     bound to **Scen->Random**.
   - evidence: `PARTICLE_RNG_CLASSIFICATION_GHIDRA_REPORT.md` §3.1–§3.5 (per-site assembly addresses);
     `reference_rng_instance_routing_truth` + `TECHNOCLASS_AI_UPDATE_BODY_GHIDRA_REPORT.md` §4 (Scen
     binding). **Must use raw modulo, not RandomRanged** (different draw count → desync).

3. **damage-helpers** (warhead/armor kernel + ReceiveDamage)
   - via: **gas-cloud damage** — the gas particle AI decrements a damage counter (`+0x12A`, reset
     from `MaxDC`); at zero it deals `Damage` with the particle's `Warhead` to all objects in the
     cell via `object.ReceiveDamage(damage, warhead, house)` (with `AdjustForZ`). This is real
     gameplay-affecting damage from a "cosmetic" system (psychic/poison clouds).
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §3 (damage tick: "deals damage to all objects
     in the cell using the particle's Warhead and Damage values", lines ~965–974).

4. **cell-map** (CellClass / MapClass)
   - via: particle/system cell placement, listener/source position; the constructor bridge check
     (`cell+0x140 & 0x100` bridge flag, layer 0xB) that offsets the system; gas/smoke ground-height
     and bridge-interaction logic (`AI_Gas` checks cell flags 0x100 at `+0x140`, compares heights);
     refinery-smoke anchoring to the building cell.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §9.12 (ctor bridge check), §3 / Open-Q2 (gas
     ground collision/bridge), §5.1 (refinery smoke caller `UnitClass::AI 0x007360c0`).

5. **rules-class** (RulesClass) + **ini-parsing** (CCINIClass accessors)
   - via: `ParticleSystemTypeClass::ReadINI 0x006442D0` and `ParticleTypeClass::ReadINI 0x00644F50`
     parse all `[ParticleSystems]`/`[Particles]` keys; `RulesClass::ReadParticleSystems 0x00672A70`
     drives the type-list load; `[General]` keys (BarrelParticle, DefaultRepairParticleSystem,
     WindDirection) and TechnoType/WeaponType keys (DamageParticleSystems, AttachedParticleSystem…)
     name the systems.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §4 (full INI key tables), §10.5 (13 systems /
     22 types enumerated), Sources (ReadINI addresses). All particle parameters come from INI at load.

6. **lookup-tables** (static read-only tables) — weak/transitive
   - via: the wind-drift DX/DY tables (`0x00836664`/`0x00836684` gas, `0x008366a4`/`0x008366c4`
     smoke), and the two BehavesLike string tables (`0x00836ee0`/`0x008370bc`) consumed at ReadINI.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §8.1 / §10.14 (table values read), §Sources
     (string tables). Static-table reads, not a live sim dependency.

7. **frontier-render-tactical** + **frontier-blitter** (render back-end) [FRONTIER]
   - via: `ParticleClass::Draw_It @ 0x0062CEC0` — Spark/Railgun write single pixels to the surface
     (blitter), Gas/Smoke/Fire submit SHP draws via `CC_Draw_Shape`; the tactical object render loop
     walks them at layer 3. Render-membership relationship, not a sim dependency.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.3 (render); `PIXEL_FX_SPARKLES` §8
     (Spark draw gated by `g_ExtraAnimationsEnabled`).

8. **frontier-render-layer** (LaserDraw / draw-segment timer lists) [FRONTIER] — weak
   - via: the Spark/laser draw path appends entries to the Rung-N (#14) draw-segment timer list
     (`DAT_00ac167c`/count `DAT_00ac1688`) which `0x005FF390` ages/purges each tick.
   - evidence: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung N row ("list filled by particle spark +
     laser/lightning draw path"). Membership/feed relationship.

9. **frontier-saveload** (SwizzleManager / IPersistStream) [FRONTIER]
   - via: PSC/Particle `Save`/`Load` (primary-vtable slots +0x14/+0x18) serialize the system+particle
     state; Load registers the swizzle ID via `FUN_006CF2C0` and re-installs vtables.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §11.6 (IPersistStream slot table + Load flow).

---

## Used-by (incoming edges)

Other services that create / drive / consume this one. Callers of
`ParticleSystemClass::Constructor @ 0x0062DC50` (§5.1):

1. **techno-foot** (TechnoClass + FootClass) — the principal producer
   - via: `TechnoClass::AI_Update @ 0x006F9E50` spawns DamageParticleSystems (damage smoke/spark);
     `TechnoClass::Fire_At @ 0x006FDD50` spawns weapon `AttachedParticleSystem` (railgun/fire stream
     following the bullet, `UseFireParticles`/`UseSparkParticles`); `TechnoClass::ReceiveDamage
     @ 0x00701900` spawns DestroyParticleSystems on death; `UnitClass::AI @ 0x007360C0` spawns
     `RefinerySmokeParticleSystem`.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1 (caller table), §8.6
     (AI_Update/ReceiveDamage smoke), §10.6 (weapon attachment); `TECHNOCLASS_AI_UPDATE_BODY` §4.

2. **logicclass** (LogicClass — the scheduler)
   - via: Rung T (`0x0055AFB0`) dispatches `ParticleSystemClass::AI` each tick for every live system
     via `vt+0x5c`; construction-time Reveal joins the live vector; AI unregisters into limbo.
   - evidence: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung T; `_spine-rung-20.md`;
     `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §10.4 (active→limbo lifecycle).

3. **damage-helpers** / area-damage (bidirectional — also a depends-on)
   - via: `Apply_area_damage @ 0x00489280` spawns a **gas cloud** particle system when an area-damage
     warhead lands (poison/psychic clouds); the gas particles then themselves call ReceiveDamage.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1 (`Apply_area_damage` caller).

4. **factory-house**-adjacent BuildingClass — gap generator / refinery
   - via: `BuildingClass::UpdateGapGenerator_Tick @ 0x00454DB0` spawns gap-generator smoke;
     refinery smoke (above) is building-driven.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1.

5. **frontier-bullet** (BulletClass) [FRONTIER]
   - via: weapon `AttachedParticleSystem` systems follow the projectile (railgun trails, fire
     streams) — the system is attached to the flying bullet at Fire.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §10.6 (`UseFireParticles`/`IsRailgun` bullet
     attachment); cross-ref `frontier-bullet.md` Used-by #5 (lists frontier-particle, weak).

6. **frontier-voxelanim** (VoxelAnimClass) [FRONTIER]
   - via: `VoxelAnimClass::Constructor @ 0x007493B0` spawns particle effects on voxel animations
     (debris smoke/sparks).
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1.

7. **frontier-trigger** (map triggers) [FRONTIER] — AI-adjacent
   - via: `TriggerAction::Execute @ 0x006DD8B0` spawns map-trigger particles.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1.

8. **frontier-capture** (CaptureManagerClass) [FRONTIER]
   - via: `CaptureManagerClass::Update @ 0x00471A50` spawns mind-control beam particles; warp/chrono
     effects via `WarpAttachClass::UpdateAttack @ 0x00629FD0`.
   - evidence: `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §5.1.

9. **frontier-saveload** (bidirectional) [FRONTIER]
   - via: the whole-game serializer walks PSC/Particle Save/Load as part of the active+limbo vectors.
   - evidence: §11.6.

---

## Active in YR / Tiberian Sun legacy

**Active in YR: YES — particle systems run every match.** Stock-live examples (from the 13 systems /
22 types enumerated in §10.5): building **damage smoke** (`BigGreySmokeSys` etc., Spawns=yes), the
**Spark** damage system spawned by `TechnoClass::AI_Update` on damaged technos, **refinery smoke**,
**gap-generator smoke**, **gas/poison clouds** from area-damage warheads (real DoT via the gas
ReceiveDamage tick), **fire streams** (`FireStreamSys`, flamethrowers), and **railgun trails**
(`AttachedParticleSystem` on railgun weapons). The constructor has stock-YR xrefs from weapon, area
damage, damage smoke, gap generator, and refinery-smoke paths.

**Rust port status note:** the current Rust port partially models particles (`src/sim/particles/{fire,
smoke,gas,spawn}.rs` exist) but **Spark and Railgun are no-op**, and the existing RNG calls use
`RandomRanged` where gamemd uses raw modulo (a `RED` lockstep mismatch flagged across the RNG report).
This is a port gap, not a binary edge.

**TS-legacy / dead-in-YR (do NOT implement as active):**
- **Fog-of-war shroud cull in `Draw_It`** (`SpecialFlags & 0x1000`) — TS legacy; `FogOfWar` defaults
  OFF in stock YR, so shrouded-particle culling never fires (matches project FogOfWar rule).
- **`NaturalParticleSystem`** (TechnoType key) — parsed in ReadINI but NO standard YR content sets
  it; ambient-particle feature is effectively dead (§Open-Q5, conditional).
- Type-class fields tied to those features remain parsed-but-inert.

---

## Open / unverified edges

- **NO live Ghidra re-verification this session** — no instance reachable (UDS empty; TCP 8089
  refused; no Java/Ghidra process). Every address is carried from `[ghidra/verified]` docs that cite
  exact decompile/assembly evidence; re-run `ParticleSystemClass::AI @ 0x0062FD60`, the
  random-scenario (Scen->Random) RNG sites, and the damage-helpers gas edge live before
  implementation. This is the top caveat.
- **RNG instance ECX binding at the particle callsites** — the routing-truth memory + AI_Update
  re-confirm Scen->Random for the damage-particle spawn, and the RNG-classification report treats the
  in-AI particle draws as the synchronized stream, but the exact `MOV/LEA ECX` before each
  `Random__Next 0x0065C780` inside `ParticleClass::Constructor` / `AI_Smoke` / `AI_Fire` /
  `SpawnParticleWithInsert` was not individually re-read this session. Confirm per-callsite ECX
  (`[0x00A8B230]+0x218` = Scen) before pinning the lockstep contract.
- **Gas system periodic self-spawn** — `ParticleSystemClass::AI_Gas` shows NO periodic self-spawn in
  the verified slice, yet the Rust `gas.rs` has a smoke-style spawn offset; classify YELLOW/RED until
  the gas creation path is traced (`PARTICLE_RNG_CLASSIFICATION` §11).
- **`FUN_00630b90`/`FUN_00630ea0`** (in the PSC range) and **color-interpolation `FUN_00661020`**
  (spark/railgun StartColor1→2) were not decompiled in the source reports — LOW priority, but
  unverified.
- **OneFrameLight vs AI_Spark light creation** (`FUN_0062e280`, vt+0x114) — redundancy between the
  two spark-light paths is an open question (§Open-Q6).
- **`RulesClass::ReadParticleSystems @ 0x00672A70`** is carried from the seed stub and not
  independently re-verified against the report family; confirm before relying on it.
