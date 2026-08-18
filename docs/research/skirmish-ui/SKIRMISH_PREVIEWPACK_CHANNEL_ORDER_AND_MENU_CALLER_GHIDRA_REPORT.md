# Skirmish PreviewPack Channel Order And Menu Caller - Ghidra Research Report

**Date:** 2026-05-22  
**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x006ACEE0`, `0x005E74E0`, `0x00641EE0`, `0x00641B00`, `0x006418B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `[PreviewPack]` serialized pixel channel order for selected-map Skirmish previews, and the direct standard offline Skirmish menu path that populates/consumes the preview surface for stock map selection.  
**Non-Scope:** network preview transfer, random-map terrain generation internals, full map chooser list filtering/sorting, exact `STARTBUT.SHP` marker projection geometry, and concrete DirectDraw surface subclass internals beyond the vtable slots used here.  
**Confidence:** High for channel order, selected-map caller path, and `0x102` paint consumption.  
**Active in YR:** Yes. The path is reached from the standard offline Skirmish setup dialog `0x102`; no TS-only or optional gameplay flag gates the selected-map preview decode/paint path.

## 1. Overview

The active offline Skirmish selected-map preview path decodes `[PreviewPack]` into a DirectDraw preview surface before `WM_PAINT` draws it in dialog child `0x468`. The serialized decompressed byte stream is row-major 3-byte **RGB**, not BGR: byte 0 is red, byte 1 is green, and byte 2 is blue.

The direct stock menu call chain is:

```text
offline Skirmish dialog 0x102
  0x006AE2C0 setup/pump
  0x006AE3F0 WM_COMMAND / WM_PAINT proc
  0x006ACEE0 Choose Map / refresh command handler
  0x005E74E0 selected-map preview wrapper loader
  0x00641EE0 selected .map INI/header preview loader
  0x00641B00 [Preview]/[PreviewPack] decode to surface
  0x006AE3F0 WM_PAINT -> DrawStartPositions(0x00640710)
```

## 2. Key Offsets / Globals

| Item | Purpose | Active in YR | Evidence |
|---|---|---:|---|
| Dialog `0x102` | Offline Skirmish setup shell. | Yes | `0x006AE2C0` creates/pumps setup dialog until Start `0x617` or Back `0x5C0`. |
| `0x5AA` | Choose Map command id. | Yes | `0x006ACEE0` has live `param_2 == 0x5AA` branch from `0x006AE3F0` `WM_COMMAND`. |
| Child `0x468` | Preview child looked up before paint. | Yes | `0x006AE3F0` `WM_PAINT` calls `GetDlgItem(hwnd, 0x468)` then `DrawStartPositions` if common paint helper returns false. |
| `DAT_00AC1154` | Global 4-byte preview wrapper; wrapper field `+0` is the preview surface pointer. | Yes | `0x005E74E0`, `0x006406E0`, `0x006406F0`, `0x006AE3F0`. |
| `DAT_00A8B8E0` | Current selected map file path opened by the normal selected-map loader. | Yes | `0x005E74E0` constructs/opens the selected file and calls `0x00641EE0`. |
| `PTR_s_Preview_007F0048` / `Preview @ 0x00836DDC` | Section read before surface allocation. | Yes | `0x00641B00`; string anchor report. |
| `PTR_s_PreviewPack_007F004C` / `PreviewPack @ 0x00836DD0` | Binary INI section decoded before LZO decompression. | Yes | `0x00641BCB`, string anchor report. |
| `0x008A0DD0/0x008A0DD4` | DirectDraw red shift/loss. | Yes | `0x00641CD1..0x00641CE3`; address map names these as red. |
| `0x008A0DE0/0x008A0DE4` | DirectDraw green shift/loss. | Yes | `0x00641C8D..0x00641CC7`; address map names these as green. |
| `0x008A0DD8/0x008A0DDC` | DirectDraw blue shift/loss. | Yes | `0x00641C89..0x00641CB5`; address map names these as blue. |

## 3. Core Logic

### 3.1 Active menu caller path

`0x006AE2C0` is the standard offline Skirmish shell entry. It initializes shell resources, creates the setup dialog, stores a result pointer with `SetWindowLongA(hwnd, 8, &local_4)`, pumps until result `0x617` or `0x5C0`, then destroys `DAT_00AC1154` if it exists.

Active in YR: Yes. Evidence: direct dialog setup/pump in `0x006AE2C0`; the same function tears down the preview wrapper after the modal loop.

`0x006AE3F0` is the setup dialog proc. It routes `WM_COMMAND` (`0x111`) to `0x006ACEE0`, and `WM_PAINT` (`0x0F`) consumes the preview wrapper:

```text
if DAT_00AC1154 != 0:
  GetDlgItem(hwnd, 0x468)
  if FUN_006067A0() == 0:
    DrawStartPositions(hwnd)
ValidateRect(hwnd, NULL)
```

Active in YR: Yes. Evidence: decompile `0x006AE3F0`.

`0x006ACEE0` handles command `0x5AA` by hiding the setup dialog, opening the modal map chooser, rebuilding selected-map state, then refreshing the preview. For normal non-random selected maps it calls `0x005E74E0`; random-map selection uses `RandMap.img` first and falls back to `0x005E74E0` if that wrapper surface remains null.

Active in YR: Yes for the command branch; Conditional for the random-map subpath. Evidence: decompile `0x006ACEE0`.

`0x005E74E0` is the selected-map preview wrapper loader. On the default stock-map path it destroys any old `DAT_00AC1154`, opens `DAT_00A8B8E0`, allocates a 4-byte wrapper through `0x006406E0`, checks the `RandMap.Sed` sentinel through `0x0069AE70`, then calls `0x00641EE0(&DAT_00A8B8E0)` and invalidates the parent.

Active in YR: Yes for normal stock selected maps. Evidence: assembly `0x005E78B7..0x005E78CB` shows `0x0069AE70`, `PUSH 0x00A8B8E0`, `CALL 0x00641EE0`, followed by `InvalidateRect`.

`0x00641EE0` reads enough of the selected map file to isolate the INI/header region before `[Map]`, initializes a `CCINIClass`, reads header preview metadata through `0x00689D30`, then calls `0x00641B00`.

Active in YR: Yes for selected-map preview loads. Evidence: decompile `0x00641EE0` and assembly `0x006420A9 CALL 0x00641B00`.

### 3.2 PreviewPack decode and RGB channel order

`0x00641B00` clears the INI section cache, destroys any existing inner preview surface, reads `[Preview]`, allocates a destination surface using the third/fourth `Size` fields, and loads `[PreviewPack]` through the generic binary INI reader:

```text
Pipe__Constructor("PreviewPack", compressed_buffer, width * height * bytes_per_pixel)
LZOStraw__Constructor(mode = 1, block_size = 0x2000)
for y in 0..height:
  for x in 0..width:
    read exactly 3 bytes
    pack byte0/byte1/byte2 through DirectDraw RGB shift/loss globals
    surface.vtable+0x24(write pixel)
```

Active in YR: Yes for selected-map previews. Evidence: `0x00641BCB CALL 0x00526FB0` reads `PreviewPack`; `0x00641C77 CALL 0x0055C7C0` reads 3 decompressed bytes per pixel; short read branches to cleanup and returns `0`.

The load-side channel mapping is decisive:

| Serialized byte | Assembly source after 3-byte read | Conversion globals | Channel |
|---:|---|---|---|
| 0 | `[ESP+0x10]` loaded at `0x00641CD1` | `0x008A0DD4` loss, `0x008A0DD0` shift | Red |
| 1 | `[ESP+0x11]` loaded at `0x00641C8D` | `0x008A0DE4` loss, `0x008A0DE0` shift | Green |
| 2 | `[ESP+0x12]` loaded at `0x00641C89` | `0x008A0DDC` loss, `0x008A0DD8` shift | Blue |

Therefore the decompressed serialized stream is `RGBRGB...`, row-major. It is not BGR.

The writer corroborates the same order. `0x006418B0` iterates source surface height outer, width inner, reads a packed source pixel through vtable `+0x28`, extracts channels through the DirectDraw globals, writes three bytes into a stack buffer, then calls `FUN_0055C350(buffer, 3)`. Assembly `0x006419D4..0x00641A2F` writes the output bytes in red, green, blue order before the pipe write.

Active in YR: Yes for map save/generated preview serialization. Evidence: decompile `0x006418B0`; assembly `0x006419F1`, `0x00641A12`, `0x00641A2B`, `0x00641A2F`.

### 3.3 Row order and edge behavior

Both load and write loops are row-major:

- load: destination surface height via vtable `+0x80`, width via `+0x7C`; inner loop reads one 3-byte RGB triple and writes one destination pixel.
- write: source surface height via vtable `+0x80`, width via `+0x7C`; inner loop extracts one pixel and writes one 3-byte RGB triple.

Active in YR: Yes. Evidence: load loop `0x00641C4D..0x00641D11`; writer loop `0x006419B5..0x00641A51`.

Short/decode failure is not ignored. If `FUN_0055C7C0(..., 3)` returns anything other than `3`, `0x00641B00` unlocks/frees pipe resources and returns `0`. A wrapper can still exist with `wrapper[0] == 0`; `DrawStartPositions` has its own null-surface guard in sibling lifecycle evidence.

Active in YR: Yes, edge condition on the normal loader. Evidence: decompile `0x00641B00`, branch after `0x00641C7C CMP EAX, 0x3`.

## 4. INI / Map Data

No rules/art INI key controls this channel order. The data source is map INI:

| Section/key | Meaning | Active in YR | Evidence |
|---|---|---:|---|
| `[Preview] Size=` | Four-int rectangle in retail maps; fields 3 and 4 are preview surface width/height for the selected-map loader. | Yes | `0x00641B00`; `Dustbowl.map` and retail census docs. |
| `[PreviewPack]` numbered values | INI-binary text section carrying LZO-compressed RGB triples. | Yes | `0x00641BCB`, `0x00526FB0`, `0x0042FE50`, `0x0055C7C0`. |

Retail stock-map census is consistent with the binary path: local stock root maps decode to `width * height * 3` RGB payloads, and baked start-marker pixels are exact RGB `(240,0,0)` components.

## 5. Integration Points

| Integration | Behavior | Active in YR | Evidence |
|---|---|---:|---|
| Dialog init | `0x006AE6E0` can refresh preview state during setup initialization and falls through to selected-map preview load when not random-map. | Yes | decompile `0x006AE6E0`. |
| Choose Map command | `0x006ACEE0` accepted non-random map branch refreshes preview through `0x005E74E0`. | Yes | decompile `0x006ACEE0`; `0x005E74E0`. |
| Paint consumption | `0x006AE3F0` checks `DAT_00AC1154`, child `0x468`, then calls `DrawStartPositions`. | Yes | decompile `0x006AE3F0`. |
| Teardown | `0x006AE2C0` destroys/free wrapper after dialog exits. | Yes | decompile `0x006AE2C0`. |
| Network preview | Separate string anchors exist (`Preview.bin`, `SERIAL_PREVIEW_MODE`, `NET_PREVIEW_MODE`) but they are not the offline `0x102` stock-map selected preview path. | No for this scope | string anchor report; not called by `0x006ACEE0 -> 0x005E74E0 -> 0x00641EE0`. |

## 6. Current Rust Implementation Status

Rust already matches the channel-order finding in the focused decode surface:

| Rust surface | Current status |
|---|---|
| `src/map/preview.rs` | `PREVIEW_CHANNEL_ORDER` is `Rgb`; `decode_preview_pack` base64-decodes, LZO-decompresses, checks `width * height * 3`, then expands triples to RGBA. |
| `src/map/preview.rs` tests | `decode_preview_pack_literal_chunk_to_rgba` and `decode_preview_image_from_ini_decodes_valid_pack` assert `[1,2,3] -> [1,2,3,255]`. |
| `src/app_skirmish_shell_render.rs` | Lazily decodes selected map previews and uploads RGBA texture, but live `STARTBUT.SHP` overlays are still gated by verified source bounds / real preview availability. |
| `src/ui/skirmish_shell/state.rs` / app routing | Choose Map behavior remains not equivalent to the binary modal chooser lifecycle in broader shell work; this report only settles the preview decode/caller facts. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish `0x102` entry/pump | verified | `0x006AE2C0` | none |
| `WM_COMMAND` and `WM_PAINT` routing | verified | `0x006AE3F0` | none for preview caller |
| Choose Map `0x5AA` preview refresh route | verified | `0x006ACEE0`; prior choose-map refresh doc | full chooser list internals out of scope |
| Normal selected-map wrapper loader | verified | `0x005E74E0`; assembly `0x005E78B7..0x005E78CB` | none for stock selected preview |
| Selected `.map` file to INI preview loader | verified | `0x00641EE0`; assembly `0x006420A9` | none for caller proof |
| `[PreviewPack]` decode to surface | verified | `0x00641B00`; assembly `0x00641BCB`, `0x00641C77..0x00641CEF` | concrete surface subclass internals out of scope |
| RGB vs BGR serialized order | verified | load assembly `0x00641C89..0x00641CEF`; writer assembly `0x006419D4..0x00641A2F` | none |
| Row-major order | verified | `0x00641B00`, `0x006418B0` loops | none |
| Random-map `RandMap.img` path | touched-not-exhausted | `0x006ACEE0`, `0x006AE6E0`, prior lifecycle docs | out of scope except as a negative distinction |
| Network preview protocol | not-touched | string anchors only | out of scope; separate online/lobby path |
| Marker projection geometry | not-touched | `DrawStartPositions` called by paint | out of scope for channel/caller slice |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the offline Skirmish preview path tied to active dialog 0x102? -> Yes, `0x006AE2C0` enters the setup dialog, `0x006AE3F0` routes command/paint, and teardown frees `DAT_00AC1154`.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-2 - What direct function path loads stock selected-map previews? -> `0x006ACEE0 -> 0x005E74E0 -> 0x00641EE0 -> 0x00641B00`.` (evidence: decompile `0x006ACEE0`, `0x005E74E0`, `0x00641EE0`, assembly `0x005E78CB`, `0x006420A9`)
- `[RESOLVED] OQ-3 - Does `0x005E74E0` actually call the selected-map file preview loader for normal stock maps? -> Yes, after `0x0069AE70` returns non-random it pushes `DAT_00A8B8E0` and calls `0x00641EE0`.` (evidence: assembly `0x005E78B7..0x005E78CB`)
- `[RESOLVED] OQ-4 - Which section does the preview loader read? -> `0x00641B00` reads `[Preview]`, then calls the binary INI reader for `PreviewPack`.` (evidence: `0x00641B00`, `0x00641BCB`, strings `0x00836DDC`, `0x00836DD0`)
- `[RESOLVED] OQ-5 - Is the decompressed stream RGB or BGR? -> RGB; byte 0 red, byte 1 green, byte 2 blue.` (evidence: assembly `0x00641C89..0x00641CEF`; DD global names from `ADDRESS_MAP.md`)
- `[RESOLVED] OQ-6 - Does the writer agree with load-side RGB? -> Yes, writer extracts red, green, blue and writes three bytes in that order before `FUN_0055C350(..., 3)`.` (evidence: assembly `0x006419D4..0x00641A2F`)
- `[RESOLVED] OQ-7 - Is row order top-to-bottom/left-to-right? -> Yes, height loop outside width loop, one 3-byte pixel per inner iteration.` (evidence: `0x00641B00`; `0x006418B0`)
- `[RESOLVED] OQ-8 - What happens on short decompressed pixel read? -> Loader cleans up and returns `0`; it does not fill partial pixels or swap to a fallback color.` (evidence: `0x00641B00`, branch after `0x00641C77`)
- `[RESOLVED] OQ-9 - Does `WM_PAINT` draw directly after Choose Map? -> No direct draw in the command branch; command refreshes object and invalidates, later `WM_PAINT` calls `DrawStartPositions`.` (evidence: `0x006ACEE0`, `0x006AE3F0`)
- `[RESOLVED] OQ-10 - Is network preview the direct stock offline menu caller? -> No for this scope; network preview strings exist but the offline `0x102` selected-map path reaches `PreviewPack` through `0x005E74E0/0x00641EE0/0x00641B00`.` (evidence: string anchor report; caller chain above)
- `[DEFERRED] OQ-11 - What concrete class implements surface vtable `+0x24/+0x28`?` (category: out-of-scope; reason: not needed to prove serialized channel order because the caller maps bytes through named DD channel globals before vtable pixel write; next-step-if-pursued: separate DSurface vtable investigation)
- `[DEFERRED] OQ-12 - Exact `STARTBUT.SHP` overlay projection and label clipping?` (category: out-of-scope; reason: marker geometry is separate from PreviewPack channel order and menu caller; next-step-if-pursued: trace `DrawStartPositions @ 0x00640710`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `[PreviewPack]` decompressed pixels are row-major RGB triples. | `0x00641C77..0x00641CEF`; writer corroboration `0x006419D4..0x00641A2F` | none observed for current decode constant/tests | `src/map/preview.rs` | Keep expanding triples as `[r,g,b,255]`; do not swap red/blue. | Decode a literal two-pixel LZO chunk `[1,2,3,4,5,6]` and get RGBA `[1,2,3,255,4,5,6,255]`. | Do not implement BGR just because the destination is DirectDraw; DD packing happens after RGB channel interpretation. |
| Standard offline selected-map preview path is `0x006ACEE0 -> 0x005E74E0 -> 0x00641EE0 -> 0x00641B00`, then paint consumes `DAT_00AC1154`. | `0x005E78B7..0x005E78CB`; `0x006420A9`; `0x006AE3F0` | partial broader shell delta: Rust has preview decode/upload, but Choose Map modal lifecycle remains approximate | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Selected stock maps should load/decode preview only from the selected scenario file path and repaint through the shell preview surface path. | Open Skirmish shell, select `Dustbowl.map`, verify the decoded preview texture is drawn in child preview rect and no live overlay markers are synthesized from gameplay waypoints. | Do not treat Choose Map as "next map"; the binary hides parent, runs modal chooser, refreshes state, invalidates. |
| Short decompressed pixel reads abort preview decode instead of drawing partial/corrupt pixels. | `0x00641C77`, `0x00641C7C`, cleanup return in `0x00641B00` | likely matched by Rust byte-count error, but should remain tested | `src/map/preview.rs` | Wrong decompressed byte count must return an error/no preview image, not pad, wrap, or render partial pixels. | Corrupt/truncate a valid PreviewPack fixture and assert no preview texture is uploaded. | Do not silently pad with black or repeat the last pixel; gamemd fails the decode helper. |

## Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md` section 5 is stale where it says channel order is not fully proven / Low confidence. Replacement wording:
  - `Channel order is verified from the active selected-map loader: after a 3-byte decompressed read in 0x00641B00, byte 0 is packed through DirectDraw red loss/shift, byte 1 through green loss/shift, and byte 2 through blue loss/shift. The serialized decompressed PreviewPack stream is row-major RGB, not BGR. Evidence: 0x00641C77..0x00641CEF; writer corroboration 0x006419D4..0x00641A2F.`
- `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md` section 7 row `RGB vs BGR serialized byte order` should change from `deferred` to `verified`, with evidence `0x00641C77..0x00641CEF` and `0x006419D4..0x00641A2F`.

## Negative Facts / Do Not Do

- Do not decode `[PreviewPack]` as BGR. Active in YR: Yes. Evidence: load assembly maps byte 0 to red, byte 1 to green, byte 2 to blue at `0x00641C89..0x00641CEF`.
- Do not treat the final DirectDraw packed surface format as the serialized file format. Active in YR: Yes. Evidence: `0x00641B00` explicitly converts RGB bytes through runtime DD shift/loss globals before vtable `+0x24`.
- Do not use network preview strings (`Preview.bin`, `SERIAL_PREVIEW_MODE`, `NET_PREVIEW_MODE`) as the direct standard offline stock-map caller. Active in YR: No for this scope. Evidence: offline `0x102` selected-map path reaches `PreviewPack` through `0x005E74E0/0x00641EE0/0x00641B00`.
- Do not synthesize live `STARTBUT.SHP` overlays merely because a decoded `[PreviewPack]` exists. Active in YR: Yes. Evidence: paint consumes `DAT_00AC1154`, but live marker eligibility/projection is separately gated in `DrawStartPositions`; retail census shows many stock maps rely on baked red preview pixels.
- Do not pad or partially render truncated decompressed PreviewPack data. Active in YR: Yes edge condition. Evidence: `0x00641B00` requires `FUN_0055C7C0(..., 3) == 3` for each pixel and returns `0` on short read.

## Sources

- Fresh Ghidra decompile: `0x006AE2C0`, `0x006AE3F0`, `0x006ACEE0`, `0x006AE6E0`, `0x005E74E0`, `0x00641EE0`, `0x00641B00`, `0x006418B0`, `0x006406E0`, `0x006406F0`.
- Fresh Ghidra assembly context: `0x005E78B7..0x005E78CB`, `0x00642079..0x006420A9`, `0x00641BCB`, `0x00641C77..0x00641CEF`, `0x006419D4..0x00641A2F`.
- Fresh Ghidra string reports: `PreviewPack @ 0x00836DD0`, `Preview @ 0x00836DDC`; `Preview` anchor report distinguishing offline map preview from network preview strings.
- Existing docs referenced for comparison: `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md`, `docs/research/ADDRESS_MAP.md`.
- Rust scan: `src/map/preview.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs`.
