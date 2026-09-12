//! Entity boundary for active Drive instance installation and retirement.
//!
//! Native SetDestination7425F8 reuses an active Drive, otherwise allocates one
//! (41C250), links it (7426C9), begins piggyback (74276F) and swaps (74277E).
//! Constructor4AF540 initializes its own destination/head/track/speed state.
//! END4AF930 transfers the stashed object; owner742587 / FootAI4DAEC3 then
//! release the displaced instance. Rust's external Drive fields follow that
//! lifetime here. The generic piggyback gate and other class policies stay with
//! their existing owners. These helpers do not mutate Cell occupation.

use super::drive_track::{DriveTrackState, ForcedDriveTrackState};
use super::locomotor::LocomotorState;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::components::DriveLocomotionRuntime;
use crate::sim::game_entity::GameEntity;

fn clear_drive_instance(entity: &mut GameEntity) {
    entity.drive_locomotion = None;
    entity.drive_track = None;
    entity.forced_drive_track = None;
}

pub(crate) fn begin_drive_for_teleporter(entity: &mut GameEntity, binary_frame: u32) -> bool {
    let Some(locomotor) = entity.locomotor.as_mut() else {
        return false;
    };
    let reused_drive = locomotor.active_kind() == LocomotorKind::Drive;
    let accepted = locomotor.begin_drive_piggyback_for_teleporter(binary_frame);
    if accepted && !reused_drive {
        // The incoming Drive is a new object, even if a retired instance left
        // external fields on this entity. A coherent active Drive is reused
        // and retains its head, curve and residual on repeated destinations.
        clear_drive_instance(entity);
    }
    accepted
}

pub(crate) fn try_restore_primary(entity: &mut GameEntity) -> bool {
    let gate = super::locomotor_end_gate_context(entity);
    let admitted = entity.locomotor.as_ref().is_some_and(|locomotor| {
        locomotor.can_restore_primary_from_piggyback(
            gate.owner_moving,
            gate.owner_teleporting,
            gate.owner_deploying,
        )
    });
    admitted && restore_admitted_primary(entity)
}

/// The caller has already evaluated its END gate, at its own required point in
/// the callback sequence. Keep that timing separate from the instance transfer.
pub(crate) fn restore_admitted_primary(entity: &mut GameEntity) -> bool {
    let Some(locomotor) = entity.locomotor.as_mut() else {
        return false;
    };
    let retired_drive = locomotor.active_kind() == LocomotorKind::Drive;
    let restored = locomotor.restore_primary_from_piggyback();
    if restored && retired_drive {
        clear_drive_instance(entity);
    }
    restored
}

/// Rollback for the existing miner command's failed-path transaction. Capture
/// the complete state changed by Drive activation, including the class payload;
/// restoring only kind/layer/phase would leave the newly installed instance's
/// payload behind. This is a Rust command transaction, not a native END call.
pub(crate) struct DriveActivationSnapshot {
    locomotor: Option<LocomotorState>,
    drive: Option<DriveLocomotionRuntime>,
    curve: Option<DriveTrackState>,
    forced: Option<ForcedDriveTrackState>,
}

impl DriveActivationSnapshot {
    pub(crate) fn capture(entity: &GameEntity) -> Self {
        Self {
            locomotor: entity.locomotor.clone(),
            drive: entity.drive_locomotion.clone(),
            curve: entity.drive_track.clone(),
            forced: entity.forced_drive_track.clone(),
        }
    }

    pub(crate) fn restore(self, entity: &mut GameEntity) {
        entity.locomotor = self.locomotor;
        entity.drive_locomotion = self.drive;
        entity.drive_track = self.curve;
        entity.forced_drive_track = self.forced;
    }
}

#[cfg(test)]
#[path = "locomotor_owner_tests.rs"]
mod tests;
