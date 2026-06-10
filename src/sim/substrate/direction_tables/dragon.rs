//! DRAGON 32-way rotating-SHP frame table + index formula. gamemd source
//! `0x007F4890` = `i32[32]` where `table[i] = (28 - i) & 31` (study Verification
//! Log #4, read_memory 0x007F4890 len128). Used for `Rotates=yes` projectiles
//! (DRAGON / AAHeatSeeker2). Rust previously lacked it (closes DRIFT D3 at the
//! data layer; the app_fire_effects cutover is a later slice).

/// gamemd DRAGON 32-way frame map `0x007F4890`: `table[i] = (28 - i) & 31`,
/// i.e. `[28,27,…,1,0,31,30,29]`. `&31` on i32 wraps the negative tail correctly
/// (`-1 & 31 == 31`).
pub const DRAGON_FRAME_TABLE: [i32; 32] = {
    let mut out = [0i32; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (28 - i as i32) & 31;
        i += 1;
    }
    out
};

/// DRAGON 32-way frame index from a BAM (binary-angle) value:
/// `index = (((bam) >> 10) + 1) >> 1 & 0x1F` (study §5).
pub fn dragon_frame_index(bam: u16) -> usize {
    ((((bam >> 10) + 1) >> 1) & 0x1F) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_frame_table_equals_gamemd_dump() {
        // read_memory 0x007F4890 len128 (study Verification Log #4).
        let expected: [i32; 32] = [
            28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7,
            6, 5, 4, 3, 2, 1, 0, 31, 30, 29,
        ];
        assert_eq!(DRAGON_FRAME_TABLE, expected);
        for i in 0..32 {
            assert_eq!(DRAGON_FRAME_TABLE[i], (28 - i as i32) & 31);
        }
    }

    #[test]
    fn dragon_frame_index_formula() {
        for bam in [0u16, 0x0400, 0x0800, 0x8000, 0xFC00, 0xFFFF] {
            assert_eq!(
                dragon_frame_index(bam),
                ((((bam >> 10) + 1) >> 1) & 0x1F) as usize
            );
        }
        assert_eq!(dragon_frame_index(0), 0);
    }
}
