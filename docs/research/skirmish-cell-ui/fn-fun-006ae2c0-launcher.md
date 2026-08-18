# FUN_006AE2C0 — Standard Offline Skirmish Launcher

## Summary

`FUN_006AE2C0` is the top-level launcher for the offline Skirmish setup
dialog (resource ID 0x102). It is called once from `Main_Game` when the player
selects Skirmish from the main menu. It sets up house-type data, creates the
modeless dialog, runs the dialog's message pump loop until the player clicks
Start (0x617) or Back (0x5C0), tears down the dialog, cleans up the random-map
object, and returns `true` if the user clicked Start (to trigger game launch)
or `false` if they clicked Back.

Active in YR: **Yes** — called directly from `Main_Game` at `0x0052E168`
via `get_xrefs_to 0x006AE2C0`. `Main_Game` is the primary game loop. No
TS-gating flag present. Active in every standard YR skirmish session.
(verified via `get_function_callers 0x006AE2C0`)

## Address

`0x006AE2C0`
(verified via `decompile_function 0x006AE2C0`)

## Signature / Return Value

```c
bool FUN_006ae2c0(void)
// returns: true  → user clicked Start (0x617) — game launch proceeds
//          false → user clicked Back (0x5C0) — returns to main menu
```

(verified via `decompile_function 0x006AE2C0` — `return local_4 == 0x617`)

## Callers

- `Main_Game` @ `0x0052D9A0` — game main loop; calls this launcher when
  entering skirmish setup.
  (verified via `get_function_callers 0x006AE2C0` + `get_xrefs_to 0x006AE2C0`
  showing UNCONDITIONAL_CALL from `0x0052E168 in Main_Game`)

## Callees

(verified via `get_function_callees 0x006AE2C0`)

| Function | Address | Role |
|---|---|---|
| `FUN_006722F0` | 0x006722F0 | House-type setup (arg: `DAT_00887048`) |
| `FUN_00672440` | 0x00672440 | House-type secondary setup (arg: `DAT_00887048`) |
| vtable call via `g_HouseTypeClass_Array` | — | Per-house vtable call at offset `+100` (dec) for each house type |
| `FUN_0072CF40` | 0x0072CF40 | Pre-dialog setup (unknown; called immediately before dialog create) |
| `FUN_00622650` | 0x00622650 | Dialog factory — `CreateDialogIndirectParamA` wrapper; creates dialog 0x102, returns HWND |
| `SetWindowLongA` | EXTERNAL | Stores `&local_4` as dialog extra-data pointer at GWL_USERDATA (offset 8) |
| `FUN_00622800` | 0x00622800 | Shows / initialises the dialog (ShowWindow-equivalent) |
| `FUN_00623120` | 0x00623120 | Message pump tick — returns `\x01` when done |
| `FUN_00532100` | 0x00532100 | Game tick helper called each pump iteration |
| `FUN_00622720` | 0x00622720 | Hides / destroys dialog |
| `FUN_006406F0` | 0x006406F0 | Frees random-map object |
| `FUN_007C8B3D` | 0x007C8B3D | CRT free |
| `FUN_0072CF90` | 0x0072CF90 | Post-dialog teardown |
| `FUN_006990A0` | 0x006990A0 | Session/flag cleanup |
| `BSurface__Constructor` | 0x0052FEC0 | Surface reset (called only on Back path) |

## Behavioral Analysis

### 1 — House-type data setup

```c
FUN_006722F0(DAT_00887048);
FUN_00672440(DAT_00887048);
for (iVar2 = 0; iVar2 < g_HouseTypeClass_Array_Count; iVar2++) {
    (**(code **)(**(int **)(g_HouseTypeClass_Array + iVar2 * 4) + 100))(DAT_00887048);
}
```

`g_HouseTypeClass_Array` is the array of `HouseTypeClass*` pointers. For each
house type, the vtable method at offset `+100` (decimal, i.e. `+0x64`) is
invoked with `DAT_00887048`. This populates country/side data that the combo
boxes will later enumerate.

(verified via `decompile_function 0x006AE2C0`)

### 2 — Pre-dialog setup

`FUN_0072CF40()` is called immediately before dialog creation. Purpose not
decoded in this task (out-of-scope); likely palette/font/resource preparation.

### 3 — Dialog creation

```c
hWnd = (HWND)FUN_00622650(0);
```

`FUN_00622650` is a `CreateDialogIndirectParamA` wrapper. Called with
`param_1 = 0` (the first argument is the dialog resource ID `ushort`), which
corresponds to the dialog template ID 0x102 (the skirmish dialog). The wrapper
selects a template via `FUN_004A3B40()`, increments an internal dialog-stack
counter (`DAT_00B72F50`), and registers the HWND in an array at `DAT_00B72D28`.
The DlgProc (`FUN_006AE3F0`) is registered at the OS level via
`CreateDialogIndirectParamA`.

Note: `FUN_00622650` is typed `(ushort param_1, DLGPROC param_2, undefined4 param_3)`
but at this call site only `param_1 = 0` is passed (Ghidra shows one explicit
arg); `param_2` and `param_3` default to zero/null in the fastcall calling
convention.

(verified via `decompile_function 0x00622650`)

### 4 — Dialog extra-data pointer

```c
if (hWnd != NULL) {
    DAT_00B0B59C = hWnd;
    SetWindowLongA(hWnd, 8, (LONG)&local_4);
    ...
}
```

`local_4` is initialised to `-1` and holds the dialog exit code. Storing its
address via `GWL_USERDATA` (offset 8) allows the DlgProc (`FUN_006AE3F0`) and
the WM_COMMAND handler (`FUN_006ACEE0`) to write back the exit code (0x617 for
Start, 0x5C0 for Back). The loop condition `local_4 != 0x617 && local_4 != 0x5C0`
checks this value each pump tick.

`DAT_00B0B59C` is the global "current skirmish dialog HWND"; cleared to
`NULL` after the loop exits.

(verified via `decompile_function 0x006AE2C0`)

### 5 — Message pump loop

```c
FUN_00622800();
while (local_4 != 0x617 && local_4 != 0x5C0) {
    if (FUN_00623120() == '\x01') break;
    FUN_00532100();
}
FUN_00622720();
DAT_00B0B59C = NULL;
```

- `FUN_00622800()` — initial dialog show/paint call.
- `FUN_00623120()` — message-pump tick; returns `\x01` (1) when a quit/abort
  condition is detected externally (e.g., Alt-F4, game shutdown).
- `FUN_00532100()` — one game-logic tick per pump iteration; keeps background
  systems alive while the dialog is visible.
- Loop exits when `local_4` is written to 0x617 or 0x5C0 by the DlgProc's
  WM_COMMAND handler, OR when the pump signals abort.
- `FUN_00622720()` — destroys the dialog; decrements the internal dialog-stack
  counter.

(verified via `decompile_function 0x006AE2C0`)

### 6 — Random-map object cleanup

```c
iVar2 = DAT_00AC1154;
if (DAT_00AC1154 != 0) {
    FUN_006406F0();
    FUN_007C8B3D(iVar2);
    DAT_00AC1154 = 0;
}
```

Frees the random-map preview object if it was allocated during the session.
`DAT_00AC1154` is the same random-map object pointer used by the WM_COMMAND
handler (`FUN_006ACEE0`).

(verified via `decompile_function 0x006AE2C0`)

### 7 — Back-button path

```c
if (local_4 == 0x5C0) {
    BSurface__Constructor();
}
```

When Back is pressed, `BSurface__Constructor` (`0x0052FEC0`) is called to
reset a surface object. This likely resets the game surface to the main-menu
state. Not called on Start.

(verified via `decompile_function 0x006AE2C0`)

### 8 — Return value

```c
return local_4 == 0x617;
```

`true` if Start was pressed; `false` if Back was pressed or the pump was
aborted.

(verified via `decompile_function 0x006AE2C0`)

## Observed Globals

| Global | Address | Access | Role |
|---|---|---|---|
| `DAT_00887048` | `0x00887048` | READ (passed to house-setup) | House-type registry or game options block |
| `g_HouseTypeClass_Array` | unknown symbol | READ | Array of `HouseTypeClass*` |
| `g_HouseTypeClass_Array_Count` | unknown symbol | READ | Array element count |
| `DAT_00B0B59C` | `0x00B0B59C` | WRITE | Current skirmish dialog HWND; cleared on exit |
| `DAT_00AC1154` | `0x00AC1154` | READ/WRITE | Random-map object ptr; shared with `FUN_006ACEE0` |

## Out-of-scope refs

- `FUN_006722F0` / `FUN_00672440` — house-type setup; not in cell-UI scope
- vtable slot at `HouseTypeClass + 0x64` — out of scope
- `FUN_0072CF40` / `FUN_0072CF90` — pre/post-dialog setup; not in scope
- `FUN_00622650` — dialog factory (shared infrastructure); not in scope
- `FUN_00622800` / `FUN_00622720` / `FUN_00623120` — dialog show/hide/pump; not in scope
- `FUN_00532100` — game tick helper; not in scope
- `BSurface__Constructor` — surface management; not in scope
- `FUN_006990A0` — session/flag cleanup; not in scope
- `FUN_006406F0` / `FUN_007C8B3D` — map-preview free / CRT free; not in scope
- `DAT_00887048` — house options block; not decoded

## Unverified (YELLOW)

- The exact dialog resource ID passed to `FUN_00622650` at `param_1 = 0` may
  differ from 0x102: Ghidra shows only one explicit arg `0` at the call site;
  the actual dialog ID selection may occur inside `FUN_004A3B40` using a
  different mechanism. The mapping `0 → dialog 0x102` is inferred from context
  (all skirmish-system docs refer to dialog 0x102 as the offline skirmish
  dialog) but not independently verified against the template selection code.
- The semantic meaning of `FUN_0072CF40` / `FUN_0072CF90` and `FUN_006990A0`
  is inferred from call-site position (pre/post-dialog); not decompiled in
  this task.
- `g_HouseTypeClass_Array` and `g_HouseTypeClass_Array_Count` addresses are
  unknown — Ghidra shows symbolic names in the decompilation but the actual
  addresses were not resolved in this decode pass.
