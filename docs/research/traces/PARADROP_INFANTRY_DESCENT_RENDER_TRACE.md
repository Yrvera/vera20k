# Paradrop Infantry Descent And Render Trace

**Scenario:** One GI dropped from a PDPLANE at flight altitude descends with parachute state to landing.

**Scope:** Fall-rate ramp/order, landing timing, PARACH visual attachment/lifetime, body sequence, locomotor identity, and final player-visible render. Adjacent cargo, V-pattern placement, carrier mission cadence, bridge landing, death-in-air, and no-chute destruction paths are out of scope.

**Date:** 2026-05-22

**Status:** COMPLETE for the requested trace. Stages without literal Rust-vs-gamemd numerical equality are marked UNCHECKED.

## Evidence Used

- Current Rust:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/aircraft/drop_payload.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/parachute_descent.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_chute_anim.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_instances/overlays.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_instances/shp.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/rules/art_data.rs`
- Data:
  - `ini/rulesmd.ini`: `FlightLevel=1500`, `ParachuteMaxFallRate=-3`, `NoParachuteMaxFallRate=-100`, `Parachute=PARACH`, `BombParachute=PARABOMB`, `ChuteSound=ParachuteDrop`
  - `ini/artmd.ini`: `[PARACH] Rate=400`, `LoopStart=20`, `LoopEnd=39`, `LoopCount=30`, `AltPalette=yes`, `ZAdjust=-10`
- Verified research:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`
- Read-only Ghidra spot checks:
  - `AircraftClass::Drop_Payload @ 0x00415C60`
  - `ObjectClass::Unlimbo @ 0x005F5940`
  - `ObjectClass::AI @ 0x005F3E70`
  - `FootClass::Locomotion_AI @ 0x00520F40`

All gamemd paths cited above are active in standard YR for stock paradrop payloads. The normal infantry path uses `PARACH`, not `PARABOMB`; the body `Paradrop` sequence path is active only for Jumpjet-gated infantry logic, not ordinary paradropped GI.

## Pipeline

`AircraftClass::Drop_Payload` / Rust `try_drop` -> passenger leaves cargo -> target cell/subcell assigned -> object-level falling/parachute state begins -> per-tick falling AI integrates Z then updates rate -> attached PARACH anim renders while falling -> landing clears falling state and kills/removes chute -> GI remains as ordinary infantry.

## Stage Verdicts

| Stage | Rust output for this scenario | gamemd output | Verdict |
|---|---|---|---|
| Rules data | `FlightLevel=1500`, `ParachuteMaxFallRate=-3`, `Parachute=PARACH`; `[PARACH] Rate=400`, loop 20..39, `ZAdjust=-10`; Rust converts Rate 400 to 133 ms | Same stock YR INI keys parsed by Rules/AnimType readers; `Rate=400` corresponds to 2 logic frames at 15 Hz, nominal 133 ms | PASS |
| Drop trigger to descent state | Rust pops one passenger, places it, sets `passenger_role=None`, then calls `begin_parachute_descent(passenger, aircraft_locomotor_altitude)` | gamemd pops passenger, finds subcell, calls passenger vtable Unlimbo, sets falling flag, and creates attached PARACH | UNCHECKED: exact initial airborne Z source in gamemd remains unresolved |
| Body sequence | Rust preserves existing `SequenceKind::Stand` or any externally changed body sequence | gamemd normal paradrop path does not call body sequence `Paradrop`; Jumpjet-gated path is separate | PASS |
| Locomotor identity | Rust keeps Walk/Ground locomotor and no override | gamemd falling is object-level state; no Jumpjet/Parachute locomotor construction or piggyback in the normal paradrop chain | PASS |
| Fall-rate order | Rust integrates current rate first, then decrements/clamps: rate-in sequence `0,-1,-2,-3,-3...`; first four cumulative descent deltas `0,1,3,6` leptons | `ObjectClass::AI` reads current fall delta, writes Z, checks landing, then decrements and clamps via `Rules+0x7B8`; same `0,-1,-2,-3` order when initial delta is 0 | PASS |
| Landing threshold | Rust lands on `altitude <= 0`, clamps visual altitude to 0, clears `parachute_state` | gamemd checks effective height `< 1`, sets height to 0, clears falling flag, changes mission to 2, kills attached anim lifetime | PASS |
| Landing duration from flight altitude | If initial altitude is exactly 1500 leptons, Rust lands on tick 502 after attach: cumulative descent is `3N - 6`, first `N` with `>=1500` is 502 | Per-tick gamemd formula gives the same 502 ticks if initial effective height is exactly 1500 | UNCHECKED: gamemd initial airborne Z assignment through base Unlimbo was not fully resolved |
| PARACH lifetime | Rust render-layer chute exists while `entity.parachute_state.is_some()` and is removed after landing or entity disappearance | gamemd creates an owned `PARACH` AnimClass during Unlimbo and landing writes `Anim+0x195=0`; cleanup later detaches | UNCHECKED: same visible lifetime is likely, but exact same-frame create/destroy ordering was not pixel/tick compared |
| PARACH frame timing | Rust starts frame 0, advances every 133 ms, plays 0..39 once then loops 20..39 | gamemd uses `[PARACH] Rate=400`, `LoopStart=20`, `LoopEnd=39`, `LoopCount=30`, but landing kills before LoopCount expiration | UNCHECKED: nominal frame rate matches, but app render delta vs gamemd logic-frame scheduling was not numerically compared under this scenario |
| PARACH screen placement/depth | Rust anchors at GI screen position, adds atlas offset, subtracts hardcoded `CHUTE_Y_LIFT=8.0`, draws depth `GI depth - 0.0005`; parsed `ZAdjust=-10` is not applied in this builder | gamemd attaches an AnimClass owner-relative to the object and uses AnimClass display/layer logic plus art `ZAdjust=-10`; no verified 8 px lift constant exists | FAIL |
| Final screen result | Rust should show normal GI body falling with a PARACH canopy and no body Paradrop frames | gamemd shows ordinary infantry body with attached PARACH canopy descending at object fall rate | UNCHECKED: no frame capture/pixel comparison was run, so final sprite offset/frame/depth equality is unproven |

## Player-Visible Findings

1. **FAIL - PARACH placement/depth is not proven and contains a known hardcoded visual fudge.**
   - Rust: `build_parachute_instances` uses `CHUTE_Y_LIFT=8.0` and `CHUTE_DEPTH_EPSILON=0.0005`.
   - gamemd: attached `PARACH` is owner-relative AnimClass display with `[PARACH] ZAdjust=-10`.
   - Player-visible difference: canopy can sit too high/low or sort slightly wrong over the GI.
   - Rust location: `src/app_instances/overlays.rs:667`, `src/app_instances/overlays.rs:675`, `src/app_instances/overlays.rs:681`, `src/app_instances/overlays.rs:745`.
   - gamemd evidence: `ObjectClass::Unlimbo @ 0x005F5940`, `AnimClass::SetOwnerObject @ 0x00424B50` per report, `[PARACH] ZAdjust=-10`.

## Improvements Confirmed By This Trace

- Normal paradropped GI no longer switches to a fake body `Paradrop` sequence.
- Normal paradropped GI no longer switches locomotor identity to a Parachute/Jumpjet-like override.
- Descent rate now follows the verified object-level ramp/order: integrate current delta first, then update `0,-1,-2,-3`.
- Landing cleanup now preserves the infantry body sequence and locomotor identity.

## Adjacent Findings

- `src/sim/aircraft/drop_payload.rs` still has a stale comment saying `begin_parachute_descent` uses `OverrideKind`; the current code no longer does. This is not player-visible.
- The normal no-chute branch using `NoParachuteMaxFallRate=-100` is outside this successful PARACH descent scenario.
- Exact initial airborne Z assignment through gamemd base Unlimbo remains the main blocker to proving total landing duration from PDPLANE flight altitude.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 5 | NOT-IMPLEMENTED: 0
