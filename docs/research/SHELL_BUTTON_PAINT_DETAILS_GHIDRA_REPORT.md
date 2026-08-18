> ⚠️ **YELLOW — DIALOG 0xE2 ATTRIBUTION REFUTED 2026-05-19**
>
> This report's claim that **dialog 0xE2 (the standard YR main menu) uses the PCX path** (`bue_*30` / `bde_*30` 3-piece composites) is **wrong**.
>
> Refuted by `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md`: `LAB_0060A330` writes `record+0xB0 = 1` (= `piVar17[0x2c] = 1`) for all 6 main-menu button IDs (0x683/0x684/0x578/0x686/0x55C/0x3EE), routing them through `OwnerDraw_Button_00612B70`'s `iVar14 == 1` branch → `g_SDBTNANM_SHP` frames 2/3/4 (colored gradient art), **not** the PCX path documented below.
>
> Where this report's findings DO apply: the PCX path itself (`iVar14 == 0 && piVar17[5] == 0`) is real and live for OTHER WW custom controls — its disabled-color computation, pressed text/art offsets, alignment math, and CSF key resolution are all accurate. Just not for the offline main menu shell.
>
> The 1 Hz hover flash mechanism documented in this report's Section §Q4 ("hover state visual changes — NONE") is also **wrong for the iVar14 == 1 path**. The SDBTNANM dispatch DOES consult `+0xC5` (the WM_TIMER-toggled flash bit), producing a 1 Hz frame 2 ↔ frame 3 hover flash. See `BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md` for the verified mechanism.
>
> Confidence labels in this report should be read as "HIGH for non-main-menu PCX shell buttons; UNKNOWN for dialog 0xE2 specifically." Treat any "Active in YR" statement here as referring to the PCX branch, not to 0xE2.

---

# Shell Button Paint Details — Ghidra Research Report

**Address(es):** `0x00612B70` (OwnerDraw_Button), `0x00621040` (Shell text wrapper), `0x00434120` (BitFont glyph blit), `0x00434CD0` (BitFont DrawWithWrap), `0x0060F9A0` (Owner-draw common init), `0x00616980` (RadioVariant), `0x0072a8b0` / `0x0072a8e0` / `0x0072a900` / `0x0072a920` (palette-derived disabled-color writers)

**Confidence:** HIGH for all five questions — every claim is sourced from live Ghidra decompilation in this session and cross-checked against assembly at the relevant call site. **CAVEAT (added 2026-05-19):** the "Active in YR" claim for dialog 0xE2 is refuted; see YELLOW banner above. The findings apply to the PCX path generally, NOT to the main-menu shell specifically.

**Active in YR:** YES for every finding below **on the PCX-button path** (e.g., skirmish setup, options dialog). For the main-menu shell (dialog 0xE2 specifically) the SDBTNANM path is used instead — see banner above. `OwnerDraw_Button_00612B70` is the live owner-draw subclass dispatched by `FUN_0060F9A0` for `class="Button"` controls with style low bits `0xB`. `bue_*30` / `bde_*30` are confirmed loaded via `b%c%c_li30.pcx` etc., but the main menu doesn't reach that branch.

Scope: targeted follow-up to `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`. The five remaining unknowns:

1. Disabled-state **text** color
2. Pressed-state **text** Y-offset
3. Trailing args `(5, 0x0C, 0, 0, 0)` to `FUN_00621040`
4. Hover-state visual changes
5. Meaning of the `'e'` second character in `bue_*30/bde_*30`

No claims about asset family rules, click-sound binding, art height/positioning, or PCX palette decoding — those are already settled in the anchor doc.

## 1. Question 1 — Disabled-state TEXT color

**Answer:** The disabled-state text uses a **palette-derived dark-red value**, not greyed-out and not the static-control disabled color. After the text is drawn, a 50% black alpha overlay is applied across the entire button rect (text + art together).

### Evidence

`OwnerDraw_Button_00612B70` reads the default RGB color global into `EDI` at `0x00612da9`:

```
00612da9: MOV EDI, [0x00ac18a4]      ; EDI = DAT_00ac18a4 = 0x0000FFFF (yellow, R=FF G=FF B=00)
```

This is the same global initialized by `FUN_0060F9A0` at `0x0060fa14` to `0xFFFF` (verified live). For **enabled** buttons, EDI is unchanged through to the text call.

For **disabled** buttons (`uStack_ac & 0x8000000 != 0` where `uStack_ac = GetWindowLongA(hwnd, GWL_STYLE)`), the path at `0x00612f5f..0x00613136` overwrites EDI with a palette-derived value selected by `[g_ScenarioClass_Instance + 0x30d8]` and `[+0x34b8]`:

| Branch | Source addresses | Bytes (R, G, B) |
|---|---|---|
| `scenario[0x30d8] != 0 && scenario[0x34b8] == 0` | `DAT_00b0f9fc..fe` | written by stub `0x0072a8e0`: `{0x00, 0x52, 0x75}` = **#005275** teal |
| `scenario[0x30d8] != 0 && scenario[0x34b8] == 1` | `DAT_00b0fb14..16` | written by stub `0x0072a900`: `{0x48, 0x00, 0x00}` = **#480000** dark red |
| `scenario[0x30d8] != 0 && scenario[0x34b8] != 0,1` | `DAT_00b0fb19..1b` | written by stub `0x0072a920`: `{0x48, 0x00, 0x00}` = **#480000** dark red |
| `scenario[0x30d8] == 0` (Branch B) | `DAT_00b0fa94..96` | written by stub `0x0072a8c0`: `{0x48, 0x00, 0x00}` = **#480000** dark red (corrected 2026-05-28: was `0x0072a8b0`; function-pointer table at `0x00815450` shows stubs start at `0x0072a8c0`; raw bytes at `0x0072a8b0` are a 4-byte `STI;MOV AL,0;RET` stub unrelated to color — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT via `read_memory 0x0072a8b0..0x0072a93f + table read_memory 0x00815450`) |

(Stub bodies decoded from `read_memory` at `0x0072a8b0..0x0072a93f`; function-pointer table confirmed via `read_memory 0x00815450`. Stubs are 0x20 bytes apart beginning at `0x0072a8c0`. Each stub writes 3 bytes for a 24-bit color.)

For the **main menu shell** (no scenario active, `scenario[0x30d8] == 0`), Branch B fires: disabled text RGB = **`{R=0x48, G=0x00, B=0x00}` = #480000 (dark red)**.

The color is packed to the display 16-bit format, then re-extracted back to a 24-bit RGB with display-quantization loss (negligible for these values), OR'd with `0x02000000`, and pushed as the color arg to `FUN_00621040` at `0x006135e1` (`PUSH EDI`).

After text drawing returns, the same disabled-state branch at `0x006135fd..0x0061361b` calls `AlphaBlendRect(rect, surface, color=0, alpha=0x80)` over the **entire button rect** — both art AND text get a 50% black overlay. Effective on-screen disabled appearance: a half-darkened dark-red label on a half-darkened yellow art family.

### Comparison to other owner-draw controls

- `OwnerDraw_Static_006153E0` (line in decomp): when disabled, uses `iVar6 = DAT_00ac1cb4 = 0x0000009F` = R=0x9F G=0x00 B=0x00 = **#9F0000** for the disabled text color. (Initialized in `FUN_0060F9A0` at `0x0060fa14..` to `0x9f`.)
- `OwnerDraw_Checkbox_006163A0` (line: `if (disabled) uVar5 = DAT_00ac1cb4;`): same `#9F0000`.

**Button is NOT one of those.** The Button has its own palette-derived path producing the `#480000` family above, not the static `#9F0000` global.

### Active in YR

YES — the disabled-text alpha overlay path is reached by any standard YR main-menu button when disabled. Branch B is the live path for the main menu (no scenario).

### Tiny details

- The disabled-color recomputation is reached **before** the text-draw call. EDI is the carrier.
- Note `OR EDI, 0x200` at `0x00613123`: this sets bit 9 of the high byte (= bit 25 of the 32-bit color). It survives into the FUN_00621040 color extract because only the low 24 bits (R, G, B) are read. Bit 25 has no observable effect.
- The text color is NOT recomputed for SDBTNANM-style buttons (`piVar17[5] != 0`) — those follow a different draw path and may not draw text at all.
- The alpha overlay uses `AlphaBlendRect(0, 0x80)` (the **`0`** = black blend color, alpha 128/255 = 50%). This is the same alpha as the disabled SHP overlay path on Static/RadioVariant.

## 2. Question 2 — Pressed-state TEXT Y-offset

**Answer:** When pressed, the text rect is shifted by `(left+=2, top+=4, right+=0, bottom+=0)`. Combined with the `flags=5` (h-center + v-center) used by `FUN_00621040`, this resolves to an **effective +2 px down and +1 px right** for the rendered text glyph baselines vs. unpressed.

### Evidence

`OwnerDraw_Button_00612B70` at `0x0061358d..0x006135cd` builds the text rect from the window geometry:

```
0061358d: MOV ECX, [ESP+0x34]           ; ECX = window_top
00613591: MOV EAX, [ESP+0x30]           ; EAX = window_left
00613595: MOV [ESP+0x18], EAX           ; rect.left = window_left
00613599: LEA EDX, [ECX + 0x1]
0061359c: MOV [ESP+0x1c], EDX           ; rect.top = window_top + 1
006135a0: MOV EDX, [ESP+0x38]           ; EDX = window_width
006135a4: LEA EDX, [EDX + EAX*0x1 + -0x2]
006135a8: MOV [ESP+0x20], EDX           ; rect.right = window_left + window_width - 2
006135ac: MOV EDX, [ESP+0x3c]           ; EDX = window_height
006135b0: ADD EDX, ECX
006135b9: MOV [ESP+0x24], EDX           ; rect.bottom = window_top + window_height
006135b6: TEST CL,0x1                   ; CL = pressed bit (piStack_dc & 1)
006135bd: JZ 0x006135d1                 ; not pressed → skip
006135bf: ADD EAX, 0x2                  ; rect.left += 2
006135c2: MOV [ESP+0x18], EAX
006135c6: MOV EAX, [ESP+0x1c]
006135ca: ADD EAX, 0x4                  ; rect.top += 4 (so total +5 from window_top)
006135cd: MOV [ESP+0x1c], EAX
```

### Resulting text rect (pixel-exact)

| State   | left              | top              | right                       | bottom                |
|---------|-------------------|------------------|-----------------------------|-----------------------|
| Up      | window_left       | window_top + 1   | window_left + width - 2     | window_top + height   |
| Pressed | window_left + 2   | window_top + 5   | window_left + width - 2     | window_top + height   |

The bottom does NOT shift when pressed. Therefore the rect **shrinks** vertically (top moves down 4, bottom unchanged).

### Vertical center math (flags=5 → v-center bit 0x04)

`FUN_00621040` measures the text with `BitFont__MeasureText`, then:

```
param_5 = param_5 + (iVar12 - text_h) / 2;     // y = rect_top + (rect_h - text_h)/2
```

where `iVar12 = rect.bottom - rect.top` is the rect height.

- **Up**: y_up = (window_top + 1) + ((window_height - 1) - text_h) / 2
- **Pressed**: y_pressed = (window_top + 5) + ((window_height - 5) - text_h) / 2
- **Δy** = y_pressed - y_up = 4 + ((-4)/2) = **+2 px down**

### Horizontal center math (flags=5 → h-center bit 0x01)

`FUN_00434CD0` per-line flush:
```
if (line_x < max_width):
  if (flags & 1): local_24 = (max_width - line_x) / 2
draw_x = rect.left + local_24
```

- **Up**: rect_width = window_width - 2; x_up = window_left + ((window_width - 2 - text_w) / 2)
- **Pressed**: rect_width = window_width - 4; x_pressed = (window_left + 2) + ((window_width - 4 - text_w) / 2)
- **Δx** = x_pressed - x_up = 2 + ((-2)/2) = **+1 px right**

### Active in YR

YES — fires every time the player clicks-and-holds any main-menu shell button.

### Tiny details

- The pressed-art shift is **+2 px down** (per anchor doc). The pressed-text shift is **+2 px down, +1 px right**. So text and art move TOGETHER vertically (both shift +2 px down), but text **also** moves +1 px right while art does not. This horizontal asymmetry is a small parity detail likely invisible to most players but should be reproduced.
- The pressed bit is `piVar17[0x3a] & 1` (control state offset `0xE8`, low bit of dword). `WS_DISABLED` (`0x08000000`) on the window style forces it to 'u' (unpressed), overriding the pressed bit.
- The `+1` on `rect.top` in the unpressed case (vs. `+0`) is an intentional 1-px top inset shared with the prior-doc's art Y formula (`(client_h - art_h) / 2`). It's not a centering rounding artifact — it's hardcoded.
- The `-2` on the right edge (`rect.right = window_left + window_width - 2`) is a 2-px right inset — the text area is 2 px narrower than the full button width.

## 3. Question 3 — Trailing args `(5, 0x0C, 0, 0, 0)` semantics

**Answer (final calling convention):** `FUN_00621040` is `__fastcall` (ECX=surface, EDX=text), followed by 8 stack arguments. The third positional is the color; the fourth is the alignment flag byte; **args 5 and 6 are dead (never read by the function body)**; args 7 and 8 are the BitFont fade_count and fade_range, both 0 here.

### Verified signature

```
__fastcall FUN_00621040(
  ECX:   XSurface* surface
  EDX:   wchar_t*  text
  arg_1: RECT*     text region
  arg_2: BitFont*  font (the BitFont instance, e.g. g_GAME_FNT)
  arg_3: u32       color (low 24 bits = 0x00BBGGRR)
  arg_4: u8        flags (bit 0x01 = h-center, 0x02 = h-right, 0x04 = v-center)
  arg_5:           UNUSED (dead arg — not read in function body)
  arg_6:           UNUSED (dead arg — not read in function body)
  arg_7: i32       fade_count (typewriter-reveal char count; 0 = no fade)
  arg_8: i32       fade_range (typewriter-reveal fade band width)
)
```

### Evidence — call-site assembly at `0x006135d1..0x006135ee`

```
006135d1: MOV EAX, [EBP+0x64]      ; EAX = BitFont* (control state offset 0x64)
006135d4: PUSH 0                    ; → arg_8 = fade_range = 0
006135d6: MOV EDX, [EBP+0x28]       ; EDX = wchar_t* text (control state offset 0x28)
006135d9: PUSH 0                    ; → arg_7 = fade_count = 0
006135db: PUSH 0                    ; → arg_6 = 0 (DEAD)
006135dd: PUSH 0xc                  ; → arg_5 = 0x0C (DEAD)
006135df: PUSH 0x5                  ; → arg_4 = flags = 5 (h-center | v-center)
006135e1: PUSH EDI                  ; → arg_3 = color (yellow for enabled, dark-red for disabled)
006135e2: LEA ECX, [ESP+0x30]       ; ECX = local rect pointer
006135e6: PUSH EAX                  ; → arg_2 = BitFont* (font)
006135e7: PUSH ECX                  ; → arg_1 = RECT*
006135e8: MOV ECX, [0x00887310]     ; ECX (fastcall arg 1) = surface global
006135ee: CALL 0x00621040
```

### Evidence — function-body usage at `0x00621040`

Assembly trace shows exactly four stack args are read by the body:

| Source offset (post-prologue) | Use |
|---|---|
| `[ESP+0x24]` (arg_1) | ESI = RECT* — reads `[ESI+0x0]`/`[ESI+0x4]`/`[ESI+0x8]`/`[ESI+0xc]` for left/top/right/bottom |
| `[ESP+0x28]` (arg_2) | EBX = BitFont* — used as `this` for SetEnable / SetClipRect / SetColor and passed as DrawWithWrap param_1 |
| `[ESP+0x1c]` / `[ESP+0x20]` (arg_3) | EAX = color RGB — split into R (byte 0), G (byte 1), B (byte 2), each shifted by g_DD_*Loss/*Shift to display format |
| `[ESP+0x30]` (arg_4) | flags — bit 0x04 tested for v-center; full byte passed unchanged as DrawWithWrap param_8 |
| `[ESP+0x3c]` (arg_7) | passed as DrawWithWrap param_9 (fade_count) |
| `[ESP+0x40]` (arg_8) | passed as DrawWithWrap param_10 (fade_range) |

**No instruction in `FUN_00621040` reads `[ESP+0x34]` (arg_5 = 0x0C) or `[ESP+0x38]` (arg_6 = 0).** Both are pure spill slots — dead.

### NO shadow / outline pass — verified

`FUN_00434120` (BitFont per-glyph blit, decompiled at `0x00434120`) does ONE pass of glyph bits → 16-bit color writes. There is no preliminary dark-color-at-offset draw and no second outline pass. Each set bit in the glyph bitmap writes `outer[+0x24]` (the configured text color) to the destination pixel. Cleared bits leave the destination unchanged. **Shell button text has no shadow and no outline.**

The full DrawWithWrap function (`0x00434CD0`) likewise calls `FUN_00434120` once per character; no second tinted pass.

### Active in YR

YES — every WM_PAINT of every shell PCX button reaches this exact call shape.

### Tiny details

- Flag `0x05` decoded: `0x01 (h-center) | 0x04 (v-center)`. Bit `0x02` (h-right) is NOT set. There is no h-left bit — left-align is the default (no h-center, no h-right).
- The `0x0C` constant at arg_5 appears in MULTIPLE callers: `OwnerDraw_Button_00612B70` passes the literal `0xC`; `OwnerDraw_Static_006153E0` passes `piVar11[0x2b]` which was initialized to `0xC` at WM_CREATE; `OwnerDraw_Checkbox_006163A0` passes the literal `0xC`. So `0x0C` is a project-wide convention for this dead arg — likely a vestige from a historical signature (possibly TS-era) that originally consumed it. **For Rust parity, this arg can be ignored.**
- Arg_6 = 0 across all checked callers (Button, Static, Checkbox, RadioVariant); also unused.
- The `OR EDI, 0x200` byte that ends up in the high bits of the color when disabled is unused (only low 24 bits are extracted as R/G/B).

## 4. Question 4 — Hover-state visual changes

**Answer: NONE.** `OwnerDraw_Button_00612B70` does not handle `WM_MOUSEMOVE` (`0x200`). There is no hover-art-swap, no hover-color shift, no hover-alpha change, no hover-scale.

### Evidence — message-dispatch enumeration

`OwnerDraw_Button_00612B70` message handler dispatches on `param_2` with these branches (from decomp + disassembly at `0x00612d33..0x00613776`):

| Message | Hex   | Behavior |
|---|---|---|
| WM_KILLFOCUS / WM_ACTIVATE / WM_GETDLGCODE | 0x06, 0x08, 0x21 | `return 0` (consume, no redraw) |
| WM_PAINT                                    | 0x0F | Full repaint (the path investigated above) |
| WM_TIMER                                    | 0x113 | Toggle keyboard-focus flash bit `[piVar17+0xc5]`, invalidate |
| WM_LBUTTONDOWN                              | 0x201 | Play `RulesClass+0x188` (GUIMainButtonSound) if not disabled, then bail |
| WM_LBUTTONDBLCLK                            | 0x203 | Same as WM_LBUTTONDOWN |
| Custom hover-related msg                    | 0x4dc | Set/clear timer flag `[piVar17+0xc4]`; arg `param_4` is the keyboard-focus state |
| WM_KEYUP custom keyboard-nav                | 0x6dc | Default-button toggle (via `0x2d9` subtraction in the switch) |
| All others                                  | `default:` | Forward to `CallWindowProcA` (no special handling) |

**Notably absent:** `0x200 = WM_MOUSEMOVE`. The default-case forwards it to the original window proc, which is the standard Windows BUTTON class. Standard BUTTON does not redraw on mouse-move for owner-draw style — it only sends WM_DRAWITEM on state change.

### Evidence — paint code only reads pressed + disabled state

The art-family decision at `0x00612eaa..0x00612f5e` and the text-color decision at `0x00612da9 / 0x00612f5f..` consult only:

- `pWStack_d8 & 1` (= `piVar17[0x3a] & 1`, the pressed bit, set by WM_LBUTTONDOWN/UP via the OS's default button proc)
- `uStack_ac & 0x08000000` (= WS_DISABLED bit of GWL_STYLE)
- `[piVar17 + 0xC5]` (the WM_TIMER-toggled flash flag) — but only the **SDBTNANM** kind (`piVar17[5]==1`) consults this; the **normal PCX path** does NOT branch on `+0xC5`.

The PCX path does not branch on any hover state, focus state, or mouse-position field.

### Behavioral implication

For a main-menu PCX button the visible-state set is exactly:
- {up + enabled} → `bue_*30` + yellow `#FFFF00` text
- {down + enabled} → `bde_*30` + yellow `#FFFF00` text, shifted +2/+1
- {up + disabled} → `bue_*30` + dark-red `#480000` text, all alpha-overlaid 50% black
- {down + disabled} cannot occur (disabled forces 'u')

There is no fifth "hovered" state.

### Active in YR

YES — confirmed by exhaustive enumeration of the message switch.

### Tiny details

- The standard Windows BUTTON class does internally track a "captured" state on mouse-down, but for owner-draw, the **only** signal back to the application is WM_DRAWITEM (which the framework here routes to WM_PAINT-equivalent). No hover signal is generated.
- The `0x4dc` custom message is NOT a hover message. It's the engine's "should this control flash for keyboard-focus?" toggle. Its `param_4 == 1` arms a 1000ms timer; the WM_TIMER handler flips `[+0xC5]`. But (again) `[+0xC5]` is only read by the SDBTNANM dropdown path, not the normal PCX path. So even the keyboard-flash has no visible effect on a standard main-menu PCX button.
- Cursor is hidden (`MOUSE.SHA` software cursor) and would be drawn by a higher layer regardless of any button state.

## 5. Question 5 — Meaning of `'e'` in `bue_*30 / bde_*30`

**Answer:** `'e'` = "enabled". The second character is hardcoded `0x65 ('e')` in the format-string call. The disabled-state family (`bud_*` / nominally `bdd_*`) is **not loaded by any live code path in YR**; the disabled visual is produced by alpha-overlaying black at 0x80 over the enabled-up art instead.

### Evidence — only two format-string call sites

Cross-references to the three format strings (live in this Ghidra session):

| String address | Format        | Caller xrefs                                                 |
|---|---|---|
| `0x0083589C` | `b%c%c_li%d.pcx` | `0x006133c2` (OwnerDraw_Button_00612B70), `0x00616db8` (OwnerDraw_RadioVariant_00616980) |
| `0x0083588C` | `b%c%c_mi%d.pcx` | `0x0061344f` (OwnerDraw_Button_00612B70), `0x00616e55` (OwnerDraw_RadioVariant_00616980) |
| `0x0083587C` | `b%c%c_ri%d.pcx` | `0x006134d4` (OwnerDraw_Button_00612B70), `0x00616ee3` (OwnerDraw_RadioVariant_00616980) |

**No other call sites format these strings.** No other code path produces a `b?_li/mi/ri*.pcx` filename.

### Evidence — second `%c` is hardcoded 'e' at every site

Asm at `0x006133b8` (the `_li` format call inside Button):
```
006133b7: PUSH ESI                  ; height suffix
006133b8: PUSH 0x65                 ; → second %c argument = 0x65 = 'e'
006133ba: PUSH EDI                  ; first %c (= 'u' or 'd' from `uStack_f4._3_1_`)
006133bb: LEA ECX, [ESP+0xD0]
006133c2: PUSH 0x83589c             ; format string addr
006133c7: PUSH ECX
006133c8: CALL 0x007c8ef4           ; sprintf
```

Identical pattern at `0x00613447`, `0x006134cc` (the `_mi` and `_ri` calls), and at the parallel sites in `OwnerDraw_RadioVariant_00616980` (`PUSH 0x65` immediate at all six sites, decoded via the format-string xrefs above).

### Evidence — first `%c` is `'u'` / `'d'` only

In `OwnerDraw_Button_00612B70` at `0x00613240..0x0061328e`:
```
00613240: MOV CL, 0x75              ; CL = 'u' = up default
00613242: AND EDI, 0x1              ; EDI = pressed bit
00613249: MOV [ESP+0x14], EDI
0061324d: JZ 0x00613254              ; not pressed → leave CL = 'u'
0061324f: MOV byte ptr [ESP+0x13], 0x64   ; pressed → store 'd' = 0x64
00613254: TEST [ESP+0x58], 0x8000000      ; disabled?
0061325c: JZ 0x00613264
0061325e: MOV byte ptr [ESP+0x13], CL     ; disabled → force 'u'
```

So the first `%c` byte (stored in `[ESP+0x13]`) takes only two values: `0x75 = 'u'` or `0x64 = 'd'`. No other character is ever produced.

### Evidence — `bud_*` files exist but no code path uses them

`FUN_0061f210` (file-pre-registration helper called at startup) registers the following PCX names with `CDFileClass__Constructor`:

```
bue_li30, bue_mi30, bue_ri30, bde_li30, bde_mi30, bde_ri30
bud_li30, bud_mi30, bud_ri30
bue_li24, bue_mi24, bue_ri24, bde_li24, bde_mi24, bde_ri24
bud_li24, bud_mi24, bud_ri24
```

`bud_*` files are registered as known assets but **the format-string call NEVER produces `bud_*` filenames** because the second `%c` is hardcoded `'e'`. The `bud_*` family is dead code/asset — TS legacy.

(Notably, no `bdd_*` strings exist at all in the binary, so the dead "disabled" family is incomplete anyway — only `bud_*` is registered, not the `bdd_*` companions.)

### Conclusion

`'e'` = **"enabled" family**, the only live family in YR. The naming convention appears to be `b{state}{ability}_{position}{height}.pcx`:
- state: `u` = up, `d` = down (pressed)
- ability: `e` = enabled (live), `d` = disabled (TS legacy, never loaded)
- position: `l` = left cap, `m` = middle (tiled), `r` = right cap
- height: `24` or `30` (px)

In YR, the disabled visual is reproduced by drawing the `bue_*` (up + enabled) art and applying `AlphaBlendRect(rect, surface, 0, 0x80)` after the text. The TS-era practice of swapping to `bud_*` for disabled is abandoned.

### Active in YR

YES — confirmed by complete enumeration of every caller of the format string in the binary and every live byte producing the first `%c`.

### Tiny details

- The third `%d` is the height suffix (24 or 30). Selected at `0x006132ca..0x006132ea` based on the control's drawable height being `>= 30 → 30` else `>= 24 → 24` (compared against `[ESP+0x70]=0x18` and `[ESP+0x74]=0x1e` with a 2-entry table walk).
- `RadioVariant` uses literally the same format strings and the same `'e'` hardcoding at three parallel sites. So radio-button siblings on the main menu (if any are present) follow the identical PCX-name pattern.
- Even the disabled-state code path inside `OwnerDraw_Button` does NOT redirect to `bud_*`. It just forces the first char to 'u' (line: `if (disabled) uStack_f4._3_1_ = 'u'`), keeping the second char hardcoded 'e'.

## 6. Open Questions — Final State of Investigation Log

- `[RESOLVED] Q1` — disabled text RGB for main-menu Button = `{R=0x48, G=0x00, B=0x00}` = **#480000** (read from `DAT_00b0fa94..96` written by stub at `0x0072a8b0`); a 50% black alpha overlay then covers art + text. (evidence: `0x00612f5f..0x00613136` color path, `0x006135fd..0x0061361b` alpha overlay, stub asm decoded from `read_memory 0x0072a8b0`)
- `[RESOLVED] Q1a` — Static and Checkbox controls use a DIFFERENT disabled-text color: `DAT_00ac1cb4 = 0x9F` = **#9F0000** (dark red). Button does NOT use this global. (evidence: `OwnerDraw_Static` line `iVar6 = DAT_00ac1cb4`; `OwnerDraw_Checkbox` line `if (disabled) uVar5 = DAT_00ac1cb4`; `FUN_0060F9A0` init at `0x0060fa14`)
- `[RESOLVED] Q2` — pressed text rect delta: `(left+=2, top+=4, right+=0, bottom+=0)`. Effective rendered text delta after h-center + v-center: **+1 px right, +2 px down**. (evidence: `0x006135bf..0x006135cd` rect adjustment; center math in `FUN_00621040` and `FUN_00434CD0`)
- `[RESOLVED] Q3a` — args `5, 0xC, 0, 0, 0` map to `(flags=5, dead=0xC, dead=0, fade_count=0, fade_range=0)`. (evidence: FUN_00621040 disassembly at `0x00621040..0x0062114d`; no read of `[ESP+0x34]` or `[ESP+0x38]`)
- `[RESOLVED] Q3b` — flag bits: `0x01=h-center, 0x02=h-right, 0x04=v-center`. Value 5 = h-center + v-center. (evidence: `FUN_00621040` v-center test at `0x006210c8`; `FUN_00434CD0` h-center test at `0x00434d54`-ish range)
- `[RESOLVED] Q3c` — NO shadow or outline pass. Single per-glyph 1bpp blit only. (evidence: `FUN_00434120` decompiled in full; single inner loop, single color write per set bit)
- `[RESOLVED] Q4` — no WM_MOUSEMOVE handler in `OwnerDraw_Button_00612B70`. No hover state mutation. No hover-art-swap. (evidence: complete message-switch enumeration of `0x00612d33..0x00613776`)
- `[RESOLVED] Q5` — `'e'` = "enabled" family. Hardcoded `PUSH 0x65` at all 3 sites in `OwnerDraw_Button` (`0x006133b8`, `0x00613445`, `0x006134cb`) and 3 mirrors in `OwnerDraw_RadioVariant`. `bud_*` files registered at startup but never loaded — TS legacy. (evidence: xrefs to `0x0083589c/0x0083588c/0x0083587c` show only 2 live callers; asm `PUSH 0x65` at each site; `FUN_0061f210` registration list)
- `[DEFERRED] Q3d` — purpose of arg_5=0x0C in historical/TS context. (category: `out-of-scope`; reason: the value is dead in the live YR binary, so its origin is not parity-relevant; next-step-if-pursued: search the TS-era binary for an earlier `FUN_00621040` signature that consumed this arg.)
- `[DEFERRED] Q1b` — exact disabled-text color in non-shell YR scenarios (e.g. in-mission menus where `[g_ScenarioClass_Instance + 0x30d8] != 0`). (category: `out-of-scope`; reason: anchor doc + this report are scoped to the initial main-menu dialog `0xE2`; in-mission dialogs may switch to `DAT_00b0f9fc..` teal or `DAT_00b0fb14..` dark-red; verify when investigating in-game pause menu; next-step-if-pursued: trace `[+0x34b8]` writes to determine which branch fires in-mission.)
- `[DEFERRED] Q4a` — whether the standard Windows BUTTON class's internal mouse-capture state generates ANY observable effect by being forwarded through `CallWindowProcA` in the default branch. (category: `needs-runtime-debugger`; reason: standard BUTTON's default-message behavior for owner-draw buttons is to issue WM_DRAWITEM only on state-change (pressed/focused/disabled), so no hover redraw is expected; next-step-if-pursued: hook WM_DRAWITEM on a main-menu PCX button under live gamemd.exe and watch for moves without clicks.)

## 7. Sources

**Ghidra functions decompiled in this session (gamemd.exe):**

- `OwnerDraw_Button_00612B70` (`0x00612B70`) — full body + assembly trace
- `FUN_00621040` (`0x00621040`) — full body + assembly + calling-convention recovery from caller
- `OwnerDraw_Static_006153E0` (`0x006153E0`) — for disabled-text comparison
- `OwnerDraw_Checkbox_006163A0` (`0x006163A0`) — for disabled-text comparison
- `OwnerDraw_RadioVariant_00616980` (`0x00616980`) — for second-format-string-caller comparison
- `FUN_0060F9A0` (`0x0060F9A0`) — for confirmation that `DAT_00ac18a4 = 0xFFFF` and `DAT_00ac1cb4 = 0x9F`
- `FUN_0069bbe0` (`0x0069bbe0`) — confirmed reads `[ScenarioClass + 0x30d8]` (single-byte scenario-active flag)
- `FUN_00433c70 / 00433c90 / 00433ca0` — BitFont SetColor / SetEnable / SetClipRect (verified outer +0x24 / +0x41 / +0x30-3F semantics)
- `FUN_00434120` (`0x00434120`) — BitFont glyph blit (no-shadow confirmation)
- `FUN_00434CD0` (`0x00434CD0`) — BitFont DrawWithWrap (flag-bit semantics)
- `FUN_006BA3E0` (`0x006BA3E0`) — middle-tile helper (no-text confirmation)
- `FUN_0061f210` (`0x0061f210`) — PCX file-pre-registration list

**Memory reads (read-only):**

- `0x00b0f9fc..ff` — disabled-text RGB Branch A subcase 0
- `0x00b0fa94..96` — disabled-text RGB Branch B (main-menu path)
- `0x00b0fb14..1c` — disabled-text RGB Branch A subcases 1, else
- `0x00ac18a4` — default text color global (yellow `0xFFFF`)
- `0x00ac1cb4` — Static/Checkbox disabled-text color global (`0x9F`)
- `0x0072a8b0..0x0072a9af` — 24-bit color-write stubs (decoded as raw asm; stubs start at `0x0072a8c0`, 0x20-byte aligned; `0x0072a8b0..0x0072a8bf` is an unrelated 4-byte stub)
- `0x00815450..` — 64-byte function-pointer table referencing the writer stubs

**Assembly traces (read-only via `disassemble_function`):**

- `OwnerDraw_Button_00612B70` — full body (`0x00612b70..0x006137a4`)
- `FUN_00621040` — full body (`0x00621040..0x0062114d`)

**Prior reports referenced:**

- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` (anchor) — confirmed asset family, art-positioning rules, normal yellow text color
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` — full BitFont algorithm reference, used to confirm flag-bit semantics and that no shadow pass exists
- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` — mouse-down vs paint-transition sound attribution (out of scope here)

**INI files checked:** none — owner-draw button paint has no INI surface.

**Rust files:** none modified or read in this investigation. (Implementation implications are noted only for downstream brainstorm/plan work.)
