# Naval System Implementation Plan

Actionable implementation steps for ships, naval yards, and water mechanics.
Derived from verified findings in `NAVAL_SYSTEM_RESEARCH.md`.

---

## Dependency Graph

```
Phase 1: Passability bug fix
    │
    ▼
Phase 2: INI flag parsing (WaterBound, Naval)
    │
    ▼
Phase 3: Water-aware building placement ──► Phase 4: Naval yard spawn cell
                                                │
                                                ▼
                                           Phase 5: Wake animation
                                                │
                                           ┌────┴────┐
                                           ▼         ▼
                                    Phase 5b:   Phase 5c:
                                    Submarine   Ship Sinking
                                    System
                                           └────┬────┘
                                                ▼
                                           Phase 6: Ship visuals (polish)
```

---

## Phase 1 — Fix Float/Ship Passability Conflation

**Problem:** `SpeedType::Float` is shared by both hover units and ships.
It maps to zone 9 (passable everywhere except rock), which is correct for
hover but wrong for ships. The zone flood-fill and terrain cost grid both
allow ships onto land.

**Root cause in the original engine:** gamemd.exe uses **MovementZone** for
cell passability (Can_Enter_Cell) and **SpeedType** only for speed multipliers
on cells already known to be passable. The Rust engine conflates them.

### Changes

**File: `sim/zone_build.rs` — `is_passable()` (lines 144–188)**

The passability check at lines 163–169 does:
```rust
let speed_type = cat.representative_speed_type();
let land = passability::tmp_terrain_to_land_type(cell.land_type);
return passability::is_passable_for_speed_type(land.as_index(), speed_type);
```

This routes through `representative_speed_type()` which maps `Water → Float → zone 9`.
Replace with a direct ZoneCategory → zone layer mapping that uses MovementZone-based
passability instead:

| ZoneCategory | Should use zone layer | Via MovementZone | Reason |
|---|---|---|---|
| Land | 1 (Track) | Normal | Ground vehicles |
| Water | 10 (Water) | Water | Ships — water only |
| WaterBeach | 11 (WaterBeach) | WaterBeach | Water + beach |
| Amphibious | 4 (Amphibious) | Amphibious | Land + water |
| Infantry | 0 (Foot) | Foot | Ground foot |
| Fly | 9 (Winged) | Winged | Everything |

Add `ZoneCategory::zone_layer() -> usize` that returns these directly,
and use it in `is_passable()` instead of going through
`representative_speed_type() → zone_layer_for_speed_type()`.

**Note:** `passability::is_passable_for_zone()` already exists at line 187–193
but is never called by zone_build.rs. The fix is to use it (or the zone_layer
method) in place of `is_passable_for_speed_type()`.

**File: `sim/zone_incremental.rs` — lines 87–88, 132, 163**

The incremental zone updater mirrors zone_build but passes `None` for
`resolved_terrain`, falling back to TerrainCostGrid. It also uses
`cat.representative_speed_type()` to select the cost grid at line 87–88.
Must apply the same fix: use ZoneCategory → zone layer directly, not via
SpeedType. The cost grid selection can remain SpeedType-based (Float for
ships is fine for *speed* costs — the issue is *passability*, not cost).

**File: `sim/terrain_cost.rs` — `classify_terrain_cost()` (lines 172–236)**

The terrain cost grid is built per-SpeedType. The Float grid gives
`COST_NORMAL` to land cells. This is correct for hover but wrong for ships.

**Approach (Option B — matches gamemd.exe):** Keep the Float cost grid as-is.
Ships will use the Float cost grid for speed multipliers, but the **pathfinder**
adds a MovementZone-based passability gate. This matches the original engine
where Can_Enter_Cell (MovementZone) is separate from speed table (SpeedType).

**File: `sim/pathfinding.rs` — `find_path_with_costs_inner()` (lines 906–1034)**

The neighbor expansion loop at line 974 checks `grid.is_walkable(nx, ny)`.
After this check, add a MovementZone-based passability check:

```
if unit has MovementZone::Water or MovementZone::WaterBeach:
    look up cell's LandType
    check passability::is_passable_for_zone(land_type, movement_zone)
    skip cell if not passable
```

This requires threading `MovementZone` (or an `Option<MovementZone>`) into
the pathfinding call. The call sites are in `sim/world_orders.rs` and
`sim/locomotor.rs` — they already have access to the entity's MovementZone
via `ObjectType`.

Also apply to `find_path_with_costs_corridor_inner()` (lines 775–903).

**File: `sim/passability.rs`**

`is_passable_for_zone(land_type: u8, mz: MovementZone) -> bool` already
exists at lines 187–193. No changes needed — just start using it.

### Verification

- Unit test: `is_passable_for_zone(LandType::Clear, MovementZone::Water)` = false
- Unit test: `is_passable_for_zone(LandType::Water, MovementZone::Water)` = true
- Zone flood-fill for Water category should only include water cells
- Ship pathfinding should refuse land cells even though Float cost grid says COST_NORMAL

---

## Phase 2 — Parse WaterBound + Naval Flags

**File: `rules/object_type.rs`**

Add two fields to `ObjectType`:

```rust
pub water_bound: bool,  // WaterBound=yes in INI
pub naval: bool,        // Naval=yes in INI
```

Parsing logic (verified against Ghidra at `0x45FF94`):
- `WaterBound` defaults to `true` if SpeedType is already Float, else `false`
- `Naval` defaults to `false`

These are simple bool reads from INI. No complex logic.

### Which units/buildings have these flags

From verified rulesmd.ini:
- `Naval=yes`: DEST, AEGIS, DLPH, CARRIER, SUB, HYD, DRED, BSUB, SQD,
  LCRF, SAPC, GAYARD, NAYARD, YAYARD (and their Yuri equivalents)
- `WaterBound=yes`: GAYARD, NAYARD, YAYARD (naval yard buildings only)

### Usage

- `water_bound` → Phase 3 (building placement on water)
- `naval` → future AI targeting, factory classification, UI filtering

---

## Phase 3 — Water-Aware Building Placement

**Reference:** `NAVAL_SYSTEM_RESEARCH.md` section 11, decompiled `IsCellSuitableForBuilding`
at `0x47C620`.

**File: `sim/production_placement.rs` — `cell_placeable()` (lines 321–336)**

Current signature:
```rust
fn cell_placeable(
    sim: &Simulation, entities: &EntityStore, rules: &RuleSet,
    path_grid: Option<&PathGrid>, cx: u16, cy: u16,
    ref_height: u8, height_map: &HashMap<(u16, u16), u8>,
) -> bool
```

Add `water_bound: bool` parameter AND `resolved_terrain: &ResolvedTerrainGrid`.

**How to query water status:** The `ResolvedTerrainCell` struct
(`map/resolved_terrain.rs` line 72) has an `is_water: bool` field.
Access via `resolved_terrain.cell(cx, cy).is_water`.

The check becomes:

```
if water_bound:
    require resolved_terrain.cell(cx, cy).is_water == true
    skip walkability check (water cells are not walkable in PathGrid)
    skip build_blocked check (water IS the valid terrain)
    still reject: occupied cells (structure_occupies_cell)
    still reject: height mismatch
else:
    require resolved_terrain.cell(cx, cy).is_water == false  (or current behavior)
    use existing walkable + build_blocked + overlap + height checks
```

**File: `map/resolved_terrain.rs` ~line 927**

No change needed. `build_blocked: is_water || is_cliff_like` stays correct
for normal buildings. The `cell_placeable()` override handles WaterBound.

### Pass water_bound through the call chain

Trace callers of `cell_placeable()`: the placement scan functions in
`production_placement.rs` that iterate foundation cells. They receive
a building type — read `building_type.water_bound` and pass it down.
Also pass resolved_terrain (which the placement system already has access
to via `Simulation` or `Map`).

### Additional placement rules for naval yards

From the research doc:
- `Adjacent=12` — naval yards need large placement radius (already parsed)
- Bridge cells should be rejected — check `cell.bridge_body` or similar
  resolved terrain field
- Ramp cells should be rejected — check cell metadata

---

## Phase 4 — Naval Yard Spawn Cell

When a naval yard finishes producing a ship, the ship must spawn on a **water
cell within the building foundation**, not on a random walkable cell.

**Pattern to follow:** `sim/production_refinery.rs` — `maybe_spawn_refinery_harvester()`

### Logic

1. Ships Unlimbo at the building center (not an exit cell) — naval yards have
   NO `ExitCoord`. They use `TargetCoordOffset` instead to bias the rally point.
2. The ship's locomotor takes over immediately after Unlimbo.
3. No `Force_Track` for ships — this is controlled by `TypeClass+0x14A` which
   is false for naval units (only ground vehicles use Force_Track at factory exit).
4. Search foundation cells for a valid water cell (passable for MovementZone::Water)
   if the center cell is blocked.
5. If no valid cell found, queue the spawn for retry next tick.

### NumberImpassableRows

Parse `NumberImpassableRows=3` from INI into building type data. Actual offset
in gamemd.exe is `BuildingTypeClass+0x1620` (verified via Ghidra at `0x45FF94`).
All three stock naval yards use 3.

**NEEDS RESEARCH:** The direction and exact semantics of NumberImpassableRows
are not yet verified via Ghidra. The research doc claims it counts along
`MapCoords.X` but this was not confirmed by decompilation. Before implementing,
decompile the function that reads this field to determine:
- Which axis it counts along (X or Y in iso coords)
- Whether it marks cells as ground-impassable or prevents building overlap
- How it interacts with the foundation footprint

---

## Phase 5 — Wake Animation

**Reference:** `NAVAL_SYSTEM_RESEARCH.md` section 5, Process at `0x69FC10`.

### Spec from gamemd.exe

Every game tick, if `frame_counter & 7 == 0` (every 8th frame):
1. Unit is alive
2. Unit is not a deployed vessel (`type+0xD69 == 0`)
3. Byte at `techno+0x8C` is zero (exact meaning TBD — likely not deployed/docked)
4. Cell LandType == 2 (Water) — in Rust: `LandType::Water`
5. `Wake=WAKE1` is defined in rules `[General]` (`RulesClass+0x94 != 0`)

Then spawn a wake SHP animation at the unit's world position.

### Implementation

**File: `rules/ruleset.rs` or general rules**

Parse `Wake=WAKE1` from `[General]` section. Store the animation type name.

**File: `sim/locomotor.rs` or a new `sim/ship_effects.rs`**

In the ship's per-tick update, add the wake check. This is a thin addition
to the existing ground movement tick, not a separate system.

**Spawning:** Use the existing `WorldEffect` system. The infrastructure is
already in place:
- `sim/components.rs` — `WorldEffect` struct with SHP name, position, frame tracking
- `sim/world.rs` — `sim.world_effects: Vec<WorldEffect>`, append to spawn
- `app_instances/overlays.rs` — `build_world_effect_instances()` handles rendering

Just push a new `WorldEffect` with `shp_name: "WAKE1"` at the unit's cell
coordinates. No new animation infrastructure needed.

### Drive vs Ship wake timing

Drive locomotor spawns wake every **10 frames** (IDIV by 10), Ship locomotor
spawns every **8 frames** (AND 0x80000007). This is an intentional difference
in the original engine — Ship wakes are slightly more frequent.

---

## Phase 5b — Submarine System

Submarines use the **CloakState** system (`TechnoClass+0x220`), NOT a
separate submersion system. The cloak state enum doubles as submersion
state for naval units.

### Cloak mechanics

- **CloakingStages=9** (from `[General]`), **CloakingSpeed=1** for all subs
  (DLPH, SUB, BSUB all set `Cloakable=yes`, `CloakingSpeed=1`).
- Transition time: `CloakingStages / CloakingSpeed = 9 frames` to
  fully submerge or surface.
- State machine: `0=Surfaced → 1=Diving → 2=Submerged → 3=Surfacing`

### Visual rendering (5 stages)

The 9-frame cloak transition maps to 5 visual stages:

1. **Normal** — fully visible, standard palette
2. **Indistinct 25%** — slight transparency/shimmer
3. **Darken 50%** — half darkened, water distortion begins
4. **Shadowy 75%** — mostly transparent, outline visible
5. **Ripple → Hidden** — only water ripple effect, then fully invisible

### Sensor detection

- Per-cell sensor counter at `CellClass+0x7C` (`SensorsOfHouses[24]`) —
  one counter per player house.
- Submerged units are only visible to players with sensor coverage on
  that cell (SubterraneanSensor, PsychicSensor, etc.).

### Combat interaction

- **Firing uncloaks immediately** — same frame as weapon discharge, no delay.
- `CloakStop=no` means the sub stays submerged when stationary (default
  for all stock subs).
- `CloakDelay=0.02` — minimum 0.02 minutes (~1.2 seconds at normal speed)
  before re-submerging after firing.

---

## Phase 5c — Ship Sinking

Ship sinking is a multi-phase death sequence, not a simple removal.

### Trigger

`IsSinking` at `TechnoClass+0x3CD` (byte flag), triggered by the death
state machine at **case 4** — after the explosion sequence completes.

### 5-phase death sequence

1. **Cases 0-3: Explosion** (~70 frames total) — fire/explosion anims play,
   ship remains at water level, still collidable.
2. **Case 4: Sink** — `IsSinking` flag set, sinking behavior begins.

### Sinking mechanics

- **Tilt:** Increments `AngleRotatedForwards` (`+0x32C`) by **0.01 rad/frame**.
  Maximum tilt is `PI/4` (~0.785 rad). Reaches max tilt in ~79 frames.
- **Tilt direction:** Based on facing octant at death:
  - Octants **0, 6, 7** → tilt left (negative angle)
  - Octants **1, 2, 3, 4, 5** → tilt right (positive angle)
- **Visual clipping:** Screen-space Y clipping at `WaterlineY` (`+0x3CA`).
  The ship's rendered image is clipped at the waterline as it tilts — the
  Z coordinate never changes. This creates the illusion of sinking below
  the water surface.
- **SinkingSound:** `GenLargeWaterDie` (from `[General]` `SinkingSound=`)
  plays **once** at the `IsSinking` transition.
- **SplashList:** `H2O_EXP3, H2O_EXP2, H2O_EXP1` — 3 random splash
  animations spawned around the ship during sinking.
- **No wreck/debris:** Ship entity is removed after sinking completes.
  No ground object, no corpse, no debris left behind.

---

## Phase 6 — Ship Visuals (Polish)

Lower priority. Implement after gameplay works.

### Ship body rocking

- Modulate `pitch` field periodically (sinusoidal) when on water
- Ghidra: `techno+0x328` (pitch), period TBD from decompilation
- Purely visual, affects voxel Draw_Matrix transform

### Submarine depth rendering

- Units with `Underwater=yes` need opacity/palette changes when submerged
- `Visual_Character` vtable method returns different codes for surfaced vs submerged
- `Is_Surfacing` (vtable 38, `0x4B4C80`) indicates transition state
- This is a larger feature touching rendering + game state

---

## Research Gaps — Study Before Implementing

These items need Ghidra decompilation before implementation:

### 1. NumberImpassableRows semantics (blocks Phase 4)

The field is parsed at `BuildingTypeClass+0x1620` but we don't know how it's
**consumed**. Need to find xrefs to offset `+0x1620` and decompile the
function(s) that read it. Questions:
- Does it mark cells as ground-impassable in the cell occupancy system?
- Which axis does it count along in isometric coordinates?
- Does it apply during placement, during gameplay, or both?
- How does it interact with the foundation footprint?

### 2. Ship spawn location at naval yard (blocks Phase 4)

The claim that ships Unlimbo at building center needs verification. Decompile
the naval yard's production completion handler — the function that transitions
a unit from "in production queue" to "placed on map." Check:
- Where exactly does the ship entity get placed? (center cell? dock cell?)
- Does it search for a free water cell, or always use a fixed offset?
- What happens if the spawn cell is blocked by another ship?
- Is `TypeClass+0x14A` actually the Force_Track gate? Verify with xrefs.

### 3. Ship body rocking period (blocks Phase 6)

The research doc mentions pitch/yaw oscillation but doesn't have the period
or amplitude. Decompile the function that updates `techno+0x328` / `+0x32C`
for ships on water. Need:
- Oscillation frequency (frames per cycle)
- Amplitude range
- Whether it's driven by locomotor Process or by a separate visual tick

### 4. Wake condition byte at techno+0x8C (clarification for Phase 5)

The research doc said "mission != 0x23" but verified decompilation shows it
checks `(char)(techno[0x23]) == '\0'`, which reads byte at offset 0x8C
(0x23 * 4). Determine what this field actually is. Likely candidates:
- OnBridge flag
- InAir flag
- Some deployment/docking state
Knowing this ensures the wake check is accurate.

### 5. Drive vs Ship Process wrapper differences (informational)

The research doc says Ship Process at `0x69FC10` and Drive Process at
`0x4B0500` are "different wrappers, same heavy subroutines." The shared
subroutines (`FUN_006a1c80`, `FUN_006a05f0`) are confirmed. But we should
understand what **Ship's wrapper does differently** beyond wake animation:
- Water-specific height handling?
- Different docking behavior?
- Any submarine-specific checks in the Ship Process?

---

## Key Constants from Ghidra (Quick Reference)

| Constant | Value | Source |
|---|---|---|
| Ship locomotor CLSID | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` | INI |
| Wake interval | every 8th frame (`& 7`) | `0x69FC10` |
| Wake INI key | `Wake=WAKE1` under `[General]` | rulesmd.ini:525 |
| Water LandType (gamemd) | 2 | name table at `0x81DA28` |
| Water LandType (Rust) | 4 (`LandType::Water`) | passability.rs:44 |
| WaterBound SpeedType trick | SpeedType=5 (Float) | `0x45FF94` |
| NumberImpassableRows offset | BuildingTypeClass+0x1620 | `0x45FF94` (verified) |
| NumberImpassableRows (naval yards) | 3 | rulesmd.ini |
| Adjacent (naval yards) | 12 | rulesmd.ini |
| Ship passability zone | 10 (water only) | passability matrix |
| WaterBeach passability zone | 11 (water + beach) | passability matrix |
| Cell bridge flag | `cell_flags & 0x100` | Ghidra |
| Cell ramp flag | `cell_flags & 0x400` | Ghidra |
| CloakState offset | TechnoClass+0x220 | enum: 0=Surfaced, 1=Diving, 2=Submerged, 3=Surfacing |
| CloakingStages | 9 | from [General] |
| CloakingSpeed (subs) | 1 | DLPH, SUB, BSUB all set to 1 |
| IsSinking offset | TechnoClass+0x3CD | byte flag |
| ShipSinkingWeight | 3.0 | rules+0x630, Weight >= 3.0 sinks |
| Sinking tilt rate | 0.01 rad/frame | ~79 frames to 45 degrees |
| Rocking epsilon | 0.005 | Draw_Matrix threshold for rocking path |

## Codebase Integration Points (Quick Reference)

| Component | File | Lines | Notes |
|---|---|---|---|
| A* neighbor expansion | `sim/pathfinding.rs` | 962–1034 | Add MovementZone gate here |
| A* corridor variant | `sim/pathfinding.rs` | 775–903 | Same fix needed |
| Terrain cost build | `sim/terrain_cost.rs` | 103–150 | Float grid stays as-is |
| Terrain cost classify | `sim/terrain_cost.rs` | 172–236 | No change needed (Option B) |
| Zone passability check | `sim/zone_build.rs` | 144–188 | Use zone_layer() not SpeedType |
| Zone incremental | `sim/zone_incremental.rs` | 87, 132, 163 | Same fix |
| Existing zone passability fn | `sim/passability.rs` | 187–193 | `is_passable_for_zone()` — use it |
| Building placement | `sim/production_placement.rs` | 321–336 | Add water_bound param |
| Water field on cell | `map/resolved_terrain.rs` | 72 | `is_water: bool` |
| World effects (wake) | `sim/components.rs` | ~400 | `WorldEffect` struct |
| World effects storage | `sim/world.rs` | — | `sim.world_effects: Vec<WorldEffect>` |
| World effects render | `app_instances/overlays.rs` | — | `build_world_effect_instances()` |

## Units Quick Reference

### True Ship Locomotor (CLSID `{2BEA74E1-...}`)

DEST, AEGIS, DLPH, CARRIER, SUB, HYD, DRED, BSUB, SQD

### Amphibious Hover (NOT ships, CLSID `{4A582742-...}`)

LCRF, SAPC — use Hover locomotor, SpeedType=Hover, MovementZone=Amphibious

### Naval Yards

GAYARD, NAYARD, YAYARD — all WaterBound=yes, Naval=yes, Adjacent=12,
NumberImpassableRows=3, NumberOfDocks=1
