# ILocomotion Push/Shove Caller Provenance - Ghidra Research Report

Report: LOCOMOTION_PUSH_SHOVE_CALLER_PROVENANCE_GHIDRA_REPORT.md
Date: 2026-05-14
Scope: Static provenance audit for ILocomotion slot `+0x68` (`Push`) and slot `+0x6C` (`Shove`) callers, with emphasis on whether Hover's `Can_Enter_Cell(target, -1, -1, 0, 1)` branch is reached by standard YR gameplay.

## Summary

This audit found no binary-proven external caller that dispatches ILocomotion `Push` or `Shove` through `FootClass+0x674` in the scanned static code. The only true ILocomotion `Push` caller verified here is internal: `HoverLocomotionClass::Shove @ 0x00516FC0` calls Hover slot `+0x68` (`Push`) at `0x00516FCD`.

The Hover `height == -1` branch remains real code, but its gameplay liveness should be downgraded from "medium-high live-looking" to **conditional / no confirmed internal standard-YR caller**. If some path invokes Hover `Shove`, it reaches the branch. This pass did not prove such a path in gamemd's normal static call graph.

Important corrected takeaway:

```text
Hover Push branch exists and is bridge-sensitive.
Static internal caller provenance: only Hover Shove -> Hover Push.
External standard-YR trigger: not confirmed.
```

## Prior-State Check

Parent report: `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`.

Prior state row applied: explicit open question / targeted extension. The parent report identified `0x00516E9B` as Hover `Push` and left external `Push/Shove` caller provenance open. This report audits that open question.

No Rust code or implementation files were edited.

## Verified ILocomotion Slot Table

The canonical ILocomotion contract from `ILOCOMOTION_COM_PROTOCOL_SPEC.md` is:

| Slot | Offset | Method | Base fallback |
|---:|---:|---|---:|
| 26 | `+0x68` | `Push` | `0x0055AB70` |
| 27 | `+0x6C` | `Shove` | `0x0055AB80` |

Base implementations are stubs:

```asm
0055ab70  XOR AL,AL
0055ab72  RET 0x8

0055ab80  XOR AL,AL
0055ab82  RET 0x8
```

Direct xrefs to the base stubs are all DATA references from locomotor vtables; no code xref directly calls either stub.

## Concrete Locomotor Override Matrix

Raw vtable read from retail `gamemd.exe`:

| Class | ILocomotion vtable | Push slot `+0x68` | Shove slot `+0x6C` | Finding |
|---|---:|---:|---:|---|
| Base | `0x007EADF4` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Drive | `0x007E7EB0` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| DropPod | `0x007E8278` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Fly | `0x007E89F4` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Hover | `0x007EACFC` | `0x00516E10` | `0x00516FC0` | Only real override |
| Jumpjet | `0x007ECD68` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Mech | `0x007EDB6C` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Rocket | `0x007F0B1C` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Ship | `0x007F2D8C` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Teleport | `0x007F5000` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Tunnel | `0x007F5A24` | `0x0055AB70` | `0x0055AB80` | Stub/stub |
| Walk | `0x007F69F8` | `0x0055AB70` | `0x0055AB80` | Stub/stub |

Findings:

1. Hover is the only concrete locomotor in this vtable set that overrides `Push` or `Shove`.
2. Calling `Push/Shove` on Drive, Ship, Walk, Jumpjet, Fly, Teleport, Rocket, Mech, DropPod, or Tunnel returns false through the base stub.
3. Therefore any gameplay-relevant `Push/Shove` effect from these slots depends on the receiver being Hover.
4. Since Hover does not implement IPiggyback per the protocol matrix, no piggyback routing path was found that would redirect a non-Hover `Push/Shove` call into a hidden Hover implementation.

## True ILocomotion Caller Matrix

| Caller | Callsite | Receiver proof | Slot | Args | YR liveness |
|---|---:|---|---|---|---|
| `HoverLocomotionClass::Shove` | `0x00516FCD` | `ESI` is the ILocomotion pointer argument to `Shove`; vtable read from `[ESI]` | `+0x68 Push` | `Push(this=ESI, raw_arg=[Shove entry arg2])` | Conditional. Runs only if Hover `Shove` is externally invoked. No external caller proven in this audit. |

Verified assembly:

```asm
00516fc0  MOV  ECX,dword ptr [ESP + 0x8]   ; raw shove arg
00516fc4  PUSH ESI
00516fc5  MOV  ESI,dword ptr [ESP + 0x8]   ; ILocomotion* this
00516fc9  PUSH ECX                         ; Push arg2: raw shove arg
00516fca  PUSH ESI                         ; Push arg1: ILocomotion* this
00516fcb  MOV  EAX,dword ptr [ESI]
00516fcd  CALL dword ptr [EAX + 0x68]      ; ILocomotion::Push
00516fd0  TEST AL,AL
00516fd2  JZ   0x00517017                  ; Shove fails if Push fails
```

This is the only true ILocomotion `Push` caller verified in this pass.

## Negative Provenance Evidence

### Raw same-offset virtual calls are numerous but mostly irrelevant

A raw `.text` scan for short indirect calls through these offsets found:

| Pattern | Count |
|---|---:|
| `CALL [reg + 0x68]` | 180 |
| `CALL [reg + 0x6C]` | 64 |

These are not automatically ILocomotion calls. Many gamemd class vtables use the same byte offsets for unrelated methods.

### FootClass+0x674 receiver scan

A second scan traced local patterns where code loads a pointer from `+0x674`, reads its vtable, and calls through that vtable within the next 25 instructions.

Result:

| Result | Count |
|---|---:|
| `+0x674 -> vtable call` patterns found | 85 |
| Calls to slot `+0x68` from those patterns | 0 |
| Calls to slot `+0x6C` from those patterns | 0 |

Observed slot offsets from `+0x674` patterns included `+0x00`, `+0x04`, `+0x08`, `+0x0C`, `+0x10`, `+0x18`, `+0x24`, `+0x28`, `+0x2C`, `+0x30`, `+0x40`, `+0x44`, `+0x58`, `+0x5C`, `+0x60`, `+0x70`, `+0x74`, `+0x80`, `+0x84`, `+0x90`, and `+0xA0`. Neither `+0x68` nor `+0x6C` appeared in this provenance scan.

Caveat: this is static pattern evidence, not a formal whole-program proof. A caller could pass an ILocomotion pointer through a long helper chain and evade a simple local `+0x674` scan. However, combined with xrefs to the concrete Hover functions and the false-positive classification below, no standard internal caller was proven.

### Concrete Hover function xrefs

Ghidra xrefs:

| Target | Xrefs |
|---:|---|
| `HoverLocomotionClass::Push @ 0x00516E10` | Data xref from Hover vtable slot `0x007EAD64` only |
| `HoverLocomotionClass::Shove @ 0x00516FC0` | Data xref from Hover vtable slot `0x007EAD68` only |

There are no direct code xrefs to the concrete Hover methods. Virtual dispatch remains possible, but this reinforces that external callers must be found by receiver provenance, not direct xrefs.

## False-Positive Families

The following same-offset virtual calls were checked and classified as not ILocomotion `Push/Shove`.

| Address / family | Offset | Evidence | Classification |
|---:|---:|---|---|
| `0x0070F1E0`, `0x0070665B`, many Techno/Object draw sites | `+0x68` | Pattern `push 0; push 0; mov ecx,this; call [vtable+0x68]`; existing docs identify `vtable+0x68` as `GetVisualState`/draw mode. | Render/visual-state, not locomotion. |
| `0x0043FA82`, `0x004519CD`, `0x00451FB8`, `0x00456EFC`, etc. | `+0x68` | Building/Techno visual-state pattern checks cloak stage byte `+0x6ED == 0xF`, passes `(0,0)` with `ECX=this`. | Building/Techno visual-state calls, not locomotion. |
| `0x00692B1D` | `+0x6C` | Receiver is `DAT_00A8E334[DAT_008809A0]`; `DISPLAYCLASS_GHIDRA_REPORT.md` identifies this as superweapon/UIMode targeting override returning an action code. | SuperWeaponType/UIMode handler, not locomotion. |
| `0x0048E62E`, `0x004E1587`, `0x004E185F`, `0x004E18AD` | `+0x6C` | `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md` identifies Gadget slot `+0x6C` as `Draw_Me(forced)`. | UI gadget draw, not locomotion. |
| `0x004BAD3E`, `0x004BADB9`, `0x004BAF70`, `0x004BB035`, `0x004BB22F`, `0x004BB6B0` | `+0x68/+0x6C` | Receiver comes from fields such as `[obj+0x1C]`; adjacent calls use slot `+0x60` and compare result to `0x887601C2`, with globals such as `0x00887628`. No `Foot+0x674`; pattern is DirectDraw/graphics-interface-like. | Graphics/COM surface family, not locomotion. |
| `0x0073C5FF` | `+0x6C` | Function starts with `ECX` as Unit/Object pointer and calls `[this_vtable+0x6C]`; nearby `+0x674` loads are render-context incidental. | Unit/Object render virtual, not locomotion. |
| `0x00728B78` | `+0x6C` | Inside Tunnel/coord-state code, receiver is result of another object virtual call (`call [eax+0x88]`), with `ECX=EAX`; no `Foot+0x674` receiver. | Class-local/object helper, not locomotion. |

Useful discriminator:

- ILocomotion COM methods pass `this` on the stack and return with `RET 0x8` for `Push/Shove`.
- Most false positives are C++ thiscall patterns with `ECX=this`, even when they also push two method arguments.
- The presence of two pushed arguments alone is insufficient. Many Techno/Object `vtable+0x68` calls push `(0,0)` for visual-state, not locomotor `Push`.

## Impact On Hover height == -1 Branch

Parent report verified Hover `Push @ 0x00516E10` calls Unit/Foot `Can_Enter_Cell` at `0x00516E9B` with:

```text
(target_adjacent_cell, direction=-1, height=-1, parent/current-cell=0, arg5=1)
```

This report changes only the trigger confidence:

| Claim | Status after this audit |
|---|---|
| Hover branch exists | Confirmed high |
| Hover branch exact Can_Enter_Cell args | Confirmed high by parent report |
| Hover `Shove` reaches Hover `Push` | Confirmed high |
| Any non-Hover locomotor has meaningful `Push/Shove` | Refuted for known concrete locomotor vtables; they inherit false stubs |
| External internal gamemd caller through `FootClass+0x674` | Not found |
| Standard YR gameplay trigger | Still unconfirmed; lower confidence than parent matrix implied |

Player-visible implication:

- If standard YR never invokes Hover `Push/Shove`, the `height == -1` Hover bridge branch is dormant despite being present in live Hover code.
- If a hidden or nonlocal caller does invoke Hover `Shove`, the bridge-sensitive behavior is exactly the parent report's `Can_Enter_Cell(target, -1, -1, 0, 1)` path.
- A future Rust port does not need to prioritize this branch as normal hover movement, but the runtime `Can_Enter_Cell` API still should be able to express it because Jumpjet landing/abort and this Hover method both demonstrate real binary call shapes with `height == -1`.

## Answered Questions

1. Which callsites dispatch through `FootClass+0x674` to slot `+0x68` or `+0x6C`?

No such callsite was found by the static local provenance scan. 85 nearby `+0x674 -> vtable call` patterns were found, but none used slot `+0x68` or `+0x6C`.

2. What exact args do real ILocomotion `Push/Shove` callers pass?

Only one real caller was verified: Hover `Shove` calls Hover `Push` with `(this=Hover ILocomotion*, raw_arg=Shove arg2)`. The parent report verifies what `Push` then passes to `Can_Enter_Cell`.

3. Are they live in standard YR?

Hover locomotion is live in standard YR, but the caller path is conditional. No external internal standard-YR caller to `Shove` or `Push` was proven.

4. Can the receiver be Hover locomotion?

The only meaningful receiver for these slots is Hover, because every other audited concrete locomotor inherits the false-returning base stubs. If a generic ILocomotion `Push/Shove` caller exists, Hover is the only class where it can succeed.

5. Are triggers collision, rocker/direct-rocker, chrono displacement, terrain correction, scatter, or something else?

No trigger from those systems was proven in this pass. Existing scatter evidence routes through Object `Scatter` and destination movement, not ILocomotion `Push/Shove`. Rocker/direct-rocker docs point to Techno body rocking (`ApplyRocker`) rather than locomotor `Push/Shove`. Chrono/piggyback docs use IPiggyback and destination routing, not these slots.

## Rust Parity Guidance

No implementation changes were made.

Future implementation should treat Hover `Push/Shove` as a low-priority conditional parity branch until a runtime trigger is proven. However, the runtime movement/collision entry API should still preserve the ability to represent:

```text
Can_Enter_Cell(target, direction=-1, height=-1, parent=null, arg5=1)
```

Reasons:

1. The Hover branch is present in binary and has exact verified arguments.
2. Jumpjet runtime landing/abort checks also use `direction == -1` and `height == -1` per the bridge callsite matrix.
3. A future runtime breakpoint may prove a Hover caller; the API shape should not make that impossible to model.
4. Sim code must preserve the project invariant: no dependency from `sim/` to render/ui/sidebar/audio/net.

## Confidence

High confidence:

- Hover is the only concrete locomotor override for ILocomotion `Push/Shove` among the audited vtables.
- Base `Push/Shove` stubs return false and have no code xrefs.
- Hover `Shove` internally calls Hover `Push` at `0x00516FCD` and passes the same raw arg.
- Raw same-offset call counts are 180 for `+0x68` and 64 for `+0x6C`; these include many unrelated vtable families.
- The local `FootClass+0x674` provenance scan found zero `+0x68/+0x6C` calls.

Medium confidence:

- No internal standard-YR external caller exists. This is based on static pattern scans and false-positive classification, not a formal whole-program dataflow proof.

Open:

- Runtime breakpoint confirmation on `0x00516E10` and `0x00516FC0` in live YR scenarios.
- Long-range provenance where an ILocomotion pointer is copied far from `Foot+0x674` before a virtual call. This pass did not prove such a chain.

## Recommended Next Verification

Use a runtime breakpoint or tracepoint on:

```text
0x00516E10  HoverLocomotionClass::Push
0x00516FC0  HoverLocomotionClass::Shove
```

Test scenarios:

1. Hover units blocked by other ground/hover units.
2. Hover units near bridge deck/ground transitions.
3. Hover units hit by Rocker/DirectRocker warheads.
4. Chrono displacement and teleport-adjacent interactions.
5. Cell evacuation/scatter from explosions, deploys, and bridge damage.

If neither breakpoint fires, classify Hover `Push/Shove` as dormant in normal YR gameplay. If it fires, capture caller return address and re-open this report with exact trigger conditions.

## Sources

- Ghidra assembly / xrefs, `gamemd.exe`:
  - `LocomotionClass__Push @ 0x0055AB70`
  - `LocomotionClass__Shove @ 0x0055AB80`
  - `HoverLocomotionClass::Push @ 0x00516E10`
  - `HoverLocomotionClass::Shove @ 0x00516FC0`
  - Hover `Shove -> Push` callsite `0x00516FCD`
  - Hover vtable `0x007EACFC`
- Raw retail `gamemd.exe` PE/vtable scan for concrete locomotor vtables.
- Existing research docs:
  - `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`
  - `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
  - `ILOCOMOTION_COM_PROTOCOL_SPEC.md`
  - `FOOTCLASS_AI_GHIDRA_REPORT.md`
  - `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`
  - `DISPLAYCLASS_GHIDRA_REPORT.md`
  - `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`
  - `ZBUFFER_DEPTH_SYSTEM.md`
  - `BODY_ROCKING_GHIDRA_REPORT.md`
