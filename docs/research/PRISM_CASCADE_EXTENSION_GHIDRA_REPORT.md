# Prism Tower Cascade — Extension Report (Iteration 4)

**Extends** `PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` and `PRISM_FORWARDING_GHIDRA_REPORT.md`.
This iteration closes every residual gap and corrects two material errors in the prior
report. Read this report together with the earlier two; the core narrative (selector
loop, mode machine, damage scaling) is still owned by the trigger report.

**Confidence:** HIGH for Sections 1–9 and 14–15 (all verified from binary disassembly
and decompilation this pass). MEDIUM for Sections 10–13 (interaction behavior —
gated by flags/fields whose exact runtime semantics were cross-checked but not
pinpoint-decompiled per case).

**Active in YR:** YES, unconditionally for ATESLA/prism path. Individual corner cases
(EMP, MC, IC, low power) are flagged in their sections.

---

## 0. Corrections to prior reports

Two material errors were found in `PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` when
tracing the selector's distance math at `0x0044b421–0x0044b49c`. Both would have
produced broken implementations.

### 0.1 Range comparison is on the FIRING TOWER, not the candidate

The selector at `0x0044b48c-0x0044b494` reads:

```
0044b48c: 8b 16               MOV EDX, [ESI]        ; ESI = firing tower vtable
0044b48e: 6a 01               PUSH 1                ; weapon index
0044b490: 8b ce               MOV ECX, ESI          ; ECX = firing tower (this)
0044b492: 8b e8               MOV EBP, EAX          ; save distance
0044b494: ff 92 68 01 00 00   CALL [EDX + 0x168]    ; firing_tower.GetWeaponRange(1)
0044b49a: 3b e8               CMP EBP, EAX          ; distance vs range
0044b49c: 7f 10               JG  skip
```

`MOV ECX, ESI` before the CALL confirms the target is the **firing tower's** vtable, not
the candidate's. The existing doc's claim that this was `candidate->Weapon_Range(1)`
is **refuted**.

**Implication:** every candidate is tested against the *firing tower's*
PrismSupport range (weapon index 1). This is a single threshold for the whole
selector loop, not a per-candidate threshold.

### 0.2 Distance is LINEAR (leptons), not squared (leptons²)

The FILD/FMUL/FADDP chain at `0x0044b421–0x0044b47a` computes `dx² + dy² + dz²` on the
FPU stack, then the next three instructions are:

```
0044b47f: e8 bc f7 07 00      CALL Math::Sqrt_Approx (0x004cac40)
0044b484: 83 c4 08            ADD ESP, 8
0044b487: e8 74 aa 37 00      CALL Math::ftol (0x007c5f00)
```

- `Math::Sqrt_Approx @ 0x004cac40` (decompiled; the approx-sqrt routine that
  reads a double arg and returns on FPU).
- `Math::ftol @ 0x007c5f00` (decompiled; FPU-to-int32 conversion).

So EBP at `CMP EBP, EAX` holds `int(sqrt(dx²+dy²+dz²))` — the true lepton distance,
truncated to int32. And `EAX` from `GetWeaponRange(1)` returns `WeaponTypeClass+0xB4` =
`Range` in leptons. The comparison is **linear lepton distance vs linear lepton
range**. The existing doc's "squared lepton distance" was wrong.

**Implication:** any Rust implementation must take a real square root (or
an approx equivalent). Squared-space comparison produces wrong candidate
inclusion for moderately-distant supporters.

---

## 1. Range threshold — final answer (Item 1, resolves O3)

| Question | Final answer | Evidence |
|---------|-------------|---------|
| What function? | `TechnoClass::GetWeaponRange @ 0x007012C0` (vtable byte 0x168 = idx 0x5A) | Decompiled |
| What weapon index? | 1 (Secondary = `PrismSupport`) | `PUSH 1` at `0x0044b48e` |
| Returns what? | `WeaponTypeClass+0xB4 = Range` field, in **leptons** | `iVar5 = *(int *)(*piVar2 + 0xb4)` in function body |
| Called on? | The **firing tower** (ESI/this), not the candidate | `MOV ECX, ESI` at `0x0044b490` |
| Distance unit compared? | **Linear leptons** (sqrt applied before comparison) | `Sqrt_Approx` call at `0x0044b47f` |

For ATESLA the Secondary is `PrismSupport` with `Range=8` cells = 2048 leptons
(1 cell = 256 leptons). So the effective selector radius is **8 cells from the
firing tower** — matching `PrismShot` in ATESLA's case, but conceptually distinct
(the cascade radius is a Secondary-weapon property).

---

## 2. Weapon index 1 identity (Item 2)

From `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` burst state:
- `Primary` burst data at 0x3D8–0x3F4 (weapon idx 0)
- `Secondary` burst data at 0x3F8–0x414 (weapon idx 1)

On `BuildingTypeClass`, weapon slots are at:
- +0x898 = weapon[0] slot (Primary) — 28 bytes, first dword is `WeaponTypeClass*`
- +0x8B4 = weapon[1] slot (Secondary)

From `FUN_007177C0` (trivial helper): returns `TypeClass + 0x898 + idx * 0x1C`.
So weapon idx 1 → offset 0x8B4.

For `[ATESLA]` in `rulesmd.ini`:
```
Primary=PrismShot    ; Range=8 cells (2048 leptons)
Secondary=PrismSupport ; Range=8 cells (2048 leptons)
```

Both weapons happen to have `Range=8` so the numeric answer doesn't change, but
the cascade semantically uses `PrismSupport`'s range, not `PrismShot`'s.

---

## 3. FUN_00712130 — the "tertiary gate" (Item 3)

**Address:** `0x00712130`. Called at `0x0044b2d0` inside the pre-cascade immediate-
fire gate (only reached when upgrade count > 0 and upgrade slot 1 is non-null).

**Full disassembly (18 instructions):**

```
00712130: MOV EDX, [ECX + 0x898]    ; EDX = upgrade_type->weapon[0].WeaponTypeClass*
00712136: TEST EDX, EDX
00712138: JZ  0x00712161             ; no primary → return false
0071213a: MOV ECX, [ECX + 0x8b4]    ; ECX = upgrade_type->weapon[1].WeaponTypeClass*
00712140: MOV EAX, 0x1
00712145: CMP EDX, ECX
00712147: JZ  0x00712163             ; primary == secondary → return true
00712149: PUSH ESI
0071214a: MOV ESI, [EDX + 0x9c]    ; primary->Burst (WeaponTypeClass+0x9C)
00712150: CMP ESI, EAX
00712152: POP ESI
00712153: JG  0x00712163             ; primary.Burst > 1 → return true
00712155: TEST ECX, ECX
00712157: JZ  0x00712161             ; no secondary → false
00712159: CMP [ECX + 0x9c], EAX     ; secondary->Burst
0071215f: JG  0x00712163             ; secondary.Burst > 1 → return true
00712161: XOR AL, AL                 ; return false
00712163: RET
```

**Semantic:** `bool HasBurstWeaponInSlot1(BuildingTypeClass *upgrade_type)`.
Returns true iff the upgrade type has a burst-style weapon (`Burst > 1`) in
either slot, or primary == secondary (self-reference shortcut).

**Key discovery:** the incoming `this` (ECX) is the **upgrade slot's content as a
BuildingTypeClass pointer**, not a BuildingClass instance. The upgrade slots
(`+0x5E8, +0x5EC, +0x5F0`) store `BuildingTypeClass*` directly, which is why
`+0x898` (weapon[0]) dereferences cleanly.

**Active in YR:** essentially dead. The pre-cascade path requires the attacking
building to have ≥ 1 upgrade AND slot 1 filled. No stock-YR Prism Tower is
upgradeable (`ATESLA.Capturable=false` + no `PowersUpBuilding=`), so the path
is never entered.

---

## 4. Charge anim selection (Item 4)

**Location:** `0x0044b539-0x0044b5ee` (supporter path) and `0x0044b5d4-0x0044b60b`
(firing-tower mode-1 path). Identical logic, applied to different struct pointers.

**INI source:** `BuildingTypeClass::ReadINI @ 0x0045FE50`, specifically the
calls at `0x00462d63` (writes +0x1204) and `0x00462dbf` (writes +0x11F4).

| Offset (BuildingTypeClass) | INI key | Purpose |
|---------------------------|---------|---------|
| +0x11F4 | `SpecialAnim=` | Healthy charge anim (16-byte char name) |
| +0x1204 | `SpecialAnimDamaged=` | Damaged charge anim (16-byte char name) |

Both are read through `CCINIClass::ReadString` from `art*.ini` (via the art-parse
pass on BuildingTypeClass::ReadINI). For GAPRIS in `artmd.ini`:

```
[GAPRIS]
SpecialAnim=GAPRIS_B
SpecialAnimDamaged=GAPRIS_BD
```

**Selection logic** (applies to both supporter and firing-tower paths):

```c
double health = ObjectClass::GetHealthRatio(this);
FCOMP  health, Rules->ConditionYellow  (Rules+0x1700, double)
FNSTSW AX
TEST   AH, 0x41   ; mask C3 (equal) | C0 (less)
JZ     HEALTHY    ; JZ taken when BOTH bits clear ⇔ health > ConditionYellow

DAMAGED_OR_EQUAL:
    anim_ptr = Type + 0x1204  ; SpecialAnimDamaged
    variant_idx = 0
    JMP play

HEALTHY:
    anim_ptr = Type + 0x11F4  ; SpecialAnim
    variant_idx = 1

play:
    if (*anim_ptr == 0) goto tail  ; null string → skip
    call PlayChargeAnim(anim_ptr, 0x0A, variant_idx, 0, 0)
```

Threshold: `Rules->ConditionYellow` (+0x1700 on RulesClass, default 0.5). When
health ratio ≤ 0.5, the damaged variant plays.

**`PlayChargeAnim` @ `0x0044b890`** — not fully decompiled; invoked with
(anim_name_ptr, layer=0x0A, variant_idx, 0, 0). `0x0A` looks like an anim layer
index, `variant_idx` passes through the healthy/damaged discriminator.

---

## 5. Firing-tower death mid-charge (Item 5)

**Finding:** There is **no active cleanup**. `BuildingClass::Limbo @ 0x00445880`
does not touch `+0x664`, `+0x704`, `+0x708–0x710`, or `+0x714` on any supporter.
(corrected 2026-05-29: was `BuildingClass::OnDestroyed`; binary shows `BuildingClass__Limbo` via `get_function_by_address 0x00445880` + `decompile_function 0x00445880` — ROOT_CAUSE: RTTI_LABEL_DRIFT)

The supporter's mode-2 state (saved-coords + timer) is entirely self-contained —
it stores the firing tower's cell coords at the moment of selection, not a pointer.
When the supporter's timer expires, `EmitPrismSupportBeam` spawns a `LaserDrawClass`
from the supporter's location to the saved coords, regardless of whether a
building still exists there.

**Observable consequence:** if a firing Prism Tower is killed mid-charge, all
supporters it had queued up will still emit visual beams pointed at the
ex-firing-tower's former location. The beams resolve harmlessly (just visual).

The firing tower itself, when killed mid-charge, loses its `+0x664` support count
and `+0x704` mode along with all other building state (freed by allocator). No
partial shot is delivered. The accumulated "would-have-been" damage multiplier is
simply lost.

---

## 6. Target-lost mid-charge (Item 6, confirms iter-3's suspicion)

From `BuildingClass::ProcessDelayedFire @ 0x004503F0` (decompiled fully):

```c
int BuildingClass::ProcessDelayedFire(BuildingClass *this) {
    int mode = this->field_0x704;
    if (mode == 0) return 0;

    int timer_prev = this->field_0x714;
    this->field_0x714 = timer_prev - 1;
    if (timer_prev - 1 >= 1) return 0;  // still charging
    this->field_0x714 = 0;

    if (mode == 1) {
        // Firing tower — fire the shot
        int result = 0;
        if (this->field_0x2b4 != 0) {  // target non-null?
            int err = vtable[0xF0](target, this->field_0x708, 1);  // GetFireError
            if (err == 0) {
                Bullet *bullet = vtable[0xF3](target, this->field_0x708);  // Fire
                if (bullet != 0 && this->field_0x664 != 0) {
                    int pct = Rules->PrismSupportModifier * this->field_0x664 + 100;
                    bullet->field_0x150 = (pct * 0x100) / 100;
                    this->field_0x664 = 0;   // count reset ONLY here
                }
            }
        }
        // NOTE: if target was null, GetFireError != 0, OR bullet was null:
        //       field_0x664 is NOT reset.
    }
    else if (mode == 2) {
        EmitPrismSupportBeam(this, saved_x, saved_y, saved_z);
        this->field_0x704 = 0;
        return ...;
    }

    this->field_0x704 = 0;  // mode cleared in all paths (mode 1 + else)
    return ...;
}
```

**Target-lost semantics:**
- Target disappears before timer expiry → `this->field_0x2b4 == 0` at fire time →
  `Fire()` not called → `field_0x664` preserved → mode cleared.
- GetFireError returns non-zero (out of range, facing wrong, etc.) → Fire not called
  → `field_0x664` preserved → mode cleared.
- Fire returns null bullet (out of ammo, bullet allocation failure) → `field_0x664`
  preserved → mode cleared.

In all these cases, the next Mission_Attack tick can re-enter the cascade. The
cascade-tail at `0x0044b4d7` increments `field_0x664` by 1 on each successful
supporter pick. So `field_0x664` can grow to `PrismSupportMax = 8` across multiple
failed fire attempts before the cap triggers.

This **confirms iter-3 Section 13.4's analysis** that count > 1 per shot is
reachable only when fire keeps getting suppressed.

---

## 7. LaserDrawClass struct layout — complete (Item 7)

**Size:** 0x5C bytes (allocated via `operator_new(0x5c)` in all 4 callers).

**Constructor signature (fully decoded from disassembly at `0x0054FE60`):**

```c
LaserDrawClass* __thiscall LaserDrawClass::Constructor(
    LaserDrawClass *this,     // ECX
    int32 src_x,              // stack arg 1
    int32 src_y,              //        2
    int32 src_z,              //        3
    int32 tgt_x,              //        4
    int32 tgt_y,              //        5
    int32 tgt_z,              //        6
    int32 param_7,            //        7  (always 0 for prism)
    uint8 one_shot,           //        8  (byte; 1 for prism)
    uint32 inner_color_rgb,   //        9  (3 bytes used: BGR)
    uint32 outer_color_rgb,   //       10
    uint32 spread_color_rgb,  //       11
    int32 duration_ticks,     //       12  (PrismSupportDuration = 15)
    uint8 param_13,           //       13  (byte; 0 for prism)
    uint8 is_laser_effect,    //       14  (byte; 1 for prism)
    float intensity_start,    //       15  (1.0f for prism)
    float intensity_end)      //       16  (0.0f typical)
```

Callee cleans the 16×4 = 64 bytes of stack args (`RET 0x40`).

**Field map (0x5C bytes):**

| Offset | Size | Name | Constructor Init | Notes |
|--------|------|------|------------------|-------|
| 0x00 | 4 | AnimStep | 0 | Increments by `StepIncrement` each repeat |
| 0x04 | 1 | IsActive | 0 | Set by AI tick |
| 0x08 | 4 | SpawnFrame | `g_CurrentFrameCounter` | Timer base |
| 0x0C | 4 | CurrentY_or_anim | copy of src_y | Anim state |
| 0x10 | 4 | RemainingTicks | 1 | Decremented by AI tick |
| 0x14 | 4 | Flag1 | 1 | |
| 0x18 | 4 | StepIncrement | 1 | Added to AnimStep on repeat |
| **0x1C** | **4** | **InnerLineCount** | **1** | **= beam thickness (# parallel lines drawn)** |
| 0x20 | 1 | IsLaserEffect | 0 | Cleared by ctor; set to 1 by EmitPrismSupportBeam AND by Fire_At for IsLaser weapons |
| 0x21 | 1 | IsBoosted | 0 | Set to 1 by Fire_At when firing tower has supporters |
| 0x24 | 4 | SrcX | param | Source coord |
| 0x28 | 4 | SrcY | param | |
| 0x2C | 4 | SrcZ | param | |
| 0x30 | 4 | TgtX | param | Target coord |
| 0x34 | 4 | TgtY | param | |
| 0x38 | 4 | TgtZ | param | |
| 0x3C | 4 | Param7 | param | Unknown; always 0 for prism |
| 0x40 | 1 | OneShot | param | |
| 0x41-0x43 | 3 | InnerColor BGR | param (low 3 bytes) | e.g. 0xB8,0x00,0xD8 (magenta) |
| 0x44-0x46 | 3 | OuterColor BGR | param | |
| 0x47-0x49 | 3 | SpreadColor BGR | param | |
| 0x4C | 4 | DurationTotal | `duration_ticks` | Total lifetime in ticks |
| 0x50 | 1 | ToggleFlag | `param_13` | If set, AI toggles 0x51 each tick |
| 0x51 | 1 | ToggledState | 0 | Flips when 0x50 set |
| 0x52 | 1 | FadeEnable | `is_laser_effect` | When set, intensity linearly fades 0x54→0x58 |
| 0x54 | 4 | IntensityStart | param (float) | Begin fade value |
| 0x58 | 4 | IntensityEnd | param (float) | End fade value |

**Global counter + array:**
- `g_LaserDraw_Count @ 0x00ABC888` — current active count
- `g_LaserDraw_Array @ 0x00ABC87C` — array of `LaserDrawClass*`
- `g_LaserDraw_Cap @ 0x00ABC880` — max size (dynamic)
- `DynVector @ 0x00ABC878` — container vtable ptr (expand-hooks)

---

## 8. Beam type enum / renderer consumer (Item 8)

**Correction:** the prior report's claim that `field_0x1C` is a "beam type enum (1/3/5)"
is **misleading**. `field_0x1C` is literally `InnerLineCount` — the renderer draws
that many parallel lines.

**Found in `FUN_005509f0` @ `0x005509f0`** (the special laser-draw path):

```c
if (0 < param_1[7]) {   // param_1[7] = field_0x1C = InnerLineCount
    uVar13 = local_11c;  // angle octant
    do {
        // draw an offset pair of lines at this pass
        ...draw...
        if (*(char*)((int)param_1 + 0x21) == '\0' || local_e8 != 1) {
            // half the color intensity each iteration
            uVar13 = color_b >> 1;
            uVar11 = color_g >> 1;
            local_12e = local_12e >> 1;
        } else {
            // first pass of a BOOSTED beam: full (pre-doubled) color
            local_130 = byte_0x41;
            bStack_12f = byte_0x42;
            local_12e = byte_0x43;
        }
        local_e8++;
    } while (local_e8 <= param_1[7]);
}
```

So:
- `field_0x1C = 1` → 1 line drawn (plain laser)
- `field_0x1C = 3` → 3 lines drawn (standard prism beam)
- `field_0x1C = 5` → 5 lines drawn (boosted prism beam, i.e. thicker)

The `field_0x21 = 1` (IsBoosted) flag does TWO things when combined with a
boosted InnerLineCount:

1. **Pre-constructor** (done by Fire_At): the inner color bytes are **doubled**
   (`*2`, clamped to 0xFF). This makes the center line extra bright.
2. **In the draw loop**: on iteration 1 (first pass), the full pre-doubled color
   is used. On subsequent passes, the normal halving resumes.

So a boosted beam:
- has the inner 3 color bytes doubled (brighter core)
- draws 5 parallel lines instead of 3
- the outermost fading halo is thus wider

Visually: brighter, fatter beam with a more prominent glow halo. This is the
only difference between "normal prism shot" (`field_0x1C=3, field_0x21=0`) and
"boosted prism shot" (`field_0x1C=5, field_0x21=1`). Everything else (color
choice, source/target, duration) is identical.

---

## 9. Outgoing PrismShot laser variants (Item 9, combines with Item 8)

The outgoing laser is spawned in `TechnoClass::Fire_At @ 0x006FDD50`, specifically
the helper `FUN_006FD210` at `0x006FD210` (I name this `SpawnOutgoingLaser`).

**`SpawnOutgoingLaser` responsibilities:**
1. Allocate 0x5C bytes via `operator_new`.
2. Compute source (firing tower's FLH or source_coord).
3. Compute target (passed in, falls back to tactical conversion).
4. Pick color source:
   - If `WeaponType+0x14D` (= `IsHouseColor=true`) → use `HouseClass+0x56FC..0x56FE`
     (owner's laser color BGR, 3 bytes, each halved for outer)
   - Else use `WeaponType+0x120..0x123` (Laser{Inner,Outer,Spread}Color INI values)
5. Call `LaserDrawClass::Constructor(...)` with the chosen colors.
6. **If `WeaponType+0x14D != 0` (IsLaser): `laser->field_0x20 = 1`**
   — enabling the special beam-draw path.

So outgoing shots from any IsLaser=true weapon get the beam-draw path. The prism
block at `0x006FF50C-0x006FF544` then runs:

```c
if (firing_tower->Type == Rules->PrismType) {
    laser->field_0x1C = 3;              // default prism thickness (3 lines)
    if (firing_tower->field_0x664 > 0) {
        laser->field_0x21 = 1;          // mark boosted
        laser->field_0x1C = 5;          // thicker boosted beam (5 lines)
    }
}
```

**Thus three laser visuals arise from the prism system:**

| Source | field_0x1C | field_0x20 | field_0x21 | Visual |
|--------|-----------|-----------|-----------|--------|
| Support beam (mode-2 emit) | 3 | 1 | 0 | 3-line laser, supporter→firing tower |
| Normal PrismShot (count=0) | 3 | 1 | 0 | 3-line laser, firing tower→target |
| Boosted PrismShot (count>0) | 5 | 1 | 1 | 5-line laser w/ doubled inner color |

The beam-draw function `FUN_005509F0` does not distinguish between these by
source — it just renders `InnerLineCount` lines with potentially doubled inner
color. All three variants color-source from their spawning caller (supporters
use owner's `HouseColor+0x56FC`, outgoing prism shots likewise get HouseColor
because `PrismShot.IsHouseColor=true` in INI).

---

## 10. EMP interaction (Item 10)

The key function is `BuildingClass::CanSellOrUndeploy @ 0x004555D0`. Despite its
name, this function is wired as `vtable[0xD4]` (byte offset 0x350) on BuildingClass
and called from both `BuildingClass::GetFireError @ 0x00447F10` and
`BuildingClass::Update @ 0x0043FB20` as an "is-this-building-currently-able-to-do-
anything" check.

**`CanSellOrUndeploy` returns false if any of:**
- `HasPower == false` AND not overpowered (< 2 overpowerers)
- `EMPLockRemaining > 0` ← **EMP check**
- `Health == 0`
- `Type+0x1573 != 0` (has power-scaled damage, e.g. Tesla-style) AND
  `Type+0xEE4 > 0` AND `owner.PowerRatio < 1.0` AND not overpowered
- Various grand-opening / sale-state checks
- `Type+0x1552` (requires engineer?) conditions
- Mission is 0x12 or 0x13 (offline missions)

### 10.1 EMPed firing tower
`GetFireError` calls `CanSellOrUndeploy`, which returns false when `EMPLockRemaining > 0`.
`GetFireError` then returns **6** (NO_POWER). Mission_Attack's jumptable[6] does NOT
re-enter the cascade. **Result:** EMPed prism towers never START a cascade.

### 10.2 EMPed supporter not yet queued
The cascade selector at `0x0044b370-0x0044b4ae` does NOT call `CanSellOrUndeploy`
on candidates. It only checks:
- `candidate->field_0x90` (IsAlive)
- `candidate->Type == Rules->PrismType`
- cooldown (+0x2EC/+0x2F4)
- `candidate->field_0x714 == 0` (not in delayed-fire)
- `IsDeploying == false`
- `candidate->vtable[0x61]() != 1` (mission != MISSION_ATTACK)
- `candidate != this`
- distance in range

None of these explicitly checks EMP. **Therefore an EMPed idle supporter can
still be selected** if its mission isn't 1/12/13 and other filters pass.

### 10.3 EMPed supporter already in mode 2
When the timer expires, `ProcessDelayedFire` calls `EmitPrismSupportBeam`
unconditionally. There is no power/EMP gate in either the top of
`ProcessDelayedFire` or inside `EmitPrismSupportBeam`. **The beam is spawned
regardless of EMP state.**

### 10.4 Practical consequence
The EMP cascade-skip is enforced at the firing-tower side only. Once a
supporter has been queued (mode-2 set, timer running), EMPing it doesn't
prevent the visual beam or the firing tower's damage bonus accumulation.

For a faithful Rust implementation: gate the cascade entry on firing-tower
EMP state (via CanSellOrUndeploy-equivalent), but do NOT re-check EMP per
candidate per tick nor inside `ProcessDelayedFire`.

---

## 11. Mind Control interaction (Item 11)

Mind control rewrites `BuildingClass->Owner (+0x21C)` from the victim's original
house to the controller's house. The cascade selector's outer setup reads
`this->Owner->field_0x78` (building count) and `this->Owner->field_0x6C`
(BuildingArray) — both through the *current* Owner pointer.

**Consequence:** An MC'd Prism Tower pulls its supporter pool from the
controller's building list, not the original owner's. If the controller has no
Prism Towers, the MC'd tower fires without any support. This matches natural
"MC transfers ownership" engine semantics; no special handling needed in code.

Not explicitly decompiled this pass but cross-checked against
`MIND_CONTROL_GHIDRA_REPORT.md`'s finding that +0x21C is updated during capture.

---

## 12. Iron Curtain interaction (Item 12)

Iron Curtain is a targeted buff applied via `BuildingClass::IronCurtain @ 0x00457C90`.
Its primary effect is to set an invulnerability timer (`+0x678`-ish); **it does NOT
touch any firing/power field**. `CanSellOrUndeploy` does not check IC state.

**Consequence:** IC'd prism towers fire normally, cascade normally, and can be
selected as supporters normally. Their beams emit normally. No special gating.

Cross-checked against `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`.

---

## 13. Partial-power interaction (Item 13)

Addressed in Section 10's `CanSellOrUndeploy` logic — item 4 of the return-false
conditions. Summary: a building with `Type+0x1573 != 0` (power-sensitive flag)
AND `Type+0xEE4 > 0` (some threshold) loses functionality when
`owner.PowerRatio < 1.0`. For ATESLA, `Type+0x1573` is set (Tesla-style), so
low power does disable it.

Note: this is an **all-or-nothing** check at the point of `GetFireError`. It's
not a gradual degradation. A random-select mechanism elsewhere (not in the
cascade code) determines which buildings go offline first when power drops.

---

## 14. Vtable indices — confirmed map (Item 14)

Cross-referenced against `TECHNOCLASS_VTABLE_COMPLETE.md` and
`BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md`.

| byte | idx | used at | doc name | actual |
|------|-----|---------|----------|--------|
| 0xAC | 0x2B | `0x0044b323`, `0x0044b415` | `Get_Source_Coord` | TechnoClass — source coord (FLH-adjusted) |
| 0xB0 | 0x2C | `0x0044b4f6` | `Get_Cell_Coord` | BuildingClass override — returns building center |
| 0x168 | 0x5A | `0x0044b494` | `GetWeaponRange(int wpn)` | **TechnoClass::GetWeaponRange @ 0x007012C0** (confirmed) |
| 0x184 | 0x61 | `0x0044b3f5-b3fb` | `Get_Mission()` | **MissionClass::GetCurrentMission @ 0x005B3040** (confirmed) |
| 0x350 | 0xD4 | `0x00447F99`, others | "Is_Active" | **BuildingClass::CanSellOrUndeploy @ 0x004555D0** (named CanSellOrUndeploy but wired as general IsReady) |
| 0x3C0 | 0xF0 | `0x0044adae`, `ProcessDelayedFire` | `Can_Fire(target, wpn)` | **TechnoClass::GetFireError @ 0x006FC0B0** (confirmed) |
| 0x3CC | 0xF3 | `ProcessDelayedFire` | `Fire(target, wpn)` | **TechnoClass::Fire_At @ 0x006FDD50** (confirmed) |
| 0x3C8 | 0xF2 | `0x0044b095`-area | `Assign_Target` | TechnoClass target-clear helper |
| 0x4E8 | 0x13A | various | `Get_Target_Coord` | Gets targeting coord for passed target |
| 0x3F8 | 0xFE | `GetWeaponRange` internal | `GetWeapon` | Returns weapon struct pointer |
| 0x3FC | 0xFF | GetFireError | (facing/timer check) | Boolean check before facing math |

The prior report's "Is_Active" label for vtable[0xD4] was functionally correct
in spirit but the concrete implementation is `CanSellOrUndeploy`, a
misleadingly-named check used across many code paths as "is this building
currently functioning."

---

## 15. +0x2EC / +0x2F0 / +0x2F4 polymorphism (Item 15)

**Major finding:** these fields are NOT prism-specific. They are the standard
**TechnoClass `FireRateTimer`** trio:

| Offset | Normal meaning | Source | Size |
|--------|---------------|--------|------|
| 0x2EC | `FireTimer.StartFrame` — set to `g_CurrentFrameCounter` on Fire_At | `TECHNOCLASS_STRUCT_LAYOUT.md` line 88+ | 4 bytes |
| 0x2F0 | `FireTimer.Duration` — set to weapon ROF on Fire_At | same | 4 bytes |
| 0x2F4 | `FireTimer.Value` — ROF countdown value | same | 4 bytes |

The standard ROF check in `TechnoClass::GetFireError` reads these to determine
"is my weapon still cooling down." `elapsed = now - StartFrame; if (elapsed < Value)
return busy`.

**What the prism emit does** (`EmitPrismSupportBeam @ 0x0044ABD0`):

```c
this->field_0x2EC = g_CurrentFrameCounter;     // matches FireTimer.StartFrame semantics
this->field_0x2F0 = saved_target_y;            // overwritten with target Y (informational)
this->field_0x2F4 = Rules->PrismSupportDelay;  // matches FireTimer.Value semantics
```

### 15.1 The reuse is intentional, not an aliasing bug

By writing `FireTimer.StartFrame = now, FireTimer.Value = PrismSupportDelay`,
the emit code effectively loads the standard ROF timer with a "45-tick
cooldown". After emitting a support beam:

- The supporter cannot be **selected as a supporter again** until 45 ticks
  elapse (cascade selector reads `+0x2EC/+0x2F4` with the standard
  "remaining = Value - (now - StartFrame)" formula).
- The supporter **also cannot fire its own primary weapon** until 45 ticks
  elapse, because TechnoClass::GetFireError uses the same ROF timer math.

This is a **deliberate conflation**: the prism engine piggybacks on the ROF
timer so that post-support, a Prism Tower is locked out of both roles for the
same duration. For YR, PrismSupportDelay=45 ticks matches PrismShot ROF=45
ticks, so the effective lockout is identical either way.

### 15.2 `+0x2F0` overwrite is informational-only

`EmitPrismSupportBeam` writes `saved_target_y` to `+0x2F0`, overwriting
`FireTimer.Duration`. But `FireTimer.Duration` isn't read by `GetFireError`'s
ROF check (only StartFrame+Value are). So the overwrite doesn't affect the
cooldown. It's stale data that persists until the next Fire_At writes a fresh
ROF duration over it.

### 15.3 Implementation note

A faithful Rust implementation should model **one** per-building timer that
serves both "ROF for Fire_At" and "support cooldown". Don't split them into
separate fields — the original engine deliberately reuses the same slot.

---

## 16. Summary of corrections and additions

For an implementation spec, apply these deltas to the prior report's Section 14.10:

1. **Distance math**: compute `sqrt(dx² + dy² + dz²)` in leptons (linear). Compare
   against **firing tower's** `Secondary` weapon range (not candidate's).
2. **Range threshold**: `firing_tower.weapon[1].Range` in leptons, not squared.
3. **Anim names**: `SpecialAnim=` (healthy) and `SpecialAnimDamaged=` (damaged),
   both at art.ini, read into `BuildingTypeClass+0x11F4` and `+0x1204`.
4. **No death cleanup**: if firing tower dies mid-charge, supporters still emit
   beams to the saved (stale) coords. Harmless.
5. **Target-lost preserves count**: `field_0x664` is reset ONLY when `Fire()`
   returns a non-null bullet. All other paths (null target, GetFireError != 0,
   bullet allocation failure) leave count intact for next cycle.
6. **LaserDrawClass**: 0x5C bytes, 14 constructor params (3 src coords, 3 tgt
   coords, flags, 3 color triples, duration, intensity fade). Renderer reads
   `field_0x1C` as line count (3 = normal, 5 = boosted), `field_0x20` as
   enable-laser-path, `field_0x21` as boost-first-pass flag.
7. **Cooldown fields are the ROF timer**: `+0x2EC/+0x2F0/+0x2F4` is TechnoClass
   FireRateTimer. Prism emit writes into these slots; the result is that a
   supporter post-emit is locked out of both firing AND supporting for
   PrismSupportDelay ticks.
8. **EMP skips cascade entry only**: firing tower EMPed → cascade never starts.
   Supporter EMPed after being queued → beam still emits (harmless visual).
9. **Mind Control transfers ownership**: cascade iterates current Owner's buildings;
   MC'd firing tower pulls supporters from controller's building list.
10. **Iron Curtain has no effect on cascade**: neither firing nor being selected
    as supporter is gated by IC state.
11. **Partial power disables attack**: if owner's PowerRatio < 1.0, ATESLA
    (Type+0x1573 set) returns false from CanSellOrUndeploy, so cascade can't
    start. Supporters already queued emit normally.
12. **FUN_00712130 is dead code for stock YR**: requires upgrade count ≥ 1 and
    upgrade slot 1 filled, neither of which a stock Prism Tower ever has.
    Safe to omit from the initial implementation.

---

## Sources (this iteration)

**Decompiled / disassembled this pass:**
- `BuildingClass::Mission_Attack @ 0x0044ACF0` (selector loop distance math re-read)
- `BuildingClass::GetFireError @ 0x00447F10`
- `BuildingClass::CanSellOrUndeploy @ 0x004555D0`
- `BuildingClass::ApplyOfflineEffects @ 0x00452480`
- `BuildingClass::OnDestroyed @ 0x00445880`
- `BuildingClass::ProcessDelayedFire @ 0x004503F0` (re-decompiled for target-lost)
- `BuildingClass::AddUpgrade @ 0x00451400`
- `BuildingClass::RemoveLastUpgrade @ 0x00451690`
- `BuildingClass::GetWeapon @ 0x004526F0`
- `BuildingClass::Update @ 0x0043FB20`
- `TechnoClass::GetWeaponRange @ 0x007012C0`
- `LaserDrawClass::Constructor @ 0x0054FE60` (full 16-param signature recovered)
- `LaserDrawClass::AI_Tick @ 0x00550150`
- `LaserDrawClass::Draw_Main @ 0x00550260`
- `LaserDrawClass::Draw_Special @ 0x005509F0` (the laser/beam draw path)
- `LaserDrawClass::Destroy_All @ 0x00550000`
- `TechnoClass::SpawnOutgoingLaser @ 0x006FD210` (the `field_0x20 = 1` site)
- `FUN_00712130 @ 0x00712130` (fully decoded as `HasBurstWeaponInSlot1`)
- `Math::Sqrt_Approx @ 0x004CAC40`
- `Math::ftol @ 0x007C5F00`
- `BuildingTypeClass::ReadINI @ 0x0045FE50` (SpecialAnim / SpecialAnimDamaged writes traced)

**String addresses confirmed:**
- `0x00819E30` "SpecialAnimDamaged" → writes `BuildingTypeClass+0x1204`
- `0x00819E44` "SpecialAnim" → writes `BuildingTypeClass+0x11F4`

**Field offsets confirmed on `WeaponTypeClass`:**
- +0x9C = `Burst` (int, default 1)
- +0xB4 = `Range` (int, in leptons)
- +0x14D = `IsLaser` (byte)

**Field offsets confirmed on `LaserDrawClass`:**
- +0x00 = AnimStep, +0x08 = SpawnFrame, +0x10 = RemainingTicks
- +0x1C = InnerLineCount, +0x20 = IsLaserEffect, +0x21 = IsBoosted
- +0x24..0x38 = src/tgt coords, +0x41/0x44/0x47 = 3 color triples
- +0x4C = DurationTotal, +0x52 = FadeEnable, +0x54/0x58 = intensity fade floats

**Struct-layout cross-references:**
- `TECHNOCLASS_STRUCT_LAYOUT.md` lines 88+ (FireTimer at 0x2EC)
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (Primary/Secondary burst offsets)
- `TECHNOCLASS_VTABLE_COMPLETE.md` (vtable index confirmations)
- `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` (BuildingClass vtable overrides)
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` (Burst, Range, IsLaser fields)
- `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md` (IC doesn't touch fire fields)
- `MIND_CONTROL_GHIDRA_REPORT.md` (MC updates Owner at +0x21C)
