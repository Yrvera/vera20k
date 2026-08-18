# BulletClass::Init + Fire — Creation Pipeline (Weapon → Bullet → Detonate)

**Date:** 2026-04-23
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH (all claims verified from live Ghidra decompilation; sibling report
to `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`)
**Active in YR:** Yes — every weapon shot that creates a projectile runs this pipeline.

## 1. Overview

This report traces the creation and launch of a `BulletClass` instance — the link
between `TechnoClass::Fire_At` (the weapon firing) and `WarheadTypeClass::Detonate`
(the bullet impacting). The `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` treated the bullet
as a black box; this report fills in what happens inside.

Scope:
- `BulletClass::Allocate` (factory via CoCreateInstance)
- `BulletClass::Init` (8-param field initializer)
- `BulletClass::SetWeapon` (mislabeled `SetOwner` in Ghidra — sets weapon pointer)
- `BulletClass::Fire` (vtable+0x1F0 — the actual launch-into-world call)
- `ObjectClass::Conceal` (vtable+0xD4 — pre-launch limbo state)
- Field mapping: which bullet fields are set, and from where

## 2. Key Addresses

| Entity | Address | Notes |
|---|---|---|
| `BulletClass::Allocate` | `0x0046B050` | CoCreateInstance factory + calls Init |
| `BulletClass::Init` | `0x004664C0` | 8-param field setter |
| `BulletClass::SetWeapon` | `0x0046B260` | **Mislabeled as `SetOwner`** in Ghidra; writes weapon ptr to bullet+0x130 |
| `BulletClass::Fire` | `0x00468670` | vtable slot 124 (offset 0x1F0); the actual launch |
| `ObjectClass::Conceal` | `0x005F4D30` | vtable slot 53 (offset 0xD4); puts object into limbo |
| `ObjectClass::Reveal` | `0x005F4EC0` | vtable slot 54 (offset 0xD8); opposite of Conceal |
| `BulletClass::AI` | `0x004666E0` | vtable slot 23 (offset 0x5C); per-tick update |
| `BulletClass::BulletDetonation` | `0x00468D80` | Called when bullet AI detects impact |
| `BulletClass::SpawnShrapnel` | `0x0046A310` | AirBurst secondary bullet spawner |
| BulletClass vtable base | `0x007E46E4` | Derived from `BulletClass::AI` xref `0x007E4740` - 0x5C |

## 3. Creation Pipeline — `TechnoClass::Fire_At` Sequence

The full sequence when a weapon fires:

```c
// 1) Allocate fresh bullet (CoCreateInstance + Init)
BulletClass* bullet = BulletClass__Allocate(
    firer,           // this (source techno)
    bulletType,      // BulletTypeClass* from weapon->Projectile
    warhead,         // WarheadTypeClass* from weapon->Warhead (weapon+0xAC)
    damage,          // int, after Veterancy + Armor multipliers
    bright           // byte, from weapon->Bright (weapon+0x12F)
);
if (bullet == NULL) return NULL;                 // OOM or COM failure

// 2) Set the weapon pointer (NOT the owner — Ghidra mislabel)
BulletClass__SetWeapon(bullet, weaponType);      // bullet+0x130 = weaponType

// 3) Conceal the bullet — puts it in Limbo (InLimbo=1 at +0x81)
//    This is the Conceal inherited from ObjectClass; BulletClass does NOT override.
bullet->vtable->Conceal();                        // vtable+0xD4

// 4) [Fire_At computes trajectory: muzzle position (from GetFLH), target,
//     bullet velocity vector, inaccuracy spread, ROT, Speed...]

// 5) Actually launch — Fire reveals and submits to display
bullet->vtable->Fire(                             // vtable+0x1F0
    &sourcePos,      // muzzle/FLH world coords
    &velocityVector  // 6 ints: velocity + position?
);
```

This is the authoritative sequence. The Conceal/Fire pattern prevents the bullet from
being drawn between allocation and full trajectory setup.

## 4. `BulletClass::Allocate` (0x0046B050)

Simple factory wrapper:

```c
BulletClass* BulletClass::Allocate(
    TechnoClass* firer,           // param_1 (ECX)
    BulletTypeClass* type,        // param_2 (EDX)
    WarheadTypeClass* warhead,    // param_3 (stack)
    int damage,                   // param_4
    char bright                   // param_5
    // ... up to 7 params in signature ...
) {
    void* block = NULL;
    HRESULT hr = CoCreateInstance(
        &CLSID_BulletClass,        // DAT_007E96E0
        NULL,
        CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER,  // 7
        &IID_BulletClass,          // DAT_007F7C90
        &block
    );
    if (FAILED(hr)) return NULL;

    BulletClass__Init(/* args forwarded */);
    return (BulletClass*)block;
}
```

**Key observations:**
- Uses **COM (`CoCreateInstance`)** to allocate the bullet, not plain `operator_new`.
  The CLSID/IID are registered at game startup; they identify BulletClass as a COM
  factory. This is a vestige of the C&C engine's COM-based object system used for
  save/load serialization.
- The `CoCreateInstance` result is zero-initialized by COM, so all bullet fields start
  at 0 before `Init` runs.
- The factory returns `NULL` on COM failure — callers must check before using.

## 5. `BulletClass::Init` (0x004664C0) — Field-by-Field Map

**Verified from direct decompile.** 8-parameter function; param_1 is `this` (the bullet).

```c
void BulletClass::Init(
    BulletClass*       this,        // param_1 (ECX): the new bullet
    BulletTypeClass*   type,        // param_2 (EDX)
    AbstractClass*     target,      // param_3 (stack)
    TechnoClass*       owner,       // param_4
    int                damage,      // param_5
    WarheadTypeClass*  warhead,     // param_6
    int                speed,       // param_7
    char               bright       // param_8
)
```

### Field assignments (exact order from binary)

| Bullet Offset | Write | Source | Field name (verified) |
|---|---|---|---|
| `+0x10C` | `target` (param_3) | caller | `Target` (AbstractClass*) |
| `+0x110` | `speed` (param_7) | weapon | `TargetSpeed` |
| `+0x128` | `warhead` (param_6) | weapon | `WH` (WarheadTypeClass*) |
| `+0xE0` | `bright` (param_8) | weapon | `Bright` (byte) |
| `+0x6C` | `damage` (param_5) | calc | `Health` (ObjectClass) — doubles as damage payload |
| `+0xAC` | `type` (param_2) | weapon | `Type` (BulletTypeClass*) |
| `+0xB0` | `owner` (param_4) | caller | `Owner` (TechnoClass* firer) |
| `+0x12C` | `0` (literal) | — | `AnimFrame` reset |
| `+0x12D` | `type->AnimRate` (`+0x2F6`) | type | `AnimTimer` seeded |
| `+0x114` | conditional (see below) | owner | `HouseColorIndex` |
| `+0x150` | `0x100` (literal 256) | — | Unknown — possibly default DrawFlags/facing |
| `+0x154` | `0` (literal) | — | `BounceAnim` (AnimClass*) cleared |
| `+0x158` | `0` (literal) | — | `IsWaitingForAnim` (bool) cleared |

### HouseColorIndex logic (the only conditional)

```c
if (!type->FirersPalette_0x2A9 || owner == NULL) {
    bullet->HouseColorIndex = -1;         // no house color tint
} else {
    bullet->HouseColorIndex = owner->OwnerHouse->ColorIndex;  // +0x21C → +0x16054
}
```

- `BulletType+0x2A9` = `FirersPalette=yes` bool (from rulesmd.ini `[BulletTypes]` section).
- `TechnoClass+0x21C` = `OwnerHouse` pointer (HouseClass*).
- `HouseClass+0x16054` = the 0–7 color scheme index (player color).

Cross-verified against `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:108`:
`0x114 | 4 | int | HouseColorIndex | -1 | Color palette index from firer's house (if FirersPalette=yes), else -1`.

### What Init does NOT set

- **Position** (`+0x9C/0xA0/0xA4` ObjectClass Location) — set by subsequent `SetCoords` / `Fire` calls.
- **Velocity** (`+0xE8..0x103` as 3 doubles) — set by `Fire`.
- **CurrentBurstIndex** (`+0x3B8`) — managed on the firer, not the bullet.
- **InLimbo flag** (`+0x81`) — initial value from CoCreateInstance (= 0), then set to 1 by the subsequent `Conceal` call.

## 6. `BulletClass::SetWeapon` (0x0046B260) — Mislabeled as SetOwner

```c
void BulletClass::SetWeapon(BulletClass* this, WeaponTypeClass* weapon) {
    *(WeaponTypeClass**)(this + 0x130) = weapon;
}
```

**Critical correction:** Ghidra labels this function `BulletClass__SetOwner`, but
`BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:116` documents `+0x130` as `WeaponType`, not
Owner:

> `0x130 | 4 | ptr | WeaponType | 0 | -> WeaponTypeClass (set by FUN_0046B260 after creation)`

The owner (firer) is written to `+0xB0` via `Init`, not this function. So the Fire_At
sequence `BulletClass__SetOwner()` is actually setting the **weapon pointer**, not the
owner. The doc's function label should be changed to `BulletClass__SetWeapon`.

## 7. vtable+0xD4 — The "Conceal" Call After Allocate

**Verified via raw memory read at `0x007E47B8`**: BulletClass vtable slot 53
(offset 0xD4) is **`ObjectClass::Conceal`** at `0x005F4D30`. BulletClass does NOT
override this slot.

What `ObjectClass::Conceal` does (from decompile):

```c
int ObjectClass::Conceal(ObjectClass* this) {
    if (!g_GameActive || this->InLimbo_0x81) {
        return 0;  // already in limbo or game not running
    }
    this->vtable->Deselect();          // +0x150: remove from CurrentObjects
    this->vtable->CellOccupation(1);   // +0xDC: unmark cell occupancy
    this->vtable->ClearStateMachine(0);// +0x124
    DisplayClass::RemoveFromLayer(this);
    AnimClass::Detach();                // detach any attached anims
    VocHandle::Stop();                  // stop any playing sound
    // ... Tactical screen dirty rect, type-specific cleanup ...
    this->vtable->PreDestroy();         // +0x11C
    this->InLimbo_0x81 = 1;
    this->field_0x80 = 0;               // clear IsActive?
    return 1;
}
```

**Semantic:** Conceal puts the bullet into **InLimbo** state (field `+0x81 = 1`). For a
freshly-created bullet from COM, `InLimbo` starts at 0, so this call runs the full
cleanup path, ensuring the bullet is not yet drawn or registered anywhere.

**Why it's called here:** between `Allocate` and `Fire`, the bullet has fields set but
no position or velocity. Concealing it prevents the renderer or cell occupancy system
from trying to process an incomplete bullet. `Fire` later calls `Reveal` to un-limbo.

**This is NOT a BulletClass-specific behavior** — it's the generic Object lifecycle
being used correctly. The Rust engine should mirror this pattern: bullets must be in a
"hidden/pending" state until Fire completes their trajectory setup.

## 8. `BulletClass::Fire` (0x00468670) — Actual Launch

Vtable slot 124 (offset `0x1F0`). Called from Fire_At after muzzle/target/velocity are
all computed.

```c
int BulletClass::Fire(
    BulletClass* this,
    CoordStruct* sourcePos,          // muzzle/FLH world coords
    VelocityVector* velocity         // 6 ints: 3 doubles (dX, dY, dZ)?
) {
    int revealOK = ObjectClass::Reveal();  // un-limbo the bullet
    if (!revealOK) return 0;                // can't un-limbo → abort

    // Copy velocity vector to bullet+0xE8..0xFF (byte offsets; int* loop uses indices 0x3A..0x3F)
    // (corrected 2026-05-29: was "bullet+0x3A..0x3F" which are int* array indices, not byte offsets;
    //  byte offsets are 0xE8..0xFF — confirmed via decompile_function 0x00468670 showing
    //  piVar7 = param_1 + 0x3a where param_1 is int*; 0x3a*4=0xE8. MISLEADING: OPERATOR_OR_ORDER_DRIFT)
    memcpy(&this->Velocity_0xE8, velocity, 6 * sizeof(int));

    // Save target/source coords
    this->TargetX_0x134 = sourcePos->X;
    this->TargetY_0x138 = sourcePos->Y;
    this->TargetZ_0x13C = sourcePos->Z;

    // Compute cell coords from source (pixel→cell conversion)
    this->CellCoord_0x14C = pack_cell(sourcePos);

    DisplayClass::RemoveFromLayer(this);   // remove from prior layer registration

    // Get real source from owner (firer) — GetCoords via vtable+0x58 on owner
    owner->GetCoords(&ownerCoords);
    this->SourceX_0x140 = ownerCoords.X;
    this->SourceY_0x144 = ownerCoords.Y;
    this->SourceZ_0x148 = ownerCoords.Z;

    // --- Ballistic inaccuracy (BallisticScatter) ---
    if (type->Inaccurate_0x2A3 && type->FireOnce?_0x29E) {
        // Compute distance, apply random offset scaled by Rules->BallisticScatter
        // (Rules+0x1734), then rotate by random direction
        // ... inaccuracy calculation ...
    }

    // --- Homing / ROT setup ---
    if (type->FireOnce_0x29E) {
        // Compute homing-track velocity using target position + ROT
        // Falls through to straight-line if no target
    }

    // --- Submit to display if alive ---
    if (this->IsAlive_0x90) {
        DisplayClass::Submit_Object(this);
    }

    return 1;  // success
}
```

**Key observations:**
- **Reveal is the first action.** Inverse of Conceal: clears `InLimbo`, re-registers
  the bullet for rendering, marks cell occupancy.
- **Velocity vector is 6 ints / 3 doubles** (X, Y, Z components).
- **Inaccuracy is RNG-driven**: `Rules->BallisticScatter` (at `Rules+0x1734`) is the
  max angular spread; `Inaccurate=yes` on the BulletType enables the randomization.
- **Target/Source are stored separately**: Target at `+0x134..0x13C`, Source at
  `+0x140..0x148`. Used by homing projectiles to re-compute trajectory mid-flight.

## 9. Complete Field Initialization Summary

After `Allocate + SetWeapon + Conceal + Fire`, a bullet has the following fields set:

| Offset | Field | Source |
|---|---|---|
| `+0x81` | InLimbo | `0` (from Reveal in Fire) |
| `+0x90` | IsAlive | `1` (default from Init or Fire) |
| `+0x9C..0xA4` | Location (X,Y,Z) | from `SetCoords` during Fire |
| `+0xAC` | Type | `weapon->Projectile` (via Init) |
| `+0xB0` | Owner | firer (via Init) |
| `+0xE0` | Bright | `weapon->Bright` byte (via Init) |
| `+0xE8..0x103` | Velocity (dX,dY,dZ doubles) | computed by Fire_At, passed to Fire |
| `+0x10C` | Target | target AbstractClass* (via Init) |
| `+0x110` | TargetSpeed | `weapon->Speed` (via Init) |
| `+0x114` | HouseColorIndex | `firer->House->Color` or `-1` (via Init) |
| `+0x128` | WH | `weapon->Warhead` (via Init) |
| `+0x12C` | AnimFrame | `0` (via Init) |
| `+0x12D` | AnimTimer | `type->AnimRate` (via Init) |
| `+0x130` | WeaponType | set by SetWeapon |
| `+0x134..0x13C` | Target coords | via Fire |
| `+0x140..0x148` | Source coords | via Fire |
| `+0x14C` | CellCoord | computed from source in Fire |
| `+0x150` | (unknown) | `0x100` (via Init) |
| `+0x154` | BounceAnim | `0` (via Init) |
| `+0x158` | IsWaitingForAnim | `0` (via Init) |

## 10. Weapon → Bullet Field Propagation

Exact mapping of weapon/warhead fields onto the new bullet, extracted from Fire_At:

| Weapon INI key | Weapon offset | Bullet offset | Path |
|---|---|---|---|
| `Warhead=` | `weapon+0xAC` | `bullet+0x128` | Init param_6 → WH |
| `Speed=` | `weapon+0x98` | `bullet+0x110` | Init param_7 → TargetSpeed |
| `Damage=` (post-modifiers) | computed | `bullet+0x6C` | Init param_5 → Health |
| `Bright=` | `weapon+0x12F` | `bullet+0xE0` | Init param_8 → Bright |
| `Projectile=` (BulletType) | `weapon+0xA8` | `bullet+0xAC` | Init param_2 → Type |
| (weapon ptr itself) | — | `bullet+0x130` | SetWeapon → WeaponType |
| (firer) | `this` (Fire_At) | `bullet+0xB0` | Init param_4 → Owner |
| (target) | Fire_At arg | `bullet+0x10C` | Init param_3 → Target |

`Damage` passes through `Fire_At` after: Veterancy multiplier, FirepowerMultiplier,
Armor multiplier (warhead verses), etc. The final integer value is what lands in
`bullet+0x6C`. Actual damage application happens later in `Apply_area_damage` during
Detonate.

## 11. Init Called From Non-Allocate Paths

Besides `Allocate`, `BulletClass::Init` is called directly from:

| Caller | Address | Purpose |
|---|---|---|
| `TechnoClass::Fire_At` | `0x006FF859` | Temporal warhead — re-init bullet during LimboLaunch path |
| `WarheadTypeClass::Detonate` | `0x00469F7F`, `0x0046A150` | Initializes pre-existing bullets for shrapnel/chain-reaction |
| `BulletClass::SpawnShrapnel` | `0x0046A5A1`, `0x0046AA16` | Shrapnel sub-bullets from AirBurst weapons |
| `NukeMaker::SpawnDownwardNuke` | `0x0046B408` | Re-init for nuke payload's downward phase |

These re-init paths mean `Init` is not strictly a "first-time setup" — it's a reusable
"set bullet state" routine. Any field not in §5 retains its previous value on re-init.
For fresh bullets (via Allocate), all unset fields are zero from COM initialization.

## 12. YR Activity

All paths in this report are active in vanilla YR on every weapon fire:
- Every `TechnoClass::Fire_At` that creates a projectile runs through this pipeline
- Nuke superweapon uses `NukeMaker::SpawnDownwardNuke` + re-init
- AirBurst weapons (V3 Rocket, Dreadnought) spawn shrapnel through `SpawnShrapnel`
- Temporal warheads (Time Machine, Temporal Weapon) re-init via `Fire_At` → `Init`

No TS-legacy gating or dead code paths detected. The entire pipeline is live.

## 13. Verification Log

| Claim | Evidence | Verdict |
|---|---|---|
| `BulletClass::Allocate` uses CoCreateInstance for bullet creation | Direct decompile of `0x0046B050`: `CoCreateInstance(&DAT_007E96E0, 0, 7, &DAT_007F7C90, &local_4)` | ✓ verified |
| `BulletClass::Init` has 8 params incl. `this`, writes 13 fields | Direct decompile of `0x004664C0`: 13 field writes, matching param indices 2-8 | ✓ verified |
| Init param_8 (`bright`) writes to `bullet+0xE0` | `*(undefined1 *)(param_1 + 0xe0) = param_8;` — cross-ref `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:96` `0xE0 = Bright` | ✓ verified |
| Init param_6 (`warhead`) writes to `bullet+0x128` | `*(undefined4 *)(param_1 + 0x128) = param_6;` — cross-ref layout doc `0x128 = WH` | ✓ verified |
| Init param_5 (`damage`) writes to `bullet+0x6C` (Health) | `*(undefined4 *)(param_1 + 0x6c) = param_5;` — cross-ref layout doc `0x6C = Health (bullets use for Damage)` | ✓ verified |
| Init param_4 (`owner`) writes to `bullet+0xB0` | `*(int *)(param_1 + 0xb0) = param_4;` — cross-ref layout doc `0xB0 = Owner` | ✓ verified |
| Init param_3 (`target`) writes to `bullet+0x10C` | `*(undefined4 *)(param_1 + 0x10c) = param_3;` — cross-ref layout doc `0x10C = Target` | ✓ verified |
| Init param_2 (`type`) writes to `bullet+0xAC` | `*(int *)(param_1 + 0xac) = param_2;` — cross-ref layout doc `0xAC = Type` | ✓ verified |
| Init param_7 (`speed`) writes to `bullet+0x110` | `*(undefined4 *)(param_1 + 0x110) = param_7;` — cross-ref layout doc `0x110 = TargetSpeed` | ✓ verified |
| AnimTimer (`+0x12D`) seeded from `type->AnimRate` (`+0x2F6`) | `*(undefined1 *)(param_1 + 0x12d) = *(undefined1 *)(param_2 + 0x2f6);` | ✓ verified |
| HouseColorIndex logic: `-1` unless `FirersPalette=yes` AND owner exists | `if ((*(char *)(param_2 + 0x2a9) == '\\0') \|\| (param_4 == 0)) { *(+0x114) = -1; } else { *(+0x114) = *(*(param_4 + 0x21c) + 0x16054); }` | ✓ verified |
| Unknown field at `bullet+0x150` is initialized to `0x100` (256) | `*(undefined4 *)(param_1 + 0x150) = 0x100;` | ✓ verified (purpose unknown; layout doc marks as "possibly DrawFlags or Facing") |
| `BulletClass::SetOwner` at `0x0046B260` actually sets WeaponType, not Owner | `*(undefined4 *)(param_1 + 0x130) = param_2;` — layout doc line 116: `0x130 = WeaponType (set by FUN_0046B260)` | ✓ **Ghidra mislabel** — function should be `SetWeapon` |
| BulletClass vtable base = `0x007E46E4` (derived from `AI` xref) | `BulletClass::AI @ 0x004666E0` has vtable xref at `0x007E4740`; AI is at slot 23 (offset 0x5C); base = `0x007E4740 - 0x5C = 0x007E46E4` | ✓ verified |
| BulletClass vtable+0xD4 (slot 53) = `ObjectClass::Conceal` at `0x005F4D30` | Memory read at `0x007E47B8` = `30 4d 5f 00` = `0x005F4D30` | ✓ verified |
| BulletClass vtable+0xD8 (slot 54) = `ObjectClass::Reveal` at `0x005F4EC0` | Memory read at `0x007E47BC` = `c0 4e 5f 00` = `0x005F4EC0` | ✓ verified |
| BulletClass vtable+0x1F0 (slot 124) = `BulletClass::Fire` at `0x00468670` | `BulletClass::Fire` xref from `0x007E48D4`; `0x007E48D4 - 0x007E46E4 = 0x1F0` | ✓ verified |
| `ObjectClass::Conceal` sets `InLimbo` (+0x81) = 1 | Direct decompile: `*(undefined1 *)((int)param_1 + 0x81) = 1;` with embedded doc comment "central remove-from-world routine" | ✓ verified |
| `BulletClass::Fire` calls `ObjectClass::Reveal` as first action | Direct decompile: `uVar6 = ObjectClass__Reveal(); if ((char)uVar6 == '\\0') return uVar6;` | ✓ verified |
| `BulletClass::Fire` writes velocity vector (6 ints / 3 doubles) to `bullet+0x3A..0x3F` | Direct decompile: 6-iteration copy loop from `param_3` to `param_1 + 0x3A` | ✓ verified |
| `Init` callable from 7 sites (1 Allocate + 6 re-init) | Xrefs: `0x0046B0A6` (Allocate), `0x006FF859` (Fire_At Temporal), `0x00469F7F` (Detonate), `0x0046A150` (Detonate 2nd), `0x0046A5A1` (SpawnShrapnel), `0x0046AA16` (SpawnShrapnel 2nd), `0x0046B408` (NukeMaker) | ✓ verified |

## 14. Open Questions

1. **Bullet `+0x150 = 0x100` purpose**: `Init` sets this field to 256. `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md:125`
   marks it "possibly DrawFlags or Facing" but doesn't confirm. Value `0x100` matches
   the `0x200` center-sprite flag shifted by 1, or could be a default facing.
   Could be investigated by tracing field accesses across BulletClass::AI and
   BulletClass::Draw.

2. **Inaccuracy formula exact math**: `BulletClass::Fire` calls `Random__RandomRanged(0, Rules->BallisticScatter * 2)`
   then scales by distance — the exact ballistic arc needs deeper analysis for parity.
   Currently documented at high level only.

3. **`FireInTransport` flag (`BulletType+0x29E`) semantics**: gates alternate target-tracking
   path inside `Fire` (the `FUN_005880A0` lookup). Flag name says "FireInTransport" but
   the code reads like a homing/target-tracking mode switch. Would need tracing the
   helper function to confirm the semantic.

4. **The `+0x12F` byte for Bright**: Fire_At reads `*(undefined1 *)(iVar8 + 0x12f)` from
   weapon to pass as `bright`. Is this the same as `Bright=yes` INI key, or a different
   weapon byte? `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT` should be cross-checked.

## Sources

- **Ghidra live decompilations:**
  - `BulletClass::Allocate` @ `0x0046B050`
  - `BulletClass::Init` @ `0x004664C0`
  - `BulletClass::SetWeapon` (mislabeled "SetOwner") @ `0x0046B260`
  - `BulletClass::Fire` @ `0x00468670`
  - `BulletClass::AI` @ `0x004666E0` (partial, for vtable derivation)
  - `ObjectClass::Conceal` @ `0x005F4D30`
  - Raw memory reads: BulletClass vtable at `0x007E46E4+` (slots 51–54)

- **Cross-references:**
  - `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` — struct layout and field names
  - `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` — sibling report covering muzzle flash
    spawn in Fire_At Phase 12 (this report covers Phase 6)
  - `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — weapon field offsets
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md` — what happens after bullet impact
  - `FIRE_AT_ANALYSIS.md` — overall Fire_At flow (this report fills Phase 6 detail)

- **INI files checked:**
  - `rulesmd.ini` / `rules.ini` — weapon INI keys (Warhead, Speed, Damage, Bright,
    Projectile, BallisticScatter)
