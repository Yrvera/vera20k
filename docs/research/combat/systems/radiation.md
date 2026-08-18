# Radiation

This doc is the canonical reference for the **Radiation system** in gamemd.exe:
the area-of-effect persistent damage field created by weapons with `RadLevel > 0`.

Two flags compose:
1. **`RadLevel=N`** on the WeaponTypeClass (`weapon+0x158`) — when N > 0, the weapon
   creates or augments a `RadSiteClass` at the impact cell.
2. **`Radiation=yes`** on the WarheadTypeClass (`wh+0x177`) — used by the per-target
   ReceiveDamage gate to apply `ImmuneToRadiation` immunity check.

A `RadSiteClass` object is allocated at the impact cell, stores radiation level + spread,
applies per-cell damage falloff over time, and emits a fading colored light. Units in
irradiated cells take periodic damage via the `RadSiteWarhead` warhead.

Out-of-scope:
- EMP system (separate but related — same `ImmuneToRadiation` flag is reused for buildings) → [`emp.md`](emp.md) (separate doc, iteration #20)
- The Desolator deploy `RadEruption` beam pattern → [`rail_gun.md`](rail_gun.md) §7
- Damage transform → [`damage_formula.md`](damage_formula.md)

---

## 1. Flag layout (verified)

### WeaponTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `weapon+0x158` | `RadLevel=` | `0x00849298` (verified live 2026-05-17) | Radiation level deposited at impact. `0` = no radiation. |

Parsed at `WeaponTypeClass::ReadINI 0x007728DA`.

### WarheadTypeClass

| Offset | INI key | Effect |
|---|---|---|
| `wh+0x177` | `Radiation=` | Gates `ImmuneToRadiation` early-out in ReceiveDamage |

### TechnoTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `type+0xD37` | `ImmuneToRadiation=` | `0x00843854` (verified live 2026-05-17) | If true, unit is immune to radiation damage AND EMP (for buildings) |

Parsed at `TechnoTypeClass::ReadINI 0x00714D53`.

### BuildingTypeClass (extra immunity slot)

| Offset | Field | Notes |
|---|---|---|
| `BuildingType+0x1701` | (mirror of ImmuneToRadiation for buildings) | Per existing canonical doc — used by EMP application loop |

### Rules `[Radiation]` constants

| Offset | INI key | Type | Effect |
|---|---|---|---|
| `Rules+0x1804` | `RadDurationMultiple` | int | `TotalDuration = RadLevel × this` (frames) |
| `Rules+0x1808` | `RadApplicationDelay` | int | Frames between radiation damage applications to units |
| `Rules+0x180C` | `RadLevelMax` | int | Maximum radiation level (clamps additive RadLevel) |
| `Rules+0x1810` | `RadLevelDelay` | int | Frames between rad-level decay steps |
| `Rules+0x1814` | `RadLightDelay` | int | Frames between light intensity updates |
| `Rules+0x1818` | `RadLevelFactor` | double | Factor for per-cell damage from level |
| `Rules+0x1820` | `RadLightFactor` | double | Factor for light intensity from level |
| `Rules+0x1828` | `RadTintFactor` | double | Factor for tint color from RadColor |
| `Rules+0x1830` | `RadColor` | RGB | Color of the radiation glow |
| `Rules+0x1834` | `RadSiteWarhead` | WarheadType* | Warhead used to apply radiation damage to units |

Parsed at `RulesClass::ReadRadiation 0x0066CF70`.

### Confidence (flags)

- **Content: HIGH** — three core flag strings re-verified live 2026-05-17.
- **Identity: HIGH** — single string per flag, single parser site.
- **Binding: HIGH** — verified consumer call sites in `WarheadTypeClass::Detonate` (radiation site creation) and `TechnoClass::ReceiveDamage` (immunity gate).

---

## 2. `RadSiteClass` struct layout (verified, 0x74 bytes)

Class size confirmed via `GetSize @ 0x0065B3A0` returning `0x74`.

| Offset | Type | Field | Notes |
|---|---|---|---|
| `0x00-0x0F` | ptr×4 | vtables | `vtable__RadSiteClass @ 0x007F0810` + 3 secondary |
| `0x10-0x23` | — | AbstractClass base | Inherited |
| `0x24` | ptr | `LightSource` | `LightSourceClass*` for the glow |
| `0x28-0x30` | int×3 | RadLevelTimer (Start/Aux/Delay) | CDTimerClass-style timer for damage application |
| `0x34-0x3C` | int×3 | RadLightTimer (Start/Aux/Delay) | CDTimerClass-style timer for light updates |
| `0x40` | short | `CellX` | Center cell X coord |
| `0x42` | short | `CellY` | Center cell Y coord |
| `0x44` | int | `Spread` | Radius in cells |
| `0x48` | int | `SpreadInLeptons` | `= Spread × 256 + 128` |
| `0x4C` | int | `RadLevel` | Current radiation level |
| `0x50` | int | `RadLevelPerStep` | `= TotalDuration / RadLevelDelay` |
| `0x54` | int | `LightIntensity` | `= ftol(RadLevel × RadLightFactor)` |
| `0x58-0x60` | int×3 | `LightTintR/G/B` | `= ftol(RadColor.R/G/B × RadTintFactor)` |
| `0x64` | int | `LightIntensityPerStep` | `= TotalDuration / RadLightDelay` |
| `0x68` | int | `LightIntensityDecrement` | `= LightIntensity / LightIntensityPerStep` |
| `0x6C` | int | `TotalDuration` | `= RadLevel × RadDurationMultiple` |
| `0x70` | int | `RemainingDuration` | Decremented by 1 each tick |

### Global container

| Address | Meaning |
|---|---|
| `0x00B04BD0` | DynVector vtable ptr |
| `0x00B04BD4` | `g_RadSiteClass_Array` data ptr |
| `0x00B04BD8` | capacity |
| `0x00B04BE0` | count |

### Confidence

- **Content: HIGH** — every offset cross-verified against constructor writes and accessor reads in existing canonical doc.
- **Identity: HIGH** — vtable at `0x007F0810` uniquely identifies the class.
- **Binding: HIGH** — single allocator (`RadSiteClass::Constructor 0x0065B1E0`), single per-tick caller (`AI 0x0065B800`).

---

## 3. CellClass radiation fields

| Offset | Type | Field |
|---|---|---|
| `cell+0xF0` | double | `RadLevel` (current intensity in this cell) |
| `cell+0xF8` | ptr | `RadSite` (pointer to the affecting RadSiteClass, NULL if none) |

Accessors:
- `CellClass::GetRadSite 0x00487C80`
- `CellClass::SetRadSite 0x00487C70`
- `CellClass::IncreaseRadLevel 0x00487CE0`
- `CellClass::DecreaseRadLevel 0x00487D00`

---

## 4. RadSite creation (from `WarheadTypeClass::Detonate`)

`WarheadTypeClass::Detonate` at `0x004690B0`, post-area-damage block:

```c
// param_1 = BulletClass, weapon at +0x4C
if (bullet.Weapon != NULL && bullet.Weapon.RadLevel (+0x158) > 0) {
    cell = MapClass::Get_CellClass(impact_coords);
    existingRadSite = cell.GetRadSite()                            // cell+0xF8

    if (existingRadSite == NULL) {
        // First radiation at this cell — allocate new RadSite
        radSite = new RadSiteClass()                                // 0x0065B1E0
        radSite.SetCell(cell)                                       // 0x0065B4C0 → +0x40/+0x42
        radSite.SetSpread(spread)                                   // 0x0065B4D0 → +0x44, +0x48
        radSite.SetRadLevel(weapon.RadLevel)                        // 0x0065B4F0 → +0x4C/+0x6C/+0x70
        radSite.Activate()                                          // 0x0065B580
        cell.SetRadSite(radSite)                                    // 0x00487C70 → cell+0xF8
    } else {
        // Cell already has a RadSite — augment
        existingRadSite.AddRadLevel(weapon.RadLevel)               // 0x0065B530
    }
}
```

### Key behaviors

- **Augmentation** (`AddRadLevel 0x0065B530`): when a cell already has a RadSite,
  the new RadLevel is **added** to the existing one. The function then recomputes
  duration and reactivates the light. Stacking radiation from multiple shots is
  additive.
- **Spread**: derived from the weapon/warhead (per existing canonical doc — the
  exact source of the spread value is not enumerated; likely `weapon.CellSpread` or a fixed default per RadLevel). Open follow-up #1.
- **SpreadInLeptons formula**: `Spread × 256 + 128` — an extra half-cell for inclusive radius.
- **TotalDuration formula**: `RadLevel × RadDurationMultiple` — higher levels last proportionally longer.

### Confidence

- **Content: HIGH** — existing canonical doc decompiles the block in `WarheadTypeClass::Detonate`.
- **Identity: HIGH** — single dispatch site post-area-damage in Detonate.
- **Binding: HIGH** — fires for every `RadLevel>0` weapon impact.

---

## 5. RadSite activation (`RadSiteClass::Activate 0x0065B580`)

When `Activate` is called:

1. Initialize `RadLevelTimer.Delay = Rules.RadLevelDelay`.
2. Initialize `RadLightTimer.Delay = Rules.RadLightDelay`.
3. Compute light intensity: `LightIntensity = ftol(RadLevel × Rules.RadLightFactor)`.
4. Compute tint RGB: `TintR/G/B = ftol(Rules.RadColor.R/G/B × Rules.RadTintFactor)`.
5. Compute per-step decrements:
   - `RadLevelPerStep = TotalDuration / RadLevelDelay`
   - `LightIntensityPerStep = TotalDuration / RadLightDelay`
   - `LightIntensityDecrement = LightIntensity / LightIntensityPerStep`
6. Create `LightSourceClass` at the center cell's 3D coords (or update existing light source intensity/tint if reactivating).
7. Call `SetCellRadLevels (0x0065B9C0)` to set initial radiation values on all cells within `Spread`.

### Cell radiation setup (`SetCellRadLevels 0x0065B9C0`)

Iterates a square region `(CellX±Spread, CellY±Spread)`:

```c
for each cell in square:
    dx = cell.X - center.X
    dy = cell.Y - center.Y
    dist = sqrt(dx² + dy²) × 256   // leptons
    if (dist <= SpreadInLeptons):
        cellRadLevel = ((SpreadInLeptons - dist) / SpreadInLeptons) × RadLevel
        // Linear falloff from center (full RadLevel) to edge (zero)
        cell.IncreaseRadLevel(cellRadLevel)
```

**Linear falloff** from center to edge. Distance > spread is excluded.

### Confidence

- **Content: HIGH** — existing canonical doc verifies activation sequence.
- **Identity: HIGH** — single Activate function, single SetCellRadLevels.
- **Binding: HIGH** — only called from Constructor and AddRadLevel.

---

## 6. `RadSiteClass::AI` per-tick update (`0x0065B800`)

Vtable slot `0x5C` — called each tick from the global RadSite array iterator.

```c
void AI() {
    RemainingDuration--                                        // +0x70

    // Timer 1: RadLevelDelay — apply per-cell radiation decay
    if (RadLevelTimer.expired):
        ApplyRadDamage()                                       // 0x0065BD00
        RadLevelTimer.Reset(Rules.RadLevelDelay)

    // Timer 2: RadLightDelay — fade light visual
    if (RadLightTimer.expired):
        // Compute remaining-fraction fade
        newR = TintRed × RemainingDuration / TotalDuration
        newG = TintGreen × RemainingDuration / TotalDuration
        newB = TintBlue × RemainingDuration / TotalDuration
        newIntensity = LightSource.intensity - LightIntensityDecrement
        LightSource.Update(newIntensity, newR, newG, newB, 0)
        RadLightTimer.Reset(Rules.RadLightDelay)

    // Self-destruct on expiry
    if (RemainingDuration <= 0):
        ~RadSiteClass()                                        // vtable+0x20, flag=1
}
```

### Per-cell radiation decay (`ApplyRadDamage 0x0065BD00`)

```c
for each cell in (CellX±Spread, CellY±Spread):
    dist = 3D_distance(center, target_cell) // leptons
    if (dist <= SpreadInLeptons):
        radAmount = ((SpreadInLeptons - dist) / SpreadInLeptons) × RadLevel
    else:
        radAmount = 0.0
    cell.DecreaseRadLevel(radAmount / RadLevelPerStep)
```

**Important:** this function DECREASES the cell-stored radiation level. The actual
**damage to units** in those cells is applied by a separate per-object update — see §7.

The decrement rate `RadLevelPerStep = TotalDuration / RadLevelDelay` means: over the
RadSite's lifetime, the cell radiation level is decremented `TotalDuration / RadLevelDelay`
times, each time by `(falloff × RadLevel) / RadLevelPerStep`, summing approximately
to the initial cell radiation level. Net effect: the radiation field decays from full
strength to zero over `TotalDuration` frames.

### Confidence

- **Content: HIGH** — AI function decomp matches the algorithm.
- **Identity: HIGH** — single per-tick function.
- **Binding: HIGH** — called by the global RadSite array iterator (TacticalClass-style every tick).

---

## 7. Damage application to units (separate path)

The per-cell radiation level (`cell+0xF0`) drives damage to units in the cell via the
**per-object update loop** (separate from RadSite::AI). Per the existing canonical doc:

> Objects in cells with `CellClass.RadLevel > 0` (offset 0xF0) receive periodic
> damage using the `RadSiteWarhead` warhead type. Units with `ImmuneToRadiation=yes`
> (TechnoTypeClass offset 0xD37) are skipped.

The exact per-object update site is **not enumerated** in the existing doc — likely
in `TechnoClass::AI_Update` or a per-object cell-check helper. **Open follow-up #2.**

### Per-tick damage formula (inferred)

Working hypothesis:
```c
// Once per RadApplicationDelay frames per unit
if (unit.Type.ImmuneToRadiation) skip
cell = unit.GetCell()
if (cell.RadLevel > 0):
    damage = ftol(cell.RadLevel × Rules.RadLevelFactor)
    unit.ReceiveDamage(damage, 0, Rules.RadSiteWarhead, ...)
```

Where `RadSiteWarhead` is the warhead from `Rules+0x1834` — typically a special
"Radiation" warhead with `Radiation=yes` and small CellSpread.

This is **inferred not verified**. Open follow-up #2.

### Confidence (damage to units)

- **Content: LOW** — formula and dispatch site are working hypothesis only.
- **Identity: LOW** — function not traced.
- **Binding: LOW** — caller chain unverified.

---

## 8. The 6-warhead-with-`Radiation=yes` survey

From `ini/rulesmd.ini`, warheads with `Radiation=yes`:

```ini
[RadBeamWarhead]            ; Desolator primary damage
Radiation=yes

[RadEruptionWarhead]        ; Desolator deploy
Radiation=yes

[RadSite]                   ; the [Radiation] RadSiteWarhead= used per Rules
Radiation=yes
```

(Verify the full list from rulesmd — there may be a few more for IFV-Desolator variants
and Elite Desolator.)

The `Radiation=yes` warhead flag at `wh+0x177` is checked in `TechnoClass::ReceiveDamage`:

```c
if (wh.Radiation (+0x177) != 0 && target.Type.ImmuneToRadiation (+0xD37) != 0):
    *pDamage = 0
    return 0
```

This is one of the 4 immunity gates in ReceiveDamage Step 7 (per [`friendly_fire.md`](friendly_fire.md) §3 → cross-reference to canonical `DAMAGE_MATH_GHIDRA_REPORT.md` §4 step 7).

---

## 9. Visual rendering

### Light source

A `LightSourceClass` is created at the RadSite center during Activate, with:
- Color: `Rules.RadColor × Rules.RadTintFactor`
- Intensity: `RadLevel × Rules.RadLightFactor`
- Fades linearly via `LightIntensityDecrement` per `RadLightDelay` frames.

Default `Rules.RadColor=0,255,0` produces the iconic green glow.

### Animation

`Rules+0x17F4` = `EMPulseSparkles` AnimType is **also** used for radiation sparkle effects
on units inside the rad cloud (per existing canonical doc §1.11). Reused asset across
radiation and EMP visuals.

---

## 10. Key addresses summary

| Address | Function |
|---|---|
| `0x0065B1E0` | RadSiteClass::Constructor |
| `0x0065B2F0` | RadSiteClass::Destructor |
| `0x0065B3A0` | RadSiteClass::GetSize (returns 0x74) |
| `0x0065B4C0` | RadSiteClass::SetCell |
| `0x0065B4D0` | RadSiteClass::SetSpread |
| `0x0065B4F0` | RadSiteClass::SetRadLevel |
| `0x0065B510` | RadSiteClass::GetCurrentRadLevel |
| `0x0065B530` | RadSiteClass::AddRadLevel (augment) |
| `0x0065B580` | RadSiteClass::Activate |
| `0x0065B800` | RadSiteClass::AI (vtable+0x5C) |
| `0x0065B9C0` | RadSiteClass::SetCellRadLevels |
| `0x0065BB50` | RadSiteClass::DecreaseCellRadLevels |
| `0x0065BD00` | RadSiteClass::ApplyRadDamage |
| `0x0066CF70` | RulesClass::ReadRadiation |
| `0x00487C70` | CellClass::SetRadSite |
| `0x00487C80` | CellClass::GetRadSite |
| `0x00487CE0` | CellClass::IncreaseRadLevel |
| `0x00487D00` | CellClass::DecreaseRadLevel |
| `0x007728DA` | WeaponTypeClass::ReadINI radiation parse site (`RadLevel=`) |
| `0x00714D53` | TechnoTypeClass::ReadINI immunity parse site (`ImmuneToRadiation=`) |

---

## 11. TS-legacy filter

- **`RadLevel` weapon flag**: LIVE in YR. Desolator weapons use it.
- **`Radiation=yes` warhead flag**: LIVE.
- **`ImmuneToRadiation=yes` type flag**: LIVE. Doubles as EMP immunity for buildings.
- **RadSiteClass**: LIVE — actively constructed and updated each match.
- **`[Radiation]` Rules section**: LIVE — all 10 keys parsed and consumed.
- **`Rules.RadSiteWarhead`**: LIVE — the per-tick damage warhead reference.

No TS-only dead paths identified.

The radiation field name reuses TS terminology — "Radiation" was named for Tiberium-style
radioactive contamination in TS, but in YR it's used for Desolator's exhaust pollution and
similar non-Tiberium contexts. Semantically the same mechanism.

---

## 12. Edge cases

| Case | Behavior |
|---|---|
| Two Desolator shots overlap on same cell | Second shot calls `AddRadLevel` on existing RadSite. RadLevel adds; TotalDuration recomputed. |
| RadLevel exceeds `RadLevelMax` after augmentation | Clamped to RadLevelMax (per parser, though augmentation may not re-clamp — needs verification, open follow-up #3). |
| Two Desolator shots on adjacent cells with overlapping spread | Two separate RadSites, each with its own falloff. Cells in the overlap zone have both cells.RadLevel values summed (since `IncreaseRadLevel` adds to the existing double). Damage to units = sum of both sites' contributions. |
| Unit walks into rad cloud | Each `RadApplicationDelay` frames, the cell's RadLevel is read and damage applied. Unit can outrun the cloud by moving to a non-irradiated cell. |
| Unit is `ImmuneToRadiation=yes` | All `Radiation=yes` warhead damage is zeroed in ReceiveDamage Step 7. (Building with the same flag also gets EMP immunity — see emp.md.) |
| Building straddles multiple cells, only some irradiated | Damage applies per-cell per-tick. Building takes damage from each irradiated cell it occupies. |
| Iron Curtained unit in rad cloud | IC blocks damage entirely (IC gate fires before Radiation gate in ReceiveDamage). |
| RadSite duration expires mid-tick | RadSiteClass::AI detects `RemainingDuration<=0` and self-destructs. Cell.RadSite is NOT explicitly cleared by AI — open follow-up #4. |
| LightSource creation fails | Activation continues but no glow. (Edge case from out-of-memory; unlikely in practice.) |
| RadLevel=0 weapon (no `RadLevel=` in INI) | The `RadLevel > 0` check fails immediately. No RadSite created. |
| Negative RadLevel set by mod | Behavior undefined. Multiplying by RadDurationMultiple gives negative TotalDuration, which makes the timer math break. Avoid. |

---

## 13. Open follow-ups

1. **Spread value source.** The existing canonical doc says "the spread is derived from the weapon/warhead" but doesn't enumerate the field. Likely candidates: `weapon.CellSpread` (no — that's the warhead's), `warhead.CellSpread` (likely), or a hardcoded constant. Trace `RadSiteClass.SetSpread` callers. Priority: MEDIUM — needed for damage radius parity.
2. **Per-unit radiation damage application site.** §7 working hypothesis is unverified. Find the function that reads `cell.RadLevel` and dispatches `ReceiveDamage` with `Rules.RadSiteWarhead`. Likely in `TechnoClass::AI_Update`. Priority: HIGH for parity.
3. **`RadLevelMax` clamp on augmentation.** Does `AddRadLevel` re-clamp to RadLevelMax? Important for cumulative-radiation balance. Trace AddRadLevel. Priority: MEDIUM.
4. **Cell.RadSite clearing on RadSite destruction.** When RadSite self-destructs at expiry, is `cell+0xF8` cleared to NULL? Or does it linger? Trace destructor. Priority: LOW (probably cleared via vtable destructor).
5. **`Rules.RadLevelFactor` consumer.** This is described as "factor for computing per-cell damage from level" but the exact formula for converting `cell.RadLevel × RadLevelFactor → damage` isn't traced. Priority: HIGH for parity (this controls how much HP per tick the rad cloud deals).
6. **`Rules.RadApplicationDelay` consumer.** Used as the per-unit damage tick interval but the exact gate site is not traced. Priority: MEDIUM.
7. **EMP cross-reference: `BuildingTypeClass+0x1701` mirror of `ImmuneToRadiation`.** When does the building parser set this offset? Trace. Priority: LOW (cosmetic — both checks use the same flag).
8. **RadSiteWarhead identity in retail.** Quote the `Rules.[Radiation] RadSiteWarhead=` value (likely `RadSite` or similar). Priority: LOW.

---

## 14. Sources

- Existing canonical doc: [`../../RADIATION_EMP_GHIDRA_REPORT.md`](../../RADIATION_EMP_GHIDRA_REPORT.md) (498 lines, Part 1: Radiation). Primary source — this systems doc migrates the radiation portion. Part 2 (EMP) will be migrated to [`emp.md`](emp.md) in a separate iteration.
- Live verification (2026-05-17):
  - `"Radiation"` at `0x00839E80` (single string match)
  - `"RadLevel"` at `0x00849298` (single string match)
  - `"ImmuneToRadiation"` at `0x00843854` (single string match)
- INI quotes from `ini/rulesmd.ini`:
  - `[Radiation]` section with 10 Rules keys
  - `[Desolator]`-family weapons set `RadLevel>0`
  - Warheads with `Radiation=yes`: `RadBeamWarhead`, `RadEruptionWarhead`, `RadSite`
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`rail_gun.md`](rail_gun.md) (Desolator usage of `RadLevel`), [`friendly_fire.md`](friendly_fire.md) (ImmuneToRadiation gate is in ReceiveDamage Step 7), [`emp.md`](emp.md) (when written).
