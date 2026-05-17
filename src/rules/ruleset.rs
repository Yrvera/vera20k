//! Master game data container loaded from rules.ini.
//!
//! RuleSet is the single source of truth for all game object definitions.
//! It parses the type registries ([InfantryTypes], [VehicleTypes], etc.),
//! then loads each referenced object's section into typed structs. Weapons
//! and warheads referenced by objects are also parsed.
//!
//! ## Loading strategy
//! 1. Parse type registries → collect all object IDs per category
//! 2. For each ID, look up its [ID] section → parse into ObjectType
//! 3. Collect weapon/warhead IDs referenced by all objects
//! 4. Parse each referenced weapon/warhead section
//! 5. Log summary counts
//!
//! ## Dependency rules
//! - Part of rules/ — depends on rules/ini_parser, rules/object_type,
//!   rules/weapon_type, rules/warhead_type.
//! - No dependencies on sim/, render/, ui/, etc.

use std::collections::{HashMap, HashSet};

use crate::rules::combat_damage::CombatDamageDefaults;
use crate::rules::error::RulesError;
use crate::rules::ini_parser::IniFile;
use crate::rules::object_type::{FactoryType, ObjectCategory, ObjectType};
use crate::rules::particle_system_type::{
    ParticleSystemType, ParticleSystemTypeId, PendingParticleSystemType,
};
use crate::rules::particle_type::{ParticleType, ParticleTypeId, PendingParticleType};
use crate::rules::projectile_type::ProjectileType;
use crate::rules::radar_event_config::RadarEventConfig;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::rules::superweapon_type::SuperWeaponType;
use crate::rules::terrain_object_type::TerrainObjectType;
use crate::rules::terrain_rules::TerrainRules;
use crate::rules::warhead_type::WarheadType;
use crate::rules::weapon_type::WeaponType;
use crate::util::fixed_math::{SimFixed, sim_from_f32};

/// Registry section names in rules.ini and their corresponding category.
const TYPE_REGISTRIES: &[(&str, ObjectCategory)] = &[
    ("InfantryTypes", ObjectCategory::Infantry),
    ("VehicleTypes", ObjectCategory::Vehicle),
    ("AircraftTypes", ObjectCategory::Aircraft),
    ("BuildingTypes", ObjectCategory::Building),
];

/// Production timing rules parsed from `[General]`.
///
/// The `_ppm` (parts-per-million) fields are pre-computed at INI parse time from the
/// corresponding f32 fields so that sim code can use pure integer arithmetic.
/// 1_000_000 = 1.0×. The f32 originals are kept for logging/debugging.
#[derive(Debug, Clone, Copy)]
pub struct ProductionRules {
    /// Minutes to build an object that costs 1000 credits before per-object modifiers.
    pub build_speed: f32,
    /// Time multiplier applied for each extra matching factory.
    pub multiple_factory: f32,
    /// Severity of the low-power speed penalty.
    pub low_power_penalty_modifier: f32,
    /// Lower bound on production speed while low power is active.
    pub min_low_power_production_speed: f32,
    /// Upper bound on production speed while low power is active.
    pub max_low_power_production_speed: f32,
    // -- Pre-computed integer-scaled values for deterministic sim math --
    /// `multiple_factory` scaled to PPM (e.g., 0.8 → 800_000).
    pub multiple_factory_ppm: u64,
    /// `low_power_penalty_modifier` scaled to PPM.
    pub low_power_penalty_modifier_ppm: u64,
    /// `min_low_power_production_speed` scaled to PPM.
    pub min_low_power_production_speed_ppm: u64,
    /// `max_low_power_production_speed` scaled to PPM.
    pub max_low_power_production_speed_ppm: u64,
    /// `build_speed` pre-scaled ×1000 for deterministic build-time computation.
    pub build_speed_x1000: u64,
    /// Speed coefficient applied to wall building production after all other
    /// queue time scaling. Parsed from `WallBuildSpeedCoefficient=` in [General].
    pub wall_build_speed_coefficient: f32,
}

/// PPM scale constant (1_000_000 = 1.0×) used for f32→integer conversion at parse time.
const PRODUCTION_PPM: u64 = 1_000_000;

/// Convert an f32 value clamped to `[min, ∞)` into PPM u64 at parse time only.
fn f32_to_ppm(val: f32, min: f32) -> u64 {
    (val.max(min) as f64 * PRODUCTION_PPM as f64) as u64
}

impl Default for ProductionRules {
    fn default() -> Self {
        Self {
            build_speed: 0.05,
            multiple_factory: 0.8,
            low_power_penalty_modifier: 1.0,
            min_low_power_production_speed: 0.5,
            max_low_power_production_speed: 0.9,
            multiple_factory_ppm: f32_to_ppm(0.8, 0.01),
            low_power_penalty_modifier_ppm: f32_to_ppm(1.0, 0.0),
            min_low_power_production_speed_ppm: f32_to_ppm(0.5, 0.0),
            max_low_power_production_speed_ppm: f32_to_ppm(0.9, 0.0),
            build_speed_x1000: (0.05f64 * 1000.0) as u64,
            wall_build_speed_coefficient: 1.0,
        }
    }
}

/// A world-effect animation reference parsed from rules.ini + art.ini.
///
/// The name comes from rules.ini `[General]` (e.g., WarpIn=WARPIN).
/// The rate comes from the anim's own art.ini section (e.g., `[WARPIN]` Rate=120).
#[derive(Debug, Clone)]
pub struct AnimRef {
    /// SHP animation name (uppercase), e.g., "WARPIN".
    pub name: String,
    /// Milliseconds per frame from art.ini `[ANIM_NAME]` Rate= key.
    pub rate_ms: u32,
}

/// Static art.ini metadata for the `[General] Parachute=` SHP.
///
/// Loaded once at startup from the art.ini section named by `[General] Parachute=`.
/// Pure render-side data: consumed by the parachute anim lifecycle and renderer.
#[derive(Debug, Clone)]
pub struct ParachuteRenderConfig {
    /// SHP section name from `[General] Parachute=` (e.g., "PARACH"). Uppercased.
    pub shp_name: String,
    /// ms per anim frame. Computed via `art_rate_to_delay_ms(Rate=)`.
    /// For Rate=400 this is 133.
    pub rate_ms: u32,
    /// Frame to wrap to after `frame >= end_frame`. From art.ini `LoopStart=`.
    pub loop_start: u16,
    /// Wraparound bound (exclusive). Set to `LoopEnd + 1` from art.ini.
    /// Frames `0..end_frame` play once on first cycle, then wrap to `loop_start`.
    pub end_frame: u16,
    /// Depth-sort offset (signed, leptons; -10 for PARACH). Used by the renderer
    /// to put the chute slightly above the GI body in the same depth band.
    pub z_adjust: i16,
    /// Whether to use the unit/Convert palette instead of the standard anim
    /// palette. From art.ini `AltPalette=`. NOT owner-tinted.
    pub alt_palette: bool,
}

/// Global gameplay constants from `[General]` that affect vision, gap generators, etc.
#[derive(Debug, Clone)]
pub struct GeneralRules {
    /// Additive sight bonus for veteran+ units (VeteranSight=).
    /// Default 0 in vanilla RA2 (no sight bonus from veterancy).
    pub veteran_sight: i32,
    /// Leptons of elevation per +1 sight cell (LeptonsPerSightIncrease=).
    /// 256 leptons = 1 z-level in RA2. 0 disables the elevation bonus.
    pub leptons_per_sight_increase: i32,
    /// Gap Generator effect radius in cells (GapRadius=). Default 10.
    pub gap_radius: i32,
    /// Height-based LOS obstruction (RevealByHeight= in [General]).
    /// When true, terrain 4+ levels above the viewer at the midpoint blocks sight.
    /// Default true (the standard RA2/YR setting).
    pub reveal_by_height: bool,
    /// How impassable cells behind ≥4-level cliffs are (CliffBackImpassability= in [General]).
    /// 0 = disabled, 2 = enabled (marks cells as Rock). Default 2 in standard YR.
    pub cliff_back_impassability: u8,
    /// Underground travel speed for Tunnel locomotor units (TunnelSpeed=).
    /// Default 6.0 cells/second matching RA2 default.
    pub tunnel_speed: SimFixed,
    /// `MissileROTVar=` from [General]. Amplitude of the sidewinder cosine
    /// modulation in homing missile flight; the per-tick ROT scales by
    /// `(1 + var) + cos(2π * frame / 15) * var`. Default 1.0 produces
    /// oscillation between 1.0 and 3.0 times the projectile's base ROT.
    pub missile_rot_var: SimFixed,
    /// Default cruise altitude for Fly-locomotor aircraft (FlightLevel= in [General]).
    /// Default 1500 leptons. Per-type override possible but not yet implemented.
    pub flight_level: i32,
    /// Descent rate cap for parachuted units, in leptons/tick (signed).
    /// Per gamemd, the rate field accumulates by `-1` per tick and clamps
    /// to this value. Default `-3` matches `[General] ParachuteMaxFallRate=-3`.
    /// Negative = falling.
    pub parachute_max_fall_rate: i32,
    /// Paradrop trigger radius in leptons. From `[General] ParadropRadius=`.
    /// Default 1024 (~4 cells). Distance to target at which the carrier aircraft
    /// reveals fog + transitions to the overfly mission.
    pub paradrop_radius: i32,
    /// Carrier aircraft type for paradrop missions. Default `PDPLANE`.
    pub paradrop_aircraft_type: String,
    /// Parsed `[General] Parachute=` value (uppercased SHP name, e.g. "PARACH").
    /// `None` if unset or empty. Used by `resolve_art_rates` to resolve
    /// `parachute_render`.
    pub parachute_shp: Option<String>,
    /// Parsed render config for the parachute SHP (from `[General] Parachute=`).
    /// `None` if the key is unset OR if the referenced art.ini section is
    /// missing. Render path is a no-op when this is `None`.
    pub parachute_render: Option<ParachuteRenderConfig>,
    /// American paradrop list: parallel `(infantry_type, count)` pairs.
    /// From `[General] AmerParaDropInf=` zipped with `AmerParaDropNum=`.
    /// Default `[("E1", 8)]`.
    pub amer_paradrop_list: Vec<(String, u32)>,
    /// Allied paradrop list. Default `[("E1", 6)]`.
    pub ally_paradrop_list: Vec<(String, u32)>,
    /// Soviet paradrop list. Default `[("E2", 9)]`. Per gamemd the dispatch
    /// case skips the count-equality assert on this branch only — preserved.
    pub sov_paradrop_list: Vec<(String, u32)>,
    /// Yuri paradrop list. Default `[("INIT", 6)]`.
    pub yuri_paradrop_list: Vec<(String, u32)>,
    /// Whether ore cells grow denser over time (TiberiumGrows= in [General]).
    /// Default true. Can be overridden per-map in [SpecialFlags].
    pub tiberium_grows: bool,
    /// Whether rich ore spreads to adjacent empty cells (TiberiumSpreads= in [General]).
    /// Default true. Can be overridden per-map in [SpecialFlags].
    pub tiberium_spreads: bool,
    /// Minutes per full map growth scan cycle (GrowthRate= in [General]).
    /// Default 5.0 minutes. Controls how fast ore regenerates.
    pub growth_rate_minutes: f32,
    /// Animation played when a unit warps in (WarpIn= in [General]).
    pub warp_in: AnimRef,
    /// Animation played when a unit warps out (WarpOut= in [General]).
    pub warp_out: AnimRef,
    /// Animation for chrono-erasing a unit (WarpAway= in [General]).
    pub warp_away: AnimRef,
    /// Sparkle particles during chrono teleport (ChronoSparkle1= in [General], YR feature).
    pub chrono_sparkle1: AnimRef,
    /// Wake animation spawned behind ships moving on water (Wake= in [General]).
    pub wake: AnimRef,
    /// Whether the attack cursor appears on a disguised Spy (AttackCursorOnDisguise= in [General]).
    /// Default false (vanilla RA2). When false, a disguised Spy does not show the attack cursor.
    pub attack_cursor_on_disguise: bool,
    /// Whether the attack cursor appears on trees/terrain (TreeTargeting= in [General]).
    /// Default false in vanilla RA2.
    pub tree_targeting: bool,
    /// Health ratio threshold below which the bar turns yellow (ConditionYellow= in [AudioVisual]).
    /// Default 0.5 (50%).
    pub condition_yellow: f32,
    /// `condition_yellow` pre-scaled to integer x1000 for deterministic sim comparisons.
    pub condition_yellow_x1000: i64,
    /// Health ratio threshold below which the bar turns red (ConditionRed= in [AudioVisual]).
    /// Default 0.25 (25%).
    pub condition_red: f32,
    /// `condition_red` pre-scaled to integer ×1000 for deterministic sim comparisons.
    /// Computed once at parse time: `(condition_red * 1000.0) as i64`.
    pub condition_red_x1000: i64,
    /// SFX played when the first occupant enters a CanBeOccupied building.
    /// Parsed from [AudioVisual] BuildingGarrisonedSound (typically "BuildingGarrisoned").
    /// None = no sound configured. Resolved at app layer to a sound.ini entry.
    pub building_garrisoned_sound: Option<String>,
    /// Sound event for shell main-menu buttons from [AudioVisual] GUIMainButtonSound.
    pub gui_main_button_sound: Option<String>,
    /// Direct rocker force coefficient (DirectRockingCoefficient= in [AudioVisual]).
    /// Multiplies the final DirectRocker impulse force. Default 1.5.
    pub direct_rocking_coefficient: SimFixed,
    /// Damping coefficient applied while a vehicle is moving (FallBackCoefficient=
    /// in [AudioVisual]). Multiplies the base 0.002 rad/tick decay rate; smaller
    /// values keep the body tilted longer between successive impulses. Default 0.1.
    pub fallback_coefficient: SimFixed,
    /// Fallback sound played at the arrival cell of a self-teleport when the
    /// per-unit `ChronoInSound=` is unset. Parsed from `[General] ChronoInSound=`
    /// (stock default `ChronoMinerTeleport`). `None` = no sound.
    pub chrono_in_sound: Option<String>,
    /// Fallback sound played at the departure cell of a self-teleport when the
    /// per-unit `ChronoOutSound=` is unset. Parsed from `[General] ChronoOutSound=`
    /// (stock default `ChronoMinerTeleport`). `None` = no sound.
    pub chrono_out_sound: Option<String>,
    /// Interval in minutes between low-power degradation damage ticks on Powered=yes buildings.
    /// Parsed from DamageDelay= in [General]. Default 1.0 minute.
    pub damage_delay_minutes: f32,
    /// Duration of spy-triggered total power blackout in game frames (15 fps).
    /// Parsed from SpyPowerBlackout= in [General]. Default 1000 frames (~67 seconds).
    pub spy_power_blackout_frames: u32,
    /// Fire/smoke anim types spawned on buildings below ConditionYellow health.
    /// Parsed from DamageFireTypes= in [General]. Default: FIRE01,FIRE02,FIRE03.
    pub damage_fire_types: Vec<AnimRef>,
    /// Particle system spawned by exploding barrels.
    /// Parsed from `BarrelParticle=` in `[General]` (NOT `[AudioVisual]`,
    /// despite the proximity to other AudioVisual keys).
    /// Holds the unresolved section name; ID resolution against the
    /// particle-system registry is deferred (matches A2/A3/A4/A5a pattern).
    pub barrel_particle: Option<String>,

    // -- Harvester scan radii and economy --
    /// Short-range ore scan radius in cells (TiberiumShortScan= in [General]).
    /// Used when harvesting a single patch — scan nearby for the next cell.
    /// Default 6 cells. YR only (RA2 hardcodes the same value).
    pub tiberium_short_scan: i32,
    /// Long-range ore scan radius in cells (TiberiumLongScan= in [General]).
    /// Used when short scan fails — look further for a new ore patch.
    /// Default 48 cells.
    pub tiberium_long_scan: i32,
    /// Slave Miner short scan distance in cells (SlaveMinerShortScan= in [General]).
    /// Deployed Slave Miner checks this range to decide if it should reposition.
    /// Default 8.
    pub slave_miner_short_scan: i32,
    /// Slave unit scan distance in cells (SlaveMinerSlaveScan= in [General]).
    /// Slaves scan further than their master since they trust it would reposition if needed.
    /// Default 14.
    pub slave_miner_slave_scan: i32,
    /// Slave Miner long scan distance in cells (SlaveMinerLongScan= in [General]).
    /// Used when searching for a new ore field to deploy near. Default 48.
    pub slave_miner_long_scan: i32,
    /// Cell improvement threshold for Slave Miner repositioning (SlaveMinerScanCorrection=).
    /// The new spot must be this many cells closer to ore to justify moving. Default 3.
    pub slave_miner_scan_correction: i32,
    /// Guard duration before deployed Slave Miner re-scans for ore (SlaveMinerKickFrameDelay=).
    /// In game frames (15 fps). Default 150 (~10 seconds).
    pub slave_miner_kick_frame_delay: u32,
    /// Standard harvester "too far" threshold in cells (HarvesterTooFarDistance=).
    /// If the nearest refinery is farther than this, the harvester drives next to it
    /// before reserving a dock. Default 5.
    pub harvester_too_far_distance: i32,
    /// Chrono harvester "too far" threshold in cells (ChronoHarvTooFarDistance=).
    /// Larger than standard because chrono miners teleport back. Default 50.
    pub chrono_harv_too_far_distance: i32,

    // -- Harvester timing --
    /// Frames per StepTimer increment during ore gathering (HarvesterLoadRate=).
    /// One bale requires 9 steps, so harvest_interval = rate * 9. Default 2.
    pub harvester_load_rate: i32,
    /// Tenths-of-a-frame per bale during refinery unloading (HarvesterDumpRate=).
    /// Pre-computed from INI double: `(rate * 9000.0).round() as u16`.
    /// Default 144 (from 0.016 × 9000 = 14.4 frames per bale, exact).
    /// The miner timer counts in tenths so the 0.4-frame fractional component
    /// at default rate is preserved instead of being truncated to 14.
    pub harvester_dump_tenths: u16,

    // -- Chrono warp delay constants --
    /// Post-warp lock duration in game frames (ChronoDelay= in [General]).
    /// Applied after Chronosphere warp. Default 60 frames.
    pub chrono_delay: i32,
    /// Chrono reinforcement warp delay in game frames (ChronoReinfDelay= in [General]).
    /// Default 180 frames.
    pub chrono_reinf_delay: i32,
    /// Distance divisor for warp delay: delay = distance_leptons / factor
    /// (ChronoDistanceFactor= in [General]). Default 48.
    pub chrono_distance_factor: i32,
    /// Whether warp delay scales with distance (ChronoTrigger= in [General]).
    /// If false, always use ChronoMinimumDelay. Default true.
    pub chrono_trigger: bool,
    /// Minimum warp delay in game frames (ChronoMinimumDelay= in [General]).
    /// Floor for the distance-based calculation. Default 16 frames.
    pub chrono_minimum_delay: i32,
    /// Distance (leptons) below which delay is forced to minimum
    /// (ChronoRangeMinimum= in [General]). Default 0.
    pub chrono_range_minimum: i32,

    /// Ore Purifier bonus as integer percentage (PurifierBonus= in [General]).
    /// Stored as `round(float_value * 100)`. 25 = 25% bonus. Default 25.
    pub purifier_bonus_pct: i32,
    /// AI virtual purifier counts indexed by difficulty
    /// (AIVirtualPurifiers= in [General]). Each entry is added to the AI
    /// player's real purifier count when computing the deposit bonus. INI
    /// convention is hardest-first, so the array is `[Brutal, Medium, Easy]`
    /// with the retail default `[4, 2, 0]`.
    pub ai_virtual_purifiers: [i32; 3],

    // -- Survivor spawning on sell/destroy --
    /// Divisor to compute survivor count for Allied buildings (AlliedSurvivorDivisor= in [General]).
    /// Survivor count = sell_refund / divisor (rounded down, min 0). Default 500.
    pub allied_survivor_divisor: i32,
    /// Divisor to compute survivor count for Soviet buildings (SovietSurvivorDivisor= in [General]).
    /// Default 250.
    pub soviet_survivor_divisor: i32,
    /// Divisor to compute survivor count for Third-side (Yuri) buildings (ThirdSurvivorDivisor= in [General]).
    /// YR addition. Default 750.
    pub third_survivor_divisor: i32,

    // -- Terrain movement modifiers --
    /// Speed multiplier when moving uphill (SlopeClimb= in [General]).
    /// Applied per-cell during movement when next cell is higher than current.
    /// Not present in vanilla rulesmd.ini; uses compiled default from the original engine.
    pub slope_climb: SimFixed,
    /// Speed multiplier when moving downhill (SlopeDescend= in [General]).
    /// Applied per-cell during movement when next cell is lower than current.
    pub slope_descend: SimFixed,

    // -- Entity ambient glow on dark maps --
    /// Additive brightness boost for unit sprites (ExtraUnitLight= in [General]).
    /// Makes vehicles visible on dark maps. Default 0.2.
    pub extra_unit_light: f32,
    /// Additive brightness boost for infantry sprites (ExtraInfantryLight= in [General]).
    pub extra_infantry_light: f32,
    /// Additive brightness boost for aircraft sprites (ExtraAircraftLight= in [General]).
    pub extra_aircraft_light: f32,

    // -- Movement arrival --
    /// Distance in leptons below which a blocked unit stops instead of repathing.
    /// CloseEnough=2.25 in vanilla rulesmd.ini (2.25 cells × 256 lep/cell ≈ 576 leptons).
    pub close_enough: SimFixed,

    // -- Service depot / unit repair --
    /// Ticks between applying RepairStep HP when a unit is on a repair depot.
    /// Derived from URepairRate= in [General] (minutes). Default 0.016 min ≈ 14 ticks at 15 Hz.
    pub unit_repair_rate_ticks: u32,
    /// HP healed per repair step on a service depot (RepairStep= in [General]). Default 8.
    pub repair_step: u16,
    /// Percent of build cost charged for a full unit repair (RepairPercent= in [General]).
    /// Default 15 (meaning 15%). Total cost = cost * repair_percent / 100.
    pub repair_percent: u16,

    // -- Aircraft ammo reload --
    /// Ticks to reload one ammo point at an airfield (from ReloadRate= minutes in [General]).
    /// Default: 270 ticks (0.3 min × 60 sec × 15 ticks/sec).
    pub reload_rate_ticks: u32,

    // -- Movement delay timers --
    /// Ticks between pathfinding retry attempts (PathDelay= in [General]).
    /// INI value is in minutes; converted to ticks: minutes × 60 × 15.
    /// Default: 0.01 min = 9 ticks. While counting down, pathfinding is not called.
    pub path_delay_ticks: u16,
    /// Ticks to wait when blocked by a friendly unit before aggressive repath
    /// (BlockagePathDelay= in [General]). INI value is in frames (directly).
    /// When this timer expires, the unit re-pathfinds with urgency=2 (scatter).
    pub blockage_path_delay_ticks: u16,

    /// Overlay type names that are opaque concrete walls (ConcreteWalls= in [General]).
    /// Concrete walls do NOT render a ghost sprite during placement -- only the
    /// valid/invalid cell grid is shown. Fence walls (not in this list) still
    /// render their connectivity ghost. Stored uppercase for case-insensitive matching.
    pub concrete_walls: Vec<String>,

    // -- Lightning Storm superweapon constants --
    /// Duration of active storm in game frames (LightningStormDuration= in [General]).
    /// Default 180 frames (12 seconds at 15 fps).
    pub lightning_storm_duration: i32,
    /// Damage per lightning bolt strike (LightningDamage= in [General]). Default 250.
    pub lightning_damage: i32,
    /// Deferment countdown before storm bolts begin (LightningDeferment= in [General]).
    /// Default 250 frames.
    pub lightning_deferment: i32,
    /// Frames between center bolt strikes (LightningHitDelay= in [General]). Default 10.
    pub lightning_hit_delay: i32,
    /// Frames between scatter bolt strikes (LightningScatterDelay= in [General]). Default 5.
    pub lightning_scatter_delay: i32,
    /// Cell radius for scatter bolt placement (LightningCellSpread= in [General]). Default 10.
    pub lightning_cell_spread: i32,
    /// Minimum manhattan distance between consecutive bolts (LightningSeparation= in [General]).
    /// Default 3.
    pub lightning_separation: i32,
    /// Warhead ID for lightning bolt damage (LightningWarhead= in [General]). Default "IonWH".
    pub lightning_warhead: String,
    // --- IronCurtain ([CombatDamage]) ---
    /// IronCurtain invulnerability duration in frames (IronCurtainDuration= in [CombatDamage]).
    pub iron_curtain_duration: u32,
    // --- IronCurtain ([General]) ---
    /// Animation played on IC target (IronCurtainInvokeAnim= in [General]). Default IRONBLST.
    pub iron_curtain_invoke_anim: String,
    // --- ForceShield ([General]) ---
    /// Cell radius of ForceShield AoE (ForceShieldRadius= in [General]).
    pub force_shield_radius: u32,
    /// ForceShield invulnerability duration in frames (ForceShieldDuration= in [General]).
    pub force_shield_duration: u32,
    /// Power blackout duration triggered by ForceShield (ForceShieldBlackoutDuration= in [General]).
    pub force_shield_blackout_duration: u32,
    /// Frames before fade sound plays (ForceShieldPlayFadeSoundTime= in [General]).
    pub force_shield_fade_sound_time: u32,
    /// Animation played on FS target (ForceShieldInvokeAnim= in [General]). Default FORCSHLD.
    pub force_shield_invoke_anim: String,
    // --- PsychicReveal ([General]) ---
    /// Cell radius revealed by PsychicReveal (PsychicRevealRadius= in [General]).
    pub psychic_reveal_radius: u32,
    // --- GeneticConverter ([CombatDamage] + [General]) ---
    /// Warhead used for mutation (MutateWarhead= in [CombatDamage]). Default "Mutate".
    pub mutate_warhead: String,
    /// Warhead used for mutate explosion (MutateExplosionWarhead= in [CombatDamage]).
    pub mutate_explosion_warhead: String,
    /// Whether MutateExplosion is enabled (MutateExplosion= in [General]). Default true.
    pub mutate_explosion: bool,
    /// `[General] MetallicDebris=` — list of animation names to spawn (50%-RNG
    /// gated, count-checked) on bridge-cell collapse. Default 20 entries.
    /// Mirrors gamemd `Rules+0x140` (data ptr) / `+0x14C` (count).
    pub metallic_debris: Vec<String>,
}

/// Default animation rate when art.ini section is missing.
/// Matches gamemd constructor default: 1 game frame at 60fps ≈ 17ms.
const DEFAULT_ANIM_RATE_MS: u32 = 17;

/// Zip a parallel pair of paradrop INI keys (`Inf` + `Num`) into `(type, count)` pairs.
/// `skip_count_assert` mirrors gamemd's Soviet branch which lacks the equality check.
fn parse_paradrop_list(
    general: &crate::rules::ini_parser::IniSection,
    inf_key: &str,
    num_key: &str,
    skip_count_assert: bool,
    default: Vec<(String, u32)>,
) -> Vec<(String, u32)> {
    let inf: Vec<String> = match general.get_list(inf_key) {
        Some(list) => list
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase())
            .collect(),
        None => return default,
    };
    let nums: Vec<u32> = match general.get_list(num_key) {
        Some(list) => list
            .into_iter()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect(),
        None => return default,
    };
    if !skip_count_assert && inf.len() != nums.len() {
        log::warn!(
            "Paradrop list mismatch: {}={} entries but {}={} entries — using defaults",
            inf_key,
            inf.len(),
            num_key,
            nums.len(),
        );
        return default;
    }
    inf.into_iter().zip(nums.into_iter()).collect()
}

impl Default for GeneralRules {
    fn default() -> Self {
        Self {
            veteran_sight: 0,
            leptons_per_sight_increase: 0,
            gap_radius: 10,
            reveal_by_height: true,
            tunnel_speed: sim_from_f32(6.0),
            missile_rot_var: sim_from_f32(1.0),
            flight_level: 1500,
            parachute_max_fall_rate: -3,
            paradrop_radius: 1024,
            paradrop_aircraft_type: "PDPLANE".to_string(),
            parachute_shp: None,
            parachute_render: None,
            amer_paradrop_list: vec![("E1".to_string(), 8)],
            ally_paradrop_list: vec![("E1".to_string(), 6)],
            sov_paradrop_list: vec![("E2".to_string(), 9)],
            yuri_paradrop_list: vec![("INIT".to_string(), 6)],
            tiberium_grows: true,
            tiberium_spreads: true,
            growth_rate_minutes: 5.0,
            warp_in: AnimRef {
                name: "WARPIN".to_string(),
                rate_ms: 120,
            },
            warp_out: AnimRef {
                name: "WARPOUT".to_string(),
                rate_ms: 120,
            },
            warp_away: AnimRef {
                name: "WARPAWAY".to_string(),
                rate_ms: 300,
            },
            chrono_sparkle1: AnimRef {
                name: "CHRONOSK".to_string(),
                rate_ms: 120,
            },
            wake: AnimRef {
                name: "WAKE1".to_string(),
                rate_ms: 120,
            },
            attack_cursor_on_disguise: false,
            tree_targeting: false,
            condition_yellow: 0.5,
            condition_yellow_x1000: 500,
            condition_red: 0.25,
            condition_red_x1000: 250,
            building_garrisoned_sound: None,
            gui_main_button_sound: None,
            direct_rocking_coefficient: SimFixed::lit("1.5"),
            fallback_coefficient: SimFixed::lit("0.1"),
            chrono_in_sound: Some("ChronoMinerTeleport".to_string()),
            chrono_out_sound: Some("ChronoMinerTeleport".to_string()),
            damage_delay_minutes: 1.0,
            spy_power_blackout_frames: 1000,
            damage_fire_types: vec![],
            barrel_particle: None,
            tiberium_short_scan: 6,
            tiberium_long_scan: 48,
            slave_miner_short_scan: 8,
            slave_miner_slave_scan: 14,
            slave_miner_long_scan: 48,
            slave_miner_scan_correction: 3,
            slave_miner_kick_frame_delay: 150,
            harvester_too_far_distance: 5,
            chrono_harv_too_far_distance: 50,
            harvester_load_rate: 2,
            harvester_dump_tenths: 144,
            chrono_delay: 60,
            chrono_reinf_delay: 180,
            chrono_distance_factor: 48,
            chrono_trigger: true,
            chrono_minimum_delay: 16,
            chrono_range_minimum: 0,
            purifier_bonus_pct: 25,
            ai_virtual_purifiers: [4, 2, 0],
            allied_survivor_divisor: 500,
            soviet_survivor_divisor: 250,
            third_survivor_divisor: 750,
            // Compiled defaults from the original engine.
            // Not present in vanilla rulesmd.ini — mods can override via [General].
            slope_climb: SimFixed::lit("0.6"),
            slope_descend: SimFixed::lit("1.2"),
            extra_unit_light: 0.2,
            extra_infantry_light: 0.2,
            extra_aircraft_light: 0.2,
            // CloseEnough=2.25 cells in vanilla rulesmd.ini → 576 leptons.
            close_enough: SimFixed::from_num(576),
            // URepairRate=.016 min = 0.96 sec ≈ 14 ticks at 15 Hz.
            unit_repair_rate_ticks: 14,
            repair_step: 8,
            repair_percent: 15,
            // ReloadRate=.3 min = 18 sec = 270 ticks at 15 Hz.
            reload_rate_ticks: 270,
            // PathDelay=.01 min = 0.6 sec = 9 ticks at 15 Hz.
            path_delay_ticks: 9,
            // BlockagePathDelay=60 frames (directly in frames, not minutes).
            blockage_path_delay_ticks: 60,
            concrete_walls: Vec::new(),
            cliff_back_impassability: 2,
            lightning_storm_duration: 180,
            lightning_damage: 250,
            lightning_deferment: 250,
            lightning_hit_delay: 10,
            lightning_scatter_delay: 5,
            lightning_cell_spread: 10,
            lightning_separation: 3,
            lightning_warhead: "IonWH".to_string(),
            iron_curtain_duration: 750,
            iron_curtain_invoke_anim: "IRONBLST".to_string(),
            force_shield_radius: 4,
            force_shield_duration: 500,
            force_shield_blackout_duration: 1000,
            force_shield_fade_sound_time: 75,
            force_shield_invoke_anim: "FORCSHLD".to_string(),
            psychic_reveal_radius: 15,
            mutate_warhead: "Mutate".to_string(),
            mutate_explosion_warhead: "MutateExplosion".to_string(),
            mutate_explosion: true,
            metallic_debris: vec![
                "DBRIS1LG", "DBRIS2LG", "DBRIS3LG", "DBRIS4LG", "DBRIS5LG", "DBRIS6LG", "DBRIS7LG",
                "DBRIS8LG", "DBRIS9LG", "DBRS10LG", "DBRIS1SM", "DBRIS2SM", "DBRIS3SM", "DBRIS4SM",
                "DBRIS5SM", "DBRIS6SM", "DBRIS7SM", "DBRIS8SM", "DBRIS9SM", "DBRS10SM",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

/// Garrison/occupation combat rules parsed from `[CombatDamage]` in `rules(md).ini`.
/// These global multipliers govern how garrisoned infantry fire from buildings.
#[derive(Debug, Clone)]
pub struct GarrisonRules {
    /// Damage multiplier applied to garrison fire.
    pub occupy_damage_multiplier: SimFixed,
    /// ROF divisor for garrison fire -- higher = faster.
    pub occupy_rof_multiplier: SimFixed,
    /// Fixed weapon range in cells for garrisoned fire, replaces weapon's own range.
    pub occupy_weapon_range: i32,
    /// Damage multiplier for bunker passengers.
    pub bunker_damage_multiplier: f32,
    /// ROF divisor for bunker passengers.
    pub bunker_rof_multiplier: f32,
    /// Range bonus in cells for bunker passengers.
    pub bunker_weapon_range_bonus: i32,
    /// Damage multiplier for open-topped passengers.
    pub open_topped_damage_multiplier: f32,
    /// Range bonus in cells for open-topped passengers.
    pub open_topped_range_bonus: i32,
}

impl Default for GarrisonRules {
    fn default() -> Self {
        Self {
            occupy_damage_multiplier: SimFixed::ONE,
            occupy_rof_multiplier: SimFixed::ONE,
            occupy_weapon_range: 5,
            bunker_damage_multiplier: 1.0,
            bunker_rof_multiplier: 1.0,
            bunker_weapon_range_bonus: 0,
            open_topped_damage_multiplier: 1.0,
            open_topped_range_bonus: 0,
        }
    }
}

impl GarrisonRules {
    fn from_ini(ini: &IniFile) -> Self {
        let section = ini.section("CombatDamage");
        let get_f32 = |key: &str, default: f32| -> f32 {
            section.and_then(|s| s.get_f32(key)).unwrap_or(default)
        };
        let get_i32 = |key: &str, default: i32| -> i32 {
            section.and_then(|s| s.get_i32(key)).unwrap_or(default)
        };
        Self {
            occupy_damage_multiplier: sim_from_f32(get_f32("OccupyDamageMultiplier", 1.0)),
            occupy_rof_multiplier: sim_from_f32(get_f32("OccupyROFMultiplier", 1.0)),
            occupy_weapon_range: get_i32("OccupyWeaponRange", 5),
            bunker_damage_multiplier: get_f32("BunkerDamageMultiplier", 1.0),
            bunker_rof_multiplier: get_f32("BunkerROFMultiplier", 1.0),
            bunker_weapon_range_bonus: get_i32("BunkerWeaponRangeBonus", 0),
            open_topped_damage_multiplier: get_f32("OpenToppedDamageMultiplier", 1.0),
            open_topped_range_bonus: get_i32("OpenToppedRangeBonus", 0),
        }
    }
}

/// Bridge damage/destruction rules parsed from `rules(md).ini`.
#[derive(Debug, Clone)]
pub struct BridgeRules {
    /// Hit points shared by a destroyable bridge span.
    pub strength: u16,
    /// Whether bridges are destroyable unless the map overrides it.
    pub destroyable_by_default: bool,
    /// SHP animation names to spawn when a bridge group is destroyed
    /// (e.g., TWLT026, TWLT036, TWLT050, TWLT070). Picked randomly per cell.
    pub explosions: Vec<String>,
    /// Maximum metallic-debris voxels spawned per destroyed bridge cell.
    /// Parsed from `[General] BridgeVoxelMax=` in rules.ini (default 3).
    /// Consumed by the damage state machine in a later tier.
    pub voxel_max: u8,
    /// Sound ID played when a bridge segment is repaired by an
    /// Engineer entering a `BridgeRepairHut=yes` building.
    /// Parsed from `[AudioVisual] RepairBridgeSound=` in rules.ini
    /// (stock default `BridgeRepaired`). Stored uppercased.
    /// `None` means the consumer applies its own default.
    pub repair_sound: Option<String>,
}

impl Default for BridgeRules {
    fn default() -> Self {
        Self {
            strength: 1500,
            destroyable_by_default: true,
            explosions: Vec::new(),
            voxel_max: 3,
            repair_sound: None,
        }
    }
}

impl BridgeRules {
    fn from_ini(ini: &IniFile) -> Self {
        let strength = ini
            .section("CombatDamage")
            .and_then(|section| section.get_i32("BridgeStrength"))
            .unwrap_or(1500)
            .max(1) as u16;
        let destroyable_by_default = ini
            .section("CombatDamage")
            .and_then(|section| section.get_bool("DestroyableBridges"))
            .unwrap_or(true);
        let explosions = ini
            .section("General")
            .and_then(|section| section.get_list("BridgeExplosions"))
            .map(|list| list.into_iter().map(|s| s.to_uppercase()).collect())
            .unwrap_or_default();
        let voxel_max = ini
            .section("General")
            .and_then(|section| section.get_i32("BridgeVoxelMax"))
            .unwrap_or(3)
            .clamp(0, 255) as u8;
        let repair_sound = ini
            .section("AudioVisual")
            .and_then(|section| section.get("RepairBridgeSound"))
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());
        Self {
            strength,
            destroyable_by_default,
            explosions,
            voxel_max,
            repair_sound,
        }
    }
}

impl GeneralRules {
    fn from_ini(ini: &IniFile) -> Self {
        let Some(general) = ini.section("General") else {
            return Self::default();
        };
        // ConditionYellow/ConditionRed live in [AudioVisual], not [General].
        let audio_visual = ini.section("AudioVisual");
        // IronCurtainDuration, MutateWarhead, MutateExplosionWarhead live in [CombatDamage].
        let combat_damage = ini.section("CombatDamage");
        // WarpIn/WarpOut/WarpAway values may contain semicolons with secondary
        // anims (e.g., "WARPIN;WAKE2"). We only use the primary anim name.
        let parse_anim_name = |key: &str, default: &str| -> String {
            general
                .get(key)
                .map(|v| v.split(';').next().unwrap_or(default).trim().to_string())
                .unwrap_or_else(|| default.to_string())
        };
        let defaults = Self::default();
        let condition_yellow_f32: f32 = audio_visual
            .and_then(|s| s.get_percent("ConditionYellow"))
            .unwrap_or(0.5);
        let condition_red_f32: f32 = audio_visual
            .and_then(|s| s.get_percent("ConditionRed"))
            .unwrap_or(0.25);
        Self {
            veteran_sight: general.get_i32("VeteranSight").unwrap_or(0),
            leptons_per_sight_increase: general.get_i32("LeptonsPerSightIncrease").unwrap_or(0),
            gap_radius: general.get_i32("GapRadius").unwrap_or(10),
            reveal_by_height: general.get_bool("RevealByHeight").unwrap_or(true),
            tunnel_speed: general
                .get_f32("TunnelSpeed")
                .map(sim_from_f32)
                .unwrap_or(sim_from_f32(6.0)),
            missile_rot_var: general
                .get_f32("MissileROTVar")
                .map(sim_from_f32)
                .unwrap_or(sim_from_f32(1.0)),
            flight_level: general.get_i32("FlightLevel").unwrap_or(1500),
            parachute_max_fall_rate: general.get_i32("ParachuteMaxFallRate").unwrap_or(-3),
            paradrop_radius: general.get_i32("ParadropRadius").unwrap_or(1024),
            paradrop_aircraft_type: general
                .get("ParaDropPlane")
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "PDPLANE".to_string()),
            parachute_shp: general
                .get("Parachute")
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty()),
            // Resolved later in `resolve_art_rates` once art.ini is available.
            parachute_render: None,
            amer_paradrop_list: parse_paradrop_list(
                general,
                "AmerParaDropInf",
                "AmerParaDropNum",
                false,
                vec![("E1".to_string(), 8)],
            ),
            ally_paradrop_list: parse_paradrop_list(
                general,
                "AllyParaDropInf",
                "AllyParaDropNum",
                false,
                vec![("E1".to_string(), 6)],
            ),
            sov_paradrop_list: parse_paradrop_list(
                general,
                "SovParaDropInf",
                "SovParaDropNum",
                true,
                vec![("E2".to_string(), 9)],
            ),
            yuri_paradrop_list: parse_paradrop_list(
                general,
                "YuriParaDropInf",
                "YuriParaDropNum",
                false,
                vec![("INIT".to_string(), 6)],
            ),
            tiberium_grows: general.get_bool("TiberiumGrows").unwrap_or(true),
            tiberium_spreads: general.get_bool("TiberiumSpreads").unwrap_or(true),
            growth_rate_minutes: general.get_f32("GrowthRate").unwrap_or(5.0),
            attack_cursor_on_disguise: general.get_bool("AttackCursorOnDisguise").unwrap_or(false),
            tree_targeting: general.get_bool("TreeTargeting").unwrap_or(false),
            condition_yellow: condition_yellow_f32,
            condition_yellow_x1000: (condition_yellow_f32 as f64 * 1000.0) as i64,
            condition_red: condition_red_f32,
            condition_red_x1000: (condition_red_f32 as f64 * 1000.0) as i64,
            building_garrisoned_sound: audio_visual
                .and_then(|s| s.get("BuildingGarrisonedSound"))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            gui_main_button_sound: audio_visual
                .and_then(|s| s.get("GUIMainButtonSound"))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            direct_rocking_coefficient: audio_visual
                .and_then(|s| s.get_f32("DirectRockingCoefficient"))
                .map(sim_from_f32)
                .unwrap_or(SimFixed::lit("1.5")),
            fallback_coefficient: audio_visual
                .and_then(|s| s.get_f32("FallBackCoefficient"))
                .map(sim_from_f32)
                .unwrap_or(SimFixed::lit("0.1")),
            chrono_in_sound: general
                .get("ChronoInSound")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| Some("ChronoMinerTeleport".to_string())),
            chrono_out_sound: general
                .get("ChronoOutSound")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| Some("ChronoMinerTeleport".to_string())),
            warp_in: AnimRef {
                name: parse_anim_name("WarpIn", "WARPIN"),
                rate_ms: defaults.warp_in.rate_ms,
            },
            warp_out: AnimRef {
                name: parse_anim_name("WarpOut", "WARPOUT"),
                rate_ms: defaults.warp_out.rate_ms,
            },
            warp_away: AnimRef {
                name: parse_anim_name("WarpAway", "WARPAWAY"),
                rate_ms: defaults.warp_away.rate_ms,
            },
            chrono_sparkle1: AnimRef {
                name: parse_anim_name("ChronoSparkle1", "CHRONOSK"),
                rate_ms: defaults.chrono_sparkle1.rate_ms,
            },
            wake: AnimRef {
                name: parse_anim_name("Wake", "WAKE1"),
                rate_ms: defaults.wake.rate_ms,
            },
            damage_delay_minutes: general.get_f32("DamageDelay").unwrap_or(1.0),
            spy_power_blackout_frames: general.get_i32("SpyPowerBlackout").unwrap_or(1000).max(0)
                as u32,
            damage_fire_types: general
                .get_list("DamageFireTypes")
                .map(|list| {
                    list.into_iter()
                        .filter(|s| !s.is_empty())
                        .map(|s| AnimRef {
                            name: s.to_uppercase(),
                            rate_ms: DEFAULT_ANIM_RATE_MS,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            barrel_particle: general
                .get("BarrelParticle")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            tiberium_short_scan: general.get_i32("TiberiumShortScan").unwrap_or(6),
            tiberium_long_scan: general.get_i32("TiberiumLongScan").unwrap_or(48),
            slave_miner_short_scan: general.get_i32("SlaveMinerShortScan").unwrap_or(8),
            slave_miner_slave_scan: general.get_i32("SlaveMinerSlaveScan").unwrap_or(14),
            slave_miner_long_scan: general.get_i32("SlaveMinerLongScan").unwrap_or(48),
            slave_miner_scan_correction: general.get_i32("SlaveMinerScanCorrection").unwrap_or(3),
            slave_miner_kick_frame_delay: general
                .get_i32("SlaveMinerKickFrameDelay")
                .unwrap_or(150)
                .max(0) as u32,
            harvester_too_far_distance: general.get_i32("HarvesterTooFarDistance").unwrap_or(5),
            chrono_harv_too_far_distance: general.get_i32("ChronoHarvTooFarDistance").unwrap_or(50),
            harvester_load_rate: general.get_i32("HarvesterLoadRate").unwrap_or(2),
            harvester_dump_tenths: {
                let rate = general.get_f32("HarvesterDumpRate").unwrap_or(0.016);
                // Tenths of a frame: 0.016 × 9000 = 144 (= 14.4 frames/bale).
                // Clamp to u16::MAX (~6553 ticks/bale) to keep the timer wraparound-safe.
                (rate * 9000.0).clamp(0.0, u16::MAX as f32).round() as u16
            },
            chrono_delay: general.get_i32("ChronoDelay").unwrap_or(60),
            chrono_reinf_delay: general.get_i32("ChronoReinfDelay").unwrap_or(180),
            chrono_distance_factor: general.get_i32("ChronoDistanceFactor").unwrap_or(48),
            chrono_trigger: general.get_bool("ChronoTrigger").unwrap_or(true),
            chrono_minimum_delay: general.get_i32("ChronoMinimumDelay").unwrap_or(16),
            chrono_range_minimum: general.get_i32("ChronoRangeMinimum").unwrap_or(0),
            purifier_bonus_pct: (general.get_percent("PurifierBonus").unwrap_or(0.25) * 100.0)
                .round() as i32,
            ai_virtual_purifiers: {
                let defaults = [4, 2, 0];
                general
                    .get("AIVirtualPurifiers")
                    .and_then(|raw| {
                        let parsed: Vec<i32> = raw
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .filter_map(|s| s.parse::<i32>().ok())
                            .collect();
                        if parsed.len() == 3 {
                            Some([parsed[0], parsed[1], parsed[2]])
                        } else {
                            None
                        }
                    })
                    .unwrap_or(defaults)
            },
            allied_survivor_divisor: general.get_i32("AlliedSurvivorDivisor").unwrap_or(500),
            soviet_survivor_divisor: general.get_i32("SovietSurvivorDivisor").unwrap_or(250),
            third_survivor_divisor: general.get_i32("ThirdSurvivorDivisor").unwrap_or(750),
            slope_climb: general
                .get_f32("SlopeClimb")
                .map(sim_from_f32)
                .unwrap_or(defaults.slope_climb),
            slope_descend: general
                .get_f32("SlopeDescend")
                .map(sim_from_f32)
                .unwrap_or(defaults.slope_descend),
            extra_unit_light: general.get_f32("ExtraUnitLight").unwrap_or(0.2),
            extra_infantry_light: general.get_f32("ExtraInfantryLight").unwrap_or(0.2),
            extra_aircraft_light: general.get_f32("ExtraAircraftLight").unwrap_or(0.2),
            close_enough: general
                .get_f32("CloseEnough")
                .map(|cells| sim_from_f32(cells * 256.0))
                .unwrap_or(defaults.close_enough),
            // URepairRate= is in minutes. Convert to ticks: minutes * 60 * 15 ticks/sec.
            unit_repair_rate_ticks: general
                .get_f32("URepairRate")
                .map(|minutes| (minutes * 60.0 * 15.0).round().max(1.0) as u32)
                .unwrap_or(defaults.unit_repair_rate_ticks),
            repair_step: general
                .get_i32("RepairStep")
                .unwrap_or(defaults.repair_step as i32)
                .max(1) as u16,
            repair_percent: general
                .get_percent("RepairPercent")
                .map(|frac| (frac * 100.0).round() as u16)
                .unwrap_or(defaults.repair_percent),
            reload_rate_ticks: general
                .get_f32("ReloadRate")
                .map(|minutes| (minutes * 60.0 * 15.0).round().max(1.0) as u32)
                .unwrap_or(defaults.reload_rate_ticks),
            // PathDelay= is in minutes. Convert to ticks: minutes * 60 * 15.
            path_delay_ticks: general
                .get_f32("PathDelay")
                .map(|minutes| (minutes * 60.0 * 15.0).round().max(1.0) as u16)
                .unwrap_or(defaults.path_delay_ticks),
            // BlockagePathDelay= is directly in frames (ticks).
            blockage_path_delay_ticks: general
                .get_i32("BlockagePathDelay")
                .map(|frames| frames.max(1) as u16)
                .unwrap_or(defaults.blockage_path_delay_ticks),
            concrete_walls: general
                .get_list("ConcreteWalls")
                .map(|list| {
                    list.into_iter()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_ascii_uppercase())
                        .collect()
                })
                .unwrap_or_default(),
            cliff_back_impassability: general
                .get_i32("CliffBackImpassability")
                .unwrap_or(2)
                .clamp(0, 2) as u8,
            lightning_storm_duration: general.get_i32("LightningStormDuration").unwrap_or(180),
            lightning_damage: general.get_i32("LightningDamage").unwrap_or(250),
            lightning_deferment: general.get_i32("LightningDeferment").unwrap_or(250),
            lightning_hit_delay: general.get_i32("LightningHitDelay").unwrap_or(10).max(1),
            lightning_scatter_delay: general.get_i32("LightningScatterDelay").unwrap_or(5).max(1),
            lightning_cell_spread: general.get_i32("LightningCellSpread").unwrap_or(10),
            lightning_separation: general.get_i32("LightningSeparation").unwrap_or(3),
            lightning_warhead: general
                .get("LightningWarhead")
                .unwrap_or("IonWH")
                .to_string(),
            iron_curtain_duration: combat_damage
                .and_then(|s| s.get_i32("IronCurtainDuration"))
                .unwrap_or(750) as u32,
            iron_curtain_invoke_anim: general
                .get("IronCurtainInvokeAnim")
                .unwrap_or("IRONBLST")
                .to_string(),
            force_shield_radius: general.get_i32("ForceShieldRadius").unwrap_or(4) as u32,
            force_shield_duration: general.get_i32("ForceShieldDuration").unwrap_or(500) as u32,
            force_shield_blackout_duration: general
                .get_i32("ForceShieldBlackoutDuration")
                .unwrap_or(1000) as u32,
            force_shield_fade_sound_time: general
                .get_i32("ForceShieldPlayFadeSoundTime")
                .unwrap_or(75) as u32,
            force_shield_invoke_anim: general
                .get("ForceShieldInvokeAnim")
                .unwrap_or("FORCSHLD")
                .to_string(),
            psychic_reveal_radius: general.get_i32("PsychicRevealRadius").unwrap_or(15) as u32,
            mutate_warhead: combat_damage
                .and_then(|s| s.get("MutateWarhead"))
                .unwrap_or("Mutate")
                .to_string(),
            mutate_explosion_warhead: combat_damage
                .and_then(|s| s.get("MutateExplosionWarhead"))
                .unwrap_or("MutateExplosion")
                .to_string(),
            mutate_explosion: general.get_bool("MutateExplosion").unwrap_or(true),
            metallic_debris: general
                .get("MetallicDebris")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| Self::default().metallic_debris),
        }
    }

    /// Resolve animation playback rates from art.ini sections.
    ///
    /// Called after both rules.ini and art.ini are loaded. Looks up each
    /// anim's own `[ANIM_NAME]` section for `Rate=` (ms per frame).
    pub fn resolve_art_rates(&mut self, art_ini: &IniFile) {
        fn rate_from_section(ini: &IniFile, name: &str, fallback: u32) -> u32 {
            ini.section(name)
                .and_then(|s| s.get_i32("Rate"))
                .map(|r| crate::rules::art_data::art_rate_to_delay_ms(r))
                .unwrap_or(fallback)
        }
        self.warp_in.rate_ms = rate_from_section(art_ini, &self.warp_in.name, DEFAULT_ANIM_RATE_MS);
        self.warp_out.rate_ms =
            rate_from_section(art_ini, &self.warp_out.name, DEFAULT_ANIM_RATE_MS);
        self.warp_away.rate_ms =
            rate_from_section(art_ini, &self.warp_away.name, DEFAULT_ANIM_RATE_MS);
        self.chrono_sparkle1.rate_ms =
            rate_from_section(art_ini, &self.chrono_sparkle1.name, DEFAULT_ANIM_RATE_MS);
        self.wake.rate_ms = rate_from_section(art_ini, &self.wake.name, DEFAULT_ANIM_RATE_MS);
        log::info!(
            "Warp anim rates: {}={}ms, {}={}ms, {}={}ms, wake: {}={}ms",
            self.warp_in.name,
            self.warp_in.rate_ms,
            self.warp_out.name,
            self.warp_out.rate_ms,
            self.warp_away.name,
            self.warp_away.rate_ms,
            self.wake.name,
            self.wake.rate_ms,
        );
        for fire in &mut self.damage_fire_types {
            fire.rate_ms = rate_from_section(art_ini, &fire.name, DEFAULT_ANIM_RATE_MS);
        }
        if !self.damage_fire_types.is_empty() {
            log::info!(
                "DamageFireTypes: {} types ({})",
                self.damage_fire_types.len(),
                self.damage_fire_types
                    .iter()
                    .map(|f| format!("{}={}ms", f.name, f.rate_ms))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        // Parachute render config: [General] Parachute= names the section in
        // artmd.ini that holds the chute SHP's animation metadata.
        self.parachute_render = self.parachute_shp.as_deref().and_then(|shp_name| {
            let section = art_ini.section(shp_name)?;
            let rate = section.get_i32("Rate").unwrap_or(1);
            let rate_ms = crate::rules::art_data::art_rate_to_delay_ms(rate);
            let loop_start = section.get_i32("LoopStart").unwrap_or(0).max(0) as u16;
            let loop_end = section.get_i32("LoopEnd").unwrap_or(0).max(0) as u16;
            let end_frame = loop_end.saturating_add(1);
            let z_adjust = section.get_i32("ZAdjust").unwrap_or(0) as i16;
            let alt_palette = section.get_bool("AltPalette").unwrap_or(false);
            Some(ParachuteRenderConfig {
                shp_name: shp_name.to_string(),
                rate_ms,
                loop_start,
                end_frame,
                z_adjust,
                alt_palette,
            })
        });
        if let Some(ref pc) = self.parachute_render {
            log::info!(
                "Parachute render config loaded: shp={} rate_ms={} loop_start={} end_frame={} z_adjust={} alt_palette={}",
                pc.shp_name,
                pc.rate_ms,
                pc.loop_start,
                pc.end_frame,
                pc.z_adjust,
                pc.alt_palette,
            );
        } else {
            log::warn!(
                "Parachute render config NOT loaded (missing [General] Parachute= or referenced art.ini section)"
            );
        }
    }
}

impl ProductionRules {
    fn from_ini(ini: &IniFile) -> Self {
        let Some(general) = ini.section("General") else {
            return Self::default();
        };

        let bs = general.get_f32("BuildSpeed").unwrap_or(0.1);
        let mf = general.get_f32("MultipleFactory").unwrap_or(0.8);
        let lpp = general.get_f32("LowPowerPenaltyModifier").unwrap_or(1.0);
        let min_lp = general.get_f32("MinLowPowerProductionSpeed").unwrap_or(0.5);
        let max_lp = general.get_f32("MaxLowPowerProductionSpeed").unwrap_or(0.9);
        let wall_coeff = general.get_f32("WallBuildSpeedCoefficient").unwrap_or(1.0);
        let result = Self {
            build_speed: bs,
            multiple_factory: mf,
            low_power_penalty_modifier: lpp,
            min_low_power_production_speed: min_lp,
            max_low_power_production_speed: max_lp,
            multiple_factory_ppm: f32_to_ppm(mf, 0.01),
            low_power_penalty_modifier_ppm: f32_to_ppm(lpp, 0.0),
            min_low_power_production_speed_ppm: f32_to_ppm(min_lp, 0.0),
            max_low_power_production_speed_ppm: f32_to_ppm(max_lp.max(min_lp), 0.0),
            build_speed_x1000: (bs.max(0.01) as f64 * 1000.0).round() as u64,
            wall_build_speed_coefficient: wall_coeff,
        };
        log::info!(
            "ProductionRules: BuildSpeed={}, MultipleFactory={}, LowPowerPenalty={}",
            result.build_speed,
            result.multiple_factory,
            result.low_power_penalty_modifier,
        );
        result
    }
}

/// Master container for all game data parsed from rules.ini.
///
/// All lookups are by string ID (case-sensitive — IDs are already stored
/// in their original casing from rules.ini). The sim/ module uses RuleSet
/// to look up costs, speeds, weapons, and prerequisites for every game action.
#[derive(Debug)]
pub struct RuleSet {
    /// All game objects indexed by their ID (e.g., "MTNK" → ObjectType).
    objects: HashMap<String, ObjectType>,
    /// All weapons indexed by ID (e.g., "105mm" → WeaponType).
    weapons: HashMap<String, WeaponType>,
    /// All warheads indexed by ID (e.g., "AP" → WarheadType).
    warheads: HashMap<String, WarheadType>,
    /// All projectiles indexed by ID (e.g., "InvisibleLow" → ProjectileType).
    projectiles: HashMap<String, ProjectileType>,
    pub production: ProductionRules,
    /// Global gameplay constants (vision, gap generator, etc.).
    pub general: GeneralRules,
    /// Infantry IDs in registry order.
    pub infantry_ids: Vec<String>,
    /// Vehicle IDs in registry order.
    pub vehicle_ids: Vec<String>,
    /// Aircraft IDs in registry order.
    pub aircraft_ids: Vec<String>,
    /// Building IDs in registry order.
    pub building_ids: Vec<String>,
    /// Maps structure ID (uppercase) → FactoryType for quick lookup.
    /// Built once at load time from all ObjectType entries with Factory= set.
    /// Used by production_tech to determine what a building produces without
    /// hardcoding building names.
    pub factory_map: HashMap<String, FactoryType>,
    /// Maps prerequisite alias (uppercase, e.g. "POWER") → list of building IDs
    /// (uppercase) that satisfy it. Built from [General] PrerequisiteXxx keys.
    /// RA2 uses these so that Prerequisite=POWER means "any power plant" rather
    /// than a specific building ID.
    pub prerequisite_groups: HashMap<String, Vec<String>>,
    /// Rules-driven terrain land-type semantics keyed by TMP land byte.
    pub terrain_rules: TerrainRules,
    /// Terrain object type definitions (TIBTRE*, TREE*, ROCK*, etc.) keyed by
    /// uppercase section name. Distinct from `terrain_rules` (land semantics);
    /// these are per-decoration-object types parsed from `[TerrainTypes]`.
    pub terrain_object_types: HashMap<String, TerrainObjectType>,
    /// Rules-driven bridge destruction defaults.
    pub bridge_rules: BridgeRules,
    /// Garrison/bunker/open-topped combat multipliers from [CombatDamage].
    pub garrison_rules: GarrisonRules,
    /// Radar event visual parameters (ping rectangles on minimap).
    pub radar_event_config: RadarEventConfig,
    /// All superweapon types indexed by ID (e.g., "LightningStormSpecial" → SuperWeaponType).
    pub super_weapons: HashMap<String, SuperWeaponType>,
    /// Default particle systems from `[CombatDamage]` (smoke, sparks, debris, fire-stream).
    pub combat_damage: CombatDamageDefaults,
    /// Pre-resolved bridge-related warhead names (`[CombatDamage]
    /// IonCannonWarhead=`, `C4Warhead=`). Resolution to interned IDs happens
    /// at world init.
    pub bridge_warheads: crate::rules::bridge_warheads::BridgeWarheads,
    /// `[CombatDamage] C4Delay=`. Default `0.03` minutes = 27 ticks @ 15 fps.
    /// Time between SEAL plant claim and detonation. Stored as integer ticks
    /// (not minutes) so the per-tick comparison stays integer/lockstep-safe.
    pub c4_delay_ticks: u32,
    /// Particle types in registry order. Index = `ParticleTypeId.0`.
    particle_types: Vec<ParticleType>,
    /// Uppercase name → `ParticleTypeId` for case-insensitive lookup.
    particle_types_by_name: HashMap<String, ParticleTypeId>,
    /// Particle system types in registry order. Index = `ParticleSystemTypeId.0`.
    particle_system_types: Vec<ParticleSystemType>,
    /// Uppercase name → `ParticleSystemTypeId` for case-insensitive lookup.
    particle_system_types_by_name: HashMap<String, ParticleSystemTypeId>,
    /// Smudge type registry parsed from `[SmudgeTypes]` and per-name sections.
    /// Populated by `RuleSet::from_ini` from rulesmd.ini.
    pub smudge_types: SmudgeTypeRegistry,
    /// Retained art.ini registry. Populated by app_init after `merge_art_data`
    /// so dispatchers (e.g. smudge spawning) can read per-anim spawn flags.
    pub art_registry: crate::rules::art_data::ArtRegistry,
    /// Pre-resolved IonCannonWarhead InternedId. Set at sim init via
    /// `resolve_bridge_warheads`; combat reads via `ion_cannon_warhead_id()`.
    /// `None` until resolved.
    ion_cannon_warhead_id: Option<crate::sim::intern::InternedId>,
    /// Pre-resolved C4Warhead InternedId. Same lifecycle as above.
    c4_warhead_id: Option<crate::sim::intern::InternedId>,
}

impl RuleSet {
    /// Parse a complete RuleSet from a rules.ini IniFile.
    ///
    /// Loads all type registries, individual object sections, and any
    /// weapons/warheads referenced by those objects. Missing sections
    /// are logged as warnings but don't cause errors — RA2's rules.ini
    /// sometimes references sections that don't exist.
    pub fn from_ini(ini: &IniFile) -> Result<Self, RulesError> {
        let mut objects: HashMap<String, ObjectType> = HashMap::new();
        let mut infantry_ids: Vec<String> = Vec::new();
        let mut vehicle_ids: Vec<String> = Vec::new();
        let mut aircraft_ids: Vec<String> = Vec::new();
        let mut building_ids: Vec<String> = Vec::new();
        let production: ProductionRules = ProductionRules::from_ini(ini);
        let general: GeneralRules = GeneralRules::from_ini(ini);
        let terrain_rules: TerrainRules = TerrainRules::from_ini(ini);
        let bridge_rules: BridgeRules = BridgeRules::from_ini(ini);
        let garrison_rules: GarrisonRules = GarrisonRules::from_ini(ini);
        let radar_event_config: RadarEventConfig = RadarEventConfig::from_ini(ini);

        // Step 1: Parse each type registry and load object sections.
        for &(registry_name, category) in TYPE_REGISTRIES {
            let ids: Vec<String> = parse_registry(ini, registry_name);
            log::info!("Registry [{}]: {} entries", registry_name, ids.len());

            for id in &ids {
                if let Some(section) = ini.section(id) {
                    let obj: ObjectType = ObjectType::from_ini_section(id, section, category);
                    objects.insert(id.clone(), obj);
                } else {
                    log::trace!(
                        "Object '{}' listed in [{}] but has no section",
                        id,
                        registry_name
                    );
                }
            }

            // Store ID lists per category.
            match category {
                ObjectCategory::Infantry => infantry_ids = ids,
                ObjectCategory::Vehicle => vehicle_ids = ids,
                ObjectCategory::Aircraft => aircraft_ids = ids,
                ObjectCategory::Building => building_ids = ids,
            }
        }

        // Step 2: Collect all weapon and warhead IDs referenced by objects.
        let (weapon_ids, warhead_refs) = collect_weapon_refs(&objects);

        // Step 3: Parse weapon sections.
        let mut weapons: HashMap<String, WeaponType> = HashMap::new();
        let mut warhead_ids: HashSet<String> = warhead_refs;

        for weapon_id in &weapon_ids {
            if let Some(section) = ini.section(weapon_id) {
                let weapon: WeaponType = WeaponType::from_ini_section(weapon_id, section);
                // Also collect warhead references from weapons themselves.
                if let Some(wh) = &weapon.warhead {
                    warhead_ids.insert(wh.clone());
                }
                weapons.insert(weapon_id.clone(), weapon);
            } else {
                log::trace!("Weapon '{}' referenced but has no section", weapon_id);
            }
        }

        // Step 4: Parse warhead sections.
        let mut warheads: HashMap<String, WarheadType> = HashMap::new();
        for warhead_id in &warhead_ids {
            if let Some(section) = ini.section(warhead_id) {
                let wh: WarheadType = WarheadType::from_ini_section(warhead_id, section);
                warheads.insert(warhead_id.clone(), wh);
            } else {
                log::trace!("Warhead '{}' referenced but has no section", warhead_id);
            }
        }

        // Step 5: Collect projectile IDs referenced by weapons and parse them.
        let mut projectiles: HashMap<String, ProjectileType> = HashMap::new();
        let mut projectile_ids: HashSet<String> = HashSet::new();
        for weapon in weapons.values() {
            if let Some(ref proj_id) = weapon.projectile {
                projectile_ids.insert(proj_id.clone());
            }
        }
        for proj_id in &projectile_ids {
            if let Some(section) = ini.section(proj_id) {
                let proj: ProjectileType = ProjectileType::from_ini_section(proj_id, section, None);
                projectiles.insert(proj_id.clone(), proj);
            } else {
                log::trace!("Projectile '{}' referenced but has no section", proj_id);
            }
        }

        // Step 6: Build factory lookup map from Factory= keys on all objects.
        let factory_map: HashMap<String, FactoryType> = objects
            .values()
            .filter_map(|obj| obj.factory.map(|ft| (obj.id.to_ascii_uppercase(), ft)))
            .collect();
        log::info!("Factory map: {} entries", factory_map.len());

        // Step 7: Parse prerequisite alias groups from [General].
        let prerequisite_groups: HashMap<String, Vec<String>> = parse_prerequisite_groups(ini);
        log::info!("Prerequisite groups: {} aliases", prerequisite_groups.len());

        // Step 8: Parse superweapon type registry.
        let mut super_weapons: HashMap<String, SuperWeaponType> = HashMap::new();
        let sw_ids: Vec<String> = parse_registry(ini, "SuperWeaponTypes");
        for sw_id in &sw_ids {
            if let Some(section) = ini.section(sw_id) {
                if let Some(sw) = SuperWeaponType::from_ini_section(sw_id, section) {
                    super_weapons.insert(sw_id.clone(), sw);
                } else {
                    log::warn!("SuperWeapon '{}' has unknown Type=, skipping", sw_id);
                }
            } else {
                log::trace!(
                    "SuperWeapon '{}' listed in [SuperWeaponTypes] but has no section",
                    sw_id
                );
            }
        }
        log::info!("SuperWeaponTypes: {} loaded", super_weapons.len());

        // Parse [CombatDamage] defaults (particle-system fallbacks).
        let combat_damage: CombatDamageDefaults = ini
            .section("CombatDamage")
            .map(CombatDamageDefaults::from_ini_section)
            .unwrap_or_default();

        // Parse [CombatDamage] bridge-warhead names (IonCannonWarhead, C4Warhead).
        let bridge_warheads = ini
            .section("CombatDamage")
            .map(crate::rules::bridge_warheads::BridgeWarheads::from_ini_section)
            .unwrap_or_default();

        // [CombatDamage] C4Delay = minutes (double). Default 0.03 = 27 ticks @ 15 fps.
        // Stored as integer ticks for lockstep-safe per-tick comparison.
        const SIM_TICKS_PER_SECOND: u32 = 15;
        let c4_delay_ticks: u32 = ini
            .section("CombatDamage")
            .and_then(|s| s.get("C4Delay"))
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|minutes| (minutes * 60.0 * SIM_TICKS_PER_SECOND as f64).round() as u32)
            .unwrap_or(27); // 0.03 × 60 × 15 = 27

        // Parse [TerrainTypes] registry → per-type sections (TIBTRE01, TREE01, etc.).
        let mut terrain_object_types: HashMap<String, TerrainObjectType> = HashMap::new();
        let terrain_names: Vec<String> = parse_registry(ini, "TerrainTypes");
        for name in &terrain_names {
            if let Some(section) = ini.section(name) {
                terrain_object_types.insert(
                    name.to_ascii_uppercase(),
                    TerrainObjectType::from_ini_section(name, section),
                );
            }
        }
        log::info!(
            "TerrainTypes: {} loaded ({} with SpawnsTiberium=yes)",
            terrain_object_types.len(),
            terrain_object_types
                .values()
                .filter(|t| t.spawns_tiberium)
                .count(),
        );

        // Step 9: Two-pass parse for [Particles] and [ParticleSystems].
        // Cross-references (NextParticle, HoldsWhat) are resolved in pass 2 so
        // that INI ordering does not matter.
        let (particle_types, particle_types_by_name) = parse_particle_types(ini);
        let (particle_system_types, particle_system_types_by_name) =
            parse_particle_system_types(ini, &particle_types_by_name);

        log::info!(
            "RuleSet loaded: {} objects ({} inf, {} veh, {} air, {} bld), \
             {} weapons, {} warheads, {} projectiles, \
             {} particle types, {} particle system types",
            objects.len(),
            infantry_ids.len(),
            vehicle_ids.len(),
            aircraft_ids.len(),
            building_ids.len(),
            weapons.len(),
            warheads.len(),
            projectiles.len(),
            particle_types.len(),
            particle_system_types.len()
        );

        Ok(RuleSet {
            objects,
            weapons,
            warheads,
            projectiles,
            production,
            general,
            infantry_ids,
            vehicle_ids,
            aircraft_ids,
            building_ids,
            factory_map,
            prerequisite_groups,
            terrain_rules,
            terrain_object_types,
            bridge_rules,
            garrison_rules,
            radar_event_config,
            super_weapons,
            combat_damage,
            bridge_warheads,
            c4_delay_ticks,
            particle_types,
            particle_types_by_name,
            particle_system_types,
            particle_system_types_by_name,
            smudge_types: SmudgeTypeRegistry::from_rules_ini(ini),
            art_registry: crate::rules::art_data::ArtRegistry::empty(),
            ion_cannon_warhead_id: None,
            c4_warhead_id: None,
        })
    }

    /// Look up a game object by ID.
    /// Intern all known type IDs (infantry, vehicle, aircraft, building) into
    /// the given interner. Ensures that `interner.get(type_id)` succeeds for
    /// any type referenced by this ruleset.
    pub fn intern_all_ids(&self, interner: &mut crate::sim::intern::StringInterner) {
        for id in &self.infantry_ids {
            interner.intern(id);
        }
        for id in &self.vehicle_ids {
            interner.intern(id);
        }
        for id in &self.aircraft_ids {
            interner.intern(id);
        }
        for id in &self.building_ids {
            interner.intern(id);
        }
    }

    /// Resolve `[CombatDamage] IonCannonWarhead=` and `C4Warhead=` against the
    /// simulation interner. Call once at sim init after the warhead registry
    /// is populated and before any combat tick.
    pub fn resolve_bridge_warheads(&mut self, interner: &mut crate::sim::intern::StringInterner) {
        self.ion_cannon_warhead_id = Some(interner.intern(&self.bridge_warheads.ion_cannon_name));
        self.c4_warhead_id = Some(interner.intern(&self.bridge_warheads.c4_name));
    }

    /// Pre-resolved IonCannonWarhead InternedId.
    ///
    /// # Panics
    /// Panics if `resolve_bridge_warheads` has not been called.
    pub fn ion_cannon_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.ion_cannon_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called at sim init \
             before combat reads warhead IDs",
        )
    }

    /// Pre-resolved C4Warhead InternedId.
    ///
    /// # Panics
    /// Panics if `resolve_bridge_warheads` has not been called.
    pub fn c4_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.c4_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called at sim init \
             before bridge cascade fires",
        )
    }

    pub fn object(&self, id: &str) -> Option<&ObjectType> {
        self.objects.get(id)
    }

    /// Look up a game object by ID case-insensitively.
    pub fn object_case_insensitive(&self, id: &str) -> Option<&ObjectType> {
        self.objects.get(id).or_else(|| {
            self.objects
                .iter()
                .find_map(|(key, obj)| key.eq_ignore_ascii_case(id).then_some(obj))
        })
    }

    /// Look up a TerrainObjectType by section name, case-insensitive.
    pub fn terrain_object_type_case_insensitive(&self, name: &str) -> Option<&TerrainObjectType> {
        self.terrain_object_types.get(&name.to_ascii_uppercase())
    }

    /// Look up a weapon by ID.
    pub fn weapon(&self, id: &str) -> Option<&WeaponType> {
        self.weapons.get(id)
    }

    /// Look up a warhead by ID.
    pub fn warhead(&self, id: &str) -> Option<&WarheadType> {
        self.warheads.get(id)
    }

    /// Look up a projectile by ID.
    pub fn projectile(&self, id: &str) -> Option<&ProjectileType> {
        self.projectiles.get(id)
    }

    /// Look up a superweapon type by ID.
    pub fn super_weapon(&self, id: &str) -> Option<&SuperWeaponType> {
        self.super_weapons.get(id)
    }

    /// Look up a particle type by ID. Panics if `id` is out of range.
    pub fn particle_type(&self, id: ParticleTypeId) -> &ParticleType {
        &self.particle_types[id.0 as usize]
    }

    /// Iterate every parsed `[Particles]` definition.
    pub fn particle_types_iter(&self) -> impl Iterator<Item = &ParticleType> {
        self.particle_types.iter()
    }

    /// Look up a particle system type by ID. Panics if `id` is out of range.
    pub fn particle_system_type(&self, id: ParticleSystemTypeId) -> &ParticleSystemType {
        &self.particle_system_types[id.0 as usize]
    }

    /// Resolve a particle type name to its ID (case-insensitive).
    pub fn p_type_id_by_name(&self, name: &str) -> Option<ParticleTypeId> {
        self.particle_types_by_name
            .get(&name.to_ascii_uppercase())
            .copied()
    }

    /// Resolve a particle system type name to its ID (case-insensitive).
    pub fn ps_type_id_by_name(&self, name: &str) -> Option<ParticleSystemTypeId> {
        self.particle_system_types_by_name
            .get(&name.to_ascii_uppercase())
            .copied()
    }

    /// Number of particle types loaded from `[Particles]`.
    pub fn particle_type_count(&self) -> usize {
        self.particle_types.len()
    }

    /// Number of particle system types loaded from `[ParticleSystems]`.
    pub fn particle_system_type_count(&self) -> usize {
        self.particle_system_types.len()
    }

    /// Look up the factory type for a structure by ID (case-insensitive).
    /// Returns None if the structure has no Factory= key in rules.ini.
    pub fn factory_type(&self, structure_id: &str) -> Option<FactoryType> {
        self.factory_map
            .get(&structure_id.to_ascii_uppercase())
            .copied()
    }

    /// Look up which building IDs satisfy a prerequisite alias (case-insensitive).
    /// Returns None if the alias is not a known prerequisite group.
    pub fn prerequisite_group(&self, alias: &str) -> Option<&[String]> {
        self.prerequisite_groups
            .get(&alias.to_ascii_uppercase())
            .map(|v| v.as_slice())
    }

    /// Whether a structure type is marked as a refinery in rules.ini.
    pub fn is_refinery_type(&self, structure_id: &str) -> bool {
        self.object_case_insensitive(structure_id)
            .is_some_and(|obj| obj.refinery)
    }

    /// Whether a structure type is a repair depot (UnitRepair=yes in rules.ini).
    pub fn is_repair_depot(&self, structure_id: &str) -> bool {
        self.object_case_insensitive(structure_id)
            .is_some_and(|obj| obj.unit_repair)
    }

    /// Resolve a refinery's free starter unit if both the refinery and the unit exist.
    pub fn refinery_free_unit(&self, structure_id: &str) -> Option<&str> {
        let obj = self.object_case_insensitive(structure_id)?;
        if !obj.refinery {
            return None;
        }
        let free_unit = obj.free_unit.as_deref()?;
        let resolved = self.object_case_insensitive(free_unit)?;
        Some(resolved.id.as_str())
    }

    /// Whether a harvester type may dock at a specific structure according to Dock=.
    pub fn harvester_can_dock_at(&self, harvester_id: &str, structure_id: &str) -> bool {
        let Some(harvester) = self.object_case_insensitive(harvester_id) else {
            return false;
        };
        let Some(_structure) = self.object_case_insensitive(structure_id) else {
            return false;
        };
        harvester
            .dock
            .iter()
            .any(|dock| dock.eq_ignore_ascii_case(structure_id))
    }

    /// Merge art.ini data into object types (Foundation, QueueingCell, DockingOffset).
    ///
    /// In the original engine, `Foundation=` is an **art.ini-only** property — it does
    /// NOT exist in rules.ini. ObjectType defaults to "1x1" and this method overwrites
    /// it with the authoritative value from art.ini, resolved via the `Image=` key.
    /// Without this, all buildings would be 1x1 which breaks placement and rendering.
    pub fn merge_art_data(&mut self, art: &crate::rules::art_data::ArtRegistry) {
        let mut patched: u32 = 0;
        let mut dock_patched: u32 = 0;
        let mut buildings_checked: u32 = 0;
        let mut infantry_checked: u32 = 0;
        let mut crawls_patched: u32 = 0;
        for obj in self.objects.values_mut() {
            // Resolve the art.ini section: use Image= override if present,
            // otherwise fall back to the object ID itself.
            let art_key: &str = &obj.image;
            let entry = art.get(art_key).or_else(|| art.get(&obj.id));
            if obj.category == crate::rules::object_type::ObjectCategory::Infantry {
                infantry_checked += 1;
                if let Some(entry) = entry {
                    obj.crawls = entry.crawls;
                    obj.fire_up_frame = entry.fire_up;
                    obj.fire_prone_frame = entry.fire_prone;
                    obj.secondary_fire_frame = entry.secondary_fire;
                    obj.secondary_prone_frame = entry.secondary_prone;
                    if entry.crawls {
                        crawls_patched += 1;
                    }
                }
                continue;
            }
            if obj.category != crate::rules::object_type::ObjectCategory::Building {
                continue;
            }
            buildings_checked += 1;
            if let Some(entry) = entry {
                if let Some(ref foundation) = entry.foundation {
                    if obj.foundation != *foundation {
                        log::trace!(
                            "Foundation patch: {} (image={}) {} → {}",
                            obj.id,
                            art_key,
                            obj.foundation,
                            foundation,
                        );
                    }
                    obj.foundation = foundation.clone();
                    patched += 1;
                }
                // Merge QueueingCell from art.ini (TibSun legacy dock system).
                if entry.queueing_cell.is_some() {
                    obj.queueing_cell = entry.queueing_cell;
                    dock_patched += 1;
                }
                // Multi-pad merge: when art declares at least one DockingOffset,
                // size pads to NumberOfDocks (from rules.ini), zero-padding missing
                // indices and truncating excess. Mirrors the original game's
                // memory layout where the array is sized by NumberOfDocks and
                // unspecified DockingOffset%d slots default to (0,0,0).
                //
                // When art declares ZERO DockingOffset entries (retail refineries
                // like GAREFN/NAREFN/YAREFN), obj.pads is left empty so existing
                // fallback paths (e.g. refinery_pad_cell's rightmost-column
                // anchor) keep firing. Otherwise zero-padding would silently
                // shift refinery dock positions, which is out of scope here.
                if !entry.pads.is_empty() {
                    let n = obj.number_of_docks as usize;
                    obj.pads = entry.pads.iter().take(n).copied().collect();
                    while obj.pads.len() < n {
                        obj.pads.push(crate::rules::object_type::DockPad {
                            lepton_offset: (0, 0, 0),
                        });
                    }
                }
                // Merge AddOccupy/RemoveOccupy from art.ini.
                if !entry.add_occupy.is_empty() {
                    obj.add_occupy = entry.add_occupy.clone();
                }
                if !entry.remove_occupy.is_empty() {
                    obj.remove_occupy = entry.remove_occupy.clone();
                }
            }
        }
        log::info!(
            "Merged art.ini → RuleSet: {} foundations, {} dock cells ({} buildings checked)",
            patched,
            dock_patched,
            buildings_checked,
        );
        log::trace!(
            "Merged infantry art metadata: {} Crawls flags ({} infantry checked)",
            crawls_patched,
            infantry_checked,
        );
    }

    /// Total number of game objects across all categories.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Total number of weapons.
    pub fn weapon_count(&self) -> usize {
        self.weapons.len()
    }

    /// Total number of warheads.
    pub fn warhead_count(&self) -> usize {
        self.warheads.len()
    }

    /// Iterate all parsed warhead types.
    pub fn warheads_iter(&self) -> impl Iterator<Item = &WarheadType> {
        self.warheads.values()
    }

    /// Iterate all parsed weapon types.
    pub fn weapons_iter(&self) -> impl Iterator<Item = &WeaponType> {
        self.weapons.values()
    }

    /// Total number of projectiles.
    pub fn projectile_count(&self) -> usize {
        self.projectiles.len()
    }

    /// Iterate over all game objects in the registry.
    pub fn all_objects(&self) -> impl Iterator<Item = &ObjectType> {
        self.objects.values()
    }
}

/// Parse a type registry section (e.g., [InfantryTypes]) into a list of IDs.
///
/// Registry sections use numbered keys: `0=E1`, `1=E2`, ...
/// Returns empty Vec if the section doesn't exist.
fn parse_registry(ini: &IniFile, section_name: &str) -> Vec<String> {
    match ini.section(section_name) {
        Some(section) => {
            let raw: Vec<String> = section
                .get_values()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            // Deduplicate: rules.ini + rulesmd.ini merge can produce the same
            // type ID at different numbered keys (e.g., 42=GAAIRC in base and
            // 150=GAAIRC in YR patch). Keep first occurrence, preserve order.
            let mut seen: std::collections::HashSet<String> =
                std::collections::HashSet::with_capacity(raw.len());
            let before = raw.len();
            let deduped: Vec<String> = raw
                .into_iter()
                .filter(|id| seen.insert(id.to_ascii_uppercase()))
                .collect();
            let removed = before - deduped.len();
            if removed > 0 {
                log::info!(
                    "Registry [{}]: removed {} duplicate entries",
                    section_name,
                    removed,
                );
            }
            deduped
        }
        None => {
            log::warn!("Registry section [{}] not found in rules.ini", section_name);
            Vec::new()
        }
    }
}

/// Collect all weapon and warhead IDs referenced by objects.
///
/// Returns (weapon_ids, warhead_ids) as sets (deduplicated).
fn collect_weapon_refs(
    objects: &HashMap<String, ObjectType>,
) -> (HashSet<String>, HashSet<String>) {
    let mut weapon_ids: HashSet<String> = HashSet::new();
    let warhead_ids: HashSet<String> = HashSet::new();

    for obj in objects.values() {
        if let Some(ref w) = obj.primary {
            weapon_ids.insert(w.clone());
        }
        if let Some(ref w) = obj.secondary {
            weapon_ids.insert(w.clone());
        }
        if let Some(ref w) = obj.elite_primary {
            weapon_ids.insert(w.clone());
        }
        if let Some(ref w) = obj.elite_secondary {
            weapon_ids.insert(w.clone());
        }
        if let Some(ref w) = obj.occupy_weapon {
            weapon_ids.insert(w.clone());
        }
        if let Some(ref w) = obj.elite_occupy_weapon {
            weapon_ids.insert(w.clone());
        }
    }

    (weapon_ids, warhead_ids)
}

/// Parse prerequisite alias groups from [General] PrerequisiteXxx keys.
///
/// RA2's rules.ini defines abstract prerequisite names (POWER, RADAR, etc.)
/// that map to lists of concrete building IDs. For example:
///   PrerequisitePower=GAPOWR,NAPOWR,NANRCT
/// means any unit with `Prerequisite=POWER` is satisfied by owning any of those.
///
/// Also registers secondary aliases used in RA2 prerequisites:
/// - FACTORY / WARFACTORY → same as PrerequisiteFactory list
/// - BARRACKS / TENT → same as PrerequisiteBarracks list
fn parse_prerequisite_groups(ini: &IniFile) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let Some(general) = ini.section("General") else {
        return groups;
    };

    /// Known [General] keys and the alias name they define.
    const PREREQ_KEYS: &[(&str, &str)] = &[
        ("PrerequisitePower", "POWER"),
        ("PrerequisiteProc", "PROC"),
        ("PrerequisiteRadar", "RADAR"),
        ("PrerequisiteTech", "TECH"),
        ("PrerequisiteBarracks", "BARRACKS"),
        ("PrerequisiteFactory", "FACTORY"),
    ];

    for &(ini_key, alias) in PREREQ_KEYS {
        if let Some(list) = general.get_list(ini_key) {
            let ids: Vec<String> = list
                .into_iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_uppercase())
                .collect();
            if !ids.is_empty() {
                groups.insert(alias.to_string(), ids);
            }
        }
    }

    // ProcAlternate entries merge into the PROC group.
    if let Some(list) = general.get_list("PrerequisiteProcAlternate") {
        let alt_ids: Vec<String> = list
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_uppercase())
            .collect();
        if !alt_ids.is_empty() {
            groups
                .entry("PROC".to_string())
                .or_default()
                .extend(alt_ids);
        }
    }

    // Register secondary aliases that RA2 prerequisites use interchangeably.
    if let Some(factory_list) = groups.get("FACTORY").cloned() {
        groups.insert("WARFACTORY".to_string(), factory_list);
    }
    if let Some(barracks_list) = groups.get("BARRACKS").cloned() {
        groups.insert("TENT".to_string(), barracks_list);
    }

    groups
}

/// Two-pass parse of `[Particles]`: collect `Pending` entries from each
/// referenced section, then resolve each `NextParticle=` name to a
/// `ParticleTypeId`. Missing references log a warning and stay `None`.
fn parse_particle_types(ini: &IniFile) -> (Vec<ParticleType>, HashMap<String, ParticleTypeId>) {
    let ids: Vec<String> = parse_registry(ini, "Particles");
    if ids.is_empty() {
        return (Vec::new(), HashMap::new());
    }

    // Pass 1: parse each section into PendingParticleType. Skip IDs whose
    // section is missing — matches the behavior used elsewhere in this file.
    let mut pending: Vec<PendingParticleType> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(section) = ini.section(id) {
            pending.push(ParticleType::from_ini_section_pending(id, section));
        } else {
            log::trace!(
                "ParticleType '{}' listed in [Particles] but has no section",
                id
            );
        }
    }

    // Build the name → ID map (uppercase keys for case-insensitive lookup).
    let mut by_name: HashMap<String, ParticleTypeId> = HashMap::with_capacity(pending.len());
    for (idx, p) in pending.iter().enumerate() {
        by_name.insert(
            p.partial.name.to_ascii_uppercase(),
            ParticleTypeId(idx as u32),
        );
    }

    // Pass 2: resolve NextParticle references.
    let particle_types: Vec<ParticleType> = pending
        .into_iter()
        .map(|p| {
            let mut partial = p.partial;
            if let Some(ref next_name) = p.next_particle_name {
                let key = next_name.to_ascii_uppercase();
                match by_name.get(&key) {
                    Some(&id) => partial.next_particle = Some(id),
                    None => {
                        log::warn!(
                            "ParticleType '{}': NextParticle='{}' references unknown particle, leaving unresolved",
                            partial.name,
                            next_name
                        );
                    }
                }
            }
            partial
        })
        .collect();

    log::info!("Particles: {} loaded", particle_types.len());
    (particle_types, by_name)
}

/// Two-pass parse of `[ParticleSystems]`: collect `Pending` entries and
/// resolve each `HoldsWhat=` name against the already-built particle-type
/// name map. Missing references log a warning and stay `None`.
fn parse_particle_system_types(
    ini: &IniFile,
    p_by_name: &HashMap<String, ParticleTypeId>,
) -> (
    Vec<ParticleSystemType>,
    HashMap<String, ParticleSystemTypeId>,
) {
    let ids: Vec<String> = parse_registry(ini, "ParticleSystems");
    if ids.is_empty() {
        return (Vec::new(), HashMap::new());
    }

    let mut pending: Vec<PendingParticleSystemType> = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(section) = ini.section(id) {
            pending.push(ParticleSystemType::from_ini_section_pending(id, section));
        } else {
            log::trace!(
                "ParticleSystemType '{}' listed in [ParticleSystems] but has no section",
                id
            );
        }
    }

    let mut by_name: HashMap<String, ParticleSystemTypeId> = HashMap::with_capacity(pending.len());
    for (idx, pst) in pending.iter().enumerate() {
        by_name.insert(
            pst.partial.name.to_ascii_uppercase(),
            ParticleSystemTypeId(idx as u32),
        );
    }

    let particle_system_types: Vec<ParticleSystemType> = pending
        .into_iter()
        .map(|pst| {
            let mut partial = pst.partial;
            if let Some(ref holds_name) = pst.holds_what_name {
                let key = holds_name.to_ascii_uppercase();
                match p_by_name.get(&key) {
                    Some(&id) => partial.holds_what = Some(id),
                    None => {
                        log::warn!(
                            "ParticleSystemType '{}': HoldsWhat='{}' references unknown particle, leaving unresolved",
                            partial.name,
                            holds_name
                        );
                    }
                }
            }
            partial
        })
        .collect();

    log::info!("ParticleSystems: {} loaded", particle_system_types.len());
    (particle_system_types, by_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal rules.ini string for testing.
    fn make_test_rules() -> String {
        "\
[InfantryTypes]
0=E1
1=E2

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]
0=MTNK

[AircraftTypes]

[BuildingTypes]
0=GAPOWR

[E1]
Name=GI
Cost=200
Strength=125
Armor=flak
Speed=4
Primary=M60
BuildTimeMultiplier=1.15

[E2]
Name=Conscript
Cost=100
Strength=100
Armor=flak
Speed=4
Primary=INTL

[MTNK]
Name=Grizzly
Cost=700
Strength=300
Armor=heavy
Speed=6
Primary=105mm
Secondary=MachGun

[GAPOWR]
Name=Power Plant
Cost=800
Strength=750
Power=200
Foundation=2x2

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[INTL]
Damage=20
ROF=20
Range=4.75
Warhead=SA

[105mm]
Damage=65
ROF=50
Range=5.75
Speed=40
Projectile=InvisibleLow
Warhead=AP
Burst=2

[MachGun]
Damage=20
ROF=15
Range=5
Projectile=InvisibleLow
Warhead=SA

[InvisibleLow]
AA=no
AG=yes

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0

[AP]
Verses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%
CellSpread=0
"
        .to_string()
    }

    #[test]
    fn test_load_ruleset() {
        let ini: IniFile = IniFile::from_str(&make_test_rules());
        let rules: RuleSet = RuleSet::from_ini(&ini).expect("Should parse");

        assert_eq!(rules.infantry_ids.len(), 2);
        assert_eq!(rules.vehicle_ids.len(), 1);
        assert_eq!(rules.aircraft_ids.len(), 0);
        assert_eq!(rules.building_ids.len(), 1);
        assert_eq!(rules.object_count(), 4); // E1, E2, MTNK, GAPOWR
        assert!((rules.production.build_speed - 0.75).abs() < 0.0001);
        assert!((rules.production.multiple_factory - 0.7).abs() < 0.0001);
        assert!((rules.production.low_power_penalty_modifier - 1.25).abs() < 0.0001);
        assert!((rules.production.min_low_power_production_speed - 0.4).abs() < 0.0001);
        assert!((rules.production.max_low_power_production_speed - 0.85).abs() < 0.0001);
        assert_eq!(rules.bridge_rules.strength, 1500);
        assert!(rules.bridge_rules.destroyable_by_default);
    }

    #[test]
    fn parse_tier1_superweapon_rules() {
        let ini_text = "[General]\n\
ForceShieldRadius=5\n\
ForceShieldDuration=600\n\
PsychicRevealRadius=12\n\
MutateExplosion=no\n\
[CombatDamage]\n\
IronCurtainDuration=900\n\
MutateWarhead=MyMutate\n\
";
        let ini = IniFile::from_str(ini_text);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.iron_curtain_duration, 900);
        assert_eq!(general.force_shield_radius, 5);
        assert_eq!(general.force_shield_duration, 600);
        assert_eq!(general.psychic_reveal_radius, 12);
        assert_eq!(general.mutate_warhead, "MyMutate");
        assert!(!general.mutate_explosion);
        // Unspecified keys fall back to defaults.
        assert_eq!(general.iron_curtain_invoke_anim, "IRONBLST");
        assert_eq!(general.force_shield_invoke_anim, "FORCSHLD");
        assert_eq!(general.mutate_explosion_warhead, "MutateExplosion");
    }

    #[test]
    fn parse_rules_rocking_coefficients_defaults() {
        // [General] must be present, otherwise GeneralRules::from_ini bails to
        // Self::default(). Missing AudioVisual keys then fall back to defaults.
        let ini = IniFile::from_str("[General]\n[AudioVisual]\n");
        let r = GeneralRules::from_ini(&ini);
        assert_eq!(r.direct_rocking_coefficient, SimFixed::lit("1.5"));
        assert_eq!(r.fallback_coefficient, SimFixed::lit("0.1"));
    }

    #[test]
    fn parse_rules_rocking_coefficients_explicit() {
        let ini = IniFile::from_str(
            "[General]\n[AudioVisual]\nDirectRockingCoefficient=2.0\nFallBackCoefficient=0.05\n",
        );
        let r = GeneralRules::from_ini(&ini);
        assert_eq!(r.direct_rocking_coefficient, SimFixed::lit("2"));
        assert_eq!(r.fallback_coefficient, SimFixed::lit("0.05"));
    }

    #[test]
    fn parse_retail_rules_rocking_coefficients() {
        let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
        let ini = IniFile::from_str(&ini_text);
        let r = GeneralRules::from_ini(&ini);
        assert_eq!(r.direct_rocking_coefficient, SimFixed::lit("1.5"));
        assert_eq!(r.fallback_coefficient, SimFixed::lit("0.1"));
    }

    #[test]
    fn test_object_lookup() {
        let ini: IniFile = IniFile::from_str(&make_test_rules());
        let rules: RuleSet = RuleSet::from_ini(&ini).expect("Should parse");

        let e1: &ObjectType = rules.object("E1").expect("E1 exists");
        assert_eq!(e1.cost, 200);
        assert_eq!(e1.strength, 125);
        assert_eq!(e1.category, ObjectCategory::Infantry);
        assert_eq!(e1.primary, Some("M60".to_string()));
        assert!((e1.build_time_multiplier - 1.15).abs() < 0.0001);

        let mtnk: &ObjectType = rules.object("MTNK").expect("MTNK exists");
        assert_eq!(mtnk.cost, 700);
        assert_eq!(mtnk.category, ObjectCategory::Vehicle);
        assert_eq!(mtnk.secondary, Some("MachGun".to_string()));

        let gapowr: &ObjectType = rules.object("GAPOWR").expect("GAPOWR exists");
        assert_eq!(gapowr.power, 200);
        assert_eq!(gapowr.foundation, "2x2");
    }

    #[test]
    fn test_weapon_and_warhead_loading() {
        let ini: IniFile = IniFile::from_str(&make_test_rules());
        let rules: RuleSet = RuleSet::from_ini(&ini).expect("Should parse");

        // Weapons referenced by objects should be loaded.
        let m60: &WeaponType = rules.weapon("M60").expect("M60 exists");
        assert_eq!(m60.damage, 25);
        assert_eq!(m60.warhead, Some("SA".to_string()));

        let cannon: &WeaponType = rules.weapon("105mm").expect("105mm exists");
        assert_eq!(cannon.damage, 65);
        assert_eq!(cannon.warhead, Some("AP".to_string()));
        assert_eq!(cannon.burst, 2);
        assert_eq!(cannon.projectile, Some("InvisibleLow".to_string()));

        // Burst defaults to 1 when not specified.
        assert_eq!(m60.burst, 1);

        // Projectiles referenced by weapons should be loaded.
        assert_eq!(rules.projectile_count(), 1);
        let proj = rules
            .projectile("InvisibleLow")
            .expect("InvisibleLow exists");
        assert!(!proj.aa);
        assert!(proj.ag);

        // Warheads referenced by weapons should be loaded.
        let sa: &WarheadType = rules.warhead("SA").expect("SA exists");
        assert_eq!(sa.verses.len(), 11);
        assert_eq!(sa.verses[0], 100); // none: 100%
        assert_eq!(sa.verses[5], 25); // heavy: 25%

        let ap: &WarheadType = rules.warhead("AP").expect("AP exists");
        assert_eq!(ap.verses[6], 60); // wood: 60%
    }

    #[test]
    fn refinery_helpers_are_data_driven_and_case_insensitive() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             0=MODHARV\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=MODPROC\n\
             1=FAKEREF\n\
             [MODHARV]\n\
             Harvester=yes\n\
             Dock=modproc\n\
             [MODPROC]\n\
             Refinery=yes\n\
             FreeUnit=modharv\n\
             [FAKEREF]\n\
             Name=Fake Refinery\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");

        assert!(rules.is_refinery_type("modproc"));
        assert!(!rules.is_refinery_type("FAKEREF"));
        assert_eq!(rules.refinery_free_unit("MODPROC"), Some("MODHARV"));
        assert!(rules.harvester_can_dock_at("modharv", "MODPROC"));
        assert!(!rules.harvester_can_dock_at("modharv", "GAREFN"));
    }

    #[test]
    fn refinery_free_unit_ignores_missing_target() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=MODPROC\n\
             [MODPROC]\n\
             Refinery=yes\n\
             FreeUnit=UNKNOWN\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");

        assert!(rules.is_refinery_type("MODPROC"));
        assert_eq!(rules.refinery_free_unit("MODPROC"), None);
    }

    #[test]
    fn harvester_scan_radii_parsed_from_general() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [General]\n\
             TiberiumShortScan=10\n\
             TiberiumLongScan=60\n\
             SlaveMinerShortScan=12\n\
             SlaveMinerSlaveScan=20\n\
             SlaveMinerLongScan=55\n\
             SlaveMinerScanCorrection=5\n\
             SlaveMinerKickFrameDelay=200\n\
             HarvesterTooFarDistance=8\n\
             ChronoHarvTooFarDistance=40\n\
             PurifierBonus=.30\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");
        assert_eq!(rules.general.tiberium_short_scan, 10);
        assert_eq!(rules.general.tiberium_long_scan, 60);
        assert_eq!(rules.general.slave_miner_short_scan, 12);
        assert_eq!(rules.general.slave_miner_slave_scan, 20);
        assert_eq!(rules.general.slave_miner_long_scan, 55);
        assert_eq!(rules.general.slave_miner_scan_correction, 5);
        assert_eq!(rules.general.slave_miner_kick_frame_delay, 200);
        assert_eq!(rules.general.harvester_too_far_distance, 8);
        assert_eq!(rules.general.chrono_harv_too_far_distance, 40);
        assert_eq!(rules.general.purifier_bonus_pct, 30);
    }

    #[test]
    fn harvester_scan_radii_use_defaults_when_missing() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [General]\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");
        assert_eq!(rules.general.tiberium_short_scan, 6);
        assert_eq!(rules.general.tiberium_long_scan, 48);
        assert_eq!(rules.general.slave_miner_short_scan, 8);
        assert_eq!(rules.general.slave_miner_slave_scan, 14);
        assert_eq!(rules.general.slave_miner_long_scan, 48);
        assert_eq!(rules.general.slave_miner_scan_correction, 3);
        assert_eq!(rules.general.slave_miner_kick_frame_delay, 150);
        assert_eq!(rules.general.harvester_too_far_distance, 5);
        assert_eq!(rules.general.chrono_harv_too_far_distance, 50);
        assert_eq!(rules.general.purifier_bonus_pct, 25);
    }

    #[test]
    fn from_ini_loads_tibtre_terrain_object_types() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n1=TIBTRE01\n2=TREE01\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\n\
             AnimationRate=3\nAnimationProbability=.003\n\
             [TREE01]\nIsAnimated=no\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let t = rules
            .terrain_object_type_case_insensitive("tibtre01")
            .expect("TIBTRE01 should be parsed");
        assert!(t.spawns_tiberium);
        assert_eq!(t.animation_probability_micros, 3000);
        // TREE01 also parsed but with default flags.
        let tree = rules
            .terrain_object_type_case_insensitive("TREE01")
            .expect("TREE01 should be parsed");
        assert!(!tree.spawns_tiberium);
    }

    #[test]
    fn metallic_debris_default_matches_retail() {
        let g = GeneralRules::default();
        assert_eq!(g.metallic_debris.len(), 20);
        assert_eq!(g.metallic_debris[0], "DBRIS1LG");
        assert_eq!(g.metallic_debris[19], "DBRS10SM");
    }

    #[test]
    fn metallic_debris_parses_from_ini() {
        let ini = IniFile::from_str("[General]\nMetallicDebris=ANIM1,ANIM2,ANIM3\n");
        let g = GeneralRules::from_ini(&ini);
        assert_eq!(g.metallic_debris, vec!["ANIM1", "ANIM2", "ANIM3"]);
    }

    #[test]
    fn bridge_rules_load_from_ini() {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [General]\n\
             BridgeVoxelMax=5\n\
             [AudioVisual]\n\
             RepairBridgeSound=foo\n\
             [CombatDamage]\n\
             BridgeStrength=900\n\
             DestroyableBridges=no\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");
        assert_eq!(rules.bridge_rules.strength, 900);
        assert!(!rules.bridge_rules.destroyable_by_default);
        assert_eq!(rules.bridge_rules.voxel_max, 5);
        assert_eq!(rules.bridge_rules.repair_sound.as_deref(), Some("FOO"));
    }

    #[test]
    fn bridge_rules_voxel_max_clamps_oversize() {
        // Regression: u8 storage clamps oversize INI values to 255 instead
        // of wrapping/truncating.
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [General]\n\
             BridgeVoxelMax=999\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");
        assert_eq!(rules.bridge_rules.voxel_max, 255);
    }

    #[test]
    fn bridge_rules_destroyable_in_specialflags_is_ignored() {
        // Regression: gamemd reads DestroyableBridges from [CombatDamage].
        // The string in [SpecialFlags] is for MP-dialog overrides, not
        // the rules.ini parser. Putting it under [SpecialFlags] should
        // be silently ignored and the default (yes) kept.
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [SpecialFlags]\n\
             DestroyableBridges=no\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("Should parse");
        assert!(rules.bridge_rules.destroyable_by_default);
    }

    #[test]
    fn test_building_garrisoned_sound_parsed() {
        let ini_str = "\
[General]
[AudioVisual]
BuildingGarrisonedSound=BuildingGarrisoned
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(
            general.building_garrisoned_sound.as_deref(),
            Some("BuildingGarrisoned")
        );
    }

    #[test]
    fn test_gui_main_button_sound_parsed() {
        let ini_str = "\
[General]
[AudioVisual]
GUIMainButtonSound=MenuClick
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.gui_main_button_sound.as_deref(), Some("MenuClick"));
    }

    #[test]
    fn barrel_particle_parsed_from_general() {
        let ini_str = "\
[General]
BarrelParticle=SmallGreySSys
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.barrel_particle.as_deref(), Some("SmallGreySSys"));
    }

    #[test]
    fn barrel_particle_default_none() {
        let ini_str = "[General]\n";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert!(general.barrel_particle.is_none());
    }

    #[test]
    fn barrel_particle_ignored_under_audiovisual() {
        // Per report sec 11.8.H the key lives in [General], not [AudioVisual].
        // Verify the parser doesn't accidentally accept it elsewhere.
        let ini_str = "\
[General]
[AudioVisual]
BarrelParticle=SmallGreySSys
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert!(general.barrel_particle.is_none());
    }

    #[test]
    fn test_building_garrisoned_sound_default_none() {
        let ini_str = "\
[General]
[AudioVisual]
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert!(general.building_garrisoned_sound.is_none());
    }

    #[test]
    fn test_parachute_max_fall_rate_parsed() {
        let ini_str = "\
[General]
ParachuteMaxFallRate=-3
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.parachute_max_fall_rate, -3);
    }

    #[test]
    fn test_parachute_max_fall_rate_default_when_missing() {
        let ini_str = "[General]\n";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(
            general.parachute_max_fall_rate, -3,
            "default must be -3 per gamemd Rules+0x7B8"
        );
    }

    #[test]
    fn test_parachute_max_fall_rate_custom() {
        // Mod-friendliness: a non-default value must be respected, not clamped.
        let ini_str = "\
[General]
ParachuteMaxFallRate=-1
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.parachute_max_fall_rate, -1);
    }

    #[test]
    fn test_missile_rot_var_defaults_to_one() {
        let ini = IniFile::from_str("[General]\n");
        let general = GeneralRules::from_ini(&ini);
        assert_eq!(general.missile_rot_var, sim_from_f32(1.0));
    }

    #[test]
    fn test_missile_rot_var_parsed() {
        let ini = IniFile::from_str("[General]\nMissileROTVar=2.5\n");
        let general = GeneralRules::from_ini(&ini);
        let diff = (general.missile_rot_var - sim_from_f32(2.5)).abs();
        assert!(
            diff < SimFixed::lit("0.001"),
            "got {:?}",
            general.missile_rot_var
        );
    }

    #[test]
    fn test_building_garrisoned_sound_empty_treated_as_none() {
        let ini_str = "\
[General]
[AudioVisual]
BuildingGarrisonedSound=
";
        let ini = IniFile::from_str(ini_str);
        let general = GeneralRules::from_ini(&ini);
        assert!(general.building_garrisoned_sound.is_none());
    }

    fn make_particle_test_rules(extra: &str) -> String {
        // Minimal rules that load into a RuleSet — empty registries for the
        // unit categories so RuleSet::from_ini doesn't reject the input.
        format!(
            "\
[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[InfantryTypes]
[VehicleTypes]
[AircraftTypes]
[BuildingTypes]

{extra}",
        )
    }

    #[test]
    fn two_pass_resolves_next_particle_regardless_of_order() {
        let extra = "\
[Particles]
1=ChainEnd
2=ChainStart

[ChainStart]
NextParticle=ChainEnd
BehavesLike=Gas

[ChainEnd]
BehavesLike=Gas
";
        let ini = IniFile::from_str(&make_particle_test_rules(extra));
        let rs = RuleSet::from_ini(&ini).unwrap();
        let start_id = rs.p_type_id_by_name("ChainStart").unwrap();
        let end_id = rs.p_type_id_by_name("ChainEnd").unwrap();
        assert_eq!(rs.particle_type(start_id).next_particle, Some(end_id));
        assert_eq!(rs.particle_type(end_id).next_particle, None);
    }

    #[test]
    fn two_pass_resolves_holds_what() {
        let extra = "\
[Particles]
1=Smoke1

[ParticleSystems]
1=BigSmoke

[BigSmoke]
HoldsWhat=Smoke1
BehavesLike=Smoke

[Smoke1]
BehavesLike=Smoke
";
        let ini = IniFile::from_str(&make_particle_test_rules(extra));
        let rs = RuleSet::from_ini(&ini).unwrap();
        let s = rs.ps_type_id_by_name("BigSmoke").unwrap();
        let p = rs.p_type_id_by_name("Smoke1").unwrap();
        assert_eq!(rs.particle_system_type(s).holds_what, Some(p));
    }

    #[test]
    fn missing_reference_logs_and_leaves_none() {
        let extra = "\
[Particles]
1=GhostRef

[GhostRef]
NextParticle=DoesNotExist
BehavesLike=Gas
";
        let ini = IniFile::from_str(&make_particle_test_rules(extra));
        let rs = RuleSet::from_ini(&ini).unwrap();
        let id = rs.p_type_id_by_name("GhostRef").unwrap();
        assert_eq!(rs.particle_type(id).next_particle, None);
    }

    #[test]
    fn p_type_id_by_name_is_case_insensitive() {
        let extra = "\
[Particles]
1=GasCloud1

[GasCloud1]
BehavesLike=Gas
";
        let ini = IniFile::from_str(&make_particle_test_rules(extra));
        let rs = RuleSet::from_ini(&ini).unwrap();
        let a = rs.p_type_id_by_name("GasCloud1").unwrap();
        let b = rs.p_type_id_by_name("GASCLOUD1").unwrap();
        let c = rs.p_type_id_by_name("gascloud1").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn combat_damage_defaults_load_from_ini() {
        let extra = "\
[Particles]
1=Fp

[ParticleSystems]
1=FireStreamSys

[FireStreamSys]
BehavesLike=Fire

[Fp]
BehavesLike=Fire

[CombatDamage]
DefaultFireStreamSystem=FireStreamSys
DefaultSparkSystem=SparkSys
";
        let ini = IniFile::from_str(&make_particle_test_rules(extra));
        let rs = RuleSet::from_ini(&ini).unwrap();
        assert_eq!(
            rs.combat_damage.default_fire_stream_system.as_deref(),
            Some("FireStreamSys")
        );
        assert_eq!(
            rs.combat_damage.default_spark_system.as_deref(),
            Some("SparkSys")
        );
        // Other slots stay None when the key isn't present.
        assert!(rs.combat_damage.default_repair_particle_system.is_none());
    }

    #[test]
    fn combat_damage_defaults_when_section_absent() {
        let ini = IniFile::from_str(&make_particle_test_rules(""));
        let rs = RuleSet::from_ini(&ini).unwrap();
        assert!(rs.combat_damage.default_fire_stream_system.is_none());
        assert!(rs.combat_damage.default_spark_system.is_none());
    }

    #[test]
    fn empty_particles_section_leaves_registries_empty() {
        // Pre-existing rules without [Particles]/[ParticleSystems] still parse.
        let ini = IniFile::from_str(&make_particle_test_rules(""));
        let rs = RuleSet::from_ini(&ini).unwrap();
        assert_eq!(rs.particle_type_count(), 0);
        assert_eq!(rs.particle_system_type_count(), 0);
        assert_eq!(rs.p_type_id_by_name("Anything"), None);
        assert_eq!(rs.ps_type_id_by_name("Anything"), None);
    }

    fn ini_with_general(body: &str) -> IniFile {
        let text = format!("[General]\n{}\n", body);
        IniFile::from_str(&text)
    }

    #[test]
    fn paradrop_defaults_when_no_general_section() {
        let ini = IniFile::from_str("[Foo]\nBar=1\n");
        let g = GeneralRules::from_ini(&ini);
        assert_eq!(g.paradrop_radius, 1024);
        assert_eq!(g.paradrop_aircraft_type, "PDPLANE");
        assert_eq!(g.amer_paradrop_list, vec![("E1".to_string(), 8)]);
        assert_eq!(g.ally_paradrop_list, vec![("E1".to_string(), 6)]);
        assert_eq!(g.sov_paradrop_list, vec![("E2".to_string(), 9)]);
        assert_eq!(g.yuri_paradrop_list, vec![("INIT".to_string(), 6)]);
    }

    #[test]
    fn paradrop_explicit_values_parse() {
        let ini = ini_with_general(
            "ParadropRadius=2048\n\
             AmerParaDropInf=E1,GHOST,ENGINEER\n\
             AmerParaDropNum=6,6,6",
        );
        let g = GeneralRules::from_ini(&ini);
        assert_eq!(g.paradrop_radius, 2048);
        assert_eq!(
            g.amer_paradrop_list,
            vec![
                ("E1".to_string(), 6),
                ("GHOST".to_string(), 6),
                ("ENGINEER".to_string(), 6),
            ]
        );
    }

    #[test]
    fn paradrop_list_mismatch_falls_back_to_default() {
        let ini = ini_with_general(
            "AllyParaDropInf=E1,E2\n\
             AllyParaDropNum=5",
        );
        let g = GeneralRules::from_ini(&ini);
        assert_eq!(g.ally_paradrop_list, vec![("E1".to_string(), 6)]);
    }

    #[test]
    fn paradrop_soviet_branch_skips_count_assert() {
        // gamemd's Soviet dispatch path has no count-equality assert; mirror it.
        let ini = ini_with_general(
            "SovParaDropInf=E2,E3\n\
             SovParaDropNum=9",
        );
        let g = GeneralRules::from_ini(&ini);
        // zip up to the shorter length — only ("E2", 9) survives.
        assert_eq!(g.sov_paradrop_list, vec![("E2".to_string(), 9)]);
    }

    #[test]
    fn paradrop_weapon_rof_reaches_resolved_weapon() {
        // Verifies the Task 5 grounding question: does [ParaDropWeapon] ROF=130
        // flow through the weapon parser into rules.weapon("ParaDropWeapon").rof?
        // The parser only reads weapon sections referenced from an ObjectType's
        // Primary= / Secondary=, so we need a minimal aircraft entry that points
        // to ParaDropWeapon.
        let text = "\
[AircraftTypes]
1=PDPLANE

[PDPLANE]
Primary=ParaDropWeapon
Strength=400
Speed=15
Image=PDPLANE

[ParaDropWeapon]
Damage=60
ROF=130
Range=1
Projectile=Invisible
";
        let ini = IniFile::from_str(text);
        let rs = RuleSet::from_ini(&ini).expect("rules parse");
        let weapon = rs
            .weapon("ParaDropWeapon")
            .expect("ParaDropWeapon must reach the weapon registry");
        assert_eq!(weapon.rof, 130);
    }

    #[test]
    fn parses_parachute_render_config_from_artmd() {
        let rules_text = "\
[General]
Parachute=PARACH
";
        let art_text = "\
[PARACH]
Rate=400
LoopStart=20
LoopEnd=39
LoopCount=30
AltPalette=yes
ZAdjust=-10
";
        let rules_ini = IniFile::from_str(rules_text);
        let art_ini = IniFile::from_str(art_text);
        let mut general = GeneralRules::from_ini(&rules_ini);
        assert_eq!(general.parachute_shp.as_deref(), Some("PARACH"));
        general.resolve_art_rates(&art_ini);
        let pc = general
            .parachute_render
            .as_ref()
            .expect("parachute_render must be loaded");
        assert_eq!(pc.shp_name, "PARACH");
        // Rate=400 → (900/400) * 1000/15 = 2 * 1000/15 = 133.
        assert_eq!(pc.rate_ms, 133);
        assert_eq!(pc.loop_start, 20);
        assert_eq!(pc.end_frame, 40); // LoopEnd + 1
        assert_eq!(pc.z_adjust, -10);
        assert!(pc.alt_palette);
    }

    #[test]
    fn parachute_render_none_when_general_parachute_unset() {
        let rules_text = "[General]\nFlightLevel=1500\n";
        let art_text = "[PARACH]\nRate=400\n";
        let rules_ini = IniFile::from_str(rules_text);
        let art_ini = IniFile::from_str(art_text);
        let mut general = GeneralRules::from_ini(&rules_ini);
        assert!(general.parachute_shp.is_none());
        general.resolve_art_rates(&art_ini);
        assert!(general.parachute_render.is_none());
    }

    #[test]
    fn merge_art_propagates_add_remove_occupy() {
        let rules_text = format!(
            "{}\n[BuildingTypes]\n0=GAREFN\n[GAREFN]\nName=Refinery\nCost=2000\nFoundation=4x3\n",
            make_test_rules()
        );
        let art_text =
            "[GAREFN]\nFoundation=4x3\nAddOccupy1=-1,0\nAddOccupy2=-1,-1\nRemoveOccupy1=3,1\n";
        let rules_ini: IniFile = IniFile::from_str(&rules_text);
        let mut rules: RuleSet = RuleSet::from_ini(&rules_ini).expect("rules parse");
        let art_ini: IniFile = IniFile::from_str(art_text);
        let art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini);
        rules.merge_art_data(&art);
        let obj = rules.object("GAREFN").expect("GAREFN");
        assert_eq!(obj.add_occupy, vec![(-1, 0), (-1, -1)]);
        assert_eq!(obj.remove_occupy, vec![(3, 1)]);
    }

    #[test]
    fn merge_art_propagates_infantry_crawls_without_building_side_effects() {
        let rules_text = format!(
            "{}\n[InfantryTypes]\n0=E1\n\n[BuildingTypes]\n0=GAPOWR\n\n[E1]\nName=GI\nImage=GI\nStrength=125\nArmor=flak\nSpeed=4\n\n[GAPOWR]\nName=Power\nStrength=750\nArmor=wood\nFoundation=2x2\n",
            make_test_rules()
        );
        let rules_ini = IniFile::from_str(&rules_text);
        let mut rules = RuleSet::from_ini(&rules_ini).expect("rules parse");
        let art_ini = IniFile::from_str(
            "[GI]\nCrawls=yes\nFireUp=2\nFireProne=3\nSecondaryFire=4\nSecondaryProne=5\n\n[GAPOWR]\nCrawls=yes\nFireUp=9\n",
        );
        let art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini);
        rules.merge_art_data(&art);

        let infantry = rules.object("E1").expect("E1");
        assert!(infantry.crawls);
        assert_eq!(infantry.fire_up_frame, 2);
        assert_eq!(infantry.fire_prone_frame, 3);
        assert_eq!(infantry.secondary_fire_frame, 4);
        assert_eq!(infantry.secondary_prone_frame, 5);
        let building = rules.object("GAPOWR").expect("GAPOWR");
        assert!(!building.crawls);
        assert_eq!(building.fire_up_frame, 0);
        assert_eq!(building.foundation, "2x2");
    }

    #[test]
    fn resolve_bridge_warheads_populates_ids() {
        use crate::sim::intern::StringInterner;
        let ini: IniFile = IniFile::from_str(&make_test_rules());
        let mut rules: RuleSet = RuleSet::from_ini(&ini).expect("rules parse");
        let mut interner = StringInterner::default();
        rules.resolve_bridge_warheads(&mut interner);
        let ion_id = rules.ion_cannon_warhead_id();
        let c4_id = rules.c4_warhead_id();
        // Defaults match retail rulesmd.ini ("IonCannonWH" + "Super") because
        // the test rules.ini has no `[CombatDamage]` overrides.
        assert_eq!(interner.resolve(ion_id), "IonCannonWH");
        assert_eq!(interner.resolve(c4_id), "Super");
    }

    #[test]
    fn resolve_bridge_warheads_honors_combat_damage_overrides() {
        use crate::sim::intern::StringInterner;
        let rules_text = format!(
            "{}\n[CombatDamage]\nIonCannonWarhead=CustomIon\nC4Warhead=CustomC4\n",
            make_test_rules()
        );
        let ini: IniFile = IniFile::from_str(&rules_text);
        let mut rules: RuleSet = RuleSet::from_ini(&ini).expect("rules parse");
        let mut interner = StringInterner::default();
        rules.resolve_bridge_warheads(&mut interner);
        assert_eq!(interner.resolve(rules.ion_cannon_warhead_id()), "CustomIon");
        assert_eq!(interner.resolve(rules.c4_warhead_id()), "CustomC4");
    }

    #[test]
    #[should_panic(expected = "resolve_bridge_warheads")]
    fn ion_cannon_warhead_id_panics_before_resolve() {
        let ini: IniFile = IniFile::from_str(&make_test_rules());
        let rules: RuleSet = RuleSet::from_ini(&ini).expect("rules parse");
        let _ = rules.ion_cannon_warhead_id();
    }

    #[test]
    fn c4_delay_defaults_to_27_ticks() {
        let ini = IniFile::from_str("");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        assert_eq!(rules.c4_delay_ticks, 27);
    }

    #[test]
    fn c4_delay_parses_double_minutes_to_ticks() {
        let ini = IniFile::from_str("[CombatDamage]\nC4Delay=0.1\n");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        // 0.1 minutes × 60 × 15 = 90 ticks
        assert_eq!(rules.c4_delay_ticks, 90);
    }

    #[test]
    fn c4_delay_retail_default_value() {
        let ini = IniFile::from_str("[CombatDamage]\nC4Delay=0.03\n");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        // 0.03 × 60 × 15 = 27 (.round())
        assert_eq!(rules.c4_delay_ticks, 27);
    }

    #[test]
    fn retail_rulesmd_c4_flags_parse_correctly() {
        // Load the actual retail rulesmd.ini from the repo's ini/ directory.
        let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("ini/rulesmd.ini");
        let ini = IniFile::from_str(&ini_text);
        let rules = RuleSet::from_ini(&ini).expect("parse retail rulesmd");

        // C4-capable units must have c4=true.
        for unit in &["GHOST", "TANY", "PTROOP"] {
            let obj = rules
                .object(unit)
                .unwrap_or_else(|| panic!("no [{}]", unit));
            assert!(obj.c4, "[{}] must have c4=true (C4=yes in INI)", unit);
        }
        // Non-C4 infantry must have c4=false.
        for unit in &["E1", "ENGINEER", "CCOMAND"] {
            if let Some(obj) = rules.object(unit) {
                assert!(!obj.c4, "[{}] must have c4=false", unit);
            }
        }

        // CanC4-opt-out buildings — verified by direct grep of ini/rulesmd.ini
        // for `^CanC4=no`. Four sections match: CAMISC01, CAMISC02, CAMISC06,
        // AMMOCRAT. (The plan originally listed CAMSC09/CAMSC10 in error;
        // the retail INI does not set the flag on either.)
        for bld in &["CAMISC01", "CAMISC02", "CAMISC06", "AMMOCRAT"] {
            let obj = rules.object(bld).unwrap_or_else(|| panic!("no [{}]", bld));
            assert!(
                !obj.can_c4,
                "[{}] must have can_c4=false (CanC4=no in INI)",
                bld
            );
        }
        // Normal buildings inherit can_c4=true.
        for bld in &["GAPILE", "NAHAND", "GAREFN"] {
            if let Some(obj) = rules.object(bld) {
                assert!(obj.can_c4, "[{}] must have can_c4=true (default)", bld);
            }
        }

        // C4Delay must match the retail value (0.03 minutes = 27 ticks).
        assert_eq!(rules.c4_delay_ticks, 27, "C4Delay must parse to 27 ticks");
    }

    /// Helper: parse a (rules.ini, art.ini) pair into a merged RuleSet for
    /// pad-merge tests. Keeps a minimal scaffolding (one BuildingType) so
    /// `RuleSet::from_ini` does not reject the input.
    fn parse_rules_with_art(building_section: &str, art_ini: &str) -> RuleSet {
        let rules_str = format!(
            "[General]\n\
             BuildSpeed=1\n\
             MultipleFactory=1\n\
             LowPowerPenaltyModifier=1\n\
             MinLowPowerProductionSpeed=1\n\
             MaxLowPowerProductionSpeed=1\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=GAAIRC\n\
             {}",
            building_section,
        );
        let rules_ini = IniFile::from_str(&rules_str);
        let mut rules = RuleSet::from_ini(&rules_ini).expect("rules parse");
        let art_ini_parsed = IniFile::from_str(art_ini);
        let art = crate::rules::art_data::ArtRegistry::from_ini(&art_ini_parsed);
        rules.merge_art_data(&art);
        rules
    }

    #[test]
    fn merge_pads_zero_pads_missing_indices() {
        // NumberOfDocks=4 but art only has DockingOffset0,1.
        // Merge must produce pads.len() == 4 with indices 2,3 zero-init.
        let rules = parse_rules_with_art(
            "[GAAIRC]\nName=Airforce\nCost=1000\nStrength=1000\nNumberOfDocks=4\n",
            "[GAAIRC]\n\
             DockingOffset0=0,-128,0\n\
             DockingOffset1=0,128,0\n",
        );
        let obj = rules.object("GAAIRC").expect("obj");
        assert_eq!(obj.pads.len(), 4, "pads sized to NumberOfDocks");
        assert_eq!(obj.pads[0].lepton_offset, (0, -128, 0));
        assert_eq!(obj.pads[1].lepton_offset, (0, 128, 0));
        assert_eq!(
            obj.pads[2].lepton_offset,
            (0, 0, 0),
            "missing index 2 zero-init"
        );
        assert_eq!(
            obj.pads[3].lepton_offset,
            (0, 0, 0),
            "missing index 3 zero-init"
        );
    }

    #[test]
    fn merge_pads_truncates_excess_offsets() {
        // NumberOfDocks=2 but art has 4 offsets. Truncate.
        let rules = parse_rules_with_art(
            "[GAAIRC]\nName=Airforce\nCost=1000\nStrength=1000\nNumberOfDocks=2\n",
            "[GAAIRC]\n\
             DockingOffset0=0,0,0\n\
             DockingOffset1=128,0,0\n\
             DockingOffset2=256,0,0\n\
             DockingOffset3=384,0,0\n",
        );
        let obj = rules.object("GAAIRC").expect("obj");
        assert_eq!(obj.pads.len(), 2, "truncated to NumberOfDocks=2");
        assert_eq!(obj.pads[0].lepton_offset, (0, 0, 0));
        assert_eq!(obj.pads[1].lepton_offset, (128, 0, 0));
    }
}
