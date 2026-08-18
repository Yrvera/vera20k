//! Cell entry classification — unified Can_Enter_Cell result codes.
//!
//! The original RA2 engine returns 8 distinct codes when a unit
//! tries to enter a cell. Each code triggers a different movement response.
//! This module centralizes the classification logic that was previously
//! scattered as inline boolean checks in movement.rs.
//!
//! Two-phase design for borrow checker compatibility:
//! - Phase 1 (`check_terrain`): terrain + occupancy presence, no EntityStore needed
//! - Phase 2 (`classify_occupied_cell`): blocker friendship/crush, needs &EntityStore
//!
//! Bridge legality is now driven by A*'s `path_layers` (set per-step by `astar_search`
//! with the Ground→Bridge gates verified against the reference predicate), which
//! approximates the post-switch output of the original two-pass `Can_Enter_Cell`. See
//! docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md §"Known Parity Boundary".
//!
//! ## The native shape, and what is not modelled
//!
//! `UnitClass::Can_Enter_Cell` @ `0x0073F0A0` and `InfantryClass::Can_Enter_Cell`
//! @ `0x0051BF90` are the `FootClass` `+0x1AC` slot (`0x007F5E1C` and
//! `0x007EB204`). Both are **accumulators**: a running code later occupants may
//! only raise, punctuated by hard `return 7` / `return 0` exits. `AStar_main_loop`
//! @ `0x00429A90` expands a neighbour iff the code is below 7 and
//! `AStar_compute_edge_cost` @ `0x00429830` indexes it into the float table at
//! `0x0081870C` — `[1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0]`, whose only
//! reader is `0x00429848`. So codes 3, 4, 5 and 6 all still expand, at 1×, 60×,
//! 20× and 8× the base step.
//!
//! Pre-flight, in order, before any code accumulates: the bridge-deck select from
//! `Cell->Flags & 0x100`; the occupier/occupation snapshot; **Unit only** the
//! `MovementRestrictedTo=` gate; the direction-8 tube endpoint test; the
//! tube-direction consistency tests at this cell and at `(dir-4)&7`; **Infantry
//! only** an unconditional admit when `level - Cell->Level > 4`; the `+0x1B0`
//! slot (`CheckBridgeTraversal` @ `0x004D9C60`, FootClass-level — it is the same
//! entry in both vtables); the deck swap; the playfield gate; and **Unit only**
//! `FootClass::LocomotorPassabilityCheck` @ `0x004D9C10`, whose result **seeds**
//! the running code (Infantry seeds a literal 0).
//!
//! VERA reproduces the accumulate-worst-code shape and the crush latch. What it
//! does not, recorded rather than guessed and ordered by ordinary-skirmish
//! impact:
//!
//! - **`Gate=` buildings take the garrison arm.** [`classify_blocker`] maps a
//!   closed or opening `BuildingGateRuntime` to `ScatterRequired` (code 3), but
//!   code 3 in gamemd is `BuildingTypeClass+0x16B7` plus
//!   `!BuildingClass::CanGarrison()` — the garrison flag, read at `0x004525F9`.
//!   `Gate=` is a different field, `+0x16C0` (read by
//!   `BuildingClass::TogglePowerOrGate` @ `0x004471CB`), and its arm is its own
//!   branch: the gate occupant is either **skipped whole**, leaving the running
//!   code untouched so an open gate leaves the cell clear, or **`return 7`** when
//!   `occupant->Owner+0x1FA` is set. A gate never yields code 3. Trigger: any
//!   move order whose path crosses a friendly gate. Player effect: VERA's code 3
//!   routes into the scatter arm, which asks a *structure* to move out of the
//!   way; retail either walks through or treats it as solid. Frequency: common
//!   from mid-game on, in every walled base. Downstream risk: the test
//!   `friendly_closed_or_opening_gate_returns_code_3_not_code_6` pins the wrong
//!   mapping and must be re-baselined with the fix.
//! - **The wall arm produces the wrong code, not no code.** `cell_rect`'s
//!   `is_wall_overlay` / `WallBlocked` path is live and MovementZone-keyed, and
//!   its Destroyer-class escape set matches native's `{2, 3, 8, 0xC}` at
//!   `0x004835BB`. But it answers a **hard block** where native answers **4** for
//!   a friendly wall and **5** for an enemy one (Unit: `OverlayTypeClass+0x2A8`
//!   at `0x0073F420` with the `Crushable=` gate `+0x22D` at `0x0073F42E`;
//!   Infantry: `5 - isAlly`), and 4 and 5 both still expand in the A*. Retail
//!   therefore routes *through* a wall line at 60×/20× cost and stops at it;
//!   VERA reports no path. [`CellEntryResult::FriendlyWall`] consequently has no
//!   producer. Trigger: any expansion into a `Wall=yes` overlay cell. Player
//!   effect: a move order whose destination is enclosed by walls is refused
//!   outright instead of routing to the wall and stopping. Frequency: pre-placed
//!   civilian fences appear on most stock maps, so this fires many times a match
//!   even against players who never build walls. Downstream risk: codes 4 and 5
//!   feed the blocked-step Override arm, whose wall case targets a *cell* rather
//!   than an object and has no Restore path (see `movement_occupancy`); a
//!   producer must land together with that arm. **Do not add a second wall gate
//!   on top of the existing one.**
//! - **A crusher can never cross a crushable wall.** `cell_rect`'s
//!   `wall_allows_crusher` is a hardcoded `false` because the parsed overlay
//!   model does not carry `OverlayTypeClass+0x22D` = `Crushable=` (written by
//!   `ObjectTypeClass::ReadINI` @ `0x005F9426`, read by
//!   `TechnoClass::Is_Crushable_By` @ `0x005F6D49` and by
//!   `CellClass::RecalcZoneType` @ `0x00483CB5`). Stock `[GASAND]` is
//!   `Wall=yes` + `Crushable=yes` + `CrushSound=WallCrushSandbag`. Trigger: a
//!   `MovementZone=Crusher` or `AmphibiousCrusher` type (eleven stock entries)
//!   meeting a sandbag wall. Player effect: retail drives through with a crush
//!   sound; VERA stops. Frequency: sandbags are common map dressing and a common
//!   early-game player wall. Downstream risk: none beyond plumbing the flag.
//! - **The head-on deadlock exit** (`0x0073F9C0`-`0x0073FA30`, Unit only).
//!   Before conceding code 2 to a moving ally, native compares both objects'
//!   `FacingClass::Current` octants — the second offset by `+0x7FFF`, so the test
//!   is literally "facing each other" — and the `Math::atan2` of the lepton
//!   delta, and returns **7** when they are closing on the same octant within
//!   `Sqrt_Approx(...) < 0x200` leptons (`0x0073FA10 CMP EAX,0x1FF / JG`).
//!   Trigger: two friendly vehicles meeting head-on inside two cells. Player
//!   effect: retail makes one treat the cell as impassable and re-path; VERA has
//!   both wait and shuffle. Frequency: continuous in any traffic. Downstream
//!   risk: code 2 is also what arms the ten-step blocker-prediction loop in
//!   `AStar_compute_edge_cost`, so the wrong code feeds the wrong cost branch.
//!   `InfantryClass::Can_Enter_Cell` has no equivalent — its ally-and-moving arm
//!   goes straight to code 2.
//! - **The unarmed-mover hard block.** [`classify_blocker`] returns
//!   `OccupiedEnemy` for every non-friendly blocker; native checks armament
//!   first — Infantry `GetWeaponRange(this, -1) < 1 && What_Am_I != 0x24` →
//!   **7**, Unit `TypeClass+0xD28 == 0 && !HasWeaponAbility()` → 7 unless an
//!   owner or `IsTrain` escape holds. Trigger: an Engineer, Spy or other
//!   weaponless unit meeting an enemy on its path. Player effect: VERA pushes it
//!   into the code-5 blocked-step attack override instead of routing around a
//!   cell it can never clear. Frequency: a few times a match for any player who
//!   uses Engineers or Spies. Downstream risk: low; it is a predicate on the
//!   mover, not the cell.
//! - **The infantry sub-cell tail.** Native's tail is explicit: `code == 0 &&
//!   (OccupationFlags & 0x1C) == 0x1C` → **7**, and in the allied-occupier arm
//!   `counter != 3 ? 6 : 2`, where `counter` counts the non-moving allied
//!   infantry found during the walk. VERA's `check_terrain` returns
//!   `NeedsBlockerCheck` with no counter and no `0x1C` test. Trigger: a fourth
//!   infantryman ordered into a full friendly cell. Player effect: VERA yields 6
//!   (scatter) where retail yields 2 (wait) or 7. Frequency: constant in
//!   infantry-heavy play. Downstream risk: the counter has to be threaded
//!   through the walk, which is the one structural change on this list.
//! - **Code 1 is the wrong producer.** VERA emits `Crushable` (code 1) for a
//!   successful crush; gamemd returns **0** for a crush on the Unit latch path
//!   and 2 when a vehicle also occupies the cell. Code 1 comes from an unrelated
//!   `occupant+0x220 == 2` test in the non-allied branch, whose field identity is
//!   UNCHECKED. Trigger: any crush. Player effect: **none today** — movement
//!   groups `Clear | Crushable` at the same call site. Frequency: n/a while the
//!   cost table is unwired. Downstream risk: the moment `0x0081870C` is wired,
//!   code 1 costs 1000× where code 0 costs 1×, so every crushable cell becomes a
//!   near-wall to the search. The crush-latch comment on
//!   [`classify_occupied_cell_with_layers_and_ignored`] is correct for
//!   `UnitClass` and **false for Infantry**, which has no crush latch at all.
//! - **`MovementRestrictedTo=`** (`UnitTypeClass+0xDFC`, Unit only): when set,
//!   the cell's land type must equal it. LandType 10 (Tunnel) is exempt from the
//!   equality test but carries its own rule — `g_IsometricTileTypeClass_Array`
//!   entries with `(+0x2E4, +0x2E8)` of `(5,3)` or `(4,3)` are impassable unless
//!   `bIsoSubTileIndex == 2`, and `(3,4)` or `(3,5)` unless it is `6`. The
//!   overlay window `0xED..=0xEE` escapes the return-7 only when the mover's
//!   level does **not** match the cell's (`0x0073F1FD JNZ`). Trigger: stock
//!   `rulesmd.ini` sets the key on `[HYD]`, `[SQD]`, `[ASW]` and `[HORNET]`,
//!   always `=Water`. Player effect: none observed — the two naval types are
//!   already covered by the water-mover path and the two carrier aircraft use
//!   `AircraftClass`'s own predicate. Frequency: effectively zero for this
//!   owner. Downstream risk: none.
//! - **The tube gates** (`0x0073F211`-`0x0073F2C2`): direction 8 — the sentinel
//!   edge `AStar_main_loop` emits as its ninth neighbour — requires a tube at the
//!   cell and then **returns 0 immediately**, skipping the whole rest of the
//!   predicate; Unit tests `tube+0x28 == 0` while **Infantry tests
//!   `tube+0x28 == tube+0x24`**. Separately, any direction whose delta from the
//!   tube's own direction falls in `3..=5` is impassable, tested both at this
//!   cell and at the back-step cell `(dir-4)&7`. Trigger: pathing into or along a
//!   tube cell. Player effect: VERA admits tube-adjacent steps native refuses,
//!   and misses the direction-8 fast admit. Frequency: tube maps only.
//!   Downstream risk: none; it is a leaf predicate.
//! - **The AI-only overlay arm** (`0x0073F3EC`-`0x0073F41D`):
//!   `OverlayTypeClass+0x2AA != 0` and the mover's house not human-controlled
//!   and `g_GameMode == 0` → 7. Infantry has the same arm without the game-mode
//!   term. Trigger: an AI mover in campaign. Player effect: none in skirmish.
//!   Frequency: zero until an AI opponent exists. Downstream risk: none.
//! - **The end-of-list land-row test.** Native checks
//!   `LandTypeSpeedBuildabilityRows[Cell->LandType][speed] == 0.0` **after** the
//!   object walk and only on the ground list; VERA applies it in the terrain
//!   head. Trigger: a bridge-deck step over a zero-row ground cell — water under
//!   a bridge. Player effect: native can still return an object code there;
//!   VERA answers impassable from the head. Frequency: every bridge crossing
//!   over water. Downstream risk: the deck plane suppresses the test in native,
//!   which VERA's `is_elevated_bridge_cell` arm already approximates in
//!   `TerrainCostGrid`, so the observable outcome usually agrees.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/bump_crush, sim/entity_store, sim/locomotor,
//!   sim/pathfinding, map/entities, map/houses, rules/locomotor_type.

use std::collections::BTreeSet;

use super::PathGrid;
use super::terrain_cost::TerrainCostGrid;
use crate::map::entities::EntityCategory;
use crate::map::houses::{self, HouseAllianceMap};
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::sim::cell_rect::{
    IsClearToMoveResult, LiveCellPassabilityQuery, evaluate_live_cell_passability,
};
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellOccupationGrid, OccupancyGrid};

// ---------------------------------------------------------------------------
// Result enums
// ---------------------------------------------------------------------------

/// Result of checking whether a unit can enter a target cell.
///
/// Maps to the original engine's Can_Enter_Cell return codes (0–7). Each variant
/// carries enough context for the movement tick to dispatch the correct
/// response without re-querying the EntityStore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellEntryResult {
    /// Code 0: Cell is passable. Enter freely.
    Clear,
    /// Code 1: Cell contains crushable occupants. Crush and enter.
    Crushable { victims: Vec<u64> },
    /// Code 2: Blocked by a moving friendly unit. Wait, then repath.
    TemporaryBlock { blocker_id: u64 },
    /// Code 2: the independent Unit occupation bit is set, but the destination
    /// object list has no blocker identity. Wait/repath without scattering.
    TemporaryOccupation,
    /// Code 3: Allied building/scatter-required soft block.
    ScatterRequired { blocker_id: Option<u64> },
    /// Code 4: Friendly wall/overlay soft block.
    ///
    /// **No producer.** VERA does not classify wall overlays at cell entry; see
    /// the module header for the native arm and its residual.
    FriendlyWall,
    /// Code 5: Enemy unit occupying. Attack blocker while waiting.
    OccupiedEnemy { blocker_id: u64 },
    /// Code 6: Friendly stationary non-building occupant.
    FriendlyStationary { blocker_id: u64 },
    /// Code 7: Terrain impassable (water, building footprint, etc.). Abort.
    Impassable,
}

impl CellEntryResult {
    pub fn yr_code(&self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::Crushable { .. } => 1,
            Self::TemporaryBlock { .. } | Self::TemporaryOccupation => 2,
            Self::ScatterRequired { .. } => 3,
            Self::FriendlyWall => 4,
            Self::OccupiedEnemy { .. } => 5,
            Self::FriendlyStationary { .. } => 6,
            Self::Impassable => 7,
        }
    }
}

/// Phase 1 result — terrain and basic occupancy check (no EntityStore needed).
///
/// Computed inside the mutable entity borrow where we cannot also access
/// EntityStore for blocker lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainCheckResult {
    /// Cell is passable (terrain OK, occupancy clear or sub-cell available).
    Clear,
    /// Terrain impassable for this unit type.
    Impassable,
    /// Cell has occupants — needs Phase 2 EntityStore lookup to classify.
    NeedsBlockerCheck,
}

/// Terrain-only result for native-shaped cell-entry checks above `PathGrid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanEnterCellResult {
    Clear,
    HardBlocked,
}

/// Search-time interpretation of the YR `FootClass` cell predicate result.
///
/// This is deliberately not a terrain-speed percentage. `TerrainCostGrid` remains
/// responsible for SpeedType movement rates; this value is the small native
/// classification consumed by `NeighborStepCost` during A* expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchCellCostDecision {
    /// The raw value returned by the per-Foot predicate.
    pub raw_cost_class: u8,
    /// The class supplied to the neighbor-cost routine when expansion continues.
    pub effective_cost_class: Option<u8>,
    /// Whether this neighbor may be expanded.
    pub expands: bool,
    /// Whether the normal NeighborStepCost path is reachable.
    pub should_call_neighbor_step_cost: bool,
}

/// Apply the search-only cost-class gate used after YR's `FootClass` +0x1AC call.
///
/// Original: `AStar_main_loop` @ `0x00429A90`, immediately after the `+0x1AC`
/// call — `if (gate && class < 7) class = 0;` then reject `class >= 7`.
///
/// The gate is neither bridge nor coercion: it is `TechnoTypeClass+0xC94`, read
/// at `0x00429B64` and `0x00429C79`, which `TechnoTypeClass::ReadINI` binds at
/// `0x00712284` to the key string at `0x008444BC` = **`IsTrain`**. No stock
/// `rulesmd.ini` entry sets it, so this arm is a correctly-shaped model of a
/// mechanism nothing in stock YR enables — latent, not live.
pub fn search_cell_cost_decision(
    raw_cost_class: u8,
    coerce_to_zero_gate: bool,
) -> SearchCellCostDecision {
    let effective_cost_class = if coerce_to_zero_gate && raw_cost_class < 7 {
        0
    } else {
        raw_cost_class
    };
    let expands = effective_cost_class < 7;

    SearchCellCostDecision {
        raw_cost_class,
        effective_cost_class: expands.then_some(effective_cost_class),
        expands,
        should_call_neighbor_step_cost: expands,
    }
}

impl CanEnterCellResult {
    pub fn is_clear(self) -> bool {
        matches!(self, Self::Clear)
    }
}

/// Caller flavor for the terrain-entry slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainEntryMode {
    AStarNeighbor,
    RuntimeTransition,
    Smoothing,
    Scatter,
    SpawnLike,
}

/// Native-shaped known-input context for the terrain/layer portion of cell entry.
///
/// This deliberately stops before the unresolved search-only cost class and the
/// runtime-only blocker response. The original evaluates those with different
/// caller state; only the shared terrain/layer admission belongs here.
#[derive(Debug, Clone, Copy)]
pub struct CanEnterCellContext<'a> {
    pub target: (u16, u16),
    pub terrain_layer: MovementLayer,
    pub movement_zone: Option<MovementZone>,
    pub speed_type: Option<SpeedType>,
    pub path_grid: Option<&'a PathGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub terrain_costs: Option<&'a TerrainCostGrid>,
    pub bypass_grid: bool,
    pub mode: TerrainEntryMode,
    /// Selects the infantry view of terrain-object occupation. Retail terrain
    /// objects occupy sub-cells, and only the infantry entry gate reads that
    /// mask; vehicles stay blocked by the whole cell.
    pub is_infantry: bool,
}

/// Evaluate the shared terrain/layer slice of Can_Enter_Cell.
///
/// `PathGrid` is a coarse structural filter. Final terrain legality must also
/// consult the mover's SpeedType against the resolved target LandType/speed row
/// so a PathGrid-walkable water cell is still illegal for ordinary ground movers.
// Original: the `FootClass` `+0x1AC` slot — `UnitClass::Can_Enter_Cell` @
// `0x0073F0A0` (`0x007F5E1C`) and `InfantryClass::Can_Enter_Cell` @
// `0x0051BF90` (`0x007EB204`). There is no `EvaluateCellEnterabilityOrCost`
// symbol in this program; the earlier name here was invented.
pub fn evaluate_can_enter_cell(ctx: CanEnterCellContext<'_>) -> CanEnterCellResult {
    match ctx.terrain_layer {
        MovementLayer::Ground => evaluate_ground_cell_entry(ctx),
        MovementLayer::Bridge => {
            let bridge_walkable = ctx.path_grid.is_some_and(|grid| {
                grid.is_walkable_on_layer(ctx.target.0, ctx.target.1, MovementLayer::Bridge)
            });
            evaluate_shared_cell_leaf(ctx, bridge_walkable)
        }
        // Air and underground locomotors are admitted by their dedicated
        // locomotion state machines, not this ground/bridge terrain slice.
        MovementLayer::Air | MovementLayer::Underground => CanEnterCellResult::Clear,
    }
}

fn evaluate_ground_cell_entry(ctx: CanEnterCellContext<'_>) -> CanEnterCellResult {
    let (x, y) = ctx.target;

    if let Some(movement_zone) = ctx.movement_zone.filter(|zone| zone.is_water_mover()) {
        let land_passable = ctx
            .resolved_terrain
            .and_then(|terrain| terrain.cell(x, y))
            .is_some_and(|cell| is_water_surface_cell_passable(cell, movement_zone));
        return evaluate_shared_cell_leaf(ctx, land_passable);
    }

    let grid_ok = ctx.path_grid.map_or(true, |grid| {
        ctx.bypass_grid
            || if ctx.is_infantry {
                grid.is_walkable_for_infantry(x, y)
            } else {
                grid.is_walkable(x, y)
            }
    });
    let speed_type = ctx.speed_type.or_else(|| {
        if ctx.terrain_costs.is_some() {
            None
        } else {
            ctx.movement_zone.map(|zone| zone.speed_type())
        }
    });
    let speed_passable = speed_type.is_none_or(|speed_type| {
        ctx.resolved_terrain
            .and_then(|terrain| terrain.cell(x, y))
            .is_none_or(|cell| speed_type_allows_cell(cell, speed_type))
    });
    let terrain_cost_passable = terrain_cost_result(ctx.terrain_costs, x, y).is_clear();

    evaluate_shared_cell_leaf(ctx, grid_ok && speed_passable && terrain_cost_passable)
}

fn evaluate_shared_cell_leaf(
    ctx: CanEnterCellContext<'_>,
    land_passable: bool,
) -> CanEnterCellResult {
    let structural_bridge = ctx
        .path_grid
        .and_then(|grid| grid.cell(ctx.target.0, ctx.target.1))
        .is_some_and(|cell| cell.has_structural_bridge())
        || ctx
            .resolved_terrain
            .and_then(|terrain| terrain.cell(ctx.target.0, ctx.target.1))
            .is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
    let bridge_transition = ctx
        .path_grid
        .and_then(|grid| grid.cell(ctx.target.0, ctx.target.1))
        .is_some_and(|cell| cell.is_bridge_transition_cell())
        || ctx
            .resolved_terrain
            .and_then(|terrain| terrain.cell(ctx.target.0, ctx.target.1))
            .is_some_and(|cell| cell.is_bridge_transition_cell());
    if bridge_transition || (ctx.terrain_layer == MovementLayer::Bridge && !structural_bridge) {
        // Native `IsClearToMove` receives an integer level, not the engine's
        // path-layer enum. A bridgehead can carry Ground while an already-on-
        // bridge mover remains at deck height, so guessing base/base+4 here
        // rejects the proved Body->Ramp->Ground transition. Until +0x1AC
        // threads its numeric path height, retain the prior structural gate.
        return if land_passable {
            CanEnterCellResult::Clear
        } else {
            CanEnterCellResult::HardBlocked
        };
    }

    let Some(speed_type) = ctx
        .speed_type
        .or_else(|| ctx.movement_zone.map(|zone| zone.speed_type()))
    else {
        return if land_passable {
            CanEnterCellResult::Clear
        } else {
            CanEnterCellResult::HardBlocked
        };
    };
    let movement_zone = ctx.movement_zone.unwrap_or(MovementZone::Normal);
    let result = evaluate_live_cell_passability(LiveCellPassabilityQuery {
        target: ctx.target,
        speed_type,
        movement_zone,
        // FootClass +0x1AC owns zone calculation outside the shared Cell leaf.
        requested_zone: None,
        actual_zone: 0,
        requested_layer: Some(ctx.terrain_layer),
        ignore_infantry: false,
        ignore_vehicles: false,
        land_passable,
        path_grid: ctx.path_grid,
        resolved_terrain: ctx.resolved_terrain,
        // Object-list and raw occupation classification remain the later
        // class-specific +0x1AC arms and must not be collapsed into terrain.
        raw_occupation: None,
    });
    if matches!(
        result,
        IsClearToMoveResult::Clear { .. } | IsClearToMoveResult::ClearWinged
    ) {
        CanEnterCellResult::Clear
    } else {
        CanEnterCellResult::HardBlocked
    }
}

fn terrain_cost_result(
    terrain_costs: Option<&TerrainCostGrid>,
    x: u16,
    y: u16,
) -> CanEnterCellResult {
    if terrain_costs.is_some_and(|costs| costs.cost_at(x, y) == 0) {
        CanEnterCellResult::HardBlocked
    } else {
        CanEnterCellResult::Clear
    }
}

/// Ship movement must use the reduced ZoneType matrix rather than PathGrid's
/// ground walkability, with the confirmed coastal-water compatibility fallback.
pub(crate) fn is_water_surface_cell_passable(
    cell: &ResolvedTerrainCell,
    movement_zone: MovementZone,
) -> bool {
    let matrix_ok = super::passability::is_passable_for_zone(cell.zone_type, movement_zone);
    if matrix_ok || cell.is_water {
        return true;
    }
    movement_zone == MovementZone::WaterBeach && cell.zone_type == zone_class::BEACH
}

fn speed_type_allows_cell(cell: &ResolvedTerrainCell, speed_type: SpeedType) -> bool {
    cell.speed_costs
        .cost_for_speed_type(speed_type)
        .is_none_or(|cost| cost > 0)
}

/// Layer selections used by Can_Enter_Cell-style checks.
///
/// The common case uses one layer for all phases. Bridge traversal may select
/// the bridge object list while the post-traversal occupancy bits remain ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEnterLayerContext {
    pub terrain_layer: MovementLayer,
    pub object_list_layer: MovementLayer,
    pub occupancy_bits_layer: MovementLayer,
}

impl CanEnterLayerContext {
    pub fn single(layer: MovementLayer) -> Self {
        Self {
            terrain_layer: layer,
            object_list_layer: layer,
            occupancy_bits_layer: layer,
        }
    }
}

/// Read-only cell-entry oracle row preserving gamemd's split layer decisions.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CellEntryOracleRow {
    pub target: (u16, u16),
    pub terrain_layer: MovementLayer,
    pub object_list_layer: MovementLayer,
    pub occupancy_bits_layer: MovementLayer,
    pub terrain_result: String,
    pub yr_code: Option<u8>,
    pub occupancy_ground_present: bool,
    pub occupancy_bridge_present: bool,
}

impl CellEntryOracleRow {
    pub fn from_terrain_result(
        target: (u16, u16),
        layers: CanEnterLayerContext,
        result: TerrainCheckResult,
        occupancy: &OccupancyGrid,
    ) -> Self {
        let occ = occupancy.get(target.0, target.1);
        Self {
            target,
            terrain_layer: layers.terrain_layer,
            object_list_layer: layers.object_list_layer,
            occupancy_bits_layer: layers.occupancy_bits_layer,
            terrain_result: format!("{:?}", result),
            yr_code: match result {
                TerrainCheckResult::Clear => Some(CellEntryResult::Clear.yr_code()),
                TerrainCheckResult::Impassable => Some(CellEntryResult::Impassable.yr_code()),
                TerrainCheckResult::NeedsBlockerCheck => None,
            },
            occupancy_ground_present: occ.is_some_and(|o| !o.is_empty_on(MovementLayer::Ground)),
            occupancy_bridge_present: occ.is_some_and(|o| !o.is_empty_on(MovementLayer::Bridge)),
        }
    }
}

/// Opt-in diagnostic wrapper for Phase-1 cell entry checks.
pub fn check_terrain_with_layers_oracle(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> (TerrainCheckResult, CellEntryOracleRow) {
    let result = check_terrain_with_layers(
        target,
        layers,
        mover_category,
        path_grid,
        cost_grid,
        occupancy,
    );
    let row = CellEntryOracleRow::from_terrain_result(target, layers, result, occupancy);
    (result, row)
}

/// Vehicle-only building entry branch that may reach the live row helper.
///
/// InfantryClass::Can_Enter_Cell does not use the radio/contact or
/// UnitRepair/Bunker NumberImpassableRows branches, so callers must not use this
/// as a shared infantry rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleBuildingEntryBranch {
    /// Contact-vector branch. The caller must supply whether this mover has
    /// RadioClass contact with the checked building.
    RadioContact { mover_has_contact: bool },
    /// UnitRepair/Bunker branch. This branch is gated by the checked building's
    /// type flags, not by RadioClass contact.
    UnitRepairOrBunker,
}

/// Decision for a checked building occupant in UnitClass-style cell entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingOccupantEntryDecision {
    /// Keep the checked building in the ordinary blocker classification path.
    KeepBlocker,
    /// Skip this building occupant and continue scanning later occupants in the
    /// cell's object list.
    SkipBlocker,
}

/// Explicit live facts needed by the UnitClass building row-helper decision.
///
/// Caller responsibilities:
/// - `candidate_building_id` must be the result of a live
///   Look_up_building_in_cell-style lookup for the candidate cell.
/// - `checked_building_id` and type/runtime flags must describe the building
///   occupant currently being inspected.
/// - `mover_category` must be the mover's semantic category; only UnitClass-style
///   vehicle movers use these exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveVehicleBuildingEntry {
    pub mover_category: EntityCategory,
    pub branch: VehicleBuildingEntryBranch,
    pub checked_building_id: u64,
    pub candidate_building_id: Option<u64>,
    pub candidate_x: u16,
    pub building_origin_x: u16,
    pub number_impassable_rows: i32,
    pub is_unit_repair: bool,
    pub is_bunker: bool,
    pub bunker_occupied: bool,
}

/// Decide whether UnitClass-style movement should skip a building occupant.
///
/// This models `FUN_00458A00` at its two UnitClass::Can_Enter_Cell callsites:
/// radio/contact and UnitRepair/Bunker. A `KeepBlocker` result means the caller
/// should continue with the existing Can_Enter_Cell return-code classification;
/// `SkipBlocker` means only this building occupant is ignored.
pub fn decide_live_vehicle_building_entry(
    input: LiveVehicleBuildingEntry,
) -> BuildingOccupantEntryDecision {
    if input.mover_category != EntityCategory::Unit {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    let branch_active = match input.branch {
        VehicleBuildingEntryBranch::RadioContact { mover_has_contact } => mover_has_contact,
        VehicleBuildingEntryBranch::UnitRepairOrBunker => input.is_unit_repair || input.is_bunker,
    };
    if !branch_active {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    if input.candidate_building_id != Some(input.checked_building_id) {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }
    if input.number_impassable_rows == -1 {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }
    if input.is_bunker && input.bunker_occupied {
        return BuildingOccupantEntryDecision::KeepBlocker;
    }

    let first_clear_x = i32::from(input.building_origin_x) + input.number_impassable_rows;
    if i32::from(input.candidate_x) >= first_clear_x {
        BuildingOccupantEntryDecision::SkipBlocker
    } else {
        BuildingOccupantEntryDecision::KeepBlocker
    }
}

// ---------------------------------------------------------------------------
// Phase 1: terrain + occupancy presence
// ---------------------------------------------------------------------------

/// Check terrain walkability and basic occupancy for a target cell.
///
/// This is Phase 1 of the two-phase cell entry check. It does NOT access
/// EntityStore, so it can run inside a mutable entity borrow.
///
/// For infantry movers, also checks sub-cell availability.
pub fn check_terrain(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> TerrainCheckResult {
    check_terrain_with_layers(
        target,
        CanEnterLayerContext::single(target_layer),
        mover_category,
        path_grid,
        cost_grid,
        occupancy,
    )
}

/// Check terrain and occupancy using explicit CanEnter layer selections.
pub fn check_terrain_with_layers(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_category: EntityCategory,
    path_grid: Option<&PathGrid>,
    cost_grid: Option<&TerrainCostGrid>,
    occupancy: &OccupancyGrid,
) -> TerrainCheckResult {
    let (nx, ny) = target;

    // --- Terrain walkability ---
    let terrain_walkable = evaluate_can_enter_cell(CanEnterCellContext {
        target,
        terrain_layer: layers.terrain_layer,
        movement_zone: None,
        speed_type: None,
        path_grid,
        resolved_terrain: None,
        terrain_costs: cost_grid,
        bypass_grid: false,
        mode: TerrainEntryMode::RuntimeTransition,
        is_infantry: mover_category == EntityCategory::Infantry,
    })
    .is_clear();
    if !terrain_walkable {
        return TerrainCheckResult::Impassable;
    }

    // --- Occupancy ---
    let occ = occupancy.get(nx, ny);

    if mover_category == EntityCategory::Infantry {
        let selected_list_blocked =
            occ.is_some_and(|o| o.has_blockers_on(layers.object_list_layer));
        let sub =
            bump_crush::allocate_sub_cell_with_reserved(occ, layers.occupancy_bits_layer, None);
        if sub.is_some() && !selected_list_blocked {
            return TerrainCheckResult::Clear;
        }
        // No sub-cell available — needs blocker classification.
        return TerrainCheckResult::NeedsBlockerCheck;
    }

    // Vehicle/aircraft/structure: cell must be unoccupied on this layer.
    match occ {
        None => TerrainCheckResult::Clear,
        Some(o)
            if o.is_empty_on(layers.object_list_layer)
                && o.is_empty_on(layers.occupancy_bits_layer) =>
        {
            TerrainCheckResult::Clear
        }
        Some(_) => TerrainCheckResult::NeedsBlockerCheck,
    }
}

// ---------------------------------------------------------------------------
// Phase 2: blocker classification (needs EntityStore)
// ---------------------------------------------------------------------------

/// Classify an occupied cell's blockers to determine the Can_Enter_Cell code.
///
/// This is Phase 2 — runs outside the mutable entity borrow so it can read
/// blocker properties from EntityStore.
///
/// Check order, mirroring `UnitClass::Can_Enter_Cell` @ `0x0073F0A0`'s object
/// walk:
/// 1. Crush: crushability is a latch consulted after the walk, not an early exit
/// 2. Blocker friendship: enemy → OccupiedEnemy, friendly → moving/stationary
/// 3. JumpJet override: codes < 7 treated as Clear
///
/// The terrain and layer half of the native predicate runs before this, in
/// [`evaluate_can_enter_cell`]. The arms of the native walk this phase does not
/// produce — the wall/overlay codes and the head-on facing test — are recorded
/// in the module header rather than approximated here.
pub fn classify_occupied_cell(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    classify_occupied_cell_with_layers(
        target,
        CanEnterLayerContext::single(target_layer),
        mover_id,
        crush_capability,
        mover_owner,
        mover_locomotor,
        mover_bypass_grid,
        occupancy,
        entities,
        alliances,
        interner,
    )
}

/// Classify an occupied cell using explicit CanEnter layer selections.
#[allow(clippy::too_many_arguments)]
pub fn classify_occupied_cell_with_layers(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    classify_occupied_cell_with_layers_and_ignored(
        target,
        layers,
        mover_id,
        crush_capability,
        mover_owner,
        mover_locomotor,
        mover_bypass_grid,
        None,
        occupancy,
        entities,
        alliances,
        interner,
    )
}

/// Classify an occupied cell while ignoring a caller-supplied subset of live
/// object-list occupants. This is the runtime UnitClass path used by refinery
/// pads and repair/bunker rows where gamemd skips only the checked building
/// occupant, then continues scanning the same cell list.
#[allow(clippy::too_many_arguments)]
pub fn classify_occupied_cell_with_layers_and_ignored(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    ignored_blockers: Option<&BTreeSet<u64>>,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    let _ = mover_bypass_grid;
    // --- Crush candidates ---
    // Crushability is a latch, not an early exit: gamemd sets it while walking
    // the cell list and consults it only after the walk, when nothing else
    // raised the running code above 0. An occupant the mover can crush does not
    // contribute a code; one it cannot crush raises the code like any blocker.
    let victims = bump_crush::collect_crush_victims(
        target,
        occupancy,
        layers.object_list_layer,
        crush_capability,
        entities,
    );
    let crushable: BTreeSet<u64> = victims.iter().copied().collect();

    // --- Walk the WHOLE selected cell list, worst occupant wins ---
    // gamemd keeps a running code across every occupant of the selected list
    // (`if (code < N) code = N;`) and only the hard cases return early. Taking
    // the first occupant instead loses to whichever entity happens to be at the
    // list head, which is wrong in any mixed-occupancy cell.
    let mut worst = CellEntryResult::Clear;
    let mut saw_candidate = false;
    if let Some(occ) = occupancy.get(target.0, target.1) {
        for occupant in occ.iter_layer(layers.object_list_layer) {
            if occupant.entity_id == mover_id {
                continue;
            }
            if ignored_blockers.is_some_and(|ids| ids.contains(&occupant.entity_id)) {
                continue;
            }
            saw_candidate = true;
            if crushable.contains(&occupant.entity_id) {
                continue;
            }
            let candidate = classify_blocker(
                occupant.entity_id,
                mover_owner,
                entities,
                alliances,
                interner,
            );
            // VERA-internal generalisation, gamemd equivalent UNCHECKED: a
            // running code of 7 aborts the walk here. Native has no such
            // threshold — it has four specific `return 7` sites (allied
            // building, head-on, unarmed mover, garrison-not-allowed) and
            // otherwise only ever raises. The outcome agrees wherever VERA's
            // classifier produces 7 only at those four, which it does today;
            // a fifth producer would silently inherit the early exit.
            if candidate.yr_code() >= CellEntryResult::Impassable.yr_code() {
                return apply_overrides(CellEntryResult::Impassable, mover_locomotor);
            }
            if candidate.yr_code() > worst.yr_code() {
                worst = candidate;
            }
        }
    }

    if !saw_candidate {
        // **VERA-internal, gamemd has no equivalent.** Exhausting the object
        // list in either `Can_Enter_Cell` simply falls through to the tail with
        // the running code unchanged — a walk that found nothing returns 0.
        // This arm fabricates a hard block instead, on the reasoning that Phase
        // 1 would have answered Clear for a genuinely empty cell.
        //
        // Trigger: the occupancy grid says a cell has occupants but none of them
        // resolves to a live `EntityStore` entity the walk will look at.
        // Player effect: the mover is told the cell is impassable and the order
        // is refused or re-pathed; retail would have entered. Frequency: zero
        // while occupancy and `EntityStore` agree — it is a consistency
        // backstop, not a modelled rule. Downstream risk: it converts any future
        // occupancy desync from a silent inconsistency into a visible refused
        // order, which is arguably the safer failure but is not parity.
        if ignored_blockers.is_some() {
            return apply_overrides(CellEntryResult::Clear, mover_locomotor);
        }
        return apply_overrides(CellEntryResult::Impassable, mover_locomotor);
    }

    if worst == CellEntryResult::Clear
        && !victims.is_empty()
        && bump_crush::cell_passable_after_crush(
            target,
            occupancy,
            layers.occupancy_bits_layer,
            crush_capability,
            entities,
        )
    {
        return apply_overrides(CellEntryResult::Crushable { victims }, mover_locomotor);
    }

    apply_overrides(worst, mover_locomotor)
}

/// Phase-2 classification including the independent Unit occupation plane.
/// A bit-only reservation has no destination-list blocker to attack or scatter,
/// so it keeps native code 2 without inventing a `CellOccupant`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_occupied_cell_with_layers_and_ignored_and_occupation(
    target: (u16, u16),
    layers: CanEnterLayerContext,
    mover_id: u64,
    crush_capability: bump_crush::CrushCapability,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    ignored_blockers: Option<&BTreeSet<u64>>,
    occupancy: &OccupancyGrid,
    cell_occupation: &CellOccupationGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    let result = classify_occupied_cell_with_layers_and_ignored(
        target,
        layers,
        mover_id,
        crush_capability,
        mover_owner,
        mover_locomotor,
        mover_bypass_grid,
        ignored_blockers,
        occupancy,
        entities,
        alliances,
        interner,
    );
    if matches!(result, CellEntryResult::Clear | CellEntryResult::Impassable)
        && cell_occupation.occupied_by_other(
            target.0,
            target.1,
            layers.occupancy_bits_layer,
            mover_id,
        )
    {
        apply_overrides(CellEntryResult::TemporaryOccupation, mover_locomotor)
    } else {
        result
    }
}

/// First blocker entity in the selected layer's cell list.
///
/// The production classifier no longer stops at this entity — it walks the whole
/// list and keeps the worst code, matching the native predicate. This helper is
/// retained only to pin the list ordering and the ignore/self-skip rules.
///
/// Live building exceptions are supplied through `ignored_blockers`; bypassing
/// the static path grid does not suppress structure occupants by itself.
#[cfg(test)]
fn find_primary_blocker(
    target: (u16, u16),
    layer: MovementLayer,
    mover_id: u64,
    _mover_bypass_grid: bool,
    ignored_blockers: Option<&BTreeSet<u64>>,
    occupancy: &OccupancyGrid,
    _entities: &EntityStore,
) -> Option<u64> {
    let occ = occupancy.get(target.0, target.1)?;
    for occupant in occ.iter_layer(layer) {
        if occupant.entity_id == mover_id {
            continue;
        }
        if ignored_blockers.is_some_and(|ids| ids.contains(&occupant.entity_id)) {
            continue;
        }
        return Some(occupant.entity_id);
    }
    None
}

/// Classify a single blocker as enemy, friendly-moving, or friendly-stationary.
fn classify_blocker(
    blocker_id: u64,
    mover_owner: &str,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    let Some(blocker) = entities.get(blocker_id) else {
        return CellEntryResult::Impassable;
    };
    let is_friendly =
        houses::are_houses_friendly(alliances, mover_owner, interner.resolve(blocker.owner));
    if !is_friendly {
        return CellEntryResult::OccupiedEnemy { blocker_id };
    }
    if blocker
        .building_gate
        .is_some_and(|gate| !gate.can_garrison_passable())
    {
        return CellEntryResult::ScatterRequired {
            blocker_id: Some(blocker_id),
        };
    }
    // Friendly and not moving. A BuildingClass occupant is a HARD block here,
    // not code 6: the ally/not-moving arm of the native predicate tests the
    // blocker's class first and returns impassable for a building. Code 6 would
    // otherwise send the mover down the scatter-and-wait path, which asks a
    // *structure* to move out of the way and hands out a grace period the
    // native code-7 path never gives. The runtime consumer treats a Structure
    // blocker as a hard block to match; do not add a second gate on top of it.
    if blocker.category == EntityCategory::Structure {
        return CellEntryResult::Impassable;
    }
    // Friendly: moving -> temporary block, stationary -> code 6.
    if blocker.movement_target.is_some() {
        CellEntryResult::TemporaryBlock { blocker_id }
    } else {
        CellEntryResult::FriendlyStationary { blocker_id }
    }
}

/// Apply locomotor-specific overrides to a cell entry result.
///
/// **VERA-internal, gamemd equivalent UNCHECKED.** JumpJet: every code except
/// Impassable is lowered to Clear. The previous citation here — "deep_113 line
/// 861" — is not an address or a named research doc and does not meet the
/// provenance form; it is dropped rather than dressed up.
///
/// The nearest native mechanism runs the other way round.
/// `FootClass::LocomotorPassabilityCheck` @ `0x004D9C10` dispatches the mover's
/// locomotor vtable `+0x1C` and **seeds** the running code before the occupant
/// walk, is Unit-only, is gated on a caller flag byte, and can only be raised
/// afterwards — nothing in either `Can_Enter_Cell` lowers an accumulated code at
/// the end. What `JumpjetLocomotionClass+0x1C` returns is UNCHECKED. Trigger:
/// any jumpjet mover meeting an occupied or soft-blocked cell. Player effect:
/// jumpjets ignore ground traffic, which is the retail feel; whether they ignore
/// it by this route is unverified. Frequency: every Rocketeer and Floating Disc
/// order. Downstream risk: replacing this with the native seed changes where the
/// locomotor hook sits relative to the walk, so it is a restructure rather than
/// a swap.
fn apply_overrides(result: CellEntryResult, locomotor: LocomotorKind) -> CellEntryResult {
    if locomotor == LocomotorKind::Jumpjet && !matches!(result, CellEntryResult::Impassable) {
        return CellEntryResult::Clear;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::occupancy::CellListInsertion;

    fn empty_occ() -> OccupancyGrid {
        OccupancyGrid::new()
    }

    #[test]
    fn test_clear_empty_cell() {
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            None,
            None,
            &empty_occ(),
        );
        assert_eq!(result, TerrainCheckResult::Clear);
    }

    #[test]
    fn test_impassable_blocked_grid() {
        use crate::sim::pathfinding::PathGrid;
        let mut grid = PathGrid::new(10, 10);
        grid.set_blocked(5, 5, true);
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            Some(&grid),
            None,
            &empty_occ(),
        );
        assert_eq!(result, TerrainCheckResult::Impassable);
    }

    #[test]
    fn test_vehicle_occupied_needs_check() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            42,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Unit,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn test_infantry_subcell_available() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Infantry,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::Clear);
    }

    #[test]
    fn test_infantry_cell_full() {
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            11,
            MovementLayer::Ground,
            Some(3),
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            12,
            MovementLayer::Ground,
            Some(4),
            CellListInsertion::PrependNonBuilding,
        );
        let result = check_terrain(
            (5, 5),
            MovementLayer::Ground,
            EntityCategory::Infantry,
            None,
            None,
            &occ,
        );
        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn test_jumpjet_override_clears_non_impassable() {
        let result = apply_overrides(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 },
            LocomotorKind::Jumpjet,
        );
        assert_eq!(result, CellEntryResult::Clear);
    }

    #[test]
    fn test_jumpjet_keeps_impassable() {
        let result = apply_overrides(CellEntryResult::Impassable, LocomotorKind::Jumpjet);
        assert_eq!(result, CellEntryResult::Impassable);
    }

    #[test]
    fn test_non_jumpjet_no_override() {
        let result = apply_overrides(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 },
            LocomotorKind::Drive,
        );
        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 1 });
    }

    #[test]
    fn cell_entry_result_yr_codes_match_verified_table() {
        assert_eq!(CellEntryResult::Clear.yr_code(), 0);
        assert_eq!(CellEntryResult::Crushable { victims: vec![1] }.yr_code(), 1);
        assert_eq!(
            CellEntryResult::TemporaryBlock { blocker_id: 1 }.yr_code(),
            2
        );
        assert_eq!(
            CellEntryResult::ScatterRequired {
                blocker_id: Some(1),
            }
            .yr_code(),
            3
        );
        assert_eq!(CellEntryResult::FriendlyWall.yr_code(), 4);
        assert_eq!(
            CellEntryResult::OccupiedEnemy { blocker_id: 1 }.yr_code(),
            5
        );
        assert_eq!(
            CellEntryResult::FriendlyStationary { blocker_id: 1 }.yr_code(),
            6
        );
        assert_eq!(CellEntryResult::Impassable.yr_code(), 7);
    }

    #[test]
    fn friendly_closed_or_opening_gate_returns_code_3_not_code_6() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGatePhase, BuildingGateRuntime, GameEntity};

        let mut entities = EntityStore::new();
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Americans", 5, 5);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime::default());
        entities.insert(gate);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(
            result,
            CellEntryResult::ScatterRequired {
                blocker_id: Some(100)
            }
        );
        assert_eq!(result.yr_code(), 3);

        entities.get_mut(100).unwrap().building_gate = Some(BuildingGateRuntime {
            mission_18_active: true,
            phase: BuildingGatePhase::Opening,
            ..Default::default()
        });
        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(
            result,
            CellEntryResult::ScatterRequired {
                blocker_id: Some(100)
            }
        );
        assert_eq!(result.yr_code(), 3);
    }

    #[test]
    fn allied_stationary_building_is_hard_blocked_not_code_6() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut entities = EntityStore::new();
        let mut refinery = GameEntity::test_default(200, "GAREFN", "Americans", 5, 5);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(200, "Americans", &entities, &alliances, &interner);
        assert_eq!(result, CellEntryResult::Impassable);
        assert_eq!(result.yr_code(), 7);
    }

    #[test]
    fn allied_stationary_unit_still_returns_code_6() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut entities = EntityStore::new();
        let mut tank = GameEntity::test_default(201, "GTNK", "Americans", 5, 5);
        tank.category = EntityCategory::Unit;
        entities.insert(tank);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(201, "Americans", &entities, &alliances, &interner);
        assert_eq!(
            result,
            CellEntryResult::FriendlyStationary { blocker_id: 201 }
        );
        assert_eq!(result.yr_code(), 6);
    }

    #[test]
    fn whole_cell_list_is_classified_and_the_worst_occupant_wins() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        // Two occupants: a friendly stationary unit at the list head (code 6)
        // and an enemy behind it. gamemd walks the whole list and raises the
        // running code, so the enemy must not be lost to list order — but code 6
        // outranks code 5, so the friendly still wins here. The reverse ordering
        // must give the same answer.
        let mut entities = EntityStore::new();
        let mut ally = GameEntity::test_default(300, "GTNK", "Americans", 5, 5);
        ally.category = EntityCategory::Unit;
        entities.insert(ally);
        let mut foe = GameEntity::test_default(301, "HTNK", "Soviets", 5, 5);
        foe.category = EntityCategory::Unit;
        entities.insert(foe);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        for order in [[300u64, 301u64], [301, 300]] {
            let mut occ = OccupancyGrid::new();
            for id in order {
                occ.add(
                    5,
                    5,
                    id,
                    MovementLayer::Ground,
                    None,
                    CellListInsertion::AppendBuilding,
                );
            }
            let result = classify_occupied_cell(
                (5, 5),
                MovementLayer::Ground,
                999,
                bump_crush::CrushCapability::new(false, false),
                "Americans",
                LocomotorKind::Drive,
                false,
                &occ,
                &entities,
                &alliances,
                &interner,
            );
            assert_eq!(
                result,
                CellEntryResult::FriendlyStationary { blocker_id: 300 },
                "order={order:?}"
            );
        }
    }

    #[test]
    fn a_structure_behind_a_unit_still_hard_blocks_the_cell() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        // Taking only the list head would report code 6 and send the mover into
        // the scatter/grace path; the whole-list walk finds the structure.
        let mut entities = EntityStore::new();
        let mut ally = GameEntity::test_default(310, "GTNK", "Americans", 5, 5);
        ally.category = EntityCategory::Unit;
        entities.insert(ally);
        let mut refinery = GameEntity::test_default(311, "GAREFN", "Americans", 5, 5);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            310,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        occ.add(
            5,
            5,
            311,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );

        let result = classify_occupied_cell(
            (5, 5),
            MovementLayer::Ground,
            999,
            bump_crush::CrushCapability::new(false, false),
            "Americans",
            LocomotorKind::Drive,
            false,
            &occ,
            &entities,
            &alliances,
            &interner,
        );
        assert_eq!(result, CellEntryResult::Impassable);
    }

    #[test]
    fn enemy_closed_gate_keeps_enemy_result_code() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::{BuildingGateRuntime, GameEntity};

        let mut entities = EntityStore::new();
        let mut gate = GameEntity::test_default(100, "GAGATE_A", "Soviets", 5, 5);
        gate.category = EntityCategory::Structure;
        gate.building_gate = Some(BuildingGateRuntime::default());
        entities.insert(gate);
        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();

        let result = classify_blocker(100, "Americans", &entities, &alliances, &interner);
        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 100 });
        assert_eq!(result.yr_code(), 5);
    }

    fn row_entry_input(
        mover_category: EntityCategory,
        branch: VehicleBuildingEntryBranch,
        candidate_x: u16,
    ) -> LiveVehicleBuildingEntry {
        LiveVehicleBuildingEntry {
            mover_category,
            branch,
            checked_building_id: 100,
            candidate_building_id: Some(100),
            candidate_x,
            building_origin_x: 10,
            number_impassable_rows: 1,
            is_unit_repair: false,
            is_bunker: false,
            bunker_occupied: false,
        }
    }

    #[test]
    fn infantry_does_not_use_vehicle_row_contact_skip() {
        let input = row_entry_input(
            EntityCategory::Infantry,
            VehicleBuildingEntryBranch::RadioContact {
                mover_has_contact: true,
            },
            11,
        );

        assert_eq!(
            decide_live_vehicle_building_entry(input),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn contacted_vehicle_row_skip_opens_east_columns_but_keeps_west() {
        let contacted = VehicleBuildingEntryBranch::RadioContact {
            mover_has_contact: true,
        };
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                contacted,
                10,
            )),
            BuildingOccupantEntryDecision::KeepBlocker
        );
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                contacted,
                11,
            )),
            BuildingOccupantEntryDecision::SkipBlocker
        );
        assert_eq!(
            decide_live_vehicle_building_entry(row_entry_input(
                EntityCategory::Unit,
                VehicleBuildingEntryBranch::RadioContact {
                    mover_has_contact: false,
                },
                11,
            )),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn empty_vs_occupied_bunker_uses_explicit_runtime_occupant_arg() {
        let mut empty = row_entry_input(
            EntityCategory::Unit,
            VehicleBuildingEntryBranch::UnitRepairOrBunker,
            10,
        );
        empty.number_impassable_rows = 0;
        empty.is_bunker = true;

        assert_eq!(
            decide_live_vehicle_building_entry(empty),
            BuildingOccupantEntryDecision::SkipBlocker
        );

        let occupied = LiveVehicleBuildingEntry {
            bunker_occupied: true,
            ..empty
        };
        assert_eq!(
            decide_live_vehicle_building_entry(occupied),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn row_helper_requires_same_candidate_building_and_rows_value() {
        let mut other_building = row_entry_input(
            EntityCategory::Unit,
            VehicleBuildingEntryBranch::UnitRepairOrBunker,
            11,
        );
        other_building.is_unit_repair = true;
        other_building.candidate_building_id = Some(200);
        assert_eq!(
            decide_live_vehicle_building_entry(other_building),
            BuildingOccupantEntryDecision::KeepBlocker
        );

        let no_rows = LiveVehicleBuildingEntry {
            candidate_building_id: Some(100),
            number_impassable_rows: -1,
            ..other_building
        };
        assert_eq!(
            decide_live_vehicle_building_entry(no_rows),
            BuildingOccupantEntryDecision::KeepBlocker
        );
    }

    #[test]
    fn find_primary_blocker_does_not_use_bypass_grid_as_structure_skip() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        // Cell occupancy: a Structure (refinery) at (5, 5).
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            100,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );

        // EntityStore with the structure entity.
        let mut entities = EntityStore::new();
        let mut refinery = GameEntity::test_default(100, "GAREFN", "Allies", 5, 5);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);

        // With bypass_grid=true: structure is filtered, no other occupants → None.
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,   // mover_id
            true, // mover_bypass_grid
            None,
            &occ,
            &entities,
        );
        assert_eq!(
            result,
            Some(100),
            "bypass_grid must not erase live structure blockers"
        );

        // With bypass_grid=false: structure is the primary blocker → Some(100).
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false, // mover_bypass_grid
            None,
            &occ,
            &entities,
        );
        assert_eq!(
            result,
            Some(100),
            "with bypass_grid=false, Structure must still be picked as blocker (regression)"
        );
    }

    #[test]
    fn find_primary_blocker_follows_layer_order() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );

        let mut entities = EntityStore::new();
        let mut blocker = GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
        blocker.category = EntityCategory::Unit;
        entities.insert(blocker);
        let mut infantry = GameEntity::test_default(20, "E1", "Allies", 5, 5);
        infantry.category = EntityCategory::Infantry;
        entities.insert(infantry);

        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false,
            None,
            &occ,
            &entities,
        );
        assert_eq!(result, Some(20));
    }

    #[test]
    fn find_primary_blocker_skips_caller_ignored_ids() {
        use crate::sim::entity_store::EntityStore;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let ignored = std::collections::BTreeSet::from([10]);
        let entities = EntityStore::new();
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false,
            Some(&ignored),
            &occ,
            &entities,
        );

        assert_eq!(result, Some(20));
    }

    #[test]
    fn split_context_uses_occupancy_bits_layer_for_presence() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let result = check_terrain_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            EntityCategory::Unit,
            Some(&grid),
            None,
            &occ,
        );

        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn oracle_wrapper_preserves_split_layers_and_yr_code() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let layers = CanEnterLayerContext {
            terrain_layer: MovementLayer::Bridge,
            object_list_layer: MovementLayer::Bridge,
            occupancy_bits_layer: MovementLayer::Ground,
        };
        let (result, row) = check_terrain_with_layers_oracle(
            (5, 5),
            layers,
            EntityCategory::Unit,
            Some(&grid),
            None,
            &empty_occ(),
        );

        assert_eq!(result, TerrainCheckResult::Clear);
        assert_eq!(row.terrain_layer, MovementLayer::Bridge);
        assert_eq!(row.object_list_layer, MovementLayer::Bridge);
        assert_eq!(row.occupancy_bits_layer, MovementLayer::Ground);
        assert_eq!(row.yr_code, Some(0));
    }

    #[test]
    fn split_context_uses_object_list_layer_for_selected_blockers() {
        use crate::sim::pathfinding::PathGrid;

        let mut grid = PathGrid::new(10, 10);
        grid.set_cell_for_test(5, 5, 0, true, true);
        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let result = check_terrain_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            EntityCategory::Unit,
            Some(&grid),
            None,
            &occ,
        );

        assert_eq!(result, TerrainCheckResult::NeedsBlockerCheck);
    }

    #[test]
    fn split_context_scans_object_list_layer_for_primary_blocker() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        let mut occ = OccupancyGrid::new();
        occ.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occ.add(
            5,
            5,
            20,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let mut entities = EntityStore::new();
        let mut ground = GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
        ground.category = EntityCategory::Unit;
        entities.insert(ground);
        let mut bridge = GameEntity::test_default(20, "HTNK", "Soviets", 5, 5);
        bridge.category = EntityCategory::Unit;
        entities.insert(bridge);

        let alliances = HouseAllianceMap::new();
        let interner = crate::sim::intern::test_interner();
        let result = classify_occupied_cell_with_layers(
            (5, 5),
            CanEnterLayerContext {
                terrain_layer: MovementLayer::Bridge,
                object_list_layer: MovementLayer::Bridge,
                occupancy_bits_layer: MovementLayer::Ground,
            },
            42,
            bump_crush::CrushCapability::new(false, false),
            "Allies",
            LocomotorKind::Drive,
            false,
            &occ,
            &entities,
            &alliances,
            &interner,
        );

        assert_eq!(result, CellEntryResult::OccupiedEnemy { blocker_id: 20 });
    }

    #[test]
    fn yr_search_cost_class_rejects_seven_without_neighbor_cost() {
        assert_eq!(
            search_cell_cost_decision(7, false),
            SearchCellCostDecision {
                raw_cost_class: 7,
                effective_cost_class: None,
                expands: false,
                should_call_neighbor_step_cost: false,
            }
        );
    }

    #[test]
    fn yr_search_cost_class_preserves_accepted_class_when_gate_is_off() {
        let decision = search_cell_cost_decision(4, false);
        assert_eq!(decision.effective_cost_class, Some(4));
        assert!(decision.expands);
        assert!(decision.should_call_neighbor_step_cost);
    }

    #[test]
    fn yr_search_cost_class_coerces_accepted_class_when_gate_is_on() {
        let decision = search_cell_cost_decision(6, true);
        assert_eq!(decision.effective_cost_class, Some(0));
        assert!(decision.expands);
        assert!(decision.should_call_neighbor_step_cost);
    }

    #[test]
    fn yr_search_cost_class_two_remains_an_accepted_special_case_input() {
        let decision = search_cell_cost_decision(2, false);
        assert_eq!(decision.effective_cost_class, Some(2));
        assert!(decision.expands);
        assert!(decision.should_call_neighbor_step_cost);
    }
}
