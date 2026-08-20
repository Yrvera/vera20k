//! Evidence-bounded cloak and disguise runtime producers.

use crate::sim::intern::InternedId;
use crate::sim::rng::SimRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloakStepTimer {
    pub start_frame: i32,
    pub speed: i32,
    pub duration_frames: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Active `TechnoClass` cloak fields consumed by radar and tactical draw.
///
/// `CloakingTick @ 0x006FB740`, `StartCloaking @ 0x00703770`,
/// `StartUncloaking @ 0x007036C0`, and `GetVisualState @ 0x00703860` establish
/// every state/progress/timer write below. Stock YR exercises this continuously
/// through DLPH/SUB/SQD/BSUB.
pub struct CloakRuntime {
    /// Native state id: 0 uncloaked, 1 cloaking, 2 fully cloaked, 3 uncloaking.
    pub state: i32,
    pub visual_phase: Option<CloakVisualPhase>,
    /// Native `CloakProgress +0x224`.
    pub depth: u32,
    pub cloaking_stages: u32,
    pub late_visible: bool,
    pub force_visible_call: bool,
    /// Native signed progress delta, +1 cloaking and -1 uncloaking.
    pub step_delta: i32,
    pub step_timer: CloakStepTimer,
    /// `ReCloakDelayTimer +0x2EC/+0x2F4`.
    pub recloak_delay_start: i32,
    pub recloak_delay_frames: i32,
    /// Independent `CanAutoCloak` gate timer at +0x240/+0x248.
    pub secondary_gate_start: i32,
    pub secondary_gate_frames: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CloakVisualPhase {
    Cloaking,
    FullyCloaked,
    Uncloaking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloakTickFacts {
    pub current_frame: i32,
    /// Head gate at 0x006FB740: usable intrinsic cloak with no firing/chrono
    /// activity, or the current rank's CLOAK ability.
    pub state_zero_head_allows: bool,
    /// Exact `CanAutoCloak @ 0x006FBDC0` result from current world facts.
    pub can_auto_cloak: bool,
    /// Exact `ShouldUncloak @ 0x006FBC90` result from current world facts.
    pub should_uncloak: bool,
    /// Strict `ConditionRed < health_ratio` comparison.
    pub health_above_red: bool,
    pub cloaking_speed: i32,
    pub cloak_delay_frames: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloakTickResult {
    pub transitioned: bool,
    pub consumed_scenario_rng: bool,
    /// An accepted `StartUncloaking(0)` transition owns one positional
    /// `[AudioVisual] CloakSound` request. Silent arg-one transitions and
    /// rejected state visits leave this clear.
    pub play_uncloak_sound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartUncloakingResult {
    pub transitioned: bool,
    pub play_sound: bool,
}

impl CloakRuntime {
    pub fn new(current_frame: i32, cloaking_stages: i32) -> Self {
        Self {
            state: 0,
            visual_phase: None,
            depth: 0,
            cloaking_stages: cloaking_stages.max(1) as u32,
            late_visible: false,
            force_visible_call: false,
            step_delta: 0,
            step_timer: CloakStepTimer {
                start_frame: current_frame,
                speed: 0,
                duration_frames: 0,
            },
            recloak_delay_start: current_frame,
            recloak_delay_frames: 0,
            secondary_gate_start: current_frame,
            secondary_gate_frames: 0,
        }
    }

    /// UnitClass::Unlimbo @ 0x00737BA0 sets only state 2 when runtime cloak
    /// ability is present and stored Techno+0x3D5 is clear.
    pub fn establish_unlimbo_fully_cloaked(&mut self) {
        self.state = 2;
        self.visual_phase = Some(CloakVisualPhase::FullyCloaked);
    }

    fn timer_remaining(start: i32, duration: i32, now: i32) -> i32 {
        if duration <= 0 {
            return 0;
        }
        if start == -1 {
            return duration;
        }
        duration.saturating_sub(now.wrapping_sub(start)).max(0)
    }

    fn start_cloaking(&mut self, now: i32, speed: i32) -> bool {
        if !matches!(self.state, 0 | 3) {
            return false;
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
        true
    }

    /// `TechnoClass::StartUncloaking @ 0x007036C0`. Native's boolean argument
    /// is a sound-suppression flag: zero plays RulesClass+0x6A0 through
    /// `VocClass::PlayAt @ 0x007509E0`, one performs only the state writes.
    fn start_uncloaking(
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
    /// sensor-count resident callback `0x006F4EB0`.
    pub(crate) fn start_cloaking_from_sensor(&mut self, now: i32, speed: i32) -> bool {
        self.start_cloaking(now, speed)
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

    fn advance_due_step(&mut self, now: i32) {
        if self.step_timer.speed == 0
            || Self::timer_remaining(
                self.step_timer.start_frame,
                self.step_timer.duration_frames,
                now,
            ) != 0
        {
            return;
        }
        self.depth = if self.step_delta < 0 {
            self.depth.saturating_sub(self.step_delta.unsigned_abs())
        } else {
            self.depth.wrapping_add(self.step_delta as u32)
        };
        self.step_timer.start_frame = now;
        self.step_timer.duration_frames = self.step_timer.speed;
    }

    /// `GetVisualState @ 0x00703860` transition ladder. For the non-negative
    /// native progress domain, integer division is output-equivalent to the
    /// x87 divide/multiply followed by truncation toward zero.
    pub fn transition_visual_state(&self) -> u8 {
        if self.depth == 0 {
            return 0;
        }
        let scaled = (u64::from(self.depth) * 256 / u64::from(self.cloaking_stages.max(1)))
            .min(i32::MAX as u64) as i32;
        match scaled {
            ..=0x3f => 1,
            0x40..=0x7f => 2,
            0x80..=0xbf => 3,
            0xc0..=0xfe => 4,
            _ => 5,
        }
    }

    /// Active state machine from `TechnoClass::CloakingTick @ 0x006FB740`.
    pub fn tick(&mut self, facts: CloakTickFacts, rng: &mut SimRng) -> CloakTickResult {
        let mut result = CloakTickResult {
            transitioned: false,
            consumed_scenario_rng: false,
            play_uncloak_sound: false,
        };
        if self.state == 0 {
            if !facts.state_zero_head_allows || !facts.can_auto_cloak {
                return result;
            }
            let start = if facts.health_above_red {
                true
            } else {
                result.consumed_scenario_rng = true;
                rng.next_range_u32_inclusive(0, 99) < 4
            };
            if start {
                result.transitioned = self.start_cloaking(
                    facts.current_frame,
                    facts.cloaking_speed,
                );
            }
            return result;
        }

        self.advance_due_step(facts.current_frame);
        match self.state {
            1 => {
                if self.step_timer.speed == 0 {
                    self.step_timer = CloakStepTimer {
                        start_frame: 1,
                        speed: 1,
                        duration_frames: 1,
                    };
                }
                match self.transition_visual_state() {
                    2 if !facts.health_above_red => {
                        result.consumed_scenario_rng = true;
                        if rng.next_range_u32_inclusive(0, 99) <= 9 {
                            let start = self.start_uncloaking(
                                facts.current_frame,
                                facts.cloaking_speed,
                                true,
                            );
                            result.transitioned = start.transitioned;
                            result.play_uncloak_sound = start.play_sound;
                        }
                    }
                    3 | 5 => {
                        self.state = 2;
                        self.visual_phase = Some(CloakVisualPhase::FullyCloaked);
                        self.depth = 0;
                        self.step_delta = 0;
                        self.step_timer = CloakStepTimer {
                            start_frame: facts.current_frame,
                            speed: 0,
                            duration_frames: 0,
                        };
                        result.transitioned = true;
                    }
                    _ => {}
                }
            }
            2 if facts.should_uncloak => {
                let start = self.start_uncloaking(
                    facts.current_frame,
                    facts.cloaking_speed,
                    false,
                );
                result.transitioned = start.transitioned;
                result.play_uncloak_sound = start.play_sound;
            }
            3 => match self.transition_visual_state() {
                0 => {
                    self.state = 0;
                    self.visual_phase = None;
                    self.depth = 0;
                    self.step_delta = 0;
                    self.step_timer = CloakStepTimer {
                        start_frame: facts.current_frame,
                        speed: 0,
                        duration_frames: 0,
                    };
                    self.recloak_delay_start = facts.current_frame;
                    self.recloak_delay_frames = facts.cloak_delay_frames;
                    result.transitioned = true;
                }
                1 if facts.can_auto_cloak => {
                    result.transitioned = self.start_cloaking(
                        facts.current_frame,
                        facts.cloaking_speed,
                    );
                }
                _ => {}
            },
            _ => {}
        }
        result
    }

    pub fn recloak_delay_expired(&self, now: i32) -> bool {
        Self::timer_remaining(self.recloak_delay_start, self.recloak_delay_frames, now) == 0
            && Self::timer_remaining(self.secondary_gate_start, self.secondary_gate_frames, now)
                == 0
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
    what_am_i: i32,
) -> FireCloakGateResult {
    FireCloakGateResult {
        should_call_reveal_area1: reveal_on_fire && target_house_passes_reveal_check,
        fire_error_code: fire_requires_uncloaking(
            decloak_to_fire,
            current_cloak_state,
            what_am_i,
        )
        .then_some(9),
    }
}

pub(crate) fn fire_requires_uncloaking(
    decloak_to_fire: bool,
    current_cloak_state: i32,
    what_am_i: i32,
) -> bool {
    decloak_to_fire
        && current_cloak_state != 0
        && (what_am_i != 2 || current_cloak_state == 2)
}

#[cfg(test)]
#[path = "cloak_sound_tests.rs"]
mod sound_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloak_transition_vectors() {
        let mut state = CloakRuntime::new(0, 9);
        let mut rng = SimRng::new(1);
        let facts = |frame, can_auto, should_uncloak| CloakTickFacts {
            current_frame: frame,
            state_zero_head_allows: true,
            can_auto_cloak: can_auto,
            should_uncloak,
            health_above_red: true,
            cloaking_speed: 1,
            cloak_delay_frames: 18,
        };
        state.tick(facts(0, true, false), &mut rng);
        assert_eq!(state.state, 1);
        for frame in 1..=5 {
            state.tick(facts(frame, true, false), &mut rng);
        }
        assert_eq!(state.state, 2, "visual state 3 completes active YR cloak");
        state.tick(facts(6, false, true), &mut rng);
        assert_eq!(state.state, 3);
        for frame in 7..=14 {
            state.tick(facts(frame, false, false), &mut rng);
        }
        assert_eq!(state.state, 0);
        assert_eq!(state.recloak_delay_frames, 18);
    }

    fn seed_with_first_roll(mut accept: impl FnMut(u32) -> bool) -> u64 {
        (0..100_000)
            .find(|seed| {
                let mut rng = SimRng::new(*seed);
                accept(rng.next_range_u32_inclusive(0, 99))
            })
            .expect("bounded seed search finds requested roll")
    }

    #[test]
    fn cloak_health_probability_branches_consume_exactly_one_scenario_draw() {
        let facts = |frame| CloakTickFacts {
            current_frame: frame,
            state_zero_head_allows: true,
            can_auto_cloak: true,
            should_uncloak: false,
            health_above_red: false,
            cloaking_speed: 1,
            cloak_delay_frames: 18,
        };

        let seed4 = seed_with_first_roll(|roll| roll < 4);
        let mut actual = SimRng::new(seed4);
        let mut expected = actual.clone();
        assert!(expected.next_range_u32_inclusive(0, 99) < 4);
        let mut cloak = CloakRuntime::new(0, 9);
        let result = cloak.tick(facts(0), &mut actual);
        assert!(result.consumed_scenario_rng && result.transitioned);
        assert_eq!(cloak.state, 1);
        assert_eq!(actual.logical_state(), expected.logical_state());

        let seed4_boundary = seed_with_first_roll(|roll| roll == 4);
        let mut actual = SimRng::new(seed4_boundary);
        let mut cloak = CloakRuntime::new(0, 9);
        let result = cloak.tick(facts(0), &mut actual);
        assert!(result.consumed_scenario_rng && !result.transitioned);
        assert_eq!(cloak.state, 0, "the 4% branch is strict `< 4`");

        let seed10 = seed_with_first_roll(|roll| roll <= 9);
        let mut actual = SimRng::new(seed10);
        let mut expected = actual.clone();
        assert!(expected.next_range_u32_inclusive(0, 99) <= 9);
        let mut cloak = CloakRuntime::new(0, 9);
        cloak.state = 1;
        cloak.visual_phase = Some(CloakVisualPhase::Cloaking);
        cloak.depth = 3; // trunc(3/9*256)=85 => active visual state 2.
        cloak.step_delta = 1;
        cloak.step_timer = CloakStepTimer {
            start_frame: 0,
            speed: 1,
            duration_frames: 1,
        };
        let result = cloak.tick(facts(0), &mut actual);
        assert!(result.consumed_scenario_rng && result.transitioned);
        assert_eq!(cloak.state, 3);
        assert_eq!(actual.logical_state(), expected.logical_state());

        let seed10_boundary = seed_with_first_roll(|roll| roll == 10);
        let mut actual = SimRng::new(seed10_boundary);
        let mut cloak = CloakRuntime::new(0, 9);
        cloak.state = 1;
        cloak.visual_phase = Some(CloakVisualPhase::Cloaking);
        cloak.depth = 3;
        cloak.step_delta = 1;
        cloak.step_timer = CloakStepTimer {
            start_frame: 0,
            speed: 1,
            duration_frames: 1,
        };
        let result = cloak.tick(facts(0), &mut actual);
        assert!(result.consumed_scenario_rng && !result.transitioned);
        assert_eq!(cloak.state, 1, "the abort branch is inclusive only through 9");
    }

    #[test]
    fn healthy_autocloak_does_not_advance_scenario_rng() {
        let mut cloak = CloakRuntime::new(0, 9);
        let mut rng = SimRng::new(0xC10A_C001);
        let before = rng.logical_state();
        let result = cloak.tick(
            CloakTickFacts {
                current_frame: 0,
                state_zero_head_allows: true,
                can_auto_cloak: true,
                should_uncloak: false,
                health_above_red: true,
                cloaking_speed: 1,
                cloak_delay_frames: 18,
            },
            &mut rng,
        );
        assert!(result.transitioned && !result.consumed_scenario_rng);
        assert_eq!(rng.logical_state(), before);
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
    fn cloaking_speed_five_delays_each_progress_step() {
        let mut state = CloakRuntime::new(0, 9);
        let mut rng = SimRng::new(1);
        let facts = |frame| CloakTickFacts {
            current_frame: frame,
            state_zero_head_allows: true,
            can_auto_cloak: true,
            should_uncloak: false,
            health_above_red: true,
            cloaking_speed: 5,
            cloak_delay_frames: 18,
        };
        state.tick(facts(0), &mut rng);
        for frame in 1..5 {
            state.tick(facts(frame), &mut rng);
            assert_eq!(state.depth, 0);
        }
        state.tick(facts(5), &mut rng);
        assert_eq!(state.depth, 1);
    }
}
