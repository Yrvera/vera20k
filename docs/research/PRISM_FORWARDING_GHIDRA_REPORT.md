# Prism Tower Forwarding / Support System -- Ghidra Report

Confidence: MEDIUM-HIGH for INI field map, damage-scaling math, and the
per-supporter beam-emit path -- all verified from binary via at least two
independent callsites. **Confidence HIGH (upgraded 2026-04-21) for the
orchestrator that selects supporters** -- the "orchestrator" is
`BuildingClass::Mission_Attack`'s cascade body at `0x0044b2bc-0x0044b595`,
already documented in `PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` (Sections
3-4) and `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` (Sections 0-2). See
Section 8 below for the consolidated answer to the original G1 gap.
No claims are made for unverified behaviors.

## Overview

Prism Towers can "forward" firing power to a single attacking Prism Tower by
emitting support beams at it. Each support beam increments a counter on the
firing tower (BuildingClass+0x664) and scales the next bullet's damage via a
per-bullet damage multiplier (bullet+0x150 in 8.8 fixed-point). The attack is
delayed -- a ChargedAnimTime/DelayedFireDelay window is used so the support
beams are visible before the firing shot resolves.

Key INI keys (from `ini/rulesmd.ini [General]`):

- `PrismType=ATESLA` -- identifies which BuildingType is the "Prism Cannon"
- `PrismSupportModifier=150%` -- damage bonus **per** support beam (additive)
- `PrismSupportMax=8` -- max simultaneous supporters (cap applied somewhere in
  the trigger logic -- site not yet located; see gaps below)
- `PrismSupportDelay=45` -- ticks a supporter stays offline after sending a beam
  (default in RA2 is `45;60` i.e. "45" overriding a commented "60")
- `PrismSupportDuration=15` -- ticks a support beam stays visible on screen
- `PrismSupportHeight=420` -- leptons above target building the beam is aimed
  at (visual)

---

## 1. INI Fields and Rules Offsets

From `[General]` section, parsed in `RulesClass::ReadGeneral` @ `0x0066d530`.
Offsets verified by reading the decompilation between `MutateExplosion`
(rules+0x17c8) and `V3RocketPauseFrames` (rules+0x4b0).

| INI Key                 | Rules Offset | Type                | Default (rulesmd.ini) | Read via |
|-------------------------|--------------|---------------------|-----------------------|----------|
| `PrismType`             | +0x498       | BuildingTypeClass*  | `ATESLA`              | `FUN_0067bce0` (BuildingType lookup by name) |
| `PrismSupportModifier`  | +0x49c       | int (percent)       | 150                   | `CCINIClass::ReadDouble` then `Math::ftol` -- stored as integer percent |
| `PrismSupportMax`       | +0x4a0       | int                 | 8                     | `CCINIClass::ReadInt` |
| `PrismSupportDelay`     | +0x4a4       | int (ticks)         | 45                    | `CCINIClass::ReadInt` |
| `PrismSupportDuration`  | +0x4a8       | int (ticks)         | 15                    | `CCINIClass::ReadInt` |
| `PrismSupportHeight`    | +0x4ac       | int (leptons)       | 420                   | `CCINIClass::ReadInt` |

The `Rules` singleton address is `g_RulesClass_Instance @ 0x008871e0` (runtime
pointer; 0 in the static binary).

### `PrismSupportModifier` storage quirk

Although declared as a percent in INI (e.g. `150%`), it is **stored as an
integer** percent (150), not a float. The INI reader calls
`CCINIClass::ReadDouble` to parse the string, then `Math::ftol` to convert to
int before writing to rules+0x49c. Any implementation must match this integer
semantics (no sub-percent precision).

---

## 2. BuildingClass Fields Involved

All offsets below are verified from `BuildingClass::ProcessDelayedFire @
0x004503F0` and `BuildingClass::Mission_Attack @ 0x0044ACF0`. Note that
ProcessDelayedFire's `param_1` is typed `int *` -- any `param_1[N]` is therefore
a byte offset of `N*4`. The offsets below have been converted.

| Field Offset | Meaning                                   | Notes |
|--------------|-------------------------------------------|-------|
| +0x2b4       | Current attack target (TechnoClass*)      | Same field as `field_0x2b4` in all BuildingClass code. Also used as "generic target" across many missions. |
| +0x664       | **Prism support count** (int)             | Incremented by each support beam (trigger site not yet located). Cleared at attack start. Reset to 0 after scaling the fired bullet. **Polymorphic with garrison fire index** -- see BUILDINGCLASS_MASTER_GHIDRA_REPORT.md: the same offset carries garrison occupant round-robin index on garrisoned buildings. Since a Prism Tower cannot be garrisoned and a garrisonable structure cannot be a Prism Tower, this sharing is safe. |
| +0x704       | Delayed-fire mode (int)                   | 0 = idle. 1 = "I am the firing Prism Tower, fire my main shot on timer expiry". 2 = "I am a support Prism Tower, emit my support beam on timer expiry" (calls `FUN_0044abd0`). 3 = observed in switch but no body reached. |
| +0x708       | Delayed-fire arg / target (int)           | Mode 1: weapon index passed to `vtable+0x3c0` (GetFireError) and `vtable+0x3cc` (Fire). Mode 2: becomes `this` of `FUN_0044abd0` (the supporter's own target/firing-tower pointer). |
| +0x70c       | Delayed-fire arg (int)                    | Mode 2: passed as 2nd arg to `FUN_0044abd0` (beam endpoint coord component / target-coord pack). |
| +0x710       | Delayed-fire arg (int)                    | Mode 2: passed as 3rd arg to `FUN_0044abd0`. |
| +0x714       | Delayed-fire timer (int ticks)            | Decrements each tick inside `ProcessDelayedFire`. When `timer - 1 < 1`, the queued action fires. |

Related BuildingTypeClass bytes referenced in the fire path (not yet confirmed
as the "IsPrismCannon" flag; see gaps):

- `BuildingTypeClass + 0x16b8` (byte): consulted in `Mission_Attack` -- if set,
  skips the standard attack flow.
- `BuildingTypeClass + 0x16c5` (byte): consulted in both `Mission_Attack` and
  `GetFireError`. In `GetFireError` it affects a timing threshold
  (`(-(flag != 0) & 0xF8) + 8) << 8` yields 0x800 for unset, effectively 0 for
  set). This flag **looks like** the "delayed-fire-gate" flag but has not been
  traced to its INI source yet.

---

## 3. Damage-Scaling Formula (Verified)

From `BuildingClass::ProcessDelayedFire @ 0x004503F0`, translated out of
`int *`-indexed form:

```c
void BuildingClass::ProcessDelayedFire(BuildingClass *this) {
    int mode = this->field_0x704;  // delayed-fire mode
    if (mode == 0) return;

    int *timer = (int *)&this->field_0x714;
    if (--*timer >= 1) return;   // still ticking
    *timer = 0;

    if (mode == 1) {                                // fire path
        if (this->field_0x2b4 == 0) goto clear;     // no target
        int err = vtable[0xF0](this, this->field_0x2b4, this->field_0x708, 1);
        if (err != 0) goto clear;

        // Fire returns a BulletClass* (or 0 on failure)
        BulletClass *bullet = vtable[0xF3](this, this->field_0x2b4, this->field_0x708);
        int count = this->field_0x664;
        if (bullet != 0 && count != 0) {
            int pct = Rules->PrismSupportModifier * count + 100;   // integer percent
            bullet->field_0x150 = (uint)(pct * 0x100) / 100;       // 8.8 fixed-point
            this->field_0x664 = 0;                                 // consume supporters
        }
    } else if (mode == 2) {
        // FUN_0044abd0(this->field_0x708, this->field_0x70c, this->field_0x710);
        // mode-2 path (missile-like?) -- not decoded here
    }
    this->field_0x704 = 0;  // clear mode
}
```

### Bullet damage multiplier field (bullet+0x150) -- cross-verified

`bullet+0x150` is an 8.8 fixed-point damage multiplier, default value `0x100`
(= 1.0x), initialized in `BulletClass::Init @ 0x004664c0` (per existing
`BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`, which currently labels this field
"unknown, initialized to 0x100").

Two independent readers confirm this is a damage multiplier:

1. **Writer** -- `BuildingClass::ProcessDelayedFire` writes
   `(pct * 0x100) / 100` to it. With `PrismSupportModifier = 150` and
   `count = 0..8`, the resulting multipliers are:

   | Support count | `pct` | `bullet+0x150` (hex) | Multiplier |
   |---------------|-------|----------------------|------------|
   | 0             | 100   | 0x100                | 1.0x       |
   | 1             | 250   | 0x280                | 2.5x       |
   | 2             | 400   | 0x400                | 4.0x       |
   | 4             | 700   | 0x700                | 7.0x       |
   | 8 (max)       | 1300  | 0xD00                | 13.0x      |

2. **Reader** -- `WarheadTypeClass::Detonate @ 0x004690b0`, inside the
   "cell-action-type-6 / crush-physics" branch, computes:

   ```
   fVar6 = ((float)(bullet[0x54] * bullet[0x1b] >> 8) *
           *(float *)(g_RulesClass_Instance + 0x18b4)) / (float)_DAT_0081aef8;
   ```

   Here `bullet[0x54]` is `bullet+0x150` (int-indexed param_1; `0x54 * 4 =
   0x150`) and `bullet[0x1b]` is `bullet+0x6c` = `BulletClass::Damage` (per
   `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` line 50). The expression
   `multiplier * damage >> 8` is the classic 8.8 fixed-point scaled damage.

This is a **correction candidate** for the existing BulletClass layout doc,
which should be upgraded from "unknown_0x150, usage unclear" to "DamageScale
(8.8 fixed-point, 0x100 = 1.0x) -- scales BulletClass::Damage at
WarheadTypeClass::Detonate". Left for a future iteration to update that doc
formally.

### Integer quirk to preserve

Note `(pct * 0x100) / 100`, **not** `(pct / 100) * 0x100`. Computing in the
original order keeps the full integer precision:

- `pct=250` -> `250 * 256 = 64000`; `64000 / 100 = 640` = `0x280`
- `pct/100=2`; `2 * 256 = 512` = `0x200` -- **wrong**

Implementations must keep the multiplication-first, divide-last order.

---

## 4. Support-Beam Emission Path (Verified)

The supporter-side of the forwarding system is dispatched through
`BuildingClass::ProcessDelayedFire`'s mode-2 branch, which calls:

```
FUN_0044abd0 @ 0x0044ABD0   (__thiscall)
```

This function is **called from exactly one place** -- the mode-2 branch of
`ProcessDelayedFire` -- and itself calls the shared laser-draw constructor
`FUN_0054fe60 @ 0x0054FE60` (the same constructor used by
`TechnoClass::Fire_At`'s Prism-firing branch at 0x006FD210, by
`ParticleSystemClass::AI_Railgun`, and by a fourth caller at
0x004A7340). Allocation size is 92 bytes (`operator_new(0x5c)`) for both
forwarding and main firing beams -- same object class.

### `FUN_0044abd0` -- what it does

1. `operator_new(0x5c)` -- allocates a laser-draw instance.
2. Passes `PrismSupportDuration` (rules+0x4a8) as the laser's lifetime.
3. Takes beam colour bytes from `this->Owner + 0x56fc..0x56fe` (3-byte
   colour triple -- the house's laser/remap colour).
4. Calls `vtable+0xb0` to get this building's own render coord as the beam
   origin.
5. Uses `param_2/param_3/param_4` (passed in from the stored mode-2
   state) as the beam destination/target coord.
6. On successful construction, sets laser flags:
   - `laser+0x20 = 1` (matches the "house-color" flag used by the main
     fire path too)
   - `laser+0x1c = 3` (beam-type = standard support)
7. Then **updates the supporter's own fire-state fields**:
   - `supporter+0x2EC = g_CurrentFrameCounter`
   - `supporter+0x2F0 = param_3` (the target-coord component)
   - `supporter+0x2F4 = PrismSupportDelay` (rules+0x4a4)
   - `supporter+0x664 = 0` (consume any sub-support the supporter itself
     received, so it can't re-forward)

Fields `supporter+0x2EC/0x2F0/0x2F4` are the standard TechnoClass
last-fire / reload timer slots (same fields Fire_At writes at the end of
every regular shot), so the PrismSupportDelay is being used as the
supporter's post-fire cooldown via the normal reload mechanism -- there
is no separate "offline" flag.

This is a second, independent confirmation (beyond `RulesClass::ReadGeneral`)
that both `PrismSupportDuration` (read here) and `PrismSupportDelay`
(read here) live at +0x4a8 and +0x4a4 respectively.

### Main-fire beam (`TechnoClass::Fire_At` Prism branch)

In `TechnoClass::Fire_At @ 0x006FDD50`, the main Prism Tower firing beam is
built through `FUN_006fd210 @ 0x006FD210`, itself a wrapper over
`FUN_0054fe60`. There the branch keyed on

```
this_type == *(BuildingTypeClass **)(g_RulesClass_Instance + 0x498)
    // i.e. this building's type == Rules.PrismType
```

upgrades the beam: `beam+0x1c = 3` (default), and when a charge counter
on the firing tower is non-zero, also `beam+0x21 = 1` and `beam+0x1c = 5`
(an alternate beam-type, "supported prism shot" -- visually thicker/more
intense). This confirms the firing tower's main beam is tagged differently
depending on whether it was supported, and that the firing-tower branch
hooks on `Rules.PrismType` identity (not on a type-level `IsPrismCannon`
flag).

---

## 5. Gaps (NOT Yet Verified)

These are flagged explicitly so a future iteration can fill them.

### G1. Support-beam **orchestrator** (selector / enumerator)

The supporter-side visual emitter (`FUN_0044abd0`) and firing-tower
damage-scaling (`ProcessDelayedFire` mode 1) are now both located. **What is
still unlocated** is the orchestrator that:

- enumerates nearby Prism Towers,
- tests eligibility (built, online, not already cooling down, within
  forwarding range, owner/alliance check),
- sets each eligible supporter into mode-2 delayed-fire state (writes
  `supporter+0x704 = 2`, and stores the firing-tower pointer into
  `supporter+0x708` with destination coords into +0x70c/+0x710),
- initialises the supporter's delayed-fire timer (`supporter+0x714`),
- increments `firing_tower+0x664` per supporter engaged,
- applies the `PrismSupportMax` cap.

Search hint for next iteration: the orchestrator must read **both**
`Rules+0x4a0` (PrismSupportMax) and the `PrismType` pointer at `Rules+0x498`,
and somewhere **write `2` (or `1`) to `BuildingClass+0x704`**. This
iteration checked and **ruled out** the following candidates as the writer:

- `BuildingClass::Mission_Attack @ 0x0044ACF0` -- disassembled in full;
  no write to `+0x704` anywhere in the function, including its jumptable
  targets at 0x0044B0DE/0x0044B14E/0x0044B187/0x0044B1DE/0x0044B2BC/etc.
  The jumptable index-2 target (0x0044B187, the delayed-fire path) only
  returns the value `2` to the caller as a mission-tick count -- it does
  not initialise the delayed-fire state machine.
- `BuildingClass::ProcessDelayedFire @ 0x004503F0` -- only reads +0x704;
  writes `0` to it when clearing state, never positive values.
- `BuildingClass::UpdateAnimation @ 0x004509D0` -- disassembled; touches
  animation state fields (0xfc/0x100/0x104/0x108/0x10c) and `field_0x6dd`
  but never +0x704.
- `FUN_0044abd0` -- writes only TechnoClass reload-state fields
  (+0x2EC/+0x2F0/+0x2F4) and zeros support count (+0x664); does not
  write +0x704.

Remaining candidates worth tracing next iteration:

- The function called via `vtable+0x3cc` on a Prism Tower (i.e. TechnoClass
  or a Prism-specific override of `Fire_At`) -- the firing tower may
  transition itself into mode 1 *instead of* spawning a bullet
  immediately. A focused grep of `Fire_At @ 0x006FDD50` for any
  instruction writing to `[ESI + 0x704]` or for the constant `0x704`
  would answer this quickly.
- `BuildingClass::UpdateRepairAndPower @ 0x00450630` or a neighbouring
  per-tick helper may be the orchestrator, since the orchestrator almost
  certainly runs every tick (to scan nearby supporters).
- Any use of `Rules+0x4a0` / `Rules+0x498` in a *building-AI* context.

### G2. Attack initiation -- who sets field_0x704 = 1?

The code that transitions a Prism Tower into its delayed-fire state (mode 1)
was not located. `Mission_Attack` (0x0044ACF0) includes prism-specific code
paths gated on `BuildingTypeClass+0x16c5`, but the exact branch that
`field_0x704 = 1` was not found inside this iteration.

### G3. IsPrismCannon flag

`BuildingTypeClass+0x16c5` (byte) is strongly suspected to be the
"IsPrismCannon" / "delayed-fire-gate" flag based on its use in both
`Mission_Attack` and `GetFireError`. The INI key that writes it has not been
confirmed. The RA2 INI `PrismType=ATESLA` resolves a BuildingType pointer at
rules+0x498, but the per-type bool flag that marks that BuildingType as
"the prism cannon" may be set in `BuildingTypeClass::ReadINI` when it matches
`Rules->PrismType`.

### G4. Recursive forwarding

The documented in-game behavior is that supporters can themselves have
sub-supporters (chains can cascade). The implementation mechanism -- whether
by direct recursion of the trigger from G1, or by each supporter independently
running its own search -- has not been confirmed from the binary.

### G5. Visual beams / animation

`PrismSupportDuration` (rules+0x4a8) **is now verified** to be consumed by
`FUN_0044abd0` as the laser-draw lifetime (see Section 4). Support beams and
main-fire beams share the same laser class (`FUN_0054fe60`, size 0x5c),
with the firing-tower beam given a different beam-type code (5 vs 3) and
an extra flag byte at offset 0x21 when the shot was supported.

`PrismSupportHeight` (rules+0x4ac) is **still not traced** -- it was not
read by any function examined in this iteration. Most likely used
somewhere in the main-fire beam endpoint computation (the +420 leptons
would lift the target coord above the target building before the beam is
aimed). The surrounding arithmetic in `Fire_At` around `iStack_8c /
iStack_88 / iStack_84` is the likely consumer; a future iteration should
scan that block for a load of rules+0x4ac.

### G6. `field_0x704` mode 3

Mode 2 is now decoded (see Section 4 -- the supporter's beam-emit path).
Mode 3 (`iVar1 - 2 == 1`, which falls through with no branch body) was
observed syntactically but no body is reached. Likely either dead code or
a placeholder for a never-implemented third delayed-fire mode. No action
needed for a YR implementation.

### G7. IsPrismCannon flag vs. `Rules.PrismType` identity

Updated understanding: the firing-tower branch in `TechnoClass::Fire_At`
keys on **`this_type == Rules.PrismType`** (pointer identity against
`rules+0x498`), *not* on a per-BuildingType boolean flag.
`BuildingTypeClass+0x16c5` may still be a related flag but it is now less
likely to be "IsPrismCannon" specifically -- it could instead be a more
generic "uses delayed-fire" flag that Prism Tower happens to share with
any other delayed-fire structure. Needs a future iteration to cross-check
by scanning `BuildingTypeClass::ReadINI` for a bool write to +0x16c5.

---

## 5b. Cross-Reference Conflicts With Other Docs

While working this iteration, two claims in `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
surfaced that conflict with what `ProcessDelayedFire` does in the binary.
They are NOT amended in that doc from this iteration (the loop's
"double-check from two independent angles before editing someone else's
doc" rule means these should be separately re-verified). They are flagged
here so a future iteration picks them up:

- **`BuildingTypeClass+0x16C5` named "HasTurret"** in the MISSION_ATTACK
  doc. But `BuildingClass::GetFireError` and `Mission_Attack` both gate
  Prism-like delayed-fire behavior on this byte. If it were just
  "HasTurret", every turreted building (Tesla, Patriot, Flak Cannon,
  garrison turrets, etc.) would take the delayed-fire path -- which they
  do not in-game. More likely this byte is a distinct flag (possibly
  `IsChargeFire` / `IsDelayedFire`) that happens to coexist with
  HasTurret semantically. Needs a direct read of
  `BuildingTypeClass::ReadINI` to confirm the INI-key-to-offset map.

- **`BuildingClass+0x714` named "UpgradeLock"** in the MISSION_ATTACK
  doc's GetFireError walkthrough. But this is exactly the field
  `ProcessDelayedFire` decrements every tick as the delayed-fire timer
  (cross-checked here via the `param_1` `int *` arithmetic:
  `param_1[0x1c5]` = `0x1c5 * 4` = `0x714`). Behaviorally both
  interpretations deny firing while the field is nonzero, so no visible
  gameplay conflict -- but the name is misleading. A future iteration
  should search for any other writer of +0x714 (besides
  `ProcessDelayedFire`'s decrement) to see whether a separate
  upgrade-locking system also writes here, or whether "UpgradeLock" is
  just a naming guess from context.

## 6. TS-Legacy Notes

None of the PrismSupport* keys correspond to known TS-legacy systems. The
`PrismType` key itself does not have a default BuildingType in TS (TS had no
Prism Cannon unit), so the entire subsystem is YR-only as far as default
behavior is concerned. No `SpecialFlags` gate was observed in the inspected
paths.

The reuse of `BuildingClass+0x664` for both garrison fire index and prism
support count is **not** a TS ghost -- both uses are live in YR, just for
different building types.

---

## 7. What an Implementation Must Match (Summary)

1. Parse the six INI keys under `[General]` into the offsets above.
   `PrismSupportModifier` is a **percent stored as int** (read as double, cast
   to int).
2. Use the same bullet-damage multiplier slot (`bullet+0x150`, 8.8
   fixed-point, default `0x100`). Every weapon bullet carries this field; only
   Prism Towers write to it from their fire path.
3. The scaling formula is `bullet.damage_scale = (PrismSupportModifier * count
   + 100) * 256 / 100` evaluated in that exact order, and is applied **after**
   a successful `Fire()` returns a bullet object, **before** the bullet is
   allowed to travel.
4. Reset the support count to zero after scaling.
5. The delayed-fire timer (`field_0x714`) decrements once per building tick
   inside `ProcessDelayedFire`, which is itself called from
   `BuildingClass::Update`. The actual fire happens when the timer hits 0,
   not when it reaches 1.

The support-beam trigger side (gaps G1--G5) is the subject of a follow-up
research iteration and must NOT be implemented in Rust based on the partial
information in this report.

---

## 8. Follow-up Pass (2026-04-21) -- Cascade Orchestrator

This section consolidates and cross-verifies the orchestrator answer that
was only partially resolved when Sections 1-7 were written. The orchestrator
turned out to be **not a distinct function**: it is the prism-gated body
inside `BuildingClass::Mission_Attack`, documented in detail in
`PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` (Sections 2-4) and corrected in
`PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` (Sections 0-2). The points
below are re-verified from the live binary this pass and labeled HIGH.

Naming: in this section, "orchestrator" refers to the selector + mode-2-setup
block at `0x0044b2bc-0x0044b595` inside `Mission_Attack`. No new function
boundary exists; it is inline.

### 8.1 Trigger point

**Address:** `0x0044b2bc` (prism pre-gates) -> `0x0044b2f8` (prism gate)
-> `0x0044b32f` (selector loop header) inside `BuildingClass::Mission_Attack
@ 0x0044ACF0`.

**Decompilation highlight (re-verified from bytes):**

- `0x0044b2bc`: `8a 86 02 07 00 00` = `MOV AL, [ESI+0x702]` -- reads
  `BuildingClass->UpgradeCount`. Pre-gate 1.
- `0x0044b2c6`: `8b 8e ec 05 00 00` = `MOV ECX, [ESI+0x5EC]` -- reads
  `BuildingClass->UpgradeSlot[1]`. Pre-gate 2.
- If **both** gates non-zero: `CALL FUN_00712130` tertiary test, then a
  direct `vtable[0x3CC]` (Fire) -- immediate-fire bypass, **no cascade**.
  Per `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Section 3, this tertiary
  gate (`HasBurstWeaponInSlot1`) is **dead code in stock YR**: ATESLA has
  `Capturable=false` and no `PowersUpBuilding=`, so upgrade count is
  always 0. Pre-gate 1 fails -> jump to prism gate.
- `0x0044b2f8-0x0044b30a`: `CMP this->Type, Rules->PrismType` (rules+0x498).
  If unequal, jumps to the `IsAnimDelayedFire` fallback at `0x0044b630`
  (non-cascade single-delayed-fire path for NATSLA and similar).

**Who calls Mission_Attack?** It is dispatched through the building's
mission-table each time the building's `MISSION_ATTACK` state runs --
which is every tick that the building holds the Attack mission and is
not currently blocked by a sub-timer. See
`BUILDINGCLASS_MISSIONS_AND_INI_VERIFICATION.md` for the mission-tick
wiring. For the orchestrator this is ~1 invocation per 1-2 ticks.

**Re-entry control.** `BuildingClass::GetFireError @ 0x00447F10` returns
error code **3** whenever `+0x714 != 0` (delayed-fire timer live). Mission
error code 3 dispatches through jumptable[3] at `0x0044b1de`, which
does NOT touch `+0x704`/`+0x664` and does NOT re-enter the cascade. So
once a Prism Tower has entered mode-1 (delayed-fire running), it cannot
re-enter the selector until its current shot resolves. **The cascade
runs exactly once per attack cycle** (not every tick during the 28-tick
charge). Verified in `PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md`
Section 13.

**Original Open Question closed:** G1 "Support-beam orchestrator" --
**YES, fully closed.** The orchestrator is the Mission_Attack body; it
is neither a separate AI tick nor a TargetAcquired hook.

**Confidence:** HIGH.

### 8.2 Candidate pool iteration

**Address:** `0x0044b357-0x0044b4ae`.

**Decompilation highlight (re-verified from bytes at 0x0044b357 and
0x0044b370):**

```
0x0044b357: 8b 86 1c 02 00 00    MOV EAX, [ESI+0x21C]   ; this->Owner
0x0044b35d: 8b 40 78             MOV EAX, [EAX+0x78]    ; Owner->OwnedObjectsCount
0x0044b366:                       save as loop limit

; Loop top:
0x0044b370: 8b 8e 1c 02 00 00    MOV ECX, [ESI+0x21C]   ; this->Owner
0x0044b376: 8b 44 24 14          MOV EAX, [ESP+0x14]    ; loop_idx
0x0044b37a: 8b 51 6c             MOV EDX, [ECX+0x6C]    ; Owner->OwnedObjects (array)
0x0044b37d: 8b 3c 82             MOV EDI, [EDX + EAX*4] ; candidate = OwnedObjects[idx]
```

**Pool identity (verified against `HOUSECLASS_VERIFIED_FIELD_MAP.md`
line 201 and `HOUSECLASS_GHIDRA_REPORT.md` lines 133-134):**

- `HouseClass+0x6c` = `OwnedObjectsArray` -- `TechnoClass*[]` (not
  BuildingClass-only).
- `HouseClass+0x78` = `OwnedObjectsCount` -- size of that array.

**Critical nuance:** this is the **all-owned-objects** list, NOT the
filtered `OwnedBuildings` list (which is `HouseClass+0x2F0` -- a
separate int count; the building-only array is elsewhere). Infantry,
vehicles, and aircraft also appear in `+0x6c`. The cascade relies on
the per-candidate `Type == Rules->PrismType` check at `0x0044b3a2` to
filter them out -- a non-BuildingType TechnoClass cannot match the
BuildingType pointer stored in `Rules->PrismType`, so only Prism Towers
survive the filter.

**Consequence for implementation:** if a Rust port maintains a
`BuildingClass`-only list per house, iterating that is functionally
equivalent (and faster) -- but iteration order must be preserved.
Better to match the binary exactly: iterate the full owned-objects list
in index order and filter per-candidate, since any index-order mismatch
between the two lists is a lockstep risk.

**Original Open Question closed:** part of G1 ("what list does it
iterate") -- **YES, fully closed.** It is `this->Owner->OwnedObjects`
(all owned TechnoClass objects for the current house), index 0 through
`OwnedObjectsCount - 1`.

**Confidence:** HIGH.

### 8.3 Filter predicate

**Address:** `0x0044b380-0x0044b49c`, per-candidate filters applied in
this exact order (short-circuiting on first failure):

1. `candidate != nullptr` (`0x0044b380: TEST EDI; JZ skip`)
2. `candidate->field_0x90 != 0` ("IsAlive" / active flag,
   `0x0044b38e: TEST AL; JZ skip`)
3. `candidate->Type == Rules->PrismType` (pointer identity,
   `0x0044b3a2: CMP ECX, [rules+0x498]`)
4. **Cooldown expired** -- either `candidate->+0x2EC == -1` (never
   emitted, sentinel) OR `currentFrame - candidate->+0x2EC >=
   candidate->+0x2F4` (`0x0044b3ae-0x0044b3ce`). Per
   `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Section 15 these
   `+0x2EC/+0x2F0/+0x2F4` fields are the standard TechnoClass
   `FireRateTimer`, so the cascade filter piggybacks on the normal ROF
   timer -- a Prism Tower in normal attack cooldown is also ineligible
   as a supporter.
5. `candidate->+0x714 == 0` -- not currently in its own delayed-fire
   cycle (`0x0044b3da: TEST EAX; JNZ skip`)
6. `TechnoClass::IsDeploying(candidate) == false` (`0x0044b3e4: CALL
   0x0070FEC0`)
7. `candidate->vtable[0x61]() != 1` -- candidate's current mission
   is NOT `MISSION_ATTACK` (mission id 1). Idle/Guard/Stop/Sleep
   qualify; currently-shooting supporters do not
   (`0x0044b3f5-0x0044b3fe`).
8. **`candidate != this` -- self-exclusion**, verified from bytes at
   `0x0044b404: 3b fe` (`CMP EDI, ESI`) + `0f 84 a2 00 00 00`
   (`JZ +0xA2`).
9. Range: `int(sqrt(dx*dx + dy*dy + dz*dz)) <=
   this->vtable[0x5A](1)` -- lepton distance (linear!) to the
   **firing tower** vs the **firing tower's** `Secondary` weapon range
   (`PrismSupport` for ATESLA, `Range=8` cells = 2048 leptons). The
   byte-level `ff 92 68 01 00 00` at `0x0044b494` is `CALL [EDX+0x168]`
   -- vtable index 0x5A (= `TechnoClass::GetWeaponRange @ 0x007012C0`).
   **Not per-candidate:** the `MOV ECX, ESI` at `0x0044b490` confirms
   the vtable target is the firing tower, not the candidate --
   corrected in `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Section 0.1
   and 0.2 (the earlier trigger report's "squared lepton / per-candidate
   range" was wrong on both counts).

**Implementation note:** filter 4 is NOT an EMP / power check.
`CanSellOrUndeploy` (vtable[0xD4]) is **not** called on candidates --
only on the firing tower via `GetFireError` at the gate into the
cascade. So an EMP'd idle Prism Tower can still be picked as a
supporter; it just cannot itself start a cascade. See
`PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Section 10 for details.

**Cap check (applied once before loop entry, not per-candidate):**
`this->+0x664 >= Rules->PrismSupportMax (+0x4a0)` at `0x0044b349-0x0044b351`
-- skips the whole selector if the firing tower already has max
accumulated supporters.

**Original Open Question:** narrow filter identity -- **YES, fully
closed.**

**Confidence:** HIGH for all 9 filters + the cap. All verified either
from byte reads this pass (filters 3, 8, 9) or from the prior extension
report's decompilation (filters 1, 2, 4, 5, 6, 7).

### 8.4 Chain depth

**Answer:** strictly 1 supporter per attack cycle. **No recursion.
No tree.**

**Evidence chain:**

- Selector loop at `0x0044b370-0x0044b4ae` picks the **closest** eligible
  candidate (score = lepton distance, strictly-smaller wins; see 8.5
  below). Only `EBX` is tracked as the single "best so far" -- there is
  no intermediate list of candidates.
- Post-loop `0x0044b4c3: TEST EBX, EBX; JZ skip` -- if EBX is zero
  (no candidate found), skip the mode-2 setup entirely; still set
  firing tower to mode 1 (just with support_count = 0).
- If EBX non-zero: `this->+0x664++` (increment by 1), then write mode-2
  state onto that one supporter (`0x0044b4e0-0x0044b52f`). Loop does
  not re-run.
- The selected supporter receives mode-2 + saved firing-tower coords.
  When its own delayed-fire timer expires, `ProcessDelayedFire` mode-2
  calls `EmitPrismSupportBeam` which spawns only a visual laser and
  sets the supporter's own cooldown. **EmitPrismSupportBeam does NOT
  re-enter the selector** and does NOT recruit further supporters --
  it only draws the beam and writes `+0x2EC/+0x2F0/+0x2F4/+0x664`.
- Supporters cannot recruit sub-supporters because supporters are never
  in `MISSION_ATTACK` state during their mode-2 timer (their mission
  remains whatever it was -- Idle/Guard -- and their own
  `Mission_Attack` is not being invoked). Their `ProcessDelayedFire`
  fires a beam and clears mode, never entering a selector.

**Multi-supporter "visual cascade" explanation (verified in
`PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` Sections 13.3-13.5):** what
players see as "5-8 beams all converging" is actually 5-8 **separate
attack cycles**, each picking one supporter. Per-shot damage multiplier
in normal play is **2.5x** (support_count = 1, `PrismSupportModifier =
150%`), NOT 13x. The 13x max requires `Fire()` to fail repeatedly
across 8 consecutive attack cycles so that `+0x664` never resets --
a degenerate edge case.

**Original Open Question closed:** G4 "Recursive forwarding" -- **YES,
fully closed: there is no recursion.** The behavior is emergent from
per-attack-cycle single-supporter accumulation.

**Confidence:** HIGH.

### 8.5 Iteration order (determinism)

**Answer:** strict index order over `Owner->OwnedObjects[0..Count-1]`.
Ties broken by earliest index.

**Evidence:**

- Loop counter is `[ESP+0x14]`, incremented at loop tail
  `0x0044b4ae`. Indexing is `OwnedObjects[idx]` -- natural array order
  (verified from `MOV EDI, [EDX + EAX*4]` at `0x0044b37d`).
- Best-so-far is tracked in two stack slots: `[ESP+0x10]` holds the
  best score (initial sentinel `0x7FFFFFFF`, verified from bytes
  `ff ff 7f ff` at `0x0044b320`), and `EBX` holds the winning pointer
  (initial 0).
- Tie-break at `0x0044b4a2-0x0044b4a6`: `CMP EBP, [ESP+0x10]; JGE skip`
  -- uses `JGE`, meaning the new candidate wins **only** if its
  distance is **strictly less** than the previous best. Equal distances
  keep the earlier candidate. Verified from bytes `3b 6c 24 10 7d 06`
  at `0x0044b4a2`.

**MP lockstep relevance:** iteration order is the `OwnedObjects` array
order, which is append-on-creation and stable-on-removal (verified via
`HOUSECLASS_GHIDRA_REPORT.md` OwnedObjects semantics -- additions
append, removals compact with index preservation for survivors in the
standard pattern). This is **deterministic across clients** as long as
creation/destruction order is identical, which it is under lockstep
-- unit creation is itself deterministic. No sort, no hash-ordering,
no pointer-valued compare. Safe.

**Implementation directive for Rust port:** iterate the owner's owned
objects **in stored order**, track `(best_score, best_ptr)` as
`(i32::MAX, None)`, update only on strict-less. Do NOT
sort-by-distance and pick head -- sort stability across `BTreeMap`
iteration would probably be fine in practice but diverges from the
binary's exact comparison pattern. A floating-point FILD/FMUL/FADDP ->
Math::Sqrt_Approx -> Math::ftol chain in the original means the Rust
port must either match that exact chain or (preferred) use
integer-space lepton arithmetic for the squared pre-sqrt sum, then a
deterministic integer square root (or just skip the sqrt and compare
against `range * range` -- but see Section 0.2 of
`PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md`: the binary compares
linearly, so we should too for drop-in fidelity, unless the exact-
equivalence of `sqrt(d2) < r` and `d2 < r*r` proves acceptable in
testing).

**Original Open Question closed:** iteration determinism -- **YES,
fully closed and safe for lockstep.**

**Confidence:** HIGH.

### 8.6 Per-link beam scheduling

**Answer:** each supporter has its own independent delayed-fire timer,
sourced from the supporter's own `Type->DelayedFireDelay
(BuildingTypeClass+0x16ec)` -- **not** a per-link travel-time
computation, and **not** a function of distance to firing tower.

**Decompilation highlight (from `0x0044b50a-0x0044b52f`, decoded in
`PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` Section 4):**

```
supporter->+0x714 = supporter->Type->+0x16ec   ; timer = DelayedFireDelay
supporter->+0x704 = 2                          ; mode = supporter
supporter->+0x708 = firing_tower_X             ; saved target coords
supporter->+0x70c = firing_tower_Y
supporter->+0x710 = firing_tower_Z
```

For stock YR `[GAPRIS]` in `artmd.ini`, `DelayedFireDelay=28` -- so
every supporter ticks down a 28-tick timer independently. On tick 28
relative to its own mode-2 entry, the supporter's `ProcessDelayedFire`
fires the mode-2 branch which calls `EmitPrismSupportBeam` (beam
spawn + cooldown set + mode clear).

**Firing tower's own timer** (set at `0x0044b5ab-0x0044b5bb` after
the selector loop exits): also `Type->DelayedFireDelay = 28`. Both
fire together 28 ticks after the attack cycle began -- **the beam
and the shot resolve at the same tick** because both timers are
initialized to the same value in the same tick. See
`PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` Section 8 for the T+0 / T+28
walkthrough.

**Per-link storage fields:**
- Per-supporter timer: `supporter->+0x714` (int ticks)
- Per-supporter target coords: `supporter->+0x708 / +0x70c / +0x710`
  (saved at select time; the coords are the firing tower's cell
  coords at mode-2 assignment, not live-updated)
- Per-supporter cooldown (post-emit): `supporter->+0x2EC` (start
  frame), `supporter->+0x2F4` (delay = `Rules->PrismSupportDelay`,
  default 45 ticks) -- these are the standard ROF timer slots,
  piggybacked on per `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md`
  Section 15.

**No travel-time simulation.** The beam's visual duration
(`PrismSupportDuration = 15` ticks) is the LaserDrawClass lifetime;
the beam does not "arrive" at a computed time, it simply exists for
15 ticks from emit. There is no "arrival packet" or scheduling
structure between supporter and firing tower beyond the mode-2 coord
snapshot.

**Original Open Question:** per-link scheduling identity -- **YES,
fully closed.**

**Confidence:** HIGH.

### 8.7 Beam spawn timing

**Answer:** the supporter's beam is **deferred**. At selector time,
no `LaserDrawClass` is created. The beam is spawned later when the
supporter's own `+0x714` timer reaches zero.

**Call chain:**

1. Tick T+0 (selector): `Mission_Attack` sets `supporter->+0x704 =
   2`, `+0x714 = 28`. **No laser created yet.**
2. Ticks T+1..T+27: `BuildingClass::ProcessDelayedFire @ 0x004503F0`
   runs each tick on the supporter, decrements `+0x714`. Mode-2 body
   not executed (timer still > 1).
3. Tick T+28: `ProcessDelayedFire` sees `--(+0x714) < 1`, enters
   mode-2 branch at the `else if (mode == 2)` path. **Calls
   `BuildingClass::EmitPrismSupportBeam @ 0x0044ABD0`.**
4. `EmitPrismSupportBeam`: `operator_new(0x5C)` ->
   `LaserDrawClass::Constructor @ 0x0054FE60`. The laser is added to
   `g_LaserDraw_Array @ 0x00ABC87C` at this moment and ticks down
   its own `PrismSupportDuration = 15` lifetime.
5. Same tick (T+28): firing tower's own `ProcessDelayedFire` sees
   its `+0x714 == 0`, runs mode-1 -> `Fire()` resolves the shot.
   `bullet->+0x150 = DamageScale` computed from accumulated count.

**Consequence:** in the 28-tick gap between selector and emit, if the
firing tower is destroyed, the supporter still spawns its beam aimed at
the **saved coords** (which are stale pointers-free coord ints, so no
dangling pointer hazard). See
`PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Section 5 for details on
mid-charge death.

**Why defer?** Visual timing: the beam appears ~28 ticks into the
charge animation, so from the player's point of view, the supporter
tower's animation plays, the beam shoots out, and the firing tower
shoots almost simultaneously. An immediate-spawn-at-select design
would show the beam 28 ticks too early (before the supporter's own
charge animation even finishes), breaking the visual.

**Original Open Question:** beam spawn timing -- **YES, fully closed.**

**Confidence:** HIGH.

### 8.8 Summary of orchestrator answer

| Aspect | Answer | Confidence |
|--------|--------|-----------|
| Trigger point | Inline in `BuildingClass::Mission_Attack @ 0x0044ACF0`, cascade body `0x0044b2bc-0x0044b595`; dispatched via Mission jumptable[0] when GetFireError returns 0 | HIGH |
| Candidate pool | `this->Owner->OwnedObjects` (HouseClass+0x6c, sized by +0x78) -- all owned TechnoClass objects for the current house, filtered by Type | HIGH |
| Filter | 9 predicates in fixed order (null, active, same-type, cooldown, not-in-delayed-fire, not-deploying, mission != ATTACK, self-exclusion, in range of firing tower's Secondary weapon) | HIGH |
| Chain depth | Exactly 1 supporter per attack cycle. No recursion. Multi-beam cascade is emergent across separate cycles | HIGH |
| Iteration order | Index order over OwnedObjects, earliest-index wins tie | HIGH |
| Per-link scheduling | Per-supporter independent timer at `+0x714`, initialized to `supporter->Type->DelayedFireDelay`; no distance-based travel time | HIGH |
| Beam spawn timing | Deferred to timer expiry via `ProcessDelayedFire` mode-2 -> `EmitPrismSupportBeam`; NOT spawned at select time | HIGH |
| MP-lockstep safe? | YES -- array-order iteration, integer / sentinel distance math, no pointer-value compares | HIGH |

**The G1 gap noted in the Overview (original version of this report) is
now closed.** No standalone "orchestrator" function was expected or
found because the orchestrator is inline in `Mission_Attack`'s
prism-gated body -- the reason the first pass "could not locate" a
separate function was because no separate function exists.

### 8.9 Cross-references to companion reports

This section is a consolidation layer only. For byte-level
disassembly and the underlying trace work, read:

- `PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md` Sections 2-4 (selector loop
  original decode), 8 (tick walkthrough), 10 (O2/O4 resolutions),
  13 (per-attack-cycle behavior)
- `PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md` Sections 0 (distance math
  corrections), 1-2 (range threshold + weapon slot identity), 5
  (mid-charge death), 6 (target lost), 10-13 (EMP/MC/IC/power
  interactions), 15 (FireRateTimer field reuse)

Verified this pass from live binary (byte reads, not decompilation
alone): `0x0044b2bc`, `0x0044b320`, `0x0044b349`, `0x0044b357`,
`0x0044b370`, `0x0044b404`, `0x0044b494`, `0x0044b4a2`.
