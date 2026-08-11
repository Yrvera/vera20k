//! Persistent YR `WaveClass` runtime state.
//!
//! This is deliberately a simulation-owned display registration, rather than a
//! one-frame weapon-fire effect.  Native `WaveClass::Ctor` registers the wave
//! with LogicClass and the display array bucket returned by `InWhichLayer`.

use std::collections::BTreeMap;

use crate::sim::intern::InternedId;
use crate::sim::projectile::ProjectileCoord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WaveRecordedCell {
    pub rx: u16,
    pub ry: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WaveDamagePayload {
    pub firer_id: u64,
    pub base_damage: i32,
    pub warhead: InternedId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveDamageRequest {
    pub wave_id: u64,
    pub payload: WaveDamagePayload,
    pub recorded_cells: Vec<WaveRecordedCell>,
    pub wave_z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveDamageEvent {
    pub wave_id: u64,
    pub target_id: u64,
    pub payload: WaveDamagePayload,
}

/// `WaveClass::InWhichLayer` returns this display registration bucket.
///
/// It is not a global tactical pass position: YR has no generic five-layer
/// traversal that gives bucket 3 an across-family ordering.
pub const WAVE_DISPLAY_REGISTRATION_BUCKET: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WaveColorMode {
    FramebufferSonicDistortion,
    FixedLaserChannelAdd,
    FramebufferMagnetronDistortion,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Wave {
    pub id: u64,
    pub wave_type: u8,
    pub source: ProjectileCoord,
    pub target: ProjectileCoord,
    /// Native `WaveClass+0x130`; types 0 and 3 decrement this every tick.
    pub lifetime: i32,
    /// Native `WaveClass+0x1d0`; types 1 and 2 subtract six every tick.
    pub intensity: i32,
    /// `WaveClass+0x1f4/+0x200`: insertion order is damage order and duplicates
    /// remain observable. The exact native UpdateCells producer is still an
    /// explicit residual; callers may only provide evidence-backed records.
    pub recorded_cells: Vec<WaveRecordedCell>,
    pub damage_payload: Option<WaveDamagePayload>,
}

impl Wave {
    pub const DEFAULT_LIFETIME: i32 = 100;
    pub const DEFAULT_LASER_INTENSITY: i32 = 160;

    /// Named location: `WaveClass::Ctor @ 0x0075e950`.
    pub const fn new(wave_type: u8, source: ProjectileCoord, target: ProjectileCoord) -> Self {
        Self {
            id: 0,
            wave_type,
            source,
            target,
            lifetime: Self::DEFAULT_LIFETIME,
            intensity: Self::DEFAULT_LASER_INTENSITY,
            recorded_cells: Vec::new(),
            damage_payload: None,
        }
    }

    pub fn with_damage_payload(mut self, payload: WaveDamagePayload) -> Self {
        self.damage_payload = Some(payload);
        self
    }

    pub fn replace_recorded_cells(&mut self, cells: Vec<WaveRecordedCell>) {
        self.recorded_cells = cells;
    }

    pub const fn color_mode(&self) -> WaveColorMode {
        match self.wave_type {
            0 => WaveColorMode::FramebufferSonicDistortion,
            1 | 2 => WaveColorMode::FixedLaserChannelAdd,
            3 => WaveColorMode::FramebufferMagnetronDistortion,
            _ => WaveColorMode::None,
        }
    }

    pub const fn registration_bucket(&self) -> u8 {
        WAVE_DISPLAY_REGISTRATION_BUCKET
    }

    /// Named location: `WaveClass::Update @ 0x00760f50`.
    pub fn advance(&mut self) -> WaveTickResult {
        match self.wave_type {
            0 | 3 => {
                self.lifetime -= 1;
                let alive = self.lifetime >= 0;
                WaveTickResult {
                    alive,
                    update_geometry: true,
                    damage_recorded_cells: self.wave_type == 0,
                    call_object_ai: alive,
                }
            }
            1 | 2 => {
                self.intensity -= 6;
                WaveTickResult {
                    alive: self.intensity >= 32,
                    update_geometry: false,
                    damage_recorded_cells: false,
                    call_object_ai: false,
                }
            }
            _ => WaveTickResult {
                alive: true,
                update_geometry: false,
                damage_recorded_cells: false,
                call_object_ai: false,
            },
        }
    }

    /// Named location: `WaveClass::DrawIt @ 0x0075f9f0`.
    pub const fn visible_through_fog(
        &self,
        scenario_fog_gate: bool,
        source_fogged: bool,
        target_fogged: bool,
    ) -> bool {
        !scenario_fog_gate || !source_fogged || !target_fogged
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveTickResult {
    pub alive: bool,
    pub update_geometry: bool,
    /// Type 0 has this native side effect. The CellClass damage kernel remains
    /// its own authority; this result keeps the handoff explicit.
    pub damage_recorded_cells: bool,
    pub call_object_ai: bool,
}

/// Stable-ID collection matching the WaveClass array/Logic registration's
/// persistence boundary. The `BTreeMap` gives a deterministic update order
/// without inventing a cross-display-bucket render order.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WaveStore {
    next_id: u64,
    waves: BTreeMap<u64, Wave>,
}

impl WaveStore {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            waves: BTreeMap::new(),
        }
    }

    pub(crate) fn next_id(&self) -> u64 {
        self.next_id
    }

    pub fn len(&self) -> usize {
        self.waves.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &Wave)> {
        self.waves.iter()
    }

    pub fn spawn(&mut self, mut wave: Wave) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        wave.id = id;
        self.waves.insert(id, wave);
        id
    }

    /// Advance registered waves. Type-0 recorded-cell damage is deliberately
    /// exposed to the caller instead of approximating CellClass effects here.
    pub fn advance(&mut self) -> Vec<WaveDamageRequest> {
        let mut sonic_damage = Vec::new();
        self.waves.retain(|&id, wave| {
            let result = wave.advance();
            // Native type 0 damages its vector before the lifetime decrement,
            // including the tick that changes lifetime 0 to -1 and uninitializes.
            if result.damage_recorded_cells
                && let Some(payload) = wave.damage_payload
            {
                sonic_damage.push(WaveDamageRequest {
                    wave_id: id,
                    payload,
                    recorded_cells: wave.recorded_cells.clone(),
                    wave_z: wave.target.z,
                });
            }
            result.alive
        });
        sonic_damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> ProjectileCoord {
        ProjectileCoord::new(0, 0, 0)
    }

    #[test]
    fn type_zero_lifetime_boundary_matches_yr() {
        let mut wave = Wave {
            lifetime: 1,
            ..Wave::new(0, point(), point())
        };
        let survive = wave.advance();
        assert!(survive.alive);
        assert_eq!(wave.lifetime, 0);
        assert!(survive.damage_recorded_cells);
        assert!(survive.call_object_ai);

        let expired = wave.advance();
        assert!(!expired.alive);
        assert_eq!(wave.lifetime, -1);
        assert!(!expired.call_object_ai);
    }

    #[test]
    fn laser_intensity_boundary_matches_yr() {
        let mut survive = Wave {
            intensity: 38,
            ..Wave::new(1, point(), point())
        };
        assert!(survive.advance().alive);
        assert_eq!(survive.intensity, 32);

        let mut expired = Wave {
            intensity: 37,
            ..Wave::new(2, point(), point())
        };
        assert!(!expired.advance().alive);
        assert_eq!(expired.intensity, 31);
    }

    #[test]
    fn fog_gate_and_registration_bucket_are_exact() {
        let wave = Wave::new(3, point(), point());
        assert_eq!(wave.registration_bucket(), 3);
        assert!(!wave.visible_through_fog(true, true, true));
        assert!(wave.visible_through_fog(false, true, true));
        assert!(wave.visible_through_fog(true, false, true));
        assert_eq!(
            wave.color_mode(),
            WaveColorMode::FramebufferMagnetronDistortion
        );
    }

    #[test]
    fn store_keeps_waves_until_their_post_decrement_expiry() {
        let mut store = WaveStore::new();
        store.spawn(Wave {
            lifetime: 1,
            ..Wave::new(3, point(), point())
        });
        store.advance();
        assert_eq!(store.len(), 1);
        store.advance();
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn type_zero_reports_recorded_cells_before_final_uninit_in_native_order() {
        let payload = WaveDamagePayload {
            firer_id: 7,
            base_damage: 20,
            warhead: InternedId::from_index(3),
        };
        let mut wave = Wave {
            lifetime: 0,
            ..Wave::new(0, point(), point()).with_damage_payload(payload)
        };
        wave.replace_recorded_cells(vec![
            WaveRecordedCell { rx: 4, ry: 5 },
            WaveRecordedCell { rx: 4, ry: 5 },
            WaveRecordedCell { rx: 6, ry: 7 },
        ]);
        let mut store = WaveStore::new();
        let id = store.spawn(wave);

        let requests = store.advance();
        assert_eq!(store.len(), 0);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].wave_id, id);
        assert_eq!(
            requests[0].recorded_cells,
            vec![
                WaveRecordedCell { rx: 4, ry: 5 },
                WaveRecordedCell { rx: 4, ry: 5 },
                WaveRecordedCell { rx: 6, ry: 7 },
            ]
        );
    }
}
