# ShellButtonSlideSound Call Site - Ghidra Report

Date: 2026-05-18

Scope: locate the live `PlaySound` call site for INI key
`[AudioVisual] ShellButtonSlideSound=` (RulesClass field at offset `+0x750`),
or prove the call is dormant in standard Yuri's Revenge. Picks up from
`MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md` (which noted the field is parsed but no
live trigger was found) and `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`
(which described a "paint/state-transition sound site" inside
`OwnerDraw_Button_00612B70` and left attribution open).

This investigation is READ-ONLY. No Ghidra mutations, no Rust changes.

## Executive Summary

**Active in YR: Yes (conditional)**. `ShellButtonSlideSound` has exactly one
live consumer in gamemd.exe: at `0x00607F59` inside `FUN_006071E0`. The trigger
is the **end-of-animation cue** in the generic shell "dialog slide-in" routine,
paired with `GUIMoveInSound` (offset `+0x1A0`, default `MenuSlideIn`) which the
caller plays at the START of the same animation.

**It is NOT the initial main-menu (dialog 0xE2) button-click sound.** The prior
follow-up doc's "paint/state-transition sound site" at `0x00612B70` is a
separate consumer that loads `RulesClass + 0x70C` (`GenericClick`,
default `MenuClick`), not `+0x750` (`ShellButtonSlideSound`). That attribution
needs no correction here since the prior doc did not pin it to the slide key,
but the two should not be confused.

**Default behavior in retail YR: silent.** `rulesmd.ini` and `rules.ini` both
ship with `ShellButtonSlideSound=` empty (defaults table in
`GLOBAL_SOUNDS_GHIDRA_REPORT.md` line 100). The call site fires whenever the
slide-in animation runs; the empty default means `VocClass__PlayAtPos` is
invoked with a `-1` sentinel sound index and produces no audible cue. Mods that
populate the key will hear it.

## Verified Findings

### 1. Parse: `[AudioVisual] ShellButtonSlideSound` -> `RulesClass + 0x750`

Evidence:

- INI string at `0x0083A3D8` (`ShellButtonSlideSound`, ASCII).
- `RULESCLASS_FIELDS.csv` row 478: parsed by `0x006691E0`
  (`RulesClass__ReadAudioVisual`) into offset `0x750` as `sound_idx`.
- Disassembly of the parse site stores the parsed sound index back to
  `[ESI + 0x750]` at `0x0066AFB7`; the prior value is read at `0x0066AF78`
  (default cleanup) and constructor initializes the slot at `0x00666098`.

Confidence: High.

### 2. Single live load site of `+0x750`: `0x00607F59`

A `50 07 00 00` byte-pattern scan against the program returned only three real
load/store sites for the field:

| Address | Instruction | Function | Role |
|---|---|---|---|
| `0x00666098` | `MOV [ESI + 0x750], EBP` | `RulesClass__ReadAudioVisual` adjacent constructor block | Initialize to default (zero) |
| `0x0066AF78` | `MOV EBX, [ESI + 0x750]` | `RulesClass__ReadAudioVisual` (`0x006691E0`) | Read prior value during parse |
| `0x0066AFB7` | `MOV [ESI + 0x750], EAX` | `RulesClass__ReadAudioVisual` (`0x006691E0`) | Store newly parsed value |
| `0x00607F59` | `MOV ECX, [EAX + 0x750]` where `EAX = [0x008871E0]` (RulesClass global) | `FUN_006071E0` | **Live consumer (PlaySound)** |

(The remaining `50 07 00 00` byte hits resolve to non-code regions or
mid-instruction matches that disassemble to unrelated instructions; verified
via per-address `get_assembly_context`.)

Confidence: High. The pattern scan covered the entire image and every code
hit was filtered against the displacement form.

### 3. Trigger context: end of shell dialog slide-in animation

`FUN_006071E0` (function body `0x006071E0..0x00607FC0`) is a `__fastcall`
animation routine. Verified parameter binding from prologue:

- `ECX = HWND` (parent dialog), saved to `local_164` after `EnumChildWindows`.
- `DL = direction flag`, saved at `[ESP + 0x11]` by `0x006071F5: MOV [ESP+0x11], DL`.

Body behavior:

- Iterates frame loop with `Sleep(0x1E)` (30 ms per frame).
- Draws SHP shapes from globals `g_SDTP_SHP`, `g_SDBTNANM_SHP`,
  `g_RadarBackground_SHP`, `g_RadarFrameOpen_SHP` and a child-window cell
  layout collected via `EnumChildWindows`.
- After the frame loop, reloads the direction flag from `[ESP+0x15]`
  (post-pushes, same byte) and tests it:

```text
00607F39  MOV AL, [ESP + 0x15]
00607F40  TEST AL, AL
00607F48  JZ 0x00607FA8           ; skip ShellButtonSlideSound if DL == 0
00607F4A  MOV EAX, [0x008871E0]   ; g_RulesClass_Instance
00607F4F  MOV EDX, 0x2000         ; flags arg
00607F54  PUSH 0x3F800000         ; 1.0f volume
00607F59  MOV ECX, [EAX + 0x750]  ; ShellButtonSlideSound index
00607F5F  CALL 0x00750920         ; VocClass__PlayAtPos
00607FA8  MOV ECX, [ESP+0x30]     ; (DL == 0 fall-through)
00607FAC  PUSH 0
00607FAE  PUSH 0x4ED              ; SendMessage 0x4ED (animation-complete)
```

So the sound plays only on the **show/open direction** (`DL == 1`), not on the
close direction. This makes it a "slide-in completion" cue, symmetric to
`GUIMoveInSound` which fires at the slide-in START (see Finding 4).

Confidence: High.

### 4. Symmetric pair: `GUIMoveInSound` plays at slide-in start

The single show-direction caller of `FUN_006071E0` is `FUN_00608260`
(`0x00608260..0x00608370`). It plays `RulesClass + 0x1A0`
(`GUIMoveInSound`, default `MenuSlideIn`) before invoking the animation:

```text
006082F6  MOV ECX, [0x008871E0]   ; g_RulesClass_Instance
00608304  MOV ECX, [ECX + 0x1A0]  ; GUIMoveInSound index
0060830A  PUSH 0x3F800000
0060830F  CALL 0x00750920         ; VocClass__PlayAtPos
...
0060833F  MOV DL, 0x1             ; direction = show/open
00608341  MOV ECX, ESI            ; this = HWND
00608343  CALL 0x006071E0
```

Mapping of `+0x1A0` from `GLOBAL_SOUNDS_GHIDRA_REPORT.md` row 87
(`GUIMoveInSound`, default `MenuSlideIn`). The two sounds form a slide-in
start/end pair on the same animation.

Confidence: High.

### 5. The "close" caller does NOT play ShellButtonSlideSound

`FUN_00607FD0` (the close/hide wrapper used by general dialog teardown)
and `FUN_00622B50` (the common shell dialog proc shared by RT_DIALOG `0xE2`,
i.e. the standard initial main menu) both invoke `FUN_006071E0` with
`XOR DL, DL` (direction = 0). The DL==0 fall-through at `0x00607F48`
short-circuits past the `+0x750` load, so neither path plays
`ShellButtonSlideSound`.

For dialog `0xE2` specifically (initial main menu), the common shell proc
`FUN_00622B50` calls only with DL=0 (verified at `0x00622CA6: XOR DL, DL`
and `0x00622CAA: CALL 0x006071E0`). This is the close/destroy animation. The
initial main menu therefore does not trigger `ShellButtonSlideSound` on entry,
exit, or button click.

Confidence: High.

### 6. The "show" callers (slide-in animation) are reachable in YR

Direct xrefs to `FUN_00608260`:

- `0x005E6B49` - inside a function body Ghidra has not analyzed (no enclosing
  function entry). Context shows it being called after a returning `cVar3`
  branch, followed by `PUSH 5 / PUSH ESI / CALL [0x007E1498]` (a Win32
  thunk). Adjacent code mentions Load/Save dialog flow.
- `0x00612690` - also inside an unanalyzed body, in the shell owner-draw
  code region just below `OwnerDraw_Button_00612B70`. The site is gated by
  a preceding `CALL EBX / TEST EAX,EAX / JZ` pattern and on success writes
  `[EDI + 0x1FC] = 3` (a state-machine transition).

The flag the show-path requires (`+0xBD` relative to dialog state, which is
`+0xC1` relative to the outer pointer) is set by `FUN_00608380`. Its only xref
is `0x00559474` inside the (misnamed) `CDFileClass__Constructor` body, which is
the **Load/Save game dialog** controller (verified by `LoadDlg_CPP`
string anchor at `0x00829F5C` and dialog ids `0x525/0x527/0x528` referenced
inside the function for Save/Load/Delete variants). The flag is set after a
successful load and just before the dialog teardown chain, so a slide-in
animation runs over the load-complete transition.

Net conclusion: the slide-in animation, and thus `ShellButtonSlideSound`,
fires on the standard YR **Load Game success path** (and any other path that
sets the same flag and reaches `FUN_00608260`). It does NOT fire on the
initial main menu, on owner-draw button clicks generally, or on dialog close.

Confidence: High for the Load Game trigger chain; Medium for "no other live
caller" because two of the three xref sites land in unanalyzed bodies and a
full WMCommand walk was out of scope.

### 7. TS-vs-YR filter

The slide-in animation function `FUN_006071E0` references shell SHP globals
(`g_SDBTNANM_SHP`, `g_RadarBackground_SHP`, etc.) used by the standard YR
sidebar - these are live assets. No `SpecialFlags` or `IniMD.SpecialFlags`
gate appears in the call chain decompilation. The Load Game dialog itself is
present and used in retail YR (it is the dialog Main_Game routes to when the
player selects "Load Mission" from the main menu). The chain is therefore
live in YR.

`ShellButtonSlideSound=` defaulting to empty in shipped INI is a CONTENT-side
choice, not a code-side dormancy. The call fires; with default INI it
produces no audible sound because `VocClass__PlayAtPos` is handed the
empty/-1 sound index. Mods that set the key will hear the cue.

Confidence: High.

## Correction to Prior Doc

`MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`
section 7 mentions an "internal paint/state-transition sound site for a
button moving from 'u' to 'd'" inside `OwnerDraw_Button_00612B70`, with a
"Medium" confidence label on naming. Verified from disassembly: that paint
transition site at `0x00613289` loads `RulesClass + 0x70C`, which is
`GenericClick` (default `MenuClick`), not `ShellButtonSlideSound`. The prior
doc did not explicitly attribute it to ShellButtonSlideSound, but the
proximity to `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`'s open question 4 made it
a natural false candidate. Eliminated.

## Open Items

- The two `FUN_00608260` callers at `0x005E6B49` and `0x00612690` live in
  Ghidra-unanalyzed code. Creating function boundaries there would let a
  future pass confirm or expand the trigger surface (e.g., whether any
  Skirmish/Network dialog button-click handler also routes through it).
  Read-only constraint of this swarm slot precluded `create_function`.
- Whether the slide-in animation visibly draws over the initial menu
  background depends on the showing-dialog's draw layer ordering, which is
  out of scope for this single-key investigation.

## Sources Checked

Fresh Ghidra functions/sites inspected in this pass:

- `FUN_006071E0` (`0x006071E0..0x00607FC0`) - animation body, ShellButtonSlideSound call site.
- `FUN_00607FD0` - close-direction wrapper (DL=0).
- `FUN_00608260` - show-direction wrapper (DL=1).
- `FUN_00608380` - sets `+0xBD = 1` flag.
- `FUN_00622B50` - common shell dialog proc (used by dialog `0xE2`).
- `OwnerDraw_Button_00612B70` - confirmed paint-transition site loads `+0x70C`, not `+0x750`.
- `RulesClass__ReadAudioVisual` `0x006691E0` - parse + init + read of `+0x750`.
- `CDFileClass__Constructor` at `0x00559474` (xref source of `FUN_00608380`) - Load/Save dialog controller.

Prior reports referenced:

- `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`
- `GLOBAL_SOUNDS_GHIDRA_REPORT.md`
- `SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md`
- `RULESCLASS_FIELDS.csv` row 478

INI files checked:

- `ini/rulesmd.ini` - `ShellButtonSlideSound=` empty (default).
- `ini/rules.ini` - `ShellButtonSlideSound=` empty (default).
