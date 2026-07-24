//! Concrete Target/NavCom effects required by Mission wrapper transactions.
//!
//! The verified Techno/Foot wrappers archive intent around virtual setters, but
//! the complete category-specific setters are not implemented in Rust yet.
//! This sealed two-phase interface lets Mission authority prove availability
//! before its first write and then commit an infallible, ordered transaction.

use crate::sim::combat::TargetKind;
use crate::sim::components::NavTargetRef;
use crate::sim::world::Simulation;

mod private {
    pub trait Sealed {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcreteSetterRequest {
    Target {
        requested: Option<TargetKind>,
    },
    TargetAndDestination {
        requested_target: Option<TargetKind>,
        requested_destination: Option<NavTargetRef>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AuthorityUnavailable {
    #[error("exact concrete Target setter is unavailable for Mission receiver {0}")]
    TargetSetter(u64),
    #[error("exact mode-one destination setter is unavailable for Mission receiver {0}")]
    DestinationSetter(u64),
}

/// A complete concrete-effect provider.
///
/// `preflight` is read-only with respect to the simulation and validates the
/// entire requested setter chain.  Successful preflight guarantees the two
/// apply operations used by that request cannot fail.
pub(crate) trait ConcreteMissionEffects: private::Sealed {
    type Prepared;

    fn preflight(
        &mut self,
        sim: &Simulation,
        receiver: u64,
        request: ConcreteSetterRequest,
    ) -> Result<Self::Prepared, AuthorityUnavailable>;

    fn apply_target(
        &mut self,
        sim: &mut Simulation,
        prepared: &Self::Prepared,
        requested: Option<TargetKind>,
    );

    fn apply_destination_mode_one(
        &mut self,
        sim: &mut Simulation,
        prepared: &Self::Prepared,
        requested: Option<NavTargetRef>,
    );
}

/// Honest production boundary until full concrete Target and destination
/// setters are implemented.
#[derive(Debug, Default)]
pub(crate) struct UnavailableConcreteMissionEffects;

impl private::Sealed for UnavailableConcreteMissionEffects {}

impl ConcreteMissionEffects for UnavailableConcreteMissionEffects {
    type Prepared = ();

    fn preflight(
        &mut self,
        _sim: &Simulation,
        receiver: u64,
        request: ConcreteSetterRequest,
    ) -> Result<Self::Prepared, AuthorityUnavailable> {
        match request {
            ConcreteSetterRequest::Target { .. }
            | ConcreteSetterRequest::TargetAndDestination { .. } => {
                Err(AuthorityUnavailable::TargetSetter(receiver))
            }
        }
    }

    fn apply_target(
        &mut self,
        _sim: &mut Simulation,
        _prepared: &Self::Prepared,
        _requested: Option<TargetKind>,
    ) {
        unreachable!("unavailable provider cannot produce a concrete Target token")
    }

    fn apply_destination_mode_one(
        &mut self,
        _sim: &mut Simulation,
        _prepared: &Self::Prepared,
        _requested: Option<NavTargetRef>,
    ) {
        unreachable!("unavailable provider cannot produce a destination token")
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecordingPrepared {
    receiver: u64,
    request: ConcreteSetterRequest,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcreteEffectEvent {
    Preflight {
        receiver: u64,
        request: ConcreteSetterRequest,
    },
    Target {
        receiver: u64,
        requested: Option<TargetKind>,
        mission_current: super::MissionId,
        suspended_mission: super::MissionId,
        archived_target: Option<TargetKind>,
        archived_destination: Option<NavTargetRef>,
    },
    Destination {
        receiver: u64,
        requested: Option<NavTargetRef>,
        mission_current: super::MissionId,
        installed_target: Option<TargetKind>,
    },
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct RecordingConcreteMissionEffects {
    pub allow_target: bool,
    pub allow_destination: bool,
    pub events: Vec<ConcreteEffectEvent>,
}

#[cfg(test)]
impl RecordingConcreteMissionEffects {
    pub(crate) fn available() -> Self {
        Self {
            allow_target: true,
            allow_destination: true,
            events: Vec::new(),
        }
    }
}

#[cfg(test)]
impl private::Sealed for RecordingConcreteMissionEffects {}

#[cfg(test)]
impl ConcreteMissionEffects for RecordingConcreteMissionEffects {
    type Prepared = RecordingPrepared;

    fn preflight(
        &mut self,
        _sim: &Simulation,
        receiver: u64,
        request: ConcreteSetterRequest,
    ) -> Result<Self::Prepared, AuthorityUnavailable> {
        self.events
            .push(ConcreteEffectEvent::Preflight { receiver, request });
        match request {
            ConcreteSetterRequest::Target { .. } if !self.allow_target => {
                return Err(AuthorityUnavailable::TargetSetter(receiver));
            }
            ConcreteSetterRequest::TargetAndDestination { .. } => {
                if !self.allow_target {
                    return Err(AuthorityUnavailable::TargetSetter(receiver));
                }
                if !self.allow_destination {
                    return Err(AuthorityUnavailable::DestinationSetter(receiver));
                }
            }
            ConcreteSetterRequest::Target { .. } => {}
        }
        Ok(RecordingPrepared { receiver, request })
    }

    fn apply_target(
        &mut self,
        sim: &mut Simulation,
        prepared: &Self::Prepared,
        requested: Option<TargetKind>,
    ) {
        debug_assert!(match prepared.request {
            ConcreteSetterRequest::Target {
                requested: prepared_target,
            } => prepared_target == requested,
            ConcreteSetterRequest::TargetAndDestination {
                requested_target, ..
            } => requested_target == requested,
        });
        let entity = sim
            .substrate
            .entities
            .get_mut(prepared.receiver)
            .expect("preflight guaranteed receiver");
        self.events.push(ConcreteEffectEvent::Target {
            receiver: prepared.receiver,
            requested,
            mission_current: entity.mission.current(),
            suspended_mission: entity.mission.suspended(),
            archived_target: entity.suspended_attack_target,
            archived_destination: entity.navigation.suspended_nav_com,
        });
        if entity.attack_target.as_ref().map(|target| target.target) != requested {
            entity.attack_target = requested.map(|target| match target {
                TargetKind::Entity(id) => crate::sim::combat::AttackTarget::new(id),
                TargetKind::Cell(rx, ry) => crate::sim::combat::AttackTarget::for_cell(rx, ry),
            });
        }
    }

    fn apply_destination_mode_one(
        &mut self,
        sim: &mut Simulation,
        prepared: &Self::Prepared,
        requested: Option<NavTargetRef>,
    ) {
        debug_assert!(matches!(
            prepared.request,
            ConcreteSetterRequest::TargetAndDestination {
                requested_destination,
                ..
            } if requested_destination == requested
        ));
        let entity = sim
            .substrate
            .entities
            .get_mut(prepared.receiver)
            .expect("preflight guaranteed receiver");
        self.events.push(ConcreteEffectEvent::Destination {
            receiver: prepared.receiver,
            requested,
            mission_current: entity.mission.current(),
            installed_target: entity.attack_target.as_ref().map(|target| target.target),
        });
        entity.navigation.nav_com_aux = None;
        entity.navigation.nav_com = requested;
        entity.navigation.pending_arrival_clear = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_concrete_effects_never_claim_partial_setter_coverage() {
        let sim = Simulation::new();
        let mut effects = UnavailableConcreteMissionEffects;

        assert_eq!(
            effects.preflight(&sim, 7, ConcreteSetterRequest::Target { requested: None }),
            Err(AuthorityUnavailable::TargetSetter(7))
        );
        assert_eq!(
            effects.preflight(
                &sim,
                7,
                ConcreteSetterRequest::TargetAndDestination {
                    requested_target: None,
                    requested_destination: None,
                }
            ),
            Err(AuthorityUnavailable::TargetSetter(7))
        );
    }
}
