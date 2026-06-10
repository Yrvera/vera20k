//! Facing / direction lookup-table substrate — pure, read-only, deterministic
//! services for the gamemd "which-way / where-next" table family (cell-delta,
//! lepton-delta, facing↔direction quantization, DRAGON 32-way frame). Tables are
//! gamemd-exact, proven by exact-equality tests; no shadow→invert.
//!
//! Foundation slice (S1–S4): canonical sim-facing tables. Drive-track tables (S5)
//! and consumer cutovers (S6+) are later slices.
//!
//! ## Dependency rules
//! - Part of sim/substrate — depends only on util/. No render/ui/audio/net.

pub mod cell;
pub mod dragon;
pub mod lepton;
pub mod quantize;

pub use cell::{CELL_DELTAS, cell_delta, cell_delta_unchecked};
pub use dragon::{DRAGON_FRAME_TABLE, dragon_frame_index};
pub use lepton::{LEPTON_DELTAS, lepton_delta, lepton_to_cell};
pub use quantize::{
    dir_from_facing8, dir_from_facing16, facing8_to_16, muzzle_anim_index_8way, opposite_dir,
};
