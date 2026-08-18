# SDBTNANM.SHP frame-10 overlay condition — state-transition semantics

Investigation target: the branch at `gamemd.exe 0x0072E5E6` (inside
`RightPanel__Draw @ 0x0072E450`) that gates the `SDBTNANM.SHP` frame-10
row-per-button overlay drawn on the common right-panel chrome (RT_DIALOG `0xE2`
and siblings).

The prior reports
(`MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`,
`MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`) verified
the branch exists and identified it as a "caller flag". The semantic name and
state transitions were unresolved. This pass identifies the predicate input,
its writer(s), and the gating mechanic. **No code or comments changed; Ghidra
MCP used read-only.**

**Status update (2026-05-18 swarm #2):** the UX semantic was settled by
the follow-up report `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md`.
All 4 setter call sites (`0x0078B808`, `0x0078BF87`, `0x00792DA6`,
`0x00793407`) live inside **Westwood Online dialog procs** (dialogs
`0x113` WOL chat/lobby, `0xC4` WOL custom-match, `0x130` WOL "Verify
Connections"). `record[+0xD8] = 1` therefore marks "this dialog is a
WOL-family screen." Standard offline YR skirmish never reaches any of
the setters, so the offline main menu (dialog `0xE2`) always renders
with `param_3 = 1` and the frame-10 row is **not drawn**. The frame-10
overlay is exclusively a WOL-screen highlight.

## TL;DR

- The frame-10 row is **not** a pulse/animation cadence. It is a **binary
  highlight-vs-default selector** with no other frame indices reachable.
- The predicate is `param_3` of `RightPanel__Draw`. The caller in the main
  shell `WM_PAINT` path passes `(record_byte == 0)`, where `record_byte` is
  the byte at offset `+0xD8` of the WindowExtra/subclass-record looked up by
  the parent dialog's HWND.
- `record_byte` defaults to `0` (no highlight). It is set to `1` by
  `FUN_00608440 @ 0x00608440`, a HWND-keyed setter that walks the
  `DAT_00ac1b00..` subclass-record hash table and writes `[record+0xD8] = 1`.
- A symmetric clearer (`FUN_006084A0 @ 0x006084A0`, writes
  `[record+0xD8] = 0`) exists in the binary but has **no live xrefs** in the
  current analysis. An alternate setter at `0x006240A0` is also unreferenced.
- The four call sites of `FUN_00608440` are inside dialog-proc continuations
  in the `0x0078B***` and `0x00792***`/`0x00793***` ranges — invoked during
  dialog construction / per-dialog setup paths. All four pass the parent
  HWND in ECX directly from the enclosing dialog proc's `param_1`.
  **Correction (added 2026-05-18 swarm #2):** an earlier draft of this
  doc claimed those call sites fetch the HWND from globals `[0x00B73E40]`
  and `[0x00B7369C]`. `DAT_00B7369C` is verified to be a Win32 HANDLE — the
  kernel event `EV_EXIT`, slot 6 of the `DAT_00B73684[18]` event array —
  NOT an HWND cache. The full setter-caller trace and the corrected
  identification of `DAT_00B7369C` live in
  `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md`. The role of
  `[0x00B73E40]` was never re-verified.
- Reading and writing live in the same subclass-record hash (the table at
  `[0x00AC1B00]`, count `[0x00AC1B04]`, mask `[0x00AC1B0C]`, hash thunk
  `[0x00AC1B18]`) confirmed in both `WM_PAINT_Handler @ 0x00621E90` and
  the setter. Binding-confidence: HIGH (same table, same walk, same offset
  arithmetic).

## Branch in RightPanel__Draw (confirmed)

`RightPanel__Draw` signature (from decompiler):
`void __fastcall RightPanel__Draw(undefined4 param_1, undefined4 *param_2,
char param_3)`. Relevant lines:

```
0072E5E2  MOV CL, byte ptr [ESP + 0x2C]   ; CL = param_3
0072E5E6  TEST CL, CL
0072E5E8  JNZ 0x0072E65C                  ; if param_3 != 0, skip frame-10 loop
...
0072E635  PUSH 0xA                        ; frame index = 10 (constant)
0072E637  PUSH ECX                        ; ECX = g_SDBTNANM_SHP
0072E63A  CALL 0x004AED70                 ; CC_Draw_Shape
```

C equivalent (already in `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION...`):

```
if (param_3 == '\0') {
    for (i = 0; i < DAT_00b0fa20; ++i) {
        CC_Draw_Shape(g_SDBTNANM_SHP, 10, ...);
        local_14 += DAT_00b0fc10[3];   // row stride
    }
}
```

No other CC_Draw_Shape on `g_SDBTNANM_SHP` was found in this function or in
the right-panel siblings (`FUN_0072E820`, `FUN_0072E9F0`, `FUN_0072EAD0`),
which all pass `0` for `param_3` (so they ALWAYS draw frame 10). Only the
main `WM_PAINT_Handler` call site at `0x00621FFE` makes the row conditional.

## Caller predicate (confirmed at machine level)

`WM_PAINT_Handler @ 0x00621E90` decompiled call:

```
RightPanel__Draw((char)piVar9[0x35] == '\0');
```

Disassembly (verified):

```
00621FEC  MOV DL, byte ptr [EBX + 0xD4]   ; DL = record[+0xD4 from piVar9]
00621FEF  TEST DL, DL
00621FF1  ...
00621FF6  SETZ AL                          ; AL = (DL == 0) ? 1 : 0
00621FF9  PUSH EAX                         ; passed as param_3
...
00621FFE  CALL 0x0072E450                  ; RightPanel__Draw
```

`piVar9` is the per-HWND record pointer from the hash-table walk. The walk
uses `piVar5 = (int*)bucket; piVar9 = piVar5 + 1`, so `[piVar9 + 0xD4]` is
`[piVar5 + 0xD8]` from the record root. The byte is at **record offset
`+0xD8`** measured from the record's first dword (the HWND key).

Semantics:

| record[+0xD8] | param_3 to RightPanel__Draw | frame-10 row |
|:---:|:---:|:---:|
| `0` (default) | `1` | **NOT drawn** |
| `1` (set) | `0` | **drawn** |

## Writer (live) — FUN_00608440

`FUN_00608440 @ 0x00608440` (entry exists; xrefs present). Decompiler:

```c
void __fastcall FUN_00608440(int param_1)   // param_1 = HWND
{
    // walks DAT_00ac1b00 hash table by hashing HWND
    if (record_found && record != 0 && record != (int*)0xFFFFFFFC)
        *(undefined1 *)(record + 0x36) = 1;   // record[+0xD8] = 1
}
```

Disassembly confirms the write at `0x00608493: MOV byte ptr [EAX+0xD4], 0x1`
where `EAX = piVar2 + 0x4` (`piVar2[0]` is the HWND key, so `+0xD4` from
`+4` is `+0xD8` from record root — same byte as the reader).

**Direct callers (4, all `CALL rel32`):**

- `0x0078B808` (in dialog-proc continuation; sets after `CALL 0x007B6760`)
- `0x0078BF87` (in sibling continuation; sets after `MOV [0x00A8B244],5;
  MOV [0x00A8B248],4`)
- `0x00792DA6` (in dialog-proc handling `wMsg == 0x499`; receives parent
  HWND in ESI from `[ESP+8]`; surrounding code references control IDs
  `0x4A9`, `0x4D3`, `0x4D5`, `0x7A9` via USER32-style import slots
  `[0x007E1494/14A4/14A8/14AC]`)
- `0x00793407` (similar shape, immediately followed by call through
  `[0x007E1498]` with arg `0x5`)

All four sites pass the parent HWND (`MOV ECX, ESI`/`MOV ECX, EBP`)
immediately before the call. None pass a child or button HWND, so the
highlight bit is set on the **parent dialog**, not per-button. The
RightPanel__Draw row loop iterates `DAT_00b0fa20` rows from the same caller's
record, meaning the bit toggles **all rows at once** — confirming this is a
whole-panel highlight, not a per-button hover state.

## Writer (dead in current xrefs) — clearer + alt setter

- `FUN_006084A0 @ 0x006084A0` — same wrapper shape as `FUN_00608440` but
  writes `[record+0xD8] = 0` (`0x006084F3: MOV byte ptr [EAX+0xD4], 0x0`).
  **No xrefs detected.** No instruction-level callers found via
  Ghidra's xref index or via direct asm scan.
- `0x006240A0` — third HWND-keyed setter writing `[record+0xD8] = 1`
  (`0x006240F7`). Also **no xrefs detected**; Ghidra did not promote the
  range to a function body.

False-positive matches that share the literal byte sequence `MOV byte ptr
[reg+0xD4], imm8` were:
- `0x0074CE8E` / `0x0074CEB2` — operate on `[ESI+0xD4]` where `ESI` is a
  VeinholeMonster-adjacent object (vtable region near
  `VeinholeMonsterClass__Constructor @ 0x0074C9F0`). Different struct; not
  the WindowExtra record. Excluded.

## YR-active status

- The reader is on the **live `WM_PAINT` path** of dialog `0xE2`
  (`WM_PAINT_Handler @ 0x00621E90`), confirmed in prior reports as the main
  menu shell paint pipeline. Active in YR.
- The setter `FUN_00608440` has 4 reachable call sites in dialog-proc code.
  All four are in the `0x0078B000..0x00794000` range, which is the
  Westwood/RA2 owner-draw dialog logic surface (same code module that
  contains the WM_*-dispatch jump tables routing to `WM_PAINT_Handler @
  0x00621E90`). Active in YR.
- The clearer and alt setter are **dormant** in current Ghidra xrefs. They
  may be reachable via indirect calls not yet resolved, or genuinely dead.
  Without a clearer, the bit stays at `1` once set and is only reset by
  re-allocating the WindowExtra record (a fresh `operator_new(0x20)` and
  table insert per HWND).
- **Not TS legacy.** The reader is on the live YR main-menu paint. The 4
  live setters fire from dialog-proc construction paths in the same module.
  No `SpecialFlags` gate, no TS-only struct (e.g. `VeinholeMonster`)
  involvement on the reader side.

## State-transition summary (only what is confirmed)

```
record[+0xD8] = 0   (default; record created in WM_PAINT_Handler via
                    operator_new(0x20) + PixelBuffer_Init when piVar9[4]==0)
   │
   ▼  FUN_00608440(parent_HWND) called from one of 4 sites at dialog
   │  setup / WM_* handlers in the owner-draw dispatch module
   ▼
record[+0xD8] = 1   (highlight on; frame-10 SDBTNANM row drawn this paint
                    and every subsequent paint until record is destroyed)
```

There is no observed transition `1 → 0` in any live code path. The bit is
effectively **sticky-on** once a dialog enters one of the four setter paths.
This is consistent with the SDBTNANM frame-10 row being a **persistent
highlight overlay** (e.g. "this panel has accepted focus / is in
post-setup state"), not a hover, click-down, or pulse effect.

## Frame-10 vs other frame indices

`SDBTNANM.SHP` has 11+ frames. Inside `RightPanel__Draw` only the literal
`0xA` (10) is pushed to `CC_Draw_Shape`. The three sibling right-panel
drawers (`FUN_0072E820`, `FUN_0072E9F0`, `FUN_0072EAD0`) all call
`RightPanel__Draw(0)` (param_3 = 0), so they unconditionally draw the
frame-10 row. No drawer references frame 1..9 or 11+ of `SDBTNANM.SHP`.
**The frame-10 branch is a binary highlight-vs-default selector — no
animation cadence, no per-frame stepping.** Other frames in the SHP appear
to be unused on the main-menu paint path.

## Confidence

- **Content (decompile correctness):** HIGH. Branch verified in both
  decompiler and raw disassembly. Frame index `0xA` is a literal in the
  binary.
- **Identity (the byte at `+0xD8` is the predicate):** HIGH. Same hash
  table, same walk, same offset arithmetic between
  `WM_PAINT_Handler @ 0x00621E90` reader and `FUN_00608440 @ 0x00608440`
  setter.
- **Binding (FUN_00608440 is the only live writer):** MEDIUM-HIGH. Direct
  CALL xrefs confirmed. The clearer at `0x006084A0` and alt setter at
  `0x006240A0` were checked for indirect references (no data refs found),
  but Ghidra's xref index may be incomplete for orphan code regions. A
  runtime watchpoint on `[record+0xD8]` would close the remaining gap.
- **Semantic name ("highlight" vs "active" vs "post-init"):** LOW.
  Observed behavior is consistent with several plausible names; the
  binding to a specific UX state (focus, post-setup, etc.) is not
  determined from the binary alone. Recommend confirming by toggling the
  bit live and watching the rendered main menu.

## Unknowns / deferred

- Exact wMsg values handled by the dialog proc at `0x00792D00` that wraps
  the call at `0x00792DA6`: dispatch jump-table at `0x00792E48` decodes
  `WM_DESTROY (0x002)`, `WM_PAINT (0x00F)` → `CALL 0x00621E90`,
  `WM_QUIT/0x011`, `WM_DRAWITEM (0x02B)` → `CALL 0x006213A0`, and on the
  long path `WM_COMMAND (0x111)`, `WM_TIMER (0x113)`, and a custom message
  reached after `SUB EAX, 0x384` (numerically `wMsg = 0x499` if my chain
  of `SUB`s is right; not independently verified by finding a sender).
- Whether the four `FUN_00608440` call sites are in distinct dialog procs
  or in the same proc reached on different message paths.
- Whether any clearer is reachable via indirect call (function pointer in
  a vtable or dispatch table). Direct data-ref scan for `0x006084A0` and
  `0x006240A0` produced no matches; this is suggestive but not conclusive.
- The semantic UX meaning of `record[+0xD8] = 1` (highlight / accepted /
  post-init / something else).

## Symbol table (this report)

| Symbol | Address | Status |
|---|---|---|
| `RightPanel__Draw` | `0x0072E450` | named (prior reports) |
| frame-10 gate | `0x0072E5E6 TEST CL,CL` | confirmed |
| `WM_PAINT_Handler` | `0x00621E90` | named (prior reports) |
| predicate read | `0x00621FEC MOV DL,[EBX+0xD4]` | confirmed |
| predicate offset (from record root) | `+0xD8` | confirmed |
| `FUN_00608440` setter (+0xD8 = 1) | `0x00608440` | live; 4 callers |
| `FUN_006084A0` clearer (+0xD8 = 0) | `0x006084A0` | no xrefs (orphan) |
| alt setter (+0xD8 = 1) | `0x006240A0` | no xrefs (orphan) |
| subclass-record hash table | `[0x00AC1B00]` | confirmed |
| hash thunk | `[0x00AC1B18]` | confirmed |
| frame-10 literal push | `0x0072E635 PUSH 0xA` | confirmed |
| g_SDBTNANM_SHP pointer | `[0x00B0FAC4]` | confirmed |
