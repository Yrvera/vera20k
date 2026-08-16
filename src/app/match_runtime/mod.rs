//! Match runtime owner (F12): the per-frame simulation advance transaction,
//! the local frame pacer, and the scenario exit cascade.

pub(crate) mod frame_pacer;
pub(crate) mod scenario_exit;
pub(crate) mod sim_tick;
pub(crate) mod state;
