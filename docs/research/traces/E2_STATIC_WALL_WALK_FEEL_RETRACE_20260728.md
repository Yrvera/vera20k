# E2 Static-Wall Walk-Feel Retrace — 2026-07-28

**Scenario:** stock YR `[E2]` Conscript, normal/unveteran owner, exact center of flat clear Temperate cell `(50,50)`, ground Z, body facing east `0x40`; one intact enemy `[GAWALL]` overlay at `(55,50)`; normal player Move to exact clear-cell center `(60,50)`.

**Status:** **RED / NOT PARITY.** The exact fixture now closes the wall-state ambiguity from the 2026-07-20 trace: native returns hard-block code `7` for the intact wall because E2's `M1Carbine -> SA` warhead has `Wall=false`. Rust also rejects that cell, so the bounded traversal result agrees. The bad feel is downstream and broader: stock-default wall-clock walking is about three times slower than gamemd, and subcell commitment, heading, occupation timing, animation cadence, synthetic bob, and arrival teardown do not implement active `WalkLocomotionClass`.

**Verdict tally:** **PASS 7 · FAIL 8 · UNCHECKED 4 · NOT-IMPLEMENTED 3** (22 bounded stages). PASS certifies only the named row.

## Scope, freshness, and evidence discipline

- Investigation only. No Rust/INI/assets were edited, no Cargo command was run, and Ghidra access was read-only. This report is the sole written artifact.
- The current source was re-read after `E2_STATIC_WALL_WALK_RETRACE_20260720.md`. Commits since that trace include ordered lifecycle authority and dormant mission authority, but the only later movement-tick change relevant to this fixture skips entities pending crush teardown; this E2 never crushes. Current source, not the old trace, is authoritative below.
- Literal native detour, subcell, lepton, and rendered-frame series were not executed against an oracle and remain `UNCHECKED`. No value was promoted from static plausibility.
- Native identities are not based on labels alone. Existing reports bind `InfantryClass::Can_Enter_Cell @ 0x0051BF90` through Infantry RTTI/COL and vtable `+0x1AC`, and bind the Walk vtable/constructor/CLSID family including `Process @ 0x0075AC80`, `ProcessMovement @ 0x0075AEC0`, and `FindSubCellDest @ 0x0075C240`. Fresh decompiles of the four critical bodies are recorded below.

## Retail inputs and command handoff

- `ini/rulesmd.ini:4327..4358`: `[E2]`, `Image=CONS`, `Speed=4`, Walk CLSID `{4A582744-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Infantry`.
- `ini/artmd.ini:138..145`, `13770..13790`: `CONS -> ConSequence`, `Walk=8,6,6`.
- `ini/rulesmd.ini:12022..12031`, `ini/artmd.ini:4122..4127`: `[GAWALL]`, `Wall=yes`, `DamageLevels=3`, one-cell overlay conversion.
- `ini/rulesmd.ini:22860..22868`, `26466..26473`: E2 primary `M1Carbine -> SA`; `[SA]` omits `Wall`. The binary constructor initializes `WarheadTypeClass+0x144` to false and `ReadINI` maps `Wall=` there (`GATE_BRIDGE_TRAVERSAL_RESOLUTION_GHIDRA_REPORT.md:99..106`).
- `src/app_context_order.rs:747..784` preserves this clear clicked point and emits `Command::Move`; `src/sim/world/world_commands.rs:128..176` accepts the living owned unit, queues Move, clears incompatible order state, and dispatches path creation.
- Native ordinary Infantry destination goes through leaf vtable `+0x480` to `InfantryClass::Set_Destination`, then `FootClass::Set_Destination_Internal @ 0x004D94B0`: clear `NavCom_Aux`, write `NavCom`, obtain target coordinates, dispatch Walk `Head_To_Coord`, then reset retry state (`NAVCOM_LIFECYCLE_GHIDRA_REPORT.md:24..32`; `FOOTCLASS_SET_DESTINATION_INTERNAL_NAVCOM_HEADTO_HANDOFF_GHIDRA_REPORT.md:40..54`).
- Current Rust installs its native-shaped `set_destination_internal_cell` only inside the Drive branch (`src/sim/movement/movement_commands.rs:545..577`). Walk receives `MovementTarget` but no equivalent `NavCom -> Head_To_Coord` handoff.

## Exact wall result

Fresh decompile of active `InfantryClass::Can_Enter_Cell @ 0x0051BF90` reconfirmed the overlay slice documented in `pathfinding/INFANTRYCLASS_CAN_ENTER_CELL_TERMINAL_OCCUPANCY_AND_WALL_GHIDRA_REPORT.md:56..93`:

```text
if overlay.Wall && ((overlay_state >> 4) != DamageLevels):
    if !CanFireOrActOnCell: return 7
    weapon = GetWeapon(0)
    if weapon == null || weapon.warhead == null || !weapon.warhead.Wall: return 7
    result = allied_wall ? 4 : 5
```

“Intact” fixes the upper nibble to `0`; `DamageLevels=3`, so the dynamic branch runs. `SA.Wall=false`, therefore native returns **7 before ownership can produce enemy-wall code 5**. Rust resolves the intact overlay to `overlay_blocks=true` (`src/map/resolved_terrain.rs:1514..1556`) and the ground grid rejects it. Thus `(55,50)` is hard-blocked in both, but Rust does not implement the native state/weapon/warhead/result-code mechanism; damaged-out wall state or a wall-capable primary weapon would diverge.

## Literal current-Rust path

Current ground A* expands `N,NE,E,SE,S,SW,W,NW` with scaled tie row `[1,5,2,6,3,7,4,8]` (`src/sim/pathfinding/core.rs:370..397`). Ground diagonals test only the destination cell; flank checks are Bridge-only (`core.rs:1248..1261`). Uniform steps add the directional tie (`core.rs:1264..1304`).

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

Direction IDs are `[2,2,2,2,1,3,2,2,2,2]`; accumulated scaled `g=10027`. North beats the symmetric south candidate because NE adds `5` and SE `6` (first detour frontier `5013` versus `5014`). This is an exact source reduction for current Rust. The native chosen side/path remains `UNCHECKED` without executing its A* over the live fixture.

### Smoothing

- Rust pass 1 (`movement_path.rs:261..284`; `path_smooth.rs:86..161`) cannot replace the `NE,SE` pair with `E,E`, because the replacement crosses blocked `(55,50)`. The path remains unchanged. Fresh native `Path_smooth_corners @ 0x0042B210` still calls its native single-segment validator under the diagonal pattern; Rust's closure is not that ordered Can-Enter/slope/cliff mechanism.
- Rust pass 2 (`path_smooth.rs:258..294`, `352..398`) is effectively dead: `find_drift_segment` compares cumulative displacement with the same endpoint displacement, making the cross product zero. Fresh native `Path_optimize_straight_segments @ 0x0042B7F0` reconfirmed active anchor splitting, validation, and reroute calls; its active chain is documented in `PATH_REROUTE_STRAIGHT_LINE_SLOPE_MARKER_RETRY_GHIDRA_REPORT.md:21..36`.
- Current Rust final path therefore equals the raw row above. Literal native post-smoothing output remains `UNCHECKED`.

## Walk execution and player-visible pace

1. `Speed=4` becomes native base integer `floor(4*256/100)=10`. Rust preserves that bounded scalar as `150 leptons/s`, i.e. `10*15` (`src/util/fixed_math.rs:338..379`; `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md:488..515`).
2. The scalar agreement does **not** produce wall-clock agreement. Stock `GameSpeed=1` (`rulesmd.ini:3026`) caps gamemd near 62.5 full logic frames/s, so an unmodified full-speed Walk spends about `10*62.5 = 625` leptons/s. Rust schedules about 63 fixed steps/s but each step advances only `150*(22/1000)=3.3` leptons (`SIM_TICK_HZ=45`, `SIM_TICK_MS=22`), about **208 leptons/s**. This is roughly **3.0× slower** at the same stock setting. The architectural mismatch and native throttle are established in `NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md:175..231`; current scheduling is `src/app_types.rs:24..46`, `src/app_sim_tick.rs:1430..1451`, and stepping is `movement_step.rs:956..985`.
3. Native Walk unmarks the current subcell, determines bridge layer, calls `FindSubCellDest -> PlaceInfantryInCell`, stores the exact longer destination, marks the selected target subcell, then walks toward it (`INFANTRY_SUBCELL_POSITIONING.md:271..302`). Fresh `0x0075AEC0` reconfirmed that active movement continuously computes `atan2` to that exact target, calls Walk `Set_Facing`, and uses the owner's current speed.
4. Rust first moves toward the next cell center. After crossing, it moves occupation old-to-new, then calls `reserve_destination_after_transition` and marks a subcell (`movement_step.rs:1301..1386`; `movement_reservation.rs:13..55`). It separately allocates a subcell in the *following* path cell, does not reserve that future choice in occupancy, overwrites `locomotor.subcell_dest`, and consumes RNG as applicable (`movement_step.rs:1401..1429`).
5. Rust's allocator has the native-shaped functional choices `{2,3,4}` and center random rotations (`movement/bump_crush.rs:40..51`, `357..409`), but invocation coordinates, reservation time, and draw order differ. The scenario supplies neither Scenario RNG state nor stable entity identity/initial functional subcell, so the literal chosen subcell series is `UNCHECKED`; the ordering disparity is independently `FAIL`.
6. Rust snaps Infantry facing to the next cell direction and aims its displacement from the current point to its locally stored subcell (`movement_step.rs:101..201`): broadly E `0x40`, NE `0x20`, SE `0x60`, E `0x40`. Native recomputes the exact subcell heading through `atan2`/FacingClass each process call. The literal headings therefore do not use the same mechanism and generally diverge around the wall.
7. Native Walk is a compact `0x3C`-byte locomotor with hard stop and destination flags, not Rust's seven-phase acceleration/deceleration diagnostic model (`WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md:53..76`, `209..260`, `739..749`; Rust `movement_tick.rs:1987..2041`).

## Occupation, SHP presentation, and arrival

- Fresh `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0` reconfirmed the arrival threshold **distance `<0x11` (17) leptons**. It unmarks occupation, shifts the 23-entry path row, updates cell/per-cell state, calls `FindSubCellDest`, marks occupation, and performs destination/mission completion in native order. Rust crosses a 256-lepton cell boundary first, updates occupancy with the old subcell, then allocates/updates the new subcell (`movement_step.rs:1301..1429`).
- Retail SHP input is exactly `Walk=8,6,6`. Rust reads this row, switches Stand→Walk while `MovementTarget` exists, and maps facing to the row (`src/sim/animation.rs:290..331`, `428..512`).
- Rust uses a generic hardcoded `DEFAULT_WALK_TICK_MS=100` (`animation.rs:36..39`, `584..613`). Native `InfantryClass::Do_Action @ 0x0051D6F0` loads a byte from the binary action-delay table, keys its timer to `g_CurrentFrameCounter`, and normalizes only six action IDs (`TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md:356..434`). Rust does not implement this cadence. The literal rendered frame-by-frame series remains `UNCHECKED`.
- Rust additionally seeds `f32` `infantry_wobble_phase` from stable entity ID, advances it, and subtracts a cosine bob from screen Y (`movement_step.rs:988..1002`; `movement_tick.rs:1664..1675`). The verified native Walk instance ends at `+0x38`/size `0x3C`; there is no Walk `+0x88` wobble field or matching process path. This is a Rust-only visual motion.
- Rust finishes once path/final-cell/local-subcell predicates pass, calls its null-destination helper for non-Drive, clears queue/`MovementTarget`/track/body-facing, snaps to stored subcell, resets phase/wobble/subcell destination (`movement_tick.rs:1682..1695`, `1931..1983`). Because Walk never received the initial native-shaped NavCom handoff and Rust omits the `<17` path-shift/per-cell/mission sequence, the arrival chain is not implemented.

## Stage verdicts

| # | Stage | Verdict | Bounded result |
|---:|---|---|---|
| 1 | Retail E2/Walk/GAWALL bindings | PASS | Exact stock type, locomotor, sequence, wall and damage-level inputs agree. |
| 2 | Exact clear-cell destination | PASS | Both preserve the requested center `(60,50)` for this isolated clear goal. |
| 3 | Input-to-Move scheduling/tick order | UNCHECKED | Rust path is identified; literal native command-frame ordering was not executed. |
| 4 | Walk NavCom/Head_To handoff | NOT-IMPLEMENTED | Rust's native-shaped handoff is Drive-only. |
| 5 | Intact E2-vs-wall traversal result | PASS | Native code `7`; Rust hard-blocks `(55,50)`. |
| 6 | Wall classification mechanism/result code | FAIL | Rust static overlay block omits state, weapon, warhead, owner, and code `4/5/7`. |
| 7 | Neighbor order/tie row | PASS | Eight directions and scaled epsilon row agree. |
| 8 | Ground diagonal corner legality | PASS | Both test the diagonal destination without cardinal flank clearance. |
| 9 | Literal raw native path/detour side | UNCHECKED | Rust north path is exact; native fixture was not executed. |
| 10 | Smoothing pass 1 mechanism | FAIL | Same local output is plausible, but Rust omits native ordered validation. |
| 11 | Straight-segment optimization | FAIL | Rust drift predicate is a no-op; native reroute pass is active. |
| 12 | Subcell selection/reservation/RNG order | FAIL | Rust selects after crossing plus unreserved lookahead; native selects/marks before walking. |
| 13 | Literal subcell and low-byte series | UNCHECKED | Scenario RNG/entity/subcell state and native execution are absent. |
| 14 | Base `Speed=4` scalar | PASS | Both reduce the unmodified base to 10 leptons per nominal/native speed frame. |
| 15 | Stock wall-clock walking pace | FAIL | About 208 Rust versus 625 native leptons/s; roughly 3× slow. |
| 16 | Walk heading/facing | FAIL | Rust snaps to cell directions; native continuously aims at exact subcell destination. |
| 17 | Occupation/cell transition order | FAIL | Rust commits cell then chooses/updates subcell; native unmarks/selects/marks around the step. |
| 18 | Walk SHP row binding | PASS | `CONS -> ConSequence -> Walk=8,6,6`. |
| 19 | Literal rendered SHP frame series | UNCHECKED | Native action/facing series was not captured. |
| 20 | Walk animation cadence | NOT-IMPLEMENTED | Generic 100 ms timer does not model the binary action-delay/game-frame timer. |
| 21 | Render-only bob | FAIL | Rust adds entity-ID-seeded cosine motion absent from verified Walk. |
| 22 | Arrival/mission/destination clear order | NOT-IMPLEMENTED | Native `<17` path/occupation/per-cell/completion chain is absent. |

## Top five player-visible failures

1. **Critical, every ordinary stock-speed move:** walking is roughly **3× too slow** in wall-clock at `GameSpeed=1`; this dominates “locomotion feels bad.”
2. **High, every entered Infantry cell:** subcells are committed late and lookahead choices are unreserved, changing the line walked, crowd spread, collision timing, and RNG consumption.
3. **High, every bend and many straight subcell approaches:** heading snaps to coarse cell directions instead of continuously tracking the exact subcell coordinate.
4. **Medium-high, whenever native straight optimization can improve a route:** Rust's second smoothing pass is effectively dead, retaining avoidable bends.
5. **Medium-high, throughout every visible walk:** generic 100 ms SHP cadence plus a synthetic cosine bob drifts footfall/frame phase and silhouette motion; arrival-to-stand ordering compounds the final hitch.

## Fresh read-only Ghidra checks

Open program: `gamemd.exe` in project `testProsjekt`. `batch_decompile` was used without renames, type writes, comments, patches, or other mutations:

- `0x0051BF90` — active Infantry wall branch and exact code-7 early return reconfirmed.
- `0x0075AEC0` — active Walk exact-subcell facing/movement and `<0x11` arrival/path-shift branch reconfirmed.
- `0x0042B210` — active corner smoothing dispatch reconfirmed.
- `0x0042B7F0` — active straight-segment anchor/validation/reroute optimizer reconfirmed.

## Conclusion

The isolated intact wall itself is not the direct mismatch: both engines reject `(55,50)`, and current Rust deterministically takes a plausible north diagonal detour. The native detour side is still unexecuted. The robust diagnosis is that ordinary Walk execution after path choice is non-native, with the stock-default frame-rate calibration making it about three times slow before the subcell, facing, occupation, animation, bob, and arrival differences are even considered.
