# Engineer Low Bridge Repair Mutation Trace

**Scenario:** Engineer enters `CABHUT`; nearby low wooden bridge cells are `Destroyed`.

**Scope lock:** This trace covers only the low-bridge repair mutation path. C4 hut destruction, high bridge repair, low bridge pathing/tubes after repair, and multi-engineer ordering are adjacent findings only.

**Ghidra access:** No Ghidra MCP tools were exposed in this subagent session. Binary-side facts below are taken from existing verified Ghidra reports:

- `TECH_CABHUT_GHIDRA_REPORT.md`
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md`
- `REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md`
- `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`

## Pipeline

`Engineer target CABHUT` -> `CABHUT BridgeRepairHut gate` -> `5x5 bridge scan` -> `low-vs-high dispatch` -> `RepairBridge_Low` -> `NS/EW low walker` -> `overlay/radar/zone side effects` -> `engineer consumed`

## Stage Results

| Stage | Our output | gamemd output | Verdict |
|---|---|---|---|
| INI gate | `CABHUT` parses `BridgeRepairHut=yes`; engineer path checks target type flag in `src/rules/object_type.rs` and `src/sim/world/world_orders.rs`. | `CABHUT` has `BridgeRepairHut=yes`; `BuildingTypeClass+0x16B6` opens the repair branch. Active in standard YR per `TECH_CABHUT_GHIDRA_REPORT.md`. | PASS |
| Trigger geometry | `tick_bridge_repair_orders` fires when engineer is Chebyshev-1 adjacent to CABHUT and uses the engineer cell as the 5x5 scan center (`world_orders.rs:307`, `world_orders.rs:340`). | `InfantryClass::PerCellProcess` fires when the infantry steps into the target building cell and scans a 5x5 region centered on the hut/building coord (`TECH_CABHUT_GHIDRA_REPORT.md` §4.2, §4.4). | FAIL |
| Low-vs-high dispatch for destroyed low overlay | Low is selected if any scan cell has overlay `0x4A..=0x65` or `is_wood_bridge_repair_tile`; destroyed low NS/EW overlays are `0x64`/`0x65` (`walker.rs:71`, `walker.rs:429`). | Low is selected if the hut-centered scan sees low bridge tile range or overlay `0x4A..0x65`; destroyed low NS/EW overlays are `0x64`/`0x65` (`TECH_CABHUT_GHIDRA_REPORT.md` §4.2; `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.4). | PASS |
| Low walker strip selection | For low NS, Rust walks across low overlays and writes `(x,y)`, `(x,y-1)`, `(x,y+1)`; for low EW, writes `(x,y)`, `(x-1,y)`, `(x+1,y)` (`walker.rs:159`, `walker.rs:198`, `walker.rs:315`). | Verified low walkers write the walker cell plus two perpendicular side cells; NS_Low and EW_Low have single callers from `RepairBridge_Low` (`REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` §1-§3). | PASS |
| Destroyed low overlay repair value | Rust maps `0x64 -> 0x4A + rng.next_range_u32(4)` and `0x65 -> 0x53 + rng.next_range_u32(4)` (`walker.rs:388`, `walker.rs:394`, `walker.rs:415`; RNG is xorshift64* modulo in `rng.rs:28`, `rng.rs:45`). | gamemd maps `0x64 -> 0x4A + FUN_00598030(0,3)` and `0x65 -> 0x53 + FUN_00598030(0,3)`; `FUN_00598030` is a `Random__Next` rejection-loop, not modulo xorshift (`REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` §2; `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.5). | FAIL |
| Damage/state byte after repair | Rust sets all touched cells to `DamageState::Healthy { variant }` (`walker.rs:337`, `walker.rs:342`, `walker.rs:353`). | gamemd repair walkers write only `Cell+0x44` overlay; no direct `+0x11E` write, and follow-up refutes RecalcAttributes resetting `+0x11E` for bridge overlays. The stale damage byte remains (`REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` §0, §5; `REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md`). | FAIL |
| Radar/minimap dirty | Rust records `outcome.radar_cells` for destroyed overlays, but `tick_bridge_repair_orders` discards it because no dirty-cell channel is wired (`walker.rs:368`, `world_orders.rs:354`). | gamemd marks radar terrain dirty for the 3 touched cells only on destroyed-anchor repair (`0x64`, `0x65`, `0xE7`, `0xE8`) (`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.7). | NOT-IMPLEMENTED |
| Zone / path rebuild signal | Rust sets `outcome.zones_dirty=true` for random main-deck/destroyed repair and returns `bridge_state_changed=true` if `zones_dirty || repaired_cells > 0` (`walker.rs:339`, `world_orders.rs:350`). | gamemd sets `bVar1=true` for low `0x4E..0x52`, `0x57..0x5B`, `0x64`, `0x65` and calls `UpdateBridgeZonesHelper` after the walker (`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.6; `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` §2). | PASS |
| Actual rebuilt zone graph equality | Not computed. Rust's low bridge zone model is known broader/simpler than gamemd's low-tube and bridge-kind model. | gamemd has low bridge records with `bridge_kind=1`, tube-backed low bridge cells, and high-only `FindBridgeRecord` behavior (`BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`). | UNCHECKED |
| Engineer consumption | Rust despawns the engineer after the repair branch, even if no cells mutated (`world_orders.rs:360`). | gamemd unconditionally Limbos the engineer after the bridge-repair branch; even no-op repair consumes the engineer (`TECH_CABHUT_GHIDRA_REPORT.md` §4.2, §4.4). | PASS |

## Findings

### FAIL: Scan Center And Trigger Geometry Differ

Player-visible difference: a bridge hut can repair a different 5x5 cell set in Rust than in gamemd. gamemd centers the scan on the hut/building coordinate after the engineer enters the target cell. Rust centers the scan on the engineer's adjacent cell. A low bridge cell that is within the hut-centered 5x5 but outside the engineer-centered 5x5 will repair in gamemd and no-op in Rust; the reverse can also happen.

Evidence:

- Rust: `src/sim/world/world_orders.rs:307` adjacency gate; `src/sim/world/world_orders.rs:341` engineer-centered scan.
- gamemd: `TECH_CABHUT_GHIDRA_REPORT.md` §4.2 and §4.4.

### FAIL: RNG Variant Selection Is Not gamemd-Equivalent

Player-visible difference: the repaired low bridge can choose a different intact art variant (`0x4A..0x4D` for NS, `0x53..0x56` for EW), changing the restored bridge tile appearance and replay RNG stream.

Evidence:

- Rust: `src/sim/bridge_state/walker.rs:415` calls `SimRng::next_range_u32(4)`; `src/sim/rng.rs:28` uses xorshift64*, `src/sim/rng.rs:45` uses modulo.
- gamemd: `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` §2 and `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.5 identify `FUN_00598030(0,3)` as a `Random__Next` rejection-loop.

### FAIL: Rust Resets Damage State But gamemd Leaves Body Damage Byte Stale

Player-visible difference: immediate restored art can still look correct, but redamaging the same repaired bridge may advance from a different state. The verified follow-up explicitly warns that gamemd likely continues from the stale body-cell damage byte on a future hit.

Evidence:

- Rust: `src/sim/bridge_state/walker.rs:337`, `src/sim/bridge_state/walker.rs:342`, and `src/sim/bridge_state/walker.rs:353` set `DamageState::Healthy`.
- gamemd: `REPAIRBRIDGEWALKER_FIELD_11E_FOLLOWUP.md` says engineer repair does not reset `+0x11E`; walkers overwrite only `+0x44`.

### NOT-IMPLEMENTED: Radar Dirty Propagation Is Captured But Dropped

Player-visible difference: the main view/pathing may update, but minimap/radar terrain dirty behavior for destroyed-anchor repair is not faithfully emitted.

Evidence:

- Rust records cells at `src/sim/bridge_state/walker.rs:368` but discards them at `src/sim/world/world_orders.rs:354`.
- gamemd: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.7 marks exactly the three repaired cells dirty for destroyed-anchor repair.

## Adjacent Findings

- Low pavement/ramp repair is not traced here. gamemd has a low ramp-tile branch when no overlay is found in the low scan; Rust's `is_wood_bridge_repair_tile` can force low dispatch but then no-op if no low overlay exists.
- Low bridge post-repair pathing/zone equality is not proven here. Existing docs show Rust's low bridge pathing model differs from gamemd's tube-backed low bridge cells and bridge-kind records.
- C4 hut destruction uses related low/high scan logic, but it is outside this scenario.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE within available tools. Live Ghidra re-check was unavailable; all gamemd evidence is from existing verified research docs.
