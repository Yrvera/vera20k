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
mod world_commands;
mod world_hash;
mod world_orders;
mod world_spawn;

use std::collections::BTreeMap;

use crate::map::actions::ActionMap;
use crate::map::entities::EntityCategory;
use crate::map::events::EventMap;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::rules::locomotor_type::SpeedType;
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
use crate::sim::occupancy::OccupancyGrid;
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
    /// An entity's owner changed (garrison transfer, engineer capture) — sprite
    /// atlas needs rebuild for the new house color.
    pub ownership_changed: bool,
    /// A bridge cell transitioned to `DamageState::Destroyed` this tick —
    /// PathGrid needs rebuild so A* sees collapsed cells as non-traversable
    /// starting next tick. Matches gamemd's one-tick-delayed visibility.
    pub bridge_state_changed: bool,
    pub movement: movement::MovementTickStats,
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
    /// Owner is the post-transfer building owner. App layer plays
    /// EVA_StructureGarrisoned if owner is local human.
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
    /// A paratrooper was dropped from a carrier aircraft.
    /// Played at the drop position; app layer resolves to [General] ChuteSound.
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
    /// Plain-struct entity storage.
    pub entities: EntityStore,
    /// Credits, build queue state, and rally points.
    pub production: ProductionState,
    /// Current simulation tick (starts at 0, increments after each advance_tick).
    pub tick: u64,
    /// Total accumulated sim-tick milliseconds since world creation.
    /// Authoritative time source; binary_frame is derived from this.
    pub total_sim_ms: u64,
    /// Synthetic gamemd 15 Hz frame counter. Computed each tick as
    /// (total_sim_ms * 15 / 1000). Used by FacingClass methods to compute
    /// animated values that match gamemd binary-frame timing exactly.
    pub binary_frame: u32,
    /// Single explicit deterministic PRNG stream for simulation logic.
    pub rng: SimRng,
    /// Deterministic fog/shroud visibility state.
    pub fog: FogState,
    /// Static alliance graph derived from map house data.
    pub house_alliances: HouseAllianceMap,
    pub(crate) next_stable_entity_id: u64,
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
    /// Per-AI-owner state for computer-controlled players.
    pub ai_players: Vec<AiPlayerState>,
    /// Per-player state keyed by uppercase owner name. Deterministic iteration
    /// via BTreeMap. Equivalent to the original engine's HouseClass array.
    pub houses: BTreeMap<InternedId, HouseState>,
    /// Per-SpeedType terrain cost grids for cost-aware A* pathfinding.
    /// Built once at map load — units look up their SpeedType to pick the right grid.
    #[serde(skip)]
    pub terrain_costs: BTreeMap<SpeedType, TerrainCostGrid>,
    /// Flat per-cell height grid for height-based LOS (RevealByHeight).
    /// Built from PathGrid; indexed by `ry * width + rx`.
    #[serde(skip)]
    pub(crate) vision_height_grid: Option<Vec<u8>>,
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
    /// Persistent cell occupancy — tracks what entities occupy each cell.
    /// Maintained incrementally via add/remove at spawn, move, and death sites.
    /// Rebuilt from entities on deserialization.
    #[serde(skip)]
    pub occupancy: OccupancyGrid,
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

    /// Create a new empty simulation with an explicit deterministic seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            interner: crate::sim::intern::StringInterner::new(),
            entities: EntityStore::new(),
            production: ProductionState::default(),
            tick: 0,
            total_sim_ms: 0,
            binary_frame: 0,
            rng: SimRng::new(seed),
            fog: FogState::default(),
            house_alliances: HouseAllianceMap::default(),
            next_stable_entity_id: 1,
            sound_events: Vec::new(),
            fire_events: Vec::new(),
            pending_smudge_requests: Vec::new(),
            bale_events: Vec::new(),
            ai_players: Vec::new(),
            houses: BTreeMap::new(),
            terrain_costs: BTreeMap::new(),
            vision_height_grid: None,
            zone_grid: None,
            prev_path_grid: None,
            resolved_terrain: None,
            bridge_state: None,
            overlay_grid: None,
            smudge_grid: None,
            occupancy: OccupancyGrid::new(),
            bridge_explosions: Vec::new(),
            metallic_debris: Vec::new(),
            bridge_anim_sounds: BTreeMap::new(),
            radar_events: RadarEventQueue::default(),
            radar_terrain_dirty_cells: Vec::new(),
            radar_terrain_dirty_generation: 0,
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
        }
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
        let id = self.next_stable_entity_id;
        self.next_stable_entity_id = self.next_stable_entity_id.saturating_add(1);
        id
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

    /// Despawn an entity by stable_id, removing it from EntityStore.
    /// Decrements owned count if the entity was not already dying (combat deaths
    /// are decremented when dying is first set, not at physical removal).
    /// Also removes the entity from the occupancy grid (origin cell only).
    pub(crate) fn despawn_entity(&mut self, stable_id: u64) {
        // Gather entity data before any mutable borrows.
        let entity_info = self.entities.get(stable_id).map(|e| {
            (
                e.dying,
                self.interner.resolve(e.owner).to_string(),
                e.category,
                e.position.rx,
                e.position.ry,
            )
        });
        if let Some((dying, owner_str, category, rx, ry)) = entity_info {
            if !dying {
                self.decrement_owned_count(&owner_str, category);
            }
            // Remove from occupancy grid (origin cell only; multi-cell structures
            // should have their foundation cells removed by the caller via
            // remove_entity_occupancy before calling despawn_entity).
            self.occupancy.remove(rx, ry, stable_id);
        }
        self.clear_radio_contacts_for(stable_id);
        self.entities.remove(stable_id);
    }

    pub(crate) fn clear_radio_contacts_for(&mut self, stable_id: u64) {
        self.entities.clear_radio_contacts_for(stable_id);
    }

    /// Check each house for defeat (owned count == 0) and game completion
    /// (all remaining houses mutually allied).
    fn check_defeat(&mut self) {
        // Mark houses with zero owned objects as defeated.
        let owners: Vec<InternedId> = self.houses.keys().copied().collect();
        for &owner in &owners {
            let house = &self.houses[&owner];
            if house.is_defeated {
                continue;
            }
            let total = house.owned_building_count + house.owned_unit_count;
            if total == 0 {
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
        for entity in self.entities.values_mut() {
            entity.position.refresh_screen_coords();
        }

        // 3. Rebuild persistent occupancy from entity positions.
        // OccupancyGrid is #[serde(skip)] — starts empty after deserialization.
        self.occupancy = OccupancyGrid::rebuild(&self.entities);
    }

    pub fn refresh_vision_heights(&mut self, grid: &PathGrid) {
        let w = grid.width() as usize;
        let h = grid.height() as usize;
        let mut heights = vec![0u8; w * h];
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if let Some(cell) = grid.cell(x, y) {
                    heights[y as usize * w + x as usize] = cell.ground_level;
                }
            }
        }
        self.vision_height_grid = Some(heights);
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
                &mut self.rng,
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
        let interner = &self.interner;
        let to_remove: Option<u64> = self.entities.iter_sorted().find_map(|(id, e)| {
            if e.position.rx == rx
                && e.position.ry == ry
                && rules
                    .object(interner.resolve(e.type_ref))
                    .is_some_and(|o| o.wall)
            {
                Some(id)
            } else {
                None
            }
        });

        if let Some(id) = to_remove {
            self.entities.remove(id);
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
        vision::recompute_owner_visibility_in_place(
            &mut self.fog,
            &self.entities,
            path_grid,
            &self.house_alliances,
            config,
            self.vision_height_grid.as_deref(),
            &self.interner,
        );

        // Apply SpySat and Gap Generator effects if rules are available.
        if let Some(rules) = rules {
            let mut spy_sat_owners: Vec<InternedId> = Vec::new();
            let mut gap_generators: Vec<(InternedId, u16, u16)> = Vec::new();

            for entity in self.entities.values() {
                if entity.category != EntityCategory::Structure {
                    continue;
                }
                if let Some(obj) = rules.object(self.interner.resolve(entity.type_ref)) {
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
            for entity in self.entities.values() {
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
        let keys = self.entities.keys_sorted();
        let mut finished: Vec<u64> = Vec::new();
        for &sid in &keys {
            if let Some(entity) = self.entities.get_mut(sid) {
                if let Some(ref mut bu) = entity.building_up {
                    bu.elapsed_ticks = bu.elapsed_ticks.saturating_add(1);
                    if bu.elapsed_ticks >= bu.total_ticks {
                        finished.push(sid);
                    }
                }
            }
        }
        for sid in finished {
            if let Some(entity) = self.entities.get_mut(sid) {
                entity.building_up = None;
            }
        }
    }

    /// Advance building-down (undeploy) animations. When done, despawn the
    /// building and spawn the mobile unit (e.g., ConYard → MCV).
    /// Returns true if any entities were spawned (triggers atlas refresh).
    fn tick_building_down(&mut self, rules: Option<&RuleSet>) -> bool {
        let keys = self.entities.keys_sorted();
        let mut finished: Vec<u64> = Vec::new();
        for &sid in &keys {
            if let Some(entity) = self.entities.get_mut(sid) {
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
            let spawn_data = self.entities.get(sid).and_then(|e| {
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
            self.despawn_entity(sid);
            let rules = match rules {
                Some(r) => r,
                None => continue,
            };
            let unit_type_str = self.interner.resolve(unit_type_id).to_string();
            let owner_str = self.interner.resolve(owner_id).to_string();
            if let Some(new_sid) =
                self.spawn_object_at_height(&unit_type_str, &owner_str, rx, ry, 0, z, rules)
            {
                if let Some(ge) = self.entities.get_mut(new_sid) {
                    ge.selected = was_selected;
                }
            }
        }
        any_finished
    }

    /// Advance one deterministic simulation tick.
    pub fn advance_tick(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
    ) -> TickResult {
        // Advance synthetic 15 Hz binary-frame counter. Drift-free: every
        // binary-frame boundary is exactly when total_sim_ms crosses a
        // multiple of 1000/15 ≈ 66.67ms.
        self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
        self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
        let execute_tick = self.tick.saturating_add(1);
        // Rebuild per-owner entity index. Cheap linear scan; captures any
        // owner mutations from the previous tick (engineer capture, mind control).
        self.entities.rebuild_owner_index();
        let mut executed_commands = 0usize;
        let mut spawned_entities = false;
        let mut destroyed_structure = false;
        let mut bridge_state_changed = false;
        let mut passenger_ownership_changed = false;

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

        // --- Phase 1: Ground movement ---
        // DEPENDS ON: commands (may set movement_target), entity positions from prior tick.
        // PRODUCES: updated entity positions, crush/bump effects, drive track state.
        let movement_stats = movement::tick_movement_with_grids(
            &mut self.entities,
            path_grid,
            &self.terrain_costs,
            &self.house_alliances,
            &mut self.occupancy,
            &mut self.rng,
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
        );
        // --- Phase 2: Air + special movement ---
        // DEPENDS ON: commands (may set movement targets for air/special units).
        // INDEPENDENT OF: ground movement (air units bypass A* and occupancy).
        air_movement::tick_air_movement(&mut self.entities, tick_ms, self.tick);
        teleport_movement::tick_teleport_movement(
            &mut self.entities,
            &mut self.occupancy,
            tick_ms,
            self.tick,
        );
        tunnel_movement::tick_tunnel_movement(
            &mut self.entities,
            &mut self.occupancy,
            tick_ms,
            self.tick,
        );
        let _rocket_detonations =
            rocket_movement::tick_rocket_movement(&mut self.entities, tick_ms, self.tick);
        // Homing missile state machine. Runs in the same air/special-movement
        // phase as rocket_movement; detonation list is currently unused — the
        // production projectile-spawn dispatch lands in a separate follow-up.
        let _homing_detonations =
            homing_movement::tick_homing_movement(&mut self.entities, tick_ms, self.tick);
        droppod_movement::tick_droppod_movement(&mut self.entities, tick_ms, self.tick);
        if let Some(rules) = rules {
            parachute_descent::tick_parachute_descent(
                &mut self.entities,
                tick_ms,
                rules.general.parachute_max_fall_rate,
                self.tick,
            );
        }

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
            crate::sim::rocking::tick(&mut self.entities, terrain, rules, &mut hook);
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
                    .entities
                    .keys_sorted()
                    .iter()
                    .filter_map(|id| {
                        let e = self.entities.get(*id)?;
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
            // Temporarily disabled: vision_height_grid is now populated for shroud
            // rendering, but enabling RevealByHeight here would also flip on cliff LoS
            // — a gameplay change shipped separately. Follow-up PR re-enables.
            reveal_by_height: false,
        };
        self.refresh_fog(path_grid, &vision_config, rules);

        if let Some(rules) = rules {
            // --- Phase 4: Power ---
            // DEPENDS ON: entity health (damaged buildings produce less power).
            // PRODUCES: power_states used by combat (cloaking) and production (build speed).
            let _power_events = power_system::tick_power_states(
                &mut self.power_states,
                &mut self.entities,
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
            crate::sim::deploy::tick_deploy_state(&mut self.entities);

            // Infantry fear decay and runtime prone transitions happen after
            // deploy state and before combat consumes the prone bit.
            crate::sim::infantry::tick_fear_for_entities(&mut self.entities, rules, &self.interner);

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
            let combat_result = combat::tick_combat_with_fog(
                &mut self.entities,
                &mut self.occupancy,
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
            );
            turret::tick_turret_rotation(
                &mut self.entities,
                rules,
                self.binary_frame,
                &self.interner,
            );
            destroyed_structure |= combat_result.structure_destroyed;
            // Decrement owned counts for entities killed in combat (dying=true set this tick).
            for &dead_id in &combat_result.despawned_ids {
                if let Some(entity) = self.entities.get(dead_id) {
                    let owner_str = self.interner.resolve(entity.owner).to_string();
                    let category = entity.category;
                    self.decrement_owned_count(&owner_str, category);
                }
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
                let mut sw_refresh_owners: Vec<InternedId> = Vec::new();
                for &dead_id in &combat_result.despawned_ids {
                    if let Some(entity) = self.entities.get(dead_id) {
                        if entity.category == crate::map::entities::EntityCategory::Structure
                            && !sw_refresh_owners.contains(&entity.owner)
                        {
                            sw_refresh_owners.push(entity.owner);
                        }
                    }
                }
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
                self.overlay_grid.as_ref(),
                self.resolved_terrain.as_ref(),
                path_grid,
            ) {
                // Phase 4.5 superweapon-emitted smudges first, then Phase 5
                // combat-emitted. Drain order matches emission order so the
                // RNG cursor advances deterministically.
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &self.pending_smudge_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    overlay,
                    &self.occupancy,
                    terrain,
                    pg,
                    &mut self.production.resource_nodes,
                    &mut self.rng,
                );
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &combat_result.smudge_spawn_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    overlay,
                    &self.occupancy,
                    terrain,
                    pg,
                    &mut self.production.resource_nodes,
                    &mut self.rng,
                );
            }
            // Always clear pending — even if grids were unbound (headless
            // tests). The vec is per-tick ephemeral state.
            self.pending_smudge_requests.clear();
            // --- Phase 5.5: ParticleSystems ---
            // DEPENDS ON: combat (gas/fire damage spawned this tick).
            // PRODUCES: damage applied via gas/fire particles, must be visible to phase 6 retaliation.
            crate::sim::particles::system_ai::tick_particle_systems(self, rules);
            // --- Phase 6: Retaliation + Passengers ---
            // DEPENDS ON: combat (sets last_attacker_id read by retaliation).
            combat::tick_retaliation(&mut self.entities, rules, &self.interner);
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
            //     &mut self.rng,
            //     self.tick,
            //     &self.interner,
            // );
            spawned_entities |=
                production::tick_production(self, rules, height_map, path_grid, tick_ms);
            production::tick_repairs(self, rules);
            building_dock::tick_building_docks(self, rules);
            aircraft_dock::tick_aircraft_docks(self, rules);
            // Ore growth/spread: incremental scan driven by rules.ini GrowthRate.
            ore_growth::tick_ore_growth(
                &self.production.ore_growth_config,
                &mut self.production.ore_growth_state,
                &mut self.production.resource_nodes,
                path_grid,
                self.overlay_grid.as_mut(),
                &mut self.rng,
            );
            // TIBTRE ore spawning: runs AFTER ore_growth so a spawn this tick
            // can't be grown/spread until next tick.
            crate::sim::terrain_spawn::tick_terrain_spawners(
                &self.production.terrain_spawners,
                &mut self.production.resource_nodes,
                self.overlay_grid.as_mut(),
                self.production.default_ore_overlay_id,
                path_grid,
                &mut self.rng,
            );
            if spawned_entities {
                self.refresh_fog(path_grid, &vision_config, Some(rules));
            }
        }

        // --- Phase 8: AI ---
        // DEPENDS ON: all prior phases (AI reads full game state to make decisions).
        // PRODUCES: commands applied immediately in the same tick.
        // AI decision loop: generate commands for computer players.
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
                    spawned_entities = true;
                }
            }
        }

        // --- Phase 8.5: Defeat detection ---
        // DEPENDS ON: combat (deaths processed), production (spawns), AI (commands applied).
        // Runs after all game-state mutations so owned counts are final for this tick.
        if self.tick > 0 {
            self.check_defeat();
        }

        // --- Phase 9: Building animations + cleanup ---
        // DEPENDS ON: production (newly placed buildings start build-up).
        self.tick_building_up();
        // Advance building-down (undeploy) animations; spawn units when done.
        spawned_entities |= self.tick_building_down(rules);

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

        // Debug-mode safety net: rebuild occupancy from scratch and compare
        // with the persistent grid. Catches missed add/remove calls.
        // Note: rebuild() only registers single cells (no multi-cell foundations),
        // so this check is conservative — extra cells from foundations are expected.
        // Enable via OCCUPANCY_DEBUG=1 environment variable for focused debugging.
        #[cfg(debug_assertions)]
        if std::env::var("OCCUPANCY_DEBUG").is_ok() {
            let expected = OccupancyGrid::rebuild(&self.entities);
            self.occupancy.debug_assert_matches(&expected);
        }

        self.tick = execute_tick;
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
