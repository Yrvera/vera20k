# TechnoClass::Draw BeingWarped Translucency - Ghidra Report

**Date:** 2026-05-28  
**Scope:** `TechnoClass::Draw` behavior when `IsWarpingOut` or `IsBeingWarped` is true, and the Rust-facing unit render delta.  
**Report path:** `C:/Users/enok/Documents/ra2-rust-game/docs/research/TECHNOCLASS_DRAW_BEINGWARPED_TRANSLUCENCY_GHIDRA_REPORT.md`

## Working Notes

Target question: confirm exact active-YR draw behavior when `TechnoClass::IsWarpingOut` or `TechnoClass::IsBeingWarped` is true, including draw flags, blitter path, alpha/translucency semantics, branch predicates, and whether non-harvester `TeleportLocomotion` post-warp cooldown reaches this path.

Non-goals: do not investigate temporal weapon phase math, Chronosphere phase sequencing beyond the scoped branch evidence, full Techno draw unrelated to warping, or teleport visual `AnimClass` rows already settled by sibling reports.

Evidence needed to mark COMPLETE: direct decompile plus assembly-context evidence for the `TechnoClass::Draw` warp flag branch; accessors proving `+0x270` and `+0x271`; blitter selector evidence for `flags & 6`; `TeleportLocomotion` evidence that non-harvester self-teleport leaves `+0x271` set during cooldown; Rust render-surface mapping.

Stop conditions: stop after the scoped branch, blitter selection, non-harvester liveness, Rust handoff, stale-doc replacement wording, and remaining uncertainty are recorded.

## Executive Summary

Active in YR: Yes. `TechnoClass::Draw @ 0x00706640` calls vtable `+0x1D4` and `+0x1D8`; if either returns nonzero and the infantry-deploy/harvester-render exclusions do not block it, the function ORs `0x2004` into the voxel draw flags, or `0x2006` for one building-type special case. `TechnoClass::IsWarpingOut @ 0x0070C5B0` returns byte `this+0x270`; `TechnoClass::IsBeingWarped @ 0x0070C5C0` returns byte `this+0x271`.

For ordinary units and non-harvester teleporters, the player-visible behavior is a binary 50% translucent unit while `+0x271` remains set. This is not a gradual alpha ramp and not solely the `WarpOut` animation overlay. The current Rust unit renderer still sets voxel unit `alpha` to `1.0` unconditionally in `src/app_instances/units.rs`, even though `src/sim/movement/teleport_movement.rs` already carries `being_warped_ticks`.

## Verified Findings

### 1. Warp predicates in `TechnoClass::Draw`

Active in YR: Yes. Evidence: `TechnoClass__Draw @ 0x00706640` decompile calls `(*vtable + 0x1D4)()` then, only if that is zero, `(*vtable + 0x1D8)()`. Assembly context shows `0x00706694: CALL dword ptr [EDX + 0x1d4]`, `0x0070669A: TEST AL,AL`, `0x0070669C: JNZ 0x007066AD`, `0x007066A3: CALL dword ptr [EAX + 0x1d8]`, `0x007066A9: TEST AL,AL`, and `0x007066AB: JZ 0x00706706`.

The branch is logical OR: `IsWarpingOut != 0 || IsBeingWarped != 0`. If both are zero, control jumps to `0x00706706` and no warp translucency flag is added.

### 2. Accessor offsets are byte reads, not inferred state

Active in YR: Yes. Evidence: `TechnoClass__IsWarpingOut @ 0x0070C5B0` decompiles to `return this->field_0x270;`; `TechnoClass__IsBeingWarped @ 0x0070C5C0` decompiles to `return *(undefined1 *)(param_1 + 0x271);`.

The field widths are byte-return semantics in these accessors. This report does not claim all writers are byte-clean beyond the scoped teleport writer evidence below.

### 3. Ordinary unit warp draw flags become `0x2004`

Active in YR: Yes. Evidence: after either warp predicate is true, `TechnoClass::Draw` gates out a special infantry deploy byte and then either ORs `0x2004` or `0x2006`. Assembly context shows the ordinary path at `0x00706700: OR ESI,0x2004`. The alternate building-type branch reads type byte `+0x16B1` and uses `0x007066F8: OR ESI,0x2006`.

For non-building units, the building-type `+0x16B1` branch is not reached because the `WhatAmI()==6` building check fails. Therefore ordinary unit warping uses `0x2004`.

### 4. The final voxel flags also receive `0x800`

Active in YR: Yes. Evidence: assembly context immediately after the warp branch shows `0x0070674D: OR ESI,0x800`, `0x00706753: NOT EDI`, `0x00706757: AND ESI,EDI`; the decompile expresses this as `param_11 = (uVar12 | 0x800) & ~uVar4`.

The implementation-facing flag for ordinary unit warping should be understood as base/visual flags with bit pattern `flags & 0x6 == 0x4`, plus the usual VXL draw `0x800` unless caller mask removes bits. Do not model this as only an `alpha` number without preserving material/depth meaning if the renderer later supports native-like draw modes.

### 5. Blitter selector consumes `flags & 6`

Active in YR: Yes. Evidence: `Blitter_selector_extended @ 0x00490E50` decompiles `uVar1 = param_2 & 6` and dispatches separate branches for `uVar1 == 2`, `uVar1 == 4`, and `uVar1 == 6`, each with additional sub-branches for `0x4000`, `_DAT_0081dc28 & flags`, bit `0x8`, and bit `0x800`.

For cached VXL draw, `VXL_CacheBlit @ 0x00707480` calls `Blitter_selector_extended(param_5 & 0xffffffef)`. For uncached VXL draw, `TechnoClass__Render @ 0x00706ED0` also re-checks the warp predicates, ORs `param_8` with `4` or `6`, then calls `Blitter_selector(param_8 & 0xffffffef)`. Both cache and raster paths therefore have a warp-translucent draw-mode path.

### 6. Non-harvester self-teleport leaves `+0x271` set for the cooldown

Active in YR: Yes for non-harvester `TeleportLocomotion` self-teleport. Evidence: `TeleportLocomotionClass__StateMachineTick @ 0x007192F0` sets `*(techno+0x271)=1` in the self-teleport path; assembly context shows `0x00719579: MOV byte ptr [ECX + 0x271],0x1`.

Immediately afterward the harvester-only branch checks `WhatAmI()==1` and `UnitType+0xE0E`; if both are true, it zeroes the timer and clears `+0x271`. Assembly context shows `0x00719588: CMP EAX,0x1`, `0x00719596: MOV AL,byte ptr [ECX + 0xe0e]`, and `0x007195B6: MOV byte ptr [EAX + 0x271],CL` after `XOR ECX,ECX`. Non-harvesters do not enter that clear branch.

### 7. Cooldown ticks keep `+0x271` live until timer expiry

Active in YR: Yes. Evidence: the pre-phase branch in `TeleportLocomotionClass__StateMachineTick @ 0x007192F0` reads `techno+0x271`, requires `WarpPhase==0` and `PendingWarpPhase==0`, and calls the locomotor vtable timer check. Assembly context around `0x00719304..0x00719322` shows read of `byte ptr [ECX + 0x271]`, tests `WarpPhase` and `PendingWarpPhase`, then calls vtable `+0x28`.

`TeleportLocomotionClass__TimerCheck @ 0x00719BF0` clears `+0x271` only when its timer has expired. Assembly context shows `0x00719C12: MOV EAX,dword ptr [ESI + 0xc]` followed by `0x00719C15: MOV byte ptr [EAX + 0x271],0x0`. Before expiry, the decompile returns without clearing the byte.

## Rust-Facing Surface

Current Rust already has the state carrier:

- `src/sim/movement/teleport_movement.rs` stores `TeleportState { phase, target_rx, target_ry, being_warped_ticks }`.
- Non-harvester teleport uses `compute_chrono_delay` and stores nonzero `being_warped_ticks`; harvester path stores zero.
- `tick_teleport_movement` keeps `TeleportState` through `ChronoDelay` and decrements `being_warped_ticks`.

Current Rust render delta:

- `src/app_instances/units.rs` sets `let alpha: f32 = 1.0;` for voxel units.
- The emitted body/turret/barrel `SpriteInstance`s all receive that unconditional alpha.
- The local comment saying the unit itself stays fully opaque during chrono teleport is stale against the binary evidence above.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Non-harvester `TeleportLocomotion` sets `+0x271=1` after relocation and `TechnoClass::Draw` maps that to ordinary unit `flags & 6 == 4` until timer expiry. | When a unit has `teleport_state.being_warped_ticks > 0`, emit voxel unit sprites with native-equivalent 50% translucent material; if the renderer only supports alpha today, use that as an interim visual but retain a draw-mode marker for future blitter parity. | `src/app_instances/units.rs`, `SpriteInstance` material fields if extended, teleport render tests. | Chrono Legionnaire-style non-harvester teleports 20 cells, arrives immediately, remains translucent for the computed cooldown, then returns opaque after countdown reaches zero. | `unit_render_non_harvester_teleport_chrono_delay_uses_warp_translucency` | High: opaque units during cooldown are visibly wrong and frequent for non-harvester teleporters. |
| Harvester special branch clears `+0x271` immediately after setting it when `WhatAmI()==1 && UnitType+0xE0E != 0`. | Do not apply the cooldown translucency to chrono miners when Rust passes `is_harvester=true` and stores `being_warped_ticks=0`. | `src/sim/movement/teleport_movement.rs`, `src/app_instances/units.rs`. | Chrono miner far self-teleport has WarpOut rows but no post-arrival translucent unit phase. | `unit_render_harvester_teleport_no_being_warped_translucency` | Medium: over-broad render rule would make chrono miners translucent when native clears the byte. |
| Native draw uses draw-mode bits (`0x2004` plus later `0x800`) and selector branches, not a generic normalized float pipeline. | Keep the implementation compatible with material/draw flags; avoid hardcoding only `alpha=0.5` as the long-term representation. | `src/render/batch.rs`, unit atlas draw pipelines, future native blitter/material model. | A render fixture can distinguish normal opaque unit, warp 50% unit, and alternate `flags & 6` modes without conflating them with debug alpha or UI alpha. | `unit_sprite_material_preserves_warp_translucency_flag_mode` | Medium: simple alpha may pass a screenshot at one background but drift under palette/Z/remap parity. |

## Negative Facts / Do Not Do

- Do not keep the comment or behavior that "the unit itself stays fully opaque" for non-harvester teleport cooldown. Active in YR: No for that scenario; evidence is `TechnoClass::Draw @ 0x00706640` plus `TeleportLocomotionClass__StateMachineTick @ 0x007192F0`.
- Do not model the scoped teleport cooldown as the temporal weapon's gradual fade. Active in YR: No for self-teleport cooldown; evidence here is the binary branch ORing fixed flag bits, while temporal phase math is separate and out of scope.
- Do not apply the same post-warp translucency to chrono miners when the harvester branch clears `+0x271`. Active in YR: No for `WhatAmI()==1 && UnitType+0xE0E`.
- Do not treat the `WarpOut` animation rows as sufficient visual parity for teleport. Active in YR: No; the unit draw branch also changes while `+0x271` remains set.
- Do not collapse the native draw-mode bits into unrelated UI/debug alpha semantics. Active in YR: No; the VXL blitter selector consumes `flags & 6` and additional bits.

## Stale Docs

`docs/research/CHRONO_WARP_VISUAL_RENDERING.md` contains stale wording in section 6:

Old wording:

> The unit itself is NOT rendered with translucency during chrono teleport. The warp effect is purely the WarpOut animation overlay (the blue flash/shimmer of WARPOUT.shp).

Replacement wording:

> Self-teleport spawns the two `WarpOut` animation rows, but those rows are not the full unit visual. While `TechnoClass+0x271` (`BeingWarped`) remains set, `TechnoClass::Draw @ 0x00706640` ORs the ordinary unit draw flags with `0x2004` (later also applying the usual VXL `0x800` bit), so non-harvester teleporters draw with the native 50% translucent draw mode during the post-warp cooldown. The harvester special case clears `BeingWarped` immediately and therefore does not get that cooldown translucency.

## Remaining Uncertainty

- Exact final framebuffer math for the `flags & 6 == 4` VXL blitter was not pixel-captured in this slot. The binary selector path is verified, but display-format pixel equivalence needs a separate raster/blitter report or capture.
- The building-only `0x2006` alternate branch was identified but not investigated beyond the branch predicate because this slot is scoped to unit teleport rendering.
- SHP-body techno rendering may have adjacent flag handling, but current Rust-facing surface for this slot is voxel units in `src/app_instances/units.rs`.

## Source Index

- Ghidra decompile/read-only: `TechnoClass__Draw @ 0x00706640`.
- Ghidra assembly context/read-only: `0x00706694..0x00706757`.
- Ghidra decompile/read-only: `TechnoClass__IsWarpingOut @ 0x0070C5B0`.
- Ghidra decompile/read-only: `TechnoClass__IsBeingWarped @ 0x0070C5C0`.
- Ghidra decompile/read-only: `Blitter_selector_extended @ 0x00490E50`.
- Ghidra decompile/read-only: `VXL_CacheBlit @ 0x00707480`.
- Ghidra decompile/read-only: `TechnoClass__Render @ 0x00706ED0`.
- Ghidra decompile/read-only: `TeleportLocomotionClass__StateMachineTick @ 0x007192F0`.
- Ghidra assembly context/read-only: `0x00719579..0x007195B6`, `0x00719304..0x00719322`.
- Ghidra decompile/read-only: `TeleportLocomotionClass__TimerCheck @ 0x00719BF0`.
- Ghidra assembly context/read-only: `0x00719C12..0x00719C15`.
- Rust source checked: `src/app_instances/units.rs`, `src/sim/movement/teleport_movement.rs`.

