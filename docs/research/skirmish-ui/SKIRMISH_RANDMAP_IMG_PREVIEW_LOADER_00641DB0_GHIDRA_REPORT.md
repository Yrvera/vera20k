# Skirmish RandMap.img Preview Loader 0x00641DB0 - Ghidra Research Report

**Address(es):** `0x00641DB0`, `0x00595BC0`, `0x007B05C0`, `0x005E8590`, `0x006ACEE0`, `0x006AE6E0`, `0x00640710`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Random-map preview source and lifetime after Create Random Map accept: `RandMap.img` write/read format, `DAT_00AC1154` replacement and fallback behavior, dimensions, and Choose Map/setup preview consumption.  
**Non-Scope:** random terrain generation formulas inside `0x00598960`, `.SED` seed/options serialized layout, normal-map `[PreviewPack]` channel-order details except direct contrast, and full random-map dialog visual layout.  
**Confidence:** High for liveness, wrapper ownership, image format header, dynamic dimensions, null-inner fallback, and Rust deltas; Medium for exact pixel-channel semantics of the PCX writer/reader because the code uses current display pixel-format shift globals.  
**Active in YR:** Conditional. This path is live in standard YR when the selected scenario record is `RandMap.Sed` or the Create Random Map dialog is accepted after a generated preview exists.

## Working Notes Gate

- Target question: What exactly is the random-map preview source/lifetime after Create Random Map accept, especially `0x00641DB0("RandMap.img") -> DAT_00AC1154` and how the shell preview consumes it?
- Non-goals: Do not drain `0x00598960` terrain generation or `.SED` seed layout; do not rediscover normal `[PreviewPack]` decode except to distinguish it from `RandMap.img`.
- Evidence needed to mark COMPLETE: writer path for `RandMap.img`, loader path at `0x00641DB0`, image/surface format and dimensions, `DAT_00AC1154` ownership/fallback behavior, Choose Map/parent paint consumer, and Rust-facing acceptance scenarios.
- Stop conditions: stop once the runtime preview image contract and failure behavior are proven; list generator/SED internals as out-of-scope if they would require a separate slice.

## 1. Overview

`RandMap.img` is a runtime-written preview image, not a map INI and not `[PreviewPack]`. The random-map dialog writes it from the current generated preview surface when the dialog closes with a non-null preview wrapper. The setup shell then loads that file through `0x00641DB0`, which decodes a PCX-style image into a temporary `BSurface`, creates a `DSurface` with the decoded dimensions, copies the temporary surface into wrapper `+0`, and leaves `DAT_00AC1154` pointing at that wrapper.

The preview dimensions are not a fixed `80x50` or `0x468` control size. They are whatever `GenerateTerrainPreview @ 0x00641140` produced for the generated map's playfield projection; the writer stores those dimensions into the image header and the loader recreates a matching `DSurface`.

## 2. Key Offsets / Globals

| Item | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00ABE154` | Random-map dialog preview wrapper; wrapper `+0` is the generated preview surface | `0x00596300` WM_PAINT and command `0x620`; cleanup/write at `0x00595C47..0x00595CAC` | Conditional: random-map dialog generated preview |
| `DAT_00AC1154` | Setup/chooser preview wrapper consumed by parent/chooser paint | replacement calls `0x005E85E7..0x005E8626`, `0x006AD9AF..0x006AD9E6`, `0x006ADAC3..0x006ADB14`, init `0x006AEEA1..0x006AEEF2` | Conditional: selected `RandMap.Sed`; otherwise normal maps use `0x005E74E0` |
| wrapper `+0` | Inner surface pointer; zero means wrapper exists but no drawable preview | constructor `0x006406E0`; destructor `0x006406F0`; draw guard `0x0064072C..0x0064072F` | Yes when wrapper exists |
| string `RandMap.img @ 0x00829ABC` | Filename passed to writer and loader | string xrefs `0x00595C65`, `0x005E861A`, `0x006AD9C8`, `0x006ADAF6`, `0x006AEECA` | Conditional |
| `0x007B05C0` | PCX-style image writer used by random-map dialog shutdown | `0x00595C65..0x00595C84` | Conditional |
| `0x00641DB0` | PCX-style image loader into preview wrapper | xrefs `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0` | Conditional |
| `0x00641140` | Generated preview surface builder; computes dynamic dimensions and paints generated preview pixels/start markers | command `0x620` calls `0x00598960(1, hwnd)` then `GenerateTerrainPreview`; allocation at `0x00641260..0x00641295` | Conditional: preview generation |

## 3. Core Logic

### 3.1 Runtime writer: `0x00595BC0` writes `RandMap.img` on dialog shutdown

Active in YR: Conditional. After the random-map dialog pump exits, `0x00595BC0` checks `DAT_00ABE154`. If the wrapper exists and wrapper `+0` is non-null, it constructs a raw file for `RandMap.img`, pushes palette/global `0x00885780`, passes the inner surface as `EDX`, calls `0x007B05C0`, then destroys the raw file object. It then destroys and frees `DAT_00ABE154` and clears the global.

Evidence: decompile `0x00595BC0`; assembly `0x00595C47..0x00595CAC`, especially `0x00595C61` wrapper `+0` guard, `0x00595C65` filename, `0x00595C79` palette/global push, `0x00595C7E` inner surface load, `0x00595C84` writer call, and `0x00595C9C..0x00595CAC` wrapper teardown.

Tiny details:

- If `DAT_00ABE154 == 0`, no `RandMap.img` write is attempted.
- If `DAT_00ABE154 != 0` but wrapper `+0 == 0`, no `RandMap.img` write is attempted; the wrapper is still destroyed and `DAT_00ABE154` is cleared.
- The file is not deleted after writing in this slice. It remains available for the later `0x00641DB0("RandMap.img")` load.
- The write is a dialog-shutdown side effect, not the launch map-data channel.

### 3.2 Image format: PCX-style `.img`, not `[PreviewPack]`

Active in YR: Conditional. The writer `0x007B05C0` emits a 128-byte PCX-style header before RLE-compressed row data. The header bytes are set directly:

- byte 0 = `0x0A`
- byte 1 = `0x05`
- byte 2 = `0x01`
- byte 3 = `0x08`
- `xmin = 0`, `ymin = 0`
- `xmax = surface_width - 1`
- `ymax = surface_height - 1`
- h/v size fields are written from surface width/height
- color planes byte is `3` when the source surface reports format value `2`, otherwise `1`
- bytes-per-line is the surface width

Evidence: writer assembly `0x007B05D4..0x007B071E`; decompile `0x007B05C0` shows dimensions pulled through source vtable `+0x7C/+0x80`, format through vtable `+0x70`, and row RLE emission. This is also matched by loader validation in `0x00641DB0` through `BSurface__Constructor @ 0x00630310`.

The loader path is therefore not the normal map preview path. Normal map previews use INI text sections `[Preview]` and `[PreviewPack]`, an INI binary reader, and LZO decompression in `0x00641B00`; `RandMap.img` uses a file open plus PCX-style BSurface decode in `0x00641DB0`.

Evidence for contrast: normal selected-map chain `0x005E74E0 -> 0x00641EE0 -> 0x00641B00`; random-map chain `0x005E8590/0x006ACEE0/0x006AE6E0 -> 0x00641DB0`.

### 3.3 Dimensions are dynamic generated-preview dimensions

Active in YR: Conditional. `GenerateTerrainPreview @ 0x00641140` first scans in-playfield cells, converts cell coordinates into projected/radar-like preview coordinates, tracks min/max projected extents, then destroys any old inner surface and allocates a `DSurface` with:

- width = `(max_projected_x - min_projected_x) * 2`
- height = `max_projected_y - min_projected_y`
- constructor flags `(width, height, 1, 0)`

Evidence: `0x00641140` decompile; assembly `0x00641260..0x00641295` computes deltas and calls `DSurface__Constructor`; `0x007B05C0` then writes the actual source surface width/height into `RandMap.img`; `0x00641DB0` later recreates a destination `DSurface` using temporary `BSurface` vtable `+0x7C/+0x80`.

This means Rust should not hardcode `80x50`, `138x75`, `144x112`, or the `0x468` child rect for `RandMap.img`. Those sizes belong to stock map preview packs or UI fitting, not the runtime random preview image.

### 3.4 Loader `0x00641DB0`: destroy old inner surface, decode file, copy into new DSurface

Active in YR: Conditional. `0x00641DB0` receives `ECX = wrapper pointer` and one filename argument. It constructs a `CCFileClass` for that filename, calls `FUN_00473C50` to verify/open availability, then:

1. If wrapper `+0` is non-null, destroys the old inner surface and writes wrapper `+0 = 0`.
2. Constructs a temporary `BSurface` from the file.
3. Requires temporary width and height both nonzero via vtable `+0x7C/+0x80`.
4. Allocates `0x24` bytes for a `DSurface`.
5. Builds the destination `DSurface` using the temporary surface width/height and flags `(1, 0)`.
6. Stores that `DSurface` at wrapper `+0`.
7. Calls destination vtable `+0x18` with `0`.
8. Calls destination vtable `+0x04` with `(temp_surface, 0, 1)` to copy/blit the decoded image.
9. Destroys the temporary `BSurface`.
10. Returns `1`.

Evidence: decompile `0x00641DB0`; assembly `0x00641DB0..0x00641EA8`. The old-inner destruction is `0x00641DD8..0x00641DE4`; temporary load is `0x00641DEA..0x00641DF9`; dimension checks are `0x00641E03..0x00641E1E`; destination allocation/construction is `0x00641E24..0x00641E57`; destination store/copy is `0x00641E57..0x00641E6E`.

### 3.5 Failure behavior: wrapper can survive with null inner surface

Active in YR: Conditional. On loader failure, `0x00641DB0` returns `0` after cleanup but does not free the wrapper passed in by the caller. If the caller allocated and stored a wrapper immediately before the call, `DAT_00AC1154` can remain non-null while wrapper `+0 == 0`.

Evidence: failure branch `0x00641EAB..0x00641EDC`; constructor `0x006406E0` writes wrapper `+0 = 0`; caller allocation/store sequences at `0x005E8601..0x005E8626`, `0x006AD9AF..0x006AD9D4`, `0x006ADADD..0x006ADB02`, `0x006AEEBB..0x006AEEE0`.

Callers handle this in two layers:

- `DrawStartPositions @ 0x00640710` validates the dialog, then immediately tests wrapper `+0`; if null, it returns without drawing. Evidence: `0x00640721..0x0064072F`.
- The parent Choose Map return/init random branches check wrapper `+0` after `0x00641DB0` and call the normal preview refresh `0x005E74E0` if still null. Evidence: `0x006AD9D9..0x006AD9E6`, `0x006ADB07..0x006ADB14`, `0x006AEEE5..0x006AEEF2`.

Important exception: `0x005E8590` itself loads `RandMap.img` after accepted random-map dialog setup but does not immediately inspect the return value or wrapper `+0` before sentinel update. The containing Choose Map command path performs the later null-inner fallback after it calls selected-record load helpers. Evidence: `0x005E861A..0x005E862B` then record scan begins at `0x005E8636`; later parent/modal command fallback in `0x006ACEE0`.

### 3.6 Consumption: preview paint uses current wrapper, not file or selected row directly

Active in YR: Yes for setup/chooser paint when a wrapper exists. The setup dialog and Choose Map modal paint paths consume `DAT_00AC1154` or `DAT_00ABE154` by passing the wrapper to `DrawStartPositions`. `DrawStartPositions` uses child `0x468` only as the destination anchor; it does not reopen `RandMap.img`, decode `[PreviewPack]`, or read listbox selection.

Evidence: setup `WM_PAINT` path `0x006AE454..0x006AE47B`; chooser `0x005E696B..0x005E699F` from prior report; random-map dialog paint `0x00596300` `WM_PAINT`; `DrawStartPositions` draw/blit sequence `0x00640710..0x00640A2F`.

## 4. INI Keys

No INI key is directly read by `0x00641DB0` or the `RandMap.img` writer. The image is generated from the already-built random map preview surface. Random-map seed/options that influence the generated surface belong to the `.SED`/generator reports, not this loader slice.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Random-map dialog Generate command `0x620` | Runs preview-time generator, then `GenerateTerrainPreview`, then posts paint | `0x00596300` command `0x620` | Conditional |
| Random-map dialog shutdown | Writes `RandMap.img` only if `DAT_00ABE154` and wrapper `+0` exist, then destroys dialog preview wrapper | `0x00595C47..0x00595CAC` | Conditional |
| Create Random Map accept setup | Saves `.SED`, replaces `DAT_00AC1154`, calls `0x00641DB0("RandMap.img")`, then continues sentinel update | `0x005E85D1..0x005E862B` | Conditional |
| Setup init / selected `RandMap.Sed` | Loads `RandMap.img`; falls back to normal preview refresh when wrapper `+0` remains null | `0x006AEEA1..0x006AEEF2` | Conditional |
| Choose Map parent return random branch | Loads `RandMap.img`; falls back to `0x005E74E0` on null inner surface and invalidates parent | `0x006AD9AF..0x006AD9F7`, `0x006ADAC3..0x006ADB1E` | Conditional |
| Paint consumer | Blits wrapper `+0` into child `0x468` fitted rect and overlays live start markers when eligible | `0x00640710..0x00640A2F` | Conditional |

## 6. Current Rust Implementation Status

| Rust area | Status vs binary | Evidence |
|---|---|---|
| normal map preview pack parser | present for `[Preview]` / `[PreviewPack]` | `src/map/preview.rs` |
| lazy selected preview texture | present, keyed to committed selected map index | `src/app_skirmish_shell_render.rs::ensure_selected_preview_texture` |
| random sentinel | present but has no random preview image source | `src/skirmish_scenarios.rs::SkirmishScenarioKind::RandomMapSentinel`, `random_map_sentinel` |
| Create Random Map modal helper | partial sentinel upsert only; no generated preview image/write/load model | `src/ui/skirmish_shell/state.rs::ChooseMapModalState::create_random_map` |
| `RandMap.img` loader/decoder | missing | `rg RandMap.img src` found no branch; preview path only decodes concrete map INI preview |
| fallback from failed random preview image to normal preview path/blank guard | missing/unchecked | no wrapper/null-inner model in Rust preview texture cache |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | report section above | none |
| `RandMap.img` writer `0x007B05C0` | verified for header/dimensions/RLE class | `0x007B05C0`, assembly `0x007B05D4..0x007B071E` | exact color-channel proof depends on display pixel-format globals |
| Writer call/lifetime from `0x00595BC0` | verified | `0x00595C47..0x00595CAC` | none |
| Generated preview dimensions | verified | `0x00641140`, assembly `0x00641260..0x00641295` | actual per-seed pixel output out-of-scope |
| Loader `0x00641DB0` | verified | decompile and assembly `0x00641DB0..0x00641EDC` | none |
| Null-inner fallback | verified | `0x0064072C..0x0064072F`, `0x006AD9D9..0x006AD9E6`, `0x006ADB07..0x006ADB14`, `0x006AEEE5..0x006AEEF2` | runtime screenshot of missing file not taken |
| Create Random Map accept path | verified for preview handoff only | `0x005E85E7..0x005E862B`; parent report for sentinel update | `.SED` layout out-of-scope |
| Normal `[PreviewPack]` contrast | verified enough for distinction | `0x005E74E0`, `0x00641EE0`, `0x00641B00`; prior PreviewPack reports | no channel-order rediscovery |
| Rust preview surfaces | verified scan | Codegraph context and `rg` over `src/` | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x00641DB0` active in standard YR Skirmish? -> Yes conditionally: init and Choose Map return call it for selected `RandMap.Sed`, and Create Random Map accepted setup calls it after saving `RandMap.Sed`.` (evidence: `0x005E8626`, `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0`)
- `[RESOLVED] OQ-02 - Where does `RandMap.img` come from? -> The random-map dialog shutdown writes it from `DAT_00ABE154+0` via `0x007B05C0` if a generated preview surface exists.` (evidence: `0x00595C47..0x00595C84`)
- `[RESOLVED] OQ-03 - Is `RandMap.img` a `[PreviewPack]` or INI section? -> No. It is a PCX-style image file loaded through `CCFileClass`/`BSurface`, separate from `0x00641B00` PreviewPack decode.` (evidence: `0x00641DB0`; writer header `0x007B05D4..0x007B071E`; normal contrast `0x00641B00`)
- `[RESOLVED] OQ-04 - What dimensions should the loaded surface have? -> The dimensions encoded in the runtime image header, sourced from `GenerateTerrainPreview` dynamic surface width/height, not the UI control rect or stock PreviewPack sizes.` (evidence: `0x00641260..0x00641295`, `0x007B05F2..0x007B0621`, `0x00641E35..0x00641E4E`)
- `[RESOLVED] OQ-05 - What happens to old preview surfaces? -> Before replacement, callers or `0x00641DB0` destroy the old inner surface; wrapper destructors clear wrapper `+0`; wrappers are heap-freed by callers.` (evidence: `0x006406F0`, `0x00641DD8..0x00641DE4`, caller teardown ranges)
- `[RESOLVED] OQ-06 - What happens if the file is missing or invalid? -> `0x00641DB0` returns `0` and can leave wrapper `+0 == 0`; draw early-outs, and main random branches fall back to `0x005E74E0` when they inspect null inner surface.` (evidence: `0x00641EAB..0x00641EDC`, `0x0064072C..0x0064072F`, `0x006AD9D9..0x006AD9E6`)
- `[RESOLVED] OQ-07 - Does `0x005E8590` itself fallback immediately? -> No; it loads `RandMap.img` then proceeds to sentinel scan/update. The surrounding command path owns the later null-inner fallback.` (evidence: `0x005E861A..0x005E8636`; `0x006ACEE0` fallback ranges)
- `[RESOLVED] OQ-08 - Does paint reopen or decode the image? -> No. Paint only consumes the current wrapper inner surface and child `0x468` anchor.` (evidence: `0x00640710..0x00640A2F`, `0x006AE454..0x006AE47B`)
- `[RESOLVED] OQ-09 - Is this TS-only legacy? -> No TS-only gate was found; it is on live YR shell/random-map UI paths, conditional only on selected random sentinel or accepted dialog result.` (evidence: `0x005E8590`, `0x006ACEE0`, `0x006AE6E0`)
- `[DEFERRED] OQ-10 - Exact generated terrain pixel colors for every seed/option combination.` (category: out-of-scope; reason: belongs to generator terrain/pixel formula slice; next-step-if-pursued: drain `0x00598960` and `0x00641140` color callees)
- `[DEFERRED] OQ-11 - Exact `.SED` serialized seed/options layout.` (category: out-of-scope; reason: sibling slot owns writer layout; next-step-if-pursued: resolve `MapSeedClass` vtable save/load methods)
- `[DEFERRED] OQ-12 - Runtime screenshot of missing/invalid `RandMap.img`.` (category: needs-runtime-debugger; reason: static binary proves null-inner/fallback branches but no runtime visual capture was taken; next-step-if-pursued: delete/corrupt `RandMap.img` between dialog close and parent load under debugger)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Accepted random-map preview uses runtime `RandMap.img`, a PCX-style image written from generated preview surface, not `[PreviewPack]` | writer `0x00595C47..0x00595C84`; loader `0x00641DB0`; normal contrast `0x00641B00` | missing | `src/app_skirmish_shell_render.rs`, `src/map/preview.rs` or a new asset image decoder surface, `src/skirmish_scenarios.rs` | Add a random-sentinel preview source that decodes/uses `RandMap.img` or an equivalent generated preview image, separate from map INI PreviewPack | Accept Create Random Map, select the `RandMap.Sed` sentinel, and assert the preview cache source is random-image data rather than `PreviewPack`; proposed test `skirmish_randmap_sentinel_uses_randmap_img_preview_source` | Do not feed `RandMap.Sed` into normal map preview INI decode or leave the previous concrete map thumbnail visible. |
| `RandMap.img` dimensions are dynamic generated-preview dimensions from `GenerateTerrainPreview`, then aspect-fit into child `0x468` at paint time | `0x00641260..0x00641295`; `0x007B05F2..0x007B0621`; `0x00640778..0x00640887` | missing/unchecked | preview texture metadata and layout fit path in `src/app_skirmish_shell_render.rs` | Preserve decoded image width/height and aspect-fit it like other preview surfaces; do not resize source pixels to `0x468` dimensions during decode | A generated random preview with non-stock dimensions renders fitted inside the map preview rect without forcing `80x50` or `144x112`; proposed test `skirmish_randmap_img_preview_preserves_dynamic_dimensions` | Do not hardcode stock map PreviewPack sizes, `0x468` rect size, or a fixed random-map thumbnail size. |
| Failure can leave a non-null wrapper with null inner surface; main random branches fallback to normal preview refresh, and paint early-outs if inner is null | loader fail `0x00641EAB..0x00641EDC`; draw guard `0x0064072C..0x0064072F`; fallback `0x006AD9D9..0x006AD9E6`, `0x006ADB07..0x006ADB14`, `0x006AEEE5..0x006AEEF2` | missing/unchecked | preview cache invalidation/fallback state in `src/app_skirmish_shell_render.rs` and modal action state in `src/ui/skirmish_shell/state.rs` | Model failed random preview load as no drawable random image and apply the same fallback/blank behavior instead of retaining stale previous preview | Missing/corrupt random image after accepting random map does not keep the old map preview; proposed test `skirmish_randmap_img_missing_does_not_reuse_previous_preview` | Do not treat loader call success as equivalent to drawable preview; callers inspect the inner surface/payload. |
| Random-map dialog writes `RandMap.img` only when a generated dialog preview surface exists; shutdown always frees dialog wrapper | `0x00595C47..0x00595CAC` | missing | future Create Random Map dialog/generator state | Only produce/persist the random preview image after generation created a preview; cancel/no-preview should not synthesize a fake image | Cancel or accept without generated preview leaves no new random preview image and frees transient preview state; proposed test `skirmish_randmap_dialog_writes_img_only_after_generated_preview_exists` | Do not create placeholder `RandMap.img` just because the sentinel exists. |

### Negative Facts / Do Not Do

- Do not decode `RandMap.img` through `[PreviewPack]` logic. Active in YR: No; `RandMap.img` uses `0x00641DB0`/`BSurface`, while `[PreviewPack]` uses `0x00641B00`.
- Do not use `RandMap.img` as playable terrain or launch map data. Active in YR: No; launch uses `.SED` seed/options and generator paths; this report only proves the UI preview image channel.
- Do not hardcode fixed dimensions for random preview images. Active in YR: No; writer and loader round-trip source surface dimensions.
- Do not retain the old concrete-map preview if `RandMap.img` load fails. Active in YR: No for the replacement paths; old inner surface is destroyed before load and paint/fallback sees null inner.
- Do not assume `0x005E8590` alone guarantees a drawable preview. Active in YR: No; it does not check `0x00641DB0` return before sentinel update.

### Remaining Uncertainty

- Exact color-channel equivalence of the PCX writer/reader depends on current DirectDraw pixel-format shift/loss globals. The report proves the file class and dimensions, but not a screenshot-level RGB comparison of a generated seed.
- Runtime visual behavior for intentionally missing/corrupt `RandMap.img` was proven by static branches but not captured in a debugger screenshot.
- Generated terrain preview pixel formulas remain out-of-scope and should be read from the generator/terrain-preview reports.

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` OQ-12 replacement:
  > `[RESOLVED] OQ-12 - Create Random Map preview is a command-side exception, not passive row browsing. The random-map dialog writes runtime `RandMap.img` from `DAT_00ABE154+0` on shutdown when a generated preview exists. Accepted setup replaces `DAT_00AC1154`, calls `0x00641DB0("RandMap.img")`, and later random branches inspect wrapper `+0`; if null, they fall back to `0x005E74E0`. The image is PCX-style and dimensioned from `GenerateTerrainPreview`, not `[PreviewPack]` or a fixed UI control size.`
- `docs/research/skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md` follow-up wording:
  > `The `RandMap.img` preview branch loads a runtime PCX-style image written by the random-map dialog shutdown path. `0x005E8590` does not itself prove drawable preview success; callers must inspect wrapper `+0` and use the documented fallback/blank behavior.`

## Sources

- Ghidra read-only decompile/disassembly: `0x00595BC0`, `0x00596300`, `0x007B05C0`, `0x00641DB0`, `0x00630310`, `0x004BA5A0`, `0x00473C50`, `0x006406E0`, `0x006406F0`, `0x00640710`, `0x00641140`, `0x00641B00`, `0x00641EE0`, `0x005E8590`, `0x006ACEE0`, `0x006AE6E0`, `0x005ED370`, `0x005B9A60`.
- Ghidra string anchor: `RandMap.img @ 0x00829ABC`.
- Prior docs referenced: `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_DAT_00AC1154_LIFECYCLE_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`, `GENERATE_TERRAIN_PREVIEW_START_MARKERS_GHIDRA_REPORT.md`.
- Rust scan: `src/app_skirmish_shell_render.rs`, `src/map/preview.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, plus Codegraph context for preview types.
