# HARV dock/unload edge uncertainties - focused Ghidra report

**Date:** 2026-05-22  
**Scope:** Resolve the War Miner re-swarm residual questions: stale `HORV` draw state after missing-refinery abort, second-miner takeover timing after state-4 release, and retained/manual target behavior during dock/unload.  
**Primary evidence:** `gamemd.exe` static decompilation. Live debugger/runtime tracing was unavailable (`Debugger server not running at http://127.0.0.1:8099`), so exact rendered-frame counts remain runtime-only.

## Executive result

1. **Missing-refinery state-3 abort does not clear the `HORV` draw flag.** `UnitClass::Mission_Deploy_Building @ 0x0073D630` queues Harvest mission `10` with `commence_now=1`, then returns a mission-timer delay. It does not write `unit+0x6D1 = 0` on that branch. `UnitClass::DrawExtras @ 0x0073CEC0` draws `UnloadingClass` whenever `Harvester=yes`, `unit+0x6D1 != 0`, and `Type+0x6B8 != 0`. Therefore a rendered frame after this abort can still draw `HORV`.
2. **Exact stale-HORV frame count is still not proven statically.** `MissionClass::Queue_Mission @ 0x005B35E0` and `MissionClass::Commence @ 0x005B3570` do not clear `unit+0x6D1`; they only update mission/queued-mission fields and timers. The abort branch returns `ftol(MissionTimerEntry[mission]+0x10 * 900.0) + RandomRanged(0,2)`, but the exact runtime timer table value was not read. A runtime trace is still needed to count rendered frames between abort and the next path that clears `+0x6D1`.
3. **Second-miner takeover is retry-driven, not directly promoted by the first miner.** The stock zero-link state-4 exit clears `unit+0x6D1`, queues Harvest, and conditionally sends radio `3`/BREAK if the path/contact state says to do so. It does not call `ReleaseDockedHarvester`, does not walk a waiter list, and does not directly issue a move to the next miner. A second miner can take over in the same global frame only if its own AI/mission dispatch runs after the first miner's release in that frame and retries `CAN_DOCK`; if it already ran earlier, takeover waits for its next dispatch.
4. **Retained/manual target behavior splits by target source.** Passive/opportunistic targets are explicitly cleared during missions including Enter `7` and Unload `16` when the opportunity-target flag is set. Manual/explicit targets are not proven cleared by that mission list alone; `UnitClass::AI @ 0x007360C0` calls `UnitClass::Fire_At_Target @ 0x00736DF0` every active tick after `FootClass::AI`, with no direct mission-7/mission-16 exclusion in `Fire_At_Target`. Runtime capture is still needed for exact manual-order frame behavior.

## Evidence details

### 1. Missing-refinery abort and stale `HORV`

`UnitClass::Mission_Deploy_Building @ 0x0073D630`, harvester unload state 3:

- The normal dump-init path sets `unit+0x6D1 = 1` before the first drain and before `param_1[0x2F] = 3`.
- The normal state-4 branch clears `unit+0x6D1 = 0` before returning to Harvest.
- The missing-refinery state-3 branch:
  - performs adjacent-cell lookup for the refinery;
  - if no building is found and path steps are valid, optionally sends radio `3`;
  - calls `Queue_Mission(10, 1)`;
  - calls `MissionClass::GetMissionTimerEntry`, `Math::ftol`, and `RandomRanged(0,2)`;
  - returns without clearing `unit+0x6D1`.

`MissionClass::Queue_Mission @ 0x005B35E0` writes queued mission fields `+0xB4/+0xB8` and, when `commence_now` is true, calls `Commence`; no `unit+0x6D1` write is present.

`MissionClass::Commence @ 0x005B3570` moves queued mission `+0xB4` to current mission `+0xAC`, resets mission timers around `+0xC0..+0xD0`, and clears the queued flag byte; no `unit+0x6D1` write is present.

`UnitClass::DrawExtras @ 0x0073CEC0` gates `UnloadingClass` drawing only on `Harvester=yes`, `unit+0x6D1`, and `TechnoTypeClass+0x6B8`. The missing-refinery abort leaves the draw gate true until some later path clears it.

### 2. Second-miner takeover timing

The normal stock CMIN/HARV unload completion remains the zero-link `Mission_Deploy_Building` state-4 path. In that path:

- `unit+0x6D1 = 0` is cleared.
- The unit queues Harvest mission `10`.
- If the object still has valid path/contact state, it sends radio `3`/BREAK and then commences queued mission work.
- There is no call to `BuildingClass::ReleaseDockedHarvester`.
- There is no direct call that assigns a waiting miner to the dock.

`MissionClass::Mission_Dispatch @ 0x005B3060` runs per object when its mission timer is due. This means same-frame takeover is possible only as an object-order effect: miner A releases the contact/pad, then miner B must still get a later mission dispatch in the same global frame and retry `CAN_DOCK`.

Static evidence therefore proves "no direct promotion" but not a universal "same frame" or "not same frame" result. The answer depends on object iteration order and the runtime mission timer state for the waiting miner.

### 3. Retained/manual target through Enter/Unload

`TechnoClass::AI_Update @ 0x006F9E50` has a target-clear gate:

- if `Target != null`;
- and `field_0x50C != 0`;
- and current mission is in a list that includes mission `7` (Enter) and mission `16` (Unload);
- then it calls target clear (`vtable+0x3C8(0)`).

The same function only starts passive acquisition for missions `2`, `10`, and `5` (`Move`, `Harvest`, `Guard`) via the later `FUN_00709290` / `vtable+0x39C` path. Enter and Unload are not passive-acquire missions.

`UnitClass::AI @ 0x007360C0` calls `FootClass::AI`, then checks current mission only to clear `unit+0x6D2` when the mission is not Harvest `10`, then calls `UnitClass::Fire_At_Target @ 0x00736DF0`. `Fire_At_Target` checks target/action/weapon gates, but it does not contain a direct "if mission is Enter/Unload, do not fire" guard.

This resolves stock opportunistic retention: opportunistic targets marked by `field_0x50C` are cleared on Enter/Unload before normal fire handling can persist them. It does not fully resolve manual/explicit attack targets, because the manual path needs runtime or caller tracing to confirm whether it sets the same `field_0x50C` flag or is cleared earlier by order handling.

## Rust-facing handoff

- Do not clear the unload visual merely because the refinery lookup fails in unload state 3 if matching stock exactly. The static binary leaves `+0x6D1` set on that abort. If Rust clears `display_type_override` immediately on destroyed/sold refinery abort, that is a deliberate visual divergence unless a later runtime capture proves no rendered stale frame.
- Do clear the visual on the normal stock state-4 handoff. Rust's `phase_departing` cleanup matches the normal `+0x6D1 = 0` branch in intent.
- The Rust visual currently starts `display_type_override` in `phase_linked`, while the binary sets `+0x6D1` at dump-init just before state 3. That remains a likely early-HORV mismatch independent of the missing-refinery abort.
- Model second-miner takeover as retry after contact/pad release, not as parent-side direct promotion. A same-frame test should be framed around deterministic entity iteration: A releases, then B retries later in the same frame only when B has not already run.
- For War Miner combat during dock, implement three separate concepts: passive acquisition allowed on Harvest mission, passive/opportunity target clear on Enter/Unload, and manual target retention/fire pending a runtime trace. Do not use mission-control `Retaliate=no` as a blanket "cannot fire" rule.

## Coverage ledger

| Question | Static status | Remaining runtime work |
|---|---:|---|
| Does missing-refinery state-3 abort clear `HORV` flag? | Resolved: no clear in branch | None for branch write |
| Can stale `HORV` render after abort? | Resolved as possible/expected if a frame renders before later clear | Pixel/runtime capture for exact count |
| Exact stale-HORV frame count | Partial | Need live trace with draw frames and `unit+0x6D1` watch |
| Does state-4 directly promote second miner? | Resolved: no direct promotion | None |
| Can second miner take over same global frame? | Conditional static answer | Runtime or scheduler trace for object-order/timer case |
| Passive target retention during Enter/Unload | Resolved: opportunity targets cleared when `field_0x50C` set | None for that path |
| Manual target retention during Enter/Unload | Partial | Need order-command trace: whether manual target sets `field_0x50C`, and whether fire happens during dock frames |

## Suggested runtime trace points

- Watch `unit+0x6D1` and current mission around `UnitClass::Mission_Deploy_Building @ 0x0073D630`, specifically the missing-refinery state-3 branch.
- Break on `UnitClass::DrawExtras @ 0x0073CEC0` for the same unit after the abort and count rendered frames where `+0x6D1 != 0`.
- Trace two full miners at one refinery with object IDs ordered both A-before-B and B-before-A if possible; log state-4 release, radio `3`, B's next `CAN_DOCK`, and first B movement toward accepted cell.
- Trace a War Miner given a manual attack target before dock admission; log target pointer, `field_0x50C`, mission `7/16`, and calls to `UnitClass::Fire_At_Target`.

## Sources touched

- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra decompile: `UnitClass::DrawExtras @ 0x0073CEC0`
- Ghidra decompile: `MissionClass::Queue_Mission @ 0x005B35E0`
- Ghidra decompile: `MissionClass::Commence @ 0x005B3570`
- Ghidra decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`
- Ghidra decompile: `TechnoClass::AI_Update @ 0x006F9E50`
- Ghidra decompile: `UnitClass::AI @ 0x007360C0`
- Ghidra decompile: `UnitClass::Fire_At_Target @ 0x00736DF0`
- Existing reports: `HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`, `HARV_POST_UNLOAD_EXIT_PATH_GHIDRA_REPORT.md`, `HARV_ARMED_BEHAVIOR_DURING_HARVEST_DOCK_GHIDRA_REPORT.md`, `TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md`
