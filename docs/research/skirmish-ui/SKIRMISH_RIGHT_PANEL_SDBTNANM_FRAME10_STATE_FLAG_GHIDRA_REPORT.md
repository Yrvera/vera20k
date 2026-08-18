# Skirmish Right-Panel SDBTNANM Frame-10 State Flag - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072E450`, `0x00608440`, `0x00623340`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the common-shell/dialog-record state that decides whether `SDBTNANM.SHP` frame 10 is drawn for standard offline Skirmish dialog resource `0x102`.  
**Non-Scope:** WOL button labels, full WOL dialog resource parsing, combo/flag/preview controls, and runtime screenshot capture.  
**Confidence:** High for the static binary slice; runtime screenshot/watchpoint not taken.  
**Active in YR:** Conditional. The read path is active for offline Skirmish `0x102`, but the frame-10 overlay is not active there; it is active only when the dialog record's frame-10 gate byte is set, with verified live setters in WOL-family dialog paths.

## 1. Overview

Standard offline Skirmish creates dialog `0x102` with proc `0x006AE3F0`; the proc delegates `WM_PAINT` to the common shell proc and then to `WM_PAINT_Handler @ 0x00621E90`. That handler calls `RightPanel__Draw @ 0x0072E450` with a boolean derived from one byte in the parent dialog's WindowExtra/hash-table record.

For a fresh offline Skirmish `0x102` first paint, that byte is still zero. The caller therefore passes `param_3 = 1`, and `RightPanel__Draw` skips the `SDBTNANM.SHP` frame-10 overlay loop. Standard offline Skirmish first paint draws the static right-panel chrome but not frame 10.

## 2. State Field / Offset Aliases

| Alias | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| Hash bucket/root record | `+0xD8` | Frame-10 gate byte after the HWND key dword | `0x00608493` writes `[EAX+0xD4]` after `ADD EAX,0x4`; same physical byte as root `+0xD8` | Conditional; live when setter is called |
| Dialog data pointer (`record + 4`) | `+0xD4` | Same physical gate byte as above | `WM_PAINT_Handler` reads `piVar9[0x35]`; `piVar9 = piVar5 + 1`, so byte offset is data `+0xD4` / root `+0xD8` | Yes, read on `0x102` paint |

Important pitfall: nearby common-shell flags at data offsets `+0xD5`, `+0xD6`, `+0xD7`, and `+0xD8` are separate bytes. For example `FUN_00622820` writes data `+0xD8` for dialog ids `0x108`/`0xBC6`; that is not the frame-10 gate read by `RightPanel__Draw`'s caller. The frame-10 gate is data `+0xD4` / root `+0xD8`.

## 3. Core Logic

### 3.1 Reader: `WM_PAINT_Handler @ 0x00621E90`

The common parent paint path looks up the dialog HWND in the hash table at `DAT_00AC1B00` and aliases the found bucket as `piVar9 = piVar5 + 1`. In the mode-1 branch, after `FUN_0072E260()` reports right-panel resources ready, it executes:

```text
RightPanel__Draw((char)piVar9[0x35] == '\0')
```

Assembly context confirms the byte load and boolean inversion:

```text
0x00621FEC  MOV DL, byte ptr [EBX + 0xD4]
0x00621FF4  TEST DL, DL
0x00621FF6  SETZ AL
0x00621FF9  PUSH EAX
0x00621FFE  CALL 0x0072E450
```

Because `SETZ` is used, zero means `param_3 = 1`; nonzero means `param_3 = 0`.

**Active in YR:** Yes for standard offline Skirmish `0x102`, because `FUN_006AE3F0` delegates `WM_PAINT` to `FUN_00622B50`, and `FUN_00622B50` calls `WM_PAINT_Handler` for message `0x0F`.

### 3.2 Draw gate: `RightPanel__Draw @ 0x0072E450`

`RightPanel__Draw` draws `SDTP.SHP`, repeated `SDBTNBKGD.SHP`, then conditionally draws repeated `SDBTNANM.SHP` frame `10` only when `param_3 == 0`:

```text
if (param_3 == 0) {
    for each right-panel row:
        CC_Draw_Shape(g_SDBTNANM_SHP, 10, ...)
}
```

Assembly context verifies the frame literal:

```text
0x0072E635  PUSH 0xA
0x0072E637  PUSH ECX
0x0072E63A  CALL 0x004AED70
```

Therefore:

| Gate byte (data `+0xD4`, root `+0xD8`) | Caller `param_3` | Frame-10 loop |
|---:|---:|---|
| `0` | `1` | skipped |
| nonzero / `1` | `0` | drawn |

**Active in YR:** Conditional. The function is active in the common shell path, but the frame-10 overlay body is reached only when the gate byte is nonzero.

## 4. Writers Checked

### 4.1 Record initialization: `FUN_00623340 @ 0x00623340`

`FUN_00623340` zero-fills `0x80` dwords of the dialog record with `STOSD.REP` before setting a few unrelated defaults (`+0x64`, `+0x68`, `+0x3C`, `+0x40`, `+0x5C`, `+0x90`). The frame-10 gate byte is not assigned afterward, so its initial value is `0`.

**Evidence:** decompile of `FUN_00623340`; assembly `0x00623344..0x0062334D` loads `ECX=0x80`, clears `EAX`, and repeats `STOSD`.  
**Active in YR:** Yes for common dialog records allocated through this shell infrastructure.

### 4.2 Live setter: `FUN_00608440 @ 0x00608440`

`FUN_00608440(HWND)` walks the same `DAT_00AC1B00` hash table and writes the gate byte to `1` for the passed parent HWND:

```text
0x0060848C  ADD EAX, 0x4
0x00608493  MOV byte ptr [EAX + 0xD4], 0x1
```

This is root `+0xD8` / data `+0xD4`, matching the reader. `get_function_xrefs` found exactly four direct call sites:

| Call site | Prior verified context | Active in YR |
|---|---|---|
| `0x0078B808` | WOL dialog `0x113` path | Yes, online/WOL |
| `0x0078BF87` | WOL dialog `0x113` path | Yes, online/WOL |
| `0x00792DA6` | WOL custom-match dialog `0xC4`, `0x497` refresh | Yes, online/WOL |
| `0x00793407` | WOL verify-connections dialog `0x130`, `0x497` refresh | Yes, online/WOL |

The assembly context for all four sites passes the parent dialog HWND (`ECX=EBP` or `ECX=ESI`) immediately before the call. No call site is in the offline Skirmish `0x102` launcher/proc path.

### 4.3 Clearer / orphan paths

`get_function_xrefs` to `0x006084A0` returned no references in this Ghidra image. Prior reports identify it as a same-record clearer (`0`) by raw bytes, but no live direct call site is known. It is not part of standard offline Skirmish `0x102`.

**Active in YR:** No direct evidence of activity; treat as not active unless a runtime watchpoint or indirect call trace proves otherwise.

## 5. Standard Offline Skirmish First Paint

Standard Skirmish creation is verified at `0x006AE317..0x006AE328`: `FUN_006AE2C0` calls `FUN_0072CF40`, then loads proc `0x006AE3F0`, dialog id `0x102`, and calls `FUN_00622650`.

During init, common shell setup assigns `0x102` mode-1 painting:

- `FUN_0060CF00` recognizes dialog `0x102` and writes parent background fields (`+0x74`, `+0xE0`, `+0xE4` aliases) but does not write the frame-10 gate.
- `FUN_0060C540` recognizes dialog `0x102` and writes paint mode `1` (`piVar3[0x2D] = 1`) plus a separate byte at data `+0xC1`; it does not write the frame-10 gate.
- `FUN_006AE3F0` handles `WM_PAINT` after common paint; its Skirmish-specific branch draws start positions and validates the rect, with no `FUN_00608440` call.

Result on first paint:

1. Gate byte begins as `0` from `FUN_00623340`.
2. No verified offline `0x102` writer sets it before `WM_PAINT_Handler`.
3. `WM_PAINT_Handler` passes `param_3 = (gate == 0) = 1`.
4. `RightPanel__Draw` sees `param_3 != 0` and skips the frame-10 loop.

**Conclusion:** Standard offline Skirmish dialog `0x102` first paint does not draw `SDBTNANM.SHP` frame 10.

**Active in YR:** Yes for the offline path; the negative result is active in standard offline YR Skirmish.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish launcher to dialog `0x102` | verified | `0x006AE317..0x006AE328` | none |
| `FUN_006AE3F0` common-first paint delegation | verified | decompile calls `FUN_00622B50`; `WM_PAINT` Skirmish work occurs after common proc returns | none |
| `FUN_00622B50` `WM_PAINT` dispatch | verified | decompile case `0x0F` calls `WM_PAINT_Handler` | none |
| `WM_PAINT_Handler` frame-10 gate read | verified | `0x00621FEC..0x00621FFE`; decompile `piVar9[0x35] == '\0'` | none |
| `RightPanel__Draw` frame-10 condition | verified | decompile branch `param_3 == 0`; assembly `0x0072E635 PUSH 0xA` | none |
| Initial gate value | verified | `FUN_00623340` zero-fill `0x80` dwords | no runtime breakpoint, but static init is explicit |
| Live setter inventory | verified for direct calls | `get_function_xrefs 0x00608440` => four WOL sites | possible indirect writer remains runtime-watchpoint territory |
| Offline `0x102` writer absence | verified-with-bounds | decompiled `0x006AE3F0`, `0x00622B50`, `0x0060CF00`, `0x0060C540`; no setter or same-byte write | no full binary byte-pattern writer sweep in this slot |
| WOL semantic details | touched-not-exhausted | prior `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md` | button text/resources out of scope |

## 7. Open Questions - Final State

- [RESOLVED] Q1 - Which state field controls `SDBTNANM.SHP` frame 10 for the common right panel? The byte at dialog data `+0xD4`, equivalent to hash/root record `+0xD8`. Evidence: `0x00621FEC..0x00621FFE`, `0x00608493`.
- [RESOLVED] Q2 - Does zero draw or skip frame 10? Zero skips: reader inverts zero to `param_3=1`, and `RightPanel__Draw` draws frame 10 only when `param_3==0`. Evidence: `0x00621FF6 SETZ`, `0x0072E450` decompile.
- [RESOLVED] Q3 - What is the first-paint value for offline Skirmish `0x102`? Zero. Evidence: `FUN_00623340` zero-fill and no same-byte write in the checked `0x102` init/paint chain.
- [RESOLVED] Q4 - Does standard offline Skirmish first paint draw frame 10? No. Evidence: `0x006AE317..0x006AE328`, `0x00621FEC..0x00621FFE`, `0x0072E450`.
- [RESOLVED] Q5 - Are the known live setters active in YR? Yes, conditionally in WOL paths; not in offline Skirmish. Evidence: `get_function_xrefs 0x00608440` => `0x0078B808`, `0x0078BF87`, `0x00792DA6`, `0x00793407`; prior setter-caller report maps them to WOL dialogs.
- [DEFERRED] Q6 - Are there any indirect same-byte writers outside the checked chain and known setter? Category: bounded-cost-too-high. Static direct-call evidence is enough for this `0x102` slice; a runtime watchpoint on the gate byte would close the global inventory.

## Sources

- Ghidra decompile / assembly context: `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072E450`, `0x00608440`, `0x006084A0`, `0x00623340`, `0x0060CF00`, `0x0060C540`, `0x00622820`.
- Ghidra xrefs: `get_function_xrefs 0x00608440`, `get_function_xrefs 0x006084A0`, `get_function_callers 0x0072E450`.
- Prior docs cross-checked: `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`, `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`, `SDBTNANM_FRAME10_OVERLAY_GATE_GHIDRA_REPORT.md`, `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md`.
