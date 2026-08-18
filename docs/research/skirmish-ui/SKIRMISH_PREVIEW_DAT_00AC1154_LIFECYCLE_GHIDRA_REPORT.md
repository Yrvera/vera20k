# Skirmish Preview DAT_00AC1154 Lifecycle - Ghidra Research Report

Date: 2026-05-21

**Address(es):** `0x00AC1154` global; primary functions `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x005E74E0`, `0x00640710`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish map-preview surface/object lifecycle around `DAT_00AC1154`, from map selection/init through `DrawStartPositions`, including immediate creators, updaters, consumers, and visible teardown.  
**Non-Scope:** online/network preview transfer, full map chooser internals before it returns, exact PreviewPack channel-order proof, and `mmpb.shp` assigned-player marker path.  
**Confidence:** High for lifecycle and call ordering; Medium for helper semantic names where Ghidra labels are constructor-collided.  
**Active in YR:** Yes. Evidence: `Main_Game` calls `FUN_006AE2C0` at `0x0052E168`; `FUN_006AE2C0` creates dialog `0x102` with proc `0x006AE3F0`; `0x006AE3F0` routes init/command/paint to the verified preview paths.

## 1. Overview

`DAT_00AC1154` is the offline Skirmish preview object global. It points to a 4-byte heap wrapper whose only field is the live preview surface pointer. The wrapper is created or replaced during Skirmish dialog init, Choose Map refresh, and selected-map preview loading; paint consumes it by passing it to `DrawStartPositions`; dialog exit and Start/Back cleanup destroy both the inner surface and wrapper allocation.

The lifecycle is pull-rendered. Choose Map and init populate or clear `DAT_00AC1154`, then invalidate the dialog; the preview is actually blitted later from the `WM_PAINT` branch.

## 2. Key Offsets / Globals

| Field / global | Behavior | Evidence | Active in YR |
| --- | --- | --- | --- |
| `DAT_00AC1154` | Global preview-wrapper pointer; zero means no preview branch in offline Skirmish paint. | Xrefs at `0x006AE454`, `0x006AE474`; teardown at `0x006AE38C..0x006AE3A6` | Yes - offline dialog proc uses it on `WM_PAINT` |
| wrapper `+0x00` | Inner surface pointer. Constructor stores zero; destructor virtual-destroys the surface if non-null, then clears it. | `0x006406E0`, `0x006406F0` | Yes - called by init, choose, loader, teardown |
| selected map path `DAT_00A8B8E0` | Normal selected-map loader opens this path and passes it to the INI/PreviewPack loader. | `0x005E78C0..0x005E78CB`, `0x00641EE0` | Yes - normal stock/custom map preview path |
| selected map record `+0x58` | Random-map test for Skirmish init/Choose Map uses string compare to `RandMap.Sed`. | `0x0069ADF0` | Conditional - only when selected map is `RandMap.Sed` |
| selected map record `+0x6A8` | Common loader random-map test uses string compare to `RandMap.Sed`. | `0x0069AE70` | Conditional - only when selected map is `RandMap.Sed` |
| `ScenarioClass+0x112C..0x113C` | Source origin/size and start-count fields consumed by `DrawStartPositions`; random-map helper copies generated values into them. | `0x0058BB30`, `0x00640710` | Yes for drawing; random-map writer is conditional |

## 3. Creator And Updater Lifecycle

### Offline Skirmish Entry

`FUN_006AE2C0` is reached from `Main_Game` at `0x0052E168`, loads Skirmish shell resources, creates dialog `0x102` with proc `0x006AE3F0`, and pumps until Start `0x617` or Back `0x5C0`.

Active in YR: Yes. Evidence: `Main_Game -> 0x006AE2C0` at `0x0052E168`; dialog creation sequence `0x006AE31C` proc `0x006AE3F0`, `0x006AE321` resource `0x102`, `0x006AE328` create call.

### Dialog Init

`FUN_006AE6E0` initializes the offline Skirmish controls and then refreshes the preview. If `FUN_0069ADF0` says the selected map is `RandMap.Sed`, it calls `FUN_0058BB30`, destroys any old wrapper, allocates 4 bytes, initializes the wrapper with `0x006406E0`, stores it to `DAT_00AC1154`, loads `RandMap.img` through `0x00641DB0`, then falls back to `0x005E74E0` if wrapper `[0]` remains null.

Active in YR: Yes for init; `RandMap.img` branch is Conditional on the selected map being `RandMap.Sed`. Evidence: `0x006AEEA1..0x006AEEF2`.

### Choose Map Refresh

`FUN_006AE3F0` routes `WM_COMMAND (0x111)` into `FUN_006ACEE0`; control id `0x5AA` is the Choose Map branch. The branch hides the parent dialog, calls the chooser, shows the parent again, refreshes map/session state, and refreshes `DAT_00AC1154`.

The random-map subpath destroys any old wrapper, allocates a new 4-byte wrapper, initializes it, stores it to `DAT_00AC1154`, calls `0x00641DB0("RandMap.img")`, then calls `0x005E74E0` if the inner surface is still null. The normal-map subpath reaches `0x005E74E0`, which destroys/replaces the wrapper and loads the selected `.map` preview.

Active in YR: Yes. Evidence: `0x006AE443` command dispatch; `0x006ACEE0` `param_2 == 0x5AA`; preview refresh xrefs `0x006AD96E`, `0x006AD9CF..0x006AD9E6`, `0x006ADAFD..0x006ADB14`.

### Normal Selected-Map Loader

`0x005E74E0` always begins by destroying and freeing any existing wrapper, then zeroing `DAT_00AC1154`. In the default/offline path it opens `DAT_00A8B8E0`; if the open succeeds, it allocates 4 bytes, initializes the wrapper, stores it to `DAT_00AC1154`, and if `0x0069AE70` is false, calls `0x00641EE0(DAT_00A8B8E0)`.

`0x00641EE0` reads the selected map file enough to find the INI text before `[Map]`, initializes a `CCINIClass`, reads header data via `0x00689D30`, and calls `0x00641B00`. `0x00641B00` clears any existing inner surface, reads `[Preview]`, creates a `DSurface`, pulls `[PreviewPack]` through an LZO straw, and writes exactly 3-byte pixels into the surface. Short reads return failure after cleanup.

Active in YR: Yes for normal selected-map previews. Evidence: `0x005E74E0`, `0x005E78A5..0x005E78CB`, `0x00641EE0`, `0x00641B00`.

### Random-Map Image Loader

`0x00641DB0` loads the passed filename, used here as `RandMap.img`, into a temporary `BSurface`, allocates a `DSurface` with the temporary surface dimensions, copies the temporary surface into wrapper `[0]`, then frees temporary resources. It first destroys any existing inner surface at wrapper `[0]`.

Active in YR: Conditional. Evidence: `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0` pass `s_RandMap_img_00829ABC` only after the `RandMap.Sed` checks at `0x0069ADF0`.

## 4. Paint Consumer Through DrawStartPositions

`FUN_006AE3F0` handles `WM_PAINT (0x0F)`. It checks `DAT_00AC1154 != 0`, gets child control `0x468`, calls `0x006067A0`, and only when that helper returns zero loads `DAT_00AC1154` into `ECX`, pushes the dialog hwnd, and calls `DrawStartPositions @ 0x00640710`.

`DrawStartPositions` immediately validates the dialog rect and early-outs unless wrapper `[0]` is non-null. It uses the inner surface vtable `+0x78` to get source dimensions, aspect-fits the source to child `0x468` with a `1000` fixed scale, blits the preview surface to `DAT_00887310`, lazily loads `STARTBUT.SHP`, and draws start markers only when `0 < ScenarioClass+0x113C < 9`.

Active in YR: Yes. Evidence: `0x006AE454..0x006AE47B`; `DrawStartPositions @ 0x00640710`; start-count guard and source-bound reads in `0x00640710`.

## 5. Teardown And Edge Cases

`FUN_006AE2C0` destroys the preview object after the dialog loop exits: read `DAT_00AC1154`, if nonzero call `0x006406F0`, free the 4-byte wrapper with `0x007C8B3D`, then write `DAT_00AC1154 = 0`.

`FUN_006ACEE0` also destroys `DAT_00AC1154` on Start/Back command completion before writing the dialog result. Replacement paths destroy-before-replace, so stale inner surfaces are not retained across map changes. Allocation failure writes `DAT_00AC1154 = 0`; paint skips because the global pointer guard fails. Decode or image-load failure can leave a wrapper with `[0] == 0`; `DrawStartPositions` has a second guard and returns without blitting.

Active in YR: Yes. Evidence: dialog teardown `0x006AE38C..0x006AE3A6`; Start/Back cleanup `0x006AD85A..0x006AD8BD`; destructor helper `0x006406F0`; draw guard `0x00640710`.

## 6. Current Rust Implementation Status

Rust now has a lazy decoded preview texture path: `src/map/preview.rs:85` parses `[Preview]`, `src/map/preview.rs:155` decodes `[PreviewPack]`, and `src/app_skirmish_shell_render.rs:765` caches the selected map preview texture. It aspect-fits with floating-point rounding in `src/app_skirmish_shell_render.rs:795`, while gamemd uses integer `1000`-scale fit math in `DrawStartPositions`.

Start marker projection is still intentionally gated by missing verified source bounds: `src/app_list_maps.rs:88` returns `None` for preview source bounds until the exact source is verified. This report does not change that status.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
| --- | --- | --- | --- |
| Offline Skirmish entry to dialog `0x102` | verified | `0x0052E168`, `0x006AE31C..0x006AE328` | none |
| `DAT_00AC1154` wrapper constructor/destructor | verified | `0x006406E0`, `0x006406F0` | none |
| Dialog init preview refresh | verified | `0x006AE6E0`, xrefs `0x006AEEA1..0x006AEEF2` | none for lifecycle |
| Choose Map `0x5AA` preview refresh | verified | `0x006ACEE0`, xrefs `0x006AD96E`, `0x006AD9CF..0x006ADB14` | map chooser internals before return are out of scope |
| Normal `.map` preview population | verified | `0x005E74E0`, `0x00641EE0`, `0x00641B00` | channel-order proof belongs to PreviewPack slot |
| Random-map `RandMap.img` population | verified | `0x0069ADF0`, `0x0069AE70`, `0x00641DB0` | none for lifecycle |
| `WM_PAINT` consumer and `DrawStartPositions` | verified | `0x006AE454..0x006AE47B`, `0x00640710` | exact marker coordinate parity is separate from lifecycle |
| Dialog/Start/Back teardown | verified | `0x006AE38C..0x006AE3A6`, `0x006AD85A..0x006AD8BD` | none |
| Online/network preview paths | deferred | non-scope; multiple non-skirmish xrefs to same global/function cluster | investigate in a separate online preview slot |
| `mmpb.shp` assigned-player marker path | deferred | sibling reports identify `0x00640A40`, not this `DAT_00AC1154` paint call | investigate only if assigned-player markers are in scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `DAT_00AC1154` active in standard YR offline Skirmish? -> Yes. It is reached from `Main_Game`, dialog `0x102`, `WM_COMMAND`, `WM_PAINT`, and teardown.` Evidence: `0x0052E168`, `0x006AE2C0`, `0x006AE3F0`.
- `[RESOLVED] OQ-2 - Who creates the wrapper? -> Init, Choose Map random-map refresh, and normal map loader allocate 4 bytes, initialize with `0x006406E0`, and store to `DAT_00AC1154`.` Evidence: `0x006AEEBB..0x006AEEDB`, `0x006AD9AF..0x006AD9CF`, `0x005E7890..0x005E78A5`.
- `[RESOLVED] OQ-3 - Who consumes it for drawing? -> Offline Skirmish `WM_PAINT` passes it as `this`/`ECX` into `DrawStartPositions` after child `0x468` setup.` Evidence: `0x006AE454..0x006AE47B`.
- `[RESOLVED] OQ-4 - What tears it down? -> Dialog exit and Start/Back cleanup call `0x006406F0`, free the wrapper, and clear the global.` Evidence: `0x006AE38C..0x006AE3A6`, `0x006AD85A..0x006AD8BD`.
- `[DEFERRED] OQ-5 - Are other xrefs to `DAT_00AC1154` player-visible online preview paths?` Category: out-of-scope. This slot only covers offline Skirmish preview lifecycle.

## Sources

- Read-only Ghidra decompiled/inspected: `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x006406E0`, `0x006406F0`, `0x00640710`, `0x005E74E0`, `0x00641B00`, `0x00641DB0`, `0x00641EE0`, `0x0069ADF0`, `0x0069AE70`, `0x0058BB30`.
- Read-only Ghidra xref/context: `DAT_00AC1154` xrefs; `Main_Game -> 0x006AE2C0` at `0x0052E168`; `DrawStartPositions` call at `0x006AE47B`; normal loader call at `0x005E78CB`; random loader calls at `0x006AD9D4`, `0x006ADB02`, `0x006AEEE0`.
- Prior docs checked: `SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_ACTION_TRACE.md`, `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`.
- Rust status checked: `src/map/preview.rs`, `src/app_skirmish_shell_render.rs`, `src/app_list_maps.rs`.
