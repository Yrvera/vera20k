//! Sprite animation system — tracks per-entity animation state and frame timing.
//!
//! Manages animation sequences (stand, walk, attack, die) for SHP sprite entities.
//! Each entity with an `Animation` component has a current sequence, frame index,
//! and reached-frame counter. The `tick_animations()` function advances frames and
//! handles auto-transitions (e.g., stand ↔ walk based on MovementTarget).
//!
//! ## SHP frame layout
//! Infantry SHP files pack multiple sequences contiguously:
//! - Stand: 1 frame × 8 facings = frames 0–7
//! - Walk: 6 frames × 8 facings = frames 8–55
//! - Idle1: 15 frames (non-directional) = frames 56–70
//! - Die1: 15 frames (non-directional) = frames 86–100
//!
//! For directional sequences (facings > 1):
//!   `shp_frame = start + facing_index * frame_count + frame_within_sequence`
//! For non-directional sequences:
//!   `shp_frame = start + frame_within_sequence`
//!
//! ## Auto-transitions
//! - Entity gains MovementTarget → switch to Walk
//! - Entity loses MovementTarget → switch back to Stand
//! - Attack sequence finishes → switch to Stand (TransitionTo)
//! - Die sequence finishes → hold last frame (HoldLast)
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/components (MovementTarget, TypeRef).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

/// Standard number of facing directions for infantry animations.
const INFANTRY_FACINGS: u8 = 8;

/// Default reached native frames per standing image.
const DEFAULT_STAND_FRAME_DELAY: u16 = 1;

/// Default reached native frames per walk-cycle image.
const DEFAULT_WALK_FRAME_DELAY: u16 = 3;

/// Default reached native frames per idle-fidget image.
const DEFAULT_IDLE_FRAME_DELAY: u16 = 3;

/// Default reached native frames per death image.
const DEFAULT_DIE_FRAME_DELAY: u16 = 1;

/// Named animation sequence types.
///
/// Each corresponds to a range of frames in the SHP file, defined
/// by a `SequenceDef`. An entity plays one sequence at a time.
///
/// Maps to art.ini sequence keys: Ready/Guard → Stand, FireUp → Attack, etc.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SequenceKind {
    /// Standing alert pose. Default state for idle entities (Ready/Guard in INI).
    Stand,
    /// Walking movement cycle. Active while entity has a MovementTarget.
    Walk,
    /// Standing still while prone (Prone= in INI).
    Prone,
    /// Moving while prone (Crawl= in INI).
    Crawl,
    /// Firing primary weapon while standing (FireUp= in INI). Transitions to Stand.
    Attack,
    /// Firing primary weapon while prone (FireProne= in INI). Transitions to Stand.
    FireProne,
    /// Transition from standing to prone (Down= in INI). Transitions to Prone.
    Down,
    /// Transition from prone to standing (Up= in INI). Transitions to Stand.
    Up,
    /// Random fidget animation while idle (Idle1= in INI).
    Idle1,
    /// Second idle fidget variant (Idle2= in INI).
    Idle2,
    /// Death animation variant 1. Plays once, holds last frame.
    Die1,
    /// Death animation variant 2. Plays once, holds last frame.
    Die2,
    /// Death animation variant 3. Plays once, holds last frame.
    Die3,
    /// Death animation variant 4. Plays once, holds last frame.
    Die4,
    /// Death animation variant 5. Plays once, holds last frame.
    Die5,
    /// Victory/celebration animation (Cheer= in INI).
    Cheer,
    /// Parachute landing animation (Paradrop= in INI).
    Paradrop,
    /// Panicked running (Panic= in INI). Uses Walk-like timing.
    Panic,
    /// Transition from standing to deployed stance (Deploy= in INI).
    Deploy,
    /// Transition from deployed back to standing (Undeploy= in INI).
    Undeploy,
    /// Standing in deployed stance (Deployed= in INI, e.g., GI sandbags).
    Deployed,
    /// Firing while deployed (DeployedFire= in INI).
    DeployedFire,
    /// Idle fidget while deployed (DeployedIdle= in INI).
    DeployedIdle,
    /// Firing secondary weapon while standing (SecondaryFire= in INI, YR only).
    SecondaryFire,
    /// Firing secondary weapon while prone (SecondaryProne= in INI, YR only).
    SecondaryProne,
    /// Swimming movement cycle (Swim= in INI, e.g., Tanya in water).
    Swim,
    /// Flying movement cycle (Fly= in INI, e.g., Rocketeer).
    Fly,
    /// Firing while flying (FireFly= in INI).
    FireFly,
    /// Hovering in place (Hover= in INI).
    Hover,
    /// Treading water / ground cycle (Tread= in INI).
    Tread,
    /// Firing while swimming (WetAttack= in INI).
    WetAttack,
    /// Idle fidget while swimming variant 1 (WetIdle1= in INI).
    WetIdle1,
    /// Idle fidget while swimming variant 2 (WetIdle2= in INI).
    WetIdle2,
}

/// How a sequence behaves when it reaches its last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoopMode {
    /// Restart from frame 0 when reaching the end (walk, stand).
    Loop,
    /// Play once and freeze on the last frame (death animations).
    HoldLast,
    /// Play once then switch to a different sequence (attack → stand).
    TransitionTo(SequenceKind),
}

/// Which native facing-to-frame-slot rule a sequence follows.
///
/// The original engine has exactly two of these — one per SHP-bodied object
/// family — and they are separate code paths, not independent knobs. They
/// differ on *both* axes at once: which way the frame blocks rotate, and where
/// the quantizer puts its slot boundaries. Frame block 0 is screen-north
/// (cell NW) in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FacingSlots {
    /// Infantry bodies: a 32-entry lookup table indexed by the facing
    /// quantized to 32 steps. Slots run **counter-clockwise** from
    /// screen-north — NW=0, W=1, SW=2, S=3, SE=4, E=5, NE=6, N=7.
    #[default]
    InfantryTable,
    /// SHP vehicle bodies: the facing rounded to the nearest octant, then
    /// advanced one slot. Slots run **clockwise** from screen-north —
    /// NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7.
    VehicleOctant,
}

/// Facing-to-frame-slot table for infantry bodies, transcribed from the
/// original engine's 32-dword table.
///
/// Indexed by [`infantry_facing_step32`]. The two trailing 7s are what make the
/// north arc wrap early: slot boundaries land 4/256 of a turn *before* the
/// octant centers rather than on them, so the arc for slot 7 is facing bytes
/// 236..=255 plus 0..=11, not the symmetric 240..=15. That bias is native
/// behavior, not an artifact of the transcription.
const INFANTRY_FACING_SLOT_TABLE: [u8; 32] = [
    7, 7, 6, 6, 6, 6, 5, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, 7, 7,
];

/// Number of frame blocks an SHP vehicle body must declare for its facing
/// slots to be used at all. Any other count draws slot 0 for every facing.
const VEHICLE_FACING_SLOTS: u8 = 8;

/// Quantize a facing byte to the 32-step index into [`INFANTRY_FACING_SLOT_TABLE`].
///
/// The original computes this from the 16-bit facing as
/// `((facing16 >> 10) + 1) >> 1 & 0x1F`. Only bits 10..=15 of the 16-bit facing
/// survive that shift, and those are exactly bits 2..=7 of the facing byte, so
/// the byte carries every bit the native expression can see — matching it needs
/// no 16-bit plumbing. The `+ 1` before the final shift is a round-half-up, which
/// is why this is not the same as truncating the facing into eight buckets.
const fn infantry_facing_step32(facing: u8) -> usize {
    (((facing as u16 >> 2) + 1) >> 1) as usize & 0x1F
}

/// Facing byte to SHP vehicle frame-block slot.
///
/// The original computes `(((facing16 >> 12) + 1) >> 1) + 1 & 7`. As above, the
/// shift keeps only bits 12..=15 of the 16-bit facing, which are bits 4..=7 of
/// the byte. The inner `+ 1` rounds to the nearest octant; the outer `+ 1` is
/// what puts NW — not N — at frame 0 of every block.
const fn vehicle_facing_slot(facing: u8) -> u16 {
    ((((facing as u16 >> 4) + 1) >> 1) + 1) & 7
}

/// Facing byte to infantry frame-block slot (0..=7), counter-clockwise from
/// screen-north. Exposed for render-side fallbacks that have to pick a standing
/// pose without a `SequenceDef` in hand.
pub fn infantry_facing_slot(facing: u8) -> u16 {
    INFANTRY_FACING_SLOT_TABLE[infantry_facing_step32(facing)] as u16
}

/// Definition of one animation sequence within an SHP file.
///
/// Describes the frame range, timing, and looping behavior for a named
/// sequence. Multiple `SequenceDef`s grouped into a `SequenceSet` define
/// all animations available for one object type.
///
/// ## Frame index formula
/// For directional sequences:
///   `start_frame + facing_slot * facing_multiplier + frame_within_sequence`
/// For non-directional (facings == 1):
///   `start_frame + frame_within_sequence`
///
/// ## Facing convention
/// RA2's DirStruct byte (0–255) is clockwise in *cell* space (0=N, 64=E,
/// 128=S, 192=W), but SHP frame blocks start at screen-north, which is cell
/// NW. `facing_slots` selects which of the two native conversions applies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SequenceDef {
    /// First SHP frame index for this sequence.
    pub start_frame: u16,
    /// Number of animation frames per facing direction.
    pub frame_count: u16,
    /// Number of facing directions (1 = non-directional, 8 = infantry standard).
    pub facings: u8,
    /// Frame stride between facings — the offset applied per facing increment.
    /// From art.ini's 3rd sequence field. Typically equals `frame_count` for
    /// contiguous packing. If 0, the animation is facing-independent.
    pub facing_multiplier: u16,
    /// Reached native frames per image. Lower = faster animation.
    pub frame_delay: u16,
    /// Whether native game-speed normalization applies to this action delay.
    pub normalized: bool,
    /// Behavior when the sequence reaches its final frame.
    pub loop_mode: LoopMode,
    /// Which native facing-to-slot rule converts the facing byte into a frame
    /// block index. Infantry and SHP vehicles use different ones.
    pub facing_slots: FacingSlots,
}

/// Per-entity animation state component.
///
/// Attach to any entity that should animate. The `tick_animations()` system
/// reads and updates this each frame. The render loop reads `sequence` and
/// `frame_index` to select the correct SHP frame via `resolve_shp_frame()`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Animation {
    /// Currently playing sequence.
    pub sequence: SequenceKind,
    /// Current frame within the sequence (0 to frame_count - 1).
    pub frame_index: u16,
    /// Reached native frames accumulated since the last image advance.
    pub elapsed_frames: u16,
    /// True if a HoldLast sequence has reached its final frame.
    pub finished: bool,
}

impl Animation {
    /// Create a new Animation starting at frame 0 of the given sequence.
    pub fn new(sequence: SequenceKind) -> Self {
        Self {
            sequence,
            frame_index: 0,
            elapsed_frames: 0,
            finished: false,
        }
    }

    /// Switch to a different sequence, resetting frame and timing.
    /// No-op if already playing the requested sequence.
    pub fn switch_to(&mut self, sequence: SequenceKind) {
        if self.sequence != sequence {
            self.sequence = sequence;
            self.frame_index = 0;
            self.elapsed_frames = 0;
            self.finished = false;
        }
    }
}

/// Whether a sequence represents the infantry being in a prone stance.
///
/// This is a temporary stance proxy until the sim carries an explicit prone bit.
pub fn sequence_is_prone(sequence: SequenceKind) -> bool {
    matches!(
        sequence,
        SequenceKind::Prone
            | SequenceKind::Crawl
            | SequenceKind::FireProne
            | SequenceKind::SecondaryProne
            | SequenceKind::Down
    )
}

/// Whether the current animation represents a prone stance.
///
/// TODO(RE): The sim does not yet enter these sequences during live infantry
/// combat because prone-entry behavior has not been reverse engineered yet.
/// This helper only recognizes prone once some other system explicitly switches
/// the animation into one of the prone-related sequences.
pub fn animation_is_prone(animation: Option<&Animation>) -> bool {
    animation.is_some_and(|anim| sequence_is_prone(anim.sequence))
}

/// Whether a sequence is a one-shot infantry fire action.
pub fn sequence_is_fire_action(sequence: SequenceKind) -> bool {
    matches!(
        sequence,
        SequenceKind::Attack
            | SequenceKind::FireProne
            | SequenceKind::DeployedFire
            | SequenceKind::SecondaryFire
            | SequenceKind::SecondaryProne
            | SequenceKind::FireFly
            | SequenceKind::WetAttack
    )
}

/// Collection of sequence definitions for one object type.
///
/// Maps `SequenceKind` → `SequenceDef`. Not all kinds need to be present;
/// entities hold their current frame if the active sequence is missing.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SequenceSet {
    sequences: BTreeMap<SequenceKind, SequenceDef>,
}

impl SequenceSet {
    /// Create an empty SequenceSet.
    pub fn new() -> Self {
        Self {
            sequences: BTreeMap::new(),
        }
    }

    /// Add a sequence definition.
    pub fn insert(&mut self, kind: SequenceKind, def: SequenceDef) {
        self.sequences.insert(kind, def);
    }

    /// Look up a sequence definition by kind.
    pub fn get(&self, kind: &SequenceKind) -> Option<&SequenceDef> {
        self.sequences.get(kind)
    }

    /// Number of defined sequences.
    pub fn len(&self) -> usize {
        self.sequences.len()
    }

    /// Whether no sequences are defined.
    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }
}

/// Compute the SHP frame index for a given sequence, facing, and animation frame.
///
/// For directional sequences (facings > 1):
///   `start_frame + facing_slot * facing_multiplier + frame_index`
/// For non-directional (facings == 1):
///   `start_frame + frame_index`
///
/// `facing` is the RA2 DirStruct byte (0–255, clockwise in cell space:
/// 0=N, 64=E, 128=S, 192=W). The facing-to-slot conversion is
/// family-specific — see [`FacingSlots`].
///
/// The `facings <= 1` early return stands in for the original's directional
/// test, which reads the facing multiplier rather than a facing count. The two
/// agree because every producer that emits a zero multiplier also emits
/// `facings == 1`, and a zero multiplier contributes nothing to the sum anyway.
pub fn resolve_shp_frame(def: &SequenceDef, facing: u8, frame_index: u16) -> u16 {
    let clamped: u16 = if def.frame_count > 0 {
        frame_index % def.frame_count
    } else {
        0
    };

    if def.facings <= 1 {
        return def.start_frame + clamped;
    }

    let facing_slot: u16 = match def.facing_slots {
        FacingSlots::InfantryTable => infantry_facing_slot(facing),
        // The vehicle draw path gates the whole slot computation on the body
        // declaring exactly 8 frame blocks; any other count draws block 0 for
        // every facing.
        FacingSlots::VehicleOctant if def.facings == VEHICLE_FACING_SLOTS => {
            vehicle_facing_slot(facing)
        }
        FacingSlots::VehicleOctant => 0,
    };

    def.start_frame + facing_slot * def.facing_multiplier + clamped
}

/// Advance a single animation by one reached native gameplay frame.
///
/// Returns `Some(SequenceKind)` if the sequence completed with a `TransitionTo`
/// loop mode, indicating the caller should switch to that sequence. Otherwise None.
pub fn advance_animation(
    anim: &mut Animation,
    def: &SequenceDef,
    game_options: &crate::sim::game_options::GameOptions,
) -> Option<SequenceKind> {
    let frame_delay = if def.normalized {
        game_options.normalized_anim_delay(def.frame_delay)
    } else {
        def.frame_delay
    };
    if anim.finished || frame_delay == 0 || def.frame_count == 0 {
        return None;
    }

    anim.elapsed_frames = anim.elapsed_frames.saturating_add(1);

    while anim.elapsed_frames >= frame_delay {
        anim.elapsed_frames -= frame_delay;
        anim.frame_index += 1;

        if anim.frame_index >= def.frame_count {
            match def.loop_mode {
                LoopMode::Loop => {
                    anim.frame_index = 0;
                }
                LoopMode::HoldLast => {
                    anim.frame_index = def.frame_count.saturating_sub(1);
                    anim.finished = true;
                    anim.elapsed_frames = 0;
                    return None;
                }
                LoopMode::TransitionTo(next) => {
                    anim.frame_index = 0;
                    anim.elapsed_frames = 0;
                    return Some(next);
                }
            }
        }
    }

    None
}

/// Advance all animated entities by one reached native gameplay frame.
///
/// 1. Dying entities: skip auto-transitions, only advance death animation.
///    Returns IDs of dying entities whose death animation has finished.
/// 2. Auto-transitions: entities with MovementTarget switch to Walk;
///    entities without MovementTarget switch back to Stand.
/// 3. Attack transitions: stationary entities with attack_target switch to
///    Attack (or FireProne/DeployedFire depending on stance).
/// 4. Advances frame timing for each entity's current sequence.
///
/// `sequences` maps type_id → SequenceSet for frame timing lookup.
/// Entities whose type_id isn't in the map are skipped (no animation advance).
fn tick_animations_impl(
    entities: &mut crate::sim::entity_store::EntityStore,
    sequences: &BTreeMap<String, SequenceSet>,
    game_options: &crate::sim::game_options::GameOptions,
    interner: &crate::sim::intern::StringInterner,
    tick_dying: bool,
) -> Vec<u64> {
    let mut dying_finished: Vec<u64> = Vec::new();
    let keys: Vec<u64> = entities.keys_sorted();

    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        if entity.dying && !tick_dying {
            continue;
        }
        let Some(anim) = entity.animation.as_mut() else {
            // Dying entity with no animation → ready for despawn.
            if entity.dying {
                dying_finished.push(id);
            }
            continue;
        };

        // Dying entities: only advance the death animation, skip all transitions.
        if entity.dying {
            if anim.finished {
                dying_finished.push(id);
                continue;
            }
            let Some(seq_set) = sequences.get(interner.resolve(entity.type_ref)) else {
                dying_finished.push(id);
                continue;
            };
            let Some(def) = seq_set.get(&anim.sequence) else {
                dying_finished.push(id);
                continue;
            };
            advance_animation(anim, def, game_options);
            if anim.finished {
                dying_finished.push(id);
            }
            continue;
        }

        let has_movement: bool = entity.movement_target.is_some();
        let pending_fire_sequence = entity
            .attack_target
            .as_ref()
            .and_then(|attack| attack.pending_infantry_fire.map(|pending| pending.sequence));
        let has_fire_action = pending_fire_sequence.is_some();
        let preserving_fire_action = sequence_is_fire_action(anim.sequence) && !has_fire_action;

        // Look up this type's sequence definitions for transition checks.
        let seq_set: Option<&SequenceSet> = sequences.get(interner.resolve(entity.type_ref));

        // Deploy state takes priority over the standard Stand/Walk/Attack cascade.
        // The visual reflects the sim phase; DeployedFire is the auto-transition
        // when a Deployed unit gains an attack target (visual-only, matches stock YR).
        match entity.deploy_state {
            Some(crate::sim::deploy::DeployPhase::Deploying { .. }) => {
                anim.switch_to(SequenceKind::Deploy);
            }
            Some(crate::sim::deploy::DeployPhase::Undeploying { .. }) => {
                anim.switch_to(SequenceKind::Undeploy);
            }
            Some(crate::sim::deploy::DeployPhase::Deployed) => {
                if let Some(sequence) = pending_fire_sequence {
                    anim.switch_to(sequence);
                } else if anim.sequence != SequenceKind::DeployedFire {
                    anim.switch_to(SequenceKind::Deployed);
                }
            }
            None => {
                let runtime_prone = entity
                    .infantry
                    .as_ref()
                    .is_some_and(|infantry| infantry.is_prone);
                if !matches!(anim.sequence, SequenceKind::Down | SequenceKind::Up) && runtime_prone
                {
                    if let Some(set) = seq_set {
                        if let Some(sequence) = pending_fire_sequence {
                            anim.switch_to(sequence);
                        } else if has_movement {
                            if set.get(&SequenceKind::Crawl).is_some() {
                                anim.switch_to(SequenceKind::Crawl);
                            }
                        } else if !preserving_fire_action && set.get(&SequenceKind::Prone).is_some()
                        {
                            anim.switch_to(SequenceKind::Prone);
                        }
                    }
                }
                // Standard cascade for upright entities — preserved verbatim from prior logic.
                if !matches!(anim.sequence, SequenceKind::Down | SequenceKind::Up)
                    && !runtime_prone
                    && has_movement
                    && anim.sequence == SequenceKind::Stand
                {
                    anim.switch_to(SequenceKind::Walk);
                } else if !matches!(anim.sequence, SequenceKind::Down | SequenceKind::Up)
                    && !runtime_prone
                    && !has_movement
                    && matches!(anim.sequence, SequenceKind::Walk | SequenceKind::Crawl)
                {
                    anim.switch_to(SequenceKind::Stand);
                }
                if !matches!(anim.sequence, SequenceKind::Down | SequenceKind::Up)
                    && !runtime_prone
                    && has_fire_action
                    && !has_movement
                {
                    if let Some(sequence) = pending_fire_sequence {
                        anim.switch_to(sequence);
                    }
                }
            }
        }

        // Advance frame timing.
        let Some(set) = seq_set else {
            continue;
        };
        let Some(def) = set.get(&anim.sequence) else {
            continue;
        };

        if let Some(next) = advance_animation(anim, def, game_options) {
            anim.switch_to(next);
        }
    }

    dying_finished
}

pub fn tick_animations(
    entities: &mut crate::sim::entity_store::EntityStore,
    sequences: &BTreeMap<String, SequenceSet>,
    game_options: &crate::sim::game_options::GameOptions,
    interner: &crate::sim::intern::StringInterner,
) -> Vec<u64> {
    tick_animations_impl(entities, sequences, game_options, interner, true)
}

/// Advance only living entity animations. Dying animation completion is owned
/// by that object's live scheduler turn so UnInit can compact the LogicVector
/// before the scheduler cursor advances.
pub(crate) fn tick_non_dying_animations(
    entities: &mut crate::sim::entity_store::EntityStore,
    sequences: &BTreeMap<String, SequenceSet>,
    game_options: &crate::sim::game_options::GameOptions,
    interner: &crate::sim::intern::StringInterner,
) {
    let _ = tick_animations_impl(entities, sequences, game_options, interner, false);
}

/// Advance one dying object's death sequence during its own scheduler turn.
/// Missing animation/type/sequence data takes the immediate-finish path used
/// by the prior app-owned completion fallback.
pub(crate) fn tick_dying_animation(
    entity: &mut crate::sim::game_entity::GameEntity,
    sequences: &BTreeMap<String, SequenceSet>,
    game_options: &crate::sim::game_options::GameOptions,
    type_name: &str,
) -> bool {
    debug_assert!(entity.dying);
    let Some(anim) = entity.animation.as_mut() else {
        return true;
    };
    if anim.finished {
        return true;
    }
    let Some(seq_set) = sequences.get(type_name) else {
        return true;
    };
    let Some(def) = seq_set.get(&anim.sequence) else {
        return true;
    };
    advance_animation(anim, def, game_options);
    anim.finished
}

/// Number of animation frames in oregath.shp per facing direction.
const HARVEST_OVERLAY_FRAMES: u16 = 15;

/// Advance all HarvestOverlay components by one reached native frame.
///
/// Cycles through the 15-frame oregath.shp animation for harvesters that are
/// actively gathering ore. When not visible, the overlay is skipped.
pub fn tick_harvest_overlays(entities: &mut crate::sim::entity_store::EntityStore) {
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(overlay) = entity.harvest_overlay.as_mut() else {
            continue;
        };
        if !overlay.visible {
            continue;
        }
        overlay.elapsed_frames = 0;
        overlay.frame = (overlay.frame + 1) % HARVEST_OVERLAY_FRAMES;
    }
}

/// Advance all VoxelAnimation components by one reached native frame.
///
/// Cycles through HVA frames for voxel entities that have `playing == true`.
/// Frame wraps around to 0 when reaching frame_count (looping animation).
pub fn tick_voxel_animations(entities: &mut crate::sim::entity_store::EntityStore) {
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let Some(anim) = entity.voxel_animation.as_mut() else {
            continue;
        };
        if !anim.playing || anim.frame_count <= 1 || anim.frame_delay == 0 {
            continue;
        }
        anim.elapsed_frames = anim.elapsed_frames.saturating_add(1);
        while anim.elapsed_frames >= anim.frame_delay {
            anim.elapsed_frames -= anim.frame_delay;
            anim.frame = (anim.frame + 1) % anim.frame_count;
        }
    }
}

/// Create the default infantry sequence set matching RA2's standard frame layout.
///
/// Based on RA2's standard infantry sequence defaults:
/// - Stand: frames 0–7 (1 frame × 8 facings)
/// - Walk: frames 8–55 (6 frames × 8 facings)
/// - Idle1: frames 56–70 (15 frames, non-directional)
/// - Idle2: frames 71–85 (15 frames, non-directional)
/// - Die1: frames 86–100 (15 frames, non-directional)
/// - Die2: frames 101–115 (15 frames, non-directional)
pub fn default_infantry_sequences() -> SequenceSet {
    let mut set: SequenceSet = SequenceSet::new();

    set.insert(
        SequenceKind::Stand,
        SequenceDef {
            start_frame: 0,
            frame_count: 1,
            facings: INFANTRY_FACINGS,
            facing_multiplier: 1,
            frame_delay: DEFAULT_STAND_FRAME_DELAY,
            normalized: false,
            loop_mode: LoopMode::Loop,
            facing_slots: FacingSlots::InfantryTable,
        },
    );
    set.insert(
        SequenceKind::Walk,
        SequenceDef {
            start_frame: 8,
            frame_count: 6,
            facings: INFANTRY_FACINGS,
            facing_multiplier: 6,
            frame_delay: DEFAULT_WALK_FRAME_DELAY,
            normalized: false,
            loop_mode: LoopMode::Loop,
            facing_slots: FacingSlots::InfantryTable,
        },
    );
    set.insert(
        SequenceKind::Idle1,
        SequenceDef {
            start_frame: 56,
            frame_count: 15,
            facings: 1,
            facing_multiplier: 0,
            frame_delay: DEFAULT_IDLE_FRAME_DELAY,
            normalized: true,
            loop_mode: LoopMode::TransitionTo(SequenceKind::Stand),
            facing_slots: FacingSlots::InfantryTable,
        },
    );
    set.insert(
        SequenceKind::Idle2,
        SequenceDef {
            start_frame: 71,
            frame_count: 15,
            facings: 1,
            facing_multiplier: 0,
            frame_delay: DEFAULT_IDLE_FRAME_DELAY,
            normalized: true,
            loop_mode: LoopMode::TransitionTo(SequenceKind::Stand),
            facing_slots: FacingSlots::InfantryTable,
        },
    );
    set.insert(
        SequenceKind::Die1,
        SequenceDef {
            start_frame: 86,
            frame_count: 15,
            facings: 1,
            facing_multiplier: 0,
            frame_delay: DEFAULT_DIE_FRAME_DELAY,
            normalized: false,
            loop_mode: LoopMode::HoldLast,
            facing_slots: FacingSlots::InfantryTable,
        },
    );
    set.insert(
        SequenceKind::Die2,
        SequenceDef {
            start_frame: 101,
            frame_count: 15,
            facings: 1,
            facing_multiplier: 0,
            frame_delay: DEFAULT_DIE_FRAME_DELAY,
            normalized: false,
            loop_mode: LoopMode::HoldLast,
            facing_slots: FacingSlots::InfantryTable,
        },
    );

    set
}

/// Create default building sequence set (single idle frame, no animation).
///
/// Buildings typically use frame 0 as their idle state. Animated buildings
/// (e.g., power plant, radar dome) will need per-type overrides later.
pub fn default_building_sequences() -> SequenceSet {
    let mut set: SequenceSet = SequenceSet::new();

    set.insert(
        SequenceKind::Stand,
        SequenceDef {
            start_frame: 0,
            frame_count: 1,
            facings: 1,
            facing_multiplier: 0,
            frame_delay: DEFAULT_STAND_FRAME_DELAY,
            normalized: false,
            loop_mode: LoopMode::Loop,
            facing_slots: FacingSlots::InfantryTable,
        },
    );

    set
}

/// Map warhead InfDeath value to the appropriate death SequenceKind.
///
/// InfDeath: 1→Die1, 2→Die2, 3→Die3, 4→Die4, 5→Die5. 0 defaults to Die1.
pub fn death_sequence_for_inf_death(inf_death: u8) -> SequenceKind {
    match inf_death.min(5) {
        2 => SequenceKind::Die2,
        3 => SequenceKind::Die3,
        4 => SequenceKind::Die4,
        5 => SequenceKind::Die5,
        _ => SequenceKind::Die1,
    }
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
