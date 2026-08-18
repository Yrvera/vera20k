# Ordinary Build Cameo Palette Path - Ghidra Research Report

**Slot:** /re-swarm soviet-sidebar-followup slot 3
**Target question:** Which ConvertClass/palette does stock YR use when drawing ordinary player build-palette cameo art: `CAMEO.PAL` / `DAT_0087f6b4`, `DAT_0087f6cc`, another ConvertClass, or source-palette logic?
**Status:** COMPLETE
**Confidence:** High for ordinary player branch image and overlay ConvertClass selection; medium for string-name mapping only where this report reuses prior Ghidra string-address evidence.

## Non-goals

- Observer/sidebar stats rows beyond using them as a negative contrast.
- Full cameo geometry, text, production state, flash cadence, or `CompareItems` ordering.
- Retail MIX membership or asset resolver order.
- Rust implementation changes.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile of `StripClass::Draw @ 0x006A9540`.
- Assembly context at the ordinary player build-cameo image `CC_Draw_Shape` call proving the ConvertClass register/global.
- Assembly context for ordinary player overlay/progress `CC_Draw_Shape` calls proving whether they use a different ConvertClass.
- Assembly context around `0x0052BA60` palette construction proving which global is built from the `CAMEO.PAL` load.
- Active-in-YR caller/xref evidence for the draw path.

## Stop Conditions

- Stop if Ghidra MCP read-only tools are unavailable.
- Stop after normal player build-cameo art palette is proven; do not expand into observer-side asset matrix.
- Write only this report plus the shared `.swarm-claims.md` row.

## Verified Findings

### 1. Ordinary player build-cameo art uses `DAT_0087f6b0` as the `CC_Draw_Shape` ConvertClass.

Active in YR: Yes. `SidebarClass::Draw` calls `StripClass::Draw @ 0x006A9540` at `0x006A6FDF`; `StripClass::Draw` is the active visible-strip renderer.

Evidence: `StripClass::Draw @ 0x006A9540` decompile enters the normal player branch when `g_PlayerPtr != DAT_00ac1198`, resolves each visible cameo, obtains the cameo SHP pointer into the local represented as `iStack_444`, and calls `CC_Draw_Shape(iStack_444, 0, ...)`. The exact assembly for that call is `0x006A99F3..0x006A9A3E`: it loads the SHP pointer from `[ESP+0x48]`, pushes frame `0`, then executes `MOV EDX,dword ptr [0x0087f6b0]` at `0x006A9A2A` immediately before `CALL 0x004AED70`.

### 2. `DAT_0087f6b0` is the ConvertClass built from the `CAMEO.PAL` load, not the later `MOUSEPAL.PAL` load.

Active in YR: Yes. This is inside the broad active game initialization function reached before gameplay/sidebar rendering.

Evidence: `0x0052BA60` decompile shows the global ConvertClass construction cluster. Assembly `0x0052C089..0x0052C150` loads the file whose prior Ghidra string mapping is `0x008204E0 = "CAMEO.PAL"`, converts 256 palette entries into the stack buffer, then allocation/constructor assembly `0x0052C0FA..0x0052C129` stores the resulting ConvertClass to `DAT_0087f6b0`. Assembly `0x0052C13D..0x0052C150` loads `0x00826084` only after `DAT_0087f6b0` has been stored, so that later file load is for the next ConvertClass, not `DAT_0087f6b0`.

### 3. `DAT_0087f6b4` is not the ordinary player build-cameo art ConvertClass in the traced draw call.

Active in YR: Yes as a negative finding for this scoped path.

Evidence: The handoff-critical normal cameo image call at `0x006A99F3..0x006A9A3E` loads `EDX` from `0x0087f6b0`, not `0x0087f6b4`. Fresh assembly around the ordinary image draw contains no `DAT_0087f6b4` read; the traced draw call's ConvertClass source is explicit.

### 4. `DAT_0087f6cc` is used by ordinary player cameo overlay SHPs, not by the base cameo art draw.

Active in YR: Yes.

Evidence: In the same normal player branch, the unaffordable/darken overlay draw loads `DAT_0087f6cc` at `0x006A9B2B` and calls `CC_Draw_Shape` at `0x006A9B46` with `DAT_00B07BC0`. The flash/darken overlay loads `DAT_0087f6cc` at `0x006A9B9D` and calls `0x004AED70` at `0x006A9BC0`. The production progress overlay loads `DAT_0087f6cc` at `0x006A9E7B` and calls `0x004AED70` at `0x006A9E97` with `DAT_00B0B484`. These are overlay/progress SHPs, separate from the base cameo art call at `0x006A9A3E`.

### 5. `DAT_0087f6d0` remains observer-branch evidence, not ordinary player build-cameo evidence.

Active in YR: Conditional, observer branch only in this trace.

Evidence: `StripClass::Draw` decompile branches to observer mode when `g_PlayerPtr == DAT_00ac1198`. Assembly in that observer branch loads `DAT_0087f6d0` at `0x006AA144` and `0x006AA2BA` before observer row/icon `CC_Draw_Shape` calls. The ordinary player branch base cameo art call instead loads `DAT_0087f6b0` at `0x006A9A2A`.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust surface | Required implementation effect | Acceptance test |
|---|---|---|---|---|
| Ordinary player build-cameo art uses `DAT_0087f6b0`, the `CAMEO.PAL` ConvertClass. | `0x006A9A2A..0x006A9A3E`; `0x0052C089..0x0052C129` | `src/app_init_helpers.rs::load_sidebar_cameo_palette`, `src/render/sidebar_cameo_atlas.rs` | Keep base build-cameo art decoded with `cameo.pal`; do not switch normal cameo images to `sidebar.pal`, `observer.pal`, or `dialog*.pal`. | `test_sidebar_build_cameo_art_uses_cameo_pal_dat_0087f6b0` |
| Ordinary player cameo overlays/progress use `DAT_0087f6cc` (`SIDEBAR.PAL` from the prior slot), not the base cameo ConvertClass. | `0x006A9B2B`, `0x006A9B9D`, `0x006A9E7B` before overlay/progress draw calls | `src/render/sidebar_chrome.rs`, `src/app_sidebar_build.rs` | Keep overlay/progress SHPs in the sidebar chrome/overlay palette path, separate from base cameo art. | `test_sidebar_cameo_overlay_shps_use_sidebar_pal_not_cameo_pal` |
| `DAT_0087f6d0` observer draws are not normal player cameo proof. | `0x006AA144`, `0x006AA2BA` observer branch vs `0x006A9A2A` normal branch | future observer sidebar rendering | Implement observer row/icon rendering separately with observer palette evidence; do not reuse it for player build palette art. | `test_observer_sidebar_icons_do_not_change_player_cameo_palette` |

## Negative Facts / Do Not Do

- Do not say ordinary player build-cameo base art uses `DAT_0087f6cc`; only overlay/progress SHPs in this branch use `DAT_0087f6cc`.
- Do not use `DAT_0087f6d0` / `OBSERVER.PAL` for normal player build-cameo art.
- Do not cite `DAT_0087f6b4` as the ordinary player build-cameo draw ConvertClass; the traced call uses `DAT_0087f6b0`.
- Do not describe `DAT_0087f6b0` as `MOUSEPAL.PAL`; the `MOUSEPAL.PAL` address is loaded after `DAT_0087f6b0` construction.
- Do not collapse base cameo art and sidebar chrome/overlay palette choices; stock YR uses separate ConvertClass globals in the same `StripClass::Draw` branch.

## Remaining Uncertainty

- Exact source string bytes were not freshly read with a Ghidra string tool in this slot; this report uses prior Ghidra string-address evidence that `0x008204E0 = CAMEO.PAL` and `0x00826084 = MOUSEPAL.PAL`, while freshly proving the construction order and draw consumers.
- This slot did not trace every non-player/observer sidebar draw or every possible custom/modded cameo source.
- This slot did not verify retail archive membership or MIX lookup precedence for cameo SHPs.

## Stale-doc Wording

- `docs/research/SIDEBAR_CAMEO_CHROME_CONVERTCLASS_SETUP_0052BA60_GHIDRA_REPORT.md` currently maps `DAT_0087f6b4` to `CAMEO.PAL` and `DAT_0087f6b0` to `MOUSEPAL.PAL`. Replace with: "`DAT_0087f6b0` is constructed from the palette loaded from `CAMEO.PAL` and is the ConvertClass used for ordinary player build-cameo base art in `StripClass::Draw`; the following file load at `0x0052C13D` is for the next ConvertClass."
- Replace wording that says current Rust `cameo.pal` use was "not proven" with: "Fresh `StripClass::Draw` evidence proves normal player build-cameo base art uses `DAT_0087f6b0`, constructed from `CAMEO.PAL`; overlay/progress SHPs in the same branch use `DAT_0087f6cc`."

## Sources

- Ghidra MCP read-only decompile: `StripClass::Draw @ 0x006A9540`, `0x0052BA60`.
- Ghidra MCP read-only xrefs: `get_function_xrefs 0x006A9540` -> `0x006A6FDF` from `SidebarClass::Draw`.
- Ghidra MCP read-only assembly: `0x006A99F3..0x006A9A3E`, `0x006A9B2B..0x006A9B46`, `0x006A9B9D..0x006A9BC0`, `0x006A9E7B..0x006A9E97`, `0x006AA144..0x006AA154`, `0x006AA2BA..0x006AA2C9`, `0x0052C089..0x0052C150`, `0x0052C0FA..0x0052C129`.
- Prior Ghidra string-address docs: `SIDEBAR_SYSTEM_GHIDRA_REPORT.md`, `EBOLT_SYSTEM_GHIDRA_REPORT.md`.
