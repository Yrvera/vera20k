# Multi-Engineer Same-Tick Bridge Repair Trace

**Scenario:** Two engineers enter/arrive adjacent to the same `CABHUT` on the same tick for the same damaged bridge. Verify deterministic processing order, engineer consumption, repeated sound behavior, and whether the second engineer can mutate already-repaired cells.

**Status:** PARTIAL. Ghidra MCP was not available in this session, so binary-side same-tick object ordering and duplicate engineer-entry behavior could not be re-read live. Existing research docs are used as pointers/evidence where they already report Ghidra-verified YR-active paths. Per trace-swarm rules, no stage is marked PASS without a literal computed comparison against both Rust and gamemd.

**Report path:** `C:/Users/enok/Documents/ra2-rust-game-docs/traces/MULTI_ENGINEER_SAME_TICK_BRIDGE_REPAIR_TRACE.md`

## Scope

Only the same-tick two-engineer bridge-repair case is traced. Adjacent bridge-repair issues such as cursor selection, pathing-to-hut vs adjacent-footprint arrival, low-bridge repair, C4-on-CABHUT collapse, and bridge render dirty propagation are not traced here.

## Sources

- Rust code:
  - `src/sim/world/world_orders.rs:250-365`
  - `src/sim/world/mod.rs:1238-1251`
  - `src/sim/world/world_orders_bridge_repair_tests.rs:83-96, 107-130, 159-186, 188-243, 368-371, 474-500`
  - `src/sim/bridge_state/walker.rs:62-80, 100-115, 138-155, 237-315, 330-374, 386-417, 501-522, 589-610, 685-694`
  - `src/app_sim_tick.rs:530-571`
  - `src/rules/object_type.rs:519-523, 977-986`
  - `src/rules/ruleset.rs:727-732, 768-772`
- INI data:
  - `ini/rulesmd.ini:721` => `RepairBridgeSound= BridgeRepaired`
  - `ini/rulesmd.ini:3037` => `MultiEngineer=no`
  - `ini/rulesmd.ini:16336-16348` => `[CABHUT] ... BridgeRepairHut=yes`
  - `ini/soundmd.ini:807,5355-5359` => `[BridgeRepaired]`, `Sounds=urepair`, `Type=global`
  - `ini/evamd.ini:49,982-987` => `EVA_BridgeRepaired`
- Existing gamemd research:
  - `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:40-57` verifies `InfantryClass::PerCellProcess @ 0x519630` and repair dispatchers are YR-active for engineer entering CABHUT.
  - `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:103-121` reports engineer-repair path: PerCellProcess, sound/EVA, 5x5 scan, low/high dispatcher.
  - `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md:695-705` reports low-bridge inner scan shape; high path is reported as same shape at lines 121 and 42-43.
  - `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md:1-27,55-70,150-169,251-270` reports YR-active repair walker behavior, current-overlay reads, 3-cell writes, zone helper conditional, and no TS-only gate.

## Concrete Rust Setup

The in-repo same-tick fixture is `two_engineers_both_repair_same_tick`:

- `CABHUT` spawned at `(9,10)`.
- Engineer A spawned at `(10,10)`.
- Engineer B spawned at `(10,11)`.
- Both have `capture_target = Some(cabhut)`.
- High bridge runtime cells seeded at `(10,9)`, `(10,10)`, `(10,11)`, `(10,12)`, `(10,13)`.
- Seeded bridge state is `DamageState::Destroyed`, overlay byte `0xE7` for all five bridge cells.
- The test asserts both engineers are removed, two `SimSoundEvent::BridgeRepaired` events are emitted, and the strip `(10,9..=11)` is healthy after the tick.

## Entry Points

1. Command/intent entry: `CaptureBuilding { engineer_id, target_building_id }` sets `capture_target`; not re-traced here.
2. Tick entry: `Simulation::advance_tick` phase 5 calls `tick_bridge_repair_orders` before normal engineer capture and C4 processing.
3. Repair order loop: `tick_bridge_repair_orders` snapshots all entities with a `capture_target`, sorted by stable entity id.
4. Map mutation entry: `BridgeRuntimeState::repair_bridge_from_engineer_scan`.
5. Audio propagation: each `SimSoundEvent::BridgeRepaired` becomes a `GameSoundEvent::BridgeRepaired` in `app_sim_tick.rs` if SFX or EVA resolves.

## Stage Trace

| Stage | Rust output for this scenario | gamemd evidence | Verdict |
|---|---|---|---|
| Rules/data gate | `ENGI Engineer=yes`; `CABHUT BridgeRepairHut=yes`; `RepairBridgeSound=BridgeRepaired`; `MultiEngineer=no` is parsed but not consulted by bridge repair. | Retail `rulesmd.ini` has same keys; existing docs verify `BridgeRepairHut` flag gates YR-active PerCellProcess path. No live binary re-read here for `MultiEngineer` non-use in this path. | UNCHECKED |
| Tick ordering | Bridge repair runs in phase 5 before normal capture and C4. | Existing docs place repair in `InfantryClass::PerCellProcess`; no live re-check of gamemd tick phase relative to two infantry in same frame. | UNCHECKED |
| Candidate ordering | Snapshot is `entities.keys_sorted()`: cabhut id first but ignored, Engineer A lower id, Engineer B higher id. Processing order is A then B. | Existing docs do not establish same-frame object iteration tie-breaker for two engineers entering the same hut. | UNCHECKED |
| Engineer A trigger | A is adjacent to hut: `dx=1`, `dy=0`; event fires this tick. | Existing research says engineer path fires when engineer steps onto CABHUT cell via `InfantryClass::PerCellProcess`; the same-tick duplicate-enter comparison was not live-verified. | UNCHECKED |
| Engineer A sound | One `SimSoundEvent::BridgeRepaired { rx:9, ry:10, owner:Americans }` is pushed before mutation. | Existing docs report sound/EVA occurs in the engineer repair path, gated by local human and `RepairBridgeSound`; duplicate ordering not verified. | UNCHECKED |
| Engineer A mutation | 5x5 scan centered at `(10,10)` visits first high overlay at `(10,9)`. High NS repair start becomes `(10,10)`. `ns_triple(10,10)` touches `(10,10)`, `(10,9)`, `(10,11)`. All three transition from overlay `0xE7`/Destroyed to `0xCD+variant`/Healthy; `repaired_cells=3`; zones dirty true. | Existing walker doc verifies 3-cell writes and RNG healthy overlay selection for damaged input. Exact random variants were not computed against gamemd RNG for this fixture. | UNCHECKED |
| Engineer A consumption | A is despawned after sound and mutation. | Existing docs identify engineer entering CABHUT path, but same-tick duplicate consumption count was not live-verified. | UNCHECKED |
| Engineer B trigger after A removal | B remains in the prebuilt candidate snapshot. It still exists, is adjacent to the same live hut (`dx=1`, `dy=1`), and fires in the same tick. | Existing docs do not establish whether gamemd allows a second infantry already in the same frame to run the repair branch after the first repaired and was consumed. | UNCHECKED |
| Engineer B sound | A second `SimSoundEvent::BridgeRepaired { rx:9, ry:10, owner:Americans }` is pushed before B's scan. Test requires exactly 2 events. | Existing docs say sound is in the repair path before/with dispatch; no live duplicate-enter verification. | UNCHECKED |
| Engineer B mutation | B's 5x5 scan centered at `(10,11)` sees `(10,9)` first among bridge overlays. That cell is already healthy overlay `0xCD+variant`; repair transition for `0xCD` is `NoChange`, so B mutates 0 cells. Cells `(10,12)` and `(10,13)` remain destroyed in this fixture because the walker returns from the first scanned high bridge overlay. | Existing walker docs verify current-overlay-driven no-op cases exist, but exact duplicate-entry scan after first repair was not computed from gamemd. | UNCHECKED |
| Engineer B consumption | B is despawned after the no-op repair scan. | Same-tick duplicate consumption not live-verified against gamemd. | UNCHECKED |
| App audio | With rules repair sound set, the two sim events each resolve to `GameSoundEvent::BridgeRepaired { sound_id:"BRIDGEREPAIRED", screen_pos:iso(9,10,0), eva_sound_id:Some(...) only for local human owner }`. | Existing docs verify SFX/EVA gates at a high level. Exact duplicate sound enqueue/playback behavior for two same-frame events was not live-verified. | UNCHECKED |

## Findings

No concrete FAIL or NOT-IMPLEMENTED finding is established for the slot-5 scenario under the trace-swarm equality rule. The Rust behavior is deterministic and covered by an integration test, but gamemd's two-engineer same-frame object ordering, duplicate sound enqueue behavior, and second-engineer no-op mutation behavior were not computed from the live binary in this session.

Most important unchecked parity risks:

1. **Same-frame processing order:** Rust uses ascending stable entity id. gamemd likely uses its infantry/object update list order, but that order was not verified for simultaneous CABHUT entries.
2. **Duplicate sound behavior:** Rust emits two bridge repair events even though only the first engineer mutates bridge state. Existing docs confirm sound is in the path, but not whether gamemd suppresses, coalesces, or repeats duplicate same-frame repair sounds.
3. **Second-engineer mutation/no-op:** Rust's second scan can hit already-healthy overlay first and mutate zero cells while still consuming the engineer. Existing walker docs make this plausible, but the exact duplicate-enter sequence was not re-read from gamemd.
4. **Unrepaired tail after first strip:** In the fixture, first engineer repairs only `(10,9..=11)` and leaves `(10,12..=13)` destroyed. The second engineer does not continue to those cells because it dispatches from the first high overlay it scans. This may be correct for the exact overlay-scan algorithm, but needs live binary confirmation for duplicate-entry ordering.

## Adjacent Findings

- Rust currently fires bridge repair for engineers adjacent to a `BridgeRepairHut` target (`dx <= 1 && dy <= 1`). Existing gamemd docs describe `InfantryClass::PerCellProcess` when the engineer steps onto the CABHUT cell. This may belong to the action-gate trace, not this slot's same-tick duplicate trace.
- `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` states `rulesmd.ini` does not set `RepairBridgeSound`, but the checked repo INI has `RepairBridgeSound= BridgeRepaired` at `ini/rulesmd.ini:721`. This looks like a stale doc claim or a base/YR patch distinction and should be verified separately.

## Verdict Tally

PASS: 0 | FAIL: 0 | UNCHECKED: 12 | NOT-IMPLEMENTED: 0

## Status

PARTIAL: Rust-side values were computed from code and fixtures, but gamemd same-tick duplicate-entry ordering and duplicate sound/mutation behavior could not be live-verified because no Ghidra MCP tool was available in this session.
