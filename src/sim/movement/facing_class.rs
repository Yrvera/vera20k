//! Timer-based 16-bit facing interpolator, mirroring gamemd's FacingClass primitive.
//!
//! At any binary frame, the animated value is a pure function of state +
//! frame: `current(frame) = prev + sign(diff) * rot_per_frame * elapsed`.
//! Setting a new target snapshots the current animated value into `prev`
//! so rotations retarget smoothly without snap-back.
//!
//! Verified against gamemd.exe — see
//! ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md
//! §1.3 (24-byte byte layout), §2.1 (Current interpolation), §2.4 (Set
//! semantics), §2.7 (SetROT clamp + shift).
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on serde and std.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FacingClass {
    /// Destination — where the rotation will end up. 16-bit DirStruct.
    current: u16,
    /// Where the current rotation began. Updated on `set` to the animated
    /// value at the moment of the new request, so retargets continue
    /// smoothly from the visible position.
    prev: u16,
    /// Binary frame when the rotation began. None = never started.
    start_frame: Option<u32>,
    /// Total binary frames needed to complete the rotation. When this is
    /// 0, `current()` returns `current` immediately (snap-on-step<1).
    duration_frames: u16,
    /// Per-frame step in 16-bit facing units. Stored as `(rot_byte << 8)`.
    /// Zero means instant rotator (no interpolation).
    rot_per_frame: u16,
}

impl FacingClass {
    /// Construct a new FacingClass at the given initial facing with the
    /// given ROT byte. ROT byte is the value from rules.ini (e.g. 5 for
    /// War Miner, 10 for Harvester) before the binary's <<8 shift.
    pub fn new(initial: u16, rot_byte: u8) -> Self {
        let mut fc = Self {
            current: initial,
            prev: initial,
            start_frame: None,
            duration_frames: 0,
            rot_per_frame: 0,
        };
        fc.set_rot(rot_byte);
        fc
    }

    /// Update the rate of turn. Mirrors gamemd's SetROT (FUN_004C9680):
    /// clamps input > 126 to 127, then stores `(byte << 8)`.
    pub fn set_rot(&mut self, rot_byte: u8) {
        let clamped: u8 = if rot_byte > 0x7E { 0x7F } else { rot_byte };
        self.rot_per_frame = (clamped as u16) << 8;
    }

    /// Destination facing — where the rotation will end (regardless of
    /// where the animation currently is).
    pub fn destination(&self) -> u16 {
        self.current
    }

    /// Per-frame step value, exposed for tests and for callers that need
    /// to know the rotation rate.
    pub fn rot_per_frame(&self) -> u16 {
        self.rot_per_frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_at_given_facing() {
        let fc = FacingClass::new(12345, 5);
        assert_eq!(fc.destination(), 12345);
        assert_eq!(fc.rot_per_frame(), 5 * 256); // 5 << 8 = 1280
    }

    #[test]
    fn set_rot_clamps_at_0x7f() {
        let mut fc = FacingClass::new(0, 0);
        fc.set_rot(0x7E);
        assert_eq!(fc.rot_per_frame(), 0x7E00); // 126 << 8
        fc.set_rot(0x7F);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // 127 << 8
        fc.set_rot(0xFF);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // clamped to 127, 127 << 8
        fc.set_rot(200);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // clamped
    }

    #[test]
    fn set_rot_zero_means_instant() {
        let mut fc = FacingClass::new(100, 5);
        fc.set_rot(0);
        assert_eq!(fc.rot_per_frame(), 0);
    }
}
