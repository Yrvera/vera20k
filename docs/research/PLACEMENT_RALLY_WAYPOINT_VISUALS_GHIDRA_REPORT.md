---
title: Placement Ghost, Rally Lines & Waypoint Markers (Ghidra Research Report)
date: 2026-04-22
---

# Placement Ghost, Rally Lines & Waypoint Markers — Ghidra Research Report

**Addresses (primary):**
- `BuildingPlacement_OverlayRenderer` @ `0x006D5030` — per-frame foundation ghost render
- `BuildingPlacement_per_cell_draw` @ `0x0047EC90` — draws one PLACE.SHP diamond per cell
- `Cell_passability_building_placement` @ `0x0047C620` — per-cell validity check
- `FUN_006DA9D0` — selected factory/building rally-line renderer
- `FUN_006DAD60` — planning/queued waypoint path renderer
- `Tactical::DrawLine3D` @ `0x006DBB60` — generic Tactical vtable line primitive; not the verified selected factory rally/planning path renderer
- `UnitClass::DrawExtras` @ `0x0073D3C0..` — separate `FLAGFLY.SHP` unit extra block, not the tactical planning-path marker path
- `RulesClass::ReadAudioVisual` @ `0x006692D4` — reads `WaypointAnimationSpeed`
- `PLACE.SHP` string @ `0x00820080` — loader at `FUN_004A8850`
- `MOUSE.SHA` string @ `0x0082604C` — tactical planning-path marker source used by `FUN_006DAD60`
- `FLAGFLY.SHP` string @ `0x008458F8` — separate 14-frame unit extra animation block

**Confidence:** HIGH for placement-ghost per-cell algorithm + PLACE.SHP frame indexing.
HIGH for 2026-05-21 corrected rally/planning renderer identity: selected factory
rally lines use `FUN_006DA9D0`, and planning/queued waypoint lines use
`FUN_006DAD60`. HIGH that tactical planning markers use `MOUSE.SHA`; `FLAGFLY.SHP`
is a separate `UnitClass::DrawExtras` block.

**Active in YR:** Yes — all three systems.

**Relationship to prior work:**
- `ra2-rust-game-docs/TARGET_LINES_GHIDRA_REPORT.md` covers unit action lines.
  Its old `DrawLine3D` rally/planning attribution has been superseded by the
  2026-05-21 swarm reports: rally lines use `FUN_006DA9D0`; planning path lines
  use `FUN_006DAD60`.
- `ra2-rust-game-docs/RALLY_POINTS_AND_UNIT_SPAWNING.md` (1231 lines) covers
  rally-point GAMEPLAY (where units spawn, zone logic) but not rendering.
- Building placement ghost was NOT covered by prior docs. New territory.

---

## 1. Overview

Three loosely-related visual feedback systems for player commands:

1. **Building placement ghost** — when a building is armed for placement,
   one PLACE.SHP diamond is drawn **per cell** of the foundation, with a
   frame index chosen per-cell: valid/invalid/special. Plus wall-placement
   variants drop a silhouette of the wall segment.

2. **Rally lines** — factory-to-rally-cell lines drawn when an eligible local
   factory/repair/cloning building is selected and has a rally target at
   `TechnoClass+0x218`. Corrected renderer: `FUN_006DA9D0`, not
   `Tactical::DrawLine3D`.

3. **Planning path markers and lines** — planning/queued waypoint path overlay
   drawn by `FUN_006DAD60`. It draws all adjacent stored waypoint segments and
   uses `MOUSE.SHA` action-frame marker data. `FLAGFLY.SHP` is a separate
   `UnitClass::DrawExtras` block, not the verified tactical planning marker.

---

## 2. Building placement ghost

### 2.1 Per-cell draw — `BuildingPlacement_per_cell_draw` @ `0x0047EC90`

The core primitive. Called once per foundation cell by the outer overlay
renderer. `param_1` is a **CellClass**.

**Key cell fields** (byte offsets from CellClass `this`):

| Offset | Type | Purpose |
|---|---|---|
| `+0x11B` | byte | Affects Y offset of the drawn diamond (`* -15`) |
| `+0x11C` | byte | Frame-index modifier for invalid cells (+2 or +3) |
| `+0x12C` (300) | u32 | Placement-render flag bits |
| `+0xE4` | ptr | Cell-content linked list head |
| `+0x24/+0x26` | short/short | Cell X/Y coordinates |

**Flag bits at `+0x12C`:**
- `& 0x02` — "render building-ghost cell"
- `& 0x04` — "render wall-ghost cell"
- Cells without either bit set return early (no ghost).

**Frame-index selection for PLACE.SHP:**

```
if cell.flags & 0x04:             # wall-ghost cell
    frame = 1
elif local_1d == 1:               # passed all validity checks
    frame = 0                      # green/valid
else:                              # building-ghost but failed validity
    frame = cell[+0x11C] + 2       # 2 or 3 depending on byte value
                                    # (two "invalid" variants)
```

**`local_1d = 1` (cell is valid)** requires:
- Cell is NOT shrouded (`IsShrouded() == 0`)
- AND `Cell_passability_building_placement()` returns non-zero (foundation
  buildable on this cell)
- AND various UIModeLock conditions satisfy:
  - If `g_UIModeLock[0x3A2] != 0` (a mode flag), require an object of
    `WhatAmI() == 2` (building) or `0x24` in the cell content list, and a
    valid `BuildingClass::CanAcceptUpgrade` result
  - Else standard cell passability + `DAT_00880968` OK

**Draw call:**

```
CC_Draw_Shape(
    g_PLACE_SHP,
    frame,                                 # 0/1/2/3 from above
    &screen_xy,                            # cell's pixel position
    clip_rect,
    (-(param_4 != 0) & 0xFFFF0000) | 0x20600,
    0,                                     # color table
    y_offset,                              # −2 or −12 depending on +0x11C
    0, 1000, 0, 0, 0, 0, 0)
```

**Draw flags decoded:**
- `0x20600` base = **alpha-blend** + **no Z-test** + **primary surface**
- High bits `0xFFFF0000` added when `param_4 != 0` — additional translucency
  or color-key setting

### 2.2 PLACE.SHP frame catalog (inferred from code)

| Frame | Condition in code | Semantic |
|---|---|---|
| 0 | `local_1d == 1` (cell is valid) | **GREEN** — valid placement cell |
| 1 | `cell.flags & 0x04` (wall cell) | Wall ghost (may be yellow for "wall preview") |
| 2 | `cell[+0x11C] == 0` and not valid | **RED** — primary invalid |
| 3 | `cell[+0x11C] == 1` and not valid | **Secondary invalid** — possibly "too close to base edge" |

The two invalid variants (frames 2 vs 3) are parity-relevant: the original
game has two different "red" presentations for different invalidity reasons.
Our current Rust implementation (`src/render/selection_overlay.rs`
`build_building_preview`) uses ONE invalid color.

### 2.3 Outer renderer — `BuildingPlacement_OverlayRenderer` @ `0x006D5030`

Drives the per-cell loop. Two phases:

**Phase A — primary armed building** (reads `g_UIModeLock`):
1. Gate: mouse must be inside viewport rect.
2. Get foundation size (via `FUN_004A94F0` reading `DAT_00880964`).
3. Nested loop over foundation cells `(dx, dy) in foundation_rect`:
   - Compute cell at `(base_cell + (dx, dy))`
   - Convert to screen pixels via `CoordsToClient`
   - Call `BuildingPlacement_per_cell_draw` to draw PLACE.SHP
4. If building is a wall:
   - `FirestormWall_PlacementShadow` for Firestorm walls (TS legacy; likely
     unreachable in YR)
   - `LaserFencePost_PlacementShadow` for laser-fence posts
   - `OverlayWall_PlacementShadow` for regular walls (default case)
5. Special-case for building type 7 (`iVar6 == 7`) with `[+0x16B7]` set:
   foundation treated as 3×3 with offsets `0xFFFFFFFF` — a sentinel for
   "repair bay" or similar auto-sized building.

**Phase B — secondary queued placement** (reads `DAT_0088097C`):
Same cell loop but in `DAT_0088096C..0x00880970` coordinates. Present only
when a second building is queued/chained. Only runs outside map editor.

### 2.4 Wall placement shadows

Three specialized variants for wall types:
- **`OverlayWall_PlacementShadow` @ `0x006D5C50`** — regular walls
  (`Eligibility: wall` buildings). Draws a single overlay cell silhouette.
- **`LaserFencePost_PlacementShadow` @ `0x006D5730`** — laser fence posts
  (gate cell `[+0x16BE] != 0`).
- **`FirestormWall_PlacementShadow` @ `0x006D59D0`** — Firestorm walls
  (gate cell `[+0x16C0] != 0`). **TS LEGACY** — Firestorm defense is a TS
  system; flag defaults off in YR content. Still reachable if a mod enables.

Wall-cell branches chosen by BuildingTypeClass flags at HouseClass-accessed
offsets `+0x16BE` (laser-fence), `+0x16C0` (Firestorm), fallback = overlay.

### 2.5 Comparison — Rust vs original

Our [`src/render/selection_overlay.rs`](src/render/selection_overlay.rs)
around lines 597-637 implements `build_ghost_sprite` using tint multipliers:

| State | Rust tint (RGB) | Original |
|---|---|---|
| Valid | `[0.5, 1.0, 0.5]` | PLACE.SHP **frame 0** (palette-driven green diamond) |
| Invalid | `[1.0, 0.5, 0.5]` | PLACE.SHP **frame 2** (plus conditional **frame 3**) |
| Wall cell | not distinguished | PLACE.SHP **frame 1** |
| Alpha | linear multiplier | Draw-flag `0x20600` alpha-blend with `0xFFFF0000`-shifted variant |

**Parity gap:** Rust tints the whole building sprite uniformly. Original
draws a per-cell diamond **ON TOP of the cell** (not tinting the building
itself) to indicate per-cell validity. A 3×3 building with one blocked
corner shows 8 green diamonds + 1 red diamond, not a uniformly-red
building. This is the per-cell-validity visual that gives players
fine-grained feedback about which specific cell is blocking the build.

The actual **building sprite** during placement is drawn elsewhere (likely
via a normal `BuildingClass::Draw` call with a "pending" flag that enables
translucency). This report did not trace that path.

---

## 3. Rally lines (factory → rally cell)

### 3.1 Corrected renderer: `FUN_006DA9D0`

**Correction 2026-05-21:** selected factory rally lines are not drawn by
`Tactical::DrawLine3D @ 0x006DBB60`. The live renderer is `FUN_006DA9D0`,
called twice from `TacticalClass_Draw` at `0x006D4648` and `0x006D46CF`.

The line draws only when all of these are true:
- object is a building (`RTTI == 6`)
- building is selected (`Object+0x83`)
- owner is the local player (`Object+0x21C == g_PlayerPtr`)
- `BuildingClass` vtable `+0x284` returns true
- rally target pointer at `TechnoClass+0x218` is non-null

The vtable `+0x284` eligibility check passes for factory-like buildings:
`Factory=UnitType`, `Factory=InfantryType`, `Cloning=yes`, or `UnitRepair=yes`.
Standard YR barracks, war factories, cloning vats, and repair depots are covered.

### 3.2 Coordinates, color, and style

The source coordinate comes from the selected building vtable `+0x48`. The target
comes from `TechnoClass+0x218`, projected through the target vtable `+0x48`; the
Z is replaced with `CellClass::GetGroundHeight`, with bridge-height adjustment
when the target cell has the bridge bit.

Color is not an INI key. The renderer submits one default/background packed-color
line, then two owner-house RGB line submits using `HouseClass+0x56F9..+0x56FB`.
Pulse phase is `(0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`, with pattern
`DAT_00842930`.

### 3.3 Relationship to `DrawLine3D`

`Tactical::DrawLine3D @ 0x006DBB60` still exists as a Tactical vtable line
primitive, but this report's older claim that it is the selected factory rally
renderer is superseded. The 2026-05-21 rally-line report found only a data/vtable
xref for `0x006DBB60` in this slice and verified direct use of `FUN_006DA9D0`.

---

## 4. Planning/queued waypoint path overlay

### 4.1 Corrected renderer: `FUN_006DAD60`

**Correction 2026-05-21:** tactical planning/queued waypoint path lines are not
drawn by `Tactical::DrawLine3D @ 0x006DBB60`, and their tactical markers are not
`FLAGFLY.SHP`. The active renderer is `FUN_006DAD60`, called twice from
`TacticalClass_Draw` at `0x006D463F` and `0x006D46C6`.

The path is stored in `WaypointPathClass`, not `NavQueue`:
- house/player path array: `House+0x210 + index*4`
- current path index: `House+0x20C`
- point array: `WaypointPathClass+0x2C`
- count: `WaypointPathClass+0x38`
- loop index: `WaypointPathClass+0x24`

The renderer draws all adjacent stored waypoint segments. `FUN_00763BA0` returns
the next point and can wrap the final point back to the loop index when a loop is
configured.

### 4.2 Tactical marker asset

`FUN_006DAD60` loads `MOUSE.SHA` (`0x0082604C`) and uses mouse/action table index
`0x3C` with start frame `0x180`, count `1` (`0x0082D6B8`). That is the verified
tactical planning-path marker source.

`FLAGFLY.SHP` at `0x008458F8` is still present, but it belongs to a separate
`UnitClass::DrawExtras` block gated by `Unit+0x6CC != -1`. Do not treat it as the
planning/queued waypoint path marker until that separate block is intentionally
implemented.

### 4.3 INI keys

`[General] MaxWaypointPathLength=15` is stored at `RulesClass+0x90` and bounds
path growth through `0x005090F0`.

`[AudioVisual] WaypointAnimationSpeed=10` is read into `RulesClass+0x50`, but the
2026-05-21 planning-path slice did not verify a direct use in `FUN_006DAD60`.

---

## 5. Constants summary (parity targets)

| Constant | Value | Source | Purpose |
|---|---|---|---|
| PLACE.SHP name | `"PLACE.SHP"` | @ `0x00820080` | placement ghost diamond |
| PLACE.SHP frames used | 0, 1, 2, 3 | `per_cell_draw` | valid, wall, invalid-A, invalid-B |
| Rally renderer | `FUN_006DA9D0` | 2026-05-21 report | selected factory rally line |
| Rally target field | `TechnoClass+0x218` | 2026-05-21 report | selected building rally target |
| Planning path renderer | `FUN_006DAD60` | 2026-05-21 report | planning/queued waypoint path overlay |
| Planning marker asset | `MOUSE.SHA` action index `0x3C` | `FUN_006DAD60` | tactical planning marker |
| MaxWaypointPathLength default | **15** | `[General]` | planning path growth limit |
| FLAGFLY.SHP name | `"FLAGFLY.SHP"` | @ `0x008458F8` | separate UnitClass extra block |
| FLAGFLY.SHP frames | **14** | literal in `UnitClass::DrawExtras` | separate animation cycle |
| WaypointAnimationSpeed default | **10** | `[AudioVisual]` | read by rules; direct planning-render use not verified |
| DrawLine3D vtable slot | `Tactical+0x60` | vtable @ `0x007F43A8` | generic Tactical line primitive, not verified rally/planning path |
| Placement cell valid bit | `& 0x02` at cell+0x12C | `per_cell_draw` | building foundation cell |
| Placement wall bit | `& 0x04` at cell+0x12C | `per_cell_draw` | wall segment cell |
| Draw-flag pattern | `0x20600` + `0xFFFF0000` toggle | `CC_Draw_Shape` arg | alpha + no-Z |

---

## 6. Rust implementation status

| Behavior | Rust state |
|---|---|
| Per-cell placement diamond (PLACE.SHP valid/invalid) | **IMPLEMENTED** — `src/render/selection_overlay.rs:507-552` `build_building_preview` uses `place.shp` diamonds per-cell |
| Per-cell validity check | **IMPLEMENTED** — `src/sim/production/production_placement.rs:22-72` `placement_preview_for_owner` → `cell_valid: Vec<bool>` |
| Three invalid variants (frames 2 + 3) | **NOT IMPLEMENTED** — only one invalid state |
| Wall ghost (frame 1) | **NOT IMPLEMENTED** — no wall placement shadow |
| Building sprite during placement | **IMPLEMENTED** — green/red tint on whole sprite (not per-cell) |
| Rally line rendering | **NOT IMPLEMENTED** — `HouseState.rally_point` stored but no `FUN_006DA9D0`-style selected-building line drawn |
| Rally line pulse animation | **NOT IMPLEMENTED** — needs `g_CurrentFrameCounter` phase and `DAT_00842930`-style pattern |
| Planning path lines/markers | **NOT IMPLEMENTED** — needs `WaypointPathClass`-style path overlay, all adjacent segments, and `MOUSE.SHA` marker |
| Separate FLAGFLY.SHP unit extra | **NOT IMPLEMENTED/UNMAPPED** — do not substitute for tactical planning marker |

---

## 7. Parity implications (what the player will feel)

Ranked by visible impact:

1. **Per-cell invalid variants (frame 2 vs 3) — low-medium.** Both frames
   are "red-ish" but differ by a byte flag on the cell. Fine-grained player
   information about WHY a placement is blocked. Current Rust shows just
   "invalid" uniformly — this is a minor drift.

2. **Rally line pulse and owner-color layering — medium.** Players visually
   check the factory's rally line when issuing queue commands. The verified
   path uses `FUN_006DA9D0`, owner RGB, and a frame-counter phase.

3. **Planning path segment topology — medium.** Planning mode draws every
   adjacent stored waypoint segment, and may close a loop. Rendering only the
   final endpoint would lose the visible route shape.

4. **Planning marker identity — low-medium.** Tactical planning markers use
   `MOUSE.SHA`; substituting `FLAGFLY.SHP` gives the wrong marker even if a
   separate flag block exists elsewhere.

5. **Wall placement ghost (PLACE.SHP frame 1) — medium for wall spam.**
   Players placing walls rely on seeing the wall-segment preview cell-by-
   cell. Missing frame-1 variant makes wall placement feel less precise.

---

## 8. Open questions

- **PLACE.SHP frame-2-vs-3 semantics.** The byte at `cell+0x11C` toggles
  between frame 2 and frame 3 for invalid cells. Our code didn't trace what
  SETS this byte. Likely candidates:
  - Frame 2 = "cell physically blocked" (unit/building/terrain in the way)
  - Frame 3 = "cell outside build radius" (too far from base's ConYard/MCV
    proximity requirement) — i.e., **yellow** rather than **red**
  - Actual labels unverified; check a standard YR game to observe.

- **Exact `FUN_006DA9D0` raster style.** Renderer identity, gates, endpoints,
  owner RGB, and phase are verified. The final low-level surface primitive
  shape should be carried from the 2026-05-21 rally report when implemented.

- **Separate `FLAGFLY.SHP` semantics.** It is no longer the tactical planning
  marker. Its `UnitClass::DrawExtras` block is separate and should be mapped
  only if implementing that specific unit-extra visual.

- **Placement-armed building sprite tint.** The per-cell PLACE.SHP is
  clear, but the BUILDING SPRITE itself during placement — how is it drawn
  translucently? Not traced. Likely a flag on `BuildingClass::Draw` that
  enables alpha-blend mode.

---

## Sources

**Ghidra addresses decompiled:**
- `0x006D5030 BuildingPlacement_OverlayRenderer`
- `0x0047EC90 BuildingPlacement_per_cell_draw`
- `0x006DA9D0` selected factory rally-line renderer
- `0x006DAD60` planning/queued waypoint path renderer
- `0x006DBB60 Tactical::DrawLine3D` (generic primitive; old rally/planning attribution superseded)
- `0x00443860 BuildingClass::SetRallyPoint`
- `0x004FBF60 HouseClass::Set_Rally_Point_Cell`
- `0x0073D3C0..E0 UnitClass::DrawExtras` (`FLAGFLY.SHP` separate block, partial)
- `0x006692C8..D8 RulesClass::ReadAudioVisual` (WaypointAnimationSpeed)

**Byte-pattern / memory inspections:**
- `0x00820080` = `"PLACE.SHP"`
- `0x0082604C` = `"MOUSE.SHA"`
- `0x008458F8` = `"FLAGFLY.SHP"` (separate UnitClass extra)
- `0x0083AB88` = `"WaypointAnimationSpeed"`
- `0x007F43A8` = `Tactical::DrawLine3D` vtable entry

**INI keys verified:**
- `[General] MaxWaypointPathLength = 15`
- `[AudioVisual] WaypointAnimationSpeed = 10` (stored at rules+0x50 in the 2026-05-21 planning-path report; direct planning-render use not verified)
- `PlacementDelay = 0.05` (INI scan; not traced in this report — relates to
  AI placement retry, not player ghost rendering)
- `MaximumBuildingPlacementFailures = 3` (AI retry limit, not rendering)

**Prior reports extended:**
- `C:/Users/enok/Documents/ra2-rust-game-docs/TARGET_LINES_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`

**Rust files referenced:**
- [`src/render/selection_overlay.rs:507-552`](src/render/selection_overlay.rs#L507-L552)
- [`src/render/selection_overlay.rs:597-637`](src/render/selection_overlay.rs#L597-L637)
- [`src/sim/production/production_placement.rs:22-72`](src/sim/production/production_placement.rs#L22-L72)
- [`src/sim/house_state.rs:30`](src/sim/house_state.rs#L30)
- [`src/app_input.rs:247`](src/app_input.rs#L247)
