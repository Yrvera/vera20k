//! Input-time Foot Cell destination queries. Synchronized commands already
//! carry the chosen Cell; their executor never repeats these input queries.

use super::Simulation;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::DriveCoord;
use crate::sim::movement::ground_pose;

impl Simulation {
    /// Resolve the ordinary grounded Walk Cell-click corridor before event
    /// encoding. None leaves other actor/caller families on their existing
    /// input adapter; it is not permission to reinterpret a decoded Move.
    pub(crate) fn ordinary_ground_walk_cell_input(
        &self,
        viewer: crate::sim::intern::InternedId,
        id: u64,
        clicked: (u16, u16),
        rules: &crate::rules::ruleset::RuleSet,
    ) -> Option<Result<Option<(u16, u16)>, String>> {
        let entity = self.substrate.entities.get(id)?;
        let object = rules.object(self.interner.resolve(entity.type_ref()))?;
        if entity.locomotor.as_ref()?.active_kind() != LocomotorKind::Walk
            || object.teleporter
            || object.jumpjet
            || object.movement_zone == MovementZone::Subterranean
        {
            return None;
        }
        let terrain = self.resolved_terrain.as_ref()?;
        let zones = self.zone_grid.as_ref()?;
        let bounds = self.playfield_bounds?;
        let size = self
            .playfield_size_height
            .map(|height| (bounds.base, height))
            .or_else(|| self.bridge_state.as_ref()?.native_zone_source_size())?;
        let current = ground_pose::position_world_coord(&entity.position);
        let cells = NativeCellQuery::isolated(terrain);
        //Classify the bounded ordinary-ground arm without changing canonical
        //query state. High/falling Walk and non-Walk input retain their prior
        //adapter pending their own +70 action selection contract.
        let ground = match query_ground(&cells, current) {
            Ok(ground) => ground,
            Err(error) => return Some(Err(error)),
        };
        let height = current
            .z
            .wrapping_sub(ground)
            .wrapping_sub(if entity.on_bridge {
                BRIDGE_HEIGHT_DELTA_LEPTONS as i32
            } else {
                0
            });
        if entity.lifecycle.cell_marked && height >= 2 * GROUND_LEVEL_HEIGHT_LEPTONS {
            return None;
        }
        let coordinate = match self.infantry_navigation_coordinate(id) {
            Ok(coordinate) => coordinate,
            Err(error) => return Some(Err(error)),
        };
        //One fresh query identity starts from the canonical retained bytes,
        //not from the classification probe above. All nested calls share it.
        let cells = NativeCellQuery::isolated(terrain);
        if cells.dummy().raw_flags() & 3 != 0 {
            return Some(Err(
                "input Dummy visibility outside the constructor-derived domain".into(),
            ));
        }
        Some(resolve_walk_cell_click(
            &cells,
            zones,
            &self.substrate.raw_cell_occupation,
            bounds,
            size,
            self.session.binary_frame,
            &WalkCellClick {
                clicked: (clicked.0 as i16, clicked.1 as i16),
                action: 1,
                current,
                coordinate,
                marked: entity.lifecycle.cell_marked,
                on_bridge: entity.on_bridge,
                in_tube: entity.low_bridge_tube_state.is_some(),
                move_to_shroud: object.move_to_shroud,
                teleporter: false,
                jumpjet_type: false,
                movement_zone: object.movement_zone,
                speed_type: object.speed_type,
            },
            |cell| {
                Ok(match cell {
                    NativeCellIdentity::Real(index) => {
                        let cell = &terrain.cells()[index];
                        self.fog.is_cell_revealed(viewer, cell.rx, cell.ry)
                    }
                    //Bounded ordinary input: successful Resize allocates every
                    //Size-diamond cell; inspected radius/whole-map reveal cannot
                    //bootstrap Dummy+12C from constructor zero (566368/5673A0,
                    //6D8700). Direct sparse/outside reveal leaf histories remain
                    //unrepresented and are not certified by this adapter.
                    NativeCellIdentity::Dummy => false,
                })
            },
        ))
    }

    /// Foot4DBDF0 (+4C), including its TubeClass exit override. This query
    /// precedes the hut/alive/navigation gates, even for pending Uninit rows.
    pub(super) fn infantry_navigation_coordinate(&self, id: u64) -> Result<DriveCoord, String> {
        let e = self
            .substrate
            .entities
            .get(id)
            .ok_or("Infantry coordinate owner disappeared")?;
        if let Some(state) = e.low_bridge_tube_state {
            // Walk75B3FC/Drive4B1380 install Foot+684; Infantry51BA8D
            // retires it. The query reads Tube+28 exit, not the path cursor.
            let tube = self
                .resolved_terrain
                .as_ref()
                .and_then(|terrain| terrain.tube(state.tube_id))
                .ok_or("Infantry coordinate references a missing active TubeClass")?;
            return Ok(DriveCoord {
                x: i32::from(tube.exit.0 as i16) * 256 + 128,
                y: i32::from(tube.exit.1 as i16) * 256 + 128,
                z: 0,
            });
        }
        let loco = e
            .locomotor
            .as_ref()
            .ok_or("Infantry coordinate requires the active Infantry locomotor")?;
        let raw = ground_pose::position_world_coord(&e.position);
        match loco.active_kind() {
            LocomotorKind::Walk => Ok(loco.step_head().unwrap_or(raw)),
            // Teleport+18/55ACA0 copies linked owner+9C exactly.
            LocomotorKind::Teleport => Ok(raw),
            LocomotorKind::Jumpjet => {
                let state = loco
                    .jumpjet_runtime()
                    .ok_or("Jumpjet payload does not match active class")?;
                let coordinate = state.coordinate(raw);
                Ok(
                    if coordinate == crate::sim::movement::jumpjet_movement::JumpjetRuntime::NULL {
                        raw
                    } else {
                        coordinate
                    },
                )
            }
            other => Err(format!(
                "Infantry coordinate requires the {other:?} +18 receiver"
            )),
        }
    }
}

use crate::map::cell_index::NativeCellIdentity;
use crate::map::resolved_terrain::NativeCellQuery;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::cell_rect::{PlayfieldBounds, cell_is_in_playfield_height_aware_in_query};
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, PassabilityArgs, find_nearby_passable_cell,
    map_owned_radius_cap,
};
use crate::sim::occupancy::RawCellOccupationGrid;
use crate::sim::pathfinding::zone_map::{ZoneGrid, cell_is_in_native_map_diamond};
use crate::util::lepton::{BRIDGE_HEIGHT_DELTA_LEPTONS, GROUND_LEVEL_HEIGHT_LEPTONS};

/// Values read by the ordinary Infantry/Walk4DE1D0 input receiver. The +70
/// action has already been selected at the input boundary; it is not a mission.
struct WalkCellClick {
    clicked: (i16, i16),
    action: u32,
    current: DriveCoord,
    coordinate: DriveCoord,
    marked: bool,
    on_bridge: bool,
    in_tube: bool,
    move_to_shroud: bool,
    teleporter: bool,
    jumpjet_type: bool,
    movement_zone: MovementZone,
    speed_type: SpeedType,
}

fn query_ground(cells: &NativeCellQuery<'_>, point: DriveCoord) -> Result<i32, String> {
    let cell = cells.lookup_world(point.x, point.y);
    let (level, slope) = cells.ground_fields(cell);
    crate::util::lepton::ground_height_leptons(level, slope, point.x, point.y)
        .map_err(|error| format!("input ground query: {error:?}"))
}

fn clicked_coordinate(
    cells: &NativeCellQuery<'_>,
    clicked: (i16, i16),
) -> Result<DriveCoord, String> {
    let mut point = DriveCoord {
        x: i32::from(clicked.0) * 256 + 128,
        y: i32::from(clicked.1) * 256 + 128,
        z: 0,
    };
    point.z = query_ground(cells, point)?;
    //4DE242/4DE38C are separate Map calls after the height query.
    if cells.flags(cells.lookup_world(point.x, point.y)) & 0x100 != 0 {
        point.z = point.z.wrapping_add(BRIDGE_HEIGHT_DELTA_LEPTONS as i32);
    }
    Ok(point)
}

///586360 queries the projected Cell's ground-open bit. Keep its odd-height
///481810(3) lookup conditional and use the first receiver's own coordinates.
fn coordinate_is_shrouded(
    cells: &NativeCellQuery<'_>,
    point: DriveCoord,
    open: &impl Fn(NativeCellIdentity) -> Result<bool, String>,
) -> Result<bool, String> {
    let level = point.z / GROUND_LEVEL_HEIGHT_LEPTONS;
    let shift = level / 2 + i32::from(level & 1 != 0);
    let first = cells.lookup((
        ((point.x / 256) as i16).wrapping_sub(shift as i16),
        ((point.y / 256) as i16).wrapping_sub(shift as i16),
    ));
    if open(first)? {
        return Ok(false);
    }
    if level & 1 != 0 {
        let (x, y) = cells.coord(first);
        let next = cells.lookup((x.wrapping_add(1), y.wrapping_add(1)));
        return open(next).map(|open| !open);
    }
    Ok(true)
}

fn source_should_be_on_bridge(
    cells: &NativeCellQuery<'_>,
    click: &WalkCellClick,
) -> Result<bool, String> {
    if click.in_tube {
        return Ok(false);
    }
    //5F6A70 queries +4C/head ground before retained Object XYZ ground.
    let head_ground = query_ground(cells, click.coordinate)?;
    let current_ground = query_ground(cells, click.current)?;
    if !click.on_bridge
        && current_ground.wrapping_sub(head_ground) > 3 * GROUND_LEVEL_HEIGHT_LEPTONS
    {
        return Ok(
            cells.flags(cells.lookup_world(click.coordinate.x, click.coordinate.y)) & 0x100 != 0,
        );
    }
    if click.on_bridge && head_ground.wrapping_sub(current_ground) > 3 * GROUND_LEVEL_HEIGHT_LEPTONS
    {
        return Ok(false);
    }
    Ok(click.on_bridge)
}

fn click_can_reach(
    cells: &NativeCellQuery<'_>,
    zones: &ZoneGrid,
    source: NativeCellIdentity,
    target: (i16, i16),
    mz: MovementZone,
    source_bridge: bool,
    target_bridge: bool,
    bounds: PlayfieldBounds,
    size: (i32, i32),
) -> Result<bool, String> {
    let source_coord = cells.coord(source);
    let source_pair = (i32::from(source_coord.0), i32::from(source_coord.1));
    if !cell_is_in_playfield_height_aware_in_query(
        source_pair,
        Some(bounds),
        Some(cells.terrain()),
        Some(cells),
    ) && cell_is_in_native_map_diamond(source_pair, size.0, size.1)
    {
        return Ok(true);
    }
    //56D187 still queries destination playfield even when arg6=false makes
    //the asymmetric destination shortcut unavailable. Preserve lookup effects.
    let _ = cell_is_in_playfield_height_aware_in_query(
        (i32::from(target.0), i32::from(target.1)),
        Some(bounds),
        Some(cells.terrain()),
        Some(cells),
    );
    //56D1FC resolves target before source. A source Dummy remains live here.
    let target_zone = zones
        .get_path_zone_id_native_in_query(
            cells.terrain(),
            (target.0 as u16, target.1 as u16),
            mz,
            target_bridge,
            Some(cells),
        )
        .ok_or("input target lacks native zone topology")?;
    let p = cells.coord(source);
    let source_zone = zones
        .get_path_zone_id_native_in_query(
            cells.terrain(),
            (p.0 as u16, p.1 as u16),
            mz,
            source_bridge,
            Some(cells),
        )
        .ok_or("input source lacks native zone topology")?;
    Ok(source_zone == target_zone)
}

/// Original4DE1D0 under the ordinary Walk type/actor gates. No A* search and
/// no entity/mission/navigation mutation occurs here. Native comparisons live
/// in tools/spatial_oracle/walk_move_admission (original input action supplied).
fn resolve_walk_cell_click(
    cells: &NativeCellQuery<'_>,
    zones: &ZoneGrid,
    raw: &RawCellOccupationGrid,
    bounds: PlayfieldBounds,
    size: (i32, i32),
    frame: u32,
    click: &WalkCellClick,
    open: impl Fn(NativeCellIdentity) -> Result<bool, String>,
) -> Result<Option<(u16, u16)>, String> {
    let terrain = cells.terrain();
    let in_playfield = cell_is_in_playfield_height_aware_in_query(
        (i32::from(click.clicked.0), i32::from(click.clicked.1)),
        Some(bounds),
        Some(terrain),
        Some(cells),
    );
    let point = clicked_coordinate(cells, click.clicked)?;
    if !click.move_to_shroud && coordinate_is_shrouded(cells, point, &open)? {
        return Ok(None);
    }
    let source = cells.lookup_world(click.coordinate.x, click.coordinate.y);
    //5F6B90 samples height only when Object+74 is true; +1C8 subtracts
    //OnBridge's height independently of the ShouldBeOnBridge query below.
    let high_flying = if click.marked {
        let height = click
            .current
            .z
            .wrapping_sub(query_ground(cells, click.current)?)
            .wrapping_sub(if click.on_bridge {
                BRIDGE_HEIGHT_DELTA_LEPTONS as i32
            } else {
                0
            });
        height >= 2 * GROUND_LEVEL_HEIGHT_LEPTONS
    } else {
        false
    };
    if click.movement_zone == MovementZone::Subterranean {
        return Err("Subterranean input receiver is outside the ordinary Walk domain".into());
    }
    let special = high_flying || click.jumpjet_type;
    let source_bridge = source_should_be_on_bridge(cells, click)?;
    let point = clicked_coordinate(cells, click.clicked)?;
    let shrouded = coordinate_is_shrouded(cells, point, &open)?;
    let unrestricted = special && click.action == 2;
    if !unrestricted && in_playfield && !shrouded {
        if special || (click.teleporter && click.action != 2) {
            return Ok((click.clicked != (0, 0))
                .then_some((click.clicked.0 as u16, click.clicked.1 as u16)));
        }
        let bridge = cells.flags(cells.lookup(click.clicked)) & 0x100 != 0;
        if click_can_reach(
            cells,
            zones,
            source,
            click.clicked,
            click.movement_zone,
            source_bridge,
            bridge,
            bounds,
            size,
        )? {
            return Ok((click.clicked != (0, 0))
                .then_some((click.clicked.0 as u16, click.clicked.1 as u16)));
        }
    }
    let target_bridge = cells.flags(cells.lookup(click.clicked)) & 0x100 != 0;
    let required_zone = if unrestricted {
        None
    } else {
        let p = cells.coord(source);
        let zone = zones
            .get_path_zone_id_native_in_query(
                terrain,
                (p.0 as u16, p.1 as u16),
                click.movement_zone,
                source_bridge,
                Some(cells),
            )
            .ok_or("input FNPC source lacks native zone topology")?;
        //Native DWORD-1 disables the comparison; raw WORDFFFF does not.
        u16::try_from(zone).ok()
    };
    Ok(find_nearby_passable_cell(
        (i32::from(click.clicked.0), i32::from(click.clicked.1)),
        &NearbyQuery {
            native_cells: Some(cells),
            raw_occupation: Some(raw),
            passability: PassabilityArgs {
                speed_type: click.speed_type,
                required_zone_id: required_zone,
                movement_zone: click.movement_zone,
                bridge_aware_zone: target_bridge,
            },
            footprint: NearbyFootprint::SINGLE,
            anchor_gate: NearbyAnchorGate::NativeHeightAware,
            allow_bridge_cells: true,
            check_height: true,
            check_occupancy: false,
            radius_cap: map_owned_radius_cap(size.0, size.1),
            target_cell: None,
            path_grid: None,
            resolved_terrain: Some(terrain),
            overlay_grid: None,
            occupancy: None,
            entities: None,
            zone_grid: Some(zones),
            playfield_bounds: Some(bounds),
        },
        frame,
    )
    .filter(|cell| *cell != (0, 0)))
}

#[cfg(test)]
#[path = "move_cell_input_tests.rs"]
mod tests;
