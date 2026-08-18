---
name: HouseClass stolen-tech / spy-veteran-bonus flag audit (+0x2BC..+0x2C0)
date: 2026-04-21
related:
  - OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md — **corrects §4 offsets (off-by-one)**
  - HOUSE_TYPE_CLASS_GHIDRA_REPORT.md §6.4 — resolves the "veteran fallback path mystery"
  - HOUSECLASS_VERIFIED_FIELD_MAP.md — adds the five flags to the field map
---

# HouseClass stolen-tech / spy-veteran flag audit — Ghidra

**Scope:** Five consecutive bytes at `HouseClass+0x2BC..+0x2C0`. The prior
`OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` labeled three of these as stolen-tech flags
(at offsets `+0x2BD`/`+0x2BE`/`+0x2BF`) and assigned `+0x2BF`/`+0x2C0` separately as
radar/power sabotage flags. Fresh decompilation shows **the stolen-tech offsets are off
by one** and the "sabotage" flags are actually the vanilla **spy-on-factory veteran
bonus** mechanic. This report resolves it.

**Confidence:** HIGH — every finding traced from string constant → writer → reader, and
correlated against vanilla game behavior (spy a Barracks → infantry spawn veteran).

---

## 1. The definitive layout

| Offset | Byte | Field | Set by | Read by | Vanilla effect |
|-------:|-----:|-------|--------|---------|----------------|
| `+0x2BC` | bool | **StolenThirdTech** | `BuildingClass::OnSpyInfiltrate` when spy enters a tech building on the Yuri side | `HouseClass::CanBuild` gates types with `RequiresStolenThirdTech=yes` | Enables buildables like Yuri Lab units |
| `+0x2BD` | bool | **StolenSovietTech** | same, Soviet-side tech building | CanBuild gates `RequiresStolenSovietTech=yes` | Enables cross-faction Soviet units |
| `+0x2BE` | bool | **StolenAlliedTech** | same, Allied-side tech building | CanBuild gates `RequiresStolenAlliedTech=yes` | Enables cross-faction Allied units |
| `+0x2BF` | bool | **InfantryVeteranBonus** | `OnSpyInfiltrate` when spy enters a building with `Factory=InfantryType` (a Barracks) | `InfantryClass::InitFromType` in a fallback `SetVeteran(1)` branch | **Spying a Barracks makes your produced infantry spawn at Veteran rank** |
| `+0x2C0` | bool | **VehicleVeteranBonus** | `OnSpyInfiltrate` when spy enters a building with `Factory=UnitType` (a War Factory) | `UnitClass::Constructor` in a fallback `SetVeteran(1)` branch | **Spying a War Factory makes your produced vehicles spawn at Veteran rank** |

All five are single bytes, all zero-initialized by `HouseClass::Constructor`
(`0x004F54A0`, verified). There are no `WriteINI`/save paths that re-cast these bytes —
they live entirely in runtime state.

---

## 2. What the prior doc got wrong

`OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §4 states:

| Byte Offset | Field | Default |
|-------------|-------|---------|
| `+0x2BD` | StolenAlliedTech | 0 |
| `+0x2BE` | StolenSovietTech | 0 |
| `+0x2BF` | StolenThirdTech | 0 |

**All three offsets are one byte too high.** The correct layout (verified from both
writer and reader, see §3 and §4):

| Correct offset | Correct label |
|----------------|---------------|
| `+0x2BC` | StolenThirdTech |
| `+0x2BD` | StolenSovietTech |
| `+0x2BE` | StolenAlliedTech |

Additionally, the prior doc's notes on §3 list:

> `+0xEB8 == 0x28` | Power sabotage — sets `house+0x2C0`, EVA: `"EVA_EnemyBasePoweredDown"`
> `+0xEB8 == 0x10` | Radar sabotage — sets `house+0x2BF`

That table misinterprets the discriminator. `TechnoTypeClass+0xEB8` is **not** a "spy
effect" or "what kind of sabotage" field — it is the `Factory=` INI enum on
BuildingType (see §5). The values `0x28` and `0x10` are **`UnitType`** and
**`InfantryType`** respectively, not "power plant" and "radar". Those two branches are
the vanilla **spy-on-factory veteran bonus** mechanic, not sabotage.

(Note: the radar/power sabotage paths exist in the same function, but they go through
different branches — `ResetRadar` flag for radar-scramble, and `SpyPowerSabotage` call
for the timed-power-outage on the victim's house. Neither of those writes `+0x2BF` or
`+0x2C0` on the spyer.)

---

## 3. Writers — `BuildingClass::OnSpyInfiltrate` at `0x004571E0`

The spy infiltration handler has a nested control flow. Simplified, with all five
flag-writing branches called out:

```c
void BuildingClass::OnSpyInfiltrate(HouseClass* spyOwner) {
    BuildingTypeClass* type = this->Type;
    if (this->Owner == spyOwner) return;       // don't spy your own

    // Outer gate: type->ResetRadar (+0x16A4)
    if (type->ResetRadar) {
        FUN_0050bd10();                         // victim's radar scramble (side-effect)
        // plays EVA_BuildingInfRadarSabotaged / EVA_RadarSabotaged
        return;                                 // no flag writes to spyOwner
    }

    // type->StolenTechIndex (+0xEE0) > 0 — cash-hungry branch
    if (type->StolenTechIndex > 0) {
        SpyPowerSabotage(this->Owner, rules+0xD64, ...);  // timed power drain on victim
        // No flag on spyOwner here either
        return;
    }

    // Stolen-tech path: type appears in rules->StolenTechBuildings (+0x920..+0x92C)
    for (i = 0; i < rules->StolenTechBuildingsCount; i++) {
        if (rules->StolenTechBuildings[i] == type) {
            // Branch by AIBasePlanningSide of the infiltrated building's type
            switch (type->AIBasePlanningSide /* +0x6D0 */) {
                case 0:  spyOwner[0x2BE] = 1; break;  // Allied → StolenAlliedTech
                case 1:  spyOwner[0x2BD] = 1; break;  // Soviet → StolenSovietTech
                default: spyOwner[0x2BC] = 1; break;  // Third (Yuri) → StolenThirdTech
            }
            spyOwner->ProductionChanged = 1;  // +0x1FC — trigger sidebar rebuild
            // plays EVA_TechnologyStolen (victim) / EVA_NewTechnologyAcquired (attacker)
            goto done;
        }
    }

    // Not in stolen-tech list → check InfiltrateWeapon / CashBounty / Factory bonus:
    if (type->InfiltrateWeapon /* +0x16F0 */ != -1) {
        OnSpyWeaponInfiltrate();                // dedicated weapon handler
    }
    else {
        if (type->CashBounty /* +0x800 */ > 0) {
            // Steal (%SpyMoneyStealPercent of victim's cash) → add to spyer
            SpendMoney(victim, amount); AddCredits(spyer, amount);
            // plays EVA_CashStolen
        }
        // The Factory-based veteran-bonus branch:
        if (type->Factory /* +0xEB8 */ == 0x28) {     // Factory = UnitType (War Factory)
            spyOwner[0x2C0] = 1;                       // VehicleVeteranBonus
            spyOwner->ProductionChanged = 1;
            // plays VoxClass::PlayEVA (message index selected elsewhere)
        }
        else if (type->Factory == 0x10) {              // Factory = InfantryType (Barracks)
            spyOwner[0x2BF] = 1;                       // InfantryVeteranBonus
            spyOwner->ProductionChanged = 1;
        }
    }
done:
    (**(this->vtable + 0x124))(2);                    // animation / aftermath
}
```

The three stolen-tech setters are at `0x004574C6` / `0x004574CC` / `0x004574D2`
(sets `+0x2BE` / `+0x2BD` / `+0x2BC`). The two veteran-bonus setters are at
`0x00457353` (`+0x2C0`) and `0x00457429` (`+0x2BF`). All writes target `spyOwner` — the
attacker's house.

---

## 4. Readers — CanBuild and the spawn-veteran fallbacks

### 4.1 `HouseClass::CanBuild` at `0x004F7870` — stolen-tech gate

Inside the main prerequisite evaluation block, before prerequisite expansion:

```c
if ( (type->RequiresStolenAlliedTech /* +0xD9D */ == 0 || house->StolenAlliedTech /* +0x2BE */)
  && (type->RequiresStolenSovietTech /* +0xD9C */ == 0 || house->StolenSovietTech /* +0x2BD */)
  && (type->RequiresStolenThirdTech  /* +0xD9B */ == 0 || house->StolenThirdTech  /* +0x2BC */)
  && (/* RequiredHouses / ForbiddenHouses / AcquiredTech bitmasks ... */) ) {
    // ... continue prerequisite check
} else {
    return 0;  // missing stolen-tech, fail CanBuild
}
```

Every TechnoType's `RequiresStolenXxxTech=` INI key writes to its matching `+0xD9B/C/D`
byte (parsed in `TechnoTypeClass::ReadINI` at `0x007144DB / +0x007144F7 / +0x0071450F`).
The CanBuild read side is definitive evidence of the offset mapping.

### 4.2 `InfantryClass::InitFromType` at `0x00517CC0` — spawn-veteran fallback

The primary path is the VeteranInfantry DVC scan (documented in
`HOUSE_TYPE_CLASS_GHIDRA_REPORT.md` §6.1). The **fallback** path:

```c
if (house != NULL
    && house->InfantryVeteranBonus /* +0x2BF */
    && type->Trainable              /* +0xC8E */ ) {
    VeterancyStruct::SetVeteran(1);
}
```

So: if the house has infiltrated an enemy Barracks at any prior point in the match
(flag set and never cleared), *and* the infantry type is `Trainable=yes` (default for
most combat infantry, disabled for engineers / civilians / dogs), *every subsequent
produced infantry of that type spawns at Veteran rank*.

### 4.3 `UnitClass::Constructor` at `0x007353C0` — spawn-veteran fallback

Identical pattern, one extra non-naval guard:

```c
if (house != NULL
    && house->VehicleVeteranBonus /* +0x2C0 */
    && type->Naval /* +0xCCE */ == 0
    && type->Trainable /* +0xC8E */ ) {
    VeterancyStruct::SetVeteran(1);
}
```

So: infiltrated War Factory → every produced non-naval vehicle spawns Veteran. Naval
units (battleships, destroyers, dolphins, etc.) are explicitly excluded.

Aircraft: **not wired** to this mechanic. `AircraftClass::InitFromType` (`0x00413F80`)
has no equivalent fallback. The only veteran path for aircraft is the `VeteranAircraft=`
DVC scan on the country. Vanilla has no spy-on-airfield veteran bonus.

---

## 5. `TechnoTypeClass+0xEB8` — the `Factory=` INI key

| INI value | Enum value | Meaning |
|-----------|-----------:|---------|
| `InfantryType` | `0x10` (16) | Barracks — produces infantry |
| `UnitType` | `0x28` (40) | War Factory — produces vehicles |
| `AircraftType` | (other) | Airfield/helipad — produces aircraft |
| `BuildingType` | (other) | ConYard — produces buildings |
| *not set* | `0` | Not a factory |

Parsed in `BuildingTypeClass::ReadINI_Water` at `0x00460321` via `FUN_00474FF0`, which
passes the string through the name-to-enum table at `DAT_00816EE0..0x00817130`. That
table contains every RTTI-like category name in the engine (`"Aircraft"`, `"Building"`,
`"Infantry"`, `"Unit"`, `"Trigger"`, `"InfantryType"`, `"UnitType"`, etc.) mapped to
sequential integers (0..~70+).

Decoded entries referenced by the spy handler:
- `0x008173C0 → "InfantryType"` → value `16` = `0x10`
- `0x008172B4 → "UnitType"` → value `40` = `0x28`

**Gotcha:** the string `"Infantry"` at `0x008173D0` maps to value `15` (`0x0F`) — a
*different* enum from `"InfantryType"` at `0x008173C0` (value `0x10`). The engine
distinguishes the bare RTTI-kind name from the type-class name. The `Factory=` key uses
the `…Type` names exclusively; Ghidra's decompilation shows `s_Factory_008173F0` as the
INI key pointer (value 12 / `0x0C`), but the parsed *result* is the enum of whichever
string the user put in the value.

---

## 6. `TechnoTypeClass+0xC8E` — the `Trainable=` INI key

Bool, parsed in `TechnoTypeClass::ReadINI` at `0x007149E4` against string
`"Trainable"` at `0x00843974`. Default value: inherited from base-class default in
TechnoType, almost always `yes`.

In vanilla `rulesmd.ini`:
- Default for infantry: `yes` (so Barracks spy applies to all standard soldiers)
- Engineers / Spies / Chrono Legionnaires / Civilians / Dogs / Chitzkoi / Yuri's
  psychic units: typically `Trainable=no` (they don't gain veterancy)
- Vehicles default to `yes` except certain support units (harvesters, MCV, …)

The read at `+0xC8E` gates the veteran-bonus fallback. If a produced unit has
`Trainable=no`, the fallback path does nothing — even if the house spied the
corresponding factory. This matches what players see in-game: spying a Barracks makes
GIs spawn veteran, but not Engineers.

---

## 7. Vanilla player-visible behavior, end to end

1. At match start, all five bytes are 0. `CanBuild` passes any `RequiresStolenXxxTech`
   gate vacuously; no spawn-veteran fallback fires.

2. Spy (or IFV-disguised spy, or Yuri-spy) enters an enemy tech building (Battle Lab,
   Allied / Soviet / Yuri variant) → `OnSpyInfiltrate` runs → matches an entry in
   `RulesClass+0x920` (StolenTechBuildings list) → sets *one* of
   `+0x2BC/+0x2BD/+0x2BE` based on the victim building's `AIBasePlanningSide`. The
   spyer's sidebar now shows stolen-tech unlocks.

3. Spy enters an enemy Barracks → `type->Factory == 0x10` branch → sets `+0x2BF`.
   From this point forward, every new infantry unit built by the spyer spawns at
   Veteran rank (primary: DVC list; fallback: `+0x2BF`).

4. Spy enters an enemy War Factory → `type->Factory == 0x28` branch → sets `+0x2C0`.
   Every new non-naval vehicle spawns Veteran.

5. **The flags never clear.** There is no writer that zeros them during a match, no
   timer, no "power wore off" reset. Once spied, the bonus persists until the house is
   destroyed. (Power sabotage is a *separate* mechanism — handled by a countdown stored
   elsewhere on HouseClass, which is outside this report's scope.)

6. Across save/load: the bytes are part of HouseClass's 0x160B8-byte instance and go
   out as-is in the save blob. No name-based remap needed since these flags do not
   reference pointers.

7. Across lockstep: deterministic — every peer computes the same flags from the same
   inputs because spy entry is an event broadcast to all peers and processed
   identically.

---

## 8. Adjacent fields (for context — not changed by this audit)

The next four dwords after `+0x2C0` are a separate, adjacent mechanism that the prior
doc also documents (and partly mislabels). `CanBuild` reads them as per-RTTI-category
**acquired-tech bitmasks**:

| Offset | Size | Read when RTTI equals | Bit-ANDed against `type->RequiredHouses (+0xDA0)` |
|-------:|-----:|:---------------------:|------|
| `+0x2C4` | u32 | `0x10` (Infantry) | Allows infantry from otherwise-forbidden country |
| `+0x2C8` | u32 | `0x28` (Building) | Allows buildings |
| `+0x2CC` | u32 | `3` (Aircraft) | Allows aircraft |
| `+0x2D0` | u32 | `7` (Unit/Vehicle) | Allows vehicles |

The prior `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §4 names these "AlliedAcquiredTech
/ SovietAcquiredTech / ThirdAcquiredTech / FourthAcquiredTech" and calls them per-side
masks. They are not per-side — they are per-RTTI bitmasks over country indices. The
naming should read `AcquiredInfantryMask` / `AcquiredBuildingMask` /
`AcquiredAircraftMask` / `AcquiredVehicleMask`. These bitmasks are set when some other
event (not stolen-tech) grants the house a specific acquired-tech permission. (Scope:
outside this audit — flagged for a follow-up pass if anyone wires it to Rust.)

The RTTI-to-offset assignment in the prior doc is also off (e.g. it lists 0x10 as
"Aircraft"). The correct RTTI table used throughout the binary is:

| RTTI value | Class |
|-----------:|-------|
| `3` | AircraftClass |
| `7` | UnitClass (vehicles) |
| `0x10` | InfantryClass |
| `0x28` | BuildingClass |

(Same values used by every other RTTI-dispatch in the engine — GetArmorBonus,
GetCostBonus, CountOwnedInstances, etc.)

---

## 9. Rust implementation implications

If the Rust engine models spy infiltration at all:

- `HouseState` needs five `bool` fields (or a 5-bit bitfield):
  - `stolen_third_tech`, `stolen_soviet_tech`, `stolen_allied_tech`
  - `infantry_veteran_bonus`, `vehicle_veteran_bonus`
- Zero on house creation. Set on spy entry. Never cleared.
- CanBuild wires the three stolen-tech bools against TechnoType's three
  `RequiresStolen{Third,Soviet,Allied}Tech` flags.
- Produced infantry/vehicle check `infantry_veteran_bonus`/`vehicle_veteran_bonus` +
  type's `trainable` flag (+ naval guard for vehicles) as a secondary veteran path
  after the primary `HouseType::Veteran{Infantry,Units}` DVC scan.
- No `+0xC8E` ambiguity remains: it's `Trainable=` on TechnoType, defaults yes, turned
  off for support/civilian units.
- The "AcquiredTech" per-RTTI bitmasks (§8) can be ignored until any RA2 mechanic
  actually sets them (nothing in vanilla skirmish appears to, based on a quick
  writer-xref scan — worth confirming if implementing cross-faction build permissions).

Neither flag interacts with per-country HouseTypeClass fields documented in
`HOUSE_TYPE_CLASS_GHIDRA_REPORT.md`. They live on per-match HouseClass state only.

---

## Sources

**Ghidra addresses decompiled:**
- `0x004F7870` — HouseClass::CanBuild (the definitive read site for stolen-tech offsets)
- `0x004571E0` — BuildingClass::OnSpyInfiltrate (the writer)
- `0x00517CC0` — InfantryClass::InitFromType (+0x2BF + 0xC8E reader)
- `0x007353C0` — UnitClass::Constructor (+0x2C0 + 0xC8E + 0xCCE reader)
- `0x00413F80` — AircraftClass::InitFromType (confirmed: no fallback veteran path)
- `0x004F54A0` — HouseClass::Constructor (zeros all five bytes at creation)
- `0x00460321` — BuildingTypeClass::ReadINI_Water (Factory= write site at +0xEB8)
- `0x00474FF0` — name-to-enum helper for Factory=
- `0x00712170` — TechnoTypeClass::ReadINI (Trainable= at +0xC8E; RequiresStolenXxxTech
  at +0xD9B/C/D)

**Memory verified:**
- `0x00816EE0` table, entry 16: `"InfantryType" → 0x10`
- Same table, entry 40: `"UnitType" → 0x28`
- `0x00843BC4 → "RequiresStolenAlliedTech"` (xref writes +0xD9D)
- `0x00843BE0 → "RequiresStolenSovietTech"` (xref writes +0xD9C)
- `0x00843BFC → "RequiresStolenThirdTech"` (xref writes +0xD9B)
- `0x00843974 → "Trainable"` (xref writes +0xC8E)

**EVA string xrefs that anchored branch identification** (all in OnSpyInfiltrate):
- `EVA_CashStolen` (`0x0081916C`)
- `EVA_EnemyBasePoweredDown` (`0x0081917C`) — in the Factory=UnitType branch (an
  attacker-side message; the name is misleading in this code path and may be a TS-era
  label, but the associated flag is `+0x2C0 = VehicleVeteranBonus`)
- `EVA_PowerSabotaged` (`0x008191B0`) — in the SpyPowerSabotage subroutine path
- `EVA_BuildingInfRadarSabotaged` (`0x008191C4`) — in the Factory=InfantryType branch
  (same TS-era labeling caveat; flag is `+0x2BF = InfantryVeteranBonus`)
- `EVA_RadarSabotaged` (`0x008191E4`) — in the ResetRadar branch (victim-side message)
- `EVA_TechnologyStolen` (`0x00819138`) / `EVA_NewTechnologyAcquired` (`0x0081911C`) —
  in the StolenTech branch

**Docs corrected / updated:**
- `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §4 needs a note pointing here for the
  definitive +0x2BC/D/E mapping and +0x2BF/0 semantics. (Not editing that doc in this
  pass; this audit is the authoritative record going forward.)
- `HOUSE_TYPE_CLASS_GHIDRA_REPORT.md` §6.4 and §12 — the veteran-fallback "open
  question" is resolved by this audit; `+0x2BF/+0x2C0` are the spy-veteran-bonus
  flags, not sabotage flags, and +0xC8E is `Trainable=`.
