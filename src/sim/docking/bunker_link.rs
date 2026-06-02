//! Tank-bunker reciprocal link helpers (install / break / release trio).
//!
//! Owns the writes to both sides of the bunker link (`GameEntity.bunker_link` on
//! the unit, `GameEntity.bunker_occupant` on the building) plus the three distinct
//! teardown helpers and the admission predicate. Fleshed out in Tasks 4/5.
//!
//! sim/ only — never render/ui/sidebar/audio/net.
