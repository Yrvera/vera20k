//! Game simulation — EntityStore + GameEntity, fixed-point math, deterministic logic.
//!
//! ALL simulation math uses fixed-point arithmetic (I32F16) — never floats.
//! This is a day-one decision for deterministic multiplayer: identical results
//! across different machines regardless of CPU floating-point behavior.
//!
//! The simulation is data-driven: all unit stats, weapon damage, build times
//! come from RuleSet (parsed from rules.ini). Sim code contains pure logic,
//! never hardcoded game balance numbers.
//!
//! ## Key types
//! - `Simulation` — owns EntityStore (BTreeMap<u64, GameEntity>), ticks game state forward
//! - `GameEntity` — unified plain struct (position, health, owner, locomotor, etc.)
//! - Systems: movement, combat, harvesting, production, pathfinding
//!
//! ## Dependency rules â€" THIS IS THE #1 ARCHITECTURAL INVARIANT
//! - sim/ depends on: rules/, map/
//! - sim/ NEVER depends on: render/, ui/, sidebar/, audio/, net/
//! - This isolation is what makes alternative views possible (Commander mode,
//!   spectator view, headless server) without touching sim code.
//! - sim/ receives commands but never calls into presentation modules.

// --- Core types: entity storage, components, commands, RNG, interning ---
pub mod anim_class;
pub mod capture_manager;
pub mod cloak_disguise;
pub mod command;
pub mod components;
pub mod crates; // scenario-start crate placement (Post_Map_Init step 3)
pub mod economy; // per-house wallet/storage/statistics value-type (production+economy substrate)
pub mod entity_store;
pub mod game_entity;
pub mod intern;
pub(crate) mod lifecycle_request;
pub mod multiplayer_checksum;
pub mod rng;
pub(crate) mod score;
pub(crate) mod scenario_bootstrap;
pub(crate) mod scenario_post_map;
pub mod scenario_session; // app->sim launch descriptor (per-match seed pipeline)
pub mod timer; // signed frame-anchored countdown primitive
pub mod type_handle_table; // InternedId -> TypeHandle, one-hop entity->type resolution

// --- Pure read-only deterministic engine-data services (gamemd-exact lookup tables) ---
pub mod substrate; // direction/facing tables; no render/ui/audio/net dep

// --- Subsystem folders (multi-file subsystems with internal mod.rs) ---
pub mod bounce;
pub mod combat; // targeting, weapons, AOE, fire gates, damage resolution
pub mod miner; // harvester state machine, dock sequences, ore delivery
pub mod production; // build queue, placement, economy, tech tree, selling
pub mod voxel_anim;
pub mod world; // Simulation orchestrator, command dispatch, spawn, hash

// --- Mission scheduler substrate + radio contact RPC vocabulary ---
pub mod mission; // MissionType, MissionTimer, MissionControl (INI table)
pub mod radio; // RadioMessage / RadioResponse / RadioPayload

// --- Movement: ground pathing, speed ramping, cell transitions,
//     special locomotors, drive tracks, turret rotation ---
pub mod movement;
pub mod pathfinding; // A* search, zone connectivity, terrain costs, path smoothing
pub mod projectile; // serialized BulletClass-style flight state and detonation handoff
pub mod wave; // serialized WaveClass-style display registrations and lifetime

// --- Docking: repair depots and airfield landing pads ---
pub mod docking;

// --- Aircraft mission state machines (attack runs, guard, RTB, idle) ---
pub mod aircraft;

// --- Vision, fog of war, power ---
pub mod power_system;
pub mod superweapon;
pub mod vision;

// --- Animation, building overlays, bridge state ---
pub mod animation;
pub mod bridge_specs;
pub mod bridge_state;

// --- Infantry deploy-fire state machine ---
pub mod deploy;
pub mod gate_runtime;
pub mod infantry;

// --- Persistent cell occupancy ---
pub mod cell_kernel;
pub mod cell_rect;
pub mod find_nearby_cell;
pub mod occupancy;

// --- Map/cell substrate (read-only services over the canonical cell store) ---
pub mod map; // bridge topology service (first member of the map/cell-substrate workstream)

// --- Mutable per-cell overlay state (ore density, wall damage, bridge frames) ---
pub mod overlay_grid;

// --- Mutable per-cell smudge state (craters, scorches, pre-placed map decals) ---
pub mod smudge_grid;

// --- Per-cell radiation field (sites, spread/decay; damage applied by combat) ---
pub mod radiation;

// --- Passengers, transport, slaves ---
pub mod parity_digest;
pub mod passenger;
pub mod slave_miner;
pub mod spawn_manager;
mod spawn_manager_tests;

// --- Economy, map resources ---
pub mod ore_growth;
pub mod radar;
pub mod rocking;
pub mod terrain_object;
pub mod terrain_spawn;
pub mod tiberium;

// --- Per-match settings, per-player state ---
pub mod game_options;
pub mod house_state;

// --- Trigger runtime (map trigger evaluation during gameplay) ---
pub mod team_script_vm;
pub mod trigger_runtime;

// --- AI, replay, selection, debug ---
pub mod ai;
pub mod debug_event_log;
pub mod runtime;
pub mod replay;
#[cfg(test)]
mod replay_determinism_tests;
pub mod selection;
// GPU-independent HVA frame-count catalog shared with the renderer's atlas seeding (F09).
pub mod voxel_frame_catalog;

// --- Snapshot serialization (mid-match save/load) ---
pub mod snapshot;
#[cfg(test)]
mod stable_identity_tests;

// --- Particle systems (visual + damage particle effects: smoke, gas, fire) ---
pub mod particles;

#[cfg(test)]
#[path = "deploy_tests.rs"]
mod deploy_tests;
