//! Ordinary Unit DeploysInto continuation. Native identities: EventClass event 9
//! 0x004C77EA..0x004C7812, UnitClass::Deploy 0x007393C0 and Mission_Unload
//! 0x0073D630. See tools/mcv_deploy_oracle.py for executable boundary evidence.
//! MissionCom owns scheduling; the entity flag represents runtime Unit+0x68C.

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::movement::{self, FacingClass};
use crate::sim::world::Simulation;

pub(crate) fn is_mcv(sim: &Simulation, entity: &GameEntity, rules: &RuleSet) -> bool {
    entity.category == EntityCategory::Unit
        && sim
            .object_type(entity.type_ref(), rules)
            .is_some_and(|obj| {
                // The passenger/refinery branches precede DeploysInto in 0x73D630;
                // slave miners already have a separate production owner.
                obj.deploys_into
                    .as_deref()
                    .is_some_and(|name| rules.object(name).is_some())
                    && obj.passengers == 0
                    && !obj.harvester
                    && obj.enslaves.is_none()
            })
}

pub(crate) fn issue_order(sim: &mut Simulation, id: u64, rules: &RuleSet) -> bool {
    if !sim
        .substrate
        .entities
        .get(id)
        .is_some_and(|e| !e.dying && !e.lifecycle.in_limbo && is_mcv(sim, e, rules))
    {
        return false;
    }
    sim.run_dock_teardown(id, crate::sim::mission::retask::DockTeardown::All);
    let entity = sim.substrate.entities.get_mut(id).unwrap();
    // Event9 clears destination and target before Queue(Unload,false). Queue's
    // existing same-mission guard preserves the handler/timer on repeated D.
    movement::stop_navigation_at_committed_head(entity);
    entity.attack_target = None;
    entity.passively_acquired_target = false;
    entity.order_intent = None;
    sim.mission_queue_exact(
        id,
        MissionId::from_known(MissionType::Unload),
        0,
        sim.session.binary_frame,
        &EntityReadyInputProvider,
    )
    .is_ok()
}

/// Native rounds FacingClass::Current, including wrap at 0xff80.
pub(crate) fn current_direction(entity: &GameEntity, frame: u32) -> u8 {
    let raw = entity
        .body_facing
        .as_ref()
        .map_or(u16::from(entity.facing) << 8, |body| body.current(frame));
    (((u32::from(raw) >> 7) + 1) >> 1) as u8
}

pub(crate) fn start_turn(entity: &mut GameEntity, target: u8, frame: u32) {
    // DriveLocomotion::Do_Turn 0x004B0EF0 delegates to FacingClass::Set
    // 0x004C9220; setting an existing destination must not restart its timer.
    let rot = entity
        .locomotor
        .as_ref()
        .map_or(0, |loco| loco.rot)
        .clamp(0, 127) as u8;
    let body = entity
        .body_facing
        .get_or_insert_with(|| FacingClass::new(u16::from(entity.facing) << 8, rot));
    body.set(u16::from(target) << 8, frame);
    entity.facing_target = Some(target);
}

pub(crate) fn queue_guard(sim: &mut Simulation, id: u64) {
    queue(sim, id, MissionType::Guard);
}

fn queue(sim: &mut Simulation, id: u64, mission: MissionType) {
    let _ = sim.mission_queue_exact(
        id,
        MissionId::from_known(mission),
        0,
        sim.session.binary_frame,
        &EntityReadyInputProvider,
    );
}

/// Returns a delay to the existing MissionClass epilogue, even when Deploy
/// removed the caller. Every ordinary MCV branch reaches 0x0073DE3A's RNG draw.
pub(crate) fn mission_unload(sim: &mut Simulation, id: u64, rules: &RuleSet) -> i32 {
    let state = sim
        .substrate
        .entities
        .get(id)
        .map(|e| e.mission.handler_state());
    if state == Some(0) {
        let e = sim.substrate.entities.get_mut(id).unwrap();
        if let Some(drive) = e.drive_locomotion.as_mut() {
            drive.path.cursor = drive.path.directions.len().min(u16::MAX as usize) as u16;
        }
        e.mission.set_handler_state(1);
        // Native state 0 falls through to state 1 in this same invocation.
    }
    match state {
        Some(0 | 1) => {
            let moving = sim
                .substrate
                .entities
                .get(id)
                .is_some_and(movement::ready_producer::is_moving_for_unit_shp_draw);
            if !moving {
                sim.deploy_mcv(id, rules, &Default::default());
                finish_initial_attempt(sim, id);
            }
        }
        Some(2) => {
            let pending = sim
                .substrate
                .entities
                .get(id)
                .is_some_and(|e| e.mcv_deploy_pending);
            if pending {
                let accepted = sim.deploy_mcv(id, rules, &Default::default());
                finish_retry(sim, id, accepted);
            }
        }
        _ => {}
    }
    let base = rules
        .mission_control
        .rate_frames(MissionType::Unload)
        .min(i32::MAX as u32) as i32;
    base.saturating_add(sim.scenario_rng.next_range_u32_inclusive(0, 2) as i32)
}

// Kept as the production result seam so original interior-block oracle outputs
// can be compared without claiming to emulate the whole construction transaction.
pub(crate) fn finish_initial_attempt(sim: &mut Simulation, id: u64) {
    if let Some(e) = sim.substrate.entities.get(id).filter(|e| !e.dying) {
        if e.mcv_deploy_pending {
            sim.substrate
                .entities
                .get_mut(id)
                .unwrap()
                .mission
                .set_handler_state(2);
        } else {
            let hunt = sim
                .houses
                .get(&e.owner())
                .is_some_and(|house| !house.is_controlled_by_human(true))
                && sim.session.game_mode_nonzero;
            queue(
                sim,
                id,
                if hunt {
                    MissionType::Hunt
                } else {
                    MissionType::Guard
                },
            );
        }
    }
}

pub(crate) fn finish_retry(sim: &mut Simulation, id: u64, accepted: bool) {
    if !accepted
        && let Some(e) = sim.substrate.entities.get_mut(id)
        && e.navigation.nav_com.is_some()
    {
        e.mcv_deploy_pending = false;
    }
}

pub(crate) fn rotation_completed(previous: &mut bool, rotating: bool) -> bool {
    let completed = *previous && !rotating;
    *previous = rotating;
    completed
}

/// Drive::Process 0x4B077B/0x4B0896: the rotating-to-rest edge calls
/// Unit::PerCellProcess(0), which retries +0x68C independently of Mission_Unload.
/// This runs at the locomotor entry, after the object's mission dispatch.
pub(crate) fn drive_process_prelude(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    if !sim.order_actor_admits(id) || !sim.substrate.entities.get(id).is_some_and(|e| {
        !e.dying
            // 0x4B055A..0x4B056D: active track owns Process and must leave
            // the previous-rotation latch alone until the track is finished.
            && e.drive_track.is_none()
            && e.forced_drive_track.is_none()
            && !e.drive_locomotion.as_ref().is_some_and(|d| d.track_valid && d.track.turn_index != -1)
            // 0x4B066C..0x4B06C3: same-cell NavCom is handled by the
            // destination/waypoint owner before the rotation branch.
            && !matches!(e.navigation.nav_com, Some(crate::sim::components::NavTargetRef::Cell { rx, ry })
                if (rx,ry) == (e.position.rx,e.position.ry))
            && is_mcv(sim, e, rules)
            && e.locomotor.as_ref().is_some_and(|l| {
                l.active_kind() == crate::rules::locomotor_type::LocomotorKind::Drive
            })
    }) {
        return;
    }
    let e = sim.substrate.entities.get_mut(id).unwrap();
    let rotating = e
        .body_facing
        .as_ref()
        .is_some_and(|f| f.is_rotating(sim.session.binary_frame));
    let completed = rotation_completed(&mut e.mcv_drive_was_rotating, rotating);
    if let Some(body) = e.body_facing.as_ref() {
        e.facing = (body.current(sim.session.binary_frame) >> 8) as u8;
    }
    if completed {
        per_cell_process(sim, id, rules);
    }
}

pub(crate) fn per_cell_process(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    // 0x739EEC..0x739EF8 has no current-mission guard. Stop and Move do not
    // erase pending intent; native Deploy decides whether NavCom allows it.
    if sim
        .substrate
        .entities
        .get(id)
        .is_some_and(|e| !e.dying && e.mcv_deploy_pending)
    {
        sim.deploy_mcv(id, rules, &Default::default());
    }
}

#[cfg(test)]
#[path = "mcv_deploy_tests.rs"]
mod tests;
