# MTNK Static GAWALL Detour Retrace — 2026-07-20

**Status:** PARTIAL — the current Rust route and the first DriveTrack frames are literal source-derived values; the literal post-smoothing `gamemd.exe` route and full native position timeline remain UNCHECKED.

**Scenario:** stock YR `[MTNK]`, center of flat clear Temperate cell `(50,50)`, ground Z, idle/from rest, body facing east `0x40`; one stock `GAWALL` overlay at `(55,50)`; normal player move to center `(60,50)`.

**Scope:** command/destination, zone precheck, A*, wall and diagonal legality, both smoothing passes, DriveTrack selection/budget/chaining, visible position/facing, and arrival. Read-only trace; no Rust/build/test mutation.

## Evidence and active-YR check

- Research-index `research_brief(query="stock YR MTNK Grizzly wall detour pathfinding DriveLocomotion A* smoothing DriveTrack current Rust", system="movement", anchors=["DriveLocomotionClass","Pathfinding","MTNK","GAWALL"])` was used first.
- Stock data: `ini/rulesmd.ini` gives `[MTNK] Speed=7`, `ROT=5`, Drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`, `Accelerates=false`, and `Crusher=yes`. `ini/rules.ini` gives `[GAWALL] Wall=yes`, `[105mm] Warhead=AP`, and `[AP] Wall=yes`.
- Read-only Ghidra MCP confirmed the open program is retail `gamemd.exe`. `decompile_function(0x00429A90)` confirmed the active A* loop calls mover vtable `+0x1AC` on the candidate cell and performs no separate cardinal-flank test for a ground diagonal. `decompile_function(0x0042C900)` and its caller `FootClass__Run_AStar @ 0x004CBBA0` confirm standard-YR reachability.
- `read_memory(0x0081870C)` yielded native entry costs `1,1000,1,1,60,20,8,10000`; `read_memory(0x0081872C)` yielded direction epsilons `.001,.005,.002,.006,.003,.007,.004,.008`; `read_memory(0x007E3774)` confirmed neighbor order N, NE, E, SE, S, SW, W, NW.
- `decompile_function(0x0042B210)` / `decompile_function(0x0042B420)` confirmed pass 1; `decompile_function(0x0042B7F0)` confirmed the distinct native pass-2 rerouter.
- Existing verified evidence used: `pathfinding/PATHFINDING_ASTAR_GHIDRA_REPORT.md`, `pathfinding/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`, `pathfinding/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`, and `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`.

## Concrete route

Current dirty Rust produces this exact raw A* path:

```text
(50,50) (51,50) (52,50) (53,50) (54,50)
(55,49) (56,50) (57,50) (58,50) (59,50) (60,50)
```

Rust direction sequence: `E,E,E,E,NE,SE,E,E,E,E` (`2,2,2,2,1,3,2,2,2,2` in the Rust A* convention).

The important A* tie is at `(55,49)`: predecessor `(54,50)->NE` has `g=5013`; the equal-cost `(54,49)->E` candidate does not replace it because relaxation is strict `<`. The selected frontier then reaches `(56,50)` at `g=6019` and the goal at `g=10027`.

Rust pass 1 leaves the route unchanged: replacing `NE,SE` with `E` would enter `(55,50)`, which its walkability closure rejects. Rust pass 2 also leaves it unchanged. The literal active-gamemd final route is **UNCHECKED**: its float A* inputs strongly derive the same raw detour (NE wins over SE by `.005 < .006`), and pass 1 rejects the wall shortcut, but this trace did not execute/capture the complete native pass-2 output. This is not a parity certification.

Wall ownership was not specified in the scenario. That matters natively: stock MTNK's AP weapon can damage walls, so `UnitClass::Can_Enter_Cell` can classify a non-allied wall as code `5` (cost `20`) or an allied wall differently, while Rust unconditionally turns `Wall=yes` into a hard-unwalkable grid cell. Both favor this cheaper one-cell detour, but the mechanism and byte result differ.

## Stage trace and verdicts

| # | Stage | gamemd.exe | Current dirty Rust | Verdict |
|---:|---|---|---|---|
| 1 | Stock type inputs | Retail MTNK values above; active Drive. | Parser now carries `Accelerates=false`; raw speed conversion is `floor(7*256/100)*15 = 255` leptons/s. | PASS |
| 2 | Command-to-first-track ownership | Move order supplies destination; no-track Drive `Process_Movement -> Process_Drive_Track(0)` starts the track in the Drive tick. | `movement_commands.rs:550-615` selects/creates the initial track during command dispatch. | FAIL |
| 3 | Requested destination | Exact center `(60,50)` is a clear legal Normal-zone goal. | `resolve_requested_move_goal` leaves `(60,50)` unchanged. Native destination-state bytes were not captured. | UNCHECKED |
| 4 | Zone precheck | `AStar_pathfind_search @ 0x0042C900` uses source/goal zone IDs and native `Zone_precheck`/hierarchy state. | `zone_search.rs:510-590` uses reduced connected-zone reachability before layered A*. Same expected boolean is insufficient proof. | FAIL |
| 5 | Wall classification | `GAWALL` is evaluated through live `UnitClass::Can_Enter_Cell`; result depends on wall owner and MTNK/AP attackability and can be soft code `4/5`, not necessarily `7`. | `overlay_grid.rs:201-208` + `pathfinding/core.rs:1864-1865` make every wall overlay hard blocked. | FAIL |
| 6 | A* neighbor/cost mechanism | Float edge cost plus exact epsilon; native heap/node mechanism. | Integer `1000 + tie` and a Rust heap ordering that additionally prefers higher `g`, then coordinates. | FAIL |
| 7 | Ground diagonal corner | No separate flank-cardinal check; candidate cell alone is queried. | Flat-ground diagonal also has no flank check; bridge-only flank checks do not apply. | PASS |
| 8 | Rust raw route | N/A. | Exact path printed above. | PASS |
| 9 | Native literal route | Source-derived candidate is the same NE/SE detour, but no executable/native capture completed. | N/A. | UNCHECKED |
| 10 | Corner smoothing | Wall shortcut must return exactly code `0`; a wall code `4/5/7` rejects it. | Wall closure rejects the same midpoint; route unchanged. | PASS |
| 11 | Straight-segment optimization | Native `Path_optimize_straight_segments @ 0x0042B7F0` uses Chebyshev regression, two reroute orderings, full `Can_Enter_Cell`, slopes/cliffs, and compaction. | `path_smooth.rs:258-398` uses a different cross-product detector; for this path it is a no-op. Exact native mechanism is absent. | NOT-IMPLEMENTED |
| 12 | MTNK speed fraction | `Accelerates=false` directly assigns Drive target fraction before `GetCurrentSpeed`. | `drive_locomotion.rs:99-133` now directly assigns target fraction; flat-clear target is `1`. | PASS |
| 13 | Fresh budget | Exact native `GetCurrentSpeed` integer for all runtime modifiers was not captured. | `255 / 15 = 17` each Rust-native movement frame. | UNCHECKED |
| 14 | First straight track | East->east selects TurnTrack `18`, RawTrack `1`, flags `3`, target facing `0x40`. | Same extracted table tuple and facing. | PASS |
| 15 | Point/residual loop | Strict `budget > 7`, subtract `7` per point, residual retained. | Same local arithmetic, but event handling below changes cadence. | PASS |
| 16 | Cell-cross continuation | Native Drive continues its same `Process_Drive_Track` point loop/state machine with remaining budget unless its verified control path explicitly retries. | `drive_track.rs:3745-3775` breaks at coordinate cell jump; `movement_tick.rs:1358-1486` ends this unit's tick. | FAIL |
| 17 | Turn-chain lookahead | Native chain tests the cell one direction beyond the current head-to and applies code-specific switch behavior. | After entering `(55,49)`, `movement_tick.rs:1494-1543` uses `path[next_index] -> path[next_index+1]`, skipping the immediate SE leg; it can select NE->E and anchor toward `(57,50)`. | FAIL |
| 18 | Full positions/facings | Literal native full timeline unavailable. | Only the source-derived prefix below was completed; full turn/arrival timeline was not executed because Cargo/build was explicitly out of scope. | UNCHECKED |
| 19 | Arrival/clear | Native NavCom/mission/locomotor byte sequence not captured. | At path exhaustion Rust waits for the active track to finish, then snaps center, clears movement/track, and sets Idle (`movement_tick.rs:1659-1677`, `1896-1941`). | UNCHECKED |

## Literal current-Rust first-cell timeline

The app simulation is 45 Hz (`util/fixed_math.rs:51`), while DriveTrack advances every third sim tick at 15 Hz (`movement_step.rs:42-57`). Other sim ticks hold the prior position. Position is `(cell; sub_x,sub_y)` in leptons.

| Sim tick | RawTrack point | Residual | Position | Facing |
|---:|---:|---:|---|---:|
| command | 0 selected | 0 | `(50,50; 128,128)` | `0x40` |
| 1 | 2 + interpolation | 3 | `(50,50; 165,128)` | `0x40` |
| 2-3 | held | 3 | `(50,50; 165,128)` | `0x40` |
| 4 | 4 + interpolation | 6 | `(50,50; 192,128)` | `0x40` |
| 5-6 | held | 6 | `(50,50; 192,128)` | `0x40` |
| 7 | 7 + interpolation | 2 | `(50,50; 219,128)` | `0x40` |
| 8-9 | held | 2 | `(50,50; 219,128)` | `0x40` |
| 10 | 9 + interpolation | 5 | `(50,50; 245,128)` | `0x40` |
| 11-12 | held | 5 | `(50,50; 245,128)` | `0x40` |
| 13 | 11; cell jump breaks loop | 8 | `(51,50; 4,128)` | `0x40` |
| 14-15 | held after jump | 8 | `(51,50; 4,128)` | `0x40` |

At tick 13, budget was `17+5=22`: point 10 leaves `15`, point 11 leaves `8`, then Rust breaks for the cell transition even though `8 > 7`. This produces a concrete hold/cadence divergence risk exactly in ordinary straight MTNK motion, before the wall is reached.

## Top player-visible findings

1. **FAIL — cell-cross budget stall:** Rust pauses after every coordinate cell jump with spendable residual; ordinary straight movement cadence can visibly pulse/stutter.
2. **FAIL — chain lookahead skips the immediate SE leg:** at the wall detour turn, Rust can select/anchor the wrong follow-on curve and therefore produce wrong position and body facing.
3. **FAIL — wall semantics collapsed to hard blocked:** gamemd can return soft attackable-wall codes; Rust loses owner/weapon/result-code behavior, altering routes and wall interaction.
4. **NOT-IMPLEMENTED — native pass-2 optimizer:** Rust's different/no-op mechanism cannot reproduce native route straightening generally.
5. **FAIL — first track starts in command dispatch:** the first-frame path/track/speed state is written in a different phase than active gamemd Drive processing.

## Adjacent findings (not expanded)

- The Rust A* heap tie mechanism and native float/epsilon mechanism can diverge on other equal-cost fronts even though the candidate raw path here is the same.
- Native wall ownership must be fixed explicitly in any executable parity fixture.
- Full MTNK `GetCurrentSpeed` rounding/modifier capture and a native frame-position capture are still required before literal movement timing can receive PASS.

## Tally

**PASS: 7 | FAIL: 6 | UNCHECKED: 5 | NOT-IMPLEMENTED: 1**

**Overall: PARTIAL.** The trace identifies concrete source-level causes for movement feeling unlike `gamemd.exe`, but strict route/timeline parity remains unproven until the native post-smoothing route, exact `GetCurrentSpeed`, and native frame positions are captured.
