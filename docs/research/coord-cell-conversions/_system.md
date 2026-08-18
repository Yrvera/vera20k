# coord-cell-conversions — System Synthesis

**Decode run:** `/decode-system coord-cell-conversions` (2026-05-24)
**Scope:** 26 symbols (24 functions + 2 structs), all PROOFED/PROOFED-YELLOW
**Anchors:** `ObjectClass__Get_Cell_Packed` (0x0041bea0), `CellClass__Get_Center_Coords` (0x00480a30), `MapCoord_Set` (0x0042d470)
**Parity report:** see [_parity.md](_parity.md) — **8 DRIFT, 0 FIXED, 1 MISSING, 25 INTERNAL-ONLY** (6 rows re-classified from INTERNAL-ONLY → DRIFT on 2026-05-24 after the byte-perfect/pixel-perfect parity doctrine update — see CLAUDE.md "Parity-judgment burden of proof — default to DRIFT". The refinery dock-reference row was re-opened after the 2026-05-24 re-swarm split accepted target, GetDockCoord arrival cell, and QueueingCell; the follow-up re-swarm then verified that no physical NW+3 -> NW+2 bridge exists. Rows 20 and 22 are the same underlying building render-anchor mechanism analyzed from two perspectives.)

---

## Summary

This is the foundational coord/cell math layer of gamemd.exe. Every system above it — combat targeting, pathfinding, building placement, bullet trajectories, splash damage, rendering, fog of war — depends on these primitives for "where is this entity, in which cell, at what altitude." The layer has no observable behavior on its own; it produces the numbers that drive observable behavior elsewhere.

The Rust engine takes a different architectural approach: where gamemd uses a `CoordStruct` (3 × int32 leptons) and a packed `CellStruct` (4 bytes, signed shorts), Rust stores entity positions natively as `Position { rx, ry, z, sub_x, sub_y }` (cell-indexed with sub-cell offset) and uses `(u16, u16)` tuples for cell coordinates throughout. Most of the conversion machinery gamemd needs (lepton → cell, packed CONCAT22, vtable dispatch) is unnecessary in Rust because cell coords are stored directly. The INTERNAL-ONLY parity rows reflect this: different internal mechanisms, same observable output.

Eight real DRIFTs are currently tracked (2 original + 6 re-classified from INTERNAL-ONLY after the parity doctrine tightened on 2026-05-24; rows 20 and 22 are facets of the same building render-anchor issue). The previously "fixed" refinery dock-reference row is re-opened: `refinery_pad_cell` matches the stock 4x3 `GetDockCoord` arrival coordinate `(NW+2,NW+1)`, but the Rust state-machine handoff still compresses several gamemd radio/timer stages. The 2026-05-24 `0x16` verification and follow-up re-swarm proved that `UnitClass::Receive_Radio(0x16)` does **not** move the unit to `GetDockCoord`; Drive arrival can leave the miner stopped at the accepted cell `(NW+3,NW+1)` while the refinery destination remains active, the first ordinary `0x16` can set the locomotor/facing rate and return without unloading, and a later/already-synchronized `0x16` can send `0x15` directly from that accepted-cell state. One MISSING entry is gated on a future feature (bounce/meteor animations). All remaining DRIFTs are tracked in [_parity.md](_parity.md) with required-evidence sub-bullets stating what would be needed to downgrade.

---

## Symbol scope (26)

**Lepton/cell conversion primitives (8):**
- `ObjectClass__Get_Cell_Packed` (0x0041bea0) — lepton → packed CellStruct; sign-correct shift gate
- `CellClass__Get_Center_Coords` (0x00480a30) — cell → CoordStruct (cell × 256 + 128 for X/Y, terrain Z)
- `MapClass__IsCoordsInPlayfield` (0x005785f0) — lepton bounds gate; sign-correct shift then delegates to cell-side
- `MapClass__Is_Cell_In_Playfield` (0x00578460) — cell-side diamond-coordinate bounds check (X+Y, X−Y)
- `MapClass__CellCoordToLinearIndex` (0x0056d430) — (x,y) → linear array index
- `MapCoord_Set` (0x0042d470) — CellStruct setter
- `MapCoord_Add` (0x0042d510) — CellStruct component-wise add
- `MapCoord_Step_By_Direction` (0x0042d490) — cell + cardinal direction (0–7) or tube traversal (8)

**CoordStruct primitives (5):**
- `CoordStruct__Set` (0x0041c230) — 3-int32 setter
- `CoordStruct__Distance3D` (0x0041c380) — sqrt(dx²+dy²+dz²) via LUT + Math__ftol (determinism hazard)
- `CoordStruct__FromDoubles` (0x004399a0) — x87 FPU stack → ints (determinism hazard)
- `CoordStruct__VecAdd` (0x006ce240) — component-wise add
- `CoordStruct__ScaleByFactor` (0x0075f540) — linear interpolation via Math__ftol (determinism hazard)

**Polymorphic position accessors (vtable dispatch chain, 6):**
- `AbstractClass__GetCoords` (0x004104c0) — vtable+0x48 root, returns zero sentinel (0,0,0)
- `ObjectClass__GetCoords` (0x005f65a0) — base override, returns Location at +0x9C/A0/A4
- `ObjectClass__GetRenderCoords` (0x0041be00) — wraps vtable+0x48 dispatch
- `BuildingClass__GetCoords` (0x00447ac0) — vtable+0x48 override; returns foundation center
- `BuildingClass__GetRenderCoords` (0x00459ef0) — vtable+0xAC override; returns Location − 128 (half-cell NW)
- `BuildingClass__GetDockCoord` (0x00447b20) — vtable+0xA8; Weeder pad / stock refinery arrival coord / approach-angle / type-defined slot
- `ObjectClass__GetOccupiedCell` (0x005f6960) — vtable+0x1BC; reads Location and looks up CellClass

**Raw position mutators (1):**
- `ObjectClass__Set_Raw_Coords` (0x005f6940) — direct write to Location fields

**Foundation lookup helpers (2):**
- `BuildingTypeClass__GetFoundationWidth` (0x0045ec90) — index into `g_FoundationWidthTable` @ 0x008192b8
- `BuildingTypeClass__GetFoundationHeight` (0x0045eca0) — index into `g_FoundationHeightTable` @ 0x00819310

**Structs (2):**
- `CoordStruct` (12 bytes, 3 × int32 leptons: X +0, Y +4, Z +8)
- `CellStruct` (4 bytes packed: short X +0, short Y +2; CONCAT22(Y, X))

**TS-excluded:** none. **Phase 0 scope-explorer additions:** +7 symbols (call-graph deepening + vtable adjacency + string sweep), kept scope under the 38-ceiling.

---

## The five reference frames (THE load-bearing finding)

CLAUDE.md's "Coordinate conventions when porting binary offsets" section names five frames. This decode run verified all five against the binary and adds detail:

| # | Frame | Source | Unit | Verified at |
|---|-------|--------|------|-------------|
| 1 | **Location** | `(class) + 0x9C` (X), `+0xA0` (Y), `+0xA4` (Z) — direct read | leptons | `ObjectClass__GetCoords @ 0x005f65a0`, `ObjectClass__Set_Raw_Coords @ 0x005f6940`, `BuildingClass__GetRenderCoords @ 0x00459ef0` |
| 2 | **Get_Cell_Packed** (NW cell) | `vtable+0x1B8` (`ObjectClass__Get_Cell_Packed` @ 0x0041bea0) | packed CellStruct (cell index) | sign-correct shift `(v + (v>>31 & 0xFF)) >> 8` confirmed |
| 3 | **GetCoords** (foundation center for buildings) | `vtable+0x48` — for buildings: `BuildingClass__GetCoords @ 0x00447ac0` | leptons | formula `Location + ((W−1)*128, (H−1)*128, 0)` confirmed |
| 4 | **Foundation outline** | `BuildingTypeClass.vtable+0x90` (returns cell-delta array) | cells relative to NW | not directly decoded; iterated by `MapCoord_Add` callers |
| 5 | **Dock/refinery reference points** | `BuildingClass::Receive_Radio(0x0E)`, `BuildingClass__GetDockCoord @ 0x00447b20`, art `QueueingCell` | mixed (cells + lepton centering) | accepted target = NW+(3,1); stock 4x3 `GetDockCoord` arrival cell = NW+(2,1); QueueingCell fallback = NW+(4,1) |

**Critical correction after 2026-05-24 re-swarms:** `+0x16BC` is `Weeder`, not stock refinery. Stock GAREFN/NAREFN use `+0x16BB Refinery=yes`; for a 4x3 foundation that `GetDockCoord` branch still resolves to cell NW+(2,1). This is a PerCellProcess/0x15 arrival coordinate, not the accepted move target. It must not replace the accepted `CAN_DOCK(0x0E)` target at NW+(3,1), and it is distinct from art `QueueingCell=4,1` at NW+(4,1). The follow-up `0x16`/MissionEnter/DriveLocomotor swarm verified that gamemd does not physically bridge NW+(3,1) -> NW+(2,1): the miner can stop at accepted NW+(3,1), keep its refinery destination, and unload through a later/aligned `0x16` path. The `(-0x80, +0x80)` lepton offset cited in CLAUDE.md as Frame #5 "refinery-specific" is actually the **general approach-angle offset** used by `GetDockCoord` Branch 3 (atan2-based, applies to any building with approach-angle docking).

---

## BuildingClass position trinity (the FOUNDATION_CENTER_INVESTIGATION solver)

The `FOUNDATION_CENTER_INVESTIGATION.md` author was stuck because they assumed gamemd had two building positions (NW corner vs. foundation center) and neither matched the SHP render anchor. This decode revealed gamemd actually has **three** distinct building positions, each at a different vtable slot:

| Slot | Function | Output | Used for |
|------|----------|--------|----------|
| `vtable+0x48` | `BuildingClass__GetCoords @ 0x00447ac0` | `Location + ((W−1)*128, (H−1)*128, 0)` = **foundation geometric center** | Combat targeting, AoE center, projectile spawn, health bar coords |
| `vtable+0xAC` | `BuildingClass__GetRenderCoords @ 0x00459ef0` | `Location − (128, 128, 0)` = **half-cell NW of NW corner** | SHP sprite draw anchor |
| `vtable+0xA8` | `BuildingClass__GetDockCoord @ 0x00447b20` | Branch-specific (Weeder pad / stock Refinery=yes arrival coord / approach angle / type-defined slot / fallback to GetCoords) | Harvester dock arrival, repair depot, dock entry |

The investigation only tried (1) NW corner and (2) foundation center for the sprite anchor. The actual anchor is (3) `Location − 128` — a half-cell NW shift. Rust's depth YSort at [shp.rs:220](src/app_instances/shp.rs:220) already applies this correction (`sy − TILE_HEIGHT/2`), which is why building placement looks correct in practice. The shifted render origin combined with per-sprite SHP atlas offsets produces the same on-screen position as gamemd's.

**Ghidra labeling oddity worth flagging:** the address `0x00410600` is labeled `ObjectClass__GetCoords` but actually decompiles to `AbstractClass__Release(); return;`. Approximately 74 vtables bind this address. The real `ObjectClass::GetCoords` is at `0x005f65a0` (19 vtable bindings). Either Ghidra's RTTI labeler is wrong about 0x00410600, or those 74 vtable slots are pointing to a different function via inlining/aliasing. Worth a follow-up `/re-investigate` to clarify whether any live YR code path actually calls `Release()` thinking it's `GetCoords()`.

---

## Control flow — the lepton ↔ cell round-trip

```
                 ┌──────────────────────────────┐
                 │  Entity has Location (leptons)│
                 │  at fields +0x9C/A0/A4        │
                 └────────────┬─────────────────┘
                              │
            ┌─────────────────┴─────────────────┐
            │                                    │
   vtable+0x48 (GetCoords)              vtable+0x1B8 (Get_Cell_Packed)
   - returns CoordStruct                 - applies sign-correct shift
   - leptons (Frame #3)                  - returns packed CellStruct
   - foundation center for buildings     - cells (Frame #2)
            │                                    │
            ▼                                    ▼
   Used by combat, range,                Used by pathfinding, occupancy,
   AoE, projectile spawn,                shroud, placement validity,
   health bars                           cell-grid lookups
            │                                    │
            ▼                                    ▼
   CoordStruct__Distance3D              CellClass__Get_Center_Coords
   (Math__ftol, x87 — non-det)          (cell × 256 + 128 + terrain Z)
            │                                    │
            └─────────────────┬─────────────────┘
                              ▼
                  Round-trip closes; both
                  frames are intercompatible
                  via the sign-correct shift.
```

The sign-correct floor-shift `(v + (v >> 31 & 0xFF)) >> 8` appears identically in:
- `Get_Cell_Packed` (the canonical implementation)
- `IsCoordsInPlayfield` (re-used inline before delegating to cell-side)
- `CellClass__GetGroundHeight` (re-used inline for height lookup)
- `MapCoord_Set` callers that take lepton inputs (sign-correct shift externally before calling)

It's a **shared primitive**, not duplicated per call site. Negative-lepton handling produces correct floor division (lepton −1 → cell 0, not cell −1).

---

## INI surface

None. The coord/cell layer is pure code — no INI keys. INI-driven foundation shapes (`Foundation=` in `art.ini`) feed `BuildingTypeClass+0xef0` (the foundation type index) which then indexes into the static `g_FoundationWidthTable` (`0x008192b8`) and `g_FoundationHeightTable` (`0x00819310`).

**Foundation table data (first 8 entries verified live):**

| Index | Width | Height | Common foundation |
|-------|-------|--------|-------------------|
| 0 | 1 | 1 | 1×1 (e.g. PILLBOX) |
| 1 | 2 | 1 | 2×1 |
| 2 | 1 | 2 | 1×2 |
| 3 | 2 | 2 | 2×2 |
| 4 | 2 | 3 | 2×3 |
| 5 | 3 | 2 | 3×2 |
| 6 | 3 | 3 | 3×3 |
| 7 | 3 | 5 | 3×5 |

(Continues to 22 standard YR foundation shapes; full table in `fn-buildingtype-getfoundationwidth.md` and `fn-buildingtype-getfoundationheight.md`.)

---

## Observable behaviors

This layer doesn't produce observable output on its own; it feeds the systems that do. The places where this layer's correctness matters most for the player:

1. **Building placement** — `BuildingClass__GetCoords` foundation center is used for combat targeting; wrong = projectiles hit corner instead of middle.
2. **Refinery docking reference points** — accepted `0x0E` target, `GetDockCoord` arrival cell, and QueueingCell are three different cells; collapsing them causes visible/state drift (see DRIFT below).
3. **Splash damage radius** — `CoordStruct__Distance3D` 3D Z component; wrong = bridge/elevated units take or skip splash incorrectly (see DRIFT below).
4. **Sub-cell unit interpolation** — `CoordStruct__ScaleByFactor` lerp during drive-track stepping; wrong = jittery movement.
5. **Pathfinding cell neighbors** — `MapCoord_Step_By_Direction` direction deltas; wrong = paths go the wrong way (CLAUDE.md flags facing-vs-drive-track confusion as a recurring bug class).
6. **Map bounds** — `Is_Cell_In_Playfield` diamond check; wrong = units placed at map edges get rejected/accepted incorrectly.

---

## Edge cases and parity hazards

### 0. Playfield bounds check — diamond + height correction missing (DRIFT, re-classified)

**gamemd:** `MapClass__Is_Cell_In_Playfield` uses diamond-coordinate bounds (`X+Y`, `X−Y` against MapClass fields 0xf4/0xfc/0x100/0x104/0x108). With `param3=1`, applies height correction at boundary cells via `[cell+0x11b]` (height level) and `[cell+0x11c]` (flag) — extends the boundary by 1 cell for elevated terrain. Used by 72 callers across pathfinding, AI, locomotors, placement, shroud.
**Rust:** rectangular bounds (`rx < width && ry < height`) at `src/map/resolved_terrain.rs:267-272`, `src/sim/pathfinding/core.rs:908`, `src/map/terrain.rs:452-466`. Height correction at boundary cells **not implemented**.
**Impact:** Two distinct gaps — (a) at the isometric playfield diamond boundary, Rust accepts cells gamemd would reject (and vice versa); (b) maps with raised plateaus near edges have a 1-cell boundary extension in gamemd that Rust doesn't apply. Both affect spawn placement, landing, dock entry, unit movement, AI scans, shroud at map edges. Player-visible whenever a unit/projectile reaches the boundary.
**Fix surface:** `src/map/resolved_terrain.rs` (add diamond-coordinate bounds check), plus a height-aware boundary helper that reads cell metadata for elevated terrain. Affects every caller of the bounds primitive.

### 1. Refinery dock reference split — DRIFT re-opened 2026-05-24

**gamemd:** Stock refinery docking uses at least three cells for a refinery at NW `(10,10)`:
- accepted `BuildingClass::Receive_Radio(0x0E)` `MOVE_TO_CELL(0x12)` target: `(13,11)` = NW+(3,1)
- later `UnitClass::PerCellProcess` / `BuildingClass__GetDockCoord` arrival cell: `(12,11)` = NW+(2,1)
- art `QueueingCell=4,1` fallback/wait target: `(14,11)` = NW+(4,1)

The old coord-cell row incorrectly attributed NW+(2,1) to `BuildingTypeClass+0x16BC` as if that were stock refinery. Re-swarm verified `+0x16BC = Weeder`; stock GAREFN/NAREFN use `+0x16BB Refinery=yes`. For a 4x3 foundation, the stock `Refinery=yes` GetDockCoord branch still resolves to NW+(2,1).

**Rust:** `refinery_can_dock_queue_cell` correctly preserves accepted NW+(3,1). `refinery_pad_cell` now returns NW+(2,1), which matches the stock 4x3 GetDockCoord arrival coordinate. The remaining parity risk is the handoff: Rust currently transitions to `Linked` at the accepted cell and then mutates only the miner snapshot to the pad cell in `phase_linked`. gamemd's handoff is source-aware: `UnitClass::Receive_Radio(0x16)` can send `0x15` from the stopped accepted-cell path after rate/idle/destination/mission gates, while `UnitClass::PerCellProcess` has both a `GetDockCoord` equality branch and a contact-flag adjacent-building branch.

**Follow-up re-swarm result (2026-05-24):** The source-order uncertainty is resolved enough for implementation. `FootClass::Mission_Enter` sends one `CAN_DOCK(0x0E)` per mission dispatch and stock `[Enter] Rate=.016` produces a 14-16 frame retry cadence. If `0x12` returns ROGER, the building sends only the accepted move to NW+(3,1); it does not send `0x18`/`0x16` in that pass. If a later retry gets `0x12 == 0x14` ("already there"), the building sends `0x18` then `0x16` synchronously.

`UnitClass::Receive_Radio(0x16)` is not a movement command. On the first ordinary call with the locomotor/rate timer not synchronized, it calls the locomotor vtable `+0x4C` with `0x4000` and returns before the unload send. Later/already-synchronized `0x16` can send `0x15` if the unit is not moving, still has a destination building, and is in mission 7. `DriveLocomotion` can make `Is_Moving_Now == false` while `Foot+0x5A4` still points at the refinery destination, so this can happen at the accepted NW+(3,1) cell. `UnitClass::PerCellProcess` still has the `GetDockCoord` equality branch, but accepted NW+(3,1) does not equal stock `GetDockCoord` NW+(2,1), and a second contact-flag/adjacent-building `0x15` branch also exists. Rust must model the staged radio/timer handoff instead of forcing a physical NW+3 -> NW+2 move or mutating only a miner snapshot to the pad.

### 2. AoE splash damage ignores Z elevation (DRIFT, player-visible)

**gamemd:** `Apply_area_damage` calls `CoordStruct__Distance3D` with full `(dx, dy, dz)` delta. Units at different elevations factor in `dz²` toward the splash radius.
**Rust:** `combat_aoe.rs:154` calls `lepton_distance_sq_raw(dx, dy)` — Z is silently dropped.
**Impact:** A unit on a bridge above a ground target (z = 4 levels = 416 leptons offset) computes 2D distance in Rust but 3D in gamemd. Rust may include the bridge unit in splash damage when gamemd would exclude it (and vice versa). Fires every AoE detonation near elevated units — common with artillery and any `Spread > 0` weapon near bridges.
**Fix surface:** `src/sim/combat/combat_aoe.rs:154`, `src/sim/combat/mod.rs:2231`. Add Z delta to the distance computation: `dx² + dy² + dz²` using deterministic integer sqrt (already available as `isqrt_i64` in `src/util/fixed_math.rs:246`).

### 2b. Distance3D LUT vs isqrt (DRIFT, re-classified)

**gamemd:** `CoordStruct__Distance3D` uses `Sqrt_Approx` (8192-entry mantissa LUT at `DAT_008650bc`, returns `float10` truncated by `Math__ftol`).
**Rust:** `compute_in_range` (`src/sim/combat/in_range.rs:150-206`) uses `isqrt_i64` (`src/util/fixed_math.rs:246`) — deterministic integer floor-sqrt via Newton's method.
**Impact:** the two functions are NOT bit-identical. LUT approximation diverges from true integer sqrt by up to 1 lepton at many inputs. At a range threshold, this flips the `distance ≤ range` gate's inclusion of borderline targets — a unit exactly at max range may be in-range in Rust but out-of-range in gamemd (or vice versa). Fires on every range check (combat AI, projectile homing, target acquisition).
**Fix surface:** `src/util/fixed_math.rs` — either reproduce the LUT approximation exactly (preserves gamemd parity, sacrifices a tiny bit of accuracy) or accept the bit-diff and validate the actual fixture set. The current `isqrt_i64` is mathematically cleaner but observably different.

### 2c. CellCoordToLinearIndex stride formula (DRIFT, re-classified)

**gamemd:** `(f8 + 1 + f4) * cell_Y + cell_X` — diamond-aware stride from MapClass dimension fields.
**Rust:** `ry * width + rx` (`src/map/resolved_terrain.rs:267-272`) — rectangular grid width.
**Impact:** Equivalent only if `width == f8 + 1 + f4` for every map gamemd can load. This has not been proven. If it differs by even 1 cell on any map, zone-index lookups misalign across the entire flat array — affects pathfinding zone membership, bridge connectivity, flood-fill reachability.
**Fix surface:** verify the stride equivalence empirically (load every standard YR map, compare `width` to the gamemd-computed stride). If they ever differ, port the gamemd stride formula.

### 2d. Building render anchor — partial compensation = DRIFT (re-classified)

**gamemd:** `BuildingClass__GetRenderCoords` returns `Location − (128, 128, 0)` — half-cell NW shift. This render origin is used CONSISTENTLY across every downstream consumer: sprite anchor, health bar, selection bracket, status icons, attack lines, garrison indicator. The algebraic health bar formula is `pip0.Y = pLoc.Y − 11 − Height*15` derived from foundation-center input.
**Rust:** **Inconsistent compensation across consumers.** Depth YSort at `src/app_instances/shp.rs:220` applies `sy − TILE_HEIGHT/2` (= -15 pixels, matching the -128 lepton shift) so the sprite layer looks correct. BUT the health bar uses `start_y = sy − 6 − Height*15` instead of gamemd's `-11`. Per `FOUNDATION_CENTER_INVESTIGATION.md` the `-6` is an empirical fudge that the investigation itself documents as "a minor artifact of the sprite anchor mismatch" producing a "5px difference consistent across all buildings." Other downstream render-pipeline consumers (selection bracket, status icons, attack lines, garrison indicator) have not been audited for the same compensation.
**Impact:** Confirmed: every building's health bar sits 5 pixels too high on every match. Possible: selection bracket, status icons, attack lines have related drift. Partial compensation across some paths while others diverge is a fragility, not parity — the gamemd render origin produces consistent downstream behavior; the Rust patchwork does not.
**Fix surface:** the canonical fix is to implement Rust's render pipeline so that all consumers read from the same render-origin position (matching gamemd's GetRenderCoords semantics), not have each consumer apply its own empirical offset. Replace `shp.rs:220`'s `sy − TILE_HEIGHT/2` and the health bar's `-6` with a single unified `building_render_origin` that all consumers use. Then the algebraic `-11` from gamemd works directly without fudge.

### 3. CoordStruct__FromDoubles MISSING (gated on future feature)

The bounce/meteor animation system (`AnimClass__AI` + `AnimClass__ProcessBounceResult`) is not implemented in Rust. When it is implemented, the FPU-stack double-to-int conversion that `FromDoubles` performs must use a **deterministic rounding convention** (truncation or fixed-point math), not x87 `Math__ftol` which is FPU-control-word-dependent and would break lockstep determinism. Not currently a player-visible issue because the calling system doesn't exist yet.

### 4. Determinism hazards (general)

Three coord primitives use x87 `Math__ftol` and are determinism hazards for any sim-side use:
- `CoordStruct__Distance3D` (Sqrt_Approx LUT-based + Math__ftol) — used in sim by `Apply_area_damage`, locomotors, EMP, homing bullets. Rust uses `isqrt_i64` (integer Newton's method); correct.
- `CoordStruct__FromDoubles` — render/anim only currently; would be a sim hazard if used in damage paths.
- `CoordStruct__ScaleByFactor` (linear lerp) — render/anim only; Rust uses integer `next_delta * residual / 7` in `interp_sub_step` for drive-track smoothing.

All three are correctly handled by the Rust port for the cases that currently exist.

### 5. The half-cell building render shift

gamemd's `BuildingClass__GetRenderCoords` returns `Location − (128, 128, 0)` — a half-cell NW shift. Rust's depth YSort compensates via `sy − TILE_HEIGHT/2` at `src/app_instances/shp.rs:220` plus per-sprite SHP atlas offsets. This works in practice but is a known fragility — any future change to SHP draw-offset handling must preserve this compensation or building placement will visibly shift.

### 6. Diamond vs. rectangular playfield check

gamemd `MapClass__Is_Cell_In_Playfield` uses diamond-coordinate (`X+Y`, `X−Y`) bounds. Rust uses rectangular (`rx < width && ry < height`) bounds. For normal gameplay cells (inside the `LocalSize` rectangle), the two are equivalent. They differ only at the extreme isometric corners of the map — areas outside normal play. Marked INTERNAL-ONLY in parity but flagged here as a potential future edge case.

### 7. Sign-correct shift in foreign Rust helper

`src/sim/production/production_spawn.rs:462` has a local `lepton_to_cell` helper that uses **round-to-nearest** `(v + 128) / 256` instead of gamemd's **floor** `(v + (v>>31 & 0xFF)) >> 8`. Rust-comparer judged this an unrelated helper (not a substitute for `Get_Cell_Packed`) and didn't flag it as DRIFT, but for production exit-candidate cells with negative lepton offsets this could produce a different cell choice than gamemd would. Worth confirming the call site never sees negative leptons.

### 8. Foundation `+1` bib extension is dead code

`BuildingTypeClass__GetFoundationHeight` has a `wantBibExtension` parameter that adds +1 to the height. Per a prior 2026-04-24 Ghidra audit, **no caller in stock YR passes `wantBibExtension != 0`** — the +1 branch is unreachable. The HasBib flag itself has a separate live reader at `UnitClass::Can_Enter_Cell`, so the flag isn't dead system-wide, just dead at this function. Rust correctly omits the bib extension.

---

## Per-symbol doc index

**Functions (24):**
- [fn-get-cell-packed.md](fn-get-cell-packed.md)
- [fn-get-center-coords.md](fn-get-center-coords.md)
- [fn-is-coords-in-playfield.md](fn-is-coords-in-playfield.md)
- [fn-map-is-cell-in-playfield.md](fn-map-is-cell-in-playfield.md)
- [fn-cell-coord-to-linear.md](fn-cell-coord-to-linear.md)
- [fn-mapcoord-set.md](fn-mapcoord-set.md)
- [fn-mapcoord-add.md](fn-mapcoord-add.md)
- [fn-mapcoord-step-by-direction.md](fn-mapcoord-step-by-direction.md)
- [fn-coordstruct-set.md](fn-coordstruct-set.md)
- [fn-coordstruct-distance3d.md](fn-coordstruct-distance3d.md)
- [fn-coordstruct-fromdoubles.md](fn-coordstruct-fromdoubles.md)
- [fn-coordstruct-vecadd.md](fn-coordstruct-vecadd.md)
- [fn-coordstruct-scalebyfactor.md](fn-coordstruct-scalebyfactor.md)
- [fn-abstract-getcoords.md](fn-abstract-getcoords.md)
- [fn-object-getcoords.md](fn-object-getcoords.md)
- [fn-object-getcoords-variant2.md](fn-object-getcoords-variant2.md)
- [fn-object-getrendercoords.md](fn-object-getrendercoords.md)
- [fn-object-getoccupiedcell.md](fn-object-getoccupiedcell.md)
- [fn-object-set-raw-coords.md](fn-object-set-raw-coords.md)
- [fn-building-getcoords.md](fn-building-getcoords.md)
- [fn-building-getrendercoords.md](fn-building-getrendercoords.md)
- [fn-building-getdockcoord.md](fn-building-getdockcoord.md)
- [../REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md](../REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md)
- [../FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md](../FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md)
- [../UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md](../UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md)
- [../UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md](../UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md)
- [../BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md](../BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md)
- [../DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md](../DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md)
- [fn-buildingtype-getfoundationwidth.md](fn-buildingtype-getfoundationwidth.md)
- [fn-buildingtype-getfoundationheight.md](fn-buildingtype-getfoundationheight.md)

**Structs (2):**
- [struct-coordstruct.md](struct-coordstruct.md)
- [struct-cellstruct.md](struct-cellstruct.md)

**Parity report:** [_parity.md](_parity.md) — 34 rows (8 DRIFT, 0 FIXED, 1 MISSING, 25 INTERNAL-ONLY).

---

## Next steps (your call)

Eight real disparities remain. Ranked by visibility × frequency (highest first):

1. **`/brainstorm refinery-dock-radio-timer-fsm`** — preserve accepted target NW+(3,1), stock GetDockCoord/arrival cell NW+(2,1), and QueueingCell NW+(4,1). The follow-up swarm proved no physical move to GetDockCoord; the stock path is a staged radio/timer handshake: accepted move only on `0x12 == 1`, 14-16 frame MissionEnter retry, `0x12 == 0x14` emits `0x18/0x16`, first ordinary `0x16` can only synchronize facing/rate, and later/aligned `0x16` can send `0x15` from stopped accepted-cell state. Repair Rust's miner/refinery FSM around those source-specific stages and add tests proving no NW+3 -> NW+2 physical move.
2. **`/brainstorm building-render-origin-unified`** — replace the patchwork (`sy − TILE_HEIGHT/2` in one path + `-6` fudge in health bar + unaudited compensations elsewhere) with a single canonical `building_render_origin` that matches gamemd's `GetRenderCoords` semantics. Then every downstream consumer (sprite, health bar, selection bracket, status icons, attack lines, garrison indicator) reads from the same position and the algebraic `-11` from gamemd works directly. Fixes the known 5px health bar drift AND any unaudited related drift in one structural change.
3. **`/brainstorm playfield-bounds-diamond-and-height-correction`** — port gamemd's diamond bounds + the `param3=1` height-corrected boundary path. 72 callers downstream — large blast radius but foundational for elevated-terrain edge correctness.
4. **`/brainstorm aoe-z-elevation-fix`** — wire Z into `combat_aoe.rs:154` using deterministic integer 3D sqrt. Every AoE detonation near elevated units.
5. **`/brainstorm distance3d-sqrt-parity`** — decide between (a) reproducing gamemd's `Sqrt_Approx` LUT bit-for-bit, or (b) keeping `isqrt_i64` and validating no observable range-gate diff via fixture trace. Affects every combat range check.
6. **`/brainstorm cellcoord-to-linear-stride`** — empirically verify `width == f8 + 1 + f4` across all maps; if any divergence, port gamemd's stride formula.
7. **`/brainstorm production-spawn-lepton-to-cell-rounding`** — confirm `src/sim/production/production_spawn.rs:462` never receives negative leptons, OR replace its round-to-nearest with the sign-correct floor that gamemd uses.
8. **Audit pass for related building-render consumers** — selection bracket, status icons, attack action lines, garrison occupant indicator. Each may have its own empirical offset hiding the same render-origin mismatch as the health bar. Could be folded into #2 or run as a separate `/disparity-scan building rendering`.

**Also surfaced for follow-up (per "no disparity is too small to surface" doctrine):**

- **`refinery_dock_cell` naming/routing** — Rust's `refinery_dock_cell` delegates to `refinery_can_dock_queue_cell` (returns `(rx+3, ry+1)`). That is correct for accepted `CAN_DOCK(0x0E)` admission but wrong if the name is interpreted as `GetDockCoord`. Rename/split helpers so accepted target, arrival coord, and QueueingCell cannot be collapsed again.

Lower-priority follow-ups:
- **CLAUDE.md memory update**: completed 2026-05-24. The "Coordinate conventions" section now preserves the three stock refinery reference points: accepted NW+(3,1), GetDockCoord/arrival NW+(2,1), QueueingCell NW+(4,1), and labels the `(-0x80, +0x80)` shift as the general approach-angle dock adjustment.
- **Investigate Ghidra label oddity at `0x00410600`**: 74 vtable slots bind an address that decompiles to `AbstractClass::Release`. If those calls run live, something is very wrong. If they don't, the label is just stale.
