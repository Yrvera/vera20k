# TechnoClass+0x431 Hover Health Flag And Building Pips

Target: TechnoClass+0x431 hover health flag and standard YR building health pips.

Scope: read-only Ghidra decompilation/assembly context only. No Rust, INI, or existing
research-doc edits.

Status: COMPLETE.

## Summary

`TechnoClass+0x431` is the one-frame cursor-hover health flag. Standard YR initializes
it to `0`, sets it from `DisplayClass::SetCursorFromAction` for the filtered object
under the cursor, clears it at the start of `TechnoClass::AI_Update`, and consumes it
in `TechnoClass::DrawExtras`.

Building health pips do not draw merely because a building is damaged. In the verified
standard render path, `DrawExtras` calls the health-bar slot for selected objects, and
also for non-selected objects whose `+0x431` hover flag is set. The building branch of
`TechnoClass::DrawHealthBar` then draws the diagonal `PIPS.SHP` health pips.

Active in YR: Yes for selected buildings and hover targets. No for damaged
non-selected/non-hover buildings in the verified `DrawExtras` render path.

## Verified Binary Evidence

### 1. Constructor initializes the flag to zero

Verified binary finding: `TechnoClass::Constructor @ 0x006F2B40` initializes
`TechnoClass+0x431` to `0`.

Evidence:
- Decompile of `0x006F2B40` contains `*(undefined1 *)((int)param_1 + 0x431) = 0`.
- Assembly context at `0x006F301D`: `MOV byte ptr [ESI + 0x431],BL`, surrounded by
  adjacent byte initializers for `+0x430` and `+0x432`; `BL` is the zero initializer in
  this constructor block.

Active in YR: Yes. This is the live `TechnoClass` construction path.

### 2. Cursor/action update sets the hover flag

Verified binary finding: `DisplayClass::SetCursorFromAction @ 0x004AAE90` sets
`filteredObject+0x431 = 1` when a target object is present, `Filter_AbstractType_InMap`
returns an object, the map editor global is false, and either the object is not a
building or its type byte `+0x1701` is false.

Evidence:
- Decompile of `0x004AAE90`: after `Filter_AbstractType_InMap()`, the function calls
  vtable `+0x2C` (`WhatAmI`) and then writes
  `*(undefined1 *)((int)piVar5 + 0x431) = 1`.
- Assembly context at `0x004AAEEC`: `MOV byte ptr [EDI + 0x431],0x1`.
- Nearby assembly confirms gates: `CALL dword ptr [EDX + 0x2c]`, `CMP EAX,0x6`,
  type read `MOV CL,byte ptr [EAX + 0x1701]`, and `MOV AL,[0x00a8ed6b]` before the
  write.

Active in YR: Yes. This is the standard cursor-shape/action update path, not a TS-only
fog path. The map-editor global disables the write in editor context.

Inference from verified code: for ordinary buildings, the hover flag is set when the
building passes the above gate; buildings with type byte `+0x1701 != 0` are excluded
from this setter. The exact semantic name of `+0x1701` is outside this slot.

### 3. AI update clears the flag

Verified binary finding: `TechnoClass::AI_Update @ 0x006F9E50` clears `+0x431` if set.

Evidence:
- Decompile begins with:
  `if (param_1->field_0x431 != '\0') { param_1->field_0x431 = 0; }`.
- Assembly context at `0x006F9E5B..0x006F9E65` reads `MOV AL,byte ptr [ESI + 0x431]`,
  tests it, then executes `MOV byte ptr [ESI + 0x431],0x0`.

Active in YR: Yes. This is the inherited per-techno AI update used by standard
buildings through their normal update chain.

### 4. DrawExtras consumes the hover flag after the selected path

Verified binary finding: `TechnoClass::DrawExtras @ 0x006F5190` calls the health-bar
slot for selected objects, then separately checks `+0x431` and `+0x83 == 0` to call the
same health-bar slot for non-selected hover targets.

Evidence:
- Selected path: decompile checks `*(char *)((int)param_1 + 0x83) != '\0'`, and later
  calls vtable `+0x44C`.
- Hover path: decompile checks
  `(*(char *)((int)param_1 + 0x431) != '\0') &&
   (*(char *)((int)param_1 + 0x83) == '\0')`, then calls vtable `+0x44C`.
- Assembly context at `0x006F5E37..0x006F5E57` reads `MOV AL,byte ptr [EBP + 0x431]`,
  tests selection byte `+0x83`, and reaches the call sequence through vtable `+0xC8`
  / `+0xD0` disguise handling before the health-bar slot call.

Active in YR: Yes. This is reached from the standard tactical object render loop's
`vtable+0x110` extras calls.

### 5. Building health pips draw only through selected or hover callers in this path

Verified binary finding: `TechnoClass::DrawHealthBar @ 0x006F64A0` draws building
health pips when `WhatAmI()==6`, but it does not decide selection, hover, or damaged
visibility. Those gates are its callers in `DrawExtras`.

Evidence:
- Decompile of `0x006F64A0` enters the building branch on vtable `+0x2C == 6`, computes
  building dimensions via type vtable `+0x7C`, computes health ratio/color, and calls
  `CC_Draw_Shape(DAT_00ac147c, frame, ..., flags 0x600, ..., 1000, ...)` for filled
  and empty pips.
- No selected-byte or hover-byte gate appears before the building `PIPS.SHP` draw
  loops in `0x006F64A0`; selected/hover gating is upstream in `0x006F5190`.
- `Tactical_ObjectRenderingLoop @ 0x006D8DB0` reaches overlays through object
  vtable `+0x110` (`DrawExtras`) in both first visible-object processing and the later
  extras pass; the checked render loop does not directly call the health-bar slot for
  arbitrary damaged buildings.

Active in YR: Yes for selected buildings and hover targets. No for merely damaged,
non-selected, non-hover buildings in the verified standard render path.

## Answer To Target Questions

What sets `TechnoClass+0x431`:
- `DisplayClass::SetCursorFromAction @ 0x004AAE90` sets it to `1` on the filtered
  cursor target, with the gates listed above.

What clears `TechnoClass+0x431`:
- `TechnoClass::Constructor @ 0x006F2B40` initializes it to `0`.
- `TechnoClass::AI_Update @ 0x006F9E50` clears it to `0` at the start of the update if
  it was set.

How hover health reaches `DrawExtras`:
- The tactical render loop calls object `vtable+0x110`; for buildings that resolves to
  `TechnoClass::DrawExtras @ 0x006F5190`.
- `DrawExtras` checks `+0x431 != 0` and `+0x83 == 0`, handles disguise exposure, then
  calls vtable `+0x44C`, which is `TechnoClass::DrawHealthBar @ 0x006F64A0` in the
  verified building path.

Do building health pips draw for damaged non-selected buildings, selected buildings,
or hover targets in standard YR:
- Selected buildings: Yes. `DrawExtras` selected path calls `DrawHealthBar`.
- Hover targets: Yes, when `+0x431` is set and the object is not selected.
- Damaged non-selected/non-hover buildings: No in the verified standard `DrawExtras`
  render path. Damage affects pip count/color once `DrawHealthBar` is called; it does
  not independently call the building-pip drawing path.

## Open Questions

- Whether the building type byte `+0x1701` is best named as an EMP/offline/special
  exclusion flag is outside this slot; this report only verifies that it gates the
  cursor-hover setter for buildings.
- A whole-program xref export could further enumerate every syntactic reference to
  `+0x431`; this slot verified the constructor, cursor setter, AI clearer, and
  `DrawExtras` consumer needed for the player-visible hover-health path.

## Sources Checked

- Ghidra decompile: `0x004AAE90`, `0x006D8DB0`, `0x006F2B40`, `0x006F5190`,
  `0x006F64A0`, `0x006F9E50`.
- Ghidra assembly context: `0x004AAEEC`, `0x006F301D`, `0x006F5E37..0x006F5E57`,
  `0x006F9E5B..0x006F9E65`.
