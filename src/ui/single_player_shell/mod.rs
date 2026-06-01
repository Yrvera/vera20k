//! Single Player intermediate shell dialog 0x100 layout and input state.

mod layout;
mod state;

pub use layout::{SinglePlayerButtonRect, SinglePlayerShellLayout, compute_layout};
pub use state::{
    SinglePlayerControlId, SinglePlayerShellAction, SinglePlayerShellState, action_for_control,
    csf_key_for_control, return_code_for_action,
};
