# Drive Rules Fields Speed Inputs - Ghidra Research Report

**Address(es):** `0x004B2630` (`DriveLocomotionClass::Process_Movement`), `0x004B0F20` (`DriveLocomotionClass::Process_Drive_Track`), `0x004DB1A0` (`FootClass::GetCurrentSpeed`), `0x004D3710` (`TechnoClass::SetSpeedFraction`), `0x00712170` / `0x0071464C..0x00715416` (`TechnoTypeClass::ReadINI` movement keys), `0x00674000` (`RulesClass::ReadSpeedTypeLandTypeTable`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** normal Drive unit speed input fields only: `Speed=`, `Accelerates=`, `AccelerationFactor=`, `DeaccelerationFactor=`, `SlowdownDistance=`, `SpeedType=`, terrain/slope speed table inputs, health/veteran/current-speed multipliers that feed Drive movement speed.
**Non-Scope:** full Drive track stepping, collision/repath state, queue/order lifecycle, ship/hover/walk/aircraft speed algorithms, exact non-Drive use of the same INI keys.
**Confidence:** High for listed Drive input reads and parser/default offsets; Medium for Rust status because Rust was scanned by file search, not compiled in this report.
**Active in YR:** Yes. Normal stock YR vehicles use `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` and reach `DriveLocomotionClass::Process`.

## Working Notes Required By Slot

- Target question: Which rules/INI/TechnoType/Locomotor fields feed normal DriveLocomotion speed fraction and movement speed, and which Rust surfaces must consume them?
- Non-goals: Do not solve full Drive residual timing, full track stepping, entity/object queued destinations, or non-Drive locomotor behavior.
- Evidence needed to mark COMPLETE: parser/default evidence for each in-scope field, Drive consumer evidence for each field that affects speed, negative evidence for attractive wrong shortcuts, and at least one Rust-facing implementation handoff.
- Stop conditions: stop after normal Drive speed input set is enumerated and every in-scope open question is resolved or explicitly deferred.

## 1. Overview

Normal Drive movement has two distinct speed stages. `Process_Movement` computes `DriveLocomotion+0x50`, the target speed fraction, from `SpeedType` terrain table lookup, uphill/downhill slope multipliers, and low-health penalty. `Process_Drive_Track` then updates `TechnoClass+0x578` current speed fraction using `Accelerates`, `SlowdownDistance`, `AccelerationFactor`, and `DeaccelerationFactor`; after that it calls `FootClass::GetCurrentSpeed`, adds `DriveLocomotion+0x4C` residual budget, and consumes movement in 7-unit Drive-track chunks.

Active in YR: Yes. Evidence: `DriveLocomotionClass::Process @ 0x004B0500` calls `Process_Movement` and `Process_Drive_Track` for active Drive locomotors; stock vehicle INI sections use the Drive CLSID.

## 2. Class Layout / Key Offsets

| Field | Owner | Offset | Type | Verified role | Active in YR |
|---|---:|---:|---|---|---|
| `Speed=` | `TechnoTypeClass` | `+0x678` | int, scaled `raw*256/100`, clamped `0..255`; `-1` skips store | base speed read by `GetTypeSpeed`/`GetCurrentSpeed` | Yes; parser `0x0071464C..0x00714699`, reader `0x0070EFE0`, `0x004DB1C3..0x004DB1D5` |
| `SpeedType=` | `TechnoTypeClass` | `+0x67C` | int enum | indexes `g_SpeedType_LandType_Table[SpeedType + LandType*9]` | Yes; parser store `0x007121E5`, Drive read `0x004B3C98..0x004B3CA3` |
| `SlowdownDistance=` | `TechnoTypeClass` | `+0x2F8` | int | distance threshold for braking branch | Yes; parser store `0x00712487`, Drive read in `0x004B0F87..0x004B1211` |
| `DeaccelerationFactor=` | `TechnoTypeClass` | `+0x300` | double | brake/down-ramp decrement, multiplied by `owner.vtable+0x38C` speed | Yes; parser store `0x007124A8`, Drive reads `type+0x300` |
| `AccelerationFactor=` | `TechnoTypeClass` | `+0x308` | double | up-ramp increment toward target fraction | Yes; parser store `0x007124C9`, Drive reads `type+0x308` |
| `Accelerates=` | `TechnoTypeClass` | `+0xDBD` | bool, default true | false skips ramp and directly assigns `loco+0x50` to current fraction | Yes; constructor writes `+0xDBD=1`; parser `0x007153FB..0x00715416`; Drive branch `0x004B0F69..0x004B1269` |
| target speed fraction | `DriveLocomotionClass` | `+0x50` | double | target fraction from terrain/slope/health, consumed by ramp/false branch | Yes; `0x004B3CA3..0x004B3DB?`, `0x004B1261..0x004B1269` |
| residual movement budget | `DriveLocomotionClass` | `+0x4C` | int | leftover budget added after `GetCurrentSpeed` | Yes; `0x004B126F..0x004B128B`, stored near function tail |
| current speed fraction | `TechnoClass`/`FootClass` | `+0x578` | double | clamped by `SetSpeedFraction`, multiplied in `GetCurrentSpeed` | Yes; setter `0x004D3710`, multiplier `0x004DB20D` |
| slope speed factor cache | `FootClass` | `+0x580` | double | multiplier in `GetCurrentSpeed`, separate from Drive target fraction | Yes; `0x004DB1D5`; exact writer out-of-scope |

## 3. Core Logic

### 3.1 Parser and defaults

- `TechnoTypeClass::ReadINI` reads `Speed=` at string `0x0081D9CC`: `ReadInt(default=-1)`, if result is `-1` it skips the write; otherwise clamps above `100`, coerces nonpositive to `0`, computes `raw * 256 / 100` via integer divide-by-100 idiom, clamps above `0xFF`, and stores to `+0x678` at `0x00714699`. Active in YR: Yes.
- `SpeedType=` is read through `CCINIClass::ReadSpeedType @ 0x00476FC0` and stored to `TechnoType+0x67C` at `0x007121E5`. The constructor stores the third constructor argument into `+0x67C`; stock ordinary ground vehicles that omit `SpeedType=` are treated as Track by existing docs/traces, but the exact UnitType constructor argument value was not re-opened in this slot. Active in YR: Yes.
- Constructor defaults: `SlowdownDistance=500` at `+0x2F8`; `DeaccelerationFactor=0x3F60624D_D2F1A9FC` (about `0.002`) at `+0x300`; `AccelerationFactor=0x3F9EB851_EB851EB8` (about `0.03`) at `+0x308`; `Accelerates=true` at `+0xDBD`. Active in YR: Yes.
- `TechnoTypeClass::ReadINI` overwrites movement factors preserving current value as default: `SlowdownDistance` store `0x00712487`, `DeaccelerationFactor` store `0x007124A8`, `AccelerationFactor` store `0x007124C9`, `Accelerates` store `0x00715416`. Active in YR: Yes.

### 3.2 Drive target fraction calculation

`DriveLocomotionClass::Process_Movement @ 0x004B2630` selects a LandType/height context, calls owner vtable `+0x84`, reads `TechnoType+0x67C`, computes `SpeedType + LandType*9`, loads a float from `0x0089EA40`, and caps above `1.0` to `1.0` (`0x004B3C98..0x004B3CCB`). If the loaded value equals zero, it later becomes `0.5`, not zero. Active in YR: Yes.

For standard Drive objects (`WhatAmI()==1`), uphill/downhill branch uses Track-specific globals when `SpeedType==1`, otherwise Wheeled-specific globals:

- Uphill if next ground height is greater than current: `SpeedType==1` multiplies by `Rules+0x768`; other speed types multiply by `Rules+0x778`.
- Downhill if next ground height is lower than current: `SpeedType==1` multiplies by `Rules+0x770`; other speed types multiply by `Rules+0x780`.

Evidence: `0x004B3D35..0x004B3D5A` for type read and `Rules+0x768`, matching branch shape in decompile. Active in YR: Yes.

After slope, `ObjectClass::GetHealthRatio(owner)` is compared against `Rules+0x1700`; if at or below threshold, target fraction is multiplied by a binary constant at `0x007E7FC0` (exact numeric value not re-read in this slot). Active in YR: Conditional; active for damaged objects below the threshold.

If `DriveLocomotion+0x58 < 0x40`, the target is stored into `DriveLocomotion+0x50`; otherwise, if the target differs from current `TechnoClass+0x578`, `SetSpeedFraction(target)` is called immediately. Active in YR: Yes.

### 3.3 Drive ramp/brake/current speed calculation

`Process_Drive_Track @ 0x004B0F20` reads `TechnoType+0xDBD` at `0x004B0F69..0x004B0F81`. If zero, it calls owner vtable `+0x544` with the double at `DriveLocomotion+0x50` (`0x004B1261..0x004B1269`). No acceleration/deceleration math runs on that branch. Active in YR: Yes, stock vehicles such as `[MTNK]` use `Accelerates=false`.

If `Accelerates=true`, the ramp branch reads destination distance, `type+0x2F8`, owner current speed fraction `+0x578`, owner vtable `+0x38C` speed, `type+0x300`, and `type+0x308`:

- within `SlowdownDistance`, subtract `owner_type_speed * DeaccelerationFactor` from current fraction and clamp to a binary minimum brake fraction at `0x007E6240/44`;
- if owner `+0x3CD` is set, subtract `owner_type_speed * const(0x007E6250)` and clamp to another minimum at `0x007E6248/4C`;
- otherwise if current fraction is below target, add `AccelerationFactor` and clamp down to target;
- if current fraction is above target, subtract `owner_type_speed * DeaccelerationFactor` and clamp up to target.

Evidence: decompile `0x004B0F87..0x004B1211` plus assembly around `0x004B0F69`, `0x004B1261`. Active in YR: Yes for types with default/true `Accelerates`.

After either branch, Drive calls owner vtable `+0x538` (`FootClass::GetCurrentSpeed`) at `0x004B126F..0x004B1274`, adds `DriveLocomotion+0x4C`, and consumes movement in 7-unit chunks. Active in YR: Yes. Exact residual stepping is covered by other swarm slots, not this input report.

### 3.4 FootClass::GetCurrentSpeed input composition

`FootClass::GetCurrentSpeed @ 0x004DB1A0` applies, in order:

1. `HouseClass::GetSpeedBonus(type)` result, then owner vtable `+0x38C` speed (`TechnoClass::GetTypeSpeed @ 0x0070EFE0`, which reads `TechnoType+0x678`), multiplied and rounded through `ftol`.
2. multiply by `FootClass+0x580` cached slope speed factor.
3. if `TechnoClass::HasWeaponAbility(0)` is true, multiply by `RulesClass+0x678` (`VeteranSpeed`) and round.
4. multiply by `FootClass/TechnoClass+0x578` current speed fraction and round.
5. if `WhatAmI()==1` and `owner+0x6CC != -1`, halve the result.

Evidence: `0x004DB1B6..0x004DB23E`; `0x004DB1FA` is the `Rules+0x678` multiply and `0x004DB20D` is current speed fraction multiply. Active in YR: Yes; some sub-branches are conditional.

## 4. INI Keys

| Key | Owner/source | Binary storage/default | Drive speed effect | Active in YR |
|---|---|---|---|---|
| `Speed=` | `rulesmd.ini` / type section | `TechnoType+0x678`, default `-1` skips parser write; conversion `raw*256/100` clamped to `0..255` | base integer for `GetCurrentSpeed`; also scales brake decrement | Yes |
| `SpeedType=` | type section; default via constructor/subclass | `TechnoType+0x67C`; parsed by `ReadSpeedType` | terrain speed table column and slope key family selector | Yes |
| `Accelerates=` | type section | `TechnoType+0xDBD`, constructor default true | false directly assigns target fraction; true runs ramp/brake | Yes |
| `AccelerationFactor=` | type section | `TechnoType+0x308`, constructor about `0.03` | true-branch up-ramp increment | Yes |
| `DeaccelerationFactor=` | type section | `TechnoType+0x300`, constructor about `0.002` | true-branch down-ramp/brake decrement scaled by type speed | Yes |
| `SlowdownDistance=` | type section | `TechnoType+0x2F8`, constructor `500` | distance threshold for braking branch | Yes |
| LandType speed columns (`Foot`, `Track`, `Wheel`, `Hover`, `Float`, `Amphibious`, `FloatBeach`) | `[LandType]` sections | global table base `0x0089EA40`, stride 9 | raw terrain target fraction before slope/health | Yes; loader `0x00674000` |
| slope globals | `[General]` rules fields | `Rules+0x768/+0x770/+0x778/+0x780` | uphill/downhill multiplier, Track vs non-Track branch | Yes |
| `VeteranSpeed` | `[General]` | `Rules+0x678` | only if `HasWeaponAbility(0)` | Conditional |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `DriveLocomotionClass::Process @ 0x004B0500` | top-level Drive tick; samples slope first, then movement/track calls | decompile | Yes |
| `DriveLocomotionClass::Process_Movement @ 0x004B2630` | computes `loco+0x50` target speed fraction | `0x004B3C98..0x004B3DB?` | Yes |
| `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` | consumes `Accelerates` and current speed fraction; calls current-speed budget | `0x004B0F69..0x004B1274` | Yes |
| `TechnoClass::SetSpeedFraction @ 0x004D3710` | clamps current fraction to `[0.0, 1.0]` | decompile | Yes |
| `FootClass::GetCurrentSpeed @ 0x004DB1A0` | final integer movement budget before residual | assembly context | Yes |

## 6. Current Rust Implementation Status

Rust now exposes the key data surfaces: `ObjectType.speed`, `accel_factor`, `decel_factor`, `accelerates`, `slowdown_distance`, and `speed_type` in `src/rules/object_type.rs`; `World` propagates `drive_accelerates` in `src/sim/world/world_spawn.rs` and `src/sim/world/world_commands.rs`; Drive target fraction scaffolding is in `src/sim/movement/drive_locomotion.rs`.

Mismatch/risk: `src/sim/movement/movement_tick.rs` still applies acceleration/deceleration primarily through generic `MovementTarget.current_speed`, `accel_factor`, `decel_factor`, and `slowdown_distance`, then multiplies by terrain cell modifier. Binary Drive owns current speed fraction at `TechnoClass+0x578`, target fraction at `DriveLocomotion+0x50`, residual at `DriveLocomotion+0x4C`, and final current speed through `FootClass::GetCurrentSpeed`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Speed=` parser and `+0x678` reader | verified | `0x0071464C..0x00714699`, `0x0070EFE0`, `0x004DB1C3..0x004DB1D5` | none for input contract |
| `SpeedType=` parser and Drive terrain read | verified | `0x007121E5`, `0x00476FC0`, `0x004B3C98..0x004B3CA3` | exact UnitType default argument not re-opened |
| speed/land table loader | verified | `0x00674000`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | none for Drive input contract |
| `Accelerates=` parser/default/Drive branch | verified | `0x007153FB..0x00715416`, constructor `+0xDBD=1`, `0x004B0F69..0x004B1269` | none |
| accel/decel/slowdown parser/defaults | verified | constructor `0x00710AF0`; stores `0x00712487`, `0x007124A8`, `0x007124C9` | none |
| exact numeric brake minimum constants | touched-not-exhausted | `0x004B0F87..0x004B1211` references `0x007E6240/48` | covered by speed-ramp slot, not input slot |
| exact residual stepping | deferred | `0x004B126F..`, loop chunk `7` | another swarm slot owns residual/track stepping |
| non-Drive locomotors | deferred | out-of-scope | separate locomotor reports |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Which field stores Speed? -> TechnoType+0x678, parsed/scaled from Speed= and read by GetTypeSpeed/GetCurrentSpeed.` (evidence: `0x0071464C..0x00714699`, `0x0070EFE0`; Active in YR: Yes)
- `[RESOLVED] OQ-002 - Does Drive consume raw Speed directly? -> Not in Process_Movement target fraction; final budget comes via owner vtable+0x538/GetCurrentSpeed, which reads type speed through vtable+0x38C.` (evidence: `0x004B126F..0x004B1274`, `0x004DB1C3`; Active in YR: Yes)
- `[RESOLVED] OQ-003 - Where is SpeedType read for Drive speed? -> Process_Movement reads type+0x67C and indexes g_SpeedType_LandType_Table.` (evidence: `0x004B3C98..0x004B3CA3`; Active in YR: Yes)
- `[RESOLVED] OQ-004 - Are terrain table values capped? -> Loader and Drive target path cap above 1.0; Drive also maps zero target to 0.5.` (evidence: `0x00674000`, `0x004B3CA3..0x004B3CCB`, decompile zero branch; Active in YR: Yes)
- `[RESOLVED] OQ-005 - Are slope multipliers SpeedType-specific? -> Drive branches on SpeedType==1 Track vs all other SpeedTypes for uphill/downhill globals.` (evidence: `0x004B3D35..0x004B3D5A`; Active in YR: Yes)
- `[RESOLVED] OQ-006 - What is Accelerates default? -> true at TechnoType+0xDBD.` (evidence: constructor `0x00710AF0`; Active in YR: Yes)
- `[RESOLVED] OQ-007 - What does Accelerates=false do for Drive? -> direct SetSpeedFraction(loco+0x50), no ramp math.` (evidence: `0x004B0F69..0x004B1269`; Active in YR: Yes)
- `[RESOLVED] OQ-008 - Do AccelerationFactor/DeaccelerationFactor/SlowdownDistance feed Drive? -> yes, only in true branch; false branch skips them for current tick ramping.` (evidence: `0x004B0F87..0x004B1211`; Active in YR: Yes)
- `[RESOLVED] OQ-009 - Does Speed factor into braking? -> yes, Drive calls owner vtable+0x38C and multiplies that integer by DeaccelerationFactor.` (evidence: `0x004B0F87..0x004B1211`; Active in YR: Yes)
- `[RESOLVED] OQ-010 - What clamps current speed fraction? -> SetSpeedFraction clamps <=0 to 0 and >=1 to 1.` (evidence: `0x004D3710`; Active in YR: Yes)
- `[RESOLVED] OQ-011 - What else multiplies final Drive budget? -> speed bonus, slope factor cache, VeteranSpeed if ability 0, current speed fraction, and conditional half-speed flag.` (evidence: `0x004DB1B6..0x004DB23E`; Active in YR: Yes/Conditional)
- `[DEFERRED] OQ-012 - Exact values of brake minimum constants at 0x007E6240/48 and special decel const 0x007E6250.` (category: out-of-scope; reason: assigned to speed-ramp slot, not rules-input slot; next-step-if-pursued: read memory and tie each constant to branch)
- `[DEFERRED] OQ-013 - Exact UnitType constructor default argument for SpeedType on normal vehicles.` (category: bounded-cost-too-high; reason: existing docs/traces and Rust default say Track, but this slot focused on Drive consumer chain; next-step-if-pursued: decompile `UnitTypeClass::Constructor @ 0x007470D0` call to `TechnoTypeClass::Constructor`)

## 9. Negative Facts / Do Not Do

- Do not use `MovementZone` as the Drive speed-table column. Active in YR: Yes; evidence `Process_Movement` reads `TechnoType+0x67C` (`SpeedType`), not `+0x5B4` (`MovementZone`), at `0x004B3C98`.
- Do not treat `Accelerates=false` as "set speed to 1.0" or "ignore terrain". Active in YR: Yes; evidence false branch assigns `DriveLocomotion+0x50`, which was already terrain/slope/health adjusted.
- Do not implement `Accelerates=false` by zeroing `AccelerationFactor`. Active in YR: Yes; stock INI can set both `Accelerates=false` and nonzero accel/decel (for example Drone), and binary uses a separate `+0xDBD` bool.
- Do not apply generic `MovementTarget.current_speed` ramp as Drive parity. Active in YR: Yes; Drive writes/reads `TechnoClass+0x578` and obtains budget via `FootClass::GetCurrentSpeed`.
- Do not read the 9th speed-table slot as a speed column. Active in YR: Yes; `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` proves slot 8 is `Buildable` byte/padding, not a SpeedType.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Drive target fraction comes from `SpeedType + LandType*9`, capped to `1.0`, zero promoted to `0.5`, then slope and health modifiers before storing `DriveLocomotion+0x50`. | `0x004B3C98..0x004B3DB?`, `0x00674000` | partially modeled; zero->0.5 and exact slope/health split unchecked | `src/sim/movement/drive_locomotion.rs`, `src/sim/pathfinding/terrain_speed.rs`, `src/rules/terrain_rules.rs` | Compute Drive target fraction as Drive-owned state before current-speed ramp; preserve terrain table cap and zero fallback. | Track Drive unit entering a LandType with speed table value 0 should receive target fraction 0.5 instead of stopping through generic terrain cost. Proposed test: `drive_target_fraction_zero_terrain_promotes_to_half`. | Do not let pathfinding passability cost zero silently become runtime speed zero for Drive target fraction. |
| `Accelerates=false` directly calls `SetSpeedFraction(loco+0x50)` and then still uses normal `GetCurrentSpeed`/residual/track stepping. | `0x004B0F69..0x004B1269`, `0x004B126F..0x004B1274` | parser/state exists; movement still largely generic | `src/sim/movement/drive_locomotion.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/components.rs` | Make Drive current speed fraction authoritative; false branch assigns target fraction for the tick without changing base `Speed=`. | Stock `[MTNK]` first Drive tick on normal terrain uses immediate target fraction while preserving `Speed=7` base budget. Proposed test: `drive_accelerates_false_assigns_target_fraction_before_budget`. | Do not hardcode Grizzly or replace this with `accel_factor=0`. |
| `Accelerates=true` uses `SlowdownDistance`, `DeaccelerationFactor`, `AccelerationFactor`, current fraction, target fraction, and type speed to ramp/brake before final budget. | `0x004B0F87..0x004B1211`, parser stores `0x00712487/A8/C9` | mismatch; Rust generic ramp subtracts `decel_factor` directly and uses generic distance logic | `src/sim/movement/movement_tick.rs`, `src/sim/movement/drive_locomotion.rs` | Implement Drive ramp helper using `TechnoClass+0x578` semantics and type-speed-scaled decel; leave exact constants to speed-ramp slot. | A default `Accelerates=true` Drive test starts below target, increments by `AccelerationFactor`, brakes within `SlowdownDistance` by `Speed*DeaccelerationFactor`, and clamps to target/min. Proposed test: `drive_accelerates_true_uses_type_speed_scaled_deceleration`. | Do not use the generic `MovementTarget.current_speed` accel/decel path as a parity substitute. |

## Stale Docs / Follow-up Docs

- `docs/research/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`: replace the field-table wording for `TechnoTypeClass+0x678` from `Crushability(?)` with: "`Speed=` parsed/scaled value; `ReadINI @ 0x0071464C..0x00714699` reads default `-1`, skips write on `-1`, clamps raw to `0..100`, scales by `*256/100`, clamps to `0..255`, and stores at `+0x678`. Read by `TechnoClass::GetTypeSpeed @ 0x0070EFE0` and `FootClass::GetCurrentSpeed @ 0x004DB1A0`."
- `docs/research/DRIVE_LOCOMOTION_CLASS.md`: replace any generic `decel_steps` label for `TechnoTypeClass+0x678` in Drive speed prose with: "`+0x678` is the parsed/scaled `Speed=` field; Drive's true-branch deceleration multiplies this type speed by `DeaccelerationFactor`, while final movement budget comes from `FootClass::GetCurrentSpeed`."
- `docs/research/timing/movement-speed-turn-rate.md`: replace "Accelerates=false jumps straight to max_speed" with: "`Accelerates=false` in normal Drive assigns the already computed `DriveLocomotion+0x50` target speed fraction to `TechnoClass+0x578`; this target may already include terrain, slope, and health modifiers, so it is not necessarily `1.0`."

## Sources

- Ghidra read-only decompile: `0x004B0500`, `0x004B0F20`, `0x004B2630`, `0x004D3710`, `0x004DB1A0`, `0x0070EFE0`, `0x00710AF0`, `0x00715402`, `0x00476FC0`, `0x00674000`.
- Ghidra read-only assembly context: `0x004B0F69`, `0x004B1261`, `0x004B126F`, `0x004B3CA3`, `0x004B3D35`, `0x007121E5`, `0x00712487`, `0x007124A8`, `0x007124C9`, `0x00715402`, `0x0071464C`, `0x00714699`, `0x004DB1B6`, `0x004DB1FA`, `0x004DB20D`.
- Existing docs: `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_ADDENDUM.md`, `timing/movement-speed-turn-rate.md`, `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- INI files checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust files scanned: `src/rules/object_type.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/world_commands.rs`, `src/sim/movement/drive_locomotion.rs`, `src/sim/movement/movement_tick.rs`.
