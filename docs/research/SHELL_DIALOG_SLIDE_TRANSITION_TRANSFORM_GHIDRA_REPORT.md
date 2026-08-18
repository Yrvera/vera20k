# Shell Dialog Slide Transition - Visual Transform Per Frame - Ghidra Report

Date: 2026-05-19

Scope: identify the per-frame visual transform of the generic shell-dialog
slide animation `FUN_006071E0` (body `0x006071E0..0x00607FC0`) - specifically
what changes per frame, how many frames it runs, and which dialogs actually
trigger the show-direction. Picks up from
`SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`, which already verified
the 30 ms Sleep, the sound pair on the show direction, and the `SendMessage`
completion notifications.

READ-ONLY pass. No Ghidra mutations. No Rust changes.

## Executive Summary

**Active in YR: Yes** (no `SpecialFlags` gate; no TS-only caller).

**The animation is NOT a position slide.** Despite the colloquial name
"slide-in," `FUN_006071E0` does not translate any window or sprite by a
per-frame X/Y delta. The only spatial offset that ever changes is a single
discrete `+0x50` (80 px) horizontal shift of the radar/SDTP shape when
screen width is above a threshold, and that shift is keyed off the per-cell
animation phase, not a per-frame ramp. There are no `SetWindowPos` calls in
the body and no per-frame `(x,y)` mutation on the cell rectangles.

**What actually animates is SHP frame index.** Each child cell (enumerated
via `EnumChildWindows(parent, FUN_0060a180, 0)`) is assigned a 1-frame
stagger offset. On each animation tick, the cell's local progress
`tick - stagger_offset` is mapped through a 3-zone classifier
(pre / mid / settled) and an SHP frame index is computed. The cells appear
to "cycle" through pre-rendered button-state frames in sequence, producing
the illusion of a wave sweeping the dialog.

**Frame count formula:** `total_frames = N + 8` where N is the number of
cells returned by the first `EnumChildWindows` pass. Per-frame wall-clock
cost: `Sleep(0x1E)` = 30 ms. Total wall-clock = `(N + 8) * 30 ms`.

**Direction-dependent frame range:** show (DL=1) cycles per-cell frames
`10 -> 5` over 6 progress steps then snaps to settled frame `1`. Close
(DL=0) cycles `5 -> 10` then snaps to `10`. The step `iStack_174` is `-1`
on show, `+1` on close. The pre-anim and settled values swap accordingly.

**Show-direction callers: exactly 2.** Prior doc cited 3
(`0x005E6B49`, `0x00612690`, `0x00559474`); re-checked via `get_xrefs_to`,
direct xrefs to `FUN_00608260` (the DL=1 wrapper) are only the first two.
`0x00559474` is an xref to the **flag-setter** `FUN_00608380`, not to the
wrapper. Both direct callers land in Ghidra-unanalyzed code regions; one
is in the Load/Save dialog controller body
(LoadDlg_CPP string anchor at `0x00829F5C` reachable via the flag-setter
caller at `0x00559474`); the other is in the shell owner-draw region just
below `OwnerDraw_Button_00612B70` and toggles a `+0x1FC` state machine
from 1 to 3.

**Anchor-doc correction:** the prior report states `FUN_00608380` sets
`+0xBD`; verified via decompilation it actually sets `+0xC1`, which matches
the gate `FUN_00608260` reads at `*(char *)((int)piVar1 + 0xc1)`. Single-byte
typo in the prior doc - the binding is otherwise correct.

## Verified Findings

### 1. Per-frame timing: Sleep(30 ms), total = (N+8) frames

Evidence (assembly inside the main loop at `0x006071E0`):

- `0x00607f0f: PUSH 0x1e` -> `0x00607f11: CALL [0x007e11f0]` (Sleep).
- Loop index `uStack_184` at `[ESP + 0x1c]`, initialized to 0
  (`0x006076a7: MOV [ESP + 0x1c], EBX` with EBX=0).
- Loop terminator `iStack_bc` at `[ESP + 0xe4]`, compared with
  `JL` at `0x00607f29`.

Construction of `iStack_bc`:

- `iStack_168 = N` (cell count from `EnumChildWindows(local_164, FUN_0060a180, 0)`
  at `0x006075ed`).
- `local_17c = operator_new((N + 3) * 4)` (4-byte array of length N+3).
- Fill loop `0x00607672..0x0060767f` sets `local_17c[0..N] = 1..N+1`.
- Then `local_17c[N+2] = 0`, `local_17c[N+1] = N+2`, `local_17c[N] = 0`
  (assembly `0x00607689..0x00607690`).
- Max-scan `0x00607696..0x006076a2` finds `ECX = max(local_17c) = N+2`.
- `iStack_bc = ECX + 6 = N + 8` at `0x006076a4`.

Confidence: High.

### 2. Per-cell phase classifier (no position translation)

For each cell at frame `f`, with the cell's stagger offset `s = local_17c[idx]`,
the progress is `iVar5 = f - s` and three branches apply (assembly pattern
appears 6 times in the body, one per drawn element class):

```text
  if iVar5 < 0:                      pre-anim     -> branch A
  else if iVar5 < 6:
        if iVar5 == -1: pre-anim    (dead branch, signed compare)
        if iVar5 == -2: settled     (dead branch)
        else:           anim:       iVar5 * step + base    -> branch B
  else:                              settled      -> branch C
```

Step `iStack_174` is derived at `0x00607508..0x00607526`:

```text
  MOV  CL, AL          ; AL = DL
  NEG  CL              ; CL = -DL
  SBB  ECX, ECX        ; ECX = -1 if DL!=0, 0 if DL==0
  AND  ECX, 0xfffffffe ; ECX = -2 or 0
  INC  ECX             ; ECX = -1 (show) or +1 (close)
```

Base frame indices, also direction-keyed, stored in stack slots used by
each drawn-element class:

- `iStack_13c` (regular cell base): `5` if DL=0 else `10` (`0x00607532..0x00607578`).
- `iStack_114` (final cell row): `0xB` if DL=0 else `0x10` (`0x0060757c..0x0060758b`).
- `iStack_10c` (radar mid-anim base): `1` if DL=0 else `6` (`0x00607598..0x006075a5`).
- `local_118` (SDTP secondary): `0` if DL=0 else `5` (`0x006075b0..0x006075bd`).
- `iStack_110`: `0` if DL=0 else `5` (`0x006075c4..0x006075cf`).

Pre-anim / settled values are computed from `cVar14` (= DL low byte tested
later in the loop) using the bitmask idiom
`(-(uint)(cVar14 != 0) & K) + C`, producing direction-keyed constants:

- Regular cell pre/settled: pre=1/settled=10 on show (DL!=0),
  pre=10/settled=1 on close (DL=0). (`LAB_00607a13` / `LAB_00607a96`.)
- SDTP shape: pre=0/settled=0 on show, pre=6/settled=6 on close (etc., per
  branch at `LAB_00607749` / `LAB_006077ba`).

Net per-cell: SHP frame index cycles 6 steps from `base` toward
`base + 5*step`, then snaps to the settled value. With step=-1 on show,
the cycle is `10,9,8,7,6,5` then settles at `1`; on close it is
`5,6,7,8,9,10` then settles at `10`. The 1-frame stagger means cell `idx`
starts animating on frame `idx+1` (its `local_17c[idx]`).

Confidence: High.

### 3. No per-frame (x,y) translation in the body

Searched the body for any windowed `SetWindowPos` call or per-iteration
mutation of cell rect coordinates:

- No `SetWindowPos` call site inside `0x006071E0..0x00607FC0`.
- Cell Y position `iStack_104` advances **once per cell** (`iVar5 += iStack_120`
  inside the inner cell loop at `0x00607b08..0x00607b16`), not per outer
  frame tick.
- The only horizontal offset that ever moves is the SDTP/radar `+0x50`
  shift at `0x00607d9e` / `0x00607df3` / `0x00607e39`, applied conditionally
  on `g_ScreenWidth >= [0x007f5be4]` AND keyed by the phase classifier
  branch (pre vs anim vs settled), not by a per-frame ramp.

The "slide" effect is therefore a wave of SHP frame transitions, not a
positional slide. The visual matches: pre-rendered cell button SHPs whose
intermediate frames depict the cell partially drawn/skewed produce the
appearance of motion when stepped through.

Confidence: High.

### 4. Show-direction callers (re-verified)

`get_xrefs_to 0x00608260` returns exactly 2 sites:

| Address | Context | Identification |
|---|---|---|
| `0x005E6B49` | After a `CALL 0x0052FEC0`, `TEST BL,BL`, then `CALL 0x00608260`, then `PUSH 5 / PUSH ESI / CALL [0x007E1498]`. Function entry unanalyzed; sits in the `0x005E6XXX` region adjacent to `0x005E6988`, which is one of many close-direction xrefs. | Unanalyzed body in shell dialog flow. Likely the post-Load-Game-success continuation - matches the wrapper sequence anchored by `FUN_00608380(EDI)` call at `0x00559474` inside `CDFileClass__Constructor` (which is the Load/Save dialog controller, `LoadDlg_CPP` string at `0x00829F5C`). |
| `0x00612690` | Inside a state-machine pattern: `CMP [EDI + 0x1FC], 1` -> `CALL EBX` -> `JZ 0x006126A3` -> `CALL 0x00608260` -> on success `MOV [EDI + 0x1FC], 3`. Adjacent to `0x0061266F: MOV [EDI + 0x1FC], 2`. | Shell owner-draw region (just below `OwnerDraw_Button_00612B70`). Drives the dialog state machine 1 -> 3 when slide-in succeeds; the `2` write at `0x0061266F` is an intermediate state. Specific dialog ID unconfirmed (would require enclosing function boundary creation, blocked by read-only constraint). |

`0x00559474` is NOT a direct caller of `FUN_00608260`. It is the sole caller
of `FUN_00608380` (the `+0xC1` flag-setter), confirmed via
`get_xrefs_to 0x00608380`. The prior doc conflated the flag-setter xref
with a direct wrapper xref.

Confidence: High for the count and addresses. Medium for the dialog-screen
identification of `0x00612690` (no enclosing function).

### 5. Anchor doc correction: flag offset

`SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` finding 6 states
"flag the show-path requires (`+0xBD` relative to dialog state)". Verified:

- `FUN_00608380` writes `*(undefined1 *)((int)piVar2 + 0xc1) = 1`.
- `FUN_00608260` gates on `*(char *)((int)piVar1 + 0xc1) == '\\0'`.

Both offsets are `+0xC1`, not `+0xBD`. Single-byte typo upstream; binding
is otherwise correct.

Confidence: High.

### 6. TS-vs-YR filter

- No `SpecialFlags` test in `FUN_006071E0`, `FUN_00608260`, `FUN_00607FD0`,
  or `FUN_00608380`.
- The SHP globals referenced (`g_SDTP_SHP`, `g_SDBTNANM_SHP`,
  `g_RadarBackground_SHP`, `g_RadarFrameOpen_SHP`) are the standard YR
  shell shapes. `g_RadarFrameOpen_SHP` is also used by the live YR
  sidebar (cross-checked with `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`).
- The Load/Save dialog (`LoadDlg_CPP`) is reachable from the YR main menu
  "Load Mission" path - live.
- The `0x00612690` site references the live `+0x1FC` shell state machine.

No TS-only gating identified. The animation function is YR-active.

Confidence: High.

## Open Items

- Enclosing functions for `0x005E6B49` and `0x00612690` are still
  unanalyzed in Ghidra; `create_function` was blocked by the read-only
  constraint. With a future read-write pass, these could be named and
  the originating dialog/button click traced.
- Whether the cell SHP frames `5..10` actually depict mid-slide
  graphical states (vs simple highlight/depress) requires an SHP file
  inspection of the relevant asset (e.g. `SDBTNANM.SHP`). That is asset-
  side, not binary-side, and is out of scope for this pass.
- The interaction between `EnumChildWindows(parent, LAB_00606800, 1)`
  (called before the animation in `FUN_00608260`) and the per-cell
  enumeration callback `FUN_0060A180` (called inside the animation):
  `LAB_00606800` likely toggles child-window visibility/state so the
  animation owns the redraw region. Worth a follow-up if the visual
  doesn't match in implementation.

## Sources Checked

- `FUN_006071E0` (`0x006071E0..0x00607FC0`) - full decompile + assembly.
- `FUN_00608260` (`0x00608260..0x00608370`) - show wrapper, decompile.
- `FUN_00607FD0` - close wrapper, decompile.
- `FUN_00608380` - flag-setter, decompile (corrected offset to `+0xC1`).
- `get_xrefs_to 0x00608260` -> 2 xrefs.
- `get_xrefs_to 0x00608380` -> 1 xref (`0x00559474` in `CDFileClass__Constructor`).
- `get_xrefs_to 0x00607FD0` -> 30 callers (close-direction, out of scope).
- `get_assembly_context 0x005E6B49`, `0x00612690`, `0x00559474` -
  20-25 instruction context windows.

Prior reports referenced:

- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` (anchor).
- `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` (SHP global cross-check).
- `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md` (cell layout context).
