---
title: FootClass Re-Investigation (extends prior reports)
date: 2026-04-24
scope: gamemd.exe — verifies and extends FOOTCLASS_COMPLETE_GHIDRA_REPORT.md, FOOTCLASS_STRUCT_LAYOUT.md, FOOTCLASS_NON_MOVEMENT_FIELDS.md, and 9 sibling FootClass reports
confidence: HIGH for newly resolved findings (verified from binary), MEDIUM for partially-resolved
active_in_yr: Yes — base class for all mobile units
---

# FootClass — Re-Investigation Report (2026-04-24)

## 0. Why this report exists

`/re-investigate FootClass` was run on a system that already has 13+ standalone reports
(820+ line FOOTCLASS_COMPLETE_GHIDRA_REPORT, 340+ line FOOTCLASS_STRUCT_LAYOUT, plus
9 mission/AI/vtable/non-movement docs). The bulk of FootClass is well-documented.

This report does not duplicate that work. It exists to:
1. **Resolve open questions** carried in the prior reports (especially `0x694`).
2. **Reconcile conflicts** between FOOTCLASS_STRUCT_LAYOUT.md and FOOTCLASS_NON_MOVEMENT_FIELDS.md
   (different names assigned to the same bytes — `0x6AC`, `0x6AD`, `0x6AE`).
3. **Re-verify** key constructor and core-method claims by re-decompiling them with fresh eyes.

For the full struct layout, vtable map, mission handlers, and INI key surface, refer to the
prior reports listed above. This document only adds, corrects, or confirms.

---

## 1. Newly resolved: byte 0x694 (`LargeObjectPtr` → **ParasiteHost back-pointer**)

### 1.1 The prior open question

Prior reports labelled this `Unknown_Ptr_0x694` / `LargeObjectPtr` and noted only that
`FootClass::AI` dispatches through it:

```c
if (param_1[0x1a5] != 0) {                                  // P694 = this->[0x694]
    (**(code **)(**(int **)(param_1[0x1a5] + 0x69c) + 0x5c))();
    //          ^^ vtable      ^^ subobj@+0x69C    ^^ slot 0x5C = AI
}
```

The puzzle was that the dispatch goes through `(P694)+0x69C` — meaning P694 must be a
struct large enough to have a sub-object at offset 0x69C. The reports concluded "not a
TeamClass\* (TeamClass is only 0xA0 bytes)" and left the identity unresolved.

### 1.2 Resolution — verified from `TechnoClass::Init_Managers` (0x006F3F40)

```c
// In Init_Managers, gated by weapon warhead Parasite=yes flag:
if (*(char *)(*(int *)(*piVar3 + 0xac) + 0x159) != '\0') {
    pvVar2 = operator_new(0x58);              // 0x58 bytes — ParasiteClass size
    iVar1 = ParasiteClass__Constructor(param_1);
    param_1[0x1a7] = iVar1;                   // store at *this+0x69C
}
```

`ParasiteClass*` is allocated and stored at the **infesting unit's own** `+0x69C`
(only when the unit's primary weapon warhead has `Parasite=yes` — Terror Drone, etc.).

The dispatch `(*(P694)+0x69C)` therefore reads the parasite component of *another* FootClass.
For this to be valid, **`FootClass+0x694` must point to another FootClass-derived unit
that owns a ParasiteClass at its own +0x69C** — i.e., the parasite-infestor.

### 1.3 Field semantics (HIGH confidence)

> **`FootClass+0x694` = `ParasiteHost*` back-pointer**. When a FootClass unit is being
> infested by a parasite (Terror Drone, etc.), this field on the **host** is set to point
> at the **infesting unit**. Each tick, the host's `FootClass::AI` dereferences
> `infestor->ParasiteClass*` (at `infestor+0x69C`) and ticks `ParasiteClass::AI`, which
> applies the per-tick infestation damage to the host.

### 1.4 Lifecycle (verified from binary)

| Operation | Address | Effect on `host->[0x694]` |
|-----------|---------|---------------------------|
| Constructor | `0x4D33E4` | initialized to 0 |
| `FootClass::PointerExpired` | `0x4D99A4` | cleared if `param_2 == this->[0x694]` AND (`P694->[0x6C] == 0` OR `DAT_00a8ed5c == 0`) |
| `FootClass::PointerExpired` | `0x4D99CD` | cleared on self-deletion (`param_2 == this`) |
| `WarpAttachClass::Detach` | `0x62A2CF`, `0x62AB2B` | also clears (defensive — when chrono detaches, ensures any linked back-pointer is also dropped) |

`PointerExpired` additionally **forwards** the pointer-expiration event into the
infestor's parasite component: when `host->[0x694]` is non-null and `P694->[0x6C] > 0`,
it calls `(*(P694+0x69C))->vtable[0x28](param_2, 1)` — i.e., propagates the
"this pointer just died" notification down to `ParasiteClass`.

### 1.5 Damage interaction — `FootClass::ReceiveDamage` (`0x4D7330`)

`ReceiveDamage` reads `this->[0x694]` to redirect excess damage into a damage-tracking
buffer at `(P694)+0x69C+0x2C/+0x30/+0x34`:

```c
piVar1 = *(int **)&param_1[1].field_0x174;                // P694
if ((piVar1 != 0) && (param_5 != piVar1) &&
    (iVar4 = (**(code **)(*piVar1 + 0x84))(),             // P694->GetType()
     *(int *)(iVar4 + 0xd6c) < *param_2)) {               // damage > TypeClass+0xD6C
    iVar2 = *(int *)(*(int *)&param_1[1].field_0x174 + 0x69c);  // ParasiteClass*
    *(undefined4 *)(iVar2 + 0x2c) = g_CurrentFrameCounter;
    *(undefined4 *)(iVar2 + 0x30) = uStack_8;             // location
    *(int *)(iVar2 + 0x34) = iVar4 * 2 - iVar5;           // redirected damage
}
```

Translation: when the host takes damage greater than the infestor TypeClass's threshold
(at `+0xD6C`), the excess is recorded in the infestor's parasite buffer. This is how
killing the infested host can kill the infestor too.

### 1.6 Open follow-up

- The exact instruction that *sets* `host->[0x694]` (the "begin infestation" write) was
  not located via byte-pattern search. The set must occur via an indirection (e.g., LEA
  + MOV through a different register) that escapes the `89 ?? 94 06 00 00` pattern, or
  via a memory copy in save/load. Only **clears** were found by direct pattern search.
  The most likely setter is a `ParasiteClass::Attach`-style method invoked from the
  Terror Drone's weapon-fire path.

---

## 2. Reconciliation: bytes `0x6AC` / `0x6AD` / `0x6AE`

### 2.1 The conflict

Two prior reports give incompatible names:

| Byte | FOOTCLASS_STRUCT_LAYOUT.md / COMPLETE | FOOTCLASS_NON_MOVEMENT_FIELDS.md |
|------|----------------------------------------|-----------------------------------|
| 0x6AC | `SkipHeadToCoord` (one-shot) | `IsUnderground` |
| 0x6AD | `IsDeploying` | `IsTunneling` |
| 0x6AE | `IsUndeploying` | `TunnelComplete` |

### 2.2 Verified behavior from `Set_Destination_Internal` (`0x4D94B0`)

Re-decompiled fresh:

```c
if ((*(char *)((int)param_1 + 0x6ad) != '\0') && (param_2 != 0)) { return; }
if ((*(char *)((int)param_1 + 0x82)  != '\0') && (param_2 != 0)) { return; }
if ((param_1[0xb9] != 0) && (param_2 != 0)) { return; }
...
if ((param_2 == 0) && (*(char *)((int)param_1 + 0x6ad) != '\0') && (param_1[0xac] != 0)) {
    *(undefined4 *)(param_1[0xac] + 0x2ac) = 0;       // clear linked-building combat
    param_1[0xac] = 0;                                 // drop the linked-building ptr
    *(undefined1 *)((int)param_1 + 0x6ae) = 1;        // mark "undeployed"
}
...
if ((char)param_1[0x1ab] == '\0') {                   // 0x6AC clear → call locomotor
    ...
    (**(code **)(*(int *)param_1[0x19d] + 0x44))(...); // ILocomotion::Head_To_Coord
} else {
    *(undefined1 *)(param_1 + 0x1ab) = 0;             // 0x6AC set → consume one-shot
}
```

### 2.3 Verdict (HIGH confidence)

The behavior matches the **Deploy** interpretation, not the **Tunnel** one:

| Byte | **Verified name** | Behavior |
|------|-------------------|----------|
| 0x6AC | **`SuppressHeadToCoord`** (one-shot) | When set, `Set_Destination_Internal` skips the `ILocomotion::Head_To_Coord` call and clears the flag. Used by the chrono-warp / IPiggyback restoration sequence to update `NavCom` without triggering a fresh locomotor move. |
| 0x6AD | **`IsDeploying`** | When set AND target ≠ 0: rejects the destination. When set AND target = 0: triggers cleanup of the linked building (`param_1[0xac]` = byte `0x2B0`) and sets `0x6AE`. This matches the MCV/IFV/Prism Tower deploy-undeploy state machine, **not** subterranean tunnel state. |
| 0x6AE | **`JustUndeployed`** | Set to 1 by the cleanup branch above. |

The "Tunneling" interpretation is incorrect: `param_1[0xac]` (byte 0x2B0) is a generic
`linked_building` pointer, not a tunnel link. There's no "tunnel" state machine in
this code path. (Subterranean movement, where it exists in YR, lives in
`SubterraneanLocomotionClass`, not in these flags.)

### 2.4 Tiberian Sun caveat

YR retains the `SubterraneanLocomotor=` plumbing from TS, but in standard YR content
no unit ships with subterranean locomotion. The "TunnelComplete" naming in the older
report likely reflects TS-era thinking projected onto a deploy flag. **Active in YR: Yes**
for the deploy interpretation; **the tunnel interpretation is dead in standard YR**.

---

## 3. Re-verification of high-impact constructor offsets

Re-decompiled `FootClass::Constructor` at `0x4D31E0` and confirmed every field listed in
prior reports. Selected spot-checks:

| Byte | Init Value | Source line | Status |
|------|------------|-------------|--------|
| 0x520 | `0xFFFFFFFF` | `param_1[0x148] = 0xffffffff` | ✓ |
| 0x53C (byte) | 0 | `*(undefined1 *)(param_1 + 0x14f) = 0` | ✓ |
| 0x59C | 10 | `param_1[0x167] = 10` (WaypointQueue.Capacity) | ✓ |
| 0x5C0 | 10 | `param_1[0x170] = 10` (EnterQueue.Capacity) | ✓ |
| 0x5C4 | `0xFFFFFFFF` | `param_1[0x171] = 0xffffffff` | ✓ |
| 0x5E0 | `0xFFFFFFFF` | `param_1[0x178] = 0xffffffff` (PathHeadIndex; **set late, near end of ctor**) | ✓ |
| 0x674 | 0 | `param_1[0x19d] = 0` (ILocomotion*) | ✓ |
| 0x684 (byte) | `0xFF` | `*(undefined1 *)(param_1 + 0x1a1) = 0xff` (DriveTrackIndex) | ✓ |
| 0x6B6 (byte) | **1** | `*(undefined1 *)((int)param_1 + 0x6b6) = 1` (only field initialized to non-zero/non-sentinel) | ✓ |
| 0x6B8 (byte) | 0 | `*(undefined1 *)(param_1 + 0x1ae) = 0` | ✓ |

The constructor also registers the new instance into **two** global object tables
(at `DAT_00b0f5dc` and `DAT_008b3dc4`), and ORs `0x4` into the byte at `param_1+5`
(an ObjectClass flag).

Timer "Value" mid-fields (`0x644`, `0x654`, `0x660`, `0x66C`, `0x6A4`) are **NOT
initialized in the constructor** — confirmed by the re-decomp. Same for the gap dword
at `0x574` (skipped between `param_1[0x15C]` and `param_1[0x15E]`). These are the
only memory holes in the field map.

---

## 4. Inter-doc conflict resolution (Agent A digest)

The prior `Explore` digest flagged claims in sibling docs that conflict with
FOOTCLASS_STRUCT_LAYOUT.md / FOOTCLASS_COMPLETE_GHIDRA_REPORT.md. Status of each:

| Conflict | Source | Verdict | Notes |
|----------|--------|---------|-------|
| `0x685 = TubeSegmentIndex` (incremented per segment, not reset on entry) | FOOTCLASS_FLAGS_BLOCK | **Likely TS-only**: tube travel exists in TS, mostly dormant in YR. Evidence in binary is thin. Hold this label as MEDIUM. |
| `0x686 = PathTargetWaypointID` | FOOTCLASS_FLAGS_BLOCK | Plausible but unverified in this pass. |
| `0x6B6 init = 1` (claim by FOOTCLASS_FLAGS_BLOCK that this is `blocked_delay`, not `IsNewlyCreated`) | conflict | Both reports agree the init **is 1**. The disagreement is over the *name*. Whichever it is, the constructor sets it to 1 — verified. |
| `0x6B7 = path_blocked` (vehicles never clear) vs. `DestinationJustSet` (cleared at end of Set_Dest_Internal) | conflict | **Both are partly right**: `Set_Destination_Internal` does clear `*(byte*)(param_1+0x6B7) = 0` at `LAB_004D96C2`. Whether other code paths set it to 1 in the "vehicle blocked" sense is plausible but not confirmed in this pass. The clear-on-Set_Destination behavior is verified. |
| Mission_Move OnArrival pops NavCom Queue (not Mission_Move itself) | FOOTCLASS_MISSION_HANDLERS / MISSION_MOVE | Internally consistent; not contradicted by the layout reports (which don't make a Mission_Move-pops-queue claim). |
| `AircraftClass::Mission_Attack` fires inline at state 4; ground units fire in per-tick AI not dispatch | FOOTCLASS_MISSION_ATTACK | Internally consistent; not contradicted by layout reports. |

Net: the **layout** reports and the **mission/flag** reports are largely orthogonal —
they document different surfaces. The few overlapping fields where they disagree
boil down to **naming**, with the binary behavior consistent across both interpretations.

---

## 5. Updated open questions

Carrying forward from prior reports, with status updates:

| # | Question | Prior status | Updated status |
|---|----------|--------------|----------------|
| 1 | Identity of `0x694` LargeObjectPtr | open | **RESOLVED** — ParasiteHost back-pointer (§1) |
| 2 | Fields `0x524–0x52A` (4 words) | open | still open — initialized to 0 in ctor, no readers found by quick xref scan |
| 3 | CDTimerClass countdown vs elapsed direction | open | still open — read-side semantics not verified this pass |
| 4 | Byte `0x6AC` writers (one-shot setter) | open | **PARTIAL** — Set_Destination_Internal self-clears (`0x4D9611`); a setter exists in `TechnoClass::Set_Destination` (`0x7425BF`) but its surrounding context is hard to read; full setter chain not enumerated this pass |
| 5 | `TeamTypeClass+0xF6` (independent-targeting flag) INI key mapping | open | still open |
| 6 | Bytes `0x685`, `0x686`, `0x68A`, `0x68B`, `0x68C`, `0x6B0`, `0x6B2`, `0x6B5` | open | still open — FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md proposes labels (TubeSegmentIndex, etc.) at MEDIUM confidence; not independently verified this pass |
| 7 | The exact instruction that **sets** `host->[0x694]` to begin infestation | new | open — only clears found by direct pattern search; setter likely uses indirect addressing |

---

## 6. Tiberian-Sun-legacy callouts

Verified during this pass that none of the FootClass logic re-investigated here is gated
behind TS-only `SpecialFlags`. All re-decompiled functions
(`AI`, `Set_Destination_Internal`, `ReceiveDamage`, `PointerExpired`,
`WarpAttachClass::Detach`, `WarpAttachClass::UpdateAttack`, `Init_Managers`) execute on
every standard YR skirmish tick.

The only flagged TS-residual concern in this pass is the **TubeSegmentIndex** label on
byte `0x685` (proposed by FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md). Tube travel between
buildings is a TS feature with very thin YR usage — labels in this byte range should
remain MEDIUM until verified against an actual YR-active code path.

---

## 7. Sources

### Re-decompiled this pass

- `0x4D31E0` — FootClass::Constructor (full re-verification)
- `0x4D7330` — FootClass::ReceiveDamage
- `0x4D94B0` — FootClass::Set_Destination_Internal
- `0x4D9960` — FootClass::PointerExpired
- `0x4DA530` — FootClass::AI
- `0x4DEBB0` — FootClass::ReceiveEMP
- `0x6F3F40` — TechnoClass::Init_Managers (key — proves ParasiteClass goes to FootClass+0x69C)
- `0x6970A0` — FUN_006970A0 (HouseClass-like; ruled out as FootClass+0x694 writer)
- `0x629FD0` — WarpAttachClass::UpdateAttack
- `0x62A4A0` — WarpAttachClass::Detach
- `0x629210`, `0x6292B0` — ParasiteClass::Constructor
- `0x741970` — TechnoClass::Set_Destination

### Byte-pattern searches

- `89 ?? 94 06 00 00` (mov [reg+0x694], reg32) — 12 hits, only 5 in FootClass-relevant code
- `c7 ?? 94 06 00 00` (mov [reg+0x694], imm32) — no hits
- `66 89 ?? 94 06 00 00` (16-bit mov) — no hits
- `88 ?? 94 06 00 00` (8-bit mov) — 2 hits, both in TechnoTypeClass (different class)
- `8B ?? 94 06 00 00` (read pattern) — 37 hits
- `c6 ?? ac 06 00 00 ??` (mov byte [reg+0x6AC], imm8) — 2 hits
- `c6 ?? b6 06 00 00 ??` (mov byte [reg+0x6B6], imm8) — 14 hits
- `c6 ?? b7 06 00 00 ??` (mov byte [reg+0x6B7], imm8) — 16 hits
- `c6 ?? b8 06 00 00 ??` (mov byte [reg+0x6B8], imm8) — 2 hits

### Prior reports cross-checked

- FOOTCLASS_COMPLETE_GHIDRA_REPORT.md (820 lines, dated 2026-04-01)
- FOOTCLASS_STRUCT_LAYOUT.md (340 lines, dated 2026-04-06)
- FOOTCLASS_NON_MOVEMENT_FIELDS.md
- FOOTCLASS_AI_GHIDRA_REPORT.md
- FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md
- FOOTCLASS_VTABLE_COMPLETE.md
- FOOTCLASS_MISSION_{ATTACK,MOVE,HANDLERS}_GHIDRA_REPORT.md
- FOOTCLASS_PATHFINDING_AND_MOVEMENT.md
- FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md

### Rust implementation reference

`src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/movement/locomotor.rs`,
`src/sim/movement/movement_tick.rs`, `src/sim/passenger.rs`, `src/sim/aircraft/mod.rs`,
`src/sim/pathfinding/`. ~70% of FootClass behavior is implemented; the largest gaps
remain **TeamClass**, the **WaypointQueue**, **parasite/infestation system**
(no `ParasiteHost` field, no Terror Drone behavior), **FogUpdate/Movement/Idle timers**,
and the **IPiggyback locomotor swap**. None of these gaps are blockers for the current
playable feature set, but they will need to land before parity with infested-unit
behavior, shift-click waypoints, and chrono-warp residual-state cleanup is achievable.
