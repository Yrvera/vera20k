# Skirmish Shell Complete Rect Table — Investigation Plan

> **For Codex:** This plan scopes a `/re-investigate` pass. Execute it by
> running `/re-investigate skirmish shell complete rect table` with this plan
> loaded as context, OR split the function inventory into bounded follow-up
> batches.

**Topic:** Offline YR Skirmish dialog `0x102` complete visible rect/paint table  
**Scope Size:** Medium-large — approx. 27 functions, 8 INI keys  
**Est. Effort:** ~5-8 hours of `/re-investigate` work  
**Prior Research:** Partial high-confidence reports exist; this plan targets the
remaining unknowns before expanding the Rust expected-rect table.  
**Expected Output:** research document at
`docs/research/skirmish-ui/SKIRMISH_SHELL_COMPLETE_RECT_TABLE_GHIDRA_REPORT.md`  
**Next Pipeline Step:** `/brainstorm` then implement the expanded table and tests
after the research report resolves the unknowns.

---

## 1. Goal

Produce a verified, player-visible geometry and paint table for every visible
offline Skirmish setup surface/control that Rust renders or hit-tests. The
report must separate binary-verified coordinates from template-derived values,
resolve every remaining `UNKNOWN` before the table expands, and state whether
each finding is active in standard Yuri's Revenge.

The target output is not a generic Win32 dialog emulator. The target output is a
Rust-facing table of exact rects, helper formulas, text caller rects, dropdown
row/scrollbar geometry, preview marker projection/clipping, and modal-control
geometry needed to assert pixel/layout parity.

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps |
|--------|-------|------------|------------|
| `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md` | `0x102` resize helper branch policy, Start/Choose/Back, right-anchor allowlist, one-pixel fixups | High | Does not enumerate every child control's final rect or text caller rect. |
| `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md` | Static controls `0x694`, `0x6EC`, `0x5A8`, preview `0x468`, text animation classification, preview paint boundary | High | Exact visible strings/source buffers partly deferred; text caller rect values need Rust-table handoff. |
| `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md` | Checkbox/trackbar active callbacks, control IDs, option mapping, assets, ranges | High | Needs exact final rect table reconciliation for every checkbox/trackbar helper and text rect. |
| `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md` | Checkbox visible labels, tooltip/string IDs, click/apply behavior | High | Does not fully tabulate label draw rects for the expanded geometry table. |
| `SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md` | Player/AI combo, color ownership, flag mapping, broad combo deltas | Medium-high | Start/team combo parity outside trace; full dropdown row and scrollbar paint not complete. |
| `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md` | Owner-proc routing for static/button/checkbox/combo; Rust architectural implication | High | Full combo dropdown/listbox row behavior explicitly out of scope. |
| `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md` | Common paint order, right-panel order, parent background, preview/start boundary | High | Combo/dropdown, text rect/color, Choose Map modal composition, full marker projection called out as separate gaps. |
| `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md` | Preview header defaults, Dustbowl loose-map path, live marker gating | High | Marker projection/clipping is partially covered but needs complete table-facing rect rules. |
| `SCENARIO_PREVIEW_BOUNDS_STOCK_MAP_POPULATION_GHIDRA_REPORT.md` | Stock-map header population and preview/start fields | High | Object reuse/zeroing edge cases noted; not a full UI table handoff. |
| `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md` | Rules out `BTN-MINS/BTN-PLUS` SHPs for standard `0x102` sliders | High | Confirms negative asset path only. |
| `docs/plans/2026-05-22-skirmish-shell-ui-parity-design.md` | Prior implementation design for current shell coverage | Design, not evidence | Explicitly deferred full dropdown rows, preview markers, exact >800 screenshots. |
| `docs/visual-checks/skirmish-shell/README.md` | Current expected rect table for key anchors | Mixed, mostly verified | Intentionally incomplete; needs expansion after this research. |

**Conflicts between reports:** older viewport-origin reports/listings treat
Start/Choose as generic right-anchor users. The complete resize-policy report
supersedes that: Start `0x617` and Choose `0x5AA` route through `FUN_0060B000`
because owner-draw Button metadata preempts the static helper.

## 3. Function Inventory

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|-------|---------|--------------|--------------|--------------|----------------|
| 1 | 1 | `0x006AE2C0` | `FUN_006AE2C0` | Offline Skirmish dialog launcher and modal lifetime | LIGHT | Low |
| 2 | 1 | `0x006AE3F0` | `FUN_006AE3F0` | Dialog proc; routes init/paint/command and preview paint | FULL | Low |
| 3 | 1 | `0x00622B50` | `FUN_00622B50` | Common shell parent handler called before Skirmish paint | MEDIUM | Low |
| 4 | 1 | `0x0060C0C0` | `ResizeShellChildControl_0060C0C0` | Central child rect policy for `0x102` | FULL | Low |
| 5 | 1 | `0x0060B000` | `FUN_0060B000` | Owner-draw PCX button snap helper for Start/Choose | FULL | Low |
| 6 | 1 | `0x0060B1D0` | `FUN_0060B1D0` | Generic right-anchor helper for statics/preview | FULL | Low |
| 7 | 1 | `0x0060B350` | `FUN_0060B350` | Back button bottom/right `SDBTNANM` helper | FULL | Low |
| 8 | 1 | `0x0060B950` | `FUN_0060B950` | One-pixel child fixups and title y adjustment | FULL | Low |
| 9 | 1 | `0x00608CD0` | `FUN_00608CD0` | `0x102` right-anchor allowlist | MEDIUM | Low |
| 10 | 1 | `0x00609730` | `FUN_00609730` | `0x102` bottom/right allowlist for Back | MEDIUM | Low |
| 11 | 1 | `0x00601360` | `FUN_00601360` | `0x695` bottom-left branch selection | MEDIUM | Low |
| 12 | 2 | `0x0060F9A0` | `FUN_0060F9A0` | Child subclass/owner-proc assignment; determines control kind | FULL | Low |
| 13 | 2 | `0x0060A5B0` | `FUN_0060A5B0` | Reclassifies static text controls and animation state | FULL | Low |
| 14 | 2 | `0x00602490` | `FUN_00602490` | Static text allowlist for `0x694`, `0x6EC`, `0x5A8` | MEDIUM | Low |
| 15 | 2 | `0x006153E0` | `OwnerDraw_Static_006153E0` | Static text paint, backing-surface update, text rect/color flags | FULL | Low |
| 16 | 2 | `0x00621040` | `FUN_00621040` | Shared shell text draw call: rect, align flags, clipping, color conversion | FULL | Low |
| 17 | 2 | `0x00612B70` | `OwnerDraw_Button_00612B70` | Button text/art offsets, press state, final label rect | MEDIUM | Low |
| 18 | 2 | `0x006163A0` | `OwnerDraw_Checkbox_006163A0` | Checkbox icon/text rects, click bounds, label drawing | FULL | Low |
| 19 | 2 | `0x0061D950` | `OwnerDraw_Trackbar_0061D950` | Trackbar rail/thumb/plaque/value text rects and disabled visuals | FULL | Low |
| 20 | 2 | `0x00617250` | `OwnerDraw_ComboBox_00617250` | Collapsed combo draw, dropdown create/open/close, row behavior | FULL | Low |
| 21 | 2 | `0x006040B0` | `FUN_006040B0` | Tooltip/string dispatcher; useful for labels/help strings tied to controls | LIGHT | Low |
| 22 | 3 | `0x005E2EF0` | `FUN_005E2EF0` | Updates game-type text `0x6EC` | MEDIUM | Low |
| 23 | 3 | `0x005E2F60` | `FUN_005E2F60` | Updates scenario/map label `0x5A8` | MEDIUM | Low |
| 24 | 3 | `0x006AE6E0` | `FUN_006AE6E0` | Dialog init/population: combos, checkboxes, trackbars, labels | FULL | Low |
| 25 | 3 | `0x006ACEE0` | `FUN_006ACEE0` | Start/Back/Choose command handler; reads control states | MEDIUM | Low |
| 26 | 3 | `0x006067A0` | `FUN_006067A0` | Preview guard/helper before `DrawStartPositions` | MEDIUM | Low |
| 27 | 3 | `0x00640710` | `DrawStartPositions` | Preview child rect conversion, aspect-fit, live `STARTBUT`/label projection and clipping | FULL | Low |

**Phase 1 checkpoint:** after functions #1-#11, executor must produce the final
resize/final-child-rect matrix for all `0x102` controls currently represented in
Rust plus any visible controls Rust is missing. If this matrix reveals unknown
controls beyond this plan, revise before Phase 2.

## 4. Detail Checklist

- **Complete child-control inventory:** enumerate every visible `0x102` child
  control ID, class, style, original resource rect, resize helper branch, final
  rect at 640x480, 800x600, 1024x768, and whether Rust currently has a field.
- **Known helper policies:** verify `FUN_0060B000`, `FUN_0060B1D0`,
  `FUN_0060B350`, `FUN_0060B550`, fallback preserve-rect behavior, and exact
  center-offset handling.
- **One-pixel fixups:** extract all `0x102` cases from `FUN_0060B950`, including
  `0x694`, `0x50C`, `0x54E`, `0x693`, `0x696`, `0x69A`, `0x6A0`, and any other
  children not currently represented.
- **Text caller rects/colors:** for button labels, checkbox labels, trackbar value
  text, combo selected text/dropdown rows, right-panel statics, and preview marker
  labels, record caller rect, align flags, color source, reveal/clipping behavior,
  and disabled-state behavior.
- **Dropdown/listbox geometry:** for each combo family, record collapsed face,
  arrow hit rect, open dropdown top/height, row height, max visible rows, content
  width shrink when scrollbar appears, scrollbar width/buttons/thumb rect, top-index
  scrolling, mouse wheel/page/drag behavior, and whether retail uses native combo
  storage or custom windows for each visible effect.
- **Combo content:** verify item labels/item data order for AI type, side/country,
  color, start, and team combos where rect/row count depends on content size or
  disabled rows.
- **Flag and swatch paint:** verify flag static clipping/centering, color swatch
  inset/size, color ramp source, disabled/random row visuals, and refresh behavior.
- **Preview surface and markers:** record preview placeholder rect, fitted preview
  image rect formula, integer scaling/rounding, `STARTBUT.SHP` offset, numeric
  label rect/align/color, clip boundary, and gates that skip overlays.
- **Choose Map modal boundary:** confirm which modal `0x6B` rects belong outside
  the setup `0x102` table and which should be linked as a separate table.
- **Draw order dependencies:** note any rect whose visible output depends on
  being drawn before/after chrome, cached parent blit, preview surface, dropdown,
  or text.
- **Edge cases:** null preview object, no `[Header]`, marker count `<=0` or `>=9`,
  dropdown item count below visible rows, disabled checkbox/trackbar/combo, 640 vs
  800 vs >800 modes, and 125% DPI screenshot capture caveat.

## 5. INI Keys in Scope

| Key | Section | Default | Suspected Purpose | Currently Parsed in Rust? |
|-----|---------|---------|-------------------|----------------------------|
| `MinMoney` | `[MultiplayerDialogSettings]` | `5000` in `rulesmd.ini` | Credits trackbar minimum and value text range | Yes/partial |
| `Money` | `[MultiplayerDialogSettings]` | `10000` in `rulesmd.ini` | Credits default | Yes/partial |
| `MaxMoney` | `[MultiplayerDialogSettings]` | `10000` in `rulesmd.ini` | Credits maximum | Yes/partial |
| `MoneyIncrement` | `[MultiplayerDialogSettings]` | `100` in `rulesmd.ini` | Credits trackbar step | Yes/partial |
| `MinUnitCount` | `[MultiplayerDialogSettings]` | `0` in `rulesmd.ini` | Unit-count trackbar minimum | Yes/partial |
| `UnitCount` | `[MultiplayerDialogSettings]` | `10` in `rulesmd.ini` | Unit-count default | Yes/partial |
| `MaxUnitCount` | `[MultiplayerDialogSettings]` | `10` in `rulesmd.ini` | Unit-count maximum | Yes/partial |
| `GameSpeed` | `[MultiplayerDialogSettings]` | `1` in `rulesmd.ini` | Game-speed default; visual value uses inverse mapping | Yes/partial |
| `ShortGame` | `[MultiplayerDialogSettings]` | `yes` in `rulesmd.ini` | Checkbox default | Yes/partial |

Other checkbox defaults (`SuperWeaponsAllowed`, `BuildOffAlly`, `MCVRepacks`,
`CratesAppear`) are covered by prior reports through Rules constructor/session
settings and should be cross-checked only if they affect visible checked state in
the final table.

## 6. Caller & Integration Map

| Caller Address | Calls Into | When Invoked | Should Executor Decompile? |
|----------------|------------|--------------|----------------------------|
| `0x006AE2C0` | dialog create/loop and cleanup | Standard offline Skirmish entry | YES, light context |
| `0x006AE3F0` | common shell handler and `DrawStartPositions` | Dialog messages including paint/command | YES, full |
| `0x00622B50` | child init/resize/common parent paint | First stage of dialog handling | YES, medium |
| `0x0060C4A0` | child resize enumeration into `0x0060C0C0` | Dialog init/fullscreen move | YES if not already fully covered in Phase 1 |
| `0x0060F9A0` | owner-proc subclass assignment | Child enumeration during init | YES, full |
| `0x006AE6E0` | combo/checkbox/trackbar initialization | custom init/populate | YES, full |
| `0x006ACEE0` | Start/Choose/Back action reads controls | command dispatch | YES, medium |
| `0x005E2EF0` | `0x6EC` text update | selected mode/game-type refresh | YES, medium |
| `0x005E2F60` | `0x5A8` text update | selected map refresh | YES, medium |

Rust integration today:

- `src/ui/skirmish_shell/layout.rs` computes rects and helper sub-rects.
- `src/ui/skirmish_shell/state.rs` uses layout rects for hit testing, dropdown
  state, checkbox/trackbar state, and modal state.
- `src/app_skirmish_shell_render.rs` consumes layout/state for sprite/text
  instances and semantic draw order.
- `src/render/skirmish_shell_chrome.rs` loads shell assets used by buttons,
  flags, checkboxes, trackbars, dropdown arrows/scrollbar pieces, and markers.
- App-level input in `src/app.rs` routes mouse events to Skirmish shell handlers.

## 7. TS-Legacy Risk Register

- **Generic shell code is shared with other dialogs and older code.** Every helper
  branch must be confirmed active for parent dialog `0x102`; do not import rules
  from main-menu `0xE2` or network lobbies unless the parent-id predicate includes
  `0x102`.
- **Trackbar `BTN-MINS/BTN-PLUS` SHPs are not active for standard offline `0x102`.**
  Prior report marks them conditional/generic and not used by the standard
  Skirmish trackbars. Executor should preserve this negative finding.
- **`GUI:Closed` appears in network-player paths, not standard offline `0x102`
  AI row population.** Do not add it to offline AI row state combo content unless
  a live offline path is found.
- **Preview marker sources differ by map source.** Loose Dustbowl lacks `[Header]`
  and skips live `STARTBUT.SHP` overlays; generated/cache or stock embedded paths
  may populate preview fields. Do not synthesize overlays from gameplay
  `[Waypoints]` unless the verified header/cache path supplies the preview fields.

## 8. Current Rust Implementation Surface

- `src/ui/skirmish_shell/layout.rs`: current layout struct includes right panel,
  Start/Choose/Back, preview, right-panel text rects, column labels, player name,
  row combos, color combos, flags, trackbars, checkboxes, Choose Map modal layout,
  and many geometry helpers.
- `src/ui/skirmish_shell/state.rs`: owns shell state, hit testing, owner-draw
  buttons, checkbox toggles, trackbar drags, combo dropdown state/top index, and
  modal interaction.
- `src/app_skirmish_shell_render.rs`: renders right-panel chrome, buttons,
  checkboxes, trackbars, combos/dropdowns, labels, preview texture, start markers,
  and marker labels.
- `src/render/skirmish_shell_chrome.rs`: loads retail shell and owner-draw assets,
  including PCX/SHP pieces for the above controls.
- `docs/visual-checks/skirmish-shell/README.md`: current human table for only
  high-value anchors.

Known implementation/test caveat: the workspace is currently dirty and may have
unrelated compile blockers in bridge code. Research execution should not modify
Rust files.

## 9. Deferred Open Questions

1. What is the complete final child-control rect table for all visible `0x102`
   controls at 640x480, 800x600, and 1024x768?
2. Which rows are binary-verified final rects versus resource-template values
   transformed by a verified helper?
3. What exact text caller rects and align/color flags apply to every visible text
   surface Rust renders?
4. What exact dropdown/listbox row geometry, scrollbar visual geometry, and input
   behavior should be asserted for each combo family?
5. Which dropdown/scrollbar behaviors are already verified enough for hard tests,
   and which require screenshot comparison rather than table-only assertions?
6. What is the full preview marker rect/projection/clipping rule for live
   `STARTBUT.SHP` overlays and numeric labels when the overlay path is active?
7. Which `0x6B` Choose Map modal rects should be part of a separate modal table
   rather than the setup `0x102` table?
8. Are there any visible `0x102` controls Rust still does not represent that must
   be added before the table can honestly say "complete"?

## 10. Execution Strategy

**Multi-phase single plan.** Execute in three phases:

1. **Phase 1 — Final child-control rect matrix.** Decompile/verify functions
   #1-#11 and produce the complete `0x102` child table first.
2. **Phase 2 — Owner-draw/text/dropdown details.** Decompile functions #12-#21,
   focused on sub-rects, text rects, dropdown and scrollbar geometry.
3. **Phase 3 — Runtime update/preview/modal boundaries.** Decompile functions
   #22-#27 to resolve dynamic text, preview marker projection, command-time state,
   and what belongs in the separate Choose Map modal table.

Pause after Phase 1. If the child-control inventory is larger than expected or
reveals a separate dialog/control family, revise before continuing.

## 11. Success Criteria

The executed research document must:

- Answer every question in Section 1 and Section 9.
- Include every function from Section 3, or explicitly justify omission.
- Provide a table row for every visible `0x102` control with final rects at
  640x480, 800x600, and 1024x768.
- Provide separate sub-rect tables for text, checkbox icon/text, trackbar rail/
  thumb/plaque/value, combo collapsed/dropdown/scrollbar, preview image/marker/
  label, and buttons.
- Mark every row as `verified-binary`, `template-through-verified-helper`,
  `conditional`, or `not-active-in-standard-YR`.
- State "Active in YR: Yes/No/Conditional" for every finding.
- Cite Ghidra addresses for every HIGH-confidence claim.
- Name any remaining unknown as a blocker before Rust table expansion.

## Sources

- Ghidra addresses sampled: none live during this planning pass; addresses above
  are from prior research reports.
- Docs searched:
  - `docs/research/skirmish-ui/*.md`
  - `docs/plans/*.md`
  - `docs/visual-checks/skirmish-shell/*.md`
- INI files checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
  - `ini/artmd.ini`
  - `ini/art.ini`
- Related plans:
  - `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
  - `docs/plans/2026-05-22-skirmish-shell-ui-parity-design.md`
  - `docs/plans/2026-05-22-high-res-skirmish-shell-screenshot-parity-plan.md`
  - `docs/plans/2026-05-22-skirmish-checkbox-trackbar-ownerdraw-design.md`
  - `docs/plans/2026-05-22-skirmish-choose-map-modal-design.md`
