# RT_DIALOG Resource `0x120` - Byte Layout Report

Date: 2026-05-19

Scope: re-parse the `gamemd.exe` PE resource template `RT_DIALOG / 0x120`
directly from the `.rsrc` section bytes, producing a control-by-control
layout table (IDs, rects, classes, styles, dialog font, overall rect).
Cross-checked against the two consumers `FUN_005D3490` (modal helper) and
`FUN_005D36A0` (dialog proc) already documented behaviorally in
`QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`.

Companion templates `0x121` (3-button) and `0xCE` (no-button) are OUT OF
SCOPE; see Open Questions for one observation that surfaced.

No Rust code, INI, or Ghidra annotations were modified. Ghidra MCP was
used read-only (`list_segments`, `read_memory`, `decompile_function`,
`search_byte_patterns`-style verification only).

## Executive Summary

`0x120` is a plain `DLGTEMPLATE` (NOT the extended `DLGTEMPLATEEX` form -
`dlgVer/signature` magic does not match). The template declares **four
items**, not three: one `Static` and three `BS_OWNERDRAW` buttons. The
prior doc's phrasing "OK-only template" describes which slots the
quit-confirm caller populates, not the physical control count.

Overall dialog: `300 x 200` DLU, `WS_CHILD | DS_SETFONT`, font
`MS Sans Serif 8pt`, no title, no menu, no window-class override.

Verified facts:

1. PE resource path: `RT_DIALOG(5) -> 0x120 -> Lang 0x0409 (en-US) ->
   Data RVA 0x00800B24, Size 0x000000E8 (232 bytes)`.
2. Live data lives at VA `0x00C00B24` (image base `0x00400000`, segment
   `.rsrc` `0x00B7A000 - 0x00C03FFF`).
3. Header DWORD style `0x40000040 (WS_CHILD | DS_SETFONT)`; `dwExtendedStyle 0`;
   `cdit 4`; rect `0,0,300,200`; font `MS Sans Serif`, 8pt.
4. Controls (in template order): `id=2 Button OWNERDRAW @ (207,175,83,15)`,
   `id=0x5B0 Static @ (40,40,220,50)`,
   `id=0x5AE Button OWNERDRAW @ (207,135,83,15)`,
   `id=0x5AF Button OWNERDRAW @ (207,155,83,15)`.
5. Each control's title field is a real CSF key prefilled at template-build
   time (`"GUI:Cancel"`, `"GUI:Blank"`, `"GUI:OK"`, `"GUI:Blank"`); the
   modal helper overwrites these via `SendMessage(WM_USER+something=0x4B2,
   ...)` at runtime, so the template-baked text is the fallback if the
   caller leaves a slot NULL.

Confidence: High (full byte parse cross-validated against both consumer
decompiles). YR-active: Yes - `FUN_005D3490` is called from 27 sites in
gamemd, including the verified main-menu Quit path; this template is the
OK-only branch (`param_4 == NULL`) selected at `0x005D3535`.

## Resource Directory Walk (PE `.rsrc`)

Section listed by `list_segments`:

```
.rsrc:  0x00B7A000 - 0x00C03FFF
```

Resource root (at `0x00B7A000`), 64 bytes:
- 0 named entries, 7 ID entries.
- Type entries: `1(Cursor)`, `3(Icon)`, `4(Menu)`, `5(Dialog)`, `12(GroupCursor)`,
  `14(GroupIcon)`. Type `5 (RT_DIALOG)` sub-dir offset = `0x80000120`
  -> `.rsrc + 0x120` = `0x00B7A120`.

RT_DIALOG sub-dir at `0x00B7A120`:
- 0 named, 98 ID entries (0x62).
- Walked the 98 entries (each `DWORD id | DWORD offset|0x80000000`); found
  ID `0x00000120` at the 73rd entry with offset `0x80000E60`
  -> `.rsrc + 0xE60` = `0x00B7AE60`.

Language sub-dir at `0x00B7AE60`:
- 1 ID entry: `LangID = 0x0409 (en-US)`, offset `0x000016D8` (no high bit ->
  Data Entry) -> `.rsrc + 0x16D8` = `0x00B7B6D8`.

Resource Data Entry at `0x00B7B6D8` (16 bytes):
- `OffsetToData (RVA)` = `0x00800B24`
- `Size`               = `0x000000E8` (232 bytes)
- `CodePage`           = `0`
- `Reserved`           = `0`

Image base `0x00400000` + RVA `0x00800B24` -> VA `0x00C00B24`. That is
where the `DLGTEMPLATE` blob lives.

## `DLGTEMPLATE` Header (bytes `0x00 - 0x37` of the blob)

```
offset  bytes              field           value
00      40 00 00 40        style           0x40000040  (WS_CHILD|DS_SETFONT)
04      00 00 00 00        dwExtendedStyle 0x00000000
08      04 00              cdit            4
0A      00 00              x               0
0C      00 00              y               0
0E      2C 01              cx              300
10      C8 00              cy              200
12      00 00              menu            0  (no menu)
14      00 00              windowClass     0  (default dialog class)
16      00 00              title           "" (empty)
18      08 00              pointsize       8   (DS_SETFONT present)
1A..35  4D 00 .. 66 00 00 00   typeface    "MS Sans Serif" + NUL
36      00 00              padding to DWORD boundary (offset 0x38)
```

Style bits decoded: only `WS_CHILD (0x40000000)` and `DS_SETFONT (0x40)`
set. Notably absent: `WS_POPUP`, `WS_VISIBLE`, `WS_CAPTION`, `DS_MODALFRAME`,
`DS_CENTER`, `DS_SETFOREGROUND`. This is consistent with the dialog being
hosted as a child of the engine's rendering window and being centered /
made visible programmatically by `FUN_00622650` / `FUN_00622800`.

## Control Item Table (4 items, in template order)

All items are `DLGITEMTEMPLATE` (not `EX`). Each follows
`{ DWORD style; DWORD exStyle; short x; short y; short cx; short cy;
WORD id; sz_Or_Ord class; WCHAR title[]; WORD creationDataLen; }`,
DWORD-aligned between items.

### Item 1 - id `2` ("Cancel" slot in the helper)

```
offset  bytes              field        value
38      0B 00 00 50        style        0x5000000B  WS_CHILD|WS_VISIBLE|BS_OWNERDRAW
3C      00 00 00 00        exStyle      0
40      CF 00              x            207
42      AF 00              y            175
44      53 00              cx           83
46      0F 00              cy           15
48      02 00              id           0x0002
4A      FF FF 80 00        class        Button (ordinal 0x0080)
4E..63  G U I : C a n c e l NUL   title  L"GUI:Cancel"
64      00 00              creationData 0
66      00 00              padding to DWORD boundary (next item at 0x68)
```

Notes:
- Bottom-right of the dialog (`x+cx = 290 of 300`, `y+cy = 190 of 200`).
- The dialog proc `FUN_005D36A0` maps `id == 1 || id == 2 -> result 1`.
  `id 1` does not exist in this template; the `id == 1` arm is dead in the
  `0x120` path but live in callers that build `IDOK`-style synthesized
  commands (e.g., default-button keyboard handling). See Open Question #4.

### Item 2 - id `0x5B0` (prompt / title text sink)

```
offset  bytes              field        value
68      00 00 00 50        style        0x50000000  WS_CHILD|WS_VISIBLE
6C      00 00 00 00        exStyle      0
70      28 00              x            40
72      28 00              y            40
74      DC 00              cx           220
76      32 00              cy           50
78      B0 05              id           0x05B0
7A      FF FF 82 00        class        Static (ordinal 0x0082)
7E..91  G U I : B l a n k NUL  title    L"GUI:Blank"
92      00 00              creationData 0
```

Notes:
- Style low bits `0x0` mean `SS_LEFT` (left-aligned text), no
  `SS_OWNERDRAW (0x0D)`, no `SS_CENTER (0x1)`. Static rendered by the
  default USER32 path, not owner-draw.
- 220x50 DLU upper region centered horizontally with margins of ~40 DLU
  left/right (`40+220 = 260` vs `cx = 300`; ~40 right margin).
- Receives the runtime prompt text via `SendMessageA(hStaticChild, 0x4B2,
  0, csfText)` from `FUN_005D3490` when `param_1` is non-NULL.

### Item 3 - id `0x5AE` (body / "TXT_OK" slot, confirm-quit click)

```
offset  bytes              field        value
94      0B 00 00 50        style        0x5000000B  WS_CHILD|WS_VISIBLE|BS_OWNERDRAW
98      00 00 00 00        exStyle      0
9C      CF 00              x            207
9E      87 00              y            135
A0      53 00              cx           83
A2      0F 00              cy           15
A4      AE 05              id           0x05AE
A6      FF FF 80 00        class        Button (ordinal 0x0080)
AA..B7  G U I : O K NUL    title        L"GUI:OK"
B8      00 00              creationData 0
BA      00 00              padding to DWORD (next item at 0xBC)
```

Notes:
- Same `x/cx` column as items 1 and 4 (right-side button stack).
- `y = 135`, top of the button stack; 20 DLU vertical gap to item 4.
- Click maps to `result = 0` in `FUN_005D36A0`. For the main-menu
  quit-confirm caller, this is the actual confirm-quit click.

### Item 4 - id `0x5AF` (middle button slot)

```
offset  bytes              field        value
BC      0B 00 00 50        style        0x5000000B  WS_CHILD|WS_VISIBLE|BS_OWNERDRAW
C0      00 00 00 00        exStyle      0
C4      CF 00              x            207
C6      9B 00              y            155
C8      53 00              cx           83
CA      0F 00              cy           15
CC      AF 05              id           0x05AF
CE      FF FF 80 00        class        Button (ordinal 0x0080)
D2..E5  G U I : B l a n k NUL  title    L"GUI:Blank"
E6      00 00              creationData 0
```

Notes:
- 20 DLU below item 3, 20 DLU above item 1.
- Click maps to `result = 2` in `FUN_005D36A0`.
- For the quit-confirm caller (`param_4 = NULL`), no `SendMessageA 0x4B2`
  is ever issued for this control, so its owner-draw routine receives no
  CSF text. It is still `WS_VISIBLE` and clickable per the template
  bits; the owner-draw paint routine is presumed to short-circuit on
  empty/NULL text. This was NOT verified at the paint level - see Open
  Question #2.

## Cross-Check Against Consumers

`FUN_005D3490` (modal helper, decompiled this session) reads template
items by id, matching the parsed template exactly:

```
GetDlgItem(hwnd, 0x5b0)  if param_1 (prompt)        -> Item 2 (Static)
GetDlgItem(hwnd, 0x5ae)  if param_2 (body button)   -> Item 3 (Button)
GetDlgItem(hwnd, 2)      if param_3 (cancel button) -> Item 1 (Button)
GetDlgItem(hwnd, 0x5af)  if param_4 (3rd button)    -> Item 4 (Button)
```

Template selection at `0x005D3535`:

```
0x005D351D  MOV ECX, 0xCE      (default: no-button template)
0x005D3528  MOV ECX, 0x121     (full 3-button template)
0x005D3535  MOV ECX, 0x120     (OK-only template, selected when
                                param_4 == NULL but param_3 non-NULL)
```

Byte-pattern verification at `0x005D351D-0x005D353C`:
`B9 CE 00 00 00 84 C0 74 07 B9 21 01 00 00 EB 0D 8A 44 24 0C 84 C0 74 05 B9 20 01 00 00 6A 00 BA`
(`B9 nn 00 00 00` = MOV ECX, imm32; followed by selection logic.)

`FUN_005D36A0` (dialog proc) handles `WM_COMMAND (0x111)`:

```
id == 0x5AE && notify == 0 -> *resultPtr = 0       (Item 3 click)
id == 0x5AF && notify == 0 -> *resultPtr = 2       (Item 4 click)
(id == 1 || id == 2) && notify == 0 -> *resultPtr = 1   (Item 1 click;
                                                          id 1 unused
                                                          in template)
```

Both functions reference exactly the IDs found in the template. No
template-side ID is unreferenced; no consumer-side ID lacks a template
control except the synthesized `id == 1` arm (Open Q #4).

## TS-vs-YR Filter

- `FUN_005D3490` is called from 27 sites across gamemd (verified by prior
  doc's `get_function_callers` run); none is gated by `SpecialFlags` or
  TS-only flags. The Quit-confirm caller (`Main_Game @ 0x0052DE1C`,
  case-6 of the main-menu state switch) is YR-live in every game mode.
- Template `0x120` is referenced as the immediate `MOV ECX, 0x120` only
  inside `FUN_005D3490`. No TS-only branch reaches it.
- The 4-control template (including the unused `0x5AF` in the OK-only
  caller path) is a generic re-use design - the same resource serves
  both this caller and the lone-OK-button callers; nothing here is
  dormant TS code.
- The static control `id 0x5B0` uses the default USER32 paint path
  (no `SS_OWNERDRAW`), so it WILL get the engine's font rendering only
  if the parent window's `WM_CTLCOLORSTATIC` handler in `FUN_00622B50`
  customizes the brush/font. That is a question for the common shell
  proc, not for this template doc.

YR-active: Yes.

## Implementation Implications

For pixel-faithful re-implementation of the quit-confirm dialog (and any
sibling caller of `FUN_005D3490` selecting `0x120`):

1. Dialog rect is **300 x 200 DLU**, not pixels. Convert with the engine's
   chosen base font metrics (MS Sans Serif 8pt -> typical 1 DLU horiz =
   ~1.5 px, 1 DLU vert = ~2 px at 96 DPI, but the engine renders via its
   own pipeline - the actual DLU-to-pixel mapping must match
   `FUN_00622650` / `FUN_00622800`, which is out of scope here).
2. Three buttons stack right-aligned at `x = 207, cx = 83`, with
   `y = 135 / 155 / 175`. Vertical pitch 20 DLU, button height 15 DLU.
3. Static prompt is at `(40, 40, 220, 50)` - top-centered region with
   ~40 DLU left margin and ~40 DLU right margin.
4. All three buttons are `BS_OWNERDRAW`; the engine's button paint code
   (not researched here) is responsible for the cameo/skinning. A native
   Win32 owner-draw button would draw nothing without a paint handler.
5. The static is NOT owner-draw; default USER32 text rendering applies.
   If the rest of the engine wants consistent skinning, it must override
   via `WM_CTLCOLORSTATIC` in the parent.
6. The CSF key bake (`"GUI:Cancel"`, `"GUI:OK"`, `"GUI:Blank"`,
   `"GUI:Blank"`) embedded in the resource is the FALLBACK label when a
   caller leaves a slot NULL. A faithful re-impl can use the same CSF
   keys as last-resort defaults if it wants to mirror that fallback
   behavior.

## Open Questions

1. The two companion templates `0x121` (full 3-button) and `0xCE`
   (no-button) were OUT OF SCOPE for this slot. The resource directory
   was indexed but their data entries were not parsed. Worth a follow-up:
   does `0x121` extend `0x120`'s 4-control layout with a different
   geometry, or share it byte-for-byte and only differ in caller-driven
   `SendMessage 0x4B2` distribution?
2. Item 4 (`0x5AF`) is `WS_VISIBLE` in the template but receives no
   runtime text in the OK-only caller path. Whether the owner-draw paint
   routine actually leaves an empty cameo / blank rect or short-circuits
   was NOT inspected here. The pre-baked title text `L"GUI:Blank"` may
   be the static fallback if the paint routine falls back to the
   `GetWindowText` value when no message-baked text exists.
3. The `id == 1` arm in `FUN_005D36A0` (which writes `result = 1`) has
   no corresponding template control. It is presumably exercised by
   default-button / keyboard `IDOK` synthesis from a `WM_COMMAND` send,
   most likely from the modal pump `FUN_00623120` or from the ESC-key
   path. NOT verified in this slot; the prior quit-confirm doc flagged
   ESC behavior as an open question.
4. The dialog has no `WS_CAPTION`, no `WS_BORDER`, no `WS_DLGFRAME`,
   no `DS_MODALFRAME`. Visual chrome (window border, drop shadow, title
   bar) is entirely owner-drawn by the engine's common shell. That
   chrome layer was not inspected in this report.
5. The DLU-to-pixel conversion the engine uses is unverified at this
   level. `FUN_00622650` (wraps `CreateDialogIndirectParamA`) and
   `FUN_00622800` (the show/center/skin step) likely apply a custom
   metric. A pixel-faithful port needs to verify which metric they use.

## Sources

PE walk (read-only, this session):

- `list_segments` -> `.rsrc 0x00B7A000-0x00C03FFF`.
- `read_memory(0x00B7A000, 64)` -> resource root.
- `read_memory(0x00B7A120, 16)` -> RT_DIALOG type sub-dir header
  (0 named, 98 ID).
- `read_memory(0x00B7A130, 784)` -> 98 RT_DIALOG ID entries; located
  `id=0x120 -> offset 0x80000E60`.
- `read_memory(0x00B7AE60, 24)` -> Language sub-dir; located
  `Lang 0x0409 -> data RVA 0x00800B24, size 0xE8`.
- `read_memory(0x00B7B6D8, 16)` -> resource data entry.
- `read_memory(0x00C00B24, 232)` -> the `DLGTEMPLATE` blob itself,
  parsed in full above.

Consumer cross-check (read-only):

- `decompile_function(0x005D3490)` -> `FUN_005D3490`, modal helper.
- `decompile_function(0x005D36A0)` -> `FUN_005D36A0`, dialog proc.
- `read_memory(0x005D351D, 32)` -> verified `MOV ECX, 0x120` selection
  immediate at `0x005D3535`.

Prior reports referenced:

- `docs/research/QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`
  - This report fills its "Resource Template `0x120` Note" open item
    (section starts at line ~228 of that doc). Outcome: that doc
    correctly identified the IDs (`0x5B0`, `0x5AE`, `0x5AF`, `2`) and
    message routing; the new finding is that the template defines **4
    controls including `0x5AF`** (not 3), with `BS_OWNERDRAW` on every
    button and default `SS_LEFT` on the static.
