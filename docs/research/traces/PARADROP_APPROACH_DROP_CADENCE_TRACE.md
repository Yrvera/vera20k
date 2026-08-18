# Paradrop Approach And Drop Cadence Trace

Scenario: `AmericanParaDropSpecial` spawns a standard SW PDPLANE, targets cell `(50,20)`, reaches `[General] ParadropRadius`, transitions from the Rust Open-equivalent `ParaDropApproach` state to the Rescue-equivalent `ParaDropOverfly` state, and performs first/subsequent `Drop_Payload` calls.

Scope: one mechanic only: approach threshold, first drop timing, repeat cadence, `ParaDropWeapon ROF=130`, and `LandingState` as standard SW throttle candidates.

## Pipeline

`AmericanParaDropSpecial` launch -> PDPLANE spawn with cargo -> Open-equivalent mission flies to `(50,20)` -> distance gate checks `ParadropRadius=1024` -> Rescue-equivalent mission calls `Drop_Payload` -> successful drop installs the next drop cooldown and mirrors `LandingState=5`.

## Stage Results

| Stage | Our output | gamemd output | Verdict |
|---|---|---|---|
| Active standard SW mission route | Rust starts the carrier in `AircraftMission::ParaDropApproach`, documented and implemented as Open-equivalent for standard SW. | `SuperClass::Launch` case 6 passes mission `0x1A`; dispatch maps `0x1A` to `AircraftClass::Mission_Open`. | PASS |
| Stock data for this scenario | `rulesmd` data parsed as `ParadropRadius=1024`; `ParaDropWeapon ROF=130` remains weapon data. | Stock YR has `[General] ParadropRadius=1024`; `[AmericanParaDropSpecial] Type=AmerParaDrop`; `[ParaDropWeapon] ROF=130`. | PASS |
| Distance threshold value | Rust compares `dist_leptons <= rules.general.paradrop_radius`, so threshold value is `1024`. | `Mission_Open` and `Mission_Rescue` compare `FUN_005F6440(target)` against `RulesClass+0x54C`, stock value `1024`. | PASS |
| Distance formula at the exact reach tick | Rust uses `max(abs(dx), abs(dy)) * 256` from cell positions. Exact aircraft position/subcell at first threshold crossing was not executed in this trace. | gamemd uses `FUN_005F6440(target)`, not the Rust Chebyshev approximation. Exact output at the reach tick was not runtime-captured. | UNCHECKED |
| Threshold sound/fog side effects | `tick_approach` sets `play_chute_sound=true` at `dist <= radius`; `tick_aircraft_missions` emits `SimSoundEvent::ChuteSound` at the target before any passenger drop. The fog flag is not consumed by a reveal call. | `Mission_Open @ 0x004158E0` only checks target/payload, distance, queues `0x1B`, decrements `+0x6D3`, and returns `3`; no `VocClass` or `MapClass::RevealAroundCell` call appears. Drop sound happens later inside `Drop_Payload` on success. | FAIL |
| Open-to-Rescue first drop delay | On the threshold tick, Rust changes mission to Rescue-equivalent with `drop_cooldown=0`; the next sim tick can call `Drop_Payload`. At fixed app tick `22ms`, this is one local tick after transition. | `Mission_Open` returns `3` after queuing `Mission_Rescue`; `MissionClass::Mission_Dispatch` stores that delay. First `Mission_Rescue` execution, and thus first drop, is after `3` gamemd frames. | FAIL |
| Successful drop call | Rust `try_drop` pops one cargo passenger and emits a drop sound at the drop cell on success. Exact sound id and cell for the runtime scenario were not executed here. | `Mission_Rescue` calls `AircraftClass::Drop_Payload`; `Drop_Payload` pops one passenger and calls `VocClass__PlayAt(0)` on successful unlimbo. | UNCHECKED |
| Subsequent drop cadence | After success, Rust sets `drop_cooldown=15` sim ticks. In the app, `SIM_TICK_MS=1000/45=22`, so 15 ticks are `330ms`; `binary_frame=floor(total_ms*15/1000)` can advance only 4 frames from a zero phase over 330ms. | `Mission_Rescue` returns `5`; mission dispatch stores an exact `5` game-frame delay. | FAIL |
| `ParaDropWeapon ROF=130` as throttle | No cadence path reads the weapon ROF in `tick_approach`, `tick_overfly`, or `try_drop`; the drop interval constant is independent of weapon data. | `Mission_Open`, `Mission_Rescue`, and `Drop_Payload` do not read `[ParaDropWeapon]`, `WeaponType+0xB0`, or `TechnoClass::GetROF`. | PASS |
| `LandingState` as in-range throttle | `tick_overfly` decrements `landing_state` but `can_drop` depends only on `drop_cooldown == 0`; successful drop writes `landing_state=5`. | `Drop_Payload` writes `+0x6D3=5`, but the in-range `Mission_Rescue` branch calls `Drop_Payload` without checking `LandingState`; `LandingState` only affects out-of-range recovery. | PASS |

## Failures

1. Threshold sound fires too early and likely duplicates the real drop sound.
   - Player-visible difference: the chute sound can play when the plane merely enters radius, before the first paratrooper appears.
   - Our surface: `src/sim/aircraft/paradrop_mission.rs:76`, `src/sim/aircraft/mod.rs:707`.
   - gamemd evidence: `Mission_Open @ 0x004158E0` decompile has no sound/reveal call; `Drop_Payload @ 0x00415C60` contains the successful-drop sound path.

2. First passenger drop is too early.
   - Player-visible difference: after the plane reaches the drop radius, the first infantry can exit after one 22ms sim tick instead of after the original 3 game-frame mission delay.
   - Our surface: `src/sim/aircraft/paradrop_mission.rs:80`, `src/sim/aircraft/paradrop_mission.rs:186`, `src/sim/aircraft/mod.rs:719`.
   - gamemd evidence: `Mission_Open @ 0x004158E0` queues `0x1B` and returns `3`; `MissionClass::Mission_Dispatch @ 0x005B3060` stores handler return values as mission delays.

3. Repeat cadence is represented in local sim ticks rather than exact gamemd frame scheduling.
   - Player-visible difference: with the current fixed app tick of `22ms`, 15 local ticks are `330ms`, slightly short of 5 original frames (`333.333ms`) and phase-dependent against `binary_frame`.
   - Our surface: `src/sim/aircraft/drop_payload.rs:35`, `src/app_types.rs:27`, `src/sim/world/mod.rs:1044`.
   - gamemd evidence: `Mission_Rescue @ 0x00415960` calls `Drop_Payload` and returns `5`; dispatch stores that exact delay.

## Not Implemented

No standard-YR threshold fog reveal was confirmed for this scenario. The current Rust `fire_fog_reveal` flag is not consumed, but the verified standard Open/Rescue path also did not show a threshold reveal call. Treat any separate reveal behavior as a new trace target unless runtime evidence proves it belongs to standard SW paradrops.

## Adjacent Findings

- Rust uses a Chebyshev cell-distance approximation for the threshold gate. For a perfectly horizontal approach to `(50,20)` this may match at common boundary cells, but the exact reach tick and any curved approach/subcell case remain unchecked.
- The binary sibling missions `Mission_ParaDropApproach`/`Mission_ParaDropOverfly` are live handlers but not the standard `AmericanParaDropSpecial` launch route; they should not be used as cadence evidence for this trace.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources

- Ghidra read-only decompile: `SuperClass::Launch @ 0x006CC390`; active case 6 calls the paradrop spawner with mission `0x1A`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`; handler return values are stored as mission delays.
- Ghidra read-only decompile: `AircraftClass::Mission_Open @ 0x004158E0`; in-range branch queues `0x1B`, decrements `+0x6D3`, returns `3`.
- Ghidra read-only decompile: `AircraftClass::Mission_Rescue @ 0x00415960`; in-range branch calls `Drop_Payload` and returns `5`; no `LandingState` gate before the call.
- Ghidra read-only decompile: `AircraftClass::Drop_Payload @ 0x00415C60`; successful drop writes `+0x6D3=5`, frame stamp, last drop cell, and plays sound.
- Ghidra read-only decompile: `AircraftClass::Fire_At @ 0x00415EE0`; cargo gate exists but is not the standard SW mission path.
- Research docs: `docs/research/PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`, `docs/research/PARADROP_MISSION_TRANSITIONS_GHIDRA_REPORT.md`.
- Rust surfaces read-only: `src/sim/superweapon/paradrop.rs`, `src/sim/aircraft/paradrop_mission.rs`, `src/sim/aircraft/mod.rs`, `src/sim/aircraft/drop_payload.rs`, `src/sim/world/mod.rs`, `src/app_types.rs`.
