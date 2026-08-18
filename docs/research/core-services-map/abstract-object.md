# Core Service Profile — AbstractClass / ObjectClass (slug: `abstract-object`)

**Role:** Root object identity + lifecycle. The `AbstractClass → ObjectClass` base every world
object inherits; the entity-store + lifecycle chokepoint (register/unregister, limbo/unlimbo,
reveal/conceal). This is the substrate every other service places objects *into* and tears them
*out of*.

**Primary doc:** `docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` (Ghidra-verified
synthesis, 2026-05-30). Supporting Ghidra reports: `OBJECTCLASS_GHIDRA_REPORT.md`,
`ABSTRACTCLASS_GHIDRA_REPORT.md`, `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`,
`CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md`,
`bridges/02-cell-state-layering-zones/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`.

**Authority note:** the primary doc cites addresses and is Ghidra-verified; this profile spot-checked
two edges live this session — `AssignUniqueID 0x00410230` (→ scenario counter) and
`Reveal 0x005F4EC0` (→ cell-map / cell-validation / rules / logicclass) via `decompile_function`.

---

## Purpose

`AbstractClass → ObjectClass` is the object-hierarchy root for every world object (units, buildings,
infantry, aircraft, terrain, anims, bullets, particle systems). In a live YR skirmish it owns the
*identity and physical-presence substrate*: per-instance heap id, world coordinates, health storage,
cell occupancy marking, the limbo↔active lifecycle FSM, active-vector membership (the per-tick
update set), selection bit, and IPersistStream save/load. It does **not** own AI, locomotion,
missions, combat math, or rules data — those sit in subclasses (`TechnoClass`/`FootClass`) or other
services. It is the *chokepoint* where an object becomes (Unlimbo/Reveal) or stops being (Conceal/
UnInit) a live participant in the world.

## Owns (state / globals / structs)

ObjectClass instance fields (ctor `0x005F3900`, offsets verified in primary doc §2a):
- `+0x0C` per-instance unique id (from `ScenarioClass+0x214` monotonic counter).
- `+0x14` AbstractClass status byte (bit `0x2` = IsObject; legacy low-3-bits mostly dormant TS).
- `+0x6C` **Health** (instance HP; MaxHealth is a TYPE field `Type+0xA0` Strength, not per-object).
- `+0x81` **InLimbo** (born = 1) — authoritative lifecycle bit.
- `+0x83` **IsSelected**.
- `+0x8C` **OnBridge** (1 = on bridge; the "+0x23" in older docs is the decompiler's int*-stride view of the same byte).
- `+0x8D` fall/height-settle flag (set by Unlimbo/DropIn, consumed by `ObjectClass::AI`).
- `+0x90` **IsAlive** (UnInit clears).
- `+0x98` **active-vector membership** boolean (gate for the LogicClass live list).
- `+0x9C/A0/A4` **world coordinate triple** (leptons).
- `+0xA8` LineTrail/effect-owner ptr.

Singleton registries the ObjectClass ctor self-appends `this` into (primary doc §2d):
- `0x00A8E360` g_ObjectClass_Array · `0x00B0F720` removal-observer · `0x00B0F670` master Abstract
  registry · `0x00B0F618` g_TagClass_RemoveListeners. Pending-delete: `0x00B0F698`.
- Selection list `g_CurrentObjects` (data `0x00A8ECBC`, count `0x00A8ECC8`).
- The `+0x98`-gated **active list** is the `DynamicVectorClass` embedded in the **LogicClass object**
  (data `+0x04`, count `+0x10`) — owned for membership semantics here, but iterated by LogicClass.

## Key functions & globals (addresses)

| Symbol | Addr | Role |
|---|---|---|
| AbstractClass ctor | `0x00410170` | vtable header + sentinels |
| AbstractClass primary vtable | `0x007E1F50` | 12 slots (QI/AddRef/Release/IsDirty/Load/Save/GetSizeMax/dtor/RTTI) |
| ObjectClass ctor | `0x005F3900` | inits fields, appends to 4 registries |
| ObjectClass primary vtable | `0x007EF060` | 122 slots |
| AssignUniqueID | `0x00410230` | reads `g_ScenarioClass_Instance`, calls counter `0x0068BCB0` (post-inc), writes `+0x0C` (**verified live this session**) |
| GetID | `0x00410220` | returns `+0x0C` |
| GetCoords | `0x005F6690` | returns `+0x9C/A0/A4` |
| ReceiveDamage | `0x005F5390` | applies damage to `+0x6C`; RTTI switch on 6/0xF/0x24 |
| GetHealthRatio | `0x005F5C60` | non-virtual helper (HP/MaxHealth) |
| In_Which_Layer | `0x005F42E0` | Ground=1/Air=4 derived from Z |
| Mark_Put | `0x005F60A0` | sets cell flag `0x40` (bridge → `+0x128`, else `+0x124`) |
| Mark_Remove | `0x005F6120` | clears cell flag `0x40` |
| Mark_Occupation | `0x007441B0` | sets coarse occupation bit `0x20` (gates on height AND bridge-flag) |
| Clear_Occupation | `0x00744210` | clears `0x20` (height alone — asymmetric vs Mark) |
| Reveal | `0x005F4EC0` | limbo→active placement (**verified live this session**) |
| Conceal | `0x005F4D30` | active→limbo teardown (deselect-first) |
| Unlimbo | `0x005F5940` | initial placement (bridge gate → mark → position) |
| UnInit | `0x005F65F0` | teardown: detach-all → Limbo → IsAlive=0 → enqueue pending-delete |
| Select / Deselect | `0x005F4520` / `0x005F44A0` | selection bit `+0x83`, O(n) shift |
| CanBeSelected | `0x005F6C30` | gate (Type+0x230) |
| Load / Save | `0x005F5E80` / `0x005F6250` | IPersistStream round-trip |
| FUN_0055BAA0 | `0x0055BAA0` | active-vector add-once (test/set `+0x98`) |
| FUN_0055BAE0 | `0x0055BAE0` | active-vector compacting remove |
| Detach_From_All_Lists | `0x007258D0` | RTTI-keyed observer-notify (vtable+0x28) |
| Scenario id counter | `0x0068BCB0` | monotonic post-inc behind `ScenarioClass+0x214` |
| name-compare matcher | `0x007C8D20` | case-INSENSITIVE (OR-0x20 fold) — type resolution |

## Tick / render position

Not a per-tick scheduler itself — it is the *substrate the scheduler iterates*. The membership bit
`+0x98` it owns gates the LogicClass live list (`DynamicVectorClass` in the LogicClass object), and
the per-tick consumer `LogicClassPerTickUpdateLiveVector 0x0055AFB0` (inside `Main_Tick 0x0055D360`)
dispatches `ObjectClass::AI` (vtable slot 23, `0x005F3E70`) on each member, **re-reading the live
count each iteration → same-pass** (a unit revealed mid-tick acts the same tick). Lifecycle
transitions (Reveal/Conceal/Unlimbo/UnInit) run from many tick phases (production, combat, movement,
cleanup) and at the cleanup phase the pending-delete queue is flushed. In the Rust spine this maps to
`World::advance_tick`: register/unregister run inside their owning phases; the deferred-delete drain
belongs at the cleanup phase.

## Depends-on (outgoing edges)

- **cell-map** — via `Reveal 0x005F4EC0` calling `MapClass__Get_CellClass_At_Coord(param_2)` to
  resolve the placement cell (verified live), and `Mark_Put 0x005F60A0` / `Mark_Remove 0x005F6120`
  writing cell occupancy flag `0x40` (cell `+0x124`/`+0x128`). Unlimbo reads `cell.Flags(cell+0x140)
  & 0x100/0x200` for the bridge gate. Evidence: primary doc §1.5, §2f; `BRIDGE_OCCUPANCY_OBJECT_LISTS`
  report.
- **cell-validation** — via the `vtable+0x1AC` cell-blocked gate invoked in Reveal
  (`(**(code**)(iVar4+0x1ac))(cell,...)`; abort placement if blocked) and the FootClass passability/
  zone-occupy check in Unlimbo. Evidence: Reveal decompile (live); primary doc §2f Unlimbo step.
- **logicclass** — via `FUN_0055BAA0 0x0055BAA0` (active-vector add-once) called from Reveal and from
  `0x005F3D90`; `FUN_0055BAE0` compacting-remove from Conceal. The membership bit `+0x98` lives on
  ObjectClass but the *list iterated* is owned by LogicClass (`0x0055AFB0`/`Main_Tick`). Bidirectional:
  ObjectClass writes membership, LogicClass reads + dispatches AI. Evidence: primary doc §1.7, C8/C9;
  Reveal decompile (live, `FUN_0055baa0(param_1,0)`).
- **random-scenario** — via `AssignUniqueID 0x00410230` reading `g_ScenarioClass_Instance` and the
  monotonic id counter `FUN_0068BCB0` behind `ScenarioClass+0x214` (verified live); Reveal also gates
  on `g_GameActive` and branches on `g_GameMode` (scenario/game state). Evidence: AssignUniqueID +
  Reveal decompiles (live); primary doc §1.2.
- **rules-class** — via Reveal reading `g_RulesClass_Instance + 0x1863/0x1865` (LineTrail default
  color) when constructing the unit's line trail; MaxHealth/Strength and the `Type+0x234` eligibility
  default are TYPE-class fields read through the type pointer. Evidence: Reveal decompile (live);
  primary doc §1.4, Slice 7.
- **techno-foot** — via the vtable dispatch points ObjectClass invokes on its subclass: `+0x88`
  GetType, `+0x1B4` commit-position, `+0x1AC` cell-blocked, `+0x2C` RTTI; Unlimbo's FootClass-only
  zone-occupy/passability branch and UnInit's `EMPPassengers(0)` Foot branch. ObjectClass is the base;
  Techno/Foot override these slots. Evidence: primary doc §1, §2b; Reveal/Unlimbo/UnInit steps.
- **drawing-helpers** (render-pass, not sim) — Reveal calls `DisplayClass__Submit_Object`,
  `AlphaShapeClass__Constructor`, `TacticalClass__CoordsToClient2`/`DirtyScreenRect` to register the
  newly-revealed object for drawing. Edge crosses out of the sim layer; in Rust this lives above the
  sim boundary. Evidence: Reveal decompile (live).
- **lookup-tables** (weak) — RTTI dispatch and WhatAmI use static type discriminants; name→type uses
  the case-insensitive matcher `0x007C8D20`. Evidence: primary doc §1.10, C13.

## Used-by (incoming edges)

- **logicclass** — iterates the `+0x98`-gated live list and dispatches `ObjectClass::AI` (slot 23)
  every tick (`0x0055AFB0` in `Main_Tick`). The scheduler depends on ObjectClass membership semantics
  and the AI vtable slot. Evidence: primary doc §1.7, C9.
- **techno-foot** — every TechnoClass/FootClass IS an ObjectClass; they call base identity/lifecycle
  (GetCoords, Mark_Put/Remove, Reveal/Conceal/Unlimbo/UnInit, ReceiveDamage) and override its vtable
  slots. Hardest incoming dependency. Evidence: primary doc Part A; TECHNOCLASS reports.
- **cell-map** — CellClass object-lists hold ObjectClass pointers; Mark/occupancy edges are mutual
  (ObjectClass writes cell flags, CellClass stores the object in FirstObject/AltObject lists).
  Evidence: `CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS` report.
- **damage-helpers** — the warhead/armor kernel terminates in `ObjectClass::ReceiveDamage 0x005F5390`
  (slot 91), which writes Health `+0x6C` and triggers Destroy/RTTI dispatch. Evidence: primary doc
  §2b; `reference_damage_kernel_constants`.
- **factory-house** — production unlimbos/reveals produced objects into the world and queries them by
  owner; HouseClass owned-counts update on the lifecycle transitions ObjectClass drives. Evidence:
  primary doc §4 (`by_owner`), §6 (change_owner).
- **mission-radio** — MissionClass/RadioClass operate on objects whose presence/limbo state and
  detach-from-all-lists (radio link teardown) are owned here (UnInit → Detach_From_All_Lists
  `0x007258D0`). Evidence: primary doc §2c, C7/C10; `SLICE6_DEFERRED_DELETE` report.
- **gadget-dialog / drawing-helpers** (render/UI) — selection (`+0x83`, `g_CurrentObjects`) and
  display-submit feed the in-game UI and renderer; these read ObjectClass identity/coords/selection.
  Evidence: primary doc §1.8, C12; Reveal display-submit path.

## Open / unverified edges

- **C11 bridge Mark/Clear asymmetry** — `Mark_Occupation 0x007441B0` gates the `0x20` bit on height
  AND `cell+0x140&0x100`; `Clear_Occupation 0x00744210` clears on height alone. A bit can strand only
  if a bridge is destroyed under an elevated unit; the destroy-path occupancy cleanup was not located.
  **Needs in-game test** (cell-map edge correctness). DRIFT/UNCHECKED.
- **C8 `FUN_005519B0` arity** — the inner DynamicVector add's true parameter count (is the
  decompiler's `param_3` real?) and whether any *other* caller passes a nonzero sort flag was not
  re-confirmed (MCP dropped mid-session). Affects the logicclass active-vector edge. MEDIUM.
- **Sub-object roles** (`+0x3C`/`+0x50` via `FUN_00405BE0`, `+0x24/28/2C` cell-list Next/Prev, `+0x34`
  height candidate) — present-VERIFIED, role UNVERIFIED. Could be additional cell-map/render edges.
- **`BridgeHeight` magnitude** (`DAT_00AC13BC`) — value not read; minor, not blocking.
- **IsDirty live caller** — method exists; no proven runtime caller (save/load edge). UNVERIFIED.
