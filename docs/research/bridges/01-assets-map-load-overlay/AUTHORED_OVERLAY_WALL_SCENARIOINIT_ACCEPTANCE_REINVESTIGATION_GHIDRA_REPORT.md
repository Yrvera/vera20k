# Authored Overlay Wall ScenarioInit Acceptance and Success-State Reinvestigation

Date: 2026-08-31
Status: **COMPLETE for the bounded authored-load wall acceptance and success-state slice**
System rows: GSI-04.12 lifecycle/order; GSI-04.13 authored OverlayPack finalization; GSI-04.15 negative Tube separation

Activity vocabulary is literal. **Active in YR: Yes** means the behavior is reached by shipped Yuri's Revenge data. **Conditional** means active executable code and retail-declared data can reach it with compatible custom/editor input. **No** means the proposed authored behavior is disproved. OpenTS is correspondence only and never parity evidence.

## Target question

Resolve the contradiction between the authored OverlayPack reports about `Wall=yes` rows: whether `OverlayClass::Mark @ 0x005FC570` can reject an authored wall through `CellClass::Is_Clear_To_Build @ 0x0047C620`, the exact predicate arguments and `ScenarioInit` lifetime, every material wall-success effect before the later OverlayData pass, the state that must survive finalization, and the boundary between authored loading and ordinary runtime wall construction.

## Non-goals

- Reimplementing Rust, changing Ghidra metadata, or applying the corrections identified here.
- Re-auditing every counter-zero caller of `Is_Clear_To_Build`; the already-audited predicate body remains authority for ordinary placement.
- Runtime wall damage, chain damage, production autofill, savegame serialization, or map-editor UI behavior except where they prove that the generic rejection branch remains live outside authored `Full_Init`.
- Treating OpenTS source as YR evidence. Its `ScenarioInit` guard and wall branch were used only as navigation leads and were independently verified in active `gamemd.exe`.

## Completion bar

COMPLETE required all of the following: the exact `Full_Init` counter lifetime through `ReadMapOverlayPacks`; the exact wall predicate arguments and return branch; the wall-success write/cleanup/owner/count order; cardinal connectivity and eight-way count lookup semantics; the later OverlayData interaction; proof of any non-reconstructible retained state; current Rust ownership; active retail type/art bindings and exhaustive installed-map activation; at least five adversarial cases; two cold native rechecks; and a zero-additional-mechanism pass. All behavior questions in this bounded fresh authored-load slice are resolved. Savegame serialization of the process-global dummy counter is excluded because this transaction starts from map resize and no fresh-load movement, path, or placement consumer can read that dummy byte.

## Primary evidence ledger

- **Active in YR: Yes.** Binary authority is active `gamemd.exe`, SHA-256 `1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`, image base `0x00400000`. Ghidra work was read-only.
- **Active in YR: Yes.** Load chain and counter: `ScenarioClass::Full_Init 0x00686B20`, `ReadMapOverlayPacks 0x005FD2E0`, `OverlayClass::Constructor 0x005FC380`, `ObjectClass::Unlimbo 0x005F4EC0`, and `OverlayClass::Mark 0x005FC570`.
- **Active in YR: Yes.** Predicate and success path: `CellClass::Is_Clear_To_Build 0x0047C620`, `CellClass::PostDestructionWallCleanup 0x00480630`, `CellClass::IsWallConnectableInDirection 0x00480510`, `CellClass::Adjacent_Cell 0x00481810`, `MapClass::Get_CellClass 0x005657A0`, `CellClass::RecalcAttributes 0x0047D2B0`, and `AStar_main_loop 0x00429A90`.
- **Active in YR: Conditional.** Concrete low-body authority is table `0x00833438 = [0xD6,0xCD,0xE3,0xDF]` plus direct writer `0x005FCF72..0x005FCF9E`. Compact source-order registry insertion makes `0xCD..0xD0 = LOBRDB01..04`; the base-`0xCD` raw variants therefore remain non-wall bridge bodies. This writer can erase an earlier wall without reversing its counts, but it does not create the hypothesized zero-count `CAFNCB`/`CAFNCW` wall.
- **Active in YR: Yes.** Retail authority is extracted `rulesmd.ini`, SHA-256 `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF`, and `artmd.ini`, SHA-256 `E1F0378394313C04EBBD5073F47785EE3E46F1B3C62D65724E8F3C310EE7BA31`.
- **Active in YR: Yes.** The retail census enumerated the 187 distinct logical map IDs supplied by `multimd.mix` and `mapsmd03.mix`, resolved every ID through active startup priority, and decoded every winning payload to a complete 262,144-byte OverlayPack. `expandmd01.mix` wins 13 of those names; 12 winners are byte-identical to their lower-priority copy. The differing `all02umd.map` winner is 465,788 bytes rather than 465,825, but both versions decode the identical OverlayPack SHA-256 `54F3990CBEA2AA4487E2E4F524FF6B70D578ECD95A712679DD45454BD09BD727`, so the occurrence totals are unchanged. The three entries lacking recoverable logical names were classified by hash and decoded too; no active-retail corpus residual remains.
- **Active in YR: Yes.** One end-to-end shipped activation witness is `xdustbowl.map` (`multimd.mix` hash `0xE4BFD9FD`), cell `(96,84)`: OverlayPack ID `0 GASAND`, IsoMap tile `493`/subtile `0`, level `1`; active TEMPERATE maps that tile through `temperatmd.ini [TileSet0041]` to retail `Green01.tem`, whose only active subtile reports `ramp_type=0`. The initiating `Cell+0x11C` is therefore exactly `0`, so this stock wall reaches the predicate and success path.
- **Lead only.** `C:\Users\enok\Documents\OpenTS\code\cell.cpp:709..714` has `Is_Clear_To_Build` return true under `ScenarioInit`; `code\overlay.cpp:208..228` has the corresponding wall-success skeleton. Every used conclusion below comes from YR binary/data evidence instead.

## Relevant layout and globals

| Owner | Field / address | Verified meaning in this slice |
|---|---:|---|
| `CellClass` | `+0x24` | packed signed-i16 cell coordinate |
| `CellClass` | `+0x44` | signed overlay runtime identity; `-1` means none |
| `CellClass` | `+0x4C` | zone state compared around wall cleanup Recalc |
| `CellClass` | `+0x50` | wall owner index/sentinel |
| `CellClass` | `+0x11C` | slope index; `>4` is the pre-wall universal rejection gate |
| `CellClass` | `+0x11E` | full overlay state byte: wall damage upper nibble, connectivity lower nibble |
| `CellClass` | `+0x122` | wrapping adjacent-object/blocker source count consumed by native A* |
| `OverlayClass` | `+0xAC` | linked `OverlayTypeClass*` |
| `OverlayTypeClass` | `+0x294` | dense zero-based runtime overlay identity |
| `OverlayTypeClass` | `+0x2A8` | `Wall=` byte; constructor default false at `0x005FE296` and INI write at `0x005FE7CB..E5` |
| global | `0x00A8E7AC` | `ScenarioInit` / load-suppression nesting counter, not map-editor mode |
| global | `0x00A8ED6B` | separate map-editor flag read later inside the counter-zero predicate body |
| global | `0x00A8E9A0` | separate game-active flag |
| global | `0x008333BC` | pending wall-owner constructor value; `-1` means no owner write |
| data | `0x0081CC70` | wall cleanup table `[0,2,4,6,-1] = [N,E,S,W,self]` |

The current Ghidra symbol on `0x00A8E7AC` says `g_MapEditorMode`. That label is stale. The binary independently reads `0x00A8ED6B` for map-editor behavior, while `0x00A8E7AC` is incremented/decremented as a nesting counter around scenario initialization and is the first guard in `Is_Clear_To_Build`.

## Exact authored counter lifetime

**Active in YR: Yes.** A successful `Full_Init` that reaches authored overlays necessarily keeps `ScenarioInit` nonzero:

1. `0x00686B35..0x00686B4F` reads `0x00A8E7AC`, increments it, and writes it back before map initialization.
2. `0x006878CC` branches successful initialization to `0x00687924`. The decrement at `0x006878CE..0x006878E6` is only the failure epilogue and is skipped on that success path.
3. `0x00687A34` calls `ReadMapOverlayPacks` while the incremented value is still installed.
4. Only later, at `0x00687C2B..0x00687C44`, `Full_Init` saves the current value, temporarily writes zero around `0x00452D40`, and restores `saved-1`.

Therefore an authored wall cannot enter the ordinary body of `Is_Clear_To_Build`. This is true for a normal top-level `0 -> 1` load and for a nested nonzero load depth.

## Exact wall predicate call and authored verdict

**Active in YR: Yes.** After base Mark succeeds and before any wall placement test, `OverlayClass::Mark` applies the universal slope gate at `0x005FC5CD..0x005FC5E3`: `Cell+0x11C > 4` rejects unless the overlay ID is the unrelated special ID `0xB2`. None of the retail wall IDs is `0xB2`.

For `OverlayType+0x2A8 != 0`, the wall call corridor is exact:

```text
0x005FC6F4 PUSH 0       ; HouseClass* who = null
0x005FC6F5 PUSH 0       ; BuildingTypeClass* what = null
0x005FC6F6 PUSH 1       ; SpeedType::Track
0x005FC6F8 MOV  ECX,CellClass*
0x005FC6FA CALL 0x0047C620
0x005FC703 TEST AL,AL
0x005FC705 JZ   0x005FC77C
```

`Is_Clear_To_Build` begins at `0x0047C620` by loading `0x00A8E7AC`; `0x0047C62E..0x0047C632` jumps directly to the success epilogue when it is nonzero; `0x0047CA70..0x0047CA79` returns `AL=1` and pops the three arguments.

**Active in YR: No.** The `0x005FC705 -> 0x005FC77C` UnInit rejection branch is unreachable for authored OverlayPack walls inside successful `Full_Init`. Occupiers, prior overlay identity/state, `Cell+0x124`, `Cell+0x140`, land/slope fallback inside the predicate, owner, game-active state, and map-editor state are not inspected. Only earlier reader admission/base-Mark/allocation gates and the universal slope gate can prevent this wall path.

**Active in YR: Yes, outside this authored path.** With `ScenarioInit==0`, the full predicate remains live and may return false. Its caller set includes `BuildingClass::MarkCellListsAndPlacement 0x0043F180`, placement/shadow routines, `OverlayClass::Mark`, and `TechnoTypeClass::CanPlaceAt`. Ordinary wall conversion constructs the same ephemeral `OverlayClass` at `0x0043F62E`. The generic runtime wall rejection/lifecycle mechanism must therefore remain; it must not be presented as authored `ReadMapOverlayPacks` behavior.

## Exact authored wall-success transaction

For every reader-admitted, allocated, slope-accepted packed `Wall=yes` row whose own ephemeral object reaches Mark, the material sequence is:

1. **Stamp anchor.** `0x005FC707..0x005FC721` writes `Cell+0x11E=0`, writes `OverlayType+0x294` to `Cell+0x44`, and calls `PostDestructionWallCleanup(cell,1)`.
2. **Refresh cardinal cross.** The helper walks `N,E,S,W,self` in that exact order from `0x0081CC70 = 00000000 02000000 04000000 06000000 FFFFFFFF`. Each visited wall recomputes lower-nibble connectivity by probing `N,E,S,W`, preserving its upper nibble, then calls `RecalcAttributes(-1)` and performs its zone-change branch. Tactical/radar dirty work is performed for each visit.
3. **Use authored connectivity boundary.** Before Terrain/Building/Techno construction, wall-to-building gate/tower connections in `IsWallConnectableInDirection @ 0x00480510` have no live `BuildingClass` object to match. Same non-`-1` overlay identity connects immediately at `0x0048051A..0x0048052C`; different wall types do not connect merely because both have `Wall=yes`.
4. **Skip only Mark's later explicit zone block.** `0x005FC726..0x005FC747` skips the separate `MergeAdjacentCellZone`/incremental-rebuild pair because `ScenarioInit!=0`. It does not suppress the cleanup helper's own Recalc/zone work.
5. **Leave owner unset.** Reader assembly pushes constructor argument `-1` at `0x005FD4C3`. The constructor publishes that value to `0x008333BC` at `0x005FC42F..0x005FC43B` while Unlimbo/Mark runs, then restores `-1`. Consequently `0x005FC747..0x005FC758` performs no `Cell+0x50` owner write.
6. **Increment all eight adjacent counts.** `0x005FC758..0x005FC775` calls `CellClass::Adjacent_Cell(i)` for direction indices `0..7`, reads each target byte at `+0x122`, increments it with raw `u8` wrap, and writes it back. Startup `0x0049F2F0..0x0049F39B` constructs the order `N,NE,E,SE,S,SW,W,NW`.
7. **Run common tail.** `0x005FD1FA..0x005FD227` calls anchor `RecalcAttributes(-1)` again, clears the ephemeral object's on-map byte, sets its limbo byte, invokes virtual UnInit, and returns success. For a wall, the material Recalc order is therefore cleanup `N,E,S,W,self`, followed by a second anchor Recalc in the common tail.
8. **Apply later OverlayData.** After every identity/Mark row, `ReadMapOverlayPacks 0x005FD5F7..0x005FD656` writes decoded data directly to every admitted real cell's `+0x11E`; it does not rerun wall cleanup. Present data-pack bytes therefore replace Mark-computed wall connectivity/damage state. If the data pack is absent/empty, Mark's connectivity byte remains. Full_Init's later global Recalc observes the final byte but does not rebuild wall connectivity.

### Authored auto-destruction is excluded

`PostDestructionWallCleanup` contains generic hardcoded removal branches for isolated damaged `GAWALL`, `NAWALL`, `GASAND`, `CYCL`, `FENC`, and `BARB` states and decrements eight neighbor counts after such a removal. Those branches are active generic runtime code but cannot fire during this authored identity pass:

- `CellClass::Constructor 0x0047BBF0` initializes identity `-1`, data `0`, owner `-1`, and `+0x122=0` (`0x0047BC21..0x0047BC2A`, `0x0047BD1C`, `0x0047BD34`).
- Every earlier packed authored wall Mark writes data `0`. Retail low procedural writes use state `0/1/2` on bridge-body identities and never call wall cleanup; those bytes remain below every hardcoded damage threshold. High bridge work does not introduce a damaged wall byte.
- OverlayData, including any damage upper nibble, is applied only after all Mark calls and does not call the cleanup helper.

Thus authored finalization must reproduce connectivity/Recalc/count effects but must not invent a damaged-neighbor auto-destruction fixture for this phase. Runtime wall damage/cleanup keeps those generic branches.

## Native fixed-grid lookup and the count plane

`Adjacent_Cell @ 0x00481810` adds the selected signed-i16 direction offset, then calls `MapClass::Get_CellClass @ 0x005657A0`. That lookup sign-extends the operands and tests `index = y*512 + x`; it does not first reject each axis against the logical map rectangle. A coordinate that looks out-of-range per axis can therefore alias an allocated real fixed-grid slot. Only an invalid/null linear slot returns the single dummy `CellClass @ 0x00ABDC50`.

Consequences:

- **Active in YR: Yes.** Every real or fixed-stride-aliased neighbor receives the raw wrapping count increment and native A* can observe it.
- **Active in YR: Yes but output-inert here.** Multiple true misses can increment the same dummy byte. A whole-program exact `+0x122` instruction audit found `AStar_main_loop @ 0x00429EB1` as the only gameplay decision read; other exact Cell sites construct, copy, increment, or decrement the byte. A* obtains candidates directly from the real 512x512 pointer table at `0x00429E0B..0x00429E21` and skips a null slot, so the fallback dummy can never reach that read. Fresh-load movement/path/placement needs no dummy count field.
- **Active in YR: Conditional.** Final wall identities cannot reconstruct the real count plane. A later low procedural body write performs `GetCell -> Scenario RNG -> direct +0x44/+0x11E stores -> Recalc` at `0x005FCB44..0x005FCB70` or `0x005FCF72..0x005FCF9E`. It has no occupancy test, DestroyOverlay, wall cleanup, or `+0x122` decrement. It can overwrite an earlier wall while leaving that wall's eight increments behind.
- Fixed low endpoint rows do not create this overwrite: wood `0x005FC907..0x005FC956` and concrete `0x005FCD35..0x005FCD84` probe all three cells for identity `-1` and abandon the fixed/body transaction if any is occupied.

A concrete compatible custom case is an earlier west `LOBRDGE2 0x7B` endpoint and a later east `LOBRDGE1 0x7A` endpoint with an authored wall placed between them in decoded order. The later endpoint finds the opposing center and its unconditional body rows overwrite the wall. The wall identity disappears; its previous real-cell `+0x122` contributions remain. The prior exhaustive 385-payload low-trigger census found no low trigger IDs in shipped/installed content, so this compound case is custom/editor-conditional, not dormant.

The exact retained count plane must replace identity-derived authored-wall accounting, not supplement it: supplementing would double-count surviving packed walls, while rebuilding from final identity would still lose stale contributions from overwritten walls.

## Active retail data and map reachability

`OverlayTypeClass` defaults `Wall` false at `0x005FE296`; `ReadINI 0x005FE7CB..0x005FE7E5` reads the literal `Wall` string at `0x0081AC58`. Dense zero-based runtime IDs come from retail `[OverlayTypes]`, not from the sparse INI key itself. Full_Init's rules-reset corridor empties the OverlayType registry at `0x00668783..0x006687A7`; `RulesClass::Process 0x00668CE3..0x00668D2F` enumerates active source entries through `GetEntryNameByIndex @ 0x00526CC0`; `OverlayTypeClass::Constructor @ 0x005FE250` takes the insertion-array index for `+0x294`. Retail has 250 active rows and omits numeric keys `40`, `41`, and `183`, so those gaps are compressed.

| Runtime ID | Type | Retail binding | Installed authored occurrence |
|---:|---|---|---:|
| `0` | `GASAND` | explicit `Wall=yes`; wall art | 19 maps / 2,622 cells |
| `2` | `GAWALL` | explicit `Wall=yes`; `Foundation=1x1`, `ToOverlay=GAWALL`, `DamageLevels=3` | 11 / 683 |
| `26` (`0x1A`) | `NAWALL` | explicit `Wall=yes`; `Foundation=1x1`, `ToOverlay=NAWALL`, `DamageLevels=3` | 5 / 210 |
| `203` (`0xCB`) | `CAFNCB` | explicit `Wall=yes`; wall art | 27 / 2,058 |
| `204` (`0xCC`) | `CAFNCW` | explicit `Wall=yes`; wall art | 19 / 1,160 |
| `240` (`0xF0`) | `CAKRMW` | explicit `Wall=yes`; `Foundation=1x1`, `ToOverlay=CAKRMW` | 8 / 901 |
| `241` (`0xF1`) | `CAFNCP` | explicit `Wall=yes`; `Foundation=1x1`, `ToOverlay=CAFNCP` | 41 / 3,779 |
| `243` (`0xF3`) | `GAFWLL` | explicit `Wall=yes`; `Foundation=1x1`, `ToOverlay=GAFWLL`, `DamageLevels=3` | 13 / 1,651 |

Literal retail references are `rulesmd.ini:[OverlayTypes]` at lines `1736..1983`; the eight `Wall=yes` writes are at `16388`, `12031`, `12827`, `16415`, `16442`, `22046`, `29911`, and `13571` for the table order above. Matching art is at `artmd.ini:4051..4071`, `4122..4141`, `6722..6727`, and `9304..9309`.

The eight wall types total 13,064 cells across 71 MIX entries: 70 unique named maps plus hash-only entry `0x9498E004`; the named-only subtotal is 13,026 cells. `GAFWLL` is stock-map active, not merely custom-conditional. `CRATE 179 (0xB3)` and `WCRATE 242 (0xF2)` are explicit crate types rather than walls; each occurs once (`xmp01du.map (170,96)` and `xmp25mw.map (106,120)` respectively). Registered `CYCL`, `BARB`, `WOOD`, and `FENC` have no active YR type section setting `Wall=yes`; constructor default false excludes them from the authored wall branch under retail rules.

The active retail boundary is 187/187 distinct logical map IDs, each decoded from its startup-priority winner. `Init_Mix_Files` mounts the expansion map media plus `multimd.mix`; base `MAPS01.MIX`, `MAPS02.MIX`, and `MULTI.MIX` appear only under tooling's startup-skipped expansion and are not part of this active-YR boundary. Loose/custom maps and generated `RandMap.Sed` are likewise not retail-authored payloads. `expandmd01.mix` shadows 13 names before the map archives; active-vs-shadow comparison plus the identical decoded `all02umd` OverlayPack proves the table totals against the winning bytes. The three hash-only entries are valid `NewINIFormat=4` maps (`0x9498E004`, `0x94C4BDFA`, `0x61B60AB4`), each has a complete OverlayPack. Entry `0x9498E004` contains 38 `CAFNCB` cells and zero of every other wall/crate target; the other two contain zero of every target. MIX logical names are not recoverable from hashes alone, but map behavior and counts are fully classified.

The stock activation is not inferred from byte occurrence alone. Active lookup resolves `xdustbowl.map` hash `0xE4BFD9FD` directly to `multimd.mix` with no higher-priority shadow. Cell `(96,84)` resolves through its decoded IsoMapPack5 record and active `temperatmd.ini`/`Green01.tem` retail TMP to slope index `0`; its `GASAND` row therefore passes the universal slope gate and executes the `ScenarioInit`-short-circuited wall transaction.

## Current Rust ownership and mismatches

1. `src/map/authored_overlay.rs::LiveOverlayCells` and `FinalizedOverlayPayload` retain only overlay identity/state. They have no authored real-cell `+0x122` plane.
2. `SharedCellDummy` retains coordinate/level/slope/bridge bits plus overlay identity/state, but no neighbor count. That is sufficient for this fresh-load count result only if true dummy increments remain output-inert; real fixed-stride aliases must still resolve through `LiveOverlayCells::target` / `ResolvedTerrainGrid::native_fixed_cell_index`.
3. `src/sim/movement/bump_crush.rs::build_blocker_neighbor_counts_with_overlays` reconstructs wall contributions from final wall identities. It necessarily loses stale contributions from a later low-body overwrite and can clip native real aliases.
4. `src/sim/pathfinding/core.rs::BlockerNeighborCounts` already uses wrapping `u8` add/sub semantics, but its ordinary rectangular increment helper is not the authored fixed-grid lookup authority.
5. `src/sim/overlay_grid.rs::refresh_wall_connectivity_after_placement` visits self before `N,E,S,W`; native placement cleanup visits `N,E,S,W,self`. It cannot be reused unchanged for the authored transaction.
6. `src/sim/production/wall_placement.rs::stamp_wall` also publishes its passability cross self-first. That ordinary runtime owner is a recorded neighboring mismatch; authored finalization must not inherit its order accidentally.
7. `src/sim/world/load_object_lifecycle.rs::finish_wall_reject` models the generic rejection lifecycle. The function may remain for counter-zero runtime construction, but authored loading must never select it. Existing comments/tests that present it as an authored terminal path are stale.
8. The current authored loader does not run this wall success transaction at all, so it cannot emit cleanup Recalc/effect order, connectivity, owner-none proof, or raw neighbor counts.

## Adversarial cases

| Case | Native verdict |
|---|---|
| Reader-admitted wall on a cell with a prior non-wall overlay, occupation bits, bridge flags, or nonbuildable land | Accepted when slope gate passes; `ScenarioInit` returns before all those predicate checks. |
| Wall anchor slope `5`, ID not `0xB2` | Rejected before the wall predicate; no wall stamp/cleanup/count increments. |
| Same wall construction with `ScenarioInit==0` and predicate false | Generic `0x005FC77C` UnInit path remains reachable; this is not authored Full_Init. |
| Adjacent same-ID wall vs different-ID wall before Buildings load | Same identity connects; merely sharing `Wall=yes` does not. No gate/tower object connection exists yet. Cleanup Recalc order is `N,E,S,W,self`, then the common tail Recalcs the anchor again. |
| Edge direction linearizes to an allocated fixed-grid slot | The real alias receives connectivity/count effects and is observable. Rectangular clipping is wrong. |
| Edge direction is a true invalid/null fixed-grid slot | Shared dummy is mutated, but no fresh-load path/movement/placement decision reads its count. |
| Wall followed by low fixed-row attempt over that cell | Fixed transaction aborts after all three occupancy probes; wall/count remain. |
| Wall followed by a later low body row over that cell | Body overwrites identity/state directly; previous real-cell count contributions remain. |
| OverlayData supplies a wall byte after Mark | It replaces connectivity/damage state without cleanup; count plane is unchanged. |
| Authored constructor receives `-1` pending owner | Wall remains unowned despite taking the full success path. |

## Open-question ledger

| ID | Question | Resolution |
|---|---|---|
| OQ-1 | Is `0x00A8E7AC` zero at the authored wall call? | **Resolved: no; nonzero through `0x00687A34`.** |
| OQ-2 | Is the apparent decrement before the reader on the success path? | **Resolved: no; `0x006878CC` skips the failure decrement.** |
| OQ-3 | Does Mark skip the helper or does the helper return true? | **Resolved: Mark calls it; the helper immediately returns true.** |
| OQ-4 | What are the exact arguments? | **Resolved: `Track(1), null what, null who`.** |
| OQ-5 | Can authored wall predicate failure reach `0x005FC77C`? | **Resolved: no.** |
| OQ-6 | Does the universal slope gate remain? | **Resolved: yes, before the wall branch.** |
| OQ-7 | Is generic runtime wall rejection dormant? | **Resolved: no; counter-zero construction/placement callers remain active.** |
| OQ-8 | What is wall cleanup order? | **Resolved: `N,E,S,W,self`; each wall scans `N,E,S,W`.** |
| OQ-9 | Can authored walls connect to gate/tower objects? | **Resolved: not at this pre-Building phase; same overlay identity still connects.** |
| OQ-10 | Does `ScenarioInit` suppress every zone effect? | **Resolved: no; only Mark's later explicit pair. Cleanup Recalc/zone work remains.** |
| OQ-11 | Is an authored owner written? | **Resolved: no; constructor pending value is `-1`.** |
| OQ-12 | Which count neighbors mutate? | **Resolved: all eight enum directions with raw byte wrap.** |
| OQ-13 | Are per-axis edge misses always dummy? | **Resolved: no; signed `y*512+x` may alias a real slot.** |
| OQ-14 | Must dummy `+0x122` be retained for this slice? | **Resolved: no fresh-load decision consumer can read it.** |
| OQ-15 | Can final wall identities rebuild real counts exactly? | **Resolved: no; a later low body overwrite can leave stale positive contributions after the wall identity disappears.** |
| OQ-16 | Can authored cleanup auto-destroy damaged neighbors? | **Resolved: no during the identity pass; damage data arrives later. Generic runtime branch remains.** |
| OQ-17 | Is the wall branch active in shipped YR maps? | **Resolved: yes; 13,064 wall cells across 71 MIX entries.** |
| OQ-18 | Is any active-retail logical map unclassified or decoded from a shadowed payload? | **Resolved: no; 187/187 winner-resolved, including 13 `expandmd01` winners and three hash-only entries.** |
| OQ-19 | Can the low procedural writer itself leave a final `Wall=yes` identity through base `0xCD` variants? | **Resolved: no; compact source-order IDs make `0xCD..0xD0 = LOBRDB01..04`.** |

## Implementation handoff

1. Remove authored wall-predicate failure from G12 and from the ephemeral lifecycle contract. Keep the generic counter-zero rejection mechanism explicitly separate.
2. In the map-owned synchronous authored row transaction, dispatch `Wall=yes` after the universal slope gate. Treat the exact predicate result as guaranteed true under the already-proved successful `Full_Init` context; do not rerun a Rust approximation of its counter-zero body.
3. Stamp identity/data, execute the `N,E,S,W,self` cleanup and its Recalc/effect order, leave owner absent, then increment all eight neighbors through the signed fixed-grid real-or-dummy lookup. Do not use the existing self-first helper unchanged.
4. Extend the load receipt with one wrapping `u8` authored blocker-neighbor plane for allocated real cells, or an equivalent persistent effect state. The plane must survive into the sim/pathfinding count composition beside finalized overlays and be the sole authored-overlay source there; do not add a second contribution by scanning final wall identities. Later Terrain/Building/Foot sources compose onto it, and runtime wall placement/removal updates the same authoritative state. Identity-derived authored reconstruction is not parity.
5. Do not add a dummy count to the fresh-load receipt. Resolve every offset through the native fixed-grid seam first, retaining aliased real writes and discarding only a true dummy target from the output plane.
6. Keep OverlayData as the later byte authority without rerunning wall cleanup; keep the raw count plane unaffected by the data pass.
7. Add focused fixtures for: occupied/predicate-bypass acceptance; slope rejection; same/different wall connectivity; exact cleanup `N,E,S,W,self` then second-anchor-Recalc effect order; owner `-1`; wrapping eight-neighbor counts; real alias vs true dummy; OverlayData replacement; fixed-row non-overwrite; and low-body overwrite with retained stale counts.
8. Reopen the living inventory entry for authored wall finalization until this success path, retained count plane, stale-doc corrections, focused validation, and fresh-critic review all pass.

## Stale-document and source corrections

- `AUTHORED_OVERLAY_EPHEMERAL_OBJECT_FINALIZATION_REINVESTIGATION_GHIDRA_REPORT.md` incorrectly classifies the generic predicate-false/UnInit path as authored-reachable. Its generic lifecycle facts may be retained, but the authored reachability claim must be corrected.
- `docs/contracts/2026-08-31-authored-overlay-finalization-implementation-contract.md` G12 and its wall-failure fixture are wrong for authored Full_Init and must be replaced with the success/count requirements above.
- `AUTHORED_OVERLAYPACK_INLINE_TRANSACTION_REINVESTIGATION_GHIDRA_REPORT.md` is correct that the load-suppression counter bypasses ordinary placement rejection; this report supplies the exact wall-call proof.
- Any report or comment calling `0x00A8E7AC` map-editor mode is stale; `0x00A8ED6B` is the separate YR map-editor flag.
- Final-wall reconstruction language in `CELL_0X122_DYNAMIC_BLOCKER_LIFECYCLE_RUST_MAPPING_GHIDRA_REPORT.md` is incomplete for a wall later overwritten by low procedural body materialization.
- `LOW_OVERLAY_MARK_FIXED_MAP_STAMP_RNG_TRANSACTION_GHIDRA_REPORT.md:51..52,78` correctly maps compact runtime `0xCD` to `LOBRDB01`. Sparse INI key subtraction is not authority: native source-entry enumeration compresses omitted keys `40`, `41`, and `183`.

## Ghidra annotation candidates

No metadata was changed. Candidates for the later certainty-gated sync are:

- global `0x00A8E7AC`: rename/comment as `g_ScenarioInitDepth` / scenario-init load-suppression nesting counter; explicitly distinguish `g_IsMapEditor @ 0x00A8ED6B`;
- `OverlayClass::Mark @ 0x005FC570`: comment `0x005FC6F4..0x005FC705` as `Is_Clear_To_Build(Track,null,null); Full_Init ScenarioInit makes false branch unreachable`;
- `CellClass::PostDestructionWallCleanup @ 0x00480630`: plate comment that placement uses fixed `N,E,S,W,self`, then Mark separately increments eight neighbor `+0x122` bytes;
- `ReadMapOverlayPacks @ 0x005FD2E0`: comment constructor argument `-1` and later OverlayData precedence.

## Verification closeout

### Cold spot checks

1. Re-disassembly of `Full_Init` reproduced entry increment `0x00686B35..4F`, success jump `0x006878CC -> 0x00687924`, reader call `0x00687A34`, and later temporary-zero/restore `0x00687C2B..44`.
2. Re-disassembly of `OverlayClass::Mark`, `Is_Clear_To_Build`, and the reader constructor call reproduced `PUSH 0/PUSH 0/PUSH 1`, immediate nonzero-counter success, wall stamp/cleanup/count order, and constructor `PUSH -1 @ 0x005FD4C3`.
3. Re-disassembly of Rules processing reproduced source-entry enumeration and insertion-index identity assignment; an independent ordered retail parse reproduced `0xCB CAFNCB`, `0xCC CAFNCW`, `0xCD LOBRDB01`, `0xF0 CAKRMW`, `0xF1 CAFNCP`, and `0xF3 GAFWLL`.

### Zero-additional-mechanism pass

- `get_function_callers(0x0047C620)` found the expected placement, preview, Mark, and CanPlace callers; no separate authored wall predicate exists.
- `get_function_callers(0x00480630)` found Building Unlimbo/Limbo, DestroyOverlay, House sell-at-cell, and Overlay Mark; authored Mark uses this same cleanup owner.
- Whole-function Mark search found the only `+0x122` access at `0x005FC762..0x005FC768`; low fixed/body write corridors contain no count increment/decrement or DestroyOverlay call. Native source-entry enumeration and constructor insertion indexing bind retail `CAFNCB/CAFNCW` to compact IDs `0xCB/0xCC` and `LOBRDB01` to `0xCD`; the table's `0xCD` variants therefore remain bridge bodies.
- Whole-program exact `0x122` instruction search found A* as the only gameplay decision read of the Cell byte; placement predicates contain no read.
- Retail rules/art enumeration found exactly eight explicit active-YR `Wall=yes` overlay types. Independent encrypted-MIX enumeration, active-priority winner resolution, and Base64/chunked-LCW decoding produced a full 262,144-byte OverlayPack for each of 187/187 retail logical maps and classified every compact-ID occurrence. All 13 `expandmd01` shadows were compared; only `all02umd.map` differs as a whole file, and its decoded OverlayPack is byte-identical to the lower-priority copy.
- No Tube constructor/call belongs to this wall-success path. GSI-04.15 remains a negative separation here.
