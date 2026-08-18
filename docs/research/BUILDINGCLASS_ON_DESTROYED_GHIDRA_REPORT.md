---
name: BuildingClass OnDestroyed — Blast Chain
description: Full chain from HP=0 to map-removal — DestructionEffects routine, survivors, debris, Tiberium spill, Limbo/OnDestroyed bookkeeping, wall cleanup, AI queue + base-plan cleanup, power/repair/factory/purifier counters, light source teardown, tactical/radar invalidation. Corrects stale offsets from BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md.
type: reference
---

> **Note (2026-04-24):** This report describes `0x004415F0` (`BuildingClass::DestructionEffects`,
> vtable slot 315) — the real HP=0 event handler. Despite the filename and Ghidra's
> prior label, `0x00445880` is NOT OnDestroyed — it's `BuildingClass::Limbo` (vtable
> slot 53 = remove-from-map cleanup). See v3 master §5 for the corrected lifecycle chain.


# BuildingClass OnDestroyed — Ghidra Research Report

**Primary address:** `0x00445880` — `BuildingClass::Limbo` (slot 53 override, a.k.a. "OnDestroyed"), body 0x00445880 – 0x00445E45 (1477 bytes).

**Secondary address (REQUIRED TO UNDERSTAND THIS CHAIN):** `0x004415F0` — **`BuildingClass::DestructionEffects`** (vtable slot 315, offset `+0x4EC`, body 0x004415F0 – 0x00441F55, ~2407 bytes). This is what ReceiveDamage case 4 invokes; it is the actual "kill the building" routine and is more invasive than the Limbo override at 0x445880.

**Confidence:** HIGH (verified from binary decompilation of every named callee).
**Active in YR:** Yes, core lifecycle. No TS-legacy gating on the main path. (Some Type flags read in this chain are themselves TS-legacy — see §14.)

---

## 1. Overview + Trigger

### 1.1 How an attack reaches this chain

```
TechnoClass::Fire → BulletClass::Detonate → WarheadTypeClass::Detonate
  → target->ReceiveDamage(...)

BuildingClass::ReceiveDamage (0x00442230)           [see BUILDING_DAMAGE_DESTRUCTION report §2]
  ├─ TechnoClass::ReceiveDamage (0x00701900)
  │   └─ ObjectClass::ReceiveDamage (0x005F5390)    ← decrements HP, returns 4 (NowDead) if HP hits 0
  ├─ case 4 (NowDead):
  │   ├─ Undock / free MC / chrono emergency undeploy
  │   ├─ Eject / damage docked units (C4Warhead = Rules+0xFA8)
  │   ├─ if Type+0x157B (CanBeOccupied): BuildingClass::SellBuilding (0x00457DE0)   ← bunker-garrison eject
  │   ├─ if this->LightSource (+0x614): FUN_00554A80(0)                             ← ambient light teardown
  │   ├─ this->vtable[0x4EC]()  →  BuildingClass::DestructionEffects (0x004415F0)   ← §2
  │   └─ if IC remaining: vtable[0xF8] (ObjectClass::UnInit) + Place_OccupyMap      ← IC carryover re-occupies
```

Note: IMPORTANT — `BuildingClass::Limbo @ 0x00445880` is not called directly from ReceiveDamage. It runs via `vtable[0xD4]` which is the **Conceal / remove-from-map** slot; it is invoked internally by `TechnoClass::Limbo_Helper` (0x006F6AC0 = "TechnoClass__Limbo_Helper") which sits behind `ObjectClass::UnInit` / `Destroy` for all techno objects. Practically, the chain during a destruction is:

```
DestructionEffects (vtable+0x4EC, 0x004415F0)    ← does survivors, debris, Tiberium, storage
   └─ runs to completion, returns to ReceiveDamage
ReceiveDamage → vtable+0xF8 (ObjectClass::UnInit, 0x005F65F0) if IC-carryover
   └─ UnInit / Destroy eventually calls Conceal (vtable+0xD4) = BuildingClass::Limbo (0x00445880)
      ← §3: walls, radar, counters, FUN_0050A490 AI queue, HouseClass::Recount, Recalc_Base_Center
```

Outside the IC-carryover path the Limbo call is reached via the later destroy cascade after TechnoClass::Limbo_Helper returns — the same 0x00445880 body runs.

### 1.2 Glossary of this report's names

| Binary symbol | Actual role | This report calls it |
|---|---|---|
| `0x00445880` labelled "BuildingClass__OnDestroyed" | `BuildingClass::Limbo` override (slot 53, `vtable+0xD4`) — remove-from-map cleanup | "Limbo/OnDestroyed" |
| `0x004415F0` (new, previously FUN_) | Slot 315 (`vtable+0x4EC`) — destruction effects invoked from ReceiveDamage case 4 | "DestructionEffects" |
| `0x00442D90` labelled "BuildingClass__SpawnSurvivors" | Crew + garrison + debris spawner called by DestructionEffects | "SpawnSurvivors" |
| `0x0044EB10` labelled "FUN_0044eb10" | Slot 195 (`vtable+0x30C`) "GetVoiceResponse" misnomer — actually **GetSurvivorType** | "GetSurvivorType" |
| `0x00451330` labelled "FUN_00451330" | Slot 180 (`vtable+0x2D0`) "GetWeaponRange_2D0" misnomer — actually **GetSurvivorCount** | "GetSurvivorCount" |
| `0x0050A490` labelled "FUN_0050a490" | Remove destroyed building from HouseClass BasePlanNodeArray | "CleanBasePlanForLostBuilding" |
| `0x00509140` labelled "UpdateRadar" | Factory queue revalidation — has nothing to do with radar | "RevalidateFactoryQueueForKind" |

---

## 2. `DestructionEffects` at `0x004415F0` (vtable `+0x4EC`)

This is the pivotal function — everything observable to the player on kill comes from here. Called with `(this, attacker, destroyedByIC_flag, warhead, foundationCellList)`. Body walks top-down:

### 2a. Destroy 8 building animation slots (`+0x5C8..+0x5E4`)

```c
for (i = 0; i < 8; i++) {
    if (this->AnimSlots[i] != NULL) {
        AnimSlots[i]->vtable[0xF8]();   // Destroy (ObjectClass::UnInit on each anim)
        AnimSlots[i] = NULL;
    }
}
```

These are the active idle/damage/production anims created during Unlimbo / state transitions.

### 2b. GapGenerator / CloakGen radius cleanup (`+0x210`, `Type+0x16A4`)

If `this->field_0x210 != 0` AND `Type+0x16A4` (GapGenerator flag):
```c
this->field_0x210 = 0;                       // clear current gap bitmask
Owner->field_0x54E4 = 0;                     // clear house gap aggregate
// re-OR contribution from every surviving gap-generating building
for each building b in Owner->Buildings:
    if !b->InLimbo && b->Type->field_0x16A4:
        Owner->field_0x54E4 |= b->field_0x210;
FUN_004f42f0(2);                             // tactical+mini-map dirty
```

This is a full *rebuild* of the per-house gap bitmask rather than subtracting the lost contribution — handles overlap correctly when two GapGens shared cells.

### 2c. SensorArray radius teardown (`Type+0x16C7`)

If Type+0x16C7 is set (NOTE: this is NOT the `SensorArray` bool at +0x16C8 — inspect further; live binary reads +0x16C7, which may be `EligibleForAllyBuilding` or a related flag, TBD):
```c
this->field_0x6EB = 0xFF;
if (this->field_0x6EC == 0) this->field_0x6EC = Type[0x1707];
this->field_0x80 = 1;                        // dirty visual
this->field_0x6EC = 1;
vtable[0x410](1);                            // slot 260 = UpdateGapGenerator_Tick(stop)
```

### 2d. Wall re-orientation for adjacent walls (`Type+0x16BE` LaserFencePost)

If this building was a laser-fence post:
```c
BuildingClass::ConnectWalls(this, 1);        // arg=1 means "disconnecting"
```

ConnectWalls with arg 1 walks the four cardinal neighbors and recomputes their `+0x11E` connectivity nibble — identical to place-path but inverted (disconnecting).

Walls/Gates (`Type+0x16BF`) and the `ConnectWalls(this, 0)` call in the Limbo body (§3c) additionally trigger the per-cell `PostDestructionWallCleanup` (§3g) on all four cardinal neighbors — that's where wall chains with no live neighbors auto-destroy.

### 2e. EVA / death sound (if Type has no custom die-sound)

```c
if (Type->field_0x520 == 0) {                // no custom VocName
    VocClass::PlayAtCoord(0, this->Location); // plays Rules [AudioVisual] BuildingDieSound (Rules+0x6E8)
}
```

The `0` argument is the default sound slot which resolves via `VocClass` global dispatch to the index in `Rules+0x6E8` = `BuildingDieSound`. There is no `VoiceDie=` on buildings — that TechnoTypeClass key is parsed but unused for buildings (only infantry/vehicle death voice lines read it).

### 2f. "Big building" center-of-footprint debris shower (Foundation ≥ 2x2)

If FoundationWidth > 1 AND FoundationHeight > 1:
- Picks a random interior cell inside the footprint (RandomRanged(0, w-2), (0, h-2))
- 50/50: `SpawnDebris(100, 1)` — scatters metallic debris via Rules metallic-debris array
- otherwise: `Debris_Smoke(100, 1)` — scatters smoke

This is in addition to the per-cell debris in SpawnSurvivors (§4c). Small 1x1 buildings skip this.

### 2g. Per-foundation-cell small-debris (`Type+0x730..0x73C`)

Walks the foundation cell list (short[] terminated by (0x7FFF, 0x7FFF)). For each cell, if `Type+0x73C` (count of per-building specific-debris anim types) > 0:
```c
for each cell in foundation:
    pos = cell_center + random_unit_offset * 0x40
    animIdx = Random_Next() % Type[0x73C]
    animType = Type[0x730][animIdx]          // AnimTypeClass** array
    new AnimClass(animType, pos, Random(0,3), loop=1, speed=0x600, ...)
```

### 2h. Nuclear reactor / chemical-plant radioactive overlay (`Type+0xD15`)

If `Type+0xD15` is set (likely `RadNuke` or `Radiation` emission flag — reactor), walks a hardcoded 4-cell offset table at `0x00818CB8`:
```c
for each of 4 offset cells near the building:
    cell = cell at offset
    if cell has overlay AND overlay->field_0x2B0 != 0:   // tile-eligible overlay
        animType = AnimTypeClass::FindByIndex(<radiation anim>)
        new AnimClass(animType, cell, Random(1,3)+3, loop=1, speed=0x600, ...)
```

This is the fallout "irradiated ground" visual. Complements the 8-cell ore-destruction block in Limbo §3e.

### 2i. Tiberium / ore spill (StorageClass)

```c
total = this->StorageClass.GetTotalAmount();
if (total >= Rules[+0x__: probably StoragePerCell threshold]):
    while (total >= threshold):
        slot = StorageClass.FindFirstNonEmptySlot();
        StorageClass.RemoveAmount(1.0, slot);          // twice (2.0 ore/pass)
        StorageClass.RemoveAmount(1.0, slot);
        offset = random unit vector * RandomRanged(0x100, 0x300)
        cell = cell at building_center + offset
        CellClass::PlaceTiberium(slot, 1);             // spawns ore overlay of matching tiberium type
```

Destroyed refineries/silos leak **half** of their stored ore as ore-overlay cells scattered within ~0x300 leptons (3 cells).

### 2j. Refund to owner (post-StorageClass)

```c
cost = Type->GetCost(Owner, 0);                // vtable+0x84 on Type — applies MultiplayerPassive scaling
if (cost / Rules[+0x5C8] > 0):
    FUN_0048DED0();                            // TBD — likely credit refund fraction or EVA trigger
```

Rules+0x5C8 is not yet identified. The ratio test suggests either an "only refund if >=1 credit worth" guard or score-tracking.

### 2k. Repair / IC timer reset

If the building was repairing (`vtable[0x184]() == 0x13` == MISSION_REPAIR) OR Type+0xD15 (reactor special):
```c
this->field_0x528 = CurrentFrame;      // IC start frame = now
this->field_0x52C = 0;                 // IC timer pad cleared
this->field_0x530 = 0;                 // IC duration cleared
this->field_0x100 = CurrentFrame;      // (second timer)
this->field_0x104 = 0;
this->field_0x108 = 0;
this->field_0x10c = 0;
```
Otherwise (non-repair kill):
```c
this->field_0x528 = CurrentFrame;
this->field_0x52C = 0;
this->field_0x530 = 8;                 // 8 frames of "post-mortem" duration
```

Note: **+0x620 (RepairProgress) is never cleared.** This is intentional — the building is about to be freed anyway, so stale repair accumulator is irrelevant. Save/load rebuilds from scratch.

### 2l. "Death explosion" main anim (`Type+0x758`, `Type+0x74C`)

```c
if (Type[0x758] > 0) {                         // count of death anims
    idx = Random_Next() % Type[0x758];
    animType = Type[0x74C][idx];               // AnimTypeClass** array
    anim = new AnimClass(animType, center_coord, 0, 1, 0x600, 0, 0);
    if (Type[0xDF0] != 0) {                   // has "owner" animation linkage
        anim->field_0xD4 = vtable[0x1E4]();    // set anim's owner color
        if (Type[0xDD0] != 0)                  // custom die-sound name
            strncpy(anim+0xDC, Type+0xDD0, 0x20);
    }
}
```

This is the primary "explosion" visual — each BuildingTypeClass can list up to N death anims in `Type+0x74C`, picked at random.

### 2m. Particle system spawn (smoke plume, for big dying buildings)

`FUN_0045AD80(0,0)` followed by a loop collecting particle-system types from `Type+0x798/0x7A4`; if the building has any particle systems attached and passes a `vtable+0x1C8` check, picks one at random and spawns a ParticleSystemClass at `this->Location + (Type+0x7C0,7C4,7C8)`. Stored at `this->field_0x320`.

### 2n. Set Health = 0

```c
this->Health = 0;                              // force-zero (redundant — already was by this point)
if (destroyedByIC_flag) this->field_0x6E0 = 1; // mark IC-kill for survivor spawn (§4a)
```

### 2o. Spawn survivors

```c
BuildingClass::SpawnSurvivors(this);           // §4
FootClass::EMPPassengers(attacker);            // freezes any passengers mid-air
```

EMPPassengers is a separate helper that cascades any EMP effect to currently-boarded infantry.

---

## 3. `Limbo/OnDestroyed` at `0x00445880` (vtable `+0xD4`)

Runs as the `Conceal`/remove-from-map slot. Precondition: `this->field_0x81 == 0` (NOT already in limbo) AND `this->Type != NULL`. Skips entirely if either fails.

### 3a. Destroy 8 anim slots (again, belt-and-braces)

Same 8-slot walk as §2a; after DestructionEffects already cleared them this is a no-op, but the Limbo path is also reached on sells/demolish where DestructionEffects did not run.

### 3b. Per-house counter decrements (ActuallyPlacedOnMap gates all four)

CORRECTION to BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md — the old report's identifications were wrong. Verified against BUILDINGTYPECLASS_FIELDS.csv:

| Type offset | INI key | Owner field effect |
|---|---|---|
| `+0x16CC` | **`OrePurifier=yes`** | `Owner->HarvestBonusCount (+0x538C) -= 1` (clamp ≥0) |
| `+0x16CB` | **`Helipad=yes`** | `Owner->AircraftDockCapacity (+0x2D4) -= Type->NumberOfDocks (+0x1780)` (clamp ≥0) |
| `+0x1564` (nonzero) | **`InfantryGainSelfHeal=N`** | `Owner->PowerOutputUnits (+0x164) -= N` (clamp ≥0) |
| `+0x1568` (nonzero) | **`UnitsGainSelfHeal=N`** | `Owner->PowerDrainUnits (+0x168) -= N` (clamp ≥0) |

Notes:
- `+0x164`/`+0x168` are misnamed in HOUSECLASS_VERIFIED_FIELD_MAP.md as "PowerOutputUnits/PowerDrainUnits". The binary subtracts **self-heal grant amounts**, not power. Those two house fields actually aggregate *bonus grants for self-healing* (a hospital gives infantry +N/tick regen, a repair depot gives vehicles +N/tick). This is a long-standing misidentification — flag for follow-up but do not re-label yet (≥90% confidence required).
- The old report's "Factory Count (Type+0x16CC)" claim is wrong. +0x16CC is OrePurifier.
- The old report's "Storage capacity (Type+0x16CB)" claim is wrong. +0x16CB is Helipad, and it's scaled by NumberOfDocks (+0x1780).

### 3c. Wall connection update (Type+0x16BE)

```c
if (Type+0x16BE != 0) {                        // IsLaserFencePost
    BuildingClass::ConnectWalls(this, 0);      // connect=0 → disconnect neighbors
}
```
This redoes the neighbor cell's `+0x11E` connectivity nibble after the wall post leaves.

### 3d. SensorArray and DetectDisguise cleanup (g_MapEditorMode gate)

```c
if (g_MapEditorMode == 0) {
    if (Type+0x16C8 != 0)  vtable[0x4F8](DAT_0089c818);  // RemoveSensorArrayAt(center_cell)
    if (Type+0xD31  != 0)  vtable[0x500](DAT_0089c818);  // RemoveDisguiseDetectorAt(center_cell)
}
```

### 3e. Power grid recalculation

```c
if ((Type->field_0x408 == 0 || Type->field_0x5EC == 0) && !vtable[0x80]() /*not cloaked*/) {
    FUN_004561F0();     // full power grid recalc (Owner->CalcPower / CalcDrain)
}
```

### 3f. Upgrade scan (for post-Limbo super-weapon recalc)

```c
bool hasSpecialUpgrade = false;
for (i = 0; i < 3; i++)
    if (this->Upgrades[i] && Upgrades[i]->field_0x1763)
        hasSpecialUpgrade = true;
```
Used at §3l to set the super-weapon dirty flag.

### 3g. Nuclear reactor ore destruction (Type == Rules+0x87C → `NukeReactor`)

```c
if (Type == Rules[+0x87C] /* = NuclearReactor type */) {
    for (i = 0; i < 8; i++) {                         // 8 cardinal+ordinal neighbors
        cell = MapClass::GetCellAt(center + DirOffset[i*2]);
        CellClass::PostDestructionWallCleanup(cell, 0);
        if (cell->OverlayTypeIndex != -1 &&
            OverlayTypeClass_Array[cell->OverlayTypeIndex]->field_0x2A8 != 0 /* wall overlay */ &&
            cell->field_0x11E < 0x10) {
            CellClass::DestroyOverlay(200);           // destroys ore/tiberium in reactor blast
        }
    }
}
```

### 3h. Bridge repair hut cleanup (Rules+0x86C / +0x874)

Bridge repair-hut types refresh cells at (-1, 0) and (+3, 0) from building origin — the two bridge segments either side of the hut.

### 3i. Bridge section cleanup (Rules+0x870 / +0x878)

Vertical equivalent: refresh cells at (0, -1) and (0, +3).

### 3j. Per-foundation-cell occupancy decrement (for non-custom-foundation buildings)

```c
if (Type+0xE58 == 0) {                          // not a custom-foundation building
    w = GetFoundationWidth() + 2;
    h = GetFoundationHeight() + 2;
    for (y = 0; y < h; y++)
        for (x = 0; x < w; x++) {
            cell = MapClass::GetCellAt(origin + (x-1, y-1));
            cell->field_0x122 -= 1;             // "has-building" counter
        }
}
```
The +2 / -1 pattern expands the footprint by 1 cell in every direction — this is the *bib row* plus a 1-cell safety margin. Used by adjacency logic, not pathfinding.

### 3k. Screen invalidation

```c
if (g_Tactical != 0) {
    rect = vtable[0x12C](center, 0);            // get building render rect
    TacticalClass::DirtyScreenRect(rect);
}
```

### 3l. LightSource/BuildingLight teardown (`+0x600` only)

```c
if (this->field_0x600 != NULL) {                // BuildingLightClass* spotlight
    this->field_0x600->vtable[0xF8]();          // Destroy (UnInit)
}
```
Note: **`this->field_0x614` (LightSourceClass* ambient) is destroyed earlier in ReceiveDamage case 4**, not here. The asymmetry flagged in v2 §19 is real but intentional — LightSource is tied to "building alive" (destroyed with death), BuildingLight spotlight is tied to "building on map" (destroyed with removal from map, e.g. also on sell).

### 3m. House counters and UI flags

```c
HouseClass::Recount(Owner, this);               // per-kind counter-- (§3m detail below)
Owner->field_0x1FC = 1;                         // AI: needs base recenter
HouseClass::Recalc_Base_Center(Owner);          // immediate recompute (center cell + radius)
if (Owner == g_PlayerPtr) {
    DAT_00880CF4 = 1;                           // sidebar-dirty global
    FUN_004F42F0(0);                            // tactical-dirty flag
}
```

**HouseClass::Recount (`0x004FF980`) kind→counter map** (switch on `Type+0xEB8`):
| Kind value | Counter field | Meaning |
|---|---|---|
| 1, 0x28 | `Owner+0x5380` (or `+0x5388` if Type+0xCCE radar bit) | ConYard / RadarTower count |
| 2, 3 | `Owner+0x5378` | Barracks / WarFactory count |
| 6, 7 | `Owner+0x5384` | Helipad / NavalYard count |
| 0xF, 0x10 | `Owner+0x537C` | Shipyard / tech |
| default | (no decrement) | |

### 3n. BasePlanNodeArray cleanup (`FUN_0050A490` = `CleanBasePlanForLostBuilding`)

```c
for (i = 0; i < Owner->BasePlanNodeCount; i++) {
    node = Owner->BasePlanNodeArray[i];         // 16-byte entry: { TypeID, CellX, CellY, TargetCell }
    if (node.TypeID == Type->ArrayIndex &&
        node.CellX == buildingCellX && node.CellY == buildingCellY) {
        // Found the node for this lost building.
        // 1) Invalidate any sibling nodes whose targetCell matches this node's targetCell.
        for (j = 0; j < count; j++)
            if (j != i && node[j].TargetCell == node[i].TargetCell)
                node[j].TargetCell = g_InvalidCell;
        // 2) If BaseNormal (Type+0x1706) and multiplayer, fully invalidate this node:
        if (Type+0x1706 != 0 && g_GameMode != 0) {
            node[i].TypeID = -1;
            node[i].TargetCell = g_InvalidCell;
        }
        break;
    }
}
```

This is the "AI threat table" cleanup referenced in Task 10's scope — except it's a **build plan**, not a threat table. The AI threat table (`+0x5704`) is a separate structure and **is not touched** by OnDestroyed. Live-combat AI threat is decayed automatically via the HouseClass update tick.

### 3o. TechnoClass::Limbo_Helper tail (`0x006F6AC0`)

Unconditional at end:
```c
uVar4 = TechnoClass::Limbo_Helper(this);
// — updates shroud/fog around cell
// — HouseClass::Removed_From_Game
// — Releases 3 SoundEvent channels (field_0x127, +0x12E, +0x135)
// — clears retreat-zone, finally calls TechnoClass__Limbo_Tail_CallConceal
// — ObjectClass::Conceal sets +0x81 InLimbo=1, deselects, removes from cell list
```

### 3p. Super-weapon recalculation

```c
if (hasSpecialUpgrade) FUN_00509130(Owner);    // sets Owner->field_0x1FB = 1 (SW-dirty flag)
```

### 3q. Production-queue revalidation (`UpdateRadar` misnomer, `0x00509140`)

```c
if (Type+0xEB8 /* FactoryKind */ != 0)
    RevalidateFactoryQueueForKind(Owner, Type->FactoryKind, Type->field_0xCCE);
```

Walks the Owner's factory(s) for matching kind (3=Infantry, 7=Vehicle/Aircraft, 0x10=Ship, 0x28=Building) and:
- Removes queued objects that can no longer be built (fails `vtable+0x94` constructability check).
- If the currently-producing object becomes unbuildable → `FactoryClass::AbandonProduction` + start next queued.
- If it remains buildable but now fails a price/power check → `FactoryClass::Suspend`, and for the player shows the "low funds/power" sidebar flash.
- Finally, if the factory has no object or queue left, calls `FactoryClass::vtable[0x20](1)` to self-destruct the FactoryClass.

This is what makes losing your only ConYard instantly stop construction and refund — the player's ConYard was the factory pointer for the `Building` kind; nothing can be built after it's gone.

### 3r. Laser fence frame reset

```c
if (Type+0x16BE != 0 && !g_MapEditorMode)
    this->LaserFenceFrame = 0;
```

---

## 4. Survivor Math (`BuildingClass::SpawnSurvivors` @ `0x00442D90`)

### 4a. Garrison occupant ejection (runs first, for bunker types)

Gate: `(Type+0x16AE /*CanBeOccupied*/ || Type+0x16AF /*CanOccupyFire*/) && this->field_0x114 > 0`.

For each occupant (pulled via `FUN_00473430` iterator):
```c
cellCoord = this->center + random_foundation_offset   // or center for vehicles
occupant->OwnerIndex = this->field_0x8C               // building's house
if (!this->field_0x6E0 /* not IC-killed */ &&
    occupant->vtable[0xD8](cellCoord, facing)         // Unlimbo
   == success) {
    occupant->field_0x439 = 0                          // clear "inside" flag
    occupant->vtable[0x174](CoordBuffer, 1, 0)         // issue Scatter
    if (!HouseClass::IsPlayerControl(occupant->Owner)) {
        occupant->vtable[0x1E8](0xF, 0)                // Mission_Hunt for AI
    }
} else {
    // IC-killed or unlimbo failed: Change_Owner(attacker-house|null), then Destroy
    occupant->vtable[0xE0](attacker_house_or_null)
    occupant->vtable[0xF8]()
}
```

### 4b. Crew survivor spawn (iterates foundation cell list)

Survivor count is a *budget* that depletes on each successful spawn:

```c
budget = GetSurvivorCount(this);                      // 0..5, §4b.1
if (Owner->field_0x1F6 /*IsDefeated*/) budget = 0;

spawnChanceDenominator = 2;
if (this->field_0x540 != 0) spawnChanceDenominator = 1;  // IC-related / was delay-killed
if (this->field_0x6E3 != 0) spawnChanceDenominator += 6; // crewless-like: much rarer

for each foundation cell:
    if (budget > 0 && Random::RandomRanged(0, denominator) == 1) {
        crewType = GetSurvivorType(this);             // §4b.2
        if (crewType != NULL) {
            survivor = new InfantryClass(crewType, Owner);
            if (this->field_0x6E9 /*C4 vet flag*/ && crewType->field_0xC9E)
                survivor->field_0x6D9 = 1;            // propagate C4 marker to survivor

            if (survivor->vtable[0xD8](cellCoord, 0 /*facing*/) == success) {
                budget--;
                survivor->Health = Random::RandomRanged(5, survivor->Type->Strength);
                survivor->vtable[0x174](CoordBuffer, 1, 0);   // Scatter

                if (attacker == NULL || IsAlly(attacker)) {
                    if (Owner == Player) survivor->Mission = MISSION_GUARD (=2)
                    else                 survivor->Mission = MISSION_HUNT  (=0xF)
                } else {
                    survivor->Mission = MISSION_ATTACK (=1);
                    survivor->SetTarget(attacker);
                }
            } else {
                survivor->Destroy();                  // unlimbo failed — discard
            }
        }
    }

    // Per-cell debris (independent of survivor budget):
    if (cell is passable) {
        if (Random::RandomRanged(0,99) < 50)
            SpawnDebris(100, 0);                       // metallic
        else
            Debris_Smoke(100, 0);                      // smoke
    }
```

#### 4b.1 GetSurvivorCount (`0x00451330`, `vtable+0x2D0`)

```c
int GetSurvivorCount(this) {
    if (this->field_0x6E0 /*IC-killed*/ != 0)  return 0;   // no survivors from IC death
    if (!Type->field_0xCCD /*Crewed=yes*/)      return 0;   // not crewed → nobody inside

    divisor = [
        Rules+0x14F8,  // AlliedSurvivorDivisor (Side 0)
        Rules+0x14FC,  // SovietSurvivorDivisor (Side 1)
        Rules+0x1500,  // ThirdSurvivorDivisor  (Side 2)
    ][Owner->SideIndex];
    if (divisor == 0) return 0;

    if (this->field_0x6E3 /*crew-reducing flag*/) divisor *= 2;

    cost = Type->GetCost(Owner, 0);            // full cost, applies MultiplayerPassive

    n = cost / divisor;
    if (n < 1) n = 1;
    if (n > 5) n = 5;
    return n;                                  // clamped to [1..5]
}
```

**Formula (plain English):** `survivors = clamp(Cost / SideSurvivorDivisor, 1, 5)`. Halved (effectively) when the building has the cheap-crew flag, and **zero** if Crewed=no or the building died to Iron Curtain.

**Corrections vs task spec / v2:** the spec suggested difficulty-scaled. It is **side-scaled**, not difficulty. Rules keys are `AlliedSurvivorDivisor`, `SovietSurvivorDivisor`, `ThirdSurvivorDivisor` (RA2 INI literals).

#### 4b.2 GetSurvivorType (`0x0044EB10`, `vtable+0x30C`)

```c
InfantryType* GetSurvivorType(this) {
    if (this->field_0x6E3 == 0 &&
        Type->field_0xEB8 /*FactoryKind*/ == 7 &&      // 7 = Vehicle/Weapons factory kind
        Random::RandomRanged(0, 99) < 25) {
        return Rules[+0xF70];                          // Engineer
    }
    return TechnoClass::Crew_Type(this);               // §4b.3
}
```

**Corrections vs v2 master §6:** spec said "Factory==7 && +0x6E3==0 → Engineer". Confirmed — 25% chance, only when `field_0x6E3 == 0`. `Type+0xEB8 == 7` means **any factory with FactoryKind=7** — which in stock YR is Soviet and Allied **Weapons Factories** (both have Factory=Vehicle/Aircraft kind 7). Not specifically a Soviet-only rule.

BUT: `Type+0xEB8` values per `HouseClass::Recount` reading — 3 = Infantry factory, 7 = Vehicle factory, 0x28 = ConYard, etc. So the effect is: *any war factory* has a 25% chance of ejecting an Engineer (not a regular crew) when destroyed. Both Allied and Soviet Weapon Factories. This is a classic RA2 surprise mechanic.

#### 4b.3 TechnoClass::Crew_Type (`0x00707D20`)

```c
InfantryType* Crew_Type(this) {
    if (!Type->field_0xCCD /*Crewed*/)  return NULL;

    side = Owner->SideIndex;
    crew = side == 0 ? Rules+0xF78 /*AlliedCrew*/ :
           side == 1 ? Rules+0xF7C /*SovietCrew*/ :
           side == 2 ? Rules+0xF80 /*ThirdCrew*/  :
                      Rules+0xF6C /*Technician*/;
    if (Owner->Country->field_0xBC == -1) crew = Rules+0xF6C;  // no country → Technician

    if (vtable[0x2AC]() /* IsTechnician helper */ &&
        Random::RandomRanged(0, 99) < 15)
        return Rules+0xF6C;                      // 15% chance of Technician

    return crew;
}
```

**Faction mix summary:**
| Owner side | Default crew | Override |
|---|---|---|
| 0 (Allied) | `AlliedCrew=` (GI, default `E1`) | 15% chance → `Technician=` |
| 1 (Soviet) | `SovietCrew=` (Conscript, default `E2`) | 15% chance → `Technician=` |
| 2 (Yuri)   | `ThirdCrew=`  (Initiate, default `INIT`) | 15% chance → `Technician=` |
| Unknown country | `Technician=` | — |

**Total GetSurvivorType decision tree:**
1. If FactoryKind==7 AND not-cheap-crew → 25% Engineer, else fall through.
2. Else: SideCrew, 15% Technician chance.

### 4c. Cell scatter pattern — **not random placement**

CRITICAL: the spawn positions are **fully deterministic** — one attempt per foundation cell, in foundation-list order. The only randomness is:
- Per-cell RNG roll to decide IF a spawn happens (`Random(0, denominator) == 1`)
- Per-survivor randomized Health (5..MaxStrength)
- Per-survivor crew-type (for the 25%/15% branches)

The building's foundation cell list (from `vtable+0x108`) is the same ordered array used for placing, so replays are deterministic given the RNG state.

---

## 5. Crater Spawn

**BuildingClass::OnDestroyed does NOT spawn craters.** Craters are purely a **warhead/anim** effect:

- `AnimTypeClass.Crater=yes` (field parsed at 0x0081a5f4, read by `AnimTypeClass::ReadINI @ 0x00427EBD`) + `ForceBigCraters=yes` flag.
- The death anim(s) listed in `Type+0x74C` (see §2l) are what trigger craters, via the normal `AnimClass` spawn pipeline. Each explosion anim decides its own crater.
- `[General] Crater=` in rules.ini is the *default crater AnimType* used by warheads that don't list one.

**Parity implication:** our Rust engine should spawn craters from the explosion anim definitions, not from a building-specific "Crater=" key. There is none.

---

## 6. Ore Pile / Rubble Leave-Behind

### 6a. `LeaveRubble=` (Type+0x1768)

**Parsed but dead.** Only xref is `BuildingTypeClass__ReadINI_Water @ 0x00460EF5`. No consumer anywhere in the binary. This is TS-legacy — in Tiberian Sun, destroyed buildings left "rubble" tile overlays; in YR this code path was removed when the rubble system was cut.

**Parity implication:** do not implement. The YR-faithful behavior is **no rubble tile** on destruction.

### 6b. `Type+0x157B` (CanBeOccupied) triggers SellBuilding on death

The ReceiveDamage case 4 has:
```c
if (Type+0x157B /*CanBeOccupied*/ != 0) BuildingClass::SellBuilding(this);
```
SellBuilding here is a misnomer — it's the **bunker-garrison ejection** routine at `0x00457DE0`. Ejects any live occupants via cell placement. This runs **in addition to** SpawnSurvivors' garrison loop (§4a), but the two paths guard each other via the occupant count / field_0x6E0 checks, so no double-eject in practice.

### 6c. Actual "leave-behind" mechanics on destruction

- **Ore spill** (refineries/silos): §2i. Scatters up to N ore cells within ~3 cells, consuming StorageClass content.
- **Nuclear reactor ore-destruction**: §3g. Destroys 8 adjacent ore cells.
- **Per-cell debris & smoke**: §2f (big buildings), §2g (per-cell type-specific), §4b (50/50 per foundation cell).

No "rubble" tile. No persistent structural remnant.

---

## 7. Money Crate Spawn (`CrateBeneath=`, `CrateBeneathIsMoney=`)

Fully documented in `CRATE_SYSTEM_GHIDRA_REPORT.md §8.1`. Summary:

- Consumer is `BuildingClass::Place_OccupyMap @ 0x00441F60` (note: despite name, runs on destroy too, via the IC-carryover path in ReceiveDamage).
- `Type+0x1767` (`CrateBeneath=yes`) → calls `PlaceCrateAtCell(buildingCenterCell, type)`.
- `Type+0x1769` (`CrateBeneathIsMoney=yes`) → type arg = 0 → forces Money crate.
- Otherwise → type arg = 0x14 (20) → forced random-crate (≥0x13 re-rolls via weighted table).

**Gating in YR:** Only fires if `g_IsMultiplayer != 0` (effectively). In SP it technically runs but the crate pool is sparse.

**CRITICAL FIDELITY NOTE:** The CrateBeneath path is only reached when the building is **IC-carried-over** (remaining IC duration > 0 after death). In a normal (non-IC) kill, Place_OccupyMap is never called on the death path, so CrateBeneath does NOT fire. This contradicts the common mod documentation that claims "a CrateBeneath=yes building always drops a crate on destruction". It only drops if it died while under Iron Curtain with time left on the timer. **This is almost certainly a gamemd bug** but we must reproduce it exactly for parity.

---

## 8. EVA Cue

No EVA line fires from OnDestroyed/DestructionEffects. The destruction audio is:

- §2e: `VocClass::PlayAtCoord(0)` — the default death-sound index, resolving to `Rules [AudioVisual] BuildingDieSound=` (Rules+0x6E8). A positional sound, not an EVA line.

EVA announcements (like "Our Construction Yard is under attack") fire from `HouseClass::NotifyUnderAttack` invoked during ReceiveDamage *before* death, not on destruction. A "Building Lost" EVA line, if any, would be the player's own EVA trigger set elsewhere — not in this chain.

---

## 9. Zone Rebuild

**Triggered? Yes — partially.**

- §3j decrements per-cell occupancy (`+0x122`).
- `PostDestructionWallCleanup` (called from §3g for reactors and from §4/neighbor walks) calls `MapClass::AssignOrphanedCellZone` (when a wall-overlay is removed) OR `MapClass::MergeAdjacentCellZone` (when a wall is retained) on every visited cell.
- `FUN_00584550` is called alongside zone operations — this is the pathfinding-zone recomputation trigger.

No full-map zone rebuild; only a local neighbourhood re-zone. This is the right granularity — otherwise every building destroyed would trigger an O(MxN) zone pass.

---

## 10. Queue Cleanup (`FUN_0050A490` — BasePlanNodeArray)

See §3n. This removes the lost building's entry from the house's *AI build plan queue*, not the *AI threat table*. The threat table (`Owner+0x5704`) decays over time via the HouseClass tick; destroying a building does not explicitly touch it.

---

## 11. AI Threat Table Removal — **Nothing explicit**

Contrary to Task 10's premise, the `BuildingClass::Limbo` path does **not** directly scrub AI threat tables. What it does:

1. Removes the building's entry from `HouseClass::BasePlanNodeArray +0x5708` (§3n).
2. Triggers `HouseClass::Recalc_Base_Center` (§3m) — recomputes center cell + radius.
3. Sets `Owner->field_0x1FC = 1` — flag the AI will honor next tick to redo base-plan threat distribution.

The actual threat table is updated lazily by HouseClass::AIUpdate — by the time it next runs, the destroyed building is already out of `Owner->Buildings` so threat naturally decreases.

---

## 12. Wall Disconnection

### 12a. During DestructionEffects (§2d)

`BuildingClass::ConnectWalls(this, 1)` if Type is a laser-fence post.

### 12b. During Limbo (§3c)

`BuildingClass::ConnectWalls(this, 0)` same condition.

### 12c. Cardinal-neighbor wall cell cleanup

For regular walls (overlay-based, not building-based — `Type+0x16BF` gates the overlay-wall path), the `PostDestructionWallCleanup` helper at `0x00480630` walks 4 cardinal neighbors per destroyed wall cell. For each:

1. Dirty tactical/radar rect.
2. If neighbor cell has a wall overlay, recompute its 4-bit connectivity nibble (`field_0x11E & 0xF`) by testing all 8 directions via `CellClass::IsWallConnectableInDirection`.
3. Apply hardcoded "orphan wall auto-destroy" rules:
   - Overlay 0 (GASAND): data ∈ {0x10, 0x20} → destroy
   - Overlay 1 (CYCL): data == 0x20 → destroy
   - Overlay 2 (GAWALL): data ∈ {0x20, 0x30} → destroy
   - Overlay 3 (BARB): data == 0x10 → destroy
   - Overlay 0x16: data ∈ {0x10, 0x20} → destroy
4. `RecalcAttributes`; if destroyed → `AssignOrphanedCellZone` + decrement `OreNeighborCount (+0x122)` on 8 neighbors; else `MergeAdjacentCellZone`.

**This gives the classic RA2 cascade of "chain of wall segments dissolves when one is shot"** — each destroyed wall cell triggers the neighbors; orphaned segments (damage level == max, no adjacencies) auto-destroy; the cascade continues outward.

---

## 13. Timer + Resource Cleanup

### 13a. What IS cleared

- IC timer (`+0x528/52C/530`): reset in §2k.
- Secondary timer (`+0x100/104/108/10C`): reset in §2k (repair-mission path only).
- 8 anim slots (`+0x5C8..+0x5E4`): destroyed in §2a + §3a.
- BuildingLightClass spotlight (`+0x600`): destroyed in §3l.
- LightSourceClass ambient light (`+0x614`): destroyed in ReceiveDamage case 4 (before DestructionEffects).
- GapGen contribution (`+0x210` + `Owner+0x54E4`): rebuilt in §2b.
- 4 counter decrements on owner: OrePurifier, Helipad-docks, InfantryGainSelfHeal, UnitsGainSelfHeal (§3b).
- House per-kind counts (`+0x5378..+0x5388`): decremented in §3m via HouseClass::Recount.
- Base plan node for this building (`+0x5708`): invalidated in §3n.
- 3 SoundEvent channels (`+0x127, +0x12E, +0x135`): released in §3o via TechnoClass::Limbo_Helper.
- Retreat zone (`+0x142`): set to 0 in TechnoClass::Limbo_Helper.
- `field_0x81` InLimbo: set to 1 at the very end (via ObjectClass::Conceal).

### 13b. What is NOT cleared

- **`+0x620` RepairProgress** — not cleared. Stale value ignored once building is freed.
- **`+0x624` RepairActive** — not cleared. Same as +0x620.
- **`+0x67C/+0x680` dock slot arrays** — not touched here; the docked units either got ejected (§ReceiveDamage case 4) or damaged by C4, and the arrays are implicitly freed when the BuildingClass destructor runs later.
- **CaptureManager (`+0x2BC`)** — `CaptureManagerClass::FreeAll` was called earlier in ReceiveDamage case 4, but the CaptureManagerClass itself is destroyed by the destructor chain.
- **`+0x6DD` ProductionDone flag** — not cleared.
- **AI threat table (`+0x5704`)** — NOT scrubbed (see §11).

---

## 14. Concrete Walkthroughs

### 14.1 Allied ConYard (GACNST) destroyed by enemy attack

Preconditions: GACNST, Cost=3000, FactoryKind=0x28 (ConYard), Crewed=yes, side=Allied, not IC-killed.

1. ReceiveDamage case 4 fires at HP=0.
2. No docked unit; no passengers; no MC slaves to free.
3. `SellBuilding` (bunker eject) is a no-op (CanBeOccupied=no).
4. LightSource (+0x614): none by default → skip.
5. DestructionEffects (§2):
   - 8 anim slots destroyed
   - GapGen cleanup skipped (not a GapGen)
   - Wall reconnection skipped (not a wall post)
   - Plays `BuildingDieSound` at center
   - Big-building debris: ConYard foundation is 4x4 → YES: 50/50 metallic-debris vs smoke at random interior cell
   - Per-foundation per-cell debris loop: 16 cells get a Type-specific debris anim (if any)
   - No reactor-fallout
   - Tiberium spill: StorageClass is usually empty → skipped
   - Cost-refund math: cost/Rules+0x5C8 > 0 → fires (effect unclear)
   - Timer reset (§2k)
   - Death explosion (§2l): GACNST's Type+0x74C anim list → pick random → new AnimClass at center
   - Particle system (§2m): if GACNST has one → spawn smoke plume at +0x320
   - Survivors:
     - `GetSurvivorCount`: Crewed=yes, side=0 (Allied), `AlliedSurvivorDivisor=1000` (default) → 3000/1000 = 3 survivors
     - Per-cell RNG: each of ~16 cells has 1/2 chance to be the "chosen" cell → typically ~3 survivors spawn before budget exhausts
     - `GetSurvivorType`: FactoryKind=0x28 ≠ 7 → falls through to `Crew_Type` → AlliedCrew = E1 (GI), 15% Technician
     - Each survivor: Health=Random(5, 125), Mission=ATTACK retaliate on attacker (since attacker is enemy)
   - Per-cell debris: 50/50 metallic vs smoke on each of the 16 foundation cells (independent of survivor spawn)
6. Limbo / §3 chain:
   - Not OrePurifier, not Helipad, no self-heal grants → no counter changes
   - Not LaserFencePost → no wall reconnection
   - Not SensorArray/DetectDisguise → skip
   - Power grid recalc (usually ConYard=no PowerOutput unless modded)
   - No upgrades with super-weapon
   - Not nuclear reactor, not bridge hut, not bridge section
   - Per-cell occupancy: 6x6 = 36 cells (foundation + 1-cell border) get `field_0x122 -= 1`
   - Tactical screen rect dirtied
   - `BuildingLight` (+0x600): most ConYards have no spotlight → skip
   - `HouseClass::Recount`: FactoryKind=0x28 → `Owner->field_0x5380 -= 1` (ConYard count)
   - `Recalc_Base_Center`: rebuilds base center (new center moves closer to surviving buildings)
   - Player's ConYard: `DAT_00880CF4 = 1` + `FUN_004F42F0(0)` → sidebar dirty, tactical dirty
   - `CleanBasePlanForLostBuilding`: ConYard's node removed from BasePlanNodeArray
   - TechnoClass::Limbo_Helper: shroud/fog update around former center, SoundEvent release, HouseClass::Removed_From_Game
   - `RevalidateFactoryQueueForKind(0x28)`: finds no other ConYard → walks the Building-kind factory, aborts production of everything queued, destroys the FactoryClass. **This is why losing your only ConYard instantly halts all building construction.**
   - InLimbo flag set to 1

**Observable to player:**
- Big explosion (random from GACNST's death-anim list)
- Smoke plume (particle system)
- 16 cells of debris/smoke + 3-ish GI survivors (health 5–125) attacking the attacker
- Death sound at the former center
- Sidebar: building tab greys out all structures (no ConYard → no Building queue)
- Minimap: building icon gone; base center recomputes
- Shroud: cell becomes shrouded after TechnoClass::Limbo_Helper's UpdateFogBorder

### 14.2 Tesla Coil (NASAM / TESLA) destroyed

Preconditions: NATESLA, Cost=1500, FactoryKind=0 (none — not a factory), Crewed=yes, side=Soviet.

Differences from ConYard:
- FactoryKind=0 → **no RevalidateFactoryQueueForKind call**
- 2x2 foundation → big-building debris still fires (foundations ≥ 2x2)
- Survivor count: 1500/`SovietSurvivorDivisor`(default 1000) = 1 survivor
- Crew type: Conscript (SovietCrew). 25% Engineer chance? Only if FactoryKind==7 — NO for a Tesla → regular Conscript
- Recount: FactoryKind=0 → default case → **no counter changes**
- CleanBasePlanForLostBuilding: Tesla node removed
- **If Tesla had Powered=yes and was a key power producer:** FUN_004561F0 power recalc kicks in, other Powered buildings may go offline (visual yellow flicker next tick)

### 14.3 Wall segment (GAWALL overlay) destroyed

Wall segments on normal overlay tiles (not laser-fence posts) are NOT BuildingClass objects — they live in CellClass's overlay field. The destruction path is `OverlayClass::TakeDamage` → `CellClass::DestroyOverlay` → `PostDestructionWallCleanup` on this cell + 4 neighbors (§12c).

The cascade:
1. Removed cell's `field_0x11E` → 0, `OverlayTypeIndex` → -1, `field_0x50` → -1.
2. For each of 4 cardinal neighbors: recompute connectivity; if orphaned (no neighbors AND damage==max) destroy them too → recurse for another hop.
3. Each destroyed cell: `AssignOrphanedCellZone` + `OreNeighborCount -= 1` on 8 neighbors.

This uses **`PostDestructionWallCleanup`, NOT BuildingClass::OnDestroyed**. Laser fences (`Type+0x16BE`) use BuildingClass::ConnectWalls path (§2d, §3c).

---

## 15. Magic Constants and Edge Cases

### 15a. Crewed=no totally disables survivors

Both GetSurvivorCount and Crew_Type return early on `Type+0xCCD == 0`. The same building dies silently with no crew — only the debris/smoke animations.

### 15b. IC-killed buildings spawn no survivors and no ejected garrison

`field_0x6E0 == 1` (set in §2n if IC kill) gates out:
- GetSurvivorCount → returns 0
- Garrison ejection loop (§4a) takes the IC-eject branch: Change_Owner(null) + Destroy (no unlimbo attempt)

### 15c. `field_0x6E3` — the "hidden" crew reduction flag

Doubles the survivor-spawn denominator AND blocks the Engineer roll. Set when: TBD — grep finds multiple writers but no obvious single semantic. Likely "crewless" variant (ConYard without engineer, refinery without harvester, etc.).

### 15d. `field_0x540` — "was delay-killed" halves the spawn denominator

Denominator goes from 2 to 1 → roll `Random(0,1)==1` → 50% chance per cell → much higher spawn rate.

### 15e. Foundation boundary handling (§3j)

The "+2 / -1" cell iteration walks a (W+2) x (H+2) rectangle centered on origin-1. This includes the 1-cell safety margin used for adjacency tests (targeting helpers). Non-custom-foundation only (Type+0xE58 == 0).

### 15f. `VocClass::PlayAtCoord(0)` — the "default" arg

Passes 0 as sound index. The Voc system at `VocClass::PlayAtCoord` globally resolves 0 to whatever "default building destruction" VOC is set — in YR that's `Rules [AudioVisual] BuildingDieSound` (`Rules+0x6E8`). If a TypeClass override is present (`Type+0x520 != 0`) the function is **not** called — see §2e condition.

### 15g. Rules+0x5C8 — unknown threshold

Referenced in §2j as the divisor for a cost-fractional check. Probably `TiberiumToCash` or `StoragePerCell`. Unidentified — see §17.

### 15h. LeaveRubble: parsed but unused

Confirmed in §6a. Adding `LeaveRubble=yes` to a YR building type has no effect. Do NOT implement.

### 15i. CrateBeneath only fires under IC-carryover

§7. Classic gotcha; the crate only appears when the building died during Iron Curtain with remaining duration > 0. Our engine must reproduce this exactly.

### 15j. RevalidateFactoryQueueForKind vs radar

Do NOT confuse `UpdateRadar` at `0x00509140` with the actual radar system. It touches 0 radar fields. It is misnamed in Ghidra.

---

## 16. Sources (all verified from live `gamemd.exe` via Ghidra MCP, 2026-04-24)

| Address | Function | Role |
|---|---|---|
| 0x00442230 | BuildingClass::ReceiveDamage | Entry — case 4 invokes §2 |
| 0x004415F0 | BuildingClass::DestructionEffects (was FUN_, newly named) | vtable+0x4EC, big destruction routine |
| 0x00445880 | BuildingClass::Limbo (aka OnDestroyed) | vtable+0xD4, remove from map |
| 0x00442D90 | BuildingClass::SpawnSurvivors | Crew + garrison + debris |
| 0x00451330 | BuildingClass::GetSurvivorCount (labelled "FUN_/GetWeaponRange_2D0") | vtable+0x2D0 |
| 0x0044EB10 | BuildingClass::GetSurvivorType (labelled "FUN_/GetVoiceResponse") | vtable+0x30C |
| 0x00707D20 | TechnoClass::Crew_Type | Side-based crew pick + 15% Technician |
| 0x00441F60 | BuildingClass::Place_OccupyMap | CrateBeneath trigger in IC-carryover |
| 0x00452A40 | BuildingClass::ConnectWalls | Wall connect/disconnect |
| 0x00480630 | CellClass::PostDestructionWallCleanup | Wall overlay cascade |
| 0x00480CB0 | CellClass::DestroyOverlay | Overlay destruction |
| 0x0050A490 | CleanBasePlanForLostBuilding (FUN_0050A490) | Remove BasePlanNode for this building |
| 0x00509130 | SetSuperWeaponDirty | `Owner->+0x1FB = 1` |
| 0x00509140 | RevalidateFactoryQueueForKind (misnamed "UpdateRadar") | Factory queue check |
| 0x004F42F0 | SetTacticalDirty + bridge counter | Sidebar/tactical dirty |
| 0x004FD150 | HouseClass::Recalc_Base_Center | Center-cell + radius recompute |
| 0x004FF980 | HouseClass::Recount | Per-kind counter decrement |
| 0x006F6AC0 | TechnoClass::Limbo_Helper | Shroud/fog, sound release, HouseClass::Removed_From_Game |
| 0x00457DE0 | BuildingClass::SellBuilding (bunker-eject) | Garrison ejection on death |
| 0x00554A80 | LightSource::Destroy | Ambient light teardown |

Cross-references:
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — parent report (has incorrect offsets for +0x157B / +0x16CB / +0x16CC; corrected here §3b + §6b)
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §6, §17, §19
- `BUILDINGCLASS_VTABLE_FULL_300.md` (slot 53 Limbo; slot 315 = vtable+0x4EC not indexed there — update needed)
- `BUILDINGTYPECLASS_FIELDS.csv` — authoritative for Type offsets
- `HOUSECLASS_VERIFIED_FIELD_MAP.md` — Owner offsets (+0x1E8 SideIndex, +0x5708 BasePlanNodeArray, +0x538C HarvestBonusCount)
- `CRATE_SYSTEM_GHIDRA_REPORT.md` §8.1 — CrateBeneath consumer detail
- `RULESCLASS_FIELDS.csv` — SurvivorRate, SideSurvivorDivisor, AlliedCrew/SovietCrew/ThirdCrew/Engineer/Technician

---

## 17. Open Questions

| # | Question | Confidence | Path to resolve |
|---|---|---|---|
| Q1 | Is Rules+0x5C8 the divisor in the §2j cost-refund fractional test? | LOW | Decompile HouseClass::Add_Credits chain; verify Rules+0x5C8 label |
| Q2 | What does `FUN_0048DED0` (called from §2j) actually do? Score, refund, or neither? | LOW | Decompile; check for Add_Credits / AddScore call |
| Q3 | Confirm `Type+0x16C7` (§2c SensorArray radius teardown) vs `Type+0x16C8` (known SensorArray bool). Are both read here? | MED | Decompile 0x004415F0 line-by-line, correlate with BUILDINGTYPECLASS_FIELDS.csv |
| Q4 | `field_0x6E3` setters — what is the exact semantic? Current hypothesis: "building has no natural crew" | LOW | Grep writes; look at MCV/ConYard/Refinery init |
| Q5 | In §7, confirm that CrateBeneath is GENUINELY only reachable on IC-carryover, or if Unlimbo's initial place-path also fires it for "just-placed" buildings. | MED | Decompile Unlimbo path / Place_OccupyMap callers |
| Q6 | In §2m, the particle system at +0x320 — does it persist after building death (orphan ParticleSystemClass) or get cleaned up? | MED | Trace +0x320 lifetime; check ParticleSystemClass destructor |
| Q7 | Does the "house +0x164/+0x168 are NOT power" correction need to propagate to PowerSystem code? | HIGH (correction is solid) | Implement + audit power report |
| Q8 | Rules+0x5C8 vs `StoragePerCell` — clarify Tiberium leak threshold | LOW | Inspect memory at 0x00***5C8; find assignment in RulesClass ctor |
| Q9 | Why is `BuildingClass::Limbo` (0x445880) labelled "OnDestroyed" in Ghidra but is actually the Conceal/remove-from-map slot? Should we rename? | HIGH (≥90%) | Rename to `BuildingClass::Limbo` to match TC semantics; note "destructive" callers |
| Q10 | Confirm vtable has slots past 300 (the doc claim of "300 slots" is wrong — slot 315 = +0x4EC is used) | HIGH | Read memory 0x7E4400..0x7E4500; update BUILDINGCLASS_VTABLE_FULL_300.md |
