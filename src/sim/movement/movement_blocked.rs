//! Blocked movement handling — repath attempts when a mover's next cell is occupied or impassable.
//!
//! Called from movement_tick when terrain, cliff, or occupancy checks fail.
//! Manages the blocked_delay timer and path_stuck_counter to prevent thrashing.

use std::collections::BTreeSet;

use crate::rules::locomotor_type::MovementZone;
use crate::sim::components::MovementTarget;
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};
use crate::sim::pathfinding::LayeredEntityBlockMap;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};

use super::movement_path::{supports_layered_bridge_pathing, try_repath_after_block};
use super::path_markers::BridgeMarkerContext;
use super::{MovementConfig, MovementTickStats, PathfindingContext};

/// Shared logic for handling a blocked movement tick.
///
/// Implements the original engine's two-timer system:
/// - `movement_delay` guards against calling Find_Path too often (PathDelay=)
/// - `blocked_delay` waits for friendlies to clear before escalating (BlockagePathDelay=)
/// - `path_stuck_counter` limits total retries before giving up (init=10)
///
/// `skip_grace_period` distinguishes gamemd's code-7 (terrain / impassable
/// / hard-block) path from code-2 (moving friendly). Code-7 has no grace
/// timer in the original — the unit stops and repaths immediately at
/// urgency=2. Code-2 spends `BlockagePathDelay` ticks at urgency=1 before
/// escalating.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_blocked_tick(
    target: &mut MovementTarget,
    facing: &mut u8,
    body_facing: Option<super::FacingClass>,
    locomotor: &Option<LocomotorState>,
    drive_locomotion: &mut Option<crate::sim::components::DriveLocomotionRuntime>,
    ship_locomotion: &mut Option<crate::sim::components::ShipLocomotionRuntime>,
    entity_id: u64,
    current_pos: (u16, u16),
    active_layer: MovementLayer,
    on_bridge: bool,
    stats: &mut MovementTickStats,
    finished_entities: &mut Vec<u64>,
    aborted_for_stuck: &mut bool,
    ctx: PathfindingContext<'_>,
    entity_cost_grid: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    too_big_to_fit_under_bridge: bool,
    mcfg: MovementConfig,
    rng: &mut SimRng,
    sim_tick: u64,
    path_stuck_init: u8,
    mover_is_crusher: bool,
    is_infantry: bool,
    skip_grace_period: bool,
    close_enough_abort: bool,
    marker_context: Option<BridgeMarkerContext<'_>>,
    occupancy: &crate::sim::occupancy::OccupancyGrid,
) -> Vec<(u32, DebugEventKind)> {
    let mut deferred_events: Vec<(u32, DebugEventKind)> = Vec::new();
    stats.blocked_attempts = stats.blocked_attempts.saturating_add(1);
    let next_cell = target.path.get(target.next_index).copied();
    let goal = target
        .final_goal
        .unwrap_or_else(|| target.path.last().copied().unwrap_or(current_pos));

    if !target.path_blocked {
        target.path_blocked = true;
        target.blocked_delay = if skip_grace_period {
            0
        } else {
            mcfg.blockage_path_delay_ticks
        };
        if let Some((nx, ny)) = next_cell {
            deferred_events.push((
                sim_tick as u32,
                DebugEventKind::Blocked {
                    by_entity: None,
                    cell: (nx, ny),
                },
            ));
        }
    } else if skip_grace_period {
        // Terrain/impassable block reached while a code-2 grace timer is
        // still running from a prior entity block. gamemd code-7 path has
        // no grace — reset so urgency=2 fires this tick.
        target.blocked_delay = 0;
    }

    // The `CloseEnough` give-up radius is not consulted by every block code.
    // All five `Rules+0x1718` compares in the Drive movement body sit outside
    // the code-2 dispatch, so a mover blocked by a moving friendly never
    // abandons its approach on that ground — it just repaths. Callers on the
    // code-2 arm pass `false`. (The remaining arms keep VERA's single shared
    // site; whether each of them maps onto one of the five native compares is
    // UNCHECKED and recorded separately.)
    if close_enough_abort && mcfg.close_enough > SIM_ZERO {
        // Native compares a genuine 3-D Euclidean lepton distance —
        // `CoordStruct::Distance3D` @ `0x0041C380`, `Sqrt_Approx(z² + y² + x²)`
        // then `Math::ftol` — against `Rules+0x1718` (`CloseEnough`, string
        // `0x0083BD84`). `Process_Movement` compares it at `0x004B297F`,
        // `0x004B2C5C`, `0x004B3141`, `0x004B37BE` and `0x004B42DB`.
        //
        // A Manhattan sum is never smaller than the Euclidean one, and the
        // predicate is `dist < CloseEnough` → stop, so the old form made the
        // abort fire *less* often: units kept pushing to the exact goal where
        // retail already declared arrival. With stock `CloseEnough = 576` the
        // only cell offsets that change answer are Δ(2,1) and Δ(1,2) —
        // Manhattan 768 (no abort) against Euclidean 572 (abort).
        //
        // Two VERA-internal residuals remain, gamemd equivalent UNCHECKED.
        // Z is dropped, because VERA's goal here is a cell pair with no height —
        // it matters on bridge approaches. And the distance is measured between
        // cell indices × 256 rather than between actual coordinates, so VERA is
        // blind to sub-cell offsets, which is the larger of the two errors.
        //
        // Unpinned: `close_enough_abort` has two references in the whole
        // tree, the path-test harness sets `close_enough` to zero which
        // disables this branch, and no fixture drives a diagonal approach at
        // the give-up radius. One at Δ(2,1) with `close_enough = 576` would
        // pin the entire behaviour delta.
        let dx = (goal.0 as i64 - current_pos.0 as i64).abs() * 256;
        let dy = (goal.1 as i64 - current_pos.1 as i64).abs() * 256;
        let dist = SimFixed::from_num(crate::util::fixed_math::isqrt_i64(dx * dx + dy * dy));
        if dist < mcfg.close_enough {
            log::info!(
                "CLOSE_ENOUGH entity={} pos=({},{}) goal=({},{}) dist={} - stopping",
                entity_id,
                current_pos.0,
                current_pos.1,
                goal.0,
                goal.1,
                dist,
            );
            finished_entities.push(entity_id);
            *aborted_for_stuck = true;
            return deferred_events;
        }
    }

    if target.movement_delay > 0 {
        return deferred_events;
    }

    // Repath every tick while movement_delay == 0. Urgency escalates once the
    // blocked_delay (BlockagePathDelay) timer has expired:
    //   urgency=1 while blocked_delay > 0 → 4x traffic penalty
    //   urgency=2 once blocked_delay == 0 → 1000x route-around
    // Matches gamemd.exe DriveLocomotionClass::Process_Movement (LAB_004b3607).
    stats.repath_attempts = stats.repath_attempts.saturating_add(1);
    let urgency: u8 = if target.blocked_delay > 0 { 1 } else { 2 };
    let layered_pathing_for_repath = locomotor
        .as_ref()
        .zip(ctx.path_grid)
        .is_some_and(|(loco, pg)| supports_layered_bridge_pathing(loco, pg, on_bridge));
    let repath_mz: Option<MovementZone> = locomotor.as_ref().map(|l| l.movement_zone);
    let marker_search = marker_context.map(|context| {
        context.build(
            occupancy,
            entity_id,
            current_pos,
            *facing,
            body_facing,
            on_bridge,
            urgency,
        )
    });
    let repath_ok = try_repath_after_block(
        target,
        facing,
        current_pos,
        active_layer,
        layered_pathing_for_repath,
        ctx,
        entity_cost_grid,
        entity_blocks,
        rng,
        repath_mz,
        too_big_to_fit_under_bridge,
        mcfg,
        entity_block_map,
        urgency,
        mover_is_crusher,
        is_infantry,
        marker_search.as_ref(),
    );
    if repath_ok {
        match locomotor.as_ref().map(|locomotor| locomotor.kind) {
            Some(crate::rules::locomotor_type::LocomotorKind::Drive) => {
                if let Some(drive) = drive_locomotion.as_mut() {
                    super::path_markers::install_path_replay(
                        &mut drive.path,
                        current_pos,
                        &target.path,
                        target.next_index,
                    );
                }
            }
            Some(crate::rules::locomotor_type::LocomotorKind::Ship) => {
                if let Some(ship) = ship_locomotion.as_mut() {
                    super::path_markers::install_path_replay(
                        &mut ship.path,
                        current_pos,
                        &target.path,
                        target.next_index,
                    );
                }
            }
            _ => {}
        }
        stats.repath_successes = stats.repath_successes.saturating_add(1);
        if is_infantry {
            target.path_blocked = false;
        }
        target.path_stuck_counter = path_stuck_init;
        deferred_events.push((
            sim_tick as u32,
            DebugEventKind::Repath {
                reason: format!(
                    "blocked repath succeeded (urgency={} effective={})",
                    urgency,
                    marker_search
                        .as_ref()
                        .map_or(urgency, |search| search.effective_urgency)
                ),
                new_path_len: target.path.len(),
            },
        ));
    } else if urgency >= 2 {
        // Only escalated (urgency=2) repath failures count toward give-up.
        // gamemd.exe decrements path_stuck_counter in a separate "no valid
        // next cell" branch, not on every code-2 repath miss — so we don't
        // decrement during the blocked_delay grace period (urgency=1).
        target.path_stuck_counter = target.path_stuck_counter.saturating_sub(1);
        if target.path_stuck_counter == 0 {
            log::warn!(
                "STUCK ABORT entity={} pos=({},{}) - path_stuck_counter exhausted",
                entity_id,
                current_pos.0,
                current_pos.1,
            );
            deferred_events.push((
                sim_tick as u32,
                DebugEventKind::StuckAbort { blocked_ticks: 0 },
            ));
            stats.stuck_recoveries = stats.stuck_recoveries.saturating_add(1);
            finished_entities.push(entity_id);
            *aborted_for_stuck = true;
        }
        // gamemd does not restart the grace timer *here*, on a failed
        // route-around — this arm writes nothing.
        //
        // It does restart it in the code-2 dispatch, on every pass and not just
        // on the `path_blocked` 0 -> 1 transition: the original's store of the
        // wait sits straight-line after its blocker-scatter call with no branch
        // between them. An earlier revision of this comment claimed the
        // transition was the only writer, and the code was gated to match; that
        // left the timer at zero forever once it first expired, so the blocker
        // scatter — and its scenario-stream draw — fired every tick instead of
        // once per span. See
        // `movement_tests::code_two_post_scatter_wait_rearms_on_every_pass_while_the_block_holds`.
        //
        // So a boxed-in unit does not sit at urgency 2 continuously; it
        // escalates to 2 once per span, then drops back to 1 when the wait
        // re-arms.
    } else {
        // urgency=1 grace-period failure: set a short movement_delay to
        // rate-limit A* calls while the blocked_delay counter keeps ticking.
        target.movement_delay = mcfg.path_delay_ticks;
    }
    deferred_events
}
