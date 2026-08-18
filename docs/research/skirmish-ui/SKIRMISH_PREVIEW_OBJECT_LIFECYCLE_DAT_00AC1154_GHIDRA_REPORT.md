# Skirmish Preview Object Lifecycle DAT_00AC1154 - Ghidra Research Report

Date: 2026-05-20

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x005E74E0`, `0x00641B00`, `0x00641DB0`, `0x00641EE0`, `0x00640710`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** offline Skirmish map-preview object lifecycle for `DAT_00AC1154`: creation/update/destruction, map-selection and dialog-paint linkage, and whether the path is active in standard YR Skirmish.
**Non-Scope:** exact `[PreviewPack]` channel-order validation, full Choose Map dialog internals, network preview transfer, and assigned-player `mmpb.shp` marker path.
**Confidence:** High for lifecycle/control-flow and object ownership; Medium for decompiler-derived helper names because several Ghidra labels are generic/misleading.
**Active in YR:** Yes. The path is reached from offline Skirmish dialog creation `0x006AE2C0`, dialog proc `0x006AE3F0`, and command handler `0x006ACEE0`.

## 1. Overview

`DAT_00AC1154` is a heap-owned one-field wrapper whose only field is a preview surface pointer. Offline Skirmish creates or refreshes it when a selected map has preview data, paints it during `WM_PAINT` through `DrawStartPositions @ 0x00640710`, and destroys it on dialog teardown or when starting/leaving the setup. The normal stock-map path loads preview data from the selected `.map` file; the random-map path loads `RandMap.img` and separately copies random-map preview bounds into `ScenarioClass`.

## 2. Key Offsets / Globals

| Field / global | Verified behavior | Evidence | Active in YR |
| --- | --- | --- | --- |
| `DAT_00AC1154` | Global pointer to 4-byte wrapper; wrapper field `+0` is a `DSurface`-like preview surface pointer. | `0x006406E0`, `0x006406F0`, `0x00641B00`, `0x00640710` | Yes |
| wrapper `+0` | Set to zero by constructor helper; destroyed by calling inner surface virtual destructor and then cleared. | `0x006406E0`, `0x006406F0` | Yes |
| selected map path `DAT_00A8B8E0` | Normal map preview loader opens this path and decodes the preview. | `0x005E74E0`, `0x00641EE0` | Yes |
| selected map record `+0x58` | Compared to `RandMap.Sed` in random-map check used by Skirmish init/choose-map branches. | `0x0069ADF0` | Conditional: random-map selection |
| selected map record `+0x6A8` | Compared to `RandMap.Sed` in the common preview-loader helper. | `0x0069AE70` | Conditional: random-map selection |
| `RandMap.img` | File loaded into wrapper surface by the random-map image path. | `0x00641DB0` callers at `0x006AEEE0`, `0x006AD9D4`, `0x006ADB02` | Conditional: random-map selection |
| `ScenarioClass+0x112C..0x113C` | Random-map helper copies generated preview source bounds/count into the scenario fields consumed by `DrawStartPositions`. | `0x0058BB30`, `0x00640710` | Conditional: random-map selection |

## 3. Core Lifecycle

### Dialog Entry And Teardown

`0x006AE2C0` creates the offline Skirmish dialog, pumps messages until Start (`0x617`) or Back (`0x5C0`), then destroys the preview wrapper if it still exists:

```text
if DAT_00AC1154 != 0:
  FUN_006406F0(DAT_00AC1154)   ; destroy inner surface and clear wrapper[0]
  FUN_007C8B3D(DAT_00AC1154)   ; free wrapper allocation
  DAT_00AC1154 = 0
```

**Active in YR:** Yes. `0x006AE2C0` is the offline Skirmish show/pump function for dialog `0x102`.

### Initial Dialog Setup

`0x006AE6E0` initializes Skirmish controls and then refreshes the preview object for the current map. The relevant branch:

1. Calls `FUN_0069ADF0`, which compares the selected map name at `record+0x58` against `RandMap.Sed`.
2. If random-map selection is true, calls `0x0058BB30`, destroys any existing wrapper, allocates 4 bytes, initializes wrapper field to zero with `0x006406E0`, stores it in `DAT_00AC1154`, then calls `0x00641DB0("RandMap.img")`.
3. If `wrapper[0] == 0` after loading, calls fallback helper `0x005E74E0`.

**Active in YR:** Yes for dialog init; the `RandMap.img` subpath is Conditional on the selected map being `RandMap.Sed`.

### Choose Map Refresh

`0x006ACEE0` handles `WM_COMMAND` and uses control `0x5AA` for Choose Map. Both the random-map branch and the normal map-list branch converge on the same preview-refresh pattern:

1. Hide parent with `ShowWindow(hwnd, 0)`.
2. Run map chooser.
3. Show parent with `ShowWindow(hwnd, 5)`.
4. Refresh map/session metadata.
5. If a preview can be loaded, destroy the old `DAT_00AC1154` wrapper, allocate a new wrapper, populate it from either `RandMap.img` or selected `.map` preview data, then invalidate the parent dialog.

For the non-random selected-map path, the common loader `0x005E74E0` opens `DAT_00A8B8E0`, allocates the wrapper if the file opens, then calls `0x00641EE0`/`0x00641B00` to parse header/preview data from the `.map` file and decode `[PreviewPack]`.

**Active in YR:** Yes. `0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0`; control `0x5AA` is the offline Skirmish Choose Map button.

### Paint Consumption

`0x006AE3F0` handles `WM_PAINT` after common shell handling. It checks only whether `DAT_00AC1154 != 0`; then it looks up child `0x468`, calls `0x006067A0`, and when that returns zero calls `DrawStartPositions @ 0x00640710`.

`DrawStartPositions` receives the wrapper pointer and:

1. Early-outs if `wrapper[0] == 0`.
2. Uses `wrapper[0]` vtable `+0x78` for preview source rectangle/size.
3. Blits `wrapper[0]` into child `0x468`'s fitted destination.
4. Draws `STARTBUT.SHP` markers and numeric labels if `ScenarioClass+0x113C` is between 1 and 8 inclusive of 1 and exclusive of 9.

**Active in YR:** Yes. Existing live render-path docs and fresh decompile of `0x006AE3F0` confirm the offline Skirmish `WM_PAINT` linkage.

## 4. Normal Map Preview Decode Path

The stock-map path is not just `RandMap.img`. In `0x005E74E0`, the default path:

1. Destroys old `DAT_00AC1154`, then zeroes the global.
2. Opens `DAT_00A8B8E0` through a `CCFileClass`-style file object.
3. If open succeeds, allocates a 4-byte wrapper and initializes it via `0x006406E0`.
4. Calls `0x0069AE70`. If selected map record `+0x6A8` is not `RandMap.Sed`, it calls `0x00641EE0` with the selected map path.
5. Invalidates the dialog.

`0x00641EE0` reads enough of the selected map file to isolate the INI text before `[Map]`, calls `0x00689D30` to read `[Header]` fields, builds a `CCINIClass`, then calls `0x00641B00`.

`0x00641B00` is the preview surface decode/population helper:

- clears INI section cache;
- if `wrapper[0]` already exists, destroys it and clears it;
- reads `[Preview]` through `FUN_00527CC0`;
- allocates a `DSurface` sized from the returned preview record;
- calls surface vtable `+0x18`;
- calls `Pipe__Constructor(PTR_s_PreviewPack_007f004c, buffer, width*height*bpp)` to load compressed preview bytes from `[PreviewPack]`;
- creates an `LZOStraw(1, 0x2000)` chain;
- loops destination surface rows/columns and reads exactly 3 bytes per pixel;
- writes converted DD pixel values through surface vtable `+0x24`;
- returns zero on short read, after unlocking/freeing temporary surfaces and pipes.

**Active in YR:** Yes for normal selected `.map` previews. Evidence is the `0x005E74E0` default path and the `0x00641B00` `[Preview]`/`[PreviewPack]` decode path.

## 5. Random-Map Specific Path

Random map selection is identified by comparing selected map names with `RandMap.Sed`:

- `0x0069ADF0`: checks `record+0x58`.
- `0x0069AE70`: checks `record+0x6A8`.

Random-map image load allocates the same 4-byte wrapper but calls `0x00641DB0("RandMap.img")`. `0x00641DB0` loads that image into a temporary `BSurface`, creates a `DSurface` using the temporary surface dimensions, copies the temporary surface into the wrapper's inner surface, then frees the temporary resources.

`0x0058BB30` copies generated random-map start/bounds data into `ScenarioClass`:

- `ScenarioClass+0x113C = DAT_00ABED0C`;
- each start pair goes to `+0x1140/+0x1144`;
- source origin/size goes to `+0x112C/+0x1130/+0x1134/+0x1138`.

**Active in YR:** Conditional. It is active when the selected map is `RandMap.Sed`; standard stock-map selection uses the normal selected `.map` loader instead.

## 6. Destruction And Edge Cases

- Destroy-before-replace is consistent: `0x006AE6E0`, `0x006ACEE0`, `0x005E74E0`, and `0x006AE2C0` all destroy the old inner surface via `0x006406F0` before freeing the wrapper.
- `0x006406F0` is null-safe for `wrapper[0]`, but callers generally test the wrapper pointer before calling it.
- Allocation failure sets `DAT_00AC1154 = 0`; paint then skips the whole preview branch because `0x006AE3F0` checks the global pointer before calling `DrawStartPositions`.
- Decode failure inside `0x00641B00` returns zero after cleanup. The wrapper may exist with no decoded surface, so `DrawStartPositions` still has its own `wrapper[0] != 0` guard.
- Dialog invalidation happens after preview refresh, so repaint is pull-based through normal `WM_PAINT`; the Choose Map handler does not call `DrawStartPositions` directly.

**Active in YR:** Yes. These branches sit on the standard Skirmish init/choose/paint path; allocation and decode failures are edge conditions.

## 7. Current Rust Implementation Status

Rust currently has only metadata and deliberately disables real preview drawing:

| Rust area | Status | Evidence |
| --- | --- | --- |
| `[Preview]` / `[PreviewPack]` metadata | Present, but no decode; `has_packed_preview` only records existence. | `src/map/preview.rs:28`, `src/map/preview.rs:34` |
| `[Preview] Size` parsing | Present but currently stores the first two values, which is wrong for `Size=0,0,w,h`. | `src/map/preview.rs:50` |
| Source bounds | Intentionally left `None` until verified. | `src/app_list_maps.rs:88` |
| Real preview surface gate | Hardcoded false, so preview surface and `STARTBUT.SHP` overlays are skipped. | `src/app_skirmish_shell_render.rs:458`, `src/app_skirmish_shell_render.rs:305` |
| Draw order roles | Preview role exists but is gated off. | `src/app_skirmish_shell_render.rs:71`, `src/app_skirmish_shell_render.rs:494` |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
| --- | --- | --- | --- |
| Offline Skirmish dialog lifecycle | verified | `0x006AE2C0` | none |
| Dialog `WM_PAINT` consumer | verified | `0x006AE3F0`, `0x00640710` | none |
| Choose Map `0x5AA` refresh branch | verified | `0x006ACEE0` | full chooser dialog internals out of scope |
| Wrapper constructor/destructor helpers | verified | `0x006406E0`, `0x006406F0` | none |
| Normal selected `.map` preview loader | verified | `0x005E74E0`, `0x00641EE0`, `0x00641B00` | exact color-channel parity belongs to PreviewPack slot |
| Random map `RandMap.img` loader | verified | `0x0069ADF0`, `0x0069AE70`, `0x00641DB0` | none for lifecycle |
| Random map scenario-bound copy | verified | `0x0058BB30` | exact generation of globals is outside this slot |
| Network preview transfer | not-touched | string report anchors `Preview.bin`, `NET_PREVIEW_MODE` | out-of-scope: online/lobby path, not offline Skirmish |
| Assigned-player `mmpb.shp` marker path | not-touched | sibling reports cite `0x00640A40` elsewhere | out-of-scope: not `DAT_00AC1154` lifecycle |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `DAT_00AC1154` active in standard offline YR Skirmish? -> Yes, dialog init/paint/command/teardown all reference it on the offline Skirmish path.` (evidence: `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`)
- `[RESOLVED] OQ-2 - What owns the wrapper allocation? -> Callers allocate 4 bytes, initialize with `0x006406E0`, store to `DAT_00AC1154`, and later free with `0x007C8B3D`.` (evidence: `0x006AEEBD..0x006AEEE0`, `0x006AD9A1..0x006AD9D4`)
- `[RESOLVED] OQ-3 - What destroys the inner preview surface? -> `0x006406F0` calls the inner surface's virtual destructor when wrapper[0] is non-null, then clears wrapper[0].` (evidence: `0x006406F0`)
- `[RESOLVED] OQ-4 - How does paint consume the object? -> `0x006AE3F0` checks `DAT_00AC1154`, then `DrawStartPositions` checks wrapper[0] and uses its vtable for size/blit.` (evidence: `0x006AE3F0`, `0x00640710`)
- `[RESOLVED] OQ-5 - Is normal stock-map preview population separate from random map? -> Yes. `0x005E74E0` opens `DAT_00A8B8E0` and calls `0x00641EE0`/`0x00641B00` unless the selected record is `RandMap.Sed`.` (evidence: `0x005E74E0`, `0x0069AE70`)
- `[RESOLVED] OQ-6 - Where does `[PreviewPack]` become a surface? -> `0x00641B00` reads `[Preview]`, allocates a `DSurface`, reads `[PreviewPack]`, LZO-decompresses, and writes 3-byte pixels into the surface.` (evidence: `0x00641B00`)
- `[RESOLVED] OQ-7 - What happens when the user picks a random map? -> Existing preview wrapper is destroyed, a new wrapper is created, `RandMap.img` is loaded through `0x00641DB0`, and `0x0058BB30` copies generated bounds/start fields.` (evidence: `0x006AE6E0`, `0x006ACEE0`, `0x00641DB0`, `0x0058BB30`)
- `[RESOLVED] OQ-8 - Does Choose Map call the paint function directly? -> No. It updates/replaces the object and invalidates the dialog; `WM_PAINT` later calls `DrawStartPositions`.` (evidence: `0x006ACEE0`, `0x006AE3F0`)
- `[RESOLVED] OQ-9 - What if allocation fails? -> The global is set to zero, so `WM_PAINT` skips the preview draw branch.` (evidence: `0x006AE6E0`, `0x006ACEE0`)
- `[RESOLVED] OQ-10 - What if the wrapper exists but no surface is decoded? -> `DrawStartPositions` early-outs because it checks `wrapper[0] != 0`.` (evidence: `0x00640710`)
- `[DEFERRED] OQ-11 - Exact RGB/BGR channel order at the serialized `[PreviewPack]` boundary.` (category: out-of-scope; reason: assigned to PreviewPack decode slot; next-step-if-pursued: compare decoded known map pixels against retail screenshot)
- `[DEFERRED] OQ-12 - Full map chooser modal internals before it returns to `0x006ACEE0`.` (category: out-of-scope; reason: this slot only traces the lifecycle after map selection returns; next-step-if-pursued: trace `FUN_005E68A0` and selected-map globals)
- `[DEFERRED] OQ-13 - Network preview download/upload lifecycle.` (category: out-of-scope; reason: strings indicate separate online transfer path, not offline Skirmish; next-step-if-pursued: investigate `Preview.bin` / `NET_PREVIEW_MODE`)

## Sources

- Fresh Ghidra decompiles: `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x006406E0`, `0x006406F0`, `0x0069ADF0`, `0x0069AE70`, `0x0058BB30`, `0x005E74E0`, `0x00641B00`, `0x00641DB0`, `0x00641EE0`, `0x00689D30`, `0x00640710`.
- Fresh Ghidra assembly context: call sites `0x006AEEAD..0x006AEEE0`, `0x006AD9A1..0x006AD9D4`, `0x006ADB02`, `0x006AE398`, `0x005E78B7..0x005E78CB`, `0x00642079..0x006420A9`.
- Existing reports: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`.
- Rust scan: `src/map/preview.rs`, `src/app_list_maps.rs`, `src/app_skirmish_shell_render.rs`.
