//! Exact arithmetic and six-slot selection for the House base-defence response.
//!
//! The receiver integration owns entity, House, Team and mission mutations.
//! This module keeps the native signed arithmetic and ranking pathology small
//! enough to prove independently before those authorities are borrowed.

use crate::util::native_x87::{NativeF64Bits, X87Chop53, distance_3d_leptons};

const RESPONSE_LIST_CAPACITY: usize = 6;
const FRAMES_PER_MINUTE: i32 = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResponderClass {
    Infantry,
    Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExistingTargetDisposition {
    NoneOrUnarmed,
    RequestedAttacker,
    OtherArmedTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThreatFacts {
    pub(crate) cost: i32,
    pub(crate) speed_leptons_per_frame: i32,
    pub(crate) current_coord: [i32; 3],
    pub(crate) attacker_coord: [i32; 3],
    pub(crate) primary_range_leptons: i32,
    pub(crate) existing_target: ExistingTargetDisposition,
    pub(crate) in_non_base_defense_team: bool,
    pub(crate) mission_is_harvest: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RankedResponder {
    pub(crate) entity_id: u64,
    pub(crate) score: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResponseSelection {
    remaining_budget: i32,
    minimum_score: i32,
    responders: Vec<RankedResponder>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResponseMission {
    Rescue,
    AreaGuard,
}

/// gamemd-derived: `TechnoClass__RespondToBaseAttack @ 0x00708080` checks the
/// attacker-owned `TechnoClass+0x650/+0x658` timer with signed wrapping frame
/// subtraction. Start `-1` is the inactive constructor sentinel.
pub(crate) fn cooldown_remaining(start_frame: i32, duration_frames: i32, now: i32) -> i32 {
    if start_frame == -1 {
        return 0;
    }
    let elapsed = now.wrapping_sub(start_frame);
    if elapsed < duration_frames {
        duration_frames.wrapping_sub(elapsed)
    } else {
        0
    }
}

/// Convert the Rules double through the native x87 `ftol` path. Invalid or
/// out-of-range values retain the x87 signed-indefinite low dword.
///
/// gamemd-derived: `TechnoClass__RespondToBaseAttack @ 0x0070878D` multiplies
/// `[General] BaseDefenseDelay` by the literal `900.0` before `Math__ftol`.
pub(crate) fn response_delay_frames(delay_minutes: f64) -> i32 {
    let loaded = X87Chop53::load_f64(NativeF64Bits::from_bits(delay_minutes.to_bits()));
    let Ok(delay) = loaded else {
        return i32::MIN;
    };
    X87Chop53::ftol_i64(X87Chop53::mul(
        delay,
        X87Chop53::load_i32(FRAMES_PER_MINUTE),
    ))
    .unwrap_or(i64::MIN) as i32
}

/// Exact signed threat score used only by the base-defence responder.
///
/// gamemd-derived: `FootClass__Evaluate_Target_Threat @ 0x004D97A0` uses the
/// current 3-D lepton coordinates, `CoordStruct__Distance3D`'s
/// `Sqrt_Approx/ftol`, signed wrapping `cost << 10`, and integer travel time.
pub(crate) fn evaluate_target_threat(facts: ThreatFacts) -> i32 {
    match facts.existing_target {
        ExistingTargetDisposition::RequestedAttacker => return facts.cost.wrapping_neg(),
        ExistingTargetDisposition::OtherArmedTarget => return 0,
        ExistingTargetDisposition::NoneOrUnarmed => {}
    }
    if facts.in_non_base_defense_team || facts.mission_is_harvest || facts.cost == 0 {
        return 0;
    }

    let distance = distance_3d_leptons(facts.current_coord, facts.attacker_coord);
    let base = facts.cost.wrapping_shl(10);
    if distance <= facts.primary_range_leptons {
        return base;
    }

    let speed = facts.speed_leptons_per_frame.max(1);
    let travel_frames = distance
        .wrapping_sub(facts.primary_range_leptons)
        .wrapping_div(speed)
        .max(1);
    base.wrapping_div(travel_frames).max(1)
}

fn class_adjusted_score(score: i32, class: ResponderClass, victim_is_self_anchor: bool) -> i32 {
    if !victim_is_self_anchor || score == 0 {
        return score;
    }
    match class {
        // The Infantry loop applies its self-anchor multiplier before the
        // negative-score budget branch.
        ResponderClass::Infantry => score.wrapping_mul(100),
        // The Unit loop reaches its multiplier only on the positive arm.
        ResponderClass::Unit if score > 0 => score.wrapping_mul(10),
        ResponderClass::Unit => score,
    }
}

impl ResponseSelection {
    pub(crate) fn new(budget: i32) -> Self {
        Self {
            remaining_budget: budget,
            minimum_score: 0,
            responders: Vec::with_capacity(RESPONSE_LIST_CAPACITY),
        }
    }

    pub(crate) fn remaining_budget(&self) -> i32 {
        self.remaining_budget
    }

    pub(crate) fn can_scan(&self) -> bool {
        self.remaining_budget > 0
    }

    /// Apply the Infantry/Unit self-anchor multiplier, debit already-engaged
    /// defenders, and reproduce the native six-slot minimum bug.
    ///
    /// gamemd-derived: `TechnoClass__RespondToBaseAttack @
    /// 0x00708340..0x0070868F`. While the list grows, `minimum_score` remains
    /// zero. The first later positive candidate therefore only exposes the real
    /// minimum (it replaces no slot); subsequent replacements overwrite every
    /// equal-minimum slot with the same candidate, preserving duplicates.
    pub(crate) fn consider(
        &mut self,
        entity_id: u64,
        raw_score: i32,
        class: ResponderClass,
        victim_is_self_anchor: bool,
    ) {
        let score = class_adjusted_score(raw_score, class, victim_is_self_anchor);
        if score < 0 {
            self.remaining_budget = self.remaining_budget.wrapping_add(score);
            return;
        }
        if score == 0 {
            return;
        }
        if self.responders.len() < RESPONSE_LIST_CAPACITY {
            self.responders.push(RankedResponder { entity_id, score });
            return;
        }
        if score <= self.minimum_score {
            return;
        }

        let old_minimum = self.minimum_score;
        let mut replaced = [false; RESPONSE_LIST_CAPACITY];
        for (index, responder) in self.responders.iter_mut().enumerate() {
            if responder.score == old_minimum {
                *responder = RankedResponder { entity_id, score };
                replaced[index] = true;
            }
        }

        self.minimum_score = score;
        for (index, responder) in self.responders.iter().enumerate() {
            if !replaced[index] && responder.score < self.minimum_score {
                self.minimum_score = responder.score;
            }
        }
    }

    /// Stable signed descending sort: equal scores never exchange positions.
    pub(crate) fn into_ranked(mut self) -> (i32, Vec<RankedResponder>) {
        for upper in (1..self.responders.len()).rev() {
            for index in 0..upper {
                if self.responders[index].score < self.responders[index + 1].score {
                    self.responders.swap(index, index + 1);
                }
            }
        }
        (self.remaining_budget, self.responders)
    }
}

/// Every selected occurrence consumes the draw. Team membership changes only
/// the interpretation, never whether the draw occurs.
///
/// gamemd-derived: `TechnoClass__RespondToBaseAttack @
/// 0x007086DB..0x00708718` queues Rescue for `0..=65`, else Area Guard; a
/// base-defence Team member is forced to Area Guard after the draw.
pub(crate) fn response_mission(draw_0_to_99: u32, in_base_defense_team: bool) -> ResponseMission {
    debug_assert!(draw_0_to_99 <= 99);
    if !in_base_defense_team && draw_0_to_99 <= 65 {
        ResponseMission::Rescue
    } else {
        ResponseMission::AreaGuard
    }
}

/// The assignment loop stops only after its signed wrapping cost sum strictly
/// exceeds the post-scan budget; equality deliberately continues.
pub(crate) fn add_assigned_cost(accumulated: i32, cost: i32, budget: i32) -> (i32, bool) {
    let accumulated = accumulated.wrapping_add(cost);
    (accumulated, accumulated > budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn threat(cost: i32, distance: i32, range: i32, speed: i32) -> ThreatFacts {
        ThreatFacts {
            cost,
            speed_leptons_per_frame: speed,
            current_coord: [0, 0, 0],
            attacker_coord: [distance, 0, 0],
            primary_range_leptons: range,
            existing_target: ExistingTargetDisposition::NoneOrUnarmed,
            in_non_base_defense_team: false,
            mission_is_harvest: false,
        }
    }

    #[test]
    fn gsi_04_05_cooldown_uses_inactive_sentinel_and_signed_wrapping_elapsed() {
        assert_eq!(cooldown_remaining(-1, 225, 100), 0);
        assert_eq!(cooldown_remaining(100, 225, 100), 225);
        assert_eq!(cooldown_remaining(100, 225, 324), 1);
        assert_eq!(cooldown_remaining(100, 225, 325), 0);
        assert_eq!(cooldown_remaining(i32::MAX - 2, 6, i32::MIN + 1), 2);
        assert_eq!(response_delay_frames(0.25), 225);
        assert_eq!(response_delay_frames(-0.25), -225);
    }

    #[test]
    fn gsi_04_05_threat_preserves_special_targets_and_signed_integer_math() {
        let mut facts = threat(100, 1024, 256, 128);
        assert_eq!(evaluate_target_threat(facts), (100_i32 << 10) / 6);
        facts.attacker_coord = [256, 0, 0];
        assert_eq!(evaluate_target_threat(facts), 100_i32 << 10);
        facts.existing_target = ExistingTargetDisposition::RequestedAttacker;
        assert_eq!(evaluate_target_threat(facts), -100);
        facts.existing_target = ExistingTargetDisposition::OtherArmedTarget;
        assert_eq!(evaluate_target_threat(facts), 0);
        facts.existing_target = ExistingTargetDisposition::NoneOrUnarmed;
        facts.mission_is_harvest = true;
        assert_eq!(evaluate_target_threat(facts), 0);
    }

    #[test]
    fn gsi_04_05_threat_uses_wrapping_base_and_native_sqrt_distance() {
        let facts = threat(i32::MAX, 513, 0, 256);
        assert_eq!(evaluate_target_threat(facts), 1);
        let diagonal = ThreatFacts {
            attacker_coord: [256, 256, 256],
            primary_range_leptons: 0,
            speed_leptons_per_frame: 1,
            cost: 2,
            ..threat(2, 0, 0, 1)
        };
        assert_eq!(distance_3d_leptons([0, 0, 0], [256, 256, 256]), 443);
        assert_eq!(evaluate_target_threat(diagonal), 4);
    }

    #[test]
    fn gsi_04_05_negative_scores_debit_budget_with_class_specific_anchor_order() {
        let mut selection = ResponseSelection::new(500);
        selection.consider(1, -2, ResponderClass::Infantry, true);
        assert_eq!(selection.remaining_budget(), 300);
        selection.consider(2, -2, ResponderClass::Unit, true);
        assert_eq!(selection.remaining_budget(), 298);
        assert!(selection.can_scan());
        selection.consider(3, -298, ResponderClass::Unit, false);
        assert!(!selection.can_scan());
    }

    #[test]
    fn gsi_04_05_first_six_leave_minimum_zero_and_seventh_only_exposes_minimum() {
        let mut selection = ResponseSelection::new(1);
        for (id, score) in [(1, 9), (2, 2), (3, 8), (4, 3), (5, 7), (6, 4)] {
            selection.consider(id, score, ResponderClass::Unit, false);
        }
        selection.consider(7, 100, ResponderClass::Unit, false);
        let (_, ranked) = selection.into_ranked();
        assert_eq!(
            ranked
                .iter()
                .map(|entry| entry.entity_id)
                .collect::<Vec<_>>(),
            [1, 3, 5, 6, 4, 2]
        );
    }

    #[test]
    fn gsi_04_05_replacement_overwrites_every_old_minimum_with_duplicates() {
        let mut selection = ResponseSelection::new(1);
        for (id, score) in [(1, 9), (2, 2), (3, 2), (4, 8), (5, 7), (6, 6)] {
            selection.consider(id, score, ResponderClass::Unit, false);
        }
        selection.consider(7, 100, ResponderClass::Unit, false);
        selection.consider(8, 5, ResponderClass::Unit, false);
        let (_, ranked) = selection.into_ranked();
        assert_eq!(
            ranked.iter().filter(|entry| entry.entity_id == 8).count(),
            2
        );
        assert_eq!(
            ranked
                .iter()
                .map(|entry| entry.entity_id)
                .collect::<Vec<_>>(),
            [1, 4, 5, 6, 8, 8]
        );
    }

    #[test]
    fn gsi_04_05_stable_descending_sort_retains_equal_score_order() {
        let mut selection = ResponseSelection::new(1);
        for (id, score) in [(1, 4), (2, 9), (3, 4), (4, 7)] {
            selection.consider(id, score, ResponderClass::Infantry, false);
        }
        let (_, ranked) = selection.into_ranked();
        assert_eq!(
            ranked
                .iter()
                .map(|entry| entry.entity_id)
                .collect::<Vec<_>>(),
            [2, 4, 1, 3]
        );
    }

    #[test]
    fn gsi_04_05_draw_boundary_and_strict_budget_overshoot_are_literal() {
        assert_eq!(response_mission(65, false), ResponseMission::Rescue);
        assert_eq!(response_mission(66, false), ResponseMission::AreaGuard);
        assert_eq!(response_mission(0, true), ResponseMission::AreaGuard);

        assert_eq!(add_assigned_cost(0, 100, 100), (100, false));
        assert_eq!(add_assigned_cost(100, 1, 100), (101, true));
        assert_eq!(add_assigned_cost(i32::MAX, 1, -1), (i32::MIN, false));
    }
}
