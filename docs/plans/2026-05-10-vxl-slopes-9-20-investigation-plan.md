# VXL Slope Tilt Matrices for Slope Types 9-20 — Investigation Plan

> **For Claude:** This plan scopes a `/re-investigate` pass to extract the
> remaining tilt-matrix entries gamemd populates at `DAT_00B45188 + n*0x30` for
> slope types 9-16 (and confirm fate of 17-20). Execute by running
> `/re-investigate` with this plan as context, OR by directly running the
> specific Ghidra reads enumerated in §3.

**Topic:** VXL slope tilt matrix entries for slope types 9-20 — the unimplemented half of the slope table that currently falls through to `Mat4::IDENTITY` in our renderer.
**Scope Size:** Small — the population code lives in a single function (`VXL_MasterLighting_Init` at `0x00754CB0`) which the light scoping pass already partially decoded.
**Est. Effort:** ~30-60 min `/re-investigate` work — the population shape is already known; the remaining work is value-extraction + indexing-function trace + edge-case verification.
**Prior Research:** `docs/research/VOXEL_SLOPE_TILT_SYSTEM.md` (verified GREEN 2026-05-07, doc updated 2026-05-10 to fix matrix-order claim).
**Expected Output:** an extension to `VOXEL_SLOPE_TILT_SYSTEM.md` titled "Slope Matrix Table — Full Entry List" enumerating all 16 (or 20) populated entries with verified formula and binary-read confirmation.
**Next Pipeline Step:** `/brainstorm` then implement — the values plug into our existing `compute_slope_rotation` match statement.

---

## 1. Goal

Determine the exact tilt magnitude and compass direction gamemd uses for slope types 9-16 (and the fate of 17-20), so that `compute_slope_rotation` in `src/render/vxl_raster.rs` can render units on cliff-transition cells with the correct tilt instead of falling through to `Mat4::IDENTITY`. The investigation must answer: for each `slope_type ∈ [9, 20]`, what is the equivalent of `(compass_rad, tilt_rad)` that produces the same `Rz(c)·Rx(t)·Rz(-c)` matrix the binary stores in `DAT_00B45188 + slope_type * 0x30`?

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps |
|---|---|---|---|
| `VOXEL_SLOPE_TILT_SYSTEM.md` | Full slope tilt pipeline: data flow, body vs turret paths, matrix construction | HIGH (verified GREEN, just updated) | Tilt magnitudes for slopes 9-20 explicitly listed as unknown; doc's "MidNW/SteepSE/Double-ramp" interpretation comes from TS++ enum names, not verified from binary |
| `SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md` | LevelHeight=104, cell geometry constants | HIGH | Independent confirmation of the LevelHeight constant the tilt formulas reduce around |

**Conflicts between reports:** none. The doc's open question "tilt for 9-20 unknown" is what this investigation resolves.

## 3. Function Inventory

Light scoping ALREADY decompiled `VXL_MasterLighting_Init` (the entire population code) and surfaced the structure below. The `/re-investigate` pass needs to extend this with binary reads and indexing-function decode.

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|---|---|---|---|---|---|
| 1 | 1 | `0x00754CB0` | `VXL_MasterLighting_Init` | Master init that populates `DAT_00B45188 + n*0x30` for n=1..16. Already partially decoded — exact compass+tilt for each n known structurally. | FULL — extract every `BuildFromRotateXAndFacing(angle_const, tilt_value)` call's args, decode IEEE 754 single literals, map index to compass | None — runs unconditionally at engine init. |
| 2 | 1 | `0x005AE6F0` | `Matrix3x4_BuildFromRotateXAndFacing` | The matrix constructor used per-entry. Confirms the `Rz(facing) × Rx(tilt) × Rz(-facing)` decomposition matches what we compute in Rust. | MEDIUM — verify the 3×4 row-major output structure and that the rotation order matches `compute_slope_rotation` | None |
| 3 | 1 | `0x007559B0` | `VXL_GetFacingMatrix` | Indexes the table from `slope_type`. Determines whether `slope_type` is used directly or has a clamp / encoding. | FULL — decode the indexing math; confirm `index = slope_type * 0x30` (no zoom encoding, no clamp). | LOW — probably direct |
| 4 | 1 | `0x00755A40` | `VXL_InterpolatedFacing` | Interpolated lookup used during slope transitions. Confirms whether the same table is shared for transition smoothing. | LIGHT — just confirm same base address; we don't need transition smoothing for slopes 9-20 yet. | None |
| 5 | 2 | `DAT_00B45188`-`DAT_00B454B8` | (Data) | The 17-entry slope table itself. **Read the raw bytes** as a sanity check that the populated matrices match the formulas. Each entry 48 bytes = 12 f32s. | FULL — `read_memory` 17 × 48 bytes = 816 bytes; decode each as 3×4 matrix; cross-check against `Rz(c)·Rx(t)·Rz(-c)` for the (c, t) the population code claims. | None — pure data |
| 6 | 2 | `0x004AFF60` | `DriveLocomotionClass::Draw_Matrix` | Caller path for body. Already decompiled this session. Verify that `slope_type` flows from `locomotor+0x18` to `VXL_GetFacingMatrix` without clamp. | LIGHT — already known; just confirm no missed clamp before lookup. | None |
| 7 | 2 | `0x00729B40` | (Turret/Barrel tilt computation) | Turret path — same `DAT_00B45188` lookup but separate state machine. Confirm turret tilts match body tilts for slopes 9-16 (i.e., turret uses the same table entries). | LIGHT — confirm same base address | LOW — the aircraft state-machine branches (states 2-7) are separately deferred, not part of this investigation. |
| 8 | 3 | TMP `+0x2A` reader | (in `FUN_005471B0` per VOXEL_SLOPE_TILT_SYSTEM.md) | Source of the slope_type byte. Verify what range of values appears in real YR map TMPs — does any standard map actually use slope_type 17-20? | LIGHT — just spot-check a few TMPs from the retail asset path | LOW — purely empirical |
| 9 | 3 | `0x0047D2B0` (`FUN_0047D2B0`) | Cell recalc — writes `cell+0x11C` | Confirms no transformation between TMP byte and cell field. Per the doc, this is direct copy. | LIGHT — confirm no transformation | None |
| 10 | 3 | `cell+0x11C` → `locomotor+0x18` flow | (multiple sites) | Confirms no clamp/encoding when slope_type moves from cell to locomotor. | LIGHT — grep references to `+0x18` write paths in DriveLocomotion | None |

**Phase 1 checkpoint rule:** after Phase 1 (functions #1-#4), stop and summarize. The expected finding is the full list of (compass_rad, tilt_rad) pairs for slope types 1-16. If those don't match the IEEE-754 decode I already did during scoping, the scoping was wrong and Phase 2 needs re-scoping.

## 4. Detail Checklist

Categories to extract during execution:

- **Magic numbers to decode:**
  - Already decoded during scoping (verify on second pass):
    - `0x4096CAC1 = 4.71239 = 3π/2 = 270°` (West)
    - `0x40490E56 = π = 180°` (North)
    - `0x3FC90E56 = π/2 = 90°` (East)
    - `0x00000000 = 0°` (South)
    - `0x407B51EC = 5π/4 = 225°` (NW)
    - `0x4016CAC1 = 3π/4 = 135°` (NE)
    - `0x3F490E56 = π/4 = 45°` (SE)
    - `0x40AFEC8B = 7π/4 = 315°` (SW)
  - Verify these are the ONLY angle constants used; no surprise "half-angle" or "negated" variants for slopes 9-16.
- **Tilt-magnitude variable:**
  - `fVar1 = local_c8 = (float)_DAT_00B44310` = `EDGE_TILT_RAD = 0.5214767`
  - `fVar2 = local_34 = (float)_DAT_00B43F08` = `CORNER_TILT_RAD = 0.3858827`
  - Verified this session.
- **Bit flags / masks:** none expected — the function unconditionally populates every entry. Confirm during execution.
- **Indexing math in `VXL_GetFacingMatrix`:** confirm it's `base + slope_type * 0x30` with no clamp, no shift, no encoding.
- **Edge cases:**
  - `slope_type = 0`: returns identity (presumably). Confirm the early-exit path.
  - `slope_type ∈ [17, 20]`: not populated by `VXL_MasterLighting_Init`. What does `VXL_GetFacingMatrix` return? Is the BSS zero-filled (gives an all-zero matrix → degenerate render)? Is there a clamp at the call site? Or does `VXL_MasterLighting_Init` have a tail loop that defaults the rest to identity?
  - `slope_type > 20`: same question. Probably never occurs in real TMP data, but verify with map spot-check.
- **Timing/ordering:** the table is populated once at engine init by `CCFileClass__Constructor` (per `get_function_callers`). Read-only thereafter. No tick-order concerns.
- **TS-legacy flags:** none in this function — no `SpecialFlags` gates.

## 5. INI Keys in Scope

**None.** Slope tilt magnitudes are binary-baked geometry, not INI-driven. Confirmed by Agent B equivalent (grep of `ini/rulesmd.ini` and `ini/artmd.ini` for `tilt`/`slope`/`ramp` keys → none touch the matrix table; `[General]` flags are unrelated).

## 6. Caller & Integration Map

| Caller Address | Calls Into | When Invoked | Should Executor Decompile? |
|---|---|---|---|
| `0x0052BA60` (`CCFileClass__Constructor`) | `VXL_MasterLighting_Init` (#1) | Engine init, before any rendering | LIGHT — already known to be one-shot init |
| `0x004B0250`, `0x004B03A5` (in `Draw_Matrix`) | `VXL_GetFacingMatrix` (#3) | Per-frame body render | Already decompiled this session |
| `0x004B0234`, `0x004B0389` (in `Draw_Matrix`) | `VXL_InterpolatedFacing` (#4) | Per-frame body render during transitions | LIGHT — confirm same table base |
| Inside `0x00729B40` (turret tilt) | `VXL_GetFacingMatrix` and/or `VXL_InterpolatedFacing` | Per-frame turret render | LIGHT — confirm same table |

**Rust integration today:**
- [src/render/vxl_raster.rs:249-267](src/render/vxl_raster.rs#L249-L267) `compute_slope_rotation` handles slopes 1-8; 9-20 fall through to `Mat4::IDENTITY`.
- [src/app_instances/units.rs:81-87](src/app_instances/units.rs#L81-L87) clamps `slope_type` at the consumer side: `if c.slope_type <= 8 { c.slope_type } else { 0 }`. **This clamp must be raised after this investigation** — it's the gate currently hiding slopes 9-20 from the renderer.
- [src/render/unit_atlas.rs:210-211](src/render/unit_atlas.rs#L210-L211) atlas pre-renders 9 slope variants (0-8) per ground vehicle. **The range must be widened** to 0-16 (or 0-20) once we know the matrices.

**Callers we will NOT investigate:**
- `Quaternion_*` paths inside `VXL_MasterLighting_Init` — those populate quaternion tables for slope-transition smoothing, not the matrix table. Out of scope for this investigation.
- The "lower block" at `DAT_00B43F70..DAT_00B44210` in the same function — those are zoom-level view matrices for VXL lighting, not the slope table. Out of scope.

## 7. TS-Legacy Risk Register

- **The doc's "MidNW/SteepSE/DoubleRamp" interpretation of slope types 9-20 comes from TS++ `TIBSUN_DEFINES.H` enum names, NOT verified from gamemd binary.** During Phase 1, do NOT assume a TMP cell with `ramp_type = 9` actually triggers any "mid ramp" rendering. The actual matrix entries the binary populates may not match the TS-era enum semantics. **Verify before believing the names.**
- **Slope types 17-20 may be effectively dead in YR.** If `VXL_MasterLighting_Init` doesn't populate them and `VXL_GetFacingMatrix` doesn't clamp, then a TMP cell with `ramp_type = 17` would produce a degenerate matrix in gamemd too — which means our parity story is "match gamemd's failure mode" rather than "fill in missing data." Worth knowing before designing the fix.
- **No `SpecialFlags`-gated branches in this function** — Agent D scoping confirmed. Low TS-legacy risk inside the population code itself.

## 8. Current Rust Implementation Surface

- [src/render/vxl_raster.rs:249-267](src/render/vxl_raster.rs#L249-L267) — `compute_slope_rotation(slope_type: u8) -> Mat4`
  - Slopes 1-8 mapped to (compass, tilt) pairs.
  - Slopes 9-20 → `Mat4::IDENTITY` (the fallthrough).
- [src/app_instances/units.rs:81-87](src/app_instances/units.rs#L81-L87) — slope_type read with `≤8` clamp.
- [src/render/unit_atlas.rs:210-211](src/render/unit_atlas.rs#L210-L211) — atlas pre-render range `0..=8`.
- [src/render/vxl_raster.rs:43-58](src/render/vxl_raster.rs#L43-L58) — `EDGE_TILT_RAD` and `CORNER_TILT_RAD` constants (already verified).

## 9. Deferred Open Questions

Questions the scoping scan surfaced but did NOT resolve — these are the explicit "answer during execution" list:

1. **Are slope_type entries 9-12 truly identical to 5-8?** Scoping showed `VXL_MasterLighting_Init` calls `BuildFromRotateXAndFacing` with the same compass+tilt args for indices 9-12 as for 5-8. Confirm by reading the raw bytes at `DAT_00B45188 + 9*0x30` and `DAT_00B45188 + 5*0x30` and verifying they're equal. If equal, document as "9-12 are aliases for 5-8 in gamemd." If not equal, find the second population code.
2. **Are slope_type entries 13-16 EDGE-tilt at corner-compass directions?** Scoping suggested yes. Confirm via raw byte read.
3. **What is at `DAT_00B45188 + 17*0x30` through `+ 20*0x30`?** Scoping found no population code for these. Read the bytes — is it zero-filled BSS, or is there another population path?
4. **Does `VXL_GetFacingMatrix` clamp `slope_type`?** Decode the function. If yes, slopes 17-20 fall to a default (likely 0 = identity). If no, they'd produce a degenerate matrix in gamemd too.
5. **Does the cell→locomotor flow ever transform `ramp_type`?** Spot-check the assignment to `locomotor+0x18`. Per the doc it's a direct copy; verify.
6. **Empirical TMP byte distribution.** Spot-check a handful of standard YR maps' TMP files for the actual range of `ramp_type` values that appear in practice. If no standard map ever uses 9+ , the parity gap is theoretical; if many maps use them, it's high-frequency.

## 10. Execution Strategy

**Single-session `/re-investigate`.** The scope is small enough (3 functions to decompile FULL + 1 raw memory read of 816 bytes + 1 INI/map empirical check) that batching to subagents would add overhead without speedup. One focused pass extracts every value.

Execution sequence:
1. Read `DAT_00B45188 + 0` through `+ 20*0x30` (1008 bytes) as raw memory; decode each 48-byte block as a 3×4 row-major matrix. Resolves Q1, Q2, Q3 directly.
2. Decompile `VXL_GetFacingMatrix` (function #3); confirm indexing. Resolves Q4.
3. Spot-check the cell→locomotor flow (function #10); confirm direct copy. Resolves Q5.
4. (Optional, deferrable) glob the retail map archive for TMP files; sample a few; tally `ramp_type` byte distribution. Resolves Q6.
5. Synthesize findings into an addendum to `VOXEL_SLOPE_TILT_SYSTEM.md`.

## 11. Success Criteria

The executed research document must:

- Answer the goal in §1: produce a complete `(compass_rad, tilt_rad)` table for `slope_type ∈ [1, 20]` (or document degenerate/fallback behavior for 17-20 if no entries exist).
- Cite raw memory bytes for at least one entry per category (edge, corner, edge-at-diagonal, alleged-mid-ramp) as binary-confirmation of the formula derivation.
- Include every function from §3 (or explicitly justify omission, e.g., "function #4 confirmed identical to #3 in indexing math, no separate decode needed").
- Resolve every deferred question from §9 (or re-document as unresolved with reason — e.g., "Q6 deferred because retail TMP empirical study is a separate task").
- State "Active in YR: Yes/No/Conditional" for each slope_type's matrix path.
- Confirm or correct the `VOXEL_SLOPE_TILT_SYSTEM.md` "Mid/Steep/Double" interpretation against actual binary entries.

## Sources

- **Ghidra addresses sampled during scoping (already done):**
  - `0x00754CB0` `VXL_MasterLighting_Init` — full decompile
  - Xrefs to `DAT_00B45188` from `VXL_GetFacingMatrix`, `VXL_InterpolatedFacing`, `VXL_MasterLighting_Init`
  - Callers of `VXL_MasterLighting_Init`: `CCFileClass__Constructor` only
  - Callees of `VXL_MasterLighting_Init`: matrix builders, quaternion utilities (out of scope)
- **Docs searched:**
  - `docs/research/VOXEL_SLOPE_TILT_SYSTEM.md` (full read)
  - `docs/research/SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md` (LevelHeight anchor)
- **INI files checked:** `ini/rulesmd.ini`, `ini/artmd.ini` — no slope-tilt-related keys.
- **Related plans:** `docs/plans/2026-05-10-vxl-slope-tilt-constants-design.md`, `docs/plans/2026-05-10-vxl-slope-tilt-constants-plan.md` (the just-completed slopes 1-8 work).

---

## Top-3 findings from scoping

1. **The population code is fully extractable in one Ghidra session.** `VXL_MasterLighting_Init` at `0x00754CB0` was decompiled during scoping and shows 16 explicit `BuildFromRotateXAndFacing(angle_const, tilt_var)` calls with IEEE 754 single literals. Most of the analytical work for slopes 1-16 is already done; the `/re-investigate` pass mainly verifies via raw memory reads.
2. **Slopes 9-12 appear to be exact duplicates of 5-8 (CORNER tilt at NW/NE/SE/SW); slopes 13-16 use EDGE tilt at the same NW/NE/SE/SW compass directions.** This contradicts the TS++ `TIBSUN_DEFINES.H` enum names ("MidNW", "SteepSE", "DoubleRamp") that VOXEL_SLOPE_TILT_SYSTEM.md cited — the actual matrix shape is just "more rotations of the same two tilt magnitudes."
3. **Slopes 17-20 are not populated by this function.** Either there's a second population path (unlikely — only one caller, `CCFileClass__Constructor`), or these slope types don't have valid matrices in gamemd at all. If true, units on TMP cells with `ramp_type ≥ 17` would render with a zero-filled (degenerate) matrix in gamemd — a "match the failure mode" parity question rather than "fill in missing data."
