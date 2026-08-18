# Verses & Armor Matrix

> **2026-07-13 active-binary correction:**
> `disassemble_function(address="0x0075d590", program="gamemd.exe")` shows
> `ReadString(...,0x80,default=0x00847c40)` and a fixed 11-store loop at
> `0x0075de0c..0x0075de58`. `read_memory(address="0x00847c40", length=128,
> program="gamemd.exe")` decodes eleven `100%%` fallback tokens.
> `decompile_function(address="0x00528a10", program="gamemd.exe")` proves the
> 127-byte payload cap, forced NUL, and trim happen before tokenization; native
> `strtok` collapses empty fields. `disassemble_function(address="0x007caf30",
> program="gamemd.exe")` proves an exhausted token pointer is dereferenced by
> `strchr`, so a present nonempty short list faults. This block and the corrected
> sections below supersede the former “preserve missing tail/empty token = zero”
> claims.

This doc is the canonical reference for:

- The 11 armor types defined in gamemd.exe
- The `Verses=` INI key on every `[Warhead]` and how it's parsed
- The `Armor=` INI key on every TechnoType/TerrainType and how it's resolved to an index
- The `WarheadTypeClass.Verses[11]` array layout (double[11] at `wh+0xA0`)
- The `IsNonDamaging` derived flag (`wh+0x149`) — what triggers it and what consumes it
- Verses-driven side effects (primary/secondary weapon swap, ForceFire bypass)

Out-of-scope:
- The damage-time multiplication itself → [`damage_formula.md`](damage_formula.md) §6
- AA dispatch logic (when `IsAntiAir` plus Verses make Secondary swap) → [`anti_air_dispatch.md`](anti_air_dispatch.md)
- General target-gate machinery (cloaks, ForceFire, immunities) → [`can_target_gates.md`](can_target_gates.md)

---

## 1. The 11 armor types (verified table)

Verified by reading the name-pointer table at `0x007e5210..0x007e523B` (11 pointers × 4 bytes
= 44 bytes, mapped to ASCII strings). Indices are **case-sensitive** and lowercase.

| Index | INI name | String address | Notes |
|---:|---|---|---|
| 0 | `none` | `0x00817694` | "no armor" — bare infantry default |
| 1 | `flak` | `0x0081db78` | flak armor |
| 2 | `plate` | `0x0081db70` | plate armor |
| 3 | `light` | `0x0081db68` | light vehicle |
| 4 | `medium` | `0x0081db60` | medium vehicle |
| 5 | `heavy` | `0x0081db58` | heavy vehicle |
| 6 | `wood` | `0x0081db50` | wood-building |
| 7 | `steel` | `0x0081db48` | steel-building |
| 8 | `concrete` | `0x0081db3c` | concrete-building |
| 9 | `special_1` | `0x0081db30` | special slot 1 |
| 10 | `special_2` | `0x0081db24` | special slot 2 |

### Lookup function (Armor name → index)

`FUN_00772a50(name)` at `0x00772a50`:
```
int armorIndex = 0;
for ppuVar2 in &PTR_DAT_007e5210 .. 0x007e523c (step 4):
    if FUN_007c8d20(*ppuVar2, name) == 0:    // case-insensitive strcmp
        return armorIndex;
    armorIndex++;
return 0;   // unknown → defaults to "none"
```

Loop bound `0x007e523c = 0x007e5210 + 11*4` — exactly 11 entries scanned. An unknown
armor name silently maps to index 0 (`none`).

### Confidence (lookup)

- **Content: HIGH.** Decompilation read 2026-05-17; loop bound `0x7e523c - 0x7e5210 = 0x2c = 44 bytes = 11 entries` matches the 11 armor strings verified individually.
- **Identity: HIGH.** This is the unique consumer of the name table at `0x007e5210` (xrefs: `FUN_00772a50`, `FUN_004753f0` (the Armor= INI helper), and constructors `FUN_004b9890` / `FUN_00475404`).
- **Binding: HIGH.** Called from `FUN_004753f0` (the `Armor=` INI reader) which itself is called from `ObjectTypeClass__ReadINI` at `0x005f9490` line `param_1[0x27] = FUN_004753f0(piVar9, s_Armor_0081d9d4, param_1[0x27])`. `param_1[0x27]` is `+0x9C`. Every TechnoType/TerrainType funnels through this single call site.

### TS-legacy filter

All 11 armor types are live in YR; INI files use every slot (`none/flak` for infantry,
`light/medium/heavy` for vehicles, `wood/steel/concrete` for buildings,
`special_1/special_2` reserved for specific units — e.g. `special_1` is used by some
naval/aircraft units, `special_2` is used sparingly). No TS-only armor slot.

---

## 2. Target-side: where the Armor index is stored

`TechnoTypeClass.Armor` (also `TerrainTypeClass.Armor`) lives at **`typeClass + 0x9C`** as a 32-bit int:

| Offset | Type | Field | INI key |
|---|---|---|---|
| `+0x9C` | `int` | `Armor` | `Armor=` |

Verified at `ObjectTypeClass__ReadINI` (`0x005f9490`):
```
param_1[0x27] = FUN_004753f0(piVar9, s_Armor_0081d9d4, param_1[0x27]);
// param_1[0x27] = param_1 + 0x9C (39 * 4)
```

The third argument is the existing/previous value (used as default when the key is
absent). For brand-new types loaded from rules, the default is the engine's
default-armor for the WhatAmI class.

### Lookup at damage time

`damage_formula.md` §6 reads:
```
verses = wh->Verses[armorType];   // wh+0xA0 + armorType*8 (double)
```

Where `armorType` is the target's `TechnoTypeClass.Armor` (`+0x9C`). The compiler emits
a direct indexed load (no bounds check); an out-of-range index would read past the
Verses array into adjacent fields (`+0xF8` ProneDamage, `+0x100` DeformThreshold, etc.).
In practice the FUN_00772a50 lookup clamps to 0..10, so out-of-range never happens at
runtime.

---

## 3. Warhead-side: the `Verses[11]` array

| Offset | Type | Field | INI key |
|---|---|---|---|
| `wh+0xA0` | `double[11]` | `Verses` | `Verses=` |

Layout (verified by walking the parse loop):

```
wh+0xA0  = Verses[0]  vs none
wh+0xA8  = Verses[1]  vs flak
wh+0xB0  = Verses[2]  vs plate
wh+0xB8  = Verses[3]  vs light
wh+0xC0  = Verses[4]  vs medium
wh+0xC8  = Verses[5]  vs heavy
wh+0xD0  = Verses[6]  vs wood
wh+0xD8  = Verses[7]  vs steel
wh+0xE0  = Verses[8]  vs concrete
wh+0xE8  = Verses[9]  vs special_1
wh+0xF0  = Verses[10] vs special_2
```

### Default

The constructor initializes every slot to 1.0. If the section omits `Verses=`,
`ReadString` supplies eleven `100%%` tokens from `0x00847c40` and the normal
11-store parser runs. A present value that trims to length zero skips the parser
and preserves constructor values. These paths finish with the same one values
but are different mechanisms.

---

## 4. INI parsing (verified from `WarheadTypeClass__ReadINI`)

Function: `FUN_0075DD80` (the WarheadTypeClass ReadINI). At its tail:

```
iVar3 = CCINIClass__ReadString();          // read the raw "Verses=" line
if (iVar3 != 0):
    CRT__strtok(",");                       // tokenize on commas
    pdVar10 = unaff_ESI + 0xa0;             // pointer to Verses[0]
    iVar3 = 0xb;                            // 11 slots
    do:
        iVar8 = CRT__strchr(token, '%');    // is there a '%' in the token?
        if (iVar8 == 0):
            // No percent: decimal-style ("0.5")
            fVar11 = FUN_007c9d66(token);   // strtod-family full-f64 parse
        else:
            // Percent-style ("50%")
            iVar8 = CRT__atoi_wrapper(token);
            fVar11 = (double)iVar8 * 0.01;  // _g_ImpassableSpeedThreshold (== 0.01 const, reused)
        *pdVar10 = fVar11;
        CRT__strtok(NULL);
        pdVar10++;
        iVar3--;
    while iVar3 != 0;
```

### Key semantic notes

- **Percentage form** (`"100%"`): `atoi("100") * 0.01 = 1.0`. Note `atoi` stops at the
  `%`, so `"150%"` → `150 * 0.01 = 1.5`.
- **Decimal form** (`"0.5"`): the strtod-family reader returns full f64 0.5.
  Any non-numeric tail is handled by its native prefix semantics.
- **Mixing**: a single `Verses=` line may mix forms: `"100%,0.5,50%,..."` works.
- **Negative values** are not blocked — `"-50%"` would store `-0.5`. The damage formula
  clamps the spread result to `0` before multiplying by Verses (`damage_formula.md` §6),
  so a negative Verses still produces non-positive damage (which then truncates to 0
  or stays negative — the negative path is gated separately in §4 of that doc).
- **Bounded input and token count**: `ReadString` first caps the source at 127
  payload bytes, forces NUL, and trims. The loop reads exactly 11 tokens; extras
  are ignored. A nonempty list with fewer than 11 native `strtok` results does
  not preserve the tail: it faults on `strchr(NULL, '%')`.
- **Empty fields collapse**: native `strtok(",")` skips leading/trailing and
  repeated delimiters. Empty fields are not tokens and are not parsed as 0.0;
  they can instead make the fixed loop exhaust early and fault.
- **No clamping**: values above 1.0 (e.g. `"200%"`) produce damage multipliers — used by
  weapons like Tanya pistol (`Verses=200%` vs infantry).

### Confidence (parsing)

- **Content: HIGH.** Decomp read 2026-05-17. The 11-iteration loop, the `%`/decimal dispatch via `strchr`, and the `0.01` constant (relabelled `_g_ImpassableSpeedThreshold` in Ghidra — same constant reused) are all visible.
- **Identity: HIGH.** This is the Verses parser inside `WarheadTypeClass__ReadINI`; the xref from `Verses` string (`0x00847c38`) lands at `0x0075ddde` which is within this function's body.
- **Binding: HIGH.** `WarheadTypeClass__ReadINI` is called from the Rules INI bootstrap and is the only function that writes `wh+0xA0..wh+0xF0`. No other write site for the Verses array.

---

## 5. The `IsNonDamaging` derived flag

After the Verses loop, ReadINI checks:

```
if ( *(double*)(wh + 0xC0) == 0.0  &&   *(double*)(wh + 0xD0) == 0.0 ):
    *(byte*)(wh + 0x149) = 1;   // IsNonDamaging = true
else:
    *(byte*)(wh + 0x149) = 0;
```

`wh+0xC0` = Verses[4] (medium) and `wh+0xD0` = Verses[6] (wood). So **`IsNonDamaging`
is set when both `Verses[medium] == 0` AND `Verses[wood] == 0`** — i.e., the warhead
cannot damage a vehicle (medium) AND cannot damage a wood-building.

### Why this specific pair?

The flag is a fast pre-check: "this warhead is a utility/effect warhead, not a damage
warhead." Medium and wood are chosen because almost every player-targetable unit and
building falls under one or the other (vehicles and the most-common building armor),
so failing both is a strong signal of "purely a status/effect warhead" (Snapshot,
ChronoBeam, LocomotorBeam, BombDisarm-style).

### Consumers of `IsNonDamaging`

To be enumerated in a follow-up pass via Ghidra xrefs to `wh+0x149`. Used so far by:
- AI threat scoring (a non-damaging weapon doesn't contribute "danger" to a target).
- Some target-acquisition skips (a turret will not auto-acquire a target if its
  weapon's warhead is `IsNonDamaging` against the target's armor).

**Status:** xref enumeration deferred to a focused pass when implementing AI/target-acquisition (`target_acquisition.md`, `can_target_gates.md`).

### Confidence (IsNonDamaging derivation)

- **Content: HIGH.** Decomp shows the literal `if-else` block on `wh+0xc0`/`wh+0xd0` writing `wh+0x149`.
- **Identity: HIGH.** Single writer of `wh+0x149`; the offset matches existing doc `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` line for `IsNonDamaging`.
- **Binding: MEDIUM.** Only ONE writer confirmed (this function). Consumer list not yet exhaustively traced via xrefs — open follow-up.

---

## 6. `Verses=0` semantics: "weapon cannot target this armor"

When `wh.Verses[armor] == 0.0`:

1. **Damage path:** `damage_formula.md` §6 multiplies by 0.0, returns 0 damage. The MaxDamage clamp is irrelevant.
2. **Weapon-selection path:** in `TechnoClass::SelectWeaponAgainst` at `0x006f3330`, the engine prefers the **other** weapon (Primary → Secondary swap) when Primary's warhead has `Verses[target.Armor] == 0` but Secondary's is nonzero. This is the engine-side mechanism behind "anti-armor weapon vs infantry" auto-switching. Documented in [`anti_air_dispatch.md`](anti_air_dispatch.md) and [`can_target_gates.md`](can_target_gates.md).
3. **ForceFire (player Ctrl-click):** ForceFire bypasses the weapon-selection guard but **not** the damage formula — i.e., the player can manually direct a Verses-0 weapon at a target, but the damage delivered will be 0. The weapon still fires (consumes RoF/ammo), produces the muzzle flash and projectile, and detonates the warhead (so AnimList still plays), but the target takes no damage. Documented in [`can_target_gates.md`](can_target_gates.md).
4. **Auto-target / opportunity fire** treats Verses-0 targets as ineligible — the auto-acquire scan skips them. Documented in [`target_acquisition.md`](target_acquisition.md) and [`opportunity_fire.md`](opportunity_fire.md).

---

## 7. Convention used in rulesmd.ini

Examples extracted from `ini/rulesmd.ini`:

```
[SA]                ; small-arms warhead, infantry weapon default
Verses=100%,75%,60%,25%,20%,10%,4%,2%,2%,100%,100%
```
- 100% vs none, 75% vs flak, 60% vs plate, 25% vs light, ..., 4% vs wood, 2% vs steel/concrete, 100% vs special_1/2.
- Reads cleanly with the 11-token parser; the trailing `100%,100%` covers special_1/special_2.

```
[NUKE]              ; nuke superweapon warhead
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
```

```
[Snapshot]          ; Mirage disguise capture — non-damaging
Verses=0%,0%,0%,0%,0%,0%,0%,0%,0%,0%,0%
```
- All zero → `IsNonDamaging = true` (both medium and wood at 0).

```
[ChronoBeam]        ; Chrono Legionnaire freeze
Verses=0,0,0,0,0,0,0,0,0,0,0
```
- Also all-zero. The Temporal flag (`wh+0x15A`) drives the actual erase; Verses is zero because erase damage doesn't go through the Verses formula.

---

## 8. The full sub-pipeline

```
[rulesmd.ini]
   [WH-name]
   Verses=tok0,tok1,...,tok10
        │
        ▼
[WarheadTypeClass__ReadINI @ 0x0075DD80]
   strtok loop × 11
     per token:
       if '%' in token: ftol(atoi(token) * 0.01)
       else:            atof(token)
     → store at wh+0xA0+i*8 (double)
   after loop:
     if Verses[4]==0 && Verses[6]==0: wh+0x149 = 1  // IsNonDamaging
        │
        ▼
[per-target damage @ damage_formula.md §6]
   armor = target.TypeClass.Armor    // type+0x9C, int 0..10
   verses = wh.Verses[armor]
   damage = ftol(damage * verses)
        │
        ▼
[final damage clamped to MaxDamage, applied to target.Health]
```

---

## 9. Edge cases & footguns

| Case | Behavior |
|---|---|
| `Verses=100%` (single token) | First value stores, then the fixed loop reaches `strchr(NULL, '%')` and faults |
| `Verses=100%,50%` (2 tokens) | Two values store, then the fixed loop faults; no default tail is preserved |
| `Verses=` with whitespace ("100 %") | `strchr` finds `%`, `atoi("100 ")` returns 100 → 1.0. Whitespace tolerant on the percent side. Decimal-style `"0.5 "` is also tolerant. |
| `Verses=,,,,,,,,,,` | `strtok` yields no token; the first `strchr(NULL, '%')` faults |
| missing `Verses` | Parses the eleven-`100%%` fallback through all 11 stores |
| present empty/whitespace-only `Verses` | Bounded/trimmed length is zero; loop is skipped and constructor ones remain |
| source beyond 127 payload bytes | Truncated and forced-NUL before trim/tokenization; token 11 or the fault point can change |
| `Verses=200%` | `wh+0xA0 = 2.0`, doubles damage vs `none` armor |
| `Verses=-50%` | `wh+0xA0 = -0.5`. Spread-result is clamped to 0 before Verses multiply (`damage_formula.md` §6), so a healthy positive damage becomes 0; negative healing inputs are not affected (they take the early-out path before Verses). |
| Mix of `,` and `;` separators | Only `,` is a separator. `;` starts a comment in INI parsing (handled before reaching the Verses parser). |
| Multi-line `Verses=` | Not supported. Single line only. |

---

## 10. Open follow-ups

- Exhaustive xref enumeration of `wh+0x149` (`IsNonDamaging`) consumers — needed for `target_acquisition.md` and `can_target_gates.md`. Currently MEDIUM binding for the consumer side.
- Identify the second helper in the `Armor=` parsing path: `FUN_004753f0` (the wrapper around `FUN_00772a50`) — confirms case-insensitive comparison and default-fallback semantics. Not load-bearing for parity, but worth documenting.

---

## 11. Sources

- Live decompilation of `WarheadTypeClass__ReadINI` at `0x0075DD80` (read 2026-05-17, Verses parser at tail, IsNonDamaging gate at very end).
- Live decompilation of `FUN_00772a50` at `0x00772a50` (armor-name → index lookup).
- Live decompilation of `ObjectTypeClass__ReadINI` at `0x005f9490` (verified `Armor=` write site → `+0x9C`).
- Live memory reads at `0x007e5210..0x007e523B` (armor name pointer table) and dereference of each pointer to verify the 11 lowercase strings.
- `Verses` string xref via `get_xrefs_to 0x00847c38` → single DATA xref from `WarheadTypeClass__ReadINI 0x0075ddde`.
- Existing canonical doc: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) — cross-checked offsets; this doc supersedes that one for Verses-specific claims.
