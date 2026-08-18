# Chrono Miner Refinery Ore Dump / Deposit / Release Trace

**Scenario:** Standard YR Allied `CMIN`, full of Riparius ore, already accepted/docked at same-owner stock `GAREFN`; human owner; no real Ore Purifier; no AI virtual purifier bonus.
**Date:** 2026-05-23
**Scope:** Docked unload only: dump gate, slot drain, credits, visible refinery effects, empty-cargo handoff, dock release.
**Out of scope:** Ore acquisition, refinery approach, chrono return distance decision, modded refinery anims, score/stat counters, unrelated sidebar credit-counter tuning.

## Verdict Tally

PASS: 12 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Sources Read

- `miner/traces/CHRONO_MINER_ORE_DUMP_DEPOSIT_TRACE.md`
- `miner/ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md`
- `miner/REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md`
- `miner/HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`
- Current Rust: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/app_building_anim.rs`, `src/app_instances/shp.rs`, `src/app_sim_tick.rs`, `src/app_sidebar_render.rs`, `src/rules/ruleset.rs`, `src/sim/miner/miner_tests.rs`

No live Ghidra mutation was performed. gamemd facts below come from the existing direct-decompile reports. Active-YR status was checked from those reports plus stock INI data: standard `CMIN` uses the Allied/Soviet `UnitClass::Mission_Deploy_Building` state-3 path, not the TS Weeder branch and not the Yuri Slave Miner `BuildingClass::DepositOreFromStorage` path.

## Pipeline

`CMIN dock dump-init` -> `HarvesterDumpRate gate` -> `FindFirstNonEmptySlot` -> `GetAmount(slot 0)` -> `RemoveAmount(full slot)` -> `HouseClass::Add_Tiberium_Credits` -> optional purifier bonus -> `GAREFNOR`/smoke event -> next dump gate finds empty -> state 4 / Departing -> release dock reservation -> SearchOre/WaitNoOre.

## Stage Results

| # | Stage | gamemd output for this scenario | Current Rust output | Verdict |
|---:|---|---|---|---|
| 1 | Stock data | `CMIN Storage=20`, `UnloadingClass=CMON`; `GAREFN Refinery=yes`, `Storage=200`; Riparius `Value=25`; `HarvesterDumpRate=0.016` default -> `14.4` frames. | Defaults/parsed rules match: `chrono_miner_capacity=20`, ore value `25`, dump tenths `144`. | PASS |
| 2 | Active route | `UnitClass::Mission_Deploy_Building` state 3 drains standard Allied/Soviet harvester storage. TS Weeder path is gated off for stock `CMIN`. | `phase_unloading` handles docked miner cargo; no refinery storage drain path is used. | PASS |
| 3 | Unloading visual window | gamemd sets the unload-active render gate at dump-init before first drain and clears it in state 4. | Rust sets `display_type_override=CMON` in `phase_linked` and clears it in `phase_departing`. | UNCHECKED |
| 4 | First drain timing | Dump gate threshold is `0.016 * 900 = 14.4`; first integer frame fire is frame 15 after dump-init. | `unload_timer=(144-10)` then drains on the 15th unloading tick; tests pin this behavior. | PASS |
| 5 | Slot selection / grain | Slot 0 Riparius is first non-empty. `GetAmount(0)=20.0`; `RemoveAmount(20.0,0)` drains the whole slot in one gate fire. | `SLOT_ORDER=[Ore,Gem]`; `retain` removes all ore bales in one fire. Full `CMIN`: `20` ore bales removed. | PASS |
| 6 | Spendable credit value | `Balance += int(25 * IncomeMult(1.0) * 20.0) = +500`. | `slot_value=sum(20 * 25)=500`; refinery owner credits get `+500`. | PASS |
| 7 | Bonus credits | Human owner, no real purifiers -> effective purifier count `0`; bonus `0`. | `effective_purifier_count` returns `0`; no bonus add. | PASS |
| 8 | Cargo state after drain | Unit StorageClass slot 0 becomes `0.0`; next `FindFirstNonEmptySlot` returns `-1`. | `miner.cargo` is empty immediately after the ore-slot retain pass. | PASS |
| 9 | Refinery SpecialAnim | Stock `GAREFN` has `SpecialAnim=GAREFNOR`; gamemd fires slot 10 once for the successful slot drain when the slot is not already playing. | One `BaleDepositEvent` is emitted for the slot drain; `consume_bale_events` creates/resets the SpecialAnim overlay. | PASS |
| 10 | Smoke burst | `GAREFN` has two non-zero smoke offsets; gamemd spawns `SmallGreySSys` at those two offsets for the drain event. | `consume_bale_events` skips zero offsets and spawns at each configured non-zero offset. | PASS |
| 11 | SpecialAnim/smoke ordering | gamemd performs the visible side-effect block inside the same state-3 gate fire, before/around credit drain in the function body. | Rust emits the event after credit add, then app code consumes it after fixed sim advance and before the render-frame anim tick. | UNCHECKED |
| 12 | Refinery ore pile tier | Standard Allied/Soviet dump does not write refinery StorageClass, so stock `GAREFN` tier stays `0`; only `GAREFNL1` should render. | `shp.rs` suppresses non-primary refinery looped ActiveAnim slots, so only the primary tier renders for stock refineries. | PASS |
| 13 | Stock slot 7/8 output | gamemd calls slot 7 at arrival and slot 8 at empty, but stock `GAREFN` defines no `PreProductionAnim` or `ProductionAnim`: visible output count `0`. | Rust does not emit slot 7/8 equivalents; stock visible output is also `0`. | PASS |
| 14 | Empty-slot release timing | After the successful drain, the counter resets; the next gate fire about 15 frames later sees `-1` and enters state 4. | After drain, timer carries fractional remainder and the empty-slot fire transitions directly to `Departing` after the next interval. | PASS |
| 15 | Dock release / cleanup | State 4 clears unload-active display, releases dock bookkeeping, and returns the miner to harvesting/search behavior. | `phase_departing` clears `display_type_override`, releases reservations, clears dock fields, and sets `SearchOre`. | PASS |
| 16 | Sidebar credit display / tick sound | gamemd `CreditsClass::AI` animates displayed credits toward actual credits and may play tick-up sound while the counter changes. | Rust smooths displayed credits with the same clamp range but exact delay/sound parity was not computed in this trace. | UNCHECKED |

## Current Rust Evidence

- `src/sim/miner/miner_dock_sequence.rs:798` decrements the unload timer before allowing a drain.
- `src/sim/miner/miner_dock_sequence.rs:809` defines ore-before-gems slot order.
- `src/sim/miner/miner_dock_sequence.rs:817` removes every bale in the selected slot in one pass.
- `src/sim/miner/miner_dock_sequence.rs:840` credits the refinery owner with the full slot value.
- `src/sim/miner/miner_dock_sequence.rs:848` uses count-based real/virtual purifier logic.
- `src/sim/miner/miner_dock_sequence.rs:863` emits one `BaleDepositEvent` per slot drain.
- `src/sim/miner/miner_dock_sequence.rs:880` moves to `Departing` at the empty-slot gate.
- `src/sim/miner/miner_dock_sequence.rs:919` clears the unloading display override during departure.
- `src/sim/miner/miner_system.rs:1426` computes real + AI virtual purifier count.
- `src/app_building_anim.rs:341` consumes bale events into SpecialAnim and smoke particles.
- `src/app_instances/shp.rs:545` suppresses non-primary refinery ActiveAnim tier slots for stock refinery rendering.
- `src/app_sim_tick.rs:176` consumes bale events before the render-frame animation tick.
- `src/app_sidebar_render.rs:60` drives displayed sidebar credits from actual owner credits.

## Computed Scenario Values

- Cargo: `20` Riparius bales.
- Base credit delta: `20 * 25 = 500`.
- Human/no-purifier bonus: `500 * 0 * 25 / 100 = 0`.
- Total spendable credits: `+500`.
- Deposit events: `1` event for the one non-empty Riparius slot.
- Smoke bursts: `2` stock `GAREFN` non-zero offsets.
- Cargo after first drain: `0` bales.
- Release gate: next dump threshold after the drain, approximately `15` game frames.

## Failures

None confirmed for the concrete stock CMIN/GAREFN dump/deposit/release slice.

## Adjacent Findings

- Modded refineries that define `PreProductionAnim` or `ProductionAnim` would still need explicit slot 7/8 support; stock `GAREFN` output is unaffected because both are absent.
- The gamemd harvested-stat field (`HarvestedCredits += amount * 5`) is not modeled as a separate Rust score/stat counter in this trace. Spendable credits are correct for the requested visible deposit value.
- Exact sidebar credit tick sound and the direction-delay detail from `CreditsClass::AI` need a separate sidebar-credit trace if UI/audio parity is being closed.
- Exact CMON swap-on frame relative to Rust `Linked`/`Pivoting` vs gamemd dump-init remains a runtime-frame mapping question; this trace only confirms it is active before the first drain and cleared on departure.

## Status

COMPLETE for the requested docked stock CMIN/GAREFN unload/deposit/release scenario, with three explicitly marked UNCHECKED timing/UI details where literal frame equality was not computed.
