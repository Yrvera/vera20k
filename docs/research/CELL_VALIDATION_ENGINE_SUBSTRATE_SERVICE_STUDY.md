# Cell Validation Helper Family — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Scope:** the shared cell-legality predicates consumed by pathfinding fallback, building placement, unit spawn, scatter, aircraft landing, harvester dock approach, and AI site selection — `CellRect::CheckPassability`, `CellRect::CheckOccupancy`, `CellClass::CheckCellPassability`, `FootClass::Find_Nearby_Passable_Cell`, `MapClass::Get_CellClass` (dummy-cell fallback), `CellClass::RecalcZoneType`, and the live cell-list writers feeding them.
**Companion:** master roadmap item **#7 (map/cell substrate)** in `docs/plans/2026-05-29-core-engine-substrate-todo.md`. This study is item #7's **first slice** and **extends** the already-drafted boundary in `docs/research/CELLCLASS_SUBSTRATE_FIRST_MIGRATION_SLICE_GHIDRA_REPORT.md` (read-only `CellClass` facade over `OccupancyGrid` exposing `check_occupancy_rect` then `check_passability_rect`). It does **not** invent a competing architecture. Section shape mirrors `docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

**Confidence posture.** Two validator bodies + the dummy-cell helper + four function identities were **re-decompiled / re-identified LIVE this session** and are cited inline (`get_function_by_address` / `decompile_function`). **PASS-2 (2026-06-04) closed all three open gates LIVE** — `CheckCellPassability` body, the full FNPC body + selection source, the RTTI-0x24 class identity, and the save/load cell-list rebuild mechanism are now bit-VERIFIED (see §9.1 PASS-2 rows and the new `## Pass 2 — Expansion` section). The remaining DOC-ONLY rows are the speed-table dump contents and the zone matrix (both byte-verified in prior reports). **A load-bearing label correction was made this session:** the task brief labeled `0x004834A0` as `CheckOccupancy`; the binary shows `0x004834A0` is **`CellClass::CheckCellPassability`** (the per-cell callee of `CheckPassability`), and `CheckOccupancy` is at **`0x00586780`**. The brief's address↔name binding for those two is **WRONG**; the docs are right. Default verdict for any unproven equivalence is **DRIFT** — there is no internal-only escape hatch for active cell-validation behavior.

---

## Executive Summary

**Verdict: Rust has the validator *ingredients* scattered across three unrelated modules and no unified rectangle-aware `CheckPassability`/`CheckOccupancy` pair — the single most player-visible gap is that every "place this unit/building somewhere legal nearby" decision (war-factory exit, rally, chrono-miner return, scatter, drop-pod, AI site) routes through `PathGrid::is_walkable` (one boolean per cell/layer) instead of the engine's typed 9-arg passability rectangle + separate occupancy rectangle, so spawn/fallback cells differ from gamemd's.** Three structural DRIFTs compound this: (1) the engine separates **passability** (terrain/zone/height/occupation-bits, no playfield corner check) from **occupancy** (object-list/reservation/cell-blocker bytes + a final 4-corner playfield containment check), while Rust fuses both coarsely into the walkable grid + `OccupancyGrid`; (2) the engine's cell lookup is a **fixed 512-wide `y*0x200+x` index with range `[0,0x3FFFF]` and a non-null dummy-cell fallback** that callers keep using after substitution, while Rust indexes by the *loaded-map width* and returns `None`/`DEFAULT_BLOCKED_CELL`; (3) the nearby-cell search is the engine's **diamond-ring `Find_Nearby_Passable_Cell` with frame-counter pseudo-random candidate selection**, while Rust uses a different `nearest_walkable_around` ring with deterministic first-match. The proposed replacement is an additive, **read-only `CellClass` facade in `sim/` over the existing `OccupancyGrid` + `PathGrid`** exposing `check_passability_rect(...)`, `check_occupancy_rect(rect, reservation)`, a `get_cellclass_fallback` parity accessor, and a `find_nearby_passable_cell` port — built shadow-first, tested at the substrate boundary before any caller migrates, and coordinated at the seam with the pathfinding sibling study (these validators are the primitives `Can_Enter_Cell` and A* fallback consume). **None of P0–P3 changes a hashed bit**; the only state-hash-affecting change is the Find_Nearby selection cell choice (P6), gated behind a `SNAPSHOT_VERSION 17→18` bump. **PASS-2 closed the former BLOCKING gate: the FNPC selection source is the deterministic `g_CurrentFrameCounter` (`0x00A8ED84`), NOT an RNG draw, so it is lockstep-safe and does not perturb either RNG stream.** A second save/load DRIFT was surfaced PASS-2: cell object-list order is serialized verbatim in gamemd, so a Rust port that rebuilds occupancy from `EntityStore::values()` on load will DRIFT unless it serializes cell-list order directly (C22).

---

## Table of Contents

- §1. Verified active-YR responsibilities of the cell-validation family
- §2. Full inventory (functions, fields, globals, tables, vtable/COM, legacy/TS)
- §3. Active vs inactive/legacy (TS) split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements C1–C20)
- §6. Rust-native replacement boundary
- §7. Old ad hoc Rust logic to retire / fold in
- §8. Migration slices + acceptance tests (P0–P7)
- §9. Sources & Verification Ledger

---

## 1. Verified active-YR responsibilities of the cell-validation family

This is what the family **owns** in a normal YR skirmish — the player-observable contract a Rust replacement must reproduce. Each line is the *behavior*, not the C++ structure.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **Rectangle passability**: `CellRect::CheckPassability` walks a `width×height` rectangle (outer x, inner y) and asks each cell — via `CheckCellPassability` — whether it is passable for a supplied SpeedType / required-zone-id / MovementZone / required-height-or-level / bridge-aware flag, with an optional overlay-reject precheck. Full-rect success only if ALL cells pass. | VERIFIED (live) | `decompile_function 0x0056e7c0` this session (loop shape, `y*0x200+x`, overlay-reject branch, `CheckCellPassability` call). |
| R2 | **Per-cell passability**: `CellClass::CheckCellPassability @ 0x004834A0` owns SpeedType==4 (Winged) fast-success, zone-id comparison via `MapClass::GetZoneID`, required-height/bridge-layer rules (`level` vs `level+4`), `+0x124` ground vs `+0x128` bridge occupation-byte selection, wall-overlay exceptions, and the `SpeedType + LandType*9` speed-table `0.0`-rejects lookup. | VERIFIED (live identity) | `get_function_by_address 0x004834a0` → `CellClass__CheckCellPassability`; behavior DOC-ONLY (validator report §3.1). |
| R3 | **Rectangle occupancy**: `CellRect::CheckOccupancy @ 0x00586780` walks a rectangle rejecting cells with an RTTI-0x24 ground-list object, an optional `+0xDC` per-house/site reservation bit, a `+0x44 != -1` overlay, a `+0x4C != 0` blocker, a `+0x11C != 0` special/slope byte, or a `WhatAmI()==6` building occupant; then requires **all four rectangle corners inside the playfield** (`IsRectInPlayfield(rect,1)`). NOT terrain passability — no SpeedType/zone read. | VERIFIED (live) | `decompile_function 0x00586780` this session (full blocker chain + final `MapClass__IsRectInPlayfield`). |
| R4 | **Reservation-layer arg semantics**: `CheckOccupancy(rect, layer)` — `layer == -1` zeroes the mask and **skips** the `+0xDC` reservation test; otherwise `mask = 1 << (layer & 0x1F)` and `+0xDC & mask != 0` rejects. AI/site helper passes a house index; nearby-cell calls pass `-1`. | VERIFIED (live) | `decompile_function 0x00586780`: `if (param_2==-1) local_8=0; else local_8 = 1<<((byte)param_2 & 0x1f);`. |
| R5 | **Nearby-passable-cell search**: `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` — diamond-ring expanding search seeded at an input cell, calling `CheckPassability` for every candidate and optionally `CheckOccupancy(rect, -1)`; selects via frame-counter pseudo-random `candidates[frame_counter % count]` when no target cell is given, preferring "direct" candidates; writes null cell `{0,0}` on no-candidate. | VERIFIED (live identity) | `get_function_by_address 0x0056dc20` → `FootClass__Find_Nearby_Passable_Cell`; search/selection DOC-ONLY (chrono-return report §4, FNPC report). |
| R6 | **Cell lookup + dummy fallback**: `MapClass::Get_CellClass @ 0x005657A0` converts a packed coord to `CellClass*` via fixed `y*0x200+x`, range `[0,0x3FFFF]`; on OOB or null pointer it returns a **non-null dummy cell `DAT_00ABDC50`** and writes the requested coord to `DAT_00ABDC74` (dummy+0x24). Callers keep dispatching on the dummy. The validators inline the same index/dummy logic. | VERIFIED (live) | `decompile_function 0x005657a0` this session (`param_2[1]*0x200 + *param_2`, range check, `DAT_00abdc74 = *param_2; puVar2 = &DAT_00abdc50`). Array base = `*(this+0x13c)`. |
| R7 | **Reduced-ZoneType classification**: `CellClass::RecalcZoneType @ 0x00483C80` writes `CellClass+0x4C` (reduced ZoneType column 0..7), the input to the zone-passability matrix that backs the `required_zone_id` comparison; companion base `LandType` at `+0x48`. | VERIFIED (live identity) | `get_function_by_address 0x00483c80` → `CellClass__RecalcZoneType`; column semantics DOC-ONLY (RecalcZoneType report, ZONE_PASSABILITY_MATRIX report). |
| R8 | **Live cell-list membership** feeding the occupancy reads: `CellClass+0xE4` ground list / `+0xE8` bridge-deck list, written by `AddContent`/`RemoveContent` selecting the layer from `ObjectClass+0x8C` (OnBridge); `+0x124/+0x128` occupation bits written by `Mark_Occupation`/`Clear_Occupation`; `+0x100` hidden building counter (CanHideThings). These are the data the validators read. | VERIFIED (writers) | live-object-list-writers report §3 (`0x0047E8A0`/`0x0047EA90`/`0x005683C0`/`0x007441B0` etc.). |

---

## 2. Full inventory

### 2a. Functions (validators, helpers, search)

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| `CellRect::CheckPassability` | `0x0056E7C0` | Rectangle passability wrapper; 9 stack args (`RET 0x24`); ALL-cells success | YES | VERIFIED live (`get_function_by_address`, `decompile_function 0x0056e7c0`) |
| `CellClass::CheckCellPassability` | `0x004834A0` | Per-cell passability callee (SpeedType/zone/height/occupation-byte/wall/speed-table) — **NOT CheckOccupancy** | YES | VERIFIED live identity (`get_function_by_address 0x004834a0`); body DOC-ONLY |
| `CellRect::CheckOccupancy` | `0x00586780` | Rectangle occupancy/blocker/reservation + playfield-corner validator; 2 args (`RET 0x8`) | YES | VERIFIED live (`get_function_by_address`, `decompile_function 0x00586780`) |
| `FootClass::Find_Nearby_Passable_Cell` (FNPC) | `0x0056DC20` | Diamond-ring nearby-cell search; sole caller of `CheckPassability`; calls `CheckOccupancy(rect,-1)` when its final flag set | YES | VERIFIED live identity (`get_function_by_address 0x0056dc20`); algorithm DOC-ONLY |
| `MapClass::Get_CellClass` | `0x005657A0` | Coord→`CellClass*`; `y*0x200+x`, `[0,0x3FFFF]`, dummy `DAT_00ABDC50` fallback | YES | VERIFIED live (`decompile_function 0x005657a0`) |
| `CellClass::RecalcZoneType` | `0x00483C80` | Writes reduced ZoneType `+0x4C` (matrix column) + `+0x48` LandType | YES | VERIFIED live identity (`get_function_by_address 0x00483c80`); column semantics DOC-ONLY |
| `MapClass::GetZoneID` | `0x0056D230` | Zone id for a cell+MovementZone+bridge-aware; compared to `required_zone_id` | YES | DOC-ONLY (validator report §3.1) |
| `MapClass::IsRectInPlayfield` | `0x00578390` | 4-corner playfield containment (NW/NE/SW/SE using `x+w-1`,`y+h-1`); CheckOccupancy tail | YES | DOC-ONLY (validator report §3.2); call confirmed in live `0x00586780` body |
| `FUN_0047C550` (ground-list RTTI scan) | `0x0047C550` | `__thiscall` (cell = ECX `this`, `param_2=0` selects `+0xE4`); rejects on RTTI `0x24` | YES | DOC-ONLY (validator report; thiscall-receiver correction 2026-05-28); call confirmed in live `0x00586780` |
| `Look_up_building_in_cell` | `0x0047C520` | Finds `WhatAmI()==6` building on `+0xE4` | YES | DOC-ONLY; call confirmed in live `0x00586780` |
| `FUN_005060B0` (AI building/site helper) | `0x005060B0` | Calls `CheckOccupancy(expanded_rect, HouseClass+0x30)` + FNPC; AIBaseSpacing footprint expand (`Rules+0x1460`), `g_DirectionOffsets`, `atan2`/`ftol`, FoundationW/H, `Rules+0xe0c` distance cap | YES (AI) | **VERIFIED body PASS-2** (`decompile_function 0x005060B0`) — confirms `CheckOccupancy` reservation arg = `HouseClass+0x30` house index |
| `FootClass::Find_Passable_Cell_Near_Unit` | `0x00500200` | **Sibling** nearby-search wrapper: draws `Random__RandomRanged(1,4)` to pick a candidate-direction variant, then calls FNPC with that zone. RNG lives HERE, not in FNPC. | YES | **VERIFIED PASS-2** (`decompile_function 0x00500200`) |
| `TerrainClass::What_Am_I` | `0x0071D300` | `return 0x24` — identifies the CheckOccupancy RTTI-0x24 ground-list blocker as TerrainClass | YES | **VERIFIED PASS-2** (`decompile_function 0x0071D300`) |

### 2b. Validator stack signatures (DOC-sourced, loop/branch shapes confirmed live)

```text
// CheckPassability — RET 0x24, nine 32-bit args (validator report §3.1; loop+overlay branch live-confirmed)
bool CheckPassability(
    CellStruct* top_left,          // arg1: packed signed low-16 x / next-16 y
    int rect_width,                // arg2
    int rect_height,               // arg3
    int speed_type,                // arg4  (TechnoType+0x67C source; 4 = Winged fast-pass)
    int required_zone_id,          // arg5  (-1 skips zone-id comparison)
    int movement_zone,             // arg6  (TechnoType+0x5B4 / matrix row family — NOT SpeedType)
    int required_height_or_level,  // arg7  (-1 = unrestricted; FNPC always passes -1)
    int bridge_layer_arg,          // arg8  (to GetZoneID + height/layer logic)
    int reject_any_overlay         // arg9  (nonzero rejects Cell+0x44 != -1 before CheckCellPassability)
)

// CheckOccupancy — RET 0x8 (live-confirmed body)
bool CheckOccupancy(CellRect* rect, int reservation_layer_or_house_index)  // -1 => skip +0xDC; else 1<<(arg&0x1F)
```

### 2c. CellClass fields read by the validators

| Off | Field | Read by | Evidence |
|---|---|---|---|
| `+0x24/+0x26` | cell map coord (also `DAT_00ABDC74` = dummy+0x24 for fallback) | GetZoneID input; dummy coord store | live `0x005657a0`/`0x00586780` (coord write to `DAT_00abdc74`) |
| `+0x44` | OverlayTypeIndex (`-1` = none) | CheckPassability overlay-reject; CheckOccupancy reject `!= -1` | live `0x0056e7c0` (`this->OverlayTypeIndex != -1`), `0x00586780` (`+0x44 != -1`) |
| `+0x48` | base LandType (RecalcZoneType companion) | speed-table land row | DOC-ONLY (RecalcZoneType report) |
| `+0x4C` | reduced ZoneType column (RecalcZoneType output) / CheckOccupancy blocker `!= 0` | matrix column; occupancy reject | live `0x00586780` (`+0x4C != 0` reject); column DOC-ONLY |
| `+0xDC` | per-house/site reservation bitmask (`1<<(layer&0x1F)`) | CheckOccupancy when layer != -1 | live `0x00586780` (`local_8 & *(uint*)(puVar4+0xdc)`) |
| `+0xE4` | ground object list head (FirstObject) | RTTI-0x24 scan + building lookup | DOC-ONLY (live-object-list report) |
| `+0xE8` | bridge/deck object list head (AltObject) | bridge-layer membership | DOC-ONLY |
| `+0x11B` | base cell level/height byte | required-height rules | DOC-ONLY (validator report) |
| `+0x11C` | special/slope byte (nonzero blocks occupancy) | CheckOccupancy reject | live `0x00586780` (`puVar4[0x11c] != '\0'`) |
| `+0x124` | ground occupation bits | CheckCellPassability ground path | DOC-ONLY |
| `+0x128` | bridge/deck occupation bits | CheckCellPassability bridge path | DOC-ONLY |
| `+0x140 & 0x100` | structural bridge flag | height/layer + bit-field selection | DOC-ONLY |
| `+0x100` | hidden building occupancy counter (CanHideThings) | not a list-membership blocker | DOC-ONLY (foundation-occupy report) |

### 2d. Globals / singletons

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| Cell array base (`MapClass+0x13C`) | `*(this+0x13c)` | Pointer-to-pointer cell array indexed `y*0x200+x` | YES | live `0x005657a0` (`*(int*)(param_1 + 0x13c)`) |
| `g_CellArray_Base` (validator-inlined) | (Ghidra symbol) | Same array, used directly inside both validators (no MapClass `this`) | YES | live `0x0056e7c0` / `0x00586780` (`g_CellArray_Base + iVar*4`) |
| Dummy cell | `DAT_00ABDC50` | Non-null CellClass-compatible fallback for OOB/null | YES | live `0x005657a0`/`0x00586780`; init field values DEFERRED (not dumped) |
| Dummy cell coord slot | `DAT_00ABDC74` (= dummy+0x24) | Stores the requested coord on fallback | YES | live `0x005657a0`/`0x00586780` |
| FNPC no-candidate sentinel | `DAT_00ABD480` / `DAT_00B1CFB8` | Both read `{0,0}`; written/compared as null-cell output AND as the input target-cell null check (selects C16 frame-counter path vs C16b distance path) | YES | DOC (chrono-return report §5); compare confirmed live `0x0056E690` |
| `g_CurrentFrameCounter` | `0x00A8ED84` | **FNPC C16 selection modulo source** (`frame % count`); per-tick game logic counter, incremented once/tick in `Main_Tick @ 0x0055DE81`; also drives anim/crate/lightning/laser timers. NOT an RNG stream. | YES | **VERIFIED PASS-2** (`get_xrefs_to ram:0x00A8ED84` sole [WRITE] in Main_Tick; `disassemble_function 0x0056DC20` tail `MOV EAX,[0x00A8ED84]; IDIV ECX`) |
| Zone-passability matrix | `0x0082A594` | `int[13][8]`; rows = MovementZone 0..12, cols = reduced ZoneType 0..7; only value `1` passes | YES | DOC-ONLY (ZONE_PASSABILITY report); mirrored in Rust `MOVEMENT_ZONE_PASSABILITY` |
| Speed/Land table | `g_SpeedType_LandType_Table` | `[speed_type + LandType*9]`, exact `0.0` rejects | YES | DOC-ONLY (SPEEDTYPE_LANDTYPE report) |

### 2e. Vtable / COM slots used

| Slot | Class | Target | Role | Evidence |
|---|---|---|---|---|
| `vtable+0x480` | FootClass | `Set_Destination` | FNPC result consumer (Set_Destination(NULL,1) on no-cell) | DOC (chrono-return report §5) |
| `WhatAmI()` (RTTI) | ObjectClass | returns `6` for buildings | CheckOccupancy building reject | DOC-ONLY |
| `vtable+0x2C` `WhatAmI()` | ObjectClass (pure virtual `0x004C9150`) | returns RTTI enum: Unit=1, Aircraft=2/4, Building=6, Infantry=0xF, Overlay=0x14, **Terrain=0x24** | both CheckOccupancy ground-list rejects (`0x24` and `6`) dispatch through it | **VERIFIED PASS-2** (`decompile_function 0x0047C550`/`0x0047C520` call `*(*piVar1+0x2c)`; ABSTRACTCLASS report slot 11) |
| RTTI `0x24` tag | **TerrainClass** (`0x0071D300` → `return 0x24`, vtable `0x007F5200`) | matched by `FUN_0047C550` | CheckOccupancy ground-list reject = tree/rock/inert terrain object present | **VERIFIED PASS-2** (class identity RESOLVED; `decompile_function 0x0071D300`) |

These are read for behavior only; the Rust port does NOT replicate vtable plumbing — it dispatches on the entity's category/role in Rust-native types.

### 2f. Legacy / dormant TS paths in this surface

| Item | Status | Evidence |
|---|---|---|
| Tunnel / subterranean MovementZone (row 6) | Matrix row exists; TS legacy. Subterranean locomotion is not in YR — the row is dead for movers but must stay in the matrix table for byte-parity. Do NOT design a subterranean code path. | matrix row 6 present in binary + Rust; `feedback_no_tunnel_subterranean` |
| Fog-of-war "previously seen" darkening | Not a cell-validation field; OFF by stock-YR default (`SpecialFlags & 0x1000`). The validators read no fog gate. | validator report OQ-14 (no TS/Fog top-level gate found) |
| `CellClass+0x128` bridge-deck occupation | Active in standard YR bridge play; NOT TS-dead. Conditional on structural bridge flag — keep. | live-object-list report §3.3 |
| Dummy-cell `(0,0)` constant assumption | Anti-pattern, NOT engine behavior: the dummy stores the *requested* coord at `DAT_00ABDC74`. Do not model a constant-coord dummy. | live `0x005657a0` |

---

## 3. Active vs inactive/legacy split

### ACTIVE-YR — must be reproduced (the player-observable contract)

| Item | One-line rationale |
|---|---|
| CheckPassability rect (9-arg typed config, ALL-cells success, overlay-reject precheck) | Every nearby-cell/spawn candidate's terrain legality; fires on every blocked-destination fallback. |
| CheckCellPassability per-cell (SpeedType==4 fast-pass, zone-id, required-height/bridge-layer, `+0x124`/`+0x128` selection, wall exception, speed-table `0.0` reject) | The per-cell terrain rule; defines which cells a wheeled/tracked/foot/float unit may stand on. |
| CheckOccupancy rect (RTTI-0x24, `+0xDC` reservation, `+0x44`/`+0x4C`/`+0x11C`, building occupant, final 4-corner playfield) | Whether a cell is *clear* to place into; AI base spacing + exit/spawn occupancy. |
| Reservation-arg `-1` skip vs `1<<(layer&0x1F)` | Distinguishes nearby-cell (skip) from AI-site (house-index) occupancy; wrong arg corrupts AI placement. |
| Find_Nearby_Passable_Cell diamond-ring + frame-counter random selection + null-cell on failure | The actual cell a freed/spawned/relocated unit ends up in; visible every war-factory exit, scatter, chrono-return, drop. |
| Get_CellClass `y*0x200+x` / `[0,0x3FFFF]` / non-null dummy + requested-coord store | Cell addressing + the fallback that lets callers keep dispatching on OOB probes. |
| RecalcZoneType reduced-ZoneType `+0x4C` (matrix column) + zone matrix (only `1` passes) | Backs `required_zone_id` reachability comparison; wrong column = wrong zone connectivity. |
| Live `+0xE4`/`+0xE8` selected-list membership + `+0x124`/`+0x128` occupation bits | The data the validators read; cell-list order is player-visible (nearest-object, area damage, collapse). |

### INACTIVE / LEGACY (TS) / DORMANT — do NOT implement as default

| Item | One-line rationale |
|---|---|
| Subterranean MovementZone row (matrix row 6) | TS legacy; no YR subterranean locomotor. Keep the table row for parity, no code path. |
| Fog-of-war darkening gate | OFF by stock-YR default; not a cell-validation field. |
| Dummy-cell `(0,0)` constant coord | Engine stores the *requested* coord; a constant-coord dummy is an anti-pattern, not behavior. |
| `Buildable=` as a passability emulation | Separate building-placement predicate (`0x0047C620`); NOT one of these validators. |

### DEFERRED (active in YR, out of this slice's scope — leave a clean seam)

| Item | One-line rationale |
|---|---|
| AI building/site placement (`FUN_005060B0`, AIBaseSpacing footprint expand `Rules+0x1460`, `+0xDC` house-index reservation) | AI-house only; `feedback_no_ai_yet`. The `check_occupancy_rect(rect, house_index)` arg + reservation map is designed as a seam, internals deferred. **PASS-2 confirmed** the reservation arg is `HouseClass+0x30` (house index). |
| Native save/load `+0xE4`/`+0xE8` rebuild order | **RESOLVED PASS-2 (C22): order is serialized verbatim, NOT rebuilt** (CellClass heads + object `+0x30` NextObject swizzle); zone `+0x4C` IS rebuilt. Out of this slice's *implementation* scope, but the contract is now known — Rust must serialize cell-list order directly, not re-derive from `EntityStore`. |

---

## 4. Comparison against the current Rust architecture

The validators' Rust equivalents are split across **three unrelated modules with no unified rectangle predicate**: `PathGrid` (terrain walkability), `OccupancyGrid` (dynamic entity membership), and ad-hoc per-caller spawn/placement checks. There is no `CheckPassability`/`CheckOccupancy` pair, no `Find_Nearby_Passable_Cell` port, and no `Get_CellClass` dummy-fallback contract. This is exactly the "collapsed into one boolean walkable grid" anti-pattern master-TODO #7 names.

### 4.1 Structural map

| Concern | gamemd authority | Current Rust | Verdict |
|---|---|---|---|
| Rectangle passability | `CheckPassability(9 args)` → `CheckCellPassability` | `PathGrid::is_walkable(x,y)` (one bool) + `is_walkable_on_layer` (`core.rs:1613/1622`); zone helpers in `passability.rs` | **DRIFT (structural)** — no rect, no SpeedType/zone/height/overlay-reject threading |
| Per-cell terrain rule | SpeedType==4 fast-pass, `+0x124`/`+0x128`, wall exception, `SpeedType+LandType*9` table | `PathGrid.cells[idx].ground_walkable`/`bridge_walkable` precomputed bool; matrix in `passability.rs` used only by zone flood-fill | **DRIFT** — collapsed to a precomputed boolean; equivalence unproven |
| Rectangle occupancy | `CheckOccupancy(rect, layer)` (RTTI-0x24, `+0xDC`, `+0x44/+0x4C/+0x11C`, building, playfield corners) | `OccupancyGrid` (dynamic entities + sub-cell + list order only) | **DRIFT** — no cell-blocker bytes, no reservation bits, no playfield-corner check |
| Reservation `+0xDC` (`-1` vs house index) | per-house/site bitmask, layer-arg gated | absent | **MISSING** |
| Cell lookup | `y*0x200+x`, `[0,0x3FFFF]`, non-null dummy + requested-coord store | `PathGrid::cell` indexes by **loaded-map width**, returns `None`/`DEFAULT_BLOCKED_CELL` (`core.rs:1642`); `ResolvedTerrainGrid` returns `None` | **DRIFT** — different index width AND `None` vs non-null dummy |
| Nearby-passable search | diamond-ring + frame-counter random select + null-cell sentinel | `find_spawn_cell_near_structure` → `nearest_walkable_around(grid, …, 12, …)` ring, deterministic first-match (`production_spawn.rs:237/290`) | **DRIFT** — different ring shape + selection rule |
| Reduced ZoneType | `RecalcZoneType` → `+0x4C` column | `ResolvedTerrainCell.zone_type` + `MOVEMENT_ZONE_PASSABILITY[13][8]` (`passability.rs:115`) | **OK on the matrix table** (byte-verified); DRIFT on who writes/uses the column |

### 4.2 Behavior table (default DRIFT)

| # | gamemd behavior (verified) | Current Rust | Verdict | Player-visible? | Trigger frequency |
|---|---|---|---|---|---|
| 1 | **Passability is a rect with typed inputs**; ALL cells must pass; `reject_any_overlay` is a caller flag (R1, live `0x0056e7c0`) | `PathGrid::is_walkable` single bool, no rect, no overlay-reject flag | **DRIFT** | YES — spawn/fallback cell choice differs when overlay/zone/height matter | every blocked-destination fallback (exit, rally, scatter, chrono-return) |
| 2 | **Cell index is fixed `y*0x200+x`, range `[0,0x3FFFF]`** independent of playfield width (R6, live `0x005657a0`) | `PathGrid::cell` / `is_walkable` index `= y*self.width + x`, bounds `x<width && y<height` (`core.rs:1614/1617`) | **DRIFT** | indirectly — same loaded map yields same cells, but edge/OOB probes differ | OOB probes near map edge; AI/large maps |
| 3 | **OOB/null lookup returns a NON-null dummy cell** + stores requested coord; callers keep dispatching (R6, live `0x005657a0`) | returns `None` / `DEFAULT_BLOCKED_CELL`; callers short-circuit on `None` (`core.rs:855/1642`) | **DRIFT** | borderline — changes whether a caller's post-lookup field reads fire | any validator probing OOB |
| 4 | **CheckOccupancy ≠ passability**: rejects object-list/reservation/cell-blocker bytes + final 4-corner playfield, NO terrain read (R3, live `0x00586780`) | no equivalent; `OccupancyGrid` is dynamic entities only, no playfield-corner check | **MISSING** | YES — placement legality (slope/special cell, reservation, edge rect) differs | every spawn/exit/AI-site occupancy test |
| 5 | **Reservation arg `-1` skips `+0xDC`; else `1<<(layer&0x1F)`** (R4, live `0x00586780`) | no reservation surface at all | **MISSING** | YES (AI base spacing) | every AI placement (deferred) + correctness of nearby `-1` path |
| 6 | **FNPC diamond-ring + frame-counter random candidate select** `candidates[frame % count]`, direct preferred, null-cell on fail (R5, FNPC report) | `nearest_walkable_around` different ring, deterministic first-match | **DRIFT** | YES — the exact cell a unit ends up in differs | every war-factory exit / scatter / chrono-return |
| 7 | **CheckCellPassability SpeedType==4 (Winged) fast-success** before any check (R2, validator report) | aircraft handled by separate layer logic; not via a shared cell predicate | **DRIFT (structural)** | borderline | every aircraft cell test |
| 8 | **Zone-id comparison uses MovementZone row, not SpeedType**; only matrix value `1` passes (R2/R7) | matrix table correct (`passability.rs:115`); but `zone_layer_for_speed_type` compatibility shim still present (`passability.rs:149`) | **OK (table)** / DRIFT (a SpeedType shim coexists) | YES if the shim is used on a validator path | depends on call routing |
| 9 | **Bridge occupation `+0x128` selected only on structural bridge cells**; required-height `-1` still selects bridge bits (R2) | `PathGrid` has separate `bridge_walkable`; no `+0x124`/`+0x128` byte selection by required-height | **DRIFT** | YES (bridgeheads) | every bridge-deck spawn/relocate |
| 10 | **CheckOccupancy zero-size rect** still runs final playfield-corner check (`width-1`/`height-1`); CheckPassability zero-size returns true (validator OQ-9, live loop guards) | no rect API → undefined | **MISSING** | borderline (degenerate rect) | rare; correctness boundary |

### 4.3 What is MISSING outright

- **No `CheckOccupancy` analog** — no rect-aware object-list/reservation/cell-blocker-byte + playfield-corner predicate (#4/#5/#10).
- **No `Find_Nearby_Passable_Cell` port** — Rust's `nearest_walkable_around` is a different ring with a different selection rule (#6). The chrono-miner-return and war-factory-exit cells are therefore not gamemd-identical.
- **No `Get_CellClass` dummy-fallback contract** — Rust uses `None`/`DEFAULT_BLOCKED_CELL`; the engine's non-null dummy + requested-coord store is not modeled (#2/#3).
- **No `+0xDC` reservation surface** — AI base spacing has no home (#5).
- **No fixed `y*0x200+x` parity index** — Rust indexes by loaded-map width (#2).

### 4.4 What is OK / partial (do not "fix" without proof)

- **`MOVEMENT_ZONE_PASSABILITY[13][8]`** matches the verified native dump byte-for-byte (`passability.rs:115`, test `matrix_matches_verified_native_dump`). Keep.
- **`OccupancyGrid` list order** (structures append / non-structures prepend within a layer) matches the verified `+0xE4`/`+0xE8` insertion rule (`occupancy.rs:200-217`; live-object-list report §4 "keep"). Keep — the new facade reads it, does not replace it.
- **`zone_layer_for_speed_type`** (`passability.rs:149`) is a compatibility shim that is NOT the validator's zone-row source (validator uses MovementZone). It is a retire-candidate IF any cell-validation path reaches it (§7).

---

## 5. gamemd-native behavior contract (testable statements)

Each is a TESTABLE invariant the substrate must satisfy. These are the §8 acceptance-test targets. Default verdict DRIFT until a test proves equivalence.

**C1 — Cell index is fixed-width.** Cell linear index = `(short)y * 0x200 + (short)x`; valid range `[0, 0x3FFFF]`. This 512-wide stride is independent of the loaded map's playfield width. *(R6, live `0x005657a0`/`0x0056e7c0`/`0x00586780`.)*

**C2 — OOB/null lookup returns a non-null dummy.** A lookup whose index is out of `[0,0x3FFFF]` or whose cell pointer is null returns the dummy cell (CellClass-compatible, non-null) and writes the requested packed coord into the dummy's coord slot (`+0x24` / `DAT_00ABDC74`); the caller may still read fields off it. *(R6/C2, live `0x005657a0`.)*

**C3 — CheckPassability is a rectangle AND-fold.** It walks `x = 0..width-1` outer, `y = 0..height-1` inner; returns true iff every in-rect cell passes `CheckCellPassability`. `width<=0` or `height<=0` returns true without checking any cell. *(R1, live `0x0056e7c0` loop guards.)*

**C4 — Overlay-reject is a caller flag.** When `reject_any_overlay != 0`, a cell with `OverlayTypeIndex (+0x44) != -1` fails the rect immediately, before `CheckCellPassability`. FNPC's chrono-return call passes `0` (overlays NOT rejected). *(R1, live `0x0056e7c0`: `(in_stack_00000024 != 0) && (this->OverlayTypeIndex != -1)`.)*

**C5 — SpeedType==4 fast-pass.** `CheckCellPassability` returns true immediately for `speed_type == 4` (Winged/Fly), skipping zone, height, occupation, overlay-wall, and speed-table checks. *(R2, validator report §3.1.)*

**C6 — Zone-id comparison is MovementZone-rowed.** If `required_zone_id != -1`, `MapClass::GetZoneID(cell, movement_zone, bridge_aware)` must equal it; the zone-passability matrix `0x0082A594` is `int[13][8]` (rows = MovementZone 0..12, cols = reduced ZoneType 0..7) and **only value 1 passes** (2 and 3 block). `required_zone_id == -1` skips the comparison. *(R2/R7, ZONE_PASSABILITY report; Rust `MOVEMENT_ZONE_PASSABILITY` is the byte-identical table.)*

**C7 — Speed/land table reject. [PASS-2 VERIFIED]** Checks `((float)(&g_SpeedType_LandType_Table)[speed_type + LandType*9] == 0.0)` and rejects — but **only when the bridge/AltOccupation path was NOT selected** (`&& !bVar2`): a structural-bridge cell whose required-height selected `AltOccupationFlags` **bypasses the speed-table `0.0` reject entirely**. The `0.0` constant is `FLOAT_007E1748` (`read_memory ram:0x007E1748` = `00000000`). Accepted wall overlays (`OverlayType+0x2A8` "is wall" AND, for movement-zones `1`/`4`, `OverlayType+0x22D`) force `LandType = 0 (Clear)` before the lookup; wall overlays in movement-zones `2,3,8,0xC` are accepted unconditionally, others reject. *(R2; `decompile_function 0x004834A0`; `read_memory ram:0x007E1748`.)*

**C8 — Occupation-byte selection by required-height + bridge flag. [PASS-2 VERIFIED]** `CheckCellPassability` selects `AltOccupationFlags` (`+0x128`, sets `bVar2=true`) only when `(required_height == -1 OR required_height == cell.Level+4)` AND the structural bridge flag (`Flags & 0x100`) is set; otherwise `OccupationFlags` (`+0x124`, `bVar2=false`). **Two extra occupation-mask modifier args exist** (the two bytes after speed_type): when arg `in_stack_00000008 != 0` the selected byte is masked `& 0xE0` (high 3 bits = sub-cell/center occupancy); when `in_stack_0000000c != 0` it is masked `& 0x5F`. The wrapper `CheckPassability` passes both as zero, so the full selected byte must be zero to pass on the wrapper path; locomotor callers that pass them nonzero check only a sub-cell subset. A non-zero (post-mask) selected byte rejects. *(R2; `decompile_function 0x004834A0`; BRIDGE_OCCUPANCY report.)*

**C9 — CheckOccupancy is not passability.** It performs no SpeedType / MovementZone / LandType / zone read. It only checks object-list/reservation/cell-blocker fields and playfield corners. *(R3, live `0x00586780`.)*

**C10 — CheckOccupancy blocker order.** For each in-rect cell, in order: (a) RTTI-0x24 ground-list object present → reject; (b) `(+0xDC & mask) != 0` → reject (mask per C12); (c) `+0x44 != -1` → reject; (d) `+0x4C != 0` → reject; (e) `+0x11C != 0` → reject; (f) `WhatAmI()==6` building on `+0xE4` → reject. *(R3, live `0x00586780` decompile.)*

**C11 — CheckOccupancy final playfield containment.** If no in-rect cell is a blocker, the result is `IsRectInPlayfield(rect, 1)`, which tests all four corners (NW, NE, SW, SE) using `x+width-1` and `y+height-1`. A zero/negative-size rect skips the blocker scan but still runs this corner check. *(R3/C11, live `0x00586780`: `MapClass__IsRectInPlayfield(param_1,1)`; corners DOC-ONLY.)*

**C12 — Reservation mask.** `CheckOccupancy(rect, layer)`: `layer == -1` → mask `0`, the `+0xDC` reservation test is skipped; otherwise mask `= 1 << (layer & 0x1F)`. A non-`-1` negative arg aliases through `& 0x1F`. *(R4, live `0x00586780`: `1 << ((byte)param_2 & 0x1f)`.)*

**C13 — FNPC always passes required-height `-1` to CheckPassability.** Find_Nearby_Passable_Cell's four `CheckPassability` calls pass `required_height_or_level = -1`; the height match is a separate FNPC internal `±2` gate, not this arg. *(R5, FNPC caller matrix; chrono-return report §3.)*

**C14 — FNPC bridge allowance is an FNPC flag, not a CheckPassability arg.** Whether structural bridge cells (`+0x140 & 0x100`) are accepted is filtered by FNPC's own `allow_bridge_cells` flag after `CheckPassability`, not inside the wrapper. The wrapper's arg8 is the bridge-aware/layer arg to GetZoneID, NOT an allow/reject. *(R5, CheckPassability full-arg report §3.4.)*

**C15 — FNPC search shape.** Concentric diamond rings outward from the seed; per ring visits top/bottom edges then left/right columns; radius cap `min(Speed + Sight, 32)`; collects candidates; early-terminates at 24 candidates or when a direct candidate completes its ring. NOT a row scan, NOT a spiral. *(R5, FNPC report §§2-3; chrono-return report §4.)*

**C16 — FNPC selection when no target cell. [PASS-2 VERIFIED — gate closed]** With target `{0,0}` (no preferred cell), selection is `candidates[frame_counter % count]`, with **direct candidates preferred** over indirect (`if direct_count>0: pick from directs; else from indirects`). **The selection source is the deterministic global game-tick frame counter `g_CurrentFrameCounter` (global `0x00A8ED84`), NOT an RNG draw** — `decompile_function 0x0056DC20` / `disassemble_function 0x0056DC20` show the tail at `0x0056E6A8`: `MOV EAX,[0x00A8ED84]; IDIV ECX` (then `local_60[index]` for indirects / `local_60[index-0x18]`==`local_c0[index]` for directs). `0x00A8ED84` is incremented once per logic tick in `Main_Tick @ 0x0055DE81` (`g_CurrentFrameCounter = g_CurrentFrameCounter + 1`, `get_xrefs_to ram:0x00A8ED84` shows the sole `[WRITE]`; the same global drives anim/crate-regen/lightning/laser timers — `read_memory ram:0x00A8ED84` = `00000000` static/BSS, set at runtime). **Determinism consequence (master-TODO #2 two-RNG interaction): FNPC's own selection consumes NEITHER `Scen->Random` NOR `g_MainRng` — it is pure frame-counter, so it does not perturb either RNG stream and is lockstep-safe by construction (every client shares the frame counter).** *(R5; `decompile_function 0x0056DC20`; `get_xrefs_to ram:0x00A8ED84`; `decompile_function 0x0055DE60`.)*

**C16b — FNPC selection when a target cell IS given. [PASS-2 VERIFIED — new]** When the input `param_14` target cell is NOT the null sentinel (`!= DAT_00ABD480`), selection switches to **nearest-to-target by Euclidean distance** over the preferred pool (directs if any, else indirects): `best=100000.0; for c in pool: d=Sqrt_Approx(dx²+dy²); if d<best: best=d, result=c`. **This path consumes NO frame counter and NO RNG.** Most cell-validation callers pass the null target (so C16 frame-counter path runs); callers that pass a real target (some convoy/AI move scripts) get the distance path. *(R5; `decompile_function 0x0056DC20` tail `0x0056E6F0..0x0056E797`; `Sqrt_Approx @ 0x004CAC40`; FNPC deep-dive report §6c.)*

**C16c — Same-tick FNPC aliasing hazard (player-visible, lockstep-safe). [PASS-2 NOTE]** Because C16 uses `frame_counter % count`, **two no-target FNPC calls on the SAME tick with the SAME candidate count return the SAME index** (e.g. an MCV deploy and a chrono-teleport landing both with 3 candidates pick `frame%3`). This is gamemd behavior, NOT a bug — the Rust port must reproduce it (do not add a per-call RNG to "spread" them). *(`FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md` §6.1.)*

**C21 — CheckOccupancy RTTI-0x24 blocker is TerrainClass. [PASS-2 VERIFIED — gate closed]** The first CheckOccupancy blocker (`FUN_0047C550`, `decompile_function 0x0047C550`) scans the ground list (`+0xE4`) and rejects on the first object whose `WhatAmI()` (vtable slot `+0x2C`) returns `0x24`. **`0x24` (36) = `RTTIType::Terrain` — a `TerrainClass` instance** (trees TREE01-36, TIBTRE tiberium trees, ICE floes, veinhole roots, crates, lights, signs, poles). Verified: `TerrainClass__What_Am_I @ 0x0071D300` is literally `return 0x24;` (`decompile_function 0x0071D300`), vtable `0x007F5200`. So "RTTI-0x24 ground-list blocker" = "an inert terrain object (tree/rock/etc.) occupies the cell." Building reject (`WhatAmI()==6`) is the separate `Look_up_building_in_cell @ 0x0047C520`. The Rust port models C10a as **"a TerrainClass-category occupant is present in the ground list"** — no longer "non-techno blocker, identity DEFERRED." *(R3/C10a; `decompile_function 0x0047C550` / `0x0047C520` / `0x0071D300`; `UNIT_COLLISION_AND_REPATH_TRIGGERS` §13.4: `RTTIType::Terrain == 0x24`.)*

**C22 — Save/load cell object-list order is serialized, NOT rebuilt. [PASS-2 VERIFIED — gate closed]** On savegame load the per-cell ground/bridge object lists (`CellClass+0xE4`/`+0xE8`) are **restored by direct serialization + pointer-swizzle, not re-derived via AddContent**: `ObjectClass::Load @ 0x005F5E80` (`decompile_function`) swizzle-registers the `+0x30` NextObject cell-linkage pointer (along with `+0x34/+0x38/+0x18/+0x88`); the CellClass list-head + the per-object `+0x30` chain are loaded as raw pointers and remapped, so **the cell-list insertion ORDER survives byte-for-byte** (same discipline as the LogicClass active vector, `SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION` report). `AddContent @ 0x0047E8A0` is reached on load ONLY for `TerrainClass::Mark @ 0x0071BFB0` (terrain re-marked) and runtime techno enter-cell — not as a generic post-load occupancy rebuild. **The reduced ZoneType column (`+0x4C`) IS re-derived on load**: Load_Game `FUN_0067E730` calls the zone-map rebuild `FUN_00581F50` at `0x0067E8CD` (`ZoneMap__BuildZoneLevel` for levels 2,1,0 then pathfinder zone arrays at `0x87E8B8`). **Hash-relevance:** cell-list order is observable (nearest-object, area-damage iteration, occupancy first-blocker), so a Rust port that rebuilds occupancy from `EntityStore::values()` after load (current `src/sim/occupancy.rs:110`) will DRIFT if creation-ID order ≠ saved live insertion order — the port must serialize cell-list order directly. *(R8; `decompile_function 0x005F5E80` / `0x005F6250`; `get_xrefs_to 0x00581F50` → `FUN_0067E730@0x0067E8CD`; `get_function_callers 0x0047E8A0`.)*

**C17 — FNPC no-candidate result.** Zero candidates writes null cell `{0,0}` to the output; the caller (e.g. Mission_Harvest) interprets `{0,0}` as "no cell" and calls `Set_Destination(NULL, 1)` (unit stays put, retries next tick). *(R5, chrono-return report §5.)*

**C18 — CheckOccupancy uses the dummy cell mid-scan but still corner-checks.** Out-of-range in-rect cells substitute the dummy during the blocker scan (so a blocker is not falsely found), but the final `IsRectInPlayfield` still rejects out-of-play rectangles. CheckPassability has NO final playfield check (dummy substitution only). *(C2 + R3, validator OQ-8, live both bodies.)*

**C19 — Reduced ZoneType is RecalcZoneType-written, not raw LandType.** `CellClass+0x4C` (the matrix column) is written by `RecalcZoneType @ 0x00483C80`: 0=Ground, 1=Crushable, 2=Wall, 3=Beach, 4=Water, 5=Building, 6=Impassable, 7=Outside. Column 1 is `Crushable=yes` overlay, NOT road art. *(R7, RecalcZoneType report; Rust `passability.rs:106` doc matches.)*

**C20 — Cell-list membership selection is by OnBridge.** `CheckOccupancy`'s ground-list reads target `+0xE4`; bridge-deck membership is `+0xE8`, selected from the object's `OnBridge` byte (`ObjectClass+0x8C`). The validators read the list the writer (`AddContent`/`RemoveContent`) populated; structures append / non-structures prepend within the selected layer. *(R8/R3, live-object-list report §§3-4; Rust `OccupancyGrid` matches the insertion order.)*

---

## 6. Rust-native replacement boundary

A cohesive **read-only `CellClass` validation facade in `sim/`** that mirrors the engine's two-validator split with clean Rust, over the existing `OccupancyGrid` + `PathGrid` + cell-state. It owns three things the current code scatters: the **rectangle passability predicate**, the **rectangle occupancy predicate** (incl. the reservation surface + playfield corners), and the **nearby-passable-cell search**. It exposes the validators as the primitives pathfinding/placement/spawn consume — **extending** the boundary already drafted in `CELLCLASS_SUBSTRATE_FIRST_MIGRATION_SLICE_GHIDRA_REPORT.md`, not replacing `OccupancyGrid`.

### 6.1 Ownership / module diagram

```
src/sim/cell_validation/                         // NEW module (sim/, no render/ui/audio/net dep)
  mod.rs        — CellValidator facade (borrows grids; never mutates; never owns storage)
  passability.rs— check_passability_rect + check_cell_passability (per-cell)
  occupancy.rs  — check_occupancy_rect + Reservation surface
  find_nearby.rs— find_nearby_passable_cell (diamond-ring port)
  cell_index.rs — cell_index() + get_cellclass_fallback() parity accessor (y*0x200+x, dummy)

Simulation (src/sim/world/mod.rs)
├── occupancy: OccupancyGrid          // existing: dynamic entity object-list membership (READ)
├── path_grid / zone / resolved_terrain / overlay_grid   // existing cell-state (READ)
└── cell_reservations: ReservationGrid   // NEW (DEFERRED-AI seam): +0xDC per-house/site bits

         ┌──────────── read-only validation service (NEW) ────────────┐
         │ CellValidator<'a> { occupancy, path_grid, zone, terrain,    │
         │                     overlay, reservations, map_dims }       │
         │   .check_passability_rect(rect, &PassabilityQuery) -> bool  │ ← C1/C3/C4/C5/C6/C7/C8
         │   .check_occupancy_rect(rect, Reservation) -> bool          │ ← C9/C10/C11/C12/C18
         │   .find_nearby_passable_cell(seed, &NearbyQuery) -> Option<Cell> │ ← C13..C17
         │   .get_cellclass_fallback(coord) -> CellRef (never None)    │ ← C1/C2
         └─────────────────────────────────────────────────────────────┘

  Pathfinding seam: Can_Enter_Cell / A* fallback consume check_passability_rect (coordinate
                    with the pathfinding sibling study — these validators are its primitives).
  Spawn/exit/scatter seam: replace nearest_walkable_around with find_nearby_passable_cell.
  AI seam (DEFERRED): check_occupancy_rect(rect, Reservation::House(id)) + ReservationGrid;
                      never called by human path; FUN_005060B0 internals not designed.
```

**Layering:** the facade lives in `sim/`, depends only on `sim/` grids + `rules/` (SpeedType/MovementZone) + `map/` cell-state. It NEVER depends on render/ui/sidebar/audio/net. It is a **borrow-only projection** — `CellValidator<'a>` holds `&` references and mutates nothing, so it cannot perturb the state hash (mirrors the shadow discipline of the Factory/House study).

**Determinism / 30-player scale:** the reservation surface is keyed by `InternedId` house, NOT a `1<<(layer&0x1F)` 32-bit mask — the engine's `+0xDC` bitmask is a 32-slot hard cap. The mask semantics (`1<<(layer&0x1F)`) are reproduced *as a query rule* for parity (C12), but the storage is an `InternedId`-keyed set so it scales to 30 players. The validator carries no per-tick mutable state; iteration over `OccupancyGrid` is already `BTreeMap`-deterministic.

### 6.2 Key types (Rust-native, fixed-point where applicable, no addresses in comments)

```rust
//! Cell-validation substrate: read-only rectangle passability/occupancy predicates
//! and the nearby-passable-cell search, the primitives pathfinding, placement,
//! spawn, scatter, and dock-approach consume. Borrows the occupancy/path/terrain
//! grids; never mutates; never owns storage. Depends only on sim grids + rules data.
//! Never depends on render/ui/sidebar/audio/net.

/// Fixed cell-array stride — the engine indexes cells y*0x200+x regardless of the
/// loaded map's playfield width. Valid linear range [0, MAX_CELL_INDEX].
pub const CELL_ROW_STRIDE: u32 = 0x200;
pub const MAX_CELL_INDEX: u32 = 0x3FFFF;
/// Winged/Fly speed type — short-circuits per-cell passability to success.
pub const SPEED_TYPE_WINGED: i32 = 4;

/// Packed top-left + size, signed cell coords (engine CellRect / CellStruct).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

/// The 9-arg passability config a caller supplies (engine CheckPassability args 4-9).
#[derive(Debug, Clone, Copy)]
pub struct PassabilityQuery {
    pub speed_type: SpeedType,
    pub required_zone_id: i32,        // -1 disables the zone-id comparison (C6)
    pub movement_zone: MovementZone,  // matrix row family — NOT speed_type (C6)
    pub required_height_or_level: i32,// -1 = unrestricted; FNPC passes -1 (C13)
    pub bridge_aware: bool,           // arg8: GetZoneID + height/layer selection (C8/C14)
    pub reject_any_overlay: bool,     // arg9: reject Cell.overlay != none before per-cell (C4)
}

/// Occupancy reservation arg (engine CheckOccupancy arg2). House(id) reproduces the
/// 1<<(idx&0x1F) mask rule (C12) but stores InternedId for 30-player scale.
#[derive(Debug, Clone, Copy)]
pub enum Reservation { SkipReservation /* -1 */, House(InternedId) }

/// FNPC config (engine Find_Nearby_Passable_Cell caller args).
#[derive(Debug, Clone, Copy)]
pub struct NearbyQuery {
    pub passability: PassabilityQuery,
    pub allow_bridge_cells: bool,     // FNPC filter, applied AFTER passability (C14)
    pub check_height: bool,           // FNPC ±2 internal gate (separate from required_height)
    pub check_occupancy: bool,        // call check_occupancy_rect(rect, SkipReservation) (C12 -1)
    pub radius_cap: u16,              // min(Speed+Sight, 32) computed by caller (C15)
}

/// A non-null cell reference — Some(real) or the dummy with the requested coord (C2).
pub enum CellRef<'a> { Real(&'a PathCell), Dummy { coord: (i32, i32) } }

pub struct CellValidator<'a> {
    occupancy: &'a OccupancyGrid,
    path_grid: &'a PathGrid,
    zone: Option<&'a ZoneGrid>,
    terrain: Option<&'a ResolvedTerrainGrid>,
    overlay: Option<&'a OverlayGrid>,
    reservations: Option<&'a ReservationGrid>, // DEFERRED-AI seam
}

impl<'a> CellValidator<'a> {
    /// Engine y*0x200+x with [0, MAX_CELL_INDEX] bound; never None — OOB returns the
    /// dummy with the requested coord stored (C1/C2). Does NOT use loaded-map width.
    pub fn get_cellclass_fallback(&self, x: i32, y: i32) -> CellRef<'a>;

    /// All-cells AND-fold over the rect (C3); overlay-reject precheck (C4); per-cell
    /// CheckCellPassability (C5/C6/C7/C8). width<=0 or height<=0 => true.
    pub fn check_passability_rect(&self, rect: CellRect, q: &PassabilityQuery) -> bool;

    /// Per-cell terrain rule. SPEED_TYPE_WINGED fast-pass (C5); zone-id (C6); required
    /// height/bridge-layer + +0x124/+0x128 selection (C8); wall exception + speed table (C7).
    fn check_cell_passability(&self, cell: CellRef<'a>, q: &PassabilityQuery) -> bool;

    /// Blocker-order scan (C10) + reservation mask (C12) + final playfield corners (C11).
    /// NO terrain/zone read (C9). Zero-size rect still runs the corner check (C18).
    pub fn check_occupancy_rect(&self, rect: CellRect, reservation: Reservation) -> bool;

    /// Diamond-ring search (C15); per candidate check_passability_rect + optional
    /// check_occupancy_rect(-, SkipReservation) (C13); allow_bridge filter after (C14);
    /// frame-counter selection when no target cell (C16); None on no candidate (C17).
    pub fn find_nearby_passable_cell(
        &self, seed: (i32, i32), q: &NearbyQuery, frame_counter: u32,
    ) -> Option<(u16, u16)>;
}
```

### 6.3 Where it sits in `advance_tick`

The validator is a **pure read-only service** — it has no tick phase of its own. It is *called by* existing phases:
- **Movement / pathfinding** (Phase 2/3): `Can_Enter_Cell` and A* blocked-destination fallback call `check_passability_rect` / `find_nearby_passable_cell` (coordinate with the pathfinding sibling study).
- **Production spawn / war-factory exit** (Phase 7): replace `nearest_walkable_around` with `find_nearby_passable_cell`.
- **Scatter** (Phase 7): scatter target cell selection.
- **Miner dock approach / chrono return**: the chrono-miner-return fallback (chrono-return report) calls `find_nearby_passable_cell` with the dock-cell + DockOffset seed.

The ONE hash-relevant consumer is `find_nearby_passable_cell`'s frame-counter selection (C16) — it returns a different *cell*, which feeds Set_Destination/spawn position (hashed). That is why P6 (FNPC) is the authority-flip slice with the `SNAPSHOT_VERSION` bump, and why P0 must pin the frame-counter source first.

### 6.4 Serialization + hashing

- `CellValidator` is **borrow-only and stateless** → nothing to serialize, nothing added to `state_hash`. The shadow slices (P1–P3) are guaranteed hash-neutral by construction (the facade only *reads*).
- The only new *stored* state is `ReservationGrid` (the `+0xDC` surface), which is **DEFERRED-AI** — not built or hashed until AI placement lands. When it does, it joins `state_hash` behind a version bump.
- The hash-affecting behavior is the **FNPC cell choice** (P6): once `find_nearby_passable_cell` replaces `nearest_walkable_around`, the spawn/destination cell changes, which is hashed. That flip is gated behind `SNAPSHOT_VERSION 17→18` and the P0 frame-counter research gate.

### 6.5 Shadow-mode rollout (mirrors Mission/Radio + the cell-substrate first slice)

1. **Shadow:** add `CellValidator` + tests at the substrate boundary; assert `check_passability_rect` agrees with `PathGrid::is_walkable` on the cases the bool grid *can* represent, and surfaces (does not equalize) divergence where the typed inputs matter (overlay-reject, zone, required-height).
2. **Invert:** flip `Can_Enter_Cell` / spawn fallback to *read from* the validator; the bool grid becomes a cache the validator can derive, not the authority.
3. **Drop shadow asserts.**
4. **Authoritative (FNPC):** replace `nearest_walkable_around` with `find_nearby_passable_cell`; this changes spawn/destination cells (hashed) → **bump `SNAPSHOT_VERSION 17→18`.**
5. **Parity harness:** deterministic replay over a recorded command stream produces a bit-identical per-tick hash sequence vs the baseline.

### 6.6 Seams

- **Pathfinding (sibling study):** the validators are the primitives A*/`Can_Enter_Cell` consume. Coordinate the seam: the validator owns the *cell-legality predicate*; pathfinding owns the *graph search*. Do not duplicate the predicate in both.
- **AI (DEFERRED):** `check_occupancy_rect(rect, Reservation::House(id))` + `ReservationGrid` is the only AI entry; `FUN_005060B0` internals (AIBaseSpacing footprint expand, three shifted rects) are not designed here. `feedback_no_ai_yet`.
- **Save/load:** native `+0xE4`/`+0xE8` rebuild order is UNCHECKED (live-object-list report OQ-WR-009). The validator reads whatever `OccupancyGrid::rebuild` produces; it does not assert byte-perfect gamemd save/load order — flagged UNCHECKED, not a design claim.
- **Building placement:** the separate `Buildable=` building-placement predicate (`production_placement.rs`) is NOT this family (C-negative-fact). Do not fuse it into the validator.

---

## 7. Old ad hoc Rust logic to retire / fold into the new service

Retire only at/after the authority-flip slice (P5/P6); until then they coexist as the shadow-derived-from source.

| Current Rust (file:symbol) | Smell | Replaced by |
|---|---|---|
| `src/sim/pathfinding/core.rs:1613` `PathGrid::is_walkable` (single bool) | collapses SpeedType/zone/height/overlay-reject into one bool (DRIFT #1) | `CellValidator::check_passability_rect` + `check_cell_passability` (C3–C8) |
| `src/sim/pathfinding/core.rs:1642/1614` `PathGrid::cell` / `is_walkable` width-based index + `None` OOB | wrong index width + `None` vs non-null dummy (DRIFT #2/#3) | `CellValidator::get_cellclass_fallback` (`y*0x200+x`, dummy) — `PathGrid` index stays for the cache, parity callers use the facade |
| `src/sim/production/production_spawn.rs:290` `nearest_walkable_around(grid, …, 12, …)` call site (fn def at `:355`) | different ring + deterministic first-match vs diamond-ring + frame-counter select (DRIFT #6) | `CellValidator::find_nearby_passable_cell` (C13–C17) |
<!-- Reviewer 2026-06-04: line citations re-verified via Grep on production_spawn.rs — find_spawn_cell_near_structure fn @ :237 (✓), nearest_walkable_around CALL @ :290 (✓ with radius 12), fn DEF @ :355 (added). -->

| `src/sim/production/production_spawn.rs:237` `find_spawn_cell_near_structure` (preferred-offsets then ad-hoc fallback) | ad-hoc preferred-offset list + `cell_available_for_spawn`/`spawn_cell_passable` split | preferred-offsets stay as caller policy; the fallback delegates to `find_nearby_passable_cell` |
| `src/sim/pathfinding/passability.rs:149` `zone_layer_for_speed_type` (SpeedType→matrix row shim) | the validator's zone row is MovementZone, not SpeedType (DRIFT #8) — retire IF any validation path reaches it | `check_cell_passability` reads MovementZone directly (C6); keep the shim only for genuinely SpeedType-only legacy callers, audited |
| (scattered) per-caller "is this cell free to spawn" checks (`cell_available_for_spawn` in `production_spawn.rs`) | partial, no `+0x44/+0x4C/+0x11C`/reservation/playfield-corner (MISSING #4) | `CellValidator::check_occupancy_rect` (C9–C12/C18) |
| `OccupancyGrid` as a stand-in for occupancy *legality* (it is membership only) | callers treat "no entity here" as "placeable" — misses cell-blocker bytes + playfield corners | keep `OccupancyGrid` as the object-list source; legality is `check_occupancy_rect` reading it + cell-state |
| `OccupancyGrid::rebuild` from `EntityStore::values()` on load (`src/sim/occupancy.rs:110`) | **DRIFT (PASS-2, C22):** orders cell lists by creation/interned-ID, but gamemd serializes cell-list insertion ORDER verbatim (CellClass heads + `+0x30` NextObject swizzle). Diverges the first time a save is loaded where insertion order ≠ ID order — observable via first-blocker/nearest-object/area-damage iteration. | serialize cell-list order directly per cell on save, restore in saved order on load; do NOT re-derive from `EntityStore` iteration. Coordinate with the save/load substrate (active-vector order is already serialized verbatim). |

**Note:** `MOVEMENT_ZONE_PASSABILITY` (`passability.rs:115`) and `OccupancyGrid` insertion order are **kept** — they are verified-correct ingredients the facade consumes, not retire-targets.

---

## 8. Migration slices + acceptance tests

Dependency-ordered, shadow-first, in the proven rhythm: **shadow → invert → drop shadow asserts → make authoritative → bump SNAPSHOT_VERSION → parity harness.** Every test runs as `cargo test -p vera20k` and is deterministic; the one hash-relevant flip (P6) gates on `state_hash()`. Per the first-slice plan, **start with the substrate query + tests; migrate the lowest-risk caller (spawn fallback) before movement/bridges/AI.**

### Slice P0 — research gate (**MOSTLY CLOSED PASS-2; no longer blocking P6**)
**Status:** The two BLOCKING items are RESOLVED live (PASS-2 2026-06-04):
- ~~`CellClass::CheckCellPassability 0x004834A0` full body~~ — **VERIFIED** (`decompile_function 0x004834A0`): Winged-4 fast-pass (C5); `+0x124`/`+0x128` selection by required-height + `Flags&0x100` (C8) incl. the `&0xE0`/`&0x5F` occupation-mask modifier args; wall-overlay exception set (movement-zones `2,3,8,0xC` accept; `1,4` accept iff overlay `+0x22D`; else force LandType=0); `g_SpeedType_LandType_Table[speed+LandType*9] == 0.0` reject **with `!bVar2` bridge bypass** (C7).
- ~~`FootClass::Find_Nearby_Passable_Cell 0x0056DC20` C16 selection source~~ — **VERIFIED = `g_CurrentFrameCounter` (`0x00A8ED84`), deterministic frame counter, NOT an RNG draw** (`decompile_function`/`disassemble_function 0x0056DC20` tail `MOV EAX,[0x00A8ED84]; IDIV ECX`; `get_xrefs_to ram:0x00A8ED84`). Plus C16b (target → nearest-distance) and C16c (same-tick aliasing). **master-TODO #2 RNG-stream concern is a NON-ISSUE for FNPC.** P6 is no longer blocked on this.
- ~~RTTI-0x24 class identity~~ — **VERIFIED = TerrainClass** (`decompile_function 0x0071D300` = `return 0x24`). C21. Rust models C10a as "TerrainClass-category occupant present."
- ~~Save/load `+0xE4`/`+0xE8` rebuild order~~ — **VERIFIED = serialized verbatim, not rebuilt** (C22).

**Remaining P0 (non-blocking, do before P6 only if exact corner parity needed):**
- `MapClass::IsRectInPlayfield 0x00578390` — exact 4-corner formula (C11). Next: `decompile_function 0x00578390`.
- Dummy cell `DAT_00ABDC50` runtime-init field values (C2) — to define `CellRef::Dummy` field reads. Statically BSS-zero; needs live-init dump.

**Output:** §9.1 PASS-2 rows replace the prior DOC-ONLY rows. **P6 is UNBLOCKED for the FNPC selection source.**

### Slice P1 — `CellValidator` facade + `get_cellclass_fallback` (shadow, read-only)
**Goal:** introduce the borrow-only facade and the parity cell-index accessor.
**Files:** `src/sim/cell_validation/{mod.rs,cell_index.rs}` (new).
**Tests:**
- `cell_index_uses_512_wide_stride_not_map_width` — `(x,y)` whose loaded-map-width check would differ from `y*0x200+x` follows the 512-wide rule (C1).
- `get_cellclass_oob_returns_dummy_with_requested_coord` — probing `(-1,0)` / index > `0x3FFFF` returns `CellRef::Dummy { coord }` (never None), coord == requested (C2).
- `cell_validator_is_read_only_no_hash_change` — constructing + querying the validator leaves `state_hash()` bit-identical (the shadow guarantee).

### Slice P2 — `check_passability_rect` + `check_cell_passability` (shadow; assert vs PathGrid)
**Goal:** the rectangle passability predicate; shadow-assert agreement with the bool grid where representable, surface divergence where typed inputs matter. **(BLOCKED on P0 for the per-cell math.)**
**Files:** `cell_validation/passability.rs` (new).
**Tests:**
- `passability_rect_all_cells_must_pass` — a 2×2 rect with one blocked cell fails; all-clear passes (C3).
- `passability_zero_size_rect_returns_true` — `width<=0` or `height<=0` returns true without reading a cell (C3).
- `passability_reject_overlay_flag_is_caller_specific` — a cell with an overlay passes when `reject_any_overlay=false`, fails when `true` (C4).
- `passability_winged_fast_passes_everything` — SpeedType Winged passes a cell the bool grid marks blocked (C5).
- `passability_zone_check_uses_movement_zone_not_speed_type` — a unit with `MovementZone=Water` + non-water SpeedType uses the Water matrix row for the `required_zone_id` comparison; `required_zone_id == -1` skips it (C6).
- `passability_rect_shadow_agrees_with_pathgrid_on_plain_cells` — on cells with no overlay/zone/height constraint, `check_passability_rect(1×1)` == `PathGrid::is_walkable`; divergence elsewhere is surfaced (tick+cell), never equalized.

### Slice P3 — `check_occupancy_rect` + `Reservation` (shadow)
**Goal:** the rectangle occupancy predicate incl. the blocker order, reservation mask, and final playfield corners.
**Files:** `cell_validation/occupancy.rs` (new).
**Tests:**
- `occupancy_minus_one_skips_reservation_but_rejects_cell_blockers` — a cell with only a `+0xDC`-style reservation passes under `Reservation::SkipReservation`; a slope/special (`+0x11C`) or overlay (`+0x44`) cell fails (C9/C10/C12).
- `occupancy_blocker_order_matches_engine` — fixture exercising each rejecter (RTTI-0x24 / reservation / `+0x44` / `+0x4C` / `+0x11C` / building) rejects in the C10 order.
- `occupancy_reservation_house_index_blocks_matching_only` — `Reservation::House(A)` rejects a cell reserved for A, passes one reserved for B (the `1<<(idx&0x1F)` rule via InternedId map) (C12).
- `occupancy_zero_size_rect_still_runs_playfield_corners` — a degenerate rect skips the blocker scan but a fully-out-of-play rect still fails via the corner check (C11/C18).
- `occupancy_out_of_play_rect_rejected_by_corner_check` — an in-bounds-looking top-left whose `x+w-1`/`y+h-1` corner is off-playfield fails (C11).

### Slice P4 — invert: `Can_Enter_Cell` / spawn-fallback read the validator (shadow→authoritative-for-reads)
**Goal:** route the lowest-risk caller (production spawn fallback) through the facade for its passability/occupancy decision, keeping the bool grid as a cache. Coordinate the seam with the pathfinding sibling study for `Can_Enter_Cell`.
**Files:** `production_spawn.rs`, `cell_validation/mod.rs`.
**Tests:**
- `spawn_fallback_uses_validator_predicates` — a candidate the validator rejects (overlay / occupancy) is skipped even when the bool grid says walkable; a previously wrongly-accepted candidate now fails.
- `spawn_fallback_no_hash_change_when_predicates_agree` — on a fixture where the validator and the old checks agree, `state_hash()` is unchanged (proves the invert is hash-neutral until the search shape changes in P6).

### Slice P5 — `find_nearby_passable_cell` (shadow against `nearest_walkable_around`)
**Goal:** implement the diamond-ring search + selection, shadow-asserting candidate *sets* against the legacy ring without yet changing the chosen cell. **(C16 selection BLOCKED on P0.)**
**Files:** `cell_validation/find_nearby.rs` (new).
**Tests:**
- `find_nearby_diamond_ring_visit_order` — fixture asserts ring/edge/column visitation order and `min(Speed+Sight,32)` radius cap (C15).
- `find_nearby_calls_occupancy_with_skip_reservation` — when `check_occupancy=true`, the occupancy call uses `Reservation::SkipReservation` (-1), never a house index (C13/C12).
- `find_nearby_allow_bridge_filters_after_passability` — a bridge candidate passes `check_passability_rect` but is dropped when `allow_bridge_cells=false` (C14).
- `find_nearby_no_candidate_returns_none` — zero candidates → `None` (engine null-cell `{0,0}`), and the caller clears the destination (C17).
- `find_nearby_passes_required_height_minus_one` — the passability calls use `required_height_or_level = -1` (C13).

### Slice P6 — authoritative FNPC + bump SNAPSHOT_VERSION (hash-relevant; **P0 C16 gate CLOSED PASS-2 — selection = `g_CurrentFrameCounter`**)
**Goal:** replace `nearest_walkable_around` with `find_nearby_passable_cell` in spawn/exit/scatter/chrono-return; the chosen cell (incl. frame-counter selection) becomes authoritative; **bump `SNAPSHOT_VERSION 17→18`.**
**Files:** `production_spawn.rs`, `miner_dock_sequence.rs` (chrono-return seed), `movement/scatter.rs`, `snapshot.rs`, `world_hash.rs` (if the reservation surface lands).
**Tests:**
- `find_nearby_selection_uses_frame_counter_modulo` — with no target cell, the chosen candidate is `candidates[g_CurrentFrameCounter % count]`, direct preferred (C16); the source is the deterministic per-tick frame counter (PASS-2 pinned), NOT an RNG draw. The Rust port must thread the sim frame counter into `find_nearby_passable_cell` (already in the signature) and must NOT substitute an RNG.
- `find_nearby_target_selection_uses_nearest_distance` — when a real target cell is given (not the null sentinel), the chosen candidate is the nearest-to-target by Euclidean distance, consuming no frame counter and no RNG (C16b).
- `find_nearby_same_tick_aliasing` — two no-target FNPC calls on the same tick with the same candidate count pick the same index (C16c); the port reproduces this, does not "spread" them with RNG.
- `chrono_return_seed_is_dock_cell_plus_dockoffset` — the chrono-miner-return seed cell = `(dock.cellX + DockOffset.X, dock.cellY + DockOffset.Y)` (chrono-return report §2).
- `snapshot_version_is_18` + `snapshot_roundtrip_after_fnpc_authority` — serialize→deserialize→`state_hash()` equal; `SNAPSHOT_VERSION == 18`.
- `war_factory_exit_cell_matches_baseline` — a recorded baseline exit-cell sequence equals the live FNPC sequence.

### Slice P7 — global parity / replay harness (acceptance)
**Goal:** the end-to-end determinism gate — a recorded command stream (build+exit, scatter, chrono-harvest cycle) replayed twice and against the pre-migration baseline yields a bit-identical per-tick `state_hash()` sequence.
**Files:** `src/sim/cell_validation/cell_validation_parity_tests.rs` (new), reusing the existing replay harness.
**Tests:**
- `cell_validation_replay_is_bit_identical` — fixed seed + scripted stream over ~600 ticks; `run()` twice → identical `Vec<hash>`.
- `cell_validation_parity_vs_baseline_hash` — the recorded baseline (captured at the P6 flip commit) equals the live sequence; a regression in any of C1–C20 flips a tick hash.
- `chrono_miner_return_cell_deterministic_over_replay` — every chrono-return over the stream picks a deterministic cell (C16/C17).

*AI base-spacing reservation (`FUN_005060B0` / `ReservationGrid`) and native save/load `+0xE4`/`+0xE8` rebuild order are explicit seams, not designed here.*

---

## 9. Sources & Verification Ledger

### 9.1 Ghidra verified LIVE this session

| Address | Function | Verifying call | Used in |
|---|---|---|---|
| `0x004834A0` | **CellClass::CheckCellPassability** (NOT CheckOccupancy — brief mislabel) | `get_function_by_address 0x004834a0` → `CellClass__CheckCellPassability` | §0/header label correction, R2, C5/C7/C8 |
| `0x0056E7C0` | CellRect::CheckPassability | `get_function_by_address` + `decompile_function 0x0056e7c0` (loop, overlay-reject, `y*0x200+x`, CheckCellPassability call) | R1, C1/C3/C4 |
| `0x00586780` | CellRect::CheckOccupancy | `get_function_by_address` + `decompile_function 0x00586780` (mask `1<<(arg&0x1f)`, blocker chain, `IsRectInPlayfield`) | R3/R4, C9–C12/C18 |
| `0x0056DC20` | FootClass::Find_Nearby_Passable_Cell | `get_function_by_address 0x0056dc20` → `FootClass__Find_Nearby_Passable_Cell` | R5, C13–C17 |
| `0x00483C80` | CellClass::RecalcZoneType | `get_function_by_address 0x00483c80` → `CellClass__RecalcZoneType` | R7, C19 |
| `0x005657A0` | MapClass::Get_CellClass | `get_function_by_address` + `decompile_function 0x005657a0` (`y*0x200+x`, `[0,0x3FFFF]`, dummy `DAT_00abdc50` + coord `DAT_00abdc74`, array base `*(this+0x13c)`) | R6, C1/C2 |
| (callers) | CheckOccupancy callers | `get_function_callers 0x00586780` → `FUN_005060b0`, `FootClass__Find_Nearby_Passable_Cell` | §2a, AI seam |
| `0x004834A0` (PASS-2) | **CheckCellPassability full body** — Winged-4 fast-pass, GetZoneID `-1` skip, height/bridge-layer `Flags&0x100`+`Level`/`Level+4` selection of `+0x124`/`+0x128`, occupation-mask modifier args `&0xE0`/`&0x5F`, wall-overlay exception set, speed-table `0.0` reject with `!bVar2` bridge bypass | `decompile_function 0x004834A0` | C5/C7/C8 (DOC-ONLY → VERIFIED) |
| `0x0056DC20` (PASS-2) | **FNPC full body + selection** — diamond ring, 24-cap, direct/indirect split, `frame_counter % count` (no-target) / nearest-distance (target) | `decompile_function 0x0056DC20`, `disassemble_function 0x0056DC20` | C16/C16b/C16c (DOC-ONLY → VERIFIED) |
| `0x00A8ED84` (PASS-2) | **g_CurrentFrameCounter** — FNPC selection source; per-tick increment, NOT an RNG stream | `get_xrefs_to ram:0x00A8ED84` ([WRITE] only at `Main_Tick 0x0055DE81`); `decompile_function 0x0055DE60`; `read_memory ram:0x00A8ED84` | C16 gate close |
| `0x0071D300` (PASS-2) | **TerrainClass::What_Am_I** = `return 0x24` → RTTI-0x24 = TerrainClass | `decompile_function 0x0071D300` | C21 gate close |
| `0x0047C550` (PASS-2) | **RTTI-0x24 ground-list scan** — calls vtable `+0x2C` (WhatAmI), rejects on `0x24` | `decompile_function 0x0047C550` | C10a/C21 (DOC-ONLY → VERIFIED) |
| `0x0047C520` (PASS-2) | **Look_up_building_in_cell** — same vtable `+0x2C`, rejects on `6` | `decompile_function 0x0047C520` | C10f (DOC-ONLY → VERIFIED) |
| `0x005F5E80` (PASS-2) | **ObjectClass::Load** — swizzle-registers `+0x30` NextObject (cell linkage), `+0x34/+0x38/+0x18/+0x88`; no AddContent rebuild | `decompile_function 0x005F5E80` | C22 gate close |
| `0x005F6250` (PASS-2) | **ObjectClass::Save** — serializes `+0x8C` OnBridge + coords `+0x9C/+0xA0/+0xA4`; does NOT serialize cell-list heads | `decompile_function 0x005F6250` | C22 |
| `0x00581F50` (PASS-2) | **ZoneMap rebuild** (levels 2,1,0 + pathfinder arrays `0x87E8B8`) — called by Load_Game `0x0067E730` at `0x0067E8CD` | `get_xrefs_to 0x00581F50`; `decompile_function 0x00581F50` | C22 (zone re-derived on load) |
| `0x005060B0` (PASS-2) | **AI base-site helper** — `CheckOccupancy(rect, HouseClass+0x30)` house-index reservation, AIBaseSpacing `Rules+0x1460`, `g_DirectionOffsets`, `atan2`/`ftol`, FoundationW/H | `decompile_function 0x005060B0` | C12 confirm, AI seam (§8 deferred) |
| `0x00500200` (PASS-2) | **Find_Passable_Cell_Near_Unit** — sibling wrapper that draws `Random__RandomRanged(1,4)` THEN calls FNPC | `decompile_function 0x00500200` | new §2a row, RNG note |
| `0x007E1748` (PASS-2) | speed-table reject constant = `0.0` | `read_memory ram:0x007E1748` = `00000000` | C7 |

### 9.2 DOC-ONLY (corroborated by a verified report, NOT re-read live — re-verify before load-bearing)

- ~~`CheckCellPassability 0x004834A0` body details~~ — **CLOSED PASS-2, now VERIFIED** (`decompile_function 0x004834A0`); see §9.1 PASS-2 + C5/C7/C8.
- ~~FNPC `0x0056DC20` search/selection~~ — **CLOSED PASS-2, now VERIFIED** (`decompile_function 0x0056DC20`); see C16/C16b/C16c.
- ~~RTTI-0x24 class identity~~ — **CLOSED PASS-2 = TerrainClass** (`decompile_function 0x0071D300`); see C21.
- `IsRectInPlayfield 0x00578390` 4-corner formula — still DOC-ONLY (validator report §3.2); call confirmed in live `0x00586780` body but the corner formula itself not re-read PASS-2. Next query: `decompile_function 0x00578390`.
- Dummy `DAT_00ABDC50` initial field values (DEFERRED — not dumped; the slot is statically BSS-zero, runtime-init).
- Zone matrix `0x0082A594` `int[13][8]`, only `1` passes (ZONE_PASSABILITY report); mirrored byte-identical in Rust `MOVEMENT_ZONE_PASSABILITY`.
- Speed/Land table `[speed_type + LandType*9]` numeric contents (SPEEDTYPE_LANDTYPE report); the `== 0.0` reject path + `0.0` constant `0x007E1748` are VERIFIED PASS-2, the per-cell dump values are DOC-ONLY.
- Live cell-list writers `+0xE4`/`+0xE8`/`+0x124`/`+0x128` (live-object-list report §3).

### 9.3 UNCHECKED / blocking

- ~~**FNPC C16 selection source**~~ — **RESOLVED PASS-2: deterministic `g_CurrentFrameCounter` (`0x00A8ED84`), NOT an RNG draw.** FNPC consumes neither RNG stream → master-TODO #2 interaction is a NON-ISSUE for FNPC. **§8 P0 is no longer blocking for P6.** (Note: the sibling wrapper `Find_Passable_Cell_Near_Unit 0x00500200` DOES draw `Random__RandomRanged(1,4)` before calling FNPC — RNG lives in that wrapper, not FNPC.)
- ~~**Native save/load `+0xE4`/`+0xE8` rebuild order**~~ — **RESOLVED PASS-2: order is serialized verbatim (CellClass heads + object `+0x30` NextObject swizzle), NOT re-derived; zone column `+0x4C` IS rebuilt.** See C22. Rust must serialize cell-list order directly, not rebuild from `EntityStore::values()`.
- ~~**RTTI-0x24 class identity**~~ — **RESOLVED PASS-2 = TerrainClass.** See C21.
- **REMAINING (non-blocking):** `IsRectInPlayfield 0x00578390` exact 4-corner formula (re-read next); dummy `DAT_00ABDC50` runtime-init field values; the `ObjectClass+0x98` post-load membership setter (save/load report OQ-SL-007, deferred — affects active-vector membership, not cell-list order).

### 9.4 Rust source consumed (in-tree, this session)

- `src/sim/occupancy.rs` — `OccupancyGrid` (insertion order `:200-217` matches verified `+0xE4`/`+0xE8` rule; dynamic-entity membership only).
- `src/sim/pathfinding/passability.rs` — `MOVEMENT_ZONE_PASSABILITY[13][8]` (`:115`, byte-verified vs native dump), `zone_layer_for_speed_type` shim (`:149`), `LandType` compat enum.
- `src/sim/pathfinding/core.rs` — `PathGrid::is_walkable` (`:1613`), `cell` (`:1642`, width-based index + `None`), `DEFAULT_BLOCKED_CELL` (`:1542`).
- `src/sim/production/production_spawn.rs` — `find_spawn_cell_near_structure` (`:237`), `nearest_walkable_around` fallback (`:290`).
- `src/sim/snapshot.rs` — `SNAPSHOT_VERSION = 17` (`:24`); the 17→18 bump in §6.5/§8 P6 is correct.

### 9.5 Research docs / plans consumed

- `docs/research/CELLCLASS_SUBSTRATE_FIRST_MIGRATION_SLICE_GHIDRA_REPORT.md` (the boundary this study extends).
- `docs/research/pathfinding/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`.
- `docs/research/pathfinding/CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md`.
- `docs/research/CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md`.
- `docs/research/MAPCLASS_GET_CELLCLASS_FALLBACK_DUMMY_CELL_GHIDRA_REPORT.md`.
- `docs/research/miner/PATHFINDING_VALIDATE_ALTERNATE_CHRONO_RETURN_GHIDRA_REPORT.md`.
- `docs/research/skirmish-ui/CELLRECT_CHECKPASSABILITY_START_RECTANGLE_CALLER_SLICE_GHIDRA_REPORT.md` (caller slice; cited by topic).
- `docs/plans/2026-05-29-core-engine-substrate-todo.md` (master TODO #7, the home of this slice).
- Section shape mirrored from `docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

---

---

## Pass 2 — Expansion (live Ghidra, 2026-06-04)

Systematic completeness sweep + gate closure. Every item below was re-read live this run; addresses cite the verifying call. Default verdict DRIFT unless proven.

### P2.1 Gate closures (all three open gates RESOLVED)

| Gate | Verdict | Evidence |
|---|---|---|
| **FNPC selection source (C16)** | **VERIFIED** — deterministic `g_CurrentFrameCounter` (`0x00A8ED84`), NOT an RNG draw. Increment is once/tick in `Main_Tick @ 0x0055DE81`. FNPC consumes neither `Scen->Random` nor `g_MainRng`. | `decompile_function 0x0056DC20`; `disassemble_function 0x0056DC20` (tail `0x0056E6A8`: `MOV EAX,[0x00A8ED84]; IDIV ECX`); `get_xrefs_to ram:0x00A8ED84` (sole [WRITE] = Main_Tick); `decompile_function 0x0055DE60` |
| **RTTI-0x24 identity (Gate B)** | **VERIFIED** — `0x24` (36) = `RTTIType::Terrain` = **TerrainClass** instance (trees/ice/veinhole/crates/lights/signs). Dispatched via WhatAmI vtable slot `+0x2C`. | `decompile_function 0x0071D300` (`return 0x24`); `decompile_function 0x0047C550` (`*(*piVar1+0x2c)()==0x24`); vtable `0x007F5200` |
| **Save/load cell-list rebuild order (Gate A)** | **VERIFIED** — cell object-list ORDER serialized verbatim (CellClass `+0xE4`/`+0xE8` heads + per-object `+0x30` NextObject swizzle-remap), NOT rebuilt via AddContent. Zone column `+0x4C` IS re-derived (zone-map rebuild). | `decompile_function 0x005F5E80` (Object::Load swizzles `+0x30`); `decompile_function 0x005F6250` (Object::Save: no cell heads); `get_xrefs_to 0x00581F50` → Load_Game `0x0067E8CD`; `get_function_callers 0x0047E8A0` (only TerrainClass::Mark + techno enter) |

### P2.2 New consumers found (FNPC caller breadth — `get_function_callers 0x0056DC20`, 40 callers)

The doc previously listed only spawn/scatter/chrono. FNPC's frame-counter selection (hash-relevant) actually fires across **40 callsites**, all of which the P6 authority flip touches:

- **Production / deploy:** `BuildingClass__ExitObject_Main 0x00443C60`, `BuildingClass__OnConstructionComplete 0x00445F80`, `BuildingClass__ReleaseDockedHarvester 0x004595C0`, `BuildingClass__SetRallyPoint 0x00443860`, `SlaveManagerClass__FindDeployCell 0x006B0300`.
- **Superweapons / chrono / paradrop:** `ChronoSphere__WarpUnitsAtCell 0x0065EC30`, `SuperClass__Launch 0x006CC390`, `FUN_0065E010`/`FUN_0065E850` (chrono-related).
- **Movement / scatter / pathfinding:** `InfantryClass__Scatter 0x0051D0D0`, `FootClass__Find_Path 0x004D3920`, `FootClass__ClickedAction_Object 0x004D74E0`, `FootClass__Mission_AreaGuard 0x004D6AA0`, `FootClass__Mission_Patrol 0x004D4280`, `FootClass__Greatest_Threat_Scan 0x004D5690`, `FootClass__Find_Passable_Cell_Near_Unit 0x00500200`.
- **Aircraft / locomotion:** `AircraftClass__Find_Nearest_Friendly_Airfield 0x0041A160`, `FlyLocomotionClass__Descent_Step 0x004CE840`, `FlyLocomotionClass__Emergency_Relocate 0x004CCFD0`.
- **House / rally / AI:** `HouseClass__AI_GroundRallyPoint 0x00509CD0`, `HouseClass__Recalc_Base_Center 0x004FD150`, `HouseClass__Set_Rally_Point_Cell 0x004FBF60`, `FUN_005060B0` (AI base-site).
- **Map / scenario:** `MapClass__PlaceCrateAtRandomCell 0x0056BD40`, `ScenarioClass__Gather_Start_Positions 0x00688380`.
- **Team scripts (AI convoys):** `TeamClass__Convoy_Script_*` ×6 (`0x006EE3F0`/`0x006EE5C0`/`0x006EE800`/`0x006EC7D0`/`0x006EF700`/`0x006EFA10`).

Material to §8 P6: replacing `nearest_walkable_around` is not just a spawn-fallback change — it is authoritative for rally points, scatter, chrono warp, paradrop, slave deploy, crate placement, and start positions. All must use the SAME frame-counter selection to stay bit-identical.

### P2.3 New sibling search wrapper (RNG location clarified)

`FootClass__Find_Passable_Cell_Near_Unit @ 0x00500200` (`decompile_function`): calls `Random__RandomRanged(1,4)` to choose one of up to 4 candidate-direction variants (skipped if all three vtable-query offsets `+0x2D4`/`+0x2D8`/`+0x2DC` sum to 0), converts the unit's lepton pos to a cell, reads `GetZoneID`, then calls FNPC inside that zone. **The RNG draw is in THIS wrapper, not in FNPC** — confirms FNPC itself is RNG-free. A Rust port of this specific entry must draw from the correct RNG instance (per-callsite ECX rule, `reference_rng_instance_routing_truth`); the other 39 FNPC callers do not.

### P2.4 CheckCellPassability body precision (DOC-ONLY → VERIFIED, `decompile_function 0x004834A0`)

New details the prior contract lacked:
- **Speed-table reject is bridge-bypassed:** `(table[...] == 0.0) && !bVar2` — the `0.0` reject does NOT fire when the AltOccupation (bridge) path was selected. (C7 updated.)
- **Two occupation-mask modifier args:** byte arg2 (`!=0` → `&0xE0`) and byte arg3 (`!=0` → `&0x5F`) mask the selected occupation byte for sub-cell-aware locomotor callers; the wrapper passes both zero. (C8 updated.)
- **Wall-overlay exception set is exhaustive:** accept in movement-zones `2,3,8,0xC` unconditionally; in `1,4` accept iff `OverlayType+0x22D`; else reject; on accept force `LandType=0`. Gated on `OverlayType+0x2A8` ("is wall").
- Reject constant `FLOAT_007E1748 == 0.0` (`read_memory ram:0x007E1748`).

### P2.5 AI base-site helper internals (deferred seam, now documented — `decompile_function 0x005060B0`)

`FUN_005060B0` confirms the AI occupancy/reservation seam: `CheckOccupancy(rect, HouseClass+0x30)` (house index → `1<<(idx&0x1F)`, C12), AIBaseSpacing from `RulesClass+0x1460` (with `+1` for naval/`+0x1765`/`+0x55E`), foundation expand via `BuildingTypeClass__GetFoundationWidth/Height`, 8-direction probing via `g_DirectionOffsets`, `Math__atan2`+`Math__ftol` for the approach facing, and a `RulesClass+0xE0C` distance cap. Internals remain DEFERRED (`feedback_no_ai_yet`) but the seam contract is fully sourced now.

### P2.6 Burden-of-proof re-flag on own claims

- **Cell-list save/load order (was "UNCHECKED, not a design claim"):** re-flagged to **DRIFT** for the current Rust approach — `OccupancyGrid` rebuilds from `EntityStore::values()` on load (`src/sim/occupancy.rs:110`), which orders by creation/interned ID, NOT by saved live insertion order. gamemd preserves insertion order verbatim (C22). This is a concrete output divergence (cell-list first-blocker, nearest-object, area-damage iteration) the moment a save is loaded where insertion order ≠ ID order. Added to §7 retire/fix list.
- **`IsRectInPlayfield` corner formula (C11):** stays DOC-ONLY — the call is confirmed live in `0x00586780` but the 4-corner arithmetic itself was NOT re-read PASS-2. Honest verdict: UNCHECKED on the exact `x+w-1`/`y+h-1` corner formula. Next: `decompile_function 0x00578390`.

### P2.7 TS-legacy / edge-case check

- RTTI-0x24 = TerrainClass is **live in YR** (trees block placement on every map) — NOT a TS ghost. Veinhole roots (`VEINTREE`) are the only borderline member, but they exist in YR maps; keep.
- No new TS-only path found in the validators or FNPC. The subterranean matrix row (6) note in §2f stands.

---

## Reviewer follow-ups (adversarial pass 2026-06-04)

**Verdict: GREEN with one line-citation precision patch.** Every load-bearing address, offset, body, and Rust ref was re-verified live this session; no DRIFT was found mislabeled as internal-only, no TS-legacy path was designed in, the slices respect the substrate program (shadow-first, hash-neutral P1–P5, single SNAPSHOT_VERSION 17→18 flip at P6, EXTENDS the first-slice boundary, no sim→render dependency).

Re-verified live (read-only):
- `0x004834A0`=`CellClass__CheckCellPassability`, `0x00586780`=`CellRect__CheckOccupancy`, `0x0056E7C0`=`CellRect__CheckPassability`, `0x0056DC20`=`FootClass__Find_Nearby_Passable_Cell`, `0x005657A0`=`MapClass__Get_CellClass`, `0x00483C80`=`CellClass__RecalcZoneType` — all via `get_function_by_address`. The brief's `0x004834A0=CheckOccupancy` mislabel correction is **confirmed correct**.
- `decompile_function 0x00586780`: blocker chain order (FUN_0047c550 → `+0xdc & 1<<(arg&0x1f)` → `+0x44 != -1` → `+0x4c` → `+0x11c` → Look_up_building_in_cell) + final `MapClass__IsRectInPlayfield(param_1,1)` — matches C9–C12/C18 exactly.
- `decompile_function 0x005657A0`: `param_2[1]*0x200+*param_2`, `[0,0x3ffff]`, dummy `DAT_00abdc50` + `DAT_00abdc74` coord store, base `*(param_1+0x13c)` — matches C1/C2/R6 exactly.
- `decompile_function 0x0056E7C0`: AND-fold loop, overlay-reject `in_stack_00000024 && OverlayTypeIndex != -1`, dummy fallback, `CheckCellPassability` call, zero-size→return 1 — matches C3/C4/R1 exactly.
- `get_function_callers 0x00586780` → `FUN_005060b0` + FNPC — matches §2a/R5.

**Notable upgrade for P0 (doc under-claimed):** `decompile_function 0x004834A0` is **live-readable now** and confirms the §8-P0 "DOC-ONLY" CheckCellPassability math verbatim: `if (speed_type != 4) {...}` Winged fast-pass (C5); `GetZoneID` with `-1` skip (C6); `Flags & 0x100` (structural bridge) selecting `AltOccupationFlags`/`OccupationFlags` gated on `required_height == -1 || == Level+4` (C8); wall-overlay exception movement-zones `2,3,8,1,4,0xC` + overlay `+0x22d` forcing LandType=0 (C7); `g_SpeedType_LandType_Table[speed_type + LandType*9] == 0.0` reject (C7). **P0 should DOWNGRADE the CheckCellPassability re-decode from BLOCKING to "spot-confirm only"** — the body is no longer DOC-ONLY-unverified. The genuinely-blocking P0 item remains **only** the FNPC C16 frame-counter-vs-RNG selection source (`0x0056DC20` interior not re-read this pass).

Rust line citations re-verified via Grep: `is_walkable` @ `core.rs:1613` (✓), `cell` @ `:1642` (✓ width-index + None), `find_spawn_cell_near_structure` @ `production_spawn.rs:237` (✓), `nearest_walkable_around` call @ `:290`/def @ `:355` (patched), `cell_available_for_spawn` @ `:553` (✓), `MOVEMENT_ZONE_PASSABILITY` @ `passability.rs:115` (✓), `zone_layer_for_speed_type` @ `:149` (✓), `OccupancyGrid` insertion order @ `occupancy.rs:200-217` (✓ AppendBuilding/PrependNonBuilding), `SNAPSHOT_VERSION = 17` @ `snapshot.rs:24` (✓ — 17→18 bump correct).

Residual UNCHECKED for synthesis (unchanged, correctly flagged): FNPC C16 selection source (BLOCKING P6), native save/load `+0xE4`/`+0xE8` rebuild order, RTTI-0x24 class identity, dummy `DAT_00ABDC50` initial field values.

---

*End of study. The substrate is additive and read-only: P1–P5 change no hashed bit (the validator only reads); the one authority flip — replacing `nearest_walkable_around` with the engine's `find_nearby_passable_cell` (which selects a different cell via the frame counter) — lands at P6 with `SNAPSHOT_VERSION 17→18`; P0 is a BLOCKING research gate that must pin the CheckCellPassability per-cell math and the FNPC frame-counter selection source before P6. AI base-spacing reservation (`FUN_005060B0` / `ReservationGrid`) and native save/load cell-list rebuild order are explicit seams, not designed here. The brief's `0x004834A0 = CheckOccupancy` label was WRONG and is corrected to CheckCellPassability throughout.*
