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
use crate::sim::components::Position;
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::occupancy::OccupancyGrid;

/// Commit the arrival slot. Infallible, like the native arrival branch — the
/// old `bool` return existed only for the failure path removed below.
pub(super) fn reserve_destination_after_transition(
    category: EntityCategory,
    entity_id: u64,
    locomotor: &mut Option<LocomotorState>,
    position: &mut Position,
    sub_cell: &mut Option<u8>,
    next_layer: MovementLayer,
    nx: u16,
    ny: u16,
    occupancy: &OccupancyGrid,
    priority: bool,
) {
    if category == EntityCategory::Infantry {
        // The slot the look-ahead reserved for this cell, recovered from the
        // lepton destination it stored on the locomotor. Functional by
        // construction, which is what makes it a usable arrival fallback.
        let preferred = locomotor
            .as_ref()
            .and_then(|loco| loco.subcell_dest)
            .and_then(bump_crush::functional_sub_cell_from_offset);
        // Priority placement bypasses every occupancy and blocker gate, exactly
        // as the original engine's priority branch does.
        let claimed = if priority {
            Some(bump_crush::priority_sub_cell(
                position.sub_x,
                position.sub_y,
            ))
        } else {
            bump_crush::claim_reserved_sub_cell(
                occupancy.get(nx, ny),
                next_layer,
                entity_id,
                preferred,
            )
        };
        // **Arrival cannot fail.** `WalkLocomotionClass::ProcessMovement` @
        // `0x0075BE0A` hands `FindSubCellDest` @ `0x0075C240` a NullCoord; that
        // stores it into `+0x28..0x30` and jumps to `LAB_0075C5C5`, which
        // reloads the infantryman's own `+0x9C` coordinate and marks it through
        // Infantry `vtable+0xF0` before `XOR AL,AL; RET 4`. The caller never
        // reads the result — `0x0075BE1D` goes straight on to `[+0x5E0]`. So the
        // slot is *derived from the man's own leptons*, nothing is scanned, and
        // there is no refusal.
        //
        // VERA moves the entity in the occupancy grid before this runs and then
        // picks a slot, so `claim_reserved_sub_cell` can come back empty when a
        // vehicle settles on the cell or three other infantry hold the
        // functional slots. Falling back to the man's own lepton offset is the
        // native answer to exactly that: it keeps the arrival total, as
        // `LAB_0075C5C5` does. VERA falls back to the look-ahead's own slot
        // (functional by construction) and, failing that, the first
        // functional slot — a VERA-internal choice of *which* slot, since the
        // native derives it from the man's leptons rather than from a stored
        // reservation. gamemd equivalent of the ordering UNCHECKED.
        //
        // The previous behaviour — snap to cell centre, drop the drive track and
        // return `false` — broke the crossing loop before
        // `configure_motion_after_transition` could advance `next_index`, so the
        // next tick re-read the cell the man was already standing in, re-ran the
        // transition and failed again, with no `aborted_for_stuck` and no route
        // through `movement_blocked`. That is a permanent freeze until the
        // contention clears on its own, and the cell centre it snapped to is a
        // position the ordinary chooser can never assign.
        let sub = claimed
            .or(preferred)
            .unwrap_or(bump_crush::FUNCTIONAL_SUB_CELLS[0]);
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
}
