//! Committed Drive/Ship head coordinates and their curve anchor.
//!
//! Fresh Drive4B32AF/4B40B0 and Ship6A28FF/6A36E0 add path directions to
//! current Foot XYZ. Chain4B1BC4/6A120A instead adds to the previous head.
//! Original executable cases: tools/spatial_oracle/locomotor_head_coordinates.json.

use super::drive_track::{self, DriveTrackPlan, DriveTrackState};
use crate::sim::components::{DriveCoord, Position};
use crate::util::direction_tables::lepton::LEPTON_DELTAS;

/// One original direction-table addition; no terrain or cell-center sampling.
pub(super) fn offset_head(base: DriveCoord, direction: u8) -> DriveCoord {
    let (dx, dy) = LEPTON_DELTAS[usize::from(direction & 7)];
    DriveCoord {
        x: base.x.wrapping_add(dx),
        y: base.y.wrapping_add(dy),
        z: base.z,
    }
}

/// Build the stored coordinate and executable curve together, before either is
/// published. Both use the same exact origin, including noncentered subcells.
pub(super) fn begin_fresh(
    plan: &DriveTrackPlan,
    position: &Position,
) -> Option<(DriveCoord, DriveTrackState)> {
    let current = super::ground_pose::position_world_coord(position);
    let from = (plan.selection.turn_track_index / 8) as u8;
    let mut head = offset_head(current, from);
    if plan.nodes == 2 {
        head = offset_head(head, (plan.selection.turn_track_index % 8) as u8);
    }
    let mut curve = drive_track::begin_drive_track_with_head_offset(
        plan.selection.raw_track_index,
        plan.selection.flags,
        head.x.wrapping_sub(i32::from(position.rx) * 256),
        head.y.wrapping_sub(i32::from(position.ry) * 256),
        plan.selection.target_facing,
    )?;
    // Fresh acceptance stores zero; RawTrack.entry is for chained adoption.
    curve.point_index = 0;
    Some((head, curve))
}

#[cfg(test)]
#[path = "track_head_tests.rs"]
mod tests;
