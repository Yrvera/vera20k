//! Game simulation - owns the EntityStore and deterministic tick stepping.
//!
//! The Simulation is the authoritative game state. It spawns entities from
//! map data, executes command envelopes on fixed ticks, advances gameplay
//! systems, and exposes deterministic state hashing for replay/desync checks.
//!
//! Implementation is split across sibling files for size:
//! - `world_commands.rs` — command dispatch and selection/ownership helpers
//! - `world_hash.rs` — deterministic state hashing
//! - `world_spawn.rs` — entity spawning from map data and production
//! - `world_orders.rs` — order-intent tick systems (attack-move, guard, area-guard)

pub(crate) mod bridge_orchestrator;
pub mod edge_cell;
mod logic_vector;
mod substrate;
mod techno_ai;
pub(crate) mod unit_post;
mod world_commands;
mod world_hash;
mod world_orders;
mod world_spawn;

pub(crate) use logic_vector::LogicVector;
pub(crate) use substrate::ObjectSubstrate;
pub use substrate::EnterOrderCounter;

use std::collections::{BTreeMap, BTreeSet};

use crate::map::actions::ActionMap;
use crate::map::bridge_facts::{BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_STRUCTURAL};
use crate::map::entities::EntityCategory;
use crate::map::events::EventMap;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::rules::locomotor_type::SpeedType;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::ai::{self, AiPlayerState};
use crate::sim::bridge_state::{BridgeRuntimeState, DamageState};
use crate::sim::combat;
use crate::sim::combat::combat_weapon::WeaponSlot;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::WorldEffect;
use crate::sim::docking::aircraft_dock;
use crate::sim::docking::building_dock;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::Presence;
use crate::sim::game_options::GameOptions;
use crate::sim::house_state::HouseState;
use crate::sim::intern::InternedId;
use crate::sim::movement;
use crate::sim::movement::air_movement;
use crate::sim::movement::droppod_movement;
use crate::sim::movement::homing_movement;
use crate::sim::movement::parachute_descent;
use crate::sim::movement::rocket_movement;
use crate::sim::movement::teleport_movement;
use crate::sim::movement::tunnel_movement;
use crate::sim::movement::turret;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid, entity_occupancy_cells};
use crate::sim::ore_growth;
use crate::sim::overlay_grid::{WallDamageEvent, cleanup_wall_neighbors, damage_wall_overlay};
use crate::sim::particles::ParticleSystemStore;
use crate::sim::passenger;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::pathfinding::terrain_speed;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::sim::power_system::{self, PowerState};
use crate::sim::production::{self, ProductionState};
use crate::sim::radar::{RadarEventQueue, RadarEventType};
use crate::sim::replay::ReplayLog;
use crate::sim::rng::SimRng;
use crate::sim::trigger_runtime::{TriggerEffect, TriggerRuntime};
use crate::sim::vision::{self, FogState};
use crate::util::fixed_math::SimFixed;

/// Default deterministic RNG seed for ad-hoc simulation instances.
const DEFAULT_SIM_SEED: u64 = 0x5EED_CAFE_D15E_A5E5;

/// Result of one deterministic simulation tick.
#[derive(Debug, Clone, Copy)]
pub struct TickResult {
    pub tick: u64,
    pub executed_commands: usize,
    pub state_hash: u64,
    pub spawned_entities: bool,
    /// A structure was destroyed (combat, sell, crush) — PathGrid needs rebuild
    /// to unblock the footprint.
    pub destroyed_structure: bool,
    /// An entity's owner changed (garrison reconciliation, engineer capture) — sprite
    /// atlas needs rebuild for the new house color.
    pub ownership_changed: bool,
    /// A bridge cell transitioned to `DamageState::Destroyed` this tick —
    /// PathGrid needs rebuild so A* sees collapsed cells as non-traversable
    /// starting next tick. Matches gamemd's one-tick-delayed visibility.
    pub bridge_state_changed: bool,
    pub movement: movement::MovementTickStats,
    /// Debug/test-only S2a measurement: how many live non-miner Units had their
    /// dispatch family differ between host-time (top-of-tick, the gamemd-faithful
    /// dispatch input) and the end-of-tick re-derivation this tick. Always 0 in
    /// release (the proof that fills it is debug/test only). NOT hashed, NOT
    /// serialized — pure instrumentation for the S2 go/no-go churn signal.
    pub dispatch_churn: u32,
}

/// A sound event produced during simulation (combat, death, production).
/// Pure data — no audio library dependency. Drained by the app layer each frame.
#[derive(Debug, Clone)]
pub enum SimSoundEvent {
    /// A weapon fired — play its Report= sound.
    WeaponFired {
        report_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// An entity was destroyed — play its DieSound=.
    EntityDied {
        die_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// An entity was crushed by a vehicle — play its CrushSound= (the squish).
    /// Crush kills also emit `EntityDied` for the death cry — these are
    /// independent audio events that play together (matches gamemd).
    EntityCrushed {
        crush_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// An infantry entity entered the Deploying phase — play its DeploySound=.
    EntityDeployed {
        deploy_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// An infantry entity entered the Undeploying phase — play its UndeploySound=.
    EntityUndeployed {
        undeploy_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// A miner docked at a refinery — play the building's deploy sound.
    /// The app layer should select the healthy or damaged sound variant
    /// based on the refinery's health ratio vs ConditionYellow.
    DockDeploy { building_id: u64 },
    /// A building finished construction — play EVA "Construction complete".
    BuildingComplete { owner: InternedId },
    /// A unit finished training — play EVA "Unit ready".
    UnitComplete { owner: InternedId },
    /// A deploy command failed target placement validation.
    /// App layer gates this to the local human player and plays
    /// `EVA_CannotDeployHere`.
    CannotDeployHere { owner: InternedId },
    /// A chrono teleport happened — play the resolved warp sound at this position.
    /// Sim emits two of these per warp: one at the source cell with the unit's
    /// `ChronoOutSound=`, one at the destination cell with the unit's
    /// `ChronoInSound=`.
    ChronoTeleport {
        sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// A superweapon was launched — play EVA warning.
    SuperWeaponLaunched { owner: InternedId, rx: u16, ry: u16 },
    /// A lightning bolt struck — play thunder sound.
    SuperWeaponStrike { rx: u16, ry: u16 },
    /// First occupant entered a CanBeOccupied building (cargo 0→1).
    /// Owner is the building owner at AddGarrisonOccupant time; civilian
    /// ownership transfer is reported separately from building reconciliation.
    /// App layer plays EVA_StructureGarrisoned if owner is local human.
    StructureGarrisoned { owner: InternedId },
    /// Last occupant left a garrisoned building (cargo 1→0).
    /// Owner is the **pre-revert** owner — the player whose garrison
    /// just emptied. Matches gamemd's CheckAutoSellOrCivilian which
    /// fires EVA before ChangeOwner. App layer plays EVA_StructureAbandoned
    /// if owner is local human.
    StructureAbandoned { owner: InternedId },
    /// First-occupant SFX from rulesmd [AudioVisual] BuildingGarrisonedSound.
    /// Positional cue gated on owner == local human.
    BuildingGarrisonedSfx { owner: InternedId, rx: u16, ry: u16 },
    /// SFX for conditional reciprocal-link harvester release. Resolved at
    /// the app layer to [AudioVisual] BunkerWallsDownSound (retail value
    /// "TankBunkerDown"). Stock zero-link refinery unload completion does
    /// not emit this event.
    RefineryExitSfx { rx: u16, ry: u16 },
    /// Tank-bunker walls-up cue — emitted on install. App resolves to
    /// [AudioVisual] BunkerWallsUpSound (retail "TankBunkerUp").
    BunkerWallsUp { rx: u16, ry: u16 },
    /// Tank-bunker walls-down cue — emitted on normal exit / clear teardown.
    /// App resolves to [AudioVisual] BunkerWallsDownSound (retail "TankBunkerDown").
    BunkerWallsDown { rx: u16, ry: u16 },
    /// A paratrooper was dropped from a carrier aircraft.
    /// Played at the drop position; app layer resolves to [AudioVisual] ChuteSound.
    ChuteSound { rx: u16, ry: u16 },
    /// A C4-capable infantry claimed a plant on a CanC4 building.
    /// Played at the attacker's position. App resolves to
    /// `[SealPlaceBomb]` in soundmd.ini.
    C4Planted { rx: u16, ry: u16 },
    /// An engineer entered a `BridgeRepairHut` and triggered bridge repair.
    /// Played at the BUILDING's cell, NOT the engineer's. `owner` is the
    /// engineer's house — app layer plays `EVA_BridgeRepaired` only if
    /// `owner` is the local human player. App layer plays the spatial
    /// `[BridgeRepaired]` sound for everyone in range, gated on
    /// `rules.bridge_rules.repair_sound.is_some()`. `eva_allowed` is the
    /// result of gamemd's non-drawing radar event creation/dedup gate.
    BridgeRepaired {
        rx: u16,
        ry: u16,
        owner: InternedId,
        eva_allowed: bool,
    },
    /// A delayed world-effect animation reached its first active frame.
    WorldEffectStarted {
        sound_id: InternedId,
        rx: u16,
        ry: u16,
        sub_x: SimFixed,
        sub_y: SimFixed,
        z: u8,
    },
}

/// A fire event produced during combat — carries firing-tick facts for
/// render-side muzzle flash positioning and future projectile origin computation.
///
/// The sim emits this whenever a weapon fires. Non-garrison fields snapshot
/// the selected weapon, attacker type/facing/veterancy, and optional report
/// sound id at the authoritative fire tick. Garrison fields remain
/// fire-port/occupant-specific so the app layer can keep the existing
/// `OccupantAnim` path separate.
/// Source position facts captured at the authoritative fire tick.
///
/// The app layer combines this deterministic snapshot with art/rules metadata
/// to resolve the visible muzzle, projectile, and report-sound origin.
#[derive(Debug, Clone)]
pub struct FireOriginSnapshot {
    pub rx: u16,
    pub ry: u16,
    pub sub_x: SimFixed,
    pub sub_y: SimFixed,
    pub z: u8,
    pub facing: u8,
    pub category: EntityCategory,
    /// Pre-shot burst index. First shot in a burst is 0.
    pub burst_index: u8,
}

#[derive(Debug, Clone)]
pub struct SimFireEvent {
    /// Stable ID of the entity that fired.
    pub attacker_id: u64,
    /// Type id of the firing object at the fire tick.
    pub attacker_type_ref: InternedId,
    /// Which weapon slot was used (Primary or Secondary).
    pub weapon_slot: WeaponSlot,
    /// Selected weapon section id.
    pub weapon_id: InternedId,
    /// Firing object's facing at the fire tick.
    pub facing: u8,
    /// Firing object's veterancy at the fire tick.
    pub veterancy: u16,
    /// Source facts from the fire tick, before burst/cooldown updates.
    pub origin_snapshot: FireOriginSnapshot,
    /// What was fired at — entity stable ID or ground cell coord.
    /// For projectile trajectory: Entity → look up entity position; Cell →
    /// use cell center as the destination.
    pub target: crate::sim::combat::TargetKind,
    /// Weapon report sound id. The app layer positions this at the resolved
    /// fire origin for both normal and garrison fire.
    pub report_sound_id: Option<InternedId>,
    /// For garrison fire: which muzzle port index fired (for fire port positioning).
    /// None = normal weapon FLH, Some(idx) = garrison fire port index.
    pub garrison_muzzle_index: Option<u8>,
    /// For garrison fire: the weapon's OccupantAnim interned ID (e.g., "UCFLASH").
    /// Pushed through the event so the render layer doesn't need to re-derive the weapon.
    pub occupant_anim: Option<InternedId>,
}

/// The game simulation - owns all authoritative game state.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Simulation {
    /// String interner for owner/type_ref — zero-cost ID clones instead of heap Strings.
    pub interner: crate::sim::intern::StringInterner,
    /// Derived cache: interned-id -> object handle for one-hop type resolution.
    /// Built at init by `resolve_type_handles`; empty after deserialize (then
    /// `object_type` uses the name-path fallback). NOT serialized, NOT hashed.
    #[serde(skip)]
    pub type_handles: crate::sim::type_handle_table::TypeHandleTable,
    /// Credits, build queue state, and rally points.
    pub production: ProductionState,
    /// Current simulation tick (starts at 0, increments after each advance_tick).
    pub tick: u64,
    /// Total accumulated sim-tick milliseconds since world creation.
    /// Authoritative time source; binary_frame is derived from this.
    pub total_sim_ms: u64,
    /// Synthetic gamemd 15 Hz frame counter (the `g_CurrentFrameCounter`
    /// analog). Derived as (total_sim_ms * 15 / 1000), but **committed late** at
    /// the end of `advance_tick` beside `self.tick` — so during a tick it holds
    /// the previous tick's committed value, i.e. the pre-increment frame `N`
    /// this tick is executing under (mirrors Main_Tick incrementing
    /// `g_CurrentFrameCounter` only after `Network_ServiceLoop`). Read it as the
    /// *current* frame for stored-start CDTimer-style consumers (capture
    /// `binary_frame`, later compute `binary_frame.saturating_sub(start)`);
    /// never as the next frame.
    pub binary_frame: u32,
    /// Scenario RNG — gamemd `Scenario->Random` (Scen+0x218). Drives in-object-tick
    /// sim draws: scatter, sub-cell placement, smudge/destruction, particles,
    /// wall/overlay damage, bridge collapse/repair, ore growth/spread, TIBTRE,
    /// anim scorch/50-50, miner-dock jitter. MUST be serialized + hashed (never
    /// #[serde(skip)]) or a divergence here hides from desync detection.
    pub(crate) scenario_rng: SimRng,
    /// Main/global RNG — gamemd `g_MainRng` (0x00886B88). Drives presentation/weapon
    /// helpers (weapon spread, warhead detonate, sound variant, EBolt/laser, building
    /// missile, HouseClass AI/superpower gate). No verified sim/ consumer routes here
    /// today; seeded + hashed regardless so it is already in lockstep when those land.
    /// MUST be serialized + hashed.
    pub(crate) main_rng: SimRng,
    /// Map-generator RNG — gamemd `g_MapGenRng` (0x00ABE890). On a non-random map
    /// this `RandomClass` is never seeded, so it stays all-zero and returns 0 on
    /// every draw. The bridge **repair** walker-variant pick consumes this stream;
    /// on a fixed map it therefore always yields variant 0 (the base overlay), and
    /// the scenario/main cursors are never advanced by a repair. Constructed via
    /// `SimRng::zeroed()` (NOT seeded). MUST be serialized + hashed like the other
    /// two streams. Seeding this for random maps is a deferred (Blocked) follow-up.
    pub(crate) mapgen_rng: SimRng,
    /// Construction seed — recorded so the replay header carries the negotiated
    /// g_RngSeed (not a mid-stream fingerprint). Both streams derive from it.
    pub(crate) seed: u64,
    /// Deterministic fog/shroud visibility state.
    pub fog: FogState,
    /// Static alliance graph derived from map house data.
    pub house_alliances: HouseAllianceMap,
    /// Object substrate — the active-object order plus the monotonic id and
    /// enter-order counters. The single owner the lifecycle contract
    /// (reveal/conceal/unlimbo/uninit) mutates; entity storage and the
    /// occupancy grid migrate here in later stages.
    pub(crate) substrate: ObjectSubstrate,
    /// Sound events produced during the current tick — drained by the app layer.
    #[serde(skip)]
    pub sound_events: Vec<SimSoundEvent>,
    /// Fire events produced during combat — drained by the app layer for
    /// muzzle flash rendering and future projectile origin computation.
    #[serde(skip)]
    pub fire_events: Vec<SimFireEvent>,
    /// Smudge spawn requests emitted by callsites that don't return through
    /// `CombatTickResult` (superweapons, etc.). Drained alongside combat-emitted
    /// smudge requests in the post-combat drain block. Ephemeral — never
    /// persists across ticks.
    #[serde(skip)]
    pub pending_smudge_requests: Vec<crate::sim::combat::SmudgeSpawnRequest>,
    /// Bale deposit events emitted during refinery dock unloading — drained
    /// by the app layer for SpecialAnim trigger and particle bursts.
    #[serde(skip)]
    pub bale_events: Vec<crate::sim::components::BaleDepositEvent>,
    /// Tank-bunker wall-anim events — walls rising on install / falling on
    /// teardown. Drained by the app layer to create SpecialAnim overlays.
    /// Render-only; never persisted or hashed.
    #[serde(skip)]
    pub bunker_wall_events: Vec<crate::sim::components::BunkerWallAnimEvent>,
    /// Per-AI-owner state for computer-controlled players.
    pub ai_players: Vec<AiPlayerState>,
    /// Per-player state keyed by uppercase owner name. Deterministic iteration
    /// via BTreeMap. Equivalent to the original engine's HouseClass array.
    pub houses: BTreeMap<InternedId, HouseState>,
    /// Per-SpeedType terrain cost grids for cost-aware A* pathfinding.
    /// Built once at map load — units look up their SpeedType to pick the right grid.
    #[serde(skip)]
    pub terrain_costs: BTreeMap<SpeedType, TerrainCostGrid>,
    /// Zone-based connectivity map for instant unreachability detection.
    /// Built from terrain data; rebuilt when buildings or bridges change.
    #[serde(skip)]
    pub zone_grid: Option<ZoneGrid>,
    /// Previous PathGrid snapshot for incremental zone diffing.
    #[serde(skip)]
    prev_path_grid: Option<PathGrid>,
    #[serde(skip)]
    pub resolved_terrain: Option<ResolvedTerrainGrid>,
    pub bridge_state: Option<BridgeRuntimeState>,
    /// Per-cell mutable overlay state (ore density, wall damage, bridge frames).
    /// Seeded from map [OverlayPack] at init, mutated during gameplay.
    pub overlay_grid: Option<crate::sim::overlay_grid::OverlayGrid>,
    /// Per-cell smudge state (craters, scorches). Seeded from map [Smudge]
    /// entries at init, mutated by combat death-handling at runtime.
    pub smudge_grid: Option<crate::sim::smudge_grid::SmudgeGrid>,
    /// SHP interned IDs for bridge destruction explosions (from rules.ini BridgeExplosions=).
    #[serde(skip)]
    pub bridge_explosions: Vec<InternedId>,
    /// SHP interned IDs for bridge metallic-debris animations
    /// (from `[General] MetallicDebris=`). Pre-interned at sim init so the
    /// per-cell debris cascade in `bridge_orchestrator::spawn_bridge_debris`
    /// runs allocation-free.
    #[serde(skip)]
    pub metallic_debris: Vec<InternedId>,
    /// Selected start/report sound for bridge animation SHPs, keyed by SHP ID.
    #[serde(skip)]
    pub bridge_anim_sounds: BTreeMap<InternedId, InternedId>,
    /// Radar event queue for minimap pings and Spacebar cycling.
    #[serde(skip)]
    pub radar_events: RadarEventQueue,
    /// Runtime terrain cells whose radar/minimap terrain pixel needs refresh.
    /// Render reads this generation without mutating sim; the list is small and
    /// de-duplicated at emission.
    #[serde(skip)]
    pub radar_terrain_dirty_cells: Vec<(u16, u16)>,
    #[serde(skip)]
    pub radar_terrain_dirty_generation: u64,
    /// Runtime cell rects dirtied by tiberium mutation side effects.
    #[serde(skip)]
    pub tactical_dirty_cells: Vec<(u16, u16)>,
    /// Per-player power state (output, drain, low-power flag, spy blackout timer).
    /// Updated each tick by `power_system::tick_power_states()`.
    pub power_states: BTreeMap<InternedId, PowerState>,
    /// Per-owner superweapon instances. Outer key = owner, inner key = SW type ID.
    /// Deterministic iteration via nested BTreeMap.
    pub super_weapons:
        BTreeMap<InternedId, BTreeMap<InternedId, crate::sim::superweapon::SuperWeaponInstance>>,
    /// Active lightning storm state (global — only one at a time).
    pub lightning_storm: Option<crate::sim::superweapon::lightning_storm::LightningStormState>,
    /// Queued lightning storm — activates when the current storm ends.
    pub queued_lightning_storm:
        Option<crate::sim::superweapon::lightning_storm::QueuedLightningStorm>,
    /// Whether superweapon grants have been initialized from map-placed buildings.
    pub super_weapons_initialized: bool,
    /// Per-cell terrain speed modifier config (slope climb/descend, crowd density).
    /// Built from [General] rules at map load.
    #[serde(skip)]
    pub terrain_speed_config: terrain_speed::TerrainSpeedConfig,
    /// Distance in leptons below which a blocked unit stops instead of repathing.
    /// From CloseEnough= in [General]. Default 576 (~2.25 cells).
    pub close_enough: SimFixed,
    /// Ticks between pathfinding retry attempts (PathDelay= in [General]).
    pub path_delay_ticks: u16,
    /// Ticks to wait when blocked by a friendly before aggressive repath (BlockagePathDelay=).
    pub blockage_path_delay_ticks: u16,
    /// Temporary world-position SHP animations (warp effects, explosions, etc.).
    /// Ticked each frame, auto-removed when finished.
    #[serde(skip)]
    pub world_effects: Vec<crate::sim::components::WorldEffect>,
    /// Frame counts for world-effect SHPs, keyed by interned ID (e.g., "WARPOUT" → 20).
    /// Populated from the sprite atlas at init time so sim code can spawn effects
    /// with the correct frame count without hardcoding it.
    #[serde(skip)]
    pub effect_frame_counts: BTreeMap<InternedId, u16>,
    /// Per-match game settings (crates, short game, superweapons, etc.).
    /// Set once at game start from lobby / [MultiplayerDialogSettings], read-only during gameplay.
    pub game_options: GameOptions,
    /// When true, newly spawned entities get a `DebugEventLog` allocated.
    /// Toggled by the debug inspector hotkey (X). Debug-only — not included in state hashing.
    #[serde(skip)]
    pub debug_event_logging: bool,
    /// In-memory replay log for this match — records commands + state hashes per tick.
    /// Initialized lazily on the first tick. Observer artifact — not included in state hashing.
    #[serde(skip)]
    pub replay_log: Option<ReplayLog>,
    /// Input delay in ticks for lockstep-style command scheduling.
    /// Commands are scheduled `now_tick + input_delay_ticks` into the future.
    /// Set once from config at game start, read-only during gameplay.
    pub input_delay_ticks: u64,
    /// Pending gameplay commands waiting for their scheduled execution tick.
    /// Pushed by the app layer (user input, sidebar, AI), drained each tick
    /// in `advance_tick()` when `cmd.execute_tick <= current_tick + 1`.
    pub pending_commands: Vec<CommandEnvelope>,
    /// Map trigger runtime state — tracks global/local variables, disabled triggers,
    /// fired one-shot triggers, and elapsed scenario ticks. Initialized from map data.
    pub trigger_runtime: TriggerRuntime,
    /// Authoritative particle-system store. Populated by `spawn_particle_system`,
    /// ticked in Phase 5.5 by `particles::system_ai::tick_particle_systems`.
    /// Skipped by serde — restored as empty on deserialize (snapshot work is separate).
    #[serde(skip)]
    pub particle_systems: ParticleSystemStore,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    /// Create a new empty simulation with the default deterministic seed.
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SIM_SEED)
    }

    /// Build the interned-id -> type-handle table. Call once at sim init AFTER
    /// `RuleSet::intern_all_ids` (mirrors `resolve_bridge_warheads`), and again
    /// after load once the RuleSet is available. Idempotent.
    pub fn resolve_type_handles(&mut self, rules: &RuleSet) {
        self.type_handles =
            crate::sim::type_handle_table::TypeHandleTable::build(rules, &self.interner);
    }

    /// Resolve an entity's type to its `ObjectType` in one precomputed hop
    /// (two array indexes, no string allocation). Falls back to the name path
    /// when the table is unbuilt (test setups that skip `resolve_type_handles`),
    /// so no caller observes a stale empty table.
    #[inline]
    pub fn object_type<'r>(
        &self,
        type_ref: InternedId,
        rules: &'r RuleSet,
    ) -> Option<&'r ObjectType> {
        match self.type_handles.handle_for(type_ref) {
            Some(handle) => Some(rules.object_by_handle(handle)),
            None if self.type_handles.is_empty() => {
                rules.object(self.interner.resolve(type_ref))
            }
            None => None,
        }
    }

    /// Create a new empty simulation with an explicit deterministic seed.
    pub fn with_seed(seed: u64) -> Self {
        let out = Self {
            interner: crate::sim::intern::StringInterner::new(),
            type_handles: crate::sim::type_handle_table::TypeHandleTable::default(),
            production: ProductionState::default(),
            tick: 0,
            total_sim_ms: 0,
            binary_frame: 0,
            scenario_rng: SimRng::new(seed),
            main_rng: SimRng::new(seed),
            mapgen_rng: SimRng::zeroed(),
            seed,
            fog: FogState::default(),
            house_alliances: HouseAllianceMap::default(),
            substrate: ObjectSubstrate::new(),
            sound_events: Vec::new(),
            fire_events: Vec::new(),
            pending_smudge_requests: Vec::new(),
            bale_events: Vec::new(),
            bunker_wall_events: Vec::new(),
            ai_players: Vec::new(),
            houses: BTreeMap::new(),
            terrain_costs: BTreeMap::new(),
            zone_grid: None,
            prev_path_grid: None,
            resolved_terrain: None,
            bridge_state: None,
            overlay_grid: None,
            smudge_grid: None,
            bridge_explosions: Vec::new(),
            metallic_debris: Vec::new(),
            bridge_anim_sounds: BTreeMap::new(),
            radar_events: RadarEventQueue::default(),
            radar_terrain_dirty_cells: Vec::new(),
            radar_terrain_dirty_generation: 0,
            tactical_dirty_cells: Vec::new(),
            power_states: BTreeMap::new(),
            super_weapons: BTreeMap::new(),
            lightning_storm: None,
            queued_lightning_storm: None,
            super_weapons_initialized: false,
            terrain_speed_config: terrain_speed::TerrainSpeedConfig::default(),
            close_enough: SimFixed::from_num(576), // 2.25 cells × 256 lep/cell
            path_delay_ticks: 9,
            blockage_path_delay_ticks: 60,
            world_effects: Vec::new(),
            effect_frame_counts: BTreeMap::new(),
            game_options: GameOptions::default(),
            debug_event_logging: false,
            replay_log: None,
            input_delay_ticks: 2,
            pending_commands: Vec::new(),
            trigger_runtime: TriggerRuntime::default(),
            particle_systems: ParticleSystemStore::new(),
        };
        debug_assert_eq!(out.scenario_rng.state(), out.main_rng.state());
        out
    }

    // --- Scenario stream (gamemd Scenario->Random @ Scen+0x218) ---
    // Keep accessors distinct even though several return the same stream today:
    // the intent name is the per-consumer routing record and the grep/audit anchor.
    pub(crate) fn scatter_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // bump displacement, idle/forced scatter, passenger unload exit, sell-eject
    pub(crate) fn subcell_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // infantry sub-cell rotation, paradrop sub-cell
    pub(crate) fn smudge_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // destruction smudge/survivor/debris, smudge type pick
    pub(crate) fn wall_damage_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // overlay/wall damage roll
    pub(crate) fn bridge_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // bridge collapse/debris/explosion
    pub(crate) fn ore_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // ore growth/spread queue + direction + variant, TIBTRE
    pub(crate) fn anim_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // building damage-fire type/start-frame
    pub(crate) fn particle_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // particle/smoke/gas/fire lifetime/offset/dir/insert
    pub(crate) fn superweapon_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // lightning-storm scatter/bolt
    pub(crate) fn miner_jitter_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // dock-entry retry + unload-deploy frame jitter
    pub(crate) fn random_assignment_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // session random country/color resolution at launch handoff

    // --- Main stream (gamemd g_MainRng @ 0x00886B88); no sim/ consumer wired yet ---
    pub(crate) fn weapon_spread_rng(&mut self) -> &mut SimRng {
        &mut self.main_rng
    } // projectile spread X/Y, warhead detonate scatter
    pub(crate) fn house_ai_rng(&mut self) -> &mut SimRng {
        &mut self.main_rng
    } // HouseClass superpower/AI gate roll

    /// Test/replay helper — reseed BOTH streams from one seed (mirrors the dual
    /// Seed+clone in gamemd Init_Random_Number_System). Replaces test code that
    /// did `sim.rng = SimRng::new(seed)`.
    pub(crate) fn reseed_both(&mut self, seed: u64) {
        self.scenario_rng = SimRng::new(seed);
        self.main_rng = SimRng::new(seed);
        // mapgen_rng mirrors gamemd's unseeded g_MapGenRng — reset to zero-state,
        // never seeded from the gameplay seed.
        self.mapgen_rng = SimRng::zeroed();
        self.seed = seed;
    }

    /// The occupancy grid (per-cell object lists). Read access for systems above sim/.
    pub fn occupancy(&self) -> &OccupancyGrid {
        &self.substrate.occupancy
    }

    /// Mutable occupancy access for the few above-sim callers that unmark cells.
    pub fn occupancy_mut(&mut self) -> &mut OccupancyGrid {
        &mut self.substrate.occupancy
    }

    /// The entity store. Read access for systems above sim/.
    pub fn entities(&self) -> &EntityStore {
        &self.substrate.entities
    }

    /// Mutable entity-store access for above-sim callers.
    pub fn entities_mut(&mut self) -> &mut EntityStore {
        &mut self.substrate.entities
    }

    /// Disjoint access to the entity store (mutable) and the interner (shared)
    /// for the few above-sim callers that need both at once. The field-level
    /// disjoint borrow that made this trivial when `entities` was a sibling
    /// field of `interner` is no longer reachable from outside sim/.
    pub fn entities_mut_and_interner(
        &mut self,
    ) -> (&mut EntityStore, &crate::sim::intern::StringInterner) {
        (&mut self.substrate.entities, &self.interner)
    }

    /// Resolve an InternedId back to its display string.
    #[inline]
    pub fn resolve(&self, id: crate::sim::intern::InternedId) -> &str {
        self.interner.resolve(id)
    }

    pub(crate) fn mark_radar_terrain_dirty_cells<I>(&mut self, cells: I)
    where
        I: IntoIterator<Item = (u16, u16)>,
    {
        let mut changed = false;
        for cell in cells {
            if !self.radar_terrain_dirty_cells.contains(&cell) {
                self.radar_terrain_dirty_cells.push(cell);
                changed = true;
            }
        }
        if changed {
            self.radar_terrain_dirty_generation =
                self.radar_terrain_dirty_generation.wrapping_add(1);
        }
    }

    pub(crate) fn reduce_tiberium_at(
        &mut self,
        cell: (u16, u16),
        amount: u16,
    ) -> crate::sim::tiberium::ReduceTiberiumOutcome {
        self.reduce_tiberium_at_with_native_context(cell, amount, None, None)
    }

    pub(crate) fn reduce_tiberium_at_with_native_context(
        &mut self,
        cell: (u16, u16),
        amount: u16,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> crate::sim::tiberium::ReduceTiberiumOutcome {
        let mut ctx = crate::sim::tiberium::ReduceTiberiumContext {
            resource_nodes: &mut self.production.resource_nodes,
            overlay_grid: self.overlay_grid.as_mut(),
            ore_growth_state: &mut self.production.ore_growth_state,
            overlay_registry,
            tiberium_types: rules.map(|rules| &rules.tiberium_types),
            resolved_terrain: self.resolved_terrain.as_ref(),
            source_object_cells: Some(&self.production.tiberium_spawning_terrain_cells),
            // ore growth/spread — scenario stream. Direct field (not ore_rng()): this
            // literal co-borrows other &mut self fields, so the all-self accessor conflicts.
            rng: Some(&mut self.scenario_rng),
            binary_frame: self.binary_frame,
            spread_enabled: self.production.ore_growth_config.spreads,
            radar_dirty_cells: Some(&mut self.radar_terrain_dirty_cells),
            radar_dirty_generation: Some(&mut self.radar_terrain_dirty_generation),
            tactical_dirty_cells: Some(&mut self.tactical_dirty_cells),
        };
        crate::sim::tiberium::reduce_tiberium(&mut ctx, cell, amount)
    }

    /// Intern a string, returning its InternedId.
    #[inline]
    pub fn intern(&mut self, s: &str) -> crate::sim::intern::InternedId {
        self.interner.intern(s)
    }

    /// Queue a command for future execution at its scheduled tick.
    pub fn queue_command(&mut self, cmd: CommandEnvelope) {
        self.pending_commands.push(cmd);
    }

    /// Drain commands that are due for the next tick from `pending_commands`.
    /// Returns owned commands; remaining commands stay queued.
    pub fn take_due_commands(&mut self) -> Vec<CommandEnvelope> {
        let execute_tick = self.tick.saturating_add(1);
        let mut due = Vec::new();
        let mut kept = Vec::new();
        for cmd in std::mem::take(&mut self.pending_commands) {
            if cmd.execute_tick <= execute_tick {
                due.push(cmd);
            } else {
                kept.push(cmd);
            }
        }
        self.pending_commands = kept;
        due
    }

    /// Advance map triggers by one tick. Uses `std::mem::take` to avoid
    /// self-borrow conflict (advance reads entity/interner state via `&Simulation`).
    pub fn advance_triggers(
        &mut self,
        graph: &TriggerGraph,
        triggers: &TriggerMap,
        events: &EventMap,
        actions: &ActionMap,
    ) -> Vec<TriggerEffect> {
        let mut rt = std::mem::take(&mut self.trigger_runtime);
        let effects = rt.advance(1, graph, triggers, events, actions, Some(self));
        self.trigger_runtime = rt;
        effects
    }

    /// Returns true if the given house name is human-controlled.
    /// Equivalent to the original engine's IsHumanPlayer (0x50b6f0).
    pub fn is_human_player(&self, owner: &str) -> bool {
        self.interner
            .get(owner)
            .and_then(|id| self.houses.get(&id))
            .is_some_and(|h| h.is_human)
    }

    pub(crate) fn allocate_stable_id(&mut self) -> u64 {
        let id = self.substrate.next_stable_entity_id;
        self.substrate.next_stable_entity_id = self.substrate.next_stable_entity_id.saturating_add(1);
        id
    }

    /// Native Reveal's append: +0x98 guard → tail-append → set flag. Idempotent.
    pub(crate) fn register_live_object(&mut self, stable_id: u64) {
        match self.substrate.entities.get_mut(stable_id) {
            Some(e) if !e.in_logic_vector => {
                // Legal source: the only non-active presence in this slice is
                // Limbo, so an object joining the active set must be in Limbo.
                debug_assert_eq!(
                    e.presence,
                    Presence::Limbo,
                    "register_live_object: entity {stable_id} joined active set from {:?}, expected Limbo",
                    e.presence,
                );
                e.in_logic_vector = true;
                e.presence = Presence::InCell;
            }
            _ => return, // absent, or already a member (idempotent)
        }
        self.substrate.logic.push(stable_id);
    }

    /// Native Conceal's remove: gate on flag → clear flag → compacting remove.
    pub(crate) fn unregister_live_object(&mut self, stable_id: u64) {
        if let Some(e) = self.substrate.entities.get_mut(stable_id) {
            if !e.in_logic_vector {
                return; // not a member — nothing to remove
            }
            // Legal source: an object leaving the active set was InCell.
            debug_assert_eq!(
                e.presence,
                Presence::InCell,
                "unregister_live_object: entity {stable_id} left active set from {:?}, expected InCell",
                e.presence,
            );
            e.in_logic_vector = false;
            e.presence = Presence::Limbo;
        }
        // Entity present-and-member, or already gone from store: scrub the order.
        self.substrate.logic.remove(stable_id);
    }

    /// Native `ObjectClass::Reveal` append: an object becomes a live AI member.
    /// Active spawns / unlimbo / unload / paradrop call this. Delegates to the
    /// membership-guarded tail-append primitive; idempotent.
    pub(crate) fn reveal(&mut self, stable_id: u64) {
        self.register_live_object(stable_id);
    }

    /// Native `ObjectClass::Conceal`: the object leaves the live AI set but stays
    /// in the store (limbo). Delegates to the compacting-remove primitive.
    pub(crate) fn conceal(&mut self, stable_id: u64) {
        self.unregister_live_object(stable_id);
    }

    pub(crate) fn add_entity_occupancy(&mut self, stable_id: u64) {
        let Some(entity) = self.substrate.entities.get_mut(stable_id) else {
            return;
        };
        if entity.passenger_role.is_inside_transport() {
            return;
        }
        let Some(layer) = entity.occupancy_list_layer() else {
            return;
        };
        let cells = entity_occupancy_cells(entity);
        let sub_cell = if entity.category == EntityCategory::Infantry {
            entity.sub_cell
        } else {
            None
        };
        let order = self.substrate.next_occupancy_enter_order.next();
        entity.occupancy_enter_order = order;
        let insertion = CellListInsertion::from_category(entity.category);
        for (rx, ry) in cells {
            self.substrate.occupancy
                .add(rx, ry, stable_id, layer, sub_cell, insertion);
        }
    }

    pub(crate) fn remove_entity_occupancy(&mut self, stable_id: u64) {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        for (rx, ry) in entity_occupancy_cells(entity) {
            self.substrate.occupancy.remove(rx, ry, stable_id);
        }
    }

    /// Debug-only invariant: the active order and the per-entity membership flag
    /// are two views of one set and must never disagree. The order must be
    /// duplicate-free, and its length must equal the number of in-store entities
    /// whose `in_logic_vector` is set. O(n); compiled out of release builds.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_logic_membership_consistent(&self) {
        let order = self.substrate.logic.as_slice();
        let mut seen = std::collections::BTreeSet::new();
        for &id in order {
            debug_assert!(seen.insert(id), "logic order has duplicate id {id}");
        }
        let flagged = self.substrate.entities.values().filter(|e| e.in_logic_vector).count();
        debug_assert_eq!(
            order.len(),
            flagged,
            "logic order length ({}) != entities flagged in_logic_vector ({})",
            order.len(),
            flagged
        );
    }

    /// Debug-only invariant: the `presence` shadow must equal the value derivable
    /// from the authoritative gates for every in-store entity. Proves transition
    /// coverage is complete (every gate flip set the shadow). O(n); compiled out of
    /// release builds. `Dying` entities exist in-store between `uninit`'s enqueue
    /// and the end-of-tick `flush_pending_delete`. The flush runs in Phase 9 before
    /// this assert, so no `Dying` entity remains in the store at this call point.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_presence_consistent(&self) {
        for e in self.substrate.entities.values() {
            debug_assert_eq!(
                e.presence,
                e.derived_presence(),
                "entity {} presence {:?} != derived {:?} (in_logic_vector={})",
                e.stable_id,
                e.presence,
                e.derived_presence(),
                e.in_logic_vector,
            );
        }
    }

    /// Refresh the `mission` component's `current`/`substate` on every entity
    /// from the authoritative `Option<T>` machines, and advance its per-entity
    /// `tick_counter`. As of Slice 8 `mission` IS folded into `world_hash`, so
    /// this is the canonical projection writer: `current`/`substate` are a
    /// deterministic function of the authoritative machines (the verbs own
    /// `queued`/`suspended`/`timer`). Runs before `state_hash()` each tick tail,
    /// so the folded value reflects the current tick. BTreeMap `values_mut()`
    /// yields deterministic ascending-id order.
    pub(crate) fn refresh_mission_shadow(&mut self) {
        self.refresh_mission_shadow_except(&BTreeSet::new());
    }

    /// Tail projection with an S2 skip set: ids dispatched in-loop this tick
    /// already committed `current`/`substate` and incremented `tick_counter` at
    /// host time (authoritative); rewriting them here would clobber the
    /// dispatch-time value and double-count the counter.
    pub(crate) fn refresh_mission_shadow_except(&mut self, dispatched: &BTreeSet<u64>) {
        for entity in self.substrate.entities.values_mut() {
            if dispatched.contains(&entity.stable_id) {
                continue;
            }
            let (current, substate) = entity.derived_mission();
            entity.mission.current = current;
            entity.mission.substate = substate;
            entity.mission.tick_counter = entity.mission.tick_counter.wrapping_add(1);
        }
    }

    /// The active order, verbatim. No sorted-ID fallback (was DRIFT).
    ///
    /// This is a point-in-time copy: a consumer iterating it CANNOT observe an
    /// object registered or unregistered during its own pass. For native
    /// same-pass membership semantics use [`Self::for_each_live_object`].
    pub(crate) fn live_object_order_snapshot(&self) -> Vec<u64> {
        self.substrate.logic.snapshot()
    }

    /// Forward pass over the active-object order that RE-READS the live length
    /// after every body call.
    ///
    /// Consequences (the native scheduler contract):
    /// - An object the body tail-appends via `register_live_object` runs later
    ///   in the SAME pass (the length grows before the cursor reaches it).
    /// - A compacting `unregister_live_object` shifts successors left while the
    ///   cursor still advances by one, so the object pulled into the just-
    ///   processed slot is skipped this pass. There is no index repair.
    /// - Each member is visited at most once per pass; the index only advances.
    ///
    /// The body must tolerate an id whose entity is absent — there is no item
    /// guard here. `uninit` always conceals before freeing the store
    /// slot, so the order never references a removed entity in practice.
    pub(crate) fn for_each_live_object<F: FnMut(&mut Simulation, u64)>(&mut self, mut body: F) {
        let mut i = 0;
        while i < self.substrate.logic.len() {
            let id = self.substrate.logic.as_slice()[i];
            body(self, id);
            i += 1;
        }
    }

    /// P1 SHADOW BUILD: mirror each existing house's authoritative `credits` into
    /// the non-hashed `economy` shadow and recompute its OrePurifier building
    /// count. Derive direction is legacy -> shadow; READ-ONLY w.r.t. all hashed
    /// state. It iterates the existing `houses` map only and NEVER inserts a house
    /// (the auto-create hazard guard). A single pass over the entity store
    /// accumulates purifier counts per owner, so the cost is O(entities), not
    /// O(houses x entities). `rules` is the advance_tick tail's `Option`; with
    /// `None` the purifier count is 0 (no type data to classify structures by).
    pub(crate) fn refresh_economy_shadow(&mut self, rules: Option<&RuleSet>) {
        use crate::map::entities::EntityCategory;
        // One pass: accumulate OrePurifier building count per owner. Mirrors
        // `count_purifiers_for_owner`'s predicate (category == Structure &&
        // object_type.ore_purifier) but in a single sweep keyed by owner id.
        let mut purifiers: std::collections::BTreeMap<crate::sim::intern::InternedId, i32> =
            std::collections::BTreeMap::new();
        if let Some(rules) = rules {
            for e in self.substrate.entities.values() {
                if e.category == EntityCategory::Structure
                    && self
                        .object_type(e.type_ref, rules)
                        .is_some_and(|obj| obj.ore_purifier)
                {
                    *purifiers.entry(e.owner).or_insert(0) += 1;
                }
            }
        }
        for (id, house) in self.houses.iter_mut() {
            // The credits mirror is RETIRED at the flip: `economy.credits` is a
            // per-sweep shim that `step_all` loads from / stores to the one
            // authoritative wallet `house.credits`; it is not hashed, so it is not
            // maintained here.
            // Purifier-bonus base = real OrePurifier building COUNT (NOT silo
            // storage capacity, NOT the AI-virtual-inclusive effective count). Hashed.
            house.economy.purifier_count = purifiers.get(id).copied().unwrap_or(0);
            // spent_credits / harvested_credits accumulate via step_all / deposits;
            // intentionally untouched here.
        }
    }

    /// Per-tick production tail: refresh the per-house economy shadow (purifier count).
    /// Runs at the advance_tick tail, AFTER all authoritative systems.
    ///
    /// P5d: the factory registry is the authoritative queue-of-record and is mutated
    /// DIRECTLY by enqueue/cancel/delivery — there is no longer a `reconcile_from_queues`
    /// pass (the `queues_by_owner` mirror is retired), so its progress simply persists
    /// across ticks with no end-of-tick rebuild. `rules` is the tail's `Option`, threaded
    /// to the economy refresh.
    pub(crate) fn refresh_production_shadow(&mut self, rules: Option<&RuleSet>) {
        self.refresh_economy_shadow(rules);
    }

    /// Debug-only P2 asserts: (a) economy tracks credits; (b) the factory shell
    /// trace is well-formed (live Structures, strictly-increasing visit order).
    /// Divergence is surfaced, never equalized.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_production_shadow(&self) {
        // (P1 `debug_assert_economy_shadow` retired: `economy.credits` no longer tracks
        //  `house.credits` — it is a per-sweep shim, demoted at the authority flip.)
        self.debug_assert_factory_shell_trace();
        self.debug_assert_factory_conservation(); // P3
        self.debug_assert_factory_invariants(); // P5b (repurposed from the P5a inversion assert)
    }

    /// Debug-only P3 assert: each live shadow factory's `advance_one_step` conserves
    /// exact cost (C15) and settles correctly (C2/C12). Steps a CLONE against a CLONE
    /// economy seeded with exactly `original_balance`; SURFACES divergence with
    /// tick + owner + category, NEVER writes back to the shadow or the wallet.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_factory_conservation(&self) {
        use crate::sim::economy::Economy;
        use crate::sim::production::{StepOutcome, PRODUCTION_STEPS};
        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            if factory.object.is_none() {
                continue; // queue-only / no active object: nothing to conserve
            }
            let cost = factory.original_balance;
            // A fresh, armed clone driven from progress 0 with exact funds.
            let mut f = factory.clone();
            f.progress = 0;
            f.balance = cost;
            f.on_hold = false;
            f.suspended = false;
            f.manual = false;
            let mut econ = Economy {
                credits: cost,
                ..Economy::default()
            };
            let mut steps = 0i32;
            loop {
                match f.advance_one_step(&mut econ) {
                    StepOutcome::Stepped => steps += 1,
                    StepOutcome::Completed => {
                        steps += 1;
                        break;
                    }
                    // Stalled/Idle cannot happen with exact funds + a fresh arm; the
                    // asserts below fire (steps != 54) and surface the divergence.
                    _ => break,
                }
            }
            debug_assert_eq!(
                steps, PRODUCTION_STEPS as i32,
                "C2: tick {} {:?}/{:?}: a full build must take 54 steps (got {})",
                self.tick, factory.owner, factory.category, steps,
            );
            debug_assert_eq!(
                econ.spent_credits, cost,
                "C15: tick {} {:?}/{:?}: total spent {} must equal full cost {}",
                self.tick, factory.owner, factory.category, econ.spent_credits, cost,
            );
            debug_assert_eq!(
                f.balance, 0,
                "C12: tick {} {:?}/{:?}: completion must zero the balance",
                self.tick, factory.owner, factory.category,
            );
            debug_assert!(
                f.suspended && f.object.is_some(),
                "C12: tick {} {:?}/{:?}: completion must suspend with the object attached",
                self.tick, factory.owner, factory.category,
            );
        }
    }

    /// Debug-only P5b invariants on the now-authoritative registry (repurposed from the
    /// P5a inversion-readiness assert — the legacy upfront charge it compared against is
    /// retired, so the comparison is gone). Read-only; SURFACES divergence with
    /// tick+owner+category; NEVER writes back.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_factory_invariants(&self) {
        use crate::sim::production::PRODUCTION_STEPS;

        // (A) ORDER: the registry sweep order must be a TOTAL order — `iter_insertion_ordered`
        // yields strictly-increasing `insertion_seq` with NO ties (the strict-monotonic
        // `enqueue_order` property the hash fold + `step_all` charge order both depend on; a
        // tie would make the sweep order ambiguous and desync lockstep). Within each factory
        // the tail stamps strictly increase AND exceed the active build's `insertion_seq`
        // (FIFO `push_back` of a monotonic mint: the active build is the oldest, the tail
        // newer) — this is the D1 invariant expressed as a real self-check.
        let ordered = self.production.factory_shadow.iter_insertion_ordered();
        let mut prev_seq: Option<u64> = None;
        for f in &ordered {
            if let Some(p) = prev_seq {
                debug_assert!(
                    f.insertion_seq > p,
                    "P5d (A): tick {}: insertion_seq must be strictly increasing across the sweep ({} after {})",
                    self.tick, f.insertion_seq, p,
                );
            }
            prev_seq = Some(f.insertion_seq);
            let mut tail_prev = f.insertion_seq;
            for e in &f.queue {
                debug_assert!(
                    e.enqueue_order > tail_prev,
                    "P5d (A): tick {} {:?}/{:?}: tail enqueue_order must strictly exceed the active build + prior tail ({} after {})",
                    self.tick, f.owner, f.category, e.enqueue_order, tail_prev,
                );
                tail_prev = e.enqueue_order;
            }
        }

        // (B) STATE: progress in 0..=54; 0 <= balance <= original_balance (the per-step
        // ladder only decrements balance, and cancel resets both to 0).
        for f in self.production.factory_shadow.iter_insertion_ordered() {
            debug_assert!(
                f.progress <= PRODUCTION_STEPS,
                "P5b (B): tick {} {:?}/{:?}: progress {} exceeds {}",
                self.tick, f.owner, f.category, f.progress, PRODUCTION_STEPS,
            );
            debug_assert!(
                f.balance >= 0 && f.balance <= f.original_balance,
                "P5b (B): tick {} {:?}/{:?}: balance {} out of [0, original {}]",
                self.tick, f.owner, f.category, f.balance, f.original_balance,
            );
        }
    }

    /// Test-only: force the active order and sync membership flags to it.
    #[cfg(test)]
    pub(crate) fn set_logic_order_for_test(&mut self, order: Vec<u64>) {
        for &id in &order {
            if let Some(e) = self.substrate.entities.get_mut(id) {
                e.in_logic_vector = true;
                e.presence = Presence::InCell;
            }
        }
        self.substrate.logic.set_order_for_test(order);
    }

    /// Increment owned count for the given owner when an entity spawns.
    pub(crate) fn increment_owned_count(&mut self, owner: &str, category: EntityCategory) {
        if let Some(house) = crate::sim::house_state::house_state_for_owner_mut(
            &mut self.houses,
            owner,
            &self.interner,
        ) {
            match category {
                EntityCategory::Structure => house.owned_building_count += 1,
                _ => house.owned_unit_count += 1,
            }
        }
    }

    /// Decrement owned count for the given owner when an entity dies or is despawned.
    pub(crate) fn decrement_owned_count(&mut self, owner: &str, category: EntityCategory) {
        if let Some(house) = crate::sim::house_state::house_state_for_owner_mut(
            &mut self.houses,
            owner,
            &self.interner,
        ) {
            match category {
                EntityCategory::Structure => {
                    house.owned_building_count = house.owned_building_count.saturating_sub(1)
                }
                _ => house.owned_unit_count = house.owned_unit_count.saturating_sub(1),
            }
        }
    }

    /// Change an entity's owner through the substrate chokepoint: updates the
    /// `by_owner` index and the entity's owner field together. Index only — the
    /// caller owns any HouseState owned-count adjustment (count semantics differ
    /// by transfer kind: engineer capture adjusts counts; garrison transfers do not).
    pub(crate) fn change_owner(&mut self, stable_id: u64, new_owner: InternedId) {
        self.substrate.entities.change_owner(stable_id, new_owner);
    }

    /// Despawn an entity by stable_id, removing it from EntityStore.
    /// Decrements owned count if the entity was not already dying (combat deaths
    /// are decremented when dying is first set, not at physical removal).
    /// Also removes the entity from every occupied foundation cell.
    pub(crate) fn uninit(&mut self, stable_id: u64) {
        // Gather entity data before any mutable borrows.
        let entity_info = self.substrate.entities.get(stable_id).map(|e| {
            (
                e.dying,
                self.interner.resolve(e.owner).to_string(),
                e.category,
            )
        });
        if let Some((dying, owner_str, category)) = entity_info {
            if !dying {
                self.decrement_owned_count(&owner_str, category);
            }
            self.remove_entity_occupancy(stable_id);
        }
        self.clear_radio_contacts_for(stable_id);
        // Despawn safety net: clear the surviving side of any bunker link.
        crate::sim::docking::bunker_link::break_links_on_despawn(self, stable_id);
        self.conceal(stable_id); // leave the active order before freeing the slot
        // Conceal moved presence to Limbo (or it was already Limbo for a never-
        // revealed limbo object); we then mark Dying + enqueue. The store slot is
        // NOT freed here — flush_pending_delete frees it at end-of-tick. The entity
        // stays resolvable by id as a Dying corpse until then (the death window).
        if let Some(e) = self.substrate.entities.get_mut(stable_id) {
            debug_assert_ne!(
                e.presence,
                Presence::Dying,
                "uninit: entity {stable_id} already Dying (double teardown?)",
            );
            e.presence = Presence::Dying;
            // IsAlive-equivalent: a queued corpse is dead for all live systems.
            // Idempotent — the count-decrement above already read the original
            // `dying`, so owned-counts are still adjusted exactly once.
            e.dying = true;
        }
        // Two-phase death: enqueue instead of freeing. The slot is freed by
        // flush_pending_delete at end-of-tick (ProcessPendingDelete).
        self.substrate.pending_delete.push(stable_id);
    }

    /// Remove an entity from the world. Retained name for existing callers and
    /// tests; routes through `uninit` so conceal-before-free stays centralized.
    pub(crate) fn despawn_entity(&mut self, stable_id: u64) {
        self.uninit(stable_id);
    }

    /// Drain the deferred-delete queue, freeing each enqueued store slot in death
    /// (insertion) order. The end-of-tick `ProcessPendingDelete` drain. Called at
    /// the end of `run_late_region` (inside `advance_tick`, before the asserts +
    /// state hash) and in the app layer after the death-animation despawn loop.
    /// After this returns the queue is empty and no `Dying` entity remains in the
    /// store.
    pub(crate) fn flush_pending_delete(&mut self) {
        // mem::take so the loop body can call entities.remove without a
        // simultaneous borrow of self.substrate.pending_delete. Removing an
        // absent id is a no-op, covering any defensive double-enqueue.
        let queued = std::mem::take(&mut self.substrate.pending_delete);
        for id in queued {
            self.substrate.entities.remove(id);
        }
    }

    pub(crate) fn clear_radio_contacts_for(&mut self, stable_id: u64) {
        self.substrate.entities.clear_radio_contacts_for(stable_id);
    }

    /// Check each house for defeat and game completion
    /// (all remaining houses mutually allied).
    fn check_defeat(&mut self, rules: Option<&RuleSet>) {
        // Short Game defeats houses with no buildings unless a BaseUnit remains.
        // Long games wait for all owned objects.
        let owners: Vec<InternedId> = self.houses.keys().copied().collect();
        for &owner in &owners {
            let house = &self.houses[&owner];
            if house.is_defeated {
                continue;
            }
            let should_defeat = if self.game_options.short_game {
                house.owned_building_count == 0 && !self.house_has_live_base_unit(owner, rules)
            } else {
                house.owned_building_count == 0 && house.owned_unit_count == 0
            };
            if should_defeat {
                if let Some(h) = self.houses.get_mut(&owner) {
                    h.is_defeated = true;
                }
            }
        }

        // Check if all remaining alive houses are mutually allied → game over.
        let alive: Vec<InternedId> = self
            .houses
            .iter()
            .filter(|(_, h)| !h.is_defeated)
            .map(|(k, _)| *k)
            .collect();

        if alive.is_empty() {
            return;
        }

        if alive.len() == 1 {
            // Last player standing.
            if let Some(h) = self.houses.get_mut(&alive[0]) {
                h.has_won = true;
            }
            return;
        }

        // O(n^2) bidirectional alliance check.
        let all_allied = alive.iter().all(|a| {
            alive.iter().all(|b| {
                a == b
                    || crate::map::houses::are_houses_friendly(
                        &self.house_alliances,
                        self.interner.resolve(*a),
                        self.interner.resolve(*b),
                    )
            })
        });

        if all_allied {
            for &owner in &alive {
                if let Some(h) = self.houses.get_mut(&owner) {
                    h.has_won = true;
                }
            }
        }
    }

    fn house_has_live_base_unit(&self, owner: InternedId, rules: Option<&RuleSet>) -> bool {
        let Some(rules) = rules else {
            return false;
        };

        self.substrate.entities.values().any(|entity| {
            entity.owner == owner
                && entity.category == EntityCategory::Unit
                && !entity.dying
                && rules.general.base_unit_types.iter().any(|type_id| {
                    self.interner
                        .resolve(entity.type_ref)
                        .eq_ignore_ascii_case(type_id)
                })
        })
    }

    /// Restore skipped cache fields after snapshot deserialization.
    ///
    /// The caller must provide the same map/rules data that was used to initialize
    /// the original simulation. Cache fields were `#[serde(skip)]`'d and are at
    /// their Default values after deserialization.
    ///
    /// Note: `zone_grid` is NOT rebuilt here — it requires the app layer's
    /// `PathGrid` (built from resolved terrain + bridge state + building
    /// footprints). The caller
    /// should call `rebuild_dynamic_path_grid()` after this method, which triggers
    /// `rebuild_zone_grid()` as part of the normal tick flow.
    pub fn rebuild_caches_after_load(
        &mut self,
        resolved_terrain: ResolvedTerrainGrid,
        terrain_speed_config: terrain_speed::TerrainSpeedConfig,
        bridge_explosions: Vec<InternedId>,
        metallic_debris: Vec<InternedId>,
        bridge_anim_sounds: BTreeMap<InternedId, InternedId>,
        effect_frame_counts: BTreeMap<InternedId, u16>,
        terrain_costs: BTreeMap<SpeedType, TerrainCostGrid>,
    ) {
        // 1. Restore externally-derived data
        self.resolved_terrain = Some(resolved_terrain);
        self.terrain_speed_config = terrain_speed_config;
        self.bridge_explosions = bridge_explosions;
        self.metallic_debris = metallic_debris;
        self.bridge_anim_sounds = bridge_anim_sounds;
        self.effect_frame_counts = effect_frame_counts;
        self.terrain_costs = terrain_costs;

        // 2. Rebuild cached screen coords for all entities
        for entity in self.substrate.entities.values_mut() {
            entity.position.refresh_screen_coords();
        }

        // 2b. Rebuild LogicClass membership from the restored order.
        self.rebuild_logic_membership();

        // 3. Rebuild persistent occupancy from entity positions.
        // OccupancyGrid is #[serde(skip)] — starts empty after deserialization.
        self.substrate.occupancy = OccupancyGrid::rebuild(&self.substrate.entities);
    }

    /// Rebuild LogicClass membership flags from the restored active order.
    ///
    /// `+0x98` is not serialized (native does not round-trip it); vector presence
    /// is authoritative. Idempotent — safe to call after any load. Standalone (no
    /// heavy load-arg dependency) so save/load membership is unit-testable.
    pub(crate) fn rebuild_logic_membership(&mut self) {
        for entity in self.substrate.entities.values_mut() {
            entity.in_logic_vector = false;
        }
        for &id in &self.substrate.logic.snapshot() {
            if let Some(entity) = self.substrate.entities.get_mut(id) {
                entity.in_logic_vector = true;
            }
        }
        // Presence is #[serde(skip)] → all-default (Limbo) straight after
        // deserialize. Reconcile it from the just-restored authoritative gates so
        // a save/load round-trip restores identical presence (Slice 2 acceptance).
        for entity in self.substrate.entities.values_mut() {
            entity.presence = entity.derived_presence();
            // `mission` round-trips via serde now, but current/substate are
            // re-derived from the just-restored authoritative machines so a
            // save/load round-trip restores identical derived state.
            let (current, substate) = entity.derived_mission();
            entity.mission.current = current;
            entity.mission.substate = substate;
        }
    }

    /// Rebuild the zone connectivity map from the current PathGrid and terrain costs.
    /// Call after the PathGrid has been rebuilt so that zones reflect the latest
    /// walkability state.
    ///
    /// Tries an incremental update first (diffing against the previous PathGrid).
    /// Falls back to full rebuild if too many cells changed or no previous state.
    pub fn rebuild_zone_grid(&mut self, path_grid: &PathGrid) {
        let Some(terrain) = &self.resolved_terrain else {
            return;
        };
        let width = terrain.width();
        let height = terrain.height();

        // Try incremental update if we have previous state.
        if let (Some(prev), Some(zones)) = (&self.prev_path_grid, &mut self.zone_grid) {
            if let Some(changed) = prev.diff_cells(path_grid) {
                if changed.is_empty() {
                    // No cells changed — zones are still valid.
                    self.prev_path_grid = Some(path_grid.clone());
                    return;
                }
                if crate::sim::pathfinding::zone_incremental::try_incremental_update(
                    zones,
                    &changed,
                    path_grid,
                    &self.terrain_costs,
                    self.resolved_terrain.as_ref(),
                    self.bridge_state
                        .as_ref()
                        .map(|bs| bs.endpoint_records())
                        .unwrap_or(&[]),
                ) {
                    log::trace!("zone: incremental update ({} cells changed)", changed.len(),);
                    self.prev_path_grid = Some(path_grid.clone());
                    return;
                }
            }
        }

        // Full rebuild fallback.
        self.zone_grid = Some(ZoneGrid::build_with_terrain(
            path_grid,
            &self.terrain_costs,
            self.resolved_terrain.as_ref(),
            self.bridge_state
                .as_ref()
                .map(|bs| bs.endpoint_records())
                .unwrap_or(&[]),
            width,
            height,
        ));
        self.prev_path_grid = Some(path_grid.clone());
    }

    pub(crate) fn effective_build_blocked(&self, rx: u16, ry: u16) -> Option<bool> {
        let terrain = self.resolved_terrain.as_ref()?;
        let cell = terrain.cell(rx, ry)?;
        if cell.bridge_facts.has_flag(BRIDGE_FLAG_STRUCTURAL)
            || cell.bridge_facts.has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP)
            || cell.overlay_blocks
            || cell.terrain_object_blocks
            || cell.slope_type != 0
        {
            return Some(true);
        }
        if let Some(bridge) = self
            .bridge_state
            .as_ref()
            .and_then(|state| state.cell(rx, ry))
        {
            return Some(if matches!(bridge.damage_state, DamageState::Destroyed) {
                cell.base_build_blocked
            } else {
                true
            });
        }
        Some(cell.build_blocked)
    }

    /// Apply combat-emitted wall damage events: drives the per-cell damage
    /// progression in `damage_wall_overlay`, runs the cardinal-neighbor cleanup
    /// for any cells the damage destroys, and despawns the wall GameEntity at
    /// each destroyed cell.
    ///
    /// `rules` and `overlay_registry` are required because the wall damage
    /// pipeline reads per-overlay-type Strength/DamageLevels and identifies
    /// wall entities via the `Wall=yes` flag on their ObjectType.
    pub(crate) fn apply_wall_damage_events(
        &mut self,
        events: &[WallDamageEvent],
        rules: &RuleSet,
        overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
    ) {
        if events.is_empty() {
            return;
        }
        let Some(grid) = self.overlay_grid.as_mut() else {
            return;
        };

        let mut destroyed_cells: Vec<(u16, u16)> = Vec::new();

        for event in events {
            let result = damage_wall_overlay(
                grid,
                overlay_registry,
                event.rx,
                event.ry,
                event.damage,
                // wall/overlay damage — scenario stream. Direct field (not
                // wall_damage_rng()): `grid` holds a live &mut self.overlay_grid borrow.
                &mut self.scenario_rng,
            );

            for &cell in &result.destroyed_cells {
                destroyed_cells.push(cell);
                let chained = cleanup_wall_neighbors(grid, overlay_registry, cell.0, cell.1);
                destroyed_cells.extend(chained);
            }
        }

        if destroyed_cells.is_empty() {
            return;
        }

        destroyed_cells.sort_unstable();
        destroyed_cells.dedup();

        for (rx, ry) in destroyed_cells {
            self.remove_wall_entity_at(rx, ry, rules);
        }
    }

    /// Despawn the GameEntity backing a wall overlay cell, if one exists.
    /// Walls have a paired `GameEntity` for HP/ownership and an OverlayCell for
    /// rendering/passability. When the overlay is destroyed via wall-damage
    /// events, the entity must be removed too. Mod-loaded maps may have stale
    /// state with no matching entity; in that case a warn is logged and the
    /// caller continues without panicking.
    fn remove_wall_entity_at(&mut self, rx: u16, ry: u16, rules: &RuleSet) {
        let to_remove: Option<u64> = self.substrate.entities.iter_sorted().find_map(|(id, e)| {
            if e.position.rx == rx
                && e.position.ry == ry
                && self
                    .object_type(e.type_ref, rules)
                    .is_some_and(|o| o.wall)
            {
                Some(id)
            } else {
                None
            }
        });

        if let Some(id) = to_remove {
            // Route through the lifecycle chokepoint, not a raw store remove: a
            // wall is an EntityCategory::Structure, so it owns owned-building
            // count, foundation occupancy, logic-vector membership, and any radio
            // contacts. uninit tears all of those down in native order, marks the
            // entity Dying, and enqueues the slot for the end-of-tick
            // flush_pending_delete (the same deferred-death window every combat
            // death uses). A direct entities.remove leaks count/occupancy and
            // leaves a dangling id in the active order.
            self.uninit(id);
        } else {
            log::warn!("apply_wall_damage_events: no wall entity at ({rx}, {ry})");
        }
    }

    pub(crate) fn default_vision_range_for_category(category: EntityCategory) -> u16 {
        match category {
            EntityCategory::Infantry => 5,
            EntityCategory::Unit => 6,
            EntityCategory::Aircraft => 8,
            EntityCategory::Structure => 7,
        }
    }

    fn refresh_fog(
        &mut self,
        path_grid: Option<&PathGrid>,
        config: &vision::VisionConfig,
        rules: Option<&RuleSet>,
    ) {
        // Recompute visibility in-place: clears FLAG_VISIBLE on existing grids
        // (preserving FLAG_REVEALED) then re-reveals from entity positions.
        // No allocation or merge_revealed_from pass needed.
        let height_grid = if config.reveal_by_height {
            path_grid.map(PathGrid::ground_height_grid)
        } else {
            None
        };

        vision::recompute_owner_visibility_in_place(
            &mut self.fog,
            &self.substrate.entities,
            path_grid,
            &self.house_alliances,
            config,
            height_grid.as_deref(),
            &self.interner,
        );

        // Apply SpySat and Gap Generator effects if rules are available.
        if let Some(rules) = rules {
            let mut spy_sat_owners: Vec<InternedId> = Vec::new();
            let mut gap_generators: Vec<(InternedId, u16, u16)> = Vec::new();

            for entity in self.substrate.entities.values() {
                // A Dying SpySat/GapGen corpse must not reveal the map or shroud
                // an area for the tick after it is destroyed/sold.
                if entity.dying {
                    continue;
                }
                if entity.category != EntityCategory::Structure {
                    continue;
                }
                if let Some(obj) = self.object_type(entity.type_ref, rules) {
                    let active = power_system::is_building_powered(
                        &self.power_states,
                        rules,
                        entity,
                        &self.interner,
                    ) && entity.building_up.is_none();
                    if obj.spy_sat && active {
                        spy_sat_owners.push(entity.owner);
                    }
                    if obj.gap_generator && active {
                        gap_generators.push((entity.owner, entity.position.rx, entity.position.ry));
                    }
                }
            }

            // Apply in order: SpySat first, then Gap Generator (gap wins in contested areas).
            if !spy_sat_owners.is_empty() {
                vision::apply_spy_sat(&mut self.fog, &spy_sat_owners, &self.interner);
            }
            if !gap_generators.is_empty() {
                vision::apply_gap_generators(
                    &mut self.fog,
                    &gap_generators,
                    rules.general.gap_radius,
                    &self.interner,
                );
            }
        }

        // Diagnostic: log fog grid stats on first tick to debug coverage issues.
        if self.tick == 1 {
            log::info!(
                "Fog grid: {}x{}, {} owners",
                self.fog.width,
                self.fog.height,
                self.fog.by_owner.len()
            );
            for (owner, vis) in &self.fog.by_owner {
                let total = vis.width() as u32 * vis.height() as u32;
                let visible_count = vis.cells_raw().iter().filter(|c| **c & 0x02 != 0).count();
                let revealed_count = vis.cells_raw().iter().filter(|c| **c & 0x01 != 0).count();
                log::info!(
                    "  Owner '{}': {}/{} visible, {}/{} revealed",
                    owner,
                    visible_count,
                    total,
                    revealed_count,
                    total
                );
            }
            use std::collections::BTreeMap as DiagMap;
            let mut entity_stats: DiagMap<String, (u32, u16, u16, u16, u16)> = DiagMap::new();
            for entity in self.substrate.entities.values() {
                let entry = entity_stats
                    .entry(self.interner.resolve(entity.owner).to_string())
                    .or_insert((0, u16::MAX, u16::MAX, 0, 0));
                entry.0 += 1;
                entry.1 = entry.1.min(entity.position.rx);
                entry.2 = entry.2.min(entity.position.ry);
                entry.3 = entry.3.max(entity.position.rx);
                entry.4 = entry.4.max(entity.position.ry);
            }
            for (owner, (count, min_rx, min_ry, max_rx, max_ry)) in &entity_stats {
                log::info!(
                    "  Entities '{}': {} units, rx={}..{}, ry={}..{}",
                    owner,
                    count,
                    min_rx,
                    max_rx,
                    min_ry,
                    max_ry
                );
            }
        }
    }

    /// Advance build-up animations: increment elapsed ticks, remove when done.
    fn tick_building_up(&mut self) {
        // Collect keys first to allow &mut iteration via get_mut().
        let keys = self.substrate.entities.keys_sorted();
        let mut finished: Vec<u64> = Vec::new();
        for &sid in &keys {
            if let Some(entity) = self.substrate.entities.get_mut(sid) {
                if let Some(ref mut bu) = entity.building_up {
                    bu.elapsed_ticks = bu.elapsed_ticks.saturating_add(1);
                    if bu.elapsed_ticks >= bu.total_ticks {
                        finished.push(sid);
                    }
                }
            }
        }
        for sid in finished {
            if let Some(entity) = self.substrate.entities.get_mut(sid) {
                entity.building_up = None;
            }
        }
    }

    /// Advance building-down (undeploy) animations. When done, despawn the
    /// building and spawn the mobile unit (e.g., ConYard → MCV).
    /// Returns true if any entities were spawned (triggers atlas refresh).
    fn tick_building_down(&mut self, rules: Option<&RuleSet>) -> bool {
        let keys = self.substrate.entities.keys_sorted();
        let mut finished: Vec<u64> = Vec::new();
        for &sid in &keys {
            if let Some(entity) = self.substrate.entities.get_mut(sid) {
                if let Some(ref mut bd) = entity.building_down {
                    bd.elapsed_ticks = bd.elapsed_ticks.saturating_add(1);
                    if bd.elapsed_ticks >= bd.total_ticks {
                        finished.push(sid);
                    }
                }
            }
        }
        let any_finished = !finished.is_empty();
        for sid in finished {
            // Extract spawn data before despawning.
            let spawn_data = self.substrate.entities.get(sid).and_then(|e| {
                e.building_down.as_ref().map(|bd| {
                    (
                        bd.spawn_type,
                        bd.spawn_owner,
                        bd.spawn_rx,
                        bd.spawn_ry,
                        bd.spawn_z,
                        bd.was_selected,
                    )
                })
            });
            let Some((unit_type_id, owner_id, rx, ry, z, was_selected)) = spawn_data else {
                continue;
            };
            self.uninit(sid);
            let rules = match rules {
                Some(r) => r,
                None => continue,
            };
            let unit_type_str = self.interner.resolve(unit_type_id).to_string();
            let owner_str = self.interner.resolve(owner_id).to_string();
            if let Some(new_sid) =
                self.spawn_object_at_height(&unit_type_str, &owner_str, rx, ry, 0, z, rules)
            {
                if let Some(ge) = self.substrate.entities.get_mut(new_sid) {
                    ge.selected = was_selected;
                }
            }
        }
        any_finished
    }

    /// Advance one deterministic simulation tick.
    /// Spine region (EARLY): apply all commands due this tick, in
    /// `(execute_tick, owner)` order. Returns
    /// `(executed_commands, spawned_entities, destroyed_structure)`.
    fn apply_due_commands(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
        execute_tick: u64,
    ) -> (usize, bool, bool) {
        let mut executed_commands = 0usize;
        let mut spawned_entities = false;
        let mut destroyed_structure = false;

        let mut due: Vec<&CommandEnvelope> = commands
            .iter()
            .filter(|c| c.execute_tick <= execute_tick)
            .collect();
        due.sort_by(|a, b| {
            a.execute_tick
                .cmp(&b.execute_tick)
                .then_with(|| a.owner.cmp(&b.owner))
        });

        for cmd in due {
            let cmd_owner_str = self.interner.resolve(cmd.owner).to_string();
            let applied =
                self.apply_command(&cmd_owner_str, &cmd.payload, rules, path_grid, height_map);
            if applied {
                if matches!(
                    cmd.payload,
                    Command::PlaceReadyBuilding { .. }
                        | Command::DeployMcv { .. }
                        | Command::UndeployBuilding { .. }
                        | Command::LaunchSuperWeapon { .. }
                ) {
                    spawned_entities = true;
                }
                if matches!(
                    cmd.payload,
                    Command::SellBuilding { .. } | Command::UndeployBuilding { .. }
                ) {
                    destroyed_structure = true;
                }
            }
            executed_commands += 1;
        }

        (executed_commands, spawned_entities, destroyed_structure)
    }

    /// Spine region (LATE): AI commands, defeat detection, building animations,
    /// radar/world-effect aging, and the late frame/tick commit. Accumulates
    /// `spawned_entities` (AI placements + undeploy spawns). Call order is
    /// unchanged from the inline pipeline.
    fn run_late_region(
        &mut self,
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
        tick_ms: u32,
        execute_tick: u64,
        spawned_entities: &mut bool,
    ) {
        // --- Phase 8: Defeat detection (runs BEFORE AI) ---
        // gamemd evaluates each house's defeat before its AI manage/produce step,
        // so a house that lost its last building/unit this tick can issue NO AI
        // command this tick. Owned counts are final here after combat + production
        // (but before this tick's AI spawns); tick_ai then skips any house already
        // flagged defeated via its is_defeated gate.
        if self.tick > 0 {
            self.check_defeat(rules);
        }

        // --- Phase 8 (cont.): AI ---
        // DEPENDS ON: all prior phases + the defeat status set just above (defeated
        // houses are gated out inside tick_ai).
        // PRODUCES: commands applied immediately in the same tick.
        // Temporarily take ai_players out to avoid borrow conflict with &self.
        if rules.is_some() && !self.ai_players.is_empty() {
            let mut ai_state = std::mem::take(&mut self.ai_players);
            let ai_commands = ai::tick_ai(
                self,
                &mut ai_state,
                rules.expect("rules checked above"),
                path_grid,
                height_map,
            );
            self.ai_players = ai_state;
            for cmd in &ai_commands {
                let cmd_owner_str = self.interner.resolve(cmd.owner).to_string();
                let applied =
                    self.apply_command(&cmd_owner_str, &cmd.payload, rules, path_grid, height_map);
                if applied
                    && matches!(
                        cmd.payload,
                        Command::PlaceReadyBuilding { .. }
                            | Command::DeployMcv { .. }
                            | Command::UndeployBuilding { .. }
                            | Command::LaunchSuperWeapon { .. }
                    )
                {
                    *spawned_entities = true;
                }
            }
        }

        // --- Phase 9: Building animations + cleanup ---
        // DEPENDS ON: production (newly placed buildings start build-up).
        self.tick_building_up();
        // Advance building-down (undeploy) animations; spawn units when done.
        *spawned_entities |= self.tick_building_down(rules);

        // Tick radar event aging (remove expired pings).
        self.radar_events.tick(tick_ms);

        // Tick world-effect animations and remove finished ones.
        let mut started_effect_sounds = Vec::new();
        self.world_effects.retain_mut(|fx| {
            let tick = fx.tick_with_start_sound(tick_ms);
            if let Some(sound_id) = tick.started_sound {
                started_effect_sounds.push(SimSoundEvent::WorldEffectStarted {
                    sound_id,
                    rx: fx.rx,
                    ry: fx.ry,
                    sub_x: fx.sub_x,
                    sub_y: fx.sub_y,
                    z: fx.z,
                });
            }
            !tick.finished
        });
        self.sound_events.extend(started_effect_sounds);

        // The SINGLE in-tick deferred-delete drain (gamemd's one ProcessPending-
        // Delete at the tail of Main_Tick). Frees every entity uninit'd anywhere
        // in this advance_tick: command deaths (sells, MCV/slave deploy-undeploy,
        // engineer capture), Phase-5 combat structure/voxel deaths, tick_building_
        // down undeploy frees. The earlier command-boundary and end-of-Phase-5
        // drains were removed — corpses now live the full Dying window and every
        // mid-tick raw-store consumer is dying-gated instead. Runs BEFORE the
        // OCCUPANCY_DEBUG rebuild (which would re-add an unflushed dying structure)
        // and before the tail presence/membership asserts + state_hash.
        self.flush_pending_delete();

        // Debug-mode safety net: rebuild occupancy from scratch and compare
        // with the persistent grid. Catches missed add/remove calls.
        // Enable via OCCUPANCY_DEBUG=1 environment variable for focused debugging.
        #[cfg(debug_assertions)]
        if std::env::var("OCCUPANCY_DEBUG").is_ok() {
            let expected = OccupancyGrid::rebuild(&self.substrate.entities);
            self.substrate.occupancy.debug_assert_matches(&expected);
        }

        // Native frame / tick contract: commit the synthetic 15 Hz frame LATE,
        // after all phase work — mirrors Main_Tick's guarded g_CurrentFrameCounter
        // increment after Network_ServiceLoop. During the tick, binary_frame held
        // the previous tick's committed value (the pre-increment frame N), so
        // stored-start CDTimer consumers captured N, not N+1. Drift-free: every
        // binary-frame boundary is exactly when total_sim_ms crosses a multiple
        // of 1000/15 ≈ 66.67ms.
        self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
        self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
        self.tick = execute_tick;
    }

    pub fn advance_tick(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
    ) -> TickResult {
        // The synthetic 15 Hz binary-frame counter is committed LATE (end of
        // this fn, beside self.tick) so consumers see the pre-increment frame
        // during the tick. execute_tick stays here: command scheduling below
        // filters on it.
        let execute_tick = self.tick.saturating_add(1);
        // ===== SPINE REGION: EARLY — command application =====
        // gamemd applies player/network input before LogicClass::PerTickUpdate.
        // Native-spine slot: pre-object. (Step 3a skeleton: extracted to a region
        // method; call order unchanged — behavior-preserving.)
        let (executed_commands, mut spawned_entities, mut destroyed_structure) =
            self.apply_due_commands(commands, rules, path_grid, height_map, execute_tick);
        // No command-boundary drain: command-applied deaths (sell, MCV/slave
        // deploy-undeploy, engineer capture) now stay in the Dying window like
        // combat deaths, freed only by the single end-of-tick drain — matching
        // gamemd's one ProcessPendingDelete at the tail of Main_Tick. The mid-
        // tick raw-store consumers (vision, power, production, movement, miner,
        // aircraft, …) are dying-gated, so a corpse is excluded until that drain.
        let mut bridge_state_changed = false;
        let mut passenger_ownership_changed = false;

        // Object-AI stage (Slice S2a): instrumented no-op walk over the live
        // object order, relocated to run immediately BEFORE Phase-1 ground
        // movement. This is the per-object dispatch site that must precede the
        // locomotor — gamemd decides each object's mission, then moves it, within
        // one pass — so a later slice (S2b) can absorb the ground-movement loop
        // into this stage. The stage is still a strict no-op here, so the
        // relocation is hash-neutral (proven by the no-hash-change tests).
        // Movement stays before the Phase-3 vision recompute, so sight is
        // unaffected. The S1 shadow PROOF stays at end-of-tick, where the mission
        // shadow is fresh.
        //
        // S2a: bind the host-time Unit dispatch trace (debug/test only; empty,
        // non-allocating Vec in release). Consumed by the end-of-tick dispatch
        // proof beside `debug_assert_s1_shadow`.
        let dispatch_trace = self.object_ai_stage();

        // --- Phase 1: Ground movement ---
        // DEPENDS ON: commands (may set movement_target), entity positions from prior tick.
        // PRODUCES: updated entity positions, crush/bump effects, drive track state.
        let movement_order = self.live_object_order_snapshot();
        // S2: ids dispatched in-loop this tick; consumed by the tail projection.
        let mut s2_dispatched: BTreeSet<u64> = BTreeSet::new();
        let movement_stats = movement::tick_movement_with_grids(
            &mut self.substrate.entities,
            &movement_order,
            path_grid,
            &self.terrain_costs,
            &self.house_alliances,
            &mut self.substrate.occupancy,
            &mut self.substrate.next_occupancy_enter_order,
            // bump/scatter + sub-cell — scenario stream. Direct field (not
            // scatter_rng()): this call co-borrows &mut self.entities and
            // &mut self.substrate.occupancy (disjoint places).
            &mut self.scenario_rng,
            tick_ms,
            self.tick,
            self.zone_grid.as_ref(),
            self.resolved_terrain.as_ref(),
            &self.terrain_speed_config,
            self.close_enough,
            self.path_delay_ticks,
            self.blockage_path_delay_ticks,
            &mut self.interner,
            rules,
            &mut self.sound_events,
            &mut s2_dispatched,
        );
        if let Some(rules) = rules {
            crate::sim::gate_runtime::tick_gate_runtimes(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
                self.binary_frame,
            );
            // Slice 7d: break each war-factory exit contact whose vehicle has cleared
            // the factory footprint this tick (gamemd's per-cell-process break).
            crate::sim::production::tick_war_factory_exit_contacts(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
            );
        }
        // --- Phase 2: Air + special movement ---
        // DEPENDS ON: commands (may set movement targets for air/special units).
        // INDEPENDENT OF: ground movement (air units bypass A* and occupancy).
        let special_movement_order = self.live_object_order_snapshot();
        air_movement::tick_air_movement(
            &mut self.substrate.entities,
            &special_movement_order,
            tick_ms,
            self.tick,
        );
        if let Some(rules) = rules {
            let warp_out_type = self.interner.intern(&rules.general.warp_out.name);
            let mut teleport_visuals = teleport_movement::TeleportVisuals {
                world_effects: &mut self.world_effects,
                effect_frame_counts: &self.effect_frame_counts,
                warp_out_type,
                warp_out_rate_ms: rules.general.warp_out.rate_ms,
            };
            teleport_movement::tick_teleport_movement(
                &mut self.substrate.entities,
                &mut self.substrate.occupancy,
                &special_movement_order,
                tick_ms,
                self.tick,
                Some(&mut teleport_visuals),
            );
        } else {
            teleport_movement::tick_teleport_movement(
                &mut self.substrate.entities,
                &mut self.substrate.occupancy,
                &special_movement_order,
                tick_ms,
                self.tick,
                None,
            );
        }
        tunnel_movement::tick_tunnel_movement(
            &mut self.substrate.entities,
            &mut self.substrate.occupancy,
            &special_movement_order,
            tick_ms,
            self.tick,
        );
        let _rocket_detonations = rocket_movement::tick_rocket_movement(
            &mut self.substrate.entities,
            &special_movement_order,
            tick_ms,
            self.tick,
        );
        // Homing missile state machine. Runs in the same air/special-movement
        // phase as rocket_movement; detonation list is currently unused — the
        // production projectile-spawn dispatch lands in a separate follow-up.
        let _homing_detonations = homing_movement::tick_homing_movement(
            &mut self.substrate.entities,
            &special_movement_order,
            tick_ms,
            self.tick,
        );
        droppod_movement::tick_droppod_movement(
            &mut self.substrate.entities,
            &special_movement_order,
            tick_ms,
            self.tick,
        );
        if let Some(rules) = rules {
            parachute_descent::tick_parachute_descent(
                &mut self.substrate.entities,
                tick_ms,
                rules.general.parachute_max_fall_rate,
                self.tick,
            );
        }
        movement::tick_locomotor_piggyback_restore(&mut self.substrate.entities);

        // --- Phase 2.5: Body rocking + slope-transition advance ---
        // DEPENDS ON: all movement above (slope_type lookups must see the
        //   latest entity positions); rules.general.fallback_coefficient for
        //   the moving-vehicle decay scale.
        // PRODUCES: per-entity RockingState (angles, velocities, slope blend
        //   state) consumed by the renderer to compose the body matrix.
        // Aircraft skip slope tilting; infantry skip ship rocking. Wide-amplitude
        // self-destruct uses NoopSelfDestruct until combat-side damage lands
        // (Task 19); swap in a real hook then.
        if let (Some(rules), Some(terrain)) = (rules, self.resolved_terrain.as_ref()) {
            let mut hook = crate::sim::rocking::self_destruct::NoopSelfDestruct;
            crate::sim::rocking::tick(&mut self.substrate.entities, terrain, rules, &mut hook);
        }

        // Aircraft mission state machines — between movement and combat.
        // Reads updated positions, controls firing and RTB decisions.
        if let Some(rules) = rules {
            crate::sim::aircraft::tick_aircraft_missions(self, rules, path_grid);
        }

        // Spawn wake effects behind moving ships on water (every 8 ticks).
        if self.tick & 7 == 0 {
            if let Some(rules) = rules {
                let wake_name_str = &rules.general.wake.name;
                let wake_rate = rules.general.wake.rate_ms;
                let wake_name_id = self.interner.get(&wake_name_str.to_uppercase());
                let wake_frames = wake_name_id
                    .and_then(|id| self.effect_frame_counts.get(&id).copied())
                    .unwrap_or(8);
                // Collect positions to avoid borrow conflict (read entities, write world_effects).
                let wake_positions: Vec<(u16, u16, u8)> = self
                    .substrate.entities
                    .keys_sorted()
                    .iter()
                    .filter_map(|id| {
                        let e = self.substrate.entities.get(*id)?;
                        if e.movement_target.is_none() {
                            return None;
                        }
                        let loco = e.locomotor.as_ref()?;
                        let is_water_mover = loco.movement_zone.is_water_mover();
                        if !is_water_mover {
                            return None;
                        }
                        Some((e.position.rx, e.position.ry, e.position.z))
                    })
                    .collect();
                if let Some(wake_id) = wake_name_id {
                    for (rx, ry, z) in wake_positions {
                        self.world_effects.push(WorldEffect {
                            anim_spawn: None,
                            shp_name: wake_id,
                            rx,
                            ry,
                            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
                            z,
                            frame: 0,
                            total_frames: wake_frames,
                            rate_ms: wake_rate,
                            elapsed_ms: 0,
                            translucent: true,
                            delay_ms: 0,
                            start_sound_id: None,
                            start_sound_emitted: false,
                        });
                    }
                }
            }
        }

        // --- Phase 3: Vision refresh ---
        // DEPENDS ON: movement (positions updated), spawn (new entities need LOS).
        // PRODUCES: fog state used by combat targeting (phase 5).
        let vision_config = vision::VisionConfig {
            veteran_sight_bonus: rules.map_or(0, |r| r.general.veteran_sight),
            leptons_per_sight_increase: rules.map_or(0, |r| r.general.leptons_per_sight_increase),
            // Temporarily disabled: shroud rendering derives heights from PathGrid,
            // but enabling RevealByHeight here would also flip on cliff LoS.
            // Follow-up PR re-enables after gameplay parity review.
            reveal_by_height: false,
        };
        self.refresh_fog(path_grid, &vision_config, rules);

        if let Some(rules) = rules {
            // --- Phase 4: Power ---
            // DEPENDS ON: entity health (damaged buildings produce less power).
            // PRODUCES: power_states used by combat (cloaking) and production (build speed).
            let _power_events = power_system::tick_power_states(
                &mut self.power_states,
                &mut self.substrate.entities,
                rules,
                tick_ms,
                &self.interner,
            );
            // --- Phase 4.5: Superweapons ---
            // DEPENDS ON: power state (suspend/resume gating).
            // PRODUCES: world_effects (bolt anims), damage to entities, sound_events.
            if self.game_options.super_weapons {
                crate::sim::superweapon::tick_superweapons(self, rules);
            }

            // --- Phase 4.6: Deploy/Undeploy state machine ---
            // DEPENDS ON: command dispatch (ToggleInfantryDeploy may have set
            //   Deploying/Undeploying this tick).
            // PRODUCES: phase advances (Deploying→Deployed, Undeploying→None)
            //   that combat (Phase 5) and animation (post-tick) read this tick.
            crate::sim::deploy::tick_deploy_state(&mut self.substrate.entities);

            // Infantry fear decay and runtime prone transitions happen after
            // deploy state and before combat consumes the prone bit.
            crate::sim::infantry::tick_fear_for_entities(&mut self.substrate.entities, rules, &self.interner);

            // --- Phase 5: Combat + Turret rotation ---
            // DEPENDS ON: vision/fog (targeting uses fog state), power (cloaking).
            // Combat reads barrel.current(binary_frame) at the START of the tick
            // (matching gamemd's Fire_At_Target which uses last-frame facing).
            // tick_turret_rotation runs AFTER combat to drive rotation toward the
            // target for the NEXT frame's fire decision (matches Facing_Update order).
            // tick_c4_plants runs alongside tick_capture_orders — both convert
            // walk-up intent into a state change on arrival. Detonation damage
            // is applied here so combat-pre conditions (invulnerability, dying)
            // are honored before tick_combat runs.
            // PRODUCES: damage, deaths, bridge damage, fire events, last_attacker_id.
            // tick_bridge_repair_orders runs BEFORE tick_capture_orders so
            // engineers targeting BridgeRepairHut buildings are consumed by
            // repair, not by capture. tick_capture_orders has an explicit
            // BridgeRepairHut skip as defense in depth.
            let bridge_repaired = self.tick_bridge_repair_orders(rules);
            spawned_entities |= self.tick_capture_orders(rules);
            let c4_outcome = self.tick_c4_plants(rules);
            destroyed_structure |= c4_outcome.destroyed_structure;
            bridge_state_changed |= bridge_repaired | c4_outcome.bridge_state_changed;
            self.tick_order_intents_pre_combat(rules);
            // Pursuit: walk units with out-of-range attack_target into range,
            // halt movement on range entry. Must run before combat so combat
            // sees the up-to-date movement_target this tick.
            self.tick_attack_pursuit(rules, path_grid);
            // LogicClass live-object order drives the firing/damage/kill-credit
            // resolution sequence. Snapshot is owned, so it does not conflict
            // with the &mut self.entities borrow below.
            let logic_order = self.live_object_order_snapshot();
            let combat_result = combat::tick_combat_with_fog(
                &mut self.substrate.entities,
                &mut self.substrate.occupancy,
                rules,
                &mut self.interner,
                Some(&self.fog),
                &self.power_states,
                Some(&mut self.sound_events),
                &mut self.production.resource_nodes,
                self.overlay_grid.as_ref(),
                overlay_registry,
                self.resolved_terrain.as_ref(),
                self.tick,
                tick_ms,
                self.binary_frame,
                &logic_order,
            );
            turret::tick_turret_rotation(
                &mut self.substrate.entities,
                rules,
                self.binary_frame,
                &self.interner,
            );
            // Unit barrel facing is authoritative in unit_post; tick_turret_rotation
            // above skips Units. Same keys_sorted set+order as the legacy sweep,
            // restricted to Units, so the state hash is unmoved.
            crate::sim::world::unit_post::tick_unit_facing(
                &mut self.substrate.entities,
                rules,
                &self.interner,
                self.binary_frame,
            );
            destroyed_structure |= combat_result.structure_destroyed;
            let combat_dead_infos: Vec<(InternedId, EntityCategory)> = combat_result
                .despawned_ids
                .iter()
                .filter_map(|&dead_id| {
                    self.substrate.entities
                        .get(dead_id)
                        .map(|entity| (entity.owner, entity.category))
                })
                .collect();
            // Decrement owned counts for entities killed in combat (dying=true set this tick).
            for &(owner_id, category) in &combat_dead_infos {
                let owner_str = self.interner.resolve(owner_id).to_string();
                self.decrement_owned_count(&owner_str, category);
            }
            for &dead_id in &combat_result.despawned_ids {
                self.unregister_live_object(dead_id);
            }
            let mut sw_refresh_owners: Vec<InternedId> = Vec::new();
            if self.game_options.super_weapons && combat_result.structure_destroyed {
                for &(owner_id, category) in &combat_dead_infos {
                    if category == EntityCategory::Structure
                        && !sw_refresh_owners.contains(&owner_id)
                    {
                        sw_refresh_owners.push(owner_id);
                    }
                }
            }
            for &dead_id in &combat_result.immediate_uninit_ids {
                // Eject a bunkered unit before the bunker is removed (UndockUnit).
                if self
                    .substrate
                    .entities
                    .get(dead_id)
                    .and_then(|b| b.bunker_occupant)
                    .is_some()
                {
                    crate::sim::docking::bunker_link::release_sell_destroy(self, dead_id);
                }
                self.uninit(dead_id);
            }
            // Bridge damage: 4-path dispatcher + cascade
            // (kill ground occupants → DropIn deck → debris → rim refresh
            // → TriggerEvent 31 → zone rebuild). Replaces the legacy
            // 2-call pipeline.
            bridge_state_changed |=
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                );
            // Wall damage: feed combat-emitted wall hits through the per-cell damage
            // pipeline and despawn destroyed wall entities. Requires overlay_registry
            // (wall flag on OverlayType plus per-type Strength/DamageLevels); rules
            // is already unwrapped in this block.
            if let Some(reg) = overlay_registry {
                self.apply_wall_damage_events(&combat_result.wall_damage_events, rules, reg);
            }
            for req in &combat_result.tiberium_reduction_requests {
                self.reduce_tiberium_at_with_native_context(
                    (req.rx, req.ry),
                    req.amount,
                    Some(rules),
                    overlay_registry,
                );
            }
            for event in &combat_result.terrain_damage_events {
                let Some(warhead) = rules.warhead(self.interner.resolve(event.warhead_ref)) else {
                    continue;
                };
                let result = crate::sim::terrain_object::damage_terrain_object_at_cell(
                    &mut self.production,
                    rules,
                    &self.interner,
                    (event.rx, event.ry),
                    event.damage,
                    warhead,
                    self.resolved_terrain.as_mut(),
                );
                if matches!(
                    result,
                    crate::sim::terrain_object::TerrainDamageResult::Destroyed
                ) {
                    destroyed_structure = true;
                }
            }
            // Apply RevealOnFire events from combat.
            for ev in &combat_result.reveal_events {
                vision::reveal_radius(&mut self.fog, ev.owner, ev.rx, ev.ry, ev.radius);
            }
            // SpySat reshroud: when a SpySat building is destroyed, fully reshroud
            // its owner. Current LOS will re-reveal on the next vision tick.
            for &owner_id in &combat_result.spy_sat_reshroud_owners {
                self.fog.reset_explored_for_owner(owner_id);
            }
            // Eject survivors from crewed buildings destroyed in combat.
            for bldg in &combat_result.destroyed_crewed_buildings {
                production::eject_destruction_survivors(
                    self,
                    rules,
                    bldg.type_id,
                    bldg.owner,
                    bldg.rx,
                    bldg.ry,
                    bldg.z,
                );
            }
            // Eject garrison occupants from CanBeOccupied buildings destroyed in combat.
            for ev in &combat_result.destroyed_garrison_buildings {
                production::eject_destruction_garrison(self, rules, ev);
            }
            // Refresh superweapon grants for owners who lost structures in combat.
            if self.game_options.super_weapons && combat_result.structure_destroyed {
                for owner_id in sw_refresh_owners {
                    crate::sim::superweapon::refresh_super_weapons_for_owner(self, rules, owner_id);
                }
            }
            // Spawn explosion animations from combat deaths.
            for fx in &combat_result.explosion_effects {
                let frames = self
                    .effect_frame_counts
                    .get(&fx.shp_name)
                    .copied()
                    .unwrap_or(20);
                self.world_effects.push(WorldEffect {
                    anim_spawn: None,
                    shp_name: fx.shp_name,
                    rx: fx.rx,
                    ry: fx.ry,
                    sub_x: fx.sub_x,
                    sub_y: fx.sub_y,
                    z: fx.z,
                    frame: 0,
                    total_frames: frames,
                    rate_ms: 67, // ~15fps, standard for Normalized=yes explosion anims
                    elapsed_ms: 0,
                    translucent: true,
                    delay_ms: 0,
                    start_sound_id: None,
                    start_sound_emitted: false,
                });
            }
            // Collect fire events for render-side muzzle flash / projectile origin.
            self.fire_events.extend(combat_result.fire_events);
            // Emit radar events for combat occurrences.
            let event_dur: u32 = rules.radar_event_config.event_duration_ms;
            for ev in &combat_result.reveal_events {
                self.radar_events
                    .push(RadarEventType::Combat, ev.rx, ev.ry, event_dur);
            }
            // Drain combat-emitted smudge spawn requests before any tick stage
            // that reads tiberium density (ore-growth, repairs that touch
            // resource nodes). Ledger #6: crater-path Reduce_Tiberium(6) must
            // land before ore-growth reads density. Skipped when the optional
            // grids/path_grid aren't bound (headless tests, no map loaded).
            if let (Some(smudge_grid), Some(overlay), Some(terrain), Some(pg)) = (
                self.smudge_grid.as_mut(),
                self.overlay_grid.as_mut(),
                self.resolved_terrain.as_ref(),
                path_grid,
            ) {
                let mut tiberium_ctx = crate::sim::combat::smudge_dispatch::SmudgeTiberiumContext {
                    resource_nodes: &mut self.production.resource_nodes,
                    overlay_grid: overlay,
                    ore_growth_state: &mut self.production.ore_growth_state,
                    radar_dirty_cells: &mut self.radar_terrain_dirty_cells,
                    radar_dirty_generation: &mut self.radar_terrain_dirty_generation,
                    tactical_dirty_cells: &mut self.tactical_dirty_cells,
                };
                // Phase 4.5 superweapon-emitted smudges first, then Phase 5
                // combat-emitted. Drain order matches emission order so the
                // RNG cursor advances deterministically.
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &self.pending_smudge_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    &self.substrate.occupancy,
                    terrain,
                    pg,
                    &mut tiberium_ctx,
                    // destruction smudge — scenario stream. Direct field: co-borrows
                    // &mut smudge_grid/tiberium_ctx.
                    &mut self.scenario_rng,
                );
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &combat_result.smudge_spawn_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    &self.substrate.occupancy,
                    terrain,
                    pg,
                    &mut tiberium_ctx,
                    // destruction smudge — scenario stream. Direct field: co-borrows
                    // &mut smudge_grid/tiberium_ctx.
                    &mut self.scenario_rng,
                );
            }
            // Always clear pending — even if grids were unbound (headless
            // tests). The vec is per-tick ephemeral state.
            self.pending_smudge_requests.clear();

            // No end-of-Phase-5 drain: combat-killed structures/voxels stay in
            // the Dying window through the Phase 5.5-8.5 consumers and are freed
            // only by the single end-of-tick drain (gamemd's one ProcessPending-
            // Delete). Those consumers (production speed/factory-spawn scans,
            // repairs, retaliation, miner, aircraft) are dying-gated. Combat
            // post-processing above still reads the dead ids while resolvable
            // (count decrement, owner snapshot) — that runs before this point.
            // --- Phase 5.5: ParticleSystems ---
            // DEPENDS ON: combat (gas/fire damage spawned this tick).
            // PRODUCES: damage applied via gas/fire particles, must be visible to phase 6 retaliation.
            crate::sim::particles::system_ai::tick_particle_systems(self, rules);
            // --- Phase 6: Retaliation + Passengers ---
            // DEPENDS ON: combat (sets last_attacker_id read by retaliation).
            let logic_order = self.live_object_order_snapshot();
            combat::tick_retaliation(&mut self.substrate.entities, rules, &self.interner, &logic_order);
            passenger_ownership_changed = passenger::tick_passenger_system(self, rules);
            self.tick_order_intents_post_combat(path_grid, Some(rules));
            // --- Phase 7: Scatter + Production + Repairs + Docks + Ore ---
            // DEPENDS ON: combat (dead entities removed), movement (positions stable).
            // PRODUCES: new entities (spawned units), credit changes, ore growth.
            // Idle scatter disabled — units were moving on their own after reaching
            // destination. Needs further RE to match original engine conditions before
            // re-enabling.
            // scatter::tick_idle_scatter(
            //     &mut self.entities,
            //     Some(rules),
            //     path_grid,
            //     &self.terrain_costs,
            //     &mut self.scenario_rng, // idle-scatter — scenario stream (dormant)
            //     self.tick,
            //     &self.interner,
            // );
            // Phase 7, FIRST production step — the authoritative factory sweep (C1:
            // factories step BEFORE the house tail `run_late_region`). The previous
            // tick's tail reconcile prepared the registry; `step_all` charges each armed
            // factory's per-step cost against the REAL wallet (house.credits) in
            // insertion_seq (temporal) order; the spawn/placement pass below then
            // delivers completed builds and advances the queue-of-record.
            {
                let mut registry = std::mem::take(&mut self.production.factory_shadow);
                // P6: prereq/factory-loss revalidation BEFORE the charge sweep. Builds whose
                // prerequisites or producing factory were lost are abandoned (partial refund)
                // + now-unbuildable queued items dropped, so a freshly-abandoned factory is not
                // charged this tick and a freshly-promoted one starts charging next tick.
                let reval_plan = registry.plan_revalidation(self, rules);
                registry.apply_revalidation(&reval_plan, &mut self.houses);
                let prepared = registry.prepare_step_inputs(self, rules);
                registry.step_all(&mut self.houses, &prepared);
                self.production.factory_shadow = registry;
            }
            spawned_entities |= production::tick_production_with_overlay_registry(
                self,
                rules,
                height_map,
                path_grid,
                overlay_registry,
                tick_ms,
            );
            production::tick_repairs(self, rules);
            building_dock::tick_building_docks(self, rules);
            crate::sim::docking::bunker_install::tick_bunker_install(self, rules, path_grid);
            aircraft_dock::tick_aircraft_docks(self, rules);
            // Ore growth/spread: use native per-type queues once map load has
            // initialized them, preserving gamemd's growth-before-spread order.
            let native_growth_ready = !rules.tiberium_types.is_empty()
                && self
                    .production
                    .ore_growth_state
                    .native_tiberium_state()
                    .classes
                    .len()
                    == rules.tiberium_types.len()
                && self.overlay_grid.is_some()
                && overlay_registry.is_some();
            if native_growth_ready {
                if let (Some(grid), Some(registry)) = (self.overlay_grid.as_mut(), overlay_registry)
                {
                    self.production.ore_growth_state.tick_native_growth_driver(
                        grid,
                        registry,
                        &rules.tiberium_types,
                        self.resolved_terrain.as_ref(),
                        &self.production.tiberium_spawning_terrain_cells,
                        &mut self.production.resource_nodes,
                        // ore growth — scenario stream. Direct field: co-borrows grid/nodes.
                        &mut self.scenario_rng,
                        self.binary_frame,
                        self.production.ore_growth_config.grows,
                        self.production.ore_growth_config.spreads,
                    );
                    self.production.ore_growth_state.tick_native_spread_driver(
                        grid,
                        registry,
                        &rules.tiberium_types,
                        &mut self.production.resource_nodes,
                        path_grid,
                        self.resolved_terrain.as_ref(),
                        &self.production.tiberium_spawning_terrain_cells,
                        // ore spread — scenario stream. Direct field: co-borrows grid/nodes.
                        &mut self.scenario_rng,
                        self.binary_frame,
                        self.production.ore_growth_config.grows,
                        self.production.ore_growth_config.spreads,
                    );
                }
            } else {
                ore_growth::tick_ore_growth(
                    &self.production.ore_growth_config,
                    &mut self.production.ore_growth_state,
                    &mut self.production.resource_nodes,
                    path_grid,
                    self.overlay_grid.as_mut(),
                    // ore growth (legacy) — scenario stream. Direct field: co-borrows nodes.
                    &mut self.scenario_rng,
                );
            }
            // TIBTRE ore spawning: runs AFTER ore_growth so a spawn this tick
            // can't be grown/spread until next tick.
            let production = &mut self.production;
            crate::sim::terrain_spawn::tick_terrain_spawners_stateful(
                &mut production.terrain_spawners,
                crate::sim::terrain_spawn::TerrainSpawnContext::new(
                    &mut production.resource_nodes,
                    self.overlay_grid.as_mut(),
                    production.default_ore_overlay_id,
                    // TIBTRE — scenario stream. Direct field: co-borrows production/grid.
                    &mut self.scenario_rng,
                )
                .with_growth_queue(&mut production.ore_growth_state, self.binary_frame)
                .with_spawning_terrain_cells(&production.tiberium_spawning_terrain_cells)
                .with_live_object_context(&self.substrate.entities, &self.substrate.occupancy, rules, &self.interner)
                .with_validation_context(
                    self.resolved_terrain.as_ref(),
                    overlay_registry,
                    path_grid,
                ),
            );
            if spawned_entities {
                self.refresh_fog(path_grid, &vision_config, Some(rules));
            }
        }

        // ===== SPINE REGION: LATE — AI, defeat, anims, frame commit =====
        // (Step 3a skeleton: extracted to a region method; call order unchanged —
        // behavior-preserving.) Native-spine note: gamemd runs HouseClass updates
        // (incl. defeat) in the tail and commits the frame counter late; AI
        // placement is project-deferred and kept in its current slot.
        self.run_late_region(
            rules,
            path_grid,
            height_map,
            tick_ms,
            execute_tick,
            &mut spawned_entities,
        );
        #[cfg(debug_assertions)]
        self.debug_assert_logic_membership_consistent();
        #[cfg(debug_assertions)]
        self.debug_assert_presence_consistent();
        // Mission projection runs after all systems and before the hash, so the
        // folded `mission` reflects the current tick. As of Slice 8 `mission` is
        // canonical hashed state; the Slice-2 shadow-agreement assert is retired.
        self.refresh_mission_shadow_except(&s2_dispatched);
        // P1+P2 production+economy shadow: mirror credits + purifier_count and
        // rebuild the factory registry from the legacy queues, after all
        // authoritative systems and before the hash. Writes only non-hashed shadow
        // fields, so state_hash stays bit-identical (proven by the *_no_hash_change
        // tests). `rules` is the advance_tick `Option<&RuleSet>` tail param.
        self.refresh_production_shadow(rules);
        // Object-AI Slice S1 shadow: for one bounded moving-UnitClass scenario,
        // assert mission dispatch is observed before the locomotor Process within
        // one object pass (the verified gamemd ordering). Read-only, unhashed,
        // debug-only — the authority flip is a later slice.
        #[cfg(debug_assertions)]
        self.debug_assert_s1_shadow();
        // S2a: end-of-tick Unit dispatch proof — router determinism + AttackMove-
        // unreachable + Skip-never asserts, plus the host-vs-tail churn metric.
        // Read-only, debug/test only; the binding is consumed (or discarded in
        // release) so the host-time trace never leaks past the tick.
        #[cfg(any(test, debug_assertions))]
        let dispatch_churn = self.debug_assert_unit_dispatch_shadow(&dispatch_trace);
        #[cfg(not(any(test, debug_assertions)))]
        let dispatch_churn = {
            let _ = dispatch_trace;
            0u32
        };
        // S2a: live-set coverage (T5) — surface any Unit a legacy dispatch phase
        // would touch that is absent from the host's LogicVector set.
        #[cfg(any(test, debug_assertions))]
        self.debug_check_dispatch_live_set_coverage();
        #[cfg(debug_assertions)]
        self.debug_assert_production_shadow();
        let state_hash = self.state_hash();
        TickResult {
            tick: self.tick,
            executed_commands,
            state_hash,
            spawned_entities,
            destroyed_structure,
            ownership_changed: passenger_ownership_changed,
            bridge_state_changed,
            movement: movement_stats,
            dispatch_churn,
        }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "smudge_integration_tests.rs"]
mod smudge_integration_tests;

#[cfg(test)]
#[path = "world_orders_c4_tests.rs"]
mod world_orders_c4_tests;

#[cfg(test)]
#[path = "world_orders_bridge_repair_tests.rs"]
mod world_orders_bridge_repair_tests;

#[cfg(test)]
#[path = "rng_routing_tests.rs"]
mod rng_routing_tests;

#[cfg(test)]
#[path = "slice6_retask_tests.rs"]
mod slice6_retask_tests;

#[cfg(test)]
#[path = "mission_authoritative_tests.rs"]
mod mission_authoritative_tests;

#[cfg(test)]
#[path = "global_parity_harness_tests.rs"]
mod global_parity_harness_tests;

#[cfg(test)]
#[path = "production_shadow_tests.rs"]
mod production_shadow_tests;
