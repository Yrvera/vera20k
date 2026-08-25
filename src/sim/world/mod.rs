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
pub(crate) mod building_anim;
pub mod edge_cell;
mod lifecycle;
mod logic_vector;
mod substrate;
mod techno_ai;
pub(crate) mod techno_ai_cloak;
pub(crate) mod unit_post;
mod world_commands;
mod world_hash;
mod world_orders;
mod world_spawn;

#[cfg(test)]
mod gsi_04_18_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod team_script_vm_tests;

pub(crate) use lifecycle::{
    ConcealOutcome, LifecycleOutput, NULL_TARGET_CELL_SENTINEL, PlacementEvidence, RevealOutcome,
    RevealPosition, RevealRequest, UninitContext,
};
#[cfg(test)]
pub(crate) use lifecycle::{LifecycleTestEvent, RevealFailure};
pub(crate) use logic_vector::LogicVector;
pub use substrate::EnterOrderCounter;
pub(crate) use substrate::ObjectSubstrate;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::map::actions::ActionMap;
use crate::map::bridge_facts::{BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_STRUCTURAL};
use crate::map::entities::EntityCategory;
use crate::map::events::EventMap;
use crate::map::houses::HouseAllianceMap;
use crate::map::overlay::OverlayEntry;
use crate::map::resolved_terrain::{
    RealCellBridgeFlags0x1180, ResolvedTerrainGrid, SharedCellDummy,
};
use crate::map::trigger_graph::TriggerGraph;
use crate::map::triggers::TriggerMap;
use crate::rules::locomotor_type::LocomotorKind;
use crate::rules::locomotor_type::SpeedType;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::ai::{self, AiPlayerState};
use crate::sim::animation;
use crate::sim::bridge_state::{BridgeRuntimeState, DamageState};
use crate::sim::combat;
use crate::sim::combat::combat_weapon::WeaponSlot;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::{Position, WorldEffect};
use crate::sim::docking::aircraft_dock;
use crate::sim::docking::building_dock;
use crate::sim::entity_store::EntityStore;
use crate::sim::house_state::HouseState;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::movement;
use crate::sim::movement::drop_pod_movement;
use crate::sim::movement::group_destination;
use crate::sim::movement::homing_movement;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::movement::parachute_descent;
use crate::sim::movement::rocket_movement;
use crate::sim::movement::teleport_movement;
use crate::sim::movement::tunnel_movement::{self, TunnelProcessContext};
use crate::sim::movement::turret;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::ore_growth;
use crate::sim::overlay_grid::{WallDamageEvent, damage_wall_overlay, recalc_overlay_passability};
use crate::sim::passenger;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::{TerrainCostGrid, build_canonical_terrain_cost_grids};
use crate::sim::pathfinding::terrain_speed;
use crate::sim::pathfinding::zone_map::ZoneGrid;
use crate::sim::power_system::{self, PowerState};
use crate::sim::production::{self, ProductionState};
use crate::sim::projectile::{
    Projectile, ProjectileBridgeCrossing, ProjectileCellObstacle, ProjectileCollisionResponse,
    ProjectileCoord, projectile_bridge_crossing, projectile_cell_obstacle,
    projectile_slope_reflect,
};
use crate::sim::radar::{RadarEventQueue, RadarEventType};
use crate::sim::rng::{SimRng, SimRngLogicalState, SimRngLogicalView};
use crate::sim::scenario_session::ScenarioSession;
use crate::sim::team_script_vm::{TeamScriptEffect, TeamScriptVm};
use crate::sim::tiberium::TiberiumPlacementObjectContext;
use crate::sim::trigger_runtime::{TriggerEffect, TriggerRuntime};
use crate::sim::vision::{self, FogState};
use crate::util::fixed_math::SimFixed;

/// Dev/test fallback seed. Real launches negotiate a per-match seed through
/// `ScenarioDescriptor`; nothing on the launch path may rely on this value.
const DEFAULT_SIM_SEED: u64 = 0x5EED_CAFE_D15E_A5E5;

/// Whether this Unit visit reaches FootClass's SHP body-counter cadence.
///
/// An entry-active TubeMovement owns the UnitClass AI call and returns before
/// FootClass AI. Tube state armed later during an ordinary Foot visit does not
/// retroactively suppress work already reached by that visit, so only the
/// entry snapshot belongs in this admission predicate.
fn shp_vehicle_counter_admitted(tube_active_at_entry: bool) -> bool {
    !tube_active_at_entry
}

#[derive(Default)]
struct ActiveVisionStructures {
    spy_sat_owners: Vec<InternedId>,
    gap_generators: Vec<(InternedId, u16, u16, i32)>,
}

/// Bounded `BulletClass::AI` terrain collision admission.
///
/// The verified `Level` path uses the current canonical cell's water identity;
/// this engine's resolved terrain owns that identity directly. Cliff and
/// elevation trajectory kernels remain explicitly outside this straight-flight
/// port rather than being guessed from cell levels.
fn projectile_collides_at(
    terrain: Option<&ResolvedTerrainGrid>,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    interner: &crate::sim::intern::StringInterner,
    house_alliances: &HouseAllianceMap,
    overlay_grid: Option<&crate::sim::overlay_grid::OverlayGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    projectile: &Projectile,
    candidate: ProjectileCoord,
) -> Option<ProjectileCollisionResponse> {
    let policy = projectile.collision;
    if !policy.level_non_water && !policy.subject_to_walls && !policy.native_cell_collision {
        return None;
    }
    let Ok(rx) = u16::try_from(candidate.x.div_euclid(256)) else {
        return policy
            .level_non_water
            .then_some(ProjectileCollisionResponse::TargetZClamp(candidate));
    };
    let Ok(ry) = u16::try_from(candidate.y.div_euclid(256)) else {
        return policy
            .level_non_water
            .then_some(ProjectileCollisionResponse::TargetZClamp(candidate));
    };

    if policy.level_non_water
        && !terrain
            .and_then(|grid| grid.cell(rx, ry))
            .is_some_and(|cell| cell.is_water)
    {
        return Some(ProjectileCollisionResponse::TargetZClamp(candidate));
    }
    // `ConnectsToOverlay` is currently represented only by the parsed wall
    // connectivity family; no unrelated overlay flag is substituted.
    let overlay_connected = overlay_grid
        .and_then(|grid| grid.cell(rx, ry).overlay_id)
        .and_then(|overlay_id| overlay_registry.and_then(|registry| registry.flags(overlay_id)))
        .is_some_and(|flags| flags.wall);
    if policy.subject_to_walls && overlay_connected {
        return Some(ProjectileCollisionResponse::TargetZClamp(candidate));
    }
    if !policy.native_cell_collision {
        return None;
    }

    // Named location: `BulletClass::Update @ 0x004674ae..0x00467778`.
    let candidate_cell = terrain.and_then(|grid| grid.cell(rx, ry))?;
    let floor_z = crate::sim::cell_kernel::cell_floor_height(
        candidate_cell.level,
        candidate_cell.slope_type,
        candidate.x,
        candidate.y,
    )
    .ok()?;
    let previous_rx = u16::try_from(projectile.position.x / 256).ok();
    let previous_ry = u16::try_from(projectile.position.y / 256).ok();
    let previous_has_bridge = previous_rx
        .zip(previous_ry)
        .and_then(|(x, y)| terrain.and_then(|grid| grid.cell(x, y)))
        .is_some_and(|cell| cell.bridge_facts.has_structural_bridge());
    let candidate_has_bridge = candidate_cell.bridge_facts.has_structural_bridge();
    let bridge_surface =
        floor_z.saturating_add(crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS);
    let crossing = if previous_has_bridge || candidate_has_bridge {
        projectile_bridge_crossing(projectile.position.z, candidate.z, bridge_surface)
    } else {
        ProjectileBridgeCrossing::None
    };

    let building_id = occupancy.first_building_on_layer(rx, ry, MovementLayer::Ground);
    let building_is_target = matches!(projectile.target, crate::sim::projectile::ProjectileTarget::Entity(id) if Some(id) == building_id);
    let target_owner = match projectile.target {
        crate::sim::projectile::ProjectileTarget::Entity(id) => entities.get(id).map(|e| e.owner),
        crate::sim::projectile::ProjectileTarget::Cell { .. }
        | crate::sim::projectile::ProjectileTarget::None
        | crate::sim::projectile::ProjectileTarget::DummyCell => None,
    };
    let building_owner = building_id.and_then(|id| entities.get(id)).map(|e| e.owner);
    let allied = target_owner
        .zip(building_owner)
        .is_some_and(|(target, building)| {
            let target = interner.resolve(target).to_ascii_uppercase();
            let building = interner.resolve(building).to_ascii_uppercase();
            target == building
                || house_alliances
                    .get(&target)
                    .is_some_and(|allies| allies.contains(&building))
        });
    // The two raw Building exemptions have closed predicates but no represented
    // runtime producer yet. Keeping them false is an explicit residual, not a
    // guessed mapping to an unrelated ObjectType flag.
    let obstacle = projectile_cell_obstacle(
        candidate.z,
        floor_z,
        building_id,
        overlay_connected,
        building_is_target,
        false,
        false,
        allied,
    );
    if crossing == ProjectileBridgeCrossing::None && obstacle == ProjectileCellObstacle::None {
        return None;
    }

    let impact_z = if crossing == ProjectileBridgeCrossing::None {
        floor_z
    } else {
        bridge_surface
    };
    let impact = ProjectileCoord::new(candidate.x, candidate.y, impact_z);
    match projectile.target {
        crate::sim::projectile::ProjectileTarget::Entity(_) => {
            Some(ProjectileCollisionResponse::TargetZClamp(impact))
        }
        crate::sim::projectile::ProjectileTarget::Cell { .. }
        | crate::sim::projectile::ProjectileTarget::None
        | crate::sim::projectile::ProjectileTarget::DummyCell => {
            let velocity =
                projectile_slope_reflect(projectile.velocity, candidate_cell.slope_type)?;
            Some(ProjectileCollisionResponse::SlopeMatrixReflect { impact, velocity })
        }
    }
}

/// Result of one deterministic simulation tick.
#[derive(Debug, Clone, Copy)]
pub struct TickResult {
    pub tick: u64,
    /// Whether the late frame/tick commit and pending-delete drain ran.
    /// Victory, defeat, quit, and connection-loss exits leave this false.
    pub frame_committed: bool,
    pub executed_commands: usize,
    pub state_hash: u64,
    /// This call latched the natural win/loss score snapshot before hashing.
    pub terminal_score_finalized: bool,
    pub spawned_entities: bool,
    /// A structure was destroyed (combat, sell, crush); the frame finalizer
    /// rebuilds navigation to unblock the footprint.
    pub destroyed_structure: bool,
    /// An entity's owner changed (garrison reconciliation, engineer capture) — sprite
    /// atlas needs rebuild for the new house color.
    pub ownership_changed: bool,
    /// A bridge cell transitioned to `DamageState::Destroyed` this tick; the
    /// frame finalizer publishes the collapsed navigation snapshot for the next
    /// tick. Matches gamemd's one-tick-delayed visibility.
    pub bridge_state_changed: bool,
    pub movement: movement::MovementTickStats,
}

/// One authoritative frame plus the transient facts emitted while producing it.
///
/// Channel order is preserved within each vector. The app owns cross-channel
/// presentation order; collecting this value only transfers ownership and does
/// not invent a single mixed event timeline.
#[derive(Debug)]
pub(crate) struct SimFrameOutput {
    pub tick: TickResult,
    pub trigger_effects: Vec<TriggerEffect>,
    pub lifecycle_outputs: Vec<LifecycleOutput>,
    pub overlay_updates: Vec<OverlayEntry>,
    pub sound_events: Vec<SimSoundEvent>,
    pub fire_events: Vec<SimFireEvent>,
    pub invulnerability_impacts: Vec<crate::sim::combat::InvulnerabilityImpactEffect>,
}

/// Front-end admission lane for one Main_Tick call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickLane {
    /// Normal gameplay: commands/input dispatch in the Main_Tick tail after
    /// the live object/global update walk.
    Ordinary,
    /// LAN/WOL modal pump: service PerTickUpdate and the late tail only.
    NetworkModal,
}

/// Static map trigger definitions borrowed for one authoritative frame.
///
/// The definitions remain map data; `Simulation` owns the mutable runtime
/// state and evaluates it in the master-frame spine.
#[derive(Clone, Copy)]
pub(crate) struct TriggerInputs<'a> {
    pub graph: &'a TriggerGraph,
    pub triggers: &'a TriggerMap,
    pub events: &'a EventMap,
    pub actions: &'a ActionMap,
    /// Bound match rules used by action callbacks that share ordinary Techno
    /// runtime calculations (not reparsed or substituted by trigger data).
    pub rules: Option<&'a RuleSet>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MasterFrameTestRung {
    SessionCommands,
    Triggers,
    LogicVector,
    Houses,
    TeamScript,
    FrameCommit,
    PendingDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MovementSoundProbe {
    rx: u16,
    ry: u16,
    z: u8,
    sub_x_bits: i32,
    sub_y_bits: i32,
    facing: u8,
    path_index: Option<usize>,
    track_point: Option<u16>,
}

/// A sound event produced during simulation (combat, death, production).
/// Pure data — no audio library dependency. Drained by the app layer each frame.
#[derive(Debug, Clone)]
pub enum SimSoundEvent {
    /// Constructor-time animation start/report sound, keyed to object identity.
    AnimationStarted {
        anim_id: crate::sim::anim_class::AnimId,
        sound_id: InternedId,
        world: crate::sim::anim_class::AnimWorldCoord,
    },
    /// Animation destruction releases its current handle before optional StopSound.
    AnimationStopped {
        anim_id: crate::sim::anim_class::AnimId,
        stop_sound_id: Option<InternedId>,
        world: crate::sim::anim_class::AnimWorldCoord,
    },
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
    /// Normal crush teardown does not also enter the ordinary DieSound path.
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
    /// One accepted HouseClass win/loss transition. The app resolves the
    /// local owner's faction-specific STANDARD EVA; this edge is transient so
    /// loading a mid-Savour snapshot cannot replay the announcement.
    MatchOutcome {
        owner: InternedId,
        kind: crate::sim::house_state::HouseOutcomeKind,
    },
    /// Global `[AudioVisual] SellSound=` for a successful wall-sale event.
    /// The EventClass receiver can differ from the wall owner.
    WallSold { receiver: InternedId },
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
    /// `TechnoClass::StartUncloaking @ 0x007036C0` accepted an arg-zero
    /// state-1/2 transition and called positional `VocClass::PlayAt @
    /// 0x007509E0` with `[AudioVisual] CloakSound` at the Techno world coord.
    CloakSound {
        sound_id: String,
        rx: u16,
        ry: u16,
        sub_x: SimFixed,
        sub_y: SimFixed,
        world_z_leptons: i32,
    },
    /// A base structure / harvester took enemy damage — the radar ping is
    /// already enqueued sim-side; `eva_allowed` mirrors the queue's dedup
    /// result (the BridgeRepaired pattern). App gates the EVA voice to the
    /// local owner.
    UnderAttack {
        rx: u16,
        ry: u16,
        owner: InternedId,
        miner: bool,
        eva_allowed: bool,
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

impl SimSoundEvent {
    pub(crate) fn cloak_sound(sound_id: String, position: &Position) -> Self {
        Self::CloakSound {
            sound_id,
            rx: position.rx,
            ry: position.ry,
            sub_x: position.sub_x,
            sub_y: position.sub_y,
            world_z_leptons: position.exact_z_leptons.unwrap_or_else(|| {
                i32::from(position.z)
                    .wrapping_mul(crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS)
            }),
        }
    }
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

/// Borrowed names for the three native RNG authorities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulationRngViews<'a> {
    pub scenario: SimRngLogicalView<'a>,
    pub main: SimRngLogicalView<'a>,
    pub mapgen: SimRngLogicalView<'a>,
}

/// Owned logical RNG evidence used by the Rust pre-first-tick receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationRngState {
    pub scenario: SimRngLogicalState,
    pub main: SimRngLogicalState,
    pub mapgen: SimRngLogicalState,
}

fn deserialized_process_rng_placeholder() -> SimRng {
    SimRng::new(0)
}

fn deserialize_scenario_rng_reset<'de, D>(deserializer: D) -> Result<SimRng, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // gamemd saves ScenarioClass as a raw block that includes Random at +0x218,
    // then immediately reseeds that embedded object with zero after reading it.
    // Consume the saved bytes to preserve our snapshot layout, but reproduce the
    // active load result instead of restoring the saved cursor.
    let _saved = <SimRng as serde::Deserialize>::deserialize(deserializer)?;
    Ok(SimRng::new(0))
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
    pub(crate) type_handles: crate::sim::type_handle_table::TypeHandleTable,
    /// Pre-resolved `[CombatDamage]` warhead handles (F04). Built by
    /// `resolve_type_handles` beside the type table; rebuilt from rules on
    /// load. NOT serialized, NOT hashed. `None` until resolved.
    #[serde(skip)]
    pub(crate) rule_handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    /// Credits, build queue state, and rally points.
    pub production: ProductionState,
    /// Session aggregate — scenario identity, seed, authoritative map
    /// bounds, MP start table, per-match options, and the frame clocks
    /// (`tick` plus the wrapping `binary_frame`, committed late at the end of
    /// `advance_tick`). `total_sim_ms` is diagnostic only. Constructed once
    /// from the app-layer descriptor; serialized + hashed except for that
    /// diagnostic accumulator. See `sim::scenario_session`.
    pub session: ScenarioSession,
    /// Scenario RNG — gamemd `Scenario->Random` (Scen+0x218). Drives in-object-tick
    /// sim draws: scatter, sub-cell placement, smudge/destruction, particles,
    /// wall/overlay damage, bridge collapse/destruction presentation, ore growth/spread, TIBTRE,
    /// anim scorch/50-50, miner-dock jitter, and the terminal score projection.
    /// Saved in the Scenario-shaped snapshot and hashed while live. Native load
    /// reads those bytes but immediately calls `Random__Seed(0)`, so deserialization
    /// deliberately discards the saved cursor.
    #[serde(deserialize_with = "deserialize_scenario_rng_reset")]
    pub(crate) scenario_rng: SimRng,
    /// Main/global RNG — gamemd `g_MainRng` (0x00886B88). This is a
    /// process-global cursor: it is neither part of ScenarioClass saves nor
    /// multiplayer checksums, and the live cursor continues across a load.
    #[serde(skip, default = "deserialized_process_rng_placeholder")]
    pub(crate) main_rng: SimRng,
    /// Map-generator RNG — gamemd `g_MapGenRng` (0x00ABE890). VERA's fresh
    /// fixed-map construction uses `Random__Seed(0)`, matching the verified
    /// native fresh-process state; an accepted generated map installs its exact
    /// post-RMG continuation. Bridge repair consumes this stream; destruction
    /// remains Scenario-owned. This cursor is not saved or checksummed; Rust
    /// retains it across in-scenario restore, while native cross-match process
    /// retention remains UNCHECKED.
    #[serde(skip, default = "deserialized_process_rng_placeholder")]
    pub(crate) mapgen_rng: SimRng,
    /// Deterministic fog/shroud visibility state.
    pub fog: FogState,
    /// Static alliance graph derived from map house data.
    pub house_alliances: HouseAllianceMap,
    /// Object substrate — the active-object order plus the monotonic id and
    /// enter-order counters. The single owner the lifecycle contract
    /// (reveal/conceal/unlimbo/uninit) mutates; entity storage and the
    /// occupancy grid migrate here in later stages.
    pub(crate) substrate: ObjectSubstrate,
    /// Ordered release-visible handoffs produced by lifecycle transactions.
    /// The app drains these without feeding them back into simulation.
    #[serde(skip)]
    pub(crate) lifecycle_outputs: Vec<LifecycleOutput>,
    /// Ordered occupied-cell identities finalized at the authoritative frame
    /// boundary. The app drains these into its render-only overlay list.
    #[serde(skip)]
    frame_overlay_updates: Vec<OverlayEntry>,
    /// One-shot raw post-match score result. Serialized and hashed so a save at
    /// the outcome wait cannot repeat its Scenario RNG draws after load.
    pub(super) terminal_score_snapshot: Option<crate::sim::score::TerminalScoreSnapshot>,
    /// Reusable movement-to-world lifecycle request buffer. Requests are applied
    /// immediately after the movement call returns.
    #[serde(skip)]
    pub(crate) pending_lifecycle_requests: Vec<LifecycleRequest>,
    /// Missiles whose rocket flight reached its target during this tick's
    /// movement pass. Drained at the end of that pass, in live-object order.
    #[serde(skip)]
    pub(crate) pending_rocket_detonations: Vec<u64>,
    /// Missile impacts awaiting the combat phase, which expands each into
    /// ordinary damage events so the shared damage → death → despawn pipeline
    /// resolves them. Filled during the movement pass, drained after combat in
    /// the same tick.
    #[serde(skip)]
    pub(crate) pending_missile_detonations: Vec<crate::sim::spawn_manager::MissileDetonation>,
    /// BulletClass AI results produced in mixed Logic order and consumed at
    /// the existing combat receiver seam later in this master frame.
    #[serde(skip)]
    pub(crate) pending_projectile_detonations:
        Vec<crate::sim::projectile::ProjectileDetonation>,
    /// WaveClass AI damage requests produced in mixed Logic order and consumed
    /// at the established wave-damage receiver seam later in this frame.
    #[serde(skip)]
    pub(crate) pending_wave_damage_requests: Vec<crate::sim::wave::WaveDamageRequest>,
    /// Internal order proof; release builds carry no ledger or recording branch.
    #[cfg(test)]
    #[serde(skip)]
    lifecycle_test_events: Vec<LifecycleTestEvent>,
    /// App-visible outcomes produced by the authoritative trigger rung.
    /// Trigger actions mutate `trigger_runtime` during the frame; only their
    /// presentation outcomes are moved into `SimFrameOutput` after the tick.
    #[serde(skip)]
    trigger_effects: Vec<TriggerEffect>,
    #[cfg(test)]
    #[serde(skip)]
    master_frame_test_trace: Vec<MasterFrameTestRung>,
    /// Sound events produced during the current tick and moved into the owned
    /// app-frame output batch.
    #[serde(skip)]
    pub sound_events: Vec<SimSoundEvent>,
    /// Fire events produced during combat and moved into the app-frame output
    /// for muzzle flash rendering and future projectile origin computation.
    #[serde(skip)]
    pub(crate) fire_events: Vec<SimFireEvent>,
    /// Native IC/ForceShield impact combat-light requests emitted in receiver
    /// order during the current master frame. The native object has no owner
    /// and is distinct from AnimClass/ParticleSystemClass, so this remains a
    /// dedicated presentation handoff rather than fabricated world state.
    #[serde(skip)]
    pub(crate) invulnerability_impact_effects: Vec<crate::sim::combat::InvulnerabilityImpactEffect>,
    /// Persistent ordinary shots. This is authoritative save/hash state, not
    /// the render-side fire-event approximation.
    #[serde(default)]
    pub projectiles: crate::sim::projectile::ProjectileStore,
    /// Persistent WaveClass registrations. They have their own logic lifetime;
    /// this is not a one-frame weapon-fire presentation list.
    pub waves: crate::sim::wave::WaveStore,
    /// Hookless-test adapter retained for old fixture callsites. Production
    /// combat and superweapons commit smudges inline, so this remains empty and
    /// never persists across ticks.
    #[serde(skip)]
    pub(crate) pending_smudge_requests: Vec<crate::sim::combat::SmudgeSpawnRequest>,
    /// Bale deposit events emitted during refinery dock unloading and consumed
    /// by the authoritative frame tail for SpecialAnim and particle creation.
    #[serde(skip)]
    pub(crate) bale_events: Vec<crate::sim::components::BaleDepositEvent>,
    /// Tank-bunker wall-anim events — walls rising on install / falling on
    /// teardown. Consumed by the authoritative frame tail before hashing.
    #[serde(skip)]
    pub(crate) bunker_wall_events: Vec<crate::sim::components::BunkerWallAnimEvent>,
    /// Per-AI-owner state for computer-controlled players.
    pub ai_players: Vec<AiPlayerState>,
    /// Resolved TeamClass/ScriptType runtime; scenario INI parsing remains a
    /// separate refused boundary until its record grammar is evidenced.
    pub(crate) team_script_vm: TeamScriptVm,
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
    pub(crate) zone_grid: Option<ZoneGrid>,
    /// Canonical dynamic navigation projection. Arc snapshots let one master
    /// frame pin its entry view while the sim publishes the next projection.
    #[serde(skip)]
    path_grid: Option<Arc<PathGrid>>,
    #[serde(skip)]
    pub resolved_terrain: Option<ResolvedTerrainGrid>,
    /// Process-global MapClass fallback CellClass identity. Native owns this at
    /// `0x00ABDC50`, outside Scenario serialization; live in-scenario loads
    /// retain the current handle and rebuilt terrain is rebound to it.
    #[serde(skip, default)]
    pub(crate) shared_cell_dummy: SharedCellDummy,
    /// Serialized value authority for allocated real CellClass bridge bits.
    /// The derived terrain grid is skipped; load installs its pristine map
    /// template and then writes these saved values directly back onto real
    /// cells. This is value state, never a setter/replay log.
    #[serde(default)]
    pub(crate) real_cell_bridge_flags_0x1180: RealCellBridgeFlags0x1180,
    pub bridge_state: Option<BridgeRuntimeState>,
    /// Per-cell mutable overlay state (ore density, wall damage, bridge frames).
    /// Seeded from map [OverlayPack] at init, mutated during gameplay.
    pub overlay_grid: Option<crate::sim::overlay_grid::OverlayGrid>,
    /// Per-cell smudge state (craters, scorches). Seeded from map [Smudge]
    /// entries at init, mutated by combat death-handling at runtime.
    pub smudge_grid: Option<crate::sim::smudge_grid::SmudgeGrid>,
    /// Per-cell radiation field + site registry. Detonations of RadLevel>0
    /// weapons feed it during the combat phase; sites decay in their own
    /// post-combat step; foot units take periodic damage from their cell.
    #[serde(default)]
    pub radiation: crate::sim::radiation::RadiationState,
    /// The map's isometric playfield diamond ([Map] Size width + the raw
    /// LocalSize rect), set at map init. Threaded into the cell-rect occupancy
    /// validator's final playfield-corner test (the engine diamond, not a
    /// rectangle). `None` only in headless tests with no map loaded.
    #[serde(default)]
    pub playfield_bounds: Option<crate::sim::cell_rect::PlayfieldBounds>,
    /// Immutable signed `[Map] Size=` height retained for later LocalSize
    /// normalization. The live predicate needs only Size width, but action 40
    /// can rewrite the four LocalSize dwords repeatedly during a scenario.
    #[serde(default)]
    pub(crate) playfield_size_height: Option<i32>,
    /// Monotonic visible-area writer generation. Every successful trigger
    /// action 0x28 advances it, even when two writers normalize to the same
    /// bounds. Presentation consumes this as the global radar/scroll rebuild
    /// edge; cell-local bridge dirtiness is deliberately a separate channel.
    #[serde(default)]
    pub(crate) playfield_revision: u64,
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
    /// Presentation reads this generation and acknowledges the exact batch
    /// only after its radar update completes. The list is de-duplicated within
    /// that pending update window, then cleared so the same cell can re-arm.
    #[serde(skip)]
    pub radar_terrain_dirty_cells: Vec<(u16, u16)>,
    #[serde(skip)]
    pub radar_terrain_dirty_generation: u64,
    /// Runtime cell rects dirtied by tiberium mutation side effects.
    #[serde(skip)]
    pub(crate) tactical_dirty_cells: Vec<(u16, u16)>,
    /// Per-player power state (output, drain, low-power flag, spy blackout timer).
    /// Updated each tick by `power_system::tick_power_states()`.
    pub power_states: BTreeMap<InternedId, PowerState>,
    /// Per-owner superweapon instances. Outer key = owner, inner key = SW type ID.
    /// Deterministic iteration via nested BTreeMap.
    pub(crate) super_weapons:
        BTreeMap<InternedId, BTreeMap<InternedId, crate::sim::superweapon::SuperWeaponInstance>>,
    /// Active lightning storm state (global — only one at a time).
    pub(crate) lightning_storm: Option<crate::sim::superweapon::lightning_storm::LightningStormState>,
    /// Whether superweapon grants have been initialized from map-placed buildings.
    pub(crate) super_weapons_initialized: bool,
    /// Per-cell terrain speed modifier config (slope climb/descend).
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
    /// When true, newly spawned entities get a `DebugEventLog` allocated.
    /// Toggled by the debug inspector hotkey (X). Debug-only — not included in state hashing.
    #[serde(skip)]
    pub debug_event_logging: bool,
    /// Negotiated lockstep ahead window. Offline producers stamp the current
    /// raw issue ordinal; a network transfer owner overwrites that stamp with
    /// `send_current + MaxAhead` before synchronized dispatch.
    pub input_delay_ticks: u64,
    /// Native Main_Tick termination gates outside House state. These are
    /// front-end connection/session facts, so they are transient and un-hashed.
    #[serde(skip)]
    pub quit_requested: bool,
    /// One-shot owner-tagged edge raised only when a due native EXIT command
    /// executes at the EventClass tail. App teardown consumes and clears it.
    #[serde(skip)]
    executed_exit_owner: Option<InternedId>,
    #[serde(skip)]
    pub(crate) connection_lost: bool,
    /// Pending gameplay commands waiting for their scheduled execution tick.
    /// Admitted through `queue_command(s)` and drained each tick when
    /// `cmd.execute_tick <= current_tick + 1`.
    pending_commands: Vec<CommandEnvelope>,
    /// Map trigger runtime state — tracks global/local variables, disabled triggers,
    /// fired one-shot triggers, and elapsed scenario ticks. Initialized from map data.
    pub trigger_runtime: TriggerRuntime,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_tiberium_reduction_inline(
    request: &crate::sim::combat::TiberiumReductionRequest,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    scenario_rng: &mut SimRng,
    resource_nodes: &mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
    overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
    terrain: Option<&mut ResolvedTerrainGrid>,
    ore_growth_state: &mut crate::sim::ore_growth::OreGrowthState,
    source_object_cells: &BTreeSet<(u16, u16)>,
    binary_frame: u32,
    spread_enabled: bool,
    radar_dirty_cells: &mut Vec<(u16, u16)>,
    radar_dirty_generation: &mut u64,
    tactical_dirty_cells: &mut Vec<(u16, u16)>,
) {
    let mut context = crate::sim::tiberium::ReduceTiberiumContext {
        resource_nodes,
        overlay_grid,
        ore_growth_state,
        overlay_registry,
        tiberium_types: Some(&rules.tiberium_types),
        resolved_terrain: terrain,
        source_object_cells: Some(source_object_cells),
        rng: Some(scenario_rng),
        binary_frame,
        spread_enabled,
        radar_dirty_cells: Some(radar_dirty_cells),
        radar_dirty_generation: Some(radar_dirty_generation),
        tactical_dirty_cells: Some(tactical_dirty_cells),
    };
    let _ = crate::sim::tiberium::reduce_tiberium(
        &mut context,
        (request.rx, request.ry),
        request.amount,
    );
}

/// World-owned half of Apply_area_damage's per-cell tiberium prelude for
/// non-combat producers. Overlay/terrain/RNG are lent by AoELayerContext at
/// each spread entry; this value owns only the disjoint Simulation fields the
/// reducer also needs.
pub(crate) struct SimulationTiberiumCellPrelude<'a> {
    rules: &'a RuleSet,
    amount: i32,
    resource_nodes: &'a mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
    ore_growth_state: &'a mut crate::sim::ore_growth::OreGrowthState,
    source_object_cells: &'a BTreeSet<(u16, u16)>,
    binary_frame: u32,
    spread_enabled: bool,
    radar_dirty_cells: &'a mut Vec<(u16, u16)>,
    radar_dirty_generation: &'a mut u64,
    tactical_dirty_cells: &'a mut Vec<(u16, u16)>,
}

impl crate::sim::combat::combat_aoe::AoECellPrelude for SimulationTiberiumCellPrelude<'_> {
    #[allow(clippy::too_many_arguments)]
    fn before_cell(
        &mut self,
        rx: u16,
        ry: u16,
        overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        terrain: Option<&mut ResolvedTerrainGrid>,
        scenario_rng: Option<&mut SimRng>,
    ) {
        if !crate::sim::combat::combat_aoe::tiberium_reduction_cell_admitted(
            overlay_grid.as_deref(),
            overlay_registry,
            rx,
            ry,
        ) {
            return;
        }
        let scenario_rng = scenario_rng
            .expect("production Apply_area_damage tiberium prelude requires scenario RNG");
        dispatch_tiberium_reduction_inline(
            &crate::sim::combat::TiberiumReductionRequest {
                rx,
                ry,
                amount: self.amount,
            },
            self.rules,
            overlay_registry,
            scenario_rng,
            self.resource_nodes,
            overlay_grid,
            terrain,
            self.ore_growth_state,
            self.source_object_cells,
            self.binary_frame,
            self.spread_enabled,
            self.radar_dirty_cells,
            self.radar_dirty_generation,
            self.tactical_dirty_cells,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn simulation_tiberium_cell_prelude<'a>(
    rules: &'a RuleSet,
    warhead: &crate::rules::warhead_type::WarheadType,
    base_damage: i32,
    affect_resource: bool,
    scenario_no_damage: bool,
    resource_nodes: &'a mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
    ore_growth_state: &'a mut crate::sim::ore_growth::OreGrowthState,
    source_object_cells: &'a BTreeSet<(u16, u16)>,
    binary_frame: u32,
    spread_enabled: bool,
    radar_dirty_cells: &'a mut Vec<(u16, u16)>,
    radar_dirty_generation: &'a mut u64,
    tactical_dirty_cells: &'a mut Vec<(u16, u16)>,
) -> Option<SimulationTiberiumCellPrelude<'a>> {
    let amount = base_damage / 10;
    if scenario_no_damage || !affect_resource || !warhead.tiberium || amount <= 0 {
        return None;
    }
    Some(SimulationTiberiumCellPrelude {
        rules,
        amount,
        resource_nodes,
        ore_growth_state,
        source_object_cells,
        binary_frame,
        spread_enabled,
        radar_dirty_cells,
        radar_dirty_generation,
        tactical_dirty_cells,
    })
}

#[allow(clippy::too_many_arguments)]
fn dispatch_smudge_inline(
    request: &crate::sim::combat::SmudgeSpawnRequest,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    interner: &StringInterner,
    occupancy: &OccupancyGrid,
    raw_occupation: &crate::sim::occupancy::RawCellOccupationGrid,
    scenario_rng: &mut SimRng,
    resource_nodes: &mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
    overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
    terrain: Option<&mut ResolvedTerrainGrid>,
    smudge_grid: Option<&mut crate::sim::smudge_grid::SmudgeGrid>,
    ore_growth_state: &mut crate::sim::ore_growth::OreGrowthState,
    source_object_cells: &BTreeSet<(u16, u16)>,
    binary_frame: u32,
    spread_enabled: bool,
    radar_dirty_cells: &mut Vec<(u16, u16)>,
    radar_dirty_generation: &mut u64,
    tactical_dirty_cells: &mut Vec<(u16, u16)>,
) {
    let (Some(smudge_grid), Some(overlay_grid), Some(terrain)) =
        (smudge_grid, overlay_grid, terrain)
    else {
        return;
    };
    let mut tiberium = crate::sim::combat::smudge_dispatch::SmudgeTiberiumContext {
        resource_nodes,
        overlay_grid,
        ore_growth_state,
        overlay_registry,
        tiberium_types: Some(&rules.tiberium_types),
        source_object_cells: Some(source_object_cells),
        binary_frame,
        spread_enabled,
        radar_dirty_cells,
        radar_dirty_generation,
        tactical_dirty_cells,
    };
    crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
        std::slice::from_ref(request),
        &rules.art_registry,
        &rules.smudge_types,
        interner,
        smudge_grid,
        occupancy,
        terrain,
        raw_occupation,
        &mut tiberium,
        scenario_rng,
    );
}

/// Single mutable bridge back into Simulation while combat owns the moved-out
/// receiver transaction. It deliberately implements both lifecycle and smudge
/// callbacks so no pair of closures can alias `&mut Simulation`.
struct SimulationCombatInlineHooks<'a> {
    sim: &'a mut Simulation,
}

impl crate::sim::combat::CombatInlineHooks for SimulationCombatInlineHooks<'_> {
    #[allow(clippy::too_many_arguments)]
    fn respond_to_base_attack(
        &mut self,
        _site: crate::sim::combat::BaseDefenseResponseCallSite,
        victim_id: u64,
        attacker_id: u64,
        borrowed_entities: &mut EntityStore,
        rules: &RuleSet,
        borrowed_interner: &StringInterner,
        borrowed_houses: &mut BTreeMap<InternedId, HouseState>,
        borrowed_scenario_rng: &mut SimRng,
        borrowed_terrain: Option<&ResolvedTerrainGrid>,
    ) {
        let current_frame = self.sim.session.binary_frame as i32;
        let game_mode_nonzero = self.sim.session.game_mode_nonzero;
        let map_size_width = i32::from(self.sim.session.map_width);
        let map_size_height = i32::from(self.sim.session.map_height);
        let playfield_bounds = self.sim.playfield_bounds;
        let mut context =
            crate::sim::combat::base_defense_response::BaseDefenseResponseContext {
                entities: borrowed_entities,
                rules,
                interner: borrowed_interner,
                houses: borrowed_houses,
                alliances: &self.sim.house_alliances,
                scenario_rng: borrowed_scenario_rng,
                teams: &mut self.sim.team_script_vm,
                zone_grid: self.sim.zone_grid.as_ref(),
                terrain: borrowed_terrain,
                playfield_bounds,
                map_size_width,
                map_size_height,
                current_frame,
                game_mode_nonzero,
            };
        crate::sim::combat::base_defense_response::respond_to_base_attack(
            victim_id,
            attacker_id,
            &mut context,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn fatal_lifecycle(
        &mut self,
        rules: &RuleSet,
        stage: crate::sim::combat::FatalLifecycleStage,
        stable_id: u64,
        category: EntityCategory,
        borrowed_entities: &mut EntityStore,
        borrowed_occupancy: &mut OccupancyGrid,
        borrowed_interner: &mut StringInterner,
        borrowed_scenario_rng: &mut SimRng,
        borrowed_terrain: Option<&ResolvedTerrainGrid>,
        mut borrowed_terrain_area_state: Option<&mut crate::sim::terrain_object::TerrainAreaState>,
        borrowed_sound_events: Option<&mut Vec<SimSoundEvent>>,
    ) {
        std::mem::swap(&mut self.sim.substrate.entities, borrowed_entities);
        std::mem::swap(&mut self.sim.substrate.occupancy, borrowed_occupancy);
        std::mem::swap(&mut self.sim.interner, borrowed_interner);
        std::mem::swap(&mut self.sim.scenario_rng, borrowed_scenario_rng);
        if let Some(state) = borrowed_terrain_area_state.as_deref_mut() {
            state.swap_authority(
                &mut self.sim.production,
                &mut self.sim.substrate.raw_cell_occupation,
            );
        }
        if let Some(events) = borrowed_sound_events {
            std::mem::swap(&mut self.sim.sound_events, events);
            self.sim
                .apply_fatal_lifecycle_stage(
                    rules,
                    stage,
                    stable_id,
                    category,
                    borrowed_terrain,
                );
            std::mem::swap(&mut self.sim.sound_events, events);
        } else {
            self.sim
                .apply_fatal_lifecycle_stage(
                    rules,
                    stage,
                    stable_id,
                    category,
                    borrowed_terrain,
                );
        }
        if let Some(state) = borrowed_terrain_area_state.as_deref_mut() {
            state.swap_authority(
                &mut self.sim.production,
                &mut self.sim.substrate.raw_cell_occupation,
            );
        }
        std::mem::swap(&mut self.sim.scenario_rng, borrowed_scenario_rng);
        std::mem::swap(&mut self.sim.interner, borrowed_interner);
        std::mem::swap(&mut self.sim.substrate.occupancy, borrowed_occupancy);
        std::mem::swap(&mut self.sim.substrate.entities, borrowed_entities);
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_tiberium_reduction(
        &mut self,
        rules: &RuleSet,
        request: crate::sim::combat::TiberiumReductionRequest,
        scenario_rng: &mut SimRng,
        resource_nodes: &mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
        overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        terrain: Option<&mut ResolvedTerrainGrid>,
        terrain_area_state: Option<&crate::sim::terrain_object::TerrainAreaState>,
    ) {
        let Some(terrain_area_state) = terrain_area_state else {
            return;
        };
        let binary_frame = self.sim.session.binary_frame;
        let spread_enabled = self.sim.production.ore_growth_config.spreads;
        dispatch_tiberium_reduction_inline(
            &request,
            rules,
            overlay_registry,
            scenario_rng,
            resource_nodes,
            overlay_grid,
            terrain,
            &mut self.sim.production.ore_growth_state,
            terrain_area_state.tiberium_spawning_terrain_cells(),
            binary_frame,
            spread_enabled,
            &mut self.sim.radar_terrain_dirty_cells,
            &mut self.sim.radar_terrain_dirty_generation,
            &mut self.sim.tactical_dirty_cells,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_smudge(
        &mut self,
        rules: &RuleSet,
        request: crate::sim::combat::SmudgeSpawnRequest,
        occupancy: &OccupancyGrid,
        interner: &StringInterner,
        scenario_rng: &mut SimRng,
        resource_nodes: &mut BTreeMap<(u16, u16), crate::sim::miner::ResourceNode>,
        overlay_grid: Option<&mut crate::sim::overlay_grid::OverlayGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        terrain: Option<&mut ResolvedTerrainGrid>,
        terrain_area_state: Option<&crate::sim::terrain_object::TerrainAreaState>,
    ) {
        let Some(terrain_area_state) = terrain_area_state else {
            return;
        };
        let binary_frame = self.sim.session.binary_frame;
        let spread_enabled = self.sim.production.ore_growth_config.spreads;
        dispatch_smudge_inline(
            &request,
            rules,
            overlay_registry,
            interner,
            occupancy,
            terrain_area_state.raw_occupation(),
            scenario_rng,
            resource_nodes,
            overlay_grid,
            terrain,
            self.sim.smudge_grid.as_mut(),
            &mut self.sim.production.ore_growth_state,
            terrain_area_state.tiberium_spawning_terrain_cells(),
            binary_frame,
            spread_enabled,
            &mut self.sim.radar_terrain_dirty_cells,
            &mut self.sim.radar_terrain_dirty_generation,
            &mut self.sim.tactical_dirty_cells,
        );
        self.sim.flush_smudge_dirty();
    }
}

impl Simulation {
    /// Combat borrows a staged house map while fatal lifecycle hooks temporarily
    /// re-enter `Simulation`. Merge only receiver-owned fields back so live
    /// lifecycle mutations to strategy mode, counts, economy, or defeat state
    /// are kept.
    fn merge_receiver_house_state(
        &mut self,
        staged: &BTreeMap<InternedId, crate::sim::house_state::HouseState>,
    ) {
        for (&owner, staged_house) in staged {
            if let Some(live_house) = self.houses.get_mut(&owner) {
                live_house.grudge_scores = staged_house.grudge_scores.clone();
                live_house.enemy_house = staged_house.enemy_house;
                live_house.strategy_emergency.note_building_attack(
                    staged_house
                        .strategy_emergency
                        .last_building_attack_frame(),
                );
                live_house.strategy_emergency.note_building_attacker(
                    staged_house
                        .strategy_emergency
                        .last_attacker_house_index(),
                );
            }
        }
    }

    fn apply_fatal_lifecycle_stage(
        &mut self,
        rules: &RuleSet,
        stage: crate::sim::combat::FatalLifecycleStage,
        stable_id: u64,
        category: EntityCategory,
        terrain: Option<&ResolvedTerrainGrid>,
    ) {
        let uninit_context = UninitContext::with_terrain_and_rules(terrain, rules);
        match stage {
            crate::sim::combat::FatalLifecycleStage::MaintainDamageSmoke { state } => {
                self.maintain_damage_smoke_after_receive(stable_id, state, rules);
            }
            crate::sim::combat::FatalLifecycleStage::PostMortemExactZero { killer_owner } => {
                self.postmortem_exact_zero_callbacks(
                    stable_id,
                    killer_owner,
                    rules,
                    uninit_context,
                );
            }
            crate::sim::combat::FatalLifecycleStage::BeforeDeathEffects => {
                if !matches!(category, EntityCategory::Unit | EntityCategory::Structure) {
                    return;
                }
                let garrison = self.substrate.entities.get(stable_id).and_then(|entity| {
                    if category != EntityCategory::Structure {
                        return None;
                    }
                    let object = rules.object(self.interner.resolve(entity.type_ref))?;
                    let passenger_ids = entity
                        .passenger_role
                        .cargo()
                        .map(|cargo| cargo.passengers.clone())
                        .unwrap_or_default();
                    if !object.can_be_occupied || passenger_ids.is_empty() {
                        return None;
                    }
                    let (foundation_w, foundation_h) =
                        crate::sim::production::foundation_dimensions(&object.foundation);
                    Some(crate::sim::combat::DestroyedGarrisonBuilding {
                        building_id: stable_id,
                        type_id: entity.type_ref,
                        owner: entity.owner,
                        rx: entity.position.rx,
                        ry: entity.position.ry,
                        z: entity.position.z,
                        foundation_w,
                        foundation_h,
                        passenger_ids,
                    })
                });
                if let Some(event) = garrison {
                    production::eject_destruction_garrison_with_context(
                        self,
                        rules,
                        &event,
                        uninit_context,
                    );
                } else {
                    self.purge_carried_passengers_for_fatal(stable_id, uninit_context);
                }
            }
            crate::sim::combat::FatalLifecycleStage::AfterDeathEffects => {
                if !matches!(category, EntityCategory::Unit | EntityCategory::Structure) {
                    return;
                }
                if category == EntityCategory::Structure
                    && self
                        .substrate
                        .entities
                        .get(stable_id)
                        .and_then(|building| building.bunker_occupant)
                        .is_some()
                {
                    crate::sim::docking::bunker_link::release_sell_destroy(self, stable_id);
                }
                self.release_move_sound(stable_id);
                self.uninit_with_context(stable_id, uninit_context);
            }
        }
    }

    fn tick_combat_with_fatal_lifecycle(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
        logic_order: &[u64],
        fire_suppressed: &BTreeSet<u64>,
        projectile_detonations: &[crate::sim::projectile::ProjectileDetonation],
        wave_damage_events: &[crate::sim::wave::WaveDamageEvent],
    ) -> crate::sim::combat::CombatTickResult {
        // Copy-read the pre-resolved handles: Some from init/load in
        // production, None for fixtures. Ticks never resolve or intern.
        let rule_handles = self.rule_handles;
        let mut entities = std::mem::take(&mut self.substrate.entities);
        let mut occupancy = std::mem::take(&mut self.substrate.occupancy);
        let mut interner = std::mem::take(&mut self.interner);
        let mut main_rng = std::mem::replace(&mut self.main_rng, SimRng::new(0));
        let mut scenario_rng = std::mem::replace(&mut self.scenario_rng, SimRng::new(0));
        let mut resource_nodes = std::mem::take(&mut self.production.resource_nodes);
        let mut overlay_grid = self.overlay_grid.take();
        let mut resolved_terrain = self.resolved_terrain.take();
        let bridge_state = self.bridge_state.take();
        let mut radiation = std::mem::take(&mut self.radiation);
        let mut terrain_area_state = crate::sim::terrain_object::TerrainAreaState::take_from(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        let missile_detonations = std::mem::take(&mut self.pending_missile_detonations);
        let mut sound_events = std::mem::take(&mut self.sound_events);
        let mut houses = self.houses.clone();
        let house_order = self.session.house_order.clone();
        let house_alliances = self.house_alliances.clone();
        let fog = self.fog.clone();
        let power_states = self.power_states.clone();
        let current_tick = u64::from(self.session.binary_frame);
        let binary_frame = self.session.binary_frame;
        let scenario_no_damage = self.session.no_damage;
        // Active YR TechnoClass::Evaluate_Candidate @ 0x006F7DB0 reads the
        // candidate's stored Techno+0x3D5 flag at 0x006F7DF1. Only a live
        // MapClass authority makes that native admission rule applicable.
        let require_playfield_membership = self.playfield_bounds.is_some();

        let combat_result = {
            let mut inline_hooks = SimulationCombatInlineHooks { sim: self };
            combat::tick_combat_with_fog_and_main_rng_with_terrain_area(
                &mut entities,
                &mut occupancy,
                rules,
                &mut interner,
                rule_handles,
                Some(&fog),
                &power_states,
                &mut houses,
                &house_order,
                &house_alliances,
                Some(&mut sound_events),
                &mut resource_nodes,
                overlay_grid.as_mut(),
                overlay_registry,
                resolved_terrain.as_mut(),
                bridge_state.as_ref(),
                scenario_no_damage,
                require_playfield_membership,
                current_tick,
                tick_ms,
                binary_frame,
                logic_order,
                fire_suppressed,
                projectile_detonations,
                wave_damage_events,
                Some(&mut radiation),
                &missile_detonations,
                &mut scenario_rng,
                &mut main_rng,
                Some(&mut inline_hooks),
                Some(&mut terrain_area_state),
            )
        };

        let inactive_terrain_logic_ids = terrain_area_state.inactive_logic_ids();
        let terrain_navigation_changed_cells = terrain_area_state.restore_into(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        for stable_id in inactive_terrain_logic_ids {
            let retired = self.retire_non_entity_object(stable_id);
            debug_assert!(retired);
        }

        self.substrate.entities = entities;
        self.substrate.occupancy = occupancy;
        self.interner = interner;
        self.main_rng = main_rng;
        self.scenario_rng = scenario_rng;
        self.production.resource_nodes = resource_nodes;
        self.overlay_grid = overlay_grid;
        self.resolved_terrain = resolved_terrain;
        self.bridge_state = bridge_state;
        self.radiation = radiation;
        self.sound_events = sound_events;
        self.merge_receiver_house_state(&houses);
        let mut combat_result = combat_result;
        combat_result.terrain_navigation_changed_cells = terrain_navigation_changed_cells;
        self.mark_wall_mutations_radar_dirty(&combat_result.wall_mutations);
        combat_result
    }

    /// Commit a completed Bullet's detonation while its current Logic slot and
    /// physical object are still present. The caller retires the Bullet only
    /// after this transaction returns.
    fn commit_logic_projectile_detonations(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        detonations: &[crate::sim::projectile::ProjectileDetonation],
    ) {
        if detonations.is_empty() {
            return;
        }
        // Copy-read the pre-resolved handles (Some in production, None for
        // fixtures); ticks never resolve or intern.
        let rule_handles = self.rule_handles;

        let mut entities = std::mem::take(&mut self.substrate.entities);
        let mut occupancy = std::mem::take(&mut self.substrate.occupancy);
        let mut interner = std::mem::take(&mut self.interner);
        let mut main_rng = std::mem::replace(&mut self.main_rng, SimRng::new(0));
        let mut scenario_rng = std::mem::replace(&mut self.scenario_rng, SimRng::new(0));
        let mut resource_nodes = std::mem::take(&mut self.production.resource_nodes);
        let mut overlay_grid = self.overlay_grid.take();
        let mut resolved_terrain = self.resolved_terrain.take();
        let bridge_state = self.bridge_state.take();
        let mut sound_events = std::mem::take(&mut self.sound_events);
        let mut terrain_area_state = crate::sim::terrain_object::TerrainAreaState::take_from(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        let mut houses = self.houses.clone();
        let house_order = self.session.house_order.clone();
        let house_alliances = self.house_alliances.clone();
        let current_tick = u64::from(self.session.binary_frame);
        let scenario_no_damage = self.session.no_damage;

        let commit = {
            let mut inline_hooks = SimulationCombatInlineHooks { sim: self };
            combat::commit_logic_projectile_detonations(
                detonations,
                &mut entities,
                &mut occupancy,
                rules,
                &mut interner,
                rule_handles,
                &mut houses,
                &house_order,
                &house_alliances,
                &mut main_rng,
                &mut scenario_rng,
                &mut resource_nodes,
                overlay_grid.as_mut(),
                overlay_registry,
                resolved_terrain.as_mut(),
                bridge_state.as_ref(),
                Some(&mut terrain_area_state),
                scenario_no_damage,
                current_tick,
                Some(&mut sound_events),
                Some(&mut inline_hooks),
            )
        };

        let inactive_terrain_logic_ids = terrain_area_state.inactive_logic_ids();
        let terrain_navigation_changed_cells = terrain_area_state.restore_into(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        for stable_id in inactive_terrain_logic_ids {
            let retired = self.retire_non_entity_object(stable_id);
            debug_assert!(retired);
        }

        self.substrate.entities = entities;
        self.substrate.occupancy = occupancy;
        self.interner = interner;
        self.main_rng = main_rng;
        self.scenario_rng = scenario_rng;
        self.production.resource_nodes = resource_nodes;
        self.overlay_grid = overlay_grid;
        self.resolved_terrain = resolved_terrain;
        self.bridge_state = bridge_state;
        self.sound_events = sound_events;
        self.merge_receiver_house_state(&houses);

        for projectile in commit.projectile_spawns {
            let stable_id = self.allocate_stable_id();
            self.admit_projectile(stable_id, projectile);
        }
        self.absorb_noncombat_damage_effects(
            rules,
            overlay_registry,
            commit.effects,
            commit.under_attack_events,
            terrain_navigation_changed_cells,
        );
    }

    /// Walk one Wave damage request in recorded-cell and current Cell-list
    /// order. Each receiver commits before the next occupant is selected, so
    /// fatal UnInit and nested effects are visible to the remaining walk.
    fn commit_logic_wave_damage_request(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        request: &crate::sim::wave::WaveDamageRequest,
    ) {
        for cell in &request.recorded_cells {
            let layer = self
                .resolved_terrain
                .as_ref()
                .and_then(|terrain| terrain.cell(cell.rx, cell.ry))
                .map(|terrain_cell| {
                    if crate::sim::cell_kernel::selects_infantry_bridge_layer(
                        terrain_cell.bridge_facts.has_structural_bridge(),
                        terrain_cell.level,
                        request.wave_z,
                    ) {
                        MovementLayer::Bridge
                    } else {
                        MovementLayer::Ground
                    }
                })
                .unwrap_or(MovementLayer::Ground);

            // Re-read the native list after every concrete receiver. `visited`
            // prevents a surviving current object from being selected again;
            // it does not preserve objects removed by an earlier callback.
            let mut visited = Vec::new();
            loop {
                let next_id = self
                    .substrate
                    .occupancy
                    .get(cell.rx, cell.ry)
                    .and_then(|occupancy| {
                        occupancy
                            .iter_layer(layer)
                            .find(|occupant| !visited.contains(&occupant.entity_id))
                    })
                    .map(|occupant| occupant.entity_id);
                let Some(target_id) = next_id else { break };
                visited.push(target_id);
                if target_id == request.payload.firer_id {
                    continue;
                }
                let Some(entity) = self.substrate.entities.get(target_id) else {
                    continue;
                };
                if !entity.is_alive() || entity.dying || entity.lifecycle.in_limbo {
                    continue;
                }
                let event = crate::sim::wave::WaveDamageEvent {
                    wave_id: request.wave_id,
                    target_id,
                    payload: request.payload,
                };
                let receiver = crate::sim::combat::combat_aoe::AreaDamageReceiver::Entity(
                    crate::sim::combat::EntityDamageEvent::from_wave(
                        event,
                        &self.substrate.entities,
                    ),
                );
                #[cfg(test)]
                self.trace_lifecycle_for_test(LifecycleTestEvent::WaveDamageReceiverSelected {
                    wave_id: request.wave_id,
                    target_id,
                });
                self.commit_noncombat_aoe_receivers(
                    rules,
                    overlay_registry,
                    std::slice::from_ref(&receiver),
                );
            }
        }
    }

    /// Commit one non-combat Apply_area_damage hit list through the ordinary
    /// ReceiveDamage -> death transaction, then hand its deferred outputs back
    /// to the world owner. `hits` remains in CellClass/object-list order; the
    /// combat helper enters a nested DeathWeapon before advancing to the next
    /// hit.
    pub(crate) fn commit_noncombat_aoe_hits(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        hits: &[crate::sim::combat::EntityDamageEvent],
    ) {
        let receivers = hits
            .iter()
            .copied()
            .map(crate::sim::combat::combat_aoe::AreaDamageReceiver::Entity)
            .collect::<Vec<_>>();
        self.commit_noncombat_aoe_receivers(rules, overlay_registry, &receivers);
    }

    /// World-owned entry for a complete non-combat Apply_area_damage receiver
    /// vector, including TerrainClass records in captured object-list order.
    pub(crate) fn commit_noncombat_aoe_receivers(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        receivers: &[crate::sim::combat::combat_aoe::AreaDamageReceiver],
    ) {
        let rule_handles = self.rule_handles;
        let mut entities = std::mem::take(&mut self.substrate.entities);
        let mut occupancy = std::mem::take(&mut self.substrate.occupancy);
        let mut interner = std::mem::take(&mut self.interner);
        let mut main_rng = std::mem::replace(&mut self.main_rng, SimRng::new(0));
        let mut scenario_rng = std::mem::replace(&mut self.scenario_rng, SimRng::new(0));
        let mut resource_nodes = std::mem::take(&mut self.production.resource_nodes);
        let mut overlay_grid = self.overlay_grid.take();
        let mut resolved_terrain = self.resolved_terrain.take();
        let mut sound_events = std::mem::take(&mut self.sound_events);
        let mut terrain_area_state = crate::sim::terrain_object::TerrainAreaState::take_from(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        let mut houses = self.houses.clone();
        let house_order = self.session.house_order.clone();
        let house_alliances = self.house_alliances.clone();
        let current_tick = u64::from(self.session.binary_frame);
        let scenario_no_damage = self.session.no_damage;
        let mut handled_deaths = Vec::new();
        let (effects, under_attack_events) = {
            let mut inline_hooks = SimulationCombatInlineHooks { sim: self };
            let mut inline_hooks: Option<&mut dyn crate::sim::combat::CombatInlineHooks> =
                Some(&mut inline_hooks);
            let mut sound_sink = Some(&mut sound_events);
            crate::sim::combat::commit_area_damage_receivers_with_scenario(
                receivers,
                &mut entities,
                &mut occupancy,
                rules,
                &mut interner,
                rule_handles,
                &mut houses,
                &house_order,
                &house_alliances,
                &mut main_rng,
                &mut scenario_rng,
                &mut handled_deaths,
                &mut resource_nodes,
                overlay_grid.as_mut(),
                overlay_registry,
                resolved_terrain.as_mut(),
                Some(&mut terrain_area_state),
                scenario_no_damage,
                current_tick,
                &mut inline_hooks,
                &mut sound_sink,
            )
        };
        let inactive_terrain_logic_ids = terrain_area_state.inactive_logic_ids();
        let terrain_navigation_changed_cells = terrain_area_state.restore_into(
            &mut self.production,
            &mut self.substrate.raw_cell_occupation,
        );
        for stable_id in inactive_terrain_logic_ids {
            let retired = self.retire_non_entity_object(stable_id);
            debug_assert!(retired);
        }
        self.substrate.entities = entities;
        self.substrate.occupancy = occupancy;
        self.interner = interner;
        self.main_rng = main_rng;
        self.scenario_rng = scenario_rng;
        self.production.resource_nodes = resource_nodes;
        self.overlay_grid = overlay_grid;
        self.resolved_terrain = resolved_terrain;
        self.sound_events = sound_events;
        self.merge_receiver_house_state(&houses);
        self.absorb_noncombat_damage_effects(
            rules,
            overlay_registry,
            effects,
            under_attack_events,
            terrain_navigation_changed_cells,
        );
    }

    /// World-owned half of a non-combat damage transaction. Physical death
    /// consequences already happened recursively in combat; this consumes the
    /// same lifecycle/presentation/terrain outputs without leaving an alternate
    /// zero-HP object registered for the next LogicClass visit.
    fn absorb_noncombat_damage_effects(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        mut effects: crate::sim::combat::DeathEffects,
        under_attack_events: Vec<crate::sim::combat::UnderAttackEvent>,
        terrain_navigation_changed_cells: Vec<(u16, u16)>,
    ) {
        // gamemd-derived: `BulletClass::AI @ 0x004666E0` reaches
        // `CellClass::DestroyOverlay @ 0x00480CB0` inside the Bullet's Logic
        // slot, where terminal removal calls `MarkTerrainDirty @ 0x004807C2`
        // synchronously. Combat owns the detached overlay mutation; this is the
        // first restored-world consumer of `DeathEffects`. The caller's earlier
        // shrapnel admission preserves Detonate's shrapnel-before-DamageArea
        // ordering; publish the trace before consuming later death effects.
        self.mark_wall_mutations_radar_dirty(&effects.wall_mutations);

        let dead_infos: Vec<(InternedId, EntityCategory)> = effects
            .despawned_ids
            .iter()
            .filter_map(|&dead_id| {
                self.substrate
                    .entities
                    .get(dead_id)
                    .map(|entity| (entity.owner, entity.category))
            })
            .collect();

        for event in &effects.destroyed_garrison_buildings {
            production::eject_destruction_garrison(self, rules, event);
        }
        for &dead_id in &effects.immediate_uninit_ids {
            if self
                .substrate
                .entities
                .get(dead_id)
                .and_then(|building| building.bunker_occupant)
                .is_some()
            {
                crate::sim::docking::bunker_link::release_sell_destroy(self, dead_id);
            }
            self.release_move_sound(dead_id);
            self.uninit_with_rules(dead_id, rules);
        }

        let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events_with_overlay_registry(
            self,
            rules,
            &effects.bridge_damage_events,
            overlay_registry,
        );
        debug_assert!(effects.tiberium_reduction_requests.is_empty());
        for request in &effects.tiberium_reduction_requests {
            self.reduce_tiberium_at_with_native_context(
                (request.rx, request.ry),
                request.amount,
                Some(rules),
                overlay_registry,
            );
        }
        for detonation in effects.rad_detonations.drain(..) {
            self.radiation.apply_detonation(
                detonation,
                self.session.binary_frame,
                &rules.radiation,
                self.resolved_terrain.as_ref(),
            );
        }

        let mut navigation_changed_cells = self
            .overlay_grid
            .as_mut()
            .map(|grid| grid.take_synchronous_navigation_cells())
            .unwrap_or_default();
        for cell in terrain_navigation_changed_cells {
            if !navigation_changed_cells.contains(&cell) {
                navigation_changed_cells.push(cell);
            }
        }
        if !navigation_changed_cells.is_empty() {
            let prior_path_grid = self.path_grid.as_deref().cloned();
            let _ = self.refresh_navigation_after_terrain_changes(
                prior_path_grid.as_ref(),
                &navigation_changed_cells,
            );
        }

        for building in &effects.destroyed_crewed_buildings {
            production::eject_destruction_survivors(
                self,
                rules,
                building.type_id,
                building.owner,
                building.rx,
                building.ry,
                building.z,
            );
        }
        if self.session.game_options.super_weapons && effects.structure_destroyed {
            let mut refreshed = Vec::new();
            for &(owner, category) in &dead_infos {
                if category == EntityCategory::Structure && !refreshed.contains(&owner) {
                    refreshed.push(owner);
                    crate::sim::superweapon::refresh_super_weapons_for_owner(self, rules, owner);
                }
            }
        }

        for fx in &effects.explosion_effects {
            let frames = rules
                .effect_frame_count(self.interner.resolve(fx.shp_name))
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
                frame_delay: 1,
                elapsed_frames: 0,
                translucent: true,
                delay_frames: 0,
                start_sound_id: None,
                start_sound_emitted: false,
            });
        }
        self.invulnerability_impact_effects
            .append(&mut effects.invulnerability_impact_effects);
        for (die_sound_id, rx, ry) in effects.death_sounds.drain(..) {
            self.sound_events.push(SimSoundEvent::EntityDied {
                die_sound_id,
                rx,
                ry,
            });
        }
        for event in under_attack_events {
            let event_type = if event.miner {
                RadarEventType::HarvesterUnderAttack
            } else {
                RadarEventType::BaseUnderAttack
            };
            let eva_allowed =
                self.radar_events
                    .push_owned(event_type, event.rx, event.ry, Some(event.owner));
            self.sound_events.push(SimSoundEvent::UnderAttack {
                rx: event.rx,
                ry: event.ry,
                owner: event.owner,
                miner: event.miner,
                eva_allowed,
            });
        }
        debug_assert!(effects.smudge_spawn_requests.is_empty());
        self.pending_smudge_requests
            .append(&mut effects.smudge_spawn_requests);
    }

    /// Borrow all three logical RNG objects without exposing mutation.
    pub fn rng_views(&self) -> SimulationRngViews<'_> {
        SimulationRngViews {
            scenario: self.scenario_rng.logical_view(),
            main: self.main_rng.logical_view(),
            mapgen: self.mapgen_rng.logical_view(),
        }
    }

    /// Summarise this tick for cross-engine parity comparison.
    ///
    /// Read-only and side-effect free: the caller decides whether to record it, so
    /// enabling capture cannot perturb the run being measured. The RNG cursor comes from
    /// the main stream, which is the one the original engine exposes globally.
    pub fn parity_digest(&self) -> crate::sim::parity_digest::ParityDigest {
        let views = self.rng_views();
        crate::sim::parity_digest::ParityDigest::capture(
            self.session.tick,
            self.entities(),
            &self.houses,
            views.main.index_a,
            views.main.index_b,
            views.scenario.index_a,
            views.scenario.index_b,
        )
    }

    /// Capture all three complete logical RNG objects as immutable evidence.
    pub fn rng_state(&self) -> SimulationRngState {
        SimulationRngState {
            scenario: self.scenario_rng.logical_state(),
            main: self.main_rng.logical_state(),
            mapgen: self.mapgen_rng.logical_state(),
        }
    }

    /// Complete the in-scenario Load Game handoff with process-global state
    /// that is live at load time. Scenario RNG has already been reset by
    /// deserialization and is deliberately untouched here. The cold front-end
    /// load route has a distinct native handoff and does not use this seam.
    pub(crate) fn retain_in_scenario_process_state_from(&mut self, live: &Self) {
        self.session.seed = live.session.seed;
        self.main_rng = live.main_rng.clone();
        self.mapgen_rng = live.mapgen_rng.clone();
        self.bind_shared_cell_dummy(live.effective_shared_cell_dummy());
    }

    /// Apply the successful load's native MapClass Resize reconstruction to
    /// every modeled field of the fixed fallback CellClass.
    ///
    /// `MouseClass::Load @ 0x005BE150` routes restored dimensions through
    /// `MapClass::Resize @ 0x00565C10`, whose unconditional call to
    /// `CellClass::Constructor @ 0x0047BBF0` reconstructs the fixed dummy in
    /// place. The app invokes this only after fallible candidate preparation,
    /// so a rejected transactional load cannot mutate the running world.
    pub(crate) fn reconstruct_cellclass_dummy_for_map_resize(&mut self) {
        self.effective_shared_cell_dummy()
            .reconstruct_for_map_resize();
        self.substrate
            .base_reservations
            .reconstruct_dummy_for_map_resize();
    }

    /// Adopt the process-global CellClass identity already bound to a load's
    /// resolved grid, before that grid is cloned into Simulation.
    pub(crate) fn bind_shared_cell_dummy(&mut self, shared_cell_dummy: SharedCellDummy) {
        self.shared_cell_dummy = shared_cell_dummy.clone();
        if let Some(terrain) = self.resolved_terrain.as_mut() {
            terrain.bind_shared_cell_dummy(shared_cell_dummy);
        }
    }

    /// Install a newly constructed map grid and capture the real CellClass
    /// `0x1180` value authority after row-major OverlayPack marking.
    pub(crate) fn install_resolved_terrain_for_new_map(
        &mut self,
        resolved_terrain: ResolvedTerrainGrid,
    ) {
        self.bind_shared_cell_dummy(resolved_terrain.shared_cell_dummy());
        self.real_cell_bridge_flags_0x1180 =
            resolved_terrain.capture_real_cell_bridge_flags_0x1180();
        self.resolved_terrain = Some(resolved_terrain);
    }

    /// Apply one live runtime setter and update only the allocated real-cell
    /// serialized values it changed. Missing slots remain owned solely by the
    /// process dummy and are intentionally absent from Scenario payload.
    pub(crate) fn apply_runtime_bridge_flag_stamp(
        &mut self,
        stamp: crate::map::bridge_facts::BridgeFlagStamp,
    ) {
        let Some(terrain) = self.resolved_terrain.as_ref() else {
            return;
        };
        if !terrain.bridge_flag_authority_matches_shape(&self.real_cell_bridge_flags_0x1180) {
            self.real_cell_bridge_flags_0x1180 = terrain.capture_real_cell_bridge_flags_0x1180();
        }
        let updates = self
            .resolved_terrain
            .as_mut()
            .expect("terrain presence checked before runtime bridge setter")
            .apply_runtime_bridge_flag_stamp(stamp);
        for (index, flags) in updates {
            self.real_cell_bridge_flags_0x1180
                .set_allocated_cell(index, flags);
        }
    }

    /// Commit the allocated real-cell half of a runtime setter that already
    /// executed synchronously through `CellClassBridgeFlagState`. Dummy
    /// coordinate/flag effects are live at the native call point and must not
    /// be replayed here.
    pub(crate) fn apply_planned_bridge_flag_stamp_to_real_cells(
        &mut self,
        stamp: crate::map::bridge_facts::BridgeFlagStamp,
    ) {
        let Some(terrain) = self.resolved_terrain.as_ref() else {
            return;
        };
        if !terrain.bridge_flag_authority_matches_shape(&self.real_cell_bridge_flags_0x1180) {
            self.real_cell_bridge_flags_0x1180 = terrain.capture_real_cell_bridge_flags_0x1180();
        }
        let updates = self
            .resolved_terrain
            .as_mut()
            .expect("terrain presence checked before planned bridge setter projection")
            .apply_planned_bridge_flag_stamp_to_real_cells(stamp);
        for (index, flags) in updates {
            self.real_cell_bridge_flags_0x1180
                .set_allocated_cell(index, flags);
        }
    }

    /// Synthetic fixtures may assign a detached grid directly. Production
    /// construction binds both owners, but gameplay must still read the live
    /// handle attached to the actual CellClass table it queried.
    pub(crate) fn effective_shared_cell_dummy(&self) -> SharedCellDummy {
        self.resolved_terrain
            .as_ref()
            .map(ResolvedTerrainGrid::shared_cell_dummy)
            .unwrap_or_else(|| self.shared_cell_dummy.clone())
    }

    /// Install the Scenario cursor advanced by the pre-IsoMapPack Fill pass.
    /// Called only by sim's opaque bootstrap owner; Main remains independently
    /// owned by the terrain variant selector.
    pub(super) fn install_terrain_load_advanced_scenario_rng(&mut self, scenario_rng: SimRng) {
        self.scenario_rng = scenario_rng;
    }

    /// Install the Main cursor advanced by one-time terrain variant-table
    /// generation. Called only by sim's opaque bootstrap owner; Scenario
    /// remains independently owned by the Fill pass.
    pub(super) fn install_variant_advanced_main_rng(&mut self, main_rng: SimRng) {
        self.main_rng = main_rng;
    }

    /// Install the exact cursor left by the accepted random-map generation.
    /// VERA fixed-map construction currently keeps `Random__Seed(0)`; native
    /// fresh-process state is verified, while cross-match retention is UNCHECKED.
    pub(super) fn install_generated_mapgen_rng(&mut self, mapgen_rng: SimRng) {
        self.mapgen_rng = mapgen_rng;
    }

    /// Create a new empty simulation with the default deterministic seed.
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SIM_SEED)
    }

    /// Build the interned-id -> type-handle table and the pre-resolved rule
    /// handles beside it. Call once at sim init AFTER `intern_rule_type_ids`,
    /// and again after load once the RuleSet is available. Idempotent: the
    /// warhead names are already interned on every re-resolution path, so
    /// serialized interner state cannot grow or reorder.
    pub fn resolve_type_handles(&mut self, rules: &RuleSet) {
        self.type_handles =
            crate::sim::type_handle_table::TypeHandleTable::build(rules, &self.interner);
        self.rule_handles = Some(crate::sim::type_handle_table::ResolvedRuleHandles::resolve(
            rules,
            &mut self.interner,
        ));
    }

    /// Intern every rules type id (infantry, vehicle, aircraft, building) so
    /// `interner.get(type_id)` succeeds for any type this ruleset references.
    /// Moved from `RuleSet::intern_all_ids` (F04): interning is sim-side work
    /// over rules-owned canonical names.
    pub fn intern_rule_type_ids(&mut self, rules: &RuleSet) {
        for id in rules
            .infantry_ids
            .iter()
            .chain(&rules.vehicle_ids)
            .chain(&rules.aircraft_ids)
            .chain(&rules.building_ids)
        {
            self.interner.intern(id);
        }
    }

    /// Pre-resolved rule handles for combat comparisons.
    ///
    /// # Panics
    /// Panics if `resolve_type_handles` has not run (init and load both call it
    /// before any combat tick).
    pub fn rule_handles(&self) -> crate::sim::type_handle_table::ResolvedRuleHandles {
        self.rule_handles.expect(
            "Simulation::resolve_type_handles must run at sim init \
             before combat reads warhead handles",
        )
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
            None if self.type_handles.is_empty() => rules.object(self.interner.resolve(type_ref)),
            None => None,
        }
    }

    /// Create a new empty simulation with an explicit deterministic seed.
    /// Test/dev entry — u64 seeds wider than 32 bits keep their full value in
    /// `session.seed` (pinned harness baselines depend on it) even though the
    /// stream seeder consumes only 32 bits.
    pub fn with_seed(seed: u64) -> Self {
        let mut session = ScenarioSession::from_descriptor(
            &crate::sim::scenario_session::ScenarioDescriptor::default(),
        );
        session.seed = seed;
        Self::construct(session)
    }

    /// Construct a session simulation from an app-layer launch descriptor.
    /// The only entry real launches use; `new()`/`with_seed()` remain for
    /// tests and dev tooling.
    pub fn from_descriptor(desc: &crate::sim::scenario_session::ScenarioDescriptor) -> Self {
        Self::construct(ScenarioSession::from_descriptor(desc))
    }

    /// Shared constructor body: seed both gameplay streams identically from
    /// the session seed and construct fresh MapGen with native Seed(0) state.
    fn construct(session: ScenarioSession) -> Self {
        let seed = session.seed;
        let mut out = Self {
            interner: crate::sim::intern::StringInterner::new(),
            type_handles: crate::sim::type_handle_table::TypeHandleTable::default(),
            rule_handles: None,
            production: ProductionState::default(),
            session,
            scenario_rng: SimRng::new(seed),
            main_rng: SimRng::new(seed),
            mapgen_rng: SimRng::new(0),
            fog: FogState::default(),
            house_alliances: HouseAllianceMap::default(),
            substrate: ObjectSubstrate::new(),
            lifecycle_outputs: Vec::new(),
            frame_overlay_updates: Vec::new(),
            terminal_score_snapshot: None,
            pending_lifecycle_requests: Vec::new(),
            pending_rocket_detonations: Vec::new(),
            pending_missile_detonations: Vec::new(),
            pending_projectile_detonations: Vec::new(),
            pending_wave_damage_requests: Vec::new(),
            #[cfg(test)]
            lifecycle_test_events: Vec::new(),
            trigger_effects: Vec::new(),
            #[cfg(test)]
            master_frame_test_trace: Vec::new(),
            sound_events: Vec::new(),
            fire_events: Vec::new(),
            invulnerability_impact_effects: Vec::new(),
            projectiles: crate::sim::projectile::ProjectileStore::new(),
            waves: crate::sim::wave::WaveStore::new(),
            pending_smudge_requests: Vec::new(),
            bale_events: Vec::new(),
            bunker_wall_events: Vec::new(),
            ai_players: Vec::new(),
            team_script_vm: TeamScriptVm::default(),
            houses: BTreeMap::new(),
            terrain_costs: BTreeMap::new(),
            zone_grid: None,
            path_grid: None,
            resolved_terrain: None,
            shared_cell_dummy: SharedCellDummy::fresh(),
            real_cell_bridge_flags_0x1180: RealCellBridgeFlags0x1180::default(),
            bridge_state: None,
            overlay_grid: None,
            smudge_grid: None,
            radiation: crate::sim::radiation::RadiationState::default(),
            playfield_bounds: None,
            playfield_size_height: None,
            playfield_revision: 0,
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
            super_weapons_initialized: false,
            terrain_speed_config: terrain_speed::TerrainSpeedConfig::default(),
            close_enough: SimFixed::from_num(576), // 2.25 cells × 256 lep/cell
            path_delay_ticks: 9,
            blockage_path_delay_ticks: 60,
            world_effects: Vec::new(),
            debug_event_logging: false,
            input_delay_ticks: 2,
            quit_requested: false,
            executed_exit_owner: None,
            connection_lost: false,
            pending_commands: Vec::new(),
            trigger_runtime: TriggerRuntime::default(),
        };
        debug_assert_eq!(out.scenario_rng.state(), out.main_rng.state());
        // Authoritative map bounds are session state, known at construction —
        // vision must never run against a zero-dim first-tick window. Fixture
        // sims built without a descriptor (zero bounds) keep the lazy
        // derivation as a fallback inside the vision recompute.
        out.fog.width = out.session.map_width;
        out.fog.height = out.session.map_height;
        out
    }

    // --- Scenario stream (gamemd Scenario->Random @ Scen+0x218) ---
    // Keep accessors distinct even though several return the same stream today:
    // the intent name is the per-consumer routing record and the grep/audit anchor.
    pub(crate) fn scatter_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // bump displacement, idle/forced scatter, passenger unload exit, sell-eject
    #[allow(dead_code)] // Stream-routing audit anchor; callers currently co-borrow the field.
    pub(crate) fn subcell_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // infantry sub-cell rotation, paradrop sub-cell
    #[allow(dead_code)] // Stream-routing audit anchor; callers currently co-borrow the field.
    pub(crate) fn smudge_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // destruction smudge/survivor/debris, smudge type pick
    #[allow(dead_code)] // Stream-routing audit anchor; callers currently co-borrow the field.
    pub(crate) fn wall_damage_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // overlay/wall damage roll
    pub(crate) fn bridge_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // bridge collapse/debris/explosion
    #[allow(dead_code)] // Stream-routing audit anchor; callers currently co-borrow the field.
    pub(crate) fn ore_rng(&mut self) -> &mut SimRng {
        &mut self.scenario_rng
    } // ore growth/spread queue + direction + variant, TIBTRE
    #[allow(dead_code)] // Stream-routing audit anchor; callers currently co-borrow the field.
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
    /// Capture the process-continuity Scenario cursor when gameplay returns to
    /// the frontend. The app stores this clone until the next successful Start
    /// reseeds the gameplay pair.
    pub(crate) fn clone_scenario_rng(&self) -> SimRng {
        self.scenario_rng.clone()
    }

    // --- Main/global gameplay stream ---
    #[allow(dead_code)] // Named Main-stream audit anchor retained beside direct borrows.
    pub(crate) fn weapon_spread_rng(&mut self) -> &mut SimRng {
        &mut self.main_rng
    } // verified main-only weapon/warhead property rolls; not detonation scatter
    #[allow(dead_code)] // Named Main-stream audit anchor for the staged House AI consumer.
    pub(crate) fn house_ai_rng(&mut self) -> &mut SimRng {
        &mut self.main_rng
    } // HouseClass superpower/AI gate roll

    /// Test/replay helper for the per-game Scenario/Main pair only.
    ///
    /// Same-process MapGen reset/retention is unverified, so reseeding the
    /// per-game pair must preserve the current MapGen object.
    #[cfg(test)]
    pub(crate) fn reseed_scenario_and_main(&mut self, seed: u64) {
        self.scenario_rng = SimRng::new(seed);
        self.main_rng = SimRng::new(seed);
        self.session.seed = seed;
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

    /// Current ObjectClass registration order used by the tactical layer.
    /// Presentation code may sort registered objects into LayerClass order,
    /// but must not reconstruct equal-key ordering from EntityStore keys.
    pub(crate) fn tactical_registration_order(&self) -> &[u64] {
        self.substrate.logic.as_slice()
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

    pub(crate) fn flush_smudge_dirty(&mut self) {
        let dirty = self
            .smudge_grid
            .as_mut()
            .map(crate::sim::smudge_grid::SmudgeGrid::drain_dirty)
            .unwrap_or_default();
        for cell in dirty {
            self.tactical_dirty_cells.push(cell);
            self.mark_radar_terrain_dirty_cells([cell]);
        }
    }

    /// Commit a non-combat AnimClass smudge at its producer boundary. Combat
    /// uses `SimulationCombatInlineHooks` because its map authority is moved
    /// out; superweapons call this after their receiver transaction restores
    /// the same fields to Simulation.
    pub(crate) fn commit_smudge_request_inline(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        request: crate::sim::combat::SmudgeSpawnRequest,
    ) {
        let binary_frame = self.session.binary_frame;
        let spread_enabled = self.production.ore_growth_config.spreads;
        dispatch_smudge_inline(
            &request,
            rules,
            overlay_registry,
            &self.interner,
            &self.substrate.occupancy,
            &self.substrate.raw_cell_occupation,
            &mut self.scenario_rng,
            &mut self.production.resource_nodes,
            self.overlay_grid.as_mut(),
            self.resolved_terrain.as_mut(),
            self.smudge_grid.as_mut(),
            &mut self.production.ore_growth_state,
            &self.production.tiberium_spawning_terrain_cells,
            binary_frame,
            spread_enabled,
            &mut self.radar_terrain_dirty_cells,
            &mut self.radar_terrain_dirty_generation,
            &mut self.tactical_dirty_cells,
        );
        self.flush_smudge_dirty();
    }

    #[cfg(test)]
    pub(crate) fn reduce_legacy_tiberium_at_for_tests(
        &mut self,
        cell: (u16, u16),
        amount: u16,
    ) -> crate::sim::tiberium::ReduceTiberiumOutcome {
        let mut ctx = crate::sim::tiberium::ReduceTiberiumContext {
            resource_nodes: &mut self.production.resource_nodes,
            overlay_grid: self.overlay_grid.as_mut(),
            ore_growth_state: &mut self.production.ore_growth_state,
            overlay_registry: None,
            tiberium_types: None,
            resolved_terrain: self.resolved_terrain.as_mut(),
            source_object_cells: Some(&self.production.tiberium_spawning_terrain_cells),
            rng: Some(&mut self.scenario_rng),
            binary_frame: self.session.binary_frame,
            spread_enabled: self.production.ore_growth_config.spreads,
            radar_dirty_cells: Some(&mut self.radar_terrain_dirty_cells),
            radar_dirty_generation: Some(&mut self.radar_terrain_dirty_generation),
            tactical_dirty_cells: Some(&mut self.tactical_dirty_cells),
        };
        crate::sim::tiberium::reduce_legacy_resource_node_for_tests(
            &mut ctx,
            cell,
            i32::from(amount),
        )
    }

    pub(crate) fn reduce_tiberium_at_with_native_context<A: Into<i32>>(
        &mut self,
        cell: (u16, u16),
        amount: A,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> crate::sim::tiberium::ReduceTiberiumOutcome {
        let mut ctx = crate::sim::tiberium::ReduceTiberiumContext {
            resource_nodes: &mut self.production.resource_nodes,
            overlay_grid: self.overlay_grid.as_mut(),
            ore_growth_state: &mut self.production.ore_growth_state,
            overlay_registry,
            tiberium_types: rules.map(|rules| &rules.tiberium_types),
            resolved_terrain: self.resolved_terrain.as_mut(),
            source_object_cells: Some(&self.production.tiberium_spawning_terrain_cells),
            // ore growth/spread — scenario stream. Direct field (not ore_rng()): this
            // literal co-borrows other &mut self fields, so the all-self accessor conflicts.
            rng: Some(&mut self.scenario_rng),
            binary_frame: self.session.binary_frame,
            spread_enabled: self.production.ore_growth_config.spreads,
            radar_dirty_cells: Some(&mut self.radar_terrain_dirty_cells),
            radar_dirty_generation: Some(&mut self.radar_terrain_dirty_generation),
            tactical_dirty_cells: Some(&mut self.tactical_dirty_cells),
        };
        crate::sim::tiberium::reduce_tiberium(&mut ctx, cell, amount.into())
    }

    /// Intern a string, returning its InternedId.
    #[inline]
    pub fn intern(&mut self, s: &str) -> crate::sim::intern::InternedId {
        self.interner.intern(s)
    }

    /// Queue one already-prepared command for future execution.
    ///
    /// This is an append-only ingress: it preserves the command owner, execution
    /// tick, and insertion position exactly. It does not sort, retime, normalize,
    /// intern, deduplicate, or apply the negotiated input delay.
    pub fn queue_command(&mut self, command: CommandEnvelope) {
        self.queue_commands([command]);
    }

    /// Queue an already-ordered batch for future execution.
    ///
    /// Commands are appended in iterator order with their owners, execution
    /// ticks, and payloads unchanged. Native byte round-trips and network timing
    /// remain the responsibility of the producer that prepares each envelope.
    pub fn queue_commands(&mut self, commands: impl IntoIterator<Item = CommandEnvelope>) {
        self.pending_commands.extend(commands);
    }

    /// Read-only queue evidence for module tests outside the owning module.
    #[cfg(test)]
    pub(crate) fn pending_commands_for_tests(&self) -> &[CommandEnvelope] {
        &self.pending_commands
    }

    /// Speed the next ordinary frame will observe after all due offline
    /// Options transitions execute in canonical house/insertion order.
    ///
    /// This read-only projection lets fresh-map and snapshot replacement sync
    /// client UI without consuming or applying a pending authoritative command.
    pub(crate) fn projected_in_game_options_speed(&self) -> Option<u8> {
        let mut projected = u8::try_from(self.session.game_options.game_speed)
            .ok()
            .filter(|speed| *speed <= crate::sim::game_options::IN_GAME_OPTIONS_MAX_SPEED);
        let execute_tick = self.session.tick.saturating_add(1);
        for owner in self.due_command_house_order(&self.pending_commands, execute_tick) {
            for command in self.pending_commands.iter().filter(|command| {
                command.execute_tick <= execute_tick && command.owner == owner
            }) {
                if !self.houses.contains_key(&owner) {
                    continue;
                }
                let Command::SetGameSpeed { speed } = &command.payload else {
                    continue;
                };
                if *speed <= crate::sim::game_options::IN_GAME_OPTIONS_MAX_SPEED {
                    projected = Some(*speed);
                }
            }
        }
        projected
    }

    /// Consume the terminal edge raised by one executed EXIT command.
    pub(crate) fn take_executed_exit_owner(&mut self) -> Option<InternedId> {
        self.executed_exit_owner.take()
    }

    /// Drain commands that are due for the next tick from `pending_commands`.
    /// Returns owned commands; remaining commands stay queued.
    pub fn take_due_commands(&mut self) -> Vec<CommandEnvelope> {
        let execute_tick = self.session.tick.saturating_add(1);
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

    /// Admit and drain one diagnostic-replay batch without consuming pending
    /// work regenerated by the simulation itself.
    ///
    /// The diagnostic log already contains the batch drained by the live app.
    /// Playback can independently recreate later work such as C4 scatter, so
    /// mixing that pending work with the recorded batch would admit it twice.
    /// Temporarily isolating the queue preserves the former direct-replay seam:
    /// the recorded batch still uses [`Self::queue_commands`] and the canonical
    /// due filter, malformed future-stamped replay entries are discarded, and
    /// pre-existing simulation work remains untouched.
    pub(crate) fn take_due_replay_commands(
        &mut self,
        commands: impl IntoIterator<Item = CommandEnvelope>,
    ) -> Vec<CommandEnvelope> {
        let simulation_pending = std::mem::take(&mut self.pending_commands);
        self.queue_commands(commands);
        let due = self.take_due_commands();
        self.pending_commands = simulation_pending;
        due
    }

    fn movement_sound_probe(&self, stable_id: u64) -> Option<MovementSoundProbe> {
        let entity = self.substrate.entities.get(stable_id)?;
        Some(MovementSoundProbe {
            rx: entity.position.rx,
            ry: entity.position.ry,
            z: entity.position.z,
            sub_x_bits: entity.position.sub_x.to_bits(),
            sub_y_bits: entity.position.sub_y.to_bits(),
            facing: entity.facing,
            path_index: entity
                .movement_target
                .as_ref()
                .map(|target| target.next_index),
            track_point: entity.drive_track.as_ref().map(|track| track.point_index),
        })
    }

    pub(crate) fn movement_sound_world(
        entity: &crate::sim::game_entity::GameEntity,
    ) -> crate::sim::anim_class::AnimWorldCoord {
        let locomotor_z = entity
            .locomotor
            .as_ref()
            .map_or(0, |locomotor| locomotor.altitude.to_num::<i32>());
        crate::sim::anim_class::AnimWorldCoord {
            x: i32::from(entity.position.rx)
                .wrapping_mul(256)
                .wrapping_add(entity.position.sub_x.to_num::<i32>()),
            y: i32::from(entity.position.ry)
                .wrapping_mul(256)
                .wrapping_add(entity.position.sub_y.to_num::<i32>()),
            z: i32::from(entity.position.z)
                .wrapping_mul(128)
                .wrapping_add(locomotor_z),
        }
    }

    /// Release an active FootClass MoveSound while the object is still
    /// represented, preserving the native stop-before-UnInit ordering.
    pub(crate) fn release_move_sound(&mut self, stable_id: u64) {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        if !entity.move_sound_active {
            return;
        }
        let world = Self::movement_sound_world(entity);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.move_sound_active = false;
            entity.move_sound_countdown = 0;
        }
        self.sound_events.push(SimSoundEvent::AnimationStopped {
            anim_id: stable_id,
            stop_sound_id: None,
            world,
        });
    }

    /// FootClass's post-locomotor MoveSound tail. A fresh moving-now virtual
    /// answer or locomotor state change keeps the handle alive and reloads the
    /// three-visit grace counter. The process-local audio owner selects the
    /// sample only after its device and spatial-acceptance gates.
    fn tick_move_sound_after_process(
        &mut self,
        stable_id: u64,
        before: Option<MovementSoundProbe>,
        rules: Option<&RuleSet>,
    ) {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        if entity.category == EntityCategory::Structure || entity.locomotor.is_none() {
            return;
        }
        let after = self.movement_sound_probe(stable_id);
        let movement_changed = before.is_some() && before != after;
        let moving_now = crate::sim::movement::ready_producer::is_moving_now_for(
            entity,
            self.session.binary_frame,
        );
        let falling_or_crashing = entity.object_is_falling_down != 0
            || entity.parachute_state.is_some()
            || entity.locomotor.as_ref().is_some_and(|locomotor| {
                locomotor.air_phase == crate::sim::movement::locomotor::AirMovePhase::Descending
                    && locomotor.jumpjet_crash_speed > crate::util::fixed_math::SIM_ZERO
            });
        let active = entity.move_sound_active;
        let countdown = entity.move_sound_countdown;
        let type_ref = entity.type_ref;
        let world = Self::movement_sound_world(entity);
        let qualifies = (movement_changed || moving_now) && !falling_or_crashing;

        if qualifies {
            let mut started = false;
            if !active {
                let configured = rules
                    .and_then(|rules| self.object_type(type_ref, rules))
                    .and_then(|object| object.move_sound.as_deref())
                    .map(str::trim)
                    .filter(|sound| !sound.is_empty() && !sound.eq_ignore_ascii_case("none"))
                    .map(str::to_owned);
                if let Some(configured) = configured {
                    // gamemd `FootClass__AI @ 0x004DA530`: the active MoveSound
                    // tail loads `g_MainRng` at 0x004DAAC0, calls `Random__Next`
                    // at 0x004DAACB, then indexes the vector at 0x004DAAD3.
                    let _sound_index_draw = self.main_rng.next_u32();
                    let sound_id = self.interner.intern(&configured);
                    self.sound_events.push(SimSoundEvent::AnimationStarted {
                        anim_id: stable_id,
                        sound_id,
                        world,
                    });
                    started = true;
                }
            }
            if active || started {
                if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                    entity.move_sound_active = true;
                    entity.move_sound_countdown = 3;
                }
            }
        } else if active {
            if countdown == 0 || falling_or_crashing {
                self.release_move_sound(stable_id);
            } else if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.move_sound_countdown = countdown - 1;
            }
        }
    }

    fn natural_outcome_exit_ready(&self) -> bool {
        let contending_houses = self.contending_house_count();
        self.houses.values().any(|house| {
            house.is_human
                && house.outcome_state.is_some_and(|outcome| {
                    outcome.exit_ready
                        && (outcome.kind == crate::sim::house_state::HouseOutcomeKind::Defeat
                            // VERA-internal opponent precondition; gamemd equivalent
                            // UNCHECKED. See `contending_house_count`.
                            || contending_houses > 1)
                })
        })
    }

    fn termination_frame_requested(&self) -> bool {
        self.quit_requested || self.connection_lost || self.natural_outcome_exit_ready()
    }

    /// Advance the global ambient scalar before ore and active superweapons.
    /// A storm selecting Ion later in this frame can first move it next frame.
    fn tick_scenario_lighting_transition(&mut self, rules: &RuleSet) {
        self.session.lighting.advance_transition_if_due(
            self.session.binary_frame,
            rules.general.ambient_change_rate_nonzero,
            rules.general.ambient_change_interval_frames,
            rules.general.ambient_change_step,
        );
    }

    fn tick_ore_growth_rungs(
        &mut self,
        rules: &RuleSet,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        // Native TiberiumClass drivers run before the main live-object vector,
        // growth first and spread second.
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
            let live_objects = TiberiumPlacementObjectContext::new(
                &self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
            );
            if let (Some(grid), Some(registry)) = (self.overlay_grid.as_mut(), overlay_registry) {
                self.production.ore_growth_state.tick_native_growth_driver(
                    grid,
                    registry,
                    &rules.tiberium_types,
                    self.resolved_terrain.as_ref(),
                    &self.production.tiberium_spawning_terrain_cells,
                    Some(live_objects),
                    &mut self.production.resource_nodes,
                    &mut self.scenario_rng,
                    self.session.binary_frame,
                    self.production.ore_growth_config.grows,
                    self.production.ore_growth_config.spreads,
                    Some(&mut self.radar_terrain_dirty_cells),
                    Some(&mut self.radar_terrain_dirty_generation),
                    Some(&mut self.tactical_dirty_cells),
                );
                self.production.ore_growth_state.tick_native_spread_driver(
                    grid,
                    registry,
                    &rules.tiberium_types,
                    &mut self.production.resource_nodes,
                    path_grid,
                    self.resolved_terrain.as_ref(),
                    &self.production.tiberium_spawning_terrain_cells,
                    Some(live_objects),
                    &mut self.scenario_rng,
                    self.session.binary_frame,
                    self.production.ore_growth_config.grows,
                    self.production.ore_growth_config.spreads,
                    Some(&mut self.radar_terrain_dirty_cells),
                    Some(&mut self.radar_terrain_dirty_generation),
                    Some(&mut self.tactical_dirty_cells),
                );
            }
        } else {
            ore_growth::tick_ore_growth(
                &self.production.ore_growth_config,
                &mut self.production.ore_growth_state,
                &mut self.production.resource_nodes,
                path_grid,
                self.overlay_grid.as_mut(),
                &mut self.scenario_rng,
            );
        }
    }

    /// Advance map triggers by one tick. Uses `std::mem::take` to avoid
    /// self-borrow conflict while actions read and mutate Simulation authority.
    fn advance_triggers(
        &mut self,
        graph: &TriggerGraph,
        triggers: &TriggerMap,
        events: &EventMap,
        actions: &ActionMap,
        rules: Option<&RuleSet>,
    ) -> Vec<TriggerEffect> {
        let mut rt = std::mem::take(&mut self.trigger_runtime);
        let effects = rt.advance_at_frame(
            self.session.binary_frame,
            graph,
            triggers,
            events,
            actions,
            Some(self),
            rules,
        );
        self.trigger_runtime = rt;
        effects
    }

    /// Install the initial normalized MapClass playfield authority.
    ///
    /// `MapClass::Set_Clipped_LocalSize @ 0x00567230` establishes the five
    /// predicate fields. Size height is retained separately because later
    /// action-40 writers normalize another raw LocalSize against the same Size.
    pub(crate) fn install_playfield_from_map_header(
        &mut self,
        header: &crate::map::map_file::MapHeader,
    ) {
        self.playfield_size_height = Some(header.height as i32);
        self.playfield_bounds = Some(
            crate::sim::cell_rect::PlayfieldBounds::from_map_header(header),
        );
        self.playfield_revision = 0;
    }

    /// Apply YR trigger action 0x28's mutable visible-map-area authority.
    ///
    /// `FUN_006E21E0` writes LocalSize, normalizes it, recalculates all cells,
    /// rebuilds zone connectivity/all levels, and refreshes radar before it
    /// returns to `TriggerAction__Execute @ 0x006DD8B0`. Rust performs every
    /// corresponding owned update synchronously here, so later actions and
    /// the same master frame observe one coherent authority.
    pub(crate) fn change_visible_map_area(
        &mut self,
        raw_local_size: [i32; 4],
        rules: Option<&RuleSet>,
    ) -> bool {
        let (Some(current), Some(size_height)) =
            (self.playfield_bounds, self.playfield_size_height)
        else {
            // A live scenario always has MapClass Size authority. Headless
            // fixtures without a map must not invent a rectangular fallback.
            return false;
        };
        let bounds = crate::sim::cell_rect::PlayfieldBounds::from_raw_local_size(
            current.base,
            size_height,
            raw_local_size,
        );
        self.playfield_bounds = Some(bounds);
        // FUN_006E21E0 runs the complete radar-surface rebuild on every
        // execution. Do not deduplicate equal normalized writers.
        self.playfield_revision = self.playfield_revision.wrapping_add(1);

        // `MapClass::Set_Clipped_LocalSize @ 0x00567230` exact-recomputes the
        // canonical TechnoClass+0x3D5 byte for every represented Techno. This
        // must precede later action consumers and may both demote and promote;
        // ordinary per-cell movement's writer is deliberately only 0 -> 1.
        let membership_updates = self
            .substrate
            .entities
            .keys_sorted()
            .into_iter()
            .filter_map(|stable_id| {
                let entity = self.substrate.entities.get(stable_id)?;
                let member = crate::sim::cell_rect::cell_is_in_playfield_height_aware(
                    (i32::from(entity.position.rx), i32::from(entity.position.ry)),
                    Some(bounds),
                    self.resolved_terrain.as_ref(),
                );
                let reveal = !entity.in_playfield
                    && member
                    && entity.lifecycle.object_alive
                    && !entity.lifecycle.in_limbo
                    && !entity.dying
                    && entity.health.current > 0
                    && entity.category != EntityCategory::Structure
                    && self
                        .houses
                        .get(&entity.owner)
                        .is_some_and(|house| house.is_human);
                Some((stable_id, member, reveal))
            })
            .collect::<Vec<_>>();
        for &(stable_id, member, _) in &membership_updates {
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.in_playfield = member;
            }
        }
        // FUN_006E21E0's false->true callback is the Techno reveal/update
        // virtual (`0x0070ADC0`), only for human-owned live nonlimbo mobiles;
        // Buildings are explicitly excluded. Rust's owned equivalent commits
        // the sight reveal immediately, before this action returns.
        let reveal_config = crate::sim::vision::VisionConfig {
            require_playfield_membership: true,
            veteran_sight_bonus: rules.map_or(0, |rules| rules.general.veteran_sight),
            leptons_per_sight_increase: rules
                .map_or(0, |rules| rules.general.leptons_per_sight_increase),
            reveal_by_height: rules.is_none_or(|rules| rules.general.reveal_by_height),
            fog_of_war: self.session.game_options.fog_of_war,
        };
        let height_grid = reveal_config
            .reveal_by_height
            .then(|| {
                self.path_grid
                    .as_ref()
                    .map(|grid| grid.ground_height_grid())
            })
            .flatten();
        for (stable_id, _, reveal) in membership_updates {
            if reveal && let Some(entity) = self.substrate.entities.get(stable_id) {
                crate::sim::vision::reveal_entity_vision(
                    &mut self.fog,
                    entity,
                    &reveal_config,
                    height_grid.as_deref(),
                );
            }
        }

        if let Some(terrain) = self.resolved_terrain.as_mut() {
            terrain.recalc_playfield_attributes(bounds);
        }

        // Rebuild from the retained structure-blocked PathGrid. PathGrid does
        // not cache the outside flag; ZoneGrid does, so a full rebuild covers
        // native connectivity plus every retained hierarchy level without
        // discarding dynamic building blockers.
        if let Some(path_grid) = self.path_grid.clone() {
            self.rebuild_zone_grid_full(path_grid.as_ref());
        }

        // Native calls RefreshRadar after the cell/zone rebuild. The distinct
        // playfield revision is the global geometry invalidation seam; the
        // presentation-acknowledged dirty-cell batch remains cell-local.
        true
    }

    /// Exact mode-one query for the stored TechnoClass+0x3D5 writer family.
    /// Absence means there is no live MapClass authority, not rectangular or
    /// permissive fallback authority.
    fn entity_playfield_membership_mode_one(&self, stable_id: u64) -> Option<bool> {
        let bounds = self.playfield_bounds?;
        let entity = self.substrate.entities.get(stable_id)?;
        Some(crate::sim::cell_rect::cell_is_in_playfield_height_aware(
            (i32::from(entity.position.rx), i32::from(entity.position.ry)),
            Some(bounds),
            self.resolved_terrain.as_ref(),
        ))
    }

    /// Unlimbo's exact establishment writer (`TechnoClass::Unlimbo @
    /// 0x006F6CFE`).
    pub(crate) fn establish_entity_playfield_membership_on_unlimbo(&mut self, stable_id: u64) {
        let Some(member) = self.entity_playfield_membership_mode_one(stable_id) else {
            return;
        };
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.in_playfield = member;
        }
    }

    /// Acknowledge one completed radar-terrain presentation update.
    ///
    /// Active `RadarClass` dirty processing at `0x00655250` drains and clears
    /// its retained cell list after the update. The generation guard prevents
    /// a stale client acknowledgement from clearing a newer producer batch.
    /// Both fields are presentation handoff state (`serde(skip)` and omitted
    /// from `state_hash`), so this consumption cannot mutate lockstep state.
    pub(crate) fn acknowledge_radar_terrain_dirty(&mut self, generation: u64) -> bool {
        if generation != self.radar_terrain_dirty_generation
            || self.radar_terrain_dirty_cells.is_empty()
        {
            return false;
        }
        self.radar_terrain_dirty_cells.clear();
        true
    }

    fn mark_wall_mutations_radar_dirty(
        &mut self,
        mutations: &[crate::sim::overlay_grid::WallMutation],
    ) {
        let Some(grid) = self.overlay_grid.as_ref() else {
            return;
        };
        let dirty = crate::sim::overlay_grid::wall_radar_dirty_cells(
            grid.width(),
            grid.height(),
            mutations,
        );
        self.mark_radar_terrain_dirty_cells(dirty);
    }

    /// Ordinary per-cell movement writer (`0x006F511A..0x006F5139`): only
    /// promote 0 -> 1. A unit that walks back outside retains membership until
    /// an exact writer (teleport or Set_Clipped_LocalSize) clears it.
    fn promote_entity_playfield_membership_after_move(&mut self, stable_id: u64) {
        if self
            .substrate
            .entities
            .get(stable_id)
            .is_none_or(|entity| entity.in_playfield)
        {
            return;
        }
        if self.entity_playfield_membership_mode_one(stable_id) == Some(true)
            && let Some(entity) = self.substrate.entities.get_mut(stable_id)
        {
            entity.in_playfield = true;
        }
    }

    /// Teleport arrival's exceptional exact outside clear (`0x00719A99`). An
    /// inside arrival does not promote a previously-false byte.
    fn clear_entity_playfield_membership_after_teleport(&mut self, stable_id: u64) {
        if self.entity_playfield_membership_mode_one(stable_id) == Some(false)
            && let Some(entity) = self.substrate.entities.get_mut(stable_id)
        {
            entity.in_playfield = false;
        }
    }

    fn poll_triggers_for_master_frame(&mut self, inputs: TriggerInputs<'_>) {
        // YR LogicClass::Update polls scenario triggers before the live-object walk.
        let effects = self.advance_triggers(
            inputs.graph,
            inputs.triggers,
            inputs.events,
            inputs.actions,
            inputs.rules,
        );
        self.trigger_effects.extend(effects);
    }

    /// Drain app-owned outcomes after their authoritative trigger actions ran.
    /// Production consumes trigger effects through `SimFrameOutput`; only
    /// fixture replay and trigger tests drain them directly.
    #[cfg(test)]
    pub(crate) fn drain_trigger_effects(&mut self) -> Vec<TriggerEffect> {
        std::mem::take(&mut self.trigger_effects)
    }

    /// Enable or disable per-entity debug event logging (F10 boundary method:
    /// the app toggles, sim owns the write). Flips the spawn-time flag and
    /// reconciles every existing entity, so existing and future entities
    /// always agree with the toggle. Enabling preserves logs an entity
    /// already accumulated; disabling drops them all.
    pub(crate) fn set_debug_event_logging(&mut self, enabled: bool) {
        self.debug_event_logging = enabled;
        for entity in self.substrate.entities.values_mut() {
            if enabled {
                if entity.debug_log.is_none() {
                    entity.debug_log =
                        Some(crate::sim::debug_event_log::DebugEventLog::new());
                }
            } else {
                entity.debug_log = None;
            }
        }
    }

    /// Pre-merge the named owner's fog visibility so render-side queries hit
    /// the O(1) merged cache (F10 boundary method: the app requests, sim owns
    /// the write). Returns false when the owner name is not interned yet —
    /// no view is built and the per-query slow path stays in effect.
    pub(crate) fn prepare_fog_view_for(&mut self, owner: &str) -> bool {
        let Some(owner_id) = self.interner.get(owner) else {
            return false;
        };
        self.fog.build_merged_for(owner_id, &self.interner);
        true
    }

    /// Configure the command input delay (F10 boundary method; the app pushes
    /// its configured value once at match install).
    pub(crate) fn set_input_delay_ticks(&mut self, ticks: u64) {
        self.input_delay_ticks = ticks;
    }

    /// Install the map trigger runtime state machine after load (F10 boundary
    /// method; the immutable trigger definitions live in `SimResources`).
    pub(crate) fn install_trigger_runtime(
        &mut self,
        runtime: crate::sim::trigger_runtime::TriggerRuntime,
    ) {
        self.trigger_runtime = runtime;
    }

    /// Advance the radar-event review cursor and return the next event cell
    /// (F10 boundary method backing the center-on-radar-event hotkey).
    pub(crate) fn cycle_radar_event(&mut self) -> Option<(u16, u16)> {
        self.radar_events.cycle_event()
    }

    #[cfg(test)]
    pub(crate) fn take_master_frame_test_trace(&mut self) -> Vec<MasterFrameTestRung> {
        std::mem::take(&mut self.master_frame_test_trace)
    }

    #[cfg(test)]
    fn trace_master_frame_rung(&mut self, rung: MasterFrameTestRung) {
        self.master_frame_test_trace.push(rung);
    }

    /// Returns true if the given house name is human-controlled.
    /// Equivalent to the original engine's IsHumanPlayer (0x50b6f0).
    pub fn is_human_player(&self, owner: &str) -> bool {
        self.interner
            .get(owner)
            .and_then(|id| self.houses.get(&id))
            .is_some_and(|h| h.is_human)
    }

    /// Shared identity source for every modeled runtime `AbstractClass` analogue.
    ///
    /// `AbstractClass::AssignUniqueID @ 0x00410230` delegates to
    /// `ScenarioClass::NextUniqueID @ 0x0068BCB0`; individual stores therefore
    /// must not own independent counters.
    pub(crate) fn allocate_stable_id(&mut self) -> u64 {
        let id = self.substrate.next_stable_object_id;
        self.substrate.next_stable_object_id =
            self.substrate.next_stable_object_id.saturating_add(1);
        id
    }

    pub(crate) fn admit_projectile(
        &mut self,
        stable_id: u64,
        spawn: crate::sim::projectile::ProjectileSpawn,
    ) -> u64 {
        self.projectiles.spawn(stable_id, spawn);
        let registered = self.register_projectile(stable_id);
        debug_assert!(registered);
        stable_id
    }

    pub(crate) fn admit_wave(
        &mut self,
        stable_id: u64,
        wave: crate::sim::wave::Wave,
    ) -> u64 {
        self.waves.spawn(stable_id, wave);
        let registered = self.register_wave(stable_id);
        debug_assert!(registered);
        stable_id
    }

    /// Native Reveal's append: +0x98 guard → tail-append → set flag. Idempotent.
    // Reveal/Conceal and raw LogicVector transactions live in lifecycle.rs.

    /// Debug-only invariant: the active order and each stored object's
    /// membership flag are two views of one set and must never disagree. The
    /// order must be duplicate-free, and its length must equal the number of
    /// objects whose `in_logic_vector` is set. O(n); compiled out of release.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_logic_membership_consistent(&self) {
        let order = self.substrate.logic.as_slice();
        let mut seen = std::collections::BTreeSet::new();
        for &id in order {
            debug_assert!(seen.insert(id), "logic order has duplicate id {id}");
            debug_assert!(
                self.substrate
                    .entities
                    .get(id)
                    .is_some_and(|entity| entity.in_logic_vector)
                    || self
                        .substrate
                        .anims
                        .get(id)
                        .is_some_and(|anim| anim.in_logic_vector)
                    || self
                        .substrate
                        .particle_systems
                        .get(id)
                        .is_some_and(|system| system.in_logic_vector)
                    || self
                        .production
                        .terrain_objects
                        .get(&id)
                        .is_some_and(|terrain| terrain.in_logic_vector)
                    || self
                        .projectiles
                        .get(id)
                        .is_some_and(|projectile| projectile.in_logic_vector)
                    || self
                        .waves
                        .get(id)
                        .is_some_and(|wave| wave.in_logic_vector),
                "logic order id {id} is missing or not membership-flagged",
            );
        }
        let flagged_entities = self
            .substrate
            .entities
            .values()
            .filter(|e| e.in_logic_vector)
            .count();
        let flagged_anims = self
            .substrate
            .anims
            .iter()
            .filter(|(_, anim)| anim.in_logic_vector)
            .count();
        let flagged_particle_systems = self
            .substrate
            .particle_systems
            .iter()
            .filter(|(_, system)| system.in_logic_vector)
            .count();
        let flagged_terrain = self
            .production
            .terrain_objects
            .values()
            .filter(|terrain| terrain.in_logic_vector)
            .count();
        let flagged_projectiles = self
            .projectiles
            .iter()
            .filter(|(_, projectile)| projectile.in_logic_vector)
            .count();
        let flagged_waves = self
            .waves
            .iter()
            .filter(|(_, wave)| wave.in_logic_vector)
            .count();
        let flagged = flagged_entities
            + flagged_anims
            + flagged_particle_systems
            + flagged_terrain
            + flagged_projectiles
            + flagged_waves;
        debug_assert_eq!(
            order.len(),
            flagged,
            "logic order length ({}) != objects flagged in_logic_vector ({})",
            order.len(),
            flagged
        );
    }

    /// Debug-only checks for relationships that remain true while the native
    /// lifecycle axes themselves are deliberately independent.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_lifecycle_consistent(&self) {
        for entity in self.substrate.entities.values() {
            if !entity.lifecycle.object_alive {
                debug_assert!(
                    entity.lifecycle.in_limbo,
                    "dead entity {} must have completed Conceal before alive clear",
                    entity.stable_id
                );
                debug_assert!(
                    !entity.lifecycle.cell_marked,
                    "dead entity {} must be unmarked before alive clear",
                    entity.stable_id
                );
                debug_assert!(
                    !entity.in_logic_vector,
                    "dead entity {} must leave LogicVector before alive clear",
                    entity.stable_id
                );
                debug_assert!(
                    entity.owned_count_released,
                    "dead entity {} must release owned count exactly once before alive clear",
                    entity.stable_id
                );
                debug_assert!(
                    self.substrate.pending_delete.contains(&entity.stable_id),
                    "dead entity {} must remain pending until finalization",
                    entity.stable_id
                );
            }
            if entity.lifecycle.cell_marked
                && !entity.passenger_role.is_inside_transport()
                && entity.occupancy_list_layer().is_some()
            {
                for (rx, ry) in crate::sim::occupancy::entity_occupancy_cells(entity) {
                    debug_assert!(
                        self.substrate
                            .occupancy
                            .contains_entity(rx, ry, entity.stable_id),
                        "cell-marked entity {} missing occupancy at ({rx}, {ry})",
                        entity.stable_id
                    );
                }
            }
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
        use crate::sim::production::{PRODUCTION_STEPS, StepOutcome};
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
                self.session.tick, factory.owner, factory.category, steps,
            );
            debug_assert_eq!(
                econ.spent_credits, cost,
                "C15: tick {} {:?}/{:?}: total spent {} must equal full cost {}",
                self.session.tick, factory.owner, factory.category, econ.spent_credits, cost,
            );
            debug_assert_eq!(
                f.balance, 0,
                "C12: tick {} {:?}/{:?}: completion must zero the balance",
                self.session.tick, factory.owner, factory.category,
            );
            debug_assert!(
                f.suspended && f.object.is_some(),
                "C12: tick {} {:?}/{:?}: completion must suspend with the object attached",
                self.session.tick,
                factory.owner,
                factory.category,
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
                    self.session.tick,
                    f.insertion_seq,
                    p,
                );
            }
            prev_seq = Some(f.insertion_seq);
            let mut tail_prev = f.insertion_seq;
            for e in &f.queue {
                debug_assert!(
                    e.enqueue_order > tail_prev,
                    "P5d (A): tick {} {:?}/{:?}: tail enqueue_order must strictly exceed the active build + prior tail ({} after {})",
                    self.session.tick,
                    f.owner,
                    f.category,
                    e.enqueue_order,
                    tail_prev,
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
                self.session.tick,
                f.owner,
                f.category,
                f.progress,
                PRODUCTION_STEPS,
            );
            debug_assert!(
                f.balance >= 0 && f.balance <= f.original_balance,
                "P5b (B): tick {} {:?}/{:?}: balance {} out of [0, original {}]",
                self.session.tick,
                f.owner,
                f.category,
                f.balance,
                f.original_balance,
            );
        }
    }

    /// Test-only: force the active order and sync membership flags to it.
    #[cfg(test)]
    pub(crate) fn set_logic_order_for_test(&mut self, order: Vec<u64>) {
        for entity in self.substrate.entities.values_mut() {
            entity.in_logic_vector = false;
        }
        for anim in self.substrate.anims.values_mut() {
            anim.in_logic_vector = false;
        }
        for (_, system) in self.substrate.particle_systems.iter_mut() {
            system.in_logic_vector = false;
        }
        for terrain in self.production.terrain_objects.values_mut() {
            terrain.in_logic_vector = false;
        }
        for (_, projectile) in self.projectiles.iter_mut() {
            projectile.in_logic_vector = false;
        }
        for (_, wave) in self.waves.iter_mut() {
            wave.in_logic_vector = false;
        }
        for &id in &order {
            if let Some(e) = self.substrate.entities.get_mut(id) {
                e.in_logic_vector = true;
            } else if let Some(anim) = self.substrate.anims.get_mut(id) {
                anim.in_logic_vector = true;
            } else if let Some(system) = self.substrate.particle_systems.get_mut(id) {
                system.in_logic_vector = true;
            } else if let Some(terrain) = self.production.terrain_objects.get_mut(&id) {
                terrain.in_logic_vector = true;
            } else if let Some(projectile) = self.projectiles.get_mut(id) {
                projectile.in_logic_vector = true;
            } else if let Some(wave) = self.waves.get_mut(id) {
                wave.in_logic_vector = true;
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

    /// Change an entity's owner through the authoritative ownership chokepoint.
    /// House counts, the `by_owner` index, and the entity owner move exactly once
    /// for every live transfer, regardless of whether capture or garrison code
    /// requested it.
    pub(crate) fn change_owner(&mut self, stable_id: u64, new_owner: InternedId) {
        self.change_owner_impl(stable_id, new_owner, None);
    }

    pub(crate) fn change_owner_with_rules(
        &mut self,
        stable_id: u64,
        new_owner: InternedId,
        rules: &RuleSet,
    ) {
        self.change_owner_impl(stable_id, new_owner, Some(rules));
    }

    fn change_owner_impl(
        &mut self,
        stable_id: u64,
        new_owner: InternedId,
        rules: Option<&RuleSet>,
    ) {
        let Some((old_owner, category, has_spawn_manager)) =
            self.substrate.entities.get(stable_id).map(|entity| {
                (
                    entity.owner,
                    entity.category,
                    entity.spawn_manager.is_some(),
                )
            })
        else {
            return;
        };
        if old_owner == new_owner {
            return;
        }
        // FootClass::ChangeOwner @ 0x004DBED0 removes from the deposited old
        // owner and adds to the new owner before later readers observe it.
        if let Some(rules) = rules {
            self.transfer_sensor_before_owner_change_with_rules(stable_id, new_owner, rules);
        } else {
            self.transfer_sensor_before_owner_change(stable_id, new_owner);
        }

        // Active YR chain: BuildingClass::ChangeOwner (0x00448260) delegates
        // to TechnoClass::ChangeOwner (0x007014A0), which calls
        // HouseClass::Removed_From_Game (0x005025F0) before the owner swap and
        // HouseClass::Added_To_Game (0x00502A80) afterward. Their building
        // cases move the old/new HouseClass ownership totals.
        let old_owner_name = self.interner.resolve(old_owner).to_string();
        let new_owner_name = self.interner.resolve(new_owner).to_string();

        // `TechnoClass::ChangeOwner` calls `SpawnManagerClass::Kill_All_Spawns`
        // before the house swap: a mind-controlled V3/Dreadnought/Boomer loses
        // the pool it built for its old owner. Run first so the children are
        // destroyed while still attributed to the previous house. The owner is
        // still alive here, so the slots re-arm with a zero regen wait and the
        // new owner's pool is rebuilt on the next manager pass.
        if has_spawn_manager {
            crate::sim::spawn_manager::kill_all_spawns(self, stable_id);
        }
        self.decrement_owned_count(&old_owner_name, category);
        // `TechnoClass::ChangeOwner` runs the live-detach targeting sweep next,
        // before the house swap: everything shooting at this object is released
        // while the object still belongs to its old house. Engineer capture and
        // garrison transfer both come through here, so a squad that was firing
        // at a building stops the instant the building changes hands instead of
        // shooting at what is now its own structure.
        self.stop_all_targeting_on_detach(stable_id);
        self.substrate.entities.change_owner(stable_id, new_owner);
        self.increment_owned_count(&new_owner_name, category);
        self.refresh_waypoint_edge_from_committed_structure(stable_id);
    }

    /// Legacy non-lifecycle contact scrub retained only for separately classified
    /// failure paths. Reveal/Conceal/UnInit must use synchronous radio authority.
    pub(crate) fn clear_radio_contacts_for(&mut self, stable_id: u64) {
        self.substrate.entities.clear_radio_contacts_for(stable_id);
    }

    /// Check each house for defeat and game completion
    /// (all remaining houses mutually allied).
    fn check_defeat(&mut self, rules: Option<&RuleSet>) {
        let outcome_tick = self.session.tick.saturating_add(1);
        let savour_frames = crate::rules::ruleset::savour_delay_frames(
            rules
                .map(|rules| rules.general.savour_delay_minutes)
                // RulesClass__Constructor @ 0x00665650 stores the exact f64
                // default 0.03 before any optional INI ReadDouble override.
                .unwrap_or(0.03),
        );
        // Short Game defeats houses with no buildings unless a BaseUnit remains.
        // Long games wait for all owned objects.
        let owners: Vec<InternedId> = self.houses.keys().copied().collect();
        for &owner in &owners {
            let house = &self.houses[&owner];
            // gamemd gates its entire defeat block on the house type's
            // MultiplayPassive being clear, so Civilian/JP houses are never
            // evaluated for defeat no matter what they own or lose.
            if house.is_defeated || house.multiplay_passive {
                continue;
            }
            let should_defeat = if self.session.game_options.short_game {
                house.owned_building_count == 0 && !self.house_has_live_base_unit(owner, rules)
            } else {
                house.owned_building_count == 0 && house.owned_unit_count == 0
            };
            if should_defeat {
                let accepted = if let Some(h) = self.houses.get_mut(&owner) {
                    h.is_defeated = true;
                    // A house that owns nothing (or, in Short Game, has no base
                    // left) has lost from its own perspective. Flag_To_Lose owns
                    // the result transition and grace timer. NOTE: gamemd does
                    // NOT destroy the
                    // defeated house's remaining objects — it scatters surviving
                    // units (ScatterAllUnits) and they persist; hard object
                    // removal only happens under the non-standard SpecialFlags
                    // 0x800 (HarvesterImmune). So no cleanup/destroy is done here.
                    h.flag_to_lose(outcome_tick, savour_frames)
                } else {
                    false
                };
                if accepted {
                    self.sound_events.push(SimSoundEvent::MatchOutcome {
                        owner,
                        kind: crate::sim::house_state::HouseOutcomeKind::Defeat,
                    });
                }
            }
        }

        // Check if all remaining alive houses are mutually allied → game over.
        // The native alive scan counts only houses that are neither defeated nor
        // passive; the Civilian/JP houses present in every skirmish own map
        // objects forever, so including them would keep the alive set above one
        // and the victory screen would never appear.
        let alive: Vec<InternedId> = self
            .houses
            .iter()
            .filter(|(_, h)| !h.is_defeated && !h.multiplay_passive)
            .map(|(k, _)| *k)
            .collect();

        if alive.len() == 1 {
            // Last player standing.
            if let Some(h) = self.houses.get_mut(&alive[0]) {
                if h.flag_to_win(outcome_tick, savour_frames) {
                    self.sound_events.push(SimSoundEvent::MatchOutcome {
                        owner: alive[0],
                        kind: crate::sim::house_state::HouseOutcomeKind::Victory,
                    });
                }
            }
        } else if !alive.is_empty() {
            // O(n^2) mutual-alliance check. Native alliance is directional — each
            // house owns its own ally bits — and the game-over scan requires BOTH
            // houses of a pair to name the other, so a one-way alliance must not end
            // the match.
            let all_allied = alive.iter().all(|a| {
                alive.iter().all(|b| {
                    a == b
                        || crate::map::houses::are_houses_mutually_allied(
                            &self.house_alliances,
                            self.interner.resolve(*a),
                            self.interner.resolve(*b),
                        )
                })
            });

            if all_allied {
                for &owner in &alive {
                    if let Some(h) = self.houses.get_mut(&owner) {
                        if h.flag_to_win(outcome_tick, savour_frames) {
                            self.sound_events.push(SimSoundEvent::MatchOutcome {
                                owner,
                                kind: crate::sim::house_state::HouseOutcomeKind::Victory,
                            });
                        }
                    }
                }
            }
        }

        // HouseClass::Update @ 0x004F8440 advances the accepted result timer
        // in the house rung. The expiry frame is terminal and therefore skips
        // the wrapping frame commit below, matching Main_Tick's early return.
        for house in self.houses.values_mut() {
            house.advance_outcome_savour(outcome_tick);
        }
    }

    /// Number of houses that can actually contend for the match outcome.
    ///
    /// MultiplayPassive houses (stock Civilian/JP) are roster filler: they are
    /// never defeated and never counted alive, so they must not make a
    /// single-player board look contested. Callers use `> 1` to mean "a real
    /// opponent exists" before announcing a victory that would otherwise be
    /// true from tick 0.
    ///
    /// VERA-internal: the gamemd equivalent is UNCHECKED. Neither the native
    /// defeat block nor its all-allied scan has an "is there a real opponent"
    /// precondition — this exists only to keep zero-opponent sandbox and dev
    /// maps, which the retail game cannot launch, from declaring instant
    /// victory. The passive filter it counts with IS gamemd-derived.
    pub(crate) fn contending_house_count(&self) -> usize {
        self.houses
            .values()
            .filter(|house| !house.multiplay_passive)
            .count()
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

    /// Restore externally-derived cache fields after validated snapshot
    /// identity fixup and substrate re-registration.
    ///
    /// The caller must provide the same map/rules data that was used to initialize
    /// the original simulation. Cache fields were `#[serde(skip)]`'d and are at
    /// their Default values after deserialization. The caller must first run
    /// `restore_after_snapshot_load`, which resolves stable-ID references and
    /// rebuilds registry, LogicVector-membership, and CellClass-list caches.
    ///
    /// Overlay, bridge, and navigation authority are restored separately by
    /// `restore_map_authority_after_snapshot_load` once rules and the overlay
    /// registry are bound.
    pub fn rebuild_caches_after_load(
        &mut self,
        mut resolved_terrain: ResolvedTerrainGrid,
        terrain_speed_config: terrain_speed::TerrainSpeedConfig,
        bridge_explosions: Vec<InternedId>,
        metallic_debris: Vec<InternedId>,
        bridge_anim_sounds: BTreeMap<InternedId, InternedId>,
    ) {
        resolved_terrain.bind_shared_cell_dummy(self.shared_cell_dummy.clone());
        // Restore externally-derived data only. Substrate caches are rebuilt
        // transactionally by `restore_after_snapshot_load` before this call.
        // The supplied map grid predates runtime Terrain lifecycle changes, so
        // serialized Terrain objects must overwrite that derived projection.
        // Clear every known source first so Limbo/Destroyed objects stay absent,
        // then replay only live objects through the shared occupation writer.
        let terrain_objects: Vec<_> = self.production.terrain_objects.values().cloned().collect();
        for terrain in &terrain_objects {
            crate::sim::terrain_object::unmark_terrain_occupation(
                &mut self.production,
                terrain,
                Some(&mut resolved_terrain),
            );
        }
        for terrain in &terrain_objects {
            if terrain.is_live() {
                crate::sim::terrain_object::mark_terrain_occupation(
                    &mut self.production,
                    terrain,
                    Some(&mut resolved_terrain),
                );
            }
        }
        let terrain_costs = build_canonical_terrain_cost_grids(&resolved_terrain);

        self.resolved_terrain = Some(resolved_terrain);
        self.terrain_speed_config = terrain_speed_config;
        self.bridge_explosions = bridge_explosions;
        self.metallic_debris = metallic_debris;
        self.bridge_anim_sounds = bridge_anim_sounds;
        self.terrain_costs = terrain_costs;
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
        for anim in self.substrate.anims.values_mut() {
            anim.in_logic_vector = false;
        }
        for (_, system) in self.substrate.particle_systems.iter_mut() {
            system.in_logic_vector = false;
        }
        for terrain in self.production.terrain_objects.values_mut() {
            terrain.in_logic_vector = false;
        }
        for (_, projectile) in self.projectiles.iter_mut() {
            projectile.in_logic_vector = false;
        }
        for (_, wave) in self.waves.iter_mut() {
            wave.in_logic_vector = false;
        }
        for &id in &self.substrate.logic.snapshot() {
            if let Some(entity) = self.substrate.entities.get_mut(id) {
                entity.in_logic_vector = true;
            } else if let Some(anim) = self.substrate.anims.get_mut(id) {
                anim.in_logic_vector = true;
            } else if let Some(system) = self.substrate.particle_systems.get_mut(id) {
                system.in_logic_vector = true;
            } else if let Some(terrain) = self.production.terrain_objects.get_mut(&id) {
                terrain.in_logic_vector = true;
            } else if let Some(projectile) = self.projectiles.get_mut(id) {
                projectile.in_logic_vector = true;
            } else if let Some(wave) = self.waves.get_mut(id) {
                wave.in_logic_vector = true;
            }
        }
        // Alive, limbo, cell Mark, and death-sequence state are independent
        // serialized facts. Load repair must never derive them from this vector.
    }

    /// Borrow the current canonical dynamic navigation projection.
    pub fn path_grid(&self) -> Option<&PathGrid> {
        self.path_grid.as_deref()
    }

    /// Pin the current navigation projection across a mutable simulation frame.
    pub fn path_grid_snapshot(&self) -> Option<Arc<PathGrid>> {
        self.path_grid.clone()
    }

    /// Rebuild terrain costs, dynamic structure blockers, zones, and the
    /// canonical PathGrid as one simulation-owned projection.
    pub fn rebuild_dynamic_navigation(&mut self, rules: &RuleSet) -> bool {
        let Some(terrain) = self.resolved_terrain.as_ref() else {
            return false;
        };
        let mut grid =
            PathGrid::from_resolved_terrain_with_bridges(terrain, self.bridge_state.as_ref());
        self.terrain_costs = build_canonical_terrain_cost_grids(terrain);

        let mut structures: Vec<(u16, u16, String)> = self
            .substrate
            .entities
            .values()
            .filter_map(|entity| {
                (entity.category == EntityCategory::Structure).then_some((
                    entity.position.rx,
                    entity.position.ry,
                    self.interner.resolve(entity.type_ref).to_string(),
                ))
            })
            .collect();
        structures.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        for (rx, ry, type_id) in structures {
            let object_type = rules.object(&type_id);
            let foundation = object_type
                .map(|object| object.foundation.as_str())
                .unwrap_or("1x1");
            let has_bib = object_type.is_some_and(|object| object.bib);
            grid.block_building_movement_cells(rx, ry, foundation, has_bib);
        }

        self.rebuild_zone_grid(&grid);
        true
    }

    /// Finalize mutable overlay identity, passability, and canonical navigation
    /// before the frame hash is latched. Only occupied cells cross the app
    /// boundary; cleared cells remain render-inert through OverlayGrid authority.
    fn finalize_frame_overlays_and_navigation(
        &mut self,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        mut navigation_rebuild_requested: bool,
    ) -> Vec<OverlayEntry> {
        let mut overlay_updates = Vec::new();
        let overlay_ready =
            rules.is_some() && self.resolved_terrain.is_some() && overlay_registry.is_some();
        if overlay_ready && let Some(grid) = self.overlay_grid.as_mut() {
            let (dirty_cells, synchronous_passability_changed) =
                grid.take_dirty_cells_with_passability_signal();
            let synchronous_navigation_cells = grid.take_synchronous_navigation_cells();
            navigation_rebuild_requested |= synchronous_passability_changed
                || !synchronous_navigation_cells.is_empty();

            let terrain = self.resolved_terrain.as_mut().expect("overlay-ready terrain");
            let registry = overlay_registry.expect("overlay-ready registry");
            for &(rx, ry) in &dirty_cells {
                navigation_rebuild_requested |=
                    recalc_overlay_passability(grid, terrain, registry, rx, ry);
            }
            for (rx, ry) in dirty_cells {
                let cell = grid.cell(rx, ry);
                if let Some(overlay_id) = cell.overlay_id {
                    overlay_updates.push(OverlayEntry {
                        rx,
                        ry,
                        overlay_id,
                        frame: cell.overlay_data,
                    });
                }
            }
        }

        if navigation_rebuild_requested && let Some(rules) = rules {
            let _ = self.rebuild_dynamic_navigation(rules);
        }
        overlay_updates
    }

    /// Rebuild the zone connectivity map from the current PathGrid and terrain costs.
    /// Call after the PathGrid has been rebuilt so that zones reflect the latest
    /// walkability state.
    ///
    /// Tries an incremental update first (diffing against the previous PathGrid).
    /// Falls back to full rebuild if too many cells changed or no previous state.
    pub fn rebuild_zone_grid(&mut self, path_grid: &PathGrid) {
        if self.resolved_terrain.is_none() {
            return;
        }

        // Try incremental update if we have previous state.
        if let (Some(prev), Some(zones)) = (self.path_grid.as_deref(), &mut self.zone_grid) {
            if let Some(changed) = prev.diff_cells(path_grid) {
                if changed.is_empty()
                    && self
                        .resolved_terrain
                        .as_ref()
                        .is_some_and(|terrain| zones.movement_classes_match(terrain))
                {
                    // PathGrid does not carry CellClass reduced zone type.
                    // Boolean path state and retained base classes must both
                    // match before connectivity can be reused.
                    self.path_grid = Some(Arc::new(path_grid.clone()));
                    return;
                }
                if !changed.is_empty()
                    && crate::sim::pathfinding::zone_incremental::try_incremental_update(
                        zones,
                        &changed,
                        path_grid,
                        &self.terrain_costs,
                        self.resolved_terrain.as_ref(),
                        self.bridge_state
                            .as_ref()
                            .map(|bs| bs.endpoint_records())
                            .unwrap_or(&[]),
                    )
                {
                    log::trace!("zone: incremental update ({} cells changed)", changed.len(),);
                    self.path_grid = Some(Arc::new(path_grid.clone()));
                    return;
                }
            }
        }

        // Full rebuild fallback.
        self.rebuild_zone_grid_full(path_grid);
    }

    /// Rebuild without the PathGrid-only incremental shortcut. Reduced zone
    /// type can change while boolean walkability stays identical (notably a
    /// live OccupationBits=0 terrain object changing Building to Ground).
    fn rebuild_zone_grid_full(&mut self, path_grid: &PathGrid) {
        let Some(terrain) = &self.resolved_terrain else {
            return;
        };
        let width = terrain.width();
        let height = terrain.height();
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
        self.path_grid = Some(Arc::new(path_grid.clone()));
    }

    /// Refresh navigation authority after inline overlay mutation or terrain
    /// object destruction. The incoming grid carries dynamic structure and
    /// wall blockers, so only synchronously changed cells are replaced from the
    /// current resolved-terrain/bridge projection.
    fn refresh_navigation_after_terrain_changes(
        &mut self,
        input_path_grid: Option<&PathGrid>,
        changed_cells: &[(u16, u16)],
    ) -> Option<PathGrid> {
        let (resolved_path_grid, terrain_costs) = {
            let terrain = self.resolved_terrain.as_ref()?;
            (
                PathGrid::from_resolved_terrain_with_bridges(terrain, self.bridge_state.as_ref()),
                build_canonical_terrain_cost_grids(terrain),
            )
        };

        self.terrain_costs = terrain_costs;
        let mut tail_path_grid = input_path_grid
            .filter(|grid| {
                grid.width() == resolved_path_grid.width()
                    && grid.height() == resolved_path_grid.height()
            })
            .cloned()?;
        for &(rx, ry) in changed_cells {
            let replaced = tail_path_grid.replace_cell_from(&resolved_path_grid, rx, ry);
            debug_assert!(replaced, "changed terrain cell must be inside the map");
        }
        self.rebuild_zone_grid_full(&tail_path_grid);
        Some(tail_path_grid)
    }

    /// Publish wall-placement passability before a later command executes and
    /// before the frame hash is latched.
    pub(crate) fn finalize_wall_placement_navigation(
        &mut self,
        input_path_grid: Option<&PathGrid>,
    ) {
        let input_path_grid = input_path_grid
            .cloned()
            .or_else(|| self.path_grid.as_deref().cloned());
        let Some(input_path_grid) = input_path_grid else {
            return;
        };
        let Some(terrain) = self.resolved_terrain.as_ref() else {
            return;
        };
        let resolved_path_grid =
            PathGrid::from_resolved_terrain_with_bridges(terrain, self.bridge_state.as_ref());
        if input_path_grid.width() != resolved_path_grid.width()
            || input_path_grid.height() != resolved_path_grid.height()
        {
            return;
        }
        let changed_cells = self
            .overlay_grid
            .as_mut()
            .map(|grid| grid.take_synchronous_navigation_cells())
            .unwrap_or_default();
        if changed_cells.is_empty() {
            return;
        }
        self.terrain_costs = build_canonical_terrain_cost_grids(terrain);
        let mut tail_path_grid = input_path_grid;
        for (rx, ry) in changed_cells {
            let replaced = tail_path_grid.replace_cell_from(&resolved_path_grid, rx, ry);
            debug_assert!(replaced, "changed wall cell must be inside the map");
        }
        self.rebuild_zone_grid(&tail_path_grid);
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
    /// for any cells the damage destroys.
    ///
    /// `overlay_registry` supplies per-overlay-type Strength/DamageLevels.
    pub(crate) fn apply_wall_damage_events(
        &mut self,
        events: &[WallDamageEvent],
        overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
    ) {
        if events.is_empty() {
            return;
        }
        let Some(grid) = self.overlay_grid.as_mut() else {
            return;
        };
        let mut wall_mutations = Vec::new();

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

            if let Some(terrain) = self.resolved_terrain.as_mut() {
                for mutation in &result.mutations {
                    let changed = recalc_overlay_passability(
                        grid,
                        terrain,
                        overlay_registry,
                        mutation.rx,
                        mutation.ry,
                    );
                    grid.record_synchronous_passability_change_at(
                        mutation.rx,
                        mutation.ry,
                        changed,
                    );
                }
            }
            wall_mutations.extend(result.mutations);
        }
        self.mark_wall_mutations_radar_dirty(&wall_mutations);
    }

    /// Movement-side wall crush: a `Crusher=yes` drive vehicle that finishes the
    /// ground-movement stage standing on a `Wall=yes` overlay cell flattens that
    /// wall outright. This mirrors gamemd's per-cell-process wall crush — a
    /// forced, instant overlay removal — which is a SEPARATE path from the
    /// probabilistic weapon/warhead wall damage (the crush deals no unit damage
    /// and skips the Strength dice roll).
    ///
    /// The gate is exactly the `Crusher=` flag (not `OmniCrusher=`, which governs
    /// unit-vs-unit crushing) plus a Drive locomotor over a wall cell. In stock
    /// YR only the Battle Fortress routes *through* walls (`MovementZone=
    /// CrusherAll`), but any Crusher drive vehicle that ends up on a wall cell
    /// crushes it. A crusher can only occupy an intact wall cell on the tick it
    /// enters (walls block non-crushers), and the wall is removed that same tick,
    /// so this self-limits to one destruction per wall with no per-tick re-fire.
    ///
    /// Runs immediately after Phase-1 ground movement, before vision. Reuses the
    /// shared `apply_wall_damage_events` path so overlay clear, cardinal
    /// neighbor connectivity cleanup, and chain reaction match weapon damage.
    ///
    /// Parity follow-up: gamemd also plays a Voc cue and adds a small forward
    /// rocking tilt on the crush; the exact Voc index is unresolved
    /// (`docs/research/WALL_CRUSH_ON_DRIVEOVER_GHIDRA_REPORT.md` §5), so the sound
    /// and cosmetic tilt are deferred rather than approximated with a wrong cue.
    pub(crate) fn apply_wall_crush_on_driveover(
        &mut self,
        rules: Option<&RuleSet>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        let (Some(_rules), Some(registry)) = (rules, overlay_registry) else {
            return;
        };
        let Some(grid) = self.overlay_grid.as_ref() else {
            return;
        };

        // Phase 1 (read-only): collect every distinct wall cell currently
        // occupied by an active Crusher drive vehicle. Sorted entity iteration
        // keeps this deterministic; the per-cell dedup means two crushers on one
        // cell emit a single forced-destruction event.
        let mut events: Vec<WallDamageEvent> = Vec::new();
        let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
        for (_id, e) in self.substrate.entities.iter_sorted() {
            if !e.regular_crusher || !e.is_active() {
                continue;
            }
            // Wall crush requires the Drive locomotor (gamemd LocomotorType ==
            // Drive); check the primary kind so a transient piggyback (e.g. the
            // chrono-miner's temporary Drive) does not change the gate.
            if e.locomotor.as_ref().map(|l| l.effective_kind()) != Some(LocomotorKind::Drive) {
                continue;
            }
            let (rx, ry) = (e.position.rx, e.position.ry);
            let has_wall = grid
                .cell(rx, ry)
                .overlay_id
                .and_then(|oid| registry.flags(oid))
                .is_some_and(|f| f.wall);
            if has_wall && seen.insert((rx, ry)) {
                // damage == -1 = forced instant removal, bypassing the
                // probabilistic Strength gate the weapon path uses.
                events.push(WallDamageEvent { rx, ry, damage: -1 });
            }
        }

        if events.is_empty() {
            return;
        }

        // Phase 2 (mutating): shared teardown, identical to the combat wall path.
        self.apply_wall_damage_events(&events, registry);
    }

    pub(crate) fn default_vision_range_for_category(category: EntityCategory) -> u16 {
        match category {
            EntityCategory::Infantry => 5,
            EntityCategory::Unit => 6,
            EntityCategory::Aircraft => 8,
            EntityCategory::Structure => 7,
        }
    }

    /// Find houses with at least one native-eligible SpySat provider. This is
    /// the House-rung edge detector; consumers read the persisted house latch.
    fn collect_spy_sat_candidate_owners(&self, rules: &RuleSet) -> BTreeSet<InternedId> {
        let selling =
            crate::sim::mission::MissionId::from_known(crate::sim::mission::MissionType::Selling);
        let mut active = BTreeSet::new();
        let mut scanned_houses = BTreeSet::new();
        for &owner in self.session.house_order.iter().chain(self.houses.keys()) {
            if !self.houses.contains_key(&owner) || !scanned_houses.insert(owner) {
                continue;
            }
            for &stable_id in self.substrate.entities.ids_for_owner(owner) {
                let Some(entity) = self.substrate.entities.get(stable_id) else {
                    continue;
                };
                let coarse_candidate = entity.category == EntityCategory::Structure
                    && !entity.lifecycle.in_limbo
                    && entity.lifecycle.cell_marked
                    && entity.mission.current() != selling
                    && entity.mission.queued() != selling
                    && self
                        .object_type(entity.type_ref, rules)
                        .is_some_and(|object| object.spy_sat);
                if !coarse_candidate {
                    continue;
                }
                // The first coarse candidate decides the house. Warp-out is a
                // blocking result, not a reason to continue to a later uplink.
                if !entity
                    .teleport_state
                    .as_ref()
                    .is_some_and(|state| state.warp_out_active())
                {
                    active.insert(owner);
                }
                break;
            }
        }
        active
    }

    /// Materialize vision effects from their final authorities: the persisted
    /// per-house SpySat latch and freshly qualified powered Gap generators.
    fn collect_active_vision_structures(&self, rules: &RuleSet) -> ActiveVisionStructures {
        let mut effects = ActiveVisionStructures {
            spy_sat_owners: self
                .houses
                .iter()
                .filter_map(|(&owner, house)| house.spy_sat_active.then_some(owner))
                .collect(),
            gap_generators: Vec::new(),
        };
        for entity in self.substrate.entities.values() {
            if entity.dying || entity.category != EntityCategory::Structure {
                continue;
            }
            let Some(obj) = self.object_type(entity.type_ref, rules) else {
                continue;
            };
            if entity.building_up.is_some() {
                continue;
            }
            if obj.gap_generator
                && power_system::is_building_powered(
                    &self.power_states,
                    rules,
                    entity,
                    &self.interner,
                )
            {
                effects.gap_generators.push((
                    entity.owner,
                    entity.position.rx,
                    entity.position.ry,
                    i32::from(obj.gap_radius_in_cells),
                ));
            }
        }
        effects
    }

    fn apply_active_vision_structures(&mut self, effects: &ActiveVisionStructures) {
        // Phase 3 already clears these transient bits while rebuilding sight,
        // but the later House rung must replace that earlier result after any
        // combat/lifecycle changes before it reapplies the final effect set.
        self.fog.clear_gap_flags();
        if !effects.spy_sat_owners.is_empty() {
            if let Some(terrain) = self.resolved_terrain.as_ref() {
                for &owner in &effects.spy_sat_owners {
                    self.fog.reveal_cells_for_owner(
                        owner,
                        terrain.iter().map(|cell| (cell.rx, cell.ry)),
                    );
                }
            } else {
                // Synthetic fixtures without production terrain retain the
                // rectangular fallback owned by the visibility subsystem.
                vision::apply_spy_sat(&mut self.fog, &effects.spy_sat_owners, &self.interner);
            }
        }
        if !effects.gap_generators.is_empty() {
            vision::apply_gap_generators(&mut self.fog, &effects.gap_generators, &self.interner);
        }
    }

    /// Commit aggregate SpySat transitions at the House update rung, then
    /// materialize the House-rung SpySat -> Gap result. EventClass commands
    /// execute later and therefore become visible to this scan next frame.
    fn reconcile_active_vision_structures(&mut self, rules: &RuleSet) {
        let active_owners = self.collect_spy_sat_candidate_owners(rules);
        let mut owners_losing_last_uplink = Vec::new();
        for (&owner, house) in &mut self.houses {
            let active = active_owners.contains(&owner);
            if house.spy_sat_active && !active {
                owners_losing_last_uplink.push(owner);
            }
            if house.spy_sat_active != active {
                house.map_is_clear = active;
            }
            house.spy_sat_active = active;
        }
        for owner in owners_losing_last_uplink {
            self.fog.restore_shroud_after_spy_sat_loss(owner);
        }
        let effects = self.collect_active_vision_structures(rules);
        self.apply_active_vision_structures(&effects);
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
            let effects = self.collect_active_vision_structures(rules);
            self.apply_active_vision_structures(&effects);
        }

        // Diagnostic: log fog grid stats on first tick to debug coverage issues.
        if self.session.tick == 1 {
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

    /// Advance build-up animations and return completed building stable IDs.
    fn tick_building_up(&mut self) -> Vec<u64> {
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
        for &sid in &finished {
            if let Some(entity) = self.substrate.entities.get_mut(sid) {
                entity.building_up = None;
            }
        }
        finished
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
            let rules = match rules {
                Some(rules) => {
                    self.uninit_with_rules(sid, rules);
                    rules
                }
                None => {
                    self.uninit(sid);
                    continue;
                }
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

    fn apply_one_due_command(
        &mut self,
        cmd: &CommandEnvelope,
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> (bool, bool, bool, Option<InternedId>) {
        let cmd_owner_str = self.interner.resolve(cmd.owner).to_string();
        let applied = self.apply_command_with_overlays(
            &cmd_owner_str,
            &cmd.payload,
            rules,
            path_grid,
            height_map,
            overlay_registry,
        );
        let placed_building_owner =
            self.successful_non_wall_placement_owner(cmd, applied, rules);
        let spawned_entity = placed_building_owner.is_some()
            || applied
                && matches!(
                    cmd.payload,
                    Command::DeployMcv { .. }
                        | Command::UndeployBuilding { .. }
                        | Command::LaunchSuperWeapon { .. }
                );
        let destroyed_structure = applied
            && matches!(
                cmd.payload,
                Command::SellBuilding { .. } | Command::UndeployBuilding { .. }
            );
        (
            applied,
            spawned_entity,
            destroyed_structure,
            placed_building_owner,
        )
    }

    fn successful_non_wall_placement_owner(
        &self,
        command: &CommandEnvelope,
        applied: bool,
        rules: Option<&RuleSet>,
    ) -> Option<InternedId> {
        let Command::PlaceReadyBuilding { type_id, .. } = command.payload else {
            return None;
        };
        (applied
            && rules
                .and_then(|rules| rules.object(self.interner.resolve(type_id)))
                .is_some_and(|object| !object.wall))
        .then_some(command.owner)
    }

    fn is_wall_placement_command(&self, command: &Command, rules: Option<&RuleSet>) -> bool {
        let Command::PlaceReadyBuilding { type_id, .. } = command else {
            return false;
        };
        rules
            .and_then(|rules| rules.object(self.interner.resolve(*type_id)))
            .is_some_and(|object| object.wall)
    }

    pub(crate) fn command_uses_megamission(command: &Command) -> bool {
        matches!(
            command,
            Command::Move { .. }
                | Command::Attack { .. }
                | Command::ForceAttack { .. }
                | Command::AttackMove { .. }
                | Command::Guard { .. }
                | Command::MinerReturn { .. }
                | Command::RepairAtDepot { .. }
                | Command::EnterTransport { .. }
                | Command::EnterBunker { .. }
                | Command::EjectBunker { .. }
                | Command::UnloadPassengers { .. }
                | Command::HarvestCell { .. }
                | Command::CaptureBuilding { .. }
                | Command::PlantC4 { .. }
                | Command::ToggleInfantryDeploy { .. }
                | Command::ForceAttackCell { .. }
        )
    }

    fn formation_key(command: &Command) -> Option<(u8, u16, u16)> {
        match command {
            Command::Move {
                target_rx,
                target_ry,
                ..
            } => Some((0, *target_rx, *target_ry)),
            Command::AttackMove {
                target_rx,
                target_ry,
                ..
            } => Some((1, *target_rx, *target_ry)),
            _ => None,
        }
    }

    fn formation_entity_id(command: &Command) -> Option<u64> {
        match command {
            Command::Move { entity_id, .. } | Command::AttackMove { entity_id, .. } => {
                Some(*entity_id)
            }
            _ => None,
        }
    }

    fn group_destination_candidate_facts(
        &self,
        grid: &PathGrid,
        zone_grid: &ZoneGrid,
        clicked_target: (i16, i16),
        member: &group_destination::GroupDestinationMember,
        candidate: (i16, i16),
    ) -> group_destination::CandidateFacts {
        let signed_candidate = (i32::from(candidate.0), i32::from(candidate.1));
        if !crate::sim::cell_rect::cell_is_in_playfield_height_aware(
            signed_candidate,
            self.playfield_bounds,
            self.resolved_terrain.as_ref(),
        ) {
            return group_destination::CandidateFacts::outside_playfield();
        }
        let (Ok(candidate_x), Ok(candidate_y)) =
            (u16::try_from(candidate.0), u16::try_from(candidate.1))
        else {
            return group_destination::CandidateFacts::outside_playfield();
        };
        let target = (clicked_target.0 as u16, clicked_target.1 as u16);
        let Some(entity) = self.substrate.entities.get(member.entity_id) else {
            return group_destination::CandidateFacts {
                in_playfield: true,
                same_zone: false,
                height_band_ok: false,
                can_enter_code: 7,
            };
        };
        let Some(target_cell) = grid.cell(target.0, target.1) else {
            return group_destination::CandidateFacts {
                in_playfield: true,
                same_zone: false,
                height_band_ok: false,
                can_enter_code: 7,
            };
        };
        let Some(candidate_cell) = grid.cell(candidate_x, candidate_y) else {
            return group_destination::CandidateFacts {
                in_playfield: true,
                same_zone: false,
                height_band_ok: false,
                can_enter_code: 7,
            };
        };

        let movement_zone = entity
            .locomotor
            .as_ref()
            .map(|locomotor| locomotor.movement_zone)
            .unwrap_or_default();
        let speed_type = entity
            .locomotor
            .as_ref()
            .map(|locomotor| locomotor.speed_type)
            .unwrap_or_default();
        let locomotor_kind = entity
            .locomotor
            .as_ref()
            .map_or(LocomotorKind::Drive, |locomotor| locomotor.effective_kind());
        let target_layer = if target_cell.has_structural_bridge() {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        let candidate_layer = if candidate_cell.has_structural_bridge() {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        let same_zone = zone_grid.map_for(movement_zone).is_some_and(|zones| {
            zones.zone_at(target.0, target.1, target_layer)
                == zones.zone_at(candidate_x, candidate_y, candidate_layer)
        });

        let target_height = target_cell.signed_level()
            + if target_cell.has_structural_bridge() {
                4
            } else {
                0
            };
        let candidate_height = candidate_cell.signed_level()
            + if candidate_cell.has_structural_bridge() {
                4
            } else {
                0
            };
        let height_band_ok = (candidate_height - target_height).abs() < 3;

        // UnitClass::Can_Enter_Cell(candidate, -1, target_height, 0, 1)
        // selects the bridge list when the candidate carries a structural deck
        // separated from the requested path height.
        let object_layer = if candidate_cell.has_structural_bridge()
            && (target_height - candidate_cell.signed_level()).abs() >= 2
        {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        };
        let layers = crate::sim::pathfinding::can_enter_layer_context(
            object_layer,
            object_layer,
            candidate_cell,
            target_height,
        );
        let cost_grid = self.terrain_costs.get(&speed_type);
        let terrain_passable = match layers.terrain_layer {
            MovementLayer::Ground => {
                crate::sim::pathfinding::is_cell_passable_for_mover_with_speed(
                    grid,
                    candidate_x,
                    candidate_y,
                    Some(movement_zone),
                    Some(speed_type),
                    self.resolved_terrain.as_ref(),
                    cost_grid,
                    false,
                    crate::sim::pathfinding::cell_entry::TerrainEntryMode::Smoothing,
                )
            }
            MovementLayer::Bridge => {
                grid.is_walkable_on_layer(candidate_x, candidate_y, MovementLayer::Bridge)
            }
            MovementLayer::Air | MovementLayer::Underground => true,
        };
        let can_enter_code = if !terrain_passable {
            7
        } else {
            match crate::sim::pathfinding::cell_entry::check_terrain_with_layers(
                (candidate_x, candidate_y),
                layers,
                entity.category,
                Some(grid),
                cost_grid,
                &self.substrate.occupancy,
            ) {
                crate::sim::pathfinding::cell_entry::TerrainCheckResult::Clear => 0,
                crate::sim::pathfinding::cell_entry::TerrainCheckResult::Impassable => 7,
                crate::sim::pathfinding::cell_entry::TerrainCheckResult::NeedsBlockerCheck => {
                    crate::sim::pathfinding::cell_entry::classify_occupied_cell_with_layers(
                        (candidate_x, candidate_y),
                        layers,
                        member.entity_id,
                        movement::bump_crush::CrushCapability::new(
                            entity.regular_crusher,
                            entity.omni_crusher,
                        ),
                        self.interner.resolve(entity.owner),
                        locomotor_kind,
                        false,
                        &self.substrate.occupancy,
                        &self.substrate.entities,
                        &self.house_alliances,
                        &self.interner,
                    )
                    .yr_code()
                }
            }
        };

        group_destination::CandidateFacts {
            in_playfield: true,
            same_zone,
            height_band_ok,
            can_enter_code,
        }
    }

    /// Adjust consecutive same-target movement runs after their house's
    /// non-megamission scan, immediately before staged command execution.
    fn adjust_staged_megamission_destinations(
        &self,
        commands: &mut [CommandEnvelope],
        path_grid: Option<&PathGrid>,
    ) {
        let (Some(grid), Some(zone_grid)) = (path_grid, self.zone_grid.as_ref()) else {
            return;
        };
        let mut run_start = 0;
        while run_start < commands.len() {
            let Some(key) = Self::formation_key(&commands[run_start].payload) else {
                run_start += 1;
                continue;
            };
            let mut run_end = run_start + 1;
            while run_end < commands.len()
                && Self::formation_key(&commands[run_end].payload) == Some(key)
            {
                run_end += 1;
            }
            if run_end - run_start > 1 {
                let mut members = Vec::new();
                for (command_index, command) in commands[run_start..run_end].iter().enumerate() {
                    let Some(entity_id) = Self::formation_entity_id(&command.payload) else {
                        continue;
                    };
                    let Some(entity) = self.substrate.entities.get(entity_id) else {
                        continue;
                    };
                    if !matches!(
                        entity.category,
                        EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
                    ) || entity.low_bridge_tube_state.is_some()
                    {
                        continue;
                    }
                    let Some(coord_z) = self
                        .resolved_terrain
                        .as_ref()
                        .and_then(|terrain| combat::in_range::effective_z_leptons(entity, terrain))
                    else {
                        continue;
                    };
                    members.push(group_destination::GroupDestinationMember {
                        command_index,
                        entity_id,
                        coord: [
                            i32::from(entity.position.rx)
                                .wrapping_mul(256)
                                .wrapping_add(entity.position.sub_x.to_num::<i32>()),
                            i32::from(entity.position.ry)
                                .wrapping_mul(256)
                                .wrapping_add(entity.position.sub_y.to_num::<i32>()),
                            coord_z as i32,
                        ],
                        source_cell: (entity.position.rx as i16, entity.position.ry as i16),
                    });
                }
                let assignments = group_destination::distribute_group_destinations(
                    (key.1 as i16, key.2 as i16),
                    &members,
                    |member, candidate| {
                        self.group_destination_candidate_facts(
                            grid,
                            zone_grid,
                            (key.1 as i16, key.2 as i16),
                            member,
                            candidate,
                        )
                    },
                );
                for assignment in assignments {
                    let Some(command) = commands.get_mut(run_start + assignment.command_index)
                    else {
                        continue;
                    };
                    match &mut command.payload {
                        Command::Move {
                            target_rx,
                            target_ry,
                            ..
                        }
                        | Command::AttackMove {
                            target_rx,
                            target_ry,
                            ..
                        } => {
                            *target_rx = assignment.destination.0 as u16;
                            *target_ry = assignment.destination.1 as u16;
                        }
                        _ => {}
                    }
                }
            }
            run_start = run_end;
        }
    }

    /// Canonical receiver order for every due command in one master frame.
    /// Registered HouseClass order wins; standalone-fixture owners are appended
    /// in their first issue order, matching the established tail dispatcher.
    fn due_command_house_order(
        &self,
        commands: &[CommandEnvelope],
        execute_tick: u64,
    ) -> Vec<InternedId> {
        let mut house_order = self.session.house_order.clone();
        for command in commands
            .iter()
            .filter(|command| command.execute_tick <= execute_tick)
        {
            if !house_order.contains(&command.owner) {
                house_order.push(command.owner);
            }
        }
        house_order
    }

    fn command_uses_frame_ingress(command: &Command) -> bool {
        matches!(command, Command::SetGameSpeed { .. })
    }

    /// Apply offline session transitions before triggers and the live-object
    /// walk. Native offline Options stores GameSpeed before the next Main_Tick;
    /// VERA transports the transition as a replayable command, then admits it
    /// at this dedicated ingress instead of the ordinary EventClass tail.
    fn apply_due_frame_ingress_commands(
        &mut self,
        commands: &[CommandEnvelope],
        execute_tick: u64,
    ) -> usize {
        let mut executed_commands = 0usize;
        for owner in self.due_command_house_order(commands, execute_tick) {
            for command in commands.iter().filter(|command| {
                command.execute_tick <= execute_tick
                    && command.owner == owner
                    && Self::command_uses_frame_ingress(&command.payload)
            }) {
                let Command::SetGameSpeed { speed } = &command.payload else {
                    unreachable!("frame-ingress predicate admitted a non-session command");
                };
                if self.houses.contains_key(&owner) {
                    let _ = self.session.game_options.apply_in_game_speed(*speed);
                }
                // Preserve the established dispatcher convention: every due
                // envelope is consumed/counts even when validation rejects it.
                executed_commands += 1;
            }
        }
        executed_commands
    }

    /// Advance one deterministic simulation tick.
    /// Apply all due commands in HouseClass registration
    /// order. Each house preserves insertion order within the normal and
    /// staged-megamission streams. Returns
    /// `(executed_commands, spawned_entities, destroyed_structure,
    /// successful_non_wall_placement_owners)`.
    fn apply_due_commands(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
        execute_tick: u64,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) -> (usize, bool, bool, Vec<InternedId>) {
        let mut executed_commands = 0usize;
        let mut spawned_entities = false;
        let mut destroyed_structure = false;
        let mut placed_building_owners = Vec::new();
        let mut tail_path_grid = path_grid
            .cloned()
            .or_else(|| self.path_grid.as_deref().cloned());

        for owner in self.due_command_house_order(commands, execute_tick) {
            for command in commands.iter().filter(|command| {
                command.execute_tick <= execute_tick
                    && command.owner == owner
                    && !Self::command_uses_frame_ingress(&command.payload)
                    && !Self::command_uses_megamission(&command.payload)
            }) {
                let (applied, spawned, destroyed, placed_owner) = self.apply_one_due_command(
                    command,
                    rules,
                    tail_path_grid.as_ref(),
                    height_map,
                    overlay_registry,
                );
                if matches!(command.payload, Command::SellWallAtCell { .. }) {
                    tail_path_grid = self.path_grid.as_deref().cloned();
                } else if applied && self.is_wall_placement_command(&command.payload, rules) {
                    tail_path_grid = self.path_grid.as_deref().cloned().or(tail_path_grid);
                }
                spawned_entities |= spawned;
                destroyed_structure |= destroyed;
                placed_building_owners.extend(placed_owner);
                executed_commands += 1;
            }

            let mut staged = commands
                .iter()
                .filter(|command| {
                    command.execute_tick <= execute_tick
                        && command.owner == owner
                        && Self::command_uses_megamission(&command.payload)
                })
                .cloned()
                .collect::<Vec<_>>();
            self.adjust_staged_megamission_destinations(&mut staged, tail_path_grid.as_ref());
            for command in &staged {
                let (_, spawned, destroyed, placed_owner) = self.apply_one_due_command(
                    command,
                    rules,
                    tail_path_grid.as_ref(),
                    height_map,
                    overlay_registry,
                );
                spawned_entities |= spawned;
                destroyed_structure |= destroyed;
                placed_building_owners.extend(placed_owner);
                executed_commands += 1;
            }
        }

        (
            executed_commands,
            spawned_entities,
            destroyed_structure,
            placed_building_owners,
        )
    }

    /// Spine region (LATE): AI commands, defeat detection, building animations,
    /// radar/world-effect aging, and the late frame/tick commit. Accumulates
    /// `spawned_entities` (AI placements + undeploy spawns). Returns false when
    /// the terminating call skips frame commit and pending-delete processing.
    fn run_late_region(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        path_grid: Option<&PathGrid>,
        height_map: &BTreeMap<(u16, u16), u8>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
        execute_tick: u64,
        executed_commands: &mut usize,
        spawned_entities: &mut bool,
        destroyed_structure: &mut bool,
        placed_building_owners: &mut Vec<InternedId>,
    ) -> bool {
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::Houses);
        if let Some(rules) = rules {
            self.reconcile_active_vision_structures(rules);
        }
        // --- Phase 8: Defeat detection (runs BEFORE AI) ---
        // gamemd evaluates each house's defeat before its AI manage/produce step,
        // so a house that lost its last building/unit this tick can issue NO AI
        // command this tick. Owned counts are final here after combat + production
        // (but before this tick's AI spawns); tick_ai then skips any house already
        // flagged defeated via its is_defeated gate.
        if self.session.tick > 0 {
            self.check_defeat(rules);
        }

        // gamemd.exe TeamClass::AI advances scenario teams after house defeat
        // admission and before the generic HouseClass AI tail.
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::TeamScript);
        let mut team_script_vm = std::mem::take(&mut self.team_script_vm);
        let team_tick = team_script_vm.tick_effects(self.session.binary_frame as i32, |owner| {
            !crate::sim::house_state::house_state_for_owner_id(&self.houses, owner)
                .is_some_and(|house| house.is_defeated)
        });
        self.team_script_vm = team_script_vm;
        for effect in team_tick.effects {
            match effect {
                // Original: TeamClass::AI action 19 walks TeamClass+0x54 and
                // invokes FootClass's panic-family virtual in member order.
                TeamScriptEffect::PanicMember { entity_id } => {
                    let Some(rules) = rules else { continue };
                    let Some(type_ref) = self
                        .substrate
                        .entities
                        .get(entity_id)
                        .map(|entity| entity.type_ref)
                    else {
                        continue;
                    };
                    let Some(object_type) = rules.object(self.interner.resolve(type_ref)) else {
                        continue;
                    };
                    if let Some(entity) = self.substrate.entities.get_mut(entity_id) {
                        crate::sim::infantry::apply_panic_force(object_type, entity);
                    }
                }
            }
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
                overlay_registry,
            );
            self.ai_players = ai_state;
            let mut ai_tail_path_grid = path_grid
                .cloned()
                .or_else(|| self.path_grid.as_deref().cloned());
            for cmd in &ai_commands {
                let cmd_owner_str = self.interner.resolve(cmd.owner).to_string();
                let applied = self.apply_command_with_overlays(
                    &cmd_owner_str,
                    &cmd.payload,
                    rules,
                    ai_tail_path_grid.as_ref(),
                    height_map,
                    overlay_registry,
                );
                if applied && self.is_wall_placement_command(&cmd.payload, rules) {
                    ai_tail_path_grid = self.path_grid.as_deref().cloned().or(ai_tail_path_grid);
                }
                let placed_owner = self.successful_non_wall_placement_owner(cmd, applied, rules);
                placed_building_owners.extend(placed_owner);
                if placed_owner.is_some()
                    || applied
                        && matches!(
                            cmd.payload,
                            Command::DeployMcv { .. }
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
        let completed_buildings = self.tick_building_up();
        if let Some(rules) = rules {
            for &stable_id in &completed_buildings {
                self.add_building_sensor_array_if_powered(stable_id, rules);
            }
            *spawned_entities |= production::spawn_completed_refinery_free_units(
                self,
                &completed_buildings,
                rules,
                path_grid,
                height_map,
            );
        }
        // Advance building-down (undeploy) animations; spawn units when done.
        *spawned_entities |= self.tick_building_down(rules);

        // Tick radar event aging (remove expired pings).
        self.radar_events.tick();

        // Tick world-effect animations and remove finished ones.
        let mut started_effect_sounds = Vec::new();
        self.world_effects.retain_mut(|fx| {
            let tick = fx.tick_with_start_sound();
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

        // EventClass dispatch is a Main_Tick tail rung: the complete live
        // Logic walk observes frame N's pre-command state, so an accepted
        // command first changes that object's AI behavior on frame N+1.
        let (executed, spawned, destroyed, placed_owners) = self.apply_due_commands(
            commands,
            rules,
            path_grid,
            height_map,
            execute_tick,
            overlay_registry,
        );
        *executed_commands += executed;
        *spawned_entities |= spawned;
        *destroyed_structure |= destroyed;
        placed_building_owners.extend(placed_owners);

        // Main_Tick returns immediately on a terminal result. The wrapping
        // frame commit, pacing tail, and pending-delete drain are all skipped
        // on that same terminating call.
        if self.termination_frame_requested() {
            return false;
        }

        // Commit the wrapping native frame LATE, after every phase has observed
        // frame N. Stored-start timer consumers therefore capture N, and the
        // next admitted advance begins on N+1. Host-provided milliseconds are
        // retained for diagnostics only and never determine the frame.
        self.session.total_sim_ms = self.session.total_sim_ms.saturating_add(tick_ms as u64);
        self.session.binary_frame = self.session.binary_frame.wrapping_add(1);
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::FrameCommit);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::BinaryFrameCommitted);

        // The one ordinary ProcessPendingDelete drain follows the native frame
        // commit. Alive queue entries keep their position; ready duplicate IDs
        // collapse and physically finalize exactly once.
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::PendingDelete);
        self.process_pending_delete();

        // Debug-mode safety net: rebuild occupancy after the drain so dead
        // structures are not reconstructed into the comparison.
        #[cfg(debug_assertions)]
        if std::env::var("OCCUPANCY_DEBUG").is_ok() {
            let expected = OccupancyGrid::rebuild(&self.substrate.entities);
            self.substrate.occupancy.debug_assert_matches(&expected);
        }

        // The separate Rust session tick has no direct native field; preserve
        // its existing post-debug-validation relation.
        self.session.tick = execute_tick;
        true
    }

    /// Fixture-only frame adapter (F09): unit tests drive one Main_Tick-shaped
    /// frame with explicitly supplied rules/heights/navigation. Production and
    /// tooling advance exclusively through `SimRuntime::advance_frame`, whose
    /// resources are bound at construction and cannot be substituted per call.
    #[cfg(test)]
    pub(crate) fn advance_tick(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
    ) -> TickResult {
        self.advance_master_frame(
            commands,
            rules,
            height_map,
            path_grid,
            overlay_registry,
            tick_ms,
            TickLane::Ordinary,
            None,
        )
    }

    /// App-facing authoritative frame transaction.
    ///
    /// The app submits commands and immutable map/rules inputs, then consumes
    /// the returned facts instead of reaching back into Simulation-owned
    /// transient queues. `SimRuntime::advance_frame` is the sole production
    /// caller (F09); headless and replay execution go through the runtime.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_app_frame(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
        lane: TickLane,
        trigger_inputs: Option<TriggerInputs<'_>>,
    ) -> SimFrameOutput {
        let path_grid = self.path_grid_snapshot();
        let tick = self.advance_master_frame(
            commands,
            rules,
            height_map,
            path_grid.as_deref(),
            overlay_registry,
            tick_ms,
            lane,
            trigger_inputs,
        );
        self.collect_frame_output(tick)
    }

    fn collect_frame_output(&mut self, tick: TickResult) -> SimFrameOutput {
        let trigger_effects = std::mem::take(&mut self.trigger_effects);
        // Preserve the established terminal-frame gate: these are committed
        // light-vector facts and the next admitted frame clears the producer
        // buffer before combat runs.
        let invulnerability_impacts = if tick.frame_committed {
            std::mem::take(&mut self.invulnerability_impact_effects)
        } else {
            Vec::new()
        };
        let lifecycle_outputs = std::mem::take(&mut self.lifecycle_outputs);
        let overlay_updates = std::mem::take(&mut self.frame_overlay_updates);
        let fire_events = std::mem::take(&mut self.fire_events);
        let sound_events = std::mem::take(&mut self.sound_events);
        SimFrameOutput {
            tick,
            trigger_effects,
            lifecycle_outputs,
            overlay_updates,
            sound_events,
            fire_events,
            invulnerability_impacts,
        }
    }

    /// Advance exactly one authoritative simulation frame.
    ///
    /// This is the sole Main_Tick-shaped entry point, reached in production
    /// only through `advance_app_frame` (F09). The fixture-only `advance_tick`
    /// adapter and the replay fixture runners are `#[cfg(test)]`; gameplay
    /// supplies its map definitions through `trigger_inputs`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_master_frame(
        &mut self,
        commands: &[CommandEnvelope],
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
        lane: TickLane,
        trigger_inputs: Option<TriggerInputs<'_>>,
    ) -> TickResult {
        self.invulnerability_impact_effects.clear();
        self.pending_projectile_detonations.clear();
        self.pending_wave_damage_requests.clear();
        let animation_sequences = rules.map(RuleSet::animation_sequences);
        // The wrapping native frame counter is committed LATE (end of this fn,
        // beside self.session.tick) so every phase sees the same pre-increment
        // frame N. execute_tick stays here: command scheduling below filters on
        // the separate monotonic ordinal.
        let execute_tick = self.session.tick.saturating_add(1);
        let mut executed_commands = 0usize;
        let mut spawned_entities = false;
        let mut destroyed_structure = false;
        let mut placed_building_owners = Vec::new();
        let mut tail_path_grid: Option<PathGrid> = None;
        // No command-boundary drain: command-applied deaths (sell, MCV/slave
        // deploy-undeploy, engineer capture) now stay in the Dying window like
        // combat deaths, freed only by the single end-of-tick drain — matching
        // gamemd's one ProcessPendingDelete at the tail of Main_Tick. The mid-
        // tick raw-store consumers (vision, power, production, movement, miner,
        // aircraft, …) are dying-gated, so a corpse is excluded until that drain.
        let mut bridge_state_changed = false;
        let mut passenger_ownership_changed = false;

        if lane == TickLane::Ordinary {
            executed_commands += self.apply_due_frame_ingress_commands(commands, execute_tick);
        }
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::SessionCommands);

        // YR LogicClass::Update establishes trigger state before visiting the
        // live LogicVector, so object work in this frame observes its actions.
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::Triggers);
        if let Some(inputs) = trigger_inputs {
            self.poll_triggers_for_master_frame(inputs);
        }

        // Object-AI stage: the authoritative per-object Mission host, run
        // immediately BEFORE Phase-1 ground movement — gamemd decides each
        // object's mission, then moves it, within one pass. Each live object
        // gets its `+0xC4` AI-counter tick, its owner-local queued-mission
        // promotion (Ready→Commence), and its absorbed mission-handler
        // dispatch (Harvest: the miner FSM, timer-gated with the post-handler
        // epilogue write) here. Player commands from frame N-1 are already
        // represented; frame N's EventClass tail is dispatched below.
        // Movement stays before the Phase-3 vision recompute.
        //
        // Dock-reservation corpse sweep first (was the global miner tick's
        // pre-pass): reservations held by/on dying objects release before the
        // Harvest dispatches run.
        if let Some(rules) = rules {
            self.tick_scenario_lighting_transition(rules);
            self.tick_ore_growth_rungs(rules, path_grid, overlay_registry);
            if self.session.game_options.super_weapons {
                crate::sim::superweapon::tick_active_superweapon_effects(
                    self,
                    rules,
                    overlay_registry,
                );
            }
            self.radiation.tick_decay(
                self.session.binary_frame,
                &rules.radiation,
                self.resolved_terrain.as_ref(),
            );
        }
        if rules.is_some() {
            crate::sim::miner::sweep_dead_dock_reservations(self);
        }
        let miner_config = rules.map(crate::sim::miner::MinerConfig::from_rules);
        let terrain_spawner_cells = self
            .production
            .terrain_spawners
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let object_ctx = techno_ai::ObjectAiCtx {
            path_grid,
            overlay_registry,
            terrain_spawner_cells: Some(&terrain_spawner_cells),
            miner_config: miner_config.as_ref(),
        };

        // --- Phase 1: Ground movement ---
        // DEPENDS ON: commands (may set movement_target), entity positions from prior tick.
        // PRODUCES: updated entity positions, crush/bump effects, drive track state.
        #[cfg(test)]
        self.trace_master_frame_rung(MasterFrameTestRung::LogicVector);
        let mut movement_stats = movement::MovementTickStats::default();
        let mut tube_turn_owned_ids = BTreeSet::new();
        self.for_each_live_object(|sim, stable_id| {
            // UnitClass::AI / InfantryClass::AI give an active TubeMovement
            // object the whole live-object turn.  Capture before the leaf:
            // successful finalization clears the payload but must still skip
            // every ordinary locomotor tail and the second mission checkpoint.
            let tube_active_at_entry =
                sim.substrate.entities.get(stable_id).is_some_and(|entity| {
                    !entity.dying
                        && matches!(
                            entity.category,
                            EntityCategory::Unit | EntityCategory::Infantry
                        )
                        && entity.low_bridge_tube_state.is_some()
                });
            let was_structure = sim
                .substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.category == EntityCategory::Structure);
            sim.object_ai_visit_one(stable_id, rules, object_ctx);
            if was_structure
                && sim
                    .substrate
                    .entities
                    .get(stable_id)
                    .is_none_or(|entity| entity.dying)
            {
                destroyed_structure = true;
            }
            if sim
                .substrate
                .entities
                .get(stable_id)
                .is_none_or(|entity| entity.dying)
            {
                return;
            }

            let before_movement = sim.movement_sound_probe(stable_id);
            let cell_before_movement = sim
                .substrate
                .entities
                .get(stable_id)
                .map(|entity| (entity.position.rx, entity.position.ry));
            let one = [stable_id];
            movement_stats.merge(movement::tick_movement_object_with_grids(
                &mut sim.substrate.entities,
                stable_id,
                path_grid,
                &sim.terrain_costs,
                &sim.house_alliances,
                &mut sim.substrate.occupancy,
                &mut sim.substrate.cell_occupation,
                &mut sim.substrate.raw_cell_occupation,
                &mut sim.substrate.next_occupancy_enter_order,
                &mut sim.scenario_rng,
                sim.session.tick,
                sim.session.binary_frame,
                sim.zone_grid.as_ref(),
                sim.resolved_terrain.as_ref(),
                sim.overlay_grid.as_ref(),
                overlay_registry,
                sim.playfield_bounds,
                &sim.terrain_speed_config,
                sim.close_enough,
                sim.path_delay_ticks,
                sim.blockage_path_delay_ticks,
                &mut sim.interner,
                rules,
                &mut sim.sound_events,
                &mut sim.pending_lifecycle_requests,
            ));

            // FootClass advances the SHP Unit body counter immediately after
            // this object's locomotor Process, against the still-current
            // absolute binary frame. The global frame commits only after the
            // complete live-object pass.
            if shp_vehicle_counter_admitted(tube_active_at_entry) {
                let shp_vehicle_cadence =
                    sim.substrate.entities.get(stable_id).and_then(|entity| {
                        if entity.category != EntityCategory::Unit || entity.is_voxel {
                            return None;
                        }
                        let object = rules?.object(sim.interner.resolve(entity.type_ref))?;
                        Some(crate::sim::animation::ShpVehicleCadence {
                            walk_rate: object.walk_rate,
                            idle_rate: object.idle_rate,
                        })
                    });
                if let (Some(cadence), Some(entity)) = (
                    shp_vehicle_cadence,
                    sim.substrate.entities.get_mut(stable_id),
                ) {
                    crate::sim::animation::tick_shp_vehicle_body_frame_counter(
                        entity,
                        cadence,
                        sim.session.binary_frame,
                    );
                }
            }

            // A direction-8 producer also ends this object's ordinary turn as
            // soon as it arms TubeMovement.  The leaf itself starts on the
            // object's next visit; an entry-active leaf may have cleared the
            // payload above, hence the captured half of this predicate.
            let tube_owns_whole_turn = tube_active_at_entry
                || sim.substrate.entities.get(stable_id).is_some_and(|entity| {
                    matches!(
                        entity.category,
                        EntityCategory::Unit | EntityCategory::Infantry
                    ) && entity.low_bridge_tube_state.is_some()
                });
            if tube_owns_whole_turn {
                tube_turn_owned_ids.insert(stable_id);
                return;
            }

            sim.tick_air_movement_with_cell_lists_one(stable_id);
            let teleport_relocating = sim
                .substrate
                .entities
                .get(stable_id)
                .and_then(|entity| entity.teleport_state.as_ref())
                .is_some_and(|state| {
                    state.phase == crate::sim::movement::teleport_movement::TeleportPhase::Relocate
                });
            if let Some(rules) = rules {
                let warp_out_type = sim.interner.intern(&rules.general.warp_out.name);
                let warp_out_total_frames = rules
                    .effect_frame_count(&rules.general.warp_out.name)
                    .unwrap_or(teleport_movement::FALLBACK_WARP_FRAME_COUNT);
                let mut teleport_visuals = teleport_movement::TeleportVisuals {
                    world_effects: &mut sim.world_effects,
                    warp_out_type,
                    warp_out_total_frames,
                    warp_out_frame_delay: rules.general.warp_out.frame_delay,
                };
                teleport_movement::tick_teleport_movement(
                    &mut sim.substrate.entities,
                    &mut sim.substrate.occupancy,
                    &one,
                    sim.session.tick,
                    Some(&mut teleport_visuals),
                );
            } else {
                teleport_movement::tick_teleport_movement(
                    &mut sim.substrate.entities,
                    &mut sim.substrate.occupancy,
                    &one,
                    sim.session.tick,
                    None,
                );
            }
            sim.pending_rocket_detonations
                .extend(rocket_movement::tick_rocket_movement(
                    &mut sim.substrate.entities,
                    &one,
                    sim.session.tick,
                ));
            sim.tick_tunnel_locomotor_one(stable_id, path_grid);
            sim.tick_drop_pod_locomotor_one(stable_id, path_grid);
            let _ = homing_movement::tick_homing_movement(
                &mut sim.substrate.entities,
                &one,
                sim.session.tick,
            );
            if let Some(rules) = rules {
                parachute_descent::tick_parachute_descent_in_order(
                    &mut sim.substrate.entities,
                    &one,
                    rules.general.parachute_max_fall_rate,
                    sim.session.tick,
                );
            }
            movement::tick_locomotor_piggyback_restore_one(&mut sim.substrate.entities, stable_id);

            let cell_after_movement = sim
                .substrate
                .entities
                .get(stable_id)
                .map(|entity| (entity.position.rx, entity.position.ry));
            if let Some(rules) = rules {
                sim.move_unit_sensor_after_cell_change(
                    stable_id,
                    cell_before_movement,
                    cell_after_movement,
                    rules,
                );
            }
            if teleport_relocating {
                // `TeleportLocomotionClass` arrival owns the exceptional exact
                // outside clear at 0x00719A99; it must not flow through the
                // ordinary promote-only per-cell writer.
                sim.clear_entity_playfield_membership_after_teleport(stable_id);
            } else if cell_before_movement != cell_after_movement {
                sim.promote_entity_playfield_membership_after_move(stable_id);
            }

            let mut lifecycle_requests = std::mem::take(&mut sim.pending_lifecycle_requests);
            for request in lifecycle_requests.drain(..) {
                let LifecycleRequest::Uninit { stable_id, .. } = request;
                sim.release_move_sound(stable_id);
                if let Some(rules) = rules {
                    sim.apply_lifecycle_request_with_rules(request, rules);
                } else {
                    sim.apply_lifecycle_request(request);
                }
            }
            debug_assert!(lifecycle_requests.is_empty());
            sim.pending_lifecycle_requests = lifecycle_requests;

            sim.tick_move_sound_after_process(stable_id, before_movement, rules);
            sim.object_ai_post_movement_promote_one(stable_id, rules);
        });
        if let Some(rules) = rules {
            self.for_each_multiplayer_feedback_anim(|sim, id| sim.visit_anim(id, rules));
        }
        // Spawn-manager missiles that reached their target during the movement
        // pass are consumed here — the missile leaves the world at the moment
        // `RocketLocomotion::Process` would have called Detonate. The impact
        // itself is queued for the combat phase below, which runs it through
        // the same damage → death → despawn pipeline as any other detonation.
        if rules.is_some() {
            if !self.pending_rocket_detonations.is_empty() {
                let detonated = std::mem::take(&mut self.pending_rocket_detonations);
                crate::sim::spawn_manager::detonate_missiles(self, &detonated);
            }
        } else {
            // No RuleSet means no spawner could have launched anything; drop
            // both queues rather than letting them accumulate across ticks that
            // never reach the combat phase.
            self.pending_rocket_detonations.clear();
            self.pending_missile_detonations.clear();
        }
        movement::sync_formation_speeds_after_live_pass(&mut self.substrate.entities);
        if let Some(rules) = rules {
            crate::sim::gate_runtime::tick_gate_runtimes(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
                self.session.binary_frame,
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
        // Movement-side wall crush (part of the ground-movement stage): a Crusher
        // drive vehicle that ended Phase-1 on a wall cell flattens the wall,
        // separate from the weapon-damage wall path. No-op when no crusher sits
        // on a wall, so it is hash-neutral for every non-crush scenario.
        self.apply_wall_crush_on_driveover(rules, overlay_registry);
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

        // Spawn wake effects behind moving ships on water (every 8 native frames).
        if self.session.binary_frame & 7 == 0 {
            if let Some(rules) = rules {
                let wake_name_str = &rules.general.wake.name;
                let wake_rate = rules.general.wake.frame_delay;
                let wake_frames = rules.effect_frame_count(wake_name_str).unwrap_or(8);
                // Collect positions to avoid borrow conflict (read entities, write world_effects).
                let wake_positions: Vec<(u16, u16, u8)> = self
                    .substrate
                    .entities
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
                if !wake_positions.is_empty() {
                    let wake_id = self.interner.intern(wake_name_str);
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
                            frame_delay: wake_rate,
                            elapsed_frames: 0,
                            translucent: true,
                            delay_frames: 0,
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
            require_playfield_membership: self.playfield_bounds.is_some(),
            veteran_sight_bonus: rules.map_or(0, |r| r.general.veteran_sight),
            leptons_per_sight_increase: rules.map_or(0, |r| r.general.leptons_per_sight_increase),
            // Height-based LOS: terrain 4+ levels above the viewer at the
            // obstruction cell blocks sight (a unit at a cliff base can't see over
            // the cliff). Parity review verified the obstruction sampling against
            // the original (mirror table + the +2 offset); default on, as in YR.
            reveal_by_height: rules.map_or(true, |r| r.general.reveal_by_height),
            fog_of_war: self.session.game_options.fog_of_war,
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
                &self.interner,
            );
            // --- Phase 4.5: Superweapons ---
            // DEPENDS ON: power state (suspend/resume gating).
            // PRODUCES: world_effects (bolt anims), damage to entities, sound_events.
            if self.session.game_options.super_weapons {
                crate::sim::superweapon::tick_superweapon_instances(self, rules);
            }

            // --- Phase 4.6: Deploy/Undeploy state machine ---
            // DEPENDS ON: the prior frame's command tail
            //   (ToggleInfantryDeploy may have set Deploying/Undeploying).
            // PRODUCES: phase advances (Deploying→Deployed, Undeploying→None)
            //   that combat (Phase 5) and animation (post-tick) read this tick.
            crate::sim::deploy::tick_deploy_state(&mut self.substrate.entities);

            // Infantry fear decay and runtime prone transitions happen after
            // deploy state and before combat consumes the prone bit.
            crate::sim::infantry::tick_fear_for_entities(
                &mut self.substrate.entities,
                &self.houses,
                rules,
                &self.interner,
            );

            // Idle fidgets, immediately after the stance pass so a man who just
            // stood back up is not eligible on the same tick he was prone.
            // Driven from the logic vector, not the entity store: limboed
            // objects never reach this in the original.
            // DEPENDS ON: prone bit, deploy phase, attack target, mission.
            // PRODUCES: Idle1/Idle2 sequence switches, idle facing changes, and
            //   scenario-RNG draws — the one idle path that moves the cursor.
            crate::sim::infantry::tick_idle_actions(
                &mut self.substrate.entities,
                self.substrate.logic.as_slice(),
                &self.houses,
                rules,
                &self.interner,
                &mut self.scenario_rng,
                self.session.binary_frame,
            );

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
            // PRODUCES: damage, deaths, bridge damage, fire events. Ordered
            // ReceiveDamage retaliation is committed inline; only legacy
            // precomputed damage producers can still write last_attacker_id.
            // tick_bridge_repair_orders runs BEFORE tick_capture_orders so
            // engineers targeting BridgeRepairHut buildings are consumed by
            // repair, not by capture. tick_capture_orders has an explicit
            // BridgeRepairHut skip as defense in depth.
            let bridge_repaired = self.tick_bridge_repair_orders_with_overlay_registry(
                rules,
                overlay_registry,
                &tube_turn_owned_ids,
            );
            spawned_entities |= self.tick_capture_orders(rules, &tube_turn_owned_ids);
            let c4_outcome = self.tick_c4_plants_with_overlay_registry(
                rules,
                overlay_registry,
                &tube_turn_owned_ids,
            );
            destroyed_structure |= c4_outcome.destroyed_structure;
            bridge_state_changed |= bridge_repaired | c4_outcome.bridge_state_changed;
            self.tick_order_intents_pre_combat(rules, &tube_turn_owned_ids);
            // Pursuit: walk units with out-of-range attack_target into range,
            // halt movement on range entry. Must run before combat so combat
            // sees the up-to-date movement_target this tick.
            self.tick_attack_pursuit_with_overlay_registry(
                rules,
                path_grid,
                overlay_registry,
                &tube_turn_owned_ids,
            );
            // LogicClass live-object order drives the firing/damage/kill-credit
            // resolution sequence. Snapshot is owned, so it does not conflict
            // with the &mut self.entities borrow below.
            let logic_order = self.live_object_order_snapshot();
            // BulletClass/WaveClass AI already ran at each object's mixed
            // LogicClass slot. Keep their established receiver boundary here.
            let projectile_detonations =
                std::mem::take(&mut self.pending_projectile_detonations);
            let sonic_damage_requests =
                std::mem::take(&mut self.pending_wave_damage_requests);
            // Rules-less fixture dispatch is the only producer of this
            // compatibility buffer. If a caller supplies Rules later in the
            // same frame, retain the live one-receiver-at-a-time contract.
            for request in sonic_damage_requests {
                self.commit_logic_wave_damage_request(rules, overlay_registry, &request);
                // Wall-overlay and cliff mutation are proven DamageArea tails,
                // but remain explicit residuals until their per-wave producer
                // inputs are represented; do not approximate them here.
            }
            let combat_result = self.tick_combat_with_fatal_lifecycle(
                rules,
                overlay_registry,
                tick_ms,
                &logic_order,
                &tube_turn_owned_ids,
                &projectile_detonations,
                &[],
            );
            for projectile in combat_result.projectile_spawns.iter().copied() {
                let stable_id = self.allocate_stable_id();
                self.admit_projectile(stable_id, projectile);
            }
            turret::tick_turret_rotation(
                &mut self.substrate.entities,
                rules,
                self.session.binary_frame,
                &self.interner,
            );
            // S3: Unit barrel destinations were computed per-object in the
            // combat Phase-2 window (pre-death state — a unit whose target died
            // this tick keeps aiming at it this tick; idle-return starts next
            // tick). This is the unchanged write point; tick_turret_rotation
            // above still skips Units (it owns Aircraft/Building barrels until
            // their slices land).
            crate::sim::world::unit_post::apply_unit_facing(
                &mut self.substrate.entities,
                &combat_result.unit_facing,
                rules,
                &self.interner,
                self.session.binary_frame,
            );
            // SpawnManager pass. Native dispatches it per object from
            // `TechnoClass::AI_Update` (+0x2D0 → vtable+0x5C), after that
            // object's Mission_Dispatch → Fire_At → SetTarget. Running it
            // immediately after the combat phase preserves that
            // "target set, then manager reads it" ordering within the tick;
            // the manager self-gates to every 10 frames regardless.
            let ordinary_logic_order = logic_order
                .iter()
                .copied()
                .filter(|id| !tube_turn_owned_ids.contains(id))
                .collect::<Vec<_>>();
            crate::sim::spawn_manager::tick_spawn_managers(self, rules, &ordinary_logic_order);
            destroyed_structure |= combat_result.structure_destroyed;
            let combat_dead_infos: Vec<(InternedId, EntityCategory)> = combat_result
                .despawned_ids
                .iter()
                .filter_map(|&dead_id| {
                    self.substrate
                        .entities
                        .get(dead_id)
                        .map(|entity| (entity.owner, entity.category))
                })
                .collect();
            // Animated infantry remain represented and live until their death
            // sequence itself reaches UnInit. Immediate classes enter the
            // common lifecycle below.
            let mut sw_refresh_owners: Vec<InternedId> = Vec::new();
            if self.session.game_options.super_weapons && combat_result.structure_destroyed {
                for &(owner_id, category) in &combat_dead_infos {
                    if category == EntityCategory::Structure
                        && !sw_refresh_owners.contains(&owner_id)
                    {
                        sw_refresh_owners.push(owner_id);
                    }
                }
            }
            // Destroyed garrisons detach/eject their cargo while the building is
            // still alive and represented. Generic carrier recursion must not
            // consume those occupants first.
            for event in &combat_result.destroyed_garrison_buildings {
                production::eject_destruction_garrison(self, rules, event);
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
                self.release_move_sound(dead_id);
                self.uninit_with_rules(dead_id, rules);
            }
            // Bridge damage: 4-path dispatcher + cascade
            // (kill ground occupants → DropIn deck → debris → rim refresh
            // → TriggerEvent 31 → zone rebuild). Replaces the legacy
            // 2-call pipeline.
            bridge_state_changed |=
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events_with_overlay_registry(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                    overlay_registry,
                );
            debug_assert!(combat_result.tiberium_reduction_requests.is_empty());
            for req in &combat_result.tiberium_reduction_requests {
                self.reduce_tiberium_at_with_native_context(
                    (req.rx, req.ry),
                    req.amount,
                    Some(rules),
                    overlay_registry,
                );
            }
            let mut navigation_changed_cells = self
                .overlay_grid
                .as_mut()
                .map(|grid| grid.take_synchronous_navigation_cells())
                .unwrap_or_default();
            for cell in combat_result
                .terrain_navigation_changed_cells
                .iter()
                .copied()
            {
                if !navigation_changed_cells.contains(&cell) {
                    navigation_changed_cells.push(cell);
                }
            }
            if !navigation_changed_cells.is_empty() {
                tail_path_grid = self
                    .refresh_navigation_after_terrain_changes(path_grid, &navigation_changed_cells);
            }
            let post_terrain_path_grid = tail_path_grid.as_ref().or(path_grid);
            // Apply RevealOnFire events from combat.
            for ev in &combat_result.reveal_events {
                vision::reveal_radius(&mut self.fog, ev.owner, ev.rx, ev.ry, ev.radius);
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
            // Refresh superweapon grants for owners who lost structures in combat.
            if self.session.game_options.super_weapons && combat_result.structure_destroyed {
                for owner_id in sw_refresh_owners {
                    crate::sim::superweapon::refresh_super_weapons_for_owner(self, rules, owner_id);
                }
            }
            // Spawn explosion animations from combat deaths.
            for fx in &combat_result.explosion_effects {
                let frames = rules
                    .effect_frame_count(self.interner.resolve(fx.shp_name))
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
                    frame_delay: 1,
                    elapsed_frames: 0,
                    translucent: true,
                    delay_frames: 0,
                    start_sound_id: None,
                    start_sound_emitted: false,
                });
            }
            // `WaveClass::Ctor @ 0x75e950` registers Sonic (0) and Magnetron
            // (3) effects persistently. The CellClass damage vector for type 0
            // remains a separate, explicit handoff in `WaveStore::advance`.
            for event in &combat_result.fire_events {
                let Some(weapon) = rules.weapon(self.interner.resolve(event.weapon_id)) else {
                    continue;
                };
                let wave_type = if weapon.is_sonic {
                    0
                } else if weapon.is_mag_beam {
                    3
                } else {
                    continue;
                };
                let target = match event.target {
                    crate::sim::combat::TargetKind::Entity(id) => {
                        let Some(entity) = self.substrate.entities.get(id) else {
                            continue;
                        };
                        ProjectileCoord::new(
                            i32::from(entity.position.rx) * 256
                                + entity.position.sub_x.to_num::<i32>(),
                            i32::from(entity.position.ry) * 256
                                + entity.position.sub_y.to_num::<i32>(),
                            i32::from(entity.position.z)
                                * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
                        )
                    }
                    crate::sim::combat::TargetKind::Cell(rx, ry) => ProjectileCoord::new(
                        i32::from(rx) * 256 + 128,
                        i32::from(ry) * 256 + 128,
                        0,
                    ),
                };
                let source = ProjectileCoord::new(
                    i32::from(event.origin_snapshot.rx) * 256
                        + event.origin_snapshot.sub_x.to_num::<i32>(),
                    i32::from(event.origin_snapshot.ry) * 256
                        + event.origin_snapshot.sub_y.to_num::<i32>(),
                    i32::from(event.origin_snapshot.z)
                        * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
                );
                let mut wave = crate::sim::wave::Wave::new(wave_type, source, target);
                if let Some(warhead) = weapon.warhead.as_deref() {
                    wave = wave.with_damage_payload(crate::sim::wave::WaveDamagePayload {
                        firer_id: event.attacker_id,
                        base_damage: weapon.damage,
                        warhead: self.interner.intern(warhead),
                    });
                }
                // `WaveClass::UpdateCells @ 0x007610f0` is the remaining exact
                // producer seam. Do not synthesize a Bresenham/supercover list;
                // an empty vector makes that residual explicit and deterministic.
                let stable_id = self.allocate_stable_id();
                self.admit_wave(stable_id, wave);
            }
            // gamemd-derived: `EBolt::Init @ 0x004C2A60` creates one spark
            // system per electric bolt at `0x004C2B30`, passing
            // `Rules+0x1020` (`[CombatDamage] DefaultSparkSystem`) and the
            // bolt's TARGET endpoint — `EBolt+0x0C..0x14`, which
            // `TechnoClass::CreateElectricBolt @ 0x006FD516` fills from the
            // target object's coordinate virtual, and which `EBolt::Init`
            // then hands the constructor by address. Owner house,
            // attachment object and target object are all NULL; the
            // fallback aim coordinate is `0x008A0E50`, a static all-zero
            // triple, and `AI_Spark` never reads it. The handle is
            // discarded: the system lives on the global particle list and
            // expires on its own `Lifetime`.
            //
            // The path consumes no `ScenarioClass::Random` draws.
            // `EBolt::Init` does take one `RandomRanged(0, 0x100)` at
            // `0x004C2AA3`, but on the cosmetic `RandomClass` at
            // `0x00886B88` — the one `LaserDrawClass::Draw` and
            // `ThemeClass::Next_Song` also use — not the lockstep scenario
            // stream, so it is not modelled here.
            //
            // VERA-internal ordering, gamemd equivalent UNCHECKED: native
            // constructs the system inside `Fire_At`, i.e. during the
            // firer's own AI. Whether that means it is visited by the SAME
            // frame's object walk is UNCHECKED — the constructor's two
            // appends (`0x0062DD7A` into the ParticleSystemClass instance
            // registry at `0x00A80208`, and `0x0062DEF6` into the abstracts
            // registry at `0x00B0F730`) are neither of them the per-frame
            // walker, and the walker itself was not identified. This engine
            // creates it in the post-combat walk that already admits Sonic
            // and Magnetron waves from the same event list — after the
            // logic walk — so its first burst lands no earlier than
            // native's, and one frame later if native does visit
            // same-frame. Bolt rendering itself is not implemented; the
            // sparks are the part of the discharge that is a simulation
            // object.
            if let Some(spark_system_name) = rules.combat_damage.default_spark_system.as_deref()
            {
                for event in &combat_result.fire_events {
                    let Some(weapon) = rules.weapon(self.interner.resolve(event.weapon_id))
                    else {
                        continue;
                    };
                    if !weapon.is_electric_bolt {
                        continue;
                    }
                    let Some(system_type) = rules.ps_type_id_by_name(spark_system_name) else {
                        continue;
                    };
                    let coords = match event.target {
                        crate::sim::combat::TargetKind::Entity(id) => {
                            let Some(entity) = self.substrate.entities.get(id) else {
                                continue;
                            };
                            glam::IVec3::new(
                                i32::from(entity.position.rx) * 256
                                    + entity.position.sub_x.to_num::<i32>(),
                                i32::from(entity.position.ry) * 256
                                    + entity.position.sub_y.to_num::<i32>(),
                                i32::from(entity.position.z)
                                    * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
                            )
                        }
                        // VERA-internal, gamemd equivalent UNCHECKED: native
                        // takes the bolt endpoint from the TARGET OBJECT's
                        // coordinate virtual, and a cell target has no object
                        // to ask, so ground height is not folded in here.
                        crate::sim::combat::TargetKind::Cell(rx, ry) => glam::IVec3::new(
                            i32::from(rx) * 256 + 128,
                            i32::from(ry) * 256 + 128,
                            0,
                        ),
                    };
                    self.spawn_particle_system(
                        system_type,
                        coords,
                        None,
                        None,
                        glam::IVec3::ZERO,
                        None,
                        rules,
                    );
                }
            }
            // Collect fire events for render-side muzzle flash / projectile origin.
            self.invulnerability_impact_effects
                .extend(combat_result.invulnerability_impact_effects.iter().copied());
            self.fire_events.extend(combat_result.fire_events);
            // Emit radar events for combat occurrences.
            for ev in &combat_result.reveal_events {
                self.radar_events.push(RadarEventType::Combat, ev.rx, ev.ry);
            }
            // Player-asset damage pings: owner-scoped radar diamond + EVA
            // dispatch (voice gated app-side to the local player; the queue's
            // dedup result rides along as `eva_allowed`, BridgeRepaired-style).
            for ev in &combat_result.under_attack_events {
                let event_type = if ev.miner {
                    RadarEventType::HarvesterUnderAttack
                } else {
                    RadarEventType::BaseUnderAttack
                };
                let eva_allowed =
                    self.radar_events
                        .push_owned(event_type, ev.rx, ev.ry, Some(ev.owner));
                self.sound_events.push(SimSoundEvent::UnderAttack {
                    rx: ev.rx,
                    ry: ev.ry,
                    owner: ev.owner,
                    miner: ev.miner,
                    eva_allowed,
                });
            }
            // Production commits every request at its native producer. The
            // vectors remain only on hookless combat fixtures; a live world
            // request here would be an ordering regression.
            debug_assert!(self.pending_smudge_requests.is_empty());
            debug_assert!(combat_result.smudge_spawn_requests.is_empty());
            self.flush_smudge_dirty();
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
            // --- Phase 6: Legacy retaliation + Passengers ---
            // DEPENDS ON: non-receiver damage producers that still use the
            // transitional last_attacker_id handoff. Ordered area/direct
            // receiver hits already completed Mission Override inline.
            let phase_six_path_grid = post_terrain_path_grid;
            let logic_order = self.live_object_order_snapshot();
            combat::tick_retaliation(
                &mut self.substrate.entities,
                rules,
                &self.interner,
                &logic_order,
            );
            passenger_ownership_changed = passenger::tick_passenger_system(self, rules);
            self.tick_order_intents_post_combat_with_overlay_registry(
                phase_six_path_grid,
                Some(rules),
                overlay_registry,
                &tube_turn_owned_ids,
            );
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
            //     self.session.tick,
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
                phase_six_path_grid,
                overlay_registry,
            );
            production::tick_repairs(self, rules);
            building_dock::tick_building_docks(self, rules);
            crate::sim::docking::bunker_install::tick_bunker_install(
                self,
                rules,
                phase_six_path_grid,
            );
            aircraft_dock::tick_aircraft_docks(self, rules);
            if spawned_entities {
                self.refresh_fog(phase_six_path_grid, &vision_config, Some(rules));
            }
        }

        // ===== SPINE REGION: LATE — AI, defeat, anims, frame commit =====
        // (Step 3a skeleton: extracted to a region method; call order unchanged —
        // behavior-preserving.) Native-spine note: gamemd runs HouseClass updates
        // (incl. defeat) in the tail and commits the frame counter late; AI
        // placement is project-deferred and kept in its current slot.
        let late_path_grid = tail_path_grid.as_ref().or(path_grid);
        let frame_committed = self.run_late_region(
            if lane == TickLane::Ordinary {
                commands
            } else {
                &[]
            },
            rules,
            late_path_grid,
            height_map,
            overlay_registry,
            tick_ms,
            execute_tick,
            &mut executed_commands,
            &mut spawned_entities,
            &mut destroyed_structure,
            &mut placed_building_owners,
        );
        self.frame_overlay_updates = self.finalize_frame_overlays_and_navigation(
            rules,
            overlay_registry,
            destroyed_structure || bridge_state_changed || spawned_entities,
        );
        #[cfg(debug_assertions)]
        self.debug_assert_logic_membership_consistent();
        #[cfg(debug_assertions)]
        self.debug_assert_lifecycle_consistent();
        // P1+P2 production+economy shadow: mirror credits + purifier_count and
        // rebuild the factory registry from the legacy queues, after all
        // authoritative systems and before the hash. Writes only non-hashed shadow
        // fields, so state_hash stays bit-identical (proven by the *_no_hash_change
        // tests). `rules` is the advance_tick `Option<&RuleSet>` tail param.
        self.refresh_production_shadow(rules);
        #[cfg(debug_assertions)]
        self.debug_assert_production_shadow();

        // Living sprite/voxel/harvest animation state belongs to the committed
        // simulation frame. Keep it inside the authoritative frame transaction
        // so hashed side effects and every snapshot observe the same state.
        // Dying animation advancement remains in the live-object scheduler,
        // which owns the single pending-delete drain above.
        if frame_committed && let Some(animation_sequences) = animation_sequences {
            let game_options = self.session.game_options.clone();
            let binary_frame = self.session.binary_frame;
            {
                let (entities, interner) = self.entities_mut_and_interner();
                animation::tick_non_dying_animations(
                    entities,
                    animation_sequences,
                    rules,
                    &game_options,
                    interner,
                    binary_frame,
                );
            }
            animation::tick_voxel_animations(self.entities_mut());
            animation::tick_harvest_overlays(self.entities_mut());
        }
        building_anim::finalize(self, &placed_building_owners, frame_committed, rules);
        #[cfg(debug_assertions)]
        self.debug_assert_logic_membership_consistent();
        #[cfg(debug_assertions)]
        self.debug_assert_lifecycle_consistent();
        let terminal_score_finalized = self.natural_outcome_exit_ready()
            && self.finalize_terminal_score_snapshot();
        let state_hash = self.state_hash();
        TickResult {
            tick: self.session.tick,
            frame_committed,
            executed_commands,
            state_hash,
            terminal_score_finalized,
            spawned_entities,
            destroyed_structure,
            ownership_changed: passenger_ownership_changed,
            bridge_state_changed,
            movement: movement_stats,
        }
    }

    /// World owner for the dormant `TunnelLocomotionClass::Process` path.
    ///
    /// `TunnelLocomotionClass::Process @ 0x00728e30` removes the surface
    /// object before it enters state 3, then restores that same object-list
    /// membership before Foot's state-7 abort-motion cleanup.
    fn tick_tunnel_locomotor_one(&mut self, stable_id: u64, path_grid: Option<&PathGrid>) {
        let Some((mut state, position, movement_target, cell_marked, layer)) =
            self.substrate.entities.get(stable_id).and_then(|entity| {
                entity.tunnel_state.map(|state| {
                    (
                        state,
                        (entity.position.rx, entity.position.ry),
                        entity
                            .movement_target
                            .as_ref()
                            .map(|target| (target.next_index, target.path.len())),
                        entity.lifecycle.cell_marked,
                        entity.locomotor.as_ref().map(|locomotor| locomotor.layer),
                    )
                })
            })
        else {
            return;
        };

        // The Burrow transition owns the remove-before-underground ordering.
        if state.phase == tunnel_movement::TunnelPhase::Burrow && cell_marked {
            self.remove_entity_occupancy(stable_id);
        }

        let destination_reached = movement_target
            .map(|(next_index, path_len)| next_index.saturating_add(1) >= path_len)
            .unwrap_or(true);
        let surface_cell_available = path_grid.map_or(true, |grid| {
            grid.is_walkable_on_layer(position.0, position.1, MovementLayer::Ground)
        }) && self.substrate.occupancy.is_empty_on_layer(
            position.0,
            position.1,
            MovementLayer::Ground,
        );
        let mut context = TunnelProcessContext {
            destination_reached,
            surface_cell_available,
            layer: layer.unwrap_or(MovementLayer::Ground),
            z: 0,
            surface_occupied: cell_marked,
            underground_occupied: false,
            abort_motion_called: false,
        };
        let outcome = tunnel_movement::process_tunnel(&mut state, &mut context);

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.tunnel_state = Some(state);
            if let Some(locomotor) = entity.locomotor.as_mut() {
                locomotor.layer = context.layer;
                locomotor.runtime_payload =
                    crate::sim::movement::locomotion::piggyback::LocomotorRuntimePayload::Tunnel(
                        Some(state),
                    );
            }
            if context.abort_motion_called {
                entity.movement_target = None;
            }
        }

        // State 6's surface mark happens before state 7 clears Foot motion.
        if context.surface_occupied
            && self
                .substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| !entity.lifecycle.cell_marked)
        {
            self.add_entity_occupancy(stable_id);
        }
        if outcome == teleport_movement::SpecialMovementOutcome::Complete
            && self
                .substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.tunnel_state.is_some())
        {
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.tunnel_state = None;
                if let Some(locomotor) = entity.locomotor.as_mut() {
                    locomotor.runtime_payload = crate::sim::movement::locomotion::piggyback::LocomotorRuntimePayload::Tunnel(None);
                }
            }
        }
    }

    /// World owner for `DropPodLocomotionClass::Process` placement.
    ///
    /// Drop pods retain no cell-list membership while descending. On the
    /// terminal frame this performs one atomic choice: unlimbo and mark the
    /// target, or zero health and enqueue the common crush teardown.
    fn drop_pod_virtual_unlimbo_admitted(
        &self,
        stable_id: u64,
        target: (u16, u16),
        path_grid: Option<&PathGrid>,
    ) -> bool {
        use crate::sim::cell_rect::{
            IsClearToMoveResult, LiveCellPassabilityQuery, evaluate_live_cell_passability,
        };
        use crate::sim::pathfinding::cell_entry::{
            CanEnterCellContext, CanEnterLayerContext, CellEntryResult, TerrainCheckResult,
            TerrainEntryMode, check_terrain_with_layers,
            classify_occupied_cell_with_layers_and_ignored_and_occupation, evaluate_can_enter_cell,
        };

        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        let category = entity.category;
        let owner = entity.owner;
        let regular_crusher = entity.regular_crusher;
        let omni_crusher = entity.omni_crusher;
        let locomotor = entity.locomotor.as_ref();
        let movement_zone = locomotor.map_or(Default::default(), |state| state.movement_zone);
        let speed_type = locomotor.map_or(Default::default(), |state| state.speed_type);
        let locomotor_kind = locomotor.map_or(
            crate::rules::locomotor_type::LocomotorKind::Drive,
            |state| state.effective_kind(),
        );
        let cost_grid = self.terrain_costs.get(&speed_type);

        // Named location: ObjectClass::Unlimbo's virtual Foot +0x1AC gate.
        // DropPod itself never substitutes a direct list-emptiness predicate.
        let land_passable = evaluate_can_enter_cell(CanEnterCellContext {
            target,
            terrain_layer: MovementLayer::Ground,
            movement_zone: Some(movement_zone),
            speed_type: Some(speed_type),
            path_grid,
            resolved_terrain: self.resolved_terrain.as_ref(),
            terrain_costs: cost_grid,
            bypass_grid: false,
            mode: TerrainEntryMode::SpawnLike,
            is_infantry: category == EntityCategory::Infantry,
        })
        .is_clear();
        let cell_clear = evaluate_live_cell_passability(LiveCellPassabilityQuery {
            target,
            speed_type,
            movement_zone,
            requested_zone: None,
            actual_zone: 0,
            requested_layer: Some(MovementLayer::Ground),
            ignore_infantry: false,
            ignore_vehicles: false,
            land_passable,
            path_grid,
            resolved_terrain: self.resolved_terrain.as_ref(),
            raw_occupation: Some(&self.substrate.raw_cell_occupation),
        });
        if !matches!(
            cell_clear,
            IsClearToMoveResult::Clear { .. } | IsClearToMoveResult::ClearWinged
        ) {
            return false;
        }

        let layers = CanEnterLayerContext::single(MovementLayer::Ground);
        match check_terrain_with_layers(
            target,
            layers,
            category,
            path_grid,
            cost_grid,
            &self.substrate.occupancy,
        ) {
            TerrainCheckResult::Clear => true,
            TerrainCheckResult::Impassable => false,
            TerrainCheckResult::NeedsBlockerCheck => matches!(
                classify_occupied_cell_with_layers_and_ignored_and_occupation(
                    target,
                    layers,
                    stable_id,
                    movement::bump_crush::CrushCapability::new(regular_crusher, omni_crusher,),
                    self.interner.resolve(owner),
                    locomotor_kind,
                    false,
                    None,
                    &self.substrate.occupancy,
                    &self.substrate.cell_occupation,
                    &self.substrate.entities,
                    &self.house_alliances,
                    &self.interner,
                ),
                CellEntryResult::Clear
            ),
        }
    }

    fn tick_drop_pod_locomotor_one(&mut self, stable_id: u64, path_grid: Option<&PathGrid>) {
        let Some(state) = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| entity.drop_pod_state.clone())
        else {
            return;
        };

        if self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.lifecycle.cell_marked)
        {
            self.remove_entity_occupancy(stable_id);
        }

        let landing = drop_pod_movement::landing_from_virtual_unlimbo(&state, |target, _facing| {
            self.drop_pod_virtual_unlimbo_admitted(stable_id, target, path_grid)
        });
        let result = {
            let entity = self
                .substrate
                .entities
                .get_mut(stable_id)
                .expect("drop pod owner remained live during its Process visit");
            let state = entity
                .drop_pod_state
                .as_mut()
                .expect("drop pod state remained attached during its Process visit");
            drop_pod_movement::process_drop_pod_state(state, landing)
        };
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            if let (Some(state), Some(locomotor)) =
                (entity.drop_pod_state.as_ref(), entity.locomotor.as_mut())
            {
                locomotor.runtime_payload =
                    crate::sim::movement::locomotion::piggyback::LocomotorRuntimePayload::DropPod(
                        Some(state.clone()),
                    );
            }
        }

        match result.outcome {
            rocket_movement::SpecialMovementOutcome::Continue => {}
            rocket_movement::SpecialMovementOutcome::Complete => {
                let target = self
                    .substrate
                    .entities
                    .get(stable_id)
                    .and_then(|entity| entity.drop_pod_state.as_ref())
                    .map(|state| (state.target_rx, state.target_ry));
                if let (Some((rx, ry)), Some(entity)) =
                    (target, self.substrate.entities.get_mut(stable_id))
                {
                    entity.position.rx = rx;
                    entity.position.ry = ry;
                    entity.position.z = 0;
                    entity.position.exact_z_leptons = None;
                    if let Some(locomotor) = entity.locomotor.as_mut() {
                        locomotor.layer = MovementLayer::Ground;
                        locomotor.runtime_payload = crate::sim::movement::locomotion::piggyback::LocomotorRuntimePayload::DropPod(None);
                    }
                    entity.drop_pod_state = None;
                }
                self.add_entity_occupancy(stable_id);
            }
            rocket_movement::SpecialMovementOutcome::Abort => {
                if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                    entity.health.current = 0;
                    if let Some(locomotor) = entity.locomotor.as_mut() {
                        locomotor.runtime_payload = crate::sim::movement::locomotion::piggyback::LocomotorRuntimePayload::DropPod(None);
                    }
                }
                self.pending_lifecycle_requests
                    .push(LifecycleRequest::Uninit {
                        stable_id,
                        reason: crate::sim::lifecycle_request::UninitReason::Crush,
                    });
            }
        }
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
pub(crate) mod tests;

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

#[cfg(test)]
#[path = "radar_dirty_ack_tests.rs"]
mod radar_dirty_ack_tests;
