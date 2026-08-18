# Stock Allied Refinery FreeUnit Completion Timing Trace

**Date:** 2026-07-28  
**Scenario:** An American human player places stock `GAREFN` on open ground through
`Command::PlaceReadyBuilding`; trace only the build-up/completion timing through exactly one
`CMIN` creation.  
**Rust under test:** clean feature worktree
`.-freeunit-completion-20260728`, commit
`799515ca9867ac189e7c6ea9b03d0d93938d5c6b`.  
**Verdict:** The scoped relative timing matches: no CMIN is created by placement or while
build-up remains active; one CMIN is created in the completion transition; ordinary later
ticks do not repeat it. Absolute stock build-up duration and the first post-spawn
AI/presentation boundary remain `UNCHECKED`.

## Scope and evidence

- Active binary: the connected current program is retail
  `<ra2-install>/gamemd.exe`, x86 image base
  `0x00400000`; this is the active YR program, not a dormant TS binary.
- Stock YR data: `ini/rulesmd.ini:11722-11736` gives `[GAREFN]`,
  `Refinery=yes`, and `FreeUnit=CMIN`; `ini/rulesmd.ini:7351-7371` defines `CMIN` and excludes
  it from random starting units. `ini/artmd.ini:1763-1773` gives the stock `4x3` foundation
  and `Buildup=GAREFNMK`.
- Fresh identity check: `read_memory 0x007E3EB8` returned the complete-object locator
  pointer `0x007FC360`; `read_memory 0x007FC360` returned TypeDescriptor `0x00818D60`;
  `read_memory 0x00818D60` returned `.?AVBuildingClass@@`. The resulting BuildingClass
  vtable base is `0x007E3EBC`.
- Fresh slot checks: `read_memory 0x007E40F8` shows BuildingClass vtable slot `+0x244`
  points to `0x00449A50`; `read_memory 0x007E4390` shows slot `+0x4DC` points to
  `0x00445F80`. Bodies and roles were then confirmed with
  `decompile_function 0x00449A50` and `decompile_function 0x00445F80`; display labels were
  not treated as proof.

## Pipeline

Player click -> scheduled `PlaceReadyBuilding` -> deterministic early command commit ->
spawn one GAREFN with `BuildingUp {0,30}` and zero CMIN -> late phase increments build-up ->
completion tick clears `BuildingUp` -> completion-owned `FreeUnit` lookup -> spawn one CMIN
and assign Harvest -> app refreshes entity presentation -> later ticks produce no second CMIN.

## Entry-point coverage for this scenario

1. Human click:
   `src/app_commands.rs:264-307` schedules `Command::PlaceReadyBuilding` from the stored
   preview origin.
2. Deterministic sim command:
   `src/sim/world/world_commands.rs:667-678` resolves owner/type IDs and calls
   `production::place_ready_building`.
3. Placement:
   `src/sim/production/production_placement.rs:163-249` spawns GAREFN, attaches
   `BuildingUp`, and consumes one ready item. It has no FreeUnit spawn hook.
4. Sole completion consumer:
   `src/sim/world/mod.rs:1985-1996` is the only caller of
   `spawn_completed_refinery_free_units`; `src/sim/production/production_refinery.rs:21-71`
   consumes only building IDs returned by that tick's completion transition.

AI can issue the same command, and MCV deploy can attach `BuildingUp`, but neither is an
additional entry point in this human-GAREFN scenario. Map-load/direct-spawn entities do not
enter this ready-building completion path.

## Concrete stage trace

| Stage | Rust value/order at `799515ca` | Active gamemd value/order | Verdict |
|---|---|---|---|
| 1. Configuration lookup | `ObjectType::from_ini_section` stores literal `CMIN` at `src/rules/object_type.rs:1030-1032`; `RuleSet::refinery_free_unit` requires `Refinery=yes` and resolves the target object at `src/rules/ruleset.rs:2345-2354`. Output is `CMIN`. | `decompile_function 0x00460540` shows section-local `FreeUnit` read, `UnitTypeClass::FindOrAllocate`, then the resolved pointer written at BuildingTypeClass `+0xEA0`. Stock YR input is `CMIN`. | **PASS** |
| 2. Ready placement | At execution tick `T`, commands are sorted by `(execute_tick, owner)` (`src/sim/world/mod.rs:1892-1905`). GAREFN is spawned and receives `BuildingUp { elapsed_ticks: 0, total_ticks: 30 }` (`production_placement.rs:206-229`). Immediate CMIN count is `0`. | `decompile_function 0x004FB0E0` shows held-object placement through vtable `+0xD8` and completed-ready-item handling; it contains no `FreeUnit`/`+0xEA0` read, no `UnitClass` constructor, and no construction-complete call. Immediate CMIN count is `0`. | **PASS** |
| 3. Build-up wait | Late phase increments elapsed with saturating `+1`; while elapsed `<30`, the building ID is not returned (`world/mod.rs:1801-1822`). Because the old placement hook was removed, CMIN count remains `0`. With command execution at `T`, end-of-tick elapsed is `1`; through `T+28`, elapsed is at most `29`. | `decompile_function 0x00449A50` shows construction state 0 starting `GrandOpening(0)` and state 1 returning without the completion call while BuildingClass `+0x6DD == 0`. Thus the FreeUnit consumer is unreachable during build-up and CMIN count remains `0`. | **PASS** |
| 4. Absolute build-up duration | Fixed `30` simulation advances; with the production/test `67 ms` step this is a nominal `2010 ms` budget. Because tick `T` itself performs increment 1, completion is processed at `T+29`. | Native duration is driven by the `GAREFNMK` animation setting `+0x6DD`; this trace did not compute the retail SHP frame count/state cadence into a literal completion tick. | **UNCHECKED** |
| 5. Completion transition | At `T+29`, elapsed becomes `30`, the stable ID is appended, then `building_up` is cleared for every finished ID before FreeUnit spawning (`world/mod.rs:1806-1821`). Finished IDs come from `keys_sorted()` and are consumed in that order (`production_refinery.rs:21-32`). | On the first construction dispatch observing `+0x6DD != 0`, `decompile_function 0x00449A50` shows radio `0x0C`, radio `0x03`, `GrandOpening(1)`, then BuildingClass vtable `+0x4DC` with argument `0`, then mission `5`. The completion callback therefore follows visual construction completion. | **PASS** |
| 6. Exactly one CMIN creation | The completion consumer re-reads the completed GAREFN, resolves `CMIN`, and calls `spawn_object` once (`production_refinery.rs:32-68,74-139`). `spawn_object` unlimbos the unit and assigns Harvest (`world_spawn.rs:405-430,254-277`); `MissionType::Harvest` is numeric `10` (`src/sim/mission/mod.rs:50-65`). Output count becomes `1`. | `decompile_function 0x00445F80` shows the callback set the one-shot field, read Type `+0xEA0`, allocate/construct one `UnitClass`, unlimbo it, then on success queue mission `10` and commence it. Output count becomes `1`. | **PASS** |
| 7. Ordinary later ticks | Completion removes `BuildingUp`; `tick_building_up` can no longer return this GAREFN on `T+30` or later. There is no second spawn hook, so CMIN count remains `1`. Source regression `stock_refinery_free_unit_spawns_on_building_up_completion_once` asserts `0 -> 0 -> 1 -> 1` at `production_placement_tests.rs:649-693`. | `decompile_function 0x00445F80` shows BuildingClass `+0x6E4` is tested at entry and set before the FreeUnit branch; a later ordinary `param_2=0` call returns immediately. `decompile_function 0x00449A50` also shows the construction mission switches to mission `5` after the callback. CMIN count remains `1`. | **PASS** |
| 8. First presented frame | Rust creates CMIN in late phase, sets `TickResult.spawned_entities`, and the app refreshes entity atlases after the tick (`src/app_sim_tick.rs:1254-1285,1372-1375`). The renderer can consume the new entity on the next presented app frame. | Native successful `Unlimbo` makes CMIN map-live inside the completion callback, but this trace did not capture the exact display-swap boundary relative to that logic frame. | **UNCHECKED** |
| 9. First downstream CMIN AI slice | Rust spawns after the tick's object-AI and movement regions, so the CMIN is Harvest-assigned immediately but cannot receive an object-AI/movement slice until `T+30`. | Native assigns/commences mission `10` in the callback. Whether a newly appended object receives another active-object iteration in the same native frame was not exhaustively traced. | **UNCHECKED** |

## Deterministic commit order

- Rust order is: early due-command commit -> all ordinary object/system phases -> late
  `keys_sorted()` build-up increment -> collect finished stable IDs -> clear every finished
  `BuildingUp` -> iterate finished IDs in stable-ID order -> allocate/unlimbo CMIN -> assign
  Harvest -> late frame/tick commit.
- Native scoped order is: construction mission observes animation-complete handshake ->
  completion radio/`GrandOpening(1)` -> one-shot callback -> one CMIN allocation/unlimbo ->
  mission 10/commence -> building mission 5. This is active standard-YR behavior because
  stock `GAREFN` supplies the non-null `FreeUnit=CMIN` pointer.
- With only one GAREFN completing, cross-building tie order cannot affect the scenario.

## Milestone failures and not implemented

None in the scoped no-during-build / once-at-completion / no-repeat path.

## Residuals

1. **Every placed refinery; visible-duration risk:** Rust uses a type-independent fixed
   30-tick `BuildingUp`, while gamemd waits for the actual building animation handshake.
   The concrete stock GAREFNMK duration was not computed, so this is `UNCHECKED`, not FAIL.
2. **One tick at creation; low but deterministic risk:** Rust's late-phase spawn guarantees
   no CMIN AI/movement until the next tick. Native same-frame active-object eligibility is
   `UNCHECKED`.
3. Exact display-swap timing is `UNCHECKED`; entity existence and completion ownership are
   verified, but no runtime pixel/frame capture was made.

## Adjacent findings (not traced and excluded from verdict)

- Coordinate and facing, including primary/fallback cells, are owned by the other trace
  slots and are intentionally not evaluated here.
- Blocked primary/fallback placement and refund-on-total-failure are owned by the other
  trace slots and are intentionally not evaluated here.
- Rust's no-repeat guarantee in this scenario relies on removing `BuildingUp`; it does not
  model gamemd's persistent `ActuallyPlacedOnMap` fence. Synthetic reattachment of
  `BuildingUp` is outside this scenario.
- Rust gates `FreeUnit` through `Refinery=yes`; native callback consumption is keyed by the
  `FreeUnit` pointer. Mod behavior is outside this stock-GAREFN scenario.

## Validation and tally

- Frozen source was inspected at exact commit `799515ca`; the worktree was clean.
- The commit records: `cargo check -p vera20k`; placement tests `40 passed, 0 failed,
  2 ignored`; refinery helper tests `2 passed, 0 failed`.
- No Cargo command was rerun because two concurrent Cargo processes already owned the
  shared build resources. The named source regression was inspected directly.

**PASS: 6 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0**
