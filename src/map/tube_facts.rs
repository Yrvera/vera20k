//! Immutable TubeClass-shaped map facts.
//!
//! The original engine stores TubeClass objects in a global array and stores a
//! per-cell tube index on CellClass. Rust keeps the static map-load facts here;
//! sim systems decide whether damage/state currently makes a tube usable.

/// Compact TubeClass array index.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TubeId(pub u16);

impl TubeId {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// Why this tube exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TubeSource {
    /// Auto-created by CellClass::RecalcAttributes for a tunnel/low-bridge cell.
    AutoLowBridge,
    /// Explicit map tube data with a real TubeClass path buffer.
    ExplicitMap,
}

/// TubeClass fields that affect pathing and movement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TubeFact {
    pub entry: (u16, u16),
    pub exit: (u16, u16),
    pub direction: u8,
    /// Explicit `[Tubes]` can later populate this. Auto low-bridge tubes have
    /// path_len=0 and binary-fills the unused 100-slot buffer with -1.
    pub path_steps: Vec<u8>,
    pub source: TubeSource,
}

impl TubeFact {
    pub fn auto_low_bridge(cell: (u16, u16), direction: u8) -> Self {
        Self {
            entry: cell,
            exit: cell,
            direction,
            path_steps: Vec::new(),
            source: TubeSource::AutoLowBridge,
        }
    }

    pub fn explicit(
        entry: (u16, u16),
        exit: (u16, u16),
        direction: u8,
        path_steps: Vec<u8>,
    ) -> Self {
        Self {
            entry,
            exit,
            direction,
            path_steps,
            source: TubeSource::ExplicitMap,
        }
    }

    pub fn path_len(&self) -> usize {
        self.path_steps.len()
    }

    pub fn path_steps(&self) -> &[u8] {
        &self.path_steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_low_bridge_tube_is_same_cell_zero_step_shell() {
        let tube = TubeFact::auto_low_bridge((12, 34), 2);

        assert_eq!(tube.entry, (12, 34));
        assert_eq!(tube.exit, (12, 34));
        assert_eq!(tube.direction, 2);
        assert_eq!(tube.path_len(), 0);
        assert_eq!(tube.source, TubeSource::AutoLowBridge);
    }

    #[test]
    fn explicit_tube_preserves_path_steps() {
        let tube = TubeFact::explicit((1, 1), (4, 1), 2, vec![2, 2, 2]);

        assert_eq!(tube.entry, (1, 1));
        assert_eq!(tube.exit, (4, 1));
        assert_eq!(tube.path_steps, vec![2, 2, 2]);
        assert_eq!(tube.path_len(), 3);
        assert_eq!(tube.source, TubeSource::ExplicitMap);
    }
}
