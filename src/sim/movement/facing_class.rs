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

    /// Animated facing at the given binary frame. Pure function of state.
    ///
    /// Returns `current` when:
    /// - rot_per_frame == 0 (instant rotator)
    /// - start_frame is None (no rotation initiated)
    /// - elapsed >= duration_frames (rotation complete)
    /// - step_size < 1 (rotation request smaller than one frame's ROT — snaps)
    ///
    /// Otherwise interpolates linearly along the shortest signed arc from
    /// prev to current at exactly rot_per_frame units per frame.
    pub fn current(&self, binary_frame: u32) -> u16 {
        if self.rot_per_frame == 0 {
            return self.current;
        }
        let Some(start) = self.start_frame else {
            return self.current;
        };
        let elapsed: u32 = binary_frame.saturating_sub(start);
        if elapsed >= self.duration_frames as u32 {
            return self.current;
        }
        let remaining: u16 = self.duration_frames - elapsed as u16;

        // Signed short subtraction gives shortest signed delta.
        // 0xFFE0 → 0x0010 wraps to +0x30, not -0xFFD0.
        let diff: i16 = self.current.wrapping_sub(self.prev) as i16;

        // step_size < 1 snaps (research doc §2.2).
        let step_size: u16 = diff.unsigned_abs() / self.rot_per_frame;
        if step_size < 1 {
            return self.current;
        }

        // animated = current - sign(diff) * rot_per_frame * remaining
        // (equivalent to: prev + sign(diff) * rot_per_frame * elapsed)
        let signed_step: i32 = (diff.signum() as i32) * (self.rot_per_frame as i32);
        let delta: i32 = signed_step * (remaining as i32);
        ((self.current as i32) - delta).rem_euclid(65536) as u16
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

    /// Helper: construct a FacingClass mid-rotation (skips set() so we can
    /// test current() in isolation).
    fn mid_rotation(prev: u16, current: u16, start: u32, duration: u16, rot_byte: u8) -> FacingClass {
        let mut fc = FacingClass::new(current, rot_byte);
        fc.prev = prev;
        fc.start_frame = Some(start);
        fc.duration_frames = duration;
        fc
    }

    #[test]
    fn current_returns_destination_when_rot_zero() {
        let mut fc = mid_rotation(0, 1000, 0, 10, 5);
        fc.set_rot(0);
        assert_eq!(fc.current(0), 1000);
        assert_eq!(fc.current(5), 1000);
        assert_eq!(fc.current(100), 1000);
    }

    #[test]
    fn current_returns_destination_when_no_start_frame() {
        let fc = FacingClass::new(1000, 5);
        // start_frame = None
        assert_eq!(fc.current(0), 1000);
        assert_eq!(fc.current(50), 1000);
    }

    #[test]
    fn current_returns_destination_when_elapsed_exceeds_duration() {
        // prev=0, current=12800 (10 frames at ROT=5 → 1280/frame), duration=10.
        let fc = mid_rotation(0, 12800, 0, 10, 5);
        assert_eq!(fc.current(10), 12800); // exactly at end
        assert_eq!(fc.current(15), 12800); // past end
        assert_eq!(fc.current(100), 12800);
    }

    #[test]
    fn current_interpolates_linearly() {
        // prev=0, current=12800 (= 10 frames * 1280), duration=10.
        // At elapsed=5, animated = 0 + 5 * 1280 = 6400.
        // Equivalently: animated = current - 5 * 1280 = 12800 - 6400 = 6400.
        let fc = mid_rotation(0, 12800, 0, 10, 5);
        assert_eq!(fc.current(0), 0);    // remaining=10, animated = 12800 - 1280*10 = 0
        assert_eq!(fc.current(1), 1280);
        assert_eq!(fc.current(5), 6400);
        assert_eq!(fc.current(9), 11520);
    }

    #[test]
    fn current_snaps_when_step_size_below_one() {
        // diff = current - prev = 100; rot_per_frame = 1280; step_size = 100/1280 = 0.
        // Should snap to current immediately.
        let fc = mid_rotation(0, 100, 0, 0, 5);
        assert_eq!(fc.current(0), 100);
    }

    #[test]
    fn current_handles_wrap_around_short_path() {
        // 0xFFE0 → 0x0010: shortest signed delta is +0x30 (48 units), not -0xFFD0.
        // ROT=1 byte → 256/frame; duration = 48/256 = 0 → snaps. Use larger arc.
        // Let's go 0xFF00 → 0x0100: signed diff = 0x0100 - 0xFF00 (as i16) = 0x0200 = 512.
        // ROT=1, rot_per_frame=256; duration = 512/256 = 2.
        let fc = mid_rotation(0xFF00, 0x0100, 0, 2, 1);
        // At elapsed=0: animated = current - sign(+) * 256 * 2 = 0x0100 - 512 = 0xFF00.
        assert_eq!(fc.current(0), 0xFF00);
        // At elapsed=1: animated = 0x0100 - 256 = 0 (i.e., 0x0000, just past wrap).
        assert_eq!(fc.current(1), 0x0000);
        // At elapsed=2: complete.
        assert_eq!(fc.current(2), 0x0100);
    }

    #[test]
    fn current_handles_wrap_around_short_path_negative_diff() {
        // 0x0100 → 0xFF00: signed diff = (0xFF00 - 0x0100) as i16 = 0xFE00 = -512.
        // shortest path is COUNTER-clockwise by 512 units (back through 0).
        let fc = mid_rotation(0x0100, 0xFF00, 0, 2, 1);
        // At elapsed=0: animated = 0xFF00 - sign(-) * 256 * 2 = 0xFF00 + 512 = 0x0100.
        assert_eq!(fc.current(0), 0x0100);
        // At elapsed=1: animated = 0xFF00 + 256 = 0x0000.
        assert_eq!(fc.current(1), 0x0000);
        // At elapsed=2: complete.
        assert_eq!(fc.current(2), 0xFF00);
    }
}
