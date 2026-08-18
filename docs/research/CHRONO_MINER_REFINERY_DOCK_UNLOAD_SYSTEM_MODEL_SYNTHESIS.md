# Chrono Miner Refinery Dock/Unload - System Model Synthesis

**Date:** 2026-05-24  
**Scope:** stock YR `CMIN/HARV -> GAREFN/NAREFN` refinery return, dock admission, unload, release, two-miner contention, and refinery-loss abort behavior.  
**Non-scope:** ore extraction/tiberium reduction, slave miners, service depots, multi-dock modded buildings, save/load vector reconstruction, and exact presented pixel frames.  
**Output type:** model-synthesis with a small runtime-uncertainty queue.  
**Overall status:** implementation-safe for static contact/retry/unload logic; doc-patch-ready for stale wording; runtime-blocked for exact natural replay frame/pixel outcomes.

## Evidence Ladder Used

| Rank | Meaning in this synthesis |
|---|---|
| BINARY_HIGH | Direct Ghidra spot-check or recent report with active YR path, caller/gate/default checked |
| RESEARCH_HIGH | Recent `re-swarm`/`re-investigate` report with addresses and Rust handoff |
| VERIFY_FINDING | Audit log contradiction/confirmation, useful mainly for stale-doc filtering |
| TRACE_HIGH | Player-visible trace tied to binary evidence |
| DOC_SYNTHESIS | Older overview prose; not canonical when contradicted by newer reports |
| INFERENCE | Plausible but not implementation-safe |

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| A miner release does not promote a waiting miner. | `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `TWO_CMIN_TAKEOVER_FRAME_ORDER_RETRY` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| A release frees contacts synchronously with `BREAK(3)`. | `RadioClass::Transmit/Receive @ 0x0065A970/0x0065A820` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| B admission belongs to B's own `Mission_Enter` / `CAN_DOCK(0x0E)`. | `FootClass::Mission_Enter @ 0x004D9290` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Same-frame takeover is conditional, not guaranteed or impossible. | live-vector iter caller `0x0055AFB0` (`LogicClass` per-tick function entry, not a data address); dispatch `0x005B3060` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Waiting B does not retry `CAN_DOCK` every tick. | direct spot-check `MissionClass::Mission_Dispatch @ 0x005B3060` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock `[Enter]` retry delay is `14..16` frames. | `WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK`; `rulesmd.ini:[Enter] Rate=.016` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Live object order is append/reveal order and normal CMIN lifecycle preserves it. | `LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY` | confirmed | medium-high | yes | IMPLEMENTATION_SAFE for rule; runtime logging for a replay |
| Accepted `CAN_DOCK` cell is refinery anchor `+(3,1)`, not art `QueueingCell=4,1`. | `BuildingClass::Receive_Radio @ 0x0043C2D0`; `CMIN_CLOSE_FAR_RETURN_SPLIT` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Empty-slot gate advances to state 4 without a post-empty dump cooldown. | `EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| State 4 clears unload visual `+0x6D1`; stock refineries do not wait on slot-8 `ProductionAnim`. | direct spot-check `0x0073D630`; art INI | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Zero-link null-refinery state-3 abort preserves cargo and awards no credits. | `REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Zero-link null-refinery abort does not clear `+0x6D1` statically. | direct spot-check `0x0073D630`; `UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS` | confirmed | high | yes | DOC_PATCH_READY; runtime needed for visible frame count |
| Current Rust active refinery tests encode the static binary rule. | `CURRENT_RUST_TWO_MINER_TESTS_VS_BINARY_RULE` | confirmed | high | n/a | IMPLEMENTATION_SAFE as current-status fact |
| Exact natural replay first movement/rendered frame is known statically. | all runtime-order reports | unknown | low | yes | NEEDS_RUNTIME_TRACE |

## Current Model

Stock refinery docking is not a refinery-owned FIFO queue. A miner finishing unload releases only its own active contact. The waiting miner is admitted later by its own mission processing, which may occur in the same frame only if the waiting miner is processed after the releaser and its mission timer is due.

The normal sequence is:

1. Full CMIN/HARV returns toward a stock refinery through `Mission_Harvest`.
2. Close/far split is per unit type via `type+0xcd4` (Teleporter): CMIN (Teleporter set) uses `RulesClass+0xd7c` = `ChronoHarvTooFarDistance` (stock `50` cells); HARV (Teleporter clear) uses `RulesClass+0xd78` = `HarvesterTooFarDistance` (stock `5` cells). Both apply `* 256` lepton scaling with an inclusive close comparison. Verified via `decompile_function 0x0073E5E0` (`UnitClass::Mission_Harvest` case 2; 2026-05-24 audit).
3. Close success sends `HELLO(0x02)` and then proceeds through Mission Enter; far/refused fallback stages at art `QueueingCell=4,1`.
4. Mission Enter sends `CAN_DOCK(0x0E)`.
5. Accepted `CAN_DOCK` sends radio `0x12` with refinery anchor `+(3,1)`. A miner at `QueueingCell=4,1` first moves to this accepted cell; it does not immediately enter/pivot unless already there.
6. Unload state 3 drains one whole storage slot per dump gate, credits the refinery owner, and resets the dump accumulator.
7. The next dump gate after all slots are empty writes state 4 and clears slot-10 special anim.
8. Stock state 4 clears `+0x6D1`, queues Harvest, and sends `BREAK(3)` if a valid contact exists.
9. `BREAK(3)` frees the contact; no waiting miner callback runs.
10. Any waiting B can claim only on B's own eligible Mission Enter pass.

## Implementation-Safe Facts

- Do not add refinery-side waiter promotion. The active Rust `RefineryDockContacts` model is aligned for this rule.
- Preserve order-dependent takeover tests: later eligible waiter can claim in the same tick; earlier waiter cannot be retroactively promoted.
- Add or preserve Mission Enter retry timing if exact takeover-frame parity is targeted: stock `[Enter] Rate=.016` yields base `14` plus inclusive `RandomRanged(0,2)`.
- Keep `QueueingCell` separate from the accepted `CAN_DOCK` cell.
- Do not release the miner on the same tick as the last real slot drain. Release follows the later empty-slot gate and state-4 handoff.
- Do not add a post-empty dump cooldown for stock refineries.
- Preserve cargo and award no credits when the refinery disappears before the zero-link state-3 drain branch.

## Doc-Patch-Ready Facts

- Replace any "refinery promotes queued miner" wording with: "A release frees the active contact via `BREAK(3)`; B admits only during B's own eligible `Mission_Enter` / `CAN_DOCK` retry."
- Replace "object-vector order unknown" with: "static binary evidence proves append/reveal order and normal CMIN lifecycle stability; concrete replay indices remain runtime-only."
- Replace "mission timer value unknown" with: "stock `[Enter]` retry is `14..16` frames; the replay-specific stored timer and RNG draw remain runtime-only."
- Replace "null-refinery abort clears unload visual" with: "static binary evidence shows the zero-link state-3 null-refinery branch does not clear `+0x6D1`; exact presentation requires runtime capture."
- Mark older hardcoded 2-cell chrono return threshold warnings as stale for current Rust; current Rust uses parsed `ChronoHarvTooFarDistance` for stock behavior.

## Stale Or Superseded Claims

- `HARVESTER_DOCK_UNLOAD.md`, `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, and `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` contain audited RED/YELLOW claims around refinery queueing, radio roles, normal exit, or dock state names. Use their audit log entries as warnings, not canonical implementation sources.
- Older "Rust promotes queued miner when refinery releases" wording is superseded by `CURRENT_RUST_TWO_MINER_TESTS_VS_BINARY_RULE`.
- Older "runtime object-vector order unknown" wording is narrowed by `LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY`.
- Older "mission timer value runtime-only" wording is narrowed by `WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK`.
- Older "state-3 missing refinery clears visual" wording is contradicted by `UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS`.

## Cross-Doc Conflicts

No broad static-model conflict remains for the scoped stock refinery dock/unload path. The older contradictory docs are superseded by newer reports plus spot-checks.

Remaining disagreement is not a static Ghidra conflict; it is an evidence boundary:

- static code proves command and flag writes;
- runtime capture is still needed for exact player-visible frame count in a concrete replay.

## Needs Re-Investigation

Use runtime/debugger-oriented research only if the exact visual frame matters before implementation:

- `/re-investigate two CMIN one refinery live-vector mission-timer runtime capture`
  - Needed to log A/B live-vector index, B `+0xC8/+0xD0`, and B RNG/timer state on A's release frame.
- `/re-investigate CMIN refinery loss unloading visual runtime frame count`
  - Needed to prove whether stock presents stale `CMON/HORV` after zero-link null-refinery abort and for how many frames.
- `/re-investigate save load live object vector order for miners`
  - Needed only if persistence parity is being implemented.

## Do-Not-Implement Notes

- Do not implement refinery-side FIFO promotion on release.
- Do not poll refused/busy `CAN_DOCK` every tick.
- Do not collapse `HELLO`, `CAN_DOCK`, accepted-cell movement, and entered/pivot into one state.
- Do not use art `QueueingCell=4,1` as the accepted dock cell.
- Do not cite generic `DockReservations::release_promotes_next` as refinery behavior.
- Do not make same-frame takeover universally true or universally false.
- Do not claim the null-refinery abort clears `CMON/HORV` unless runtime capture proves no stale frame is presented.

## Source Ledger

Recent reports:

- `miner/TWO_CMIN_TAKEOVER_FRAME_ORDER_RETRY_GHIDRA_REPORT.md`
- `miner/WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK_GHIDRA_REPORT.md`
- `miner/LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY_GHIDRA_REPORT.md`
- `miner/UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS_GHIDRA_REPORT.md`
- `miner/CURRENT_RUST_TWO_MINER_TESTS_VS_BINARY_RULE_GHIDRA_REPORT.md`
- `miner/EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING_GHIDRA_REPORT.md`
- `miner/REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS_GHIDRA_REPORT.md`
- `miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`

Spot-checked directly:

- `MissionClass::Mission_Dispatch @ 0x005B3060`: confirms timer gate and mission `7` vtable `+0x240`.
- `UnitClass::Mission_Deploy_Building @ 0x0073D630`: confirms state-3 drain/null-refinery/state-4 `+0x6D1` behavior.

INI defaults:

- `ini/rulesmd.ini:[General] ChronoHarvTooFarDistance=50` (CMIN threshold; `type+0xcd4` set)
- `ini/rulesmd.ini:[General] HarvesterTooFarDistance=5` (HARV threshold; `type+0xcd4` clear)
- `ini/rulesmd.ini:[Enter] Rate=.016`
- `ini/rulesmd.ini:[CMIN]/[HARV] Harvester=yes`, `Dock=NAREFN,GAREFN`, `UnloadingClass=CMON/HORV`
- `ini/rulesmd.ini:[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`, stock one-dock behavior
- `ini/artmd.ini:[GAREFN]/[NAREFN] QueueingCell=4,1`

Rust surfaces:

- `src/sim/miner/miner_dock.rs::RefineryDockContacts`
- `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`
- `src/sim/miner/miner_dock_sequence.rs::phase_unloading`
- `src/sim/miner/miner_dock_sequence.rs::phase_departing`
- `src/sim/miner/miner_system.rs::tick_miners`
- `src/sim/miner/miner_tests.rs` two-miner and accepted-cell tests
