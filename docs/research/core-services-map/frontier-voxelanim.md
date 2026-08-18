# frontier-voxelanim — VoxelAnimClass (voxel debris / effects)

**Service slug:** `frontier-voxelanim`
**Status:** promoted from catalog stub (`_frontier.md` §D4) to full profile.
**Active in YR:** YES — voxel debris fires during any vehicle explosion in a standard YR
skirmish (flying turrets, tires, gas tanks, crystal shards; plus meteorites/gem shards via
the optional weather/SW paths). Not TS-only.

**Authority / verification note (READ FIRST):** the Ghidra MCP instance was **not reachable
this session** (`connect_instance gamemd` → connection refused; `list_instances` → none),
so addresses below were **NOT re-verified live this session**. They are sourced from two
existing `ghidra/verified` reports that decompiled the class from the binary:
`docs/research/VOXELANIMCLASS_GHIDRA_REPORT.md` (Confidence HIGH, all core fns decompiled)
and `docs/research/BOUNCE_CLASS_GHIDRA_REPORT.md`. Per project authority order (binary →
Ghidra → docs) these are doc-sourced, one rung below a live Ghidra read. **One stub claim is
corrected here from the verified spine spec (rung N → rung T).** Anything that still needs a
live read before implementation is flagged **[NEEDS-LIVE-VERIFY]**.

---

## PURPOSE

Transient 3D voxel debris with a physics simulation. When a vehicle dies, the killing
warhead's / techno's `DebrisTypes[]` + `DebrisMaximums[]` lists are read and N
`VoxelAnimClass` instances are spawned at the death cell. Each instance embeds a
`BounceClass` physics sim (gravity, elasticity bounce, angular spin, slope reflection,
bridge/building collision). It launches upward, spins, bounces off the ground, and on
`Duration` expiry (or water landing) plays an expire animation, optionally deals
warhead-typed area damage, optionally spawns child VoxelAnims (meteor `Spawns=`), and
deletes itself. Meteor-type VoxelAnims (`IsMeteor`) can also dirty terrain into Tiberium/ore
on impact. Purely visual/effect debris — no strategic / radar significance beyond the
terrain-dirty crater path.

Class hierarchy: `VoxelAnimClass : ObjectClass` (WhatAmI = `0x29` / 41; instance size
`0x148`/328 bytes). Its type class `VoxelAnimTypeClass : ObjectTypeClass` is parsed from
`[VoxelAnims]` in rulesmd.ini (10 stock types PIECE…PEBBLE).

---

## WHAT IT OWNS (globals / structs — addresses from VOXELANIMCLASS report, not re-verified this session)

- **`VoxelAnimClass` live list** — `DynamicVectorClass<VoxelAnimClass*>` @ **`0x00887388`**
  (count `+0x08`, buffer `+0x04`, capacity `+0x0C`). Added in ctor, removed in dtor, iterated
  for save. This is the per-service live vector.
- **`VoxelAnimTypeClass` type list** — `DynamicVectorClass<VoxelAnimTypeClass*>` @
  **`0x00B0F670`** (registration in ctor) and a secondary list @ **`0x00A8EB28`** (likely the
  save/load TypeList). [NEEDS-LIVE-VERIFY which is the canonical TypeList.]
- **`BounceClass`** — 0x50 (80) byte physics struct embedded **inside** VoxelAnimClass at
  byte offset `+0xB0` (NOT a standalone allocatable entity; also embedded inside AnimClass for
  2D bouncing debris). Owns: Elasticity, Gravity (always 1.4, hardcoded
  `0x3FF66666_60000000`), AngularVelocityMagnitude, Position(float), Velocity(float),
  Orientation quaternion (`+0x30`), RotationPerTick quaternion (`+0x40`). Fully mapped in
  `BOUNCE_CLASS_GHIDRA_REPORT.md`.
- **Per-instance state** (VoxelAnimClass byte offsets): `+0x104` Type ptr, `+0x108`
  AttachedSystem (ParticleSystemClass*), `+0x10C` Owner (HouseClass*), `+0x110` marked-for-
  delete bool, `+0x114`/`+0x128` two SoundEvents (Start/Stop loop sound), `+0x140` Duration
  countdown.
- **VoxelAnimClass CLSID** `{0E272DC1-9C0F-11D1-B709-00A024DDAFD1}` @ `0x007E9650` (save/load).

---

## KEY FUNCTIONS + GLOBALS (addresses from existing verified reports; NOT re-verified live this session)

| Symbol | Address | Role |
|---|---|---|
| `VoxelAnimClass::AI` (representative fn) | **`0x00749F30`** | per-tick AI: BounceClass::Update, Duration countdown, trailer spawn, expire → anim + area damage + child spawns + delete. ~2599 bytes. Internal child-spawn call at `0x0074A2FB`. |
| `VoxelAnimClass::Constructor` (main, 4 params) | `0x007493B0` | spawns one debris; calls `BounceClass::Init` at `0x0074981F`; adds to live vector. |
| `VoxelAnimClass::Constructor` (default, serialization) | `0x007498D0` | |
| `VoxelAnimClass::Destructor` | `0x007499F0` | removes from live vector `0x00887388`. |
| `VoxelAnim::Draw` | **`0x0046B0C0`** | rasterizes the VXL at the bounced+spun position. Sole caller `0x0046824A` inside the bouncing-object draw dispatcher `FUN_00468090`. |
| `VoxelAnimClass::GetLayer` | `0x0074A960` | returns **3** (Top layer). |
| `VoxelAnimTypeClass::Constructor` | `0x0074AD80` | |
| `VoxelAnimTypeClass::ReadINI` | `0x0074B050` | parses one `[VoxelAnims]` type section. |
| `BounceClass::Init` | `0x004397E0` | seeds physics from Type fields. |
| `BounceClass::Update` | `0x00439B00` | one physics step; returns 0=airborne / 1=bounced / 2=stopped. |
| `RulesClass__ReadVoxelAnims` | `0x00672920` *(stub-claimed)* | **[NEEDS-LIVE-VERIFY]** — not cited in the VOXELANIMCLASS report; locate the rules-level `[VoxelAnims]` list-reader and confirm before relying on it. |

Primary vtable @ `0x007F6318` (64 entries). Save `0x0074AA10`, Load `0x0074A970`,
GetClassID `0x0074AAD0`.

---

## TICK / RENDER PLUG POINT (cite the verified spine)

- **Sim tick:** `VoxelAnimClass::AI` runs through the universal per-object AI fan-out via
  vtable slot `+0x5C`. Per the verified 28-rung spine
  (`docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`), that fan-out is **Rung T —
  "MAIN object vector tick (universal per-object AI fan-out)" `@ 0x005F3E70`** (ObjectClass::AI
  vt+0x5c, polymorphic). The rung-T member list in the spec **explicitly names
  "bullets/voxelanims/particles"** among its ObjectClass-derived occupants — i.e. VoxelAnim
  ticks here, alongside `frontier-bullet` and `frontier-particle`. The driver walks the
  LogicClass live vector (`0x0087f778`, base `+0x04` / count `+0x10`), FORWARD, count re-read
  each iteration; membership is established by Reveal on construction. **No game-mode gate**
  (unlike Rung U, the AnimClass-subset vector, which IS skirmish-mode-gated 0/5).

  **CORRECTION vs stub:** `_frontier.md` §D4 (and the §D1/D2/D3 anim/bullet/particle stubs)
  say "PerTickUpdate object pass (rung N)". That is **wrong** against the verified spine —
  spine rung **N** is "Laser/draw-segment timer purge `@ 0x005FF390`", a different driver.
  The universal object fan-out is rung **T**. Use **rung T** for voxelanim/bullet/particle/
  anim-list object ticking.

- **Render:** out-of-sim render pass, NOT a spine rung. `VoxelAnim::Draw @ 0x0046B0C0` is
  invoked from the bouncing-object draw dispatcher `FUN_00468090` (VXL branch when
  `TypeClass+0x236` set), which the tactical object render loop calls for **Layer 3 (Top)**
  objects. Ties to the render entry `TacticalClass_Draw @ 0x006D3D10` → object z-pass
  `Tactical_ObjectRenderingLoop @ 0x006D8DB0` (per `frontier-render-tactical`). No dedicated
  shadow pass for the VXL branch.

- **Load-time:** `VoxelAnimTypeClass::ReadINI @ 0x0074B050` (and the rules-level list reader)
  at boot/map load, out-of-sim.

---

## OUTGOING EDGES (this service depends on / drives)

| → Target service | Via (symbol + offset) | Evidence |
|---|---|---|
| `abstract-object` | ObjectClass base ctor/dtor, Reveal/conceal registers into the LogicClass live vector + Layer 3; vtable `+0x5C` AI / `+0x08` Draw slots | VoxelAnimClass : ObjectClass; ctor adds to `0x00887388` and the object layer (`GetLayer 0x0074A960` = 3). Doc-verified. |
| `frontier-render-tactical` | `VoxelAnim::Draw 0x0046B0C0` ← `FUN_00468090` (VXL rasterizer) ← object render loop | sole render path; needs the VXL/voxel rasterizer + Quaternion→matrix (`FUN_004399E0`, `Quaternion_ToMatrix 0x00646980`). Doc-verified. |
| `rules-class` | `VoxelAnimTypeClass::ReadINI 0x0074B050`; rules-level `[VoxelAnims]` list reader `RulesClass__ReadVoxelAnims 0x00672920` **[NEEDS-LIVE-VERIFY]**; reads global splash/Wake offsets on Rules instance (`+0x94` Wake, `+0xBC0` SplashList vector) | type data + global splash anim list consumed in AI water-landing branch. Doc-verified (rules instance offsets); list-reader address unverified. |
| `damage-helpers` | expire branch in `VoxelAnimClass::AI 0x00749F30` → area damage with Type `Warhead`/`Damage`/`DamageRadius` (`Apply_area_damage` family) | non-zero only for types with `Damage>0` (stock PIECE Damage=5 / DamageRadius=100 / Warhead=TankOGas). Doc-verified. |
| `frontier-anim` | AI spawns AnimClass for `BounceAnim` (each ground bounce), `ExpireAnim` (on expiry), `TrailerAnim` (every other tick) | type-string AnimType lookups; VoxelAnim spawns 2D anims but is not itself an anim. Doc-verified. |
| `frontier-particle` | ctor creates `ParticleSystemClass*` AttachedSystem (`+0x108`) if `Type->AttachedSystem != null` | attached emitter rides the bouncing debris while alive. Doc-verified. |
| `cell-map` | meteor `IsTiberium` impact crater: `CellClass::CanPlaceTiberium` + `OverlayToTiberiumIndex`, marks cell ore/overlay dirty (8-neighbor for meteors, single-cell otherwise); BounceClass::Update reads cell bridge flag `0x100` + building-in-cell for collision | conditional on meteor/Tiberium VoxelAnim types; bounce collision every airborne tick. Doc-verified. |
| `frontier-radar` | terrain-dirty crater marks radar via `RadarClass__MarkTerrainDirty` | only on the IsTiberium crater path; no object-tracker registration (debris is not radar-tracked). Doc-verified. |
| `random-scenario` | ctor draws launch velocity (X/Y spread, Z up) + random spin axis/angle (degrees→radians `0x007F65E8`) | **[NEEDS-LIVE-VERIFY which RNG instance]** — must confirm Scen->Random (synchronized/lockstep) vs g_MainRng (cosmetic). Debris spawns from a unit-death event so it is on the lockstep path; the per-callsite ECX binding must be read live. Rung-T RNG draws are "NOT statically enumerable" per the spine. |
| `frontier-audio-voc` | two SoundEvents (`+0x114` Start loop, `+0x128` Stop) from Type `StartSound`/`StopSound`; init via `FUN_00405BE0`; loop update analogous to AnimClass loop-sound | start loop on spawn, stop on expiry. Doc-verified (offsets); cue routing to the VocClass mixer is the audio service's side. |
| `factory-house` (HouseClass) | Owner ptr `+0x10C` carried from the spawning unit's house | ownership tag only (no economy interaction). Doc-verified. |

## INCOMING EDGES (who creates / drives this service)

The VOXELANIMCLASS report (§12.5) enumerates **exactly 5 creation sites** (all xrefs to ctor
`0x007493B0`) — doc-verified, no others:

| ← Source service | Via (call site) | Evidence |
|---|---|---|
| `techno-foot` / `damage-helpers` | `TechnoClass::ReceiveDamage 0x00701900` call @ `0x00702397` | **primary path** — vehicle death reads `TechnoTypeClass->DebrisTypes[]`/`DebrisMaximums[]`, spawns debris at death cell. Fires every vehicle explosion. |
| `damage-helpers` | `WarheadTypeClass::Detonate` call @ `0x00469DD5` | warhead's own DebrisTypes/DebrisMaximums on detonation. |
| `damage-helpers` | `Apply_area_damage 0x00489280` call @ `0x0048A3CF` | area-damage destruction spawning debris from affected units. |
| `frontier-voxelanim` (self) | `VoxelAnimClass::AI` internal call @ `0x0074A2FB` | meteor type with `Spawns=`/`SpawnCount` spawns child VoxelAnims on impact (recursive). |
| `frontier-trigger` | `FUN_006E2520` call @ `0x006E25E8` | map trigger/script action creates VoxelAnims by type index (campaign/scripted maps). |
| `frontier-saveload` | `FUN_0067D300` (SaveGame) @ `0x0067DF3C` iterates the live vector | serialization, not creation. |

---

## ACTIVE-IN-YR / TS-LEGACY

- **VoxelAnim debris (ordinary path, gravity=1.4):** LIVE in YR — fires on every vehicle
  explosion in a standard skirmish. Not TS-only.
- **Meteor / IsTiberium / IsMeteor branch:** LIVE but event-driven (meteor shower / gem-shard
  / weather paths), not every match tick.
- **`FUN_00439690` terrain-meteor spawn helper (gravity=3.0, angVel clamp 3.0):** **TS-LEGACY
  / DEAD in YR** — per `BOUNCE_CLASS_GHIDRA_REPORT.md` §13, `BounceClass__SpawnRandom
  0x00439690` is unreachable in a standard YR skirmish (distinct from the live gravity=1.4
  ordinary debris path). Do NOT implement as default.
- **Draw `+0x2F7` byte check:** a known original-engine bug (reads the MSB of the int
  `DamageRadius` at `+0x2F4` where it intended to read `Translucent` at `+0x295`). Reproduce
  the observable behavior, not the "intended" one, for parity.

---

## RUST PORT STATUS

No Rust implementation of the debris/bounce system. The existing `VoxelAnimation` struct
(`src/sim/components.rs:348`) is **unrelated** — it is a simple HVA frame-cycler for voxel
*unit* idle/walk animation, not the bouncing-debris physics. A real port needs:
VoxelAnimTypeClass INI parsing, BounceClass physics, the VoxelAnimClass entity on the
rung-T object tick, debris spawning wired into the death/damage path, and VXL draw at the
spun/bounced transform.

---

## OPEN ITEMS BEFORE IMPLEMENTATION (need a live Ghidra read)

1. **[NEEDS-LIVE-VERIFY]** Re-verify the representative `VoxelAnimClass::AI @ 0x00749F30`
   and `VoxelAnim::Draw @ 0x0046B0C0` against the live binary (Ghidra was offline this
   session) — confirm identity via `get_function_by_address` + body, not label.
2. **[NEEDS-LIVE-VERIFY]** `RulesClass__ReadVoxelAnims @ 0x00672920` — the rules-level
   `[VoxelAnims]` list reader claimed by the stub is not cited in the existing report; locate
   and confirm the real address.
3. **[NEEDS-LIVE-VERIFY]** The RNG instance(s) the spawn ctor draws from (Scen->Random
   lockstep vs g_MainRng cosmetic) — per-callsite ECX binding. Lockstep-critical because
   debris spawns on a synchronized unit-death event.
4. **[NEEDS-LIVE-VERIFY]** Which of `0x00B0F670` / `0x00A8EB28` is the canonical
   VoxelAnimTypeClass TypeList vs a transient registration vector.
