# Shroud System — Implementation Report

Every feature gamemd.exe's shroud system has, mapped to what we've built, what's missing,
and exactly how to implement each missing piece.

---

## Feature Inventory

### FEATURE 1: Per-Cell Visibility State

**What gamemd does:**
- Two bits per cell in `CellClass+0x12C`: bit 3 (explored), bit 4 (fully revealed)
- Both set together by `CellClass__RevealShroudFlags`, never cleared (except Gap Gen)
- Per-house: `cell+0x7C` has `short[18]` sight provider ref counts (one per house)

**Our status: DONE**
- `OwnerVisibility` grid: `FLAG_REVEALED` (0x01) + `FLAG_VISIBLE` (0x02) per cell per owner
- In-place recompute preserves REVEALED across ticks
- Alliance-aware merged grid for O(1) lookups

**Difference:** gamemd uses per-house ref counts (multiple units stacking sight). We
recompute from scratch each tick (clear VISIBLE, re-reveal from all units). Functionally
equivalent — the ref count approach is an optimization for incremental updates, but our
full recompute is correct and fast enough.

---

### FEATURE 2: Vision Radius Reveal

**What gamemd does:**
- `MapClass::RevealShroud` (0x005673a0) iterates a pre-built spiral table (0x00ABD490)
- Cumulative ring sizes at 0x007ED3D0: sight 0→1, 1→9, 2→21, ... 9→253 cells
- Hard cap at sight 10 (crashes above)
- Euclidean distance check via fast sqrt LUT (0x008650BC)
- Each passing cell: `cell+0x12C |= 0x18`

**Our status: DONE**
- `reveal_radius_into()` uses the exact 253-entry spiral table
- `REVEAL_RING_SIZES` matches gamemd's cumulative table
- Sight capped at `MAX_SIGHT_RANGE = 10`
- Sight 10 outer ring uses integer distance² check (no sqrt needed)

**File:** [vision/mod.rs:514-570](src/sim/vision/mod.rs#L514-L570)

---

### FEATURE 3: Height-Based Line of Sight (RevealByHeight)

**What gamemd does:**
- Default ON (`rules+0x17EE`, `RevealByHeight=true` in [General])
- For each spiral cell, looks up a "midpoint" cell from mirror table (0x00ABCF60)
- Comparison: `if (viewer_level + 3 < midpoint_cell.Level) → blocked`
- Terrain 4+ levels above the viewer at the halfway point blocks sight
- `viewer_level = unit_z / leptons_per_height_level` (computed at init from trig)

**Our status: NOT IMPLEMENTED**

**How to implement:**
1. Extract the mirror/midpoint table from gamemd (0x00ABCF60, ~253 entries of (dx,dy) pairs,
   same format as the reveal spiral). Each entry `i` corresponds to spiral entry `i` and
   gives the cell offset of the midpoint along the line of sight.
2. Add a `height_map: Option<&[u8]>` parameter to `reveal_radius_into()` — a flat grid of
   cell height levels (from `CellClass+0x11B`, the terrain Level field 0-15).
3. In the spiral loop, after the bounds check and before marking visible:
   ```rust
   if reveal_by_height {
       let (mdx, mdy) = REVEAL_MIRROR[i];
       let mx = cx + mdx as i32;
       let my = cy + mdy as i32;
       if mx >= 0 && mx < w && my >= 0 && my < h {
           let midpoint_level = height_map[my * w + mx];
           if viewer_level + 3 < midpoint_level as i32 {
               continue; // LOS blocked by terrain
           }
       }
   }
   ```
4. `viewer_level` = entity's Z position converted to terrain level units. gamemd uses
   `z / DAT_00abde88` where DAT_00abde88 ≈ 256 leptons per level. We already have the
   entity's `position.z` — divide by the equivalent height step.

**Where:** [vision/mod.rs:514-570](src/sim/vision/mod.rs#L514-L570) — add to `reveal_radius_into()`
**Data needed:** Mirror table (extract from Ghidra), cell height grid (from map terrain)

---

### FEATURE 4: Veteran + Elevation Sight Bonuses

**What gamemd does:**
- `TechnoClass::UpdateReveal` (0x0070af50) computes effective sight:
  - Base sight from type's `Sight=` value
  - Veteran bonus: if veterancy >= 1.0, add `rules+0x680` (VeteranSight multiplier)
  - Elevation bonus: `(unit_z * 256) / LeptonsPerSightIncrease`
- Result clamped to 10

**Our status: DONE**
- `recompute_owner_visibility_in_place()` applies both bonuses
- VeteranSight from VisionConfig, elevation from entity.position.z
- Clamped to MAX_SIGHT_RANGE

**File:** [vision/mod.rs:460-475](src/sim/vision/mod.rs#L460-L475)

---

### FEATURE 5: Shroud Edge Rendering (SHROUD.SHP + ABuffer)

**What gamemd does:**
- `Shroud_EdgeBitmask_Calculator` (0x006d8700): checks 8 neighbors, builds 8-bit mask
- 256-byte LUT at 0x007f4194 maps mask → SHROUD.SHP frame index (0-46, or 0xFF/0xFE)
- `ShroudEdge_BlitToABuffer` (0x0047efe0): writes SHP brightness pixels directly to ABuffer
- ABuffer is 16-bit per-pixel, initialized to 0x7F (neutral)
- All tile/sprite blitters read ABuffer per-pixel to modulate color brightness

**Our status: DONE**
- `shroud_edge_mask_8bit()` matches exact bit layout
- `SHROUD_EDGE_LUT` is the exact 256-byte table from gamemd
- `ShroudBuffer` blits SHROUD.SHP brightness pixels into R8 CPU buffer
- GPU multiply pass replicates per-pixel darkening
- Change detection via fog generation counter + camera position

**Files:**
- [vision/mod.rs:311-346](src/sim/vision/mod.rs#L311-L346) — 8-bit mask
- [shroud_buffer.rs:35-84](src/render/shroud_buffer.rs#L35-L84) — LUT
- [shroud_buffer.rs:187-292](src/render/shroud_buffer.rs#L187-L292) — rebuild + blit
- [shroud_multiply.wgsl](src/render/shroud_multiply.wgsl) — GPU shader

---

### FEATURE 6: Object Discovery / Concealment

**What gamemd does:**
- `ObjectClass+0x81` (`IsUndiscovered`): 1 = hidden from rendering entirely
- When cell is revealed: `CellChangeNotify` → `TechnoClass::Discover` (vtable 0x198)
  → sets `+0x81 = 0`, submits to display layer
- When gap generator covers: `ObjectClass::Conceal` (vtable 0xD4)
  → removes from display layer, sets `+0x81 = 1`
- Once discovered (`TechnoClass+0x41A = HasBeenDiscovered`), set permanently per house
- Standard YR (no fog): discovered objects stay visible forever

**Our status: PARTIALLY DONE**
- Entity visibility filtering exists in app layer:
  - [app_instances/helpers.rs](src/app_instances/helpers.rs) — `is_entity_visible_for_local_owner()`
  - [app_entity_pick.rs](src/app_entity_pick.rs) — `HiddenEnemy` target classification
  - [app_selection_brackets.rs](src/app_selection_brackets.rs) — health bar hiding
  - [app_ui_overlays.rs](src/app_ui_overlays.rs) — UI overlay hiding
- Combat targeting respects fog: [combat_targeting.rs](src/sim/combat/combat_targeting.rs)

**What's missing:**
- No persistent `HasBeenDiscovered` flag per entity per house. Currently we check
  `is_cell_revealed()` every frame. In standard YR without fog this is equivalent
  (revealed = permanent), so functionally correct.
- No `CellChangeNotify` event when shroud state changes — we don't need it because
  our render filtering happens at draw time, not via a display layer submit model.
- Gap Generator concealment should set entities as unselectable/untargetable in the
  gap area for enemies. Need to verify our gap suppression covers this.

**Verdict:** Functionally complete for standard YR. The architectural difference
(render-time filtering vs display-layer events) produces the same result.

---

### FEATURE 7: Gap Generator

**What gamemd does:**
The gap generator is a **separate visual subsystem** from shroud. It does NOT modify
`cell+0x12C` (the explored/revealed bits) and does NOT interact with the shroud edge
rendering system (Shroud_EdgeBitmask_Calculator).

The gap generator operates through:
1. **Cell counter** (`cell+0x130`): reference count of gap generators covering this cell
2. **GAP overlay flag** (`cell+0x140 |= 0x20`): bookkeeping for gap state
3. **AlphaShapeClass overlay**: 21 animation objects per gap gen (building+0x55C..+0x5B0)
   with translucency values 0-15 for gradual expand/contract visual effect
4. **Shroud map surface** (DAT_0089ddc0): dedicated drawing surface using Bresenham
   circle rasterizer (FUN_007bb920) for the circular dark area
5. **4-state machine**: inactive(0) → expanding(1) → active(2) → contracting(3)
   with 15-step counter at building+0x6ED

The visual dark circle is alpha-blended onto the ABuffer via a lookup table at
DAT_0088a118. It is NOT rendered through SHROUD.SHP edges.

The INI entry `[GAGAP]` has `GapGenerator=yes` but NOT `CloakGenerator=yes`.
These are separate INI flags.

**Our status: GAMEPLAY DONE, VISUAL NOT DONE**
- `apply_gap_generators()` clears `FLAG_VISIBLE` for enemies — correct for gameplay
  (targeting, entity visibility suppression)
- Applied after SpySat so gap wins in overlap
- Euclidean distance check within gap radius

**What's missing (separate implementation scope, not shroud):**
- AlphaShapeClass visual overlay (the dark circle rendering)
- Expand/contract animation (15-step gradual transition)
- Shroud map surface for circular pattern rendering
- Per-cell reference counting for overlapping gap generators
  (functionally equivalent since we recompute each tick)

**File:** [vision/mod.rs:656-700](src/sim/vision/mod.rs#L656-L700)

---

### FEATURE 8: SpySat Full-Map Reveal

**What gamemd does:**
- Building with `SpySat=yes` and `Radar=yes` in rules.ini
- When powered: reveals entire map for owner (all cells explored + visible)
- When destroyed: `fog.reset_explored_for_owner()` — owner's entire explored map resets
- Vision re-reveals from remaining units on next tick

**Our status: DONE**
- `apply_spy_sat()` fills all cells with REVEALED | VISIBLE
- Called after normal vision recompute, before gap generators
- Power state check via `is_building_powered()`
- Destruction triggers `reset_explored_for_owner()`

**Files:**
- [vision/mod.rs:637-654](src/sim/vision/mod.rs#L637-L654) — apply_spy_sat
- [world/mod.rs:555-577](src/sim/world/mod.rs#L555-L577) — refresh_fog integration
- [world/mod.rs:880](src/sim/world/mod.rs#L880) — destruction handler

---

### FEATURE 9: Shroud Game Setting (Shroud=yes/no)

**What gamemd does:**
- `[MultiplayerDialogSettings] Shroud=yes` (default) — map starts fully shrouded
- `Shroud=no` — all cells start pre-revealed, no shroud rendering

**Our status: PARTIALLY DONE**
- `GameOptions.shroud: bool` exists in [game_options.rs](src/sim/game_options.rs)
- Default: `true`
- **NOT WIRED UP:** The shroud buffer always renders. No code checks `game_options.shroud`
  to skip shroud or pre-reveal all cells.

**How to implement:**
In `refresh_fog()` or at game start: if `game_options.shroud == false`, mark all cells
as revealed for all owners. Or simpler: in `rebuild_if_needed()`, skip the cell iteration
and leave the buffer at NEUTRAL (0x7F = bright) when shroud is disabled.

```rust
// In refresh_fog() or game init:
if !game_options.shroud {
    for vis in fog.by_owner.values_mut() {
        for cell in &mut vis.cells {
            *cell |= FLAG_REVEALED | FLAG_VISIBLE;
        }
    }
}
```

**File:** [game_options.rs:38](src/sim/game_options.rs#L38), [world/mod.rs:536](src/sim/world/mod.rs#L536)

---

### FEATURE 10: Minimap Shroud Rendering

**What gamemd does:**
- Minimap shows shroud as dark areas
- Unexplored cells are black on radar
- Explored cells show terrain colors
- Gap-covered cells appear dark to enemies
- Radar only available when player has powered `Radar=yes` building

**Our status: MOSTLY DONE**
- [minimap.rs](src/render/minimap.rs) imports FogState
- `update_unit_dots()` receives visibility info, fills unrevealed cells with `COLOR_SHROUD`
- Only stamps revealed cell pixels
- Hides enemy entities in unrevealed cells (except `RadarVisible=yes` units)
- Radar availability gated on [radar.rs](src/sim/radar.rs) `has_radar_for_owner()`

**What's missing:**
- Gap generator coverage not reflected on minimap (cells still show as explored)
- Verify minimap updates when fog generation changes

---

### FEATURE 11: Shroud Affects Pathfinding / Movement

**What gamemd does:**
- Pathfinding works through shroud (units CAN path to unexplored cells)
- `MoveToShroud=no` on specific unit types prevents them from moving into shroud
- `AllowShroudedSubteranneanMoves=true` lets subterranean units path through shroud

**Our status: NOT IMPLEMENTED**
- Pathfinding currently ignores shroud entirely (units can always move anywhere)
- No `MoveToShroud` check on movement commands

**How to implement:**
1. Parse `MoveToShroud=` from rules.ini per unit type (default: yes)
2. In pathfinding cost function: if `MoveToShroud=no` and target cell is not revealed
   for the unit's owner, treat as impassable
3. Parse `AllowShroudedSubteranneanMoves=` from [General] (default: true)
4. Subterranean movement mode bypasses the MoveToShroud check when enabled

**Priority:** Low — most units have MoveToShroud=yes. Only specific units (harvesters?)
are restricted.

---

### FEATURE 12: RevealOnFire (Weapons Reveal Shroud)

**What gamemd does:**
- Weapons with `RevealOnFire=yes` reveal shroud around the firing position
- Uses `RevealTriggerRadius` from [General] (default 9, max 10)

**Our status: PARTIALLY DONE**
- `reveal_radius()` public API exists for external reveal events
- Combat system has plumbing to call it

**What's missing:**
- Verify `RevealOnFire` INI key is parsed per weapon type
- Verify combat code actually calls `reveal_radius()` when a RevealOnFire weapon fires
- Verify `RevealTriggerRadius` is read from rules.ini

**File:** [vision/mod.rs:623-635](src/sim/vision/mod.rs#L623-L635)

---

### FEATURE 13: Building Placement Reveal

**What gamemd does:**
- When a building is placed, `FUN_0043f180` (enter-cell handler) calls
  `FUN_005678e0` with sight=1 for individual foundation cells
- This is a one-time reveal for the placement footprint area
- Normal ongoing reveal happens through vtable+0x488/0x48C (same as all technos)
- Buildings reveal from center point, NOT per foundation cell

**Our status: DONE (implicitly)**
- Buildings are entities with a position and sight range
- `recompute_owner_visibility_in_place()` reveals from each building's position
- Per-tick recompute means placement reveal happens on the next tick automatically
- Center-point reveal matches gamemd behavior

**No action needed** — our per-tick full recompute handles this correctly.

---

### FEATURE 14: Paranoid Reveal Pass

**What gamemd does:**
- `MapClass::ParanoidRevealAll` (0x004adee0) periodically iterates ALL TechnoClass objects
- Calls `ReReveal` (vtable+0x48C) for player-controlled and allied units
- Ensures no reveal gaps exist after edge cases (teleporting, airdrop, etc.)

**Our status: DONE (implicitly)**
- Our per-tick full recompute IS the paranoid pass — every tick iterates all entities
  and reveals from their positions. No gaps possible.

---

### FEATURE 15: Alliance Vision Sharing (AllyReveal)

**What gamemd does:**
- `AllyReveal=true` in [General] (default)
- Allied players share vision — each player sees what their allies see
- Checked in `MapClass::RevealShroud`: if revealer is allied AND AllyReveal=true,
  treat as local player reveal

**Our status: DONE**
- `build_merged_for()` ORs all allied owners' visibility grids together
- `is_cell_visible()` and `is_cell_revealed()` use merged grid
- Alliance check via `are_houses_friendly()`

**File:** [vision/mod.rs:191-208](src/sim/vision/mod.rs#L191-L208)

---

### FEATURE 16: Transport Passenger Vision

**What gamemd does:**
- Units inside a transport do NOT provide vision
- Only the transport itself reveals based on its own Sight range

**Our status: DONE**
- `recompute_owner_visibility_in_place()` skips entities where
  `entity.passenger_role.is_inside_transport()` is true

**File:** [vision/mod.rs:450-451](src/sim/vision/mod.rs#L450-L451)

---

### FEATURE 17: BlendedFog Rendering Mode

**What gamemd does:**
- `BlendedFog=true` in [AudioVisual] (default) — smooth gradient edges
- `BlendedFog=false` — checkerboard dithering pattern for edges
- Affects how SHROUD.SHP / FOG.SHP edge pixels are composited

**Our status: NOT IMPLEMENTED**
- Always renders smooth edges (BlendedFog=true equivalent)
- No dithered fallback

**Priority:** Very low — BlendedFog=true is the default and the better-looking mode.
Nobody plays with it off.

---

### FEATURE 18: Fog of War (TS Legacy)

**What gamemd does:**
- `FogOfWar=true` in [MultiplayerDialogSettings] — OFF by default in YR
- Gated behind `SpecialFlags & 0x1000`
- When ON: previously-explored cells that are no longer in sight become "fogged"
  (dimmed, showing last-known terrain/buildings, hiding enemy movement)
- Uses FOG.SHP instead of SHROUD.SHP for edge rendering
- Per-cell fog counters at `cell+0xAC` (short[18])
- FoggedObject/AlphaShapeClass renders ghosted copies of last-seen objects

**Our status: NOT IMPLEMENTED (correctly — it's dormant in YR)**

**Priority:** None for standard YR. Only implement if we add an explicit fog-of-war
game mode. The infrastructure is noted in SHROUD_FOG_RENDERING_PIPELINE.md if needed.

---

### FEATURE 19: Sensor Arrays and Psychic Sensors

**What gamemd does:**
- `SensorsSight=` per unit — extended detection range separate from Sight
- `SensorArray=yes` buildings detect cloaked/subterranean units within range
- `PsychicDetectionRadius=` — reveals mind-controlled/psi units
- Sensor data stored at `cell+0x78` (per-house sensor bits)

**Our status: NOT IMPLEMENTED**

**How to implement:**
1. Parse `SensorsSight=` per unit type, `PsychicDetectionRadius=` from rules
2. Add a separate sensor visibility layer (doesn't affect shroud rendering, only
   reveals cloaked/subterranean units on the minimap and for targeting)
3. This is orthogonal to shroud — it's a detection system, not a vision system

**Priority:** Medium — needed for proper cloaked unit gameplay.

---

### FEATURE 20: Crate/Reveal Superweapon

**What gamemd does:**
- Crate pickup `Darkness=` effect: re-shrouds the entire map for target player
- Spy satellite uplink superweapon: reveals entire map
- Trigger actions can reveal areas (`RevealTriggerRadius`)

**Our status: NOT IMPLEMENTED**

**How to implement:**
- Darkness effect: call `fog.reset_explored_for_owner(target)` then `refresh_fog()`
- Spy satellite SW: same as SpySat building — mark all cells revealed
- Trigger reveal: call `reveal_radius()` with `RevealTriggerRadius` at the trigger waypoint

---

## Integration Map

Where each shroud feature connects in the codebase:

```
SIMULATION TICK (src/sim/world/mod.rs)
  └─ refresh_fog()
       ├─ recompute_owner_visibility_in_place()    [vision/mod.rs]
       │    ├─ clear VISIBLE flags
       │    ├─ iterate entities → reveal_radius_into()
       │    │    ├─ spiral table iteration
       │    │    ├─ bounds + distance check
       │    │    └─ (TODO) height-based LOS check
       │    └─ veteran + elevation bonuses
       ├─ apply_spy_sat()                          [vision/mod.rs]
       └─ apply_gap_generators()                   [vision/mod.rs]

APP LAYER (src/app_sim_tick.rs)
  └─ after tick: fog.build_merged_for(owner) → generation++

RENDER FRAME (src/app_render/mod.rs)
  ├─ Phase 4b: shroud_buf.rebuild_if_needed()      [shroud_buffer.rs]
  │    ├─ fill buffer NEUTRAL (0x7F)
  │    ├─ iterate cells: dark diamond / edge frame / skip
  │    └─ upload R8 texture to GPU
  ├─ Phase 6: shroud_buf.draw() → multiply pass    [shroud_multiply.wgsl]
  └─ Entity filtering:
       ├─ app_instances: is_entity_visible_for_local_owner()
       ├─ app_entity_pick: HiddenEnemy classification
       ├─ app_selection_brackets: health bar hiding
       └─ app_ui_overlays: overlay hiding

MINIMAP (src/render/minimap.rs)
  └─ update_unit_dots(visibility: (&str, &FogState))
       ├─ fill unrevealed cells with COLOR_SHROUD
       └─ hide enemy entities in unrevealed cells

COMBAT (src/sim/combat/combat_targeting.rs)
  └─ acquire_best_target(): skip if !fog.is_cell_visible()
```

---

## Priority-Ordered TODO List

### P0 — Needed for correct gameplay
1. **Gap Generator rendering suppression** — verify gap-covered cells render as dark
   for enemies (check if gap clears REVEALED or just VISIBLE)
2. **Shroud=no game setting** — wire up `GameOptions.shroud` to disable shroud

### P1 — Needed for authentic behavior
3. **Height-based LOS** — extract mirror table, integrate into reveal_radius_into()
4. **MoveToShroud** — parse INI key, block pathfinding for restricted units
5. **RevealOnFire** — verify weapon firing triggers reveal_radius()

### P2 — Polish
6. **Minimap gap coverage** — show gap-covered cells as dark on minimap
7. **Sensor arrays** — separate detection layer for cloaked units
8. **Darkness crate effect** — re-shroud on crate pickup
9. **Trigger reveal** — map trigger actions that reveal areas

### P3 — Low priority
10. **BlendedFog=false** — dithered edge rendering mode
11. **Fog of War** — TS legacy mode (only if explicitly adding this game mode)
12. **SubterraneanMoves** — AllowShroudedSubteranneanMoves setting
