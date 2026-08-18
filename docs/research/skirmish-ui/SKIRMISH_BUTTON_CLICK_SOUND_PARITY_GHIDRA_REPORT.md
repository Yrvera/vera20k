# Skirmish Button Click Sound Parity - Ghidra Research Report

**Address(es):** `0x00612B70`, `0x006AE3F0`, `0x006ACEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** dialog `0x102` owner-draw button click/sound timing for `0x617` Start Game, `0x5AA` Choose Map, and `0x5C0` Back; command handoff into `FUN_006ACEE0`; active-YR status.  
**Non-Scope:** Choose Map modal internals, Start Game validation semantics beyond command gating, wider shell button families, and slide-in animation sound use outside this click path.  
**Confidence:** High for the two sound call sites, source RulesClass fields, message timing, and command handoff; Medium for current Rust parity status because this report only scanned the relevant shell mouse paths.  
**Active in YR:** Yes. `FUN_006AE2C0` creates dialog resource `0x102` with dialog proc `0x006AE3F0` at `0x006AE31C..0x006AE328`, pumps until result `0x617` or `0x5C0`, and this is the standard Skirmish setup path.

## 1. Overview

The three Skirmish buttons are normal Win32 `Button` controls subclassed into `OwnerDraw_Button_00612B70`. A click can produce sound before `FUN_006ACEE0` runs: `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK` plays `GUIMainButtonSound` immediately, and the first paint that observes an up-to-down state transition plays `GenericClick`.

`FUN_006ACEE0` is not the owner-draw sound source. It receives the later parent `WM_COMMAND` dispatch from `FUN_006AE3F0` and performs the Start Game, Choose Map, or Back action.

## 2. Key Offsets / Fields

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `RulesClass + 0x188` | `[AudioVisual] GUIMainButtonSound`, default `MenuClick` | `0x00613759..0x00613771`; `RULESCLASS_FIELDS.csv`; `ini/rulesmd.ini:643` | Yes: loaded directly on enabled owner-draw button mouse down/double-click |
| `RulesClass + 0x70C` | `[AudioVisual] GenericClick`, default `MenuClick` | `0x00613273..0x00613289`; `RULESCLASS_FIELDS.csv`; `ini/rulesmd.ini:703` | Yes: loaded directly on first `WM_PAINT` seeing button state `'u' -> 'd'` |
| `RulesClass + 0x750` | `[AudioVisual] ShellButtonSlideSound`, default empty | prior `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`; `ini/rulesmd.ini:712` | No for this click path: no `+0x750` load in `0x00612B70`, `0x006AE3F0`, or `0x006ACEE0` |
| `DAT_00833684` | last rendered owner-draw PCX button state character, compared to `'u'` before playing `GenericClick` | `0x00613264..0x0061329B` | Yes: global participates in the live paint path |
| owner state byte `+0xBC` | suppresses owner-draw button mouse-down sound when nonzero | `0x0061374B..0x00613753` | Yes, conditional: active gate on the live mouse path |
| owner state byte `+0xC5` | toggled by `WM_TIMER` and affects visual pressed/hover variants, not the click sound source field | `0x0061363F..0x0061365C`; `0x00612ED7..0x00612F56` | Yes, but not the sound index source |

## 3. Core Logic

### 3.1 Owner-draw hook selection

`FUN_0060F9A0` subclasses child controls during common shell initialization. For class name `"Button"`, it selects `OwnerDraw_Button_00612B70` when the low style bits satisfy `(style & 0x0B) == 0x0B`; prior dialog-resource reports identify Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0` as those Skirmish `0x102` controls.

**Active in YR:** Yes. `FUN_00622B50` handles `WM_INITDIALOG (0x110)` by enumerating children and calling `FUN_0060F9A0`; `FUN_006AE2C0` creates dialog `0x102` with `FUN_006AE3F0`.

### 3.2 Mouse-down / double-click sound

In `OwnerDraw_Button_00612B70`, messages `0x201` and `0x203` share one sound path:

1. If owner state byte `+0xBC` is nonzero, return `0` and do not play sound.
2. Otherwise load `RulesClass` from `0x008871E0`.
3. Push sound handle/source argument `0`.
4. Set `EDX = 0x2000`.
5. Push volume `1.0f` (`0x3F800000`).
6. Load `ECX = [RulesClass + 0x188]`.
7. Call `0x00750920`.
8. Fall through to `CallWindowProcA` so the standard button proc can continue mouse capture/state handling.

**Evidence:** `0x0061374B..0x00613776` and `0x0061378B..0x00613794`.

**Active in YR:** Yes. These messages are normal Win32 mouse messages delivered to the subclassed Skirmish buttons; no TS-only or special-mode gate appears in this path.

### 3.3 Paint-time state-transition sound

During `WM_PAINT (0x0F)`, the same callback computes the visual state character from the current button state:

- default state char is `'u'`;
- if the button state bit is set, state char becomes `'d'`;
- disabled style `0x08000000` forces the state back to `'u'`;
- if state is `'d'` and `DAT_00833684` is `'u'`, it plays `RulesClass + 0x70C` via `0x00750920`;
- after the test, it writes the current state char back to `DAT_00833684`.

The call parameters at the `GenericClick` site match the mouse-down site: source/handle `0`, `EDX = 0x2000`, volume `1.0f`, `ECX = [RulesClass + 0x70C]`.

**Evidence:** `0x0061323C..0x0061329B`.

**Active in YR:** Yes. The callback is active for the three Skirmish buttons. Conditional: the `GenericClick` call requires an enabled button and the first rendered transition from global last state `'u'` to current `'d'`.

### 3.4 `0x00750920` call signature details

`0x00750920` treats `ECX` as the VocClass index: it rejects negative/out-of-range indices, indexes `0x00B1D37C` when valid, and proceeds only if the resolved sound object is non-null. The pushed `0` is a sound handle/source pointer argument; when non-null the routine updates/reuses it, but the Skirmish owner-draw calls pass `0`.

**Evidence:** `0x00750920..0x007509D7`.

**Active in YR:** Yes. Both owner-draw call sites call this function directly.

## 4. INI Keys

| INI key | YR value | Rules offset | Effect in this slice | Active in YR |
|---|---|---:|---|---|
| `[AudioVisual] GUIMainButtonSound` | `MenuClick` | `+0x188` | Mouse-down / double-click owner-draw button sound | Yes |
| `[AudioVisual] GenericClick` | `MenuClick` | `+0x70C` | Paint-time `'u' -> 'd'` transition sound | Yes |
| `[AudioVisual] ShellButtonSlideSound` | empty | `+0x750` | Not used by these button clicks; used by shell slide-in completion per prior report | No for this click path |

## 5. Integration Points

### 5.1 Dialog proc handoff

`FUN_006AE3F0` first delegates each message to `FUN_00622B50`. If unhandled and the message is `WM_COMMAND (0x111)`, it splits `wParam`:

- low word -> `EDX` control id for `FUN_006ACEE0`;
- high word -> pushed notification code;
- `lParam` -> pushed child HWND;
- `ECX` -> parent HWND.

Then it returns `1`.

**Evidence:** `0x006AE404..0x006AE411` for common proc first, `0x006AE425..0x006AE448` for `WM_COMMAND` split/call.

**Active in YR:** Yes. This is the dialog proc passed to `FUN_00622650` for resource `0x102` at `0x006AE31C..0x006AE328`.

### 5.2 `FUN_006ACEE0` control handling

| Control | Command behavior | Notification gate | Evidence | Active in YR |
|---|---|---|---|---|
| `0x617` Start Game | disables Start button, validates player/map settings, then stores result `0x617` through the dialog result pointer on success | requires notification `0` | `0x006ACF7B..0x006ACFA4`, `0x006AD05F..0x006AD0ED`, `0x006AD2D9..0x006AD8DA` | Yes |
| `0x5AA` Choose Map | saves current map token, hides Skirmish dialog, runs chooser flow, restores/rebuilds preview/session state after return | no explicit notification-code gate in the `0x5AA` branch | `0x006ACF60..0x006ACF67`, `0x006AD8E7..0x006ADA21` | Yes |
| `0x5C0` Back | shares the Start/Back branch after the notification gate; skips Start-only validation and can store result `0x5C0` through the dialog result pointer | requires notification `0` | `0x006ACF6D..0x006ACF8C`, `0x006AD2BA..0x006AD8DA` | Yes |

Command execution is therefore release/activation driven through the normal button command path, while the two owner-draw sound sites happen earlier in the child control's subclassed window procedure.

## 6. Current Rust Implementation Status

Rust has the three Skirmish owner-draw button identities and press/release gating, but no equivalent Skirmish click sound call was found in the scanned path.

| Rust area | Status | Evidence |
|---|---|---|
| button identities | implemented | `src/ui/skirmish_shell/state.rs:97-117` |
| Skirmish mouse down | stores pressed owner-draw button only; no sound call in this function | `src/app.rs:563-572` |
| Skirmish mouse up | acts only when release hits the same owner-draw button | `src/app.rs:574-600` |
| main-menu sound helper | exists for main-menu owner-draw buttons only | `src/app.rs:603-621`, `src/app.rs:639-650` |
| Choose Map action | currently cycles map index in-place rather than opening chooser | `src/ui/skirmish_shell/state.rs:165-169`; prior trace |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_Button_00612B70` mouse `0x201/0x203` | verified | `0x0061374B..0x00613776` | none for this slice |
| `OwnerDraw_Button_00612B70` paint transition | verified | `0x0061323C..0x0061329B` | none for this slice |
| `FUN_006AE3F0` `WM_COMMAND` split | verified | `0x006AE425..0x006AE448` | none |
| `FUN_006ACEE0` `0x617` branch | verified | `0x006ACF7B..0x006AD8DA` | deeper Start validation semantics out of scope |
| `FUN_006ACEE0` `0x5AA` branch | verified | `0x006AD8E7..0x006ADA21`; prior trace | chooser internals out of scope |
| `FUN_006ACEE0` `0x5C0` branch | verified | `0x006ACF6D..0x006AD8DA` | none for command gating |
| `FUN_0060F9A0` Button subclass selection | verified | `0x0060FE26..0x0060FE8B` | none |
| `FUN_006AE2C0` active dialog creation | verified | `0x006AE31C..0x006AE328` | none |
| `ShellButtonSlideSound +0x750` exclusion | verified for this path | no load in inspected functions; prior slide report locates separate `0x006071E0` consumer | wider shell animation not re-investigated |
| current Rust Skirmish sound parity | touched-not-exhausted | `src/app.rs:563-600`, `src/app.rs:603-650` | broader UI/audio audit out of scope |

## 8. Open Questions - Final State

[RESOLVED] OQ-SK-BTN-SND-001 - Do the three Skirmish buttons route to `OwnerDraw_Button_00612B70`? Yes, via Button class style `(style & 0x0B) == 0x0B` in `FUN_0060F9A0`, with prior resource docs mapping `0x617`, `0x5AA`, `0x5C0` to this callback. Evidence: `0x0060FE26..0x0060FE8B`, `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`.

[RESOLVED] OQ-SK-BTN-SND-002 - Is click sound triggered by `FUN_006ACEE0`? No. Sound calls are in `OwnerDraw_Button_00612B70` before command handling; `FUN_006ACEE0` performs actions. Evidence: `0x0061374B..0x00613776`, `0x0061323C..0x0061329B`, `0x006AE425..0x006AE448`.

[RESOLVED] OQ-SK-BTN-SND-003 - Which source fields feed the sound calls? `GUIMainButtonSound` at Rules `+0x188` for mouse down/double-click; `GenericClick` at Rules `+0x70C` for paint-time up-to-down transition. Evidence: `0x00613759..0x00613771`, `0x00613273..0x00613289`, `RULESCLASS_FIELDS.csv`.

[RESOLVED] OQ-SK-BTN-SND-004 - Does `ShellButtonSlideSound` participate? No for this click path. Prior report places `+0x750` at the shell slide-in completion routine, not owner-draw click or `FUN_006ACEE0`. Evidence: `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`, inspected `0x00612B70`, `0x006AE3F0`, `0x006ACEE0`.

[RESOLVED] OQ-SK-BTN-SND-005 - Is the path active in standard YR? Yes. `FUN_006AE2C0` creates/pumps dialog `0x102` with `FUN_006AE3F0`; `FUN_00622B50` subclasses children on `WM_INITDIALOG`; no TS-only gate appears in this chain. Evidence: `0x006AE31C..0x006AE328`, `0x00622B50`, `0x0060F9A0`.

[DEFERRED] OQ-SK-BTN-SND-006 - Does retail Windows emit both the mouse-down and paint-transition sounds audibly on every click, or can message coalescing suppress the paint transition in some runtime cases? Category: needs-runtime-debugger. Binary proves both call sites and ordering conditions, but exact audible double-play behavior should be runtime-captured if needed.

## Sources

- Ghidra functions/regions: `OwnerDraw_Button_00612B70`, `FUN_006AE3F0`, `FUN_006ACEE0`, `FUN_0060F9A0`, `FUN_00622B50`, `FUN_006AE2C0`, `0x00750920`.
- Prior docs: `SKIRMISH_CHOOSE_MAP_ACTION_TRACE.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`, `GLOBAL_SOUNDS_GHIDRA_REPORT.md`, `RULESCLASS_FIELDS.csv`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust read-only scan: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`.
