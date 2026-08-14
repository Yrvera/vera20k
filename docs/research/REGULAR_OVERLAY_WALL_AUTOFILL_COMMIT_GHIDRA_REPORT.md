# Regular Overlay-Wall Autofill Commit — Ghidra Research Report

**Date:** 2026-08-14
**Program:** active Yuri's Revenge `gamemd.exe`, x86 32-bit, image base `0x00400000`
**Investigation mode:** exhaustive-slice
**Scope:** the live stock-YR player path for ordinary `Wall=yes` / `ToOverlay=` walls, from tactical preview and left-click command creation through authoritative placement, automatic gap filling, overlay ownership, connectivity, passability, and production consumption. The exact pending-owner source is closed for local-human placement; nonlocal network-client owner-global choreography is a named residual.
**Non-scope:** Laser Fence Post extension, Firestorm Wall placement, AI base-perimeter planning, and unrelated building-placement predicates except where the ordinary wall path calls them.
**Overall confidence:** **HIGH** for the active stock-YR ordinary-wall algorithm and local-human placement path; **MEDIUM** for reproducing every legacy nonlocal client's pending-owner global sequence.

## 1. Verdict and corrections

Regular wall autofill is an **authoritative simulation-side consequence of one deterministic placement command**. The input event contains one clicked cell. `HouseClass::Place_Production` places the completed wall there, then `FUN_00588750 @ 0x00588750` scans outward and creates any qualifying intermediate segments before the factory consumes the one completed product.

The exact stock rule is:

- scan cardinal directions in **N, E, S, W** order;
- use `BuildingTypeClass+0x5B8`, parsed from `GuardRange=`, as the maximum endpoint distance;
- accept only an endpoint with the same linked overlay type and the same owning house;
- require every intermediate cell to pass the normal building-placement predicate;
- fill from the clicked cell outward, stopping before the endpoint;
- consume **one** completed wall item for the clicked segment and all generated segments;
- make no direct RNG decision or RNG call in the cell-selection/autofill helper.

For the three sidebar-buildable stock walls, `GuardRange=5`, so an endpoint may be one through five cells away and at most four intermediate cells are generated per direction. A wall six cells away is not examined.

This corrects five stale or over-broad claims in earlier reports:

1. `BuildingTypeClass+0xE54` is not a drag-count or span field. `BuildingTypeClass::ReadINI @ 0x00460310`, instructions `0x004611DD..0x0046121F`, resolve `ToOverlay=` and store the resulting `OverlayTypeClass*` at `+0xE54`.
2. `BuildingClass::ExtendWallInDirection @ 0x00452DC0` is not the ordinary player wall autofill owner. It belongs to the building-layer fence/post path. The ordinary `Wall=yes` / `ToOverlay=` autofill owner is `FUN_00588750`.
3. `PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md` collapses the regular-wall preview to a single `PLACE.SHP` frame-1 ghost. The live specialized regular-wall band draws frame 0 once per qualifying gap cell; the outer foundation marker remains a separate validity pass.
4. `0x0043F180` is a broad, multi-mode BuildingClass vtable `+0x124` target, not a wall-only placement routine. This report names only its verified ordinary-wall branch.
5. `WALL_PLACEMENT_AND_PROTECTWITHWALL_GHIDRA_REPORT.md` types the fourth `Cell_passability_building_placement` argument as an integer house index. The live autofill call passes a `HouseClass*`, and the predicate resolves `Cell+0x50` through `g_HouseClass_Array` before comparing that pointer.

## 2. End-to-end active path

| Stage | Live evidence | Result |
|---|---|---|
| Tactical input | `Tactical_Mouse_Message_Handler @ 0x006930A0`, `WM_LBUTTONUP` (`0x202`) | Dispatches the placement release to `DisplayClass::BandBox_LeftUp`. |
| Command construction | `DisplayClass::BandBox_LeftUp @ 0x004AB9B0` | After local preview validation, builds event opcode `0x0B`. The event header identifies the issuing house; the payload carries RTTI, heap id, naval flag, and one cell through `EventClass::BuildEnvelope_3Dwords_Cell`. |
| Deterministic execution | `EventClass::Execute @ 0x004C6CB0`, instructions `0x004C70E1..0x004C7110` | Reads the event cell, resolves the issuing `HouseClass*` from the event header, and dispatches through `HouseClass::Place_Production`; the concrete type comes from that house's completed factory object. |
| Primary placement | `HouseClass::Place_Production @ 0x004FB0E0`, instructions `0x004FB1E0..0x004FB236` | Calls the completed object's virtual `Unlimbo` at the clicked cell center. Failure branches before autofill and does not consume the ready item. |
| Ordinary-wall branch | `0x004FB23C..0x004FB29F` | For RTTI 6 building, skips the Firestorm branch, requires non-null `type+0xE54` and linked `OverlayType+0x2A8 Wall`, then calls `FUN_00588750(cell, house, type)`. |
| Autofill | `FUN_00588750 @ 0x00588750` | Finds and commits all eligible cardinal gaps. |
| Production completion | `0x004FB29F..0x004FB2C7` | Calls `FactoryClass::CompletedProduction @ 0x004CA1A0` only after autofill returns, then clears the placement/production state. |

The click does **not** encode a drag endpoint or a list of filler cells. The visible band is recomputed from current world state. Semantically the request is still one issuer plus one completed item plus one clicked cell; the concrete native event payload does not independently serialize the building type or filler list.

## 3. Key classes, offsets, and globals

| Owner | Offset / address | Verified role in this slice |
|---|---:|---|
| `BuildingTypeClass` | `+0x5B8` | `GuardRange=` in leptons/fixed range. Autofill uses arithmetic shift right by 8 to obtain the integer cell scan bound. |
| `BuildingTypeClass` | `+0x67C` | Speed/land-placement selector passed to `Cell_passability_building_placement`. |
| `BuildingTypeClass` | `+0xE54` | Linked `OverlayTypeClass*` resolved from `ToOverlay=`. |
| `BuildingTypeClass` | `+0x1571` | Ordinary building `Wall=yes` conversion gate in the ordinary-wall branch of BuildingClass vtable `+0x124` target `0x0043F180`. |
| `BuildingTypeClass` | `+0x16BE` | Laser-fence-post routing gate in the placement renderer. Nonzero selects the Laser Fence Post preview sibling. |
| `BuildingTypeClass` | `+0x16C0` | Firestorm-wall routing gate. Nonzero selects the Firestorm preview/commit sibling when `+0x16BE` is zero. |
| `OverlayTypeClass` | `+0x294` | Overlay type's global array index. |
| `OverlayTypeClass` | `+0x2A8` | `Wall` flag used by preview, authoritative autofill, overlay marking, and connectivity. |
| `BuildingClass` | `+0x520` | `BuildingTypeClass*`. |
| `BuildingClass` | `+0x21C` | Owning `HouseClass*` on the conversion path. |
| `HouseClass` | `+0x30` | House array index used for owned-wall identity comparisons. |
| `HouseClass` | `+0x34` | `HouseTypeClass*`. |
| `HouseTypeClass` | `+0xB8` | Country self-index passed to the overlay constructor by wall conversion. It is not the fill range. |
| `CellClass` | `+0x44` | Raw overlay type index (`-1` means none). |
| `CellClass` | `+0x50` | Wall owner house index. |
| `CellClass` | `+0x11B` | Slope byte used by the preview's screen-Y adjustment. |
| `CellClass` | `+0x11C` | Slope index used by placement rejection. |
| `CellClass` | `+0x11E` | Overlay data: low cardinal connectivity nibble plus upper damage state for walls. |
| `CellClass` | `+0x122` | Dynamic blocker-neighbor reference count. Wall placement increments it on the eight surrounding cells; buildings, foot objects, terrain, and landing aircraft also participate in this broader counter. |
| global | `g_DirectionOffsets @ 0x0089F688` | Eight signed `(dx,dy)` pairs. `InitializeDirectionOffsets @ 0x0049F2F0` proves even indices are N, E, S, W. |
| global | `DAT_0088098C` | Pending building object used by the placement renderer. |
| global | `DAT_00880994` | Pending wall owner index. `HouseClass::Begin_Building_Placement @ 0x004FB840` proves the local-human placement write of `House+0x30`; the exact nonlocal event-execution source is not closed. |
| global | `g_PLACE_SHP` | Placement preview shape drawn for each prospective gap cell. |

### 3.1 `GuardRange` parser proof

The ASCII string `GuardRange` is at `0x008444A4`, with its live xref at `0x007122AB` inside `TechnoTypeClass::ReadINI @ 0x00712170`:

```text
007122A4  MOV ECX,[EBP+0x5B8]
007122AA  PUSH ECX
007122AB  PUSH 0x008444A4        ; "GuardRange"
007122B3  CALL 0x00474620        ; CCINIClass::ReadRange
007122B8  MOV [EBP+0x5B8],EAX
```

`ReadRange` stores cell ranges in leptons (`cells * 256`). Both preview and commit use `SAR value, 8`, so fractional values truncate with native signed fixed-point semantics. `Adjacent=` is not read by either autofill routine.

### 3.2 `ToOverlay` parser proof

The ASCII string `ToOverlay` is at `0x0081A740`, with its live xref at `0x004611F5`. `BuildingTypeClass::ReadINI` reads the name, resolves it through the overlay-type lookup at `0x005FEC70`, and stores the `OverlayTypeClass*` at `BuildingTypeClass+0xE54` (`0x0046121F`).

## 4. Exact authoritative autofill algorithm

`FUN_00588750 @ 0x00588750` receives:

- `param_1`: clicked `CellStruct*`;
- `param_2`: placing `HouseClass*`;
- `param_3`: placed `BuildingTypeClass*`.

Equivalent control flow is:

```text
overlay_type = building_type.ToOverlay
if overlay_type is null:
    return

limit = arithmetic_shift_right(building_type.GuardRange, 8)
for direction in [N, E, S, W]:
    gap = []
    probe = clicked + direction
    while len(gap) < limit:
        cell = map.cell_or_dummy(probe)
        if cell.overlay_type == overlay_type.index
           and cell.wall_owner == house.index:
            for gap_cell in gap, nearest-to-click first:
                object = building_type.CreateObject(house)
                object.Unlimbo(center(gap_cell), facing=0)
            break
        if not cell.CellPassabilityForBuilding(
            building_type.SpeedType, building_type, house):
            break
        gap.push(probe)
        probe += direction
```

The raw call at `0x0058885F..0x00588875` passes `param_2`, the `HouseClass*`, as the placement predicate's owner argument. Inside `Cell_passability_building_placement`, an existing `Cell+0x50` owner index is mapped through `g_HouseClass_Array` and compared as a `HouseClass*`; this call is not passing `House+0x30` directly.

### 4.1 Tiny behavioral details

1. The direction loop starts at zero and adds two until eight. With the initialized direction table, that is exactly N, E, S, W.
2. Every direction starts at the cell immediately adjacent to the clicked cell.
3. The endpoint test runs **before** the placement predicate.
4. Endpoint identity requires both `Cell+0x44 == ToOverlay+0x294` and `Cell+0x50 == House+0x30`.
5. Endpoint matching does not inspect `Cell+0x11E`; a damaged same-type, same-owner wall is still an endpoint.
6. An adjacent matching endpoint yields a zero-length gap and creates nothing.
7. The endpoint itself is never recreated or charged as a filler.
8. If a probed non-endpoint cell fails the placement predicate, the whole accumulated gap for that direction is discarded. There is no partial fill before the blocker.
9. Reaching the range limit without finding an endpoint also discards that direction's accumulated gap.
10. An endpoint exactly `limit` cells away is visible; an endpoint `limit + 1` cells away is not probed.
11. A wrong-owner same-type wall is not an endpoint. It then reaches the placement predicate and ordinarily stops the scan as an occupied wall.
12. A different wall type is not an endpoint and ordinarily stops the scan through the same predicate.
13. Out-of-map or missing map-array cells resolve to `g_MapClass_DummyCell`; the placement predicate terminates the direction rather than wrapping coordinates.
14. Once an endpoint is found, filler buildings are created nearest-to-click first and advance outward one cell at a time. The disassembly at `0x0058889D..0x00588933` advances both coordinate halves; the decompiler's reused short temporaries obscure that detail.
15. Directions commit independently. A successful north fill remains even if a later east scan finds a blocker.
16. `BuildingTypeClass` virtual `+0x8C` is `0x0045E880`, proven by `vtable__BuildingTypeClass @ 0x007E4570` and RTTI `.?AVBuildingTypeClass@@`. It allocates `0x720` bytes and constructs a `BuildingClass` for the placing house.
17. `BuildingClass` virtual `+0xD8` is `BuildingClass::Unlimbo @ 0x00440580`, proven by `vtable_BuildingClass @ 0x007E3EBC` and RTTI `.?AVBuildingClass@@`.
18. The helper assumes the object allocation succeeds and dereferences the result. It does not branch on a null object.
19. It ignores each filler `Unlimbo` return value. The cells were prevalidated, but unexpected dynamic failure does not roll back earlier fillers.
20. There is no direct call to `Random::Next` or another RNG routine in `FUN_00588750`; wall cell selection therefore makes no RNG decision. The generic object-construction/`Unlimbo` transitive lifecycle was not exhaustively audited for unrelated RNG side effects, so this report does not claim that broader call graph consumes zero RNG draws.
21. `FactoryClass::CompletedProduction` executes once, after all four directions have been processed. Autofill does not debit cash or consume queue items per generated cell.
22. A null `ToOverlay` pointer or nonpositive truncated `GuardRange` produces no autofill.

## 5. Filler conversion, ownership, and navigation order

The filler is not written directly into a cell by `FUN_00588750`. It uses the same pseudo-building lifecycle as the primary clicked wall.

### 5.1 Building-to-overlay conversion

The broad, multi-mode BuildingClass vtable `+0x124` target at `0x0043F180` checks `BuildingTypeClass+0x1571` in its ordinary-wall branch. In that branch it:

1. allocates `0xB0` bytes for an `OverlayClass`;
2. calls `OverlayClass::Constructor @ 0x005FC380` with `type+0xE54`, the wall cell, and the owning country's `HouseTypeClass+0xB8` index;
3. calls the transient building's virtual `+0x280` with `3`;
4. calls its virtual `+0xF8` cleanup;
5. returns success.

Thus neither the clicked wall nor its generated fillers remains an authoritative `BuildingClass`. The persistent identity is cell-owned overlay state.

If the inner `OverlayClass` allocation fails, this branch still cleans up the pseudo-building and returns success. That exceptional allocation behavior is native and is not a reason to emulate unsafe failure in Rust.

### 5.2 `OverlayClass` is ephemeral

`OverlayClass::Constructor @ 0x005FC380` installs `vtable__OverlayClass`, stores the linked type at `+0xAC`, temporarily exposes its third constructor value through the overlay-placement globals, resolves the cell center, and calls `ObjectClass::Reveal`. The installed virtual `+0x124` dispatches to `OverlayClass::Mark @ 0x005FC570` with mark mode 1.

For `Wall=yes`, `OverlayClass::Mark` performs the following successful path:

1. rejects unsupported steep slope before stamping;
2. runs the cell placement/passability gate;
3. writes `Cell+0x44 = OverlayType+0x294` and `Cell+0x11E = 0`;
4. calls `CellClass::PostDestructionWallCleanup(cell, 1) @ 0x00480630`, which recomputes the N/E/S/W/self cross and preserves the upper damage nibble;
5. outside map-editor mode, calls `MapClass::MergeAdjacentCellZone` and `MapClass::IncrementalRebuildZoneGraphAroundCell` for the placed cell;
6. when the pending-owner sentinel is active, writes `Cell+0x50 = DAT_00880994`; `HouseClass::Begin_Building_Placement` proves that the local-human path writes the placing house's `House+0x30` there;
7. increments the broader dynamic blocker-neighbor reference count at `Cell+0x122` on all eight neighboring cells;
8. calls `CellClass::RecalcAttributes` in the common successful tail;
9. uninits the ephemeral `OverlayClass` object while leaving the cell state persistent.

The important boundary consequence is that overlay identity, connectivity, passability attributes, and incremental zone state are one authoritative mutation sequence; local-human ownership is also closed in that sequence. These effects complete before the next filler and before `FactoryClass::CompletedProduction`. A Rust implementation should take owner explicitly from the deterministic command rather than reproduce the native UI global.

### 5.3 Connectivity is cardinal; neighborhood accounting is eight-way

`CellClass::PostDestructionWallCleanup @ 0x00480630` uses the fixed table at `0x0081CC70` containing `[N, E, S, W, self]`. For each wall visited it recomputes only low-nibble cardinal connection bits:

- N = `0x1`
- E = `0x2`
- S = `0x4`
- W = `0x8`

The separate `Cell+0x122` dynamic-blocker accounting increment touches all eight neighbors and is not wall-specific. These two fan-outs must not be conflated.

## 6. Preview path and visual composition

The preview is a presentation consumer of the same authoritative rule, but native duplicates the scan rather than calling the commit helper:

```text
TacticalClass::Draw @ 0x006D3D10
  -> BuildingPlacement_OverlayRenderer @ 0x006D5030
     -> OverlayWall_PlacementShadow @ 0x006D5C50
```

The outer renderer reaches the ordinary-wall routine only for a pending RTTI-6 building when:

- `BuildingType+0x16BE == 0` (not Laser Fence Post),
- `BuildingType+0x16C0 == 0` (not Firestorm Wall),
- `BuildingType+0xE54 != null`, and
- the linked `OverlayType+0x2A8 Wall` flag is set.

`OverlayWall_PlacementShadow` then uses the same:

- `ToOverlay+0x294` identity,
- pending player's `House+0x30` owner,
- `GuardRange >> 8` bound,
- N/E/S/W order,
- endpoint-before-passability check,
- cell placement predicate, and
- gap-only output.

For each gap cell it computes the cell center, converts to tactical screen coordinates, applies terrain Z/slope adjustment using `Cell+0x11B`, and calls `CC_Draw_Shape` with `g_PLACE_SHP`, frame 0. The exact flag arithmetic at `0x006D5D56..0x006D5D7E` yields `0x00020606` when the boolean is zero and `0x00010606` when it is nonzero. The older claim that this is a bitwise OR with `0xFFFF0000` is not what the live instructions do.

### 6.1 Scoped visual/UI composition ledger

| Element | Coordinate source | Asset / frame | Palette / flags | Occlusion / paint order | Fallback |
|---|---|---|---|---|---|
| Clicked wall foundation marker | Pending building foundation and cursor cell | `PLACE.SHP` through `BuildingPlacement_per_cell_draw` | Normal valid/invalid placement path | Drawn by the outer placement renderer before its specialized ordinary-wall band call | No marker when placement cell is out of bounds. |
| Autofill gap diamonds | Sim-equivalent gap query: clicked cell plus N/E/S/W scans | `PLACE.SHP`, frame 0 | `0x00020606` or `0x00010606`; tactical clipping globals supplied to `CC_Draw_Shape` | Drawn after the main pending foundation pass in `BuildingPlacement_OverlayRenderer` | A blocked direction or absent endpoint draws no gap cells. |
| Committed wall | Persistent `Cell+0x44/+0x11E/+0x50` overlay state | Normal tactical overlay renderer; exact asset/frame/palette is outside this preview slice | Owner/connectivity/damage feed normal overlay presentation | Appears through the regular tactical overlay pipeline after command execution | No persistent `BuildingClass` sprite remains. |

### 6.2 Asset role matrix

| Asset | Role in this slice | Verified source |
|---|---|---|
| `PLACE.SHP` | Per-cell placement feedback for the clicked foundation and prospective autofill gap | String `0x00820080`, loader documented in `PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`, live draw call in `OverlayWall_PlacementShadow`. |

`GAWALL`, `NAWALL`, and `GAFWLL` are verified here as linked overlay **type identifiers**, not as a closed artwork lookup. Their persistent renderer asset/frame/palette selection is deliberately outside this preview-specific slice.

Paint-path closure for the scoped preview is complete: top-level tactical draw -> pending-building overlay renderer -> ordinary-wall band -> `CC_Draw_Shape`. Broader tactical overlay sorting is unchanged and outside this slice.

## 7. Active stock-YR INI proof

YR loads the `rulesmd.ini` / `artmd.ini` pair as its active ruleset; RA2 base files are not fallback layers for missing YR keys. This project authority is recorded in `ENGINE.md:68-69` and was applied when selecting active data for this investigation.

| Building | Active list registration | Active rules evidence | Active art evidence | Native maximum |
|---|---|---|---|---|
| `GAWALL` | `[BuildingTypes]` `rulesmd.ini:1191`; `[OverlayTypes]` `rulesmd.ini:1739` | `rulesmd.ini:12022-12046`: `Wall=yes`, `Adjacent=8`, `Cost=100`, `GuardRange=5` | `artmd.ini:4122-4127`: `Foundation=1x1`, `ToOverlay=GAWALL`, `DamageLevels=3` | Endpoint distance 5, four fillers per direction |
| `NAWALL` | `[BuildingTypes]` `rulesmd.ini:1195`; `[OverlayTypes]` `rulesmd.ini:1763` | `rulesmd.ini:12818-12841`: same relevant values | `artmd.ini:4136-4141`: `ToOverlay=NAWALL`, `DamageLevels=3` | Same |
| `GAFWLL` | `[BuildingTypes]` `rulesmd.ini:1499`; `[OverlayTypes]` `rulesmd.ini:1983` | `rulesmd.ini:13561-13586`: `Wall=yes`, `Adjacent=8`, `Cost=100`, `GuardRange=5`, `TechLevel=2`, `Prerequisite=YABRCK` | `artmd.ini:4129-4134`: `ToOverlay=GAFWLL`, `DamageLevels=3` | Same |

Additional active context:

- `[General] WallBuildSpeedCoefficient=3.0` changes wall build time, not autofill distance.
- `[AI] ConcreteWalls=GAWALL,NAWALL,GAFWLL` names the three stock wall types for AI data.
- `AIBuildsWalls=no` and `NodAIBuildsWalls=no` are the configured stock values. Their exact AI reader/effect is deferred with AI wall planning; neither key is consulted by the independently verified player event path above.
- No separate drag-endpoint, direction-order, or connectivity INI key controls this helper; `GuardRange` alone supplies its scan bound.
- `Adjacent=8` governs normal base-adjacency placement reach. It is not read by either autofill scan.

## 8. Current Rust implementation and verified mismatch

### 8.1 App-owned preview guesses the outcome

`src/app_render/build_instances.rs:650-779` owns `compute_wall_autofill_cells` in the renderer. It:

- scans from the cursor to map edge (`0..511`) instead of `GuardRange`;
- accepts the nearest same overlay type without checking owner;
- checks only app-visible structure occupancy rather than the authoritative placement predicate;
- reads `state.overlays`, the app's render list, rather than only the authoritative live `OverlayGrid`;
- returns presentation cells that are never part of the command.

For a stock wall, this can preview fillers toward a wall more than five cells away or toward an enemy wall. Native does neither.

### 8.2 Sim commits only the clicked cell

`src/sim/production/production_placement.rs:219-275` correctly revalidates `PlaceReadyBuilding`, recognizes a `Wall=yes` type, and stamps an owned overlay, but it calls `OverlayGrid::place_owned_wall` only for `(rx, ry)`. There is no authoritative native autofill.

The command shape at `src/sim/command.rs:530-535` and its app producer at `src/app_commands.rs:294-337` are already conceptually correct: one owner/type/cell command. Filler cells should not be copied into the command from the app.

### 8.3 Overlay state is sim-owned, but navigation completion is app-owned

`OverlayGrid` and wall ownership are authoritative sim state (`src/sim/overlay_grid.rs:116-137`, `422-438`). Connectivity is also recalculated in sim (`909-925`). However, the grid exposes runtime dirty cells for the app to drain, and `src/app_sim_tick.rs:1682-1748` then:

1. mutably borrows `sim.overlay_grid` and `sim.resolved_terrain` from the app;
2. calls `recalc_overlay_passability` in the app layer;
3. decides whether to rebuild the app-owned dynamic path grid and sim zones;
4. synchronizes a separate render-overlay list.

That leaves authoritative overlay identity and navigation authority split across the sim/app boundary, and the work occurs after `advance_master_frame` has produced its state hash. Native completes the overlay's attribute and zone effects in the placement mutation sequence.

### 8.4 Existing reusable pieces

- `RuleObject.guard_range` is already parsed as deterministic `SimFixed` in `src/rules/object_type.rs`.
- `OverlayGrid` already stores `wall_owner`, hashes/serializes persistent cells, and has exact cardinal connectivity helpers.
- `Cell_passability_building_placement` behavior has a verified Rust-facing research report and existing placement evaluation surfaces.
- The app already queues a single deterministic `PlaceReadyBuilding` command.

## 9. Coverage ledger

| Required stage | Evidence reached | Status | Confidence |
|---|---|---|---|
| Player input | `Tactical_Mouse_Message_Handler` LMB release | CLOSED | High |
| Local validation | `DisplayClass::BandBox_LeftUp` placement branch | CLOSED | High |
| Command payload | event opcode `0x0B`; issuer in header; RTTI/heap id/naval flag/one cell in payload | CLOSED | High |
| Deterministic dispatch | `EventClass::Execute -> HouseClass::Place_Production` | CLOSED | High |
| Primary object placement | virtual `BuildingClass::Unlimbo` | CLOSED | High |
| Ordinary-wall routing | linked `ToOverlay`, `Wall`, non-Firestorm branch | CLOSED | High |
| Range source | `GuardRange` string/parser/store at `+0x5B8` | CLOSED | High |
| Exact fill scan | full decompile plus full disassembly of `0x00588750` | CLOSED | High |
| Owner/type endpoint | `Cell+0x44/+0x50` versus overlay/house indices | CLOSED | High |
| Gap legality | live `Cell_passability_building_placement @ 0x0047C620` | CLOSED | High |
| Filler instantiation | BuildingType vtable `+0x8C -> 0x0045E880` | CLOSED | High |
| Filler commit | BuildingClass vtable `+0xD8 -> 0x00440580` | CLOSED | High |
| Persistent representation | BuildingClass vtable `+0x124` wall branch -> `OverlayClass::Constructor/Mark` | CLOSED | High |
| Local-human owner source | `Begin_Building_Placement -> DAT_00880994 -> OverlayClass::Mark` | CLOSED | High |
| Nonlocal client owner-global source | direct-write census plus event execution path | DEFERRED | Medium; no event-local write was found |
| Connectivity/passability/zones | `OverlayClass::Mark` and `CellClass::PostDestructionWallCleanup` | CLOSED | High |
| Production consumption | one `FactoryClass::CompletedProduction` after helper | CLOSED | High |
| Preview producer | `OverlayWall_PlacementShadow`, full decompile/disassembly | CLOSED | High |
| Draw sink | `CC_Draw_Shape(g_PLACE_SHP, frame 0, ...)` | CLOSED | High |
| Stock activation | all three active wall sections in `rulesmd.ini` / `artmd.ini` | CLOSED | High |
| Rust producer/consumer delta | app preview, command, sim placement, overlay grid, app dirty drain | CLOSED | High |

No local-human scope-critical stage remains inferred from a name alone. The nonlocal pending-owner global is explicitly deferred rather than silently generalized from the local path.

## 10. Adversarial corner cases

| Scenario | Native result | Why |
|---|---|---|
| Same-owner/same-type wall directly north | Clicked wall commits; no north filler | Endpoint is detected with gap count zero. |
| Same-owner/same-type wall five cells east | Four east fillers commit nearest-to-click first | Distance five is still probed for stock `GuardRange=5`. |
| Same-owner/same-type wall six cells east | No east fillers | The fifth non-endpoint reaches the limit; distance six is never probed. |
| Legal cells followed by a blocking structure before the endpoint | No fillers in that direction | The gap is only committed after a matching endpoint; a failed predicate discards it. |
| Same-type wall owned by another house | No connection through it and ordinarily no farther scan | Owner test fails, then occupied-wall placement fails. |
| Damaged same-type, same-owner endpoint | It terminates the scan as an endpoint | Endpoint test ignores overlay data/damage. |
| Valid endpoints north and west, blocker east | North then west fillers remain; east contributes none | Directions are independent and execute N, E, S, W. |
| `GuardRange=0` or fractional value below one cell | No autofill | Arithmetic shift/truncation yields a nonpositive limit. |
| No `ToOverlay` link | Primary ordinary autofill helper returns immediately | Null linked pointer guard. |
| Filler `Unlimbo` unexpectedly fails after earlier fillers | Earlier fillers remain; helper continues | Return value is ignored and there is no rollback transaction. |

## 11. Open Questions Log — final

All local-human scope-critical questions are resolved. The nonlocal owner-global question and explicitly excluded siblings are deferred rather than left open.

| ID | Question | Final state | Resolution / reason |
|---|---|---|---|
| Q1 | What is the player input -> command -> commit path? | RESOLVED | LMB release -> `BandBox_LeftUp` -> event `0x0B` -> `EventClass::Execute` -> `HouseClass::Place_Production`. |
| Q2 | Does the command contain a drag endpoint or fill cells? | RESOLVED | No. The event header identifies the issuer and its payload contains RTTI, heap id, naval flag, and one clicked cell; concrete type comes from the completed factory object. |
| Q3 | Which function owns ordinary autofill? | RESOLVED | `FUN_00588750`, not `BuildingClass::ExtendWallInDirection`. |
| Q4 | What controls maximum distance? | RESOLVED | `GuardRange` at `BuildingType+0x5B8`, shifted right eight. |
| Q5 | What are direction order and coordinate deltas? | RESOLVED | Even direction indices: N, E, S, W. |
| Q6 | What identifies an endpoint? | RESOLVED | Same linked overlay index and same `House+0x30` owner. |
| Q7 | Do damaged endpoints count? | RESOLVED | Yes; endpoint check does not read overlay data. |
| Q8 | What happens at blockers or range exhaustion? | RESOLVED | No partial fill for that direction. |
| Q9 | Is the four-direction operation atomic? | RESOLVED | No; directions commit sequentially and independently. |
| Q10 | How are fillers represented? | RESOLVED | Transient `BuildingClass` -> ephemeral `OverlayClass` -> persistent cell overlay. |
| Q11 | When do passability/connectivity/zones update? | RESOLVED | During each overlay Mark before the next filler and before factory completion. |
| Q12 | How many products/cash debits occur? | RESOLVED | One completed product is consumed after the full helper; no per-filler debit. |
| Q13 | Does wall cell selection use RNG? | RESOLVED | No direct RNG decision/call in `FUN_00588750`. Generic constructor/`Unlimbo` transitive RNG cadence was not exhausted and is not claimed. |
| Q14 | Does preview use the same rule? | RESOLVED | Yes, via a duplicated read-only scan and the same core fields/predicate. |
| Q15 | Is preview a drag gesture? | RESOLVED | No authoritative drag endpoint exists; it is an automatic band around the one pending cell. |
| Q16 | How do off-map cells behave? | RESOLVED | They resolve to the dummy cell and terminate through placement legality. |
| Q17 | Which stock walls activate the rule? | RESOLVED | `GAWALL`, `NAWALL`, and `GAFWLL` are registered in both active type lists and all use `GuardRange=5`. |
| Q18 | Does `Adjacent=8` set the fill span? | RESOLVED | No. Neither preview nor commit reads it. |
| Q19 | Does the Rust result require extra save/replay state? | RESOLVED | No filler list is needed. It is recomputed from the explicit command owner, completed type, clicked cell, and current authoritative cells. |
| Q20 | What exact global plumbing supplies wall owner on every nonlocal network-client execution path? | DEFERRED | A direct-write census of `DAT_00880994` closed the local `Begin_Building_Placement` source but found no event-local issuer write before `OverlayClass::Mark`. This is a real native-global uncertainty, not a presentation-only detail. Rust must avoid it by carrying the owner explicitly in `PlaceReadyBuilding` and stamping that owner on every peer. |
| Q21 | How does stock AI choose wall perimeter cells? | DEFERRED | AI wall planning is excluded. The stock `AIBuildsWalls` keys are configured `no`; their exact AI reader/effect is deferred. |
| Q22 | What are Laser Fence Post and Firestorm fill rules? | DEFERRED | Explicit non-scope siblings with separate gates and helpers. |

## 12. Zero-add and cold verification passes

The zero-add pass revisited the primary helper, both upstream callers, the conversion sink, the preview sibling, and every direct write reference found for `DAT_00880994`. It added no new local-human scope-critical question, but it retained the nonlocal owner-global residual because no event-local issuer write was found.

Two cold spot checks were then performed from raw instructions rather than relying on the first decompile:

1. `FUN_00588750 @ 0x00588750` full disassembly reconfirmed `SAR [type+0x5B8],8`, direction `+2`, endpoint owner/type comparisons, predicate-before-count increment, BuildingType vtable `+0x8C`, BuildingClass vtable `+0xD8`, ignored `Unlimbo` return, and both-half coordinate stepping.
2. `HouseClass::Place_Production @ 0x004FB0E0`, instructions `0x004FB23C..0x004FB2C7`, reconfirmed that `FUN_00588750` executes after successful primary `Unlimbo` and before the single `FactoryClass::CompletedProduction` call.

The live `GuardRange` parser sequence and `OverlayWall_PlacementShadow` flag arithmetic were also re-read from disassembly to correct stale prose.

## 13. Implementation handoff

### 13.1 Required behavior deltas

| Verified native behavior | Binary / data evidence | Current Rust delta | Required Rust effect | Acceptance scenario | Risk |
|---|---|---|---|---|---|
| One placement command recomputes autofill authoritatively | event `0x0B`; `Place_Production -> 0x00588750` | App previews guesses; sim places one cell | Keep one-cell command with explicit owner/type/cell; sim recomputes and commits fill from current state | Two identical sims fed one command produce identical primary/filler cells and hashes | High if left app-owned: replays/network can diverge from preview-only logic |
| Fill bound is `GuardRange >> 8` | parser `0x007122A4..B8`; helper `0x00588777..80` | App scans to map edge | Read parsed deterministic `guard_range`, truncate to integer cells | Stock endpoint at distance 5 fills; distance 6 does not | Player-visible on routine wall placement |
| Endpoint requires same type and owner | helper `0x005887F5..0x00588836`; local owner source closed, nonlocal native global deferred | App ignores owner | Query `OverlayGrid` identity and `wall_owner`; use the command's explicit owner for commit on every peer | Enemy wall never attracts a filler band | Frequent in multiplayer/AI proximity |
| Every gap cell uses authoritative placement legality | helper call `0x0058885F..75` | App checks only structure occupancy | Use one sim-owned wall-gap query shared by preview and command execution | Terrain/object blocker cancels that entire direction | High pathing/placement correctness risk |
| Direction order is N,E,S,W and each direction is independent | helper `0x0058876B..0x00588943` | App order happens to match but is non-authoritative | Preserve order in sim result/event and commit sequence | Multi-endpoint test asserts exact event cell order | Determinism-sensitive |
| One ready item covers all generated cells | `0x004FB29F..0x004FB2A6` | Sim consumes after one cell | Commit primary + fills, then consume once | Queue count decreases by one with four fillers | Economy-visible |
| Overlay identity, owner, connectivity, passability, zones complete before hash | `OverlayClass::Mark`, `PostDestructionWallCleanup` | App drains/mutates navigation after sim hash | Move overlay terrain/path/zone consequences fully inside sim frame before hash | A unit/path query in the same committed frame sees the new wall | Determinism and one-frame pathing risk |
| Preview mirrors current authority but does not prescribe it | `OverlayWall_PlacementShadow` | Renderer owns algorithm over a separate list | App requests a read-only `WallPlacementPreview` from sim and renders returned cells | Preview cell set equals explicit placement-result event when state is unchanged | Player trust / visual mismatch |

### 13.2 Recommended API shape

The implementation should preserve this authority direction:

```text
App cursor + selected type
  -> Simulation::wall_placement_preview(owner, type, clicked_cell) [read-only]
  -> App renders returned primary/gap legality

App click
  -> Command::PlaceReadyBuilding { owner, type, clicked_cell }
  -> Simulation revalidates and recomputes the same query against execution-time state
  -> Simulation commits primary + N/E/S/W fillers
  -> Simulation updates overlay terrain/navigation authority before hashing
  -> SimFrameOutput::PlacementCommitted { primary, autofill_cells, ... }
  -> App consumes event and read-only overlay deltas for UI/render/audio
```

Do **not** put app-computed filler cells into the command. Between preview and execution, another deterministic command can change an endpoint or blocker. Native resolves the band at execution time.

### 13.3 Focused acceptance tests

1. `GuardRange=5`, same owner/type endpoint five cells north: primary plus four fillers, ordered nearest-to-click, one ready item consumed.
2. Endpoint six cells away: primary only.
3. Same type but different owner within range: primary only in that direction.
4. Blocker after two legal cells and before a matching endpoint: no partial fillers in that direction.
5. Matching endpoints in N/E/S/W: assert direction and within-direction event order.
6. Damaged same-owner/type endpoint: still closes the gap.
7. Preview query and command result match when world state is unchanged.
8. Preview changes but command safely recomputes when an endpoint/blocker changes before execution.
9. New wall passability and zone/path authority are visible before state hash and before the next sim reader.
10. Save/load and lockstep replay reproduce the same owned overlay cells and connectivity bytes.

## 14. Ghidra annotation candidates

No Ghidra metadata was changed in this investigation.

| Address | Current label | Proposed label | Confidence | Rationale |
|---:|---|---|---|---|
| `0x00588750` | `FUN_00588750` | `AutofillRegularOverlayWalls` | High | Called after successful ordinary wall placement with plain `(cell, house, building_type)` arguments; no receiver convention is proven, so a `HouseClass__` prefix would overstate ownership. |

`0x00588570` remains intentionally unnamed here because it is the excluded Firestorm sibling.

## 15. Sources

### Live binary functions

- `Tactical_Mouse_Message_Handler @ 0x006930A0`
- `DisplayClass::BandBox_LeftUp @ 0x004AB9B0`
- `EventClass::Execute @ 0x004C6CB0`
- `HouseClass::Place_Production @ 0x004FB0E0`
- `HouseClass::Begin_Building_Placement @ 0x004FB840`
- `FUN_00588750 @ 0x00588750`
- `BuildingTypeClass::ReadINI @ 0x00460310`
- `TechnoTypeClass::ReadINI @ 0x00712170`
- `CCINIClass::ReadRange @ 0x00474620`
- `Cell_passability_building_placement @ 0x0047C620`
- `BuildingClass::Unlimbo @ 0x00440580`
- BuildingClass multi-mode vtable `+0x124` target `0x0043F180` (ordinary-wall branch)
- `OverlayClass::Constructor @ 0x005FC380`
- `OverlayClass::Mark @ 0x005FC570`
- `CellClass::PostDestructionWallCleanup @ 0x00480630`
- `BuildingPlacement_OverlayRenderer @ 0x006D5030`
- `OverlayWall_PlacementShadow @ 0x006D5C50`
- `InitializeDirectionOffsets @ 0x0049F2F0`

### Active data

- `ENGINE.md` (YR standalone `*md` loading authority)
- `ini/rulesmd.ini`
- `ini/artmd.ini`

### Prior research consulted and corrected where noted

- `docs/research/OVERLAYWALL_PLACEMENTSHADOW_AND_HEIGHTADJUST_GHIDRA_REPORT.md`
- `docs/research/WALL_PLACEMENT_AND_PROTECTWITHWALL_GHIDRA_REPORT.md`
- `docs/research/WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md`
- `docs/research/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`
- `docs/research/CELL_0X122_DYNAMIC_BLOCKER_LIFECYCLE_RUST_MAPPING_GHIDRA_REPORT.md`
- `docs/research/pathfinding/CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`
- `docs/research/BUILD_QUEUE_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`
- `docs/research/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`

### Current Rust surfaces audited

- `src/app_render/build_instances.rs`
- `src/app_commands.rs`
- `src/app_sim_tick.rs`
- `src/rules/object_type.rs`
- `src/sim/command.rs`
- `src/sim/production/production_placement.rs`
- `src/sim/overlay_grid.rs`
