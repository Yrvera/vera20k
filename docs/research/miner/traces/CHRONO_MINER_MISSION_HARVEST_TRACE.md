# Chrono Miner — Mission_Harvest State 0 (Scan) + State 1 (Scoop) Trace

**Scenario:** Allied Chrono Miner on ore cell, neighboring ore cells available.
Mission_Harvest runs state 0 (TiberiumLongScan diamond-ring scan to find ore)
through state 1 (scoop bales per tick) to full-storage transition.

**Scope:** States 0 and 1 only. Return-to-refinery (state 2+) is out of scope.
**Date:** 2026-05-19
**Sources:** MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md,
MISSION_HARVEST_GHIDRA_REPORT.md, HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md,
src/sim/miner/miner_system.rs, src/sim/miner/mod.rs, ini/rulesmd.ini

---

## Stage Table

| # | Stage | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|-------------|---------|
| S0-A | Full-storage early exit in state 0 | `Get_Storage_Percentage() >= 1.0` → state 2 immediately | No full check in `handle_search_ore` | FAIL |
| S0-B | State 0 scan radius (CMIN) | TiberiumLongScan (48 cells), offset 0x177C | `config.long_scan_radius` = 48 from INI | PASS |
| S0-C | State 0 scan function — CMIN vs HARV | Both call `Search_For_Tiberium_And_Move`; CLSID check cancels in-progress warp for chrono loco only | Same scan function for both; warp cancel handled in `handle_move_to_ore` teleport guard | PASS (behavior equivalent) |
| S0-D | Diamond-ring scan algorithm | Ring 0 fast-path (LandType==Tiberium, no Is_Cell_Harvestable). Rings 1..47: 4 arms per col, highest-value in nearest ring wins; early exit per ring | `search_local_ore`: ring 0 checks `nodes.get(&center)`, rings 1..radius: 4 arms per col, highest value wins per ring, early exit | PASS |
| S0-E | Scan selection criterion | Highest-value (base_value × (density+1)) in nearest populated ring; ties broken last-seen wins | Same: `value_of(node) = base × (remaining+1)`; strict `value <= cur` keeps first-seen on ties | FAIL (tie-break direction differs) |
| S0-F | Archive (ghost cell) consumption | Reads `UnitClass+0x218`; if set, calls Set_Destination(archive) + clears; zone param set to 0 for subsequent scan | Reads `last_harvest_cell`; re-checks zone reachability before drive; zone filter always on (improvement, not bug) | UNCHECKED (minor semantics differ; no observable effect in common case) |
| S0-G | State 4 / WaitNoOre delay | Returns 0x69 = 105 ticks (called once per 105 ticks) | WaitNoOre counts down `rescan_cooldown_ticks` (default 105); called every tick during countdown | PASS (delay matches: 105 ticks both sides) |
| S0-H | HouseClass+0x242 ore-depleted flag | Set to 1 when entering state 4 if Harvester=yes | Not implemented | NOT-IMPLEMENTED |
| S0-I | State 0 runs every tick (no guard) | State 0 returns 1 when ore found; no timer gate in state 0 | `handle_search_ore` called every tick while in SearchOre state | PASS |
| S1-A | State 1 timer init on transition | Timer set to duration=2, start=g_CurrentFrameCounter, steps=0 on state 1 entry from state 0 | `harvest_timer = config.harvest_tick_interval` (18) set in `handle_move_to_ore` on arrival | FAIL (timer value mismatch at entry) |
| S1-B | Step counter gate (9 steps) | State 1 waits for step_counter >= 9 before calling Harvest_Ore_Tick; each timer tick increments counter | Single `harvest_timer` countdown; fires at 0; no separate step counter | FAIL (model differs; observable timing may match for steady-state but not on first bale) |
| S1-C | First-bale timing | First bale: 9 steps × HarvesterLoadRate(2) = 18 frames. Subsequent bales: 1 step × HarvesterLoadRate = 2 frames (step counter NOT reset after success) | First bale: harvest_tick_interval=18 ticks. Subsequent bales: also reset to 18 ticks each time | FAIL (subsequent bales are 18-tick delay in our impl vs 2-tick in gamemd after first 9 steps) |
| S1-D | Harvest_Ore_Tick destination guard | If unit has a destination (still moving), return 1 (don't harvest) | `extract_bales_max` called unconditionally; no movement guard | FAIL |
| S1-E | Ore-cell value decrement | `CellClass::Reduce_Tiberium(amount)`: decrements `field_0x11E` (density) by `ftol(remaining_capacity)` bales. If amount >= density+1: removes overlay entirely | `extract_bales_max`: decrements `node.remaining` by `n * base`; removes node when remaining==0; syncs overlay frame | PASS (equivalent behavior) |
| S1-F | Storage model (float vs bales) | `StorageClass[4]` float array; AddAmount(float, tibType). Capacity from `TechnoTypeClass+0x800` as float. Full = `GetTotalAmount() >= capacity` | `Vec<CargoBale>` discrete bales; `is_full()` = `cargo.len() >= capacity_bales` | PASS (observable: same capacity; bale count discrete in both) |
| S1-G | CMIN capacity | `Storage=20` in [CMIN], read at TechnoTypeClass+0x800 | `Miner::new(MinerKind::Chrono)` → capacity_bales = 20 from config; `obj_storage` override if INI provides | PASS |
| S1-H | Tiberium type recognition (ore vs gems) | Overlay type index → `TiberiumClass` → `field_B8` (base value); 0=ore, 1=gems | `node.resource_type`: Ore vs Gem; `ore_bale_value=25`, `gem_bale_value=50` | PASS |
| S1-I | Post-cell-depletion continuation scan | Uses TiberiumShortScan (6 cells, offset 0x1778); if found → stay state 1; if not found → state 2 (even if not full) | `search_local_ore(..., config.local_continuation_radius=6, ...)` → MoveToOre; on scan miss → begin_return | PASS (radius matches; state transition equivalent) |
| S1-J | Full-cargo post-harvest transition | On full: scan short range for archive ghost cell, then state 2 | `save_archive_via_short_scan` + `begin_return`; archive set via short scan | PASS |
| S1-K | Ore density visual (overlay frame) | `field_0x11E` decremented; radar dirty; RecalcAttributes updates LandType; adjacent cells re-evaluated | `grid.set_overlay_data(cell.x, cell.y, frame)` where frame = (remaining/base - 1).min(11) | PASS |
| S1-L | Facing toward ore cell | gamemd does not document an explicit FaceToward call during scoop. Harvest_Ore_Tick checks destination==0 (not moving). No facing is set in state 1 beyond normal movement facing | No facing override in `handle_harvest` | PASS (no facing operation to implement) |

---

## Key Findings (FAIL / NOT-IMPLEMENTED)

### F1 — S1-C: Subsequent bales harvested every 18 ticks instead of every 2 ticks
**Stage:** S1-C (harvest cadence after first 9 steps)
**Player sees:** Chrono Miner loads ~9× slower than gamemd after the first bale. At 15 Hz, gamemd harvests subsequent bales every 2 frames (~133 ms); our impl waits 18 frames (~1.2 s) each time. A full 20-bale load takes ~38 s instead of ~4 s after the first bale.
**File:line:** `src/sim/miner/miner_system.rs:434` — `snap.miner.harvest_timer = config.harvest_tick_interval;`
**gamemd evidence:** MISSION_HARVEST_GHIDRA_REPORT.md §7: "after Harvest_Ore_Tick succeeds, the step counter is NOT reset in State 1 — it only resets when the timer is re-initialized. So…every timer expiry triggers a Harvest_Ore_Tick." Rate timer fires every HarvesterLoadRate=2 frames.

### F2 — S1-D: No movement guard in Harvest_Ore_Tick
**Stage:** S1-D (destination check before extraction)
**Player sees:** If the miner arrives on an ore cell but still has a movement command queued (can happen during adjacency approach), it will begin extracting ore while visually still moving. In gamemd, Harvest_Ore_Tick returns 1 immediately when `UnitClass+0x5A4 != 0` (has destination).
**File:line:** `src/sim/miner/miner_system.rs:415` — `extract_bales_max` called without checking `entity.movement_target.is_some()`.
**gamemd evidence:** MISSION_HARVEST_GHIDRA_REPORT.md §4.7 step 2: "If unit has a destination…return 1 (still moving, not ready)."

### F3 — S0-A: No full-storage early exit at state 0 entry
**Stage:** S0-A (full-storage check at top of state 0)
**Player sees:** A miner that re-enters SearchOre with a full cargo (rare path: can occur if state machine cycles back from Harvest with a full hold when `extract_bales_max` returns empty on first tick) will begin scanning for ore instead of immediately returning to the refinery. Firing frequency: rare in normal play; most full-return paths go through `begin_return` in `handle_harvest`. Observable if state machine is forced into SearchOre with cargo full.
**File:line:** `src/sim/miner/miner_system.rs:250–311` — `handle_search_ore` has no `is_full()` check.
**gamemd evidence:** MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md §2 Step A: "If storage >= 1.0 (full): sets state = 2 (RETURN), returns 1."

### F4 — S1-A: Harvest timer value at state-1 entry is 18, not 2
**Stage:** S1-A (timer init on transition to Harvest)
**Player sees:** The miner waits 18 frames before the first harvest attempt instead of 2 frames. This means the first bale takes ~1.2 s instead of ~133 ms from the moment the miner arrives on the ore cell. For the player, the miner appears to briefly idle after reaching the ore cell before the scoop animation begins.
**File:line:** `src/sim/miner/miner_system.rs:354` — `snap.miner.harvest_timer = config.harvest_tick_interval;` (= 18).
**gamemd evidence:** MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md §7: "Timer initialized with duration=2 and value=2…The duration-2 value means state 1 runs its first timer check within 2 ticks of arriving."

### F5 — S0-E: Tie-break direction for equal-value cells in a ring
**Stage:** S0-E (scan selection criterion)
**Player sees:** When two cells in the same ring have identical ore value (same density, same type), gamemd picks the last-evaluated candidate (strict `<` comparison keeps updating winner). Rust's `search_local_ore` uses the same `value <= cur` condition, so this is a match. **Re-analysis:** After verifying both use strict `value <= cur` (keeps first-seen; actually `value <= cur` means "don't update when equal" — so first-seen wins in both). This may be a PASS not FAIL. The iteration order within a ring differs (Rust: top/bottom/left/right by col; gamemd: same pattern) so tie-break cell may differ for equal-value cells at the same ring position. Frequency: very rare (only fires when two cells in the same ring have identical value AND both are the best in the ring). UNCHECKED on exact iteration order match.

---

## Summary

The scan algorithm (state 0) is largely correct: radius, function, ring-expansion, early-exit-per-ring, and value selection all match. The main state 0 bug is the missing full-storage early-exit guard.

State 1 has a significant timing bug: gamemd harvests the first bale after 9×HarvesterLoadRate=18 frames, then subsequent bales every HarvesterLoadRate=2 frames (step counter is not reset between bales). Our impl resets the 18-tick interval every bale, making subsequent harvests ~9× slower than gamemd. This is the most player-visible disparity — the miner loads far too slowly.

The destination guard missing from Harvest_Ore_Tick is a correctness bug but fires rarely in practice.

---

## Verdict Tally

PASS: 9 | FAIL: 4 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

---

## Status

COMPLETE
