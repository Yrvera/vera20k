# TechnoClass AI Migration Boundary - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, main object loop `0x0055B5FB..0x0055B619`, `TechnoClass::AI_Update @ 0x006F9E50`, `FootClass::AI @ 0x004DA530`, `UnitClass::AI @ 0x007360C0`, `InfantryClass::AI @ 0x0051BAB0`, `AircraftClass::AI @ 0x00414BB0`, `BuildingClass::Update @ 0x0043FB20`
**Investigation Mode:** coverage-map
**Claimed Scope:** migration boundary for TechnoClass-derived `vtable+0x5C` bodies, emphasizing UnitClass/FootClass/common Techno slices and classifying current Rust movement/combat/mission phases as object-AI candidates vs global services.
**Non-Scope:** exhaustive decompilation of every TechnoClass AI branch, full BuildingClass breadth, new implementation, BulletClass/AnimClass migration slices, and every mission handler's internals.
**Confidence:** High for scheduler placement and Unit/Foot/Techno call order; Medium for phase classification where it maps broad Rust systems to not-yet-ported class bodies.
**Active in YR:** Yes for the core object AI path. Conditional where noted for specific class/type gates.

## 0. Investigation Contract

**Target question:** Which current Rust movement/combat/mission phases belong under TechnoClass-derived `vtable+0x5C` object AI, which must stay global pre/post LogicClass services, and what is the first safe TechnoClass/UnitClass/InfantryClass/AircraftClass migration boundary?

**Non-goals:** Do not implement Rust; do not expand into BulletClass/AnimClass; do not drain BuildingClass-specific systems except to mark the boundary; do not re-prove the already-settled live LogicClass vector contract.

**Evidence needed to mark COMPLETE:** scheduler anchor evidence; vtable/call-order evidence for `UnitClass::AI -> FootClass::AI -> TechnoClass::AI_Update -> Mission_Dispatch -> locomotor Process`; evidence that UnitClass post-Foot work includes turret/fire/facing/harvest/ammo/spawn-manager surfaces; Rust `advance_tick` phase evidence; at least one actionable implementation handoff.

**Stop conditions:** stop once the migration boundary is classified and the first safe slice is named; defer full mission-handler, BuildingClass, and class-specific edge-case inventories.

## 1. Overview

Native YR does not run "movement", "combat", or "missions" as top-level global phases for Techno objects. The main `LogicClass::PerTickUpdate` object loop calls each registered object's `vtable+0x5C`; for common vehicles this reaches `UnitClass::AI`, which calls `FootClass::AI`, which immediately calls `TechnoClass::AI_Update`, which dispatches the mission before `FootClass` later calls the active locomotor `Process`.

That means the safest first Techno migration boundary is a per-object AI shell for mobile Techno-derived objects, not a whole-system transplant of combat, power, ore, or house AI. The shell should preserve per-object order and live-vector append/remove semantics while leaving global `PerTickUpdate` services in their verified pre/post positions.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning | Evidence | Active in YR |
|---|---|---|---|---|
| `vtable+0x5C` | all Abstract-derived objects | per-object AI/update slot called by the main LogicClass object vector | `0x0055B60E..0x0055B610`; scheduler report | Yes |
| `ObjectClass+0x98` | ObjectClass | LogicClass active-list membership byte | add/remove helpers `0x0055BAA0`, `0x0055BAE0`; scheduler report | Yes |
| `FootClass+0x674` | FootClass | active `ILocomotion*`; `Process` is vtable `+0x40` | `FootClass::AI` process site around `0x004DA877`; `FOOTCLASS_AI_GHIDRA_REPORT.md` | Yes for mobile Foot objects |
| `MissionClass+0xC8/+0xD0` | MissionClass | mission timer start/delay used by `Mission_Dispatch` | `Mission_Dispatch @ 0x005B3060`; unload accumulator report | Yes |
| `TechnoClass+0x2D0` | TechnoClass | `SpawnManagerClass*`; manager AI called from `TechnoClass::AI_Update` | `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` | Conditional; only types with `Spawns=` |
| `TechnoClass+0x220..0x2F4` | TechnoClass | cloak state/timers | `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` | Conditional; cloak-capable/veteran cloak |
| `TechnoClass+0x140/+0x144/+0x148` | TechnoClass | gattling stage/value/cycle count | `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` | Conditional; `IsGattling=yes` |
| `UnitClass+0x6C4` | UnitClass | UnitType pointer used by UnitClass-specific AI after `FootClass::AI` | `UNITCLASS_GHIDRA_REPORT.md` | Yes for UnitClass |
| `AircraftClass+0x6C4..0x6D5` | AircraftClass | aircraft-specific type/mission state flags | `AIRCRAFTCLASS_GHIDRA_REPORT.md` | Yes for AircraftClass |
| `BuildingClass+0x520` | BuildingClass | BuildingType pointer; `BuildingClass::Update` wraps `TechnoClass::AI_Update` with building-specific work | `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` | Yes for BuildingClass |

## 3. Core Logic

### 3.1 Native object AI ordering for common units

Verified active-YR order:

1. `LogicClass::PerTickUpdate` main object loop loads a live object and calls `vtable+0x5C` at `0x0055B610`.
2. For UnitClass, `vtable+0x5C` is `UnitClass::AI @ 0x007360C0` by the UnitClass vtable map; fresh assembly context confirms `UnitClass::AI` calls `FootClass::AI` at `0x0073647B`.
3. `FootClass::AI @ 0x004DA530` starts by calling `TechnoClass::AI_Update @ 0x006F9E50` at `0x004DA539`; fresh assembly context confirms the call.
4. `TechnoClass::AI_Update` calls `MissionClass::Mission_Dispatch @ 0x005B3060` at `0x006FA655`; fresh assembly context confirms the call.
5. `FootClass::AI` later calls active locomotor `ILocomotion::Process` through locomotor vtable `+0x40`; fresh context shows the call immediately before `0x004DA87A`.
6. `UnitClass::AI` then continues UnitClass-specific behavior: turret AI, fire-at-target, facing update, harvest brain, ammo/reload wrapper, SpawnManager update, auto-hunt/stuck harvester rescue per `UNITCLASS_GHIDRA_REPORT.md`.

Active in YR: Yes. Stock vehicles, harvesters, gattling units, carriers, dreadnoughts, and normal mobile units reach these bodies when revealed and registered in LogicClass.

### 3.2 TechnoClass common work is object AI, not a global service

`TechnoClass::AI_Update` is inherited common object AI. Reports place these surfaces inside it or directly under it:

| Work | Native owner | Evidence | Active in YR |
|---|---|---|---|
| Mission timer dispatch and mission handlers | `TechnoClass::AI_Update -> MissionClass::Mission_Dispatch` | `0x006FA655`; `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md` | Yes |
| Timer cluster accumulator after mission dispatch | `TechnoClass::AI_Update` | `0x006FABC4..0x006FAC2A`; unload accumulator report | Yes for non-building timer-cluster consumers |
| Cloak tick / auto-recloak / visual progress | Techno common cloak helpers called from Techno AI | `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` | Conditional; cloakable/veteran cloak |
| EMP/temporal/gap visual/health smoothing/target validation | Techno common AI update | `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` phase 11 summary; Techno systems docs | Conditional by state/type |
| SpawnManager and SlaveManager AI | `TechnoClass::AI_Update` manager pointers | `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md`, `SpawnManager->vtable+0x5C` | Conditional; `Spawns=`/slave systems |
| Passive target acquisition / retaliation scan | Techno combat systems under AI_Update | `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` | Conditional; unit state/rules/IQ |

These belong in the future per-object AI body or helper calls invoked from that body. They should not stay as independent global sweeps once the native tick spine is authoritative.

### 3.3 Current Rust phase classification

| Current Rust phase/surface | Native migration classification | Evidence | Active in YR |
|---|---|---|---|
| `movement::tick_movement_with_grids` Phase 1 | Object-AI candidate: FootClass locomotor `Process` and per-cell callbacks, but current Rust is a snapshot/global phase | Rust `src/sim/world/mod.rs:1534..1563`; native `0x004DA87A` process after mission dispatch | Yes for Foot objects |
| `air_movement`, `teleport_movement`, `tunnel_movement`, `rocket/homing/droppod/parachute` Phase 2 | Mixed object-AI candidates. Air/teleport/tunnel locomotor progression belongs under the object's AI/locomotor; projectile movement is BulletClass scope, not this Techno slice | Rust `src/sim/world/mod.rs:1567..1615`; Foot locomotor process evidence; Bullet reports for projectiles | Conditional by locomotor/class |
| `tick_locomotor_piggyback_restore` | Object-AI candidate: explicitly FootClass AI tail behavior | Rust `src/sim/world/mod.rs:1617`; `FOOTCLASS_AI_GHIDRA_REPORT.md` IPiggyback sequence | Conditional; chrono/piggyback locos |
| `rocking::tick` Phase 2.5 | Likely object-AI/render-state candidate for UnitClass/Foot visual physics; do not make global forever if per-object AI migrates | Rust `src/sim/world/mod.rs:1619..1632`; rocking and frame-basis docs | Conditional; moving vehicles/ships |
| `aircraft::tick_aircraft_missions` | Object-AI candidate: AircraftClass mission dispatch/mission handlers, currently snapshot global phase | Rust `src/sim/aircraft/mod.rs:144..183`; AircraftClass vtable/missions in `AIRCRAFTCLASS_GHIDRA_REPORT.md` | Yes for AircraftClass |
| `refresh_fog` vision | Global/pre-post service, not Techno object AI as a whole; object sight updates and reveal side effects are object/lifecycle, but full owner visibility recompute is Rust-specific global | Rust `src/sim/world/mod.rs:1690..1701`; native top-level ladder has separate shroud/fog branches and object AI has local visibility hooks | Conditional/default YR shroud; TS fog branch off by default |
| `power_system::tick_power_states` | Global/House/building service, not first Unit/Techno AI slice | Rust `src/sim/world/mod.rs:1704..1713`; BuildingClass and HouseClass docs | Yes, but outside first mobile slice |
| `superweapon::tick_superweapons` | Global pre-object `PerTickUpdate`/HouseClass-style service, not Techno object AI | Rust `src/sim/world/mod.rs:1714..1719`; PerTick ladder LightningStorm order 18; HouseClass tail reports | Conditional; game option/charged SW |
| `deploy::tick_deploy_state` | Likely object-AI candidate for infantry/simple deployers; keep narrow until InfantryClass/UnitClass deploy handlers are traced | Rust `src/sim/world/mod.rs:1721..1726`; UnitClass deploy/Infantry sequence docs | Conditional; deployable types |
| `infantry::tick_fear_for_entities` | Object-AI candidate: InfantryClass AI/sequence/fear-related work, but not safe in first UnitClass-only slice | Rust `src/sim/world/mod.rs:1728..1730`; Infantry death/AI sequence docs | Conditional; infantry |
| `combat::tick_combat_with_fog` | Split: UnitClass fire/turret/facing/target validation belongs in object AI; AoE/damage helper internals are called by fire paths, not a top-level global combat sweep | Rust `src/sim/world/mod.rs:1732..1777`; UnitClass AI post-Foot order; Techno combat docs | Yes |
| `turret::tick_turret_rotation` | Object-AI candidate: UnitClass `Facing_Update` after `Fire_At_Target`, not a global post-combat sweep | Rust `src/sim/world/mod.rs:1778..1783`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` | Conditional; turreted units |
| bridge/wall/terrain damage event drains | Global/effect-service or helper side effects called from weapon/damage paths; do not fold blindly into Techno AI | Rust `src/sim/world/mod.rs:1784..1958`; bridge reports | Conditional |
| `particles::system_ai::tick_particle_systems` | Global pre-object `PerTickUpdate` particle/effect service for RadSite/Wave/etc, plus object-spawned systems. Not first Techno slice | Rust `src/sim/world/mod.rs:1966..1969`; PerTick unnamed callee/rad/wave reports | Conditional |
| `tick_retaliation` | Object-AI candidate if mirroring TechnoClass retaliation/passive scan; currently a global phase | Rust `src/sim/world/mod.rs:1970..1975`; Techno combat systems report | Conditional |
| passenger system | Mixed: cargo/limbo side effects are object/lifecycle; broad ownership/ejection sweeps need separate proof | Rust `src/sim/world/mod.rs:1972..1975`; passenger/garrison reports | Conditional |
| production/repairs/docks/ore Phase 7 | Mostly global tail or separate Factory/Building/House services. Ore growth/spread is verified pre-object global, not Techno AI | Rust `src/sim/world/mod.rs:1976..2078`; PerTick ladder orders 9-10, 27-28 | Yes/conditional; not first Techno slice |
| AI players/defeat/building anim/world effects late region | HouseClass/global/tail services or BuildingClass-specific; not first mobile Techno slice | Rust `src/sim/world/mod.rs:1407..1503`; Factory/House and BuildingClass reports | Yes/conditional |

### 3.4 First safe migration boundary

First safe slice: introduce or use the existing active-object scheduler to run a **mobile Techno object-AI shell** for UnitClass-family objects, but initially migrate only behavior whose native order is already proved:

1. `UnitClass::AI` entry in active LogicClass order.
2. call `FootClass::AI` at the same point.
3. inside `FootClass::AI`, call a Techno common update shell before locomotor progression.
4. inside the Techno shell, keep mission dispatch before locomotor `Process`.
5. after FootClass returns, leave UnitClass post-Foot behavior in a compatibility bridge until each sub-slice is verified, but do not move unrelated global services under this shell.

This first slice can start with an instrumented/no-op or narrow behavior-bearing shell that preserves call order and membership semantics before moving high-risk combat/math. The first behavior-bearing candidate is **locomotor process order relative to mission dispatch**, because the order is strongly verified and current Rust movement already exists as a separable phase. The first combat-bearing candidate should be **UnitClass turret/fire/facing for one bounded vehicle scenario**, after acceptance tests cover mission-before-locomotor and same-pass append/remove behavior.

BuildingClass should not be the first mobile Techno migration slice. `BuildingClass::Update @ 0x0043FB20` runs extensive building-specific work before and after `TechnoClass::AI_Update`, including damage fires, docked-object update, warp sparkle early returns, building animation, delayed fire, auto-sell/civilian checks, gates, power/repair, and zero-health destruction. Active in YR: Yes, but high blast radius.

## 4. INI Keys

This boundary report does not exhaust parsers, but these keys prove active-YR reachability for the object-AI candidates:

| Key | Source / default or stock consumer | Effect | Active in YR |
|---|---|---|---|
| `Spawns=` / `SpawnsNumber=` / `SpawnRegenRate=` / `SpawnReloadRate=` | stock `[CARRIER]`, `[DEST]`, `[DRED]`, `[BSUB]`, `[V3]` | enables SpawnManager at `TechnoClass+0x2D0` and manager AI from Techno AI | Yes |
| `IsGattling=yes` and stage/rate keys | stock `[YTNK]`, `[YAGGUN]` | enables Unit/Building gattling fire-stage behavior | Yes |
| `Cloakable=`, `CloakingSpeed=`, `CloakStop=` | TechnoType keys | enable Techno cloak tick gates | Conditional; only set on relevant types/veterancy |
| `FogOfWar=no` | stock YR defaults | TS fog branch in top-level ladder is skipped by default | No for standard skirmish default; conditional if enabled |
| `ShroudGrow=no` | stock YR defaults | shroud regrowth helper skipped by default | No for standard skirmish default; conditional if enabled |
| `Harvester=yes`, `DockUnload=yes`, `Refinery=yes` | stock `HARV/CMIN`, `GAREFN/NAREFN` | proves Unit/Foot/Techno mission-dispatch/locomotor ordering on stock refinery flow | Yes |

## 5. Integration Points

| Integration | Finding | Evidence | Active in YR |
|---|---|---|---|
| Main object loop | Calls registered objects through `vtable+0x5C` with live count reload | `0x0055B608..0x0055B619` | Yes |
| UnitClass | `UnitClass::AI` calls `FootClass::AI`, then continues Unit-specific turret/fire/facing/harvest/ammo/spawn work | `0x0073647B`; `UNITCLASS_GHIDRA_REPORT.md` | Yes |
| FootClass | `FootClass::AI` calls `TechnoClass::AI_Update` before locomotor `Process` | `0x004DA539`, `0x004DA877..0x004DA87A` | Yes |
| TechnoClass | `AI_Update` calls `Mission_Dispatch` before later timer-cluster accumulator work | `0x006FA655`, `0x006FABC4..0x006FAC2A` | Yes |
| InfantryClass | `InfantryClass::AI @ 0x0051BAB0` is a `vtable+0x5C` body and can self-remove via sequencer paths | `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` | Yes |
| AircraftClass | AircraftClass `vtable+0x5C = 0x00414BB0`; mission handlers are AircraftClass slots | `AIRCRAFTCLASS_GHIDRA_REPORT.md` | Yes |
| BuildingClass | `BuildingClass::Update` is `vtable+0x5C` and calls `TechnoClass::AI_Update` in phase 11, surrounded by building-specific phases | `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

Rust has pieces of the needed data structures but still runs most behavior as global staged phases:

| Rust surface | Current status | Evidence |
|---|---|---|
| Active object order | `World::live_object_order_snapshot()` exposes `self.logic.snapshot()`; current consumers still use snapshots, not the native live per-object call loop | `src/sim/world/mod.rs:745`, `:1760`, `:1972` |
| Movement | global `tick_movement_with_grids` snapshots/sorts entity keys | `src/sim/world/mod.rs:1534`; `src/sim/movement/movement_tick.rs:856` |
| Air/special movement | global `tick_air_movement`, `tick_teleport_movement`, `tick_rocket_movement`, `tick_homing_movement`, etc. | `src/sim/world/mod.rs:1570..1615`; movement module key snapshots |
| Aircraft missions | global mission sweep snapshots all aircraft missions before processing | `src/sim/aircraft/mod.rs:144..183` |
| Combat/fire/turret | combat snapshots attackers, sorts by live order, then turret rotation is a separate later sweep | `src/sim/combat/mod.rs:1174..1549`; `src/sim/world/mod.rs:1760..1783`; `src/sim/movement/turret.rs:82..95` |
| Techno common systems | no single TechnoClass AI_Update owner; cloak, target, gattling, spawn-manager, mission, ammo, and timer-cluster equivalents are scattered or absent | repo scan plus reports cited above |
| Global services | power, superweapons, production, repairs, ore, AI, defeat, building/world effects remain global phases | `src/sim/world/mod.rs:1704..2078`, `run_late_region` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Main LogicClass object scheduler | verified | `0x0055B608..0x0055B619`; scheduler report; fresh assembly context | none |
| `UnitClass::AI -> FootClass::AI` | verified | `0x0073647B`; UnitClass report; fresh assembly context | exact pre-Foot UnitClass branches not drained here |
| `FootClass::AI -> TechnoClass::AI_Update` | verified | `0x004DA539`; fresh assembly context | full FootClass body not redrained |
| `TechnoClass::AI_Update -> Mission_Dispatch` | verified | `0x006FA655`; fresh assembly context | full AI_Update branch inventory deferred |
| `FootClass` locomotor `Process` after mission dispatch | verified | `0x004DA877..0x004DA87A`; Foot/Unit caller reports | exact locomotor internals deferred |
| UnitClass post-Foot behavior inventory | touched-not-exhausted | `UNITCLASS_GHIDRA_REPORT.md` | verify each migration sub-slice before moving |
| AircraftClass AI boundary | touched-not-exhausted | `AIRCRAFTCLASS_GHIDRA_REPORT.md` | aircraft AI internal order not redrained |
| InfantryClass AI boundary | touched-not-exhausted | Infantry/self-removal reports | full fear/deploy/sequence inventory deferred |
| BuildingClass boundary | touched-not-exhausted | `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` | deliberately not first-slice scope |
| Current Rust phase map | verified for current source | `src/sim/world/mod.rs:1508..2087`; module rg | none for boundary classification |
| TS legacy filter | verified for key global branches | `FogOfWar=no`, `ShroudGrow=no` reports | exact custom-map flag activation deferred |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is Techno-derived object AI reached through the main native object vector? -> Yes; Unit/Infantry/Aircraft/Building `vtable+0x5C` bodies are called from the LogicClass loop when registered.` (evidence: `0x0055B608..0x0055B619`; class reports)
- `[RESOLVED] OQ-02 - For UnitClass, does mission dispatch precede locomotor processing? -> Yes; `UnitClass::AI -> FootClass::AI -> TechnoClass::AI_Update -> Mission_Dispatch`, then later FootClass locomotor `Process`.` (evidence: `0x0073647B`, `0x004DA539`, `0x006FA655`, `0x004DA877`)
- `[RESOLVED] OQ-03 - Is current Rust movement globally phased rather than object-AI dispatched? -> Yes; `advance_tick` runs movement before air/special movement, vision, power, combat, and production.` (evidence: `src/sim/world/mod.rs:1534..1617`)
- `[RESOLVED] OQ-04 - Should ore growth/spread move into Techno object AI? -> No; it is a global PerTickUpdate pre-object service.` (evidence: `0x0055B4D7`, `0x0055B4DC`; PerTick ladder)
- `[RESOLVED] OQ-05 - Should superweapon/LightningStorm tick move into Techno object AI? -> No for this boundary; it is a global PerTickUpdate/House-style service, not common Techno object AI.` (evidence: PerTick ladder `0x0055B5C8`; Factory/House reports)
- `[RESOLVED] OQ-06 - Is BuildingClass a good first mobile Techno migration slice? -> No; it is active and important but has broad building-specific pre/post work around Techno AI.` (evidence: `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-07 - Exact order of every UnitClass pre-Foot and post-Foot branch.` (category: `requires-different-system-context`; reason: boundary classification only; next-step-if-pursued: exhaustive `UnitClass::AI` sub-slice)
- `[DEFERRED] OQ-08 - Exact InfantryClass fear/deploy/sequence placement relative to FootClass and TechnoClass.` (category: `requires-different-system-context`; reason: infantry slice is separate; next-step-if-pursued: `InfantryClass::AI` migration contract)
- `[DEFERRED] OQ-09 - Exact AircraftClass AI internal order around mission handlers and FootClass call.` (category: `requires-different-system-context`; reason: aircraft breadth is separate; next-step-if-pursued: `AircraftClass::AI` migration contract)
- `[DEFERRED] OQ-10 - Whether any current global Rust combat helper can remain as a helper called by object AI without byte-order drift.` (category: `bounded-cost-too-high`; reason: requires per-weapon/fire path migration tests; next-step-if-pursued: one vehicle fire-at-target slice)

## 9. Negative Facts / Do Not Do

- Do not migrate ore growth/spread under TechnoClass AI. Active in YR: Yes as global services; evidence: `TiberiumClass::GrowthDriver_AllTypes` and `SpreadDriver_AllTypes` at `0x0055B4D7/0x0055B4DC`, before the main object vector.
- Do not treat current Rust `tick_movement_with_grids` as already equivalent to `FootClass::AI` locomotor processing. Active in YR: Yes for FootClass; evidence: native mission dispatch precedes locomotor `Process` at `0x004DA539 -> 0x006FA655 -> 0x004DA877`, while Rust movement runs before aircraft missions/combat in `advance_tick`.
- Do not keep UnitClass turret rotation as a permanent global post-combat sweep when claiming UnitClass AI parity. Active in YR: Yes; evidence: UnitClass AI order places `Fire_At_Target` then `Facing_Update` inside the UnitClass object AI body; Rust calls `tick_turret_rotation` separately at `src/sim/world/mod.rs:1778`.
- Do not start the Techno migration with BuildingClass breadth. Active in YR: Yes; evidence: `BuildingClass::Update @ 0x0043FB20` has many building-specific phases before and after `TechnoClass::AI_Update`, including damage fires, building animation, delayed fire, gates, and zero-health destruction.
- Do not collapse global `PerTickUpdate` loops into the main Techno object loop. Active in YR: Yes; evidence: PerTick ladder has copied team vector, reverse DiskLaser/RadSite loops, main object vector, conditional anim loop, tactical, factory, and house loops with distinct positions.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| For UnitClass, `TechnoClass::AI_Update` and `Mission_Dispatch` run before FootClass locomotor `Process` in the same object AI tick. | `0x0073647B`, `0x004DA539`, `0x006FA655`, `0x004DA877`; `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md` | Rust runs ground movement as Phase 1 before aircraft missions/combat and without a per-object mission-dispatch owner. | future mobile Techno object-AI shell; `src/sim/world/mod.rs::advance_tick`; `src/sim/movement/*`; mission/miner surfaces | Introduce a per-object mobile Techno AI boundary where mission dispatch/update hooks precede locomotor progress for that object. | A harvester with due mission retry and a cell-crossing opportunity runs the mission retry before the locomotor/per-cell arrival callback in the same object tick. Proposed test: `unit_ai_mission_dispatch_precedes_locomotor_process`. | Do not move movement wholesale without preserving current/pass mission timer ordering. |
| UnitClass post-Foot AI owns turret/fire/facing/harvest/ammo/spawn manager order, not a global combat phase. | `UNITCLASS_GHIDRA_REPORT.md` ordered calls; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` fire/facing/ammo order; SpawnManager report | Rust combat snapshots attackers, then rotates turret in a separate sweep; SpawnManager is not native-owned by Techno AI. | `src/sim/combat/mod.rs`, `src/sim/movement/turret.rs`, future UnitClass AI body, spawn-manager surface | Migrate one bounded vehicle fire slice under UnitClass AI after the shell exists: fire gate, fire side effects, facing update, ammo/reload in native order for that object. | A turreted unit that fires this tick uses the previous facing for the shot, then advances facing for the next frame inside the same UnitClass AI turn. Proposed test: `unit_ai_fire_then_facing_update_then_ammo_reload_order`. | Do not preserve a permanent "combat all attackers then turret all attackers" shape for UnitClass parity. |
| Global `PerTickUpdate` services around the object loop must remain outside Techno object AI. | PerTick ladder `0x0055B4D7..0x0055B6B1`; scheduler report | Rust phases currently mix global services before/after object-like behavior in a different order. | `src/sim/world/mod.rs::advance_tick`, future LogicClass spine | Keep ore, bombs, team/global FX/EMP/tactical/factory/house services as ordered pre/post stages; only object bodies run inside the live object loop. | On a tick with ore spread, a vehicle AI action, factory completion, and House AI, native order remains ore/spread before object AI and factory/house after object AI. Proposed test: `logic_spine_global_services_bracket_techno_object_ai`. | Do not move every current phase under per-object AI just because it touches entities. |

## 11. Remaining Uncertainty

- Full `TechnoClass::AI_Update @ 0x006F9E50` branch order was not drained in this slot; only migration-boundary-critical call sites and prior verified reports were used.
- `InfantryClass::AI` and `AircraftClass::AI` need separate migration contracts before moving fear/deploy/sequence and aircraft attack/RTB/dock internals.
- BuildingClass remains intentionally outside the first mobile slice; a later BuildingClass boundary must preserve its pre-Techno and post-Techno phases.
- Some current Rust helper functions may survive as helpers called by object AI, but their iteration/snapshot behavior must be audited per helper before reuse.

## 12. Stale Docs / Follow-up Wording

None found in tracked `docs/research/` for this boundary beyond stale wording already recorded in scheduler and UnitClass PerCellProcess reports.

## Sources

- Fresh read-only Ghidra assembly context: `0x0055B60E`, `0x0073647B`, `0x004DA539`, `0x006FA655`, `0x004DA877`.
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_AI_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`
- `docs/research/AIRCRAFTCLASS_GHIDRA_REPORT.md`
- `docs/research/SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md`
- `docs/research/GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
- Rust source read-only: `src/sim/world/mod.rs`, `src/sim/movement/*`, `src/sim/aircraft/mod.rs`, `src/sim/combat/mod.rs`, `src/sim/movement/turret.rs`.
