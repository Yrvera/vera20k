# Movies & Credits Dialog — Main_Game Case 4 Ghidra Report

Date: 2026-05-19

Scope: When the player clicks the Movies & Credits button on main menu dialog `0xE2`
(control `0x686`, return code 4), `FUN_00531CC0` returns 4, and `Main_Game @ 0x0052D9A0`
case 4 fires. This report identifies: (a) the dialog resource IDs and dialog procs
launched; (b) all child controls; (c) the cutscene playback path; (d) the Credits path;
(e) where the cutscene list comes from.

Active in YR: **Yes** (standard YR skirmish reachable path).  
No Rust code, INI files, or Ghidra annotations were modified.

---

## Executive Summary

Case 4 does NOT open a single movie-picker dialog. It opens an **intermediate panel**
(dialog `0x101`) that presents three sub-options: Sneak Preview, Movies, and Credits.
Clicking Movies from that panel opens a second dialog (`0x129`) which is the actual
cutscene picker. The cutscene list is populated from the `[Movies]` section of
`art(md).ini`. Credits launches a standalone scrolling-text renderer that reads
`CREDITSMD.TXT`.

---

## 1. Dispatch Chain from Main_Game Case 4

Verified via disassembly of `Main_Game @ 0x0052D9A0` jump table at `0x0052EB58`.
EAX = return-from-main-menu + 1 → index 5 (case 4) → target `0x0052DD93`.

```asm
0052dd93: PUSH 0x1
0052dd95: MOV EDX, 0x0052D790   ; dialog proc for 0x101
0052dd9a: MOV ECX, 0x101        ; RT_DIALOG resource ID = 0x101
0052dd9f: CALL 0x0060D380       ; generic dialog pump
```

**Verified via `get_assembly_context` at `0x0052DD93`.**

Compare: case 1 (single player) uses ECX=`0x100`, EDX=`0x52D640` — confirming the two
cases use different dialog IDs and procs despite both calling `FUN_0060D380`.

---

## 2. Dialog 0x101 — Movies & Credits Sub-Panel

### RT_DIALOG resource ID: `0x101`
### Dialog proc: `0x0052D790`

**Control layout (verified via disassembly of proc `0x0052D790`):**

The WM_COMMAND handler at `0x0052D7D2` computes `ECX = ctrl_id - 0x686` and indexes
a byte table at `0x0052D85C` to dispatch:

| Control ID | Offset from 0x686 | Byte index → return code | Tooltip key |
|---:|---:|---:|---|
| `0x686` | 0 | → `0x0052D816` → return `0x12` (back to main menu) | `STT:OptionsButtonBack` |
| `0x68D` | 7 | → `0x0052D7EC` → return `0xD` (Sneak Preview) | `STT:OptionsButtonSneak` |
| `0x68E` | 8 | → `0x0052D7FA` → return `0xE` (Movies) | `STT:OptionsButtonMovies` |
| `0x68F` | 9 | → `0x0052D808` → return `0xF` (Credits) | `STT:OptionsButtonCredits` |

Controls `0x687`–`0x68C` (offsets 1–6) use byte `0x04` → fall through (ignored).

**Tooltip keys verified** via `FUN_006040B0` tooltip lookup table for dialog ID `0x101`
(`grep` of decompile output).

**WM_PAINT:** proc delegates to `FUN_00622B50` first; if `FUN_00622B50` returns 0,
the proc handles WM_PAINT by `GetDlgItem(hwnd, 0x71A)` →
`SendMessage(0x71A, 0x4F0, 0, 0)` — same RA2TS Bink movie-panel draw as the main
menu. This means the left Bink panel stays active in the sub-panel.

---

## 3. Case Dispatch from Dialog 0x101 return codes

After `FUN_0060D380` returns, `Main_Game` dispatches on result via the same jump table
(`0x0052EB58`). Key cases verified via `get_assembly_context`:

### Case 0xD — Sneak Preview

```asm
0052de4c: PUSH EBX
0052de4d–0052de5f: set up args
0052de55: MOV ECX, 0x0082634C   ; "RENEGADE.BIK" (verified via inspect_memory)
0052de5a: MOV ESI, 0x4
0052de5f: CALL 0x005BED40       ; FUN_005bed40 = movie playback
```

Plays hardcoded `RENEGADE.BIK`. No INI lookup.  
**Verified:** `inspect_memory_content` at `0x0082634C` reads `RENEGADE.BIK\0`.

### Case 0xE — Movies (opens dialog 0x129)

```asm
0052de72: PUSH 0x1
0052de74: MOV EDX, 0x0052D870   ; dialog proc for 0x129
0052de79: MOV ECX, 0x129        ; RT_DIALOG resource ID = 0x129
0052de7e: CALL 0x0060D380       ; run movie-picker dialog
```

After `FUN_0060D380` returns with the selected movie data ptr (or cancel):

```c
if (result != cancel && result != 0) {
    EDI = result;
    EAX = EDI[+8];           // movie name string ptr from LB_GETITEMDATA
    call FUN_005BED40(...)   ; play selected movie
}
```

**Verified via `get_assembly_context` at `0x0052DE72`.**

### Case 0xF — Credits

```asm
0052ded3: MOV ESI, 0x4
0052ded8: CALL 0x004C3E30       ; Credits roll renderer
0052dedd: MOV ECX, 0xa83d10
0052dee2: PUSH 0x8263a8         ; "INTRO"
0052dee7: CALL 0x00721210       ; music lookup (stop/queue)
```

After credits, plays music `INTRO` via `FUN_00721210`.  
**Verified via `get_assembly_context` at `0x0052DED3`.**

---

## 4. Dialog 0x129 — Movie Picker

### RT_DIALOG resource ID: `0x129`
### Dialog proc: `0x0052D870`

**Child controls (verified via disassembly of proc `0x0052D870`):**

| Control ID | Win32 class | Role | Evidence |
|---:|---|---|---|
| `0x744` | ListBox | Cutscene picker list box | `GetDlgItem(hwnd, 0x744)` at `0x0052D879`; `LB_GETCURSEL = 0x188` at `0x0052D964`; `LB_GETITEMDATA = 0x199` at `0x0052D979` |
| `0x745` | Button (owner-draw) | Play button | WM_COMMAND ctrl `0x745` sets "play" flag; tooltip `GUI:PlayMovie` at `0x008352B4` |
| `0x686` | Button (owner-draw) | Back button | WM_COMMAND ctrl `0x686` writes `-1` (cancel) to result ptr |

The proc also handles message `0x497` (hover/focus) which calls `FUN_005FC000`
to populate the list box.

**WM_COMMAND logic:**

```
AND EAX, 0xFFFF         ; ctrl_id
SUB EAX, 0x686
JZ  → Back: write 0xFFFFFFFF to result ptr
SUB EAX, 0xBE           ; ctrl - 0x744
JZ  → list selection changed (SHR EBX, 16: update on LBN_SELCHANGE)
DEC EAX                 ; ctrl - 0x745
JNZ → ignore
    → Play: set play_flag = 1
```

**Play path:**

```c
LB_GETCURSEL(list_box)  → selected_idx
LB_GETITEMDATA(list_box, selected_idx)  → movie_data_ptr
write movie_data_ptr → result slot
```

`FUN_0060D380` returns `movie_data_ptr`. Case 0xE then checks it and calls
`FUN_005BED40` (the playback function).

**Tooltip keys for dialog `0x129` (verified via `FUN_006040B0` at `0x0060470D`):**

| Control | Tooltip key |
|---|---|
| `0x745` | `GUI:PlayMovie` (string at `0x008352B4`) |
| `0x744` | `GUI:SelectMovie` (string at `0x0083528C`) |
| `0x686` | `STT:MsnDltButtonBack` (string at `0x0083529C`) |

---

## 5. Cutscene List Source — `[Movies]` in art(md).ini

### List population: `FUN_005FC000 @ 0x005FC000`

Called with `(this = g_OptionsClass_or_similar @ 0x00A8EB60, list_hwnd = ESI)`.

It iterates a hardcoded table of `[Movies]`-section entries starting at `PTR_s_A00_F00E_00832C20`,
uses custom message `0x4CD` to add items, then `0x19A` (LB_SETITEMDATA) to attach
the movie name pointer as item data. The selected item's data pointer is what gets
passed to `FUN_005BED40`.

### INI section: `[Movies]`

**Parsed from:** `art(md).ini` — **not** `rules(md).ini`.

`FUN_00674550 @ 0x00674550` loads `[Movies]` from the passed INI object
(`&g_ArtINI` per caller `CDFileClass__Constructor @ 0x0052CD70`):

```c
iVar2 = FUN_00526810(PTR_s_Movies_007f0ce4);  // check [Movies] section exists
iVar2 = FUN_00526960(PTR_s_Movies_007f0ce4);  // count entries
// for each entry:
uVar3 = FUN_00526CC0(PTR_s_Movies_007f0ce4, iVar5);  // get key at index
CCINIClass__ReadString(PTR_s_Movies_007f0ce4, uVar3, &DAT_00817474, local_20, 0x20);
// store into DAT_00abf394[] array
```

**Verified:** section name `"Movies"` at `0x00839D50` confirmed via
`inspect_memory_content`.

**artmd.ini `[Movies]` entry count:** 59 entries (verified by reading
`C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`). First entry: `A00_F00e`.
Last entry: `S08_F01e`. These are all YR campaign cutscene base names.

**TS-filter:** The base `art.ini` `[Movies]` section contains older entries
(e.g., `CAP_TRAT`, `COUP`, `VEGAWIN`, `GDI_M02`, ...) inherited from TS/RA2.
`artmd.ini` overrides with the 59 YR-specific entries. Standard YR uses the merged
result, so `artmd.ini` entries take full priority.

**Active in YR:** Yes — `FUN_00674550` is called unconditionally from the
rules-init path which always runs at startup.

---

## 6. Movie Playback Path — `FUN_005BED40`

Entry point for all Movies & Credits playback: `FUN_005BED40 @ 0x005BED40`.
Signature: `void __fastcall FUN_005BED40(param_1, param_2, param_3, param_4, param_5)`.

**Bink branch (`.bik` extension detected):**

```c
pcVar4 = strrchr(local_100, '.');
if (FUN_007C8D20(pcVar4, &DAT_0082D9CC) == 0) {  // ".bik" at 0x0082D9CC
    Register_heap_pool("Play_Movie() as Bink!\n");
    FUN_00432690(local_100);   // open Bink file
    FUN_0040A7C0(2);           // ...
    FUN_00432C70();            // Bink main loop
    FUN_00432700();            // cleanup
}
```

`FUN_00432690` is the Bink file opener. `FUN_00432C70` is the Bink playback loop.

**VQA branch (fallback):**

```c
iVar5 = CDFileClass__Constructor(0,...);  // VQMovieHandle (0x005C07D0 path)
FUN_005BFF60(param_2, 0);                  // VQA playback loop
```

Identical extension-resolution logic as documented in
`MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`: `.BIK` tried first, `.VQA` fallback.
The playback path is shared between the movie picker and the Sneak Preview.

**Verified:** `decompile_function` of `FUN_005BED40` shows the `.bik` check against
`DAT_0082D9CC` and both branches.

---

## 7. Credits Path — `FUN_004C3E30`

Mislabeled `CDFileClass__Constructor` in Ghidra. Actual function: **Credits roll renderer**.

Entry: `0x004C3E30`. Body range: `0x004C3E30 – 0x004C494B`.

Key behavior:
1. Opens `CREDITSMD.TXT` via `CCFileClass__Constructor(s_CREDITSMD_TXT_0082084C)`
2. Parses text content (word-wrap, tab stops, color codes `{label}`)
3. Looks up CSF strings via `StringTable__LoadString` for `{label}` substitutions
4. Plays music track `CREDITS` via `FUN_00721210(s_CREDITS_008207E4)` then
   `FUN_00720B20` (start/play)
5. Scrolls entries upward until ESC (`0x1B`) pressed or all entries scrolled off

**File read:** `CREDITSMD.TXT` (YR variant). `Credits.CPP` source path at
`0x0081FBBC = "D:\\ra2mdpost\\Credits.CPP"`.

**Active in YR:** Yes — called unconditionally from case 0xF.

---

## 8. Verified Facts Summary

| # | Claim | Evidence |
|---|---|---|
| 1 | Main_Game case 4 sets ECX=`0x101`, EDX=`0x0052D790` before calling `FUN_0060D380`. | `get_assembly_context` at `0x0052DD93` |
| 2 | Dialog `0x101` proc at `0x0052D790` handles WM_COMMAND for controls `0x686` (back), `0x68D` (sneak), `0x68E` (movies→dialog `0x129`), `0x68F` (credits). | Disassembly of `0x0052D7D2`–`0x0052D820`; jump table at `0x0052D848`; byte table at `0x0052D85C` |
| 3 | Dialog `0x129` has list box `0x744`, Play button `0x745`, Back button `0x686`; Play uses `LB_GETCURSEL (0x188)` + `LB_GETITEMDATA (0x199)`. | Disassembly of `0x0052D870`–`0x0052D996` |
| 4 | Cutscene list is populated from `[Movies]` section of `art(md).ini`; 59 entries in `artmd.ini`, first `A00_F00e`, last `S08_F01e`. | `decompile_function` of `FUN_00674550`; section name verified via `inspect_memory_content` at `0x00839D50`; artmd.ini grep |
| 5 | Credits roller reads `CREDITSMD.TXT`, plays music `CREDITS`, is implemented in `FUN_004C3E30` (`CDFileClass__Constructor` label is wrong). | `decompile_function` of `0x004C3E30` |

---

## 9. TS-vs-YR Filter

| Item | Active in YR? | Notes |
|---|---|---|
| Sneak Preview (`RENEGADE.BIK`) | Conditional | File must exist in MIX archives; hardcoded filename |
| `[Movies]` section (artmd.ini) | Yes | 59 YR campaign movies; artmd.ini overrides art.ini |
| art.ini base `[Movies]` entries | No (overridden) | TS/RA2-era movies replaced by artmd.ini merge |
| Credits (`CREDITSMD.TXT`) | Yes | YR credits text file |
| `FinalMovie`, `SovFinalMovie`, `AllFinalMovie` INI keys | In-game only | Not in this dialog path; fired from win/defeat logic |

---

## 10. Unverified Items

- The exact layout (DLU rect, style flags) of RT_DIALOG `0x101` and `0x129` from
  the binary resource section — resource binary not inspected this session.
- Whether dialog `0x101` has a movie panel child `0x71A` or just the three buttons
  — the WM_PAINT path in proc `0x0052D790` calls `GetDlgItem(hwnd, 0x71A)` and
  sends `0x4F0` to it, implying yes, but the dialog template was not byte-parsed.
- The exact title/label text shown in the list box for each movie entry — it goes
  through `StringTable__LoadString` inside `FUN_005FC000` which suggests a CSF
  lookup per entry, but the key format was not traced.
