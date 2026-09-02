//! Game sound events — the bridge between sim and audio.
//!
//! The simulation produces GameSoundEvents when things happen (weapon fired,
//! unit selected, entity destroyed). The app layer collects these events each
//! tick and feeds them to the SfxPlayer for playback.
//!
//! Events carry the sound ID (from rules.ini / sound.ini) rather than a
//! filename — the SfxPlayer resolves IDs to files via SoundRegistry.
//!
//! ## Design
//! Events are plain data — no audio library handles, no asset references. This keeps
//! sim/ free from audio dependencies. The event queue is a simple Vec that
//! gets drained each frame.
//!
//! ## Dependency rules
//! - Part of audio/ — but contains no rodio code, only data types.
//! - sim/ may reference this module to push events (acceptable because
//!   it's pure data with zero audio-library dependencies).

/// Where a positional sound plays: the world-pixel point the presentation
/// frame draws it at, and the cell it occupies.
///
/// gamemd-derived: `VocClass::PlayAt @ 0x007509E0` receives the object's
/// coordinate; `VocClass::CalcVolumeAndPan @ 0x00750AC0` projects it to the
/// tactical client point for volume and pan and derives the cell as
/// `coord >> 8` (`0x00750B32..0x00750B4E`) for the `Type=SHROUD` gate. VERA
/// carries both halves so the app never has to invert a projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundSource {
    pub screen_x: f32,
    pub screen_y: f32,
    pub rx: u16,
    pub ry: u16,
}

impl SoundSource {
    pub fn new(screen: (f32, f32), cell: (u16, u16)) -> Self {
        Self {
            screen_x: screen.0,
            screen_y: screen.1,
            rx: cell.0,
            ry: cell.1,
        }
    }

    pub fn screen_pos(&self) -> (f32, f32) {
        (self.screen_x, self.screen_y)
    }

    pub fn cell(&self) -> (u16, u16) {
        (self.rx, self.ry)
    }
}

/// A sound event produced by the game simulation or UI.
#[derive(Debug, Clone)]
pub enum GameSoundEvent {
    /// Local player's base structure / harvester is under enemy attack — queue
    /// the EVA voice (no spatial SFX; the radar diamond is sim-side).
    UnderAttackEva { eva_sound_id: String },

    /// Accepted local HouseClass win/loss transition — play immediately with
    /// STANDARD Vox semantics and do not persist/reconstruct across load.
    OutcomeEva { eva_sound_id: String },

    /// Start/report sound owned by one authoritative animation object.
    AnimationStarted {
        anim_id: u64,
        sound_id: String,
        source: Option<SoundSource>,
    },

    /// Release one animation's active handle, then optionally play StopSound.
    AnimationStopped {
        anim_id: u64,
        stop_sound_id: Option<String>,
        source: Option<SoundSource>,
    },
    /// A weapon fired — play the weapon's Report= sound.
    WeaponFired {
        /// sound.ini ID from the weapon's Report= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        /// If None, plays at full volume (non-positional).
        source: Option<SoundSource>,
    },

    /// A unit was selected by the player — play VoiceSelect.
    UnitSelected {
        /// sound.ini ID from the unit's VoiceSelect= field.
        sound_id: String,
    },

    /// A unit was ordered to move — play VoiceMove.
    UnitMoveOrder {
        /// sound.ini ID from the unit's VoiceMove= field.
        sound_id: String,
    },

    /// A unit was ordered to attack — play VoiceAttack.
    UnitAttackOrder {
        /// sound.ini ID from the unit's VoiceAttack= field.
        sound_id: String,
    },

    /// An entity was destroyed — play DieSound.
    EntityDestroyed {
        /// sound.ini ID from the entity's DieSound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        source: Option<SoundSource>,
    },

    /// An entity was crushed by a vehicle — play CrushSound (the squish).
    EntityCrushed {
        /// sound.ini ID from the entity's CrushSound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        source: Option<SoundSource>,
    },

    /// An infantry entity entered the Deploying phase — play DeploySound.
    EntityDeployed {
        /// sound.ini ID from the entity's DeploySound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        source: Option<SoundSource>,
    },

    /// An infantry entity entered the Undeploying phase — play UndeploySound.
    EntityUndeployed {
        /// sound.ini ID from the entity's UndeploySound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        source: Option<SoundSource>,
    },

    /// A chrono teleport happened — play the resolved warp sound at this position.
    /// Emitted twice per warp (source = ChronoOutSound, destination = ChronoInSound).
    ChronoTeleport {
        /// sound.ini ID — already resolved to the per-unit ChronoIn/OutSound by sim.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        source: Option<SoundSource>,
    },

    /// A local-player object crossed a veterancy rank — the positional
    /// `[AudioVisual] UpgradeVeteranSound=`/`UpgradeEliteSound=` cue
    /// (`VocClass::PlayAt` from `TechnoClass::AI_Update @ 0x006FA0BC`).
    UnitPromoted {
        sound_id: String,
        source: Option<SoundSource>,
    },

    /// The `EVA_UnitPromoted` voice that accompanies `UnitPromoted`.
    UnitPromotedEva { eva_sound_id: String },

    /// One-shot positional `[AudioVisual] CloakSound` requested by an accepted
    /// native StartUncloaking arg-zero transition.
    CloakSound {
        sound_id: String,
        source: Option<SoundSource>,
    },

    /// A building finished construction — play the EVA "Construction complete" or similar.
    BuildingReady {
        /// sound.ini ID for the completion announcement.
        sound_id: String,
    },

    /// A unit finished training — play the EVA "Unit ready" or similar.
    UnitReady {
        /// sound.ini ID for the unit-ready announcement.
        sound_id: String,
    },

    /// EVA cue: a deploy command failed placement validation.
    CannotDeployHere {
        /// sound.ini ID for the EVA announcement.
        sound_id: String,
    },

    /// EVA cue: a friendly building was garrisoned (first occupant entered).
    StructureGarrisoned {
        /// sound.ini ID for the EVA announcement.
        sound_id: String,
    },

    /// EVA cue: a friendly garrison was abandoned (last occupant left).
    StructureAbandoned {
        /// sound.ini ID for the EVA announcement.
        sound_id: String,
    },

    /// Positional SFX from [AudioVisual] BuildingGarrisonedSound — plays at
    /// the building's screen position when the first occupant enters.
    BuildingGarrisonedSfx {
        /// sound.ini ID for the SFX (resolves "BuildingGarrisoned" → file).
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Positional SFX from `[SealPlaceBomb]` — plays at the attacker's screen
    /// position when a C4-capable infantry claims a plant on a CanC4 building.
    C4Planted {
        /// sound.ini ID for the SFX (resolves "SealPlaceBomb" → file).
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Positional SFX played when a docked harvester departs after dumping.
    /// Resolved from [AudioVisual] BunkerWallsDownSound (retail "TankBunkerDown").
    /// Fires every refinery dock cycle.
    RefineryExitSfx {
        /// sound.ini ID for the SFX.
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Positional SFX for tank-bunker walls raising (install) or falling
    /// (exit/teardown). The up/down choice — and thus which rules key resolves
    /// `sound_id` — is made when the event is built; plays at the bunker's
    /// screen position. Skipped upstream when the rules key is empty.
    BunkerWalls {
        /// sound.ini ID (BunkerWallsUpSound or BunkerWallsDownSound).
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Positional SFX played when a paradropped passenger successfully opens
    /// a parachute. Resolved from [AudioVisual] ChuteSound.
    ChuteSound {
        /// sound.ini ID for the SFX.
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Positional SFX + EVA cue from a bridge repair triggered by an engineer
    /// entering a `BridgeRepairHut`. Plays the spatial `[BridgeRepaired]`
    /// sound (resolved from `rules.bridge_rules.repair_sound`) at the hut's
    /// screen position when `sound_id` is non-empty. When `eva_sound_id` is
    /// `Some`, the EVA arm plays it as a non-positional cue (gated upstream
    /// on local-human owner).
    BridgeRepaired {
        /// Spatial `[BridgeRepaired]` SFX id. Empty when
        /// `RepairBridgeSound=` is unset in rules.
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
        /// `Some(eva_id)` when the engineer's owner is the local human;
        /// `None` otherwise.
        eva_sound_id: Option<String>,
    },

    /// Positional sound emitted when a world-effect animation starts.
    WorldEffectStarted {
        /// sound.ini ID for the selected animation's StartSound/Report.
        sound_id: String,
        /// Screen position for spatial audio.
        source: Option<SoundSource>,
    },

    /// Generic UI sound (button click, error beep, etc.).
    UiSound {
        /// sound.ini ID for the UI sound.
        sound_id: String,
    },
}

impl GameSoundEvent {
    /// Get the sound ID for this event.
    pub fn sound_id(&self) -> &str {
        match self {
            Self::WeaponFired { sound_id, .. }
            | Self::AnimationStarted { sound_id, .. }
            | Self::UnitSelected { sound_id }
            | Self::UnitMoveOrder { sound_id }
            | Self::UnitAttackOrder { sound_id }
            | Self::EntityDestroyed { sound_id, .. }
            | Self::EntityCrushed { sound_id, .. }
            | Self::EntityDeployed { sound_id, .. }
            | Self::EntityUndeployed { sound_id, .. }
            | Self::ChronoTeleport { sound_id, .. }
            | Self::UnitPromoted { sound_id, .. }
            | Self::CloakSound { sound_id, .. }
            | Self::BuildingReady { sound_id }
            | Self::UnitReady { sound_id }
            | Self::CannotDeployHere { sound_id }
            | Self::UiSound { sound_id }
            | Self::StructureGarrisoned { sound_id }
            | Self::StructureAbandoned { sound_id }
            | Self::BuildingGarrisonedSfx { sound_id, .. }
            | Self::C4Planted { sound_id, .. }
            | Self::RefineryExitSfx { sound_id, .. }
            | Self::BunkerWalls { sound_id, .. }
            | Self::ChuteSound { sound_id, .. }
            | Self::BridgeRepaired { sound_id, .. }
            | Self::WorldEffectStarted { sound_id, .. } => sound_id,
            Self::AnimationStopped { stop_sound_id, .. } => stop_sound_id.as_deref().unwrap_or(""),
            Self::UnderAttackEva { eva_sound_id }
            | Self::OutcomeEva { eva_sound_id }
            | Self::UnitPromotedEva { eva_sound_id } => eva_sound_id,
        }
    }

    /// Get the positional source for spatial audio, if this event has one.
    pub fn source(&self) -> Option<SoundSource> {
        match self {
            Self::WeaponFired { source, .. }
            | Self::AnimationStarted { source, .. }
            | Self::AnimationStopped { source, .. }
            | Self::EntityDestroyed { source, .. }
            | Self::EntityCrushed { source, .. }
            | Self::EntityDeployed { source, .. }
            | Self::EntityUndeployed { source, .. }
            | Self::ChronoTeleport { source, .. }
            | Self::UnitPromoted { source, .. }
            | Self::CloakSound { source, .. }
            | Self::BuildingGarrisonedSfx { source, .. }
            | Self::C4Planted { source, .. }
            | Self::RefineryExitSfx { source, .. }
            | Self::BunkerWalls { source, .. }
            | Self::ChuteSound { source, .. }
            | Self::BridgeRepaired { source, .. }
            | Self::WorldEffectStarted { source, .. } => *source,
            _ => None,
        }
    }

    /// Get the screen position for spatial audio, if this event has one.
    pub fn screen_pos(&self) -> Option<(f32, f32)> {
        self.source().map(|source| source.screen_pos())
    }
}

/// Collects sound events during a simulation tick for later playback.
///
/// Drained by the app layer each frame after sim ticking.
///
/// RESIDUAL (GSI-15.05/15.08) — pass 2 split this into what is closed, what is
/// landable and what is genuinely blocked.
///
/// **NO-DIFF, retired from the list.** `DownReport=` needs no consumer: all 15
/// stock occurrences are commented out. The eight-facing report selection is
/// `Report[facing_u16 % Report.Count]` (`0x006FF349`..`0x006FF393`), and no
/// stock weapon authors a comma-separated `Report=` — 0 of 223 — so a single
/// report is observationally identical. Two of the three voice guards native
/// carries are already here: re-selecting an already-selected object emits
/// nothing (`ObjectClass::Select @ 0x005F4520` returns false), and the
/// one-voice-per-batch latch matches `g_SelectionVoice_Enable @ 0x00822CF2`.
///
/// **The repeat guard proper is NOT a timer, and it is what is missing.** Each
/// techno owns a pending-voice index (`+0x4F0`, sentinel -1), a live handle
/// (`+0x4DC`) and the playing index (`+0x4F4`). `TechnoClass::Queue_Voice @
/// 0x00708D90` only latches; `TechnoClass::AI_Update @ 0x006F9EBB` drains it —
/// handle free plays, handle live with the SAME index drops, handle live with a
/// different index holds and retries next tick. Voices are non-positional
/// (volume 1.0, pan 0x2000). VERA cannot model this yet because the drain needs
/// a liveness fact that only the audio layer knows, and `sim/` must not depend
/// on `audio/`: it wants a read-only per-tick liveness view passed INTO the
/// tick, or an inbound event queue — never an import.
/// - Trigger: ordering the same unit twice in quick succession.
/// - Player effect: the line restarts where retail lets it finish.
/// - Frequency: continuous — it is how players click.
///
/// **Still absent, unchanged.** Nothing emits ambient sound: `WorkingSound=`
/// (9 stock), `AuxSound1=` (8), `AuxSound2=` (5), `TurretRotateSound=` (2) and
/// the water enter/leave pair have no producer, and `MoveSound=` is parsed with
/// per-entity countdown state but no event carries it — and its real trigger is
/// UNKNOWN, since the site an existing research note names plays
/// `[AudioVisual] ScoldSound=` instead. `VoiceFeedback=` (133 stock) and
/// `VoiceCapture=` (3) have no consumers, `VoiceCrashing=` (7) is not parsed,
/// and taunts reach an empty match arm.
/// - Player effect: the soundscape is thin — units move silently and bases have
///   no working hum.
/// - Frequency: continuous.
/// - Downstream risk: every ambient family needs the looping handles recorded on
///   `audio/sfx.rs`, so they sit behind the same device-free arbiter. The voice
///   guard does not — it needs only the liveness channel above.
#[derive(Debug, Default)]
pub struct SoundEventQueue {
    events: Vec<GameSoundEvent>,
}

impl SoundEventQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Push a sound event into the queue.
    pub fn push(&mut self, event: GameSoundEvent) {
        self.events.push(event);
    }

    /// Drain all pending events for playback.
    pub fn drain(&mut self) -> Vec<GameSoundEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether there are pending events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_id_accessor() {
        let evt: GameSoundEvent = GameSoundEvent::WeaponFired {
            sound_id: "VGCannon1".to_string(),
            source: None,
        };
        assert_eq!(evt.sound_id(), "VGCannon1");
    }

    #[test]
    fn test_structure_garrisoned_sound_id_accessor() {
        let evt: GameSoundEvent = GameSoundEvent::StructureGarrisoned {
            sound_id: "ceva107".to_string(),
        };
        assert_eq!(evt.sound_id(), "ceva107");
        assert_eq!(evt.screen_pos(), None);
    }

    #[test]
    fn test_cannot_deploy_here_sound_id_accessor() {
        let evt = GameSoundEvent::CannotDeployHere {
            sound_id: "ceva063".to_string(),
        };
        assert_eq!(evt.sound_id(), "ceva063");
        assert_eq!(evt.screen_pos(), None);
    }

    #[test]
    fn test_building_garrisoned_sfx_screen_pos_accessor() {
        let evt: GameSoundEvent = GameSoundEvent::BuildingGarrisonedSfx {
            sound_id: "BuildingGarrisoned".to_string(),
            source: Some(SoundSource::new((100.0, 200.0), (3, 4))),
        };
        assert_eq!(evt.sound_id(), "BuildingGarrisoned");
        assert_eq!(evt.screen_pos(), Some((100.0, 200.0)));
        assert_eq!(evt.source().map(|s| s.cell()), Some((3, 4)));
    }

    #[test]
    fn test_chute_sound_screen_pos_accessor() {
        let evt = GameSoundEvent::ChuteSound {
            sound_id: "ParachuteDrop".to_string(),
            source: Some(SoundSource::new((128.0, 256.0), (5, 6))),
        };
        assert_eq!(evt.sound_id(), "ParachuteDrop");
        assert_eq!(evt.screen_pos(), Some((128.0, 256.0)));
    }

    #[test]
    fn test_bridge_repaired_carries_spatial_and_eva_sound_ids() {
        let evt = GameSoundEvent::BridgeRepaired {
            sound_id: "BridgeRepaired".to_string(),
            source: Some(SoundSource::new((32.0, 64.0), (1, 2))),
            eva_sound_id: Some("EVA_BridgeRepaired".to_string()),
        };

        assert_eq!(evt.sound_id(), "BridgeRepaired");
        assert_eq!(evt.screen_pos(), Some((32.0, 64.0)));
        match evt {
            GameSoundEvent::BridgeRepaired { eva_sound_id, .. } => {
                assert_eq!(eva_sound_id.as_deref(), Some("EVA_BridgeRepaired"));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_queue_drain() {
        let mut queue: SoundEventQueue = SoundEventQueue::new();
        assert!(queue.is_empty());
        queue.push(GameSoundEvent::UiSound {
            sound_id: "click".to_string(),
        });
        queue.push(GameSoundEvent::UiSound {
            sound_id: "beep".to_string(),
        });
        assert!(!queue.is_empty());
        let events: Vec<GameSoundEvent> = queue.drain();
        assert_eq!(events.len(), 2);
        assert!(queue.is_empty());
    }
}
