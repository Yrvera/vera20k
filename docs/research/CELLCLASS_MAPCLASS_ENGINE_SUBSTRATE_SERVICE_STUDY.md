# CellClass / MapClass as an Engine Substrate Service — Study & Replacement-Boundary Design

> ## ⛔ STATUS: CLOSED — 2026-06-10
> This study's actionable items are dispositioned; it is no longer a live work tracker and should be
> read as an **evidence archive** (verified native contracts, offset maps, slice history).
> **Implemented this closure pass:** Slice 7 per-cell radiation (`86b0d4bf`, §4.2 #12 sim core),
> Slice 3b playfield-diamond wiring (`7044fcec`), CliffBackImpassability consequence-set completion
> (`8a7e2ea4`, on top of the pre-existing `09d3fa67` pass), Slice 5 occupancy list-order acceptance
> tests (#8). **Downgraded with evidence:** #6 (live terrain-entry path already reads the native INI
> speed table; the confused MZ-row mapping is a dead-with-stock-INI fallback). **Carried forward:** the
> remaining items (A* live classification/Slice 6, reservation-on-intent, corner-cutting, crowd-jam
> verification, radiation render glow, Slices 3c/4, research opens) moved to
> `SUBSTRATE_OPEN_ITEMS_20260610.md` — update THAT doc, not this one.

**Date:** 2026-05-29 · **re-verified & refreshed 2026-05-31** (10-agent pass: 6 live-Ghidra + 4 current-Rust) · **2026-06-04 refresh** (7-lane workflow: 5 current-Rust re-anchor + 2 live-Ghidra; CliffBack predicate + `reveal_by_height` gate re-verified by hand) · **2026-06-10 refresh** (11-lane workflow: 5 current-Rust + 6 live-Ghidra; g_DirectionOffsets + radiation contract + RecalcZoneType tree + MapClass/CellClass offset gaps resolved) · **CLOSED 2026-06-10** (see status box)
**Mode:** study/design only — no Rust written. Authority order binary → Ghidra → docs; load-bearing
native claims re-verified live this session (citations inline; verification tag per claim:
**LIVE** = decompiled/read this session, **LIVE-0531** = re-decompiled in the 2026-05-31 pass,
**DOC** = prior verified report, **UNVERIFIABLE-static** = global reads all-zero because the process is
not running).
**Bar:** active in a standard **local skirmish** (`g_GameMode == 0` campaign-local or `== 5`
skirmish/LAN). MP-only / SpecialFlags / TS-legacy behavior is flagged DORMANT/LEGACY with its gate.
**Builds on (does not re-decide):** the `CELLCLASS_SUBSTRATE_*` design series
(`..._FIRST_MIGRATION_SLICE`, `..._RUST_CALLER_INVENTORY`, `..._LIVE_OBJECT_LIST_WRITERS`,
`..._CELLRECT_VALIDATOR_CONTRACTS`, `..._CAN_ENTER_CELL_RUNTIME_SHAPE`), `CELLCLASS_STRUCT_GHIDRA_REPORT`,
`MAPCLASS_COMPLETE_DECODE`, `MAPCLASS_GHIDRA_REPORT(+followups)`, `LOGICCLASS_VS_MAPCLASS`,
`SUBSTRATE_PARITY_LEDGER_20260529`. This is the spatial-substrate companion to
`LOGICCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (the temporal substrate) and uses the same shape.

**2026-05-31 refresh (what changed vs the 2026-05-29 cut).** A 10-agent verification pass re-checked every
load-bearing claim. Net:
- The two HIGH headline removal/lifecycle bugs and the divergent-matrix bug are **RESOLVED in current Rust**
  — Slices 1 & 2 landed since the prior cut.
- **One new HIGH removal-path hole** the prior cut missed: combat-destroyed **walls** bypass
  `conceal` + foundation removal (`world/mod.rs:1199-1203`).
- The **CellClass constructor address is corrected**: entry `0x0047bbf0` (the prior `0x0047BC50` is interior,
  entry+0x60); `0x0047bb60` is the **destructor** (label-registry dtor-as-ctor warning confirmed).
- The **effective-height contract is split**: cell method `0x00487d50` (`Level+4 iff cell Flags&0x80`) vs
  object variant `0x005F5F00` (driven by the object's `OnBridge+0x8C`).
- The `RulesClass+0x664` open question is **resolved** — it is the `[General] CliffBackImpassability` key
  (default 2, **active in YR and base RA2**), not a TS shore-water holdover.
- `RecalcAttributes` has **38** callers (not 37); `AddContent`/`RemoveContent` select the list by the
  caller-passed bridge **argument**, not the cell's own `+0x8C`; several Rust file:line refs refreshed; the
  persistent `vision_height_grid` was **removed** (height-for-vision is now a transient Vec), dropping
  Level/height duplication from 5 sites to 4.

Per-claim verdicts are folded inline below.

**2026-06-04 refresh (what changed vs the 2026-05-31 cut).** A 7-lane verification workflow (5 current-Rust
re-anchor lanes + 2 live-Ghidra lanes; the CliffBack predicate and `reveal_by_height` gate re-decompiled/read
by hand this session) re-checked the doc after heavy churn (object-substrate Slices 3–8, mission/radio Slices
1–8, tank-bunker Slice 7b, factory/house economy P1–P4). Net:

- **The 2026-05-31 headline NEW HIGH (wall-removal hole, §4.2 #4) is RESOLVED.** `remove_wall_entity_at`
  (`world/mod.rs:1597`) now routes through `self.uninit(id)` (`:1620`); test
  `wall_destruction_routes_through_uninit_no_leak` (`combat/combat_tests.rs:1714`) proves logic-vector +
  occupancy + `Presence` teardown then slot-free after flush. Commit `dfd9f7a4`. §0 / §4.2 #4 / §4.3 / §7 #1 /
  Slice 1b all flip to DONE.
- **NEW lifecycle: two-phase deferred death (Slice 6, LANDED).** `uninit` (`world/mod.rs:1240`) no longer frees
  the store slot — it decrements owned-count → `remove_entity_occupancy` → `clear_radio_contacts_for` →
  bunker-link teardown (`break_links_on_despawn`) → `conceal` → marks `Presence::Dying` (new FSM variant,
  `game_entity.rs:176`) → pushes `pending_delete` (`substrate.rs:76`). `flush_pending_delete` (`:1292`) frees the
  slot at each death-region boundary (command-region `~:2010`, late-region before hash `:1959`, anim-end
  `app_sim_tick.rs:316`). A `Dying` corpse stays id-resolvable for the rest of the tick. This is MORE
  gamemd-faithful (gamemd also defers deletes). Adds a 2nd per-tick invariant `debug_assert_presence_consistent`
  (`:888`, run `:2667`). **Rewrites §5 C-LIFECYCLE and the §6 chokepoint box** to: "uninit → conceal +
  foundation-remove + radio/bunker-link teardown + mark Dying + enqueue; flush_pending_delete frees at region
  boundary."
- **NEW: Slice 3 CellRect validator facade IS IMPLEMENTED — but UNWIRED.** New file `src/sim/cell_rect.rs`:
  two separate query surfaces `check_passability_rect` (`:200`) / `check_occupancy_rect` (`:221`) — never fused
  (C-VALIDATORS #11); `+0xDC` skip-on-`-1` via a separate `CellReservationGrid` (`:107`, `reservation_mask :508`);
  never-null `get_cellclass_fallback`/`CellRef::Dummy` with fixed `y*0x200+x` stride (`:33/:69`); diamond
  `cell_in_playfield_diamond`/`rect_in_playfield` (`:479/:430`); design-series acceptance tests present
  (`:596/:630/:676`). **But every live caller passes `playfield_bounds: None`** (`find_nearby_cell.rs:265`,
  `production_spawn.rs:546,944`) and `PlayfieldBounds` (`:187`) is never built from a loaded map → at real
  callsites containment is still the rectangle fallback and the canonical `ResolvedTerrainGrid::cell` lookup
  still returns `Option`. So **C-GRID #2/#3 are IMPLEMENTED-BUT-UNWIRED, not absent**; the off-diamond-corner
  parity gap is still open. Slice 3 facade = DONE; caller-migration = still open.
- **CORRECTED binary fact (load-bearing): the `CliffBackImpassability` predicate was BACKWARDS** in §3/§5 #13/§9.
  Re-decompiled `RecalcAttributes 0x0047d2b0` this session: `LandType=3` is set when **at least one of 6
  neighbors has `Level >= this.Level+4`** (a higher/cliff-face neighbor); the all-`<` (all-lower) case is the
  branch that **skips** the reclass via `goto`. The prior "neighbors ALL higher → all `+0x11b < Level+4` →
  LandType=3" was doubly wrong (`< Level+4` = neighbor is *lower*; the all-true branch *skips*). Runs at **3
  branch sites** with different guards (overlay-LAT unconditional; iso-clear `LandType==0`; tail
  `LandType∈{0,2,6,8}`) over the asymmetric **6-offset set `{(0,−1),(−1,0),(+2,+2),(+1,+1),(−1,+1),(+1,−1)}`**
  (note `(+2,+2)`, NOT an 8-ring). Spec corrected inline in §3/§5 #13/§9. *(2026-06-10 status correction:
  the "unimplemented DRIFT" verdict was stale — a correct-predicate build-time pass existed since `09d3fa67`
  (2026-03-31); `8a7e2ea4` completed the zone/speed/base-snapshot consequence set. See §3.)*
- **NEW player-visible DRIFT: `reveal_by_height` is hardcoded `false` in the live tick** (`world/mod.rs:2225`,
  "temporarily disabled… re-enables after gameplay parity review"), though INI `RevealByHeight` defaults true
  (`ruleset.rs:856`). Height-based line-of-sight blocking is therefore **inactive in real skirmish** — a unit at
  a cliff base sees over/through the cliff; the transient `ground_height_grid()` (`core.rs:1600`) it feeds is
  never built at runtime. Fires every match on any map with elevation. (Verified by hand this session.)
- **§4.2 #9 overlay-byte "two homes + collapse divergence" is now a FALSE-POSITIVE.** `OverlayGrid`
  deliberately **excludes** bridge overlay bytes (`overlay_grid.rs:68-72`; `is_bridge_overlay_index`
  `overlay_types.rs:32`; test `:594`), so a bridge structural cell has exactly ONE +0x44 home
  (`BridgeRuntimeCell.overlay_byte`) — partitioned by cell class, not duplicated. BR-10/BR-15 collapse-TODO is
  gone; `clear_collapsed_span_overlay_bytes` (`:1016`) is single-home-consistent. Demoted to a narrow UNCHECKED
  (`OverlayGrid::place_overlay :109` is unguarded but has no bridge-id caller). The **A*-corner-cutting half of
  #9 is unchanged (STILL-OPEN)**.
- **§4.2 #8 save/load list-order parity → mostly RESOLVED via Slice 5.** New `EnterOrderCounter` newtype
  (`substrate.rs:28`) + serialized `occupancy_enter_order` (`game_entity.rs:255`) make `OccupancyGrid::rebuild`
  sort by `(occupancy_enter_order, stable_id)` (`occupancy.rs:121`) + category insertion; tests prove the
  rebuild order (`occupancy.rs:788,813`). Residual: no UNCONDITIONAL test builds a multi-occupant cell
  incrementally then asserts rebuild reproduces the identical occupant Vec (only the `OCCUPANCY_DEBUG` runtime
  compare `:292` checks it).
- **Field-duplication re-count: Level/height homes are 5, not 4** — the doc omitted `PathCell.bridge_deck_level`
  (`core.rs:1468`) beside `PathCell.ground_level` (`:1467`). LandType still 4 slots; overlay 2 homes
  (partitioned); slope/tube 2 each. Slice 4 consolidation has NOT landed.
- **§9 `g_DirectionOffsets` N-first guess DEMOTED (unsupported).** Values remain UNVERIFIABLE-static
  (runtime-init; ~95 xrefs all reads, no static-initializer xref). The adjacent sibling tables
  `0x00a8efa8`/`0x00a8ef78` (decoded from `FUN_005060b0` init literals) order CW **E→SE→S→SW→W→NW→N→NE** with +X
  east/+Y south; the {0,±1} magnitude set + sign convention ARE confirmed for direction stepping;
  `g_DirectionOffsets`' own index-0 anchor is unconfirmed. Fixed inline in §9.
- **New vision mechanisms (active, conditional) to fold into §3/§4.3:** gap-generator suppression
  (`apply_gap_generators` + `FLAG_GAP_COVERED`, `vision/mod.rs:787`; uses the General `gap_radius`, IGNORES
  per-object `GapRadiusInCells`/`SuperGapRadiusInCells` — a mod-only sub-drift), SpySat full-map reveal (`:764`),
  per-tick alliance-merge grid (`build_merged_for :204`), spy-infiltration explored reset (`:287`), PsychicReveal
  double-reveal (`psychic_reveal.rs:26`). FogState stays shroud-only (no TS fog darkening) — faithful.
- **New bridge mechanisms (the doc only documented collapse) to fold into §2.4/§5:** repair-walker reverse
  transition (`RepairOutcome`, `bridge_state/mod.rs:453`), `update_adjacent_bridges` rim stub-reset
  (`bridge_orchestrator.rs:1070`), 4-path damage dispatch + Z-gate (`mod.rs:348/903`), damaged-variant
  flood-fill (`:1244`).
- **Re-anchor table (heavy `world/mod.rs` churn, ~+90..+480 lines):** reveal 726→816, conceal 732→822, add_occ
  742→826, remove_occ 768→851, membership-assert 782→865, uninit 875→**1240** (rewritten), OCCUPANCY_DEBUG
  1536→1964, wall-removal 1199→1597, combat-immediate-uninit 1884→1802, sell→uninit `production_sell.rs:712→728`,
  `occupancy_list_layer` `game_entity.rs:601→743`. `pathfinding/*`, `bridge_state/*`, `resolved_terrain.rs`,
  `passability.rs` line refs are stable. `ProductionState.*` cell maps +6.
- **Binary reconfirms (no change):** `Get_CellClass 0x005657A0` (index `y*0x200+x`, literal `0x3FFFF`, never-null
  dummy `&DAT_00abdc50`, coord store `DAT_00abdc74`) VERIFIED; effective-height split (`0x00487d50` cell-own
  `Flags&0x80` vs `0x005F5F00` object `OnBridge+0x8C`; `0x005f5f40/0x005f5f30` are object Z-frame) VERIFIED.
  `RecalcAttributes` callers re-counted **~37–38** (soften the "38").

Per-claim verdicts and re-anchored refs are folded inline where the prior text was factually wrong; elsewhere,
read this study against the 2026-05-31 line numbers and apply the re-anchor table above.

**2026-06-10 refresh (what changed vs the 2026-06-04 cut).** An 11-lane verification workflow (5 current-Rust
re-anchor lanes + 6 live-Ghidra lanes, ~426 tool calls) re-checked the doc after the post-06-04 churn
(substrate cutovers merge `15d1feed`, RNG-parity Slices 1–5, factory/house economy P3–P7, direction-table +
cell-spread embeds, dispatch host). Verification tag **LIVE-0610** = decompiled/read in this pass. Net:

- **Lifecycle drain model REWRITTEN (was 3 drains, now 1+1).** Commit `f61ad4c3` (hash-changing) collapsed the
  deferred-delete drains: `flush_pending_delete` (`world/mod.rs:1239`) now runs at ONE in-sim site — the tail of
  `run_late_region` (`:1917`), after Phase 9, deliberately before the `OCCUPANCY_DEBUG` rebuild (`:1922-1926`),
  the tail asserts (`:2639/:2641`) and `state_hash()` (`:2675`) — plus one conditional app-layer drain after
  death-anim despawn (`app_sim_tick.rs:316`; path correction: the file is `src/app_sim_tick.rs`). A corpse
  uninit'd anywhere in the tick stays id-resolvable (off occupancy, off logic vector) for the whole tick —
  matching gamemd's single tail-of-tick delete drain. Mid-tick consumers are now **Dying-gated** (`344d2539` +
  `f61ad4c3`): vision/fog scans (`world/mod.rs:1612-1617`), power, production tech, AoE fallbacks + retaliation,
  bump/crush + movement-occupancy + scatter + path markers, war-factory exit, aircraft dock, deploy-MCV, miner
  snapshot + purifier count, and repairs (`production_sell.rs:786` — `tick_repairs` was auto-repairing a dying
  corpse for credits; real bug found and fixed by the collapse). Canonical gate predicate
  `GameEntity::is_active()` = `!dying` (`game_entity.rs:811`). `uninit` re-anchored to `world/mod.rs:1187`
  (internal order unchanged; NEW `debug_assert_ne!(presence, Dying)` double-teardown guard `:1211-1215`);
  `pending_delete` is `#[serde(skip)]`, empty at every tick/save boundary (`world/substrate.rs:69-76` — path
  correction: the substrate file is `src/sim/world/substrate.rs`). §5 C-LIFECYCLE and the §6 chokepoint box
  read accordingly.
- **Slice 3 caller migration STARTED — the FNPC authority cutover (`52ca8d99`) is the first live facade
  caller.** The production exit/spawn fallback routes `find_spawn_cell_near_structure`
  (`production_spawn.rs:244`) → `find_nearby_cell::find_nearby_passable_cell` (`:313`) →
  `check_passability_rect`/`check_occupancy_rect` (`find_nearby_cell.rs:237/:256`), with the engine 4-segment
  ring order (gate fix `d64ad257`) and `Simulation::binary_frame` as the frame-counter analog
  (`production_spawn.rs:176`). `find_nearby_cell.rs` is no longer test-only; the old box-ring
  `nearest_walkable_around` + `spawn_fallback_candidate_passable` are retired to `cfg(test)` oracles
  (`production_spawn.rs:440/:594`). Census: **1 of ~40** binary FNPC-caller analogs routes through the facade
  (miner dock `miner_dock_sequence.rs:363/:413`, scatter, chrono, rally, crates, start positions remain
  unmigrated). **`PlayfieldBounds` wiring — RESOLVED later the same day (Slice 3b, commit `7044fcec`):**
  the bounds are built from the map header at init, persisted on `Simulation`, and threaded through
  `NearbyQuery` into the live FNPC occupancy check (the cheapest-path wiring this cut proposed).
  `2a29dd92` added the verified field-meaning doc comments
  (+3 line shift below `cell_rect.rs:179`: facade fns now `:203/:224`, diamond `:482/:433`, `PlayfieldBounds`
  `:190`, acceptance tests `:599/:633/:679`). `ResolvedTerrainGrid::cell` (`resolved_terrain.rs:363`) still
  returns `Option` at real callsites. Also `d64ad257` rewrote `rect_in_playfield` to the verified isometric
  four-corner test.
- **OnBridge two-layer occupancy cutover LANDED (`1ee888a4`).** `OccupancyGrid::remove_on_layer`
  (`occupancy.rs:244`) walks only the selected layer (RemoveContent semantics); `move_entity_layered` (`:289`)
  removes from the OLD cell on the OLD layer then adds to the NEW cell on the NEW layer; call sites sample
  `on_bridge` per-half (`movement_step.rs:1180-1213`, `movement_tick.rs:1285-1325`, `tube_movement.rs:301-320`);
  the collapse DropIn relayer goes Bridge→Ground explicitly (`bridge_orchestrator.rs:1353/:1396-1406`).
  §5 C-RECORD #6's caller-passed-layer semantics are now implemented, not just documented.
- **Bridge deck height: the coordinate-Z deck offset is 2 levels (208 leptons), NOT 4.** Gate pass `d64ad257`
  verified `DAT_00AC13BC` (= per-level height × 2) and split it from the separate +4 Level-unit
  effective-height/pathfinding seed; `3a718775` cut combat AoE over: deck Z from
  `BRIDGE_DECK_HEIGHT_LEVELS = 2` (`map/bridge_topology.rs:76`), AoE layer pick
  `CellBridgeView::aoe_object_layer` (`:248-254`), occupancy bit-layer threshold `ground_z + 2 <= obj_z`
  (`:286`). The Level-unit `GetEffectiveHeight` "+4 iff Flags&0x80" contract (§5 #5) and the high/low
  dispatcher `deck_level >= 4` (`bridge_state/mod.rs:895`) are UNCHANGED — two different unit frames, do not
  conflate. (Stale in-code comments at `bridge_topology.rs:23/:71-75/:239-243` still call the cutover
  "deferred" — they predate `3a718775`.)
- **§9 `g_DirectionOffsets` → RESOLVED (two independent confirmations).** The CRT static initializer was found
  and decoded LIVE-0610: real entry `0x0049F2F0` (Ghidra mis-splits the body as `FUN_0049f300`); 8 dword stores
  at `0x0089F688..6A4` = **N(0,−1), NE(1,−1), E(1,0), SE(1,1), S(0,1), SW(−1,1), W(−1,0), NW(−1,−1)** — index 0
  = NORTH, CW, +X east/+Y south, cell-scaled. Matches facing-byte semantics (idx = facing>>5); does NOT match
  the E-first sibling tables `0x00a8efa8/0x00a8ef78` — rebase when mixing tables. The adjacent initializer
  `0x0049F3A0` writes the lepton twin `0x0089F6D8`. Rust already embeds both with exact-equality tests:
  `substrate/direction_tables/cell.rs` `CELL_DELTAS` + `lepton.rs` `LEPTON_DELTAS` (S1/S2; diagonal lepton step
  is exactly ±256 per axis, NOT the ±181 sin/cos diagonal). Direction-table family is data-layer complete
  (S1–S4: + facing quantization `quantize.rs`, DRAGON 32-way `dragon.rs`); consumer cutovers (S5+) not started.
  Cell-spread is fully cut over (`combat/cell_spread.rs`: 369-entry verbatim table incl. the real index-322
  duplicate defect; consumers repointed by `1266b61e`).
- **NEW player-visible DRIFT (§4.2 #12): the per-cell radiation runtime is MISSING in Rust** (fires on every
  Desolator deploy; also demo truck / nuke payload radiation). A LIVE-0610 lane decoded the full native
  contract — new §2.6 and C-RAD (§5 #16). Rust parses weapon `RadLevel` + warhead `Radiation` but has no
  per-cell field, no decay, no damage (only the immunity gate `combat/damage/gates.rs:32-34`).
- **R5/R7 cadences VERIFIED in the caller (LIVE-0610) + the "exactly twice per loop" claim sharpened.**
  `RecalcBridgeShroudFlags 0x00578100` fires on `(int)frame(0x00A8ED84) % 0x78 == 0` — signed modulo
  re-evaluated every tick, not a countdown (disasm `0x0055AFB0` @ `0x0055b294-0x0055b2ad`); the body is
  unconditional. `UpdateCrateRegenTimers 0x0056BBE0` is called unconditionally every tick (@ `0x0055b655`);
  the `0xa8b238 && 0xa8b261` double-gate is in the body (verified). But the per-tick driver has **7 more
  map-singleton call sites**, all gated — see the note under §2.2: a shroud-REGROW full-grid sweep
  `0x004ACAC0` (conditional, active-capable in YR), a fog-regrow sweep `0x004ACBC0` (TS-legacy,
  SpecialFlags&0x1000), a ZAdjust sweep `0x004AE4C0` (timer-gated), bridge-counter `0x004F42F0` and a
  read-only getter `0x004AEB10`. "Exactly twice" holds only for unconditional default-skirmish invocations.
- **CellClass offset gaps largely closed (LIVE-0610; §2.3 rows updated):** `+0x30` is NOT "per-cell scratch" —
  it is a **persisted, save-swizzled object-pointer slot** (CellClass::Load `0x004839f0` swizzles it alongside
  `+0x2C/+0x3C/+0x40/+0xE0/+0xE4/+0xE8/+0xF8`; MapClass::Resize `0x00565c10` NULLs it with the bridge anchor)
  with **no runtime writer found** — role UNKNOWN/dormant. `+0x50` = **wall-overlay owner** (writer
  `FUN_0047d210` ← InitCellAttributes, via building vtable+0x38; reset −1 by `PostDestructionWallCleanup
  0x00480630`; owner trio with `+0x54/+0x58`). `+0x64..+0x77` = **bridge-overlay draw-dedup cache** (last-draw
  frame stamp + viewport-rect snapshot; render-only; `DrawOverlay_Body 0x0047f6a0`); `+0x5C/+0x60` (ctor −1)
  remain UNKNOWN. `+0xFC` = **lazily-allocated `PixelFXClass*`** water/ore sparkle (render-only; freed by the
  dtor; reset on Load).
- **MapClass offset gaps byte-pinned (LIVE-0610; §9 entry updated):** `+0x50..+0x64` is a vtable-bearing
  DynamicVector (vtable `0x007ED4C0`, Items `+0x54`, Capacity `+0x58`, valid/allocated bytes `+0x5C/+0x5D`,
  ActiveCount `+0x60`, GrowthStep `+0x64` = 10) of **0x10-byte bridge/tube records** (endpoint A/B cell coords,
  intact byte `+0x8`, kind `+0xC`: 0=bridge / 1=tube; tube records skipped by `FindBridgeRecord 0x0056DA10`);
  append path = `ComputeBridgeZones 0x0056D6E0`. `+0x68` = 4-byte-per-zone-cell array (byte0 ZoneType mirror,
  byte1 Level, u16@+2 cluster index), `+0x6C` = count = `(w+1+h)²`, `+0x70` = 10-byte-per-zone-cell pathfinder
  array (byte+8 = Level; other 9 bytes UNKNOWN), `+0x74` has NO accessor anywhere (UNKNOWN/likely padding).
  CellIterator `+0x10C/+0x110/+0x114/+0x118` fully pinned = current X / current Y / diagonal-remaining /
  direct cell-slot ptr (canonical writers `0x00578350/0x00578290`; InitCellAttributes inlines them).
- **RecalcZoneType building/terrain branch fully decoded (LIVE-0610) — two corrections.** (1) The prior
  "IsRubble +0x1fa" wording conflated two fields: **IsRubble is `OverlayTypeClass+0x2B4`** → ZoneType **0**
  (short-circuits everything below it); the `+0x1FA` byte is read off the building's **owner `HouseClass*`**
  (`+0x21C`) and is the TS firestorm-active flag. (2) The building sub-branches are **dormant in stock YR**:
  `FirestormWall`(+0x16C0)/`LaserFence`(+0x16BF) are set nowhere in rules(md).ini, and the LaserFence zone-6
  write is a **dead write** (never returns; overwritten by the loop-exit zone-0 write). The terrain branch IS
  active: `Temperate/SnowOccupationBits == 7` → zone 2, else zone 5; theater check `ScenarioClass+0x1258 == 1`
  (= SNOW; index pinned via the theater table) — all non-snow theaters use the Temperate key (TS two-theater
  legacy). Speed column verified = **Wheel**. §2.4 row updated with the corrected priority chain.
- **Smaller re-anchors / facts:** field-duplication census unchanged — NO Slice-4 consolidation landed (note a
  6th nested Level copy `BridgeLayer.deck_level` `resolved_terrain.rs:112` if counting nested structs).
  `reveal_by_height` still forced false — now `world/mod.rs:2184` (INI default true `ruleset.rs:882`);
  precision fix: the RENDER layer builds `ground_height_grid` every frame for shroud drawing
  (`app_render/mod.rs:66-69`) — the grid is never built in the **sim/vision** tick, not "never at runtime".
  Vision anchors: `apply_gap_generators :792`, SpySat `:769`; psychic reveal lives at
  `src/sim/superweapon/psychic_reveal.rs:26-27` (path correction). NEW latent narrow drift (§4.2 #13): the UI
  range circle prefers per-object `(Super)GapRadiusInCells` (`app_ui_overlays.rs:877-883`) while sim
  suppression uses `[General] GapRadius` only — identical on stock INI, diverges under mods. Wall-damage RNG
  boundary changed by `b134dcd8` (inclusive `next_range_u32_inclusive(0,strength)`, no-op on
  `roll >= damage` — gamemd-verified inclusive boundary). ProductionState anchors shifted −11
  (`resource_nodes :193` etc.; new `factory_shadow: FactoryRegistry :233`); economy P7 adds a deposit-time
  income transform (`sim/economy.rs` IncomeMult ppm + purifier bonus) — `resource_nodes` readership unchanged.
  A* snapshot producer now skips Dying corpses (`bump_crush.rs:129-134`) and its **bridge-layer hard-block set
  is constructed empty and never populated** (`bump_crush.rs:126`) — flagged as an UNCHECKED sub-drift of
  §4.2 #5. Infantry sub-cell allocation main path is now `allocate_sub_cell_with_preference`
  (`bump_crush.rs:357`); `_with_reserved` survives as fallback (`cell_entry.rs:419`). `OccupancyGrid` struct
  decl `occupancy.rs:99`; `BridgeRuntimeState` `bridge_state/mod.rs:546`; rebuild-order tests `:900/:925`;
  `occupancy_list_layer()` `game_entity.rs:748`; OCCUPANCY_DEBUG compare `world/mod.rs:1922-1926`
  (deliberately AFTER the drain); save/load list-order residual (§4.2 #8) resolved later the same day
  (Slice 5 acceptance tests landed — see §4.2 #8). `SNAPSHOT_VERSION` is 19
  (factory/house bumps `1db41ebf`/`06eae652`, not cell/bridge). Lifecycle anchor table: reveal `:822`, conceal
  `:828`, add_occ `:832`, remove_occ `:857`, membership-assert `:871` (run `:2639`), presence-assert `:894`
  (run `:2641`), combat-immediate-uninit `:2317`, wall-removal `:1544` (uninit `:1567`), sell
  `production_sell.rs:728` (unchanged), death-anim uninit `app_sim_tick.rs:310`.

---

## 0. Executive summary

"CellClass / MapClass" is **two cooperating substrate roles**, not one thing:

1. **MapClass = the cell-grid OWNER + spatial-query SERVICE.** A single global object (`g_Map @
   0x0087F7E8`) owning a fixed **512×512 = 262 144** `CellClass*` array, plus the canonical lookup
   (`Get_CellClass`), playfield-containment tests (diamond `IsCellInPlayfield`, 4-corner
   `IsRectInPlayfield`), the per-MovementZone reachability graph (zones/floodfill), bridge geometry,
   shroud, and crate-regen services. It is **passive geometry the tick reads** — the spatial analogue
   of what `LogicClass` is to time.
2. **CellClass = the per-cell DATA RECORD.** A **328-byte (0x148)** struct holding *everything* about
   one cell: terrain (LandType/ZoneType/Level/Slope/Height), overlay+smudge, the two object linked
   lists (`FirstObject`/`AltObject`), the two occupancy bitfields, per-house visibility/sensor/reservation
   bitmasks, bridge flags, shroud/fog state, radar color, and render Z-adjust. Every gameplay system
   reads/writes it.

**What this substrate is NOT:** MapClass is **not** the per-tick object scheduler — that is
`LogicClass::PerTickUpdate @ 0x0055AFB0`, which calls *into* MapClass exactly twice **unconditionally** per
loop (`RecalcBridgeShroudFlags` on `frame % 120 == 0`, `UpdateCrateRegenTimers` every tick — both cadences
LIVE-0610-verified in the caller), plus **7 gated map-singleton call sites** (shroud-regrow / TS-fog-regrow /
ZAdjust full-grid sweeps, bridge counter, a read-only getter — see the note under §2.2). (verified
`LOGICCLASS_VS_MAPCLASS` §3/§4 + LIVE-0610 disasm `0x0055AFB0`; MapClass's 30-slot vtable has no AI/Update
driver.) Also: in gamemd
the global `Map` is **one ~21 868-byte single-inheritance mega-object**
(`GScreenClass→MapClass→DisplayClass→RadarClass→PowerClass→SidebarClass→…`); the cell grid, the
tactical display, and the sidebar are literally the same object. **Do NOT port that fused hierarchy** —
the project's `sim/`-never-depends-on-`render/ui/sidebar/` split is correct; "stored on Map" in the
binary may conceptually belong to any of those six layers.

**State of the Rust port:** there is **no single CellClass-equivalent record**. The native
one-cell-holds-everything design is **fragmented across 9+ Rust per-cell structures** plus side-maps:
`ResolvedTerrainGrid`, `OverlayGrid`, `SmudgeGrid`, `OccupancyGrid`, `BridgeRuntimeState`, `PathGrid`,
`TerrainCostGrid×SpeedType`, `ZoneGrid`, `FogState`, + `ProductionState.{resource_nodes,
terrain_occupation_bits, terrain_object_cells, terrain_spawners}` + render-only `CellLightGrid`/
`TerrainGrid`. **Some of that split is correct and decided** (the prior design series explicitly says
do NOT collapse pathfinding/placement/occupancy/zone into one boolean walkable grid). **Some of it is
unintended duplication**: LandType lives in up to 4 slots, Level/height in **4** sites (down from 5 — the
persistent `vision_height_grid` was removed since 2026-05-29; height-for-vision is now a transient
`PathGrid::ground_height_grid()` Vec), the overlay byte in 2 homes (with a known divergence), and before
Slice 2 there were **two numerically different 13×8 passability matrices** (now one).

**Headline verdict (2026-06-10 re-verification; radiation status updated same day):** the substrate
boundary is sound, and the removal/lifecycle drift that dominated the 2026-05-29 verdict is **fully fixed**
— every removal path (combat, sell, wall) routes through the two-phase `uninit` → single end-of-tick drain
chokepoint, gamemd-faithfully. The newly-decoded per-cell radiation runtime (#12) landed the same day
(Slice 7, commit `86b0d4bf` — sim core complete; the render-layer green glow is the remaining #12
residual). The playfield diamond is now wired into the live FNPC caller (Slice 3b, `7044fcec`), the
CliffBackImpassability reclass is implemented with its full consequence set (`09d3fa67` + `8a7e2ea4`),
and #6 downgraded to fallback-only code hygiene (the live terrain-entry path already reads the native
INI speed table). What remains open: the A* snapshot + corner-cutting drift (#5/#9b),
reservation-on-intent (#7), the save/load list-order residual test (#8), the synthetic crowd/slope
factors (#10), the radiation render glow (#12 residual), and the deferred slices 3c/4/5/6.
Status, default-DRIFT:
- **(HIGH — RESOLVED) Multi-cell footprint leak on removal.** The central helper `remove_entity_occupancy`
  now **exists** (`world/mod.rs:768`) and removes **all** foundation cells via `entity_occupancy_cells`
  (`occupancy.rs:144-163`); save-load `rebuild` expands foundations too (`occupancy.rs:117-138`). Add and
  remove are symmetric — no phantom cells on any path that routes through `uninit`.
- **(HIGH — RESOLVED) Combat death and sell bypassing `conceal`.** Sell → `uninit` (`production_sell.rs:712`);
  combat immediate deaths → `uninit` (`world/mod.rs:1884`); lingering SHP deaths `unregister_live_object` the
  same tick (`world/mod.rs:1870-1872`) then `uninit` on death-anim finish (`app_sim_tick.rs:306`); `uninit`
  conceals before freeing (`world/mod.rs:891-892`). The per-tick `debug_assert_logic_membership_consistent`
  (`world/mod.rs:782`) now holds across a full replay. (Parity-ledger
  `combat-immediate-remove-skips-logic-unregister-1` → fixed.)
- **(HIGH — RESOLVED) Two divergent passability matrices.** Slice 2 retired the duplicate; a single
  `MOVEMENT_ZONE_PASSABILITY` (`passability.rs:115`) matches the verified native dump (test
  `matrix_matches_verified_native_dump`) and is imported by `zone_build.rs:343`. No `PASSABILITY_MATRIX` /
  `MOVEMENT_CLASS_PASSABILITY` symbols remain.
- **(HIGH — RESOLVED 2026-06-04; was the 2026-05-31 headline NEW HIGH) Combat-destroyed WALL bypass — FIXED.**
  `remove_wall_entity_at` (`world/mod.rs:1597`) now routes through `self.uninit(id)` (`:1620`) — full conceal +
  foundation-remove + logic-vector + radio + `Presence` teardown — instead of the old bare
  `self.entities.remove(id)`. Acceptance test `wall_destruction_routes_through_uninit_no_leak`
  (`combat/combat_tests.rs:1714`) builds the wall via active `unlimbo`, then after `apply_wall_damage_events`
  asserts it left the logic order, released occupancy, and is `Presence::Dying`, and is freed after
  `flush_pending_delete`. Commit `dfd9f7a4`. (Same root class as #1/#2, on the path Slice 1 had not enumerated.)
- **(HIGH — open) A* uses a precomputed entity-block snapshot, not per-neighbor live `Can_Enter_Cell`.** Still
  snapshot-based (`core.rs:821` reads `entity_blocks`), but cross-mover same-tick staleness is now mitigated
  by `OccupancyGrid.generation` + `refresh_owner_block_set_if_stale` (`movement_tick.rs:182,999`, commit
  `d2c35ab`); residual staleness is only within one mover's own multi-step expansion.
- **(MEDIUM — open) SpeedType-vs-MovementZone row confusion (§4.2 #6); A*-vs-zone corner-cutting (#9);
  reservation-on-intent unmodeled (#7); `TerrainSpeedConfig` hardcoded/synthetic crowd+slope factors (#10).**
  *(The A* snapshot above is §4.2 #5.)*

The correct move per CLAUDE.md ("Rust-native structure, gamemd-native semantics") still holds: keep the
verified CellClass behavior contract behind a **thin per-cell substrate facade**, finish the field
consolidation, and close the wall-removal hole — **not** collapse the grids into one bool, and **not**
literally port the 328-byte struct.

---

## 1. Verified active-YR responsibilities

| # | Owner | Responsibility | Evidence (this session unless noted) | Active in skirmish |
|---|-------|----------------|--------------------------------------|--------------------|
| R1 | MapClass | **Own the 512×512 `CellClass*` grid** and serve the canonical packed-coord → cell lookup | `Get_CellClass 0x005657A0` LIVE: `index=y*0x200+x`, 30+ callers | Yes |
| R2 | MapClass | **Never-null lookup** — OOB/null returns the dummy sentinel `&DAT_00ABDC50` and stores the probed coord at `DAT_00ABDC74` (=dummy+0x24) | `0x005657A0` LIVE | Yes |
| R3 | MapClass | **Playfield containment** — diamond `IsCellInPlayfield 0x00578460`, 4-corner `IsRectInPlayfield 0x00578390`, lepton wrapper `IsCoordsInPlayfield 0x005785F0` | LIVE | Yes |
| R4 | MapClass | **Per-MovementZone reachability** — zone graph build (`UpdateBridgeZonesHelper 0x56C510`), `GetZoneID 0x0056D230`, incremental fast-paths; A*-retry-local edge repair `FloodFillReachableZones 0x005840C0` | `GetZoneID`/`0x5840C0` LIVE; 38/1 callers | Yes |
| R5 | MapClass | **Bridge geometry/state** — bridge records, 16 damage-state walkers, `SetOverlayAndPropagate`, `RecalcBridgeShroudFlags 0x00578100` on `frame % 120 == 0` | DOC + LIVE-0610 (cadence: signed modulo in caller, disasm `0x0055b294-0x0055b2ad`) | Yes |
| R6 | MapClass | **Shroud service** — `RevealShroud`, explored/needs-redraw on cell `+0x12C` bits 3/4 only (FogOfWar darkening is TS-legacy) | DOC | Yes (shroud); fog = LEGACY |
| R7 | MapClass | **Crate-regen timers** `UpdateCrateRegenTimers 0x0056BBE0` (slot 27); caller invokes it unconditionally every tick (LIVE-0610 @ `0x0055b655`) | LIVE, body double-gated `0xa8b238 && 0xa8b261` (LIVE-0610 re-verified) | Conditional (Crates=yes) |
| R8 | MapClass | **Map-init / post-load cell-attribute rebuild** — `InitCellAttributes 0x00568bb0` re-derives all cell attributes from terrain after load/map-gen | **LIVE this session**; callers `ScenarioClass__Full_Init`, `FUN_00567110`, `FUN_00598960` | Yes |
| R9 | CellClass | **Per-cell terrain truth** — `RecalcAttributes 0x0047d2b0` recomputes LandType/Slope/Level/HeightInPixels/Zone and mirrors Level+ZoneType into the compact ZoneMap arrays | `get_function_callers 0x0047d2b0` LIVE-0531 = **38 callers** | Yes |
| R10 | CellClass | **Live object-list membership** — `AddContent 0x0047E8A0` / `RemoveContent 0x0047EA90` maintain `+0xE4` ground / `+0xE8` bridge lists (building-append / non-building-prepend) | LIVE | Yes |
| R11 | CellClass | **Occupancy bitfields** — `Mark_Occupation 0x007441B0` / `Clear_Occupation 0x00744210` set/clear `+0x124` ground / `+0x128` bridge bits (asymmetric bridge gate) | LIVE | Yes |
| R12 | CellClass | **Passability/entry decisions** — `CheckCellPassability 0x004834a0`, `Can_Enter_Cell` hierarchy, `CheckBridgeTraversal 0x004D9C60`, `GetEffectiveHeight 0x00487d50` | DOC + LIVE spot | Yes |
| R13 | CellClass | **Tiberium (ore) cell lifecycle** — `PlaceTiberium 0x00487190`, `Reduce_Tiberium 0x00480A80`, `SpreadTiberium 0x00483780`, queue seeding | LIVE | Yes |
| R14 | CellClass | **Per-house masks/counters** — visibility+GapGen `+0x78`, sensor `+0x7C[24]`, disguise-detect `+0xAC[24]`, AI base-placement reservation `+0xDC` | LIVE (`+0x78`,`+0xDC`) | Yes / Conditional |
| R15 | CellClass | **Render & minimap** — Z-adjust `+0x104..+0x114` (`Cell_ComputeZAdjust 0x00484680`), radar color `GetRadarColor 0x0047C060`, LightConvert `+0x34` | DOC | Conditional (Z-adjust) |
| R16 | CellClass | **Radiation field** — `+0xF0` level (additive linear-falloff spread, per-step decay by RadSiteClass), `+0xF8` center-cell site ptr; damage applied to FootClass occupants every RadApplicationDelay frames (buildings never) | LIVE-0610 (full decode §2.6) | Yes (any RadLevel>0 weapon: Desolator/demo/nuke) |

**Substrate framing:** R1–R8 are the *MapClass service*; R9–R16 are the *CellClass record*. The two are
the **spatial substrate** every other system queries. The temporal substrate (`LogicClass`) is a
separate study; the only coupling is the two per-tick calls MapClass receives (R5/R7).

---

## 2. Surface inventory

### 2.1 MapClass singleton & grid (LIVE this session)

| Symbol | Addr/Off | Role | Verify |
|--------|----------|------|--------|
| `g_Map` singleton | `0x0087F7E8` | The one MapClass instance; base of the fused `GScreen→…→Sidebar` mega-object (~21 868 B) | LIVE (`read_memory`; ctor `this`-base) |
| `g_CellArray_Base` | `0x0087F924` (= singleton `+0x13C`) | Process-global ptr to `CellClass*[262144]`; hot-path index `[y*0x200+x]` | LIVE (`get_xrefs_to`) |
| `MapClass__constructor` | `0x00565090` | Sets vtable, embedded `VectorClass<CellClass*>` vtable `+0x138`, `num_movement_zones=0xD` `+0x148`, zeros 256 crate-slot coords. **Does not alloc the array.** | LIVE |
| `MapClass__Init_Alloc` (vslot 5) | `0x00565800` | Allocs grid + zone tables; `map_width=+0x14C=0x200`, `map_height=+0x150=0x200`, `total=+0x154=0x40000`; 256-bucket zone hash + 3 zone-graphs + 13 zone_id ptrs | LIVE |
| `MapClass__Get_CellClass` | `0x005657A0` | **cell-coord input.** `index = y*0x200 + x`; bound `[0,0x3FFFF]` (hardcoded literal = 0x40000−1); OOB/null → `&DAT_00ABDC50` + store coord at `DAT_00ABDC74`; **never null** | LIVE-0531 |
| `MapClass__Get_CellClass_At_Coord` | `0x00565730` | **lepton input cousin** (registry rename of the old `CellClass__Get_Cell_At`). Sign-corrected `>>8` (`v+(v>>31&0xFF)>>8`) → cell, then same `y*0x200+x` frame, same `+0x13C` base, same dummy fallback; **bounds against the dynamic total `MapClass+0x140`**, not the hardcoded `0x3FFFF`. Do not confuse input frames. | LIVE-0531 |
| `MapClass__CoordToZoneLinearIndex` | `0x0056D430` | **NOT a cell index** (registry rename of the mislabeled `…CellCoordToLinearIndex`). Stride = `(MapClass+0xf8 + 1 + MapClass+0xf4)` (packed-zone width), index = `stride*y + x`. Feeds the zone arrays `+0x70`, **never** the `+0x13C` cell table — the classic coordinate-frame bug class. | LIVE-0531 |
| `MapClass__InitCellAttributes` | `0x00568bb0` | **Map-init / post-load cell rebuild.** Finishes anims; sets CellIterator `+0x10C..+0x118`; clears bridge-zone Flags bits (`&0xffcfffff`); per cell: zero `+0x30`, `FUN_00483e30(0,0x10000,0,1000,1000,1000)` (LightConvert/ZAdjust reset), clear `0x20000`, re-propagate AttachedTag bridge zones (`0x200000`/`0x100000`) to neighbors, accumulate tiberium (`param2==0` sum / else germinate), `RecalcAttributes` per cell, wall fixup `FUN_0047d210` on `IsWall`. Returns total tiberium. | **LIVE** |
| `MapClass__Is_Cell_In_Playfield` | `0x00578460` | Diamond test on `S=X+Y`,`D=X−Y` vs `+0xF4/+0xFC/+0x100/+0x104/+0x108`; `param3` height-adjust reads cell `+0x11B/+0x11C` via dummy-fallback | LIVE |
| `MapClass__IsRectInPlayfield` | `0x00578390` | 4 inclusive corners NW/NE(x+w−1)/SW(y+h−1)/SE; all must pass; caller `CheckOccupancy` | LIVE |
| `MapClass__IsCoordsInPlayfield` | `0x005785F0` | lepton→cell sign-correct shift `(v+(v>>31&0xFF))>>8`, then `IsCellInPlayfield(param3=1)` | LIVE |
| cell-array reset/resize (vslots 22/23) | `0x00565AA0` / `0x00565B00` | Reset embedded VectorClass + resize to `0x40000`, null all entries | LIVE |
| `DAT_00ABDC50` / `DAT_00ABDC74` | globals | Dummy sentinel cell (non-null) + its `+0x24` coord store | LIVE (resolve; fields runtime-init) |
| MapClass vtable | `0x007ED404` (30 slots) | slot5 Init_Alloc, 22 reset, 23 resize; no per-tick driver slot | DOC |

### 2.2 MapClass services (zone / slope / crate / bridge)

| Symbol | Addr | Role | Verify |
|--------|------|------|--------|
| `GetZoneID` | `0x0056D230` | `zone_ids[mz][cluster_id]`; bridge cells via `FindBridgeRecord`; reads bridge-record base `MapClass+0x54`, cluster id from `+0x68` zone_cell_data | LIVE (38 callers) |
| `UpdateBridgeZonesHelper` | `0x0056C510` | 8-phase full zone rebuild (flood clusters → adjacency hash `+0x14` → per-MZ BFS into `zone_ids[13]` `+0x18`) | DOC |
| `FloodFillReachableZones` | `0x005840C0` | **A*-retry-local** block-flood (size 2/4/8) edge-split detector; returns 1 invalidate / 0 collect-adjacent. **NOT a persistent rebuild.** Sole caller `PathfinderClass__UpdateHierarchicalEdges 0x0042ccd0` | LIVE |
| `g_PassabilityMatrix` | `0x82A594` | 13 MovementZone rows × 8 reduced-ZoneType cols; `1`=pass, `2`/`3`=blocked. Indexed `mz*8 + cell+0x4C` | **VERIFIED** by `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` (`read_memory 0x0082A594 len 416`) and re-read in this follow-up; exact rows below |
| `Get_Slope_Cost_At_Cell` | `0x0056BCD0` | Path-smoothing slope cost. **`+0x24` holds the cell's packed COORDINATE (lepton/sub-cell X,Y), not a "slope value"** (LIVE-0531 wording fix); index = `(X÷4)+(Y÷4)*0x82` into the per-mover table at `base+0x59F0`. **`0x0056BCD0`, NOT `0x00483C80`** (which is `RecalcZoneType`). | LIVE-0531 |
| `UpdateCrateRegenTimers` | `0x0056BBE0` | Per-tick 256-slot crate regen; double-gated `0xa8b238 && 0xa8b261` | LIVE |
| `RecalcBridgeShroudFlags` | `0x00578100` | Clears shroud bits 3/4 on bridge cells (`+0x12C &= ~0x18`, LIVE-0531). The body has **no frame gate** — it rescans unconditionally when called; the cadence lives in the caller `0x0055afb0`: **`(int)frame(0x00A8ED84) % 0x78 == 0`** — signed modulo re-evaluated every tick (fires at frame 0), NOT a countdown timer | LIVE-0531 (body); cadence **LIVE-0610 VERIFIED** (disasm `0x0055b294-0x0055b2ad`) |
| `RevealShroud` | `0x005673A0` | Spiral shroud reveal on `+0x12C` bits 3/4 | DOC |

**LIVE-0610 note — the OTHER map-singleton calls in the per-tick driver `0x0055AFB0`** (all `ECX=0x87F7E8`
thiscalls; the literal cell-array base `0x0087F924` appears nowhere in the function):

| Site | Callee | Gate | Role |
|---|---|---|---|
| `0x0055b1c8` | `0x004F42F0(2)` | ScenarioClass timer `+0x11E8/+0x11F0` | redraw-flag + bridge counter increment |
| `0x0055b273` | `0x004ACAC0` | `Rules+0x17F0` byte && `Rules+0x1640` double && Scen timer `+0x1218` | **shroud-REGROW full 512×512 sweep**: cells with explored(bit3) set but visible(bit4 path `&0x10`) clear get re-shrouded. Active-capable in YR when the rules gate is on; INI key binding for `+0x17F0/+0x1640` not pinned this session |
| `0x0055b31e` | `0x004ACBC0` | `*g_Scenario & 0x1000` (FogOfWar SpecialFlags) && `Rules+0x1648` && Scen timer `+0x1224` | fog-regrow sweep — **TS-LEGACY, dormant in default YR** |
| `0x0055b4c1` | `0x004AE4C0` | Scen `+0x3530 != +0x352C` && `Rules+0x1668` && Scen timer `+0x1248` | cell-iterator loop calling `Cell_ComputeZAdjust` per cell (matches the §2.4 "only while LightningStorm/PsychicDominator active" gating) |
| `0x0055b4cd` | `0x004F42F0(1)` | same block | dirty marker |
| `0x0055b6b8/0x0055b6c5` | `0x004AEB10` | none / first-call-result | read-only getter (`+0x119C/+0x11A0`) feeding g_Tactical |

Verified `g_PassabilityMatrix` rows (`MovementZone` 0..12 × reduced `ZoneType` 0..7; only `1` passes):

| Row | MovementZone | Values |
|---:|---|---|
| 0 | Normal | `1,2,2,2,2,2,2,3` |
| 1 | Crusher | `1,1,2,2,2,2,2,3` |
| 2 | Destroyer | `1,1,1,2,2,2,2,3` |
| 3 | AmphibiousDestroyer | `1,1,1,1,1,1,2,3` |
| 4 | AmphibiousCrusher | `1,1,2,1,1,2,2,3` |
| 5 | Amphibious | `1,2,2,1,1,2,2,3` |
| 6 | Subterranean | `1,1,1,2,2,2,1,3` |
| 7 | Infantry | `1,2,2,2,2,1,2,3` |
| 8 | InfantryDestroyer | `1,1,1,2,2,1,2,3` |
| 9 | Fly | `1,1,1,1,1,1,1,3` |
| 10 | Water | `2,2,2,2,1,2,2,3` |
| 11 | WaterBeach | `2,2,2,1,1,2,2,3` |
| 12 | CrusherAll | `1,1,1,2,2,2,2,3` |

### 2.3 CellClass record — consolidated offset map (328 B / 0x148, ctor `0x0047bbf0`)

**Constructor/destructor disambiguation (LIVE-0531 — corrects the prior cut and confirms the label-registry
dtor-as-ctor warning):** the genuine **constructor entry is `0x0047bbf0`** (body `0x0047bbf0`–`0x0047bda7`);
the prior `0x0047BC50` is **interior** (entry+0x60), so drop it as a separate address. The ctor calls
`AbstractClass__Constructor_Full` **first** (base-ctor-first), inits fields, sets the 4 vtables, calls
`AbstractClass__AssignUniqueID`, and zeroes `+0x11C` (`param_1[0x47]=0`) — it does **not** increment any
"CellClass array" counter. **`0x0047bb60` is the DESTRUCTOR** (leaf-vtable-set → `DECREMENT *(field[0xd]+0x194)`
refcount when `g_GameActive` → chains `AbstractClass__Destructor_ResetVtables`); **`0x00487e80` is the
vtable-bound scalar-deleting destructor** (slot `0x007e4f0c`, bit0 → free). Struct size **328 B** confirmed via
`get_struct_layout CellClass`.

| Offset | Field | Role | Active | Verify |
|--------|-------|------|--------|--------|
| 0x00–0x0C | 4 vtable ptrs | CellClass + 3 secondary (INoticeSink) | Yes | LIVE |
| 0x10–0x23 | AbstractClass base | unique id etc. | Yes | LIVE |
| 0x24 / 0x26 | MapCoord_X / Y | signed cell coords; **center = (c<<8)+0x80**, not NW | Yes | LIVE |
| 0x28 | CellTag / FoggedObject | tag ptr (fog-object list = TS-legacy) | Cond | DOC |
| 0x2C | BridgeAnchorPtr | bridge anchor | Yes | DOC |
| **0x30** | (swizzled object ptr) | **persisted, save-swizzled pointer slot** — `CellClass::Load 0x004839f0` swizzle-remaps it alongside `+0x2C/+0x3C/+0x40/+0xE0/+0xE4/+0xE8/+0xF8`; `MapClass::Resize 0x00565c10` NULLs it with the bridge anchor; zeroed by `InitCellAttributes`. **No runtime writer found** — NOT scratch; distinct from the OBJECT-side `+0x30` list link | DORMANT/UNKNOWN | LIVE-0610 |
| 0x34 | **LightConvert** | refcounted owned palette-remap ptr (mgr `FUN_00483e30`) | Yes | LIVE (struct name) |
| 0x38 | IsoTileTypeIndex | iso tile, init 0xFFFF | Yes | LIVE |
| 0x3C | AttachedTag | trigger tag; bridge-zone propagation source | Yes | LIVE |
| **0x44** | **OverlayTypeIndex** | overlay (−1=none); wall/ore/crate/gate/railroad | Yes | LIVE |
| 0x48 | SmudgeTypeIndex | −1 | Yes | LIVE |
| **0x4C** | **ZoneType** | reduced zone 0–7 (Ground/Road/Wall/Beach/Water/Building/Impassable/OOB); ≠ LandType | Yes | LIVE (`RecalcZoneType`) |
| 0x50 | WallOverlayOwner | wall-overlay owner (building vtable+0x38 result, −1 sentinel; writer `FUN_0047d210` ← `InitCellAttributes` for wall-overlay cells, nearest building; reset −1 by `PostDestructionWallCleanup 0x00480630`). Owner trio with `+0x54/+0x58`; readers UNCHECKED | Yes | LIVE-0610 |
| 0x54 / 0x58 | InfantryOwner Ground/Bridge | sub-cell owner house id (−1) | Yes | LIVE |
| 0x5C..0x77 | bridge-overlay draw-dedup cache | `+0x64` last-draw frame stamp (init −1) + `+0x68..+0x74` viewport-rect snapshot — draw-once-per-frame dedup so a multi-cell bridge overlay isn't redrawn per member cell (`DrawOverlay_Body 0x0047f6a0` bridge branch only; shadow pass has no cache). `+0x5C/+0x60` (ctor −1) UNKNOWN. **Render-only** | Yes (render) | LIVE-0610 |
| **0x78** | **VisibleToHouses + GapGen** | **one** per-house bitmask, dual use (visibility `IsVisibleToHouse 0x004870b0` AND gap-gen writer `0x00487110`) | Yes | LIVE |
| 0x7C | SensorCounts[24] | per-house short array | Yes | DOC |
| 0xAC | DisguiseDetectCounts[24] | per-house short array | Yes | DOC |
| **0xDC** | **ReservationBitmask** | per-house **AI base-placement** mask (bit = `HouseClass+0x30`); **NOT GapGen** | Cond | LIVE (`FUN_0050b760`) |
| 0xE0 | Jumpjet | jumpjet ptr | Yes | LIVE |
| **0xE4** | **FirstObject** | ground object-list head; link via `+0x30` | Yes | LIVE |
| **0xE8** | **AltObject** | bridge object-list head | Yes | LIVE |
| 0xEC | LandType | terrain 0–11 (12 vals); drives speed table | Yes | LIVE |
| 0xF0 / 0xF8 | RadLevel(double) / RadSite | radiation field — full runtime contract in §2.6: `+0xF0` written only by RadSiteClass add/sub helpers `0x00487CE0/0x00487D00`; `+0xF8` site ptr on the CENTER cell only (setter `FUN_00487C70`) | Yes | LIVE-0610 |
| 0xFC | PixelFX ptr | lazily-allocated `PixelFXClass*` (0x3C B) water/ore sparkle — render-only (16-bit RGB565 mode + extra-animations option; `DrawPixelFXSparkles 0x006d7840`; freed by dtor `0x0047bb60`; reset to 0 on Load) | Yes (render) | LIVE-0610 |
| **0x100** | **HiddenOccupancyCounter** | building CanHideThings AddOccupy/RemoveOccupy → behind-marker; **NOT passability** | Yes | LIVE (`FUN_00487E00`) |
| 0x104 | ZAdjust_Scale | 16.16, init 0x10000 | Yes | LIVE |
| 0x108–0x114 | ZAdjust Base/Ground/GroundScaled/Bridge/+2 | render Z, init 1000; `Cell_ComputeZAdjust 0x00484680` writes 0x10A/0x10C/0x10E; rest by `FUN_00483e30` | Cond | LIVE |
| 0x116 | TubeIndex | init 0xFFFF — **TS-legacy** | LEGACY | LIVE |
| 0x11A | Height | raw TMP byte | Yes | LIVE |
| **0x11B** | **Level (signed)** | effective-height base; read signed everywhere | Yes | LIVE |
| **0x11C** | **SlopeIndex** | slope 0–20; **not ctor-set** (from RecalcAttributes / zeroed alloc) | Yes | LIVE |
| 0x11D | HeightInPixels | `(height−30)/15` | Yes | LIVE |
| 0x11E | OverlayData | ore density 0–11 / wall dmg / bridge frame | Yes | LIVE |
| 0x120/0x121 | Cached Shroud/Fog edge frame | init 0xFE; **fog edge = TS-legacy** | Cond | LIVE |
| **0x122** | **BlockerNeighborCount** | 8-neighbor blocker refcount; read **only** as bool by hierarchical A*; **NOT ore, NOT fog** | Yes | LIVE (`AStar 0x00429a90 @0x00429eb1`) |
| **0x124** | **OccupationFlags (ground)** | bits2-4 infantry sub-cells, bit5 vehicle, bit6 building; filter `&0xE0`/`&0x5F` | Yes | LIVE |
| **0x128** | **AltOccupationFlags (bridge)** | bridge-layer mirror; selected at `Level+4` & `Flags&0x100` | Yes | LIVE |
| 0x12C | ShroudFlags | bit3 explored, bit4 needs-redraw | Yes | LIVE |
| 0x130/0x134 | GapConcealment Counter/Max | init 1 / 0 | Yes | LIVE |
| 0x138/0x13C | NeedsRedraw / FogVisionCounter | dirty / fog vision (fog = TS) | Cond | DOC |
| **0x140** | **Flags** | bridge/state dword | Yes | LIVE |
| 0x144 | (trailing) | end of 328-B struct | — | LIVE |

**`Flags (+0x140)` bit map:** `0x80` HasBridgeOverlay (+4 height) · **`0x100` structural-bridge** ·
`0x200` bridgehead · **`0x400` destroyed/inactive-bridge marker** (NOT "rail"; NOT the A* cost bit) ·
`0x800` orientation · `0x1000` dir · `0x2000` pavement · `0x10000` tall-tile-neighbor · `0x20000`
has-tile-anim · **`0x40000` transient A* bridge cost** (search-scoped, separate from `0x400`) ·
`0x100000`/`0x200000` bridge zones (NS/EW) · `0x400000` fog render (TS). **TIBTRE/placement reject mask =
`0x500` (`0x100|0x400`)**, `TEST AH,0x5`.

### 2.4 CellClass writer lifecycle (LIVE)

| Symbol | Addr | Role |
|--------|------|------|
| `RecalcAttributes` | `0x0047d2b0` | **Central producer (38 callers, LIVE-0531).** Recomputes LandType/Slope/Level/HeightInPixels/Zone; **mirrors Level+ZoneType into the compact ZoneMap arrays `DAT_0087f850+idx*4` / `DAT_0087f858+idx*10`** (native cross-grid coupling; both DAT pointers are **runtime-allocated** → values UNVERIFIABLE-static, code path verified); side-effect clears `+0x44`/`+0x11E` on slope auto-overlay-removal; writes **neighbor** cells' `Flags|=0x10000` (note: 0x10000/0x20000, NOT 0x100/0x400); `RulesClass+0x664` **CliffBackImpassability** reclass (active in YR — see §3); hosts the TS `LandType==10` TubeClass branch |
| `RecalcZoneType` | `0x00483C80` | Writes `+0x4C` (0–7). **CellClass method, called only by RecalcAttributes.** Priority (LIVE-0610 full decode): OOB(7) > Crushable-overlay(1) > Wall(2) > overlay-**Wheel**-speed-0(6) > IsARock(6) > **IsRubble-overlay(0, short-circuits all below)** > Water LandType2(4) > Beach LandType6(3) > land-Wheel≤0.01(6) > object walk: building FirestormWall→6 / LaserFence→**dead write** (both **dormant stock-YR** — keys set nowhere in rules(md).ini) / terrain `OccupationBits==7`→2 else→5 (ACTIVE; Snow key iff `Scen+0x1258==1`) > default(0). Speed column = Wheel (`0x89EA48+Land*36+4`) |
| `AddContent` | `0x0047E8A0` | Insert into one selected list (`+0xE4` ground / `+0xE8` bridge) — **selected by the bridge flag the CALLER passes (3rd arg), not by the cell reading its own `+0x8C`** (LIVE-0531 correction); building (WhatAmI==6) tail-append (next-ptr link field `+0x30`), else head-prepend; fires coupled `Mark_Occupation` + (if shrouded & `g_GameMode!=0`) `DiscoverByHouse` (vtable `+0x198`) |
| `RemoveContent` | `0x0047EA90` | Unlink from **one** selected list (same caller-passed bridge arg), preserve order, clear `+0x30`; fires `Clear_Occupation`; does NOT scan the other layer |
| `EnterCell/ExitCell_*MultiCells` | `0x005683C0` / `0x005687F0` | **Sole callers** of Add/RemoveContent; iterate foundation cells, Add/Remove + `RecalcAttributes` each; drive `+0x100` AddOccupy/RemoveOccupy for CanHideThings |
| `Mark_/Clear_Occupation` | `0x007441B0` / `0x00744210` | Set/clear vehicle bit `0x20` in `+0x124`/`+0x128`. **Asymmetry:** Mark requires height AND `Flags&0x100` for bridge; Clear checks height only (so a destroyed bridge can still clear a leftover bit) |
| `BlowUpBridge` | `0x0047DD70` | Collapse: walk `+0xE4` ground list (snapshot next first) → C4-kill each (`vtable+0x16c`); then `+0xE8` deck list → `DropIn` (`vtable+0xEC`) each (deck NOT killed) |
| `DropIn` | `0x005F4160` | Relayer deck→ground: DoCloak(0) remove from bridge list, **clear `+0x8c` OnBridge=0**, Submit, DoCloak(1) re-add to ground list |
| `PlaceTiberium` | `0x00487190` | Grow-or-germinate; `+0x11E += amount` clamp 11; germinate picks variant via **`Scen->Random`** RandomRanged; feeds spread/growth heaps |
| `Reduce_Tiberium` | `0x00480A80` | Harvest/destroy; full removal sets `+0x44=−1`,`+0x11E=0`, `RecalcAttributes`, MarkTerrainDirty, 8-neighbor reseed |
| `Cell_ComputeZAdjust` | `0x00484680` | Per-tick Z recompute — **only while LightningStorm/PsychicDominator active**; else map-load values |
| `GetRadarColor` | `0x0047C060` | Minimap RGB; terrain-obj/bridge/overlay/tile branches; **no fog/shroud darkening branch** (confirms shroud-only YR) |

### 2.5 Validators & per-class `Can_Enter_Cell` (DOC + LIVE spot; consolidates the two design docs)

- `CellRect__CheckPassability 0x0056E7C0` — **9 args** (`RET 0x24`): top-left + w/h + speed_type +
  required_zone + movement_zone + required_height + bridge/layer arg + reject_overlay. Rectangle-wide
  per-cell terrain/zone/height/occupation-byte via `CheckCellPassability`. **No final playfield check**;
  dummy-cell fallback.
- `CellClass__CheckCellPassability 0x004834a0` — `speed_type==4` (Winged/Fly) **short-circuits PASSABLE**;
  required-zone gate via `GetZoneID`; height/bridge selects `+0x124` vs `+0x128` occupation byte; then
  `g_SpeedType_LandType_Table[speed + LandType*9]==0` rejects. Speed table values are covered by
  `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`; this is separate from the MovementZone passability matrix.
- `CellRect__CheckOccupancy 0x00586780` — **2 args** (`RET 0x8`): ground-list RTTI-`0x24` blocker +
  `+0xDC` reservation (only when `arg != −1`, mask `1<<(arg&0x1F)`) + `+0x44/+0x4C/+0x11C` must be
  clear/−1 + building lookup (WhatAmI==6) + final `IsRectInPlayfield`. **`−1` skips reservation only**,
  not the rest.
- `UnitClass::Can_Enter_Cell 0x0073F0A0` — **5 stack args** (`RET 0x14`): target, direction, mutable
  height, optional parent/current cell (null is a real runtime mode), arg5 locomotor-passability gate.
  Computes an **early object-list byte** (ground if `!(Flags&0x100)` or `|height−Level|<2`, else bridge)
  then independently **re-reads bridge occupancy bits** when `height==Level+4 & Flags&0x100`. Returns code
  0..7.
- `CheckBridgeTraversal 0x004D9C60` (vtable `+0x1B0`) — null-parent reconstructs predecessor via
  `(dir−4)&7`; height-diff/slope branches; **forces bridge-list byte** on ascending `diff==4` bridgehead.
- **Effective height — two distinct functions (LIVE-0531 disambiguation):**
  - `CellClass::GetEffectiveHeight 0x00487d50` — the **cell method**: `signed Level(+0x11B) + 4 iff the cell's
    own Flags(+0x140) bit 0x80`. This is the address the cell-level height contract (C-RECORD #5) should cite.
  - `Get_Effective_Height 0x005F5F00` — the **object-on-cell variant**: same arithmetic but `+4` is driven by
    the **object's `OnBridge +0x8C`**, fetching the cell via vtable `+0x1BC`. This is the locomotion/Scatter/
    mission-facing one.
  - **Do NOT** use `ObjectClass::GetHeight 0x005f5f40` (absolute `Z − ground − bridge`) or
    `ObjectClass::GetCoordZ 0x005f5f30` (raw `Z` at `+0xA4`) for the cell contract — both are object Z-frame.

**Per-class hierarchy:** Unit and Infantry share the early-list/bridge-traversal substrate then diverge in
class policy; Building `Can_Enter_Cell 0x00449440` is a tiny placement check. Object-list layer and
occupancy-bit layer are **independent outputs** — a substrate API must expose both, not one
`MovementLayer`.

### 2.6 Radiation cell service (LIVE-0610 full decode — NEW section)

Active in stock YR skirmish (no SpecialFlags/TS gate anywhere in the path); triggered by any weapon with
`RadLevel > 0` — `[RadEruptionWeapon]=500` (Desolator deploy), `[NukePayload]=500`, `[CRNuke]=500`,
`[Demobomb]=100`, `[Nukebomb]=500` (in-repo `ini/rulesmd.ini`; `[Radiation]` section at line 913).

| Piece | Addr / offset | Verified behavior |
|---|---|---|
| `[Radiation]` parser | `0x0066CF90`-area | RulesClass fields: RadDurationMultiple `+0x1804`, RadApplicationDelay `+0x1808`, RadLevelMax `+0x180C`, RadLevelDelay `+0x1810`, RadLightDelay `+0x1814`, RadLevelFactor `+0x1818` (dbl), RadLightFactor `+0x1820`, RadTintFactor `+0x1828`, RadColor `+0x1830` (3×u8), RadSiteWarhead `+0x1834` (ptr) |
| `RadSiteClass` | 0x74 B, vtable `0x007F0810`, global vector items `0x00B04BD4` / count `0x00B04BE0` | fields: `+0x40/+0x42` center cell X/Y, `+0x44` spread cells, `+0x48` radius leptons = `spread*256+128`, `+0x4C` level, `+0x6C` duration = `RadDurationMultiple × RadLevel`, `+0x70` remaining frames; `+0x24` LightSourceClass* |
| Site creation | `WarheadTypeClass::Detonate 0x004690B0` | fires iff `WeaponType+0x158 RadLevel > 0`; spread = `ftol(warhead CellSpread +0x124)`; sets cell `+0xF8` on the **center cell only** (setter `FUN_00487C70`) |
| Spread | `SetCellRadLevels 0x0065B9C0` | (2·spread+1)² square around center; per cell 3D lepton distance (incl. height); if `dist <= radius`: `cell.RadLevel += (radius−dist)/radius × level` — **linear falloff, additive** (different-center sites stack per cell) |
| Cell level write | add `0x00487CE0` / sub `0x00487D00` | only callers are RadSiteClass methods; sub clamps negative to exactly 0 |
| Decay | `RadSiteClass::AI 0x0065B800` — called once per tick per site; LogicClass iterates the vector **backward** (`0x0055b5cd`, right after LightningStorm) | `remaining -= 1`/tick; every RadLevelDelay frames: per-cell `RadLevel -= falloff/levelSteps` (`0x0065BD00` — Ghidra label "ApplyRadDamage" is **WRONG**, it is the decay step); every RadLightDelay: light intensity step-down; `remaining < 1` → site self-deletes (dtor clears center `+0xF8`, removes from vector) |
| Damage | `FootClass::AI 0x004DA530` (sole caller of reader `0x00487CB0`) | every `frame % RadApplicationDelay == 0`, if not ImmuneToRadiation (`TechnoType+0xD37`): `damage = ftol(min(cell.RadLevel, RadLevelMax) × RadLevelFactor)` → ReceiveDamage with RadSiteWarhead. **FootClass-only — buildings never take radiation damage.** Two residual gates UNKNOWN (`vtbl+0x54()==0`, `this+0x81==0`) |
| Re-detonation on same center | `AddRadLevel 0x0065B530` | one site per center cell: removes this site's outstanding per-cell contribution (`DecreaseCellRadLevels 0x0065BB50`), then `level = current_effective + added`, duration/remaining reset, re-spreads |
| Effective level query | `GetCurrentRadLevel 0x0065B510` | `remaining × level / duration` (int); used by deployed-Desolator mission step `FUN_00521320` to re-fire when `< weaponRadLevel/3` |
| Green glow | LightSource ctor `0x00554760` | `intensity = ftol(min(level × RadLightFactor, 2000.0))`; tint = `min(ch×1000/255 × RadTintFactor, 2000)`; on SNOW theater (`Scen+0x1258==1`, cross-pinned by the RecalcZoneType lane) R/B channels forced — exact intent UNKNOWN |

**Rust status (2026-06-10, commit `86b0d4bf`): SIM CORE IMPLEMENTED** — `sim/radiation.rs`
(`RadiationState`: site registry keyed by center cell + sparse per-cell f64 level field, spread/decay/
merge per the table above), `[Radiation]` parsed into `RadiationRules` (`rules/ruleset.rs`,
`RadSiteWarhead` pulled into the referenced-warhead set), `ImmuneToRadiation` on ObjectType→GameEntity,
detonation hook + periodic foot-unit damage in the combat tick (`combat/mod.rs` Phase 3.5), decay after
the combat phase (`world/mod.rs`), persisted + state-hashed, `SNAPSHOT_VERSION` 20→21. Deployed
Desolator re-fire gate (`< RadLevel/3`) implemented as a per-tick check (no 10–20-frame mission-timer
RNG jitter — internally deterministic, cadence-equivalent). **Remaining drift (open):** the green
LightSource glow/tint is NOT implemented (render layer has no dynamic-light infrastructure yet) —
player-visible on every Desolator deploy; see §4.2 #12 residual. Accepted sub-cell residuals: falloff Z
uses `level × 104` leptons without the ramp-table sub-level refinement of the native height-at-coord
helper (`0x0047b3a0` slope branch) — ±1-lepton-class distance error on ramp cells only; exact `sqrt`
vs the native `Sqrt_Approx`; the `levelSteps==0` NaN-poisoning corner (falloff 0 ÷ steps 0) is guarded
to a no-op instead of replicated (stock-unreachable: needs RadLevel < RadLevelDelay).

---

## 3. Active vs inactive / legacy / dormant

**Active in standard skirmish:** R1–R15 except as below; all CellClass terrain/overlay/occupancy/bridge/
tiberium/zone/shroud(reveal) behavior; both validators; per-class `Can_Enter_Cell`.

**Conditional (gated, but on by default or common in YR):**
- `+0xDC` AI base-placement reservation reader (`FUN_0050b760`) — returns 1 (skip) when `g_GameMode==0`;
  active in skirmish for **AI** players' base placement (`AIBaseSpacing`), dormant for the human player.
  (Out of current scope per `feedback_no_ai_yet`.)
- `+0x104..+0x114` Z-adjust per-tick recompute — only while LightningStorm/PsychicDominator active.
- Crate regen (R7) — only with `Crates=yes`.
- Tiberium growth/spread gates (`ScenarioClass+0x34A6` TiberiumGrowthEnabled, SpecialFlags bit `0x80`
  TiberiumSpreads) — both stock-enabled in YR.
- `AddContent` DiscoverByHouse branch — fires in skirmish (`g_GameMode==5 > 0`) when adding into a
  shrouded cell.
- **Shroud-regrow sweep `0x004ACAC0`** (LIVE-0610) — full-grid re-shroud of explored-but-not-visible cells,
  gated `Rules+0x17F0` byte && `Rules+0x1640` double && a scenario timer (`+0x1218`). Active-capable in YR;
  the INI keys behind `+0x17F0/+0x1640` were not pinned this session (do NOT guess them).

**TS-legacy / dormant on the cell/map substrate (do NOT implement as default):**
- **Fog-of-war darkening** — cell `+0x120/+0x121` fog-edge frames, `+0x13C` FogVisionCounter, `Flags`
  bits `0/1/6/0x400000`. Gated `SpecialFlags & 0x1000`; **`FogOfWar=no` default**. Only black shroud for
  unexplored is active. `GetRadarColor` has **no** fog/shroud darkening branch — corroborates shroud-only.
  The per-tick driver's fog-regrow sweep `0x004ACBC0` sits behind the same `SpecialFlags & 0x1000` gate
  (LIVE-0610).
- **RecalcZoneType building sub-branches** — `FirestormWall`(+0x16C0) / `LaserFence`(+0x16BF) gates are set
  by no entry in rules(md).ini (TS Firestorm/laser-fence leftovers); the LaserFence zone-6 write is a dead
  write even if modded in (LIVE-0610). The theater pivot `Scen+0x1258==1` (SNOW vs everything-else
  OccupationBits) is TS two-theater legacy — urban/desert/lunar all take the Temperate key.
- **Tunnel/subterranean** — `+0x116 TubeIndex` (init 0xFFFF), `LandType==10` Tunnel branch + TubeClass
  construction **inside `RecalcAttributes`**, and `MapCoord_Step_By_Direction` direction-8 tube path. TS
  legacy, out of scope project-wide. *(Note: YR-live low-bridge "tube" jump cells are a **distinct**
  mechanism, in scope — do not conflate.)*
- *(RESOLVED 2026-05-31; predicate CORRECTED 2026-06-04.)* **`RulesClass+0x664` is the `[General] CliffBackImpassability`
  key**, not a TS shore-water holdover. Verified: the string `"CliffBackImpassability"` (`0x0083c8cc`) is read
  via `CCINIClass::ReadInt 0x005276d0` and stored as a byte at `RulesClass+0x664` (`0x0066f1e6`); both
  `ini/rules.ini:319` and `ini/rulesmd.ini:409` set it to `2` (the maximal/default). The 6-neighbor scan in
  `RecalcAttributes` **is active in standard YR and base RA2** and the `==2` branch sets `LandType=3`
  (impassable cliff; `RecalcZoneType` maps LandType 3 → ZoneType 3). **CORRECT predicate (re-decompiled
  `0x0047d2b0` 2026-06-04):** `LandType=3` is set when **at least one** of the 6 neighbors has
  `Level >= this.Level+4` (a sufficiently-higher / cliff-face neighbor); the all-`<` case (every neighbor
  *lower* than `Level+4`) is the branch that **skips** the reclass via `goto`. (The prior wording "cell fully
  surrounded by higher terrain → all neighbors' `+0x11b < Level+4`" was doubly wrong: `< Level+4` means the
  neighbor is *lower*, and the all-true branch *skips* the write.) It runs at **3 branch sites** with different
  guards (overlay-LAT unconditional; iso-clear `LandType==0`; tail `LandType∈{0,2,6,8}`) over the asymmetric
  6-offset set `{(0,−1),(−1,0),(+2,+2),(+1,+1),(−1,+1),(+1,−1)}` (note `(+2,+2)`, NOT an 8-ring).
  **IMPLEMENTED in Rust** — and the prior cuts' "unimplemented DRIFT" verdict was itself stale: a build-time
  pass with the CORRECT (≥1-higher-neighbor) predicate, the exact 6-offset set and the `==2` gate has existed
  in `map/resolved_terrain.rs` since commit `09d3fa67` (2026-03-31); the verification lanes missed it. Commit
  `8a7e2ea4` (2026-06-10) completed the consequence set: reclassed cells now also flip the reduced zone to
  Impassable (engine ordering: reclass precedes zone derivation; overlay-claimed zones outrank land), take
  Rock terrain_class/speed_costs and unbuildability, and bake into the base (pre-overlay) snapshot so the
  runtime overlay add/remove restore path cannot resurrect passable clear terrain (levels are static at
  runtime, so base-baking ≡ the native per-recompute re-derive). Eligible-set note: the Rust guard
  {Clear, Water, Beach} buckets covers the native tail set `{0,2,6,8}` because Ice TMP bytes collapse into
  the Clear bucket in this port's land-bucket frame; the overlay-LAT-unconditional branch site is NOT
  separately modeled (a Road-landtype cell under an overlay-LAT tile at a cliff base would natively reclass,
  ours doesn't — UNCHECKED-narrow, needs a branch-site decompile to pin which landtypes reach it).
- **Trigger-tag pre-filter registries** (`DAT_008B40C8`/`DAT_008B41A8` + `HouseClass+0x3C`) — DORMANT in
  standard skirmish (populated only from scenario `[Tags]/[Triggers]/[Events]`, empty in skirmish maps).

**Out-of-sim (render/UI layer, never in `sim/`):** `CellLightGrid` (LightConvert `+0x34`), the render
`TerrainGrid` draw list, Z-adjust render values, radar colors. Their cadence must be matched in the
render/app layer, not inside `sim/`.

---

## 4. Current Rust architecture comparison

### 4.1 The fragmentation map — one CellClass → 9+ Rust grids

gamemd packs everything into **one 328-B CellClass**. Rust spreads it across (Persist = serialized +
state-hashed; Rebuilt = `#[serde(skip)]`, re-derived in `rebuild_caches_after_load` or per tick):

| Rust grid | File | Persist? | CellClass field(s) |
|-----------|------|----------|--------------------|
| `ResolvedTerrainGrid` | `map/resolved_terrain.rs:298` | Rebuilt (restored from map) | IsoTile`+0x38`, Level`+0x11B`, Slope`+0x11C`, Height`+0x11A`, LandType`+0xEC`, tube`+0x116`, radar, bridge subset, `zone_type` cache |
| `OverlayGrid` | `sim/overlay_grid.rs:40` | **Persist+hash** | OverlayTypeIndex`+0x44` + OverlayData`+0x11E` |
| `SmudgeGrid` | `sim/smudge_grid.rs:44` | **Persist+hash** | SmudgeTypeIndex`+0x48` |
| `OccupancyGrid` | `sim/occupancy.rs:98` | Rebuilt (+ transient `generation` counter) | FirstObject`+0xE4` / AltObject`+0xE8` lists (+ retired reservation set) |
| `BridgeRuntimeState` | `sim/bridge_state/mod.rs:545` | **Persist+hash** | bridge fork of `+0x44`/OverlayData/Level |
| `PathGrid` (`PathCell`) | `sim/pathfinding/core.rs:1561` (PathCell `:1459`) | Rebuilt | derived walkability (fuses ground+bridge), Level/Slope/tube copies |
| `TerrainCostGrid` ×SpeedType | `sim/pathfinding/terrain_cost.rs:33` | Rebuilt | derived per-locomotor cost (gamemd computes on the fly) |
| `ZoneGrid` ×MovementZone | `sim/pathfinding/zone_map.rs:182` | Rebuilt | per-cell Zone (derived; one full array per ground MZ) |
| `FogState` (per-owner) | `sim/vision/mod.rs` | **Persist+hash** | per-house visibility bits `+0x78`/`+0x12C` (shroud only) |
| ~~`vision_height_grid`~~ → transient `PathGrid::ground_height_grid()` | `sim/pathfinding/core.rs:1600` | **Removed** (was persistent; now a throwaway Vec) | Level`+0x11B`; built per vision pass only when `reveal_by_height` set — no longer a stored copy |
| `ProductionState.resource_nodes` | `production_types.rs:204` | **Persist+hash** | authoritative ore value (overlay/`+0x11E` is the visual mirror) |
| `ProductionState.terrain_occupation_bits` | `production_types.rs:224` | **Persist+hash** | OccupationFlags`+0x124` bits 0x04/0x08/0x10 only |
| `terrain_spawners` / `terrain_object_cells` / `tiberium_spawning_terrain_cells` | `production_types.rs:216` / `:220` / `:229` | **Persist+hash** | terrain-object linkage + TIBTRE |
| `CellLightGrid` | `map/lighting.rs:236` | render-only | LightConvert`+0x34` |
| `TerrainGrid` (render) | `map/terrain.rs:210` | render-only | derived draw list |

**Same-byte duplication (the unintended fragmentation):** LandType → **4 slots**
(`resolved_terrain.rs:132,137,187,190`: `land_type`,`yr_cell_land_type`,`base_land_type`,
`base_yr_cell_land_type`); Level/height → **4 persistent sites** (down from 5 since 2026-05-29:
`ResolvedTerrainCell.level` `:129`, `PathCell.ground_level` `core.rs:1467`, `BridgeRuntimeCell.deck_level`
`bridge_state/mod.rs:464`, `ResolvedTerrainCell.bridge_deck_level` `:201` — the 5th, `vision_height_grid`, was
removed → now the transient `ground_height_grid` Vec); overlay byte `+0x44` → **2 homes** on bridge cells
(`OverlayGrid` `overlay_grid.rs:40` + `BridgeRuntimeCell.overlay_byte` `bridge_state/mod.rs:487`) with a known
collapse-path divergence; SlopeIndex, tube_index duplicated. Tiberium amount → `resource_nodes` (value) +
`OverlayGrid` density + terrain override (3 representations, 2 hashed).

**CellClass fields with NO Rust home (LIVE-0531 re-confirmed):** the `+0xDC` reservation-on-intent (folded
into `OccupancyGrid`, which only tracks *current* occupancy — see §4.2); `RadLevel +0xF0`/`RadSite +0xF8`
(per-cell radiation field — **partial INI home only**: weapon `RadLevel` `rules/weapon_type.rs:204` and
warhead `Radiation` `rules/warhead_type.rs:176` are parsed, but there is **no per-cell field / decay / damage
runtime** — UNCHECKED vs active-YR radiation weapons e.g. Desolator); `AltOccupationFlags +0x128` as a distinct
dword (reconstructed via `iter_layer(Bridge)` from the unified occupant Vec, not a separate field).

### 4.2 What drifts (default-DRIFT, player-visible first)

1. **(HIGH — RESOLVED since 2026-05-29) Multi-cell footprint leak on removal.** `add_entity_occupancy`
   (`world/mod.rs:742`, called from `world_spawn.rs:244,400`) adds *all* foundation cells, and the central
   helper `remove_entity_occupancy` (`world/mod.rs:768`) now **exists** and removes *all* foundation cells via
   `entity_occupancy_cells` (`occupancy.rs:144-163`, which expands `foundation_dimensions` for structures).
   Sell (`production_sell.rs:712`) and combat death (see #2) route through `uninit`; save-load `rebuild`
   (`occupancy.rs:117-138`) expands foundations (test `rebuild_expands_structure_foundation_cells`
   `occupancy.rs:842`). No phantom cells remain on any path that goes through `uninit`. *(All `:1003-1011`,
   `:264-278`, `:859` etc. line refs in the prior cut are stale; current refs above.)*
2. **(HIGH — RESOLVED) Combat death + sell no longer bypass `conceal`.** Sell → `sim.uninit`
   (`production_sell.rs:712`); combat immediate deaths (structure/voxel) → `uninit` (`world/mod.rs:1884`);
   lingering SHP/infantry deaths `unregister_live_object` the same tick (`world/mod.rs:1870-1872`) then full
   `uninit` on death-anim completion (`app_sim_tick.rs:306`); `uninit` conceals before freeing the slot
   (`world/mod.rs:891-892`). The `debug_assert_logic_membership_consistent` invariant now lives at
   `world/mod.rs:782` and **runs every tick** (`world/mod.rs:2178-2179`). Parity-ledger
   `combat-immediate-remove-skips-logic-unregister-1` → fixed.
3. **(HIGH — RESOLVED, Slice 2) Two divergent 13×8 passability matrices.** A single
   `MOVEMENT_ZONE_PASSABILITY` (`passability.rs:115`) now matches the verified native dump (test
   `matrix_matches_verified_native_dump` `passability.rs:225`) and is imported by `zone_build.rs:343`; water-mover
   legality feeds reduced `cell.zone_type` (not legacy `land_type`) into the MovementZone lookup
   (`core.rs:1388`). Repo grep finds **zero** `PASSABILITY_MATRIX` / `MOVEMENT_CLASS_PASSABILITY` symbols.
4. **★ (HIGH — RESOLVED 2026-06-04) Combat-destroyed WALL bypass — FIXED.** `remove_wall_entity_at` now ends in
   `self.uninit(id)` (`world/mod.rs:1620`, moved from `:1199`) with a comment naming the exact leak (owned count,
   foundation occupancy, logic-vector id, radio contacts). The dangling-id + phantom-cell + skipped-conceal holes
   are closed; acceptance test `wall_destruction_routes_through_uninit_no_leak` (`combat/combat_tests.rs:1714`)
   covers both the logic-vector and occupancy assertions across a real tick + `flush_pending_delete`. Commit
   `dfd9f7a4`. (Same root class as #1/#2, on the path Slice 1 had not enumerated.)
5. **(HIGH — open) A* uses a precomputed entity-block snapshot, not per-neighbor live `Can_Enter_Cell`.**
   `astar_search` (`core.rs:821`) reads `entity_blocks` at `:1188/:1197` and `entity_block_map` at `:1276`,
   built once per command by `build_entity_block_set` (singular merged wrapper, `bump_crush.rs:222`; the layered
   producer is `build_entity_block_sets` plural `:114`). gamemd's `AStar_main_loop` instead classifies live per
   neighbor. **Staleness qualifier amended:** cross-mover same-tick staleness is now mitigated by
   `OccupancyGrid.generation` + `refresh_owner_block_set_if_stale` (`movement_tick.rs:183`, callsite `:1000`,
   test `owner_block_set_refreshes_when_occupancy_generation_advances` `:1878`; commit `d2c35ab`); residual
   staleness is only within one mover's own multi-step expansion. *(2026-06-10: the producer now skips Dying
   corpses, `bump_crush.rs:129-134`; and its bridge-layer hard-block set is constructed EMPTY and never
   populated, `bump_crush.rs:126` — an UNCHECKED sub-drift: bridge-deck blockers are invisible to A*'s hard
   set.)*
6. **(DOWNGRADED 2026-06-10: live-path FAITHFUL; fallback-only confusion.) SpeedType-vs-MovementZone row
   confusion.** Re-audited against the verified native split (terrain-entry legality = the
   `g_SpeedType_LandType_Table` speed-vs-land table, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`; zone
   legality = the MovementZone 13×8 matrix): **all three** Rust CheckCellPassability-analog sites
   (`cell_rect.rs:384`, `cell_entry.rs:184`, `terrain_cost.rs:69`) read the per-cell INI speed profile
   (`SpeedCostProfile.cost_for_speed_type`, parsed from the same `[Clear]/[Water]/…` sections the native
   loader reads) as the PRIMARY source and reject on `0` — exactly the native `table[speed+land*9]==0`
   reject. The confused `zone_layer_for_speed_type` MZ-row mapping (`passability.rs:149`) survives only as
   the `None`-profile FALLBACK, which never fires with stock INI (all 12 land sections define all keys).
   Residuals: (a) TMP byte 15 maps to a "Cliff" semantics name with no `[Cliff]` INI section, so cliff-tile
   cells keep a default (all-`None`) profile and do reach the fallback — harmless today because such cells
   are independently hard-blocked (`is_cliff_like`/zone), but the fallback's row choice is still wrong-frame
   code; (b) native missing-section behavior is all-zeros (= reject everything), ours is pass — both
   unreachable with stock INI. Cleanup (retire the fallback mapping) stays in §7 #3 as code hygiene, no
   longer a player-visible drift.
7. **(MEDIUM — open) `+0xDC` reservation-on-intent has no model.** `movement_reservation.rs:1-3` now declares
   "the live `OccupancyGrid` is the single source of truth"; `reserve_destination_after_transition` (`:13`)
   commits the dest cell only *after* a successful move. There is no reserve-on-intent surface, so two **vehicles**
   can still both path toward one empty cell within a tick. (Infantry have a partial intra-tick **sub-cell**
   packing reservation, `allocate_sub_cell_with_reserved` `bump_crush.rs:312` — sub-cell only, not cell-level.)
   Equivalence to native reserve-on-intent UNPROVEN.
8. **(RESOLVED 2026-06-10) Save/load occupancy list-order parity — proven.** `OccupancyGrid::rebuild`
   re-derives by `(occupancy_enter_order, stable_id)` + category insertion (foundation-aware); the Slice 5
   acceptance tests now exist and pass: `saveload_occupancy_list_order_matches_incremental` (snapshot.rs —
   full `GameSnapshot` round trip, mixed structure/unit ids inserted in id-descending enter order, rebuilt
   list == incremental list + state-hash equality), `saveload_rebuild_is_deterministic`, and
   `saveload_occupancy_list_order_survives_reentry` (a re-entered entity carries the newest enter order with
   the lowest id — the ordering a naive id sort cannot reproduce).
9. **(split 2026-06-04) overlay-byte divergence → FALSE-POSITIVE; A* corner-cutting → STILL-OPEN (MEDIUM).**
   *(a) Overlay-byte:* `OverlayGrid` now deliberately **excludes** bridge overlay bytes (`overlay_grid.rs:68-72`
   via `is_bridge_overlay_index` `overlay_types.rs:32`; test `from_overlay_entries_skips_bridge_overlay_bytes :594`),
   so a bridge structural cell has **one** +0x44 home (`BridgeRuntimeCell.overlay_byte`), partitioned by cell
   class — not duplicated. `clear_collapsed_span_overlay_bytes` (`:1016`) is single-home-consistent and the
   BR-10/BR-15 TODO markers are gone. Residual UNCHECKED-narrow: the unguarded `OverlayGrid::place_overlay`
   (`overlay_grid.rs:109`) would re-create a 2nd home if any caller ever placed a bridge id — none found this
   pass. *(b) A* corner-cutting (STILL-OPEN):* `core.rs:1249` exempts the **Ground** layer from flanking-cardinal
   validation (bridge-only), while `zone_build.rs:503-514` requires **both** adjacent cardinals on **every** layer
   (the prior `:526` ref now lands in the Ground-only height-continuity block). A ground unit can clip a diagonal
   corner the zone flood-fill treats as blocked.
10. **(MEDIUM/UNCHECKED — open) `TerrainSpeedConfig` crowd+slope factors are hardcoded/synthetic**
    (`terrain_speed.rs:28,31,34,37,40,58-59`). No binary citation; the crowd-jam model (threshold 3, 0.7 jam
    factor, radius-2 scan) appears entirely synthetic and is **not even INI-driven** (`from_general` `:68` sets
    only the slope factors). Flag the whole crowd-jam subsystem UNCHECKED-vs-binary, not just the constants.
11. **(LOW — RESOLVED on the cited cause) `OCCUPANCY_DEBUG` rebuild-compare.** `rebuild` is now foundation-aware
    (`occupancy.rs:136`), so the env-gated debug compare (now `world/mod.rs:1922-1926`, deliberately AFTER the
    single delete drain at `:1917`) no longer false-panics on the multi-cell mismatch the prior cut flagged.
    Still opt-in (`OCCUPANCY_DEBUG`) + debug-only; a *residual* incremental-removal leak would surface as a real
    mismatch — the safety net working as intended.
12. **(HIGH — SIM CORE RESOLVED 2026-06-10, commit `86b0d4bf`; render glow still open) Per-cell radiation
    runtime.** Slice 7 landed: `RadiationState` (`sim/radiation.rs`) implements the §2.6/C-RAD #16 contract
    — (2·spread+1)² linear-falloff additive spread over 3D lepton distance, per-site activation-anchored
    countdown decay (`falloff/levelSteps` per `RadLevelDelay` expiry), same-center merge / different-center
    stack, self-delete at `remaining < 1`; periodic foot-unit damage
    `trunc(trunc(min(level, RadLevelMax)) × RadLevelFactor) × Verses/100` through `RadSiteWarhead` every
    `frame % RadApplicationDelay == 0` (buildings/corpses/transported/airborne/`ImmuneToRadiation` exempt;
    sourceless — never arms retaliation); deployed-Desolator re-fire below `RadLevel/3`. Acceptance tests in
    `sim/radiation.rs` + `combat/combat_tests.rs` (falloff exactness, boundary gating, verses scaling,
    exemptions, merge-vs-stack, lifetime/residue, re-fire loop, serde round-trip). All math re-verified live
    this session from the binary (decay/merge arithmetic incl. the int-division `levelSteps` and the
    `(remaining/delay + 1)` outstanding-removal factor). **Residual OPEN (player-visible): the green
    radiation glow/tint** — native LightSourceClass per site (`intensity = min(level×RadLightFactor, 2000)`,
    RadColor tint × remaining/duration); our render layer has no dynamic light sources yet. Fires on every
    Desolator deploy. Batching note: damage collects once per tick after fire resolution (a site armed this
    tick damages all victims the same application frame), where native interleaves per-object — a ≤1-frame
    first-application skew on the creation tick only, inside the native's own object-order ambiguity band.
13. **(LOW — latent, stock-invisible) UI-vs-sim gap-radius source divergence.** The UI range circle prefers
    per-object `SuperGapRadiusInCells`/`GapRadiusInCells` with General fallback (`app_ui_overlays.rs:877-883`);
    sim suppression uses only `[General] GapRadius` (`world/mod.rs:1645` → `vision/mod.rs:792`). Outputs
    coincide on stock rulesmd (both 10); diverges under mods. Fires never in stock play — recorded so the
    vision substrate doesn't silently inherit two sources.

### 4.3 What is faithful (do NOT retire)

- **List insertion discipline** — non-buildings prepend, buildings append, per layer
  (`OccupancyGrid::add` `occupancy.rs:199-216`, via `CellListInsertion::PrependNonBuilding/AppendBuilding`);
  per-layer independent order via `iter_layer` — mirrors `FirstObject`/`AltObject`.
- **`on_bridge` as the list selector** (`occupancy_list_layer()` `game_entity.rs:601`), None for
  Air/Underground — matches `+0x8C`. *(Note `:588 movement_layer_or_ground` is the separate path/locomotor
  layer, not the list selector.)*
- **The split-layer model** `CanEnterLayerContext{terrain_layer, object_list_layer, occupancy_bits_layer}`
  (`sim/pathfinding/cell_entry.rs:195` — file relocated into `pathfinding/`) — faithfully reproduces native
  list-vs-occupancy-bits separation.
- **`CellEntryResult` 0–7 codes** asserted against the verified table (`sim/pathfinding/cell_entry.rs:48`).
- **PathGrid fuses both ground+bridge layers into one PathCell** — the *right* level of fusion.
- **Central `reveal/conceal/uninit` lifecycle** (`world/mod.rs` reveal `:822` / conceal `:828` / uninit
  `:1187`) with the idempotent membership guard — combat, sell AND wall removal (#1/#2/#4) all route through
  it. Since Slice 6 + the `f61ad4c3` drain collapse it is **two-phase**: `uninit` tears down occupancy/radio/
  links/conceal and marks `Presence::Dying`; the single end-of-tick `flush_pending_delete` (`:1917`) frees the
  slot — gamemd-faithful deferred deletion.
- **`FogState` shroud-only** (correct TS-fog avoidance); **`resource_nodes` authoritative ore value**;
  **`yr_cell_land_type`** preserving the raw binary LandType for exact gates.

---

## 5. The gamemd-native behavior contract (what the substrate must reproduce)

Reproduce the *outputs*; clean Rust internals are fine.

**C-GRID (MapClass lookup):**
1. Cell lookup index is **always `y*512 + x`** (stride `0x200`), independent of the loaded map's diamond
   size. The diamond `Size/LocalSize` parameterizes only the playfield-containment test, not the array
   stride.
2. Lookup **never returns null**: OOB (`<0` or `>0x3FFFF`) or null cell → a non-null dummy sentinel that
   **stores the probed coord**; consumers dereference it safely. Don't model as `Option`/`None` at parity
   call sites.
3. Playfield containment is the **diamond** test (`S=X+Y`, `D=X−Y`, four inequalities), not a Cartesian
   rectangle; rect containment = all four inclusive corners (w−1/h−1). Lepton→cell uses the sign-correct
   floor shift `(v + (v>>31 & 0xFF)) >> 8`.

**C-RECORD (per-cell state):**
4. ZoneType (`+0x4C`, 8 vals, derived) and LandType (`+0xEC`, 12 vals, raw) are **distinct** — keep both
   concepts.
5. Effective height = `signed Level(+0x11B) + 4 iff Flags&0x80` — one shared rule across Z-adjust/A*/
   cliff/bridge-occupancy selection. Two native entry points implement it: the **cell method**
   `GetEffectiveHeight 0x00487d50` (gated on the cell's own `Flags&0x80`) and the **object-on-cell variant**
   `0x005F5F00` (gated on the object's `OnBridge +0x8C`). Keep both readings distinct; do not source it from the
   object Z-frame functions (`0x005f5f40` GetHeight / `0x005f5f30` GetCoordZ). *(2026-06-10 unit-frame caveat:
   this "+4" is the Level-unit flag-frame seed. The COORDINATE-Z bridge deck height is **2 levels = 208
   leptons** (`DAT_00AC13BC`, gate pass `d64ad257`) — AoE layer classification and occupancy-bit Z-thresholds
   use the 2-level deck, Rust: `bridge_topology.rs:76/:248-254/:286`. Do not port the +4 into a Z comparison.)*
6. The two object lists (`+0xE4` ground / `+0xE8` bridge) preserve insertion order: **buildings tail-append,
   all else head-prepend**; layer chosen by the inserting object's `OnBridge`, not by cell state. Removal
   touches **one** list (the object's current-OnBridge list) — wrong-layer removal leaks.
7. The two occupancy bitfields (`+0x124`/`+0x128`) are separate from the lists; bits2-4 infantry, bit5
   vehicle, bit6 building; bridge layer at `Level+4 & Flags&0x100`; ignore-infantry `&0xE0`,
   ignore-vehicle `&0x5F`. Mark/Clear are **asymmetric** on the bridge gate.
8. Bridge `Flags` bits are independent: `0x100` (structural) and `0x400` (destroyed marker) tested
   separately; placement/TIBTRE reject on **either** (`0x500`); `0x400` survives a non-walkable deck; never
   conflate with the transient A* cost bit `0x40000`.
9. `+0x78` is **one** per-house mask used by **both** visibility and gap-gen; `+0xDC` is a **separate**
   per-house AI base-placement reservation; do not merge, and `+0xDC` is **not** GapGen.
10. Off-intent reservation: native blocks a cell a unit is *moving toward* before arrival (reserve-on-intent),
    distinct from current occupancy.

**C-VALIDATORS:**
11. `CheckPassability` (terrain/zone/height/occupation-byte, **no** playfield check) and `CheckOccupancy`
    (object/reservation/cell-field blockers + final playfield) are **two separate queries** — never fused.
    `CheckOccupancy(rect,−1)` skips reservation **only**.
12. `Can_Enter_Cell` carries a 5-arg context (target, direction, mutable height, optional parent/current —
    null is a runtime mode, arg5 locomotor gate) and emits **separate** object-list-layer and
    occupancy-bit-layer decisions plus a 0..7 return code. `speed_type==4` (fly) short-circuits passable.

**C-PRODUCER (truth-setting):**
13. `RecalcAttributes` is the single producer that recomputes LandType/Slope/Level/Zone **and atomically
    mirrors Level+ZoneType into the compact ZoneMap arrays** and writes neighbor `Flags|=0x10000`. Any Rust
    that keeps zone-type on the cell *and* a separate zone array must update both in the same step. It also runs
    the **`CliffBackImpassability`** reclass (when `RulesClass+0x664 == 2`, default in YR): a cell with **at
    least one neighbor `Level >= this.Level+4`** (a cliff-face neighbor) becomes `LandType=3` impassable cliff
    (re-decompiled `0x0047d2b0` 2026-06-04 — the all-neighbors-lower case is the one that *skips* it; runs at 3
    branch sites over the 6-offset set `{(0,−1),(−1,0),(+2,+2),(+1,+1),(−1,+1),(+1,−1)}`). **IMPLEMENTED**
    (`map/resolved_terrain.rs` build-time pass, `09d3fa67` + consequence-set completion `8a7e2ea4` — see §3
    for the residual overlay-LAT-branch UNCHECKED).
14. Map-init / post-load **re-derives** cell attributes (`InitCellAttributes`: clear bridge-zone Flags →
    LightConvert/ZAdjust reset → re-propagate AttachedTag bridge zones → RecalcAttributes per cell), then
    object-list membership comes from object Unlimbo order. gamemd does **not** serialize cell attributes —
    it rebuilds them.

**C-RNG (lockstep):**
15. Tiberium germinate variant + spread start direction + spread/growth-queue jitter draw from
    **`Scen->Random` (ScenarioClass+0x218)**, not `g_MainRng`. Verified LIVE-0531 at all three callsites, each
    loading `[g_ScenarioClass_Instance 0x00a8b230]+0x218` as the RNG receiver: queue-jitter `0x00722B5B`,
    spread-direction `0x0048382A`, germinate-variant `0x004871F4`/`0x00487252`. `g_MainRng @ 0x00886b88` (a
    different address) is never touched here. Part of the two-stream split tracked in the substrate parity ledger.

**C-RAD (radiation field, LIVE-0610 — full decode in §2.6):**
16. A `RadLevel > 0` weapon detonation creates ONE radiation site per center cell (`+0xF8` ptr on the center
    cell only; same-center re-detonation MERGES via current-effective-level + added, different centers stack);
    the site writes `cell.RadLevel += (radius−dist)/radius × level` over a (2·spread+1)² square using 3D lepton
    distance, radius = `CellSpread×256+128`. Decay: `remaining` ticks down every frame; per-cell level steps
    down every `RadLevelDelay` frames; site self-deletes at `remaining < 1` (clearing the center ptr). Damage:
    applied by the per-object Foot AI — NOT by a cell sweep — every `frame % RadApplicationDelay == 0`:
    `ftol(min(cell.RadLevel, RadLevelMax) × RadLevelFactor)` via `RadSiteWarhead`, gated on
    `ImmuneToRadiation`; **buildings never take radiation damage**. Deployed Desolators re-fire when the
    center site's effective level drops below `weaponRadLevel/3`.

---

## 6. Rust-native replacement boundary

**Principle:** *Rust-native structure, gamemd-native semantics.* Do **not** port the 328-byte struct, the
fused `GScreen→…→Sidebar` hierarchy, or the COM/vtable plumbing. Do **not** collapse the grids into one
walkable bool (the prior design series decided against this). Model the **behavior contract** behind a thin
substrate facade and fix the duplication/removal drift.

```
                      ┌────────────────────────────────────────────────────────┐
  spawn / map-load    │  CELL LIFECYCLE CHOKEPOINT (Simulation methods)         │
  production / sell   │   reveal/unlimbo → add ALL foundation cells + register  │
  death / crush ─────▶│   conceal        → remove ALL foundation cells          │
  bridge collapse     │   uninit         → conceal + foundation-remove + free   │
                      └───────────────┬────────────────────────────────────────┘
                                      │ single foundation-aware add/remove
                                      ▼
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │  CELL SUBSTRATE  (a thin facade; the grids stay separate underneath)           │
  │   • object lists      → OccupancyGrid (FirstObject/AltObject order)  ← faithful │
  │   • occupancy bits     → OccupancyGrid layer queries                ← faithful │
  │   • terrain/zone/level → ResolvedTerrainGrid (one authoritative copy)          │
  │   • overlay/smudge      → OverlayGrid / SmudgeGrid                              │
  │   • bridge state        → BridgeRuntimeState (single overlay-byte owner)        │
  │   • per-house masks      → FogState + (new) reservation surface (+0xDC)          │
  │   derived caches (PathGrid / TerrainCostGrid / ZoneGrid) rebuilt from the above │
  └───────────────┬──────────────────────────────────────────────────────────────┘
                  │ two SEPARATE query surfaces (never fused)
                  ▼
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │  VALIDATORS   check_passability_rect(...)   |   check_occupancy_rect(rect, res) │
  │  Can_Enter_Cell context {target, dir, height, parent?, arg5} → {object_list,    │
  │     occupancy_bits, terrain} layers + 0..7 code                                 │
  └──────────────────────────────────────────────────────────────────────────────┘
```

**Owners, clean responsibilities:**
- **`MapClass`-equivalent = the grid owner.** A fixed-stride (`y*512+x` or the project's chosen stride)
  cell index with a **never-null** lookup that returns a dummy/sentinel for OOB, plus the diamond
  playfield test. *(Project scale exception: gamemd's 512×512 and the 24/32-house masks cap scale; widen
  the structure — `short[24]` per-house arrays bind at 24 < the 30-player target — preserve the per-house
  semantics.)*
- **`CellClass`-equivalent = the per-cell record, but kept as the existing grids.** The facade routes
  reads/writes; it does **not** introduce one giant struct. Each native field has **one** authoritative
  Rust home (collapse the duplicates) + derived caches.
- **Cell lifecycle chokepoint** = `reveal/conceal/unlimbo/uninit` with a **single foundation-aware**
  add/remove (`remove_entity_occupancy`, `world/mod.rs:857`). Spawn/death/sell/wall-removal ALL route through
  it. Two-phase since Slice 6 + `f61ad4c3`: `uninit` (`:1187`) = occupancy/radio/bunker-link teardown +
  conceal + mark `Presence::Dying` + enqueue; the SINGLE end-of-tick `flush_pending_delete` (`:1917`, before
  the OCCUPANCY_DEBUG compare, tail asserts and `state_hash`) frees slots in death order; one conditional
  app-layer drain after death-anim despawn (`app_sim_tick.rs:316`).
- **Two validator surfaces** = `check_passability_rect` and `check_occupancy_rect` (separate, per the
  design-series contract) + the split-layer `Can_Enter_Cell` context (already present, keep).

**Decided, do not re-open** (from the design series): keep `OccupancyGrid` as the object-list substrate
(do not replace with a sorted-id set); keep object-list and occupancy-bit layers separate; keep
`CheckPassability`/`CheckOccupancy` distinct; keep `+0xDC` reservation separate from dynamic occupancy;
migrate callers by **API surface**, not by C++ class shape; the first slice is a read-only facade + tests,
**not** a caller rewrite.

---

## 7. Ad hoc Rust logic to retire / demote / consolidate

The grids themselves are **not** retired (the multi-grid split is correct). What's retired is the
**duplication and the broken paths**:

1. **The origin-cell-only removal in combat/sell/rebuild** — ✅ **DONE (fully, incl. walls).**
   `remove_entity_occupancy` (`world/mod.rs:857`) is the one foundation-aware helper; combat
   (`world/mod.rs:2317`), sell (`production_sell.rs:728`) and wall removal (`remove_wall_entity_at`
   `world/mod.rs:1544` → uninit `:1567`, commit `dfd9f7a4`) all route through `uninit`; `rebuild` expands
   foundations (`occupancy.rs:117-138`). No removal path does a bare `entities.remove` any more.
2. **`PASSABILITY_MATRIX` in `passability.rs`** — ✅ **DONE.** Replaced by `MOVEMENT_ZONE_PASSABILITY`, the
   verified native table (rows = MovementZone, cols = reduced `CellClass+0x4C` ZoneType, only `1` passes);
   `zone_build.rs:343` imports it. No duplicate symbol remains.
3. **The two SpeedType/MovementZone→row mappings** — DOWNGRADED to code hygiene (2026-06-10, see §4.2 #6):
   the live path already uses the native INI speed table as primary at all three terrain-entry sites; the
   `zone_layer_for_speed_type` MZ-row mapping is a dead-with-stock-INI fallback. Retiring it (and aligning
   the missing-section default with the native all-zeros = reject) is cleanup, not a parity fix.
4. **Duplicated field copies** — `vision_height_grid` ✅ **already removed** (now transient
   `ground_height_grid`); still pending: `PathCell.{slope_type, ground_level, tube_index}` (borrow
   `ResolvedTerrainGrid`), `LandType` 4-slot → one field + lookup fn, `BridgeRuntimeCell.overlay_byte` vs
   `OverlayGrid` (one overlay owner for bridge cells, or a documented fork with sync points — latent divergence).
5. **`TerrainSpeedConfig` hardcoded crowd/slope constants** — still UNCHECKED; verify the real gamemd speed
   mechanism (crowd-jam model appears synthetic and isn't INI-driven — a "hardcode threshold X" red flag).
6. **The `OCCUPANCY_DEBUG` compare** — ✅ **DONE / re-enabled** now that `rebuild` is foundation-aware
   (`world/mod.rs:1536-1539`); the safety net is live again.

**Not retired (faithful):** the §4.3 list — list insertion order, `on_bridge` selector, split-layer model,
0–7 codes, PathGrid two-layer fusion, central lifecycle, FogState shroud-only, `resource_nodes`.

---

## 8. Migration slices + acceptance tests

Sequenced to land the highest player-visible parity earliest at lowest risk. Each slice is gated on a
**full-skirmish replay state-hash regression** (unchanged, or changed only in the expected
parity-improving direction).

### Slice 0 — Substrate primitives (DONE / faithful)
Split-layer `CanEnterLayerContext`, `CellEntryResult` 0–7, `on_bridge` list selector, list insertion
order, central `reveal/conceal/uninit`. *Baseline; keep.*

### Slice 1 — Foundation-aware removal + lifecycle routing — ✅ DONE (verified 2026-05-31)
The foundation-aware helper `remove_entity_occupancy` (`world/mod.rs:768`) exists; combat-death + sell route
through `uninit`; `OccupancyGrid::rebuild` expands structure foundations (`occupancy.rs:117-138`).
- **Acceptance (met):** `rebuild_expands_structure_foundation_cells` (`occupancy.rs`);
  `debug_assert_logic_membership_consistent` runs every tick (run site `world/mod.rs:2639`); the
  `OCCUPANCY_DEBUG` compare is live (`world/mod.rs:1922-1926`).
- **Slice 1b — ✅ DONE (2026-06-04, commit `dfd9f7a4`):** wall removal routes through `uninit`
  (`world/mod.rs:1544/:1567`); acceptance test `wall_destruction_routes_through_uninit_no_leak`
  (`combat/combat_tests.rs:1714`) covers logic-vector exit, occupancy release, mid-window `Presence::Dying`,
  and slot-free after `flush_pending_delete`.

### Slice 2 — Collapse the two passability matrices (correctness) — ✅ DONE (verified 2026-05-31)
Native `g_PassabilityMatrix` values are now verified: `MOVEMENT_CLASS_PASSABILITY` matches
`0x0082A594`, while `PASSABILITY_MATRIX` does not. Retire the duplicate/remapped matrix and unify legality
lookups on the MovementZone-indexed reduced-ZoneType table; keep `SpeedType` for terrain speed/cost only
unless a separate verified reader proves otherwise.
- **Acceptance:** `zone_connectivity_and_astar_agree_on_every_mz_zonetype` (exhaustive 13×8 ×
  SpeedType sweep); existing zone/path tests pass; hash regression.
- **Implementation note (2026-05-29):** Rust now owns a single verified
  `MOVEMENT_ZONE_PASSABILITY` table in `passability.rs`; `zone_build.rs` imports it, and water-mover
  legality in `core.rs` feeds reduced `zone_type`, not legacy `land_type`, into MovementZone lookups.

### Slice 3 — CellRect validator facade — facade ✅ DONE; first caller ✅ DONE (2026-06-04); wiring open
The read-only facade landed (`src/sim/cell_rect.rs`: `check_passability_rect :203` / `check_occupancy_rect
:224`, separate queries; `CellReservationGrid` skip-on-−1; never-null `get_cellclass_fallback`; all three
design-series acceptance tests present `:599/:633/:679`). `d64ad257` upgraded `rect_in_playfield` to the
verified isometric four-corner test. **First live caller migrated** by the FNPC authority cutover
(`52ca8d99`): production exit/spawn fallback → `find_nearby_cell::find_nearby_passable_cell` →
both validators, engine 4-segment ring order, `binary_frame` counter.
- **Slice 3b — ✅ DONE (2026-06-10, commit `7044fcec`):** `PlayfieldBounds` is built from the loaded map
  header at init (`app_init_helpers.rs` — `[Map] Size=` width + raw `LocalSize` rect, stored verbatim per
  the verified field meanings), persisted on `Simulation.playfield_bounds`, and threaded through
  `NearbyQuery` into `check_occupancy_rect`'s playfield-corner test. The live FNPC caller now rejects
  off-diamond border-filler cells the rectangle fallback accepted; regression test
  `find_nearby_occupancy_rejects_off_diamond_cells_when_bounds_threaded` proves both the old acceptance
  and the new rejection with the pool in engine ring order.
- **Remaining (Slice 3c):** migrate the other FNPC-analog callers (miner dock `miner_dock_sequence.rs:363/:413`
  + `bunker_link.rs:222`, scatter, chrono outbound, rally, crate placement, start positions) — 39 of ~40
  binary callsites still on separate implementations.

### Slice 4 — Consolidate duplicated fields + overlay-byte owner
Collapse Level×5 / LandType×4 to one authoritative home + derived caches; make one overlay-byte owner for
bridge cells (fix BR-10/BR-15 collapse divergence).
- **Acceptance:** `level_has_single_source_of_truth`; `bridge_collapse_clears_overlay_on_full_span`;
  hash regression.

### Slice 5 — Save/load substrate re-derivation parity — ✅ ACCEPTANCE LANDED (2026-06-10)
The occupancy-rebuild contract is proven: `saveload_occupancy_list_order_matches_incremental` (full
`GameSnapshot` round trip + state-hash equality), `saveload_rebuild_is_deterministic`, and
`saveload_occupancy_list_order_survives_reentry` (re-entered entity = newest enter order, lowest id) all
pass in `snapshot.rs`. The broader PathGrid/ZoneGrid/TerrainCostGrid rebuild-reproduces-live-state sweep
remains routine regression coverage (these grids are pure functions of serialized state and are rebuilt,
not restored), tracked in the successor open-items doc.

### Slice 6 — A* live `Can_Enter_Cell` per neighbor (DEFERRED, large)
Replace the precomputed entity-block snapshot with per-neighbor live classification (or prove the snapshot
bit-equivalent on a measured scenario). Reserve-on-intent (`+0xDC`-style) modelling rides here.
- **Acceptance:** `astar_neighbor_uses_live_can_enter_cell`; measured parity on a two-movers-contend
  scenario; full regression.

### Slice 7 — Per-cell radiation service — ✅ SIM CORE DONE (2026-06-10, commit `86b0d4bf`)
The §2.6/C-RAD contract landed Rust-natively: `RadiationState` (`sim/radiation.rs` — site registry keyed by
center cell + sparse per-cell level field), detonation hook + periodic foot damage in the combat tick,
decay after the combat phase, `RadSiteWarhead` through the live Verses path, persisted + hashed
(`SNAPSHOT_VERSION` 21). One deliberate deviation from this slice's spec: cell levels are **f64 with
trunc-staging**, not fixed-point — the same documented float exception as `combat::damage`, because the
native pipeline computes the falloff/decay kernel in doubles and exact ftol boundaries (`level 500 × 0.2 =
100` vs fixed-point `99`) are unreachable in I16F16. `Scen->Random` confirmed not consumed (the only native
RNG on the path is the deployed-Desolator mission-recheck jitter, which the Rust gate-per-tick replaces).
- **Acceptance (all landed, green):** `desolator_deploy_irradiates_square_with_linear_falloff` (exact
  center/edge/corner values incl. the truncated-distance corner exclusion);
  `rad_damage_fires_on_application_delay_boundary_only`; `buildings_take_no_rad_damage` (+ ImmuneToRadiation);
  `same_center_redetonation_merges_not_stacks`; `different_center_sites_stack_additively`;
  `site_self_deletes_and_clears_center_ptr` (+ float-residue bound); `midlife_merge_resets_to_effective_plus_added`;
  `decay_countdown_is_activation_anchored`; `deployed_desolator_self_irradiates_and_refires_below_third`;
  `radiation_state_serde_round_trip`. Full suite 3842 green; hash composition adds zero folds while the
  field is empty, so committed golden baselines are unshifted.
- **Remaining (Slice 7b — render):** the green LightSource glow (intensity/tint stepping, RadColor) needs
  dynamic-light infrastructure in the render layer. Player-visible on every Desolator deploy — tracked as
  the §4.2 #12 residual.

---

## 9. Open questions / deferred (carried forward)

- **`g_PassabilityMatrix 0x82A594` values are RESOLVED.** `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`
  verified the full 416-byte `int[13][8]` dump and this follow-up re-read it via Ghidra `read_memory`.
  As of the 2026-05-29 Slice 2 implementation, `MOVEMENT_ZONE_PASSABILITY` in `passability.rs` is the
  matching Rust table; the old divergent `PASSABILITY_MATRIX`/private `MOVEMENT_CLASS_PASSABILITY` split is retired.
  **`g_SpeedType_LandType_Table` (`0x0089EA40`, with older `DAT_0089ea48` pointing at the Wheel column)
  is a separate speed/cost table, not the MovementZone passability matrix.**
- **`+0xDC` reservation SETTER lifecycle** — *(RESOLVED 2026-05-31)* **no setter exists in reachable code.**
  Both readers (`FUN_0050b760`, `FUN_005060b0`) only *read* the per-house bitmask `cell+0xDC & (1<<houseIdx)`
  (`houseIdx = HouseClass+0x30`), and both are AI base-placement (`FUN_005060b0` ← `HouseClass__AI_ChooseNextProduction`).
  The field is zero-init by the cell ctor. Needed only when AI base placement is implemented (out of scope per
  `feedback_no_ai_yet`).
- **Dummy cell `DAT_00ABDC50` full field table** UNVERIFIABLE-static — the contract "bridge flags on the
  dummy are clear" is asserted but unproven without a live dump (off-map `IsCellInPlayfield` param3 height
  path reads dummy `+0x11B/+0x11C`).
- **`g_DirectionOffsets 0x0089F688` — ✅ RESOLVED (2026-06-10, two independent confirmations).** The CRT
  static-init writer was located and decoded: real entry **`0x0049F2F0`** (Ghidra mis-splits the body as
  `FUN_0049f300`; the `unaff_retaddr` decompile artifact comes from the wrong function start). Found via
  WRITE-xrefs on **interior** entry addresses (`get_xrefs_to 0x0089F68C/690/694` → stores at
  `0x0049f322/336/34a`); prologue raw-byte decode (`read_memory 0x0049f2c0`: `XOR EDX,EDX; OR ECX,-1` → DX=0,
  CX=−1) pins entry 0; reachability = CRT `_initterm` pointer table at `0x00812bac` (runs before any game
  code — which is why static reads return zeros); consumer layout cross-check `decompile_function 0x0048182d`.
  Decoded table: **index 0 = N (0,−1)**, CW N,NE(1,−1),E(1,0),SE(1,1),S(0,1),SW(−1,1),W(−1,0),NW(−1,−1) —
  +X east / +Y south, cell-scaled; idx = facing>>5. The E-first sibling tables `0x00a8efa8/0x00a8ef78` have a
  DIFFERENT index-0 anchor — rebase when mixing. The adjacent initializer `0x0049F3A0` writes the lepton twin
  `g_DirectionDeltaX/Y 0x0089F6D8` (= cell table ×256; diagonal is exactly ±256/axis, NOT ±181 sin/cos). Rust
  embeds both with exact-equality tests (`substrate/direction_tables/{cell,lepton}.rs`, independently decoded
  in `FACING_DIRECTION_SUBSTRATE_STUDY.md` Verification Log #1/#3). Label-drift flag: interior entry
  `0x0089F6A0` carries the stale label `_g_refinery_unload_adjacent_lookup_dx` — it is just entry 6 (W).
  Residual UNKNOWN: initializer `0x0049F2D0` zeroes the 4 dwords after the table (`0x0089F6A8..B4`), purpose
  unidentified.
- **`RulesClass+0x664` reclassification in RecalcAttributes** — *(RESOLVED 2026-05-31; predicate CORRECTED
  2026-06-04)* this is the `[General] CliffBackImpassability` key (default `2`, **active in YR**), not
  Beach/Shore and not a TS holdover — see §3. Promoted to an active behavior contract (§5 #13). **The neighbor
  predicate in the prior cut was inverted** — `LandType=3` fires when at least one of the 6 neighbors is
  `>= Level+4` (higher), not when all are lower; corrected inline in §3 / §5 #13.
- **`SlopeIndex +0x11C` init-order** — *(RESOLVED 2026-05-31)* the ctor **does** zero it (`param_1[0x47]=0`
  covers `0x11C`); `RecalcAttributes` treats it as a persisted attribute and read-before-write branches see the
  zero-init/persisted value, never uninitialized memory. No stale-slope read.
- **CellClass offset-table gaps — largely RESOLVED 2026-06-10** (see updated §2.3 rows): `+0x50` = wall-overlay
  owner; `+0x64..+0x77` = bridge-overlay draw-dedup cache (render-only); `+0xFC` = lazy `PixelFXClass*` sparkle
  (render-only); `+0x30` reclassified from "scratch" to a **persisted save-swizzled object-pointer slot with no
  runtime writer found** (role still UNKNOWN — needs a read-side sweep before declaring unused). Still open:
  `+0x5C/+0x60` (ctor −1, no accessor found), `+0x50` reader side, and whether building vtable+0x38 returns a
  house index or `HouseClass*` (−1 sentinel suggests index).
- **MapClass offset gaps — ✅ byte-pinned 2026-06-10** (refresh block has the full table): `+0x50..+0x64` =
  vtable-bearing DynamicVector of 0x10-byte bridge/tube records (Items `+0x54`, Capacity `+0x58`, ActiveCount
  `+0x60`, GrowthStep 10; record = endpoint A/B coords, intact byte `+0x8`, kind `+0xC` 0=bridge/1=tube;
  appended by `ComputeBridgeZones 0x0056D6E0`); `+0x68` = 4-B/zone-cell array (zone, level, cluster-u16),
  `+0x6C` = `(w+1+h)²` bound, `+0x70` = 10-B/zone-cell pathfinder array (byte+8 = level), CellIterator
  `+0x10C/110/114/118` = X / Y / diagonal-remaining / cell-slot ptr (writers `0x00578350/0x00578290`).
  Still open: the other 9 bytes of the `+0x70` records; `+0x74` (no accessor found — likely padding). **Diamond fields RESOLVED
  2026-06-04** (`CELLCLASS_PLAYFIELD_BOUNDS_FROM_LOCALSIZE_GHIDRA_REPORT.md`): `base +0xf4 = Size.width`
  (set unconditionally by `MapClass::Resize 0x00565c10`), `+0xfc/+0x100/+0x104/+0x108 =
  LocalSize.{left,top,width,height}` verbatim (set by the map loader `Read_Map… 0x004ad76b` via
  `INIClass::ReadRect 0x00527cc0`, `sscanf "%d,%d,%d,%d"`). No transform at store time — the iso transform
  is entirely in the consumer `Is_Cell_In_Playfield 0x00578460`. So `cell_rect.rs` `PlayfieldBounds` field
  names are no longer UNVERIFIED.
- **`RecalcZoneType` building-zone(5) detail — ✅ RESOLVED 2026-06-10** (full predicate tree decoded; §2.4 row
  updated). Corrections: "IsRubble `+0x1fa`" conflated two fields — IsRubble is `OverlayTypeClass+0x2B4` →
  zone 0 short-circuit (`ReadBool("IsRubble")` store `0x5fea0a`), while `+0x1FA` is read off the building's
  OWNER `HouseClass*` (`+0x21C`) = TS firestorm-active flag (no writer found in the image —
  UNVERIFIABLE-static beyond that). `FirestormWall +0x16C0` / `LaserFence +0x16BF` / `LaserFencePost +0x16BE`
  parser stores verified at `0x460ada/0x460aba/0x460aaf`; both gate branches DORMANT in stock YR (keys unset)
  and the LaserFence zone-6 write is a dead write. Terrain branch ACTIVE:
  `Temperate/SnowOccupationBits` (+0x2A8/+0x2AC, stores `0x71e08b/0x71e0a0`) `==7` → 2 else 5;
  `ScenarioClass+0x1258` = theater index (writers `0x4acff0`/`0x687649`; 0=TEMPERATE, 1=SNOW). Residual
  UNKNOWN: `BuildingClass+0x618` laser-fence state value semantics (8/0xC by placement facing).
- **`FUN_00483e30`** — *(RESOLVED 2026-05-31)* confirmed the **refcounted LightConvert (`+0x34`) manager** that
  *also* writes the ZAdjust fields (`+0x104..+0x114`). Lifecycle: refcount at `LightConvert+0x194`
  (`++` on adopt/identity, `--` on release **gated by `g_GameActive`**); the factory `FUN_00544e70` constructs/
  caches `LightConvertClass` (0x1b4 B) keyed by `(R,G,B)` tint, short-circuiting identity `(1000,1000,1000)` to the
  cached default `DAT_0087f69c`. All callers are render-time (`CellClass__DrawOverlay_*`, `TechnoClass_DrawSHP`,
  `TerrainClass__Draw_It`, `AnimClass__DrawIt`, `MapClass__InitCellAttributes`). Render-layer concern only; the
  refcount-on-save/load parity is the open part. *(Struct-doc correction: it is more than a ZAdjust initializer.)*
- **NEW open (2026-06-10, radiation lane — §2.6):** identities of the two FootClass rad-damage gates
  (`vtbl+0x54() == 0`, `this+0x81 == 0` — likely in-air/limbo, INFERRED only); `weapon+0xAC` = warhead ptr
  (INFERRED, strong); the LightSource snow-theater R/B channel force (`Scen+0x1258==1` → `0xFFFFFF80`) intent;
  `ReceiveDamage` trailing-arg semantics on the rad path.
- **NEW open (2026-06-10):** the A* bridge-layer hard-block set is constructed empty and never populated
  (`bump_crush.rs:126`) — prove whether gamemd's per-neighbor classification blocks bridge-deck cells on
  occupants before closing §4.2 #5.
- **Doc corrections this study makes** (re-verified LIVE): `+0xDC` is per-house **reservation**, not
  GapGen (struct doc line 95 stale); `+0x122` is a **generic blocker-neighbor refcount**, not OreNeighbor
  (struct doc line 147 stale); `+0x78` is **one** field for both visibility and GapGen; `+0x100` is the
  **hidden-occupancy** counter (was "Unknown"); `Flags 0x400` is the **destroyed-bridge marker**, not
  "BridgeRail"; `Get_Slope_Cost_At_Cell` is `0x0056BCD0` not `0x00483C80`; `RecalcZoneType` Ghidra plate
  comment is stale (Crushable+0x22D vs "IsCrate", IsARock+0x2B5 vs "IsGate").
- **Doc corrections added 2026-05-31** (re-verified LIVE-0531): CellClass **constructor entry is `0x0047bbf0`**,
  not `0x0047BC50` (interior, entry+0x60) — and `0x0047bb60` is the **destructor**, `0x00487e80` the
  scalar-deleting destructor (label-registry dtor-as-ctor warning confirmed); **`AddContent`/`RemoveContent`
  select the list by the caller-passed bridge argument, not the cell's own `+0x8C`**; `RecalcAttributes` has **38**
  callers (was 37); **effective height splits** into cell method `0x00487d50` vs object variant `0x005F5F00`
  (and `0x005f5f40`/`0x005f5f30` are object Z-frame, not the cell contract); **`Get_Slope_Cost_At_Cell` reads a
  packed COORDINATE at `+0x24`**, not a "slope value" (slope/cost from the per-mover table at `base+0x59F0`);
  the **120-frame `RecalcBridgeShroudFlags` cadence is caller-sourced** (`0x0055afb0`), not in the body;
  **`RulesClass+0x664` = `CliffBackImpassability`** (active). Label-registry renames folded into §2.1:
  `0x0056D430`=`MapClass__CoordToZoneLinearIndex` (packed-zone frame), `0x00565730`=`MapClass__Get_CellClass_At_Coord`
  (lepton cousin); the real `GetRadarColor` is `0x0047c060` (the `0x00587410` collision was renamed away).

---

## 10. Sources

**Live Ghidra this session (gamemd.exe, read-only):**
- `decompile_function` — `0x0047bbf0` (CellClass ctor — the 2026-05-29 cut mis-cited interior `0x0047BC50`),
  `0x005657A0` (Get_CellClass), `0x00565090`
  (MapClass ctor), `0x00565800`/`0x00565AA0`/`0x00565B00` (Init_Alloc/reset/resize), `0x00578460`/
  `0x005785F0`/`0x00578390` (playfield tests), `0x0047d2b0` (RecalcAttributes — via prior decompile +
  this-session caller verify), `0x00568bb0` (**InitCellAttributes**), `0x00483C80` (RecalcZoneType),
  `0x0047E8A0`/`0x0047EA90` (Add/RemoveContent), `0x005683C0`/`0x005687F0` (Enter/ExitCell),
  `0x007441B0`/`0x00744210` (Mark/Clear_Occupation), `0x0047DD70` (BlowUpBridge), `0x005F4160` (DropIn),
  `0x0056D230` (GetZoneID), `0x005840C0` (FloodFillReachableZones), `0x0056BBE0` (UpdateCrateRegenTimers),
  `0x00487190` (PlaceTiberium), `0x00480A80` (Reduce_Tiberium), `0x00483780` (SpreadTiberium),
  `0x00722AF0`/`0x007235A0` (spread/growth queue), `0x00487d50` (GetEffectiveHeight), `0x004870b0`
  (IsVisibleToHouse), `0x0050b760` (+0xDC reservation reader), `0x00487E00` (+0x100 reader), `0x00429a90`
  (AStar +0x122 gate).
- `get_function_callers` — `0x0047d2b0` (**38 callers**, re-counted 2026-05-31), `0x00568bb0` (Full_Init + 2), `0x005657A0`
  (30+), `0x0056D230` (38), `0x0047E8A0`/`0x0047EA90` (single Enter/ExitCell caller each), `0x00578390`
  (CheckOccupancy), `0x005840C0` (UpdateHierarchicalEdges only).
- `get_struct_layout CellClass` (328 B, field offsets); `read_memory` — `0x0087F7E8` (singleton),
  `0x0087F924` (g_CellArray_Base), `0x00ABDC50` (dummy), `0x82A594` (passability matrix; initially
  misread as all-zero static, corrected by `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` and this
  follow-up's 416-byte re-read).

**Live Ghidra — 2026-05-31 refresh pass (10-agent workflow, 6 binary + 4 Rust, ~337 tool calls):**
`decompile_function`/`disassemble_function` — `0x005657A0`, `0x00565730`, `0x0056D430` (grid lookup cousins),
`0x00568bb0`, `0x00567110`, `0x00686b20` (InitCellAttributes + callers), `0x00578460`/`0x00578390`/`0x005785F0`
(playfield), `0x00565800` (Init_Alloc), `0x0047bbf0`/`0x0047bb60`/`0x00487e80` (**ctor/dtor disambiguation**),
`0x004870b0`/`0x00487110` (+0x78), `0x0050b760`/`0x005060b0` (+0xDC), `0x00487e00` (+0x100), `0x00429a90` (+0x122),
`0x0047e040`/`0x0042acf0`/`0x00486ff0`/`0x0048703c` (Flags 0x100/0x400/0x40000/0x500), `0x0047d2b0` (+ **38** callers),
`0x00483C80` (RecalcZoneType), `0x0047E8A0`/`0x0047EA90` (Add/RemoveContent — caller-arg list select),
`0x007441B0`/`0x00744210` (Mark/Clear), `0x005683C0`/`0x005687F0` (Enter/ExitCell), `0x0047DD70`/`0x005F4160`
(BlowUpBridge/DropIn), `0x0056D230` (GetZoneID, 38), `0x005840C0` (FloodFill, sole caller `0x0042ccd0`),
`0x0056BCD0` (Get_Slope_Cost), `0x0056BBE0`/`0x00578100` (crate/bridge-shroud), `0x00487d50`/`0x005F5F00`/
`0x005f5f40`/`0x005f5f30` (**height split**), `0x0056E7C0`/`0x004834a0`/`0x00586780`/`0x0073F0A0`/`0x004D9C60`
(validators/Can_Enter_Cell), `0x00722af0`/`0x00483780`/`0x00487190` (Tiberium RNG = `Scen->Random`),
`0x00483e30`/`0x00544e70` (LightConvert), `0x005276d0`/`0x0066f1e6` (**CliffBackImpassability**);
`read_memory` — `0x82A594` (416-B passability dump, non-zero), `0x0089F688`/`0x0087F7E8`/`0x0087F924` (UNVERIFIABLE-static),
`0x0083c8cc` ("CliffBackImpassability"); `get_struct_layout CellClass` (328 B). Cross-checked
`GHIDRA_LOAD_BEARING_LABEL_AUDIT_REGISTRY.md` (Slices 1–3 renames). **Current-Rust re-map** (2026-05-31):
`world/mod.rs` (lifecycle 726/732/768/782/875/1199/1536/1870/1884/2178), `occupancy.rs` (98/117/144/199/842),
`production_sell.rs:712`, `app_sim_tick.rs:306`, `pathfinding/{core,passability,zone_build,terrain_speed,cell_entry}.rs`,
`movement/{movement_tick,bump_crush,movement_reservation}.rs`, `map/{resolved_terrain,terrain,lighting}.rs`,
`{overlay_grid,smudge_grid,bridge_state/mod,vision/mod,game_entity}.rs`, `production/production_types.rs`,
`rules/{weapon_type,warhead_type}.rs`.

**Live Ghidra — 2026-06-10 refresh pass (11-lane workflow: 6 binary + 5 Rust, ~426 tool calls):**
`disassemble_function`/`decompile_function` — `0x0055AFB0` (per-tick driver: bridge-shroud `%120` gate
`0x0055b294-0x0055b2ad`, crate-regen `0x0055b655`, 7 gated singleton sites incl. shroud-regrow `0x004ACAC0`,
TS-fog-regrow `0x004ACBC0`, ZAdjust sweep `0x004AE4C0`, getter `0x004AEB10`, RadSite backward vector loop
`0x0055b5cd`), `0x00578100`/`0x0056BBE0` (bodies), `0x004839f0` (CellClass::Load swizzle set incl. +0x30),
`0x00565c10` (Resize +0x2C/+0x30 NULLing), `0x0047d210` (+0x50 wall-owner writer), `0x00480630`
(+0x50 reset), `0x00426270`/`0x00426300`/`0x005217c0` (+0x54/+0x58 owner trio), `0x0047f6a0`/`0x0047f510`
(+0x64..+0x77 draw-dedup cache), `0x006d7840` (+0xFC PixelFX), `0x0047bb60` (dtor +0xFC free), `0x00483c10`
(CellClass::Save raw stream), `0x0049f300`+`read_memory 0x0049f2c0` (**g_DirectionOffsets initializer** — 8
entries decoded, N-first; CRT table `0x00812bac`), `0x0048182d` (consumer layout), radiation chain —
`0x0066cfc9` ([Radiation] parser), `0x0065b1e0`/`0x0065b2f0`/`0x0065b4d0`/`0x0065b4f0`/`0x0065b510`/
`0x0065b530`/`0x0065b580`/`0x0065b800`/`0x0065b9c0`/`0x0065bb50`/`0x0065bd00` (RadSiteClass; vtable
`0x007F0810`; vector `0x00B04BD4`), `0x00487c70/c90/cb0/ce0/d00` (cell rad helpers), `0x004DA530`
(FootClass::AI damage block), `0x004690B0` (Detonate site creation), `0x00521320` (Desolator re-fire),
`0x00565090` (MapClass ctor — +0x50 DynVec, +0x138 VectorClass, +0x8C/+0xA4/+0xBC/+0xD4 vectors),
`0x0058adb0` (DynVec ctor), `0x0056D6E0` (ComputeBridgeZones append + record layout), `0x0056DA10`
(FindBridgeRecord), `0x00567110` (InitZoneMap +0x68/+0x6C/+0x70), `0x0056D3F0`/`0x0056D430` (zone index),
`0x00578350`/`0x00578290` (CellIterator), `0x00483C80` (+`disassemble`; RecalcZoneType full tree),
`0x00440a57`/`0x00445e32` (BuildingClass+0x618 writers), `0x0048dbe0`+`read_memory 0x7E1B78/0x7E1BE8`
(theater table), parser stores via `get_assembly_context` — `0x460aaf/0x460aba/0x460ada`
(LaserFencePost/LaserFence/FirestormWall), `0x5fea0a` (IsRubble), `0x71e08b/0x71e0a0` (OccupationBits),
`0x4acfd9/0x687634` (Theater), `0x0067413b` (Wheel speed column), `0x5fe7b7/0x5fe7e5` (overlay Land/Wall);
`get_xrefs_to` interior `0x0089F68C/690/694` (WRITE refs), `0x0087F85C` (none), `0x0087F838` (none).
**Current-Rust re-map (2026-06-10):** `world/mod.rs` (uninit 1187, flush 1239/1917, drain-before-debug
1922-1926, asserts 894/2639/2641, lifecycle 822-871, wall 1544/1567, combat-uninit 2317, reveal_by_height
2184), `world/substrate.rs` (28/69-76), `app_sim_tick.rs` (308-316), `cell_rect.rs` (+3 shift; 203/224/190),
`find_nearby_cell.rs` (89/237/256/265), `production_spawn.rs` (244/313/342/176; cfg-test 440/594),
`occupancy.rs` (99/121/184/244/289/900/925), `game_entity.rs` (186/256/748/811),
`bridge_topology.rs` (48/76/248/286), `combat_aoe.rs` (69/240), `bump_crush.rs` (114/126/129/222/323/357),
`movement_tick.rs` (183/1000/1878), `passability.rs` (149/165), `core.rs` (821/1188/1249/1276/1467/1600),
`zone_build.rs` (503-514), `vision/mod.rs` (204/287/769/792), `superweapon/psychic_reveal.rs` (26),
`production_types.rs` (193-233), `terrain_speed.rs` (28-68), `overlay_grid.rs` (40/68-73/344/599),
`bridge_state/mod.rs` (454/546/851/895/925/1244), `bridge_orchestrator.rs` (1070/1353/1720),
`substrate/direction_tables/{cell,lepton,quantize,dragon}.rs`, `combat/cell_spread.rs`,
`combat/damage/gates.rs` (32-34), `snapshot.rs` (37), `ruleset.rs` (882).

**Research docs digested:** `CELLCLASS_STRUCT_GHIDRA_REPORT`, `MAPCLASS_COMPLETE_DECODE`,
`MAPCLASS_GHIDRA_REPORT(+followup+revisit)`, `MAPCLASS_GET_CELLCLASS_FALLBACK_DUMMY_CELL`,
`MAPCLASS_ZONES_RAMPS_HUT_REGISTRY`, `MAPCLASS_FLOODFILLREACHABLEZONES_005840C0`,
`MAPCLASS_GET_SLOPE_COST_AT_CELL_PATH_SMOOTHING`, `LOGICCLASS_VS_MAPCLASS`,
`CELLCLASS_SUBSTRATE_{FIRST_MIGRATION_SLICE,RUST_CALLER_INVENTORY,LIVE_OBJECT_LIST_WRITERS,
CELLRECT_VALIDATOR_CONTRACTS,CAN_ENTER_CELL_RUNTIME_SHAPE}`, `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS`
(+ full-arg-decode + full-blocker-tree), `CELL_OBJECT_LIST_ORDERING_PARITY`,
`CELL_OCCUPANCY_ORDERING(+FOLLOWUP)`, `CELLCLASS_0XDC_RESERVATION_LIFECYCLE`,
`CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS`, `CELL_0X122_*`, `CELL_FLAGS_0X500_TIBTRE`,
`CELL_COMPUTE_ZADJUST_FORMULA`, `CELLCLASS_GETRADARCOLOR_FULL_BRANCH_INVENTORY`,
`CELLCLASS_{CANGROW_CANSPREAD,GROWTIBERIUM,PLACETIBERIUM,REDUCE_TIBERIUM}`, `SPATIAL_PRIMITIVES_LAYER`,
`coord-cell-conversions/*`, `SUBSTRATE_PARITY_LEDGER_20260529`, `CORE_PRIMITIVE_PARITY_20260529`,
`ENGINE_STATE_OVERVIEW`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT`,
`NAVAL_ZONE_LEGALITY_GHIDRA_REPORT`, `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT`,
`SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT`.

**Rust source mapped:** `src/sim/occupancy.rs`, `src/sim/world/mod.rs` (lifecycle 730-873, rebuild
944-1000, grids), `src/sim/game_entity.rs`, `src/sim/combat/mod.rs:1003`, `src/sim/production/production_sell.rs`,
`src/sim/world/bridge_orchestrator.rs`, `src/sim/movement/{movement_step,movement_tick,movement_occupancy,
movement_reservation,bump_crush}.rs`, `src/sim/pathfinding/{core,cell_entry,passability,terrain_cost,
terrain_speed,zone_map,zone_build}.rs`, `src/map/{resolved_terrain,terrain,overlay,lighting}.rs`,
`src/sim/{overlay_grid,smudge_grid,bridge_state/mod,vision/mod}.rs`,
`src/sim/production/production_types.rs`, `src/sim/world/world_hash.rs`.

**Method/caveat:** detect-and-design only; default verdict on any difference is DRIFT/UNCHECKED unless
proven equivalent. Static all-zero reads of runtime-init globals are flagged UNVERIFIABLE-static, never
asserted as verified. One decode cluster (validators/Can_Enter_Cell) did not return structured output;
that surface is sourced from the two design docs read directly (`..._CELLRECT_VALIDATOR_CONTRACTS`,
`..._CAN_ENTER_CELL_RUNTIME_SHAPE`) plus the LIVE spot-checks above.
