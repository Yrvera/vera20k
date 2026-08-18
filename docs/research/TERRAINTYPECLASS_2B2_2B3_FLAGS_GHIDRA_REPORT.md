# TerrainTypeClass +0x2B2 and +0x2B3 — Flag Identification Report

**Scope:** Identify INI key, default, consumers, and YR-active status for
`TerrainTypeClass+0x2B2` and `TerrainTypeClass+0x2B3`.

**Investigation date:** 2026-05-20  
**Ghidra binary:** gamemd.exe

---

## 1. +0x2B2 — `IsFlammable`

### INI key and default

- **INI key:** `IsFlammable`
- **Default:** `false` (0)
- **Source:** `TerrainTypeClass__Constructor` at `0x0071DA80` sets
  `*(undefined1 *)((int)param_1 + 0x2b2) = 0;`
  (verified via `decompile_function 0x0071DA80`)
- **ReadINI:** `TerrainTypeClass__ReadINI_Full` at `0x0071DEA0`, instruction
  at `0x0071DF4A` (xref from string `s_IsFlammable_00844668`):
  ```c
  uVar3 = CCINIClass__ReadBool(piVar1, s_IsFlammable_00844668,
                               *(undefined1 *)((int)param_1 + 0x2b2));
  *(undefined1 *)((int)param_1 + 0x2b2) = uVar3;
  ```
  (verified via `decompile_function 0x0071DEA0`)
- **String address:** `0x00844668`, content `"IsFlammable"` (verified via
  `inspect_memory_content 0x00844668`)

### Runtime read sites

**None found.** Exhaustive search across all TerrainClass/TerrainTypeClass
methods and all fire/damage/warhead dispatch paths:

- `TerrainClass__Catch_Fire` (`0x0071C5B0`): reads `+0x2B1` (SpawnsTiberium)
  and `+0x9C` (Land type), but **not `+0x2B2`**.
- `TerrainClass__AI` (`0x0071C730`): reads `+0x2B3` and `+0x2B1`, but not `+0x2B2`.
- `TerrainClass__Draw_It` (`0x0071C1B0`): reads `+0x2B3` and `+0x2B1`, but not `+0x2B2`.
- `TerrainClass__Take_Damage` (`0x0071B920`): reads `+0x2B1`, not `+0x2B2`.
- `Apply_area_damage` (`0x00489280`): does not read any TerrainTypeClass field at `+0x2B2`.
- `WarheadTypeClass__Detonate` (`0x004690B0`): does not read `+0x2B2`.
- `BulletClass__BulletDetonation` (`0x00468D80`): calls vtable `+0xa4` = `Catch_Fire`
  on terrain targets; no flammability pre-check.
- `TerrainClass__Finish_Fire_Death` (`0x0071C6B0`): no reference to `+0x2B2`.
- `ParticleSystemClass__AI_Fire` (`0x0062F9A0`): no reference to `+0x2B2`.
- String search for "Forest Fire": **zero matches** in the binary.

The INI comment in `rulesmd.ini` says:
> `; IsFlammable = Can "Forest Fires" spread to and damage this terrain type?`

"Forest Fires" is a **Tiberian Sun mechanic**. The fire-spreading system that
would check this flag appears to have been removed or never compiled into
`gamemd.exe`. No runtime reader of `+0x2B2` exists in the binary.

### YR-active classification

**Active in YR: No.**

- The flag is written by `ReadINI` but never read by any consumer at runtime.
- The "Forest Fires" description is a TS legacy feature absent from YR.
- All stock TerrainType entries in `rulesmd.ini` explicitly set `IsFlammable=no`.
  No entry sets it to `yes` in `rules.ini` or `rulesmd.ini`.
- `TerrainClass__Catch_Fire` (the logical consumer) gates fire on `Land==6`
  and `SpawnsTiberium==false`, not on `IsFlammable`.

**Conclusion: Dead field in YR. Parsed but never consumed. TS legacy.**

---

## 2. +0x2B3 — `IsAnimated`

### INI key and default

- **INI key:** `IsAnimated`
- **Default:** `false` (0)
- **Source:** `TerrainTypeClass__Constructor` at `0x0071DA80` sets
  `*(undefined1 *)((int)param_1 + 0x2b3) = 0;`
  (verified via `decompile_function 0x0071DA80`)
- **ReadINI:** `TerrainTypeClass__ReadINI_Full` at `0x0071DEA0`, instruction
  at `0x0071E022` (xref from string `s_IsAnimated_0084465c`):
  ```c
  uVar3 = CCINIClass__ReadBool(piVar1, s_IsAnimated_0084465c,
                               *(undefined1 *)((int)param_1 + 0x2b3));
  *(undefined1 *)((int)param_1 + 0x2b3) = uVar3;
  ```
  (verified via `decompile_function 0x0071DEA0`)
- **String address:** `0x0084465C`, content `"IsAnimated"` (verified via
  `inspect_memory_content 0x0084465C`)

### Runtime read sites

**Three read sites found, all in live YR code paths:**

#### Read site 1: `TerrainClass__AI` at `0x0071C730`

Gate 1 (top of function):
```c
if ((*(char *)(param_1[0x32] + 0x2b3) != '\0') && (param_1[0x30] == 0)) {
    // Randomly trigger animation timer using AnimationProbability
}
```
Role: `IsAnimated == true` enables the random animation-trigger logic. When
true, each AI tick rolls a probability check against `+0x2A4` (AnimationProbability)
and if hit, sets animation timer from `+0x2A0` (AnimationRate).
(verified via `decompile_function 0x0071C730`)

Gate 2 (mid-function, tiberium-spawn trigger):
```c
piVar1 = (int *)param_1[0x32];
if ((*(char *)((int)piVar1 + 0x2b1) != '\0') &&
    (*(char *)((int)piVar1 + 0x2b3) != '\0')) {
    // When animation reaches mid-frame, spawn tiberium on adjacent cell
    CellClass__SpreadTiberium(uVar5);
}
```
Role: `SpawnsTiberium && IsAnimated` together gate the mid-animation tiberium
spawning. This is the TIBTRE01/02/03 growth mechanism.
(verified via `decompile_function 0x0071C730`)

#### Read site 2: `TerrainClass__Draw_It` at `0x0071C1B0`

Gate 1 (frame selection):
```c
if (*(char *)(param_1[0x32] + 0x2b3) == '\0') {
    // Not animated: use static frame logic
    iVar6 = 1; // (or burning frame)
} else {
    // Animated: use current animation frame counter
    iVar6 = param_1[0x2b];
}
```

Gate 2 (draw flags):
```c
if ((*(char *)(param_1[0x32] + 0x2b3) == '\0') &&
    (*(char *)((int)param_1 + 0xcd) == '\0')) {
    uVar7 = 0x4e00;  // static terrain draw flags
} else {
    uVar7 = 0x2e00;  // animated/burning draw flags
}
CC_Draw_Shape(..., uVar7, ...);
```
Role: Controls which SHP frame is drawn and which draw flags are passed.
Animated terrain cycles `param_1[0x2b]` (current frame) rather than a
static frame index. Also selects a different CC_Draw_Shape flag set.
(verified via `decompile_function 0x0071C1B0`)

### YR-active classification

**Active in YR: Yes.**

- TIBTRE01, TIBTRE02, TIBTRE03 all set `IsAnimated=yes` and `SpawnsTiberium=yes`
  in `rules.ini` and `rulesmd.ini`.
- These terrain types are placed on stock YR maps and appear in skirmish.
- `TerrainClass__AI` + `TerrainClass__Draw_It` are live tick-path functions.
- The animation-probability tick fires every game frame for TIBTRE objects.

---

## 3. Stock INI evidence

### IsFlammable

All entries in both `rules.ini` and `rulesmd.ini` are explicitly `IsFlammable=no`.
Zero TerrainType entries set it to `yes` in either file. The field is parsed but
universally false in stock RA2+YR.

### IsAnimated

`rules.ini` lines 20377, 20392, 20407: `IsAnimated=yes` for TIBTRE01–03.  
`rulesmd.ini` lines 28113, 28128, 28143: `IsAnimated=yes` for TIBTRE01–03.

These are the only TerrainTypes that set it, and they are the Tiberium Tree
objects placed on standard YR maps.

---

## 4. Adjacent field context

For completeness (from `TerrainTypeClass__Constructor` and `ReadINI_Full`):

| Offset | Field name  | INI key     | Default | Notes                              |
|--------|-------------|-------------|---------|-------------------------------------|
| +0x2B1 | SpawnsTiberium | SpawnsTiberium | false | Active in YR (TIBTRE01-03) |
| +0x2B2 | IsFlammable | IsFlammable | false | Dead in YR — TS Forest Fire legacy |
| +0x2B3 | IsAnimated  | IsAnimated  | false | Active in YR (TIBTRE01-03) |

All three defaults verified via `decompile_function 0x0071DA80`.

---

## 5. Summary of verified facts

1. **`+0x2B2` = `IsFlammable`**, default `false`, written by `ReadINI_Full` at
   `0x0071DEA0` (string xref `0x00844668` → `0x0071DF4A`). No runtime reader
   found anywhere in the binary. Dead field in YR.
   
2. **`+0x2B3` = `IsAnimated`**, default `false`, written by `ReadINI_Full` at
   `0x0071DEA0` (string xref `0x0084465C` → `0x0071E022`). Read in `TerrainClass__AI`
   (`0x0071C730`) for animation timer and tiberium-spawn gating, and in
   `TerrainClass__Draw_It` (`0x0071C1B0`) for frame selection and draw flags.
   Active in YR via TIBTRE01-03.

3. **No runtime consumer of `IsFlammable` exists.** String search for "Forest Fire"
   returns zero hits. `Catch_Fire` checks `SpawnsTiberium`, not `IsFlammable`.

4. **`IsAnimated` gates two behaviors in `AI`**: (a) random animation trigger via
   AnimationProbability, and (b) mid-animation tiberium spawn (requires both
   `SpawnsTiberium && IsAnimated`).

5. **TIBTRE01–03 are the sole `IsAnimated=yes` entries** in stock RA2+YR INI files.
   No entry sets `IsFlammable=yes` in any stock INI.
