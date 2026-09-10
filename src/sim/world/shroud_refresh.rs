//! FootAI4DA692..4DA7AA: admitted high-flying sight refresh before Process.
//! Per-viewer clocks preserve the original local-player admission boundary;
//! House/view-cache reconciliation never advances these clocks or emits events.
use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::{locomotor_type::LocomotorKind, ruleset::RuleSet};
use crate::sim::{game_entity::GameEntity, pathfinding::PathGrid, vision};

/// Active represented slot80 implementations: Fly4CCAC0 compares current speed
/// with zero; Rocket661F90 accepts signed states3..5; other live families use
/// the existing native-readiness owner (including Jumpjet54D0D0 state!=0,2).
fn moving_now(entity: &GameEntity, frame: u32) -> bool {
    let Some(loco) = entity.locomotor.as_ref() else {
        return false;
    };
    match loco.active_kind() {
        LocomotorKind::Fly => loco.fly_current_speed != crate::util::fixed_math::SIM_ZERO,
        LocomotorKind::Rocket => entity.rocket_state.as_ref().is_some_and(|state| {
            matches!(
                state.phase,
                crate::sim::movement::rocket_movement::RocketPhase::Ascent
                    | crate::sim::movement::rocket_movement::RocketPhase::Cruise
                    | crate::sim::movement::rocket_movement::RocketPhase::Terminal
            )
        }),
        _ => crate::sim::movement::ready_producer::is_moving_now_for(entity, frame),
    }
}

impl Simulation {
    pub(super) fn refresh_high_flying_sight_before_process(
        &mut self,
        stable_id: u64,
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
    ) {
        let frame = self.session.binary_frame;
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        if entity.dying
            || entity.lifecycle.in_limbo
            || !matches!(
                entity.category,
                EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
            )
            || !moving_now(entity, frame)
        {
            return;
        }
        // Object+74/on-map and GetHeight>=2*104. Rocket's current altitude has
        // its own represented owner, also used by the ordinary reveal geometry.
        let high = entity.rocket_state.as_ref().map_or_else(
            || crate::sim::combat::in_range::is_high_flying(entity),
            |state| {
                state.altitude.to_num::<i32>()
                    >= crate::util::lepton::HIGH_FLIGHT_THRESHOLD_LEPTONS as i32
            },
        );
        if !high {
            return;
        }
        self.fog.alliances = self.house_alliances.clone();
        let viewers = vision::direct_reveal_viewers(&self.fog, entity.owner(), &self.interner);
        let due: Vec<_> = viewers
            .into_iter()
            //4DA6B4 loads SOURCE House before4F9A50; reverse-only alliance
            //does not admit this event or mutate this viewer's countdown.
            .filter(|&viewer| {
                crate::map::houses::is_allied_with(
                    &self.house_alliances,
                    self.interner.resolve(entity.owner()),
                    self.interner.resolve(viewer),
                )
            })
            .filter(|&viewer| {
                entity
                    .sight_refresh_timers
                    .timer(viewer)
                    .expired(frame as i32)
            })
            .collect();
        if due.is_empty() {
            return;
        }
        let config = vision::VisionConfig {
            require_playfield_membership: true,
            veteran_sight: rules.map_or(0.0, |r| r.general.veteran_sight),
            leptons_per_sight_increase: rules.map_or(0, |r| r.general.leptons_per_sight_increase),
            reveal_by_height: rules.map_or(true, |r| r.general.reveal_by_height),
            fog_of_war: self.session.game_options.fog_of_war,
        };
        let height_grid = if config.reveal_by_height {
            path_grid.map(PathGrid::ground_height_grid)
        } else {
            None
        };
        let ability = vision::entity_has_sight_ability(entity, &self.interner, rules);
        for &viewer in &due {
            // The two reveal leaves may reject membership/zero-radius. The
            // due FootAI branch nevertheless reloads15 after its calls.
            if entity.in_playfield && !entity.passenger_role.is_inside_transport() {
                vision::refresh_entity_vision_for_viewer(
                    &mut self.fog,
                    entity,
                    &config,
                    height_grid.as_deref(),
                    ability,
                    &self.interner,
                    viewer,
                );
            }
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            for viewer in due {
                entity.sight_refresh_timers.reload(viewer, frame);
            }
        }
    }
}
