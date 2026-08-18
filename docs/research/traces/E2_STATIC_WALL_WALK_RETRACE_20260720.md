# E2 Static-Wall Walk Retrace — 2026-07-20

**Scenario:** stock YR `[E2]` Conscript, exact center of flat clear Temperate cell `(50,50)`, ground Z, idle/from rest, body facing east `0x40`; one stock `GAWALL` overlay at `(55,50)`; normal player move order to exact clear-cell center `(60,50)`.

**Trace status:** **RED / NOT PARITY**. The current Rust route can be reduced exactly for this fixture, but the execution after path selection does not implement the active `WalkLocomotionClass` mechanism. The largest visible differences are sub-cell commitment timing, facing, speed/state handling, animation cadence, and arrival completion.

**Verdict tally:** **PASS 4 · FAIL 7 · UNCHECKED 4 · NOT-IMPLEMENTED 3** (18 stages). A PASS below means literal equality was established for that bounded item; it does not certify the larger system.

## Evidence and scope

- This is a read-only retrace of the dirty 2026-07-20 worktree. No Rust, INI, asset, or existing research file was edited.
- No Cargo command or executable trace was run. The Rust path below was evaluated directly from the current deterministic source for the stated flat-grid fixture. Literal native values needing execution are `UNCHECKED`.
- `rulesmd.ini:4327..4358` supplies `Image=CONS`, `Speed=4`, Walk CLSID `{4A582744-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Infantry`. `artmd.ini:138..145` maps `CONS -> ConSequence`; `artmd.ini:13770..13779` gives `Walk=8,6,6`. `rulesmd.ini:12022..12031` gives `GAWALL` and `Wall=yes`.
- Native wall classification additionally reads the overlay-state upper nibble, `DamageLevels`, wall owner/alliance, weapon 0, and `Warhead.Wall`. The scenario does not specify wall state or owner. Rust's static path grid ignores all of those inputs.

## Exact current-Rust path

The current ground A* expands directions in `N, NE, E, SE, S, SW, W, NW` order and uses scaled direction tie values `[1,5,2,6,3,7,4,8]` (`src/sim/pathfinding/core.rs:388..397`, `988..1333`). `GAWALL` becomes `overlay_blocks=true` and then ground-unwalkable (`src/map/resolved_terrain.rs:1514..1556`; `src/sim/pathfinding/core.rs:1859..1866`).

For this fixture, the exact reconstructed Rust path is:

```text
(50,50)
  E  (51,50)
  E  (52,50)
  E  (53,50)
  E  (54,50)
  NE (55,49)
  SE (56,50)
  E  (57,50)
  E  (58,50)
  E  (59,50)
  E  (60,50)
```

Direction row: `[E,E,E,E,NE,SE,E,E,E,E]`, encoded by the current A* as `[2,2,2,2,1,3,2,2,2,2]`. Scaled accumulated `g` is **10027**. North wins over the symmetric south candidate because the NE tie add is `5` while SE is `6`; at the first detour frontier the source-level evaluator obtains north `g=5013` versus south `g=5014`.

Ground diagonal corner cutting is allowed: `(54,50) -> (55,49)` is accepted although cardinal flank `(55,50)` is blocked. The next SE step reaches `(56,50)`. This matches the native destination-cell-only ground diagonal legality mechanism documented for active A*.

### Smoothing output

- Pass 1 (`smooth_layered_path`) cannot replace `NE,SE` with `E,E`, because that replacement enters blocked `(55,50)`. The Rust path is unchanged.
- Pass 2 (`optimize_layered_path`) is a no-op here and, in the current implementation, effectively a no-op generally: `find_drift_segment` compares cumulative displacement with the identical ideal displacement, so its cross product remains zero (`src/sim/pathfinding/path_smooth.rs:352..398`). Native `Path_optimize_straight_segments @ 0x0042B7F0` is an active validation/reroute pass, not this identity test.
- Final current-Rust path is therefore exactly the raw path above.

**Native chosen detour:** **UNCHECKED**. Static evidence proves native direction order/epsilons and active smoothing, but this run did not execute gamemd's A* with the fixture's live cell records. The omitted native wall owner/state also prevents selecting the literal `Can_Enter_Cell` result. It would be unsound to label the north side a native PASS merely because Rust selects it.

## Current-Rust walk execution

1. `world_commands.rs:128..175` accepts the owned living E2, sets the Move mission, and clears attack/dock/order state. `movement_commands.rs:432..471` requests the path. The clear requested goal stays `(60,50)`.
2. `Speed=4` is converted to **150 leptons/second** (`world_commands.rs:70..109`; `src/util/fixed_math.rs:338..379`). `movement_commands.rs:524..537` starts `current_speed` at that full value. The first direction is cell-center east, and infantry facing is immediately written as `0x40` (`movement_commands.rs:495..514`, `579..631`).
3. Walk does **not** receive the destination/NavCom state installed for Drive: `set_destination_internal_cell` is inside the Drive-only branch at `movement_commands.rs:545..577`. Rust instead attaches only `MovementTarget`.
4. The first leg targets the center of `(51,50)`, because no destination sub-cell is selected before movement. Native Walk first unmarks the old sub-cell, calls `FindSubCellDest -> PlaceInfantryInCell`, marks the selected destination sub-cell, then walks toward that exact coordinate (`INFANTRY_SUBCELL_POSITIONING.md:271..302`).
5. On a Rust boundary crossing, `reserve_destination_after_transition` allocates the sub-cell only after the entity has entered the new cell (`movement_step.rs:1368..1386`; `movement_reservation.rs:13..55`). It then separately pre-allocates a sub-cell in the *next* cell, does not reserve that future choice in occupancy, and overwrites `locomotor.subcell_dest` (`movement_step.rs:1401..1429`). This differs in state timing and can consume an extra `RandomRanged(0,3)` equivalent.
6. With one unit, the actual sub-cell is one of native-valid spots `2,3,4`; the exact choice and subsequent low-byte coordinates depend on Scenario RNG state/entity state not supplied by the fixture. Rust and native both use the verified quadrant/preference tables, but Rust invokes them at different coordinates and times.
7. Rust snaps infantry facing to the next cell direction at transitions (`movement_step.rs:101..201`): broadly `0x40`, then `0x20` for NE, `0x60` for SE, then `0x40`. Native Walk recomputes a continuous sub-cell heading with `atan2` and passes it through `FacingClass::UpdateFacing` each process call.
8. Movement advances with fixed-point `effective_speed * dt`, then `step / move_dir_len` (`movement_step.rs:956..985`). The app currently supplies 22 ms simulation slices at `SIM_TICK_HZ=45`; game-speed scaling changes tick scheduling separately (`src/util/fixed_math.rs:51`; `src/app_types.rs:25..45`; `src/app_sim_tick.rs:547..609`). Exact low-byte positions and arrival tick are `UNCHECKED` without executing the dirty build and a native oracle.
9. Rust maps locomotion to its local seven-phase diagnostic/speed model and applies configured acceleration/deceleration fields (`movement_tick.rs:1278..1355`, `1944..1993`). Active Walk has a compact destination/flag state, calls Techno `Set_Speed(1.0)` while moving, and hard-stops; it has no equivalent seven-state Walk phase field.
10. Rust additionally advances an entity-ID-seeded `f32` wobble and adds a cosine screen-Y displacement (`movement_step.rs:988..1002`; `movement_tick.rs:1645..1655`). This is not part of the verified 0x3C-byte Walk locomotor mechanism and is an extra player-visible motion path.

## Infantry frame and arrival trace

- The retail sequence input is `CONS -> ConSequence -> Walk=8,6,6`. Current Rust correctly reads those three source values, but final native facing-to-SHP-frame equality remains `UNCHECKED` because the native render-index reduction is not exhaustive in the timing research.
- Rust switches to `SequenceKind::Walk` whenever `MovementTarget` exists, advances after movement using the same 22 ms slice, and uses a hardcoded **100 ms/frame** (`src/sim/animation.rs:36..45`, `343..360`, `381..510`; `src/rules/infantry_sequence.rs:25..37`, `209..240`).
- Native infantry cadence is game-frame/action-timer driven. `InfantryClass::Do_Action` loads a binary action-delay-table byte, and only a specific action subset is normalized for game speed (`TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md:356..434`). Rust does not model that action table, so a 100 ms loop is not a parity implementation even where it looks close.
- Rust arrival occurs when the path is exhausted, the final cell is reached, and any local sub-cell target is reached (`movement_tick.rs:1659..1677`). It then clears `MovementTarget`, queue/drive fields, snaps to its stored sub-cell destination, resets local phase/wobble, and clears `subcell_dest` (`movement_tick.rs:1898..1941`).
- Native Walk's fresh decompile reaches its cell step at distance **< 0x11 (17) leptons**, removes/updates occupation, shifts the 23-entry path row, calls per-cell height/commit helpers, calls `FindSubCellDest` again, and only then performs stop/mission-completion and destination-null writes. Rust has no equivalent ordered completion chain.

## Stage verdicts

| # | Stage | Verdict | Bounded result |
|---:|---|---|---|
| 1 | Stock E2/Walk/GAWALL input binding | PASS | Literal retail keys above are used by both targets. |
| 2 | Requested clear destination | PASS | Requested/head destination remains exact `(60,50)`; no Rust redirect fires. |
| 3 | Walk destination/NavCom state handoff | NOT-IMPLEMENTED | Rust installs native-shaped NavCom only for Drive, not Walk. |
| 4 | Zone precheck and literal zone IDs | UNCHECKED | One isolated wall leaves the flat region connected, but dirty-Rust/native IDs and hierarchy markers were not executed. |
| 5 | Wall classification | FAIL | Rust hard-blocks statically; native selects dynamic code `4/5/7` from wall state, ownership, weapon, and warhead. |
| 6 | Eight-neighbor order and tie row | PASS | `N,NE,E,SE,S,SW,W,NW`; scaled `[1,5,2,6,3,7,4,8]` equals native epsilons ×1000. |
| 7 | Ground diagonal corner legality | PASS | Both mechanisms test the diagonal destination without requiring both cardinal flanks clear. |
| 8 | Literal raw path and detour side | UNCHECKED | Rust north route is exact; native side was not executed and native wall-state inputs are incomplete. |
| 9 | Smoothing pass 1 | FAIL | Same fixture result may remain unchanged, but Rust substitutes a simple closure for native Can-Enter/slope/cliff ordering. |
| 10 | Smoothing pass 2 | FAIL | Current Rust drift-segment predicate collapses to zero; native optimizer is active. |
| 11 | Sub-cell selection/reservation/RNG order | FAIL | Rust selects after crossing and makes an unreserved future selection; native selects, marks, then walks. |
| 12 | Walk speed and locomotion phase | FAIL | Rust seconds-based full-speed/7-phase model differs from native `Set_Speed(1.0)` fraction and compact Walk flags. |
| 13 | Exact lepton sequence and arrival tick | UNCHECKED | Requires specified RNG/entity identity plus dirty-Rust and native execution. |
| 14 | Facing sequence | FAIL | Rust cell-quantizes/snap-writes; native continuously aims at the chosen sub-cell through `atan2`/FacingClass. |
| 15 | Literal rendered SHP frame sequence | UNCHECKED | Retail row is known; complete native facing/action-to-frame mapping was not proven for this run. |
| 16 | Walk animation cadence | NOT-IMPLEMENTED | Rust 100 ms timer does not model native infantry action-delay-table/game-frame cadence. |
| 17 | Arrival/mission/destination clearing order | NOT-IMPLEMENTED | Rust direct clear/snap omits native <17-lepton path shift, occupation, per-cell, and mission-completion chain. |
| 18 | Render-only infantry bob | FAIL | Rust adds an entity-ID-seeded f32 cosine displacement absent from the verified Walk mechanism. |

## Top five player-visible blockers

1. **Sub-cell commitment is one cell late and future choices are not reserved.** This changes the line walked inside every cell, crowd spreading, collision timing, and RNG consumption.
2. **Facing follows eight cell headings instead of the chosen sub-cell coordinate.** Turns around the wall visibly snap through E/NE/SE/E instead of following native continuous heading updates.
3. **The wall is flattened into a permanent terrain blocker.** Native infantry behavior depends on wall health/state, owner/alliance, and weapon/warhead and can transition into attack/notify/retry behavior.
4. **The second path-smoothing pass is effectively dead.** Any route that native straight-segment optimization rewrites can retain unnecessary bends in Rust.
5. **Walking presentation uses a generic 100 ms loop plus synthetic cosine bob.** Native cadence is tied to the infantry action timer and game frames, so footfall/frame phase and arrival-to-stand timing drift during ordinary movement.

## Targeted live Ghidra check (read-only)

The required live check used only read operations against open `gamemd.exe`:

- `read_memory(address="0x007F6A34", length=8, program="gamemd.exe")` returned `b0ab550080ac7500`; the dword at `0x007F6A38` is `0x0075AC80`, agreeing with the already RTTI/COL-proven Walk ILocomotion `Process` slot in `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`.
- `get_function_by_address(address="0x0075AEC0", program="gamemd.exe")` returned `WalkLocomotionClass__ProcessMovement`, body `0x0075AEC0..0x0075C23B`.
- `decompile_function(address="0x0075AEC0", program="gamemd.exe", timeout=30)` freshly confirmed the active standard branch: destination-cell `Can_Enter_Cell`; code 0 -> `FindSubCellDest`, `atan2`, Set_Facing, `Set_Speed(1.0)`; active longer-destination movement; and the `<0x11` arrival/path-shift/completion sequence described above.

## Conclusion

The user-visible diagnosis is **not just “A* picks a slightly different route.”** For this exact simple wall, current Rust chooses a plausible north diagonal detour, but the native detour side remains unverified. Independently of that unknown, the unit's movement after path selection is mechanically different on nearly every ordinary walking cell: destination authority, reservation timing, heading, speed state, animation clock, synthetic bob, and arrival clearing. Those compounded differences explain why most infantry motion can look unlike `gamemd.exe` even when the coarse cell path appears reasonable.
