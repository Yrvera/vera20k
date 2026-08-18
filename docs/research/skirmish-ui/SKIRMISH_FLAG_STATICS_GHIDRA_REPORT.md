# Skirmish Flag Statics -- Ghidra Research Report

**Address(es):** `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x004E3B90`, `0x004E3A00`, `0x004E3F70`, `0x004E3690`, `0x004E3560`, `0x00603D30`, `0x006153E0`, `0x0061F210`, `0x006BA140`, `0x006BA580`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline standard YR Skirmish dialog `0x102` flag statics `0x6DA..0x6E1`, side/country combo item data as the source of flag selection, owner-draw static paint, PCX asset names, palette/key behavior at the owner-draw layer, blank conditions, and update messages.  
**Non-Scope:** combo dropdown visuals beyond item data; color/start/team controls; map preview; online/WOL shell; whole-shell PCX use outside the flag path.  
**Confidence:** High for control IDs, update chain, PCX filename mapping, owner-draw static placement/blank behavior, and callback-layer palette/key behavior. Medium for the lower PCX decoder's embedded-palette internals because this pass rechecked preload/cache/callback functions, not the unnamed decoder body below `CDFileClass__Constructor`.  
**Active in YR:** Yes. Evidence: active Skirmish dialog proc `FUN_006AE3F0` handles custom init `0x497`, `WM_PAINT`, and `WM_COMMAND`; `FUN_006AE6E0` calls the side/flag setup helpers during standard Skirmish init; `FUN_006ACEE0` routes side combo command IDs in dialog `0x102`.

## 1. Overview

The offline Skirmish flag statics are not independent country controls. They are owner-draw `Static` controls whose current image pointer is updated from the selected item data of the matching side/country combo. The observable flag is a cached PCX surface keyed by exact item-data values, centered only when smaller than the static rect, clipped when larger, and transparent-blitted with a magenta key converted to the active 16-bit display format.

## 2. Control and State Mapping

| Slot | Side combo ID | Flag static ID | Active in YR | Evidence |
|---:|---:|---:|---|---|
| 0 | `0x6A1` | `0x6DA` | Yes | `FUN_004E3F70`, `FUN_004E3690`, `FUN_004E37D0` map slot/control IDs |
| 1 | `0x510` | `0x6DB` | Yes | same |
| 2 | `0x513` | `0x6DC` | Yes | same |
| 3 | `0x51E` | `0x6DD` | Yes | same |
| 4 | `0x514` | `0x6DE` | Yes | same |
| 5 | `0x51F` | `0x6DF` | Yes | same |
| 6 | `0x520` | `0x6E0` | Yes | same |
| 7 | `0x521` | `0x6E1` | Yes | same |

Key owner-draw static state offsets are expressed relative to the per-control state body used by `OwnerDraw_Static_006153E0`:

| Offset / slot | Purpose | Active in YR | Evidence |
|---:|---|---|---|
| `+0x70` / `piVar11[0x1C]` | static kind; `2` means PCX image | Yes | written by `FUN_00603D30`, read by `OwnerDraw_Static_006153E0` |
| `+0x18` from state block / `piVar2[6]` in setter | image/surface pointer for kind `2` | Yes | `FUN_00603D30`; later read as `piVar11[5]` in paint |
| `+0xEC` / `piVar11[0x3B]` | text color for text-kind statics; initialized to `DAT_00AC18A4` | Yes, but not flag image-visible | `OwnerDraw_Static_006153E0`, message `0x497` |
| cached backing surface pointer `piVar11[4]` | parent/background copy used before drawing image | Yes | allocated/restored in `OwnerDraw_Static_006153E0` paint path |

## 3. Setup and Update Chain

1. **Dialog entry is standard YR Skirmish. Active in YR: Yes.** `FUN_006AE3F0` calls shell base handling first, runs `FUN_006AE6E0` on custom `0x497`, handles `WM_PAINT`, and routes `WM_COMMAND` to `FUN_006ACEE0`.
2. **Owner-draw hook assigns `Static` controls to `OwnerDraw_Static_006153E0`. Active in YR: Yes.** `FUN_0060F9A0` class-compares `"Static"`, stores callback `OwnerDraw_Static_006153E0`, installs wrapper wndproc `0x00610CA0`, and sends custom init message `0x497`.
3. **Static init message `0x497` sets static defaults. Active in YR: Yes.** `OwnerDraw_Static_006153E0` sets kind `0`, vertical spacing `0x0C`, and default text color `DAT_00AC18A4`.
4. **Side combo population inserts item data, not PCX names. Active in YR: Yes.** `FUN_004E3B90` iterates eight side combos. Normal path `FUN_004E3A00` clears the combo (`0x14B`), sets max rows with `0x4DE=7`, adds a first row with item data `-2`, then adds multiplayer house types whose `HouseType+0x1A5` is nonzero, whose side item data at `+0xB8` is in `-2..9`, and whose UI name pointer is nonempty.
5. **Observer/restricted path is separate. Active in YR: Conditional.** In `FUN_004E3B90`, when the current house matches `DAT_00AC11B4`, the combo is replaced by a single row with item data `-3` and grey state `0x4F1=1`; otherwise the normal `-2` plus house-country list is used.
6. **Programmatic setup writes flags through `FUN_004E3F70`. Active in YR: Yes.** `FUN_006AE6E0` calls `FUN_004E3F70(hwnd, side_combo_id, selected_item_data)` for existing player records. `FUN_004E3F70` selects the matching combo item with `CB_SETCURSEL` `0x14E`, then reads selected item data with `0x147`/`0x150`, resolves a PCX surface, and calls `FUN_00603D30`.
7. **User side changes use a different wrapper but the same final setter. Active in YR: Yes.** `FUN_006ACEE0` handles side combo IDs `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E`, `0x51F`, `0x520`, `0x521`; it calls `FUN_004E3830(control_id)` to get the slot and then `FUN_004E3690(hwnd, slot)`, which reads current selection/item data and ends with `FUN_004E3560` -> `FUN_00603D30`.
8. **Setter message side effect is invalidation, not immediate paint. Active in YR: Yes.** `FUN_00603D30` finds the owner-draw state by `HWND`, writes kind `2`, writes the PCX pointer, then calls `InvalidateRect(hwnd, NULL, TRUE)`.

## 4. Exact PCX Asset Mapping

`FUN_004E3560` is the only PCX-name selector in this flag path. It calls `FUN_006BA140(name, 0)` and returns the cached surface pointer from that lookup. Active in YR: Yes, because both init and user side-change chains call it before setting flag static image state.

| Side item data | PCX asset | Active in YR | Evidence |
|---:|---|---|---|
| `-3` | `obsi.pcx` | Conditional | observer/restricted row in `FUN_004E3B90`; string at `0x00836334`; branch in `FUN_004E3560` |
| `-2` | `rani.pcx` | Yes | normal first side-combo row in `FUN_004E3A00`; string at `0x008363AC`; branch in `FUN_004E3560` |
| `0` | `usai.pcx` | Yes | string at `0x008363A0`; branch in `FUN_004E3560` |
| `1` | `japi.pcx` | Yes | string at `0x00836394`; branch in `FUN_004E3560` |
| `2` | `frai.pcx` | Yes | string at `0x00836388`; branch in `FUN_004E3560` |
| `3` | `geri.pcx` | Yes | string at `0x0083637C`; branch in `FUN_004E3560` |
| `4` | `gbri.pcx` | Yes | string at `0x00836370`; branch in `FUN_004E3560` |
| `5` | `djbi.pcx` | Yes | string at `0x00836364`; branch in `FUN_004E3560` |
| `6` | `arbi.pcx` | Yes | string at `0x00836358`; branch in `FUN_004E3560` |
| `7` | `lati.pcx` | Yes | string at `0x0083634C`; branch in `FUN_004E3560` |
| `8` | `rusi.pcx` | Yes | string at `0x00836340`; branch in `FUN_004E3560` |
| `9` | `yrii.pcx` | Yes | string at `0x00836328`; branch in `FUN_004E3560` |

YR INI cross-check: `ini/rulesmd.ini` lists `[Countries]` `0=Americans`, `1=Alliance`, `2=French`, `3=Germans`, `4=British`, `5=Africans`, `6=Arabs`, `7=Confederation`, `8=Russians`, `9=YuriCountry`; those sections have `Multiplay=yes` and sides `GDI`, `Nod`, or `ThirdSide` at `rulesmd.ini:3225..3332`. Active in YR: Yes, because `FUN_004E3A00` filters the already parsed HouseType array by multiplayer flag and item data range.

## 5. Owner-Draw Static Paint Behavior

1. **Paint message is `WM_PAINT` `0x0F`. Active in YR: Yes.** `OwnerDraw_Static_006153E0` handles it for hooked statics.
2. **Backing copy occurs before image draw. Active in YR: Yes.** On first paint, the static allocates a `BSurface` sized from `GetClientRect`, copies from display/parent backing into its cached surface, and restores that cached background before kind-specific drawing.
3. **Kind `2` is the flag PCX path. Active in YR: Yes.** Paint checks `piVar11[0x1C] == 2`; if the image pointer is non-null it reads source width/height through vtable slots `+0x7C` and `+0x80`.
4. **No scaling is performed. Active in YR: Yes.** If source width is smaller than the static rect width, X is centered by `(rect_w - src_w) / 2`; if source height is smaller, Y is centered. If the source is wider or taller, the destination size remains the static rect and the blit clips/crops.
5. **Transparent blit uses magenta as key after 16-bit conversion. Active in YR: Yes.** The call to `FUN_006BA580` passes the current display surface, the cached PCX surface, and a key computed from RGB `0xFF00FF` through DirectDraw channel loss/shift globals.
6. **Blank condition is explicit. Active in YR: Yes.** If kind `2` has a null PCX pointer, the image branch is skipped and the callback still validates the rect. `FUN_006BA140` returns `0` on cache miss, and `FUN_00603D30` can store that null pointer.

## 6. Palette / Cache Behavior

`FUN_0061F210` preloads the owner-draw PCX pool once from `FUN_0060F9A0` when `DAT_00AC48D4 == 0`. The flag PCXs are preloaded with mode `2,0`, and `dlgsysa.pcx` is the special mode-`1` wrapper case, not a flag image. Active in YR: Yes, because the hook setup runs for Skirmish shell controls before the flag statics are painted.

Verified in this pass:

- **Cached surface lookup, not per-paint file open. Active in YR: Yes.** `FUN_004E3560` calls `FUN_006BA140`; `FUN_006BA140` searches the owner-draw cache and returns a converted surface pointer or `0`.
- **Flag paint consumes converted surfaces. Active in YR: Yes.** `OwnerDraw_Static_006153E0` and `FUN_006BA580` operate on 16-bit display/`BSurface` style surfaces and convert only the transparent key at paint time.
- **No external palette is loaded by the flag selector/static callback/preload list. Active in YR: Yes for this layer.** `FUN_0061F210`, `FUN_004E3560`, `FUN_006BA140`, `FUN_00603D30`, `OwnerDraw_Static_006153E0`, and `FUN_006BA580` do not reference `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL`.

Bounded inference, not overclaimed: prior report `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md` says the lower PCX loader converts embedded PCX palettes to 16-bit surfaces. This pass did not recover a named function boundary for the loader below `CDFileClass__Constructor`, so the safe claim here is that the flag path receives cached 16-bit PCX surfaces and applies a magenta transparency key during blit.

## 7. Current Rust Implementation Status

The repo already contains the literal item-data-to-PCX table in `src/app_skirmish_shell_render.rs:350..363` and preloads the PCX list in `src/render/skirmish_shell_chrome.rs:165..176`. The current renderer uses `push_entry_fit` for flags (`src/app_skirmish_shell_render.rs:154`, calls at `:604..617`), which scales to fit; binary behavior is native-size center-if-smaller and crop-if-larger. Country/side combo interaction remains out of this report's implementation scope, but the trace doc at `docs/research/traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md` records it as absent.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Active Skirmish dialog path | verified | `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0` | none for this slice |
| Static owner-draw hook | verified | `FUN_0060F9A0` class `"Static"` -> `OwnerDraw_Static_006153E0`, sends `0x497` | none |
| Side combo population source data | verified | `FUN_004E3B90`, `FUN_004E3A00`, `ini/rulesmd.ini:959..987`, `:3225..3332` | exact localized display strings not needed for flags |
| Programmatic/init flag updates | verified | `FUN_006AE6E0` -> `FUN_004E3F70` -> `FUN_004E3560` -> `FUN_00603D30` | none |
| User side-change flag updates | verified | `FUN_006ACEE0` -> `FUN_004E3830` -> `FUN_004E3690` -> `FUN_004E3560` -> `FUN_00603D30` | none |
| Item data to PCX names | verified | `FUN_004E3560`; strings `0x00836328..0x008363AC`; `FUN_0061F210` preload | none |
| Static kind-2 paint placement | verified | `OwnerDraw_Static_006153E0` kind `2` branch | none |
| Transparent-key behavior | verified | `OwnerDraw_Static_006153E0` call to `FUN_006BA580`; `FUN_006BA580` skips pixels equal to key | none |
| Missing PCX / null pointer blank | verified | `FUN_006BA140` returns `0`; `OwnerDraw_Static_006153E0` skips image if pointer is `0` | none |
| Lower PCX decoder embedded-palette internals | touched-not-exhausted | `FUN_0061F210`, `FUN_006BA140`; prior follow-up report | direct decoder function boundary remains unnamed in this pass |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1` - Are `0x6DA..0x6E1` active standard YR Skirmish controls? Yes; they are updated by the active Skirmish init/user side-change helpers and painted by the static owner-draw callback. Evidence: `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_004E3F70`, `FUN_004E3690`, `OwnerDraw_Static_006153E0`.
- `[RESOLVED] OQ-2` - Does flag rendering use country names, side names, or combo item data? Combo item data. Evidence: `FUN_004E3A00` stores item data with `0x151`; `FUN_004E3F70`/`FUN_004E3690` read with `0x147`/`0x150`; `FUN_004E3560` maps item data to PCX.
- `[RESOLVED] OQ-3` - What are the exact PCX names? `obsi`, `rani`, `usai`, `japi`, `frai`, `geri`, `gbri`, `djbi`, `arbi`, `lati`, `rusi`, `yrii` with `.pcx` suffixes. Evidence: `FUN_004E3560`, string addresses `0x00836328..0x008363AC`.
- `[RESOLVED] OQ-4` - What makes the static blank? A null image pointer from cache miss or unmapped item data. Evidence: `FUN_006BA140` return `0`, `FUN_00603D30` stores pointer, `OwnerDraw_Static_006153E0` skips image branch when null.
- `[RESOLVED] OQ-5` - Is the flag scaled? No. It is centered only if smaller and otherwise cropped/clipped. Evidence: `OwnerDraw_Static_006153E0` kind `2` branch.
- `[RESOLVED] OQ-6` - Is observer always available? Conditional. Normal population uses `-2` plus multiplayer houses; observer `-3` is used by the restricted current-house branch in `FUN_004E3B90`. Evidence: `FUN_004E3B90`, `FUN_004E3A00`.
- `[DEFERRED] OQ-7` - Exact lower PCX decoder palette algorithm. Category: bounded-cost-too-high. Reason: current Ghidra function boundary is not exposed as a named function; prior follow-up reports it, while this report only needs the flag path's observable use of cached converted surfaces and magenta-keyed paint.

## Sources

- Ghidra: `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_0060F9A0`, `FUN_004E3B90`, `FUN_004E3A00`, `FUN_004E3F70`, `FUN_004E3690`, `FUN_004E3830`, `FUN_004E37D0`, `FUN_004E3560`, `FUN_00603D30`, `OwnerDraw_Static_006153E0`, `FUN_0061F210`, `FUN_006BA140`, `FUN_006BA580`.
- Ghidra strings: flag PCX strings at `0x00836328..0x008363AC`.
- INI: `ini/rulesmd.ini` `[Countries]`, `[Sides]`, and country `Multiplay=yes` / `Side=` sections.
- Cross-check docs: `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md`.
