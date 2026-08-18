# Core Service Profile — frontier-bullet

**Slug:** `frontier-bullet`
**Service:** BulletClass — in-flight projectiles (homing/ballistic/inviso flight, arming/proximity, detonation, cluster/airburst/shrapnel spawn)
**Status:** FRONTIER (promoted from catalog stub D2 in `_frontier.md`). No prior substrate study; this profile consolidates the scattered `BULLET*` / `AAHEATSEEKER2_*` Ghidra reports into the core-services map.
**Primary docs (existing, Ghidra-verified):**
- `docs/research/BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` (end-to-end pipeline: ReadINI → Fire → AI → BulletDetonation; addresses + offsets bit-checked)
- `docs/research/BULLETCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md` (scheduler membership, same-pass first-AI timing, detonation/removal order)
- `docs/research/AAHEATSEEKER2_*` family (speed/launch-vector/acceleration, retarget AI, first-tick damage latency, detonation parameters)
- `docs/research/AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`, `INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md`, `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`
- Layout/struct: `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` (instance 0x00–0x15F, size 0x160), `BULLETTYPECLASS_GHIDRA_REPORT.md` (type struct, size 0x2F8)

**This profile:** edge/graph extract for the core-services map. Long content lives in the docs above.

**Evidence base / re-verification note:** No Ghidra instance was reachable this session
(`list_instances` → none; `connect_instance gamemd` → refused), so the addresses below were
**NOT re-run live this session**. They are carried from the `[ghidra/verified]` reports listed
above, which cite exact `decompile_function` / `disassemble_function` / assembly-line evidence
inline. Where a stub claim is corrected, the corrected value is the one those reports verify.
Treat every address as **VERIFIED-VIA-CITED-DOC**, not VERIFIED-LIVE-THIS-SESSION. Re-run the
representative `BulletClass::AI @ 0x004666E0` and the cross-service edges against the binary
before any implementation.

---

## Stub corrections (representative address + plug point)

The seed stub (D2 in `_frontier.md`) had two issues; both are corrected here against the
verified reports:

1. **Representative function — CORRECTED.** Stub named `BulletClass__HomingTrack @ 0x005B20F0`
   as the representative fn. That address is the **homing-track math helper**, not the
   per-frame AI entry. The real per-tick driver — the `ObjectClass`-derived slot-23 (`vt+0x5c`)
   AI head that the spine dispatches — is **`BulletClass::AI @ 0x004666E0`**. Evidence:
   `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §2 (lifecycle box: "BulletClass::AI
   0x004666E0 — per-tick update … ROT>0: homing (HomingTrack @ 0x005B20F0)"), and
   `BULLETCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md` §3 (assembly
   `0x00467FA2 CALL 0x00468D80` then `0x00467FB4 CALL [vtable+0xF8]` inside `0x004666E0`).
   `0x005B20F0` (HomingTrack) is retained below as a **key function**, just not the rep entry.
   The stub's `BulletClassBulletDetonationImpactDamage @ 0x00468D80` and alloc
   `BulletClassAllocate @ 0x0046B050` are both confirmed correct.

2. **Plug point rung label — CORRECTED.** Stub said "PerTickUpdate object pass (rung N)". The
   verified spine (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`, 28 rungs A–AB) places the universal
   per-object AI fan-out at **Rung T (#20)**, driver `ObjectClass::AI @ 0x005F3E70` dispatched
   via `vt+0x5c` over the LogicClass live vector (`this=0x0087f778`, items `+0x04`, count
   `+0x10`). `_spine-rung-20.md` explicitly names "bullets, voxel anims, particle systems" among
   the slot-23 subclasses. ("Rung N" was the older provisional letter; the verified ladder uses
   T.) BulletClass is one of those polymorphic subclasses: its slot-23 override is
   `BulletClass::AI @ 0x004666E0`.

---

## Purpose

The **projectile-flight service** — the live `BulletClass` object that exists between a weapon
firing and its warhead detonating. It owns: per-tick flight integration (three trajectory
families — arcing/ballistic, straight/level, homing-missile ROT>0), the inviso instant-impact
path, the proximity/arming fuse, detonation triggering, and the detonation fan-out (pre-impact
damage, Cluster loop, Airburst sub-bullet spawn, shrapnel). It does NOT own the armor/warhead
damage math (that's `damage-helpers`), target acquisition / weapon selection (that's the firing
techno + `target-scoring`), or the explosion/trailer sprite animation (that's `frontier-anim`)
— it *drives* those at its boundaries.

Conceptual core that is purely this service: **`BulletClass::AI` (per-tick flight + detonation
trigger)** + **`BulletClass::Fire` (launch: reveal, velocity, arm, inviso snap)** +
**`BulletClass::BulletDetonation` (pre-impact damage + Cluster/Airburst dispatch)** +
**`BulletTypeClass::ReadINI` (the projectile type-data the flight reads)**.

---

## Owns

- **The live bullet object & its flight state.** Instance layout `BulletClass` size **0x160**
  (`BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`): Location `+0x9C/0xA0/0xA4`, Velocity `+0xE8`
  (3 doubles / 24 bytes), Target `+0x10C`, Damage `+0x6C`, Warhead ptr, TargetSpeed `+0x110`
  (= WeaponType.Speed, set at Init — there is **no** BulletType speed field), FirersPalette
  copy `+0x114`, SourceCoord `+0x134/0x138/0x13C`, TargetCoord `+0x140/0x144/0x148`, LastCell
  `+0x14C`, the embedded `ProximityDetector` (`Prox`), and the nuke delayed-detonation listener
  state (`+0x154` anim ptr / `+0x158` waiting flag).
- **The projectile type data** `BulletTypeClass` size **0x2F8** (`BULLETTYPECLASS_GHIDRA_REPORT.md`):
  trajectory flags (Arcing `+0x29B`, Level `+0x29D`, Inviso `+0x29E`, Vertical `+0x2C0`,
  ROT `+0x2DC`), fuse flags (Ranged `+0x2A0` = the real prox gate; Proximity `+0x29F` is
  parsed-but-dead in YR), Arm `+0x2F0` (arming delay), Cluster `+0x2AC`, Airburst `+0x294`,
  AirburstWeapon `+0x2B0`, ShrapnelWeapon/Count `+0x2B4/+0x2B8`, Elasticity `+0x2C8`,
  Acceleration `+0x2D0`, Trailer `+0x2D8`, AA `+0x2A4` / AG `+0x2A5`. Non-zero constructor
  defaults to mirror: Shadow=true, AG=true, Cluster=1, Elasticity=0.75, Acceleration=3,
  SpawnDelay=3, inverted Rotates storage `+0x2A1`.
- **The proximity/arming fuse** `ProximityDetector::Set @ 0x004E1130` / `::Check @ 0x004E11F0`
  (embedded sub-object): writes ArmingDelay `+0x14` = `Arm=` (forced 0 when target is
  ground-layer), ReferenceCoord `+0x18/1C/20`, ClosestDistance watermark `+0x24`. Returns 0
  ("keep flying") until armed, then detonation.
- **The global bullet registry** (save/load list, NOT the AI scheduler):
  `g_BulletClass_Array` / `DAT_00A8ED40`. NOTE per the migration slice: this registry is the
  EntityStore-equivalent; scheduler membership is the separate LogicClass live vector
  (`0x0087f778`), joined at Fire/Reveal via `ObjectClass::Reveal`, not at construction.
- **The detonation fan-out logic** in `BulletClass::BulletDetonation @ 0x00468D80`: pre-impact
  damage gate (narrow for ground buildings — turreted only, foundation-adjusted < 42 leptons;
  wide for airborne targets < 128), the Cluster loop (`WarheadTypeClass::Detonate` up to
  `Cluster` times, scatter `RandomRanged(256,512)` per iter, bails on `IsAlive==false`), and
  Airburst's single-detonate (sub-bullets spawned inside the Warhead detonation).

It does **not** own: the armor/Verses/falloff damage number, the AoE target collection
(`Apply_area_damage`), explosion/trailer AnimClass sprites, or weapon selection — all delegated.

---

## Key functions & globals (addresses)

All VERIFIED-VIA-CITED-DOC (not re-run live this session — see evidence-base note above).

| Symbol | Address | Role |
|---|---|---|
| `BulletClass::AI` | 0x004666E0 | **Representative fn** — per-tick flight + detonation trigger; the slot-23 (`vt+0x5c`) AI head dispatched by Rung T |
| `BulletClass::Fire` | 0x00468670 | Launch: `ObjectClass::Reveal` (scheduler join) → velocity copy → SourceCoord/TargetCoord → FlakScatter+Inviso scatter → inviso instant-snap → `ProximityDetector::Set(arm)` → ROT>0 velocity-normalize → `DisplayClass::Submit_Object` |
| `BulletClass::Init` | 0x004664C0 | Writes Type/Owner/Target/Damage/Warhead/TargetSpeed(`+0x110`=WeaponType.Speed) into the allocated bullet |
| `BulletClass::Allocate` | 0x0046B050 | operator_new(0x160) + Constructor + Init (registers in global array) |
| `BulletClass::Constructor` | 0x00466380 | Registers instance in `g_BulletClass_Array` / DAT tables |
| `BulletClass::BulletDetonation` | 0x00468D80 | Pre-impact damage + Cluster loop / Airburst dispatch; calls `WarheadTypeClass::Detonate` |
| `BulletClass::HomingTrack` | 0x005B20F0 | ROT>0 homing-track math helper (turn-toward-target, speed ramp) — key fn, NOT the AI entry |
| `BulletClass::UpdateTarget` | 0x00468430 | Re-reads / validates target; pointer-expired handler `0x004684E0..0x004685C6` |
| `BulletClass::Fire` velocity normalize | (inside 0x00468670) | ROT>0 → unit-length velocity (start speed 1, ramp via Acceleration) |
| `BulletTypeClass::ReadINI` | 0x0046BEE0 | Parses every projectile key (37) from rules/art sections; caller wrapper `0x0046BE10` |
| `BulletTypeClass::Constructor` (defaults) | 0x0046BBC0 | Sets non-zero defaults; stores logic-enable byte `+0x234=1` |
| `ProximityDetector::Set` | 0x004E1130 | Writes ArmingDelay/ReferenceCoord/ClosestDistance from `Arm=` |
| `ProximityDetector::Check` | 0x004E11F0 | Per-tick fuse: returns 0/1/2 (keep-flying / detonate / …) |
| `WarheadTypeClass::Detonate` | 0x004690B0 | Top of damage chain (owned by `damage-helpers`; called from BulletDetonation) |
| `RegisterScalable` (trail throttle) | 0x0046B280 | `Scalable=yes` trail rate-limiter (only gated from `UnitClass::Fire`) |
| `TechnoClass::Fire_At` | 0x006FDD50 | Caller — allocates+launches the bullet (owned by `techno-foot`) |
| **Scheduler infra (not owned; shared with all live objects):** | | |
| `LogicClass::PerTickUpdate` | 0x0055AFB0 | The spine; Rung T loop `0055b5fb..0055b619` dispatches bullet AI |
| `ObjectClass::AI` (base slot 23) | 0x005F3E70 | Base `vt+0x5c` body; BulletClass overrides it |
| `ObjectClass::Reveal` | 0x005F4EC0 | Fire-time scheduler join (appends to live vector `0x0087f778`) |
| `ObjectClass::UnInit` (`vtable+0xF8`) | 0x005F65F0 | Bullet teardown after detonation (live-vector compaction `FUN_0055BAE0`) |

**Globals / fields:**
- `g_BulletClass_Array` / `DAT_00A8ED40` — global bullet registry (save/load; not the scheduler).
- LogicClass live vector `0x0087f778` (items `+0x04`, count `+0x10`) — the AI scheduler bullets join at Fire/Reveal.
- `g_CurrentFrameCounter` `0x00A8ED84` — trailer-spawn cadence + arming-frame baseline.
- `RulesClass` ballistic globals (read by flight/launch): Gravity `+0x16B8`, BallisticScatter
  `+0x1734`, HomingScatter `+0x1730`, MissileSpeedVar `+0x0590`, MissileROTVar `+0x0598`,
  **MissileSafetyAltitude `+0x05A0`** (the lost-target detonation threshold — prior docs
  mislabeled this `FlightLevel`; FlightLevel `+0x07B4` is aircraft cruise, unrelated).

---

## Tick / render position

**Plug point: the per-tick spine, Rung T (#20) — the universal object-AI fan-out.**

- **Per-tick AI:** `LogicClass::PerTickUpdate @ 0x0055AFB0` Rung T (`0055b5fb..0055b619`)
  walks the LogicClass live vector (`this=0x0087f778`) forward, **count re-read each iteration**,
  and dispatches `vt+0x5c` (slot 23) on every live object. For a bullet that resolves to
  `BulletClass::AI @ 0x004666E0`. Gate = vector count > 0 only; **not mode-gated** (unlike the
  AnimClass Rung U that follows). Cross-ref `_spine-rung-20.md` (lists bullets among slot-23
  subclasses) and `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`.
- **Same-pass first AI (lockstep-relevant):** a bullet fired during another object's AI earlier
  in the same Rung-T pass tail-appends via `BulletClass::Fire → ObjectClass::Reveal`
  (`FUN_0055BAA0` test-and-set `+0x98`); because the loop re-reads count each iter, it can
  receive its first `BulletClass::AI` **the same tick** if the cursor hasn't passed the tail.
  This is NOT a next-tick queue — a next-tick queue would add one frame of projectile/damage
  latency to common missile shots. (`BULLETCLASS_AI_FIRST_SAFE_MIGRATION_SLICE` §2.)
- **Detonation/removal order (within the AI call):** detonation side effects run **before**
  self-removal — `0x00467FA2 CALL 0x00468D80` (BulletDetonation) precedes
  `0x00467FB4 CALL [vtable+0xF8]` (UnInit); UnInit then compacts the live vector
  (`FUN_0055BAE0`, shift-left), so the shifted immediate successor is skipped this pass. Detonation
  children (Airburst sub-bullets / explosion anims) are appended before parent UnInit and can be
  same-pass eligible. (`BULLETCLASS_AI_FIRST_SAFE_MIGRATION_SLICE` §3.)
- **Launch ordering vs Fire_At:** `TechnoClass::Fire_At @ 0x006FDD50` (techno-foot,
  turrets+combat stage) allocates → Init → calls `BulletClass::Fire` (vtable+0x1F0). For Inviso
  bullets the impact point and zeroed velocity are set at Fire; detonation lands on the next AI
  tick via the proximity check (already at reference coord → Check returns 1).
- **Render:** the bullet sprite/voxel draws in the tactical object render loop
  (`Tactical_ObjectRenderingLoop`, the z-sorted pass under `TacticalClass_Draw @ 0x006D3D10`);
  Inviso bullets bind no SHP (skip image load in ReadINI) and never draw. Trailer/explosion
  visuals are AnimClass objects (frontier-anim). This is the render-pass side, not the tick.
- **RNG (lockstep contract):** bullet draws are part of Rung T's per-callsite RNG subtree —
  flight scatter/inaccuracy and Cluster scatter `RandomRanged(256,512)` bind to **Scen->Random**
  (lockstep stream `Scen+0x218`); detonation→`Apply_area_damage` debris/destruction rolls
  `RandomRanged(0,99)` also Scen->Random. Reordering bullet AI within the pass shifts every later
  RNG result → desync.

---

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence. (Evidence = cited verified docs; not re-run live.)

1. **damage-helpers** (warhead/armor kernel + AoE distributor) — STRONGEST edge
   - via: `BulletClass::BulletDetonation @ 0x00468D80` → `WarheadTypeClass::Detonate @ 0x004690B0`
     → `Apply_area_damage @ 0x00489280` → per-target `ReceiveDamage` → kernel
     `ApplyWarheadDamage @ 0x00489180`. Also direct pre-impact `ReceiveDamage` (vtable+0xA4)
     to turreted buildings (<42 leptons) / airborne targets (<128).
   - evidence: `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §9.1 (BulletDetonation flow);
     `damage-helpers.md` lists `WarheadTypeClass::Detonate 0x004690b0` as "top of the damage
     chain." The bullet computes WHERE/WHAT detonates; damage-helpers computes HOW MUCH HP.

2. **random-scenario** (RandomClass + ScenarioClass)
   - via: Cluster scatter `Random::RandomRanged(0x100,0x200)` per iteration in BulletDetonation;
     FlakScatter+Inviso launch scatter (`RandomRanged(0, BallisticScatter*2)` + random facing) in
     `BulletClass::Fire`; homing inaccuracy (HomingScatter). All bind **Scen->Random** (lockstep).
   - evidence: `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §5.2 (Fire scatter), §9.1.1
     (`Random::RandomRanged(256,512)`); spine Rung-T RNG note. Scatter/inaccuracy is RNG-driven.

3. **target-scoring** (target-scoring helpers)
   - via: homing retarget — `BulletClass::UpdateTarget @ 0x00468430` re-validates the target each
     tick; `BulletClass::HomingTrack @ 0x005B20F0` steers toward the current target coord; lost-
     target / sentinel-coord guards consult `MissileSafetyAltitude` (Rules+0x5A0) to decide
     detonate-vs-climb.
   - evidence: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md` (UpdateTarget +
     pointer-expired handler `0x004684E0..0x004685C6`); `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED`
     §6/§11 (MissileSafetyAltitude). NOTE: weapon *selection* (which projectile) is upstream in
     the firing techno, not in BulletClass — this edge is the in-flight homing retarget only.

4. **cell-map** (CellClass / MapClass)
   - via: per-tick cell occupancy at the bullet's position (`LastCell +0x14C` packed X,Y, updated
     each tick), bridge-crossing / out-of-bounds forced-detonation checks, BounceCheck cliff/wall
     deflection reads, and `CellClass::GetGroundHeight` for inviso impact-Z and arcing ground
     contact. Inviso raycast `FUN_005880a0` resolves the impact cell.
   - evidence: `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §5.4 (inviso raycast +
     GetGroundHeight), §9.2 (bounce/bridge); `BULLET_CLASS_AI_GHIDRA_REPORT.md` (cell collision).

5. **rules-class** (RulesClass)
   - via: ballistic globals read by flight/launch — Gravity `+0x16B8` (arcing VelZ decrement),
     BallisticScatter `+0x1734`, HomingScatter `+0x1730`, MissileSpeedVar/ROTVar `+0x0590/+0x0598`,
     MissileSafetyAltitude `+0x05A0`; plus the BulletType data itself originates from rules/art INI.
   - evidence: `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §6 (Rules ballistic keys with
     offsets, verified from `RulesClass::ReadCombatDamage`/`ReadGeneral`). Trajectory tuning is
     RulesClass-global.

6. **ini-parsing** (CCINIClass / INIClass accessors)
   - via: `BulletTypeClass::ReadINI @ 0x0046BEE0` (ReadInt/ReadBool/ReadDouble/ReadColor/ReadString
     for all 37 projectile keys); `BulletTypeClass::Constructor @ 0x0046BBC0` defaults.
   - evidence: `BULLETTYPECLASS_GHIDRA_REPORT.md` (authoritative ReadINI key/offset/default table);
     consolidated §3–§4. All projectile parameters come from INI parse at load.

7. **abstract-object** (AbstractClass / ObjectClass) — lifecycle substrate
   - via: BulletClass IS an ObjectClass subclass — `ObjectClass::Reveal @ 0x005F4EC0` (Fire-time
     scheduler join), `ObjectClass::UnInit @ 0x005F65F0` (`vtable+0xF8` teardown), the `+0x98`
     membership bit, slot-23 (`vt+0x5c`) dispatch contract, Location/Coords accessors.
   - evidence: `_spine-rung-20.md` (Reveal/UnInit/membership-bit machinery is base ObjectClass);
     `BULLETCLASS_AI_FIRST_SAFE_MIGRATION_SLICE` §2 (Fire calls Reveal; UnInit at vtable+0xF8).

8. **frontier-anim** (AnimClass) — sibling object service [FRONTIER]
   - via: trailer spawns (`BulletType.Trailer +0x2D8` → AnimClass on a SpawnDelay/RandomRate
     cadence each tick), bounce/expire anims, and the explosion AnimClass spawned inside
     `WarheadTypeClass::Detonate` at impact; the **NUKE delayed-detonation** path keeps the bullet
     alive listening on an AnimClass pointer (`+0x154/+0x158`).
   - evidence: consolidated §3.4 (trailer cadence), §9.1 (warhead spawns anim);
     `BULLETCLASS_DELAYED_DETONATION_ANIM_LISTENER_PATH_RESWARM_20260528.md` (nuke listener).

9. **lookup-tables** (static read-only tables) — weak/transitive
   - via: trig/distance helpers for scatter facing (`cos/sin` of a random facing in Fire scatter),
     and the leptons-per-cell constant (256) shared with damage-helpers' CellSpread walk reached
     via the warhead detonation.
   - evidence: consolidated §5.2 (cos/sin facing math). Weak edge (most math is inline doubles).

(Optional/weak) **frontier-render-tactical** — the bullet's draw (sprite SHP / voxel) happens in
the tactical object render loop; non-Inviso bullets submit themselves via
`DisplayClass::Submit_Object` in Fire and are walked by the z-pass. Listed as a render-membership
relationship, not a sim dependency.

---

## Used-by (incoming edges)

Other services that drive / consume this one:

1. **techno-foot** (TechnoClass + FootClass) — the producer
   - via: `TechnoClass::Fire_At @ 0x006FDD50` allocates (`BulletClass::Allocate 0x0046B050`),
     initializes (`Init 0x004664C0` — writes TargetSpeed=WeaponType.Speed), and launches
     (`BulletClass::Fire 0x00468670` via vtable+0x1F0) every projectile-bearing weapon discharge.
     `UnitClass::Fire @ 0x00741340` additionally gates `Scalable=yes` trail throttling
     (`RegisterScalable 0x0046B280`).
   - evidence: `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md` (Fire_At
     allocation site `0x006FE55D`); consolidated §2, §7B. Every weapon with `Projectile=` routes here.

2. **logicclass** (LogicClass — the scheduler)
   - via: Rung T (`0x0055AFB0` loop `0055b5fb..0055b619`) dispatches `BulletClass::AI` each tick
     for every live bullet via `vt+0x5c`; Fire/Reveal joins the live vector; UnInit compacts it.
   - evidence: `_spine-rung-20.md` (bullets among slot-23 subclasses; count-re-read same-pass
     append). This is the structural per-tick driver edge.

3. **damage-helpers** (bidirectional — also a depends-on)
   - via: BulletDetonation is the principal *projectile* caller of the warhead/AoE damage chain;
     `damage-helpers.md` "Used-by/Tick position" frames the attack→detonate→ReceiveDamage chain as
     reached through projectile combat.
   - evidence: `damage-helpers.md` §Tick (Fire_At → detonate → Apply_area_damage), §Used-by.

4. **frontier-anim** (bidirectional — sibling) [FRONTIER]
   - via: explosion/trailer/nuke anims are *spawned by* bullet detonation/flight; the nuke listener
     path is the bullet *consuming* an anim's lifetime callback.
   - evidence: consolidated §9.1; nuke listener report. Listed both directions (spawns + listens).

5. **frontier-particle** (sibling) [FRONTIER] — weak
   - via: `SpawnsParticle`/`NumParticles` on the projectile's art (railgun/trail particle emitters)
     attach to flying bullets.
   - evidence: consolidated §3.3 (SpawnsParticle key, though attributed to the art/AnimType layer).
     Weak/conditional edge.

---

## Active in YR / Tiberian Sun legacy

**Active in YR: YES — every weapon discharge with a `Projectile=` entry runs this pipeline.**
Standard live examples: `[AAHeatSeeker2] ROT=60` (homing, GGI/most missiles), `[Cannon]`/`[120mm]`
(ballistic tank shells), the `Invisible*` family (instant-hit small arms / beams via the inviso
path), `[V3AirburstP]` (airburst), Flak Cannon's `[FlakProj]` (Inviso+FlakScatter). The flight
families (arcing / straight / homing / inviso), the proximity/arming fuse, Cluster, and Airburst
are all live.

**TS-legacy / dead-in-YR flags (parsed but inert — do NOT implement as active):**
- `Proximity=` (BulletType `+0x29F`) — parsed and stored but **never read** at runtime; the real
  prox-fuse gate is `Ranged=` (`+0x2A0`). Exhaustive byte-pattern search found zero AI/Fire/
  Detonation reads of `+0x29F` (consolidated §7).
- `Floater=` (`+0x295`), `Dropping=` (`+0x29C`) — TS-era; no standard YR unit sets them.
- `Scalable=` (`+0x2EC`) — LIVE but only from `UnitClass::Fire` (vehicle-fired); infantry/building
  fire paths don't gate on it (consolidated §7B).
- `TrailerSeperation=` (`+0x30C`) — written by ReadINI but no AI read of `+0x30C`; dead in YR.
- The `RulesClass+0x5A0` lost-target threshold is **MissileSafetyAltitude**, not `FlightLevel`
  (`+0x07B4`); prior bullet docs that say "FlightLevel" are stale on this point (consolidated §6/§11).

---

## Open / unverified edges

- **NO live Ghidra re-verification this session** — no instance reachable. Every address is
  carried from `[ghidra/verified]` docs that cite exact decompile/assembly evidence; re-run
  `BulletClass::AI @ 0x004666E0` and the damage-helpers / target-scoring edges live before
  implementation. This is the top caveat.
- **HomingTrack `0x005B20F0` exact role** — confirmed as the ROT>0 homing-track helper by the
  consolidated lifecycle box and AAHeatSeeker2 reports, but its full math (turn-rate clamp,
  CourseLockDuration interaction, speed ramp) is decoded in the AAHeatSeeker2 family, not
  re-verified here.
- **frontier-particle edge confidence** — `SpawnsParticle`/`NumParticles` are attributed in one
  report to a phantom BulletType second-pass that was later proven to belong to AnimType
  (`BULLET_PROJECTILE_SYSTEM_CONSOLIDATED` §3.3 SUPERSEDED). Whether railgun/trail particles attach
  to the BulletClass or to its trailer AnimClass needs a clean trace before counting this edge firm.
- **Cluster vs Airburst child-object representation** — Cluster does NOT spawn child BulletClass
  objects (loops Warhead::Detonate in place); only Airburst spawns real sub-bullets (inside the
  single Warhead detonation). The exact sub-bullet append ordering relative to parent UnInit is
  verified directionally but the full child scheduler ownership is deferred
  (`BULLETCLASS_DETONATION_SAMEPASS_CHILD_SPAWN_ORDER_RESWARM_20260528.md`).
- **Rust port status (for the implementation handoff, not a binary edge):** the sim currently
  models weapons as hit-scan (instant `ReceiveDamage`); no live BulletClass-equivalent entity,
  no flight, no proximity/arm, no Cluster/Airburst (consolidated §9.2). `rocket_movement.rs` /
  `homing_movement.rs` exist but are orphaned staged passes, not driven from the live scheduler.
