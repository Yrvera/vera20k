---
name: BuildingClass Unlimbo and Placement Chain
description: Full end-to-end placement flow — cell marking, zone rebuild, house stats, sidebar cameos, AI notification.
type: reference
---

# BuildingClass Unlimbo + Placement — Ghidra Research Report

**Address:** `0x00440580` (body `0x00440580 – 0x004415E8`, 4200 bytes)
**Vtable slot:** 0xD8 (TechnoClass::Unlimbo override — vtable base `0x007E3EBC`; slot 0xD8 pointer at `0x007E3F94` = `0x00440580`) <!-- corrected 2026-05-28: was "vtable at 0x007E3F94" which implied 0x007E3F94 was the vtable base; binary shows vtable base is 0x007E3EBC = 0x007E3F94 − 0xD8, verified via read_memory@0x007E3EBC and @0x007E3F94 — MISLEADING/RTTI_LABEL_DRIFT -->
**Signature:** `BOOL __thiscall BuildingClass::Unlimbo(BuildingClass *this, CoordStruct *coord /* EBX: DirType facing packed in low byte */)`
**Confidence:** HIGH (decompiled in one pass, cross-checked with master report v2,
BIB report, MCV deploy report, vtable full-300 map, and Limbo-symmetric OnDestroyed).
**Active in YR:** Yes — primary placement entry point.

---

## 1. Overview + Invocation Context

Unlimbo is invoked with the building's **target coordinate** in `param_2` and the
**DirType facing** packed in EBX (observed via the `sStack_38/sStack_36` setup). It
is the single entry point that transitions a BuildingClass from "limbo" (not on
the map) to "on the map".

### Confirmed callers (direct + via vtable[0xD8])

| Site | Function | Purpose |
|------|----------|---------|
| `0x004FB2C9` | `HouseClass::Place_Production` (`0x004FB0E0`) | Sidebar cameo "place-down" confirmation — called after `FactoryClass::IsComplete()` and after the player clicks a valid cell. Calls `vtable[0xD8]` = `Unlimbo`. |
| `0x0073EFC0` | `BuildingClass::DeployHelper` (via MCV deploy) | MCV→ConYard path; calls UnitClass::Deploy which constructs BuildingClass + Unlimbo. (See `MCV_DEPLOY_GHIDRA_REPORT.md`.) |
| `BuildingClass::ExtendWallInDirection` (`0x00452DC0`) | Recursive wall auto-extension | Unlimbos newly-created wall segments via `vtable[0xD8]` on sibling fence-posts. |
| `HouseClass::Place_ProductionInEditor` / map trigger pre-placement | Map-editor / scenario placement | Identical path. |

### Caller **NOT** in the chain

`BuildingClass::Save/Load` does **NOT** go through Unlimbo. Post-load rehydration
uses the swap-map pointer-fixup mechanism, and runtime-cached fields
(`+0x614` LightSourceClass*, +0x644 production anim, etc.) are rebuilt lazily
on the first post-load tick. This is a significant correctness constraint:
**a Rust impl of Unlimbo should NOT be reused for save/load restoration**;
post-load needs its own rehydrate-from-snapshot path.

### High-level flow (top-level branches)

The function has **three main branches** selected by `Type+0x1571` (a wall/extension-only
flag, seen in Master v2 table) and `Type+0xE88` (upgrade-parent name — the
`PowersUpBuilding=` target):

1. **Wall/fence-extension branch** (`Type+0x1571 != 0`): look up the existing
   wall at this cell, verify ownership + type-name match, attach as sibling wall
   segment (no independent placement). Calls `MapClass::RevealAroundCell` twice
   (the only shroud-reveal sweep in Unlimbo). Returns 1.
2. **Upgrade attach branch** (`Type+0xE88 != 0`, i.e. `PowersUpBuilding=X`):
   find the parent building via `FUN_004AA290` (= `CellClass::FindBlockingObject`
   on the foundation cell), validate owner + name match, install into parent's
   `Upgrades[0..2]` slot at `+0x5E8/+0x5EC/+0x5F0` <!-- corrected 2026-05-28: was +0x5EC/+0x5F0/+0x5F4; binary shows piVar8[0x17a + level] with int* → level-0 offset = 0x17a*4 = 0x5E8, level-1 = 0x5EC, level-2 = 0x5F0; verified via decompile_function@0x00440580 — WRONG/STRUCT_FAMILY_CASCADE -->, increment `UpgradeLevel`
   (`+0x702`), call `BuildingClass::AddUpgrade` 1..3 times (full heal + anim
   creation), call `HouseClass::AI_ResumeProduction`, set `Owner+0x1FC = 1`
   (queue-dirty flag), and if `Type+0x1763` ("ReloadableBalance") call
   `FUN_00509130`. Returns 1 **without** going through the main placement body.
3. **Normal placement branch** (default): the rest of §§2–13 below.

---

## 2. Cell Marking and Occupancy

### 2.1 Occupant-count increment loop (the actual cell mark)

The authoritative cell-mark loop in Unlimbo walks a rectangular
`(FoundationWidth + 2) × (FoundationHeight + 2)` bounding box, starting at
`(origin_x - 1, origin_y - 1)`, and **increments `cell+0x122` (byte) by +1**
on every cell:

```
w = BuildingTypeClass::GetFoundationWidth(Type)   // Type+0xEF0 → g_FoundationWidthTable
h = BuildingTypeClass::GetFoundationHeight(Type, 0)
origin_x = (coord.X + sign_adjust) >> 8
origin_y = (coord.Y + sign_adjust) >> 8
for dy in 0..h+2:
    for dx in 0..w+2:
        cell = MapClass::Get_CellClass(origin_x - 1 + dx, origin_y - 1 + dy)
        cell.byte_0x122 += 1    // occupant/coverage counter
```

Key facts (verified against OnDestroyed's symmetric `-= 1` loop at `0x00445880`):

- **This is a BYTE counter, not a bitflag.** It permits multiple buildings to
  overlap-touch adjacent cells (diagonal bib/edge overlap from neighbors).
- **The loop covers `(W+2)*(H+2)`, not the foundation-only cells.** The outer
  ring is for placement-collision testing of neighboring placements.
- **No edge-case branch for ramps / bridges / water cells.** Water-cell buildings
  (shipyards) use the same increment. Bridge-cell buildings (bridge repair hut)
  use the same increment.
- The increment is **unconditional** — it does not consult `cell+0x44` (overlay
  type), `cell+0x100` (passability flags), or any ramp/bridge state.

### 2.2 Cell flag bits: NOT touched by Unlimbo

Unlimbo does **NOT** write `cell+0x40` (OverlayType index), `cell+0x50`
(OwnerHouseIndex), `cell+0x140` (PassabilityFlags), or
`cell->OverlayTypeIndex = 0xEF` (the "building here" sentinel). Those writes
happen in a **separate function**: `BuildingClass::Place_OccupyMap` at
`0x00441F60`, which is called from the **first tick of `Update`** (`0x0043FB20`
→ `0x004400E5`) and from `ReceiveDamage` (`0x00442230` → `0x004426A2`).

**Consequence (fidelity-critical):** at the exact moment Unlimbo returns,
the cells are "occupant-counted" but not yet "overlay-marked". Code that
tests `cell->OverlayTypeIndex == 0xEF` to detect "building here" will miss
the building between Unlimbo and the first tick. Anything that runs on
that same tick (e.g. same-tick selection, same-tick attack orders) must
use either the occupant-counter (`+0x122`) or a radio-lookup via
`Look_up_building_in_cell` (`0x0047C520`).

### 2.3 Passability-flag OR-mask sweep (for pathfinder)

After the occupant-count loop, Unlimbo calls `FUN_00455F10` which sweeps a
bigger box: `(W + 2*Rules+0x1460) × (H + 2*Rules+0x1460)` cells. For each cell
it sets `cell+0xDC |= (1 << owner_house_index)`. This is the
**"house-owned region" mask** used by AI pathfinding, base-threat heatmap,
and house-region auto-flag. `Rules+0x1460` is `BaseBias` or equivalent
(further research: §15 open questions).

FUN_00455F10 *also* updates per-house bounding-box tracking:
`house+0x5754..+0x5760` (x_min, y_min, width, height). This is the
"house base rectangle" used for `Rally=` / `DefaultBuildingPlace` logic.

### 2.4 Bridge-adjacent cell passability cleanup

If `Type` equals one of `Rules+0x86C` / `+0x870` / `+0x874` / `+0x878` /
`+0x87C` (the five bridge-repair-hut types), Unlimbo calls
`CellClass::PostDestructionWallCleanup` on **four** orthogonal neighbors of
the origin cell. This is repurposed here — not destruction-related — to
recompute cell attributes on bridge-adjacent cells. Each bridge type has a
different neighbor pattern (±1 on X axis for `+0x86C`/`+0x874`, ±1 on Y axis
for `+0x870`/`+0x878`, and 4-direction for `+0x87C` ConYard variant).

---

## 3. Wall Orientation Computation (`+0x618` / `param_1[0x186]`)

For building types with `Type+0x16BF != 0` (`FirestormWall=yes`-style wall
flag), Unlimbo writes `this+0x618`:

```
eax = (EBX_low_byte << 8) >> 12   // DirType >> 4, signed
if ((eax + 1) & 6) == 4:
    this+0x618 = 8
else:
    this+0x618 = 0xC

// And unconditionally (for Type+0x16BE "IsWall"):
if blocking_object_is_ally_wall_of_same_type:
    this+0x618 = 0     // connect "inward"
```

So `+0x618` is a **wall-sprite-facing selector** with values 0, 4, 8, 0xC
(4 values = 2 bits of facing, used to pick which of 4 wall-cap sprites to
draw). The value is also inspected by `BuildingClass::Update` to pick the
correct wall-frame when drawing. Master v2 §2 lists "0x0/0x4/0x8/0xC" as
the valid domain — confirmed.

`BuildingClass::ExtendWallInDirection` / `RecalculateWallConnections` run
later (after `TechnoClass::Unlimbo` succeeds) to propagate facing to
newly-linked sibling wall segments via `AdjustWallConnections`.

---

## 4. Bib Creation — NONE

Unlimbo does **NOT** call `ClearBibArea` (`0x00449540`) and does NOT create
any bib overlay. This was verified directly:

- `ClearBibArea` is gated on `Type+0x16BD` (`WeaponsFactory=yes`), NOT on
  `Bib=yes` / `HasBib`. It is called from `ExitObject`-style vehicle-emit
  paths to scatter units parked on the exit cell before a new vehicle emerges.
  It is NOT symmetric to Unlimbo.
- `Bib=yes` / `Type+0x1570 (HasBib)` does not extend the foundation cell list
  and does not add bib cells to the occupancy footprint (per
  `BIB_SYSTEM_GHIDRA_REPORT.md` §2.3). The bib is a **visual-only** artifact
  that renders from the building's SHP on a separate row; occupancy does not
  include it.
- The *live* effect of `HasBib` is in `UnitClass::Can_Enter_Cell` (relaxes
  entry-block on one edge) — per BIB report §2.5.

**Conclusion:** do not implement "on Unlimbo, also mark bib cells". The
original engine does not.

---

## 5. Zone Rebuild Trigger — NONE in Unlimbo body

Unlimbo does **NOT** call `MapClass::UpdateBridgeZonesHelper` (`0x56C510`)
or `MapClass::AssignOrphanedCellZone`. Those are called from
`BuildingClass::Place_OccupyMap` (`0x00441F60`), which runs on the
**first Update tick after placement**, not inside Unlimbo itself. So:

- **Pathfinding zones are NOT rebuilt at Unlimbo time.**
- **AI sub-zone / bridge-zone recompute is deferred to the next tick.**

This matches the master-v2 observation that Unlimbo is "place on map"
but the overlay/zone effects are deferred by one frame. For a Rust impl,
this means the zone-rebuild call should be scheduled in the
same-tick post-placement pipeline, NOT inlined into Unlimbo.

---

## 6. House Stats Updates

### 6.1 Inside `TechnoClass::Unlimbo` (called early in Unlimbo)

`TechnoClass::Unlimbo` at `0x006F6CA0` calls `HouseClass::Added_To_Game`
(`0x00502A80`), which is the **authoritative house-stat bump**. Behavior
by RTTI type (Building = RTTI 6):

| Field | Offset | Condition | Effect |
|-------|--------|-----------|--------|
| House+0x158 | "BuildingsBuilt" or similar | `Type+0x5EC` (CanBeOccupied) — checked **before the RTTI switch**, applies to ALL RTTI types <!-- corrected 2026-05-28: was implied RTTI==6-only; binary shows this runs unconditionally before switch(RTTI), verified via decompile_function@0x00502A80 — WRONG/OPERATOR_OR_ORDER_DRIFT --> | `+= 1` |
| House+0x15C | "GarrisonableBuilt" | `Type+0x5ED` — checked **before the RTTI switch**, applies to ALL RTTI types <!-- corrected 2026-05-28: same as above; pre-switch unconditional check — WRONG/OPERATOR_OR_ORDER_DRIFT --> | `+= 1` |
| House+0x160 | "Factory building count" | RTTI==6 && `Type+0x16BD` && `!Type+0xCCE` (not naval) | `+= 1` |
| House+0x310 | Storage cap sum | RTTI==6 | `+= Type+0x800` (`Storage=`) |
| House+0x1608C (BuildingTypesOwned sum) | | (implicit via IndexClass::Increment) | See §6.2 |

Also: `BuildingClass::GetPowerDrain(this)` is called and fed to
`HouseClass::Update_Power_And_EVA` (`0x005018C0`) — this is **when** the
power/EVA state is first updated after placement.

Storage is copied in via `StorageClass::AddFrom` (merges unit-level
tiberium storage into house-level storage bank — matters for ore refineries
with pre-filled storage from captured buildings).

### 6.2 Tech-tree increment (`IndexClass::Increment` at `0x0049FA00`)

`Added_To_Game` calls `IndexClass::Increment(buildings_owned_index, Type_index)`
for each "counts-toward-tech-tree" building. This is the **sidebar-cameo
unlock trigger** — see §7. `IndexClass` is a sparse-array counter backed by
`+0x160A8/+0x160AC/+0x160B0` depending on factory kind:

| Counter | Offset in House | Populated from |
|---------|-----------------|----------------|
| `+0x160A8` | InfantryType factories (RTTI 0xF) | non-`Type+0xD96` (not ServiceDepot) |
| `+0x160AC` | UnitType factories (RTTI 1) | non-`Type+0xD96` |
| `+0x160B0` | Aircraft/NavalYard/other | `Type+0xD96 != 0` (ServiceDepot split) |

### 6.3 Back in Unlimbo — trait-list registration (the "docks / factories / labs" arrays)

After `TechnoClass::Unlimbo` returns, Unlimbo does a **long sequence of 11
trait-list insertions** into the HouseClass, gated by different `Type+0x16xx`
booleans. Each insertion uses the same DynamicVector-push pattern:

```c
if (Type+0x16A9 != 0):   // PowerBuilding / AirTrafficControl etc.
    house->vec_0x80.push(this)
if (Type+0x16AD != 0):   // Some sensor/lab
    house->vec_0x98.push(this)
if (Type+0x16AE != 0 || Type+0x16AF != 0):  // Radar OR SpySat
    house->vec_0xB0.push(this)
if (Type+0x16AB != 0):   // SuperWeapon primary
    house->vec_0xC8.push(this)
if (Type+0x157B != 0 && Type+0x634 >= 0):  // CanBeOccupied && has garrison weapon
    house->vec_0xE0.push(this)
if (Type+0x16AC != 0):   // Some other cap
    house->vec_0xF8.push(this)
if (Type+0x16B0 != 0):   // GapGenerator
    house->vec_0x110.push(this)
if (Type+0x170C > 0):    // NumSecondaryFactories
    house->vec_0x128.push(this)
if (Type+0x16CD != 0):   // BonusBuilding (RecalcBonuses trigger)
    house->vec_0x140.push(this)
    HouseClass::RecalcBonuses(house)
// Unconditional (all buildings):
house->vec_0x68.push(this)    // "all buildings" master vector
```

Each DynamicVector uses the standard 4-word layout: `[vtable, data_ptr, count,
reserved, capacity, step]`. The push-with-grow protocol checks count ≥ capacity
and grows by `step` — if grow fails, the push is **silently dropped**. (Fidelity
hazard: a Rust Vec<_> grows until OOM; the original's cap+step limits the list.)

Additional gates earlier in Unlimbo:

- `Rules+0x8B0..+0x8BC` (a RulesClass pointer-array of "special building types")
  causes an insertion into `house+0x54` (the "tech-building" list) if the
  building's type matches one of those entries. The loop walks `Rules+0x8B0`
  (base ptr) for `Rules+0x8BC` (count) entries.
- **CloakGenerator**: `Type+0x16C7 != 0` sets `Owner+0x56F8 = 1`. This is the
  "has-cloakgen-active" flag that gates the cloak-propagation sweep on the
  next tick. Matching decrement is in OnDestroyed.

### 6.4 "Base is dirty" flags

- `Owner+0x1FC = 1` — "base/queue recalc needed" (set twice in Unlimbo)
- `Owner+0x56F8 = 1` — CloakGen count dirty
- `this+0x3D3 = 0` (IsVisibleToPlayer init — direct byte offset, verified via decompile_function@0x00440580)
- `this+0x544 = 0` (AnimStateMachine reset — `param_1[0x151]` with int*, byte offset = 0x151×4 = 0x544) <!-- corrected 2026-05-28: was "this+0x151"; binary uses param_1[0x151] where param_1 is int*, so byte offset is 0x544 not 0x151 — WRONG/PARAM1_TYPE_MISREAD -->

---

## 7. Sidebar Cameo Propagation

The sidebar cameo-unlock chain is **indirect** and **multi-stage**:

1. In Unlimbo → `TechnoClass::Unlimbo` → `HouseClass::Added_To_Game` →
   `IndexClass::Increment(BuildingsOwned, Type->Index)`.
2. `Owner+0x1FC = 1` is set twice during Unlimbo (once at the start of the
   main placement body, once later after the trait-vector inserts).
3. The `Owner+0x1FC` flag is read by `HouseClass::AI_ManageProduction` /
   `HouseClass::Update` on the next tick. That tick's base-recalc walks the
   new `BuildingsOwned` counts, queries each BuildingType's `Prerequisite=`
   string, and flips cameo-unlock bits on each tech-type (`TechnoTypeClass`
   has a per-house "is-buildable" cache).
4. When the player is human (`HouseClass::IsPlayerControl`), the sidebar
   reads the is-buildable cache at its next refresh (sidebar has its own
   "dirty" flag set by `Owner+0x1FC`).

**Unlimbo itself does NOT touch the sidebar directly.** There is no
"cameo-unlock" call in Unlimbo's body. Every sidebar update is deferred
via the `Owner+0x1FC` dirty flag and re-derived from `IndexClass` counts.

This is the right pattern for Rust: Unlimbo writes house counters; a
next-tick `HouseClass::refresh_prerequisites()` recomputes what's buildable
from those counters; the sidebar reads the result.

---

## 8. AI / Threat Table Notification

Unlimbo calls `FUN_0042F260(this)` **only if the building's owner is not
player-controlled** (AI house). This function walks the house's base-plan
node list (at `house+0x8..+0x14`) looking for a slot matching this type's
`+0xDF8` (Type->Index) at this cell, and if found sets
`slot+0x8 (byte) = 1; slot+0xC (int) = 0` — marking "this plan entry has
been built, reset retry counter".

So `FUN_0042F260` = "HouseClass::AI::MarkBasePlanSlotFilled".

AI-threat / AI-zone / AI-build-queue notifications **beyond this** are
deferred to the next AI tick, triggered by the `Owner+0x1FC` flag plus
the per-vector insertions in §6.3.

---

## 9. Shroud Reveal

Shroud reveal is **conditional** on two branches:

### 9.1 In the wall/fence-extend branch (§1 branch 1)

Called twice with fixed arguments (radius = `Type+0x5E8` = `Sight=`):

```
MapClass::RevealAroundCell(coord, Type+0x5E8 /*radius*/, Owner,
                           0, 0, 0, 1 /*fogMode*/, 0 /*alsoMapSurround*/)
MapClass::RevealAroundCell(coord, Type+0x5E8, Owner,
                           0, 0, 0, 1, 1 /*alsoMapSurround=true*/)
```

### 9.2 In the normal placement branch

**No explicit `RevealAroundCell` call.** Shroud reveal in the normal path
is performed by `TechnoClass::Unlimbo` itself, earlier, via vtable slot
`+0x488` (`ObjectClass::Reveal`-equivalent) on `this`. That call reveals
the cells around the building at `Type+0x5E8` radius.

### 9.3 Fog-of-war branch (TS-legacy, off by default in YR)

Unlimbo has a whole branch gated by `g_ScenarioClass+0x0 & 0x1000`
(`SpecialFlags.FogOfWar` bit). When this bit is set, Unlimbo walks the
foundation cell list (vtable[0x90] — the ACTUAL foundation cell list,
not `(w+2)*(h+2)` box) and for each cell checks `cell+0x140 & 0x400000`
(fog-covered bit). If ALL cells are fog-covered, it calls `FUN_00457AA0`
which **creates a `BuildingClass*` clone-snapshot** (`CreateFoggedSnapshot`
at `0x004D0EF0`) — this is the ghost-building that shows on the minimap
after the player un-scouts.

**TS-LEGACY WARNING (per CLAUDE.md §"Tiberian Sun legacy"):** `SpecialFlags
& 0x1000` is the FogOfWar flag, which **defaults to FALSE in YR**. Fogged
snapshots are TS-era "explored-but-not-visible" ghost buildings. Do NOT
implement this branch by default; only gate it behind an opt-in fog setting
if/when FogOfWar is enabled.

---

## 10. Initial HP / State Assignment

Unlimbo itself does **NOT** set `this->Strength` (`+0x158` — current HP).
HP is **already set** before Unlimbo:

- In the sidebar-placement path: `HouseClass::Place_Production` → the object
  is constructed with Strength = Type->Strength (default MaxHP) by the
  constructor chain at `0x0043B740` (BuildingClass ctor) calling the
  TechnoClass ctor which copies `Type+0xA0` (MaxHP) into `+0x158`.
- In the MCV→ConYard deploy path: `UnitClass::Deploy` explicitly carries
  HP across: the old MCV's `+0x158` is converted proportionally to ConYard
  Strength (see `MCV_DEPLOY_GHIDRA_REPORT.md` §2). Unlimbo does not re-assign.
- In the upgrade-attach branch (§1 branch 2): `BuildingClass::AddUpgrade`
  (`0x00451400`) sets `parent->Health = parent->Type+0xA0` — **full heal** on
  upgrade attach. This is the famous "Attack Dog + Iron Curtain then Mix Tank
  revival"-scale bug that came from upgrade-heal.

The only HP-adjacent write in Unlimbo's body is `this+0x151 = 0` (animation
frame-state reset — not HP).

---

## 11. Special Cases

### 11.1 MCV / Construction Yard captures

`Rules+0x87C` is the ConstructionYard type pointer. Unlimbo has a
ConYard-specific **wall-cleanup** block: if `Type == Rules+0x87C`, it walks
`g_DirectionOffsets` (the 8-compass direction table) in 2-step increments
(4 cardinal directions) and calls `CellClass::PostDestructionWallCleanup`
on each neighbor. This scrubs ConYard-adjacent walls.

### 11.2 Construction-yard spawning-survivor (captured buildings)

When a building is captured via engineer, `Unlimbo` is NOT re-invoked on
the captured building — ownership is transferred in-place via
`BuildingClass::ChangeOwnership` (`0x0044F100`, per `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`).
**Survivor-placement** (`SpawnSurvivors` at `0x00442D90`) is on the *destruction*
path, not the placement path. Master v2 §17 mentions survivor-placement — it's
not in Unlimbo.

### 11.3 Naval buildings

No special-case code for water cells in Unlimbo. The occupant-count loop
increments `+0x122` regardless of cell terrain type. Naval-building
placement validation is done **upstream** (in the placement-validator vtable
slot `+0x2BC` — `CanEnter_Cell` equivalent, called from the player's ghost-
placement UI and from `HouseClass::Place_Production`). Once Unlimbo runs,
water cells are marked just like land cells.

### 11.4 Laser fence / firestorm wall

`Type+0x16BE` (IsWall) + `Type+0x16BF` (FirestormWall / LaserFence-mode)
gates the auto-wall-connection logic (`ExtendWallInDirection` 4× + `RecalculateWallConnections`)
at the end of the main body. This is ONLY run when NOT in map-editor
and `+0x81` flag (InConstruction) is clear.

### 11.5 Slave miner / secondary production

`Type+0x170C` ("NumSecondaryFactories" — slave-miner spawn count) gates
insertion into `house+0x128` vec. The actual slave spawn happens elsewhere
(per `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`).

### 11.6 Spotlight / Ambient-light source

- **Spotlight** (`Type+0x154B` = `Spotlight=yes`): allocates a
  `BuildingLightClass` (0xE8 bytes) at `this+0x600` via `BuildingLightClass::Constructor`
  (`0x00435820`).
- **Ambient light** (`Type+0xE34 != 0` = has `LightVisibility=`): allocates
  a `LightSourceClass` (0x4C bytes) at `this+0x614` via
  `LightSourceClass::Constructor` (`0x00554760`). Five color params copied from
  `Type+0xE30/+0xE34/+0xE38/+0xE3C/+0xE40` (LightRed/Green/Blue/Intensity/Visibility).
- These are **render-layer runtime caches**, not persisted across save/load
  (per `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` `+0x614` note).

### 11.7 Particle-system ambient (smokestacks)

`Type+0x764 != 0` (`DamageParticleSystems=X,Y,Z` style) allocates a
`ParticleSystemClass` (0x100 bytes) at `this+0x30C` at the cell offset
`(Type+0x768, Type+0x76C, Type+0x770)` relative to `this+0x9C..+0xA4`
(building coord). This is **always-on ambient** (smokestack smoke);
damage-conditional particle systems are separately allocated by `SetDamagedState`.

---

## 12. Sound Cues

Unlimbo itself **does NOT play any sound**. The sound chain lives in the caller:

- `HouseClass::Place_Production` plays the `ConstructionComplete` voc
  (`VocClass::PlayAtPos(0x3F800000, 0)` for player-side) **only if**
  `Unlimbo == 1 (success)` and `this == g_PlayerPtr`. This is the
  "thud" cue when a building lands.
- EVA "Construction complete" voice is played **before** Unlimbo (when
  the factory completes, not when it lands on the map).

No separate place-down sound inside Unlimbo. A Rust impl should play the
"place" voc in the caller (placement-event handler), not in Unlimbo.

---

## 13. Limbo Symmetry (Fields Cleared vs Set)

The Limbo-equivalent function is `BuildingClass::OnDestroyed` at `0x00445880`
(vtable slot 0xD4, labeled `BuildingClass::Limbo (OnDestroyed)` in master
docs). It overrides `TechnoClass::Limbo`. Symmetric operations:

| Operation | Unlimbo sets/inserts | OnDestroyed clears/removes |
|-----------|----------------------|----------------------------|
| Cell occupant-counter `+0x122` | +1 per cell in (W+2)×(H+2) box | −1 per cell (same box) |
| Wall orientation `+0x618` | write 0/4/8/0xC | not explicitly cleared — but wall is free'd |
| `Owner+0x538C` (OrePurifier count) | (by `Added_To_Game`) | `-=1` if `Type+0x16CC` |
| `Owner+0x2D4` (Helipad count) | (in `Added_To_Game`) | `-=Type+0x1780` if `Type+0x16CB` |
| `Owner+0x164` (counter A) | `+=Type+0x1564` in `Added_To_Game` | `-=Type+0x1564` |
| `Owner+0x168` (counter B) | `+=Type+0x1568` | `-=Type+0x1568` |
| `Owner+0x56F8` (HasCloakGen) | `= 1` if `Type+0x16C7` | (not cleared — recounted on next tick) |
| `Owner+0x1FC` (dirty flag) | `= 1` | `= 1` |
| Trait-list vectors (`+0x68/0x80/0x98/0xB0/0xC8/0xE0/0xF8/0x110/0x128/0x140`) | push `this` | NOT removed in OnDestroyed directly; `HouseClass::Recount(this)` at `OnDestroyed` end walks and rebuilds |
| `BuildingLightClass*` (`+0x600`) | `new BuildingLightClass` if `Type+0x154B` | `vtable[0xF8]` release → null |
| `LightSourceClass*` (`+0x614`) | `new LightSourceClass` if `Type+0xE34 != 0` | part of the 8-slot anim-release loop at OnDestroyed start |
| 8 anim-slots `+0x5C8..+0x5E4` | `CreateProductionAnim(0)` fills slot | loop releases all 8 via `vtable[0xF8]` |
| Wall connections | `ExtendWallInDirection(4 dirs) + RecalculateWallConnections` | `ConnectWalls(0)` → propagate disconnect |
| Bridge neighbor cleanup | `PostDestructionWallCleanup` 4× if bridge-type | same 4× pattern |
| `HouseClass::Added_To_Game` (counters bump) | YES | `HouseClass::Recount` (re-derive from scratch) — NOT symmetric, it re-scans |
| ParticleSystemClass (`+0x30C`) | `new ParticleSystemClass` if `Type+0x764 != 0` | freed in TechnoClass::Limbo_Helper |
| `Owner+0x26C / +0x246` (DiscoveredBy) | set in `DiscoveredBy` vtable call | not cleared |

**Asymmetry hazard:** `HouseClass::Added_To_Game` is an ADD-style function, but
`HouseClass::Recount` on destruction is a RE-SCAN that iterates `Owner->Buildings`
and re-derives counters. This means if any counter is bumped during gameplay
outside of Unlimbo, it gets silently clobbered on next destruction.

---

## 14. Magic Constants and Edge Cases

| Constant | Meaning | Used in |
|----------|---------|---------|
| `0x7FFF, 0x7FFF` | Foundation cell-list sentinel | vtable[0x90] cell iterators |
| `0xEF` | Cell OverlayType index for "building here" | `Place_OccupyMap` (NOT Unlimbo) |
| `0x122` | Cell occupant-counter byte offset | Unlimbo inc, OnDestroyed dec |
| `(W+2)*(H+2)` | Occupant-count box | inclusive outer ring |
| `Type+0x5E8` | Radius of shroud reveal | `Sight=` key |
| `Type+0xEF0` | Foundation enum (0..N) → g_FoundationWidthTable / g_FoundationHeightTable | lookup |
| `Type+0xED4` | Foundation exit-cell table ptr | bib/exit path |
| `Rules+0x86C..+0x87C` | Bridge repair hut + ConYard type pointers | special-case wall cleanup |
| `Rules+0x8B0 / +0x8BC` | "Special-building types" array + count | tech-list registration |
| `Rules+0x1460` | "BaseBias" / house-region expansion | passability OR-mask |
| `SpecialFlags & 0x1000` | FogOfWar — **DEFAULT OFF IN YR**, TS-legacy | fogged-snapshot branch |
| `Scenario+0 & 0x1000` | same flag via ScenarioClass | gated check |
| `g_MapEditorMode` | In-editor skip | walls, lights, sounds |
| `g_GameMode != 0` | Multiplayer / skirmish | discovery state |

---

## 15. Open Questions

1. **`Rules+0x1460` exact meaning** — used as passability OR-mask radius. Is
   it `BaseBias`, `NeighborBuffer`, or something else? Field-mapping work needed.
2. **The 11 trait-list vectors (`+0x80/+0x98/+0xB0/+0xC8/+0xE0/+0xF8/+0x110/+0x128/+0x140`)** —
   each corresponds to one `Type+0x16xx` bool; names come from HouseClass internals.
   Master v2 §2 has partial labels (`CanBeOccupied`, `CanOccupyFire`, gap-gen) but
   the full table-cross-reference has not been written down.
3. **`+0x186` wall-orientation domain** — the four values 0/4/8/0xC. How do
   they map to N/E/S/W wall-cap sprites exactly? Need to correlate with
   `DrawBody` vtable slot.
4. **Fogged-snapshot clone lifecycle** — when is the clone freed? Does it
   persist across save/load? (`FogOfWar` defaults off in YR, so low priority,
   but if user enables it via `SpecialFlags`, correctness matters.)
5. **FUN_004FFA50** — the factory-counter dispatch (`case 1/2/3/6/7/0x28/0xF/0x10/0x28`)
   writes to `Owner+0x5378..+0x5388`. Each case maps to an RTTI kind; naming is
   half-done. Master v2 §10 (Docking) has the mapping for some.

---

## Sources

- **Decompiled in this investigation:**
  - `BuildingClass::Unlimbo` `0x00440580` (4200 bytes — single pass)
  - `BuildingClass::OnDestroyed` `0x00445880` (Limbo symmetry)
  - `BuildingClass::Place_OccupyMap` `0x00441F60` (tick-1 cell-mark)
  - `BuildingClass::ClearBibArea` `0x00449540` (bib-area — not called by Unlimbo)
  - `TechnoClass::Unlimbo` `0x006F6CA0` (base-class tail)
  - `HouseClass::Place_Production` `0x004FB0E0` (only live caller, vtable[0xD8])
  - `HouseClass::Added_To_Game` `0x00502A80` (stat bump)
  - `HouseClass::Recalc_Base_Center` `0x004FD150`
  - `HouseClass::RecalcBonuses` `0x0050BF60`
  - `BuildingClass::AddUpgrade` `0x00451400`
  - `BuildingClass::ExtendWallInDirection` `0x00452DC0`
  - `BuildingClass::DiscoveredBy` (vtable[0x198]) `0x0044D5D0`
  - `BuildingTypeClass::GetFoundationHeight` `0x0045ECA0`
  - `MapClass::RevealAroundCell` `0x005678E0`
  - `IndexClass::Increment` `0x0049FA00`
  - `FUN_00455F10` (passability OR-mask)
  - `FUN_00457AA0` (CreateFoggedSnapshot / FogOfWar — TS-legacy)
  - `FUN_004FFA50` (factory dispatch)
  - `FUN_0042F260` (AI base-plan mark-filled)
  - `FUN_00448070` (house-edge determination helper)

- **Cross-referenced docs:**
  - `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §§2 (field table), 4 (vtable), 5 (lifecycle), 9 (upgrades), 10 (docking), 13 (walls)
  - `BUILDINGCLASS_VTABLE_FULL_300.md` slot 0xD4/0xD8/0xDC/0x198
  - `BIB_SYSTEM_GHIDRA_REPORT.md` §§2.3–2.5 (bib is visual-only for occupancy; HasBib effect lives in Can_Enter_Cell)
  - `MCV_DEPLOY_GHIDRA_REPORT.md` §§Path 1 (MCV→ConYard via Unlimbo), Path 2 (undeploy)
  - `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` (Unlimbo NOT on load path — uses swap-map fixup)
  - `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` (zone-rebuild not in Unlimbo; deferred to Place_OccupyMap)

- **Vtable binding:** vtable at `0x007E3F94`, slot 0xD8 = `0x00440580`
  (decode: `mcp__ghidra-mcp__read_memory@0x007E3F94 = 80 05 44 00 …`).
