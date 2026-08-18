# Validation Modal OK Button State Frames - Ghidra Research Report

**Address(es):** `0x00612B70`, `0x005D36A0`, prior activation evidence `0x00609E20`, `0x0060A330`, `0x0072B050`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** frame/state selection for ordinary Start-validation dialog `0xCE` OK button control `0x5AE` after it has been classified as owner-draw button type `3` using `MNBTTN.SHP` and `MAINBTTN.PAL`.  
**Non-Scope:** dialog background, all modal dialog inventory, full parent paint path, validation trigger text, optional `0x120/0x121` dialog behavior except where needed for negative comparison, and runtime screenshot capture.  
**Confidence:** High for type-3 frame selection, `WS_DISABLED` separation, timer-highlight byte behavior, text offset condition, and current Rust delta; Medium for whether ordinary `0xCE` ever visibly enters the timer-highlight frame because no runtime message trace proved a `0x4DC` sender for `0x5AE`.  
**Active in YR:** Yes for the ordinary OK button paint path and normal/down frames; Conditional for timer/default-highlight frame `2`.

## 0. Working Notes

- **Target question:** For ordinary Start-validation dialog `0xCE`, control `0x5AE`, which `MNBTTN.SHP` frame does native YR draw for normal, down/pressed, disabled, default/focus, and timer-highlight states?
- **Non-goals:** Do not re-investigate `PUDLGBG*` backgrounds, final dialog pixel rects, all other dialog buttons, or keyboard dismissal except as it affects default/focus claims.
- **Evidence needed to mark COMPLETE:** decompile of `OwnerDraw_Button_00612B70`, address-bounded disassembly for the type-3 frame branch, `WS_DISABLED` style gate, `CC_Draw_Shape` argument order, text-rect offset branch, timer/custom-message writers, resource/default-button facts for `0xCE`, current Rust scan, and Rust-facing handoff.
- **Stop conditions:** Stop when frame `0/1/2`, disabled-style behavior, timer/default-highlight behavior, and OK-label text offset are classified with resolved or explicitly deferred questions.

## 1. Overview

The native type-3 OK button does not map "pressed" to frame `2`. In `OwnerDraw_Button_00612B70`, owner-draw type `3` sets the shape to `MNBTTN.SHP`, the palette/convert resource to `MAINBTTN.PAL`, and selects the frame as:

- frame `0`: normal, when button-state low bit is clear and the timer-highlight byte is clear;
- frame `1`: button-state low bit set, the same state bit that other owner-draw button paths use as down/pressed;
- frame `2`: timer/default-highlight byte `+0xC5` set while the button-state low bit is clear.

`WS_DISABLED` is a separate style branch after frame selection. It changes the text color source for nonzero owner-draw types, but the type-3 branch does not use `WS_DISABLED` to select frame `1`, and the final `AlphaBlendRect(..., 0x80)` disabled overlay is gated to owner-draw type `0`, not type `3`.

## 2. Key Offsets / State Fields

| Field / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| state `+0xB0` / decompiler `piVar17[0x2C]` | owner-draw visual mode; value `3` enters `MNBTTN` type-3 path | prior `0x0060A330` report; branch in `0x00612B70` | Yes for `0xCE/0x5AE` |
| state `+0xE8` / decompiler `piVar17[0x3A]`; stack byte `[esp+0x2C]` in disassembly | button-state word; bit `0` selects down/pressed art in default button paths and frame `1` in type `3` | `0x00612F2A..0x00612F4A`; text offset `0x006135B2..0x006135CD`; default PCX reports use same bit for `'d'` art | Yes when the OK button is down/armed |
| state byte `+0xC4` | timer/highlight active flag set/cleared by custom message `0x4DC` | `0x006136ED..0x00613720` | Conditional; requires a sender of `0x4DC` |
| state byte `+0xC5` | timer-highlight byte toggled by `WM_TIMER 0x113`; selects frame `2` when bit `0` is clear | `0x0061363F..0x0061365C`; type-3 read `0x00612F4C..0x00612F56` | Conditional; timer path is live, target activation not proven |
| state byte `+0xBC` | paint/mouse sound suppress flag; if set, `WM_PAINT` validates and returns, and mouse down sound returns early | decompile `0x00612B70`; mouse block `0x0061374B..0x00613776` | Conditional |
| state `+0x14` | nonzero custom image path gate; button text draw later requires this to be zero | text gate `0x00613568..0x00613578` | Yes; ordinary OK uses default type-3 art, not custom image |
| state `+0x28` / state `+0x64` | text pointer gate and text pointer passed to `FUN_00621040` | `0x00613573..0x006135EE` | Yes for OK text copied to `0x5AE` |
| `GWL_STYLE` bit `0x08000000` | Win32 `WS_DISABLED`; invokes disabled text-color computation for type `3` after frame selection | `GetWindowLongA(...,-0x10)` in decompile; disassembly `0x00612F5F..0x00612F67` | Conditional; ordinary OK is normally enabled |

## 3. Core Logic

### 3.1 Type-3 frame selection

Active in YR: Yes for `0xCE/0x5AE`. Evidence: parent reports prove `0x00609E20` accepts parent `0xCE`, child `0x5AE`; `0x0060A330` writes owner-draw type `3`; this function's type-3 branch is entered when state `+0xB0 == 3`.

Verified disassembly:

| Address range | Verified behavior | Active in YR |
|---|---|---|
| `0x00612F20..0x00612F25` | compare owner-draw type with `3`; non-type-3 exits to later block | Yes |
| `0x00612F25..0x00612F34` | call `0x0072B050` and load `DAT_00B0FACC`; prior asset reports map these to `MAINBTTN.PAL` and `MNBTTN.SHP` | Yes |
| `0x00612F36..0x00612F43` | initialize frame register `EAX = 0`; test state low bit from `[esp+0x2C] & 1` | Yes |
| `0x00612F45..0x00612F4A` | if low bit set, set `EAX = 1` and store that as frame | Yes when down/armed |
| `0x00612F4C..0x00612F56` | if low bit clear, read byte `[EBP+0xC5]`; nonzero sets `EAX = 2` | Conditional on timer-highlight |
| `0x00612F5B` | store final frame in stack slot used by later draw call | Yes |

Pseudocode:

```text
frame = 0
if state_e8_bit0:
    frame = 1
else if state_c5_timer_highlight != 0:
    frame = 2
draw MNBTTN.SHP frame through MAINBTTN.PAL
```

### 3.2 Disabled style does not select frame 1

Active in YR: Conditional. The branch is live if `WS_DISABLED` is set on a type-3 button, but ordinary OK `0x5AE` is not disabled in normal Start-validation use.

After frame selection, the function tests the window style:

- `0x00612F5F`: `test dword ptr [esp + 0x58], 0x8000000`
- `0x00612F67`: if clear, skip to the draw setup at `0x00613138`
- `0x00612F6D..0x006130CE`: compute a replacement text color from shell/side color globals and DirectDraw loss/shift fields

There is no write to the frame slot in this `WS_DISABLED` block. The final alpha overlay path at `0x006135F3..0x0061361B` is additionally gated by `state+0xB0 == 0`, so it does not apply to type `3`.

This corrects the older shorthand "disabled frame 1" claim for `MNBTTN`: frame `1` is the state low-bit/down frame, not a direct `WS_DISABLED` frame selected by this function.

### 3.3 Draw argument order

Active in YR: Yes. The selected frame reaches the shape draw call as the frame argument.

The shape branch checks both palette/convert and shape pointers before drawing:

- `0x00613188..0x00613196`: skip if either pointer is null.
- `0x006131A3`: load selected frame from stack into `EDX`.
- `0x006131B5`: push draw flags `0x400`.
- `0x006131A9`: push constant `1000`.
- `0x006131C9`: push selected frame.
- `0x006131CA`: push `MNBTTN.SHP` pointer.
- `0x006131CD`: call `0x004AED70` (`CC_Draw_Shape` in prior docs), with `EDX` holding the palette/convert resource.

The decompile of `0x00612B70` matches this as `CC_Draw_Shape(shape, frame, ..., flags=0x400, ..., 1000, ...)`.

### 3.4 Timer/default-highlight frame 2

Active in YR: Conditional. The code is live in the callback, but this pass did not find proof that ordinary `0xCE/0x5AE` receives the custom `0x4DC` start message in the normal Start-validation modal.

The timer path is:

- `0x006136ED..0x0061370D`: custom message `0x4DC` with `param_4 == 1` sets byte `+0xC4 = 1` and calls `SetTimer(hwnd, 0, 1000, null)`.
- `0x0061363F..0x0061365C`: `WM_TIMER 0x113` toggles byte `+0xC5` and invalidates the button.
- `0x00613715..0x00613743`: custom message `0x4DC` with non-`1` param clears `+0xC4` and `+0xC5`, kills timer `0`, and invalidates.
- `0x00612F4C..0x00612F56`: type-3 frame selection turns nonzero `+0xC5` into frame `2`.

The active `0xCE` resource has no `BS_DEFPUSHBUTTON` style and no visible `IDOK`/`IDCANCEL` controls. Prior keyboard report proves Enter/Escape are handled through `IsDialogMessageA` translated command IDs, not by making `0x5AE` a default pushbutton. Therefore frame `2` should be treated as an optional timer/default-highlight frame until a runtime message trace proves `0x4DC` is sent to `0x5AE`.

### 3.5 Text offset in the same function

Active in YR: Yes for buttons with text, including OK `0x5AE`.

Text drawing is gated by:

- `0x00613568..0x0061356D`: state `+0x14 == 0`
- `0x00613573..0x00613578`: state `+0x28 != 0`

The text rectangle is then built from the button client rect:

| State | Rect construction | Evidence | Active in YR |
|---|---|---|---|
| normal/timer-highlight with low bit clear | `left = x`, `top = y + 1`, `right = x + w - 2`, `bottom = y + h` | `0x00613591..0x006135B9` | Yes |
| state low bit set / frame `1` | same right/bottom, but `left += 2`, `top += 4` after the prior `top + 1`, net `top = y + 5` | `0x006135B2..0x006135CD` | Yes when down/armed |

The call at `0x006135D4..0x006135EE` pushes the text draw arguments. Prior text reports corrected the effective `FUN_00621040` flags to `0x05` (`h-center | v-center`); the adjacent pushed `0x0C` is not the flags slot in the recovered fastcall signature.

Implication: the OK label should offset with frame `1`/down state, not with frame `2` timer-highlight.

## 4. INI Keys

No INI key controls the scoped frame selection. The behavior is hardcoded owner-draw button state logic plus retail SHP/PAL assets.

| INI key | Effect in this slice | Active in YR |
|---|---|---|
| none | `MNBTTN` frame selection, disabled style color path, timer-highlight byte, and text offset are binary-defined callback behavior | Yes |

## 5. Integration Points

| Function / site | Role | Evidence | Active in YR |
|---|---|---|---|
| `0x005D36A0` | modal proc handles OK `0x5AE`, IDOK `1`, IDCANCEL `2`; does not itself control visual frames | decompile in this pass | Yes |
| `0x00609E20` | allow-lists dialog `0xCE`, control `0x5AE` as type-3-capable owner-draw control | prior report | Yes |
| `0x0060A330` | writes owner-draw visual mode `3` for the OK child | prior report | Yes |
| `0x00612B70` | active owner-draw button WndProc/paint logic | decompile and disassembly in this pass | Yes |
| `0x0072B050` | returns `MAINBTTN.PAL` convert/palette resource | prior report plus type-3 call | Yes |
| `0x004AED70` | shape draw call consuming `MNBTTN.SHP` pointer and selected frame | disassembly `0x00613188..0x006131CD`; prior symbol report | Yes |
| `FUN_00621040 @ 0x00621040` | draws OK label after the SHP art | call site `0x006135D4..0x006135EE`; prior text report | Yes |

## 6. Current Rust Implementation Status

Current Rust has implemented the asset load and a two-state helper, but the frame mapping is wrong against this slice:

- `src/render/skirmish_shell_chrome.rs` loads `MAINBTTN.PAL` and `MNBTTN.SHP` frames `0`, `1`, and `2`.
- `src/app_skirmish_shell_render/chrome.rs` defines `modal_button_mnbttn_frame_index(pressed: bool)` as `false -> 0`, `true -> 2`.
- `src/app_skirmish_shell_render/chrome.rs` `modal_button_mnbttn_entry` ignores frame `1`.
- `src/app_skirmish_shell_render/text.rs` draws the validation OK label using `modal.ok_button_pressed` for the pressed text rect.
- `src/app.rs` sets `modal.ok_button_pressed` on mouse down inside the OK rect.

Rust should instead use frame `1` for the mouse-down/armed pressed state and apply the pressed text offset with that same state. Frame `2` should remain available only for a future native-default/timer-highlight state if a runtime trace or broader shell-modal system proves activation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target activation `0xCE/0x5AE -> type 3` | verified-by-prior | `MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md`; `0x00609E20`, `0x0060A330` | none |
| Type-3 asset binding | verified | decompile `0x00612B70`; disassembly `0x00612F25..0x00612F34`; prior `0x0072B050` mapping | none |
| Frame `0/1/2` selection | verified | disassembly `0x00612F36..0x00612F5B` | none |
| `WS_DISABLED` effect for type `3` | verified | disassembly `0x00612F5F..0x006130CE`; final alpha gate `0x006135F3..0x0061361B` | final screenshot of disabled type-3 button not captured |
| Timer/default-highlight byte writers | verified | disassembly `0x0061363F..0x00613743` | sender to ordinary `0xCE/0x5AE` not proven |
| Text rect/offset condition | verified | disassembly `0x00613568..0x006135EE` | none |
| Resource default-pushbutton status | verified-by-prior | keyboard/default report resource parse for `0xCE` | runtime focus rectangle not captured |
| Current Rust frame mapping | verified | source scan of `src/app_skirmish_shell_render/chrome.rs`, `text.rs`, `app.rs` | implementation not performed in this report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this ordinary Start-validation OK button active in YR? -> Yes; `0xCE/0x5AE` is the ordinary one-button Start-validation OK control and is classified as owner-draw type `3`.` (evidence: prior `0x00609E20`, `0x0060A330`, resource `0xCE`)
- `[RESOLVED] OQ-02 - Which asset/palette does type `3` bind? -> `DAT_00B0FACC`/`0x0072B050` map to `MNBTTN.SHP`/`MAINBTTN.PAL`.` (evidence: `0x00612F25..0x00612F34`; prior asset pointer report)
- `[RESOLVED] OQ-03 - What selects frame `0`? -> default state: low bit clear and `+0xC5 == 0`.` (evidence: `0x00612F36..0x00612F5B`)
- `[RESOLVED] OQ-04 - What selects frame `1`? -> state low bit `+0xE8 & 1`, the same bit used for down/pressed art/text offset in owner-draw buttons.` (evidence: `0x00612F38..0x00612F4A`; `0x006135B2..0x006135CD`; prior PCX button reports)
- `[RESOLVED] OQ-05 - What selects frame `2`? -> timer-highlight byte `+0xC5 != 0` while low bit is clear.` (evidence: `0x00612F4C..0x00612F56`; timer toggle `0x0061363F..0x0061365C`)
- `[RESOLVED] OQ-06 - Does `WS_DISABLED` directly select frame `1` for type `3`? -> No; the disabled-style block occurs after frame selection and does not write the frame slot.` (evidence: `0x00612F5F..0x006130CE`)
- `[RESOLVED] OQ-07 - Does type `3` get the type-0 disabled alpha overlay? -> No; final `AlphaBlendRect` is gated by owner-draw type `0`.` (evidence: `0x006135F3..0x0061361B`)
- `[RESOLVED] OQ-08 - Which state offsets the OK label? -> state low bit, matching frame `1`; rect left adds `2`, top adds `4` after the normal `+1`, net top `+5`.` (evidence: `0x00613591..0x006135CD`)
- `[RESOLVED] OQ-09 - Is `0x5AE` a native default pushbutton in the resource? -> No; resource style is owner-draw `0x5000000B` and prior report found no `BS_DEFPUSHBUTTON`.` (evidence: keyboard/default report resource parse)
- `[RESOLVED] OQ-10 - Does `0x005D36A0` itself drive frame selection? -> No; it handles `WM_COMMAND` results only after common shell handling.` (evidence: decompile `0x005D36A0`)
- `[DEFERRED] OQ-11 - Does ordinary `0xCE/0x5AE` receive `0x4DC` and visibly flash frame `2` as a default/focus highlight?` (category: `needs-runtime-debugger`; reason: callback supports the timer path but no sender to this exact control was proven statically; next-step-if-pursued: runtime message trace or xref-focused investigation of `SendMessage/PostMessage 0x4DC`)
- `[DEFERRED] OQ-12 - Exact final RGB/text color for disabled type-3 OK.` (category: `needs-runtime-debugger`; reason: binary identifies disabled color computation, but final DirectDraw mode pixels need runtime capture; next-step-if-pursued: force/observe a disabled type-3 modal button and capture surface pixels)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Button_00612B70`, type dispatch `0x00612F20..0x00612F5B` | `state+0xB0 == 3`; frame from `+0xE8 & 1` / `+0xC5` | `MNBTTN.SHP#0/#1/#2` | button client rect, shape draw target from destination surface setup | `MAINBTTN.PAL` via `0x0072B050` | Yes; frame `2` conditional | OK button chrome |
| 2 | `OwnerDraw_Button_00612B70`, disabled color block `0x00612F5F..0x006130CE` | `GWL_STYLE & 0x08000000` | no frame change | n/a | display-format disabled text color from shell/side globals | Conditional | disabled label color source |
| 3 | `OwnerDraw_Button_00612B70 -> CC_Draw_Shape` `0x00613188..0x006131CD` | non-null shape and convert pointers | selected `MNBTTN.SHP` frame | destination surface clip/bounds from `DAT_00887310 +0x78` | `MAINBTTN.PAL` convert resource | Yes | SHP draw |
| 4 | `OwnerDraw_Button_00612B70 -> FUN_00621040` `0x00613568..0x006135EE` | text pointer exists and custom image pointer is zero | no bitmap | normal rect `x,y+1,x+w-2,y+h`; down rect `x+2,y+5,x+w-2,y+h` | `GAME.FNT`; text color source from normal or disabled branch | Yes | OK label |

### Asset Role Matrix

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `MNBTTN.SHP` frame `0` | Yes | Yes | Yes, normal OK | No | Button chrome | No | No | No | `0x00612F25..0x00612F5B`; asset report |
| `MNBTTN.SHP` frame `1` | Yes | Yes when low bit set | Yes during down/armed OK | No | Button chrome | No | No | No | `0x00612F38..0x00612F4A`; text offset shares low bit |
| `MNBTTN.SHP` frame `2` | Yes | Conditional | Conditional, only timer-highlight/default path | No | Button chrome | No | Yes/conditional | Inactive unless `+0xC5` toggles | `0x00612F4C..0x00612F56`; `0x0061363F..0x00613743` |
| `MAINBTTN.PAL` | Yes | palette input | Yes for OK button | No | Palette | No | No | No | `0x0072B050`; type-3 branch |
| `bue_*30.pcx` / `bde_*30.pcx` | Yes elsewhere | No for `0xCE/0x5AE` | No for target | No | No for target | No | No | Inactive for target | `state+0xB0 == 3` bypasses type-0 PCX path |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Type-3 normal OK uses `MNBTTN.SHP` frame `0` through `MAINBTTN.PAL` | `0x00612F25..0x00612F5B`; prior asset report | none observed for normal frame after recent load work | `src/app_skirmish_shell_render/chrome.rs`, `src/render/skirmish_shell_chrome.rs` | Keep normal/unarmed OK on frame `0` | Open validation modal without pressing OK: OK button uses dark red/native frame `0` | Do not fall back to `push_button_30` for the target |
| Down/pressed/armed OK uses state low-bit frame `1`, and the OK label offsets with that same low bit | `0x00612F38..0x00612F4A`; `0x006135B2..0x006135CD`; prior button state-bit reports | mismatch: current `modal_button_mnbttn_frame_index(true)` returns `2`; frame `1` is loaded but ignored | `src/app_skirmish_shell_render/chrome.rs::modal_button_mnbttn_frame_index`, `modal_button_mnbttn_entry`, `src/app_skirmish_shell_render/text.rs` | Map mouse-down/armed OK to frame `1`; keep pressed text offset tied to the same armed state | Click-hold OK: button switches to frame `1`, label shifts to native down rect; release dismisses | Do not use frame `2` as the ordinary mouse-down frame |
| Frame `2` is timer/default-highlight byte `+0xC5`, not the normal pressed state | `0x00612F4C..0x00612F56`; `0x0061363F..0x00613743`; resource has no default pushbutton style | mismatch/overclaim risk: current Rust treats pressed as frame `2` and has no separate timer-highlight state | future modal/default-focus state if implemented | Keep frame `2` available but do not use it for normal mouse down until `0x4DC` activation is proven for `0x5AE` | If a later runtime trace proves default flashing, OK alternates frame `0/2` while unpressed; mouse down still uses frame `1` | Do not describe frame `2` as "pressed" in contracts |
| `WS_DISABLED` does not itself select frame `1` for type `3`; it only enters disabled text-color computation for this nonzero type | `0x00612F5F..0x006130CE`; final alpha gate `0x006135F3..0x0061361B` | unchecked; current validation OK is not modeled disabled | future generalized modal button renderer | If a disabled type-3 button is ever needed, do not equate disabled with frame `1`; derive disabled text treatment separately | Synthetic disabled type-3 render keeps frame from state bits and changes label color only if modeling disabled color | Do not copy type-0 Start-button disabled alpha behavior onto `MNBTTN` without target evidence |

### Stale Docs / Follow-up Docs

- `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`: replace "enabled/unpressed frame `0`, disabled frame `1`, pressed/timer frame `2`" with "normal frame `0`; button-state low bit/down frame `1`; timer/default-highlight byte `+0xC5` frame `2`; `WS_DISABLED` is a separate style/color path and does not directly select frame `1` for type `3`."
- `MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md`: replace "Map normal/up visual state to frame `0` and pressed/active state to frame `2`" with "Map normal/up to frame `0`, mouse-down/armed pressed state to frame `1`, and reserve frame `2` for the timer/default-highlight byte path unless target activation is proven."

## Sources

- Ghidra read-only decompile: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_005D36A0 @ 0x005D36A0`.
- Local read-only PE disassembly with Capstone from retail `gamemd.exe`: `0x00612F20..0x00612F70`, `0x00612F5F..0x006130CE`, `0x00613188..0x006131CD`, `0x00613568..0x006135EE`, `0x0061363F..0x00613743`.
- Prior reports referenced: `MNBTTN_MAINBTTN_MODAL_BUTTON_ART_GHIDRA_REPORT.md`, `VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`, `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_TEXT_COLOR_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`.
- Current Rust scan: `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render/chrome.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app.rs`.

**Status:** COMPLETE for type-3 frame/state mapping; runtime-only default-highlight activation remains deferred.
