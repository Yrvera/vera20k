# Cell-Validation Facade (Slice 2) Design

**Status:** DESIGN SPEC (brainstorm output, not an approved implementation plan). Doc-only — no `src/` touched this run.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. Default-to-DRIFT; no internal-only escape hatch for active cell-validation behavior.
**Slots into:** master roadmap item **#7 (map/cell substrate)** in `docs/plans/2026-05-29-core-engine-substrate-todo.md`. This is item #7's first implementable slice and **extends** (does not replace) the boundary drafted in `docs/research/CELLCLASS_SUBSTRATE_FIRST_MIGRATION_SLICE_GHIDRA_REPORT.md`.
**Primary source:** `docs/research/CELL_VALIDATION_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (binary-verified PASS-2 2026-06-04; cited inline as STUDY §/Cn).

---

## Design-review corrections (2026-06-04, adversarial pass — verdict GREEN)

The spec is grounded: every PARITY claim traces to a STUDY Cn that was re-verified live in PASS-2, or is explicitly flagged UNKNOWN/UNCHECKED (L3 dummy init, L12 speed-table dump, L19 corner formula). Tiny-detail coverage L1–L31 maps the STUDY C1–C22 + §P2 ledger with no silent drops. It EXTENDS (does not compete with) the first-slice report — same read-only-facade-over-`OccupancyGrid`, same passability≠occupancy separation, same "keep `OccupancyGrid`, keep `MOVEMENT_ZONE_PASSABILITY`" — and respects the substrate program (shadow-first, hash-neutral P1–P5, single `SNAPSHOT_VERSION 17→18` at P6, `advance_tick` order unchanged, no sim→render dep). Retire targets all grep-confirmed real this run. Corrections below were applied inline; uncertain items stay flagged as open questions.

1. **FNPC frame-counter source IS already in the sim — named it (the spec under-specified "the sim per-tick logic counter").** The deterministic counter the engine uses for FNPC selection (`g_CurrentFrameCounter`, L24/L26) already has a sim analog: `World::binary_frame: u32`, documented in-code as "the `g_CurrentFrameCounter` analog" (verified `src/sim/world/mod.rs:293-302`, field decl `:302`). **Two load-bearing semantics the write-plan MUST honor:** (a) `binary_frame` is **committed late** at the END of `advance_tick` (`src/sim/world/mod.rs:1742` `self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32`), so during a tick it holds the *pre-increment* frame N this tick executes under — which exactly mirrors gamemd incrementing `g_CurrentFrameCounter` only after `Network_ServiceLoop`. FNPC must read it as the **current** frame (the value modulo'd), NOT a next-frame value. (b) It is derived from `total_sim_ms`, which is serialized/hashed — so threading `binary_frame` into `find_nearby_passable_cell` is lockstep-safe by construction (every client shares it). The `frame_counter: u32` arg in the signature sketch (L24, §6.2) should be wired to `World::binary_frame`, not a fresh counter. Verified by reading `src/sim/world/mod.rs:288-302` and `:1742` this run.

2. **C22-correction test line cite off-by-one.** §"Note on C22" says the enter-order test is at `occupancy.rs:813`; it is at `occupancy.rs:814` (`fn rebuild_uses_cell_entry_order_not_stable_id_order`, verified via Grep this run). Substance unchanged; fixed inline below.

3. **No DRIFT mislabeled as internal-only; no TS path designed in.** L31 correctly fences the subterranean row as TS-dead; L18/L21/L23 keep the active-YR contract. The one declared "Deviation" (`InternedId`-keyed reservations vs the engine's `1<<(idx&0x1F)` 32-bit mask) is a legitimate scale-target replacement with the query rule preserved (L15) — acceptable, not a hand-waved equivalence, because the result is identical for valid house indices and the 32-slot cap is a scale-limiting internal.

**For the write-plan stage (carry forward):**
- Thread `World::binary_frame` (NOT a new counter) into `find_nearby_passable_cell`; honor the late-commit "read as current frame N" semantics (correction 1).
- P0 still has two genuine UNCHECKED gates before P3/P6 land their respective math: `IsRectInPlayfield 0x00578390` 4-corner formula (L19, blocks the P3 corner test — see Open Question 2 for the P3-vs-P3.5 split) and dummy `0x00ABDC50` runtime-init field values (L3, only blocks if a human-path caller reads fields off the dummy — see Open Question 3). Neither is closed; do not let the plan assume them.
- L12 speed-table dump values are DOC-ONLY — spot-confirm before the P2 per-cell math lands.

---

## Goal

Add a read-only, borrow-only `CellClass`-style validation facade in `sim/` that owns the three cell-legality primitives the engine separates — rectangle **passability**, rectangle **occupancy** (incl. reservation bits + playfield-corner containment), and the **diamond-ring nearby-passable-cell** search with deterministic frame-counter candidate selection — so spawn/exit/scatter/chrono-return/rally cells become gamemd-identical, replacing the scattered `PathGrid::is_walkable` bool + `nearest_walkable_around` box-ring.

## Non-Goals

- **No AI base-site placement** (`feedback_no_ai_yet`). The `check_occupancy_rect(rect, Reservation::House(id))` arg + `ReservationGrid` are designed as a seam; `FUN_005060B0` internals (AIBaseSpacing footprint expand, 8-direction probing, atan2 facing) are NOT implemented here. STUDY §P2.5.
- **No save/load cell-list serialization rewrite** (C22) in this slice. The DRIFT is documented and a fix is scoped, but it is a save/load-substrate seam, not built here.
- **No building-placement (`Buildable=`) fusion.** That is the separate `production_placement` predicate, not this validator family. STUDY §6.6.
- **No `OccupancyGrid` replacement.** The facade reads it; the verified insertion order (`occupancy.rs:200-217`) stays authoritative. STUDY §4.4.
- **No pathfinding graph-search rewrite.** This owns the *predicates*; pathfinding owns the *search*. Seam coordination only.
- **No new tick phase.** The validator is a pure read-only service called by existing phases; `advance_tick` phase order is unchanged.

---

## Architecture Context

### What the engine does (verified)

The engine splits cell legality into two independent validators plus a search, over a fixed cell array:

- **`CheckPassability` (0x0056E7C0)** — 9-arg rectangle AND-fold over terrain rules (SpeedType / required-zone-id / MovementZone / required-height / bridge-aware / overlay-reject). No playfield check. STUDY R1, C3–C8.
- **`CheckCellPassability` (0x004834A0)** — per-cell callee: Winged-4 fast-pass, MovementZone-rowed zone-id compare, `+0x124`/`+0x128` occupation-byte selection by required-height + structural-bridge flag, wall-overlay exception set, `SpeedType+LandType*9` speed-table `0.0`-reject (bridge-bypassed). STUDY R2, C5/C7/C8.
- **`CheckOccupancy` (0x00586780)** — 2-arg rectangle blocker scan: TerrainClass (RTTI-0x24) ground-list object, `+0xDC` reservation bit, `+0x44` overlay, `+0x4C` blocker, `+0x11C` special/slope, `WhatAmI()==6` building, then **all-four-corners-in-playfield** (`IsRectInPlayfield`). No terrain read. STUDY R3, C9–C12/C18/C21.
- **`Find_Nearby_Passable_Cell` (0x0056DC20)** — diamond-ring search seeded at a cell, calls `CheckPassability` (+ optional `CheckOccupancy(rect,-1)`) per candidate; selection is `candidates[g_CurrentFrameCounter % count]` (direct preferred) when no target cell, nearest-Euclidean when a target is given. Frame counter, **not RNG**. STUDY R5, C13–C17, C16/C16b/C16c.
- **`Get_CellClass` (0x005657A0)** — coord→cell via fixed `y*0x200+x`, range `[0,0x3FFFF]`, non-null **dummy** fallback storing the requested coord. STUDY R6, C1/C2.

### What current Rust has (read this run)

The ingredients are scattered across three modules with no unified rectangle predicate:

| Concern | Current Rust | File:line (verified this run) |
|---|---|---|
| Per-cell walkability (one bool) | `PathGrid::is_walkable(x,y)` | `src/sim/pathfinding/core.rs:1613` |
| Cell lookup (width-based index, `None` OOB) | `PathGrid::cell` / `is_walkable` `y*width+x`, bounds `x<width && y<height` | `core.rs:1642`, `:1614`/`:1617` |
| Blocked-cell sentinel | `DEFAULT_BLOCKED_CELL` | `core.rs:1542` |
| Zone matrix (verified byte-identical) | `MOVEMENT_ZONE_PASSABILITY[13][8]` | `src/sim/pathfinding/passability.rs:115` |
| SpeedType→row shim (NOT the validator's row source) | `zone_layer_for_speed_type` | `passability.rs:149` |
| Dynamic occupancy + verified list order | `OccupancyGrid` (`add` insertion `:200-217`) | `src/sim/occupancy.rs:99`, `:184` |
| Occupancy rebuild on load | `OccupancyGrid::rebuild` sorts by `(occupancy_enter_order, stable_id)` | `occupancy.rs:118`, sort `:121` |
| Spawn-cell fallback search (box-ring, first-match) | `nearest_walkable_around(grid,…,12,…)` | `production_spawn.rs:355` (def), call `:290` |
| Spawn entry + preferred offsets | `find_spawn_cell_near_structure` | `production_spawn.rs:237` |
| Snapshot version | `SNAPSHOT_VERSION = 17` | `src/sim/snapshot.rs:24` |

**Note on C22 (correction to STUDY).** The STUDY states `OccupancyGrid::rebuild` "orders by creation/interned ID." The actual code sorts by `(occupancy_enter_order, stable_id)` (`occupancy.rs:121`), and there is already a test (`rebuild_uses_cell_entry_order_not_stable_id_order`, `occupancy.rs:814` — corrected from `:813` by design-review Grep this run) asserting enter-order, not ID-order. The DRIFT claim still stands in substance — `occupancy_enter_order` is a Rust-side re-derivation, NOT the gamemd saved live cell-list insertion order serialized verbatim — but the spec must describe it as "enter-order re-derivation vs serialized insertion order," not "ID order." Verified by reading `occupancy.rs:118-142` this run.

### Layering

The facade lives in `sim/`, depends only on `sim/` grids + `rules/` (SpeedType/MovementZone/foundation) + `map/` cell-state. It NEVER touches render/ui/sidebar/audio/net. It is a borrow-only projection (`&` references, mutates nothing) — by construction it cannot perturb the state hash. Mirrors the shadow discipline of the Factory/House study and the Mission/Radio substrate rhythm.

---

## Impact Analysis

**Read-only / hash-neutral (P1–P5):** new module `src/sim/cell_validation/` + tests. Constructing and querying `CellValidator` reads existing grids and mutates nothing → `state_hash()` bit-identical. No risk to determinism.

**Hash-relevant (P6 only):** replacing `nearest_walkable_around` with `find_nearby_passable_cell` changes the chosen cell (diamond-ring + frame-counter selection vs box-ring + first-match). The chosen cell feeds `Set_Destination`/spawn position, which is hashed → `SNAPSHOT_VERSION 17→18` bump + parity harness. STUDY §6.4, §8-P6.

**Blast radius of the P6 flip (large — STUDY §P2.2, 40 FNPC callers).** The engine's FNPC is authoritative for far more than spawn fallback: rally points, scatter, chrono warp, paradrop, slave deploy, crate placement, start positions, aircraft airfield search, AI convoy scripts. **The Rust port migrates these incrementally** — P6 migrates only the human-reachable spawn/exit/scatter/chrono-return callers; the remaining callers (AI convoy, slave deploy, crate placement) migrate in later slices but MUST all use the SAME frame-counter selection so any migrated-vs-unmigrated mix stays internally consistent. This is the scoping decision flagged in the brief.

**Dependencies to coordinate:**
- **Pathfinding sibling study** — `Can_Enter_Cell` / A* blocked-destination fallback consume `check_passability_rect`. The validator owns the predicate; pathfinding owns the search. Do NOT duplicate the predicate.
- **Save/load substrate** — C22 cell-list-order serialization is a seam; coordinate with the active-vector-order serialization already in place.
- **AI substrate (deferred)** — `ReservationGrid` + `Reservation::House(id)` is the only AI entry point; not wired this slice.

**What could break:** if the diamond-ring or frame-counter selection is even one cell / one index off, every war-factory exit, scatter, and chrono-return diverges from baseline replay — caught by the P7 parity harness. The frame-counter source MUST be the sim per-tick logic counter threaded explicitly (it is in the signature), never an RNG substitute (C16c same-tick aliasing must reproduce, not be "spread").

---

## Tiny-Detail Ledger (parity constraint set)

Every item is carried through to `/write-plan`. Source cited; default DRIFT until a test proves equivalence.

### Cell addressing / fallback
- **L1** Cell linear index = `(short)y * 0x200 + (short)x`, valid range `[0, 0x3FFFF]`; the 512 stride is **independent of loaded-map width** (current Rust uses `y*width+x`). [STUDY C1; live `0x005657A0`]
- **L2** OOB/null lookup returns a **non-null dummy** cell (never `None`) and stores the *requested* packed coord into the dummy's coord slot; caller may still read fields off it. The dummy is NOT a constant `(0,0)` coord. [STUDY C2, §2f; live `0x005657A0`]
- **L3** Dummy `DAT_00ABDC50` runtime-init field values are **UNKNOWN — needs RE** (statically BSS-zero, runtime-init not dumped). Affects what `CellRef::Dummy` field reads return. [STUDY §9.2, §9.3 remaining]

### Passability rectangle (`check_passability_rect`)
- **L4** All-cells AND-fold: walk `x = 0..width-1` outer, `y = 0..height-1` inner; true iff **every** in-rect cell passes. `width<=0` or `height<=0` returns true **without reading any cell**. [STUDY C3; live `0x0056E7C0`]
- **L5** Overlay-reject is a **caller flag** (arg9): when set, a cell with `overlay != none` (`+0x44 != -1`) fails the rect **before** the per-cell check. FNPC's chrono-return call passes `0` (overlays NOT rejected). [STUDY C4; live `0x0056E7C0`]
- **L6** SpeedType==4 (Winged) **fast-pass**: per-cell returns true immediately, skipping zone/height/occupation/wall/speed-table. [STUDY C5; live `0x004834A0`]
- **L7** Zone-id comparison uses the **MovementZone row, NOT SpeedType**; matrix `0x0082A594` is `int[13][8]`, **only value 1 passes** (2/3 block); `required_zone_id == -1` skips the comparison. [STUDY C6; matrix mirrored at `passability.rs:115`]
- **L8** Occupation-byte selection: `+0x128` (bridge, sets bridge-path flag) chosen only when `(required_height == -1 OR required_height == cell.Level+4)` AND structural-bridge flag `Flags & 0x100`; else `+0x124` (ground). A non-zero (post-mask) selected byte rejects. [STUDY C8; live `0x004834A0`]
- **L9** Two occupation-mask modifier args exist: arg `!=0` → mask `& 0xE0` (high 3 bits / sub-cell); next arg `!=0` → mask `& 0x5F`. The `CheckPassability` wrapper passes **both zero** → full selected byte must be zero to pass on the wrapper path. Sub-cell-aware locomotor callers pass them nonzero. [STUDY C8/§P2.4; live `0x004834A0`]
- **L10** Speed-table reject: `g_SpeedType_LandType_Table[speed_type + LandType*9] == 0.0` rejects, **but only when the bridge/AltOccupation path was NOT selected** (`&& !bridge_selected`) — a bridge cell bypasses the `0.0` reject entirely. Constant `0x007E1748 == 0.0`. [STUDY C7/§P2.4; live `0x004834A0`, `read_memory 0x007E1748`]
- **L11** Wall-overlay exception set: accept in movement-zones `2,3,8,0xC` unconditionally; in `1,4` accept iff overlay `+0x22D`; else reject. On accept, **force LandType=0 (Clear)** before the speed-table lookup. Gated on overlay `+0x2A8` ("is wall"). [STUDY C7/§P2.4; live `0x004834A0`]
- **L12** Speed/Land table per-cell numeric contents are **DOC-ONLY** (SPEEDTYPE_LANDTYPE report); the `==0.0` reject path + constant are VERIFIED, the dump values are not re-read this run — re-verify before the per-cell math lands. [STUDY §9.2]

### Occupancy rectangle (`check_occupancy_rect`)
- **L13** It is **NOT passability** — no SpeedType / MovementZone / LandType / zone read. [STUDY C9; live `0x00586780`]
- **L14** Blocker scan order (per in-rect cell): (a) TerrainClass (RTTI-0x24) ground-list object → reject; (b) `(+0xDC & mask) != 0` → reject; (c) `+0x44 != -1` (overlay) → reject; (d) `+0x4C != 0` → reject; (e) `+0x11C != 0` (special/slope) → reject; (f) `WhatAmI()==6` building on ground list → reject. **Order is observable** (first-blocker semantics). [STUDY C10/C21; live `0x00586780`]
- **L15** Reservation mask: `layer == -1` → mask 0, `+0xDC` test **skipped**; else `mask = 1 << (layer & 0x1F)`. A non-`-1` negative arg aliases through `& 0x1F`. FNPC passes `-1`; AI-site passes a house index. [STUDY C12; live `0x00586780`]
- **L16** Final containment: if no in-rect blocker, result = `IsRectInPlayfield(rect, 1)` testing all four corners using `x+width-1`, `y+height-1`. A zero/negative-size rect **skips the blocker scan but still runs this corner check**. [STUDY C11/C18; live call confirmed, exact 4-corner formula DOC-ONLY — re-read `0x00578390` before this lands]
- **L17** Out-of-range in-rect cells substitute the dummy mid-scan (so no false blocker is found), but the final corner check still rejects out-of-play rectangles. CheckPassability has **NO** final playfield check. [STUDY C18; live both bodies]
- **L18** RTTI-0x24 = **TerrainClass** (trees TREE01-36, TIBTRE, ICE, veinhole roots, crates, lights, signs, poles) — **live in YR, not a TS ghost**. Rust models L14(a) as "a TerrainClass-category occupant is present in the ground list." [STUDY C21/§P2.7; live `0x0071D300`]
- **L19** Exact 4-corner `IsRectInPlayfield` formula is **DOC-ONLY / UNCHECKED** — call confirmed in the live body, arithmetic not re-read. Re-verify (`decompile_function 0x00578390`) before P3 lands the corner test. [STUDY §9.3 remaining]

### Nearby search (`find_nearby_passable_cell`)
- **L20** Search shape: concentric **diamond** rings outward from the seed; per ring visit top/bottom edges then left/right columns; radius cap `min(Speed + Sight, 32)`; early-terminate at 24 candidates or when a direct candidate completes its ring. **NOT a box-ring, NOT a spiral, NOT a row scan** (current Rust `nearest_walkable_around` is a box-ring). [STUDY C15; `production_spawn.rs:355`]
- **L21** FNPC always passes `required_height_or_level = -1` to `CheckPassability`; height match is a separate FNPC `±2` internal gate. [STUDY C13]
- **L22** Bridge allowance is an **FNPC filter applied AFTER passability**, not a CheckPassability arg. [STUDY C14]
- **L23** Optional occupancy call uses `Reservation::SkipReservation` (`-1`), **never** a house index. [STUDY C13/C12]
- **L24** Selection, no target cell: `candidates[g_CurrentFrameCounter % count]`, **direct candidates preferred** (`if direct_count>0: pick from directs; else indirects`). Source = the deterministic per-tick frame counter `g_CurrentFrameCounter` (`0x00A8ED84`), **NOT an RNG draw** — consumes neither RNG stream, lockstep-safe by construction. [STUDY C16; live `0x0056DC20` tail, `get_xrefs_to 0x00A8ED84`]
- **L25** Selection, target cell given (not null sentinel): **nearest-to-target by Euclidean distance** over the preferred pool; consumes no frame counter and no RNG. Most cell-validation callers pass the null target (frame-counter path); convoy/AI move scripts pass a real target. [STUDY C16b; live `0x0056DC20`]
- **L26** **Same-tick aliasing is gamemd behavior:** two no-target FNPC calls on the same tick with the same candidate count return the SAME index. Reproduce it; do NOT add per-call RNG to spread them. [STUDY C16c]
- **L27** No-candidate result: write null cell `{0,0}` → modeled as `None`; the caller interprets it as "no cell" and clears the destination (unit stays put, retries next tick). [STUDY C17]
- **L28** Chrono-miner-return seed cell = `(dock.cellX + DockOffset.X, dock.cellY + DockOffset.Y)` (BuildingType offsets). [STUDY §8-P6, chrono-return report §2]
- **L29** The sibling wrapper `Find_Passable_Cell_Near_Unit` (`0x00500200`) draws `Random__RandomRanged(1,4)` **before** calling FNPC — RNG lives in that wrapper, not FNPC. If/when that specific entry is ported, the RNG draw must use the correct per-callsite RNG instance (`reference_rng_instance_routing_truth`); the other 39 FNPC callers do not draw RNG. [STUDY §P2.3]

### Save/load (documented seam, not built this slice)
- **L30** gamemd serializes cell object-list **insertion order verbatim** (CellClass `+0xE4`/`+0xE8` heads + per-object `+0x30` NextObject swizzle); the reduced-ZoneType column `+0x4C` IS re-derived on load. The Rust `OccupancyGrid::rebuild`-from-`EntityStore` (enter-order re-derivation) will DRIFT the first time a save is loaded where enter-order ≠ saved live insertion order — observable via first-blocker / nearest-object / area-damage iteration. [STUDY C22/§P2.6; live `0x005F5E80`/`0x005F6250`/`0x00581F50`; current Rust `occupancy.rs:118-142`]

### TS-legacy guard
- **L31** Subterranean MovementZone row 6 exists in the matrix (kept for byte-parity) but is **TS-dead** for movers — do NOT design a subterranean code path. [STUDY §2f/§P2.7; `feedback_no_tunnel_subterranean`]

---

## Chosen Approach

**A borrow-only `CellValidator<'a>` facade in `src/sim/cell_validation/`** that mirrors the engine's two-validator-plus-search split, reads the existing `OccupancyGrid` + `PathGrid` + cell-state grids, and exposes the four primitives. Built shadow-first; the only hash-relevant flip (FNPC adoption) lands last behind `SNAPSHOT_VERSION 17→18`.

This is the approach the STUDY (§6) and the first-slice plan converge on, and it is the only one that satisfies the layering invariant + shadow-first rhythm + the full tiny-detail ledger. Alternatives considered and rejected below.

### Components

```
src/sim/cell_validation/                  // NEW module (sim/, no render/ui/audio/net dep)
  mod.rs          — CellValidator<'a> facade (borrows grids; never mutates; never owns storage)
  cell_index.rs   — cell_index() (y*0x200+x) + get_cellclass_fallback() (dummy)   ← L1/L2/L3
  passability.rs  — check_passability_rect + check_cell_passability (per-cell)     ← L4..L12
  occupancy.rs    — check_occupancy_rect + Reservation surface                     ← L13..L19
  find_nearby.rs  — find_nearby_passable_cell (diamond-ring + selection)           ← L20..L29
  cell_validation_parity_tests.rs — replay/parity harness (P7)                     ← acceptance
```

`ReservationGrid` (the `+0xDC` per-house/site surface) is a **DEFERRED-AI seam**: declared in the `Reservation` enum + an `Option<&ReservationGrid>` field, but not built or hashed until AI placement lands (then it joins `state_hash` behind its own version bump). For 30-player scale it is keyed by `InternedId` house, NOT a 32-bit mask; the `1<<(idx&0x1F)` rule is reproduced as a query rule (L15) only.

### Interfaces / Contracts (signatures sketch — proposed, not written into the tree)

```rust
//! Cell-validation substrate: read-only rectangle passability/occupancy predicates
//! plus the nearby-passable-cell search — the primitives pathfinding, placement,
//! spawn, scatter, and dock-approach consume. Borrows the occupancy/path/terrain
//! grids; never mutates; never owns storage. Depends only on sim grids + rules data.
//! Never depends on render/ui/sidebar/audio/net.

pub const CELL_ROW_STRIDE: u32 = 0x200;   // L1 — fixed stride, NOT loaded-map width
pub const MAX_CELL_INDEX: u32 = 0x3FFFF;  // L1
pub const SPEED_TYPE_WINGED: i32 = 4;     // L6

/// Packed top-left + size; signed cell coords (engine CellRect / CellStruct).
pub struct CellRect { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

/// 9-arg passability config (engine CheckPassability args 4-9).
pub struct PassabilityQuery {
    pub speed_type: SpeedType,
    pub required_zone_id: i32,         // -1 disables the zone-id compare (L7)
    pub movement_zone: MovementZone,   // matrix ROW source — NOT speed_type (L7)
    pub required_height_or_level: i32, // -1 = unrestricted; FNPC passes -1 (L21)
    pub bridge_aware: bool,            // arg8: GetZoneID + height/layer selection (L8)
    pub reject_any_overlay: bool,      // arg9: reject overlay before per-cell (L5)
}

/// Occupancy reservation arg (engine CheckOccupancy arg2).
pub enum Reservation { SkipReservation /* -1 */, House(InternedId) }  // L15

/// FNPC config (engine Find_Nearby_Passable_Cell caller args).
pub struct NearbyQuery {
    pub passability: PassabilityQuery,
    pub allow_bridge_cells: bool,  // FNPC filter applied AFTER passability (L22)
    pub check_height: bool,        // FNPC ±2 internal gate (separate from required_height)
    pub check_occupancy: bool,     // call check_occupancy_rect(rect, SkipReservation) (L23)
    pub radius_cap: u16,           // min(Speed+Sight, 32), computed by caller (L20)
    pub target_cell: Option<(i32, i32)>, // None => frame-counter (L24); Some => nearest-dist (L25)
}

/// A non-null cell reference — Some(real) or the dummy carrying the requested coord (L2).
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
    pub fn get_cellclass_fallback(&self, x: i32, y: i32) -> CellRef<'a>;            // L1/L2
    pub fn check_passability_rect(&self, rect: CellRect, q: &PassabilityQuery) -> bool; // L4..L12
    fn check_cell_passability(&self, cell: CellRef<'a>, q: &PassabilityQuery) -> bool;  // L6..L11
    pub fn check_occupancy_rect(&self, rect: CellRect, r: Reservation) -> bool;     // L13..L19
    pub fn find_nearby_passable_cell(
        &self, seed: (i32, i32), q: &NearbyQuery, frame_counter: u32,              // L24 thread the counter
    ) -> Option<(u16, u16)>;                                                        // L20..L29
}
```

### Data Flow

The validator is a pure read-only service with **no tick phase of its own**; existing phases call it:
- Movement/pathfinding (Phase 2/3): `Can_Enter_Cell` / A* fallback → `check_passability_rect` / `find_nearby_passable_cell` (seam with pathfinding study).
- Production spawn / war-factory exit (Phase 7): `find_spawn_cell_near_structure` fallback → `find_nearby_passable_cell`.
- Scatter (Phase 7): scatter target → `find_nearby_passable_cell`.
- Miner dock / chrono-return: seed = dock cell + DockOffset (L28) → `find_nearby_passable_cell`.

The ONE hash-relevant consumer is the FNPC selected cell (L24), which feeds `Set_Destination`/spawn position (hashed). The sim per-tick frame counter — **`World::binary_frame`, the existing `g_CurrentFrameCounter` analog** (`src/sim/world/mod.rs:302`; committed late at end of `advance_tick`, `:1742`, so it holds the pre-increment frame N this tick runs under — read it AS the current frame, see Design-review correction 1) — is threaded into the `frame_counter` arg explicitly (L24/L26), never an RNG substitute.

### Error Handling

`get_cellclass_fallback` never returns `None` (L2) — OOB yields `CellRef::Dummy { coord }`. `find_nearby_passable_cell` returns `None` for the no-candidate case (L27); the caller clears the destination. No panics on degenerate rects: `width<=0`/`height<=0` returns true for passability (L4) and skips the blocker scan but runs the corner check for occupancy (L16).

### Testing Strategy

Substrate-boundary unit tests first (pure sim, no engine spin-up), one per ledger item, then a replay/parity harness:
- **P1 (cell index/dummy):** `cell_index_uses_512_wide_stride_not_map_width` (L1); `get_cellclass_oob_returns_dummy_with_requested_coord` (L2); `cell_validator_is_read_only_no_hash_change` (shadow guarantee).
- **P2 (passability):** all-cells AND-fold + zero-size-true (L4); overlay-reject caller flag (L5); Winged fast-pass (L6); MovementZone-rowed zone compare + `-1` skip (L7); shadow-agrees-with-PathGrid-on-plain-cells (surfaces, never equalizes, divergence).
- **P3 (occupancy):** `-1` skips reservation but rejects cell blockers (L13/L14/L15); blocker-order matches engine (L14); house-index blocks matching only (L15); zero-size rect still runs corner check (L16); out-of-play rect rejected by corner (L16/L19).
- **P5 (find_nearby):** diamond-ring visit order + radius cap (L20); occupancy call uses SkipReservation (L23); allow_bridge filters after passability (L22); no-candidate → None (L27); required-height -1 (L21).
- **P6 (authoritative, hash):** frame-counter modulo selection, direct preferred (L24); target → nearest-distance (L25); same-tick aliasing (L26); chrono seed = dock+DockOffset (L28); `snapshot_version_is_18` + roundtrip; war-factory-exit cell matches baseline.
- **P7 (replay):** scripted ~600-tick stream replayed twice → identical `Vec<hash>`; baseline-hash equality captured at the P6 flip commit; chrono-return cell deterministic over replay.

### Determinism

`CellValidator` is borrow-only and stateless → nothing serialized, nothing added to `state_hash`; P1–P5 are hash-neutral by construction. FNPC selection is frame-counter-driven (L24), consuming neither RNG stream → lockstep-safe; the only requirement is threading the *same* sim frame counter on every client — that counter is `World::binary_frame`, derived from the serialized/hashed `total_sim_ms`, so it is already shared by construction (Design-review correction 1). The P6 cell-choice change is the single hashed delta → `SNAPSHOT_VERSION 17→18` + replay harness.

---

## Shadow-First Rollout Shape

In the proven rhythm (shadow → invert → drop asserts → authoritative → bump → harness), mirroring Mission/Radio and the first-slice plan:

| Slice | What | Hash? |
|---|---|---|
| **P0** | Research gate (mostly closed PASS-2). Remaining before P3/P6: re-read `IsRectInPlayfield 0x00578390` 4-corner formula (L19); dummy `0x00ABDC50` runtime-init field values (L3); spot-confirm speed-table dump values (L12). | n/a |
| **P1** | `CellValidator` + `get_cellclass_fallback` (cell index / dummy). | **read-only** |
| **P2** | `check_passability_rect` + `check_cell_passability`; shadow-assert agreement with `PathGrid::is_walkable` where representable, surface divergence elsewhere. (Blocked on P0 for per-cell math.) | **read-only** |
| **P3** | `check_occupancy_rect` + `Reservation`. | **read-only** |
| **P4** | Invert: route the spawn fallback (lowest-risk caller) through the facade predicates; bool grid becomes a cache. Coordinate `Can_Enter_Cell` seam. | **read-only** (predicates agree) |
| **P5** | `find_nearby_passable_cell` shadowed against `nearest_walkable_around` — assert candidate *sets*, do not yet change the chosen cell. | **read-only** |
| **P6** | Authoritative: replace `nearest_walkable_around` with FNPC in spawn/exit/scatter/chrono-return; chosen cell (frame-counter selection) becomes authoritative. **Bump `SNAPSHOT_VERSION 17→18`.** | **HASH-RELEVANT** |
| **P7** | Global parity/replay harness — bit-identical per-tick hash sequence vs baseline. | acceptance |

**Read-only vs hash-relevant boundary:** P1–P5 change no hashed bit (the facade only reads; the shadow asserts surface divergence without acting on it). The single authority flip is P6 (FNPC cell choice); `SNAPSHOT_VERSION` and the parity harness apply there. `ReservationGrid` is DEFERRED-AI — when it lands it joins `state_hash` behind its own future bump, not this slice's.

---

## Ad-hoc Rust to Retire (file:symbol)

Retire only at/after the authority-flip (P4/P6); until then they coexist as the shadow-derived source.

| Current Rust (file:symbol, verified this run) | Smell | Replaced by |
|---|---|---|
| `src/sim/pathfinding/core.rs:1613` `PathGrid::is_walkable` (single bool) | collapses SpeedType/zone/height/overlay-reject into one bool (DRIFT L4–L11) | `CellValidator::check_passability_rect` + `check_cell_passability` |
| `core.rs:1642`/`:1614` `PathGrid::cell` / `is_walkable` width-based index + `None` OOB | wrong index width + `None` vs non-null dummy (L1/L2) | `CellValidator::get_cellclass_fallback` — `PathGrid` index stays as the cache; parity callers use the facade |
| `src/sim/production/production_spawn.rs:355` `nearest_walkable_around` (def; box-ring, first-match), call at `:290` (radius 12) | box-ring + deterministic first-match vs diamond-ring + frame-counter select (L20/L24) | `CellValidator::find_nearby_passable_cell` |
| `production_spawn.rs:237` `find_spawn_cell_near_structure` (preferred-offsets then ad-hoc fallback) | preferred offsets fine as caller policy; the fallback is ad-hoc | preferred offsets stay; fallback delegates to `find_nearby_passable_cell` |
| `production_spawn.rs` `cell_available_for_spawn` / `spawn_cell_passable` (scattered "is cell free" checks) | partial — no `+0x44/+0x4C/+0x11C`/reservation/playfield-corner (L14/L16) | `CellValidator::check_occupancy_rect` |
| `src/sim/pathfinding/passability.rs:149` `zone_layer_for_speed_type` (SpeedType→row shim) | the validator's zone row is MovementZone, not SpeedType (L7) — retire IF any validation path reaches it | `check_cell_passability` reads MovementZone directly; keep the shim only for audited SpeedType-only legacy callers |
| `src/sim/occupancy.rs:118` `OccupancyGrid::rebuild` from `EntityStore` (enter-order re-derivation, sort `:121`) | C22 DRIFT: re-derives cell-list order from enter-order, not the saved live insertion order serialized verbatim (L30) | serialize cell-list order directly per cell on save, restore in saved order on load — **save/load-substrate seam, not this slice** |

**Kept (verified-correct ingredients, NOT retire targets):** `MOVEMENT_ZONE_PASSABILITY` (`passability.rs:115`, test `matrix_matches_verified_native_dump`); `OccupancyGrid` insertion order (`occupancy.rs:200-217`). The facade reads these.

---

## Architectural Decisions

- **Follows** the substrate-program pattern: storage owner (`OccupancyGrid`/`PathGrid`) stays; the validator is a borrow-only read projection; the hash-relevant change is isolated and version-gated. Mirrors Factory/House + Mission/Radio.
- **Follows** the layering invariant: `sim/`-only, no upward deps.
- **Deviation:** reproduces the engine's `1<<(idx&0x1F)` reservation mask as a *query rule* but stores reservations by `InternedId` (30-player scale), not a 32-bit mask. Justified by `project_scale_target` — the mask is a scale-limiting internal, not observable behavior (the 32-slot cap is never hit at 8 players, and the query result is identical for valid house indices).
- **Tech debt accepted (documented, not silent):** C22 save/load cell-list-order serialization (L30) is a real DRIFT left as a save/load seam; the dummy runtime-init values (L3) and exact `IsRectInPlayfield` corner formula (L19) are UNCHECKED P0 items that must close before P3/P6.

## Alternatives Considered

1. **Extend `OccupancyGrid` in place with the cell-blocker bytes + a `check_occupancy_rect` method (no new module).** Rejected: it fuses two independently-verified validators (passability ≠ occupancy, STUDY C9 / first-slice plan "Must Remain Unchanged") into one type, and it would tempt storing `+0xDC` reservations as if they were entity occupants (first-slice "Do Not Do"). A dedicated read-only facade keeps the separation the binary enforces.
2. **Port the engine's literal `CellClass` (per-cell struct with `+0xE4/+0xE8/+0xDC/+0x124/+0x128` fields) + `MapClass` cell array.** Rejected: literal C++-port reflex (raw cell array, pointer lists). The verified behavior contract (predicates, ordering, selection) is what matters; current Rust already models the object-list order correctly via `OccupancyGrid`. A facade preserves semantics without the struct port.
3. **Make FNPC authoritative immediately (skip the shadow slices).** Rejected: it flips a hashed bit across ~40 callsites in one patch with no baseline to diff against — violates the shadow-first discipline and the first-slice plan's "migrate the lowest-risk caller first." P5 shadow + P7 baseline are the safety net.

---

## Open Questions / Assumptions for Design Review

1. **Caller-migration scope at P6.** Assumed: P6 migrates only human-reachable spawn/exit/scatter/chrono-return; AI-convoy/slave-deploy/crate-placement FNPC callers (STUDY §P2.2) migrate later but all share the frame-counter selection. Confirm this split is acceptable, or should P6 migrate all non-AI callers at once for a single clean hash flip?
2. **`IsRectInPlayfield` corner formula (L19).** UNCHECKED — only the call is confirmed. Must `decompile_function 0x00578390` before P3 lands the corner test. Block P3 on it, or land P3 with the blocker scan and add the corner check in a P3.5 once verified?
3. **Dummy runtime-init field values (L3).** UNKNOWN. Needs a live-init dump to define what `CellRef::Dummy` field reads return. Do any human-path callers actually read fields off the dummy after an OOB probe, or is the non-null-vs-`None` distinction the only observable? If the latter, L3 can stay deferred.
4. **C22 save/load (L30) sequencing.** Confirmed DRIFT but scoped out of this slice. Assumed it lands with the save/load substrate (active-vector-order serialization already exists). Confirm it is not a blocker for P6 (it is not — P6 is runtime cell choice, C22 is a load-time order issue).
5. **STUDY C22 wording correction.** Recorded: current `OccupancyGrid::rebuild` sorts by `(occupancy_enter_order, stable_id)` (`occupancy.rs:121`), not "creation/interned ID" as the STUDY says. The DRIFT substance is unchanged (enter-order re-derivation ≠ serialized insertion order). Flag to STUDY author for a one-line doc fix.
6. **`check_height` ±2 gate (L21).** The FNPC internal `±2` height gate is in `NearbyQuery::check_height` but its exact comparison (against seed level? against required height?) is not re-read this run — confirm before P5.
