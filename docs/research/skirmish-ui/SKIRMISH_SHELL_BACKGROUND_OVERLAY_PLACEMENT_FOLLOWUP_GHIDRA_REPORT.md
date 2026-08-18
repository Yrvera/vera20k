# Skirmish Shell Background Overlay Placement Follow-up - Ghidra Report

Date: 2026-05-17

Scope: focused follow-up for the offline Yuri's Revenge Skirmish shell parent background path:
`WM_PAINT_Handler -> RightPanel__Draw -> Background_Overlay -> CC_Draw_Shape`.

This report extends and corrects the background-specific parts of:

- `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
- `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md`

Active in YR: Yes, for the standard offline Skirmish dialog resource `0x102`.

## Summary

The prior Skirmish reports correctly identified the active parent-record fields and the
`Background_Overlay` width switch, but they misnamed the SHP behind `DAT_00B0FB50`.
Fresh disassembly of the right-panel loader shows the returned SHP pointer from table
entry `0x00844CE0` is stored into `DAT_00B0FB50`; `0x00844CE0` points to
`MNSCRNS.SHP`, not `MNSCRNL.SHP`. The next table entry, `0x00844CE4`, loads
`MNSCRNL.SHP` and is stored into `DAT_00B0FA04`.

For offline Skirmish dialog `0x102`, `FUN_0060CF00` writes:

| Parent record field | Value | Verified meaning |
|---|---|---|
| `+0x74` / `piVar2[0x1E]` | `FUN_0072D030()` | convert/palette object for the Skirmish background path |
| `+0xE0` / `piVar2[0x39]` | `DAT_00B0FB50` | `MNSCRNS.SHP`, selected only when `g_ScreenWidth == 640` |
| `+0xE4` / `piVar2[0x3A]` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp`, loaded only when `g_ScreenWidth == 800` |

Player-visible implication: the 640 Skirmish parent background should use the small
`MNSCRNS.SHP` corner/background, not the large `MNSCRNL.SHP`. The 800 path still uses
`MnScrnLCoopGameSetup.shp`. The `>800` path selects the alternate `+0xE4` pointer, but
the Skirmish loader only populates that pointer at exactly 800; if it is null, the lower
draw routine returns without drawing.

## Verified Findings

### 1. Common paint order draws right-panel lower corner before parent overlay

Evidence:

- `WM_PAINT_Handler @ 0x00621E90`
- `RightPanel__Draw @ 0x0072E450`
- `Background_Overlay @ 0x0072E730`

`WM_PAINT_Handler` reaches the parent mode-1 branch for the offline Skirmish shell.
In that branch it calls `RightPanel__Draw(...)` first, then fetches the parent record
fields `+0x74`, `+0xE0`, and `+0xE4`, then calls `Background_Overlay(...)`.

`RightPanel__Draw` draws, in this order:

1. `SDTP.SHP` frame 0 at `DAT_00B0FC20`
2. repeated `SDBTNBKGD.SHP` frame 0 at `DAT_00B0FC24`
3. repeated `SDBTNANM.SHP` frame 10 at `DAT_00B0FC10` when the caller flag allows it
4. `SDBTM.SHP` frame 0 at `DAT_00B0FC28`
5. `LWSCRNL.SHP` or `LWSCRNS.SHP` frame 0 at `DAT_00B0FC2C`

Then `Background_Overlay` draws the parent background at `DAT_00B0FC1C`.

2026-05-23 correction: this list is only the base `RightPanel__Draw` stack.
Standard Skirmish `0x102` continues after `Background_Overlay` with two
additional flag-gated chrome draws: `Sidebar_TopHighlight @ 0x0072E8C0` draws
`SDTP.SHP` frame 1 at `DAT_00B0FC20`, and `Minimap_Button @ 0x0072E860` draws
`SDMPBTN.SHP` frame 0 at `DAT_00B0FC14`. See
`SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`.

Why this matters: the bottom-left lower-corner strip is not part of the parent
background SHP. It is drawn by the right-panel function before the overlay. At
640, the expected left-side fill is `MNSCRNS.SHP` plus `LWSCRNS.SHP`; at 800, it is
the Skirmish parent SHP plus `LWSCRNL.SHP`.

Confidence: High.

### 2. `DAT_00B0FB50` is loaded from `MNSCRNS.SHP`

Evidence:

- `Sidebar_RightPanel_SHP_Loading / FUN_0072DFB0` disassembly
- string table search for `MNSCRN`

Relevant disassembly sequence:

```asm
0072e0b0  MOV ECX,dword ptr [0x00844ce0]
0072e0c0  CALL 0x004a38d0
0072e0c5  MOV ECX,dword ptr [0x00844ce4]
0072e0d0  MOV [0x00b0fb50],EAX
0072e0d5  CALL 0x004a38d0
0072e0e5  MOV [0x00b0fa04],EAX
```

The store happens after the call using the previous `ECX` value:

- call using `[0x00844CE0]` returns into `EAX`, then `EAX` is stored in `DAT_00B0FB50`
- call using `[0x00844CE4]` returns into `EAX`, then `EAX` is stored in `DAT_00B0FA04`

String search verified:

- `0x00845150`: `MNSCRNS.SHP`
- `0x00845144`: `MNSCRNL.SHP`

This matches the older `SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md` mapping and corrects
the newer Skirmish background reports that named `DAT_00B0FB50` as `MNSCRNL.SHP`.

Confidence: High.

### 3. Dialog `0x102` writes the small background global into parent `+0xE0`

Evidence:

- `FUN_0060CF00 @ 0x0060CF00`

For dialog ids `0x102`, `0xBC`, `0xBD`, `0xC2`, and `0xC9`, the function executes:

```c
iVar3 = FUN_0072d030();
piVar2[0x1e] = iVar3;
piVar2[0x39] = DAT_00b0fb50;
piVar2[0x3a] = DAT_00b0fa18;
```

The same function has many other dialog-id branches that pair `DAT_00B0FB50` with
different alternate backgrounds, but the standard offline Skirmish id `0x102` uses
`DAT_00B0FA18` as its alternate.

Why this matters: the 640 Skirmish branch is not choosing a generic large background;
it is choosing the parent `+0xE0` value, which is the small background global.

Confidence: High.

### 4. `Background_Overlay` selects `+0xE0` only at width 640

Evidence:

- `Background_Overlay @ 0x0072E730`

The decompiler signature is shifted by calling convention, but the branch is clear:

```c
local_18 = *DAT_00b0fc1c;
local_14 = DAT_00b0fc1c[1];
if (g_ScreenWidth == 0x280) {
    CC_Draw_Shape(param_4, 0, &local_18, &local_10, 0x400, ...);
    return;
}
CC_Draw_Shape(param_5, 0, &local_18, &local_10, 0x400, ...);
```

For dialog `0x102`, `param_4` is parent `+0xE0` (`DAT_00B0FB50` / `MNSCRNS.SHP`) and
`param_5` is parent `+0xE4` (`DAT_00B0FA18` / `MnScrnLCoopGameSetup.shp` when loaded).

Result:

| Screen width | Selected parent SHP pointer | Offline Skirmish visible role |
|---:|---|---|
| 640 | parent `+0xE0` = `DAT_00B0FB50` | `MNSCRNS.SHP` |
| 800 | parent `+0xE4` = `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` |
| >800 | parent `+0xE4` = `DAT_00B0FA18` | selected, but not loaded by `0x0072CF40` unless already populated |

Confidence: High for the branch and 640/800; High that a null selected pointer draws
nothing in `CC_Draw_Shape`; Medium for whether a stale non-null `DAT_00B0FA18` can ever
exist when entering Skirmish at `>800` without runtime watchpoints.

### 5. `0x0072CF40` loads the alternate Skirmish background only at exact width 800

Evidence:

- `FUN_0072CF40 @ 0x0072CF40`
- `FUN_0072CF90 @ 0x0072CF90`

Disassembly:

```asm
0072cf49  CMP dword ptr [0x008a00a4],0x320
0072cf53  JNZ 0x0072cf6a
0072cf55  MOV ECX,dword ptr [0x00844d6c]
0072cf60  CALL 0x004a38d0
0072cf65  MOV [0x00b0fa18],EAX
0072cf6a  MOV ECX,dword ptr [0x00844d70]
0072cf7a  CALL 0x0072ade0
```

`0x00844D6C` is the `MnScrnLCoopGameSetup.shp` table entry. `0x00844D70` is the
`MnScrnLCoopGameSetup.PAL` entry. The SHP is only loaded when `g_ScreenWidth == 800`;
the palette/convert object is loaded regardless once the function runs.

`FUN_0072CF90` clears `DAT_00B0FA18 = 0` on cleanup, so the normal lifecycle does not
preserve the alternate SHP after leaving this setup path.

Confidence: High.

### 6. `CC_Draw_Shape` treats a null selected SHP as no draw

Evidence:

- `CC_Draw_Shape @ 0x004AED70`

The decompiler argument names are shifted, but the early-out is explicit:

```c
if (param_3 == (short *)0x0) {
    return;
}
...
if (*param_3 == -1) {
    ...
    frame_index = *(short **)(param_3 + 6);
}
if (frame_index == (short *)0x0) {
    return;
}
```

For `Background_Overlay`, the chosen background pointer is passed into this SHP
argument position. If `DAT_00B0FA18` is null on a non-640 width, the draw routine
returns before attempting frame decode or blit.

Why this matters: the previously unresolved `>800` case has a concrete lower-level
null outcome. This does not by itself prove a live screenshot will be blank in every
entry history, but it proves the selected null pointer is not converted into a
fallback asset by `CC_Draw_Shape`.

Confidence: High for null behavior.

### 7. Overlay position comes from `DAT_00B0FC1C`, not hardcoded `(0,0)`

Evidence:

- `RightPanel__ComputeLayoutRects @ 0x0072EC70`
- `Background_Overlay @ 0x0072E730`

`RightPanel__ComputeLayoutRects` computes:

```c
local_c = 0;
if (screen_w > 1023) {
    local_c = (screen_w - 800) / 2;
}
iVar6 = 0;
if (screen_h > 767) {
    iVar6 = (screen_h - 600) / 2;
}
DAT_00b0fc1c = { local_c, iVar6, selected_corner_w, selected_corner_h };
```

`Background_Overlay` uses:

```c
local_18 = *DAT_00b0fc1c;      // x
local_14 = DAT_00b0fc1c[1];    // y
```

Therefore:

| Resolution | Background overlay origin |
|---:|---:|
| 640x480 | `(0, 0)` |
| 800x600 | `(0, 0)` |
| 1024x768 | `(112, 84)` |

Since the active Skirmish `>800` background remains pointer-dependent, the 1024 origin
is a verified common-layout value, not proof that the Skirmish parent background draws
there in a fresh `>800` entry.

Confidence: High.

### 8. Overlay clip rect clamps right/bottom against a centered 800x600 region

Evidence:

- `Background_Overlay @ 0x0072E730`
- `RightPanel__Draw @ 0x0072E450`

Both functions copy a target rect and apply the same right/bottom clamp:

```c
if (800 < local_8) {
    local_8 = (local_8 - 800) / 2 + 800;
}
if (600 < local_4) {
    local_4 = (local_4 - 600) / 2 + 600;
}
```

The decompiler labels the rect fields as copied from the parent cached surface region.
At 1024x768, if the copied right/bottom are the full client size, the clamp produces
right `912` and bottom `684`, aligning the clipped draw region with the centered
800x600 shell band.

Confidence: High for the formula; Medium for every possible caller's source rect shape.

### 9. Frame is 0, draw flags are `0x400`, and the shape is not centered by flags

Evidence:

- `Background_Overlay @ 0x0072E730`
- `CC_Draw_Shape @ 0x004AED70`

`Background_Overlay` always passes frame `0` and flags `0x400`. `CC_Draw_Shape` only
applies the center-sprite adjustment if flag `0x200` is set:

```c
if ((param_6 & 0x200) != 0) {
    x -= width / 2;
    y -= height / 2;
}
```

Because `0x400` does not include `0x200`, the background is drawn at the computed
origin plus the frame's own SHP frame offset. It is not centered by the draw helper.

Confidence: High.

## Corrected Asset Matrix

| Global | Load-table evidence | Asset | Role |
|---|---|---|---|
| `DAT_00B0FB50` | return from call using `[0x00844CE0]` | `MNSCRNS.SHP` | small screen parent/corner background |
| `DAT_00B0FA04` | return from call using `[0x00844CE4]` | `MNSCRNL.SHP` | large generic parent/corner background |
| `DAT_00B0FA18` | `0x0072CF40`, exact `g_ScreenWidth == 800` | `MnScrnLCoopGameSetup.shp` | offline Skirmish alternate parent background |
| `DAT_00B0FAE8` | right-panel loader later table slot | `LWSCRNS.SHP` | small lower-left corner strip |
| `DAT_00B0FA54` | right-panel loader later table slot | `LWSCRNL.SHP` | large lower-left corner strip |

## Current Rust Parity Implications

No Rust files were modified during this investigation.

Current Rust state observed:

- `src/render/skirmish_shell_chrome.rs` loads `"MNSCRNL.SHP"` and
  `"MnScrnLCoopGameSetup.shp"` as parent backgrounds.
- `src/app_skirmish_shell_render.rs` maps width `640` to `background_640_mnscrnl`.
- `src/app_skirmish_shell_render.rs` draws that parent background at
  `layout.screen.x, layout.screen.y`.
- `src/render/skirmish_shell_chrome.rs` does not currently load `MNSCRNS.SHP`,
  `LWSCRNS.SHP`, or `LWSCRNL.SHP` for the Skirmish shell atlas.

Based on the verified binary findings:

1. The 640 parent background asset should be `MNSCRNS.SHP`, not `MNSCRNL.SHP`.
2. The lower-left strip should be represented separately through `LWSCRNS.SHP` at 640
   and `LWSCRNL.SHP` at non-640 heights/widths, following `RightPanel__Draw`.
3. The 800 parent background remains `MnScrnLCoopGameSetup.shp`.
4. The `>800` parent background should remain gated/unresolved unless implementing the
   verified null/no-draw outcome explicitly. Do not invent a fallback to `MNSCRNL.SHP`.
5. If a future implementation draws any background at `>800`, it must use the verified
   centered origin from `DAT_00B0FC1C`, not fullscreen `(0,0)`.

## Verification Performed

- Ghidra decompiled/disassembled:
  - `0x0060CF00`
  - `0x00621E90`
  - `0x0072CF40`
  - `0x0072CF90`
  - `0x0072DFB0`
  - `0x0072E450`
  - `0x0072E730`
  - `0x0072E820`
  - `0x0072EC70`
  - `0x004AED70`
- Ghidra string search verified:
  - `0x00845144`: `MNSCRNL.SHP`
  - `0x00845150`: `MNSCRNS.SHP`
  - `0x00844F8C`: `MnScrnLCoopGameSetup.PAL`
  - `0x00844FA8`: `MnScrnLCoopGameSetup.shp`
  - `0x00845104`: `LWSCRNL.SHP`
  - `0x00845110`: `LWSCRNS.SHP`
- Local Rust scan checked current atlas/render assumptions.
- `cargo test retail_right_panel_shell_shps_decode -- --ignored --nocapture` ran, but
  no matching tests existed.
- `cargo test --test sidebar_chrome_inspect -- --nocapture` ran; all seven tests in
  that target are ignored, so no asset-dimension runtime assertion was executed.

## Open Questions

1. Runtime watchpoint evidence is still needed to prove whether a stale non-null
   `DAT_00B0FA18` can ever survive into an offline Skirmish `>800` entry. Static
   evidence shows the normal cleanup clears it and `0x0072CF40` only loads it at
   width 800.
2. Exact retail visual screenshots at 640 and 800 would still be useful to validate
   the final composed result after the small-background and lower-strip fixes.
