# Bunker System — Ghidra Research Report

**Scope:** The `Bunker=yes` building type (Battle Bunker / Tank Bunker) and its
associated fields, state machine, combat multipliers, and sounds in gamemd.exe.

**Status in YR:** **Live.** Not Tiberian Sun legacy.
- `BunkerDamageMultiplier`, `BunkerROFMultiplier`, `BunkerWeaponRangeBonus` are actively
  tuned in `rulesmd.ini`.
- Soviet Siege Bunker (`NABNKR`) appears in `aimd.ini` taskforces (AI builds and uses them).
- Tank Bunker (`NATBNK`) ships with full art including 8 damaged-state animation slots.
- Per-unit `Bunkerable=yes/no` is used throughout `rulesmd.ini`.
- State machine at `0x00458E50` is reached via the standard
  `BuildingClass::MissionRepairAndProduce` dispatcher — not gated by `SpecialFlags`.

**Relationship to civilian garrison (`CanBeOccupied`):** Distinct system. Shares
`RulesClass` float group layout but uses a different flag on the building type,
a different flag on the unit type, a single-slot storage instead of a vector,
and a multi-state entry animation machine.

This report distinguishes **verified** (read directly from the binary this session)
from **inferred / sourced from existing reports**.

---

## 1. INI Keys and Storage Offsets

### BuildingTypeClass — the `Bunker=yes` flag

| Offset  | INI Key  | Type | Source                                          | Confidence |
|---------|----------|------|-------------------------------------------------|------------|
| `+0x16AB` | `Bunker` | bool | String `0x0081AADC` → xref `BuildingTypeClass_ReadINI_Water` at `0x00460941` | **HIGH — verified xref** |

Corroborates existing entries in `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md:173`,
`BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md:14`, and `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md:31`.

### TechnoTypeClass — the `Bunkerable=yes` flag

| INI Key      | Read in                           | Source                              | Confidence                |
|--------------|-----------------------------------|-------------------------------------|---------------------------|
| `Bunkerable` | `TechnoTypeClass::ReadINI`        | String `0x0084371C` → xref `0x0071500A` | MEDIUM — parsed, exact type-class offset not read this session |

`Bunkerable` defaults to `yes` for unit types (per in-INI comments) and `no` for
other techno types. Applies to *which units can be garrisoned into a Battle Bunker*.

### RulesClass — combat multipliers (CombatDamage section)

Verified directly from `RulesClass::ReadCombatDamage` at `FUN_0066BBB0`.
The Bunker triple sits immediately after the garrison triple:

| Offset  | INI Key                     | Type  | Read at | Meaning |
|---------|-----------------------------|-------|---------|---------|
| `+0xF40` | `OccupyDamageMultiplier`    | float | `0x0066C68F` | garrison (for contrast) |
| `+0xF44` | `OccupyROFMultiplier`       | float | `0x0066C6B4` | garrison (for contrast) |
| `+0xF48` | `OccupyWeaponRange`         | int   | `0x0066C6D4` | garrison (for contrast) |
| `+0xF4C` | `BunkerDamageMultiplier`    | float | `0x0066C6F9` | **bunker damage scale** |
| `+0xF50` | `BunkerROFMultiplier`       | float | `0x0066C71D` | **bunker ROF divisor** |
| `+0xF54` | `BunkerWeaponRangeBonus`    | int   | `0x0066C73D` | **bunker range (cells, additive)** |
| `+0xF58` | `OpenToppedDamageMultiplier`| float | `0x0066C761` | open-topped (for contrast) |

All three are confirmed reads in `RulesClass::ReadCombatDamage` — **HIGH confidence**.

Note the semantic difference vs garrison:
- `OccupyWeaponRange` **replaces** range, `BunkerWeaponRangeBonus` **adds** to it
  (confirmed in `TechnoClass::InRange`, §5 below).
- `OccupyROFMultiplier` and `BunkerROFMultiplier` are **both divisors** applied to
  the base ROF (confirmed in `TechnoClass::GetROF`, §4 below).

### RulesClass — sounds (AudioVisual section)

Verified from `RulesClass::ReadAudioVisual` at `0x00669E87`:

| Offset  | Int-Index      | INI Key                | Source | Confidence |
|---------|----------------|------------------------|--------|------------|
| `+0x240` | `param_1[0x90]` | `BunkerWallsUpSound`   | String `0x0083A828` | **HIGH — verified** |
| `+0x244` | `param_1[0x91]` | `BunkerWallsDownSound` | String `0x0083A810` | **HIGH — verified** |

`BunkerWallsUpSound` is stored as a `VocClass` sound index (resolved via `VocClass::FindByName`
at parse time); `-1` means "not set, don't play". The gate check `if (g_RulesClass + 0x240 != -1)`
appears in the state machine (§3, state 5).

---

## 2. BuildingClass / TechnoClass Runtime Fields

### BuildingClass (contained-unit side)

| Offset  | Name                         | Type       | Meaning                                    |
|---------|------------------------------|------------|--------------------------------------------|
| `+0x2E4` | `BunkerLinkedUnit`           | `FootClass*` | **Single** contained unit pointer (not a vector) |
| `+0x718` | `BunkerState`                | int        | State machine slot (0..6)                  |

Confirmed by direct reads in the state machine:
- `*(int **)&param_1->field_0x2e4 = piVar5;` → building stores unit pointer
- `*(undefined4 *)&param_1->field_0x718 = <state>;` → state transitions

### TechnoClass / FootClass (contained unit's side)

| Offset  | Int-Index       | Name                      | Meaning                              |
|---------|-----------------|---------------------------|--------------------------------------|
| `+0x2E4` | `param_1[0xB9]` | `BunkerLinkedBuilding`    | Back-reference to containing building |
| `+0x214` | `param_1[0x85]` | (cleared to -1 on install) | Unknown companion field, possibly target index |

The unit's `+0x2E4` field is the **"in-bunker" tag** used by combat code (§§4-6).
Non-zero means "this unit is currently inside a bunker." Note: byte offset `0x2E4`
is identical on both classes — they are distinct fields on different classes that
happen to share the same offset (observed fact; no claim made about why).

**Pointer-arithmetic caveat (per CLAUDE.md):** the state machine treats `piVar5`
as `int *`, so `piVar5[0xb9]` is byte offset `0xB9 × 4 = 0x2E4`. In
`TechnoClass::GetROF` (`param_1` is `int *`), the same field is read as `param_1[0xb9]`.
In `TechnoClass::InRange` (`param_1` is `int *`), same pattern.

---

## 3. Entry State Machine — `FUN_00458E50`

**Address:** `0x00458E50`
**Callers:** only `BuildingClass::MissionRepairAndProduce` at `0x0044B780`
(verified via `get_function_callers`).
(corrected 2026-05-29: was `0x0044B7A8`; `get_function_by_address 0x0044B780` confirms entry `0x0044B780`; `0x0044B7A8` is a stack-pushed code-offset value inside the prologue, not the function entry — GHIDRA_ADDRESS_SHIFT)
**Dispatch:** the first branch in `MissionRepairAndProduce` (before CY, Hospital,
Armory, UnitRepair, UnitReload):
```c
if (param_1->Type[0x16ab] != '\0') {
    FUN_00458e50();   // Bunker state machine
}
```
**Not behind `SpecialFlags`** — always runs when building has `Bunker=yes`.

### Preflight
```c
piVar5 = building->field_0x2e4;              // linked unit
if (piVar5 == NULL)
    piVar5 = FootClass::GetDestination(0);   // fall back to docking-queue target
if (piVar5 == NULL || piVar5->WhatAmI() != 1) {  // 1 = UnitClass
    building->field_0x718 = 0;                // reset state
    building->MissionSet(5, 0);               // vtable+0x1E8, mission 5 (likely Guard)
    return;
}
```

### State table

| State | Purpose                                         | Exit condition                                                                 |
|-------|-------------------------------------------------|--------------------------------------------------------------------------------|
| **0** | Verify unit has arrived at building cell; shove nearby objects | unit's cell == this building → fallthrough; else remain                  |
| **1** | Scan foundation exits via `building.vtable+0x108`; compute `atan2` facing angle from unit→building; start `RateTimer` | unreachable exit list end → state 2                                       |
| **2** | Wait for timer; select approach anim frame from angle (4/6/8/5 variants at ~0x43..0x46); call `unit.vtable+0x544(0, 0x3FF00000)` (hide?) | timer elapsed → state 3                                                 |
| **3** | Re-verify unit at building cell; start `CDTimer(0x8000)` | unit at cell → state 4                                                       |
| **4** | Play entry animation via `CreateAnimForSlot` — slot varies by health ratio vs `RulesClass+0x1700` (`ConditionRed`): healthy uses `Type+0x11F4` / `+0x1238`, damaged uses `Type+0x1204` / `+0x1248` | timer elapsed → state 5                                                  |
| **5** | **Install unit:** `building.field_0x2E4 = unit`, `unit.field_0x2E4 = building`, `unit.field_0x214 = -1`, call `unit.vtable+0x150` (Limbo/hide), state=6, `unit.MissionSet(5, force=1)`. Play `BunkerWallsUpSound` at building location if index != -1 | terminal                                                                |

### Animation slot offsets on BuildingTypeClass

Observed as conditional art-slot reads inside state 4:

| Offset     | Meaning                                                  |
|------------|----------------------------------------------------------|
| `+0x11F4`  | Bunker entry anim, healthy variant (primary slot)        |
| `+0x1204`  | Bunker entry anim, damaged variant (primary slot)        |
| `+0x1238`  | Bunker entry anim, healthy variant (secondary slot)      |
| `+0x1248`  | Bunker entry anim, damaged variant (secondary slot)      |

Sibling branches in `MissionRepairAndProduce` (ConstructionYard, UnitRepair, etc.)
use the same health-gated slot pattern at different offsets. The same
`ConditionRed` threshold (`RulesClass+0x1700`) is used throughout.

### Mission value on install

The unit is put into mission `5` with force-flag `1` (`unit.vtable+0x1E8(5, 1)`).
In the YR mission enum, `5` is most commonly Guard/Sleep. **Not verified against
the mission enum table this session** — marked MEDIUM.

---

## 4. ROF Application — `TechnoClass::GetROF` (0x006FCFA0)

Verified directly. The Bunker ROF divisor is applied at the end of the ROF chain,
independently of the occupant (garrison) divisor earlier in the function:

```c
// After veterancy + occupant-count division + OccupyROFMultiplier:
if (param_1[0xb9] != 0) {                         // IsInBunker (unit.field_0x2E4)
    int whatami = param_1->vtable_0x2c();          // WhatAmI()
    if (whatami != 6                               // not a Building
        && *(float*)(RulesClass + 0xF50) != 0.0f)  // BunkerROFMultiplier != 0
    {
        rof = ftol((float)rof / BunkerROFMultiplier);
    }
}
```

**Semantics:** divide base ROF by `BunkerROFMultiplier` when the firer is a non-building
TechnoClass with an active bunker back-reference. Building self-shooting is excluded
by the `WhatAmI() != Building` guard.

Confidence: **HIGH** — verified from decompilation this session.

---

## 5. Range Application — `TechnoClass::InRange` (0x006F7220)

Verified directly. Bunker range is **additive**, unlike garrison which replaces:

```c
iVar8 = weapon->Range;                                  // weapon+0xB4
if (weapon->Range == -0x200) return true;               // always-in-range sentinel

if (target->IsInAir())
    iVar8 += this->Type->AirRange;                      // type+0x68C

if (this->IsOccupied()) {                               // vtable+0x400 (garrison)
    iVar8 = (this->GetOccupantCount() + RulesClass->OccupyWeaponRange) * 256;
    // REPLACES range with garrison formula
}

if (param_1[0xB9] != 0 && WhatAmI() != Building) {      // IN BUNKER
    iVar8 += RulesClass->BunkerWeaponRangeBonus * 256;   // ADDS cells × 256 leptons
}

if (this->field_0x82 != 0) {                            // IN OPEN-TOPPED TRANSPORT
    iVar8 += RulesClass->OpenToppedRangeBonus * 256;
}
```

**Key differences from garrison:**
- Garrison path **overwrites** `iVar8` — original weapon range is discarded and replaced
  by `(occupant_count + OccupyWeaponRange) × 256`.
- Bunker path **adds** `BunkerWeaponRangeBonus × 256` (cell units, 256 leptons per cell).
- Bunker and OpenTopped can stack (additive), but Garrison is mutually exclusive
  with both (it overwrote the base first).

Confidence: **HIGH** — verified from decompilation this session.

---

## 6. Damage Application

Sourced from `DAMAGE_MATH_GHIDRA_REPORT.md:654-656`:

```c
// Inside Fire_At damage accumulation:
if (this->IsInBunker && WhatAmI() != Building)
    damage = ftol((float)damage * occupantBunkerMult);
```

where `occupantBunkerMult` = `RulesClass->BunkerDamageMultiplier` (`+0xF4C`, verified
offset). Same guard pattern as ROF and range (non-building + bunker back-reference).
Applied as a **multiplier** on outgoing damage.

Confidence: **HIGH on offset**, **MEDIUM on application site** (not re-verified this session;
cited from existing report).

---

## 7. Rulesmd.ini Defaults (YR retail)

From [rulesmd.ini:843-845](../ra2-rust-game/ini/rulesmd.ini#L843-L845):

```ini
;***Tank Bunker***
BunkerDamageMultiplier=1.3   ; damage bonus (not penalty — scales output up)
BunkerROFMultiplier=1.3      ; ROF divisor — higher value = slower fire
BunkerWeaponRangeBonus=2     ; +2 cells range while bunkered
```

Applied semantics:
- `damage_out = base_damage × 1.3`
- `rof_frames = base_rof_frames / 1.3`  (faster fire, not slower — since lower ROF ticks = faster)
- `range = base_range + 2` cells

**Interpretation note:** YR `ROF=` values are "frames between shots," so dividing by a
multiplier > 1.0 **shortens** the interval → faster fire. The in-INI comment describing
"divisor" is consistent with this.

---

## 8. Rust Port Status

From [src/rules/ruleset.rs:480-522](../ra2-rust-game/src/rules/ruleset.rs#L480-L522):

```rust
pub bunker_damage_multiplier: f32,     // parsed, UNUSED
pub bunker_rof_multiplier: f32,        // parsed, UNUSED
pub bunker_weapon_range_bonus: i32,    // parsed, UNUSED
```

**INI parsing: complete. Runtime logic: none.**

Missing components:
- `BuildingTypeClass::bunker` flag (not parsed) → ruleset flag for `Bunker=yes`
- `TechnoTypeClass::bunkerable` flag (not parsed)
- `BuildingClass` runtime field equivalent to `+0x2E4` (contained unit slot)
- Equivalent of `TechnoClass::+0x2E4` (back-reference / IsInBunker tag)
- 6-state entry machine (cell approach, anim, install)
- BunkerWallsUp / BunkerWallsDown sound hooks
- Damaged-state art slot selection (`Type+0x11F4` family)
- Integration with damage/ROF/range pipelines (none of the three multipliers are
  consulted when resolving combat math)
- Eject / exit path on bunker destruction or sell (**not traced this session**)

---

## 9. Known Gaps / Deferred Investigations

Items the research did not fully resolve; flagged honestly rather than guessed.

1. **Exit / Eject mechanism** — not traced. The state machine only handles entry.
   Likely lives alongside `SellBuilding` or `BuildingClass::ReceiveDamage` destruction path;
   `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md:227` references "Bunker/ForceShield" logic
   at case 12 gated on `field_0x2E4 != 0` — indicates a dedicated destruction branch for
   garrisoned/bunkered buildings, but the eject mechanics themselves weren't read.

2. **Entry trigger** — how a Bunkerable unit initiates entry. The state machine
   assumes `building.field_0x2E4` or `FootClass::GetDestination()` already points at
   the unit. The dock-queue / radio-protocol handshake that assigns a unit to the
   bunker was not traced. Suspected analog of `BuildingClass::CanDock` (garrison)
   but with `Type+0x16AB` branching.

3. **Mission value `5`** — used in `MissionSet(5, 1)` on install. Common YR convention
   is `5 = Guard`, but not verified against the mission enum in gamemd.exe.

4. **`Bunkerable` exact offset on TechnoTypeClass** — confirmed parsed in `TechnoTypeClass::ReadINI`
   (xref at `0x0071500A`), but the store offset wasn't read this session. The ReadINI
   function is large (148KB decompile) and was skipped for scope.

5. **Tank Bunker (`NATBNK`) vs Battle Bunker (`NABNKR`) gating** — both set `Bunker=yes`
   in art/rules, both follow the same `Type+0x16AB` branch, so they share the state machine.
   Whether unit classes that can enter differ (e.g., infantry vs vehicle) is controlled
   by `Bunkerable=yes` on the unit type. **Not confirmed by empirical test.**

6. **State 2 animation frame selection** — approach animation uses frame indices
   `0x43/0x44/0x45/0x46` selected by the angle computed in state 1. Full mapping of
   which angle bucket → which frame wasn't extracted.

7. **`unit.vtable+0x150` (called on install)** — the "hide / limbo" behavior.
   Likely analogous to passenger-in-transport hide. Not decoded.

---

## 10. Confidence Summary

| Finding                                                         | Confidence |
|-----------------------------------------------------------------|------------|
| `Bunker=yes` at `BuildingTypeClass+0x16AB`                      | HIGH       |
| `BunkerDamageMultiplier` at `RulesClass+0xF4C`                  | HIGH       |
| `BunkerROFMultiplier` at `RulesClass+0xF50`                     | HIGH       |
| `BunkerWeaponRangeBonus` at `RulesClass+0xF54`                  | HIGH       |
| `BunkerWallsUpSound` at `RulesClass+0x240`                      | HIGH       |
| `BunkerWallsDownSound` at `RulesClass+0x244`                    | HIGH       |
| State machine at `0x00458E50`, 6 states                         | HIGH       |
| Sole caller = `MissionRepairAndProduce`, unconditional on `+0x16AB` | HIGH   |
| Contained unit slot at `BuildingClass+0x2E4`                    | HIGH       |
| Back-reference at `TechnoClass+0x2E4` (`param_1[0xB9]`)         | HIGH       |
| BunkerROFMultiplier applied in `GetROF` as divisor              | HIGH (verified this session) |
| BunkerWeaponRangeBonus applied in `InRange` as additive `×256`  | HIGH (verified this session) |
| BunkerDamageMultiplier applied in damage accumulation           | MEDIUM (cited from existing report) |
| Health-gated damaged-anim slots at `+0x11F4/+0x1204/+0x1238/+0x1248` | HIGH |
| `ConditionRed` health threshold at `RulesClass+0x1700`          | HIGH (pattern-shared with other mission branches) |
| `Bunkerable` key parsed in `TechnoTypeClass::ReadINI`           | MEDIUM (xref confirmed, offset not read) |
| Mission `5` = Guard after install                               | MEDIUM (naming inference, not verified against enum) |
| Entry trigger / docking-queue assignment                        | LOW (not traced) |
| Eject / exit on destroy or sell                                 | LOW (not traced) |

---

## 11. References

- Live decompilation this session:
  - `RulesClass::ReadCombatDamage` at `FUN_0066BBB0`
  - `RulesClass::ReadAudioVisual` at `0x006691E0` (corrected 2026-05-29: was `0x00669E87`; `get_function_by_address 0x006691E0` confirms entry; `0x00669E87` was an interior code-offset, not the entry — GHIDRA_ADDRESS_SHIFT)
  - `BuildingClass::MissionRepairAndProduce` at `0x0044B780`
  - `FUN_00458E50` (bunker state machine)
  - `TechnoClass::GetROF` at `0x006FCFA0`
  - `TechnoClass::InRange` at `0x006F7220`
- Sibling reports (consulted, not re-verified):
  - `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` — BuildingTypeClass layout
  - `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md` — sibling state machines (Hospital/Armory/UnitRepair)
  - `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` — dock/radio framework
  - `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` — destruction-path hints
  - `DAMAGE_MATH_GHIDRA_REPORT.md` — bunker damage-multiplier application site
  - `GARRISON_SYSTEM_GHIDRA_REPORT.md` / `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` — contrast with civilian garrison
