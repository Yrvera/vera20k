# Skirmish RandMap.img Preview Generation Handoff - Ghidra Research Report

**Address(es):** `0x00596300`, `0x00598960`, `0x00641140`, `0x00595BC0`, `0x00641DB0`, `0x00640710`, `0x005E6920`, `0x005E8590`, `0x006ACEE0`, `0x006AE6E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** random-map preview generation/display handoff around `RandMap.img`: what UI command/control path triggers preview generation, when `GenerateTerrainPreview` runs, when `RandMap.img` is written/loaded into preview state, whether passive Choose Map browsing updates preview, and what Rust must reproduce for pixel/UI parity.  
**Non-Scope:** `.SED` writer field layout, full random terrain-generation formulas, exact random-map setup dialog option semantics, normal map `[PreviewPack]` decode internals except contrast, and full `0x6B`/random-map-dialog button/listbox pixel styling.  
**Confidence:** High for trigger timing, liveness, write/load ownership, passive-browse negative, and Rust-facing preview handoff; Medium for screenshot-exact generated terrain RGB because the full terrain/overlay color formulas and runtime DirectDraw shift/loss globals are outside this slice.  
**Active in YR:** Conditional. The path is active in standard YR when the player enters Choose Map, clicks enabled `Create Random Map` (`0x583`), uses the random-map setup dialog's Generate/Create controls, and the dialog returns success. Passive Choose Map browsing is active as a negative: it does not refresh the preview.

## 0. Working Notes Gate

**Target question:** How does standard YR generate, save, reload, and display the random-map preview image around `RandMap.img`, and does the ordinary Choose Map list browser live-refresh previews?

**Non-goals:** Do not investigate `.SED` bytes, do not reconstruct random terrain algorithms, do not implement Rust, do not broaden into every modal paint pixel, and do not treat `0x583` command behavior as passive list browsing.

**Evidence needed to mark COMPLETE:**

- prove `0x583` / random-map dialog path is live in standard YR, not TS-only legacy;
- prove the exact command/control path that calls `GenerateTerrainPreview`;
- prove preview-time generation ordering, including direct `0x620` command calls and repeated preview refreshes inside `0x00598960`;
- prove `RandMap.img` write timing and guard conditions;
- prove `RandMap.img` load timing into `DAT_00AC1154` and null-inner fallback behavior;
- prove whether `0x6B` passive list browsing updates preview;
- scan current Rust surfaces and state required deltas/tests.

**Stop conditions:** Stop once every preview-generation/display handoff edge is classified. Defer only terrain pixel formulas, `.SED` layout, and screenshot capture of intentionally corrupt runtime files.

## 1. Overview

`RandMap.img` is the UI preview image handoff for generated random maps. It is generated from the random-map dialog preview wrapper `DAT_00ABE154`, written on random-map dialog shutdown only when that wrapper has a drawable inner surface, and then loaded into the setup/chooser preview wrapper `DAT_00AC1154` through the PCX-style loader `0x00641DB0`.

Active in YR: Conditional. Evidence: `0x005E68A0` creates the Choose Map dialog `0x6B` with callback label `0x005E6920`; command `0x583` calls `0x005E8590`; `0x005E8590` calls random-map dialog pump `0x00595BC0`; random-map dialog command `0x620` calls `0x00598960(1, hwnd)` and `GenerateTerrainPreview`; accepted `0x005E8590` then loads `RandMap.img`.

The ordinary Choose Map list does not live-refresh previews while browsing. The modal paint path draws whatever `DAT_00AC1154` already contains; map-list highlight notifications from `0x553` have no preview-loader branch. Create Random Map is a command-side exception, not evidence for passive row browsing.

## 2. Key State / Offsets / Controls

| Item | Purpose | Evidence | Active in YR |
|---|---|---|---|
| Choose Map dialog `0x6B` | Modal parent for map/mode lists and `Create Random Map` button | `0x005E68A0` passes `LAB_005E6920` to dialog creation | Yes |
| Button `0x583` | Choose Map `Create Random Map` command | `0x005E6920` disassembly branch `0x005E69FD..0x005E6B57`; xref to `0x005E8590` at `0x005E6A11` | Conditional: selected mode enables random maps |
| Random-map dialog command `0x620` | Generate/Create preview command inside random-map setup dialog | `0x00596300` decompile; assembly context `0x00596644..0x00596657` | Conditional: random-map dialog |
| `DAT_00ABE154` | Transient random-map dialog preview wrapper; wrapper `+0` is generated preview surface | `0x00596300` paint/generate path; `0x00595BC0` writer guard | Conditional |
| `DAT_00AC1154` | Setup/Choose Map preview wrapper consumed by `DrawStartPositions` | `0x005E8590`, `0x006ACEE0`, `0x006AE6E0`, `0x00640710` | Yes when preview exists |
| `GenerateTerrainPreview @ 0x00641140` | Creates/replaces generated preview surface and bakes terrain/start-marker pixels | direct xrefs from `0x00596300`, `0x00598960`, save paths | Conditional |
| `RandMap.img @ 0x00829ABC` | Runtime image file name for generated preview | writer `0x00595BC0`; loader xrefs `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0` | Conditional |
| `DrawStartPositions @ 0x00640710` | Blits current preview wrapper into child `0x468` and draws live marker overlay when applicable | xrefs from `0x005E6981`, `0x00596ACD`, `0x006AE47B` | Conditional on wrapper inner surface |

## 3. Core Logic

### 3.1 `0x583` is the live Choose Map entry, but it only continues on accepted random setup

Active in YR: Conditional. `0x005E68A0` creates dialog resource `0x6B` with callback label `LAB_005E6920`, sends init message `0x4A9`, shows it, and pumps the modal. In that callback, command `0x583` calls `0x005E8590`; xref evidence shows the direct call at `0x005E6A11`. `0x005E8590` calls `0x00595BC0` and returns `-1` unless the random-map dialog result is exactly `1`.

Evidence: `0x005E68A0` decompile; `0x005E6920..0x005E7044` read-only disassembly; `get_function_xrefs(0x005E8590)` returns `0x005E6A11`; `0x005E8590` decompile.

Implementation consequence: Rust must not commit a random-map sentinel or preview image when the setup dialog cancels or returns any non-`1` result.

### 3.2 Random-map dialog command `0x620` triggers preview generation

Active in YR: Conditional. In `FUN_00596300`, command `0x620` disables the dialog controls, resets random-map working globals, sets display text id `0xF5E`, then calls:

```text
0x00596644 MOV ECX, 0x00ABDFD8
0x00596649 PUSH hwnd
0x0059664A PUSH 1
0x0059664C CALL 0x00598960
0x00596651 MOV ECX, [0x00ABE154]
0x00596657 CALL 0x00641140 ; GenerateTerrainPreview
```

After the final direct preview generation, the branch re-enables controls, copies `DAT_00ABDFD8` into a `MapSeedClass` object at `DAT_00ABE150`, calls display cleanup helpers, and posts `WM_PAINT` (`0x0F`) to the dialog.

Evidence: `0x00596300` decompile; assembly context from `get_assembly_context` at `0x00596657`.

Important tiny details:

- The preview flag passed to `0x00598960` is exactly `1`.
- The dialog HWND is passed into `0x00598960`, so preview refresh messages target the random-map dialog, not the Choose Map modal.
- `GenerateTerrainPreview` is called once directly by the `0x620` command after `0x00598960` returns.
- The final seed-copy to `DAT_00ABE150` happens after the final direct preview generation.

### 3.3 `0x00598960(…, 1, hwnd)` performs repeated preview refreshes during generation

Active in YR: Conditional on nonzero preview flag. The generator helper `0x00598960` contains multiple `(char)param_2 != 0` branches. Each branch loads `DAT_00ABE154` into `ECX`, calls `GenerateTerrainPreview`, and sends/posts `WM_PAINT` (`0x0F`) to the dialog HWND. Xrefs to `GenerateTerrainPreview` from this function are at:

```text
0x00598AA8
0x00598B6A
0x00598BF0
0x00598DD9
0x0059904B
0x005990F0
0x005991DB
0x0059930D
```

Evidence: `0x00598960` decompile; `get_function_xrefs(0x00641140)`; assembly contexts for those xrefs show `MOV ECX,[0x00ABE154]`, `CALL 0x00641140`, then `PUSH 0xF` and a message call.

Implementation consequence: a parity implementation of the actual random-map setup dialog should display progressive preview updates during generation, not only a final image after the generator exits. A staged implementation may start with the final handoff, but that is not full UI parity.

### 3.4 `GenerateTerrainPreview` creates the source image; `RandMap.img` stores that source, not the UI rect

Active in YR: Conditional. `GenerateTerrainPreview @ 0x00641140` scans only in-playfield cells, projects cell centers, computes bounds, destroys any old inner surface in the wrapper, then constructs a new `DSurface` with:

```text
width  = (max_projected_x - min_projected_x) * 2
height =  max_projected_y - min_projected_y
flags  = (1, 0)
```

The terrain pass fills two horizontal pixels per playable cell, and the marker pass writes baked `4x4` red start-marker rectangles for valid waypoint indices `0..7` before the surface is flushed/unlocked.

Evidence: `0x00641140` decompile; read-only disassembly range `0x00641260..0x0064129F`; prior reports `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md` and `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`.

Implementation consequence: decode/render `RandMap.img` as an image with its own dynamic dimensions. Do not derive its source dimensions from child `0x468`, `80x50`, `138x75`, `144x112`, or any stock `[Preview]` size.

### 3.5 `RandMap.img` is written on random-map dialog shutdown only if a generated preview surface exists

Active in YR: Conditional. `0x00595BC0` pumps the random-map dialog and then checks `DAT_00ABE154`. If the wrapper exists and wrapper `+0` is non-null, it constructs `RawFileClass("RandMap.img")`, calls writer `0x007B05C0` with the inner surface, then destroys the file object. It then destroys/frees the transient preview wrapper and clears `DAT_00ABE154` regardless of whether a file was written.

Evidence: `0x00595BC0` decompile; read-only disassembly range `0x00595C40..0x00595CB4`.

Tiny details:

- `DAT_00ABE154 == 0` means no write.
- `DAT_00ABE154 != 0` but wrapper `+0 == 0` also means no write.
- The writer is a PCX-style image writer, not a map INI or `[PreviewPack]` writer.
- `RandMap.img` is a UI preview artifact; launch consumes `.SED` seed/options through a separate branch.

### 3.6 Accepted setup loads `RandMap.img` into `DAT_00AC1154`

Active in YR: Conditional. After `0x00595BC0` returns `1`, `0x005E8590` saves `RandMap.Sed`, destroys any existing `DAT_00AC1154` wrapper, allocates a new preview wrapper, stores it in `DAT_00AC1154`, and calls `0x00641DB0("RandMap.img")`. It does not immediately inspect the loader result before scanning/upserting the sentinel record.

Evidence: `0x005E8590` decompile; assembly context around `0x005E8626` shows `PUSH 0x829ABC` then `CALL 0x00641DB0`, followed by scenario-record scanning.

Parent/setup random branches do inspect the wrapper inner pointer afterward and fall back to normal preview refresh when `+0` is null:

```text
0x006AD9D4 CALL 0x00641DB0
0x006AD9DF CMP [DAT_00AC1154], 0
0x006AD9E6 CALL 0x005E74E0 if null

0x006ADB02 CALL 0x00641DB0
0x006ADB0D CMP [DAT_00AC1154], 0
0x006ADB14 CALL 0x005E74E0 if null

0x006AEEE0 CALL 0x00641DB0
0x006AEEEB CMP [DAT_00AC1154], 0
0x006AEEF2 CALL 0x005E74E0 if null
```

Evidence: `get_assembly_context` for xrefs `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0`.

Implementation consequence: Rust must not keep rendering a stale concrete-map preview after a failed random preview load. The native model distinguishes wrapper existence from drawable inner surface.

### 3.7 `0x00641DB0` preserves runtime image dimensions and can leave wrapper `+0` null

Active in YR: Conditional. Loader `0x00641DB0` opens the file, destroys any old inner surface in the passed wrapper, builds a temporary `BSurface`, requires nonzero temp width and height, allocates a destination `DSurface` using those exact dimensions with flags `(1, 0)`, stores it at wrapper `+0`, copies/blits from the temporary surface, and returns `1`. On failure it returns `0` after cleanup and can leave wrapper `+0 == 0`.

Evidence: `0x00641DB0` decompile; read-only disassembly range `0x00641DB0..0x00641EDF`.

Implementation consequence: Rust should preserve decoded `RandMap.img` dimensions and model failed load as no drawable preview. The current direct-RGB PCX parser is the right file class direction, but the generation/write lifecycle and exact null-inner fallback still need app-state integration.

### 3.8 Paint consumes current wrapper; passive Choose Map list browsing does not refresh it

Active in YR: Yes as normal chooser behavior. `0x005E6920` `WM_PAINT` calls the shell paint helper, tests `DAT_00AC1154`, calls `DrawStartPositions` with the chooser HWND, then validates the dialog. `DrawStartPositions` gets child `0x468`, aspect-fits the wrapper inner surface into it with integer per-mille math, blits the preview, and draws live start-marker overlays where applicable.

Active in YR: Yes as negative for passive browsing. The `0x005E6920` `WM_COMMAND` dispatch handles `0x5C0`, `0x583`, `0x6C5`, and category list `0x6EB` notification `1`. It has no normal `0x553` map-list selection/highlight branch. The `0x6EB` category branch rebuilds/reselects map rows and enables/disables `0x583`, but does not call `0x005E7BF0`, `0x005E74E0`, `0x00641DB0`, `DrawStartPositions`, or `InvalidateRect` for a newly highlighted row.

Evidence: `0x005E68A0` decompile; read-only disassembly range `0x005E6920..0x005E7044`; prior focused report `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`; `DrawStartPositions @ 0x00640710` decompile.

Implementation consequence: do not implement a helpful live preview while browsing the ordinary map list. Normal preview replacement occurs at commit/parent return or through the `0x583` command-side random-map exception.

## 4. INI Keys

No INI key is directly read by the preview handoff functions in this slice. Random-map options that influence generated terrain belong to the random-map setup/generator reports. The preview handoff itself uses dialog commands, globals, runtime files, and surface wrappers.

Active in YR: Not applicable as direct INI behavior. Evidence: `rg` over `ini/` and preview-related source found no handoff-specific INI key; Ghidra paths here read existing random-map state, not INI keys.

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map opens | Resource `0x6B` callback label `0x005E6920` is created and pumped | `0x005E68A0` | Yes |
| `Create Random Map` | Command `0x583` calls `0x005E8590`; aborts if result is `-1` | `0x005E6A11`, `0x005E8590` | Conditional |
| Random-map Generate/Create | Command `0x620` calls `0x00598960(1, hwnd)`, then `GenerateTerrainPreview` | `0x00596300`; `0x00596644..0x00596657` | Conditional |
| Progressive preview | `0x00598960` calls `GenerateTerrainPreview` repeatedly when preview flag is nonzero, sending `WM_PAINT` after calls | xrefs `0x00598AA8..0x0059930D` | Conditional |
| Runtime image write | `0x00595BC0` writes `RandMap.img` only when `DAT_00ABE154+0` exists, then frees `DAT_00ABE154` | `0x00595C40..0x00595CB4` | Conditional |
| Accepted setup load | `0x005E8590` replaces `DAT_00AC1154`, loads `RandMap.img`, then upserts sentinel | `0x005E8626` | Conditional |
| Setup/init fallback | Parent/setup random branches inspect wrapper `+0` and call `0x005E74E0` if null | `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0` contexts | Conditional |
| Paint | `DrawStartPositions` consumes current wrapper and child `0x468`; it does not reopen image files | `0x00640710` | Conditional |
| Passive browsing | Map-list `0x553` highlight has no preview refresh branch | `0x005E6920..0x005E7044` | Yes as negative |

## 6. Current Rust Implementation Status

| Rust area | Status vs binary | Evidence |
|---|---|---|
| `RandMap.img` filename/sentinel detection | present | `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/preview.rs` |
| Runtime `RandMap.img` decode branch | partially present: selected random sentinel reads `RandMap.img` from configured RA2 dir | `decode_randmap_preview_from_runtime_file` in `src/app_skirmish_shell_render/preview.rs` |
| PCX 3-plane direct RGB support | present in current source, including bounds dimensions and RGB plane order | `src/assets/pcx_file.rs` |
| Random sentinel baked-marker overlay suppression | present for selected random sentinel | `is_random_map_sentinel_entry`, `should_draw_start_marker_overlays` |
| Preview cache behavior for random sentinel | partial: random sentinel bypasses cache-current early return and rereads runtime file; failed decode sets preview texture `None` | `ensure_selected_preview_texture` |
| `Create Random Map` button app route | missing: recognized but logs not implemented | `src/app.rs` button branch |
| Random-map setup/generation dialog | missing | no Rust equivalent found in `src/ui/skirmish_shell/state/choose_map.rs` / `src/app.rs` |
| Progressive preview updates during generation | missing | no random generator dialog lifecycle in Rust |
| Native wrapper/null-inner fallback model | partial: Rust has `Option<SkirmishPreviewTexture>` but not explicit wrapper/inner split or parent random-branch fallback after `0x00641DB0` | `src/app_skirmish_shell_render/preview.rs` |
| Passive Choose Map browsing no-refresh | mostly aligned in state: highlight is separate from committed selection; modal preview render/integration remains partial | `ChooseMapModalState::select_map_filtered_row`, `accept_selection` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | Section 0 | none |
| YR liveness of Choose Map `0x6B` | verified | `0x005E68A0`, `0x005E6920` disassembly | none |
| `0x583` to `0x005E8590` | verified | xref `0x005E6A11`; `0x005E8590` decompile | none |
| Random-map dialog `0x620` preview command | verified | `0x00596300`; `0x00596644..0x00596657` | none |
| Repeated preview refreshes inside `0x00598960` | verified | `0x00598960` decompile; xrefs `0x00598AA8..0x0059930D` | exact visual cadence under runtime load not screenshot-captured |
| `GenerateTerrainPreview` source dimensions / baked markers | verified enough for handoff | `0x00641140`; prior dimensions/marker reports | full terrain RGB formulas deferred |
| `RandMap.img` write guard/lifetime | verified | `0x00595BC0`; `0x00595C40..0x00595CB4` | none |
| `RandMap.img` loader/failure behavior | verified | `0x00641DB0`; `0x00641DB0..0x00641EDF` | corrupt-file screenshot deferred |
| Accepted setup load and parent/setup fallback | verified | `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0` contexts | none |
| Passive `0x553` browsing no-refresh | verified | `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`; `0x005E6920..0x005E7044` disassembly pass | row paint visuals out-of-scope |
| Current Rust scan | verified | `rg`, Codegraph context, read of `preview.rs`, `pcx_file.rs`, `choose_map.rs`, `app.rs` | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this live in standard YR, not TS-only legacy? -> Yes, conditionally through standard Choose Map `0x6B`, command `0x583`, random-map dialog `0x00595BC0`, and accepted setup load.` (evidence: `0x005E68A0`, `0x005E6A11`, `0x005E8590`, `0x00596300`)
- `[RESOLVED] OQ-02 - Which control triggers preview generation? -> Random-map dialog command `0x620` triggers preview generation, not ordinary Choose Map map-list browsing.` (evidence: `0x00596300`, `0x00596644..0x00596657`)
- `[RESOLVED] OQ-03 - Does `0x620` call `GenerateTerrainPreview` directly? -> Yes, after `0x00598960(1, hwnd)` returns.` (evidence: `0x00596651..0x00596657`)
- `[RESOLVED] OQ-04 - Does `0x00598960` also call `GenerateTerrainPreview`? -> Yes, multiple nonzero-preview-flag branches call it and send `WM_PAINT`.` (evidence: xrefs `0x00598AA8`, `0x00598B6A`, `0x00598BF0`, `0x00598DD9`, `0x0059904B`, `0x005990F0`, `0x005991DB`, `0x0059930D`)
- `[RESOLVED] OQ-05 - When is `RandMap.img` written? -> After the random-map dialog pump exits, only if `DAT_00ABE154` and wrapper `+0` are non-null.` (evidence: `0x00595BC0`, `0x00595C40..0x00595CB4`)
- `[RESOLVED] OQ-06 - When is `RandMap.img` loaded into setup/chooser preview state? -> Accepted `0x005E8590` replaces `DAT_00AC1154` and calls `0x00641DB0("RandMap.img")`; parent/setup random branches also load it and fallback on null inner surface.` (evidence: `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0`)
- `[RESOLVED] OQ-07 - Does passive Choose Map list browsing update preview? -> No; `0x553` selection/highlight has no preview-refresh branch, and modal paint consumes current `DAT_00AC1154`.` (evidence: `0x005E6920..0x005E7044`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-08 - Does category list change update preview? -> No; `0x6EB` rebuilds/reselects map list rows and button state but does not call preview loaders.` (evidence: `0x005E6B78..0x005E6DB4` from prior report)
- `[RESOLVED] OQ-09 - Does paint reopen `RandMap.img`? -> No; paint calls `DrawStartPositions` on the current wrapper and child `0x468`.` (evidence: `0x00640710`)
- `[RESOLVED] OQ-10 - Does a failed random image load retain old preview? -> Native replacement paths destroy/replace first; loader failure can leave wrapper `+0 == 0`, and later branches fallback or draw early-out.` (evidence: `0x00641DB0`, `0x006AD9D9..0x006AD9E6`, `0x0064072C..0x0064072F`)
- `[RESOLVED] OQ-11 - Is current Rust completely missing `RandMap.img` decode? -> No. Current source has a runtime `RandMap.img` decode branch and 3-plane direct RGB PCX support, but generation/setup handoff is still missing.` (evidence: `src/app_skirmish_shell_render/preview.rs`, `src/assets/pcx_file.rs`)
- `[DEFERRED] OQ-12 - Exact generated terrain RGB for a fixed seed.` (category: out-of-scope; reason: requires full generator/terrain color formula plus runtime display-format validation; next-step-if-pursued: drain `0x00598960` terrain color callees and capture a golden `RandMap.img`)
- `[DEFERRED] OQ-13 - Runtime screenshot of corrupt/missing `RandMap.img` fallback.` (category: needs-runtime-debugger; reason: static branches prove state behavior but no screenshot was captured; next-step-if-pursued: interrupt file between dialog shutdown and parent load under debugger)
- `[DEFERRED] OQ-14 - Exact random-map dialog control paint during progressive generation.` (category: out-of-scope; reason: this handoff proves generation timing and preview state, not every dialog chrome/list/button pixel; next-step-if-pursued: trace `0x00596300` full paint path)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | random-map dialog `0x00596300` command `0x620` | command low word `0x620`; controls disabled before generation | none | dialog HWND | none | Conditional | preview-generation command |
| 2 | `0x00598960` preview branches | `(char)param_2 != 0`; `DAT_00ABE154` passed in `ECX` | generated surface | wrapper `DAT_00ABE154+0` | DirectDraw packed surface pixels | Conditional | progressive generated preview content |
| 3 | direct `GenerateTerrainPreview @ 0x00641140` from `0x620` | after `0x00598960(1, hwnd)` | generated surface with baked red marker pixels | dynamic source dimensions | DD loss/shift packed RGB | Conditional | final dialog preview content |
| 4 | random-map dialog `WM_PAINT` | `DAT_00ABE154 != 0` and suppress helper false | current wrapper inner surface | child `0x468` | surface blit through `DrawStartPositions` | Conditional | dialog preview display |
| 5 | `0x00595BC0` shutdown writer | `DAT_00ABE154 != 0 && *DAT_00ABE154 != 0` | `RandMap.img` | source surface dimensions | PCX-style writer, 1-plane or 3-plane | Conditional | runtime preview file |
| 6 | `0x005E8590` / parent random branches | accepted result or selected random sentinel | `RandMap.img` | loads into `DAT_00AC1154+0` | PCX-style loader to `DSurface` | Conditional | setup/chooser preview state |
| 7 | Choose Map/setup paint via `DrawStartPositions @ 0x00640710` | wrapper exists and wrapper `+0 != 0` | current preview surface plus possible live marker overlay | child `0x468`, aspect-fit integer per-mille | surface blit plus shell SHP markers | Conditional | displayed map preview |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `RandMap.img` | Yes after accepted setup / selected random sentinel | Yes through decoded wrapper | Conditional | Yes | No | No | No | No | `0x00595BC0`, `0x00641DB0`, `0x00640710` |
| Generated `DSurface` at `DAT_00ABE154+0` | Yes during random-map dialog | Yes in random-map dialog paint | Conditional | Yes | No | baked marker pixels included | No | No | `0x00596300`, `0x00641140` |
| Generated `DSurface` at `DAT_00AC1154+0` | Yes after `RandMap.img` load | Yes in setup/chooser paint | Conditional | Yes | No | live marker overlay may be separate | No | No | `0x005E8626`, `0x00640710` |
| `STARTBUT.SHP` marker overlay | Lazy-loaded by `DrawStartPositions` | Conditional | Conditional | No | No | Yes | No | Not a substitute for baked `RandMap.img` markers | `0x00640710`; prior marker reports |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Random-map preview generation is triggered by random-map dialog command `0x620`; it calls `0x00598960(1, hwnd)` and then `GenerateTerrainPreview` again, while `0x00598960` also posts progressive preview paints when the preview flag is nonzero | `0x00596300`, `0x00596644..0x00596657`, xrefs `0x00598AA8..0x0059930D` | missing: no Rust random-map setup/generation dialog or progressive preview lifecycle | `src/app.rs`, future random-map setup UI/state, future generator bridge | Model generation as a dialog command that can update preview during generation and produce a final generated preview before accepted handoff | Open Create Random Map, press Generate/Create, observe preview updates during generation and final preview after completion | `skirmish_randmap_generate_command_updates_dialog_preview_progressively` | Do not synthesize preview at Choose Map button click without the setup dialog/result gate. |
| `RandMap.img` is written only on random-map dialog shutdown when `DAT_00ABE154+0` exists, then accepted setup loads it into `DAT_00AC1154` | `0x00595BC0`, `0x005E8590`, `0x00641DB0` | partial: Rust can read `RandMap.img` if sentinel selected, but no native write/generate lifecycle exists | `src/ui/skirmish_shell/state/choose_map.rs`, `src/app.rs`, `src/app_skirmish_shell_render/preview.rs` | Tie sentinel commit/preview refresh to an accepted random-map setup result with a generated preview image; no image on cancel/no-preview | Accept random-map setup after generation and assert the committed random sentinel preview source is `RandMap.img`; cancel leaves prior selection/preview | `skirmish_randmap_accept_loads_runtime_img_preview_after_generated_surface_exists` | Do not create/update `RandMap.img` or sentinel just because the `0x583` button was clicked. |
| Passive Choose Map map-list browsing does not refresh preview; `0x553` highlight has no preview-loader branch and paint draws current `DAT_00AC1154` | `0x005E6920..0x005E7044`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `0x00640710` | mostly aligned in state but modal preview rendering/integration remains partial | `src/ui/skirmish_shell/state/choose_map.rs`, `src/app_skirmish_shell_render.rs` | Keep highlighted modal row separate from committed preview; refresh only after Use Map/parent return or the `0x583` command-side exception | Highlight several map rows in Choose Map without pressing Use Map; preview remains the previously committed map | `choose_map_modal_highlight_does_not_change_preview_before_accept` | Do not add "live preview while browsing" as a UX improvement. |
| Failed `RandMap.img` load can leave no drawable inner preview; native branches do not retain stale old preview and may fallback to `0x005E74E0` | `0x00641DB0`, `0x0064072C..0x0064072F`, `0x006AD9D9..0x006AD9E6`, `0x006ADB07..0x006ADB14`, `0x006AEEE5..0x006AEEF2` | partial: Rust `Option` clears texture on decode failure, but parent random-branch fallback semantics are not modeled | `src/app_skirmish_shell_render/preview.rs`, app-level modal/selection commit flow | On failed random preview load, clear random preview and apply native fallback/blank behavior without reusing prior concrete thumbnail | Select random sentinel with missing/corrupt `RandMap.img`; old concrete preview is not retained | `skirmish_randmap_img_missing_does_not_reuse_previous_preview` | Do not equate wrapper allocation or sentinel selection with a drawable preview. |
| `RandMap.img` source dimensions are generated-preview dimensions and the file can be PCX-style 3-plane direct RGB | `0x00641140`, `0x007B05C0` via prior loader/dimensions reports, `0x00641DB0` | mostly present: current `PcxFile` handles 3-plane direct RGB and `preview.rs` preserves decoded dimensions | `src/assets/pcx_file.rs`, `src/app_skirmish_shell_render/preview.rs` | Preserve decoded dimensions and aspect-fit at paint; keep tests for 3-plane RGB and dynamic dimensions | Generated random preview with non-stock dimensions decodes and fits into child `0x468` without resizing source pixels | `skirmish_randmap_img_preview_preserves_dynamic_dimensions` | Do not force random preview to `[Preview]`/`[PreviewPack]` dimensions or palette-index semantics. |

### Negative Facts / Do Not Do

- Do not implement passive live preview for ordinary Choose Map `0x553` row highlighting. Active in YR: No; evidence `0x005E6920` command dispatch lacks a `0x553` preview branch.
- Do not use `Create Random Map` `0x583` as proof that ordinary list browsing refreshes preview. Active in YR: Conditional command-side exception only; evidence `0x005E69FD..0x005E6B57`.
- Do not write or load `RandMap.img` on `0x583` click alone. Active in YR: No; accepted setup result `1` and generated preview surface guards are required.
- Do not retain the old concrete-map preview if `RandMap.img` fails to decode/load. Active in YR: No; native destroys/replaces before load and paint early-outs or falls back on null inner surface.
- Do not decode `RandMap.img` as `[PreviewPack]` or force fixed preview dimensions. Active in YR: No; writer/loader use PCX-style runtime image dimensions from the generated surface.

### Remaining Uncertainty

- Exact RGB output for a fixed random-map seed is not proven in this handoff; it belongs to generator/terrain color investigation and runtime fixture capture.
- Runtime screenshot timing during long preview generation was not captured; static binary proves repeated `GenerateTerrainPreview` + `WM_PAINT` calls when preview flag is nonzero.
- Corrupt/missing `RandMap.img` visual fallback was proven statically but not captured in a live debugger screenshot.

### Stale Docs / Follow-up Docs

- Replace any wording that says "ordinary Choose Map browsing live-refreshes the preview" with:
  > Dialog `0x6B` paints preview `0x468` from the current global preview wrapper `DAT_00AC1154`; passive map-list `0x553` highlighting has no preview-refresh branch. Normal preview replacement happens after Use Map/parent return, while `Create Random Map` is a separate command-side exception.

- Refine older Rust-delta wording that says "`RandMap.img` loader/decoder is missing" with:
  > Current Rust has a selected-random-sentinel branch that reads runtime `RandMap.img` and current `PcxFile` supports 3-plane direct RGB. The remaining gap is the native random-map setup/generation/write/accepted-load lifecycle, progressive dialog preview refresh, and exact null-inner/fallback integration.

## Sources

- Ghidra read-only decompile/disassembly: `0x005E68A0`, `0x005E6920..0x005E7044`, `0x005E8590`, `0x00595BC0`, `0x00595C40..0x00595CB4`, `0x00596300`, `0x00596644..0x00596657`, `0x00598960`, `0x00641140`, `0x00641260..0x0064129F`, `0x00641DB0`, `0x00641DB0..0x00641EDF`, `0x00640710`.
- Ghidra xrefs/assembly contexts: `GenerateTerrainPreview` xrefs `0x00596657`, `0x00598AA8`, `0x00598B6A`, `0x00598BF0`, `0x00598DD9`, `0x0059904B`, `0x005990F0`, `0x005991DB`, `0x0059930D`; loader xrefs `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0`.
- Prior docs read: `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md`, `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/preview.rs`, `src/assets/pcx_file.rs`, `src/ui/skirmish_shell/state/choose_map.rs`, `src/ui/skirmish_shell/state.rs`, `src/map/preview.rs`.
