# RulesClass Constructor Defaults

**Function:** `FUN_00665650` (RulesClass constructor, body 0x00665650–0x00667A26)
**Param:** `undefined4 *param_1` — pointer to a freshly-allocated RulesClass
  instance (`operator_new(0x18C0)` in `CCFileClass__Constructor` at 0x0052BAD8). (corrected 2026-05-29: was "Init_Game"; binary label at 0x0052BAD8 is `CCFileClass__Constructor` via decompile_function 0x0052BAD8 — RTTI_LABEL_DRIFT)

**Canonical data:** [RULESCLASS_CONSTRUCTOR_DEFAULTS.csv](RULESCLASS_CONSTRUCTOR_DEFAULTS.csv) — 1087 field-init rows, columns:
`offset, size, type, value, form`.

## Summary stats

| Metric | Count |
|---|---:|
| Total store statements | 1087 |
| Unique offsets written | ~1050 (3 per DynVec init: vtable+capacity+size) |
| Zero-init stores | 386 |
| `0xFFFFFFFF` stores (unset sound index / -1) | 105 |
| Vtable pointer writes (DynVec/DynStr constructors) | 106 |
| 1-byte (bool) stores | 65 |
| Highest stored offset | `0x18B8` (+ 4 bytes = 0x18BC, matches `operator_new(0x18C0)` size) |

## Form legend

All four forms mean "write this value to a field in RulesClass":

| Form | Pattern | Offset calc |
|:---:|---|---|
| A | `param_1[N] = V;` | byte `N × 4` |
| B | `*(type*)(param_1 + N) = V;` | byte `N × 4` (int-stride pointer arith) |
| C | `*(type*)((int)param_1 + N) = V;` | byte `N` (direct cast) |
| D | `*param_1 = V;` | byte `0` |

The CSV's `offset` column has already applied the scaling — all offsets are bytes.

## How to use

Cross-reference with [RULESCLASS_FIELDS.csv](RULESCLASS_FIELDS.csv) (the INI-reader
offset map) to decide, for any given field:

- **Ctor-default + INI reader** (normal case) — INI silence means "stock YR value".
- **INI reader, no ctor default** (potentially uninitialized) — DynVec strings aren't
  literal defaults but shouldn't be treated as "0". Usually these are `0` or point
  to a trivial vtable via the DynVec init pattern.
- **Ctor-default, no INI reader** (runtime-only / unused) — internal caches or
  TS-legacy fields the rules parser doesn't touch. Keep the ctor value as
  runtime default.

Task 7 in the decode plan produces a structured cross-ref (this doc + the
fields CSV joined by offset) in the consolidation pass.

## Recognized value patterns

The extractor annotates common IEEE-754 bit patterns in the `value` column:

| Bit pattern | Meaning |
|---|---|
| `0x3ff00000` | double `1.0` (high 4 bytes of 1.0) |
| `0x3fe00000` | double `0.5` |
| `0x3fd00000` | double `0.25` |
| `0x40000000` | double `2.0` |
| `0x40240000` | double `10.0` |
| `0x3f800000` | float `1.0f` |
| `0x3f000000` | float `0.5f` |
| `0x3f4ccccd` | float `~0.8f` |
| `0x3f19999a` | float `~0.6f` |
| `0x3ecccccd` | float `~0.4f` |
| `0x3d4ccccd` | float `~0.05f` |
| `0xffffffff` | `-1` / unset sound index / unset type* |

Doubles always span **two consecutive rows**: low bits at offset `N`, high bits
at `N+4`. Field at offset `0x1748` = `0, 0x3ff00000` → `BuildSpeed = 1.0` (INI
overrides to `0.7` via `[General] BuildSpeed=.7`).

## Not captured by this extractor

1. **ColorAdd table init loop** (offset `0x1874`, 16 × 3-byte RGB entries). The
   constructor runs a `do {...} while` loop to zero the 48 bytes; the extractor
   only captures direct `param_1[N]=V` statements, not loop bodies. The loop is
   visible near the end of the ctor decomp at `puVar2 = param_1 + 0x61d; ...`.

2. **Helper-call field setup.** DynVec fields are initialized via
   `FUN_00525680(0,0)` / `FUN_00477BE0(0,0)` / `FUN_005105A0(0,0)` etc. followed
   by three direct stores (vtable + capacity + size). The three stores ARE
   captured; the helper call is not. Effect: DynVec field offsets appear as
   three consecutive rows at `N, N+4*(1-3), N+4*(size_idx)` starting 1 entry
   per vector.

3. **FUN_0067c310 / FUN_0067c3a0 / FUN_0067c430** DynStr-like constructors
   used for 16-byte string fields (same pattern).

## Cross-reference with struct size

The RulesClass instance size from `CCFileClass__Constructor` (0x0052BAD8): `operator_new(0x18C0)` = **6336 bytes**.

- Highest ctor store: `0x18B8` (4-byte int, value `0x3f800000` = 1.0f)
- Tail `0x18BC–0x18BF` (4 bytes): not stored by ctor — likely padding.
- Total covered: `0–0x18B8` inclusive.

Field at `0x18B4` and `0x18B8` both store `0x3f800000` (float `1.0f`) — these
are two adjacent float fields at the tail of the struct. Candidate role: aspect-
ratio or zoom multipliers (no `[General]` / `[AudioVisual]` INI key known to
write here — flag as runtime-only until Task 7 cross-ref confirms).

## Notable clusters

Cluster-by-address patterns from the decomp:

| Offset range | Content | INI section |
|---|---|---|
| `0x00–0x5B` | Tuning constants (veterancy, reload, repair, color-scheme start) | `[General]` head |
| `0x5C–0xFC` | Ratio constants, `FreeMCV` bool, overlay ptrs | `[General]` + `[CrateRules]` |
| `0x140–0x25F` | Various anim / sound ptrs, 4×4-entry rocket configs | `[General]` missiles/V3/DMisl/CMisl |
| `0x2B0–0x3DF` | SplashList, scorches, flame warheads, overload tables, drain config | `[CombatDamage]` |
| `0x3E0–0x48F` | Secret unit slots, special-weapon warheads, occupy/bunker/opentopped | `[CombatDamage]` + `[SpecialWeapons]` |
| `0x490–0x56F` | Ion/Nuke, PsychicReveal, radar damage, particle-system defaults | `[CombatDamage]` |
| `0x570–0x7B7` | Hover/track/wheel terrain speeds, flight level, build-rate tuning | `[General]` locomotion + build rates |
| `0x7C4–0x85F` | Scorch lists (`[CombatDamage] Scorches` through `Scorches4`) | `[CombatDamage]` |
| `0x8AC–0xAE0` | AI build-category DynVecs | `[AI]` |
| `0xAF8–0xB20` | AI wall/base-defense coefficients | `[AI]` |
| `0xBC0–0xBDF` | SplashList DynVec | `[CombatDamage]` |
| `0xBE0–0xFE0` | Chrono, miner, all warhead ptrs, Ivan, drain, death weapon | `[General]` + `[CombatDamage]` |
| `0xFE8–0xFF3` | Iron Curtain duration, Psychic Reveal, Ion Cannon warhead | `[CombatDamage]` |
| `0x1018–0x103B` | Default smoke/spark/fire/particle systems | `[CombatDamage]` |
| `0x1098` | TurboBoost | `[CombatDamage]` |
| `0x10A0–0x1103` | AI ratios/limits | `[AI]` |
| `0x1108–0x112F` | War/Defense/AA/Tesla/Helipad/Airstrip + Paranoid/CompEasy | `[AI]` |
| `0x1138–0x115F` | InfantryReserve, PathDelay, BlockagePathDelay | `[AI]` |
| `0x117C–0x132F` | **Embedded DifficultyClass slots** (3 × 0x4C = 0xE4 bytes somewhere here) | `[Easy]`/`[Normal]`/`[Difficult]` |
| `0x1428–0x1440` | ExpSpread, FireSupress, IQ base | `[CombatDamage]` + `[IQ]` |
| `0x1460` | AIBaseSpacing | `[AI]` |
| `0x1464–0x1474` | Silver/Wood/Water crate types + min/max | `[CrateRules]` |
| `0x1480–0x14BB` | MultiplayerDialogSettings block | `[MultiplayerDialogSettings]` |
| `0x14CC–0x14FC` | — (gap, mostly 0, possible TS-legacy) | — |
| `0x1510–0x1528` | AutocreateTime, misc AI timers | `[AI]` |
| `0x1530` | AtomDamage | `[CombatDamage]` |
| `0x15C0–0x16BF` | — (gap, ~400 bytes mostly 0) | — (runtime / TS-legacy region) |
| `0x16C0–0x16C8` | Incoming, MinDamage, MaxDamage | `[CombatDamage]` |
| `0x16D0–0x16FC` | RepairPercent, RepairRate, LEGACY vein/weed | `[General]` + TS |
| `0x1700–0x1748` | ConditionYellow, ConditionRed, IdleActionFrequency, Shroud/Fog/Vein/IceGrowth, BuildSpeed | `[AudioVisual]` + `[General]` |
| `0x1748–0x175F` | BuildSpeed=1.0, MultipleFactory, RepairRate, tibBreak | `[General]` |
| `0x1768–0x1770` | BlockagePathDelay, LightningDamage/StormDuration | `[General]` |
| `0x17CC` | CollapseChance | `[CombatDamage]` |
| `0x17E0–0x17F3` | Compact bool block — `Paranoid`, `TiberiumExplosive`, `PlayerAutoCrush`, `TreeTargeting`, `NamedCivilians`, `EnemyHealth`, `AllyReveal`, `BerzerkAllowed`, `RevealByHeight`, `AllowShroudedSubteranneanMoves`, etc. | `[CombatDamage]` + `[AudioVisual]` + `[AI]` + `[General]` |
| `0x1804–0x1834` | Radiation block (9 fields) | `[Radiation]` |
| `0x1838–0x184F` | Elevation + Wall models | `[ElevationModel]`/`[WallModel]` |
| `0x1860–0x186B` | Local RGB, LineTrail, ChronoBeam, MagnaBeam colors | `[AudioVisual]` |
| `0x186C–0x1870` | OreTwinkleChance, OreTwinkle | `[AudioVisual]` + `[General]` |
| `0x1874–0x18A3` | **ColorAdd** table (16 × 3-byte RGB) | `[ColorAdd]` |
| `0x18A4–0x18B8` | Laser/IronCurtain/Berserk/ForceShield color-palette indices + direct-rocking/fallback float coefs | `[AudioVisual]` |

Most clusters follow the logical ordering of their owning Read_* function.

## Validation

BuildSpeed cross-check:
- INI (`rulesmd.ini` `[General]`): `BuildSpeed=.7`
- Ctor default: offset `0x1748`+`0x174C` = `0x0, 0x3FF00000` → IEEE-754 `1.0`
- Reader: [General] BuildSpeed at `0x1748` (RULESCLASS_FIELDS.csv)
- Interpretation: INI overrides to `0.7`; without INI the runtime default would be `1.0`. ✓

OccupyDamageMultiplier cross-check:
- INI: `[CombatDamage] OccupyDamageMultiplier=1.2`
- Ctor default: offset `0xF40` = `0x3F800000` (float `1.0f`) — runtime field is a *float* (single-precision)
- Reader: [CombatDamage] OccupyDamageMultiplier at `0xF40` as `double` ... but **ctor stored 4 bytes** → field is `float`, NOT `double`. The reader casts.
- Confirmed via earlier ReadCombatDamage decomp: `*(float *)(param_1 + 0xf40) = (float)fVar6;` — so reader writes as float too. ✓

IronCurtainDuration cross-check:
- INI: `[CombatDamage] IronCurtainDuration=750`
- Ctor default: offset `0xFE8` = `0x2000` (hex) = `8192` (decimal) — wait, let me check.

Actually: looking at ctor line `param_1[0x3fa] = 0;` and `param_1[0x3fb] = 3;` and scanning forward... The IronCurtainDuration field at 0xFE8 (byte) = index 0x3FA. Ctor stores 0 at 0x3FA... no, actually need to find the exact row.

(Runtime validation deferred to Task 7's formal join pass.)

## Field-specific notes

- **`CrewEscape`** at `0x16B8` — ctor stores `3` (meaning 0.06 probability? or just the raw int?). INI has `CrewEscape=50%` which is an int-percent.
- **`TiberiumHeal`** at `0x1730` region — `0x280`, `0x200`, `0x200`, `0x180`... these are packed bytes likely. The ctor at indices `0x5C6–0x5CD` stores such mixed values — need careful interpretation per field.
- **Aspect ratio pair at `0x18B4, 0x18B8`** both `0x3f800000` (1.0f) — two consecutive floats, possibly UI scale factors or camera-related. No INI reader known.
- **Many DynVec fields** start with `&PTR_FUN_007e4dd8` (int vector vtable), `&PTR_FUN_007eb6d4` (string vector), `&PTR_FUN_007eac08` (anim vector), `&PTR_FUN_007ed90c` (BuildingType vector), `&PTR_FUN_007eaa08` (generic DynVec vtable) — each vector has initial capacity=10, size=0.

## Next steps

**Task 7** (cross-ref defaults vs INI readers) consumes this CSV + `RULESCLASS_FIELDS.csv`
and emits three tables:
- Fields with both ctor-default and INI-reader (happy path)
- Fields with INI-reader but no ctor-default (flag as potentially UB on INI-silent)
- Fields with ctor-default but no INI-reader (runtime-only / dead TS-legacy)

**Task 8** (DifficultyClass slot bases) uses the ctor's 3 DynVec+helper call patterns
in the `0x117C–0x132F` region to pin the 3 × 0x4C embedded DifficultyClass instances.

## Sources

- Ghidra address: `0x00665650`
- Decomp saved to: `scripts/research/_decomp/FUN_00665650.c`
- Extractor script: `scripts/research/extract_ctor_defaults.py`
- CSV: `ra2-rust-game-docs/RULESCLASS_CONSTRUCTOR_DEFAULTS.csv`
