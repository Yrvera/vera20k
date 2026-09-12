//! Saved-browser adapter for the shared618D40/61C690 list geometry.
use super::layout::SavedSeedControl;
use crate::ui::shell::list::ListScrollPart;
pub use crate::ui::shell::list::{ROW_HEIGHT, ShellListGeometry as SeedListGeometry, thumb_height};

impl SeedListGeometry {
    pub fn scroll_control_at(&self, x: i32, y: i32) -> Option<SavedSeedControl> {
        self.scroll_part_at(x, y).map(|part| match part {
            ListScrollPart::Up => SavedSeedControl::ScrollUp,
            ListScrollPart::Down => SavedSeedControl::ScrollDown,
            ListScrollPart::Thumb => SavedSeedControl::ScrollThumb,
            ListScrollPart::Track => SavedSeedControl::ScrollTrack,
        })
    }
}
