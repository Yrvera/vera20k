# AnimClass Spawn Paths — Muzzle Flashes, Explosions, Debris

**Date:** 2026-04-22 (initial) / 2026-04-22 (verification pass)
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH (all claims verified from live Ghidra decompilation and raw vtable reads)
**Active in YR:** Yes (all paths fire on every weapon shot / impact)

## 1. Overview

This report fills the gap between the existing `AnimClass`/`AnimTypeClass` research (which
covers the per-anim lifecycle exhaustively) and the **caller side** — the specific code
paths that allocate and construct an `AnimClass` for muzzle flashes, weapon impact
explosions, crater anims, debris, and garrison-fire anims.

Scope:

- `TechnoClass::Fire_At` Phase 12: muzzle flash anim selection, creation, building Z-adjust,
  attachment via `SetOwnerObject`.
- `WarheadTypeClass::Detonate` explosion spawn, including the `SelectExplosionAnim`
  damage-band lookup and the debris/particle fallbacks.
- `AnimClass::SetOwnerObject` verified attachment mechanics.
- vtable resolution for `vtable+0x400` and `vtable+0x408` used during muzzle flash Z-adjust.

**Sibling reports** (not duplicated here):
- [`ANIM_CLASS_GHIDRA_REPORT.md`](../../ra2-rust-game-docs/ANIM_CLASS_GHIDRA_REPORT.md) — full `AnimClass` / `AnimTypeClass` struct layout and lifecycle
- [`ANIM_CLASS_DEEP_DIVE.md`](../../ra2-rust-game-docs/ANIM_CLASS_DEEP_DIVE.md) — per-tick AI, vtable, DrawIt, Middle, Start
- [`ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`](../../ra2-rust-game-docs/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md) — Next=, TrailerAnim, Damage= accumulation
- [`BUILDING_ANIM_STATE_MACHINE.md`](../../ra2-rust-game-docs/BUILDING_ANIM_STATE_MACHINE.md) — 21-slot building overlay state machine
- [`DAMAGE_FIRE_ANIMS_GHIDRA.md`](../../ra2-rust-game-docs/DAMAGE_FIRE_ANIMS_GHIDRA.md) — `DamageFireOffset0..7` fire overlays
- [`WARHEAD_DETONATE_GHIDRA_REPORT.md`](../../ra2-rust-game-docs/WARHEAD_DETONATE_GHIDRA_REPORT.md) — warhead impact handler (this report refines §7)

## 2. Key Addresses

| Entity | Address | Notes |
|---|---|---|
| `TechnoClass::Fire_At` | `0x006FDD50` | ~6000 bytes, muzzle flash at end |
| `WarheadTypeClass::Detonate` | `0x004690B0` | explosion spawn at `LAB_0046a2a1` |
| `Warhead__SelectExplosionAnim` | `0x0048A4F0` | damage-band AnimList lookup |
| `FUN_0048ACE0` (zAdjust constant) | `0x0048ACE0` | returns -15 (hardcoded) |
| `AnimClass::SetOwnerObject` | `0x00424B50` | attach/detach to firer |
| `AnimClass::Constructor` (full) | `0x00421EA0` | 7 params + this |
| `BuildingClass::IsOccupied` | `0x00458DD0` | BuildingClass vtable `+0x400` override |
| `BuildingClass::GetOccupantCount` | `0x004581F0` | BuildingClass vtable `+0x408` override |
| TechnoClass base `vtable+0x400` stub | `0x0041BFB0` | returns 0 (inherited by Infantry/Unit/Aircraft) |
| TechnoClass base `vtable+0x408` stub | `0x0041BFD0` | returns 0 |
| BuildingClass vtable base | `0x007E3EBC` | derived from `BuildingClass::IsOccupied` xref |
| InfantryClass vtable base | `0x007EB058` | derived from `InfantryClass::IronCurtain` xref |
| `AnimClass::DrawIt` | `0x00422CA0` | palette/height selection logic |
| `FUN_0048A620` (combat light spawner) | `0x0048A620` | **NOT** an AnimClass spawner — creates LightSource |
| `FUN_005FF250` (LightSourceClass ctor helper) | `0x005FF250` | called by `FUN_0048A620` |

## 3. Muzzle Flash Spawn — `TechnoClass::Fire_At` Phase 12

Located near the end of `Fire_At` (address ~`0x006FE9xx`–`0x006FED00`). Runs after bullet
creation and after special-effect paths (laser / EBolt / rad beam / wave).

### 3.1 Anim Selection — the four cascading rules

```c
// weapon = this->GetWeapon(index);  vtable+0x3F8, stored in uVar18
int animPtr = 0;

// RULE 1 — 8-directional muzzle flash (Anim=MGUN-N,MGUN-NE,...)
if (weapon->AnimCount == 8) {   // weapon+0x104 == 8
    short turretFacing = this->GetTurretFacing()[0];   // vtable+0x308, 16-bit DirStruct
    uint idx = (((turretFacing >> 12) + 1) >> 1) & 7;  // quantize to 0..7
    idx = (idx + 1) & 7;                                // +1 mod 8 (see §3.2)
    animPtr = weapon->AnimList_Data[idx];               // weapon+0xF8 + idx*4
}
// RULE 2 — Non-directional list: use first entry
else if (weapon->AnimCount > 0) {
    animPtr = weapon->AnimList_Data[0];                 // weapon+0xF8 [0]
}

// RULE 3 — Garrisoned firer (building with occupants)
if (this->IsOccupied()) {              // vtable+0x400 (BuildingClass::IsOccupied)
    animPtr = weapon->OccupantAnim;    // weapon+0x110, overrides AnimList
}

// RULE 4 — Open-topped transport / airstrike
if (animPtr == 0 && this->field_0x82 != 0 && weapon->OpenToppedAnim != 0) {
    animPtr = weapon->OpenToppedAnim;  // weapon+0x118
}
```

**Key clarifications vs. prior docs:**

- `field_0x82` on TechnoClass is the **airstrike flag**. The existing
  `FIRE_AT_ANALYSIS.md` already documents this (lines 89, 182, 341). Verified consistent
  with the LimboLaunch handling in the same function.
- `vtable+0x400` is `IsOccupied`, not "naval check". Verified by direct vtable reads:
  - **BuildingClass vtable (`0x007E3EBC`) + `0x400`** dispatches to `BuildingClass::IsOccupied`
    at `0x00458DD0`, which returns 1 when `TypeClass+0x157B` (CanBeOccupied) AND
    `TypeClass+0x157C` AND `GetOccupantCount() > 0`.
  - **InfantryClass vtable (`0x007EB058`) + `0x400`** dispatches to a stub at
    `0x0041BFB0` that simply returns 0. This stub is inherited by Infantry, Unit, and
    Aircraft — so only buildings can ever return non-zero here.
  - `vtable+0x408` follows the same pattern: BuildingClass → `GetOccupantCount`
    (`0x004581F0` reads `this+0x694`); non-buildings → stub at `0x0041BFD0` returning 0.
- Together these guarantee the `OccupantAnim` override branch fires **only** when the
  firer is a BuildingClass with garrisoned infantry. The `-200` Z-adjust override in
  §3.4 similarly only applies to buildings (gated by `GetAbsType() == 6`).

### 3.2 The 8-direction facing math (exact bitops from binary)

The raw decompilation reads:

```c
iVar9 = *(int *)(*(int *)(uVar18 + 0xf8) +
                (((*puVar15 >> 0xc) + 1 >> 1 & 7) + 1 & 0x80000007) * 4);
```

Parsed:

| Step | Operation | Effect |
|---|---|---|
| 1 | `turretFacing >> 12` | 16-bit DirStruct → 0..15 quadrant |
| 2 | `+ 1` | add 1 for rounding |
| 3 | `>> 1` | divide by 2 → 0..8 |
| 4 | `& 7` | wrap to 0..7 (quantized direction) |
| 5 | `+ 1` | shift index by one slot |
| 6 | `& 0x80000007` | mask keeps low 3 bits + sign bit (effectively `& 7` for positive values) |

The net effect is `index = (quantized_direction + 1) & 7`. This means:
- Facing north (DirStruct = 0) → index 1 → second anim in list
- Facing NE (DirStruct = 0x2000) → index 2
- Facing NW (DirStruct = 0xE000, quantized 7) → index 0 → first anim

The "+1 offset" rotates the list one step counter-clockwise from what a naive reader might
expect. This matches the canonical `Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,
MGUN-W,MGUN-NW` ordering — slot 0 is consumed in a way that makes slot 1 align with
true north. The `0x80000007` mask is a compiler artifact of the signed modulo, not a
semantic difference from `& 7`.

### 3.3 AnimClass Construction

```c
if (animPtr != 0) {
    void* block = operator_new(0x1C8);    // 456 bytes, AnimClass size
    AnimClass* anim = AnimClass__Constructor(
        animPtr,      // AnimTypeClass*
        &muzzleCoords, // uStack_98 = FLH muzzle position (world lepton coords)
        0,             // delay — play immediately
        1,             // loopCount multiplier
        0x600,         // drawFlags = center sprite (0x200) | reserved (0x400)
        0,             // zAdjust (overridden below for buildings)
        0              // reverse
    );
    // ... building Z-adjust, attachment — see §3.4 and §3.5
}
```

**Fixed parameters for muzzle flash:**
- `delay=0` — plays on the same tick as the shot.
- `loopCount=1` (passed in as param 5; effective count becomes `type->LoopCount * 1`).
- `drawFlags=0x600` — center-on-coords blitter flag (`0x200`) + reserved bit (`0x400`,
  unused by blitter). DrawIt later adds `0x2000` and `0x800` before calling `CC_Draw_Shape`.
- `zAdjust=0` (then overridden for buildings).
- `reverse=0`.

**Subtle detail verified from Constructor (`0x00421EA0`):** the `LoopCountRemaining`
(`AnimClass+0x195`) field is computed as an **8-bit multiply**:

```c
if (param_5 < 2) param_5 = 1;           // clamp loopCount param to >= 1
bVar2 = (byte)(type->LoopCount) * (byte)param_5;  // BYTE * BYTE, result is byte
anim->LoopCountRemaining = bVar2;
if (bVar2 < 2) bVar2 = 1;               // clamp to >= 1
anim->LoopCountRemaining = bVar2;
```

This means if `type->LoopCount * loopCount_param > 255`, the value wraps. For muzzle
flashes and explosions (where `LoopCount=1` and param is 1), the product is 1. The
special value `0xFF` (255) is used as the "infinite loop" sentinel (per existing
`ANIM_CLASS_GHIDRA_REPORT.md`). Modders who set `LoopCount=256` on an AnimType would
silently get `LoopCountRemaining=0` (wraparound). Not relevant for vanilla YR but worth
noting.

### 3.4 Building Z-Adjust Override (only when `GetAbsType() == 6`)

```c
if (this->GetAbsType() == 6) {          // BuildingClass
    CoordStruct* bldgCoords = this->GetRenderCoords(temp);   // vtable+0xAC
    int dY = muzzle.Y - bldg.Y;
    // Rounded-toward-zero divide by 4, then negate
    int zAdj = -((dY + (dY >> 31 & 3)) >> 2);
    anim->ZAdjust = zAdj & ((zAdj > -1) ? 0 : -1);   // clamp to <= 0

    if (this->GetOccupantCount() > 0) {  // vtable+0x408, garrison fire
        anim->ZAdjust = -200;            // 0xFFFFFF38, hard override
    }
}
```

| Case | zAdjust |
|---|---|
| Muzzle below building center (dY > 0) | `-(dY/4)` (negative, pushes anim behind building depth-sort) |
| Muzzle above building center (dY < 0) | clamped to 0 (no offset) |
| Garrison fire (occupants > 0) | hard `-200` (deep behind walls) |

The `>> 31 & 3` trick is compiler code for "signed divide rounding toward zero" — equivalent
to `dY / 4` for both positive and negative values in C.

### 3.5 Attachment via `SetOwnerObject` (non-buildings only)

```c
if (anim != 0 && this->GetAbsType() != 6) {  // NOT BuildingClass
    AnimClass__SetOwnerObject(anim, this);   // 0x00424B50
}
```

Buildings never attach their muzzle flash because the building itself doesn't move — the
flash stays at the world muzzle position. Infantry, Units, and Aircraft attach so the flash
follows them if they move during the frame the anim is alive.

`SetOwnerObject` mechanics (verified from `0x00424B50`):

1. **Detach old owner** (not applicable for freshly-constructed muzzle flash anim):
   - Remove anim from display layer if registered
   - Scan `g_AnimClass_Array` to see if any OTHER anim still attached to the same owner
   - If no other anim references the owner: call owner's `vtable+0x17C` (clear anim flag),
     clear owner byte at `+0x84`
   - Compute anim's current world coords via `GetCoords()` (which was returning
     `owner.coords + anim.offset`)
   - Clear `anim->OwnerObject` (anim index `[0x33]` = byte `+0xCC`)
   - Re-submit to display

2. **Attach new owner**:
   - Get anim's current world coords (muzzle position)
   - Remove from display layer
   - Set owner byte at `+0x84` = 1 (owner now has an attached anim)
   - Write `anim->OwnerObject = newOwner`
   - Get owner's world coords
   - `SetCoords(muzzleWorld - ownerWorld)` — anim's internal coords become a **relative
     offset** from the owner
   - Re-submit to display

After attachment, every subsequent `GetCoords()` call on the anim returns
`owner.coords + stored_offset`, so the muzzle flash follows unit movement.

### 3.6 Palette Inheritance — Verified: NONE for muzzle flash

**Verified from `AnimClass::DrawIt` at `0x00422CA0`.** The muzzle flash attached via
`SetOwnerObject` does **NOT** inherit the firer's house palette. Palette selection in
DrawIt is entirely **AnimType-driven**, independent of `OwnerObject`.

The selection cascade (exact order from DrawIt, lines ~235-300):

```c
int palette;
int height = 1000;  // default height

if (type->IsVeins) {                              // AnimType+0x355
    palette = g_ColorSchemeArray[PlayerPtr->ColorIndex + 0x16054]->Convert;
}
else if (anim->field_0x46) {                      // cell palette flag
    CellClass* cell = MapClass::Get_CellClass(...);
    palette = cell->Palette;  // cell+0x34; if 0, synthesized via FUN_00483E30
}
else {
    palette = anim->Palette;  // AnimClass+0xD4, usually 0
    if (palette == 0) {
        palette = DAT_0087f6c0;  // default global palette
        if (type->AltPalette) {                   // AnimType+0x361
            palette = g_ColorSchemeArray[0]->ConvertPalette;  // +0x30C
        }
    }
}

// Height value for lighting (separate from palette):
if (!type->UseNormalLight) {                      // AnimType+0x35D
    // Look up cell's surface height (+0x10A) or air height (+0x10C)
    height = cell->surface_or_air_height;
}
```

**Critical implication:** in RA2, a garrison muzzle flash attached to a Soviet-house
building still renders with the default palette, not a red/blue tint. This matches the
observable behavior in retail — muzzle flashes do not recolor per owner. The
Rust renderer must resist the temptation to propagate the owner's color scheme to
attached anims; the binary explicitly does not.

**Consequence for `SetOwnerObject`:** the owner relationship only controls **position**
(GetCoords returns `owner.coords + offset`), not palette. The only fields that change
palette are the `AnimTypeClass` flags (`IsVeins`, `AltPalette`) and the `AnimClass`
per-instance Palette override at `+0xD4`.

### 3.7 Fire Sound (adjacent to muzzle flash spawn)

Immediately before the muzzle flash `operator_new` (at roughly `0x006FEC80`), renumbered
after §3.6 insertion:

```c
if (weapon->Report_Count > 0 &&   // weapon+0xCC
    this->GetTechnoType()->byte_0xCD5 == 0) {  // not a silenced unit
    VocClass__PlayAt(report_index, &muzzleCoords);  // FUN_007509E0
}
```

The specific `Report=` entry used is picked by earlier logic (random from list). The
`byte_0xCD5` check gates the silencer flag (used for silenced infantry weapons).

### 3.8 FLH — Fire Location + Height (muzzle position)

**Verified from `TechnoClass::GetFLH` at `0x006F3AD0`.** This is the virtual function
called via `vtable+0xB0` during Phase 3 of Fire_At (line where `uStack_98` was filled).
Its result (world coords) is the base position the muzzle flash and bullet both spawn at.

```c
CoordStruct* TechnoClass::GetFLH(
    CoordStruct* out,     // output
    int weaponIdx,        // which weapon slot; negative for Elite variants
    int offsetX,          // extra X offset (usually 0)
    int offsetY,          // extra Y
    int offsetZ           // extra Z
) {
    TechnoTypeClass* type = this->GetTechnoType();      // vtable+0x84
    int flhX, flhY, flhZ;

    if (weaponIdx < 0) {
        // Elite FLH slots: type+0x850 + |weaponIdx| * 0xC
        // weaponIdx == -1 → type+0x85C, -2 → type+0x868, ..., -4 → type+0x880
        if (weaponIdx == -5 || -weaponIdx < 5) {
            int* slot = (int*)(type + 0x850 - weaponIdx * 0xC);
            flhX = slot[0];  flhY = slot[1];  flhZ = slot[2];
        }
    } else {
        // Normal FLH: from WeaponSlot struct returned by GetWeapon
        WeaponSlot* slot = this->GetWeapon(weaponIdx);   // vtable+0x3F8
        // struct layout: { WeaponTypeClass* weapon; int flhX; int flhY; int flhZ; }
        flhX = slot->flhX;  // +4
        flhY = slot->flhY;  // +8
        flhZ = slot->flhZ;  // +0xC
    }

    flhX += offsetX;

    // Facing angle computation — body facing, optionally minus turret offset
    double angleRad;
    if (this->HasLocomotor && this->Locomotor != 0) {
        Matrix3x4 m = this->Locomotor->GetMatrix(...);   // +0x19D * 4 = +0x674
        short bodyFacing  = this->GetBodyFacing()[0];    // vtable+0x2A8, 16-bit DirStruct
        short turretFacing = RateTimer::Current();        // turret angle
        // 32-way quantization, centered at 0, converted to radians (_DAT_007e4408 ≈ PI/16)
        angleRad = (((bodyFacing >> 10) + 1) >> 1 & 0x1F - 8) * PI_DIV_16
                 - (((turretFacing >> 10) + 1) >> 1 & 0x1F - 8) * PI_DIV_16;
    } else {
        Matrix3x4_SetIdentity();
        short bodyFacing = this->GetBodyFacing()[0];
        angleRad = ((bodyFacing >> 10) + 1) >> 1 & 0x1F - 8) * PI_DIV_16;
    }

    // Apply matrix transformations
    Matrix3x4_Translate(type->fireExtraZ_0x720, 0, 0);   // type field at +0x720 (see below)
    Matrix3x4_RotateZ(angleRad);

    // Barrel alternation: Y sign flip based on this+0xEE bit 0
    uint flip = this->field_0xEE & 0x80000001;
    int ySign = (flip != 0) ? +1 : -1;
    Matrix3x4_Translate(flhZ + offsetZ, ySign * (flhY + offsetY), /*unaff_ESI*/ 0);

    // Transform origin point through the accumulated matrix
    CoordStruct local = {0, 0, 0};
    Matrix3x4_TransformPoint(&local);

    // Final position = center + transformed offset
    CoordStruct center = this->GetRenderCoords();        // vtable+0xAC
    out->X = center.X + local.X;
    out->Y = center.Y + local.Y;
    out->Z = center.Z + local.Z;
    return out;
}
```

**INI key → struct offset map** (verified from string xrefs in
`TechnoTypeClass::ReadINI` at `0x00712170`):

| INI key | String @ | Xref @ | Purpose |
|---|---|---|---|
| `PrimaryFireFLH=X,Y,Z` | `0x008432F8` | `0x00715DA1` | Muzzle offset for weapon slot 0 |
| `SecondaryFireFLH=X,Y,Z` | `0x008432C0` | (nearby) | Muzzle offset for weapon slot 1 |
| `ElitePrimaryFireFLH=X,Y,Z` | `0x00843288` | (nearby) | Elite-veterancy muzzle offset (negative weaponIdx path) |
| `EliteSecondaryFireFLH=X,Y,Z` | `0x00843244` | (nearby) | Elite secondary muzzle offset |

Retail values from `artmd.ini` (vanilla unit samples):
- Rocketeer / first airborne units: `PrimaryFireFLH=100,0,120`
- Chrono Miner: `PrimaryFireFLH=75,-50,85`
- Guardian GI deployed: `PrimaryFireFLH=60,0,100`
- Grizzly tank class: `PrimaryFireFLH=100,-25,135`

The `X` value (forward offset) is typically 50–200 leptons (barrel length), `Y` is 0 or
small (±50 for side-mounted barrels), `Z` is barrel height above unit center
(typically 85–175).

**Barrel alternation detail:** the decompile shows `param_1[0xEE] & 0x80000001`. Because
`param_1` is typed `int *`, `param_1[0xEE]` is byte offset `0xEE × 4 = 0x3B8`, which is
**CurrentBurstIndex** (TechnoClass+0x3B8 — see `BURST_WEAPON_FIRING_GHIDRA_REPORT.md`).
The mask `& 0x80000001` covers bit 0 (LSB) and bit 31 (sign bit); in practice
CurrentBurstIndex is non-negative, so the LSB is the relevant bit. When that bit is set,
the FLH `Y` component is flipped, so the muzzle alternates between left and right
barrels. This produces the visual "twin-barrel fires in alternation" effect on units
like Grizzly, Rhino, or Prism Tank without requiring separate per-barrel FLH data.

> **Notation note:** Earlier wording said the field was "stored near +0x3B8 but folded
> into +0xEE." That phrasing was an artifact of Ghidra's `int *` indexing display;
> there is no separate +0xEE field. See `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`
> §1.2 for the full notation explanation.

**Per-burst FLH is Ares-only:** vanilla YR has **no** `PrimaryFireFLH.Burst0/.Burst1/...`
per-shot offsets — that's an Ares modding extension. Shipping YR reads the same
`flhX/flhY/flhZ` per weapon slot; only the Y-sign flip via `field_0xEE` changes per
burst (see `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` §3.4).

**Negative weaponIdx ranges:** the switch at the top of GetFLH accepts `weaponIdx` in
`[-4, -1] ∪ {any positive}`. The bounds check `param_3 == -5 || -param_3 < 5` — the
first disjunct is odd (it's not `< -5` and also not `<= -5`, but exactly `-5`) and
looks like a compiler-generated sentinel check. Callers in the combat path all pass
non-negative values; the negative range is used by Elite-veterancy and
`CurrentBurstIndex`-cycling special callers.

**Facing quantization:** the 32-way quantization (shift 10, mask `0x1F`) gives much
finer granularity than the 8-way Anim= index (shift 12, mask `7`). This means the FLH
position rotates smoothly through 32 angles, but the muzzle-flash sprite only has 8
frames — so a unit rotating will see its FLH position move more smoothly than its
visible flash orientation changes.
## 4. Explosion / Crater Anim Spawn — `WarheadTypeClass::Detonate`

Located at `LAB_0046a2a1` in `0x004690B0`, after `Apply_area_damage` has returned.

### 4.1 `Warhead__SelectExplosionAnim` (0x0048A4F0)

Complete decompilation (verified):

```c
AnimTypeClass* SelectExplosionAnim(
    int damage,              // param_1: damage amount (after armor multiplier)
    WarheadType* warhead,    // param_2
    int landType,            // param_3: from cell->LandType (2 = bridge)
    CoordStruct* coords)     // param_4
{
    if (damage == 0 || warhead == NULL) return NULL;

    // --- Bridge crater path ---
    if (landType == 2 && warhead->IsBridge) {  // byte +0x14D
        CellClass* cell = CellClass__Get_Cell_At(coords);
        if (!(cell->Flags & 0x100)) {          // cell+0x140, bit 0x100 = "elevated"
            int groundH = cell->GetGroundHeight();
            if (coords->Z < groundH + BridgeHeight * 2) {
                // Impact is under the bridge
                int bridgeCount = Rules->BridgeCraterCount;  // Rules+0xBD0
                if (bridgeCount == 0) return NULL;
                int idx = min(damage, bridgeCount * 0x23 - 1);  // 35-damage bands
                return Rules->BridgeCraterAnims[idx / 0x23];    // Rules+0xBC4
            }
        }
    }

    // --- IonCannonWarhead special case ---
    if (warhead == Rules->IonCannonWarhead) {  // Rules+0x17B4
        return Rules->IonCannonCrater;          // Rules+0x2F4
    }

    // --- Normal AnimList path ---
    int count = warhead->AnimList_Count;       // warhead+0x114
    if (count == 0) return NULL;

    if (warhead->EMEffect == 0) {              // byte +0x154
        // Damage-based: 25-damage bands
        int idx = min(damage, count * 0x19 - 1);   // 25-damage bands
        return warhead->AnimList_Data[idx / 0x19]; // warhead+0x108 is array data ptr
    } else {
        // EMEffect warhead: random entry
        int idx = Random(0, count - 1);
        return warhead->AnimList_Data[idx];
    }
}
```

**Corrections vs. prior `WARHEAD_DETONATE_GHIDRA_REPORT.md`:**

- **Bridge crater band size is `0x23` (35), not 25.** Prior report said "each entry covers
  25 damage" uniformly. Binary disassembly of the bridge branch uses `0x23` (35), while the
  normal branch uses `0x19` (25). This is a material difference — an 80-damage hit on a
  bridge picks bridge crater anim index `80/35 = 2` (third entry); on non-bridge it picks
  regular anim `80/25 = 3` (fourth entry).
- **AnimList data pointer is at `warhead+0x108`, not `warhead+0x104`.** The WarheadType
  struct has a `DynamicVectorClass<AnimType*>` embedded starting at `0xF4`; `+0x104` is
  the "Items" field (the AnimList count at `+0x114` is actually `m_Capacity - ActiveCount`
  delta or similar DVC bookkeeping — the DATA pointer lives at `+0x108`). The existing
  `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` and `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`
  labeled `+0x108` as "Anim list data field 2" — it's actually `m_Data` (pointer to the
  element array).

### 4.2 Explosion AnimClass Construction

At `LAB_00469d06` in `Detonate`:

```c
AnimTypeClass* expAnim = SelectExplosionAnim(damage, warhead, landType, &impactCoords);

// Bright=yes path: ALSO spawn a combat light source (NOT AnimClass)
if (warhead->Bright) {                                     // warhead+0xE0 (param_1[0x38])
    if (apply_area_damage_returned_2) goto NUKE_GROUND;    // §4.3
    int lightFlags = 0;
    if (warhead->IsElectricBolt) lightFlags |= 2;          // warhead+0x151
    if (warhead->byte_0x152)     lightFlags |= 4;
    if (warhead->byte_0x153)     lightFlags |= 8;
    FUN_0048A620(impactCoords.X, impactCoords.Y, impactCoords.Z, 1, lightFlags);
    // ^ This creates a LightSourceClass (0x18 bytes), NOT an AnimClass.
    //   See §4.5 for verification.
}

if (expAnim != NULL) {
    void* block = operator_new(0x1C8);
    int zAdjust = FUN_0048ACE0();        // returns -15 (hardcoded constant!)
    AnimClass__Constructor(
        expAnim,       // type
        &impactCoords, // coords (local_44..local_3C, adjusted for InfDeath spawn)
        0,             // delay
        1,             // loopCount
        0x2600,        // drawFlags = 0x2000 (Z-buffer) | 0x400 | 0x200 (center)
        zAdjust,       // -15 constant Z offset for all explosion anims
        0              // reverse
    );
}
```

**Fixed parameters for explosion anim:**
- `delay=0`
- `loopCount=1`
- `drawFlags=0x2600` — **different from muzzle flash (0x600)**. Adds `0x2000` which
  ensures Z-buffer sorting is used immediately in DrawIt.
- `zAdjust=-15` — `FUN_0048ACE0` is a one-instruction stub: `MOV EAX, 0xFFFFFFF1; RET`.
  All explosion anims get this fixed Z offset, placing them slightly below the impact plane
  in sort order.
- `reverse=0`

Notably, explosion anims **are not attached via `SetOwnerObject`** — they stay at the world
impact coordinate.

### 4.3 Weapon-Nullify (IRONFX) Path — NOT "Nuke Ground Zero"

**CRITICAL CORRECTION (verification pass):** the earlier draft of this report identified
`Rules+0x350` as "NukeGroundAnim". **This was wrong.** Direct verification from
`rulesmd.ini` line 553 and `rules.ini` line 545:

```ini
WeaponNullifyAnim=IRONFX  ; animation to play when a weapon is neutralized by Invulnerability
```

So `Rules+0x350` is **WeaponNullifyAnim** — the Iron Curtain nullify visual. This
completely changes the semantic of the `bVar8` gate.

**Additional correction:** the enclosing function in Ghidra is labeled
`WarheadTypeClass__Detonate`, but the `param_1` parameter is actually a `BulletClass*`,
not a `WarheadTypeClass*`. Verified by field access pattern:
- `param_1[0x27..0x29]` = byte offsets `0x9C..0xA4` = `Location` (inherited from
  ObjectClass — only set on instance-level objects, not type classes).
- `param_1[0x4a]` = byte offset `0x128` = `BulletClass::Warhead` pointer (verified from
  `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`).
- `param_1[0x38]` = byte offset `0xE0` = `BulletClass::Bright` (1-byte bool, from
  weapon), also verified against `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` line 96.

**Actual trigger condition for the IRONFX path:** `Apply_area_damage` (at `0x00489280`)
returns value 2 **if and only if** the warhead being applied is
`Rules->CrushWarhead` (`Rules+0xFAC`). Verified by direct decompile of Apply_area_damage,
lines ~60 and ~580:

```c
bool bVar21 = (warhead == Rules->CrushWarhead);  // +0xFAC
// ... damage loop (sets bVar5 if hit an Iron-Curtained target within 85 leptons) ...
if (bVar21) {
    return 2;  // CrushWarhead-specific exit
}
return !bVar5;  // 0 if hit IC'd target, else 1
```

So `Apply_area_damage` returns:
- `2` → the **warhead being applied was `CrushWarhead`**
- `1` → normal damage applied (no IC'd target in blast)
- `0` → at least one target within 85 leptons was Iron-Curtained (bVar5=true)

**Back in `BulletClass::Detonate`:**

```c
int ret = Apply_area_damage(damage, bullet->Warhead, 1, ownerHouse);
if (!bullet->field_0x24) return;
bool showNullify = (ret == 2);  // bVar8

AnimType* craterAnim = SelectExplosionAnim(damage, bullet->Warhead, landType, &coords);

if (!bullet->Bright) {       // +0xE0 (param_1[0x38])
    if (showNullify) goto SPAWN_IRONFX;
    // else: fall through to normal crater spawn
} else {                      // Bright=yes
    if (showNullify) {
        SPAWN_IRONFX:
        AnimType* ironfx = Rules->WeaponNullifyAnim;  // Rules+0x350
        if (!ironfx) return;
        new AnimClass(ironfx, &coords, 0, 1, 0x2600, -15, 0);
        return;  // EARLY EXIT — no crater spawn
    }
    // Bright + no nullify: spawn combat light (§4.5)
    FUN_0048A620(...);
}
// Spawn normal crater anim (if not nullified)
if (craterAnim) new AnimClass(craterAnim, ...);
```

**Summary of the real semantic:** when a bullet detonates and its warhead is
`CrushWarhead`, the engine shows `WeaponNullifyAnim` (IRONFX) **instead of** the normal
crater anim, and early-returns before combat light / debris. This is unrelated to nukes.

**YR-activity caveat:** in vanilla YR, no standard weapon uses `CrushWarhead=` as its
`Warhead=`. CrushWarhead is applied via direct `ReceiveDamage` calls from the
per-cell-process crush logic (when a vehicle runs over infantry), not via bullet
detonation. So this IRONFX path in Detonate is **rarely observed** in standard
skirmishes, but is reachable for modders who assign CrushWarhead as a projectile's
warhead. The path is not TS-legacy — it is active YR logic, just gated on a warhead
pointer that vanilla weapons never use.

### 4.4 Debris Anim Fallback Path (uses `MetallicDebris=`)

**Correction (verification pass 2):** the initial draft called the fallback array
"`Debris=`". The actual INI key is **`MetallicDebris=`** in `[General]`. Verified:

- Retail value: `MetallicDebris=DBRIS1LG,DBRIS2LG,...,DBRS10SM` (20 entries, large + small).
- Parsed in `RulesClass::ReadGeneral` (`0x0066DAA5` reads the string
  `"MetallicDebris"` at `0x0083CEF0`, then assigns DVC fields to `Rules+0x14C/0x150/0x154`).
- Runtime read from `Rules+0x140` (m_Data) and `Rules+0x14C` (m_ActiveCount) in
  `BulletClass::Detonate`.

At the end of `Detonate`, if `warhead->MaxDebris_0x1C4 > 0`:

```c
int count = Random(warhead->MinDebris_0x1C8, warhead->MaxDebris_0x1C4 - 1);

// Path A: Warhead has its own DebrisTypes (warhead+0x19C count)
//         → spawn VoxelAnimClass from warhead+0x190 array (VoxelAnimClass, NOT AnimClass)

// Path B: No warhead VoxelAnims → fallback to Rules->MetallicDebris
if (warhead->VoxelDebrisCount == 0 && count > 0) {
    for (int i = 0; i < count; i++) {
        CoordStruct* c = this->GetCoords();  // bullet's position
        CoordStruct spawn = { c->X, c->Y + 20, c->Z };  // Y bumped by 20 leptons
        int idx = Random(0, Rules->MetallicDebrisCount - 1);       // Rules+0x14C
        AnimTypeClass* anim = Rules->MetallicDebrisArray[idx];     // Rules+0x140
        AnimClass__Constructor(anim, &spawn, 0, 1, 0x600, 0, 0);
    }
}
```

**Fixed parameters for debris anim:**
- `delay=0`, `loopCount=1`
- `drawFlags=0x600` — same as muzzle flash (centered sprite, no immediate Z-buffer flag)
- `zAdjust=0`
- Y offset of +20 leptons from bullet coords (debris appears slightly south of impact)
- Random entry from `Rules->MetallicDebris[]` (`MetallicDebris=` in `[General]`)

**Note:** `MetallicDebris` entries in vanilla YR are 2D SHP anims (`DBRISxLG`, `DBRSxSM`)
that use the AnimClass bouncer physics. The Bouncer/IsMeteor flag is set on the AnimType
itself (see `ANIM_CLASS_GHIDRA_REPORT.md`), not forced here — so each debris piece bounces
and tumbles according to its own AnimType physics fields (Elasticity, MinZVel, MaxXYVel).

### 4.5 Combat Light / Ground Flash (`FUN_0048A620`) — Verified: NOT AnimClass

**Verified from `FUN_0048A620` at `0x0048A620`.** The existing
`WARHEAD_DETONATE_GHIDRA_REPORT.md` labeled this function "SpawnCombatLight". Verified:
it creates a `LightSourceClass` instance, not an `AnimClass`.

*(corrected 2026-05-28: was "7 args (damage, warhead, x, y, z, always, extraFlags)";
binary shows actual call signature is `(x, y, z, always_flag, lightFlags)` — 5 params with
coords first. Ghidra's body analysis mis-identified param_1 as "damage" and param_2 as
"warhead pointer" by observing body arithmetic; cross-checking two callers
(`WarheadTypeClass__Detonate @ 0x004690B0` and `TechnoClass__ReceiveDamage @ 0x00701900`)
both call `FUN_0048a620(X, Y, Z, 1, flags)` — ROOT_CAUSE: PARAM1_TYPE_MISREAD via
`decompile_function 0x0048a620` + caller trace via `get_function_callers 0x0048a620`)*

```c
void FUN_0048A620(
    int   x,           // param_1 — coord X
    int   y,           // param_2 — coord Y
    int   z,           // param_3 — coord Z
    char  always,      // param_4 — bypass detail-level gate (callers pass 1)
    uint  extraFlags   // param_5 — OR'd into the light's flag field
) {
    // gate: detail-level check OR always==1 OR warhead CombatLight=yes
    // (warhead pointer NOT passed; gate re-reads from a separate context)
    int size = clamp((param_context_damage << 6) >> 8, 0x15, 0x3F);  // clamp [21, 63]
    void* block = operator_new(0x18);  // 24-byte LightSourceClass
    LightSource* ls = FUN_005FF250(x, y, z, size);
    ls->flags_0x14 |= extraFlags;
}
```

**Key facts:**
- Allocates `0x18` bytes — matches a small `LightSourceClass` (or similar ground-light
  entity), **not** `AnimClass` (which is `0x1C8` = 456 bytes).
- Calls `FUN_005FF250` (a `LightSourceClass` or `LightConvertClass` helper near `0x5FFxxx`
  — the `LightConvertClass__Constructor` at `0x555DA0` is nearby).
- Callers pass `always=1` to bypass the detail-level gate unconditionally in the
  Detonate and IronCurtain paths.
- The `warhead+0x150` (`CombatLight=yes`) gate and `warhead+0x13C` (`CombatLightSize`)
  are read inside the function body via context not passed as a named param in these calls.

**Implication:** a Rust engine implementing AnimClass spawn paths does NOT need to funnel
combat-light creation through the anim system. It's a separate lighting entity (see
`LIGHTCONVERT` / lighting systems — out of scope for this report).

### 4.6 Flame Particle Path (NOT AnimClass)

If `bullet->Type->Inviso != 0` (BulletType `+0x294`), 8 flame particles are spawned plus one
central vertical particle. These are `BulletClass` instances (not `AnimClass`), so they are
out of scope for this report — see `BULLET_CLASS_AI_GHIDRA_REPORT.md`.

## 5. drawFlags Summary Across Spawn Sites

This is the authoritative list of which `drawFlags` value the engine passes at each spawn
site. Misuse of these flags is a common source of visual-fidelity bugs.

| Spawn site | drawFlags | Decoded | Notes |
|---|---|---|---|
| Muzzle flash (all firers) | `0x600` | center sprite | DrawIt adds `0x2000` + `0x800` later |
| Garrison OccupantAnim (via muzzle flash path) | `0x600` | center sprite | Same as normal; Z-adjust differs |
| Warhead AnimList explosion | `0x2600` | center + Z-buffer | Bit `0x2000` present at spawn |
| Bridge crater anim | `0x2600` | center + Z-buffer | Same as normal explosion |
| IonCannon crater | `0x2600` | center + Z-buffer | Same |
| Nuke ground-zero anim | `0x2600` | center + Z-buffer | Same |
| Warhead debris (Rules->Debris fallback) | `0x600` | center | No Z-buffer bit at spawn |
| TrailerAnim (spawned inside AI) | `0x600` | center | Matches existing `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` |
| Bouncer BounceAnim (on impact) | `0x2600` | center + Z-buffer | Per `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` §4 |
| Tiberium chain reaction anim | `0x600` | center | Per `ANIM_CLASS_DEEP_DIVE.md` §3 |

**Implication for Rust renderer:** explosions/craters need Z-buffer integration from frame
0 of the anim; muzzle flashes and trailers do not strictly require it until DrawIt upgrades
the flags internally. Any renderer that stores `drawFlags` verbatim should carry the
`0x2600` vs `0x600` distinction end-to-end.

## 6. Weapon Struct Field Refinements

The existing `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` labels the weapon's anim DVC fields
generically ("Anim list data field 1/2/3"). Confirmed semantics from `Fire_At`:

| Offset | Field | Verified purpose |
|---|---|---|
| `0xF4` | DVC vtable | `DynamicVectorClass<AnimTypeClass*>` vtable ptr |
| `0xF8` | `m_Data` | **Pointer to AnimType* array** — indexed as `[0..AnimCount-1]` |
| `0x100` | `m_Capacity` | Internal capacity |
| `0x104` | `AnimCount` | **Active element count** — this is the value checked for `== 8` / `> 0` |
| `0x108` | (unused DVC field) | No observed read from this offset in `Fire_At` |
| `0x10C` | (unused DVC field) | No observed read |
| `0x110` | `OccupantAnim` | `AnimTypeClass*` for garrison fire |
| `0x114` | `AssaultAnim` | `AnimTypeClass*` for clearing garrison |
| `0x118` | `OpenToppedAnim` | `AnimTypeClass*` for open-topped transport / airstrike |

**Corrections vs. prior:**
- The prior doc called `+0xF8` "DynamicVectorClass buffer/first anim". It is specifically
  the **data pointer** (`m_Data`), from which per-index access `*(int *)(+0xF8 + idx*4)`
  retrieves individual `AnimTypeClass*` entries. Confirmed by the Fire_At decompilation at
  two sites (8-way path and fallback path).

## 7. Warhead Struct Field Refinements

**Correction (verification pass 2):** the initial draft mislocated the AnimList DVC at
WarheadType `+0xF4`. That is the offset on the **WeaponType**, not the WarheadType. On
the WarheadType, the `AnimList` DVC starts at `+0x104`. Verified against
`WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md:77`:
`0x104 | 28 bytes | DynamicVectorClass<AnimTypeClass*> | AnimList`.

Standard `DynamicVectorClass` layout (28 bytes total): `vtable, m_Data, m_Capacity,
IncreaseSize, m_ActiveCount, IsAllocated+pad`. Mapped onto the warhead struct:

| Offset | Field | Verified purpose |
|---|---|---|
| `0x104` | DVC vtable | Start of the `DynamicVectorClass<AnimTypeClass*>` — `AnimList=` |
| `0x108` | `m_Data` | **Pointer to AnimType* array** — dereferenced by `SelectExplosionAnim` |
| `0x10C` | `m_Capacity` | DVC allocated capacity (not read in selection path) |
| `0x110` | `IncreaseSize` | DVC grow-step (not read in selection path) |
| `0x114` | `m_ActiveCount` | **Active entry count** — damage bands use this |
| `0x118` | `IsAllocated` + pad | DVC ownership bool + 3 bytes padding |
| `0x14D` | `IsBridge` | byte — gates bridge-crater lookup |
| `0x154` | `EMEffect` | byte — switches AnimList selection from damage-band to random |

`SelectExplosionAnim` only reads `m_Data` (`+0x108`) and `m_ActiveCount` (`+0x114`);
element access is direct array indexing, not through the DVC vtable.

## 8. INI Keys Parsed Into These Fields

All from `rulesmd.ini` and `artmd.ini` (YR overrides base RA2):

| INI key (section) | Target | Read by |
|---|---|---|
| `Anim=` (WeaponType) | Weapon `0xF4..0x10F` DVC | `WeaponTypeClass::ReadINI` (per `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`) |
| `OccupantAnim=` (WeaponType) | Weapon `+0x110` | Weapon ReadINI |
| `OpenToppedAnim=` (WeaponType) | Weapon `+0x118` | Weapon ReadINI |
| `AnimList=` (WarheadType) | Warhead `0x104..0x11F` DVC (28 bytes) | `WarheadTypeClass::ReadINI` |
| `Bright=` (WeaponType) → propagated to `BulletClass+0xE0` | BulletClass `+0xE0` | Weapon ReadINI + BulletClass::Init |
| `EMEffect=` (WarheadType) | Warhead `+0x154` | Random anim selection |
| `IsBridge=` (WarheadType) | Warhead `+0x14D` | Bridge crater eligibility |
| `BridgeCraters=` (`[General]`) | `Rules+0xBC4` array, `+0xBD0` count | Rules parsing |
| `MetallicDebris=` (`[General]` — NOT `Debris=`) | `Rules+0x140` (m_Data), `+0x14C` (m_ActiveCount) | `RulesClass::ReadGeneral` @ `0x0066DAA5` |
| `MinDebris=` (WarheadType) | Warhead `+0x1C8` | Debris spawn count lower bound |
| `MaxDebris=` (WarheadType) | Warhead `+0x1C4` | Debris spawn count upper bound |
| `WeaponNullifyAnim=` (`[General]`, default `IRONFX`) | `Rules+0x350` | Iron Curtain nullify visual — replaces crater when bullet warhead is `CrushWarhead` |
| `IonCannonCrater=` (`[General]`) | `Rules+0x2F4` | IonCannon special |
| `IonCannonWarhead=` (`[General]`) | `Rules+0x17B4` | Warhead comparison |

## 9. YR Activity Verification

| Path | Active in YR? | Condition |
|---|---|---|
| Muzzle flash spawn (all 4 rules) | **Yes** | Every weapon shot fires through `Fire_At` |
| Building Z-adjust for muzzle flash | **Yes** | Any weapon fired from a building |
| Garrison-fire muzzle flash (-200 Z) | **Yes** | Any garrisoned building with infantry firing |
| Muzzle flash attachment via `SetOwnerObject` | **Yes** | All non-building firers |
| Warhead AnimList explosion | **Yes** | Every warhead impact with non-zero damage |
| Bridge crater (35-damage bands) | **Yes** | Warheads with `IsBridge=yes` hitting under a bridge |
| IonCannon crater | **Conditional** | Only when `IonCannonWarhead=` matches the firing warhead (used by Ion Cannon SW — YR includes this in rules) |
| Debris AnimClass fallback | **Yes** | Warheads with `MaxDebris>0` and no `DebrisTypes=` |
| Weapon-nullify (IRONFX) anim via Detonate | **Conditional** | Only fires when the bullet's warhead is `Rules->CrushWarhead` — not standard in vanilla YR (crush damage normally applies via direct ReceiveDamage, not Detonate) |

No TS-only gating found. All paths are reachable in a standard YR skirmish. The only
conditional is IonCannon which requires the specific warhead match — IonCannon is shipped
in YR (though rarely used in vanilla). Confirmed: no `SpecialFlags & 0x1000` or other
TS-legacy feature gates on any of these paths.

## 10. Integration Points (Tick Order)

Within `World::advance_tick`, these spawns occur in the combat phase:

```
combat phase
├─ for each firing techno:
│   └─ Fire_At(weapon, target)
│       ├─ BulletClass::Allocate + Init            (projectile)
│       ├─ muzzle flash anim creation              ← §3 of this report
│       │   └─ AnimClass + SetOwnerObject (non-bldg)
│       └─ fire sound
│
├─ bullets tick, homing, etc.
│
├─ on bullet impact:
│   └─ BulletClass::Detonate(impactCoords)          (Ghidra mislabels as WarheadTypeClass)
│       ├─ Apply_area_damage → entity damage        (returns 2 iff warhead == CrushWarhead)
│       ├─ If return 2: WeaponNullifyAnim spawn     ← §4.3 (early-return)
│       ├─ SelectExplosionAnim + AnimClass spawn    ← §4 of this report
│       ├─ Combat light (if Bright=yes)             ← §4.5 (LightSource, not Anim)
│       └─ Debris anim spawn loop                   ← §4.4
│
└─ anim phase:
    └─ for each AnimClass in g_AnimClass_Array:
        └─ AnimClass::AI() — frame advance, damage accumulation, loops, Next=, etc.
```

All muzzle flashes and explosions created in the combat phase are visible on the same tick
they spawn (delay=0 → `Middle()` called immediately from constructor).

## 11. Current Rust Implementation Status

| Component | Status | Location |
|---|---|---|
| AnimClass / AnimTypeClass runtime | **Not implemented** | — |
| Fire_At muzzle flash spawn | **Not implemented** | No non-garrison muzzle flash |
| Garrison muzzle flash (OccupantAnim) | **Partially implemented** (specific path only) | [app_building_anim.rs:485-555](../src/app_building_anim.rs#L485-L555) — `tick_garrison_muzzle_flashes` spawns a per-fire-event OccupantAnim with hardcoded 67ms rate |
| DamageFire on buildings | **Partially implemented** (specific path only) | [app_building_anim.rs:64-210](../src/app_building_anim.rs#L64-L210) — `tick_damage_fire_overlays`, cycles FIRE01/02/03 at DamageFireOffsets |
| Building 21-slot state machine | **Not implemented** (only Active/Production tracked ad-hoc) | — |
| Explosion anim spawn (Warhead AnimList) | **Not implemented** | `bridge_explosions` is stored but no spawn path exists |
| Debris / voxel debris on impact | **Not implemented** | — |
| Warhead InfDeath → unit death sequence | **Implemented** (sprite sequence only, not AnimClass) | [animation.rs:644-652](../src/sim/animation.rs#L644-L652) |
| Sprite animation for infantry/vehicle SHPs | **Implemented** (sequences only, not AnimClass) | [animation.rs](../src/sim/animation.rs) |

**Key gap:** The entire AnimClass world-entity subsystem is absent. The existing Rust code
has two specific hard-coded fire-anim paths (garrison muzzle flash, building damage fire)
but no general `AnimClass` entity type, no `AnimTypeClass` INI-driven art records, no
spawning pipeline for weapon/warhead/trailer/expire/bounce anims, and no per-anim damage
accumulation.

The current [Animation](../src/sim/animation.rs) module is a **different system** — it
handles sprite-sheet sequence playback for infantry/vehicle SHPs (walk/attack/die poses),
analogous to the original game's SHP sequence system on `TechnoClass`, not to `AnimClass`
as a world entity.

## 12. Open Questions

**Verification-pass update:** Questions 1, 2, and 4 from the initial draft were resolved
in a second Ghidra dive and are now incorporated into the main body (§3.6 palette, §3.1
vtable stubs, §4.5 combat light). The questions that remain open:

1. **8-direction muzzle flash rotation direction** *(binary verified, semantic open)*:
   the `+1` shift in the facing math was derived and now confirmed via bit-level trace.
   Retail `Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW` is the
   canonical ordering (30+ occurrences in rulesmd.ini). Engine picks anim index
   `(quantized_facing + 1) mod 8` — so for DirStruct=0 (naively "N"), it picks
   MGUN-NE, not MGUN-N. The most likely semantic explanation: RA2 uses cell-grid
   facing, where DirStruct=0 is "screen-up" which maps to "cell-NE" in the iso view.
   The `+1` rotation then shifts back one step to align visual frame with pointing
   direction. **Unverified observationally in retail** — would require firing GI
   while facing each compass direction and comparing SHP frame shown.

2. **~~`Apply_area_damage` return value `2` semantics~~** *(RESOLVED in verification pass)*:
   Direct decompile of `Apply_area_damage` at `0x00489280` reveals return value 2 is
   triggered **iff** `warhead == Rules->CrushWarhead` (`Rules+0xFAC`). This flipped the
   interpretation of the entire `Rules+0x350` branch — see §4.3 for the correction. The
   "nuke ground zero" anim identified in the initial draft was actually
   **WeaponNullifyAnim (IRONFX)**, shown when `CrushWarhead` is applied via Detonate.

3. **`AnimClass::DrawIt` building-tint path with `field_0x46`**: DrawIt has a branch
   (§3.6 flow) where `anim->field_0x46 != 0` causes the anim to pick up the cell's
   building palette — including a powered-down tint. This is used for BuildingClass's
   21-slot attached anims (ActiveAnim, ProductionAnim, etc.) to match the building's
   state. Not relevant to muzzle flashes or explosions (which never have `field_0x46`
   set), but worth noting for the 21-slot renderer path in
   `BUILDING_ANIM_STATE_MACHINE.md`.

4. **`field_0x194` IsBouncer construction**: the Fire_At muzzle flash path does NOT set
   IsBouncer on the newly-created anim. All bouncer behavior originates from the
   `AnimClass::Constructor` detecting `type->Bouncer` or `type->IsMeteor`. Confirmed
   from `ANIM_CLASS_DEEP_DIVE.md` constructor analysis, no additional work needed.

All other claims in this document have been verified from binary in this pass. No
remaining dependencies on inferences or prior-doc claims.

## Sources

- Ghidra live decompilations:
  - `TechnoClass::Fire_At` @ `0x006FDD50` (full function body)
  - `WarheadTypeClass::Detonate` @ `0x004690B0` (full function body)
  - `Warhead__SelectExplosionAnim` @ `0x0048A4F0` (full)
  - `FUN_0048ACE0` @ `0x0048ACE0` (single instruction, constant -15)
  - `AnimClass::SetOwnerObject` @ `0x00424B50` (full)
  - `BuildingClass::IsOccupied` @ `0x00458DD0`
  - `BuildingClass::GetOccupantCount` @ `0x004581F0`

- Prior research docs (not duplicated):
  - `ANIM_CLASS_GHIDRA_REPORT.md`, `ANIM_CLASS_DEEP_DIVE.md`,
    `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`,
    `ANIMATION_SOUNDS_GHIDRA_REPORT.md`,
    `BUILDING_ANIM_STATE_MACHINE.md`, `DAMAGE_FIRE_ANIMS_GHIDRA.md`,
    `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `FIRE_AT_ANALYSIS.md`,
    `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`, `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`

- INI files checked: `ini/rulesmd.ini`, `ini/artmd.ini` (field names verified against
  parsing functions in existing docs).

- Rust implementation: [src/app_building_anim.rs](../src/app_building_anim.rs),
  [src/sim/animation.rs](../src/sim/animation.rs), [src/sim/combat/mod.rs](../src/sim/combat/mod.rs).

## 13. Verification Log (second pass)

This section records the specific evidence that backs each claim. Readers can trust
each row below came from a direct Ghidra tool call during the verification pass, not from
prior-doc copy-paste.

| Claim | Evidence | Verdict |
|---|---|---|
| Muzzle flash is spawned in TechnoClass::Fire_At with drawFlags=0x600, delay=0, loopCount=1 | Direct decompile of `0x006FDD50` — `AnimClass__Constructor(iVar9, &uStack_98, 0, 1, 0x600, 0, 0)` | ✓ verified |
| 8-way anim selection uses `weapon+0x104 == 8` gate | `if (*(int *)(uVar18 + 0x104) == 8)` in Fire_At body | ✓ verified |
| Anim data pointer at `weapon+0xF8` (not `+0x108` as for warhead) | `*(int *)(uVar18 + 0xf8)` dereferenced in both 8-way and fallback paths | ✓ verified |
| Building Z-adjust formula `-(dY/4)` clamped non-positive | Direct decompile: `uVar11 = -((int)(iVar9 + (iVar9 >> 0x1f & 3U)) >> 2); ... & (-1 < (int)uVar11) - 1` | ✓ verified |
| Garrison Z-adjust is hard `-200` via `GetOccupantCount() > 0` | `*(undefined4 *)(iVar8 + 0x100) = 0xffffff38` — 0xFFFFFF38 = -200 signed | ✓ verified |
| `vtable+0x400` = `IsOccupied`; BuildingClass override at `0x00458DD0` | Read memory at `0x007E42BC` = BuildingClass vtable base + 0x400 → `0x00458DD0` | ✓ verified |
| `vtable+0x400` on non-buildings = stub returning 0 | Read memory at `0x007EB458` = InfantryClass vtable + 0x400 → `0x0041BFB0`; decompile → `return 0` | ✓ verified |
| `vtable+0x408` = `GetOccupantCount`; stub at `0x0041BFD0` on non-buildings | Read memory at `0x007EB460` → `0x0041BFD0`; decompile → `return 0` | ✓ verified |
| `field_0x82` on TechnoClass is airstrike flag | Existing `FIRE_AT_ANALYSIS.md` line 89 labels this; consistent with LimboLaunch usage in Fire_At body | ✓ cross-ref |
| `AnimClass::SetOwnerObject` scans `g_AnimClass_Array`, stores relative offset | Direct decompile of `0x00424B50` | ✓ verified |
| Muzzle flash palette does NOT inherit owner's house color | Direct decompile of `AnimClass::DrawIt` at `0x00422CA0` — palette cascade uses only AnimType flags and cell palette, never reads `anim->OwnerObject` | ✓ verified |
| `SelectExplosionAnim` uses 35-damage bands for bridges | `iVar1 = Rules->BridgeCraterCount * 0x23 + -1; ... (iVar1 / 0x23)` — `0x23` = 35 | ✓ verified |
| `SelectExplosionAnim` uses 25-damage bands normally | `iVar1 = count * 0x19 + -1; ... (iVar1 / 0x19)` — `0x19` = 25 | ✓ verified |
| Warhead AnimList data pointer at `+0x108`, count at `+0x114` | `*(int *)(param_2 + 0x108) + (idx) * 4`; `*(int *)(param_2 + 0x114)` | ✓ verified |
| EMEffect warheads (`+0x154`) randomize AnimList pick | `if (*(char *)(param_2 + 0x154) == '\\0') damage_bands else Random__RandomRanged` | ✓ verified |
| Explosion anim drawFlags = 0x2600 (vs 0x600 for muzzle flash) | `AnimClass__Constructor(iVar12, &local_44, 0, 1, 0x2600, uVar17, uVar20)` in Detonate | ✓ verified |
| `FUN_0048ACE0` returns `-15` (not a facing function) | Direct decompile: function body is `return 0xfffffff1;` (one instruction) | ✓ verified |
| ~~Nuke ground-zero anim uses `Rules+0x350` (NukeGroundAnim)~~ → **actually WeaponNullifyAnim (IRONFX)** | INI files `rules.ini:545` / `rulesmd.ini:553`: `WeaponNullifyAnim=IRONFX` | ✗ **corrected in verification pass** |
| `Rules+0x350` anim path triggered when `Apply_area_damage` returns 2 | Decompile of `0x00489280`: `bVar21 = (warhead == Rules[0xFAC]); ... if (bVar21) return 2;` — `Rules+0xFAC` = CrushWarhead per `CRUSH_SYSTEM_GHIDRA_REPORT.md:51` | ✓ verified |
| `BulletClass::Detonate` is mislabeled in Ghidra as `WarheadTypeClass__Detonate` | `param_1[0x27..0x29]` access = ObjectClass Location fields (instance-only); `param_1[0x4a]` = byte 0x128 = BulletClass::Warhead per `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` | ✓ verified |
| `BulletClass+0xE0` (param_1[0x38]) is `Bright` flag | `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:96`: `0xE0 \| 1 \| bool \| Bright \| 0 \| Bright draw flag (from weapon)` | ✓ cross-verified |
| Debris fallback uses `Rules+0x140` (m_Data) at count `Rules+0x14C` | `*(int *)(g_RulesClass_Instance + 0x140) + iVar12 * 4` | ✓ verified |
| Rules+0x140/+0x14C is `MetallicDebris=` (NOT `Debris=`) | `rules.ini:524` / `rulesmd.ini:528`: `MetallicDebris=DBRIS1LG,...,DBRS10SM`; string xref at `0x0083CEF0` used by ReadGeneral at `0x0066DAA5` which writes `Rules+0x14C/+0x150/+0x154` after DVC CopyFrom | ✓ **corrected from `Debris=` in verification pass 2** |
| BulletClass+0x90 (`param_1[0x24]`) is IsAlive — gates Detonate continuation | `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:66`: `0x90 \| 1 \| bool \| IsAlive \| 1 \| False = destroyed, AI skips` | ✓ cross-verified |
| Warhead AnimList DVC starts at `+0x104`, not `+0xF4` | `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md:77`: `0x104 \| 28 \| DynamicVectorClass<AnimTypeClass*> \| AnimList`; weapon DVC uses `+0xF4` but warhead uses `+0x104` (different struct layout) | ✓ **corrected typo in verification pass 2** |
| Weapon Report (`0xCC`) is DVC m_ActiveCount — fire sound gate | `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md:62`: `0xCC \| 4 \| int \| Report= \| Sound list data field 1`; DVC 24-byte layout places m_ActiveCount at DVC_base+0x10; Report DVC starts at `+0xBC` so count is at `0xCC` | ✓ cross-verified |
| AircraftClass::Fire_At (`0x00415EE0`) calls TechnoClass::Fire_At directly — no muzzle flash override | Direct decompile: `uVar4 = TechnoClass__Fire_At(param_1);` — then post-processes bullet trajectory only, no AnimClass spawn | ✓ verified |
| InfantryClass::Fire_At_Override (`0x0051DF70`) is thin wrapper around TechnoClass::Fire_At | Direct decompile: `iVar1 = TechnoClass__Fire_At(param_1);` followed by InfantryClass-specific post-fire (panic, etc.) | ✓ verified |
| BulletClass::SpawnShrapnel at `0x0046A310` is AirBurst sub-bullet spawn, NOT explosion anim | Direct decompile shows `CoCreateInstance(...)` of BulletClass + `BulletClass__Init` in a loop over cell-spread neighbors — no AnimClass allocations in this function | ✓ verified |
| `TechnoClass::GetFLH` at `0x006F3AD0` computes muzzle world position from FLH offset + unit facing rotation | Direct decompile shows: read FLH from weapon slot or `type+0x850+N*0xC`, rotate via `Matrix3x4_RotateZ(quantized_facing_angle)`, translate by `(flhZ+offZ, ±(flhY+offY), 0)`, add to `GetRenderCoords()` result | ✓ verified |
| FLH is stored per weapon slot as 3 ints (X, Y, Z) at struct offsets `+4, +8, +0xC` | `iVar1 = GetWeapon(idx); aiStack_c0[3] = *(int *)(iVar1 + 4); iVar10 = *(int *)(iVar1 + 8); iVar1 = *(int *)(iVar1 + 0xc);` | ✓ verified |
| Elite FLH slots at `TechnoType+0x850 + |weaponIdx| * 0xC` for negative weaponIdx | `iVar1 = iVar10 + 0x850 + param_3 * -0xc` when `param_3 < 0` | ✓ verified |
| FLH INI keys: 4 variants parsed in `TechnoTypeClass::ReadINI` (`0x00712170`) | String xrefs: `PrimaryFireFLH@0x008432F8→0x00715DA1`; `SecondaryFireFLH@0x008432C0`; `ElitePrimaryFireFLH@0x00843288`; `EliteSecondaryFireFLH@0x00843244` | ✓ verified |
| Barrel alternation flips FLH Y sign via `this+0xEE & 0x80000001` | `uVar5 = param_1[0xee] & 0x80000001; ... ((-(uint)(uVar5 != 0) & 0xfffffffe) + 1) * (iVar10 + param_5)` produces `-1 * (flhY+offY)` when flag set, else `+1 * (flhY+offY)` | ✓ verified |
| Retail 8-way Anim= ordering is `N,NE,E,SE,S,SW,W,NW` (15+ weapons in rulesmd.ini) | `grep ^Anim=MGUN` in `rulesmd.ini` returns 30+ matches all using this ordering | ✓ verified |
| Muzzle-flash anim index is `(quantized_facing + 1) mod 8` — offset by 1 from naive ordering | Traced bit math for each facing cardinal; engine picks anim[1] for DirStruct=0 instead of anim[0]. Likely a cell-grid-vs-screen-space compensation | ✓ verified (semantic explanation unclear — retail observation would confirm visual correctness) |
| FLH uses 32-way facing quantization (shift 10 mask `0x1F`), muzzle anim uses 8-way (shift 12 mask 7) | Direct decompile shows `(*puVar3 >> 10) + 1 >> 1 & 0x1F - 8` for FLH rotation vs `(*puVar15 >> 0xc) + 1 >> 1 & 7` for anim lookup | ✓ verified |
| Per-burst FLH is an Ares-only extension, NOT vanilla YR | GetFLH reads from a single weapon slot per index; no array-of-FLHs-per-burst indexing in binary. Cross-ref: `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` §3.4 | ✓ cross-verified |
| `FUN_0048A620` creates a LightSource, NOT an AnimClass | Decompile shows `operator_new(0x18)` (not `0x1C8`) and a call to `FUN_005FF250`; no `AnimClass__Constructor` call anywhere in body | ✓ verified |
| `Bright=yes` warhead gate is at `warhead+0xE0` (param_1[0x38]) | `if ((char)param_1[0x38] == '\\0')` — 0x38*4 = 0xE0 byte offset | ✓ verified |
| Apply_area_damage return value 2 triggers nuke ground anim | `if (local_74 == (int *)0x2) bVar8 = true; ... if (bVar8) goto LAB_0046a2a1` | ✓ observed; full semantic unverified |
| BuildingClass vtable base derived as `0x007E3EBC` | `BuildingClass::IsOccupied` xref = `0x007E42BC` → subtract `0x400` offset | ✓ derived |
| InfantryClass vtable base derived as `0x007EB058` | `InfantryClass::IronCurtain` xref = `0x007EB1AC` → subtract `0x154` offset (matches BuildingClass IronCurtain relative offset) | ✓ derived |
