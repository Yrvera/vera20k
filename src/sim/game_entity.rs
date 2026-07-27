//! Unified entity struct replacing hecs ECS components.
//!
//! All 31 former ECS components are fields on `GameEntity`. Always-present
//! data is stored directly; optional/conditional components use `Option<T>`.
//! Zero-size markers (Selected, Repairing, VoxelModel/SpriteModel) become bools.
//!
//! ## Why plain structs?
//! - Deterministic iteration (sorted by stable_id) without per-query sorting
//! - Direct field access (`entity.position`) instead of `world.get::<&Position>(e)`
//! - No two-phase snapshot patterns needed for simple mutations
//! - Simpler borrow checker interactions than ECS archetype queries
//!
//! ## Dependency rules
//! - Part of sim/ — depends on map/ (EntityCategory), sim/components, sim/locomotor,
//!   sim/combat (AttackTarget), sim/animation, sim/miner, and special movement modules.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::map::entities::EntityCategory;
use crate::sim::aircraft::AircraftMission;
use crate::sim::animation::Animation;
use crate::sim::combat::{AttackTarget, TargetKind};
use crate::sim::components::{
    BridgeOccupancy, BuildingAnimOverlays, BuildingDown, BuildingUp, C4PlantState,
    DriveLocomotionRuntime, HarvestOverlay, Health, MovementTarget, NavigationState, OrderIntent,
    PendingC4Detonation, Position, RockingState, VoxelAnimation,
};
use crate::sim::debug_event_log::{DebugEventKind, DebugEventLog};
use crate::sim::deploy::DeployPhase;
use crate::sim::docking::aircraft_dock::AircraftAmmo;
use crate::sim::docking::building_dock::DockState;
use crate::sim::intern::InternedId;
use crate::sim::miner::Miner;
use crate::sim::mission::{MissionCom, MissionLeafState, MissionTimer, MissionType};
use crate::sim::movement::drive_track::{DriveTrackState, ForcedDriveTrackState};
use crate::sim::movement::droppod_movement::DropPodState;
use crate::sim::movement::locomotor::LocomotorState;
use crate::sim::movement::rocket_movement::RocketState;
use crate::sim::movement::teleport_movement::TeleportState;
use crate::sim::movement::tube_movement::LowBridgeTubeMovementState;
use crate::sim::movement::tunnel_movement::TunnelState;
use crate::sim::passenger::PassengerRole;
use crate::sim::radio::Contacts;
use crate::sim::slave_miner::SlaveHarvester;
use crate::sim::superweapon::invulnerability::InvulnerabilityState;

/// Infantry-only runtime fear/prone state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InfantryRuntime {
    pub fear_level: u16,
    pub is_prone: bool,
}

impl InfantryRuntime {
    pub fn new() -> Self {
        Self {
            fear_level: 0,
            is_prone: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_foundation() -> String {
    "1x1".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BuildingGatePhase {
    #[default]
    ClosedStable,
    Opening,
    OpenStable,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BuildingGateMissionState {
    #[default]
    Setup,
    OpeningWait,
    OpenHold,
    BeginClose,
    ClosingWait,
    PostClose,
}

/// The unit side of the tank-bunker reciprocal link (the pre-install approach
/// state plus the installed link, folded into one hashed field). Distinct from
/// `PassengerRole` cargo: a bunker is a single reciprocal link, never cargo.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum BunkerLink {
    /// Not heading to or inside any bunker.
    #[default]
    None,
    /// Ordered into bunker `id`, still approaching (pre-install). Cleared on any
    /// retask, which lets the building install machine reset. (The explicit
    /// unit-side marker is the abort signal the install machine reads.)
    Approaching(u64),
    /// Installed inside bunker `id` (reciprocal of `building.bunker_occupant`).
    Installed(u64),
}

impl BunkerLink {
    /// The bunker this unit is installed in, if any.
    pub fn installed_in(self) -> Option<u64> {
        match self {
            BunkerLink::Installed(id) => Some(id),
            _ => None,
        }
    }
    /// The bunker this unit is approaching, if any.
    pub fn approaching(self) -> Option<u64> {
        match self {
            BunkerLink::Approaching(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildingGateRuntime {
    pub mission_18_active: bool,
    pub phase: BuildingGatePhase,
    #[serde(default)]
    pub mission_state: BuildingGateMissionState,
    /// Open/close transition deferral. `duration` is the active remaining ticks;
    /// `start_frame` is the native helper baseline that direction reversal
    /// preserves while it rewrites the duration.
    #[serde(default)]
    pub transition_timer: MissionTimer,
    /// Nominal transition length (not a live timer) — direction reversal reads it
    /// to recompute the reversed remaining.
    #[serde(default)]
    pub transition_total_ticks: u32,
    /// Stable-open hold countdown (reseeds while occupants remain in the footprint).
    #[serde(default)]
    pub hold_timer: MissionTimer,
}

impl Default for BuildingGateRuntime {
    fn default() -> Self {
        Self {
            mission_18_active: false,
            phase: BuildingGatePhase::ClosedStable,
            mission_state: BuildingGateMissionState::Setup,
            // armed(0, 0) — NOT the sentinel — preserves the exact numeric values
            // the old (last_frame, ticks_remaining) u32 pairs held at default,
            // keeping the gate's wrapping_sub arithmetic and the state hash identical.
            transition_timer: MissionTimer::armed(0, 0),
            transition_total_ticks: 0,
            hold_timer: MissionTimer::armed(0, 0),
        }
    }
}

impl BuildingGateRuntime {
    pub fn can_garrison_passable(self) -> bool {
        self.mission_18_active && self.phase == BuildingGatePhase::OpenStable
    }
}

/// Independent ObjectClass lifecycle facts.
///
/// These bytes deliberately do not derive from health, store presence, cell
/// occupancy, LogicVector membership, or the Rust death-sequence state. Active
/// gamemd keeps those concerns independent, including alive objects that are in
/// limbo, active objects that are temporarily off-cell, and dead-limbo objects
/// that remain resolvable until the pending-delete drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectLifecycle {
    /// ObjectClass native-alive state (`ObjectClass+0x90` analogue).
    pub object_alive: bool,
    /// ObjectClass InLimbo state. This is independent of LogicVector membership.
    pub in_limbo: bool,
    /// Whether Mark/cell-list insertion currently owns cell membership.
    pub cell_marked: bool,
}

impl Default for ObjectLifecycle {
    fn default() -> Self {
        Self {
            object_alive: true,
            in_limbo: true,
            cell_marked: false,
        }
    }
}

/// Unified entity struct — replaces all hecs ECS components.
///
/// Every game object (unit, infantry, building, aircraft) is one `GameEntity`.
/// Core fields are always present; optional subsystems use `Option<T>`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameEntity {
    // --- Always present (every entity has these) ---
    /// Deterministic stable ID — primary key, used for cross-entity references,
    /// replay logs, state hashing, and networking. Never reused.
    pub stable_id: u64,
    /// World position in isometric cell coordinates + cached screen position.
    pub position: Position,
    /// Body facing direction (0–255, RA2 convention: 0=N, 64=E, 128=S, 192=W).
    pub facing: u8,
    /// Target body facing for gradual rotation (vehicles only).
    /// When `Some`, the entity is rotating in place and should not advance position.
    /// Infantry always turn instantly (RA2 behavior), so this stays `None` for them.
    pub facing_target: Option<u8>,
    /// Binary-frame body-rotation interpolator, active only while turning in place
    /// toward `facing_target`. Mirrors gamemd's hull `FacingClass` (the body
    /// PrimaryFacing turned by the drive locomotor at the unit's rules `ROT=`):
    /// turn duration is `abs(delta_8bit) / ROT` binary frames, frame-count based —
    /// NOT millisecond based. `facing` (the u8 above) stays the authoritative
    /// rendered/logic heading and is refreshed from this each tick; this is
    /// cleared to `None` whenever no in-place rotation is in progress.
    #[serde(default)]
    pub body_facing: Option<crate::sim::movement::FacingClass>,
    /// Owning player/faction name (e.g., "Americans", "Soviet") — interned for zero-cost clones.
    pub owner: InternedId,
    /// Current and maximum hit points.
    pub health: Health,
    /// rules.ini section name (e.g., "HTNK", "E1", "GAPOWR") — interned for zero-cost clones.
    pub type_ref: InternedId,
    /// Entity category: Unit, Infantry, Aircraft, or Structure.
    pub category: EntityCategory,
    /// Rules foundation string for structure footprint occupancy.
    ///
    /// Native CellClass list membership is removed from every foundation cell
    /// during ExitCell/Unlimbo-style lifecycle paths. Storing the parsed source
    /// string here lets `Simulation::uninit` perform that cleanup without a
    /// RuleSet borrow.
    #[serde(default = "default_foundation")]
    pub foundation: String,
    /// Veterancy level: 0 = rookie, 100 = veteran, 200 = elite.
    pub veterancy: u16,
    /// Fog-of-war sight range in cells.
    pub vision_range: u16,

    // --- Render model (mutually exclusive) ---
    /// true = VXL voxel model (vehicles/aircraft), false = SHP sprite (infantry/buildings).
    pub is_voxel: bool,

    // --- Bool markers (were zero-size ECS components) ---
    /// Whether this entity is currently selected by the local player.
    /// App-layer state — NOT part of authoritative simulation. Never read by sim logic.
    /// Mutations: `Command::Select` → `apply_selection_snapshot()` in world_commands.rs;
    /// combat.rs sets `selected = false` on death/transport entry.
    pub selected: bool,
    /// Building is being repaired (spending credits to heal).
    pub repairing: bool,
    /// LogicClass active-vector membership — mirrors gamemd ObjectClass+0x98.
    /// True iff this entity is currently in `Simulation::logic`. Not serialized:
    /// Rust snapshots rebuild it from the serialized LogicVector order. Exact
    /// native save/load reconstruction remains unverified.
    #[serde(skip)]
    pub in_logic_vector: bool,
    /// Independent, serialized ObjectClass lifecycle facts.
    #[serde(default)]
    pub lifecycle: ObjectLifecycle,
    /// Explicit represented type fact for the native type `+0xAC` tactical-dirty
    /// branch. False unless a caller has positive evidence; never inferred from
    /// category or render representation.
    #[serde(default)]
    pub dirty_rect_eligible: bool,
    /// Rust bookkeeping that makes the represented owner-count decrement
    /// exactly-once. This does not stand in for native-alive or `dying`.
    #[serde(default)]
    pub owned_count_released: bool,
    /// Monotonic order of the last successful insertion into a CellClass-style
    /// object list. Serialized because `OccupancyGrid` is a rebuilt cache; this
    /// is the authoritative fact needed to reconstruct its linked-list order.
    #[serde(default)]
    pub occupancy_enter_order: u64,

    // --- Optional subsystem components ---
    /// Locomotor state — present on movable entities (speed > 0 in rules.ini).
    pub locomotor: Option<LocomotorState>,
    /// Active movement path — present when unit is moving along an A* path.
    pub movement_target: Option<MovementTarget>,
    /// FootClass-style owner navigation destination state.
    ///
    /// Native `NavCom` is distinct from the active execution path, so this can
    /// remain visible after a `MovementTarget` or DriveTrack segment has cleared.
    #[serde(default)]
    pub navigation: NavigationState,
    /// Active attack target — present when entity is firing at something.
    pub attack_target: Option<AttackTarget>,
    /// RadioClass-style live contacts for this entity, stored as stable IDs.
    /// Used by runtime building-entry/pathing exceptions such as contacted
    /// war factory exits and refinery dock entry. Kept per mover; a building
    /// being contacted does not globally relax passability for unrelated units.
    #[serde(default)]
    pub radio_contacts: Contacts,
    /// Models the TechnoClass dock-entered flag (ENTER_DOCK(0x18) sets it,
    /// LEAVE_DOCK(0x19)/BREAK clears it). `Some(other_sid)` while this entity is
    /// linked-and-entered at that dock partner; `None` otherwise. Written by the
    /// radio bus; the legacy `RefineryDockContacts.contact_entered` map is kept
    /// as a transitional mirror and retired in a later slice.
    #[serde(default)]
    pub dock_entered_with: Option<u64>,
    /// Per-producer rally target cell for selected factory rally visuals.
    /// Owner-level `HouseState.rally_point` remains the production fallback.
    #[serde(default)]
    pub rally_target: Option<(u16, u16)>,
    /// Stable ID of the last entity that dealt damage (for retaliation).
    pub last_attacker_id: Option<u64>,
    /// Independent turret/barrel facing — only on entities with Turret=yes in rules.ini.
    /// Timer-based 16-bit interpolator mirroring gamemd's BarrelFacing primitive.
    pub barrel_facing: Option<crate::sim::movement::FacingClass>,
    /// Building construction animation progress.
    pub building_up: Option<BuildingUp>,
    /// Reverse build-up animation — building is undeploying into a mobile unit.
    pub building_down: Option<BuildingDown>,
    /// Active one-shot building animation overlays (e.g., ConYard crane).
    pub building_anim_overlays: Option<BuildingAnimOverlays>,
    /// Scoped native-like damaged-state gate for building visuals.
    ///
    /// Models only the proven zero/nonzero damage gate, not the full native
    /// BuildingClass BState table.
    #[serde(default)]
    pub building_damage_state_active: bool,
    /// Native BuildingClass damage-fire transition cache. Distinct from the
    /// generic damaged-art state above.
    #[serde(default)]
    pub damage_fire_state_active: bool,
    /// Eight fixed AnimClass ownership slots at Building+0x5C8..+0x5E4.
    #[serde(default)]
    pub damage_fire_anim_ids: [Option<crate::sim::anim_class::AnimId>; 8],
    /// Bridge deck occupancy marker.
    pub bridge_occupancy: Option<BridgeOccupancy>,
    /// Persistent bridge layer flag — authoritative source for "is this entity on a bridge?"
    /// Mirrors original engine's FootClass+0x8C. Survives repath operations that reset
    /// locomotor.layer. Set during spawn, updated at cell-crossing bridge transitions.
    #[serde(default)]
    pub on_bridge: bool,
    /// Infantry sprite animation state (sequence + frame + timing).
    pub animation: Option<Animation>,
    /// Voxel HVA animation state (frame cycling for multi-frame models).
    pub voxel_animation: Option<VoxelAnimation>,
    /// Harvest overlay animation (oregath.shp ore-gathering visual).
    pub harvest_overlay: Option<HarvestOverlay>,
    /// Harvester state machine (ore collection, refinery docking, cargo).
    pub miner: Option<Miner>,
    /// Slave infantry harvest AI (picks up ore, returns to master Slave Miner).
    pub slave_harvester: Option<SlaveHarvester>,
    /// Persistent high-level order (AttackMove, Guard) that survives transient state changes.
    pub order_intent: Option<OrderIntent>,
    /// Teleport movement state machine (warp out/in phases).
    pub teleport_state: Option<TeleportState>,
    /// Tunnel movement state machine (dig in/underground/dig out phases).
    pub tunnel_state: Option<TunnelState>,
    /// Active low-bridge TubeClass movement. Separate from subterranean tunnels.
    #[serde(default)]
    pub low_bridge_tube_state: Option<LowBridgeTubeMovementState>,
    /// Rocket/missile flight state machine (launch/ascend/terminal/detonate).
    pub rocket_state: Option<RocketState>,
    /// Homing missile flight state. `Some` while this entity is an in-flight
    /// homing projectile; `None` otherwise. Distinct from `rocket_state` —
    /// ballistic-arc rockets keep using `rocket_state`; only `Ranged=yes`
    /// projectiles attach a `HomingState`.
    #[serde(default)]
    pub homing_state: Option<crate::sim::movement::homing_movement::HomingState>,
    /// Drop pod descent state machine (falling/landing).
    pub droppod_state: Option<DropPodState>,
    /// Parachute descent state. `Some` while a paradropped unit is descending
    /// under a parachute, `None` otherwise. Set by
    /// `parachute_descent::begin_parachute_descent`, cleared on landing.
    #[serde(default)]
    pub parachute_state: Option<crate::sim::movement::parachute_descent::ParachuteDescentState>,
    /// Active IronCurtain or ForceShield invulnerability timer.
    /// `None` = entity is vulnerable to damage. `Some` = all damage is nullified
    /// (except healing) until the timer expires. Applied by superweapon launch handlers.
    #[serde(default)]
    pub invulnerability: Option<InvulnerabilityState>,
    /// Native `TechnoClass::IsMindControlled` gate surrogate.
    #[serde(default)]
    pub mind_controlled: bool,
    /// Active drive track curve state — present when a Drive vehicle is
    /// following a pre-computed curved path between cells.
    pub drive_track: Option<DriveTrackState>,
    /// DriveLocomotion destination/head-to state separate from curve stepping.
    #[serde(default)]
    pub drive_locomotion: Option<DriveLocomotionRuntime>,
    /// One-shot forced drive track, independent of normal path movement.
    #[serde(default)]
    pub forced_drive_track: Option<ForcedDriveTrackState>,
    /// Docking state machine — present when unit is approaching, waiting,
    /// or servicing at a repair depot.
    pub dock_state: Option<DockState>,
    /// Aircraft ammo tracking and airfield docking state.
    /// Present on aircraft with finite `Ammo=` (>= 0) from rules.ini.
    /// None for unlimited-ammo aircraft (`Ammo=-1`) and non-aircraft entities.
    pub aircraft_ammo: Option<AircraftAmmo>,
    /// Aircraft mission state machine — controls attack runs, guard, RTB, idle.
    /// Present on aircraft with Fly locomotor. None for non-aircraft and jumpjets.
    pub aircraft_mission: Option<AircraftMission>,
    /// Infantry sub-cell position (0–4). Only meaningful for infantry.
    pub sub_cell: Option<u8>,
    /// Whether this entity can be crushed by vehicles (Crushable= in rules.ini).
    /// Default false — only specific infantry and some walls are crushable.
    pub crushable: bool,
    /// Whether deployed infantry remains crushable by regular crushers.
    /// Defaults true; `DeployedCrushable=no` low-silhouette infantry blocks regular crush.
    #[serde(default = "default_true")]
    pub deployed_crushable: bool,
    /// Whether this entity can crush non-Crushable targets (OmniCrusher= in rules.ini).
    /// Only Battle Fortress has this in YR.
    pub omni_crusher: bool,
    /// Whether this entity has normal TechnoType `Crusher=yes` capability.
    /// Kept separate from MovementZone and OmniCrusher; activation waits for the
    /// Drive PerCellProcess path so legacy cell-based crush does not drift.
    #[serde(default)]
    pub regular_crusher: bool,
    /// Whether DriveLocomotion should ramp toward the computed target speed fraction.
    /// Parsed from `Accelerates=` and kept separate from raw `Speed=`.
    #[serde(default = "default_true")]
    pub drive_accelerates: bool,
    /// Whether this entity is immune to ALL crush types (OmniCrushResistant= in rules.ini).
    pub omni_crush_resistant: bool,
    /// Whether this entity ignores per-cell radiation damage (ImmuneToRadiation= in rules.ini).
    #[serde(default)]
    pub immune_to_radiation: bool,
    /// Render-only depth bias used when this entity is under or near a bridge.
    pub zfudge_bridge: i32,
    /// Prevents the unit from taking under-bridge water routes.
    pub too_big_to_fit_under_bridge: bool,
    /// Whether this entity is playing its death animation (health=0, not yet despawned).
    /// Dying entities are excluded from combat targeting, pathfinding, and selection.
    pub dying: bool,
    /// Ticks remaining before a permanently blocked infantry scatters sideways.
    /// Set when movement is stuck on a non-temporary obstacle; counts down each tick.
    /// When it reaches 0, the unit scatters to a random adjacent cell instead of
    /// endlessly repathing to the same blocked destination.
    /// Original engine: 30-frame scatter queue interval.
    pub blocked_scatter_timer: u8,

    // --- Passenger/transport system ---
    /// Original owner of a CanBeOccupied building, saved when the first garrison
    /// occupant enters. Used to revert ownership when the last occupant exits.
    /// Matches original engine's `CheckAutoSellOrCivilian` which transfers back
    /// to the Civilian house — we store the actual pre-garrison owner instead of
    /// hardcoding "Neutral".
    pub garrison_original_owner: Option<InternedId>,
    /// Combined passenger/transport role — replaces separate passenger_cargo,
    /// transport_id, and boarding_state fields. See `PassengerRole` variants.
    pub passenger_role: PassengerRole,
    /// Weapon-selection override applied when this entity is acting as a
    /// transport firing a passenger's weapon. See `WeaponOverride` for the
    /// semantics of each variant — `IfvSlot` for Gunner=yes transports,
    /// `OpenTransport` for open-topped non-Gunner transports.
    ///
    /// Set by `passenger.rs` when a passenger boards; cleared when the
    /// transport is empty.
    pub weapon_override: Option<crate::sim::combat::combat_weapon::WeaponOverride>,
    /// Temporary VXL model override for visual-only state changes.
    /// When Some, the renderer should use this type's VXL model instead of `type_ref`.
    /// Set during refinery unloading (UnloadingClass= from rules.ini).
    pub display_type_override: Option<InternedId>,
    /// Target building for an engineer-arrival intent. Set by
    /// `CaptureBuilding`, cleared on arrival or if the target is lost.
    /// Overloaded: when the target's type has `BridgeRepairHut=yes`,
    /// `tick_bridge_repair_orders` consumes the engineer for bridge repair
    /// instead of capture (the original game never captures CABHUTs).
    pub capture_target: Option<u64>,
    /// Active C4 plant intent on this attacker. Set by `Command::PlantC4`,
    /// cleared on arrival (after the building's pending detonation is set),
    /// when the player retasks the unit, or when the target is lost.
    /// `None` for non-C4 attackers or attackers not currently planting.
    #[serde(default)]
    pub c4_plant: Option<C4PlantState>,
    /// Active C4 detonation timer on this building. Set by `tick_c4_plants`
    /// when a C4-capable attacker arrives on this building's cell. Once set,
    /// `tick_c4_plants` Phase 2 fires C4Warhead damage every tick after
    /// `plant_start_tick + rules.c4_delay_ticks` until the building dies.
    /// Never cleared in the C4 path — matches gamemd marker semantics.
    /// `None` for non-buildings or buildings not currently being C4'd.
    #[serde(default)]
    pub pending_c4_detonation: Option<PendingC4Detonation>,
    /// Stable ID of the unit installed in a `Bunker=yes` building.
    ///
    /// Mirrors the live `BuildingClass+0x2E4` role for tank bunkers: an empty
    /// bunker can be skipped by the NumberImpassableRows helper, while an
    /// occupied bunker remains a normal building blocker.
    #[serde(default)]
    pub bunker_occupant: Option<u64>,
    /// Unit side of the bunker reciprocal link (approach + installed states).
    #[serde(default)]
    pub bunker_link: BunkerLink,
    /// Runtime state for `Gate=yes` building passability.
    ///
    /// Native `CanGarrison` accepts only mission `0x18` plus stable-open helper
    /// state. Opening and closing gates are still blockers for the same check.
    #[serde(default)]
    pub building_gate: Option<BuildingGateRuntime>,
    /// Tank-bunker install state machine. `Some` on `Bunker=yes` buildings from
    /// spawn (state `Idle` when empty); its presence marks the entity as a tank
    /// bunker. Drives entry admission → install.
    #[serde(default)]
    pub bunker_runtime: Option<crate::sim::docking::bunker_install::BunkerRuntime>,
    /// Active deploy-fire phase. `None` = upright (default). `Some(Deploying)` /
    /// `Some(Deployed)` / `Some(Undeploying)` for the three machine states.
    /// Hashed for lockstep determinism. Set by `Command::ToggleInfantryDeploy`,
    /// advanced by `tick_deploy_state`. Animation reflects this; combat does not
    /// read it (weapon pick is target-driven).
    #[serde(default)]
    pub deploy_state: Option<DeployPhase>,
    /// Infantry fear/prone runtime. `None` for non-infantry entities.
    #[serde(default)]
    pub infantry: Option<InfantryRuntime>,
    /// Body rocking + slope-transition state. `None` for entities that don't
    /// rock (infantry, aircraft, SHP-bodied buildings). `Some(default)` for
    /// vehicles and voxel-bodied buildings.
    #[serde(default)]
    pub rocking: Option<RockingState>,
    /// Exact native-width Mission state. All writes pass through a named legacy
    /// compatibility adapter or the dormant exact-authority surface.
    pub mission: MissionCom,
    /// Category-specific bytes read by Mission readiness and Aircraft policy.
    pub(crate) mission_leaf: MissionLeafState,
    /// Target identity archived by the Techno Override wrapper.
    pub(crate) suspended_attack_target: Option<TargetKind>,
    /// ObjectClass falling-down byte read by Infantry readiness.
    pub(crate) object_is_falling_down: u8,
    /// Sim-side model of gamemd's TechnoClass `+0x308` (`DamageSparkSystem`): the
    /// `session.tick` at which the live AI_Update damage-Spark particle system
    /// expires and the object may roll again. `0` = no live system (may roll;
    /// matches `+0x308 == NULL`); `u64::MAX` = a system whose `Lifetime <= 0`
    /// holds indefinitely; otherwise the system is live while `session.tick <
    /// live_until`. Set on a successful spark roll to `spawn_tick + sparkType.Lifetime`
    /// (matching the spawned `ParticleSystemClass` decrementing once per tick), so
    /// the per-object draw cadence stays bit-aligned with gamemd. Hashed (it gates
    /// future `scenario_rng` draws). Dormant in stock YR — see
    /// [`crate::rules::object_type::ObjectType::emits_damage_spark`].
    #[serde(default)]
    pub damage_particle_live_until: u64,
    /// Debug event log — records movement/state transitions for the inspector panel.
    /// Only allocated when debug inspector is active (X hotkey). Not included in state hashing.
    #[serde(skip)]
    pub debug_log: Option<DebugEventLog>,
}

impl GameEntity {
    /// The Harvest FSM cursor of record, decoded from
    /// `MissionCom::handler_state`. `None` when the entity has no Miner
    /// component. An out-of-vocabulary cursor decodes as `SearchOre` (the
    /// zeroed handler state every mission transition writes) — the only
    /// writers are the FSM commit and the zeroing transitions, so anything
    /// else is a logic error surfaced by the dispatch-time debug assert.
    pub fn miner_state(&self) -> Option<crate::sim::miner::MinerState> {
        self.miner.as_ref().map(|_| {
            crate::sim::miner::MinerState::from_cursor(self.mission.handler_state())
                .unwrap_or(crate::sim::miner::MinerState::SearchOre)
        })
    }

    /// Debug/test classifier: the mission + sub-phase the legacy `Option<T>`
    /// machines imply. Since the authority flip, `mission` advances only
    /// through the exact verbs — this derivation is no longer projected into
    /// it and survives purely as the cross-check the harvest seam and the
    /// passive-acquire shadow assert against.
    #[cfg(any(test, debug_assertions))]
    pub fn derived_mission(&self) -> (MissionType, u8) {
        if self.miner.is_some() {
            // The whole harvest loop is one mission; the FSM cursor of record
            // (MissionCom.handler_state) is its sub-phase.
            return (MissionType::Harvest, self.mission.handler_state() as u8);
        }
        if let Some(aircraft) = &self.aircraft_mission {
            return match aircraft {
                AircraftMission::Idle => (MissionType::Guard, 0),
                AircraftMission::Move { sub_state } => (MissionType::Move, *sub_state),
                AircraftMission::Attack { sub_state, .. } => (MissionType::Attack, *sub_state),
                AircraftMission::Guard => (MissionType::Guard, 0),
                AircraftMission::ReturnToBase { .. } => (MissionType::Enter, 0),
                AircraftMission::Docking { sub_state, .. } => (MissionType::Enter, *sub_state),
                AircraftMission::DockedIdle { .. } => (MissionType::Guard, 0),
                AircraftMission::ParaDropApproach { .. } => (MissionType::ParadropApproach, 0),
                AircraftMission::ParaDropOverfly { .. } => (MissionType::ParadropOverfly, 0),
            };
        }
        if self.dock_state.is_some() {
            return (MissionType::Enter, 0);
        }
        if self.attack_target.is_some() {
            return (MissionType::Attack, 0);
        }
        if self.movement_target.is_some() {
            return (MissionType::Move, 0);
        }
        // S3: gamemd has no "None" mission — an idle ground vehicle sits in
        // Guard(5), dispatched at [Guard] Rate (and the passive-acquire gate
        // covers missions {Move, Harvest, Guard} only, so idle Units must be
        // Guard for the later acquisition slice to ever fire). Units only this
        // slice: infantry is S6, aircraft already maps idle via
        // aircraft_mission, buildings are S8. In-transport passengers keep the
        // legacy None placeholder until the enter-transport mission commit is
        // traced.
        if self.category == EntityCategory::Unit && !self.passenger_role.is_inside_transport() {
            return (MissionType::Guard, 0);
        }
        (MissionType::None, 0)
    }

    /// Create a new entity with all required fields. Optional fields default to None/false.
    pub fn new_at_frame(
        stable_id: u64,
        rx: u16,
        ry: u16,
        z: u8,
        facing: u8,
        owner: InternedId,
        health: Health,
        type_ref: InternedId,
        category: EntityCategory,
        veterancy: u16,
        vision_range: u16,
        is_voxel: bool,
        construction_frame: u32,
    ) -> Self {
        // Infantry spawn at sub-cell 2 (top of diamond) instead of cell center
        // so they don't overlap with other units at the same position.
        let (init_sub_x, init_sub_y) = if category == EntityCategory::Infantry {
            crate::util::lepton::subcell_lepton_offset(Some(2))
        } else {
            (
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
            )
        };
        let (screen_x, screen_y) =
            crate::util::lepton::lepton_to_screen(rx, ry, init_sub_x, init_sub_y, z);
        Self {
            stable_id,
            position: Position {
                rx,
                ry,
                z,
                sub_x: init_sub_x,
                sub_y: init_sub_y,
                screen_x,
                screen_y,
            },
            facing,
            facing_target: None,
            body_facing: None,
            owner,
            health,
            type_ref,
            category,
            foundation: default_foundation(),
            veterancy,
            vision_range,
            is_voxel,
            selected: false,
            repairing: false,
            in_logic_vector: false,
            lifecycle: ObjectLifecycle::default(),
            dirty_rect_eligible: false,
            owned_count_released: false,
            occupancy_enter_order: stable_id,
            locomotor: None,
            movement_target: None,
            navigation: NavigationState::default(),
            attack_target: None,
            radio_contacts: Contacts::default(),
            dock_entered_with: None,
            rally_target: None,
            last_attacker_id: None,
            barrel_facing: None,
            building_up: None,
            building_down: None,
            building_anim_overlays: None,
            building_damage_state_active: false,
            damage_fire_state_active: false,
            damage_fire_anim_ids: [None; 8],
            bridge_occupancy: None,
            on_bridge: false,
            animation: None,
            voxel_animation: None,
            harvest_overlay: None,
            miner: None,
            slave_harvester: None,
            order_intent: None,
            teleport_state: None,
            tunnel_state: None,
            low_bridge_tube_state: None,
            rocket_state: None,
            homing_state: None,
            droppod_state: None,
            parachute_state: None,
            invulnerability: None,
            mind_controlled: false,
            drive_track: None,
            drive_locomotion: None,
            forced_drive_track: None,
            dock_state: None,
            aircraft_ammo: None,
            aircraft_mission: None,
            // Infantry get sub-cell 2 (first distinct position) at spawn so
            // they don't all pile up at cell center when multiple are created.
            sub_cell: if category == EntityCategory::Infantry {
                Some(2)
            } else {
                None
            },
            crushable: false,
            deployed_crushable: true,
            omni_crusher: false,
            regular_crusher: false,
            drive_accelerates: true,
            omni_crush_resistant: false,
            immune_to_radiation: false,
            zfudge_bridge: 7,
            too_big_to_fit_under_bridge: false,
            dying: false,
            blocked_scatter_timer: 0,
            garrison_original_owner: None,
            passenger_role: PassengerRole::None,
            weapon_override: None,
            display_type_override: None,
            capture_target: None,
            c4_plant: None,
            pending_c4_detonation: None,
            bunker_occupant: None,
            bunker_link: BunkerLink::None,
            building_gate: None,
            bunker_runtime: None,
            deploy_state: None,
            infantry: if category == EntityCategory::Infantry {
                Some(InfantryRuntime::new())
            } else {
                None
            },
            rocking: None,
            mission: MissionCom::at_frame(construction_frame),
            mission_leaf: MissionLeafState::for_entity_category(category),
            suspended_attack_target: None,
            object_is_falling_down: 0,
            damage_particle_live_until: 0,
            debug_log: None,
        }
    }

    /// Explicit frame-zero constructor for tests that do not exercise
    /// construction-time Mission timer anchoring.
    #[cfg(test)]
    pub fn new_at_frame_zero_for_test(
        stable_id: u64,
        rx: u16,
        ry: u16,
        z: u8,
        facing: u8,
        owner: InternedId,
        health: Health,
        type_ref: InternedId,
        category: EntityCategory,
        veterancy: u16,
        vision_range: u16,
        is_voxel: bool,
    ) -> Self {
        Self::new_at_frame(
            stable_id,
            rx,
            ry,
            z,
            facing,
            owner,
            health,
            type_ref,
            category,
            veterancy,
            vision_range,
            is_voxel,
            0,
        )
    }

    #[cfg(test)]
    pub(crate) fn set_object_is_falling_down_for_test(&mut self, raw: u8) {
        self.object_is_falling_down = raw;
    }

    /// Record a debug event if the event log is active. No-op when `debug_log` is `None`.
    pub fn push_debug_event(&mut self, tick: u32, kind: DebugEventKind) {
        if let Some(log) = &mut self.debug_log {
            log.push(tick, kind);
        }
    }

    /// Mark a live RadioClass-style contact with another entity.
    ///
    /// First-null slot insert: idempotent, and full slots deny without evicting
    /// (the receiver dock idiom). Slot order is hash-relevant and deterministic.
    pub fn mark_live_contact_with(&mut self, other_stable_id: u64) {
        self.radio_contacts.insert(other_stable_id);
    }

    /// Whether this entity has a live RadioClass-style contact with another entity.
    pub fn has_live_contact_with(&self, other_stable_id: u64) -> bool {
        self.radio_contacts.contains(other_stable_id)
    }

    /// Clear a live RadioClass-style contact with another entity (BREAK: nulls
    /// the slot in place, no compaction).
    pub fn clear_live_contact_with(&mut self, other_stable_id: u64) {
        self.radio_contacts.remove(other_stable_id);
    }

    /// Refresh the scoped building damaged-state visual gate from current HP.
    ///
    /// Returns true when the stored gate changed. Non-structures cannot carry
    /// this building visual state and are forced inactive.
    pub fn refresh_building_damage_state_gate(&mut self, condition_yellow_x1000: i64) -> bool {
        let previous = self.building_damage_state_active;
        let active = if self.category == EntityCategory::Structure && self.health.max > 0 {
            let current = self.health.current as i64;
            let max = self.health.max as i64;
            current * 1000 <= max * condition_yellow_x1000
        } else {
            false
        };
        self.building_damage_state_active = active;
        previous != active
    }

    /// Runtime movement/path layer with Ground as the fallback.
    ///
    /// This is not the object-list selector. Use `occupancy_list_layer` when
    /// selecting gamemd `FirstObject` versus `AltObject` style occupancy.
    pub fn movement_layer_or_ground(&self) -> crate::sim::movement::locomotor::MovementLayer {
        self.locomotor.as_ref().map_or(
            crate::sim::movement::locomotor::MovementLayer::Ground,
            |l| l.layer,
        )
    }

    /// Object-list layer for occupancy/cache membership.
    ///
    /// This mirrors gamemd's `ObjectClass+0x8C` / `OnBridge` selector for
    /// `CellClass::FirstObject` versus `AltObject`. It is intentionally not the
    /// same as locomotor/path layer; ramps can have `loco.layer` and `on_bridge`
    /// disagree for a tick.
    pub fn occupancy_list_layer(&self) -> Option<crate::sim::movement::locomotor::MovementLayer> {
        use crate::sim::movement::locomotor::MovementLayer;

        let motion_layer = self
            .locomotor
            .as_ref()
            .map_or(MovementLayer::Ground, |l| l.layer);
        if matches!(
            motion_layer,
            MovementLayer::Air | MovementLayer::Underground
        ) {
            return None;
        }

        Some(if self.on_bridge {
            MovementLayer::Bridge
        } else {
            MovementLayer::Ground
        })
    }

    /// Whether this entity is currently on a bridge deck.
    pub fn is_on_bridge_layer(&self) -> bool {
        self.on_bridge
    }

    /// Create a minimal entity for testing. Fills sensible defaults for most fields.
    #[cfg(test)]
    /// Create a minimal test entity with the given owner and type_ref strings.
    /// Uses a shared test interner via `test_intern()` for consistent IDs.
    pub fn test_default(stable_id: u64, type_ref: &str, owner: &str, rx: u16, ry: u16) -> Self {
        Self::new_at_frame_zero_for_test(
            stable_id,
            rx,
            ry,
            0, // z = ground level
            0, // facing = north
            crate::sim::intern::test_intern(owner),
            Health {
                current: 100,
                max: 100,
            },
            crate::sim::intern::test_intern(type_ref),
            EntityCategory::Unit,
            0, // veterancy = rookie
            5, // vision_range = 5 cells
            true,
        )
    }

    /// Whether this entity is alive (health > 0).
    pub fn is_alive(&self) -> bool {
        self.health.current > 0
    }

    /// Whether ObjectClass native-alive state is set. This is intentionally
    /// independent from health and the Rust death-sequence state.
    pub fn is_object_alive(&self) -> bool {
        self.lifecycle.object_alive
    }

    /// Transitional Rust system gate until ordinary Infantry lifecycle authority
    /// migrates. Distinct from both health-based `is_alive()` and native-alive
    /// `is_object_alive()`; death-sequence state still suppresses current raw-store
    /// consumers even while ObjectClass native-alive remains set.
    pub fn is_active(&self) -> bool {
        self.lifecycle.object_alive && !self.dying
    }

    /// Whether this entity is in any deploy phase (Deploying, Deployed, or Undeploying).
    /// Used by the 7 movement-command handlers to silently ignore movement orders.
    pub fn is_deployed(&self) -> bool {
        self.deploy_state.is_some()
    }

    /// Whether this entity has finished deploying and is in the stationary
    /// Deployed phase (not transitioning).
    pub fn is_fully_deployed(&self) -> bool {
        matches!(self.deploy_state, Some(DeployPhase::Deployed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::terrain;

    #[test]
    fn test_new_entity_defaults() {
        let e = GameEntity::test_default(1, "HTNK", "Americans", 30, 40);
        assert_eq!(e.stable_id, 1);
        assert_eq!(e.type_ref, crate::sim::intern::test_intern("HTNK"));
        assert_eq!(e.owner, crate::sim::intern::test_intern("Americans"));
        assert_eq!(e.position.rx, 30);
        assert_eq!(e.position.ry, 40);
        assert_eq!(e.position.z, 0);
        assert_eq!(e.facing, 0);
        assert_eq!(e.health.current, 100);
        assert_eq!(e.health.max, 100);
        assert_eq!(e.category, EntityCategory::Unit);
        assert_eq!(e.veterancy, 0);
        assert_eq!(e.vision_range, 5);
        assert!(e.is_voxel);
        assert!(!e.selected);
        assert!(!e.repairing);
        assert!(e.locomotor.is_none());
        assert!(e.movement_target.is_none());
        assert!(e.attack_target.is_none());
        assert!(e.radio_contacts.is_empty());
        assert_eq!(e.rally_target, None);
        assert!(e.last_attacker_id.is_none());
        assert!(e.barrel_facing.is_none());
        assert!(e.miner.is_none());
        assert!(e.order_intent.is_none());
        assert!(!e.building_damage_state_active);
        assert!(!e.on_bridge);
    }

    #[test]
    fn occupancy_list_layer_from_on_bridge_not_loco_layer() {
        // GATE A2 / P5 (L13): the object-list layer is selected by the occupant's
        // OnBridge byte, NOT the locomotor/path layer. Pin a mismatch — loco.layer
        // = Ground while on_bridge = true — and assert the list layer follows
        // on_bridge (Bridge), the gamemd `Object+0x8C` selector.
        use crate::rules::locomotor_type::LocomotorKind;
        use crate::sim::movement::locomotor::{LocomotorState, MovementLayer};

        let mut entity = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
        let mut loco = LocomotorState::for_test_kind(LocomotorKind::Drive);
        loco.layer = MovementLayer::Ground;
        entity.locomotor = Some(loco);

        entity.on_bridge = true;
        assert_eq!(
            entity.occupancy_list_layer(),
            Some(MovementLayer::Bridge),
            "list layer must follow on_bridge, not loco.layer"
        );

        entity.on_bridge = false;
        assert_eq!(
            entity.occupancy_list_layer(),
            Some(MovementLayer::Ground),
            "list layer must follow on_bridge when off the deck"
        );
    }

    fn building_damage_state_entity(current: u16, max: u16) -> GameEntity {
        let mut entity = GameEntity::test_default(10, "GAPOWR", "Americans", 4, 5);
        entity.category = EntityCategory::Structure;
        entity.health = Health { current, max };
        entity
    }

    #[test]
    fn building_damage_state_non_structure_stays_false_even_below_yellow() {
        let mut entity = GameEntity::test_default(10, "MTNK", "Americans", 4, 5);
        entity.health = Health {
            current: 25,
            max: 100,
        };

        assert!(!entity.refresh_building_damage_state_gate(500));
        assert!(!entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_structure_above_yellow_stays_false() {
        let mut entity = building_damage_state_entity(51, 100);

        assert!(!entity.refresh_building_damage_state_gate(500));
        assert!(!entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_structure_exactly_at_yellow_sets_true() {
        let mut entity = building_damage_state_entity(50, 100);

        assert!(entity.refresh_building_damage_state_gate(500));
        assert!(entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_structure_below_yellow_sets_true() {
        let mut entity = building_damage_state_entity(49, 100);

        assert!(entity.refresh_building_damage_state_gate(500));
        assert!(entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_repaired_above_yellow_clears_true() {
        let mut entity = building_damage_state_entity(49, 100);
        entity.building_damage_state_active = true;
        entity.health.current = 51;

        assert!(entity.refresh_building_damage_state_gate(500));
        assert!(!entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_zero_max_health_clears_false() {
        let mut entity = building_damage_state_entity(0, 0);
        entity.building_damage_state_active = true;

        assert!(entity.refresh_building_damage_state_gate(500));
        assert!(!entity.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_serde_round_trip_preserves_true() {
        let mut entity = building_damage_state_entity(40, 100);
        entity.building_damage_state_active = true;

        let json = serde_json::to_string(&entity).expect("serialize entity");
        let restored: GameEntity = serde_json::from_str(&json).expect("deserialize entity");

        assert!(restored.building_damage_state_active);
    }

    #[test]
    fn building_damage_state_serde_default_absent_field_is_false() {
        let mut value =
            serde_json::to_value(building_damage_state_entity(40, 100)).expect("serialize entity");
        value
            .as_object_mut()
            .expect("entity serializes to object")
            .remove("building_damage_state_active");

        let restored: GameEntity = serde_json::from_value(value).expect("deserialize entity");

        assert!(!restored.building_damage_state_active);
    }

    #[test]
    fn new_entity_has_no_rally_target() {
        let e = GameEntity::test_default(1, "GAWEAP", "Americans", 30, 40);
        assert_eq!(e.rally_target, None);
    }

    #[test]
    fn live_contacts_are_per_entity_and_idempotent() {
        let mut contacted = GameEntity::test_default(1, "MTNK", "Americans", 30, 40);
        let unrelated = GameEntity::test_default(2, "MTNK", "Americans", 31, 40);

        contacted.mark_live_contact_with(100);
        contacted.mark_live_contact_with(100);

        assert_eq!(contacted.radio_contacts.len(), 1); // idempotent — one slot used
        assert!(contacted.has_live_contact_with(100));
        assert!(!unrelated.has_live_contact_with(100));

        contacted.clear_live_contact_with(100);
        assert!(!contacted.has_live_contact_with(100));
    }

    #[test]
    fn test_is_alive() {
        let mut e = GameEntity::test_default(1, "E1", "Soviet", 10, 10);
        assert!(e.is_alive());
        e.health.current = 0;
        assert!(!e.is_alive());
    }

    #[test]
    fn test_screen_coords_computed() {
        let e = GameEntity::new_at_frame_zero_for_test(
            1,
            30,
            40,
            2, // z=2 for elevation
            0,
            crate::sim::intern::test_intern("Americans"),
            Health {
                current: 100,
                max: 100,
            },
            crate::sim::intern::test_intern("HTNK"),
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        // lepton_to_screen = CoordsToClient(cell_center) = iso_to_screen + (30, 15)
        let (corner_sx, corner_sy) = terrain::iso_to_screen(30, 40, 2);
        assert!((e.position.screen_x - (corner_sx + 30.0)).abs() < 0.01);
        assert!((e.position.screen_y - corner_sy).abs() < 0.01);
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn lifecycle_authority_state_axes_are_independent() {
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        assert_eq!(e.lifecycle, ObjectLifecycle::default());
        assert!(!e.in_logic_vector);
        assert!(!e.dirty_rect_eligible);
        assert!(!e.owned_count_released);

        // Exercise every combination without deriving one fact from another.
        for bits in 0u8..64 {
            e.lifecycle.object_alive = bits & 0b00_0001 != 0;
            e.lifecycle.in_limbo = bits & 0b00_0010 != 0;
            e.lifecycle.cell_marked = bits & 0b00_0100 != 0;
            e.in_logic_vector = bits & 0b00_1000 != 0;
            e.dying = bits & 0b01_0000 != 0;
            e.health.current = if bits & 0b10_0000 != 0 { 1 } else { 0 };

            assert_eq!(e.is_object_alive(), bits & 0b00_0001 != 0);
            assert_eq!(e.lifecycle.in_limbo, bits & 0b00_0010 != 0);
            assert_eq!(e.lifecycle.cell_marked, bits & 0b00_0100 != 0);
            assert_eq!(e.in_logic_vector, bits & 0b00_1000 != 0);
            assert_eq!(e.dying, bits & 0b01_0000 != 0);
            assert_eq!(e.is_alive(), bits & 0b10_0000 != 0);
            assert_eq!(e.is_active(), e.lifecycle.object_alive && !e.dying,);
        }
    }
}

#[cfg(test)]
mod mission_shadow_tests {
    use super::*;
    use crate::sim::combat::{AttackTarget, TargetKind};
    use crate::sim::mission::state::MissionTestFixture;
    use crate::sim::mission::{MissionDispatchTimer, MissionId, MissionType};

    #[test]
    fn mission_defaults_to_idle_none() {
        let e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        assert_eq!(e.mission.current(), MissionId::NONE);
        assert_eq!(e.mission.handler_state(), 0);
        assert_eq!(e.mission.ai_counter(), 0);
    }

    #[test]
    fn derived_mission_idle_when_no_machine_active() {
        // S3: an idle machine-less Unit sits in Guard (the gamemd idle mission
        // for ground vehicles); other categories keep the legacy None
        // placeholder until their slices land (infantry S6, buildings S8).
        let e = GameEntity::test_default(1, "E1", "Americans", 3, 3); // Unit
        assert_eq!(e.derived_mission(), (MissionType::Guard, 0));

        let mut s = GameEntity::test_default(2, "GAPILE", "Americans", 3, 3);
        s.category = crate::map::entities::EntityCategory::Structure;
        assert_eq!(s.derived_mission(), (MissionType::None, 0));

        let mut i = GameEntity::test_default(3, "E1", "Americans", 3, 3);
        i.category = crate::map::entities::EntityCategory::Infantry;
        assert_eq!(i.derived_mission(), (MissionType::None, 0));
    }

    #[test]
    fn passenger_derive_unchanged_placeholder() {
        // In-transport passengers keep the legacy None placeholder: the
        // mission a passenger holds inside a transport is untraced (do NOT
        // guess Sleep/Guard); this pin flips with the traced value later.
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        e.passenger_role = crate::sim::passenger::PassengerRole::Inside { transport_id: 99 };
        assert_eq!(e.derived_mission(), (MissionType::None, 0));
    }

    #[test]
    fn derived_mission_tracks_attack_target() {
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        e.attack_target = Some(AttackTarget {
            target: TargetKind::Entity(2),
            cooldown_ticks: 0,
            burst_remaining: 0,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        });
        assert_eq!(e.derived_mission().0, MissionType::Attack);
    }

    #[test]
    fn mission_round_trips_through_serde() {
        // Slice 6 un-skips the field so the queued/suspended interrupt stack +
        // timer survive a save/load. (current/substate are also reconciled from
        // the legacy machines on load; the rest persists as serialized.)
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        e.mission.apply_test_fixture(MissionTestFixture {
            current: MissionId::from_known(MissionType::Attack),
            suspended: MissionId::NONE,
            queued: MissionId::from_known(MissionType::Guard),
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 99,
            dispatch_timer: MissionDispatchTimer::at_frame(0),
        });
        let json = serde_json::to_string(&e).expect("serialize entity");
        assert!(
            json.contains("mission"),
            "mission must round-trip — present in serialized form"
        );
        let restored: GameEntity = serde_json::from_str(&json).expect("deserialize entity");
        assert_eq!(
            restored.mission.current(),
            MissionId::from_known(MissionType::Attack)
        );
        assert_eq!(
            restored.mission.queued(),
            MissionId::from_known(MissionType::Guard)
        );
        assert_eq!(restored.mission.ai_counter(), 99);
    }
}
