---
title: Skirmish Player/AI Combos and Flags Trace
date: 2026-05-20
scenario: "Default Skirmish dialog 0x102: interact with player color/country area and first AI slot"
---

# Skirmish Player/AI Combos and Flags Trace

## Scope

Single traced mechanic: in the default offline Skirmish setup dialog `0x102`,
interact with the player color/country area and the first AI slot, then verify
combo hit testing, color ownership table behavior, side-to-flag PCX mapping, and
visible flag/swatch update.

No Rust, INI, or in-repo docs were modified. Ghidra use was read-only only.

## Evidence Used

Verified research docs:

- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`

Read-only Ghidra spot checks:

- `FUN_006ae3f0`: active Skirmish dialog procedure for dialog `0x102`; handles
  custom init `0x497`, paint, `WM_COMMAND`, and tooltip/dropdown help dispatch.
- `FUN_006acee0`: active Skirmish command handler; routes player/AI, side,
  color, start, button, and launch handling.
- `FUN_004e4c20`: active color-combo selection handler; updates global color
  ownership and refreshes all eight color combos.
- `FUN_004e3560`: active side-item-data to flag-PCX lookup.
- `OwnerDraw_ComboBox_00617250`: active combo owner-draw callback; handles
  dropdown creation, arrow-area click toggling, custom color swatch messages,
  and collapsed swatch/text drawing.

## Pipeline

1. Dialog `0x102` creates and hooks owner-draw controls.
2. Player/AI row combo controls receive custom init `0x497`.
3. Side/country combos select item data `-3`, `-2`, or `0..9`.
4. Side selection updates flag statics `0x6DA..0x6E1`.
5. Color combos populate from a nine-row color table with owner slots.
6. Color selection updates ownership and refreshes all color combos.
7. Combo/static owner-draw callbacks paint collapsed combos, swatches, and flags.

## Stage Verdicts

### Stage 1 - Active YR dialog path

gamemd: `FUN_006ae3f0` delegates shell base handling, initializes Skirmish on
`0x497`, paints the preview/starts, and routes `WM_COMMAND` to `FUN_006acee0`.
The prior docs explicitly mark dialog `0x102` owner-draw callbacks active in YR.

Rust: the dev Skirmish shell is gated by `dev_skirmish_shell_enabled` and uses
`src/app.rs` to route mouse input to `src/ui/skirmish_shell/state.rs`.

Verdict: UNCHECKED. Active binary evidence exists, but this trace did not compute
a full gamemd-vs-Rust modal lifecycle equality.

### Stage 2 - Player/AI and country combo control surface

gamemd: dialog `0x102` exposes player/AI controls `0x50B`, `0x50E`, `0x516`,
`0x51A..0x51D`; country controls `0x6A1`, `0x510`, `0x513`, `0x51E`, `0x514`,
`0x51F`, `0x520`, `0x521`; color controls `0x6A2`, `0x522..0x528`.

Rust: `src/ui/skirmish_shell/layout.rs` defines color combo rects and flag rects
only. `src/ui/skirmish_shell/state.rs` has `SelectColor` but no player/AI combo
action and no country/side combo action.

Verdict: NOT-IMPLEMENTED. The visible country/player-AI combo interaction for the
player row and first AI row is absent.

### Stage 3 - Color combo hit testing

Concrete point: default `800x600`, player color rect from Rust layout is
`(423,59,44,119)`. A click at `(423,59)` is accepted by Rust because
`RectPx::contains` uses inclusive top-left and exclusive bottom-right.

gamemd: the combo owner-draw callback toggles the dropdown on mouse down only
when the click is in the right/arrow area (`x > client_width - 0x14`); actual
selection changes later through the dropdown/`WM_COMMAND` notification path.

Rust: `hit_test` maps any point inside the color rect directly to `SelectColor`,
and `apply_action` immediately cycles the selected color.

Verdict: FAIL. At `(423,59)`, Rust changes player color; gamemd would not take the
arrow/dropdown path for that left-edge point.

### Stage 4 - Color table and ownership

gamemd: `FUN_004e43c0` initializes nine color rows, string IDs `0x1DB..0x1E3`,
swatch values `0x000DE2DD`, `0x001919FF`, `0x00E2742A`, `0x002ED13E`,
`0x0019A0FF`, `0x00E6D732`, `0x00BD2895`, `0x00EB9AFF`, `0x00606060`, and owner
slot `-1`. Normal population includes colors owned by this slot or unowned,
plus a first `-2` row.

Rust: `SkirmishShellState` stores `player_color_index: usize` and one
`opponent.color_index`. There is no nine-row color table, no owner column, no
reserved `-2` item-data row, and no all-combo refresh.

Verdict: NOT-IMPLEMENTED. The player-visible exclusion of already claimed colors
from other rows is missing.

### Stage 5 - Color selection action

gamemd: `FUN_004e4c20` maps control IDs `0x6A2`, `0x522..0x528` to slots `0..7`,
clears the old owner row for that slot, reads current selection and item data,
writes the slot to `color_table[item_data].owner` unless item data is `-2`, then
refreshes every color combo.

Rust: `apply_action` increments player color with `(color + 1) % 8`; first AI
increments its own `color_index` with the same modulo. Other slots are not
refreshed and color `8` is unreachable.

Verdict: FAIL. The concrete transition and option space differ: Rust cycles eight
local indices; gamemd selects item data from a nine-color ownership table plus a
special `-2` row.

### Stage 6 - Visible color swatch update

gamemd: collapsed color combos draw a swatch when custom `0x4DD` has enabled
swatches, current selection is `0..49`, and per-item swatch data was written by
`0x498`. The raw color is converted through the active DirectDraw format before
filling the swatch rectangle.

Rust: the Skirmish shell render path draws button labels, preview labels, and
flags. No color combo frame, dropdown, selected text, or swatch rectangle is
rendered, and `color_index` is not consumed by the renderer.

Verdict: NOT-IMPLEMENTED. Clicking color changes hidden state only; the player
does not see the gamemd-style swatch update.

### Stage 7 - Side-to-flag PCX mapping

gamemd: `FUN_004e3560` maps item data `-3 -> obsi.pcx`, `-2 -> rani.pcx`,
`0 -> usai.pcx`, `1 -> japi.pcx`, `2 -> frai.pcx`, `3 -> geri.pcx`,
`4 -> gbri.pcx`, `5 -> djbi.pcx`, `6 -> arbi.pcx`, `7 -> lati.pcx`,
`8 -> rusi.pcx`, `9 -> yrii.pcx`.

Rust: `flag_pcx_for_side_item_data` returns the same mapping.

Verdict: PASS. The literal item-data to PCX filename mapping matches for all
verified entries.

### Stage 8 - Default player/first-AI flag asset choice

gamemd: if the player selected side item data is `0`, the visible flag is
`usai.pcx`; if first AI side item data is `8`, the visible flag is `rusi.pcx`.

Rust: default state is player `America` and first AI `Russia`, and the renderer
maps those countries to `usai.pcx` and `rusi.pcx`.

Verdict: UNCHECKED. The mapping equality is computed, but this trace did not
prove gamemd's default selected side item data for a fresh retail profile.

### Stage 9 - Flag placement and sizing

Concrete default layout: Rust flag rect for player row at `800x600` is
`(225,59,48,20)`. Verified flag PCXs are `47x23`.

gamemd: static kind `2` centers only when the source is smaller than the static
rect and clips when larger; it does not scale the PCX. For a `47x23` flag in a
`48x20` rect, height is clipped to the static area.

Rust: `push_entry_fit` scales the PCX by `min(48/47,20/23)`, producing a rounded
draw size of `41x20` and x offset `+4`.

Verdict: FAIL. Rust shrinks and recenters the flag; gamemd blits at native size
with vertical clipping.

### Stage 10 - Flag update after country interaction

gamemd: side/country selection flows through side combo item data, `FUN_004e3560`,
and `FUN_00603d30`, which stores the PCX pointer in the target static and
invalidates the flag.

Rust: there is no country/side hit test or action in this shell, so the player
cannot change the player row or first AI country from the shell. Existing flags
only reflect initial `SkirmishShellState`.

Verdict: NOT-IMPLEMENTED. The visible flag update after country interaction is
missing even though the static country-to-PCX mapping exists.

## Failures and Missing Pieces

1. Country/player-AI combo interaction is absent. Players cannot change the
   player country or first AI country in the Skirmish shell.
2. Color combo hit testing is not gamemd-like. Rust changes color from any point
   in the stored color rect; gamemd uses owner-drawn combo/dropdown semantics.
3. The color ownership table is absent. Claimed colors are not removed from other
   rows, color ID `8` is unreachable via Rust's `% 8`, and the special `-2` row
   is not represented.
4. Color swatches are not drawn. Clicking color changes hidden state but gives no
   gamemd-style visible swatch feedback.
5. Flag rendering scales flags down instead of native blit plus clip, making
   player and AI flags visibly narrower than retail.

## Adjacent Findings

- Start/team combo parity remains outside this trace. The controls are adjacent
  in dialog `0x102`, but this run was limited to player/AI combos, country/color,
  flags, and swatches.
- The current shell is still dev-gated, so normal main-menu accessibility is a
  separate integration question, not counted in this mechanic tally.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 4

## Status

COMPLETE
