# Mind Control Link Lines DrawLinks - Ghidra Research Report

**Address(es):** `CaptureManagerClass::DrawLinks @ 0x00472160`, `CaptureManagerClass::ShouldDrawLinks @ 0x00472640`, line helper `0x00704E40`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Mind-control link line rendering only: CaptureManager node iteration, DrawLinks/ShouldDrawLinks gates, endpoints, color, line helper, tactical draw-order placement, and standard YR activity.  
**Non-Scope:** Capture eligibility, ownership transfer semantics beyond MCNode fields needed by the line renderer, Mastermind overload AI/damage, Psychic Dominator permanent MC, and full surface vtable raster internals behind `DAT_0088731C`.  
**Confidence:** High for the decompiled functions and draw-order sites; Medium for INI parser names inherited from prior docs where this slice only re-verified the runtime consumer.  
**Active in YR:** Yes. Standard YR content uses mind-control weapons and warheads (`ini/rulesmd.ini:5206`, `ini/rulesmd.ini:8643`, `ini/rulesmd.ini:13690`, `ini/rulesmd.ini:27127`, `ini/rulesmd.ini:27132`) and the tactical draw loop calls this renderer from active render pass sites `0x006D47BF` and `0x006D47F6`.

## 1. Overview

Mind-control link lines are persistent relationship overlays from a controller's `CaptureManagerClass` to controlled technos. They are not selected-unit action lines and are not governed by `[Options] UnitActionLines`; they are drawn from `TacticalClass_Draw` after selected action/Psychic Sensor action lines and before the later service/tether/airstrike-style overlay in the same techno loop.

The important correction against older shorthand is that this slice did not find an "on screen" gate in `ShouldDrawLinks` or `DrawLinks`. The verified gates are selected state and the per-link `MindControlAttackLineFrames` timer; viewport clipping happens later in the line helper.

## 2. Class Layout / Key Offsets

| Structure | Offset | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `CaptureManagerClass` | `+0x28` | DynamicVector data pointer, array of `MCNode*` | Yes | DrawLinks reads `*(this+0x28) + index*4` at `0x0047218E..0x00472193`; constructor sets vector vtable at `0x004717D0`. |
| `CaptureManagerClass` | `+0x34` | node count | Yes | DrawLinks initializes reverse index from `this+0x34`; ShouldDrawLinks does same. |
| `CaptureManagerClass` | `+0x48` | controller/owner `TechnoClass*` | Yes | DrawLinks reads controller selected byte and FLH/color source from `this+0x48`; constructor stores owner at `0x004717D0`. |
| `MCNode` | `+0x00` | controlled victim `TechnoClass*` | Yes | CaptureUnit writes victim at `0x00471D40`; DrawLinks dereferences node[0]. |
| `MCNode` | `+0x08` | capture frame, `-1` means timer never ages | Conditional | DrawLinks special-cases `-1` at `0x0047219E..0x004721B1`; normal CaptureUnit writes current frame. |
| `MCNode` | `+0x10` | link visible frame budget | Yes | CaptureUnit writes `RulesClass+0x310`; DrawLinks and ShouldDrawLinks read node[4]. |
| `TechnoClass` | `+0x83` | selected/render-selection byte used by these gates | Conditional | DrawLinks reads controller/victim selected bytes; ShouldDrawLinks also reads controller/transport/victim selected bytes. |
| `TechnoClass` | `+0x2BC` | controller's `CaptureManagerClass*` | Yes | Tactical draw calls own manager at `0x006D47A6..0x006D47BF`. |
| `TechnoClass` | `+0x2C0` | victim's `MindControlledBy` controller pointer | Yes | Tactical draw second branch follows current techno's `+0x2C0` to controller manager at `0x006D47CB..0x006D47F6`. |
| `TechnoTypeClass` | `+0x3DC` | victim endpoint Z offset for link line | Yes | DrawLinks calls victim type getter (`vtable+0x84`) then adds `type+0x3DC` to victim Z. |
| `HouseClass` | `+0x56F9..+0x56FB` | RGB bytes for link color | Yes | DrawLinks reads controller house through `controller[0x87] + 0x56F9`. |

## 3. Core Logic

`ShouldDrawLinks @ 0x00472640` is a cheap pre-gate used by `TacticalClass_Draw`:

1. If controller `Techno+0x83` is selected, return true.
2. Else if controller `Techno+0x11C` is non-null and that pointed techno has `+0x83` selected, return true. This is the "controller's transport/host selected" style gate in prior docs; this slice did not expand the field beyond the immediate use.
3. Else iterate MCNodes in reverse order.
4. For each node, return true if the victim `Techno+0x83` is selected.
5. Else evaluate the link timer:
   - If `capture_frame == -1`, the timer condition is true only when `link_visible_frames > 0`.
   - Otherwise compute `current_frame - capture_frame`; if that age is less than `link_visible_frames`, compute remaining frames and return true if remaining is positive.
6. If no node passes, return false.

`DrawLinks @ 0x00472160` repeats the per-node timer/selection evaluation rather than trusting only the pre-gate:

1. Cache controller selected byte from `manager+0x48 -> Techno+0x83`.
2. Iterate `nodes_count - 1` down to `0`; reverse order is verified.
3. For each node, compute `bShouldDrawNode` from the same timer rule, then OR in victim selected byte.
4. Draw only if controller pointer is non-null, victim pointer is non-null, and either the controller was selected or this node passed the victim/timer gate.
5. Victim endpoint starts from `victim+0x9C/+0xA0/+0xA4` (`piVar4[0x27..0x29]`) and adds `victim_type+0x3DC` to Z.
6. Controller endpoint is computed by `TechnoClass::GetFLH @ 0x006F3AD0`, passed a negative index `-1 - (node_index % 5)` with zero extra offsets. In `GetFLH`, negative indices `-1..-5` select `TechnoType+0x850`, `+0x85C`, `+0x868`, `+0x874`, `+0x880`; standard `[MIND]` supplies five `AlternateFLH0..4` entries in `ini/artmd.ini:642..646`.
7. Color is built from the controller's house RGB bytes at `House+0x56F9..+0x56FB`.
8. The final draw call is `FUN_00704E40(start_x,start_y,start_z,end_x,end_y,end_z,color)` at `0x00472282`.

`FUN_00704E40 @ 0x00704E40` is the link-line helper for this path:

1. Project both 3D endpoints through `TacticalClass__CoordsToClient2`.
2. Add `g_RadarViewportOffsetY` to the projected Y values.
3. Draw clipped `3x3` endpoint boxes offset by `(-2,-2)` at both projected endpoints using `DAT_0088731C` surface vtable `+0x14`.
4. Build a viewport clip rect from `g_RadarViewportOffsetX/Y/Width/Height`.
5. Use `timeGetTime()` with a phase derived as `(timeGetTime() >> 4) << 3`.
6. Iterate 32 subsegments in 1/32 increments (`8,16,...,256`), clipping each segment with `FUN_007BC2B0` and drawing visible segments through surface vtable `+0x30`.
7. The helper is therefore not the same as selected-unit `ActionLines_DrawLine @ 0x007049C0`; it has endpoint boxes plus animated segmented body drawing.

## 4. INI Keys

| Key | Default / value observed | Runtime use in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `[CombatDamage] MindControlAttackLineFrames` | `20` | Stored in MCNode `+0x10`; DrawLinks/ShouldDrawLinks compare it against `g_CurrentFrameCounter - capture_frame`. | Yes | INI `ini/rulesmd.ini:853`; parser write to `RulesClass+0x310` at `0x0066C92E`; CaptureUnit copies `RulesClass+0x310` into node[4]. |
| `MindControl=yes` on warheads | `Controller`, `ControllerBuilding` | Makes standard YR units/buildings create CaptureManager links upstream of this renderer. | Yes | `ini/rulesmd.ini:27127`, `ini/rulesmd.ini:27132`; prior docs trace warhead dispatch, this report only needs YR activity. |
| `AlternateFLH0..4` on `[MIND]` art | `0,25,90`, `0,-25,90`, `-50,25,90`, `-50,-25,90`, `-25,0,90` | `GetFLH` negative indices select five type FLH slots for link origin scatter. | Yes for Master Mind visuals | `ini/artmd.ini:642..646`; DrawLinks passes `-1 - index%5`; `GetFLH @ 0x006F3AD0` negative-index branch reads five 12-byte slots. |

No `[Options] UnitActionLines` read appears in this path. That option gates selected-unit action lines at `0x006D473F..0x006D4750`, not CaptureManager links.

## 5. Integration Points

`TacticalClass_Draw @ 0x006D3D10` calls mind-control link rendering from the same per-techno overlay loop that handles action lines:

1. For non-human visible/Psychic Sensor-eligible technos, `DrawRadarActionLines` can run at `0x006D478E`.
2. For human selected technos, selected action/target lines can run at vtable `+0x438` from `0x006D473F..0x006D4750`.
3. After those branches, the loop checks the current techno's own CaptureManager: `Techno+0x2BC`, `ShouldDrawLinks`, then `DrawLinks` at `0x006D47BF`.
4. It then checks whether the current techno is a controlled victim: `Techno+0x2C0 -> controller -> +0x2BC`, `ShouldDrawLinks`, then `DrawLinks` at `0x006D47F6`.
5. The airstrike/service/tether-like line block begins after this at `0x006D47FB..0x006D48FA`, so mind-control link calls precede that block in the same loop.

There is no dedupe guard in the verified local code around the two DrawLinks call sites. If the loop reaches both a controller and its victims while `ShouldDrawLinks` is true, the same manager can be submitted more than once; the helper writes the same colored pixels, so the visible effect may be idempotent on the target surface. Runtime overdraw visibility was not separately measured.

## 6. Current Rust Implementation Status

The current Rust tree parses some upstream flags but does not implement this renderer:

| Area | Status | Evidence |
|---|---|---|
| Warhead parser has `mind_control` | partial upstream parse | `src/rules/warhead_type.rs` contains `pub mind_control` and reads `MindControl`. |
| Weapon parser has `infinite_mind_control` | partial upstream parse | `src/rules/weapon_type.rs` reads `InfiniteMindControl`. |
| Selected-unit target/action lines exist | separate system only | `src/app_target_lines.rs` builds command lines from app commands. |
| CaptureManager state / MCNode list / DrawLinks equivalent | missing in this scan | `rg` found no `CaptureManager`, `DrawLinks`, `MindControlAttackLineFrames`, or MC link rendering implementation under `src/`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CaptureManagerClass::DrawLinks @ 0x00472160` | verified | direct decompile and assembly context around `0x00472160..0x00472292` | none for this slice |
| `CaptureManagerClass::ShouldDrawLinks @ 0x00472640` | verified | direct decompile; xrefs only from `0x006D47B0`, `0x006D47E1` | none for this slice |
| `FUN_00704E40` link helper | verified for caller-visible behavior | direct decompile and xref from `0x00472282` | internal surface vtable implementation behind `+0x14/+0x30` not expanded |
| Tactical draw order | verified | `TacticalClass_Draw @ 0x006D3D10`; call sites `0x006D47BF`, `0x006D47F6` | none for requested relative order |
| MCNode creation of timer fields | verified | `CaptureUnit @ 0x00471D40` writes capture frame and `RulesClass+0x310` into node | none for line timer |
| `TechnoClass::GetFLH @ 0x006F3AD0` negative index use | verified for negative-index branch | direct decompile; DrawLinks passes `-1 - index%5` | parser mapping of all art-side FLH keys not re-expanded |
| Standard YR activity | verified | rules/art lines for YURI/MIND/YAPSYT and warheads; tactical draw active call sites | none for this slice |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does DrawLinks iterate controlled units forward or reverse? Reverse from `nodes_count - 1` to `0`. Evidence: `0x00472180..0x00472193`, decrement at `0x00472287`.

[RESOLVED] OQ-2 - What makes links visible? Controller selected, controller host/transport selected via `+0x11C`, victim selected, or per-node timer remaining. Evidence: `ShouldDrawLinks @ 0x00472640`; `DrawLinks @ 0x00472160`.

[RESOLVED] OQ-3 - Is older "on-screen visibility" wording verified? No. This slice found selected-state and timer gates; no viewport visibility predicate appears before line-helper clipping. Evidence: `0x00472640`, `0x00472160`, line clipping in `0x00704E40`.

[RESOLVED] OQ-4 - What are endpoints? Victim raw coords plus `TechnoType+0x3DC` Z offset; controller `GetFLH` with negative slot `-1 - index%5`. Evidence: `0x00472220..0x00472282`, `0x006F3AD0`.

[RESOLVED] OQ-5 - What is the color source? Controller house RGB bytes at `House+0x56F9..+0x56FB`. Evidence: `DrawLinks @ 0x00472236` decompile expression.

[RESOLVED] OQ-6 - Which renderer draws the line? `FUN_00704E40`, not selected-unit `ActionLines_DrawLine`. Evidence: direct call at `0x00472282`, xrefs to `0x00704E40`.

[RESOLVED] OQ-7 - Where does this draw relative to other line families? After selected/Psychic Sensor action lines and before the later service/tether/airstrike-style block. Evidence: `TacticalClass_Draw @ 0x006D473F..0x006D48FA`.

[DEFERRED] OQ-8 - Does repeated manager submission from controller/victim loop positions ever produce visible overdraw? Category: needs-runtime-debugger. The local binary has no dedupe guard, but pixel idempotence depends on the surface helper behavior and current clip visibility.

## Sources

- Ghidra decompiled: `0x00472160`, `0x00472640`, `0x00704E40`, `0x006D3D10`, `0x00471D40`, `0x004717D0`, `0x006F3AD0`, `0x0066BBB0`.
- Ghidra xrefs: `DrawLinks` called from `0x006D47BF`, `0x006D47F6`; `ShouldDrawLinks` called from `0x006D47B0`, `0x006D47E1`; `0x00704E40` called from `0x00472282`.
- Prior context read, not trusted without verification: `MIND_CONTROL_GHIDRA_REPORT.md`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`, `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`.
- INI evidence: `ini/rulesmd.ini:853`, `ini/rulesmd.ini:5206`, `ini/rulesmd.ini:8643`, `ini/rulesmd.ini:13690`, `ini/rulesmd.ini:27127`, `ini/rulesmd.ini:27132`, `ini/artmd.ini:642..646`.
- Rust scan: `src/rules/warhead_type.rs`, `src/rules/weapon_type.rs`, `src/app_target_lines.rs`.
