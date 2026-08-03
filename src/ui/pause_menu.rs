//! In-scenario modal menu — the in-game menu and the abort-mission confirmation.
//!
//! ## What gamemd does
//!
//! A scenario carries one in-scenario state variable. Zero means "no modal, the
//! world runs"; every other value names a modal the scenario state machine
//! dispatches to. The states reachable in a stock offline skirmish are:
//!
//! | state | dialog | how it is entered | where it goes |
//! |---|---|---|---|
//! | 0 | none | every other state falls back here | the mission runs |
//! | 1 | the in-game menu | the sidebar menu control | its buttons |
//! | 3 | abort-mission confirm | the menu's Abort Mission button | see below |
//! | 5 | Game Controls / Options | the menu's Options button | back to 1 |
//!
//! Two re-entry rules matter to the player and are reproduced here:
//!
//! * **Options is a child of the menu.** When the Options dialog returns, the
//!   state machine re-enters state 1 — closing Options puts you back on the
//!   menu, it does not resume the mission.
//! * **Cancelling the abort confirmation resumes the mission**, it does not
//!   return to the menu: the confirm dialog's cancel result matches no case in
//!   the state machine's switch, so it falls through to state 0.
//!
//! The in-game menu's own dialog template is selected by game mode; offline
//! campaign and skirmish share one template whose controls are Load Game, Save
//! Game, Delete Game, Game Controls, Abort Mission, Resume Mission, plus a
//! campaign-only mission-restate control. Only Game Controls, Abort Mission and
//! Resume Mission are wired here; the three save-game controls drive dialogs
//! this port does not have yet.
//!
//! The abort confirmation is a two-button dialog whose second button is mode
//! dependent: campaign labels it `GUI:Restart` (restart the scenario),
//! multiplayer with two or more live human players labels it `GUI:Observe`, and
//! **offline skirmish hides it outright**. What is left in skirmish is one
//! action button captioned `GUI:Leave` and a cancel. Confirming Leave queues an
//! EXIT event for the local player; when that event executes it raises the
//! graceful-exit session flag, which tears the session down *without* the
//! victory or defeat teardown — no result screen, no outcome announcement.
//!
//! Evidence: live decompilation of gamemd.exe this session — the scenario state
//! machine, the in-game menu modal and its dialog proc, the abort-confirm modal
//! and its dialog proc, the exit-event helper, `EventClass::Execute`'s EXIT
//! case, and the session-end router.
//!
//! ## Dependency rules
//! - Part of ui/ — no dependencies on render/, assets/, sim/, audio/.
//! - The state enum is pure data with pure transitions; the app layer owns the
//!   side effects (freezing the sim, tearing the match down).

use crate::ui::client_theme;

/// In-scenario modal state. Mirrors gamemd's in-scenario state variable,
/// restricted to the values a stock offline skirmish can reach.
///
/// States deliberately absent: the surrender-and-be-scored state (gated on a
/// WOL-only mode), the replay state, the campaign mission-restate state, the
/// multiplayer objectives state, and the Keyboard/Sound sub-dialogs of Options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InGameMenuState {
    /// No modal is open — the mission runs. (gamemd: 0)
    #[default]
    Closed,
    /// The in-game menu. (gamemd: 1)
    Menu,
    /// The abort-mission confirmation. (gamemd: 3)
    AbortConfirm,
    /// Game Controls / Options — a child of [`InGameMenuState::Menu`]. (gamemd: 5)
    Options,
}

impl InGameMenuState {
    /// True while any in-scenario modal owns the screen. The app layer freezes
    /// the simulation for exactly this condition.
    pub fn is_open(self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Where Escape goes from here.
    ///
    /// gamemd's keyboard binding for the in-game menu was not traced, so the
    /// Escape *binding* is VERA-internal and UNCHECKED against gamemd; the
    /// destinations below are the verified button routes it stands in for.
    pub fn on_escape(self) -> Self {
        match self {
            // Nothing open: Escape stands in for the sidebar menu control.
            Self::Closed => Self::Menu,
            // Resume Mission.
            Self::Menu => Self::Closed,
            // Cancelling the confirmation resumes the mission — it does not
            // return to the menu.
            Self::AbortConfirm => Self::Closed,
            // Options is a child of the menu.
            Self::Options => Self::Menu,
        }
    }
}

/// Does an Escape press belong to the in-scenario modal machine?
///
/// Escape is this port's stand-in for gamemd's sidebar menu control (the
/// binding itself is VERA-internal; gamemd's keyboard route into the menu is
/// UNCHECKED). It only reaches the machine when no in-world mode is armed:
/// while a placement/targeting cursor or a repair/sell mode is live and no
/// modal is open, Escape cancels that instead.
pub fn escape_belongs_to_modal_machine(state: InGameMenuState, in_world_mode_armed: bool) -> bool {
    state.is_open() || !in_world_mode_armed
}

/// What the player picked on the in-game menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InGameMenuAction {
    /// Nothing this frame.
    None,
    /// Resume Mission.
    Resume,
    /// Game Controls — opens the Options dialog as a child of the menu.
    GameControls,
    /// Abort Mission — opens the confirmation.
    Abort,
}

/// What the player picked on the abort-mission confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortConfirmAction {
    /// Nothing this frame.
    None,
    /// Cancel — resume the mission.
    Cancel,
    /// Leave — end the session through the graceful-exit route.
    Leave,
}

/// What the app layer must do once a modal reports the player's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalOutcome {
    /// Nothing was picked this frame — the modal stays as it is.
    Stay,
    /// Move the in-scenario state variable to this value.
    Enter(InGameMenuState),
    /// End the running match by the graceful-exit route.
    LeaveMatch,
}

/// The in-game menu's button routes.
///
/// Resume dismisses the modal (the state machine clears the variable to 0),
/// Game Controls writes state 5 and Abort Mission writes state 3.
pub fn resolve_menu_action(action: InGameMenuAction) -> ModalOutcome {
    match action {
        InGameMenuAction::None => ModalOutcome::Stay,
        InGameMenuAction::Resume => ModalOutcome::Enter(InGameMenuState::Closed),
        InGameMenuAction::GameControls => ModalOutcome::Enter(InGameMenuState::Options),
        InGameMenuAction::Abort => ModalOutcome::Enter(InGameMenuState::AbortConfirm),
    }
}

/// The abort confirmation's button routes.
///
/// Cancel resumes the mission — it does **not** return to the menu, because
/// gamemd's cancel result matches no case in the state machine's switch and
/// falls through to state 0. Confirming leaves the match.
pub fn resolve_abort_action(action: AbortConfirmAction) -> ModalOutcome {
    match action {
        AbortConfirmAction::None => ModalOutcome::Stay,
        AbortConfirmAction::Cancel => ModalOutcome::Enter(InGameMenuState::Closed),
        AbortConfirmAction::Leave => ModalOutcome::LeaveMatch,
    }
}

/// Width of both in-scenario cards, in points.
const CARD_WIDTH: f32 = 340.0;
/// Size of a primary button on both cards, in points.
const BUTTON_SIZE: egui::Vec2 = egui::vec2(240.0, 40.0);
/// Backdrop alpha over the frozen battlefield.
const BACKDROP_ALPHA: u8 = 140;

/// Dim the frozen battlefield behind an in-scenario modal.
fn draw_backdrop(ctx: &egui::Context, id: &'static str) {
    egui::Area::new(id.into())
        .fixed_pos(egui::pos2(0.0, 0.0))
        .interactable(false)
        .show(ctx, |ui| {
            let screen: egui::Rect = ctx.content_rect();
            ui.painter().rect_filled(
                screen,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, BACKDROP_ALPHA),
            );
        });
}

/// Draw the in-game menu over the frozen battlefield.
///
/// Presentation only: this is an egui stand-in for the native dialog template,
/// which needs shell descriptor/layout work this module does not own. The
/// button set and the routes are the verified ones.
pub fn draw_in_game_menu(ctx: &egui::Context) -> InGameMenuAction {
    let palette = client_theme::apply_client_theme(ctx);
    let mut action = InGameMenuAction::None;

    draw_backdrop(ctx, "in_game_menu_backdrop");

    egui::Window::new("")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(client_theme::card_frame(palette.panel, palette.line))
        .min_width(CARD_WIDTH)
        .show(ctx, |ui| {
            ui.set_max_width(CARD_WIDTH);
            ui.vertical_centered(|ui| {
                client_theme::section_label(ui, "MISSION", palette);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Options")
                        .size(30.0)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(18.0);

                if ui
                    .add_sized(
                        BUTTON_SIZE,
                        egui::Button::new(
                            egui::RichText::new("Resume Mission").size(17.0).strong(),
                        ),
                    )
                    .clicked()
                {
                    action = InGameMenuAction::Resume;
                }
                ui.add_space(10.0);
                if ui
                    .add_sized(
                        BUTTON_SIZE,
                        egui::Button::new(egui::RichText::new("Game Controls").size(17.0)),
                    )
                    .clicked()
                {
                    action = InGameMenuAction::GameControls;
                }
                ui.add_space(10.0);
                if ui
                    .add_sized(
                        BUTTON_SIZE,
                        egui::Button::new(
                            egui::RichText::new("Abort Mission")
                                .size(17.0)
                                .color(palette.danger),
                        ),
                    )
                    .clicked()
                {
                    action = InGameMenuAction::Abort;
                }
                ui.add_space(6.0);
            });
        });

    action
}

/// Draw the abort-mission confirmation over the frozen battlefield.
///
/// `leave_label` is the caption for the confirm button; gamemd loads it from
/// the string table under `GUI:Leave`, so the app layer resolves that key and
/// passes the result through.
///
/// The second action button of the native dialog is not drawn: it is hidden in
/// offline skirmish, and its campaign (restart scenario) and multiplayer
/// (become an observer) variants have no route in this build.
pub fn draw_abort_confirm(ctx: &egui::Context, leave_label: &str) -> AbortConfirmAction {
    let palette = client_theme::apply_client_theme(ctx);
    let mut action = AbortConfirmAction::None;

    draw_backdrop(ctx, "abort_confirm_backdrop");

    egui::Window::new("")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(client_theme::card_frame(palette.panel, palette.line))
        .min_width(CARD_WIDTH)
        .show(ctx, |ui| {
            ui.set_max_width(CARD_WIDTH);
            ui.vertical_centered(|ui| {
                client_theme::section_label(ui, "ABORT MISSION", palette);
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Do you want to abort the current mission?")
                        .size(16.0)
                        .color(palette.text),
                );
                ui.add_space(18.0);

                if ui
                    .add_sized(
                        BUTTON_SIZE,
                        egui::Button::new(
                            egui::RichText::new(leave_label)
                                .size(17.0)
                                .strong()
                                .color(palette.danger),
                        ),
                    )
                    .clicked()
                {
                    action = AbortConfirmAction::Leave;
                }
                ui.add_space(10.0);
                if ui
                    .add_sized(
                        BUTTON_SIZE,
                        egui::Button::new(egui::RichText::new("Cancel").size(17.0)),
                    )
                    .clicked()
                {
                    action = AbortConfirmAction::Cancel;
                }
                ui.add_space(6.0);
            });
        });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mission runs only at state 0; every modal freezes it.
    #[test]
    fn only_the_closed_state_lets_the_mission_run() {
        assert!(!InGameMenuState::Closed.is_open());
        assert!(InGameMenuState::Menu.is_open());
        assert!(InGameMenuState::AbortConfirm.is_open());
        assert!(InGameMenuState::Options.is_open());
    }

    /// Doing nothing on either card leaves the modal exactly where it was.
    #[test]
    fn no_choice_leaves_the_modal_alone() {
        assert_eq!(
            resolve_menu_action(InGameMenuAction::None),
            ModalOutcome::Stay
        );
        assert_eq!(
            resolve_abort_action(AbortConfirmAction::None),
            ModalOutcome::Stay
        );
    }

    /// Options is a child of the menu: Game Controls enters it, and both ways
    /// out of it — its own Back control and Escape — return to the menu (5 → 1)
    /// rather than resuming the mission.
    #[test]
    fn options_returns_to_the_menu_not_to_the_mission() {
        assert_eq!(
            resolve_menu_action(InGameMenuAction::GameControls),
            ModalOutcome::Enter(InGameMenuState::Options)
        );
        assert_eq!(InGameMenuState::Options.on_escape(), InGameMenuState::Menu);
    }

    /// The menu's own dismissal resumes the mission.
    #[test]
    fn resume_dismisses_the_menu() {
        assert_eq!(
            resolve_menu_action(InGameMenuAction::Resume),
            ModalOutcome::Enter(InGameMenuState::Closed)
        );
    }

    /// Cancelling the abort confirmation ends at state 0 — never back at the
    /// menu — and so does dismissing it with Escape.
    #[test]
    fn abort_cancel_resumes_the_mission_instead_of_returning_to_the_menu() {
        assert_eq!(
            resolve_abort_action(AbortConfirmAction::Cancel),
            ModalOutcome::Enter(InGameMenuState::Closed)
        );
        assert_eq!(
            InGameMenuState::AbortConfirm.on_escape(),
            InGameMenuState::Closed
        );
    }

    /// The full skirmish route out of a match: Abort Mission opens the
    /// confirmation, and confirming leaves the match rather than moving the
    /// state variable.
    #[test]
    fn abort_then_confirm_leaves_the_match() {
        assert_eq!(
            resolve_menu_action(InGameMenuAction::Abort),
            ModalOutcome::Enter(InGameMenuState::AbortConfirm)
        );
        assert_eq!(
            resolve_abort_action(AbortConfirmAction::Leave),
            ModalOutcome::LeaveMatch
        );
    }

    /// Escape opens the menu from the mission and resumes from the menu.
    #[test]
    fn escape_opens_and_closes_the_menu() {
        assert_eq!(InGameMenuState::Closed.on_escape(), InGameMenuState::Menu);
        assert_eq!(InGameMenuState::Menu.on_escape(), InGameMenuState::Closed);
    }

    /// An armed in-world mode takes Escape ahead of opening the menu, but never
    /// ahead of a modal that is already up.
    #[test]
    fn in_world_cancel_outranks_opening_the_menu_only_while_closed() {
        assert!(!escape_belongs_to_modal_machine(
            InGameMenuState::Closed,
            true
        ));
        assert!(escape_belongs_to_modal_machine(
            InGameMenuState::Closed,
            false
        ));
        for open in [
            InGameMenuState::Menu,
            InGameMenuState::AbortConfirm,
            InGameMenuState::Options,
        ] {
            assert!(escape_belongs_to_modal_machine(open, true));
        }
    }
}
