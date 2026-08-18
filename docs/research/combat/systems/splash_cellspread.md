# Splash & CellSpread — Area-Damage Dispatcher

This doc is the canonical reference for **`Apply_area_damage`** at `0x00489280` —
the per-impact AoE dispatcher that:

- Computes maximum radius from `Warhead.CellSpread × 256` leptons
- Collects targets (airborne + ground) within that radius
- Computes per-target distance (with building / aircraft special cases)
- Applies filters: ProtectedFromAOE list, friendly-fire/affects-allies, self-target,
  in-limbo, dead, bridge-infantry tolerance
- Dispatches `ReceiveDamage(raw_damage, distance, wh, ...)` to each surviving target —
  letting the [`damage_formula.md`](damage_formula.md) per-target transform compute
  the actual delivered damage from `(raw_damage, wh, target.armor, distance)`
- Performs warhead-side cell effects: tiberium reduction, overlay destruction, wall
  destruction, sparky push, bridge destruction (low/high), IC barrel chain reaction,
  particle-system spawn

Out-of-scope:
- The per-target damage transform itself → [`damage_formula.md`](damage_formula.md)
- WarheadTypeClass struct layout → [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md)
- `ReceiveDamage` pipeline downstream → [`receive_damage_pipeline.md`](receive_damage_pipeline.md)
- Warhead detonate dispatcher upstream of `Apply_area_damage` → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)
- AnimList / Bright / InfDeath warhead anim selection (separate `detonate` step) → [`animlist_warhead_anim.md`](animlist_warhead_anim.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Address | `0x00489280` |
| Ghidra label | `Apply_area_damage` (named) |
| Signature | `__fastcall(impactCoord: CoordStruct*, baseDamage: int, attacker: TechnoClass*, wh: WarheadTypeClass*, allowTiberiumChain: char, sourceHouse: HouseClass*)` |
| Returns | `bool` — semantics: `!bVar5` where `bVar5` = "any bridge-infantry hit detected." Used by callers as "did this damage actually land somewhere unusual." A second early-return path returns `2` when `wh == Rules.C4Warhead` (self-detonating barrel). |

### Callers (verified live, 19 sites)

```
AnimClass__AI                  @ 00423ac0
AnimClass__Middle              @ 00424ce0
BombClass__Detonate            @ 00438720
DiskLaserClass__AI             @ 004a7340
FUN_0048a700                   @ 0048a700  (likely the wall/cell-damage helper)
FUN_00663030                   @ 00663030  (likely a Rules-driven AoE trigger)
FUN_006e0490                   @ 006e0490  (likely TechnoClass damage applicator)
FUN_006e2390                   @ 006e2390  (likely a fire/anim damage tick)
FlyLocomotionClass__Process    @ 004cd600
InfantryClass__PerCellProcess  @ 00519630
LightningStorm__GroundStrike   @ 0053a300
NukeGroundZero__ApplyDamage    @ 004251f0
PsychicDominator__MindControlArea @ 0053b080
SuperClass__Launch             @ 006cc390
TerrainClass__Take_Damage      @ 0071b920
VoxelAnimClass__AI             @ 00749f30
WarheadTypeClass__Detonate     @ 004690b0
Wave_splash_forces             @ 0053cbe0
Apply_area_damage              @ 00489280  (recursive self-call for IC-barrel chain)
```

This is **the central AoE dispatcher** in gamemd.exe. Every projectile impact,
animation tick that applies damage, superweapon ground-effect, deathweapon, and
ambient-damage tick funnels through here.

### Confidence

- **Content: HIGH** (function decompiled live 2026-05-17; matches existing canonical doc §6 of `DAMAGE_MATH_GHIDRA_REPORT.md`).
- **Identity: HIGH** (named function; signature inferred from caller-side packing matches the 6-argument fastcall convention).
- **Binding: HIGH** — 19 verified callers via `get_function_callers 0x00489280`. Every combat-impact path in YR funnels here.

---

## 2. Inputs

| Param | Decomp name | Meaning |
|---|---|---|
| 1 | `param_1` | `CoordStruct* impactCoord` — impact in leptons (X, Y, Z) |
| 2 | `param_2` | `int baseDamage` — raw weapon damage AFTER attacker-side modifiers (firepower / vet / country / gattling / bunker / deploy) — see [`fire_at_pipeline.md`](fire_at_pipeline.md) |
| 3 | `param_3` | `TechnoClass* attacker` — who fired (NULL for ambient sources like death-weapon residual) |
| 4 | `param_4` | `WarheadTypeClass* wh` — warhead to detonate |
| 5 | `param_5` | `char allowTiberiumChain` — if non-zero, this call MAY reduce tiberium in affected cells |
| 6 | `param_6` | `HouseClass* sourceHouse` — house responsible for the damage (for kill credit) |

### Early-out (verified)

```
if baseDamage == 0  ||  scenarioFlag & 0x20  ||  wh == NULL:
    return true
```

The `0x20` scenario flag is the same global "no damage" gate cited in [`damage_formula.md`](damage_formula.md) §3.

---

## 3. Max radius

```
maxRadius_leptons = ftol(wh->CellSpread * 256.0)        // wh+0x124 (float), 256 leptons/cell
```

For a `CellSpread=0` warhead, `maxRadius = 0`. The cell-scan loop in §5 will still run
once for the impact cell itself (because `DAT_007ed3d0[0]` is 1 — the impact cell
counts), but most targets fail the `distance <= 0` final check unless they are
**dead-center on the impact**. Net effect: zero-spread = single-target hit on whatever
is at the impact point.

---

## 4. Bridge-infantry tolerance precheck (`bVar5` gate)

```
bridgeTolerance = (wh->CellSpread > DAT_007e5168)        // threshold (existing doc says 0.5f)
```

If `CellSpread > 0.5`, the function enables "bridge infantry tolerance" mode: it will
later detect whether the impact has bridge-deck infantry within 0x55 leptons and flag
the dispatch to **only damage bridge infantry**, not ground units. This is the
mechanism behind "anti-bridge bombs only hit deck units, not ground beneath."

The threshold constant `DAT_007e5168` reads as `0.5f` per existing canonical doc.

---

## 5. Airborne target collection (if impact above ground)

```
if impactCoord.Z > CellClass.GetGroundHeight(impactCoord.cell):
    // Impact is above the cell's ground — look at airborne objects too
    cellList = MapClass.GetAirCellsInRadius(impactCoord, maxRadius)
    for each object in cellList (via FUN_004137a0 iterator):
        if !object.IsAlive:              continue   // (piVar12[0x24] != 0)
        if !object.IsOnMap:              continue   // (piVar12[0x1d] != 0)
        if object.Health <= 0:           continue   // (0 < piVar12[0x1b])
        dist = CoordStruct.Distance3D(impactCoord, object.coords)
        if dist > maxRadius_leptons:     continue

        // Bridge-tolerance check: small-radius hits on bridge-deck infantry flag bVar5
        if bridgeTolerance && dist < 0x55 && object.IsOnBridge() && !object.InLimbo:
            bVar5 = true

        append {object, dist} to damage_vector
```

This is the air-path. It is run **only when the impact is above ground level** —
typically for missile detonations, falling debris, airburst clusters.

`piVar12[0x24]`, `piVar12[0x1d]`, `piVar12[0x1b]` correspond to ObjectClass fields:
- `+0x90` (= `int* index 0x24 × 4`): `IsAlive` flag (verified)
- `+0x74` (= `0x1d × 4`): `IsAirborne` / `IsOnMap` gate
- `+0x6C` (= `0x1b × 4`): `Health`

---

## 6. Ground cell scan (the main loop)

Uses two static tables:

| Address | Contents |
|---|---|
| `DAT_007ed3d0[spread_index]` | int count: how many cells to scan for `CellSpread = spread_index` |
| `DAT_00abd490` | X-delta table (short) |
| `DAT_00abd492` | Y-delta table (short, interleaved with X table at stride 4 bytes per `{dx,dy}` pair) |

```
spread_index = ftol(wh->CellSpread)                     // 0, 1, 2, 3, ...
cell_count   = DAT_007ed3d0[spread_index]

for i in 0 .. cell_count:
    dx = DAT_00abd490[i * 2]
    dy = DAT_00abd492[i * 2]
    cellCoord = (impact.cellX + dx, impact.cellY + dy)
    cell = MapClass.GetCellClass(cellCoord)

    // ── Cell-side warhead effects (per scanned cell) ──
    if cell.OverlayTypeIndex != -1:
        overlay = OverlayTypeClass[cell.OverlayTypeIndex]
        // 1. Tiberium reduction
        if overlay.byte+0x2B1 (ChainReaction) != 0
           && (overlay.byte+0x2A9 (Tiberium) == 0 || wh->Tiberium != 0)
           && allowTiberiumChain:
            CellClass.Reduce_Tiberium(cell)
        // 2. Overlay destruction (walls, etc.)
        if overlay.byte+0x2A8 (Wall) != 0
           && (wh->Wall || wh->WallAbsoluteDestroyer ||
               (wh->Veinhole && overlay.armor == 6 /*wood*/)):
            CellClass.DestroyOverlay(cell)
        if cell.OverlayTypeIndex == -1:
            TechnoClass.StopAllTargeting()                   // forces unit deselection of dead overlay

    // ── Object iteration for damage ──
    if bridgeOccupant_mode (uStack_cc._2_1_):
        objList = cell.BridgeOccupants     // cell+0x3a (= cell+0xE8)
    else:
        objList = cell.GroundOccupants     // cell+0x39 (= cell+0xE4)

    for obj in linked-list at objList (next = obj[0xc] = obj+0x30):
        // Self-attacker filter
        if obj == attacker
           && !attacker.Type.IsSelfHealing (TypeClass+0xCA0)
           && wh != Rules.C4Warhead (Rules+0xFAC):
            continue                                  // skip self unless self-healing or C4

        if !obj.IsAlive:                              // obj[0x24] != 0
            continue

        // ProtectedFromAOE filter (scenario flag 0x800)
        if obj.WhatAmI() == 1                         // ground unit
           && (scenarioFlag & 0x800) != 0:
            objType = obj.GetType()                   // vtable+0x88
            for pType in Rules.ProtectedFromAOE[0 .. Rules.ProtectedFromAOE_Count]:
                // Rules+0xB40 = pointer-list, Rules+0xB4C = count
                if pType == objType: skip-this-object (goto LAB_004899b3)

        // Build {obj, dist} record
        if obj.WhatAmI() == 6:                        // Building
            if cell != impactCell:
                // Use cell-center coords from cell.Get_Coords (vtable+0x48)
                dist = Sqrt_Approx(dx² + dy² + dz²)
            else if impact.Z - cell.Z > DAT_0089e870 * 2:
                // Same cell, but impact way above cell — subtract 2*CellHeight from dist
                dist = Sqrt_Approx(dx² + dy² + dz²) - 2 * DAT_0089e870
            else:
                dist = 0
        else:
            // Non-building: use object's coordinates (vtable+0xA4 = Get_Coord_BUT_NEW_BUFFER)
            dist = Sqrt_Approx(dx² + dy² + dz²)

        // Bridge-infantry detection on the impact cell
        if bridgeTolerance && i == 0 && obj != NULL
           && (obj.byte+0x14 bit 0x1) != 0       // IsOnBridge
           && obj.IsOnBridge() (vtable+0x160)
           && !obj.InLimbo (obj[0x71] == 0)
           && dist < 0x55:
            bVar5 = true

        append {obj, dist} to damage_vector
```

### Cell-occupant order

Within one cell, occupants are scanned **in the order they appear in the cell's
linked list** (ground at `cell+0xE4`, bridge at `cell+0xE8`). Append-order is preserved
into the damage_vector. This means side-effect ordering (e.g., who explodes first) is
deterministic per cell, but matches the engine's cell-occupation order — see
[`../../CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`](../../CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md).

### Building distance special case (verified)

Buildings present a unique challenge: their bounding box is multi-cell. The function
handles three sub-cases:

1. **Building in a non-impact cell** (the most common case): use the cell's
   `Get_Coords` (vtable+0x48) center point — gives the cell-center distance, not the
   building's anchor distance.
2. **Building in the impact cell AND impact is high above** (`impact.Z - cell.Z > 2 × DAT_0089e870`): use 3D distance to cell center, then subtract `2 × DAT_0089e870` (= `2 × CellHeight`).
3. **Building in the impact cell, low impact**: `dist = 0`.

`DAT_0089e870` is the LevelHeight constant (= 104 leptons per RA2 convention,
confirmed at `COORDINATE_SYSTEM_GAMEMD.md`).

---

## 7. Aircraft distance halving (verified)

In the damage-dispatch loop:

```
for {obj, dist} in damage_vector:
    if obj.WhatAmI() == 2 && obj.IsHighFlying():     // Aircraft in air (vtable+0x54)
        dist = dist / 2
    if obj.Health > 0 && obj.IsOnMap && !obj.InLimbo && dist <= maxRadius:
        obj.ReceiveDamage(&baseDamage, dist, wh, attacker, false, false, sourceHouse)
```

**Aircraft in flight take half-distance damage** — equivalently, splash effectively
reaches them at half the lepton-cost. This is a parity-critical detail. Active in YR.

The `obj.ReceiveDamage` call is `vtable+0x16C` per existing canonical doc — same as
the direct-fire damage path (see [`receive_damage_pipeline.md`](receive_damage_pipeline.md)).

**Important:** the SAME `baseDamage` is passed to every target's `ReceiveDamage`,
along with each target's individual `dist`. The CellSpread falloff is computed **inside
`GetDamage` (FUN_00489180)**, not here — see [`damage_formula.md`](damage_formula.md) §5.

---

## 8. C4Warhead self-targeting gate (Rules+0xFAC)

```
bVar21 = (wh == *(int*)(Rules + 0xFAC))           // is this the C4Warhead?
```

The self-attacker filter normally skips the attacker as a target. But:
- If `wh == Rules.C4Warhead`, the attacker IS a valid target (a barrel/IC node detonating itself).
- Or if the attacker's type has `IsSelfHealing=yes` (`TypeClass+0xCA0`), the attacker is allowed as a target.

After the cell scan loop, if `bVar21` is true, the function returns `2` early
(skipping the bridge-destruction / sparky / particle phases). This is the "I detonated
into myself, all done" path.

`Rules+0xFAC` is the `C4Warhead=` Rules key (separate from `Rules+0xFA8` which is the
*recursive call* C4Warhead used for IC barrel chains — see §11).

---

## 9. ProtectedFromAOE filter

Triggered only when `(scenarioFlag & 0x800) != 0`. This is the
"ProtectedFromAOE=yes-list-active" flag — used for certain campaign / scripted
contexts. The unit-types in `Rules.ProtectedFromAOE[]` (pointer list at `Rules+0xB40`,
count at `Rules+0xB4C`) are immune to AoE damage when this flag is set.

In retail multi-player skirmish, this flag is typically clear → the filter never
activates. The TODO list `Rules.ProtectedFromAOE=` is parsed from `[CombatDamage]` /
`[General]` and is normally empty for skirmish maps.

### TS-legacy filter

**Status: PROBABLY active in YR campaign play; INACTIVE in standard skirmish.** The
flag `0x800` is a per-scenario gate that maps/campaigns can set. Default in skirmish
INIs is clear. Flag for follow-up if a player observes the difference.

---

## 10. Sparky push (post-dispatch)

After all targets receive damage:

```
// Compute sparky intensity (uStack_68)
intensity = uStack_cc * 0.01                  // uStack_cc = damage_vector_count or similar
if intensity >= 4.0: intensity = 4.0

if wh->Sparky (wh+0x14E) && intensity > DAT_007e5138:
    for each cell in (impact +- 3, impact +- 3):
        for each obj in cell.occupants:
            if (cell == impactCell && attacker != NULL):
                // Push obj away from attacker, scaled by intensity
                direction = normalize(attacker - obj)
                pushVector = direction * BridgeDiag_NonBridge_10_0
                obj.LocationChange(obj.coords + pushVector, intensity)
            else if wh->CellSpread > FLOAT_007e1748:
                obj.LocationChange(impactCell, intensity)
```

The push uses `vtable+0x3D8` on each obj (likely `LocationChange` / `Push` /
`Hit_Mark`-style). The constant `_g_BridgeDiag_NonBridge_10_0` is 10.0f — the
push-distance scalar.

Sparky activates for warheads with `Sparky=yes` at `wh+0x14E`. Used by Demo Truck
warhead, Tesla nearby-zaps, and a few other "shake the ground" effects.

---

## 11. IC barrel chain reaction (recursive Apply_area_damage)

After the dispatch + sparky steps, if the impact cell has an **IC overlay** (offset
`+0x2B0` on the OverlayTypeClass is "IsIC" / "chain explosive"):

```
if cell.OverlayTypeIndex != -1
   && OverlayType[cell.OverlayTypeIndex].byte+0x2B0 != 0:    // chain-explosive
    FUN_00486e70()                                            // un-set overlay
    cell.OverlayTypeIndex = -1
    cell.RecalcAttributes()
    MapClass.AssignOrphanedCellZone()
    FUN_00584550()                                            // re-pathfind
    TechnoClass.StopAllTargeting()

    // Spawn smoke animation
    new AnimClass(Rules.IConSmokeAnim (Rules+0x54), cell, 0, 1, 0x600, 0, 0)

    // RECURSIVE: detonate a C4Warhead at this cell
    Apply_area_damage(NULL, Rules.C4Warhead (Rules+0xFA8), 1, sourceHouse)
    // Note: the recursive call uses Rules+0xFA8, the secondary C4Warhead pointer
    //       (Rules+0xFAC is the self-target-allowed C4Warhead — likely the same WH)

    // Spawn 15%-chance debris voxel
    for i in 0 .. Rules.DebrisCount (Rules+0x68):
        if random(0..99) < 15:
            new VoxelAnimClass(Rules.DebrisVoxelArray[i] (Rules+0x5C + i*4), cell, 0)
            break

    // Spawn 25%-chance smoke particle
    if random(0..99) < 25:
        new ParticleSystemClass(Rules.IConSmokeParticleSys (Rules+0x74), cell, ...)
```

This is the IC-barrel / explosive-overlay chain reaction. Used by red explosive
barrels in skirmish maps — one barrel hit chains to neighbors via this recursive
call. The recursion depth is naturally bounded by the spatial range of `C4Warhead`'s
CellSpread.

### Confidence

- **Content: HIGH** (decomp shows literal recursive `Apply_area_damage(0, *Rules+0xFA8, 1, param_6)`).
- **Identity: HIGH** (named overlay flag `+0x2B0` on OverlayTypeClass; matches conventional `OverlayType.Explosive` parsing).
- **Binding: HIGH** (the cell-overlay check fires every match where a barrel is present and hit).

---

## 12. Bridge destruction (scenario flag 0x8000)

Only triggered when `(scenarioFlag & 0x8000) != 0` AND `wh->Wall != 0`. The function
checks two iso-tile-index ranges for bridge tiles and an overlay-index range:

| Check | Range | Caller |
|---|---|---|
| Low bridge tiles | iso = `DAT_00abad30` etc. or `DAT_00aa1028` etc. | `ApplyDamageToCell` |
| High bridge tiles | iso = `DAT_00abad1c +` matches | `ApplyDamageToCell` |
| Low bridge overlay | overlay index `0x4A .. 0x63` (74..99) | `DestroyBridge_Low` |
| High bridge overlay | overlay index `0xCD .. 0xE6` (205..230) | `DestroyBridge_High` |

For each, the destruction check requires:

```
if wh == Rules.NUKE_Warhead_or_similar (Rules+0xFF0):
    always destroy
else:
    if random(1..Rules.BridgeStrength (Rules+0x1740)) < intensity:
        destroy
```

Where `Rules+0x1740` = `[CombatDamage] BridgeStrength=` (default ~10 — verify).

### Random damage retries

When the check passes, `ApplyDamageToCell()` is called up to 4 times (4 attempts) to
ensure destruction. The retry loop:

```
attempts = 3
while !DestroyBridge_X() && attempts > 0:
    if wh == Rules+0xFF0 (force-destroy WH): retry
    attempts--
```

Force-destroy warheads (`wh == Rules+0xFF0`) get unlimited retries.

---

## 13. Pre-spawned warhead particle system

At the very end, if `wh->Particle != NULL` (`wh+0x140`):

```
new ParticleSystemClass(wh.Particle, impactCell, ...)
```

This is the warhead's INI-configured `Particle=` (e.g., smoke trails, dust clouds)
spawned at the impact point. Independent of the IC-barrel chain particle.

---

## 14. Constants used

| Address | Meaning | Notes |
|---|---|---|
| `wh+0x124` | `CellSpread` (float) | cells |
| `wh+0x140` | `Particle` (ParticleSystemTypeClass*) | warhead INI |
| `wh+0x144` | `Wall` | bool |
| `wh+0x145` | `WallAbsoluteDestroyer` | bool |
| `wh+0x147` | (unknown bool — overlay-vs-wood gate) | bool, used with veinhole+wood-armor |
| `wh+0x148` | `Tiberium` | bool |
| `wh+0x14E` | `Sparky` | bool |
| `wh+0x179` | `AffectsAllies` | bool — used downstream in ReceiveDamage |
| `Rules+0x54` | IC-barrel smoke `AnimClass*` | |
| `Rules+0x5C` | IC-barrel debris voxel-array | |
| `Rules+0x68` | `DebrisCount` int | |
| `Rules+0x74` | IC-barrel particle-system | |
| `Rules+0xB40` | ProtectedFromAOE list | pointers |
| `Rules+0xB4C` | ProtectedFromAOE count | int |
| `Rules+0xFA8` | C4Warhead (recursive call) | WarheadType* |
| `Rules+0xFAC` | C4Warhead (self-target gate) | WarheadType* |
| `Rules+0xFF0` | Force-destroy warhead (likely NUKE) | WarheadType* |
| `Rules+0x1740` | `BridgeStrength` (random max) | int |
| `DAT_007e5168` | bridge-tolerance threshold (= `0.5f`) | |
| `DAT_007e5138` | sparky-intensity threshold | |
| `DAT_007e3cc8` | intensity cap (= 4.0) | |
| `DAT_0089e864` | bridge-Z offset | |
| `DAT_0089e870` | LevelHeight (= 104) | |
| `DAT_007ed3d0[]` | per-spread-index cell-count table | |
| `DAT_00abd490/492` | per-spread cell-offset table (X/Y interleaved) | |
| `_g_BridgeDiag_NonBridge_10_0` | sparky push distance (= 10.0f) | |
| scenarioFlag `0x20` | global no-damage | |
| scenarioFlag `0x800` | ProtectedFromAOE active | |
| scenarioFlag `0x8000` | bridge-destruction active | |
| `OverlayType+0x2A8` | `Wall=` (IsWall) | bool |
| `OverlayType+0x2A9` | `Tiberium=` (IsTiberium) | bool — **corrected 2026-05-17** (was "Veinhole_like" — wrong) |
| `OverlayType+0x2B0` | `Explodes=` (IC barrel chain) | bool |
| `OverlayType+0x2B1` | `ChainReaction=` | bool — **corrected 2026-05-17** (was "IsTiberium" — wrong); see [`chain_reaction.md`](chain_reaction.md) §1 |
| `OverlayType+0x9C` | Armor (for veinhole+wood check) | int |
| `TechnoType+0xCA0` | `IsSelfHealing` | bool — gates self-target |

---

## 15. TS-legacy filter

- **ProtectedFromAOE (§9):** gated on scenarioFlag `0x800`. INACTIVE in standard skirmish, ACTIVE for campaign/scripted maps that set the flag. Not pure TS-legacy.
- **Bridge destruction (§12):** gated on scenarioFlag `0x8000`. ACTIVE in all standard YR play (maps with bridges enable it via mission INI). Confirmed live.
- **Sparky push (§10):** active in YR — driven by warhead `Sparky=yes` (`wh+0x14E`). Demo Truck warhead, Tesla Coil zap-area, etc.
- **IC barrel chain (§11):** active in YR. Red explosive barrels in skirmish maps trigger this.
- **Veinhole overlay check (§6 cell-side):** the `wh->Veinhole` flag (`wh+0x17B`) and the `overlay.armor == 6 /*wood*/` combination is **TS-legacy** — Veinholes are TS-only. The check fires only for warheads with `Veinhole=yes`, which no shipping YR warhead sets.

---

## 16. Edge cases

| Case | Behavior |
|---|---|
| `wh->CellSpread = 0` | maxRadius=0; only objects at exact impact point take damage (if any). |
| `attacker == NULL` | Allowed — used for ambient/death-weapon paths. ProtectedFromAOE filter still applies if scenarioFlag 0x800 set. |
| `attacker` is also a target (overlap with damage_vector) | Skipped unless `wh == Rules.C4Warhead` OR `attacker.Type.IsSelfHealing == true`. |
| Target is in limbo (`+0x6C IsAlive=true` but `+0x71 InLimbo=true`) | Skipped in final dispatch (`piVar20+0x81 == 0` check). |
| Target.Health <= 0 | Skipped (`piVar20[0x1B] > 0` check). |
| Aircraft in flight | Distance HALVED before final compare. |
| Building in impact cell, low impact | Distance = 0 (always hit). |
| Building in impact cell, very high impact | Distance subtracted by 2×LevelHeight (gives the building some elevation immunity). |
| Bridge infantry on impact cell + small CellSpread (>0.5f) | `bVar5` flag set → damage only goes to bridge infantry, ground occupants on the cell unaffected. |
| `wh == Rules.C4Warhead (FAC)` | Function returns `2` after dispatch, skipping bridge/sparky/particle phases (this is the recursive-IC-barrel termination path). |

---

## 17. Open follow-ups

1. **Rules-offset INI-key identification.** The doc lists 11 Rules offsets. Of these, `+0xB40/+0xB4C` (ProtectedFromAOE), `+0xFA8/+0xFAC` (C4Warhead), `+0xFF0`, and `+0x1740` are referenced by the function but their INI-key strings haven't been traced in this pass. Priority: MEDIUM — needed for full reproducibility of the cell-effect phase from INI alone.
2. **`wh+0x147` unknown bool.** Used at the overlay-destruction check alongside `Veinhole` flag. Existing canonical `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` lists `+0x147` as `(unknown bool)`. Priority: LOW (no shipping warhead sets it).
3. **`DAT_007ed3d0[]` cell-count table contents.** The table has at least 11 entries (CellSpread 0..10) per existing doc, but values not enumerated here. Priority: MEDIUM — needed to know exactly which cells are scanned for which CellSpread.
4. **`DAT_00abd490/492` offset tables full dump.** Similar — needed for cell-scan reproduction. Priority: MEDIUM.
5. **Sparky intensity formula precise derivation.** `uStack_cc * 0.01` then capped at 4.0 — `uStack_cc` is the damage_vector_count or similar; precise source unclear. Priority: LOW.
6. **`vtable+0x3D8` identity.** The push function called for sparky. Likely `LocationChange` or `Apply_Force`. Priority: LOW.
7. **Bridge cell-overlay range thresholds.** Document says overlay `0x4A..0x63` = low bridge, `0xCD..0xE6` = high bridge. Verified per existing canonical doc; not independently re-verified this pass. Priority: LOW.

---

## 18. Sources

- Live decompilation of `Apply_area_damage` at `0x00489280` (read 2026-05-17).
- Live caller list via `get_function_callers 0x00489280` (19 sites, 2026-05-17).
- Existing canonical doc: [`../../DAMAGE_MATH_GHIDRA_REPORT.md`](../../DAMAGE_MATH_GHIDRA_REPORT.md) §6 — content cross-verified; this doc supersedes for AoE dispatch specifically.
- WarheadTypeClass struct: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md).
- Cell ordering: [`../../CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`](../../CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md).
- Sister system docs cross-referenced for downstream/upstream phases (damage_formula, receive_damage_pipeline, warhead_detonate_dispatch, animlist_warhead_anim, fire_at_pipeline).
