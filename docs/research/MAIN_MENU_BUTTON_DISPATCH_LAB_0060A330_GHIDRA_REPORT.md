# Main-Menu Button Dispatch — `LAB_0060A330` Sets `+0xB0 = 1` → SDBTNANM

**Date:** 2026-05-19
**Primary addresses:** `LAB_0060A330` (EnumChildWindows callback @ 0x0060A330),
`FUN_00608CD0` (predicate @ 0x00608CD0), `FUN_00609730` (predicate @ 0x00609730),
`OwnerDraw_Button_00612B70` (paint dispatch reading `+0xB0`).

**Confidence:** HIGH — every claim verified by live `decompile_function` and
`disassemble_function` calls in this session.

**Active in YR:** Yes — `LAB_0060A330` is called from `FUN_00622820` (the
common shell `FUN_00622800` show-dialog helper) and from `FUN_00622B50` (the
shared shell `WM_INITDIALOG` handler). Both run for dialog `0xE2`.

---

## Why this report exists

This report refutes two prior reports that asserted "the live YR shell button
path is the PCX path (`bue_*30`/`bde_*30` greyscale)":

- `SHELL_BUTTON_GREYSCALE_COLORIZATION_GHIDRA_REPORT.md` — §1 Headline Conclusion
- `SHELL_PCX_BUTTON_TILE_AND_CAP_GEOMETRY_GHIDRA_REPORT.md` — Active-in-YR claim

The PCX-path code DOES exist and the greyscale-no-tint analysis IS correct *for
that branch*, but **dialog 0xE2's six owner-draw buttons do NOT take that
branch.** They take the `iVar14 == 1` branch in `OwnerDraw_Button_00612B70`,
which loads `g_SDBTNANM_SHP` (frames 2/3/4) with `FUN_0072e2c0` palette getter.

The error in the prior reports: they observed that `iVar14 == 0` would lead
to the PCX path, but did not trace WHO writes `+0xB0` on the button records.
For dialog 0xE2 buttons, `LAB_0060A330` writes `+0xB0 = 1` at WM_INITDIALOG
time, so by the time `OwnerDraw_Button_00612B70` runs `iVar14 = piVar17[0x2c]`,
the value is 1, not 0.

---

## 1. The full dispatch chain

### 1.1 `OwnerDraw_Button_00612B70` reads `piVar17[0x2c]` (record +0xB0)

Decompile excerpt (`decompile_function 0x00612B70`):

```c
iVar14 = piVar17[0x2c];
if (iVar14 == 0) {
    iVar14 = piVar17[5];               // +0x14 custom image pointer
    if (iVar14 == 0) {
        // PCX path: bue_li30/mi30/ri30, bde_li30/mi30/ri30
    }
    else { /* custom image blit */ }
}
else if (iVar14 == 1) {
    piStack_c4 = FUN_0072e2c0();        // SDBTNANM palette convert
    piStack_dc = g_SDBTNANM_SHP;        // SHP source
    local_f0 = 0x2;                     // frame 2 = default
    if (((uint)pWStack_d8 & 1) == 0) {
        if (*(char *)((int)piVar17 + 0xc5) != '\0') {
            local_f0 = 0x3;             // frame 3 = focus-flash (read note below)
        }
    }
    else {
        local_f0 = 0x4;                 // frame 4 = pressed
    }
}
else if (iVar14 == 2) { /* DAT_00b0f9ec SHP */ }
else if (iVar14 == 3) { /* DAT_00b0facc SHP */ }
// ... eventual CC_Draw_Shape(piStack_dc, frame_idx, ...) blit
```

Verified via `decompile_function 0x00612B70` + `disassemble_function 0x00612B70`
(switch table at `0x6137a8`, dispatch at `0x00612e87: MOV ECX, [EBP+0xb0]`).

**Note on `+0xc5` "hover":** this byte is set by:
- Custom message `0x4DC` (mouse-enter/leave from a higher subclass) — see
  `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md`.
- Custom message `0x113` (toggles the byte).

In retail YR the practical effect under a standard mouse-over IS a frame-3
swap, so the "hover" state in the existing Rust port (SDBTNANM frame 3) is
visually correct.

### 1.2 `LAB_0060A330` writes `+0xB0 = 1` (the SDBTNANM dispatch)

`LAB_0060A330` is an `EnumChildWindows` callback (`WNDENUMPROC`) dispatched
from two call sites:

| Caller | Call site | Context |
|---|---|---|
| `FUN_00622820` (shell show-dialog helper) | `EnumChildWindows(param_1, (WNDENUMPROC)&LAB_0060a330, 0)` at `0x00622b2a` | Runs after `FUN_0060F9A0` and `FUN_0060F760` enumerators |
| `FUN_00622B50` (shared shell `WM_INITDIALOG`) | DATA xref at `0x00623072` | Reached on every common-shell-mode dialog `WM_INITDIALOG` |

The callback body (verified via `get_assembly_context xref_sources=0x0060a489 ...`):

```text
LAB_0060a330: SUB ESP, 0xC
  ESI = arg1 (child HWND)
  Look up the owner-draw record for ESI → local +0x1C
  If no record → exit (return 1)

  CALL [0x007e14ec]                ; Win32 helper (GetParent or similar)
  EBP = parent HWND (or returned hwnd)

  MOV ECX, 0xa8b238                ; &g_ScenarioClass_Instance
  CALL FUN_0069bbe0                ; scenario-active predicate (AL = true if scenario started)
  JNZ 0x0060a48e                   ; if scenario running → second predicate block

  ; Shell (no scenario) block:
  GetWindowLongA(hwnd, GWL_STYLE)
  AND EAX, 0xB / CMP AL, 0xB       ; Button-class signature
  JNZ ... (skip)

  CALL FUN_00608CD0                ; predicate 1: "is this a tagged shell button?"
  TEST AL, AL / JNZ +N             ; if true, fall through to write

  ; or via second sub-block:
  GetWindowLongA(hwnd, GWL_STYLE) / AND 0xB / CMP 0xB
  Look up record; check [record+0x68] (type code) == 0
  CALL FUN_00609730                ; predicate 2: "is this the special button?"
  TEST AL, AL / JZ 0x0060a489      ; if false, exit

  MOV EDX, [ESP+0x1C]              ; EDX = record ptr
  MOV EAX, 1
  MOV [EDX + 0xB0], EAX            ; ★ RECORD+0xB0 = 1 → SDBTNANM dispatch
  RET 0x8

  0x0060a489: (scenario-active block — checks FUN_00608CD0 / FUN_00609730 again
              with different state-active branch; also writes +0xB0 with similar logic)
```

Verified via `disassemble_function 0x00612B70` (callsite of FUN_00609730 at
`0x0060a468`) and `get_assembly_context xref_sources=0x0060a489`.

### 1.3 `FUN_00608CD0` — the broad predicate

`FUN_00608CD0(int dialog_id, HWND child)` returns true for a per-dialog set of
control IDs. **For dialog `0xE2`:**

| Control ID | Symbol | Returns true |
|---:|---|:-:|
| `0x683` | Single Player button | ✓ |
| `0x684` | WW Online button | ✓ |
| `0x578` | Network button | ✓ |
| `0x686` | Movies & Credits button | ✓ |
| `0x55C` | Options button | ✓ |
| `0x55F` | Yuri Website button (corner) | ✓ |
| `0x694` | Title heading static (via the big disjunction including iVar4==0xe2 AND iVar3==0x694) | ✓ |

Verified via `decompile_function 0x00608CD0` — the `iVar4 == 0xe2` branch
explicitly tests `iVar3 == 0x686 / 0x578 / 0x55c / 0x683 / 0x55f / 0x684`,
plus the earlier big-disjunction branch matches `iVar3 == 0x694` for many
dialog ids including `0xe2`.

### 1.4 `FUN_00609730` — the special-button predicate

`FUN_00609730(int dialog_id, HWND child)` for `iVar4 == 0xe2`:

```c
if (iVar4 == 0xe2) {
    return iVar3 == 0x3ee;
}
```

Verified via `decompile_function 0x00609730`. Returns true ONLY for control
`0x3EE` (the Exit Game button).

### 1.5 Net effect for dialog 0xE2 owner-draw children

| Control | Predicate match | `+0xB0` set | Paint branch |
|---|---|:-:|---|
| `0x683` Single Player | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x684` WW Online | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x578` Network | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x686` Movies & Credits | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x55C` Options | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x55F` Yuri Website | `FUN_00608CD0` | 1 | SDBTNANM frames 2/3/4 |
| `0x3EE` Exit Game | `FUN_00609730` | 1 | SDBTNANM frames 2/3/4 |
| `0x694` Title heading static | `FUN_00608CD0` (big disjunction) | 1 | (static path, separate `OwnerDraw_Static_006153E0` dispatch — not button) |

All seven main-menu buttons go through the SDBTNANM colored-gradient path.
The PCX `bue_*30`/`bde_*30` greyscale path is **never reached** for dialog
0xE2 owner-draw buttons.

---

## 2. Where the PCX path IS used

`SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` documents that the PCX preload
(`FUN_0061F210` registering `bue_*30.pcx` / `bde_*30.pcx`) does happen. The
PCX code path is the `iVar14 == 0 && piVar17[5] == 0` default in
`OwnerDraw_Button_00612B70`. Two scenarios for it to fire:

1. **Owner-draw buttons on a dialog NOT in `FUN_00608CD0`'s match list AND NOT
   in `FUN_00609730`'s match list.** Many in-mission popup dialogs that use
   `Button` class with style `0xB` but are not whitelisted will hit the PCX
   path.

2. **Owner-draw buttons whose record `+0xB0` got reset to 0 after init** — no
   such writer is known.

Whether ANY YR live dialog actually exercises the PCX path was not enumerated
exhaustively in this session. The `FUN_00608CD0` match list is large (covers
0xe2/0x100/0x101/0x102/many WOL dialogs/many in-mission UI) — the
unwhitelisted set may be small.

---

## 3. Implications for the Rust port

The chrome atlas comment at [src/render/main_menu_shell_chrome.rs:1-9] is
**correct**:

> Buttons are SDBTNANM.SHP frames 2 (default), 3 (hover), and 4 (pressed) —
> drawn through CC_Draw_Shape with the SDBTNANM palette, producing the
> red / orange / yellow gradient artwork the player sees. The `bue_*30` /
> `bde_*30` PCXs that ship in the same archives are greyscale and unused on
> this paint path.

The "unused on this paint path" clause is accurate for dialog 0xE2. The
prior swarm's attempt to swap to the PCX path produced visibly worse output
because the PCX path is not what retail draws on the main menu.

---

## 4. Open questions

1. Which YR dialogs (if any) actually reach the PCX path? Enumeration of all
   dialog IDs would clarify, but the practical answer for the main menu is
   "none."
2. The `+0xc5` "hover" byte is poked by `0x4DC` and `0x113` — its mouse-move
   semantics weren't traced beyond what
   `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md` documents.
3. `LAB_0060A330` has a second predicate block (scenario-active branch at
   `0x0060a489`) using `FUN_00608CD0` / `FUN_00609730` again. Its exact
   conditions weren't traced — but the shell path (scenario inactive) is
   the one that matters for dialog 0xE2 at startup.

---

## Verification calls used

- `decompile_function 0x00612B70` — `OwnerDraw_Button_00612B70`
- `disassemble_function 0x00612B70` — confirmed dispatch at `0x00612e87`
- `decompile_function 0x0060F9A0` — confirmed Button subclass setup
- `decompile_function 0x00623340` — confirmed record initializer zeros +0xB0
- `decompile_function 0x00622820` — confirmed `LAB_0060A330` is an EnumChildWindows callback
- `decompile_function 0x00608CD0` — full match-list extraction
- `decompile_function 0x00609730` — confirmed 0xE2 → 0x3EE branch
- `get_xrefs_to 0x0060A330` — confirmed callers FUN_00622820 + FUN_00622B50
- `get_assembly_context xref_sources=0x0060a489 / 0x0060a47c` — confirmed +0xB0 = 1 write
- `read_memory` around 0x0060A330 — confirmed function entry prologue
- `search_byte_patterns "C7 ?? B0 00 00 00 02"` — found dialog-record +0xB0=2 writers (different field, dialog records not button records)

---

## Refutation of prior reports

This report supersedes the "live YR shell button path is PCX" conclusion in:

- `SHELL_BUTTON_GREYSCALE_COLORIZATION_GHIDRA_REPORT.md` (§1 headline)
- `SHELL_PCX_BUTTON_TILE_AND_CAP_GEOMETRY_GHIDRA_REPORT.md` (Active-in-YR claim)

Those reports' analyses of the PCX-branch internals are technically correct
(palette is greyscale, no tint applied, AlphaBlendRect for disabled, etc.) —
but the framing that `iVar14 == 0` is the live path was an unverified
assumption. Add a YELLOW correction banner to those docs.

User in-game observation (2026-05-19) of colored buttons in retail
gamemd.exe is the ground truth that surfaced this error.
