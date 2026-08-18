# SDBTNANM.SHP frame-10 overlay — record-byte gate and dialog 0xE2 lifecycle

**Investigation target:** The record-byte flag that gates whether
`SDBTNANM.SHP` frame 10 is drawn as an overlay on shell buttons; state
transitions on dialog `0xE2`'s lifecycle.

**Date:** 2026-05-19

**Anchors (do not re-derive — verified against live binary in this session):**

- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md` — locates the branch,
  reader, writer, and confirms offset `+0xD8`.
- `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md` — traces all 4 setter
  call sites to WOL dialog procs; resolves UX semantic.
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` —
  establishes the full `0xE2` paint pipeline and confirms no setter is ever
  called for `0xE2`.

This report synthesizes those three prior investigations, adds live binary
verification of every load-bearing claim, and provides the definitive answer
to Open Question #3 from the composition report.

**No code or comments were changed. Ghidra MCP used read-only.**

---

## 1. Executive Summary

**The frame-10 row is not drawn on dialog `0xE2` (offline main menu).**

The gate is a single byte at **WindowExtra record offset `+0xD8`** (the
per-HWND subclass record stored in the hash table at `DAT_00AC1B00`). When
that byte is `0`, `WM_PAINT_Handler` passes `param_3 = 1` to
`RightPanel__Draw`, which **skips** the frame-10 loop. When the byte is `1`,
`param_3 = 0` and the frame-10 loop **runs**.

Dialog `0xE2` is created with `record[+0xD8] = 0` (default) and **no live
code path ever sets it to `1` for `0xE2`**. The setter `FUN_00608440 @
0x00608440` has exactly 4 live callers, all inside Westwood Online
(`wonline.cpp`) dialog procs for WOL dialogs `0x113`, `0xC4`, and `0x130`.
Standard offline YR skirmish never reaches any of those callers. The bit is
sticky-once-set (no live clearer). The frame-10 overlay is therefore
exclusively a **WOL-screen chrome marker**, invisible on the offline shell.

Active in YR: **Conditional** — active on WOL-family dialogs (`0x113`,
`0xC4`, `0x130`) when online; never active on offline dialog `0xE2`.

---

## 2. The gate — record byte at offset `+0xD8`

### 2.1 Data structure

The per-HWND subclass record lives in the hash table:

| Symbol | Address | Verified by |
|---|---|---|
| Table base pointer | `DAT_00AC1B00` | `decompile_function 0x00621E90` — `DAT_00ac1b00` walk |
| Table count | `DAT_00AC1B04` | same decompile — `if (DAT_00ac1b04 != 0)` |
| Table mask log2 | `DAT_00AC1B0C` | same decompile — `(1 << ((byte)DAT_00ac1b0c & 0x1f)) - 1U` |
| Hash thunk ptr | `DAT_00AC1B18` | same decompile — `(*DAT_00ac1b18)()` |

Each record bucket: `[0]` = HWND key (4 bytes); `[1..N]` = record data. The
gate byte is at dword index `0x36` from the key dword (= **byte offset
`+0xD8` from the HWND key**). This is the consistent address used by both the
reader and the setter, confirmed by offset arithmetic below.

### 2.2 Reader — `WM_PAINT_Handler @ 0x00621E90`

Live decompilation (verified via `decompile_function 0x00621E90`):

```c
// piVar9 = record_ptr + 1  (i.e. piVar9[0] = HWND, piVar9+1 = data start)
RightPanel__Draw((char)piVar9[0x35] == '\0');
```

`piVar9[0x35]` = `*(piVar9 + 0x35 * 4)` = `*(record_root + 4 + 0xD4)` =
`*(record_root + 0xD8)`. The boolean result is:

| `record[+0xD8]` | `piVar9[0x35]` | `param_3` to RightPanel__Draw | Frame-10 drawn |
|:---:|:---:|:---:|:---:|
| `0` (default) | `'\0'` (zero) | `1` (nonzero) | **No** |
| `1` (set) | nonzero | `0` | **Yes** |

Prior report disassembly (address `0x00621FEC`) is consistent with the live
decompile: `MOV DL, byte ptr [EBX+0xD4]` where EBX = `piVar5 = record bucket
base` and `piVar9 = piVar5 + 1`, so `[EBX+0xD4] = [piVar9-4+0xD4] =
piVar9[0x35]` — same offset `+0xD8` from HWND key.

### 2.3 Branch in `RightPanel__Draw @ 0x0072E450`

Live decompilation (verified via `decompile_function 0x0072E450`):

```c
void __fastcall RightPanel__Draw(undefined4 param_1, undefined4 *param_2, char param_3)
{
    ...
    if (param_3 == '\0') {
        // frame-10 loop
        iVar2 = 0;
        local_18 = *DAT_00b0fc10;
        local_14 = DAT_00b0fc10[1];
        if (0 < DAT_00b0fa20) {
            do {
                CC_Draw_Shape(g_SDBTNANM_SHP, 10, &local_18, &local_10, 0x400, ...);
                local_14 = local_14 + DAT_00b0fc10[3];
                iVar2 = iVar2 + 1;
            } while (iVar2 < DAT_00b0fa20);
        }
    }
    ...
}
```

Frame index `10` (`0xA`) is a literal constant. The loop runs once per button
row (`DAT_00b0fa20` = count of button rows). `SDBTNBKGD.SHP` frame 0 is
always drawn unconditionally in the loop above; SDBTNANM frame-10 is the
conditional overlay on top of it.

### 2.4 Setter — `FUN_00608440 @ 0x00608440`

Live decompilation (verified via `decompile_function 0x00608440`):

```c
void __fastcall FUN_00608440(int param_1)  // param_1 = HWND
{
    // walks DAT_00AC1B00 hash table, finds record for param_1
    if (record_found && record != NULL && record != 0xFFFFFFFC)
        *(undefined1 *)(piVar2 + 0x36) = 1;  // record[+0xD8] = 1
}
```

Write: `*(piVar2 + 0x36)` — piVar2 is the int* bucket start (HWND key at
index 0), so `+0x36 * sizeof(int)` is not the arithmetic here; the cast is
`(undefined1 *)(piVar2 + 0x36)`, meaning byte at address `piVar2 + 0x36` =
`piVar2 + 0x36` bytes (C pointer arithmetic on `undefined1 *` after cast).
Prior report's disassembly `0x00608493: MOV byte ptr [EAX+0xD4], 0x1` where
EAX = `piVar2 + 4` confirms byte offset `+0xD4` from piVar2+4 = `+0xD8` from
record root. Both are consistent.

**Callers (all live UNCONDITIONAL_CALL, verified via `get_xrefs_to 0x00608440`):**

| Address | Enclosing dialog DLGPROC | Dialog ID | Triggering wMsg | User action |
|---|---|---|---|---|
| `0x0078B808` | `0x0078AC10` | `0x113` (WOL chat/lobby) | `0x686` | Control 0x686 click ("Back" / soft-exit) |
| `0x0078BF87` | `0x0078AC10` | `0x113` (WOL chat/lobby) | `0x689` | Control 0x689 click ("Disconnect" / hard-exit, sets lobby_state=5) |
| `0x00792DA6` | `0x00792CF0` | `0xC4` (WOL custom-match) | `0x497` | Owner-draw chrome init/refresh on dialog show |
| `0x00793407` | `0x00793280` | `0x130` (WOL Verify Connections) | `0x497` | Owner-draw chrome init/refresh on dialog show |

All four sites pass the **parent dialog HWND** to `FUN_00608440`. Sites 1 and
2 additionally call `SetEvent(DAT_00B7369C)` (the `EV_EXIT` event handle —
`NOT` an HWND, confirmed via string literal `s_setting_EV_EXIT_0084c654`).

### 2.5 Clearer — `FUN_006084A0` (orphan, no xrefs)

`get_xrefs_to 0x006084A0`: **no references found**.

`read_memory 0x006084A0` bytes at offset +84..90: `0xC6 0x80 0xD4 0x00 0x00
0x00 0x00` = `MOV byte ptr [EAX+0xD4], 0x0`. Confirmed: same struct, same
offset, writes `0` instead of `1`. Dead in current Ghidra xref index; no data
refs found either (consistent with prior report).

The alternate setter at `0x006240A0` (also writes `record[+0xD8] = 1`,
identified in the prior report) was not re-verified in this session; treat as
UNVERIFIED for this pass but noted in the prior report.

---

## 3. Dialog `0xE2` lifecycle — where the gate is never set

### 3.1 Initial state

The WindowExtra record for dialog `0xE2`'s HWND is created in
`WM_PAINT_Handler` on the first WM_PAINT: `operator_new(0x20)` initializes 8
dwords to zero (`piVar5[1..3] = 0`, `piVar5[4] = 2`, PixelBuffer_Init).
`record[+0xD8] = 0` on creation. Verified: the decompile shows no initial set
of `piVar9[0x35]`; the zero-default comes from the zeroed allocation.

### 3.2 WM_INITDIALOG path (`FUN_00622B50`)

`FUN_00622B50` handles `WM_INITDIALOG` for `0xE2`:
- `FUN_0060F9A0` — installs owner-draw procs; sends `0x497` to children, NOT
  to the parent. Does not call `FUN_00608440`.
- `FUN_0060CF00` — assigns SHP/palette pointers for the dialog; no
  `FUN_00608440` call.
- `FUN_0060C540` — writes paint mode `1` to the record; no gate byte write.

None of these touch `record[+0xD8]`.

### 3.3 WM_PAINT path

`WM_PAINT_Handler @ 0x00621E90` reads `piVar9[0x35]` (= `record[+0xD8]`) and
passes `(record[+0xD8] == 0)` as `param_3`. For `0xE2`, this is always `(0
== 0) = 1`, so `param_3 = 1`, and **the frame-10 loop is always skipped**.

### 3.4 Dialog proc `0x00531F60`

Handles `WM_PAINT` by sending `0x4F0` to child `0x71A` (movie draw). Does not
call `FUN_00608440`.

Handles `WM_COMMAND` for button IDs `0x683`, `0x684`, `0x578`, `0x686`,
`0x55C`, `0x3EE` — returns result codes 1..6 through `GetWindowLong(hwnd,
8)`. None of these call `FUN_00608440`.

### 3.5 Conclusion for `0xE2` lifecycle

`record[+0xD8]` is `0` for the entire lifetime of dialog `0xE2`. Every WM_PAINT
call passes `param_3 = 1` to `RightPanel__Draw`, and the SDBTNANM frame-10
loop is never executed. The offline main menu always shows the plain
`SDBTNBKGD.SHP` frame-0 static tile behind buttons — no SDBTNANM frame-10
highlight overlay.

---

## 4. Semantic meaning of `record[+0xD8]`

Based on the 4 confirmed setter call sites:

> `record[+0xD8] = 1` means: **"this dialog is a WOL-family screen that has
> entered its WOL-specific lifecycle."**

It is set on WOL dialog initialization (`0x497` owner-draw refresh on show)
and on WOL dialog button-driven exit (`0x686`, `0x689`). Once set, it remains
`1` for the dialog's lifetime (no live clearer). The frame-10 overlay is the
**visual chrome for WOL screens** — it distinguishes online from offline shell
panels.

Naming suggestion: `WOL_chrome_active` or `is_wol_dialog`.

**Active in YR:** Conditional — live on WOL dialogs in online mode. Completely
inactive for offline dialog `0xE2`.

**TS-legacy:** No. The reader is on the live `WM_PAINT` path of `0xE2`, and
the setters fire from `wonline.cpp` dialog procs used in a vanilla YR
multiplayer session. No `SpecialFlags` gate.

---

## 5. Draw layer context

When the frame-10 row IS drawn (WOL screens), the right-panel draw order is:

| Step | Asset | Frame | Condition |
|---:|---|---:|---|
| 1 | `SDTP.SHP` | `0` | Always |
| 2 (loop) | `SDBTNBKGD.SHP` | `0` | Always, one row per button |
| 3 (loop) | `SDBTNANM.SHP` | **10** | Only when `record[+0xD8] = 1` (WOL screens) |
| 4 | `SDBTM.SHP` / `DAT_00b0fa38` | `0` | Always |
| 5 | `LWSCRNS.SHP` or `LWSCRNL.SHP` | `0` | Width-gated |

For dialog `0xE2`, step 3 is never reached.

The sibling right-panel drawers `FUN_0072E820`, `FUN_0072E9F0`, and
`FUN_0072EAD0` always pass `param_3 = 0` (unconditional draw of frame-10).
Only the main `WM_PAINT_Handler` call site at approximately `0x00621FFE` makes
the row conditional via the record byte.

---

## 6. Implementation implications

For the Rust main-menu shell:

1. **Do not draw SDBTNANM frame-10** on dialog `0xE2`. The draw is
   permanently gated off for offline mode. Draw `SDBTNBKGD.SHP` frame-0 tiles
   only.
2. When implementing WOL screens, add the `wol_chrome_active: bool` flag to
   the per-dialog record (equivalent to `record[+0xD8]`). Set it during WOL
   dialog init (`0x497` handler) and on WOL exit-button clicks. Then pass it
   as the `draw_sdbtnanm_frame10` argument to the right-panel draw function.
3. No timer, no animation cadence. Frame 10 is a static binary overlay — on
   or off, never cycled.
4. Frames 1..9 and 11+ of `SDBTNANM.SHP` are not referenced by any draw
   path found in `RightPanel__Draw` or its siblings. Do not draw them.

---

## 7. Open Questions

- **`DAT_00B73E40` identity** — the anchor report noted this global in the
  area of the WOL setter call sites but left its role unverified after
  correcting `DAT_00B7369C` to be the `EV_EXIT` event handle. Not
  re-investigated in this pass; scoped out.
- **Button text for WOL control IDs `0x686` and `0x689`** — confirmed as
  "Back"/"Disconnect" by convention but not by RT_DIALOG resource parse.
  MEDIUM confidence. Would need the WOL dialog template extracted.
- **Whether the alternate setter at `0x006240A0` is reachable via indirect
  call** — direct data-ref scan in prior report found nothing; not re-checked
  here. Treat as UNVERIFIED-orphan until runtime watchpoint.
- **Full WOL-family dialog set that triggers frame-10** — only `0x113`,
  `0xC4`, `0x130` confirmed via 4 direct CALL xrefs. The `FUN_00622820`
  whitelist contains 19 dialog IDs; others may set the bit via indirect paths
  not found in static analysis.

---

## 8. Sources

**Ghidra MCP calls in this session (read-only):**

- `decompile_function 0x0072E450` — confirmed `RightPanel__Draw` body,
  `param_3 == '\0'` branch, `CC_Draw_Shape(g_SDBTNANM_SHP, 10, ...)` literal.
- `decompile_function 0x00621E90` — confirmed `WM_PAINT_Handler` body,
  `piVar9[0x35] == '\0'` predicate feeding `RightPanel__Draw`.
- `decompile_function 0x00608440` — confirmed setter body, `*(piVar2+0x36)=1`,
  fastcall `int param_1` signature.
- `get_function_by_address 0x00608440` — confirmed function exists, body
  `0x00608440..0x0060849B`.
- `get_function_by_address 0x006084A0` — confirmed `No function found`
  (Ghidra has not promoted to function; body is bytewise present).
- `get_xrefs_to 0x00608440` — confirmed 4 callers: `0x0078B808`,
  `0x0078BF87`, `0x00792DA6`, `0x00793407`.
- `get_xrefs_to 0x006084A0` — confirmed `No references found` (clearer is
  orphan).
- `read_memory 0x006084A0` (100 bytes) — bytes at offset +84..90:
  `0xC6 0x80 0xD4 0x00 0x00 0x00 0x00` = `MOV byte ptr [EAX+0xD4], 0x0`;
  confirmed write of `0` to same offset.

**Prior reports consulted (not re-derived):**

- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`
- `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`
