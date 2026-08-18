# Gattling Weapon Stage System — Ghidra Research Report

**Address(es):**
- `0x0070E000` — `TechnoClass__UpdateGattlingStage` (decay)
- `0x0070DE70` — `TechnoClass__IncreaseGattlingStage` (charge-up + stage advance)  *(labelled in Ghidra)*
- `0x0070DDF0` — `TechnoClass__GetGattlingValue` (trivial getter)  *(labelled in Ghidra)*
- `0x0070E140` — `TechnoClass__GetWeapon` (elite-aware weapon lookup by index)
- `0x006F3330` — `TechnoClass__SelectWeaponAgainst` (stage→weapon-index mapping)
- `0x00714000+` — `TechnoTypeClass__ReadINI` (parses all keys at offsets ~0x71401D..0x71411F)
- `0x00736DF0` — `UnitClass__Fire_At_Target` (calls both stage update functions)
- `0x0044ACF0` — `BuildingClass__Mission_Attack` (per-fire-error jumptable at `0x0044B728`; charge / decay sites within its handlers)
- `0x004496B0` — separate BuildingClass mission helper (also calls decay; not Mission_Attack — appears to be construction / power-down state)
- `0x0071AF20` — `TemporalClass__InitiateWarp` (decays target's stage when warp begins)
- `0x00772080` — `WeaponTypeClass__ReadINI` (confirms `Report=` sound list at WeaponType+0xCC)
- `0x006F9E50` — `TechnoClass__AI_Update` (per-tick AI; increments `+0xC4` and gates `AnimClass__UpdateLoopingSound` on `IsGattling`)
- `0x00710AF0` — `TechnoTypeClass__Constructor` (sets all gattling defaults to 0)

**Confidence:** HIGH for offsets, formulas, INI parsing, stage transitions, BuildingClass call sites, `WeaponType+0xCC = Report.Count`, and **vtable+0x3F8 = `GetWeapon`** (verified — see §5). HIGH for the gattling-exclusivity of bytes 0x4B8 / 0x4D4 (no consumer outside the two stage functions). MEDIUM only for the suggested *names* of those two bookkeeping bytes.

**Active in YR:** Yes (live). Used by `[YTNK]` Gattling Tank, `[YAGGUN]` Yuri Gattling Cannon, and any other unit/building setting `IsGattling=yes`.

---

## 1. Overview

The Gattling system lets a unit cycle through up to 6 progressively faster/heavier
weapon "stages". Each fire adds `RateUp` to a per-instance accumulator
(`GattlingValue`); each non-fire / failed-fire tick subtracts `RateDown * ticks`.
When the accumulator crosses a stage threshold upward, the stage index advances
and `SelectWeaponAgainst` returns a higher-index weapon pair (ground / AA). When
it crosses a threshold downward, the stage drops back. The accumulator is capped
at `Stage[WeaponStages]` (the last threshold value).

Stages always come in pairs: **Weapon[stage*2] = ground**, **Weapon[stage*2+1] = anti-air**,
selected per-target by `SelectWeaponAgainst`. Elite veterans use a parallel
`EliteStage*` threshold table and `EliteWeapon*` set.

---

## 2. Class Layout / Key Offsets

### TechnoTypeClass (per-type, set from INI)

| Byte offset | `int *` index | Field | Type | Notes |
|---|---|---|---|---|
| `0xCD5` | byte | `IsGattling` | `bool` (1 byte) | Master enable flag |
| `0xCD8` | `[0x336]` | `WeaponStages` | `int` | Number of reachable stages (0..N-1). Default 0 (off). |
| `0xCDC` | `[0x337]` | `Stage1` threshold | `int` | Threshold to enter / maintain stage 1 |
| `0xCE0` | `[0x338]` | `Stage2` threshold | `int` | |
| `0xCE4` | `[0x339]` | `Stage3` threshold | `int` | Also acts as **value cap** when WeaponStages=3 |
| `0xCE8` | `[0x33A]` | `Stage4` threshold | `int` | |
| `0xCEC` | `[0x33B]` | `Stage5` threshold | `int` | |
| `0xCF0` | `[0x33C]` | `Stage6` threshold | `int` | Max — last possible stage |
| `0xCF4` | `[0x33D]` | `EliteStage1` threshold | `int` | |
| `0xCF8` | `[0x33E]` | `EliteStage2` threshold | `int` | |
| `0xCFC` | `[0x33F]` | `EliteStage3` threshold | `int` | |
| `0xD00` | `[0x340]` | `EliteStage4` threshold | `int` | |
| `0xD04` | `[0x341]` | `EliteStage5` threshold | `int` | |
| `0xD08` | `[0x342]` | `EliteStage6` threshold | `int` | |
| `0xD0C` | `[0x343]` | `RateUp` | `int` | Accumulator gain per "fire tick" (default 1) |
| `0xD10` | `[0x344]` | `RateDown` | `int` | Accumulator loss per "decay tick" (default 0; YR INIs typically set 50) |

Stage table layout (stride 4 bytes, 6 ints wide for both normal + elite):
```
+0xCD8  WeaponStages (count)
+0xCDC  Stage1   +0xCE0  Stage2   +0xCE4  Stage3   +0xCE8  Stage4   +0xCEC  Stage5   +0xCF0  Stage6
+0xCF4  EliteStage1                                                              ...    +0xD08  EliteStage6
+0xD0C  RateUp                                                                          +0xD10  RateDown
```

### TechnoClass (per-instance runtime state)

| Byte offset | `int *` index | Field | Type | Notes |
|---|---|---|---|---|
| `0x140` | `[0x50]` | `CurrentGattlingStage` | `int` | 0..WeaponStages-1 |
| `0x144` | `[0x51]` | `GattlingValue` | `int` | Accumulator. Range [0, Stage[WeaponStages]] |
| `0x148` | `[0x52]` | `GattlingCycleCount` | `int` | UnitClass-specific; incremented per successful fire when IsGattling and `GetGattlingValue() >= 1`. Purpose appears cosmetic / animation. |
| `0x4A4` | byte (`+0x129`) | (sound playback location?) | byte[?] | Passed as arg to `VocClass__PlayAt` in IncreaseGattlingStage |
| `0x4B8` | byte (`+0x12E`) | `gattling_spinup_sound_flag` | `bool` | Cleared on decay entry; set when stage-advance sound plays. (Low conf on name) |
| `0x4D4` | byte (`+0x135`) | `gattling_muzzle_anim_flag` | `bool` | Cleared on stage drop / transition. (Low conf on name) |

**`GattlingCycleCount` (+0x148) is incremented in BOTH UnitClass and BuildingClass**
on successful fire (BuildingClass increment is at `0x0044B235`, byte pattern
`8B 86 48 01 00 00 40 89 86 48 01 00 00`). It is **never read** by any function
in the binary — the only `MOV reg, [esi+0x148]` reads outside the increment
sites are inside `Blitter_selector_extended` (`0x004910C7`) and a memcpy-style
helper at `0x007C3910`, where ESI is not a TechnoClass. Strongly suggests
`GattlingCycleCount` is **dead/vestigial** — likely a Tiberian Sun legacy
counter or a debug field that no consumer survived in YR. Safe to defer or
omit in our Rust port.

### TechnoClass+0xC4 — per-tick "frames since last gattling update" counter

**Confirmed via `TechnoClass__AI_Update` at `0x006F9E50`:**
```c
*(int *)&param_1->field_0xc4 = *(int *)&param_1->field_0xc4 + 1;
MissionClass__Mission_Dispatch();
```

Every per-instance AI tick (units, infantry, buildings, aircraft):
1. `+0xC4` is incremented by 1.
2. The current mission is dispatched, which (for buildings using gattling) reads
   `+0xC4`, passes it to `IncreaseGattlingStage` / `UpdateGattlingStage` as the
   `fire_ticks`/`decay_ticks` argument, and zeros it back to 0.

**Why buildings care but units don't:** mission dispatch returns a "rate" — the
number of ticks until the mission should fire again. UnitClass `Mission_Attack`
typically fires every tick (rate=1), so `+0xC4` is always exactly 1 between
calls — UnitClass just hard-codes the constant `1` and ignores `+0xC4`.
BuildingClass `Mission_Attack` may fire less often than every tick (its base
rate depends on the building's `MissionTimerEntry` and the result of the
previous dispatch), so the building reads the actual accumulated delta from
`+0xC4` to make `GattlingValue` advance/decay in proportion to wall-clock
time rather than dispatch count.

This means for a building whose Mission_Attack ran 3 ticks ago, `+0xC4` will be
3 on entry, and `IncreaseGattlingStage(3)` will add `RateUp * 3` to the
accumulator (not `RateUp * 1`).

**Implication for our Rust port:** if we mirror Westwood's "missions return next-
tick rate" structure for buildings, we need the equivalent delta accumulator.
If we instead drive Mission_Attack every tick uniformly, we can pass `1`
everywhere like UnitClass does — but then INI-tuned `RateUp`/`RateDown` values
will fire *more* often per real-time second than in the original game when the
building's mission rate is > 1. Either approach is internally consistent;
matching original cadence requires tracking the rate.

---

## 3. Core Logic

### 3.1 Stage advancement (charge-up)
`TechnoClass__IncreaseGattlingStage(this, fire_ticks)` at `0x0070DE70`. Called
**once per successful-or-near-successful fire** with `fire_ticks = 1`.

```
fn increase_gattling_stage(this, fire_ticks):
    type      = this.vtable.GetType()      # vtable+0x84
    stages    = type.WeaponStages          # +0xCD8
    is_elite  = VeterancyClass::IsElite(this)

    # 1. Cap check & accumulator gain
    cap = (is_elite ? type.EliteStage[stages]   # +0xCF0 + stages*4 (corrected 2026-05-29: was +0xCF4; binary at 0x0070DE70 shows 0xcf0+iVar2*4 for elite cap — OFFSET_RETYPED_WRONG)
                    : type.Stage[stages])       # +0xCD8 + stages*4
                                                # NOTE: stages-th element = "one past last reachable
                                                #       stage" = cap value
    if this.GattlingValue < cap:
        this.GattlingValue += type.RateUp * fire_ticks

    # 2. Refresh muzzle weapon for current stage
    stage = this.CurrentGattlingStage
    weapon_struct = this.vtable.GetWeapon(stage * 2)   # vtable+0x3F8
    AnimClass::Detach(...)
    this.gattling_muzzle_anim_flag = false             # +0x4D4

    # 3. Stage-up check (only stages 0..count-2 can advance)
    if 0 <= stage < stages - 1:
        next_thr = (is_elite ? type.EliteStage[stage + 1]   # +0xCF4 + stage*4
                             : type.Stage[stage + 1])       # +0xCDC + stage*4
        if this.GattlingValue >= next_thr:                  # NB: compares old value (pre-EBX clobber)
            stage += 1
            this.CurrentGattlingStage = stage
            weapon_struct = this.vtable.GetWeapon(stage * 2)
            AnimClass::Detach(...)
            this.gattling_spinup_sound_flag = false         # +0x4B8

    # 4. Play stage-advance sound once per advance
    if !this.gattling_spinup_sound_flag and weapon_struct.weapon.field_0xCC > 0:
        AnimClass::Detach(...)
        Random::Next()
        VocClass::PlayAt(this + 0x129)                      # play "spin-up" voc
        this.gattling_spinup_sound_flag = true
```

Notes:
- Reachable stage range is `[0, WeaponStages-1]`. With `WeaponStages=3`, stages are
  `0, 1, 2` and the last threshold (`Stage3`) is the **value cap**, not a
  threshold to enter a stage 3.
- The threshold to enter stage `s+1` is `Stage[s+1]`, computed as
  `(TypeClass+0xCDC) + s*4`. Symmetric with the decay path.
- Cap is checked **before** the increment, so the accumulator may overshoot the
  cap by up to `RateUp` on the last increment.

### 3.2 Stage decay
`TechnoClass__UpdateGattlingStage(this, decay_ticks)` at `0x0070E000`. Called
once per game frame when the unit/building is in attack mission but cannot fire
(out of range, wrong facing, target invalid, weapon arming, etc.) or when its
gattling target gets warped by a temporal weapon.

```
fn update_gattling_stage(this, decay_ticks):
    SoundEvent::Release(...)
    this.gattling_spinup_sound_flag = false                # +0x4B8 byte cleared

    type            = this.vtable.GetType()                # vtable+0x84
    decay_amount    = type.RateDown * decay_ticks          # +0xD10
    this.GattlingValue -= decay_amount
    if this.GattlingValue < 0 or decay_amount == 0:
        this.GattlingValue = 0

    value = this.GattlingValue
    stage = this.CurrentGattlingStage

    if value == 0 and stage == 0:
        # fully cooled — detach muzzle anims, clear flags
        if this.gattling_muzzle_anim_flag or this.gattling_spinup_sound_flag:
            AnimClass::Detach(...); AnimClass::Detach(...)
            this.gattling_muzzle_anim_flag = false
            this.gattling_spinup_sound_flag = false
        return

    # otherwise: refresh muzzle weapon for current stage
    this.vtable.GetWeapon(stage * 2)                       # vtable+0x3F8

    if stage > 0:
        thr = (is_elite ? type.EliteStage[stage]           # +0xCF0 + stage*4
                        : type.Stage[stage])               # +0xCD8 + stage*4
        if value < thr:
            stage -= 1
            if stage >= 0:
                this.CurrentGattlingStage = stage
            this.vtable.GetWeapon(stage * 2)
            AnimClass::Detach(...)
            this.gattling_muzzle_anim_flag = false
```

Notes:
- The threshold to **maintain** stage `s` is `Stage[s]`, computed as
  `(TypeClass+0xCD8) + s*4` (= same array, so for `s=1` you read +0xCDC =
  Stage1). This is symmetric with the entry threshold: `Stage[s]` is both the
  entry and the maintain threshold for stage `s`.
- `RateDown == 0` short-circuits the value to 0 in **one** tick (instant cool).
  This matches the modder docs ("if 0, instantly drops to zero").
- The decay function only drops **one** stage per call. With `RateDown=50` and
  e.g. `Stage2=400`, it can take several frames of decay before falling out of
  stage 2 → stage 1.

### 3.3 Stage → weapon-index mapping
`TechnoClass__SelectWeaponAgainst` at `0x006F3330`. For an `IsGattling` unit:

```
if type.IsGattling:                                # +0xCD5
    s = this.CurrentGattlingStage
    if (target_warhead.AA != 0) and target.IsInAir():
        return s * 2 + 1            # anti-air weapon for stage s
    return s * 2                    # ground weapon for stage s
```

So the ground/AA pair for stage `s` lives at weapon-list indices `2s` and `2s+1`.
With `WeaponStages=3` (max stage 2), valid indices are 0..5 → `WeaponCount=6`.

`TechnoClass__GetWeapon(idx)` at `0x0070E140` returns the elite-or-normal weapon
struct at `idx`, looking it up via `WeaponList`/`EliteWeaponList` lookups
(`FUN_007177C0` / `FUN_007177E0`). It does the elite swap automatically — callers
do not pass an "elite" flag.

### 3.4 Call sites — when each function fires

Verified via `get_xrefs_to` on both stage functions (UNCONDITIONAL_CALL list).

**`IncreaseGattlingStage` (charge / advance):**

| Address | Function | Fire-error context | `decay_ticks`/`fire_ticks` arg |
|---|---|---|---|
| `0x0073708A` | `UnitClass__Fire_At_Target` | OK / REARM / ROTATING / FACING (codes 0,2,3,4) | constant `1` |
| `0x0044B1C6` | `BuildingClass__Mission_Attack` (FIRE_REARM handler at `0x44B187`) | code 2 | `[this+0xC4]` (elapsed ticks) |
| `0x0044B21D` | `BuildingClass__Mission_Attack` (FIRE_ROTATING handler at `0x44B1DE`) | code 3 | `[this+0xC4]` |
| `0x0044B6EF` | `BuildingClass__Mission_Attack` (FIRE_OK fall-through, after `vtable[0x3CC]` Fire dispatch at `0x44B6D0`) | code 0 | `[this+0xC4]` |

**`UpdateGattlingStage` (decay):**

| Address | Function | Fire-error context | `decay_ticks` arg |
|---|---|---|---|
| `0x007370A9` | `UnitClass__Fire_At_Target` | non-OK fire result (anything not 0/2/3/4) | `1` |
| `0x00737116` | `UnitClass__Fire_At_Target` | end-of-function: no target / weapon arming timer == 0 | `1` |
| `0x0044B12C` | `BuildingClass__Mission_Attack` (handler at `0x44B0DE`: AMMO/RANGE/BUSY/MOVING) | codes 1/5/6/8 | `[this+0xC4]` |
| `0x0044B26C` | `BuildingClass__Mission_Attack` (handler at `0x44B24F`: code 0xa) | code 0xa (CLOAKED-ish) | `[this+0xC4]` |
| `0x0044B2AC` | `BuildingClass__Mission_Attack` (FIRE_ILLEGAL handler at `0x44B284`, then JMPs into FIRE_FACING/CANT block) | code 9 (and falls into 4/7) | `[this+0xC4]` |
| `0x004496DA` | `BuildingClass FUN_004496B0` (separate mission helper, NOT Mission_Attack — likely Mission_Construction or Mission_Repair: handles `GrandOpening`, `BibArea`) | non-attack mission | const arg |
| `0x0071B10B` | `TemporalClass__InitiateWarp` | when this warp starts, the warp **target**'s stage is decayed if target `IsGattling` | `1` |

**Notable difference, UnitClass vs BuildingClass charge-up:**
- UnitClass charges on **FIRE_FACING** (code 4); BuildingClass does **not** —
  there is no `IncreaseGattlingStage` call inside the FIRE_FACING handler at
  `0x44B14E`. This is presumably because a vehicle that's still rotating its
  turret is "trying to fire" and should keep its spool-up, while a building's
  fixed turret can't be in a meaningful "facing" state mid-attack.

**Building decay differences:**
- BuildingClass decays during the FIRE_ILLEGAL handler before falling through
  to the FACING block (so an illegal fire causes BOTH decay AND the FACING
  block runs). UnitClass has no equivalent.

---

## 4. INI Keys

All keys are read in `TechnoTypeClass__ReadINI` (parses around `0x71401D..0x71411F`).
Stage1..N and EliteStage1..N are read inside a loop guarded by
`IsGattling && WeaponStages > 1`. The loop iterates `i = 1..WeaponStages` (so
`Stage1`..`Stage[WeaponStages]` are all read — one extra beyond the reachable
stage count, which becomes the value cap).

| Key | Section / level | Type | Default in binary | Effect |
|---|---|---|---|---|
| `IsGattling=` | `[<unit>]` (TechnoType) | bool | **false** (verified in ctor `0x00710AF0`) | Enables stage system |
| `WeaponStages=` | TechnoType | int | **0** (verified) | Number of reachable stages (0..N-1). Loop also reads one extra entry as cap. The Stage/EliteStage read loop is gated by `IsGattling && WeaponStages > 1`, so leaving this 0 disables the system safely. |
| `Stage1=` … `Stage6=` | TechnoType | int | **0** (verified — both arrays cleared by 6-iteration loop in ctor) | Threshold (in `GattlingValue` units) to enter / maintain stage N. Last entry (`Stage[WeaponStages]`) is the **value cap**. |
| `EliteStage1=` … `EliteStage6=` | TechnoType | int | **0** (verified) | Same, for veteran/elite units |
| `RateUp=` | TechnoType | int | **0** (verified — YR INIs always set 1; with default 0 the unit would never advance) | Accumulator gain per fire tick |
| `RateDown=` | TechnoType | int | **0** (verified — YR INIs always set 50; **default 0 = instant snap to 0** because `param_2 == 0` short-circuits in `UpdateGattlingStage`) | Accumulator loss per decay tick |
| `WeaponCount=` | TechnoType | int | 0 | General weapon-list size (not gattling-specific). Gattling units set this to `2 * WeaponStages` (e.g., 6 for `WeaponStages=3`). Stored at TypeClass+0x80C. |
| `TurretCount=` | TechnoType | int | 0 | General turret count (not gattling-specific). Stored at TypeClass+0x808. |
| `Weapon1=`..`Weapon8=` | TechnoType | id | — | Weapon list. Gattling stage `s` uses `Weapon[s*2+1]` (ground) and `Weapon[s*2+2]` (AA), 1-indexed in INI. |
| `EliteWeapon1=`..`EliteWeapon8=` | TechnoType | id | — | Elite weapon list, same indexing |

YR retail values for `[YTNK]` and `[YAGGUN]`:
```
WeaponStages=3
Stage1=200       Stage2=400       Stage3=600
EliteStage1=100  EliteStage2=200  EliteStage3=300
RateUp=1         RateDown=50
WeaponCount=6
Weapon1=AGGattling   Weapon2=AAGattling   (stage 0)
Weapon3=AGGattling2  Weapon4=AAGattling2  (stage 1)
Weapon5=AGGattling3  Weapon6=AAGattling3  (stage 2)
```

Concrete implications with these values:
- 200 successful fires to enter stage 1; 400 cumulative to enter stage 2.
- Cap = 600. After saturation, 4 failed-fire-ticks (4 * 50 = 200) bring you back
  below the Stage2 maintain threshold.
- Elite needs only 100/200 fires to ramp through (twice as fast).

---

## 5. Integration Points

**Read by** (consumers of `IsGattling`/`CurrentGattlingStage`/`GattlingValue`):
- `TechnoClass__SelectWeaponAgainst` (0x6F3330): translates stage → weapon idx.
- `UnitClass__Fire_At_Target` (0x736DF0): both updates the state and consults the
  weapon struct via vtable+0x3F8 to know which barrel/animation to play.
- `BuildingClass FUN_004496B0`: same decay-on-cannot-fire pattern.
- `TemporalClass__InitiateWarp` (0x71AF20): decays the target's stage when warp
  begins.

**Tick ordering:**
- `IncreaseGattlingStage` and `UpdateGattlingStage` are both invoked from inside
  the unit/building **mission-attack** dispatch (plus the `FUN_004496B0`
  construction-like building mission helper and `TemporalClass__InitiateWarp`).
  There is no separate "gattling subsystem update" — the state machine is
  driven entirely by fire-attempt outcomes (and warp / construction events).
- **`TechnoClass__AI_Update` does NOT decay gattling.** Its per-tick body
  increments `+0xC4` and (if `IsGattling`) calls
  `AnimClass__UpdateLoopingSound()` to keep the gattling spool-up audio
  positioned, but it does not touch `GattlingValue` or `CurrentGattlingStage`.
- **CONFIRMED idle-stage retention:** A Gattling Tank that disengages from
  attack mission entirely (e.g., player commands move, Mission_Guard with no
  visible target) will not decay its stage until it next runs Mission_Attack
  with a failed fire — at which point the building/unit bulk-decays by the
  accumulated `+0xC4` (for buildings) or by `1` per failed-fire tick (for
  units). With `RateDown=50` and `Stage2=400`, even just 8 failed-fire ticks
  in attack mission drop a unit from stage 2 → 1, so the stuck-stage window
  closes quickly once attack mission resumes — but it can persist arbitrarily
  long during idle / move missions.

**vtable+0x3F8 = `GetWeapon(idx) -> WeaponStruct*`** (HIGH confidence). Verified:
- `BuildingClass__GetWeapon` exists at `0x004526F0` with a single DATA xref from
  `0x007E42B4` — i.e., it is the BuildingClass override of one specific virtual
  method.
- `BuildingClass__GetWeapon` (1) has the matching signature
  `int * __thiscall(int *this, idx)`, (2) internally calls
  `(**(code **)(*piVar2 + 0x3F8))(0)` recursively on a sub-object — proving
  vtable+0x3F8 is the same method it itself overrides, and (3) falls through to
  `TechnoClass__GetWeapon(idx)` at `0x0070E140` for the default case.
- The byte pattern `FF 90 F8 03 00 00` (`CALL [EAX+0x3F8]`) appears in 60+
  combat functions including `SelectWeaponAgainst`, `GetWeaponRange`,
  `IncreaseGattlingStage`, `UpdateGattlingStage`, `Fire_At_Target` — all
  consistent with weapon-by-index lookup.

**Bytes +0x4B8 and +0x4D4 are gattling-exclusive** (HIGH confidence). Byte
patterns for both reads (`80 BE B8 04 00 00` / `80 BE D4 04 00 00`) and writes
(`C6 86 B8 04 00 00` / `C6 86 D4 04 00 00`) were searched binary-wide:
- All 4 writes to `+0x4B8` are inside `IncreaseGattlingStage`/`UpdateGattlingStage`
  (`0x0070DF79`, `0x0070DFE2`, `0x0070E017`, `0x0070E092`).
- All 3 writes to `+0x4D4` are likewise inside the two functions
  (`0x0070DF0E`, `0x0070E08B`, `0x0070E10B`).
- **No `CMP byte ptr [esi+0x4B8/0x4D4], imm` exists anywhere else** — the only
  reads are the existing `(char)param_1[0x12E]` / `(char)param_1[0x135]` checks
  inside the two functions themselves.
- Implication: these two bytes are pure internal state for the gattling state
  machine — no rendering, audio, AI, or networking code observes them.

---

## 6. Current Rust Implementation Status

**Status: NOT IMPLEMENTED.** Rust scan found:
- No matches for `gattling`/`gatling`/`weapon_stage`/`stage_index`/`IsGattling`/
  `WeaponStages`/`RateUp`/`RateDown` anywhere in `src/`.
- [src/rules/weapon_type.rs](../ra2-rust-game/src/rules/weapon_type.rs) `WeaponType`
  has no gattling-relevant fields.
- [src/rules/object_type.rs](../ra2-rust-game/src/rules/object_type.rs) `ObjectType`
  carries `primary` / `secondary` / `weapon_list` (the latter for IFV) but no
  `is_gattling`, `weapon_stages`, stage thresholds, or rate fields.
- [src/sim/game_entity.rs:187](../ra2-rust-game/src/sim/game_entity.rs#L187)
  `GameEntity` has `ifv_weapon_index: Option<u32>` for IFV passenger override but
  no `gattling_stage` / `gattling_value` fields.
- [src/sim/combat/mod.rs](../ra2-rust-game/src/sim/combat/mod.rs) `AttackTarget`
  tracks ROF/burst state but nothing per-stage.
- [src/sim/combat/combat_weapon.rs:86](../ra2-rust-game/src/sim/combat/combat_weapon.rs#L86)
  `select_weapon()` selects only Primary/Secondary by warhead verses — no
  stage-aware selection.

**Rough plug-in plan** (for a future implementation conversation):
1. Parse the 5 new TechnoType keys + Stage1..6 / EliteStage1..6 into the
   `ObjectType` / `TechnoType` rules struct.
2. Add `gattling_stage: u8` and `gattling_value: i32` to whichever per-entity
   struct holds runtime weapon state (alongside `AttackTarget`).
3. Modify `select_weapon()` so `IsGattling` units bypass primary/secondary
   selection and instead use `gattling_stage * 2 + (target_is_air ? 1 : 0)` to
   index into the weapon list (with elite swap).
4. In the attack-tick handler:
   - On successful fire dispatch (fire-OK, REARM, ROTATING, FACING): call the
     stage-up routine — apply cap, `value += rate_up`, then check
     `value >= threshold[stage+1]` to advance.
   - On failed-fire dispatch / no-target: call the decay routine —
     `value -= rate_down`, clamp to 0; if `rate_down == 0` snap to 0; if
     `value < threshold[stage]` and `stage > 0`, decrement stage.
5. The `WeaponCount=` / `TurretCount=` fields are general (not gattling-specific)
   and should be parsed independently if not already.

---

## 7. Open Questions

**Resolved in follow-up pass (2026-04-19):**
- ~~BuildingClass charge-up path~~ — **CONFIRMED** at three sites in
  `BuildingClass__Mission_Attack` (FIRE_REARM / FIRE_ROTATING / FIRE_OK
  handlers; see §3.4). Buildings pass `[this+0xC4]` as the elapsed-tick arg,
  not the constant `1` UnitClass uses.
- ~~`GattlingCycleCount` purpose~~ — **CONFIRMED dead/vestigial.** Byte-pattern
  search for `MOV r, [esi+0x148]` finds only the increment sites in
  UnitClass/BuildingClass and false positives in unrelated functions
  (`Blitter_selector_extended`, a memcpy helper). No consumer reads it. Likely
  TS legacy.
- ~~`WeaponType+0xCC` semantics~~ — **CONFIRMED `WeaponType.Report.Count`** via
  `WeaponTypeClass__ReadINI` at `0x00772080`. The layout from the parser is:
  `+0xCC` = Report count, `+0xD0` = Report Items pointer, `+0xD4` = Report
  CapacityIncrement / list-tail field (DynamicVectorClass<int>, sound IDs).
  The check `weapon.Report.Count > 0` gates whether the stage-transition VOC
  plays — i.e., the same `Report=` list a normal weapon uses on fire. *Implication:*
  the spin-up sound on stage advance is just one of the new stage's
  `Report=` sounds picked at random — not a separate gattling-spinup INI key.

**Resolved in second follow-up pass (2026-04-19):**
- ~~vtable+0x3F8 confirmation~~ — **CONFIRMED HIGH** = `TechnoClass::GetWeapon`.
  See §5 for evidence (BuildingClass override + recursive call + 60+ matching
  byte patterns).
- ~~Bytes at +0x4B8 and +0x4D4~~ — **CONFIRMED gattling-exclusive.** Binary-wide
  byte-pattern search shows zero readers/writers outside the two stage
  functions. Names "spinup-sound flag" / "muzzle-anim flag" remain working
  labels but the bytes are 100% safe to model as private state.
- ~~Identity of `FUN_004496B0`~~ — **PARTIALLY identified.** Vtable position
  (`0x007E40D8`) sits 12 bytes (3 enum slots) past `BuildingClass__Mission_Attack`
  (`0x007E40CC`). Behavior — calling `BuildingClass__GrandOpening` and
  `BuildingClass__ClearBibArea`, plus a state-machine on `+0x2F` (0→1
  transition) and a child-unit mission re-trigger — is consistent with
  `Mission_Construction` (the building-rising-up animation), but the 3-slot
  offset from Mission_Attack does not match the standard YR mission enum. Most
  likely the BuildingClass mission method order differs from the enum order.
  **Did not rename in Ghidra** — confidence below the 90% bar per
  `CLAUDE.md`. Worth a dedicated /re-investigate pass on building mission
  dispatch to nail down all overrides at once.

**Resolved in third follow-up pass (2026-04-19):**
- ~~Idle decay outside attack mission~~ — **CONFIRMED via `TechnoClass__AI_Update`
  decompilation.** Per-tick AI does NOT decay gattling; it only increments
  `+0xC4` and refreshes the looping spool-up sound when `IsGattling`. So units
  in non-attack missions retain their stage indefinitely. (See §5 Tick ordering
  for the full picture.) Players testing Gattling Tank idle behavior should see
  "stuck stage" until Mission_Attack resumes.
- ~~`RateUp=0` default~~ — **CONFIRMED 0** in `TechnoTypeClass__Constructor`
  at `0x00710AF0` (`param_1[0x343] = 0`). YR INIs always set `RateUp=1`. With
  the binary default of 0, an `IsGattling=yes` unit that omits `RateUp=` would
  never advance past stage 0.
- ~~`RateDown=0` default~~ — **CONFIRMED 0** in same constructor
  (`param_1[0x344] = 0`). Default behavior = instant snap to 0 on any decay
  call (because the `param_2 == 0` branch in `UpdateGattlingStage` zeros
  `GattlingValue` directly).
- ~~All other gattling defaults~~ — `IsGattling` (false), `WeaponStages` (0),
  Stage1..6 (0), EliteStage1..6 (0) all confirmed 0 in the constructor.
- ~~`BuildingClass+0xC4` increment site~~ — **CONFIRMED at
  `TechnoClass__AI_Update`** (line `*(int *)&param_1->field_0xc4 =
  *(int *)&param_1->field_0xc4 + 1;`). Increment is on **all** TechnoClass
  instances (units, infantry, buildings, aircraft), not buildings-only. The
  earlier byte-pattern misses (`INC`/`ADD imm`) were because the compiler
  emitted load-modify-store as `MOV reg,[esi+0xC4]; ADD reg,1; MOV [esi+0xC4],reg`
  — different opcode signature.

**Still open:**

1. **Building mission cadence.** With `+0xC4` understood, the remaining unknown
   is what rates `BuildingClass::Mission_Attack` actually returns from
   `MissionClass__GetMissionTimerEntry` for gattling buildings vs other
   buildings. This determines how the `+0xC4` delta typically looks in
   practice (always 1? always 5? variable?). Worth a quick check on
   `MissionTimerEntry` defaults for buildings.

---

## Sources

**Ghidra functions decompiled:**
- `0x0070E000` `TechnoClass__UpdateGattlingStage`
- `0x0070DE70` (FUN_) `TechnoClass__IncreaseGattlingStage`
- `0x0070DDF0` (FUN_) `TechnoClass__GetGattlingValue`
- `0x0070E140` `TechnoClass__GetWeapon`
- `0x006F3330` `TechnoClass__SelectWeaponAgainst`
- `0x00736DF0` `UnitClass__Fire_At_Target`
- `0x004496B0` `BuildingClass` mission-attack helper
- `0x0071AF20` `TemporalClass__InitiateWarp`
- `0x00714000+` `TechnoTypeClass__ReadINI` (string xrefs at `0x71401D..0x71411F`)

**Related docs in `ra2-rust-game-docs/`:**
- `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` (§15 — earlier decay-only writeup of UpdateGattlingStage)
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (§11 — fields at 0x140/0x144)
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` (gattling decay on FIRE_FACING)
- `UNITCLASS_GHIDRA_REPORT.md` (`GattlingCycleCount` at 0x148)
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` (mentions WeaponStages array at +0xCD8)
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` (for follow-up on +0xCC)

**INI files checked:**
- `ini/rulesmd.ini` — `[YTNK]`, `[YAGGUN]` (active YR units)
- `ini/artmd.ini` — no gattling-specific keys
- `ini/rules.ini` / `ini/art.ini` — no gattling units (system unused in base RA2)
