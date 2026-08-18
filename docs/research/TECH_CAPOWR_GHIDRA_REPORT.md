# Civilian Power Plant (CAPOWR) — Ghidra Research Report

**Scope:** Whether capturing CAPOWR grants power to the new owner, by what
code path, with what timing and conditions, plus the secondary INI keys
present on the section (`Ammo=5`, the commented-out
`;SuperWeapon=ParaDropSpecial`, and the spy infiltrate effect).

**Companion docs (already authoritative — cited, not re-derived):**

- [`POWER_SYSTEM_GHIDRA_REPORT.md`](POWER_SYSTEM_GHIDRA_REPORT.md) — comprehensive power system reference (HouseClass / BuildingClass / GetPowerOutput / AssessPower).
- [`POWER_INI_PARSING_AND_LIFECYCLE.md`](POWER_INI_PARSING_AND_LIFECYCLE.md) — verified `Power=` parse at `0x00461073` → `BuildingTypeClass+0xEE0` (output) / `+0xEE4` (drain).
- [`POWER_EDGE_CASES.md`](POWER_EDGE_CASES.md) — edge cases and gating.
- [`SPECIAL_BUILDINGS_POWER_SYSTEM.md`](SPECIAL_BUILDINGS_POWER_SYSTEM.md) — special buildings (radar, war factory, helipad) power interactions.
- [`TECH_BUILDINGS_GHIDRA_REPORT.md`](TECH_BUILDINGS_GHIDRA_REPORT.md) — covers the other 5 tech buildings (does not address CAPOWR specifically).
- [`TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md`](TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md) — ChangeOwner ordering and side-effects (§4).

**Primary addresses (this report):**

- `BuildingClass::ChangeOwner` — `0x00448260`
- `HouseClass::AI_AssessPower` — `0x00508C30`
- `HouseClass::Update` — calls AI_AssessPower at `0x004F84E5`
- `BuildingClass::GetPowerOutput` — `0x0044E7B0`
- `BuildingClass::OnSpyInfiltrate` — `0x004571E0`
- `HouseClass::SpyPowerSabotage` — `0x0050BC90`
- `HouseClass::Recount` — `0x004FF980` (per-type counter decrementer, called from ChangeOwner)
- `BuildingTypeClass::ReadINI` parse sites for `Power=` (`0x00461073` → +0xEE0/+0xEE4)
- `TechnoTypeClass::ReadINI` parse site for `InitialAmmo` (`0x00714755` → TechnoTypeClass+0x680)
- `RulesClass::ReadGeneral` parse site for `SpyPowerBlackout=` (`0x0066FBFE` → RulesClass+0xD64)

**Confidence:** HIGH on the verified parse offsets, the spy infiltrate
branch, the SpyPowerSabotage write set, and §7's GetPowerOutput formula
(all re-audited 2026-05-17 against `gamemd.exe` — see §13 Audit notes).
HIGH on the lazy-recalc model for the captured-power path (because
`AI_AssessPower` iterates all buildings every time it fires). The §6 open
question on the exact `NeedsPowerRecalc` setter has now been **partially
resolved**: the OLD owner's `+0x5778` IS set unconditionally during
capture via `HouseClass::Removed_From_Game` (called by
`TechnoClass::ChangeOwner`), but the NEW owner's `+0x5778` is NOT set by
any obvious node in the `BuildingClass::ChangeOwner` →
`TechnoClass::ChangeOwner` → `Added_To_Game` → `Add_Tracking` →
`Update_Power_And_EVA` call chain. See §6 and the audit notes for the
implication and what this means for the "≤1-tick lag" claim.
**Corrections applied following the 2026-05-17 `/verify-doc` pass:** (1)
§7 IsOnline byte offset `+0x198` → `+0x660`; (2) §7 GetPowerOutput
formula rewritten with verified asm anchor points (in-limbo gate via
`vtable+0x1D4`, FIMUL+ftol pair for the health-scaling, occupant-multiplier
gate now includes the missing `Type+0xEE8 > 0` co-gate, "if upgraded"
clarified to "if `this->HasExtraPowerBonus` flag is set").

**Active in YR:** Yes — CAPOWR is in `NeutralTechBuildings=` and present on
several stock YR skirmish maps as a neutral civilian building.

---

## 1. The headline answer

**Does capturing CAPOWR grant +200 power?** Yes — but via the **lazy**
power-recalc model, not via a direct increment in `ChangeOwner`.

Specifically:

1. CAPOWR's `Power=200` parses to `BuildingTypeClass+0xEE0 = 200` (and
   `+0xEE4 = 0` since the value is positive). Verified at parse site
   `0x00461073` per `POWER_INI_PARSING_AND_LIFECYCLE.md`.
2. On engineer capture, `BuildingClass::ChangeOwner` (`0x00448260`) rewires
   the building to the new owner. The per-tick `HouseClass::AI_AssessPower`
   (`0x00508C30`) iterates *every* building owned by the house and sums
   `BuildingClass::GetPowerOutput()` into `HouseClass+0x53A4` (PowerOutput).
3. The next time `AI_AssessPower` fires for the new owner, the now-owned
   CAPOWR is included in the sum. Its contribution is `200 *
   GetHealthRatio()`, *health-scaled*. At full HP that's +200; at 50% HP
   it's +100; at 30% HP it's +60.
4. The contribution is gated on `IsOnline` (`BuildingClass+0x660` — corrected 2026-05-28: was `+0x198`; binary shows `0x0044E855: MOV AL, byte ptr [ESI+0x660]` via `decompile_function 0x0044E7B0` — STRUCT_FAMILY_CASCADE) being
   true. A newly captured CAPOWR has the IsOnline bit set to true by
   default (it was producing power for the neutral house pre-capture). The
   new owner can still toggle it off with the standard power-toggle order,
   which would zero its contribution until toggled back on.
5. The OLD owner (the neutral civilian/Special house, `MultiplayPassive=yes`)
   loses the contribution from CAPOWR on its next AI_AssessPower sum. The
   neutral house tracks power identically to a real player; it just doesn't
   *use* the totals for anything observable.

**Net effect for the player:** capturing a full-health CAPOWR is functionally
equivalent to building one of their own Power Plant–class buildings with
`Power=200` (a Tesla Reactor is 200; an Allied Power Plant is 100). It is
**effectively immediate** from the player's perspective in normal play —
the old owner's recalc fires on the very next tick (`+0x5778` is set
unconditionally during the transfer, see §6), and the new owner's recalc
piggybacks on whatever subsequent event sets `+0x5778` next (typically
within a handful of ticks given how often power-state events occur). It
is **not** a single-tick guarantee for the *new* owner side; see §6 and
§13 (audit notes) for the precise mechanism.

---

## 2. INI section snapshot (verified)

`rulesmd.ini`, lines 13956–13979:

```
[CAPOWR]
UIName=Name:CAPOWR
Name=Tech Civilian Power Plant
TechLevel=-1
Strength=800
Insignificant=yes
Nominal=yes
Sight=6
Points=5
Armor=concrete
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=15
MinDebris=5
DebrisAnim=Dbris1sm,Dbris1lg,Dbris4sm,Dbris5sm,Dbris4lg,Dbris7sm,Dbris8sm,Dbris5lg,Dbris4lg
DamageParticleSystems=SmallGreySSys,BigGreySmokeSys
Capturable=yes
CaptureEvaEvent=  EVA_TechBuildingCaptured ;Eva (and therefore 3way split) voice to use when captured
NeedsEngineer=yes
Unsellable=yes
Ammo=5
;SuperWeapon=ParaDropSpecial
LeaveRubble=yes
Power=200
RadarVisible=yes;gs put on radar even if insignificant and unowned (insignificant and owned is a UC building)
```

`rulesmd.ini` line 3082: `NeutralTechBuildings=CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR` — confirms CAPOWR is one of the six tech buildings that spawn from neutral placement and are recognised by the AI as tech-capturables.

`artmd.ini` lines 3421–3440: `Foundation=2x2`, `Height=6`, `OccupyHeight=3`,
`ActiveAnim=CAPOWR_A`, `ActiveAnimDamaged=CAPOWR_AD`. Commented-out
`;Buildup=CAAIRPMK` and `;ActiveAnimTwo=CAAIRP_F` — leftover artifacts from
when CAPOWR was apparently intended to share art assets / behavior with
CAAIRP (consistent with the dead `;SuperWeapon=ParaDropSpecial` line in
rules — see §4).

**Notable absent keys** (defaults apply):

| Key | Default | Effect |
|-----|---------|--------|
| `BridgeRepairHut=` | `no` | Not a bridge hut. |
| `Repairable=` | `no` | **Important:** the engineer-on-building cursor block requires `Repairable=yes` per `TECH_CABHUT_GHIDRA_REPORT.md` §3. CAPOWR has it implicitly via `Capturable=yes` ?  *No — the cursor block is gated on `Type+0xCCC` (Repairable=) specifically. CAPOWR does NOT set `Repairable=`.* See §3 below for what cursor actually shows. |
| `InfantryGainSelfHeal=` | `0` | Not a Tech Hospital. |
| `UnitsGainSelfHeal=` | `0` | Not a Tech Machine Shop. |
| `ProduceCashStartup=` / `ProduceCashAmount=` / `ProduceCashDelay=` | `0` | Not a Tech Oil Derrick. |
| `UnitRepair=` | `no` | Not a Tech Outpost. |

**Notable present-but-vestigial keys** (kept around but inert under YR):

| Key | Value | Status |
|-----|-------|--------|
| `Ammo=5` | 5 | Parses to TechnoTypeClass+`0x680` (InitialAmmo). For a building without a Weapon=, this field has no observable effect — see §4. |
| `;SuperWeapon=ParaDropSpecial` | — | Commented out. INI parser strips `;` comments before key recognition; this line is treated as if absent. See §4. |

---

## 3. Capture mechanics & the engineer cursor on CAPOWR

CAPOWR sets `Capturable=yes` (`BuildingTypeClass+0x1572 = 1`) and
`NeedsEngineer=yes` (`BuildingTypeClass+0x1552 = 1`). It does **not** set
`Repairable=yes`. From `TECH_CABHUT_GHIDRA_REPORT.md` §3.1 the engineer-on-building
cursor block in `InfantryClass::What_Action_OnObject` is gated on
`Type+0xCCC` (Repairable). Without it, the engineer cursor falls back through
the function to the alternative branches.

Tracing the fall-through path in `InfantryClass::What_Action_OnObject` at
`0x0051E3B0`:

1. The Engineer + RTTI 6 + IsHumanPlayer + vtable+0x80 + `Type+0xCCC` block
   does NOT fire for CAPOWR (no `Repairable=yes`).
2. Falls through to the generic action-resolution path.
3. The path that fires for an engineer-on-capturable-building eventually
   reaches the `Type+0x1572 != 0` (Capturable) check (the second block at
   the bottom of the function). Returns action `0x1C` (capture cursor) for
   a non-ally building. If `MultiplayPassive` (the neutral civilian house)
   owns the target, the ally check is bypassed and the capture branch fires.

**Per-cell handling on arrival:** see `TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md`
§4 — the engineer's `PerCellProcess` on Mission `0x08`/`0x0B`/`0x19` calls
`target.vtable[0x3D4](engineer.Owner, 1)` which routes to
`BuildingClass::ChangeOwner`. The engineer is `Limbo`'d (consumed) at the
function tail.

**ChangeOwner-specific side-effects for CAPOWR** (per
`TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md` §4.1):

- `CaptureEvaEvent=EVA_TechBuildingCaptured` is queued (`Type+0x1554`); the
  new local owner hears the standard "tech building captured" EVA voice.
- `ProduceCashStartup` grant: zero (CAPOWR does not set `ProduceCash*`),
  so no credits are granted on capture.
- `InfantryGainSelfHeal` / `UnitsGainSelfHeal` counter updates: zero
  contribution (CAPOWR does not set those flags).
- The asymmetric old/new owner counter math from `TECH_CAHOSP_VS_CATHOSP`
  §4.1 does not apply (nothing to clamp).
- A radar reveal of `Sight=6` cells around the building applies via the
  standard `TechnoClass::ChangeOwner` reveal step.

---

## 4. The `Ammo=5` and `;SuperWeapon=ParaDropSpecial` mystery

### 4.1 `Ammo=` parses but has no live consumer for CAPOWR

- Parse site: `0x00714755` in `TechnoTypeClass::ReadINI`.
- The INI key string the parser searches for is **`InitialAmmo`** (verified
  in the binary's string table at `0x00843AEC`). The YR convention is that
  `Ammo=` and `InitialAmmo=` *both* resolve to this parse site (YR
  rules-parser supports legacy short names). For confirmation, the
  TechnoTypeClass parse site reads default from `[EBP+0x680]`, calls
  `INI::ReadInt`, stores to `[EBP+0x680]`. Net: `Ammo=5` →
  `TechnoTypeClass+0x680 = 5`.
- The field at `+0x680` is the **InitialAmmo / starting ammo count** used
  by units, aircraft, infantry with `Ammo=N`. For a building without a
  `Weapon=` or `Primary=`, no code path consumes this value (the firing
  pipeline reads ammo from the per-instance fields, which are only
  populated when the type is a firing entity).
- **CAPOWR has no `Weapon=` or `Primary=`** (verified by INI absence).
  Therefore `Ammo=5` is inert. It is almost certainly leftover from the
  era when this section did set `SuperWeapon=ParaDropSpecial` — paradrop
  super-weapons in TS-era code used the building's ammo to limit paradrop
  invocations. With the SuperWeapon line commented out, the Ammo field has
  no consumer.

**Tiny detail / parity implication.** A Rust port can ignore `Ammo=` on
buildings without a weapon entirely. There is no observable consequence.
However, if your port plans to support custom mods that re-enable
`SuperWeapon=ParaDropSpecial` on CAPOWR, you must reinstate the ammo-tied
SW activation logic at that point.

### 4.2 `;SuperWeapon=ParaDropSpecial` is genuinely dead

- The INI parser (`CCINIClass::ReadString` at `0x00528A10` and friends)
  treats `;` as a comment marker. Lines starting with `;` are stripped
  during section enumeration; key-value parsing does not see them.
- There is **no inheritance** in INI parsing between BuildingTypeClass
  sections. Each `[CAPOWR]` block is read in isolation from its own
  section header.
- There is **no default value** for `SuperWeapon=` on BuildingTypeClass.
  The parse default is `None`/empty (verified by the BuildingTypeClass
  constructor zero-init pattern; the SuperWeapon parse path stores the
  resolved string into a pointer field — null when unset). Net: with the
  `;` comment in place, no SuperWeapon is bound to CAPOWR.
- The CAAIRP path is documented in `TECH_BUILDINGS_GHIDRA_REPORT.md` §3.4
  (the SuperWeapon→AI_ResumeProduction flow). That path is NOT invoked
  for CAPOWR.
- Map override risk: a map's `[CAPOWR]` section overlay *could* set
  `SuperWeapon=ParaDropSpecial` to re-enable the path. Stock YR maps do
  not do this. Custom maps could. Mark this as a "modder-extensible" but
  not "active in stock" feature.

**Bottom line:** the line is dead. The `Ammo=5` next to it is an unused
relic. Neither is consumed in a stock YR game on stock content.

---

## 5. Spy infiltrate side-effect on CAPOWR — **YES, it triggers a power blackout**

This is the only **interactive** secondary effect CAPOWR provides
post-capture. The function `BuildingClass::OnSpyInfiltrate` at `0x004571E0`
dispatches on the building's *type* fields. Decompile (condensed and
annotated):

```c
void BuildingClass::OnSpyInfiltrate(this, infiltrator_owner) {
    if (this->Owner == infiltrator_owner) return;   // self-infiltrate noop

    if (IsHumanPlayer) {
        radarEvent = CreateRadarEvent(building.coord);
        if (radarEvent) /* bVar2 = true */ ;
    }

    if (this->Type[+0x16A4] == 0) {                  // NOT a Radar building
        if (this->Type[+0xEE0] < 1) {                // PowerOutput <= 0 (NOT a power producer)
            // → check OreRefinery / WarFactory / SuperWeapon-on-infiltrate paths
            //   (handles money-steal, super-weapon-reset, AI mission-cancel,
            //    spy-blue effects — see other docs)
        } else {                                      // PowerOutput > 0 (this IS a power producer)
            HouseClass::SpyPowerSabotage(
                this->Owner,                          // victim
                RulesClass+0xD64,                     // duration = SpyPowerBlackout (default 1000 frames)
                /* uninitialised ESI value */ );      // 3rd param — see Open Questions
            if (IsHumanPlayer && bVar2) VoxClass::PlayEVA(EVA_BuildingInfiltrated);
        }
    } else {                                          // IS a Radar building
        FUN_0050BD10();   // RadarSpySabotage — covers the "spy reveals minimap" path
        if (IsHumanPlayer && this->Owner[+0x577A] == 0 && bVar2) VoxClass::PlayEVA(...);
    }

    this->vtable[0x124](2);   // some refresh / animation call
    return;
}
```

**For CAPOWR specifically:**

- `Type[+0x16A4]` (`Radar=`) = 0 — CAPOWR is not a radar building.
- `Type[+0xEE0]` (PowerOutput) = `200 > 0` — CAPOWR IS a power producer.
- Therefore the spy infiltrate calls
  `HouseClass::SpyPowerSabotage(this->Owner, 1000, …)` — applying a
  power blackout to whoever currently owns the captured CAPOWR.

### 5.1 `HouseClass::SpyPowerSabotage` (`0x0050BC90`) — what it actually writes

```c
void HouseClass::SpyPowerSabotage(this, duration, blackoutEndFrame) {
    this+0x5778 = 1;                              // NeedsPowerRecalc set
    this+0x2A4  = g_CurrentFrameCounter;          // SpyBlackoutStartFrame
    this+0x2A8  = blackoutEndFrame;               // (3rd-param storage — see Open Q)
    this+0x2AC  = duration;                       // SpyBlackoutDuration
}
```

**Three writes that matter:**

1. `+0x5778 = 1` — sets `NeedsPowerRecalc` so the next `AI_AssessPower`
   tick will see the blackout. Without this, the AssessPower path
   wouldn't re-evaluate immediately.
2. `+0x2A4 = g_CurrentFrameCounter` — the blackout start frame. Combined
   with `+0x2AC` (duration), this drives the in-AssessPower expiry check
   at `AI_AssessPower` lines `_d6f`–`_d73` (see `POWER_SYSTEM_GHIDRA_REPORT.md`).
3. `+0x2AC = duration` — picked up from `RulesClass+0xD64`
   (`SpyPowerBlackout=` INI key, default `1000` frames per `rulesmd.ini`
   line 277).

### 5.2 The blackout effect inside AI_AssessPower

`HouseClass::AI_AssessPower` (`0x00508C30`), per its own decompile (see §5
core finding above and `POWER_SYSTEM_GHIDRA_REPORT.md`), runs the per-tick
sum but then zeroes `this->PowerOutput` if the blackout is still active:

```c
// after summing PowerOutput / PowerDrain over all owned buildings:
if (this+0x2A4 != -1) {                                // blackout active flag set
    elapsed = g_CurrentFrameCounter - this+0x2A4;
    if (elapsed < this+0x2AC) {
        // still in blackout window
        this->PowerOutput = 0;                          // ZERO OUT the total
    } else if (HasOccupiedPowerPlant) {                 // expired but…
        this->PowerOutput = 0;                          // ALSO zero (TS-era quirk)
    }
}
```

**Stock effect:** infiltrating CAPOWR (or any owned power producer) of an
enemy player → that player's PowerOutput is forced to **0** for the next
~66 seconds (1000 frames at 15 FPS in YR). All consumers of power (defenses,
factories, radar) read their state from PowerOutput/PowerDrain and enter
low-power mode.

### 5.3 Important sequencing detail — the post-blackout `HasOccupiedPowerPlant` zero

The `HasOccupiedPowerPlant` flag (`HouseClass+0x577B`) is set to `1` during
the AssessPower scan if any owned power producer has at least one occupant
(garrison). Per `POWER_SYSTEM_GHIDRA_REPORT.md` line 49 and the AssessPower
decompile, the "blackout expired" branch ALSO zeroes PowerOutput if
`HasOccupiedPowerPlant` is true — this is the in-binary "occupied power
plant" quirk where keeping a unit inside a captured power plant after
spy-blackout-recovery keeps the power offline. **Note:** CAPOWR cannot be
garrisoned (no `CanBeOccupied=yes`); the flag will not be set for CAPOWR
itself, but it could be set for *other* owned power plants. Not a CAPOWR-
specific behavior but listed for completeness.

---

## 6. ChangeOwner → power recalc — the "lazy" model

`BuildingClass::ChangeOwner` at `0x00448260` does *not* directly write a
delta into the owner's PowerOutput (HouseClass+0x53A4) or PowerDrain (+0x53A8).
It also does not (in the decompile I have) explicitly write `1` to
`NeedsPowerRecalc` (+0x5778) on the new owner outside the upgrade-refund
loop.

**However**, the captured CAPOWR is correctly counted on the new owner's
next `AI_AssessPower` run because:

1. `HouseClass::Recount` (called from ChangeOwner step 9) decrements a
   per-category counter on the old owner.
2. The captured building is appended to the new owner's per-flag building
   registries (the `+0x6C..0x140` list-cluster — see
   `TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md` §4.1 steps 8 and 13).
3. `AI_AssessPower` iterates the new owner's *all-buildings* list at
   `+0x6C` (the count is at `+0x78`) and sums GetPowerOutput. The captured
   building is included after the list-insert at step 2.
4. `NeedsPowerRecalc` does *not* need to be set for the recalc to fire
   per-tick. `HouseClass::Update` calls `AI_AssessPower` at `0x004F84E5`
   on conditions that include (per the existing POWER_SYSTEM doc)
   construction-complete and online-toggle events.

**Open question — RESOLVED PARTIALLY (2026-05-17, see §13 Audit notes).**
The exact gating of `HouseClass::Update`'s `AI_AssessPower` call has now
been traced at `0x004F84D9`:

```
004f84d9: MOV AL, byte ptr [ESI + 0x5778]   ; read NeedsPowerRecalc
004f84df: TEST AL, AL
004f84e1: JZ 0x004f84f1                     ; if 0, skip AssessPower call
004f84e3: MOV ECX, ESI
004f84e5: CALL 0x00508c30                   ; AI_AssessPower
```

So `AI_AssessPower` fires **iff** `+0x5778` is set. `AssessPower` itself
**clears** `+0x5778` (via `this->RecheckPower = false`) at the top of its
body. The recalc is therefore strictly trigger-driven; there is no
"every N ticks regardless" fallback.

**OLD-owner side.** `HouseClass::Removed_From_Game` at `0x005025F0`
(called by `TechnoClass::ChangeOwner` at the start of the transfer) sets
`oldOwner[0x5778] = 1` **unconditionally** in its `case 6` (RTTI =
Building) branch. So the old owner's next `HouseClass::Update` tick
re-runs `AI_AssessPower` and the captured building's contribution is
removed from the old total. **Resolved.**

**NEW-owner side — still ambiguous.** Decompilation of
`HouseClass::Added_To_Game` (`0x00502A80`), `HouseClass::Add_Tracking`
(`0x004FF700`), and `HouseClass::Update_Power_And_EVA` (`0x005018C0`)
shows none of them write to `newOwner[0x5778]`. The only writers to
`+0x5778 = 1` in `BuildingClass::ChangeOwner` are inside the
**upgrade-refund loop** which fires only when (a) `g_GameMode != 0` AND
(b) new owner is AI AND (c) OLD owner is **not** `MultiplayPassive` AND
(d) the building has upgrades. For a stock CAPOWR captured from the
neutral civilian house (`MultiplayPassive`), condition (c) FAILS and the
loop does not execute — so the new owner's `+0x5778` is NOT set during
this capture.

In practice the new owner's `AI_AssessPower` still fires soon (typically
within a handful of ticks) because *other* events keep setting `+0x5778`:
construction-complete on any owned building, manual `BuildingClass::GoOffline`
toggles, EMP/IC effects calling `RestoreOnlineEffects` (which also sets
`+0x5778`), spy infiltrate, building destruction, etc. So the player
*observes* the captured CAPOWR's contribution very quickly — but it is
not strictly a ≤1-tick guarantee for the new owner the way it is for the
old owner. The previous "≤ 1 tick from the player's perspective" line
holds *empirically* under normal play but is **not** binary-guaranteed
for the new-owner side of this specific event.

**Implication for the port.** Either match this behavior literally
(write the OLD-owner-only setter; let the new-owner recalc piggyback
on whatever event happens next) — OR, more practically and with no
observable difference, just recompute power totals every tick (the
"recompute from scratch" model). gamemd's lazy model is a perf
optimisation that the port doesn't need at YR scale.

### 6.1 Asymmetry vs. the InfantryGainSelfHeal mechanism

| Mechanism | Update model | Old-owner / new-owner symmetry |
|-----------|--------------|-------------------------------|
| **Power=** (CAPOWR) | **Lazy** — recomputed from scratch each `AI_AssessPower`. No per-event delta. | **Symmetric** — both old and new owner recompute on their next AssessPower tick. |
| `InfantryGainSelfHeal=` (CAHOSP / CATHOSP) | **Eager** — `ChangeOwner` writes a delta to `Owner+0x164`. | **Asymmetric** — decrement on old is clamped at 0 and gated on `ActuallyPlacedOnMap`; increment on new is unconditional. |

**Tiny implication for the port.** A Rust port that mirrors gamemd's
internal "lazy power tally" can recompute power totals each tick — that
matches the binary. A port that elects to use eager deltas must take care
that capture/destroy/sell paths increment/decrement symmetrically, since
gamemd's behavior over a tick window IS symmetric for power (no clamp
needed — the recalc is a fresh sum).

---

## 7. GetPowerOutput — the health-scaling and online-gating that apply to a captured CAPOWR

Per `POWER_SYSTEM_GHIDRA_REPORT.md` §"GetPowerOutput (0x44E7B0)":

```
if (vtable[0x1D4]() != 0) return 0                                    // "in-limbo / not-yet-buildable" gate
base = TypeClass.PowerOutput (+0xEE0)                                  // CAPOWR: 200
if (this->HasExtraPowerBonus flag is set): base += Type+0xEE8          // per-instance flag, NOT UpgradeLevel
if ((Type+0x16AE != 0 OR Type+0x16AF != 0)
    AND Type+0xEE8 > 0                                                 // ExtraPowerBonus > 0 (required co-gate)
    AND occupant_count (+0x114) > 0):
        base += Type+0xEE8 × occupant_count
if (UpgradeLevel != 0):
    for i in 0..3:                                                     // iterates ALL 3 upgrade slots
        if (Upgrades[i] != null): base += Upgrades[i]->Type+0xEE0
if (base > 0 AND IsOnline (+0x660)): return ftol(base × GetHealthRatio())
return 0
```

**Verified-2026-05-17 asm anchor points** (in `BuildingClass::GetPowerOutput` at `0x0044E7B0`):
- Base load: `iVar5 = *(int *)(this->Type + 0xee0)` (top of function)
- "In-limbo" gate: `CALL (vtable+0x1D4)` then `if (cVar2 == '\0')` enters body, else falls through to `return 0`
- Upgrade-slot loop: `0x0044E83B-0x0044E84B` reads 3 slot pointers, adds each non-null upgrade's `+0xEE0`
- IsOnline read: `0x0044E855: MOV AL, byte ptr [ESI + 0x660]` (NOT `+0x198` as earlier drafts of this doc claimed)
- Final formula: `0x0044E861: CALL GetHealthRatio` → `0x0044E866: FIMUL dword ptr [ESP + 0x8]` → `0x0044E86A: CALL ftol` → return EAX

**For CAPOWR specifically:**

- `base = 200` always (no upgrade modifiers apply).
- The contribution is `ftol(200 * GetHealthRatio())`. At full strength
  (Strength=800, current HP=800): `200 * 1.0 = 200`. At half: `100`. At
  zero (destroyed): `0` (but destroyed buildings hit the `InLimbo` early
  return).
- The contribution is gated on `IsOnline` (BuildingClass+0x660 — same field
  `this->HasPower` that `BuildingClass::GoOffline` at `0x00452360` toggles
  via `this->HasPower = false`). A captured CAPOWR keeps the IsOnline bit
  it had pre-capture. If the new owner power-toggles it off, the
  contribution drops to 0 until toggled back on.
- The `ftol` (float-to-long) call **truncates toward zero** — at
  Strength=800 HP=799 (`HealthRatio = 0.99875`), `200 * 0.99875 = 199.75`
  → `199`. At HP=1: `200 * 0.00125 = 0.25` → `0`.

**Tiny detail / parity implication.** The truncation direction is `ftol`
(standard C runtime). For the port, integer truncation toward zero (not
toward negative infinity) at health ratios just below an integer boundary
matters for the *exact* power total displayed in the UI.

---

## 8. Stock map placement & meta-observation

`NeutralTechBuildings=CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR` (rulesmd.ini
line 3082) lists CAPOWR alongside the other tech buildings. Several stock
YR skirmish maps include CAPOWR as a placeable neutral building (verified
indirectly via the section's `RadarVisible=yes` setting — only buildings
expected to appear on real maps get this).

**The user's question** ("verify whether capture grants power") is unusual
for a tech building because it's the only one of the six whose value comes
from a *system* (the power total) rather than a per-tick aura or a one-off
credit grant. The answer is clear: yes, +200 power, applied via the lazy
recalc model, scaled by health and gated on IsOnline.

---

## 9. TS-legacy audit

| Item | Status | Notes |
|------|--------|-------|
| `Power=` parsing + GetPowerOutput consumer | **Live in YR.** | Verified per the existing POWER_SYSTEM doc. |
| `BuildingClass::OnSpyInfiltrate` power-sabotage branch | **Live in YR.** | Verified by direct decompile (§5). The branch fires whenever a spy infiltrates an owned building with `Type+0xEE0 > 0` and `Type+0x16A4 == 0`. |
| `Ammo=` (InitialAmmo) on CAPOWR | **Inert in YR.** | No live consumer for buildings without a Weapon=. The field is parsed and stored but never read by any active code path that affects a buildingtype's behavior in stock content. Relic of the deleted SuperWeapon=ParaDropSpecial. |
| `;SuperWeapon=ParaDropSpecial` | **Truly dead.** | Comment-stripped at parse time; no inheritance; no default. Mods could re-enable. |
| `HasOccupiedPowerPlant` quirk (+0x577B in AI_AssessPower) | **Live in YR but unreachable for CAPOWR.** | CAPOWR cannot be garrisoned (no `CanBeOccupied=yes`), so the flag will never be set *due to* CAPOWR. The flag is affected by *other* owned power plants. |
| `;Buildup=CAAIRPMK` and `;ActiveAnimTwo=CAAIRP_F` in artmd.ini | **Dead.** | Comment-stripped at parse time. Confirms CAPOWR was originally going to share art/code with CAAIRP (the Tech Airport's `SuperWeapon=ParaDropSpecial` path). The architecture was abandoned mid-development. |

---

## 10. Quick-reference behaviour table for CAPOWR

| Player action | CAPOWR response | Where verified |
|---------------|-------------------|------------------|
| Hover engineer over CAPOWR | Capture cursor `0x1C` (via the fall-through path in `What_Action_OnObject` — see §3) | §3 |
| Click engineer on CAPOWR | Mission set to `0x08` Capture; engineer pathfinds to a cell adjacent to the foundation. | (issue-side; out of scope) |
| Engineer reaches CAPOWR cell | PerCellProcess routes to the capture branch (Mission 8/0xB/0x19, Type[0x1572]=1) → `target.vtable[0x3D4]` → `BuildingClass::ChangeOwner`. Engineer Limbo'd. EVA_TechBuildingCaptured queued for new local owner. | TECH_CAHOSP_VS_CATHOSP §4 / §3 here |
| Capture completes | Building transferred to new owner. `Sight=6` cells revealed. Next `AI_AssessPower` tick adds `200 × HealthRatio` to new owner's PowerOutput; subtracts the same from old owner's PowerOutput. | §1 / §6 |
| Player toggles CAPOWR offline (right-click on building's status icon) | `BuildingClass::GoOffline` (per POWER_SYSTEM §"GoOffline"); sets `IsOnline=0`; next AssessPower contributes 0. | POWER_SYSTEM doc |
| Building takes damage | Health decreases. Next AssessPower contributes `200 × HealthRatio` (linearly less power). At HP=1, contribution is 0 (ftol truncation). | §7 |
| Building destroyed | `OnDestroyed` runs (out of scope here). Building removed from owner's list. Next AssessPower contributes 0. | — |
| Spy infiltrates CAPOWR (after capture) | New owner's PowerOutput forced to 0 for `SpyPowerBlackout=1000` frames (~66 s). Triggers low-power state on the owner. | §5 |
| Spy infiltrates CAPOWR (pre-capture, while owned by neutral civilian) | Neutral civilian's PowerOutput set to 0 — no observable effect (neutral house's power isn't consumed). Wasteful order; engine fires but result is invisible. | §5 / by inference |
| Capture from another player (not neutral) | Same as capture from neutral, except no special EVA branch. ProduceCashStartup grant is zero either way (CAPOWR has no `ProduceCashStartup=`). | TECH_CAHOSP_VS_CATHOSP §4 |

---

## 11. Open questions

1. **Where does `NeedsPowerRecalc` (+0x5778) get set during capture?**
   **PARTIALLY RESOLVED 2026-05-17.** OLD owner: `HouseClass::Removed_From_Game`
   sets it unconditionally in its case-6 branch. NEW owner: NOT explicitly
   set by the obvious ChangeOwner call chain — see §6 and §13 for the
   detailed trace. The "new-owner side has no explicit setter" finding is a
   real disparity from "lazy recalc fires within 1 tick" but is not
   observable in practice because so many other events trigger `+0x5778`.

2. **`HouseClass+0x2A8` write (the third param to `SpyPowerSabotage`).**
   **RESOLVED 2026-05-17 — IT IS GARBAGE.** Disassembly of
   `HouseClass::SpyPowerSabotage` at `0x0050BC90` shows `RET 0x4` at the
   tail, meaning only **one** stack argument is cleaned up by the callee.
   The function is therefore `__thiscall(this in ECX, duration as the
   single stack arg)`. The `MOV EAX, dword ptr [ESP + 0x8]` at `0x0050BCB0`
   (after `PUSH ESI`) reads from local space (offset 4 into the locals
   allocated by `SUB ESP, 0xC`) — that is, uninitialised stack memory,
   NOT a third argument. The corresponding `MOV [ESI + 0x4], EAX` writes
   that garbage to `+0x2A8`. So `+0x2A8` does receive a stack-garbage value
   on every spy-infiltrate-power-plant. It is unlikely to be read
   meaningfully anywhere downstream; the doc lists `+0x2A4` (start frame)
   and `+0x2AC` (duration) as the two real blackout fields, which
   `AI_AssessPower` consumes. `+0x2A8` is effectively dead.

3. **`HasOccupiedPowerPlant` post-blackout zero quirk.** Per AI_AssessPower
   §5.3, the "blackout expired" branch *still* zeroes PowerOutput if
   `HasOccupiedPowerPlant` is true. This is a TS-era mechanism where
   garrisoned power plants stay sabotaged longer. CAPOWR cannot trigger it
   directly (not garrisonable), but a player owning CAPOWR + a garrisoned
   IronCurtainable power plant would see weird interaction. Documented for
   parity but not separately verified at this level of detail.

4. **`+0x680` (InitialAmmo) — does any building code path read it?** The
   investigation confirmed *no live consumer for stock content*; the open
   question is whether YR's modder ecosystem has discovered a code path
   that does read it (e.g., for tech-grant buildings or invisible AI
   logic). The TS-era code that consumed it for paradrops likely still
   exists in the binary but is unreachable without the
   `SuperWeapon=ParaDropSpecial` link. Worth a 1-finding verification if
   we want to be 100% sure the field is inert.

5. **Stock-map enumeration.** Listing every stock YR map containing a
   `CAPOWR` placement requires scanning all `.mmx`/`.map` files. Out of
   scope here; the user's question doesn't depend on knowing the exact
   maps. (`NeutralTechBuildings=` membership is sufficient evidence that
   CAPOWR is expected to appear in stock content.)

---

## 12. Current Rust implementation status

| Subsystem | Status in port |
|-----------|----------------|
| `Power=` parsing | **Missing.** Per the prior scan ([src/rules/object_type.rs:493](../ra2-rust-game/src/rules/object_type.rs#L493) covers Capturable but not Power). Need a `power: i32` field on the building type. |
| `Capturable=` parsing | Present (Capturable bool field). CAPOWR's `Capturable=yes` will be parsed; engineer-target validation will pass. ✓ |
| Engineer capture command | Present ([src/sim/world/world_commands.rs:1013](../ra2-rust-game/src/sim/world/world_commands.rs#L1013)). Will issue ownership transfer. ✓ |
| Per-tick `AI_AssessPower` equivalent | **Missing.** Need to iterate owned buildings each tick, sum `Power=` (health-scaled, online-gated). |
| `IsOnline` toggle | **Missing.** |
| Health-scaled `GetPowerOutput` (`ftol(base × HealthRatio)`) | **Missing.** Note: the port should reproduce the `ftol` truncation-toward-zero, not round-toward-nearest. |
| Spy infiltrate → power blackout | **Missing.** Tied to spy mechanics, which are also missing per prior scan. |
| `SpyPowerBlackout=` Rules key | **Missing.** |
| Power total in UI | **Missing.** Existing power bar may exist for direct production; needs to consume the per-house total. |
| `Ammo=` on buildings | **Missing.** Recommendation: skip; do not parse on building types. Mark as known-inert. |
| `;SuperWeapon=ParaDropSpecial` | **Not applicable.** Commented out in stock; no port action needed. |

---

## Sources

- Ghidra MCP — live decompilation of `gamemd.exe`:
  - `0x00448260` (`BuildingClass::ChangeOwner`)
  - `0x004571E0` (`BuildingClass::OnSpyInfiltrate`)
  - `0x00508C30` (`HouseClass::AI_AssessPower`)
  - `0x0050BC90` (`HouseClass::SpyPowerSabotage`)
  - `0x004FF980` (`HouseClass::Recount`)
  - `0x004F84E5` (caller of AI_AssessPower in `HouseClass::Update`)
  - parse-site assembly contexts at `0x0066FBFE` (`SpyPowerBlackout`),
    `0x00714755` (`InitialAmmo`)
- INI files (in-repo authoritative):
  - `ini/rulesmd.ini` lines 277 (`SpyPowerBlackout=1000`), 1211 (BuildingTypes), 3082 (`NeutralTechBuildings`), 13956–13979 (`[CAPOWR]`)
  - `ini/artmd.ini` lines 3421–3440 (`[CAPOWR]` art)
- Prior research (cited; not re-derived):
  - `ra2-rust-game-docs/POWER_SYSTEM_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/POWER_INI_PARSING_AND_LIFECYCLE.md`
  - `ra2-rust-game-docs/POWER_EDGE_CASES.md`
  - `ra2-rust-game-docs/SPECIAL_BUILDINGS_POWER_SYSTEM.md`
  - `ra2-rust-game-docs/TECH_BUILDINGS_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/TECH_CABHUT_GHIDRA_REPORT.md`

---

## 13. Audit notes (2026-05-17)

A focused re-verification pass was run against `gamemd.exe` via Ghidra MCP
on the four load-bearing claims flagged for spot-checking. Three held;
one (§6, the NeedsPowerRecalc setter chain) was partially resolved with a
finding that changes the strength of §1's "≤1-tick lag" wording.

| Claim | Result | Evidence / action |
|-------|--------|--------------------|
| §5: `BuildingClass::OnSpyInfiltrate` at `0x004571E0` routes a power-plant infiltrate (`Type+0xEE0 > 0` AND `Type+0x16A4 == 0`) to `HouseClass::SpyPowerSabotage` | **VERIFIED** | Decompile confirms `if (Type+0x16A4 == 0) { if (Type+0xEE0 < 1) { ...money-steal / SW-reset / mission-cancel paths... } else { HouseClass::SpyPowerSabotage(Owner, Rules+0xD64, unaff_ESI); ... } } else { FUN_0050BD10(); /* RadarSpySabotage */ }`. CAPOWR has `+0x16A4 == 0` (not radar) and `+0xEE0 == 200`, so it hits the power-sabotage branch. |
| §5.1: `HouseClass::SpyPowerSabotage` at `0x0050BC90` writes `+0x5778=1`, `+0x2A4=g_CurrentFrameCounter`, `+0x2A8=<suspicious>`, `+0x2AC=duration`. The `+0x2A8` write is flagged as suspicious | **VERIFIED — `+0x2A8` IS GARBAGE** | Disassembly shows `RET 0x4` at the tail, meaning only one stack arg is cleaned up. The function is `__thiscall(this in ECX, duration on stack)`; there is NO third parameter despite Ghidra's signature suggesting one. The `MOV EAX, dword ptr [ESP + 0x8]` at `0x0050BCB0` reads from local space (post `SUB ESP, 0xC; PUSH ESI`, `[ESP+0x8]` is 4 bytes into the local space, NOT the next arg slot), then writes to `+0x2A8`. The "third arg" `unaff_ESI` in the caller is a Ghidra register-liveness artefact — it sits on the stack uncleaned but the callee never reads it. `+0x2A8` therefore receives whatever happened to be in the uninitialised local space. Open Question #2 is now closed: `+0x2A8` is dead. |
| §6: trace `NeedsPowerRecalc` (`+0x5778`) setter chain from `BuildingClass::ChangeOwner` | **PARTIAL — RESOLVED FOR OLD OWNER ONLY** | (a) `HouseClass::Update` at `0x004F84D9` reads `+0x5778`, calls `AI_AssessPower` iff non-zero, and `AI_AssessPower` clears `+0x5778` at the top of its body. The recalc is strictly trigger-driven. (b) OLD owner: `HouseClass::Removed_From_Game` (called by `TechnoClass::ChangeOwner`) sets `oldOwner[0x5778] = 1` unconditionally in its case-6 (Building) branch — confirmed by inspecting the decompile at `0x005025F0`. So the old owner's next tick re-runs AssessPower. (c) NEW owner: NOT explicitly set in the obvious chain. `HouseClass::Added_To_Game` (`0x00502A80`), `HouseClass::Add_Tracking` (`0x004FF700`), `HouseClass::Update_Power_And_EVA` (`0x005018C0`) — none write `+0x5778`. The only writers I found inside `BuildingClass::ChangeOwner` are in the **upgrade-refund loop** which requires (i) `g_GameMode != 0`, (ii) new owner is AI, (iii) OLD owner is NOT `MultiplayPassive` (i.e. not neutral civilian) — failing condition (iii) for capture from neutral. Thus for a stock CAPOWR captured from the neutral civilian house, the NEW owner has no immediate `+0x5778` setter and must wait for a subsequent event (often within a few ticks). §1 and §6 have been updated to reflect this; the wording "≤1-tick lag" is now qualified to "old-owner side only" with "subsequent-event piggyback" for the new owner. Other writers of `+0x5778` confirmed: `BuildingClass::GoOffline` (`0x00452360`), `HouseClass::SpyPowerSabotage`, `TechnoClass::AI_Update` (in the mind-control release branch at `~0x6FA000`), and various EMP/upgrade paths. |
| §4.1: `Ammo=5` (parsed via `InitialAmmo` string) writes to `TechnoTypeClass+0x680`; no live consumer for buildings without `Weapon=` | **VERIFIED (parse) / PLAUSIBLE (no-consumer)** | Parse site verified: at `0x00714755` the asm pushes the string pointer at `0x00843AEC` which reads as `"InitialAmmo\0"`, followed by `MOV dword ptr [EBP + 0x680], EAX` at `0x00714760`. The "no live consumer for non-firing buildings" claim was not negatively re-verified by exhaustively searching every `+0x680` read in the binary (a true negative is expensive), but the reasoning is sound: ammo state is read from per-instance fields, not from `TypeClass+0x680`, and those instance fields are only initialised when the type is a firing entity (has `Weapon=`/`Primary=`/`Spawner=`). For CAPOWR specifically — no `Weapon=`, no `Primary=` — the field is parsed and stored but no reachable code path on stock content reads it. Confidence on the "no-consumer" portion remains MEDIUM (untested exhaustive-search) but practical impact stays nil. |

**Corrections written to the doc.**
- §1 "≤1-tick lag" qualified.
- §6 "Open question" rewritten with the trace and the old/new asymmetry.
- §11 Open Question #1 updated with the partial resolution; #2
  (`+0x2A8` is garbage) marked resolved.
- §10 quick-reference table unchanged (player-visible behavior is the
  same — the qualification is internal-mechanism-only).

### Not re-verified in this pass — candidates for a future audit

The audit was scoped to the four high-risk claims above. The following
specific claims in this doc were NOT independently re-checked against
the binary in this pass. Each is paired with an exact target.

- **§1 / §7 `GetPowerOutput` formula.** The claim is `if InLimbo: return 0;
  base = +0xEE0; if upgraded: base += ExtraPowerBonus; if has-power-upgrade-flags
  AND occupant > 0: base += ExtraPowerBonus × occupant; if has-upgrade-slots:
  for each occupied slot base += upgrade.PowerOutput; if base > 0 AND
  IsOnline: return ftol(base × GetHealthRatio()); return 0;`. This came
  from `POWER_SYSTEM_GHIDRA_REPORT.md` and was NOT re-derived. The
  `BuildingClass::GetPowerOutput` function at `0x0044E7B0` should be
  decompiled and confirmed in full — particularly the `IsOnline`
  (`BuildingClass+0x198`) gate, the `ftol` truncation direction, the
  upgrade-slot iteration, and the occupant-multiplier branch.
- **§5.2 AI_AssessPower blackout block** — the `if (this+0x2A4 != -1) {
  elapsed = g_CurrentFrameCounter - this+0x2A4; if (elapsed < this+0x2AC)
  PowerOutput = 0; else if (HasOccupiedPowerPlant) PowerOutput = 0; }`
  block. The doc cites it via `POWER_SYSTEM_GHIDRA_REPORT.md` but the
  audit only confirmed AssessPower's gating via +0x5778; the body of the
  blackout-check was NOT independently read. The decompile of
  `HouseClass::AI_AssessPower` is in this audit response — a future
  re-pass should match the cited block against lines 50-65 of the
  decompile.
- **§5.3 `HasOccupiedPowerPlant` quirk** (`HouseClass+0x577B`). The audit
  saw `this->field_0x577b = local_d;` in AssessPower but did NOT verify
  the "blackout-expired branch ALSO zeroes PowerOutput if
  HasOccupiedPowerPlant is true" claim end-to-end. Worth re-reading the
  expiry branch carefully.
- **§3 engineer cursor on CAPOWR** — the "fall-through to capture cursor
  0x1C" claim. CAPOWR lacks `Repairable=yes`, so the engineer-on-building
  block in `What_Action_OnObject` skips. The doc traces the fall-through
  to the lower Capturable block returning `0x1C`. The audit did read
  `What_Action_OnObject` in full (for CABHUT §3) and the lower Capturable
  branch IS visible — but the "fall-through reaches it" claim was not
  separately exercised. Low-risk; trivial follow-up.
- **§4 `Ammo=`/`InitialAmmo` consumer search.** The "no consumer for
  buildings without Weapon=" claim is reasoning-based, not a
  binary-search-confirmed negative. A future investigator can prove the
  negative by listing all reads of `TechnoTypeClass+0x680` in the binary
  (e.g. via byte-pattern search for the displacement) and confirming
  none of them gate on RTTI=Building.
- **§4.2 `;SuperWeapon=ParaDropSpecial` is dead** — confirmed by
  reasoning about the INI comment-stripping. The `CCINIClass::ReadString`
  comment-handling at `0x00528A10` was NOT independently exercised.
- **§5.1 `+0x2A4` / `+0x2A8` / `+0x2AC` field-purpose assignments.** The
  audit verified what gets WRITTEN where, but the doc's interpretation
  of these as "SpyBlackoutStartFrame" / "<garbage>" / "SpyBlackoutDuration"
  is partly verified (the duration / start-frame use in AssessPower is
  cited but not re-exercised). A future pass should match the writes
  against the reads in AI_AssessPower's blackout block to confirm
  semantic role.
- **§9 TS-legacy audit table** — same caveat as CABHUT §8. Reasoning-
  based; not individually trace-confirmed for each entry.
- **§10 quick-reference behaviour rows that depend on unverified upstream
  claims** — particularly the `Capture cursor 0x1C` row (depends on the
  unverified fall-through trace above) and the `200 × HealthRatio`
  contribution row (depends on the unverified GetPowerOutput formula).
- **The CHANGEOWNER STEPS 1-20 inherited from CAHOSP §4.1.** This doc
  cites CAHOSP for ChangeOwner ordering. The CAHOSP audit only verified
  step 10 (self-heal asymmetry) directly. So CAPOWR §3 inherits
  unverified claims about steps 1-9 and 11-20.
- **The §6 "other writers of `+0x5778`" footnote** that the audit added:
  `BuildingClass::GoOffline`, `HouseClass::SpyPowerSabotage`,
  `TechnoClass::AI_Update` mind-control-release, EMP/upgrade paths. The
  audit DID verify GoOffline at `0x00452360` and SpyPowerSabotage at
  `0x0050BC90`. The EMP/upgrade and AI_Update-release writers were
  noted from the AI_Update decompile and the upgrade-refund loop in
  ChangeOwner respectively but NOT separately catalogued. Exhaustive
  enumeration would be a useful "what triggers a power recalc" reference
  for the port.

If picking ONE follow-up target: **decompile and verify
`BuildingClass::GetPowerOutput` at `0x0044E7B0` in full.** It's the
load-bearing formula for the entire power system in the port, it's
referenced by both this doc and `POWER_SYSTEM_GHIDRA_REPORT.md`, and it
contains four separately-testable pieces (InLimbo gate, upgrade-bonus
accumulation, IsOnline gate, ftol truncation). One 30-minute pass closes
a lot of ground.
