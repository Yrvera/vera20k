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
use crate::sim::combat::AttackTarget;
use crate::sim::components::{
    BridgeOccupancy, BuildingAnimOverlays, BuildingDown, BuildingUp, C4PlantState,
    DamageFireOverlays, DriveLocomotionRuntime, HarvestOverlay, Health, MovementTarget,
    NavigationState, OrderIntent, PendingC4Detonation, Position, RockingState, VoxelAnimation,
};
use crate::sim::debug_event_log::{DebugEventKind, DebugEventLog};
use crate::sim::deploy::DeployPhase;
use crate::sim::docking::aircraft_dock::AircraftAmmo;
use crate::sim::docking::building_dock::DockState;
use crate::sim::intern::InternedId;
use crate::sim::miner::Miner;
use crate::sim::movement::drive_track::{DriveTrackState, ForcedDriveTrackState};
use crate::sim::movement::droppod_movement::DropPodState;
use crate::sim::movement::locomotor::LocomotorState;
use crate::sim::movement::rocket_movement::RocketState;
use crate::sim::movement::teleport_movement::TeleportState;
use crate::sim::movement::tube_movement::LowBridgeTubeMovementState;
use crate::sim::movement::tunnel_movement::TunnelState;
use crate::sim::passenger::PassengerRole;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildingGateRuntime {
    pub mission_18_active: bool,
    pub phase: BuildingGatePhase,
    #[serde(default)]
    pub mission_state: BuildingGateMissionState,
    #[serde(default)]
    pub transition_ticks_remaining: u32,
    #[serde(default)]
    pub transition_total_ticks: u32,
    /// Native transition helper start-frame baseline. Direction reversal rewrites
    /// the active duration field but preserves this frame.
    #[serde(default)]
    pub transition_last_frame: u32,
    #[serde(default)]
    pub hold_ticks_remaining: u32,
    #[serde(default)]
    pub hold_last_frame: u32,
}

impl Default for BuildingGateRuntime {
    fn default() -> Self {
        Self {
            mission_18_active: false,
            phase: BuildingGatePhase::ClosedStable,
            mission_state: BuildingGateMissionState::Setup,
            transition_ticks_remaining: 0,
            transition_total_ticks: 0,
            transition_last_frame: 0,
            hold_ticks_remaining: 0,
            hold_last_frame: 0,
        }
    }
}

impl BuildingGateRuntime {
    pub fn can_garrison_passable(self) -> bool {
        self.mission_18_active && self.phase == BuildingGatePhase::OpenStable
    }
}

/// Authoritative-shadow lifecycle state of an object (the substrate `Presence`
/// FSM, Slice 2). Mirrors the single InLimbo bit: an object is either in the
/// active set (`InCell`) or out of it (`Limbo`). `Dying` is a transient marker
/// set during teardown right before the store slot is freed; it becomes a
/// persistent, observable state only once deferred-delete lands (later slice).
///
/// In this slice `presence` *shadows* the old gates (`in_logic_vector` + store
/// membership) — those stay authoritative — and a debug assert proves the two
/// never disagree. Not serialized (`#[serde(skip)]` on the field); rebuilt on
/// load from the restored active order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Presence {
    /// Out of the active set: born-in-limbo, concealed, or loaded as cargo. The
    /// default for a freshly constructed entity (born InLimbo).
    #[default]
    Limbo,
    /// In the active-object set and placed on the playfield (`in_logic_vector`).
    InCell,
    /// Teardown in progress — set after conceal, before the slot is freed.
    Dying,
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
    /// rebuilt from the restored order on load (native does not round-trip it).
    #[serde(skip)]
    pub in_logic_vector: bool,
    /// Substrate lifecycle shadow (Slice 2). Tracks `Limbo | InCell | Dying`.
    /// Authoritative gates remain `in_logic_vector` + store membership; this
    /// field rides alongside them and a per-tick debug assert proves they agree.
    /// Not serialized (rebuilt from the restored active order on load), and NOT
    /// hashed (non-authoritative this slice).
    #[serde(skip)]
    pub presence: Presence,
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
    pub radio_contacts: Vec<u64>,
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
    /// Persistent fire/smoke overlays on damaged buildings (health < ConditionYellow).
    pub damage_fire_overlays: Option<DamageFireOverlays>,
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
    /// Runtime state for `Gate=yes` building passability.
    ///
    /// Native `CanGarrison` accepts only mission `0x18` plus stable-open helper
    /// state. Opening and closing gates are still blockers for the same check.
    #[serde(default)]
    pub building_gate: Option<BuildingGateRuntime>,
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
    /// Debug event log — records movement/state transitions for the inspector panel.
    /// Only allocated when debug inspector is active (X hotkey). Not included in state hashing.
    #[serde(skip)]
    pub debug_log: Option<DebugEventLog>,
}

impl GameEntity {
    /// Ground-truth presence derived from the authoritative gates. A unit in the
    /// active set is `InCell` (this includes a dying-but-animating unit, which
    /// keeps ticking and stays in its cell until teardown); otherwise `Limbo`.
    /// `Dying` is never *derived* in this slice — it is only ever set imperatively
    /// during `uninit`, after which the slot is freed in the same call.
    pub fn derived_presence(&self) -> Presence {
        if self.in_logic_vector {
            Presence::InCell
        } else {
            Presence::Limbo
        }
    }

    /// Create a new entity with all required fields. Optional fields default to None/false.
    pub fn new(
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
            presence: Presence::Limbo,
            occupancy_enter_order: stable_id,
            locomotor: None,
            movement_target: None,
            navigation: NavigationState::default(),
            attack_target: None,
            radio_contacts: Vec::new(),
            rally_target: None,
            last_attacker_id: None,
            barrel_facing: None,
            building_up: None,
            building_down: None,
            building_anim_overlays: None,
            building_damage_state_active: false,
            damage_fire_overlays: None,
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
            building_gate: None,
            deploy_state: None,
            infantry: if category == EntityCategory::Infantry {
                Some(InfantryRuntime::new())
            } else {
                None
            },
            rocking: None,
            debug_log: None,
        }
    }

    /// Record a debug event if the event log is active. No-op when `debug_log` is `None`.
    pub fn push_debug_event(&mut self, tick: u32, kind: DebugEventKind) {
        if let Some(log) = &mut self.debug_log {
            log.push(tick, kind);
        }
    }

    /// Mark a live RadioClass-style contact with another entity.
    ///
    /// Contacts are idempotent and keep first-observed order so replay hashing
    /// stays deterministic.
    pub fn mark_live_contact_with(&mut self, other_stable_id: u64) {
        if !self.radio_contacts.contains(&other_stable_id) {
            self.radio_contacts.push(other_stable_id);
        }
    }

    /// Whether this entity has a live RadioClass-style contact with another entity.
    pub fn has_live_contact_with(&self, other_stable_id: u64) -> bool {
        self.radio_contacts.contains(&other_stable_id)
    }

    /// Clear a live RadioClass-style contact with another entity.
    pub fn clear_live_contact_with(&mut self, other_stable_id: u64) {
        self.radio_contacts.retain(|&sid| sid != other_stable_id);
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
        Self::new(
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

        assert_eq!(contacted.radio_contacts, vec![100]);
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
        let e = GameEntity::new(
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
mod presence_tests {
    use super::*;

    #[test]
    fn derived_presence_tracks_active_membership() {
        let mut e = GameEntity::test_default(1, "E1", "Americans", 3, 3);
        // Born in limbo: not yet in the active set.
        assert!(!e.in_logic_vector);
        assert_eq!(e.derived_presence(), Presence::Limbo);

        // Joins the active set.
        e.in_logic_vector = true;
        assert_eq!(e.derived_presence(), Presence::InCell);

        // A dying-but-animating unit stays active → still InCell (dying ignored).
        e.dying = true;
        assert_eq!(e.derived_presence(), Presence::InCell);

        // Leaves the active set.
        e.in_logic_vector = false;
        assert_eq!(e.derived_presence(), Presence::Limbo);
    }
}
