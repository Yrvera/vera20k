//! Harvest mission handler — host-dispatched (AI-shell migration, the L5
//! seam grown into the real dispatch point).
//!
//! The per-object AI host (the Unit arm of `techno_ai_shell`, at the native
//! Mission_Dispatch position: after the AI-counter/promotion step, before the
//! post-mission common block) calls [`dispatch_harvest_for_object`] for every
//! live Unit. The dispatch is timer-gated and ends with the verified
//! post-handler epilogue write (host shape: `timer.start = current frame`,
//! `timer.delay = handler return` — the same write every native mission case
//! performs after its handler call).
//!
//! Authority: `MissionCom::handler_state` is the FSM cursor of record; the
//! bespoke `miner.state` field is retired. The handler decodes the cursor into
//! `MinerSnapshot::state`, runs the FSM step, and commits the cursor + the
//! dispatch delay back through the mission component.
//!
//! Cadence (verified against the native handler): the harvesting and dock
//! states plus the productive search paths return `DISPATCH_NEXT_FRAME`
//! (per-frame); the return/finding-home state, the idle state, the search
//! state's archive-consume and still-driving returns, and every cursor
//! outside the native switch exit through the default epilogue
//! (`ftol([Harvest] Rate × 900)` + `RandomRanged(0,2)` on the scenario
//! stream, ~14-16 frames stock); the no-ore transition into idle returns the
//! fixed 105-frame wait with no RNG draw. The Mission_Deploy state-4 dock
//! exit installs the same Rate epilogue at its own site.
//!
//! Structural residuals (native returns with no Rust dispatch equivalent):
//! the 450-frame non-harvester hold — dispatch is gated on Miner-component
//! presence, so a non-harvester never reaches the handler; and the
//! slave-host preamble's Rate epilogue — Slave hosts are dispatched by
//! `slave_miner.rs`, never through this handler.
//!
//! Dispatch gating residual: the host dispatches on Miner-component presence,
//! not strictly on `current == Harvest`. A player retask (Move/Stop/Attack)
//! flips `current` away from Harvest while the legacy FSM keeps driving the
//! behavior (waits out the player move, then resumes) — the pre-absorption
//! behavior. The strict mission-id gate becomes exact once the creation/idle
//! caller family (roadmap Track B1) commits missions at those points.
//!
//! Depends on: `world::Simulation`, `miner::miner_system`.
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch stays a `match` on the FSM cursor — no trait / dyn / vtable
//! (invariant #2).

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

use super::miner_system::{
    MinerSnapshot, build_miner_snapshot, commit_miner_snapshot, process_miner,
};
use super::{MinerKind, MinerState};

/// Host dispatch point: run one timer-gated Harvest handler step for `id`.
///
/// Called from the Unit arm of the per-object AI shell in live-object order.
/// No-op for objects that are not live, dispatchable miners, and for miners
/// whose dispatch timer is still pending.
pub(crate) fn dispatch_harvest_for_object(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &super::MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    id: u64,
) {
    let now = sim.session.binary_frame;
    {
        let Some(entity) = sim.substrate.entities.get(id) else {
            return;
        };
        if entity.dying {
            return;
        }
        let Some(miner) = entity.miner.as_ref() else {
            return;
        };
        if miner.kind == MinerKind::Slave {
            return;
        }
        // Native Mission_Dispatch gate: run the handler only when the
        // dispatch timer is due (verified host shape). The strength>0 gate is
        // the bracket's IsAlive guard upstream.
        if !entity.mission.dispatch_timer().due(now) {
            return;
        }
    }
    let Some(mut snap) = build_miner_snapshot(sim, rules, id) else {
        return;
    };
    harvest_mission_step(sim, rules, config, path_grid, overlay_registry, &mut snap);
    commit_miner_snapshot(sim, &snap, now);
}

/// One Harvest handler step: the miner FSM body, unchanged from the
/// pre-absorption `process_miner`.
pub(super) fn harvest_mission_step(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &super::MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    snap: &mut MinerSnapshot,
) {
    // Cursor sanity (debug-only, never hashed): the working cursor must have
    // decoded from the entity's handler state — pins the cursor round-trip the
    // substate-authority flip relies on.
    #[cfg(debug_assertions)]
    if let Some(entity) = sim.substrate.entities.get(snap.entity_id) {
        debug_assert_eq!(
            MinerState::from_cursor(entity.mission.handler_state()).unwrap_or(MinerState::SearchOre),
            snap.state,
            "Harvest dispatch entry: entity {} working cursor must equal the \
             decoded MissionCom.handler_state",
            snap.entity_id,
        );
    }

    process_miner(sim, rules, config, path_grid, overlay_registry, snap);
}
