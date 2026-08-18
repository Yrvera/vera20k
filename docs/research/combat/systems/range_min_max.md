# Range — Min / Max / Effective

This doc is the canonical reference for the **range check** in gamemd.exe:

- `Range=` and `MinimumRange=` on WeaponTypeClass
- The `Range = -0x200` sentinel ("always in range")
- The two-phase check (MinimumRange first, then Range)
- Distance metric in each of the three branches (3D Euclidean, 2D, 2D + ballistic arc)
- Effective-range bonus chain: AirRange, Garrison-occupy, Bunker, OpenTopped, height-fire (elevation), Foundation
- Source/target coordinate sources, low-flying target Z override
- Bridge LOS occlusion gate
- TS-legacy filter (one dead branch identified)

Out-of-scope:
- Cadence after the check passes → [`rof_burst_timing.md`](rof_burst_timing.md)
- The ballistic-arc check itself in detail → [`projectile_arc_gravity.md`](projectile_arc_gravity.md)
- AA dispatch upstream of range check → [`anti_air_dispatch.md`](anti_air_dispatch.md)
- CanFireAt full pipeline (which builds `src` and calls InRange) → [`fire_at_pipeline.md`](fire_at_pipeline.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Address | `0x006F7220` |
| Ghidra label | `TechnoClass__InRange` (named in current annotation set) |
| Calling convention | `__thiscall(attacker: TechnoClass*, src: CoordStruct const*, target: AbstractClass*, weapon: WeaponTypeClass*) → bool` |
| Stack cleanup | `RET 0xC` (callee pops 12 bytes — 3 stack args) |
| Caller (verified) | `TechnoClass::CanFireAt` at `0x006F77B0` |

### Caller invariant

`src` is a **stack-built CoordStruct** populated by the caller, typically from
`attacker->Get_Coords()` (vtable+0x48) but possibly with cell-snap adjustments (e.g.
anti-aircraft weapons may snap to cell-center). **InRange does NOT call
`attacker->Get_Coords` itself** — the caller-builds-source pattern is part of the API
contract.

### Confidence

- **Content: HIGH** (function decompiled and disassembled 2026-05-07; live re-decomp on 2026-05-17 matches).
- **Identity: HIGH** (named function with verified disassembly prologue and `RET 0xC` epilogue matching the 3-arg signature).
- **Binding: HIGH** (called from `CanFireAt 0x006F77B0`, verified by decomp of that function in the existing canonical doc).

---

## 2. Two-phase range check

```
1. MinimumRange check (only if weapon.MinimumRange != 0)
   - Distance metric: 3D Euclidean (always)
   - Comparison: dist < MinimumRange → REJECT (strict <)
2. MaximumRange check
   - Distance metric: 3D or 2D depending on Projectile.Arcing
   - Comparison: dist <= effective_range → ACCEPT (inclusive <=)
3. Bridge LOS gate (Branch A only, after distance pass)
```

### Sentinel: `Range = -0x200`

If `weapon.Range == -0x200` (= -512), the function **returns true immediately** —
"always in range." Used by superweapon launchers and dispatch weapons.

### Boundary semantics (verified)

| Check | Comparison | Operator |
|---|---|---|
| MinimumRange | `dist < MinimumRange` → reject | **strict `<`** |
| MaxRange (3D, Branch A2) | `dist <= range` → accept | **inclusive `<=`** |
| MaxRange (2D+arc, Branch B) | `dist > range` → reject (i.e., `<=` accept) | **inclusive `<=`** |
| Range sentinel | `weapon.Range == -0x200` → return true | exact match |

So: standing exactly at `MinimumRange` IS in range; standing exactly at `Range` IS in range.

### Confidence

- **Content: HIGH** (verified at disassembly instruction level — `CMP EAX,[ESP+0x18]; JL exit_false` for min, `SETLE AL` for max).
- **Identity: HIGH** (single function, no other gates with these exact comparators).
- **Binding: HIGH** (this is the only place in the binary that does the full range check; CanFireAt calls into here).

---

## 3. Distance metric

```
dist_int = (int)Math::ftol( Sqrt_Approx( dx² + dy² + dz²_optional ) )
```

- `dx, dy, dz` are `int32` deltas in **leptons**, loaded via FILD (integer→float80) and squared with FMUL.
- `Sqrt_Approx` at `0x004CAC40` is **NOT a precise sqrt** — uses an IEEE-754 mantissa LUT at `DAT_008650BC`. Float32-grade precision. Can drift ±1 lepton at large distances vs a true `sqrt`.
- `Math::ftol` at `0x007C5F00` truncates toward zero (per default x87 control word).

### When dz is included

| Branch | Distance | Trigger |
|---|---|---|
| A2 (default) | 3D: `√(dx² + dy² + dz²)` | `Projectile.Arcing == 0` AND `attacker.WhatAmI() != 3` |
| A1 (dead) | 2D: `√(dx² + dy²)` | `Projectile.Arcing == 0` AND `attacker.WhatAmI() == 3` — **unreachable in YR** (see §5) |
| B (ballistic) | 2D: `√(dx² + dy²)`, plus separate slope-arc check | `Projectile.Arcing != 0` (`Arcing=yes`) |

### Minimum-range distance is always 3D

The MinimumRange compare runs **before** the Arcing branch decision, and always uses
the 3D metric. A high-arcing weapon's minimum range therefore includes altitude in
the inside-cone check.

### Confidence

- **Content: HIGH** (formula and sqrt approximation verified at disassembly).
- **Identity: HIGH** (Sqrt_Approx is uniquely used in distance contexts; LUT at `DAT_008650BC` is its sole input).
- **Binding: HIGH** (only InRange consumes this exact sqrt-then-ftol pattern with leptons).

### Sqrt drift note

For Rust ports that work in **squared leptons** (range² compares dist²), there is zero
drift. For paths that genuinely require the scalar `dist` value (e.g., Branch B's
inclusive `dist <= range` compare after sqrt + ftol), ±1 lepton drift is possible at
~30+ cells. Below the parity-bar pain threshold in practice (cell-aligned movement
makes the integer dist value snap, masking small float drift), but flag for testing.

---

## 4. Three distance flavors (branch decision)

The branch test reads two flags on the **weapon's Projectile** (i.e.,
`weapon->Projectile = WeaponTypeClass+0xA0 = BulletTypeClass*`). NOT on the
target's type, and NOT on the weapon itself.

```
projectile = weapon.Projectile           // weapon+0xA0
if projectile.Arcing != 0:               // projectile+0x29B
    → Branch B (2D + slope arc check)
else:
    if attacker.WhatAmI() == 3:
        → Branch A1 (2D, dead in YR)
    else:
        → Branch A2 (3D Euclidean — DEFAULT)
```

### Branch A2 — 3D Euclidean (DEFAULT)

Covers virtually every direct-fire weapon: machine guns, lasers, cannon, missiles
fired horizontally. 3D distance compared inclusive (`<=`) against effective range.

### Branch B — 2D + ballistic arc (Arcing weapons)

For projectiles with `Arcing=yes` (V3, Tank Howitzer, Prism Tank lobs, MIRV missiles):

1. Compute 2D distance `(int)sqrt(dx² + dy²)`.
2. Compare `dist <= effective_range` (inclusive).
3. If `Projectile.SubjectToElevation == yes` (`+0x297`), add height-fire bonus via `FUN_006F70E0` (see §6 item 9).
4. Run **slope-arc check** via helpers `FUN_0048AB90` (set tan² limit) and `FUN_0048ABC0` (verify `dz` within reachable cone).
5. Slope parameter source:
   - If `Projectile.Floater == yes` (`+0x295`): `FUN_0048ACF0()` → `Rules.Gravity × _DAT_007e1738`. **TS-legacy** — no standard YR projectile sets `Floater=yes`.
   - Else: `Rules.Gravity` (`Rules+0x16B8`, `[AudioVisual]/Gravity=`).
6. **Bridge tolerance:** if attacker's cell has the bridge flag (`cell+0x140 bit 0x100`) and `(target.Z - src.Z) >= DAT_00B0EB34 × 3`, reject.

This is the **AA-arc / ballistic-reach** check: the target must be horizontally close
enough AND vertically within the projectile's reachable cone (computed from `Rules.Gravity`).

### Branch A1 — DEAD in YR

`attacker.WhatAmI() == 3` would gate this 2D-without-dz branch. Verified 2026-05-07 by
vtable scan + RTTI: only `*TypeClass` template classes (AircraftTypeClass / AnimTypeClass /
OverlayTypeClass / TerrainTypeClass / VoxelAnimTypeClass) return 3 from `WhatAmI()`, and
those are never used as TechnoClass attackers. TechnoClass instances return:

| Class | WhatAmI |
|---|---:|
| UnitClass | 1 |
| AircraftClass | 2 |
| BuildingClass | 6 |
| InfantryClass | 0xF |

**No TechnoClass-derived instance returns 3.** Branch A1 is unreachable in standard YR play. Do not implement it.

### Confidence (branch decision)

- **Content: MEDIUM** for the `byte+0x29B` and `byte+0x297` reads as on Projectile vs attacker. The Ghidra C decompiler tracking is ambiguous here (shows `param_1+0x29b` while the disassembly verified by the existing canonical doc shows `[EDX+0x29B]` with EDX loaded from the saved projectile pointer at `[ESP+0x10]`). The existing doc's asm-level trace is more authoritative. **Flag for asm re-verification** before any implementation depends on this — see §10.
- **Identity: HIGH** for Arcing/SubjectToElevation/Floater being on BulletTypeClass at those offsets (cross-verified against [`../../BULLETTYPECLASS_GHIDRA_REPORT.md`](../../BULLETTYPECLASS_GHIDRA_REPORT.md) BulletTypeClass::ReadINI).
- **Binding: HIGH** for Arcing semantics (every Arcing=yes weapon in retail goes through Branch B).

---

## 5. Source / target coords

### 5.1 Source (attacker side)

Caller-provided. InRange reads:
- `*src` = X
- `*(src+4)` = Y
- `*(src+8)` = Z

All **lepton-space int32**. The caller (typically `CanFireAt`) builds `src` from
`attacker->Get_Coords()`, with optional cell-snap adjustments.

### 5.2 Target (always via virtual)

```
target->Get_Coords(out_buf)   // vtable+0x48, slot 18
// reads:
out.X = *(int*)(target + 0x9C)
out.Y = *(int*)(target + 0xA0)
out.Z = *(int*)(target + 0xA4)
```

For non-overriding subclasses (Unit / Aircraft / Infantry), dispatches to
`ObjectClass::GetCoords` at `0x005F65A0`. **NOTE:** `FootClass::GetDestinationCoords`
is at vtable slot 19 (`0x4C`), **NOT** slot 18 — Get_Coords always returns position,
not destination.

**Critical fact:** the Z field at `+0xA4` is **already the absolute Z in leptons**. It
is NOT a level count, NOT level × LevelHeight. The movement/locomotor code maintains it
as `cell_level × LevelHeight (104) + bridge_offset_if_any + altitude_if_airborne`.
There is no separate altitude field that InRange reads.

### 5.3 Low-flying target Z override (verified)

After reading target.Z, InRange checks `target->IsLowFlying()` (vtable+0x50). If true:

```
target_Z = CellClass::GetGroundHeight(...)    // ground beneath target's cell
if cell.flags & 0x100:                         // cell is on a bridge
    target_Z += DAT_00B0EB24                   // bridge-height delta
```

**Low-flying targets are ranged to the GROUND BENEATH them**, not their actual
altitude. So a Harrier descending to attack does not escape fire by being slightly
elevated. This is a parity-critical detail.

### `IsLowFlying` / `IsHighFlying` (verified at `0x005F6B60` / `0x005F6B90`)

```
bool IsLowFlying()  { return this->byte@0x74 != 0 && Get_Height() <  DAT_00AC13C8 * 2; }
bool IsHighFlying() { return this->byte@0x74 != 0 && Get_Height() >= DAT_00AC13C8 * 2; }
```

- `byte@0x74` = "is currently airborne" flag (zero for ground, nonzero for airborne).
- `vtable+0x1C8` = `Get_Height()` returns altitude in leptons.
- `DAT_00AC13C8` = `HighFlightLevel` (BSS, runtime-init). Probably driven by `Rules.FlightLevel`.
- Mutually exclusive at `HighFlightLevel × 2`.

### Confidence (coords)

- **Content: HIGH** for the offset reads (ObjectClass::GetCoords decompiled).
- **Identity: HIGH** for the +0x9C/+0xA0/+0xA4 layout (matches existing ObjectClass struct docs).
- **Binding: HIGH** for the `IsLowFlying` Z-override (verified at `0x006F7332-0x006F7373` in disasm).

---

## 6. Effective-range bonus chain

Computed as `iVar8` (base + replacements) then `iVar3` (final, includes height/foundation bonuses):

| # | Bonus | Trigger | Source | Operation |
|---|---|---|---|---|
| 1 | Base | always | `weapon.Range` (`+0xB4`, leptons) | `range = weapon.Range` |
| 2 | Sentinel | `range == -0x200` | weapon.Range | **return true immediately** |
| 3 | AirRange | `target.IsHighFlying()` (`target->vtable+0x54`) | `attacker.Type.AirRange` (`+0x68C`) | `range += AirRange` |
| 4 | Garrison (REPLACES) | `attacker.IsOccupied()` (`attacker->vtable+0x400`) | `(occupant_count + Rules.OccupyWeaponRange) × 256` | `range = (count + Rules+0xF48) << 8` |
| 5 | Bunker | `attacker.byte@0x2E4 != 0 && attacker.WhatAmI() != 6` | `Rules.BunkerWeaponRangeBonus × 256` | `range += Rules+0xF54 × 256` |
| 6 | OpenTopped | `attacker.byte@0x82 != 0` | `Rules.OpenToppedRangeBonus × 256` | `range += Rules+0xF5C × 256` |
| 7 | Height-fire (Branch A, ballistic helper) | `Projectile.byte+0x297 != 0` (`SubjectToElevation=yes`) | `FUN_006F6F60(attacker, target)` | `final = range + bonus` |
| 8 | Foundation | Branch A only, `target.WhatAmI() == 6` (Building) | `(BuildH + BuildW) × 0x40` | `final += (h+w) × 64` leptons |
| 9 | Height-fire (Branch B variant) | Branch B + `Projectile.byte+0x297 != 0` | `FUN_006F70E0(attacker, target)` | similar to (7), returns `delta << 8` only (no ballistic term) |

**Composition rules:**
- (3, 5, 6) ADD to the running base.
- (4) **REPLACES** — Garrison overrides the running base entirely.
- (7, 8, 9) add to the final after the base is settled.

### Garrison REPLACES, not adds (important)

If a unit is in a garrison structure, its base range is *replaced* by
`(occupant_count + Rules.OccupyWeaponRange) × 256`. AirRange/Bunker/OpenTopped then add
to that. For a Conscript inside a Civilian-Garrison building with 5 occupants and
`Rules.OccupyWeaponRange=2`, base range = `(5+2)×256 = 1792 leptons = 7 cells` —
*regardless* of the Conscript's own weapon Range.

### Foundation bonus formula

`(FoundationHeight + FoundationWidth) × 64` leptons. Examples:
- 2x2 building: `(2+2)×64 = 256 leptons = 1 cell`.
- 4x2 ConYard: `(4+2)×64 = 384 leptons = 1.5 cells`.

Only applied to MAX range, not min. Only applied when target is a Building
(`target.WhatAmI() == 6`).

### Height-fire (high-ground advantage)

`FUN_006F6F60` and `FUN_006F70E0` both compute:

```
if !attacker.IsLowFlying() || !target.IsLowFlying(): return 0
attacker_h = CellClass::GetEffectiveHeight(attacker.cell)
target_h   = CellClass::GetEffectiveHeight(target.cell)
delta = max(0, target_h - attacker_h)
delta_cells = delta / Rules.ElevationIncrement   // Rules+0x1838
```

- `FUN_006F70E0` (Branch B): returns `delta_cells << 8` (cells → leptons).
- `FUN_006F6F60` (Branch A): returns the sqrt of `(delta_cells × 256)² + (DAT_00B0EB34 × delta)²` — a ballistic-style term combining horizontal and vertical.

The `max(0, …)` means bonus only kicks in when the **target** is on higher ground.
Practical effect: a weapon with `SubjectToElevation=yes` shoots farther uphill (more
intuitive: more range to reach an uphill target). Active in standard YR — the
high-ground advantage is a visible/audible feature.

### Rules-class constants (verified)

| Offset | INI key | Section | Type |
|---|---|---|---|
| `+0xF48` | `OccupyWeaponRange=` | `[CombatDamage]` (probable) | int (cells) |
| `+0xF54` | `BunkerWeaponRangeBonus=` | `[General]`/`[CombatDamage]` | int (cells) |
| `+0xF5C` | `OpenToppedRangeBonus=` | `[General]`/`[CombatDamage]` | int (cells) |
| `+0x16B8` | `Gravity=` | `[AudioVisual]` (parsed at `0x66B3D9`) | int |
| `+0x1838` | `ElevationIncrement=` | `[ElevationModel]` (parsed at `FUN_0066D150`) | int |
| `+0x1840` | `ElevationIncrementBonus=` | `[ElevationModel]` | double |
| `+0x1848` | `ElevationBonusCap=` | `[ElevationModel]` | double |

### Confidence (bonus chain)

- **Content: HIGH** (every bonus item decompiled and disassembled in the existing canonical doc; live re-decomp on 2026-05-17 matches every offset).
- **Identity: HIGH** (Rules offsets cross-referenced to ReadINI calls — `Gravity=` write at `0x0066B3D9` confirms `Rules+0x16B8`).
- **Binding: HIGH** (every bonus has a confirmed trigger and runtime call site; height-fire helpers FUN_006F6F60/FUN_006F70E0 were decompiled and verified).

---

## 7. Bridge LOS gate

After Branch A's distance compare passes, InRange runs **one more reject check** that
blocks shots from below a bridge to above it:

```
src_cell = MapClass::Get_Cell_At(src)
if (src_cell.flags & 0x100):                       // attacker is in a bridge cell
    bridge_top = CellClass::GetGroundHeight(src) + DAT_00B0EB24   // ground + bridge_height
    if (src.Z < bridge_top) && (target.Z >= bridge_top):
        return false                                // shot blocked by bridge
```

**Plain meaning:** a unit on the ground *beneath* a bridge cannot fire upward
*through* the bridge to a target on the deck. Real LOS occlusion, not anti-self-fire
and not TS-legacy. The reverse case (attacker on bridge, target on ground below)
isn't checked here — no occlusion exists in that geometry.

Active every match with bridge geometry.

### Confidence

- **Content: HIGH** (disassembled at `0x006F75FB-0x006F762F` 2026-05-07; intent matches the literal compare).
- **Identity: HIGH** (single conditional gate, single unconditional rejection path).
- **Binding: HIGH** (runs on every Branch A path after distance compare succeeds; bridge cells are common in retail maps).

---

## 8. Key offsets summary

| Symbol | Offset / Address | Class |
|---|---|---|
| `weapon.Range` | `+0xB4` | WeaponTypeClass |
| `weapon.MinimumRange` | `+0xB8` | WeaponTypeClass |
| `weapon.Projectile` | `+0xA0` | WeaponTypeClass (BulletTypeClass*) |
| `weapon.Warhead` | `+0xAC` | WeaponTypeClass (WarheadTypeClass*) — NOT used by InRange |
| `weapon.Range == -0x200` | sentinel | "always in range" |
| `attacker.Type.AirRange` | `+0x68C` | TechnoTypeClass |
| `attacker.byte_isInBunker` | `+0x2E4` (= `param_1[0xB9]`) | TechnoClass |
| `attacker.byte_isOpenTopped` | `+0x82` | TechnoClass |
| `Projectile.Arcing` | `+0x29B` | BulletTypeClass |
| `Projectile.SubjectToElevation` | `+0x297` | BulletTypeClass |
| `Projectile.Floater` | `+0x295` | BulletTypeClass (TS-legacy) |
| `ObjectClass.Coords` | `+0x9C / +0xA0 / +0xA4` | ObjectClass — `(X, Y, Z)` int leptons |
| `byte_isAirborne` | `+0x74` | ObjectClass |
| `Rules.OccupyWeaponRange` | `Rules+0xF48` | int cells |
| `Rules.BunkerWeaponRangeBonus` | `Rules+0xF54` | int cells |
| `Rules.OpenToppedRangeBonus` | `Rules+0xF5C` | int cells |
| `Rules.Gravity` | `Rules+0x16B8` | int |
| `Rules.ElevationIncrement` | `Rules+0x1838` | int |
| `Sqrt_Approx` | `0x004CAC40` | mantissa-LUT sqrt approximation |
| `Math::ftol` | `0x007C5F00` | x87 float→int truncate |
| `DAT_00AC13C8` | runtime-init | `HighFlightLevel` |
| `DAT_00B0EB24` | runtime-init | bridge-height delta (leptons) |
| `DAT_00B0EB34` | runtime-init | ballistic-elevation scalar |

The three `DAT_00…` constants read as zero from the binary (BSS), so they're populated
at runtime. Exact numeric values would require a debugger session — flag for follow-up.
Defaults from RA2/YR convention: BridgeHeight = 4 levels × LevelHeight 104 = 416 leptons.

---

## 9. TS-legacy filter

- **Branch A1 (`WhatAmI()==3`):** Confirmed TS-legacy / unreachable in YR. Vtable scan + RTTI identifies all four functions returning 3 as `*TypeClass` template classes (AircraftType / AnimType / OverlayType / TerrainType / VoxelAnimType). No TechnoClass instance reaches this branch. **Do not implement.**
- **`Projectile.Floater=yes` (Branch B per-projectile gravity override):** TS-legacy. No standard YR projectile sets it. The standard Branch B path uses `Rules.Gravity` directly. **Do not implement Floater handling.**

All other branches (3D Euclidean, 2D ballistic, garrison/bunker/open-topped chain,
foundation bonus, height-fire, bridge LOS) are live in vanilla YR play.

---

## 10. Open follow-ups

1. **`Projectile.Arcing` / `SubjectToElevation` / `Floater` byte reads — asm re-verification.** Live Ghidra decompilation (2026-05-17) shows the C decompiler tracking `param_1+0x29B` while the existing canonical doc verified these reads are from `[EDX+0x29B]` where EDX is the saved projectile pointer. The asm-level evidence is more authoritative, but a fresh disassembly walk would resolve all ambiguity. Priority: MEDIUM (the alternative interpretation would put these flags on TechnoClass at offsets that don't appear used elsewhere — strong indirect evidence the Projectile interpretation is correct).
2. **Exact runtime values of `DAT_00AC13C8` / `DAT_00B0EB24` / `DAT_00B0EB34`.** Semantic meaning known; numeric values would require runtime inspection. Priority: LOW (parity tests will surface drift if present).
3. **`Rules.OccupyWeaponRange` INI section.** Listed in this doc as `[CombatDamage]` (probable). Verify by tracing `Rules+0xF48` writer. Priority: LOW.
4. **`FootClass::IsHighFlying` actually overrides?** Vtable slot decompiled the same body as `ObjectClass::IsHighFlying`. Cosmetic — likely an inherited method that Ghidra duplicated. Priority: VERY LOW.

---

## 11. Sources

- Live decompilation of `TechnoClass__InRange` at `0x006F7220` (2026-05-17).
- Existing canonical doc: [`../../TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`](../../TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) — with its 2026-05-07 corrections section (`§0`) where the Projectile byte-flag identities, the WhatAmI()==3 dead-branch verification, the height-fire renaming, and the bridge LOS reading were resolved. This systems doc supersedes it for range-check spec; the existing doc retains historic value for the asm-level disassembly trace.
- WeaponTypeClass struct: [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
- BulletTypeClass struct: [`../../BULLETTYPECLASS_GHIDRA_REPORT.md`](../../BULLETTYPECLASS_GHIDRA_REPORT.md) — for the `+0x294..+0x29F` byte-flag offsets (Airburst / Floater / SubjectToCliffs / SubjectToElevation / Arcing).
- Bunker / garrison range chain: [`../../BUNKER_SYSTEM_GHIDRA_REPORT.md`](../../BUNKER_SYSTEM_GHIDRA_REPORT.md) §5.
- Coordinate system: [`../../COORDINATE_SYSTEM_GAMEMD.md`](../../COORDINATE_SYSTEM_GAMEMD.md) — LevelHeight = 104 leptons confirmed.
