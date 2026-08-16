//! gamemd lepton-delta (8-direction, sub-cell) table — the integer per-tick
//! locomotor step vector = cell-delta ×256. gamemd source
//! `g_DirectionDeltaX/Y_Table @ 0x0089F6D8` (runtime-init; study Verification
//! Log #2). 256 leptons = 1 cell. This is the exact integer table gamemd uses
//! for the 8-direction body translation — NOT sin/cos (closes DRIFT D1 at the
//! data layer; locomotor cutover is a later slice).

use super::cell::CELL_DELTAS;

const LEPTONS_PER_CELL: i32 = 256;

/// 8-direction lepton-delta table = `CELL_DELTAS[i] * 256`, compass order.
/// Const-derived from the (proven-identical) cell table so it cannot drift.
pub const LEPTON_DELTAS: [(i32, i32); 8] = {
    let mut out = [(0i32, 0i32); 8];
    let mut i = 0;
    while i < 8 {
        out[i] = (
            CELL_DELTAS[i].0 * LEPTONS_PER_CELL,
            CELL_DELTAS[i].1 * LEPTONS_PER_CELL,
        );
        i += 1;
    }
    out
};

/// Checked lepton-delta for a direction. `None` for `dir > 7`.
pub fn lepton_delta(dir: u8) -> Option<(i32, i32)> {
    LEPTON_DELTAS.get(dir as usize).copied()
}

/// Signed lepton→cell toward zero, matching gamemd `(v + (v>>31 & 0xFF)) >> 8`.
pub fn lepton_to_cell(v: i32) -> i32 {
    (v + ((v >> 31) & 0xFF)) >> 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lepton_delta_table_equals_gamemd_dump() {
        // gamemd 0x0089F6D8 (study Verification Log #2): cell ×256.
        let expected = [
            (0, -256),
            (256, -256),
            (256, 0),
            (256, 256),
            (0, 256),
            (-256, 256),
            (-256, 0),
            (-256, -256),
        ];
        assert_eq!(LEPTON_DELTAS, expected);
        // Diagonal step is exactly ±256 per axis, NOT the ±181 sin/cos diagonal.
        assert_eq!(lepton_delta(1), Some((256, -256)));
        assert_eq!(lepton_delta(8), None);
    }

    #[test]
    fn lepton_is_cell_times_256() {
        for i in 0..8 {
            assert_eq!(
                LEPTON_DELTAS[i],
                (CELL_DELTAS[i].0 * 256, CELL_DELTAS[i].1 * 256)
            );
        }
    }

    #[test]
    fn lepton_to_cell_rounds_toward_zero() {
        assert_eq!(lepton_to_cell(256), 1);
        assert_eq!(lepton_to_cell(-256), -1);
        assert_eq!(lepton_to_cell(255), 0);
        assert_eq!(lepton_to_cell(-1), 0);
        assert_eq!(lepton_to_cell(-255), 0);
        assert_eq!(lepton_to_cell(384), 1); // 1.5 cells → 1 toward zero
        assert_eq!(lepton_to_cell(-384), -1);
    }
}
