# Mind Control

This doc is the canonical reference for **mind control** in gamemd.exe.

There are **two distinct MC mechanisms** in the binary — one reversible, one not:

1. **CaptureManager-based MC** — Reversible. Standard Yuri Clone / Yuri Prime / Psychic Tower / Mastermind. Uses a per-controller `CaptureManagerClass` that tracks victims via an MCNode linked list. When the controller dies, the FreeAll path restores all victims to their original houses.
2. **Psychic Dominator permanent MC** — Irreversible. The Psychic Dominator superweapon's area effect transfers ownership outright (no controller pointer stored, no node tracking), sets the `PermanentlyMindControlled` flag, and uses a separate ring anim type. Once flagged, the unit cannot be freed by any normal release path.

Out-of-scope:
- The Psychic Dominator superweapon launch / target-pick → [`../../PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md`](../../PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md)
- The warhead dispatch parent → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)
- The damage transform itself → [`damage_formula.md`](damage_formula.md)
- AI fate decision (DecideUnitFate) probability tables → [`../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md) §5.8 (AI domain, not combat-math)

---

## 1. Mutually-exclusive warhead-special priority

In `WarheadTypeClass::Detonate` at `0x004690B0`, the special-warhead effects are checked as an if-else cascade. Only ONE fires per detonation. This is critical context for understanding why a MindControl=yes warhead with also Temporal=yes would only do mind control:

| Priority | WH offset | INI key | Description |
|---:|---|---|---|
| 1 | `+0x155` | `MindControl=` | Mind control capture |
| 2 | `+0x157` | `IvanBomb=` | Ivan bomb attachment |
| 3 | `+0x158` | `ElectricAssault=` | Electric assault |
| 4 | `+0x159` | `Temporal=` | Chrono warp / erase |
| 5 | `+0x15A` | `Parasite=` | Parasite attachment |
| 6 | `+0x15B` | (unknown special) | Another special WH |
| 7 | `+0x16C` | `IsLocomotor=` | Magnetron / locomotor override |
| 8 | `+0x14F` | (tractor beam, inf only) | TS-legacy tractor beam |
| 9 | `+0x16E` | `BombDisarm=` | Disarm bombs |
| 10 | `+0x175` | `MakesDisguise=` | Force disguise |
| 11 | `+0x176` | `NukeMaker=` | Spawn nuke |
| — | (default) | — | `Apply_area_damage()` normal path |

### Confidence

- **Content: HIGH** — extracted from existing canonical doc with explicit address citations; the if-else cascade structure at `0x004690B0..` is consistent with the warhead Detonate decompile.
- **Identity: HIGH** — string xrefs verified for `MindControl` (`0x0081BBC8` → `WarheadTypeClass::ReadINI 0x0075D7CF`) and `InfiniteMindControl` (`0x0084948C` → `WeaponTypeClass::ReadINI 0x00772218`).
- **Binding: HIGH** — single dispatch site in Detonate; one early-cascade entry per flag.

---

## 2. Field layout (verified)

### WarheadTypeClass

| Offset | INI key | Effect |
|---|---|---|
| `wh+0x155` | `MindControl=` | Enables MC warhead path in Detonate |

String at `0x0081BBC8`. Read at `WarheadTypeClass::ReadINI 0x0075D7CF`. Stored to `ESI + 0x155`.

### WeaponTypeClass

| Offset | INI key | Effect |
|---|---|---|
| `weapon+0x140` | `InfiniteMindControl=` | Unlimited capacity + Mastermind overload damage |
| `weapon+0xA4` | `Damage=` | Used as max-control count for the CaptureManager |

String `"InfiniteMindControl"` at `0x0084948C`. Read at `WeaponTypeClass::ReadINI 0x00772218`.

**Damage-as-max-control note:** When `InfiniteMindControl=no`, the weapon's `Damage` value is the maximum number of simultaneous victims this controller may hold. **For pip-display purposes only** the value is also used when InfiniteMindControl=yes; the actual capacity becomes unlimited but the overload-tier check is on `nodes_count` not on the Damage value.

### TechnoTypeClass

| Offset | INI key | Effect |
|---|---|---|
| `type+0xD35` | `ImmuneToPsionics=` | Unit cannot be mind-controlled at all |
| `type+0xD6A` | (separate immunity flag) | Immune to Psychic Dominator specifically |
| `type+0x60C` | `MindControlRingOffset=` | Z-offset (leptons) for the MC ring anim on victim |
| `type+0x3DC` | `LeptonMindControlOffset=` | Z-offset for the MC link-line endpoint |
| `type+0x5B0` | `MindClearedSound=` | Per-type sound on MC release (`-1` = use global) |

### TechnoClass instance fields

| Offset | Type | Field | Role |
|---|---|---|---|
| `+0x2BC` | `CaptureManagerClass*` | `CaptureManager` | Pointer to MC manager. Non-NULL on units whose primary warhead has `MindControl=yes` |
| `+0x2C0` | `TechnoClass*` | `MindControlledBy` | On a VICTIM: pointer to its controller. NULL = not MC'd |
| `+0x2C4` | `bool` | `PermanentlyMindControlled` | On a VICTIM: set ONLY by Psychic Dominator. Irreversible. No controller pointer in this case |
| `+0x2C8` | `AnimClass*` | `MindControlAnim` | The MC ring anim on the victim |
| `+0x2CC` | `int` | IronCurtain/ForceShield timer (countdown) | Blocks MC and damage while > 0 |
| `+0x2E4` | (state) | Warping/limbo state | Used by CanCapture's infantry-warping check |

### Critical correction

`+0x2C4` is **NOT** a general "IsMindControlled" flag. It is specifically the Psychic
Dominator permanent flag. Regular MC sets `+0x2C0` (the controller pointer), not `+0x2C4`.

The unified predicate is `TechnoClass::IsMindControlled` at `0x007105E0`:

```c
bool IsMindControlled(TechnoClass* this) {
    return (*(int*)(this + 0x2C0) != 0)        // CaptureManager link
        || (*(char*)(this + 0x2C4) != 0);      // Psychic Dominator flag
}
```

### Confidence (field layout)

- **Content: HIGH** — every offset cross-verified against existing canonical docs + live xrefs.
- **Identity: HIGH** — INI strings have single xrefs to their parsers.
- **Binding: HIGH** — multiple verified consumer call sites for each field.

---

## 3. `CaptureManagerClass` struct (verified)

**Size: 0x50 (80 bytes).** Class ID: `0x42`. Allocated in `TechnoClass::Init_Managers` at `0x006F3F40` when the unit's primary weapon's warhead has `MindControl=yes`.

| Offset | Type | Field | Notes |
|---|---|---|---|
| `0x00-0x0F` | ptr × 4 | vtables (4 secondary tables for COM) | `vtable__CaptureManagerClass @ 0x007E4B40` |
| `0x10-0x23` | — | AbstractClass base fields | Inherited |
| `0x24` | ptr | DynVector vtable | `PTR_FUN_007E4BA4` |
| `0x28` | ptr | `nodes_data` | array of `MCNode*` |
| `0x2C` | int | `nodes_capacity` | DV allocated capacity |
| `0x30` | bool | `nodes_is_valid` | DV valid flag |
| `0x31` | bool | (unknown flag) | Initialized to 0 |
| `0x34` | int | `nodes_count` | Current number of controlled units |
| `0x38` | int | `nodes_grow_step` | Growth increment (default: 10) |
| `0x3C` | int | `max_control` | Max simultaneous victims (from weapon `Damage=`) |
| `0x40` | bool | `infinite_mind_control` | From weapon's `InfiniteMindControl=` |
| `0x41` | bool | `overload_spark_active` | Whether overload sparks are currently playing |
| `0x44` | int | `overload_spark_delay` | Cooldown counter for spark visual effects |
| `0x48` | ptr | `owner` | Pointer to owning TechnoClass (the controller) |
| `0x4C` | int | `overload_tick_timer` | Countdown for next overload damage tick |

### MCNode (per-victim link record)

**Size: 0x14 (20 bytes).** Allocated via `operator_new(0x14)` in CaptureUnit.

| Offset | Field |
|---|---|
| `0x00` | `victim` (TechnoClass*) |
| `0x04` | `original_owner` (HouseClass*) |
| `0x08` | `capture_frame` (int) — frame # when captured; `-1` = permanent visible link |
| `0x0C` | (reserved — set from an uninitialized register) |
| `0x10` | `link_visible_frames` (from `RulesClass.MindControlAttackLineFrames`) |

### Confidence

- **Content: HIGH** — fully verified via Constructor + member function decompiles in existing canonical doc; CanCapture decompile (read live 2026-05-17) confirms `+0x34` (nodes_count), `+0x3C` (max_control), `+0x40` (infinite_mind_control), `+0x48` (owner).
- **Identity: HIGH** — class ID `0x42` matches `GetClassID @ 0x004729B0`.
- **Binding: HIGH** — single allocator (`Init_Managers`); single destructor.

---

## 4. RulesClass MC-related constants

| Section | INI key | Rules offset | Type |
|---|---|---|---|
| `[AudioVisual]` | `YuriMindControlSound=` | `+0x214` | VocIndex (int) |
| `[AudioVisual]` | `MindClearedSound=` | `+0x264` | VocIndex |
| `[AudioVisual]` | `MasterMindOverloadDeathSound=` | `+0x258` | VocIndex |
| `[CombatDamage]` | `MindControlAttackLineFrames=` | `+0x310` | int (frames) |
| `[CombatDamage]` | `ControlledAnimationType=` | `+0x320` | AnimType* |
| `[CombatDamage]` | `PermaControlledAnimationType=` | `+0x324` | AnimType* (Psychic Dominator) |
| `[CombatDamage]` | `OverloadCount=` | `+0xEEC` (DynVector header) | int[] |
| `[CombatDamage]` | `OverloadDamage=` | `+0xF08` | int[] |
| `[CombatDamage]` | `OverloadFrames=` | `+0xF24` | int[] |

### DynVector layout note

Each `DynamicVectorClass<int>` is 0x1C bytes. For each of the three Overload* fields:
- header at `Rules+offset`
- data ptr at `Rules+offset+4`
- count at `Rules+offset+8`
- capacity at `Rules+offset+0xC`

### Quoted defaults (verify from current rulesmd.ini)

Per existing canonical doc:
```
OverloadCount=3,6,10,50
OverloadDamage=0,50,100,500
OverloadFrames=30,60,60,60
```

---

## 5. `CanCapture` — the gate (verified live)

`CaptureManagerClass::CanCapture` at `0x00471C90`, decompiled live 2026-05-17. Returns
true if ALL conditions are met:

```c
bool CanCapture(this, target):
    if target == NULL: return false

    controller_owner = this->owner->House (via vtable+0x3C)   // captureManager+0x48 → House
    target_owner = target->GetHouse() (vtable+0x3C)
    if target_owner == controller_owner: return false        // can't MC own units

    target_type = target->GetTechnoType()                     // vtable+0x84
    if target_type.byte+0xD35 != 0: return false              // ImmuneToPsionics

    if target.byte+0x2E4 != 0 && target.WhatAmI() == 1:       // infantry being warped
        return false

    if TechnoClass::IsMindControlled(target): return false    // already MC'd (either flag)

    if target.byte+0x2CC != 0: return false                   // IC / ForceShield timer

    if target.vtable+0x160() != 0: return false               // generic immunity (drained etc.)

    // Capacity check
    if !infinite_mind_control:
        if nodes_count >= max_control && max_control != 1:
            return false                                       // at capacity, not override mode
    // (else: infinite OR room available OR override mode (max_control==1))

    if target.Mission == 0x13 || target.Mission == 0x12:      // Selling-related blocked states
        return false

    return true
```

### Key behavioral consequences

- **Cannot MC an already-MC'd unit.** Re-MC requires the first controller to die first. The Psychic Dominator has a special bypass (see §8).
- **`max_control == 1` is "override mode":** capturing a new victim FREES the previous one. Used by Yuri Clones (Damage=1, so max_control=1).
- **`max_control > 1` at capacity:** capture silently fails.
- **`InfiniteMindControl=yes`:** capacity check is skipped; node DynVector grows by step 10 as needed. The Mastermind/Yuri Prime fall here.

### Confidence (CanCapture)

- **Content: HIGH** — decomp read 2026-05-17 matches existing doc point-for-point.
- **Identity: HIGH** — named function in Ghidra annotation set.
- **Binding: HIGH** — called from `CaptureUnit` (always) and `GetFireError` Phase W gate #58 (via `CaptureManagerClass::CanCapture` direct call).

---

## 6. `CaptureUnit` — the capture (verified)

`CaptureManagerClass::CaptureUnit` at `0x00471D40`. Flow:

```
1. validate (NULL + AbstractFlags)
2. CanCapture(target) → return false if denied
3. If max_control == 1 (override mode): iterate existing nodes, FreeUnit each
4. previous_owner = target.GetHouse()
5. target.SetOwner(controller.House)                      // vtable+0x3D4 — OWNERSHIP TRANSFER
6. allocate MCNode (0x14 bytes):
     node.victim          = target
     node.original_owner  = previous_owner
     node.capture_frame   = g_CurrentFrameCounter
     node.link_visible_frames = Rules.MindControlAttackLineFrames
7. append node to DynVector at captureManager.nodes_data
8. target.MindControlledBy = controller     // target+0x2C0 = captureManager.owner (+0x48)
9. (some mission filtering for buildings — skip scatter for missions 0x10/0x12/0x13)
10. target.Scatter()                          // vtable+0x3D0
11. captureManager.DecideUnitFate(target)
12. create anim from Rules.ControlledAnimationType (+0x320)
13. attach anim to victim, store at victim+0x2C8
14. if victim is Building: set anim Z-offset to -1024 leptons (-0x400)
15. return true
```

### Confidence

- **Content: HIGH** — fully decomp-verified in existing canonical doc.
- **Identity: HIGH** — named function, single named caller (the warhead Detonate MC branch).
- **Binding: HIGH** — invoked from Detonate's MindControl branch at `0x004692D0`.

---

## 7. Release paths (FreeUnit / FreeAll / Cancel)

### `FreeUnit(victim)` at `0x00471FF0` — release a single victim

```
1. iterate nodes, find matching victim
2. remove MC ring anim (anim.Remove(); victim.MindControlAnim = NULL)
3. play "mind cleared" sound:
     if (victim.Type.MindClearedSound != -1): use that VocIndex
     else: use Rules.MindClearedSound (+0x264)
4. victim.SetOwner(node.original_owner, redraw=true)        // restore ownership
5. captureManager.DecideUnitFate(victim)                     // AI re-decide
6. victim.MindControlledBy = NULL                            // clear +0x2C0
7. free MCNode
8. shift DynVector entries
```

### `FreeAll()` at `0x00472140`

Reverse-iteration loop: `for i from count-1 down to 0: FreeUnit(nodes[i].victim)`.

### When is `FreeAll()` called?

Verified call sites (from existing canonical doc):

| Caller | Location | Trigger |
|---|---|---|
| `TechnoClass::ReceiveDamage` | `0x00702112` | Controller killed |
| `BuildingClass::ReceiveDamage` | `0x004424F9` | Controller building destroyed |
| `BuildingClass::EnterTransport` | `0x0070FDBD` | Controller enters a transport |
| `TemporalClass::InitiateWarp` | `0x0071AF48` | Controller chronoshifted / temporal-warped |
| `BuildingClass::UpdateGapAndSpecialEffects` | `0x00454B47` | (gap/effects update — needs trace) |
| `FUN_004DE5D0` | `0x004DE5DD` | unit removal |

So MC is released whenever:
- The controller dies (damage path).
- The controller enters a transport.
- The controller is chronoshifted out.
- The unit is removed via the generic removal path.

### `FreeUnit` direct callers (single-victim release)

| Caller | Location | Trigger |
|---|---|---|
| `InfantryClass::Mission_Enter` | `0x0051A2DA`, `0x0051A438` | A specific MC'd infantry enters a transport |
| `UnitClass::Mission_Enter` | `0x0073A2CD`, `0x0073A72B` | A specific MC'd vehicle enters a transport |
| `PsychicDominator::MindControlArea` | `0x0053B080` | Permanent-MC capture frees the existing CaptureManager link first |

---

## 8. Mastermind overload damage (`InfiniteMindControl`)

`CaptureManagerClass::Update` at `0x00471A50`. Called per-tick from `TechnoClass::AI_Update`. **Only active when `infinite_mind_control` (+0x40) is true.**

```
each tick:
    --captureManager.overload_spark_delay      (+0x44)
    --captureManager.overload_tick_timer       (+0x4C)

    if overload_tick_timer reaches 0:
        find tier T s.t. nodes_count <= Rules.OverloadCount[T]
        damage    = Rules.OverloadDamage[T]
        interval  = Rules.OverloadFrames[T]
        overload_tick_timer = interval

        if damage > 0:
            overload_spark_delay = 10
            controller.ReceiveDamage(damage, 0, Rules.C4Warhead (+0xFA8), ...)   // ★ attacks SELF
            if first spark: play MasterMindOverloadDeathSound

        spawn 5 spark particles at controller.location
        optionally apply heading wobble (±0.015 or ±0.03 rad)
```

### With default INI

| Victim count | Damage/tick | Tick interval |
|---:|---:|---:|
| 0–2 | 0 | 30 frames |
| 3–5 | 0 | 30 frames |
| 6–9 | 50 | 60 frames |
| 10–49 | 100 | 60 frames |
| 50+ | 500 | 60 frames |

So a Mastermind controlling 7 units takes 50 damage every 60 frames (= 4 seconds at
~15 FPS sim) — **damage is applied to the controller itself, not to the victims**. This
is the "Mastermind starts dying when over-controlled" effect.

The damage warhead is `Rules+0xFA8` = `Rules.C4Warhead` (same warhead used for IC-barrel
chain — see [`chain_reaction.md`](chain_reaction.md) §5 and [`splash_cellspread.md`](splash_cellspread.md) §11).

### Confidence

- **Content: HIGH** — Update decomp in existing canonical doc; thresholds match `[CombatDamage]` defaults.
- **Identity: HIGH** — named function; `Rules+0xEEC/+0xF08/+0xF24` DynVector arrays.
- **Binding: HIGH** — single caller (`TechnoClass::AI_Update` per-tick).

---

## 9. Psychic Dominator permanent MC

`PsychicDominator::MindControlArea` at `0x0053B080`. Separate code path from CaptureManager.

### Flow

For each unit in the dominator's target area:

```
1. if unit.WhatAmI() == 6 (Building): skip
2. if unit.Type.byte+0xD35 (ImmuneToPsionics) != 0: skip
3. if unit.vtable+0x160() != 0: skip                            // generic immunity
4. if unit.Type.byte+0xD6A != 0 (ImmuneToPsychicDominator?): skip
5. if !unit.vtable+0x54() (IsAlive/valid): skip
6. if unit.MindControlledBy (+0x2C0) != 0:
       CaptureManagerClass::FreeUnit(existing_controller, unit)   // release first
7. unit.SetOwner(dominator_house)                                 // vtable+0x3D4
8. unit.PermanentlyMindControlled = 1                             // unit+0x2C4
9. anim = new AnimClass(Rules.PermaControlledAnimationType (+0x324))
10. anim Z-offset = unit.Type.MindControlRingOffset (+0x60C)
11. unit.MindControlAnim = anim                                   // unit+0x2C8
```

### Differences vs CaptureManager MC

| Aspect | CaptureManager MC | Psychic Dominator |
|---|---|---|
| Reversible | Yes — FreeAll on controller death | NO |
| Controller pointer | Stored on victim (`+0x2C0`) | NOT stored |
| MCNode tracking | Yes | No |
| Anim type | `Rules.ControlledAnimationType` | `Rules.PermaControlledAnimationType` |
| Targets buildings | Yes | NO (Step 1 skips them) |
| Pre-empts existing MC | No (CanCapture rejects) | YES (Step 6 explicitly frees) |
| Restoration on owner death | (N/A) | Permanent — owner cannot lose them via death |

### Confidence

- **Content: HIGH** — function fully decompiled in existing canonical doc.
- **Identity: HIGH** — sole consumer of `+0x2C4` for writes.
- **Binding: HIGH** — single call site (superweapon launch).

---

## 10. Interaction rules

| Scenario | Behavior |
|---|---|
| MC an already-MC'd unit (regular MC) | **Blocked** by CanCapture check 5 (`IsMindControlled` returns true). Silent fail. |
| MC a unit that's mind-controlling others | **Allowed**. The captured controller keeps its CaptureManager and its victims, but ownership transfers. Victims now functionally belong to the new owner via the chain. |
| MC a unit under Iron Curtain | **Blocked** by CanCapture check 6 (`+0x2CC` timer != 0). |
| MC a unit under Force Shield | **Blocked** — same timer check. |
| MC a unit being temporally erased | **Blocked** by CanCapture check 4. Additionally, when the controller is temporally erased, `TemporalClass::InitiateWarp` calls `FreeAll` on all victims. |
| Mass MC (Yuri Prime mass-control range) | Iterates targets in radius, calls CaptureUnit for each. Each subject to CanCapture. |
| Psychic Dominator on an MC'd unit | **Allowed** — explicit FreeUnit call (Step 6) releases the old controller, then permanent flag set. |
| Re-MC a freed unit | **Allowed** — after FreeUnit, `+0x2C0` is cleared and `+0x2C4` is unset. |
| MC mission 0x12 / 0x13 (selling-related) | **Blocked** by CanCapture check 9. |
| Mastermind over-control | **Allowed** with damage. Capacity check passes (InfiniteMindControl); Update ticks apply self-damage per tier. |

---

## 11. Key offsets summary

| Symbol | Offset / Address |
|---|---|
| `wh.MindControl` | `+0x155` |
| `weapon.InfiniteMindControl` | `+0x140` |
| `weapon.Damage` (as max-control) | `+0xA4` |
| `type.ImmuneToPsionics` | `+0xD35` |
| `type.ImmuneToPsychicDominator (probable)` | `+0xD6A` |
| `type.MindControlRingOffset` | `+0x60C` |
| `type.LeptonMindControlOffset` | `+0x3DC` |
| `type.MindClearedSound` | `+0x5B0` |
| `techno.CaptureManager` | `+0x2BC` |
| `techno.MindControlledBy` | `+0x2C0` |
| `techno.PermanentlyMindControlled` | `+0x2C4` |
| `techno.MindControlAnim` | `+0x2C8` |
| `techno.IronCurtainTimer` | `+0x2CC` |
| `captureMgr.max_control` | `+0x3C` |
| `captureMgr.infinite_mind_control` | `+0x40` |
| `captureMgr.owner` | `+0x48` |
| `Rules.YuriMindControlSound` | `+0x214` |
| `Rules.MindClearedSound` | `+0x264` |
| `Rules.MasterMindOverloadDeathSound` | `+0x258` |
| `Rules.MindControlAttackLineFrames` | `+0x310` |
| `Rules.ControlledAnimationType` | `+0x320` |
| `Rules.PermaControlledAnimationType` | `+0x324` |
| `Rules.OverloadCount` | `+0xEEC` (DynVector header) |
| `Rules.OverloadDamage` | `+0xF08` |
| `Rules.OverloadFrames` | `+0xF24` |
| `Rules.C4Warhead` (for overload self-damage) | `+0xFA8` |
| `CaptureManagerClass::Constructor` | `0x004717D0` |
| `CaptureManagerClass::Update` | `0x00471A50` |
| `CaptureManagerClass::CanCapture` | `0x00471C90` |
| `CaptureManagerClass::CaptureUnit` | `0x00471D40` |
| `CaptureManagerClass::FreeUnit` | `0x00471FF0` |
| `CaptureManagerClass::FreeAll` | `0x00472140` |
| `CaptureManagerClass::DrawLinks` | `0x00472160` |
| `CaptureManagerClass::DecideUnitFate` | `0x004723B0` |
| `TechnoClass::Init_Managers` | `0x006F3F40` |
| `TechnoClass::IsMindControlled` | `0x007105E0` |
| `TechnoClass::FreeAllMindControlCaptures` | `0x00710460` |
| `WarheadTypeClass::Detonate MC branch` | `0x00469211` (within `0x004690B0`) |
| `PsychicDominator::MindControlArea` | `0x0053B080` |

---

## 12. TS-legacy filter

- **All MC mechanisms are LIVE in YR.** Yuri faction units (Yuri Clone, Yuri Prime, Mastermind, Psi-Corps Trooper, Brute) and Allied Psychic Tower exercise the CaptureManager path. The Psychic Dominator (Yuri SW) is the only consumer of the permanent-MC path. Mind-Controlled Pet (campaign-specific NPC dog?) and a few civilians can also use MC weapons.
- **`+0x14F` "tractor beam" warhead flag** (priority 8 in §1) is **TS-legacy** — no YR retail warhead sets it.
- **`DecideUnitFate` probability tables** are inherited from TS. The function IS called in YR but the probability constants may be inappropriate; flag for AI-system audit (out of scope for this doc).
- **Overload damage** is YR-original (used by Mastermind).

---

## 13. Edge cases

| Case | Behavior |
|---|---|
| Yuri Clone has `Damage=1, InfiniteMindControl=no` | `max_control=1`, override mode — each new capture frees the previous |
| Mastermind has `Damage=3, InfiniteMindControl=yes` | Unlimited capacity; the `Damage=3` is for the pip display only |
| Psi-Corps Trooper has `Damage=N` per shipping rulesmd.ini | `max_control=N`; capture fails when at capacity (no override) |
| Controller dies while engaged with 9 victims | FreeAll iterates 9 times, releases each; each gets DecideUnitFate AI re-roll |
| Victim is sold mid-MC | Mission 0x13 triggers; capture fails to RE-MC. Already-captured victim: the SetOwner restored owner becomes the controller's owner, then mission 0x13 proceeds normally |
| Multiple MC'd victims of one Mastermind, then Mastermind dies | All revert via FreeAll. AI re-roll determines fate |
| Victim of regular MC then Psychic Dominator-targeted | Step 6 explicitly frees the existing controller, then permanent flag set |
| Friendly MC of own units | CanCapture check 2 fails (owners equal). Always blocked |
| Building targeted by regular MC | Allowed (no check explicitly skips buildings). Building Z-offset for anim is -1024 leptons |
| Building targeted by Psychic Dominator | **Skipped** (Step 1) — buildings are immune to permanent MC. |
| Mind-controlled unit attacks its original ally | The unit's owner is now the controller's house. Standard AffectsAllies rules apply per [`friendly_fire.md`](friendly_fire.md). |

---

## 14. Open follow-ups

1. **`weapon+0x140` vs `+0x141`/+0x142` ordering.** The InfiniteMindControl flag is at +0x140, but `can_target_gates.md` lists weapon flags clustered at 0x14X with somewhat different semantics. Cross-verify offsets against [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md). Priority: LOW.
2. **`type+0xD6A` ImmuneToPsychicDominator identity.** The existing doc names it but doesn't trace the INI key string. Likely `ImmuneToPsychicDominator=` or similar. Priority: MEDIUM — needed for parity if any retail unit sets it.
3. **`UpdateGapAndSpecialEffects FreeAll caller`** identity — what triggers it? Could be a Gap Generator transitioning a unit between visible/invisible states. Priority: LOW.
4. **`captureManager+0x4C` overload_tick_timer initialization.** Constructor sets this to what value? If 0, the first tick triggers immediately; if a high value, there's a startup delay. Trace Constructor body. Priority: LOW.
5. **`overload_spark_delay`** purpose — is it visual-only or does it also gate the damage tick? Audit Update. Priority: LOW.
6. **`type+0x60C` and `+0x3DC` Z-offset semantics.** Both are MC-anim Z-offsets but used in different contexts (ring vs link line). Confirm in animation-render code. Priority: LOW.
7. **`Rules.MindControlAttackLineFrames` default value** — quote from rulesmd.ini. Priority: LOW.
8. **AI DecideUnitFate**: probability tables may be TS-vestigial. Confirm in YR test if outcome distribution matches design intent. Priority: LOW (AI domain).

---

## 15. Sources

- Live decompilation of `CaptureManagerClass::CanCapture` at `0x00471C90` (2026-05-17) — confirmed offsets `+0xD35`, `+0x2CC`, `+0x2E4`, `+0x34`, `+0x3C`, `+0x40`, `+0x48`.
- Live xrefs (2026-05-17):
  - `"MindControl"` at `0x0081BBC8` → `WarheadTypeClass::ReadINI 0x0075D7CF`
  - `"InfiniteMindControl"` at `0x0084948C` → `WeaponTypeClass::ReadINI 0x00772218`
- Existing canonical doc: [`../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md) (615 lines, 2026-04-21) — primary source for CaptureManagerClass / MCNode struct, function flow, overload system, Psychic Dominator separation. **High-quality doc.** This systems doc supersedes for the combat-systems index; the original retains historic value for the full per-function decomp record.
- Existing canonical doc: [`../../MIND_CONTROL_GHIDRA_REPORT.md`](../../MIND_CONTROL_GHIDRA_REPORT.md) (451 lines, earlier) — superseded by the SYSTEM version.
- WarheadTypeClass struct: [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md).
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`splash_cellspread.md`](splash_cellspread.md), [`can_target_gates.md`](can_target_gates.md) (gate #58), [`anti_air_dispatch.md`](anti_air_dispatch.md) (warhead.MindControl is Phase H in SelectWeaponAgainst), [`friendly_fire.md`](friendly_fire.md), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md), [`chain_reaction.md`](chain_reaction.md).
