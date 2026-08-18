# Bridge Helpers — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. Do NOT port the C++ CellClass tree, raw object-list pointers, or COM vtables literally; reproduce the verified observable contract.
**Bar:** active in a standard local skirmish. SpecialFlags/map-gated, TS-legacy, and trigger-system paths are flagged DORMANT/DEFERRED.
**Confidence posture:** This family is **largely already decoded** in `docs/research/bridges/` (60+ docs). This study is SYNTHESIS into a substrate-service boundary, not a fresh decode. The load-bearing helper-primitive addresses were **re-decompiled live** to ground the contract: `CellClass::GetEffectiveHeight @ 0x00487D50`, `CellClass::IsBridge @ 0x00486750`, `CellClass::IsLowBridgeCell/IsTubeCell @ 0x00484AB0`, `CheckBridgeTraversal @ 0x004D9C60` — all four confirmed bit-for-bit against the cited doc claims. Default verdict for any unproven equivalence is **DRIFT** — no internal-only escape hatch for active movement/combat/render/occupancy behavior. The §9 ledger separates live-verified from doc-sourced.

**Verify-and-EXPAND pass (2026-06-04, this session) — all four open gates CLOSED live:**
1. **`DAT_0089E864` / `DAT_00B1D0AC` / `DAT_00AC13BC` init RESOLVED — VERIFIED.** Each is written by one tiny `__cdecl` init thunk (`PUSH ECX; MOV EAX,[src]; LEA ECX,[EAX*4]; FILD; FADD 0.5; ftol; MOV [dst],EAX; POP ECX; RET`). Formula: `dst = ftol((double)(src * 4) + 0.5)` = `round(src * 4)`. The `+0.5` constant at `0x007E1738` reads bytes `00 00 00 00 00 00 e0 3f` = IEEE-754 **0.5** (read_memory 0x007e1738). Sources: `DAT_0089E864 ← DAT_0089E870` (get_assembly_context 0x00489120), `DAT_00B1D0AC ← DAT_00B1D0B8` (get_assembly_context 0x00735310), `DAT_00AC13BC ← DAT_00AC13C8` (get_assembly_context 0x005f3880). All three are the **same bridge-deck-height constant** (`round(per-level-height × 4)` = full 4-level deck height) computed independently in three modules (combat AoE, occupancy-bit mark, mark-put). `DAT_0089E870` itself is fixed **iso-projection geometry** (`ftol((DAT_0089E7F8 − DAT_0089E820) × cos/sin × DAT_0089E818 × 0.5)`, init block `0x00488F80..0x00489126`, get_assembly_context 0x00488fae/0x0048902c) — **engine-constant, NOT map/theater/rules data**. Cold-zero static reads are lazy-init, but the resolved values are constant across all runs. **P0 blocker (a)/(c) cleared:** a Rust port may hardcode the resolved integers; no theater-init trace needed.
2. **Warhead `+0x144` RESOLVED — VERIFIED `Wall=` (the "Bridge=" alternative is WRONG).** `WarheadTypeClass::ReadINI_Body @ 0x0075D3A0` at `0x0075D508` writes `*(u8*)(warhead+0x144) = ReadBool(..., &DAT_0081AC58)`; string at `0x0081AC58` reads bytes `57 61 6c 6c 00` = ASCII **"Wall"** (read_memory 0x0081ac58). Adjacent: `+0x145 = WallAbsoluteDestroyer`, `+0x146 = PenetratesBunker`, `+0x148 = Tiberium`. `Apply_area_damage` gates bridge tile-damage on `(scenario & 0x8000) && warhead+0x144` and overlay-destroy on `warhead+0x145 || warhead+0x144` (decompile_function 0x00489280). Tile-damage-only concern — out of pure-predicate scope, but the name is now settled.
3. **Tileset-vs-structural distinction RESOLVED — VERIFIED they are CO-USED, not redundant; DRIFT #6 is a REAL drift.** Tileset `IsBridge` (C5, `0x00486750`) gates the **zone-graph/path-snap** layer: `MapClass::GetZoneID`, `ComputeBridgeZones`, `RemoveBridgeZoneEdges`/`AddBridgeZoneEdges`, `ResolvePathCoord_BridgeAware`, `FindBridgeAdjacentZoneCell` (get_xrefs_to 0x00486750). Structural `Flags & 0x100` (C1) gates **runtime movement/AoE/occupancy layering**. `ResolvePathCoord_BridgeAware @ 0x00583295` (decompile) uses BOTH in one function with distinct roles: `Flags & 0x100` for the early-out + bridge-body walk; `IsBridge() || IsWoodBridge()` (with `LandType != 3` water-exclude) for bridgehead-endpoint selection. **NEW: a second tileset predicate `IsWoodBridge @ 0x00486770`** (identical shape, `g_WoodBridgeSet_TileSetBase`, `[base, base+0x10)`; decompile_function 0x00486770) is also co-used — the doc never listed it.
4. **vtable `+0x1B0` RESOLVED — VERIFIED, and the doc's "(CellClass)" attribution is WRONG.** The four `[DATA]` xrefs to `CheckBridgeTraversal` are slot `+0x1B0` in four **FootClass-family** vtables, NOT CellClass: AircraftClass (`0x007E2454`, base `0x007E22A4` per get_xrefs_to → AircraftClass ctor), FootClass (`0x007E8E44`), InfantryClass (`0x007EB208`), UnitClass (`0x007F5E20`). `0x007E2454 − 0x007E22A4 = 0x1B0`; the slot DWORD reads `60 9c 4d 00` = `0x004D9C60` (read_memory 0x007e2454). The function is a **`__stdcall` free function** (`RET 0x14`, 5 stack args, no `this`/ECX use — entry reads `[ESP+0xc]`/`[ESP+0x14]`/`[ESP+0x20]`, get_assembly_context 0x004d9c60) installed into the mover vtable slot; the receiver is ignored and the candidate/parent cells are stack args. The Rust `BridgeTopology::check_bridge_traversal(candidate, dir, height_io, list_byte_io, parent)` free-function shape is therefore correct; only the doc's CellClass-method framing was wrong.

**Companion:** the in-flight core-engine-substrate program — master TODO `docs/plans/2026-05-29-core-engine-substrate-todo.md` (native tick spine/LogicClass scheduler, two RNG streams, object lifecycle/unregister, frame/timing, combat/projectile pipeline, target-acquisition cadence, **map/cell substrate**, save/load/hash/MP). The bridge-helpers service is a **bridge-topology read service over the cell substrate** — it slots into the map/cell-substrate workstream and is consumed by movement, combat-AoE, pathfinding, render, and occupancy. It does NOT invent a parallel architecture. Format mirrors `docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

---

## Executive Summary

**Verdict: the bridge "helper" primitives in Rust are real and mostly correct in formula, but they are scattered across at least seven modules with no single owner, and the most load-bearing read primitive — `CheckBridgeTraversal`'s parent-fallback traversal gate — is not modeled in its binary-shaped form.** gamemd has a small family of pure, stateless predicate/offset helpers (`IsBridge`, `IsLowBridgeCell`, `GetEffectiveHeight`, the `0x100/0x200/0x80` flag tests, `CheckBridgeTraversal`, the AoE impact-Z layer selector, `Get_Draw_Offset`'s bridge branch, and the dual object-list selector) that many systems call. Rust today reimplements each at its call site against its own per-module cell view (`PathCell`, `BridgeCellFacts`, `ResolvedTerrainCell`, `OccupancyGrid`), with three confirmed parity-relevant divergences and one player-visible bug. The single largest player-visible gap is that **`CheckBridgeTraversal` is not reproduced**: Rust's `compute_bridge_transition` models the on-bridge *render/occupancy* layer flip at a boundary but does NOT model the *traversal legality* gate (the diff-{0,1,4} slope/bridgehead checks, the `direction == -1` candidate-only height seed, and the parent-`0` reconstruction), so bridge-entry/exit pathing legality is approximated rather than reproduced. The proposed replacement is an additive, read-only **`BridgeTopology` service** living in `sim/map/` (or `sim/` cell substrate) that owns the cell-flag bit semantics, the height/effective-height math, the low-bridge/tube predicate, the traversal gate in its binary `(candidate, direction, height_io, list_byte_io, parent_or_none)` shape, the AoE impact-Z layer selector, and a render-only draw-offset query (behind a `render/`-facing trait so `sim/` never depends on `render/`). Rollout follows the proven Mission/Radio rhythm — shadow → assert-equal against existing scattered helpers → make authoritative → retire scattered copies → `SNAPSHOT_VERSION` bump only if a hashed field changes. **The P0 research gate is now mostly CLOSED (2026-06-04 expand pass):** the three runtime Z-threshold globals (`DAT_0089E864`, `DAT_00B1D0AC`, `DAT_00AC13BC`) are all `round(src×4)` = the full 4-level bridge-deck height (an engine iso-geometry constant, not map/theater data — see header gate #1), so a Rust port may hardcode the resolved integer; the warhead `+0x144` key is `Wall=` (gate #2); and tileset `IsBridge`/`IsWoodBridge` are confirmed CO-USED with structural `0x100` (gate #3), making DRIFT #6 a real gap. Only `BridgeStrength=1500` (INI read) remains as a trivial confirm.

---

## Table of Contents

- §1. Verified active-YR responsibilities of the bridge-helper family
- §2. Full inventory (helpers, globals, registries, tables, vtable slots, TS-legacy)
- §3. Active-YR vs inactive/legacy split (two lists)
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements C1–C18)
- §6. Rust-native replacement boundary (`BridgeTopology` service)
- §7. Old ad hoc Rust logic to retire / fold in
- §8. Migration slices + acceptance tests (P0–P7)
- §9. Sources & Verification Ledger

---

## 1. Verified active-YR responsibilities of the bridge-helper family

"Bridge helpers" here = the shared, mostly-stateless predicate/offset primitives consumed cross-system. NOT the full damage/collapse state machine (that is a separate study), but the predicates those systems read.

| # | Responsibility (what the helper owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **`is_bridge_cell` (structural)** — `cell.Flags(+0x140) & 0x100`. The runtime "this cell carries a high-bridge structure" bit. Consumed by movement, AoE layer select, occupancy, traversal. | VERIFIED | `CellClass__GetEffectiveHeight @ 0x00487D50` (live), `CheckBridgeTraversal @ 0x004D9C60` (live), `Apply_area_damage @ 0x00489562` (BRIDGE_AOE_LAYER_DAMAGE §2). |
| R2 | **`is_bridgehead`** — `cell.Flags & 0x200`. The "bridge entry/exit transition" bit; required to legally step onto the deck from ground in directed traversal. | VERIFIED | `CheckBridgeTraversal @ 0x004D9C60` (live, `& 0x200` gates the diff-4 path). |
| R3 | **`is_anchor` / draw-offset trigger** — `cell.Flags & 0x80`. The bridge-body anchor bit; drives `Get_Draw_Offset` Y adjustment and the `GetEffectiveHeight` `+4`. | VERIFIED | `GetEffectiveHeight @ 0x00487D50` (live: `(flags>>7)&1`); `Get_Draw_Offset @ 0x00480110` (BRIDGE_RENDERING §2.2). |
| R4 | **`get_effective_height`** — `Level(+0x11B) + ((Flags>>7)&1)*4`. The single canonical "where is the deck surface" height helper for movement/positioning. | VERIFIED | `CellClass__GetEffectiveHeight @ 0x00487D50` (live decompile this session). |
| R5 | **`is_low_bridge_cell` (tube)** — `TubeIndex(+0x116) ∈ [0,g_TubeCount) AND LandType(+0xEC) == 10`. Distinguishes low-bridge/tube cells from high-bridge cells; gates tube traversal. | VERIFIED | `CellClass__IsLowBridgeCell/IsTubeCell @ 0x00484AB0` (live decompile this session). |
| R6 | **`is_bridge` / `is_wood_bridge` (tileset)** — `IsoTileTypeIndex ∈ [g_BridgeSet_TileSetBase, +0x10)` (concrete) or `[g_WoodBridgeSet_TileSetBase, +0x10)` (wood). A *second, distinct* "is this a bridge tile" predicate based on the theater bridge tileset, NOT the `0x100` flag. **Co-used WITH `0x100`** by the zone-graph/path-snap layer (`ComputeBridgeZones`, `GetZoneID`, `Add/RemoveBridgeZoneEdges`, `ResolvePathCoord_BridgeAware`, `FindBridgeAdjacentZoneCell`) — NOT interchangeable with structural `0x100`. | VERIFIED | `CellClass__IsBridge @ 0x00486750` + `IsWoodBridge @ 0x00486770` (live this session); consumers via get_xrefs_to 0x00486750; co-use in `ResolvePathCoord_BridgeAware @ 0x00583295` (decompile this session). |
| R7 | **`bridge_traversal_gate`** — `CheckBridgeTraversal`: validates a move across a bridge height boundary; returns 0=OK / 7=blocked; mutates an in/out height seed and a bridge-list selector byte. The cross-cutting pathing/locomotion legality helper. | VERIFIED | `CheckBridgeTraversal @ 0x004D9C60`, vtable slot `+0x1B0` (live + BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK). |
| R8 | **`aoe_layer_select_by_impact_z`** — picks ground (`+0xE4`) vs deck (`+0xE8`) object list once per detonation: bridge list iff `impact_cell & 0x100` AND `impact_z > ground_z + BridgeHeight/2` (strict). | VERIFIED | `Apply_area_damage @ 0x00489280` (BRIDGE_AOE_LAYER_DAMAGE §3.1). |
| R9 | **`on_bridge_layer` selector** — `ObjectClass+0x8C` (OnBridge) is the authoritative cell-list-layer selector for normal add/remove (NOT the locomotor layer). | VERIFIED | `AddContent/RemoveContent @ 0x0047E8A0/0x0047EA90` (BRIDGE_OCCUPANCY_OBJECT_LISTS §"Verified Writers"). |
| R10 | **`bridge_draw_offset`** — `Get_Draw_Offset` bridge branch: `Y -= 16` for `0x80` cells; further `-= 15` for state 9..0x11 (NS direction); plus `heightLevel*-15 + 15`. Render-only. | VERIFIED | `Get_Draw_Offset @ 0x00480110`, `FUN_005FDCC0 @ 0x005FDCC0` (BRIDGE_RENDERING §2.2–2.3). |
| R11 | **height-transition predicate** — boundary on_bridge flip used by locomotors: enter iff `dst.level == src.level - 4 AND dst structural`; exit iff `!dst structural AND src structural`. | VERIFIED | `BRIDGE_OBJECT_ONBRIDGE_FIELD` transition ordering; modeled in Rust `compute_bridge_transition`. |

---

## 2. Full inventory

### 2.1 Helper functions / methods (gamemd)

| Symbol | Address | Role | Confidence / source |
|---|---|---|---|
| `CellClass::GetEffectiveHeight` | `0x00487D50` | `Level + ((Flags>>7)&1)*4`. Movement/positioning deck height. | **LIVE this session.** |
| `CellClass::IsBridge` (tileset) | `0x00486750` | `g_BridgeSet_TileSetBase != -1 AND IsoTileTypeIndex ∈ [base, base+0x10)`. | **LIVE this session.** |
| `CellClass::IsWoodBridge` (tileset) | `0x00486770` | `g_WoodBridgeSet_TileSetBase != -1 AND IsoTileTypeIndex ∈ [base, base+0x10)`. Second tileset predicate; co-used with `IsBridge` in zone-graph + path-snap. | **LIVE this session** (decompile_function 0x00486770). NEW — not in pass 1. |
| `CellClass::IsLowBridgeCell` / `IsTubeCell` | `0x00484AB0` | `TubeIndex(+0x116) valid AND LandType(+0xEC)==10`. | **LIVE this session.** |
| `CheckBridgeTraversal` | `0x004D9C60` (FootClass-family vtable `+0x1B0`, `__stdcall`, `RET 0x14`, 5 stack args, receiver ignored) | Traversal legality + height-seed + list-byte. Parent-`0` reconstruct via `(dir-4)&7` over `g_DirectionOffsets`; `dir==-1` candidate-only seed; diff {0,1,4}. Signature `(p1=candidate cell, p2=direction, p3=*height_io, p4=*list_byte, p5=parent cell)`. | **LIVE this session** (decompile_function 0x004D9C60; slot in Aircraft/Foot/Infantry/Unit vtables, NOT CellClass). |
| `CellClass::GetTubeAtCell` | `0x00484F20` | `g_TubeArray[TubeIndex]` if signed index valid. | DOC (BRIDGE_OCCUPANCY §LowBridge). |
| `CellClass::Get_Draw_Offset` | `0x00480110` | Cell draw offset incl. bridge `-16`/`-31` branch. RENDER-only. | DOC (BRIDGE_RENDERING §2.2). |
| `FUN_005FDCC0` | `0x005FDCC0` | Generic overlay-type Y base offset (0 or -12) feeding Get_Draw_Offset. | DOC (BRIDGE_RENDERING §2.3). |
| `Apply_area_damage` (impact-Z layer select) | `0x00489280` | Picks `+0xE4` vs `+0xE8` once per detonation; bridge tile-damage phase. | DOC (BRIDGE_AOE_LAYER_DAMAGE §3, BRIDGE_RUNTIME_DEEP_DIVE §3). |
| `ApplyDamageToCell` | `0x00587180` | Tile-level bridge dispatcher (overlay-range → state machine / DestroyBridge). | DOC (BRIDGE_AOE_LAYER_DAMAGE §6). |
| `CellClass::AddContent` / `RemoveContent` | `0x0047E8A0` / `0x0047EA90` | Object-list head select `+0xE4`/`+0xE8` from the OnBridge list-layer arg. | DOC (BRIDGE_OCCUPANCY §Writers). |
| `ObjectClass::Mark_Occupation` / `Clear_Occupation` | `0x007441B0` / `0x00744210` | Occupancy-bit layer `+0x124`/`+0x128` from Z+bridge-flag predicate (Clear lacks the `0x100` gate — asymmetry). | DOC (BRIDGE_OCCUPANCY §Writers). |
| `ObjectClass::DropIn` (vtable `+0xEC`) | `0x005F4160` | Bridge-deck occupant fall on collapse: clears OnBridge, relayers, marks. | DOC (BRIDGE_OCCUPANCY §DropIn). |
| `ObjectClass::Unlimbo` | `0x005F5940` | Placement: sets OnBridge=1 on `0x100`; fails if no `0x200`. | DOC (BRIDGE_OCCUPANCY §Unlimbo). |
| `ObjectClass::ShouldBeOnBridge` | `0x005F6A70` | OnBridge derivation helper. | DOC (BRIDGE_OCCUPANCY §Sources). |
| `UnitClass::Can_Enter_Cell` | `0x0073F0A0` | List-layer decision pre-traversal + occupancy re-snapshot post-traversal. | DOC (BRIDGE_OCCUPANCY §Readers). |
| `CellClass::CheckCellPassability` | `0x004834A0` | Lower-level passability w/ explicit OnBridge/height; selects `+0x124`/`+0x128`. | DOC (BRIDGE_OCCUPANCY §Readers). |
| `IsNearBridge` (ZFudge) | `0x703B10` | Render Z-fudge proximity test; current cell or 4 diagonals `& 0x100`, skip if on-bridge. RENDER-only. | DOC (BRIDGE_RENDERING §15). |
| `ComputeZFudge` | `0x4DAFF0` | max(cliff,column,tunnel,bridge) Z-fudge for units near/under bridge. RENDER-only. | DOC (BRIDGE_RENDERING §15). |
| `SetBridgeDirection_NESW` / `_NWSE` | `0x47E040` / `0x47E470` | Stamp/clear cell flags (`0x80/0x100/0x200/0x800/0x1000/0x10000`) + default state byte. (mutator, not a pure read helper — but the *flag layout* it writes is the helper contract). | DOC (BRIDGE_RUNTIME_DEEP_DIVE §3.3, BRIDGE_RENDERING §9). |

### 2.2 Singleton / global state & static tables

| Global | Address | Meaning | Confidence |
|---|---|---|---|
| `g_BridgeSet_TileSetBase` | (read in `IsBridge` body) | Theater bridge tileset base; `-1` when not loaded. Gates `IsBridge`. | **LIVE this session** (referenced in `0x00486750` body). |
| `g_TubeCount` | (read in `IsLowBridgeCell` body) | Tube array length; bounds-checks `TubeIndex(+0x116)`. | **LIVE this session** (referenced in `0x00484AB0` body). |
| `g_DirectionOffsets` | `0x0089F688` | 8-entry `{i16 dx, i16 dy}` cell-neighbor delta table (N..NW clockwise, **cell units**); parent reconstruction in `CheckBridgeTraversal` via `(dir-4)&7` + the edge walk. **Runtime-init; cold image 0** (filled by `Foundation_direction_table_init @ 0x0049F2F0`; matches Rust `src/util/direction.rs DIRECTION_DELTAS`). | DOC + **LIVE** (referenced in `0x004D9C60` body; cross-family Phase-1 finding). |
| `DAT_0089E864` | `0x0089E864` | Full bridge-deck height = `round(DAT_0089E870 × 4)`; **halved** in the AoE object-layer threshold (`ground + DAT/2` = ground + 2 levels). Runtime lazy-init (cold image 0) but value is an **engine constant**. | **VERIFIED this session** — init thunk `0x00489100..0x00489126`, src `DAT_0089E870`, `+0.5` round const `0x007E1738`=0.5 (get_assembly_context 0x00489120; read_memory 0x007e1738). |
| `DAT_0089E870` | `0x0089E870` | Per-cell-level height (iso-projection geometry constant; the level-height multiplier). Init block `0x00488F80..` from fixed trig consts, NOT theater/rules data. | **VERIFIED this session** (get_assembly_context 0x0048908b/0x00488fae/0x0048902c). |
| `DAT_00B1D0AC` | `0x00B1D0AC` | Bridge Z threshold for `Mark_Occupation`/`Clear_Occupation` layer select; `= round(DAT_00B1D0B8 × 4)` — same deck-height value as `DAT_0089E864`, separate copy. | **VERIFIED this session** — init thunk write `0x00735310`, src `DAT_00B1D0B8`, same `round(src×4)` shape (get_assembly_context 0x00735310). |
| `DAT_00AC13BC` | `0x00AC13BC` | Bridge Z threshold for `Mark_Put`/`Mark_Remove` (`0x40` bit); `= round(DAT_00AC13C8 × 4)` — same deck-height value, separate copy. | **VERIFIED this session** — init thunk write `0x005F3880`, src `DAT_00AC13C8`, same `round(src×4)` shape (get_assembly_context 0x005f3880). |
| `DAT_007ED3D0[]` | `0x007ED3D0` | CellSpread→cell-count table (AoE radius). | DOC (BRIDGE_AOE §2). |
| `DAT_00ABD490/492` | `0x00ABD490` | CellSpread X/Y offset table (i16). Order not dumped. | DOC, partial. |
| `Rules + 0x1740` | `BridgeStrength` | RNG denominator. Retail YR = **1500** (rulesmd.ini). | DOC (BRIDGE_RUNTIME_DEEP_DIVE §10; AOE §7 says 1500, runtime-deep-dive header says 100 — see §0 conflict). |
| `Rules + 0xFF0` | `IonCannonWarhead` | Bypasses BridgeStrength RNG + retries. | DOC. |
| `WarheadType + 0x144` | `Wall=` (bool) | Inner gate for bridge tile-damage in `Apply_area_damage` (with scenario `0x8000`). Also drives overlay-destroy with `+0x145`. | **VERIFIED this session** — parser `0x0075D508` writes from string `"Wall"` @ `0x0081AC58` (read_memory 0x0081ac58 = `57 61 6c 6c 00`; decompile_function 0x0075d3a0). "Bridge=" label is WRONG. |
| `WarheadType + 0x145` / `+0x146` / `+0x148` | `WallAbsoluteDestroyer` / `PenetratesBunker` / `Tiberium` | `+0x145` co-gates overlay/bridge destroy in `Apply_area_damage`. | **VERIFIED this session** (decompile_function 0x0075d3a0: strings `s_WallAbsoluteDestroyer`, `s_PenetratesBunker`, `s_Tiberium`). |
| Scenario flag `0x8000` | `g_ScenarioClass & 0x8000` | `DestroyableBridges` outer gate. **Map/SpecialFlags-gated.** | DOC (DESTROYABLEBRIDGES_INI_GATE, BRIDGE_AOE §2). |
| `DAT_0081cc30[16]` | `0x0081cc30` | Bridge body frame-variation table (values 0-3). RENDER-only. | DOC (BRIDGE_RENDERING §14). |
| `DAT_0087F8C0..D0` | `0x0087F8C0` | Global bridge "death-list" DynamicVectorClass. **DEAD TS-LEGACY** — BSS-zero, no consumer, every push dropped. | DOC (BRIDGE_RUNTIME_DEEP_DIVE §6). |

### 2.3 CellClass field map (the helper substrate's backing store)

| Offset | Type | Meaning | Source |
|---|---|---|---|
| `+0x2C` | `CellClass*` | Anchor back-pointer (set on non-anchor body cells; nulled on collapse). | RUNTIME_DEEP_DIVE §10. |
| `+0x44` | `int` | OverlayTypeIndex; `-1` = none; cleared on partial collapse. | RUNTIME_DEEP_DIVE §10. |
| `+0x54` / `+0x58` | `int` | Ground / bridge secondary occupancy metadata (Can_Enter_Cell snapshot). | OCCUPANCY §Field Map. |
| `+0xE4` / `+0xE8` | `ObjectClass*` | Ground (`FirstObject`) / bridge-deck (`AltObject`) object-list heads. | OCCUPANCY §Field Map. |
| `+0xEC` | `int` | LandType (==10 → tube/low-bridge). | **LIVE** (`0x00484AB0`). |
| `+0x116` | `i16` | TubeIndex. | **LIVE** (`0x00484AB0`). |
| `+0x11A` | `u8` | Sub-tile index / low-bridge shore-ramp stage counter. | RUNTIME_DEEP_DIVE §3.4. |
| `+0x11B` | `i8` | Height level (drives LOS + effective height). | **LIVE** (`0x00487D50`). |
| `+0x11C` | `i8` | Ramp passability byte (diff-1 slope check). | **LIVE** (`0x004D9C60`). |
| `+0x11E` | `u8` | Anchor damage-state byte (18-state ladder). | RUNTIME_DEEP_DIVE §3.3. |
| `+0x124` / `+0x128` | `u32` | Ground / bridge occupancy bitfields. | OCCUPANCY §Field Map. |
| `+0x12C` | `u32` | Shroud bits (0x08 explored, 0x10 visible). | RUNTIME_DEEP_DIVE §7. |
| `+0x140` | `u32` | Flags: `0x80` anchor, `0x100` structural, `0x200` bridgehead, `0x400` destroyed/ramp marker, `0x800` direction-zero/orientation, `0x1000`/`0x10000` state-driven, `0x20` shroud-edge-dirty, `0x2000` pavement. | OCCUPANCY + RUNTIME_DEEP_DIVE + RENDERING §8. |
| `IsoTileTypeIndex` (`+0x34` region) | `i32` | Tileset index for `IsBridge`. | **LIVE** (`0x00486750`). |

### 2.4 vtable / COM slots

| Slot | Function | Note |
|---|---|---|
| `+0x1B0` (**FootClass family** — Aircraft/Foot/Infantry/Unit, NOT CellClass) | `CheckBridgeTraversal @ 0x004D9C60` | **VERIFIED this session.** Slot `+0x1B0` in four mover vtables: AircraftClass `0x007E2454` (base `0x007E22A4`, get_xrefs_to → AircraftClass ctor), FootClass `0x007E8E44`, InfantryClass `0x007EB208`, UnitClass `0x007F5E20`. Slot DWORD = `0x004D9C60` (read_memory 0x007e2454 = `60 9c 4d 00`). Function is a `__stdcall` free function (`RET 0x14`, 5 stack args, receiver/ECX ignored — get_assembly_context 0x004d9c60); candidate/parent cells are stack args. The doc's earlier "(CellClass)" attribution was WRONG. |
| `+0xEC` (ObjectClass) | `DropIn` | Bridge-deck occupant fall on collapse. DOC. |
| `+0x16C` (ObjectClass) | `Take_Damage` | C4Warhead damage to ground-list occupants on collapse. DOC. |
| `+0xF0` / `+0xF4` (ObjectClass) | Mark / clear-mark | Called by Add/RemoveContent after list edit. DOC. |

### 2.5 §0 conflicts to resolve before any math becomes authoritative

| Contested claim | Disagreement | Verdict / action |
|---|---|---|
| `BridgeStrength` retail default | RUNTIME_DEEP_DIVE header table says **100**; BRIDGE_AOE §7 + body say **1500** (rulesmd.ini line 816). | **Use the INI value (1500).** 100 is the engine struct *default* when the key is absent; retail rulesmd.ini sets 1500. Both can be true (default vs configured). Confirm by reading `ini/rulesmd.ini [CombatDamage] BridgeStrength` before any RNG-gate test. NOT re-read live this session — flagged for P0. |
| Warhead `+0x144` semantic | RUNTIME_DEEP_DIVE calls it `Bridge=`; BRIDGE_AOE calls it `Wall=`. | **RESOLVED — `Wall=` (VERIFIED this session, decompile_function 0x0075d3a0 @ 0x0075D508, string `"Wall"` @ 0x0081AC58).** "Bridge=" is WRONG. Tile-damage-only gate; object-splash layer select does NOT use it — out of pure-predicate scope. |
| `DAT_0089E864` / `DAT_00B1D0AC` / `DAT_00AC13BC` semantics | Three runtime-init Z thresholds; cold image 0; exact load-time formula unknown. | **RESOLVED — all three = `round(src × 4)` = the full 4-level bridge-deck height (VERIFIED this session via the init thunks; see header gate #1 / §2.2).** They are three separate copies of one engine iso-geometry constant, NOT map/theater/rules data. Rust may hardcode the resolved integer. P0 layer-select authority is UNBLOCKED. |

---

## 3. Active-YR vs inactive/legacy split

**ACTIVE in a standard YR skirmish (must reproduce):**

- `IsBridge` (tileset), `IsLowBridgeCell` (tube), `GetEffectiveHeight`, all three `0x80/0x100/0x200` flag predicates.
- `CheckBridgeTraversal` full gate (parent-`0` reconstruct, `dir==-1` candidate seed, diff {0,1,4}, bridgehead gate, list-byte write).
- Dual object-list select (`+0xE4`/`+0xE8`) via OnBridge; dual occupancy-bit select (`+0x124`/`+0x128`) via Z+`0x100`; `Clear_Occupation`'s no-`0x100` asymmetry.
- AoE impact-Z object-layer selector (`impact_z > ground_z + BridgeHeight/2`, strict, computed once per detonation).
- `Get_Draw_Offset` bridge branch, `IsNearBridge`/`ComputeZFudge`, body frame-variation table — RENDER-only but observable.
- Boundary on_bridge transition predicate; DropIn relayering on collapse.
- `DestroyableBridges` outer gate — **active but MAP/SpecialFlags-gated** (`0x8000`); retail rulesmd sets `DestroyableBridges=yes`, but a map can disable it, and when off NO bridge tile damage fires for ANY warhead. Treat the gate itself as always-present; its value is per-map data.

**INACTIVE / LEGACY / DEFERRED (do NOT design substrate around):**

- **Global bridge death-list `DAT_0087F8C0..D0`** — DEAD TS-legacy. BSS-zero, no consumer/allocator/processor; every `BlowUpBridge` push silently dropped. Do not port. (RUNTIME_DEEP_DIVE §6.)
- **TS subterranean / tunnel locomotion** — TS legacy, not in YR. The `LandType==10` tube path IS the *low-bridge* mechanic (active), but subterranean movement is not. Do NOT fold low-bridge tube semantics into a tunnel/subterranean model. (project memory `feedback_no_tunnel_subterranean`.)
- **Fog-of-war darkening on bridge cells** — TS legacy; `FogOfWar` defaults false in YR. `RecalcBridgeShroudFlags @ 0x578100` (120-frame poll) clears explored+visible bits on `0x20` cells, but the *re-darkening* visual is FoW-gated. Model only the shroud-cache resync (state-hash-neutral), not FoW darkening. (RUNTIME_DEEP_DIVE §7.5; CLAUDE.md TS-only list.) `FUN_004d1890` FoggedObject snapshot walker is dormant in YR (RENDERING §13).
- **`FUN_006E61F0` / `DAT_008B41A8`** — NOT a bridge predicate. It is `TagTypeClass::GetEventCategoryBitmask` and `g_DestroyedEventTagList` (trigger system). Bit-4 means "tag has a Destroyed event (code 8/0x18)", not "bridge-linked cell". The §16.2 "bridge cell predicate" hypothesis was REFUTED. Trigger-system bookkeeping, no-op on skirmish maps. Do NOT include in the bridge service. (FUN_006E61F0 doc.)
- **`_Low` EW-collapse `*_High` helper anomaly** — RESOLVED: the helper pairs are bit-identical (similarity 1.0); only a theater-base global differs. ONE parameterized Rust helper is correct; the binary's split is internal plumbing, not behavior. (RUNTIME_DEEP_DIVE §13 Q1.)

---

## 4. Comparison against the current Rust architecture

**Verdict: the predicates exist and are mostly formula-correct, but there is no single owner and they are reimplemented per consumer against four different cell views. One traversal gate is missing; three occupancy/layer behaviors drift; one CABHUT-C4 bug is open.**

### 4.1 Where the helpers live today (scattered)

| Helper concept | Rust location(s) | State |
|---|---|---|
| Flag-bit constants + predicates | `src/map/bridge_facts.rs` (`BRIDGE_FLAG_*`, `has_structural_bridge`, `has_transition_flag`, `is_anchor_self`) | EXISTS — but this is the **map-load stamp** view, not a runtime cell-substrate view. |
| Path-time predicates | `src/sim/pathfinding/core.rs::PathCell` (`has_structural_bridge`, `has_bridge_marker_0x80`, `has_bridgehead_transition`, `bridge_deck_level_if_any`, `effective_cell_z_for_layer`, `is_elevated_bridge_cell`, `is_at_bridge_level @ line 410`) | EXISTS — a **third** cell view with its own bool fields (`bridge_walkable`, `bridge_structural`, `transition`, `bridge_marker_0x80`). |
| Effective height | `PathCell::effective_cell_z_for_layer` (deck vs ground by layer) | PARTIAL — correct for layer-driven Z, but NOT the gamemd `Level + (anchor?4:0)` flag-driven form (R4). Two different inputs (layer vs flag). |
| Resolved-terrain bridge facts | `src/map/resolved_terrain.rs` (`BridgeLayer`, `BridgeDirection`, `BridgeOracleCellFacts`, `deck_level`, `bridge_walkable`, `bridge_transition`) | EXISTS — a **fourth** cell view (load-time resolved). |
| Pure RE-ported helpers | `src/bridge_re.rs` (overlay-range classifiers, `get_cell_zone_id_bridge_policy_decision`, zone-connection record decode) | EXISTS — stateless, well-tested, but a parallel island not wired into a service. |
| Boundary on_bridge transition | `src/sim/movement/movement_bridge.rs` (`compute_bridge_transition`, `resolve_cell_transition_bridge_state`, `apply_pending_bridge_render_state`) | EXISTS — models R11 (the *render/occupancy layer flip*), well-tested incl. ramp-up/ramp-down decoupling. |
| Traversal LEGALITY gate (R7) | — | **MISSING.** No `CheckBridgeTraversal` analog. `pathfinding/cell_entry.rs` takes a single `target_layer` and cannot express nullable parent or `dir==-1` candidate-only seed (per PARENT_FALLBACK §Rust status). |
| AoE impact-Z layer select (R8) | `src/sim/combat/combat_aoe.rs` (`AoELayerContext`, single-layer select from impact cell) | PARTIAL — direct-fire/death-AoE wired (2026-05-17 pass); superweapon paths still all-entity until impact-Z traced. |
| Object-list layer (R9) | `src/sim/occupancy.rs` (`OccupancyGrid` layer-tagged), `src/sim/movement/movement_occupancy.rs` | CORRECT for `rebuild` — `OccupancyGrid::rebuild @ occupancy.rs:118` already selects layer via `GameEntity::occupancy_list_layer @ game_entity.rs:743`, which derives Ground/Bridge from `self.on_bridge` (line 757), NOT `locomotor.layer` (locomotor only filters out Air/Underground). Runtime move path still single-layered (see 4.2#2). [Reviewer-verified 2026-06-04: Read `occupancy.rs:118-142` + `game_entity.rs:743-762`.] |
| Collapse object handling | `src/sim/world/bridge_orchestrator.rs` (`drop_in_bridge_deck_entities`, kill ground-layer) | PARTIAL — matches outcome but does not relayer the persistent `OccupancyGrid` entry like `DropIn`. |
| Render draw-offset (R10) | `src/render/bridge_atlas.rs`, `src/render/bridge_railing_atlas.rs`, `resolved_terrain.rs::BridgeDirection` (-16/-31 offsets) | EXISTS in render; uses WAE -16/-31 (equal-height) vs gamemd -16/-16 (iso-correct) — RENDERING §14 flags this as a deliberate-for-now divergence. |
| Damage state machine / specs | `src/sim/bridge_state/`, `src/sim/bridge_specs.rs`, `src/rules/bridge_warheads.rs` | EXISTS (separate study scope). |

### 4.2 Confirmed drifts / risks (from the occupancy + traversal docs, re-stated against current code)

1. ~~**Rebuild layer source (DRIFT).**~~ **STALE — NOT a drift in current code.** [Reviewer-verified 2026-06-04: Read `occupancy.rs:118-142`, `game_entity.rs:743-762`.] `OccupancyGrid::rebuild` does NOT use `locomotor.layer` for the list layer; it calls `entity.occupancy_list_layer()` which returns `Bridge`/`Ground` from `self.on_bridge` (game_entity.rs:757) — exactly C15 / `ObjectClass+0x8C`. The locomotor layer is read only to drop Air/Underground entities from the ground/bridge lists, which is correct. The original OCCUPANCY-doc claim predates the `occupancy_list_layer` helper. The §7 P5 "retire `locomotor.layer` source" item is therefore already satisfied for the `rebuild` path; only the runtime single-layer move (#2) remains.
2. **Transition insertion timing (DRIFT).** `movement_step.rs::process_cell_crossings` moves occupancy with the path layer *before* `resolve_cell_transition_bridge_state` knows the post-transition OnBridge. gamemd removes from old cell with OLD OnBridge, then inserts into new cell with UPDATED OnBridge. (OCCUPANCY §Confirmed Parity Gaps.)
3. **DropIn relayering (DRIFT).** `bridge_orchestrator.rs::drop_in_bridge_deck_entities` clears entity state but does not remove/re-submit the persistent occupancy entry the way `DropIn @ 0x005F4160` does. (OCCUPANCY §Confirmed Parity Gaps.)
4. **Traversal legality not modeled (DRIFT — biggest).** No `CheckBridgeTraversal` equivalent; bridge-entry/exit pathing legality is approximated by `compute_bridge_transition` (a render/occupancy flip), missing the diff-{0,1,4} slope checks, bridgehead `0x200` gate, `dir==-1` seed, and parent-`0` reconstruction. (PARENT_FALLBACK §Rust Implications.)
5. **`Clear_Occupation` no-`0x100` asymmetry (DRIFT, latent).** Rust has no explicit `+0x124`/`+0x128` bitfield model; any future reservation work must preserve that the bridge bit can be CLEARED without the `0x100` flag present (matters during collapse cleanup). (OCCUPANCY §Clear asymmetry.)
6. **Two distinct "is bridge" predicates collapsed (CONFIRMED DRIFT — was latent/UNCHECKED).** gamemd has BOTH the tileset predicates `IsBridge`/`IsWoodBridge` (R6) and `Flags & 0x100` (structural, R1); Rust only models the structural flag. **Verified this session that active YR callers DO distinguish them:** the zone-graph + path-snap layer (`ComputeBridgeZones`, `GetZoneID`, `ResolvePathCoord_BridgeAware`, `Add/RemoveBridgeZoneEdges`, `FindBridgeAdjacentZoneCell`) uses the tileset predicate to classify destination tiles for bridgehead-endpoint selection, while movement/AoE/occupancy use `0x100` for runtime layering. `ResolvePathCoord_BridgeAware @ 0x00583295` uses both in one body. The Rust zone-build path (`src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`, per BRIDGE_ZONE_LIFECYCLE §14) already does some tileset-table detection; the `BridgeTopology` service MUST expose `is_bridge_tileset`/`is_wood_bridge_tileset` distinct from `is_bridge_cell`. Do NOT assume equivalence.

### 4.3 Known open port bug (flag, do NOT fix here)

**SEAL/Tanya C4 on CABHUT does nothing.** gamemd has NO Immune gate; the bug is port-side (project memory `project_c4_bridge_hut_followup`, 2026-05-12 investigation refuted the Immune-gate hypothesis). This is a CABHUT-action-routing bug, adjacent to but not part of the pure-helper family. Surface it; the substrate service does not fix it but must not entrench it (the traversal/occupancy helpers it touches should expose CABHUT cells correctly).

---

## 5. gamemd-native behavior contract (C1–C18)

The exact observable semantics any Rust replacement must reproduce. Each is testable.

**Predicate primitives**

- **C1 — Structural bridge bit.** `is_bridge_cell(cell) ≡ (cell.flags & 0x100) != 0`. (LIVE.)
- **C2 — Bridgehead bit.** `is_bridgehead(cell) ≡ (cell.flags & 0x200) != 0`. (LIVE.)
- **C3 — Anchor bit.** `is_anchor(cell) ≡ (cell.flags & 0x80) != 0`. (LIVE.)
- **C4 — Effective height.** `effective_height(cell) = (i8)cell.level + ((cell.flags >> 7) & 1) * 4`. Signed level read. Returns `level` for non-anchor, `level+4` for anchor. (LIVE `0x00487D50`.)
- **C5 — Tileset IsBridge / IsWoodBridge.** `is_bridge_tileset(cell) ≡ g_BridgeSet_TileSetBase != -1 AND iso_tile_index ∈ [base, base+0x10)`; `is_wood_bridge_tileset(cell) ≡ g_WoodBridgeSet_TileSetBase != -1 AND iso_tile_index ∈ [woodbase, woodbase+0x10)`. Distinct from C1 (structural `0x100`) and **co-used with it** in zone-graph/path-snap (see DRIFT #6). Gated on the respective tileset being theater-loaded. (LIVE `0x00486750` / `0x00486770`.)
- **C6 — Low-bridge / tube cell.** `is_low_bridge_cell(cell) ≡ tube_index ∈ [0, g_TubeCount) AND land_type == 10`. (LIVE `0x00484AB0`.)

**Traversal gate (`CheckBridgeTraversal`, LIVE `0x004D9C60`)** — signature `(candidate, direction i32, height_io *i32, list_byte_io *u8, parent_or_null) -> {OK=0|BLOCKED=7}`:

- **C7 — Parent-`0` reconstruction.** If `parent == NULL`, reconstruct `parent = cell_at(candidate.x + g_DirectionOffsets[(direction-4)&7].dx, candidate.y + .dy)` BEFORE the `direction==-1` branch. `None` parent is NOT "use mover's current cell". (LIVE; PARENT_FALLBACK §"Parent 0".)
- **C8 — `direction == -1` candidate-only seed.** If `*height == -1 AND (candidate.flags & 0x100)`, set `*height = (i8)candidate.level + 4` and return OK. No bridgehead/diff/slope checks. (LIVE.)
- **C9 — Directed `height==-1` seed from parent.** If `*height==-1 AND (parent.flags & 0x100)`: `*height = (i8)parent.level + 4`; then if `(candidate.flags & 0x200)==0` return 7 (BLOCKED). (LIVE.)
- **C10 — Directed diff.** `iVar5 = (i8)candidate.level`; `base = (parent.flags & 0x100) ? (i8)parent.level : *height`; `diff = base - iVar5`. Only `abs(diff) ∈ {0,1,4}` may pass; else return 7. (LIVE.)
  - `abs==0`: return 7 if `((candidate lacks 0x100) OR (candidate lacks 0x200) OR (parent not bridge)) AND (*height != -1 AND *height != candidate.level)`. [Reviewer-verified 2026-06-04 against `decompile_function 0x004D9C60`: the condition is `(((flags&0x100)==0) || ((flags&0x200)==0) || (uVar1==0))` — *either* bit missing, NOT both. Earlier "& (0x100|0x200) absent" wording was ambiguous/wrong.]
  - `abs==1`: if `diff < 1` test `parent.+0x11C != 0` (else 7); else test `candidate.+0x11C != 0` (else 7).
  - `abs==4`: if `(i8)parent.level == candidate.level-4`: require `*height == candidate.level` AND parent bridge, else 7. If `candidate.level == (i8)parent.level-4`: require candidate `0x100` AND `0x200`, then `*list_byte = 1; return OK`.
- **C11 — Default OK.** All paths not hitting a `return 7` return OK (0). (LIVE.)

**AoE object-layer select (`Apply_area_damage`, DOC `0x00489280`)**

- **C12 — Layer chosen once per detonation from the impact cell.** `use_bridge_list = (impact_cell.flags & 0x100) AND (impact_z > ground_z(impact) + DAT_0089E864/2)`. Comparison is STRICT `>`. `DAT_0089E864 = round(per_level_height × 4)` = full 4-level deck height, so `/2` = 2 level-heights above ground. **VERIFIED this session** (decompile_function 0x00489280 `iVar10 + DAT_0089e864 / 2 < param_1[2]`; init resolved — see §2.2). The constant is engine iso-geometry, hardcodable.
- **C13 — Same selector for every spread cell.** Not recomputed per affected cell; the whole CellSpread reads the selected list from each cell. (DOC.)
- **C14 — Collect-then-dispatch.** Targets gathered `{object,distance}` first, then `ReceiveDamage` (vtable `+0x16C`) in collected order. `Wall=` does NOT decide object splash. (DOC.)

**Object-list / occupancy layer (DOC)**

- **C15 — List-layer selector is OnBridge.** Normal Add/RemoveContent select `+0xE4`/`+0xE8` from `ObjectClass+0x8C`, not the locomotor layer. (DOC `0x0047E8A0`.)
- **C16 — Occupancy-bit layer + Clear asymmetry.** `Mark_Occupation` writes `+0x128` iff `ground_z + Zthresh <= obj.z AND (cell.flags & 0x100)`, else `+0x124`; `Clear_Occupation` uses the SAME Z test but does NOT require `0x100` to clear `+0x128`. List-layer and bit-layer may legitimately disagree at ramps. (DOC.)
- **C17 — Boundary transition order.** Remove-from-old (old OnBridge) → move coords → evaluate transition predicate → update OnBridge → add-to-new (new OnBridge). Enter iff `dst.level == src.level - 4 AND dst structural`; Exit iff `!dst structural AND src structural`. (DOC; modeled by Rust `compute_bridge_transition`.)

**Render (RENDER-only, observable)**

- **C18 — Draw offset.** `Get_Draw_Offset` for `0x80` cells: `Y_adjust = overlay_base_y - 16`; if state ∈ [9, 0x11] further `-= 15`; final `Y = Y_adjust + level*-15 + 15`. NS bridges (state 9-17) get the extra -15 AND a shadow shift (X-15, Y+7) regardless of damage. (DOC `0x00480110`.) gamemd uses -16 for both directions (iso-correct, 15px EW/NS difference from SHP frame_y); Rust currently uses -16/-31 (WAE equal-height) — a documented deliberate divergence to revisit when bridge render is reworked.

---

## 6. Rust-native replacement boundary: `BridgeTopology` service

**Principle:** one read-only topology service over the cell substrate. It owns the bit semantics, height math, traversal gate, low-bridge predicate, and AoE layer selection. It is `sim/`-layer and consumed by movement/combat/pathfinding/occupancy. Render-facing offset math lives behind a trait so `sim/` never depends on `render/` (invariant #1). All math fixed-point or integer; no f32/f64 in the gate.

### 6.1 Location

`src/sim/map/bridge_topology.rs` (new), part of the map/cell-substrate workstream of the core-engine-substrate program. It reads the existing resolved cell store; it does NOT introduce a fifth cell view — it is the single accessor the other four collapse into over the migration (§7).

### 6.2 Surface sketch (types & signatures — illustrative, not final)

```rust
// Pure flag-bit semantics, mirrors CellClass+0x140. Bit values are gamemd-native.
bitflags! {
    pub struct BridgeFlags: u32 {
        const ANCHOR        = 0x0080; // R3 / C3
        const STRUCTURAL    = 0x0100; // R1 / C1
        const BRIDGEHEAD    = 0x0200; // R2 / C2
        const DESTROYED_RAMP= 0x0400;
        const DIRECTION_ZERO= 0x0800;
        const PAVEMENT      = 0x2000;
        // ... 0x1000 / 0x10000 state-driven (set by SetBridgeDirection)
    }
}

/// Read-only view of one cell's bridge-relevant substrate fields.
/// Backed by the cell substrate; the service does not own storage.
pub struct CellBridgeView<'a> { /* level: i8, flags: BridgeFlags, ramp_byte: i8,
    iso_tile_index: i32, tube_index: Option<i16>, land_type: u8, state_byte: u8 */ }

pub enum TraversalResult { Ok, Blocked }       // 0 / 7
pub enum ListLayer { Ground, Bridge }          // +0xE4 / +0xE8

pub trait BridgeTopology {
    // Predicate primitives — C1..C6
    fn is_bridge_cell(&self, c: Cell) -> bool;           // flags & STRUCTURAL
    fn is_bridgehead(&self, c: Cell) -> bool;            // flags & BRIDGEHEAD
    fn is_anchor(&self, c: Cell) -> bool;                // flags & ANCHOR
    fn effective_height(&self, c: Cell) -> i32;          // level + (anchor?4:0)
    fn is_bridge_tileset(&self, c: Cell) -> bool;        // C5: concrete bridge tileset window
    fn is_wood_bridge_tileset(&self, c: Cell) -> bool;   // C5: wood bridge tileset window
    fn is_low_bridge_cell(&self, c: Cell) -> bool;       // tube_index valid && land==10

    // Traversal gate — C7..C11. Binary-shaped: parent is Option, dir/height include -1.
    fn check_bridge_traversal(
        &self,
        candidate: Cell,
        direction: i32,            // -1 allowed
        height_io: &mut i32,       // -1 = "unknown", seeded in place
        list_byte_io: &mut bool,   // forced true on exit-orientation
        parent: Option<Cell>,      // None != current cell
    ) -> TraversalResult;

    // AoE object-layer select — C12. impact_z and ground_z in leptons.
    fn aoe_object_layer(&self, impact_cell: Cell, impact_z: i32, ground_z: i32) -> ListLayer;

    // Occupancy-bit layer select — C16 (Mark vs Clear asymmetry exposed via `require_flag`).
    fn occupancy_bit_layer(&self, c: Cell, obj_z: i32, ground_z: i32, require_structural: bool) -> ListLayer;

    // Boundary on_bridge transition — C17 (the existing compute_bridge_transition, folded in).
    fn bridge_transition(&self, src: Cell, dst: Cell) -> BridgeTransition;
}

/// Render-only offset query lives in a SEPARATE trait so sim/ has no render/ dep.
/// Implemented in render/ over the same CellBridgeView. C18.
pub trait BridgeDrawOffset {
    fn bridge_draw_offset(&self, c: Cell, overlay_base_y: i32) -> (i32, i32);
}
```

### 6.3 Ownership & dependency direction

- Lives in `sim/`; depends only on the cell substrate (`map/` + `sim/` cell store) and `util/direction` (for `g_DirectionOffsets`). NO dependency on `render/`, `ui/`, `audio/`, `net/`.
- `combat_aoe`, `pathfinding`, `movement`, `occupancy` call the trait. `render/` implements `BridgeDrawOffset` separately and reads the same views.
- `bridge_re.rs` pure helpers fold IN as private implementation detail of the service (overlay-range classifiers stay, re-exported through the topology API).
- The two UNCHECKED Z thresholds (`DAT_0089E864`, `DAT_00B1D0AC`) become named constants in the service, resolved from the cell substrate's theater-init — gated behind P0.

---

## 7. Old ad hoc Rust logic to RETIRE / fold into the service

| File:symbol | Action | Rationale |
|---|---|---|
| `src/sim/pathfinding/core.rs::PathCell::{has_structural_bridge, has_bridge_marker_0x80, has_bridgehead_transition, bridge_deck_level_if_any, effective_cell_z_for_layer, is_elevated_bridge_cell}` and `is_at_bridge_level` (line 410) | FOLD into `BridgeTopology` (keep `PathCell` as a backing view, route predicates through the service). | Predicate logic duplicated against PathCell's bool fields; `effective_cell_z_for_layer` is a layer-driven approximation of C4's flag-driven `Level+(anchor?4:0)`. |
| `src/map/bridge_facts.rs::BridgeCellFacts::{has_flag, has_structural_bridge, has_transition_flag, is_anchor_self}` | KEEP as the map-load stamp producer; route its read predicates through the same `BridgeFlags` constants the service defines (single source of bit values). | Three copies of `0x80/0x100/0x200` constants today (`bridge_facts.rs`, `bridge_re.rs`, implicit in `pathfinding`). |
| `src/sim/combat/combat_aoe.rs::AoELayerContext` single-layer select | FOLD the impact-Z threshold into `BridgeTopology::aoe_object_layer`; keep the context plumbing. | C12 threshold (`> ground + BridgeHeight/2`) is the load-bearing part; currently inline. |
| `src/sim/occupancy.rs::OccupancyGrid::rebuild` | NO ACTION — already drives list layer from `on_bridge` via `GameEntity::occupancy_list_layer` (game_entity.rs:757). [Reviewer-verified 2026-06-04.] The runtime *move* path (movement_step.rs:1190-1210) is the remaining gap, not `rebuild`. | Stale; DRIFT #1 retracted. |
| `src/sim/movement/movement_step.rs::process_cell_crossings` occupancy move ordering | RE-ORDER to remove-old(OLD OnBridge)→update→add-new(NEW OnBridge) (C17). | Confirmed DRIFT #2. |
| `src/sim/world/bridge_orchestrator.rs::drop_in_bridge_deck_entities` | EXTEND to relayer the persistent `OccupancyGrid` entry (DropIn semantics). | Confirmed DRIFT #3. |
| `src/sim/pathfinding/cell_entry.rs` (single `target_layer`) | REPLACE the bridge portion with `BridgeTopology::check_bridge_traversal` in binary shape. | Missing gate (DRIFT #4). |
| `src/bridge_re.rs` overlay classifiers + `get_cell_zone_id_bridge_policy_decision` | KEEP, re-home as private service internals. | Already pure + tested; just unowned. |

---

## 8. Migration slices (shadow-first, dependency-ordered) + acceptance tests

Each slice is independently shippable. Shadow-first means: build the new service, assert it returns identical results to the existing scattered helper, THEN cut consumers over, THEN delete the old copy. No `SNAPSHOT_VERSION` bump is required unless a *hashed* field changes — most of these are read-helper consolidations (hash-neutral); flag the one that may touch hash (P5).

- **P0 — RESEARCH GATE (NOW MOSTLY CLOSED, 2026-06-04).** (a) `BridgeStrength` retail value — corpus says `ini/rulesmd.ini:816` = 1500; **trivial INI confirm remains.** (b) warhead `+0x144` = **`Wall=` — RESOLVED** (string `0x0081AC58`). (c) `DAT_0089E864`/`DAT_00B1D0AC`/`DAT_00AC13BC` = **`round(src×4)` engine deck-height constant — RESOLVED** (init thunks). The only residual P0 item is the trivial `BridgeStrength` INI read; layer-select math is UNBLOCKED. *Test: none (research).*
- **P1 — `BridgeFlags` + predicate primitives (C1–C6, now SEVEN predicates).** Introduce `BridgeTopology` with the pure predicates over a `CellBridgeView`: structural/bridgehead/anchor (C1–C3), effective-height (C4), **`is_bridge_tileset` AND `is_wood_bridge_tileset`** (C5, both — DRIFT #6), low-bridge (C6). Shadow: assert equal to `PathCell`/`BridgeCellFacts` predicates for every cell of three fixture maps. **Tests:** `bridge_topology_predicates_match_pathcell`, `effective_height_anchor_plus4_signed_level`, `is_bridge_tileset_distinct_from_structural_flag`, `is_wood_bridge_tileset_distinct_from_concrete_and_structural`, `is_low_bridge_requires_landtype10_and_tube_in_range`.
- **P2 — Traversal gate (C7–C11), shadow.** Implement `check_bridge_traversal` in binary shape; run it alongside the current pathing in shadow and log disagreements (do not change pathing yet). **Tests:** `traversal_parent_none_reconstructs_via_dir_minus4`, `traversal_dir_minus1_candidate_only_seed_no_bridgehead`, `traversal_directed_diff4_exit_sets_list_byte`, `traversal_diff_other_than_0_1_4_blocks`, plus a golden table of `(candidate,dir,height,parent)→(result,height_out,list_byte)` derived from the `0x004D9C60` decompile.
- **P3 — Traversal gate authoritative.** Route `pathfinding/cell_entry.rs` + A*/runtime locomotion through the gate; preserve A*'s explicit-parent vs runtime null-parent distinction (PARENT_FALLBACK caller matrix). **Tests:** `astar_uses_explicit_parent`, `drive_runtime_uses_null_parent_reconstruct`, `jumpjet_landing_dir_minus1_path`; a deterministic replay over a high-bridge fixture must produce identical paths to the recorded baseline.
- **P4 — AoE object-layer select authoritative (C12–C14).** Fold C12 threshold into the service; cut `combat_aoe` + remaining superweapon callers over once their impact-Z is traced (depends on P0c). **Tests:** `aoe_strict_gt_ground_plus_half_bridge_height`, `aoe_layer_chosen_once_per_detonation`, `aoe_does_not_double_hit_deck_and_under_bridge`.
- **P5 — Occupancy/list-layer correctness (C15–C17).** Drive list layer from `on_bridge`; re-order `movement_step` insertion; preserve `Clear_Occupation` asymmetry. **May touch state hash** (occupancy contents) — bump `SNAPSHOT_VERSION` if the hashed occupancy representation changes; otherwise hash-neutral. **Tests:** `occupancy_list_layer_from_on_bridge_not_loco_layer`, `transition_removes_old_layer_inserts_new_layer`, `clear_occupation_no_structural_flag_required`, plus a ramp-crossing replay diff.
- **P6 — DropIn relayering on collapse.** Extend `bridge_orchestrator::drop_in_bridge_deck_entities` to relayer the persistent occupancy entry. **Tests:** `collapse_dropin_relayers_occupancy_to_ground`, `collapse_ground_list_takes_c4_damage_deck_list_drops_in`.
- **P7 — Render draw-offset query (C18) + retire scattered predicates.** Implement `BridgeDrawOffset` in `render/`; delete the folded predicate copies (§7) once consumers are on the service. Decide -16/-16 (gamemd) vs -16/-31 (WAE) — recommend switching to -16/-16 with a visual verify. **Tests:** `bridge_draw_offset_ns_extra_minus15`, `bridge_shadow_shift_ns_x_minus15_y_plus7`; visual regression on a temperate high-bridge fixture.

---

## Pass 2 — Expansion (completeness sweep, 2026-06-04)

Systematic xref/consumer/slot sweep of the family's core helpers and globals. Everything here is bit-VERIFIED live this session unless tagged otherwise.

### P2.1 — NEW methods / predicates not in pass 1

| Symbol | Address | Role | Status |
|---|---|---|---|
| `CellClass::IsWoodBridge` | `0x00486770` | Second tileset predicate (`g_WoodBridgeSet_TileSetBase`, `[base, base+0x10)`). Co-used with `IsBridge` in zone-graph + `ResolvePathCoord_BridgeAware` + `Apply_area_damage` (the `g_WoodBridgeSet_TileSetBase` low/high-bridge tile-damage branches). | VERIFIED (decompile_function 0x00486770). |
| `MapClass::ResolvePathCoord_BridgeAware` | `0x00583295` (entry `0x005833F0`-region; body decompiled) | Path-snap helper: given a bridge cell + flag, returns the bridgehead endpoint coord. Uses `Flags & 0x100` (early-out + body walk) AND `IsBridge()||IsWoodBridge()` (endpoint pick) AND `LandType != 3`. Reads `g_DirectionOffsets`-style bridge-record table `DAT_0087F83C`. | VERIFIED (decompile_function 0x00583295). |
| `MapClass::ComputeBridgeZones` | `0x0056D6E0` | Map-init zone-graph builder; calls `IsBridge`/`IsWoodBridge`/`IsLowBridgeCell` per cell. | DOC (BRIDGE_ZONE_LIFECYCLE §17) + xref-confirmed this session. |

### P2.2 — NEW consumers of the core helpers (who calls them — get_xrefs_to this session)

| Helper | Newly-enumerated consumers | Player-visible? |
|---|---|---|
| `GetEffectiveHeight @ 0x00487D50` | `FUN_004CC360`, `FUN_004CC680` (locomotor Z-resolve, `0x004cc...` locomotion region), `FUN_006F6F60`, `FUN_006F70E0` (TechnoClass coord/render region) — each calls it 4× (cell + 3 neighbors). | Yes — movement Z + render coord. |
| `IsLowBridgeCell @ 0x00484AB0` | `MapClass::ComputeBridgeZones`, `FUN_00704000` (UnitClass region), `FUN_00484AE0`, **`InfantryClass::What_Action_OnCell @ 0x0051F9B9`**, **`UnitClass::What_Action_OnCell @ 0x007406DC`**, `FUN_00728280` (cursor/action). | Yes — `What_Action_OnCell` is the **cursor/move-order hit-test**; low-bridge cells change the cursor + legal-move decision. |
| `IsBridge (tileset) @ 0x00486750` | `MapClass::GetZoneID`, `ComputeBridgeZones`, `RemoveBridgeZoneEdges`, `AddBridgeZoneEdges`, `ResolvePathCoord_BridgeAware`, `FindBridgeAdjacentZoneCell`, `FUN_00582D70`, **`UnitClass::TurretAI @ 0x00746984`**. | Yes — zone/path graph (path legality) + turret AI targeting-over-bridge. |

### P2.3 — NEW globals / tables touched (get_xrefs_from / init-chain this session)

| Global | Address | Meaning | Status |
|---|---|---|---|
| `g_WoodBridgeSet_TileSetBase` | (read in `IsWoodBridge` body + `Apply_area_damage`) | Wood-bridge tileset base; `-1` when not loaded. Gates `IsWoodBridge`. | VERIFIED (decompile_function 0x00486770 / 0x00489280). |
| `DAT_0089E870` source-constant chain | `0x0089E7F8`, `0x0089E820`, `0x0089E818`, consts `0x007E1708/10/18/20/28/30`, `0x007E5128/30` | Fixed iso-projection geometry feeding the per-level height (and thence the three deck-height thresholds). NOT theater/rules data. | VERIFIED shape (get_assembly_context 0x00488fae/0x0048902c/0x00489000). Exact numeric values UNCHECKED (cold-zero, lazy-init) — irrelevant since the *resolved* `DAT_0089E870`→`×4`→deck-height is a single constant. |
| `DAT_0087F83C` | `0x0087F83C` | Bridge-record table base (`record = idx*0x10 + DAT_0087F83C`), read by `ResolvePathCoord_BridgeAware` for bridgehead coords. | VERIFIED ref (decompile_function 0x00583295). Cross-refs BridgeRecord layout in BRIDGE_ZONE_LIFECYCLE §17. |
| `DAT_00ABAD30` / `DAT_00AA1028` | (read in `Apply_area_damage`) | Bridge-tile sub-index windows for low/high bridge-tile damage classification (`iVar19 == DAT_00abad30 + {0..3}` etc.). | DOC + xref this session (decompile_function 0x00489280). Out of pure-predicate scope (tile-damage). |

### P2.4 — vtable slot catalogue (the `+0x1B0` family)

`CheckBridgeTraversal @ 0x004D9C60` occupies slot `+0x1B0` in the FootClass virtual hierarchy. The four installer vtables (each verified via its bridge-relevant slot + ctor xref this session):

| Class | vtable base | `+0x1B0` slot addr | slot DWORD |
|---|---|---|---|
| AircraftClass | `0x007E22A4` | `0x007E2454` | `0x004D9C60` (read_memory) |
| FootClass | `0x007E8C94` | `0x007E8E44` | `0x004D9C60` |
| InfantryClass | `0x007EB058` | `0x007EB208` | `0x004D9C60` |
| UnitClass | `0x007F5C70` | `0x007F5E20` | `0x004D9C60` |

All four share the same function pointer (installed by FootClass, inherited). The function ignores its receiver. **Correction to pass 1:** this is NOT a CellClass slot.

### P2.5 — Burden-of-proof re-flag of this doc's own claims

- **C16 occupancy-bit Z-threshold** previously cited `Zthresh` as `DAT_00B1D0AC` UNCHECKED — now VERIFIED `= round(src×4)` (same deck-height as AoE). The `Mark`/`Clear` asymmetry (Clear lacks the `0x100` gate) remains DOC-only (BRIDGE_OCCUPANCY); re-flagged **UNCHECKED for live re-verify** of `Clear_Occupation @ 0x00744210` (next query: decompile_function 0x00744210, diff the `0x100` test vs `Mark_Occupation @ 0x007441B0`). The threshold globals it reads (`DAT_00B1D0AC`) are now resolved.
- **C18 draw-offset** stays DOC-only (RENDER), not re-verified this session — explicitly UNCHECKED-for-this-pass; the -16/-16 vs -16/-31 divergence is the live render decision for P7, unchanged.
- **`g_DirectionOffsets @ 0x0089F688`** confirmed cold-zero/runtime-init (cross-family Phase-1 finding folded in): it is real BSS lazy-init filled by `Foundation_direction_table_init @ 0x0049F2F0` with 8 `{i16 dx,i16 dy}` cell-neighbor deltas (N..NW clockwise, **cell units**), matching Rust `src/util/direction.rs DIRECTION_DELTAS`. `CheckBridgeTraversal`'s parent reconstruction reads it via `(dir-4)&7`. The §2.2 row is now tagged with the runtime-init caveat.

### P2.6 — TS-legacy / edge-case separation (re-confirmed, no new live decode)

No new TS-legacy paths surfaced in the sweep. The `IsWoodBridge` predicate is **active in YR** (wood bridges are standard YR map content; `Apply_area_damage` branches on it for low-bridge tile damage). The bridge-record table `DAT_0087F83C` and `ComputeBridgeZones` are active (run at every map load). The death-list `DAT_0087F8C0` remains dead TS-legacy (unchanged from §3).

### P2.7 — Slice impact of Pass 2

- **P0 (research gate):** now CLOSED except the trivial `BridgeStrength=1500` INI confirm. The three Z-threshold init formulas are resolved; warhead key is `Wall=`.
- **P1 (predicates):** ADD `is_wood_bridge_tileset` to the six predicates → **seven**; add test `is_wood_bridge_tileset_distinct_from_concrete_and_structural`.
- **DRIFT #6 (tileset vs structural):** promoted from latent/UNCHECKED to a **confirmed real drift**; the service must expose tileset predicates separately and the zone-build path is a consumer (P1/P3 scope, not just movement). Add a retire/route note for `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency`.
- **P4 (AoE layer):** C12's `BridgeHeight` is now a known engine constant — no longer blocked on a theater-init trace.
- **P5 (occupancy):** `Clear_Occupation` `0x100`-asymmetry re-flagged UNCHECKED for a live re-decode before P5 authority.

---

## 9. Sources & Verification Ledger

**Live-verified (re-decompiled 2026-06-04, base pass):**
- `CellClass::GetEffectiveHeight @ 0x00487D50` — `Level + ((flags>>7)&1)*4` (C4).
- `CellClass::IsBridge @ 0x00486750` — tileset window predicate (C5); reads `g_BridgeSet_TileSetBase`.
- `CellClass::IsLowBridgeCell/IsTubeCell @ 0x00484AB0` — `TubeIndex valid && LandType==10` (C6); reads `g_TubeCount`.
- `CheckBridgeTraversal @ 0x004D9C60` — full gate (C7–C11); confirmed parent-`0` reconstruct via `(dir-4)&7` over `g_DirectionOffsets`, `dir==-1` candidate seed, diff {0,1,4}, `0x200` gate, `*param_4=1` write. Matches BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK exactly.

**Live-verified THIS SESSION (verify-and-EXPAND pass, 2026-06-04) — gate closures:**
- `Apply_area_damage @ 0x00489280` — full body decompiled; C12 threshold `iVar10 + DAT_0089e864/2 < param_1[2]` (strict), `Wall=`/`WallAbsoluteDestroyer=` gates, `IsBridge`/`IsWoodBridge` tile-damage branches.
- `DAT_0089E864`/`DAT_00B1D0AC`/`DAT_00AC13BC` init thunks (`0x00489100`/`0x00735310`/`0x005F3880`) — each `round(src×4)`, round const `0x007E1738`=0.5 (read_memory). `DAT_0089E870` iso-geometry init block `0x00488F80..`.
- `WarheadTypeClass::ReadINI_Body @ 0x0075D3A0` — `+0x144` write @ `0x0075D508` from string `"Wall"` @ `0x0081AC58` (read_memory `57 61 6c 6c 00`).
- `CellClass::IsWoodBridge @ 0x00486770` — second tileset predicate (`g_WoodBridgeSet_TileSetBase`).
- `MapClass::ResolvePathCoord_BridgeAware @ 0x00583295` — co-use of `Flags & 0x100` + `IsBridge||IsWoodBridge`; reads bridge-record table `DAT_0087F83C`.
- vtable `+0x1B0` slot resolution — Aircraft/Foot/Infantry/Unit vtables (bases `0x007E22A4`/`0x007E8C94`/`0x007EB058`/`0x007F5C70`), slot DWORD `0x004D9C60` (read_memory 0x007e2454). NOT CellClass.
- Consumer xref sweep of `GetEffectiveHeight`/`IsLowBridgeCell`/`IsBridge` (get_xrefs_to) — see §P2.2.

**Doc-sourced (NOT re-read live this session — cited per claim):**
- `docs/research/bridges/00-system-models/BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` — damage RNG, state ladder, death-list TS-legacy, shroud poll, §10 offsets, §12 Rust status, §13 anomaly resolution.
- `docs/research/bridges/02-cell-state-layering-zones/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` — dual list/bit layers, OnBridge selector, Clear asymmetry, DropIn, confirmed parity gaps, Rust comparison.
- `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md` — caller matrix, A* explicit-parent vs runtime null-parent.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` — impact-Z layer selector, `DAT_0089E864`, bridge tile-damage phase, retry, INI table.
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_RENDERING_GHIDRA_REPORT.md` — Get_Draw_Offset, ZFudge/IsNearBridge, frame-variation table, -16/-31 vs -16/-16.
- `docs/research/bridges/07-cross-system-consumers/FUN_006E61F0_BRIDGE_LINKED_PREDICATE_GHIDRA_REPORT.md` — REFUTES the "bridge cell predicate" hypothesis; trigger-system bookkeeping.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md` — `0x8000` map gate.

**Rust files read (current state, 2026-06-04):**
- `src/bridge_re.rs`, `src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/pathfinding/core.rs` (PathCell predicates + `is_at_bridge_level`), `src/sim/combat/combat_aoe.rs` (referenced), `src/sim/occupancy.rs` (referenced via OCCUPANCY doc), `src/sim/world/bridge_orchestrator.rs` (referenced).

**RESOLVED this session (were UNCHECKED/BLOCKING P0):**
- ~~`DAT_0089E864` semantic + init~~ → `round(DAT_0089E870×4)` = full 4-level deck height; engine iso-geometry constant. VERIFIED.
- ~~`DAT_00B1D0AC` / `DAT_00AC13BC` init~~ → same `round(src×4)` deck-height value, separate copies. VERIFIED.
- ~~Warhead `+0x144` name~~ → `Wall=` (NOT `Bridge=`). VERIFIED (string `0x0081AC58`).
- ~~Whether any caller distinguishes tileset `IsBridge` (C5) from `Flags & 0x100` (C1)~~ → YES, distinguished; zone-graph/path-snap use tileset, movement/AoE/occupancy use structural. CONFIRMED DRIFT #6.

**Remaining UNCHECKED (non-blocking):**
- `BridgeStrength` retail value — INI confirm (`ini/rulesmd.ini:816` = 1500 per corpus; not re-read live this pass). Trivial.
- `Clear_Occupation @ 0x00744210` vs `Mark_Occupation @ 0x007441B0` — the `0x100`-asymmetry (C16) is DOC-only; re-verify live before P5 authority (next query: decompile both, diff the `0x100` test).
- `C18` draw-offset (RENDER) — not re-verified this pass; live render decision deferred to P7.
- `DAT_0089E870` exact iso-geometry numeric constants (cold-zero, lazy-init) — irrelevant, the resolved deck-height is a single constant; would need a live-init dump if ever wanted.
- Open port bug (not in scope to fix): SEAL/Tanya C4 on CABHUT no-op (port-side; gamemd has no Immune gate).

---

## Reviewer follow-ups (adversarial pass, 2026-06-04)

Verified live this pass: `0x00487D50` (C4 ✓ exact), `0x00486750` (C5 ✓ exact, `[base, base+0x10)`), `0x00484AB0` (C6 ✓ exact), `0x004D9C60` (C7–C11 ✓; one wording fix applied to C10 abs==0), `0x00489280` AoE selector (C12 ✓ — `ground + DAT_0089e864/2 < impact_z`, strict, confirmed). `FUN_006E61F0` refutation ✓ (walks a `+0xA0`/`+0xA8` tag list ORing `FUN_007271e0` results — not a cell/bridge predicate). BridgeStrength=1500 ✓ (`ini/rulesmd.ini:816`). All §7 retire-list Rust symbols exist as cited.

Patched this pass:
- **DRIFT #1 retracted (was a stale Rust ref).** `OccupancyGrid::rebuild` already selects the list layer from `on_bridge`, not `locomotor.layer`. Fixed in §4.1, §4.2#1, §7.
- **C10 abs==0 wording** corrected to "either bit missing," not "(0x100|0x200) absent."

Residual (left as notes, not patched — uncertain or refinement-only):
- **`g_DirectionOffsets @ 0x0089F688` reads all-zero in the cold image** (`read_memory 0x0089F688` → 32 bytes of 0x00). It is runtime-initialized like the three Z-threshold globals. The §2.2 row presents `0x0089F688` as established but does not flag the cold-zero/runtime-init caveat. The *symbol reference* in `0x004D9C60` is real; the static address value is not. Suggest adding the same "runtime-init; cold image 0" tag the Z-thresholds carry. (Not blocking — used only for parent reconstruction direction deltas, a known stable 8-entry table.)
- **DRIFT #2 causal description is slightly off.** The runtime move (movement_step.rs:1190-1210) computes a *single* `occupancy_layer` from the **post**-transition `projected_on_bridge_state` (line 1182, after `resolve_cell_transition_bridge_state`) and uses it for BOTH the remove-from-old and insert-into-new inside `move_entity`. The doc says it moves "with the path layer *before* resolve knows the post-transition OnBridge" — the layer is actually computed *after* resolve, but it is still the wrong (single, new) layer for the remove half. The DRIFT verdict stands; only the "before resolve" phrasing is imprecise.
- ~~**Vtable slot `+0x1B0` for `CheckBridgeTraversal` not directly confirmed.**~~ **RESOLVED in the 2026-06-04 expand pass.** The 4 `[DATA]` refs are slot `+0x1B0` in the **FootClass-family** vtables (Aircraft `0x007E22A4`, Foot `0x007E8C94`, Infantry `0x007EB058`, Unit `0x007F5C70`), NOT CellClass. `0x007E2454 − 0x007E22A4 = 0x1B0`; slot DWORD = `0x004D9C60` (read_memory). Function is `__stdcall` free (`RET 0x14`, receiver ignored). See header gate #4 / §2.4 / §P2.4. The earlier "(CellClass)" attribution was wrong.
