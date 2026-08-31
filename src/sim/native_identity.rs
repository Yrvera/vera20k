//! Active-YR numeric `AbstractClass` identity during fresh map construction.
//!
//! This wrapping signed-32-bit namespace is independent of Rust's monotonic
//! collision-free stable handles and of every RNG stream. Constructors
//! preincrement it; they neither search for nor prevent duplicate values.

pub(crate) const FRESH_SCENARIO_NATIVE_ID_SEED: u32 = 1_000_000;
pub(crate) const MAP_READ_NATIVE_ID_RESERVATION: u32 = 0x2710;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeFreshIdPhase {
    PrefixSaved,
    MapReadReserved,
}

/// One fresh Scenario's wrapping numeric-ID cursor.
#[derive(Debug)]
pub(crate) struct NativeUniqueIdCursor {
    value: u32,
    saved_after_fresh_prefix: u32,
    phase: NativeFreshIdPhase,
}

impl NativeUniqueIdCursor {
    fn from_saved_prefix(saved_after_fresh_prefix: u32) -> Self {
        Self {
            value: saved_after_fresh_prefix,
            saved_after_fresh_prefix,
            phase: NativeFreshIdPhase::PrefixSaved,
        }
    }

    /// Assign the next native ID. Addition deliberately wraps and the returned
    /// i32 preserves the resulting bit pattern.
    pub(crate) fn next_id(&mut self) -> i32 {
        self.value = self.value.wrapping_add(1);
        self.value as i32
    }

    /// Enter the map reader by setting from the saved Full_Init snapshot.
    ///
    /// Shadowed theater constructors may have advanced the current cursor in
    /// between; native overwrites that current value with `C_saved + 0x2710`.
    pub(crate) fn reserve_map_read_from_saved(&mut self) -> Result<u32, NativeIdentityError> {
        if self.phase != NativeFreshIdPhase::PrefixSaved {
            return Err(NativeIdentityError::MapReadReservationAlreadyApplied);
        }
        self.value = self
            .saved_after_fresh_prefix
            .wrapping_add(MAP_READ_NATIVE_ID_RESERVATION);
        self.phase = NativeFreshIdPhase::MapReadReserved;
        Ok(self.value)
    }

    pub(crate) fn current_raw(&self) -> u32 {
        self.value
    }

    #[cfg(test)]
    pub(crate) fn saved_after_fresh_prefix(&self) -> u32 {
        self.saved_after_fresh_prefix
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NativeIdentityError {
    #[error("fresh native-ID map-read reservation was already applied")]
    MapReadReservationAlreadyApplied,
}

/// Consumed-once native-ID half of the stock-offline pre-Fill plan.
#[derive(Debug)]
pub(crate) struct NativeFreshIdPrefixReceipt {
    cursor: NativeUniqueIdCursor,
    #[cfg(test)]
    checkpoints: NativeFreshIdPrefixCheckpoints,
}

impl NativeFreshIdPrefixReceipt {
    pub(crate) fn into_cursor(self) -> NativeUniqueIdCursor {
        self.cursor
    }

    #[cfg(test)]
    pub(crate) fn checkpoints(&self) -> NativeFreshIdPrefixCheckpoints {
        self.checkpoints
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeFreshIdPrefixCheckpoints {
    pub(crate) after_early_types: u32,
    pub(crate) after_first_house_generation: u32,
    pub(crate) after_first_resize: u32,
    pub(crate) after_rebuilt_types: u32,
    pub(crate) after_final_house_generation: u32,
    pub(crate) after_final_resize: u32,
}

/// Fold the exact noncampaign Full_Init constructor generations without
/// constructing a second registry or touching Scenario RNG.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_noncampaign_fresh_id_prefix(
    early_type_count: usize,
    first_super_weapon_type_count: usize,
    first_house_count: usize,
    rebuilt_type_count: usize,
    final_super_weapon_type_count: usize,
    final_house_count: usize,
    map_width: u32,
    map_height: u32,
) -> NativeFreshIdPrefixReceipt {
    let mut value = FRESH_SCENARIO_NATIVE_ID_SEED;
    value = advance(value, early_type_count);
    #[cfg(test)]
    let after_early_types = value;
    value = advance(
        value,
        house_block_count(first_house_count, first_super_weapon_type_count),
    );
    #[cfg(test)]
    let after_first_house_generation = value;
    value = value.wrapping_add(resize_constructor_count(map_width, map_height));
    #[cfg(test)]
    let after_first_resize = value;
    value = advance(value, rebuilt_type_count);
    #[cfg(test)]
    let after_rebuilt_types = value;
    value = advance(
        value,
        house_block_count(final_house_count, final_super_weapon_type_count),
    );
    #[cfg(test)]
    let after_final_house_generation = value;
    value = value.wrapping_add(resize_constructor_count(map_width, map_height));
    #[cfg(test)]
    let after_final_resize = value;

    NativeFreshIdPrefixReceipt {
        cursor: NativeUniqueIdCursor::from_saved_prefix(value),
        #[cfg(test)]
        checkpoints: NativeFreshIdPrefixCheckpoints {
            after_early_types,
            after_first_house_generation,
            after_first_resize,
            after_rebuilt_types,
            after_final_house_generation,
            after_final_resize,
        },
    }
}

fn advance(value: u32, count: usize) -> u32 {
    value.wrapping_add(count as u32)
}

fn house_block_count(house_count: usize, super_weapon_type_count: usize) -> usize {
    house_count.wrapping_mul(super_weapon_type_count.wrapping_add(1))
}

fn resize_constructor_count(map_width: u32, map_height: u32) -> u32 {
    map_height
        .wrapping_mul(map_width.wrapping_mul(2).wrapping_sub(1))
        .wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::{
        MAP_READ_NATIVE_ID_RESERVATION, NativeFreshIdPrefixCheckpoints,
        build_noncampaign_fresh_id_prefix,
    };

    #[test]
    fn fixture_b_folds_both_house_and_resize_generations_in_order() {
        let receipt = build_noncampaign_fresh_id_prefix(
            0, 2, 2, 5, 2, 2, 2, 3,
        );
        assert_eq!(
            receipt.checkpoints(),
            NativeFreshIdPrefixCheckpoints {
                after_early_types: 1_000_000,
                after_first_house_generation: 1_000_006,
                after_first_resize: 1_000_016,
                after_rebuilt_types: 1_000_021,
                after_final_house_generation: 1_000_027,
                after_final_resize: 1_000_037,
            }
        );
        assert_eq!(receipt.cursor.saved_after_fresh_prefix(), 1_000_037);
    }

    #[test]
    fn map_read_reservation_sets_from_wrapping_saved_snapshot() {
        let mut cursor = super::NativeUniqueIdCursor::from_saved_prefix(0xFFFF_FFF0);
        assert_eq!(cursor.next_id() as u32, 0xFFFF_FFF1);
        assert_eq!(
            cursor.reserve_map_read_from_saved().unwrap(),
            0xFFFF_FFF0u32.wrapping_add(MAP_READ_NATIVE_ID_RESERVATION)
        );
        assert_eq!(cursor.current_raw(), 0x0000_2700);
        assert!(cursor.reserve_map_read_from_saved().is_err());
    }
}
