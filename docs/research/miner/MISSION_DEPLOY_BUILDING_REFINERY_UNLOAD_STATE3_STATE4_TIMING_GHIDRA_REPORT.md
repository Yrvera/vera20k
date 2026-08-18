# Mission Deploy Building Refinery Unload State 3 -> 4 Timing - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock healthy `HARV`/`CMIN` unloading at stock `GAREFN`/`NAREFN`; exact state-3 empty-storage gate to state-4 scheduler boundary.  
**Non-Scope:** miner target selection, two-miner handoff, destroyed/sold refinery handling, Force_Track visuals, slave miner/Yuri refinery, modded refinery art beyond stock delay implications.  
**Confidence:** High for stock state-3/state-4 timing; Medium for the exact same-frame vs next-frame external scheduler label because no runtime trace was taken.  
**Active in YR:** Yes. The path is reached by stock `DockUnload=yes`, `Refinery=yes` buildings and stock `Harvester=yes` `HARV/CMIN` units.

## 0. Working Notes Gate

Target question: After the last ore/gem storage slot drains in `UnitClass::Mission_Deploy_Building` state 3, exactly when does standard YR write substate 4, clear `+0x6D1`, and release the unload/display state?

Non-goals: Do not re-open whole miner AI, two-miner handoff, destroyed refinery, reciprocal `+0x2E4` release paths, or Force_Track visuals except where they constrain this boundary.

Evidence needed to mark COMPLETE: decompile plus inspected disassembly ranges for `0x0073D630`; decompile of storage helpers; INI/art proof for stock `HarvesterDumpRate`, `DockUnload`, `Refinery`, `UnloadingClass`, and absent stock refinery `ProductionAnim`; current Rust scan of `phase_unloading`, `phase_deposit_cooldown`, and `phase_departing`.

Stop conditions: Stop once every state-3 timer branch, empty-slot branch, state-4 guard, `+0x6D1` clear, return value, and current Rust delta has a resolved or explicitly deferred evidence line.

## 1. Overview

For stock healthy refinery unload, the last real dump gate removes the full first non-empty StorageClass slot and resets the unit dump counter. The miner does not leave on that same gate. On the next dump-rate gate, state 3 finds no non-empty slot, optionally requests building animation slot 8, writes mission substate `4`, optionally clears slot 10, and returns `1`. State 4 then runs on a later mission call, waits only if stock-unset `ProductionAnim` slot 8 exists, clears `unit+0x6D1`, and resumes harvest mission scheduling.

## 2. Class Layout / Key Offsets

| Object | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| UnitClass | `+0xBC` (`param_1[0x2f]`) | Deploy-building substate; `3` dump, `4` finish | `0x0073D630` decompile; state switch | Yes |
| UnitClass | `+0xF8` (`param_1[0x3e]`) | Dump gate counter compared to `HarvesterDumpRate * 900.0`; reset to 0 after a positive removal | `0x0073E330..0x0073E539` decompile/disassembly inspected | Yes |
| UnitClass | `+0x6D1` | Unload-active byte; set before state 3, cleared in state 4 | set path `0x0073DFDA`; clear paths `0x0073E0D0..0x0073E2A4`, `0x0073DEF0..0x0073DF10` | Yes |
| UnitClass | `+0x6C4` (`param_1[0x1b1]`) | UnitType pointer; flags distinguish harvester vs weeder-like storage path | `0x0073D630` decompile | Yes |
| BuildingClass | `+0x57C` | `Anims_0[8]`, `ProductionAnim` slot pointer | `BuildingClass::SetAnimSlotImage @ 0x00451750`; `ClearAnimSlot @ 0x00451E40`; prior guard doc | Conditional |
| BuildingClass | `+0x584` | `Anims_0[10]`, `SpecialAnim` slot pointer | `0x0073E330..0x0073E539` decompile; clear after state-4 write | Yes |
| BuildingTypeClass | `+0x16BB` | `Refinery=yes` gate for slot-8/slot-10 anim calls and state-4 guard | `0x0073D630`; `rulesmd.ini` stock refineries | Yes |
| RulesClass | `+0x1528` | `HarvesterDumpRate` double | `0x0073E330..0x0073E374`; `RulesClass::ReadGeneral @ 0x00670CD4` | Yes |

## 3. Core Logic

State 3 only evaluates storage at the dump-rate boundary:

- Gate condition: `RulesClass+0x1528 * 900.0 <= UnitClass+0xF8`.
- Stock default: `0.016 * 900.0 = 14.4` frames. `rulesmd.ini` and `rules.ini` do not override `HarvesterDumpRate`; Rust parser default is `144` tenths.
- On each gate, it calls `StorageClass::FindFirstNonEmptySlot`.
- If a slot exists, it reads the full slot amount, removes that amount, credits it, and resets `UnitClass+0xF8` to `0`.
- If no slot exists (`-1`), it does not seed another dump gate. It requests slot 8 if `Refinery=yes`, writes substate `4`, clears slot 10 if the pointer is non-null, then returns `1`.

Critical timing result:

`last real slot drain gate` -> `counter reset` -> waits another dump-rate interval -> `empty-slot gate writes state 4 and returns 1` -> next state-4 execution can clear `+0x6D1` immediately for stock refineries with no slot-8 object.

State 4:

- Rediscovers the refinery from the unit cell using the standard adjacent lookup, then checks `Refinery=yes` and `building+0x57C`.
- If slot 8 exists, returns `1` without clearing `+0x6D1`.
- For stock `GAREFN/NAREFN`, slot 8 is not populated because stock art has no active `ProductionAnim`; the guard normally falls through.
- It clears `unit+0x6D1 = 0`, calls the mission-set/scheduler path for mission `10`, optionally stops/requeues based on movement/path state, and returns through `MissionClass::GetMissionTimerEntry` plus `RandomRanged(0,2)` on one return path.

Return/delay boundary:

- State-3 real deposit gate: returns `1` after the branch.
- State-3 empty-slot gate: returns `1`; state 4 is not executed inside the same switch case after writing `+0xBC = 4`.
- State-4 stock no-slot-8 path: performs cleanup in that mission call; return is path-dependent but no dump-rate or SpecialAnim cooldown is introduced.

## 4. INI Keys

| Key | Stock source | Stock value / status | Effect | Active in YR |
|---|---|---|---|---|
| `[General] HarvesterDumpRate` | absent in `rulesmd.ini`/`rules.ini`; binary/Rust default | `0.016` min per gate | `0.016 * 900 = 14.4` frames per slot/empty gate | Yes |
| `[CMIN] Storage` | `rulesmd.ini:7374` | `20` bales | stock chrono miner capacity | Yes |
| `[CMIN] UnloadingClass` | `rulesmd.ini:7384` | `CMON` | display override while `+0x6D1` unload-active is set | Yes |
| `[HARV] Storage` | `rulesmd.ini:8236` | `40` bales | stock war miner capacity | Yes |
| `[HARV] UnloadingClass` | `rulesmd.ini:8246` | `HORV` | display override while unloading | Yes |
| `[GAREFN] DockUnload/Refinery` | `rulesmd.ini:11726..11727` | `yes/yes` | activates stock refinery unload path | Yes |
| `[NAREFN] DockUnload/Refinery` | `rulesmd.ini:12519..12520` | `yes/yes` | activates stock refinery unload path | Yes |
| `[GAREFN] SpecialAnim` | `artmd.ini:1787` | `GAREFNOR` | slot 10 deposit animation on real dump gates | Yes |
| `[NAREFN] SpecialAnim` | `artmd.ini:1739` | `NAREFNOR` | slot 10 deposit animation on real dump gates | Yes |
| `[GAREFN/NAREFN] ProductionAnim` | `artmd.ini:1749` commented for NAREFN; none active for GAREFN | absent/commented | slot-8 state-4 wait is normally a no-op | Conditional: no stock delay |

## 5. Integration Points

`UnitClass::Mission_Deploy_Building @ 0x0073D630` owns this stock refinery unload slice. Older docs that route stock dump completion through `BuildingClass::MissionRepairAndProduce`, `BuildingClass::UndockUnit`, or `BuildingClass::ReleaseDockedHarvester` are stale for the zero-link stock branch.

Storage helper verification:

- `StorageClass::FindFirstNonEmptySlot @ 0x006C9820` scans four float slots, returns first slot whose amount is greater than zero, else `-1`.
- `StorageClass::GetAmount @ 0x006C9680` returns the full float amount at the selected slot.
- `StorageClass::RemoveAmount @ 0x006C96B0` subtracts the requested amount or saturates the slot to zero if the stored amount is smaller.

Animation helper verification:

- `BuildingClass::SetAnimSlotImage @ 0x00451750` computes the slot art pointer from `Type + 0xF4C + slot * 0x44`, damaged/alternate variants included, and only creates an anim if the selected name is non-empty.
- `BuildingClass::ClearAnimSlot @ 0x00451E40` nulls the selected `Anims_0[slot]` pointer before destroying the old anim.

## 6. Current Rust Implementation Status

Current Rust appears to have already fixed the known extra post-last-slot hold for new stock unloads:

- `src/sim/miner/miner_dock_sequence.rs:798..882`: `phase_unloading` decrements `unload_timer`; on a real slot it drains one resource type and adds `unload_tick_interval`; on `next_slot == None` it sets `deposit_cooldown_ticks = 0` and transitions directly to `Departing`.
- `src/sim/miner/miner_dock_sequence.rs:888..895`: `phase_deposit_cooldown` is retained only for legacy save/test states.
- `src/sim/miner/miner_dock_sequence.rs:898..941`: `phase_departing` clears display override, dock/contact bookkeeping, stale movement/track state, and returns to `SearchOre`.
- `src/sim/miner/mod.rs:144..232` and `src/rules/ruleset.rs:1017..1018`: Rust represents `HarvesterDumpRate` as tenths of a frame, default `144`.
- `src/sim/miner/miner_tests.rs:4074`: existing test `empty_unload_gate_releases_dock_on_next_stock_state4_handoff` pins the empty gate -> Departing -> next-tick stock handoff boundary.

Delta against binary for this exact slice: no current extra `DepositCooldown` observed for new stock unloads. Remaining Rust delta in this narrow slice is mainly representational: Rust has no explicit `+0x6D1` byte, state-4 mission return value, or stock `ProductionAnim` slot-8 wait model. For stock `GAREFN/NAREFN`, the slot-8 wait is player-invisible because no active `ProductionAnim` exists.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building` stock zero-link path | verified | decompile `0x0073D630`; disassembly ranges inspected `0x0073E330..0x0073E545`, `0x0073E0D0..0x0073E2A4` | none for timing slice |
| State-3 dump-rate gate | verified | `0x0073E330..0x0073E374`; `Rules+0x1528 * 900.0 <= unit+0xF8` | none |
| State-3 real slot removal | verified | `0x0073E374..0x0073E539`; storage helpers `0x006C9820/9680/96B0` | none |
| State-3 empty-slot transition | verified | `0x0073E4B0..0x0073E545`; `FindFirstNonEmptySlot == -1` -> slot 8 -> `+0xBC = 4` -> clear slot 10 | none |
| State-4 slot-8 guard | verified for stock no-delay condition | `0x0073E0D0..0x0073E2A4`; `artmd.ini` absence/commented `ProductionAnim` | modded `ProductionAnim` duration out-of-scope |
| `unit+0x6D1` clear timing | verified | set before state 3 at `0x0073DFDA`; clear in state 4 at `0x0073E1F6`/weeder equivalent | exact rendered frame of display swap would require runtime capture |
| `BuildingClass::ReleaseDockedHarvester` | verified negative for stock zero-link | top branch `param_1[0xB9] != 0` call near `0x0073D64F..0x0073D672`; stock docs say zero-link | none for this slice |
| Current Rust `phase_unloading` | verified by source scan | `src/sim/miner/miner_dock_sequence.rs:798..882` | run focused test suite if implementing |
| Current Rust `phase_deposit_cooldown` | verified by source scan | `src/sim/miner/miner_dock_sequence.rs:888..895` | legacy-save-only path, not stock binary |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is the stock path live in YR? -> Yes, stock `CMIN/HARV` dock at `DockUnload=yes` and `Refinery=yes` stock refineries, entering mission `0x10`/`UnitClass::Mission_Deploy_Building`.` (evidence: `rulesmd.ini:7361, 11726..11727, 12519..12520`; `0x0073D630`)
- `[RESOLVED] OQ-2 - Does state 3 run the empty check every frame? -> No; it runs only after the dump-rate gate passes.` (evidence: `0x0073E330..0x0073E374`)
- `[RESOLVED] OQ-3 - What happens on the last real slot drain? -> The full slot is removed, credits are applied, and `unit+0xF8` is reset to zero.` (evidence: `0x0073E374..0x0073E539`; `0x006C9680`; `0x006C96B0`)
- `[RESOLVED] OQ-4 - What happens on the first empty-slot gate? -> State 3 sets slot 8 if possible, writes substate 4, clears slot 10 if occupied, and returns 1.` (evidence: `0x0073E4B0..0x0073E545`)
- `[RESOLVED] OQ-5 - Does state 4 execute in the same switch pass after the state write? -> No; the state-3 case breaks/returns after writing `+0xBC = 4`.` (evidence: `0x0073D630` switch decompile around state 3/4)
- `[RESOLVED] OQ-6 - When is `+0x6D1` cleared? -> In state 4 cleanup, not in the state-3 empty-slot branch.` (evidence: `0x0073E0D0..0x0073E2A4`)
- `[RESOLVED] OQ-7 - Does stock GAREFN/NAREFN wait on slot 8? -> No stock delay; no active stock `ProductionAnim` is created.` (evidence: `artmd.ini:1749` commented, `artmd.ini:1763..1787`, `0x00451750`)
- `[RESOLVED] OQ-8 - Is an extra SpecialAnim-duration hold real in gamemd? -> No. Slot 10 is not a post-last-slot cooldown gate; state 4 only waits on slot 8.` (evidence: `0x0073E4B0..0x0073E545`; `0x0073E0D0..0x0073E2A4`)
- `[RESOLVED] OQ-9 - Does current Rust still seed an extra `DepositCooldown` after empty-slot detection? -> No for current source; `deposit_cooldown_ticks` is set to 0 and `Departing` is reached directly.` (evidence: `src/sim/miner/miner_dock_sequence.rs:878..882`)
- `[DEFERRED] OQ-10 - Exact rendered frame where CMON/HORV display reverts relative to mission scheduler.` (category: needs-runtime-debugger; reason: binary says `+0x6D1` clears in state 4, but pixel/frame capture was not part of this slice; next-step-if-pursued: runtime trace display type across empty gate and state-4 tick)
- `[DEFERRED] OQ-11 - Modded refinery `ProductionAnim` exact wait length and destruction order.` (category: out-of-scope; reason: stock refineries do not set the anim; next-step-if-pursued: trace slot-8 AnimClass lifecycle with a test art override)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| After a real slot drain, gamemd waits one more dump-rate interval before empty-slot state-4 transition. | `0x0073E330..0x0073E545`; storage helpers | none observed: Rust re-arms `unload_timer` after a real slot drain | `src/sim/miner/miner_dock_sequence.rs::phase_unloading` | Preserve one empty-slot gate after last real drain; do not release on the same tick as the last slot drain. | `cmin_last_real_slot_drain_requires_one_empty_dump_gate_before_departing`: full ore CMIN drains cargo on first gate, remains unloading until the next 14.4-frame gate. | Do not use `SpecialAnim` duration as this wait. |
| The empty-slot gate writes state 4 immediately and does not seed another dump/DepositCooldown interval. | `0x0073E4B0..0x0073E545` | none observed: `deposit_cooldown_ticks = 0`, `Departing` | `phase_unloading`, `RefineryDockPhase::DepositCooldown` | Empty cargo with timer <= 0 transitions directly to the stock state-4 handoff phase. | `empty_unload_gate_does_not_seed_deposit_cooldown`: after empty gate, `deposit_cooldown_ticks == 0` and next tick runs handoff. | Do not resurrect the stale extra `DepositCooldown` path for stock unload. |
| `+0x6D1` clears in state 4, not state 3; display override should remain through the empty-slot gate and clear on the state-4 handoff. | `0x0073DFDA`; `0x0073E1F6`; `UnloadingClass` INI | representation partial: Rust clears `display_type_override` in `phase_departing` | `phase_departing`, `app_instances/units.rs` display override consumer | Keep CMON/HORV until state-4 handoff tick, then clear it. | `unloading_class_override_clears_on_state4_handoff_not_empty_gate`: empty gate leaves override, following handoff clears it. | Do not clear display override at the instant cargo becomes empty unless that is also the state-4 handoff tick in Rust's model. |

## 10. Negative Facts / Do Not Do

- Do not route stock healthy `HARV/CMIN -> GAREFN/NAREFN` completion through `BuildingClass::ReleaseDockedHarvester`; that call belongs to a nonzero `unit+0x2E4` branch, not the standard zero-link stock path.
- Do not add a post-empty `DepositCooldown` equal to another dump interval. The empty-slot gate itself is the gate; state 4 is the next phase.
- Do not hold stock departure for `GAREFNOR`/`NAREFNOR` SpecialAnim length. State 4 waits on slot 8 (`ProductionAnim`), not slot 10 (`SpecialAnim`), and stock slot 8 is absent/commented.
- Do not clear `UnloadingClass`/display override on the last real slot drain; binary clear is state 4 via `+0x6D1 = 0`.
- Do not implement `ProductionAnim` wait as a generic fixed cooldown; it is pointer occupancy of building anim slot 8.

## 11. Remaining Uncertainty

- Exact rendered frame where the unloading voxel override disappears relative to the external mission scheduler was not runtime-captured. Binary evidence says the causal clear is state 4 `+0x6D1 = 0`.
- Modded refinery `ProductionAnim` duration/destruction ordering remains out-of-scope. Stock `GAREFN/NAREFN` do not exercise it.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/traces/2026-05-21-trace-chrono-miner-post-dump-exit.md`: replace "Normal dump complete - gamemd calls `BuildingClass::ReleaseDockedHarvester` from `UnitClass::Mission_Deploy_Building`. This is the standard ore-delivery exit path." with "Normal stock `HARV/CMIN -> GAREFN/NAREFN` dump completion stays in `UnitClass::Mission_Deploy_Building`: state 3 writes substate 4 on the empty-slot gate, and state 4 clears `+0x6D1`/resumes harvest scheduling. `BuildingClass::ReleaseDockedHarvester` is conditional nonzero-link behavior, not the stock zero-link exit."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/CHRONO_MINER_REFINERY_UNDOCK_TRACE.md`: replace "DepositCooldown hold: gamemd holds the miner on the pad while the building-side anim plays" with "Stock gamemd holds only until the empty-slot dump gate and then state-4 cleanup; it does not wait for `SpecialAnim` completion. The only state-4 animation guard is slot 8 `ProductionAnim`, which stock `GAREFN/NAREFN` do not define."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md`: replace "current unload completion still appears to add one extra dump-gate hold" with "Older Rust revisions appeared to add an extra post-empty hold; current `phase_unloading` transitions directly to `Departing` with `deposit_cooldown_ticks = 0` after the empty-slot gate. Re-audit queue handoff against current code before using this row."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARVESTER_DOCK_UNLOAD.md`: replace "per bale" wording around `HarvesterDumpRate` with "per StorageClass slot gate; stock pure ore cargo drains all ore in one gate, then the next gate finds no slot and writes state 4."

## Sources

- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra disassembly ranges inspected: `0x0073E330..0x0073E3F5`, `0x0073E4B0..0x0073E545`, `0x0073E0D0..0x0073E2A4`
- Ghidra decompile: `StorageClass::FindFirstNonEmptySlot @ 0x006C9820`, `StorageClass::GetAmount @ 0x006C9680`, `StorageClass::RemoveAmount @ 0x006C96B0`
- Ghidra decompile: `BuildingClass::SetAnimSlotImage @ 0x00451750`, `BuildingClass::ClearAnimSlot @ 0x00451E40`
- Ghidra decompile: `RulesClass::ReadGeneral @ 0x00670CD4`
- INI/art: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/rules/ruleset.rs`, `src/sim/miner/miner_tests.rs`
- Prior/stale docs referenced above
