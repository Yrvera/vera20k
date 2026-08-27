//! Drive/Ship locomotor slope cache and global-frame transition timer.
//!
//! Active `gamemd.exe` stores this state on each Drive/Ship locomotor object:
//! Drive constructor `0x004AF540`, Process `0x004B0500`, force helper
//! `0x004AFB40`, and Draw_Matrix `0x004AFF60`; Ship uses the instruction-for-
//! instruction equivalent constructor `0x0069EC50`, Process `0x0069FC10`,
//! force helper `0x0069F250`, and Draw_Matrix `0x0069F670`. The native timer
//! helpers are `CDTimerClass::Start @ 0x0046B640` and `Remaining @ 0x004B4D70`.

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotion::{LocomotorRuntime, LocomotorRuntimePayload};
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};

/// Literal Drive/Ship slope interpolation duration installed by both native
/// `Process` implementations.
pub(crate) const SLOPE_TRANSITION_FRAMES: u8 = 3;

/// Defined, persistent state owned by one active or stashed Drive/Ship
/// locomotor. The native unused timer dword is deliberately not represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SlopeTransitionState {
    previous_slope: u8,
    current_slope: u8,
    start_frame: i32,
    transition_total: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlopeRenderPhase {
    Stable(u8),
    Transition {
        from_slope: u8,
        to_slope: u8,
        phase_num: i32,
        phase_den: u8,
    },
}

impl SlopeTransitionState {
    /// Drive/Ship constructor defaults use the current signed global frame.
    pub(crate) fn at_binary_frame(binary_frame: u32) -> Self {
        Self {
            previous_slope: 0,
            current_slope: 0,
            start_frame: binary_frame as i32,
            transition_total: 0,
        }
    }

    /// `Force_Slope`/successful Foot unlimbo synchronizes immediately.
    pub(crate) fn snap(&mut self, sampled_slope: u8, binary_frame: u32) {
        self.previous_slope = sampled_slope;
        self.current_slope = sampled_slope;
        self.start_frame = binary_frame as i32;
        self.transition_total = 0;
    }

    /// Drive/Ship `Process` entry starts a literal three-frame timer only when
    /// the current containing-cell slope differs from the cached target.
    pub(crate) fn sample_process_entry(&mut self, sampled_slope: u8, binary_frame: u32) {
        if sampled_slope == self.current_slope {
            return;
        }
        self.previous_slope = self.current_slope;
        self.current_slope = sampled_slope;
        self.start_frame = binary_frame as i32;
        self.transition_total = SLOPE_TRANSITION_FRAMES;
    }

    pub(crate) fn remaining(&self, binary_frame: u32) -> i32 {
        if self.transition_total == 0 {
            return 0;
        }
        if self.start_frame == -1 {
            return i32::from(self.transition_total);
        }
        let elapsed = (binary_frame as i32).wrapping_sub(self.start_frame);
        if elapsed < i32::from(self.transition_total) {
            i32::from(self.transition_total).wrapping_sub(elapsed)
        } else {
            0
        }
    }

    pub(crate) fn render_phase(&self, binary_frame: u32) -> SlopeRenderPhase {
        let remaining = self.remaining(binary_frame);
        if self.transition_total == 0 || remaining == 0 {
            return SlopeRenderPhase::Stable(self.current_slope);
        }
        let phase_num = i32::from(self.transition_total).wrapping_sub(remaining);
        if phase_num >= i32::from(self.transition_total) {
            SlopeRenderPhase::Stable(self.current_slope)
        } else {
            SlopeRenderPhase::Transition {
                from_slope: self.previous_slope,
                to_slope: self.current_slope,
                phase_num,
                phase_den: self.transition_total,
            }
        }
    }

    pub(crate) fn hash_fields(&self) -> (u8, u8, i32, u8) {
        (
            self.previous_slope,
            self.current_slope,
            self.start_frame,
            self.transition_total,
        )
    }

    #[cfg(test)]
    pub(crate) fn from_fields_for_test(
        previous_slope: u8,
        current_slope: u8,
        start_frame: i32,
        transition_total: u8,
    ) -> Self {
        Self {
            previous_slope,
            current_slope,
            start_frame,
            transition_total,
        }
    }
}

fn foot_equivalent(category: EntityCategory) -> bool {
    matches!(
        category,
        EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
    )
}

pub(crate) fn state_for_entity(entity: &GameEntity) -> Option<&SlopeTransitionState> {
    foot_equivalent(entity.category)
        .then_some(())
        .and_then(|()| entity.locomotor.as_ref()?.active_slope_transition())
}

fn state_for_entity_mut(entity: &mut GameEntity) -> Option<&mut SlopeTransitionState> {
    foot_equivalent(entity.category)
        .then_some(())
        .and_then(|()| entity.locomotor.as_mut()?.active_slope_transition_mut())
}

/// FootClass::Unlimbo @ `0x004D7170` dispatches the active locomotor force-
/// slope slot at `0x004D71A9` only after TechnoClass::Unlimbo succeeds.
pub(crate) fn snap_after_successful_unlimbo(
    entity: &mut GameEntity,
    sampled_slope: u8,
    binary_frame: u32,
) {
    if let Some(state) = state_for_entity_mut(entity) {
        state.snap(sampled_slope, binary_frame);
    }
}

/// Drive Process `0x004B050B..0x004B0557` and Ship Process
/// `0x0069FC1B..0x0069FC67` sample before any track/movement branch.
pub(crate) fn sample_process_entry(
    entity: &mut GameEntity,
    sampled_slope: u8,
    binary_frame: u32,
) {
    if let Some(state) = state_for_entity_mut(entity) {
        state.sample_process_entry(sampled_slope, binary_frame);
    }
}

/// Proof captured from the complete pre-restore locomotor objects. Its private
/// runtime cannot be caller-asserted: it exists only while an active ground
/// Tunnel owns a suspended Drive with its class-local slope payload.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct TunnelDriveRestoreToken {
    restored_runtime: LocomotorRuntime,
}

#[allow(dead_code)]
pub(crate) fn tunnel_drive_restore_token(
    locomotor: &LocomotorState,
) -> Option<TunnelDriveRestoreToken> {
    if locomotor.active_kind() != LocomotorKind::Tunnel
        || locomotor.layer != MovementLayer::Ground
        || !matches!(
            &locomotor.runtime_payload,
            LocomotorRuntimePayload::Tunnel(_)
        )
    {
        return None;
    }
    let stashed = locomotor.piggyback.as_deref()?;
    if stashed.kind != LocomotorKind::Drive
        || !matches!(&stashed.payload, LocomotorRuntimePayload::Drive(_))
    {
        return None;
    }
    Some(TunnelDriveRestoreToken {
        restored_runtime: stashed.clone(),
    })
}

/// `TechnoClass::Set_Destination @ 0x00741970` has one extra force-slope call
/// at `0x00742BE3`, confined to ground-layer Tunnel/piggyback restoration.
/// Rust has no active stock Tunnel caller; keeping this predicate explicit
/// prevents a future restoration bridge from becoming a generic move snap.
#[allow(dead_code)]
pub(crate) fn snap_after_tunnel_piggyback_restore(
    entity: &mut GameEntity,
    token: TunnelDriveRestoreToken,
    sampled_slope: u8,
    binary_frame: u32,
) {
    if !foot_equivalent(entity.category) {
        return;
    }
    let Some(locomotor) = entity.locomotor.as_mut() else {
        return;
    };
    if LocomotorRuntime::capture(locomotor) != token.restored_runtime
        || locomotor.active_kind() != LocomotorKind::Drive
    {
        return;
    }
    if let LocomotorRuntimePayload::Drive(state) = &mut locomotor.runtime_payload {
        state.snap(sampled_slope, binary_frame);
    }
}

#[cfg(test)]
mod tests {
    use super::{SlopeRenderPhase, SlopeTransitionState};
    use crate::map::entities::EntityCategory;
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotion::LocomotorRuntimePayload;
    use crate::sim::movement::locomotor::LocomotorState;

    #[test]
    fn slope_timer_preserves_signed_minus_one_and_negative_phase() {
        assert_eq!(super::SLOPE_TRANSITION_FRAMES, 3);
        let mut state = SlopeTransitionState::at_binary_frame(0);
        state.previous_slope = 4;
        state.current_slope = 9;
        state.start_frame = 0;
        state.transition_total = super::SLOPE_TRANSITION_FRAMES;

        assert_eq!(
            state.render_phase(u32::MAX),
            SlopeRenderPhase::Transition {
                from_slope: 4,
                to_slope: 9,
                phase_num: -1,
                phase_den: super::SLOPE_TRANSITION_FRAMES,
            }
        );
        assert_eq!(state.remaining(0xffff_feff), 260);
        assert_eq!(
            state.render_phase(0xffff_feff),
            SlopeRenderPhase::Transition {
                from_slope: 4,
                to_slope: 9,
                phase_num: -257,
                phase_den: super::SLOPE_TRANSITION_FRAMES,
            }
        );

        state.start_frame = -1;
        assert_eq!(state.remaining(0), 3);
        assert_eq!(state.remaining(u32::MAX), 3);
    }

    #[test]
    fn slope_timer_uses_signed_wrapping_frame_subtraction() {
        let mut state = SlopeTransitionState::at_binary_frame(0x7fff_fffe);
        state.sample_process_entry(7, 0x7fff_ffff);
        assert_eq!(state.remaining(0x8000_0000), 2);
        assert_eq!(state.remaining(0x8000_0001), 1);
        assert_eq!(state.remaining(0x8000_0002), 0);
    }

    #[test]
    fn equal_process_sample_is_a_complete_no_write() {
        let mut state = SlopeTransitionState::at_binary_frame(10);
        state.sample_process_entry(6, 10);
        let before = state;
        state.sample_process_entry(6, 99);
        assert_eq!(state, before);
    }

    #[test]
    fn drive_ship_slope_three_frame_ledger_and_mid_transition_retarget_are_exact() {
        let mut state = SlopeTransitionState::at_binary_frame(8);
        state.snap(2, 8);
        state.sample_process_entry(7, 10);
        assert_eq!(
            state.render_phase(10),
            SlopeRenderPhase::Transition {
                from_slope: 2,
                to_slope: 7,
                phase_num: 0,
                phase_den: super::SLOPE_TRANSITION_FRAMES,
            }
        );
        assert_eq!(state.remaining(11), 2);
        assert_eq!(state.remaining(12), 1);
        assert_eq!(state.render_phase(13), SlopeRenderPhase::Stable(7));

        state.sample_process_entry(12, 11);
        assert_eq!(state.hash_fields(), (7, 12, 11, 3));
        assert_eq!(
            state.render_phase(11),
            SlopeRenderPhase::Transition {
                from_slope: 7,
                to_slope: 12,
                phase_num: 0,
                phase_den: super::SLOPE_TRANSITION_FRAMES,
            }
        );
    }

    fn entity_with(category: EntityCategory, kind: LocomotorKind) -> GameEntity {
        let mut entity = GameEntity::test_default(1, "SLOPE", "Americans", 2, 2);
        entity.category = category;
        entity.locomotor = Some(LocomotorState::for_test_kind(kind));
        entity
    }

    #[test]
    fn slope_ownership_is_foot_plus_matching_active_drive_ship_only() {
        for category in [
            EntityCategory::Unit,
            EntityCategory::Infantry,
            EntityCategory::Aircraft,
        ] {
            assert!(super::state_for_entity(&entity_with(category, LocomotorKind::Drive)).is_some());
            assert!(super::state_for_entity(&entity_with(category, LocomotorKind::Ship)).is_some());
        }
        assert!(
            super::state_for_entity(&entity_with(
                EntityCategory::Structure,
                LocomotorKind::Drive
            ))
            .is_none()
        );
        for kind in [
            LocomotorKind::Walk,
            LocomotorKind::Hover,
            LocomotorKind::Fly,
            LocomotorKind::Jumpjet,
            LocomotorKind::Rocket,
            LocomotorKind::Teleport,
            LocomotorKind::Tunnel,
        ] {
            assert!(super::state_for_entity(&entity_with(EntityCategory::Unit, kind)).is_none());
        }

        let mut mismatched = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        mismatched.locomotor.as_mut().unwrap().runtime_payload = LocomotorRuntimePayload::Walk;
        assert!(super::state_for_entity(&mismatched).is_none());
    }

    #[test]
    fn active_drive_piggyback_is_eligible_while_stashed_drive_is_not_active() {
        let constructor = LocomotorState::for_test_kind_at_frame(LocomotorKind::Drive, 17);
        assert_eq!(
            constructor
                .active_slope_transition()
                .unwrap()
                .hash_fields(),
            (0, 0, 17, 0),
            "ordinary construction retains the live nonzero frame"
        );

        let mut active_drive = entity_with(EntityCategory::Unit, LocomotorKind::Teleport);
        assert!(
            active_drive
                .locomotor
                .as_mut()
                .unwrap()
                .begin_drive_piggyback_for_teleporter(22)
        );
        assert!(super::state_for_entity(&active_drive).is_some());
        assert_eq!(
            super::state_for_entity(&active_drive).unwrap().hash_fields(),
            (0, 0, 22, 0),
            "fresh Drive replacement retains the live nonzero frame"
        );
        assert_eq!(
            active_drive.locomotor.as_ref().unwrap().effective_kind(),
            LocomotorKind::Teleport
        );

        let mut stashed_drive = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        assert!(stashed_drive.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Teleport,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            23,
        ));
        assert!(super::state_for_entity(&stashed_drive).is_none());
        assert!(matches!(
            stashed_drive
                .locomotor
                .as_ref()
                .unwrap()
                .piggyback
                .as_deref()
                .map(|runtime| &runtime.payload),
            Some(LocomotorRuntimePayload::Drive(_))
        ));
        assert!(stashed_drive.locomotor.as_mut().unwrap().end_piggyback());
        assert_eq!(
            super::state_for_entity(&stashed_drive).unwrap().hash_fields(),
            (0, 0, 0, 0),
            "generic piggyback restore does not invent a terrain snap"
        );
    }

    #[test]
    fn only_ground_tunnel_piggyback_restore_uses_the_extra_force_slope_gate() {
        use crate::sim::movement::locomotor::MovementLayer;

        let mut exact = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        assert!(exact.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Tunnel,
            MovementLayer::Ground,
            22,
        ));
        let token = super::tunnel_drive_restore_token(exact.locomotor.as_ref().unwrap())
            .expect("active ground Tunnel with a typed Drive stash");
        assert_eq!(
            exact.locomotor.as_ref().unwrap().active_kind(),
            LocomotorKind::Tunnel
        );
        assert!(exact.locomotor.as_mut().unwrap().end_piggyback());
        assert_eq!(
            exact.locomotor.as_ref().unwrap().active_kind(),
            LocomotorKind::Drive,
            "the test performs the real complete-runtime restore before snapping"
        );
        super::snap_after_tunnel_piggyback_restore(&mut exact, token, 9, 30);
        assert_eq!(
            super::state_for_entity(&exact).unwrap().hash_fields(),
            (9, 9, 30, 0)
        );

        let mut wrong_layer = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        assert!(wrong_layer.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Tunnel,
            MovementLayer::Air,
            22,
        ));
        assert!(
            super::tunnel_drive_restore_token(wrong_layer.locomotor.as_ref().unwrap()).is_none(),
            "an air-layer Tunnel cannot mint the proof"
        );

        let missing_stash = entity_with(EntityCategory::Unit, LocomotorKind::Tunnel);
        assert!(
            super::tunnel_drive_restore_token(missing_stash.locomotor.as_ref().unwrap()).is_none(),
            "an active Tunnel without a suspended runtime cannot mint the proof"
        );

        let mut wrong_stash = entity_with(EntityCategory::Unit, LocomotorKind::Ship);
        assert!(wrong_stash.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Tunnel,
            MovementLayer::Ground,
            22,
        ));
        assert!(
            super::tunnel_drive_restore_token(wrong_stash.locomotor.as_ref().unwrap()).is_none(),
            "a suspended Ship is not a typed Drive slope runtime"
        );

        let mut not_restored = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        assert!(not_restored.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Tunnel,
            MovementLayer::Ground,
            22,
        ));
        let token = super::tunnel_drive_restore_token(not_restored.locomotor.as_ref().unwrap())
            .expect("exact pre-restore proof");
        super::snap_after_tunnel_piggyback_restore(&mut not_restored, token, 9, 30);
        assert_eq!(
            not_restored.locomotor.as_ref().unwrap().active_kind(),
            LocomotorKind::Tunnel
        );
        assert!(not_restored.locomotor.as_ref().unwrap().piggyback.is_some());
        assert!(not_restored.locomotor.as_mut().unwrap().end_piggyback());
        assert_eq!(
            super::state_for_entity(&not_restored)
                .unwrap()
                .hash_fields(),
            (0, 0, 0, 0),
            "the proof cannot snap the suspended Drive before it is restored"
        );

        let mut wrong_restore = entity_with(EntityCategory::Unit, LocomotorKind::Drive);
        assert!(wrong_restore.locomotor.as_mut().unwrap().begin_piggyback(
            LocomotorKind::Tunnel,
            MovementLayer::Ground,
            22,
        ));
        let token = super::tunnel_drive_restore_token(wrong_restore.locomotor.as_ref().unwrap())
            .expect("exact pre-restore proof");
        assert!(wrong_restore.locomotor.as_mut().unwrap().end_piggyback());
        wrong_restore.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Ship));
        super::snap_after_tunnel_piggyback_restore(&mut wrong_restore, token, 9, 30);
        assert_eq!(
            super::state_for_entity(&wrong_restore).unwrap().hash_fields(),
            (0, 0, 0, 0),
            "the token cannot snap a different now-active restored kind/runtime"
        );
    }
}
