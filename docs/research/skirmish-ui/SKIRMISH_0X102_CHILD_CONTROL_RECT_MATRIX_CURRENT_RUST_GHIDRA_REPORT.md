# Skirmish 0x102 Child-Control Rect Matrix vs Current Rust - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x0060C4A0`, `0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust ownership of visible offline Skirmish dialog `0x102` child-control rectangles needed for a cohesive shell, compared against the verified binary child rect matrix.  
**Non-Scope:** visual paint internals, dropdown popup internals, Choose Map dialog `0x6B`, preview pixel decode, start marker projection, and Rust implementation changes.  
**Confidence:** High for binary rect policy and current Rust layout ownership scan; Medium for semantic draw-order conclusions because `SkirmishShellDrawRole` is a sprite-role subset, not a complete UI-order model.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`; AI/opponent rows are Active in YR: Conditional when map start count hides/disables rows.

## 0. Working Notes

- Target question: Does current Rust give first-class rect ownership to every player-visible `0x102` component needed for a cohesive Skirmish shell: name edit, flags, side/color/start/team/AI combos, checkboxes, trackbars, preview, right-panel text, status/help strip, Start/Choose/Back, and visible static labels?
- Non-goals: Do not redo paint internals, dropdown internals, preview projection, Choose Map `0x6B`, or any Rust patch.
- Evidence needed to mark COMPLETE: active `0x102` creation and resize path verified; binary helper rules for status/fallback/fixups spot-checked; existing complete child matrix consumed; current Rust layout/render/state surfaces scanned; every requested component classified as first-class, derived, missing, or subset-only.
- Stop conditions: stop if Ghidra read-only evidence is unavailable for the active path or `0x695`/fixup policy, if the existing complete matrix conflicts with rechecked Ghidra evidence, or after all requested ownership classes are classified.

## 1. Overview

The active binary creates dialog `0x102`, resizes the parent to the full screen, then enumerates child HWNDs through `ResizeShellChildControl_0060C0C0`. The complete prior matrix remains the binary baseline: the setup screen has 72 visible resource children, with selective right-panel movement, owner-draw button snapping, status `0x695` bottom-left anchoring, and small one-pixel fixups.

Current Rust now owns most of that cohesive shell layout directly in `src/ui/skirmish_shell/layout.rs`. The remaining rect-ownership gaps are status/help static `0x695` and the three trackbar option-label statics `0x699`, `0x69B`, `0x69C`. The `ShellControlId` enum and `SkirmishShellDrawRole` enum are subsets and should not be treated as complete child-control inventories.

## 2. Binary Evidence Rechecked

| Binary behavior | Status | Active in YR | Evidence |
|---|---|---|---|
| Offline launcher creates dialog id `0x102` with proc `0x006AE3F0` | verified | Yes | decompile `FUN_006ae2c0`; assembly `0x006AE31C..0x006AE328` moves proc/id then calls `0x00622650` |
| Parent is resized to full screen, then children enumerate through `ResizeShellChildControl_0060C0C0` | verified | Yes | decompile `FUN_0060c4a0` |
| Dispatcher routes Start/Choose owner-draw buttons through `FUN_0060B000` before generic right-anchor | verified | Yes | decompile `0x0060C0C0`; assembly `0x0060C1B0..0x0060C1C8` tests record `+0x68 == 0`, calls `0x00608CD0`, then `0x0060B000` |
| Generic right-anchor allowlist includes `0x694`, `0x468`, `0x6EC`, `0x5A8` for `0x102` | verified | Yes | decompile `FUN_00608cd0`; decompile `FUN_0060b1d0` |
| Back `0x5C0` uses bottom/right owner-draw button helper | verified | Yes | decompile `FUN_00609730`; assembly `0x0060C213..0x0060C227` calls `0x0060B350` |
| Status/help static `0x695` is a real `0x102` child routed to bottom-left helper | verified | Yes | decompile `FUN_00601360`, `FUN_0060b550`; assembly `0x0060C2B6..0x0060C2D0` |
| Ordinary controls preserve DLU-derived rects except documented `0x102` one-pixel fixups | verified | Yes | decompile `FUN_0060b950`; assembly `0x0060BE0A..0x0060BE20`, `0x0060C065..0x0060C092` |

No INI keys participate in this child rect policy. Rules/map INI affects combo content and option defaults, not the child-window rectangles.

## 3. Current Rust Ownership Matrix

| Component family | Binary child ids | Current Rust ownership | Evidence | Delta |
|---|---|---|---|---|
| Start / Choose / Back | `0x617`, `0x5AA`, `0x5C0` | first-class rects: `start_button`, `choose_map_button`, `back_button`; hit-tested first | `layout.rs:151..153`, `layout.rs:494..496`, `state.rs:1452..1488` | none observed for rect ownership |
| Right-panel title/game/map text | `0x694`, `0x6EC`, `0x5A8` | first-class grouped rects: `right_panel_text` | `layout.rs:118..122`, `layout.rs:150`, `layout.rs:417..420`, `app_skirmish_shell_render.rs:1726..1759` | none observed for rect ownership |
| Preview anchor | `0x468` | first-class `map_preview` rect | `layout.rs:154`, `layout.rs:497`, `app_skirmish_shell_render.rs:2179` | none observed for rect ownership |
| Player-name edit | `0x6A0` | first-class `player_name` rect plus client/text inset helpers | `layout.rs:28..29`, `layout.rs:156`, `layout.rs:234..248`, `layout.rs:422..424` | rect owned; render/input semantics still partial because render uses literal `"Player"` and no edit hit/focus route was found |
| Column labels | `0x796`, `0x791`, `0x792`, `0x793`, `0x794` | first-class grouped rects: `column_labels` | `layout.rs:130..136`, `layout.rs:155`, `layout.rs:498..503`, `app_skirmish_shell_render.rs:1709..1724` | none observed |
| AI/player row combos | AI `0x50B..0x51D`, side `0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521`, start `0x6A3..0x6AB`, team `0x76D..0x774` | first-class arrays in `rows`; `combo_rect` maps semantic combo ids to rects | `layout.rs:139..143`, `layout.rs:448..488`, `state.rs:648..655` | none observed for rect ownership |
| Color combos | `0x6A2`, `0x522..0x528` | first-class `color_combos: [RectPx; 8]` | `layout.rs:158`, `layout.rs:428..437`, `state.rs:652` | none observed |
| Flags | `0x6DA..0x6E1` | first-class `flags: [RectPx; 8]`; render consumes by row | `layout.rs:159`, `layout.rs:438..447`, `app_skirmish_shell_render.rs:1429..1451` | none observed |
| Checkboxes | `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D` | first-class `checkboxes` with ids and final rects; label rect is derived from owner-draw checkbox rect | `layout.rs:79..94`, `layout.rs:160..161`, `layout.rs:514..535`, `state.rs:430..436`, `app_skirmish_shell_render.rs:1770..1779` | none observed for rect ownership |
| Trackbars | `0x529`, `0x511`, `0x50C` | first-class grouped rects: `trackbars`; hit/input helper maps ids to rects | `layout.rs:102..113`, `layout.rs:160`, `layout.rs:509..512`, `state.rs:333..337`, `state.rs:439..458` | none observed for rect ownership |
| Trackbar option-label statics | `0x699`, `0x69B`, `0x69C` | missing as first-class rect fields and not rendered in scanned text builder | no `0x699/0x69B/0x69C` hits in `src/ui/skirmish_shell` or `src/app_skirmish_shell_render.rs`; binary matrix rows 12..14 | missing |
| Status/help strip | `0x695` | missing as first-class rect field and not rendered/hit-tested | no `0x695`, `status`, or `help` layout field in scanned Rust; binary evidence `0x0060C2B6..0x0060C2D0` | missing |
| Complete child id inventory | 72 visible children | `ShellControlId` is a subset only | `layout.rs:52..74` | do not use as complete matrix |
| Semantic draw role inventory | selected sprite roles | `SkirmishShellDrawRole` is sprite/chrome-oriented, not a complete child-control draw/input order | `app_skirmish_shell_render.rs:101..115`; text/control rendering is built separately in `build_shell_text_draws` | partial by design; dangerous if treated as complete |

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Active offline `0x102` launcher | verified | `FUN_006ae2c0`; `0x006AE31C..0x006AE328` | none |
| Full-screen parent resize and child enumeration | verified | `FUN_0060c4a0` | none |
| Dispatcher branch order | verified | `ResizeShellChildControl_0060C0C0` | none |
| Start/Choose/Back helper ownership | verified | `0x0060C1B0..0x0060C1C8`, `0x0060C213..0x0060C227` | none |
| Right-panel statics/preview helper ownership | verified | `FUN_00608cd0`, `FUN_0060b1d0` | none |
| Status `0x695` helper ownership | verified | `FUN_00601360`, `FUN_0060b550`, `0x0060C2B6..0x0060C2D0` | Rust implementation pending |
| One-pixel fixups | verified | `FUN_0060b950`, `0x0060BE0A..0x0060C092` | none for current represented controls |
| Current Rust layout struct fields | verified | `src/ui/skirmish_shell/layout.rs:147..161` | add missing status/option-label fields |
| Current Rust render/text consumers | verified | `src/app_skirmish_shell_render.rs:1400..1455`, `:1678..1797` | status/option-label text absent; player edit semantics partial |
| Current Rust input consumers | verified | `src/ui/skirmish_shell/state.rs:420..458`, `:648..655`, `:1452..1488` | player-name edit focus/input and status/help hover text absent |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is dialog 0x102 active in standard offline YR? -> Yes, `FUN_006ae2c0` passes id `0x102` and proc `0x006AE3F0` to shell dialog creation.` (evidence: `0x006AE31C..0x006AE328`)
- `[RESOLVED] OQ-02 - Is current Rust using global scaling for child rects? -> No; current `layout.rs` computes DLU/fixup/helper rects directly, matching the settled model.` (evidence: `src/ui/skirmish_shell/layout.rs:36..43`, `:417..535`)
- `[RESOLVED] OQ-03 - Are Start/Choose/Back first-class and on the right helper model? -> Yes; Start/Choose use `owner_draw_button_snap_rect`, Back uses `back_rect`.` (evidence: `layout.rs:494..496`; binary `0x0060C1B0..0x0060C227`)
- `[RESOLVED] OQ-04 - Are right-panel title/game/map statics first-class? -> Yes; current `right_panel_text` owns all three rects.` (evidence: `layout.rs:118..122`, `:417..420`)
- `[RESOLVED] OQ-05 - Is preview `0x468` first-class? -> Yes; `map_preview` is a layout field and render anchor.` (evidence: `layout.rs:154`, `:497`; binary `FUN_00608cd0`, `FUN_0060b1d0`)
- `[RESOLVED] OQ-06 - Is player-name edit `0x6A0` first-class? -> Rect yes, including edit client/text inset helpers; input/render semantics are still partial.` (evidence: `layout.rs:156`, `:234..248`, `app_skirmish_shell_render.rs:1762..1768`)
- `[RESOLVED] OQ-07 - Are row combos/color combos/flags represented? -> Yes; arrays cover AI, side, start, team, color, and flags.` (evidence: `layout.rs:139..143`, `:428..488`; `state.rs:648..655`)
- `[RESOLVED] OQ-08 - Are checkboxes and trackbars represented? -> Yes; rect groups include binary one-pixel fixups and are consumed by render/input.` (evidence: `layout.rs:509..535`, `state.rs:420..458`, `app_skirmish_shell_render.rs:1770..1797`)
- `[RESOLVED] OQ-09 - Is status/help strip `0x695` represented? -> No; binary owns it as a visible bottom-left child, but scanned Rust has no first-class rect or render/hit surface.` (evidence: `0x0060C2B6..0x0060C2D0`; `rg 0x695/status/help`)
- `[RESOLVED] OQ-10 - Are all visible static labels represented? -> Column labels and right-panel statics are represented; trackbar option-label statics `0x699/0x69B/0x69C` are missing.` (evidence: binary matrix rows 12..14; `rg 0x699/0x69B/0x69C`)
- `[RESOLVED] OQ-11 - Can `ShellControlId` be used as a complete matrix? -> No; it omits many active children, including row combos, checkboxes, trackbars, status, and option-label statics.` (evidence: `layout.rs:52..74`)
- `[RESOLVED] OQ-12 - Are INI/default sources needed for rect ownership? -> No; child rect policy is shell resource/helper code, not INI-driven.` (evidence: helper decompiles; prior complete matrix)
- `[DEFERRED] OQ-13 - Exact runtime hover text content for status/help `0x695`.` (category: out-of-scope; reason: this slice verifies rect ownership, not tooltip text population; next-step-if-pursued: trace command/hover tooltip writes to `0x695`)

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Status/help static `0x695` is a visible `0x102` child anchored bottom-left by `FUN_0060B550`: `(10,459,615,20)` at 640, `(10,579,615,20)` at 800, `(122,663,615,20)` at 1024 | `0x0060C2B6..0x0060C2D0`; `FUN_0060b550`; complete matrix row 42 | missing | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, future hover/help state | Add a first-class status/help rect and render blank/tooltip text through it when the shell owns hover help | Test `skirmish_status_help_strip_0x695_bottom_left_rects` checks the three standard resolution rects | Do not derive this from lower strip art or right panel; it is a child static with its own bottom-left helper |
| Trackbar option-label statics `0x699`, `0x69B`, `0x69C` preserve resource rects `(302,286,90,16)`, `(302,314,90,16)`, `(302,341,90,16)` | complete matrix rows 12..14; fallback branch in `0x0060C0C0` | missing | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Add first-class option-label rects or an explicitly named grouped struct; render localized Game Speed/Credits/Unit Count labels from those rects | Test `skirmish_option_label_static_rects_preserve_resource_positions` verifies the three rects at 640/800/1024 | Do not infer these labels from trackbar value/plaque rects; they are separate visible Static children |
| Most requested shell components already have first-class rect ownership in current Rust: buttons, preview, right-panel text, player-name rect, row combos, color combos, flags, checkboxes, trackbars, and column labels | Rust scan `layout.rs:147..161`, `:417..535`; binary helper evidence above | none observed for rect ownership; player-name semantics partial | same layout/state/render surfaces | Preserve current direct rect fields/arrays and keep tests that pin helper/fixup rects | Test `skirmish_visible_child_rect_ownership_matrix_is_complete_for_cohesive_shell` enumerates every cohesive component and fails until status and option labels are included | Do not collapse this into `ShellControlId`; that enum is not complete |
| `SkirmishShellDrawRole` and `ShellControlId` are subsets, not complete child-control order/inventory models | `app_skirmish_shell_render.rs:101..115`; `layout.rs:52..74` | partial if used for audits | semantic draw-order tests and future UI ownership docs | Name/limit these as sprite/control subsets, or add a separate complete rect inventory for parity audits | Test `skirmish_shell_control_id_is_not_used_as_complete_0102_inventory` or a complete matrix test with 72 binary ids | Do not mark parity complete by checking only these enums |
| Player-name edit `0x6A0` has rect ownership but not complete semantic input/render behavior | binary `0x006AE6E0`, `0x00614190`, player-name edit report; Rust `layout.rs:234..248`, `state.rs:544`, `app_skirmish_shell_render.rs:1762..1768` | rect present; render/input partial | `state.rs`, `app.rs`, `app_skirmish_shell_render.rs` | Keep the first-class rect; later wire focus/text/caret/render to `shell.player_name` and Start readback behavior | Test `skirmish_player_name_edit_rect_participates_in_input_focus_order` after input is implemented | Do not regress the verified `(58,59,151,23)` rect while adding edit semantics |

Concrete Rust test-name proposals:

- `skirmish_status_help_strip_0x695_bottom_left_rects`
- `skirmish_option_label_static_rects_preserve_resource_positions`
- `skirmish_visible_child_rect_ownership_matrix_is_complete_for_cohesive_shell`
- `skirmish_shell_control_id_is_not_used_as_complete_0102_inventory`
- `skirmish_player_name_edit_rect_participates_in_input_focus_order`

## 7. Negative Facts / Do Not Do

- Do not treat the status/help strip as decorative lower-strip chrome. Active in YR: No. It is child static `0x695`, routed through `FUN_0060B550`.
- Do not omit `0x699/0x69B/0x69C` because trackbar values are rendered. Active in YR: No. The labels are separate visible Static children.
- Do not use `ShellControlId` as the complete dialog `0x102` inventory. Active in Rust: it is a subset; Active in YR: the binary has 72 visible children.
- Do not globally center, scale, or high-res-offset row combos, color combos, flags, checkboxes, trackbars, column labels, or option-label statics. Active in YR: No; ordinary children preserve resource rects except explicit `FUN_0060B950` fixups.
- Do not count player-name parity complete from rect ownership alone. The rect is present, but current render/input semantics still need the owner-draw edit behavior.

## 8. Remaining Uncertainty

- Exact status/help text population and hover ownership are outside this rect-ownership slice. The rect and helper branch are verified; tooltip content writers are not traced here.
- Semantic draw ordering for all text/control surfaces is not fully represented by `SkirmishShellDrawRole`; this report classifies that enum as a subset rather than proving a complete draw-order replacement.
- Runtime screenshots were not taken in this slot; the report compares binary layout policy and current Rust source/tests.

## 9. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md` replacement wording for current Rust status: "Current Rust now owns Start/Choose snap rects, Back, preview, right-panel title/game/map text, player-name fixup rect, row/color/start/team/AI combos, flags, checkboxes, and trackbars. Remaining first-class rect gaps for a cohesive shell are status/help static `0x695` and trackbar option-label statics `0x699/0x69B/0x69C`; player-name edit semantics remain partial despite rect ownership."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` follow-up wording: "The note that status `0x695` and option labels `0x699/0x69B/0x69C` lack named Rust layout fields remains current; the older warnings about right-panel text, checkboxes, and trackbars being absent are superseded by current dirty Rust."

## Sources

- Ghidra read-only decompile: `FUN_006ae2c0`, `FUN_0060c4a0`, `ResizeShellChildControl_0060C0C0`, `FUN_00608cd0`, `FUN_00609730`, `FUN_00601360`, `FUN_0060b000`, `FUN_0060b1d0`, `FUN_0060b350`, `FUN_0060b550`, `FUN_0060b950`.
- Ghidra assembly contexts: `0x006AE31C..0x006AE328`, `0x0060C1B0..0x0060C1C8`, `0x0060C213..0x0060C227`, `0x0060C2B6..0x0060C2D0`, `0x0060BE0A..0x0060BE20`, `0x0060C065..0x0060C092`.
- Prior reports referenced: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`, `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Rust scanned read-only: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/mod.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, plus targeted `rg` checks for `0x695`, `0x699`, `0x69B`, `0x69C`, `status`, and `help`.
