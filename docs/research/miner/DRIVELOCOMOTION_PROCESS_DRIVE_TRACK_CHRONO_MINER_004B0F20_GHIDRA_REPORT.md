# DriveLocomotionClass::Process_Drive_Track Chrono Miner Drive Slice - Ghidra Research Report

**Address(es):** `0x004B0F20` primary; context callers `0x004B0500`, `0x004B2630`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Per-tick speed budget, track-point consumption, and facing update semantics in `DriveLocomotionClass::Process_Drive_Track` as they affect chrono miner drive phases.
**Non-Scope:** Full A* pathfinding, refinery radio protocol, teleport/chrono warp timing, and full drive locomotion collision recovery.
**Confidence:** High for the primary function's budget/facing mechanics; Medium for CMIN-specific runtime activation because this slot did not trace the full piggyback owner chain.
**Active in YR:** Conditional. The function is active for DriveLocomotion units every locomotor tick; for CMIN it is active only when the chrono miner is in a DriveLocomotion/piggyback drive phase, not during teleport phases. Evidence: stock `[CMIN]` uses `Teleporter=yes` and `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` in `ini/rulesmd.ini:7351-7402`, while `DriveLocomotionClass::Process @ 0x004B0500` calls `Process_Drive_Track @ 0x004B0F20` at `0x004B0576` and `0x004B0AAA`.

## 1. Overview

`Process_Drive_Track` advances a ground unit through precomputed drive-track curves. Movement is not "leptons directly per tick"; the function converts current speed into an integer track budget, consumes one track point per 7 budget units, stores the remainder at `DriveLocomotionClass+0x4C`, and interpolates visible coordinates through the next point using a `1/7` factor.

For CMIN drive phases, the visible body turn cadence is therefore tied to track-point consumption. `ROT`/turn-rate state is read from `TechnoTypeClass+0x11C` by drive-facing helpers and `Process`, but this primary function updates the body facing from each consumed track point's heading byte, not from a per-tick standalone ROT formula.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Purpose | Active in YR |
|---:|---|---|---|---|
| `+0x4C` | DriveLocomotionClass | int | residual movement budget after 7-unit track consumption | Yes; read/write in `0x004B0F20` |
| `+0x50` | DriveLocomotionClass | double | local current speed target/ramp value | Yes; read/write in `0x004B0F20`, initialized by Force_Track |
| `+0x58` | DriveLocomotionClass | int | drive track index; `-1` means no active track | Yes; loop/table selector in `0x004B0F20` |
| `+0x5C` | DriveLocomotionClass | int | point index inside active track | Yes; incremented once per consumed point |
| `+0x60` | DriveLocomotionClass | byte | short/reversed track selector for table byte `+1` instead of `+0` | Yes; table lookup branch in `0x004B0F20` |
| `+0x63` | DriveLocomotionClass | byte | active head_to / on-track flag | Yes; early-out and head_to state |
| `+0x5E0` | FootClass | int[24] | path queue; first slot is current movement direction/state | Yes; read by `0x004B0F20` and `0x004B2630` |
| `+0x11C` | TechnoTypeClass | byte | ROT/turn-rate field read by drive facing helper | Yes; `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0` |
| `+0x15E` | TechnoTypeClass | double | base Speed value from INI movement speed path | Yes; used in speed ramp calculations |

## 3. Core Logic

### 3.1 Guards and CMIN relevance

The primary function exits immediately and clears residual budget if no active track is present (`is_on_track == 0` or `track_index == -1`) unless the Foot path queue state is `8`. It also exits if the drive locomotor's deploy flag is set and the type does not allow deploy while moving. Active in YR: Yes for DriveLocomotion; Conditional for CMIN because CMIN only reaches this during drive phases. Evidence: `0x004B0F20` first branch reads `+0x63`, `+0x58`, Foot `+0x5E0`, and TechnoType `+0xCA1`.

### 3.2 Speed budget

The material budget formula is:

```text
speed_units = FootClass::GetCurrentSpeed()           // vtable +0x538, address 0x004DB1A0
budget = (retry_param ? 0 : speed_units) + residual  // residual at DriveLocomotion+0x4C
```

Active in YR: Yes. Evidence: `Process_Drive_Track @ 0x004B0F20` calls vtable `+0x538`, then computes `(~-(param_2 != 0) & speed) + *(this+0x4C)`.

`param_2` is the caller's same-tick retry/chained flag. When nonzero, the function adds no new speed for that second call; only the stored residual carries forward. Active in YR: Yes. Evidence: `Process @ 0x004B0500` calls `Process_Drive_Track(0)` before movement handling, then may call `Process_Drive_Track(1)` after `Process_Movement`.

### 3.3 Track-point consumption

When `budget > 7`, the step loop starts. Each complete track point costs exactly 7 units:

```text
while budget > 7:
    budget -= 7
    point = track_points[point_index]
    apply point / cell transition / facing update
    point_index += 1
residual = budget
```

Active in YR: Yes. Evidence: `0x004B0F20` subtracts `7` before each point fetch, loops while `7 < budget`, then stores the leftover at `this+0x4C`.

The curve source is a 72-entry turn table at `0x007E7B28` with 12-byte entries. Normal track byte is entry `+0`; short/reversed track byte is entry `+1`; target direction byte is entry `+4`; flags are entry `+8`. The raw track-data pointer table is rooted at `0x007E7A28`, and each point is 12 bytes: x, y, heading. Active in YR: Yes. Evidence: table reads in `0x004B0F20`; corroborated by `DRIVE_LOCOMOTION_CLASS.md` and `PROCESS_DRIVE_TRACK_DECOMPILATION.md`.

Track end is a point with `x == 0 && y == 0 && point_index != 0`. The function then clears `head_to`, sets `track_index = -1`, resets `point_index = 0`, and may clear destination when the nav target cell matches and Z differs by less than `g_DriveHeightStep * 2`. Active in YR: Yes. Evidence: `0x004B0F20` track-end branch.

### 3.4 Fractional residual interpolation

After complete point consumption, if residual is positive and a track remains active, the function computes a fractional coordinate toward the next point using `residual * 1/7`. The constant is `0x007E7FA8 = 1/7`. Active in YR: Yes. Evidence: `0x004B0F20` residual branch calls `CoordStruct__ScaleByFactor` with `(float)this+0x4C * _DAT_007e7fa8`; constant documented in `DRIVE_LOCOMOTION_CLASS.md`.

This interpolation updates visible position and occupancy/cell state as needed, but it does not increment `point_index` and does not perform the per-point `FacingClass__UpdateFacing` call. Active in YR: Yes. Evidence: residual branch after the loop stores `+0x4C`, scales coords, and returns after cell/coord updates without the `0x004B1AC1` facing call site.

### 3.5 Facing update cadence

For each consumed mid-track point, the point's heading field (`track_point+8`) is shifted left by 8 and passed to `FacingClass__UpdateFacing`. The call happens after the function updates cell/coord state for that point. Active in YR: Yes. Evidence: `0x004B0F20` sets `sStack_28 = (ushort)(byte)heading << 8` and calls `FacingClass__UpdateFacing` at `0x004B1AC1`.

`FacingClass__UpdateFacing` itself uses timer state: if time remains, it computes an interpolated current facing from target-current delta divided by the rate field; when the facing already equals the requested value, it resets timing fields and returns 0; otherwise it copies the requested target/current pair and returns 1. Active in YR: Yes. Evidence: decompile of `FacingClass__UpdateFacing` at its xrefs, including caller `0x004B1AC1`.

`DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0` reads `TechnoTypeClass+0x11C` and dispatches vtable `+0x7C`, whose implementation at `0x004B0EF0` calls `RateTimer__Set`. `Process @ 0x004B0500` also reads `TechnoTypeClass+0x11C` at tick start and starts a 3-tick `CDTimer` if it changes. Active in YR: Yes for DriveLocomotion; Conditional for CMIN drive phases. Evidence: decompiles of `0x004B04D0`, `0x004B0EF0`, and `0x004B0500`.

### 3.6 ROT/speed interaction

No multiplication or direct formula combines ROT with movement speed inside `Process_Drive_Track`. Speed controls how many track points are consumed this tick; consumed track points trigger heading updates. Therefore a faster unit can visibly turn through more track headings in one tick, but only because it consumes more curve points. Active in YR: Yes. Evidence: `0x004B0F20` budget loop and `0x004B1AC1` facing call; no read of `TechnoTypeClass+0x11C` in the primary decompile.

## 4. INI Keys

| Section / key | Stock value | Effect for this slice | Active in YR |
|---|---:|---|---|
| `[CMIN] Speed` | `4` | Base movement speed feeding TechnoType speed and `GetCurrentSpeed` budget path | Yes; `ini/rulesmd.ini:7351-7402` |
| `[CMIN] ROT` | `5` in INI | Type turn-rate input; prior docs report harvester parse override to 10, but this slot only verified `+0x11C` consumers | Conditional; consumer active, exact CMIN runtime value deferred |
| `[CMIN] Harvester` | `yes` | CMIN participates in harvester mission/dock logic; prior docs claim this also forces ROT override | Yes for harvester behavior; override not reverified here |
| `[CMIN] Teleporter` | `yes` | Explains why DriveLocomotion applies only in drive/piggyback phases, not normal locomotor identity | Yes |
| `[CMIN] Locomotor` | Teleport CLSID | Stock CMIN primary locomotor is TeleportLocomotion | Yes |
| `[CMIN] MovementZone` | `Crusher` | Pathing/cell entry context, not expanded in this slot | Yes, but full A* out of scope |

## 5. Integration Points

`DriveLocomotionClass::Process @ 0x004B0500` is the only direct code caller found for `0x004B0F20`. It calls the function with `param_2 = 0` for normal active-track processing and with `param_2 = 1` after a same-tick `Process_Movement` pass so speed is not double-counted. Active in YR: Yes. Evidence: Ghidra xrefs to `0x004B0F20` from `0x004B0576` and `0x004B0AAA`.

`DriveLocomotionClass::Process_Movement @ 0x004B2630` chooses new track indices and may re-enter movement on block/retry paths, but full A* and blocker handling were not investigated. Active in YR: Yes for DriveLocomotion. Evidence: `0x004B0500` calls `0x004B2630`; `0x004B2630` has recursive self-calls and assigns `track_index` from direction pairs.

## 6. Current Rust Implementation Status

Not audited in this slot. Prior trace `traces/CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` already flags chrono miner drive-track budget parity as unchecked. This report should be used as the binary reference for a follow-up implementation/audit, not as a claim about current Rust correctness.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` speed budget | verified | fresh Ghidra decompile; budget formula and `-7` loop | none for this slice |
| Track table consumption and point cadence | verified | fresh Ghidra decompile; `0x007E7B28`, `0x007E7A28`, `point_index++` | exact contents of every raw track not dumped here |
| Residual interpolation | verified | fresh Ghidra decompile; `CoordStruct__ScaleByFactor`, `0x007E7FA8` | none for cadence semantics |
| Facing update from track heading | verified | fresh Ghidra decompile; `0x004B1AC1` call to `FacingClass__UpdateFacing` | exact frame-to-render mapping is outside scope |
| `DriveLocomotionClass::Process @ 0x004B0500` caller behavior | verified | fresh Ghidra decompile and xrefs | none for `param_2` semantics |
| `DriveLocomotionClass::Process_Movement @ 0x004B2630` context | touched-not-exhausted | fresh xrefs; prior docs | full A* and blocker handling explicitly out of scope |
| CMIN runtime activation of DriveLocomotion | touched-not-exhausted | stock INI + prior chrono miner docs + caller path | full piggyback chain belongs to another slot |
| CMIN exact runtime ROT value after parser overrides | deferred | INI has `ROT=5`; prior doc says harvester override to 10 | verify `TechnoTypeClass::ReadINI` override in a dedicated parser slice |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is the per-tick speed budget formula? `budget = (param_2 ? 0 : FootClass::GetCurrentSpeed()) + residual`. Evidence: `0x004B0F20`.

[RESOLVED] OQ-2 - What is one drive-track point's budget cost? Exactly 7 units, consumed before the point body. Evidence: `0x004B0F20` loop.

[RESOLVED] OQ-3 - Does the same tick's second drive-track call add speed again? No. Nonzero `param_2` masks speed to zero and uses residual only. Evidence: `0x004B0F20`, caller `0x004B0500`.

[RESOLVED] OQ-4 - Does residual movement update facing? No complete point is consumed in the residual interpolation branch, and the `FacingClass__UpdateFacing` call is absent there. Evidence: `0x004B0F20` residual branch.

[RESOLVED] OQ-5 - What drives body facing during curves? The raw track point heading field, shifted left by 8, passed to `FacingClass__UpdateFacing`. Evidence: `0x004B1AC1`.

[RESOLVED] OQ-6 - Is ROT directly multiplied by speed in this function? No direct ROT read or multiplication in `0x004B0F20`; speed changes point cadence, which changes how often heading updates occur. Evidence: fresh primary decompile.

[DEFERRED] OQ-7 - What exact runtime ROT value does CMIN have after all parser overrides? Prior docs claim harvester override to 10, but this slot did not exhaust `TechnoTypeClass::ReadINI`. Category: out-of-scope parser follow-up.

[DEFERRED] OQ-8 - Does Rust currently reproduce the 7-budget/residual/no-double-count cadence? Category: out-of-scope implementation audit.

## Sources

- Ghidra decompiled: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- Ghidra decompiled: `DriveLocomotionClass::Process @ 0x004B0500`
- Ghidra xrefs: `0x004B0F20` called from `0x004B0576`, `0x004B0AAA`
- Ghidra decompiled: `DriveLocomotionClass::Process_Movement @ 0x004B2630` as caller/context only
- Ghidra decompiled: `DriveLocomotionClass__Update_Facing_From_Type @ 0x004B04D0`
- Ghidra decompiled: `DriveLocomotionClass__Do_Turn @ 0x004B0EF0`
- Ghidra decompiled: `FacingClass__UpdateFacing` and `RateTimer__Set`
- INI checked: `ini/rulesmd.ini` `[CMIN]`
- Prior docs referenced: `DRIVE_LOCOMOTION_CLASS.md`, `PROCESS_DRIVE_TRACK_DECOMPILATION.md`, `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md`, `traces/CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md`
