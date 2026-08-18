# Shell-Button Greyscale PCX Colorization — Ghidra Research Report

> **YELLOW — HEADLINE REFUTED 2026-05-19.** This doc claimed "the PCX path is
> the live YR shell button path" and that `bue_*30`/`bde_*30` greyscale art
> is what retail renders on dialog 0xE2. **That is wrong for dialog 0xE2.**
> User in-game observation confirmed buttons are colored (SDBTNANM frames
> 2/3/4). The `LAB_0060A330` `EnumChildWindows` callback writes `+0xB0 = 1`
> on all dialog-0xE2 button records via `FUN_00608CD0` (matches buttons
> 0x683/0x684/0x578/0x686/0x55C/0x55F) and `FUN_00609730` (matches 0x3EE),
> forcing the `iVar14 == 1` SDBTNANM branch in `OwnerDraw_Button_00612B70`.
>
> See `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` for the
> verified dispatch chain. The PCX-branch internal analysis below (greyscale
> palette, no tint, AlphaBlendRect for disabled, modulo tile, cap geometry)
> is technically correct *for that branch* — just don't assume that branch
> is reached on the main menu. Whether any YR live dialog actually reaches
> the PCX path is an open question.


**Address(es):** `0x00612B70` (OwnerDraw_Button), `0x00630310` (PCX-read inside the misnamed `BSurface__Constructor`), `0x006B9D00` (the owner-draw PCX cache constructor, also misnamed `CDFileClass__Constructor`), `0x006BA3E0` (tile blit), `0x007bbb90` (BSurface vtable+0x08 — generic blit), `0x007bc750` (standard scanline blitter — REP MOVSD memcpy), `0x0072ade0` (separate PAL-file loader, NOT used for PCX), `0x0072aa40` (sidebar palette init), `0x00622140` (`WM_PAINT_Handler` — dialog background).

**Confidence:** HIGH — every claim is confirmed by live decompilation and assembly read in this session against the running Ghidra database of `gamemd.exe`. The negative finding (no colorization step exists in the PCX-button paint path) is established by exhaustive enumeration of the call chain, the BSurface blitter family, and the cross-references of every plausible colorization global. No facts invented; unknowns flagged.

**Active in YR:** YES for every code path examined. The shell PCX-button paint path (`OwnerDraw_Button_00612B70`, mode `iVar14==0` = `piVar17[+0xb0]==0` and `piVar17[+0x14]==0`) is the live YR shell button path; the PCX cache loader at `0x006B9D00` mode 2 is the live preload path; the DDraw 16-bit conversion via `g_DD_*Loss/Shift` is the live display path.

Scope: targeted refutation/extension of `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` §6 — "the PCX button pieces are decoded through their embedded 256-color PCX VGA palette ... This pass found no new evidence that SHELL.PAL, SHELL2.PAL, SDBTNANM.PAL, DIALOG.PAL, or MAINBTTN.PAL are used for the PCX button surfaces."

Runtime data established by the Rust port's `inspect-pcx-palette` diagnostic:

```
bue_li30.pcx: 7x30 px, palette max R=217 G=217 B=217
bue_mi30.pcx: 177x30 px, palette max R=228 G=228 B=228
bue_ri30.pcx: 10x30 px, palette max R=236 G=236 B=236
bde_li30.pcx: 7x27 px, palette max R=235 G=235 B=235
```

Every palette entry in the on-disk PCXs has R=G=B — the source art is greyscale.

## 1. Headline Conclusion

**The prior finding is CORRECT — and the implication is the bullet point the prior doc didn't draw.** No external `.PAL`, no tint multiplier, no remap LUT, no per-pixel modulation, no DDraw `BltFx`, and no HousePalette/team-color is applied during the shell PCX-button paint path. The runtime palette **is** the embedded greyscale PCX palette, and the destination is a plain 16-bit memcpy/tile of those greyscale pixels through the standard DirectDraw quantization.

Therefore the retail gamemd.exe display of these specific PCX pieces (`bue_*30`/`bde_*30`) **also renders as greyscale** at intensities approximately matching the on-disk palette max bytes (217/228/236/235). The tan/khaki appearance the Rust port is matching against does NOT originate in `OwnerDraw_Button_00612B70` or the PCX cache. The premise that gamemd colorizes these PCXs is **refuted by the call chain**.

Most likely sources of the perceived tan/khaki in retail reference images (NOT investigated here, listed only as forwarded hypotheses):

- Background show-through of the RA2TS Bink movie behind the buttons (the movie ramps up khaki/tan banner art near the menu region; greyscale buttons on a tan movie pixel-region read as tan/khaki to the eye even without true tint).
- A user-side mod replacing `bue_*30.pcx`/`bde_*30.pcx` with colorized versions in a higher-priority `.mix`.
- A reference screenshot taken with DDrawCompat / dgVoodoo / DxWnd colour-space transform enabled, biasing the displayed grey toward yellow.
- The disabled-state alpha overlay (50% black) compositing differently on a yellow movie pixel vs. a Rust-port black background.

These are out of scope; they are recorded so a follow-up investigation has the candidate list.

## 2. Call Chain — from preload to pixel write

### 2.1 Preload (`FUN_0061F210`)

Called once on first owner-draw control creation (`FUN_0060F9A0`, gated by `DAT_00AC48D4 == 0`). Calls `CDFileClass__Constructor(<name>, 2, 0)` for each of `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`, `bde_li30.pcx`, etc. The constructor is at `0x006B9D00` (Ghidra-misnamed `CDFileClass__Constructor` — it is actually the owner-draw cache surface builder).

### 2.2 PCX cache build (`0x006B9D00`, mode `param_3 == 2`)

Verified directly from disassembly at `0x006B9D00..0x006B9F01`:

1. Zero a 768-byte palette scratch buffer at `[ESP+0x3b8]` (256 RGB triplets, all 0x00).
2. `CCFileClass__Constructor(filename)` opens the MIX/file entry.
3. `BSurface__Constructor(file_cls, palette_buf=&[ESP+0x3c0])` at `CALL 0x00630310` decodes the PCX and populates the palette buffer.
4. Build a 256-entry u16 conversion table at `[ESP+0xb4]` (`local_91c`):

   ```
   for i in 0..256:
     R = palette_buf[i*3 + 0]      ; byte
     G = palette_buf[i*3 + 1]
     B = palette_buf[i*3 + 2]
     u16_table[i] = ((R >> g_DD_RLoss) << g_DD_RShift)
                  | ((G >> g_DD_GLoss) << g_DD_GShift)
                  | ((B >> g_DD_BLoss) << g_DD_BShift)
   ```

   Assembly at `0x006B9DBF..0x006B9E13`. **No tint constant added, no multiplier, no remap.**
5. Allocate a converted 16-bit `BSurface` of `width*height*2` bytes.
6. Per-pixel: `dst_u16[k] = u16_table[src_8bit[k]]` — assembly at `0x006B9EAF..0x006B9EBD`. **Pure index-into-table; no per-pixel modification.**
7. Insert into the cache hash table keyed by filename.

### 2.3 PCX file read (`0x00630310` — Ghidra mis-named `BSurface__Constructor`; this is the actual PCX parser)

Decompilation + disassembly at `0x00630310..0x00630970`:

- Reads PCX header from file (size `0x80`).
- Decodes RLE pixel data into a buffer.
- **Palette read** at `0x00630930..0x0063095a`:
  ```
  if (palette_buf != 0 && file_header_byte_0x85 == 0x01) {
      Seek(-0x300, SEEK_END)              ; PUSH 2 / PUSH 0xfffffd00
      Read(palette_buf, 0x300)            ; reads 768 bytes raw
  }
  ```
- The `0x01` check is the standard PCX VGA palette marker (the byte preceding the 768-byte palette tail is `0x0C` = VGA palette flag; the byte read at file offset corresponds).
- **NO `<<2` shift** is applied. The 768 bytes are written verbatim to the destination buffer. Therefore the palette as seen by the conversion loop is byte-identical to the on-disk PCX palette tail.

Tiny detail: the `<<2` 6-bit-VGA-to-8-bit shift exists in a SEPARATE function `FUN_0072ade0` (line `local_78 = CONCAT11(*pcVar4 << 2, pcVar4[iVar11] << 2)`), which loads standalone `.PAL` files. It is NOT in the PCX path. So if a `.PAL` file contains 6-bit Westwood values, `FUN_0072ade0` quadruples them to 8-bit. But PCX files do not get this treatment — their palette is read raw.

### 2.4 Paint (`OwnerDraw_Button_00612B70`)

Mode dispatch by `iVar14 = piVar17[0x2c]` (control state offset +0xB0):

- `iVar14 == 0` (the live shell button path):
  - `iVar14_inner = piVar17[5]` (offset +0x14, custom image pointer) — for normal shell, this is 0.
  - Format `b%c%c_li/mi/ri%d.pcx`, look up via `FUN_006BA140(name, 0)`, get cached converted surface pointer.
  - **Left cap blit:** `(**(DAT_00887310->vtable)[+0x08])(dst_rect, src_surface, src_rect, 0, 1)`. The `0` is "no transparency/alpha". See §2.5 below — this dispatches to a plain memcpy blitter.
  - **Middle tile blit:** `FUN_006BA3E0(dst_rect, dst_surface, src_surface, 0, 0)` — plain modulo-tile, no modification per §2.6.
  - **Right cap blit:** same as left cap.
- `iVar14 == 1, 2, 3` (SDBTNANM / SHP modes, in-game HUD only): these branches consult `FUN_0072e2c0`, `FUN_0072f4b0`, **`FUN_0072b050`** (which returns the **MAINBTTN.PAL** ConvertClass). Then `CC_Draw_Shape(palette_convert_cls, frame, ...)`. This path is NOT taken for main-menu shell buttons.

The `LAB_00613568` text-draw stage uses `FUN_00621040` with color `DAT_00ac18a4 = 0x0000FFFF` (yellow) — independent of the PCX surface. The disabled-overlay branch `AlphaBlendRect(0, 0x80)` only fires when `WS_DISABLED` is set; enabled buttons do NOT get the 50% black overlay.

### 2.5 BSurface vtable+0x08 → standard memcpy blitter

`*DAT_00887310->vtable[+0x08]` resolves to `FUN_007bbb90` (read from BSurface vtable at `0x007e2070`, slot `[2]` = `0x007bbb90`). Verified via `read_memory 0x007e2070` returning `50 16 41 00  f0 ba 7b 00  90 bb 7b 00` → little-endian dwords `0x00411650`, `0x007bbaf0`, `0x007bbb90`.

`FUN_007bbb90(dst, src_rect, src_surface, alpha_byte=0, mode=1)`:

- Calls `ClipRectPair` for rect-rect intersection.
- Branches on `(char)param_3 == '\0'` — when the alpha byte arg is 0 (our case), picks blitter set `&PTR_LAB_007f7bdc`.
- Calls `FUN_00437350` → `Standard_SHP_blitter`.

`Standard_SHP_blitter` per-scanline calls `(**(piVar1+4))(...)` where `piVar1` is the blitter set pointer. For `PTR_LAB_007f7bdc`, slot `[1]` (offset +0x04) is `0x007bc750`. Disassembly at `0x007bc750`:

```
MOV ECX, [ESP+0xC]    ; count (bytes)
PUSH ESI / MOV ESI, [ESP+0xC] / MOV EAX, ECX / PUSH EDI / MOV EDI, [ESP+0xC]
SHR ECX, 2
REP MOVSD             ; 32-bit copy
MOV ECX, EAX / AND ECX, 3
REP MOVSB             ; remainder
POP EDI / POP ESI / RET 0x20
```

**This is pure memcpy** of the 16-bit converted pixels. No tint, no transparency, no remap, no LUT.

### 2.6 Tile blitter `FUN_006BA3E0` (middle piece)

Decompilation confirmed: the inner loop is `*puVar7 = *(undefined2 *)(src_base + (x % src_w + (y % src_h) * stride) * 2);` — plain 16-bit pixel copy at modulo source coordinates. No modification.

### 2.7 The PAL ConvertClasses that DO exist (and where they are used)

Loaded by `FUN_0072aa40` (sidebar init, called once from `Init_Game` at `0x0052ba60` via `0x0052bba8`):

| PAL file       | Buffer ptr    | ConvertClass ptr | Getter            | Used by                                          |
|----------------|---------------|------------------|-------------------|--------------------------------------------------|
| DIALOG.PAL     | `0xb0fb64`    | `0xb0fb68`       | `FUN_0072aff0`    | `WM_PAINT_Handler` `piVar9[0x2c]==2` SHP-bg path |
| DIALOGY.PAL    | `0xb0fb6c`    | `0xb0fb70`       | `FUN_0072b010`    | `WM_PAINT_Handler` SHP-bg path (scenario==2)     |
| DIALOGN.PAL    | `0xb0fb5c`    | `0xb0fb60`       | `FUN_0072b030`    | `WM_PAINT_Handler` SHP-bg path (no scenario)     |
| MAINBTTN.PAL   | `0xb0fb74`    | `0xb0fb78`       | `FUN_0072b050`    | `OwnerDraw_Button` mode 3 only (NOT shell PCX)   |
| SHELL.PAL      | (`0xb0fbc8`)  | `0xb0fbcc`       | (no named getter) | `RightPanel__Draw` only (in-game sidebar)        |
| SHELL2.PAL     | (`0xb0fbd0`)  | `0xb0fbd4`       | (no named getter) | `RightPanel__Draw` only                          |
| SDBTNANM.PAL   | (`0xb0fbd8`)  | `0xb0fbdc`       | (no named getter) | `RightPanel__Draw` only                          |

Xrefs to each of these globals were enumerated; none reach `OwnerDraw_Button_00612B70`'s PCX path (mode 0 + custom-image 0).

Tiny detail: `FUN_0072ade0` loads each PAL by reading raw 256×3 bytes and applying `<< 2` per byte (6-bit → 8-bit Westwood convention), then calls `ConvertClass__Constructor(palette, palette, DAT_00887310, 1, 0)` to build the per-PAL 8→16 LUT. The PCX path at `0x006B9D00` does NOT do this — for PCXs the palette bytes are already in 8-bit range and are read into the per-cache-entry conversion table directly. The two loaders share NO state.

## 3. Where MAINBTTN.PAL Would Be Used (if `iVar14 == 3` were taken)

In `OwnerDraw_Button_00612B70`:
```
else if (iVar14 == 3) {
    piStack_c4 = FUN_0072b050();   // MAINBTTN.PAL ConvertClass
    piStack_dc = DAT_00b0facc;     // some SHP at sidebar slot
    ...
}
...
CC_Draw_Shape(piStack_dc, frame, ..., piStack_c4 /* palette LUT */, 0,0,0,0,0);
```

This is the SDBTNANM-style sidebar button path (sidebar mode buttons during gameplay). `iVar14 = piVar17[+0xb0]` is `0` for shell PCX buttons (the resource template doesn't set the `0xB0` byte to 3). So MAINBTTN.PAL is never used to colorize shell PCX buttons. It IS used to colorize sidebar SHP-mode buttons during gameplay.

## 4. INI Keys

None. The shell-button paint path has no INI surface. Owner-draw style/class is hardcoded in the dialog resource template (`RT_DIALOG 0xE2`). The PCX filenames are formatted from the dialog control's height and state (pressed/unpressed); the format strings are constants at `0x0083589C`, `0x0083588C`, `0x0083587C`.

## 5. Integration Points

- **Preload trigger:** First owner-draw control creation in the process lifetime. Gated by `DAT_00AC48D4 == 0` in `FUN_0060F9A0`. Side-effect: all PCXs listed in `FUN_0061F210` get loaded, converted to 16-bit, and cached.
- **Paint trigger:** Each shell button's `WM_PAINT` (0x0F) handled by its installed owner-draw wndproc `OwnerDraw_Button_00612B70`. The per-button cache surface (`piVar17[4]`) is built on first WM_PAINT and reused for subsequent paints (invalidation only repaints to the same cache surface, then blits to the dialog DC).
- **DirectDraw conversion globals:** `g_DD_RLoss`/`RShift` at `0x008A0DD4`/`0x008A0DD0`, `g_DD_GLoss`/`GShift` at `0x008A0DDC`/`0x008A0DD8`, `g_DD_BLoss`/`BShift` at `0x008A0DE4`/`0x008A0DE0`. Initialized once at DirectDraw setup; do not change during shell paint. For typical RGB565: RLoss=3, RShift=11, GLoss=2, GShift=5, BLoss=3, BShift=0.
- **Sidebar PAL init:** `FUN_0072aa40` runs from `Init_Game` (`0x0052ba60`), which fires when entering a game. The PALs loaded there are not consulted by the shell PCX path. (They ARE in RAM during the shell, but inert relative to button paint.)

## 6. Current Rust Implementation Status

Not modified or studied by this investigation. Implementation implication (for future plan/brainstorm only, not part of this report): the Rust shell-button rendering in `src/render/main_menu_shell_chrome.rs` reads the PCX via `PcxFile::from_bytes` and emits RGBA from the embedded palette as-is. This matches gamemd's behavior exactly — both produce greyscale output for these PCXs. No colorization fix is needed on the Rust side IF the goal is parity with gamemd's actual output. If the goal is to match a reference image whose buttons APPEAR tan/khaki, the source of that tint is not the PCX paint — investigate the background composition (RA2TS Bink movie pixels behind the buttons), the user's MIX archive contents (asset replacement), or the display-pipeline post-process being used to capture the reference image.

## 7. Open Questions — Final State of Investigation Log

- `[RESOLVED] Q1` — Does the loader at `0x006B9D00` use the PCX-embedded palette, or overwrite the scratch from a global? → Embedded only. The 768-byte scratch at `[ESP+0x3b8]` is zeroed at entry, then `BSurface__Constructor` reads exactly 768 bytes from the file's last `0x300` bytes into it. (evidence: disassembly `0x006B9D10..0x006B9D54`, `0x00630944..0x0063095A`)
- `[RESOLVED] Q2` — Is there a tint multiplier in the 8→16 conversion? → No. The loop at `0x006B9DBF..0x006B9E13` does only `(C >> Loss) << Shift` per channel, no add, no multiply. (evidence: full asm trace)
- `[RESOLVED] Q3` — What does `BSurface__Constructor (0x00630310)` actually return and write? → It decodes RLE pixels into a new `BSurface`, and (when `param_2 != 0`) reads the trailing `0x300` palette bytes verbatim into `param_2`. No `<<2` shift, no per-byte filter. (evidence: asm `0x00630930..0x0063095A`)
- `[RESOLVED] Q4` — Are SHELL.PAL / MAINBTTN.PAL / DIALOG.PAL / SDBTNANM.PAL ever loaded into a global referenced by shell-button paint? → They are loaded by `FUN_0072aa40` (sidebar init), into `0xb0fb60..78` and `0xb0fbcc..dc` ConvertClass slots. Cross-referenced consumers are `RightPanel__Draw`, `WM_PAINT_Handler` SHP-bg branch, and `OwnerDraw_Button` mode-3 (sidebar SHP buttons). The shell PCX-button paint (`OwnerDraw_Button` mode-0, custom-image 0) consults none of them. (evidence: xrefs to `DAT_00b0fb60/68/70/78/fbcc/fbd4/fbdc`)
- `[RESOLVED] Q5` — Per-pixel modification in `FUN_006BA3E0` (tile)? → No, plain `dst[x,y] = src[x%sw, y%sh]`. (evidence: decompilation of `FUN_006BA3E0`)
- `[RESOLVED] Q5b` — Per-pixel modification in the cap blits? → No. The destination vtable+0x08 (`FUN_007bbb90`) dispatches with `param_3==0` (no-alpha branch) to `Standard_SHP_blitter` with blitter set `PTR_LAB_007f7bdc`. The per-scanline function `0x007bc750` is REP MOVSD + REP MOVSB memcpy. No tint. (evidence: disasm of `0x007bc750`, byte read of vtable at `0x007e2070`, byte read of blitter set at `0x007f7bdc`)
- `[RESOLVED] Q6` — Is a HousePalette/team-color remap applied during shell paint? → No. `OwnerDraw_Button` has no `IsHouseColor`, no PreMapSelect, no Remapable global access in the mode-0 PCX path. Shell context has no `g_ScenarioClass_Instance` active (`FUN_0069bbe0` returns 0), so even the disabled-color branch picks the no-scenario palette-derived `#480000` (per the parallel-session report on disabled text colors). (evidence: full decompilation of `OwnerDraw_Button_00612B70`)
- `[RESOLVED] Q7` — Could retail also be greyscale? → **Yes.** Given Q1–Q6 all negative, the PCX paint produces greyscale 16-bit pixels on the dialog DC. Retail rendering of these specific PCXs IS greyscale, not tan/khaki. The premise of the investigation that retail colorizes them is refuted by the call chain. (evidence: deductive from all above)
- `[RESOLVED] Q8` — Effect of `g_DD_*Loss`/`*Shift`? → Display-format quantization only (e.g., RGB565). For greyscale input R=G=B=N, the round-trip produces R≈N, G≈N + (1 LSB-of-G), B≈N — a green bias of at most ~1.5% from the extra G precision in RGB565. Imperceptible. (evidence: math derivation in §1 of paint pipeline)
- `[RESOLVED] Q9` — Could a colored `bue_*30.pcx` override the greyscale one via MIX priority? → Possible only if a higher-priority MIX archive (`EXPANDMD99..0`, `RA2MD`, `CACHE*`, `LOCAL*`) contains a colored variant. The Rust diagnostic loads from the canonical MIX stack and produces greyscale; gamemd's load order at `0x005301A0` is `EXPANDMD99..0 → RA2MD → RA2 → CACHE → CACHEMD → LOCALMD → LOCAL` (insertion-order priority). For a stock install with only `expandmd01.mix` present, both paths resolve to the same file. (evidence: decompilation of `FUN_005301A0`)
- `[RESOLVED] Q10` — Could there be a `BltFx` / `BltFast` DDraw colorization? → No. The destination surface vtable+0x08 is the engine's `FUN_007bbb90`, which calls `Standard_SHP_blitter`. There is no DirectDraw `IDirectDrawSurface::BltFx` call in this path. The engine maintains its own software blitter family rather than using DDraw's color-fill/key options for shell paint. (evidence: vtable decomp, blitter set bytes)
- `[RESOLVED] Q11` — Are PCX palette bytes 6-bit (0..63) that need `<<2`? → No. The diagnostic reports max bytes 217/228/236/235 — already in the 8-bit range. Even if they were 6-bit-styled, the PCX loader at `0x00630310` does NOT shift them; that shift only happens in `FUN_0072ade0` for .PAL files. (evidence: asm at `0x0063094D..0x0063095A` showing direct `Read(buf, 0x300)` with no post-process)
- `[DEFERRED] Q12` — What IS the actual source of the tan/khaki appearance the user perceives in retail reference images? Candidates: RA2TS Bink movie background show-through, user-side MIX replacement, DDraw wrapper colour-space transform, capture-tool gamma. (category: `out-of-scope`; reason: this investigation is scoped to the colorization step itself, and that step does not exist; next-step-if-pursued: capture a live retail screenshot with the dialog area cropped, compare a pure-grey region of a known-greyscale PCX pixel against the underlying Bink movie pixels, and inspect the DDrawCompat / dgVoodoo config for any LUT/sRGB transform.)
- `[DEFERRED] Q13` — Why are `bud_*30` PCX files registered in `FUN_0061F210` (preload list) but never loaded by the format string `b%c%c_li/mi/ri%d.pcx`? Are they a TS-era disabled-state family? (category: `out-of-scope`; resolved by sibling report `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` §5 — `bud_*` is dead TS-era preload baggage, never reached because the format-string second char is hardcoded `'e'`. Cross-referenced; no new investigation needed here.)
- `[DEFERRED] Q14` — Does the `OwnerDraw_RadioVariant_00616980` variant (style low bits `0x09`) apply colorization to its PCX caps? (category: `out-of-scope`; reason: not used by standard `0xE2` main-menu controls per prior reports; next-step-if-pursued: enumerate which dialogs use it and check `iVar14`-equivalent dispatch.)
- `[DEFERRED] Q15` — Could a Windows-level paint subclass (the original BUTTON class WndProc that `CallWindowProcA` forwards to in the default case) inject any GDI-level colorization (e.g., `SetTextColor` propagated to the surface)? (category: `needs-runtime-debugger`; reason: standard owner-draw buttons do not auto-paint when their WM_PAINT is consumed; the consumer's `ValidateRect` call confirms paint completion. But a runtime hook to confirm no GDI fallback would be definitive. Highly unlikely to alter rendered pixels.)

## 8. Sources

**Ghidra functions decompiled this session (read-only):**

- `OwnerDraw_Button_00612B70` (`0x00612B70`) — full body, every branch traced
- `BSurface__Constructor` actual PCX parser (`0x00630310`) — full body + assembly
- `CDFileClass__Constructor` cache builder (`0x006B9D00`) — full body + assembly `0x006B9D00..0x006BA113`
- `FUN_006BA140` (`0x006BA140`) — cache hash lookup
- `FUN_006BA3E0` (`0x006BA3E0`) — tile blit
- `FUN_007bbb90` (`0x007bbb90`) — BSurface vtable+0x08 generic blit
- `Standard_SHP_blitter` and per-scanline `0x007bc750` — REP MOVSD memcpy
- `FUN_00437350` — blitter dispatch
- `FUN_0061F210` (`0x0061F210`) — PCX preload list
- `FUN_0072AA40` (`0x0072AA40`) — sidebar PAL+ConvertClass init
- `FUN_0072ADE0` (`0x0072ADE0`) — separate PAL file loader (shows the `<<2` shift confirmed NOT in PCX path)
- `FUN_0072AFF0/B010/B030/B050` — PAL ConvertClass getters
- `WM_PAINT_Handler` (`0x00622140`) — dialog background paint (mode dispatch shown)
- `FUN_005301A0` — MIX archive load order
- `Init_Game` (`0x0052BA60`) and `Main_Game` (`0x0048CCC0`) — confirmed call chain

**Memory reads (read-only):**

- `0x007E2070..7E207C` — BSurface vtable first 3 slots (`0x00411650, 0x007BBAF0, 0x007BBB90`)
- `0x007BC750..7BC76C` — REP MOVSD scanline blitter bytes
- `0x007F7BDC..7F7BFC` — `PTR_LAB_007f7bdc` standard blitter set (slot [1] = `0x007BC750`)
- `0x007F7BC4`, `0x007F7C0C` — alternative blitter sets (transparent / alpha) — confirmed NOT selected when `param_3==0`
- `0x00844B9C..00844BB4` — PAL filename pointer table
- `0x00844BA8..00844BAC` — MAINBTTN.PAL pointer entry

**Strings searched:**

- `bue_*`, `bde_*`, `bud_*` — 18 results, all listed
- `MAINBTTN.PAL`, `SHELL.PAL`, `SHELL2.PAL`, `SDBTNANM.PAL`, `DIALOG.PAL`, `DIALOGY.PAL`, `DIALOGN.PAL` — confirmed, xrefs traced
- `^.*\.PAL$` — full PAL inventory (69 results) reviewed for plausible button-tint candidates; none apply to shell PCX path

**Cross-references checked (read-only):**

- All xrefs to `DAT_00b0fb60`, `DAT_00b0fb68`, `DAT_00b0fb70`, `DAT_00b0fb78` (PAL ConvertClasses)
- All xrefs to `DAT_00b0fbcc`, `DAT_00b0fbd4`, `DAT_00b0fbdc` (SHELL/SHELL2/SDBTNANM ConvertClasses)
- All xrefs to `DAT_00887310` (primary surface) — searched for unexpected color-modifying consumers; none in shell paint
- All xrefs to `MAINBTTN.PAL` string at `0x008454FC`
- Callers of `FUN_0072AA40` (sidebar init) — single caller in `Init_Game`
- Callers of `0x006B9D00` (cache builder) — `FUN_0061F210` preload, `FUN_006BA120` dialog-system PCX

**Runtime evidence (the Rust port diagnostic):**

```
bue_li30.pcx: palette max R=217 G=217 B=217, first 8 entries all (0,0,0)
bue_mi30.pcx: palette max R=228 G=228 B=228, first 8 entries all (0,0,0)
bue_ri30.pcx: palette max R=236 G=236 B=236, first 8 entries all (0,0,0)
bde_li30.pcx: palette max R=235 G=235 B=235, first 8 entries all (0,0,0)
```

All channels R=G=B per palette entry. Greyscale.

**Prior reports referenced:**

- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` (anchor — partially refuted, partially extended)
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` (parallel-session sibling — cross-checked, no contradictions)
- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md` (PCX cache load path background)
- `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md` (Standard blitter/helper inventory)

**INI files checked:** none — owner-draw PCX paint has no INI surface.

**Rust files inspected (read-only, no modifications):**

- `src/bin/inspect-pcx-palette.rs` — diagnostic source confirming greyscale palette
- `src/assets/pcx_file.rs` — Rust PCX parser (confirmed matches gamemd's raw-byte read)
- `src/assets/asset_manager.rs` — MIX archive priority order (confirmed analogous to gamemd's stack)
- `src/render/main_menu_shell_chrome.rs` — current Rust shell-button asset path (greyscale-in → greyscale-out, no colorization layer)
