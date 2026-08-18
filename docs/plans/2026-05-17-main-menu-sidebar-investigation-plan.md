# Main Menu Sidebar / Graphic Menu - Investigation Plan

> **For Claude:** This plan scopes a `/re-investigate` pass. Execute it by
> running `/re-investigate main menu sidebar / graphic menu` with this plan
> loaded as context. Do not write Rust during the investigation.

**Topic:** Yuri's Revenge initial main menu button/sidebar, backed by the
`GraphicMenu` / `ShapeButton` shell path rather than the in-game `SidebarClass`.
**Scope Size:** Medium - approx. 28 functions, 2 INI sound keys, 1 custom dialog
control family.
**Est. Effort:** ~5-7 hours of `/re-investigate` work.
**Prior Research:** Skirmish shell and owner-draw reports cover adjacent shell
systems; no dedicated main menu `GraphicMenu` report was found.
**Expected Output:** research document at
`docs/research/MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
**Next Pipeline Step:** `/brainstorm` before implementation if the report shows a
new menu UI layer is needed; `/write-plan` directly only for small asset/sound
fixes.

---

## 1. Goal

Recover how standard YR draws and drives the initial main menu button/sidebar:
which dialog/control owns it, which assets it loads, how buttons animate and play
sounds, how menu selections map to `Main_Game` return codes, and how 640/800+
screen widths affect placement. The report must clearly separate this path from
the already researched Skirmish dialog `0x102` and from in-game `SidebarClass`.

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps |
|--------|-------|------------|------------|
| `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md` | Offline Skirmish dialog `0x102`, parent background, owner-draw button path | High for Skirmish | Does not cover initial main menu `GraphicMenu` / control `0x71A` |
| `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md` | Skirmish parent backgrounds, text, preview overlay | High for 640/800 Skirmish | Explicitly not a main menu report |
| `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md` | Win32 owner-draw control callbacks, button PCX pieces, combo/flag drawing | High | Covers owner-draw dialog controls, not the custom graphic menu list |
| `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` | GAME.FNT and shell text drawing wrappers | High | Text may be reused by main menu, but specific main menu labels/layout are not mapped |
| `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` | In-game gadget framework and shell-vs-game UI boundary | Medium/High | Says shell UI can be implemented differently, but does not document main menu visuals |
| `GLOBAL_SOUNDS_GHIDRA_REPORT.md` | `[AudioVisual]` sound keys including GUI sounds | High | Sound trigger sites for main menu button select/slide still need tracing |
| `SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md` | Sound trigger inventory | Medium for this scope | Mentions `ShellButtonSlideSound` as main menu button, but no full main menu trace |
| `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` and related sidebar docs | In-game build sidebar | High | Different system; useful only as a naming trap |

**Conflicts between reports:** none found for this specific scope. The main risk
is terminology: "sidebar" in prior docs usually means in-game build sidebar,
while this plan targets the initial shell graphic/menu button stack.

## 3. Function Inventory

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|-------|---------|--------------|--------------|--------------|----------------|
| 1 | 1 | `0x00531CC0` | `FUN_00531cc0` | Main menu loop found from `Main_Game`; creates shell dialog, positions child `0x71A`, sends `0x4E3/0x4E4` with `Ra2ts_s/l` | FULL | Low |
| 2 | 1 | `0x0052B9B0` | `FUN_0052b9b0` | Helper that repositions `0x71A` and sends the same `Ra2ts_s/l` messages | FULL | Low |
| 3 | 1 | `0x0052D9A0` | `Main_Game` | Caller/context; maps menu return codes to campaign, network, options, Skirmish, exit | MEDIUM | Low |
| 4 | 1 | `0x00622650` | `FUN_00622650` | Common shell dialog creation through `CreateDialogIndirectParamA`; needed to identify resource ID/proc for main menu | FULL | Low |
| 5 | 1 | `0x00622800` | `FUN_00622800` | Dialog show/activation helper called before the main menu message loop | MEDIUM | Low |
| 6 | 1 | `0x00623120` | `FUN_00623120` | Common shell message/tick pump used while menu waits for exit code | MEDIUM | Low |
| 7 | 1 | `0x00622720` | `FUN_00622720` | Common dialog teardown after main menu exits | MEDIUM | Low |
| 8 | 1 | `0x004F2140` | `GraphicMenu__Constructor` | Loads/initializes `Title.PCX`/`Intro` graphic menu state | FULL | Medium - inherited TS-style graphic menu code possible |
| 9 | 1 | `0x004F21A0` | `GraphicMenu__Constructor` | Destructor/reset path; clears surfaces and item list | MEDIUM | Medium |
| 10 | 1 | `0x004F2300` | `FUN_004f2300` | GraphicMenu interaction loop: input scan, highlight, select, return item id | FULL | Medium |
| 11 | 1 | `0x004F4780` | `FUN_004f4780` | Blits shell/menu surface to main display with screen/sidebar offsets | FULL | Medium - name collision with in-game sidebar offsets |
| 12 | 2 | `0x004F3140` | `FUN_004f3140` | Builds image-backed menu items from INI keys `Origin`, `ActiveRect`, `Image`, `Highlighted`, `Disabled`, sounds | FULL | Medium |
| 13 | 2 | `0x004F38B0` | `FUN_004f38b0` | Parses one menu item section and dispatches image/shortcut/other item constructors | FULL | Medium |
| 14 | 2 | `0x004F3460` | `GraphicMenuImageItem__Constructor` | Image menu item: loads normal/highlight/disabled art and highlight sound | FULL | Medium |
| 15 | 2 | `0x004F3840` | `GraphicMenuImageItem__Constructor` | Alternate image-item constructor; verify if live for main menu | MEDIUM | Medium |
| 16 | 2 | `0x004F30E0` | `GraphicMenuAnimItem__Constructor` | Animation item constructor; determine whether main menu button slide uses this | MEDIUM | High - may be unused TS path |
| 17 | 2 | `0x004F3C40` | `GraphicMenuShortcutItem__Constructor` | Shortcut/key menu item constructor; menu hotkeys may use this | MEDIUM | Medium |
| 18 | 2 | `0x004F3D40` | `FUN_004f3d40` | Shortcut item parse helper called by #17 | MEDIUM | Medium |
| 19 | 2 | `0x004F3A50` | `GraphicMenuItem__Constructor` | Base item constructor; verify fields/state shared by image/shortcut items | MEDIUM | Medium |
| 20 | 2 | `0x004F3A70` | `GraphicMenuItem__Constructor` | Base item destructor/reset | LIGHT | Medium |
| 21 | 2 | `0x004F3A90` | `FUN_004f3a90` | Highlight enter/leave called by #10; should resolve visual/sound behavior | FULL | Medium |
| 22 | 2 | `0x004F3B10` | `FUN_004f3b10` | Item state helper near highlight path | MEDIUM | Medium |
| 23 | 2 | `0x004F1B00` | `FUN_004f1b00` | Timer helper based on `GetRadarTimer`; possible animation delay state | MEDIUM | Medium |
| 24 | 2 | `0x004F1B20` | `FUN_004f1b20` | Timer remaining helper; decode units and use sites | MEDIUM | Medium |
| 25 | 2 | `0x0069DCF0` | `ShapeButtonClass__Constructor` | RTTI/string cluster near `ShapeButtonClass`; determine relationship to main menu graphic buttons | LIGHT | Medium |
| 26 | 2 | `0x0069DD30` | `ShapeButtonClass__Constructor` | Shape button overload; verify if only shell/dialog or also in-game | LIGHT | Medium |
| 27 | 2 | `0x0069DDC0` | `ShapeButtonClass__Constructor` | Shape button overload with most xrefs; include only if called from main menu path | LIGHT | Medium |
| 28 | 3 | `0x004790B0` | `FUN_004790b0` | Sets a global before main menu dialog; likely shell/campaign/menu state | MEDIUM | Medium |
| 29 | 3 | `0x006040B0` | `FUN_006040b0` | String tooltip registry for `STT:MainButton*` and `STT:MainOpt*` keys | MEDIUM | Low |
| 30 | 3 | `0x006691E0` | `RulesClass__ReadAudioVisual` | Parses `GUIMainButtonSound` and `ShellButtonSlideSound` defaults | LIGHT | Low |
| 31 | 3 | `0x004E1D00` | `OptionsClass__ShowInGameDialog` | Compare launcher/options shell flow only if `MainButtonOptions` dispatch reaches it | LIGHT | Low |
| 32 | 3 | `0x0055FC80` | `OptionsClass__ShowLauncherDialog` | Main menu options dispatch target from `Main_Game` case 5 | MEDIUM | Low |

**Phase 1 checkpoint:** after functions #1-#11, pause and summarize: dialog
resource ID/proc, control `0x71A` class, `Ra2ts_s/l` asset role, menu item count,
and return-code mapping. If `0x71A` is not the graphic menu owner, revise the
function inventory before Phase 2.

## 4. Detail Checklist

- **Dialog/control identity:** resource ID created by `FUN_00531CC0`, dialog proc,
  child control `0x71A` class/style/rect, and whether it is custom-window,
  owner-draw, or another control type.
- **Custom messages:** exact meaning of `0x4E3` and `0x4E4`; arguments for
  `Ra2ts_s` and `Ra2ts_l`; whether these are INI section names, asset bases, or
  layout profiles.
- **Assets:** every PCX/SHP/PAL loaded for the initial main menu, including
  `Title.PCX`, `Ra2ts_s`, `Ra2ts_l`, button normal/highlight/disabled art, any
  slide/animation frames, and whether `MAINBTTN.PAL` participates.
- **Layout:** 640 vs non-640 placement, `(screen_w - 800) / 2` and
  `(screen_h - 600) / 2` offsets, active rects, origin offsets, hit boxes, and
  clipping behavior.
- **Menu item model:** fields in `GraphicMenu`, `GraphicMenuItem`,
  `GraphicMenuImageItem`, `GraphicMenuAnimItem`, and `GraphicMenuShortcutItem`
  needed for observable behavior.
- **Input ordering:** mouse highlight, click, keyboard shortcut, escape/back,
  focus behavior, and the exact point where the selected item id is returned.
- **Animation/timing:** use of `GetRadarTimer`, `timeGetTime`, frame delays,
  slide-in/out behavior, and whether `ShellButtonSlideSound` is active by default.
- **Sounds:** highlight, select, `GUIMainButtonSound`, `ShellButtonSlideSound`,
  and whether item-local INI sound fields override rules defaults.
- **Text/tooltips:** `STT:MainButton*`, `STT:MainOpt*`, and whether labels are
  text-drawn, baked into images, or only tooltips.
- **Return codes:** map item ids to `Main_Game` cases: single player/campaign,
  network, WOL, movies, options, exit, Skirmish.
- **Edge cases:** missing assets, 640x480, 800x600, >1023 width centering,
  disabled buttons, hidden internet/WOL options, and Smart App/launcher return
  paths.

## 5. INI Keys in Scope

| Key | Section | Default | Suspected Purpose | Currently Parsed in Rust? |
|-----|---------|---------|-------------------|----------------------------|
| `GUIMainButtonSound` | `[AudioVisual]` | `MenuClick` in `rulesmd.ini` | Main shell button click/select sound | Yes in rules/audio data if global sounds parser is wired; trigger not proven |
| `ShellButtonSlideSound` | `[AudioVisual]` | empty in `rulesmd.ini` | Main button slide/entry animation sound | Parsed by binary; Rust trigger likely absent |
| `Image` | `Ra2ts_s/l` or menu item sections | unknown | Normal image per graphic menu item | No known Rust parser |
| `Highlighted` | `Ra2ts_s/l` or menu item sections | unknown | Highlighted image per item | No known Rust parser |
| `Disabled` | `Ra2ts_s/l` or menu item sections | unknown | Disabled image per item | No known Rust parser |
| `Origin` | `Ra2ts_s/l` or menu item sections | unknown | Item origin offset | No known Rust parser |
| `ActiveRect` | `Ra2ts_s/l` or menu item sections | unknown | Hit rectangle | No known Rust parser |
| `HighlightSound` | `Ra2ts_s/l` or menu item sections | unknown | Per-item hover sound override | No known Rust parser |
| `SelectSound` | `Ra2ts_s/l` or menu item sections | unknown | Per-item select sound override | No known Rust parser |
| `SelectVQ` | `Ra2ts_s/l` or menu item sections | unknown | Video/VQ or command on select | No known Rust parser |

## 6. Caller & Integration Map

| Caller Address | Calls Into | When Invoked | Should Executor Decompile? |
|----------------|------------|--------------|----------------------------|
| `0x0052D9A0` | `0x00531CC0` | `Main_Game` menu state when game mode returns to shell | YES - dispatch context is critical |
| `0x00531CC0` | `0x00622650`, `0x0052B9B0`-equivalent logic, `0x00623120` | Initial main menu screen loop | YES |
| `0x0052B9B0` | child `0x71A` messages | Shell resize/reposition path | YES |
| `0x004F2300` | item highlight/select virtuals | Graphic menu item loop, if `0x71A` dispatches here | YES after Phase 1 proves linkage |
| `0x006040B0` | tooltip string table | Tooltip setup for `STT:MainButton*`/`MainOpt*` | MEDIUM |
| `0x006691E0` | Rules sound parse | Rules load, not per-frame | LIGHT |

Rust today:

- `src/ui/main_menu.rs` is an egui menu/skirmish setup surface, not original
  graphic-menu chrome.
- `src/app.rs` has a dev-only Skirmish shell path gated by
  `RA2_DEV_SKIRMISH_SHELL`; this is for dialog `0x102`, not the initial main
  menu graphic menu.
- `src/render/skirmish_shell_chrome.rs` has Skirmish-specific verified assets
  such as `MNSCRNL.SHP` and `MnScrnLCoopGameSetup.shp`; do not reuse those as
  main menu evidence unless the investigation proves shared use.

## 7. TS-Legacy Risk Register

- **`GraphicMenu*` source cluster (`GOptions.CPP`)** - medium risk. It may be
  legacy Westwood shell infrastructure reused by RA2/YR. Verify which items are
  live in standard YR and which are leftover constructors.
- **`Title.PCX` and `Intro` in `GraphicMenu__Constructor`** - medium risk.
  Names may refer to generic title/intro infrastructure, not necessarily the
  visible main menu button panel.
- **`ShapeButtonClass` constructors** - medium risk. RTTI exists near shell
  strings, but only include behavior if the call chain reaches main menu.
- **`ShellButtonSlideSound`** - medium risk. Binary parses it, but default is
  empty in `rulesmd.ini`; verify whether the trigger is live and silent by
  default, or completely unused.
- **WOL/Internet buttons** - conditional. Main menu may hide/disable buttons
  depending on install/network state. Verify default retail YR menu state.

## 8. Current Rust Implementation Surface

| File | Current Surface | Notes |
|------|-----------------|-------|
| `src/ui/main_menu.rs` | egui-based map/credits/options setup | Functional placeholder; no original main menu graphic button stack |
| `src/ui/game_screen.rs` | `GameScreen::MainMenu` state | Names current menu state generically |
| `src/app.rs` | routes MainMenu to egui or dev Skirmish shell | The dev shell is dialog `0x102`, not the initial main menu |
| `src/ui/skirmish_shell/*` | Skirmish setup layout/state | Adjacent pattern only; do not assume main menu control IDs |
| `src/app_skirmish_shell_render.rs` | Skirmish setup renderer | Adjacent render glue only |
| `src/render/skirmish_shell_chrome.rs` | Skirmish shell asset atlas | Contains broad shell candidates, but evidence is Skirmish-specific |

## 9. Deferred Open Questions

1. What is the RT_DIALOG resource ID and template for the initial main menu?
2. What class/proc backs control `0x71A`, and where are messages `0x4E3` and
   `0x4E4` handled?
3. Are `Ra2ts_s` and `Ra2ts_l` INI sections, MIX asset names, or graphic menu
   layout profiles?
4. Which exact PCX/SHP/PAL files are visible on the initial menu, and which are
   preload-only?
5. Does `GraphicMenu` draw button text dynamically, or are button labels baked
   into art with `STT:MainButton*` used only as tooltips?
6. What are the exact selected-item ids returned to `Main_Game`, and how do they
   map to visible buttons?
7. Is `ShellButtonSlideSound` actually triggered in retail YR when default
   config leaves it empty?
8. Does the menu behave differently for 640x480, 800x600, and centered high-res
   shell modes?

## 10. Execution Strategy

**Multi-phase single investigation.**

Run Phase 1 first (#1-#11) and stop for a checkpoint. If `0x71A` is confirmed as
the graphic menu host, continue into Phase 2 item parsing/rendering. If not,
replace Phase 2 with the actual control proc/message handlers discovered from
the dialog resource and xrefs.

Avoid parallel split unless Phase 1 reveals more than 50 live menu/control
functions. If that happens, split into:

- main menu dialog/control host,
- `GraphicMenu` asset/layout parser,
- menu item animation/sound/input.

## 11. Success Criteria

The executed research document must:

- Prove whether the visible initial menu button stack is `GraphicMenu`, a
  Win32 owner-draw dialog, or another custom control path.
- Include every function from Section 3 or explicitly justify omission.
- Identify the dialog resource/control IDs and custom message meanings.
- List every visible asset and palette with active/inactive status.
- Decode button layout, hit testing, animation timing, and sounds.
- State "Active in YR: Yes/No/Conditional" for every finding.
- Keep the in-game `SidebarClass` and Skirmish dialog `0x102` separate from this
  scope unless a direct call/data path proves shared behavior.

## Sources

- Ghidra scoping sampled:
  - `0x00531CC0`, `0x0052B9B0`, `0x0052D9A0`, `0x004F2140`,
    `0x004F21A0`, `0x004F2300`, `0x004F3140`, `0x004F3460`,
    `0x004F38B0`, `0x004F3C40`, `0x004F4780`, `0x00622650`,
    `0x00623120`, `0x004790B0`
  - string anchors: `Ra2ts_s`, `Ra2ts_l`, `Title.PCX`,
    `STT:MainButton*`, `STT:MainOpt*`, `GUIMainButtonSound`,
    `ShellButtonSlideSound`
- Docs searched:
  - `docs/research/`
  - `docs/`
  - `docs/plans/`
- INI files checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
  - `ini/artmd.ini`
  - `ini/art.ini`
- Related plans:
  - `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
  - `docs/plans/2026-05-17-skirmish-shell-live-active-render-path-plan.md`
  - `docs/plans/2026-05-17-skirmish-shell-background-text-preview-plan.md`
