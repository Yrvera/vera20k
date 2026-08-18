# Slice 5a — In-Game Options Dialog (native 0xBBB/0xF5) — Design Spec

**Status:** design (brainstorm output) — awaiting approval, no code yet.
**Feeds:** `/write-plan` (sub-stepped) → execute on `dev`.
**Package:** `vera20k`. **Parity bar:** indistinguishable from gamemd.exe on observable output.

## 0. Decisions locked (this brainstorm)

1. **Full native replacement** of the egui ESC pause card with the native gamemd
   in-game Options dialog chrome. (User choice.)
2. **Build `0xBBB`/`0xF5` now; sequence the rest.** Keyboard/Sound buttons render but
   open a "not-yet-implemented" stub; the native Abort-Mission/quit dialog and the
   Keyboard (state 4) / Sound (state 6) sub-dialogs are deferred to follow-on sub-steps.
   (User choice.)

### Scope correction that drove the decisions

The verified binary (`OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`,
High confidence) shows **ESC *is* the Options dialog**: `State_Machine` case 5 →
`ShowInGameDialog` → renders RT_DIALOG `0xBBB` (active) / `0xF5` (shell) directly. There
is **no separate parent menu** with Save/Load/Delete/Restart/Surrender buttons — the
dialog's only buttons are **Keyboard (0x52C), Sound (0x52D), Back (0x686)**. The earlier
"dead Save/Load buttons" fear was a misremembered combined menu and does not apply.

Consequences that "full replacement" still exposes (all deferred per decision 2):
- **Quit-to-menu has no button in `0xBBB`.** gamemd's in-game Abort-Mission/quit is a
  SEPARATE, un-researched dialog (the quit-confirm doc explicitly says it is not the
  main-menu `0x120` path). Retiring the egui card drops today's "Return to Menu" until
  that path lands — see §7 interim gap.
- **Keyboard/Sound open sub-dialogs** (`g_GameState` 4 and 6), not yet decoded. Today's
  egui music-volume lives in gamemd's Sound sub-dialog — also deferred.
- **`0xBBB` has 17 template controls; only 15 are identified** (3 buttons + 3 trackbars +
  3 checkboxes + 6 statics). The last 2 must be re-parsed in 5a-i to confirm neither is a
  hidden button.

## 1. The gamemd mechanism (verified — source of truth)

All facts from `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` unless noted.

### Template selection
`ShowInGameDialog` reads byte `g_GameActive`; `== 1` (active game) → `0xBBB`, else `0xF5`
(shell). For the in-game ESC case we render **`0xBBB`**. `0xF5` differs (wider 148-DLU
sliders, +Difficulty `0x50F`, +ScrollCoasting `0x51A`, +static `0x71C`, no `0x52C/0x52D`)
and is NOT derived from `0xBBB` — built separately. (5a renders `0xBBB`; `0xF5` is the
shell-Options variant, lower priority for the in-game path but in the same builder.)

### `0xBBB` controls (DLU rects, behavior)

| ID | Kind | DLU rect | Behavior | 5a treatment |
|---|---|---|---|---|
| `0x686` Back | Button | (425,346,108,23) | WM_COMMAND → result **1** (close + persist), unconditional | **close + persist** |
| `0x52C` Keyboard | Button | (425,149,108,23) | → `g_GameState=4`, result 1 | **STUB** (not-yet-impl) |
| `0x52D` Sound | Button | (425,122,108,23) | → `g_GameState=6`, result 1; init `EnableWindow` via `0x00407000` | **STUB** (not-yet-impl) |
| `0x529` GameSpeed | Trackbar | (144,100,128,13) | range **0..6**, **inverted** (internal = `6 - pos`); label `0x671` | full |
| `0x52A` ScrollRate | Trackbar | (144,131,128,13) | range **0..6**, **inverted** (`6 - pos`); label `0x672` | full |
| `0x52B` VisualDetails | Trackbar | (144,162,128,13) | range **0..2**, **direct** (`pos`); label `0x673` | full |
| `0x601` TargetLines | Checkbox | (89,206,119,10) | `BM_GETCHECK==1` → Options `+0x1E` (`UnitActionLines`); apply calls `0x0070D180` | full |
| `0x604` ShowHidden | Checkbox | (89,224,119,10) | `BM_GETCHECK==1` → Options `+0x1F` (`ShowHidden`) | full |
| `0x602` Tooltips | Checkbox | (214,206,127,10) | `BM_GETCHECK==1` → Options `+0x20` (`ToolTips`); updates tooltip mgr if active | full |
| statics | Static | — | `0x714` GameSpeed caption, `0x671/0x672/0x673` dynamic labels, `0x694` title, `0x695` footer | render |

**Slider inversion is the #1 parity trap:** GameSpeed/ScrollRate are `6 - pos`;
VisualDetails is direct. Do NOT treat all sliders alike (doc §12 Negative Facts).

### Live label update (WM_HSCROLL, low word 5)
Slider drag updates its label static from a CSF table (sender HWND → label control):
- GameSpeed `0x671`, ScrollRate `0x672`: `TXT_SLOWEST, SLOWER, SLOW, MEDIUM, FAST, FASTER, FASTEST` (index 0..6).
- VisualDetails `0x673`: `TXT_LOW, TXT_MEDIUM, TXT_HIGH` (index 0..2).

### Result + persist
Caller inits result slot `-1`, pumps until it changes. **result == 1** (every close
button) → `ApplyFromInGameDialog` then `WriteToINI` (`RA2MD.INI`). **result == 2** is
caller-generated only when the game ENDS while open → no persist. **No
cancel-without-save path exists** (doc §12). `modal.rs` already encodes this:
`ModalResult::InGameOptions(1).options_persists() == true`, `(2) == false`.

### Apply / persist fields (`[Options]` in `RA2MD.INI`)
GameSpeed `+0x00` (`6 - pos`; offline modes 0/5 store directly — the network event-queue
branch is **dead code** in current play), ScrollRate `+0x10` (`6 - pos`), DetailLevel
`+0x18` (`pos`; changed → call render-detail apply `0x004AE450`), UnitActionLines `+0x1E`,
ShowHidden `+0x1F`, ToolTips `+0x20`. Defaults: GameSpeed 3, ScrollRate 3, DetailLevel 2,
UnitActionLines 1, ShowHidden 0, ToolTips 1. gamemd's `WriteToINI` rewrites the WHOLE
Options object (also `[Video]/[Audio]/[Network]` keys) — see §7 deferred.

### DLU → pixel
`geom::dlu_rect` already implements the 8pt MS-Sans factor (`x*6/4`, `y*13/8`,
round-half-up). Dialog is **533×369 DLU ≈ 800×600 px** — effectively full-screen at the
RA2 base resolution. Projection at 800×600 vs 1024×768 is a bounded 5a-i item (§9).

## 2. Existing architecture (what to reuse — mapped this session)

| Surface | Status | Reuse for 5a |
|---|---|---|
| `ui/shell/descriptor.rs` `ControlKind` | has `Button/Static/Checkbox/Trackbar/...` | descriptor controls directly |
| `ui/shell/descriptor.rs` `DialogDescriptor` | full dialog + bg/slide/reposition | the Options descriptor |
| `ui/shell/modal.rs` `ModalKind::InGameOptions` | `template_id(true)=0xBBB`, `result_convention=OwnProc`, `options_persists()` | result/persist convention done |
| `ui/shell/geom.rs` `dlu_rect` | DLU→px factor | layout projection |
| `ui/shell/layout.rs` `layout_pass` | DLU→px + per-control re-anchor | extend for the Options reposition policy |
| `app_skirmish_shell_render/controls.rs` `paint_control` | `Checkbox`/`Trackbar`/`Combo`/`ScrollBar` arms | **Checkbox + Trackbar reused as-is**; add **Button** arm |
| `render/skirmish_shell_chrome.rs` `ControlChrome` | cue_i/cce_i, trofl/trofm/trofr/trakgrip/rail, SDBTNANM, MNBTTN | checkbox/trackbar glyphs ready; **SIDEBTTN missing** |
| `util/ini_writer.rs` `set_ini_value` | single-key INI write (preserves other keys) | `[Options]` persist |
| `app_input.rs:440` ESC → `state.paused=true` | trigger | keep |
| `app.rs:3003-3055` `draw_pause_menu` + `PauseMenuAction` | egui card | **replace** with native render+input |
| `app.rs` `state.paused` freeze (`advance_in_game_runtime` gate) | sim freeze | keep (5a); `service_tick` swap is 5b |

### Gaps this slice must fill
- **Button paint arm** — `paint_control` has none. Add `ControlPaint::Button` (3-frame
  owner-draw: released/hover/pressed).
- **SIDEBTTN.SHP + SIDEBAR.PAL** — active-`0xBBB` buttons (Keyboard/Sound/Back) are
  owner-draw "type 2" = SIDEBTTN via SIDEBAR.PAL frames 0/1/2 (per
  `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS`). `0xF5` Back is "type 1" = SDBTNANM
  (already loaded). Load SIDEBTTN into `ControlChrome`.
- **Options dialog background** — full-screen panel art. Identify the SHP/PCX + palette
  from `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS` (5a-i). New `BgKind` variant.
- **Client-options state** — no in-game options state exists in Rust (doc §7). 5a
  introduces an app/ui-level options state (GameSpeed/ScrollRate/DetailLevel/
  UnitActionLines/ShowHidden/ToolTips). These are render/UI/camera/tick options →
  **NOT** `sim/` (honors the layering rule). Distinct from `sim/game_options.rs`
  (skirmish gameplay options).

## 3. Design

### D1 — Module home
New file **`src/ui/shell/in_game_options.rs`**: builds the `DialogDescriptor` for
`0xBBB`/`0xF5` (controls + DLU rects from §1) and the full-dialog layout (DLU→px +
centered/scaled projection). Keeps `modal.rs` focused on the count-rule message-box
family. Pure data + geometry, no render/sim deps (layering-clean, unit-testable).

### D2 — Composition / background
Add `BgKind::InGameOptions` (full-screen Options panel art, identified in 5a-i) and a
reposition policy for a full-screen-centered dialog (NOT `IncludeSetReanchor`, NOT
`ModalShp`). At 800×600 the dialog fills the screen; at 1024×768 it centers (confirm
gamemd's behavior in 5a-i — center vs stretch).

### D3 — Paint reuse + Button arm
Render through the existing `paint_control` seam:
- **Checkbox / Trackbar**: reuse the existing arms unchanged (same trofl/trofm/trofr +
  trakgrip + rail, same cue_i/cce_i). The Options trackbar is the same horizontal control
  as the skirmish GameSpeed/Credits/UnitCount trackbars.
- **Button (new)**: `ControlPaint::Button { rect, frame_state }` resolving SIDEBTTN
  (active) / SDBTNANM (shell) frames + the CSF caption text.
- **Statics/labels**: title/caption/footer + the dynamic slider labels via the existing
  BitFont static text path.

**Where the Options render lives:** add `app_skirmish_shell_render/in_game_options.rs`
(sibling emitter) reusing `paint_control` from the same module. The `app_skirmish_shell_render`
module is the established home for the owner-draw seam; painting a second owner-draw
dialog there avoids duplicating glyph resolution and keeps `render/` free of app-layer
deps. (If the module name grates, a rename to `app_shell_render` is a separate
non-behavioral cleanup — not in 5a.)

### D4 — Trigger / freeze swap
- Keep ESC → `state.paused = true` (`app_input.rs:440`) and the existing paused freeze
  (the `service_tick`/session-mode swap is 5b, already specced).
- Replace the `draw_pause_menu` egui call (`app.rs:3003-3055`) with: build descriptor +
  layout, emit sprite instances (native chrome), route input — Back → `paused=false` +
  persist; slider drag → update value + live label; checkbox click → toggle;
  Keyboard/Sound → stub (no state change, optional "not implemented" feedback).
- Affordance mapping: egui Resume → **Back**; egui SetGameSpeed → **GameSpeed slider**
  (see §5 GameSpeed parity note); egui music-volume → **deferred** (Sound sub-dialog);
  egui Return-to-Menu → **deferred** (Abort dialog) — see §7 interim gap.

### D5 — Persistence
On Back (result == 1): apply slider/checkbox values to the client-options state, then
write the touched `[Options]` keys to `RA2MD.INI` via `set_ini_value` (preserves
untouched keys). GameSpeed/ScrollRate internal = `6 - pos`; DetailLevel = `pos`;
checkboxes = `BM_GETCHECK==1`. result == 2 path (game-ended) writes nothing —
unit-testable via the existing `options_persists()` convention.

## 4. Sub-stepping (one per cycle; re-anchor by content)

- **5a-i — descriptor + layout (pure).** `in_game_options.rs`: `DialogDescriptor` for
  `0xBBB` (+`0xF5`), full-dialog DLU→px layout at 800×600 & 1024×768. Re-parse the 2
  unidentified `0xBBB` controls and confirm the background asset. Unit tests on control
  set + rect geometry. No paint, no behavior.
- **5a-ii — owner-draw paint + assets.** Add `ControlPaint::Button` arm; load
  SIDEBTTN.SHP/SIDEBAR.PAL + the background into `ControlChrome`; emit the dialog
  pixel-faithfully behind the existing paused freeze; replace the egui card draw.
  Keyboard/Sound render as stub buttons. Draw-list tests (byte-identical emission order).
- **5a-iii — behavior + INI persist.** Slider drag + live CSF labels, checkbox toggles,
  Back → apply + write `[Options]` to `RA2MD.INI`; wire each control's downstream effect
  where it exists (UnitActionLines render gate, ToolTips manager, GameSpeed tick rate),
  persist-only where it does not (ShowHidden, DetailLevel, ScrollRate — note each).

## 5. Tiny-detail ledger (must all be honored)

1. GameSpeed/ScrollRate sliders inverted (`6 - pos`); VisualDetails direct (`pos`). ✔ §1
2. GameSpeed/ScrollRate range 0..6; VisualDetails range 0..2. ✔ §1
3. Checkbox → Options bytes `+0x1E/+0x1F/+0x20`, value `BM_GETCHECK==1` only. ✔ §1
4. result == 1 persists (every close button); result == 2 (game-ended) does not; no
   discard path. ✔ `modal.rs`
5. DLU factor `x*6/4, y*13/8` round-half-up. ✔ `geom::dlu_rect`
6. CSF label tables: GameSpeed/ScrollRate SLOWEST..FASTEST, VisualDetails LOW/MED/HIGH. §1
7. Active-`0xBBB` buttons use SIDEBTTN+SIDEBAR.PAL frames 0/1/2; `0xF5` Back uses
   SDBTNANM. (Asset gap — 5a-ii.)
8. **GameSpeed parity note (open):** gamemd GameSpeed is 0..6 mapped to specific frame
   cadence; the current egui pause card uses arbitrary tps presets (15/30/.../500) — a
   VERA-ism. The native slider must map 0..6 to gamemd's actual tick cadence, not the tps
   presets. Confirm the 0..6 → frame-delay mapping (may need a small research check) in
   5a-iii; do NOT silently keep the tps-preset model behind the native slider.
9. The 2 unidentified `0xBBB` controls (17 total − 15 known) — re-parse in 5a-i.
10. `0xF5` differs from `0xBBB` (wider sliders, +Difficulty/+ScrollCoasting/+static, no
    Keyboard/Sound) — build separately, do not derive (doc §12).

## 6. Layering / determinism checks
- Client-options state is app/ui-level, **never `sim/`** (render/UI/camera/tick options).
- `in_game_options.rs` (descriptor/layout) depends only on `ui/shell` geom — no
  render/sim/assets.
- No new float in sim. DLU/px math is UI-side integer (matches existing shell geom).

## 7. Deferred / follow-on (explicitly out of 5a)
- **Native Abort-Mission / quit-to-menu dialog** (separate un-researched dialog). Needs a
  Ghidra research pass, then build. **Interim gap:** retiring the egui card removes
  quit-to-menu during 5a — see §8 Q1.
- **Keyboard sub-dialog** (`g_GameState=4`) and **Sound sub-dialog** (`g_GameState=6`,
  owns in-game music/voice volume) — research + build later; Keyboard/Sound buttons are
  stubs until then. Interim: no in-game music-volume UI (audio still plays).
- **Whole-object `WriteToINI`** (`[Video]/[Audio]/[Network]` keys). 5a writes only the
  dialog-touched `[Options]` keys via `set_ini_value`; observably equivalent for the
  dialog's effect (untouched keys keep their existing values), modulo file
  formatting/ordering. Promote to full-object write when the Options object is modelled.
- **`service_tick` + session-mode pump (5b)** — already specced; swaps the paused-only
  freeze for the mode-gated sim pump + C2 assertion.

## 8. Open questions for the user (pre-write-plan)

- **Q1 (interim quit-to-menu): RESOLVED → (a) keep a temp escape.** During 5a the native
  dialog has no quit-to-menu button and the egui card is retired, so 5a retains a
  temporary non-chrome quit-to-menu (the existing ReturnToMenu keybind / a dev shortcut)
  until the native Abort dialog lands. It is trivially removed when the Abort dialog
  sub-step ships. (User decision 2026-06-12.)
- **Q2 (`0xF5` in 5a?):** Build only `0xBBB` (active in-game) in 5a, or include `0xF5`
  (shell Options, reached from the main menu) in the same descriptor builder now?
  **Recommend `0xBBB`-only behavior**, but author the descriptor builder to take
  `in_active_game` so `0xF5` is a cheap follow-on (matches `modal.rs::template_id`).

## 9. Bounded items to resolve inside 5a-i (from slice5b kickoff §9)
- DLU→pixel full-dialog projection at 800×600 and 1024×768 (center vs stretch at 1024).
- Confirm the `0xBBB`/`0xF5` background + control SHP/PAL (assumed `0x102`-family +
  SIDEBTTN; not yet confirmed identical) — read `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS`.

## 10. Sources
- `docs/research/OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` (controls,
  inversion, result/persist, INI fields — High confidence).
- `docs/research/OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md` (button
  owner-draw type/asset routing).
- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md`, `...-slice5b-kickoff.md`.
- Code: `src/ui/shell/{modal,descriptor,geom,layout}.rs`,
  `src/app_skirmish_shell_render/controls.rs`, `src/render/skirmish_shell_chrome.rs`,
  `src/ui/pause_menu.rs`, `src/app.rs:3003-3055`, `src/app_input.rs:440`.
