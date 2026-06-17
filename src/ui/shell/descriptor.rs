//! Front-end shell dialog/control descriptors.
//!
//! Render-agnostic data describing a Win32-native shell dialog (Framework B) as
//! plain Rust: the dialog's controls, their raw DLU rects, and the per-control
//! re-anchor rule the layout pass applies (contract C7). Depends only on the
//! shared geometry primitives (no sim/render/assets), so it honors the ui/
//! layering rule. The wider controller/paint/modal/slide substrate consumes
//! these (see docs/plans/2026-05-31-shell-substrate-design.md §5/§6); this slice
//! drives the main-menu (0xE2) layout off a descriptor table feeding `layout_pass`.

use super::geom::RectPx;

/// Win32 dialog resource id (e.g. `0x00E2` main menu, `0x0100` single player,
/// `0x0102` skirmish). A newtype so dialog ids never mix with control ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogId(pub u16);

/// Owner-draw control class (subclass classification, study §2.2 / contract C6).
/// Only the kinds the migrated shells use today are exercised; the remaining
/// variants are placeholders the skirmish controls (Slice 4) will fill in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Button,
    Static,
    Checkbox,
    Radio,
    Combo,
    Listbox,
    Trackbar,
    Edit,
    ScrollBar,
}

/// Per-control re-anchor rule applied by `layout_pass` after the one-shot
/// DLU->pixel conversion (contract C7). Each variant is a faithful port of a
/// current main-menu (0xE2) helper; later shells extend this as they migrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorRule {
    /// Owner-draw button snapped round-half-up to the nearest 42-px SDBTNANM
    /// row, flush to the panel right edge at `cell_w`. Uses only the DLU top.
    /// (0xE2 stacked Single Player / WW Online / Network / Movies / Options.)
    OwnerDrawButtonSnap { cell_w: i32 },
    /// Owner-draw button kept at its raw DLU top (no row snap), flush-right at
    /// `cell_w`. Uses only the DLU top. (0xE2 Exit, which sits in the gap below
    /// the stack rather than snapping to a row.)
    OwnerDrawButtonRawTop { cell_w: i32 },
    /// Right-panel child: sidebar-inset, oversized-screen compensated, anchored
    /// to `panel.top.y + dlu_top`. (0xE2 Yuri-website static.) This is the main
    /// menu's convention specifically; single-player and skirmish anchor Y to
    /// `center_offset(screen_h, 600)` instead, so they will need their own
    /// variant when they migrate. Do NOT collapse this to one convention — that
    /// would silently shift the shipped 0xE2 title/website pixels.
    RightAnchor,
    /// `RightAnchor` then a fixed post-pass nudge `(dy, dh)` — the 1-px
    /// finalizer family. (0xE2 0x694 heading: +7 y, +1 h.)
    RightAnchorNudge { dy: i32, dh: i32 },
}

/// Background composition mode (study §C8): mode-1 right-panel shells vs mode-2
/// SHP modal. Only `RightPanelShell` is exercised this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgKind {
    /// Right-panel chrome + MNSCRN background (front-end shells 0xE2/0x100/0x102).
    RightPanelShell,
    /// PUDLGBGN.SHP modal panel + DIALOGN.PAL (roadmap; Slice 5).
    ModalShp,
    /// In-game Options dialog (0xBBB/0xF5): composited as an OVERLAY over the
    /// frozen battlefield — the template carries no opaque full-screen panel art
    /// and its statics are text-only (verified: every static in the 0xBBB
    /// template is a `GUI:*` caption/label, no image control). The exact backdrop/
    /// frame composition is resolved with the owner-draw paint sub-step (5a-ii).
    InGameOptions,
}

/// Reposition policy (contract C7 include-set gating). Include-set dialogs
/// (0xE2/0x6B/0x100/0x102) expand the parent fullscreen and re-anchor
/// allow-listed children; modal-centered dialogs (0xCE/0x120) do NOT re-anchor
/// (study §3 DRIFT-CORRECTED: those ids are excluded from the reposition pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositionPolicy {
    IncludeSetReanchor,
    ModalCentered,
    /// In-game Options dialog. 5a-i resolves it to the raw DLU->pixel client
    /// rect (baseline, see `layout::layout_pass`); the native child-resize helper
    /// family (centered offsets for ordinary controls + right-edge button
    /// anchoring from the runtime SIDEBTTN canvas) is layered on in 5a-ii.
    InGameOptions,
}

/// One control inside a shell dialog template.
#[derive(Debug, Clone)]
pub struct ControlDescriptor {
    /// Win32 control resource id (e.g. `0x0683` Single Player).
    pub id: u16,
    pub kind: ControlKind,
    /// Raw resource rect in dialog DLUs (pre-conversion). For the owner-draw
    /// button anchors only the DLU top is consumed; the x/w/h carry the template
    /// client rect the SDBTNANM cell replaces.
    pub dlu_rect: RectPx,
    pub anchor: AnchorRule,
    /// CSF label key for the control's caption (`None` for art-only controls).
    pub csf_key: Option<&'static str>,
    /// CSF key for the hover tooltip/status line (`None` if the control has none).
    pub tooltip_key: Option<&'static str>,
    /// Slide/paint group marker (study +0xD5). `0` = ungrouped.
    pub group: u8,
    /// Resource-template default. Runtime enable/disable (e.g. the single-player
    /// LoadSavedGame guard) layers over this in the controller, not here.
    pub enabled: bool,
    /// Resource-template `WS_VISIBLE`. A `false` control is created hidden and the
    /// dialog proc never shows it, so it is not emitted or hit-tested. (Active
    /// `0xBBB` hides the VisualDetails trackbar + its caption/value-label.)
    pub visible: bool,
}

/// A full shell dialog: its controls plus the dialog-level composition, slide,
/// and reposition policy.
#[derive(Debug, Clone)]
pub struct DialogDescriptor {
    pub id: DialogId,
    pub controls: Vec<ControlDescriptor>,
    pub bg_kind: BgKind,
    pub slide_eligible: bool,
    pub reposition_policy: RepositionPolicy,
}
