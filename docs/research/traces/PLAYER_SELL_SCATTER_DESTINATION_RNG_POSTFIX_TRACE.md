# Player-Sell Scatter Destination / RNG Postfix Trace

**Scenario:** Player sells an occupied `CanBeOccupied` garrison building; one infantry occupant is successfully placed on a valid edge cell, then receives the direct scatter handoff.

**Concrete Rust fixture for numeric values:** `CAGAS01`-style `2x2` garrison at `(10,10)`, one hidden infantry occupant, first accepted edge cell `(12,12)`, building center approximated by Rust as `(11,11)`, no adjacent scatter blockers, default `Simulation::new()` RNG seed.

**Scope:** only the post-placement player-sell scatter handoff: `RandomRanged(0,4)` jitter, 8-direction candidate ordering, destination write, and immediate facing-visible state.

## Verdict

PASS: 4 | FAIL: 2 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Active YR Confirmation

This path is active in standard Yuri's Revenge. Existing verified reports and a read-only Ghidra recheck confirm `BuildingClass::SellBuilding @ 0x00457DE0` calls occupant vtable `+0x174` after successful `Unlimbo`, and infantry resolves that virtual to `InfantryClass::Scatter @ 0x0051D0D0`. `rulesmd.ini` has active civilian `CanBeOccupied=yes` buildings, including `CAGAS01`.

No TS-only path is used as evidence here.

## Pipeline

`player sell` -> `SellBuilding successful Unlimbo` -> `clear archive target +0x3C8(0)` -> `direct Scatter(building center,1,1)` -> `InfantryClass::Scatter gates` -> `RandomRanged(0,4)-2` -> `g_DirectionOffsets candidate scan` -> `Queue_Mission(2)` -> `Set_Destination(...,1)` -> later `SellBuilding` mission `0xF`

Rust pipeline:

`place_garrison_passenger_at_cell` -> `sellbuilding_direct_scatter_handoff` -> local infantry/alive/locomotor gates -> `next_range_u32_inclusive(0,4)-2` -> `SCATTER_DIRECTION_OFFSETS` scan -> `movement::issue_direct_move` -> `MovementTarget` + immediate infantry `facing`

## Stage Results

| Stage | gamemd.exe output | Current Rust output | Verdict |
|---|---|---|---|
| 1. Direct scatter call timing | After successful `Unlimbo`, native calls `+0x3C8(0)`, gets building coords, then calls occupant `+0x174` before the later `+0x1E8(0xF,0)`. Evidence: `0x004580E9..0x00458138`. | Rust places the passenger, inserts occupancy, then calls `sellbuilding_direct_scatter_handoff` at `src/sim/production/production_sell.rs:458`. No native virtual-call surface exists. | PASS for post-placement timing; NOT-IMPLEMENTED for exact virtual surface |
| 2. RNG primitive and bounds | Directional infantry scatter calls scenario `Random::RandomRanged(0,4)` and uses `roll - 2`. Evidence: `0x0051D2AC..0x0051D2D0`; `RandomRanged @ 0x0065C7E0`. | Rust calls `sim.rng.next_range_u32_inclusive(0,4) as i32 - 2` at `src/sim/production/production_sell.rs:394`. | PASS |
| 3. Concrete Rust RNG draw | Not live-captured for an identical YR scenario seed/state in this run. | Default Rust seed's first `next_range_u32_inclusive(0,4)` accepts raw sample `0x0B836DE3`, returns `3`, jitter `+1`. | UNCHECKED |
| 4. Pre-RNG scatter gates | Native gates include sequence/table checks, locomotor busy downgrade, mission timer/type/player scatter gates, and can return before RNG. Evidence: `InfantryClass::Scatter @ 0x0051D0D0..0x0051D220`. | Rust only checks infantry category, alive, not dying, not inside transport, and locomotor present at `src/sim/production/production_sell.rs:372..377`. | FAIL |
| 5. Direction-offset table order | Native `g_DirectionOffsets @ 0x0089F688` maps `0..7` to `N,NE,E,SE,S,SW,W,NW`. Existing trace `DIRECTION_ID_TABLE_COMPASS_OFFSETS_TRACE.md` computed exact pairs. | Rust `SCATTER_DIRECTION_OFFSETS` at `src/sim/production/production_sell.rs:26..35` is the same eight signed pairs. | PASS |
| 6. Start-direction computation | Native uses `atan2`/`ftol` quantization, then `(((angle >> 12)+1)>>1)&7`, plus jitter. For the intended `(12,12)->(11,11)` shape this should be NW plus jitter, but this run did not compute live native lepton inputs. | Rust uses cell delta `(11-12,11-12)=(-1,-1)`, `facing_from_delta/32 = 7`, roll `3`, start dir `(7+1)&7=0`. | UNCHECKED |
| 7. Candidate ordering | Native scans `start_dir+i & 7` through `g_DirectionOffsets`; if start dir is `0`, candidates are `(12,11),(13,11),(13,12),(13,13),(12,13),(11,13),(11,12),(11,11)`. | Rust loop at `src/sim/production/production_sell.rs:397..408` scans the same table order for its computed `start_dir`; with Rust start dir `0`, first candidate is `(12,11)`. | PASS for table scan; UNCHECKED for native start dir in live fixture |
| 8. Destination acceptance predicate | Native candidate must pass in-playfield, `InfantryClass::Can_Enter_Cell`, effective-height/snap check, and blocked-cell flags before destination success. Evidence: `0x0051D487..0x0051D694`. | Rust uses `garrison_infantry_can_enter_cell(..., false)`, a `check_terrain` phase-1 stand-in, at `src/sim/production/production_sell.rs:405` and `300..328`. | FAIL |
| 9. Destination / mission state | On success, native queues mission `2`, then calls destination virtual `+0x480(...,1)`; later `SellBuilding` queues mission `0xF`. Evidence: `0x0051D6BE..0x0051D6E0`, `0x00458132..0x00458138`. | Rust calls `movement::issue_direct_move` at `src/sim/production/production_sell.rs:412`, which writes a `MovementTarget` and does not queue native mission `2` or `0xF`. | FAIL |
| 10. Immediate facing-visible state | Native immediate facing effect after `Set_Destination(...,1)` was not computed in this run. | `issue_direct_move` sets infantry `facing` immediately at `src/sim/movement/movement_commands.rs:242..249`; with Rust candidate `(12,11)`, facing becomes `0` (N). | UNCHECKED |

## Concrete Rust Output

For the scoped Rust fixture:

- Edge placement: `(12,12)`.
- Building-center target used by Rust scatter: `(11,11)`.
- Base direction: `7` (`NW`).
- First accepted Rust RNG draw: return `3`, jitter `+1`.
- Rust start direction: `0` (`N`).
- Rust first candidate: `(12,11)`.
- Rust destination surface: `MovementTarget { path: [(12,12),(12,11)], next_index: 1 }`.
- Rust immediate visible facing byte: `0`.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

1. **Stage 9 - destination / mission state FAIL:** ejected infantry receives Rust `MovementTarget` instead of native mission `2` + `Set_Destination` + later mission `0xF`; Rust `src/sim/production/production_sell.rs:412`, gamemd `0x0051D6BE..0x0051D6E0` and `0x00458132..0x00458138`.
2. **Stage 8 - destination acceptance predicate FAIL:** Rust may choose a different scatter destination because it uses `check_terrain` instead of native `InfantryClass::Can_Enter_Cell` plus height/snap checks; Rust `src/sim/production/production_sell.rs:300..328`, gamemd `0x0051D487..0x0051D694`.
3. **Stage 4 - pre-RNG scatter gates FAIL:** Rust may consume or skip scatter RNG differently because it lacks native sequence/table/mission/player scatter gates; Rust `src/sim/production/production_sell.rs:372..377`, gamemd `0x0051D0D0..0x0051D220`.
4. **Stage 1 - exact virtual surface NOT-IMPLEMENTED:** Rust has no explicit `+0x3C8(0)` / occupant `+0x174(building coord,1,1)` call/state surface; Rust `src/sim/production/production_sell.rs:458`, gamemd `0x004580E9..0x0045810A`.

## Adjacent Findings

- Full Infantry `Can_Enter_Cell @ 0x0051BF90` remains the largest remaining parity dependency for this player-sell scatter destination.
- Exact native immediate facing after `Set_Destination(...,1)` needs a narrower locomotor/destination trace; this run did not mark it PASS.
- Exact mission queue field writes behind vtable `+0x1E8` are adjacent and not decoded here.

## Sources

- Read-only Ghidra recheck: `InfantryClass::Scatter @ 0x0051D0D0`.
- Existing verified reports: `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`, `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`, `DIRECTION_ID_TABLE_COMPASS_OFFSETS_TRACE.md`, `GARRISON_PLAYER_SELL_CANENTER_SCATTER_POSTFIX_TRACE.md`.
- Rust read-only scan: `src/sim/production/production_sell.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/rng.rs`.

## Status

COMPLETE
