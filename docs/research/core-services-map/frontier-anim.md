# frontier-anim — AnimClass (sprite animations) — Core Service Profile

**Slug:** `frontier-anim`
**Status:** promoted from catalog stub D1 (`_frontier.md`) to full profile.
**Authority order:** binary → Ghidra → docs. **Active in YR:** Yes (every match).

> **Verification note (this session):** the live Ghidra MCP bridge for gamemd.exe was
> **not reachable** this session (no UDS instances; TCP `127.0.0.1:8089` refused;
> `list_instances` empty). Per RE discipline I did **not** invent or re-confirm addresses
> from a live decompile. Every address below is **carried from prior `[ghidra/verified]`
> research docs** in `docs/research/` (the AnimClass corpus is unusually deep — 20+
> dedicated reports), each of which cites its own Ghidra call. Addresses marked
> **UNVERIFIED-THIS-SESSION** mean "verified in the cited doc, not re-read live today."
> The two stub claims I correct below are corrected **against those verified docs**, not
> against a fresh decompile.

---

## PURPOSE

`AnimClass` is the engine's transient **SHP sprite-animation object**: explosions, muzzle
flashes, smoke, debris, building-slot/damage/death overlays, wake/warp/teleport visuals,
superweapon overlays, mind-control links, MoveFlash click feedback, and `Next=`/`Trailer`/
`Bounce`/`Expire` chains. It is an `ObjectClass`-derived live object — constructed, revealed
(registered into the LogicClass active-object vector), AI-ticked each frame (frame advance,
delay countdown, looping-sound update, bouncer/meteor physics, child spawns, damage-on-frame,
loop/`Next=` transition), drawn via its own `DrawIt`, and self-destroyed at end of life.

It also inherits the `StageClass` frame-advance primitive (RTTI-confirmed at +0xAC) but
**does not** use `StageClass::Stage_Changed` — `AnimClass::AI` drives advance directly through
its own CDTimer.

---

## WHAT IT OWNS (globals / structs — addresses from verified corpus, UNVERIFIED-THIS-SESSION)

| Global / struct | Address / offset | Role |
|---|---|---|
| `g_AnimClass_Array` (data ptr) | `0x00A8E9AC` | `DynamicVectorClass<AnimClass*>` — **registry / lifetime / owner-scan** list, NOT the AI scheduler |
| `g_AnimClass_Array_Count` | `0x00A8E9B8` | registry count |
| `g_AnimClass_Array_Capacity` | `0x00A8E9B0` | registry capacity |
| `g_AnimTypes_Array` (data ptr) | `0x008B4154` | `AnimTypeClass*` array (parsed from art(md).ini) |
| `g_AnimTypeClass_Count` | `0x008B4160` | type count |
| MoveFlash subset vector (`DAT_00a83e00`) | base `DAT_00a83e04`, count `DAT_00a83e10`, cap `0xA` | **separate** small vector iterated by spine Rung U; stock occupants = MoveFlash anims |
| `AnimClass` vtable | `0x007E3354` | AI at slot `+0x5C`, DrawIt at slot `+0x114` |
| `AnimTypeClass` vtable (primary) | `0x007E3608` | + 3 secondary vtables (`0x7E35EC/35E4/35DC`, IUnknown bridge) |
| `AnimClass` instance | size `0x1C8` (456 B) | inherits `ObjectClass`; key fields below |

**Key AnimClass instance fields** (byte offsets, from `ANIM_CLASS_GHIDRA_REPORT.md`):
`+0xAC` CurrentFrame · `+0xB0` FrameAdvanced · `+0xB4` LastFrameTime (= `g_CurrentFrameCounter`)
· `+0xBC` FrameDelay · `+0xC0` FrameDelayReload (Rate) · `+0xC4` FrameStep (±1) · `+0xC8`
`AnimTypeClass*` · `+0xCC` OwnerObject · `+0xD4` Palette · `+0xFC` Strength · `+0x100` ZAdjust
· `+0x180` OwnerHouse · `+0x184` Delay · `+0x188` AccumulatedDamage (double) · `+0x190`
DrawFlags · `+0x194` IsBouncer · `+0x195` LoopCountRemaining · `+0x19B` IsInactive · `+0x19C`
**first-AI guard** · `+0x1A0` looping-sound handle.

**Key AnimTypeClass fields** (relevant to cross-service behavior):
`+0x2B0` Rate (`900 / INI Rate`) · `+0x2C8` `Next=` · `+0x2CC` `SpawnsParticle=` ·
`+0x2D0` `NumParticles=` · `+0x2F8` `StartSound=`/`Report=` (VocClass index) · `+0x2FC`
`StopSound=` · `+0x300` `BounceAnim=` · `+0x304` `ExpireAnim=` · `+0x308` `TrailerAnim=` ·
`+0x30C` `TrailerSeperation=`.

---

## KEY FUNCTIONS (addresses from verified corpus, UNVERIFIED-THIS-SESSION)

| Function | Address | Role |
|---|---|---|
| `AnimClass::AI` | `0x00423AC0` | **representative fn** — per-frame update (vtable slot **`+0x5C`**) |
| `AnimClass::Constructor` (full) | `0x00421EA0` | 7 params + this; appends to `g_AnimClass_Array`, sets first-AI guard `+0x19C=1`, calls `ObjectClass::Reveal` for normal types |
| `AnimClass::Constructor` (load) | `0x00422720` | deserialization (no params) |
| `AnimClass::DrawIt` | `0x00422CA0` | render (vtable slot `+0x114`) |
| `AnimClass::Middle` | `0x00424CE0` | called when delay expires / `Next=` transition; plays StartSound |
| `AnimClass::Start` | `0x00424F00` | sound/particle/scorch on start |
| `AnimClass::Destroy` | `0x004255B0` | detach owner, release sound, optional StopSound, → `ObjectClass::UnInit` |
| `AnimClass::SetOwnerObject` | `0x00424B50` | attach/detach to a TechnoClass owner |
| `AnimClass::GetZAdjust` | `0x00425630` | depth: `+0xA4` (+ owner `+0xA4` when attached) |
| `AnimClass::ProcessBounceResult` | `0x00423930` | bouncer/meteor bounce → optional `BounceAnim` child |
| `AnimClass::UpdateLoopingSound` | `0x00750D40` | maintains continuous anim sound, volume/pan by distance, stops if too far (stub had this address; name corrected in corpus) |
| `AnimTypeClass::ReadINI` | `0x00427D00` | parses art(md).ini AnimType fields |
| `AnimTypeClass::Constructor` | `0x00427530` | |
| `AnimTypeClass::FindOrAllocate` | `0x00428B80` | name lookup over `g_AnimTypes_Array` |

---

## TICK / RENDER PLUG POINT (cite the spine)

AnimClass touches **two different rungs** of `LogicClass::PerTickUpdate` (`0x0055AFB0`,
sole caller `Main_Tick @ 0x0055D360`) — this is the key correction to the stub:

- **Rung T (rung 20), MAIN object vector** — driver `0x005F3E70` (`ObjectClass::AI`,
  vt+0x5c, polymorphic). This is where **general / live AnimClass objects** (explosions,
  muzzle flashes, debris, trailers, building overlays, etc.) get `AnimClass::AI` dispatched
  every tick. They were inserted into the LogicClass active-object vector at construction via
  `ObjectClass::Reveal → FUN_0055BAA0` (singleton `0x0087F778`). The scheduler **reloads the
  live count after each `vt+0x5c` call** (`0x0055B613`), so child anims tail-appended during a
  parent's AI (trailer/bounce/expire) are **same-pass eligible**. Removal compacts left
  (`FUN_0055BAE0`); the cursor is not repaired. **Active in YR: Yes, every tick.**

- **Rung U (rung 21), AnimClass-SUBSET vector** — driver `0x00423ac0` (`AnimClass::AI`
  called directly) over a **separate** small `DynamicVectorClass` `DAT_00a83e00` (cap `0xA`),
  whose stock occupants are **MoveFlash** anims. This rung is **mode-gated**:
  `g_GameMode (0x00a8b238) != 0 && != 5`. Stock MoveFlash occupants trigger **0 RNG draws**
  here. **Active in YR: Yes** (MoveFlash created on essentially every move/attack order),
  but only in game modes other than 0/5.

> **Stub correction:** the stub said "the anim pass is SKIPPED in skirmish modes 0/5." That
> is true **only for Rung U** (the MoveFlash subset vector). General AnimClass AI runs every
> tick via **Rung T** and is **not** mode-gated. Source: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`
> rungs T (20) & U (21); `ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`.

> **Stub correction:** the stub gave the AI head as `+0x5C` but separately referenced "rung N"
> and vtable slot `0x60`. The verified vtable slot is **`+0x5C` (slot 23)**, not `+0x60`
> (corrected in `ANIM_CLASS_GHIDRA_REPORT.md`, root cause OFFSET_RETYPED_WRONG). And the rung
> labels in the 28-rung spine are **T and U**, not "N" (the stub used an older lettering).

**Render plug point:** the RENDER pass, not the tick. `AnimClass::DrawIt @ 0x00422CA0` is
called from the tactical draw pass `TacticalClass_Draw @ 0x006D3D10` (animations/object
layer), with depth via `AnimClass::GetZAdjust @ 0x00425630` → `Tactical__AdjustForZ @
0x006D20E0`. The animations layer head is `Tactical_layer_animations @ 0x006D3870`
(per stub A1). Draw flags originate from instance `+0x190` (`0x600` general / `0x1600`
building-slot / `0x2600` warhead-explosion & bouncer-expire).

**Load-time plug point:** `AnimTypeClass::ReadINI @ 0x00427D00` parses art(md).ini at boot.

---

## OUTGOING EDGES (frontier-anim depends on …)

| Target service | Via symbol / mechanism | Evidence |
|---|---|---|
| `abstract-object` | `ObjectClass::Reveal @ 0x005F4EC0` → `FUN_0055BAA0` register; `ObjectClass::UnInit @ 0x005F65F0` on destroy; AnimClass IS an ObjectClass subclass; vt+0x5c AI dispatch | `ANIMCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md`; `ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md` |
| `logicclass` | scheduled by `LogicClass::PerTickUpdate @ 0x0055AFB0` Rung T (general) / Rung U (MoveFlash); live-vector count reload + compaction is the same-pass contract | `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` rungs T/U |
| `rules-class` | `AnimTypeClass::ReadINI @ 0x00427D00` reads art(md).ini; debris / wake / death / explosion AnimTypes selected from `RulesClass` pointer lists at spawn | `ANIM_CLASS_GHIDRA_REPORT.md`; `ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md` |
| `damage-helpers` | bouncer/meteor impact & damage-on-frame → area damage (`Apply_area_damage`); `AccumulatedDamage +0x188` | `ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`; `ANIMCLASS_BOUNCER_WATER_SPLASH_BRANCH_GHIDRA_REPORT.md` |
| `cell-map` | anim cell placement / GetCoords; bouncer landtype (water vs land) branch; TIBTRE ore-spawn via `CellClass::SpreadTiberium` (note: ore spawn is a TerrainClass path, NOT an AnimClass path — see TS/legacy note) | `ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`; `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md` |
| `frontier-audio-voc` | `StartSound`/`Report` (`+0x2F8`) + `StopSound` (`+0x2FC`) → `VocClass::PlayAt @ 0x007509E0`; looping sound via `AnimClass::UpdateLoopingSound @ 0x00750D40` | `ANIMATION_SOUNDS_GHIDRA_REPORT.md`; `ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING_GHIDRA_REPORT.md` |
| `frontier-particle` | AnimType `SpawnsParticle=` (`+0x2CC`) / `NumParticles=` (`+0x2D0`) spawn ParticleSystem | `ANIMATION_SOUNDS_GHIDRA_REPORT.md` field table |
| `frontier-render-tactical` | `AnimClass::DrawIt @ 0x00422CA0` drawn by `TacticalClass_Draw @ 0x006D3D10`; depth via `Tactical__AdjustForZ @ 0x006D20E0` | `ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md`; `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md` |
| `random-scenario` | RNG sites inside `AnimClass::AI` bind `Scen->Random` (e.g. debris/expire jitter, random current-frame on damage-fire); none triggered by stock MoveFlash in Rung U | `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung U; `ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md` |
| `lookup-tables` | StageClass frame-advance primitive (inherited at +0xAC); 0x10000-entry gradient/translucency LUTs consumed by DrawIt translucency branch | `STAGECLASS_FRAME_ADVANCE_PRIMITIVE_GHIDRA_REPORT.md`; `ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md` |

---

## INCOMING EDGES (… spawn or drive frontier-anim)

`AnimClass::Constructor @ 0x00421EA0` has **70+ direct callers** (per the caller-taxonomy
report). The materially active-in-YR families:

| Source service | Via symbol | Evidence |
|---|---|---|
| `techno-foot` | muzzle flash / `OccupantAnim` via `TechnoClass::Fire_At @ 0x006FDD50` (`0x006FDD50`); attached via `SetOwnerObject` | `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md` |
| `damage-helpers` | `WarheadTypeClass::Detonate @ 0x004690B0` → explosion + debris anims (`drawFlags=0x2600`) | caller taxonomy report |
| `factory-house` / building (`abstract-object`) | `BuildingClass::CreateAnimForSlot @ 0x00451890` (slot overlays, `0x1600`), `CreateDamageFireAnims @ 0x0043C0D0`, `DestructionEffects @ 0x004415F0` | caller taxonomy report; `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md` |
| `techno-foot` (infantry/unit/aircraft) | `InfantryClass::DoType_Sequencer @ 0x00520AE0` (death), `AircraftClass::AI/ReceiveDamage` (smoke/debris), `UnitClass::ReceiveDamage @ 0x00737C90` (water death) | caller taxonomy report |
| `techno-foot` (locomotors) | wake/warp: `DriveLocomotionClass::Process @ 0x004B0500`, `HoverLocomotionClass::Move @ 0x00514310`, `ShipLocomotionClass::Process @ 0x0069FC10`, `TeleportLocomotionClass::InitiateWarp @ 0x00719400` | caller taxonomy report; `WARPOUT_SHP_DRAW_FRAME_PALETTE_RATE_GHIDRA_REPORT.md` |
| `frontier-super` | `LightningStorm::GroundStrike @ 0x0053A300` / `CreateCloudBolt @ 0x0053A140`; `PsychicDominator::MindControlArea @ 0x0053B080` | caller taxonomy report |
| `frontier-voxelanim` | `VoxelAnimClass::AI @ 0x00749F30` spawns AnimClass trailer/bounce/expire children | `VOXELANIMCLASS_GHIDRA_REPORT.md` |
| `frontier-bullet` | bomb detonation `BombClass::Detonate @ 0x00438720` explosion anims; bullet detonation explosion/debris | caller taxonomy report |
| `frontier-anim` (self) | `AnimClass::AI` spawns `TrailerAnim`/`BounceAnim`/`ExpireAnim` children + `Next=` in-place transition (self-edge) | `ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md`; `ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md` |
| `bridge-helpers` | bridge collapse anims `CellClass::BlowUpBridge @ 0x0047DD70` (random delay) | caller taxonomy report |
| `frontier-trigger` (campaign) | script/action helper anims (`FUN_006E*` cluster) — conditional, campaign-scoped | caller taxonomy report (liveness UNCHECKED) |
| `frontier-render-tactical` | walks `AnimClass::DrawIt` from the tactical draw pass (render consumer, not a sim spawner) | render edge above |

---

## ACTIVE-IN-YR / TS-LEGACY

- **General AnimClass AI (Rung T): Active every match** — explosions, muzzle flashes,
  debris, building overlays, death/smoke effects fire constantly. Not mode-gated.
- **MoveFlash subset (Rung U): Active** in modes ≠ 0/5 — click feedback on (almost) every
  move/attack order. Mode-gated; 0 RNG with stock occupants.
- **`Next=` chains, looping sound, owner attach/detach: Active.** Stock content uses them.
- **TrailerAnim / TrailerSeperation / ExpireAnim / Bouncer / IsMeteor / BounceAnim:
  Conditional** — present in stock `artmd.ini` for debris/meteor rows (`DBRIS*`, `METLARGE`,
  `METSMALL`); `BounceAnim=` has **no stock row** (engine path only). Not TS-legacy, just
  content-gated.
- **TS-legacy / NOT an AnimClass path:** TIBTRE ore spawn is **not** an AnimClass
  effect — `TerrainClass::AI` calls `CellClass::SpreadTiberium` directly. A prior doc claimed
  TIBTRE spawns ore via an AnimClass with `TiberiumSpawnType`; that was corrected to WRONG.
  The bouncer-impact ore-spawn path (`AnimType+0x338/+0x33C`) is the only ore link, and it is
  not the TIBTRE path. (`ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`)

---

## STUB DELTA SUMMARY (what this profile corrects vs `_frontier.md` D1)

1. **Representative address `AnimClass::AI @ 0x00423AC0`** — confirmed correct by the
   verified corpus (UNVERIFIED-THIS-SESSION live; live bridge was down).
2. **Vtable slot:** AI is `+0x5C` (slot 23), **not** `+0x60`. Stub's own "+0x5C AI head"
   was right; its "(rung N)" / slot-0x60 phrasing was the older lettering/offset.
3. **Plug point:** general anims run via **spine Rung T** (every tick, not mode-gated);
   only the **MoveFlash subset (Rung U)** is mode-gated to `g_GameMode != 0 && != 5`. The
   stub's "anim pass is SKIPPED in skirmish modes 0/5" applies **only to Rung U**.
4. **`AnimClass::UpdateLoopingSound @ 0x00750D40`** — address matches stub; the corpus
   confirms this is the looping-sound maintainer (not a generic SpawnDetached).
5. **Most-depends-on** in the stub listed abstract-object / damage-helpers / cell-map; the
   real heaviest in-edge volume is from `techno-foot` + `damage-helpers` (combat visuals)
   and the heaviest render out-edge is `frontier-render-tactical`.

---

## SOURCES (all `[ghidra/verified]` in docs/research; addresses UNVERIFIED-THIS-SESSION — live bridge down)

`ANIM_CLASS_GHIDRA_REPORT.md`, `ANIMCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md`,
`ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`,
`ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md`,
`ANIMATION_SOUNDS_GHIDRA_REPORT.md`, `ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING_GHIDRA_REPORT.md`,
`ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md`,
`OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`,
`ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`,
`ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md`,
`ANIMCLASS_AI_TRAILER_NEXT_INTERACTION_GHIDRA_REPORT.md`,
`ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`,
`ANIMCLASS_BOUNCER_WATER_SPLASH_BRANCH_GHIDRA_REPORT.md`,
`STAGECLASS_FRAME_ADVANCE_PRIMITIVE_GHIDRA_REPORT.md`,
`WARPOUT_SHP_DRAW_FRAME_PALETTE_RATE_GHIDRA_REPORT.md`,
`TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`, `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`,
`VOXELANIMCLASS_GHIDRA_REPORT.md`, `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`.

**Next step before implementation:** re-verify `AnimClass::AI @ 0x00423AC0`, the vtable
slot at `0x007E3354+0x5C`, and the Rung T/U driver split (`0x005F3E70` vs `0x00423AC0`) with
a **live** Ghidra session — this profile is a synthesis of prior verified docs, not a fresh
decompile.
