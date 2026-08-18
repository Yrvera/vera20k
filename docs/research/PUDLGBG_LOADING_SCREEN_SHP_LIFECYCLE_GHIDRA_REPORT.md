# PUDLGBG Loading Screen SHP Lifecycle - Ghidra Report

**Target question:** Where and how are `PUDLGBGN.SHP`, `PUDLGBGA.SHP`, `PUDLGBGS.SHP`, and `PUDLGBGY.SHP` loaded into `DAT_00b0fc80/84/88/8c`, what owns/frees them, which MIX/source path supplies them, and whether normal skirmish loading reaches this before `WM_PAINT_Handler` mode 2.

**Investigation Mode:** exhaustive-slice
**Primary addresses:** `0x0072AA40` load, `0x0072B130` local teardown/nulling, `0x005B40B0` cached file loader, `0x00621E90`/containing function `0x006219D0` mode-2 draw reads, `0x0072AC40` UI asset teardown, `0x0052BBBC` startup caller.
**Non-goals:** Full mode-2 visual composition, progress-bar callback, dialog palette construction, skirmish button validation/start packing, full native MIX search-order implementation.
**Evidence needed to mark COMPLETE:** xrefs from all four globals; load/free decompilation; assembly address-range confirmation of filename-to-global mapping; caller evidence for startup and teardown; mode-2 read xrefs only; current Rust loading surface scan; retail archive source check.
**Stop conditions:** Stop at `WM_PAINT_Handler` once the mode-2 reads are proven; do not trace progress-bar drawing or skirmish launch logic except for preload ordering.
**Overall confidence:** HIGH for binary lifecycle and preload-vs-lazy; MEDIUM for exact archive priority because the native `CCFileClass` archive-stack internals were not re-decompiled in this slice.
**Active in YR:** Yes. The strings and load path are present in `gamemd.exe`; `PUDLGBGY.SHP` is YR-only and confirms the YR asset set is active.

## 1. Verified Facts

| Fact | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `FUN_0072AA40` preloads all four PUDLGBG assets into globals. | Decompile `0x0072AA40`; write xrefs `0x0072AA5B/70/85/9A`; assembly shows four consecutive `0x004A38D0` calls. | HIGH | Yes |
| Exact load mapping is `DAT_00b0fc80=PUDLGBGN`, `00b0fc84=PUDLGBGA`, `00b0fc88=PUDLGBGS`, `00b0fc8c=PUDLGBGY`; owner bytes are `00b0fc90..93`. | Assembly: `0x0072AA40` loads pointers from `0x00844C44/48/4C/50`; memory at `0x00845328/18/08/0x008452F8` contains the four filenames. | HIGH | Yes |
| The loader is preload/cached, not lazy from paint. `0x004A38D0` clears the owner byte, calls `LoadFileFromMIX` (`0x005B40B0`) first, and only allocates/direct-reads fallback data if the cached loader returns null. | Decompile `0x004A38D0` and `0x005B40B0`; `0x00621E90` mode-2 xrefs are reads only. | HIGH | Yes |
| `FUN_0072B130` is the only direct PUDLGBG teardown: it checks owner bytes, frees only owner-byte-true fallback allocations, clears owner bytes, and nulls all four globals. MIX/cache-owned assets are not freed by this local path. | Decompile `0x0072B130`; read/write xrefs `0x0072B13D..0x0072B1EF`; calls to `FUN_0069E500` then `FUN_007C8B3D` are gated by owner bytes. | HIGH | Yes |
| Normal game startup reaches `FUN_0072AA40` before any later shell/skirmish mode-2 paint can read the globals. | Caller xref `0x0052BBBC` in startup function calls `0x0072AA40`; no other `0x0072AA40` callers; mode-2 only reads globals at `0x00622233/45/52/5F`. | HIGH | Yes |

## 2. Source MIX / Asset Evidence

Native binary evidence proves the startup path goes through the cached file loader `LoadFileFromMIX` (`0x005B40B0`) via `0x004A38D0`, using the embedded filename strings. A retail-install probe using the repo MIX reader found:

| Asset | CRC hash | Retail source evidence |
|---|---:|---|
| `PUDLGBGN.SHP` | `0x993E1589` | present in `localmd.mix` and `local.mix`; YR should take the `localmd.mix` override under normal MD priority |
| `PUDLGBGA.SHP` | `0x1B6E8258` | present in `local.mix` |
| `PUDLGBGS.SHP` | `0x014E46BA` | present in `local.mix` |
| `PUDLGBGY.SHP` | `0x4BFE5E1B` | present in `localmd.mix` |

The binary lifecycle does not inspect SHP frame count during load. Frame assumption comes from the mode-2 draw path: the selected PUDLGBG pointer is drawn with frame `0`; there is no lifecycle-side validation of frame count or dimensions in `0x0072AA40`.

## 3. Integration Points

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Startup preload | verified | `0x0052BBBC -> 0x0072AA40` | none for lifecycle |
| Cached loader / fallback owner byte | verified | `0x004A38D0`, `0x005B40B0` | exact `CCFileClass` archive priority not re-opened |
| Mode-2 reads | verified | `0x00622233/45/52/5F` read globals; no load calls | full composition covered by slot-1 report |
| Teardown/nulling | verified | `0x0072AC40 -> 0x0072B130`; `0x006BE118`, `0x006BEA3E` shutdown callers | none for these four globals |
| Current Rust | verified | `src/ui/main_menu.rs:301`, `:388`; `src/app.rs` loading states | native preload/render surface absent |

## 4. Current Rust Implementation Status

Rust currently draws an egui loading panel with text and no native SHP surface: `src/ui/main_menu.rs::draw_loading_screen` paints themed background, "Mission deployment", "Loading...", map text, and `loading_screen_image()` returns `None`. `src/app.rs` switches into `GameScreen::Loading` and later performs deferred map load, but there is no PUDLGBG preload/cache, no side-selected loading background, and no owner/lifetime model corresponding to the native globals.

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Startup preloads four PUDLGBG SHPs once into long-lived shell/loading cache before mode-2 paint. | `0x0052BBBC -> 0x0072AA40`; writes `0x0072AA5B/70/85/9A`; mode-2 reads only. | Missing. | Asset/UI startup cache, not `sim/`. | Launch app, open skirmish, start Allied/Soviet/Yuri/no-game loading; first loading paint has the selected native bitmap without lazy hitch or text overlay. | `loading_screen_preloads_pudlgbg_before_first_paint` | Do not lazy-load from the render callback as the primary path. |
| Native selection uses the cached PUDLGBG pointer and draws frame 0 only; lifecycle does not load/validate alternate frames. | Mode-2 read xrefs `0x00622233/45/52/5F`; known slot-1 draw call uses frame 0. | Missing/mismatched: egui text panel. | Loading-screen renderer/compositor. | Allied start shows `PUDLGBGA.SHP` frame 0 at `(0,0)`; Soviet shows `PUDLGBGS`; Yuri shows `PUDLGBGY`; no visible "Loading..." text. | `loading_screen_side_background_frame0_matches_native` | Do not add progress/text overlays to this branch; progress callback is separate. |
| Teardown clears PUDLGBG globals and only frees direct-file fallback allocations; MIX/cache-owned data is not locally freed by `0x0072B130`. | Decompile `0x0072B130`; owner bytes `0x00B0FC90..93`; `0x0072AC40` caller. | Missing cache lifetime model. | Asset cache/shutdown path. | Quitting/restarting shell does not double-free, leak per-start loading SHPs, or reload every skirmish start. | `loading_screen_pudlgbg_cache_lifetime_shutdown_only` | Do not free shared MIX/cache bytes on each game start/end; this is shell/app lifetime, not map lifetime. |

## 6. Negative Facts / Do Not Do

- Do not load PUDLGBG assets from INI; there are no INI keys in this lifecycle.
- Do not treat these as per-skirmish-start allocations; `0x0072AA40` has one startup caller, not a skirmish-start caller.
- Do not make `WM_PAINT_Handler` mode 2 responsible for asset loading; its PUDLGBG global references are read-only.
- Do not free cache-owned MIX assets in the local PUDLGBG teardown; only owner-byte-true fallback allocations take the local free path.
- Do not infer `PUDLGBGN` from base-only data: YR retail has a `localmd.mix` copy as well.

## 7. Remaining Uncertainty

- Exact native `CCFileClass` archive-stack priority was not re-decompiled here. Retail source evidence plus YR conventions identify `localmd.mix` for `PUDLGBGY`/`PUDLGBGN` and `local.mix` for `PUDLGBGA`/`PUDLGBGS`; future archive-order work can harden this.
- SHP header dimensions/frame counts were not parsed in this report because the binary lifecycle does not read them; visual tests should still assert the decoded frame-0 dimensions/content from retail assets.
- The progress callback path is intentionally out of scope; see the separate loading-progress report.

## 8. Stale Docs / Follow-up Wording

- Replace "Rust loading screen uses native loading art" with: "Rust currently uses an egui/text loading panel; native startup preloads PUDLGBGN/A/S/Y SHPs into shell-lifetime cache and mode 2 draws exactly one selected frame-0 background."
- Replace "PUDLGBG can be loaded on demand during loading paint" with: "Native preloads PUDLGBG assets during startup via `0x0072AA40`; mode-2 paint only reads the selected global."
- Replace "free PUDLGBG after each map load" with: "Native PUDLGBG teardown is shutdown/shell teardown; local freeing is gated by fallback owner bytes, while MIX/cache-owned pointers are only nulled by this path."

## Sources

- Ghidra: `0x0072AA40`, `0x004A38D0`, `0x005B40B0`, `0x0072B130`, `0x0072AC40`, `0x006219D0`/mode-2 xref range, startup xref `0x0052BBBC`, teardown xrefs `0x006BE118`, `0x006BEA3E`.
- Retail asset probe: `probe-mix.exe` archive/hash output for `local.mix` and `localmd.mix`; Node hash reproduction using repo `mix_hash.rs` algorithm.
- Rust scan: `src/ui/main_menu.rs`, `src/app.rs`.

**Status:** COMPLETE for the requested PUDLGBG load/free lifecycle slice.
