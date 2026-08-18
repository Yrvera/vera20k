# Surface DrawLine ABuffer/Z-Test Pixel Contract - Ghidra Research Report

**Address(es):** `0x004BFD30` primary function; selected-building callers through `0x006DBB60`, `0x006F5EF0`, `0x006F60D0`, `0x006F5190`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` per-pixel contract as reached by selected building bracket line callers: clipping order, endpoint inclusion, dominant-axis selection/stepping, Z-test, A-buffer suppression/modulation, and Z-write behavior.  
**Non-Scope:** full non-bracket `DrawLine3D` caller census, runtime screenshots, exact stock pixel overlap matrix against shroud edge frames, or Rust implementation changes.  
**Confidence:** High for the primary surface predicate, branch selection, endpoint inclusion, A-buffer formula, Z-write flag, and selected-building activity; Medium for decompiler-local variable names and exact pre-raster depth interpolation naming because the prototype is not recovered cleanly.  
**Active in YR:** Yes for selected-building bracket lines. Fog A-buffer blending is Conditional and off by default in standard YR.

## 1. Overview

Selected building bracket lines are not drawn as a depthless UI overlay in `gamemd.exe`. The bracket path projects 3D endpoints through `Tactical::DrawLine3D @ 0x006DBB60`, then reaches the primary surface vtable slot `+0x34`, resolved in prior vtable evidence to `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`.

The surface routine clips the line to the active clip rectangle, rasterizes one dominant-axis pixel per loop iteration, samples both the Z-buffer and A-buffer at the candidate pixel, writes color only when the strict Z-test and nonzero A-buffer test pass, and writes Z only when the caller's final flag byte is nonzero. Selected building bracket callers pass that final flag as `0`, so they Z-test but do not Z-write.

## 2. Class Layout / Key Offsets

| Object / global | Offset / address | Meaning in this slice | Active in YR |
|---|---:|---|---|
| Surface vtable slot | `+0x34` | Line drawer reached from `Tactical::DrawLine3D`; prior report resolves DSurface slot to `0x004BFD30`. | Yes; selected-building bracket callers go through `g_Tactical->vtable+0x60`, then the primary surface line slot. |
| Surface vtable slot | `+0x78` | Clip-rect provider used before rasterization in `0x004BFD30`. | Yes; first operation in the surface routine. |
| Surface vtable slot | `+0x5C` | Destination pixel pointer lookup for clipped start coordinate. | Yes; null pointer suppresses drawing. |
| Surface vtable slots | `+0x70`, `+0x60`, `+0x74` | Surface lock/unlock and scanline pitch/stride helpers around the raster loops. | Yes; called by `0x004BFD30`. |
| `g_ZBuffer` | global | 16-bit circular depth surface; each candidate line pixel tests against it. | Yes; if null, the checked routine returns without drawing. |
| `g_ABuffer` | global | 16-bit circular A-buffer; low-byte values drive suppression/modulation. | Yes; sampled for every candidate line pixel. |
| CircBuf `+0x18`, `+0x1C`, `+0x20`, `+0x28` | offsets | buffer start/end/wrap span/pitch used when line stepping crosses circular bounds. | Yes; Z and A pointers wrap in all branch paths. |
| `TechnoClass+0x83` | byte | selected-state gate before building brackets. | Yes; checked by `DrawBehind @ 0x006F60D0` and `DrawExtras @ 0x006F5190`. |

## 3. Core Logic

### 3.1 Clipping and endpoint normalization

1. `0x004BFD30` obtains a clip rect through surface vtable `+0x78`, copies `x,y,width,height`, and only proceeds when width or height is nonzero. Evidence: decompile of `0x004BFD30`. Active in YR: Yes.
2. The two input screen endpoints are offset by the clip rect origin before clipping. Evidence: `0x004BFD30` adds clip `x/y` to both endpoint pairs before calling `FUN_007BC2B0`. Active in YR: Yes.
3. If endpoint B has smaller screen X than endpoint A, the routine swaps endpoints and swaps the endpoint depth values before clipping. This makes the raster run left-to-right for non-vertical lines. Evidence: `0x004BFD30` branch before `FUN_007BC2B0`. Active in YR: Yes.
4. `FUN_007BC2B0 @ 0x007BC2B0` is a Cohen-Sutherland-style clipper. It treats left/top as inclusive and clips right/bottom to `rect.x + rect.width - 1` and `rect.y + rect.height - 1`. Shared outside outcodes reject the line. Evidence: decompile of `0x007BC2B0`; prior address spans `0x007BC447`, `0x007BC454`. Active in YR: Yes.
5. After clipping, if the clipped endpoints collapse to the same screen coordinate, the surface routine returns success without writing a pixel. Evidence: zero-length bypass in `0x004BFD30` before the dominant loops. Active in YR: Yes.

### 3.2 Endpoint inclusion

All three raster branches are start-inclusive and end-exclusive after clipping:

| Branch | Loop count | Endpoint behavior | Evidence | Active in YR |
|---|---:|---|---|---|
| Z-dominant | `abs(dz)` iterations | writes current pixel before stepping; final endpoint excluded | `0x004BFD30`, branch body previously bracketed at `0x004C0171..0x004C0358` | Yes in surface routine; not reached by stock selected-building bracket segments per prior depth-dominant report. |
| X-dominant | `dx` iterations | writes current pixel, then advances X every iteration; final endpoint excluded | `0x004BFD30`, branch body previously bracketed at `0x004C0373..0x004C0554` | Yes; reached by constant-Z building bracket stubs. |
| Y-dominant | `abs(dy)` iterations | writes current pixel, then advances Y every iteration; final endpoint excluded | `0x004BFD30`, branch body previously bracketed at `0x004C0557..0x004C0738` | Yes; reached by vertical selected-building bracket stubs. |

Material edge case: a line whose clipped dominant delta is zero writes no pixel. Active in YR: Yes, because this is inside the selected bracket surface drawer.

### 3.3 Dominant-axis choice and stepping

After clipping, the routine computes:

```text
dx = clipped_end_x - clipped_start_x  // nonnegative after X normalization
dy = abs(clipped_start_y - clipped_end_y)
dz = abs(clipped_end_depth - clipped_start_depth)
```

Branch selection is strict:

```text
if dx < dz && dy < dz: z-dominant
else if dy < dx:       x-dominant
else:                  y-dominant
```

Tie behavior matters: depth does not win ties; X wins only when `dx > dy`; equal X/Y ties and Y/depth ties fall to the Y-dominant path. Evidence: `0x004BFD30` decompile, matching prior `BUILDING_BRACKET_DEPTH_DOMINANT_RASTER_REACHABILITY_GHIDRA_REPORT.md`. Active in YR: Yes.

Stepping is integer Bresenham-style with separate error accumulators for the two non-dominant axes. Each branch writes/samples the current candidate pixel before applying step updates. X movement advances the destination pixel pointer by 2 bytes and advances both Z/A pointers by one 16-bit entry; Y movement advances the destination pointer by the surface scanline stride and advances both circular buffers by their pitch, with wrap handling. Z/depth movement changes the candidate line depth by the endpoint-depth sign. Evidence: `0x004BFD30` pointer/error update blocks in all three branches. Active in YR: Yes.

### 3.4 Per-pixel write predicate

For every candidate pixel in all three branches, the material predicate is:

```text
abuf = *ABufferPixel
if ((uint16)line_depth < *ZBufferPixel && abuf != 0) {
    framebuffer_pixel = color_or_modulated_color
    if ((byte)z_write_flag != 0) {
        *ZBufferPixel = (uint16)line_depth
    }
}
```

The Z-test is strictly less-than, not less-or-equal. A candidate equal to the current Z-buffer value does not draw and does not Z-write. Evidence: `0x004BFD30` decompile; prior branch address spans `0x004C024F`, `0x004C043A`, `0x004C062B` for guarded Z writes. Active in YR: Yes.

If `g_ZBuffer` is null, or the surface destination pointer lookup returns null, the checked line routine does not enter the pixel loops and returns `0`. Evidence: `0x004BFD30` gates the raster on `g_ZBuffer != 0` and destination pointer non-null. Active in YR: Yes as defensive runtime behavior; normal tactical rendering has these buffers.

### 3.5 A-buffer suppression and modulation

The A-buffer sample is a 16-bit load, but the active shroud/fog writers store byte-scale values into it. The line routine treats the loaded value numerically:

| A-buffer value | Pixel result | Evidence | Active in YR |
|---:|---|---|---|
| `0` | suppresses the pixel entirely, even if Z-test passes | `abuf != 0` predicate in all three branches of `0x004BFD30` | Yes; shroud writes can produce zero. |
| `0x7F` | writes the original 16-bit source color with no channel modulation | explicit `abuf == 0x7F` fast path in `0x004BFD30` | Yes; dirty A-buffer reset uses neutral `0x7F` per prior `0x00411330` evidence. |
| nonzero and not `0x7F` | writes a per-channel dimmed/modulated 16-bit color | formula in all three branches of `0x004BFD30` | Yes; shroud edge/fog values can hit this path. |

Modulation formula, in display-channel terms:

```text
expanded_r = (uint8)(src_color >> g_DD_RShift) << g_DD_RLoss
expanded_g = (uint8)(src_color >> g_DD_GShift) << g_DD_GLoss
expanded_b = (uint8)(src_color >> g_DD_BShift) << g_DD_BLoss

out_r = (((abuf * expanded_r) >> 7) >> g_DD_RLoss) << g_DD_RShift
out_g = (((abuf * expanded_g) >> 7) >> g_DD_GLoss) << g_DD_GShift
out_b = (((abuf * expanded_b) >> 7) >> g_DD_BLoss) << g_DD_BShift
out = out_r | out_g | out_b
```

There is no extra clamp visible in this routine beyond integer truncation and the target channel shifts. Evidence: `0x004BFD30` per-branch formulas. Active in YR: Yes.

### 3.6 Selected-building bracket Z-write flag

`TechnoClass::DrawBracketCorner @ 0x006F5EF0` calls `g_Tactical->vtable+0x60` with final argument `0` for both generated 25% stubs. `TechnoClass::DrawExtras @ 0x006F5190` also uses final argument `0` for the direct single-stub calls. Evidence: decompile of `0x006F5EF0` and `0x006F5190`, with prior assembly examples `0x006F5FCE`, `0x006F5FED`, `0x006F5762`, `0x006F58B1`, `0x006F59D3`. Active in YR: Yes.

Therefore selected-building bracket pixels can be hidden by existing Z-buffer contents, but successful bracket pixels do not update the Z-buffer and cannot occlude later Z-tested pixels by depth write. Active in YR: Yes; this follows directly from caller flag `0` plus the surface guarded Z-write predicate.

## 4. INI Keys

No INI key controls `0x004BFD30` itself. Relevant A-buffer activity gate:

| File / section / key | Value | Effect in this slice | Active in YR |
|---|---|---|---|
| `ini/rulesmd.ini [General] FogOfWar` | `no` at line 205 | Standard YR leaves fog-of-war blending path off. | Conditional: fog A-buffer blending needs `SpecialFlags & 0x1000`; default No. |
| `ini/rulesmd.ini [MultiplayerDialogSettings] FogOfWar` | `no` at line 3040 | Multiplayer default also disables fog-of-war. | Conditional: default No. |
| `ini/rules.ini [General] FogOfWar` | `no` at line 172 | Base fallback agrees with YR. | Conditional: default No. |

Standard shroud A-buffer writes are separate from fog-of-war and remain active in normal YR tactical rendering. Evidence: `Shroud_fog_edge_rendering @ 0x004801F0` always calls `ShroudEdge_BlitToABuffer`; fog blend is gated by `(*g_ScenarioClass_Instance & 0x1000) != 0`. Active in YR: Yes for shroud, Conditional for fog.

## 5. Integration Points

Selected building path:

```text
Tactical object draw phases
  -> TechnoClass::DrawBehind @ 0x006F60D0
       gate: WhatAmI()==6 and selected byte +0x83
       -> TechnoClass::DrawBracketCorner @ 0x006F5EF0
  -> TechnoClass::DrawExtras @ 0x006F5190
       gate: selected byte +0x83 and WhatAmI()==6
       -> DrawBracketCorner and direct DrawLine3D single-stub calls
  -> Tactical::DrawLine3D @ 0x006DBB60
  -> primary surface vtable +0x34
  -> Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30
```

Active in YR: Yes. The checked gates are selected state and building `WhatAmI()==6`, not TS-only feature flags. Building bracket visibility can still be blocked earlier by shroud/object visibility gates before a line reaches the surface routine; this report only claims the final surface contract once called.

Shroud/fog A-buffer path:

```text
Shroud_fog_edge_rendering @ 0x004801F0
  -> ShroudEdge_BlitToABuffer @ 0x0047EFE0      // active standard shroud
  -> FogEdge_BlendToABuffer @ 0x0047F250        // gated by SpecialFlags & 0x1000
```

Active in YR: Yes for standard shroud; Conditional for fog because `FogOfWar=no` in stock YR rules.

## 6. Current Rust Implementation Status

Source read only:

| Rust area | Current behavior | Gap versus binary contract |
|---|---|---|
| `src/app_selection_brackets.rs:152` | Emits 1x1 sprite instances with integer start-inclusive/end-exclusive 2D Bresenham-style stepping. | Does not include surface clipper contract, Z-buffer test, A-buffer suppression/modulation, circular-buffer sampling, or Z-write flag behavior. |
| `src/app_render/draw_passes.rs:112` and `:126` | Draws back and first-front bracket buffers before object bodies using passthrough texture draw. | Phase approximation does not model per-pixel primary-surface Z-test/no-Z-write semantics. |
| `src/app_render/draw_passes.rs:288` and `:315` | Applies shroud A-buffer multiply before later UI/front bracket drawing. | Binary samples A-buffer inside the line draw itself for bracket pixels that reach the primary surface; Rust final-front bracket draw is currently after the global shroud pass and is not A-buffer suppressed/modulated. |

Active in YR: Rust status is not binary evidence; included only for implementation targeting.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` entry and gates | verified | Fresh Ghidra decompile `0x004BFD30` | none |
| Clip helper `FUN_007BC2B0` bounds contract | verified | Fresh Ghidra decompile `0x007BC2B0` | none |
| X-normalization before clipping | verified | Fresh Ghidra decompile `0x004BFD30` | none |
| Start-inclusive/end-exclusive loops | verified | Fresh Ghidra decompile `0x004BFD30`; prior spans `0x004C0171..0x004C0738` | none |
| Dominant-axis tie rules | verified | Fresh Ghidra decompile `0x004BFD30`; prior depth-dominant reachability report | none |
| Per-pixel Z-test predicate | verified | Fresh Ghidra decompile `0x004BFD30` | none |
| A-buffer suppression/modulation formula | verified | Fresh Ghidra decompile `0x004BFD30` | none |
| Z-write flag behavior | verified | Fresh Ghidra decompile `0x004BFD30`; prior write guard spans `0x004C024F`, `0x004C043A`, `0x004C062B` | none |
| Selected-building callers pass final flag `0` | verified | Fresh Ghidra decompile `0x006F5EF0`, `0x006F5190`; prior assembly examples listed above | none |
| Selected-building YR activity gates | verified | Fresh Ghidra decompile `0x006F60D0`, `0x006F5190` | none |
| Shroud/fog A-buffer relevance | verified | Fresh Ghidra decompile `0x004801F0`, `0x0047EFE0`, `0x0047F250`; INI grep lines for `FogOfWar=no` | stock pixel-overlap matrix deferred |
| Runtime screenshot parity | deferred | not performed by this Ghidra-only slot | run visual trace/pixel probe separately |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does clipping happen before raster branch selection? Yes. `0x004BFD30` offsets endpoints by the clip rect and calls `FUN_007BC2B0` before computing branch deltas and entering loops. Evidence: Ghidra `0x004BFD30`, `0x007BC2B0`.

[RESOLVED] OQ-2 - Are clipped endpoints inclusive? Start is included, final endpoint is excluded; zero-length clipped lines write no pixel. Evidence: Ghidra `0x004BFD30` loop counts.

[RESOLVED] OQ-3 - Which axis wins ties? Depth requires strict dominance; X requires `dx > dy` after depth fails; remaining ties go Y-dominant. Evidence: Ghidra `0x004BFD30`.

[RESOLVED] OQ-4 - What is the per-pixel Z predicate? Strict unsigned 16-bit line depth `<` current Z-buffer sample. Evidence: Ghidra `0x004BFD30`.

[RESOLVED] OQ-5 - Can A-buffer suppress a bracket pixel? Yes, `abuf == 0` suppresses the pixel entirely. Evidence: Ghidra `0x004BFD30`; shroud writer `0x0047EFE0`.

[RESOLVED] OQ-6 - What does neutral A-buffer do? `0x7F` writes the original source color without modulation. Evidence: Ghidra `0x004BFD30`; prior A-buffer reset evidence `0x00411330`.

[RESOLVED] OQ-7 - Do selected building brackets write Z after drawing? No. Checked selected-building callers pass final flag `0`, and `0x004BFD30` writes Z only when the flag byte is nonzero. Evidence: Ghidra `0x006F5EF0`, `0x006F5190`, `0x004BFD30`.

[RESOLVED] OQ-8 - Is this selected-building path active in standard YR? Yes. The checked caller gates are selected byte `+0x83` and building `WhatAmI()==6`; no TS-only flag gates the bracket line call once the object reaches the draw/extras phase. Evidence: Ghidra `0x006F60D0`, `0x006F5190`.

[RESOLVED] OQ-9 - Is fog A-buffer modulation active in standard YR? Conditional. The fog blend path is gated by `SpecialFlags & 0x1000` and stock `FogOfWar=no` leaves it off; standard shroud A-buffer writes remain active. Evidence: Ghidra `0x004801F0`; `ini/rulesmd.ini:205`, `ini/rulesmd.ini:3040`.

[DEFERRED] OQ-10 - Which exact stock building bracket pixels overlap non-neutral A-buffer samples in common camera positions? Category: needs-runtime-debugger. Static Ghidra proves the per-pixel mechanism, but enumerating concrete screenshots/pixels is outside this slot.

## Sources

- Ghidra read-only decompile: `0x004BFD30`, `0x007BC2B0`, `0x006DBB60`, `0x006F5EF0`, `0x006F60D0`, `0x006F5190`, `0x004801F0`, `0x0047EFE0`, `0x0047F250`
- Prior reports used as seeds/cross-checks: `building-selection-brackets/SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md`, `building-selection-brackets/BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`, `building-selection-brackets/BUILDING_BRACKET_DEPTH_DOMINANT_RASTER_REACHABILITY_GHIDRA_REPORT.md`
- INI read-only grep: `ini/rules.ini`, `ini/rulesmd.ini`
- Rust source read-only comparison: `src/app_selection_brackets.rs`, `src/app_render/draw_passes.rs`
