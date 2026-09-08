//! Grounded ObjectClass coordinate writes shared by movement and placement.
//!
//! Native FootClass::Set_Height_On_Bridge @ 0x005F5FA0 samples the committed
//! world XY through GetGroundHeight @ 0x00578080, then adds the explicit
//! OnBridge offset. Callers own cadence: Drive/Ship residual movement does
//! not call this setter and retains the last raw coordinate Z.

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::components::Position;
use crate::sim::pathfinding::PathGrid;
use crate::util::lepton::{BRIDGE_HEIGHT_DELTA_LEPTONS, ground_height_leptons};

pub(crate) fn position_world_xy(position: &Position) -> [i32; 2] {
    [
        i32::from(position.rx)
            .wrapping_mul(256)
            .wrapping_add(position.sub_x.to_num::<i32>()),
        i32::from(position.ry)
            .wrapping_mul(256)
            .wrapping_add(position.sub_y.to_num::<i32>()),
    ]
}

/// Sample the live surface at full world XY. A PathGrid supplies the same
/// level/ramp fields only for callers without resolved terrain. Missing
/// headless terrain leaves the caller's existing coordinate authoritative.
pub(crate) fn ground_surface_z_at(
    world_xy: [i32; 2],
    on_bridge: bool,
    terrain: Option<&ResolvedTerrainGrid>,
    path_grid: Option<&PathGrid>,
) -> Option<i32> {
    let rx = (world_xy[0] / 256) as i16;
    let ry = (world_xy[1] / 256) as i16;
    let (level, slope) = if let Some(terrain) = terrain {
        if let Some(index) = terrain.native_fixed_cell_index(rx, ry) {
            let cell = &terrain.cells()[index];
            (cell.level, cell.slope_type)
        } else {
            // GetGroundHeight @ 0x578080 evaluates the shared dummy's live
            // fields too. Its default zero height is not an invariant.
            let shared = terrain.shared_cell_dummy();
            shared.stamp_coord(i32::from(rx), i32::from(ry));
            let dummy = shared.snapshot();
            (dummy.level as u8, dummy.slope_type)
        }
    } else {
        let cell = path_grid?.cell(rx as u16, ry as u16)?;
        (cell.ground_level, cell.slope_type)
    };
    let ground = match ground_height_leptons(level, slope, world_xy[0], world_xy[1]) {
        Ok(ground) => ground,
        Err(_) => {
            log::warn!(
                "ground pose at {world_xy:?} has unsupported slope {slope}; retaining raw Z"
            );
            return None;
        }
    };
    Some(ground.wrapping_add(if on_bridge {
        BRIDGE_HEIGHT_DELTA_LEPTONS as i32
    } else {
        0
    }))
}

pub(crate) fn commit_ground_height(
    position: &mut Position,
    on_bridge: bool,
    terrain: Option<&ResolvedTerrainGrid>,
    path_grid: Option<&PathGrid>,
) -> bool {
    let Some(z) = ground_surface_z_at(position_world_xy(position), on_bridge, terrain, path_grid)
    else {
        return false;
    };
    position.exact_z_leptons = Some(z);
    true
}
