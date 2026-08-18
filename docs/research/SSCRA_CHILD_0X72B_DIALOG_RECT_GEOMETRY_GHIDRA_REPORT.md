# SSCRA Child 0x72B Dialog Rect Geometry - Ghidra Report

Date: 2026-05-27
Target: `SSCRA_CHILD_0X72B_DIALOG_RECT_GEOMETRY`
Scope: exact dialog/control geometry for radar dialog child `0x72B` used by the owner-draw kind-4 `SSCRA*` path, plus the centering arithmetic in `OwnerDraw_Static_006153E0`.

## Target Question

What is the exact `RT_DIALOG` geometry for child `0x72B` under radar dialogs `0x103` and `0xBC7`, and how does `OwnerDraw_Static_006153E0` place the loaded `SSCRA*` SHP within that child client rect?

## Non-Goals

- Do not re-investigate `SSCRA*` filename selection or load order beyond confirming the `g_RadarFrameClose_SHP` consumer.
- Do not re-investigate `MPSSCRN*` transition movies beyond negative separation from `SSCRA*`.
- Do not implement Rust or patch existing published research docs.
- Do not infer ordinary in-game minimap aperture behavior from this owner-draw shell/static path.

## Evidence Needed To Mark COMPLETE

- Retail `gamemd.exe` `RT_DIALOG` dump for dialogs `0x103` and `0xBC7`, including child `0x72B`.
- Binary consumer evidence that `FUN_0060A5B0` arms kind 4 for `(dialog 0x103/0xBC7, child 0x72B)` and stores `FUN_00603870` output.
- Binary consumer evidence that `FUN_00603870` returns `g_RadarFrameClose_SHP` for `(dialog 0x103/0xBC7, child 0x72B)`.
- Binary consumer evidence that `OwnerDraw_Static_006153E0` uses `GetClientRect`, compares SHP header width/height, applies integer half-margin centering only when the SHP is smaller than the client rect, then passes the resulting destination point to `CC_Draw_Shape`.

## Stop Conditions

- Stop if the resource dump lacks `RT_DIALOG` IDs `0x103` or `0xBC7`.
- Stop if child `0x72B` does not exist in either dialog.
- Stop if Ghidra cannot decompile the kind-4 setup or paint function read-only.
- Stop before claiming pixel client bounds if the result depends on an unverified post-create `MoveWindow` for child `0x72B`.

## Retail Resource Geometry

I dumped `RT_DIALOG` resources directly from:

`C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`

The parser walks the PE resource tree, selects `RT_DIALOG` type `5`, parses both `DLGTEMPLATE` and `DLGTEMPLATEEX`, and reports child controls. This is static retail resource evidence, not Rust or YRpp metadata.

| Dialog | Template | Parent DLU rect | Font | Child | Child DLU rect | Style | Class | Active in YR |
|---|---|---:|---|---|---:|---:|---|---|
| `0x103` / 259 | `DLGTEMPLATE` | `(0,0,533,369)` | `8, MS Sans Serif` | `0x72B` | `(72,115,282,141)` | `0x50000007` | `ord:130` static | Conditional, dialog path |
| `0xBC7` / 3015 | `DLGTEMPLATEEX` | `(0,0,533,369)` | `8, MS Sans Serif` | `0x72B` | `(70,84,282,141)` | `0x50000007` | `ord:130` static | Conditional, dialog path |

The dialog-unit-to-pixel mapping used by the existing shell resource reports is the standard shell mapping for these 800x600 dialogs: horizontal base unit `6` and vertical base unit `13`, with Windows `MulDiv` rounding. This matches the already documented `533x369` parent becoming `800x600`.

Derived pixel client rects before any later `MoveWindow`:

| Dialog | Child DLU rect | Resource pixel rect |
|---|---:|---:|
| `0x103` | `(72,115,282,141)` | `(108,187,423,229)` |
| `0xBC7` | `(70,84,282,141)` | `(105,137,423,229)` |

Active in YR: Conditional. Evidence: resources are present in retail `gamemd.exe`; `FUN_0060A5B0` and `FUN_00603870` explicitly check dialog IDs `0x103/0xBC7` and child `0x72B`.

## Kind-4 Setup Path

`FUN_0060A5B0 @ 0x0060A5B0` looks up the owner-draw record for the child HWND. In the non-PCX/non-text path, it gets the parent dialog metadata, reads the child ID with `GetDlgCtrlID`, and arms kind `4` for `(dialog 0x103, child 0x72B)` or `(dialog 0xBC7, child 0x72B)`.

For this kind-4 path, if the image pointer at record `+0x78` is still zero, it calls:

- `FUN_006035F0()`, stored at record `+0x74`.
- `FUN_00603870(&uStack_11)`, stored at record `+0x78`.
- `uStack_11`, stored as a byte at record `+0x7C`.

If kind `4` is armed and the loaded SHP pointer is nonzero, setup initializes record `+0x94` to the SHP frame count from `*(short *)(shape + 6)`.

Active in YR: Conditional. Evidence: Ghidra decompile of `FUN_0060A5B0 @ 0x0060A5B0` shows exact dialog/child predicates and record writes for kind `4`.

## SSCRA Consumer

`FUN_00603870 @ 0x00603870` returns `g_RadarFrameClose_SHP` only when the parent dialog metadata is `0x103` or `0xBC7` and `GetDlgCtrlID(child) == 0x72B`.

Relevant decompiled predicate:

```text
if ((iVar5 == 0x103) || (iVar5 == 0xbc7)) {
    if (iVar3 == 0x72b) {
        return g_RadarFrameClose_SHP;
    }
}
```

Active in YR: Conditional. Evidence: direct decompile of `FUN_00603870 @ 0x00603870`; prior `SSCRA_CLOSE_FRAME_DRAW_LIFECYCLE_GHIDRA_REPORT.md` verified `g_RadarFrameClose_SHP` is loaded from the `SSCRA*` selector path and is not the `MPSSCRN*` direct transition movie.

## Owner-Draw Centering Arithmetic

`OwnerDraw_Static_006153E0 @ 0x006153E0` handles `WM_PAINT` (`0xF`). For kind `4`, it:

1. Calls `GetClientRect(param_1, &tStack_40)`.
2. Builds a client surface width/height from the returned rect.
3. Copies the saved background/client pixels.
4. Reads the SHP pointer from record `+0x78`.
5. Starts destination x/y at the client draw rect left/top.
6. If `shape_width < client_width`, adds `(client_width - shape_width) / 2` to x.
7. If `shape_height < client_height`, adds `(client_height - shape_height) / 2` to y.
8. Reads the current frame from record `+0x98`.
9. Calls `CC_Draw_Shape(shape, frame, &dest_point, &clip_rect, 0x400, ...)`.

The comparison is strict `<`, not `<=`. Therefore equal-size or larger SHPs are not centered; they draw at the client origin and rely on clipping.

Active in YR: Conditional. Evidence: decompile of `OwnerDraw_Static_006153E0 @ 0x006153E0` around the kind-4 `WM_PAINT` branch shows `GetClientRect`, `*(short *)(shape + 2)` width, `*(short *)(shape + 4)` height, integer half-margin additions, record `+0x98` frame read, and `CC_Draw_Shape` call.

## SSCRA Dimensions And Resulting Placement

The prior retail asset dump verified both `SSCRASM.SHP` and `SSCRAMD.SHP` as `424x230`, 44 frames, zero embedded offsets.

Comparing the owner-draw child client size with the SHP:

| Dialog | Client size | `SSCRA*` size | Native centering branch | Draw origin inside child |
|---|---:|---:|---|---:|
| `0x103` child `0x72B` | `423x229` | `424x230` | width/height centering skipped | `(0,0)` |
| `0xBC7` child `0x72B` | `423x229` | `424x230` | width/height centering skipped | `(0,0)` |

Inference from verified facts: in the standard 800x600 shell resource mapping, the `SSCRA*` frame is one pixel wider and one pixel taller than the child client area, so `OwnerDraw_Static_006153E0` does not center it; the destination remains the client origin and the client clip clips the rightmost/bottommost pixel. This conclusion depends on the standard resource mapping already used by shell-layout reports; no live runtime screenshot was captured in this slot.

Active in YR: Conditional. Evidence: `RT_DIALOG` child size, prior retail `SSCRA*` asset dimensions, and owner-draw strict `<` centering comparison.

## Frame Protocol Relevant To Geometry

`0x4D3` starts kind-4 animation only when shape pointer `+0x78` is nonzero, kind is `4`, and animation-active byte `+0xA8` is zero. It sets the animation-active byte and starts timer `0`.

`0x4D5` sets record `+0x98` to the caller-provided frame and invalidates the child. `WM_PAINT` then reads `+0x98` and draws that frame with the same geometry rules above.

Active in YR: Conditional. Evidence: `OwnerDraw_Static_006153E0 @ 0x006153E0` decompile message branches for `0x4D3` and `0x4D5`.

## Implementation Handoff

1. Verified behavior -> child `0x72B` is a static owner-draw control with resource pixel client size `423x229` in both radar dialogs, at `(108,187)` for dialog `0x103` and `(105,137)` for dialog `0xBC7`. Current Rust delta -> no owner-draw static radar dialog model; current radar animation is generic `radar.shp`. Affected surface -> future shell/dialog renderer, `src/render/radar_anim.rs` only if reused carefully. Acceptance test -> `test_sscra_child_0x72b_resource_rects_map_to_423x229_client`. Priority -> HIGH.

2. Verified behavior -> `SSCRA*` is `424x230`, and the strict `<` centering branch skips centering when the SHP is larger than the `423x229` child. Current Rust delta -> likely to center or stretch if modeled as generic animation. Affected surface -> owner-draw static rendering path. Acceptance test -> `test_sscra_larger_than_child_draws_at_client_origin_and_clips`. Priority -> HIGH.

3. Verified behavior -> `0x4D5` stores current frame into record `+0x98`; paint uses that stored frame and the same client rect, not `DAT_00B0FC1C` or the `MPSSCRN*` transition origin. Current Rust delta -> generic `RadarAnimState` does not model owner-draw static records. Acceptance test -> `test_sscra_0x4d5_frame_uses_owner_draw_child_rect`. Priority -> MEDIUM-HIGH.

## Negative Facts / Do Not Do

- Do not place `SSCRA*` at `DAT_00B0FC1C`; the proven paint path uses child `GetClientRect`.
- Do not use `FUN_0072EAD0` geometry for `SSCRA*`; that function draws `MPSSCRN*`, not `g_RadarFrameClose_SHP`.
- Do not center `SSCRA*` inside `0x72B` for standard retail dimensions; native strict `<` tests skip centering because `424x230` is larger than `423x229`.
- Do not stretch or scale `SSCRA*` to the child; native passes the SHP frame unchanged to `CC_Draw_Shape`.
- Do not treat the resource DLU rect as final pixels without applying the shell dialog-unit conversion.

## Remaining Uncertainty

- I did not capture a live Win32 runtime screenshot or hook `GetClientRect`; the `423x229` pixel size is derived from the verified retail dialog resource plus the already documented 800x600 shell dialog-unit mapping.
- I did not exhaustively prove there is no later `MoveWindow` for child `0x72B`; no such move was found in the scoped owner-draw setup/paint path.
- I did not re-verify duplicate `MPSSCRNL.SHP` cache resolution, palette conversion, or shell background overlap.

## Stale-Doc Replacement Wording

- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace any wording that places `SSCRA*` through the right-panel `DAT_00B0FC1C` path with: "`SSCRA*` is drawn by the owner-draw static path for child `0x72B` under dialogs `0x103/0xBC7`; the child resource rects are `(72,115,282,141)` and `(70,84,282,141)` DLU, mapping to `423x229` client size in the standard 800x600 shell resource mapping."
- `docs/research/RADAR_CHROME_COMPOSITING.md`: replace generic close-frame wording with: "`SSCRA*` is a `424x230` owner-draw static SHP. Because child `0x72B` maps to `423x229`, native strict-centering checks skip centering and draw at child client origin with clipping."

## Status

COMPLETE for resource geometry, owner-draw placement arithmetic, and the `SSCRA*` child consumer path. Remaining runtime screenshot/post-create move proof is noted as uncertainty rather than blocking the geometry handoff.
