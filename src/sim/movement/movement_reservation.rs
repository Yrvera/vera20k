//! Destination commitment — commits the infantry sub-cell or the vehicle
//! cell-center dest after a successful cell transition. Previously also wrote to
//! local reservation sets; now the live OccupancyGrid is the single source of
//! truth.
//!
//! **This is the arrival side and it consumes no RNG.** The original engine's
//! arrival branch hands its sub-cell chooser a null coordinate, which returns
//! before any placement runs, so no preference table is consulted and no random
//! draw is taken. The slot was chosen one cell earlier by the look-ahead
//! placement; here it is only claimed. Adding a draw here would double the
//! scenario-stream consumption of the highest-rate consumer in movement.

use crate::map::entities::EntityCategory;
use crate::sim::components::{MovementTarget, Position};
use crate::sim::movement::bump_crush;
use crate::sim::movement::drive_track::DriveTrackState;
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::occupancy::OccupancyGrid;

pub(super) fn reserve_destination_after_transition(
    category: EntityCategory,
    entity_id: u64,
    locomotor: &mut Option<LocomotorState>,
    drive_track: &mut Option<DriveTrackState>,
    position: &mut Position,
    sub_cell: &mut Option<u8>,
    target: &mut MovementTarget,
    next_layer: MovementLayer,
    nx: u16,
    ny: u16,
    occupancy: &OccupancyGrid,
    priority: bool,
) -> bool {
    if category == EntityCategory::Infantry {
        // Priority placement bypasses every occupancy and blocker gate, exactly
        // as the original engine's priority branch does.
        let claimed = if priority {
            Some(bump_crush::priority_sub_cell(
                position.sub_x,
                position.sub_y,
            ))
        } else {
            // The slot the look-ahead reserved for this cell, recovered from
            // the lepton destination it stored on the locomotor.
            let preferred = locomotor
                .as_ref()
                .and_then(|loco| loco.subcell_dest)
                .and_then(bump_crush::functional_sub_cell_from_offset);
            bump_crush::claim_reserved_sub_cell(
                occupancy.get(nx, ny),
                next_layer,
                entity_id,
                preferred,
            )
        };
        let Some(sub) = claimed else {
            position.sub_x = crate::util::lepton::CELL_CENTER_LEPTON;
            position.sub_y = crate::util::lepton::CELL_CENTER_LEPTON;
            *drive_track = None;
            target.movement_delay = 0;
            return false;
        };
        *sub_cell = Some(sub);
        if let Some(loco) = locomotor {
            let (dest_x, dest_y) = crate::util::lepton::subcell_lepton_offset(Some(sub));
            loco.subcell_dest = Some((dest_x, dest_y));
        }
    } else {
        if let Some(loco) = locomotor {
            loco.subcell_dest = Some((
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
            ));
        }
    }

    true
}
