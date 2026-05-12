//! Infantry deploy-fire state machine.
//!
//! Models the sim-authoritative phase: Deploying → Deployed → Undeploying → None.
//! The animation system reads `entity.deploy_state` and reflects the visual
//! sequence (Deploy / Deployed / DeployedFire / Undeploy). `DeployedFire` is
//! not a sim phase — it's a visual sub-state of `Deployed` driven by
//! `attack_target.is_some()` (existing tick_animations auto-transition).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::entity_store::EntityStore;

/// Default deploy/undeploy duration in sim ticks when the per-type art.ini
/// frame count cannot be resolved from this scope.
///
/// At SIM_TICK_MS=22, 55 ticks ≈ 1210ms. Sized to roughly match stock GI
/// Deploy (15 frames × ~80ms/frame ≈ 1200ms), so when the sim phase advances
/// to Deployed and the animation cascade switches the sequence, the visual
/// Deploy animation has just about completed. Without this sizing, sim phase
/// transitions ahead of the art.ini-driven visual and `tick_animations`
/// truncates the Deploy sequence mid-playback. Per-type precise lookup is
/// deferred — see plan Open Questions.
pub(crate) const DEPLOY_DEFAULT_TICKS: u16 = 55;

/// Sim-authoritative deploy phase for an entity.
///
/// `None` on `GameEntity.deploy_state` means upright (default). Any `Some(_)`
/// variant gates the Set_Destination early-return — deployed units silently
/// ignore Move/AttackMove/Enter/etc. until explicitly undeployed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DeployPhase {
    /// Deploy animation playing — sim ticks count down to Deployed.
    Deploying { ticks_remaining: u16 },
    /// Stationary in deployed stance. Visual flips to DeployedFire when
    /// `attack_target.is_some()` (existing tick_animations auto-transition).
    Deployed,
    /// Undeploy animation playing — sim ticks count down to None.
    Undeploying { ticks_remaining: u16 },
}

/// Resolve the number of sim ticks the deploy or undeploy phase should run.
///
/// B1 returns the constant fallback regardless of input — see the doc on
/// `DEPLOY_DEFAULT_TICKS` for the rationale and the deferred follow-up.
pub(crate) fn compute_anim_ticks() -> u16 {
    DEPLOY_DEFAULT_TICKS
}

/// Advance every entity's `deploy_state` by one tick.
///
/// `Deploying { N }` → `Deploying { N-1 }` until N == 1, then promotes to
/// `Deployed`. `Undeploying { N }` follows the same shape, ending at `None`.
pub fn tick_deploy_state(entities: &mut EntityStore) {
    let keys = entities.keys_sorted();
    for id in keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        match entity.deploy_state {
            Some(DeployPhase::Deploying { ticks_remaining }) => {
                if ticks_remaining > 1 {
                    entity.deploy_state = Some(DeployPhase::Deploying {
                        ticks_remaining: ticks_remaining - 1,
                    });
                } else {
                    entity.deploy_state = Some(DeployPhase::Deployed);
                }
            }
            Some(DeployPhase::Undeploying { ticks_remaining }) => {
                if ticks_remaining > 1 {
                    entity.deploy_state = Some(DeployPhase::Undeploying {
                        ticks_remaining: ticks_remaining - 1,
                    });
                } else {
                    entity.deploy_state = None;
                }
            }
            Some(DeployPhase::Deployed) | None => {}
        }
    }
}
