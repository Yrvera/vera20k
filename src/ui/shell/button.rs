//! Pointer capture shared by physical shell buttons. Timer highlighting is
//! separate from pointer hover; callers retain eligibility and action authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellButtonInteraction<Button> {
    pub pressed: Option<Button>,
    pub hovered: Option<Button>,
}

impl<Button> Default for ShellButtonInteraction<Button> {
    fn default() -> Self {
        Self {
            pressed: None,
            hovered: None,
        }
    }
}

impl<Button: Copy + PartialEq> ShellButtonInteraction<Button> {
    /// Capture survives dragging outside; the native button's pushed state
    /// does not. 612B70 forwards mouse movement to the Windows BUTTON owner.
    pub fn is_pressed(&self, button: Button) -> bool {
        self.pressed == Some(button) && self.hovered == Some(button)
    }

    pub fn press(&mut self, over: Option<Button>) {
        self.hovered = over;
        self.pressed = over;
    }

    pub fn release(&mut self, over: Option<Button>) -> Option<Button> {
        self.hovered = over;
        self.pressed.take().filter(|held| Some(*held) == over)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_requires_the_original_button_and_consumes_capture() {
        let mut interaction = ShellButtonInteraction::default();
        interaction.press(Some(1));
        assert_eq!(interaction.release(Some(2)), None);
        assert_eq!(interaction.release(Some(1)), None);
        interaction.press(Some(1));
        assert_eq!(interaction.release(Some(1)), Some(1));
        assert_eq!(interaction.release(Some(1)), None);
    }

    #[test]
    fn dragging_off_releases_visual_state_but_preserves_return_capture() {
        let mut interaction = ShellButtonInteraction::default();
        interaction.press(Some(1));
        assert!(interaction.is_pressed(1));
        interaction.hovered = None;
        assert!(!interaction.is_pressed(1));
        assert_eq!(interaction.pressed, Some(1));
        interaction.hovered = Some(1);
        assert!(interaction.is_pressed(1));
        assert_eq!(interaction.release(Some(1)), Some(1));
        assert!(!interaction.is_pressed(1));
    }
}
