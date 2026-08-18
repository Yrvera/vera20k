# FUN_0060CF00 Dialog Background Pointer Table — Ghidra Research Report

**Address:** `0x0060CF00`  
**Confidence:** High (full decompile + assembly verified per-branch)  
**Active in YR:** Conditional per branch — see table below  
**Date:** 2026-05-19

---

## 1. Overview

`FUN_0060CF00` is called from the common shell init proc `FUN_00622B50` on
`WM_INITDIALOG`. Given the Win32 `HWND` as `param_1`, it looks up the dialog's
window-extra record in a global hash table, reads a type code from the record
(`piVar2[0x1c]`, the dialog class/type field), then writes three fields used by
`WM_PAINT_Handler @ 0x00621E90` to compose the parent background:

| Record field (int-slot) | Byte offset from piVar2−4 base | Role |
|---|---|---|
| `piVar2[0x1E]` | ESI+0x74 | Convert (PAL) object pointer |
| `piVar2[0x39]` | ESI+0xE0 | Small (640-width) parent background SHP |
| `piVar2[0x3A]` | ESI+0xE4 | Large (>640-width) parent background SHP |

**Assembly verification:** generic branch (`0x0060D20B`–`0x0060D224`):

```asm
0060d20b: CALL 0x0072e280          ; returns DAT_00B0FBCC (SHELL.PAL convert)
0060d210: MOV [ESI+0x74], EAX     ; piVar2[0x1E] = convert
0060d213: MOV EDX, [0x00b0fb50]   ; MNSCRNS.SHP
0060d219: MOV [ESI+0xe0], EDX     ; piVar2[0x39] = small SHP
0060d21f: MOV EAX, [0x00b0fa04]   ; MNSCRNL.SHP
0060d224: MOV [ESI+0xe4], EAX     ; piVar2[0x3A] = large SHP
```

Offset reconciliation: decompiler shows `piVar2` base = ESI−4, so
`piVar2[0x1E]` = piVar2_base+0x78 = ESI+0x74 ✓; `piVar2[0x39]` = ESI+0xE0 ✓;
`piVar2[0x3A]` = ESI+0xE4 ✓. The composition doc's cited offsets are correct.

The function **returns void** (no EAX return value). It only side-effects the
dialog record.

---

## 2. Full Dialog-ID → Background Assignment Table

`piVar2[0x1c]` is the type code read from the dialog record. In all branches
below, `piVar2[0x39]` receives the **small SHP** used at `g_ScreenWidth == 640`,
and `piVar2[0x3A]` receives the **large SHP** used at `g_ScreenWidth > 640`.
All convert-getter functions are single-return stubs returning a dedicated global.

Evidence for full decompile: `decompile_function 0x0060CF00`.
Evidence for each convert-getter: `decompile_function 0x0072????` (address per row).
Evidence for each SHP global: assembly `MOV [global],EAX` at loader site
(xrefs to each global + `get_assembly_context` at the write address).

| Type code (hex) | Type code (dec) | Convert getter | Convert global | Small SHP global | Small SHP filename | Large SHP global | Large SHP filename | Active in YR |
|---|---|---|---|---|---|---|---|---|
| `0x94` | 148 | `FUN_0072DAE0` | `DAT_00B0FCC4` | `DAT_00B0FA6C` | `FSBKGDLG.SHP` | `DAT_00B0FA6C` | `FSBKGDLG.SHP` (same) | No — FS-prefixed WOL/TS full-screen dialog |
| `0x103` | 259 | `FUN_0072D450` | `DAT_00B0FBA8` | `g_RadarFrameOpen_SHP` (`DAT_00B0FB34`) | `ASCRBKSM.SHP`/`ASCRBKMD.SHP` | `g_RadarFrameOpen_SHP` | (same) | Yes — in-game radar dialog |
| `0xBC7` | 3015 | `FUN_0072D450` | `DAT_00B0FBA8` | `g_RadarFrameOpen_SHP` | `ASCRBKSM.SHP`/`ASCRBKMD.SHP` | `g_RadarFrameOpen_SHP` | (same) | Yes — in-game radar dialog |
| `0x108` | 264 | `FUN_0072D820` | `DAT_00B0FBB4` | `g_MinimapMovie_SHP` (`DAT_00B0FB1C`) | `MPASCRNS.SHP` | `g_MinimapMovie_SHP` | `MPASCRNL.SHP` | Yes — in-game radar map |
| `0xBC6` | 3014 | `FUN_0072D450` | `DAT_00B0FBA8` | `g_RadarFrameOpen_SHP` | `ASCRBKSM.SHP`/`ASCRBKMD.SHP` | `g_RadarFrameOpen_SHP` | (same) | Yes — in-game radar dialog |
| `0x6B` | 107 | `FUN_0072D210` | `DAT_00B0FCD4` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FAB8` | `MnScrnLCustomizeBattle.shp` | Yes — Customize Battle shell dialog |
| `0x102` | 258 | `FUN_0072D030` | `DAT_00B0FCE0` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` | Yes — offline Skirmish |
| `0xBC` | 188 | `FUN_0072D030` | `DAT_00B0FCE0` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` | Yes — Co-op/Skirmish variant |
| `0xBD` | 189 | `FUN_0072D030` | `DAT_00B0FCE0` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` | Yes — Co-op/Skirmish variant |
| `0xC2` | 194 | `FUN_0072D030` | `DAT_00B0FCE0` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` | Yes — Skirmish variant |
| `0xC9` | 201 | `FUN_0072D030` | `DAT_00B0FCE0` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` | Yes — Skirmish variant |
| `0x113` | 275 | `FUN_0072CE50` | `DAT_00B0FCEC` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FAAC` | `MnScrnLCustomMatchLobby.shp` | No — WOL Custom Match Lobby |
| `0x114` | 276 | `FUN_0072CAB0` | `DAT_00B0FCF8` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA60` | `MnScrnLMyInformation.shp` | No — WOL My Information |
| `0x10E` | 270 | `FUN_0072C8D0` | `DAT_00B0FD04` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0F9E8` | `MultiplaySelection.shp` | No — WOL/MP selection dialog |
| `0x11C` | 284 | `FUN_0072C8D0` | `DAT_00B0FD04` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0F9E8` | `MultiplaySelection.shp` | No — WOL variant |
| `0x10F` | 271 | `FUN_0072C6F0` | `DAT_00B0FD10` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FB0C` | `LoginScreen.shp` | No — WOL Login |
| `0x11D` | 285 | `FUN_0072C6F0` | `DAT_00B0FD10` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FB0C` | `LoginScreen.shp` | No — WOL variant |
| `0xE6` | 230 | `FUN_0072C510` | `DAT_00B0FD1C` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA64` | `RegistrationScreen.shp` | No — WOL Registration |
| `0xF3` | 243 | `FUN_0072C510` | `DAT_00B0FD1C` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA64` | `RegistrationScreen.shp` | No — WOL Registration variant |
| `0xF4` | 244 | `FUN_0072C510` | `DAT_00B0FD1C` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA64` | `RegistrationScreen.shp` | No — WOL Registration variant |
| `0xE7` | 231 | `FUN_0072C330` | `DAT_00B0FD28` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA30` | `NewNick2.shp` | No — WOL New Nickname |
| `0x116` | 278 | `FUN_0072C150` | `DAT_00B0FD34` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA4C` | `BuddyList.shp` | No — WOL Buddy List |
| `0x117` | 279 | `FUN_0072BF70` | `DAT_00B0FD40` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA9C` | `quickmatch.shp` | No — WOL Quick Match |
| `0x112` | 274 | `FUN_0072BD90` | `DAT_00B0FD4C` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA98` | `AutoLoginQuery.shp` | No — WOL Auto-Login |
| `0x2BC` (700) | 700 | `FUN_0072BBB0` | `DAT_00B0FD58` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FB2C` | `WOLOptions.shp` | No — WOL Options |
| `0xD6` | 214 | `FUN_0072B9D0` | `DAT_00B0FD64` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FB54` | `WOLSoundOptions.shp` | No — WOL Sound Options |
| **generic (all others, including `0xE2`)** | — | `FUN_0072E280` | `DAT_00B0FBCC` | `DAT_00B0FB50` | `MNSCRNS.SHP` | `DAT_00B0FA04` | `MNSCRNL.SHP` | **Yes** — all standard shell dialogs including `0xE2` main menu |

---

## 3. Key Findings

### 3.1 Dialogs using MNSCRNS.SHP / MNSCRNL.SHP (generic branch)

Every dialog ID **not listed as a special case** falls into the generic branch
(bottom of function). These receive:

- Small parent: `DAT_00B0FB50` = `MNSCRNS.SHP`
- Large parent: `DAT_00B0FA04` = `MNSCRNL.SHP`
- Convert: `DAT_00B0FBCC` = SHELL.PAL convert (via `FUN_0072E280`)

This is the common shell background for all standard YR shell dialogs that don't
need a specific branded background, including the **initial main menu `0xE2`**.
Evidence: `decompile_function 0x0060CF00`, assembly `0060d20b–0060d224`.

### 3.2 Skirmish special case (`0x102`, `0xBC`, `0xBD`, `0xC2`, `0xC9`)

These five dialog IDs share a common block in the function:

- Small parent: `DAT_00B0FB50` = `MNSCRNS.SHP` (same as generic)
- Large parent: `DAT_00B0FA18` = `MnScrnLCoopGameSetup.shp`
- Convert: `DAT_00B0FCE0` = `MnScrnLCoopGameSetup.PAL` convert (via `FUN_0072D030`)

The 800-wide background (`MnScrnLCoopGameSetup.shp`) is only loaded when
`g_ScreenWidth == 800` (exact match). Above 800, the loader (`FUN_0072CF40`)
leaves `DAT_00B0FA18 = 0`. `Background_Overlay` then skips the draw silently
via `CC_Draw_Shape`'s null-SHP early return.
Evidence: prior report `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`.

### 3.3 Record field offsets — confirmed from assembly

Direct assembly verification at `0060d210–0060d224` (generic branch):

- `piVar2[0x1E]` → `[ESI + 0x74]` — convert pointer
- `piVar2[0x39]` → `[ESI + 0xE0]` — small (640) background SHP
- `piVar2[0x3A]` → `[ESI + 0xE4]` — large (>640) background SHP

The decompiler's `piVar2` base is `ESI − 4`, making all three int-slot indices
consistent with the byte offsets in the assembly. The composition doc's cited
offsets `[0x1E]`, `[0x39]`, `[0x3A]` are confirmed correct.

### 3.4 Dead-in-YR branches

The following branches are present in `FUN_0060CF00` but use WOL-only dialogs
not reachable in standard offline YR skirmish:

| Group | Dialog IDs | SHP | Reason not reachable |
|---|---|---|---|
| WOL Registration | `0xE6`, `0xF3`, `0xF4` | `RegistrationScreen.shp` | WOL account creation only |
| WOL Nick | `0xE7` | `NewNick2.shp` | WOL account creation only |
| WOL Login | `0x10F`, `0x11D` | `LoginScreen.shp` | WOL login path only |
| WOL MP Select | `0x10E`, `0x11C` | `MultiplaySelection.shp` | WOL lobby path only |
| WOL Info | `0x114` | `MnScrnLMyInformation.shp` | WOL profile path only |
| WOL Custom Match | `0x113` | `MnScrnLCustomMatchLobby.shp` | WOL Custom Match only |
| WOL BuddyList | `0x116` | `BuddyList.shp` | WOL only |
| WOL QuickMatch | `0x117` | `quickmatch.shp` | WOL only |
| WOL AutoLogin | `0x112` | `AutoLoginQuery.shp` | WOL only |
| WOL Options | `0x2BC` | `WOLOptions.shp` | WOL only |
| WOL Sound | `0xD6` | `WOLSoundOptions.shp` | WOL only |
| Unknown FS dialog | `0x94` | `FSBKGDLG.SHP` | Unverified origin; FS-prefix suggests TS-era FireStorm |

The in-game radar dialogs (`0xBC6`, `0xBC7`, `0x103`, `0x108`) and the
Customize Battle dialog (`0x6B`) **are** reachable in standard YR gameplay.

### 3.5 `g_RadarFrameOpen_SHP` is resolution-dependent

`g_RadarFrameOpen_SHP @ 0x00B0FB34` is loaded by `RadarBackground_SHPLoad`:
- At `g_ScreenWidth == 640`: `ASCRBKSM.SHP` (from `[0x00844C58]`)
- At `g_ScreenWidth != 640`: `ASCRBKMD.SHP` (from `[0x00844C5C]`)

Similarly, `g_MinimapMovie_SHP @ 0x00B0FB1C` is loaded by
`RadarTransitionMovie_SHPLoad`:
- At 640: `MPASCRNS.SHP` (from `[0x00844CA0]`)
- At non-640: `MPASCRNL.SHP` (from `[0x00844CA4]`)

These are stored identically to both `piVar2[0x39]` and `piVar2[0x3A]`, so
`Background_Overlay` always draws the same SHP regardless of screen width for
those dialogs (no separate large variant — the caller-side resolution switch
happens at load time, not in the draw path).

---

## 4. Return / Write Summary

`FUN_0060CF00(HWND hwnd)`:
- **Returns:** void
- **Writes:** three fields on the dialog window-extra record found by hash of `hwnd`:
  - `[ESI + 0x74]` = convert object pointer
  - `[ESI + 0xE0]` = small-parent SHP pointer  
  - `[ESI + 0xE4]` = large-parent SHP pointer
- **Reads:** `[ESI + 0x1C]` / `piVar2[0x1c]` = dialog type/class code (the switch input)
- Does NOT write any other fields. Does NOT set up child controls.
- Called once per dialog lifetime from `FUN_00622B50` on `WM_INITDIALOG`.

---

## 5. Open Questions — Final State

- `[RESOLVED] Q1` — Full dialog-id → (convert, small SHP, large SHP) table. → See §2.
  (evidence: `decompile_function 0x0060CF00` + per-row assembly + string table reads)
- `[RESOLVED] Q2` — Which YR dialogs use MNSCRNS/MNSCRNL? → All unmatched IDs
  fall into generic branch: `0xE2` (main menu), plus all other common-shell dialogs
  not in the special-case list. Evidence: `0060d20b–0060d224` assembly.
- `[RESOLVED] Q3` — Skirmish override SHP? → `MnScrnLCoopGameSetup.shp` for
  dialogs `0x102`, `0xBC`, `0xBD`, `0xC2`, `0xC9`. Evidence: `decompile_function 0x0060CF00`.
- `[RESOLVED] Q4` — Dead branches in YR? → All WOL-path dialogs (`0xE6`–`0x117`,
  `0x2BC`, `0xD6`) are not reachable in standard offline skirmish. `0x94` is
  unverified origin (TS-era FS-prefix); treat as dead. Evidence: WOL-gated dialog
  flow, `FS`-prefix naming consistent with TS FireStorm.
- `[RESOLVED] Q5` — Confirm `record[0x1E]`/`[0x39]`/`[0x3A]` offsets → confirmed
  via `get_assembly_context 0060d20b`. piVar2 base = ESI−4.
- `[DEFERRED] Q6` — Exact dialog semantic names for `0xBC`, `0xBD`, `0xC2`, `0xC9`
  (which are the four Skirmish-group dialogs beyond `0x102`). Category: out-of-scope;
  the SHP assignment is verified; the dialog resource IDs themselves are not
  investigated here. Next step: read those RT_DIALOG resources.
- `[DEFERRED] Q7` — Dialog `0x94` FSBKGDLG.SHP origin. Category: out-of-scope;
  appears TS/FireStorm-era; not active in standard YR. Next step: trace callers of
  `FUN_00622B50` from the `0x94` dialog to confirm TS-only path.

---

## 6. Sources

Ghidra MCP calls in this session:

- `decompile_function 0x0060CF00` — full function body
- `decompile_function 0x0072DAE0`, `0x0072D820`, `0x0072D450`, `0x0072D210`,
  `0x0072D030`, `0x0072CE50`, `0x0072CAB0`, `0x0072C8D0`, `0x0072C6F0`,
  `0x0072C510`, `0x0072C330`, `0x0072C150`, `0x0072BF70`, `0x0072BD90`,
  `0x0072BBB0`, `0x0072B9D0` — all convert-getter stubs
- `decompile_function 0x0072D460` (RadarBackground_SHPLoad),
  `0x0072D830` (RadarTransitionMovie_SHPLoad), `0x0072D300`
- `get_xrefs_to` at each SHP/convert global to find writer sites
- `get_assembly_context` at each writer address to extract ECX (filename ptr)
- `inspect_memory_content` at pointer table entries and string table regions:
  `0x00844D60–0x00844DC8`, `0x00844DCC–0x00844FFC`, `0x00844C58`, `0x00844CA0`
- `list_globals` for `g_RadarFrameOpen_SHP` and `g_MinimapMovie_SHP`
- `search_strings` for `RADARY`, `MNBTTN` (orientation check)

Prior reports cross-referenced (not re-investigated):

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
- `RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md`

SHP/PAL global address table (from prior reports, verified consistent):

- `DAT_00B0FB50` = `MNSCRNS.SHP`
- `DAT_00B0FA04` = `MNSCRNL.SHP`
- `DAT_00B0FA18` = `MnScrnLCoopGameSetup.shp`
- `DAT_00B0FBCC` = SHELL.PAL convert
