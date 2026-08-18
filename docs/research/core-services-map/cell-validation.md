# Core Service Profile — Cell-validation helpers (slug: `cell-validation`)

**Service:** passability / occupancy validators + nearby-passable-cell search + cell-lookup fallback.
**Primary doc:** `docs/research/CELL_VALIDATION_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, dated 2026-06-04, two-pass Ghidra-verified, reviewer GREEN).
**Confidence:** all six core function identities + two full validator bodies + the FNPC body + the dummy-cell helper are VERIFIED LIVE in gamemd.exe (image base 0x400000); addresses cited inline. Speed-table dump values and the zone matrix contents are DOC-ONLY (byte-verified in sibling reports). `IsRectInPlayfield` 4-corner formula and dummy-cell runtime-init field values remain UNCHECKED (non-blocking).

---

## Purpose

The shared cell-legality predicate layer. It answers two distinct questions for callers across the engine:

1. **Passability** — "may a unit of this SpeedType / MovementZone / required-height stand on / move through this rectangle of cells?" (terrain/zone/height/occupation-bits; NO playfield-corner check). `CellRect::CheckPassability` → per-cell `CellClass::CheckCellPassability`.
2. **Occupancy** — "is this rectangle clear to *place into*?" (object-list / reservation / cell-blocker bytes + a final 4-corner playfield containment). `CellRect::CheckOccupancy`. NOT terrain passability — it reads no SpeedType/zone.

It also owns the **nearby-passable-cell search** (`FootClass::Find_Nearby_Passable_Cell`, "FNPC") — the diamond-ring fallback that picks the actual cell a freed/spawned/relocated unit ends up in — and the **cell-lookup + non-null dummy-cell fallback** (`MapClass::Get_CellClass`) that lets callers keep dispatching on OOB probes.

These are pure read predicates over cell state. They own no per-tick phase; they are primitives consumed by movement, production, missions, superweapons, scatter, locomotion, house/AI placement, and scenario setup.

---

## Owns

State/structs this service is the authority over (read-side), plus the helper functions that define the contract:

- **The two rectangle predicates** (passability vs occupancy split) and their exact blocker/check order.
- **The reservation-arg semantics** of `CheckOccupancy(rect, layer)`: `layer == -1` skips the `+0xDC` reservation test; else mask = `1 << (layer & 0x1F)` (32-slot cap).
- **The nearby-cell search algorithm** (diamond rings, 24-candidate cap, direct/indirect split) and its selection rule:
  - no target cell → `candidates[g_CurrentFrameCounter % count]` (deterministic, NOT RNG);
  - real target cell → nearest-to-target by Euclidean distance (no frame counter, no RNG).
- **The cell-index contract**: linear index = `(short)y * 0x200 + (short)x`, valid range `[0, 0x3FFFF]`, 512-wide stride independent of loaded-map width; OOB/null returns a **non-null dummy cell** (`DAT_00ABDC50`) with the requested coord stored at `DAT_00ABDC74` (dummy+0x24).
- **The dummy cell** `DAT_00ABDC50` and its coord slot `DAT_00ABDC74`.
- **The no-candidate null-cell sentinel** `DAT_00ABD480` / `DAT_00B1CFB8` (`{0,0}`), both the output-on-failure and the input target-null check.

CellClass fields it *reads* (does not own the writes — those belong to `cell-map` + the live-list writers): `+0x44` overlay index, `+0x4C` reduced ZoneType / occupancy blocker, `+0xDC` reservation bitmask, `+0xE4`/`+0xE8` ground/bridge object-list heads, `+0x11B` cell level, `+0x11C` special/slope byte, `+0x124`/`+0x128` ground/bridge occupation bits, `+0x140 & 0x100` structural bridge flag, `+0x24/+0x26` coord.

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `CellRect::CheckPassability` | `0x0056E7C0` | Rectangle passability wrapper; 9 stack args (`RET 0x24`); ALL-cells AND-fold; overlay-reject precheck; sole caller is FNPC |
| `CellClass::CheckCellPassability` | `0x004834A0` | Per-cell passability: SpeedType==4 (Winged) fast-pass; GetZoneID comparison; required-height/bridge-layer `+0x124`/`+0x128` selection; wall-overlay exception; `g_SpeedType_LandType_Table[speed + LandType*9] == 0.0` reject (bridge-bypassed) |
| `CellRect::CheckOccupancy` | `0x00586780` | Rectangle occupancy; blocker chain (RTTI-0x24 / `+0xDC` reservation / `+0x44` / `+0x4C` / `+0x11C` / building) + final `IsRectInPlayfield(rect,1)`; `RET 0x8` |
| `FootClass::Find_Nearby_Passable_Cell` (FNPC) | `0x0056DC20` | Diamond-ring nearby search; frame-counter selection (no target) / nearest-distance (target); null-cell on no candidate |
| `MapClass::Get_CellClass` | `0x005657A0` | Coord→CellClass*; `y*0x200+x`, `[0,0x3FFFF]`, non-null dummy fallback + requested-coord store |
| `CellClass::RecalcZoneType` | `0x00483C80` | Writes reduced ZoneType `+0x4C` (matrix column) + base LandType `+0x48` |
| `MapClass::GetZoneID` | `0x0056D230` | Zone id for cell+MovementZone+bridge-aware; compared to `required_zone_id` (edge into `cell-map`) |
| `MapClass::IsRectInPlayfield` | `0x00578390` | 4-corner playfield containment; CheckOccupancy tail (corner formula UNCHECKED) |
| `FUN_0047C550` | `0x0047C550` | Ground-list RTTI-0x24 scan (TerrainClass present → reject), dispatches WhatAmI vtable slot `+0x2C` |
| `Look_up_building_in_cell` | `0x0047C520` | Finds `WhatAmI()==6` building on `+0xE4` |
| `TerrainClass::What_Am_I` | `0x0071D300` | `return 0x24` — identifies the RTTI-0x24 blocker as TerrainClass (trees/ice/veinhole/crates/lights/signs), vtable `0x007F5200` |
| `FootClass::Find_Passable_Cell_Near_Unit` | `0x00500200` | Sibling wrapper: draws `Random__RandomRanged(1,4)` THEN calls FNPC (RNG lives HERE, not in FNPC) |
| `FUN_005060B0` | `0x005060B0` | AI base-site helper: `CheckOccupancy(rect, HouseClass+0x30)` house-index reservation + AIBaseSpacing footprint expand (deferred AI seam) |

**Globals / tables:**

| Name | Address | Role |
|---|---|---|
| Cell array base | `*(MapClass+0x13C)` / `g_CellArray_Base` | Pointer-to-pointer cell array indexed `y*0x200+x`; inlined directly in both validators |
| Dummy cell | `DAT_00ABDC50` | Non-null CellClass-compatible OOB/null fallback |
| Dummy coord slot | `DAT_00ABDC74` (dummy+0x24) | Stores requested coord on fallback |
| FNPC null-cell sentinel | `DAT_00ABD480` / `DAT_00B1CFB8` | `{0,0}` output-on-failure and input target-null check |
| `g_CurrentFrameCounter` | `0x00A8ED84` | FNPC no-target selection modulo source (`frame % count`); incremented once/tick in `Main_Tick @ 0x0055DE81`; NOT an RNG stream |
| Zone-passability matrix | `0x0082A594` | `int[13][8]`, rows = MovementZone 0..12, cols = reduced ZoneType 0..7; only value `1` passes |
| Speed/Land table | `g_SpeedType_LandType_Table` | `[speed_type + LandType*9]` float; exact `0.0` rejects; reject constant `FLOAT_007E1748 == 0.0` |

---

## Tick / render position

**No tick phase of its own.** Pure read-only predicate service — it is *called by* existing phases, never scheduled by `LogicClass`. Where its callers sit in the per-tick pipeline:

- **Ground movement / pathfinding (Phase 2/3):** `Can_Enter_Cell` and A* blocked-destination fallback call CheckPassability / FNPC (seam with `pathfinding-helpers`).
- **Production spawn / war-factory exit / rally (Phase 7):** FNPC picks the exit/rally cell.
- **Scatter (Phase 7):** scatter target cell selection via FNPC.
- **Miner dock approach / chrono return:** FNPC seeded at dock cell + DockOffset.
- **Superweapons / chrono warp / paradrop / slave deploy / crate placement / scenario start positions:** all route through FNPC.

The ONE hash-relevant consumer is FNPC's no-target selection (`g_CurrentFrameCounter % count`) — it returns a different *cell* that feeds Set_Destination / spawn position (hashed). It does NOT consume either RNG stream, so it perturbs neither `Scen->Random` nor `g_MainRng` and is lockstep-safe by construction (every client shares the frame counter).

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `cell-map` | `MapClass::Get_CellClass @ 0x005657A0` array base `*(this+0x13c)`; `MapClass::GetZoneID @ 0x0056D230` (called by CheckCellPassability for the `required_zone_id` comparison); `MapClass::IsRectInPlayfield @ 0x00578390` (CheckOccupancy tail). Reads CellClass fields `+0x44/+0x4C/+0xDC/+0xE4/+0xE8/+0x11C/+0x124/+0x128/+0x140`. The cell grid + cell state are owned by CellClass/MapClass; this service reads them. | live `decompile_function 0x005657a0` / `0x004834A0` / `0x00586780` |
| `lookup-tables` | Zone-passability matrix `0x0082A594` (`int[13][8]`, only `1` passes) read by the zone-id comparison; `g_SpeedType_LandType_Table[speed+LandType*9]` + reject constant `FLOAT_007E1748` read by CheckCellPassability speed-table reject. Static read-only tables. | `decompile_function 0x004834A0`; ZONE_PASSABILITY + SPEEDTYPE_LANDTYPE reports; `read_memory ram:0x007E1748` |
| `bridge-helpers` | Bridge-layer predicate: CheckCellPassability selects `AltOccupationFlags` (`+0x128`) vs `OccupationFlags` (`+0x124`) by `(required_height == -1 || == Level+4) && (Flags & 0x100)` structural-bridge flag; CheckOccupancy / FNPC honor the bridge-deck list `+0xE8` and `allow_bridge_cells`. | `decompile_function 0x004834A0` (C8); BRIDGE_OCCUPANCY report |
| `abstract-object` | CheckOccupancy ground-list rejects dispatch the `WhatAmI()` RTTI vtable slot `+0x2C` on list objects (`0x24` = TerrainClass via `TerrainClass::What_Am_I @ 0x0071D300`; `6` = building via `Look_up_building_in_cell @ 0x0047C520`). Object identity/RTTI is an `abstract-object` concern. | `decompile_function 0x0047C550` / `0x0047C520` / `0x0071D300` |
| `lookup-tables` (frame counter) | FNPC selection reads `g_CurrentFrameCounter @ 0x00A8ED84` (read-only per-tick global) for the `% count` modulo. (Read-only global substrate; written by the tick spine, not by this service.) | `disassemble_function 0x0056DC20` tail `MOV EAX,[0x00A8ED84]; IDIV ECX`; `get_xrefs_to ram:0x00A8ED84` |

**Notes on non-edges:** FNPC itself draws NO RNG — it does NOT depend on `random-scenario`. The RNG draw lives in the *sibling* wrapper `Find_Passable_Cell_Near_Unit @ 0x00500200` (`Random__RandomRanged(1,4)`), which is a *caller* of FNPC, not part of this service. The `Buildable=` building-placement predicate (`0x0047C620`) is a separate family, NOT this service.

---

## Used-by (incoming edges)

FNPC (`0x0056DC20`) alone has **47+ callers** (`get_function_callers 0x0056DC20`, confirmed live this session). CheckPassability is called only via FNPC; CheckOccupancy is called by FNPC + the AI base-site helper; Get_CellClass is called engine-wide. Grouped by consuming service:

| Consumer slug | Via (representative callsites) | Evidence |
|---|---|---|
| `techno-foot` | `FootClass__Find_Path @ 0x004D3920`, `FootClass__ClickedAction_Object @ 0x004D74E0`, `FootClass__Find_Passable_Cell_Near_Unit @ 0x00500200`, `UnitClass__Mission_Deploy_Building @ 0x0073D630`, `TechnoClass__Set_Destination @ 0x00741970`, `FlyLocomotionClass__Descent_Step/Emergency_Relocate`, `TeleportLocomotionClass__Process/Update_Position`, `UnitClass__Scatter @ 0x00743A50`, `InfantryClass__Scatter @ 0x0051D0D0` | `get_function_callers 0x0056DC20` |
| `mission-radio` | `FootClass__Mission_AreaGuard @ 0x004D6AA0`, `FootClass__Mission_Patrol @ 0x004D4280`, `UnitClass__Mission_Harvest @ 0x0073E5E0`, `FootClass__Greatest_Threat_Scan @ 0x004D5690` | `get_function_callers 0x0056DC20` |
| `factory-house` | `BuildingClass__ExitObject_Main @ 0x00443C60`, `BuildingClass__OnConstructionComplete @ 0x00445F80`, `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, `BuildingClass__SetRallyPoint @ 0x00443860`, `HouseClass__AI_GroundRallyPoint @ 0x00509CD0`, `HouseClass__Recalc_Base_Center @ 0x004FD150`, `HouseClass__Set_Rally_Point_Cell @ 0x004FBF60`, `FUN_005060B0` (AI base-site, also passes the `+0xDC` reservation arg) | `get_function_callers 0x0056DC20` / `0x00586780` |
| `random-scenario` | `ScenarioClass__Gather_Start_Positions @ 0x00688380`; (scenario/superweapon) `ChronoSphere__WarpUnitsAtCell @ 0x0065EC30`, `SuperClass__Launch @ 0x006CC390`, `MapClass__PlaceCrateAtRandomCell @ 0x0056BD40` | `get_function_callers 0x0056DC20` |
| `damage-helpers` | `UnitClass__ReceiveDamage @ 0x00737C90` (relocate-on-damage path) | `get_function_callers 0x0056DC20` |
| `pathfinding-helpers` | A* blocked-destination fallback / `Can_Enter_Cell` consume CheckPassability as the cell-legality primitive (seam, per study §6.6) | study §6.6; `FootClass__Find_Path @ 0x004D3920` |
| `frontier-ai` | TeamClass convoy scripts ×6 (`TeamClass__Convoy_Script_*` @ `0x006EE3F0`/`0x006EE5C0`/`0x006EE800`/`0x006EC7D0`/`0x006EF700`/`0x006EFA10`), `SlaveManagerClass__FindDeployCell @ 0x006B0300`, `AircraftClass__Find_Nearest_Friendly_Airfield @ 0x0041A160` | `get_function_callers 0x0056DC20` |

(Effectively every service that needs "find a legal cell near here" depends on this one — it is a leaf-level shared primitive.)

---

## Open / unverified edges

- **`IsRectInPlayfield` 4-corner formula (`0x00578390`)** — the *call* into `cell-map` is confirmed live in CheckOccupancy's body; the exact `x+w-1`/`y+h-1` corner arithmetic (C11) is DOC-ONLY, not re-read. Edge to `cell-map` is real; the formula detail is UNCHECKED.
- **Dummy cell `DAT_00ABDC50` runtime-init field values** — statically BSS-zero; runtime-init not dumped. Affects what fields a Rust `CellRef::Dummy` exposes; does not change the edge set.
- **Save/load cell-list order (C22)** — RESOLVED in study (order serialized verbatim, NOT rebuilt; zone column re-derived on load). This is a `frontier-*` save/load seam, not a per-tick edge of this service, but it constrains how the Rust occupancy grid must serialize. Listed for completeness; not a cross-service runtime edge.
- **AI base-site internals (`FUN_005060B0`)** — the `CheckOccupancy(rect, HouseClass+0x30)` reservation edge from `factory-house`/`frontier-ai` is confirmed; the helper's internal footprint-expand / direction-probing logic is a deferred AI seam (`feedback_no_ai_yet`), not designed.

---

*Source of truth: `docs/research/CELL_VALIDATION_ENGINE_SUBSTRATE_SERVICE_STUDY.md`. FNPC caller breadth re-confirmed live this session via `get_function_callers 0x0056DC20` (47+ callers). All addresses are gamemd.exe, image base 0x400000.*
