# Validation Modal Text Layout And Wrapping - Ghidra Research Report

**Address(es):** `0x005D3490`, `0x00610CA0`, `0x006153E0`, `0x00612B70`, `0x00621040`, `0x00434CD0`, `0x0060F9A0`, resource `0xCE` at `0x00BF5A3C`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** text write and paint behavior for ordinary dialog `0xCE` controls `0x5B0` body static and `0x5AE` OK button only.  
**Non-Scope:** PUDLGBG theme selection, native screenshot capture, full `FUN_00621040` internals beyond flags/arguments used here, optional resources `0x120/0x121`, and keyboard/default dismissal.  
**Confidence:** High for resource styles/rects, `0x4B2` write path, owner-draw text callers, alignment flags, color source, and current Rust delta; Medium for final display pixels because no retail screenshot/pixel sample was captured.  
**Active in YR:** Yes for standard offline Skirmish Start validation failures that select dialog `0xCE`.

## 0. Working Notes

- Target question: How does native YR lay out, wrap, align, color, clip, and offset body static `0x5B0` and OK button `0x5AE` text in ordinary validation dialog `0xCE`?
- Non-goals: Do not re-investigate which PUDLGBG theme is selected, button SHP frame selection except text draw order, generic optional modal buttons, or parent Start-button underlay timing.
- Evidence needed to mark COMPLETE: resource bytes for class/style/rects, decompile plus assembly for `0x005D3490 -> 0x4B2` writes, `0x00610CA0` text copy, static/button paint calls into `FUN_00621040`, and lower wrapper/delegated wrapping behavior.
- Stop conditions: Stop after `0x5B0` and `0x5AE` text behavior is resolved with no un-deferred open questions for this slice.

## 1. Overview

The ordinary Start validation modal does not use Win32's default static/button text drawing. `0x005D3490` sends caller-provided UTF-16 text to child controls with message `0x4B2`; the common subclass thunk copies the text into the owner-draw record, and the static/button owner procs later draw the record text through `FUN_00621040` / `FUN_00434CD0`.

The player-visible result is asymmetric. Body `0x5B0` is a plain left/top anchored, wrapped/clipped static in the native `40,40,220,50` dialog-unit control area. OK `0x5AE` is centered in the button text inset and shifts to the pressed inset when active. Active in YR: Yes. Evidence: resource bytes at `0x00BF5A3C`, helper writes `0x005D3573..0x005D35A7`, static paint `0x00615A8B..0x00615AE8`, button paint `0x00613591..0x006135EE`.

## 2. Resource Control Facts

| Control | Class | Style | Dialog-unit rect | Initial title | Runtime text source | Active in YR |
|---|---|---:|---|---|---|---|
| `0x5B0` | Static ordinal `0x82` | `0x50000000` | `x=40 y=40 cx=220 cy=50` | `GUI:Blank` | `param_1` body via `0x4B2` | Yes |
| `0x5AE` | Button ordinal `0x80` | `0x5000000B` | `x=207 y=175 cx=83 cy=15` | `GUI:OK` | `param_2` / `TXT_OK` via `0x4B2` | Yes |

Resource byte proof: `read_memory 0x00BF5A3C length 138` decodes the standard `DLGTEMPLATE`: style `40 00 00 40`, child count `02`, dialog `cx=0x012C cy=0x00C8`, font `8pt MS Sans Serif`, then control `0x5B0` static style `00 00 00 50` and control `0x5AE` button style `0B 00 00 50`. Active in YR: Yes, these are retail `gamemd.exe` resource bytes.

The 8pt `MS Sans Serif` resource font is relevant to dialog-unit conversion. The visible text itself is owner-draw shell text through `GAME.FNT`, initialized in `FUN_0060F9A0` (`uStack_99C = g_GAME_FNT`) and drawn by the shell BitFont path. Active in YR: Yes. Evidence: `0x0060F9A0` decompile, `0x00621040`, `0x00434CD0`.

## 3. Text Write Path

| Step | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Body write | If `param_1` is non-null and first UTF-16 code unit is nonzero, helper gets `GetDlgItem(hwnd, 0x5B0)` and sends `SendMessageA(child, 0x4B2, 0, param_1)`. | decompile `0x005D3490`; assembly `0x005D3573..0x005D3588` | Yes |
| OK write | If `param_2` is non-null and first UTF-16 code unit is nonzero, helper gets `GetDlgItem(hwnd, 0x5AE)` and sends `SendMessageA(child, 0x4B2, 0, param_2)`. | decompile `0x005D3490`; assembly `0x005D3592..0x005D35A7` | Yes |
| Text ownership | The subclass thunk compares message `0x4B2`, frees stale `record+0x28` text if needed, allocates `wcslen*2+2`, copies the incoming UTF-16 string, and stores dirty/type state. | assembly `0x00611BC1..0x00611C67` | Yes |
| Owner-proc dispatch | After copying text, the thunk calls the stored owner proc with the original message arguments; static `0x4B2/0x4B4` refreshes cached surface and invalidates. | assembly `0x00612318..0x0061234B`; static decompile `0x006153E0` | Yes |

Tiny detail: the `0x4B2` incoming pointer is not borrowed until paint. The thunk copies the string immediately. Implementations should store owned modal strings and should not depend on the caller buffer lifetime. Active in YR: Yes. Evidence: allocation/copy at `0x00611C47..0x00611C5B`.

## 4. Core Text Layout

`FUN_00621040` treats the caller rectangle as both layout bounds and clip/scissor bounds. It computes width as `right-left` and height as `bottom-top`, sets the BitFont clip rectangle, converts the packed source RGB using DirectDraw loss/shift globals, optionally vertical-centers when flags include `0x04`, then delegates wrapping and per-line horizontal alignment to `FUN_00434CD0`. Active in YR: Yes. Evidence: decompile `0x00621040`; lower core `0x00434CD0`.

`FUN_00434CD0` wraps at spaces when a line exceeds the supplied width, falls back to backing up one glyph when no prior space is available and more than one glyph exists, handles CR/LF as explicit line breaks, and stops once line advance reaches the supplied height. Horizontal line alignment bits are `0x01` center and `0x02` right; absence of both means left anchored. Active in YR: Yes. Evidence: `0x00434CD0` decompile.

## 5. Per-Control Text Contract

| Control | Native text rect source | Alignment / wrap | Color | Pressed offset | Draw order | Active in YR |
|---|---|---|---|---|---|---|
| `0x5B0` body static | Static client rect from resource `40,40,220,50` after runtime DLU conversion | Left anchored; no `SS_CENTER`/`SS_RIGHT` style bits in `0x50000000`; wraps/clips in rect via `FUN_00434CD0`; no caller-side truncation | Normal shell text `DAT_00AC18A4 = 0x0000FFFF`; disabled style would use `DAT_00AC1CB4 = 0x9F`, but this modal body is not disabled in ordinary Start validation | None | After parent background, as child/static paint | Yes |
| `0x5AE` OK label | Button text inset built at `0x00613591..0x006135CD`: released `left=button_left`, `top=button_top+1`, `right=button_left+width-2`, `bottom=button_top+height`; pressed shifts `left+=2`, `top+=5` with right/bottom retained | Flags `0x05` (`h-center | v-center`); wraps/clips if text exceeds inset; no caller-side truncation | Normal shell text `DAT_00AC18A4 = 0x0000FFFF`; disabled path can replace color before draw | Yes, `left+2/top+5` | After `MNBTTN.SHP` button art, before any disabled post-treatment | Yes |

Evidence for body: resource style `0x50000000`; `OwnerDraw_Static_006153E0` style-to-flags branch reads `GetWindowLongA(hwnd, GWL_STYLE)` and chooses left/center/right mode from low bits before calling `FUN_00621040`; style low bits are zero for `0x5B0`. Active in YR: Yes. Evidence: resource bytes `0x00BF5A3C`, static decompile `0x006153E0`, call context `0x00615A8B..0x00615AE8`.

Evidence for OK label: `OwnerDraw_Button_00612B70` calls `FUN_00621040` at `0x006135EE`; the call-site contract is the same button label contract already verified for owner-draw shell buttons, with flags `0x05` and pressed text inset. Active in YR: Yes. Evidence: assembly context `0x00613591..0x006135EE`; `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`.

## 6. INI Keys

No INI key controls the text layout, wrapping, alignment, font, or colors for `0xCE/0x5B0/0x5AE`. The inputs are Win32 resource style/rects, shell owner-draw setup globals, and caller-provided CSF/localized strings. Active in YR: Yes. Evidence: local `rules*.ini`/`art*.ini` scan found GUI sounds and general colors, but no dialog text-layout keys; binary uses resource/style globals instead.

## 7. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `0x005D3490` | Creates ordinary `0xCE` modal and sends `0x4B2` body/OK text. | decompile; assembly `0x005D3573..0x005D35A7` | Yes |
| `0x0060F9A0` | Installs subclass thunk, owner proc, text record, `GAME.FNT`, and shell colors. | decompile `0x0060F9A0` | Yes |
| `0x00610CA0` | Common subclass thunk; owns `0x4B2` text copy. | assembly `0x00611BC1..0x0061234B`; no function boundary mutation made | Yes |
| `OwnerDraw_Static_006153E0` | Paints body static text using style-derived alignment and wrapper. | decompile `0x006153E0`; call `0x00615AE8` | Yes |
| `OwnerDraw_Button_00612B70` | Paints OK button label after art using button text inset/pressed shift. | decompile `0x00612B70`; call `0x006135EE` | Yes |
| `FUN_00621040` / `FUN_00434CD0` | Shared text clipping, vertical center, wrapping, and per-line h-align. | decompile `0x00621040`, `0x00434CD0` | Yes |

## 8. Current Rust Implementation Status

Current Rust already uses resource-derived validation layout rects in `src/ui/skirmish_shell/layout.rs:849..859`: body uses `dlu_rect(40, 40, 220, 50)` and OK uses `dlu_rect(207, 175, 83, 15)`.

Current Rust text draw at `src/app_skirmish_shell_render/text.rs:734..751` centers the body with `ShellAlign::H_CENTER | ShellAlign::V_CENTER` and centers the OK label with the shared `button_text_rect`. The OK label is close to the native button contract; the body static is mismatched because native `0x5B0` is left/top anchored wrapped text, not centered both ways.

Current Rust `src/render/shell_text.rs:57..112` has an appropriate wrapper model for rect=scissor, wrapping, h-center/right, and v-center. It needs the correct flags per control, not a new text engine for this slice.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Resource `0xCE` body/button styles and rects | verified | `read_memory 0x00BF5A3C length 138` | final runtime pixel capture belongs to slot 1 |
| `0x005D3490` body/OK `0x4B2` writes | verified | decompile `0x005D3490`; assembly `0x005D3573..0x005D35A7` | none |
| `0x00610CA0` `0x4B2` text copy | verified | assembly `0x00611BC1..0x00611C67` | full non-text thunk behavior out of scope |
| Static `0x5B0` paint | verified | `0x006153E0`, call context `0x00615A8B..0x00615AE8` | screenshot pixel sample deferred |
| OK `0x5AE` label paint | verified | `0x00612B70`, call context `0x00613591..0x006135EE` | screenshot pixel sample deferred |
| `FUN_00621040` flags used by this slice | verified | `0x00621040`, `0x00434CD0`, existing text contract report | broader text engine internals out of scope |
| Optional `0x120/0x121` text layout | deferred | explicit non-scope | separate optional modal report if needed |
| Current Rust delta | verified | `text.rs`, `layout.rs`, `shell_text.rs` scan | implementation patch by parent, not this report |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this text path active in standard YR Start validation? -> Yes, ordinary Start validation selects `0xCE`, writes body/OK through `0x4B2`, and subclasses both controls through the common shell owner-draw setup.` (evidence: `0x005D3490`, resource `0x00BF5A3C`, `0x0060F9A0`)
- `[RESOLVED] OQ-02 - Does `0x5B0` use default Win32 static drawing? -> No; it is subclassed to `OwnerDraw_Static_006153E0` and later drawn through `FUN_00621040`.` (evidence: `0x0060F9A0`, `0x006153E0`)
- `[RESOLVED] OQ-03 - Is body `0x5B0` centered? -> No for ordinary resource `0xCE`; style `0x50000000` has no center/right static bits, so the static call uses the left-anchored mode.` (evidence: resource bytes `0x00BF5A3C`, static style branch `0x00615A8B..0x00615AE8`)
- `[RESOLVED] OQ-04 - Does body text wrap or truncate before drawing? -> No caller-side truncation; the lower BitFont core wraps within supplied width and clips/stops by height.` (evidence: `0x00621040`, `0x00434CD0`)
- `[RESOLVED] OQ-05 - What font is visible? -> Owner-draw shell `GAME.FNT`, not the resource's `MS Sans Serif`; the resource font is for DLU conversion.` (evidence: `0x0060F9A0`, `0x00621040`)
- `[RESOLVED] OQ-06 - Does OK `0x5AE` text draw before or after MNBTTN art? -> After button art and before disabled post-treatment.` (evidence: `0x00612B70`, `0x00613568..0x0061361B`)
- `[RESOLVED] OQ-07 - Does OK label shift when pressed? -> Yes; the shared button text rect shifts `left += 2`, `top += 5`.` (evidence: `0x00613591..0x006135EE`; text contract report)
- `[RESOLVED] OQ-08 - What color is used for enabled body and OK labels? -> `DAT_00AC18A4 = 0x0000FFFF`, decoded as shell yellow source RGB; disabled branch uses `DAT_00AC1CB4` only if the control has disabled style.` (evidence: `0x0060F9A0`, `0x006153E0`, `0x00612B70`, `0x00621040`)
- `[RESOLVED] OQ-09 - Does `0x4B2` borrow caller string memory? -> No; thunk allocates and copies UTF-16 text into the owner record.` (evidence: `0x00611C47..0x00611C5B`)
- `[RESOLVED] OQ-10 - Does current Rust already match body alignment? -> No; Rust centers body text both horizontally and vertically.` (evidence: `src/app_skirmish_shell_render/text.rs:734..742`)
- `[DEFERRED] OQ-11 - Exact retail RGB/subpixel screenshot for long body messages` (category: `needs-runtime-debugger`; reason: binary gives layout/wrap/clip behavior, but no native screenshot was captured in this slot; next-step-if-pursued: capture no-opponent/capacity modal and pixel-compare body text origin)
- `[DEFERRED] OQ-12 - Optional resource `0x120/0x121` extra-button text behavior` (category: `out-of-scope`; reason: slot target is ordinary dialog `0xCE`; next-step-if-pursued: run a separate optional-helper-modal text report)

## 11. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | parent mode-2 paint | Slot 2 proved `0xCE` parent mode `2` | `PUDLGBG*` frame 0 | dialog client | DIALOG-family palette | Yes | modal background; text drawn after this |
| 2 | `OwnerDraw_Static_006153E0` | control `0x5B0`; style `0x50000000` low bits zero | text only | static client rect from DLU `40,40,220,50`; left/top anchored, wrapped/clipped | `GAME.FNT`, `DAT_00AC18A4` | Yes | body/message text |
| 3 | `OwnerDraw_Button_00612B70` | control `0x5AE`; owner-draw type 3 | `MNBTTN.SHP` frame by state | button client rect from DLU `207,175,83,15` | `MAINBTTN.PAL` | Yes | OK button chrome |
| 4 | `OwnerDraw_Button_00612B70 -> FUN_00621040` | text pointer exists; button label flags `0x05` | text only | released inset `top+1/right-2`; pressed inset `left+2/top+5`; centered both ways | `GAME.FNT`, `DAT_00AC18A4` unless disabled branch | Yes | OK label |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `GAME.FNT` | Yes | Yes | Yes | Text glyphs | No | No | No | No | `0x0060F9A0`, `0x00621040` |
| `PUDLGBG*.SHP` | Yes | Yes | Yes | No | modal background | No | No | No | slot-2 paint report |
| `MNBTTN.SHP` | Yes | Yes | Yes | No | OK button chrome | No | No | No | `0x00612B70`, MNBTTN report |
| `MAINBTTN.PAL` | Yes | palette input | Yes | No | button palette | No | No | No | `0x0072B050`, MNBTTN report |
| `MS Sans Serif` resource font | OS/runtime | Not owner-draw visible | No direct glyph role in target | No | DLU conversion metadata | No | No | Not visible as text glyphs | resource `0xCE`, owner-draw path |

## 12. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Body static `0x5B0` is left/top anchored wrapped text in the resource static rect, not centered. | resource `0x00BF5A3C`; `0x006153E0`; `0x00621040`; `0x00434CD0` | mismatch: Rust uses `ShellAlign::H_CENTER | ShellAlign::V_CENTER` for modal body | `src/app_skirmish_shell_render/text.rs::push_validation_modal_text_draws`; `src/render/shell_text.rs` already supports the needed flags | Draw validation body with no horizontal/vertical centering; rely on wrapper wrap/clip inside `layout.message` | Capacity/no-opponent failure body starts at the top-left of the native `0x5B0` rect and wraps/clips there | Do not tune by eye with centered paragraph layout; native style is plain static low bits zero |
| OK label `0x5AE` uses shared owner-draw button text inset and flags `0x05`, including pressed `left+2/top+5`. | `0x00613591..0x006135EE`; text contract report | mostly matched: Rust uses `button_text_rect` and centered flags | `src/app_skirmish_shell_render/text.rs::button_text_rect` and validation OK call | Preserve centered OK label and pressed inset while using MNBTTN art | Mouse-down OK shifts label right/down with pressed MNBTTN frame, then release dismisses | Do not apply body-left alignment to button labels |
| Both body and OK text arrive via `0x4B2` and are copied into owner records before paint. | `0x005D3573..0x005D35A7`; `0x00611BC1..0x00611C67` | Rust already owns modal strings in state | `src/ui/skirmish_shell/state.rs`, app modal construction | Keep owned `message`/`ok_button` strings; no borrowed pointer semantics needed | Modal remains stable if state refreshes while displayed | Do not model `0x4B2` as a transient pointer into CSF buffers |
| Enabled text source color is shell yellow `DAT_00AC18A4 = 0x0000FFFF`; disabled color is conditional only on disabled style. | `0x0060F9A0`, `0x006153E0`, `0x00612B70`, `0x00621040` | currently matched for validation text via `SHELL_LABEL_TEXT_RGB` | `src/app_skirmish_shell_render/text.rs` constants | Keep enabled validation body and OK label yellow unless a disabled-style path is explicitly implemented | Normal validation modal shows yellow body/OK text | Do not use dark value text or arbitrary modal text color for this path |

## Stale Docs / Follow-up Docs

- `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md` should replace "body text through common owner-draw text path" with: "body static `0x5B0` uses common owner-draw text path with resource style `0x50000000`, so it is left/top anchored wrapped text in the static rect, not centered."
- Any implementation note saying current Rust text layout is parity after DLU rect adoption should be corrected: the rect is now resource-derived, but body alignment is still wrong until centered flags are removed for `0x5B0`.

## Sources

- Ghidra read-only decompile: `0x005D3490`, `0x006153E0`, `0x00612B70`, `0x00621040`, `0x00434CD0`, `0x0060F9A0`, `0x0060A330`, `0x00609E20`, `0x00602490`, `0x0060A5B0`.
- Ghidra read-only assembly/context: `0x005D3573..0x005D35A7`, `0x00611BC1..0x00611C67`, `0x00612318..0x0061234B`, `0x00615A8B..0x00615AE8`, `0x00613591..0x006135EE`, `0x006135F3..0x0061361B`.
- Resource bytes: `read_memory 0x00BF5A3C length 138` from retail `gamemd.exe` loaded image.
- Prior reports referenced: `VALIDATION_MODAL_0X005D3490_DIALOG_TEMPLATE_CONTROL_RECTS_GHIDRA_REPORT.md`, `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`, `MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/app_skirmish_shell_render/text.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/render/shell_text.rs`.

**Status:** COMPLETE for ordinary dialog `0xCE` text layout/wrapping slice; screenshot RGB remains deferred.
