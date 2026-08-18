# Skirmish Owner-Draw Button Text Color Source - Ghidra Research Report

**Address(es):** `0x00612B70` (`OwnerDraw_Button_00612B70`), `0x00621040`, `0x0060F9A0`, `0x00622B50`, `0x006AE3F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** text color source for standard offline Skirmish owner-draw PCX buttons `Start Game` `0x617`, `Choose/Customize Map` `0x5AA`, and `Back` `0x5C0`; enabled, pressed, and `WS_DISABLED` paint behavior.  
**Non-Scope:** PCX strip geometry, middle tiling phase, SHP/SDBTNANM button variants except as negative evidence, non-button controls, final retail screenshot pixel sampling after display conversion.  
**Confidence:** High for source color argument and disabled dimming order; Medium for exact final on-monitor disabled appearance without runtime pixel capture.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`.

## Working Notes

- **Target question:** Which packed color does `OwnerDraw_Button_00612B70` pass to the text renderer for Skirmish PCX owner-draw buttons, and does pressed/disabled state change that source?
- **Non-goals:** do not rediscover button PCX art, snap rects, or geometry except where a branch proves the text-color path.
- **Evidence needed to mark COMPLETE:** decompile of `OwnerDraw_Button_00612B70`, assembly around the text call, decompile of `FUN_00621040` color-byte interpretation, decompile/caller evidence that standard Skirmish buttons route to this proc, and a Rust surface scan.
- **Stop conditions:** once all source-color, pressed-state, disabled-state, argument-order, and live-route questions are resolved or explicitly deferred.

## 1. Overview

Standard Skirmish `Start Game`, `Choose/Customize Map`, and `Back` are live Win32 `Button` controls subclassed to `OwnerDraw_Button_00612B70`. For the default PCX button mode (`state+0xB0 == 0` / `piVar17[0x2C] == 0`), enabled and pressed labels pass `DAT_00AC18A4`, initialized to `0x0000FFFF`, to `FUN_00621040`. The dark packed color `0x00000C05` is not the enabled/pressed source for these labels.

Disabled PCX buttons do not switch the text-call color argument to `0x00000C05` or `DAT_00AC1CB4` in this path. The PCX path forces released art, draws the label with the same yellow source argument, then applies a half-alpha surface blend over the whole button surface.

## 2. Key Offsets / Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00AC18A4` | normal shell text source color, initialized to `0x0000FFFF` | `FUN_0060F9A0` decompile sets `DAT_00ac18a4 = 0xffff`; `0x00612DA9` loads it into `EDI` | Yes |
| `state+0x14` / `piVar17[5]` | alternate image pointer; zero selects formatted PCX strip path | `OwnerDraw_Button_00612B70` branch before PCX filename formatting | Yes for scoped buttons |
| `state+0x28` / `piVar17[10]` | text pointer/string state; nonzero gates label draw | `0x00613568..0x00613578` test before text call | Yes |
| `state+0xB0` / `piVar17[0x2C]` | button visual type; zero is PCX owner-draw button path | `OwnerDraw_Button_00612B70` `iVar14 == 0` branch | Yes for scoped buttons |
| `GWL_STYLE & 0x08000000` | `WS_DISABLED`; PCX path forces released art and later alpha-dims | `0x00613254..0x00613262`, `0x006135FD..0x0061361B` | Conditional |

## 3. Core Logic

### Live routing

`FUN_00622B50` handles dialog initialization and, when initialization is not the first pass, calls `EnumChildWindows(param_1, FUN_0060F9A0, 0)` and then `FUN_0060F9A0(param_1, 0)`. `FUN_0060F9A0` checks the class name and style. For class `Button`, if `(style_byte & 0x0B) == 0x0B`, assembly at `0x0060FE78..0x0060FE8B` assigns `OwnerDraw_Button_00612B70` (`0x00612B70`) as the control callback. `FUN_006AE3F0` is the Skirmish dialog proc and delegates common owner-draw/init handling to `FUN_00622B50` before Skirmish-specific `WM_PAINT`, `WM_COMMAND`, and status handling.

Active in YR: Yes. The audited Skirmish traces identify dialog `0x102` and the scoped button IDs; the binary route above is the generic shell subclass path those children use.

### Source color and text-call argument order

At paint entry, `OwnerDraw_Button_00612B70` loads the default text color:

- decompile: `piVar20 = DAT_00ac18a4`
- assembly: `0x00612DA9 MOV EDI,dword ptr [0x00AC18A4]`, then `0x00612DBC MOV dword ptr [ESP + 0x28],EDI`

After the PCX art path, label drawing is gated by `state+0x14 == 0` and `state+0x28 != 0`. The text rect is built, then the wrapper call is:

- assembly `0x006135D4..0x006135EE`: pushes trailing zeros, `0x0C`, `0x05`, `EDI`, text pointer, rect pointer, then calls `0x00621040`
- decompile: `FUN_00621040(&local_f0, piVar17[0x19], piVar20, 5, 0xc, 0, 0, 0)`

Because `FUN_00621040` is `__fastcall`, the first two decompiler parameters are register arguments. The material color argument is the pushed `EDI` / decompiler `param_3`; the `0x05` is the text flags argument, and nearby `0x0C` is a different wrapper argument, not the color.

Active in YR: Yes. This is the live `WM_PAINT` text call for the scoped PCX button path.

### Color byte interpretation

`FUN_00621040` interprets the caller color by source bytes before display packing:

- low byte -> red component, shifted by `g_DD_RLoss/RShift`
- next byte -> green component, shifted by `g_DD_GLoss/GShift`
- third byte -> blue component, shifted by `g_DD_BLoss/BShift`

Therefore `DAT_00AC18A4 = 0x0000FFFF` is source RGB `(255,255,0)` before 16-bit display-format conversion. `0x00000C05` would be RGB `(5,12,0)`, but it is not the enabled/pressed source for these PCX button labels.

Active in YR: Yes. `FUN_00621040` is the text wrapper called by the button paint path.

### Pressed state

Pressed state is read from the previous/default button state bit in `pWStack_d8 & 1`. It changes PCX art family from `u` to `d` and shifts the text rect left/top (`+2/+5`) while preserving the right/bottom edge setup, but it does not change `EDI`/`piVar20` before the text call. The same `DAT_00AC18A4` source color reaches `FUN_00621040`.

Active in YR: Yes. Pressed state is active during normal mouse-down paint on these controls.

### Disabled state

For PCX buttons (`state+0xB0 == 0`), `WS_DISABLED` is tested at `0x00613254`. If set, the code forces the art state byte back to `'u'` at `0x0061325E` and skips the pressed-click transition. The label is still drawn through the same `0x006135D4..0x006135EE` text call. After text drawing, `0x006135F3..0x0061361B` checks `state+0xB0 == 0` and `WS_DISABLED`, then calls `AlphaBlendRect(..., 0x80)` over the composed button surface.

Active in YR: Conditional. It applies only when a scoped button is actually disabled by the dialog state; standard enabled first paint does not take it.

## 4. INI Keys

No INI key controls the text color source for these button labels in this slice. The source color is initialized by the shell owner-draw setup globals, not read from `rules*.ini` or `art*.ini`.

## 5. Integration Points

| Function | Role | Evidence | Active in target |
|---|---|---|---|
| `FUN_006AE3F0` | Skirmish dialog proc; delegates common shell owner-draw messages to `FUN_00622B50` | decompile shows first call `FUN_00622B50(param_3,param_4)` | Yes |
| `FUN_00622B50` | common dialog init/paint handler; calls child enumeration/subclass setup | decompile `WM_INITDIALOG` path calls `EnumChildWindows(...FUN_0060F9A0...)` | Yes |
| `FUN_0060F9A0` | subclasses shell children and initializes text-color globals | decompile and assembly `0x0060FE78..0x0060FE8B` | Yes |
| `OwnerDraw_Button_00612B70` | button WndProc; paints PCX art and calls text wrapper | decompile and assembly `0x006135D4..0x006135EE` | Yes |
| `FUN_00621040` | shell text wrapper; converts source RGB bytes and draws text | decompile `0x00621040` | Yes |

## 6. Current Rust Implementation Status

Current Rust is mismatched for button label color:

- `src/app_skirmish_shell_render.rs:67` defines `SHELL_BUTTON_TEXT_RGB_00000C05 = [5,12,0]/255`.
- `src/app_skirmish_shell_render/text.rs:151` currently uses that dark constant in `push_button_label_draw`.
- `src/app_skirmish_shell_render.rs:1326` has a test asserting button labels should not use generic yellow; this assertion conflicts with the binary evidence for PCX Skirmish buttons.

Rust should use the yellow shell source color for enabled/pressed PCX button labels. Disabled PCX button rendering should model the binary as released art + yellow text followed by a whole-button dim/blend, not as a separate dark text color argument.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0060F9A0` global color init | verified | decompile sets `DAT_00AC18A4 = 0xFFFF` | none |
| Button subclass route | verified | `0x0060FE78..0x0060FE8B` maps `(style & 0x0B)==0x0B` to `0x00612B70` | scoped button resource style not re-decompiled here; covered by prior traces |
| Skirmish proc to common setup | verified | `FUN_006AE3F0` first calls `FUN_00622B50`; `FUN_00622B50` init enumerates `FUN_0060F9A0` | none for this color slice |
| PCX button enabled/pressed color | verified | `0x00612DA9`, `0x006135D4..0x006135EE` | none |
| `FUN_00621040` byte order | verified | decompile `0x00621040` | final screenshot sampling deferred |
| PCX disabled color/dim order | verified | `0x00613254..0x00613262`, `0x006135F3..0x0061361B` | exact final display pixel value deferred |
| SHP button variant disabled color branch | deferred | `0x00612F5F..0x00613138` touched | out-of-scope; not the scoped Skirmish PCX buttons |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the target mode? -> exhaustive-slice for PCX Skirmish button text color only.` (evidence: user slot scope)
- `[RESOLVED] OQ-02 - Are the scoped Skirmish buttons on a live YR path? -> Yes, dialog `0x102` uses `FUN_006AE3F0`, which delegates common setup to `FUN_00622B50`; setup enumerates children through `FUN_0060F9A0`.` (evidence: decompile `0x006AE3F0`, `0x00622B50`)
- `[RESOLVED] OQ-03 - Which subclass proc handles low-style shell buttons? -> `OwnerDraw_Button_00612B70` when `(style & 0x0B) == 0x0B`.` (evidence: `0x0060FE78..0x0060FE8B`)
- `[RESOLVED] OQ-04 - What initializes the normal text color global? -> `FUN_0060F9A0` sets `DAT_00AC18A4 = 0xFFFF`.` (evidence: decompile `0x0060F9A0`)
- `[RESOLVED] OQ-05 - What color is passed for enabled PCX button labels? -> `DAT_00AC18A4`, not `0x00000C05`.` (evidence: `0x00612DA9`, `0x006135E1`, call `0x006135EE`)
- `[RESOLVED] OQ-06 - Does pressed state change the text color argument? -> No; it changes art state/rect but the same `EDI` color reaches the call.` (evidence: `0x0061323C..0x00613295`, `0x006135D4..0x006135EE`)
- `[RESOLVED] OQ-07 - How does the text wrapper decode packed colors? -> low byte red, next green, third blue; `0xFFFF` is RGB `(255,255,0)`.` (evidence: decompile `0x00621040`)
- `[RESOLVED] OQ-08 - Does disabled PCX state switch to dark text? -> No; it forces released art and applies `AlphaBlendRect(...,0x80)` after the text call.` (evidence: `0x00613254..0x00613262`, `0x006135F3..0x0061361B`)
- `[RESOLVED] OQ-09 - Is `0x00000C05` active for these enabled button labels? -> No for the scoped PCX buttons.` (evidence: text-call color source above)
- `[DEFERRED] OQ-10 - What exact final RGB appears on-screen for disabled buttons after alpha blend?` (category: `needs-runtime-debugger`; reason: source/dim order is verified but final display pixel sampling was not captured; next-step-if-pursued: retail screenshot/pixel capture of disabled Start button)
- `[DEFERRED] OQ-11 - What color branches apply to non-PCX SHP button variants?` (category: `out-of-scope`; reason: user scope is Skirmish PCX buttons; next-step-if-pursued: separate main-menu/SDBTNANM button color report)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Button_00612B70` PCX path | `state+0xB0 == 0`; art state `'u'` or `'d'`; disabled forces `'u'` | `bue_*30.pcx` or `bde_*30.pcx` | existing button client rect | PCX surface path | Yes | button chrome |
| 2 | `FUN_00621040` text call `0x006135EE` | `state+0x14 == 0 && state+0x28 != 0` | text only | rect built at `0x00613591..0x006135CD` | `DAT_00AC18A4 -> 0xFFFF -> RGB(255,255,0)` source then display packing | Yes | label |
| 3 | `AlphaBlendRect` `0x0061361B` | `state+0xB0 == 0 && WS_DISABLED` | composed button surface | whole composed button rect | alpha `0x80` | Conditional | disabled dim overlay |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `bue_*30.pcx` | Yes | Yes | enabled/released and disabled forced-released | No | Yes | No | No | No | PCX path decompile |
| `bde_*30.pcx` | Yes | Yes | pressed only, not disabled | No | Yes | No | No | Conditional | PCX path and disabled force-to-`u` |
| Text label | N/A | Yes | Yes | No | No | No | No | No | `0x006135D4..0x006135EE` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Enabled PCX Skirmish button labels use `DAT_00AC18A4 = 0xFFFF` / yellow source RGB | `0x00612DA9`, `0x006135E1`, `0x006135EE`, `FUN_00621040` | mismatch: Rust uses `SHELL_BUTTON_TEXT_RGB_00000C05` | `src/app_skirmish_shell_render/text.rs::push_button_label_draw` | render Start/Customize/Back labels with `SHELL_LABEL_TEXT_RGB`/yellow source | First Skirmish paint: Start/Customize/Back labels are yellow, not dark olive | Do not use `0x00000C05` for enabled PCX button labels; proposed test `skirmish_button_label_color_uses_owner_draw_yellow_source` |
| Pressed PCX button labels keep the same color source | same call after pressed rect/art setup | mismatch if Rust dark color remains | same | pressing Start changes art/rect, not label color source | Mouse-down Start: text remains yellow while position changes | Do not add a pressed-only color branch; proposed test `skirmish_pressed_button_label_keeps_yellow_source` |
| Disabled PCX buttons draw released art + yellow text, then alpha-dim whole button | `0x00613254..0x00613262`, `0x006135F3..0x0061361B` | unchecked/likely partial | skirmish button disabled rendering path | model disabled as post-compose dim, not a dark text argument | Disable Start in validation/error state: button appears dimmed as a whole, text not independently recolored before dim | Do not replace disabled label color with `0x00000C05` or static disabled `0x9F`; proposed test `skirmish_disabled_button_dims_composed_pcx_surface` |

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace the button-current-status wording that says button labels use `SHELL_BUTTON_TEXT_RGB_00000C05` with: "For standard Skirmish PCX owner-draw buttons (`Start`, `Choose/Customize`, `Back`), enabled and pressed labels use the normal shell text source `DAT_00AC18A4 = 0x0000FFFF` / yellow. The dark `0x00000C05` source belongs to other control/value paths, not these enabled PCX button labels."
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`: replace "button and trackbar value text" with: "`0x00000C05` is verified for trackbar value text and other dark value paths; do not generalize it to standard Skirmish PCX button labels."
- `docs/research/skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`: replace "Button text in `0x00612B70` passes color `0x00000C05`" with: "The standard PCX Skirmish button text call in `OwnerDraw_Button_00612B70` passes `DAT_00AC18A4 = 0x0000FFFF`; a separate non-PCX/other-control dark-color path must not be conflated with it."

## Sources

- Ghidra read-only decompile: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_00622B50 @ 0x00622B50`, `FUN_006AE3F0 @ 0x006AE3F0`.
- Ghidra read-only assembly context: `0x0060FE78..0x0060FE8B`, `0x00612DA9..0x00612DBC`, `0x00613254..0x00613262`, `0x006135D4..0x006135EE`, `0x006135F3..0x0061361B`.
- Prior docs consulted for conflict/staleness: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/text.rs`.
