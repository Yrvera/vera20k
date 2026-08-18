# Building Bracket A-buffer/Z-test/Depth Semantics - Ghidra Report

**Report path:** `docs/research/building-selection-brackets/BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`  
**Target:** selected building bracket `Tactical::DrawLine3D -> Surface::Draw_Line` depth, A-buffer, Z-test, and Z-write behavior  
**Investigation mode:** exhaustive-slice, read-only Ghidra/live decompilation  
**Status:** COMPLETE  
**Confidence:** High for call arguments and surface write predicates; Medium for stock content overlap with shroud edge pixels because no runtime screenshot matrix was captured.  
**Active in YR:** Yes for selected-building bracket line draw and shroud A-buffer modulation; Conditional for fog-of-war modulation because standard YR has `FogOfWar=no`.

## 1. Scope

This report resolves the final line-draw semantics after selected building brackets call:

```text
TechnoClass::DrawBehind / DrawExtras
  -> TechnoClass::DrawBracketCorner or direct DrawLine3D stubs
  -> Tactical::DrawLine3D @ 0x006DBB60
  -> g_PrimarySurface vtable +0x34
  -> Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30
```

Non-scope: enumerating every stock building segment against every shroud-edge SHP pixel, runtime screenshots, and implementing Rust behavior.

## 2. Verified Binary Evidence

### 2.1 Building bracket callers pass final flag `0`

`TechnoClass::DrawBracketCorner @ 0x006F5EF0` pushes `0` before both `g_Tactical->vtable+0x60` calls. Evidence: assembly `0x006F5FCE` and `0x006F5FED`.

The three direct building bracket stubs in `TechnoClass::DrawExtras @ 0x006F5190` also push `0` before their direct `DrawLine3D` calls. Evidence examples: `0x006F5762`, `0x006F58B1`, `0x006F59D3`.

Active in YR: Yes. These sites are in the selected building bracket paths already verified by `DrawBehind` / `DrawExtras` reports.

### 2.2 `DrawLine3D` forwards two endpoint depths plus the final flag

`Tactical::DrawLine3D @ 0x006DBB60` projects both 3D endpoints, then calls `g_PrimarySurface` slot `+0x34`. The call frame before `0x006DBCC7` is:

```text
push incoming final flag              ; 0x006DBC70
push depth_a = 0xE - AdjustForZ(z_a)  ; 0x006DBC91, 0x006DBC99
push depth_b = 0xE - AdjustForZ(z_b)  ; 0x006DBCA9..0x006DBCB8
push color                            ; 0x006DBCB9
push screen endpoint pointers         ; 0x006DBCBE, 0x006DBCBF
push clip rect 0x00886FA0             ; 0x006DBCC0
mov ecx, g_PrimarySurface             ; 0x006DBCC5
call [vtable+0x34]                    ; 0x006DBCC7
```

The `AdjustForZ` expression uses `ftol(z * g_AdjustForZ_Mult + (z >= 0x2D8 ? 1 : 0) + 0.5)`. Evidence: `Tactical::AdjustForZ @ 0x006D20E0..0x006D2113`; matching inline depth setup at `0x006DBC62..0x006DBCA4`.

Active in YR: Yes.

### 2.3 Surface pixel writes require both Z-test pass and nonzero A-buffer

`Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30` samples `g_ZBuffer` and `g_ABuffer` for every candidate line pixel. In all three dominant raster branches, the write predicate is:

```text
if ((ushort)line_z < *zbuf && abuf_value != 0) {
    write color or A-buffer-modulated color to primary surface;
    if (z_write_flag != 0) *zbuf = line_z;
}
```

Evidence:
- z-dominant path: decompile at `0x004C0171..0x004C0358`; Z-write flag read at `0x004C024F`, write skipped by `JZ 0x004C0265`.
- x-dominant path: `0x004C0373..0x004C0554`; flag read at `0x004C043A`, write skipped by `JZ 0x004C0450`.
- y-dominant path: `0x004C0557..0x004C0738`; flag read at `0x004C062B`, write skipped by `JZ 0x004C0639`.

Active in YR: Yes.

### 2.4 Building bracket lines Z-test but do not Z-write

Because the selected-building bracket callers pass final flag `0`, the surface routine can draw a bracket pixel only if the existing Z-buffer value is greater than the line's interpolated `line_z`, but it does not replace the Z-buffer value after drawing.

This means bracket pixels are occluded by earlier Z-buffer contents but do not occlude later pixels that also Z-test. Evidence: caller flag `0` at `0x006F5FCE`, `0x006F5FED`, `0x006F5762`; surface flag guards at `0x004C024F`, `0x004C043A`, `0x004C062B`.

Active in YR: Yes.

### 2.5 A-buffer value controls bracket suppression/dimming

When the A-buffer sample is:

| A-buffer sample | Surface behavior | Evidence |
|---:|---|---|
| `0` | suppress pixel entirely | `if (... && abuf_value != 0)` in all three surface branches |
| `0x7F` | write original 16-bit color | explicit `abuf_value == 0x7F` fast path in `0x004BFD30` |
| nonzero, not `0x7F` | channel modulation by `(abuf * channel) >> 7` | formulas in `0x004C01C0`, `0x004C03AE`, `0x004C058B` regions |

Active in YR: Yes.

## 3. Shroud/Fog State Relevance

### 3.1 Shroud A-buffer writes are active in standard YR

`Shroud_fog_edge_rendering @ 0x004801F0` always computes the shroud frame, stores it at `cell+0x120`, maps `-2 -> 0x0F` and `-1 -> 0`, and calls `ShroudEdge_BlitToABuffer`. Evidence: decompile `0x004801F0`.

`ShroudEdge_BlitToABuffer @ 0x0047EFE0` writes SHP pixel values directly into `g_ABuffer` unless the source pixel is transparent (`0xFE`). It uses `SHROUD.SHP` when `(*g_ScenarioClass_Instance & 0x1000) == 0`. Evidence: decompile `0x0047EFE0`.

Active in YR: Yes. `ini/rulesmd.ini` has `[MultiplayerDialogSettings] FogOfWar=no` and shroud is on by default.

### 3.2 Fog A-buffer writes are conditional and off by default

The fog pass in `Shroud_fog_edge_rendering @ 0x004801F0` calls `FogEdge_BlendToABuffer @ 0x0047F250` only when:

```text
(*g_ScenarioClass_Instance & 0x1000) != 0
&& *(byte *)(g_PlayerPtr + 0x1F5) == 0
```

`FogEdge_BlendToABuffer` blends only source pixels `< 0x80` into `g_ABuffer`. Evidence: decompile `0x004801F0` and `0x0047F250`.

Active in YR: Conditional. Standard YR `FogOfWar=no` leaves `SpecialFlags & 0x1000` clear, so this fog dimming path is dormant unless explicitly enabled.

### 3.3 Dirty A-buffer regions are reset to neutral before shroud redraw

`Tactical_layer_shroud_edges @ 0x006D3660` calls `FUN_00411330` with `ECX = g_ABuffer` for dirty regions. `FUN_00411330 @ 0x00411330` fills 16-bit A-buffer pixels with `0x007F` / `0x007F007F`, the neutral value used by the surface line fast path.

Evidence: assembly `0x006D382F..0x006D3835`; decompile/assembly of `0x00411330`.

Active in YR: Yes.

## 4. Player-visible Conclusions

1. Selected building bracket pixels are not a pure UI overlay. They are per-pixel clipped, A-buffer-modulated, and Z-tested through the primary surface line drawer. Active in YR: Yes.

2. Selected building bracket pixels can be suppressed by standard shroud A-buffer state when the candidate line pixel lands on an A-buffer value of `0`. Active in YR: Yes.

3. Selected building bracket pixels can be dimmed by standard shroud edge A-buffer values when the candidate line pixel lands on a nonzero value other than `0x7F`. Active in YR: Yes.

4. Fog-of-war dimming can also modulate bracket pixels through the same A-buffer mechanism, but only when the session has `FogOfWar=yes` / `SpecialFlags & 0x1000`. Active in YR: Conditional; default standard YR: No.

5. Fully undiscovered/shrouded objects are normally prevented earlier from rendering as selected objects/display-layer entries; this report only proves final surface behavior once a selected building bracket line reaches `Surface::DrawLine_ABufModulated_ZClipped`. Active in YR: Conditional.

## 5. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Building bracket final flag | verified | `0x006F5FCE`, `0x006F5FED`, `0x006F5762`, `0x006F58B1`, `0x006F59D3` | none |
| `DrawLine3D` depth call frame | verified | `0x006DBC62..0x006DBCC7`, `0x006D20E0..0x006D2113` | none |
| Surface Z-test | verified | `0x004BFD30`, branch bodies at `0x004C0171`, `0x004C0373`, `0x004C0557` | none |
| Surface Z-write flag | verified | `0x004C024F`, `0x004C043A`, `0x004C062B` | none |
| Surface A-buffer suppression/modulation | verified | `0x004BFD30` write predicates and modulation formulas | none |
| Standard shroud A-buffer writes | verified | `0x004801F0`, `0x0047EFE0`, `0x006D3835`, `0x00411330` | no stock screenshot matrix |
| Fog A-buffer writes | verified conditional | `0x004801F0`, `0x0047F250`, `ini/rulesmd.ini FogOfWar=no` | only relevant when fog is enabled |

## 6. Open Questions - Final State

[RESOLVED] OQ-1 - Is the final `DrawLine3D` argument a line depth? No. It is forwarded as the surface Z-write flag; endpoint depths are computed separately from endpoint Z. Evidence: `0x006DBC70`, `0x006DBC91`, `0x006DBCB8`, surface flag reads at `0x004C024F`/`0x004C043A`/`0x004C062B`.

[RESOLVED] OQ-2 - Do selected building brackets Z-test? Yes. Candidate pixels require `(ushort)line_z < *zbuf`. Evidence: `0x004BFD30` decompile across all three dominant branches.

[RESOLVED] OQ-3 - Do selected building brackets write Z? No for stock bracket callers; the final flag is zero. Evidence: `0x006F5FCE`, `0x006F5FED`, `0x006F5762`; surface guarded writes.

[RESOLVED] OQ-4 - Can standard shroud A-buffer suppress/dim selected bracket pixels after the line reaches the surface routine? Yes. `abuf==0` suppresses; nonzero/non-`0x7F` modulates. Evidence: `0x004BFD30`, `0x004801F0`, `0x0047EFE0`.

[RESOLVED] OQ-5 - Is fog-of-war bracket dimming active in standard YR? No by default. It requires `SpecialFlags & 0x1000`; `rulesmd.ini` default is `FogOfWar=no`. Evidence: `0x004801F0`, `ini/rulesmd.ini`.

[DEFERRED] OQ-6 - Which exact stock building bracket pixels overlap non-neutral shroud edge samples in normal camera positions? Category: needs-runtime-debugger. Static evidence proves the mechanism; a screenshot/pixel probe should enumerate concrete cases.

## Sources

- Ghidra decompiled/read-only: `0x006DBB60`, `0x004BFD30`, `0x006F5EF0`, `0x006F60D0`, `0x006F5190`, `0x006D20E0`, `0x004801F0`, `0x0047EFE0`, `0x0047F250`, `0x006D3660`, `0x00411330`
- Ghidra assembly context: `0x006DBC62..0x006DBCC7`, `0x004C024F`, `0x004C043A`, `0x004C062B`, `0x006D382F..0x006D3835`
- Prior docs used as seeds/cross-checks: `DRAWBRACKETCORNER_DRAWLINE3D_STUB_RASTER_GHIDRA_REPORT.md`, `SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md`, `SHROUD_FOG_RENDERING_PIPELINE.md`, `OBJECT_FOG_VISIBILITY_GHIDRA_REPORT.md`
- INI checked read-only: `ini/rulesmd.ini`
