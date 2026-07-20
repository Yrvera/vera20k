//! Table-based approximate square root used for every distance the start
//! phase computes.
//!
//! The original does NOT take an FPU square root for these paths: it narrows
//! the input to single precision, splits exponent and mantissa, and rebuilds
//! a float from a 16384-entry mantissa table plus the halved exponent. The
//! result differs from a true sqrt in the low mantissa bits, and those bits
//! decide strict comparisons in the start selector — so a real `sqrt()` here
//! would drift start placement.
//!
//! The retail table is exactly `trunc((sqrt(v) - 1) * 2^23)` over each
//! bucket's *starting* value `v`, verified entry-for-entry against the retail
//! binary bytes (see the golden test below), so the table is generated at
//! startup instead of shipping binary data.

use std::sync::OnceLock;

/// Entries: 24-bit mantissa (implicit bit folded in for odd exponents)
/// bucketed by its top 14 bits.
const TABLE_LEN: usize = 16384;
/// Sub-tile mantissa resolution: each bucket spans 1024 mantissa values.
const BUCKET: u32 = 1024;
/// f32 mantissa width and implicit-one bit.
const MANTISSA_BITS: u32 = 23;
const IMPLICIT_ONE: u32 = 1 << MANTISSA_BITS;

fn table() -> &'static [u32; TABLE_LEN] {
    static TABLE: OnceLock<[u32; TABLE_LEN]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut entries = [0u32; TABLE_LEN];
        for (index, entry) in entries.iter_mut().enumerate() {
            let m24 = index as u32 * BUCKET;
            // Bucket start value: [1, 2) for even exponents, [2, 4) with the
            // implicit bit folded in for odd ones.
            let value = if m24 >= IMPLICIT_ONE {
                f64::from(m24) / f64::from(IMPLICIT_ONE) * 2.0
            } else {
                1.0 + f64::from(m24) / f64::from(IMPLICIT_ONE)
            };
            *entry = ((value.sqrt() - 1.0) * f64::from(IMPLICIT_ONE)) as u32;
        }
        entries
    })
}

/// The approximate square root, single-precision in and out.
///
/// Semantics ported literally: zero returns zero, negatives are mirrored to
/// their absolute value, odd exponents fold the implicit bit into the
/// mantissa before the bucket lookup, and the exponent is halved with an
/// arithmetic shift (so sub-one inputs round their exponent toward -inf).
pub fn sqrt_approx(value: f64) -> f32 {
    let narrowed = value as f32;
    if narrowed == 0.0 {
        return 0.0;
    }
    let bits = narrowed.abs().to_bits();
    let mut mantissa = bits & (IMPLICIT_ONE - 1);
    let exponent = (bits >> MANTISSA_BITS) as i32 - 127;
    if exponent & 1 != 0 {
        mantissa |= IMPLICIT_ONE;
    }
    let half_exponent = (exponent as i16) >> 1;
    let result_bits = table()[(mantissa / BUCKET) as usize]
        .wrapping_add(((i32::from(half_exponent) + 127) as u32) << MANTISSA_BITS);
    f32::from_bits(result_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retail bytes of the table, dumped from the game binary's data section.
    /// Machine-derived golden; regenerate by re-extracting the 64 KiB at the
    /// table's address from the retail executable.
    const RETAIL_TABLE: &[u8] = include_bytes!("../../../ini/sqrt_table.bin");

    #[test]
    fn generated_table_is_bit_identical_to_retail() {
        assert_eq!(RETAIL_TABLE.len(), TABLE_LEN * 4);
        for (index, entry) in table().iter().enumerate() {
            let retail =
                u32::from_le_bytes(RETAIL_TABLE[index * 4..index * 4 + 4].try_into().unwrap());
            assert_eq!(*entry, retail, "table entry {index}");
        }
    }

    #[test]
    fn zero_and_negatives() {
        assert_eq!(sqrt_approx(0.0), 0.0);
        assert_eq!(sqrt_approx(-4.0), sqrt_approx(4.0), "negatives mirror");
    }

    #[test]
    fn exact_squares_land_close_but_not_exact() {
        // The approximation is close to the true root but intentionally NOT
        // equal to it — asserting inexactness guards against someone
        // "simplifying" this into f32::sqrt.
        assert_eq!(sqrt_approx(1.0), 1.0, "1.0 is exact (entry 0 is 0)");
        let value = sqrt_approx(2.0);
        assert!((f64::from(value) - std::f64::consts::SQRT_2).abs() < 1e-4);
        let value = sqrt_approx(100.0);
        assert!((f64::from(value) - 10.0).abs() < 1e-3);
        // Mid-bucket inputs take the bucket-start root: visibly off a true
        // sqrt, which is the point of keeping the table.
        assert_ne!(
            sqrt_approx(1_234_567.0),
            1_234_567.0f32.sqrt(),
            "table root differs from a real square root"
        );
    }

    #[test]
    fn integer_distance_inputs_are_monotone() {
        // The start phase feeds dx*dx + dy*dy squared distances; the bucketed
        // root may tie between adjacent inputs but must never decrease.
        let mut previous = 0.0f32;
        for squared in 0..2000 {
            let value = sqrt_approx(f64::from(squared));
            assert!(value >= previous, "non-monotone at {squared}");
            previous = value;
        }
    }
}
