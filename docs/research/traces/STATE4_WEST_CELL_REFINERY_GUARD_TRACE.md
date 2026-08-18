# State 4 West-Cell Refinery Guard Trace

Scenario: `UnitClass::Mission_Deploy_Building` state 4 completion after stock harvester/refinery unload, with two concrete west-cell cases:

- Miner current cell `(13,11)`, west cell `(12,11)` contains a live `Refinery=yes` building.
- Miner current cell `(13,11)`, west cell `(12,11)` contains a live non-refinery building.

Scope guard: this trace covers only state-4 completion identity: west-cell rediscovery, `Refinery=yes` guard ownership, and Rust `reserved_refinery` cleanup identity. It does not trace radio `0x16`, far-return fallback search, teleport lifecycle, state-3 credits, or two-miner queue takeover timing.

Ghidra status: Ghidra MCP was queried read-only, but no running Ghidra instance was available (`list_instances` returned no instances). Gamemd evidence below uses existing verified read-only Ghidra reports that cite decompile/disassembly addresses and mark the path active in standard YR.

## Verdict

Overall: PARTIAL.

The recent west-cell rediscovery shape is correct for live west-cell refinery and live west-cell non-refinery identity: Rust looks at miner current cell plus `(-1,0)`, does not use `reserved_refinery` as the state-4 building identity, and releases reservation/contact bookkeeping against `reserved_refinery`.

The remaining state-4 parity gap is the live `Refinery=yes && building+0x57C != 0` wait branch. Gamemd waits before clearing the unload byte while a west-cell refinery's slot-8 `ProductionAnim` object is non-null. Rust computes `state4_refinery_wait_live` and discards it, and there is no sim-side building slot-8 occupancy state for this guard. Stock `GAREFN/NAREFN` normally avoid the visible delay because stock art has no active `ProductionAnim`, but the active mechanism is not implemented for modded/nonstandard refineries.

Verdict tally: PASS: 4 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Pipeline

1. Trigger: state 3 empty-storage gate writes state 4 / Rust `Departing`.
2. Identity: state 4 rediscovers west-cell building from miner current cell plus `(-1,0)`.
3. Guard: caller applies `Refinery=yes` to the rediscovered building before checking slot-8 wait state.
4. Completion: if no wait, clear unload display/state and return to harvest/search scheduling.
5. Cleanup: release Rust reservation/contact using `reserved_refinery`, not the rediscovered west-cell building.

## Stage Results

### Stage 1 - Active YR state-4 path

Gamemd: verified active for standard `HARV/CMIN -> GAREFN/NAREFN`; stock refineries have `DockUnload=yes` and `Refinery=yes`, and stock miners have `Harvester=yes`. State 3 empty-slot writes substate 4 and returns before state 4 runs.

Rust: current `phase_unloading` transitions empty cargo to `RefineryDockPhase::Departing` and schedules the next handoff instead of clearing display in the empty branch.

Verdict: PASS for reaching the state-4/departing handoff boundary in this scenario.

Evidence: `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`; Rust `src/sim/miner/miner_dock_sequence.rs:1108..1117`.

### Stage 2 - West-cell rediscovery identity

Gamemd output: state 4 finds the building from current cell plus signed `(-1,0)`, then calls `Look_up_building_in_cell`. The helper scans the cell object list and returns the first building object.

Rust output for both concrete live cases: `mission_deploy_unload_building` reads the miner position, computes `lookup_rx = miner.rx - 1`, `lookup_ry = miner.ry`, iterates the same occupancy layer, and returns the first live structure id. With miner at `(13,11)`, Rust looks at `(12,11)`.

Verdict: PASS for the concrete live west-cell refinery/non-refinery cases.

Evidence: gamemd `0x0073E181`, `0x0073E2C8`, `0x0049F2F0`, `0x0047C520`; report `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`; Rust `src/sim/miner/miner_dock_sequence.rs:432..456`.

Adjacent finding: Rust filters out dying/dead structures inside this helper; the verified helper summary says `Look_up_building_in_cell` itself only checks building type. Dead/dying first-building behavior is outside this concrete live-building scenario and remains UNCHECKED here.

### Stage 3 - `Refinery=yes` caller guard

Gamemd output: state 4 applies `building->Type+0x16BB` (`Refinery=yes`) after the west-cell lookup. If the west-cell building is not a refinery, the slot-8 wait guard is skipped and normal cleanup continues.

Rust output: `phase_departing` calls `mission_deploy_unload_building`, then computes `structure_has_refinery_yes(sim, rules, building_id)`. With west-cell `GAREFN`, this is true. With west-cell `GAPOWR`, this is false.

Verdict: PASS for locating the guard at the caller boundary and not treating `reserved_refinery` as the guarded building identity.

Evidence: gamemd `0x0073E1CF..0x0073E1EA`; `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_GHIDRA_REPORT.md`; Rust `src/sim/miner/miner_dock_sequence.rs:1141..1144`, `src/sim/miner/miner_dock_sequence.rs:458..462`.

### Stage 4 - Slot-8 wait effect

Gamemd output: if the rediscovered west-cell building exists, is `Refinery=yes`, and `building+0x57C` is non-null, state 4 direct-returns `1` and does not clear the unit unload/deploy byte that tick. `building+0x57C` is `Anims_0[8]`, the live `ProductionAnim` pointer.

Rust output: `state4_refinery_wait_live` is assigned and immediately discarded. No sim-side building `Anims_0[8]`/ProductionAnim occupancy exists for state-4 wait, and `phase_departing` clears the unload override and finishes in the same call.

Verdict: NOT-IMPLEMENTED for refineries whose slot-8 `ProductionAnim` is live. For stock `GAREFN/NAREFN`, the player-visible delay is normally absent because stock art leaves `ProductionAnim` inactive, so the stock no-wait output is effectively the same.

Evidence: gamemd `0x0073E1CB..0x0073E1F6`; `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_GHIDRA_REPORT.md`; stock art `ini/artmd.ini:[GAREFN]` no active `ProductionAnim`, `[NAREFN] ProductionAnim=NAREFN_AR` commented; Rust `src/sim/miner/miner_dock_sequence.rs:1141..1174`; search found no sim-side `ProductionAnim` state except render/art parsing.

### Stage 5 - Normal completion and display clear

Gamemd output: after the state-4 wait guard passes, state 4 clears `unit+0x6D1`, sets mission Harvest `0x0A`, optionally sends radio `3`, queues mission, and reaches timer epilogue. It does not call `ReleaseDockedHarvester` or force track `0x47` on normal stock zero-link completion.

Rust output: `phase_departing` clears `display_type_override`, movement/track fields, unload cluster state, target/exit cache, then sets `dock_phase=Approach` and `state=SearchOre`.

Verdict: PASS for stock no-wait visible completion and no `0x47` release-helper path. UNCHECKED for byte-exact mission return value/timer epilogue and optional radio `3` equivalence; those were not recomputed in this slot and two-miner queue timing is adjacent.

Evidence: gamemd `0x0073E1F6`, `0x0073E24F..0x0073E2BE`, `0x0073D66D`; report `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`; Rust `src/sim/miner/miner_dock_sequence.rs:1158..1184`.

### Stage 6 - `reserved_refinery` cleanup identity

Gamemd output: state-4 building identity comes from the west-cell lookup, not a cached reserved refinery. Normal stock completion is the zero-link path, not `ReleaseDockedHarvester`.

Rust output: west-cell identity is used only for `mission_deploy_unload_building` / `structure_has_refinery_yes`; contact and pad cleanup use the passed `ref_sid` from `reserved_refinery`: `release_on_pad(ref_sid, miner)` and `release_contact(ref_sid, miner)`.

Verdict: PASS for the requested Rust ownership split: rediscovered west-cell building for state-4 identity, `reserved_refinery` for reservation/contact cleanup.

Evidence: gamemd west-cell lookup `0x0073E2C8`, normal stock release-helper exclusion `0x0073D66D`; Rust `src/sim/miner/miner_dock_sequence.rs:670..676`, `src/sim/miner/miner_dock_sequence.rs:1141..1156`; focused test `src/sim/miner/miner_tests.rs:4807..4837`.

## Player-Visible Findings

1. NOT-IMPLEMENTED - Stage 4: modded/nonstandard refinery with live slot-8 `ProductionAnim` will let the miner leave/clear unload display immediately instead of waiting while the refinery production anim exists; Rust `src/sim/miner/miner_dock_sequence.rs:1141..1174`; gamemd `0x0073E1CB..0x0073E1F6` and `BuildingClass+0x57C == Anims_0[8]`.

No FAIL findings in the concrete live west-cell refinery/non-refinery stock cases.

## Adjacent Findings

- Rust's `mission_deploy_unload_building` filters out dying/dead structures, while the verified `Look_up_building_in_cell` helper summary does not include that filter. This trace did not test a dead/dying first building in the west cell.
- Exact queue takeover timing after `phase_departing` remains adjacent. Existing reports explicitly defer two-miner handoff timing.
- Exact runtime frame count for a modded `ProductionAnim` wait remains UNCHECKED because no live Ghidra/runtime trace was available.

## Sources

- `docs/research/miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`
- `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`
- `docs/research/miner/BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_tests.rs`
