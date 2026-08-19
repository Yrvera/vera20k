# gamemd.exe bridge subsystem map

**Date:** 2026-08-19
**Scope:** the Bridge, Tube and Ramp function families in active Yuri's Revenge `gamemd.exe`,
plus the globals they read and write.
**Ghidra:** `testProsjekt` / `gamemd.exe`, MCP on `127.0.0.1:8089`.

**Nothing in this document is VERIFIED.** Every finding here is static binary evidence only —
decompile, disassembly, vtable bytes, xrefs. No claim was checked against a live YR run, so
every entry reads `UNCHECKED` per `ENGINE.md`. Where a prior session's plate comment already
carried a dated verification, it is quoted as *that session's* claim, not re-certified here.

This is a mapping pass, not a port. No Rust changed.

---

## 1. How to read this map

The family splits into four tag groups, applied as Ghidra function tags this session so the
set stays queryable (`search_functions_by_tag BRIDGE_HIGH` etc.):

| Tag | Members | Meaning |
|---|---|---|
| `BRIDGE_HIGH` | 60 | Elevated concrete bridge — deck sits 4 levels above ground |
| `BRIDGE_LOW` | 66 | Ground-level wooden bridge |
| `BRIDGE_TUBE` | 12 | `TubeClass` — the low-bridge under-pass ("drive under the bridge") |
| `BRIDGE_RMG` | 16 | Random-map-generator bridge and ramp carving |

Counts overlap: shared helpers carry both `BRIDGE_HIGH` and `BRIDGE_LOW`. 123 distinct
functions are tagged.

**The High/Low split is partly a data split — but only among the members that touch a tileset
base at all.** State it precisely, because the loose version is false:

- **Members that reference a tileset base** are cleanly split, and *no* member reads both. High
  members compare against `g_BridgeSet_TileSetBase` (`0x00AA0E28`) and are the ones that apply
  the four-level deck rise (`cell+0x11B += 4`); Low members use `g_WoodBridgeSet_TileSetBase`
  (`0x00ABAD1C`). **Corrected 2026-08-19: the rise is NOT a High-only marker.**
  `MapClass__RepairBridgeOrRestoreRamp_Low` `0x00570050` reads the wood base and applies
  `cell+0x11B += 4` to three neighbours while restoring a repaired low bridge, so the base a
  member reads is the reliable half of this split and the deck rise is not. The only functions appearing in both xref lists are
  the genuinely shared ones of §6 — `ComputeBridgeZones`, `Add`/`RemoveBridgeZoneEdges`,
  `RegisterBridgeOrTubeHierarchyPairs`, `ApplyDamageToCell`, `Apply_area_damage`,
  `FindBridgeConnection_Predicate`, `CellClass__IsSpecialTerrainTile`,
  `Read_Theater_TileSets_INI`.
- **Most of the span machinery references neither base.** `DestroyBridge_High 0x0057CCF0`,
  `DestroyBridge_Low 0x0057BAA0`, and essentially all of §4.2, §4.3, §5.2 and §5.3 split on the
  **overlay ordinal `cell+0x44`** instead — the bands in §2.3. That is the real discriminator
  for the walkers.

So: the ramp updaters, edge-tile updaters, pavement span walkers and damage state machines are
tileset-keyed; the collapse/destroy/repair walkers are overlay-keyed.

Both bases are written by `Read_Theater_TileSets_INI` and are `-1` in a theater with no bridge
tileset, which is why every predicate tests the sentinel first. The write sites differ per base:
`g_BridgeSet_TileSetBase` at `0x00545A80`, `0x00545DDB`, `0x00546CB3`;
`g_WoodBridgeSet_TileSetBase` at `0x00545A86`, `0x00545DEA`, `0x00546CB9`.

*Evidence: `get_xrefs_to 0x00AA0E28` and `get_xrefs_to 0x00ABAD1C` (both complete lists);
`decompile_function 0x00486750`, `0x00486770`, `0x0057CCF0`, `0x0057BAA0`, `0x00568E40`,
`0x00570050`.*

---

## 2. State model

### 2.1 `CellClass` fields the family touches

| Offset | Type | Role | Where established |
|---|---|---|---|
| `+0x24/+0x26` | i16 x2 | Map coord (packed as one dword) | used everywhere |
| `+0x2C` | ptr | Back-pointer to the bridge anchor cell; cleared on destroy | `SetBridgeDirection_NESW 0x0047E040` |
| `+0x38` | i32 | Iso-tile type index — compared against the tileset bases | `CellClass__IsBridge 0x00486750` |
| `+0x3C` | ptr | Trigger `Tag`; collapse fires event `0x1F` through it | `NotifyBridgeSpanCollapse 0x00575EE0` |
| `+0x44` | i32 | Overlay type — the bridge band, see 2.3 | `DestroyBridge_Low 0x0057BAA0` |
| `+0xE4 / +0xE8` | ptr | Ground object list / bridge-deck object list | `FindNearbyBridgePeer 0x0042B080` |
| `+0xEC` | i32 | LandType; `10` marks a tube mouth | `CellClass__IsTubeCell 0x00484AB0` |
| `+0x116` | i16 | Tube index into `g_TubeArray`, `-1` when none | `MapClass__ReadTubesINI 0x007283C0` |
| `+0x118` | u8 | Cached bridge-overlay draw generation | `CellClass__DrawOverlay_Body 0x0047F6A0` |
| `+0x11A` | u8 | Iso sub-tile index — ramp/endpoint discrimination | `MapClass__IsBridgeRampTile 0x005746C0` |
| `+0x11B` | i8 | Cell level; bridge deck is ground `+4` | `CheckBridgeTraversal 0x004D9C60` |
| `+0x11C` | u8 | Raw TMP slope byte — any nonzero value permits a 1-level step | `CheckBridgeTraversal 0x004D9C60` |
| `+0x11E` | u8 | Bridge damage frame (`0..3 -> 4`, `5 -> 6`) | `UpdateRamp_NS_DamageA_Low 0x0056ED40` |
| `+0x124` | u8 | Ground occupation | `UpdateBridgePassability 0x0042ACF0` |
| `+0x140` | u32 | Flags — see 2.2 | `SetBridgeDirection_NESW 0x0047E040` |

### 2.2 `CellClass+0x140` bridge flag bits

Established by reading the single writer, `CellClass__SetBridgeDirection_NESW 0x0047E040`
(and its byte-identical twin `_NWSE 0x0047E470`), and cross-checked against the consumers.

`0xFFFEE07F` — clearing bits 7–12 and 16 — is the mask of the **first** write, the one applied
to the anchor cell. It is *not* the function's uniform mask: the function writes flags to up to
six cells and each write clears a different set. The per-cell table below matters more than the
mask, because a port that applies one mask everywhere gets the deck-entry and orientation bits
wrong on the neighbours.

| Bit | Set as | Which of the six writes set it | Consumer evidence |
|---|---|---|---|
| `0x0080` | `(param_3 & 1) << 7` | write 1 (anchor) **only**; the later writes neither set nor clear it | `DrawOverlay_Body 0x0047F6A0` uses it to select the deck branch and to add `(Flags>>7 & 1)*4` levels; the ramp updaters gate their damage-frame bump on it |
| `0x0100` | `(param_3 & 1) << 8` | writes 1, 2, 3, 5 | "cell belongs to a bridge" — the primary test in `CheckBridgeTraversal`, `UpdateBridgePassability`, `FindNearbyBridgePeer`, `ResolvePathCoord_BridgeAware`, `IsOnBridge_ForFiring` |
| `0x0200` | `(param_3 & 1) << 9` | writes 1, 2, 5. **Write 3 clears it and never sets it** | required *together with* `0x0100` to enter the deck in `CheckBridgeTraversal 0x004D9C60` |
| `0x0400` | `((char)param_3 == 0) << 10` — a **byte** test on the low 8 bits, not a dword test | writes 1, 2, 3, 5 | set when the direction stamp is a *clear*; `MapClass__RepairBridgeOrRestoreRamp_Low` scans neighbours on `0x400` and on the pair mask `0x500` |
| `0x0800` | `(param_2 == 0) << 11` — a genuine dword test, unlike `0x400` | writes 1, 2, 3, 5 | bridge axis/orientation. `IsOnBridge_ForFiring 0x00703B10` pairs each of four neighbours with a required `0x800` state, which is what proves this bit selects NS vs EW |
| `0x1000` | `(param_3 & 1) << 12` | writes 1, 2, 3, 4 — **four** cells. **Write 5 clears it and never sets it** | — |
| `0x10000` | `(param_3 & 1) << 16` | writes 1, 2, 3, 5, 6 — **five** sites | — |
| `0x40000` | XOR-toggled elsewhere | not written here at all | **not** a bridge structure bit — the A\* temporary cost marker set by `UpdateBridgePassability`; A\* multiplies a marked destination's cost by `4.0` (`0x007E37BC`) |

Write 4 is the narrowest: `Flags & 0xFFFFEFFF | uVar15`, i.e. it touches only `0x1000`. Write 6
fires only when `param_2 == 6` and touches only `0x10000`.

*Evidence: `decompile_function 0x0047E040` (all six flag-write expressions),
`disassemble_function 0x0047E040`, `0x0047F6A0`, `0x004D9C60`, `0x0042ACF0`, `0x00703B10`,
`0x00570050`.*

### 2.3 Bridge overlay bands (`CellClass+0x44`)

The low family's dispatchers classify a cell purely by its overlay ordinal:

- **Low, NS-class:** `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}`
- **Low, EW-class:** `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}`
- **Whole low band** (the "is this a bridge overlay at all" test): `[0x4A..0x65]`
- **High, NS-class:** `[0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}`
- **High, EW-class:** `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}`

**Naming trap, carried from prior sessions and confirmed here:** the `_NS_` / `_EW_` suffix on
the walkers names the *overlay orientation*, and the walker then steps along the
**perpendicular** axis. `DestroyBridgeFromCell_Low` reads an NS-band overlay and calls
`CollapseBridge_EW_Low`. Do not read the suffix as the walk axis.

*Evidence: `decompile_function 0x0057BAA0`, `0x00574780`; plate comment on `0x0057CCF0`.*

### 2.4 `BridgeRecord` — the persistent span table

`MapClass+0x54` is the array base, `MapClass+0x60` the count, stride 16 bytes.

| Offset | Type | Role |
|---|---|---|
| `+0x00` | packed i16 x2 | Endpoint A |
| `+0x04` | packed i16 x2 | Endpoint B |
| `+0x08` | u8 | `is_intact` |
| `+0x0C` | i32 | Kind — `0` is high bridge; nonzero is skipped by `FindBridgeRecord` |

Built once at map load by `MapClass__ComputeBridgeZones 0x0056D6E0`; flipped by
`InvalidateBridgeZones` (intact → 0, then `RemoveBridgeZoneEdges`) and `ValidateBridgeZones`
(intact → 1, then `AddBridgeZoneEdges`, then a `Can_Reach_Zone` check).

*Evidence: `decompile_function 0x0056DA10`, `0x0056DAE0`, `0x0056DB70`, `0x00583180`
(`DAT_0087F83C` = `0x0087F7E8 + 0x54`).*

### 2.5 The `MapClass` singleton

`0x0087F7E8`. Proved rather than assumed: `g_CellArray_Base` is `0x0087F924`, exactly
`0x0087F7E8 + 0x13C`, and `MapClass__RepairBridgeOrRestoreRamp_Low 0x00570050` reads its cell array as
`this->+0x13C`. This matters because several one-line family members are `__fastcall` with the
receiver loaded as a literal (e.g. `MOV ECX,0x87f7e8` at `0x004F4311`), and the singleton
identity is the only thing that makes their field offsets meaningful.

*Evidence: `list_globals name_substring=CellArray`; `decompile_function 0x00570050`;
`disassemble_function 0x004F42F0`.*

### 2.6 Globals

| Global | Address | Role | Writer |
|---|---|---|---|
| `g_BridgeSet_TileSetBase` | `0x00AA0E28` | High/concrete bridge tileset base | `Read_Theater_TileSets_INI` |
| `g_WoodBridgeSet_TileSetBase` | `0x00ABAD1C` | Low/wooden bridge tileset base | `Read_Theater_TileSets_INI` |
| `g_TubeArray` | `0x008B413C` | Tube record vector | `TubeClass__Constructor` |
| `g_TubeCount` | `0x008B4148` | Tube count | Written at only three sites: `TubeClass__Constructor 0x0072806F`, `0x007287A8`, `0x004E6791`. **`MapClass__WriteTubesINI` does not write it** — it holds three READs (`0x0072829B`, `0x00728302`, `0x007283A7`) and no WRITE, so its compaction decrements a local, and its emission loop re-reads the undiminished global while indexing a compacted array |
| `g_BridgeZOffset_Drive` | `0x008A07C4` | `ftol(4 * g_DriveHeightStep)` — the deck rise for Drive | `DriveLocomotionClass__ComputeBridgeZOffset 0x004AF4A0` |
| `g_BridgeZOffset_Teleport` | `0x00B0EC2C` | Deck rise for the teleport locomotor | not traced this session — UNCHECKED |
| `g_BridgeZ_Offset` | `0x00B0782C` | Deck rise, third consumer | not traced this session — UNCHECKED |
| `g_nFootOnBridgeDeckOffsetLeptons` | `0x00AC13BC` | `416` leptons added when `Object+0x8C` OnBridge is set | `FootClass__Set_Height_On_Bridge 0x005F5FA0` |
| `g_BridgeEndpoint_InvalidCoord_Sentinel` | `0x0089C278` | Endpoint search failure value | — |
| `g_nHighBridgeZoneStartSubtileByTileOffset` | `0x0082A734` | `int[16]`, start sub-tile per tile offset | consumed by `ComputeBridgeZones` |
| `g_nHighBridgeZoneEndSubtileByTileOffset` | `0x0082A7B4` | `int[16]`, end sub-tile | consumed by `ComputeBridgeZones` |
| `g_nHighBridgeZoneWalkDirectionByTileOffset` | `0x0082A774` | `int[16]`, walk direction | consumed by `ComputeBridgeZones` |
| `g_nHighBridgeHierarchyOffsetDirectionByTileOffset` | `0x0082A944` | `int[16]` | consumed by the zone hierarchy pass |
| `g_BridgeDiag_BothSides_2_0` | `0x007E37B4` | A\* float `2.0` | `AStar_compute_edge_cost` |
| `g_BridgeDiag_NonBridge_10_0` | `0x007E37B8` | A\* float `10.0` | `AStar_compute_edge_cost` |
| `g_BridgeApproach_CostMult_4_0` | `0x007E37BC` | A\* float `4.0` applied to a `0x40000`-marked destination | A\* at `0x004299AA` |
| `g_szBridgeRepairHutTypeName` | `0x0082BA00` | Repair-hut type name string | RMG hut placement |
| `MapClass+0x1158` | `0x00880940` | Bridge-overlay redraw generation counter | `MapClass__IncrementBridgeCounter 0x00578AC0` |
| `vtable__TubeClass` | `0x007F59B0` | Primary TubeClass vtable | — |

---

## 3. Entry points — what actually reaches this family

Everything below is a verified caller binding (`get_function_callers` / `get_bulk_xrefs`),
which is what fixes each family member's trigger and frequency.

| Entry | Reaches | Trigger | Skirmish frequency |
|---|---|---|---|
| ~~`LogicClass__PerTickUpdate 0x0055AFB0`~~ | ~~`RecalcBridgeShroudFlags 0x00578100`~~ | **RETRACTED 2026-08-19** — `0x00578100` is not a bridge function; see §10 | — |
| `AStar_main_loop 0x00429A90` | `UpdateBridgePassability 0x0042ACF0` → `FindNearbyBridgePeer 0x0042B080` | every A\* search with nonzero urgency | very high — every unit path request |
| `UnitClass__AI 0x007360C0` | `UnitClass__TubeMovement 0x007359F0` | per tick per unit inside a tube | high on maps with `[Tubes]` |
| `UnitClass` / `InfantryClass` `Can_Enter_Cell` | `CheckBridgeTraversal 0x004D9C60`, `CellClass__IsTubeCell 0x00484AB0` | every movement legality test | very high |
| `ApplyDamageToCell 0x00587180` | `ProcessBridgeDamageStateMachine_Low 0x00571490` / `_High 0x00576BA0`, `DestroyBridge_High 0x0057CCF0` | a weapon damages a bridge cell | uncommon — only when a bridge is shot |
| `Apply_area_damage 0x00489280` | `DestroyBridge_High 0x0057CCF0` | area damage over a deck | uncommon |
| `BombClass__Detonate 0x00438720` | `DestroyBridge_{High,Low}_OnHutDeath 0x00574000` / `0x00574C20` | planted charge kills the repair hut | rare per match, decisive |
| `BuildingClass__Update 0x0043FB20` | same pair | repair hut death noticed by building update | rare per match |
| `InfantryClass__PerCellProcess 0x00519630` | `ProcessBridgeDestruction_{Low,High} 0x00570050` / `0x00573540` | infantry enters a bridge cell | common on bridge maps |
| `InfantryClass__What_Action_On{Cell,Object}` | `FindBridgeConnection_Predicate 0x00587410`, `CellClass__GetTubeAtCell 0x00484F20` | cursor hover with infantry selected | very high while hovering |
| `TechnoClass__GetFireError 0x006FC0B0` | `IsOnBridge_ForFiring 0x00703B10` | every fire-error evaluation | very high |
| `UnitClass` draw vtable slot `0x1CC` | `Draw_Sprite_With_BridgeFudge 0x0073B140` → `CountAdjacentBridgeDeckTiles 0x00703E70` | every visible unit draw | every frame |
| `MapClass__InitZoneMap 0x00567110` | `ComputeBridgeZones 0x0056D6E0` | map load | once per match |
| `ScenarioClass__Full_Init 0x00686B20` | `MapClass__ReadTubesINI 0x007283C0` | map load | once per match |
| `Save_Scenario_Map_File 0x00687CE0` | `MapClass__WriteTubesINI 0x00728280` | map save | **never in skirmish** — editor path |
| `ScenarioClass__Read_Scenario 0x00684620` | `RandomMapGenerator__Generate 0x00598960` | only when the scenario extension is `.SED` | **never in stock skirmish** — see §8.1 |
| `ChooseMap__AcceptRandomMapSetup 0x005E8590` | `RandomMapSetupDialog__Run 0x00595BC0` → `Proc 0x00596300` → `Generate` | random-map setup dialog | see §8.1 |

---

## 4. Inventory — High bridge (`BRIDGE_HIGH`)

All entries live in stock YR unless marked otherwise. Evidence for each is the caller binding
in §3 plus its `g_BridgeSet_TileSetBase` usage; all `UNCHECKED`.

### 4.1 Entry and dispatch

| Address | Name | Purpose |
|---|---|---|
| `0x00574000` | `MapClass__DestroyBridge_High_OnHutDeath` | Repair-hut death → locate span anchor → hand to `0x005749C0` |
| `0x005749C0` | `MapClass__DestroyBridgeFromCell_High` | Detect span axis from the anchor overlay, dispatch a Collapse walker |
| `0x00576BA0` | `ProcessBridgeDamageStateMachine_High` | Per-cell damage state machine; drives the 8 `*_High` ramp updaters and `BlowUpBridge` |
| `0x00573540` | `MapClass__RepairBridgeOrRestoreRamp_High` | Post-destruction cell fixup; calls `RepairBridge_High`, `ToggleBridgePavement`, `ValidateBridgeZones`, `BridgePavementSpanWalker_High` |
| `0x0057CCF0` | `DestroyBridge_High` | Per-cell destroyer; classifies the high overlay band and dispatches a `DestroyBridgeWalker_*_High` |

### 4.2 Span walkers

| Address | Name | Purpose |
|---|---|---|
| `0x00575870` | `MapClass__CollapseBridge_EW_High` | Collapse walk, EW-band anchor (steps NS) |
| `0x00575BA0` | `MapClass__CollapseBridge_NS_High` | Collapse walk, NS-band anchor (steps EW) |
| `0x0057D530` | `MapClass__DestroyBridgeWalker_EW_High` | Per-cell destruction walk |
| `0x0057CF60` | `MapClass__DestroyBridgeWalker_NS_High` | Per-cell destruction walk |
| `0x0057ED00` | `MapClass__ApplyBridgeDestruction_EW_High` | Applies the destruction result to the span |
| `0x0057E7A0` | `MapClass__ApplyBridgeDestruction_NS_High` | Applies the destruction result to the span |
| `0x0057CAB0` | `MapClass__CheckBridgeNeighbors_EW_High` | Neighbour band test used by the walkers |
| `0x0057CBE0` | `MapClass__CheckBridgeNeighbors_NS_High` | Neighbour band test used by the walkers |
| `0x0057DAF0` | `MapClass__FindBridgeEndpoints_EW_High` | Endpoint pair for the span; calls `NotifyBridgeSpanCollapse` |
| `0x0057DC20` | `MapClass__FindBridgeEndpoints_NS_High` | Endpoint pair for the span |
| `0x00568E40` | `MapClass__BridgePavementSpanWalker_High` | **Newly labelled.** 30-cell span rebuild: restamp ramp tiles, validate zones, place one `OverlayClass` deck piece per cell, return the dirty screen rect, then rebuild zone connectivity |

### 4.3 Repair

| Address | Name | Purpose |
|---|---|---|
| `0x0057F440` | `MapClass__RepairBridge_High` | Repair-side twin of `DestroyBridgeFromCell_High` |
| `0x00580600` | `MapClass__RepairBridgeWalker_EW_High` | Repair walk |
| `0x005800D0` | `MapClass__RepairBridgeWalker_NS_High` | Repair walk |

### 4.4 Tile and edge maintenance

| Address | Name | Purpose |
|---|---|---|
| `0x00576770` | `MapClass__UpdateAdjacentBridges_High` | Re-stamp neighbouring spans after a change |
| `0x00576200` | `MapClass__UpdateBridgeEdgeTiles_High` | Edge-tile restamp; calls `NotifyBridgeSpanCollapse` |
| `0x0047E040` | `CellClass__SetBridgeDirection_NESW` | The flag/anchor writer of §2.2. Called only by High-family sites (8 of 10 callsites are `*_High` updaters); the `NESW` half of the name is not provable from the body, which a prior session verified byte-identical to `0x0047E470` |

### 4.5 Ramp updaters (8 of 16)

`MapClass__UpdateRamp_{NS,EW}_{DamageA,DamageB,CollapseA,CollapseB}_High` at
`0x00572230`, `0x00572330`, `0x00572440`, `0x005727E0`, `0x00572B80`, `0x00572C90`,
`0x00572DA0`, `0x00573170`. Shape documented on the Low reference member `0x0056ED40` (§5.5).

### 4.6 Draw

| Address | Name | Purpose |
|---|---|---|
| `0x0073B140` | `UnitClass__Draw_Sprite_With_BridgeFudge` | `UnitClass` vtable slot `0x1CC`. Split-blits a `TooBigToFitUnderBridge` unit at a deck edge so it is not clipped |
| `0x00703E70` | `TechnoClass__CountAdjacentBridgeDeckTiles` | **Newly labelled.** Returns 0/1/2, never a true count: the first two direction hits assign 1 and the third adds 1. Consumers only compare against 0 — see §9 |

---

## 5. Inventory — Low bridge (`BRIDGE_LOW`)

### 5.1 Entry and dispatch

| Address | Name | Purpose |
|---|---|---|
| `0x00574C20` | `MapClass__DestroyBridge_Low_OnHutDeath` | Repair-hut death → `0x00574780` |
| `0x00574780` | `MapClass__DestroyBridgeFromCell_Low` | Overlay band → walk back ≤2 cells to the canonical anchor → `CollapseBridge_{EW,NS}_Low` |
| `0x00571490` | `ProcessBridgeDamageStateMachine_Low` | Per-cell damage state machine; drives the 8 `*_Low` ramp updaters, `BlowUpBridge`, `InvalidateBridgeZones` |
| `0x00570050` | `MapClass__RepairBridgeOrRestoreRamp_Low` | 5×5 rescan for a surviving bridge cell → `RepairBridge_Low`; otherwise walks the 8 directions, restamps pavement, recurses, and dirties the screen rect |
| `0x0057BAA0` | `DestroyBridge_Low` | Per-cell destroyer; returns 0 when the cell is outside the band so the caller retries |

### 5.2 Span walkers

| Address | Name |
|---|---|
| `0x00575220` | `MapClass__CollapseBridge_EW_Low` |
| `0x00575540` | `MapClass__CollapseBridge_NS_Low` |
| `0x0057C2B0` | `MapClass__DestroyBridgeWalker_EW_Low` |
| `0x0057BCF0` | `MapClass__DestroyBridgeWalker_NS_Low` |
| `0x0057E2A0` | `MapClass__ApplyBridgeDestruction_EW_Low` |
| `0x0057DD50` | `MapClass__ApplyBridgeDestruction_NS_Low` |
| `0x0057B870` | `MapClass__CheckBridgeNeighbors_EW_Low` |
| `0x0057B990` | `MapClass__CheckBridgeNeighbors_NS_Low` |
| `0x0057C870` | `MapClass__FindBridgeEndpoints_EW_Low` |
| `0x0057C990` | `MapClass__FindBridgeEndpoints_NS_Low` |
| `0x00569760` | `MapClass__BridgePavementSpanWalker` — the Low counterpart of `0x00568E40` |

### 5.3 Repair

| Address | Name |
|---|---|
| `0x0057F200` | `MapClass__RepairBridge_Low` — compiled twin of `DestroyBridgeFromCell_Low`, calls the Repair walkers instead of Collapse |
| `0x0057FBC0` | `MapClass__RepairBridgeWalker_EW_Low` |
| `0x0057F6A0` | `MapClass__RepairBridgeWalker_NS_Low` |

### 5.4 Cell and tile state

| Address | Name | Purpose |
|---|---|---|
| `0x0057A320` | `MapClass__ClearBridgeCell_Low` | Clear one deck cell |
| `0x0057A430` | `MapClass__UpdateBridgeTile_Low` | Re-select the tile for one deck cell |
| `0x0057ACF0` | `MapClass__SelectBridgeTileVariant_Low` | Variant pick from the surface mask |
| `0x0057B210` | `MapClass__ComputeBridgeSurfaceMask` | Surface mask; calls `IsOnBridgeRamp` |
| `0x00579B70` | `MapClass__ComputeBridgeAdjacencyMask_Low` | Adjacency mask, also used by the RMG dilation pass |
| `0x00570AE0` | `MapClass__UpdateBridgeEdgeTiles_Low` | Edge restamp; calls `NotifyBridgeSpanCollapse` |
| `0x00571050` | `MapClass__UpdateAdjacentBridges` | Neighbour span restamp |
| `0x00574600` | `MapClass__IsLowBridgeEndpointTile` | Endpoint tile predicate, keyed on `+0x11A` sub-tile and direction 2 or 4 |
| `0x00578D80` | `IsOnBridgeRamp` | Cliff/waterfall tileset ramp predicate. **The name overstates it** — the body tests `g_nCliffSet_TileSetBase[+0x28]` and the four waterfall tilesets, not a bridge tileset. Single caller, inside `ComputeBridgeSurfaceMask` |
| `0x0047E470` | `CellClass__SetBridgeDirection_NWSE` | Flag/anchor writer; called only by Low-family sites |

### 5.5 Ramp updaters (8 of 16)

`MapClass__UpdateRamp_{NS,EW}_{DamageA,DamageB,CollapseA,CollapseB}_Low` at
`0x0056ED40`, `0x0056EE40`, `0x0056EF50`, `0x0056F2F0`, `0x0056F690`, `0x0056F7A0`,
`0x0056F8B0`, `0x0056FC80`.

Reference member `0x0056ED40`, decompiled in full this session: step one cell along
`g_DirectionOffsets[dir & 7]`; if the cell carries flag `0x80`, advance the damage frame
`+0x11E` (`0..3 → 4`, `5 → 6`); then, keyed on
`(cell+0x38 − g_WoodBridgeSet_TileSetBase + 1)`, either `ToggleBridgePavement` for the two
endpoint tilesets or `FloodFillIsoTileType` to restamp the ramp piece. The Collapse members
clear the deck instead of advancing a frame, and the larger members add `BlowUpBridge` and
`SetBridgeDirection` calls — which is why `BlowUpBridge` has roughly 80 callsites, almost all
inside this family.

---

## 6. Inventory — shared across both layers

| Address | Name | Purpose | Trigger / frequency |
|---|---|---|---|
| `0x0047DD70` | `CellClass__BlowUpBridge` | Kills everything on a collapsing deck cell and spawns the debris. Walks `FirstObject` calling virtual `+0x16C` with the `Rules+0xFA8` warhead; walks `AltObject` calling virtual `+0xEC`; records the cell in a global list; then, gated on `Rules+0x168 > 0` and a `< 0.95` draw, spawns 1–2 anims from the `Rules+0x140` / `Rules+0x15C` lists. **Consumes 4–6 RNG draws per cell** — draw-order sensitive | roughly 80 callsites, all in the ramp/state-machine families; fires per cell of a collapsing span |
| `0x004D9C60` | `CheckBridgeTraversal` | The deck entry gate. Level difference exactly 4 is the bridge on/off ramp case; difference 1 needs a nonzero slope byte; 2, 3 and >4 are blocked. Virtual — reached only through 4 vtable slots (`0x007E2454`, `0x007E8E44`, `0x007EB208`, `0x007F5E20`), no direct callers | every `Can_Enter_Cell` — very high |
| `0x0042ACF0` | `PathfinderClass__UpdateBridgePassability` | A\* bridge-marker overlay; XOR-toggles cell bit `0x40000` over a replayed peer path and a 5×5 square. Direction `8` in a replayed path means "follow the tube at `cell+0x116`" — the one place the Tube and Bridge families meet inside pathfinding | every A\* search with nonzero urgency — very high |
| `0x0042B080` | `PathfinderClass__FindNearbyBridgePeer` | 5×5 fallback peer search when the probe's object list head is null; picks the deck list `+0xE8` when the level gap exceeds 2 | only when the probe list is empty |
| `0x004DDC40` | `FootClass__ShouldBeOnBridge` | Early-out on `Foot+0x684 >= 0`, else delegates to the Object version. Virtual, 4 vtable slots | movement/height updates |
| `0x005F6A70` | `ObjectClass__ShouldBeOnBridge` | Compares ground height at current vs target coord against `3 * g_nFootLevelHeightLeptons` and the target cell's `0x100` flag | as above |
| `0x005F5FA0` | `FootClass__Set_Height_On_Bridge` | Adds `g_nFootOnBridgeDeckOffsetLeptons` (416) when `Object+0x8C` is set; brackets the write in REMOVE/PUT when marked | Fly locomotor process — high for air units |
| `0x004AF4A0` | `DriveLocomotionClass__ComputeBridgeZOffset` | Static init: `g_BridgeZOffset_Drive = ftol(4 * g_DriveHeightStep)` | once at startup |
| `0x0069EBB0` | `ShipLocomotionClass__Compute_BridgeZOffset` | Ship-side deck offset; body not re-derived this session — **UNCHECKED** | once at startup (no CALL xrefs) |
| `0x00486750` | `CellClass__IsBridge` | High tileset membership, 16 tiles | zone build and path-coord resolution — high |
| `0x00486770` | `CellClass__IsWoodBridge` | Low tileset membership, 16 tiles | as above |
| `0x005746C0` | `MapClass__IsBridgeRampTile` | Ramp tile predicate over six tileset ids paired with required `+0x11A` sub-tile values | span anchor location |
| `0x0056D6E0` | `MapClass__ComputeBridgeZones` | Builds the `BridgeRecord` table at map load | `InitZoneMap`, `Invalidate`/`ValidateBridgeZones`, RMG — once per map plus on invalidation |
| `0x0056DA10` | `MapClass__FindBridgeRecord` | Linear scan of `BridgeRecord`s with a perpendicular tolerance; skips kind ≠ 0 and ignores `is_intact` | every bridge-aware path coord and cursor test — high |
| `0x0056DAE0` | `MapClass__InvalidateBridgeZones` | `is_intact → 0` plus `RemoveBridgeZoneEdges` | bridge destroyed |
| `0x0056DB70` | `MapClass__ValidateBridgeZones` | `is_intact → 1` plus `AddBridgeZoneEdges` plus a `Can_Reach_Zone` check | bridge repaired |
| `0x005851B0` | `MapClass__AddBridgeZoneEdges` | Zone graph edges for an intact span | via `ValidateBridgeZones` |
| `0x00584E50` | `MapClass__RemoveBridgeZoneEdges` | Zone graph edges removed | via `InvalidateBridgeZones` |
| `0x00583180` | `MapClass__ResolvePathCoord_BridgeAware` | Projects a bridge cell onto the nearer endpoint, preserving the lane offset; ties pick endpoint B | `AStar_pathfind_search`, `EstimateZoneCost` — high |
| `0x00583820` | `MapClass__FindBridgeAdjacentZoneCell` | Walks off the deck and returns the first of six candidates matching the requested zone id | `EstimateZoneCost` |
| `0x00582D70` | `MapClass__RegisterBridgeOrTubeHierarchyPairs` | Registers bridge and tube pairs into the zone hierarchy | zone graph build/rebuild |
| `0x00587410` | `MapClass__FindBridgeConnection_Predicate` | Cursor-side bridge test | `InfantryClass__What_Action_*` — very high while hovering |
| `0x0056E990` | `MapClass__ToggleBridgePavement` | Pavement stamp/unstamp for a bridge cell | every ramp updater |
| `0x00575EE0` | `NotifyBridgeSpanCollapse` | Walks the endpoint-exclusive four-cell-wide span and fires trigger event `0x1F` through each cell's `Tag +0x3C` | 6 callers, all collapse/edge paths |
| `0x00578AC0` | `MapClass__IncrementBridgeCounter` | `++MapClass+0x1158`, the bridge-overlay draw-cache generation. Readers are the two bridge-branch sites in `CellClass__DrawOverlay_Body` | single caller `FUN_004F42F0`; frequency **UNCHECKED** |
| `0x00578100` | `MapClass__RecalcShroudEdgesAndEnqueueRedraw` | **NOT A BRIDGE FUNCTION — retracted 2026-08-19.** Body decompiled: two full-map cell iterations. Pass 1 takes cells with `+0x140` bit `0x20`, clears `+0x12C` bits `0x08`/`0x10` and `+0x140` bits `0x01`/`0x02`/`0x20`, sets the `+0x138` dirty byte, enqueues a tactical redraw via `FUN_006DA7D0`, and notifies on `+0x120 == -2`. Pass 2 recomputes every cell's shroud edge bitmask through `Shroud_EdgeBitmask_Calculator` into `+0x120` and enqueues a redraw on change. No bridge field, flag, or tileset appears anywhere in it — and bit `0x20` is not a bridge bit: `SetBridgeDirection`'s clear mask `0xFFFEE07F` preserves it. See §10 | per tick, but **not bridge work** |
| `0x00703B10` | `TechnoClass__IsOnBridge_ForFiring` | Own cell or an axis-matched neighbour carries `0x100`; the axis pairing is what proves bit `0x800` | `GetFireError` and draw — very high |
| `0x004865D0` | `CellClass__HasBridgeOverlay` | **Name understates it** — tests shore pieces, water, and the four RMG river-bridge tilesets. A prior session recorded this drift; re-read this session and the drift stands. Only RMG consumers | RMG only — dormant in skirmish |
| `0x0056A080` | `FUN_0056A080` | Bridge-family by callee set (calls both pavement span walkers, reads `g_BridgeSet_TileSetBase`) but **has zero xrefs** — unreachable in the current database | never |

---

## 7. Inventory — Tube (`BRIDGE_TUBE`)

`TubeClass` is the low-bridge under-pass: a scripted path that carries a ground unit *beneath*
a bridge deck. It is **not** the dormant Tiberian Sun subterranean locomotor, and `ENGINE.md`
explicitly separates the two.

Record layout, from the constructor, the INI reader and the CRC contributor, which agree:
`+0x24`/`+0x26` entry X/Y (i16), `+0x28`/`+0x2A` exit X/Y (i16), `+0x2C` direction (i32),
`+0x30`..`+0x1BC` direction path (100 × i32, `-1` terminated), `+0x1C0` path length.

| Address | Name | Purpose | Trigger / frequency | Live |
|---|---|---|---|---|
| `0x00727FD0` | `TubeClass__Constructor` | Builds the record, installs 4 vptrs, appends to `g_TubeArray`, writes the index into `cell+0x116` | `ReadTubesINI`, `CellClass__RecalcAttributes` — map load | yes |
| `0x007283C0` | `MapClass__ReadTubesINI` | Parses `[Tubes]`; field order is entry X, entry Y, **direction**, exit X, exit Y, then the path | `ScenarioClass__Full_Init` — once per map | yes |
| `0x00728280` | `MapClass__WriteTubesINI` | **Newly labelled.** Compacts `g_TubeArray` (dropping tubes whose cell no longer qualifies and renumbering the survivors) then emits the `[Tubes]` rows. It does **not** write `g_TubeCount` — see §2.6 | `Save_Scenario_Map_File` only | **editor path — never in skirmish** |
| `0x007359F0` | `UnitClass__TubeMovement` | Steps a unit through the path, interpolating Z between entry and exit ground heights over `+0x1C0` steps; on exit snaps to the exit cell centre, un-limbos, restores speed, and sets facing from `(record+0x2C << 13) − 0x8000` | `UnitClass__AI` — per tick per unit in a tube | yes |
| `0x00484AB0` | `CellClass__IsTubeCell` | `cell+0x116` in range **and** `cell+0xEC == 10` | every ground `Can_Enter_Cell` — very high | yes |
| `0x00484F20` | `CellClass__GetTubeAtCell` | Index → record; **does not** re-check LandType, unlike `IsTubeCell` | tube exit, cursor tests | yes |
| `0x007281A0` | `TubeClass__Load` | **Label corrected** — was `TubeClass__Save`. vtable slot 5 | save-game load | yes |
| `0x007281E0` | `TubeClass__Save` | **Label corrected** — was `TubeClass__Load`. vtable slot 6 | save-game write | yes |
| `0x007286D0` | `TubeClass__GetClassID` | **Label corrected** — was `TubeClass__AI`. vtable slot 3; writes CLSID `{0B4CA41C-B3A7-11D1-B457-006097C6A979}` | COM identity queries | yes |
| `0x00728630` | `TubeClass__Compute_CRC` | **Newly labelled.** vtable slot 13; contributes 105 CRC words per tube in a fixed order | lockstep checksum — every checksum frame | yes |
| `0x007286B0`, `0x007286C0`, `0x00728710` | *(no function boundary)* | `vtable__TubeClass` slots 12, 11 and 8 point here, but Ghidra holds no function at these addresses, so they cannot be decompiled. **Creating the boundaries was not authorized this session.** Recorded so the vtable inventory is complete | unknown | UNCHECKED |

**`TubeClass` has no per-tick AI.** The removed `TubeClass__AI` label was the direct cause of a
plausible-but-wrong model; tube traversal is driven entirely from `UnitClass__AI`.

---

## 8. Inventory — Random map generator (`BRIDGE_RMG`)

### 8.1 Dormancy verdict

**Dormant in stock YR skirmish.** Two reachability paths exist and neither is exercised by
ordinary play:

1. `ScenarioClass__Read_Scenario 0x00684620` sets `Scen->IsRandom` (`ScenarioClass+0x34BD`) only
   when the scenario filename's extension compares equal to the string at `0x0083DA88`, which
   `read_memory` shows to be **`.SED`** (that address has exactly one xref, `0x0068465C`, inside
   this function). Retail YR ships `.MPR` / `.YRM` / `.MAP` skirmish maps; no `.SED` file is
   involved in choosing a stock map. There is a **second gate** on the same path: `Generate` runs
   only if `FUN_00597A10(scenario_name)` also returns nonzero.
2. `ChooseMap__AcceptRandomMapSetup 0x005E8590` → `RandomMapSetupDialog__Run 0x00595BC0` →
   `RandomMapSetupDialog__Proc 0x00596300` → `Generate`. The caller of `AcceptRandomMapSetup`
   sits at `0x005E6A11`, inside a code region for which Ghidra holds no function boundary.

**What is verified:** the `.SED` gate, the string bytes, and both call chains.
**What is UNCHECKED:** whether the retail YR skirmish map dialog exposes a random-map button at
all. That is a UI observation, not a binary one, and it was not made this session. Treat the
family as dormant-pending-observation rather than proven-dead.

RMG is the only part of the bridge family that consumes RNG at map-generation scale. The
existing plate comment on `0x0058EF10` (a prior session, 2026-07-25) enumerates 21 draw sites in
that subtree and is left untouched.

### 8.2 Members

| Address | Name | Purpose |
|---|---|---|
| `0x0058EF10` | `RandomMapGenerator__BridgeAndConnectorPass` | MapType 3/4 connector-plus-bridge pass; rebuilds regions then runs the connector loop |
| `0x005905D0` | `RmgRegion__CarveConnectorsOrBridges` | Per region: ramps on land, low-bridge decks on water |
| `0x0058F2C0` | `RandomMapGenerator__PlaceLowBridgeDeck` | Places a low-bridge deck; 5 RNG sites per the prior report |
| `0x005902C0` | `RandomMapGenerator__ValidateLowBridgeDeckArea` | Area precondition for the deck |
| `0x005A7440` | `RandomMapGenerator__IsUniformLevelBridgeEndArea` | End-area level uniformity test |
| `0x005904B0` | `RandomMapGenerator__PlaceBridgeRepairHut` | Places the CABHUT for a generated bridge |
| `0x0059E740` | `RandomMapGenerator__BuildRiverBridge` | River crossing; heading switch over the four river-bridge tilesets |
| `0x0057A0C0` | `MapClass__RmgFinalizeWaterShore` | Shore/water finalize; drives `UpdateBridgeTile_Low` and `SelectBridgeTileVariant_Low` |
| `0x00590FD0` | `RmgRegion__BuildRampOrientationMask` | Ramp orientation mask |
| `0x005910F0` | `RmgRegion__CarveStraightRamp_ClearSouth` | Straight ramp carve |
| `0x00591740` | `RmgRegion__CarveStraightRamp_ClearEast` | Straight ramp carve |
| `0x00591D80` | `RmgRegion__CarveStraightRamp_ClearNorth` | Straight ramp carve |
| `0x00592440` | `RmgRegion__CarveStraightRamp_ClearWest` | Straight ramp carve |
| `0x00593030` | `RmgRegion__CarveCornerRamp_Diagonal` | Corner ramp carve |
| `0x00593550` | `RmgRegion__CarveCornerRamp_Reflected` | Corner ramp carve |
| `0x004865D0` | `CellClass__HasBridgeOverlay` | Shore/water/river-bridge tileset predicate; RMG is its only consumer |

---

## 9. Label corrections and new labels applied

All applied with `save_program` plus readback after each mutation, per
`docs/research/ghidra-workflow.md`.

| Address | Prior name | New name | Proof |
|---|---|---|---|
| `0x007286D0` | `TubeClass__AI` | `TubeClass__GetClassID` | `vtable__TubeClass` slot 3 in an IPersistStream layout whose slots 0/1/2/4/7 are the named `AbstractClass` QueryInterface/AddRef/Release/IsDirty/GetSizeMax; body returns `E_POINTER` on null and writes a 16-byte CLSID |
| `0x007281A0` | `TubeClass__Save` | `TubeClass__Load` | Calls `AbstractClass__Load` (reads via IStream vtable `+0x0C`, registers the swizzle token) and re-installs the four vptrs — the post-deserialisation fixup. Slot 5 |
| `0x007281E0` | `TubeClass__Load` | `TubeClass__Save` | Calls `AbstractClass__Save` (writes via IStream vtable `+0x10`, clears the dirty byte). Slot 6 |
| `0x00728630` | `FUN_00728630` | `TubeClass__Compute_CRC` | Calls `AbstractClass__ComputeCRC` then feeds exactly the tube field set that `ReadTubesINI` writes |
| `0x00728280` | `FUN_00728280` | `MapClass__WriteTubesINI` | Emits `[Tubes]` rows via `INIClass__PutString` after compacting `g_TubeArray`; sole caller `Save_Scenario_Map_File` |
| `0x00568E40` | `FUN_00568E40` | `MapClass__BridgePavementSpanWalker_High` | Same `(cell, dir, out_rect)` shape and caller position as `0x00569760`, but every tileset comparison is against `g_BridgeSet_TileSetBase` and it applies the `+4` deck rise |
| `0x00703E70` | `FUN_00703E70` | `TechnoClass__CountAdjacentBridgeDeckTiles` | Receiver shared with `IsOnBridge_ForFiring`; samples three neighbour cells against the high bridge tileset range 7..16. Exact 0/1/2 semantics recorded in the plate comment because the name rounds off |

Prior names are preserved verbatim in each function's plate comment.

---

## 10. Names this map does not trust

Left un-renamed on purpose: a guess is worse than `FUN_*`.

| Address | Current name | Why it is suspect |
|---|---|---|
| `0x00543F10` | `BridgeShadowTable_StaticInit_00543F10` | CRT static initialiser (only reference is the init table at `0x00813730`), no callers, and nothing in it was bound to bridges. The bridge tileset bases it would need to touch are written by `Read_Theater_TileSets_INI` instead. **Left untagged.** |
| `0x00544691` | `BridgeSlopeTable_StaticInit_00544691` | Zero-fills 16-byte records across `0x00ABC210`..`0x00ABC2AC` with four non-zero values. The nearby addresses that *are* bridge-bound (`0x00ABC2B4`, `0x00ABC1E8`, `0x00ABC1D0`) lie outside the block it writes. **Left untagged.** |
| `0x004AF470` | `DriveLocomotionClass__ComputeBridgeRenderOffset` | Stores an **angle** — `atan(2·g_DriveHeightStep / _DAT_008A0778)` into `_DAT_008A07A0` — not a render offset, and no consumer was bound to bridge code. Its neighbour `0x004AF4A0` is genuinely bridge-bound; adjacency is not evidence. **Left untagged.** |
| `0x004865D0` | `CellClass__HasBridgeOverlay` | A prior session's drift note stands: the predicate covers shore pieces, water, and four RMG river-bridge tilesets. Tagged `BRIDGE_RMG` only |
| `0x00578D80` | `IsOnBridgeRamp` | Tests the cliff tileset and the four waterfall tilesets; no bridge tileset appears in the body. Kept because its single caller sits inside `ComputeBridgeSurfaceMask`, which *is* bridge code |
| `0x0047E040` / `0x0047E470` | `..._NESW` / `..._NWSE` | A prior session verified the two bodies byte-identical, so the compass suffixes cannot come from the code. What *is* verified is the caller split: `0x0047E040` is called only from High-family sites, `0x0047E470` only from Low-family sites |
| `0x00578100` | `MapClass__RecalcShroudEdgesAndEnqueueRedraw` | **Body decompiled 2026-08-19: the `Bridge` in the name is wrong, and the family has no per-tick member at all.** It is a whole-map shroud-edge recalculation that enqueues tactical redraws — `Shroud_EdgeBitmask_Calculator` into `cell+0x120`, dirty byte `+0x138`, redraw queue via `FUN_006DA7D0`. Candidate name `MapClass__RecalcShroudEdgesAndEnqueueRedraw`; **not applied — Ghidra is read-only under the current goal.** The `BRIDGE_HIGH`/`BRIDGE_LOW` tags applied to it earlier in the session are wrong and should be removed when a sync is next authorised. Its second caller `FUN_006E1A70` is moot for this family |

---

## 11. Residuals

- **Retracted:** the earlier residual about `FUN_006E1A70` is moot. `0x00578100` is not bridge
  code (§10), so neither of its callers belongs to this family and the family has no per-tick
  member. The consequence for ranking: nothing in Bridge/Tube/Ramp runs every tick, so the
  visibility×frequency ordering is driven by the event members (destroy, repair, unit crossing)
  and by the per-frame draw member `0x0073B140`.
- **Frequency of `MapClass__IncrementBridgeCounter`** is unknown — its single caller
  `FUN_004F42F0` was not traced to a trigger.
- **`FUN_00597A10`**, the second gate in front of `RandomMapGenerator__Generate`, was not opened.
- **`ShipLocomotionClass__Compute_BridgeZOffset 0x0069EBB0`** has no CALL xrefs and its body was
  not decompiled; its relationship to `g_BridgeZ_Offset` / `g_BridgeZOffset_Teleport` is
  unestablished.
- **Three `vtable__TubeClass` slots** (`0x007286B0`, `0x007286C0`, `0x00728710`) have no function
  boundary. Creating boundaries needs separate authorization and was not done.
- **`FUN_0056A080`** is bridge code with zero xrefs. Whether that is genuine dead code or a lost
  indirect reference was not resolved; it needs a byte-pattern search for the address.
- **RMG skirmish exposure** needs a live observation of the retail map-choose dialog, not more
  decompilation.
- **`Read_Theater_TileSets_INI`** is the producer of every bridge tileset base and was identified
  but not mapped; it belongs to the theater family, not this one.
- **Plate comments** were written for the members verified in this session and for every drift
  finding. Members that already carried a dated verified plate comment from a prior session were
  left untouched rather than overwritten; the remainder carry tags and this document only. Two
  plate comments written earlier in this session contained errors and were corrected in place —
  see §12.

## 12. Adversarial re-derivation pass

A fresh read-only critic re-derived twelve sampled claims from the binary without access to this
session's reasoning. Ten were confirmed. Six defects were raised and **all six were reproduced
independently before being accepted**; every one is fixed above, and the two that had already been
written into the Ghidra database were corrected there as well (`0x00728280` and `0x00486750`, each
re-saved and read back).

| Defect | Where it was | Fix |
|---|---|---|
| `MapClass__WriteTubesINI` claimed to decrement `g_TubeCount`. `get_xrefs_to 0x008B4148` shows three READs and no WRITE in that function | §2.6 **and the applied plate comment** | Corrected in both; the plate now records the mistake and the real writers |
| "**Every** High-family body … / **every** Low-family body …" — falsified by `0x0057CCF0`, `0x0057BAA0` and essentially all of §4.2/§4.3/§5.2/§5.3, which reference no tileset base and split on the overlay ordinal instead | §1, contradicting §2.3 two sections later | §1 rewritten to separate tileset-keyed from overlay-keyed members |
| `g_WoodBridgeSet_TileSetBase`'s writers given as the `g_BridgeSet_TileSetBase` triple | §1 **and the applied plate comment on `0x00486750`** | Both corrected to `0x00545A86` / `0x00545DEA` / `0x00546CB9` |
| `MapClass__RecalcShroudEdgesAndEnqueueRedraw` has a second caller, `FUN_006E1A70`, absent from the document | §3, §6, §11 | Added to all three; the "only per-tick member" claim narrowed |
| §2.2's per-bit "where written" column wrong for `0x200`, `0x1000` and `0x10000`, and `0xFFFEE07F` presented as the function's uniform mask when it is the first write's mask | §2.2 | Table rebuilt per write, with the clear-but-never-set cases called out |
| `0x0400` given as `(param_3 == 0) << 10` when the test is on the low byte | §2.2 | Corrected, with the asymmetry against `0x0800` noted |

The critic's own list is not treated as authority either: each defect above was re-checked against
`get_xrefs_to 0x008B4148`, `get_xrefs_to 0x00ABAD1C`, and the six flag-write expressions in
`0x0047E040` before the document was changed.

## 13. Ghidra synchronisation applied 2026-08-19

Applied on direct user request, after the comparison work. Each met the label bar (exact
boundary, verified behaviour, active caller binding) and each kept its prior name in the plate:

| Address | Change |
|---|---|
| `0x00578100` | Renamed `MapClass__RecalcBridgeShroudFlags` -> `MapClass__RecalcShroudEdgesAndEnqueueRedraw`; **`BRIDGE_HIGH` and `BRIDGE_LOW` tags removed** (both were applied earlier in the same session and were wrong). Not bridge code |
| `0x00570050` | Renamed `ProcessBridgeDestruction_Low` -> `MapClass__RepairBridgeOrRestoreRamp_Low`. The old name said the opposite of what it does |
| `0x00573540` | Renamed `ProcessBridgeDestruction_High` -> `MapClass__RepairBridgeOrRestoreRamp_High`, same reason |
| `0x00728630` | Plate corrected: 106 CRC words per tube, not 105 |
| `0x004DDC40` | Plate added: vtable slot `+0xBC` established from five constructors, live caller `FootClass::PerCellProcess` `0x004D8BE6`, and the callsite-vs-callee trap that hid it |

Tag counts after: `BRIDGE_HIGH` 59, `BRIDGE_LOW` 65. The family is now 121 distinct members.
`save_program` ran after each function and every edit was read back.

## 14. Incidental database action

One call to `disassemble_bytes 0x005E69E0 length=64` was made while chasing the RMG caller chain.
That tool disassembles rather than only reading, so it is a database mutation. The region is
plainly existing code — it ends in a `POP/POP/POP/XOR EAX,EAX/POP/ADD ESP/RET 0x10` epilogue and
was already reachable as instructions — and no function, label, or reference was created;
`get_function_by_address 0x005E6A11` still reports no function. Recorded rather than omitted. The
tool was not used again.
