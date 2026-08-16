//! Input owner (F12): tactical/menu input dispatch, hotkeys, context orders,
//! command scheduling, camera, cursor, entity picking, sidebar gadget input,
//! in-game options input, tooltips, and messages.

pub(crate) mod camera;
pub(crate) mod commands;
pub(crate) mod context_order;
pub(crate) mod dispatch;
pub(crate) mod cursor;
pub(crate) mod entity_pick;
pub(crate) mod gadget_input;
pub(crate) mod hotkeys;
pub(crate) mod state;
pub(crate) mod in_game_options;
pub(crate) mod messages;
pub(crate) mod tooltips;
