# Paradrop Drop Cadence - Ghidra Research Report

**Address(es):** `0x006CC390`, `0x0065E660`, `0x005B3060`, `0x004158E0`, `0x00415960`, `0x00415C60`, `0x00415EE0`, `0x004155F0`, `0x004157C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard Yuri's Revenge `Type=ParaDrop` / `Type=AmerParaDrop` superweapon aircraft payload cadence only.
**Non-Scope:** Target-cell validation, side list selection beyond cadence call-site evidence, V-pattern geometry beyond direct cadence side effects, passenger descent/rendering, Carryall cargo/rescue behavior outside the paradrop SW path, and unrelated aircraft weapons.
**Confidence:** High
**Active in YR:** Yes for the `Mission_Open` -> `Mission_Rescue` cadence path. No for using `Mission_ParaDropApproach` / `Mission_ParaDropOverfly` or `[ParaDropWeapon] ROF=130` as the standard superweapon cargo cadence.

## 0. Working Notes

Target question: Which active standard YR paradrop superweapon path controls cargo drop cadence: `AircraftClass::Fire_At` / weapon ROF, `Mission_Open` / `Mission_Rescue` 5-frame scheduling, `LandingState`, or a combination?

Non-goals: Do not re-investigate target validation, edge spawn details, passenger type lists, parachute descent visuals, Carryall behavior, or unrelated aircraft combat weapons.

Evidence needed to mark COMPLETE: Superweapon call-site mission argument; mission dispatch mapping for that argument; decompile and assembly evidence for `Mission_Open`, `Mission_Rescue`, `Drop_Payload`, and `Fire_At`; caller/xref evidence proving which drop caller is reachable from standard SW PDPLANE; INI evidence for the dummy weapon value and current Rust comparison points.

Stop conditions: Stop when the launch-to-drop chain is proven, every scoped candidate path is classified Active in YR Yes / No / Conditional, and the Rust handoff can name a concrete cadence test. Defer anything requiring runtime debugger timing capture only if static evidence cannot prove the scheduling.

## 1. Overview

Standard YR paradrop superweapon aircraft are spawned into mission `0x1A`, not mission `0x1E`. `MissionClass::Mission_Dispatch` maps `0x1A` to `AircraftClass::Mission_Open`, and `Mission_Open` switches to mission `0x1B` (`Mission_Rescue`) when the aircraft is within `RulesClass+0x54C` / `ParadropRadius`. `Mission_Rescue` directly calls `AircraftClass::Drop_Payload` and returns `5`, so once in range the normal cadence is one passenger every 5 game frames.

`AircraftClass::Fire_At` also has a cargo gate that calls `Drop_Payload` when `aircraft+0x118` is non-null, but the standard `Type=ParaDrop` / `Type=AmerParaDrop` launch path does not enter the aircraft attack mission or route drops through weapon ROF. `[PDPLANE] Primary=ParaDropWeapon` and `[ParaDropWeapon] ROF=130` are real parsed type data, but they are not the active standard superweapon drop cadence.

## 2. Class Layout / Key Offsets

| Offset | Owner | Purpose | Active in YR | Evidence |
|---|---|---|---|---|
| `MissionClass+0xC8` (`param_1[0x32]`) | Mission timer start frame | Stores `g_CurrentFrameCounter` after mission handler returns. | Yes | `MissionClass::Mission_Dispatch @ 0x005B3060` |
| `MissionClass+0xD0` (`param_1[0x34]`) | Mission timer delay | Stores the handler return delay. | Yes | `MissionClass::Mission_Dispatch @ 0x005B3060` |
| `MissionClass+0xAC` (`param_1[0x2B]`) | Current mission id | Switch selects handler slots; `0x1A`, `0x1B`, `0x1E`, `0x1F` are distinct. | Yes | `MissionClass::Mission_Dispatch @ 0x005B3060` |
| `AircraftClass+0x118` | Cargo head / payload-present pointer | If non-zero, `AircraftClass::Fire_At` calls `Drop_Payload` instead of normal weapon fire. | Conditional | `AircraftClass::Fire_At @ 0x00415EE0`; active only when `Fire_At` is invoked on an aircraft with cargo |
| `AircraftClass+0x5A4` (`param_1[0x169]`) | Cargo/target-live gate used by missions | `Mission_Open` checks non-zero before entering Rescue; `Mission_ParaDropOverfly` uses zero to redirect to exit. | Yes for mission logic | `0x004158E0`, `0x004157C0` |
| `AircraftClass+0x6D2` | Strafe / payload-run byte | `Mission_Rescue` sets it while running drop logic. | Yes | `0x00415960` |
| `AircraftClass+0x6D3` | LandingState byte | `Drop_Payload` writes `5`; `Mission_Open` decrements it when entering Rescue. It does not throttle in-range Rescue drops. | Yes, but not cadence-dominant | `0x00415C60`, `0x004158E0`, `0x00415960` |
| `AircraftClass+0x2FC` (`param_1[0xBF]`) | Payload count | `Drop_Payload` post-decrements before V-pattern parity. | Yes | `0x00415C60` |

## 3. Core Logic

### 3.1 Standard SW launch passes mission `0x1A`

Active in YR: Yes. `SuperClass::Launch` cases 5 and 6 are the active `Type=ParaDrop` and `Type=AmerParaDrop` superweapon launch handlers.

At all four `FUN_0065E660` call-sites used by generic side branches and American paradrop, the call setup pushes `0x1A` immediately before the aircraft count argument:

| Branch | Call-site | Cadence-critical evidence | Active in YR |
|---|---:|---|---|
| Generic Allied | `0x006CD421` | assembly context includes `PUSH 0x1A`, then `PUSH 0x1`, then `CALL 0x0065E660` | Yes |
| Generic Yuri | `0x006CD493` | same `PUSH 0x1A` / `PUSH 0x1` / call pattern | Yes |
| Generic Soviet fallback | `0x006CD4EB` | same `PUSH 0x1A` / `PUSH 0x1` / call pattern | Yes |
| American | `0x006CD655` | same `PUSH 0x1A` / `PUSH 0x1` / call pattern | Yes |

`FUN_0065E660 @ 0x0065E660` calls the aircraft virtual mission override slot (`vtable+0x1E8`) with the mission argument supplied by those call-sites, then sets destination/target and loads passengers. Handoff-critical conclusion: the active SW-spawned PDPLANE starts on mission `0x1A`.

### 3.2 Mission dispatch maps `0x1A` to Open and `0x1B` to Rescue

Active in YR: Yes. `TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch @ 0x005B3060`, which runs the object's current mission after the mission timer expires.

Cadence-relevant dispatch mapping from `0x005B3060`:

| Mission id | Slot | Handler | Active for standard SW PDPLANE? | Evidence |
|---:|---:|---|---|---|
| `0x1A` | `vtable+0x260` | `AircraftClass::Mission_Open @ 0x004158E0` | Yes | SW spawner passes `0x1A`; dispatch case `0x1A` calls slot `0x260` |
| `0x1B` | `vtable+0x264` | `AircraftClass::Mission_Rescue @ 0x00415960` | Yes, after Open transitions | `Mission_Open` calls `Override_Mission(0x1B, 0)` |
| `0x1E` | `vtable+0x26C` | `AircraftClass::Mission_ParaDropApproach @ 0x004155F0` | No for standard SW launch | Dispatch has the slot, but SW call-sites pass `0x1A`, not `0x1E` |
| `0x1F` | `vtable+0x270` | `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0` | No for standard SW launch | Only reached from `Mission_ParaDropApproach`; not the SW-spawned mission |

`MissionClass::Mission_Dispatch` writes `g_CurrentFrameCounter` to the mission timer start field and the handler return value to the active delay field after each handler call. Therefore `Mission_Rescue` returning `5` is a real 5-game-frame reschedule, not a comment-level guess.

### 3.3 `Mission_Open` prepares Rescue, but does not drop

Active in YR: Yes for standard SW PDPLANE because mission `0x1A` is the spawn mission.

`AircraftClass::Mission_Open @ 0x004158E0` has three cadence-relevant branches:

| Branch | Behavior | Active in YR |
|---|---|---|
| No target (`aircraft+0x2B4 == 0`) | Clears destination, overrides mission `4`, returns `3`. | Conditional edge case |
| Cargo/target gate empty (`aircraft+0x5A4 == 0`) | Sets destination to target and returns `3`; no drop. | Conditional edge case |
| In range (`FUN_005F6440(target) <= Rules+0x54C`) | Overrides mission to `0x1B`, decrements `LandingState`, returns `3`. | Yes, normal transition |

`Mission_Open` has only one resolved callee in the Ghidra call list: `FUN_005F6440`. It does not call `Drop_Payload`, `Fire_At`, or `GetROF`.

### 3.4 `Mission_Rescue` directly drops one passenger and returns `5`

Active in YR: Yes for standard SW PDPLANE after `Mission_Open` transitions to mission `0x1B`.

`AircraftClass::Mission_Rescue @ 0x00415960` sets `AircraftClass+0x6D2` to `1`, checks target/destination existence, computes distance with `FUN_005F6440`, and when distance is within `Rules+0x54C` it checks that the aircraft coordinates are in the playfield. If in playfield, it calls `AircraftClass::Drop_Payload @ 0x00415C60` and immediately returns `5`.

Assembly confirmation: at `0x004159FB`, `CALL 0x00415C60` is followed by `MOV EAX,0x5` and `RET`. This is the core cadence proof: one direct payload drop per Rescue handler execution, then a 5-frame mission delay.

The out-of-range path does not drop. It clears `+0x6D2`; if `LandingState > 0`, it overrides mission `0x1A` and returns `5`, otherwise it clears target/destination, overrides mission `4`, and returns `5`.

### 3.5 `Drop_Payload` sets LandingState but does not schedule cadence

Active in YR: Yes. `Drop_Payload` is reached from the active `Mission_Rescue` path.

`AircraftClass::Drop_Payload @ 0x00415C60` pops one passenger, decrements payload count, computes V-pattern placement, attempts cell entry/unlimbo, plays the drop sound on success, then writes:

| Write | Meaning | Active in YR |
|---|---|---|
| `*(byte *)(aircraft+0x6D3) = 5` | LandingState reset | Yes |
| `aircraft+0x2EC = g_CurrentFrameCounter` (`param_1[0xBB]`) | Last drop/fire frame stamp | Yes |
| `aircraft+0x2F0 = drop cell`, `aircraft+0x2F4 = 0` | Stores last drop cell/scratch | Yes |

The function does not read `[ParaDropWeapon]`, `WeaponType+0xB0`, or `TechnoClass::GetROF`. It returns `0` whether the drop succeeds, fails, or cargo is empty. Cadence comes from the caller's mission timer.

LandingState is therefore not the dominant in-range drop interval. It is set to `5` by successful `Drop_Payload`, decremented by `Mission_Open` when transitioning into Rescue, and consulted by the out-of-range Rescue recovery path. The in-range Rescue branch does not check LandingState before calling `Drop_Payload`.

### 3.6 `AircraftClass::Fire_At` has a cargo gate, but it is not the standard SW cadence path

Active in YR: Conditional. The function is active aircraft combat code, and its cargo gate is real when `Fire_At` is invoked on an aircraft with `+0x118 != 0`. It is not the standard paradrop superweapon drop scheduler because standard SW PDPLANE starts on mission `0x1A`, and `Mission_Open` / `Mission_Rescue` directly drive drops.

`AircraftClass::Fire_At @ 0x00415EE0` first checks `aircraft+0x118`. If non-zero, assembly at `0x00415EF8` calls `Drop_Payload`, zeroes `EAX`, and returns before normal weapon fire. If cargo is absent, it falls through to `TechnoClassFireAtSpawnsBullet @ 0x006FDD50` and the normal aircraft bullet/weapon path.

`Drop_Payload` xrefs are only:

| Caller | Meaning | Active in YR for SW cadence |
|---|---|---|
| `AircraftClass::Mission_Rescue @ 0x00415960` | Direct mission drop path | Yes |
| `AircraftClass::Fire_At @ 0x00415EE0` | Cargo hijack for normal fire invocation | No for standard SW cadence; conditional for other aircraft contexts |

This resolves the stale claim: `Fire_At`'s cargo gate exists, but it is not evidence that `[ParaDropWeapon] ROF=130` schedules standard superweapon drops.

### 3.7 `Mission_ParaDropApproach` / `Mission_ParaDropOverfly` are sibling aircraft missions, not this SW path

Active in YR: No for standard `Type=ParaDrop` / `Type=AmerParaDrop` launch cadence. Conditional for any separate code path that explicitly assigns mission `0x1E`.

`Mission_ParaDropApproach @ 0x004155F0` transitions to `0x1F` when distance is below `0x301`; `Mission_ParaDropOverfly @ 0x004157C0` returns `3`, performs reveal/exit behavior, and does not call `Drop_Payload`. The standard superweapon spawner does not assign `0x1E`, so these handlers should not be used to define standard SW drop cadence.

## 4. INI Keys

| INI key | Value in stock YR | Effect for this slice | Active in YR |
|---|---|---|---|
| `[PDPLANE] Primary=ParaDropWeapon` | `ParaDropWeapon` (`rulesmd.ini:11543`) | Real aircraft type data; can feed normal weapon selection if combat fire path is entered. | Conditional; not standard SW cadence |
| `[ParaDropWeapon] ROF=130` | `130` (`rulesmd.ini:23186`) | Real parsed weapon ROF; not read by `Mission_Open`, `Mission_Rescue`, or `Drop_Payload`. | No for standard SW drop cadence |
| `[General] ParadropRadius=1024` | `1024` (`rulesmd.ini:202`) | Range threshold for `Mission_Open` -> `Mission_Rescue` and Rescue direct-drop branch. | Yes |
| `[ParaDropSpecial] Type=ParaDrop` | `ParaDrop` (`rulesmd.ini:30961`) | Routes to `SuperClass::Launch` case 5, which passes mission `0x1A`. | Yes |
| `[AmericanParaDropSpecial] Type=AmerParaDrop` | `AmerParaDrop` (`rulesmd.ini:30976`) | Routes to `SuperClass::Launch` case 6, which passes mission `0x1A`. | Yes |

## 5. Integration Points

Active chain for standard YR superweapon paradrops:

1. `SuperClass::Launch @ 0x006CC390` case 5 or 6 validates the target and calls `FUN_0065E660`.
2. The call-sites at `0x006CD421`, `0x006CD493`, `0x006CD4EB`, and `0x006CD655` pass mission `0x1A`.
3. `FUN_0065E660 @ 0x0065E660` creates the PDPLANE, applies the mission argument via `vtable+0x1E8`, sets destination/target, and loads cargo.
4. `TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch @ 0x005B3060`.
5. Mission `0x1A` dispatches `Mission_Open @ 0x004158E0`.
6. When within `ParadropRadius`, Open queues mission `0x1B`.
7. Mission `0x1B` dispatches `Mission_Rescue @ 0x00415960`.
8. Rescue calls `Drop_Payload @ 0x00415C60` once, then returns `5`.
9. Mission dispatch stores that `5` as the next delay.

Timing interpretation: the first drop occurs on the first Rescue execution after Open enters range and returns its 3-frame delay. Subsequent in-range Rescue executions are spaced by 5 game frames. At the original 15 logical frames per second, that is one passenger every one-third second, before wall-clock game speed scaling.

## 6. Current Rust Implementation Status

| Surface | Current behavior | Status vs binary |
|---|---|---|
| `src/sim/aircraft/paradrop_mission.rs` | Defines `PARADROP_OPEN_TO_RESCUE_DELAY_TICKS=9`, modeling the 3-game-frame Open return delay under the current 3 local ticks per game frame convention. | Matches the verified first-drop delay shape for the local tick model. |
| `src/sim/aircraft/paradrop_mission.rs` | Models Rescue-equivalent `drop_cooldown` without using `LandingState` as an extra in-range throttle. | Updated after the verified fix; keep tests pinned to 5 gamemd-frame Rescue spacing, not ROF=130. |
| `src/sim/aircraft/mod.rs` | Applies `try_drop`, then sets Rescue-equivalent cooldown and bookkeeping. | Between-drop cadence should remain 5 gamemd frames after conversion. |
| `src/sim/superweapon/paradrop_tests.rs` | Test comments now describe Mission_Rescue cadence rather than `4 drops x 130-tick ROF`. | Stale audit item resolved; future tests should continue avoiding ROF-based expectations. |
| `src/rules/ruleset.rs` weapon parsing | Parses `ParaDropWeapon` and exposes `ROF=130`. | Correct parser behavior, but this value must not drive standard SW passenger cadence. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SuperClass::Launch` cases 5/6 mission arg | verified | decompile `0x006CC390`; assembly call-sites `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655` | none |
| `FUN_0065E660` mission application | verified | decompile `0x0065E660`; callers only `SuperClass::Launch` | exact stack arg names are decompiler-confused, but call-site push order proves mission value |
| `MissionClass::Mission_Dispatch` mapping | verified | decompile `0x005B3060`; caller `TechnoClass::AI_Update @ 0x006F9E50` | none |
| `Mission_Open @ 0x004158E0` | verified | decompile and callee list | none for cadence slice |
| `Mission_Rescue @ 0x00415960` | verified | decompile; assembly `0x004159FB` call and return `5` | none for cadence slice |
| `Drop_Payload @ 0x00415C60` | verified | decompile and callee list | V-pattern details owned by parent paradrop doc |
| `AircraftClass::Fire_At @ 0x00415EE0` cargo gate | verified | decompile; assembly `0x00415EF8` call to `Drop_Payload`; xref data/vtable | no runtime test of a non-SW cargo aircraft using Fire_At |
| `Mission_ParaDropApproach @ 0x004155F0` | verified-negative for SW cadence | decompile; dispatch slot `0x1E`; no SW call-site passes `0x1E` | may belong to non-SW or legacy aircraft contexts |
| `Mission_ParaDropOverfly @ 0x004157C0` | verified-negative for SW cadence | decompile; dispatch slot `0x1F`; no `Drop_Payload` callee | may belong to non-SW or legacy aircraft contexts |
| `[ParaDropWeapon] ROF=130` | verified-negative for SW cadence | `rulesmd.ini:23184-23186`; no read in Open/Rescue/Drop | no issue for parser retaining the value |
| Current Rust cadence implementation | touched-not-exhausted | `drop_payload.rs`, `paradrop_mission.rs`, `aircraft/mod.rs`, `paradrop_tests.rs` scan | no code changes in this report |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which mission id does standard SW PDPLANE receive? -> 0x1A.` (evidence: `SuperClass::Launch` call-sites `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`)
- `[RESOLVED] OQ-02 - What handler does mission 0x1A dispatch? -> `AircraftClass::Mission_Open @ 0x004158E0`.` (evidence: `MissionClass::Mission_Dispatch @ 0x005B3060`)
- `[RESOLVED] OQ-03 - What handler does mission 0x1B dispatch? -> `AircraftClass::Mission_Rescue @ 0x00415960`.` (evidence: `MissionClass::Mission_Dispatch @ 0x005B3060`)
- `[RESOLVED] OQ-04 - Does Mission_Open drop cargo? -> No, it transitions to Rescue and returns 3.` (evidence: decompile `0x004158E0`; callee list only `FUN_005F6440`)
- `[RESOLVED] OQ-05 - Does Mission_Rescue drop cargo? -> Yes, in range and in playfield it calls `Drop_Payload` and returns 5.` (evidence: decompile `0x00415960`; assembly `0x004159FB..0x00415A09`)
- `[RESOLVED] OQ-06 - Does Drop_Payload schedule its own cadence? -> No, it returns 0 and caller scheduling controls cadence.` (evidence: decompile `0x00415C60`)
- `[RESOLVED] OQ-07 - Does LandingState gate every in-range Rescue drop? -> No, the in-range Rescue branch calls Drop_Payload without checking LandingState.` (evidence: decompile `0x00415960`; write in `0x00415C60`)
- `[RESOLVED] OQ-08 - Is AircraftClass::Fire_At cargo gate real? -> Yes, if `+0x118 != 0`, it calls Drop_Payload and returns before normal fire.` (evidence: decompile `0x00415EE0`; assembly `0x00415EF8`)
- `[RESOLVED] OQ-09 - Is Fire_At the active standard SW cadence path? -> No, SW spawner uses mission 0x1A and Rescue directly calls Drop_Payload.` (evidence: `0x006CD421` etc.; `0x005B3060`; `0x004159FB`)
- `[RESOLVED] OQ-10 - Is `[ParaDropWeapon] ROF=130` parsed type data? -> Yes, stock INI defines it and Rust parses it.` (evidence: `rulesmd.ini:23184-23186`; `src/rules/ruleset.rs` scan)
- `[RESOLVED] OQ-11 - Does `[ParaDropWeapon] ROF=130` drive standard SW cargo cadence? -> No.` (evidence: Open/Rescue/Drop decompile; no `GetROF` or weapon read in those functions)
- `[RESOLVED] OQ-12 - Are Mission_ParaDropApproach / Overfly active for standard SW? -> No for this path; dispatch slots exist but SW call-sites do not assign their mission ids.` (evidence: `0x005B3060`; `0x006CD421` etc.)
- `[RESOLVED] OQ-13 - What is the first-drop scheduling implication? -> Open returns 3 after entering range and queues Rescue; Rescue then drops and returns 5.` (evidence: decompile `0x004158E0`, `0x00415960`)
- `[RESOLVED] OQ-14 - What happens if Rescue is in range but aircraft coords are outside playfield? -> It returns 5 without dropping.` (evidence: decompile `0x00415960`; `MapClass::IsCoordsInPlayfield` branch)
- `[RESOLVED] OQ-15 - What happens if target/destination is missing in Rescue? -> It clears strafe, target, destination, queues mission 4, returns 5.` (evidence: decompile `0x00415960`)
- `[RESOLVED] OQ-16 - Does Mission_ParaDropOverfly call Drop_Payload? -> No.` (evidence: decompile and callee list `0x004157C0`)
- `[RESOLVED] OQ-17 - Tick-cycle integration: who calls mission dispatch? -> `TechnoClass::AI_Update @ 0x006F9E50`.` (evidence: xref to `MissionClass::Mission_Dispatch @ 0x005B3060`)
- `[RESOLVED] OQ-18 - TS legacy filter: is this path gated by TS-only flags? -> No TS-only gate observed on SuperClass cases 5/6 or mission Open/Rescue cadence; activity is gated by charged/available SW and cargo/target state.` (evidence: decompile `0x006CC390`, `0x004158E0`, `0x00415960`)
- `[DEFERRED] OQ-19 - Does any non-superweapon aircraft context intentionally use Fire_At's cargo gate for drops?` (category: out-of-scope; reason: this report is limited to standard SW paradrop cadence; next-step-if-pursued: trace all writers of aircraft cargo + attack mission assignments)
- `[DEFERRED] OQ-20 - Runtime wall-clock observation of exact first visible drop frame from click?` (category: needs-runtime-debugger; reason: static scheduling proves handler delays but not UI/input frame alignment from click; next-step-if-pursued: debugger breakpoint on `0x004159FB` and log `g_CurrentFrameCounter` from launch)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard SW PDPLANE receives mission `0x1A`, which dispatches Open then Rescue. | `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`; `0x005B3060` | mismatch in naming/model: Rust uses `ParaDropApproach` / `ParaDropOverfly` for standard SW | `src/sim/superweapon/paradrop.rs`; `src/sim/aircraft/paradrop_mission.rs` | It is acceptable to keep cleaner Rust mission names, but behavior must match Open->Rescue timing: no ROF wait, first Rescue after Open transition, then 5 gamemd-frame intervals. | `paradrop_standard_sw_enters_rescue_cadence_not_overfly_rof` | Do not model standard SW as mission `0x1E/0x1F` if that changes first-drop timing or drop conditions. |
| Once Rescue is in range and in playfield, it calls `Drop_Payload` once and returns `5`. | decompile `0x00415960`; assembly `0x004159FB..0x00415A09` | mostly matches interval via `PARADROP_DROP_INTERVAL_TICKS=15` for 45 Hz sim | `src/sim/aircraft/drop_payload.rs`; `src/sim/aircraft/mod.rs` | Keep 5 gamemd-frame between-drop cadence after conversion to local sim ticks. | `paradrop_rescue_drops_every_five_gamemd_frames` | Do not use `[ParaDropWeapon] ROF=130` as passenger interval. |
| `Mission_Open` returns `3` when it enters Rescue; it does not drop in the same handler execution. | decompile `0x004158E0` | unchecked / possible first-drop drift because Rust transitions Approach->Overfly then may drop on the next local tick | `src/sim/aircraft/paradrop_mission.rs` | First actual passenger drop should be delayed by the equivalent of the Open return delay before Rescue executes. | `paradrop_first_drop_occurs_after_open_to_rescue_delay` | Do not collapse in-range transition and first drop into the same game frame. |
| LandingState is written `5` by Drop_Payload but is not checked by the in-range Rescue branch before dropping. | decompile `0x00415C60`, `0x00415960` | mismatch risk: Rust gates drops on `landing_state == 0` in `tick_overfly` | `src/sim/aircraft/paradrop_mission.rs:147-180`; `src/sim/aircraft/mod.rs:731-737` | If the local model keeps LandingState, ensure it does not extend the in-range interval beyond 5 gamemd frames. | `paradrop_landing_state_does_not_add_extra_in_range_delay` | Do not stack a 5-frame LandingState throttle on top of a 5-frame Rescue cooldown. |
| Fire_At cargo gate exists but is not standard SW cadence. | decompile `0x00415EE0`; xrefs to `Drop_Payload`; SW call-sites mission `0x1A` | current parser exposes `ParaDropWeapon` ROF and tests mention 130 | `src/rules/ruleset.rs`; `src/sim/superweapon/paradrop_tests.rs` | Leave ROF parsed, but tests/docs should state it is not used for standard SW passenger cadence. | `paradrop_weapon_rof_parsed_but_not_used_for_sw_drop_interval` | Do not delete `ParaDropWeapon` parsing; moddable normal fire paths may still need the weapon data. |
| `Mission_ParaDropOverfly` does not call Drop_Payload and is not assigned by SW launch. | decompile `0x004157C0`; dispatch `0x005B3060`; launch call-sites | mismatch risk if Rust Overfly is treated as binary-equivalent mission | `src/sim/aircraft/paradrop_mission.rs` | Use behavior tests rather than binary mission names as the parity contract. | `paradrop_overfly_name_does_not_imply_binary_mission_1f_cadence` | Do not cite binary `Mission_ParaDropOverfly` as evidence for standard SW drop timing. |

Stale Docs / Follow-up Docs:

- In `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, replace claims that standard SW drop cadence is driven by `[ParaDropWeapon] ROF=130` with: "Standard `Type=ParaDrop` / `Type=AmerParaDrop` spawns PDPLANE with mission `0x1A`; `Mission_Open` transitions to `Mission_Rescue`, and in-range `Mission_Rescue` calls `Drop_Payload` once and returns `5`. `[ParaDropWeapon] ROF=130` is parsed weapon data but does not schedule standard SW passenger drops."
- In the same report, replace claims that `Mission_ParaDropApproach` / `Mission_ParaDropOverfly` are the standard SW path with: "Those dispatch slots exist (`0x1E` / `0x1F`) but standard SW call-sites pass `0x1A`; they are not the evidence source for standard SW cadence."
- In Rust test comments, replace "`4 drops x 130-tick ROF`" with "drops use the 5 gamemd-frame Rescue cadence, converted to local sim ticks."

## 10. Negative Facts / Do Not Do

- Active in YR: No for standard SW cadence. Do not use `[ParaDropWeapon] ROF=130` as the interval between paratroopers.
- Active in YR: No for standard SW cadence. Do not cite `Mission_ParaDropOverfly @ 0x004157C0` as the drop scheduler; it does not call `Drop_Payload`.
- Active in YR: No for standard SW cadence. Do not enter standard SW aircraft directly into mission `0x1E` unless a separate compatibility layer preserves `0x1A` / `0x1B` timing.
- Active in YR: Conditional. Do not remove or ignore the `AircraftClass::Fire_At` cargo gate globally; it is real, just not the standard SW cadence path.
- Active in YR: Yes. Do not stack LandingState as an additional in-range throttle that changes the 5-frame Rescue cadence.

## 11. Remaining Uncertainty

- Runtime click-to-first-drop frame alignment is not measured. Static evidence proves mission delays after spawn and after Open/Rescue execution, but a debugger trace would be needed to pin the exact frame count from UI launch click to first `Drop_Payload`.
- Fire_At cargo-gate users outside the standard superweapon path were not traced. This report only says they do not control standard `Type=ParaDrop` / `Type=AmerParaDrop` cadence.
- Exact Ghidra stack parameter names in `FUN_0065E660` are decompiler-confused because of fastcall plus stack arguments. The call-site assembly is stronger evidence for the mission argument than the local pseudocode variable names.

## 12. Rust Test-Name Proposals

- `paradrop_rescue_drops_every_five_gamemd_frames`
- `paradrop_weapon_rof_130_does_not_delay_passenger_drops`
- `paradrop_first_drop_occurs_after_open_to_rescue_delay`
- `paradrop_landing_state_does_not_add_extra_in_range_delay`
- `paradrop_standard_sw_uses_mission_open_rescue_cadence`

## Sources

- Ghidra decompile: `SuperClass::Launch @ 0x006CC390`
- Ghidra assembly context: `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`
- Ghidra decompile: `FUN_0065E660 @ 0x0065E660`
- Ghidra decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`; xref caller `TechnoClass::AI_Update @ 0x006F9E50`
- Ghidra decompile: `AircraftClass::Mission_Open @ 0x004158E0`
- Ghidra decompile and assembly context: `AircraftClass::Mission_Rescue @ 0x00415960`, `CALL 0x00415C60` at `0x004159FB`
- Ghidra decompile: `AircraftClass::Drop_Payload @ 0x00415C60`
- Ghidra decompile and assembly context: `AircraftClass::Fire_At @ 0x00415EE0`, cargo-gate call at `0x00415EF8`
- Ghidra decompile: `AircraftClass::Mission_ParaDropApproach @ 0x004155F0`
- Ghidra decompile: `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0`
- INI: `ini/rulesmd.ini:202`, `11543`, `23184-23186`, `30961`, `30976`
- Rust scan: `src/sim/aircraft/drop_payload.rs`, `src/sim/aircraft/paradrop_mission.rs`, `src/sim/aircraft/mod.rs`, `src/sim/superweapon/paradrop_tests.rs`, `src/rules/ruleset.rs`
