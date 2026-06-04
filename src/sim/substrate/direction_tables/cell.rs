//! gamemd 8-direction cell-delta table + stepping accessors.
//!
//! Single sim-facing entry point for the "which adjacent cell" primitive. The
//! values are the PARITY-verified `util::direction::DIRECTION_DELTAS`
//! (re-exported; sim may depend on util). Compass order 0=N..7=NW, +X=east,
//! +Y=south; gamemd runtime-init at the foundation direction-table initializer.

use crate::util::direction::DIRECTION_DELTAS;

/// gamemd 8-direction cell-delta table, compass order. Canonical reference
/// (identical to `util::direction::DIRECTION_DELTAS`).
pub const CELL_DELTAS: [(i32, i32); 8] = DIRECTION_DELTAS;

/// Checked cell-delta. `None` for `dir > 7` (incl. the tube sentinel 8) — the
/// safe sim accessor.
pub fn cell_delta(dir: u8) -> Option<(i32, i32)> {
    CELL_DELTAS.get(dir as usize).copied()
}

/// Faithful mirror of gamemd's unchecked `MapCoord_Step_By_Direction` indexing
/// (no mask/bounds; callers sanitize upstream). Debug-asserts `dir <= 7` and
/// masks `&7` to stay memory-safe; use only when mirroring that contract.
pub fn cell_delta_unchecked(dir: u8) -> (i32, i32) {
    debug_assert!(dir <= 7, "cell_delta_unchecked: dir {dir} > 7 (gamemd OOB read)");
    CELL_DELTAS[(dir & 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_delta_table_equals_gamemd_dump() {
        // gamemd 0x0089F688, decoded from init 0x0049F2F0 (study Verification Log #1).
        let expected = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        assert_eq!(CELL_DELTAS, expected);
        for (i, &e) in expected.iter().enumerate() {
            assert_eq!(cell_delta(i as u8), Some(e));
        }
        assert_eq!(cell_delta(8), None); // tube sentinel, not a 9th compass dir
        assert_eq!(cell_delta(255), None);
    }
}
