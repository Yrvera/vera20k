//! Initial main-menu shell dialog 0xE2 layout and input state.

mod layout;
mod state;

pub use layout::{
    MainMenuButtonRect, MainMenuMovieBase, MainMenuShellLayout, RIGHT_PANEL_TILE_H,
    RIGHT_PANEL_WIDTH, RectPx, compute_layout, compute_responsive_layout,
    movie_base_for_screen_width,
};
pub use state::{
    MainMenuControlId, MainMenuShellAction, MainMenuShellState, action_for_control,
    csf_key_for_control, hit_test_owner_draw_button, mouse_down, mouse_move, mouse_up,
    return_code_for_action, tooltip_csf_key_for_control,
};
