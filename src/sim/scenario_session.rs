//! Scenario session substrate — the launch descriptor the app layer feeds the
//! sim exactly once at construction.
//!
//! Mirrors the original engine fixing one per-match RNG seed before any
//! setup-phase draw, then seeding the scenario and main streams identically.
//! Data flows one-way app→sim; this module depends only on sim/ siblings.

use std::collections::BTreeMap;

use crate::sim::game_options::GameOptions;
use crate::sim::intern::InternedId;

/// Everything the app layer decides about a session before the sim exists.
/// Built from the lobby/launch flow and the selected map file — never
/// hardcoded inside sim/.
#[derive(Debug, Clone, Default)]
pub struct ScenarioDescriptor {
    /// The negotiated per-match seed. 32 bits wide because the original's
    /// negotiated seed is 32 bits and the RNG seeder consumes exactly 32; SP
    /// entropy, future MP handshake, and replay headers all funnel through
    /// this one field.
    pub seed: u32,
    /// Scenario identity: the selected map file name (lobby record / loading
    /// request), with the map's `[Basic]` Name as a human-facing fallback.
    pub map_name: String,
    /// Theater name from the map header (e.g. "TEMPERATE").
    pub theater: String,
    /// Full map `Size=` width/height (3rd/4th values) — authoritative bounds
    /// at load.
    pub map_width: u16,
    pub map_height: u16,
    /// Playable-area `LocalSize=` rect, stored verbatim.
    pub local_left: u16,
    pub local_top: u16,
    pub local_width: u16,
    pub local_height: u16,
    /// MP start waypoints (index -> cell) from the map `[Waypoints]` list.
    /// BTreeMap for deterministic iteration; sized by content, never by a
    /// player-count assumption.
    pub mp_start_waypoints: BTreeMap<u32, (u16, u16)>,
}

impl ScenarioDescriptor {
    /// Reconstruct the descriptor a recorded match was created from, so
    /// playback seeds the sim exactly as the original run did. Identity and
    /// bounds come from the same map-load path the original run used; only
    /// the seed and map name travel in the header.
    pub fn from_replay_header(header: &crate::sim::replay::ReplayHeader) -> Self {
        Self {
            seed: header.seed as u32,
            map_name: header.map_name.clone(),
            ..Self::default()
        }
    }
}

/// The sim-resident session aggregate. Owns session identity, the seed,
/// authoritative map bounds, the MP start table, the per-match options, and
/// the frame clock. Constructed once from the descriptor; serialized and
/// hashed (lockstep state, set before tick 0).
///
/// Bounds note: `Simulation.playfield_bounds` (the FNPC diamond lens over
/// `LocalSize`) keeps its own verbatim copy; consolidating the two is a
/// follow-up once the diamond consumers read through the session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioSession {
    /// Construction seed — the negotiated per-match value; the replay header
    /// records it. Stored widened (the negotiated value is 32-bit).
    pub seed: u64,
    /// Scenario identity: map file name (with `[Basic]` Name fallback).
    pub map_name: String,
    /// Theater name from the map header.
    pub theater: String,
    /// Full map `Size=` width/height.
    pub map_width: u16,
    pub map_height: u16,
    /// Playable-area `LocalSize=` rect, stored verbatim.
    pub local_left: u16,
    pub local_top: u16,
    pub local_width: u16,
    pub local_height: u16,
    /// MP start waypoints (index -> cell) from the map `[Waypoints]` list.
    pub mp_start_waypoints: BTreeMap<u32, (u16, u16)>,
    /// Start waypoint index -> owning house, filled during launch application
    /// (after the random-assignment draws), before tick 0.
    pub start_slot_houses: BTreeMap<u32, InternedId>,
    /// Per-match game settings (the lobby options card). Set once at game
    /// start, read-only during gameplay.
    pub game_options: GameOptions,
    /// Current simulation tick (starts at 0, increments after each
    /// advance_tick).
    pub tick: u64,
    /// Total accumulated sim-tick milliseconds since world creation.
    /// Authoritative time source; `binary_frame` is derived from this.
    pub total_sim_ms: u64,
    /// Synthetic 15 Hz frame counter, **committed late** at the end of
    /// advance_tick beside `tick` — during a tick it holds the previous
    /// tick's committed value (the pre-increment frame this tick executes
    /// under). Read it as the *current* frame for stored-start timer
    /// consumers; never as the next frame.
    pub binary_frame: u32,
}

impl ScenarioSession {
    pub fn from_descriptor(desc: &ScenarioDescriptor) -> Self {
        Self {
            seed: u64::from(desc.seed),
            map_name: desc.map_name.clone(),
            theater: desc.theater.clone(),
            map_width: desc.map_width,
            map_height: desc.map_height,
            local_left: desc.local_left,
            local_top: desc.local_top,
            local_width: desc.local_width,
            local_height: desc.local_height,
            mp_start_waypoints: desc.mp_start_waypoints.clone(),
            start_slot_houses: BTreeMap::new(),
            game_options: GameOptions::default(),
            tick: 0,
            total_sim_ms: 0,
            binary_frame: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world::Simulation;

    #[test]
    fn from_descriptor_equals_with_seed_widened() {
        let a = Simulation::from_descriptor(&ScenarioDescriptor {
            seed: 0xDEAD_BEEF,
            ..Default::default()
        });
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
            let mut sim = Simulation::from_descriptor(&ScenarioDescriptor {
                seed,
                ..Default::default()
            });
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

    /// AT-5: authoritative bounds are queryable before any advance_tick — no
    /// zero-dim fog window between construction and the first vision pass.
    #[test]
    fn map_bounds_known_before_first_tick() {
        let desc = ScenarioDescriptor {
            seed: 7,
            map_width: 80,
            map_height: 60,
            ..Default::default()
        };
        let sim = Simulation::from_descriptor(&desc);
        assert_eq!((sim.fog.width, sim.fog.height), (80, 60));
        assert_eq!((sim.session.map_width, sim.session.map_height), (80, 60));
    }

    /// AT-4: scenario identity (map name, theater) is sim-resident and
    /// survives a snapshot round-trip.
    #[test]
    fn scenario_identity_is_sim_resident() {
        let desc = ScenarioDescriptor {
            seed: 9,
            map_name: "tournamentb.map".into(),
            theater: "SNOW".into(),
            map_width: 100,
            map_height: 100,
            mp_start_waypoints: [(0u32, (10u16, 12u16)), (1, (88, 90))].into_iter().collect(),
            ..Default::default()
        };
        let sim = Simulation::from_descriptor(&desc);
        let bytes = crate::sim::snapshot::GameSnapshot::save(&sim, 1, 2, "tournamentb.map", 0);
        let restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("snapshot load")
            .sim;
        assert_eq!(restored.session.map_name, "tournamentb.map");
        assert_eq!(restored.session.theater, "SNOW");
        assert_eq!(
            restored.session.mp_start_waypoints,
            sim.session.mp_start_waypoints
        );
        assert_eq!(restored.state_hash(), sim.state_hash());
    }

    /// AT-6: the MP start-waypoint table is hashed lockstep state — a one-cell
    /// difference diverges the desync detector — and round-trips save/load.
    #[test]
    fn mp_waypoints_round_trip_and_hash() {
        let mut desc = ScenarioDescriptor {
            seed: 11,
            map_width: 64,
            map_height: 64,
            mp_start_waypoints: [(0u32, (5u16, 5u16)), (1, (50, 50))].into_iter().collect(),
            ..Default::default()
        };
        let a = Simulation::from_descriptor(&desc);
        desc.mp_start_waypoints.insert(1, (50, 51)); // one waypoint, one cell off
        let b = Simulation::from_descriptor(&desc);
        assert_ne!(
            a.state_hash(),
            b.state_hash(),
            "a one-cell waypoint difference must be visible to the desync detector"
        );

        let bytes = crate::sim::snapshot::GameSnapshot::save(&a, 1, 2, "wp", 0);
        let restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("snapshot load")
            .sim;
        assert_eq!(restored.state_hash(), a.state_hash());
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
