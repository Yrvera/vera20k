//! Scenario session substrate — the launch descriptor the app layer feeds the
//! sim exactly once at construction.
//!
//! Mirrors the original engine fixing one per-match RNG seed before any
//! setup-phase draw, then seeding the scenario and main streams identically.
//! Data flows one-way app→sim; this module depends only on sim/ siblings.

/// Everything the app layer decides about a session before the sim exists.
/// Built from the lobby/launch flow — never hardcoded inside sim/.
#[derive(Debug, Clone, Default)]
pub struct ScenarioDescriptor {
    /// The negotiated per-match seed. 32 bits wide because the original's
    /// negotiated seed is 32 bits and the RNG seeder consumes exactly 32; SP
    /// entropy, future MP handshake, and replay headers all funnel through
    /// this one field.
    pub seed: u32,
}

impl ScenarioDescriptor {
    /// Reconstruct the descriptor a recorded match was created from, so
    /// playback seeds the sim exactly as the original run did.
    pub fn from_replay_header(header: &crate::sim::replay::ReplayHeader) -> Self {
        Self {
            seed: header.seed as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world::Simulation;

    #[test]
    fn from_descriptor_equals_with_seed_widened() {
        let a = Simulation::from_descriptor(&ScenarioDescriptor { seed: 0xDEAD_BEEF });
        let b = Simulation::with_seed(0xDEAD_BEEF);
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn from_replay_header_roundtrips_u32_seed() {
        let header = crate::sim::replay::ReplayHeader {
            version: 1,
            tick_hz: 15,
            seed: 0x1234_5678,
            map_name: String::new(),
            rules_hash: 0,
        };
        assert_eq!(
            ScenarioDescriptor::from_replay_header(&header).seed,
            0x1234_5678
        );
    }
}
