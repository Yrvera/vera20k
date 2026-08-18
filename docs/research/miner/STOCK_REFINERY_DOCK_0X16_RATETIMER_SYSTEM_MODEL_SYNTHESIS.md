# Stock Refinery Dock 0x16 RateTimer System Model Synthesis

**Date:** 2026-05-26  
**System:** stock YR `CMIN/HARV -> GAREFN/NAREFN` refinery dock sync, radio `0x16`, and mission `0x10` unload-facing gate.  
**Output type:** model-synthesis.  
**Included surfaces:** accepted-cell handoff context, `0x18`, first/later `0x16`, `0x15`, mission `0x10`, RateTimer/facing gate, Rust dock-facing deltas.  
**Non-scope:** full cargo credit arithmetic, state-4 exit details except where they prevent `Force_Track` confusion, slave miners, service depots, aircraft docks, VXL render bucket proof, and runtime capture of exact visible body-facing on every refinery approach.  
**Ghidra spot-check status:** unavailable in this session. `mcp__ghidra_mcp__.list_instances` reported no running Ghidra instances, so this synthesis relies on existing Ghidra-backed reports, audits, and INI defaults.  
**Safety classification:** implementation-safe for the static stock dock stage split and for removing direct forced body-facing snaps from dock `0x16` / unload start. Runtime trace still needed for exact visible body-facing frame on every approach path.

## 1. Current Model

Stock refinery docking is a staged radio/mission pipeline, not a single "arrived on pad, face East, unload" event.

1. The miner enters mission `7` / `Enter` and sends `CAN_DOCK(0x0E)` only on a due mission-dispatch pass.
2. The refinery replies with `MOVE_TO_CELL(0x12)` to accepted cell `NW+(3,1)`. If the miner must move, no `0x18`, `0x16`, or unload handoff is sent in that pass.
3. On a later already-there pass, the refinery sends `0x18` then `0x16`.
4. `0x18` sets the contact-entered flag `+0x418`; it is not unload-active state and not reciprocal pad occupancy.
5. First ordinary `0x16` performs active-locomotor timing/rate sync and returns. It does not call `GetDockCoord`, does not set a destination, does not write location, and does not start unload.
6. Later/already-synced `0x16`, under stopped/destination/contact/mission gates, may send `0x15` to the building.
7. Building `0x15` queues sender mission `0x10`; it does not itself start unload.
8. Mission `0x10` / `UnitClass::Mission_Deploy_Building` owns the actual unload-start gates. It checks path validity, then checks the RateTimer accept window.
9. If the RateTimer window is not ready, mission `0x10` calls active locomotor `+0x4C(0x4000)` and returns delay `5`.
10. If accepted, mission `0x10` writes unload-active/timer/substate fields and enters dump state. It does not explicitly snap the body-facing byte to East in the verified init block.

The important facing distinction: East itself is not inverted. `0x40` is East in normal 8-bit facing, direction index `2` is East, and cell delta `(1,0)` is East. The drift is that dock code treats `0x4000` as if it were a direct body-facing target. In this path, existing Ghidra-backed reports identify `0x4000` as the argument to active Drive locomotion `Do_Turn`, whose concrete decompile is a RateTimer set.

## 2. Claim Table

| Claim | Best evidence | Status | Confidence | Active YR | Safe? |
|---|---|---|---|---|---|
| Stock HARV and CMIN both enter this refinery dock/unload path. | `rulesmd.ini` `[CMIN]/[HARV] Dock=NAREFN,GAREFN`, `Harvester=yes`; `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`; mission deploy verification. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Accepted `0x12` movement target is `NW+(3,1)`, not `GetDockCoord`. | current stock refinery system synthesis; accepted-cell audit. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| First ordinary `0x16` syncs rate/timer only and can return before `0x15`. | `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT`, `RADIO_LINK...DOC_CONFLICT_AUDIT`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x16` does not call `GetDockCoord`, set destination, write position, or start unload. | same conflict audits and current system model. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x4000` in the dock `0x16` path is not a direct body-facing setter. | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT`: Drive `+0x4C` is `Do_Turn`, decompile is `RateTimer__Set(&param_2)`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Mission `0x10` unload-start gate samples `((RateTimerCurrent >> 7) + 1) & 0x1FE == 0x80`. | `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| If mission `0x10` is not ready, it calls locomotor `+0x4C(0x4000)` and returns delay `5`. | same mission deploy verification. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Accepted unload start explicitly forces body facing to East. | mission deploy verification OQ-07; facing report. | contradicted | high | yes | DO_NOT_IMPLEMENT |
| Current Rust exact East snap at unload start is parity-correct. | Rust scan plus mission deploy verification. | contradicted | high | yes | DO_NOT_IMPLEMENT |
| Exact visible body-facing byte during every dump scenario is fully known from static docs. | facing report OQ-10; runtime-sensitive notes. | unknown | medium | yes | NEEDS_REINVESTIGATE/runtime trace |

## 3. Implementation-Safe Facts

- `CMIN` and `HARV` share this stock dock/unload path once the Chrono Miner is in active Drive dock approach. CMIN's Teleporter property does not make radio `0x16` a CMIN-only behavior.
- Normal compass data is correct: `0=N`, `64=E`, `128=S`, `192=W`; direction index `2` maps to East `(1,0)`.
- `0x18`, `0x16`, `0x15`, and mission `0x10` are separate stage boundaries.
- First `0x16` is not unload start and should not produce unload sound, display override, cargo drain, pad occupancy, or body-facing snap.
- `0x15` queues mission `0x10`; it is not unload start.
- Mission `0x10` starts unload only after path and RateTimer/facing-window gates.
- A failed mission `0x10` RateTimer gate returns delay `5`; every-frame polling is not parity unless another scheduler proves equivalent.
- Unload-start init writes the unload-active/timer/substate cluster. It does not explicitly force the unit facing byte.

## 4. Doc-Patch-Ready Facts

- Any prose saying dock `0x16` is a proven East-facing pivot should be patched to say it is active-locomotor RateTimer sync through `Do_Turn(0x4000)`.
- Any prose saying `0x16` bridges the miner to `GetDockCoord` should be patched: `0x16` has no `GetDockCoord`, no `Set_Destination`, and no location write.
- Any prose saying `0x16 == 1` means `0x15` was sent should be patched: first ordinary `0x16` can return `1` after sync only.
- Any prose saying unload start snaps body facing should be patched: verified mission `0x10` init requires the RateTimer window and does not contain an explicit body-facing write.

## 5. Stale Or Superseded Claims

| Stale claim | Superseding evidence | Replacement model |
|---|---|---|
| "`0x4000` is East, therefore dock `0x16` forces East." | `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT`; `Do_Turn` resolves to RateTimer set. | `0x4000` is passed to Drive locomotion RateTimer/turn sync in this path. |
| "Accepted pad arrival, `0x16`, `0x15`, and unload start are one linked/on-pad event." | `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS`; lifecycle map. | Keep accepted movement, contact-entered, first sync, later handoff, mission queue, and mission deploy as separate stages. |
| "Rust's final `entity.facing = 0x40` snap hides harmless internal differences." | mission deploy verification; parity rules. | It is a behavior write where the verified gamemd init block has no body-facing write. Treat as drift. |
| "War Miner and Chrono Miner need different dock `0x16` handling." | stock INI gates plus CMIN active Drive dock approach reports. | Both share the stock Drive-owned dock sync path during dock approach. |

## 6. Cross-Doc Conflicts

- The current stock refinery dock system model is mostly aligned with the newer facts, but its wording "`0x16` may only set/sync facing timer `+0x388` toward `0x4000`" can still be misread as a body-facing pivot. The stricter wording should be "active-locomotor RateTimer/rate sync through `Do_Turn(0x4000)`; no direct body-facing setter."
- Older dock arrival/pivot and radio-link reports are mixed-validity. Use their doc-conflict audits and the current system synthesis, not the stale body prose.
- The exact runtime first `0x15` source remains intentionally unresolved: later `0x16` and PerCellProcess branches can both be valid source families depending on frame/order.

## 7. Needs Re-Investigation

- Runtime trace HARV and CMIN dock cycles around first `0x18/0x16`, later `0x15`, mission `0x10`, RateTimer current, body-facing byte, active locomotor class, and render-facing frame. This is needed to pin the exact visible body-facing sequence, not to prove the direct snap is wrong.
- Fresh Ghidra spot-check of `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, `RateTimer::Set`, `RateTimer::Current`, and the `UnitClass::Receive_Radio(0x16)`/`Mission_Deploy_Building` ranges when Ghidra is available. Existing docs are strong enough for the implementation direction, but a fresh check would make the handoff self-contained.
- A bounded render investigation for how `CMON`/`HORV` chooses facing buckets if visual frame parity is the next target.

## 8. Do-Not-Implement Notes

- Do not implement `0x16` as a direct `entity.facing = 0x40` or as a miner-FSM-owned East pivot.
- Do not snap facing at unload start.
- Do not poll mission `0x10` facing/RateTimer readiness every frame after a failed gate.
- Do not treat `0x18` contact-entered or Rust `on_pad` as a stock reciprocal `+0x2E4` pad link.
- Do not play `DockDeploy` or start unload display/cargo drain at `0x16` or `0x15`.
- Do not special-case CMIN away from HARV for this dock sync; the distinction is how CMIN returns, not the stock dock `0x16`/mission `0x10` gate once it is driving into the refinery.

## 9. Rust-Facing Handoff

| Rust surface | Current issue | Required direction |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs` `DOCK_FACING_EAST*` and `sync_dock_facing` | Models `0x4000` as miner-owned East body-facing target. | Replace with active-locomotor RateTimer/turn-sync representation or a bridge that preserves that ownership. |
| `phase_pivoting` | Checks readiness every sim tick and goes straight to `start_unload_deploy`. | Model mission `0x10` dispatch/return delay `5` when not ready, plus path gate before unload init. |
| `start_unload_deploy` | Calls `link_on_pad`, forces `entity.facing = 0x40`, emits `DockDeploy`, seeds local timer. | Remove facing snap; avoid stock physical pad-link meaning; model `+0x6D1`-like unload-active latch and timer/substate ordering. |
| miner tests | Some tests assert exact East snap/pivot behavior. | Rewrite around no direct body-facing write, RateTimer-window accept, and five-frame not-ready delay. |

## 10. Source Ledger

- `docs/research/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/STOCK_REFINERY_DOCK_UNLOAD_LIFECYCLE_DOC_MAP.md`
- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_0X16_DOTURN_RATETIMER_UNLOAD_GATE_RECHECK_20260526.md`
- INI gates checked: `ini/rulesmd.ini` `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`; `ini/artmd.ini` `[GAREFN]/[NAREFN] QueueingCell=4,1`.
- Current Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_tests.rs`.

