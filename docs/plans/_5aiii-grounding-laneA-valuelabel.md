# Slice 5a-iii — Lane A grounding: in-game Options slider value-label update mechanism

Status: VERIFIED-FROM-BINARY (gamemd.exe, image base 0x00400000). Research only; no code/annotations changed.
Scope: the value labels next to the in-game Options (0xBBB) sliders — GameSpeed (slider 0x529 -> label 0x671),
ScrollRate (0x52a -> 0x672), VisualDetails (0x52b -> 0x673), Difficulty (0x50f -> 0x670).

---

## 1. Verified value-label update mechanism (end-to-end)

The Options own dialog proc is `FUN_004E1FE0` @ 0x004E1FE0 (sole caller: `OptionsClass__ShowInGameDialog`
@ 0x004E1D34). On `WM_HSCROLL` (msg 0x114) with low-word(wParam)==5 (SB_THUMBTRACK), it:

1. Matches lParam (the slider HWND) against `GetDlgItem(dlg, 0x529/0x52a/0x52b/0x50f)` to pick the
   target value-label id (0x671 GameSpeed / 0x672 ScrollRate / 0x673 VisualDetails / 0x670 Difficulty)
   AND the per-control CSF-key pointer table.
2. **Shifts the high word of wParam (the new slider position) into an index** and reads a CSF *key*
   string pointer out of a control-specific table: `key_ptr = table_base[position]`.
3. Resolves that key to a localized wide string via `StringTable__LoadString` @ 0x00734E60
   (`__fastcall`; key pointer passed in ECX).
4. Sends `SendMessageA(label_hwnd, 0x4b2, 0, resolved_wide_string)` to push the text into the label.

### The decompiler hid the indexing — disassembly is authoritative

`decompile_function 0x004E1FE0` renders step 2-3 as a *fixed* call
`StringTable__LoadString(s_"D:\\ra2mdpost\\GameDlg.CPP", 0x1b2)`. That is a decompiler artifact.
The disassembly (`disassemble_function 0x004E1FE0`) shows the real per-position table indexing in the
WM_HSCROLL branch (0x004E2278..0x004E232B):

```
004e228e  SHR  EDI,0x10                              ; EDI = high word of wParam = slider position
004e2297  MOV  EDI,[EDI*0x4 + 0x822730]              ; GameSpeed table  -> CSF key ptr (label 0x671)
004e22b1  MOV  EDI,[EDI*0x4 + 0x82274c]              ; ScrollRate table -> CSF key ptr (label 0x672)
004e22cb  MOV  EDI,[EDI*0x4 + 0x822768]              ; VisualDetails    -> CSF key ptr (label 0x673)
004e22f6  MOV  EDI,[EDI*0x4 + 0x822774]              ; Difficulty       -> CSF key ptr (label 0x670)
...
004e230f  PUSH 0x1b2                                  ; param_4 = source line (diagnostics only)
004e2314  PUSH 0x822848                               ; param_3 = "D:\ra2mdpost\GameDlg.CPP" (diag only)
004e2319  XOR  EDX,EDX                                ; param_2 = NULL  (optional out-color ptr)
004e231b  MOV  ECX,EDI                                ; param_1 = CSF KEY POINTER  <-- the real input
004e231d  CALL 0x00734e60                             ; StringTable__LoadString
004e2322  PUSH EAX                                    ; lParam = resolved wide string
004e2325  PUSH 0x4b2
004e232b  CALL [0x007e14a4]                           ; SendMessageA(label, 0x4b2, 0, wstr)
```

So the CSF key (e.g. `TXT_MEDIUM`) is passed in **ECX**, which the decompiler dropped from its argument
list. `0x822848` ("GameDlg.CPP") and `0x1b2` (=434, source line) are only `StringTable__LoadString`'s
`param_3`/`param_4`, consumed solely in the missing-string error path.

### `StringTable__LoadString` @ 0x00734E60 — how (key, file, line) maps to a string

`wchar_t* __fastcall StringTable__LoadString(char* key /*ECX*/, u32* out_color /*EDX*/, char* file, int line)`:
- Looks the **key** up in the loaded CSF hash table (`FUN_007c8e25(key, ...)`).
- On hit: returns `*(wchar_t**)(DAT_00b1cf78 + entry[0x24]*4)` (the localized wide string); optionally
  writes a color word to `*out_color`.
- On miss: builds a `MISSING:<key>` placeholder and logs `NO_STRING: %s -> file %s line %d` using
  `file`/`line`. So `0x1b2`/GameDlg.CPP are diagnostic-only; `0x1b2` is **not** a key, format, or index base.

### Message 0x4b2 = "set this custom control's caption to the wide string at lParam"

Verified from the WWLib custom-control superclass proc `FUN_00610CA0` (handles 0x4b2 at the
`if (uVar13 != 0x4b2) goto ...` block). On 0x4b2 it: frees the control's old text buffer
(`piVar24[10]`), and if lParam is a non-empty wide string, allocates `len*2+2` bytes and copies the
string into `piVar24[10]`, clears a flag, and flags a redraw. The label **stores the supplied string
and renders it** — it does **not** read its buddy slider. lParam carries the already-resolved string;
the parent dialog proc owns the resolution. (Same 0x4b2 contract is reused for tooltips in
`FUN_00622B50`'s WM_NOTIFY/0x84 branch and in `FUN_00610CA0`'s WM_MOUSEMOVE/0x200 branch.)

Evidence: `decompile_function 0x004E1FE0`, `disassemble_function 0x004E1FE0`,
`decompile_function 0x00734E60`, `decompile_function 0x00610CA0`,
`read_memory 0x00822848` (= "D:\ra2mdpost\GameDlg.CPP"),
`read_memory 0x0082280c` (= "GameControls: GameSpeed = %d, ScrollRate = %d, Detail = %d\n", a debug log).

---

## 2. index -> CSF-key tables (verbatim keys read from memory)

Pointer tables are arrays of 4-byte little-endian pointers into the GameDlg key-string blob
(0x008227B0..0x00822808). Read via `read_memory 0x00822730` (80 bytes) and the pointed-to ASCII
strings via `read_memory 0x00822780/0x008227B0/0x00822800`.

Resolved key-string addresses (verbatim ASCII, NUL-terminated):
| addr       | key          |
|------------|--------------|
| 0x00822780 | `TXT_HARD`    |
| 0x0082278c | `TXT_NORMAL`  |
| 0x00822798 | `TXT_EASY`    |
| 0x008227a4 | `TXT_HIGH`    |
| 0x008227b0 | `TXT_LOW`     |
| 0x008227b8 | `TXT_FASTEST` |
| 0x008227c4 | `TXT_FASTER`  |
| 0x008227d0 | `TXT_FAST`    |
| 0x008227dc | `TXT_MEDIUM`  |
| 0x008227e8 | `TXT_SLOW`    |
| 0x008227f4 | `TXT_SLOWER`  |
| 0x00822800 | `TXT_SLOWEST` |

### GameSpeed @ 0x00822730 (7 entries) — label 0x671, slider 0x529
| pos | ptr        | key          |
|-----|------------|--------------|
| 0   | 0x00822800 | `TXT_SLOWEST` |
| 1   | 0x008227f4 | `TXT_SLOWER`  |
| 2   | 0x008227e8 | `TXT_SLOW`    |
| 3   | 0x008227dc | `TXT_MEDIUM`  |
| 4   | 0x008227d0 | `TXT_FAST`    |
| 5   | 0x008227c4 | `TXT_FASTER`  |
| 6   | 0x008227b8 | `TXT_FASTEST` |

### ScrollRate @ 0x0082274c (7 entries) — label 0x672, slider 0x52a
Identical sequence to GameSpeed: pos0 `TXT_SLOWEST` ... pos6 `TXT_FASTEST`
(same 7 pointers 0x00822800,f4,e8,dc,d0,c4,b8).

### VisualDetails @ 0x00822768 (3 entries) — label 0x673, slider 0x52b
| pos | ptr        | key         |
|-----|------------|-------------|
| 0   | 0x008227b0 | `TXT_LOW`    |
| 1   | 0x008227dc | `TXT_MEDIUM` |
| 2   | 0x008227a4 | `TXT_HIGH`   |

### Difficulty @ 0x00822774 (3 entries) — label 0x670, slider 0x50f (only when NOT in-game)
| pos | ptr        | key          |
|-----|------------|--------------|
| 0   | 0x00822798 | `TXT_EASY`    |
| 1   | 0x0082278c | `TXT_NORMAL`  |
| 2   | 0x00822780 | `TXT_HARD`    |

Note: GameSpeed/ScrollRate sliders are programmatically set at init to `6 - DAT_00a8eb60` /
`6 - DAT_00a8eb70` (range 0..6, 0x406 set with hi=0x60000 => 7 stops); VisualDetails/Difficulty use
range 0..2 (0x406 hi=0x20000 => 3 stops). The "6 -" inversion means stored value 3 -> slider pos 3 for
GameSpeed (TXT_MEDIUM at the midpoint), consistent with the 7-entry table above.

---

## 3. Resolution of OPTIONS_PROC_004E1FE0 ... REPORT.md section 4 discrepancy

**Verdict: the prior doc (section 4) was CORRECT; the current decompile is what misleads.**

- Doc section 4 claimed the proc "shifts the high word into an index and uses control-specific CSF
  pointer tables" with per-position keys TXT_SLOWEST..TXT_FASTEST, citing bases 0x00822730 (GameSpeed),
  0x0082274C (ScrollRate), 0x00822768 (VisualDetails). **All of this is verified true** in the
  disassembly (`disassemble_function 0x004E1FE0`, instructions at 0x004E228E/97, 0x004E22B1,
  0x004E22CB) and the memory reads above.
- The Ghidra **decompiler** folds the table lookup away and prints a fixed
  `StringTable__LoadString(s_"...GameDlg.CPP", 0x1b2)`, because the CSF key travels in **ECX** (the
  `__fastcall` `this`/first arg) which the decompiler omitted from the printed arg list, and it surfaced
  the two stack diagnostics args (file, line) instead. Reading only the decompile would wrongly conclude
  "fixed key 0x1b2." The disassembly resolves it.
- One addition the doc omitted: there is a **fourth** table at **0x00822774** for Difficulty
  (label 0x670, slider 0x50f), 3 entries TXT_EASY/TXT_NORMAL/TXT_HARD. This branch is gated on
  `!g_GameActive` (`byte [0x00a8e9a0]==0`), i.e. the Difficulty slider/label only exist when the dialog
  is opened outside a live game.

Evidence: `disassemble_function 0x004E1FE0`; `read_memory 0x00822730/0x00822768/0x00822780/0x008227B0/0x00822800`.

---

## 4. CRITICAL for the Rust port: labels are set on DRAG ONLY, never at open

**Definitive answer: at populate (default GameSpeed=3 -> slider pos 3), gamemd does NOT show
`TXT_MEDIUM`. The label keeps the dialog TEMPLATE's default caption ("GUI:Faster", i.e. the localized
"Faster" text) next to the GameSpeed and ScrollRate sliders until the user first drags that slider.**

Why, with evidence:

(a) **The proc never sends 0x4b2 to the labels at init.** The custom-init branch is `param_2 == 0x497`
   (`disassemble_function 0x004E1FE0`, 0x004E2028..0x004E2275). It sends only slider messages to the
   sliders: `0x4ac` (reset/clear) , `0x406` (set range, hi=0x60000 or 0x20000) and `0x405`
   (set position, `6 - DAT_00a8eb60` etc.) to 0x529/0x52a/0x52b/0x50f. It calls `GetDlgItem(...,0x671)`
   only to `ShowWindow(...,0)` (hide) it under certain modes — never `SendMessage(...,0x4b2,...)`.
   The labels 0x671/0x672/0x673/0x670 receive **no** text message at init.

(b) **0x4b2 to a label is emitted only from the WM_HSCROLL (0x114, SB_THUMBTRACK) branch** —
   i.e., user drag. Verified in section 1.

(c) **Programmatic SetPos does not generate WM_HSCROLL.** The custom slider proc (`FUN_006040b0`,
   which owns msgs 0x405/0x406/0x4ac) contains no `WM_HSCROLL`/0x114 send to its parent — the only
   `0x114` token in its body is an unrelated WOL dialog-id comparison. So pushing the position at init
   does not bounce a WM_HSCROLL back to the dialog, and the label-update path is not triggered at open.
   This matches standard Win32 (TBM_SETPOS / a programmatic set-pos does not notify the parent).

(d) **The label control does not self-initialize from its buddy.** In the WWLib superclass proc
   `FUN_00610CA0`, the label's rendered text is solely `piVar24[10]`, written only by 0x4b2. There is
   no code path that reads a buddy slider position to derive a caption. The static is passive: it draws
   whatever caption it currently holds (initially the template's), until a 0x4b2 overwrites it.

(e) **The sole caller adds nothing.** `OptionsClass__ShowInGameDialog` @ 0x004E1D34 only creates the
   dialog (`FUN_00622650`), runs the modal loop, and on OK applies + writes INI. It issues no
   SetDlgItemText / 0x4b2 to any label. (The two `GameControls: GameSpeed=%d,...` calls are debug logs,
   `read_memory 0x0082280c`.)

**Net behavior to reproduce in the Rust port:** the GameSpeed and ScrollRate value labels display the
template default string ("Faster") when the Options overlay first opens, regardless of the actual stored
value (default 3 -> midpoint). The label only switches to the position-correct CSF text (TXT_SLOWEST..
TXT_FASTEST) the moment the user starts dragging the corresponding slider (SB_THUMBTRACK). VisualDetails
(0x673) and Difficulty (0x670) labels behave the same way (template default until dragged). This is a
gamemd quirk, not a bug — faithfully reproduce it (stale "Faster" at open, live-correct after drag).

Evidence: `disassemble_function 0x004E1FE0` (0x497 branch + WM_HSCROLL branch), `decompile_function
0x004E1FE0`, `decompile_function 0x004E1D34`, `decompile_function 0x00610CA0`, slider-proc 0x114 scan
of `FUN_006040b0`.
