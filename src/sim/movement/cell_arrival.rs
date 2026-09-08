//! Complete accepted cell arrivals for ordinary crossings and Drive track jumps.
//!
//! VERA-internal ownership boundary, gamemd equivalent UNCHECKED. Preserve the
//! represented crossing order: fresh serialized list stamp, list relink,
//! Drive current occupation, arrival claim and matching Infantry list repair.
//! Geometry, path advancement, bridge rendering and look-ahead placement remain
//! in their existing caller phases. Arrival consumes no RNG: WalkLocomotion
//! ProcessMovement @ 0x0075BE0A reaches the NullCoord FindSubCellDest branch
//! @ 0x0075C240; see the detailed arrival evidence beside the private claim.

use crate::map::entities::EntityCategory;
use crate::sim::components::{DriveLocomotionRuntime, Position};
use crate::sim::movement::bump_crush;
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::occupancy::{CellListInsertion, CellOccupationGrid, OccupancyGrid};
use crate::sim::world::EnterOrderCounter;

use super::MovementTickStats;

/// Borrow only the projections an accepted arrival must update together.
/// Position and old/new list layers have already been resolved by the caller;
/// the path layer is separate because bridge predicates need not agree with it.
pub(super) struct CellArrival<'a> {
    pub entity_id: u64,
    pub category: EntityCategory,
    pub from: (u16, u16),
    pub to: (u16, u16),
    pub old_list_layer: MovementLayer,
    pub new_list_layer: MovementLayer,
    pub position: &'a Position,
    pub locomotor: &'a mut Option<LocomotorState>,
    pub drive_locomotion: &'a mut Option<DriveLocomotionRuntime>,
    pub sub_cell: &'a mut Option<u8>,
    pub occupancy_enter_order: &'a mut u64,
    pub next_occupancy_enter_order: &'a mut EnterOrderCounter,
    pub occupancy: &'a mut OccupancyGrid,
    pub cell_occupation: &'a mut CellOccupationGrid,
    pub stats: &'a mut MovementTickStats,
    pub priority: bool,
}

impl CellArrival<'_> {
    /// Ordinary crossings commit their path layer after current occupation.
    pub(super) fn ordinary(mut self, next_layer: MovementLayer) {
        self.relink();
        if let Some(loco) = self.locomotor.as_mut() {
            loco.layer = next_layer;
        }
        self.finish(next_layer);
    }

    /// Track jumps have already committed the path layer during bridge
    /// resolution, before publishing the cell-list transition.
    pub(super) fn track_jump(mut self, active_layer: MovementLayer) {
        self.relink();
        self.finish(active_layer);
    }

    fn relink(&mut self) {
        *self.occupancy_enter_order = self.next_occupancy_enter_order.next();
        self.occupancy.move_entity_layered(
            self.from.0,
            self.from.1,
            self.to.0,
            self.to.1,
            self.entity_id,
            self.old_list_layer,
            self.new_list_layer,
            *self.sub_cell,
            CellListInsertion::from_category(self.category),
        );
        if self.category == EntityCategory::Unit
            && let Some(drive) = self.drive_locomotion.as_mut()
        {
            crate::sim::occupancy::mark_current_drive_occupation_after_crossing(
                drive,
                self.cell_occupation,
                self.entity_id,
                self.to,
                self.new_list_layer,
            );
        }
    }

    fn finish(self, path_layer: MovementLayer) {
        reserve_destination_after_transition(
            self.category,
            self.entity_id,
            self.locomotor,
            self.position,
            self.sub_cell,
            path_layer,
            self.to.0,
            self.to.1,
            self.occupancy,
            self.priority,
        );
        // Claim and list correction are one operation: callers cannot publish
        // a new Infantry sub_cell while leaving its cell-list entry stale.
        if self.category == EntityCategory::Infantry {
            self.occupancy
                .update_sub_cell(self.to.0, self.to.1, self.entity_id, *self.sub_cell);
        }
        self.stats.moved_steps = self.stats.moved_steps.saturating_add(1);
    }
}

/// Commit the arrival slot. Infallible, like the native arrival branch — the
/// old `bool` return existed only for the failure path removed below.
fn reserve_destination_after_transition(
    category: EntityCategory,
    entity_id: u64,
    locomotor: &mut Option<LocomotorState>,
    position: &Position,
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
