# TechnoClass Damage-Receiver Gates Re-investigation

Date: 2026-07-13  
Target binary: active Yuri's Revenge gamemd.exe  
Primary function: TechnoClass::ReceiveDamage at 0x00701900  
Investigation mode: exhaustive slice, read-only Ghidra  
Status: PARTIAL — every load-bearing receiver-prefix, gate-order, readiness-formula, and negative-kernel row in the bounded slice is verified. Three semantic-name or downstream-consumer questions remain explicitly UNKNOWN; none changes a gate verdict or formula.

## Finding summary matrix

| ID | Finding | Verdict | Active-YR status | Implementation impact |
|---|---|---|---|---|
| F01 | The defender arithmetic block runs when ignoreDefenses is false and the original signed damage is nonnegative. It therefore turns an incoming zero into one. | VERIFIED | Active core path | Rust's dmg > 0 condition is wrong. |
| F02 | Per-hit defender armor is one ftol of damage / (house armor multiplier × Techno+0x158 double), followed by a separate veteran-armor divide/ftol. | VERIFIED | Active core path | Preserve grouping and the two conversion points; do not add zero-divisor branches. |
| F03 | TypeImmune is inside that nonnegative, non-ignore block and returns zero without overwriting the caller's damage integer. | VERIFIED | Active conditional path | A nullification enum alone cannot preserve the native pointer result. |
| F04 | Vtable +0x160 tests the active invulnerability timer; vtable +0x1D4 returns Techno+0x270, WarpingOut. Existing Rust names/order are reversed. | VERIFIED | Active core path | Rename/reorder the two status inputs. |
| F05 | Invulnerability exempts negative damage, but WarpingOut does not. ignoreDefenses suppresses both. | VERIFIED | Active core path | The gates cannot be evaluated as unconditional booleans. |
| F06 | DamageReducesReadiness runs after WarpingOut and before bunker/warhead gates, independent of sign and ignoreDefenses. | VERIFIED | Live parser; stock YR default disabled | It can reduce ammo on a hit later nullified by bunker or immunity, and negative damage can increase ammo. |
| F07 | FUN_006fb080 schedules/restarts the reload timer. It is not an ammo-depletion animation helper. | VERIFIED | Active when readiness is enabled | Existing research wording is stale. |
| F08 | The bunker branch is target-kind-sensitive. On a linked Building, PenetratesBunker=yes nullifies the hit; on a linked non-Building, PenetratesBunker=no can nullify after a cell building lookup. | VERIFIED | Active core path | A single bunkered && !penetrates boolean is not equivalent. |
| F09 | AffectsAllies uses attacker owner versus target owner; the Psychedelic alliance gate instead uses target owner versus sourceHouse. | VERIFIED | Active stock warheads | Do not collapse attacker and sourceHouse or their alliance predicates. |
| F10 | Psychedelic early-outs may return zero without changing *pDamage; the accepted path runs the damage kernel, stores its result in *pDamage and Techno+0x29C, sets Techno+0x298, invokes callbacks, and returns state 1 before ObjectClass HP mutation. | VERIFIED | Active stock warheads | It is not an always-zero-HP “MindControlled” gate. |
| F11 | FUN_00489180's negative-damage mask compares signed distance against 8. It does not compare armor index. | VERIFIED | Active kernel | Rust kernel.rs and its tests currently encode the wrong operand. |
| F12 | After ObjectClass::ReceiveDamage, TechnoClass performs threat feedback, result timers, WasAttacked, a virtual refresh, health-particle maintenance, and only then uses the original sign to decide whether retaliation/scatter processing is skipped. | VERIFIED at receiver boundary | Active core path | HP subtraction alone is not a receiver implementation. |

## Evidence / claim matrix

| Claim IDs | Classification | Exact evidence used | Confidence / limitation |
|---|---|---|---|
| F01–F05 | VERIFIED binary | Ghidra disassemble_function(address="0x00701900", program="/gamemd.exe"), especially 0x0070191F–0x00701AD8; read_memory(address="0x007F4AB8", length=0x90); disassemble_function(0x0041BF40); disassemble_function(0x0070C5B0) | Assembly-level branch and slot evidence. |
| F06–F07 | VERIFIED binary | disassemble_function(0x00701900), 0x00701ADB–0x00701B67; disassemble_function and decompile_function(0x006FB080); disassemble_function(0x00710AF0), constructor writes at 0x007110E6–0x00711169; disassemble_function(0x007148BF), parser at 0x007148B8–0x007148EB; read_memory(0x00843A08, 0x50) | Formula, defaults, parser, and reload scheduling verified. Meaning of timer dword +0x200 remains UNKNOWN. |
| F08–F10 | VERIFIED binary | disassemble_function(0x00701900), 0x00701B67–0x00701DC9; disassemble_function(0x004F9A50) | Branch direction, owner/source operands, writes, and early returns verified. |
| F11 | VERIFIED binary | Cold disassemble_function(0x00489180), 0x004891AF–0x004891C3; caller assembly at 0x00701D4D–0x00701D69 | Stack layout proves [ESP+0x1C] is the second stack argument, distance. |
| F12 | VERIFIED binary boundary | disassemble_function(0x00701900), 0x00701DCC–0x0070202E and 0x007027D2–0x00702A4A | Exact receiver-side ordering verified; internals of the state-4 branch and retaliation helper are outside this slice. |
| Runtime writers | VERIFIED binary, bounded | disassemble_function(0x006F2B40), 0x00482D56/0x00482E79, 0x0074FF50, 0x00750080, 0x00750090, 0x007500B0 and their xrefs; raw Techno save/load paths | Bounded to arithmetic runtime values Techno+0x158 and +0x150. Status-system writer closures were not expanded. |
| Stock activation | VERIFIED data | ini/rulesmd.ini:3501, 3540, 7226–7227, 26995, 27006, 27026, 27175, 27183, 27330, 27343, 27352, 30346–30347 | Readiness is supported but stock-disabled; relevant warhead and crate data are stock-active. |
| Rust disparity | VERIFIED source | src/sim/combat/damage/mod.rs:20–21, 90–120, 132–140; gates.rs:14–56; receive.rs:49–66; kernel.rs:50–58, 120–128; src/sim/combat/mod.rs:1849–1883; src/sim/game_entity.rs:224, 371 | Source scan only; no Rust was changed. |

## Target question and boundaries

The target question was:

> What exact active-YR mechanism does TechnoClass::ReceiveDamage apply from entry through its Techno-owned gates and immediate post-Object side effects, and what current Rust assumptions would violate that mechanism?

Included:

- TechnoClass::ReceiveDamage identity, arguments, signed-damage snapshot, defender arithmetic, TypeImmune, invulnerability, WarpingOut, readiness, bunker routing, warhead immunities, AffectsAllies, Psychedelic, ObjectClass delegation, and the receiver-owned common postlude.
- FUN_00489180 only where needed to settle the negative-damage operand.
- Parser/default evidence needed for readiness.
- Writers of the two mutable arithmetic inputs: Techno+0x158 armor multiplier and Techno+0x150 veterancy.
- A direct Rust disparity scan.

Excluded:

- ObjectClass::ReceiveDamage's internal HP/death mechanism.
- The internals of the state-4 delayed-death branch.
- Full retaliation/scatter behavior behind TechnoClass::ShouldRetaliate.
- The complete lifecycle implementation of Iron Curtain, Force Shield, WarpingOut, bunker installation, and crate collection.
- Attacker-side Fire_At arithmetic except where a caller established an argument.
- Rust implementation or tests.

## Preflight and source workflow

- The research index was queried first with 0x00701900, 0x00489180, Techno ReceiveDamage, related graph edges, and implementation handoffs.
- The active Ghidra program was /gamemd.exe at image base 0x00400000. Ghidra was used read-only.
- The output report did not exist at preflight.
- Relevant existing reports were read before decompilation, including RECEIVE_DAMAGE_GHIDRA_REPORT.md, RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md, DAMAGE_MATH_GHIDRA_REPORT.md, both gate-resolution reports, the veterancy report, and the TechnoType base report.
- This report is the only file owned by this investigation. The index is intentionally not rebuilt here because reindexing would violate the one-file output boundary.

## Function identity and call contract

RTTI and vtable evidence identify the receiver as active TechnoClass behavior:

- TechnoClass vtable base: 0x007F4960.
- Slot +0x16C at 0x007F4ACC contains 0x00701900.
- Complete-object locator 0x0080C058 references TypeDescriptor 0x00817B58, .?AVTechnoClass@@.
- Direct active callers include FootClass at 0x004D742C and BuildingClass at 0x00442425.
- The function ends in RET 0x1C, consuming seven 32-bit stack arguments.

The verified effective call contract is:

    thiscall TechnoClass::ReceiveDamage(
        int32_t *pDamage,
        int32_t distanceLeptons,
        WarheadTypeClass *warhead,
        ObjectClass *attacker,
        bool ignoreDefenses,
        bool arg6_unknown,
        HouseClass *sourceHouse)

At 0x00701DCC–0x00701DF8 the same seven values are passed to ObjectClass::ReceiveDamage in that order. The semantic name of arg6_unknown is not established here; its pass-through identity is verified.

## Exact receiver prefix and gate order

### 1. Snapshot the original sign

At 0x00701910–0x00701927 the function loads signed *pDamage and records originalNegative = (*pDamage < 0). This snapshot remains authoritative later even after *pDamage is transformed.

### 2. Defender arithmetic, only when !ignoreDefenses && !originalNegative

This condition includes zero.

#### 2a. House and per-unit armor, one conversion

    *pDamage = ftol(
        (double)*pDamage
        / (targetOwner.GetArmorMultForType(targetType)
           * target.ArmorMultiplier_double)
    )

Evidence:

- target.ArmorMultiplier is the 64-bit double at Techno+0x158.
- The multiplication of the two divisors occurs before FDIVR.
- There is one call to the native ftol helper at 0x00701961.
- Native code does not branch around a zero divisor.

#### 2b. Veteran armor, a separate conversion

If the target is Veteran and its VeteranAbilities.Armor byte at TechnoType+0x29D is set, divide by Rules+0x688 and ftol. If it is Elite, VeteranAbilities.Armor or EliteAbilities.Armor at +0x2AF enables the same divide. This is a second conversion boundary at 0x007019D1.

#### 2c. Minimum one

At 0x007019D8–0x007019DD:

    if (*pDamage < 1) *pDamage = 1;

Because the outer predicate is nonnegative rather than positive, incoming zero becomes one when defenses are not ignored.

#### 2d. TypeImmune

TypeImmune returns zero only when all are true:

1. attacker is non-null;
2. target type byte TechnoType+0xC8C is true;
3. attacker and target return the same exact type pointer from virtual +0x84;
4. attacker and target have the same exact owner pointer at Techno+0x21C.

The return at 0x00701A2C does not write zero through pDamage. The already transformed integer remains visible to the caller.

### 3. Active invulnerability timer, virtual +0x160

Raw vtable bytes at 0x007F4AC0 resolve +0x160 to 0x0041BF40. That helper tests timer fields Techno+0x18C/+0x194 against the current frame and returns true while time remains.

The hit is nullified only when:

    invulnerabilityActive && !ignoreDefenses && !originalNegative

Before zeroing *pDamage, the receiver calls FUN_0048A620. The amount register is *pDamage << 1; the copied target coordinate and constants 1 plus a selector are passed, where the selector is 6 when Techno+0x1C4 == 1 and 1 otherwise. Then *pDamage is written to zero and the receiver returns zero.

This status is the shared invulnerability mechanism used by Iron Curtain / Force Shield. The exact semantic name of Techno+0x1C4 is not established in this slice.

### 4. WarpingOut, virtual +0x1D4

Raw vtable bytes at 0x007F4B34 resolve +0x1D4 to 0x0070C5B0, which returns byte Techno+0x270.

The hit is nullified when:

    warpingOut && !ignoreDefenses

There is no negative-damage exemption. WarpingOut writes *pDamage = 0 and returns zero before readiness.

This proves the current Rust labels are reversed: +0x160 is not WarpingOut, and +0x1D4 is WarpingOut rather than ForceShield.

### 5. DamageReducesReadiness

The gate is TechnoType+0x6B1. It is tested after WarpingOut and is not conditioned on ignoreDefenses, original sign, warhead presence, or any later immunity.

All operands below are signed 32-bit integers except the multiplier:

- currentAmmo: Techno+0x2FC, int32
- maxAmmo: TechnoType+0x684, int32
- Strength: TechnoType+0xA0, int32
- ReadinessReductionMultiplier: TechnoType+0x6B4, float32 loaded into x87

The exact calculation is:

    ratio = (x87) *pDamage / Strength
    scaled = ReadinessReductionMultiplier * ratio
    newAmmo = ftol((double)currentAmmo - (double)maxAmmo * scaled)
    currentAmmo = max(newAmmo, 0)

There is one final ftol after the complete expression. There is no upper clamp.

Consequences that must be preserved:

- Positive damage can reduce ammo.
- Negative damage can increase ammo above maxAmmo.
- Zero damage still calls the timer helper, even if ammo remains unchanged.
- ignoreDefenses does not skip readiness.
- A later bunker or warhead gate does not undo the ammo/timer side effect.
- Native code contains no Strength-zero guard.

After the write/clamp, TechnoClass calls FUN_006FB080.

#### Reload-timer scheduling in FUN_006FB080

If currentAmmo >= maxAmmo, it returns without a timer write. Otherwise:

- If currentAmmo == 0 and EmptyReload at TechnoType+0x69C is not -1, duration = EmptyReload.
- Else group = 1 when TechnoType+0x3E4 PipWrap is zero; otherwise group = signed currentAmmo / PipWrap.
- duration = Reload(+0x698) + ReloadIncrement(+0x6A0) × group × group.
- It writes current frame to Techno+0x1FC and duration to Techno+0x204.
- It also writes the uninitialized local stack dword at [ESP+8] to Techno+0x200.

No direct animation or audio call exists in this helper. The semantic role and downstream consumption of +0x200 remain UNKNOWN; the assembly-level indeterminate write is verified and must not be silently replaced with a claimed native constant.

#### Parser and defaults

TechnoTypeClass constructor 0x00710AF0 establishes:

| Field | Default |
|---|---:|
| InitialAmmo +0x680 | -1 |
| Ammo +0x684 | -1 |
| Reload +0x698 | 0 |
| EmptyReload +0x69C | -1 |
| ReloadIncrement +0x6A0 | 0 |
| DamageReducesReadiness +0x6B1 | false |
| ReadinessReductionMultiplier +0x6B4 | 0.0f |

TechnoTypeClass::ReadINI at 0x007148B8–0x007148EB reads DamageReducesReadiness with ReadBool and ReadinessReductionMultiplier with ReadDouble, using the current float value as the default and storing the result back as float.

Stock rulesmd.ini contains the definitions and a commented AEGIS example, but no live assignment. Thus the mechanism is live/mod-capable YR code and stock-disabled, not dead TS-only code.

### 6. Bunker/link routing

This branch runs only when Techno+0x2E4 is nonzero and ignoreDefenses is false.

| Target kind | Warhead | PenetratesBunker +0x146 | Result |
|---|---|---:|---|
| Building, WhatAmI == 6 | null | n/a | Jump directly to ObjectClass; all warhead immunities are skipped. |
| Building | non-null | true | Write *pDamage = 0; return zero. |
| Building | non-null | false | Continue to warhead immunities. |
| Non-Building | null | n/a | Jump directly to ObjectClass; all warhead immunities are skipped. |
| Non-Building | non-null | true | Continue to warhead immunities. |
| Non-Building | non-null | false | Get target cell through virtual +0x1BC, call Look_up_building_in_cell at 0x0047C520, and nullify only if the returned building pointer equals Techno+0x2E4. |

The counterintuitive Building row is assembly-verified at 0x00701B8A–0x00701BB3. PenetratesBunker is not a universal “bypass this protection” switch.

### 7. Warhead immunities

If warhead is null, the receiver jumps to ObjectClass before these checks.

For non-null warheads, the order is:

1. Warhead+0x177 Radiation && TechnoType+0xD37 ImmuneToRadiation.
2. Warhead+0x178 PsychicDamage && TechnoType+0xD36 psychic immunity.
3. Warhead+0x156 Poison && TechnoType+0xD3B poison immunity.
4. Warhead+0x179 AffectsAllies is false && attacker is non-null && attackerOwner.IsAlliedWith(targetOwner).

Each of these four accepted nullifications writes *pDamage = 0 and returns zero.

HouseClass::IsAlliedWith at 0x004F9A50 treats a null argument as false, identical house pointers or identical house indices as true, and otherwise tests the receiver house's alliance bitset at +0x5788.

sourceHouse is not consulted by AffectsAllies. An allied sourceHouse with a null attacker does not activate this gate.

### 8. Psychedelic path

When Warhead+0x16D Psychedelic is true, the receiver checks:

1. targetOwner.IsAlliedWith(sourceHouse);
2. TechnoType+0xD35 ImmuneToPsionics;
3. target WhatAmI == 6, Building.

Each of those returns zero without writing *pDamage.

Otherwise the receiver:

1. leaves distance 0 as the second stack argument;
2. obtains armor index from target type +0x9C and pushes it as the first stack argument;
3. calls FUN_00489180 with ECX = *pDamage and EDX = the actual warhead;
4. stores the returned integer in *pDamage and Techno+0x29C;
5. if Techno+0x298 was false, sets it true, conditionally calls 0x006EA870 for the passenger-bearing object case, then calls virtual +0x3C8 with 0 and virtual +0x1E8 with 0xF, 0;
6. returns state 1 without calling ObjectClass::ReceiveDamage.

The returned kernel value can be positive, zero, or negative. Calling this “mind control with zero HP damage” destroys verified state and pointer semantics.

## Negative-damage kernel correction

FUN_00489180 uses fastcall registers plus two stack arguments:

    ECX = signed damage
    EDX = WarheadTypeClass pointer
    stack argument 1 = armor index
    stack argument 2 = signed distance in leptons

Its prologue is SUB ESP,0xC; PUSH ESI; PUSH EDI. Therefore the original two stack arguments are visible at [ESP+0x18] and [ESP+0x1C].

The negative branch is:

    MOV EDI,[ESP+0x1C]
    CMP EDI,8
    SETGE AL
    DEC EAX
    AND EAX,ESI

That is:

    if damage < 0:
        return damage when signed distance < 8
        return 0 when signed distance >= 8

The positive Verses lookup later uses [ESP+0x18] as armor index, independently proving that the negative comparison is not armor.

The Psychedelic caller at 0x00701D4D pushes distance 0 first, then armor, so it conforms to this signature.

## ObjectClass delegation and immediate Techno postlude

When no Techno gate returns early, 0x00701DCC–0x00701DF8 calls ObjectClass::ReceiveDamage at 0x005F5390 with the same seven arguments.

The bounded Techno-owned postlude is:

1. If attacker is non-null, compute:

       threatDelta = ftol(
           (double)*pDamage / targetStrength
           * targetType.virtual_0xAC()
       )

   Then call targetOwner.Update_Threat_Score(attackerOwner, threatDelta). This happens even when ObjectClass returned state 0.

2. Result state 4 enters the delayed-death branch; its internals are out of scope. Result state 5 can return early.

3. For a nonzero result reaching 0x00701FA6, write:

   - Techno+0x174 = current frame
   - Techno+0x178 = distance argument
   - Techno+0x17C = Rules+0x8C

4. If result is nonzero and not 4, TechnoType+0xD2F Trainable is true, and TechnoType+0xD30 DamageReducesReadiness is false:

   - call virtual +0xC4;
   - if true, call virtual +0x470;
   - write Techno+0x1E0 = current frame, +0x1E4 = distance, +0x1E8 = *pDamage << 1.

5. On the shared non-dead tail, if attacker is non-null and targetOwner.Is_Ally_ByObject(attacker) is false, set Techno+0x3D1 = 1, the WasAttacked byte.

6. Call virtual +0xFC. Its semantic name is UNKNOWN in this report.

7. Maintain the health-particle system at Techno+0x310 using the post-Object health ratio, Rules ConditionYellow at +0x1700, result states 2/3, the target type's particle list, particle-system type field +0x2B4, target height, and RNG selection.

8. Test originalNegative. If true, return after particle maintenance.

9. Otherwise call TechnoClass::ShouldRetaliate at 0x007087C0 with attacker and warhead, then continue to retaliation or scatter handling.

This section intentionally records the receiver boundary and ordering without claiming the internals of state 4, ShouldRetaliate, or scatter.

## Mutable arithmetic inputs and writers

### Techno+0x158: per-unit armor multiplier, double

- TechnoClass constructor 0x006F2B40 writes IEEE-754 double 1.0 at 0x006F2BF2/0x006F2BF8.
- The full-image write-pattern sweep found the active gameplay writer at the Armor crate path around 0x00482D56–0x00482EC0.
- That path selects nearby eligible Foot objects, requires the current double to equal 1.0, multiplies it by the Powerups Armor data value, and stores the result at 0x00482E79.
- Stock rulesmd.ini:30346 supplies Armor's 1.5 multiplier.
- Generic Techno raw save/load preserves the field.
- No transient combat-status writer to +0x158 was found in the bounded writer sweep. Iron Curtain and WarpingOut do not write it.

### Techno+0x150: veterancy, float

- The Techno constructor calls the veterancy initializer, which writes 0.0.
- Add_Experience at 0x0074FF50 writes:

      new = old
          + killedCost
            / (ownCost * Rules.VeteranRatio_at_0x668)

  and applies the verified VeteranCap behavior at Rules+0x698.
- Its direct active callers include RecordKill and the two TemporalClass update paths.
- SetRookie at 0x00750080 writes 0.0.
- SetVeteran at 0x00750090 writes 1.0 when enabled, otherwise 0.0.
- SetElite at 0x007500B0 writes 2.0 when enabled, otherwise 0.0.
- Active setter callers cover per-country VeteranInfantry/VeteranUnits/VeteranAircraft lists, spy-derived future-unit bonuses, team/script rank, scenario placement, building/undeploy-related creation paths, and crate promotion.
- Generic raw save/load preserves the float.

The stale inference that House+0x2BF is a propagation target for [SpecialFlags] InitialVeteran must not be reused. It is a spy-derived per-house bonus consumed by infantry creation; the vehicle bonus uses +0x2C0. The global InitialVeteran path is distinct and promotes through SetElite in the verified starting-unit paths.

## Current Rust disparity

The damage module explicitly says it is a shadow service and is not wired into live apply sites at src/sim/combat/damage/mod.rs:20–21. The live path at src/sim/combat/mod.rs:1849–1883 performs a separate invulnerability check, unsigned saturating HP subtraction, building-damage refresh, fear, death collection, and last-attacker assignment.

Specific drift:

| Rust location | Current behavior | Native requirement |
|---|---|---|
| damage/receive.rs:49–66 | Defender prefix runs only when dmg > 0 and adds zero-divisor guards. | Run when !ignoreDefenses && !originalNegative, including zero; native has no such guards. |
| damage/mod.rs:98–101 and gates.rs:19–25 | +0x160 is named WarpingOut and +0x1D4 ForceShield. | +0x160 is active invulnerability timer; +0x1D4 reads WarpingOut +0x270. |
| gates.rs:14–56 | Gates are unconditional booleans after arithmetic. | Sign and ignoreDefenses alter TypeImmune/invulnerability/WarpingOut/bunker, while readiness and later warhead gates have different predicates. |
| damage/mod.rs:102–104 and gates.rs:27–30 | Bunker is one precomputed blocked boolean based on not penetrating. | Preserve target kind, link pointer, warhead nullability, PenetratesBunker direction, cell lookup, and pointer equality. |
| gates.rs:44–54 | One is_allied flag is reused and Psychedelic becomes MindControlled with zero HP delta. | AffectsAllies and Psychedelic use different owner/source inputs; accepted Psychedelic calls the kernel and stores state. |
| damage/kernel.rs:54–58, 121–128 | Negative damage is retained only for armor index below 8; tests enforce it. | Negative damage is retained only for signed distance below 8 leptons. |
| game_entity.rs:224, 371 | Veterancy is u16; runtime ammo is aircraft-specific. | Receiver consumes float veterancy, signed general Techno ammo, a per-unit armor double, timers/status bytes, and exact pointer/source distinctions. |
| rules/object_type.rs and rules/warhead_type.rs | Only a subset of receiver flags exists; relevant WarheadType comments also do not match these live offsets. | Parse/store all verified TechnoType and Warhead gates with native defaults and roles. |

## Exact-form implementation handoff

### H1. Preserve one mutable damage integer and gate-specific write semantics

Implement the receiver as an ordered state machine over a signed int32 damage value plus originalNegative. Do not collapse early exits into one Nullified outcome. Each exit must specify whether it:

- leaves *pDamage unchanged;
- writes zero;
- writes the Psychedelic kernel result;
- returns state 0 or state 1;
- occurs before or after readiness side effects.

The order is:

    sign snapshot
    -> conditional armor/veterancy/min-one/TypeImmune
    -> invulnerability
    -> WarpingOut
    -> readiness + reload timer
    -> bunker routing
    -> Radiation
    -> PsychicDamage
    -> Poison
    -> AffectsAllies
    -> Psychedelic
    -> ObjectClass receiver
    -> Techno postlude

### H2. Add the signed readiness and runtime state before cutover

The implementation needs signed int32 current/max ammo, the float multiplier, reload/empty/increment values, PipWrap grouping, reload timer state, the per-unit armor double, float-tier veterancy semantics, owner/source identities, bunker link/cell lookup, invulnerability and WarpingOut states, and the receiver postlude fields. Preserve the single final ftol, lower-only clamp, negative-damage ammo increase, and later-nullified-hit side effects.

The +0x200 timer dword must remain an explicit implementation blocker until its consumers establish whether a deterministic Rust value is byte-equivalent.

### H3. Correct the kernel signature and golden coverage

Change the negative branch to use signed distanceLeptons < 8, not armor. Add binary-derived or oracle-backed coverage for distances -1, 0, 7, 8, and 9 across multiple armor indices, including 8–10. Existing Rust-only tests are regression checks and must not be presented as gamemd parity evidence.

## Do-not-do constraints

1. Do not use armor index to gate negative damage in FUN_00489180.
2. Do not keep +0x160 named WarpingOut or +0x1D4 named ForceShield/invulnerability.
3. Do not collapse bunker routing, attacker-owner alliance, and sourceHouse alliance into precomputed booleans that erase native operands and order.
4. Do not run the defender prefix only for positive damage, add native-absent divisor/Strength guards, or omit the incoming-zero-to-one case.
5. Do not call FUN_006FB080 an animation helper, clamp readiness increases to max ammo, or move readiness after immunity gates.

## Adversarial checks

| Case | Verified result |
|---|---|
| incoming damage = 0, ignoreDefenses = false | Defender arithmetic executes, min-one writes 1, then gates continue. |
| negative damage, invulnerability active, not WarpingOut | Invulnerability does not block it; readiness can increase ammo; later gates still apply. |
| negative damage, WarpingOut active, ignoreDefenses = false | WarpingOut writes zero and returns before readiness. |
| ignoreDefenses = true | Skips armor/veterancy/min-one/TypeImmune, suppresses invulnerability and WarpingOut, still runs readiness and warhead gates, and passes the flag to ObjectClass. |
| attacker = null, allied sourceHouse, AffectsAllies = false | AffectsAllies is skipped; a Psychedelic warhead still performs the separate targetOwner/sourceHouse alliance test. |
| readiness reduces ammo to zero, later immunity matches | EmptyReload scheduling happens first; later immunity can still zero damage. No direct readiness animation is called. |
| Psychedelic, non-allied, non-building, non-immune | Kernel runs at distance 0, result is stored in two places, status/callbacks run, return is 1, and ObjectClass HP mutation is skipped. |
| negative kernel damage, armor 10, distance 0 | Negative value is retained; armor is irrelevant to this branch. |
| negative kernel damage, armor 0, distance 8 | Returns zero; signed distance reaches the cutoff. |

## Cold spot-checks

Two fresh assembly reads were used after the main analysis:

1. disassemble_function(0x00489180) reconfirmed the prologue-adjusted argument positions and signed CMP of [ESP+0x1C] against 8.
2. disassemble_function(0x00701900) reconfirmed the independent status-gate range 0x00701A3B–0x00701AD8 and the readiness formula range 0x00701ADB–0x00701B67.

Both reproduced the claimed instructions without relying on decompiler variable names.

## Zero-add pass

A final full disassembly call inventory for 0x00701900 was compared against the bounded state-machine ledger. No additional Techno-owned early-return gate was found between entry and ObjectClass delegation.

The remaining direct/virtual calls after ObjectClass belong to the state-4/result dispatch, result timers, WasAttacked/refresh, health particles, retaliation, and scatter/death consequence branches already either bounded above or explicitly excluded. No new load-bearing receiver-prefix predicate was added from a label alone.

## Stale or misleading wording to supersede

- RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md:334–344 and 477–485 say armor classes 8–10 block negative healing. Supersede with the signed distance < 8-lepton rule. DAMAGE_MATH_GHIDRA_REPORT.md:44–56 already carries the corrected distance interpretation.
- GATE_DAMAGE_COUNTRY_ARMOR_ORDER_RESOLUTION_GHIDRA_REPORT.md:101 calls +0x160 WarpingOut and +0x1D4 ForceShield/invulnerability and describes the Psychedelic path as zero HP. Supersede all three statements with F04/F10.
- RECEIVE_DAMAGE_GHIDRA_REPORT.md:87 and 597 call 0x006FB080 an ammo-depletion animation trigger. Supersede with the reload-timer scheduler formula.
- RECEIVE_DAMAGE_GHIDRA_REPORT.md:414 says PenetratesBunker bypasses bunker protection. That wording is too broad and is wrong for the linked-Building branch, where PenetratesBunker=true nullifies.
- VETERANCY_SYSTEM_GHIDRA_REPORT.md:227–238 treats House+0x2BF as InitialVeteran propagation. Its own later correction begins at 1682; the stronger current statement is that +0x2BF is a spy-derived per-house infantry bonus, while the global InitialVeteran path is separate.
- Current Rust repeats several of these stale claims in damage/mod.rs:98–104, gates.rs:19–54, and kernel.rs:54–58.

## Coverage ledger

| Question | Status | Closure |
|---|---|---|
| Q01 Function identity and active reachability | COMPLETE | Vtable, RTTI, direct callers. |
| Q02 Seven-argument contract | COMPLETE | RET 0x1C and Object delegation push order. |
| Q03 Original sign snapshot | COMPLETE | Assembly SETL and preserved stack byte. |
| Q04 House/per-unit armor grouping | COMPLETE | x87 sequence and one ftol. |
| Q05 Veterancy tier/ability divide | COMPLETE | Helper calls, ability bytes, separate ftol. |
| Q06 Incoming zero behavior | COMPLETE | CMP against 1 under nonnegative outer gate. |
| Q07 TypeImmune predicate/write behavior | COMPLETE | Type/owner comparisons and write-free return. |
| Q08 Vtable +0x160 role/predicates | COMPLETE | Raw slot plus helper body. |
| Q09 Vtable +0x1D4 role/predicates | COMPLETE | Raw slot plus +0x270 accessor. |
| Q10 Readiness formula and widths | COMPLETE | x87 assembly. |
| Q11 Readiness helper side effects | PARTIAL | Scheduling formula complete; +0x200 semantic/consumer UNKNOWN. |
| Q12 Bunker branch truth table | COMPLETE | All target/warhead/flag branches. |
| Q13 Radiation/Psychic/Poison gates | COMPLETE | Order, offsets, writes. |
| Q14 AffectsAllies operands | COMPLETE | Attacker owner receiver, target owner argument. |
| Q15 Psychedelic operands/state/writes | COMPLETE | Separate sourceHouse alliance, kernel call, status/callbacks. |
| Q16 Negative kernel operand | COMPLETE | Cold assembly plus caller stack layout. |
| Q17 ObjectClass delegation boundary | COMPLETE | Same seven arguments. |
| Q18 Immediate Techno postlude | PARTIAL | Shared ordering complete; state-4 and retaliation internals intentionally excluded. |
| Q19 Arithmetic runtime writers | COMPLETE for bounded fields | +0x158 and +0x150 writer inventories. |
| Q20 Rust/data/stock activation scan | COMPLETE | Direct source and INI reads. |

## Remaining uncertainties

1. arg6_unknown is passed unchanged to ObjectClass::ReceiveDamage, but its semantic name is UNKNOWN.
2. Techno+0x200 receives an indeterminate local-stack dword in FUN_006FB080. Its field meaning and all consumers were not followed; this remains a byte-parity blocker for implementing that timer structure.
3. The exact semantic names of Techno+0x1C4 and virtual +0xFC are UNKNOWN. Their receiver-side values, call positions, and branches are verified.
4. The state-4 delayed-death internals and the full ShouldRetaliate/scatter state machine are separate investigations, not evidence gaps in the prefix/gate rows.

## Final status

PARTIAL, with the partial label limited to Q11, Q18, and semantic-name/consumer items listed above. The implementable receiver-prefix, immunity order, readiness arithmetic, bunker truth table, alliance source distinctions, Psychedelic state writes, and negative-distance kernel correction are assembly-verified for active Yuri's Revenge.
