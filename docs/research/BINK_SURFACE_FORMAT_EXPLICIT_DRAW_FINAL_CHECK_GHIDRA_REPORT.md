# Bink Surface Format Explicit Draw Final Check - Ghidra Report

**Address(es):** `0x00432750`, `0x005C05F0`, `0x00433060`, `0x00433270`, `0x00433330`, Bink vtable `0x007EE154`  
**Investigation mode:** focused `/re-swarm` retry slot 5  
**Date:** 2026-05-27  
**Status:** COMPLETE

## Working Notes

- **Target question:** Fresh-check, with live Ghidra MCP, the Bink surface-format setup and explicit draw path: `_BinkDDSurfaceType` setup, `0x005C05F0` thunk, `0x00433060` destination branch, `BinkObject+0x0C` assignment, `_BinkCopyToBuffer` arguments, `object+0x08 | 0x80000000`, and whether static binary evidence can identify the actual runtime 565/555/RGBA format.
- **Non-goals:** Cadence, `_BinkWait`, loop/restart, BIK/VQA lookup, Bink parser packet indexing, Bink audio, Rust edits, INI edits, or broad shell redraw.
- **Evidence needed to mark COMPLETE:** Live MCP decompile/disassembly for setup and explicit draw; vtable xref tying `0x005C05F0` to the Bink wrapper; enough setup evidence to explain `object+0x08` and `object+0x0C`; explicit statement of runtime-only surface-format limits.
- **Stop conditions:** Missing MCP function boundaries, mutating Ghidra requirement, or a branch that changes explicit draw target/copy flags outside this slice.

## Summary

Live Ghidra MCP confirms the prior static-disassembly result. `BinkMovie_ExplicitDraw_005C05F0` is only a thunk from the generic movie wrapper to the inner Bink object: it loads `ECX = [ECX+0x10]` and jumps to `0x00433060`. The real explicit draw function locks `BinkObject+0x0C`, queries destination pitch and height from that surface, and calls `_BinkCopyToBuffer@28` with `BinkObject+0x04`, the lock pointer, surface pitch, surface height, stored x/y, and `BinkObject+0x08 | 0x80000000`.

At open/init, `FUN_00432750` chooses `BinkObject+0x0C`: standard helper-surface mode uses `DAT_0088730C` HiddenSurface, while fallback/direct mode uses `DAT_00887308` primary. It separately stores `BinkObject+0x08 = _BinkDDSurfaceType([DAT_00887308+0x1C])`. Static binary evidence proves the queried object and flag flow, but not the exact numeric runtime Bink surface type, because that value is returned by `binkw32.dll` from the live DirectDraw surface.

## Load-Bearing Verified Facts

1. **Vtable dispatch reaches the explicit draw thunk. Active in YR: Yes.**  
   Evidence: live MCP `read_memory(0x007EE154, 64)` shows vtable entry `+0x28 = 0x005C05F0`; `get_xrefs_to(0x005C05F0)` reports data xref from `0x007EE17C`; `disassemble_function(0x005C05F0)` is `MOV ECX,[ECX+0x10]` then `JMP 0x00433060`.

2. **`0x00432750` assigns the draw target surface at `BinkObject+0x0C`. Active in YR: Yes.**  
   Evidence: live MCP decompile/disassembly of `0x00432750`: if the helper BSurface exists and its vtable pointer is nonzero, `0x004328DE..0x004328E5` writes `DAT_0088730C` to `[ESI+0x0C]`; otherwise `0x00432903..0x0043290D` writes `DAT_00887308` to `[ESI+0x0C]`.

3. **`0x00432750` stores Bink surface-format flags from the primary DirectDraw surface member. Active in YR: Yes.**  
   Evidence: live MCP decompile of `0x00432750` ends with `_BinkDDSurfaceType_4(*(undefined4 *)(DAT_00887308 + 0x1c)); *(undefined4 *)(param_1 + 8) = uVar4;`; disassembly `0x00432A35..0x00432A44` performs `MOV EAX,[0x00887308]`, `MOV EAX,[EAX+0x1C]`, `PUSH EAX`, `CALL [0x007E15A8]`, `MOV [ESI+0x8],EAX`. External-location listing identifies `_BinkDDSurfaceType@4` in `BINKW32.DLL`.

4. **Explicit draw branches only on `BinkObject+0x0C == DAT_00887308`; HiddenSurface branch does not add client-origin offsets. Active in YR: Yes/Conditional by branch.**  
   Evidence: live MCP decompile/disassembly of `0x00433060`: `0x00433063..0x00433073` loads `DAT_00887308`, loads `[ESI+0x0C]`, compares, and jumps to the non-primary branch. Primary branch calls `GetClientRect` and `ClientToScreen`, then adds `local_10.left/top` to `[ESI+0x10/+0x14]` at `0x0043309C..0x004330AE`. Non-primary branch uses `[ESI+0x10/+0x14]` directly at `0x00433100..0x00433109`.

5. **`_BinkCopyToBuffer@28` uses surface lock pointer, surface pitch, surface height, stored x/y, and `object+0x08 | 0x80000000`. Active in YR: Yes.**  
   Evidence: live MCP disassembly of `0x00433060`: both branches call surface vtable `+0x5C` for the lock (`0x004330C3`, `0x0043311C`), vtable `+0x74` for pitch (`0x004330D6`, `0x00433129`), vtable `+0x80` for destination height (`0x004330E1`, `0x00433134`), OR `[ESI+0x08]` with `0x80000000` (`0x004330EE`, `0x00433141`), then push arguments ending with `PUSH [ESI+0x04]` and call `[0x007E15B8]` at `0x00433155`. External-location listing identifies `_BinkCopyToBuffer@28` in `BINKW32.DLL`.

## Detailed Evidence

### `0x005C05F0` Thunk

Live MCP decompile:

```c
void BinkMovie_ExplicitDraw_005C05F0(void)
{
  BinkMovie_CopyStoredRectToPrimary();
  return;
}
```

Live MCP disassembly:

```asm
005c05f0: MOV ECX,dword ptr [ECX + 0x10]
005c05f3: JMP 0x00433060
```

Active in YR: Yes. The vtable at `0x007EE154` contains `0x005C05F0` at `+0x28`, and the owner-draw static path calls that vtable slot per prior main-menu paint research.

### Open/Setup Surface Selection

Live MCP decompile and disassembly of `0x00432750` show the target surface is selected after successful `_BinkOpen` and helper BSurface construction:

- Helper valid: `0x004328DE..0x004328E5` writes `DAT_0088730C` to `BinkObject+0x0C`. Active in YR: Yes for standard helper mode.
- Helper missing/invalid: `0x00432903..0x0043290D` writes `DAT_00887308` to `BinkObject+0x0C`. Active in YR: Conditional fallback/direct mode.

Immediately after clipping x/y/w/h into `BinkObject+0x10..0x1C`, the same function calls `_BinkDDSurfaceType([DAT_00887308+0x1C])` and stores the return value at `BinkObject+0x08`. Active in YR: Yes for every successful Bink object opened by this path.

### Explicit Draw Copy

Live MCP decompile of `0x00433060`:

```c
piVar6 = *(int **)(param_1 + 0xc);
if (piVar6 == DAT_00887308) {
  GetClientRect(g_hWnd,&local_10);
  ClientToScreen(g_hWnd,(LPPOINT)&local_10);
  unaff_EBX = *(int *)(param_1 + 0x10) + local_10.left;
  iVar5 = *(int *)(param_1 + 0x14) + local_10.top;
  ...
} else {
  iVar5 = *(int *)(param_1 + 0x14);
  ...
}
...
_BinkCopyToBuffer_28(*(undefined4 *)(param_1 + 4),
                     iVar2,uVar3,uVar4,unaff_EBX,iVar5,
                     uVar1 | 0x80000000);
```

The decompiler loses the non-primary x register name, but the listing closes it: non-primary branch loads `MOV EAX,[ESI+0x10]` and saves it to stack at `0x00433100..0x00433109`; that saved value is later pushed as the x argument at `0x0043314B..0x00433150`.

Active in YR: Yes for the function and copy mechanics; primary-vs-hidden branch is conditional on the target chosen during setup.

### Runtime Surface Format Limit

Static binary can prove:

- `object+0x08` comes from `_BinkDDSurfaceType([DAT_00887308+0x1C])`. Active in YR: Yes.
- `object+0x08` is ORed with `0x80000000` for explicit draw. Active in YR: Yes.
- The copy target is a DirectDraw-compatible surface selected by `BinkObject+0x0C`. Active in YR: Yes/Conditional by branch.

Static binary cannot determine the exact numeric `_BinkDDSurfaceType` return for the user's run. That value is computed by `binkw32.dll` from the live DirectDraw surface object at runtime. Existing shell research indicates native UI surfaces are 16-bit DirectDraw-format and use RGB loss/shift globals, but this slot did not attach to a running game frame or capture `_BinkDDSurfaceType` return. The exact 565/555/RGBA answer therefore remains runtime-only for this slice.

## Current Rust Delta

Current Rust in `src/render/bink_movie.rs` decodes Bink frames into RGBA bytes, writes `width * 4` rows to a `wgpu` texture, and uses alpha `255`. Current Rust in `src/app_main_menu_shell_render.rs` draws that movie as a GPU sprite at `MOVIE_DEPTH`, then draws chrome/buttons/text in the same render pass. That is not the verified retail mechanism: gamemd asks Bink to copy into a locked DirectDraw surface using `_BinkDDSurfaceType` flags and a copy-all high bit.

## Implementation Handoff

- Explicit draw uses `_BinkDDSurfaceType([DAT_00887308+0x1C])` stored at `object+0x08`, then passes `object+0x08 | 0x80000000` to `_BinkCopyToBuffer` -> live MCP evidence `0x00432A35..0x00432A44`, `0x004330EE`, `0x00433141`, `0x00433155` -> current Rust delta: RGBA8888 conversion/upload with `bytes_per_row = width * 4` -> affected surface `src/render/bink_movie.rs::frame_to_rgba` and `BinkMovieSurface::upload_rgba` -> acceptance test `bink_explicit_draw_uses_detected_surface_format_copyall_flags` -> do not treat RGBA8888 output as pixel-perfect without proving equivalence through the live DirectDraw/Bink surface type.

- Standard explicit draw target is `BinkObject+0x0C`, normally `DAT_0088730C` HiddenSurface, not the per-dialog BSurface or `DAT_00887310` -> live MCP evidence `0x004328DE..0x004328E5`, fallback `0x00432903..0x0043290D`, explicit read `0x0043306E` -> current Rust delta: movie is a GPU sprite in the shell render pass -> affected surface `src/app_main_menu_shell_render.rs::render_main_menu_shell` -> acceptance test `main_menu_ra2ts_movie_does_not_render_into_shell_alternate_surface_layer` -> do not route Bink pixels through the shell alternate surface or parent dialog BSurface in docs/implementation.

- Primary branch adds main-window client-origin offsets, HiddenSurface branch uses stored x/y directly -> live MCP evidence primary `0x00433079..0x004330AE`, non-primary `0x00433100..0x00433109` -> current Rust delta: layout movie rect is used directly for all cases -> affected surface `src/ui/main_menu_shell/layout.rs` and `src/app_main_menu_shell_render.rs::movie_instance` -> acceptance test `bink_explicit_draw_primary_branch_offsets_by_client_origin_only` -> do not apply client-origin offsets to the normal HiddenSurface path.

## Negative Facts / Do Not Do

- Do not claim explicit draw copies to `DAT_00887310` or a per-dialog BSurface; live MCP shows it locks `BinkObject+0x0C`.
- Do not pass stored movie width/height as `_BinkCopyToBuffer` width/height; live MCP shows destination surface height from vtable `+0x80` and no width argument in the call.
- Do not interpret `0x80000000` as alpha, tint, z-order, scaling, or transparency; gamemd ORs it into the Bink copy flags.
- Do not apply the primary-surface client-origin offset to HiddenSurface draws.
- Do not conclude the exact runtime DDSurfaceType numeric value from static gamemd.exe alone; it is returned by `binkw32.dll` for the live DirectDraw surface.

## Remaining Uncertainty

- Exact numeric `_BinkDDSurfaceType` return on the user's runtime display mode. Reason: value is produced by `binkw32.dll` from the live DirectDraw surface object; static gamemd.exe only proves the call and stored/used value.
- Exact final monitor RGB/pixel delta between Bink's DirectDraw-format copy and Rust RGBA8888/WGPU presentation. Reason: requires runtime capture or a proved emulation of the live DDSurfaceType conversion.

## Stale-Doc Replacement Wording

`docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md` Stage 7 replacement:

> With `0x80000000` ORed in (`BinkMovie_ExplicitDraw_005C05F0` -> `0x00433060`), gamemd passes `object+0x08 | 0x80000000` as `_BinkCopyToBuffer` flags. The stored `object+0x08` value comes from `_BinkDDSurfaceType([DAT_00887308+0x1C])`; static gamemd.exe proves this flag flow but not the exact runtime numeric DDSurfaceType value returned by `binkw32.dll`.

`docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` Stage 2 replacement:

> The explicit draw blits the previously decoded Bink frame into `BinkObject+0x0C`: normally `DAT_0088730C` HiddenSurface when the helper BSurface exists, or `DAT_00887308` primary surface in fallback/direct mode. It does not blit through the per-dialog BSurface or `DAT_00887310`. Only the primary-surface branch adds the main-window client-origin offset.

## Sources

- Live Ghidra MCP `gamemd.exe` program info: image base `0x00400000`, executable `<ra2-install>/gamemd.exe`.
- Live MCP `read_memory(0x007EE154, 64)`: Bink wrapper vtable, including `+0x28 = 0x005C05F0`.
- Live MCP `decompile_function` / `disassemble_function`: `0x005C05F0`, `0x00432750`, `0x00433060`, `0x00433270`, `0x00433330`.
- Live MCP `list_external_locations`: `_BinkDDSurfaceType@4`, `_BinkCopyToBuffer@28`, `_BinkOpen@8`, and related Bink imports in `BINKW32.DLL`.
- Rust read-only scan: `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`.
