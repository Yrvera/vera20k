# DIALOG Palette Startup Path `FUN_0072AA40` - Ghidra Research Report

**Address(es):** `0x0072AA40` primary loader, `0x0072ADE0` PAL loader/converter, `0x0072B230` free path, `0x0072AFF0/0x0072B010/0x0072B030/0x0072B050` accessors  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** startup load order, lifetime, missing-file behavior, conversion behavior, accessors, and normal YR reachability for `DIALOG.PAL`, `DIALOGY.PAL`, `DIALOGN.PAL`, and `MAINBTTN.PAL`.  
**Non-Scope:** `0x0072F350` left-panel palette loader except as a negative contrast, full `PUDLGBG*` SHP lifecycle, progress callback rendering, and complete shell/right-panel asset composition.  
**Confidence:** High for load order, conversion, accessors, teardown, and direct consumers; Medium for whole-program shutdown sequencing beyond the observed teardown callers.  
**Active in YR:** Yes. `0x0052BA60` calls `FUN_0072AA40` during normal startup at `0x0052BBBC`, before audio init.

## 0. Investigation Contract

**Target question:** What exactly does `FUN_0072AA40` load for the loading-screen dialog palette family, in what order, with what lifetime, conversion, missing-file, accessor, and YR-reachability behavior?

**Non-goals:** Do not re-investigate `0x0072F350`; do not trace all `PUDLGBG*` SHP load/free details; do not implement Rust; do not mutate Ghidra state.

**Evidence needed to mark COMPLETE:** decompile plus assembly address-range evidence for `0x0072AA40` and `0x0072ADE0`; xref/caller evidence for startup reachability and accessors; decompile plus assembly evidence for teardown; current Rust scan for implementation deltas.

**Stop conditions:** stop at read-only Ghidra evidence; if a claim needs runtime debugger or a different owner system, mark uncertainty instead of expanding scope.

## 1. Overview

`FUN_0072AA40` is the normal startup loader for the DIALOG-family `ConvertClass` globals used by loading-screen mode 2 and for `MAINBTTN.PAL` used by owner-draw button type `3`. It also loads several SHP globals and initializes right-panel assets under `DAT_00B0FBE0`, but the four palette calls are unconditional and occur before the right-panel lazy-init branch.

The four target palettes are not side-switched by `FUN_0072AA40`. They are all loaded once at startup; side/no-game routing happens later through small accessors consumed by `WM_PAINT_Handler @ 0x00621E90` and `OwnerDraw_Button_00612B70`.

## 2. Core Findings

| Item | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Startup caller | `0x0052BA60` calls `FUN_0072AA40` at `0x0052BBBC`, after `FUN_0060D430` and before `AudioSystem__Init`. | decompile `0x0052BA60`; xref to `0x0072AA40`; assembly context `0x0052BBB7..0x0052BBC7` | Yes |
| Palette load order | Order is `DIALOG.PAL`, `DIALOGY.PAL`, `DIALOGN.PAL`, `MAINBTTN.PAL`. | decompile `0x0072AA40`; assembly contexts `0x0072AAA4..0x0072AAF8`; string/data xrefs `0x00844BA4/BA0/B9C/BA8` | Yes |
| Palette globals | `DIALOG.PAL` -> raw `0x00B0FB64`, convert `0x00B0FB68`; `DIALOGY.PAL` -> raw `0x00B0FB6C`, convert `0x00B0FB70`; `DIALOGN.PAL` -> raw `0x00B0FB5C`, convert `0x00B0FB60`; `MAINBTTN.PAL` -> raw `0x00B0FB74`, convert `0x00B0FB78`. | bulk xrefs to globals; assembly `MOV EDX,<raw>` plus `PUSH <convert>` around `0x0072AAAA..0x0072AAF8` | Yes |
| PAL conversion | `0x0072ADE0` allocates `0x300` bytes and writes 256 RGB triplets with each source component shifted left by 2; max raw `63` becomes `252`, not `255`. | decompile `0x0072ADE0`; assembly context `0x0072AE52..0x0072AE97` shows byte reads, `SHL ...,0x2`, 256 loop | Yes |
| ConvertClass construction | After successful PAL read, `0x0072ADE0` allocates `0x188` bytes and calls `ConvertClass__Constructor(raw, raw, DAT_00887310, 1, 0)`, then writes the convert global. | decompile `0x0072ADE0`; assembly context `0x0072AEBA..0x0072AEDB` | Yes |
| Missing-file behavior | If `FUN_004A3890` returns zero, `0x0072ADE0` jumps to epilogue before allocation and before writing raw/convert outputs; caller continues to the next palette and does not abort. Existing global values are not cleared by this helper. | decompile `0x0072ADE0`; assembly context `0x0072ADF9..0x0072AE06` and consecutive caller calls in `0x0072AA40` | Yes |
| Startup missing-file result | On first normal startup, globals are BSS-zero before `FUN_0072AA40`; a missing target PAL therefore leaves that palette accessor returning null. Consumers check null before drawing. | missing-file helper evidence; mode-2 null guard in `0x00621E90`; button draw null guard in `0x00612B70` | Yes |
| Teardown | `FUN_0072B230` frees raw/convert pairs in order `DIALOGY`, `DIALOG`, `DIALOGN`, `MAINBTTN`, nulling each pointer after free/destructor. Convert destructors are called with argument `3`. | decompile `0x0072B230`; assembly contexts `0x0072B230..0x0072B2E5` | Yes |
| Teardown reachability | `FUN_0072AC40` calls `FUN_0072B230` at `0x0072AC46`; `FUN_0072AC40` is called from `WinMain @ 0x006BE118` and `FUN_006BE1C0 @ 0x006BEA3E`. | decompile/xrefs `0x0072AC40`; xrefs to `0x0072AC40`; assembly context `0x0072AC41..0x0072AC60` | Yes, shutdown/cleanup |

## 3. Exact Load Order and Globals

| Order | Filename pointer slot | Filename string addr | Raw RGB buffer global | ConvertClass global | Call evidence |
|---:|---|---|---|---|---|
| 1 | `0x00844BA4` | `0x0084550C` (`DIALOG.PAL`) | `0x00B0FB64` | `0x00B0FB68` | `0x0072AAA4..0x0072AAB9` |
| 2 | `0x00844BA0` | `0x00845518` (`DIALOGY.PAL`) | `0x00B0FB6C` | `0x00B0FB70` | `0x0072AABE..0x0072AACE` |
| 3 | `0x00844B9C` | `0x00845524` (`DIALOGN.PAL`) | `0x00B0FB5C` | `0x00B0FB60` | `0x0072AAD3..0x0072AAE3` |
| 4 | `0x00844BA8` | `0x008454FC` (`MAINBTTN.PAL`) | `0x00B0FB74` | `0x00B0FB78` | `0x0072AAE8..0x0072AAF8` |

Important ordering detail: table order by pointer address is `DIALOGN`, `DIALOGY`, `DIALOG`, `MAINBTTN`, but execution order is `DIALOG`, `DIALOGY`, `DIALOGN`, `MAINBTTN`.

## 4. Accessors and Consumers

| Accessor | Returns | Verified consumers | Evidence |
|---|---|---|---|
| `FUN_0072AFF0` | `DAT_00B0FB68` (`DIALOG.PAL` ConvertClass) | `WM_PAINT_Handler` calls it twice: Allied side `0`, Soviet side `1`. | decompile `0x0072AFF0`; xrefs `0x00622239`, `0x0062224B`; assembly context `0x00622229..0x00622250` |
| `FUN_0072B010` | `DAT_00B0FB70` (`DIALOGY.PAL` ConvertClass) | `WM_PAINT_Handler` side fallback/Yuri branch. | decompile `0x0072B010`; xref `0x00622258`; assembly context `0x00622252..0x0062225D` |
| `FUN_0072B030` | `DAT_00B0FB60` (`DIALOGN.PAL` ConvertClass) | `WM_PAINT_Handler` no-game branch. | decompile `0x0072B030`; xref `0x00622265`; assembly context `0x0062225F..0x0062226A` |
| `FUN_0072B050` | `DAT_00B0FB78` (`MAINBTTN.PAL` ConvertClass) | `OwnerDraw_Button_00612B70` button type `3`. | decompile `0x0072B050`; xref `0x00612F25`; assembly context `0x00612F20..0x00612F34` |

There are no raw-buffer accessors in this slice. The raw RGB globals are read by teardown and used as constructor inputs; consumers use `ConvertClass` pointers.

## 5. Missing-File and Null Behavior

`0x0072ADE0` does not return a success/failure value to the caller. It opens/reads the named PAL, and only the nonzero read-pointer branch allocates the raw palette and `ConvertClass`. If the read pointer is zero, the helper runs cleanup/destructor epilogue and returns without touching the caller-provided output globals.

`FUN_0072AA40` does not test after any of the four palette calls. A missing `DIALOG.PAL` does not prevent attempts to load `DIALOGY.PAL`, `DIALOGN.PAL`, or `MAINBTTN.PAL`. In normal first startup, the globals begin zero, so missing files propagate as null accessors. The observed consumers guard against null: mode-2 `WM_PAINT_Handler` skips `CC_Draw_Shape` if either selected palette or selected SHP is null, and button paint type `3` also checks both selected convert and SHP before drawing.

## 6. Lifetime

The DIALOG-family palettes are startup-lifetime assets. `FUN_0072AA40` is called from normal startup (`0x0052BA60`) and is not the side-switch palette loader. It does not free existing DIALOG-family palette globals before loading, so it should not be modeled as an idempotent reload entry point.

Cleanup is centralized through `FUN_0072AC40`, which first calls `FUN_0072B130`, then `FUN_0072B230`, then conditionally tears down right-panel assets and later left-panel assets. `FUN_0072B230` is the specific DIALOG-family palette free function.

## 7. Current Rust Implementation Status

Current Rust has a generic PAL parser at `src/assets/pal_file.rs` that scales 6-bit PAL components to 0..255 using `(value * 255 + 31) / 63`. That mismatches this verified gamemd path, which shifts left by 2 and caps at 252. The parser also forces index 0 and exact magenta transparent, while this binary PAL-load helper produces 768 RGB bytes and `ConvertClass`; transparency belongs to later shape/blit semantics, not the PAL conversion itself.

Focused scan did not find a Rust DIALOG-family startup palette set or native loading-screen indexed render path. Existing shell palette loading is currently oriented around `SHELL.PAL`, `SHELL2.PAL`, and other shell/sidebar palettes, while prior loading-screen composition research reports current Rust as an egui one-frame loading surface mismatch.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0072AA40` target palette calls | verified | decompile plus assembly contexts `0x0072AAA4..0x0072AAF8` | none |
| Startup caller `0x0052BA60` | verified | xref and assembly context `0x0052BBBC` | none |
| PAL conversion in `0x0072ADE0` | verified | decompile plus assembly contexts `0x0072AE52..0x0072AEDB` | none |
| Missing-file branch | verified | decompile plus assembly `0x0072ADF9..0x0072AE06` | no runtime missing-file experiment performed |
| Accessors | verified | decompile `0x0072AFF0/B010/B030/B050` plus xrefs | none |
| Consumer null guards | verified | decompile `0x00621E90`, `0x00612B70` | exact player-facing result under intentionally missing retail file not runtime-tested |
| `FUN_0072B230` teardown | verified | decompile plus assembly `0x0072B230..0x0072B2E5` | none |
| `0x0072F350` contrast | touched-not-exhausted | prior `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md` | intentionally not re-investigated |
| Full `PUDLGBG*` SHP lifecycle | deferred | globals consumed in `0x00621E90`; loader partially seen in `0x0072AA40` | separate retry slot target |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `FUN_0072AA40` on normal YR startup? -> Yes, direct call from `0x0052BA60` at `0x0052BBBC`.` (evidence: xref plus assembly context)
- `[RESOLVED] OQ-2 - What is the exact target palette load order? -> `DIALOG`, `DIALOGY`, `DIALOGN`, `MAINBTTN`.` (evidence: `0x0072AAA4..0x0072AAF8`)
- `[RESOLVED] OQ-3 - Which globals hold raw and convert pointers? -> See section 3 table.` (evidence: bulk xrefs plus loader assembly)
- `[RESOLVED] OQ-4 - Does conversion scale to 255 or shift by 2? -> Shift by 2; max 252.` (evidence: `0x0072AE52..0x0072AE97`)
- `[RESOLVED] OQ-5 - What happens if a PAL file is missing? -> Helper does not write outputs; caller continues; first startup leaves null globals.` (evidence: `0x0072ADF9..0x0072AE06`)
- `[RESOLVED] OQ-6 - Which functions expose the palettes? -> `0x0072AFF0`, `0x0072B010`, `0x0072B030`, `0x0072B050` return convert globals only.` (evidence: decompile plus assembly context)
- `[RESOLVED] OQ-7 - Are Allied and Soviet separate DIALOG palettes? -> No; both call `0x0072AFF0`, so both use `DIALOG.PAL`.` (evidence: `0x00622239`, `0x0062224B`)
- `[RESOLVED] OQ-8 - Is `MAINBTTN.PAL` used by loading-screen mode 2? -> No in this slice; it is consumed by owner-draw button type `3`.` (evidence: xref `0x00612F25`)
- `[RESOLVED] OQ-9 - What frees the target palettes? -> `FUN_0072B230`, called by `FUN_0072AC40`.` (evidence: decompile/xrefs)
- `[RESOLVED] OQ-10 - Is this the `0x0072F350` side-switch path? -> No; `0x0072AA40` is startup, while `0x0072F350` is the left-panel palette loader per prior report.` (evidence: this caller xref; prior report)
- `[DEFERRED] OQ-11 - What exact stock user-visible failure occurs if one retail PAL is absent?` (category: `needs-runtime-debugger`; reason: binary shows null propagation and guarded draw, but retail missing-file UX needs a controlled runtime asset-removal experiment; next-step-if-pursued: run under debugger with one PAL removed from MIX/loose override)
- `[DEFERRED] OQ-12 - Does every shutdown route call `0x0072AC40` exactly once?` (category: `requires-different-system-context`; reason: xrefs show two callers, but complete process-exit control flow is outside this startup-loader slice; next-step-if-pursued: trace `WinMain` and `FUN_006BE1C0` teardown modes)

## 10. Active In YR Labels

| Claim | Active in YR | Proof |
|---|---|---|
| `FUN_0072AA40` target palette load | Yes | startup caller `0x0052BBBC` |
| `DIALOG.PAL` loading | Yes | unconditional first target palette call |
| `DIALOGY.PAL` loading | Yes | unconditional second target palette call |
| `DIALOGN.PAL` loading | Yes | unconditional third target palette call |
| `MAINBTTN.PAL` loading | Yes | unconditional fourth target palette call |
| DIALOG-family loading-screen consumption | Conditional | `WM_PAINT_Handler` mode `2`; side/no-game branches |
| `MAINBTTN.PAL` button consumption | Conditional | owner-draw button record type `3` |
| `FUN_0072B230` free path | Yes, cleanup | `FUN_0072AC40` xrefs from WinMain/cleanup function |

No TS-legacy-only gate was found for the target loader or accessors.

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Target PAL conversion is component `<< 2`, not normalized 0..255, and PAL conversion itself does not assign alpha. | `0x0072ADE0`, assembly `0x0072AE52..0x0072AE97`; Rust `src/assets/pal_file.rs` | mismatch | `src/assets/pal_file.rs` or a gamemd-compatible shell/loading palette conversion path | Loading-screen/shell palettes must decode `63` as RGB `252`, not `255`, and avoid PAL-level transparency assumptions where native `ConvertClass` semantics are expected. | Decode a fixture with component `63`; expected channel `252` for DIALOG-family parity path. | Do not globally assume VGA normalization to 255 for gamemd UI palettes without separating modern-preview behavior from parity render behavior. |
| `FUN_0072AA40` loads all four target palettes once at startup and later consumers route via accessors; side selection is not performed by the loader. | loader assembly `0x0072AAA4..0x0072AAF8`; caller `0x0052BBBC`; accessor xrefs | missing | loading-screen asset initialization / shell palette cache | Add a DIALOG-family palette cache with `DIALOG`, `DIALOGY`, `DIALOGN`, `MAINBTTN` loaded in startup-equivalent setup and consumed by loading-screen/button rendering. | First loading screen after Skirmish launch can select Allied/Soviet/Yuri/no-game palette without per-side reload. | Do not put these palettes into the `0x0072F350` side-switch/sidebar loader model. |
| Missing PAL read leaves that palette global null and does not abort remaining target loads; consumers skip drawing when selected convert/SHP is null. | missing branch `0x0072ADF9..0x0072AE06`; no caller tests in `0x0072AA40`; null guards in `0x00621E90` and `0x00612B70` | unchecked/mismatch risk | asset loading error handling for shell/loading screen | Treat missing DIALOG-family PAL as a nullable asset for the parity path, not as an all-loading-screen fatal error, unless a higher-level retail asset availability policy deliberately fails earlier. | With `DIALOGY.PAL` unavailable in a controlled fixture, Allied `DIALOG.PAL` still loads/selects while Yuri mode-2 SHP draw is skipped or flagged null. | Do not make one missing DIALOG-family palette prevent loading the other three. |

Proposed test names:

- `pal_file_gamemd_shift_left_two_maps_63_to_252`
- `loading_palette_startup_loads_dialog_dialogy_dialogn_mainbttn_once`
- `loading_palette_missing_dialogy_does_not_abort_dialog_or_dialogn`

## 12. Negative Facts / Do Not Do

- Do not route `DIALOG.PAL`, `DIALOGY.PAL`, or `DIALOGN.PAL` through `FUN_0072F350`/side-switch palette semantics; `0x0072AA40` is the startup loader, and `0x0072F350` is separate left-panel palette loading per prior report.
- Do not map Soviet to `DIALOGY.PAL`; `WM_PAINT_Handler` calls `FUN_0072AFF0` for side `1`, same as Allied.
- Do not normalize this PAL path to 255 if the target is gamemd UI/loading parity; binary conversion uses `<< 2` and caps at 252.
- Do not treat PAL conversion as RGBA transparency assignment; this helper creates 768 RGB bytes and a `ConvertClass`, while draw/blit code handles visibility.
- Do not make missing `DIALOGY.PAL` abort loading `DIALOGN.PAL` or `MAINBTTN.PAL`; `FUN_0072AA40` continues after each helper call.

## 13. Remaining Uncertainty

- Runtime UX for intentionally missing retail PAL files was not tested; binary evidence only proves null propagation and guarded draw behavior.
- Full `PUDLGBG*` SHP lifecycle remains a sibling retry-slot target; this report only notes their startup loader adjacency and mode-2 consumption where needed for palette consumers.
- Complete shutdown path uniqueness for `FUN_0072AC40` was not exhausted beyond observed xrefs from `WinMain` and `FUN_006BE1C0`.

## 14. Stale Docs / Follow-up Docs

- `docs/research/ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`: replace any wording implying the DIALOG-family palettes are merely "loaded at game startup" without order/lifetime details with: "`FUN_0072AA40` loads `DIALOG.PAL`, `DIALOGY.PAL`, `DIALOGN.PAL`, and `MAINBTTN.PAL` in that execution order during normal startup at caller `0x0052BBBC`; each PAL component is converted by `<< 2` to max 252; missing files leave the corresponding raw/convert globals unchanged/null and do not abort later palette loads; teardown is `FUN_0072B230` via `FUN_0072AC40`."
- `src/assets/pal_file.rs` docs/tests are stale relative to this binary path if used for gamemd UI/loading parity: replace "scale to 8-bit 0..255" wording with a split model, or a gamemd-parity note: "native gamemd PAL conversion shifts 6-bit components left by 2, producing 0..252."

## Sources

- Ghidra read-only decompile: `0x0072AA40`, `0x0072ADE0`, `0x0072B230`, `0x0072AC40`, `0x0052BA60`, `0x00621E90`, `0x00612B70`, `0x0072AFF0`, `0x0072B010`, `0x0072B030`, `0x0072B050`
- Ghidra read-only xrefs: `0x0072AA40`, `0x0072B230`, `0x0072AC40`, accessors `0x0072AFF0/B010/B030/B050`, globals `0x00B0FB5C..0x00B0FB78`, string pointers `0x0084550C/18/24`, `0x008454FC`
- Ghidra read-only assembly contexts: `0x0052BBBC`; `0x0072AAA4..0x0072AAF8`; `0x0072ADF9..0x0072AE06`; `0x0072AE52..0x0072AE97`; `0x0072AEBA..0x0072AEDB`; `0x0072B230..0x0072B2E5`; `0x00622239/4B/58/65`; `0x00612F25`
- Prior docs: `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`, `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`
- Current Rust scan: `src/assets/pal_file.rs`, focused `rg` for DIALOG/MAINBTTN/loading palette surfaces

**Status:** COMPLETE
