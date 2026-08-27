# Bridge movement coverage matrix — frozen ledger

**Status: FROZEN.** This is the ledger of an active autonomous program. Adopt it; do not rebuild it.

Freeze point: the three inventory lanes read the tree at HEAD `cf91caa3` on
`feature/opents-cross-verify`. Every `file:line` below is that HEAD. Every citation in this
document is **second-hand from those three lanes** — no line was re-opened while writing the
ledger, and **no lane ran a crossing**. Where a lane wrote "pinned by", it had read an assertion
body, not observed a pass.

---

## 0. Operating rules for this program

A fresh session must read these before touching a row.

1. **The bar is gamemd, read through Ghidra.** Every behavioural claim carries a live decompile
   citation or a named research doc, or it is written `UNCHECKED`. Prose never upgrades a status.
   A VERA-internal gate with "gamemd has no equivalent" in its own provenance block is a
   divergence from the bar whether or not it currently produces a wrong pixel.
2. **A fix counts only when an ordinary undisabled order crosses on a real retail map.** Not when
   it compiles, not when a gate is bypassed by a test helper, not when a synthetic grid agrees.
   "Ordinary undisabled" means: a normal `Command::Move` (or the row's named order source) issued
   with every production gate ON, on a map loaded from the retail root.
3. **Unit tests on synthetic grids prove self-consistency, never parity.** They are regression
   ratchets. They can move a row out of `KNOWN-BROKEN`; they can never move a row into
   `VERIFIED-OK`.
4. **The ratchet comes before any row is worked.** The ratchet is the retail harness plus a bridge
   replay golden. Until both exist and are green, no row is worked — a fix landed without the
   ratchet cannot be shown not to have broken the two crossings that already sort-of work.
5. **One row per slice.** A slice touches exactly one matrix row, names it by id, and ends with
   the literal `test result:` line in its report.
6. **A fresh critic judges each slice.** The critic reads the diff and the literal test output.
   The critic does not read, and is not given, the builder's reasoning. A slice the critic cannot
   verify from the diff and the output is not landed.
7. **Re-entry is idempotent.** A session picking this up adopts this ledger as-is, re-derives the
   frontier from each row's own evidence column, and **re-traces nothing**. If a row says
   UNCHECKED, that is the finding — do not re-investigate to reconfirm it, measure it.

**`OI-31` — RESOLVED, was never another session's.** The inventory lane observed this repository
mid-edit and reported the retail harness as not compiling, because
`issue_crossing_order_without_hierarchy_gate` had been deleted while a second callsite still
referenced it. That edit was this program's own ratchet work and it landed at `c3c1cda2`: the
helper is gone deliberately, both the crossing and the control move now go through
`issue_ordinary_move`, and the harness compiles and passes. Nothing is blocked on it. Recorded
rather than deleted because the lesson generalises — an inventory lane reading a working tree
sees whatever is uncommitted at that instant, so a "live blocker" it reports is a claim about a
moment, not about the branch.

---

## 1. Dispositions

| Disposition | Meaning |
|---|---|
| `UNCHECKED` | No evidence either way. **The default.** Every row starts here. |
| `VERIFIED-OK` | An ordinary undisabled order was **observed** to behave correctly on a real retail map, with the citation. The only green state. Unit tests alone can never produce it. |
| `KNOWN-BROKEN` | A defect is confirmed, with citation and linked open-item id. |
| `RESIDUAL` | Recorded and deliberately not fixed. Must carry trigger, player effect, ordinary-play frequency. |
| `NOT-APPLICABLE` | Cannot occur in stock YR. One line of why. Lives in §6, not in the matrix. |

**Two rows are `VERIFIED-OK`: T1-03 and T1-04.** Both were settled on 2026-08-27 at `c3c1cda2`,
after this section was first written and while it still said zero.

The correction matters, so here is what changed rather than just the number. When the inventory
lanes read the tree, `tank_crosses_bay_of_pigs_high_bridge_at_deck_height` and
`tank_crosses_hills_high_bridge_at_deck_height` did issue their orders through a
hierarchy-gate-disabling helper, which rule 2 excludes — the lane was right. `c3c1cda2` deleted
that helper along with the defect it worked around: both the crossing and the control move now
go through `issue_ordinary_move`, and a refused order panics instead of silently re-issuing a
widened search. The tests were then run against the real install with nothing disabled —
`test result: ok. 4 passed; 0 failed` — which is the first crossing this project has recorded
under rule 2.

They remain `#[ignore]`d and require `RA2_DIR`; that is a property of needing retail assets, not
a disabled order, and it is why the always-on guard is the golden
(`bridge_parity_harness_tests.rs`, `5600f494`) rather than these.

---

## 2. Collapsed dimensions

Stated once so no slice re-derives them and no row is duplicated.

| Collapsed | Evidence |
|---|---|
| **High concrete × high wood** → one row each | Both resolve through `high_bridge_stamp_for_overlay` (`src/map/bridge_facts.rs:225-233`) to the same stamp; `BridgeStampFamily::{Nesw,Nwse}` names the two native setters, not the materials, and every consumer only tests `family != None`. Concrete/wood separate solely in theater tileset windows (`bridge_topology.rs:186-210`) and the CABHUT repair predicate (`resolved_terrain.rs:2280-2281`) — neither is movement. Fixtures happen to cover both anyway: BayOPigs is concrete, Hills is wood. |
| **`Nesw` × `Nwse` stamp family** → not a relation | Same evidence; `apply_modeled_cellclass_bridge_slot` is family-agnostic. |
| **Low wood × low urban/concrete** → one row each | Same `TubeClass` movement path; they differ only in the overlay damage bands (`bridge_specs.rs:96-146`). Concrete-low has no loose fixture anyway (§5). |
| **Walk order-source rows** → folded into the Drive order-source rows | Walk and Drive share the planner entry: both are admitted by `supports_layered_bridge_pathing` (`movement_path.rs:91-95`) and both reach `resolve_cell_transition_bridge_state`. Order source is selected above that. Hover does **not** fold — it is excluded from the layered builder, so it keeps its own attack-move row. |
| **Teleport warp-onto-deck × Drive-piggyback warp** → one row | `set_destination_for_teleporter_entity` (`movement_commands.rs:180-206`) converts the piggyback case into an ordinary Drive move, so it inherits the Drive rows wholesale; only the pure warp arm is distinct. |
| **`onto` × `off` for low spans** → one `along` row | Low decks sit at the surrounding terrain level (`deck_level = level`, `resolved_terrain.rs:3571-3593`); there is no height event at either end. `tube_movement.rs` owns the whole object turn and writes `position.z = cell.level` with no deck term. One crossing exercises all three relations. |

---

## 3. THE MATRIX

Order sources: `move` = player Move click · `atk-mv` = attack-move · `AI` = AI team script ·
`bump` = scatter / repath-after-block · `retreat` = flee/retreat mission · `guard` = guard-area
pursuit.

Fixture shorthand — **LOOSE** = the map file sits in the retail root and
`headless_scenario::load` can name it today. **MIX** = it must be extracted first; record the
extraction as a slice prerequisite. **SYNTHETIC-NEEDED** = no retail fixture exists at all.

### Tier 1 — ordinary skirmish traffic

Drive, Walk and Hover, on and under intact high and low spans, under a player Move order.
Rows are ordered by expected player-visibility × frequency within the tier.

| ID | Loc | Kind | Rel | Src | Disp | Fixture | Open | The one check that settles it |
|---|---|---|---|---|---|---|---|---|
| T1-01 | Hover | high intact | onto | move | **VERIFIED-OK** | BayOPigs.mmx `(111,152)`→`(111,151)`; Hills.mmx `(98,74)`→`(97,74)` | OI-06 (=L1+L2), closed | **Fixed and settled 2026-08-27.** Cause: `supports_layered_bridge_pathing` excluded Hover, so the mover got a flat path with `Ground` stamped on every node including the deck, and the crossing loop's terrain test then asked whether it could enter the riverbed *under* the span. Seven refusals measured, all `layer_walkable`, zero `bridge_traversal` — the bridge check passed every time and was simply never asked about the right plane. The order was never dropped, so the unit drove into the abutment and was snapped back to cell centre ~13×/s indefinitely. Fix admits Hover to the whitelist; the native gate (`FootClass::Find_Path` `vtable+0x2CC`) is per-*class* and all four Hover types are `[VehicleTypes]`, the same `UnitClass` as the whitelisted Drive vehicles, so it cannot distinguish them. Settled by `hover_tank_crosses_bay_of_pigs_high_bridge_at_deck_height` and `hover_tank_crosses_hills_high_bridge_at_deck_height`, ordinary undisabled orders, `test result: ok. 5 passed; 0 failed`. Diagnosis: `docs/plans/bridge-hover-t1-01-diagnosis.md`. |
| T1-02 | Hover | high intact | along | move | **KNOWN-BROKEN** | BayOPigs.mmx, `(111,143)` mid-span, LOOSE | OI-25 (=L1), L5 | Log `terrain_layer` and `on_bridge` for a ROBO on a deck cell: `build_flat_fallback_layers` (`movement_path.rs:539-558`) returns all-`Ground`, and `core.rs:683-702` copies it verbatim into the runtime legality test, so the deck is checked on the ground plane. L5: `hover_vertical_tick`'s `climbing` input (`movement_tick.rs:2851-2862`) reads raw `ground_level`, so the lift controller sees no 4-level rise. |
| T1-03 | Drive | high intact | onto | move | **VERIFIED-OK** | BayOPigs.mmx `(111,152)`→`(111,151)`; Hills.mmx `(98,74)`→`(97,74)`, both LOOSE | OI-04, OI-12 | **Settled 2026-08-27 at `c3c1cda2`.** The named check was run: hierarchy gate ON, no disabled helper (it no longer exists), ordinary `Command::Move` — which the harness now panics on if refused. `tank_crosses_bay_of_pigs_high_bridge_at_deck_height` and `tank_crosses_hills_high_bridge_at_deck_height`, literal `test result: ok. 4 passed; 0 failed`. Every structural deck frame asserts `on_bridge` set and `z == terrain + BRIDGE_DECK_LEVEL_DELTA`. |
| T1-04 | Drive | high intact | along | move | **VERIFIED-OK** | BayOPigs.mmx col x=111, 17 deck cells; Hills.mmx row y=74, 22 cells | OI-10, OI-11, OI-19 | **Settled 2026-08-27, same run as T1-03.** The harness asserts a non-empty mid-span frame set and the height invariant on *every* deck frame, over 17 and 22 deck cells. Caveat kept honest: "mid-span" is the harness's own definition, not this row's "≥3 from either end" wording; the two agree on these spans but a future narrow span could separate them. Stands for the corpus of spans with ≥3 anchors. |
| T1-05 | Drive | high intact | off | move | UNCHECKED | BayOPigs.mmx `(111,135)`→`(111,134)` | OI-20, OI-21 | **Half observed, deliberately not upgraded.** The same run shows the exit frame clearing `on_bridge` with `bridge_occupancy` gone. The other half of the named check — that the occupancy entry lands on the Ground plane — is asserted nowhere: `movement_tick.rs:188-232` hardcodes `MovementLayer::Ground` for the terminal commit, which is right off-deck and wrong for a track that *ends* on the deck, and no test looks. A row is not VERIFIED-OK because most of it passed. |
| T1-06 | Walk | high intact | onto | move | UNCHECKED | Hills.mmx `(98,74)`→`(97,74)`, LOOSE | — | Drive an `E1` across; no infantry has ever been driven across a span in this project. The harness drives only `MTNK` and `ROBO` (`movement_bridge_retail_tests.rs:1099,1107,1120`). The cited native twin `WalkLocomotionClass::ProcessMovement 0x0075C154-0x0075C199` has never been exercised. |
| T1-07 | Walk | high intact | along | move | UNCHECKED | Hills.mmx row y=74 | OI-19 | Same run; additionally assert the **sub-cell reservation** arm (`movement_step.rs:2229-2231, 2255-2294`) picks the Bridge layer — infantry are the only movers that take it, and it is the one Walk-specific bridge code path. |
| T1-08 | Walk | high intact | off | move | UNCHECKED | Hills.mmx `(76,74)`→`(75,74)` | — | Same run; exit frame clears `on_bridge`, sub-cell slot released on the Bridge plane. |
| T1-09 | Drive | low intact | along | move | UNCHECKED | Lostlake.mmx row y=117, x=39..51 (13 cells), LOOSE — already in the harness map list | OI-07 | Order an `MTNK` from x=38 to x=52 on y=117 and assert it reaches. Low spans are on 32 % of stock maps and `tube_movement.rs` carries **zero** bridge provenance comments — that is "not examined", not "clean". Stands for the collapsed onto/along/off set and for the wood/concrete pair. |
| T1-10 | Walk | low intact | along | move | UNCHECKED | Lostlake.mmx y=117 x=39..51 | OI-07 | Same order with an `E1`. `tube_movement.rs:103-104` gates on category `Unit \| Infantry`, so infantry is a genuinely separate arm, not a collapse of T1-09. |
| T1-11 | Hover | low intact | along | move | UNCHECKED | Lostlake.mmx y=117 x=39..51 | — | Same order with a `ROBO`. Note `tube_movement.rs` gates on **category and layer**, not locomotor kind, so Hover is *expected* to work here where it fails on high spans — that asymmetry is the check. |
| T1-12 | Hover | high intact | off | move | UNCHECKED | BayOPigs.mmx `(111,135)`→`(111,134)` | OI-06 | Unreachable until T1-01 clears. Listed so it is not mistaken for clean. |
| T1-13 | Drive | high intact | under | move | UNCHECKED · **demotion-pending** | Hills.mmx, ground plane at `(76,74)…(97,74)`, terrain level 2, LOOSE | OI-14 (=R5) | Add `facts.ground_walkable` to `print_inventory` (`movement_bridge_retail_tests.rs:196-228`) and re-run `retail_high_bridge_inventory`. If the field is true under Hills' span, order an `MTNK` along the ground plane beneath it and see whether it drives under an intact deck. **If the field is false, T1-13/14/15 demote to Tier 3** — see §7 gap 1. |
| T1-14 | Walk | high intact | under | move | UNCHECKED · **demotion-pending** | Hills.mmx ground under y=74 (land); BayOPigs.mmx x=111 (water) for the amphibious sub-case | OI-14 | Same print, then an `E1` under Hills' span. Stands for the amphibious arm too — `GHOST`/`TANY`/`YURIPR` are `SpeedType=Amphibious`, so they can be under BayOPigs' span on water while ordinary infantry cannot. |
| T1-15 | Hover | high intact | under | move | UNCHECKED · **demotion-pending** | BayOPigs.mmx x=111 water riverbed under the span | OI-14, OI-06 | Order a `ROBO` (AmphibiousDestroyer) along the riverbed under the span. This is the row that decides whether L1 is a nuisance or the reason ROBO never reached the deck: if a structural deck cell is ground-plane-passable for an amphibious mover, the flat-branch planner routes it *under* the span rather than refusing. |

### Tier 2 — ramps, damaged spans, and non-player order sources

| ID | Loc | Kind | Rel | Src | Disp | Fixture | Open | The one check that settles it |
|---|---|---|---|---|---|---|---|---|
| T2-01 | Drive | high intact | onto | atk-mv | UNCHECKED | BayOPigs.mmx x=111 span | OI-15 | Attack-move a group across the span and assert (a) it crosses, (b) it does **not** select a bridge repair hut as a path obstacle. VERA has four consumers of `bridge_repair_hut` and none is a passability or A* cost test; gamemd returns `MOVE_NO` unconditionally (`0x0051C62F` / `0x0073FC00`). If VERA prices the hut as ordinary destroyable, an attack-moving force cuts its own bridge. Stands for Walk. |
| T2-02 | Drive | high intact | along | AI | UNCHECKED | Hills.mmx y=74 span | OI-08 | Scripted team move across the span. `ObjectClass::ShouldBeOnBridge 0x005F6A70` feeds zone reachability from the **destination** height; VERA passes the raw mover layer (`movement_bridge.rs:1105-1108`, `#[ignore]`d, panics "unimplemented"). A team goal on the far side is exactly the ≥4-level destination that triggers it. |
| T2-03 | Drive | high intact | along | bump | UNCHECKED | BayOPigs.mmx x=111, block the span mid-crossing with a second unit | — | Block a deck cell in front of a crossing tank and let `try_repath_after_block` (`movement_path.rs:561`) run. It is untested on a deck anywhere in the tree. Assert the repath stays on the Bridge layer instead of dropping to Ground. |
| T2-04 | Hover | high intact | onto | atk-mv | UNCHECKED | BayOPigs.mmx x=111 | OI-06 | Attack-move a `ROBO` at a target across the span. Does **not** fold into T2-01: Hover takes the flat branch, where `is_bridge_only_goal` (`movement_path.rs:119-121`) can drop the order outright, and an attack-move's goal selection may or may not reach that predicate. |
| T2-05 | Drive | high ramp/bridgehead | onto | move | UNCHECKED | BayOPigs.mmx `(111,151)` and `(111,135)`; Hills.mmx `(97,74)`/`(76,74)` — the anchor / F1 / Opposite slots that carry `0x200` | OI-11, OI-12, OI-04 | Log `rejected_reason` for a step onto a bridgehead cell. Three separate ports converge here: `cell_entry.rs:442-452` short-circuits the object-list and speed-row half of the entry test on any transition cell; `core.rs:638-644` makes whole-deck `0x200` load-bearing for the diff-0 arm; `is_at_bridge_level` reads `bridge_walkable` where native reads `0x100`. |
| T2-06 | Walk | high ramp/bridgehead | onto | move | UNCHECKED | Hills.mmx `(97,74)` | OI-12 | Same as T2-05 with an `E1`; infantry additionally carry the sub-cell reservation across the transition cell. |
| T2-07 | Drive | low intact | along | AI | **KNOWN-BROKEN** | Lostlake.mmx y=117; any map with an authored `[Tubes]` section | OI-07 | `tube_hierarchy_pairs_are_unregistered` (`zone_build.rs:3041-3045`) `panic!`s "unimplemented: tube branch of `RegisterBridgeOrTubeHierarchyPairs 0x00582D70`". The high-bridge half of the same function **is** ported. Settled by implementing the tube branch and asserting a long cross-map order routes through the tube instead of around it. |
| T2-08 | Drive | high damaged (body byte 6) | along | move | UNCHECKED | **MIX**: `all06umd.map` `(37,34)-(41,34)`, `c3y01md.map` `(59,99)-(65,99)`, `xmp04t8.map` `(163,192)-(167,192)`. No loose map carries byte 6. | — | Extract one MIX map to the retail root, then order a crossing over the damaged body cell. `Damaged(NS)` is 7 cells across the whole 184-map corpus — rare but author-placed and real. |
| T2-09 | Drive | high partially collapsed | onto | move | UNCHECKED | Deadman.mmx y=41: intact `(55,41)`, collapse stub `(56,41)` state 8, gap x=57..60, stub `(61,41)` state 7. LOOSE | OI-28 | Order across the gap. Correct behaviour is a refusal or a route around, never a drive into the gap. Deadman and YuriPlot are the only loose maps with author-placed collapse states. |
| T2-10 | Drive | low damaged/destroyed | along | move | UNCHECKED | Shrapnel.mmx `(106..108, 46..48)` and `(114..116, 58..60)` — ids 82/100/81 and 90/101/91, where `0x64/0x65` are the terminal sinks of the wood damage table. LOOSE | OI-29e | Order a crossing through the destroyed middle strip. This is the only loose map in the corpus with author-placed non-pristine low-bridge ids. |
| T2-11 | Drive | high repaired | along | move | **KNOWN-BROKEN** | Any high-wood map with a CABHUT; Hills.mmx or Carville.mmx | OI-29 | `bridge_repair_terrain_restoration_is_unported` (`bridge_state/tests.rs:2708-2712`) is `#[ignore]`d: `cell+0x11B += 4` is never re-applied and `ValidateBridgeZones 0x0056DB70` + `RebuildZoneConnectivity` never run. Repairs are uncommon per match but the effect **persists for the rest of the game** and both players then path over that span. Blocked: the branch selectors are runtime BSS theater constants that cannot be read statically — recorded rather than guessed. |
| T2-12 | Drive | high intact | off | retreat | UNCHECKED | BayOPigs.mmx `(111,135)` under fire | — | Damage a unit mid-span so it takes the retreat mission and observe the exit. The concern is not the order source itself but that a mission-driven repath re-enters `movement_commands.rs:288-289`, which hardcodes `path_layers: vec![current_layer, current_layer]` for direct two-node targets (OI-24). |
| T2-13 | Drive | high intact | along | guard | UNCHECKED | Hills.mmx y=74 | — | Put a unit on guard beside a span and give it a target across; assert the pursuit crosses rather than stalling at the bridgehead. No observation exists for any order source other than a single ordinary Move. |
| T2-14 | Drive+Walk | high intact | alongside | move | **RESIDUAL** | BayOPigs.mmx `x ∈ {109,110,112,113}` at `y=143` — the F2/Opposite structural slots that are stamped but off the drive line | OI-30c | **Trigger:** any ground unit standing on a structural bridge cell that is not the deck drive line. **Player effect:** the under-bridge SHP pass writes depth (`merge_passes.rs:215`, `LessEqual`, write ON) where the ordinary ground pass does not (`Always`, no write), so an infantryman one cell inside the structural band can clip later voxel units while the same man one cell outside does not. **Frequency:** uncommon, but gated on nothing rare — any bridged map, any unit walking past the abutment. Render-side; deliberately not fixed while sim rows are open. |

### Tier 3 — Ship, Teleport, and destroyed-span edges

Expected to end largely as residuals. Kept listed so they are not rediscovered.

| ID | Loc | Kind | Rel | Src | Disp | Fixture | Open | The one check that settles it |
|---|---|---|---|---|---|---|---|---|
| T3-01 | Teleport | high intact | onto | move | **KNOWN-BROKEN** | BayOPigs.mmx `(111,143)` as a warp destination | L3 (new) | `TeleportPhase::Relocate` (`teleport_movement.rs:348-390`) writes `rx/ry`, centres the sub-cell, clears `exact_z_leptons` — and **never writes `position.z`, never touches `on_bridge`**, and passes the *source* `loco.layer` into `occupancy.move_entity`. A `CLEG`/`CMIN` warping onto a deck keeps the riverbed height. Settled by adding the deck term and asserting `z == terrain+4, on_bridge=true` after a warp onto a structural cell. Stands for the Drive-piggyback arm, which converts to an ordinary Drive move. |
| T3-02 | Teleport | high intact | off | move | **KNOWN-BROKEN** | BayOPigs.mmx `(111,143)` → a ground cell | L3 | Same defect, opposite sign: a unit warping *off* a deck keeps `on_bridge=true` and deck height, so it floats 4 levels above the ground. Same fix, separate assertion. |
| T3-03 | Ship | high intact | under | move | UNCHECKED | BayOPigs.mmx x=111 column, water riverbed under the 17-cell span. LOOSE | — | Order a `DEST` under the span and assert it passes. Ship shares the drive track with Drive (`movement_step.rs:48-53`) and `BRIDGE_Z_OFFSET` (416 leptons, `movement_bridge.rs:161`) widens its braking window when its current cell carries a deck. A deck row for Ship is excluded (X-04), so this is Ship's whole column. |
| T3-04 | Drive | high intact→collapsing | along | move | UNCHECKED | BayOPigs.mmx x=111 with a column crossing while the span is cut | OI-29c, OI-29d | Cut the span with traffic on it. `drop_in_bridge_deck_entities` (`bridge_orchestrator.rs:1760`) snaps deck occupants to ground level with health untouched, and `kill_ground_occupants_at` (`:1312`) explicitly filters `!e.is_on_bridge_layer()`. **LEAD only — TS-sourced, no gamemd citation exists.** The check is a Ghidra pass on the native collapse occupant sweep before any Rust is written. |
| T3-05 | Drive | low intact→destroyed | along | move | UNCHECKED | Shrapnel.mmx low span, destroyed under a crossing unit | OI-29e | Same shape one tier down: `walker.rs:509` produces `StateOutcome::Absorbed` for intermediate stages with no occupant effect and no `Kill_Illegal_Occupiers` analogue. Also LEAD-only. Ranked below T3-04 because it needs the low-bridge damage path working first (T2-10). |
| T3-06 | Walk | high destroyed | onto | move | UNCHECKED | YuriPlot.mmx stub `(111,78)` state 16 / `(111,83)` state 17. LOOSE | OI-28 | Order an `E1` onto a surviving collapse stub. Correct behaviour is whatever gamemd's re-stamp leaves passable — and VERA never re-stamps (`bridge_edge_tile_rewrite_after_collapse_is_unported`, `bridge_state/tests.rs:2654-2658`), so its predicates keep reading a span the engine has already unstamped. Recorded correction: this is **not** a tile-art gap; the writers touch bits the zone build, both walkers and `CheckBridgeTraversal 0x004D9C60` all read. |
| T3-07 | Drive | high intact | under | move | UNCHECKED | **MIX**: `all02umd.map` NEWURBAN elevated freeway over city streets (18 ground-unit / 22 building / 21 ore shared under-deck template pairs) | OI-14 | The city-overpass variant of T1-13. Only worth extracting if T1-13's `ground_walkable` print comes back true and the Hills case turns out to be map-specific. `c1a02md`, `rushhr`, `manhatta`, `austintx`, `downtown`, `c4w01md` are the same family. |

**Row count: 36.** Tier 1: 15 · Tier 2: 14 · Tier 3: 7.

---

## 4. Non-order entries onto a deck — recorded, not matrix rows

These are real defects in the same family as T3-01, but none of the six order sources applies to
them, so they are recorded here rather than padding the matrix.

| ID | Item | Disposition | Evidence |
|---|---|---|---|
| N-01 | `spawn_object` has no bridge-deck term (`world_spawn.rs:354`, `height_map…unwrap_or(0)`) | **KNOWN-BROKEN** (OI-22) | MEASURED: spawning a Grizzly at BayOPigs `(111,143)` gives `z=1, on_bridge=false` on a cell whose deck is at 5 — literally under the bridge — and it then paths nowhere in either gate state. |
| N-02 | Map-placed units on a deck are placed at riverbed height | **KNOWN-BROKEN** (OI-22) | `austintx.map` (MIX) is the only map in 184 setting `[Units]` field 10 (`High`) to `1`; all **76** such units stand on stamped structural cells. `parse_common_fields` (`map/entities.rs:280-330`) reads fields 0-5 and 8-9 and **never reads field 10**. Field identity is inferred, not read from a VERA parser — but no unit with `High=0` lands on a structural cell anywhere in the corpus. |
| N-03 | No air-layer landing path writes bridge state | UNCHECKED (L4) | `air_movement.rs`, `jumpjet_movement.rs` and `parachute_descent.rs` contain **zero** occurrences of "bridge". A paradropped GI or a landing SHAD on a deck cell gets N-01's symptom. Deck landings are rare; paradrops are not. |

---

## 5. Fixtures — the frozen index

| Fixture | Map | Cells | Deck / approach level | State |
|---|---|---|---|---|
| High concrete, intact | **BayOPigs.mmx** LOOSE, URBAN | anchors `(112,135)-(112,151)`; engine drive line `(111,134)`→`(111,152)`, 17 deck cells. Second span anchors `(160,135)-(160,151)` | terrain 1, deck-Z 5, approach 5 | overlay 25, state byte 9 = `Healthy(EW)`. **Riverbed is water** (0 of 9 under-deck templates ore-capable). VERIFIED-BY-ENGINE. |
| High wood, intact, **land under** | **Hills.mmx** LOOSE, TEMPERATE | anchors `(76,75)-(97,75)`; drive line `(75,74)`→`(98,74)`, 22 deck cells | terrain 2, deck-Z 6, approach 6 | overlay 237, byte 0. 10 of 13 under-deck templates carry ore elsewhere on the map ⇒ ground is land at level 2. VERIFIED-BY-ENGINE for the span; **the land claim is map-byte-derived, engine-UNCHECKED** (§7 gap 1). |
| High, destroyed at load | **YuriPlot.mmx** LOOSE | stubs `(111,78)`/`(111,83)`, `(135,131)`/`(135,137)`, `(79,110)`/`(84,110)` | — | states 16/17 and 8/7 = `PartialCollapse`; no overlay in the gap. |
| High, partially collapsed | **Deadman.mmx** LOOSE | y=41 and y=90 rows; intact + stub + gap + stub | terrain 0, approach 4 | states 0 / 8 / 7. |
| High, `Damaged` body | **MIX only** | `all06umd.map`, `c3y01md.map`, `xmp04t8.map` | — | byte 6; 7 cells corpus-wide. Extraction is a slice prerequisite. |
| Low wood, intact | **Lostlake.mmx** LOOSE (already in the harness map list) | y=117, x=39..51 (13 cells) | level 1 throughout, no `+4` | end id 94, body 74-77. Alternatives: Killer.mmx x=93 (22), Shrapnel.mmx y=74 (13), EB3.mmx y=101 (13). |
| Low wood, damaged/destroyed | **Shrapnel.mmx** LOOSE, SNOW | `(106..108,46..48)` ids 82/100/81; `(114..116,58..60)` ids 90/101/91 | — | `0x64`/`0x65` are the terminal damage sinks. Only loose map with author-placed damaged low bridges. |
| Low concrete (`LOBRDB*`) | **MIX only** — `c2s02md.map` x=129 (25 cells), `nearoref.map` x=72 (20), `xmp13*.map` | — | Collapsed into the low-wood rows (§2); listed only if a family-specific defect appears. |
| Units pre-placed on a deck | **austintx.map** MIX, NEWURBAN | 76 civilian vehicles on four freeway spans | terrain 0, approach 4 | N-02's fixture. |
| Under-deck passable, city variant | **all02umd.map** MIX, NEWURBAN | elevated freeway over streets | — | T3-07. |

**Corpus context** (full sweep, 184 maps, 0 parse failures): 61 maps carry a high span, 59 carry a
low span, 15 carry both, 79 carry neither. 258 of 301 anchor runs are ≥3 anchors, i.e. long enough
to have a mid-span. Author-damaged high bridges: 58 cells across 15 maps. Bridge behaviour is
on-screen in ordinary skirmish, not an edge case.

---

## 6. Excluded — combinations that cannot occur in stock YR

One line each. These are **not** untested rows; do not add them.

| ID | Excluded combination | Why |
|---|---|---|
| X-01 | Fly (ORCA, HORNET, BEAG, BPLN, SPYP, PDPLANE, CARGOPLANE, ASW) × any bridge relation | `MovementZone=Fly` ⇒ `can_use_bridges() == false`; air-layer entities are filtered out of the ground movement loop at `movement_tick.rs:1657-1661` before it starts, and `air_movement.rs` contains zero "bridge". |
| X-02 | Jumpjet (Rocketeer, Kirov, DISK, SHAD, HIND, SCHP, SCHD, LUNR) × any bridge relation | Same `Air`-layer exclusion. The `[LocomotorBeam]` warhead (Magnetron lift) is the one way a ground unit could acquire a Jumpjet locomotor, and `combat_weapon.rs:161-182` records that branch as unimplemented — so it cannot occur today either. |
| X-03 | Rocket (V3ROCKET, DMISL, CMISL) × anything | A Rocket-locomotor object is a projectile and is never given a move order. |
| X-04 | Ship × any **on-deck** relation | `MovementZone::can_use_bridges()` (`locomotor_type.rs:377-386`) returns false for `Water`/`WaterBeach`. A deck row for Ship is impossible, not untested. |
| X-05 | Any locomotor × **under** a low span | Low decks sit at the surrounding terrain level (`resolved_terrain.rs:3571-3593`); `bridge_walkable` is never set for a low bridge. There is no space beneath — the crossing *is* the ground plane. |
| X-06 | Mech locomotor × anything | No CLSID resolves to Mech (`locomotor_type.rs:75-99`); the GUID appears only in commented `;origional -` lines. It is nonetheless in the `supports_layered_bridge_pathing` whitelist (`movement_path.rs:93`) — a dead branch, so that whitelist is a **two**-locomotor list, not three. |
| X-07 | Tunnel / subterranean × anything | TS legacy, absent from RA2/YR, no CLSID, pinned absent from retail INIs by `dormant_clsids_absent_from_retail_inis`. **Not the same thing as low-bridge `TubeClass` movement**, which is live and has rows T1-09/10/11, T2-07, T2-10, T3-05. |
| X-08 | DropPod / Parachute locomotor × anything | Inert; no CLSID. Paradrop descent carries its own `parachute_descent` state instead of displacing the locomotor (that surface is N-03, not a locomotor row). |
| X-09 | Train / rail bridges | YR ships no rail bridges. The ancestor's `Can_Repair_Bridge` has a bridge-or-train iso-tile match; match the bridge half only. |
| X-10 | Concrete × wood as separate movement rows; `Nesw` × `Nwse` as a relation | Collapsed per §2 — the family field has no behavioural consumer. |
| X-11 | Low-bridge end pieces `LOBRDGE1-4` (ids 122-125) and `LOBRDGB1-4` (233-236) | **Zero cells in the entire 184-map corpus.** TS heritage. |
| X-12 | `IsAvoidBridges` bridge-cost multipliers; threat-avoidance HS scoring; `IsTrain`/`IsPassive` A* gates | Never set true anywhere in the ancestor and absent from stock `rulesmd.ini`. Do not create a bridge-avoidance column. |
| X-13 | Fog of war × bridges | `FogOfWar` defaults false; shroud only. No bridge interaction exists to test. |
| X-14 | High-bridge state `0x0F = Damaged(EW)` and `Healthy` variants 1/2/3 | Absent from the entire corpus; reachable only through the runtime state machine. Synthetic-only, and the renderer re-derives healthy jitter anyway — not a movement row. |

---

## 7. FROZEN SCOPE

The matrix above is the program's whole scope. **A later discovery is a residual unless
correctness requires it.** Concretely:

- A defect found while working a row is recorded against that row as a linked open item and
  deferred, unless the row cannot be settled without it.
- New locomotor/kind/relation/order-source combinations are **not** added. If one turns out to be
  load-bearing, it replaces a row rather than extending the matrix, and the replacement is stated
  in the slice report.
- Refactors, cleanups and style edits are out of scope inside a slice, per `ENGINE.md`.
- The excluded set (§6) is closed. Re-deriving an exclusion is a symptom that the ledger was not
  adopted.

---

## 8. EVIDENCE GAPS

What **no lane has established.** These are the program's blind spots, not its backlog.

1. **Whether any retail map has passable ground under a high span.** Hills.mmx is
   **PLAUSIBLE-STRONG, engine-UNCHECKED**: 10 of 13 distinct terrain templates directly under its
   22-cell span carry ore elsewhere on the same map, and the method's two controls (BayOPigs and
   Grinder, both known-water riverbeds) correctly returned zero. But this is terrain-template
   identity, not an engine land-type read. The settling action is one line: add
   `facts.ground_walkable` to `print_inventory` (`movement_bridge_retail_tests.rs:196-228`) and
   re-run `retail_high_bridge_inventory`. **T1-13, T1-14, T1-15 and T3-07 all hang on this**, and
   demote to Tier 3 if it comes back false.
2. **Whether the corridor-gate defect generalises beyond Bay of Pigs.** OI-30 is a **PREDICTION,
   never measured.** The repro established Bay of Pigs only, two bridges; the reachability
   diagnosis says so in its own §7/§9. The gate runs on every path build so the generalisation is
   expected — but "almost every map" is not a measurement and must not be cited as one. The
   instrument exists and has been run once for inventory (`retail_high_bridge_inventory`, output
   recorded), but no cross-bridge **order** has been issued on any map except Bay of Pigs.
3. **CLOSED for Drive on a high span; still open for everything else.** This gap read "nobody has
   run a crossing", and for Drive that is no longer true: `c3c1cda2` removed the
   gate-disabling helper and the two Drive crossings were run against the real install with
   nothing disabled (`test result: ok. 4 passed; 0 failed`), settling T1-03 and T1-04. What
   remains true, and is the live half of this gap: no crossing has been run for **Walk** or
   **Hover**, on any span kind, and none for any low span. Every "pinned by" for those rows is
   still a reading of an assertion body rather than an observation.
4. **Walk on a high bridge has zero observations of any kind.** 60 stock types, structurally
   identical planner treatment to Drive, no test anywhere in the tree. The harness drives exactly
   two unit types ever: `MTNK` and `ROBO`.
5. **Order sources other than a single ordinary Move have zero observations.** Attack-move, AI
   team, scatter/bump, retreat and guard across a span have never been exercised;
   `try_repath_after_block` is untested on a deck.
6. **Four native questions that decide whether landed fixes are parity or coincidence** — all
   UNCHECKED, all need Ghidra: (a) is gamemd's zone comparison at `0x00429EA4` the same gate as
   `HierarchyGate`? (b) does `blocker_neighbor_counts` have a native counterpart at all — the
   module header (`zone_search.rs:26-28`) maps it to `CellClass+0x122` and the core comment says
   no counterpart is identified, **and the two in-repo texts contradict each other**; (c) does
   gamemd write `0x100` on low-bridge cells? — this decides whether "low bridges get no exemption"
   is drift or agreement; (d) does gamemd's deck-to-deck step consult `0x200` at all (OI-11)?
7. **Render-side deck height has never been examined by any lane.** Every `z` in every repro is
   the sim value. The matrix's acceptance condition is what the player sees, and
   `bridge_height_map` is read only by click-pick, target lines and the debug overlay — **no draw
   path uses it** (`render/.../instances/bridges.rs:162-165` anchors deck art to the ground height
   map plus fixed pixel offsets). One screenshot of a unit mid-span with the sim `z` logged
   alongside closes this.
8. **13 loose `.yro` maps were never swept** — `CrctBrd, DeepFrze, HighExpR, Ice_Age, IrvineCa,
   IsleLand, MojoSprt, MonsterM, MoonPatr, RiverRam, SinkSwim, Transylv, Unrepent`. These are the
   YR-exclusive skirmish maps; `.yro` is a MIX container the sweep script cannot read, but
   `map_file::load_from_path` handles them. `RiverRam.yro` ("River Rampage") is the obvious suspect
   for missed bridge content. The corpus counts in §5 are therefore lower bounds.
9. **UNMAPPED dimensions: none.** All three inventory lanes reported — locomotors, bridge kinds and
   fixtures, and known-open items. No matrix dimension was recorded as UNMAPPED.
