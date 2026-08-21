# Phase 3 Radar Base-Terrain Save/Load Reconstruction — Ghidra Research Report

**Address(es):** `Load_Game_From_File @ 0x0067E440`, `Load_Game_Content_From_Stream @ 0x0067E730`, `MouseClass__Load @ 0x005BDF70`, `MouseClass__Save @ 0x005BE6D0`, `FUN_00685120`, `RadarClass__Init @ 0x00655B20`, `RadarClass__ComputeRadarMapBounds @ 0x00654490`, `RadarClass__RebuildRadarSurfaces @ 0x00654650`, `RadarClass__GenerateTerrainSurface @ 0x006547C0`, `RadarClass__FillTerrainColors @ 0x00654EA0`, `CellClass__GetRadarColor @ 0x0047C060`, `RadarClass__RefreshRadar @ 0x00657CE0`, `RadarClass__RenderCellPixel @ 0x00655C50`  
**Investigation mode:** exhaustive-slice  
**Status:** COMPLETE for active-retail in-scenario `.SAV` load reconstruction  
**Confidence:** High for entry ownership, call order, saved-versus-regenerated state, surface lifecycle, playfield bounds, bridge/cell authority, primary composition, error/null behavior, and the current Rust mismatch.  
**Active in YR:** Yes. `0x00559DC5` directly calls the retail OLE/`CONTENTS` load wrapper `0x0067E440`; the wrapper emits `LOADING GAME` and unconditionally reaches the radar reinitialization tail once the content call has been made.

## Target and verdict

Target: how an active Yuri's Revenge save load reconstructs radar base-terrain and final primary pixels from loaded map/cell/overlay/bridge state, and why pixels from the abandoned timeline cannot survive.

Verdict: GameMD does not restore a saved radar bitmap. It clears the scene, loads the Map/Mouse singleton and separately persisted `CellClass` objects, completes object loading and swizzling, discards all radar surface/tracker pointers, normalizes the loaded playfield, regenerates the raw RGB and secondary 16-bit terrain surface from the loaded cells, allocates a cleared primary surface, and finally full-sweeps the primary through current shroud/fog/object composition. A destroyed or repaired bridge from the abandoned timeline has no pixel authority after this sequence. The current Rust load does request a full minimap re-derive, but that re-derive takes overlay identities from the persistent presentation index and structural-bridge presence from static `ResolvedTerrainGrid::bridge_facts`, rather than taking both from the restored live `OverlayGrid`/`BridgeRuntimeState`; that is the exact remaining GSI-04.01 divergence.

## 1. Entry owner and exact call sequence

### 1.1 Content load

`Load_Game_From_File @ 0x0067E440` opens the compound-storage save and `CONTENTS` stream, instantiates the content object, and calls `Load_Game_Content_From_Stream @ 0x0067E730` at `0x0067E659`.

The in-scope order inside `0x0067E730` is:

1. `Clear_Scene @ 0x006851F0` destroys the outgoing object graph and resets the Map/Radar inheritance chain.
2. `ScenarioClass__Load_From_Stream` restores Scenario state.
3. Side/theater resources and the sidebar surface are made available; `Set_View_Dimensions @ 0x004A8960` itself calls `SidebarClass__InitSurface`.
4. At `0x0067E86B..0x0067E870`, receiver `0x0087F7E8` calls `MouseClass__Load @ 0x005BDF70`. That load reads the raw `0x556C` singleton body, recreates dynamic map storage, reads allocated `CellClass` objects through `OleLoadFromStream`, reloads theater tile/terrain/smudge resources, and swizzles its stored pointers.
5. `MapClass__RebuildAllZoneLevels @ 0x00581F50` runs at `0x0067E8CD`, after the Map/Cell load. The remaining OLE class lists load afterward. Thus `0x00581F50` is a zone rebuild, not the Map serializer.
6. Only after all content groups finish can `0x0067E730` return `1`.

### 1.2 Post-content radar tail

Assembly `0x0067E659..0x0067E6BD` proves this exact outer order:

```text
Load_Game_Content_From_Stream @ 0x0067E730
custom content-interface slot +0x10
SwizzleManagerClass__ConvertPendingNodes @ 0x006CF230
FUN_00685120
  -> theater/terrain/smudge resource reloads
  -> RadarClass__Init @ 0x00655B20
       -> ComputeRadarMapBounds @ 0x00654490
       -> RebuildRadarSurfaces @ 0x00654650
FUN_006D03A0 (sidebar/game UI init)
FUN_006D04F0(arg=1) (sidebar toggle/layout)
SidebarClass__InitSurface @ 0x006ABD30
TiberiumClass__InitGrowthQueues_All @ 0x00722D00
TiberiumClass__InitSpreadQueues_All @ 0x00722240
RadarClass__RefreshRadar @ 0x00657CE0
post-load globals/helpers
return 1
```

`SidebarClass__InitSurface` is not the radar surface builder. The radar raw buffer and secondary/primary surfaces already exist after `RadarClass__Init` and before the later sidebar calls.

## 2. Saved state versus regenerated state

| State | Save/load evidence | Post-load authority |
|---|---|---|
| Map/Mouse fixed fields, including `Size`, `LocalSize`, and incidental pointer/header bytes | `MouseClass__Save @ 0x005BE6D0` writes the raw `0x556C` singleton; `MouseClass__Load @ 0x005BDF70` reads it | Loaded fixed fields, followed by normalization/reconstruction |
| Allocated `CellClass` objects | Save counts allocated iterator cells and `OleSaveToStream`s each; Load recreates storage and `OleLoadFromStream`s each | Authoritative loaded cell fields and object lists |
| Overlay/bridge-relevant Cell state | Part of separately persisted cells: tile `+0x38`, overlay id `+0x44`, tile subimage `+0x11A`, overlay data `+0x11E`, flags `+0x140`, shroud `+0x12C`, fog count `+0x13C` | Loaded cell state |
| Primary `+0x121C`, secondary `+0x1220`, raw RGB `+0x123C`, visited bits `+0x1274` | Pointer values occur incidentally in the raw object image; no pointed-to pixel bytes are written | `RadarClass__Init` zeros the pointers; `RebuildRadarSurfaces` regenerates/reallocates all four |
| Object tracker `+0x1258` | Pointer value is not semantic payload | `RadarClass__Init` zeros it, constructs a fresh 256-bucket tracker, and clears each Techno `+0x423` registration byte |
| Terrain dirty vector at embedded object `+0x1224` (`data +0x1228`, `count +0x1234`) | `RadarClass__Init_Clear` clears it before stream load. Its shallow header bytes are later inside the raw `0x556C` read, but `MouseClass__Save` writes no backing-entry payload | Not an authority for load reconstruction; neither Rebuild nor Refresh consumes it |
| Pixel dirty vector at embedded object `+0x125C` (`data +0x1260`, `count +0x126C`) | Same pre-load clear/raw-header caveat; no separately serialized backing-entry payload | Not an authority; Refresh directly visits every primary pixel |
| Theater TMP/overlay resources | Not save pixels; Mouse load and `FUN_00685120` reload theater resources | Current loaded theater/type resources |

The raw-header caveat is real and must not be rewritten as a fictional post-read vector clear: `RadarClass__Init @ 0x00655B20` does not clear the two dirty-vector objects. It does not affect this slice's result because the lists are neither used nor needed by the load reconstruction. The save stream contains no radar-dirty entry array, the terrain surface is rebuilt directly from all loaded cells, and `RefreshRadar` repaints every primary pixel.

## 3. Reset and surface lifecycle

### 3.1 Reset ordering

`Clear_Scene -> FUN_005BDF50 -> SidebarClass__Init_Clear -> PowerClass__Init_Clear -> FUN_00652DE0` reaches the Radar clear stage. `FUN_00652DE0` calls virtual clear slot `+0x0C` on both embedded vectors at `Radar+0x1224` and `Radar+0x125C` before the save stream is read.

After all saved content and swizzles are complete, `RadarClass__Init @ 0x00655B20`:

- writes zero to `+0x121C`, `+0x1220`, `+0x123C`, `+0x1258`, and `+0x1274`;
- allocates/initializes a fresh 256-bucket object tracker at `+0x1258`;
- calls `ComputeRadarMapBounds` and `RebuildRadarSurfaces`;
- sets every loaded Techno `+0x423` byte to zero, requiring ordinary object-driven re-registration.

The outgoing radar allocations may become unreachable when the pointers are zeroed; regardless of allocator cleanup, no old pixel is referenced or copied into the replacement surfaces.

### 3.2 Bounds and allocation

`ComputeRadarMapBounds @ 0x00654490` first calls `MapClass__Set_Clipped_LocalSize @ 0x00567230` on the loaded `MapClass+0xFC` rectangle. That normalizes it against loaded Map `Size`, forces left/top margins to at least `2`, caps width/height, and refreshes Techno playfield membership. It then iterates the allocation-backed Map cell diamond and uses `MapClass__Is_Cell_In_Playfield @ 0x00578460` with mode `1` to compute radar-space min/max fields `+0x1498..+0x14A8`.

`RebuildRadarSurfaces @ 0x00654650` runs only when computed width `+0x14A4` and height `+0x14A8` are both positive. It:

1. zeros radar offsets `+0x149C/+0x14A0`;
2. deletes any current secondary/raw/visited allocations;
3. calls `GenerateTerrainSurface(..., mode=1)`;
4. records the generated size and centers it inside the `140x108` aperture;
5. deletes/replaces primary `+0x121C` with a surface matching secondary dimensions;
6. clears the new primary and regenerates brush shapes.

`GenerateTerrainSurface @ 0x006547C0` allocates the raw three-byte RGB grid `+0x123C`, calls `FillTerrainColors @ 0x00654EA0`, creates/updates the 16-bit secondary terrain surface `+0x1220`, packs current display-format pixels, and allocates the visited bitset `+0x1274`.

For a valid retail save, at least one in-playfield cell produces positive bounds. If an empty/corrupt map produces no positive bounds, Rebuild returns without allocating; the later Refresh has no null-surface guard. This is a valid-save invariant, not an empty-map fallback. Allocation failure is likewise not handled defensively before pointer dereference.

## 4. Loaded cell and bridge authority

`FillTerrainColors @ 0x00654EA0` walks the newly loaded allocation-backed `CellClass` set. For each projected one/two-pixel footprint it calls `CellClass__GetRadarColor @ 0x0047C060` and writes both RGB triples into the new raw buffer. The branch priority relevant here is:

1. loaded TerrainClass occupier (RTTI `0x24`) -> fixed `(200,200,160)`;
2. loaded `Cell+0x140 & 0x100` structural-bridge flag -> fixed `BRIDGE1` type, SHP metadata frame `0`;
3. non-skipped loaded overlay id `Cell+0x44` -> current overlay type; `[0x4A,0x63]` or `[0xCD,0xE6]` forces SHP metadata frame `1`, otherwise frame/data byte `Cell+0x11E`;
4. loaded tile `+0x38`, subimage `+0x11A`, and variant bit `Cell+0x140 & 0x2000` -> current theater TMP metadata, brightness, unsigned `>>1`;
5. missing subimage -> `(60,60,60)`.

The exact overlay skip values are `-1, 100, 101, 231, 232, 239`. Stock `rulesmd.ini [OverlayTypes]` maps the relevant values to destroyed/boundary low-bridge identities including `LOBRDG24`, `LOBRDG25`, `LOBRDB23`, `LOBRDB24`, and `LOBRDGB3`. Those values fall through rather than painting the old deck overlay. The loaded tile/flags then determine the fallen result. A loaded intact structural/deck cell instead takes the structural flag or low-bridge overlay branch.

Therefore both adversarial timeline directions are closed:

- Save intact -> destroy in abandoned timeline -> load: new surfaces read the saved intact Cell/overlay/flag state; destroyed pixels are unreachable.
- Save destroyed -> repair in abandoned timeline -> load: new surfaces read the saved destroyed Cell/tile/skip-overlay state; repaired pixels are unreachable.

No dirty-cell replay is required to obtain either answer.

## 5. Mandatory primary composition

The newly generated secondary is base terrain/terrain-object/bridge/overlay color. `RadarClass__RefreshRadar @ 0x00657CE0` then produces the primary:

- Normal Windows path (`g_hWnd != 0`): query primary width/height and invoke `RenderCellPixel @ 0x00655C50` for every `(x,y)`.
- Null-window path (`g_hWnd == 0`): full secondary-to-primary blit, then `RenderAllCells @ 0x00656150` for tracker dots. This branch does not run per-pixel shroud/fog.

`RenderCellPixel` returns immediately only if `g_PlayerPtr == 0`; otherwise it applies this priority:

1. eligible tracker object/Techno dot (owner/alliance/radar-visibility gates);
2. fogged -> half-bright secondary pixel;
3. shrouded -> packed zero;
4. visible -> exact secondary pixel.

Shroud reads current Cell `+0x12C & 0x08`; fog reads current Cell `+0x13C`. The tracker itself is freshly empty because `RadarClass__Init` replaced it and cleared every Techno registration byte. Thus shroud/fog/base pixels are part of the mandatory synchronous load refresh; loaded Technos are eligible state but object dots repopulate through the ordinary virtual `TechnoClass__RegisterOnRadar @ 0x0070CC90` / `BuildingClass__RegisterOnRadar @ 0x00456580` path after their `+0x423` reset. Radar events and late spy/beacon overlays belong to ordinary `RadarClass__Update`, not this direct load-time `RefreshRadar` call.

## 6. Error, null, pause, replay, and load variants

| Case | Verified behavior |
|---|---|
| Storage/header/second-open/`CONTENTS` failure before content call | Outer wrapper returns `0` at `0x0067E4B1`, `0x0067E53A`, or `0x0067E598`; no radar rebuild tail |
| Inner content load reports failure | Outer wrapper does not test `AL` after `0x0067E659`; it still performs interface fixup, swizzle, full post-load tail, and returns `1`. This is an unsafe retail oddity, not a success guarantee |
| Valid in-scenario load | Full sequence in sections 1–5 |
| Paused/menu invocation | No read of `g_GameRunning @ 0x00A8ED80` occurs in `0x0067E440`, `0x0067E730`, `0x00685120`, `0x00655B20`, `0x00654650`, or `0x00657CE0`; rebuild is synchronous and pause-independent |
| Replay/network resync | `get_xrefs_to(0x0067E440)` has one direct load-game call at `0x00559DC5`; no replay/resync caller reaches this entry. Those are distinct mechanisms and are excluded rather than treated as hidden variants |
| `g_PlayerPtr == 0` | Surfaces still rebuild; normal per-pixel Refresh calls return without painting, leaving the newly cleared primary |
| `g_hWnd == 0` | Full secondary copy plus tracker-only `RenderAllCells`; not the ordinary active retail UI path |
| Zero bounds/OOM | No safe fallback: Rebuild skips zero bounds; later Refresh dereferences surfaces. Allocation failures are also dereferenced. Valid save/content is the contract |

Rust's prepare-then-commit failure behavior is intentionally safer than the native ignored-inner-error oddity: `PreparedLoad::prepare_candidate` performs fallible validation/restoration against owned candidate state and `commit_prepared_load` runs only on success. This report does not recommend emulating native partial-load corruption.

## 7. INI/base-data and asset matrix

There is no INI key that enables, suppresses, or alters the save-load radar rebuild. The active `rulesmd.ini` values `FogOfWar=no`, `ShroudGrow=no`, `ShroudRate=4`, `DestroyableBridges=yes`, and `BridgeStrength=1500` affect ordinary game rules, not the reconstruction trigger/order. `RadarColor=`/`RadarInvisible=` and bridge overlay identities affect `GetRadarColor` inputs. `artmd.ini [BRIDGE] Theater=yes` selects theater art but not persistence.

| Input/art family | Stock mapping | Load-time use | Persistence verdict |
|---|---|---|---|
| `BRIDGE1` | `Image=BRIDGE`; `[BRIDGE] Theater=yes` | Structural `Cell+0x140 & 0x100` uses SHP frame-metadata RGB at frame `0` | Resource reloaded; pixels not saved |
| `LOBRDG*` / `LOBRDB*` | Stock `[OverlayTypes]` and per-type `Image=` rows | Current loaded deck overlay uses own SHP metadata frame `1`; destroyed skip ids fall through | Overlay id/data saved in Cell; art resource reloaded |
| Theater TMP | Selected by loaded tile/subimage/variant and Scenario theater | Raw terrain RGB then brightness/halving | Cell selection saved; TMP pixels/metadata reloaded |
| Radar surfaces | Generated `140x108`-bounded secondary/primary | Reconstructed after content/swizzle | No image asset or pixel payload in save |

No asset rendering was needed: the slice asks which state selects the pixel source after load, not the already documented retail RGB contents of every SHP/TMP entry.

## 8. Current Rust path and exact divergence

Current worktree evidence:

1. `PreparedLoad::prepare_candidate` validates the snapshot, runs `restore_after_snapshot_load`, rebuilds caches, and calls `restore_map_authority_after_snapshot_load` before committing.
2. `restore_map_authority_after_snapshot_load` validates `OverlayGrid`/terrain dimensions, reapplies serialized playfield bounds, recalculates every overlay cell row-major, reconciles low-bridge surface state, rebuilds navigation, and exports the full occupied overlay set.
3. `BridgeRuntimeState` and `OverlayGrid` results are serialized; transient `overlay_projection_ops` is skipped. `Simulation::radar_terrain_dirty_cells` and its generation are also skipped and restore empty/zero.
4. `commit_prepared_load` installs the restored simulation, upserts only occupied overlay identities into the persistent `OverlayRenderIndex`, rebuilds fog, and calls `minimap.mark_stale()`.
5. `mark_stale` sets `installed_playfield_authority=None`, so next `update_minimap` does run a full `reconcile_playfield`; the problem is not failure to request a rebuild.
6. That rebuild calls `build_minimap_overlay_data` over `match_presentation.overlays` and `terrain_objects`. The presentation index preserves cleared-coordinate tombstones; the helper does not consult the restored live `OverlayGrid`.
7. `MinimapPlayfieldProjection::derive` selects structural bridge color through static `ResolvedTerrainGrid::bridge_facts.has_structural_bridge()`, not restored `BridgeRuntimeState`. It also accepts stale presentation overlay entries without validating them against live `OverlayGrid`.
8. Only the later incremental `apply_radar_terrain_dirty_cells` has the correct current-source inputs (`BridgeRuntimeState`, `OverlayGrid`, registry/rules, terrain). The restored dirty list is empty, so it cannot correct the just-derived base.

Result: a full re-derive occurs, but it can reproduce source-map/abandoned presentation bridge pixels instead of the restored live bridge/cell result. The comment in `MinimapRenderer::mark_stale` that pixels are “not re-derived” is imprecise; they are re-derived from the wrong authority.

## 9. Implementation handoff

| Verified behavior | Evidence | Rust delta | Affected surface | Acceptance | Risk / do not do |
|---|---|---|---|---|---|
| Load fully regenerates base terrain from restored current cells; old surfaces/dirty entries are not an authority | `0x005BDF70`, `0x00655B20`, `0x00654650`, `0x006547C0`, `0x00654EA0` | Full projection exists, but uses persistent presentation overlay/static bridge facts | `src/render/minimap.rs::reconcile_playfield`, `src/render/minimap_projection.rs::derive`, `src/app/loading/transitions.rs::build_minimap_overlay_data` | `gsi_04_01_load_intact_bridge_discards_abandoned_destroyed_pixels`: save intact, destroy bridge, load, first minimap frame equals fresh projection of saved intact state with an empty dirty list | Do not preserve `base_terrain_rgba` patches or rely on replaying skipped dirty events |
| Current loaded overlay id/flags/tile decide destroyed versus intact bridge color; skip ids fall through | `MouseClass__Save/Load`, per-cell OLE loop, `0x0047C060`, skip list/ranges | Derive does not read restored `OverlayGrid` and uses static `bridge_facts` | Pass restored `OverlayGrid` and `BridgeRuntimeState` (or one canonical current-cell radar-source adapter) into full projection | `gsi_04_01_load_destroyed_bridge_discards_abandoned_repair_pixels`: save destroyed, repair afterward, load, first minimap frame shows the saved destroyed/fallen cell without any radar dirty entries | Do not infer current bridge visibility solely from immutable map `bridge_facts` or presentation tombstones |
| Loaded playfield is normalized before full surface generation | `0x00654490`, `0x00567230`, `0x00578460` | Rust restores playfield before `mark_stale`/reconcile; retain ordering | `src/sim/snapshot.rs`, `src/render/minimap.rs` | Save after a LocalSize/action-40 change, alter abandoned timeline, load; first radar geometry/bounds and pixels match saved authority | Do not rebuild before `restore_map_authority_after_snapshot_load` completes |
| Full primary composition applies restored fog/shroud over rebuilt base; tracker is reset independently | `0x00655B20`, `0x00657CE0`, `0x00655C50`, `0x00586360`, `0x005864A0` | Rust correctly rebuilds fog and resets tracker, but this must remain ordered after canonical base reconstruction | `src/app/input/dispatch.rs::commit_prepared_load`, `src/render/minimap.rs` | First post-load render uses saved shroud/fog and no outgoing tracker/event entries; later object dots repopulate normally | Do not fold radar events/SpySat animation into base-terrain reconstruction |

Preferred Rust shape: make the full projection and incremental dirty update call the same canonical `current restored cell -> radar source` adapter. Feed it live `OverlayGrid`, live `BridgeRuntimeState`, current terrain-object occupancy, and restored terrain. This removes the authority split without serializing render pixels or dirty queues.

## 10. Coverage ledger

| Required area | Status | Evidence | Remaining |
|---|---|---|---|
| Exact duplicate check | verified negative | research-index brief/search; exact report path absent before this work | none |
| Active load owner/reachability | verified | `0x00559DC5 -> 0x0067E440`; `LOADING GAME`; OLE `CONTENTS` | none |
| Content/map/cell/object order | verified | `0x0067E730`, `0x005BDF70`, call sites `0x0067E86B..0x0067E8CD` | none |
| Save-serialized versus regenerated state | verified | `0x005BE6D0`, `0x005BDF70`, `0x00655B20`, `0x00654650` | raw vector-header oddity explicitly retained |
| Surface existence/order | verified | `0x004A8960`, `0x00685120`, `0x00655B20`, `0x006ABD30`, `0x00657CE0` | none |
| Dirty-list reset/rebuild ordering | verified | `0x006851F0 -> 0x005BDF50 -> 0x00652DE0`; post-load Init/Rebuild/Refresh | none |
| Playfield bounds/zero case | verified | `0x00567230`, `0x00654490`, `0x00654650` | none |
| Bridge overlay/cell input | verified | `0x0047C060`, `0x005FED00`, `rulesmd.ini [OverlayTypes]` | none |
| Abandoned-timeline exclusion | verified | pointer reset + all-cell regeneration + full primary sweep | none |
| Shroud/fog/object layer | verified | `0x00657CE0`, `0x00655C50`, `0x00586360`, `0x005864A0`, fresh tracker | none |
| Empty/null/error cases | verified | Rebuild/Refresh decompile; outer failure exits and ignored inner result | none |
| Pause/replay/load variants | verified/negative | zero pause-global refs in chain; sole direct load xref | replay/resync distinct and excluded |
| INI/base data/assets | verified | primary checkout `ini/rulesmd.ini`, `ini/artmd.ini`, bridge overlay list/type sections | none for load trigger |
| Current Rust parity surface | verified | source paths in section 8 | implementation required |

## 11. Open-question log — final state

- `[RESOLVED] OQ-01` Entry owner? -> active `.SAV` OLE wrapper `0x0067E440`, direct call `0x00559DC5`.
- `[RESOLVED] OQ-02` Exact post-content sequence? -> interface fixup, swizzle, `0x00685120/Radar Init`, sidebar init/toggle/surface, tiberium rebuilds, Refresh.
- `[RESOLVED] OQ-03` Is `0x00581F50` the Map load? -> No, it is `MapClass__RebuildAllZoneLevels`; Map/Cell persistence is `MouseClass__Save/Load`.
- `[RESOLVED] OQ-04` Are radar pixels serialized? -> No. Only incidental pointer bytes occur in the raw body; no pointed surface/RGB bytes are saved.
- `[RESOLVED] OQ-05` Are dirty entries serialized? -> No backing-entry payload; embedded vector header bytes are shallow raw state and are not reconstruction authority.
- `[RESOLVED] OQ-06` When are dirty vectors cleared? -> Before stream load in `FUN_00652DE0`; no invented post-read clear.
- `[RESOLVED] OQ-07` When do radar surfaces exist? -> After `RadarClass__Init/Rebuild` in `FUN_00685120`, before later sidebar/tiberium/Refresh calls.
- `[RESOLVED] OQ-08` What provides bounds? -> loaded Map Size/LocalSize, normalized by `Set_Clipped_LocalSize`, then allocated-cell/playfield iteration.
- `[RESOLVED] OQ-09` Empty/invalid bounds? -> Rebuild no-op; later Refresh is unguarded, so valid nonempty save is an invariant.
- `[RESOLVED] OQ-10` Which state colors a loaded bridge? -> current loaded `Cell+0x140` structural flag or current non-skipped `Cell+0x44` overlay; destroyed skip ids fall through to current tile state.
- `[RESOLVED] OQ-11` Does low-bridge color use `Cell+0x11E`? -> No for forced ranges `[0x4A,0x63]`/`[0xCD,0xE6]`; they force frame `1`. Other overlays use `+0x11E`.
- `[RESOLVED] OQ-12` Can abandoned bridge pixels survive? -> No old surface/raw pointer remains; every new base pixel comes from loaded cells.
- `[RESOLVED] OQ-13` Is shroud part of the mandatory rebuild? -> Yes on normal `g_hWnd!=0` Refresh; shroud writes zero after base regeneration.
- `[RESOLVED] OQ-14` Is fog part of it? -> Yes; fog reads the new secondary and halves channels.
- `[RESOLVED] OQ-15` Are object dots restored as bitmap state? -> No; fresh tracker and cleared Techno registration bytes. Immediate Refresh consults the empty tracker; ordinary registration repopulates it.
- `[RESOLVED] OQ-16` Null player/window behavior? -> null player leaves new primary cleared; null window copies secondary then draws tracker cells without shroud/fog.
- `[RESOLVED] OQ-17` Early load failures? -> three pre-content failure exits return `0` before rebuild.
- `[RESOLVED] OQ-18` Inner load failure? -> outer ignores result and still executes tail/returns `1`.
- `[RESOLVED] OQ-19` Pause variant? -> no pause-global test in the entire load/rebuild/refresh chain.
- `[RESOLVED] OQ-20` Replay/resync variant? -> neither calls this entry; distinct mechanism, excluded.
- `[RESOLVED] OQ-21` Does an INI key control reconstruction? -> verified negative; keys affect color/rules, not save-load rebuild ownership/order.
- `[RESOLVED] OQ-22` Exact Rust divergence? -> full rebuild is triggered, but its bridge/overlay sources are static/presentation rather than restored live cell authority.
- `[RESOLVED] OQ-23` Does `SidebarClass__InitSurface` build radar surfaces? -> No; it lays out/sidebar resources. Radar surfaces are built earlier by `RadarClass__Init`.
- `[RESOLVED] OQ-24` Does immediate load Refresh include radar events/SpySat animation? -> No direct calls; those are late ordinary `RadarClass__Update` layers.

No `OPEN` entries remain.

## 12. Adversarial corner pass

1. **Intact save / destroyed abandoned timeline:** replacement surfaces use intact saved Cell/overlay state; PASS.
2. **Destroyed save / repaired abandoned timeline:** destroyed skip-overlay/tile state is loaded and refilled; PASS.
3. **Restored dirty list empty:** full all-cell regeneration and full primary sweep do not require dirty events; PASS.
4. **Changed saved LocalSize:** normalization/bounds calculation precedes allocation; PASS.
5. **Corrupt/zero bounds, null player/window, OOM:** exact unsafe/null branches are documented; no fabricated fallback; PASS.

## 13. Zero-add pass and cold spot checks

Zero-add pass repeated research-index searches for `radar save load reconstruction`, the exact address cluster, raw Mouse persistence, and abandoned bridge pixels. It found navigation docs and the corrective full `GetRadarColor` inventory, but no second exact load/reconstruction report and no new in-scope question.

Cold spot check A re-decompiled/disassembled `Load_Game_From_File @ 0x0067E440`: call at `0x0067E659`, untested return, exact `0x0067E685..0x0067E6BD` tail, and three early failures all matched this report. Cold spot check B re-decompiled `RadarClass__Init @ 0x00655B20`, `RadarClass__RebuildRadarSurfaces @ 0x00654650`, `RadarClass__RefreshRadar @ 0x00657CE0`, and `CellClass__GetRadarColor @ 0x0047C060`: reset/allocation/full-sweep and bridge-source claims matched. No new questions were added.

## 14. Visual composition ledger

| Order | Function | Condition | Source | Target | Conversion | In load rebuild? |
|---:|---|---|---|---|---|---|
| 1 | `FillTerrainColors @ 0x00654EA0` | each allocated projected cell | current Cell/TerrainClass/bridge/overlay/TMP RGB | raw `+0x123C` | raw RGB triples | Yes |
| 2 | `GenerateTerrainSurface @ 0x006547C0` | positive bounds | raw RGB grid | secondary `+0x1220` | clamp and active DD 16-bit pack | Yes |
| 3 | `RebuildRadarSurfaces @ 0x00654650` | positive bounds | secondary dimensions | cleared primary `+0x121C`, centered in `140x108` | surface construct/clear | Yes |
| 4 | `RefreshRadar @ 0x00657CE0` / `RenderCellPixel` | normal `g_hWnd!=0`, every primary pixel | tracker dot else loaded fog/shroud else secondary | primary `+0x121C` | owner-color pack / half brightness / zero / copy | Yes |
| 5 | ordinary `RadarClass__Update` | later game draw/update | radar events, beacon/SpySat late overlays, viewport/chrome | primary/sidebar | separate incremental pipeline | No; integration exclusion |

## 15. Conflicts and replacement wording

- `SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION_GHIDRA_REPORT.md` navigation that identifies `0x00581F50` as a Map singleton load is wrong. Replace with: `MouseClass__Load @ 0x005BDF70` owns Map/Cell persistence at `0x0067E870`; `0x00581F50` is the subsequent zone-level rebuild.
- `BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT.md` calls RTTI `0x24` a Building occupant. Replace with TerrainClass occupant; BuildingClass RTTI is `6`.
- That bridge report's skip-id names are stale for `231/232/239`. Stock YR `[OverlayTypes]` maps them to `LOBRDB23`, `LOBRDB24`, and `LOBRDGB3`; preserve the exact numeric skip list as authoritative.
- Older radar/Rust reports that say Rust has only a bridge-specific dirty path or only a static cached minimap are superseded for the current worktree: Rust now has generic `apply_radar_terrain_dirty_cells`, native `140x108` geometry/surfaces, and a load-triggered full reconcile. The remaining mismatch is the authority fed to that full reconcile.
- `MinimapRenderer::mark_stale` says abandoned pixels are “not re-derived.” More exact wording: the next frame does re-derive the full base, but derives structural bridge/overlay source from static/presentation state instead of restored live map authority.

## 16. Ghidra annotation candidates — not applied

No metadata was synchronized. Candidates only:

- `FUN_00652DE0` -> `RadarClass__Init_Clear` (High: embedded diagnostic string and exact inherited clear body).
- `FUN_00685120` -> `PostLoad_ScenePresentation_Reinitialize` (Medium: sole caller/load role is clear, but it also handles timers/theater/pathfinder details beyond radar).
- `FUN_00655990` -> `RadarClass__RebuildForPlayfieldChange` (Medium-High: tracker replacement, bounds/surface rebuild, Techno registration reset; caller families include action-40/full-init/RMG).

## 17. Active YR scope and exclusions

Included: active retail in-scenario `.SAV` load, loaded Map/Cell/overlay/bridge state, radar raw/secondary/primary surfaces, playfield bounds, dirty reset relation, shroud/fog/object tracker integration, and first mandatory Refresh.

Excluded except at direct integration points: generic radar transition animation/assets; radar events; beacon/so-called SpySat late animation; ordinary bridge damage/repair simulation; replay/network resync serialization; generic OLE class inventory; exact RGB dump of every TMP/SHP; allocator leak/OOM remediation; emulation of native corrupt-load behavior.

## Sources

### Live Ghidra, read-only, active `gamemd.exe`

- Decompile/disassembly/call/xref evidence: `0x0067E440`, `0x0067E730`, `0x006851F0`, `0x005BDF50`, `0x006A5030`, `0x0063F730`, `0x00652DE0`, `0x005BDF70`, `0x005BE6D0`, `0x00581F50`, `0x00685120`, `0x00655B20`, `0x00654490`, `0x00567230`, `0x00578460`, `0x00654650`, `0x006547C0`, `0x00654EA0`, `0x0047C060`, `0x0047C4D0`, `0x005FED00`, `0x0069E860`, `0x00657CE0`, `0x00655C50`, `0x00656150`, `0x00586360`, `0x005864A0`, `0x0070CC90`, `0x00456580`, `0x006ABD30`, `0x006D03A0`, `0x006D04F0`.
- Assembly contexts: `0x0067E7F4`, `0x0067E86B`, `0x0067E8C8`, `0x0067F7AF`, `0x0067D3AC`; outer tail `0x0067E659..0x0067E6BD`; active caller xref `0x00559DC5`.
- Negative instruction searches: pause global `0x00A8ED80` in the load/rebuild/refresh functions; direct xrefs to `0x0067E440` beyond the load-game caller.

### Prior research read fully as navigation/correction evidence

- `docs/research/RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md`
- `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md`
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_GETRADARCOLOR_FULL_BRANCH_INVENTORY_GHIDRA_REPORT.md`
- `docs/research/MARKTERRAINDIRTY_FULL_CALLER_MATRIX_GHIDRA_REPORT.md`
- `docs/research/MARKTERRAINDIRTY_FUN_00486E70_UPSTREAM_REPAINT_HELPER_GHIDRA_REPORT.md`
- `docs/research/RENDERALLCELLS_MODE_SELECTOR_GHIDRA_REPORT.md`
- `docs/research/RADAR_SURFACE_SIZING_ZOOM_SAMPLING_GHIDRA_REPORT.md`
- `docs/research/TIBERIUMCLASS_QUEUE_SAVE_LOAD_REBUILD_GHIDRA_REPORT.md`
- `docs/research/SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION_GHIDRA_REPORT.md`
- `docs/research/ghidra-workflow.md`

### INI/base data (read-only primary checkout)

- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`: `[General]`/game rules, `[OverlayTypes]`, `[BRIDGE1]`, `LOBRDG*`, `LOBRDB*`, `LOBRDGB*`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`: `[BRIDGE] Theater=yes`.
- `rules.ini`/`art.ini` were also searched for RA2 inheritance/context; no save-load radar rebuild key exists in either generation.

### Current Rust inspected, no edits

- `src/app/persistence/mod.rs`
- `src/app/input/dispatch.rs`
- `src/sim/snapshot.rs`
- `src/sim/world/mod.rs`
- `src/sim/bridge_state/mod.rs`
- `src/sim/overlay_grid.rs`
- `src/app/presentation/overlay_index.rs`
- `src/app/match_runtime/sim_tick.rs`
- `src/app/loading/transitions.rs`
- `src/app/presentation/render/build_instances.rs`
- `src/render/minimap.rs`
- `src/render/minimap_projection.rs`
- `src/render/native_radar_terrain.rs`

No Rust source, Ghidra metadata, or non-report file was modified. No Cargo command was run.
