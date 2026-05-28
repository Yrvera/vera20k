//! Game object type definitions parsed from rules.ini.
//!
//! Every unit, vehicle, aircraft, and building in RA2 is defined by a section in
//! rules.ini. This module provides the `ObjectType` struct which captures the
//! common properties shared by all game objects. Object-specific behavior (e.g.,
//! infantry prone stance, building power grid) is handled by category-specific
//! fields with sensible defaults.
//!
//! ## rules.ini format
//! ```ini
//! [MTNK]
//! Name=Grizzly Battle Tank
//! Cost=700
//! Strength=300
//! Armor=heavy
//! Speed=6
//! Sight=6
//! TechLevel=2
//! Owner=Americans,Alliance,British,French,Germans,Koreans
//! Prerequisite=GAWEAP
//! Primary=105mm
//! Image=MTNK
//! ```
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use glam::IVec3;

use crate::rules::ini_parser::IniSection;
use crate::rules::jumpjet_params::JumpjetParams;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::util::fixed_math::{SimFixed, sim_from_f32};

/// Which type registry an object belongs to.
///
/// Determines which `[XxxTypes]` section listed this object and affects
/// which game behaviors apply (e.g., only buildings have power, only
/// infantry can garrison).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObjectCategory {
    Infantry,
    Vehicle,
    Aircraft,
    Building,
}

/// Sidebar/build-queue classification for buildable buildings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildCategory {
    Tech,
    Resource,
    Power,
    Infrastructure,
    Combat,
}

impl BuildCategory {
    fn from_ini(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "tech" => Some(Self::Tech),
            "resource" | "resoure" => Some(Self::Resource),
            "power" => Some(Self::Power),
            "infrastructure" => Some(Self::Infrastructure),
            "combat" => Some(Self::Combat),
            _ => None,
        }
    }
}

/// What pip display to show below a unit's health bar (PipScale= in rules.ini).
///
/// Controls the type of pip overlay rendered beneath selected units:
/// - `Tiberium`: cargo fill pips for harvesters (green=ore, colored=gem)
/// - `Passengers`: passenger count pips for transports
/// - `Ammo`: ammunition count pips
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipScale {
    #[default]
    None,
    Tiberium,
    Passengers,
    Ammo,
}

impl PipScale {
    fn from_ini(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "tiberium" => Self::Tiberium,
            "passengers" => Self::Passengers,
            "ammo" => Self::Ammo,
            _ => Self::None,
        }
    }
}

/// What type of objects this building can produce (Factory= in rules.ini).
///
/// RA2 uses this key to determine which production queue a building serves:
/// a building with `Factory=InfantryType` acts as a barracks, one with
/// `Factory=UnitType` as a war factory, etc. This replaces hardcoded
/// building-name checks and lets modders add new factories without code changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactoryType {
    /// Produces buildings (ConYards: GACNST, NACNST, YACNST).
    BuildingType,
    /// Produces infantry (Barracks: GAPILE, NAHAND, YABRCK).
    InfantryType,
    /// Produces vehicles (War Factories: GAWEAP, NAWEAP, YAWEAP).
    UnitType,
    /// Produces aircraft (Airfields: GAAIRC, AMRADR).
    AircraftType,
}

impl FactoryType {
    /// Parse the Factory= INI value (case-insensitive).
    pub fn from_ini(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "buildingtype" => Some(Self::BuildingType),
            "infantrytype" => Some(Self::InfantryType),
            "unittype" => Some(Self::UnitType),
            "aircrafttype" => Some(Self::AircraftType),
            _ => None,
        }
    }
}

/// One docking pad on a building. Stored in `ObjectType.pads` as a Vec
/// whose index IS the pad index (0-based, matching `DockingOffset0..N-1`).
///
/// `lepton_offset` is parsed from art.ini `DockingOffset%d=X,Y,Z` and is
/// interpreted as an offset from the building's geometric center (not its
/// origin top-left). 256 leptons = 1 cell. Zero-initialized entries are
/// valid: when rules declares `NumberOfDocks=N` but art only specifies
/// `DockingOffset0..K-1` with K < N, the remaining pads get zero offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DockPad {
    pub lepton_offset: (i32, i32, i32),
}

/// A game object definition parsed from a rules.ini section.
///
/// Fields use sensible defaults when the INI key is absent, matching
/// the original game's behavior (RA2 uses hardcoded defaults for missing keys).
#[derive(Debug, Clone)]
pub struct ObjectType {
    /// Section name in rules.ini (e.g., "MTNK", "E1", "GAWEAP").
    /// This is the unique identifier used throughout the engine.
    pub id: String,
    /// Which type registry this object belongs to.
    pub category: ObjectCategory,
    /// Display name (CSF string table key or raw text). None if not specified.
    pub name: Option<String>,
    /// Credit cost to produce this object.
    pub cost: i32,
    /// Hit points (health). 0 = invincible or not applicable.
    pub strength: i32,
    /// Armor type name (e.g., "heavy", "light", "wood"). Determines damage
    /// multipliers from warhead Verses= values.
    pub armor: String,
    /// Movement speed (0 = immobile, e.g., buildings).
    pub speed: i32,
    /// Inertia weight (`Weight=` in vehicle/aircraft sections). Default 2.0.
    /// Used as the divisor in rocker-impulse force scaling: heavier units rock
    /// proportionally less per equivalent impulse. Retail range: 0.5 (lightest
    /// vehicles) to 5 (Aircraft Carrier).
    pub weight: SimFixed,
    /// Fraction of max speed gained per tick during acceleration (AccelerationFactor=).
    /// Default 0.03. At 15 fps, reaches max speed in ~2 seconds.
    pub accel_factor: SimFixed,
    /// Fraction of max speed lost per tick during braking (DeaccelerationFactor=).
    /// Default 0.02. Applied when within slowdown_distance of destination.
    pub decel_factor: SimFixed,
    /// Whether Drive locomotors ramp toward target speed (`Accelerates=`).
    /// Defaults to true; `Accelerates=false` is handled by DriveLocomotion speed
    /// fraction ownership, not by mutating raw `Speed=`.
    pub accelerates: bool,
    /// Lepton distance from destination at which braking begins (SlowdownDistance=).
    /// Default 512 (~2 cells). Original engine default is 500.
    pub slowdown_distance: i32,
    /// Vision range in cells.
    pub sight: i32,
    /// Technology level required (-1 = unbuildable by player).
    pub tech_level: i32,
    /// Multiplier to the time it takes for this object to be built.
    pub build_time_multiplier: f32,
    /// `build_time_multiplier` pre-scaled ×1000 for deterministic build-time computation.
    pub build_time_multiplier_x1000: u64,
    /// Which houses/sides can build this (e.g., ["Americans", "Alliance"]).
    pub owner: Vec<String>,
    /// Specific countries that may build this object.
    pub required_houses: Vec<String>,
    /// Countries explicitly forbidden from building this (ForbiddenHouses= in rules.ini).
    /// Inverse of Owner — if the player's country is in this list, they cannot build.
    pub forbidden_houses: Vec<String>,
    /// Whether this type may appear in multiplayer starting-unit generation.
    pub allowed_to_start_in_multiplayer: bool,
    /// Building prerequisites required before this can be built.
    pub prerequisite: Vec<String>,
    /// Alternative prerequisite path (PrerequisiteOverride= in rules.ini).
    /// If non-empty AND the owner has ANY building from this list, the normal
    /// Prerequisite check is skipped entirely (OR logic).
    pub prerequisite_override: Vec<String>,
    /// Maximum simultaneous copies allowed (BuildLimit= in rules.ini). Default 0 = unlimited.
    /// Positive: hard cap. Negative: abs value cap with rebuild-after-death semantics.
    pub build_limit: i32,
    /// Requires spy infiltration of an Allied Battle Lab to unlock.
    pub requires_stolen_allied_tech: bool,
    /// Requires spy infiltration of a Soviet Battle Lab to unlock.
    pub requires_stolen_soviet_tech: bool,
    /// Requires spy infiltration of a Yuri Battle Lab to unlock.
    pub requires_stolen_third_tech: bool,
    /// Primary weapon ID (references a [WeaponName] section).
    pub primary: Option<String>,
    /// Secondary weapon ID (e.g., anti-air for dual-purpose units).
    pub secondary: Option<String>,
    /// `ElitePrimary=` from rules.ini. Replaces `primary` when the unit is at
    /// Elite tier (veterancy >= 200). Falls back to `primary` if absent.
    /// Veteran tier (100..199) does NOT swap — only Elite does.
    pub elite_primary: Option<String>,
    /// `EliteSecondary=` from rules.ini. Replaces `secondary` when the unit is
    /// at Elite tier (veterancy >= 200). Falls back to `secondary` if absent.
    pub elite_secondary: Option<String>,
    /// Art.ini image reference. Defaults to the object's ID if not specified.
    /// Used to look up sprite/voxel filenames in art.ini.
    pub image: String,
    /// Power generation (positive) or consumption (negative). Buildings only.
    pub power: i32,
    /// Extra power bonus per occupant for `InfantryAbsorb`/`UnitAbsorb`
    /// buildings. Parsed from `ExtraPower=` (signed i32). Only contributes
    /// when the building has `InfantryAbsorb=yes` or `UnitAbsorb=yes` and
    /// at least one passenger is garrisoned. Stock YR: YAPOWR Bio-Reactor
    /// uses `ExtraPower=100` × up to 5 garrisoned infantry.
    pub extra_power: i32,
    /// Building foundation footprint (e.g., "3x2", "1x1"). Buildings only.
    pub foundation: String,
    /// Pixel offset for health bar / selection bracket Y position.
    /// Negative values shift the bar UP (above taller sprites). Default 0.
    /// Parsed from `PixelSelectionBracketDelta` in rules.ini.
    pub pixel_selection_bracket_delta: i32,
    /// Sidebar/build tab grouping for structures.
    pub build_cat: Option<BuildCategory>,
    /// Human placement radius away from existing base-normal structures.
    pub adjacent: i32,
    /// Whether this structure expands the owner's build area.
    pub base_normal: bool,
    /// Whether this structure can expand allied build area when BuildOffAlly is enabled.
    pub eligibile_for_ally_building: bool,
    /// Whether selling/destruction can eject infantry crew from this structure.
    pub crewed: bool,
    /// Sound ID played when this unit is selected (references sound.ini section).
    pub voice_select: Option<String>,
    /// Sound ID played when this unit is ordered to move.
    pub voice_move: Option<String>,
    /// Sound ID played when this unit is ordered to attack.
    pub voice_attack: Option<String>,
    /// Sound ID played when this entity dies or is destroyed.
    pub die_sound: Option<String>,
    /// Sound ID played while this entity moves (looping engine/footstep).
    pub move_sound: Option<String>,
    /// Sound ID played when this unit reacts to taking fire (fear cry).
    pub voice_feedback: Option<String>,
    /// Sound ID played when this unit performs a special attack.
    pub voice_special_attack: Option<String>,
    /// Sound ID played when this entity is crushed by a vehicle (squish).
    pub crush_sound: Option<String>,
    /// Sound ID played when this unit deploys (e.g. GI sandbag-up).
    pub deploy_sound: Option<String>,
    /// Sound ID played when this unit undeploys.
    pub undeploy_sound: Option<String>,
    /// Sound played at the destination cell when this unit warps in
    /// (chrono teleport arrival).
    pub chrono_in_sound: Option<String>,
    /// Sound played at the source cell when this unit warps out
    /// (chrono teleport departure).
    pub chrono_out_sound: Option<String>,
    /// Whether this unit has an independently rotating turret.
    /// Parsed from rules.ini `Turret=yes`. Only meaningful for vehicles/aircraft.
    pub has_turret: bool,
    /// Turret rotation speed in RA2 "ROT" units (degrees per game frame at 15fps).
    /// Higher = faster turret rotation. Only meaningful when `has_turret` is true.
    /// Typical values: 5 (War Miner), 7 (Grizzly/Rhino).
    pub turret_rot: i32,
    /// VXL turret model name for buildings (TurretAnim= in rules.ini, e.g., "SAM").
    /// The engine loads `{TurretAnim}.VXL` + `{TurretAnim}.HVA` as the turret model.
    pub turret_anim: Option<String>,
    /// Whether the turret anim is a VXL model (TurretAnimIsVoxel=, default false).
    /// When false, the TurretAnim is an SHP overlay handled by the building anim system.
    pub turret_anim_is_voxel: bool,
    /// Pixel X offset for building turret placement (TurretAnimX=).
    pub turret_anim_x: i32,
    /// Pixel Y offset for building turret placement (TurretAnimY=).
    pub turret_anim_y: i32,
    /// Depth adjustment for building turret (TurretAnimZAdjust=, negative = behind).
    pub turret_anim_z_adjust: i32,
    /// Scan radius in cells for auto-targeting idle enemies. If None, defaults
    /// to the primary weapon's range at runtime.
    pub guard_range: Option<SimFixed>,
    /// Range bonus (in cells) added to the weapon's max range when firing at
    /// high-flying targets. Read from `AirRangeBonus=` in the unit section.
    /// None means no bonus.
    pub air_range_bonus: Option<SimFixed>,
    /// Whether this unit fires a warhead at its own position on death (e.g.,
    /// Apocalypse Tank explosion damages nearby units).
    pub explodes: bool,
    /// Specific weapon fired on death (overrides default explosion behavior).
    /// References a [WeaponName] section in rules.ini.
    pub death_weapon: Option<String>,
    /// Superweapon type ID granted when this building is completed (SuperWeapon= in rules.ini).
    /// References a section listed in [SuperWeaponTypes].
    pub super_weapon: Option<String>,
    /// Secondary superweapon type ID, typically from an upgrade (SuperWeapon2= in rules.ini).
    pub super_weapon2: Option<String>,
    /// When true, this building provides full map vision while powered.
    /// Used by the Allied Spy Satellite Uplink (GASPYSAT).
    pub spy_sat: bool,
    /// When true, this building/unit emits a gap field that hides enemy vision
    /// within GapRadius cells (parsed from [General]).
    pub gap_generator: bool,
    /// When true, this building activates the owner's radar display (minimap).
    /// Radar=yes in rules.ini. Used by GARADR (Allied), NARADR (Soviet), YARADR (Yuri).
    /// SpySat=yes buildings also implicitly provide radar.
    pub radar: bool,
    /// When true, this unit does NOT appear on enemy radar even when in line of sight.
    /// RadarInvisible= in rules.ini. Used by subs, Night Hawk, dolphins, giant squid.
    pub radar_invisible: bool,
    /// When true, this unit ALWAYS appears on radar even when under shroud.
    /// RadarVisible= in rules.ini. Used by certain special objects.
    pub radar_visible: bool,
    /// Whether this unit is a resource harvester (Harvester=yes in rules.ini).
    /// Data-driven replacement for hardcoded type ID string checks.
    pub harvester: bool,
    /// Whether this structure accepts ore/gem delivery (Refinery=yes in rules.ini).
    pub refinery: bool,
    /// Whether this building has a bib (`Bib=yes` in rules.ini). When true, the
    /// east-edge column of the foundation footprint is unit-passable — units
    /// can drive across that strip even though the cells remain part of the
    /// building's placement / ownership footprint. Matches the original
    /// engine's HasBib relaxation in the per-cell occupant chain check.
    pub bib: bool,
    /// Whether this building is a native opening gate (`Gate=yes`).
    ///
    /// Consumed by the runtime cell-entry classifier together with live building
    /// gate state. `GateStages=` is visual timing data and is not part of the
    /// `CanGarrison` passability predicate.
    pub gate: bool,
    /// Native helper transition duration from `DeployTime=` converted as
    /// `trunc(value * 900)`. Used by building gates for opening/closing.
    pub deploy_time_ticks: u32,
    /// Native open-hold delay from `GateCloseDelay=` converted as
    /// `trunc(value * 900)`.
    pub gate_close_delay_ticks: u32,
    /// Bonus credits storage for refineries (Storage= in rules.ini).
    /// Refineries typically have Storage=300 — added to owner credits on placement.
    pub storage: i32,
    /// Free unit spawned when this structure is placed (FreeUnit= in rules.ini).
    pub free_unit: Option<String>,
    /// Structures this unit may dock with (Dock= in rules.ini), normalized uppercase.
    pub dock: Vec<String>,
    /// Queueing cell offset from building origin (QueueingCell= in art.ini).
    /// Where miners wait outside the dock. Merged from art.ini during init.
    pub queueing_cell: Option<(u16, u16)>,
    /// All docking pads on this building, parsed from art.ini `DockingOffset0..N-1`.
    /// Index in vec IS the pad index.
    ///
    /// After the art→rules merge:
    /// - If art declared at least one `DockingOffset%d`, the vec is sized to
    ///   `number_of_docks` (zero-padding any indices missing in art, truncating excess).
    /// - If art declared none, the vec is left empty — consumers fall back to
    ///   their own anchor (e.g. `refinery_pad_cell`'s rightmost-column default).
    ///
    /// Service depots / single-pad helipads / airfields with declared offsets
    /// use this vec directly. Retail refineries (GAREFN/NAREFN/YAREFN) leave
    /// it empty.
    pub pads: Vec<DockPad>,
    /// Cells added to the rectangular foundation (from art.ini AddOccupy1..N).
    /// Merged from art.ini during init.
    pub add_occupy: Vec<(i16, i16)>,
    /// Cells removed from the rectangular foundation (from art.ini RemoveOccupy1..N).
    /// Merged from art.ini during init.
    pub remove_occupy: Vec<(i16, i16)>,
    /// Alternative VXL model displayed while unloading at a refinery (UnloadingClass= in rules.ini).
    /// e.g. HARV uses HORV (harvester without ore bin), CMIN uses CMON.
    pub unloading_class: Option<String>,
    /// Ammo count for aircraft. -1 = unlimited (default), 0+ = finite.
    /// Aircraft with finite ammo return to a helipad/airfield to reload after depleting.
    pub ammo: i32,

    // -- Slave Miner / economy fields --
    /// Infantry type enslaved/spawned by this unit (Enslaves= in rules.ini, YR only).
    /// Used by Slave Miner (SMIN) to spawn SLAV workers.
    pub enslaves: Option<String>,
    /// Number of slaves to spawn (SlavesNumber= in rules.ini). Default 0.
    pub slaves_number: i32,
    /// Frames before a dead slave is regenerated (SlaveRegenRate= in rules.ini). Default 0.
    pub slave_regen_rate: u32,
    /// Minimum frames between individual slave respawns (SlaveReloadRate= in rules.ini). Default 0.
    pub slave_reload_rate: u32,
    /// Whether this infantry is a slave unit (Slaved=yes in rules.ini).
    /// Slave units are bound to a master (Slave Miner) and have restricted AI.
    pub slaved: bool,
    /// `Fearless=yes` on InfantryType. Suppresses fear/prone panic changes.
    pub fearless: bool,
    /// `Fraidycat=yes` on InfantryType. First fear hit immediately maxes fear.
    pub fraidycat: bool,
    /// `Crawls=yes` from art.ini. Controls the prone movement speed branch.
    pub crawls: bool,
    /// Primary standing infantry projectile/damage frame from art.ini `FireUp=`.
    pub fire_up_frame: u8,
    /// Primary prone infantry projectile/damage frame from art.ini `FireProne=`.
    pub fire_prone_frame: u8,
    /// Secondary standing infantry projectile/damage frame from art.ini `SecondaryFire=`.
    pub secondary_fire_frame: u8,
    /// Secondary prone/deploy infantry projectile/damage frame from art.ini `SecondaryProne=`.
    pub secondary_prone_frame: u8,
    /// Whether VeteranAbilities includes FEARLESS for this type.
    pub veteran_fearless: bool,
    /// Whether EliteAbilities includes FEARLESS for this type.
    pub elite_fearless: bool,
    /// Frames between bale pickups for slave harvesters (HarvestRate= in rules.ini). Default 0.
    pub harvest_rate: u32,
    /// AI flag: this unit earns money (ResourceGatherer=yes in rules.ini). Default false.
    pub resource_gatherer: bool,
    /// AI flag: this is a resource delivery point (ResourceDestination=yes in rules.ini). Default false.
    pub resource_destination: bool,
    /// Whether this building is an Ore Purifier (OrePurifier=yes in rules.ini).
    /// Owning one grants a PurifierBonus to all harvested ore.
    pub ore_purifier: bool,

    // -- Locomotor / movement fields --
    /// Which locomotor class controls this unit's movement (parsed from Locomotor= CLSID).
    pub locomotor: LocomotorKind,
    /// Which terrain cells are traversable (SpeedType= in rules.ini).
    pub speed_type: SpeedType,
    /// Pathfinder routing assumptions (MovementZone= in rules.ini).
    pub movement_zone: MovementZone,
    /// Whether this unit is treated as aircraft for game logic (ConsideredAircraft=).
    pub considered_aircraft: bool,
    /// Per-type render depth bias used when a unit is near or under a bridge.
    /// Original engine default is 7 when the key is absent.
    pub zfudge_bridge: i32,
    /// Prevents naval/large units from traversing under bridge structural cells.
    pub too_big_to_fit_under_bridge: bool,
    /// Whether this unit shows a visible crash animation on death (Crashable=).
    pub crashable: bool,
    /// Whether this unit can use chrono teleport movement (Teleporter=).
    pub teleporter: bool,
    /// Whether this unit can fire while hovering / in air (HoverAttack=).
    pub hover_attack: bool,
    /// Whether this unit stays airborne by default / doesn't land (BalloonHover=).
    pub balloon_hover: bool,
    /// AirportBound=yes — aircraft must dock at helipad; crashes if none available.
    pub airport_bound: bool,
    /// Fighter=yes — fighter aircraft classification (affects targeting).
    pub fighter: bool,
    /// FlyBy=yes — strafing fly-by attack pattern (continue forward after firing).
    pub fly_by: bool,
    /// FlyBack=yes — after fly-by, reverse course back over target.
    pub fly_back: bool,
    /// Landable=yes — aircraft can land on the ground.
    pub landable: bool,
    /// Whether this unit uses jumpjet controls (JumpJet= in rules.ini).
    pub jumpjet: bool,
    /// Jumpjet-specific tuning parameters. Only populated when `jumpjet` is true.
    pub jumpjet_params: Option<JumpjetParams>,

    // -- Deploy / undeploy fields --
    /// What this unit deploys into (e.g., AMCV DeploysInto=GACNST).
    /// Parsed from rules.ini `DeploysInto=`. Used for MCV→ConYard and similar transforms.
    pub deploys_into: Option<String>,
    /// What this building undeploys into (e.g., GACNST UndeploysInto=AMCV).
    /// Parsed from rules.ini `UndeploysInto=`. Used for ConYard→MCV sell-back.
    pub undeploys_into: Option<String>,
    /// Raw 8-bit facing required before a unit can deploy into this building type.
    /// Parsed from building-side `DeployFacing=` as INI value << 5; default is 0x80.
    pub deploy_facing: u8,
    /// Whether this building is a construction yard. Enables ConYard-only MCV repack gates.
    pub construction_yard: bool,

    /// Whether this unit can be crushed by vehicles with Crusher movement zones.
    /// Default: false for all types. Parsed from `Crushable=` in rules.ini.
    /// Only specific infantry (GI, GGI, SEAL, Rocketeer, Lunar) and some walls
    /// have this set to true.
    pub crushable: bool,
    /// Whether deployed infantry remains crushable by regular crushers.
    /// Defaults to true; stock Guardian GI overrides this with `DeployedCrushable=no`.
    pub deployed_crushable: bool,
    /// Whether this unit has normal TechnoType `Crusher=yes` capability.
    /// Distinct from `OmniCrusher=` and MovementZone crusher names.
    pub crusher: bool,
    /// When true, this building cannot receive ForceShield invulnerability.
    /// From `NoForceShield=yes` in rulesmd.ini.
    pub no_force_shield: bool,
    /// Whether this unit can crush non-Crushable targets (only Battle Fortress).
    /// Default: false. Parsed from `OmniCrusher=` in rules.ini.
    pub omni_crusher: bool,
    /// Whether this unit is immune to ALL crush types including OmniCrusher.
    /// Default: false. Parsed from `OmniCrushResistant=` in rules.ini.
    pub omni_crush_resistant: bool,

    /// What type of objects this building can produce (Factory= in rules.ini).
    /// None for non-factory buildings/units. Data-driven replacement for
    /// hardcoded building-name checks in production queue logic.
    pub factory: Option<FactoryType>,
    /// Whether this building clones produced infantry (Cloning=yes in rules.ini).
    pub cloning: bool,

    /// Exit coordinate for produced units, in leptons relative to building origin.
    /// Parsed from `ExitCoord=X,Y,Z` in rules.ini. 256 leptons = 1 cell.
    /// Used by spawn logic to place newly built units near the correct factory exit.
    pub exit_coord: Option<(i32, i32, i32)>,

    // -- Cursor / interaction capability flags --
    // These drive which cursor is shown when hovering this unit/building.
    /// Whether this infantry type behaves as an engineer (captures buildings,
    /// repairs structures). Parsed from `Engineer=yes` in rules.ini.
    /// Triggers `EngineerRepair` cursor on damaged friendly buildings and
    /// `Enter` cursor on capturable enemy buildings when this unit is selected.
    pub engineer: bool,

    /// Whether this unit can self-deploy/undeploy via the Deploy command.
    /// Parsed from `Deployer=yes` in rules.ini. Triggers `Deploy`/`NoDeploy`
    /// cursor when the player hovers over this unit itself.
    pub deployer: bool,

    /// Whether this building can be infiltrated by a spy or captured by an engineer.
    /// Parsed from `Capturable=yes` in rules.ini. Enables the `Enter` cursor
    /// when an enemy Engineer or Spy is selected and hovering this building.
    pub capturable: bool,

    /// Whether this building can be repaired via the Repair command.
    /// Parsed from `Repairable=yes` in rules.ini. Defaults to true for buildings.
    pub repairable: bool,

    /// Whether infantry can garrison/occupy this building.
    /// Parsed from `CanBeOccupied=yes` in rules.ini. Enables `Enter` cursor
    /// for friendly infantry hovering this building.
    pub can_be_occupied: bool,

    /// Whether garrisoned infantry can fire from this building.
    /// Parsed from `CanOccupyFire=yes` in rules.ini. Building must also
    /// have `CanBeOccupied=yes` and at least one occupant for fire to occur.
    pub can_occupy_fire: bool,

    /// Whether to show pip indicators for each occupant inside the building.
    /// Parsed from `ShowOccupantPips=yes` in rules.ini.
    pub show_occupant_pips: bool,

    /// Whether Engineer entry into this building triggers bridge-segment
    /// repair on the nearest damaged bridge. Parsed from
    /// `BridgeRepairHut=yes` in rules.ini. Stock CABHUT is the only
    /// consumer in retail. Default `false`.
    pub bridge_repair_hut: bool,

    /// Maximum number of infantry passengers this vehicle can carry.
    /// Parsed from `Passengers=N` in rules.ini. >0 enables `Enter` cursor
    /// for friendly infantry hovering this transport.
    pub passengers: u32,

    /// Maximum Size= of individual passenger allowed (SizeLimit= in rules.ini).
    /// 0 means no size restriction. SizeLimit=2 means only Size<=2 can enter.
    pub size_limit: u32,

    /// How much transport space this unit occupies (Size= in rules.ini).
    /// Infantry default 1, vehicles default 3.
    pub size: u32,

    /// Whether this transport is open-topped — passengers can fire from inside.
    /// Parsed from `OpenTopped=yes` in rules.ini.
    pub open_topped: bool,

    /// Whether this transport uses the Gunner system (IFV weapon swap).
    /// Parsed from `Gunner=yes` in rules.ini. When a passenger enters, the
    /// transport's active weapon changes based on the passenger's IFVMode.
    pub gunner: bool,

    /// Which IFV weapon turret index this infantry type selects when inside
    /// a Gunner=yes transport. Parsed from `IFVMode=N` in rules.ini. Default 0.
    pub ifv_mode: u32,

    /// `OpenTransportWeapon=` from rules.ini. Slot selector when this infantry
    /// is riding inside an open-topped transport that is NOT `Gunner=yes`:
    /// `0` fires the passenger's Primary, `1` fires the passenger's Secondary,
    /// `-1` (default) means no override — the transport doesn't fire on the
    /// passenger's behalf. Distinct from IFVMode, which only applies to Gunner
    /// transports that swap their own weapon based on the passenger's slot.
    pub open_transport_weapon: i32,

    /// Whether this infantry can toggle into a deploy-fire stance (DeployFire=yes
    /// in rules.ini). Only deploy-fire types respond to `Command::ToggleInfantryDeploy`.
    /// Stock YR sets this on GI (E1), GuardianGI (GGI), and a handful of others.
    pub deploy_fire: bool,

    /// Index of the weapon (0=primary, 1=secondary) that the AI auto-deploy planner
    /// considers when deciding "should I deploy here?". Parsed from `DeployFireWeapon=N`
    /// in rules.ini. Default `None`. Not consulted in B1 (no AI auto-deploy);
    /// fire-time weapon pick is target-driven via `select_weapon_with_ifv`.
    pub deploy_fire_weapon: Option<i32>,

    /// Maximum number of garrison occupants for CanBeOccupied buildings.
    /// Parsed from `MaxNumberOccupants=N` in rules.ini. Default 0.
    pub max_number_occupants: u32,

    /// Whether this infantry can garrison `CanBeOccupied` buildings.
    /// Parsed from `Occupier=yes` in rules.ini. Only GI and Conscript in base RA2.
    pub occupier: bool,

    /// Whether this infantry can assault enemy buildings (hostile garrison entry).
    /// Parsed from `Assaulter=yes` in rules.ini.
    pub assaulter: bool,

    /// Weapon used when this infantry fires from inside a garrisoned building.
    /// Parsed from `OccupyWeapon=WeaponName` in rules.ini. Falls back to
    /// primary weapon if not specified.
    pub occupy_weapon: Option<String>,

    /// Elite-level weapon used when garrisoned. Falls back to `OccupyWeapon`
    /// or primary weapon if not specified.
    /// Parsed from `EliteOccupyWeapon=WeaponName` in rules.ini.
    pub elite_occupy_weapon: Option<String>,

    /// pips.shp frame index for this infantry when garrisoned in a building.
    /// Parsed from `OccupyPip=PersonGreen` in rules.ini.  Default 7 (PersonGreen).
    /// Values: 7=PersonGreen, 8=PersonYellow, 9=PersonWhite, 10=PersonRed,
    /// 11=PersonBlue, 12=PersonPurple.  Empty slots use frame 6.
    pub occupy_pip: u32,

    /// What pip display to render below the health bar (PipScale= in rules.ini).
    /// `Tiberium` shows per-bale cargo pips for harvesters using pips2.shp.
    pub pip_scale: PipScale,

    /// Whether this building absorbs infantry (Yuri Bio Reactor).
    /// Parsed from `InfantryAbsorb=yes` in rules.ini.
    pub infantry_absorb: bool,

    /// Whether this building absorbs vehicles.
    /// Parsed from `UnitAbsorb=yes` in rules.ini.
    pub unit_absorb: bool,

    /// Whether this techno type can enter a Tank Bunker.
    /// Parsed from `Bunkerable=` in rules.ini. UnitTypeClass entries default
    /// true; other object categories default false.
    pub bunkerable: bool,

    /// Indexed weapon list for IFV (Weapon1..Weapon17 in rules.ini).
    /// Only populated when Gunner=yes. Index 0 = Weapon1.
    pub weapon_list: Vec<String>,

    /// Whether this unit shows an `Attack` cursor even on friendly targets.
    /// Parsed from `AttackCursorOnFriendlies=yes` in rules.ini.
    /// Used by Desolator and Boris whose weapons affect friendlies.
    pub attack_cursor_on_friendlies: bool,

    /// Whether this infantry uses the `Enter`/sabotage cursor instead of
    /// the normal `Attack` cursor on enemy structures.
    /// Parsed from `SabotageCursor=yes` in rules.ini. Used by Tanya and Navy SEAL.
    pub sabotage_cursor: bool,

    /// `C4=yes` on InfantryType. Gates the player-issued C4 plant mission path
    /// (SEAL, Tanya, Psi-Corp Trooper). Distinct from `sabotage_cursor`, which
    /// is now purely a modder-flag for cursor display on weapons; the live
    /// cursor + click behavior is driven by `c4 + can_c4` instead.
    pub c4: bool,

    /// `CanC4=yes` on BuildingType. When false, the building cannot be C4'd by
    /// SEAL/Tanya/PTROOP. Default `true` for buildings, `false` for non-buildings.
    /// Stock buildings opting out in retail rulesmd.ini: CAMISC01 (Concrete
    /// Barrel), CAMISC02 (Wooden Barrel), CAMISC06 (Civilian Barrel variant),
    /// AMMOCRAT (Ammo Crate).
    pub can_c4: bool,

    /// `Invisible=yes` on BuildingType. Live building rejection checks this
    /// plain invisible byte separately from `InvisibleInGame`.
    pub invisible: bool,

    /// `InvisibleInGame=yes` on BuildingType. Logical-only buildings (e.g., bridge
    /// anchors) that should not receive C4 or other interaction cursors.
    pub invisible_in_game: bool,

    /// Whether this building repairs docked ground units (UnitRepair=yes in rules.ini).
    /// Used by Service Depots (GADEPT, NADEPT, YADEPT).
    pub unit_repair: bool,
    /// Whether this building is a Tank Bunker (Bunker=yes in rules.ini).
    /// Stock YR uses this on NATBNK. The live pathing helper treats empty and
    /// occupied bunkers differently, so this must be data-driven rather than a
    /// NATBNK string check.
    pub bunker: bool,
    /// Whether this building reloads ammo for docked aircraft (UnitReload=yes in rules.ini).
    /// Used by Airfields (GAAIRC, NAAIRC).
    pub unit_reload: bool,
    /// Whether this building is a helipad (Helipad=yes in rules.ini).
    pub helipad: bool,
    /// How many units may dock at this building simultaneously (NumberOfDocks= in rules.ini).
    /// Default 1. Airfields typically have 4.
    pub number_of_docks: u8,

    /// Whether this building can be toggled on/off by the player.
    /// Parsed from `TogglePower=yes` in rules.ini.
    /// Defaults to true for buildings (most can be powered down).
    /// Triggers `TogglePower` cursor when hovering this building in power-toggle mode.
    pub toggle_power: bool,

    /// Whether this building is affected by low-power situations.
    /// Parsed from `Powered=yes` in rules.ini. Defaults to true for buildings.
    /// When true and the owner is in low power, the building deactivates:
    /// defenses stop firing, radar goes offline, gap/spysat/superweapons pause.
    /// Power plants (positive Power=) are never deactivated regardless of this flag.
    pub powered: bool,

    /// Whether this unit can use the disguise ability (Spy).
    /// Parsed from `CanDisguise=yes` in rules.ini. Enables `Disguise` cursor
    /// when the selected Spy hovers over an eligible enemy infantry target.
    pub can_disguise: bool,

    /// Whether this building is a wall segment (Wall=yes in rules.ini).
    /// Wall buildings render as overlays (auto-tiled connectivity frames),
    /// not as normal SHP building sprites. GAWALL, NAWALL, GAFWLL etc.
    pub wall: bool,

    // -- Naval flags --
    /// Building requires water placement (WaterBound=yes in INI).
    /// When set, the placement validator checks the water speed column instead
    /// of Buildable. Default: true if SpeedType is Float, false otherwise.
    pub water_bound: bool,
    /// Unit or building is classified as naval (Naval=yes in INI).
    /// Controls AI targeting priority, factory classification, and UI filtering.
    pub naval: bool,
    /// Number of foundation rows (from the top, Y-axis) that are impassable.
    /// Default -1 = all rows impassable. Parsed from NumberImpassableRows= in rules.ini.
    /// Controls which foundation cells units can path through (e.g., war factory
    /// exit lanes, naval yard docks).
    pub number_impassable_rows: i32,

    // -- Point light source fields (from rules.ini, primarily buildings) --
    /// Light emission range in leptons (LightVisibility= in rules.ini). Default 5000.
    /// 256 leptons = 1 cell. Used by lamp posts (GALITE=5000) and other light-emitting buildings.
    pub light_visibility: i32,
    /// Light emission brightness (LightIntensity= in rules.ini). Default 0.0.
    /// Negative values darken the area. Typical range: 0.0–1.0.
    pub light_intensity: f32,
    /// Red channel tint for emitted light (LightRedTint= in rules.ini). Default 1.0.
    pub light_red_tint: f32,
    /// Green channel tint for emitted light (LightGreenTint= in rules.ini). Default 1.0.
    pub light_green_tint: f32,
    /// Blue channel tint for emitted light (LightBlueTint= in rules.ini). Default 1.0.
    pub light_blue_tint: f32,

    // -- TechnoType particle effects --
    // String fields below hold unresolved particle-system section names; ID
    // resolution against the particle-system registry is deferred (matches
    // the same deferred-typing decision used for ParticleType.warhead and
    // ParticleSystemType.holds_what's parse-side captures).
    /// `NaturalParticleSystem=` — gap-generator / cloak-related particle
    /// system. Live code path in YR (BuildingClass::UpdateGapGenerator_Tick),
    /// but no retail INI sets the key, so the slot is normally null.
    pub natural_particle_system: Option<String>,
    /// `NaturalParticleLocation=` X,Y,Z offset paired with the system above.
    pub natural_particle_location: IVec3,
    /// `RefinerySmokeParticleSystem=` — chimney smoke for refineries.
    pub refinery_smoke_particle_system: Option<String>,
    /// `DamageParticleSystems=` CSV list — spawned periodically while the
    /// object is damaged.
    pub damage_particle_systems: Vec<String>,
    /// `DestroyParticleSystems=` CSV list. Parsed for completeness;
    /// no live consumer in retail YR.
    pub destroy_particle_systems: Vec<String>,
    /// `DamageSmokeOffset=` X,Y,Z — anchor offset for damage smoke.
    pub damage_smoke_offset: IVec3,
    /// `DamSmkOffScrnRel=` — interpret `DamageSmokeOffset` as screen-relative
    /// rather than world-relative.
    pub dam_smk_off_scrn_rel: bool,
    /// `DestroySmokeOffset=` X,Y,Z — anchor offset for destruction smoke.
    pub destroy_smoke_offset: IVec3,
    /// Four `RefinerySmokeOffsetOne/Two/Three/Four=` X,Y,Z triplets.
    /// Used to position the four chimney-smoke emitters on a refinery.
    pub refinery_smoke_offsets: [IVec3; 4],
    /// `RefinerySmokeFrames=` — frame count for the smoke particle system.
    pub refinery_smoke_frames: u16,
    /// `GapRadiusInCells=` — per-object gap-generator radius (overrides the
    /// global default for this object).
    pub gap_radius_in_cells: u8,
    /// `SuperGapRadiusInCells=` — oversized gap radius applied during
    /// specific game states.
    pub super_gap_radius_in_cells: u8,
    /// `PsychicDetectionRadius=` — selected Psychic Sensor ring radius.
    pub psychic_detection_radius: u8,
    /// `SensorArray=` — building sensor-array flag used by GetSensorRange.
    pub sensor_array: bool,
    /// `Sensors=` — generic cloak-sensor flag.
    pub sensors: bool,
    /// `SensorsSight=` — fallback sensor/cloak-generator ring radius.
    pub sensors_sight: u8,
    /// `CloakGenerator=` — cloak field provider flag used by GetSensorRange.
    pub cloak_generator: bool,
}

fn native_minutes_to_ticks(value: f32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        (f64::from(value) * 900.0).trunc().min(u32::MAX as f64) as u32
    }
}

impl ObjectType {
    /// Whether this type participates in selected factory rally-line visuals.
    pub fn has_rally_line(&self) -> bool {
        matches!(
            self.factory,
            Some(FactoryType::InfantryType | FactoryType::UnitType)
        ) || self.unit_repair
            || self.cloning
    }

    /// Parse an ObjectType from a rules.ini section.
    ///
    /// Missing keys get sensible defaults matching RA2's behavior.
    /// The `id` is the section name, and `category` comes from which
    /// type registry listed this object.
    pub fn from_ini_section(id: &str, section: &IniSection, category: ObjectCategory) -> Self {
        let owner: Vec<String> = section
            .get_list("Owner")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let prerequisite: Vec<String> = section
            .get_list("Prerequisite")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let required_houses: Vec<String> = section
            .get_list("RequiredHouses")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let forbidden_houses: Vec<String> = section
            .get_list("ForbiddenHouses")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let prerequisite_override: Vec<String> = section
            .get_list("PrerequisiteOverride")
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let btm_f32: f32 = section.get_f32("BuildTimeMultiplier").unwrap_or(1.0);

        Self {
            id: id.to_string(),
            category,
            name: section.get("Name").map(|s| s.to_string()),
            cost: section.get_i32("Cost").unwrap_or(0),
            strength: section.get_i32("Strength").unwrap_or(0),
            armor: section.get("Armor").unwrap_or("none").to_string(),
            speed: section.get_i32("Speed").unwrap_or(0),
            weight: section
                .get_f32("Weight")
                .map(sim_from_f32)
                .unwrap_or(SimFixed::lit("2.0")),
            accel_factor: section
                .get_f32("AccelerationFactor")
                .map(sim_from_f32)
                .unwrap_or(SimFixed::lit("0.03")),
            decel_factor: section
                .get_f32("DeaccelerationFactor")
                .map(sim_from_f32)
                .unwrap_or(SimFixed::lit("0.002")),
            accelerates: section.get_bool("Accelerates").unwrap_or(true),
            slowdown_distance: section.get_i32("SlowdownDistance").unwrap_or(500),
            sight: section.get_i32("Sight").unwrap_or(0),
            tech_level: section.get_i32("TechLevel").unwrap_or(-1),
            build_time_multiplier: btm_f32,
            build_time_multiplier_x1000: (btm_f32.max(0.01) as f64 * 1000.0).round() as u64,
            owner,
            required_houses,
            forbidden_houses,
            allowed_to_start_in_multiplayer: section
                .get_bool("AllowedToStartInMultiplayer")
                .unwrap_or(true),
            prerequisite,
            prerequisite_override,
            build_limit: section.get_i32("BuildLimit").unwrap_or(0),
            requires_stolen_allied_tech: section
                .get_bool("RequiresStolenAlliedTech")
                .unwrap_or(false),
            requires_stolen_soviet_tech: section
                .get_bool("RequiresStolenSovietTech")
                .unwrap_or(false),
            requires_stolen_third_tech: section
                .get_bool("RequiresStolenThirdTech")
                .unwrap_or(false),
            primary: section.get("Primary").map(|s| s.to_string()),
            secondary: section.get("Secondary").map(|s| s.to_string()),
            elite_primary: section.get("ElitePrimary").map(|s| s.to_string()),
            elite_secondary: section.get("EliteSecondary").map(|s| s.to_string()),
            image: section.get("Image").unwrap_or(id).to_string(),
            power: section.get_i32("Power").unwrap_or(0),
            extra_power: section.get_i32("ExtraPower").unwrap_or(0),
            // The original resolves Foundation= through a fixed name table.
            // merge_art_data() applies the art-vs-rules precedence observed in gamemd.
            foundation: crate::rules::foundation::foundation_name(
                section.get("Foundation").unwrap_or("1x1"),
            )
            .to_string(),
            pixel_selection_bracket_delta: section
                .get_i32("PixelSelectionBracketDelta")
                .unwrap_or(0),
            build_cat: section.get("BuildCat").and_then(BuildCategory::from_ini),
            adjacent: section.get_i32("Adjacent").unwrap_or(3),
            base_normal: section.get_bool("BaseNormal").unwrap_or(true),
            eligibile_for_ally_building: section
                .get_bool("EligibileForAllyBuilding")
                .unwrap_or(false),
            crewed: section.get_bool("Crewed").unwrap_or(false),
            voice_select: section.get("VoiceSelect").map(|s| s.to_string()),
            voice_move: section.get("VoiceMove").map(|s| s.to_string()),
            voice_attack: section.get("VoiceAttack").map(|s| s.to_string()),
            die_sound: section.get("DieSound").map(|s| s.to_string()),
            move_sound: section.get("MoveSound").map(|s| s.to_string()),
            voice_feedback: section.get("VoiceFeedback").map(|s| s.to_string()),
            voice_special_attack: section.get("VoiceSpecialAttack").map(|s| s.to_string()),
            crush_sound: section.get("CrushSound").map(|s| s.to_string()),
            deploy_sound: section.get("DeploySound").map(|s| s.to_string()),
            undeploy_sound: section.get("UndeploySound").map(|s| s.to_string()),
            chrono_in_sound: section.get("ChronoInSound").map(|s| s.to_string()),
            chrono_out_sound: section.get("ChronoOutSound").map(|s| s.to_string()),
            has_turret: section.get_bool("Turret").unwrap_or(false),
            // gamemd writes a separate UnitType +0x398=10 for Harvester/Weeder,
            // but ROT= remains the parsed TechnoType +0x71C facing-rate field.
            turret_rot: section.get_i32("ROT").unwrap_or(0),
            turret_anim: section
                .get("TurretAnim")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_uppercase()),
            turret_anim_is_voxel: section.get_bool("TurretAnimIsVoxel").unwrap_or(false),
            turret_anim_x: section.get_i32("TurretAnimX").unwrap_or(0),
            turret_anim_y: section.get_i32("TurretAnimY").unwrap_or(0),
            turret_anim_z_adjust: section.get_i32("TurretAnimZAdjust").unwrap_or(0),
            guard_range: section.get_f32("GuardRange").map(sim_from_f32),
            air_range_bonus: section.get_f32("AirRangeBonus").map(sim_from_f32),
            explodes: section.get_bool("Explodes").unwrap_or(false),
            death_weapon: section.get("DeathWeapon").map(|s| s.to_string()),
            super_weapon: section.get("SuperWeapon").map(|s| s.to_string()),
            super_weapon2: section.get("SuperWeapon2").map(|s| s.to_string()),
            spy_sat: section.get_bool("SpySat").unwrap_or(false),
            gap_generator: section.get_bool("GapGenerator").unwrap_or(false),
            radar: section.get_bool("Radar").unwrap_or(false),
            radar_invisible: section.get_bool("RadarInvisible").unwrap_or(false),
            radar_visible: section.get_bool("RadarVisible").unwrap_or(false),
            harvester: section.get_bool("Harvester").unwrap_or(false),
            refinery: section.get_bool("Refinery").unwrap_or(false),
            bib: section.get_bool("Bib").unwrap_or(false),
            gate: section.get_bool("Gate").unwrap_or(false),
            deploy_time_ticks: native_minutes_to_ticks(
                section.get_f32("DeployTime").unwrap_or(0.0),
            ),
            gate_close_delay_ticks: native_minutes_to_ticks(
                section.get_f32("GateCloseDelay").unwrap_or(0.0),
            ),
            storage: section.get_i32("Storage").unwrap_or(0),
            free_unit: section.get("FreeUnit").map(|s| s.to_string()),
            dock: section
                .get_list("Dock")
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_uppercase())
                .collect(),
            queueing_cell: None,       // merged from art.ini later
            pads: Vec::new(),          // merged from art.ini later
            add_occupy: Vec::new(),    // merged from art.ini later
            remove_occupy: Vec::new(), // merged from art.ini later
            unloading_class: section.get("UnloadingClass").map(|s| s.to_string()),
            ammo: section.get_i32("Ammo").unwrap_or(-1),

            // Slave Miner / economy fields
            enslaves: section.get("Enslaves").map(|s| s.to_string()),
            slaves_number: section.get_i32("SlavesNumber").unwrap_or(0),
            slave_regen_rate: section.get_i32("SlaveRegenRate").unwrap_or(0).max(0) as u32,
            slave_reload_rate: section.get_i32("SlaveReloadRate").unwrap_or(0).max(0) as u32,
            slaved: section.get_bool("Slaved").unwrap_or(false),
            fearless: section.get_bool("Fearless").unwrap_or(false),
            fraidycat: section.get_bool("Fraidycat").unwrap_or(false),
            crawls: false,
            fire_up_frame: 0,
            fire_prone_frame: 0,
            secondary_fire_frame: 0,
            secondary_prone_frame: 0,
            veteran_fearless: ability_list_has(section.get_list("VeteranAbilities"), "FEARLESS"),
            elite_fearless: ability_list_has(section.get_list("EliteAbilities"), "FEARLESS"),
            harvest_rate: section.get_i32("HarvestRate").unwrap_or(0).max(0) as u32,
            resource_gatherer: section.get_bool("ResourceGatherer").unwrap_or(false),
            resource_destination: section.get_bool("ResourceDestination").unwrap_or(false),
            ore_purifier: section.get_bool("OrePurifier").unwrap_or(false),

            // Locomotor / movement fields
            locomotor: section
                .get("Locomotor")
                .map(LocomotorKind::from_clsid)
                .unwrap_or_else(|| LocomotorKind::default_for_category(category)),
            speed_type: section
                .get("SpeedType")
                .map(SpeedType::from_ini)
                .unwrap_or_default(),
            movement_zone: section
                .get("MovementZone")
                .map(MovementZone::from_ini)
                .unwrap_or_default(),
            considered_aircraft: section.get_bool("ConsideredAircraft").unwrap_or(false),
            zfudge_bridge: section.get_i32("ZFudgeBridge").unwrap_or(7),
            too_big_to_fit_under_bridge: section
                .get_bool("TooBigToFitUnderBridge")
                .unwrap_or(false),
            crashable: section.get_bool("Crashable").unwrap_or(false),
            teleporter: section.get_bool("Teleporter").unwrap_or(false),
            hover_attack: section.get_bool("HoverAttack").unwrap_or(false),
            balloon_hover: section.get_bool("BalloonHover").unwrap_or(false),
            airport_bound: section.get_bool("AirportBound").unwrap_or(false),
            fighter: section.get_bool("Fighter").unwrap_or(false),
            fly_by: section.get_bool("FlyBy").unwrap_or(false),
            fly_back: section.get_bool("FlyBack").unwrap_or(false),
            landable: section.get_bool("Landable").unwrap_or(false),
            jumpjet: section.get_bool("JumpJet").unwrap_or(false),
            jumpjet_params: if section.get_bool("JumpJet").unwrap_or(false) {
                Some(JumpjetParams::from_ini_section(section))
            } else {
                None
            },

            // Crush properties -- default false for all types.
            crushable: section.get_bool("Crushable").unwrap_or(false),
            deployed_crushable: section.get_bool("DeployedCrushable").unwrap_or(true),
            crusher: section.get_bool("Crusher").unwrap_or(false),
            no_force_shield: section.get_bool("NoForceShield").unwrap_or(false),
            omni_crusher: section.get_bool("OmniCrusher").unwrap_or(false),
            omni_crush_resistant: section.get_bool("OmniCrushResistant").unwrap_or(false),

            deploys_into: section.get("DeploysInto").map(|s| s.to_string()),
            undeploys_into: section.get("UndeploysInto").map(|s| s.to_string()),
            deploy_facing: section
                .get_i32("DeployFacing")
                .map(|v| (v.clamp(0, 7) as u8) << 5)
                .unwrap_or(0x80),
            construction_yard: section.get_bool("ConstructionYard").unwrap_or(false),
            factory: section.get("Factory").and_then(FactoryType::from_ini),
            cloning: section.get_bool("Cloning").unwrap_or(false),
            exit_coord: parse_exit_coord(section.get("ExitCoord")),

            // Cursor / interaction capability flags
            engineer: section.get_bool("Engineer").unwrap_or(false),
            deployer: section.get_bool("Deployer").unwrap_or(false),
            capturable: section.get_bool("Capturable").unwrap_or(false),
            // Repairable defaults to true — most buildings can be repaired in RA2.
            repairable: section.get_bool("Repairable").unwrap_or(true),
            can_be_occupied: section.get_bool("CanBeOccupied").unwrap_or(false),
            can_occupy_fire: section.get_bool("CanOccupyFire").unwrap_or(false),
            show_occupant_pips: section.get_bool("ShowOccupantPips").unwrap_or(true),
            bridge_repair_hut: section.get_bool("BridgeRepairHut").unwrap_or(false),
            passengers: section.get_i32("Passengers").unwrap_or(0).max(0) as u32,
            size_limit: section.get_i32("SizeLimit").unwrap_or(0).max(0) as u32,
            size: section
                .get_i32("Size")
                .unwrap_or(if category == ObjectCategory::Infantry {
                    1
                } else {
                    3
                })
                .max(0) as u32,
            open_topped: section.get_bool("OpenTopped").unwrap_or(false),
            gunner: section.get_bool("Gunner").unwrap_or(false),
            ifv_mode: section.get_i32("IFVMode").unwrap_or(0).max(0) as u32,
            open_transport_weapon: section.get_i32("OpenTransportWeapon").unwrap_or(-1),
            deploy_fire: section.get_bool("DeployFire").unwrap_or(false),
            deploy_fire_weapon: section.get_i32("DeployFireWeapon"),
            max_number_occupants: section.get_i32("MaxNumberOccupants").unwrap_or(0).max(0) as u32,
            occupier: section.get_bool("Occupier").unwrap_or(false),
            assaulter: section.get_bool("Assaulter").unwrap_or(false),
            occupy_weapon: section.get("OccupyWeapon").map(|s| s.to_string()),
            elite_occupy_weapon: section.get("EliteOccupyWeapon").map(|s| s.to_string()),
            occupy_pip: section
                .get("OccupyPip")
                .map(|s| match s.to_ascii_lowercase().as_str() {
                    "persongreen" => 7,
                    "personyellow" => 8,
                    "personwhite" => 9,
                    "personred" => 10,
                    "personblue" => 11,
                    "personpurple" => 12,
                    _ => 7,
                })
                .unwrap_or(7),
            pip_scale: section
                .get("PipScale")
                .map(|s| PipScale::from_ini(s))
                .unwrap_or_default(),
            infantry_absorb: section.get_bool("InfantryAbsorb").unwrap_or(false),
            unit_absorb: section.get_bool("UnitAbsorb").unwrap_or(false),
            bunkerable: section
                .get_bool("Bunkerable")
                .unwrap_or(category == ObjectCategory::Vehicle),
            weapon_list: if section.get_bool("Gunner").unwrap_or(false) {
                (1..=17)
                    .filter_map(|i| section.get(&format!("Weapon{}", i)).map(|s| s.to_string()))
                    .collect()
            } else {
                Vec::new()
            },
            attack_cursor_on_friendlies: section
                .get_bool("AttackCursorOnFriendlies")
                .unwrap_or(false),
            sabotage_cursor: section.get_bool("SabotageCursor").unwrap_or(false),
            c4: section.get_bool("C4").unwrap_or(false),
            can_c4: section
                .get_bool("CanC4")
                .unwrap_or(category == ObjectCategory::Building),
            invisible: section.get_bool("Invisible").unwrap_or(false),
            invisible_in_game: section.get_bool("InvisibleInGame").unwrap_or(false),
            unit_repair: section.get_bool("UnitRepair").unwrap_or(false),
            bunker: section.get_bool("Bunker").unwrap_or(false),
            unit_reload: section.get_bool("UnitReload").unwrap_or(false),
            helipad: section.get_bool("Helipad").unwrap_or(false),
            number_of_docks: section.get_i32("NumberOfDocks").unwrap_or(1).max(1) as u8,
            // TogglePower defaults to true for buildings, false for units.
            toggle_power: section
                .get_bool("TogglePower")
                .unwrap_or(category == ObjectCategory::Building),
            // Powered defaults to true for buildings — most deactivate during low power.
            powered: section
                .get_bool("Powered")
                .unwrap_or(category == ObjectCategory::Building),
            can_disguise: section.get_bool("CanDisguise").unwrap_or(false),
            wall: section.get_bool("Wall").unwrap_or(false),

            // Naval flags
            water_bound: {
                // WaterBound defaults to true if SpeedType is already Float.
                let default = section
                    .get("SpeedType")
                    .map_or(false, |s| s.eq_ignore_ascii_case("Float"));
                section.get_bool("WaterBound").unwrap_or(default)
            },
            naval: section.get_bool("Naval").unwrap_or(false),
            number_impassable_rows: section.get_i32("NumberImpassableRows").unwrap_or(-1),

            // Point light source fields
            light_visibility: section.get_i32("LightVisibility").unwrap_or(5000),
            light_intensity: section.get_light_f32("LightIntensity").unwrap_or(0.0),
            light_red_tint: section.get_light_f32("LightRedTint").unwrap_or(1.0),
            light_green_tint: section.get_light_f32("LightGreenTint").unwrap_or(1.0),
            light_blue_tint: section.get_light_f32("LightBlueTint").unwrap_or(1.0),

            // TechnoType particle effects
            natural_particle_system: section
                .get("NaturalParticleSystem")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            natural_particle_location: section
                .get("NaturalParticleLocation")
                .map(parse_ivec3_offset)
                .unwrap_or(IVec3::ZERO),
            refinery_smoke_particle_system: section
                .get("RefinerySmokeParticleSystem")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            damage_particle_systems: parse_csv_string_list(section.get("DamageParticleSystems")),
            destroy_particle_systems: parse_csv_string_list(section.get("DestroyParticleSystems")),
            damage_smoke_offset: section
                .get("DamageSmokeOffset")
                .map(parse_ivec3_offset)
                .unwrap_or(IVec3::ZERO),
            dam_smk_off_scrn_rel: section.get_bool("DamSmkOffScrnRel").unwrap_or(false),
            destroy_smoke_offset: section
                .get("DestroySmokeOffset")
                .map(parse_ivec3_offset)
                .unwrap_or(IVec3::ZERO),
            refinery_smoke_offsets: [
                section
                    .get("RefinerySmokeOffsetOne")
                    .map(parse_ivec3_offset)
                    .unwrap_or(IVec3::ZERO),
                section
                    .get("RefinerySmokeOffsetTwo")
                    .map(parse_ivec3_offset)
                    .unwrap_or(IVec3::ZERO),
                section
                    .get("RefinerySmokeOffsetThree")
                    .map(parse_ivec3_offset)
                    .unwrap_or(IVec3::ZERO),
                section
                    .get("RefinerySmokeOffsetFour")
                    .map(parse_ivec3_offset)
                    .unwrap_or(IVec3::ZERO),
            ],
            refinery_smoke_frames: section.get_i32("RefinerySmokeFrames").unwrap_or(0).max(0)
                as u16,
            gap_radius_in_cells: section
                .get_i32("GapRadiusInCells")
                .map(|n| n.clamp(0, u8::MAX as i32) as u8)
                .unwrap_or(0),
            super_gap_radius_in_cells: section
                .get_i32("SuperGapRadiusInCells")
                .map(|n| n.clamp(0, u8::MAX as i32) as u8)
                .unwrap_or(0),
            psychic_detection_radius: section
                .get_i32("PsychicDetectionRadius")
                .map(|n| n.clamp(0, u8::MAX as i32) as u8)
                .unwrap_or(0),
            sensor_array: section.get_bool("SensorArray").unwrap_or(false),
            sensors: section.get_bool("Sensors").unwrap_or(false),
            sensors_sight: section
                .get_i32("SensorsSight")
                .map(|n| n.clamp(0, u8::MAX as i32) as u8)
                .unwrap_or(0),
            cloak_generator: section.get_bool("CloakGenerator").unwrap_or(false),
        }
    }
}

/// Parse an `X,Y,Z` coordinate string into an `IVec3`. Missing or unparseable
/// components default to 0.
fn parse_ivec3_offset(raw: &str) -> IVec3 {
    let mut parts = raw.split(',').map(|s| s.trim());
    let x = parts
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let y = parts
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let z = parts
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    IVec3::new(x, y, z)
}

fn ability_list_has(list: Option<Vec<&str>>, needle: &str) -> bool {
    list.unwrap_or_default()
        .into_iter()
        .any(|ability| ability.eq_ignore_ascii_case(needle))
}

/// Parse a CSV string list, trimming each entry and dropping empties. Returns
/// an empty Vec for `None` or whitespace-only input.
fn parse_csv_string_list(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Parse ExitCoord=X,Y,Z from rules.ini. Values are in leptons (256 = 1 cell).
fn parse_exit_coord(value: Option<&str>) -> Option<(i32, i32, i32)> {
    let val = value?;
    let parts: Vec<&str> = val.split(',').collect();
    if parts.len() >= 2 {
        let x: i32 = parts[0].trim().parse().ok()?;
        let y: i32 = parts[1].trim().parse().ok()?;
        let z: i32 = parts
            .get(2)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        Some((x, y, z))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn test_parse_vehicle() {
        let ini: IniFile = IniFile::from_str(
            "[MTNK]\nName=Grizzly Battle Tank\nCost=700\nStrength=300\n\
             Armor=heavy\nSpeed=6\nSight=6\nTechLevel=2\n\
             Owner=Americans,Alliance\nRequiredHouses=Americans\n\
             Prerequisite=GAWEAP\nPrimary=105mm\n",
        );
        let section: &IniSection = ini.section("MTNK").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);

        assert_eq!(obj.id, "MTNK");
        assert_eq!(obj.category, ObjectCategory::Vehicle);
        assert_eq!(obj.name, Some("Grizzly Battle Tank".to_string()));
        assert_eq!(obj.cost, 700);
        assert_eq!(obj.strength, 300);
        assert_eq!(obj.armor, "heavy");
        assert_eq!(obj.speed, 6);
        assert_eq!(obj.tech_level, 2);
        assert!((obj.build_time_multiplier - 1.0).abs() < f32::EPSILON);
        assert_eq!(obj.owner, vec!["Americans", "Alliance"]);
        assert_eq!(obj.required_houses, vec!["Americans"]);
        assert!(obj.allowed_to_start_in_multiplayer);
        assert_eq!(obj.prerequisite, vec!["GAWEAP"]);
        assert_eq!(obj.primary, Some("105mm".to_string()));
        assert_eq!(obj.secondary, None);
        assert_eq!(obj.image, "MTNK"); // Defaults to ID when Image= absent.
        assert_eq!(obj.build_cat, None);
        assert_eq!(obj.adjacent, 3);
        assert!(obj.base_normal);
        assert!(!obj.eligibile_for_ally_building);
        assert!(!obj.crewed);
    }

    #[test]
    fn allowed_to_start_in_multiplayer_parses_and_defaults_yes() {
        let enabled_ini = IniFile::from_str("[MTNK]\nAllowedToStartInMultiplayer=yes\n");
        let enabled = ObjectType::from_ini_section(
            "MTNK",
            enabled_ini.section("MTNK").unwrap(),
            ObjectCategory::Vehicle,
        );
        assert!(enabled.allowed_to_start_in_multiplayer);

        let disabled_ini = IniFile::from_str("[HTNK]\nAllowedToStartInMultiplayer=no\n");
        let disabled = ObjectType::from_ini_section(
            "HTNK",
            disabled_ini.section("HTNK").unwrap(),
            ObjectCategory::Vehicle,
        );
        assert!(!disabled.allowed_to_start_in_multiplayer);

        let default_ini = IniFile::from_str("[E1]\n");
        let defaulted = ObjectType::from_ini_section(
            "E1",
            default_ini.section("E1").unwrap(),
            ObjectCategory::Infantry,
        );
        assert!(defaulted.allowed_to_start_in_multiplayer);
    }

    #[test]
    fn test_parse_building() {
        let ini: IniFile = IniFile::from_str(
            "[GAPOWR]\nName=Power Plant\nCost=800\nStrength=750\n\
             Power=200\nFoundation=2x2\nArmor=wood\nBuildCat=Power\nCrewed=yes\n",
        );
        let section: &IniSection = ini.section("GAPOWR").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GAPOWR", section, ObjectCategory::Building);

        assert_eq!(obj.power, 200);
        assert_eq!(obj.foundation, "2x2");
        assert_eq!(obj.armor, "wood");
        assert_eq!(obj.build_cat, Some(BuildCategory::Power));
        assert_eq!(obj.adjacent, 3);
        assert!(obj.base_normal);
        assert!(!obj.eligibile_for_ally_building);
        assert!(obj.crewed);
    }

    #[test]
    fn test_light_visibility_defaults_to_5000_without_intensity() {
        let ini: IniFile = IniFile::from_str("[GALITE]\n");
        let section: &IniSection = ini.section("GALITE").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GALITE", section, ObjectCategory::Building);

        assert_eq!(obj.light_visibility, 5000);
        assert_eq!(obj.light_intensity, 0.0);
    }

    #[test]
    fn test_light_fields_use_light_float_parser() {
        let ini: IniFile = IniFile::from_str(
            "[GALITE]\n\
             LightVisibility=4096\n\
             LightIntensity=0.75\n\
             LightRedTint=-0.25\n\
             LightGreenTint=0,01\n\
             LightBlueTint=1.5\n",
        );
        let section: &IniSection = ini.section("GALITE").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GALITE", section, ObjectCategory::Building);

        assert_eq!(obj.light_visibility, 4096);
        assert!((obj.light_intensity - 0.75).abs() < 0.001);
        assert!((obj.light_red_tint + 0.25).abs() < 0.001);
        assert_eq!(obj.light_green_tint, 0.0);
        assert!((obj.light_blue_tint - 1.5).abs() < 0.001);
    }

    #[test]
    fn parse_bridge_repair_hut_flag() {
        let ini: IniFile = IniFile::from_str("[CABHUT]\nBridgeRepairHut=yes\n[NACABH]\n");
        let obj_on: ObjectType = ObjectType::from_ini_section(
            "CABHUT",
            ini.section("CABHUT").unwrap(),
            ObjectCategory::Building,
        );
        let obj_off: ObjectType = ObjectType::from_ini_section(
            "NACABH",
            ini.section("NACABH").unwrap(),
            ObjectCategory::Building,
        );
        assert!(obj_on.bridge_repair_hut);
        assert!(!obj_off.bridge_repair_hut);
    }

    #[test]
    fn parse_no_force_shield_flag() {
        let ini: IniFile = IniFile::from_str("[TST1]\nNoForceShield=yes\n[TST2]\n");
        let obj_on: ObjectType = ObjectType::from_ini_section(
            "TST1",
            ini.section("TST1").unwrap(),
            ObjectCategory::Building,
        );
        let obj_off: ObjectType = ObjectType::from_ini_section(
            "TST2",
            ini.section("TST2").unwrap(),
            ObjectCategory::Building,
        );
        assert!(obj_on.no_force_shield);
        assert!(!obj_off.no_force_shield);
    }

    #[test]
    fn test_defaults_for_missing_keys() {
        let ini: IniFile = IniFile::from_str("[BARE]\n");
        let section: &IniSection = ini.section("BARE").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("BARE", section, ObjectCategory::Infantry);

        assert_eq!(obj.cost, 0);
        assert_eq!(obj.strength, 0);
        assert_eq!(obj.armor, "none");
        assert_eq!(obj.speed, 0);
        assert_eq!(obj.sight, 0);
        assert_eq!(obj.tech_level, -1);
        assert!((obj.build_time_multiplier - 1.0).abs() < f32::EPSILON);
        assert!(obj.owner.is_empty());
        assert!(obj.required_houses.is_empty());
        assert!(obj.prerequisite.is_empty());
        assert_eq!(obj.primary, None);
        assert_eq!(obj.image, "BARE");
        assert_eq!(obj.power, 0);
        assert_eq!(obj.foundation, "1x1");
        assert_eq!(obj.build_cat, None);
        assert_eq!(obj.adjacent, 3);
        assert!(obj.base_normal);
        assert!(!obj.eligibile_for_ally_building);
        assert!(!obj.crewed);
    }

    #[test]
    fn test_parse_build_time_multiplier() {
        let ini: IniFile = IniFile::from_str("[HTNK]\nBuildTimeMultiplier=1.3\n");
        let section: &IniSection = ini.section("HTNK").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("HTNK", section, ObjectCategory::Vehicle);

        assert!((obj.build_time_multiplier - 1.3).abs() < 0.0001);
    }

    #[test]
    fn test_parse_building_placement_flags() {
        let ini: IniFile = IniFile::from_str(
            "[GAGAP]\nFoundation=2x2\nAdjacent=0\nBaseNormal=no\nEligibileForAllyBuilding=yes\n",
        );
        let section: &IniSection = ini.section("GAGAP").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GAGAP", section, ObjectCategory::Building);

        assert_eq!(obj.adjacent, 0);
        assert!(!obj.base_normal);
        assert!(obj.eligibile_for_ally_building);
    }

    #[test]
    fn test_parse_bridge_render_flags() {
        let ini: IniFile =
            IniFile::from_str("[DEST]\nZFudgeBridge=11\nTooBigToFitUnderBridge=yes\n");
        let section: &IniSection = ini.section("DEST").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("DEST", section, ObjectCategory::Vehicle);
        assert_eq!(obj.zfudge_bridge, 11);
        assert!(obj.too_big_to_fit_under_bridge);
    }

    #[test]
    fn test_bridge_render_flags_default() {
        let ini: IniFile = IniFile::from_str("[BOAT]\n");
        let section: &IniSection = ini.section("BOAT").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("BOAT", section, ObjectCategory::Vehicle);
        assert_eq!(obj.zfudge_bridge, 7);
        assert!(!obj.too_big_to_fit_under_bridge);
    }

    #[test]
    fn test_parse_deploys_into() {
        let ini: IniFile = IniFile::from_str("[AMCV]\nDeploysInto=GACNST\nSpeed=4\n");
        let section: &IniSection = ini.section("AMCV").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("AMCV", section, ObjectCategory::Vehicle);
        assert_eq!(obj.deploys_into, Some("GACNST".to_string()));
        assert_eq!(obj.undeploys_into, None);
    }

    #[test]
    fn test_parse_undeploys_into() {
        let ini: IniFile = IniFile::from_str("[GACNST]\nUndeploysInto=AMCV\nFoundation=4x4\n");
        let section: &IniSection = ini.section("GACNST").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GACNST", section, ObjectCategory::Building);
        assert_eq!(obj.undeploys_into, Some("AMCV".to_string()));
        assert_eq!(obj.deploys_into, None);
    }

    #[test]
    fn parse_construction_yard_and_deploy_facing() {
        let ini: IniFile = IniFile::from_str("[GACNST]\nConstructionYard=yes\nDeployFacing=2\n");
        let section: &IniSection = ini.section("GACNST").unwrap();
        let obj = ObjectType::from_ini_section("GACNST", section, ObjectCategory::Building);
        assert!(obj.construction_yard);
        assert_eq!(obj.deploy_facing, 0x40);

        let default_ini = IniFile::from_str("[GAPOWR]\n");
        let default_obj = ObjectType::from_ini_section(
            "GAPOWR",
            default_ini.section("GAPOWR").unwrap(),
            ObjectCategory::Building,
        );
        assert!(!default_obj.construction_yard);
        assert_eq!(default_obj.deploy_facing, 0x80);
    }

    #[test]
    fn test_parse_slave_miner_fields() {
        let ini: IniFile = IniFile::from_str(
            "[SMIN]\nEnslaves=SLAV\nSlavesNumber=5\nSlaveRegenRate=500\n\
             SlaveReloadRate=25\nResourceGatherer=yes\nResourceDestination=yes\n\
             DeploysInto=YAREFN\nStorage=20\nSpeed=3\n",
        );
        let section: &IniSection = ini.section("SMIN").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("SMIN", section, ObjectCategory::Vehicle);
        assert_eq!(obj.enslaves, Some("SLAV".to_string()));
        assert_eq!(obj.slaves_number, 5);
        assert_eq!(obj.slave_regen_rate, 500);
        assert_eq!(obj.slave_reload_rate, 25);
        assert!(obj.resource_gatherer);
        assert!(obj.resource_destination);
        assert_eq!(obj.deploys_into, Some("YAREFN".to_string()));
        assert!(!obj.slaved); // SMIN is master, not slave
    }

    #[test]
    fn test_parse_slave_infantry_fields() {
        let ini: IniFile = IniFile::from_str("[SLAV]\nSlaved=yes\nStorage=4\nHarvestRate=150\n");
        let section: &IniSection = ini.section("SLAV").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("SLAV", section, ObjectCategory::Infantry);
        assert!(obj.slaved);
        assert_eq!(obj.storage, 4);
        assert_eq!(obj.harvest_rate, 150);
        assert_eq!(obj.enslaves, None);
    }

    #[test]
    fn test_parse_ore_purifier() {
        let ini: IniFile = IniFile::from_str("[GAPROC]\nOrePurifier=yes\n");
        let section: &IniSection = ini.section("GAPROC").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("GAPROC", section, ObjectCategory::Building);
        assert!(obj.ore_purifier);
    }

    #[test]
    fn test_parse_refinery_free_unit_and_dock() {
        let ini: IniFile = IniFile::from_str(
            "[MODPROC]\nRefinery=yes\nFreeUnit=MODHARV\n\
             [MODHARV]\nHarvester=yes\nDock=modproc,NAREFN\n",
        );

        let refinery = ObjectType::from_ini_section(
            "MODPROC",
            ini.section("MODPROC").expect("MODPROC section"),
            ObjectCategory::Building,
        );
        assert!(refinery.refinery);
        assert_eq!(refinery.free_unit, Some("MODHARV".to_string()));

        let harvester = ObjectType::from_ini_section(
            "MODHARV",
            ini.section("MODHARV").expect("MODHARV section"),
            ObjectCategory::Vehicle,
        );
        assert!(harvester.harvester);
        assert_eq!(
            harvester.dock,
            vec!["MODPROC".to_string(), "NAREFN".to_string()]
        );
    }

    #[test]
    fn stock_cmin_rot_remains_5_after_harvester_parse() {
        let ini: IniFile = IniFile::from_str("[CMIN]\nHarvester=yes\nROT=5\n");
        let section: &IniSection = ini.section("CMIN").unwrap();
        let obj: ObjectType =
            ObjectType::from_ini_section("CMIN", section, ObjectCategory::Vehicle);

        assert!(obj.harvester);
        assert_eq!(obj.turret_rot, 5);
    }

    #[test]
    fn cloning_key_parses_and_participates_in_rally_lines() {
        let ini = IniFile::from_str("[YACLON]\nName=Cloning Vats\nStrength=1000\nCloning=yes\n");
        let section = ini.section("YACLON").unwrap();
        let obj = ObjectType::from_ini_section("YACLON", section, ObjectCategory::Building);
        assert!(obj.cloning);
        assert!(obj.has_rally_line());
    }

    #[test]
    fn rally_line_accepts_infantry_vehicle_factories_and_repair() {
        let ini = IniFile::from_str(
            "[GAPILE]\nFactory=InfantryType\n\
             [GAWEAP]\nFactory=UnitType\n\
             [GADEPT]\nUnitRepair=yes\n",
        );
        let barracks = ObjectType::from_ini_section(
            "GAPILE",
            ini.section("GAPILE").unwrap(),
            ObjectCategory::Building,
        );
        let factory = ObjectType::from_ini_section(
            "GAWEAP",
            ini.section("GAWEAP").unwrap(),
            ObjectCategory::Building,
        );
        let depot = ObjectType::from_ini_section(
            "GADEPT",
            ini.section("GADEPT").unwrap(),
            ObjectCategory::Building,
        );
        assert!(barracks.has_rally_line());
        assert!(factory.has_rally_line());
        assert!(depot.has_rally_line());
    }

    #[test]
    fn test_parse_transport_fields() {
        let ini: IniFile = IniFile::from_str(
            "[HTK]\nPassengers=5\nSizeLimit=2\nOpenTopped=no\nGunner=no\nSize=3\n",
        );
        let section: &IniSection = ini.section("HTK").unwrap();
        let obj = ObjectType::from_ini_section("HTK", section, ObjectCategory::Vehicle);
        assert_eq!(obj.passengers, 5);
        assert_eq!(obj.size_limit, 2);
        assert_eq!(obj.size, 3);
        assert!(!obj.open_topped);
        assert!(!obj.gunner);
        assert!(obj.weapon_list.is_empty());
    }

    #[test]
    fn test_parse_ifv_gunner_fields() {
        let ini: IniFile = IniFile::from_str(
            "[FV]\nPassengers=1\nSizeLimit=1\nGunner=yes\nSize=3\n\
             Weapon1=Missiles\nWeapon2=FlakGun\nWeapon3=RepairArm\n",
        );
        let section: &IniSection = ini.section("FV").unwrap();
        let obj = ObjectType::from_ini_section("FV", section, ObjectCategory::Vehicle);
        assert_eq!(obj.passengers, 1);
        assert_eq!(obj.size_limit, 1);
        assert!(obj.gunner);
        assert_eq!(obj.weapon_list, vec!["Missiles", "FlakGun", "RepairArm"]);
    }

    #[test]
    fn test_parse_infantry_ifv_mode() {
        let ini: IniFile = IniFile::from_str("[E1]\nIFVMode=0\nSize=1\n");
        let section: &IniSection = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert_eq!(obj.ifv_mode, 0);
        assert_eq!(obj.size, 1);
    }

    #[test]
    fn test_parse_garrison_building() {
        let ini: IniFile =
            IniFile::from_str("[GAPOST]\nCanBeOccupied=yes\nMaxNumberOccupants=10\n");
        let section: &IniSection = ini.section("GAPOST").unwrap();
        let obj = ObjectType::from_ini_section("GAPOST", section, ObjectCategory::Building);
        assert!(obj.can_be_occupied);
        assert_eq!(obj.max_number_occupants, 10);
    }

    #[test]
    fn test_parses_elite_weapon_overrides() {
        let ini: IniFile = IniFile::from_str(
            "[GGI]\nPrimary=M60\nSecondary=MissileLauncher\nElitePrimary=M60E\nEliteSecondary=MissileLauncherE\n",
        );
        let section: &IniSection = ini.section("GGI").unwrap();
        let obj = ObjectType::from_ini_section("GGI", section, ObjectCategory::Infantry);
        assert_eq!(obj.primary.as_deref(), Some("M60"));
        assert_eq!(obj.secondary.as_deref(), Some("MissileLauncher"));
        assert_eq!(obj.elite_primary.as_deref(), Some("M60E"));
        assert_eq!(obj.elite_secondary.as_deref(), Some("MissileLauncherE"));
    }

    #[test]
    fn test_elite_weapons_default_to_none() {
        let ini: IniFile = IniFile::from_str("[E1]\nPrimary=M60\n");
        let section: &IniSection = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert_eq!(obj.elite_primary, None);
        assert_eq!(obj.elite_secondary, None);
    }

    #[test]
    fn test_parses_open_transport_weapon() {
        let ini: IniFile = IniFile::from_str("[GGI]\nOpenTransportWeapon=1\n");
        let section: &IniSection = ini.section("GGI").unwrap();
        let obj = ObjectType::from_ini_section("GGI", section, ObjectCategory::Infantry);
        assert_eq!(obj.open_transport_weapon, 1);
    }

    #[test]
    fn test_open_transport_weapon_defaults_to_neg_one() {
        let ini: IniFile = IniFile::from_str("[E1]\nPrimary=M60\n");
        let section: &IniSection = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert_eq!(obj.open_transport_weapon, -1);
    }

    #[test]
    fn test_parse_absorb_fields() {
        let ini: IniFile = IniFile::from_str("[YABRCK]\nInfantryAbsorb=yes\nUnitAbsorb=no\n");
        let section: &IniSection = ini.section("YABRCK").unwrap();
        let obj = ObjectType::from_ini_section("YABRCK", section, ObjectCategory::Building);
        assert!(obj.infantry_absorb);
        assert!(!obj.unit_absorb);
    }

    #[test]
    fn test_parse_extra_power_positive() {
        let ini: IniFile =
            IniFile::from_str("[YAPOWR]\nPower=150\nExtraPower=100\nInfantryAbsorb=yes\n");
        let section: &IniSection = ini.section("YAPOWR").unwrap();
        let obj = ObjectType::from_ini_section("YAPOWR", section, ObjectCategory::Building);
        assert_eq!(obj.power, 150);
        assert_eq!(obj.extra_power, 100);
        assert!(obj.infantry_absorb);
    }

    #[test]
    fn test_parse_extra_power_negative() {
        // GAOREP (Allied Ore Processor) in rules.ini has ExtraPower=-9000.
        // Signed parse; downstream gate will suppress the bonus.
        let ini: IniFile = IniFile::from_str("[GAOREP]\nExtraPower=-9000\n");
        let section: &IniSection = ini.section("GAOREP").unwrap();
        let obj = ObjectType::from_ini_section("GAOREP", section, ObjectCategory::Building);
        assert_eq!(obj.extra_power, -9000);
    }

    #[test]
    fn test_parse_extra_power_default_zero() {
        let ini: IniFile = IniFile::from_str("[GAPOWR]\nPower=200\n");
        let section: &IniSection = ini.section("GAPOWR").unwrap();
        let obj = ObjectType::from_ini_section("GAPOWR", section, ObjectCategory::Building);
        assert_eq!(obj.extra_power, 0);
    }

    #[test]
    fn test_size_defaults_by_category() {
        // Infantry defaults to Size=1
        let ini: IniFile = IniFile::from_str("[INF]\n");
        let section: &IniSection = ini.section("INF").unwrap();
        let obj = ObjectType::from_ini_section("INF", section, ObjectCategory::Infantry);
        assert_eq!(obj.size, 1);

        // Vehicle defaults to Size=3
        let ini2: IniFile = IniFile::from_str("[VEH]\n");
        let section2: &IniSection = ini2.section("VEH").unwrap();
        let obj2 = ObjectType::from_ini_section("VEH", section2, ObjectCategory::Vehicle);
        assert_eq!(obj2.size, 3);
    }

    #[test]
    fn c4_flag_parses_from_ini() {
        let ini = IniFile::from_str("[GHOST]\nC4=yes\n");
        let section = ini.section("GHOST").unwrap();
        let obj = ObjectType::from_ini_section("GHOST", section, ObjectCategory::Infantry);
        assert!(obj.c4);
    }

    #[test]
    fn c4_defaults_to_false() {
        let ini = IniFile::from_str("[E1]\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(!obj.c4);
    }

    #[test]
    fn can_c4_defaults_to_true_for_buildings() {
        let ini = IniFile::from_str("[GAPILE]\n");
        let section = ini.section("GAPILE").unwrap();
        let obj = ObjectType::from_ini_section("GAPILE", section, ObjectCategory::Building);
        assert!(obj.can_c4);
    }

    #[test]
    fn can_c4_defaults_to_false_for_non_buildings() {
        let ini = IniFile::from_str("[E1]\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(!obj.can_c4);
    }

    #[test]
    fn can_c4_no_overrides_default() {
        let ini = IniFile::from_str("[CAMISC01]\nCanC4=no\n");
        let section = ini.section("CAMISC01").unwrap();
        let obj = ObjectType::from_ini_section("CAMISC01", section, ObjectCategory::Building);
        assert!(!obj.can_c4);
    }

    #[test]
    fn bunker_flag_parses_from_ini() {
        let ini = IniFile::from_str("[NATBNK]\nBunker=yes\nNumberImpassableRows=0\n");
        let section = ini.section("NATBNK").unwrap();
        let obj = ObjectType::from_ini_section("NATBNK", section, ObjectCategory::Building);
        assert!(obj.bunker);
        assert_eq!(obj.number_impassable_rows, 0);
    }

    #[test]
    fn gate_flag_parses_from_ini() {
        let ini = IniFile::from_str(
            "[GAGATE_A]\nGate=yes\nFoundation=3x1\nDeployTime=.044\nGateCloseDelay=.2\n",
        );
        let section = ini.section("GAGATE_A").unwrap();
        let obj = ObjectType::from_ini_section("GAGATE_A", section, ObjectCategory::Building);
        assert!(obj.gate);
        assert_eq!(obj.deploy_time_ticks, 39);
        assert_eq!(obj.gate_close_delay_ticks, 180);
    }

    #[test]
    fn bunkerable_defaults_true_for_vehicles_only() {
        let ini = IniFile::from_str("[TEST]\n");
        let section = ini.section("TEST").unwrap();

        let vehicle = ObjectType::from_ini_section("TEST", section, ObjectCategory::Vehicle);
        let infantry = ObjectType::from_ini_section("TEST", section, ObjectCategory::Infantry);
        let aircraft = ObjectType::from_ini_section("TEST", section, ObjectCategory::Aircraft);
        let building = ObjectType::from_ini_section("TEST", section, ObjectCategory::Building);

        assert!(vehicle.bunkerable);
        assert!(!infantry.bunkerable);
        assert!(!aircraft.bunkerable);
        assert!(!building.bunkerable);
    }

    #[test]
    fn bunkerable_ini_overrides_vehicle_default() {
        let ini = IniFile::from_str("[HTNK]\nBunkerable=no\n");
        let section = ini.section("HTNK").unwrap();
        let obj = ObjectType::from_ini_section("HTNK", section, ObjectCategory::Vehicle);
        assert!(!obj.bunkerable);
    }

    #[test]
    fn invisible_in_game_defaults_to_false() {
        let ini = IniFile::from_str("[GAPILE]\n");
        let section = ini.section("GAPILE").unwrap();
        let obj = ObjectType::from_ini_section("GAPILE", section, ObjectCategory::Building);
        assert!(!obj.invisible);
        assert!(!obj.invisible_in_game);
    }

    #[test]
    fn invisible_and_invisible_in_game_parse_independently() {
        let ini = IniFile::from_str(
            "[BRIDGEA]\nInvisible=yes\nInvisibleInGame=no\n\
             [BRIDGEB]\nInvisible=no\nInvisibleInGame=yes\n",
        );

        let plain = ObjectType::from_ini_section(
            "BRIDGEA",
            ini.section("BRIDGEA").unwrap(),
            ObjectCategory::Building,
        );
        let in_game = ObjectType::from_ini_section(
            "BRIDGEB",
            ini.section("BRIDGEB").unwrap(),
            ObjectCategory::Building,
        );

        assert!(plain.invisible);
        assert!(!plain.invisible_in_game);
        assert!(!in_game.invisible);
        assert!(in_game.invisible_in_game);
    }

    #[test]
    fn techno_type_particle_fields_default_to_empty() {
        let ini: IniFile = IniFile::from_str("[E1]\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);

        assert_eq!(obj.natural_particle_system, None);
        assert_eq!(obj.natural_particle_location, IVec3::ZERO);
        assert_eq!(obj.refinery_smoke_particle_system, None);
        assert!(obj.damage_particle_systems.is_empty());
        assert!(obj.destroy_particle_systems.is_empty());
        assert_eq!(obj.damage_smoke_offset, IVec3::ZERO);
        assert!(!obj.dam_smk_off_scrn_rel);
        assert_eq!(obj.destroy_smoke_offset, IVec3::ZERO);
        assert_eq!(obj.refinery_smoke_offsets, [IVec3::ZERO; 4]);
        assert_eq!(obj.gap_radius_in_cells, 0);
        assert_eq!(obj.super_gap_radius_in_cells, 0);
    }

    #[test]
    fn techno_type_parses_damage_particle_systems_csv() {
        let ini: IniFile = IniFile::from_str(
            "[GAPOWR]\n\
             DamageParticleSystems=BigGreySSys, SmallGreySSys ,SparkSys\n\
             DestroyParticleSystems=DebrisSmokeSys\n",
        );
        let section = ini.section("GAPOWR").unwrap();
        let obj = ObjectType::from_ini_section("GAPOWR", section, ObjectCategory::Building);

        assert_eq!(
            obj.damage_particle_systems,
            vec![
                "BigGreySSys".to_string(),
                "SmallGreySSys".to_string(),
                "SparkSys".to_string(),
            ]
        );
        assert_eq!(
            obj.destroy_particle_systems,
            vec!["DebrisSmokeSys".to_string()]
        );
    }

    #[test]
    fn techno_type_parses_refinery_smoke_offsets() {
        let ini: IniFile = IniFile::from_str(
            "[GAREFN]\n\
             RefinerySmokeParticleSystem=BigGreySSys\n\
             RefinerySmokeOffsetOne=10,20,30\n\
             RefinerySmokeOffsetTwo=-5,0,15\n\
             RefinerySmokeOffsetThree=0,0,0\n\
             RefinerySmokeOffsetFour=100,-50,200\n",
        );
        let section = ini.section("GAREFN").unwrap();
        let obj = ObjectType::from_ini_section("GAREFN", section, ObjectCategory::Building);

        assert_eq!(
            obj.refinery_smoke_particle_system.as_deref(),
            Some("BigGreySSys")
        );
        assert_eq!(obj.refinery_smoke_offsets[0], IVec3::new(10, 20, 30));
        assert_eq!(obj.refinery_smoke_offsets[1], IVec3::new(-5, 0, 15));
        assert_eq!(obj.refinery_smoke_offsets[2], IVec3::ZERO);
        assert_eq!(obj.refinery_smoke_offsets[3], IVec3::new(100, -50, 200));
    }

    #[test]
    fn techno_type_parses_refinery_smoke_frames() {
        let ini: IniFile = IniFile::from_str("[FOO]\nRefinerySmokeFrames=50\n");
        let section = ini.section("FOO").expect("section");
        let obj = ObjectType::from_ini_section("FOO", section, ObjectCategory::Building);
        assert_eq!(obj.refinery_smoke_frames, 50);
    }

    #[test]
    fn techno_type_refinery_smoke_frames_defaults_to_zero() {
        let ini: IniFile = IniFile::from_str("[FOO]\n");
        let section = ini.section("FOO").expect("section");
        let obj = ObjectType::from_ini_section("FOO", section, ObjectCategory::Building);
        assert_eq!(obj.refinery_smoke_frames, 0);
    }

    #[test]
    fn techno_type_parses_natural_and_smoke_keys() {
        let ini: IniFile = IniFile::from_str(
            "[GAGAP]\n\
             NaturalParticleSystem=GapPSys\n\
             NaturalParticleLocation=0,0,256\n\
             DamageSmokeOffset=12,-12,48\n\
             DamSmkOffScrnRel=yes\n\
             DestroySmokeOffset=0,0,128\n\
             GapRadiusInCells=8\n\
             SuperGapRadiusInCells=12\n\
             PsychicDetectionRadius=15\n\
             SensorArray=yes\n\
             Sensors=yes\n\
             SensorsSight=14\n\
             CloakGenerator=yes\n",
        );
        let section = ini.section("GAGAP").unwrap();
        let obj = ObjectType::from_ini_section("GAGAP", section, ObjectCategory::Building);

        assert_eq!(obj.natural_particle_system.as_deref(), Some("GapPSys"));
        assert_eq!(obj.natural_particle_location, IVec3::new(0, 0, 256));
        assert_eq!(obj.damage_smoke_offset, IVec3::new(12, -12, 48));
        assert!(obj.dam_smk_off_scrn_rel);
        assert_eq!(obj.destroy_smoke_offset, IVec3::new(0, 0, 128));
        assert_eq!(obj.gap_radius_in_cells, 8);
        assert_eq!(obj.super_gap_radius_in_cells, 12);
        assert_eq!(obj.psychic_detection_radius, 15);
        assert!(obj.sensor_array);
        assert!(obj.sensors);
        assert_eq!(obj.sensors_sight, 14);
        assert!(obj.cloak_generator);
    }

    #[test]
    fn parse_csv_string_list_drops_empties_and_trims() {
        assert!(parse_csv_string_list(None).is_empty());
        assert!(parse_csv_string_list(Some("")).is_empty());
        assert!(parse_csv_string_list(Some("  ,  ,  ")).is_empty());
        assert_eq!(
            parse_csv_string_list(Some(" A , B , ,C ")),
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );
    }

    #[test]
    fn parses_infantry_fear_flags_and_fearless_abilities() {
        let ini = IniFile::from_str(
            "[E1]\nFearless=yes\nFraidycat=yes\nVeteranAbilities=FEARLESS\nEliteAbilities=SELF_HEAL,FEARLESS\n",
        );
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(obj.fearless);
        assert!(obj.fraidycat);
        assert!(!obj.crawls);
        assert!(obj.veteran_fearless);
        assert!(obj.elite_fearless);
    }

    #[test]
    fn parses_deployed_crushable_with_default_yes() {
        let ini = IniFile::from_str("[E1]\nCrushable=yes\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(obj.deployed_crushable);

        let ini = IniFile::from_str("[GGI]\nCrushable=yes\nDeployedCrushable=no\n");
        let section = ini.section("GGI").unwrap();
        let obj = ObjectType::from_ini_section("GGI", section, ObjectCategory::Infantry);
        assert!(!obj.deployed_crushable);
    }

    #[test]
    fn object_type_parses_regular_crusher_for_amcv_fixture() {
        let ini = IniFile::from_str("[AMCV]\nSpeed=4\nMovementZone=Normal\nCrusher=yes\n");
        let section = ini.section("AMCV").unwrap();
        let obj = ObjectType::from_ini_section("AMCV", section, ObjectCategory::Vehicle);
        assert!(obj.crusher);
    }

    #[test]
    fn object_type_crusher_defaults_false() {
        let ini = IniFile::from_str("[MTNK]\nSpeed=7\nMovementZone=Normal\n");
        let section = ini.section("MTNK").unwrap();
        let obj = ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);
        assert!(!obj.crusher);
    }

    #[test]
    fn object_type_parses_accelerates_false() {
        let ini = IniFile::from_str("[MTNK]\nSpeed=7\nAccelerates=false\n");
        let section = ini.section("MTNK").unwrap();
        let obj = ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);
        assert!(!obj.accelerates);
    }

    #[test]
    fn object_type_accelerates_defaults_true() {
        let ini = IniFile::from_str("[AMCV]\nSpeed=4\n");
        let section = ini.section("AMCV").unwrap();
        let obj = ObjectType::from_ini_section("AMCV", section, ObjectCategory::Vehicle);
        assert!(obj.accelerates);
    }

    #[test]
    fn parses_air_range_bonus() {
        let ini: IniFile =
            IniFile::from_str("[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nAirRangeBonus=4\n");
        let section = ini.section("MTNK").expect("section");
        let obj = ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);
        assert_eq!(obj.air_range_bonus, Some(sim_from_f32(4.0)));
    }

    #[test]
    fn air_range_bonus_default_none() {
        let ini: IniFile = IniFile::from_str("[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n");
        let section = ini.section("MTNK").expect("section");
        let obj = ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);
        assert_eq!(obj.air_range_bonus, None);
    }

    #[test]
    fn parse_weight_default_two() {
        let ini: IniFile = IniFile::from_str("[MTNK]\nStrength=300\n");
        let section = ini.section("MTNK").expect("section");
        let obj = ObjectType::from_ini_section("MTNK", section, ObjectCategory::Vehicle);
        assert_eq!(obj.weight, SimFixed::lit("2.0"));
    }

    #[test]
    fn parse_weight_custom() {
        let ini: IniFile = IniFile::from_str("[HTNK]\nWeight=3.5\n");
        let section = ini.section("HTNK").expect("section");
        let obj = ObjectType::from_ini_section("HTNK", section, ObjectCategory::Vehicle);
        assert_eq!(obj.weight, SimFixed::lit("3.5"));
    }

    #[test]
    fn parse_retail_weight_apocalypse_is_three_point_five() {
        // Retail rulesmd.ini: APOC (Apocalypse Tank) has Weight=3.5.
        // The heaviest unit in stock retail is CARRIER (Aircraft Carrier) at Weight=5.
        let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
        let ini = IniFile::from_str(&ini_text);
        let apoc = ObjectType::from_ini_section(
            "APOC",
            ini.section("APOC").expect("APOC section"),
            ObjectCategory::Vehicle,
        );
        assert_eq!(apoc.weight, SimFixed::lit("3.5"));
    }

    #[test]
    fn parse_retail_weight_grizzly_defaults_to_two() {
        // Retail rulesmd.ini: MTNK (Grizzly) has no Weight= line, so it should
        // fall back to the engine default 2.0.
        let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
        let ini = IniFile::from_str(&ini_text);
        let mtnk = ObjectType::from_ini_section(
            "MTNK",
            ini.section("MTNK").expect("MTNK section"),
            ObjectCategory::Vehicle,
        );
        assert_eq!(mtnk.weight, SimFixed::lit("2.0"));
    }
}
