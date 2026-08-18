# TIBTRE Source Overlay Type Reachability After Unlimbo - Ghidra Research Report

**Address(es):** `0x00686B20` (`ScenarioClass::Full_Init`), `0x005FD2E0` (`ReadMapOverlayPacks`), `0x0071CA70` (`TerrainClass__Read_Map_Section`), `0x0071BB90` (`TerrainClass__Constructor`), `0x0071D000` (`TerrainClass__Unlimbo`), `0x0071C730` (`TerrainClass::AI`), `0x00483780` (`CellClass::SpreadTiberium`), `0x005FDD20` (`CellClass::OverlayToTiberiumIndex`), `0x005FC380` (`OverlayClass__Constructor`), `0x0047C550` (terrain-object overlay placement blocker), `0x004838E0` (`CellClass::CanPlaceTiberium`), `0x00487190` (`CellClass::PlaceTiberium`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Reconcile `SpreadTiberium(force=true)` source-cell overlay type selection with `TerrainClass::Unlimbo` clearing of source-cell tiberium overlays, including stock map-load reachability and bounded modded/map-edit restore cases.  
**Non-Scope:** TIBTRE animation timing, target-cell acceptance gates beyond source-cell reachability, full `PlaceTiberium` side effects, full savegame serialization, and trigger/action deletion matrices.  
**Confidence:** High for standard YR map-load and gameplay reachability; Medium for map-editor/live-edit restore cases because exact editor `g_GameActive` state was not redrained.  
**Active in YR:** Yes for the standard stock path. Conditional for source-overlay type propagation: active only if a recognized tiberium overlay exists on the source cell at the later `TerrainClass::AI -> SpreadTiberium` call.

## 0. Investigation Setup

- Target question: Can source-cell tiberium overlay type propagation in `SpreadTiberium(force=true)` survive or become reachable after `TerrainClass::Unlimbo` clears source-cell overlays?
- Non-goals: Do not repeat timing, force flag, target rejection, placement queue, or terrain light investigations; do not modify Rust, INI, in-repo docs, or Ghidra state.
- Evidence needed to mark COMPLETE: decompile plus source/order evidence for map load ordering, `Unlimbo` clear, `SpreadTiberium` source overlay type read, and later overlay placement blockers/writers relevant to restoring source-cell ore.
- Stop conditions: Stop once stock reachability, modded/map-edit reachability, Rust priority, negative facts, and implementation handoff are resolved or explicitly deferred.

## 1. Overview

`CellClass::SpreadTiberium(force=true)` genuinely supports source-cell overlay type propagation, but stock TIBTRE terrain trees normally do not reach that branch with a same-cell ore overlay. Standard map load stamps `[OverlayPack]` ore first, then constructs `[Terrain]` objects; `TerrainClass::Unlimbo` clears any source-cell overlay whose `OverlayTypeClass+0x2A9` tiberium byte is set. Therefore stock TIBTRE source cells reach spawn time with no tiberium overlay and default to tiberium type index `0` (`Riparius`).

The propagation branch is still real for unusual states. If a recognized tiberium overlay is directly restored onto the live source cell after `Unlimbo` and before the AI midpoint spawn, `SpreadTiberium` will derive the spawned type from that overlay. Normal YR placement paths make that hard: `PlaceTiberium`/`CanPlaceTiberium` rejects a live `SpawnsTiberium` terrain-object cell, and `OverlayClass__Constructor` blocks overlay placement on terrain-object cells while `g_GameActive != 0`.

## 2. Class Layout / Key Offsets

| Owner | Offset / value | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `CellClass` | `+0x44` | `OverlayTypeIndex`; `-1` means no overlay. Source for `OverlayToTiberiumIndex`. | Yes |
| `CellClass` | `+0x11E` | Overlay data/density byte; cleared by `TerrainClass::Unlimbo` when it clears source overlay. | Yes |
| `CellClass` | `+0xE4` | Object-list head scanned by `CanPlaceTiberium` and `FUN_0047C550`. | Yes during active game |
| `OverlayTypeClass` | `+0x294` | Overlay array index written to `CellClass+0x44` by overlay stamping. | Yes |
| `OverlayTypeClass` | `+0x2A9` | Tiberium/ore overlay byte used by `Unlimbo` clear and `OverlayToTiberiumIndex`. | Yes |
| `TerrainClass` | `+0x27/+0x28/+0x29` | Object coordinates used by AI to resolve the source cell at spawn midpoint. | Yes |
| `TerrainClass` | `+0x32` / byte `+0xC8` | `TerrainTypeClass*`. AI reads `SpawnsTiberium` and `IsAnimated`. | Yes |
| `TerrainTypeClass` | `+0x2B1` | `SpawnsTiberium`; enables AI spawn and makes `CanPlaceTiberium` reject that terrain object cell. | Yes for stock TIBTRE |
| `TerrainTypeClass` | `+0x2B3` | `IsAnimated`; enables the AI animation/spawn path. | Yes for stock TIBTRE |
| `TiberiumClass` | `+0x98` | Type index returned when source overlay belongs to this TiberiumClass image range. | Yes |
| `TiberiumClass` | `+0xE0/+0xE8/+0xEC` | Base overlay image pointer and flat/slope image counts used by overlay-to-type mapping. | Yes |

## 3. Core Logic

### 3.1 Standard Map Load Order

Active in YR: Yes. `ScenarioClass::Full_Init @ 0x00686B20` is the standard scenario/skirmish initialization path.

The load order relevant to source-cell overlays is:

1. `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`
2. `ReadMapOverlayPacks @ 0x005FD2E0`
3. all-cell `CellClass::RecalcAttributes`
4. `FUN_005FDDF0` (touched, not material here)
5. `TerrainClass__Read_Map_Section @ 0x0071CA70`
6. `TiberiumClass__InitGrowthQueues_All`
7. `TiberiumClass__InitSpreadQueues_All`

Evidence: decompile of `0x00686B20` shows direct sequential calls: `ReadMapOverlayPacks(); ... CellClass__RecalcAttributes(this); ... TerrainClass__Read_Map_Section(); ... TiberiumClass__InitGrowthQueues_All(); TiberiumClass__InitSpreadQueues_All();`.

Implication: map `[OverlayPack]` tiberium on a TIBTRE cell is already in `CellClass+0x44` before the terrain object is constructed and unlimboed. Queue initialization runs after terrain placement, so any same-cell tiberium overlay cleared by `Unlimbo` is not later restored by initial tiberium queue setup.

### 3.2 OverlayPack Stamps Source Overlays Before Terrain Exists

Active in YR: Yes. `ReadMapOverlayPacks @ 0x005FD2E0` is called from `ScenarioClass::Full_Init`.

`ReadMapOverlayPacks` decodes `[OverlayPack]`, reads each byte as an overlay type index, validates the overlay has SHP or `CellAnim`, then constructs an `OverlayClass` at that cell. The constructor path stamps the overlay into the cell. The second pass decodes `[OverlayDataPack]` and writes `CellClass+0x11E` for every in-bounds cell.

Tiny details that matter:

- Empty overlay byte is `0xFF`; non-`0xFF` values are direct indices into `g_OverlayTypeClass_Array`.
- The first pass saves existing `+0x11E` and restores it only for bridge overlay ids `0x18`, `0x19`, `0xED`, `0xEE`.
- The second `[OverlayDataPack]` pass writes `+0x11E` unconditionally for in-bounds cells, independent of whether a later terrain object will clear the overlay.
- During this standard order, same-cell TIBTRE terrain objects do not yet exist, so later terrain-object blockers cannot reject the initial overlay stamp.

Evidence: decompile `0x005FD2E0`; `OverlayClass__Constructor @ 0x005FC380`.

### 3.3 TerrainClass Map Read And Unlimbo Clear

Active in YR: Yes. Stock map `[Terrain]` entries create `TerrainClass` objects; stock `TIBTRE01..03` are valid terrain types.

`TerrainClass__Read_Map_Section @ 0x0071CA70` reads the `"Terrain"` section, resolves the value string through `TerrainTypeClass__Find_Or_Allocate`, decodes the key as `rx = key % 1000`, `ry = key / 1000` for modern map format, allocates `0xE0` bytes, and calls `TerrainClass__Constructor`.

`TerrainClass__Constructor @ 0x0071BB90` initializes animation state and calls `TerrainClass__Unlimbo` at the cell center. `TerrainClass__Unlimbo @ 0x0071D000` then:

- calls `ObjectClass__Reveal`;
- increments the eight neighboring cells' `CellClass+0x122` byte;
- resolves the terrain source cell from the object coordinates;
- reads `CellClass+0x44`;
- if an overlay exists and `g_OverlayTypeClass_Array[overlay_id]+0x2A9 != 0`, writes `CellClass+0x44 = -1` and `CellClass+0x11E = 0`.

Implication: stock or map-authored ore/gem/tiberium overlays on the TIBTRE source cell are removed during terrain placement before the first TIBTRE AI spawn can occur.

Evidence: decompile `0x0071CA70`, `0x0071BB90`, `0x0071D000`.

### 3.4 SpreadTiberium Still Reads The Runtime Source Overlay At Spawn Time

Active in YR: Yes. `TerrainClass::AI @ 0x0071C730` calls this path for stock TIBTRE at the animation midpoint.

`TerrainClass::AI` resolves the terrain object's current cell and calls `CellClass::SpreadTiberium(1)`. `SpreadTiberium @ 0x00483780` then calls `CellClass::OverlayToTiberiumIndex @ 0x005FDD20` on the source cell overlay. With `force=true`, if the helper returns `-1`, the local tiberium type is set to `0`. If the helper returns a valid type index, that type index is used for adjacent placement.

`OverlayToTiberiumIndex` returns:

- `-1` when the source `OverlayTypeIndex == -1`;
- `-1` when `OverlayTypeClass+0x2A9 == 0`;
- the matching `TiberiumClass+0x98` when the overlay index falls within a registered TiberiumClass image range;
- fallback `0` if the overlay is flagged tiberium but does not fall within any registered range, after logging.

Implication: source-overlay propagation is not dead code. It is unreachable for ordinary stock same-cell map ore because `Unlimbo` clears that overlay first, but it becomes reachable if a recognized tiberium overlay is somehow present on the source cell at AI spawn time.

Evidence: decompile `0x0071C730`, `0x00483780`, `0x005FDD20`.

### 3.5 Normal Later Ore Placement Does Not Restore Source-Cell Overlay

Active in YR: Yes for normal game tiberium placement.

Normal `CellClass::PlaceTiberium @ 0x00487190` calls `CanPlaceTiberium @ 0x004838E0` for new-cell germination. `CanPlaceTiberium` scans the active object list when `g_GameActive != 0`; if it finds RTTI `0x24` (`TerrainClass`) and that terrain object's type has `SpawnsTiberium != 0`, it rejects the cell. Therefore a live TIBTRE source cell is not a valid target for normal new tiberium placement.

The additive/grow-existing branch in `PlaceTiberium` also cannot bootstrap a stock source cell after `Unlimbo`, because it requires an existing tiberium overlay on the cell. `Unlimbo` removed that overlay.

Evidence: decompile `0x00487190`, `0x004838E0`, `0x0071D000`.

### 3.6 Direct Overlay Placement After Terrain Is Usually Blocked During Active Game

Active in YR: Conditional.

`OverlayClass__Constructor @ 0x005FC380` directly stamps overlays through `ObjectClass__Reveal` only after checking `FUN_0047C550(0)`. `FUN_0047C550 @ 0x0047C550` scans the cell object list when `g_GameActive != 0` and returns a terrain object (`RTTI == 0x24`) if one is present. If it returns nonzero, `OverlayClass__Constructor` does not reveal/stamp the overlay.

Implication: during normal active gameplay, direct `OverlayClass` placement also tends to be blocked on a live TIBTRE source cell, even before considering `CanPlaceTiberium`. This is broader than the tiberium placement gate: it blocks any overlay stamp when a terrain object is on that cell.

Map-editor or pre-active direct-cell-write cases remain plausible because `FUN_0047C550` is gated by `g_GameActive`. In those cases, if an editor/tool/mod path directly writes a recognized tiberium overlay to the source cell after `TerrainClass::Unlimbo` and before `TerrainClass::AI` reaches its midpoint, `SpreadTiberium` will use that source overlay's type.

Evidence: decompile `0x005FC380`, `0x0047C550`, `0x00483780`.

## 4. INI Keys

| INI key / section | Stock YR value | Effect in this slice | Binary evidence | Active in YR |
|---|---|---|---|---|
| `[TIBTRE01..03] SpawnsTiberium` | `yes` | Enables terrain AI spawn and makes `CanPlaceTiberium` reject the tree's own cell as a placement target. | `TerrainClass::AI @ 0x0071C730`, `CanPlaceTiberium @ 0x004838E0`, `ini/rulesmd.ini` | Yes |
| `[TIBTRE01..03] IsAnimated` | `yes` | Enables the TIBTRE animation/midpoint AI path that eventually calls `SpreadTiberium(1)`. | `0x0071C730`, `ini/rulesmd.ini` | Yes |
| `[TIBTRE01..03] AnimationRate` | `3` | Timing only; confirms stock TIBTRE reaches the delayed AI path rather than an instant load-time spawn. | `0x0071C730`, `ini/rulesmd.ini` | Yes |
| `[TIBTRE01..03] AnimationProbability` | `.003` | Probability gate for later source-cell read. | `0x0071C730`, `ini/rulesmd.ini` | Yes |
| `[OverlayTypes] TIB01..TIB20`, `TIB2_01..`, etc. | `Tiberium=yes` | Sets the overlay-type tiberium byte read by `Unlimbo` and `OverlayToTiberiumIndex`. | `0x0071D000`, `0x005FDD20`, `ini/rulesmd.ini` | Yes |
| `[Tiberiums] 0` | `Riparius` | Forced no-overlay source default. | `TiberiumClass__ReadINI_All @ 0x00721D10`, `ini/rulesmd.ini` | Yes |
| `[Tiberiums] 1` | `Cruentus` | Type used only if the source overlay maps to Cruentus; not the TerrainClass call literal. | `0x00721D10`, `ini/rulesmd.ini` | Yes |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass::Full_Init @ 0x00686B20` | Proves standard load order: overlays before terrain, queues after terrain. | decompile | Yes |
| `ReadMapOverlayPacks @ 0x005FD2E0` | Initial overlay/data stamp from map `[OverlayPack]` and `[OverlayDataPack]`. | decompile | Yes |
| `TerrainClass__Read_Map_Section @ 0x0071CA70` | Reads `[Terrain]` after overlay packs. | decompile and caller order in `0x00686B20` | Yes |
| `TerrainClass__Unlimbo @ 0x0071D000` | Clears source-cell tiberium overlay/data during placement. | decompile | Yes |
| `TerrainClass::AI @ 0x0071C730` | Later reads source cell and calls `SpreadTiberium(1)`. | decompile | Yes |
| `CellClass::SpreadTiberium @ 0x00483780` | Runtime source overlay to tiberium type selection/default. | decompile | Yes |
| `CellClass::OverlayToTiberiumIndex @ 0x005FDD20` | Maps source overlay id to tiberium type or default/failure. | decompile | Yes |
| `CellClass::CanPlaceTiberium @ 0x004838E0` | Rejects normal new ore placement on live `SpawnsTiberium` terrain cells. | decompile | Yes |
| `OverlayClass__Constructor @ 0x005FC380` + `FUN_0047C550 @ 0x0047C550` | Blocks direct overlay stamping on terrain-object cells while `g_GameActive != 0`. | decompile | Conditional |

## 6. Current Rust Implementation Status

Current Rust surface:

- `src/sim/terrain_spawn.rs::TerrainSpawnerState` stores the terrain type ref and cached probability only; it has no source-overlay type field or live source-cell overlay lookup.
- `src/sim/terrain_spawn.rs::seed_terrain_spawners` seeds from map terrain objects whose rules have `spawns_tiberium && is_animated`.
- `src/sim/terrain_spawn.rs::tick_terrain_spawners` always places `ResourceType::Ore` through `place_tiberium_additive`.
- `src/sim/terrain_spawn.rs` resolves `default_ore_overlay_id` as the first overlay whose name starts with `"TIB"`, which approximates type-0 Riparius rather than using `TiberiumClass[0].Image`.
- Current Rust does not model `TerrainClass::Unlimbo` clearing a source-cell tiberium overlay during map load.

Rust priority decision from this slice:

- For stock TIBTRE parity, Rust can safely default source type to Riparius/Ore for now, as long as map-load source-cell tiberium overlays are cleared or ignored for source type. This matches the standard binary path.
- Do not implement source-overlay type propagation as an urgent prerequisite for stock maps. Add a future hook for modded/editor states where a source-cell overlay can exist at spawn time.
- If Rust preserves map overlay/resource data under a TIBTRE source cell without applying the `Unlimbo` clear, it can accidentally make source-overlay propagation appear reachable in cases where GameMD would have defaulted to type 0. That would be a stock-parity bug.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required setup lines | verified | Section 0 | none |
| Standard load order: overlay before terrain | verified | `ScenarioClass::Full_Init @ 0x00686B20` | none |
| OverlayPack same-cell stamp | verified | `ReadMapOverlayPacks @ 0x005FD2E0`, `OverlayClass__Constructor @ 0x005FC380` | none |
| Terrain map read and constructor | verified | `0x0071CA70`, `0x0071BB90` | none |
| Unlimbo source-cell tiberium overlay/data clear | verified | `0x0071D000` | exact public field name for `+0x2A9` belongs to overlay-type docs, not this slice |
| TerrainClass AI source-cell lookup | verified | `0x0071C730` | none |
| SpreadTiberium source-overlay type/default | verified | `0x00483780`, `0x005FDD20` | none |
| Initial growth/spread queues after terrain read | verified | `0x00686B20` call order | queue internals out of scope |
| Normal PlaceTiberium cannot restore live TIBTRE source overlay | verified | `0x00487190`, `0x004838E0`, `0x0071D000` | none for stock reachability |
| Direct overlay constructor active-game terrain blocker | verified | `0x005FC380`, `0x0047C550` | exact map-editor `g_GameActive` state deferred |
| Map-editor/live-edit overlay restoration | touched-not-exhausted | `0x005FC380`, `0x0047C550`, `OverlayClass::Mark @ 0x005FC8E0` | requires editor runtime context if full UI/editor behavior matters |
| Savegame restore order for terrain and overlays | deferred | not inspected | separate serialization investigation |
| Current Rust terrain spawner type behavior | verified | `src/sim/terrain_spawn.rs` scan; Codegraph context | no Rust changes made |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-01 - What is the exact target question? -> Whether source-overlay type propagation is reachable after TerrainClass::Unlimbo clears same-cell overlays, and whether Rust needs it now.` (evidence: user target)
- `[RESOLVED] OQ-02 - What is non-scope? -> Timing, target gates, full placement effects, full serialization, trigger deletion matrices, Rust edits.` (evidence: user target and report header)
- `[RESOLVED] OQ-03 - What evidence marks COMPLETE? -> Load order, Unlimbo clear, SpreadTiberium source read, later placement blockers/writers, Rust handoff.` (evidence: section 0)
- `[RESOLVED] OQ-04 - What are the stop conditions? -> Stop after stock reachability, modded/editor cases, Rust priority, negatives, and handoff are resolved/deferred.` (evidence: section 0)
- `[RESOLVED] OQ-05 - Does standard Full_Init load overlays before terrain? -> Yes, `ReadMapOverlayPacks` precedes all-cell Recalc and `TerrainClass__Read_Map_Section`.` (evidence: `0x00686B20`)
- `[RESOLVED] OQ-06 - Does OverlayPack write the source cell before TIBTRE exists? -> Yes, it constructs overlay objects from `[OverlayPack]` before `[Terrain]` objects are read.` (evidence: `0x00686B20`, `0x005FD2E0`, `0x005FC380`)
- `[RESOLVED] OQ-07 - Does TerrainClass constructor immediately Unlimbo map terrain? -> Yes, constructor calls `TerrainClass__Unlimbo` at cell center unless sentinel coords are used.` (evidence: `0x0071BB90`)
- `[RESOLVED] OQ-08 - Does Unlimbo clear source-cell tiberium overlays? -> Yes, if `Cell+0x44 != -1` and overlay type byte `+0x2A9 != 0`, it writes `Cell+0x44=-1` and `Cell+0x11E=0`.` (evidence: `0x0071D000`)
- `[RESOLVED] OQ-09 - Are stock TIBTRE overlay keys active in YR? -> Yes, `TIBTRE01..03` have `SpawnsTiberium=yes`, `IsAnimated=yes`, and stock tiberium overlays have `Tiberium=yes`.` (evidence: `ini/rulesmd.ini`, `0x0071C730`, `0x005FDD20`)
- `[RESOLVED] OQ-10 - Does SpreadTiberium still support source-overlay type selection? -> Yes, it calls `OverlayToTiberiumIndex`; forced no-overlay defaults to type 0.` (evidence: `0x00483780`, `0x005FDD20`)
- `[RESOLVED] OQ-11 - Is source-overlay type propagation reachable for ordinary stock TIBTRE map load? -> No for same-cell map ore: overlay is loaded first and then cleared by Unlimbo before any AI spawn.` (evidence: `0x00686B20`, `0x0071D000`, `0x0071C730`)
- `[RESOLVED] OQ-12 - Can initial tiberium queue setup restore same-cell source overlay after Unlimbo? -> No evidence of restore; queue init runs after terrain read but consumes remaining cell overlays, and the same-cell overlay has already been cleared.` (evidence: `0x00686B20`, `0x0071D000`)
- `[RESOLVED] OQ-13 - Can normal PlaceTiberium later place new ore on the live TIBTRE source cell? -> No in active gameplay; `CanPlaceTiberium` rejects terrain objects whose type has `SpawnsTiberium`.` (evidence: `0x004838E0`, `0x00487190`)
- `[RESOLVED] OQ-14 - Can PlaceTiberium's grow-existing branch bootstrap source type after Unlimbo? -> No for stock cleared cells because grow-existing requires an existing tiberium overlay.` (evidence: `0x00487190`, `0x0071D000`)
- `[RESOLVED] OQ-15 - Can direct OverlayClass placement restore source overlay in active gameplay? -> Usually blocked; constructor calls `FUN_0047C550`, which returns a terrain object on that cell when `g_GameActive != 0`, preventing reveal/stamp.` (evidence: `0x005FC380`, `0x0047C550`)
- `[RESOLVED] OQ-16 - Is a modded/map-editor restore state theoretically reachable? -> Yes conditionally if a recognized tiberium overlay is written after Unlimbo by a path not blocked by active-game terrain-object checks, such as editor/pre-active direct cell writes or custom memory/save state.` (evidence: `0x00483780`, `0x005FDD20`, `0x005FC380`, `0x0047C550`)
- `[RESOLVED] OQ-17 - Does current Rust model source-overlay type propagation? -> No; `terrain_spawn.rs` always creates `ResourceType::Ore` and uses a default ore overlay id.` (evidence: `src/sim/terrain_spawn.rs`)
- `[RESOLVED] OQ-18 - Should Rust implement source-overlay type propagation now for stock TIBTRE? -> No; stock path can default to Riparius if Rust also clears/ignores source-cell tiberium overlays during terrain placement. Add a documented future hook.` (evidence: `0x00686B20`, `0x0071D000`, `0x00483780`)
- `[DEFERRED] OQ-19 - Does binary savegame restore ever create terrain+overlay same-cell states that differ from map load?` (category: `requires-different-system-context`; reason: savegame serialization order is outside this map-load reachability slice; next-step-if-pursued: investigate TerrainClass and Cell/Overlay save/load slots together.)
- `[DEFERRED] OQ-20 - What exact map-editor UI path can place a tiberium overlay on a live TIBTRE cell?` (category: `requires-different-system-context`; reason: editor input/tool state and `g_GameActive` value need runtime/editor context; next-step-if-pursued: trace editor overlay tool owner and `OverlayClass__Constructor` caller under map editor mode.)

Adversarial corner-case answers:

- A stock map with `[OverlayPack]` ore and `[Terrain]` TIBTRE on the same cell defaults to type 0 at spawn because `Unlimbo` clears the source overlay.
- A stock map with blue ore adjacent to a TIBTRE does not make the TIBTRE spawn blue ore; source type is the tree cell, not neighbors.
- A modded same-cell blue source overlay saved in `[OverlayPack]` still gets cleared before spawn if its overlay type has `+0x2A9`.
- A direct post-Unlimbo source-cell Cruentus overlay, if somehow written and preserved, makes `SpreadTiberium` use Cruentus because the source overlay is read at spawn time.
- Normal active-game new ore spread cannot restore source overlay on a live TIBTRE cell because the terrain object gate rejects `SpawnsTiberium` terrain cells.

## 9. Negative Facts / Do Not Do

- Do not infer TIBTRE source type from adjacent ore. `SpreadTiberium` reads the source cell's `OverlayTypeIndex`, not neighboring cells. Active in YR: Yes; evidence `0x00483780`, `0x005FDD20`.
- Do not preserve same-cell map ore under stock TIBTRE and use it as the source type. GameMD loads the overlay first, then `TerrainClass::Unlimbo` clears it. Active in YR: Yes; evidence `0x00686B20`, `0x0071D000`.
- Do not implement urgent stock TIBTRE source-overlay propagation before map-load Unlimbo clearing. Without the clear, propagation would reproduce a state GameMD removes. Active in YR: Yes; evidence `0x0071D000`, `src/sim/terrain_spawn.rs`.
- Do not default forced no-overlay TIBTRE to type `1`. The forced no-overlay default is type `0` Riparius; type `1` is Cruentus. Active in YR: Yes; evidence `0x00483780`, `0x00721D10`, `ini/rulesmd.ini [Tiberiums]`.
- Do not assume normal active-game overlay construction can freely stamp ore onto a live terrain object. `OverlayClass__Constructor` calls a terrain-object blocker while `g_GameActive != 0`. Active in YR: Conditional; evidence `0x005FC380`, `0x0047C550`.
- Do not treat `PlaceTiberium` grow-existing behavior as a way for TIBTRE source cells to recover source type after `Unlimbo`. It needs an existing overlay, and the stock source overlay was cleared. Active in YR: Yes; evidence `0x00487190`, `0x0071D000`.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard map load stamps overlays before terrain objects, then `TerrainClass::Unlimbo` clears same-cell tiberium overlays. | `0x00686B20`, `0x005FD2E0`, `0x0071CA70`, `0x0071BB90`, `0x0071D000` | missing: Rust seeds spawners but does not model source-cell overlay clear | map load overlay/resource seeding; `src/sim/terrain_spawn.rs`; future terrain object lifecycle surface | During terrain object placement, remove/ignore same-cell tiberium overlay/resource for `SpawnsTiberium` terrain sources. | `tibtree_unlimbo_clears_same_cell_tiberium_overlay_before_source_type_resolution` | Do not let a map-authored same-cell ore overlay change stock TIBTRE source type. |
| Forced no-source-overlay TIBTRE defaults to type index 0 Riparius. | `0x00483780`, `0x005FDD20`, `0x00721D10`, `ini/rulesmd.ini [Tiberiums]` | mostly matched by always placing Ore, but overlay id is approximate first `TIB*` | `src/sim/terrain_spawn.rs`, future TiberiumClass metadata | Keep stock TIBTRE source type as Riparius/Ore when no source overlay exists; later resolve overlay image through type-0 metadata. | `tibtre_stock_source_without_overlay_spawns_riparius_type_zero` | Do not use type 1 or adjacent ore type as the stock default. |
| Source-overlay type propagation is real if a recognized tiberium overlay is present at spawn time. | `0x00483780`, `0x005FDD20` | missing/future hook; current Rust always places Ore | future source-overlay lookup / tiberium metadata; `src/sim/terrain_spawn.rs` | Add a documented future hook to derive source type from current source-cell overlay after Unlimbo/lifecycle state exists. | `modded_tibtree_post_unlimbo_source_overlay_controls_spawned_type` | Do not implement this before also modeling the Unlimbo clear, or stock maps can diverge. |
| Normal active-game tiberium placement does not restore source-cell ore on a live TIBTRE cell. | `0x004838E0`, `0x00487190`, `0x0071D000` | current Rust can keep resource nodes on spawner cells unless explicitly cleared | `src/sim/terrain_spawn.rs`; resource/overlay grid load reconciliation | Reject or clear source-cell resource/overlay for live `SpawnsTiberium` terrain object cells; do not rely on later spread to fix it. | `tibtre_source_cell_resource_is_not_restored_by_growth_or_spread` | Do not let additive resource nodes on the source cell become source type evidence. |
| Active-game direct overlay construction is blocked on terrain-object cells by `FUN_0047C550`. | `0x005FC380`, `0x0047C550` | no live terrain object overlay placement surface yet | future map editor / overlay placement / terrain lifecycle | If adding live overlay editing/placement, preserve terrain-object blocker semantics for active gameplay; editor mode can be handled separately. | `active_game_overlay_place_on_live_tibtree_source_cell_is_rejected` | Do not make editor/pre-active behavior the default active-game behavior. |

### Stale Docs / Follow-up Docs

- No new stale-doc replacement is required beyond the prior `TERRAIN_CLASS_GHIDRA_REPORT.md` correction: `TerrainClass::AI` passes `force=true`, not tiberium type `1`.
- Add this clarification to any future TIBTRE implementation contract: "Source-overlay type propagation is a real `SpreadTiberium` behavior, but stock TIBTRE map-load source cells normally have no source overlay because `TerrainClass::Unlimbo` clears same-cell tiberium overlays after `[OverlayPack]` and before the first AI spawn. Implement stock default type 0 first; leave source-overlay propagation as a future mod/editor hook gated behind the Unlimbo clear."

## 11. Remaining Uncertainty

- Full binary savegame restore ordering for cell overlays plus terrain object runtime state was not investigated. This is not needed for stock map-load parity, but it could affect exotic restored states.
- Exact map-editor UI pathway for placing a tiberium overlay onto a live TIBTRE source cell was not drained. The report only proves the lower-level conditional blocker and the fact that `SpreadTiberium` would use the overlay if present.
- The public semantic name of `OverlayTypeClass+0x2A9` is kept as "tiberium overlay byte" here. The exact parser field name belongs to overlay-type layout docs.

## Sources

- Ghidra read-only decompile: `ScenarioClass::Full_Init @ 0x00686B20`
- Ghidra read-only decompile: `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`
- Ghidra read-only decompile: `ReadMapOverlayPacks @ 0x005FD2E0`
- Ghidra read-only decompile: `OverlayClass__Constructor @ 0x005FC380`
- Ghidra read-only decompile: `OverlayClass__Mark @ 0x005FC8E0` (touched only)
- Ghidra read-only decompile: `FUN_0047C550`
- Ghidra read-only decompile: `TerrainClass__Read_Map_Section @ 0x0071CA70`
- Ghidra read-only decompile: `TerrainClass__Constructor @ 0x0071BB90`
- Ghidra read-only decompile: `TerrainClass__Unlimbo @ 0x0071D000`
- Ghidra read-only decompile: `TerrainClass::AI @ 0x0071C730`
- Ghidra read-only decompile: `CellClass::SpreadTiberium @ 0x00483780`
- Ghidra read-only decompile: `CellClass::OverlayToTiberiumIndex @ 0x005FDD20`
- Ghidra read-only decompile: `CellClass::CanPlaceTiberium @ 0x004838E0`
- Ghidra read-only decompile: `CellClass::PlaceTiberium @ 0x00487190`
- Prior reports referenced: `TIBTRE_SPREADTIBERIUM_FORCE_TYPE_AND_FLAG_GATE_GHIDRA_REPORT.md`, `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`, `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`, `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`, `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini` `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]`, tiberium overlay sections, and `[Tiberiums]`
- Rust scanned: `src/sim/terrain_spawn.rs`
