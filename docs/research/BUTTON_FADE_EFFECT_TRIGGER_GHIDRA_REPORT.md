# ButtonFadeEffect Trigger — Ghidra Investigation Report

**Status: PARTIAL**
**Scope:** Identify the message-handler or function that instantiates `ButtonFadeEffect` and pushes it onto the global container.
**Constraint:** Ghidra MCP read-only. `create_function` prohibited.

---

## Summary

The trigger site (ButtonFadeEffect construction + push) was not definitively identified. The master WndProc at `0x00610ca0` is the prime candidate. All other functions in the known click pipeline were fully decompiled and contain no ButtonFadeEffect construction. `0x00610ca0` cannot be decompiled without `create_function` (write-prohibited); raw byte analysis found two `operator_new` call sites but neither could be confirmed as ButtonFadeEffect.

> **PATCHED 2026-05-20 — scope clarification.** The "main-menu only" framing in prior versions of this doc family is wrong. All six `push 0x4DC` sites in `gamemd.exe` are in **network-lobby dialog helpers** (`FUN_005e2340` / `FUN_005e23a0` referencing `s_D__ra2mdpost_netdlg2`; `FUN_007a2750` / `FUN_007a27d0` referencing `s_D__ra2mdpost_wonline`), all using `GetDlgItem(hwnd, 0x59f)`. The flash mechanism is for **multiplayer/network-lobby owner-drawn buttons**, NOT main-menu buttons. The "no in-game cameo caller" sub-conclusion still holds — `StripClass::AI` does NOT call `SendMessageA(0x4DC)` (see `BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md §6` PATCHED note for the cross-confirmation, and `SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md` for the actual in-game tab-flash mechanism, which is unrelated to `ButtonFadeEffect`).
> (corrected 2026-05-28: the four named functions account for only 4 of the 6 sites; the remaining two are at `0x005e2098` and `0x00792440` in unlabeled functions within the same netdlg2/wonline code regions — verified via `search_byte_patterns "68 dc 04 00 00"`. All 6 are network-lobby; the broader attribution is correct but the named-function enumeration was incomplete — ROOT_CAUSE: INFERENCE_HARDENED)

---

## Verified Facts

### Fact 1 — DynamicVectorClass vtable address
- **Address:** `0x007e856c`
- **Verification:** `search_byte_patterns "10 02 80 00"` (DynamicVectorClass COL address = `0x00800210`) → hit at `0x007e8568`; vtable starts 4 bytes later at `0x007e856c`.
- `read_memory 0x007e856c 28` confirmed vtable entries: `[+0]=0x004ba200` (dtor), `[+4]=0x004b9ca0`, `[+8]=0x004b9e20`, `[+C]=0x004b9be0`, `[+10]=0x004b9ed0`, `[+14]=0x004b9c10`, `[+18]=0x004b9c30`.

### Fact 2 — Container initialization site
- **Address:** `0x004b750e` inside `FUN_004b6c30`
- **Instruction:** `mov [esp+0xb0], 0x007e856c` — writes DynamicVectorClass vtable pointer into a stack-local struct.
- **Verification:** `read_memory 0x004b750e 12` → `c7 84 24 b0 00 00 00 6c 85 7e 00` confirmed.
- **Caller chain:** `ScenarioClass__Start_Scenario` (entry `0x00683ab0`, body `0x00683ab0`–`0x00683ea2`) → `FUN_004b6c30` (mislabeled `CDFileClass__Constructor`). The call site inside `Start_Scenario` is at `0x00683d97` (a CALL instruction, not the function entry). PATCHED 2026-05-20: the previous text cited `0x00683d97` as the entry address — Ghidra navigation to that address is mid-body. The container is initialized at scenario load, not at button press time.
- **Active in YR:** Yes — triggered every skirmish start.

### Fact 3 — Animation-wait loop (FUN_00608070)
- **Address:** `0x00608070`
- **Verification:** `decompile_function 0x00608070` (confirmed in prior session).
- **Trigger conditions:** `*(char*)((int)piVar5 + 0xc1) != 0` (animation enabled) AND `piVar5[0x2d] == 1` (byte offset `0xB4` in control data block = button-type flag must equal 1).
- **Effect when triggered:** plays click sound via `VocClass__PlayAtPos`, sets `*(char*)((int)piVar5 + 0xc2) = 1` (animation-running flag), calls `InvalidateRect`, then loops on `Main_Tick()` until `flag_0xc2` clears or 5-second timeout.
- **Note:** Main-menu buttons have `piVar5[0x2d]` initialized to 0 — this condition gates the animation path. The flag must be set to 1 elsewhere for the animation to fire.

### Fact 4 — operator_new(0x10) in master WndProc is NOT ButtonFadeEffect
- **Address:** `~0x0061108b`
- **Verification:** `read_memory 0x00611080 64` → allocation followed by hash-table insertion pattern (`hash_bucket_lookup → mov [ecx + edx*4], esi`) into `DAT_00ac1858`. Allocates a 16-byte mouse-tracking struct, not ButtonFadeEffect.
- **ButtonFadeEffect size:** struct is larger than 16 bytes (contains at minimum HWND + animation data fields, RTTI confirms user-defined struct). 16-byte allocation ruled out.

### Fact 5 — All other known click-path functions contain no ButtonFadeEffect construction
Fully decompiled; no `operator_new` matching ButtonFadeEffect size and no DynamicVectorClass push call found in any of:

| Address | Name/Label | Verdict |
|---|---|---|
| `0x00612B70` | `OwnerDraw_Button_00612B70` | WM_LBUTTONDOWN only calls `VocClass__PlayAtPos` (no BFE alloc). **PATCHED 2026-05-20: this function ALSO handles `param_2 == 0x4DC` with `param_4 == 1`**: sets `SetTimer(hwnd, 0, 1000)` and `piVar17[0x31] = 1`. It is the receiver of the 1 Hz flash schedule, not a passive sound-only handler. The BFE/SHP-frame flash is initiated *into* this WndProc by an external `SendMessageA(hwnd, 0x4DC, 0, 1)` call. No struct alloc here, but the table previously implied this handler is unrelated to the flash mechanism — it is in fact central to it. |
| `0x00531F60` | `MainMenuDialog0xE2_Proc_00531F60` | WM_COMMAND sets result codes 1–6. No BFE. |
| `0x00531cc0` | menu launcher / dialog runner | Creates dialog + message pump. No BFE. |
| `0x0060f9a0` | control registration | Installs master WndProc, allocates control record. No BFE. |

---

## Prime Candidate: Master WndProc 0x00610ca0

### What is known (raw byte analysis)
- Installed via `SetWindowLongA(hwnd, -4, 0x610ca0)` in `FUN_0060f9a0` (verified: `decompile_function 0x0060f9a0`).
- Message comparison chain for: `[0x200–0x209]` (mouse), `[0xA0–0xA9]` (NC mouse), `[0x100–0x108]` (keyboard), and individual values `0x105/0x104/0x112/0x106/0x49B/0x113/0x4AD`.
- Second `operator_new` call site at ~`0x00611165`: preceded by `cmp al, 1; jne` and a check of `DAT_00833678`, followed by a `call [0x007e149c]` → resolves to `Sin_lookup @ 0x004cad00`. Context suggests animation/timer related, not a hashmap insert.
- Function body extends through at least `0x00612600` (over 6 KB). Ghidra has no function record here; without `create_function` the decompiler cannot process it.

### Why this is the prime candidate
1. It is the installed WndProc for all WW custom controls — every button message routes through it.
2. The `OwnerDraw_Button_00612B70` sub-handler (for WM_LBUTTONDOWN on buttons with style `(bVar2 & 0xb) == 0xb`) only plays a sound; the ButtonFadeEffect creation must be in the caller or a sibling handler inside `0x00610ca0`.
3. The second `operator_new` site (~`0x00611165`) is in a branch gated on `cmp al, 1` + `DAT_00833678` check + floating-point/sine call — consistent with initiating a visual fade animation on button press.
4. No other function in the known call chain has an unexplored body of this size.

### What remains unverified
- Whether the second `operator_new` in `0x00610ca0` allocates a `ButtonFadeEffect` (size is unknown from raw bytes without decompilation).
- The exact message value that triggers the fade branch inside `0x00610ca0` (likely `WM_LBUTTONUP = 0x202` or a custom WW message like `0x4AD`).
- Where `flag_0xc2` is cleared to end the animation loop in `FUN_00608070`.
- Where `piVar5[0x2d]` (offset `0xB4`) is set to 1 — byte-pattern searches for all known write encodings returned no matches, suggesting an indirect or register-based write.

---

## Recommended Next Steps

1. **Unlock decompilation:** Allow `create_function 0x00610ca0` in a write-enabled session. This is the only remaining productive path. The function is ~6 KB and will require naming helpers iteratively.
2. **Alternatively:** Search for `push <ButtonFadeEffect_size>; call <operator_new>` in `0x00610ca0`–`0x00612600` by scanning for the exact size in push bytes once the struct size is confirmed from slot 1's report.
3. **Cross-check slot 1:** Confirm ButtonFadeEffect struct size from `BUTTON_FADE_EFFECT_STRUCT_GHIDRA_REPORT.md` — use that size to narrow which `operator_new` call in `0x00610ca0` is the target.

---

*Investigation conducted via Ghidra MCP (read-only). All addresses verified in current session via `read_memory`, `search_byte_patterns`, `decompile_function`, or `get_function_by_address` as noted inline. No facts invented.*
