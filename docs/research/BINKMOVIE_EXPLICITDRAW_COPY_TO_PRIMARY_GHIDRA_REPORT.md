# BinkMovie ExplicitDraw Copy-To-Primary Path - Ghidra Research Report

**Address(es):** `0x005C05F0` thunk, `0x00433060` explicit copy path, `0x00432A35..0x00432A44` surface-type setup  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the explicit draw path reached by owner-draw static `0x71A` message `0x4F0`: target surface selection, `BinkCopyToBuffer` arguments, copy flags, rect offsets, and Rust-facing color/composition implications.  
**Non-Scope:** BIK-vs-VQA lookup, `_BinkWait` cadence, loop/end behavior, Bink audio, and full DirectDraw initialization.  
**Confidence:** High for binary call/argument/order findings from retail `gamemd.exe` static disassembly; Medium for the symbolic SDK name of `0x80000000` because that comes from Bink API references, not from `gamemd.exe` symbols.  
**Active in YR:** Yes. The path is reached by standard main menu dialog `0xE2` paint dispatch to child static `0x71A`, as established by `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`.

## Working Notes Gate

- **Target question:** What exactly does `BinkMovie_ExplicitDraw_005C05F0` copy to, with which rect, pitch/height, and copy flags, when main-menu static `0x71A` receives `0x4F0`?
- **Non-goals:** Do not re-investigate dialog `0xE2` dispatch, BIK file lookup, cadence, audio, loop behavior, or Rust implementation.
- **Evidence needed to mark COMPLETE:** Decompile/disassembly evidence for `0x005C05F0 -> 0x00433060`, `_BinkCopyToBuffer@28` argument order, target surface branch, `_BinkDDSurfaceType` storage/use, and current Rust surface/color/composition delta.
- **Stop conditions:** Missing live Ghidra function boundary under the read-only rule; runtime-only DirectDraw surface format value unavailable; or any unresolved branch that changes explicit-draw output.

## 1. Overview

`BinkMovie_ExplicitDraw_005C05F0` is a tiny thunk: it loads the inner `BinkObject*` from the generic movie handle at `handle+0x10` and jumps to `0x00433060`. The real explicit draw function locks the Bink object's destination surface, queries pitch and destination height from that surface, then calls `_BinkCopyToBuffer@28` with the stored Bink handle, locked pointer, pitch, surface height, stored destination x/y, and `object+0x08 | 0x80000000`.

The target is not the per-dialog shell `BSurface` and not `DAT_00887310` (`AlternateSurface`). The Bink object destination at `object+0x0C` is selected during open: normally `DAT_0088730C` (`HiddenSurface`) when the Bink helper BSurface exists, otherwise `DAT_00887308` (primary DirectDraw surface). The explicit copy handles both branches; the primary branch adds the main window client-origin offset before copying.

## 2. Class Layout / Key Offsets

| Offset | Type / role | Evidence | Active in YR |
|---:|---|---|---|
| `BinkMovieHandle+0x10` | pointer to inner `BinkObject` | `0x005C05F0`: load `[ecx+0x10]`, jump `0x00433060` | Yes - vtable+`0x28` from static `0x71A` |
| `BinkObject+0x04` | `HBINK` passed as arg 1 to `_BinkCopyToBuffer@28` | `0x00433151..0x00433155` | Yes |
| `BinkObject+0x08` | `_BinkDDSurfaceType` result, reused as base copy flags | setup at `0x00432A35..0x00432A44`; use at `0x004330E7..0x004330F4` and `0x0043313A..0x00433147` | Yes |
| `BinkObject+0x0C` | destination surface pointer | branch compare to `DAT_00887308` at `0x0043306E..0x00433073`; lock/query/copy target | Yes |
| `BinkObject+0x10/+0x14` | stored clipped x/y | used as copy `dest_x/dest_y`; primary branch adds client-origin offset | Yes |
| `BinkObject+0x18/+0x1C` | stored clipped w/h | not direct `_BinkCopyToBuffer` args; used by pre/post helper overlays | Yes, when helper BSurface exists |
| `BinkObject+0x20` | optional helper BSurface/event surface | checked by `0x00433330` and `0x00433270`; also decides HiddenSurface target during open | Conditional - active when constructed helper is valid |

## 3. Core Logic

### 3.1 Thunk

`0x005C05F0` does not compute draw state. It performs `ecx = [ecx+0x10]` and jumps to `0x00433060`. Active in YR: Yes, because owner-draw static `0x71A` dispatches vtable+`0x28` on message `0x4F0`.

### 3.2 Explicit Copy Target Branch

`0x00433060` begins by loading `surface = object+0x0C` and comparing it to `DAT_00887308`.

| Branch | Condition | Copy x/y | Destination | Evidence | Active in YR |
|---|---|---|---|---|---|
| Primary branch | `object+0x0C == DAT_00887308` | `x = object+0x10 + client_origin.x`, `y = object+0x14 + client_origin.y` after `GetClientRect` + `ClientToScreen` on `DAT_00B73550` | primary DirectDraw surface | `0x00433063..0x004330FE`; imports `GetClientRect` at `0x007E14C4`, `ClientToScreen` at `0x007E14B8` | Conditional - fallback/direct primary mode |
| Non-primary branch | `object+0x0C != DAT_00887308` | `x = object+0x10`, `y = object+0x14` | `object+0x0C`, normally `DAT_0088730C` HiddenSurface | `0x00433100..0x00433155`; target selected in open path `0x004328D5..0x00432903` | Yes in standard menu when helper BSurface is valid |

### 3.3 `_BinkCopyToBuffer@28` Arguments

Both branches lock the destination surface through vtable slot `+0x5C` with two zero arguments. If the lock returns `0`, no copy occurs; the function still runs the post-copy helper and returns. On success, it queries pitch through surface vtable `+0x74`, destination height through vtable `+0x80`, and calls `_BinkCopyToBuffer@28` at import IAT `0x007E15B8`.

The argument order is:

| Bink arg | Primary branch value | Non-primary branch value | Evidence | Active in YR |
|---|---|---|---|---|
| `bink_handle` | `object+0x04` | `object+0x04` | push before call at `0x00433151..0x00433155` | Yes |
| `dest_ptr` | lock return pointer | lock return pointer | lock result stored/tested at `0x004330C3..0x004330CC`, `0x0043311C..0x00433123` | Yes |
| `pitch` | surface vtable `+0x74` result | surface vtable `+0x74` result | `0x004330D2..0x004330DD`, `0x00433125..0x00433130` | Yes |
| `dest_height` | surface vtable `+0x80` result | surface vtable `+0x80` result | `0x004330D9..0x004330E1`, `0x0043312C..0x00433134` | Yes |
| `dest_x` | `object+0x10 + client_origin.x` | `object+0x10` | primary `0x0043309C..0x004330A7`; non-primary `0x00433100..0x00433109` | Yes/Conditional by branch |
| `dest_y` | `object+0x14 + client_origin.y` | `object+0x14` | primary `0x004330A9..0x004330AE`; non-primary `0x00433103` | Yes/Conditional by branch |
| `copy_flags` | `object+0x08 | 0x80000000` | `object+0x08 | 0x80000000` | `0x004330E7..0x004330F4`, `0x0043313A..0x00433147` | Yes |

The explicit draw path does not pass `object+0x18/+0x1C` as width/height to `_BinkCopyToBuffer`; it uses the destination surface height from vtable `+0x80`. Active in YR: Yes; evidence is the push sequence at `0x004330F4..0x00433155`.

### 3.4 `0x80000000` Flag

The binary unconditionally ORs `0x80000000` into the stored surface-type flags for explicit draw. Active in YR: Yes, on every successful explicit draw copy. Evidence: `or ecx, 0x80000000` at `0x004330EE` and `0x00433141`.

External Bink API references identify this high bit as `BINKCOPYALL`, i.e. copy all pixels, not only changed blocks. The binary evidence alone proves the high-bit OR; the name/semantic explanation is corroborating API context, not a `gamemd.exe` symbol.

### 3.5 Surface Type Path

`FUN_00432750` stores the surface-format flags once at open time:

- it reads `DAT_00887308`;
- loads `[DAT_00887308+0x1C]`;
- calls `_BinkDDSurfaceType@4` via IAT `0x007E15A8`;
- stores the return value at `BinkObject+0x08`.

Evidence: `0x00432A35..0x00432A44`. Active in YR: Yes for every successful Bink open in the main-menu path.

Important detail: the format query uses the primary DirectDraw surface member `[DAT_00887308+0x1C]`, even when the Bink copy target later becomes `DAT_0088730C` HiddenSurface. This implies the hidden target is expected to be format-compatible with primary, and the copy format is retail DirectDraw format, not an engine-chosen RGBA8888 format.

## 4. INI Keys

None. No INI key participates in this explicit draw copy path. Active in YR: not applicable.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Dialog paint | parent `WM_PAINT` sends `0x4F0` to child `0x71A`; child dispatches vtable+`0x28` | prior `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` | Yes |
| Vtable thunk | `0x005C05F0` loads `handle+0x10` and jumps to `0x00433060` | static disassembly `0x005C05F0..0x005C05F3` | Yes |
| Bink open | destination surface and `object+0x08` flags are initialized before draw | static disassembly `0x004328D5..0x00432A44` | Yes |
| Copy | `_BinkCopyToBuffer@28` called only after destination lock succeeds | `0x004330C3..0x00433155` | Yes |
| Post-copy helper | `0x00433270` runs after copy or lock failure; when `object+0x20` is valid it blits helper-surface rects via surface vtable `+8` | `0x00433162..0x00433165`, `0x00433270..0x0043332D` | Conditional |

## 6. Current Rust Implementation Status

Current Rust does not model a DirectDraw destination surface or Bink's surface-format flags. `src/render/bink_movie.rs` decodes each frame into RGBA bytes with alpha `255`, uploads `width * 4` bytes per row to a GPU texture, and chooses MPEG/JPEG YUV conversion internally. `src/app_main_menu_shell_render.rs` then draws the movie texture first, with `MOVIE_DEPTH`, neutral tint, and alpha `1.0`; chrome/buttons/text are drawn afterward through GPU depth and pass-through sprite batches.

Rust therefore matches the broad "opaque untinted movie behind shell chrome" intent, but not the retail mechanism: retail Bink writes directly into a locked 16-bit DirectDraw-compatible surface using `_BinkDDSurfaceType` flags and `BINKCOPYALL` on explicit draw. Any final color-parity decision must account for 16-bit 565/555 quantization and Bink's own YUV-to-surface conversion, not only the current RGBA8888 conversion.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BinkMovie_ExplicitDraw_005C05F0` thunk | verified | `0x005C05F0..0x005C05F3` static disassembly | none |
| `BinkMovie_CopyStoredRectToPrimary @ 0x00433060` | verified | `0x00433060..0x00433171` static disassembly | live Ghidra decompile unavailable, but binary bytes disassembled |
| primary-target branch | verified | `0x00433063..0x004330FE` | runtime branch frequency not captured |
| non-primary/HiddenSurface branch | verified | `0x00433100..0x00433155`; open setup `0x004328D5..0x00432903` | none for standard static mechanism |
| `_BinkDDSurfaceType` storage | verified | `0x00432A35..0x00432A44`; import table `0x007E15A8` | numeric runtime return value depends on DirectDraw mode |
| current Rust render/color path | verified | `src/render/bink_movie.rs:80`, `:113`, `:146`; `src/app_main_menu_shell_render.rs:283`, `:426` | no code change made |
| full DirectDraw mode initialization | deferred | `FUN_00621040_RGB_BYTE_PERMUTATION_GHIDRA_REPORT.md` covers 565/555 globals | follow-up only if exact runtime format value is needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is vtable+0x28 doing work itself or thunking?` -> It is a thunk from generic handle to inner `BinkObject`, then jumps to `0x00433060`. (evidence: `0x005C05F0..0x005C05F3`)
- `[RESOLVED] OQ-2 - Which surface is locked by explicit draw?` -> `BinkObject+0x0C`; normally HiddenSurface in standard helper mode, primary in fallback/direct mode. (evidence: `0x0043306E`, `0x004328D5..0x00432903`)
- `[RESOLVED] OQ-3 - What are `_BinkCopyToBuffer` args?` -> handle, lock pointer, surface pitch, surface height, stored x, stored y, `object+0x08 | 0x80000000`. (evidence: `0x004330F4..0x00433155`)
- `[RESOLVED] OQ-4 - Does explicit draw use stored width/height as Bink copy args?` -> No; it uses destination surface height and Bink's flags. Stored w/h feed helper overlay math only. (evidence: `0x00433147..0x00433155`, `0x00433270..0x0043332D`)
- `[RESOLVED] OQ-5 - Where does the surface-format flag come from?` -> `_BinkDDSurfaceType([DAT_00887308+0x1C])`, stored at `BinkObject+0x08`. (evidence: `0x00432A35..0x00432A44`)
- `[RESOLVED] OQ-6 - Is `0x80000000` conditional in explicit draw?` -> No, both explicit branches always OR it before copying. (evidence: `0x004330EE`, `0x00433141`)
- `[RESOLVED] OQ-7 - Is this path active in standard YR?` -> Yes, standard main menu `0xE2` paint path sends `0x4F0` to `0x71A`. (evidence: prior `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-8 - What exact numeric `_BinkDDSurfaceType` value is returned on the user's runtime display mode?` (category: `needs-runtime-debugger`; reason: the value is computed by `binkw32.dll` from the live DirectDraw surface; next-step-if-pursued: trace `_BinkDDSurfaceType@4` return at `0x00432A3E` during shell startup)
- `[DEFERRED] OQ-9 - Does a retail screenshot show a 565/555 quantization difference versus Rust RGBA8888?` (category: `needs-runtime-debugger`; reason: needs capture/comparison, not static binary; next-step-if-pursued: capture first visible RA2TS frame and compare against Rust output quantized through detected DD format)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `WM_PAINT_Handler @ 0x00621E90` | common shell mode-1 paint | shell SHP stack | screen-sized BSurface -> AlternateSurface | 16-bpp shell surfaces | Yes | chrome/background |
| 2 | `MainMenuDialog0xE2_Proc @ 0x00531F60` -> `SendMessage(0x71A,0x4F0)` | parent `WM_PAINT` | none | child static `0x71A` | none | Yes | dispatch |
| 3 | `BinkMovie_ExplicitDraw_005C05F0` -> `0x00433060` | vtable+`0x28` | current decoded RA2TS frame | stored clipped x/y, plus client-origin only for primary branch | `_BinkDDSurfaceType` flags OR `0x80000000` | Yes | movie content |
| 4 | Win32 owner-draw controls | normal child draw order | PCX/text controls | control rects | shell PCX/text paths | Yes | buttons/text over movie |

Asset role matrix:

| Asset / surface | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `ra2ts_l.bik` / `ra2ts_s.bik` | Yes | Yes | Yes | content | no | no | no | no | prior RA2TS reports; explicit copy path |
| `DAT_0088730C` HiddenSurface | Yes in helper mode | Yes | Yes after downstream primary presentation | target surface | no | no | no | no | `0x004328DE`, `0x0043306E` |
| `DAT_00887308` primary surface | Yes | Conditional direct target | Yes | final DirectDraw target | no | no | no | no | `0x00433063..0x00433073` |
| `DAT_00887310` AlternateSurface | Yes | shell BSurface blit only | Yes for shell chrome | no | chrome target | no | no | no for Bink pixels | `SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Explicit draw calls `_BinkCopyToBuffer` with `object+0x08 | 0x80000000` and destination-surface pitch/height | `0x004330E7..0x00433155` | mismatch/unchecked: Rust uploads RGBA8888 with `bytes_per_row = width * 4` | `src/render/bink_movie.rs::frame_to_rgba`, `BinkMovieSurface::upload_rgba` | Future color-parity mode must model Bink's DirectDraw surface type and copy-all behavior, or prove RGBA output is pixel-equivalent after quantization | Test proposal: `bink_explicit_draw_uses_detected_surface_format_copyall_flags` | Do not treat `0x80000000` as alpha, tint, z-order, or "make transparent"; it is a Bink copy flag |
| Standard explicit draw target is `BinkObject+0x0C`, normally HiddenSurface, not the per-dialog BSurface or AlternateSurface | `0x0043306E`, `0x004328D5..0x00432903`; shell composition report | mismatch/unchecked: Rust draws movie as first GPU sprite in the same render pass as chrome | `src/app_main_menu_shell_render.rs::render_main_menu_shell` | Preserve retail draw order: movie content is a separate explicit-draw surface path before/under owner-draw buttons, not pixels mixed into shell `BSurface` chrome | Test proposal: `main_menu_ra2ts_movie_does_not_render_into_shell_alternate_surface_layer` | Do not use the parent shell `BSurface` as the Bink decode/copy target |
| Primary branch adds the main-window client-origin offset to stored x/y; HiddenSurface branch does not | `0x00433079..0x004330AE` vs `0x00433100..0x00433109` | unchecked: Rust uses layout movie rect directly in swapchain coordinates | `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs::movie_instance` | If a primary-surface fallback path is modeled, add client-origin only in that branch; normal HiddenSurface path uses stored rect as-is | Test proposal: `bink_explicit_draw_primary_branch_offsets_by_client_origin_only` | Do not add client-origin offsets to the HiddenSurface path |

### Negative Facts / Do Not Do

- Do not claim the explicit draw copies to `DAT_00887310` or the per-dialog `BSurface`; evidence shows Bink locks `BinkObject+0x0C`, while shell chrome separately blits to AlternateSurface.
- Do not pass stored width/height as `_BinkCopyToBuffer` width/height; the function passes destination surface height and uses x/y plus flags.
- Do not interpret `0x80000000` as alpha, opacity, tint, z, or transparency. The binary only ORs it into Bink copy flags, and Bink API references identify it as copy-all.
- Do not assume current Rust RGBA8888 is pixel-perfect by construction. Retail asks Bink to convert into the active DirectDraw surface format queried by `_BinkDDSurfaceType`.
- Do not apply primary-client-origin offset to every draw. That offset exists only in the `object+0x0C == DAT_00887308` branch.

### Stale Docs / Follow-up Docs

- `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md` Stage 7 replacement wording:
  "With `0x80000000` ORed in (`BinkMovie_ExplicitDraw_005C05F0` -> `0x00433060`), gamemd passes `object+0x08 | 0x80000000` as `_BinkCopyToBuffer` flags. The stored `object+0x08` value comes from `_BinkDDSurfaceType([DAT_00887308+0x1C])`; `0x80000000` is the Bink copy-all high bit, not an alpha/tint/upscale control."
- `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` Stage 2 replacement wording:
  "The explicit draw blits the previously decoded Bink frame into `BinkObject+0x0C`: normally `DAT_0088730C` HiddenSurface when the helper BSurface exists, or `DAT_00887308` primary surface in fallback/direct mode. It does not blit through the per-dialog BSurface or `DAT_00887310`."

## Sources

- Retail binary static disassembly from `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`:
  - `0x005C05F0` - explicit draw thunk
  - `0x00433060..0x00433171` - explicit copy path
  - `0x004328D5..0x00432903` - Bink destination surface assignment
  - `0x00432A35..0x00432A44` - `_BinkDDSurfaceType@4` setup
  - `0x00433270..0x0043332D`, `0x00433330..0x004333D7` - helper pre/post surface paths
- Import table parsed from retail `gamemd.exe`:
  - `0x007E15A8` = `binkw32.dll!_BinkDDSurfaceType@4`
  - `0x007E15B8` = `binkw32.dll!_BinkCopyToBuffer@28`
  - `0x007E14C4` = `USER32.dll!GetClientRect`
  - `0x007E14B8` = `USER32.dll!ClientToScreen`
- Prior reports referenced:
  - `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - `docs/research/FUN_00432AB0_BINK_CLIP_RECT_SETTER_GHIDRA_REPORT.md`
  - `docs/research/SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md`
  - `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - `docs/research/FUN_00621040_RGB_BYTE_PERMUTATION_GHIDRA_REPORT.md`
- Rust source inspected read-only:
  - `src/render/bink_movie.rs`
  - `src/app_main_menu_shell_render.rs`
- External API reference checked for symbolic meaning only:
  - Google Patents `US20110115824A1`, Bink support discussion: documents that `BINKCOPYALL` causes Bink copy functions to update blocks that would otherwise be skipped as unchanged: https://patents.google.com/patent/US20110115824A1/en
