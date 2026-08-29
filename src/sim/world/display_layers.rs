//! Persistent native flat display-layer submission order.
//!
//! `DisplayClass::Submit_Object @ 0x004A9720` removes an object's prior
//! registration and appends it to unsorted layers 3/4. Those vectors are
//! independent from LogicClass and survive save/load through
//! `DisplayClass::Save @ 0x004AE720`. Ground (layer 2) remains owned by the
//! existing integer Y-sort planner; this state retains its membership only so
//! a later transition into Air/Top can be distinguished from an in-layer no-op.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::game_entity::GameEntity;
use crate::util::fixed_math::SIM_ZERO;

pub(crate) const NATIVE_GROUND_LAYER: u8 = 2;
pub(crate) const NATIVE_AIR_LAYER: u8 = 3;
pub(crate) const NATIVE_TOP_LAYER: u8 = 4;

/// Exact display-layer number returned by the active locomotor.
///
/// gamemd-derived: Fly `In_Which_Layer @ 0x004CFCF0`; Jumpjet
/// `In_Which_Layer @ 0x0054B8D0`; Rocket is Air. A parachuting infantryman
/// retains Walk and therefore remains Ground.
pub(crate) fn entity_display_layer(entity: &GameEntity) -> u8 {
    if entity.parachute_state.is_some() {
        return NATIVE_GROUND_LAYER;
    }
    if entity.rocket_state.is_some() {
        return NATIVE_AIR_LAYER;
    }
    let Some(locomotor) = entity.locomotor.as_ref() else {
        return NATIVE_GROUND_LAYER;
    };
    match locomotor.kind {
        LocomotorKind::Rocket => NATIVE_AIR_LAYER,
        LocomotorKind::Fly if locomotor.altitude > SIM_ZERO => NATIVE_TOP_LAYER,
        LocomotorKind::Jumpjet if locomotor.altitude > SIM_ZERO => {
            if locomotor.altitude >= locomotor.target_altitude {
                NATIVE_TOP_LAYER
            } else {
                NATIVE_AIR_LAYER
            }
        }
        _ => NATIVE_GROUND_LAYER,
    }
}

/// Saved membership plus the two native unsorted vectors this renderer must
/// consume verbatim. Layer 2's vector is reconstructible from exact Y-sort;
/// only its membership is retained here to detect 2 -> 3/4 transitions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct FlatDisplayOrder {
    #[serde(default)]
    membership: BTreeMap<u64, u8>,
    #[serde(default)]
    air: Vec<u64>,
    #[serde(default)]
    top: Vec<u64>,
}

impl FlatDisplayOrder {
    /// Native remove-then-submit. Stable-layer ObjectClass AI does not call
    /// this; a genuine transition does, so a returning object moves to tail.
    pub(crate) fn submit(&mut self, object_id: u64, layer: u8) {
        self.remove_from_flat_vector(object_id);
        self.membership.insert(object_id, layer);
        match layer {
            NATIVE_AIR_LAYER => self.air.push(object_id),
            NATIVE_TOP_LAYER => self.top.push(object_id),
            _ => {}
        }
    }

    /// Submit only when the object's current answer changed, matching the
    /// `ObjectClass::AI @ 0x005F3E70` old/new layer comparison.
    pub(crate) fn transition(&mut self, object_id: u64, layer: u8) -> bool {
        if self.membership.get(&object_id).copied() == Some(layer) {
            return false;
        }
        if !self.membership.contains_key(&object_id) {
            return false;
        }
        self.submit(object_id, layer);
        true
    }

    pub(crate) fn remove(&mut self, object_id: u64) -> bool {
        let removed = self.membership.remove(&object_id).is_some();
        self.remove_from_flat_vector(object_id);
        removed
    }

    pub(crate) fn layer_order(&self, layer: u8) -> &[u64] {
        match layer {
            NATIVE_AIR_LAYER => &self.air,
            NATIVE_TOP_LAYER => &self.top,
            _ => &[],
        }
    }

    pub(crate) fn hash_state(&self, hasher: &mut impl Hasher) {
        if self.membership.is_empty() {
            return;
        }
        b"flat-display-order-v1".hash(hasher);
        self.membership.hash(hasher);
        self.air.hash(hasher);
        self.top.hash(hasher);
    }

    pub(crate) fn validate(
        &self,
        identities: &BTreeMap<u64, &'static str>,
    ) -> Result<(), crate::sim::snapshot::SnapshotRestoreError> {
        use crate::sim::snapshot::SnapshotRestoreError;

        let mut seen = std::collections::BTreeSet::new();
        for (layer, order) in [
            (NATIVE_AIR_LAYER, self.air.as_slice()),
            (NATIVE_TOP_LAYER, self.top.as_slice()),
        ] {
            for &object_id in order {
                if !seen.insert(object_id) {
                    return Err(SnapshotRestoreError::DuplicateDisplayLayerIdentity {
                        object_id,
                    });
                }
                if !identities.contains_key(&object_id) {
                    return Err(SnapshotRestoreError::MissingDisplayLayerIdentity {
                        object_id,
                    });
                }
                let stored_layer = self.membership.get(&object_id).copied();
                if stored_layer != Some(layer) {
                    return Err(SnapshotRestoreError::DisplayLayerMembershipMismatch {
                        object_id,
                        vector_layer: layer,
                        stored_layer,
                    });
                }
            }
        }
        for (&object_id, &stored_layer) in &self.membership {
            if !identities.contains_key(&object_id) {
                return Err(SnapshotRestoreError::MissingDisplayLayerIdentity {
                    object_id,
                });
            }
            if !matches!(
                stored_layer,
                NATIVE_GROUND_LAYER | NATIVE_AIR_LAYER | NATIVE_TOP_LAYER
            ) {
                return Err(SnapshotRestoreError::DisplayLayerMembershipMismatch {
                    object_id,
                    vector_layer: u8::MAX,
                    stored_layer: Some(stored_layer),
                });
            }
            let occurrences = usize::from(self.air.contains(&object_id))
                + usize::from(self.top.contains(&object_id));
            let expected = usize::from(matches!(stored_layer, NATIVE_AIR_LAYER | NATIVE_TOP_LAYER));
            if occurrences != expected {
                return Err(SnapshotRestoreError::DisplayLayerMembershipMismatch {
                    object_id,
                    vector_layer: if self.air.contains(&object_id) {
                        NATIVE_AIR_LAYER
                    } else if self.top.contains(&object_id) {
                        NATIVE_TOP_LAYER
                    } else {
                        u8::MAX
                    },
                    stored_layer: Some(stored_layer),
                });
            }
        }
        Ok(())
    }

    fn remove_from_flat_vector(&mut self, object_id: u64) {
        if let Some(index) = self.air.iter().position(|&id| id == object_id) {
            self.air.remove(index);
        }
        if let Some(index) = self.top.iter().position(|&id| id == object_id) {
            self.top.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_removes_and_reappends_at_the_destination_tail() {
        let mut order = FlatDisplayOrder::default();
        order.submit(1, NATIVE_GROUND_LAYER);
        order.submit(2, NATIVE_GROUND_LAYER);
        assert!(order.transition(2, NATIVE_TOP_LAYER));
        assert!(order.transition(1, NATIVE_TOP_LAYER));
        assert_eq!(order.layer_order(NATIVE_TOP_LAYER), &[2, 1]);
        assert!(!order.transition(1, NATIVE_TOP_LAYER));
        assert_eq!(order.layer_order(NATIVE_TOP_LAYER), &[2, 1]);

        assert!(order.transition(2, NATIVE_GROUND_LAYER));
        assert!(order.transition(2, NATIVE_TOP_LAYER));
        assert_eq!(order.layer_order(NATIVE_TOP_LAYER), &[1, 2]);
    }
}
