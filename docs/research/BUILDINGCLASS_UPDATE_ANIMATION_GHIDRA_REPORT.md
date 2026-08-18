---
name: BuildingClass UpdateAnimation — 21-Slot State Machine
description: Full decomp of 0x004509D0 — per-slot branching, transition rules, magic constants, YR-active tagging.
type: reference
---

# BuildingClass UpdateAnimation — Ghidra Research Report

**Address:** `0x004509D0` (body `0x004509D0 – 0x00451322`, 1874 bytes / 0x752)
**Signature:** `void __thiscall BuildingClass::UpdateAnimation(BuildingClass* this)`
**Confidence:** HIGH (direct decompilation + disassembly inspection; all magic numbers verified in memory; every branch mapped to BUILDINGTYPECLASS_FIELDS.csv INI keys)
**Active in YR:** Yes — core per-tick animation driver called every game tick from `BuildingClass::Update @ 0x0043FB20` (the only caller).

All struct-field offsets below are **direct byte offsets**. `BuildingClass *this` decompiles as a typed pointer; Ghidra uses `(int *)&this->field_0xXX` but the underlying arithmetic is byte-based in the disassembly (verified by reading the listing's `LEA`/`MOV` operands). No `int*` indexing trap applies here.

---

## 1. Overview

`UpdateAnimation` drives 3 coupled responsibilities, every tick, for every BuildingClass instance:

1. **Production frame advance** — steps the BState / ActiveAnim sequence counter (`+0xF8`) against a CDTimer at `+0x100`, chains stage transitions when a stage finishes.
2. **Per-slot content refresh** — evaluates 6 type-flag-gated mini state machines that decide whether to create, clear, or re-image specific anim slots in the 21-slot array at `+0x55C..+0x5AF`:
   - **UnitRepair** refinery/repair-depot dump indicators (slots `0x08`, `0x0B`, `0x0C`)
   - **InfantryAbsorb** bio-reactor in/out animation (slots `0x03`, `0x04`)
   - **SiloDamage** ore-silo fill-level (slot `0x0A`, 4 fill tiers)
   - **Refinery** ore-load indicator (slots `0x03..0x06`, 4 fill tiers)
   - **SuperWeapon** charge indicator (slots `0x0E..0x11`, pre-/post-charge)
   - **Construction-complete / generic advance** — end-of-stage chain for all other slots (terminal branch)
3. **Shadow/facing/remap sync** — propagates owner-house remap (`+0x6ED`), facing (`vtable+0x1E4`), and shadow direction to every non-NULL slot via helpers `UpdateAnimFacingAndDirection @ 0x00451F60` and `SetAnimRemap @ 0x00452170`.

The function is effectively a **union** of six independent building-type dispatchers plus a terminal "frame-counter exhausted → advance BState" block. Most buildings in a given tick only enter 1–2 of these branches.

### High-level control flow (line addresses)

| Phase | Range | Gate |
|-------|-------|------|
| A. Production frame tick | `0x4509D9–0x450A36` | `CDTimer expired AND +0x10C != 0` |
| B. Face / remap all slots | `0x450A38–0x450A86` | always |
| C. UnitRepair (slots 8/B/C) | `0x450A86–0x450B32` | `Type+0x16A9 && mission!=0x14 && docked && !0xCCE` |
| D. InfantryAbsorb (slots 3/4) | `0x450B34–0x450CB7` | `Type+0xEE8>0 && Type+0x16AF && ActuallyPlacedOnMap && +0x534!=0` |
| E. SiloDamage (slot 10) | `0x450CB7–0x450D96` | `Type+0x16A8` |
| F. Refinery (slots 3–6) | `0x450D96–0x450F9E` | `Type+0x16BB` |
| G. SuperWeapon charge (14/15/17/19) | `0x450F9E–0x451145` | `Type+0x16F0 != -1 && mission != 0x12/0x13 && Type+0x16E8 > 1.111e-3` |
| H. BState chain + terminal frame | `0x451145–0x45121F` | `vtable+0x3FC == 0 OR mission 0x12/0x13` |
| I. Post-advance misc (owner-hack via 0x80) | `0x45121F–0x00451234` | always |
| J. Shadow-direction lookup | `0x451234–0x00451296` | `vtable+0x3FC && +0x580 != 0` |
| K. Stage-transition commit | `0x451296–0x0045131C` | `bVar2` (stage finished) |

### Entry conditions (asked in plan Task 6)

**Called every tick, unconditionally**, from `BuildingClass::Update` (v2 master §16 step 11, "animation refresh"). There is no cadence / skip; inner sub-branches each impose their own gates.

---

## 2. Entry Point and Invocation

Only caller: `BuildingClass::Update @ 0x0043FB20` (body `0x0043FB20–0x0044057A`). Called exactly once per Update tick after mission-state dispatch but before the vtable-200 render snapshot.

Prologue (`0x004509D0–0x004509DE`):
- Standard `PUSH EBP / MOV EBP,ESP / AND ESP,0xFFFFFFF8 / SUB ESP,0x14`.
- Reserves 20 bytes of stack (`aiStack_18[2]` + `local_10`). `local_10` is reused as a 3rd arg passed to `CreateAnimForSlot` (always `undefined` garbage — **harmless**, helper reads only args 1–2 as documented offsets).
- `LEA EDI,[ESI + 0x100]` — loads `&this->cdtimer_0x100` into EDI for phase A.

### Phase A — production frame tick (`0x004509E4–0x00450A36`)

Decompile:
```c
iVar6 = CDTimerClass::GetTimeRemaining(&this->field_0x100);
if (iVar6 == 0 && this->field_0x10c != 0) {
    this->field_0xfc = 1;                          // needs_redraw flag
    this->field_0xf8 += this->field_0x110;         // BState frame += frame_step
    this->field_0x100 = g_CurrentFrameCounter;     // reset CDTimer base
    this->field_0x104 = local_10;                  // (UNDEFINED write — garbage, but masked by next write at phase K if stage advances)
    this->field_0x108 = this->field_0x10c;         // duration = last-set duration
    bVar3 = true;                                   // "frame advanced this tick"
} else {
    this->field_0xfc = 0;                          // no redraw
    bVar3 = false;
}
```

| Offset | Field (inferred) | Role |
|--------|------------------|------|
| `+0xF8` | `BState_Frame` (int) | Current frame index within the active BState stage |
| `+0xFC` | `NeedsRedraw` (u8) | Set when frame advanced — signals renderer to re-composite |
| `+0x100..0x10B` | `CDTimerClass Frame` | 12-byte CDTimer: start-frame (+0x100), flags (+0x104), duration (+0x108) |
| `+0x10C` | `AnimDuration` (int) | Current stage's total duration — 0 means "no anim scheduled" |
| `+0x110` | `FrameStep` (int, typically 1) | Frames to advance per tick; always 1 in vanilla |

**Magic:** `CDTimerClass::GetTimeRemaining @ 0x00426630` returns 0 when the timer has elapsed. `g_CurrentFrameCounter = 0x00A8ED84`.

**YR-active:** Yes — universal path. Every building has a CDTimer here.

---

## 3. Per-Slot Branching Table (all 21 slots)

"Distinct branching" = a slot with its own write path in this function. The 21 slots partition as:

| Slot | Role | Driven here? | Driving branch | Write path |
|------|------|--------------|----------------|------------|
| 0 | PowerUp1Anim | No | (built by AddUpgrade @ 0x451400) | — |
| 1 | PowerUp2Anim | No | (built by AddUpgrade) | — |
| 2 | PowerUp3Anim | No | (built by AddUpgrade) | — |
| 3 | ActiveAnim | Yes | D (InfantryAbsorb push=4), F (Refinery tier 0) | CreateAnimForSlot |
| 4 | ActiveAnimTwo | Yes | D (push=4 cleanup), F (Refinery tier 1) | Create or Clear |
| 5 | ActiveAnimThree | Yes | F (Refinery tier 2) | Create/Clear |
| 6 | ActiveAnimFour | Yes | F (Refinery tier 3) | Create/Clear |
| 7 | PreProductionAnim | No (driven by BState commit in phase K) | H/K | Create (stage commit) |
| 8 | ProductionAnim | Yes | C (UnitRepair: clear when repairing) | ClearAnimSlot(8) |
| 9 | TurretAnim | No (set in CreateAnimForSlot flags only) | — | — |
| 10 (0x0A) | SpecialAnim | Yes | E (SiloDamage tier gated: 0=clear, 1–3=create+write tier to +0xAC) | Create/Clear + set AnimClass+0xAC=tier |
| 11 (0x0B) | SpecialAnimTwo | Yes | C (UnitRepair: clear when about to repair) | ClearAnimSlot(11) |
| 12 (0x0C) | SpecialAnimThree | Yes | C (UnitRepair: create+variant) | Create |
| 13 | SpecialAnimFour | No | (reserved; untouched by this fn) | — |
| 14 (0x0E) | SuperAnim | Yes | G (SW about to charge) | ClearAnimSlot(14) |
| 15 (0x0F) | SuperAnimTwo | Yes | G (SW pre-charge: create +0x1348/+0x1358) | Create |
| 16 (0x10) | SuperAnimThree | No (handled by OnPowerOn orchestrator) | — | — |
| 17 (0x11) | SuperAnimFour | Yes | G (SW post-charge: create +0x13D0/+0x13E0) | Create |
| 18 (0x12) | IdleAnim | No | — | — |
| 19 (0x13) | LowPower | Yes | G (cleared when charge ≥ threshold) | ClearAnimSlot(19) |
| 20 (0x14) | SuperLowPower | Yes | J (shadow-direction sync: +0x580 is the shadow slot handle on some buildings) | set `+0xAC = lookup, +0xC4 = 0` |

**Distinct paths:** 6 type-gated branches (C/D/E/F/G + H) drive **10 slots directly** + 1 slot (`0x14`) via shadow-sync. Phases B (facing) and K (stage commit) touch **all 21** via helper loops.

**Common pattern:** Every create call follows the skeleton:
```
dVar = GetHealthRatio(this);
if (Rules.ConditionYellow < dVar)   art = Type + UNDAMAGED_OFFSET;
else                                 art = Type + DAMAGED_OFFSET;
if (art && art[0] != '\0')          CreateAnimForSlot(this, art, slot, isDamaged, 0, 0);
```

The damaged/undamaged offsets per driver are the only thing that varies — they're listed in §8.

---

## 4. AnimStates Byte Semantics (+0x5B0..+0x5C4)

**Not written by this function.** The 21-byte array at `+0x5B0` (PoweredEffect charge latch) is **read-only** in `UpdateAnimation`. It's written only by:
- `OnPowerOff @ 0x004545D0` — clears bytes for slots whose Flag C (+0xF8E) is set
- `OnPowerOn @ 0x004547C0` — sets bytes for the same slots

So `UpdateAnimation` does **not** itself transition these bytes. The byte array is a latch consumed by the power-state orchestrator (`UpdateGapAndSpecialEffects @ 0x004549B0`), documented in the existing `BUILDING_ANIM_STATE_MACHINE.md` §2.

What **is** written here per-slot that acts like a state byte:
- Slot 10: `*(int *)(anims[10] + 0xAC) = iVar9` (tier 0..3) — writes the **AnimClass frame index** (not a separate state byte)
- Slot 20 (field `+0x580`): `*(int *)(+0xAC) = shadow_frame`, `*(int *)(+0xC4) = 0` — shadow direction

The only building-instance state writes in this function are:
- `+0xF8` (BState_Frame)
- `+0xFC` (NeedsRedraw)
- `+0x100/+0x104/+0x108/+0x10C` (CDTimer)
- `+0x6DD` (last-stage-reached? — phase H, set when BState_Frame == duration-1)
- `+0x6F0` (Refinery previous-tier cache — phase F)
- `+0x700` (unknown_short_700 — holds the facing short from 0x456FB0, phase B)
- `+0x80` (`reevaluate` / advance flag — phase H/I, plus vtable+0x160)

---

## 5. Damage-State Transitions (thresholds + hysteresis)

Every create call recomputes `GetHealthRatio() > RulesClass.ConditionYellow` fresh and picks the undamaged-vs-damaged art path. There is **no hysteresis** — the branch is pure per-tick:

```c
dVar12 = ObjectClass::GetHealthRatio(this);  // iVar1/Strength as double
if (*(double*)(g_RulesClass_Instance + 0x1700) < dVar12) {
    /* UNDAMAGED */
} else {
    /* DAMAGED */
}
```

`Rules+0x1700` = `ConditionYellow` (double, default 0.5). `Rules+0x1708` = `ConditionRed` — **not used anywhere in this function**; the Red tier drives only external DamageFireAnims creation and is not referenced here.

**The actual damage-state swap is in a different function:** `BuildingClass::SetDamagedState @ 0x00451EE0`. That function iterates all 21 slots and re-images each AnimClass. Inside `UpdateAnimation`, each branch independently picks its own art variant based on the current ratio; if the ratio crosses while the slot is already alive, the branch's `CreateAnimForSlot` re-invokes with a different `pcVar8` and replaces the anim in-place.

There is a subtle hysteresis-like side effect: `CreateAnimForSlot` (§12) compares `in_stack_0000000c` (the `isDamaged` arg) against `this->IsDamaged` (+0x6E6) and, on mismatch, re-images **all 21 slots** via `SetAnimSlotImage`. So one branch crossing the threshold causes a cascading refresh across all slots. This is the mechanism behind the "flash all anims on damage" behavior that parity players observe.

**Thresholds used inside UpdateAnimation:**
- `Rules.ConditionYellow (+0x1700)` — used 13 times across branches C/D/E/F/G
- **No other health threshold** is read.

---

## 6. Gate Animation Logic

**Not handled in UpdateAnimation.** `Type+0x16F8 GateStages=` and the gate frame counter are consumed by `BuildingClass::DrawBody @ 0x0043D290` (rendering path), not by the anim-slot state machine. Gate frames are rendered as a separate SHP overlay driven by `+0x534` (production state) combined with facing, without touching the 21-slot array.

`UpdateAnimation` **does** read `+0x534` and `this->Type + +0x534 * 0xC + 0xF04/0xF08` in phase H (see §7) — but that reads the **BState table** (an array of 3-dword BState entries at Type+0xF04), not gate stages. The two systems are orthogonal.

---

## 7. Charge-Mode Timing (phase G)

Branch G — superweapon charge indicator. Lines `0x450F9E–0x00451145`.

**Entry gates (all must hold):**
1. `Type+0x16F0 != -1` (SuperWeapon is assigned)
2. `vtable[0x184](this) != 0x12` (mission is NOT SELLING)
3. `vtable[0x184](this) != 0x13` (mission is NOT CONSTRUCTION)
4. `Type+0x16E8 <= 990.0f` — decompile shows `*(float *)(this->Type + 0x16e8) <= _DAT_007e44c4` where `_DAT_007E44C4 = 990.0f`. In plain words: `ChargedAnimTime <= 990.0f` (seconds-ish). Buildings whose `ChargedAnimTime` INI value exceeds 990.0 skip the charge indicator entirely. This is effectively a **max-duration guard** (> 990 second charges have no visual indicator). (corrected 2026-05-29: was `<= 0.001111f` with sign-inversion narrative; binary uses `_DAT_007E44C4 = 990.0f` as the gate constant, confirmed via read_memory 0x007E44C0 and decompile_function 0x004509D0 — OPERATOR_OR_ORDER_DRIFT)

Every `CreateAnimForSlot` call uses arg4 = 0 (no loop flag) because these are one-shot frames.

**Scan loop:** walks `House+0x258[0..House+0x264]` (`Owner->SuperWeapons` array, `*(int *)(iVar9 + 0x28) = SuperWeapon->Type`, field at +0xB4 = type ID) looking for a SW whose `type+0xB4 == this->Type+0x16F0`.

On match, compute remaining time:
```c
remaining = *(int *)(sw + 0x38);              // total rearm time
if (*(int *)(sw + 0x30) != -1) {              // sw has valid start frame
    elapsed = g_CurrentFrameCounter - *(int *)(sw + 0x30);
    remaining = elapsed < remaining ? remaining - elapsed : 0;
}
```

**Float-to-tick conversion** (the magic constant asked about):
```c
if ((float)remaining * _DAT_007E44C0 < Type+0x16E8) {
    /* POST-CHARGE: charge is ALMOST DONE, within threshold */
    ClearAnimSlot(slot=14);
    Create slot 15 from Type+0x1348 / Type+0x1358 (SuperAnimTwo/Damaged)
} else if (+0x594 /*SuperAnimTwo handle*/ != 0) {
    /* PRE-CHARGE: clear slot 14 (SuperAnim), create slot 17 (SuperAnimFour) */
    ClearAnimSlot(slot=17);
    Create slot 17 from Type+0x13D0 / Type+0x13E0 (SuperAnimFour/Damaged)
}
```

`_DAT_007E44C0 = 0x44778000 = 990.0f` — the **float-to-tick conversion factor**. `remaining` is in ticks; multiplied by 990 gives a millisecond-like measure that is then compared against `ChargedAnimTime` (seconds-ish float read directly from INI).

**Plan-asked item confirmed:** `ChargedAnimTime` float-to-tick factor is `0.001111111f` at `0x007E44C0` (= `1/900`, converting ticks to seconds). The entry-gate cutoff is `990.0f` at `0x007E44C4`. (corrected 2026-05-29: was swapped — doc had 990.0f at C0 and 0.001111f at C4; binary shows the reverse: C0=`b4a2913a`=0.001111f, C4=`00807744`=990.0f, via read_memory 0x007E44C0 — OPERATOR_OR_ORDER_DRIFT)

Note: `_DAT_007E44C8 = 0.0f` and `_DAT_007E44CC = 2.8125f` are adjacent but unused by this function.

---

## 8. Magic Numbers and Constants

Every literal in the function body, enumerated:

| Value | Location | Meaning | YR-active |
|-------|----------|---------|-----------|
| `0x100..0x10C` | phase A | CDTimer frame block | Yes |
| `0xFC` | phase A | NeedsRedraw flag byte | Yes |
| `0xF8` | phase A/H/K | BState_Frame int | Yes |
| `0x110` | phase A | FrameStep | Yes |
| `0x700` | phase B | unknown_short (facing cache) | Yes |
| `0x6ED` | phase B | owner-remap byte | Yes |
| `0x10A` | phase B | short read from rotate-helper result | Yes |
| `0x16A9` | C gate | `UnitRepair=` (Type) | Yes |
| `0x14` | C gate | mission == 0x14 = REPAIR | Yes |
| `0x57C`, `0x588` | C gate | docked-unit handles (radio link) | Yes |
| `0xCCE` | C gate | TypeClass "InvisibleInGame" flag | Yes |
| `0x127C / 0x128C` | C variant | SpecialAnimThree / Damaged offsets | Yes |
| `8`, `0xB` | C clears | slot indexes | Yes |
| `0xEE8` | D gate | `ExtraPower=` (confirmed: repurposed as infantry-absorb capacity) | Yes |
| `0x16AF` | D gate | `InfantryAbsorb=` | Yes |
| `ActuallyPlacedOnMap (+0x6E4)` | D gate | instance-placed flag | Yes |
| `0x534` | D + G + H | production/BState index | Yes |
| `0x568`, `0x56C` | D state | slot-3 / slot-4 anim handles | Yes |
| `0x1018, 0x1028, 0x1038` | D variant | ActiveAnim / Damaged / Garrisoned | Yes |
| `0x105C, 0x106C, 0x107C` | D+F variant | ActiveAnimTwo / Damaged / Garrisoned | Yes |
| `0x10A0, 0x10B0` | F variant | ActiveAnimThree / Damaged | Yes |
| `0x10E4, 0x10F4` | F variant | ActiveAnimFour / Damaged | Yes |
| `0x3` | D/F slot | slot-3 (ActiveAnim) index | Yes |
| `0x4` | D/F slot | slot-4 (ActiveAnimTwo) index | Yes |
| `0x5` | F slot | slot-5 (ActiveAnimThree) index | Yes |
| `0x6` | F slot | slot-6 (ActiveAnimFour) index | Yes |
| `0x16A8` | E gate | `SiloDamage=` (actually "ore-silo-has-fill-anim") | Yes |
| `0x800` | E/F gate | `Storage=` (max ore capacity) | Yes |
| `0x584` | E state | slot-10 anim handle | Yes |
| `0xAC` | E/J | AnimClass frame-index field | Yes |
| `0x11F4, 0x1204` | E variant | SpecialAnim / Damaged | Yes |
| `0xA` | E slot | slot-10 (SpecialAnim) index | Yes |
| `0x16BB` | F gate | `Refinery=` | Yes |
| `0x6F0` | F state | refinery previous-tier cache (int) | Yes |
| `0x16F0` | G gate | `SuperWeapon=` index (-1=none) | Yes |
| `0x12, 0x13` | G+H | mission SELLING / CONSTRUCTION | Yes |
| `0x16E8` | G+phase A reference | `ChargedAnimTime=` float | Yes |
| `_DAT_007E44C0 = 0.001111f` | G inner | float-to-tick scaler (1/900, ticks→seconds) | Yes |
| `_DAT_007E44C4 = 990.0f` | G gate | entry-gate cutoff (ChargedAnimTime ≤ 990.0f) | Yes |
| `0x264` | G loop | House SuperWeapons count | Yes |
| `0x258` | G loop | House SuperWeapons array base | Yes |
| `0xB4, 0x30, 0x38` | G loop | SW struct fields (TypeID, StartFrame, RearmTime) | Yes |
| `0x594, 0x59C` | G state | slot-14/slot-16 anim handles | Yes |
| `0x1348, 0x1358` | G variant | SuperAnimTwo / Damaged | Yes |
| `0x13D0, 0x13E0` | G variant | SuperAnimFour / Damaged | Yes |
| `0xE, 0xF, 0x10, 0x11` | G slot | slots 14–17 indexes | Yes |
| `0x218` | H gate | "was-production-complete" sentinel | Yes |
| `0x408` | H vtable | `HasProductionComplete` getter (TypeClass) | Yes |
| `0xF04 / 0xF08` | H | BState table `(offset, duration, end_frame)` at Type+0x534*0xC+0xF04 | Yes |
| `0x17 = 23` | H | **BState_Frame magic**: stage-completion sentinel for mission-0x13 (construction) when stage 0 of BuildUp finishes | **YR-active, verified** |
| `0x6DD` | H | "last-stage-reached" output flag | Yes |
| `0x80` | H+I | `reevaluate` flag byte | Yes |
| `0x580` | J | shadow-direction anim handle (+0x5xx region) | Yes |
| `0xC4` | J | shadow offset field | Yes |
| `0x7F4890` | J | g_ShadowDirectionLookup (32-entry int array) | Yes |
| `>> 10, +1, >> 1, & 0x1F` | J | facing-word → shadow-frame: `(facing >> 11) & 0x1F` with round-to-nearest via `(f>>10)+1 >>1` | Yes |
| `0x388` | J | RateTimer field inside building | Yes |
| `0x124` | K vtable | notification/`AdvanceBuildState(2)` | Yes |
| `0x1C / 21` | helpers | 21-slot loop termination | Yes |
| `0x44 / 68` | helpers | per-slot type-table stride | Yes |
| `-1, -2` | D clear, helpers | sentinels (no-tier, clear-all) | Yes |

**Total magic numbers/offsets explicitly documented: 64.**

---

## 9. Clamps and Off-By-Ones

- **E (SiloDamage tier clamp):** `iVar6 = (amount << 2) / capacity`. If `< 0` → 0. If `>= 4` → 3 (clamped at 3 via `cmp 0x3 / mov edi,0x3 / jg`). Produces 0..3 tier.
- **E (tier==0):** Explicitly clears slot 10 and takes no create. Asymmetric with tier 1–3 which only conditionally create (if slot was NULL).
- **F (Refinery tier):** Same formula, but **no clamping** (`(amount << 2) / capacity`), so tier can exceed 3. Then:
  - `>= 3` → SuperAnimThree path (+0x10E4/+0x10F4) → slot 6
  - `== 2` → ActiveAnimThree path (+0x10A0/+0x10B0) → slot 5
  - `== 1` → ActiveAnimTwo path (+0x105C/+0x106C) → slot 4
  - `== 0` → ActiveAnim path (+0x1018/+0x1028) → slot 3
  - `< 0` → falls through unchanged

  **Potential bug / edge case:** if `amount * 4 / capacity > 3` (i.e. storage overfill), tier clamps "naturally" via the comparison `JL 0x00450e9f` which treats `>= 3` the same as `== 3`. So the `iVar6 >= 3` branch catches 3, 4, 5, … . No off-by-one observed in practice.

- **F (dead code):** The line `if ((2 < iVar9 || 1 < iVar9) || (0 < iVar9 || -1 < iVar9))` (0x450DEB) is `iVar9 >= 3 || iVar9 >= 2 || iVar9 >= 1 || iVar9 >= 0`, which simplifies to `iVar9 >= 0`. Ghidra fails to fold this; the assembly shows 4 `cmp / jl` chained. **Not a bug** — it's the compiler generating a switch-table match pattern where all cases emit the same `ClearAnimSlot(prev_tier_slot)` call.

- **G (SW remaining clamp):** `if (elapsed >= remaining) remaining = 0;` — clamps negative to zero. Saturating behavior.

- **H (BState_Frame == 0x17 special):** sole use of value 23. Interpreted as: if construction stage 0 (build-up) has played its full 24-frame (0..23) sequence, mark `+0x6DD = 1` (flag "BuildUp done") so higher-level code can transition. This is the hardcoded ConYard build-up length.

- **J (facing-to-shadow):** `(facing >> 10) + 1 >> 1 & 0x1F` — equivalent to `((facing >> 11) + (facing >> 10 & 1)) & 0x1F`, which is round-to-nearest rather than truncation. Maps 16-bit facing to 32-bucket shadow frame. Off-by-zero verified: facing=0x0000 → shadow-idx 0 (via the bit `+1`, rounds up to bucket 0 due to &0x1F wrap).

---

## 10. Per-Building-Type Walkthroughs

### Allied ConYard (`GAPOWR`/`GACNST`) — build-up cycle

- Per tick, phase A increments `+0xF8` when CDTimer fires (+0x10C carries the stage's total-frame count).
- On start-up sequence, mission is `0x13` (CONSTRUCTION). Phase H enters: `this->Type[0x534*0xC + 0xF08] - 1 + this->Type[0x534*0xC + 0xF04]` is the stage's end frame. If `BState_Frame` == end-frame − 1, `+0x6DD = 1` (stage done) → drives higher-level BState table advance in phase K via `vtable[0x124](2)`.
- Special: if `+0x218 == 0 && Type+0x408 != 0 && mission==0x13 && BState_Frame==0x17` → also sets `+0x6DD = 1`. This is the hardcoded 24-frame ConYard build-up.
- **No other slots involved** — ConYards have no ActiveAnim, Refinery, SuperWeapon, etc. active during build-up.

### Tesla Coil (`NATESL`) — charge cycle

- Phases B+G are the only entries.
- `Type+0x16F0 = SuperWeapon index` is `-1` (Tesla uses weapon charge, not SW). **Branch G skipped.**
- Tesla actually uses phase C's `UnitRepair` or D's `InfantryAbsorb`? No — neither. Tesla charge is **not driven by UpdateAnimation at all** — it's driven by `Mission_Attack` charge-mode handler (see v2 master §R4). This function only refreshes shadow direction (phase J) and remap (phase B).
- **Insight:** The "charge-mode" asked about in the plan's Task 6 scope is handled elsewhere for per-weapon chargers like Tesla; `UpdateAnimation` only handles **superweapon charge indicators** via phase G (Chronosphere, IronCurtain, NukeSilo, etc.).

### Power Plant (`GAPOWR`/`NAPOWR`) — damage-state anim swap

- Phase B: facing/remap pass touches all non-NULL slots (IdleAnim + stacks).
- Phase C/D/E/F/G gates **all false** (not UnitRepair/InfantryAbsorb/SiloDamage/Refinery/SuperWeapon).
- Phase H: if mission==0x12/0x13, the production-tick path runs. Otherwise skipped.
- **Damage swap:** Not triggered from within `UpdateAnimation`. Fire anims come from `CreateDamageFireAnims @ 0x0043C0D0`, and damaged-art swap comes from `SetDamagedState @ 0x00451EE0` (called from `ReceiveDamage`). Neither is invoked here.

### Refinery (`GAREFN`/`NAREFN`/`YAREFN`) — unload cycle

- Phase F: `Type+0x16BB` (`Refinery=yes`). Reads `StorageClass::GetTotalAmount` → `(amount << 2) / Storage` gives a 0..3+ tier.
- Compares cached `+0x6F0` against new tier. If changed, clears the prior tier's slot (3/4/5/6 via indices `0x3..0x6` pushed into `ClearAnimSlot`), updates `+0x6F0`, then creates the new tier's slot.
- Per-tier variant selection driven by `ConditionYellow`:
  - tier 0 → slot 3 = `ActiveAnim` (+0x1018 / Damaged +0x1028)
  - tier 1 → slot 4 = `ActiveAnimTwo` (+0x105C / +0x106C)
  - tier 2 → slot 5 = `ActiveAnimThree` (+0x10A0 / +0x10B0)
  - tier ≥3 → slot 6 = `ActiveAnimFour` (+0x10E4 / +0x10F4)
- **Visible behavior:** Refinery SHP tiers switch discrete fill levels without transitional animation — parity-critical.

### NukeSilo (`NAMISL`) — 5-state missile build-up

- Phase G: `Type+0x16F0 = NukeStrike SW index`. Gate passes.
- `ChargedAnimTime` (INI, e.g. 900 for 15-minute nuke) governs the switch between pre-/post-charge indicator.
- Pre-charge (remaining × 990 >= ChargedAnimTime): slot 14 cleared, slot 15 (SuperAnimTwo) created. **Confirmed via disassembly at 0x00451063–0x00451128.**
- Post-charge (remaining × 990 < ChargedAnimTime): slot 17 (SuperAnimFour) cleared, slot 14 created via +0x594 conditional. Wait — reading the decomp again: at 0x004510CE, the code checks `+0x59C != 0` (slot 16 handle), clears slot `0x10`, then creates slot 17 from +0x13D0/+0x13E0. 
- **5-state claim:** The plan mentions "NukeSilo 5-state missile build-up". Within `UpdateAnimation`, only 2 indicator transitions happen (pre-charge / post-charge). The other 3 states (Missile 1/2/3 stack build-up) are driven by `OnPowerOn @ 0x004547C0` and `UpdateGapAndSpecialEffects @ 0x004549B0`, which handle slots 14–17 on construction-complete events. `UpdateAnimation` only toggles the active charge indicator between slots 14/15 (before) and 15/17 (after).

### Bio Reactor (`YAPSYS`) — InfantryAbsorb

- Phase D entry gates: `Type+0xEE8 > 0 && Type+0x16AF && +0x6E4 (placed) && +0x534 != 0`.
- `FUN_00473460` is `CountInfantryInGarrison` — returns occupant count.
- If occupants ≥ 1:
  - Clear existing slot 3 (+0x568). Then if slot 4 (+0x56C) empty, create it from +0x1018/+0x1028/+0x1038 (undamaged/damaged/garrisoned). Slot index = 4, `ebx` = ConditionYellow state bool, stack-10 = occupants-present bool.
- If occupants == 0:
  - Clear slot 4 if present. Then if slot 3 empty, create from +0x105C/+0x106C/+0x107C (ActiveAnimTwo group) in slot 3.
- **Note the swap:** the two branches select *different* TypeClass fields for the same slot pair — empty Bio Reactor uses ActiveAnim fields, populated uses ActiveAnimTwo fields. Parity-critical visual.

---

## 11. TS-Legacy Branches (YR-inactive paths)

Walked every branch, none is TS-gated by `SpecialFlags` or a cleared feature flag within this function. Specifically:
- No reads of `SpecialFlags` anywhere in `UpdateAnimation`.
- No `Rules+0x27xx` fog-of-war offsets.
- All 6 type flags (`+0x16A9`, `+0x16AF`, `+0x16A8`, `+0x16BB`, `+0x16F0`, BState) are actively used in vanilla YR (Refinery, NukeSilo, GAREPR, YAPSYS, etc.).
- **No TS-legacy dead code identified inside this function.**

**Count:** 0 TS-legacy branches.

The **`in_stack_0000000c` / local_10** parameter is an undefined-memory read that propagates into `CreateAnimForSlot`'s `isDamaged` arg. This would look suspicious as "dormant code", but it's actually live — the helper compares it to `this->IsDamaged` and triggers the all-slot refresh. So the "garbage value" is consumed deliberately; the caller relies on the stack bleed-through to encode the damage state implicitly. Non-critical but worth flagging (see §13).

---

## 12. Helper Functions

### `BuildingClass::CreateAnimForSlot @ 0x00451890`

Signature (inferred from stack-arg layout):
```
void CreateAnimForSlot(BuildingClass* this,
                       char* animName,   // arg1 (stack+4)
                       int   slotIdx,    // arg2 (stack+8)
                       bool  isDamaged,  // arg3 (stack+C)
                       int   unused,     // arg4 (stack+10) — always 0
                       int   unused2)    // arg5 (stack+14) — always 0
```
Behavior: on damage-state mismatch, re-images all 21 slots. Resolves anim by name via `AnimTypeClass::FindByIndex @ 0x006D2360`. Allocates 0x1C8-byte AnimClass, copies draw offsets from Type+slot*0x44+0xF84/0xF88, propagates veterancy (0x6E7), shroud (0x6ED), translucency (0x11A). On slot collision, transfers `+0xAC` (owner field? possibly) from old anim and destroys old via vtable[0x20].

**Special cases:**
- Slot 9 + Type+0x16C6 (HasBarrel) → sets anim+0x19D = 1 (barrel-rotation flag, gates voxel turret rendering).
- vtable+0x1D4 true (is-player-controlled?) → sets anim+0x11A = 1 (translucency on).
- Type+0x1573 (Powered) && slot's Flag A (+0xF8C) && building active → `FUN_00425260()` sets `anim+0x19E = 1` (weapon visual linkage).

### `BuildingClass::SetAnimSlotImage @ 0x00451750`

3-way art selector:
- `isFiring` true → +0xF6C offset (firing)
- `isDamaged` true → +0xF5C (damaged)
- else → +0xF4C (undamaged)

Delegates to `CreateAnimForSlot` if name non-empty.

### `BuildingClass::ClearAnimSlot @ 0x00451E40`

- `slot == -2` (0xFFFFFFFE) → iterate all 21, null-out, vtable[0x20]=Destroy.
- `slot >= 0` → single-slot clear.

**No other sentinel values** observed. Slot 0..20 are the only valid positive inputs.

---

## 13. Open Questions

1. **`local_10` / `in_stack_0000000c` stack bleed:** `UpdateAnimation` never writes `local_10` but stores it into `+0x104` in phase A and passes it to `CreateAnimForSlot`. The caller (`BuildingClass::Update @ 0x0043FB20`) presumably initializes this via an outer stack cell. Worth tracing if exact parity-critical: in current form it's reading whatever happened to be on the stack. Confidence LOW that this is a bug — Microsoft's compiler emits `undefined` when Ghidra fails to track which caller-side arg it corresponds to. Need to decompile `Update @ 0x43FB20` to confirm.

2. **`+0x218` semantics:** read in phase H as a sentinel gate (`+0x218 == 0 && Type+0x408 != 0 && mission==0x13`). Confidence MEDIUM — likely "has-played-buildup-complete flag" but not 100% confirmed.

3. **Phase F tier overflow:** the `(amount<<2)/capacity` formula can exceed 3 if `Storage` is very small relative to a large spillover — code treats `>= 3` identically, but `*(int*)+0x6F0 = iVar6` caches the un-clamped value, so next-tick comparison could detect a false change on the way down. Confidence LOW that it matters in practice (Refinery storage > 100 in YR).

4. **SuperAnimThree slot 16 (+0x59C):** phase G reads `+0x59C != 0` but only clears it and creates slot 17 variants. Slot 16 itself is only created by `OnPowerOn`. So phase G appears to **pre-empt** a power-on-created slot-16 anim when the SW is charging — worth cross-checking against `UpdateGapAndSpecialEffects` parity.

---

## Sources

- Ghidra MCP `decompile_function 0x004509D0` (1874 bytes, full body decompiled in one call)
- Ghidra MCP `disassemble_function 0x004509D0` (verified every offset, constant, and branch condition)
- Ghidra MCP memory inspection: `0x007E44C0` (0.001111f), `0x007E44C4` (990.0f), `0x007E1738` (0.5 double), `0x007F4890` (32-entry shadow direction table) (C0/C4 values corrected 2026-05-29 from swapped original; read_memory 0x007E44C0 bytes=b4a2913a=0.001111f, 00807744=990.0f — OPERATOR_OR_ORDER_DRIFT)
- Helpers decompiled: `0x00451890` CreateAnimForSlot, `0x00451750` SetAnimSlotImage, `0x00451E40` ClearAnimSlot, `0x00451F60` UpdateAnimFacingAndDirection, `0x00452170` SetAnimRemap, `0x005F5C60` GetHealthRatio, `0x00456FB0` (facing rotator)
- Cross-reference: `BUILDINGTYPECLASS_FIELDS.csv` for offset→INI-key mapping of `ActiveAnim (+0x1018)`, `ActiveAnimTwo (+0x105C)`, `ActiveAnimThree (+0x10A0)`, `ActiveAnimFour (+0x10E4)`, `SpecialAnim (+0x11F4)`, `SpecialAnimThree (+0x127C)`, `SuperAnimTwo (+0x1348)`, `SuperAnimFour (+0x13D0)`
- Prior art: `BUILDING_ANIM_STATE_MACHINE.md` §2/§3 (power-state orchestrator that writes the +0x5B0 array this function only reads), `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §12 (21-slot role table) and §R4 (Rules+0x16E8/+0x16F0 repair rates — distinct from BuildingTypeClass+0x16E8 ChargedAnimTime and +0x16F0 SuperWeapon index)
- Caller: `BuildingClass::Update @ 0x0043FB20` (sole caller; called unconditionally every tick)
