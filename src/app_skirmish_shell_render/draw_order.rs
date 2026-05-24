//! Semantic draw ordering for the skirmish shell renderer.
//!
//! This module is app-layer render planning only. It preserves the verified
//! relative paint order used by the sprite construction path.

use std::sync::Once;

use crate::ui::skirmish_shell::SkirmishShellLayout;

static HIGH_RES_PARENT_BACKGROUND_LOG: Once = Once::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParentBackgroundRole {
    Mnscrns640,
    CoopGameSetup800,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LowerStripRole {
    Lwscrns640,
    LwscrnlLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishShellDrawRole {
    ParentBackgroundMnscrns640,
    ParentBackgroundCoopGameSetup800,
    ChooseMapBackgroundCustomizeBattle800,
    ChooseMapModalBackdrop,
    ChooseMapListbox,
    ChooseMapOwnerDrawButton,
    ChooseMapPreviewStatic,
    ValidationModal,
    ValidationModalButton,
    LowerSideLwscrns,
    LowerSideLwscrnl,
    RightPanelTopSdtp,
    RightPanelTopHighlightSdtpFrame1,
    RightPanelTileSdbtnbkgd,
    RightPanelOverlaySdbtnanmFrame10,
    RightPanelBottomSdbtm,
    RightPanelMapButtonSdmpbtn,
    OwnerDrawButton,
    PreviewSurface,
    StartMarker,
    StartMarkerLabel,
    Flag,
}

pub(super) fn parent_background_role(layout: &SkirmishShellLayout) -> Option<ParentBackgroundRole> {
    match layout.screen.w {
        640 => Some(ParentBackgroundRole::Mnscrns640),
        800 => Some(ParentBackgroundRole::CoopGameSetup800),
        width => {
            if width > 800 {
                HIGH_RES_PARENT_BACKGROUND_LOG.call_once(|| {
                    log::info!(
                        "Skirmish shell parent background skipped for {width}px width; Ghidra verifies no fresh >800 parent substitution"
                    );
                });
            }
            None
        }
    }
}

pub(super) fn lower_strip_role(layout: &SkirmishShellLayout) -> LowerStripRole {
    match layout.screen.w {
        640 => LowerStripRole::Lwscrns640,
        _ => LowerStripRole::LwscrnlLarge,
    }
}

pub fn skirmish_shell_semantic_draw_order(
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
    preview_surface_available: bool,
    start_marker_overlay_available: bool,
    flag_count: usize,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    roles.push(SkirmishShellDrawRole::RightPanelTopSdtp);
    roles.extend(
        std::iter::repeat(SkirmishShellDrawRole::RightPanelTileSdbtnbkgd)
            .take(layout.right_panel.tile_count.max(0) as usize),
    );
    if overlay_frame10_active {
        roles.extend(
            std::iter::repeat(SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10)
                .take(layout.right_panel.tile_count.max(0) as usize),
        );
    }
    roles.push(SkirmishShellDrawRole::RightPanelBottomSdbtm);
    roles.push(match lower_strip_role(layout) {
        LowerStripRole::Lwscrns640 => SkirmishShellDrawRole::LowerSideLwscrns,
        LowerStripRole::LwscrnlLarge => SkirmishShellDrawRole::LowerSideLwscrnl,
    });
    if let Some(role) = parent_background_role(layout) {
        roles.push(match role {
            ParentBackgroundRole::Mnscrns640 => SkirmishShellDrawRole::ParentBackgroundMnscrns640,
            ParentBackgroundRole::CoopGameSetup800 => {
                SkirmishShellDrawRole::ParentBackgroundCoopGameSetup800
            }
        });
    }
    roles.push(SkirmishShellDrawRole::RightPanelTopHighlightSdtpFrame1);
    roles.push(SkirmishShellDrawRole::RightPanelMapButtonSdmpbtn);
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::OwnerDrawButton).take(3));
    if preview_surface_available {
        roles.push(SkirmishShellDrawRole::PreviewSurface);
    }
    if start_marker_overlay_available {
        roles.push(SkirmishShellDrawRole::StartMarker);
        roles.push(SkirmishShellDrawRole::StartMarkerLabel);
    }
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::Flag).take(flag_count));
    roles
}

pub fn choose_map_modal_semantic_draw_order(
    customize_battle_background_available: bool,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    if customize_battle_background_available {
        roles.push(SkirmishShellDrawRole::ChooseMapBackgroundCustomizeBattle800);
    } else {
        roles.push(SkirmishShellDrawRole::ChooseMapModalBackdrop);
    }
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::ChooseMapListbox).take(2));
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::ChooseMapOwnerDrawButton).take(3));
    roles.push(SkirmishShellDrawRole::ChooseMapPreviewStatic);
    roles
}

pub fn validation_modal_semantic_draw_order() -> Vec<SkirmishShellDrawRole> {
    vec![
        SkirmishShellDrawRole::ValidationModal,
        SkirmishShellDrawRole::ValidationModalButton,
    ]
}
