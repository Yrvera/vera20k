//! The generator's sine/cosine table, read from the retail executable.
//!
//! The river carver steers by a floating-point heading and resolves it through
//! two table lookups every step. The table is **not reproducible from a
//! formula**: computing `sin` in double precision and rounding disagrees with it
//! on 4997 of its 10240 entries, and so does the correctly-rounded true sine, by
//! up to one unit in the last place. It is an artifact of whatever generated it
//! in the original build. It is not even exactly periodic — one of the 2048
//! wrap-around pairs disagrees, which is what an accumulating recurrence looks
//! like and what a per-index evaluation does not.
//!
//! So the bytes have to come from the retail install. They are read out of
//! `gamemd.exe` the same way the rest of the engine reads retail `.mix` assets:
//! from the player's own copy, at load time. Nothing retail-derived lives in
//! this repository.
//!
//! Both trig functions share one table. Cosine is the same data read a quarter
//! period further along, so there is a single array and two index derivations.

use std::fmt;

/// Virtual address of the table in the retail image.
const TABLE_VA: u32 = 0x0084_F084;
/// Entries. The extra quarter period past a full turn is what lets the cosine
/// offset run past the end of the sine range without wrapping.
pub const TABLE_LEN: usize = 0x2800;
/// Entries in one full turn.
const PERIOD: i32 = 0x2000;
/// Quarter period: the offset that turns the sine table into a cosine table.
const QUARTER: i32 = 0x800;
/// Highest index each lookup will round *up* to.
const SIN_CEILING: i32 = PERIOD - 1;
const COS_CEILING: i32 = PERIOD + QUARTER - 1;

/// Caller angle units per full turn.
///
/// The index derivation halves the caller's value before using it as a table
/// index, so one caller unit is half a table step and a full turn is twice the
/// table period.
pub const UNITS_PER_TURN: f64 = (2 * PERIOD) as f64;

/// FNV-1a (64-bit) over the table's raw little-endian bytes in the retail
/// image, machine-derived by reading all 40960 bytes out of the binary. This is
/// the whole-table check: it is a genuine exhaustive comparison, not a sample.
pub const RETAIL_FNV1A64: u64 = 0x4642_825e_c40e_ce86;

/// Something went wrong reading the table out of the executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrigTableError {
    NotPeFile,
    UnmappedAddress(u32),
    Truncated { need: usize, have: usize },
}

impl fmt::Display for TrigTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPeFile => write!(f, "not a PE executable"),
            Self::UnmappedAddress(va) => {
                write!(f, "address {va:#010x} is not inside any section")
            }
            Self::Truncated { need, have } => {
                write!(f, "table needs {need} bytes, only {have} available")
            }
        }
    }
}

impl std::error::Error for TrigTableError {}

/// The retail sine table.
#[derive(Debug, Clone)]
pub struct TrigTable {
    entries: Vec<f32>,
}

/// Translate a virtual address to a file offset using the PE section table.
///
/// Parsed rather than hardcoded: a hardcoded offset silently reads the wrong
/// bytes if the executable is ever a different build, whereas a section walk
/// either finds the address or says it could not.
fn file_offset_of(image: &[u8], va: u32) -> Result<usize, TrigTableError> {
    let u16_at = |off: usize| -> Option<u16> {
        Some(u16::from_le_bytes(
            image.get(off..off + 2)?.try_into().ok()?,
        ))
    };
    let u32_at = |off: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            image.get(off..off + 4)?.try_into().ok()?,
        ))
    };

    if image.get(..2) != Some(b"MZ") {
        return Err(TrigTableError::NotPeFile);
    }
    let pe = u32_at(0x3C).ok_or(TrigTableError::NotPeFile)? as usize;
    if image.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err(TrigTableError::NotPeFile);
    }

    let sections = u16_at(pe + 6).ok_or(TrigTableError::NotPeFile)? as usize;
    let optional_size = u16_at(pe + 20).ok_or(TrigTableError::NotPeFile)? as usize;
    let optional = pe + 24;
    // PE32 optional header: ImageBase at +28.
    let image_base = u32_at(optional + 28).ok_or(TrigTableError::NotPeFile)?;
    let rva = va
        .checked_sub(image_base)
        .ok_or(TrigTableError::UnmappedAddress(va))?;

    let table = optional + optional_size;
    for i in 0..sections {
        let hdr = table + i * 40;
        let virtual_size = u32_at(hdr + 8).ok_or(TrigTableError::NotPeFile)?;
        let virtual_addr = u32_at(hdr + 12).ok_or(TrigTableError::NotPeFile)?;
        let raw_size = u32_at(hdr + 16).ok_or(TrigTableError::NotPeFile)?;
        let raw_ptr = u32_at(hdr + 20).ok_or(TrigTableError::NotPeFile)?;
        // Sections are often larger in memory than on disk (bss-style tail);
        // the span that actually exists in the file is the raw size.
        let span = virtual_size.max(raw_size);
        if rva >= virtual_addr && rva < virtual_addr.saturating_add(span) {
            return Ok((raw_ptr + (rva - virtual_addr)) as usize);
        }
    }
    Err(TrigTableError::UnmappedAddress(va))
}

impl TrigTable {
    /// Read the table out of a retail `gamemd.exe` image.
    pub fn from_executable(image: &[u8]) -> Result<Self, TrigTableError> {
        let start = file_offset_of(image, TABLE_VA)?;
        let need = TABLE_LEN * 4;
        let bytes = image
            .get(start..start + need)
            .ok_or(TrigTableError::Truncated {
                need,
                have: image.len().saturating_sub(start),
            })?;
        let entries = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Ok(Self { entries })
    }

    /// FNV-1a over the raw little-endian bytes, for comparison against
    /// [`RETAIL_FNV1A64`].
    pub fn fnv1a64(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for entry in &self.entries {
            for byte in entry.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }

    /// Is this the table the retail build shipped?
    pub fn matches_retail(&self) -> bool {
        self.entries.len() == TABLE_LEN && self.fnv1a64() == RETAIL_FNV1A64
    }

    pub fn entry(&self, index: usize) -> f32 {
        self.entries[index]
    }

    /// Sine of an angle given in caller units.
    pub fn sin(&self, angle_units: i32) -> f32 {
        self.entries[sin_index(angle_units) as usize]
    }

    /// Cosine of an angle given in caller units.
    pub fn cos(&self, angle_units: i32) -> f32 {
        self.entries[cos_index(angle_units) as usize]
    }
}

/// Fold a caller's angle into the table's index range.
///
/// The mask keeps the sign bit and the low 13 bits, then the odd-looking
/// `(x - 1) | 0xFFFF_E000` then `+ 1` sign-extends that 13-bit value back to a
/// negative number. Written out rather than tidied because the two callers
/// diverge on what they do with a still-negative result.
/// All arithmetic wraps. The masked value can be `i32::MIN` (sign bit set, low
/// bits clear), and subtracting one from it is exactly the case the original
/// wraps through without noticing — a checked subtraction would panic on an
/// angle the original handles fine.
fn fold(angle_units: i32) -> (i32, bool) {
    let raw = angle_units;
    let mut index = (raw / 2) & 0x8000_1FFFu32 as i32;
    let mut wrapped_negative = false;
    if index < 0 {
        let extended = index.wrapping_sub(1) | 0xFFFF_E000u32 as i32;
        index = extended.wrapping_add(1);
        if index < 0 {
            index = extended;
            wrapped_negative = true;
        }
    }
    (index, wrapped_negative)
}

fn sin_index(angle_units: i32) -> i32 {
    let (folded, wrapped) = fold(angle_units);
    let mut index = if wrapped {
        folded.wrapping_add(PERIOD + 1)
    } else {
        folded
    };
    if angle_units & 1 != 0 && index < SIN_CEILING {
        index += 1;
    }
    index
}

fn cos_index(angle_units: i32) -> i32 {
    let (folded, wrapped) = fold(angle_units);
    let mut index = if wrapped {
        folded.wrapping_add(PERIOD + QUARTER + 1)
    } else {
        folded + QUARTER
    };
    if angle_units & 1 != 0 && index < COS_CEILING {
        index += 1;
    }
    index
}

/// Convert radians to the caller units the lookups take.
pub fn radians_to_units(radians: f64) -> f64 {
    radians * (UNITS_PER_TURN / std::f64::consts::TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where the retail install lives. The table check is skipped, loudly, when
    /// it is absent — the same convention the retail tile-resolution pass uses.
    fn retail_executable() -> Option<Vec<u8>> {
        let dir = std::env::var("RA2_DIR").ok()?;
        std::fs::read(std::path::Path::new(&dir).join("gamemd.exe")).ok()
    }

    #[test]
    fn indices_stay_inside_the_table() {
        // Every representable caller angle, at a stride that still visits every
        // residue class of the fold.
        for raw in (i32::MIN..=i32::MAX - 1).step_by(9973) {
            for index in [sin_index(raw), cos_index(raw)] {
                assert!(
                    (0..TABLE_LEN as i32).contains(&index),
                    "angle {raw} produced out-of-range index {index}"
                );
            }
        }
    }

    /// Cosine is the sine table a quarter period along — that relationship is
    /// the whole reason there is one array and not two.
    #[test]
    fn cosine_leads_sine_by_a_quarter_period() {
        for raw in (-40_000..40_000).step_by(7) {
            assert_eq!(
                cos_index(raw),
                sin_index(raw) + QUARTER,
                "angle {raw}: cosine is not a quarter period ahead of sine"
            );
        }
    }

    /// Periodicity holds for **even** caller units only.
    ///
    /// The fold halves the angle with a truncation toward zero, and that is not
    /// symmetric about zero: an odd negative angle rounds up where its positive
    /// counterpart a turn away rounds down, so the two land one index apart.
    /// This is the original's behaviour, not a defect — the first version of
    /// this test asserted periodicity for all angles and failed on -7997 vs
    /// 8387, which is how the asymmetry was found.
    #[test]
    fn a_full_turn_returns_to_the_same_index_for_even_angles() {
        let turn = UNITS_PER_TURN as i32;
        for raw in (-8_000..8_000).step_by(2) {
            assert_eq!(
                sin_index(raw),
                sin_index(raw + turn),
                "even angle {raw} and {} disagree a full turn apart",
                raw + turn
            );
        }
    }

    /// And the odd-angle asymmetry itself, pinned so it cannot be "fixed" later
    /// by someone who assumes it is a bug.
    #[test]
    fn odd_negative_angles_are_asymmetric_across_a_turn() {
        let turn = UNITS_PER_TURN as i32;
        assert_eq!(sin_index(-7997) - sin_index(-7997 + turn), 1);
    }

    /// The exhaustive check. Not a sample: every one of the 40960 bytes feeds
    /// the hash, and the expected value was derived by reading them out of the
    /// binary rather than computed by hand.
    #[test]
    fn the_table_matches_the_retail_image_byte_for_byte() {
        let Some(image) = retail_executable() else {
            eprintln!("skipped: set RA2_DIR to the retail install to run this");
            return;
        };
        let table = TrigTable::from_executable(&image).expect("read the table");
        assert_eq!(table.entries.len(), TABLE_LEN);
        assert_eq!(
            table.fnv1a64(),
            RETAIL_FNV1A64,
            "the table read out of gamemd.exe is not the one this port was \
             written against"
        );
        assert!(table.matches_retail());
    }

    /// Anchors that would catch a table read at the wrong offset even if a hash
    /// somehow agreed, and that pin the identification as *sine*: a cosine table
    /// would have to start at 1.0.
    #[test]
    fn the_table_has_the_shape_of_a_sine() {
        let Some(image) = retail_executable() else {
            eprintln!("skipped: set RA2_DIR to the retail install to run this");
            return;
        };
        let table = TrigTable::from_executable(&image).expect("read the table");
        assert_eq!(table.entry(0), 0.0, "sine starts at zero");
        assert_eq!(table.entry(PERIOD as usize / 4), 1.0, "quarter turn is one");
        assert!(
            table.entry(PERIOD as usize / 2).abs() < 1e-6,
            "half turn is zero"
        );
        assert_eq!(
            table.entry(3 * PERIOD as usize / 4),
            -1.0,
            "three-quarter turn is minus one"
        );
        // The quarter-period offset really does behave like a cosine.
        assert_eq!(table.sin(0), 0.0);
        assert_eq!(table.cos(0), 1.0);
    }

    #[test]
    fn a_non_pe_input_is_refused_rather_than_misread() {
        assert_eq!(
            TrigTable::from_executable(b"not an executable").unwrap_err(),
            TrigTableError::NotPeFile
        );
    }
}
