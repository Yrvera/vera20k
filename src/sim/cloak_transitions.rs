//! Native cloak transition entry points and their arg-sensitive sound edge.

use super::{CloakRuntime, CloakStepTimer, CloakVisualPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartCloakingResult {
    pub transitioned: bool,
    pub play_sound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartUncloakingResult {
    pub transitioned: bool,
    pub play_sound: bool,
}

impl CloakRuntime {
    /// `TechnoClass::StartCloaking @ 0x00703770`. Native accepts states zero
    /// and three, performs the state/timer writes first, then plays the exact
    /// current coordinate only when the boolean sound-suppression argument is
    /// zero.
    pub(super) fn start_cloaking(
        &mut self,
        now: i32,
        speed: i32,
        suppress_sound: bool,
    ) -> StartCloakingResult {
        if !matches!(self.state, 0 | 3) {
            return StartCloakingResult {
                transitioned: false,
                play_sound: false,
            };
        }
        self.state = 1;
        self.visual_phase = Some(CloakVisualPhase::Cloaking);
        self.depth = 0;
        self.step_delta = 1;
        self.step_timer = CloakStepTimer {
            start_frame: now,
            speed,
            duration_frames: speed,
        };
        StartCloakingResult {
            transitioned: true,
            play_sound: !suppress_sound,
        }
    }

    /// `TechnoClass::StartUncloaking @ 0x007036C0`. Native's boolean argument
    /// is a sound-suppression flag: zero plays RulesClass+0x6A0 through
    /// `VocClass::PlayAt @ 0x007509E0`, one performs only the state writes.
    pub(super) fn start_uncloaking(
        &mut self,
        now: i32,
        speed: i32,
        suppress_sound: bool,
    ) -> StartUncloakingResult {
        if !matches!(self.state, 1 | 2) {
            return StartUncloakingResult {
                transitioned: false,
                play_sound: false,
            };
        }
        self.state = 3;
        self.visual_phase = Some(CloakVisualPhase::Uncloaking);
        self.depth = self.cloaking_stages.saturating_sub(1);
        self.step_delta = -1;
        self.step_timer = CloakStepTimer {
            start_frame: now,
            speed,
            duration_frames: speed,
        };
        StartUncloakingResult {
            transitioned: true,
            play_sound: !suppress_sound,
        }
    }

    /// Virtual `StartCloaking +0x460 @ 0x00703770` reached from the active
    /// sensor-count resident callback `0x006F4EB0` with arg zero.
    pub(crate) fn start_cloaking_from_sensor(
        &mut self,
        now: i32,
        speed: i32,
    ) -> StartCloakingResult {
        self.start_cloaking(now, speed, false)
    }

    /// `TechnoClass::ReceiveDamage @ 0x0070281D` invokes virtual `+0xFC`
    /// (`0x00703850`, a plain `StartUncloaking(0)` wrapper) for every damage
    /// result that is not NowDead — the heal case included, because the
    /// `if (damage < 0) return` early-out sits AFTER this call. Argument zero,
    /// so the transition owns a `CloakSound`.
    pub(crate) fn start_uncloaking_from_damage(
        &mut self,
        now: i32,
        speed: i32,
    ) -> StartUncloakingResult {
        self.start_uncloaking(now, speed, false)
    }

    /// `FootClass::PerCellProcess @ 0x004D8829` invokes the same `+0xFC`
    /// wrapper when a fully cloaked mover enters a cell one of whose eight
    /// neighbours holds a non-allied `Sensors=yes` object.
    pub(crate) fn start_uncloaking_from_sensor_neighbour(
        &mut self,
        now: i32,
        speed: i32,
    ) -> StartUncloakingResult {
        self.start_uncloaking(now, speed, false)
    }

    /// `CellClass::Mark_Objects_Redraw @ 0x00483480` — misnamed; the body walks
    /// the cell's `FirstObject(+0xE4)` chain calling `+0xFC` on every resident.
    /// Every caller is a locomotor sitting on the branch where
    /// `Can_Enter_Cell` returned 1 for a cloaked occupant
    /// (`DriveLocomotionClass::Process_Movement @ 0x004B395E`, `0x004B445B`;
    /// `ShipLocomotionClass::Process_Movement @ 0x006A2FAD`, `0x006A3A87`;
    /// `WalkLocomotionClass::ProcessMovement @ 0x0075BBAE`; the two
    /// `Process_Drive_Track` sites `0x004B1E63` / `0x006A14A6`).
    pub(crate) fn start_uncloaking_from_mover_bump(
        &mut self,
        now: i32,
        speed: i32,
    ) -> StartUncloakingResult {
        self.start_uncloaking(now, speed, false)
    }

    /// `UnitClass::Fire_At_Target @ 0x00736DF0` case 9 invokes virtual
    /// `StartUncloaking +0x45C @ 0x007036C0` after rechecking CanFireAt.
    pub(crate) fn start_uncloaking_to_fire(
        &mut self,
        now: i32,
        speed: i32,
    ) -> StartUncloakingResult {
        self.start_uncloaking(now, speed, false)
    }
}
