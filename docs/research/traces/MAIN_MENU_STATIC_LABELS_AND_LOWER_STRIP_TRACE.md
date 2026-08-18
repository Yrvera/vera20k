# Main-Menu Static Labels and Lower-Strip Trace
**Scope:** Dialog 0xE2 title heading, version line, tooltip line, lower-strip (LWSCRNL/LWSCRNS)  
**Resolutions traced:** 800×600 (base) and 640×480  
**Ghidra key function:** `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`

---

## Stage Results

### Stage 1 — Title heading TEXT (CSF key)
**FAIL**

Our code: `resolve_csf(state, "GUI:MainMenu")` — string key lookup.  
gamemd: The string `"GUI:MainMenu"` does **not appear anywhere** in the binary (verified via `search_strings` — zero matches). The title heading is a Windows static-text control (0x71a) whose text is set from the Windows RT_DIALOG resource template at dialog creation, not via a dynamic CSF lookup. The dialog proc only sends message `0x4f0` (custom repaint trigger) to control 0x71a on WM_PAINT — it never calls SetWindowText or StringTable__LoadString for the title. The text comes from the dialog template's embedded string, not from `GUI:MainMenu` in the CSF.  
**Evidence:** `decompile_function 0x00531F60` — no `StringTable__LoadString` call for 0x71a; `search_strings "GUI:MainMenu"` returns zero matches.

### Stage 2 — Title heading RECT at 800×600
**UNCHECKED**

Our code: DLU (425, 1, 108, 10) → `dlu_rect(BASE_X=6, BASE_Y=13)` → `mul_div_round(425,6,4)=638`, `mul_div_round(1,13,8)=2`, `mul_div_round(108,6,4)=162`, `mul_div_round(10,13,8)=16` → pixel (638, 2, 162, 16).  
gamemd: Dialog resource template for dialog 0xE2 is not directly decompilable from Ghidra (RT_DIALOG is in the PE resource section, not the code segment). The DLU formula and BASE_X/BASE_Y constants cannot be independently verified from Ghidra alone. UNCHECKED.

### Stage 3 — Title heading +7Y +1H NUDGE
**FAIL (unsubstantiated)**

Our code: constants `TITLE_HEADING_NUDGE_Y=7` and `TITLE_HEADING_NUDGE_H=1` in `layout.rs:115-116`. No citation or binary evidence is provided in comments.  
gamemd: The dialog proc (`0x00531F60`) never applies any pixel nudge to the title control rect. It sends `0x4f0` to 0x71a on WM_PAINT with no offset adjustment. The Windows dialog manager positions the static control at the DLU-derived rect from the dialog template. No nudge code path was found anywhere in the dialog proc or WM_PAINT handler for control 0x71a.  
**Evidence:** Full decompile of `0x00531F60` — no nudge applied; layout doc comment says "no doc citation" confirming this is an assumption.  
**Risk:** The title heading sits 7 pixels lower than the DLU position. If gamemd paints at the DLU-derived rect directly, our heading is 7 px low.

### Stage 4 — Title heading ALIGNMENT
**UNCHECKED**

Our code: `ShellAlign::H_CENTER` (top-anchored, not V_CENTER).  
gamemd: The title static 0x71a is drawn by the custom ownerdraw system. The text render flags (DT_CENTER vs DT_LEFT vs DT_VCENTER) are inside the ownerdraw paint handler for the shell static type, which is not directly traced here without deeper investigation of the child control's WndProc. Cannot determine alignment without decompiling the static control's paint message handler.

### Stage 5 — Version line TEXT FORMAT
**PASS**

Our code: `format!("{} {}", resolve_csf("GUI:Version"), state.version_txt)`  
gamemd: `MainMenuDialog0xE2_Proc_00531F60` WM_CTLCOLOR (0x497) handler:  
1. Calls `FUN_0074fae0()` — reads and caches VERSION.TXT content  
2. Calls `FUN_00735120()` — narrow-to-wide converter applied to VERSION.TXT content  
3. Calls `StringTable__LoadString(Init.CPP, 0x1757)` — loads "GUI:Version" text by numeric ID  
4. Calls `FUN_007ca564(buf, L"%s %s", csf_string, wide_version_txt)` — wide `sprintf` with format `%s %s`  
5. `SendMessageA(pHVar3, 0x4b2, 0, buf)` — sets control 0x71d text  

Format is `"{GUI:Version-text} {VERSION.TXT-content}"` exactly matching our code.  
**Evidence:** `decompile_function 0x00531F60`; `read_memory 0x00826960` → bytes `[0x25,0x00,0x73,0x00,0x20,0x00,0x25,0x00,0x73,0x00,0x00,0x00]` = UTF-16LE `%s %s`; `search_strings "GUI:Version"` → `0x82696c`.

### Stage 6 — Version line RECT at 800×600
**UNCHECKED**

Our code: DLU (425, 357, 108, 10) → ctrl 162×16; sidebar inset = (168-162)/2 = 3; X = screen_w - 3 - 162 - delta_x = 800 - 3 - 162 - 0 = 635; Y anchored to right_panel.bottom bottom edge.  
gamemd: The dialog template rect for control 0x71d cannot be verified from Ghidra code decompilation (requires RT_DIALOG resource parsing). UNCHECKED.

### Stage 7 — Tooltip line TRIGGER (hover-only vs always-drawn)
**FAIL (behavioral difference)**

Our code: tooltip text is only drawn when `hovered_button.is_some()` — no draw call at all when idle.  
gamemd: WM_NCHITTEST (0x84) fires on **every mouse move** (including over background), and always calls `SendMessageA(control_0x695, 0x4b2, 0, lParam)` with `FUN_007b7140()` returning the current tooltip. When not hovering a button, `FUN_007b7140()` returns `&DAT_00887734` which is a null pointer → empty wide string. So gamemd **always sets the tooltip control text** (to empty string on idle, to the STT: key text on hover).  
Visible result: identical (empty label = nothing visible). However: gamemd uses control ID **0x695** for the tooltip, not 0x71d. This is a different control than the version line. Our code maps this to `layout.tooltip_line` correctly in behavior, but the control ID difference means the dialog resource geometry (DLU-derived rect) for control 0x695 may differ from what we compute.  
**Evidence:** `decompile_function 0x00622b50` WM_NCHITTEST case: `GetDlgItem(param_1, 0x695)` + unconditional `SendMessageA`.

### Stage 8 — Tooltip line ALIGNMENT
**UNCHECKED**

Our code: `ShellAlign::H_CENTER` — 455 px wide rect, text centered.  
gamemd: The ownerdraw system for control 0x695 uses `FUN_007b7140()` to get a wide string then sends 0x4b2 to the control window. The underlying render flags in the control's paint handler are not traced. Cannot verify H_CENTER vs DT_LEFT without decompiling the control's WndProc.

### Stage 9 — Tooltip CSF keys
**PASS**

Our code maps: `0x683→STT:MainButtonSinglePlayer`, `0x684→STT:MainButtonWWOnline`, `0x578→STT:MainButtonNetwork`, `0x686→STT:MainButtonMovies`, `0x55c→STT:MainButtonOptions`, `0x3ee→STT:MainButtonExitGamemd`, `0x71b→STT:MainButtonYuriWebSite`.  
gamemd: `FUN_006040b0` (tooltip key resolver, called from WM_NCHITTEST) checks `iVar4 == 0xe2` (dialog ID) and returns:
- 0x683 → `s_STT_MainButtonSinglePlayer_00835784`
- 0x684 → `s_STT_MainButtonWWOnline_0083576c`
- 0x578 → `s_STT_MainButtonNetwork_00835754`
- 0x686 → `s_STT_MainButtonMovies_0083573c`
- 0x55c → `s_STT_MainButtonOptions_00835724`
- 0x3ee → `s_STT_MainButtonExitGamemd_00835708`
- 0x71b → `s_STT_MainButtonYuriWebSite_00833de4` (via `LAB_00605c3a`)

All 7 key strings verified in binary via `search_strings "STT:MainButton"`.  
**Evidence:** `search_strings "STT:MainButton"` → all 7 addresses confirmed; tool-results file b15up6kk2.txt confirms the dialog-0xe2 branch in `FUN_006040b0`.

### Stage 10 — Lower-strip ASSET selection threshold
**CONDITIONAL PASS / FAIL**

Our code in `lower_strip_rect()`: `if screen_w == 640 { 472 } else { 632 }` — uses `==` (matches gamemd).  
Our code in `build_chrome_instances()`: `if layout.screen.w <= 640 { atlas.lower_side_640_lwscrns } else { atlas.lower_side_large_lwscrnl }` — uses `<=`.  
gamemd: `RightPanel__Draw @ 0x0072E450`: `if (g_ScreenWidth == 0x280) { uVar1 = DAT_00b0fae8; } else { uVar1 = DAT_00b0fa54; }` — uses **exact equality `== 640`**.  
`RightPanel__ComputeLayoutRects @ 0x0072EC70`: `if (param_1 != 0x280)` for width selection — also exact equality.  
**Result:** Width selection in `lower_strip_rect` (== 640) matches gamemd. But `build_chrome_instances` uses `<= 640` which would select LWSCRNS for any screen width ≤ 640 (e.g. 320, 400, 480, 512 px wide) — gamemd only selects LWSCRNS for exactly 640 wide.  
**Evidence:** `decompile_function 0x0072E450`.

### Stage 11 — Lower-strip RECT dimensions and position
**PARTIAL PASS**

gamemd `RightPanel__ComputeLayoutRects`: lower-strip rect stored in `DAT_00b0fc2c`:
- X = `local_c` = `(screen_w > 800) ? (screen_w - 800) / 2 : 0` → 0 at 800×600
- Y = `local_4 - sVar2` where `local_4 = screen_h - (screen_h > 600 ? (screen_h-600)/2 : 0)` → `600 - strip_h` at 800×600
- Width = read from SHP header `*(short*)(LWSCRNL+2)` or `*(short*)(LWSCRNS+2)` — SHP-native width
- Height = read from SHP header `*(short*)(SHP+4)` — SHP-native height

Our code: width hardcoded as 472 (LWSCRNS) or 632 (LWSCRNL); height hardcoded as `LOWER_STRIP_H=32`. These must match the actual SHP header values. Hardcoded values are unverified assumptions.  
Position formula matches: X = `left_margin` (0 at 800 wide), Y = `top_margin + shell_h - LOWER_STRIP_H`.  
**Evidence:** `decompile_function 0x0072EC70` — width/height come from `*(short*)(SHP+2)` / `*(short*)(SHP+4)`.

### Stage 12 — Lower-strip PALETTE
**PASS (inferred)**

Our code: uses SHELL.PAL in the chrome atlas for all right-panel and lower-strip SHPs.  
gamemd: `CC_Draw_Shape(uVar1, 0, rect, origin, 0x400, 0, 0, 0, 1000, 0, 0, 0, 0, 0)` — flag `0x400` = use the shape's associated palette from the SHP load table. SHELL.PAL at `0x845454` is confirmed in the binary resource table adjacent to LWSCRNL.SHP (0x845104) and LWSCRNS.SHP (0x845110). The `Sidebar_RightPanel_SHP_Loading` data table loads LWSCRNL/LWSCRNS alongside SDBTM, SDTP, SDBTNBKGD — all of which use SHELL.PAL.  
**Evidence:** `search_strings "SHELL.PAL"` → `0x845454`; `read_memory 0x00845100` → shows LWSCRNL.SHP / LWSCRNS.SHP string block adjacent to SHELL.PAL region.

---

## Summary

| Stage | Result | Notes |
|-------|--------|-------|
| 1 Title TEXT | FAIL | "GUI:MainMenu" absent in binary; gamemd uses RT_DIALOG resource, not CSF key |
| 2 Title RECT | UNCHECKED | RT_DIALOG resource not parseable from Ghidra code section |
| 3 Title +7Y +1H NUDGE | FAIL | No nudge code in gamemd; assumed constant with no citation |
| 4 Title ALIGNMENT | UNCHECKED | Owner-draw paint handler not traced |
| 5 Version TEXT FORMAT | PASS | `%s %s` (GUI:Version + VERSION.TXT) confirmed |
| 6 Version RECT | UNCHECKED | RT_DIALOG resource not parseable |
| 7 Tooltip TRIGGER | FAIL | gamemd uses control 0x695 and always sets text; we skip draw on no-hover |
| 8 Tooltip ALIGNMENT | UNCHECKED | Owner-draw paint flags not traced |
| 9 Tooltip CSF KEYS | PASS | All 7 STT:MainButton* keys confirmed in binary |
| 10 Lower-strip ASSET SELECT | CONDITIONAL PASS | `lower_strip_rect` OK (==640); `build_chrome_instances` wrong (<=640) |
| 11 Lower-strip RECT | PARTIAL PASS | Position formula matches; width/height hardcoded, not SHP-derived |
| 12 Lower-strip PALETTE | PASS (inferred) | SHELL.PAL confirmed adjacent in resource table |

---

## Top 5 Player-Visible Failures

1. **Stage 1 — Title heading TEXT**: CSF key "GUI:MainMenu" used in our code but absent in gamemd binary. gamemd sets the title via RT_DIALOG resource text, not a CSF lookup. If "GUI:MainMenu" resolves correctly in the CSF file at runtime, this may be invisible — but if the CSF doesn't have that key, the title shows the key string literal ("GUI:MainMenu") instead of the localized title. Player sees wrong title text. Code: `src/app_main_menu_shell_render.rs:205`. gamemd evidence: `search_strings "GUI:MainMenu"` → zero matches; `decompile_function 0x00531F60`.

2. **Stage 3 — Title heading +7Y nudge**: Title heading painted 7 px lower than the DLU-derived Y position with no binary evidence. gamemd sends only repaint message 0x4f0 to control 0x71a — no offset adjustment. Player sees title heading 7 px too low. Code: `src/ui/main_menu_shell/layout.rs:115-116`. gamemd evidence: `decompile_function 0x00531F60` — no offset applied.

3. **Stage 7 — Tooltip control ID mismatch**: Our tooltip maps to `layout.tooltip_line` (computed from DLU 2, 355, 303, 12) but gamemd uses control **0x695** for tooltip — not 0x71d (version) or the same DLU as above. If dialog template has control 0x695 at a different DLU position than our computed rect, the tooltip label renders at wrong screen position. Player sees tooltip at wrong Y or X. Code: `src/ui/main_menu_shell/layout.rs:219` (tooltip_line_rect), `src/app_main_menu_shell_render.rs:219-228`. gamemd evidence: `decompile_function 0x00622b50` WM_NCHITTEST case, `GetDlgItem(param_1, 0x695)`.

4. **Stage 10 — Lower-strip threshold (<=640 vs ==640)**: `build_chrome_instances` selects LWSCRNS for any `screen_w <= 640`, but gamemd uses exact `== 640`. At resolutions like 400×300 or 512×384 (non-standard but possible), the wrong SHP variant would be drawn. Player sees wrong lower-strip graphic. Code: `src/app_main_menu_shell_render.rs:173`. gamemd evidence: `decompile_function 0x0072E450` — `g_ScreenWidth == 0x280`.

5. **Stage 11 — Lower-strip width/height hardcoded**: Width (472/632) and height (32) are hardcoded in `LOWER_STRIP_H` and `lower_strip_rect`. gamemd reads these from the SHP header at `*(short*)(SHP+2)` and `*(short*)(SHP+4)`. If the actual SHP dimensions differ from the hardcoded values, the strip is stretched or clipped. Code: `src/ui/main_menu_shell/layout.rs:91,204`. gamemd evidence: `decompile_function 0x0072EC70`.

---

**PASS: 3 | FAIL: 3 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0**

Status: **PARTIAL** — Stages 2, 4, 6, 8 are UNCHECKED (require RT_DIALOG resource parsing or owner-draw WndProc tracing not reachable from Ghidra code decompilation alone).
