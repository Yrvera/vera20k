//! Sidebar gadget flash primitive.
//!
//! Mirrors the SBGadgetClass flash sub-struct (+0x34 state / +0x38 period /
//! +0x3c countdown / +0x1e disabled) and the three driver functions
//! Start_Flash / Stop_Flash / Flash_AI. One instance per pressable gadget
//! that may flash. See ra2-rust-game-docs/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md
//! for the source semantics.
//!
//! ## Dependency rules
//! - Part of `sidebar/` — pure data + logic, no rendering or sim dependencies.

/// Persistent flash state for one pressable gadget.
///
/// Mirrors three contiguous fields on an SBGadgetClass instance plus the
/// separate +0x1e disabled gate. All four are byte-for-byte semantic
/// counterparts of the binary; the field names use plain-English meanings
/// rather than the binary offsets.
#[derive(Debug, Clone, Copy, Default)]
pub struct GadgetFlash {
    /// "Draw as pressed" bit. 0 = idle visual, 1 = pressed-look visual.
    /// Toggled by `tick` when a flash period elapses.
    pub state: u8,

    /// Toggle interval in ticks AND the "is-flashing" sentinel (non-zero ⇒ active).
    /// Stays constant for the lifetime of an active flash; reset by `stop`.
    pub period: u32,

    /// Ticks remaining until the next toggle. On the first cycle this is
    /// `period + extra_delay`; on every subsequent cycle it resets to `period`.
    pub countdown: u32,

    /// Auto-stop gate. When set, the next `tick` zeros all three flash fields
    /// and reports a state change.
    pub disabled: bool,
}

impl GadgetFlash {
    /// Schedule a flash. No-op (returns `false`) if a flash is already active —
    /// matches the gamemd Start_Flash guard at +0x38 != 0.
    ///
    /// `extra_delay` is added to the FIRST countdown only; the steady-state
    /// toggle interval is `period`.
    pub fn start(&mut self, period: u32, extra_delay: u32, initial_state: u8) -> bool {
        if self.period != 0 {
            return false;
        }
        self.period = period;
        self.countdown = period + extra_delay;
        self.state = initial_state;
        true
    }

    /// Cancel any active flash. No-op (returns `false`) if not currently flashing.
    /// Field-write order matches the binary: state → countdown → period.
    pub fn stop(&mut self) -> bool {
        if self.period == 0 {
            return false;
        }
        self.state = 0;
        self.countdown = 0;
        self.period = 0;
        true
    }

    /// Advance one game-logic tick. Returns `true` when the visible state
    /// changed (caller marks redraw / picks a new frame index).
    pub fn tick(&mut self) -> bool {
        if self.disabled {
            if self.period != 0 {
                self.state = 0;
                self.countdown = 0;
                self.period = 0;
                return true;
            }
            return false;
        }
        if self.countdown == 0 {
            return false;
        }
        self.countdown -= 1;
        if self.countdown == 0 {
            self.state ^= 1;
            self.countdown = self.period;
            return true;
        }
        false
    }

    /// True while a flash is scheduled (period != 0).
    pub fn is_active(&self) -> bool {
        self.period != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_from_idle_initialises_all_fields() {
        let mut g = GadgetFlash::default();
        assert!(g.start(10, 7, 1));
        assert_eq!(g.period, 10);
        assert_eq!(g.countdown, 17, "first countdown is period + extra_delay");
        assert_eq!(g.state, 1);
        assert!(g.is_active());
    }

    #[test]
    fn start_while_active_is_noop() {
        let mut g = GadgetFlash::default();
        g.start(10, 7, 1);
        let snapshot = g;
        assert!(!g.start(20, 0, 0), "guard returns false");
        assert_eq!(g.period, snapshot.period);
        assert_eq!(g.countdown, snapshot.countdown);
        assert_eq!(g.state, snapshot.state);
    }

    #[test]
    fn stop_from_active_zeros_in_order() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 1);
        assert!(g.stop());
        assert_eq!(g.state, 0);
        assert_eq!(g.countdown, 0);
        assert_eq!(g.period, 0);
    }

    #[test]
    fn stop_from_idle_is_noop() {
        let mut g = GadgetFlash::default();
        assert!(!g.stop());
    }

    #[test]
    fn tick_decrements_countdown_until_toggle() {
        let mut g = GadgetFlash::default();
        // First cycle: period=10, extra_delay=5, initial=0 → countdown=15.
        g.start(10, 5, 0);
        for _ in 0..14 {
            assert!(!g.tick(), "no toggle yet");
        }
        assert!(g.tick(), "15th tick toggles");
        assert_eq!(g.state, 1, "state XOR-toggled to 1");
        assert_eq!(g.countdown, 10, "countdown resets to period, not period+extra");
    }

    #[test]
    fn tick_steady_state_toggles_every_period_ticks() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 0);
        // First cycle: countdown=10. 10 ticks → toggle.
        for _ in 0..9 {
            assert!(!g.tick());
        }
        assert!(g.tick());
        assert_eq!(g.state, 1);
        // Second cycle: countdown=10. 10 more ticks → toggle back to 0.
        for _ in 0..9 {
            assert!(!g.tick());
        }
        assert!(g.tick());
        assert_eq!(g.state, 0);
    }

    #[test]
    fn tick_when_idle_is_noop() {
        let mut g = GadgetFlash::default();
        assert!(!g.tick());
        assert_eq!(g.countdown, 0);
        assert_eq!(g.state, 0);
    }

    #[test]
    fn tick_when_disabled_auto_stops_active_flash() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 1);
        g.disabled = true;
        assert!(g.tick(), "auto-stop reports a change");
        assert_eq!(g.state, 0);
        assert_eq!(g.countdown, 0);
        assert_eq!(g.period, 0);
    }

    #[test]
    fn tick_when_disabled_and_idle_is_noop() {
        let mut g = GadgetFlash::default();
        g.disabled = true;
        assert!(!g.tick());
    }
}
