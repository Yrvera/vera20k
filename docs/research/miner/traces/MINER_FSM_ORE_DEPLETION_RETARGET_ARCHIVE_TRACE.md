# Miner FSM Ore Depletion Retarget/Archive Trace

Date: 2026-05-20  
Trace slot: 2  
Mechanic: Chrono Miner harvesting a target ore cell that becomes empty before cargo is full, then retargeting nearby ore or falling out of the local harvest loop.  
Scope: One concrete standard YR Chrono Miner scenario only.

## Concrete Scenario

Standard YR `CMIN` is already in the harvest mission, physically on its target ore cell `(20,20)`, with no movement destination, no saved archive/ghost cell, and `0/20` cargo. The target cell contains 5 ore bales. A second reachable ore cell `(21,20)` contains 5 ore bales. No other ore exists inside `TiberiumShortScan`. The miner is not full after draining `(20,20)`, so this is the depleted-before-full continuation path, not the full-cargo return path.

Active standard YR confirmation:
- `ini/rulesmd.ini` has `[CMIN]`, `Harvester=yes`, `Storage=20`, `Teleporter=yes` at lines 7351, 7364, 7374, and 7396.
- `UnitClass::Mission_Harvest` @ `0x0073E5E0` gates this path on the live `Harvester` flag at `TypeClass+0xE0E` and the live `Teleporter` flag at `TypeClass+0xCD4`; this is not TS-only fog/weed legacy.
- `UnitClass::Harvest_Ore_Tick` @ `0x0073D450`, `FootClass::Search_For_Tiberium_And_Move` @ `0x004DCFE0`, `FootClass::Scan_For_Tiberium` @ `0x004DD0A0`, and `CellClass::Reduce_Tiberium` @ `0x00480A80` were spot-checked read-only in Ghidra.

## Pipeline

`Harvest timer/step gate expires -> Harvest_Ore_Tick drains current ore -> next harvest attempt sees empty current cell -> TiberiumShortScan from current cell -> Set_Destination to nearby ore -> continue harvest-state loop until arrival/extraction -> render harvest flags/overlays`

## Stage Table

| Stage | Boundary Checked | Our Output | gamemd Output | Verdict |
|---|---|---:|---:|---|
| 1. Rules/entity setup | CMIN cargo capacity and harvester identity | Capacity 20 via `Miner::new` `Storage=20`; state can be `Harvest` | CMIN `Storage=20`, `Harvester=yes`, `Teleporter=yes`; Mission_Harvest active for standard YR | PASS |
| 2. First extraction amount | From target cell `(20,20)` with 5 bales and empty capacity 20 | `min(20,5)=5` bales; target node removed | `Reduce_Tiberium(20)` clamps to density+1=5, removes overlay, returns 5 | PASS |
| 3. Cargo after depletion | Cargo after drain before retarget | `0+5=5`, not full (`5<20`) | `StorageClass::AddAmount(5, ore)`, not full (`5/20<1.0`) | PASS |
| 4. Empty-cell continuation trigger | Next harvest cycle on now-empty current cell | After `harvest_tick_interval=18`, `extract_bales_max` returns 0 | State 1 waits StepTimer, `Harvest_Ore_Tick` returns 0 when current cell LandType is no longer Tiberium | PASS |
| 5. Local continuation scan | Scan from `(20,20)` with radius 6 and one candidate at `(21,20)` | `search_local_ore(... radius=6)` returns `(21,20)` | `Search_For_Tiberium_And_Move(TiberiumShortScan=6, zone arg 0)` returns true and sets destination to `(21,20)` | PASS |
| 6. State after retarget hit | FSM state and harvesting flag after nearby ore found | `target_ore_cell=Some((21,20))`, `state=MoveToOre` | `MissionSubState=1`, `UnitClass+0x6D2=1`, destination set to `(21,20)` | FAIL |
| 7. Archive/ghost cell use | Saved archive on non-full retarget hit | `last_harvest_cell` remains `None` | No `SetGhostCell(found_cell)` in the non-full found branch; archive is only cleared on miss or saved in full branch | PASS |
| 8. Visual propagation | Whether harvest visuals remain active while the miner moves to `(21,20)` | `is_harvesting = state == Harvest`, so voxel/oregath visibility turns off in `MoveToOre` | Ghidra verifies `UnitClass+0x6D2=1`; exact render consumer from `+0x6D2` to pixels was not traced in this run | UNCHECKED |

## Failures

### Stage 6 - Retarget hit exits Harvest state in our FSM

Player-visible risk: when a Chrono Miner drains one ore cell and selects the adjacent one, our miner leaves `Harvest` and enters `MoveToOre`; gamemd keeps `Mission_Harvest` substate 1 and sets its active-harvesting flag. In our code this immediately changes downstream behavior because `tick_miners` drives both voxel animation and `HarvestOverlay` visibility from `snap.miner.state == MinerState::Harvest`.

Our code:
- `src/sim/miner/miner_system.rs:548-551`: continuation hit sets `target_ore_cell` then `MinerState::MoveToOre`.
- `src/sim/miner/miner_system.rs:157-184`: voxel animation and `HarvestOverlay` are hidden whenever state is not `Harvest`.

gamemd evidence:
- `UnitClass::Mission_Harvest` @ `0x0073E5E0`, state 1 failed-harvest non-full branch calls `FootClass::Search_For_Tiberium_And_Move(TiberiumShortScan, 0)`.
- If that call succeeds or destination is present, it writes `param_1[0x2F] = 1` and `byte UnitClass+0x6D2 = 1`, then returns 1.
- This path is active for `CMIN` because `TypeClass+0xE0E` is true and `TypeClass+0xCD4` is true in standard YR.

## Not Implemented

None found inside this concrete retarget-present scenario.

## Timing Notes

- Standard YR `TiberiumShortScan=6` and `TiberiumLongScan=48` are present in `ini/rulesmd.ini:311-312`.
- Our `MinerConfig::from_general_rules` parses those values into `local_continuation_radius` and `long_scan_radius`, and the default config also uses 6 and 48.
- For this scenario, both engines wait for another harvest-cycle gate after the successful 5-bale drain before the empty-cell continuation branch runs. Our default `harvest_tick_interval=18` equals `HarvesterLoadRate(2) * 9`.
- Exact post-retarget extraction timing after movement was not marked PASS because it depends on movement duration and gamemd's StepTimer interaction while `Destination != 0`; only the retarget state write was verified.

## Adjacent Findings

- If the local continuation scan misses while the miner is not full, gamemd clears the ghost cell and sets state 2 return-to-refinery. Our branch also clears `last_harvest_cell` and begins return when a refinery exists, but the no-refinery fallback was not traced here.
- `search_local_ore` scores candidates using `node.remaining + 1` rather than density levels plus one. This does not change the concrete one-candidate selection above, so it is not counted as a failure for this scenario.
- Older research text claiming Chrono Miner uses a different state-0 scan radius is superseded by the verified state-0 report: both war and chrono miners use `TiberiumLongScan` in state 0; `TiberiumShortScan` applies to the state-1 continuation path traced here.

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Player-Visible Ranking

1. Stage 6 - Miner retargets correctly but leaves harvest state; our harvest voxel/oregath visibility turns off during the hop to the next ore cell, while gamemd keeps harvest substate 1 and `UnitClass+0x6D2=1`.

## Status

COMPLETE
