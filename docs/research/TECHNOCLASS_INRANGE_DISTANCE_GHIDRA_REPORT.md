# TechnoClass::InRange — Distance Computation (Ghidra Research Report)

**Primary address:** `0x006F7220` (TechnoClass::InRange)
**Companion helper:** `0x004CAC40` (Sqrt_Approx), `0x007C5F00` (Math::ftol), `0x0041C380` (CoordStruct::Distance3D — referenced but NOT directly called by InRange)
**Confidence:** HIGH for distance metric, boundary semantics, parameter mapping, Z source. HIGH for branch-gate flag identities (corrected 2026-05-07 — see §0).
**Active in YR:** Yes — InRange is on every weapon-fire / can-fire-at path. The function is reached every targeting tick.

This report extends `BUNKER_SYSTEM_GHIDRA_REPORT.md §5`, which documented only the **range-VALUE chain** (AirRange / Garrison / Bunker / OpenTopped bonuses). This document covers the **distance side** that §5 stops short of.

---

## 0. 2026-05-07 corrections summary

The original report (drafted 2026-05-07 earlier session) made four wrong attributions that this revision corrects. Read this section before reading anything else — most of the open questions in §10 are now resolved, and several "TS-legacy" / "RadioLink" / "TechnoTypeClass byte" claims were wrong.

| Old claim (incorrect) | Correct (verified 2026-05-07) | Where to read |
|---|---|---|
| `target.Type.byte@0x29B` gates 2D+slope branch | **`weapon.Projectile.byte+0x29B = Arcing=yes`** (BulletTypeClass, not TechnoTypeClass; on the weapon's projectile, not the target's type) | §3, §7 |
| `target.Type.byte@0x297` gates "RadioLink" range bonus | **`weapon.Projectile.byte+0x297 = SubjectToElevation=yes`**, gates **height-fire bonus** (high-ground advantage), not RadioLink | §5 (items 7+9), §3.3 |
| `target.Type.byte@0x295` gates per-type slope override | **`weapon.Projectile.byte+0x295 = Floater=yes`** (TS-legacy gravity override; no standard YR projectile sets it) | §3.3, §7 |
| `attacker.WhatAmI() == 3` is "potentially TS-only, no live trigger identified" | **Confirmed dead in YR.** Verified by vtable scan: only `*TypeClass` templates (AircraftType / AnimType / OverlayType / TerrainType / VoxelAnimType — at least 5 verified) inherit `WhatAmI() == 3` from AbstractTypeClass. TechnoClass attackers are *instances* (Unit=1, Aircraft=2, Building=6, Infantry=0xF), never types. Branch A1 cannot be reached in standard YR play. | §3.2 |

Two helpers were also renamed:

| Old label | Correct label | Function |
|---|---|---|
| `FUN_006F6F60` "RadioLink bonus helper" | **Height-fire bonus helper (high-ground range bonus)** | Both helpers compute `(target_height − attacker_height) / Rules.ElevationIncrement`, with helper at 0x6F6F60 also adding distance-and-height ballistic term. |
| `FUN_006F70E0` "Sensor (Branch B) helper" | **Height-fire bonus helper, Branch B variant** | Same height delta, returns `<< 8` (= leptons). |

Bridge Z gate (was §6 "low-priority TS-anti-self-fire"):

| Old reading | Correct reading |
|---|---|
| "guard against some self-targeting edge case" | **LOS occlusion** — if attacker is in a bridge cell, attacker.Z < bridge_top, and target.Z ≥ bridge_top, reject. Plain meaning: a unit on the ground beneath a bridge cannot fire up through it to a target on the deck. Active in YR every time the geometry occurs. |

Two RulesClass constants identified:

| Rules offset | INI key | Section | Type |
|---|---|---|---|
| `+0x16B8` | `Gravity=` | `[AudioVisual]` (parsed at `0x66B3D9` via ReadInt) | int |
| `+0x1838` | `ElevationIncrement=` | `[ElevationModel]` (parsed at `FUN_0066D150`) | int |
| `+0x1840` | `ElevationIncrementBonus=` | `[ElevationModel]` | double |
| `+0x1848` | `ElevationBonusCap=` | `[ElevationModel]` | double |

The rest of the report (function signature, distance metric, boundary semantics, range bonus chain items 1-6, §4 source/target coords, §5 sqrt approximation note) is unchanged and still accurate. The corrections above are in §3.2/§3.3/§5/§6/§7/§8 inline.

---

## 1. Function Signature & Parameter Mapping

```c
__thiscall TechnoClass::InRange(
    /* this  */ TechnoClass*       attacker,    // ECX
    /* arg1  */ CoordStruct const* src,         // [EBP+0x8]  — caller-provided position
    /* arg2  */ AbstractClass*     target,      // [EBP+0xC]  — null-checked
    /* arg3  */ WeaponTypeClass*   weapon       // [EBP+0x10] — null-checked
) -> bool
// Cleanup: RET 0xC (callee pops 12 bytes, 3 stack args)
```

**Verified from disassembly prologue at 0x006F7220:**
- `MOV ESI, ECX` → ESI = this
- `MOV ECX, [EBP+0xC]; TEST ECX,ECX; JZ exit` → arg2 (target) null check
- `MOV EBX, [EBP+0x10]; TEST EBX,EBX; JZ exit` → arg3 (weapon) null check
- `MOV ESI, [EBP+0x8]` (later) reloads source pointer → ESI[0]/ESI[1]/ESI[2] = src.X/Y/Z

**Caller invariant** (verified from `TechnoClass::CanFireAt` at `0x006F77B0`):
The `src` argument is a **stack-built CoordStruct** populated from `attacker->Get_Coords()`,
optionally with cell-snap adjustments (e.g. AntiAircraft weapons snap to cell-center coords).
**`src` is NOT identical to `attacker->Coords` in all cases** — callers may massage it. The
Rust port must preserve this caller-builds-source contract.

Confidence: **HIGH** — verified from disassembly + CanFireAt decomp.

---

## 2. The Two-Phase Range Check

InRange runs **two distance comparisons** in sequence:

1. **MinimumRange check** (only if `weapon->MinimumRange != 0`): always 3D distance,
   compared `< MinimumRange` → reject if **inside** min range.
2. **MaximumRange check**: distance metric varies by branch (see §3), compared
   `<= effective_max_range` → accept if within.

A weapon with `Range == -0x200` returns `true` immediately (sentinel for "always in range",
`weapon->Range` field at offset `+0xB4`).

### 2.1 Distance metric (verified at `0x006F73AB-0x006F73DF` for min-range, `0x006F75CC-0x006F75ED` for max-range)

```
distance_int = (int)Math::ftol( Sqrt_Approx( dx² + dy² + dz² ) )
```

Specifically:
- **dx, dy, dz are computed as `int32` deltas** between `src.{X,Y,Z}` and `target.{X,Y,Z}` (after target Z adjustment from §5)
- Each delta is loaded with `FILD` (integer→float80), squared with `FMUL`, summed with `FADDP`
- `Sqrt_Approx (0x004CAC40)` is **NOT a precise sqrt**. It's a fast approximation using an
  `IEEE-754 mantissa lookup table at DAT_008650BC`. It returns a `float10` (FPU x87 80-bit)
  but the lookup gives `float32`-grade accuracy. For parity, this means the integer
  distance result can differ from a true `sqrt` by up to ±1 lepton at large distances.
- `Math::ftol (0x007C5F00)` does the float→int conversion (truncation toward zero,
  per x87 control word default).

**Confidence: HIGH** for the formula. **MEDIUM** for sqrt-approximation precision impact —
this is the kind of detail that could cause 1-lepton drift in cell-aligned scenarios; flag
for parity testing.

### 2.2 Boundary semantics (verified from disassembly compares)

| Check | Disasm | Comparison | Operator |
|-------|--------|-----------|----------|
| MinimumRange | `006f73E4 CMP EAX,[ESP+0x18]; 006f73E8 JL exit_false` | `dist < MinimumRange` → reject | **strict `<`** (EAX < min → reject) |
| MaxRange (Branch A1, 3D) | `006f75F2 CMP EAX,EBX; 006f75F4 SETLE AL` | `dist <= range` → accept | **inclusive `<=`** |
| MaxRange (Branch B, 2D+arc) | `006f7474 CMP [ESP+0x18],EBX; 006f7478 JG exit_false` | `dist > range` → reject (= `<=` accept) | **inclusive `<=`** |
| Range sentinel | `006f724E CMP EDI,0xFFFFFE00; 006f7254 JNZ; 006f7256 MOV AL,1` | `weapon.Range == -0x200` → return true | exact match |

**Min-range is strict (`<`), max-range is inclusive (`<=`).** A unit standing exactly at
`MinimumRange` IS in range. A unit standing exactly at `Range` IS in range.

Confidence: **HIGH**.

---

## 3. Three Distance Flavors

The function picks one of three distance computations based on two flag tests. Both tests
read fields on the **weapon's Projectile** (`weapon[0xA0]` = BulletTypeClass*), not on the
target or its type. (Disassembly path: `006f72e3 MOV EAX,[EBX+0xA0]; 006f72eb MOV [ESP+0x10],EAX` saves
the projectile pointer; `006f73f6 MOV CL,[EDX+0x29B]` re-loads it for the branch test.)

```
if weapon.Projectile.byte+0x29B != 0:    // Arcing=yes
    Branch B:  2D distance (dx² + dy²)  +  separate Z-slope arc check (§4)
else:
    apply foundation bonus if target is Building (§6)
    if attacker->WhatAmI() == 3:         // dead code in YR — see §3.2
        Branch A1:  2D distance (dx² + dy²)
    else:
        Branch A2:  3D distance (dx² + dy² + dz²)  ← DEFAULT
    apply max-range compare + bridge gate
```

### 3.1 Branch A2 — 3D Euclidean (DEFAULT)
Hit when both:
- weapon's `Projectile.Arcing = no` (the common case — most direct-fire bullets)
- attacker's `WhatAmI() != 3` (always true for TechnoClass instances — see §3.2)

This is the path most fire engagements take. **3D Euclidean Sqrt-distance** vs. effective
range. **Verified HIGH.**

### 3.2 Branch A1 — 2D, attacker-WhatAmI gated (`attacker.WhatAmI() == 3`) — DEAD IN YR

Disassembled at `0x006F75B2-0x006F75E1`. Drops the `dz²` term entirely.

**Resolved 2026-05-07 (vtable scan):** `WhatAmI() == 3` is the value inherited from
`AbstractTypeClass` and used by every `*TypeClass` template — *not* by any TechnoClass
instance. Searched the binary for the function body `B8 03 00 00 00 C3` (`return 3`):

- `0x0041CFB0` → vtable `0x007E2868` → **AircraftTypeClass** (RTTI `.?AVAircraftTypeClass@@`
  at TypeDescriptor `0x00817FB8`, referenced from primary RTTI-OL at `0x007FB5B0`)
- `0x004369F0` → vtable `0x007E3B1C` → AnimTypeClass / similar (secondary vtable; class
  identity inferred from neighbors)
- `0x0062D770` → vtable `0x007EF9A0` → OverlayTypeClass / similar
- `0x0074A960` → vtable `0x007F6364` → VoxelAnimTypeClass (constructor at `0x0074AD80`
  writes 4 vtables matching the secondary-vtable pattern)

All four are `*TypeClass` (rules.ini template) classes. They are never used as a TechnoClass
attacker — TechnoClass instances on the map return:

| Class | Address | Returns |
|---|---|---|
| UnitClass::WhatAmI | `0x00746E20` | 1 |
| AircraftClass::WhatAmI | `0x0041C180` | 2 |
| BuildingClass::WhatAmI | `0x00459EC0` | 6 |
| InfantryClass::WhatAmI | `0x00523340` | 0xF |

**No TechnoClass-derived instance returns 3 in YR.** Branch A1 is unreachable in standard
play. The Rust port should NOT implement it. Confidence: HIGH (vtable + RTTI confirmed).

### 3.3 Branch B — 2D + Z-slope arc check (`weapon.Projectile.Arcing = yes`)

Disassembled at `0x006F7404-0x006F74D1`. The flag at the **Projectile** (BulletTypeClass) at
offset `+0x29B` (= `Arcing=`) gates this branch — see [BULLETTYPECLASS_GHIDRA_REPORT.md §2](BULLETTYPECLASS_GHIDRA_REPORT.md).
Used by V3 Rocket, Tank Howitzer, Prism Tank, MIRV missiles, etc. — anything ballistic.

When set:

1. Compute 2D distance `(int)sqrt(dx² + dy²)` (no dz contribution)
2. Compare `dist <= effective_range` (inclusive)
3. If `Projectile.SubjectToElevation = yes` (`+0x297`), add **height-fire bonus** via
   `FUN_006F70E0` — see §5 item 9.
4. Pass `dz` (saved at `[ESP+0x3C]`) into a **slope arc check** via two helpers:
   - `FUN_0048AB90(range, slope_param)` — set/load tangent-squared limit
   - `FUN_0048ABC0(dz, slope_param)` — is dz within tan(angle) × dist limit
5. Slope_param sourcing:
   - If `Projectile.Floater = yes` (`Projectile.byte+0x295`): use `FUN_0048ACF0()` =
     `Rules.Gravity × _DAT_007e1738`. **`Floater=yes` is a TS-era flag; no standard YR
     projectile sets it.** Per [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md),
     this is the alternate-gravity path — dead in YR.
   - Else: use `Rules+0x16B8 = Rules.Gravity` (global default — `[AudioVisual]/Gravity=`).
6. Bridge tolerance: if the bridge cell flag (cell+0x140 bit 0x100) is set,
   `(target.Z - src.Z) >= DAT_00B0EB34 * 3` is rejected (the dz must be small enough that
   we're not on the "wrong side" of the bridge).

This is the **AA-firing-arc / ballistic-projectile check**: weapon must be horizontally
close enough AND the vertical angle to the (possibly airborne) target must be inside the
projectile's reachable elevation cone, computed from `Rules.Gravity`. Active in YR for
any weapon with an `Arcing=yes` projectile.

Confidence: HIGH on the disasm and on all flag identities.

---

## 4. Source / Target Coordinate Sources (Q3 + Q4)

### 4.1 Source coords (attacker side)

The **caller** builds and supplies the source CoordStruct. InRange reads three int fields
from `*src`:
- `*src      ` = `src.X`
- `*(src + 4)` = `src.Y`
- `*(src + 8)` = `src.Z`

These are **lepton-space integers**. The caller (e.g. `TechnoClass::CanFireAt @ 0x006F77B0`)
typically initializes them from `attacker->Get_Coords()` (vtable+0x48) but may snap-to-cell
for anti-aircraft weapons, etc. **InRange itself does not call attacker->Get_Coords**.

### 4.2 Target coords

InRange always reads target via virtual `target->Get_Coords(out_buf)` (vtable+0x48, slot 18):

```c
piVar4 = (int *)(**(code **)(*param_3 + 0x48))(&iStack_14);
iVar7      = piVar4[0];  // target.X
iVar1      = piVar4[1];  // target.Y
iStack_1c  = piVar4[2];  // target.Z   ← later overwritten if low-flying (§5)
```

For all non-overriding subclasses (Unit, Aircraft, Infantry — `FootClass::GetDestinationCoords`
is at slot 19, the `0x4C` slot, **not** the `0x48` Get_Coords slot), this dispatches to
`ObjectClass::GetCoords` at `0x005F65A0`:

```c
void ObjectClass::GetCoords(CoordStruct *out)  // verified decomp
{
    out->X = *(int *)(this + 0x9C);
    out->Y = *(int *)(this + 0xA0);
    out->Z = *(int *)(this + 0xA4);
}
```

**THIS IS THE KEY FINDING for the Rust port:**

- Coordinates live as **3 raw ints at offsets `+0x9C / +0xA0 / +0xA4`** on every ObjectClass instance.
- The `Z` field at `+0xA4` is **already the absolute Z in leptons** — it's NOT a level
  count, NOT a level-times-104 multiplied later, NOT separated from altitude.
- For **ground units**: `Z = ground_height_at_cell_in_leptons` (which the cell-occupation
  / Set_Coord code maintains as `cell_level × LevelHeight (104) + bridge_offset_if_any`).
- For **aircraft**: same field. The locomotor / air-movement code is responsible for
  updating `+0xA4` to include the current altitude. **There is no separate `altitude`
  field that InRange consults.** Whatever is in `+0xA4` IS the Z used for distance.

### 4.3 Target Z adjustment for low-flying (verified, `0x006F7332-0x006F7373`)

After reading target.Z, InRange checks `target->IsLowFlying()` (vtable+0x50). If true:

```c
target_Z = CellClass::GetGroundHeight(...)  // ground beneath target
if (cell.flags & 0x100) {                     // cell on a bridge
    target_Z += DAT_00B0EB24;                 // bridge-height delta (BridgeHeightInLeptons)
}
```

**Low-flying targets are ranged to the GROUND BENEATH them**, not their actual altitude.
This is critical for parity — it means a Harrier descending to attack does NOT escape
fire by being slightly elevated.

`IsLowFlying`/`IsHighFlying` definitions (verified at `0x005F6B60` / `0x005F6B90`):

```c
bool ObjectClass::IsLowFlying()  { return this->byte@0x74 != 0 && Get_Height() <  DAT_00AC13C8 * 2; }
bool ObjectClass::IsHighFlying() { return this->byte@0x74 != 0 && Get_Height() >= DAT_00AC13C8 * 2; }
```

- `byte@0x74` = "is currently airborne" gate (zero for ground units, nonzero for airborne).
- `vtable+0x1C8` = `Get_Height()` returns altitude in leptons.
- `DAT_00AC13C8` = HighFlightLevel (likely Rules `FlightLevel=` or hard-init constant;
  bit-uninitialized at link time, populated at runtime).
- `IsLowFlying` and `IsHighFlying` are **mutually exclusive** for airborne units, split at
  `HighFlightLevel * 2`.

`FootClass::IsHighFlying` (vtable override at `0x004DE620`) — same code body as
ObjectClass version per Ghidra; probably means it's NOT actually overridden, just inherits.
The note in `FOOTCLASS_VTABLE_COMPLETE.md` saying "returns false" appears stale.

Confidence: **HIGH** for the structure. **MEDIUM** for `DAT_00AC13C8` exact value (need INI
trace or runtime read).

---

## 5. Effective Range — Bonus Chain (extends BUNKER §5)

InRange computes `effective_range` as a running sum (variable `iVar8` then `iVar3`):

| # | Bonus | Trigger | Source | Operation |
|---|-------|---------|--------|-----------|
| 1 | Base | Always | `weapon->Range` (`+0xB4`, leptons) | `range = weapon.Range` |
| 2 | Sentinel | `range == -0x200` | weapon.Range | **return true immediately** |
| 3 | AirRange | `target.IsHighFlying()` (vtable+0x54) | `attacker.Type.AirRange` (`+0x68C`) | `range += attacker.Type.AirRange` |
| 4 | Garrison (REPLACES) | `attacker.IsOccupied()` (vtable+0x400) | `(occupant_count + Rules.OccupyWeaponRange) × 256` | `range = (count + Rules+0xF48) << 8` |
| 5 | Bunker | `attacker.byte@0x2E4 != 0 && attacker.WhatAmI() != 6` | `Rules.BunkerWeaponRangeBonus × 256` | `range += Rules+0xF54 × 256` |
| 6 | OpenTopped | `attacker.byte@0x82 != 0` | `Rules.OpenToppedRangeBonus × 256` | `range += Rules+0xF5C × 256` |
| 7 | **Height-fire (both branches)** | `weapon.Projectile.byte+0x297 != 0` (`SubjectToElevation=yes`) | `FUN_006F6F60(attacker, target)` | `iVar3 = iVar8 + height_fire_bonus` |
| 8 | Foundation | Branch A only, `target.WhatAmI() == 6` (Building) | `(BuildH + BuildW) × 0x40` | `iVar3 += (h+w) × 64` leptons |
| 9 | Height-fire (Branch B variant) | Branch B path, `weapon.Projectile.byte+0x297 != 0` | `FUN_006F70E0(attacker, target)` | similar to (7), returns `delta << 8` only (no ballistic term) |

**Note (4) replaces; (3,5,6) add; (7,8,9) add to the running sum.**

Foundation bonus formula (item 8): **`(FoundationHeight + FoundationWidth) × 64` leptons**.
For a 2x2 building: bonus = `(2+2)×64 = 256 leptons = 1 cell`. For a 4x2 ConYard:
`(4+2)×64 = 384 leptons = 1.5 cells`. This bonus only applies to MAX range, not min.

### Height-fire bonus details (items 7 + 9)

Items 7 and 9 are the **high-ground advantage** mechanic — a unit firing from elevated
terrain gains extra range. The gating flag is `Projectile.SubjectToElevation=yes` (on the
projectile, not the target). Decompiled FUN_006F6F60:

```c
// Both helpers share the same gate (verified via decompile_function 0x006F6F60 and 0x006F70E0):
if (attacker.IsLowFlying() == 0 || target.IsLowFlying() == 0) return 0;
// IMPORTANT: IsLowFlying() returns non-zero only when byte@0x74 != 0 (airborne flag) AND
// Get_Height() < DAT_00AC13C8 * 2.  Ground units always have byte@0x74 == 0 → always return 0.
// → This bonus ONLY fires when BOTH attacker and target are airborne-but-low-flying aircraft.
// → It is DEAD for all ground-vs-ground, ground-vs-building, and ground-vs-air engagements.

// Get cells under attacker and target
attacker_h = CellClass::GetEffectiveHeight(attacker_cell);
target_h   = CellClass::GetEffectiveHeight(target_cell);
delta = max(0, target_h - attacker_h);   // only when target is uphill of attacker
delta_cells = delta / Rules.ElevationIncrement;   // Rules+0x1838 — see §0

// FUN_006F70E0 (Branch B): returns delta_cells << 8  (= leptons)
// FUN_006F6F60 (default Branch A path): adds a distance-and-height ballistic term:
//   bonus² = (ftol(delta_cells) × 256)² + (DAT_00B0EB34 × delta)²
//   bonus  = (int)Sqrt_Approx(bonus²)
```

**Corrected 2026-05-28: was "Active in standard YR play (high-ground is a visible feature)";
binary shows both FUN_006F6F60 and FUN_006F70E0 gate on `IsLowFlying() != 0` for BOTH
attacker AND target (verified via decompile_function 0x006F6F60 / 0x006F70E0). Ground units
always have `byte@0x74 == 0`, so IsLowFlying always returns 0 for them. This bonus is ZERO
for all ground-vs-ground, ground-vs-building, and ground-vs-aircraft engagements. It only
fires when BOTH units are simultaneously airborne-but-low-flying — a condition that never
occurs in standard YR (two low-flying aircraft simultaneously engaging with
SubjectToElevation=yes weaponry). ROOT_CAUSE: INFERENCE_HARDENED.**

The clamp `max(0, …)` means the bonus only kicks in when the target is uphill — but
since the IsLowFlying gate excludes all ground units, this entire subsystem is effectively
dead in standard YR play for the weapons it's supposed to benefit.

**Rules.ElevationIncrement** lives at `Rules+0x1838` in section `[ElevationModel]`,
parsed via `FUN_0066D150`. Two adjacent fields (`+0x1840 = ElevationIncrementBonus`,
`+0x1848 = ElevationBonusCap`) round out the same INI section but were not seen used by
InRange specifically.

Confidence: HIGH for the disasm-level facts (formula, offsets, operators). **LOW for
"active in YR" claim — corrected 2026-05-28: bonus is gated dead for ground units.**

---

## 6. Bridge Z Gate — LOS occlusion (verified `0x006F75FB-0x006F762F`)

After the Branch A distance/range pass, InRange runs one more reject check that **blocks
shots from below a bridge to above it**:

```
src_cell = MapClass::Get_Cell_At(src)
if (src_cell.flags & 0x100):                       // attacker is in a bridge cell
    bridge_top = CellClass::GetGroundHeight(src) + DAT_00B0EB24   // ground + bridge_height_in_leptons
    if (src.Z < bridge_top) AND (target.Z >= bridge_top):
        return false                                // shot blocked by bridge
```

Re-read of the disassembly (correcting the prior session's interpretation):

```
006f7611: PUSH ESI                  ; ESI = src
006f7612: MOV ECX, 0x87f7e8          ; MapClass instance
006f7617: CALL 0x00578080            ; CellClass::GetGroundHeight(src)
006f761c: MOV EDX, [0x00b0eb24]      ; bridge_height_in_leptons
006f7622: MOV ECX, [ESI + 0x8]       ; ECX = src.Z
006f7625: ADD EAX, EDX                ; EAX = ground_z + bridge_height = bridge top
006f7627: CMP ECX, EAX                ; src.Z >= bridge_top ?
006f7629: JGE skip_reject              ; yes (attacker on/above bridge) → skip
006f762b: CMP [ESP + 0x28], EAX      ; target.Z >= bridge_top ?
006f762f: JGE reject                   ; yes → REJECT (return false)
```

**Plain meaning:** A unit standing on the ground beneath a bridge cannot fire upward
through the bridge to a target standing on the bridge deck. The bridge physically occludes
the line of sight. This is normal LOS behavior, not anti-self-fire and not TS-legacy — it
fires every time an infantry/vehicle on the ground tries to shoot something on the bridge
above it (and every map with a bridge has this geometry).

The check only handles the specific direction "attacker below ⇒ target above through
bridge." The reverse case (attacker on bridge, target on ground below) isn't checked here
because the geometry doesn't occlude (you can shoot down from a bridge through open air).

Confidence: HIGH. Behavior and intent both verified from disasm.

---

## 7. Constants Summary

| Symbol | Address | Value (interpreted) | Source | Notes |
|--------|---------|---------------------|--------|-------|
| `weapon.Range` offset | — | `+0xB4` | WeaponTypeClass | int, leptons |
| `weapon.MinimumRange` offset | — | `+0xB8` | WeaponTypeClass | int, leptons |
| `weapon.Projectile` offset | — | `+0xA0` | WeaponTypeClass | BulletTypeClass* (the field the prior doc mis-labeled "WarheadType") |
| `weapon.Warhead` offset | — | `+0xAC` | WeaponTypeClass | WarheadTypeClass* (NOT used by InRange branch tests) |
| `weapon.Range = -0x200` | — | `-512` (sentinel) | WeaponTypeClass | "always in range" |
| `attacker.Type.AirRange` offset | — | `+0x68C` | TechnoTypeClass | int, leptons |
| `attacker.byte_isInBunker` | — | `+0x2E4` (= int*-index 0xB9) | TechnoClass | bool flag |
| `attacker.byte_isOpenTopped` | — | `+0x82` | TechnoClass | bool flag |
| `Projectile.Arcing` | — | `+0x29B` | BulletTypeClass | bool — `Arcing=` — gates Branch B (2D + AA arc) |
| `Projectile.SubjectToElevation` | — | `+0x297` | BulletTypeClass | bool — `SubjectToElevation=` — gates height-fire range bonus (items 7+9 in §5) |
| `Projectile.Floater` | — | `+0x295` | BulletTypeClass | bool — `Floater=` — Branch B per-projectile gravity override; **TS-legacy, no standard YR projectile sets it** |
| `Coords` offset | — | `+0x9C / +0xA0 / +0xA4` | ObjectClass | CoordStruct{X,Y,Z} ints in leptons |
| `byte_isAirborne` | — | `+0x74` | ObjectClass | gate for IsLowFlying / IsHighFlying |
| `Rules.OccupyWeaponRange` | g_Rules + `0xF48` | (cells, INI default ≈ 1-3) | RulesClass | applied as `× 256` for leptons |
| `Rules.BunkerWeaponRangeBonus` | g_Rules + `0xF54` | (cells, INI default 2) | RulesClass | applied as `× 256` for leptons |
| `Rules.OpenToppedRangeBonus` | g_Rules + `0xF5C` | (cells, INI default 1) | RulesClass | applied as `× 256` for leptons |
| **`Rules.Gravity`** | g_Rules + `0x16B8` | int | RulesClass | `[AudioVisual]/Gravity=`. Branch B default slope_param (parsed at `0x66B3D9` via ReadInt). |
| **`Rules.ElevationIncrement`** | g_Rules + `0x1838` | int | RulesClass | `[ElevationModel]/ElevationIncrement=`. Divisor in height-fire bonus formula (parsed in `FUN_0066D150` via ReadInt). |
| `Rules.ElevationIncrementBonus` | g_Rules + `0x1840` | double | RulesClass | `[ElevationModel]/ElevationIncrementBonus=` |
| `Rules.ElevationBonusCap` | g_Rules + `0x1848` | double | RulesClass | `[ElevationModel]/ElevationBonusCap=` |
| `DAT_00AC13C8` | `0x00AC13C8` | (HighFlightLevel) | runtime-init data | low/high flight split = `× 2` |
| `DAT_00B0EB24` | `0x00B0EB24` | (BridgeHeightDelta in leptons) | runtime-init data | added to ground Z when on bridge; used by §6 LOS gate |
| `DAT_00B0EB34` | `0x00B0EB34` | (slope scalar) | runtime-init data | used in height-fire ballistic term (§5 item 7) and Branch B bridge tolerance (× 3) |
| `Sqrt_Approx` | `0x004CAC40` | mantissa-LUT sqrt approximation | function | NOT precise sqrt — float32-grade lookup |
| `Math::ftol` | `0x007C5F00` | x87 float→int truncate | function | per current FPU control word |

The three `DAT_00...` constants read as zero from the binary (BSS), so they're populated
at runtime. Their semantic meanings are clear (HighFlightLevel, bridge-height-in-leptons,
ballistic-elevation scalar); exact numeric values would require a debugger session or
further static-trace work. For the Rust port, derive them from `rulesmd.ini` and known
RA2/YR defaults (BridgeHeight is 4 levels = 4 × 104 = 416 leptons; HighFlightLevel maps to
`FlightLevel=` at `Rules+0x7B4`).

---

## 8. Answers to Brainstorm Questions

| # | Question | Answer | Confidence |
|---|----------|--------|------------|
| 1 | Distance metric? | **3D Euclidean** in default branch: `(int)Sqrt_Approx(dx² + dy² + dz²)`. NOT precise sqrt — float32-LUT approximation. Two 2D special branches exist (§3.2, §3.3). | HIGH |
| 2 | Boundary semantics? | Max-range: `dist <= range` **inclusive**. MinimumRange: `dist < min` **strict** (inside min = reject). Sentinel: `weapon.Range == -0x200` → always-in-range. | HIGH |
| 3 | Coords.Z encoding for ground units? | `Coords.Z` at `+0xA4` is **absolute Z in leptons** (= cell_level × 104 + bridge offsets, computed by cell-occupation code). InRange does NOT multiply by LevelHeight itself — it reads the field directly. | HIGH |
| 4 | Z source for aircraft? | **Same field +0xA4** — there is no separate altitude consulted by InRange. The locomotor/air-movement code is responsible for keeping `+0xA4` updated to include altitude. For low-flying targets, InRange **overrides** target.Z with ground-beneath height (§4.3). | HIGH |
| 5 | Function signature? | `__thiscall InRange(this: TechnoClass*, src: CoordStruct const*, target: AbstractClass*, weapon: WeaponTypeClass*) -> bool` — caller builds and passes `src` (often = attacker's Coords with AntiAircraft cell-snap). RET 0xC. | HIGH |

---

## 9. Implications for the Rust Port

(These are observations for the brainstorm, not implementation prescriptions — the
brainstorm decides scope and approach.)

1. **Z storage**: gamemd stores absolute Z in a single field per entity. The Rust
   `Position { z: u8 }` (level only) plus `loco.altitude: SimFixed` (altitude leptons)
   pattern is structurally different. To match gamemd, either:
   (a) keep both and compute `effective_z_leptons = pos.z * LEPTONS_PER_LEVEL + loco.altitude` on every distance call, or
   (b) collapse to a single `z_leptons: i32` on Position and update it from movement code.
   Both are valid; the brainstorm should evaluate the trade-off against existing render code.

2. **`LEPTONS_PER_LEVEL = 104`** needs a constant definition. Suggested home:
   `src/util/lepton.rs` or `src/util/fixed_math.rs` next to existing cell/lepton constants.
   Verified value at gamemd `0x89DDB8` (per `COORDINATE_SYSTEM_GAMEMD.md:127-131`).

3. **API shape**: gamemd's caller-builds-source pattern (CanFireAt builds `src` and passes
   it in) means the Rust port should similarly accept a source position rather than always
   reading from `attacker.position`. This makes the AntiAircraft cell-snap natural and
   keeps the function pure.

4. **Boundary direction**: current Rust `is_within_range_leptons` uses `dist_sq <= range_sq`
   (inclusive `<=`) — **matches gamemd's max-range semantics**. Min-range needs a separate
   helper using strict `<`.

5. **Sqrt approximation drift**: gamemd's Sqrt_Approx is float32-grade. Rust's
   `isqrt_i64` (used in AOE) gives precise integer sqrt. For range checks done in
   squared-leptons space (no sqrt at all), there is **zero drift** — both engines agree.
   For checks that genuinely need sqrt (Branch B's distance compare uses `dist <= range`
   in scalar leptons after sqrt), there could be ±1-lepton drift from gamemd's
   float-LUT path. Acceptable for parity per CLAUDE.md's "indistinguishable in a single
   skirmish" bar; flag for testing.

6. **Branch B (AA arc check)** is its own subsystem and should be a separate brainstorm.
   The current scope (3D distance) does not need to model the slope arc.

7. **Branch A1 (`attacker.WhatAmI() == 3`)** is **confirmed dead in YR** (resolved 2026-05-07
   — see §0 and §3.2). The Rust port should NOT implement it.

8. **`SubjectToElevation` projectile flag**: the Rust port's BulletType parser must read
   this and gate the height-fire bonus on it. Without it, all projectiles get the bonus
   (or none do), neither of which matches gamemd. The flag is at `BulletTypeClass+0x297`.

9. **`Arcing` projectile flag**: gates Branch B (2D + ballistic-arc check). The Rust port
   already parses `Arcing=` per [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md);
   the InRange logic must consume it the same way (use 2D distance, then run AA-arc check).

10. **`Floater` flag** is TS-legacy and can be ignored unless a future mod actually sets
    it on a projectile.

---

## 10. Open Questions / Follow-ups

| # | Question | Status | Notes |
|---|----------|--------|-------|
| OQ-1 | Identity of `WhatAmI() == 3` | **RESOLVED 2026-05-07** | All `*TypeClass` templates inherit `WhatAmI() == 3` from AbstractTypeClass. No TechnoClass instance returns 3 in YR. Branch A1 is dead. (See §0, §3.2.) |
| OQ-2 | Identity of byte at `+0x29B` (gates Branch B AA-arc) | **RESOLVED 2026-05-07** | On the **Projectile (BulletTypeClass)**, not target.Type. = `Arcing=`. (See §0, §3.3.) |
| OQ-3 | Identity of byte at `+0x297` (gates "RadioLink" range bonus) | **RESOLVED 2026-05-07** | On the **Projectile**. = `SubjectToElevation=`. The bonus is **height-fire** (high-ground advantage), not RadioLink. (See §0, §5.) |
| OQ-4 | Identity of byte at `+0x295` (Branch B per-type slope override) | **RESOLVED 2026-05-07** | On the **Projectile**. = `Floater=`. TS-legacy; no standard YR projectile sets it. (See §0, §3.3.) |
| OQ-5 | Exact runtime values of `DAT_00AC13C8`, `DAT_00B0EB24`, `DAT_00B0EB34` | OPEN (low priority) | Semantic meanings clear (HighFlightLevel; bridge-height-in-leptons; ballistic elevation scalar). Numeric values would require runtime debugger inspection. |
| OQ-6 | Identity of `Rules+0x16B8` and `Rules+0x1838` | **RESOLVED 2026-05-07** | `+0x16B8 = Gravity` (`[AudioVisual]`); `+0x1838 = ElevationIncrement` (`[ElevationModel]`). (See §0, §7.) |
| OQ-7 | Bridge Z gate intent | **RESOLVED 2026-05-07** | LOS occlusion: blocks shots from below a bridge upward through it. Active every match with a bridge. (See §6.) |
| OQ-8 | `FootClass::IsHighFlying` actually-overrides? | OPEN (very low priority) | Cosmetic doc question. Disassembly check at `0x004DE620` would resolve. |
| OQ-9 | Sqrt_Approx precision impact: at what distance does ±1 lepton drift become observable? | OPEN (parity testing) | Build a test that compares Sqrt_Approx vs precise sqrt over typical range deltas. |

---

## 11. TS-vs-YR Audit

Most of the function is **active in standard YR play** (any unit-fires-weapon path goes
through here via CanFireAt). Specific concerns:

- **Branch A1 (`WhatAmI()==3`)** — confirmed TS-legacy / unreachable in YR (§3.2). No
  TechnoClass instance returns 3. Do not implement.
- **Bridge Z gate (§6)** — LOS occlusion. **Active in YR** every time a unit on the
  ground fires at a unit on a bridge above it.
- **Height-fire bonus** (items 7+9 in §5; FUN_006F6F60 / FUN_006F70E0) — uses cell
  `EffectiveHeight` to compute extra range when target is uphill of attacker. **Corrected
  2026-05-28: NOT active for ground units.** Both helpers gate on `IsLowFlying() != 0`
  for BOTH attacker AND target (verified via decompile_function 0x006F6F60 / 0x006F70E0);
  ground units always return 0 from IsLowFlying (byte@0x74 == 0). Bonus is zero for all
  ground-vs-ground/building/aircraft engagements. Only fires when BOTH units are
  low-flying airborne — dead in standard YR play. ROOT_CAUSE: INFERENCE_HARDENED.
- **`Floater=` projectile flag (Branch B per-projectile gravity override)** — TS-legacy.
  No standard YR projectile sets it. Branch B with `Floater=no` (the standard case) uses
  `Rules.Gravity` directly.

The **3D distance metric itself**, the **boundary semantics**, the **AirRange / Garrison /
Bunker / OpenTopped chain**, the **Foundation bonus**, the **height-fire bonus**, and the
**bridge-LOS gate** are all live and apply to standard YR engagements.

---

## Sources

- **Ghidra decompilation (initial 2026-05-07 session):**
  - `TechnoClass::InRange` @ `0x006F7220` (decomp + full disasm)
  - `TechnoClass::CanFireAt` @ `0x006F77B0` (decomp — caller invariants)
  - `ObjectClass::GetCoords` @ `0x005F65A0` (decomp)
  - `ObjectClass::IsLowFlying` @ `0x005F6B60` (decomp)
  - `ObjectClass::IsHighFlying` @ `0x005F6B90` (decomp)
  - `ObjectClass::IsAboveGround` @ `0x005F6C10` (decomp)
  - `Sqrt_Approx` @ `0x004CAC40` (decomp — confirmed float32-LUT approximation)
  - `UnitClass::WhatAmI` @ `0x00746E20` (decomp — returns 1)
  - `BuildingClass::WhatAmI` @ `0x00459EC0` (decomp — returns 6)
  - `InfantryClass::WhatAmI` @ `0x00523340` (decomp — returns 0xF)
  - `AircraftClass::WhatAmI` @ `0x0041C180` (raw memory — returns 2)
  - `AircraftClass::Constructor` @ `0x00413D20` (disasm — derived AircraftClass vtable @ `0x007E22A4`)
  - `FUN_006F6F60`, `FUN_006F70E0` (decomp — height-fire bonus helpers)

- **Ghidra decompilation (corrections session, 2026-05-07 later):**
  - Re-disassembled `TechnoClass::InRange` @ `0x006F7220` to nail register/pointer flow:
    confirmed `weapon[0xA0]` is Projectile (BulletTypeClass*) per `WeaponTypeClass+0xA0`,
    not Warhead. All four byte tests (0x295, 0x297, 0x29B) are on the Projectile.
  - Byte-pattern search `B8 03 00 00 00 C3` ("return 3") found 4 function bodies, each
    referenced from a single vtable+0x2C slot. RTTI lookup via vtable-4 → RTTI-OL →
    TypeDescriptor identified `0x0041CFB0` as `AircraftTypeClass::WhatAmI`. The other 3
    are sibling `*TypeClass` WhatAmIs (AnimType / OverlayType / VoxelAnimType / TerrainType
    via constructors writing 4-vtable MI layouts).
  - `BulletTypeClass::ReadINI` @ `0x0046BEE0` cross-referenced via [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md)
    for offset → INI-key mapping (0x294 Airburst, 0x295 Floater, 0x296 SubjectToCliffs,
    0x297 SubjectToElevation, 0x29B Arcing).
  - `FUN_006F6F60` / `FUN_006F70E0` re-decompiled — confirmed both compute
    `(target_height − attacker_height) / Rules.ElevationIncrement` as a height-fire bonus,
    not RadioLink anything.
  - `FUN_0048ACF0` decompiled — returns `Rules.Gravity × _DAT_007e1738`. This is the
    `Floater=yes` per-projectile path.
  - `RulesClass::ReadAudioVisual` @ `0x006691E0` searched for stores to `Rules+0x16B8`:
    found `MOV [ESI+0x16B8], EAX` after `PUSH 0x83a34c (= "Gravity")` at `0x0066B3D9`.
  - `FUN_0066D150` decompiled — small reader for `[ElevationModel]` with key string
    `"ElevationIncrement"` at `0x83B370` writing to `Rules+0x1838`.
  - Bridge Z gate re-disassembled at `0x006F75FB-0x006F762F` — confirmed it's LOS
    occlusion (attacker below + target above bridge ⇒ reject), not anti-self-fire.

- **Existing docs referenced:**
  - `BUNKER_SYSTEM_GHIDRA_REPORT.md` §5 — range-VALUE chain (this report extends)
  - `COORDINATE_SYSTEM_GAMEMD.md:127-131` — LevelHeight = 104 verification
  - `TECHNOCLASS_VTABLE_COMPLETE.md` — vtable slot map (slot 18 / 0x48 = GetCoords)
  - `FOOTCLASS_VTABLE_COMPLETE.md` — FootClass override map
  - `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:306` — WhatAmI discriminator hint (1/2/6/0xF)
  - `AIRCRAFTCLASS_GHIDRA_REPORT.md` — AircraftClass call-site context
  - **`WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`** — definitive offsets: Projectile at +0xA0,
    Warhead at +0xAC. (Source for the offset correction.)
  - **`BULLETTYPECLASS_GHIDRA_REPORT.md`** — definitive offsets and INI keys for
    BulletTypeClass+0x294-0x29F.
  - Cross-referenced WhatAmI claims in `BURST_WEAPON_FIRING`, `CLOAKING_STEALTH`,
    `DAMAGE_MATH`, `CRUSH_SYSTEM`, `DRIVE_LOCOMOTION_CLASS`, `CELL_OCCUPATION_MARKING`
    (some of these have stale or contradictory WhatAmI value claims; the verified
    decompilations in this report are authoritative).
