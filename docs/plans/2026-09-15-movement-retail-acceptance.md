# Movement: retail acceptance and increment ledger

Goal: VERA20k movement looks and works like retail Yuri's Revenge. This page
fixes what "like retail" means for movement, how each claim is evidenced, and
which increments have landed. It is a working ledger, not a parity certificate:
every status below cites the evidence it rests on and nothing more.

## Acceptance

A movement behaviour is accepted when an **ordinary, undisabled player order on
a retail map** produces the retail outcome, and the native mechanism behind it is
established from `gamemd.exe` (body, active caller, retail data). The bar is the
one frozen in [bridge-movement-matrix.md](bridge-movement-matrix.md) §0: synthetic
grids are regression ratchets, never parity; a gate whose own provenance says
"gamemd has no equivalent" is a divergence even while it produces no wrong pixel.

Player-visible criteria, in priority order (frequency × visibility):

| # | Criterion | Retail behaviour to match | Evidence instrument |
|---|---|---|---|
| A1 | Order admission | Every reachable ground destination is accepted; refusals only where retail refuses (`+0x1AC` code 7, zone reachability) | Retail-map `Command::Move` harness (`movement_bridge_retail_tests.rs`), Ghidra `+0x1AC` bodies |
| A2 | Route choice | A* over `Can_Enter_Cell` codes with the native cost table `0x0081870C`, hierarchy precheck, smoothing `0x0042B420` | `spatial_oracle.path_entry`, A* trace lanes, retail-map routes |
| A3 | Drive turning and tracks | Turn-table curves, ROT, residual budget, `Accelerates` ramp per `Process_Drive_Track 0x004B0F20` | `locomotor_track_*` oracles, `drive_track_tests.rs` |
| A4 | Column following | Followers chain onto an occupant in transit (`0x0073FA2C..FA7C`, code 2) instead of stopping | Retail-map convoy run; Ghidra |
| A5 | Blocked recovery | Retry counter, `CloseEnough`, scatter of blockers (`Scatter_Objects 0x00481670`, ten-entry cap) | Ghidra; retail-map blocked run |
| A6 | Infantry stepping | Sub-cell slots, `Walk` head/boundary/completion at `0x0075AEC0` family | `walk_*` oracles (consumed), retail-map runs |
| A7 | Crush | Crusher over `Crushable` infantry, sandbags and fences; victim lifecycle inside the crusher's slot | Ghidra; retail-map run |
| A8 | Height, slopes, bridges | Ground/deck height, ramp tilt, slope transitions; under-span and over-span routing | `ramp_height_oracle`, retail bridge matrix |
| A9 | Hover motion | Own XY integrator and vertical bob, not the Drive track | Ghidra `HoverLocomotionClass`; retail run |
| A10 | Scale | 20,000 units move without per-object whole-map work | `sim-phase-profiler`, hash-equal regression |

## Instruments

- **Retail-map harness:** `src/sim/movement/movement_bridge_retail_tests.rs`
  drives ordinary orders on `Hills.mmx`, `BayOPigs.mmx`, `Deadman.mmx` and records
  per-tick cell, z, on-bridge and occupancy. Ignored in CI; needs `config.toml`.
- **Native oracles:** `tools/spatial_oracle/*` (Unicorn, binary
  `1cdd1180…4298c`), see [tools/native_oracle.md](../../tools/native_oracle.md).
  No motion-over-time oracle exists; the executable capture programme is
  `BLOCKED` ([capture report](../research/GROUND_MOVEMENT_EXECUTABLE_NATIVE_ORACLE_CAPTURE_REPORT.md)).
- **Ghidra:** `gamemd.exe` in `testProsjekt`; UnitClass vtable `0x007F5C70`
  (`+0x1AC` at `0x007F5E1C` → `0x0073F0A0`), InfantryClass vtable `0x007EB058`
  (`+0x1AC` at `0x007EB204` → `0x0051BF90`).

## Increment ledger

Each row lands as its own reviewed PR. Status words: `LANDED` (merged, retail
harness green), `OPEN` (not started), `IN PROGRESS`.

| Increment | Criterion | Status | Evidence |
|---|---|---|---|
| I1 Ground-plane routing beneath an intact high span for Drive and Hover (matrix T1-13 / T1-15) | A1, A8 | LANDED (PR #371) | `UnitClass::Can_Enter_Cell 0x0073F0A0` never calls `CheckCellPassability 0x004834A0` (xref list: CellRect, threat scan, placement, paradrop, overlay Mark, Jumpjet touchdown); it defers height legality to `CheckBridgeTraversal 0x004D9C60` (equal-level arm admits) and selects the ground list `+0xE4` at `0x0073F51A` for a path height within one of the cell level (`0x0073F0B7..F0E8`), reading the land row at `0x0073FAB5` only on that branch. Rust: `cell_entry.rs` class arm extended from Infantry to every ground-layer mover; `TerrainCostGrid::ground_cost_at` supplies the terrain's own row beneath a deck. |
| I2a Movement pass whole-map setup skipped on object turns that build no path | A10 | LANDED (PR #372) | `pass_may_build_paths` gates the blocker plane; NavCom re-aim scan scoped to the pass. Exact: suite identical (8964/0), two retail crossings 23.55 s → 8.78 s. |
| I2b Blocker plane cached across object turns | A10 | LANDED (PR #374) | Profile (400 MTNK on Hills, 150 frames, test profile, taken under concurrent build load so only the proportions are meaningful): blocker plane ≈27% of the frame, marker-peer snapshot ≈20%, per-pass owner block-set build ≈20%, all O(N) or O(W·H) per moving object's turn. The plane is a function of the cell-marked, non-dying, non-passenger objects (cell, list layer, category, foundation), each cell's terrain-object occupation and the retained wall plane. `MovementPassCache` on `Simulation` keys it on `OccupancyGrid::generation` (membership and layer), `EntityStore::dying_epoch` (every production `dying` writer notes it; a dying object stays marked until its terminal UnInit), `ResolvedTerrainGrid::mutation_epoch` (bumped in `cell_mut`, the one path of the single terrain-object writer) and `OverlayGrid::mutation_epoch` (bumped by the public and wall-transaction mutators). Debug builds rebuild and compare on every reuse, so the test profile gains nothing and every test exercising a reuse is an exactness check; the full suite, the death-window unit test and the 400-mover run hold. A fresh critic traced every input's writers; its dying-window and un-bumped-mutator findings are applied. Measured under identical machine conditions (test profile, `scale_benchmark_many_movers_on_hills`, 400 movers, 150 frames): 52.8 ms/frame without the cache, 42.2 ms with it (cross-check off), 54.4 ms with the debug cross-check (each reuse rebuilds and compares). Earlier readings taken while other builds ran (115 → 39 ms) overstated both sides and are withdrawn; the remaining per-mover terms below dominate the frame. Not exact-cacheable and therefore left per turn: the owner block sets and the marker-peer snapshot read `movement_target`, `foot_occupation_enabled`, facing and sub-cell position, none of which bump a generation; their native shape is a live cell-list walk per mover, which needs the mover taken out of the store for its turn (next design step). |
| I3 Ally-occupant arm of `UnitClass::Can_Enter_Cell`: in-transit skip and head-on exit | A4, A5 | LANDED (PR #373) | Native `0x0073FA2C..FA7C`: an in-transit (`Foot+0x6B6 == 0`) or infantry ally is skipped unless its locomotor slot `+0xA4` answers true (Drive `0x004B4B00` / Ship `0x006A4130` `Can_Use_Track`; base `0x004B6640` always false); head-on exit `0x0073F8D4..FA26` (opposed octants, `Sqrt_Approx` distance ≤ `0x1FF`, bearing in the mover's octant → 7). Rust: `drive_track::occupant_can_use_track` (parity demonstrated: `tools.spatial_oracle.locomotor_can_use_track`, 12,320 cases), `cell_entry::classify_blocker` and `bump_crush::build_entity_block_sets` skip rule, `cell_entry::head_on_exit` on the live walk and the Drive selection lane (`MovingAllyOccupant` record). The two-tank head-on fixture's floor re-derived to the measured 116 leptons (see `repro_two_moving_vehicles_pass_through_each_other`); that number is a VERA ratchet, not a native bound. Residuals: the "moving" test is still `movement_target` rather than native's NavCom/rotating/`Is_Moving` triple; the per-owner head-on record refreshes only on occupancy-generation change. The residual first recorded here ("VERA re-sets `foot_occupation_enabled` on every crossing; Ship/Hover never clear it") was traced in I3b: Drive and Ship production turns run `track_host.rs`, which already clears the enable at the first paid point and restores it at the terminal point (`0x004B161A` / `0x004B1FEF`, Ship `0x006A0CDA` / `0x006A1632`) and gates the raw mark on it like `AddContent`; the re-setting crossing in `cell_arrival.rs` is reachable only from the non-suspending test entry (`tick_movement_with_grids`). Hover was the real gap. |
| I4 Crushable overlays fall to any crusher, with the overlay's `CrushSound=` | A7 | LANDED (PR #375) | `UnitClass::PerCellProcess 0x0073AFD4..B074` (disassembly read 2026-09-15): `Crusher=` (`+0xD28`) or ability 0x11 → overlay present → `Crushable=` (`+0x22D`) or (`Wall=` `+0x2A8` and `LocomotorType +0x5B4 == 0xC`) → `VocClass::PlayAt(OverlayType+0x1F0 CrushSound)` at the unit → `DestroyOverlay(-1)` → `RockingForwardsPerFrame += 0.02`. VERA's `apply_wall_crush_on_driveover` previously required `Wall=` and Drive for everything, so fences and sandbags (`CAFNCB/CAFNCW/CAFNCP/GASAND`, `Crushable=yes`) survived a Robot Tank and every crush was silent. Now: crushable clause added; `OverlayTypeFlags::crush_sound` parsed; `SimSoundEvent::WallCrushed` → `GameSoundEvent::WallCrushed` positional Voc. Residuals: sweep runs post-movement rather than in the per-cell entry callback; rocking tilt has no renderer; crush-clamp byte `Foot+0x6B5` unmodelled; ability-0x11 arm unmodelled (no stock grant). Follow-up recorded by the critic: the `cell_entry.rs` header (lines ~77-92) and `cell_rect.rs` comments still claim the overlay model carries no `Crushable` authority and that miners stop at sandbags; `OverlayTypeFlags::crushable` and the `CRUSHABLE` zone class contradict that prose. |
| I3b Hover occupation enable (`+0x6B6`) lifecycle | A4, A9 | IN PROGRESS | `HoverLocomotionClass::Move 0x00514310` (decompile and disassembly read 2026-09-15; re-read by the increment's critic): speed is `ftol` of the current speed (`0x00514383`); a translating frame (speed > 0) with `IsCellOccupationEnabled` (`+0xC0` → `0x0041C070`) true and a live Head_To calls `+0xF4` (`UnitClass 0x00744210`, raw 0x20 clear) and zeroes `+0x6B6`/`+0x6B7` at `0x005147CC..7DF`; the arrival arm (`Sqrt_Approx` distance to Head_To ≤ speed, `0x0051450F`) sets `+0x6B6 = 1` at `0x0051451E` at **every reached head**, and the same call re-clears it only when the next step is accepted; every next-step call (`FUN_00514F70`, prologue `0x00514F70..0x00514FB2`) first releases the reached cell's raw claim (`+0xF4`) and invalidates Head_To, and a refused step returns with the enable still set (code 7 at `0x00514711`, `+0x684` bit 7 clear), so natively only the enable survives a refusal. `Stop_Moving 0x00516320` leaves Head_To untouched, so a stopped hover still reaches the arrival arm. VERA's hover never touched the flag, so a moving hover blocked every ally (code 2) where retail skips it through the in-transit arm. Rust: `movement_tick.rs` hover arm clears the enable and the owner-plane claim on a translating frame, the deferred-refusal tail restores the enable (and, as VERA's representation, its owner-plane claim) while the hover waits at its boundary, `finalize_finished_entities` restores them at arrival; a hover whose target is dropped mid-transit re-enables on its next own turn (`restore_stopped_hover_occupation_enable`; VERA timing, native restores at the Head_To cell centre it still glides to). Tests: `hover_mover_is_in_transit_from_first_translating_frame_until_arrival`, `hover_owner_plane_claim_is_absent_in_transit_and_present_at_arrival`, `hover_refused_next_step_re_enables_occupation_while_it_waits`, `stopped_hover_left_in_transit_re_enables_occupation_on_its_next_turn`, `classify_blocker_skips_a_hover_ally_only_while_it_is_in_transit`. Residuals: `+0x6B7` unmodelled; the raw 0x20 plane is not moved by hover crossings at all (pre-existing; readers are the raw-occupation rect checks and the 5x5 marker scan); VERA's hover integrator translates on sub-lepton speeds where native (`ftol`) holds still, so the gate is "will translate"; VERA's turn-stall (I4b) holds the enable set during a >45° turn from rest where native has no stall; a refused VERA hover waits at the cell boundary, native at the cell centre, and VERA restores the enable after the deferred response where native sets it before the next-step admission (no flag or plane reader sits in that window today). Adjacent, not I3b: native code 7 from the arrival arm drops the path (`+0x5E0 = -1`), starts the path delay and re-paths at once through `FUN_005164D0`; VERA keeps the path and waits at the boundary (blocked-response mechanism). |
| I4b Hover hard-turn translation while braking | A9 | OPEN | Disclosed approximation at `movement_step.rs:309` (Hover steering, part of I7). |
| I5 Path smoothing rejection terms | A2 | OPEN | `path_smooth.rs:18` ports `0x0042B420` without three rejection terms. |
| I6 Scatter model | A5 | OPEN | `scatter.rs` header retraction; `Scatter_Objects 0x00481670`. |
| I7 Hover locomotor phases 2/3 | A9 | OPEN | `hover.rs` header. |

## Residuals carried

- I1 consequence: beneath a collapsed high span the structural stamp is kept
  while the deck is no longer elevated, so Drive is admitted on the ground plane
  by the same rule as beneath an intact span. This follows the native reading
  (`0x004D9C60` and the land row decide; `0x100` only selects the plane) but no
  retail damaged-span crossing has been run for Drive.

- Attack-move characterizations `*_attack_moved_across_*_is_currently_dropped`
  in the retail harness now fail because the stall they pin is gone on `main`;
  they must be rewritten as positive crossings (observed 2026-09-15, 4 tests).
- `bridge_tile_retail_probe` and `bridge_restamp_retail_probe` need extracted
  campaign maps (`c3y03md.map`, `xmp34u4.map`) and a `.local/` directory; they
  fail for environment reasons on a fresh worktree, not behaviour.
