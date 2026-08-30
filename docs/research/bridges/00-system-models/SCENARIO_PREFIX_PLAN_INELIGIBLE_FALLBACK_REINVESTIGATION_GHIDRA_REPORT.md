# Scenario Prefix Plan-Ineligible Fallback — Ghidra Re-investigation

**Address(es):** `0x00686B20`, `0x00687F10`, `0x004F54A0`, `0x004F631F..0x004F634F`,
`0x00688380`, `0x005D6BE0`, `0x005D6C70`, `0x005D6890`, `0x005C2EF0`,
`0x005C2D00`, `0x006686C0`, `0x0066899F..0x006689C1`, `0x0068ACAA..0x0068AD04`,
`0x00565C10`, `0x0047BBF0`, `0x0056DC20`, `0x0056E7C0`, `0x004834A0`,
`0x00684620`, `0x00598960`, `0x00599650`, `0x00594870`, `0x006F3254`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The active-retail offline multiplayer Scenario-RNG prefix from the successful
Start reseed through the first terrain-Fill draw, especially every case for which current Rust
`preload_standard_battle_start_plan` returns `None`: sparse or deficient starts, custom fixed
maps, stock non-Battle/FFA modes, Cooperative, and the stock accepted `RandMap.Sed` launch path.
The report also fixes the exact relative boundary of Fill, authored overlay `Mark`, Terrain and
Techno construction, without taking ownership of those downstream mechanisms.
**Non-Scope:** Campaign (`g_GameMode == 0`), online/network assignment when
`DAT_00A8B244 == 2`, replay playback, malformed external seed files, full terrain-Fill formulas,
full overlay-Mark parity, Post-Map starting-unit formulas, and runtime gameplay RNG.
**Confidence:** High. Every load-bearing order edge, active stock mode callback, House count/order,
House destruction, Gather draw, pre-Fill cell-state dependency, and ordinary `.SED` provenance is
direct active-binary or YR retail-data evidence. Current Rust findings are from `origin/main`
`a3e4ce9a`; the investigation worktree remained at `7143d171`, and the relevant production files
have no intervening diff.
**Active in YR:** Yes. The prefix runs for every stock offline skirmish launch (`g_GameMode == 5`).

## 0. Working Notes Gate

- **Target question:** What exact Scenario-RNG work must occur before Fill when the current optional
  preload plan is ineligible, and what is the smallest exact Rust correction?
- **Prior hypothesis:** Complete Battle/FFA starts could be preloaded, while sparse/deficient and
  other modes had to wait for a resolved terrain grid after Fill.
- **Decisive contrary evidence:** Retail creates and destroys a complete first House set, executes
  both start callbacks, reloads rules, then creates a second complete House set, all before Fill.
  Deficient Gather sees newly resized default `CellClass` objects, not resolved authored terrain.
- **Stop condition:** exact first/second House count and order, destruction semantics, both Gather
  calls, mode dispatch, deficient-cell authority, generated-source provenance, and the first Fill
  boundary all resolved. This condition is met for the active offline bridge scope.

## 1. Verdict

Current Rust is not cursor-exact in either branch.

The eligible Battle/FFA preload advances only one House-constructor pass. Retail advances two
identical House-constructor passes, with the start callbacks between them, because the rules/type
registry reset destroys the first set and `Read_INI_Basic` creates it again. The current
plan-ineligible fallback is more displaced: it begins Fill with a pristine Scenario cursor, then
runs Gather and assignment after terrain, overlay, Terrain and authored Techno construction. Native
runs all of those start-prefix draws before Fill.

Sparse and deficient starts do **not** require resolved terrain. Full Init has already applied
`[Map] Size`/`LocalSize` and `MapClass__Resize`, so Gather operates on allocated `CellClass` objects,
but Fill/IsoMapPack, OverlayPack, TerrainClass and Technos have not run. Those fresh cells have the
constructor defaults: clear land, no overlay, no occupation and no object. The fallback result is
therefore a function of the match Scenario cursor, authored waypoint slots, session roster,
MapClass cell-array rectangle, LocalSize/playfield geometry, frame zero, and the native nearby-cell
scan—not the authored isometric terrain payload.

The smallest exact correction is to replace the eligibility-gated plan with one universal
stock-offline pre-Fill prefix transaction. It must advance a single `ScenarioBootstrapRng` through:

1. first House pass;
2. selected mode `+0x80` Gather/preassignment;
3. selected mode or offline `+0x84` second Gather/assignment;
4. the draw-free destroy/reset boundary;
5. second House pass;
6. then hand the same cursor to Fill.

The transaction retains gathered/assignment outcomes for later House projection; applying them
after map construction must draw nothing. It uses a narrow default-cell pre-Fill MapClass view,
not `ResolvedTerrainGrid`.

No load-bearing `BLOCKED` or `UNKNOWN` item remains inside the claimed scope. A filename ending in
`.SED` is a low-level suffix trigger, but a fresh external `.SED` is not an active stock offline
chooser boundary: retail loose-map discovery does not add it. The only stock chooser record is the
synthetic `RandMap.Sed`, created only after the random-map setup has generated/accepted a preview;
that preview has already populated Scenario start staging. This is an evidence-backed exclusion,
not an approximation.

## 2. Exact Native Full-Init Order

The noncampaign path through `ScenarioClass__Full_Init @ 0x00686B20` has this fixed order:

| Ordinal | Native operation | Scenario-RNG consequence |
|---:|---|---|
| 1 | Scenario clear/default work | no prefix draw established here |
| 2 | Read active/authored waypoints; random path copies eight staging starts `+0x11C0..+0x11DF` into active waypoint storage `+0x632` | none |
| 3 | Read general/rule HouseType input | none established |
| 4 | first `ScenarioClass__Create_Houses @ 0x0068745E` | exactly one ranged invocation per created House |
| 5 | read `[Map] Size`/`LocalSize`; `MapClass__Resize @ 0x00565C10`; radar/playfield bounds | none |
| 6 | selected mode vtable `+0x80` at `0x00687558` | first Gather; two ranged invocations per deficient retry |
| 7 | `DAT_00A8B244 == 2` common assign, otherwise selected mode `+0x84` at `0x0068756B` | offline stock: second Gather plus mode chooser draws |
| 8 | draw loading screen and reload theater/rules | no Scenario draw found |
| 9 | `RulesClass__ResetTypeRegistriesAndReloadRules @ 0x006686C0`, call `0x006876AC` | destroys every first-pass House; no Scenario draw |
| 10 | `Read_INI_Basic`, call `0x00687853`; noncampaign branch calls second `Create_Houses @ 0x0068ACFF` | same one ranged invocation per House |
| 11 | selected mode vtable `+0x7C` | no Scenario draw in active stock callbacks |
| 12 | map Read/Fill at `0x006879FF` | first downstream Fill draws |
| 13 | OverlayPack/OverlayDataPack reader at `0x00687A34` | conditional authored Overlay `Mark` draws after Fill |
| 14 | cell recalculation and `[Terrain]` | after Overlay packs |
| 15 | `[Units]`, `[Aircraft]`, `[Infantry]`, `[Structures]` at `0x00687AA7`, `0x00687ABF`, `0x00687ACB`, `0x00687AEA` | each constructed Techno reaches the constructor raw draw |

This is the order the Rust cursor owner must preserve. A later projection may retain precomputed
outcomes, but no second cursor or cursor replacement can cross these steps.

## 3. The Two House Passes

### 3.1 Count and stable order

`ScenarioClass__Create_Houses @ 0x00687F10` uses the same inputs on both calls:

1. all session human nodes in stable ascending signed node-priority byte `+0x53`; equal priorities
   retain source order;
2. valid AI slots in ascending session-slot order, skipping the `-1`/`-3` invalid markers;
3. `Neutral`;
4. `Special`.

Let `N_human_nodes = DAT_00A8DA84` and let `N_valid_ai` be the number of valid entries in the AI
slot range represented by the session AI count (`DAT_00A8B274` in a valid packed stock session).
Then each pass constructs

```text
H = N_human_nodes + N_valid_ai + 2
```

Houses. Observer nodes are included in `N_human_nodes` and therefore consume the House-constructor
timer draw even though Gather excludes observer humans from its required-start count.

The registry reload replaces HouseType objects but does not mutate the human-node array, AI slot
array, their ordering keys, or the valid AI count. Consequently the second pass has the same `H`
and the same semantic roster order as the first. In ordinary offline Rust, where the local human
and every configured opponent are the packed participants and no observer UI is exposed, this
reduces to `participants + Neutral + Special`; the implementation should retain the native formula
rather than bake in that reduced UI assumption.

### 3.2 Exact draw per House

`HouseClass__Constructor @ 0x004F54A0` contains:

```text
004F631F  PUSH 0x708              ; 1800
004F6334  PUSH 0x1C2              ; 450
004F6343  MOV  ECX,[0x00A8B230]
004F6349  ADD  ECX,0x218           ; Scenario RandomClass
004F634F  CALL 0x0065C7E0         ; Random__RandomRanged
```

Thus each pass is `H` logical `RandomRanged(450,1800)` invocations. Each invocation may consume
more than one raw R250 word because native ranged selection rejects out-of-range samples. Tests
must compare logical RNG state/cursor, not assume `H` raw words.

### 3.3 The first set is actually destroyed

The reset is not a harmless type reload. Assembly `0x0066899F..0x006689C1` repeatedly:

1. reads `g_HouseClass_Array_Count @ 0x00A80238`;
2. reads the first pointer from `g_HouseClass_Array @ 0x00A8022C`;
3. invokes its deleting virtual at vtable `+0x20` with argument `1`;
4. repeats until the global count is zero.

The deleting destructor removes the entry from the global array, so the loop reaches zero. No
first-pass House survives into the final scenario. `Read_INI_Basic` then tests
`g_GameMode @ 0x00A8B238` at `0x0068ACAA`; every nonzero mode reaches
`ScenarioClass__Create_Houses @ 0x0068ACFF`. Campaign zero takes the separate campaign player path
and is outside this bridge scope.

This destruction changes object identity but consumes no Scenario RNG. Rust need not allocate and
delete throwaway House objects if it reproduces their constructor draws and retains no first-pass
object state as final state.

## 4. Active Stock Mode Dispatch

Retail `MPModesMD.ini` plus direct vtable bytes give the active offline set:

| Stock id(s) | Retail category / display rows | vtable | `+0x80` | `+0x84` | Assignment family |
|---|---|---:|---:|---:|---|
| `1`, `9` | Battle, Team Game | `0x007EE184` | `0x005D6BE0` | `0x005D6C70` | Battle-family |
| `5`, `6`, `7`, `8` | Megawealth, Duel, Meat Grinder, Naval War (`ManBattle`) | `0x007EE50C` | `0x005D6BE0` | `0x005D6C70` | Battle-family |
| `4` | Unholy Alliance | `0x007EE814` | `0x005D6BE0` | `0x005D6C70` | Battle-family |
| `2` | Free For All | `0x007EE424` | `0x005D6BE0` | `0x005D6C70` | Battle-family |
| `3` | Cooperative | `0x007EE27C` | `0x005D6BE0` | `0x005C2EF0` | Cooperative |

All active stock categories therefore execute the common `+0x80` Gather/preassignment first.
Every Battle-family category executes the common `+0x84`, which gathers again before final House
assignment. Cooperative `+0x84 @ 0x005C2EF0` also begins by gathering again; it then uses its
custom chooser `0x005C2D00`. Current Rust's Cooperative fallback reuses the first vector and is one
Gather short.

Siege has an implemented vtable (`0x007EE6FC`) but no stock retail `MPModesMD.ini` row. It is not an
active stock offline owner and is excluded rather than inferred into the roster.

`DAT_00A8B244 == 2` selects `AssignStartingPoints @ 0x005EE9D0` instead of the selected `+0x84`.
The global defaults to zero; complete write-xref review found its writes in network/WOL paths, not
the offline skirmish shell. Standard `g_GameMode == 5` therefore uses the selected `+0x84` path.
That network branch is not a reason to approximate or delay the offline correction.

## 5. Gather: Authored, Sparse and Deficient Starts

`ScenarioClass__Gather_Start_Positions @ 0x00688380` executes identically each time it is called.

### 5.1 Target and sparse-slot behavior

1. Scan waypoint indices `0..7` until the first sentinel. This yields `authored_prefix`.
2. Compute `required = nonobserver_human_nodes + valid_ai_count`.
3. Set `target = max(authored_prefix, required)`.
4. Visit waypoint indices `0..target-1`; append every nonsentinel entry encountered.
5. Generate fallback cells until the vector length reaches `target`.

This is not equivalent to requiring one contiguous vector. A sparse waypoint within
`0..target-1` is retained even if an earlier slot is sentinel; a sparse waypoint at or above
`target` is ignored. Explicit-start ownership is by vector/start-slot index after this collection,
not by a Rust-normalized list that silently closes holes.

### 5.2 Exact draw block

Every fallback attempt at `0x00688528..0x006885B5` performs, in order:

```text
Y = RandomRanged(0, cell_array_height - 10) + 10 + cell_array_top
X = RandomRanged(10, cell_array_width  - 10)      + cell_array_left
candidate = FootClass__Find_Nearby_Passable_Cell((X,Y), footprint=8x8, ...)
```

The seed rectangle is the MapClass cell-array bounds (`0x0087F90C..0x0087F918`), not LocalSize.
The nearby helper separately applies the LocalSize/playfield diamond. Each attempt is exactly two
logical ranged invocations; either may be rejection-capable. A sentinel result retains both draws
and retries. There is no artificial retry cap.

The first and second Gather calls are independent. Deficient maps can produce different vectors,
and all retries from the first remain in the cursor before the second begins. The `+0x80` table
stores explicit ownership by start-vector index; `+0x84` applies those table indices to its fresh
second vector.

## 6. Why Gather Must Precede Fill

### 6.1 Cell lifetime at the callback

The exact native boundary is:

```text
Clear_Scene deletes/nulls old CellClass objects
  -> read [Map] Size / LocalSize
  -> MapClass__Resize @ 0x00565C10 allocates new CellClass objects
  -> both start callbacks and both Gather calls
  -> rules reset and second Houses
  -> Read/Fill @ 0x006879FF
```

`MapClass__Resize` constructs a fresh `CellClass @ 0x0047BBF0` for each valid cell in the Size
diamond. At Gather time the relevant constructor state is:

- `OverlayType = -1` at `Cell+0x44`;
- land type/default passability class `0` at `Cell+0xEC`;
- level `0`;
- occupation fields `Cell+0x124/+0x128 = 0`;
- no object/Terrain/overlay lists;
- clear flags.

`CellRect` validation at `0x0056E7C0` and cell passability at `0x004834A0` therefore see clear
default cells, limited by the MapClass Size/LocalSize geometry. The current frame was reset to zero
by the scenario-start preparation path, so the nearby search's frame-dependent scan begins from the
native frame-zero state.

### 6.2 Evidence-backed negative dependencies

The following data cannot affect native deficient Gather because it does not exist yet:

- IsoMapPack tile/subtile/slope or resolved land type;
- authored OverlayPack/OverlayDataPack;
- overlay `Mark` expansion;
- TerrainClass objects;
- authored Units, Aircraft, Infantry or Structures;
- any occupancy derived from those objects.

Consequently current Rust's `native_gather_start_positions` dependency on
`ResolvedTerrainGrid`, post-projection `OccupancyGrid`, and a post-Fill Simulation owner is not a
native dependency. It both delays the draws and makes the chosen cell sensitive to data retail had
not loaded.

The correct support object is a narrow pre-Fill MapClass view built from parsed `Size`/`LocalSize`
and default CellClass values. Reusing the common nearby-search implementation is acceptable only if
its query can explicitly select that view and reproduce the native 8x8 scan without reading
resolved terrain, overlays or object occupancy.

## 7. Assignment-Chooser Scenario Draws

### 7.1 Common `+0x80`

`0x005D6BE0` performs the first Gather and writes the explicit ownership/preassignment table at
Scenario `+0x1180`. It does not own a separate private RNG. Duplicate explicit requests resolve in
the native House/table order; later `+0x84` reads the completed table.

### 7.2 Battle family

`0x005D6C70` performs the second Gather, builds the occupied table, then walks non-Special Houses.
An explicitly owned slot is deterministic. For ordinary automatic placement,
`0x005D6890` uses one `RandomRanged(0,n-1)` when no start is occupied; subsequent automatic choices
are deterministic maximum-sum-distance selections. If the explicit table is already nonempty, the
first automatic choice is also deterministic. Observer-specific branches remain represented by
the native House/session model; the ordinary offline Rust UI currently exposes no observer launch.

### 7.3 Cooperative

`0x005C2EF0` performs the second Gather, then `0x005C2D00` partitions starts by
`Scenario+0x11E4` (`NumCoopHumanStartSpots`). While the occupied count is below the human House
count, each automatically assigned human draws `RandomRanged(0,human_start_spots-1)` and linearly
probes to a free entry. Remaining AI Houses deterministically take the first free suffix entry.
The Cooperative `+0x7C` callback has no Scenario draw.

These chooser invocations occur between the two House passes. Raw word count remains
rejection-dependent.

## 8. Generated `.SED` Launch Boundary

### 8.1 Ordinary active retail path

The stock offline chooser does not discover arbitrary `.SED` files as loose maps. Its only random
record is the synthetic `RandMap.Sed` record created or updated by
`ChooseMap__AcceptRandomMapSetup @ 0x005E8590`, and that routine is reached only after the modal
setup returns accepted result `1`. Setup acceptance requires a valid generated preview; Use Map
with no preview calls the generator once before accepting.

Preview generation reaches `RandomMapGenerator__Generate @ 0x00598960` and
`RmgRegion__PlaceStartingPoints @ 0x00594870`. The latter clears the eight staging entries to the
sentinel and then writes generated start cells into Scenario `+0x11C0..+0x11DF`. No path between
accepted setup and `Main__PrepareSession` clears that staging array. The successful Start reseeds
Scenario/Main RNG, but the reseed does not overwrite start staging.

At launch, `ScenarioClass__Read_Scenario @ 0x00684620` recognizes the `.SED` suffix, loads the seed
options, and calls `RandomMapGenerator__Generate(0,0)`. Its nonpreview initialization
`0x00599650` calls Full Init before launch-time region/start generation. Full Init sees
`Scenario+0x34BD`, copies the accepted preview staging starts into active waypoint slots, and runs
the same double-House/two-callback prefix proved above. Later launch generation deterministically
regenerates terrain and overwrites staging for future use, but that later write cannot retroactively
change the prefix.

Thus the active accepted `.SED` prefix is fully specified. Rust's regenerated `MapFile` may contain
the same start cells in a deterministic successful run, but the prefix input should be modeled as
accepted random-map start staging/source provenance, not inferred from construction-trace presence.

### 8.2 Direct external `.SED` exclusion

The low-level suffix predicate is generic and would accept an externally supplied `.SED` filename
if some nonstock caller injected it. That is not an active stock offline chooser mechanism:

- loose-map scanning does not create `.SED` scenario records;
- the synthetic `RandMap.Sed` record is created only after accepted setup;
- the selected-record loader receives only the chooser's committed record;
- stock random-map allowance further limits that sentinel to Battle id `1` and FFA id `2`.

Therefore a fresh-process, no-preview direct `.SED` launch is excluded from this bridge row. It is
not a basis for leaving the active accepted-RMG mechanism approximate. Replay playback and editor
filename injection are separate owners and remain outside this report's stated scope.

### 8.3 Generated overlay boundary

Synthetic Full Init has no authored OverlayPack payload to replay. Generated low decks are stamped
directly later by RMG and do not invoke fixed-map `OverlayClass::Mark`. The generated branch must
therefore execute this prefix, then Fill and the recorded RMG construction trace, without adding an
authored low/high Mark pass.

## 9. Downstream Fill, Mark and Techno Boundary

This report does not re-own the formulas of GSI-04.12/04.13/04.15; it fixes their cursor entry.

- Fill begins only after the second House pass. Its Scenario draws therefore continue from the
  complete prefix cursor.
- Authored OverlayPack reading follows Fill. Procedural low-endpoint `Mark` expansion can consume
  three Scenario words per longitudinal step, and Mark-created animation construction may consume
  further Scenario draws. Those belong after Fill, never before either Gather.
- Cell recalculation and `[Terrain]` follow overlay packs.
- authored `[Units]`, `[Aircraft]`, `[Infantry]`, `[Structures]` follow Terrain in that order.
  `TechnoClass__Constructor @ 0x006F3254` takes one raw Scenario word per constructed Techno before
  later Unlimbo success/failure.
- generated `.SED` construction events occur after synthetic Full Init on the same owner; they do
  not justify replaying authored Overlay `Mark`.

Any downstream owner that currently starts from the match seed or the one-pass plan cursor must be
fed the corrected post-second-House cursor. This report does not certify the downstream mechanism's
own internal completeness.

## 10. Complete Pre-Fill Scenario Invocation Ledger

For one active offline noncampaign load, the bounded interval from successful-Start reseed to the
first Fill invocation contains these Scenario consumers, in this order:

| Consumer | Logical ranged invocations |
|---|---:|
| first House pass | `H` times `RandomRanged(450,1800)` |
| `+0x80` first Gather | `2 * A1`, where `A1` is its total fallback attempt count |
| selected `+0x84` second Gather | `2 * A2`, independently determined |
| Battle-family chooser | `0` or the conditional first-automatic invocation described above |
| Cooperative chooser | one invocation for each automatic human-prefix assignment reached |
| reset/destroy/reload | `0` |
| second House pass | `H` times `RandomRanged(450,1800)` |

No Scenario call was found in the active stock `+0x7C` callbacks, the House destruction loop,
Rules registry reload, or the intervening pre-Fill metadata readers. Network-service paths that
reach random functions in this interval load the distinct Main RNG object `0x00886B88`; they are
not hidden Scenario draws. Logical invocation counts must be advanced with native rejection, so
the raw-word delta is seed-dependent.

## 11. Current Rust at `origin/main` `a3e4ce9a`

### 11.1 Production paths

| Rust surface | Current behavior | Exact mismatch |
|---|---|---|
| `src/sim/scenario_bootstrap.rs::preload_standard_battle_start_plan` | eligible only for exact metadata identities of Battle id `1` or FFA id `2`, nonempty/non-`auto` selected filename, and a complete contiguous start prefix | excludes ids `3..9`, sparse/deficient/custom identity cases despite active callbacks |
| same preload | seeds a temporary `SimRng`, advances `participant_count + 2` House ranged calls once, then Battle assignment | omits the second identical House pass; reduced count has no observer/valid-slot model |
| `PreloadedBattleStartPlan::install_before_terrain` | replaces the fresh loading cursor with the plan's after-cursor | transfer shape is usable, but transferred cursor is incomplete |
| `src/app/loading/init.rs` | creates `ScenarioBootstrapRng` immediately before terrain Fill and optionally installs the plan | no universal pre-Fill House/start transaction exists |
| `native_gather_start_positions` | requires `ResolvedTerrainGrid`, occupancy, playfield state and a Simulation-owned cursor | retail Gather uses fresh default cells before Fill; current result can depend on later authored state |
| `initialize_skirmish_scenario` no-plan branch | after map construction, gathers once for preassignment and again only when non-Cooperative, then assigns | all draws occur after Fill/Mark/Technos; Cooperative is missing its second Gather |
| House initialization | creates final Rust Houses for projection but spends no second House timer pass | neither plan branch represents both retail construction passes |
| mode model | `SkirmishGameMode` retains id, override, filter and flags but not the parsed category/vtable family | exact stock family can be recovered from validated stock ids/override; retaining category is cleaner but not required for smallest fix |
| generated launch | regenerates `.SED`, carries start waypoints and a construction trace | start provenance should explicitly distinguish accepted staging; construction trace cannot stand in for start staging or overlay source |

The relevant files have no `7143d171..a3e4ce9a` diff, so inspecting `origin/main` was not defeated by
the older worktree checkout.

### 11.2 Existing useful pieces to preserve

- `ScenarioBootstrapRng` is the correct single owner for Scenario/Main/MapGen continuation.
- `native_assign_launch_starts` models the common Battle-family final chooser.
- `native_assign_cooperative_launch_starts` models the Cooperative partition/chooser shape.
- Gather's asymmetric Y-then-X ranged block, uncapped retry, and sparse `0..target` scan are already
  represented in Rust.
- loading already has a pre-Fill cursor installation boundary and downstream Fill handoff.

The correction should move and generalize these pieces, not create a bridge-private RNG.

## 12. Smallest Exact Rust Correction

### 12.1 Required production delta

1. Generalize `PreloadedBattleStartPlan` into a stock-offline pre-Fill plan/transaction that is not
   optional for a valid offline launch. Preserve its prestate fingerprint and immutable outcome
   handoff.
2. Derive `H` from the normalized native session roster: human nodes including observers, valid AI
   slots, then Neutral/Special. Advance first-pass `RandomRanged(450,1800)` in stable House order.
3. Parse/retain MapClass `Size` and `LocalSize` early enough to create a default-cell pre-Fill view.
   Run the first Gather and `+0x80` explicit table against that view.
4. Dispatch the second callback by active stock mode family. Ids `1,2,4,5,6,7,8,9` use the common
   Battle path; id `3` uses Cooperative. All perform a second Gather.
5. Preserve both vectors or the exact data needed to apply the `+0x80` table to the second vector;
   preserve final assignment/start table for later House/base projection.
6. Cross the reset boundary without RNG, discard all first-pass object identity, then advance the
   identical second House pass.
7. Install/continue that exact cursor before terrain Fill. Later scenario initialization consumes
   the retained assignment and performs zero Gather, chooser or House-timer draws.
8. Remove filename trimming/`auto` and contiguous-start eligibility as RNG-ownership gates. Invalid
   launch data should fail at its actual validation boundary, not silently move native prefix draws
   after Fill.
9. For accepted generated maps, carry explicit start-staging/source provenance. Continue to suppress
   authored Overlay Mark for materialized generated decks.

The only new substrate required by this correction is the narrow default-cell pre-Fill MapClass
view. Full resolved-terrain construction is not a prerequisite.

### 12.2 Required tests

- `scenario_prefix_runs_two_identical_house_passes_before_fill`: include multiple AI slots and an
  observer-capable roster fixture; assert cursor state after each rejection-capable pass.
- `scenario_prefix_sparse_waypoints_preserve_only_entries_below_target`: exercise `{0,2}` with
  target `2` and target `3`.
- `scenario_prefix_deficient_gathers_are_independent_and_pre_fill`: force a rejected nearby result
  in each Gather; assert exact two-draw retry continuation and that Fill receives the cursor after
  the second House pass.
- `scenario_prefix_gather_ignores_authored_iso_overlay_terrain_and_technos`: two maps with identical
  Size/LocalSize/waypoints but different downstream payloads must yield the same prefix outcome and
  cursor.
- `scenario_prefix_stock_battle_family_ids_share_callbacks`: ids `1,2,4,5,6,7,8,9` all execute two
  Gathers and the common chooser; id `3` executes two Gathers and Cooperative chooser.
- `scenario_prefix_coop_does_not_reuse_first_gather`: deficient first/second vectors differ under
  the same cursor and final assignment uses the second.
- `scenario_prefix_generated_accept_uses_staged_starts`: accepted `RandMap.Sed` launch consumes the
  double-House prefix from accepted staging, then performs no fixed-map Mark replay.
- `scenario_prefix_apply_is_draw_free_after_fill`: applying retained assignments during final House
  projection must not advance Scenario RNG.
- one interleaved order oracle: `house1 -> gather1 -> gather2/chooser -> house2 -> fill -> authored
  low Mark -> Units/Aircraft/Infantry/Structures`, comparing full logical cursor states rather than
  invocation counters alone.

Focused implementation validation belongs in the relevant `--lib` modules. This research-only
pass ran no Cargo command and changed no Rust.

## 13. Coverage Ledger

| Mechanism / question | Status | Direct evidence | Remaining work |
|---|---|---|---|
| Full Init order through Fill | VERIFIED | `0x00686B20`, call sites listed in section 2 | implementation only |
| first/second House call activation | VERIFIED | `0x0068745E`, `0x0068ACAA..0x0068AD04` | none |
| House count and stable order both passes | VERIFIED | `0x00687F10`, session arrays and loop order | none |
| observer House draw vs Gather exclusion | VERIFIED | `0x00687F10`; Gather node `+0x6B` test | none |
| House timer RNG owner/range | VERIFIED | `0x004F631F..0x004F634F` | none |
| first set destruction | VERIFIED | `0x0066899F..0x006689C1` | none |
| registry reload hidden Scenario draws | VERIFIED NEGATIVE | `0x006686C0` call graph plus direct receiver checks | none |
| active stock mode callback table | VERIFIED | vtable bytes plus retail `MPModesMD.ini` fixture | none |
| offline `DAT_00A8B244 != 2` | VERIFIED | initialization and complete write-xref census | network branch out of scope |
| two Gather calls in Battle family | VERIFIED | `0x005D6BE0`, `0x005D6C70` | none |
| two Gather calls in Cooperative | VERIFIED | `0x005D6BE0`, `0x005C2EF0` | none |
| sparse/deficient target algorithm | VERIFIED | `0x00688380` | none |
| exact fallback draw order/ranges/retry | VERIFIED | `0x00688528..0x006885B5` | none |
| fallback cell-state authority | VERIFIED | Full Init order, `0x00565C10`, `0x0047BBF0`, `0x0056E7C0`, `0x004834A0` | Rust substrate implementation |
| fixed authored map activation | VERIFIED | normal `Read_Scenario_INI -> Full_Init` | none |
| accepted generated `.SED` staging | VERIFIED | `0x00596300`, `0x00594870`, `0x00684620`, `0x00598960`, `0x00599650` | explicit Rust provenance |
| fresh external `.SED` active chooser reachability | VERIFIED NEGATIVE | sentinel-only creation and no loose `.SED` discovery | excluded |
| Fill/Mark/Terrain/Techno relative order | VERIFIED | `0x006879FF..0x00687AEA`, `0x006F3254` | downstream owners remain separate |
| current `origin/main` production/test state | VERIFIED | direct `git show origin/main` reads; no relevant commit diff | implementation only |

## 14. Open Questions — Final State

- `[RESOLVED] OQ-1 - Is there one House pass or two? -> Two for every noncampaign game mode; both
  precede Fill.`
- `[RESOLVED] OQ-2 - Does reset retain the first Houses? -> No; the global array is deleting-looped
  to count zero, then rebuilt.`
- `[RESOLVED] OQ-3 - Can the second pass differ in roster count/order? -> Not in a valid unchanged
  session; it uses the same node/AI arrays and stable order after HouseType reload.`
- `[RESOLVED] OQ-4 - Does Cooperative Gather once? -> No; common +0x80 and Cooperative +0x84 each
  call Gather.`
- `[RESOLVED] OQ-5 - Does deficient Gather depend on resolved terrain? -> No; it runs after Resize
  on default cells and before Fill/overlay/Terrain/Technos.`
- `[RESOLVED] OQ-6 - Are sparse waypoints rejected wholesale? -> No; nonsentinel entries below the
  computed target are retained even after a prefix hole.`
- `[RESOLVED] OQ-7 - Which stock modes share Battle assignment? -> ids 1,2,4,5,6,7,8,9; id 3 is
  Cooperative.`
- `[RESOLVED] OQ-8 - What supplies generated launch starts before launch regeneration reaches its
  start phase? -> accepted preview staging at Scenario +0x11C0, copied by synthetic Full Init.`
- `[RESOLVED] OQ-9 - Can stock offline launch a fresh external .SED without preview staging? -> No;
  `.SED` is not loose-map discovered, and the only chooser sentinel is installed after accepted
  generated preview.`

**BLOCKED:** none inside claimed scope.

**UNKNOWN:** none inside claimed scope.

**Evidence-backed exclusions:** Campaign, network state `DAT_00A8B244 == 2`, Siege without a retail
row, replay playback, editor/direct filename injection, malformed external seed files, and
downstream Fill/Mark/Techno internal formulas.

## 15. Ghidra Annotation Candidates

This pass was read-only and changed no Ghidra metadata. Candidates for a later certainty-gated sync:

- `0x0068745E`: comment as first, disposable noncampaign House pass.
- `0x0068ACFF`: comment as second/final noncampaign House pass after registry reset.
- `0x0066899F`: comment that the loop deletes House array element zero until count reaches zero.
- `0x00687558`: comment that all active stock categories enter first Gather through `+0x80`.
- `0x00688528`: comment that deficient Gather sees resized default cells before Fill.

No rename is required to implement the Rust correction.

## 16. Implementation Handoff

**Implementation readiness:** READY for the active offline bridge scope. The current design's
optional plan/no-plan split must not be implemented as written; use the universal pre-Fill
transaction in section 12.

**Primary affected owners:**

- `src/sim/scenario_bootstrap.rs` — plan type, House-pass advancement, Gather input view, mode
  dispatch, draw-free later application;
- `src/app/loading/init.rs` — construct/execute the universal transaction before Fill and preserve
  one cursor;
- `src/skirmish_modes.rs` or validated launch-mode adapter — retain/derive active stock callback
  family;
- map header/pre-Fill substrate — expose Size/LocalSize default-cell MapClass geometry without
  resolving authored terrain;
- generated-map launch state — retain accepted start-staging/source provenance.

**Largest regression risks:** advancing House timer calls as raw words instead of native ranged
invocations; performing only one Cooperative Gather; reading resolved terrain/occupancy during
fallback; applying retained assignment with a second draw; and replaying fixed-map Overlay Mark on
generated materialized decks.

## 17. Sources

- Fresh read-only Ghidra decompile/disassembly/callgraph/xref work in active retail
  `gamemd.exe`: all addresses in the report header, plus `ScenarioClass__Constructor @ 0x006832C0`,
  `ScenarioClass__Set_Defaults @ 0x00683610`, and `Main__PrepareSession @ 0x0052D9A0`.
- YR retail roster fixture derived from `MPModesMD.ini`:
  `tests/fixtures/ini/mpmodesmd_stock_contract.ini`.
- Current Rust at `origin/main` `a3e4ce9a`: `src/sim/scenario_bootstrap.rs`,
  `src/app/loading/init.rs`, `src/skirmish_modes.rs`, `src/app/random_map_lifecycle_tests.rs`.
- Reconciled prior native reports:
  `RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`,
  `RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md`,
  `SKIRMISH_GATHER_START_POSITIONS_AND_BATTLE_ASSIGNMENT_GHIDRA_REPORT.md`,
  `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`,
  `SKIRMISH_SELECTED_MPMODE_OBJECT_INI_OVERRIDE_LOAD_GHIDRA_REPORT.md`,
  `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`, and
  `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`.
- `C:\Users\enok\Documents\OpenTS` was consulted only as a readable navigation lead for inherited
  scenario/map/start concepts. Every material conclusion above was independently verified in active
  YR `gamemd.exe` and retail data; no OpenTS behavior is parity authority.
