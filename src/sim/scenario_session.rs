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

    /// AT-1: two sims constructed from the same descriptor seed stay in
    /// per-stream lockstep across 300 ticks of identical commands; a
    /// different seed diverges. Proves seed injection reaches all three
    /// streams before the first tick.
    #[test]
    fn mp_sibling_rng_state_matches_after_seed_sync() {
        use crate::map::entities::{EntityCategory, MapEntity};
        use crate::sim::command::{Command, CommandEnvelope};
        use std::collections::BTreeMap;

        fn build(seed: u32) -> Simulation {
            let mut sim = Simulation::from_descriptor(&ScenarioDescriptor { seed });
            let entity = MapEntity {
                owner: "Americans".to_string(),
                type_id: "MTNK".to_string(),
                health: 256,
                cell_x: 2,
                cell_y: 2,
                facing: 64,
                category: EntityCategory::Unit,
                sub_cell: 0,
                veterancy: 0,
                high: false,
            };
            sim.spawn_from_map(&[entity], None, &BTreeMap::new());
            sim
        }
        fn run_300(sim: &mut Simulation) -> Vec<u64> {
            let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
            let owner = sim.interner.get("Americans").expect("owner interned");
            (0..300u64)
                .map(|t| {
                    let cmds = if t == 5 {
                        vec![CommandEnvelope::new(
                            owner,
                            6,
                            Command::Move {
                                entity_id: 1,
                                target_rx: 20,
                                target_ry: 2,
                                queue: false,
                                group_id: None,
                            },
                        )]
                    } else {
                        Vec::new()
                    };
                    sim.advance_tick(&cmds, None, &heights, None, None, 67).state_hash
                })
                .collect()
        }

        let (mut a, mut b) = (build(0xA5EED), build(0xA5EED));
        assert_eq!(
            run_300(&mut a),
            run_300(&mut b),
            "same descriptor seed must produce an identical hash timeline"
        );
        assert_eq!(a.scenario_rng.state(), b.scenario_rng.state());
        assert_eq!(a.main_rng.state(), b.main_rng.state());
        assert_eq!(a.mapgen_rng.state(), b.mapgen_rng.state());

        let mut c = build(0xA5EED + 1);
        run_300(&mut c);
        assert_ne!(
            a.state_hash(),
            c.state_hash(),
            "different descriptor seeds must diverge"
        );
        assert_ne!(a.scenario_rng.state(), c.scenario_rng.state());
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
