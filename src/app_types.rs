//! Shared type definitions and constants used across app_* modules.
//!
//! These types were extracted from app_render.rs because multiple sibling
//! modules (app_cursor, app_input, app_entity_pick, app_ui_overlays, etc.)
//! depend on them. Centralizing them here avoids coupling unrelated modules
//! to the rendering orchestration file.
//!
//! ## Dependency rules
//! - Part of the app layer — no sim/render dependencies.

use std::collections::HashMap;

use crate::render::batch::BatchTexture;

/// Background clear color — black, matching the shroud/fog of war in RA2.
/// Areas outside the isometric terrain diamond are not visible in the original game.
pub(crate) const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// Fixed deterministic simulation rate — re-exported from util::fixed_math.
pub(crate) const SIM_TICK_HZ: u32 = crate::util::fixed_math::SIM_TICK_HZ;
/// Integer tick duration used by deterministic step execution.
pub(crate) const SIM_TICK_MS: u32 = 1000 / SIM_TICK_HZ;
/// Verified retail/YR skirmish fallback from rulesmd.ini
/// `[MultiplayerDialogSettings] GameSpeed=1`.
pub(crate) const DEFAULT_YR_SKIRMISH_GAME_SPEED: u32 = 1;
const GAME_SPEED_BUCKET_MS: u32 = 16;

/// Approximate GameMD's single-player/skirmish throttle for the app-level
/// fixed-step scheduler. The stored speed byte is inverted relative to the
/// UI slider: `0=fastest`, `6=slowest`.
pub(crate) fn tps_for_game_speed(stored_speed: u32) -> u32 {
    if stored_speed == 0 {
        return 60;
    }
    let bucket_ms = stored_speed.saturating_mul(GAME_SPEED_BUCKET_MS).max(1);
    ((1000 + bucket_ms / 2) / bucket_ms).max(1)
}

pub(crate) fn default_yr_skirmish_tps() -> u32 {
    tps_for_game_speed(DEFAULT_YR_SKIRMISH_GAME_SPEED)
}
/// Next right-click order mode selected via hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderMode {
    Move,
    AttackMove,
    Guard,
}

/// Identifies a visual cursor from mouse.sha. Used as HashMap key in SoftwareCursor.
/// Frame ranges are hardcoded constants matching the vanilla RA2 exe (not INI-driven).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CursorId {
    Default,
    Select,
    Move,
    NoMove,
    Attack,
    AttackOutOfRange,
    AttackMove,
    Deploy,
    NoDeploy,
    // Directional scroll cursors (move-allowed).
    ScrollN,
    ScrollNE,
    ScrollE,
    ScrollSE,
    ScrollS,
    ScrollSW,
    ScrollW,
    ScrollNW,
    // Directional scroll cursors (can't-scroll-further).
    NoMoveN,
    NoMoveNE,
    NoMoveE,
    NoMoveSE,
    NoMoveS,
    NoMoveSW,
    NoMoveW,
    NoMoveNW,
    MinimapMove,
    Enter,
    NoEnter,
    EngineerRepair,
    TogglePower,
    NoTogglePower,
    /// 4-way scroll arrow for middle-mouse pan (frame 385 in mouse.sha).
    Pan,
    // Sell / repair mode cursors.
    Sell,
    SellUnit,
    NoSell,
    Repair,
    NoRepair,
    // Special unit cursors.
    DesolatorDeploy,
    GIDeploy,
    Crush,
    Tote,
    IvanBomb,
    Detonate,
    Demolish,
    Disarm,
    InfantryHeal,
    // Spy / infiltration cursors.
    Disguise,
    SpyTech,
    SpyPower,
    // Mind control cursors.
    MindControl,
    NoMindControl,
    RemoveSquid,
    InfantryAbsorb,
    // Superweapon cursors.
    Nuke,
    Chronosphere,
    IronCurtain,
    LightningStorm,
    Paradrop,
    ForceShield,
    NoForceShield,
    GeneticMutator,
    AirStrike,
    PsychicDominator,
    PsychicReveal,
    SpyPlane,
    Beacon,
}

/// All loaded cursor animation sequences from mouse.sha, keyed by CursorId.
pub(crate) struct SoftwareCursor {
    pub(crate) sequences: HashMap<CursorId, SoftwareCursorSequence>,
}

impl SoftwareCursor {
    /// Look up a cursor sequence by id, falling back to Default if not found.
    pub(crate) fn get(&self, id: CursorId) -> Option<&SoftwareCursorSequence> {
        self.sequences
            .get(&id)
            .or_else(|| self.sequences.get(&CursorId::Default))
    }
}

pub(crate) struct SoftwareCursorFrame {
    pub(crate) texture: BatchTexture,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) struct SoftwareCursorSequence {
    pub(crate) frames: Vec<SoftwareCursorFrame>,
    pub(crate) interval_ms: u64,
    pub(crate) hotspot: [f32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skirmish_speed_uses_verified_yr_stored_speed_one() {
        assert_eq!(DEFAULT_YR_SKIRMISH_GAME_SPEED, 1);
        assert_eq!(default_yr_skirmish_tps(), 63);
    }

    #[test]
    fn game_speed_one_is_not_the_old_speed_two_or_options_three_calibration() {
        let default = default_yr_skirmish_tps();
        assert_ne!(default, tps_for_game_speed(2));
        assert_ne!(default, tps_for_game_speed(3));
    }
}

/// Eight compass directions used for edge-scroll cursor selection.
/// Maps directly to the MoveN..MoveNW frames in mouse.sha (reference §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollDir {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorFeedbackKind {
    Move,
    AttackMove,
    Guard,
    FriendlyUnit,
    FriendlyStructure,
    EnemyUnit,
    EnemyStructure,
    EnemyOutOfRange,
    Invalid,
    PlaceValid,
    PlaceInvalid,
    /// Edge-scroll arrow — shown when cursor is near a screen edge.
    Scroll(ScrollDir),
    /// Move cursor minimap variant (frames 42–51) — shown when hovering over the minimap.
    MinimapMove,
    /// Deploy/undeploy cursor — shown when a Deployer unit hovers over itself.
    Deploy,
    /// Enter cursor — garrison, capture, board transport, sabotage.
    Enter,
    /// Engineer repair cursor — engineer hovering a damaged friendly building.
    EngineerRepair,
    /// C4 plant cursor — SEAL/Tanya/PTROOP hovering a CanC4 enemy structure
    /// (action 0x10 in gamemd, distinct mouse.shp frames from Enter).
    Demolish,
    /// Pan cursor — shown while middle-mouse dragging to scroll the map.
    Pan,
    /// Superweapon targeting reticle — shown while a charged SW is armed
    /// and the cursor is over the tactical map. Payload is the per-SW
    /// CursorId resolved from the `Action=` INI string.
    SuperWeaponTarget(CursorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoverTargetKind {
    FriendlyUnit,
    FriendlyStructure,
    EnemyUnit,
    EnemyStructure,
    HiddenEnemy,
}

/// Mutually-exclusive cursor-on-tactical-map targeting modes.
///
/// Building placement and superweapon targeting cannot both be active at
/// once. Arming one clears the other; right-click and Esc clear both.
/// The variant payload is the type_id (interned section name) the
/// targeting refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetingMode {
    /// Ready building waiting to be placed on the tactical map.
    /// Payload: building INI section name (e.g., "GAPOWR").
    BuildingPlacement(String),
    /// Charged superweapon waiting for a target cell.
    /// Payload: SW INI section name (e.g., "LightningStormSpecial").
    SuperWeapon(String),
}

impl TargetingMode {
    pub fn as_building_placement(&self) -> Option<&str> {
        match self {
            Self::BuildingPlacement(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_super_weapon(&self) -> Option<&str> {
        match self {
            Self::SuperWeapon(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn is_building_placement(&self) -> bool {
        matches!(self, Self::BuildingPlacement(_))
    }

    pub fn is_super_weapon(&self) -> bool {
        matches!(self, Self::SuperWeapon(_))
    }
}
