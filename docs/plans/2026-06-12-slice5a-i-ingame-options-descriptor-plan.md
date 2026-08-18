# Slice 5a-i — In-Game Options Dialog: Descriptor + Layout Baseline — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Pure data +
> geometry only — NO paint, input, INI, assets, or egui changes (those are 5a-ii/5a-iii).

**Goal:** Add the render-agnostic descriptor for the active in-game Options dialog
(`0xBBB`) — its nine interactive controls at verified DLU rects — plus the raw
DLU→pixel layout baseline and the `BgKind`/`RepositionPolicy` enum scaffolding,
all unit-tested.

**Architecture:** Lands entirely in `src/ui/shell/` (Framework-B shell substrate),
which depends on nothing above the ui/ layer. Mirrors the existing `modal.rs`
pattern (a dialog-specific descriptor builder feeding the shared `descriptor`/
`layout`/`geom` primitives). No `sim/`, `render/`, `assets/`, or egui involvement.

**Design Doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md`

---

## Grounding Summary

- **Docs:** `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` (High confidence)
  gives the verified `0xBBB` control table — 3 buttons (`0x686` Back, `0x52C` Keyboard,
  `0x52D` Sound), 3 trackbars (`0x529` GameSpeed, `0x52A` ScrollRate, `0x52B`
  VisualDetails), 3 checkboxes (`0x601` TargetLines, `0x604` ShowHidden, `0x602`
  Tooltips) — each with its DLU rect. `0xBBB` template has 17 controls; the remaining
  ~8 are text statics (title `0x694`, caption `0x714`, labels `0x671/0x672/0x673`,
  footer `0x695`, +2 unidentified), whose DLU rects are NOT in the doc.
- **Ghidra/chrome grounding** (`OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`):
  refined the design twice. (1) The dialog's screen-relative layout is the native
  child-resize helper family (`FUN_0060B000/B350/B1D0/B7A0/B950`): ordinary controls get
  centered screen offsets (active scenario), buttons get right-edge anchoring whose x
  depends on the **runtime `SIDEBTTN.SHP` canvas size** (offset `0x93`). So button
  pixel-x is NOT purely computable — it needs the asset (→ 5a-ii). (2) There is **no
  opaque full-screen Options background**; the dialog composites over the frozen
  battlefield and its statics are text-only.
- **Repo pattern:** `src/ui/shell/modal.rs` — a dialog-specific module that builds a
  `DialogDescriptor` from verified DLU rects + dedicated layout helpers, using
  `descriptor::{ControlKind, DialogDescriptor, ...}` and `geom::dlu_rect`. We mirror it.
  `layout.rs::layout_pass` already has a raw-DLU→pixel baseline arm (`ModalCentered`).
- **INI keys:** none consumed in 5a-i (persistence is 5a-iii).
- **Still unknown (→ 5a-ii, not this step):** the 8 static rects + the 2 unidentified
  controls (need `read_memory 0x00C01B18` template transcription); the B-helper
  anchoring; `SIDEBTTN.SHP`/`SIDEBAR.PAL` load; overlay composition; the `0xF5` shell
  descriptor (its full control rects are not all verified in the docs).

## Key Technical Decisions

- **5a-i = descriptor + raw-DLU→pixel baseline only.** The faithful screen-relative
  anchoring (centered offsets for ordinary controls + button right-edge anchoring) is
  deferred to 5a-ii because the button anchoring needs the runtime `SIDEBTTN` canvas
  size and the `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS` semantics. — **Confidence:**
  high — **Source:** `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS` rect-anchoring section.
  *(Refinement of design §4 5a-i, which assumed a simple center/stretch projection.)*
- **`BgKind::InGameOptions` is an overlay, not panel art.** — **Confidence:** high —
  **Source:** chrome-assets doc (no image static activated; "renders the game in the
  background").
- **5a-i builds `0xBBB` only; no `in_active_game` param yet.** `0xF5`'s full control
  rects are not all verified in the docs, so authoring its branch now would invent
  rects. The id-level split already exists in `modal.rs::ModalKind::InGameOptions::
  template_id`; the `0xF5` descriptor is a clean follow-on once its template is
  transcribed. — **Confidence:** high — **Source:** proc doc §3 (`0xF5` table omits
  checkbox rects). *(Refinement of design §8 Q2, which suggested authoring the flag now.)*
- **Statics deferred to 5a-ii.** Their DLU rects aren't in the docs and they are
  render-only text; transcribing the template + rendering the text belong together in
  the paint step. — **Confidence:** high — **Source:** proc doc §3.
- **Reuse `layout_pass` with a new `InGameOptions` arm** (baseline = raw `dlu_rect`,
  same body as `ModalCentered`) rather than a parallel layout fn, so 5a-ii extends the
  arm in place. — **Confidence:** high — **Source:** repo pattern `layout.rs`.

## Open Questions

### Resolved During Planning
- *Does the Options dialog have full-screen background art?* No — overlay over frozen
  game (chrome-assets doc).
- *Can the 800×600/1024×768 projection be done purely in 5a-i?* No for buttons (asset
  dependent); deferred whole to 5a-ii.
- *Will the new enum variants break other matches?* No — `layout_pass` is the only
  exhaustive `RepositionPolicy` match (gets the new arm); `BgKind` has no exhaustive
  match in the crate (verified via grep over `src/`).

### Deferred to Implementation (later 5a sub-steps, not 5a-i)
- Exact static rects + the 2 unidentified `0xBBB` controls (5a-ii template transcription).
- `SIDEBTTN.SHP` canvas dimensions for button anchoring (5a-ii, read from the asset).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/ui/shell/descriptor.rs` | Add `BgKind::InGameOptions` + `RepositionPolicy::InGameOptions` variants |
| Modify | `src/ui/shell/layout.rs` | Add the `RepositionPolicy::InGameOptions` baseline arm + test |
| Create | `src/ui/shell/in_game_options.rs` | `0xBBB` descriptor builder + control id consts + unit tests |
| Modify | `src/ui/shell/mod.rs` | `pub mod in_game_options;` |

## Interface Changes

- **`BgKind` gains `InGameOptions`** (public enum in `descriptor.rs`). No exhaustive
  match consumes `BgKind` today (only construction + `==`), so no downstream break.
- **`RepositionPolicy` gains `InGameOptions`** (public enum). The only exhaustive match
  is `layout.rs::layout_pass` — Task 2 adds its arm. Verified no other match via grep.
- **New public fn `build_in_game_options_descriptor()`** in `ui::shell::in_game_options`.
  No existing caller; consumed by 5a-ii.

## Sim Checklist
N/A — no `sim/` files touched. (ui/shell depends on nothing above the ui/ layer; this
plan adds no sim state, no tick-order change, no float-in-sim risk.)

## Risk Areas
- **Enum-variant exhaustiveness** — mitigated: grep confirmed `layout_pass` is the sole
  exhaustive `RepositionPolicy` match and `BgKind` has none. Task 6 build catches any
  miss.
- **Frozen suites must stay green & unchanged** — `src/ui/skirmish_shell/state/tests.rs`
  (87) and `src/ui/skirmish_shell/layout.rs` (30) are not touched by this plan; Task 6
  confirms they still pass. The `src/ui/shell/layout.rs` suite gains one test (allowed —
  that is this slice's file).
- **Inventing rects** — avoided by scoping to the 9 doc-verified controls; statics/`0xF5`
  deferred.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | The nine `0xBBB` control DLU rects | A 1-DLU error shifts a control every time the player opens in-game Options; these are the parity anchor for 5a-ii paint/hit-test | Rects transcribed verbatim from proc doc §3; Task 5 tests assert each `dlu_rect` and its DLU→pixel result |
| Task 4 | `ControlKind` per control (Button/Trackbar/Checkbox) | Drives the 5a-ii owner-draw paint dispatch and hit-test family | Task 5 `control_kinds_match_template` test |
| Task 2 | Baseline layout = raw `dlu_rect` (no anchor) | Must be a faithful passthrough now so 5a-ii can layer the verified B-helper anchoring without hidden offsets | Task 2 test asserts `== geom::dlu_rect(...)` |

---

## Tasks

### Task 1: Add the two enum variants

**Why:** The descriptor needs a background mode and reposition policy for the in-game
Options dialog; both enums currently lack them. Done first so later files can name them.

**Files:**
- Modify: `src/ui/shell/descriptor.rs` (`BgKind` ~line 61-67, `RepositionPolicy` ~line 73-77)

**Pattern:** Existing variant doc-comment style in the same enums.

**Step 1: Add the `BgKind` variant.** In `descriptor.rs`, extend the `BgKind` enum:
```rust
    /// PUDLGBGN.SHP modal panel + DIALOGN.PAL (roadmap; Slice 5).
    ModalShp,
    /// In-game Options dialog (0xBBB/0xF5): composited as an OVERLAY over the
    /// frozen battlefield — there is no opaque full-screen Options panel art, and
    /// its statics are text-only. The exact backdrop/frame composition is resolved
    /// with the owner-draw paint sub-step (5a-ii).
    InGameOptions,
```

**Step 2: Add the `RepositionPolicy` variant.** Extend `RepositionPolicy`:
```rust
    IncludeSetReanchor,
    ModalCentered,
    /// In-game Options dialog. 5a-i resolves it to the raw DLU->pixel client rect
    /// (baseline, see `layout_pass`); the native child-resize helper family
    /// (centered offsets for ordinary controls + right-edge button anchoring from
    /// the SIDEBTTN canvas) is layered on in 5a-ii.
    InGameOptions,
```

**Step 3: Verify** — none yet (crate will not compile until Task 2 adds the
`layout_pass` arm; that is expected and fixed next).

### Task 2: Add the `InGameOptions` baseline arm to `layout_pass`

**Why:** Makes `layout_pass` exhaustive again and defines the 5a-i baseline (raw
DLU→pixel, no re-anchor), mirroring the existing `ModalCentered` arm.

**Files:**
- Modify: `src/ui/shell/layout.rs:24-42` (the `match desc.reposition_policy`) and its
  `#[cfg(test)] mod tests`

**Pattern:** The existing `RepositionPolicy::ModalCentered` arm.

**Step 1: Add the match arm.** Inside `layout_pass`, after the `ModalCentered` arm:
```rust
                // In-game Options baseline (5a-i): raw DLU->pixel client rect, no
                // re-anchor. The native child-resize helper family (centered
                // offsets for ordinary controls + right-edge button anchoring from
                // the SIDEBTTN canvas) lands in 5a-ii, where the button asset
                // dimensions are known.
                RepositionPolicy::InGameOptions => {
                    geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h)
                }
```

**Step 2: Add a test.** Append to `mod tests` in `layout.rs`:
```rust
    /// In-game Options (5a-i baseline) is NOT re-anchored — it keeps its DLU->pixel
    /// client rect, identical to the ModalCentered baseline. (5a-ii adds the native
    /// B-helper anchoring and this expectation changes.)
    #[test]
    fn in_game_options_baseline_is_raw_dlu_to_pixel() {
        let desc = DialogDescriptor {
            id: DialogId(0x0BBB),
            bg_kind: BgKind::InGameOptions,
            slide_eligible: false,
            reposition_policy: RepositionPolicy::InGameOptions,
            controls: vec![ctrl(
                0x0529,
                ControlKind::Trackbar,
                RectPx::new(144, 100, 128, 13),
                // Anchor is ignored under InGameOptions baseline.
                AnchorRule::RightAnchor,
            )],
        };
        let laid = layout_pass(&desc, 800, 600);
        assert_eq!(rect_for(&laid, 0x0529), geom::dlu_rect(144, 100, 128, 13));
        assert_eq!(rect_for(&laid, 0x0529), RectPx::new(216, 163, 192, 21));
        // Baseline is screen-size invariant for now (oversized-screen centered
        // offsets are deferred to 5a-ii).
        assert_eq!(layout_pass(&desc, 1024, 768), laid);
    }
```
(The `tests` module already imports `BgKind`, `DialogDescriptor`, `DialogId`,
`ControlKind`, `AnchorRule`, `RepositionPolicy`, `RectPx`, and the `ctrl`/`rect_for`
helpers — no new `use` needed.)

**Step 3: Verify** — `cargo test -p vera20k ui::shell::layout` → the new test PASSES
and the existing layout tests stay green. (Full verify batched in Task 6.)

### Task 3: Format the two modified shell files

**Why:** Keep formatting localized to edited files (never crate-wide `cargo fmt`).

**Files:** `src/ui/shell/descriptor.rs`, `src/ui/shell/layout.rs`

**Step 1:** Run `rustfmt --edition 2024 src/ui/shell/descriptor.rs src/ui/shell/layout.rs`

**Step 2: Verify** — `git diff --stat` shows only the regions you edited changed (no
churn in untouched code). If rustfmt reflows unrelated lines, revert those hunks.

### Task 4: Create the `0xBBB` descriptor builder

**Why:** The core deliverable — the verified active in-game Options control set as
render-agnostic data, ready for 5a-ii paint/hit-test.

**Files:**
- Create: `src/ui/shell/in_game_options.rs`

**Pattern:** `src/ui/shell/modal.rs` (`control` consts module + `build_*_descriptor` +
`modal_control` helper).

**Step 1: Write the module + builder.**
```rust
//! In-game Options dialog (0xBBB active) descriptor.
//!
//! Render-agnostic data for the native in-game Options dialog — the screen ESC
//! opens during an active game. Depends only on the shared shell descriptor +
//! geometry types (no sim/render/assets), honoring the ui/ layering rule.
//!
//! Scope of this sub-step (5a-i): the ACTIVE `0xBBB` set of nine interactive
//! controls with their verified resource DLU rects, plus the raw DLU->pixel
//! baseline (see `layout::layout_pass`). The text statics (title/captions/labels/
//! footer + 2 currently-unidentified controls), the native child-resize anchoring,
//! the owner-draw paint, input, and INI persistence land in later 5a sub-steps. The
//! shell variant `0xF5` is a follow-on (its full control rects are not yet verified).

use super::descriptor::{
    AnchorRule, BgKind, ControlDescriptor, ControlKind, DialogDescriptor, DialogId,
    RepositionPolicy,
};
use super::geom::RectPx;

/// Resource control ids for the active-game Options dialog (`0xBBB`).
pub mod control {
    /// Back button -> own-proc result 1 (close + persist), unconditional.
    pub const BACK: u16 = 0x0686;
    /// Keyboard button -> sub-dialog (g_GameState 4); STUB until a later 5a step.
    pub const KEYBOARD: u16 = 0x052C;
    /// Sound button -> sub-dialog (g_GameState 6); STUB until a later 5a step.
    pub const SOUND: u16 = 0x052D;
    /// Game Speed trackbar (range 0..6; value inverted `6 - pos` at apply time).
    pub const GAME_SPEED: u16 = 0x0529;
    /// Scroll Rate trackbar (range 0..6; value inverted `6 - pos` at apply time).
    pub const SCROLL_RATE: u16 = 0x052A;
    /// Visual Details trackbar (range 0..2; direct value).
    pub const VISUAL_DETAILS: u16 = 0x052B;
    /// Target Lines checkbox -> Options UnitActionLines.
    pub const TARGET_LINES: u16 = 0x0601;
    /// Show Hidden checkbox -> Options ShowHidden.
    pub const SHOW_HIDDEN: u16 = 0x0604;
    /// Tooltips checkbox -> Options ToolTips.
    pub const TOOLTIPS: u16 = 0x0602;
}

/// RT_DIALOG resource id of the active-game Options dialog.
const DIALOG_0BBB: u16 = 0x0BBB;

/// Build the render-agnostic descriptor for the ACTIVE in-game Options dialog
/// (`0xBBB`): the nine interactive controls with their verified resource DLU rects.
/// Background composites as an overlay over the frozen battlefield. Reposition uses
/// the `InGameOptions` baseline (raw DLU->pixel in 5a-i; native anchoring in 5a-ii).
pub fn build_in_game_options_descriptor() -> DialogDescriptor {
    DialogDescriptor {
        id: DialogId(DIALOG_0BBB),
        controls: vec![
            options_control(control::BACK, ControlKind::Button, RectPx::new(425, 346, 108, 23)),
            options_control(control::KEYBOARD, ControlKind::Button, RectPx::new(425, 149, 108, 23)),
            options_control(control::SOUND, ControlKind::Button, RectPx::new(425, 122, 108, 23)),
            options_control(control::GAME_SPEED, ControlKind::Trackbar, RectPx::new(144, 100, 128, 13)),
            options_control(control::SCROLL_RATE, ControlKind::Trackbar, RectPx::new(144, 131, 128, 13)),
            options_control(control::VISUAL_DETAILS, ControlKind::Trackbar, RectPx::new(144, 162, 128, 13)),
            options_control(control::TARGET_LINES, ControlKind::Checkbox, RectPx::new(89, 206, 119, 10)),
            options_control(control::SHOW_HIDDEN, ControlKind::Checkbox, RectPx::new(89, 224, 119, 10)),
            options_control(control::TOOLTIPS, ControlKind::Checkbox, RectPx::new(214, 206, 127, 10)),
        ],
        bg_kind: BgKind::InGameOptions,
        slide_eligible: false,
        reposition_policy: RepositionPolicy::InGameOptions,
    }
}

/// One Options control descriptor. The `anchor` field is unused under
/// `RepositionPolicy::InGameOptions` (the native child-resize helpers key off
/// control id/kind, not a per-control anchor enum), so a benign value is stored;
/// CSF captions/labels are attached with the paint sub-step (5a-ii).
fn options_control(id: u16, kind: ControlKind, dlu_rect: RectPx) -> ControlDescriptor {
    ControlDescriptor {
        id,
        kind,
        dlu_rect,
        anchor: AnchorRule::RightAnchor,
        csf_key: None,
        tooltip_key: None,
        group: 0,
        enabled: true,
    }
}
```

**Step 2: Verify** — not yet (module not declared until Task 5; tests in Task 6).

### Task 5: Add the unit tests + declare the module

**Why:** Lock the verified control set, kinds, and baseline rects; wire the new module
into the shell tree so it compiles.

**Files:**
- Modify: `src/ui/shell/in_game_options.rs` (append `#[cfg(test)] mod tests`)
- Modify: `src/ui/shell/mod.rs` (add `pub mod in_game_options;` after `pub mod modal;`)

**Step 1: Declare the module.** In `mod.rs`, add (keeping alphabetical-ish order with
the existing `pub mod` list):
```rust
pub mod in_game_options;
```

**Step 2: Append the tests** to `in_game_options.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::geom;
    use crate::ui::shell::layout::layout_pass;

    fn control_ids(d: &DialogDescriptor) -> Vec<u16> {
        d.controls.iter().map(|c| c.id).collect()
    }
    fn kind_of(d: &DialogDescriptor, id: u16) -> ControlKind {
        d.controls.iter().find(|c| c.id == id).expect("control id").kind
    }

    #[test]
    fn descriptor_carries_the_nine_active_0bbb_controls() {
        let d = build_in_game_options_descriptor();
        assert_eq!(d.id, DialogId(0x0BBB));
        assert_eq!(d.bg_kind, BgKind::InGameOptions);
        assert_eq!(d.reposition_policy, RepositionPolicy::InGameOptions);
        assert!(!d.slide_eligible);
        let ids = control_ids(&d);
        assert_eq!(ids.len(), 9);
        for id in [
            control::BACK, control::KEYBOARD, control::SOUND,
            control::GAME_SPEED, control::SCROLL_RATE, control::VISUAL_DETAILS,
            control::TARGET_LINES, control::SHOW_HIDDEN, control::TOOLTIPS,
        ] {
            assert!(ids.contains(&id), "missing control {id:#06x}");
        }
    }

    #[test]
    fn control_kinds_match_template() {
        let d = build_in_game_options_descriptor();
        assert_eq!(kind_of(&d, control::BACK), ControlKind::Button);
        assert_eq!(kind_of(&d, control::KEYBOARD), ControlKind::Button);
        assert_eq!(kind_of(&d, control::SOUND), ControlKind::Button);
        assert_eq!(kind_of(&d, control::GAME_SPEED), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::SCROLL_RATE), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::VISUAL_DETAILS), ControlKind::Trackbar);
        assert_eq!(kind_of(&d, control::TARGET_LINES), ControlKind::Checkbox);
        assert_eq!(kind_of(&d, control::SHOW_HIDDEN), ControlKind::Checkbox);
        assert_eq!(kind_of(&d, control::TOOLTIPS), ControlKind::Checkbox);
    }

    #[test]
    fn descriptor_dlu_rects_match_verified_template() {
        // Verbatim from OPTIONS_PROC_004E1FE0 §3 (0xBBB). A 1-DLU drift here
        // shifts the control every time in-game Options opens.
        let d = build_in_game_options_descriptor();
        let dlu = |id: u16| d.controls.iter().find(|c| c.id == id).unwrap().dlu_rect;
        assert_eq!(dlu(control::BACK), RectPx::new(425, 346, 108, 23));
        assert_eq!(dlu(control::KEYBOARD), RectPx::new(425, 149, 108, 23));
        assert_eq!(dlu(control::SOUND), RectPx::new(425, 122, 108, 23));
        assert_eq!(dlu(control::GAME_SPEED), RectPx::new(144, 100, 128, 13));
        assert_eq!(dlu(control::SCROLL_RATE), RectPx::new(144, 131, 128, 13));
        assert_eq!(dlu(control::VISUAL_DETAILS), RectPx::new(144, 162, 128, 13));
        assert_eq!(dlu(control::TARGET_LINES), RectPx::new(89, 206, 119, 10));
        assert_eq!(dlu(control::SHOW_HIDDEN), RectPx::new(89, 224, 119, 10));
        assert_eq!(dlu(control::TOOLTIPS), RectPx::new(214, 206, 127, 10));
    }

    #[test]
    fn baseline_layout_is_raw_dlu_to_pixel_per_control() {
        // 5a-i baseline: every control == its raw DLU->pixel rect (no anchor).
        let d = build_in_game_options_descriptor();
        let laid = layout_pass(&d, 800, 600);
        let rect_for = |id: u16| laid.iter().find(|c| c.id == id).unwrap().rect;
        for c in &d.controls {
            let expected = geom::dlu_rect(c.dlu_rect.x, c.dlu_rect.y, c.dlu_rect.w, c.dlu_rect.h);
            assert_eq!(rect_for(c.id), expected, "control {:#06x}", c.id);
        }
        // Concrete spot-checks (round-half-up DLU factor x*6/4, y*13/8).
        assert_eq!(rect_for(control::BACK), RectPx::new(638, 562, 162, 37));
        assert_eq!(rect_for(control::GAME_SPEED), RectPx::new(216, 163, 192, 21));
        assert_eq!(rect_for(control::TOOLTIPS), RectPx::new(321, 335, 191, 16));
    }
}
```

**Step 3: Verify** — batched in Task 6.

### Task 6: Build + test verify pass (separate, bounded)

**Why:** Confirm compilation and that all suites — new and frozen — pass. Read the
literal `test result:` line; never report counts before reading it.

**Files:** none (verification only).

**Step 1: Format the new file.** `rustfmt --edition 2024 src/ui/shell/in_game_options.rs`
then `git diff --stat` to confirm no churn outside it.

**Step 2: Build.** `cargo build -p vera20k` → expect a clean build (the wrong `-p`
exits 101 without building — confirm it actually compiled).

**Step 3: Run the shell suite.** `cargo test -p vera20k ui::shell` → read the literal
`test result: ok. N passed; 0 failed` line. Expect the new `in_game_options` 4 tests +
the new `layout` test to pass alongside the existing shell tests.

**Step 4: Confirm the frozen suites are untouched & green.**
`cargo test -p vera20k ui::skirmish_shell` → read the `test result:` line; the state
suite (87) and skirmish layout suite (30) must still pass and were not edited.

**Step 5:** Report the literal pass/fail lines. If anything failed, STOP and reassess —
do not layer fixes.

### Task 7: STOP for in-game OK, then commit

**Why:** Cadence rule — the user verifies in-game before each sub-step commits to `dev`.
(5a-i adds no visible behavior yet — it is pure data/tests — so "in-game OK" here is the
user's go-ahead that the build is clean and the sub-step is accepted.)

**Step 1:** Present the verify results (literal `test result:` lines) and wait for the
user's go-ahead.

**Step 2 (after go-ahead):** Commit to `dev`:
```
git add src/ui/shell/descriptor.rs src/ui/shell/layout.rs src/ui/shell/in_game_options.rs src/ui/shell/mod.rs
git commit -m "ui: Slice 5a-i — in-game Options (0xBBB) descriptor + DLU baseline"
```
(Do NOT add anything under `docs/` — gitignored/local-only. Do NOT push.)

## Sources & References

- **Design doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md`
- **Ghidra reports:** `docs/research/OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`
  (control table §3, DLU rects, result/persist), `docs/research/OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`
  (overlay-not-panel, B-helper anchoring, SIDEBTTN type-2 buttons — drives the 5a-ii deferral).
- **gamemd.exe addresses** (kept here, not in Rust comments): `0x004E1D00`
  ShowInGameDialog, `0x004E1FE0` own proc, RT_DIALOG `0xBBB` bytes at `0x00C01B18`
  (template transcription deferred to 5a-ii).
- **Related code:** `src/ui/shell/modal.rs` (mirrored pattern), `src/ui/shell/layout.rs`
  (`ModalCentered` baseline), `src/ui/shell/geom.rs` (`dlu_rect`, `center_offset`).
- **Frozen suites:** `src/ui/skirmish_shell/state/tests.rs` (87), `src/ui/skirmish_shell/layout.rs` (30).
