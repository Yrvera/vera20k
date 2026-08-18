# Static Control 0x71C Visibility Trace — Dialog 0xE2 (Main Menu)

**Investigation date:** 2026-05-19  
**Scope:** Every site in gamemd.exe that references control ID 0x71C; whether 0x71C
ever shows visible content on dialog 0xE2 in a stock-YR session.  
**Status:** COMPLETE

---

## 1. Executive Summary

**Control 0x71C never displays visible content during a standard YR main-menu session.**

- The only executable reference to control ID 0x71C in all of gamemd.exe is inside
  `ToggleMpScoreControls_0046DE20` (0x0046DE20–0x0046DFC6).
- `ToggleMpScoreControls_0046DE20` has **zero callers** — no CALL instruction or
  data pointer anywhere in the binary targets 0x0046DE20 (verified via
  `get_xrefs_to 0x0046DE20` and `search_byte_patterns 20 DE 46 00`).
- The function operates on a multiplayer post-game score dialog, **not** on dialog 0xE2.
  Its control IDs (0x732, 0x72f, 0x798–0x79b, 0x77e, 0x6d2–0x6db, 0x411–0x412,
  0x6a1–0x6a4, 0x6d1, 0x5a8, 0x71C, 0x510, 0x522, 0x468) are entirely absent from
  the dialog 0xE2 hit-test function (`FUN_006015e0`) and from both dialog-0xE2 lifecycle
  functions `FUN_00531CC0` and `FUN_0052B9B0`.
- **Active in YR: No.** `ToggleMpScoreControls_0046DE20` is dead code in a standard
  YR single-player or skirmish session. It would be reachable only from a
  network/multiplayer post-game score screen that never instantiates during a
  main-menu session.

**Rust port implication:** Control 0x71C on dialog 0xE2 can be rendered as
invisible/empty. No setup, no ShowWindow, no owner-draw callback is needed.

---

## 2. Search Strategy and Coverage

### 2a. Byte-pattern search — `push 0x71C` opcode

Pattern searched: `68 1C 07 00 00` (PUSH imm32 0x71C)  
Result: **one hit** — address `0x0046df98`  
(verified via `search_byte_patterns 68 1C 07 00 00`)

### 2b. Byte-pattern search — dword value 0x71C

Pattern searched: `1C 07 00 00`  
Hits in .text range: `0x0046df99` (part of the PUSH above), `0x00413fe0`,
`0x00442c9a`, `0x0044b06a`, `0x00513cd4`.

- `0x00413fe0` → `AircraftClass__InitFromType`: `0x71C` is a **struct field offset**
  on `AircraftTypeClass`, not a dialog control ID.
- `0x00442c9a` → `BuildingClass__Init_Managers`: same — `0x71C` is a struct offset
  on `BuildingTypeClass`, not a dialog ID.
- `0x0044b06a` → `BuildingClass__Mission_Attack`: `0x71C` is a struct offset
  (`this->Type + 0x71c`), not a dialog ID.
- `0x00513cd4` → no instruction (data region), not executable code.

All non-`0x0046df98` hits are struct field offsets, not control ID references.

### 2c. No SendMessage/PostMessage/GetDlgItem outside ToggleMpScoreControls

No additional `GetDlgItem(_, 0x71C)` call exists anywhere in .text.  
The single `push 0x71C` at 0x0046df98 is the only dialog-control usage.

---

## 3. ToggleMpScoreControls_0046DE20 — Full Analysis

**Address:** 0x0046DE20 – 0x0046DFC6  
(verified via `decompile_function 0046DE20`, `get_function_by_address 0046DE20`)

### 3a. Behavior

The function takes `(HWND param_1, int param_2)` and calls `GetDlgItem`/`ShowWindow`
on ~22 control IDs belonging to a multiplayer post-game score panel.

For control **0x71C specifically**, the show logic is **inverted**:

```c
pHVar1 = GetDlgItem(param_1, 0x71c);
if (pHVar1 != NULL) {
    ShowWindow(pHVar1, (uint)(param_2 == 0));
}
```

All other controls use `ShowWindow(pHVar1, param_2)`.  
0x71C is shown (`SW_SHOW = 1`) when `param_2 == 0`, and hidden when `param_2 != 0`.
This is consistent with 0x71C being a "no scores yet" placeholder or "awaiting data"
static that is visible only while the score panel is in its unset/empty state.

### 3b. Caller graph

- `get_xrefs_to 0x0046DE20` → **No references found**
- `search_byte_patterns 20 DE 46 00` → **No matches found**

The function has **zero callers**. It is dead code in the binary with respect to
any reachable code path in a standard YR session.

### 3c. Neighboring functions (context)

Functions immediately surrounding `ToggleMpScoreControls_0046DE20` in the 0x0046xxxx
module include:

- `CampaignScoreClass__Constructor` @ 0x0046D330 (called from 0x005C3B3B, 0x005C42F0)
- `FUN_0046d360`: formats elapsed-time scores
- `FUN_0046d8b0`: network player slot/country/team synchronization
- `FUN_0046dd70`: EnableWindow on lobby controls (0x411, 0x412, 0x732)

These are all part of the multiplayer/LAN-lobby/post-game score subsystem, not dialog 0xE2.

---

## 4. Dialog 0xE2 Lifecycle — No Reference to 0x71C

### 4a. FUN_00531CC0 (dialog 0xE2 init / message loop)

Called from `Main_Game` @ 0x0052DC9A (verified via `get_xrefs_to 00531CC0`).  
Sets up only **control 0x71A** (Bink movie static, SendMessageA with msgs 0x4E3/0x4E4).  
No reference to 0x71C.  
(verified via `decompile_function 00531CC0`)

### 4b. FUN_0052B9B0 (dialog 0xE2 refresh)

Sets up only **control 0x71A** (same as above).  
No reference to 0x71C.  
(verified via `decompile_function 0052B9B0`)

### 4c. FUN_006015E0 (hit-test / control routing for dialog 0xE2)

Explicitly checks `iVar5 != 0xe2` (where `iVar5` = dialog resource ID) in its
composite condition tree. The control IDs it references for dialog 0xE2 include
0x6D2, 0x3EA, 0x733, 0x71B, 0x732, 0x69D–0x69F, 0x6A0, 0x6A3–0x6A8 — **0x71C is absent**.  
(verified via `decompile_function 006015E0`)

### 4d. Main_Game flow

`Main_Game` (0x0052D9A0) calls `FUN_00531CC0` (dialog 0xE2) in the `iVar11 == 0x12`
(main-menu state) branch only. `ToggleMpScoreControls_0046DE20` is never invoked
from any path reachable from `Main_Game`.  
(verified via `decompile_function 0052D9A0`)

---

## 5. What Dialog Does ToggleMpScoreControls Target?

The function targets a **multiplayer post-game score dialog** (likely dialog 0x108
or similar, used in LAN/IPX game-end screens). Evidence:

- Control IDs 0x732, 0x72f appear in `FUN_0046DD70` (EnableWindow on lobby/score
  controls) and in `CampaignScoreClass` context.
- `CampaignEndScoreClass__Constructor` (0x00470C00) is called from 0x005C4539, which
  is in a multiplayer game-end flow using player-slot arrays (`DAT_00a8da78`) and
  campaign score timing.
- Control 0x77E appears in data at 0x0046EE18 (same module block).
- None of these IDs appear in the dialog 0xE2 WndProc or its known hit-test function.

---

## 6. Control 0x71C Physical Placement (from RT_DIALOG parse)

Per prior investigation (MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION):

- Dialog 0xE2 resource: control ID 0x71C, STATIC, owner-draw style 0x50000007
- Rect: left=447, top=29, width=61, height=33
- No title text

This rect overlaps the upper-right area where the YR logo static (0x71B) lives.
Despite existing in the RT_DIALOG resource, **no runtime code ever shows or updates
this control** during a dialog 0xE2 session. It remains hidden at all times.

---

## 7. Open Questions

None — the investigation is complete within scope. One note for awareness:

- It is possible 0x71C was a placeholder "Yuri's Revenge" logo control for a
  640×480 layout that was never activated (0x71B handles the logo at 800×600
  and 640×480). This is consistent with the inverted show logic in ToggleMpScoreControls:
  0x71C would be the "fallback visible" state when no score data is present, but since
  ToggleMpScoreControls is never called from dialog 0xE2, this branch never executes.

---

## 8. Verification Call Index

| Claim | Ghidra call |
|-------|-------------|
| push 0x71C at 0x0046df98 | `search_byte_patterns 68 1C 07 00 00` |
| ToggleMpScoreControls body | `decompile_function 0046DE20` |
| Zero callers of 0046DE20 | `get_xrefs_to 0046DE20` |
| Zero address refs 20 DE 46 00 | `search_byte_patterns 20 DE 46 00` |
| FUN_00531CC0 only touches 0x71A | `decompile_function 00531CC0` |
| FUN_0052B9B0 only touches 0x71A | `decompile_function 0052B9B0` |
| Dialog 0xE2 hit-test excludes 0x71C | `decompile_function 006015E0` |
| Main_Game calls 00531CC0 | `get_xrefs_to 00531CC0` + `decompile_function 0052D9A0` |
| Other 0x71C dword hits are struct offsets | `decompile_function 00413FE0`, `00442C9A`, `0044B06A` |
| 0x79b/0x798/0x732 only in score module | `search_byte_patterns 9B 07 00 00`, `32 07 00 00` |
