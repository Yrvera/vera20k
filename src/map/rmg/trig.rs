//! Random-map compatibility namespace for the engine-wide retail trig table.

pub(crate) use crate::map::retail_trig::file_offset_of;
pub use crate::map::retail_trig::{
    RETAIL_FNV1A64, TABLE_LEN, TrigTable, TrigTableError, UNITS_PER_TURN, global, install_from_dir,
    radians_to_units,
};
