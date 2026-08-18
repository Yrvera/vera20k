//! Evidence-bounded cloak and disguise runtime producers.

use crate::sim::intern::InternedId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpaqueCloakTuple {
    pub start_frame: i32,
    pub payload: i32,
    pub duration_frames: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// RESIDUAL (GSI-12.05) — this whole module is write-dead. No production code
/// ever sets `GameEntity::cloak` or `::disguise`; `apply_transition`,
/// `evaluate_fire_cloak_gates`, `can_open_still_disguise_gate` and
/// `choose_default_mirage_disguise` have only test callers. `Cloakable=` and
/// `CloakingSpeed=` are not parsed anywhere either, despite four stock entries
/// each, so even the inputs are missing.
/// - Trigger: any cloaking or disguising unit — Yuri's stealth armour, the
///   Mirage Tank, a Spy.
/// - Player effect: nothing ever cloaks or disguises. Those units fight as
///   ordinary visible ones, which removes the entire stealth layer of the game.
/// - Frequency: continuous in any match involving those factions or units.
/// - Downstream risk: cloak state gates drawing, targeting legality and the
///   sensor plane recorded as write-dead in `sim/vision/mod.rs`; landing it
///   means landing both together, and it changes what every scan can see, so it
///   moves the pinned replay hash.
pub struct CloakRuntime {
    /// Native state id. States 0..3 deliberately remain opaque.
    pub state: i32,
    /// Presentation phase is separate because the RE contract intentionally
    /// does not assign gameplay names to every raw state id.
    pub visual_phase: Option<CloakVisualPhase>,
    pub depth: u32,
    pub cloaking_stages: u32,
    pub late_visible: bool,
    pub force_visible_call: bool,
    pub opaque_counter1: i32,
    pub opaque_tuple: OpaqueCloakTuple,
    pub opaque_cooldown2: i32,
    pub opaque_mode_flag: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CloakVisualPhase {
    Cloaking,
    FullyCloaked,
    Uncloaking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloakTransitionKind {
    Uncloak,
    Recloak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloakTransitionParams {
    pub kind: CloakTransitionKind,
    pub current_frame: i32,
    pub saved_payload: i32,
    pub rules_counter1_frames: i32,
    pub rules_tuple_duration_frames: i32,
    pub suppress_side_effect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloakTransitionResult {
    pub transitioned: bool,
    pub should_emit_side_effect: bool,
}

impl CloakRuntime {
    /// `FootClass::Uncloak @ 0x00515620` and its adjacent recloak-side helper.
    pub fn apply_transition(&mut self, params: CloakTransitionParams) -> CloakTransitionResult {
        let accepted = match params.kind {
            CloakTransitionKind::Uncloak => matches!(self.state, 1 | 2),
            CloakTransitionKind::Recloak => matches!(self.state, 0 | 3),
        };
        if !accepted {
            return CloakTransitionResult {
                transitioned: false,
                should_emit_side_effect: false,
            };
        }

        self.state = match params.kind {
            CloakTransitionKind::Uncloak => 3,
            CloakTransitionKind::Recloak => 1,
        };
        self.visual_phase = Some(match params.kind {
            CloakTransitionKind::Uncloak => CloakVisualPhase::Uncloaking,
            CloakTransitionKind::Recloak => CloakVisualPhase::Cloaking,
        });
        self.opaque_counter1 = match params.kind {
            CloakTransitionKind::Uncloak => params.rules_counter1_frames - 1,
            CloakTransitionKind::Recloak => 0,
        };
        self.opaque_tuple = OpaqueCloakTuple {
            start_frame: params.current_frame,
            payload: params.saved_payload,
            duration_frames: params.rules_tuple_duration_frames,
        };
        self.opaque_cooldown2 = params.rules_tuple_duration_frames;
        self.opaque_mode_flag = match params.kind {
            CloakTransitionKind::Uncloak => -1,
            CloakTransitionKind::Recloak => 1,
        };
        CloakTransitionResult {
            transitioned: true,
            should_emit_side_effect: !params.suppress_side_effect,
        }
    }

    /// Advance only the renderer-facing depth; raw native state remains owned
    /// by the proved transition edges above.
    pub fn advance_visual_depth(&mut self) {
        match self.visual_phase {
            Some(CloakVisualPhase::Cloaking) => {
                self.depth = self.depth.saturating_add(1).min(self.cloaking_stages);
                if self.depth == self.cloaking_stages {
                    self.visual_phase = Some(CloakVisualPhase::FullyCloaked);
                }
            }
            Some(CloakVisualPhase::Uncloaking) => {
                self.depth = self.depth.saturating_sub(1);
                if self.depth == 0 {
                    self.visual_phase = None;
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct DisguiseRevealTuple {
    pub start_frame: i32,
    pub neighbor_cell_packed: i32,
    pub duration_frames: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct DisguiseRuntime {
    pub disguised: bool,
    pub disguise_creation_frame: u32,
    pub disguise_type: Option<InternedId>,
    pub disguised_as_house: Option<InternedId>,
    pub reveal: DisguiseRevealTuple,
}

impl DisguiseRuntime {
    /// `InfantryClass::DisguiseAs` / `UnitClass::DisguiseAs`.
    pub fn acquire(
        &mut self,
        frame: u32,
        disguise_type: Option<InternedId>,
        house: Option<InternedId>,
    ) {
        self.disguised = true;
        self.disguise_creation_frame = frame;
        self.disguise_type = disguise_type;
        self.disguised_as_house = house;
    }

    /// `TechnoClass::ClearDisguise` clears only the active bit.
    pub fn clear_techno(&mut self) {
        self.disguised = false;
    }

    /// `UnitClass::ClearDisguise` additionally clears type and house.
    pub fn clear_unit(&mut self) {
        self.disguised = false;
        self.disguise_type = None;
        self.disguised_as_house = None;
    }

    pub fn raw_reveal_remaining(&self, current_frame: u32) -> i32 {
        if self.reveal.start_frame == -1 {
            return self.reveal.duration_frames;
        }
        let elapsed = (current_frame as i64 - self.reveal.start_frame as i64).max(0);
        (self.reveal.duration_frames as i64 - elapsed).max(0) as i32
    }

    pub fn arm_idle_reveal(&mut self, frame: u32, packed_cell: i32, duration: i32) {
        self.reveal = DisguiseRevealTuple {
            start_frame: frame as i32,
            neighbor_cell_packed: packed_cell,
            duration_frames: duration,
        };
    }

    /// `TechnoClass::ReceiveDamage @ 0x00701900` reveal tuple writer.
    pub fn arm_damage_reveal(&mut self, frame: u32, packed_cell: i32, applied_damage: i32) {
        self.reveal = DisguiseRevealTuple {
            start_frame: frame as i32,
            neighbor_cell_packed: packed_cell,
            duration_frames: applied_damage.wrapping_shl(1),
        };
    }
}

pub fn can_open_still_disguise_gate(
    blocked_by_self_state: bool,
    blocked_by_linked_object_state: bool,
    disguise_when_still: bool,
    tracked_slot0_present: bool,
) -> bool {
    !blocked_by_self_state
        && !blocked_by_linked_object_state
        && disguise_when_still
        && !tracked_slot0_present
}

pub fn choose_default_mirage_disguise<T: Copy>(pool: &[Option<T>], random_index: i32) -> Option<T> {
    if pool.is_empty() {
        return None;
    }
    let index = random_index.clamp(0, pool.len().saturating_sub(1) as i32) as usize;
    pool[index]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireCloakGateResult {
    pub should_call_reveal_area1: bool,
    pub fire_error_code: Option<u8>,
}

/// `TechnoClass::FireWeaponImpl` / `TechnoClass::GetFireError` closed gate windows.
pub fn evaluate_fire_cloak_gates(
    reveal_on_fire: bool,
    target_house_passes_reveal_check: bool,
    decloak_to_fire: bool,
    current_cloak_state: i32,
    movement_class_id: i32,
) -> FireCloakGateResult {
    let blocked = decloak_to_fire
        && current_cloak_state != 0
        && (movement_class_id != 2 || current_cloak_state == 2);
    FireCloakGateResult {
        should_call_reveal_area1: reveal_on_fire && target_house_passes_reveal_check,
        fire_error_code: blocked.then_some(9),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloak_transition_vectors() {
        let mut state = CloakRuntime {
            state: 1,
            visual_phase: Some(CloakVisualPhase::Cloaking),
            depth: 0,
            cloaking_stages: 9,
            late_visible: false,
            force_visible_call: false,
            opaque_counter1: 0,
            opaque_tuple: OpaqueCloakTuple {
                start_frame: 0,
                payload: 0,
                duration_frames: 0,
            },
            opaque_cooldown2: 0,
            opaque_mode_flag: 0,
        };
        let out = state.apply_transition(CloakTransitionParams {
            kind: CloakTransitionKind::Uncloak,
            current_frame: 1000,
            saved_payload: 4660,
            rules_counter1_frames: 45,
            rules_tuple_duration_frames: 30,
            suppress_side_effect: false,
        });
        assert_eq!(
            out,
            CloakTransitionResult {
                transitioned: true,
                should_emit_side_effect: true
            }
        );
        assert_eq!(
            (state.state, state.opaque_counter1, state.opaque_mode_flag),
            (3, 44, -1)
        );
        assert_eq!(
            state.opaque_tuple,
            OpaqueCloakTuple {
                start_frame: 1000,
                payload: 4660,
                duration_frames: 30
            }
        );
    }

    #[test]
    fn reveal_tuple_and_choice_vectors() {
        let mut state = DisguiseRuntime::default();
        state.reveal = DisguiseRevealTuple {
            start_frame: 100,
            neighbor_cell_packed: 4660,
            duration_frames: 10,
        };
        assert_eq!(state.raw_reveal_remaining(105), 5);
        assert_eq!(state.raw_reveal_remaining(110), 0);
        state.reveal.start_frame = -1;
        state.reveal.duration_frames = 9;
        assert_eq!(state.raw_reveal_remaining(500), 9);
        assert_eq!(
            choose_default_mirage_disguise(&[Some(7), Some(11), Some(13)], 99),
            Some(13)
        );
    }

    #[test]
    fn fire_gate_vectors() {
        assert_eq!(
            evaluate_fire_cloak_gates(true, true, false, 0, 2),
            FireCloakGateResult {
                should_call_reveal_area1: true,
                fire_error_code: None
            }
        );
        assert_eq!(
            evaluate_fire_cloak_gates(true, false, true, 1, 0).fire_error_code,
            Some(9)
        );
        assert_eq!(
            evaluate_fire_cloak_gates(false, true, true, 1, 2).fire_error_code,
            None
        );
        assert_eq!(
            evaluate_fire_cloak_gates(false, true, true, 2, 2).fire_error_code,
            Some(9)
        );
    }

    #[test]
    fn visual_depth_progresses_without_renaming_raw_state() {
        let mut state = CloakRuntime {
            state: 1,
            visual_phase: Some(CloakVisualPhase::Cloaking),
            depth: 1,
            cloaking_stages: 2,
            late_visible: false,
            force_visible_call: false,
            opaque_counter1: 0,
            opaque_tuple: OpaqueCloakTuple {
                start_frame: 0,
                payload: 0,
                duration_frames: 0,
            },
            opaque_cooldown2: 0,
            opaque_mode_flag: 0,
        };
        state.advance_visual_depth();
        assert_eq!(state.state, 1);
        assert_eq!(state.visual_phase, Some(CloakVisualPhase::FullyCloaked));
    }
}
