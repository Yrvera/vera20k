# Sidebar Blit To Screen Dirty Rects - Ghidra Report

Status: COMPLETE
Date: 2026-05-27
Scope: `SidebarClass::BlitToScreen @ 0x006A70E0` and immediate caller/consumer evidence only.

## Working Notes Gate

- Target question: Which sidebar-surface-local rectangles does `SidebarClass::BlitToScreen` copy to the screen, under which dirty/full-redraw predicates, and does Soviet sidebar/radar have any special blit rectangle path?
- Non-goals: Do not re-investigate sidebar draw internals, SHP selectors, minimap content, gadget geometry, or radar animation lifecycle except where those directly feed `BlitToScreen` inputs.
- Evidence needed to mark COMPLETE: read-only Ghidra decompile of `0x006A70E0`, immediate caller evidence from `SidebarClass::Draw @ 0x006A6C30`, xref/callsite evidence, byte/disassembly ranges for branch predicates and blit argument construction, and Rust-facing handoff scenarios.
- Stop conditions: stop if Ghidra MCP is unavailable, if function boundaries are missing and cannot be inspected read-only, or if proof requires expanding into unrelated sidebar/radar draw internals.

## Evidence Log

- Ghidra MCP read-only: `decompile_function 0x006A70E0`, `decompile_function 0x006A6C30`, `get_function_xrefs 0x006A70E0`.
- Ghidra MCP read-only: `disassemble_bytes 0x006A70E0..0x006A72FF`, `disassemble_bytes 0x007776E0..0x0077775F`.
- Local read-only retail `gamemd.exe` byte disassembly with Capstone for exact instruction ranges `0x006A70E0..0x006A7495`, `0x006A6FE0..0x006A70B3`, and `0x007776E0..0x00777730`. This was used only to render mnemonics from the same retail bytes because the MCP disassembly call returned success metadata without instruction text.
- Sibling evidence consulted, not treated as sole source: `DAT_00A8EB7C_FLAG_IDENTITY_GHIDRA_REPORT.md`, `SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`.

## Verified Findings

### 1. `BlitToScreen` is active in standard YR and is reached from the normal sidebar draw path.

Active in YR: Yes.

Evidence: `get_function_xrefs 0x006A70E0` returns a call from `0x006A70AE` inside `SidebarClass::Draw @ 0x006A6C30`. The caller decompile computes the argument as `(DAT_00b0b518 != 0 || this+0x53A7 != 0)`, then calls `SidebarClass__BlitToScreen(this, byte)`, and afterwards clears `this+0x53A6` and `this+0x53A7`. Local byte disassembly confirms `0x006A7096..0x006A70AE`: compare folded dirty byte, compare `this+0x53A7`, push `0` or `1`, `mov ecx, edi`, `call 0x006A70E0`.

There is a second xref from `0x00777723`. Ghidra has no function boundary at that exact address, but read-only byte disassembly of `0x007776E0..0x00777730` shows `push 1; mov ecx, 0x87f7e8; call 0x006A70E0` inside a display/window loop gated by `DAT_00A8ED5C == 1`. This is not the normal `SidebarClass::Draw` path, but it proves an immediate display consumer can force the blit.

### 2. The function has hard early-outs before any screen copy.

Active in YR: Yes.

Evidence: `SidebarClass::BlitToScreen @ 0x006A70E0` decompile and byte range `0x006A70EB..0x006A7109` show three guards before `GetClientRect`: `this+0x53A5 != 0`, `g_GameActive != 0` (`DAT_00A8E9A0`), and `DAT_00A8ED5C != 0`. If any guard fails, control jumps to `0x006A7488`, and the epilogue at `0x006A748A` writes `DAT_00B0B518 = 0` before returning. No `DAT_00B0B519` dirty-for-display write occurs on those early-outs.

Player/Rust implication: a sidebar draw can update off-screen state and still skip screen copy if the sidebar is inactive or display/game gates are down; the final dirty byte is cleared on exit.

### 3. Destination coordinates are based on the window client origin plus a single sidebar x-origin transform.

Active in YR: Yes.

Evidence: `0x006A710F..0x006A713D` calls `GetClientRect(g_hWnd, local_rect)`, copies the client left/top into a `POINT`, then calls `ClientToScreen(g_hWnd, point)`. All later destination rect builders use `point.x` and `point.y` as the base. The x base additionally uses `(-(DAT_00A8EB7C != 0) & g_RadarViewportWidth)` at `0x006A71CA..0x006A71F3`, `0x006A72B0..0x006A72D9`, `0x006A731B..0x006A735F`, `0x006A73F3..0x006A7409`, and `0x006A7433..0x006A7465`.

Active-in-YR note: `DAT_00A8EB7C` is proven by sibling report `DAT_00A8EB7C_FLAG_IDENTITY_GHIDRA_REPORT.md` as `OptionsClass::bSidebarOnRight`, hard-set to `1` in normal YR. Therefore standard YR adds `g_RadarViewportWidth` to `ClientToScreen`'s x origin. The function has no Soviet/Yuri side check and no radar-asset check.

### 4. If there is no folded dirty flag and no forced argument, matching surface dirty globals produce either no copy or only a top-strip copy.

Active in YR: Yes, conditional.

Evidence: `0x006A7143..0x006A71AB` enters this fast path only when `DAT_00B0B518 == 0`, function argument `param_1 == 0`, and the four cached rect globals `DAT_008809F4/F8/FC/A00` match `DAT_00B07DC8/CC/D0/D4`. If `this+0x53A8 == 0`, the function returns after clearing `DAT_00B0B518`; no `DAT_00B0B519` is set. If `this+0x53A8 != 0`, `0x006A71B7..0x006A722F` first calls display-chain vtable `+0x3C(g_SidebarSurface, 1)`, then blits from `g_SidebarSurface` to `DAT_00887308` with source rect `{x=0, y=0, w=g_SidebarTopClip, h=0x10}` and destination rect `{x=client_x + right-sidebar-offset, y=client_y, w=g_SidebarTopClip, h=0x10}`. It then clears `this+0x53A8`, sets `DAT_00B0B519 = 1`, clears `DAT_00B0B518`, and returns.

The rectangle ABI is treated here as x/y/width/height: the destination only adds screen origin to the first two fields; width and height are copied as `g_SidebarTopClip` and `0x10`, not converted to right/bottom coordinates.

### 5. When a dirty/full-redraw copy is needed, `BlitToScreen` can split the copy into top strip plus lower body, or copy the full current surface rectangle.

Active in YR: Yes, conditional.

Evidence: If the early no-op fast path does not return, `0x006A723A` sets `DAT_00B0B518 = 1` when the caller argument or prior dirty flag requires it, then `0x006A7241..0x006A7252` marks the sidebar surface through display-chain vtable `+0x3C(g_SidebarSurface, 1)`.

When the cached/current dirty globals still match and `param_1 == 0`, `0x006A729E..0x006A739D` takes a split-copy path. If `this+0x53A8 != 0`, it first performs the same top-strip copy `{0,0,g_SidebarTopClip,0x10}` and clears `this+0x53A8`. It then copies the lower body from source rect `{x=0, y=g_SidebarWidth, w=g_SidebarTopClip, h=DAT_00886F9C}` to destination `{x=client_x + right-sidebar-offset, y=client_y + g_SidebarWidth, w=g_SidebarTopClip, h=surface_height - g_SidebarWidth}`. The surface height is read via `g_SidebarSurface` vtable `+0x80` at `0x006A7361..0x006A7369`.

If the split-copy condition is not met and `DAT_00B0B518 != 0`, `0x006A73B1..0x006A742E` asks `g_SidebarSurface` vtable `+0x78` for the current dirty/full rect, then blits that rect from sidebar surface to the screen with destination x/y adjusted by the shared origin and width/height left unchanged.

### 6. If the folded dirty flag is zero but the sidebar-surface dirty rect globals changed, the function performs a partial rect copy instead of a full-screen sidebar copy.

Active in YR: Yes, conditional.

Evidence: The branch at `0x006A7433..0x006A747E` runs when the cached/current dirty globals differ but `DAT_00B0B518 == 0`. It passes `&DAT_008809F4` as the source rect and builds a destination rect by adding the shared screen origin to `DAT_008809F4`/`DAT_008809F8`, leaving `DAT_008809FC`/`DAT_00880A00` as width/height. The call is `DAT_00887308->vtable+0x08(dest_rect, g_SidebarSurface, &DAT_008809F4, 0, 1)`.

This is the exact partial dirty-rectangle path that current Rust does not appear to model: it is neither every-layer redraw nor unconditional full-sidebar copy.

### 7. Successful screen-copy paths always set the post-blit display flag and clear the folded dirty flag.

Active in YR: Yes.

Evidence: All copying branches converge at `0x006A7481`, which writes `DAT_00B0B519 = 1`, then the epilogue writes `DAT_00B0B518 = 0` at `0x006A748A`. Early-outs skip `DAT_00B0B519` but still clear `DAT_00B0B518`. Sibling `RADAR_CHROME_COMPOSITING.md` identifies `DAT_00B0B519` as consumed later by a render/display-frame stage to mark the sidebar surface for final compositing; that consumer was not expanded in this narrow slot.

### 8. No Soviet-specific blit rectangle behavior exists in this function.

Active in YR: Yes.

Evidence: `0x006A70E0` reads sidebar-active/draw flags, game/display globals, client origin, cached surface dirty rect globals, `DAT_00A8EB7C`, `g_RadarViewportWidth`, `g_SidebarWidth`, `g_SidebarTopClip`, `DAT_00886F9C`, `g_SidebarSurface`, and `DAT_00887308`. It does not read side index, house side, theater side, `SSCR*`/`MPSSCRN*` globals, or radar/Soviet asset pointers. The only side-like branch is `DAT_00A8EB7C`, which is the right-sidebar option flag, not Soviet.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `BlitToScreen` applies one shared screen transform to sidebar-surface-local rects: destination x/y = client origin + optional `g_RadarViewportWidth` right-sidebar offset + source x/y, width/height unchanged. | Replace or wrap independent screen-space sidebar batches with a sidebar-surface-local composition model, or prove all batches share exactly the same transform and rect semantics. | `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, `src/app_sidebar_build.rs`, `src/sidebar/mod.rs` | Resize/toggle viewport dimensions and verify chrome, cameos, gclock, text, power, radar, and partial dirty regions all move by the same origin with no per-layer x drift. | `test_sidebar_blit_uses_single_surface_origin_for_all_layers` | HIGH; every visible sidebar frame can drift by pixels if layer transforms diverge. |
| Native has partial dirty-rectangle copy paths: top strip `{0,0,g_SidebarTopClip,0x10}`, lower body from y `g_SidebarWidth`, full current surface dirty rect via vtable `+0x78`, and changed cached rect via `DAT_008809F4..A00`. | Add a dirty-rect/cache abstraction for the sidebar surface, or explicitly document/test why a full redraw implementation cannot affect pixel output before claiming parity. | Future sidebar-surface cache, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | Mark only a gadget/tooltip/radar region dirty and assert Rust copies or redraws the same sidebar-local rectangle set as native instead of repainting unrelated side-piece areas. | `test_sidebar_dirty_rect_copy_modes_match_native_blit` | HIGH for transition/flicker/overlap parity; medium for steady-state screenshots. |
| `g_SidebarWidth=158` and `g_SidebarTopClip=168` are both consumed by `BlitToScreen`: body copy starts at y `158`, top strip width is `168`. | Current Rust exposes a single `SIDEBAR_WIDTH=168`; implementation must distinguish body/top split in blit/layout code, not only in SHP atlas dimensions. | `src/sidebar/mod.rs`, `src/sidebar/layout_spec.rs`, sidebar blit/cache model | At 800x600 standard YR, top strip copy is 168x16 from source y 0, while body copy begins at sidebar-surface y 158; no code should use 168 as the body start. | `test_sidebar_blit_distinguishes_158_body_y_from_168_top_clip` | HIGH; conflating the constants shifts body/radar/power copy boundaries. |

## Negative Facts / Do Not Do

- Do not implement `BlitToScreen` as an unconditional full-sidebar copy every draw. Evidence: `0x006A7143..0x006A71AB` can no-op, `0x006A71B7..0x006A722F` can copy only the 168x16 top strip, and `0x006A7433..0x006A747E` can copy a partial cached dirty rect.
- Do not model the copied rect fields as left/top/right/bottom. Evidence: each destination builder adds screen origin only to the first two fields and leaves the third/fourth as `g_SidebarTopClip`, `0x10`, `DAT_008809FC`, or `DAT_00880A00`.
- Do not add a Soviet-specific blit branch. Evidence: `0x006A70E0` has no side/house/radar asset reads; Soviet radar differences are upstream composition inputs, not final sidebar-surface-to-screen copy behavior.
- Do not use `screen_width - 168` as the direct blit origin inside this function. Evidence: standard YR uses `ClientToScreen(g_hWnd).x + (DAT_00A8EB7C ? g_RadarViewportWidth : 0)`, and `DAT_00A8EB7C` is the right-sidebar option flag.
- Do not assume `DAT_00B0B518` survives a skipped blit for later. Evidence: the epilogue writes `DAT_00B0B518 = 0` on both early-outs and copied paths.

## Rust Reconnaissance

- `src/sidebar/mod.rs` currently exposes one `SIDEBAR_WIDTH = 168.0`, but native `BlitToScreen` consumes both `g_SidebarWidth = 158` and `g_SidebarTopClip = 168`.
- `src/app_render/build_instances.rs` builds separate `sidebar`, `chrome`, `cameo`, `gclock`, `cameo_overlay`, `text`, `minimap`, `viewport_rect`, and `radar_anim` batches; there is no obvious sidebar-surface dirty-rect accumulator.
- `src/app_render/draw_passes.rs` draws `minimap`, `viewport_rect`, `sidebar`, `sidebar_chrome`, `radar_anim`, `sidebar_cameo`, `sidebar_gclock`, `sidebar_cameo_overlay`, and `sidebar_text` as independent UI passes rather than one blit-equivalent surface copy.
- `src/app_sidebar_text.rs` uses egui overlay text, which is outside the native sidebar-surface blit model.

## Remaining Uncertainty

- The exact identity of `g_SidebarSurface` vtable `+0x78` and `+0x80` methods was inferred from use as current dirty/full rect and surface height/extent; their implementations were not decompiled in this slot.
- The later consumer of `DAT_00B0B519` was not re-investigated beyond sibling-doc evidence because this slot was restricted to `BlitToScreen` and immediate callers/consumers.
- The second callsite at `0x00777723` has no Ghidra function boundary in the current database; byte disassembly proves the call and forced argument, but the owning function name/lifecycle remains unresolved.
- What specifically sets `this+0x53A8` is outside this scope; this report only proves its effect inside `BlitToScreen`.

## Stale Doc Replacement Wording

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: replace the short `Blit sidebar surface to screen` function-table wording with: `SidebarClass::BlitToScreen @ 0x006A70E0 copies sidebar-surface-local x/y/width/height rectangles to DAT_00887308 using ClientToScreen(g_hWnd) plus the right-sidebar viewport offset. It can no-op, copy only the 168x16 top strip, copy the lower body from y=158, copy the surface current dirty rect, or copy the cached partial dirty rect; it has no Soviet-specific branch.`
- `docs/research/RADAR_CHROME_COMPOSITING.md`: replace `For the sidebar window, screen_rect = (screen_width - 168, 0, 168, screen_height)` with: `For `SidebarClass::BlitToScreen`, destination x is ClientToScreen(g_hWnd).x plus `g_RadarViewportWidth` when the right-sidebar option flag is set; copied rectangles remain x/y/width/height from sidebar-surface space. The common full-window intuition `screen_width - 168` is an approximation and does not capture the native dirty-rect branches.`
