//! Persistent native MapClass crate-slot authority.

pub(crate) const CRATE_SLOT_CAPACITY: usize = 256;

/// One native 16-byte crate slot (`MapClass+0x158`, physical array order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct CrateSlot {
    pub(crate) start_frame: i32,
    pub(crate) timer_aux: u32,
    pub(crate) duration_frames: i32,
    pub(crate) cell_x: i16,
    pub(crate) cell_y: i16,
}

impl Default for CrateSlot {
    fn default() -> Self {
        // gamemd-derived: `MapClass::Init_Clear @ 0x005659F0` establishes this
        // exact tuple. Coordinate alone discriminates empty from occupied.
        Self {
            start_frame: -1,
            timer_aux: 0,
            duration_frames: 0,
            cell_x: 0,
            cell_y: 0,
        }
    }
}

impl CrateSlot {
    pub(crate) const fn is_occupied(self) -> bool {
        self.cell_x != 0 || self.cell_y != 0
    }

    pub(crate) const fn cell(self) -> Option<(i16, i16)> {
        if self.is_occupied() {
            Some((self.cell_x, self.cell_y))
        } else {
            None
        }
    }

    /// Native `CDTimerClass::Expired` shape consumed by
    /// `MapClass__UpdateCrateRegenTimers @ 0x0056BBE0`.
    pub(crate) const fn expired(self, current_frame: i32) -> bool {
        if self.start_frame == -1 {
            self.duration_frames == 0
        } else {
            current_frame.wrapping_sub(self.start_frame) >= self.duration_frames
        }
    }

    /// Preserve/rebase the remaining timer after the removal attempt.
    pub(crate) fn clear_coordinate_and_preserve_timer(&mut self, current_frame: i32) {
        self.cell_x = 0;
        self.cell_y = 0;
        if self.start_frame != -1 {
            let elapsed = current_frame.wrapping_sub(self.start_frame);
            self.duration_frames = self.duration_frames.wrapping_sub(elapsed);
        }
        self.start_frame = -1;
    }
}

/// The complete persistent MapClass crate authority. Slot order is physical
/// and duplicates/ghosts are intentionally retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrateAuthority {
    pub(crate) slots: [CrateSlot; CRATE_SLOT_CAPACITY],
    pub(crate) pickup_any_latch: bool,
}

impl serde::Serialize for CrateAuthority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut out = serializer.serialize_struct("CrateAuthority", 2)?;
        out.serialize_field("slots", self.slots.as_slice())?;
        out.serialize_field("pickup_any_latch", &self.pickup_any_latch)?;
        out.end()
    }
}

impl<'de> serde::Deserialize<'de> for CrateAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Wire {
            slots: Vec<CrateSlot>,
            #[serde(default)]
            pickup_any_latch: bool,
        }

        let wire = Wire::deserialize(deserializer)?;
        let slots = wire.slots.try_into().map_err(|slots: Vec<CrateSlot>| {
            serde::de::Error::invalid_length(slots.len(), &"exactly 256 native crate slots")
        })?;
        Ok(Self {
            slots,
            pickup_any_latch: wire.pickup_any_latch,
        })
    }
}

impl Default for CrateAuthority {
    fn default() -> Self {
        Self {
            slots: [CrateSlot::default(); CRATE_SLOT_CAPACITY],
            pickup_any_latch: false,
        }
    }
}

impl CrateAuthority {
    pub(crate) fn first_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|slot| !slot.is_occupied())
    }

    pub(crate) fn first_slot_at(&self, cell: (u16, u16)) -> Option<usize> {
        let packed = (cell.0 as i16, cell.1 as i16);
        self.slots
            .iter()
            .position(|slot| slot.cell() == Some(packed))
    }
}
