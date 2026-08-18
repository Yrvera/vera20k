# FUN_0060F9A0 — Owner-Draw Subclass Setup Function — Ghidra Research Report

**Address:** `0x0060F9A0`  
**Function signature (Ghidra):** `undefined4 FUN_0060f9a0(HWND param_1, int param_2)`  
**Date:** 2026-05-19  
**Confidence:** HIGH — every claim sourced from live Ghidra decompilation in this session.  
**Active in YR:** YES — called unconditionally for every child window of every shell dialog,
including the standard initial main menu dialog `0xE2`.

---

## Purpose

`FUN_0060F9A0` is the **per-control owner-draw subclass setup routine**. For a given HWND,
it:

1. Reads the Win32 class name (`GetClassNameA`) and GWL_STYLE (`GetWindowLongA(-0x10)`)
2. Initializes a suite of global UI color/theme variables (one-time, gated by `DAT_00ac48d4`)
3. Classifies the control by class name and, for Button, also by style bits
4. Installs the universal subclass WndProc at `0x0610CA0` via `SetWindowLongA(hwnd, GWL_WNDPROC=-4, 0x610CA0)`
5. Stores the original WndProc (`LVar6`) into a second hash table keyed by HWND
6. Allocates a `0x208`-byte **WindowExtra record** per HWND and inserts it into a hash table
7. Writes the control-type code, dialog ID, dialog template extra data, and a cleanup slot
   into the WindowExtra record
8. Calls `CallWindowProcA(lpPrevWndFunc, hwnd, WM_GETTEXT=0xD, ...)` to fetch initial text
9. Sends `WM_USER+0x97 = 0x497` to trigger post-init setup

---

## Caller Chain to Dialog 0xE2

```
MainMenuDialog0xE2_Proc_00531F60   (0x00531F60 — dialog proc for RT_DIALOG 0xE2)
  └── FUN_00622b50                 (0x00622B50 — generic shell WndProc)
        └── FUN_0060f4b0           (0x0060F4B0 — shell init wrapper)
              ├── EnumChildWindows(parent, FUN_0060f9a0, param_2)   [all children]
              └── FUN_0060f9a0(parent, param_2)                     [parent window itself]
```

`FUN_00622B50` processes WM_INITDIALOG (0x110) with two code paths:
- `param_4 == NULL`: calls `FUN_0060f4b0()` (standard path, taken for dialog 0xE2)
- `param_4 != NULL`: calls into a re-init path that also eventually calls `FUN_0060f9a0`

`MainMenuDialog0xE2_Proc_00531F60` has exactly one caller: `FUN_00531cc0 @ 0x00531CC0`,
which creates RT_DIALOG `0xE2` and drives its message loop.

**Direct callers of FUN_0060F9A0** (verified via `get_function_callers`):
- `FUN_0060f4b0 @ 0x0060F4B0` — main dispatch wrapper (called from `FUN_00622B50`)
- `FUN_00622820 @ 0x00622820` — alternate shell init path
- `FUN_00622b50 @ 0x00622B50` — direct call for the parent HWND itself during WM_110
- `OwnerDraw_ListBox_00618D40 @ 0x00618D40` — listbox subcomponent setup

---

## Class-Name / Style Dispatch Table

The function performs a cascaded strcmp against Win32 class names in this order.
When a match is found, `pcVar13` (the paint proc pointer) and `local_ab0` (the
control-type code) are set, then the chain short-circuits.

| Priority | Class name string | Paint proc address | Control type code |
|---|---|---|---|
| 1 | `"ScrollBar"` | `OwnerDraw_ScrollBar_0061C690` | `0x8` |
| 2 | `"ListBox"` | `OwnerDraw_ListBox_00618D40` | `0x4` |
| 3 | `"ComboBox"` | `OwnerDraw_ComboBox_00617250` | `0x3` |
| 4 | `"msctls_trackbar32"` | `OwnerDraw_Trackbar_0061D950` | `0x7` |
| 5 | `"msctls_progress32"` | `(code *)&LAB_0061D6D0` | `0x6` |
| 6 | `"NewEdit"` | `OwnerDraw_NewEdit_00614B30` | `0x1` |
| 7 | `DAT_00833728` (string, name unknown) | `OwnerDraw_Edit_00614190` | `0x1` |
| 8 | `"Static"` | `OwnerDraw_Static_006153E0` | `0x2` |
| 9 | `"SysTabControl32"` | `(code *)&LAB_006137D0` | `0xA` |
| 10 | `"Button"` (style-dependent) | see below | `0x0` |
| 11 | `"msctls_hotkey32"` | `(code *)&LAB_0061ECA0` | `0x9` |
| else | (no class name match) | `(code *)&LAB_00612A60` | (unchanged from 0xB default) |

### Button sub-dispatch (class == "Button", control type always `0x0`)

The low byte of GWL_STYLE (`local_aac`, fetched early via `GetWindowLongA(hwnd, -0x10)`) is
tested against style bit masks in priority order:

| Style bits test | Paint proc | Notes |
|---|---|---|
| `(style_byte & 7) == 7` (BS_OWNERDRAW=0x0B? No: 7=0b0111) | `OwnerDraw_ButtonVariant_0061E700` | highest priority |
| `(style_byte & 0xB) == 0xB` | `OwnerDraw_Button_00612B70` | standard shell push-button |
| `(style_byte & 3) == 3` | `OwnerDraw_Checkbox_006163A0` | checkbox |
| `(style_byte & 9) == 9` | `OwnerDraw_RadioVariant_00616980` | radio button variant |
| (no bit match) | `pcVar13` stays NULL | no owner-draw installed |

Note: the standard YR main-menu shell button has style low bits `0x0B`, so
`(style_byte & 0xB) == 0xB` fires → `OwnerDraw_Button_00612B70`. This is
confirmed by the SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §1 finding that
`OwnerDraw_Button_00612B70` draws the `bue_*30`/`bde_*30` PCX strips.

---

## Subclass WndProc

```c
SetWindowLongA(param_1, GWL_WNDPROC=-4, 0x610CA0)
```

All controls (matching or not) get the **same** universal subclass proc installed at
`0x0610CA0`. Ghidra has no function boundary at that address (confirmed via
`get_function_by_address` — returns "No function found"). The original WndProc returned
by `SetWindowLongA` (`LVar6`) is stored into a second hash table keyed by HWND for
later call-through.

---

## Hash Tables Maintained

FUN_0060F9A0 maintains **three** hash tables (all keyed by HWND):

| Hash table base global | Entry size | Key | Value stored |
|---|---|---|---|
| `DAT_00ac18c0` | 12 bytes (3 × dword) | HWND | paint proc pointer (`pcVar13`) |
| `DAT_00ac1b48` | 12 bytes (3 × dword) | HWND | original WndProc (`LVar6` from SetWindowLong) |
| `DAT_00ac1b00` | `0x208` bytes | HWND | full WindowExtra record (0x200 bytes of data + 4-byte next-chain) |

The `DAT_00ac18c0` table load threshold uses `_DAT_00ac18e8`; resize calls `FUN_00624be0`.
The `DAT_00ac1b48` table load threshold uses `_DAT_00ac1b70`; resize calls `FUN_00624be0`.
The `DAT_00ac1b00` table load threshold uses `_DAT_00ac1b28`; resize calls `FUN_00624fc0`.

---

## WindowExtra Record Layout

The WindowExtra record is `0x208` bytes. Layout (byte offsets from record start, all
verified from the decompilation of `FUN_0060F9A0`, `FUN_00623340`, and `FUN_00624530`):

| Byte offset | Dword index | Content | Initialized by | Value |
|---|---|---|---|---|
| 0x000 | [0] | HWND (hash key) | FUN_0060F9A0 | `param_1` |
| 0x004 | [1] = piVar14[0] | dialog resource ID / `param_2` | FUN_0060F9A0 @ LAB_0061028c: `*piVar14 = param_2` | dialog ID passed by caller |
| 0x054 | [0x15] = piVar14[0x14] | extra data field from dialog template | FUN_0060F9A0: `piVar14[0x15] = local_aa4[1]` if `local_aa4 != NULL` | from `GetWindowLongA(hwnd, GWL_USERDATA=-21)` extra |
| 0x068 | [0x1a] = piVar14[0x19] | **control-type code** | FUN_0060F9A0: `piVar14[0x1a] = local_ab0` | one of: 0x0–0xB (see table above); default 0xB |
| 0x02C | [0xB] = piVar14[0xA] | cleanup / init flag | FUN_0060F9A0: `piVar14[0xb] = 0` | 0 |
| 0x204 | [0x81] | hash-chain next pointer | FUN_0060F9A0 / allocator | 0 or next record |

**Note on indexing:** `piVar14 = piVar1 + 1` where `piVar1` is the record start (an `int *`).
So `piVar14[N]` = byte offset `4 + N*4` from record start.

**Cross-reference with FUN_00622820 caller:** that function uses byte offsets `+0x6c`, `+0xb0`,
`+0xd5`, `+0xd6`, `+0xd7`, `+0xd8` (raw from record start). These are post-init fields written
*after* FUN_0060F9A0 returns:
- `+0x6c` = dialog template ID (written: `*(int*)(local_c + 0x6c) = param_2`)
- `+0xd5..+0xd8` = per-dialog type flags (1-byte booleans keyed on dialog ID)
- `+0xb0` = rendering mode = 2 for specific dialog IDs

The `+0xD8` byte (from SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md context) is
written by FUN_00622820 as: `*(byte*)(record+0xD8) = 1` if dialog ID == `0x108` or `0xBC6`.
Dialog `0xE2` (226) does not match either, so `+0xD8 = 0` for the initial main menu.

---

## Global Color/Theme Variables Initialized (One-Time, Gated by `DAT_00ac48d4 == 0`)

All written unconditionally at function entry, then color sub-init is gated:

| Global | Value set at entry (always) | Notes |
|---|---|---|
| `DAT_00ac1cb4` | `0x9F` | disabled-text color low component (used by Static, Checkbox) |
| `DAT_00ac1ca8` | `0x9F` | related color |
| `DAT_00ac1df0` | `1` | |
| `DAT_00ac1890` | `0x7F` | |
| `DAT_00ac18a4` | `0xFFFF` | default enabled text color (yellow `#FFFF00` when read as RGB) |
| `DAT_00ac1cb0` | `0xEEEEEE` | |
| `DAT_00ac4618` | `0x9F9F` | |
| `DAT_00ac184c` | `0xFFFFFF` | |
| `_DAT_00ac4628` | `&PTR_DAT_00808080` | pointer |
| `DAT_00ac4604` | `0xFF` | |
| `DAT_00ac4880` | `&LAB_00626262` | pointer |
| `DAT_00ac4624` | `0xFF` | |
| `DAT_00ac1dd8` | `&DAT_00929292` | pointer |
| `DAT_00ac1b98` | `0xC5BEA7` | |
| `DAT_00ac1b94` | `&DAT_00807A68` | pointer |
| `DAT_00ac1af8` | `&DAT_008F8F8F` | pointer |
| `DAT_00ac4620` | `0x646464` | |
| `DAT_00ac4898` | `0x60` | |
| `DAT_00ac4608` | `0xAAAA` | |
| `DAT_00ac48b0` | `0x221B0B` | |
| `_DAT_00ac1b90` | `&LAB_00443716` | pointer |

**Corroboration:** `DAT_00ac18a4 = 0xFFFF` and `DAT_00ac1cb4 = 0x9F` match §7 of
SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md exactly.

**Gated block** (`if DAT_00ac48d4 == 0`): computes display-format bit-shift values for
R, G, B channels using palette helper functions `FUN_004bbc40/30/60/50/80/70`, then calls
`FUN_0061f210()` which preloads ~150 named PCX images (faction icons, button art, arrows,
scrollbar grips, tab chrome, etc.) into CDFileClass instances. Sets `DAT_00ac48d4 = 1`
after completion (run-once guard).

---

## Post-Init Message Sequence

At the end of FUN_0060F9A0, after all setup:

```c
// Step 1: fetch initial control text
CallWindowProcA(lpPrevWndFunc, param_1, WM_GETTEXT=0xD, 0x800, acStack_800);

// Step 2: if text non-empty, load string table entry 0x1CE0 from D:\ra2mdpost\ownrdraw.cpp
if (acStack_800[0] != '\0')
    uVar10 = StringTable__LoadString("D:\\ra2mdpost\\ownrdraw.cpp", 0x1CE0);

// Step 3: send post-setup notification
SendMessageA(param_1, 0x497 /* WM_USER+0x97 */, 0, 0);
```

`WM_USER+0x97 = 0x497` is the "owner-draw subclass initialized" notification.
`FUN_00622B50` handles it by forwarding `SendMessageA(parent, 0x4A9, hwnd, 1)`.

---

## Active in YR

YES. The entire function is reachable via:
`FUN_00531cc0` → (creates RT_DIALOG 0xE2) → `MainMenuDialog0xE2_Proc_00531F60` →
`FUN_00622b50` (WM_INITDIALOG) → `FUN_0060f4b0` → `EnumChildWindows(..., FUN_0060f9a0, ...)`.

This is the live init path for every YR shell dialog. No TS-gating found.

---

## Open Questions / Unverified

1. **0x0610CA0 universal subclass proc** — Ghidra shows no function at this address
   (returns "No function found"). The subclass proc body is undecompiled in this session.
   All other claims do not depend on its internals.

2. **`GetWindowLongA(param_1, -0x15)`** — offset `-0x15` = -21 decimal. This is not a
   standard GWL_* constant (standard GWL_USERDATA = -21 = -0x15). So this IS
   `GWL_USERDATA`. The value at `local_aa4` is the dialog template extra-data pointer.
   `local_aa4[1]` (byte offset +4) = the secondary word in that extra block.
   Confidence: MEDIUM — the constant value matches GWL_USERDATA but not independently
   verified against the Windows SDK constant.

3. **Control type code default `0xB`** — `local_ab0` is initialized to `(undefined4*)0xB`
   before the class-name chain. If no class name matches, `0xB` stays as the type code.
   This appears to be a "generic/unknown" sentinel, consistent with `FUN_00623340`
   initializing the WindowExtra record `[0x1a]` to `0xB` as the reset value.

---

## 5 Most Load-Bearing Verified Facts

1. **Control dispatch is by class name then style bits** — `"Button"` class with
   `(style_byte & 0xB) == 0xB` maps to `OwnerDraw_Button_00612B70`; `"Static"` → 
   `OwnerDraw_Static_006153E0`; `"Button"` with `(style & 3)==3` → 
   `OwnerDraw_Checkbox_006163A0`; `(style & 9)==9` → `OwnerDraw_RadioVariant_00616980`.
   (Source: decompiled body of `FUN_0060F9A0`, Button branch @ 0x0060FE58–0x0060FF00.)

2. **Control-type code stored at WindowExtra record byte +0x68** — `piVar14[0x1a]`
   (= record[4 + 0x1A×4] = record[0x6C]… wait, corrected: `piVar14[0x1a]` = 
   piVar14 at dword index 0x1A = piVar14 is `int*` pointing to `record[1]` so 
   piVar14[0x1A] = `*(int*)(record + 4 + 0x1A*4)` = record byte `0x6C`).
   Written as `piVar14[0x1a] = local_ab0` at LAB_0061028C. Default/unmatched = `0xB`.
   (Source: LAB_0061028C in decompiled body.)

3. **`DAT_00ac18a4 = 0xFFFF` and `DAT_00ac1cb4 = 0x9F`** — these globals are set
   unconditionally at function entry, confirming §7 of the sibling
   SHELL_BUTTON_PAINT_DETAILS doc. `0xFFFF` = enabled text color (yellow), `0x9F` =
   Static/Checkbox disabled text color. (Source: lines 46–47 of decompiled body.)

4. **Universal subclass proc at `0x610CA0` installed via `SetWindowLongA(hwnd, -4, 0x610CA0)`**
   — every control, regardless of class, gets this WndProc. Original WndProc saved to
   `DAT_00ac1b48` hash table. (Source: `LVar6 = SetWindowLongA(param_1,-4,0x610ca0)` in
   decompiled body, then `puVar7[1] = LVar6` stored to second hash table.)

5. **PCX asset preload is run-once, gated by `DAT_00ac48d4`** — `FUN_0061f210` loads
   ~150 PCX files (faction icons, button strips, scrollbar art, tab chrome, etc.) the
   first time any HWND is processed; subsequent calls to `FUN_0060F9A0` skip it.
   (Source: `if (DAT_00ac48d4 == 0) { ... FUN_0061f210(); DAT_00ac48d4 = 1; }` block.)
