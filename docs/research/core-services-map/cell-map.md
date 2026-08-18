# cell-map — CellClass / MapClass (cell-grid owner + spatial-query service)

> Service profile for the core-services graph. Long evidence base lives in
> `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (CLOSED archive, Ghidra-verified,
> 2026-06-10 last refresh). This profile distills its cross-service edges; addresses cited inline are from
> that study. Default verdict on any unproven difference is DRIFT/UNCHECKED.

## Purpose

The **spatial substrate** — the geometry the per-tick spine reads, the analogue of what `logicclass`
is to time. Two cooperating native roles:

1. **MapClass = cell-grid OWNER + spatial-query SERVICE.** One global object (`g_Map @ 0x0087F7E8`,
   base of the fused `GScreen→MapClass→DisplayClass→RadarClass→PowerClass→SidebarClass→…` ~21,868-B
   mega-object) owning a fixed **512×512 = 262,144** `CellClass*` array (`g_CellArray_Base @
   0x0087F924`), the canonical packed-coord lookup (`Get_CellClass 0x005657A0`, `index = y*0x200+x`,
   never-null dummy fallback `&DAT_00ABDC50`), playfield-containment (diamond `IsCellInPlayfield
   0x00578460`, 4-corner `IsRectInPlayfield 0x00578390`), the per-MovementZone reachability graph
   (zones/floodfill), bridge geometry/state, shroud, and crate-regen. **Passive geometry the tick
   reads** — NOT a per-tick object scheduler.

2. **CellClass = per-cell DATA RECORD.** A 328-byte (0x148) struct (ctor `0x0047bbf0`) holding
   everything about one cell: terrain (LandType `+0xEC` / ZoneType `+0x4C` / Level `+0x11B` / Slope
   `+0x11C` / Height), overlay+smudge, two object linked-lists (`FirstObject +0xE4` /
   `AltObject +0xE8`), two occupancy bitfields (`+0x124` ground / `+0x128` bridge), per-house
   visibility/sensor/reservation bitmasks (`+0x78`/`+0x7C`/`+0xAC`/`+0xDC`), bridge flags (`+0x140`),
   shroud/fog (`+0x12C`), radar color, render Z-adjust, and the per-cell radiation field
   (`RadLevel +0xF0` / `RadSite +0xF8`). Every gameplay system reads/writes it.

**Not the spec:** do NOT port the 328-B struct, the fused GScreen→Sidebar hierarchy, or COM/vtable
plumbing. The Rust port fragments one CellClass across 9+ grids (`ResolvedTerrainGrid`, `OverlayGrid`,
`SmudgeGrid`, `OccupancyGrid`, `BridgeRuntimeState`, `PathGrid`, `TerrainCostGrid×SpeedType`,
`ZoneGrid`, `FogState` + side-maps); the split is largely intended.

## Owns

State/globals/structs owned by this service (addresses from the study):

- **`g_Map` singleton** `0x0087F7E8` — the one MapClass instance (ctor `0x00565090`).
- **`g_CellArray_Base`** `0x0087F924` (= singleton `+0x13C`) — `CellClass*[262144]`, hot-path index
  `[y*0x200+x]`. Alloc by `MapClass__Init_Alloc` (vslot 5) `0x00565800` (width `+0x14C=0x200`, height
  `+0x150=0x200`, total `+0x154=0x40000`).
- **Dummy sentinel cell** `DAT_00ABDC50` (+ probed-coord store `DAT_00ABDC74`) — never-null lookup
  fallback.
- **Playfield diamond fields** on the singleton — `+0xF4 = Size.width`, `+0xFC/+0x100/+0x104/+0x108 =
  LocalSize.{left,top,width,height}` (set verbatim by map loader `0x004ad76b`; iso transform is in the
  consumer `0x00578460`).
- **Bridge/tube record DynamicVector** `MapClass+0x50..0x64` (vtable `0x007ED4C0`; 0x10-B records:
  endpoint coords, intact `+0x8`, kind `+0xC` 0=bridge/1=tube; appended by `ComputeBridgeZones
  0x0056D6E0`).
- **Zone arrays** — `+0x68` (4-B/zone-cell: zone, level, cluster-u16), `+0x6C = (w+1+h)²` bound,
  `+0x70` (10-B/zone-cell pathfinder, byte+8 = level); per-MZ zone graphs `zone_ids[13]`. Compact
  ZoneMap mirror arrays `DAT_0087f850` (Level) / `DAT_0087f858` (ZoneType), runtime-allocated.
- **CellIterator state** `+0x10C/+0x110/+0x114/+0x118` = X / Y / diagonal-remaining / cell-slot ptr
  (writers `0x00578350/0x00578290`).
- **`g_PassabilityMatrix`** `0x0082A594` — 13 MovementZone × 8 reduced-ZoneType `int[13][8]` (verified
  416-B dump; only value `1` passes). **Note:** the per-cell terrain-legality table
  `g_SpeedType_LandType_Table 0x0089EA40` (speed×land) is a SEPARATE table, often colocated in the
  `cell-validation` / `lookup-tables` story but read by `CheckCellPassability`.
- **Per-cell record (CellClass) fields** — all `+0x24..+0x144` listed above; canonical producers
  `RecalcAttributes 0x0047d2b0` (LandType/Slope/Level/Zone + ZoneMap mirror + neighbor `Flags|=0x10000`)
  and `RecalcZoneType 0x00483C80` (`+0x4C`); list maintainers `AddContent 0x0047E8A0` /
  `RemoveContent 0x0047EA90`; occupancy `Mark_/Clear_Occupation 0x007441B0/0x00744210`.
- **Per-cell radiation field** `+0xF0` RadLevel / `+0xF8` RadSite ptr (writers `0x00487CE0/0x00487D00`;
  site setter `FUN_00487C70`).

Rust homes: `ResolvedTerrainGrid` (`map/resolved_terrain.rs`), `OverlayGrid`/`SmudgeGrid`,
`OccupancyGrid` (`sim/occupancy.rs`), `BridgeRuntimeState`, `PathGrid`/`TerrainCostGrid`/`ZoneGrid`
(`sim/pathfinding/*`), `FogState` (`sim/vision/`), `RadiationState` (`sim/radiation.rs`),
`cell_rect.rs` validator facade, lifecycle in `sim/world/mod.rs`.

## Key functions & globals (addresses)

| Symbol | Addr | Role |
|---|---|---|
| `Get_CellClass` | `0x005657A0` | canonical packed-coord→cell, `y*0x200+x`, never-null dummy |
| `Get_CellClass_At_Coord` | `0x00565730` | lepton-input cousin (sign-correct `>>8`) |
| `MapClass ctor` / `Init_Alloc` | `0x00565090` / `0x00565800` | singleton init / grid alloc |
| `IsCellInPlayfield` / `IsRectInPlayfield` / `IsCoordsInPlayfield` | `0x00578460` / `0x00578390` / `0x005785F0` | diamond + 4-corner containment |
| `InitCellAttributes` | `0x00568bb0` | map-init / post-load full cell rebuild |
| `RecalcAttributes` | `0x0047d2b0` | per-cell terrain producer (38 callers); CliffBackImpassability reclass |
| `RecalcZoneType` | `0x00483C80` | writes reduced ZoneType `+0x4C` |
| `AddContent` / `RemoveContent` | `0x0047E8A0` / `0x0047EA90` | object-list membership (caller-passed layer) |
| `EnterCell/ExitCell_MultiCells` | `0x005683C0` / `0x005687F0` | sole Add/RemoveContent callers (foundation iterate) |
| `Mark_/Clear_Occupation` | `0x007441B0` / `0x00744210` | occupancy bits `+0x124`/`+0x128` |
| `GetZoneID` / `UpdateBridgeZonesHelper` / `FloodFillReachableZones` | `0x0056D230` / `0x0056C510` / `0x005840C0` | zone reachability graph |
| `CheckCellPassability` / `CellRect__CheckPassability` / `CellRect__CheckOccupancy` | `0x004834a0` / `0x0056E7C0` / `0x00586780` | validators |
| `UnitClass::Can_Enter_Cell` / `CheckBridgeTraversal` | `0x0073F0A0` / `0x004D9C60` | per-class entry decision |
| `GetEffectiveHeight` (cell) / object variant | `0x00487d50` / `0x005F5F00` | `Level + 4 iff Flags&0x80` / `OnBridge+0x8C` |
| `RecalcBridgeShroudFlags` / `UpdateCrateRegenTimers` | `0x00578100` / `0x0056BBE0` | the two unconditional per-tick calls received |
| `RevealShroud` | `0x005673A0` | shroud reveal `+0x12C` bits 3/4 |
| `PlaceTiberium` / `Reduce_Tiberium` / `SpreadTiberium` | `0x00487190` / `0x00480A80` / `0x00483780` | ore cell lifecycle |
| `GetRadarColor` / `Cell_ComputeZAdjust` | `0x0047C060` / `0x00484680` | minimap / render Z (no fog/shroud darkening branch) |
| Radiation: `SetCellRadLevels` / `RadSiteClass::AI` / Detonate site-create | `0x0065B9C0` / `0x0065B800` / `0x004690B0` | per-cell rad spread/decay/creation |

## Tick / render position

**Not in the per-tick scheduler — it is the spatial substrate the spine reads.** The temporal spine is
`LogicClass::PerTickUpdate 0x0055AFB0`, which calls *into* MapClass exactly **twice unconditionally per
loop** (verified LIVE-0610 in the caller):

- `RecalcBridgeShroudFlags 0x00578100` on `(int)frame(0x00A8ED84) % 0x78 == 0` (every 120 frames; signed
  modulo in caller `0x0055b294-0x0055b2ad`, body has no gate).
- `UpdateCrateRegenTimers 0x0056BBE0` every tick (`0x0055b655`; body double-gated `0xa8b238 && 0xa8b261`,
  active only with Crates=yes).

Plus **7 gated map-singleton call sites** in the same driver: shroud-regrow full-grid sweep `0x004ACAC0`
(active-capable in YR), TS-fog-regrow `0x004ACBC0` (SpecialFlags&0x1000, dormant), ZAdjust sweep
`0x004AE4C0` (only while LightningStorm/PsychicDominator active), bridge counter `0x004F42F0`, read-only
getter `0x004AEB10`, and the RadSiteClass backward-vector AI loop `0x0055b5cd` (decay, right after
LightningStorm). The MapClass 30-slot vtable has **no AI/Update driver slot**.

Within the project's documented `World::advance_tick` order, this substrate is queried at: commands,
ground movement, air/special movement, **vision**, turrets+combat (radiation damage Phase 3.5),
scatter/production/ore-growth (ore cell lifecycle), and is rebuilt at map-load / post-load
(`InitCellAttributes` analogue). Rust lifecycle chokepoint = `reveal/conceal/uninit` +
`flush_pending_delete` at the tail of `run_late_region`.

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence.

- **logicclass** — via `RecalcBridgeShroudFlags 0x00578100` + `UpdateCrateRegenTimers 0x0056BBE0` (+ the
  7 gated singleton calls). *Direction note:* this is the reverse leg of logicclass→cell-map; it is the
  cadence coupling, and the RadSiteClass AI decay loop is driven by the logic spine. The substrate is the
  callee here, so the primary edge is incoming (see Used-by); recorded as a depends-on only insofar as
  cell-map's bridge-shroud/crate/rad-decay state advances *because* logicclass invokes it each tick.

- **random-scenario** — via `Scen->Random (ScenarioClass+0x218)` consumed by tiberium logic:
  germinate-variant `0x004871F4/0x00487252`, spread-direction `0x0048382A`, spread/growth queue-jitter
  `0x00722B5B` (PlaceTiberium/SpreadTiberium). Verified LIVE-0531 each callsite loads `[0x00a8b230]+0x218`
  as the RNG receiver — NOT `g_MainRng @ 0x00886b88`. (C-RNG #15.) Also reads `ScenarioClass+0x1258`
  theater index in `RecalcZoneType` (SNOW vs Temperate OccupationBits), `+0x34A6` TiberiumGrowthEnabled
  gate, and `Cell_ComputeZAdjust` gates on Scenario LightningStorm/PsychicDominator timers.

- **rules-class** — via `RulesClass+0x664` `[General] CliffBackImpassability` read in `RecalcAttributes`
  (string `0x0083c8cc` → `CCINIClass::ReadInt 0x005276d0` → store `0x0066f1e6`; default 2, active YR);
  `[Radiation]` block parsed into `RulesClass+0x1804..0x1834` (RadDurationMultiple/RadApplicationDelay/
  RadLevelMax/RadLevelFactor/RadColor/RadSiteWarhead) consumed by the radiation cell service;
  `[General] GapRadius` read for gap-gen vision suppression. Many cell attribute derivations read parsed
  INI tunables.

- **ini-parsing** — via `CCINIClass::ReadInt 0x005276d0` (CliffBackImpassability), `INIClass::ReadRect
  0x00527cc0` (`[Map] LocalSize` → diamond fields, map loader `0x004ad76b`), `[Radiation]` parser
  `0x0066CF90`-area. The cell/map substrate reads INI accessors at load to seed terrain/playfield/rad
  rules. (At the Rust layer this is the ruleset/map-load path, not a per-tick edge.)

- **damage-helpers** — via the radiation damage path: `FootClass::AI 0x004DA530` reads cell `RadLevel`
  (`0x00487CB0`) and applies `ReceiveDamage` with `RadSiteWarhead` every `frame % RadApplicationDelay`.
  The per-cell rad field is the *input* to the warhead/armor kernel here; the immunity gate
  `ImmuneToRadiation (TechnoType+0xD37)` and Verses scaling are the damage-helper side. Rust: combat tick
  Phase 3.5 + `combat/damage/gates.rs:32-34`.

- **abstract-object** — via the object linked lists `FirstObject +0xE4` / `AltObject +0xE8`: cells store
  `ObjectClass*` heads and AddContent/RemoveContent traverse the object `+0x30` next-link;
  `AbstractClass__Constructor_Full` / `AssignUniqueID` are called inside the CellClass ctor (cell is an
  AbstractClass subtype). BlowUpBridge walks the list and invokes object vtable slots (`+0x16c` C4-kill,
  `+0xEC` DropIn). So cell-map both *is-a* AbstractClass and *holds* ObjectClass pointers.

- **techno-foot / mission-radio** — via `AddContent`'s coupled `DiscoverByHouse` (object vtable `+0x198`)
  on insertion into a shrouded cell; `Mark_Occupation` driven by Techno/Foot Enter/Exit; the
  object-on-cell `Get_Effective_Height 0x005F5F00` gated on the object's `OnBridge+0x8C`; radiation
  damage targets `FootClass` occupants only (buildings never). These are the locomotion/mission consumers
  reaching back into the cell — primarily incoming (Used-by), but cell-map calls object/techno vtable
  slots during list maintenance and bridge collapse, creating a bidirectional coupling. *(Direction edge
  to techno-foot recorded under Used-by as the dominant direction.)*

- **bridge-helpers** — via `FindBridgeRecord 0x0056DA10`, `CheckBridgeTraversal 0x004D9C60`,
  `BlowUpBridge 0x0047DD70`, `DropIn 0x005F4160`, and the `Flags +0x140` bridge bits (0x100 structural /
  0x400 destroyed / 0x80 has-overlay-+4 / 0x100000/0x200000 zones). GetZoneID consults bridge records;
  cell-validation/passability tests branch on bridge gates. The bridge predicates/offsets are a tightly
  coupled helper family operating on cell records. Rust: `sim/bridge_state/`, `map/bridge_topology.rs`
  (deck height 2 levels = 208 leptons, `DAT_00AC13BC`).

- **lookup-tables** — via `g_PassabilityMatrix 0x0082A594` (cell-map owns/reads it for zone legality) and
  `g_SpeedType_LandType_Table 0x0089EA40` (terrain-entry legality in `CheckCellPassability`), plus
  `g_DirectionOffsets 0x0089F688` (8-dir cell deltas, initializer `0x0049F2F0`) for neighbor stepping in
  zone floodfill / RecalcAttributes neighbor scan. Static read-only tables consumed during spatial
  queries. *(If lookup-tables is not a distinct node, these fold into cell-map's own Owns + cell-validation.)*

## Used-by (incoming edges)

Who depends on cell-map (the substrate every other system queries):

- **logicclass** — calls `RecalcBridgeShroudFlags` + `UpdateCrateRegenTimers` unconditionally every tick,
  the 7 gated map-singleton sweeps, and drives the RadSiteClass decay loop. The temporal spine's only
  coupling to the spatial substrate. (via `0x0055AFB0` → MapClass thiscalls.)

- **pathfinding-helpers** — the heaviest consumer: A* reads cell occupancy/blocker `+0x122`, zone graph
  via `GetZoneID 0x0056D230`, the passability matrix, `FloodFillReachableZones 0x005840C0` (sole caller
  `PathfinderClass__UpdateHierarchicalEdges 0x0042ccd0`), `Get_Slope_Cost_At_Cell 0x0056BCD0`, effective
  height. Rust `PathGrid`/`ZoneGrid`/`TerrainCostGrid` are rebuilt FROM the cell substrate.

- **cell-validation** — `CheckCellPassability 0x004834a0`, `CellRect__CheckPassability 0x0056E7C0`,
  `CellRect__CheckOccupancy 0x00586780` read terrain/zone/height/occupation-byte + reservation `+0xDC` +
  list blockers + final `IsRectInPlayfield`. Two separate validator surfaces over the cell record. Rust:
  `cell_rect.rs`.

- **techno-foot** — `UnitClass::Can_Enter_Cell 0x0073F0A0` and FootClass locomotion read object-list
  layer + occupancy bits + effective height; `EnterCell/ExitCell_MultiCells` (the sole AddContent callers)
  are driven by Techno/Foot movement; `FootClass::AI 0x004DA530` reads the rad field. Locomotion is gated
  entirely on cell-map queries.

- **mission-radio** — missions read cell occupancy/passability for placement, scatter, dock approach;
  deployed-Desolator mission step `FUN_00521320` reads `GetCurrentRadLevel 0x0065B510` to re-fire.

- **factory-house** — building placement/sell route through `EnterCell/ExitCell` + foundation occupancy +
  `+0xDC` AI base-placement reservation (reader `FUN_0050b760`, `houseIdx = HouseClass+0x30`); the
  CanHideThings hidden-occupancy counter `+0x100`; per-house visibility masks `+0x78`. Rust: production
  spawn/sell route through `uninit`.

- **target-scoring** — threat/target acquisition scans cells for occupants via the object lists +
  visibility masks (`IsVisibleToHouse 0x004870b0`).

- **drawing-helpers** — `GetRadarColor 0x0047C060`, `Cell_ComputeZAdjust 0x00484680`, LightConvert `+0x34`
  (`FUN_00483e30`), bridge-overlay draw-dedup cache `+0x64..0x77`, PixelFX sparkle `+0xFC`. Render reads
  cell record fields. (Rust render-only grids: `CellLightGrid`, `TerrainGrid`.)

- **damage-helpers** — reads the per-cell `RadLevel` as damage input (see Depends-on; bidirectional).

- **abstract-object / random-scenario** — abstract-object lifecycle (reveal/conceal/uninit) drives cell
  list membership; tiberium consumes Scen->Random (bidirectional with random-scenario).

## Open / unverified edges

- **lookup-tables boundary** — `g_PassabilityMatrix` / `g_SpeedType_LandType_Table` /
  `g_DirectionOffsets` could be modeled as cell-map-owned data or as a separate `lookup-tables` node;
  recorded as a depends-on edge but the ownership split is a graph-modeling choice, not a binary fact.
- **`+0xDC` reservation-on-intent** — no SETTER exists in reachable code (both readers are AI
  base-placement, out of scope per `feedback_no_ai_yet`); the reserve-on-intent contract (C-RECORD #10)
  is UNMODELED in Rust — two vehicles can path to one cell. Edge to cell-validation/pathfinding is
  partial.
- **Two FootClass rad-damage gates** (`vtbl+0x54()==0`, `this+0x81==0`) — identities INFERRED (likely
  in-air/limbo), not pinned; affects the exact damage-helpers edge condition.
- **shroud-regrow sweep `0x004ACAC0`** — INI keys behind `Rules+0x17F0/+0x1640` not pinned; the
  logicclass→cell-map gated edge condition is partially unverified.
- **Dummy cell `DAT_00ABDC50` field table** — UNVERIFIABLE-static (process not running); the
  "bridge flags clear on dummy" contract is asserted, unproven.
- **A* bridge-layer hard-block set** — Rust constructs it empty (`bump_crush.rs:126`); whether gamemd's
  per-neighbor classification blocks bridge-deck occupants is unproven (open §4.2 #5 sub-drift).
- **RecalcZoneType building sub-branches** (FirestormWall/LaserFence) — DORMANT in stock YR (keys unset);
  no live edge to factory-house through them.
