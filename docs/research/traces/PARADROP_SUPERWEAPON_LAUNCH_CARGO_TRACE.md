# Paradrop Superweapon Launch + Cargo Trace

Scenario: American house launches `AmericanParaDropSpecial` onto open ground at target `(50,20)` on a `100x100` map.

Scope: PDPLANE spawn, owner, American payload list, cargo role, and limbo/occupancy behavior only.

Status: COMPLETE, with one input caveat. The scenario does not explicitly set the house's `WaypointEdge` / `House+0x1E0`, so exact carrier spawn-cell equality is only computed under the current Rust default edge `0` (North). All cargo facts are independent of that caveat.

## Pipeline

`AmericanParaDropSpecial` command
-> `SuperWeaponKind::AmerParaDrop`
-> `sim::superweapon::paradrop::launch(..., ParaDropKind::American)`
-> resolve `[General] ParaDropPlane` / default `PDPLANE`
-> resolve `AmerParaDropInf/Num`
-> create one carrier at the house edge
-> set carrier owner + Open-equivalent mission target
-> create `AmerParaDropNum` infantry in limbo
-> mark each passenger `Inside { transport_id: pdplane_id }`
-> force-link passengers into carrier cargo without normal capacity checks
-> leave passengers out of map occupancy.

## Stage Verdicts

### 1. Superweapon dispatch

Our output: `src/sim/world/world_commands.rs:1219-1229` maps `SuperWeaponKind::AmerParaDrop` to `paradrop::launch(..., ParaDropKind::American)`.

gamemd output: Standard YR `[AmericanParaDropSpecial] Type=AmerParaDrop` (`ini/rulesmd.ini:30967-30977`) routes through live `SuperClass::Launch` case 6, which calls `FUN_0065E660`.

Verdict: PASS. Both select the American paradrop path.

### 2. Payload data

Our output: `RuleSet` parses `AmerParaDropInf` and `AmerParaDropNum` into `general.amer_paradrop_list` (`src/rules/ruleset.rs:198-201`, `src/rules/ruleset.rs:838-844`). Stock YR data is `E1` and `8` (`ini/rulesmd.ini:241-242`), so the scenario payload is exactly eight `E1`.

gamemd output: The active American call site passes the infantry index from the American list and the matching `AmerParaDropNum[i]`; the spawner loop count is the final passenger-count argument. The verified report identifies the American call at `0x006CD655`.

Verdict: PASS. Both produce one payload entry: `E1 x 8`.

### 3. Carrier type and count

Our output: `launch` uses `rules.general.paradrop_aircraft_type` (`src/sim/superweapon/paradrop.rs:135-140`). `RuleSet` defaults `[General] ParaDropPlane` to `PDPLANE` when absent (`src/rules/ruleset.rs:827-831`). The loop spawns one PDPLANE per payload list entry (`src/sim/superweapon/paradrop.rs:106-121`), so stock American payload spawns one PDPLANE.

gamemd output: Standard call sites pass aircraft count `1` and PDPLANE type index to `FUN_0065E660`; stock `[AircraftTypes]` contains `PDPLANE`, and `[PDPLANE]` is active in `rulesmd.ini:11536-11575`.

Verdict: PASS. Both spawn one `PDPLANE`.

### 4. Carrier owner

Our output: `spawn_pdplane` resolves `owner` to `"Americans"` and calls `spawn_object_at_height(&pdplane_type, &owner_str, ...)` (`src/sim/superweapon/paradrop.rs:135-147`). `spawn_object_at_height` interns that owner into the `GameEntity` (`src/sim/world/world_spawn.rs:318-330`).

gamemd output: The spawner receives the owner house in `ECX`; carrier `CreateObject` is invoked from that live owner context. The spawner report verifies standard `SuperClass::Launch` cases 5/6 call `FUN_0065E660` on active YR paths.

Verdict: PASS. Both assign the PDPLANE to the launching American house.

### 5. Carrier spawn cell

Our output if `waypoint_edge=0`: `find_paradrop_carrier_edge_cell(100,100, North, (50,20))` scans the north edge and picks `(50,0)` by squared-distance minimum (`src/sim/world/edge_cell.rs:64-80`, `src/sim/world/edge_cell.rs:108-128`).

gamemd output if `House+0x1E0=0`: `FUN_0065E660` calls the map edge helper with criterion `4`, which bypasses ordinary passability; for a north-edge `100x100` map and target x `50`, the closest accepted north candidate is `(50,0)`.

Verdict: UNCHECKED for the scenario as written, because the house edge value is not specified. Under the explicit default-edge assumption, both compute `(50,0)`.

### 6. Carrier spawn passability

Our output: `launch` ignores the supplied `PathGrid` for carrier spawn and calls `find_paradrop_carrier_edge_cell` with only map width, height, edge, and target (`src/sim/superweapon/paradrop.rs:43`, `src/sim/superweapon/paradrop.rs:81-86`). The helper returns `None` only for zero-sized maps (`src/sim/world/edge_cell.rs:64-80`).

gamemd output: The spawner edge helper call uses criterion `4`; `FUN_004AAB30` returns true immediately for that criterion, before ordinary passability/object checks.

Verdict: PASS. Both ignore ground passability for carrier edge selection.

### 7. Passenger creation count/type

Our output: `spawn_pdplane` loops `0..num`, with `num=8` for stock American, and calls `spawn_object_limbo_at_height("E1", "Americans", edge_cell, ...)` (`src/sim/superweapon/paradrop.rs:178-193`). `spawn_object_limbo_at_height` inserts each `E1` into `EntityStore` and owned counts without occupancy (`src/sim/world/world_spawn.rs:446-569`).

gamemd output: The passenger loop runs exactly `passenger_count` times, using the infantry type from the American list. Passenger creation failure decrements the remaining loop counter, but in the normal scenario with valid `E1`, eight passengers are created.

Verdict: PASS. Both create eight American `E1` passengers for stock YR data.

### 8. Passenger cargo role and cargo count

Our output: Each passenger receives `PassengerRole::Inside { transport_id: pdplane_id }` (`src/sim/superweapon/paradrop.rs:194-198`). The carrier cargo is created if absent and loaded with `board_forced` (`src/sim/superweapon/paradrop.rs:160-175`, `src/sim/superweapon/paradrop.rs:199-212`). `board_forced` pushes the stable id without `capacity` or `size_limit` gates (`src/sim/passenger.rs:78-86`).

gamemd output: `FUN_0065E660` creates passengers in limbo, then calls `CargoClass::AddPassenger`; the verified `CargoClass::AddPassenger` path links passengers and recomputes count without normal `Passengers=` / `SizeLimit=` gates.

Verdict: PASS. Both produce eight passengers linked as carrier cargo, not normal boarded map occupants.

### 9. Passenger occupancy / limbo behavior

Our output: Passenger creation uses `spawn_object_limbo_at_height`, which inserts into `EntityStore` but does not call `occupancy.add` (`src/sim/world/world_spawn.rs:446-569`). `spawn_pdplane` no longer removes transient occupancy because none is added. `OccupancyGrid::contains_entity` would only find ids previously added to a cell (`src/sim/occupancy.rs:235-240`).

gamemd output: Passenger objects are not `Unlimbo`'d in `FUN_0065E660`; they are directly linked into cargo. Therefore they never transiently occupy the edge cell during loading.

Verdict: PASS. Both keep the eight loaded `E1` passengers out of map occupancy.

### 10. Silent carrier lifecycle side effects

Our output: Carrier creation currently uses normal `spawn_object_at_height`, which inserts the PDPLANE and registers occupancy (`src/sim/superweapon/paradrop.rs:138-158`, `src/sim/world/world_spawn.rs:292-443`). This path does not model gamemd's `g_MapEditorMode` carrier-create/unlimbo suppression boundary.

gamemd output: `FUN_0065E660` wraps carrier `CreateObject` and carrier `Unlimbo` in `g_MapEditorMode`, while passenger loading is not unlimbo'd.

Verdict: UNCHECKED. The spawner suppression window is verified in gamemd, but the current Rust side-effect surface for construction sounds, radar pings, reveal hooks, or other lifecycle notifications was not exhaustively traced in this single cargo slot. No concrete player-visible mismatch is proven here.

## Findings

No FAIL or NOT-IMPLEMENTED result was found for the requested cargo-loading surface.

The only unresolved equality check is the exact carrier spawn cell because the scenario omits the house edge. With the current Rust default `waypoint_edge=0`, both sides compute `(50,0)` for target `(50,20)` on a `100x100` map.

## Adjacent Findings

Carrier lifecycle silence may need a separate trace if the engine later adds spawn sounds, radar events, fog first-seen hooks, or other lifecycle notifications to `spawn_object_at_height`.

The current Rust `spawn_object_limbo_at_height` duplicates normal spawn construction. That is not a parity failure in this scenario, but shared construction should eventually be extracted to prevent normal and limbo spawns from drifting.

## Verdict Tally

PASS: 8
FAIL: 0
UNCHECKED: 2
NOT-IMPLEMENTED: 0

## Sources

- `docs/research/PDPLANE_SPAWNER_EDGE_SILENT_PATH_GHIDRA_REPORT.md`
- `docs/research/PARADROP_MISSION_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/sim/superweapon/paradrop.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/world/edge_cell.rs`
- `src/sim/passenger.rs`
- `src/sim/occupancy.rs`
- `src/rules/ruleset.rs`
