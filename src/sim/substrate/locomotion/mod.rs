//! Pure locomotion class identity, capability, and base-default services.
//!
//! This is the read-only half of locomotion. World mutation and installed
//! locomotor state belong in `sim::movement`; this module depends only on
//! low-level rules/util concepts and has no render, UI, audio, or net edge.

pub mod capability;
pub mod class;
pub mod defaults;

pub use capability::piggyback_capable;
pub use class::{
    CLSID_CLASS_TABLE, LocomotorClass, class_from_clsid, clsid_for_class,
};
pub use defaults::{
    BASE_DEFAULT_SLOTS, BaseDefaultSlot, can_enter_cell, do_turn,
    force_immediate_destination, force_new_slope, force_track, head_to_coord,
    inherits_base_default, is_moving_now, mark_all_occupation_bits,
    overrides_base_default, unlimbo,
};
