# Skirmish Shell Right-Panel/Background/Palette Follow-up - Ghidra Report

Date: 2026-05-17

Scope: targeted follow-up for the remaining offline Yuri's Revenge Skirmish shell asset questions after `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`.

Primary target path:

`FUN_006AE2C0 -> FUN_0072CF40 -> dialog 0x102 -> WM_PAINT_Handler -> RightPanel__Draw -> Background_Overlay -> CC_Draw_Shape`

Active in YR: Yes for standard offline Skirmish dialog resource `0x102`, except where explicitly marked as generic/research-only.

## Executive Summary

This pass resolves two important asset questions that were still wrong or incomplete in the previous plan/report chain:

1. The 640-width parent background for offline Skirmish is `MNSCRNS.SHP`, not `MNSCRNL.SHP`.
2. The lower-left/right-panel side strips are separate SHPs: `LWSCRNS.SHP` at 640 and `LWSCRNL.SHP` at non-640 widths.

It also verifies the right-panel palette split:

- `SHELL.PAL` convert object `DAT_00B0FBCC` is used for `SDTP.SHP`, `SDBTM.SHP`, and the `LWSCRN*.SHP` lower/side strips.
- `SHELL2.PAL` convert object `DAT_00B0FBD4` is used for repeated `SDBTNBKGD.SHP`.
- `SDBTNANM.PAL` convert object `DAT_00B0FBDC` is used for repeated `SDBTNANM.SHP` frame `10`.

The 800-width parent background remains `MnScrnLCoopGameSetup.shp` with `MnScrnLCoopGameSetup.PAL`. For non-640/non-800 widths, `Background_Overlay` selects the alternate parent pointer, but `FUN_0072CF40` only loads that pointer at exact width 800 and `CC_Draw_Shape` returns immediately for a null SHP pointer. Static evidence therefore supports blank/no parent background for a fresh `>800` Skirmish entry, with a remaining runtime-only question about stale non-null pointer history.

2026-05-23 correction: this report's base `RightPanel__Draw` stack is still
correct, but it was incomplete for standard Skirmish `0x102` first paint. After
`RightPanel__Draw` and `Background_Overlay`, `WM_PAINT_Handler` draws two
additional `0x102`-enabled chrome pieces: `SDTP.SHP` frame `1` through
`Sidebar_TopHighlight @ 0x0072E8C0`, and `SDMPBTN.SHP` frame `0` through
`Minimap_Button @ 0x0072E860`. `SDMPBTN.SHP` is still not the preview bitmap or
preview backing; it is the lower Battle/map-name right-panel chrome at
`DAT_00B0FC14`. See
`SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`.

## Verified Binary Findings

### 1. Right-panel loader table order maps `DAT_00B0FB50` to `MNSCRNS.SHP`

Evidence:

- `FUN_0072DFB0 @ 0x0072DFB0` disassembly
- string table near `0x00845104`
- pointer table near `0x00844CD4`

Relevant loader sequence:

```asm
0072e076  MOV ECX,dword ptr [0x00844cd4]  ; SDBTNANM.SHP
0072e081  CALL 0x004a38d0
0072e091  MOV [0x00b0fac4],EAX

0072e086  MOV ECX,dword ptr [0x00844cd8]  ; SDMPBTN.SHP
0072e096  CALL 0x004a38d0
0072e0a6  MOV [0x00b0f9dc],EAX

0072e09b  MOV ECX,dword ptr [0x00844cdc]  ; SDWRNTMP.SHP
0072e0ab  CALL 0x004a38d0
0072e0bb  MOV [0x00b0fac0],EAX

0072e0b0  MOV ECX,dword ptr [0x00844ce0]  ; MNSCRNS.SHP
0072e0c0  CALL 0x004a38d0
0072e0d0  MOV [0x00b0fb50],EAX

0072e0c5  MOV ECX,dword ptr [0x00844ce4]  ; MNSCRNL.SHP
0072e0d5  CALL 0x004a38d0
0072e0e5  MOV [0x00b0fa04],EAX
```

String/pointer evidence:

| Pointer table | String address | String | Stored global |
|---|---:|---|---|
| `0x00844CD4` | `0x00845178` | `SDBTNANM.SHP` | `DAT_00B0FAC4` |
| `0x00844CD8` | `0x0084516C` | `SDMPBTN.SHP` | `DAT_00B0F9DC` |
| `0x00844CDC` | `0x0084515C` | `SDWRNTMP.SHP` | `DAT_00B0FAC0` |
| `0x00844CE0` | `0x00845150` | `MNSCRNS.SHP` | `DAT_00B0FB50` |
| `0x00844CE4` | `0x00845144` | `MNSCRNL.SHP` | `DAT_00B0FA04` |
| `0x00844CE8` | `0x00845138` | `SDTP.SHP` | `DAT_00B0FAF8` |
| `0x00844CEC` | `0x00845128` | `SDBTNBKGD.SHP` | `DAT_00B0FA74` |
| `0x00844CF0` | `0x0084511C` | `SDBTM.SHP` | `DAT_00B0FA38` |
| `0x00844CF4` | `0x00845110` | `LWSCRNS.SHP` | `DAT_00B0FAE8` |
| `0x00844CF8` | `0x00845104` | `LWSCRNL.SHP` | `DAT_00B0FA54` |

Tiny but load-bearing detail: the store to `DAT_00B0FB50` occurs after the call using the previous `ECX` value `[0x00844CE0]`, not after the next table entry. That makes `DAT_00B0FB50 = MNSCRNS.SHP`.

Confidence: High.

### 2. Dialog `0x102` uses `DAT_00B0FB50` as parent `+0xE0`

Evidence:

- `FUN_0060CF00 @ 0x0060CF00`

For dialog ids `0x102`, `0xBC`, `0xBD`, `0xC2`, and `0xC9`, the function writes:

```c
iVar3 = FUN_0072d030();
piVar2[0x1e] = iVar3;        // byte +0x78? decompiler int slot 0x1e; report-visible parent convert path
piVar2[0x39] = DAT_00b0fb50; // byte +0xE4 if counted from piVar2+1 record base in older notes; active parent +0xE0 naming retained in prior reports
piVar2[0x3a] = DAT_00b0fa18;
```

Prior reports used the shorthand parent fields `+0x74`, `+0xE0`, and `+0xE4` for the dialog metadata record. The important output-determining fact is unchanged: dialog `0x102` receives `DAT_00B0FB50` as the first/background SHP pointer and `DAT_00B0FA18` as the alternate SHP pointer.

Because Finding 1 proves `DAT_00B0FB50 = MNSCRNS.SHP`, the 640 branch draws `MNSCRNS.SHP`, not `MNSCRNL.SHP`.

Confidence: High.

### 3. `Background_Overlay` selects the small parent SHP only at width 640

Evidence:

- `Background_Overlay @ 0x0072E730`

The branch is:

```c
local_18 = *DAT_00b0fc1c;
local_14 = DAT_00b0fc1c[1];
if (g_ScreenWidth == 0x280) {
    CC_Draw_Shape(param_4, 0, &local_18, &local_10, 0x400, ...);
    return;
}
CC_Draw_Shape(param_5, 0, &local_18, &local_10, 0x400, ...);
```

For offline Skirmish dialog `0x102`:

| Screen width | Selected pointer | Verified asset |
|---:|---|---|
| 640 | `DAT_00B0FB50` | `MNSCRNS.SHP` |
| 800 | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` |
| `>800` | `DAT_00B0FA18` | selected pointer; normally null unless loaded by exact-800 lifecycle |

The frame is always `0`. The flags passed are `0x400`, which does not include center flag `0x200`.

Confidence: High for 640/800 branch and selected pointer; High for null selected pointer no-draw; Medium for stale-pointer runtime history above 800.

### 4. `FUN_0072CF40` still only loads `MnScrnLCoopGameSetup.shp` at exact width 800

Evidence:

- `FUN_0072CF40 @ 0x0072CF40`
- `FUN_0072CF90 @ 0x0072CF90`

Relevant sequence:

```asm
0072cf49  CMP dword ptr [0x008a00a4],0x320
0072cf53  JNZ 0x0072cf6a
0072cf55  MOV ECX,dword ptr [0x00844d6c]  ; MnScrnLCoopGameSetup.shp
0072cf60  CALL 0x004a38d0
0072cf65  MOV [0x00b0fa18],EAX
0072cf6a  MOV ECX,dword ptr [0x00844d70]  ; MnScrnLCoopGameSetup.PAL
0072cf7a  CALL 0x0072ade0
```

`FUN_0072CF90` clears `DAT_00B0FA18 = 0` during cleanup. Static lifecycle evidence therefore supports `>800` fresh entry as no parent background draw.

Confidence: High.

### 5. `CC_Draw_Shape` null-SHP handling is an early no-op

Evidence:

- `CC_Draw_Shape @ 0x004AED70`

The early exits:

```c
if (param_3 == (short *)0x0) {
    return;
}
frame_index = param_3;
if (*param_3 == -1) {
    ...
    frame_index = *(short **)(param_3 + 6);
}
if (frame_index == (short *)0x0) {
    return;
}
```

For `Background_Overlay`, the chosen background SHP pointer reaches this SHP argument position. If `DAT_00B0FA18` is null on a non-640 width, the lower draw routine returns before frame rect/data lookup and before any blit.

Confidence: High for null behavior.

### 6. `RightPanel__Draw` exact SHP order and palette split

Evidence:

- `RightPanel__Draw @ 0x0072E450` disassembly
- palette table near `0x00844BE4`
- palette string block near `0x00845438`
- `FUN_0072ADE0 @ 0x0072ADE0`

Palette load sequence inside `RightPanel__Draw` lazy-init and `FUN_0072DFB0` full loader:

```asm
MOV ECX,[0x00844be4] ; SHELL.PAL
MOV EDX,0xb0fbc8
PUSH 0xb0fbcc
CALL 0x0072ade0

MOV ECX,[0x00844be8] ; SHELL2.PAL
MOV EDX,0xb0fbd0
PUSH 0xb0fbd4
CALL 0x0072ade0

MOV ECX,[0x00844bec] ; SDBTNANM.PAL
MOV EDX,0xb0fbd8
PUSH 0xb0fbdc
CALL 0x0072ade0
```

`FUN_0072ADE0` allocates a raw 256-entry RGB palette, shifts 6-bit channels left by 2, and builds a ConvertClass object against `DAT_00887310`.

Draw order with SHP and convert object:

| Step | Draw site | SHP global | Asset | Frame | Convert object | PAL |
|---:|---:|---|---|---:|---|---|
| 1 | `0x0072E547..0x0072E56A` | `DAT_00B0FAF8` | `SDTP.SHP` | 0 | `DAT_00B0FBCC` | `SHELL.PAL` |
| 2 | `0x0072E594..0x0072E5C1` loop | `DAT_00B0FA74` | `SDBTNBKGD.SHP` | 0 | `DAT_00B0FBD4` | `SHELL2.PAL` |
| 3 | `0x0072E60D..0x0072E63A` loop if branch allows | `DAT_00B0FAC4` | `SDBTNANM.SHP` | 10 | `DAT_00B0FBDC` | `SDBTNANM.PAL` |
| 4 | `0x0072E68C..0x0072E69F` | `DAT_00B0FA38` | `SDBTM.SHP` | 0 | `DAT_00B0FBCC` | `SHELL.PAL` |
| 5a | `0x0072E6CD..0x0072E71F` at width 640 | `DAT_00B0FAE8` | `LWSCRNS.SHP` | 0 | `DAT_00B0FBCC` | `SHELL.PAL` |
| 5b | `0x0072E6F7..0x0072E71F` otherwise | `DAT_00B0FA54` | `LWSCRNL.SHP` | 0 | `DAT_00B0FBCC` | `SHELL.PAL` |

The repeated tile loops increment Y by the corresponding rect height:

- `SDBTNBKGD.SHP` advances by `DAT_00B0FC24[3]`.
- `SDBTNANM.SHP` advances by `DAT_00B0FC10[3]`.

The `SDBTNANM.SHP` loop executes only when the caller flag reaches `RightPanel__Draw` as zero in the decompiled branch:

```c
if (param_3 == '\0') {
    repeat frame 10 DAT_00B0FA20 times
}
```

For `WM_PAINT_Handler`, the call is `RightPanel__Draw((char)piVar9[0x35] == '\0')`, so the overlay depends on the dialog state byte/flag at that record slot. This pass verifies the branch and assets, not every state transition of that flag.

Confidence: High for SHP/PAL mapping and draw order; Medium for semantic naming of the caller flag.

### 7. Right-panel layout rects choose small vs large assets independently for width and height

Evidence:

- `RightPanel__ComputeLayoutRects @ 0x0072EC70`

Important branches:

```c
if (param_1 != 0x280) {
    sVar1 = *(short *)(DAT_00b0fa04 + 2); // MNSCRNL width
} else {
    sVar1 = *(short *)(DAT_00b0fb50 + 2); // MNSCRNS width
}

if (param_2 != 0x1e0) {
    sVar2 = *(short *)(DAT_00b0fa04 + 4); // MNSCRNL height
} else {
    sVar2 = *(short *)(DAT_00b0fb50 + 4); // MNSCRNS height
}
```

The lower/side strip has the same independent width/height choice:

```c
if (param_1 != 0x280) {
    sVar1 = *(short *)(DAT_00b0fa54 + 2); // LWSCRNL width
} else {
    sVar1 = *(short *)(DAT_00b0fae8 + 2); // LWSCRNS width
}

if (param_2 != 0x1e0) {
    sVar2 = *(short *)(DAT_00b0fa54 + 4); // LWSCRNL height
} else {
    sVar2 = *(short *)(DAT_00b0fae8 + 4); // LWSCRNS height
}
```

Why this matters: the asset chosen for width and the asset chosen for height are controlled by separate comparisons (`screen_w == 640`, `screen_h == 480`). Standard shell modes usually pair them, but odd/custom sizes can produce mixed dimensions in the computed rect.

The origin for both parent overlay and lower/side rects uses:

```c
local_c = 0;
if (screen_w > 1023) {
    local_c = (screen_w - 800) / 2;
}
iVar6 = 0;
if (screen_h > 767) {
    iVar6 = (screen_h - 600) / 2;
}
```

So at 1024x768, the common centered shell origin is `(112, 84)`.

Confidence: High.

### 8. Owner-draw PCX button/flag palette conclusion remains unchanged

Evidence:

- previous owner-draw reports
- current `OwnerDraw_Button_00612B70 @ 0x00612B70`

The button PCX path formats filenames such as `b%c%c_li%d.pcx`, loads PCX surfaces, and blits those surfaces. This pass found no evidence that right-panel SHP palettes (`SHELL.PAL`, `SHELL2.PAL`, `SDBTNANM.PAL`) are used for the owner-draw PCX button/flag assets.

Active standard button PCXs remain:

- unpressed: `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`
- pressed: `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`
- disabled normal path: unpressed asset triplet with alpha `0x80`; no normal `bud_*` substitution in the verified path

Flag PCXs remain the country/observer/random PCX assets mapped in the prior report.

Confidence: High for button assets and alpha; High that these PCXs are not right-panel SHP-palette users.

### 9. Preview surface/source-bounds status

Evidence:

- `DrawStartPositions @ 0x00640710`
- string xrefs for `Preview`, `PreviewPack`, `Waypoints`, and `LocalSize`
- decompile around `0x006418D8` / `0x00641A78` preview pack generation
- decompile `FUN_0068BDC0` waypoint reader
- decompile `ScenarioClass__Full_Init @ 0x00687502`

This pass confirms more surrounding evidence but does not fully resolve preview projection source bounds.

Verified:

- `DrawStartPositions` still draws preview surface first, then `STARTBUT.SHP` frame 0, then labels.
- Available-start marker count comes from `ScenarioClass + 0x113C`.
- Marker coordinate pairs are read from `ScenarioClass + 0x1140 + i * 8` and `+0x1144 + i * 8`.
- Projection subtracts source origin fields `ScenarioClass + 0x112C` and `+0x1130`, divides by source width/height fields `+0x1134` and `+0x1138`, then scales into the preview destination.
- `[Waypoints]` is read in `FUN_0068BDC0`, storing up to `0x2BE` waypoint pairs starting at `param_1 + 0x632`.
- `[Preview]` and `[PreviewPack]` strings are used by the preview pack generation/decompression path around `0x006418D8`.
- `[Map] LocalSize` is read during full scenario init and passed to radar map bounds computation, but this pass did not prove it is the exact source for the menu preview fields `+0x112C..+0x1138`.

Inference:

- `[Map] LocalSize` remains a plausible contributor to preview/radar bounds, but it must not be treated as the verified source for menu preview marker projection.

Confidence: High for `DrawStartPositions` field usage and draw order; Medium for preview pack generation interpretation; Low for any direct `LocalSize -> menu preview source bounds` claim.

## Corrected Active Asset Matrix For Offline Skirmish `0x102`

| Asset | Active in standard offline Skirmish? | Role | Palette/convert path |
|---|---:|---|---|
| `MNSCRNS.SHP` | Yes at width 640 | parent/background SHP selected by `Background_Overlay` | parent convert `FUN_0072D030()` path; previous report ties Skirmish background convert to `MnScrnLCoopGameSetup.PAL`, but note this small parent pointer comes from common shell loader |
| `MNSCRNL.SHP` | Not as dialog `0x102` parent at 640 | generic large parent/corner background global `DAT_00B0FA04`; used by other common-shell helpers/layout dimensions | common shell loader |
| `MnScrnLCoopGameSetup.shp` | Yes at width 800 | offline Skirmish alternate parent background | `MnScrnLCoopGameSetup.PAL` |
| `MnScrnLCoopGameSetup.PAL` | Yes | Skirmish parent alternate/background convert path | `DAT_00B0FCDC` raw, `DAT_00B0FCE0` convert |
| `SDTP.SHP` | Yes | right-panel top: frame 0 from `RightPanel__Draw`; frame 1 overlay from `Sidebar_TopHighlight` for `0x102` | `SHELL.PAL` |
| `SDBTNBKGD.SHP` | Yes | right-panel repeated tile | `SHELL2.PAL` |
| `SDBTNANM.SHP` | Conditional | right-panel repeated frame-10 overlay | `SDBTNANM.PAL` |
| `SDBTM.SHP` | Yes | right-panel bottom cap | `SHELL.PAL` |
| `LWSCRNS.SHP` | Yes at width 640 | lower/side strip | `SHELL.PAL` |
| `LWSCRNL.SHP` | Yes at non-640 widths | lower/side strip | `SHELL.PAL` |
| `SDMPBTN.SHP` | Yes for standard offline `0x102` when `FUN_0060C930` sets the flag | lower Battle/map-name right-panel chrome via `Minimap_Button`; not the preview bitmap/backing | `SHELL.PAL` / common helper draw site |
| `SDWRNTMP.SHP` | Loaded by common shell loader | generic warning/temp shell asset; not established as standard Skirmish first-paint surface | common shell loader |
| `STARTBUT.SHP` | Yes after real preview surface | available-start marker, frame 0 | right-panel/common SHP palette path from marker loader context remains separate from parent/background path |
| `mmpb.shp` | Active elsewhere, not standard first-paint preview | assigned-player/house marker path | separate preview/marker path |
| `bue_li30/mi30/ri30.pcx` | Yes | unpressed owner-draw buttons | embedded PCX palette/surface |
| `bde_li30/mi30/ri30.pcx` | Yes | pressed owner-draw buttons | embedded PCX palette/surface |
| `bud_*` button PCXs | No evidence in normal path | not used for normal disabled Start/Choose/Back rendering | n/a |
| flag PCXs | Yes | country/random/observer flag rows | embedded PCX palette/surface |
| `MNSCRNL.SHP` as 640 Skirmish parent | No | previous-report mistake | n/a |
| `MNSCRNS.SHP` as generic only | No | previous-report mistake; it is active at 640 | see above |
| `MnScrnLCustomizeBattle.shp` | No standard offline `0x102` evidence | broader/customization shell asset | not active for standard offline Skirmish first paint |
| `dbak6440.pcx` | Generic fallback/preload only | non-mode-1/2 fallback in `WM_PAINT_Handler` | PCX path |
| `dlgsysa.pcx`, `dlgsysi.pcx` | Generic preload only in this scope | owner-draw/system preload pool | PCX path |

## Implementation Guidance

1. Correct the 640 parent background role from `MNSCRNL.SHP` to `MNSCRNS.SHP`.
2. Keep `MNSCRNL.SHP` available only if implementing generic/large common-shell helpers, not as the verified 640 Skirmish parent.
3. Add separate right-panel lower/side entries for `LWSCRNS.SHP` and `LWSCRNL.SHP`; do not fold them into the parent background.
4. Use right-panel palette paths per asset:
   - `SHELL.PAL`: `SDTP.SHP`, `SDBTM.SHP`, `LWSCRNS.SHP`, `LWSCRNL.SHP`
   - `SHELL2.PAL`: `SDBTNBKGD.SHP`
   - `SDBTNANM.PAL`: `SDBTNANM.SHP` frame 10
5. Keep `MnScrnLCoopGameSetup.shp` as the verified 800 parent background and decode it through `MnScrnLCoopGameSetup.PAL`.
6. Keep `>800` parent background blank/no-draw for fresh Skirmish entry unless runtime watchpoints prove a non-null `DAT_00B0FA18` can enter the draw path.
7. Do not use `SDMPBTN.SHP`, `mmpb.shp`, `dbak6440.pcx`, or `dlgsys*.pcx` as the preview bitmap/backing. `SDMPBTN.SHP` must still be drawn as its own right-panel chrome at `DAT_00B0FC14` for standard `0x102`.
8. Do not populate preview marker source bounds from `[Map] LocalSize` until a direct write chain to `ScenarioClass + 0x112C..+0x1138` or a retail screenshot comparison verifies it.

## Open Questions

1. Runtime watchpoint evidence for stale non-null `DAT_00B0FA18` at `>800` remains open. Static evidence says normal cleanup clears it and the Skirmish loader only loads it at exact width 800.
2. The semantic meaning of the `RightPanel__Draw` caller flag controlling `SDBTNANM.SHP` frame-10 overlay is not fully named. The binary branch is verified; state transitions of `piVar9[0x35]` are not.
3. Exact `STARTBUT.SHP` palette/convert binding was not re-opened in this pass beyond confirming marker order. It should remain tied to the previously verified marker path, not to parent/background palettes.
4. Exact preview source bounds remain unresolved. `[Map] LocalSize` is not yet proven as the source for `DrawStartPositions` projection fields.

## Sources Checked

- Ghidra live decompile/disassembly:
  - `0x0060CF00`
  - `0x00621E90`
  - `0x00640710`
  - `0x00640A40`
  - `0x0068BDC0`
  - `0x00687502`
  - `0x0072ADE0`
  - `0x0072CF40`
  - `0x0072CF90`
  - `0x0072DFB0`
  - `0x0072E260`
  - `0x0072E280`
  - `0x0072E2A0`
  - `0x0072E2C0`
  - `0x0072E450`
  - `0x0072E730`
  - `0x0072E820`
  - `0x0072EC70`
  - `0x004AED70`
- Ghidra memory/string evidence:
  - SHP string block `0x00845104..0x00845188`
  - SHP pointer table `0x00844CD4..0x00844CF8`
  - PAL string block `0x00845438..0x00845524`
  - PAL pointer table `0x00844BE4..0x00844BEC`
  - strings `Preview`, `PreviewPack`, `Waypoints`, `LocalSize`
- Prior reports:
  - `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`
  - `SKIRMISH_SHELL_BACKGROUND_OVERLAY_PLACEMENT_FOLLOWUP_GHIDRA_REPORT.md`
  - `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
  - `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
