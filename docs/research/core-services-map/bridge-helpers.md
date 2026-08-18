# Core Service Profile — Bridge Helpers (`bridge-helpers`)

**Role:** A bridge-topology READ service over the cell substrate. A small family of mostly-stateless predicate/offset primitives (`IsBridge`, `IsWoodBridge`, `IsLowBridgeCell`, `GetEffectiveHeight`, the `0x80/0x100/0x200` flag tests, `CheckBridgeTraversal`, the AoE impact-Z layer selector, `Get_Draw_Offset`'s bridge branch, the dual object-list/occupancy-bit selectors) that movement, combat-AoE, pathfinding, occupancy, and render all read.

**Primary doc:** `docs/research/BRIDGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, 2026-06-04; four load-bearing primitives re-decompiled live this session). This profile only maps the cross-service graph edges; the long contract (C1–C18) lives in that doc.

---

## Purpose

Answer "what is at this cell, bridge-wise, and may a mover legally cross this height boundary?" without owning any new state. It is a pure read/predicate layer that classifies cells from the existing cell-substrate fields (flags, level, land-type, tube-index, iso-tile-index) and exposes:
- structural/bridgehead/anchor bit predicates,
- effective deck-surface height,
- tileset-based bridge classification (concrete + wood) for the zone/path graph,
- low-bridge/tube classification,
- the traversal-legality gate (`CheckBridgeTraversal`),
- the AoE detonation object-layer selector (ground vs deck),
- the occupancy-bit/list-layer selector (via OnBridge),
- a render-only draw-offset query.

It is NOT the bridge damage/collapse/repair state machine — that is a separate study. It is the set of predicates those systems READ.

## Owns

This service is read-only; it owns **semantics**, not storage. The backing store is `CellClass` (owned by `cell-map`). What it owns conceptually:

- **The bit-layout contract of `CellClass+0x140` flags:** `0x80` anchor, `0x100` structural-bridge, `0x200` bridgehead, `0x400` destroyed/ramp, `0x800` direction-zero, `0x1000`/`0x10000` state-driven, `0x2000` pavement.
- **The effective-height formula:** `level(+0x11B, signed i8) + ((flags>>7)&1)*4`.
- **The low-bridge/tube predicate:** `TubeIndex(+0x116) ∈ [0,g_TubeCount) AND LandType(+0xEC)==10`.
- **The traversal gate's full state machine** (C7–C11): parent-`0` reconstruction, `dir==-1` candidate seed, diff-{0,1,4} slope/bridgehead checks, the in/out height seed and list-byte selector.
- **The AoE impact-Z layer threshold:** `impact_z > ground_z + DAT_0089E864/2` (strict; `DAT_0089E864 = round(per-level-height×4)` = full 4-level deck height, an engine iso-geometry constant).

No globals are *written* by this service. Globals it reads (owned elsewhere): `g_BridgeSet_TileSetBase`, `g_WoodBridgeSet_TileSetBase`, `g_TubeCount`, `g_DirectionOffsets @ 0x0089F688`, the three deck-height threshold copies `DAT_0089E864`/`DAT_00B1D0AC`/`DAT_00AC13BC`.

## Key functions & globals (addresses)

**Predicate primitives (live-verified 2026-06-04):**
- `CellClass::GetEffectiveHeight @ 0x00487D50` — `Level + ((Flags>>7)&1)*4` (C4).
- `CellClass::IsBridge` (tileset) `@ 0x00486750` — `g_BridgeSet_TileSetBase != -1 AND iso_tile_index ∈ [base, base+0x10)` (C5).
- `CellClass::IsWoodBridge` (tileset) `@ 0x00486770` — second tileset window over `g_WoodBridgeSet_TileSetBase` (C5).
- `CellClass::IsLowBridgeCell/IsTubeCell @ 0x00484AB0` — `TubeIndex valid AND LandType==10` (C6).
- `CheckBridgeTraversal @ 0x004D9C60` — `__stdcall` free function (`RET 0x14`, 5 stack args, receiver ignored), installed in FootClass-family vtable slot `+0x1B0`. Full gate C7–C11.

**Layer/occupancy/render (doc-sourced):**
- `Apply_area_damage @ 0x00489280` — AoE impact-Z object-layer selector (C12) + `Wall=` tile-damage gate.
- `CellClass::AddContent/RemoveContent @ 0x0047E8A0/0x0047EA90` — dual object-list head select `+0xE4`/`+0xE8` from OnBridge.
- `ObjectClass::Mark_Occupation/Clear_Occupation @ 0x007441B0/0x00744210` — occupancy-bit layer `+0x124`/`+0x128`; Clear lacks the `0x100` gate (asymmetry, DOC-only, flagged for live re-verify before P5).
- `CellClass::Get_Draw_Offset @ 0x00480110` + `FUN_005FDCC0 @ 0x005FDCC0` — render draw-offset bridge branch (C18).
- `IsNearBridge @ 0x703B10`, `ComputeZFudge @ 0x4DAFF0` — render Z-fudge.
- `MapClass::ResolvePathCoord_BridgeAware @ 0x00583295`, `ComputeBridgeZones @ 0x0056D6E0` — path-snap + zone-graph build (co-use `0x100` AND `IsBridge||IsWoodBridge`).
- `SetBridgeDirection_NESW/_NWSE @ 0x47E040/0x47E470` — flag-layout writers (mutators; define the contract this service reads).

**Globals/tables:**
- `g_BridgeSet_TileSetBase`, `g_WoodBridgeSet_TileSetBase` — theater bridge tileset bases (`-1` when not loaded).
- `g_TubeCount` — tube-array length.
- `g_DirectionOffsets @ 0x0089F688` — 8-entry `{i16 dx,i16 dy}` cell-neighbor delta table (runtime-init; cold image 0); used for parent reconstruction via `(dir-4)&7`.
- `DAT_0089E864 / DAT_00B1D0AC / DAT_00AC13BC` — three copies of the full 4-level deck-height threshold = `round(src×4)` (engine iso-geometry constant, hardcodable).
- `WarheadType+0x144` = `Wall=` (verified via parser `0x0075D3A0` @ `0x0075D508`, string `"Wall"` @ `0x0081AC58`).
- `Rules+0x1740` = `BridgeStrength` (1500, `ini/rulesmd.ini:816`); `Rules+0xFF0` = `IonCannonWarhead`.
- `DAT_0087F83C` — bridge-record table (bridgehead coords, read by ResolvePathCoord_BridgeAware).
- `DAT_007ED3D0[]`, `DAT_00ABD490/492` — CellSpread→count / X-Y offset tables (AoE).

## Tick / render position

Not a tick-spine owner — it is a **passive read service called within other phases**:
- **Ground + air/special movement** (locomotor Z-resolve via `GetEffectiveHeight`; traversal legality via `CheckBridgeTraversal`).
- **Turrets + combat** (AoE detonation reads the impact-Z layer selector once per detonation).
- **Occupancy / cell-content updates** (object-list and occupancy-bit layer selection on add/remove/transition).
- **Pathfinding / zone-graph build** (map-init `ComputeBridgeZones`, runtime path-snap `ResolvePathCoord_BridgeAware`).
- **Render pass** (`Get_Draw_Offset` bridge branch, ZFudge) — render-only, behind a separate trait so `sim/` never depends on `render/`.

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `cell-map` | Reads `CellClass+0x140` flags, `+0x11B` level, `+0xEC` LandType, `+0x116` TubeIndex, `+0x11C` ramp byte, `+0xE4`/`+0xE8` object-list heads, `+0x124`/`+0x128` occupancy bits; calls `MapClass::ComputeBridgeZones @ 0x0056D6E0`, `MapClass::GetZoneID`, `MapClass::ResolvePathCoord_BridgeAware @ 0x00583295`, `cell_at(x+dx,y+dy)` lookups in `CheckBridgeTraversal`. | All four live primitives read CellClass fields directly; §2.3 field map; ResolvePathCoord reads neighbor cells (decompile 0x00583295). The cell substrate is the backing store. |
| `lookup-tables` | Reads `g_DirectionOffsets @ 0x0089F688` (8-entry neighbor delta table, parent reconstruct via `(dir-4)&7`); reads `g_BridgeSet_TileSetBase`/`g_WoodBridgeSet_TileSetBase`/`g_TubeCount`; AoE reads `DAT_007ED3D0[]` CellSpread table + `DAT_00ABD490/492` offset table; bridge-record table `DAT_0087F83C`; deck-height constants `DAT_0089E864`/`DAT_00B1D0AC`/`DAT_00AC13BC`. | `CheckBridgeTraversal @ 0x004D9C60` body references g_DirectionOffsets; tileset predicates read the static bases; Apply_area_damage reads CellSpread tables. These are static read-only tables. |
| `rules-class` | Reads `Rules+0x1740` (`BridgeStrength`=1500) and `Rules+0xFF0` (`IonCannonWarhead`) for the bridge tile-damage RNG gate; the AoE selector's deck-height half-threshold is an engine constant (not Rules). | §2.2; BRIDGE_AOE §7 / RUNTIME_DEEP_DIVE §10. (Tile-damage-phase edge, adjacent to the pure-predicate core.) |
| `ini-parsing` | Warhead `+0x144` (`Wall=`) is populated by `WarheadTypeClass::ReadINI_Body @ 0x0075D3A0` (@ `0x0075D508`, string `"Wall"` @ `0x0081AC58`); the AoE bridge tile-damage gate reads this parsed bool. | Live-verified this session (read_memory 0x0081ac58 = `57 61 6c 6c 00`). Edge is "the parsed value flows into the bridge AoE gate." |
| `drawing-helpers` | `Get_Draw_Offset @ 0x00480110` bridge branch (`-16`/`-31`, NS extra `-15`, shadow shift) + `FUN_005FDCC0`; `IsNearBridge @ 0x703B10` / `ComputeZFudge @ 0x4DAFF0` Z-fudge; body frame-variation table `DAT_0081cc30[16]`. RENDER-only. | §2.1, C18, BRIDGE_RENDERING. In Rust this is a SEPARATE `BridgeDrawOffset` trait implemented in `render/` so `sim/` keeps no render dep. |

## Used-by (incoming edges)

| Source slug | Via symbol / field | Evidence |
|---|---|---|
| `techno-foot` | `CheckBridgeTraversal @ 0x004D9C60` is installed at FootClass-family vtable slot `+0x1B0` (AircraftClass `0x007E2454`, FootClass `0x007E8E44`, InfantryClass `0x007EB208`, UnitClass `0x007F5E20`; all → `0x004D9C60`). `GetEffectiveHeight` consumed by locomotor Z-resolve `FUN_004CC360`/`FUN_004CC680` and TechnoClass coord/render `FUN_006F6F60`/`FUN_006F70E0` (each calls 4×: cell + 3 neighbors). | §2.4, §P2.2, §P2.4 (read_memory 0x007e2454 = `60 9c 4d 00`). Movers ARE the principal consumer of the traversal gate + height. |
| `pathfinding-helpers` | Zone-graph: `MapClass::GetZoneID`, `ComputeBridgeZones`, `RemoveBridgeZoneEdges`, `AddBridgeZoneEdges`, `FindBridgeAdjacentZoneCell`, `ResolvePathCoord_BridgeAware @ 0x00583295` all call `IsBridge`/`IsWoodBridge`/`IsLowBridgeCell` (get_xrefs_to 0x00486750). Path-entry traversal-legality routes through `CheckBridgeTraversal`. | §P2.2, §P2.3; DRIFT #6 (tileset predicates feed bridgehead-endpoint selection in the zone/path graph). |
| `cell-validation` | `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `CellClass::CheckCellPassability @ 0x004834A0`, `InfantryClass::What_Action_OnCell @ 0x0051F9B9`, `UnitClass::What_Action_OnCell @ 0x007406DC` read `IsLowBridgeCell` + the occupancy-bit layer selectors (`+0x124`/`+0x128`). What_Action_OnCell is the cursor/move-order hit-test. | §2.1, §P2.2 — low-bridge cells change cursor + legal-move decision. |
| `damage-helpers` | `Apply_area_damage @ 0x00489280` calls the bridge AoE impact-Z layer selector (`impact_z > ground_z + DAT_0089E864/2`, strict) once per detonation to pick `+0xE4` (ground) vs `+0xE8` (deck) object list before `ReceiveDamage` dispatch; bridge tile-damage gated on `Wall=`/`WallAbsoluteDestroyer`. | C12–C14; §2.1; the warhead/armor kernel reads the bridge layer selector to decide which occupants splash. |
| `abstract-object` | `ObjectClass+0x8C` (OnBridge) is the authoritative list-layer selector for `AddContent`/`RemoveContent`; `ObjectClass::Unlimbo @ 0x005F5940` sets OnBridge on `0x100` (fails without `0x200`); `ObjectClass::DropIn @ 0x005F4160` relayers on collapse; `Mark/Clear_Occupation` pick the occupancy-bit layer from the bridge Z+`0x100` predicate. | §2.1, C15–C17; object lifecycle reads the bridge predicates to decide its layer. |
| `target-scoring` | `UnitClass::TurretAI @ 0x00746984` reads tileset `IsBridge @ 0x00486750` (targeting-over-bridge decisions). | §P2.2 (get_xrefs_to 0x00486750). |
| `cell-map` | Reciprocal: the cell substrate's own zone-build / map-init calls the bridge predicates (`ComputeBridgeZones`), so `cell-map` both backs and consumes this service. | §P2.1, §P2.2. |

## Open / unverified edges

- **`Clear_Occupation @ 0x00744210` vs `Mark_Occupation @ 0x007441B0` `0x100`-asymmetry (C16)** — DOC-only; flagged for live re-decode (diff the `0x100` test) before the occupancy migration slice (P5) becomes authoritative.
- **C18 draw-offset (`Get_Draw_Offset` bridge branch)** — DOC-only, not re-verified this pass; the `-16/-16` (gamemd, iso-correct) vs `-16/-31` (current Rust/WAE) divergence is a live render decision (P7). RENDER-only edge into `drawing-helpers`.
- **`g_DirectionOffsets @ 0x0089F688` static value** — symbol reference in `0x004D9C60` is real, but the address reads all-zero in the cold image (runtime-init). Matches Rust `src/util/direction.rs DIRECTION_DELTAS`; not blocking.
- **`BridgeStrength` retail value** — `ini/rulesmd.ini:816` = 1500 per corpus + reviewer pass; not a binary edge, trivial INI confirm. (Edge into `rules-class`/`ini-parsing`.)
- **Superweapon AoE impact-Z paths** — Rust wires direct-fire/death-AoE through the layer selector; superweapon callers still all-entity until their impact-Z is traced (consumer-side gap, not a binary uncertainty).
- **`damage-helpers` warhead `+0x144` edge** — the `Wall=` gate is tile-damage-phase (out of the pure-predicate core); included as an edge because the parsed value flows through the bridge AoE function, but it does not decide object splash (`Wall=` is NOT read by C14 object dispatch).
