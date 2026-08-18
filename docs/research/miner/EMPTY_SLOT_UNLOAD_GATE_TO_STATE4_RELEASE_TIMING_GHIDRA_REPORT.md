# Empty-Slot Unload Gate To State-4 Release Timing - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR stock `CMIN`/`HARV` unloading at stock `GAREFN`/`NAREFN`, from the last cargo/storage slot deposit through the next empty-slot dump gate, state 3 -> state 4 write, and first possible stock state-4 release.  
**Non-Scope:** two-miner queue handoff order, close/far chrono return selection, destroyed/sold refinery runtime rendering, `ReleaseDockedHarvester` nonzero-link visuals, slave miner/Yuri refinery, modded refineries with active `ProductionAnim`.  
**Confidence:** High for binary timing and current Rust delta in this slice. Medium only for exact external rendered-frame presentation because no live runtime capture was taken.  
**Active in YR:** Yes. Stock `CMIN/HARV` have `Harvester=yes` and dock lists including `GAREFN/NAREFN`; stock `GAREFN/NAREFN` have `DockUnload=yes`/`Refinery=yes`.

## 1. Overview

The last real dump gate does not release the miner. It drains the first non-empty `StorageClass` slot, awards credits to the refinery owner, resets the unit dump accumulator at `UnitClass+0xF8`, and returns from state 3.

Only after the next dump-rate gate does state 3 observe `StorageClass::FindFirstNonEmptySlot == -1`. That empty-slot gate requests building anim slot 8 if the building type is a refinery, writes `UnitClass+0xBC = 4`, clears slot 10 if still active, and returns `1`. State 4 runs on a later mission call; for stock `GAREFN/NAREFN`, `BuildingClass+0x57C`/ProductionAnim slot 8 is empty, so the first state-4 pass can clear `UnitClass+0x6D1` and schedule Harvest without any extra DepositCooldown or SpecialAnim hold.

## 2. Key Offsets / Fields

| Object | Offset | Meaning | Evidence |
|---|---:|---|---|
| `UnitClass` | `+0xBC` | unload mission substate; `3` deposit loop, `4` depart handoff | `0x0073D630` decompile, state switch |
| `UnitClass` | `+0xF8` | dump accumulator compared with `HarvesterDumpRate * 900.0` | `0x0073E330..0x0073E545` decompile/disassembly range inspected |
| `UnitClass` | `+0x6D1` | unload-active/display flag; set before state 3, cleared in state 4 | `0x0073DFBD..0x0073E09A`, `0x0073E0D0..0x0073E2A4` |
| `UnitClass` | `+0x33C` | `StorageClass`, four float tiberium slots | `StorageClass` helpers at `0x006C9820`, `0x006C9680`, `0x006C96B0` |
| `BuildingClass` | `+0x57C` | anim slot 8 pointer (`ProductionAnim`) checked by state 4 | `0x0073E0D0..0x0073E2A4`; `BuildingClass::SetAnimSlotImage @ 0x00451750` |
| `BuildingClass` | `+0x584` | anim slot 10 pointer (`SpecialAnim`) cleared on completion | `0x0073E3B0..0x0073E53A`; `BuildingClass::ClearAnimSlot @ 0x00451E40` |
| `BuildingTypeClass` | `+0x16BB` | `Refinery=yes` gate for slot-8 completion anim and state-4 wait guard | `0x0073D630`; stock INI |
| `RulesClass` | `+0x1528` | `HarvesterDumpRate` double | state-3 gate in `0x0073D630` |

## 3. Core Timing Logic

### State 3 dump gate

Verified state-3 gate:

- The branch compares `RulesClass+0x1528 * 900.0` against `UnitClass+0xF8`.
- Stock default is `0.016 * 900.0 = 14.4` frames.
- If the accumulator is below the threshold, state 3 returns without storage inspection.

### Last real slot deposit

When the gate passes and a slot exists:

1. The building particle/emitter vtable call runs.
2. If `BuildingClass+0x584 == 0`, `BuildingClass::SetAnimSlotImage(slot=10, ...)` is requested.
3. `StorageClass::FindFirstNonEmptySlot` scans slots `0..3` and returns the first slot whose float is greater than `0.0`.
4. `StorageClass::GetAmount(slot)` returns the whole float amount in that slot.
5. `StorageClass::RemoveAmount(amount, slot)` subtracts the whole amount and saturates to zero if needed.
6. Credits are added using the refinery/building owner, plus purifier bonus where applicable.
7. `UnitClass+0xF8` is reset to zero.
8. State 3 returns `1`.

This means a pure ore stock cargo pays one dump interval for the real ore slot, then another dump interval before empty-slot detection can occur. Mixed ore/gem cargo drains slot 0 first, then slot 1 at the next dump gate, then detects empty at the following dump gate.

### Empty-slot gate

On the first dump-rate gate after all slots are zero:

1. `FindFirstNonEmptySlot` returns `-1`.
2. If `BuildingTypeClass+0x16BB` (`Refinery=yes`) is true, state 3 requests `SetAnimSlotImage(slot=8, ...)`.
3. It writes `UnitClass+0xBC = 4`.
4. If `BuildingClass+0x584` slot 10 is non-null, it calls `ClearAnimSlot(slot=10)`.
5. It returns `1`.

No new dump interval is seeded here. The empty-slot gate is itself the delayed check after the last real slot.

### State 4 release

State 4 re-locates the refinery and checks:

`building exists && building.Type.Refinery && building+0x57C != 0`

If true, it returns `1` and waits. For stock `GAREFN/NAREFN`, this is normally false because neither has active `ProductionAnim`; `NAREFN` only has commented `;ProductionAnim=NAREFN_AR`, and `GAREFN` has none. Therefore the first stock state-4 pass can clear `UnitClass+0x6D1 = 0`, set/schedule mission `0x0A` (`Harvest`), optionally radio clear `3`, and exit through the mission timer path.

## 4. INI / Art Proof

| Key | Stock value | Effect |
|---|---|---|
| `[General] HarvesterDumpRate` | absent in `rulesmd.ini`/`rules.ini`, binary/Rust default `0.016` | dump gate threshold `14.4` frames |
| `[CMIN] Dock` | `NAREFN,GAREFN` | stock chrono miner can unload at both refineries |
| `[CMIN] Storage` | `20` | chrono miner cargo capacity |
| `[HARV] Dock` | `NAREFN,GAREFN` | stock war miner can unload at both refineries |
| `[HARV] Storage` | `40` | war miner cargo capacity |
| `[GAREFN] DockUnload` | `yes` | stock unload target |
| `[NAREFN] DockUnload` | `yes` | stock unload target |
| `[GAREFN] SpecialAnim` | `GAREFNOR` | slot 10 per real dump gate |
| `[NAREFN] SpecialAnim` | `NAREFNOR` | slot 10 per real dump gate |
| `[GAREFN] ProductionAnim` | absent | no stock state-4 slot-8 wait |
| `[NAREFN] ProductionAnim` | commented `;ProductionAnim=NAREFN_AR` | no stock state-4 slot-8 wait |

## 5. Current Rust Implementation Status

Current Rust matches the critical no-extra-hold behavior for new stock unloads:

- `src/sim/miner/miner_dock_sequence.rs:798` decrements `unload_timer` before any slot/empty check.
- `src/sim/miner/miner_dock_sequence.rs:815` drains one resource-type slot atomically.
- `src/sim/miner/miner_dock_sequence.rs:868` re-arms `unload_timer` after a real slot drain.
- `src/sim/miner/miner_dock_sequence.rs:875` treats empty cargo at a dump-gate crossing as the state-3 empty-slot branch.
- `src/sim/miner/miner_dock_sequence.rs:881` sets `deposit_cooldown_ticks = 0`.
- `src/sim/miner/miner_dock_sequence.rs:882` transitions directly to `RefineryDockPhase::Departing`.
- `src/sim/miner/miner_dock_sequence.rs:919` clears `display_type_override` during the departing/state-4 handoff.
- `src/sim/miner/miner_tests.rs:4529` has `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`, pinning empty gate -> Departing -> next tick release.

Representation gaps remain: Rust does not model a literal `UnitClass+0x6D1` byte, literal state-4 mission return value, or a `BuildingClass+0x57C`/slot-8 wait. For stock `GAREFN/NAREFN`, the missing slot-8 wait has no visible effect because the stock art does not define `ProductionAnim`.

## 6. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| stock zero-link state-3/state-4 unload path | verified | decompile `0x0073D630`; prior reachability docs | none for this slice |
| dump gate threshold | verified | `0x0073E330..0x0073E545` decompile/disassembly range inspected | none |
| last real slot drain | verified | `0x0073D630`; helpers `0x006C9820`, `0x006C9680`, `0x006C96B0` | none |
| empty-slot state-3 -> state-4 write | verified | `0x0073E3B0..0x0073E53A` decompile/disassembly range inspected | none |
| state-4 `+0x57C` wait guard | verified for stock no-delay condition | `0x0073E0D0..0x0073E2A4`; `artmd.ini` | modded `ProductionAnim` duration out-of-scope |
| `+0x6D1` clear | verified | `0x0073DFBD..0x0073E09A`; `0x0073E0D0..0x0073E2A4` | exact rendered-frame capture deferred |
| current Rust empty-slot behavior | verified by source scan and Codegraph | `phase_unloading`; `empty_unload_gate_releases_dock_on_next_stock_state4_handoff` | run tests during implementation branch if touched |
| two-miner takeover after release | deferred | separate swarm slot | queue handoff frame order |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this path active in standard YR? -> Yes, stock `CMIN/HARV` use `Harvester=yes` and `Dock=NAREFN,GAREFN`; stock `GAREFN/NAREFN` use `DockUnload=yes`/`Refinery=yes`.` (evidence: `rulesmd.ini`; `0x0073D630`)
- `[RESOLVED] OQ-2 - Does state 3 inspect storage every frame? -> No, storage is inspected only after `HarvesterDumpRate * 900.0 <= UnitClass+0xF8`.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-3 - What happens on the last real slot deposit? -> Whole slot is removed, credits/bonus are awarded, and `UnitClass+0xF8` is reset to zero.` (evidence: `0x0073D630`; `0x006C9680`; `0x006C96B0`)
- `[RESOLVED] OQ-4 - Is there an immediate release on the same tick as the last real slot drain? -> No. Because the accumulator resets, empty-slot detection waits until the next dump-rate gate.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-5 - What writes state 4? -> The empty-slot gate after `FindFirstNonEmptySlot == -1` writes `UnitClass+0xBC = 4`.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-6 - Does the empty-slot gate seed a post-empty DepositCooldown? -> No. It writes state 4 and returns; the wait was already paid before the empty check.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-7 - Does state 4 wait on SpecialAnim slot 10? -> No. It waits only on slot 8 pointer `BuildingClass+0x57C`; slot 10 is cleared by the empty-slot branch.` (evidence: `0x0073D630`; `0x00451E40`)
- `[RESOLVED] OQ-8 - Do stock `GAREFN/NAREFN` exercise the slot-8 wait? -> No active stock `ProductionAnim`, so no stock slot-8 wait.` (evidence: `artmd.ini`)
- `[RESOLVED] OQ-9 - Does current Rust still double-hold after empty-slot detection? -> No for current source; empty gate sets `deposit_cooldown_ticks = 0` and `Departing`.` (evidence: `src/sim/miner/miner_dock_sequence.rs:875..882`; Codegraph `phase_unloading`)
- `[DEFERRED] OQ-10 - Exact pixel frame when `CMON/HORV` display reverts relative to scheduler.` (category: needs-runtime-debugger; reason: binary causal clear is state 4 `+0x6D1 = 0`, but no frame capture was taken; next-step-if-pursued: runtime trace display type across empty gate and following state-4 call)
- `[DEFERRED] OQ-11 - Modded refinery `ProductionAnim` wait duration and destruction order.` (category: out-of-scope; reason: stock refineries do not define it; next-step-if-pursued: art override with active slot-8 anim and trace AnimClass lifetime)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Last real slot drain resets the dump accumulator and does not release the miner. | `0x0073D630`; `0x006C9680`; `0x006C96B0` | none observed | `src/sim/miner/miner_dock_sequence.rs::phase_unloading` | Preserve one later empty-slot gate after the last real slot drain. | `cmin_last_real_slot_drain_requires_empty_dump_gate_before_state4` | Do not release on the same tick that the last non-empty slot drains. |
| Empty-slot gate writes state 4 and does not seed another dump interval or DepositCooldown. | `0x0073D630`, empty-slot branch; disassembly range `0x0073E3B0..0x0073E53A` inspected | none observed; `deposit_cooldown_ticks = 0` | `phase_unloading`; `RefineryDockPhase::Departing` | Empty cargo with timer <= 0 advances directly to the state-4 handoff phase. | existing `empty_unload_gate_releases_dock_on_next_stock_state4_handoff` | Do not add a post-empty `DepositCooldown`. |
| State 4 clears unload-active display after the empty-slot gate; stock refineries do not wait on slot 8. | `0x0073E0D0..0x0073E2A4`; `artmd.ini` no active refinery `ProductionAnim` | partial representation only; clears `display_type_override` in departing | `phase_departing`; renderer display override consumer | Keep unloading display through state 3; clear on state-4 handoff. | `unloading_class_override_clears_on_state4_handoff_not_last_slot` | Do not clear `UnloadingClass` at last real slot drain or at cargo becoming empty before state-4 handoff. |

## 9. Negative Facts / Do Not Do

- Do not add an extra post-empty `DepositCooldown` or another `HarvesterDumpRate` wait after the empty-slot gate.
- Do not hold stock `GAREFN/NAREFN` release for `GAREFNOR`/`NAREFNOR` `SpecialAnim`; state 4 checks slot 8, not slot 10.
- Do not call the conditional nonzero-link `ReleaseDockedHarvester` path for stock healthy zero-link `CMIN/HARV -> GAREFN/NAREFN` unload completion.
- Do not drain cargo per bale. The binary drains one whole storage slot per gate.
- Do not credit the harvester controller; credits are awarded through the refinery/building owner path.

## 10. Stale Docs / Follow-Up Wording

- In any trace still saying "Rust adds one extra post-empty dump interval", replace with: "Current Rust transitions from empty cargo at a dump-gate crossing directly to `Departing` with `deposit_cooldown_ticks = 0`; re-audit queue handoff separately if takeover frame order matters."
- In any doc saying "release waits for SpecialAnim completion", replace with: "Stock state 4 waits only on slot 8 `ProductionAnim` (`BuildingClass+0x57C`), and stock `GAREFN/NAREFN` do not define an active `ProductionAnim`; slot 10 `SpecialAnim` is cleared at the empty-slot transition."

## Sources

- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra decompile: `StorageClass::FindFirstNonEmptySlot @ 0x006C9820`
- Ghidra decompile: `StorageClass::GetAmount @ 0x006C9680`
- Ghidra decompile: `StorageClass::RemoveAmount @ 0x006C96B0`
- Ghidra decompile: `BuildingClass::SetAnimSlotImage @ 0x00451750`
- Ghidra decompile: `BuildingClass::ClearAnimSlot @ 0x00451E40`
- Ghidra disassembly ranges inspected read-only: `0x0073E330..0x0073E545`, `0x0073E0D0..0x0073E2A4`, `0x0073DFBD..0x0073E09A`, `0x0073E3B0..0x0073E53A`
- Prior report used as baseline: `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`
- INI/art: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`, `src/rules/ruleset.rs`, `src/sim/game_entity.rs`

Status: COMPLETE
