# Dock 0x16 Do_Turn / RateTimer / Unload Gate Recheck

**Date:** 2026-05-26  
**Investigation Mode:** coverage-map  
**Scope:** reconcile existing Ghidra-backed reports and current Rust around stock YR harvester/refinery dock radio `0x16`, `DriveLocomotionClass::Do_Turn(0x4000)`, the mission `0x10` unload gate, and Rust's forced East dock-facing behavior.  
**Non-Scope:** live runtime watch of HARV/CMIN body-facing on every refinery approach, complete cargo-credit timing, full refinery animation/audio timing, and Rust implementation.  
**Live Ghidra status:** unavailable in this session. No running Ghidra instance was found, so this report is based on existing Ghidra-backed research docs plus current Rust source scan.  
**Confidence:** High for the static control-flow facts recorded in the cited Ghidra-backed docs: radio `0x16` calls active locomotor vtable `+0x4C(0x4000)`, concrete Drive locomotor `+0x4C` resolves to `DriveLocomotionClass::Do_Turn`, `Do_Turn` sets the RateTimer value, mission `0x10` gates unload start on the RateTimer window, not-ready mission `0x10` returns delay `5`, and unload start does not explicitly write unit body facing. Medium for exact body-facing value held during every runtime dock approach because that still needs a runtime watch.

## 1. Executive Finding

The base compass substrate is not the problem. Rust's normal direction/facing convention matches the verified YR convention: facing byte `0x40` is East, direction index `2` is East, and East cell delta is `(1,0)`.

The parity gap is at the refinery dock consumer. Current Rust treats dock sync as a miner-FSM-owned smooth pivot to East and then snaps `entity.facing` to East when unload starts. Existing Ghidra-backed reports say gamemd does not do that. In active YR:

- radio `0x16` does not write the unit body-facing byte;
- radio `0x16` does not start unload;
- radio `0x16` does not call `GetDockCoord`, set a destination, or write position;
- first unsynchronized radio `0x16` calls the active locomotor vtable `+0x4C(0x4000)` and returns;
- for Drive locomotion, vtable `+0x4C` is `DriveLocomotionClass::Do_Turn`, whose decompiled body sets a RateTimer value;
- mission `0x10` later samples the unit RateTimer and accepts only the specific East-window expression;
- if mission `0x10` is not ready, it calls locomotor `+0x4C(0x4000)` and returns hard delay `5`;
- when unload start is accepted, no explicit body-facing write is observed in the verified init block.

So the current Rust behavior "force a smooth dock pivot to East and snap exactly to East at unload start" is a concrete mechanism drift. It is not just an ownership nit.

## 2. Verified Facts

| Fact | Evidence | Active in YR | Confidence |
|---|---|---:|---:|
| Normal East convention is `0x40` in 8-bit facing, `0x4000` in 16-bit facing-like space, direction index `2`, and cell delta `(1,0)`. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`, direction table init evidence, Rust scan of movement/fixed-math helpers. | Yes | High |
| Radio `0x16` case calls base radio handling first, then checks the chrono/teleporting flag before the ordinary sync path. | `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` section 3.1. | Yes | High |
| If the ordinary sync path is active and the sampled RateTimer value is not `0x4000`, radio `0x16` calls active locomotor vtable `+0x4C(0x4000)` and returns `1`. | `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` section 3.1; `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` section 3.8. | Yes | High |
| Concrete Drive locomotor vtable `+0x4C` maps to `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` section 3.8; `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`. | Yes for HARV and CMIN dock drive phase | High |
| `DriveLocomotionClass::Do_Turn` decompiles to a RateTimer set operation for the argument, so `0x4000` is a RateTimer sync value in this path, not a direct UnitClass body-facing write. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` section 3.8. | Yes | High |
| Later/already-synced radio `0x16` can send radio `0x15` only under stopped/destination/building/mission/contact gates. | `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` section 3.1; `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md` cited by the earlier recheck. | Yes | High |
| Mission `0x10` dispatch reaches `UnitClass::Mission_Deploy_Building @ 0x0073D630` for stock HARV/CMIN unload. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` sections 3.1 and 3.2. | Yes | High |
| Mission `0x10` checks `PathType::Has_Valid_Steps` before the facing/unload-start gate. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.3. | Yes | High |
| The mission `0x10` facing accept condition is `((RateTimerCurrent >> 7) + 1) & 0x1FE == 0x80`. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.4. | Yes | High |
| If that accept condition fails and the chrono/teleporting flag is clear, mission `0x10` calls locomotor `+0x4C(0x4000)` and returns delay `5` without setting unload-active fields. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.4. | Yes | High |
| Accepted unload start writes the unload-active/timer/substate fields after the path and facing gates, and only then enters active dump substate. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.5. | Yes | High |
| No explicit unit body-facing write was observed in the verified unload-start init block. The branch requires the live RateTimer to already be in the accepted window. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` OQ-07. | Yes | High for absence in that block |
| Exact body-facing byte on the dump frame remains path/runtime dependent. Static decompile proves no forced East snap in these handlers, but a runtime watch is still needed for every approach case. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` OQ-10. | Conditional by approach path | Medium |

## 3. Current Rust Delta

| Rust surface | Current behavior | Parity status |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs` constants `DOCK_FACING_EAST` and `DOCK_FACING_EAST_DIR` | Treat dock sync as explicit East body-facing target. | Drift as a claimed model of radio `0x16`; `0x4000` is passed to Drive `Do_Turn`/RateTimer in gamemd. |
| `sync_dock_facing` | Owns a Rust `FacingClass`, advances it inside the miner dock FSM, writes `entity.facing`, and sets `entity.facing_target`. | Drift/risk. gamemd evidence places the request through the active locomotor and RateTimer ownership, not a miner-FSM body-facing assignment. |
| `phase_face_sync` | Runs a first sync-like step after the `0x18/0x16` admission handoff. | Directionally close for the first-vs-later event split, but it still uses the Rust miner-owned pivot model. |
| `phase_pivoting` | Polls the facing gate every sim tick until accepted, then starts unload. | Drift unless another scheduler proves a hard 5-frame retry cadence. gamemd mission `0x10` returns delay `5` when not ready. |
| `start_unload_deploy` | Calls pad/link bookkeeping, sets display override, snaps `entity.facing` to East, clears target, emits `DockDeploy`, seeds local unload timer, enters `Unloading`. | Concrete drift. The verified gamemd unload-start block does not explicitly snap body facing and instead writes the unload-active/timer/substate cluster after the RateTimer gate accepts. |
| Miner tests around Pivoting/Unloading | Assert exact East snap and East-facing target behavior. | These tests encode current Rust behavior, not verified gamemd mechanism. They should be rewritten when the dock sync model is patched. |

## 4. Interpretation

The important distinction is domain ownership:

- `0x40` is East in 8-bit body-facing terms.
- `0x4000` is numerically East in 16-bit facing-like terms.
- In the radio `0x16` and mission `0x10` dock paths, the binary passes `0x4000` to active Drive locomotion `Do_Turn`, whose concrete implementation sets a RateTimer value.
- The unload gate samples that RateTimer through a window expression; it does not require Rust-style direct equality of `entity.facing == 0x40`.

That means Rust may still end up displaying an East-ish frame during some dock sequences because the RateTimer target is the East window. But the Rust implementation is not parity-correct if it owns the turn from the miner FSM, polls it every tick, or snaps `entity.facing` exactly to `0x40` at unload start.

The static evidence is enough to say the forced snap is wrong. It is not enough to claim the miner never visually faces East during dump in every stock runtime path. That latter question requires a runtime watch of final drive path, RateTimer current, body facing, mission, and radio sequence.

## 5. Open Questions Log

- `[RESOLVED] OQ-01 - Is Rust's base East convention inverted relative to gamemd? -> No. East is `0x40`/direction index `2`/delta `(1,0)` in the verified substrate and current Rust helpers.`
- `[RESOLVED] OQ-02 - Is dock radio `0x16` a direct body-facing setter? -> No. It calls active locomotor `+0x4C(0x4000)` under the ordinary sync branch and returns; no direct UnitClass body-facing write is recorded.`
- `[RESOLVED] OQ-03 - Does Drive locomotor `+0x4C` support treating `0x4000` as a direct miner-FSM facing assignment? -> No. Existing docs identify it as `DriveLocomotionClass::Do_Turn`, with decompile reducing to RateTimer set behavior.`
- `[RESOLVED] OQ-04 - Does mission `0x10` force facing East when unload starts? -> No explicit body-facing write was observed in the verified unload-start init block; it requires the live RateTimer window to already accept.`
- `[RESOLVED] OQ-05 - Does not-ready mission `0x10` poll every frame? -> No. The verified path calls locomotor `+0x4C(0x4000)` when allowed and returns delay `5`.`
- `[RESOLVED] OQ-06 - Is current Rust's East snap a parity gap? -> Yes. It writes a field in Rust at unload start where the verified gamemd unload-start block does not write body facing.`
- `[DEFERRED] OQ-07 - Exact body-facing byte on the first dump frame for HARV and CMIN on each refinery/map approach. Category: needs-runtime-debugger. Reason: final display value is path/runtime dependent even though the static handlers do not force a snap. Next step: runtime watch one stock HARV and one stock CMIN dock cycle, capturing active locomotor, RateTimer current, body facing byte, mission, radio command, and destination/contact fields.`
- `[DEFERRED] OQ-08 - Exact RateTimer field layout and all non-miner consumers of the same timer. Category: live-Ghidra-unavailable. Reason: enough evidence exists for the dock handoff, but a full field-level model should decompile `RateTimer::Set`, `RateTimer::Current`, and adjacent consumers when Ghidra is available.`

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First unsynchronized radio `0x16` calls active locomotor `+0x4C(0x4000)` and returns before the later `0x15` cascade. | `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` section 3.1. | Rust approximates with miner-owned `sync_dock_facing`. | `phase_face_sync`, `sync_dock_facing`, future active-locomotor turn/timer API. | Represent radio `0x16` as an active locomotor RateTimer/turn request. Keep first `0x16` separate from later `0x16 -> 0x15`. | First sync event does not start unload, does not queue mission `0x10`, and does not write body facing directly. | Do not collapse `0x16` return `1` into "dock now". |
| Drive locomotor `Do_Turn(0x4000)` sets the timing/RateTimer target used by the dock gate. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` section 3.8. | Rust stores a miner-local pivot and writes `entity.facing`. | Movement/locomotor facing timer state; miner dock sequence. | Move the dock sync state into the active Drive locomotor or a shared locomotor-owned RateTimer model. | HARV and CMIN in active Drive dock phase share the same dock sync path. | Do not special-case CMIN; CMIN docks through active Drive piggyback after the locomotor ownership refactor. |
| Mission `0x10` checks path validity before unload start. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.3. | Rust's `phase_pivoting` goes straight to facing sync/start unload. | `phase_pivoting`, future mission deploy helper. | Add the path/contact-valid gate before unload-active side effects. | If the path gate fails, unload-active display/state and cargo drain do not start. | Do not start unload solely because the RateTimer window accepts. |
| Not-ready mission `0x10` calls locomotor `+0x4C(0x4000)` and returns delay `5`. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.4. | Rust polls every sim tick while `Pivoting`. | Miner dock phase scheduling / mission return delay. | Model the 5-frame retry delay or prove an equivalent scheduler boundary. | A slow-turning miner cannot pass the deploy gate during the 5-frame delay after a failed mission `0x10` pass. | Do not evaluate the deploy-facing gate every frame. |
| Accepted unload start writes unload-active/timer/substate fields and does not explicitly snap body facing. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` sections 3.5 and OQ-07. | `start_unload_deploy` snaps `entity.facing` to East and uses local Rust unload timer/display state. | `start_unload_deploy`, miner unload state, render-facing unit state. | Remove the unload-start body-facing snap; preserve the live locomotor/timer-derived facing. Model the unload-active latch/timer/substate ordering separately. | Captured facing immediately before and after accepted unload start does not jump solely because unload started. | Do not hide dock drift with a final exact `0x40` assignment. |
| The RateTimer accept window is not direct equality of the 8-bit facing byte. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` section 3.4. | Rust tests and comments expect exact East in some transitions. | Dock tests and facing target assertions. | Rewrite tests around the RateTimer window and no-snap behavior. | A value inside the verified window is accepted without forcing exact body-facing equality. | Do not make `entity.facing == 0x40` the source of truth for deploy acceptance. |

## 7. Proposed Test Names

- `dock_radio_0x16_sets_drive_rate_timer_without_body_facing_write`
- `dock_radio_0x16_first_sync_does_not_queue_unload`
- `mission_deploy_not_ready_calls_drive_doturn_and_delays_five`
- `mission_deploy_accepts_rate_timer_window_not_exact_facing_byte`
- `mission_deploy_unload_start_does_not_snap_body_facing`
- `cmin_and_harv_share_drive_owned_dock_sync`
- `pivoting_gate_does_not_poll_every_frame_after_not_ready`
- `dock_sync_preserves_first_0x16_then_later_0x15_split`

## 8. Sources

- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_RADIO_0X16_FACING_RECHECK_20260525.md`
- `docs/research/miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`
- `docs/research/miner/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- Current Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_tests.rs`

## 9. Supersession Note

This report supersedes the uncertain phrasing in `DOCK_RADIO_0X16_FACING_RECHECK_20260525.md` where that note said the player-visible East-facing outcome might still be correct enough to preserve. The newer reconciliation with `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` makes the implementation gap stronger:

- East as a compass value is correct.
- `0x4000` in this dock path is still passed through Drive locomotion and RateTimer ownership.
- the mission deploy gate samples the RateTimer window and returns delay `5` when not ready.
- unload start does not explicitly snap the unit body-facing byte.

The remaining uncertainty is the runtime display value on the pad for each approach path, not whether Rust should directly force `entity.facing = 0x40` at unload start.
