# Skirmish Flag Static Rendering - Ghidra Research Report

**Address(es):** `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x004E3A00`, `0x004E3B90`, `0x004E3F70`, `0x004E3690`, `0x004E3560`, `0x0060F9A0`, `0x0061F210`, `0x00603D30`, `0x006153E0`, `0x006BA140`, `0x006BA580`  
**Investigation Mode:** exhaustive-slice, downgraded to partial for the lower PCX decoder internals only  
**Claimed Scope:** offline standard YR Skirmish dialog `0x102` flag static controls `0x6DA..0x6E1`: control geometry, country item-data to PCX image selection, cache/preload path, static owner-draw placement/clipping, null/blank behavior, and side/country update linkage.  
**Non-Scope:** combo dropdown geometry/population beyond the side item-data link; color/start/team combo behavior; online lobby variants; full PCX file decoder internals below the cached surface API.  
**Confidence:** High for active YR control/update/render path; Medium for palette conversion internals because this pass verified converted-surface use and paint-time key conversion, not the lower PCX decoder body.  
**Active in YR:** Yes. Evidence: dialog proc `FUN_006AE3F0` dispatches init `0x497`, `WM_PAINT`, and `WM_COMMAND`; `FUN_006AE6E0` initializes dialog `0x102`; `FUN_006ACEE0` routes side combo changes.

## 1. Overview

Skirmish row flags are transparent `STATIC` controls whose image pointer is set from the selected side/country combo item data. The image selector maps item data directly to cached PCX filenames, then the owner-draw static callback restores its saved background and blits the PCX at native size with optional centering and clipping.

Active in YR: Yes. Evidence: `FUN_004E3F70` and `FUN_004E3690` both end in `FUN_004E3560` -> `FUN_00603D30`; `OwnerDraw_Static_006153E0` handles the paint path.

## 2. Controls and Geometry

| Slot | Side combo | Flag static | Dialog resource rect | Active in YR | Evidence |
|---:|---:|---:|---|---|---|
| 0 | `0x6A1` | `0x6DA` | `(150,36,32,12)` DLU | Yes | resource `0x102`; `FUN_004E3F70` / `FUN_004E3690` |
| 1 | `0x510` | `0x6DB` | `(150,52,32,12)` DLU | Yes | same |
| 2 | `0x513` | `0x6DC` | `(150,68,32,12)` DLU | Yes | same |
| 3 | `0x51E` | `0x6DD` | `(150,84,32,12)` DLU | Yes | same |
| 4 | `0x514` | `0x6DE` | `(150,100,32,12)` DLU | Yes | same |
| 5 | `0x51F` | `0x6DF` | `(150,116,32,12)` DLU | Yes | same |
| 6 | `0x520` | `0x6E0` | `(150,132,32,12)` DLU | Yes | same |
| 7 | `0x521` | `0x6E1` | `(150,148,32,12)` DLU | Yes | same |

The resource style for the flag cells is `STATIC`, style `0x50000005`, extended style `0x20` (`WS_EX_TRANSPARENT`). Active in YR: Yes; evidence is the embedded `RT_DIALOG` resource `0x102` as extracted in `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md:112..128`.

## 3. Image Source and File Naming

`FUN_004E3560` is the filename selector. It does not inspect country names, side names, or INI strings; it switches on combo item data and calls the owner-draw PCX cache lookup `FUN_006BA140(name, 0)`.

| Item data | PCX | Active in YR | Evidence |
|---:|---|---|---|
| `-3` | `obsi.pcx` | Conditional | observer/restricted branch in `FUN_004E3B90`; string `0x00836334`; `FUN_004E3560` |
| `-2` | `rani.pcx` | Yes | normal first row inserted by `FUN_004E3A00`; string `0x008363AC`; `FUN_004E3560` |
| `0` | `usai.pcx` | Yes | `[Countries] 0=Americans`, `rulesmd.ini:960`; string `0x008363A0`; `FUN_004E3560` |
| `1` | `japi.pcx` | Yes | `[Countries] 1=Alliance`, `rulesmd.ini:961`; string `0x00836394`; `FUN_004E3560` |
| `2` | `frai.pcx` | Yes | `[Countries] 2=French`, `rulesmd.ini:962`; string `0x00836388`; `FUN_004E3560` |
| `3` | `geri.pcx` | Yes | `[Countries] 3=Germans`, `rulesmd.ini:963`; string `0x0083637C`; `FUN_004E3560` |
| `4` | `gbri.pcx` | Yes | `[Countries] 4=British`, `rulesmd.ini:964`; string `0x00836370`; `FUN_004E3560` |
| `5` | `djbi.pcx` | Yes | `[Countries] 5=Africans`, `rulesmd.ini:966`; string `0x00836364`; `FUN_004E3560` |
| `6` | `arbi.pcx` | Yes | `[Countries] 6=Arabs`, `rulesmd.ini:967`; string `0x00836358`; `FUN_004E3560` |
| `7` | `lati.pcx` | Yes | `[Countries] 7=Confederation`, `rulesmd.ini:968`; string `0x0083634C`; `FUN_004E3560` |
| `8` | `rusi.pcx` | Yes | `[Countries] 8=Russians`, `rulesmd.ini:969`; string `0x00836340`; `FUN_004E3560` |
| `9` | `yrii.pcx` | Yes | `[Countries] 9=YuriCountry`, `rulesmd.ini:971`; string `0x00836328`; `FUN_004E3560` |

Asset source cross-check: prior archive probe resolved `usai.pcx`, `rusi.pcx`, and `obsi.pcx` from `ra2.mix -> local.mix`, and `yrii.pcx` from `ra2md.mix -> localmd.mix`; checked flag PCXs are `47x23` 8-bit PCX images with embedded VGA palette markers. Active in YR: Yes for the filename set; evidence `FUN_0061F210` preloads all twelve flag PCXs with mode `2,0`, and `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md:130..137`.

## 4. Update Chain

1. Dialog `0x102` owner-draw setup is active. `FUN_0060F9A0` class-compares `"Static"`, assigns `OwnerDraw_Static_006153E0`, subclasses the control through wrapper `0x00610CA0`, and sends custom init message `0x497`. Active in YR: Yes; `FUN_006AE3F0` calls the shell setup path before handling Skirmish messages.
2. Normal side combo population inserts item data, not PCX names. `FUN_004E3A00` clears the combo, sets max visible rows with `0x4DE=7`, inserts Random item data `-2`, then inserts multiplayer house types whose `HouseType+0x1A5` is true and whose `HouseType+0xB8` item data is `-2..9`. Active in YR: Yes; called from `FUN_004E3B90`.
3. Observer/restricted population is separate. `FUN_004E3B90` can replace a combo with a single item data `-3` and grey state `0x4F1=1` when the current house matches `DAT_00AC11B4`. Active in YR: Conditional; this is not the normal unrestricted offline player/AI country list.
4. Programmatic initialization uses `FUN_004E3F70`. It selects the requested side combo item via `CB_SETCURSEL` (`0x14E`), then reads the current selection (`0x147`) and item data (`0x150`) before updating the paired static. Active in YR: Yes; reached from `FUN_006AE6E0`.
5. User side/country changes use `FUN_004E3830` -> `FUN_004E3690`. The command handler `FUN_006ACEE0` recognizes side combo IDs and reaches the same `FUN_004E3560` -> `FUN_00603D30` final setter. Active in YR: Yes; dialog `0x102` `WM_COMMAND` path.
6. `FUN_00603D30` writes owner-draw static state and invalidates. It finds the state by `HWND`, writes kind `2`, writes the image/surface pointer, and calls `InvalidateRect(hwnd, NULL, TRUE)`. Active in YR: Yes; both init and user-change chains call it.

## 5. Render Behavior

`OwnerDraw_Static_006153E0` handles `WM_PAINT` (`0x0F`). For kind `2`, it:

- restores the cached parent/background surface before drawing the image. Active in YR: Yes; paint path allocates/copies a `BSurface` from `GetClientRect`.
- reads source width/height through image vtable slots `+0x7C` and `+0x80`. Active in YR: Yes; kind `2` branch.
- does not scale. If source width is smaller than the static rect width, X becomes `left + (rect_w - src_w) / 2`; if source height is smaller, Y becomes `top + (rect_h - src_h) / 2`. If source is wider or taller, the destination rect size stays the static rect, so the blit clips/crops. Active in YR: Yes; kind `2` branch.
- calls `FUN_006BA580` with a transparent key computed from RGB magenta `0xFF00FF` using DirectDraw channel loss/shift globals. Active in YR: Yes; paint-time key expression and `FUN_006BA580`.
- validates the rect after the draw attempt. Active in YR: Yes; kind `2` branch.

For the checked retail flag PCXs (`47x23`) in a `32x12` DLU placeholder, the observable rule is native-size blit into the final pixel control rect, with clipping when the final pixel rect is smaller than the image. Active in YR: Yes for the rule; exact final pixel rect depends on dialog hosting/DLU conversion outside this slot.

## 6. Blank and Disabled Behavior

Blank is caused by a null image pointer, not by a separate disabled flag. If `FUN_006BA140` misses in the owner-draw cache or `FUN_004E3560` gets an unmapped item data value, `FUN_00603D30` can store `0`; `OwnerDraw_Static_006153E0` then restores the background and skips the image branch.

Active in YR: Yes. Evidence: `FUN_006BA140` returns `0` on lookup miss; `OwnerDraw_Static_006153E0` checks the image pointer before width/height reads and blit.

Disabled row behavior is indirect. If a row is inactive, Skirmish init can disable the adjacent side/color/start/team controls after setting the side flag to Random (`-2`) through `FUN_004E3F70(0xFFFFFFFE)`. The static itself still uses the same kind-2 image path; this slot found no separate disabled-flag static renderer. Active in YR: Yes for the init behavior, with evidence in `FUN_006AE6E0`; exact inactive-row presentation beyond the static image is out of scope.

## 7. Palette and Conversion

The flag path uses cached converted surfaces. `FUN_0061F210` preloads the flag PCXs through `CDFileClass__Constructor(name, 2, 0)`, and `FUN_006BA140` returns a cached surface pointer. `OwnerDraw_Static_006153E0` and `FUN_006BA580` operate on 16-bit surfaces and convert only the magenta transparent key at paint time.

Active in YR: Yes. Evidence: `FUN_0060F9A0` invokes `FUN_0061F210` once when `DAT_00AC48D4 == 0`; `FUN_004E3560` uses `FUN_006BA140`; `FUN_006BA580` copies 16-bit pixels except those equal to the passed key.

Bounded partial: this retry did not fully recover the lower PCX decoder's exact embedded-palette-to-16-bit algorithm. The safe verified claim is that this render path consumes cached converted PCX surfaces, not that a named external `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL` is loaded by the selector/static/preload functions.

## 8. Current Rust Status

Rust already has the item-data to PCX filename table in `src/app_skirmish_shell_render.rs:350..363` and preload entries in `src/render/skirmish_shell_chrome.rs:165..176`. The visible mismatch is placement: current flag drawing calls `push_entry_fit` from `src/app_skirmish_shell_render.rs:607` and `:617`, which scales to fit; gamemd's static path uses native-size draw with centering only when smaller and clipping when larger.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Dialog `0x102` active path | verified | `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0` | none for flag statics |
| Static owner-draw hook | verified | `FUN_0060F9A0` -> `OwnerDraw_Static_006153E0` | none |
| Side combo item-data source | verified | `FUN_004E3A00`, `FUN_004E3B90`, `rulesmd.ini:959..971` | combo dropdown visuals out of scope |
| Programmatic flag update | verified | `FUN_006AE6E0` -> `FUN_004E3F70` -> `FUN_004E3560` -> `FUN_00603D30` | none |
| User side-change flag update | verified | `FUN_006ACEE0` -> `FUN_004E3830` -> `FUN_004E3690` -> `FUN_004E3560` -> `FUN_00603D30` | none |
| PCX name mapping | verified | `FUN_004E3560`; strings `0x00836328..0x008363AC` | none |
| PCX preload/cache lookup | verified | `FUN_0061F210`, `FUN_006BA140` | lower decoder internals not exhausted |
| Native-size placement/clipping | verified | `OwnerDraw_Static_006153E0` kind `2` branch | final high-res hosting rect belongs to slot 1 |
| Magenta transparent blit | verified | `OwnerDraw_Static_006153E0`, `FUN_006BA580` | none at render layer |
| Null/blank behavior | verified | `FUN_006BA140`, `FUN_00603D30`, `OwnerDraw_Static_006153E0` | none |
| Lower PCX decoder palette conversion | touched-not-exhausted | `CDFileClass__Constructor` touched; prior asset doc confirms embedded VGA palette marker | needs dedicated decoder investigation if byte-exact conversion is required |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-FSR-1` - Are controls `0x6DA..0x6E1` active in standard YR Skirmish? Yes. Evidence: resource `0x102`; `FUN_004E3F70`, `FUN_004E3690`, `OwnerDraw_Static_006153E0`.
- `[RESOLVED] OQ-FSR-2` - Are image updates tied to side/country changes? Yes. Programmatic and user paths both read combo item data and end in `FUN_00603D30`. Evidence: `FUN_004E3F70`, `FUN_004E3690`, `FUN_006ACEE0`.
- `[RESOLVED] OQ-FSR-3` - What image names are used? `obsi.pcx`, `rani.pcx`, `usai.pcx`, `japi.pcx`, `frai.pcx`, `geri.pcx`, `gbri.pcx`, `djbi.pcx`, `arbi.pcx`, `lati.pcx`, `rusi.pcx`, `yrii.pcx`. Evidence: `FUN_004E3560`, strings `0x00836328..0x008363AC`.
- `[RESOLVED] OQ-FSR-4` - Does the static scale the flag into the rect? No. It centers only when source is smaller and clips otherwise. Evidence: `OwnerDraw_Static_006153E0`.
- `[RESOLVED] OQ-FSR-5` - What blanks the flag? Null cached image pointer or unmapped item data. Evidence: `FUN_006BA140` return `0`; `OwnerDraw_Static_006153E0` null check.
- `[DEFERRED] OQ-FSR-6` - Exact lower PCX decoder palette math. Category: bounded-cost-too-high. Reason: out below this slot's owner-draw render path; current pass verified cached converted surface use and paint-time magenta key conversion.

## Sources

- Ghidra read-only decompile: `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_004E3A00`, `FUN_004E3B90`, `FUN_004E3830`, `FUN_004E3F70`, `FUN_004E3690`, `FUN_004E3560`, `FUN_0060F9A0`, `FUN_0061F210`, `FUN_00603D30`, `OwnerDraw_Static_006153E0`, `FUN_006BA140`, `FUN_006BA580`, `CDFileClass__Constructor` (touched only).
- Dialog/resource evidence: `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md:112..128`.
- Asset evidence: `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md:130..137`, `:182..191`.
- Prior focused report: `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`.
- INI cross-check: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:959..971`, `:3225..3332`.
- Rust comparison only: `src/app_skirmish_shell_render.rs:350..363`, `:607`, `:617`; `src/render/skirmish_shell_chrome.rs:165..176`.
