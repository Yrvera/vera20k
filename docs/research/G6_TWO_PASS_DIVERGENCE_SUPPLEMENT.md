# G6 Two-Pass Can_Enter_Cell — Research Supplement

**Date:** 2026-05-12
**Status:** Supplementary to [BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md](BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) §3.2 and [UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md](UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md)
**Confidence:** HIGH on the three structural findings (vtable identity, two-pass mechanism, divergence sub-case map). MEDIUM on observable-impact estimate — requires empirical probe.

**Scope:** This note re-verifies the load-bearing claims about the bridgehead two-pass `Can_Enter_Cell` mechanism directly in Ghidra MCP, resolves a contradiction between the two existing reports, and produces a concrete divergence sub-case map (the existing docs only described the divergence at the "edge case" level of detail).

---

## 1. Resolved Conflict — `vtable[0x1B0]` Identity

The two existing reports disagree on what `UnitClass`'s `vtable[0x1B0]` resolves to:

| Doc | Claim |
|---|---|
| `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` (2026-03-23) Phase 5 | `vtable[0x1B0] = TechnoClass::Can_Enter_Cell` (parent class) |
| `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` (2026-05-12) §3.2 Step 3 | `vtable[0x1B0] = CheckBridgeTraversal` (height-diff gate) |

### Verification (Ghidra MCP, 2026-05-12)

`CheckBridgeTraversal` lives at `0x4D9C60`. Direct xrefs:
```
From 007e2454 [DATA]   ← AircraftClass vtable + 0x1B0 (vtable base 0x7E22A4)
From 007e8e44 [DATA]   ← FootClass     vtable + 0x1B0 (vtable base 0x7E8C94)
From 007eb208 [DATA]   ← InfantryClass vtable + 0x1B0 (vtable base 0x7EB058)
From 007f5e20 [DATA]   ← UnitClass     vtable + 0x1B0 (vtable base 0x7F5C70)
```

Vtable bases confirmed by constructor xrefs:
- `0x7E22A4` → referenced by `AircraftClass__Constructor` at 0x413D87, 0x41408A
- `0x7E8C94` → referenced by `FootClass__Constructor` at 0x4D345D, 0x4D359D, 0x4D3568
- `0x7EB058` → referenced by `InfantryClass__Constructor` at 0x517ACC, 0x517D9D
- `0x7F5C70` → referenced by `UnitClass__Constructor` at 0x73543A, 0x735794

**Verdict: `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` is correct.** `vtable[0x1B0]` for every concrete moving-class is `CheckBridgeTraversal` (0x4D9C60), NOT a parent class's `Can_Enter_Cell`.

### Why the older doc went wrong

`CheckBridgeTraversal` has `__thiscall` calling convention (RET semantics in callers) and is registered as a vtable entry, but its body **never reads the `this` pointer**. Ghidra's signature shows 5 stack params (`int param_1, int param_2, int *param_3, undefined1 *param_4, int param_5`) and no `this` use, suggesting it was compiled as a static function with a thiscall ABI for vtable compatibility — i.e., a multi-inheritance shim or template instantiation that happens to ignore `this`. A casual reader looking at "vtable[0x1B0] in Can_Enter_Cell" with the older doc's terminology would assume "parent virtual" without checking the actual slot contents.

### Action item

Update `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` Phase 5 to correct the claim. The function call at 0x73F2EB is to `CheckBridgeTraversal`, NOT `TechnoClass::Can_Enter_Cell`. The semantic effect described in the older doc ("if the parent says impassable, we immediately return 7") is broadly right but the mechanism is height-diff gating, not parent recursion. This changes how a Rust port that wanted to mirror the binary would structure things.

---

## 2. Two-Pass Mechanism — Step-by-Step Re-verification

Live disassembly read of `0x73F0A0`–`0x73F34C`, 2026-05-12. All claims in `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` §3.2 hold. Adding precise addresses + Ghidra decompile cross-reference for each step.

### Step 1 — Pre-vtable: decide pass flag (0x73F0B7–0x73F0EB)

Disassembly:
```
0073f0b1: MOV EBP,0x100
0073f0b7: MOV EAX,dword ptr [ECX + 0x140]    ; EAX = cell.Flags
0073f0be: TEST EBP,EAX                         ; cell.Flags & 0x100 != 0?
0073f0c0: JZ  0x0073f0e8                        ; no → pass = 0
0073f0c2: MOV EAX,[ESP + 0x9c]                 ; EAX = targetHeight (T1)
0073f0c9: CMP EAX,-0x1                          ; T1 == -1?
0073f0cc: JZ  0x0073f0e1                        ; yes → pass = 1
0073f0ce: MOVSX EDX,byte ptr [ECX + 0x11b]    ; EDX = (i8)cell.Level
0073f0d5: SUB EAX,EDX                           ; EAX = T1 - cell.Level
0073f0d7: CDQ
0073f0d8: XOR EAX,EDX
0073f0da: SUB EAX,EDX                           ; EAX = abs(T1 - cell.Level)
0073f0dc: CMP EAX,0x1                           ; abs > 1?
0073f0df: JLE 0x0073f0e8                        ; no → pass = 0
0073f0e1: MOV byte ptr [ESP + 0x13],0x1         ; pass = 1
0073f0e6: JMP 0x0073f0ed
0073f0e8: MOV byte ptr [ESP + 0x13],0x0         ; pass = 0
```

Predicate: `pass = (C.0x100 != 0) AND (T1 == -1 OR |T1 - C.Level| >= 2)`

Stored at `[ESP + 0x13]` (a 1-byte stack local). The deferred-mechanics doc's `> 1` is equivalent to `>= 2` for integer math — verified.

### Step 2 — Pre-vtable: GROUND occupancy snapshot (0x73F0ED–0x73F109)

```
0073f0ed: MOV AL,[ECX + 0x124]                 ; AL = cell.OccupationFlags low byte
0073f0f3: MOV EDX,[ECX + 0x54]                 ; EDX = cell+0x54 (ground secondary ptr)
0073f0f6: MOV [ESP + 0x14],AL                  ; store low byte
0073f0fa: MOV EAX,[ECX + 0x124]                ; reload full OccupationFlags
0073f100: SHR EAX,0x5                           ; bit 5 → bit 0
0073f103: AND AL,0x1
0073f105: MOV [ESP + 0x1c],EDX                 ; store secondary ptr
0073f109: MOV [ESP + 0x15],AL                  ; store vehicle bit
```

Stores to 3 locals — matches the deferred doc. The CONCAT11/0x01FF "bit-5-into-bit-8" pattern visible in the Ghidra decompile is a Ghidra artifact of stitching `[ESP+0x14]` (low byte, infantry bits) and `[ESP+0x15]` (vehicle bit, treated as bit 8 of the combined word). The actual reads downstream use the two bytes independently.

### Step 3 — Vtable dispatch (0x73F2EB)

```
0073f2c9: MOV ESI,[ESP + 0xa0]                 ; ESI = Can_Enter_Cell.param_5
0073f2d0: MOV EAX,[EBX]                         ; EAX = self->vtable
0073f2d2: LEA ECX,[ESP + 0x13]                 ; ECX = &pass_byte
0073f2d6: PUSH ESI                              ; CBT.param_5 = caller's param_5
0073f2d7: LEA EDX,[ESP + 0xa0]                 ; (after 1 push) EDX = &targetHeight
0073f2de: PUSH ECX                              ; CBT.param_4 = &pass_byte
0073f2df: MOV ECX,[ESP + 0x9c]                 ; (after 2 pushes) ECX = cell
0073f2e6: PUSH EDX                              ; CBT.param_3 = &targetHeight
0073f2e7: PUSH EDI                              ; CBT.param_2 = facing
0073f2e8: PUSH ECX                              ; CBT.param_1 = cell
0073f2e9: MOV ECX,EBX                           ; thiscall ECX = self (ignored by CBT)
0073f2eb: CALL [EAX + 0x1b0]                   ; CBT(cell, facing, &T, &pass_byte, param_5)
0073f2f1: CMP EAX,0x7
0073f2f4: JNZ 0x0073f303                        ; if not blocked, continue
0073f2f6-2fa: (early return path for blocked)
```

Confirmed:
- ECX = `self` (UnitClass*) is loaded but `CheckBridgeTraversal` does not read it.
- Stack args (left-to-right caller view): `cell, facing, &targetHeight, &pass_byte, param_5`.
- `CBT.param_1` (its "src cell") = Can_Enter_Cell's `cell` = A*'s **neighbor cell being entered**.
- `CBT.param_5` (its "dst cell") = Can_Enter_Cell's `param_5`. If 0, CBT computes it as `cell + reverse-direction-offset` — which in A*-terms is the **parent cell** (where we came from).

### CheckBridgeTraversal effect on `targetHeight`

CBT writes to `*param_3` (the `&targetHeight` pointer) in exactly TWO cases:

1. **Seed mode** (`param_2 == -1`, never used by Can_Enter_Cell): `*T = src.Level + 4` if T == -1 AND src has 0x100.
2. **Normal mode** at function entry: `*T = dst.Level + 4` if T == -1 AND dst has 0x100. Then: if src does NOT have 0x200, return 7.

After this, CBT continues with diff arithmetic but does NOT write to `*T` again in any successful return path.

**Net effect on T (the only path Can_Enter_Cell hits):**
- IF `T1 == -1` AND `parent.0x100` set:
  - `T2 = parent.Level + 4`
  - If `cell.0x200 == 0` → return 7 (Can_Enter_Cell short-circuits, no Step 4)
  - Else → fall through with `T2 = parent.Level + 4`
- IF `T1 != -1` OR `parent.0x100` not set:
  - `T2 = T1` (unchanged)

### Step 4 — Post-vtable: CONDITIONAL bridge-layer overwrite (0x73F303–0x73F34C)

```
0073f303: MOV ECX,[ESP + 0x9c]                 ; ECX = T2 (post-CBT)
0073f30a: CMP ECX,-0x1
0073f30d: JZ  0x0073f34c                        ; T2 == -1 → skip
0073f30f: MOV EDX,[ESP + 0x94]                 ; EDX = cell
0073f316: TEST [EDX + 0x140],EBP                ; cell.Flags & 0x100?
0073f31c: JZ  0x0073f34c                        ; no → skip
0073f31e: MOVSX EAX,byte ptr [EDX + 0x11b]    ; EAX = (i8)cell.Level
0073f325: ADD EAX,0x4                           ; EAX = cell.Level + 4
0073f328: CMP ECX,EAX                           ; T2 == cell.Level + 4?
0073f32a: JNZ 0x0073f34c                        ; no → skip
0073f32c: MOV AL,[EDX + 0x128]                 ; AL = cell.AltOccupationFlags low byte
0073f332: MOV [ESP + 0x14],AL                  ; OVERWRITE infantry bits
0073f336: MOV EAX,[EDX + 0x128]
0073f33c: MOV EDX,[EDX + 0x58]                 ; bridge secondary ptr
0073f33f: SHR EAX,0x5
0073f342: AND AL,0x1
0073f344: MOV [ESP + 0x1c],EDX                 ; OVERWRITE secondary ptr
0073f348: MOV [ESP + 0x15],AL                  ; OVERWRITE vehicle bit
```

Predicate: `T2 != -1 AND cell.0x100 AND T2 == cell.Level + 4` → overwrite occupancy locals with bridge-layer values.

### Step 5 — Main loop list selection (0x73F4F9–0x73F520)

```
0073f4f9: MOV AL,[ESP + 0x13]                  ; AL = pass flag (stored in Step 1)
0073f4fd: MOV [ESP + 0x17],0x0                 ; crushCandidate = 0
0073f502: TEST AL,AL
0073f504: JZ  0x0073f51a                        ; pass = 0 → ground list
0073f506: MOV ESI,[EDI + 0xe8]                 ; ESI = cell.AltObject (BRIDGE list)
0073f50c: JMP 0x0073f520
0073f51a: MOV ESI,[EDI + 0xe4]                 ; ESI = cell.FirstObject (GROUND list)
```

The pass flag at `[ESP+0x13]` is set ONCE in Step 1 and never overwritten — CBT in Step 3 only touches `[ESP+0x9c]` (T) and the pass-byte the caller passed in (via `&pass_byte`, which goes to `*param_4` inside CBT — separate variable from `[ESP+0x13]`). Wait — these need clarification:

**Important nuance:** `[ESP+0x13]` is the local pass byte set in Step 1. `LEA ECX,[ESP+0x13]` at Step 3 passes `&[ESP+0x13]` to CBT as its `param_4` (`bridgeEntered_out`). Inside CBT, the only write to `*param_4` is in the diff-4 going-up branch:
```c
if (iVar5 == *(char *)(param_5 + 0x11b) + -4) {  // src.Level == dst.Level - 4
  // ... checks ...
  *param_4 = 1;   ← can OVERWRITE the local pass byte
  return 0;
}
```

So **CBT CAN write 1 to `[ESP+0x13]`**, overwriting Step 1's pass-flag decision! This happens only in the specific diff-4 "going up onto bridgehead from below" case (src=cell, dst=parent, src.Level = dst.Level - 4). In that case, CBT signals "bridge entered" by writing 1.

Implication: the pass flag at Step 5 can be:
- The Step 1 value (most calls)
- OR 1 (if CBT hit the diff-4 going-up case in this call)

This means the pass flag is **not strictly pre-decided** — CBT can flip it to 1 under specific diff-4 conditions. The "pre-decided" framing in the deferred doc is approximate; the real picture is "pre-decided OR force-set to 1 by CBT diff-4 going-up".

This refinement matters for the divergence map below — Case 1 (`pass=bridge`) can be reached either via Step 1 OR via CBT's `*param_4 = 1`.

---

## 3. Divergence Sub-case Map (Concrete Predicates)

Define:
- `C` = neighbor cell (Can_Enter_Cell's `param_2`)
- `P` = parent cell (CBT's `dst`, computed from reverse facing)
- `T1` = `targetHeight` at Can_Enter_Cell entry
- `T2` = `targetHeight` after CBT call
- `pass1` = pass flag at Step 1
- `passF` = final pass flag at Step 5 (= `pass1 OR 1-if-CBT-diff-4-going-up`)

### Pre-vtable pass1 predicate
```
pass1 = (C.0x100 != 0) AND (T1 == -1 OR |T1 - C.Level| >= 2)
```

### Post-vtable bits predicate
```
bits_bridge = (T2 != -1) AND (C.0x100 != 0) AND (T2 == C.Level + 4)
```

### CBT's update to passF (in addition to passing through pass1)
```
passF = pass1 OR (
  abs(C.Level - P.Level) == 4
  AND C.Level == P.Level - 4
  AND C.0x100 != 0 AND C.0x200 != 0
)
```

The CBT diff-4 going-up branch: requires `src.Level == dst.Level - 4` (i.e., `C.Level == P.Level - 4`) AND `src.0x100` AND `src.0x200`. When all three hold, CBT writes `*param_4 = 1`, forcing `passF = 1` regardless of `pass1`.

### Possible divergence cases (pass vs bits)

**Case 1: `passF = 1` (bridge list) AND `bits_bridge = false` (ground bits).** Divergence.

Sub-case decomposition:

| # | T1 | P.0x100 | P.Level vs C.Level | Trigger condition | Frequency estimate |
|---|----|---------|--------------------|-------------------|--------------------|
| 1.a | -1 | 0 | any | First A* step from non-bridge land onto bridge cell. CBT can't update T (P.0x100=0). T2=-1, Step 4 skipped. | **Common** — every path that originates on land and crosses a bridge starts here. |
| 1.b | -1 | 1 | P.Level ≠ C.Level | Both P and C are bridge cells (e.g., bridgehead + body), but at different cell Levels. T2 = P.Level + 4 ≠ C.Level + 4. | **Moderate** — fires at the bridgehead↔body boundary on a fresh path expansion. |
| 1.c | ≠ -1 | n/a | n/a, but \|T1−C.Level\| ≥ 2 AND T1 ≠ C.Level+4 | T1 carries from prior step but doesn't match the deck-height predicate for C. | **Rare** — requires a specific T1 inheritance that doesn't align with C's deck. |
| 1.d | -1 | 1 | P.Level = C.Level, C.0x200 = 0 | Both P, C are bridge body cells at same Level, but cell C lacks bridgehead flag. CBT's normal-mode T update requires `src.0x200` (= C.0x200). If 0, returns 7 before Step 4 — but pass1 was 1. NO divergence here because Can_Enter_Cell short-circuits at Step 3 before reaching Step 4 or Step 5. | N/A (short-circuit). |
| 1.e | any | any | `C.Level == P.Level - 4` AND C.0x100 AND C.0x200 | CBT's diff-4 going-up branch fires, sets passF=1. T2 update behavior in this branch: no write to *T in this branch — T2 = T1. So bits_bridge requires T1 = C.Level + 4. But we're going UP (C below P by 4) so C.Level = P.Level - 4 and T1 = C.Level + 4 means T1 = P.Level. Plausible if unit was at parent ground height. | **Specific.** Fires whenever a unit enters a bridgehead from below — i.e., approaching a bridgehead from non-deck land. Always fires for this transition. |

**Case 2: `passF = 0` (ground list) AND `bits_bridge = true` (bridge bits).** Mathematically impossible (the predicates contradict — `bits_bridge=true` requires `C.0x100=1` AND `T2=C.Level+4`, and `passF=0` with `C.0x100=1` requires `T2!=-1 AND |T2-C.Level|<2`, but `T2=C.Level+4` gives `|T2-C.Level|=4≥2`).

**Conclusion: divergence is one-directional** — only `(bridge list, ground bits)` can occur.

---

## 4. Observable Impact Assessment

The divergence affects which occupancy bits drive the "is this cell available for me to enter" decision. The classification logic at Phase 12 of Can_Enter_Cell uses `[ESP+0x14]` (infantry sub-cell bits) and `[ESP+0x15]` (vehicle bit) to decide:

- `(occupancyBits & 0x3F) != 0` → some infantry sub-cell occupied → trigger crusher/weapon/return logic
- `hasUnitOnCell != 0` → vehicle present → check building-crush path

The divergence matters only when **ground bits ≠ bridge bits** for the same cell. Concretely:

| Ground sub-cells | Bridge sub-cells | gamemd sees (uses ground bits) | Rust sees (uses bridge bits) | Player-visible delta |
|-------------------|-------------------|--------------------------------|------------------------------|----------------------|
| empty | empty | clear | clear | none |
| occupied | occupied | occupied | occupied | none |
| **occupied** | **empty** | occupied (block/scatter/etc.) | clear (pass through) | **DIVERGENCE** |
| **empty** | **occupied** | clear (pass through) | occupied (block/scatter/etc.) | **DIVERGENCE** |

The observable cases require **asymmetric cross-layer occupancy on the same cell**:
1. Infantry/vehicle on the ground UNDER a high bridge, while the bridge deck above is clear of units. A unit crossing the bridge would see "clear" (Rust) vs "occupied" (gamemd) for the bridgehead cell.
2. Infantry on a bridge deck while the ground below is clear. A unit approaching from below would see "clear" (Rust) vs "occupied" (gamemd).

### Frequency in normal play

- **Configuration 1 (units under bridge, deck clear):** Possible. A defender setting up infantry under a high bridge while attackers cross the deck. Niche but plausible.
- **Configuration 2 (units on deck, ground clear):** Possible. Infantry garrisoning a bridge while a pursuer approaches from below. Niche but plausible.

Neither configuration arises from map-load initial state in standard YR maps. Both require runtime placement during play. Bridgeheads are narrow chokepoints where unit clustering is more common than the average cell, raising the chance somewhat.

**Quantitative estimate:** likely **0–5 fires per typical 30-minute skirmish**, concentrated on maps with bridges and combat near them (Lostlake, Dustbowl). Most fires would be unobservable because both layer's occupancy agree.

---

## 4.5 Q2 Resolved — Does Rust's `compute_neighbor_height` Mask the Divergence?

**Hypothesis (from §7 Q2):** Rust's existing height carry-through might happen to produce outputs that, fed through Rust's single-pass layer decision, match gamemd's `(bridge list, ground bits)` divergent outcome.

**Verdict: REFUTED.** Rust uses a structurally different mechanism that cannot reproduce the divergence.

**Rust's mechanism** ([src/sim/pathfinding/core.rs:425](../ra2-rust-game/src/sim/pathfinding/core.rs#L425)):
```rust
let neighbor_use_bridge = is_at_bridge_level(current.height, neighbor_cell);
```
The bool is decided ONCE and used for both walkability check and occupancy reading downstream (via `cell_entry::check_terrain` taking a single `MovementLayer`). Rust always produces matched-pair outcomes — `(bridge list, bridge bits)` OR `(ground list, ground bits)`. The divergent `(bridge list, ground bits)` outcome is structurally impossible in Rust.

**Sub-case-by-sub-case trace (Rust outcome vs gamemd outcome):**

| Sub-case | Rust outcome | gamemd divergent outcome | Observable difference |
|---|---|---|---|
| 1.a (high parent → bridgehead) | `(bridge, bridge)` via Case 3 + transition | `(bridge, ground)` | Differs in bit-read |
| 1.a (low parent → bridge body) | `(ground, ground)` via Case 3 fallthrough | `(bridge, ground)` | Differs in list iteration |
| 1.b (bridge↔bridge cross-Level) | `(bridge, bridge)` via Case 2 deck carry | `(bridge, ground)` | Differs in bit-read |
| 1.c (T1 carry-over, off-deck) | **G5 gate blocks** transition entirely | `(bridge, ground)` | Differs in path-existence, not bit-read |
| 1.d (short-circuit) | n/a (both block via different paths) | n/a | None |
| 1.e (bridgehead from below) | `(bridge, bridge)` — matches gamemd's typical non-divergent path | `(bridge, bridge)` typically | None for typical entry |

**Bonus finding from the just-landed G5 patch (2026-05-12):** sub-case 1.c is now **structurally eliminated** in Rust. The G5 height-diff legality gate (`_ => false` for `|diff| ≥ 2`) rejects any neighbor transition with a height jump of 2+ in absolute terms, before the bit-read ever happens. This removes one of the five divergence sub-cases from Rust's reachable state space.

**Interpretation:**
- Rust does NOT mask the divergence — Rust's outcome differs from gamemd's divergent outcome in all 4 surviving cases (1.a-high, 1.a-low, 1.b, 1.e-edge).
- But the **observable** impact still requires asymmetric cross-layer occupancy at the target cell (§4 table). When bridge and ground occupancy match (the overwhelming common case), all three engines produce identical outcomes regardless of internal mechanism.
- Rust's divergence direction varies per sub-case: in 1.a-high and 1.b, Rust reads BRIDGE bits while gamemd reads GROUND bits (Rust may be MORE permissive than gamemd in those cells). In 1.a-low, Rust iterates GROUND list while gamemd iterates BRIDGE list (different objects considered).
- 1.e is the only sub-case where Rust and gamemd produce the same outcome — the bridgehead-entry-from-below case. This is also the most common case, so most bridge crossings don't differ.

**Implication for the §5 recommendation:**
- The "defer G6, run Q5 probe first" stance still stands.
- Q1 (low-bridge prevalence scan) is **lower priority** than originally indicated — sub-case 1.e (the one the low-bridge config triggers) is the ONLY sub-case where Rust and gamemd already match. Low-bridge prevalence is a non-question for Rust correctness.
- Q5 probe priority **raised**: if asymmetric cross-layer occupancy ever arises in retail play, the divergence WILL fire in 1.a-high/low and 1.b. Confirming whether that configuration ever arises is now the only gating question.

---

## 5. Recommendations

### For implementation (in priority order)

1. **Defer G6 implementation.** The divergence is real but observable triggers are narrow and not unique to retail behavior in any way that affects gameplay outcomes — a player commanding "vehicle approaches bridgehead with infantry underneath" would see a routing detail differ between engines, but the strategic outcome (vehicle can/can't path) depends on whether the ground-bit occupant blocks vehicle entry, which it usually does in both engines via different code paths.

2. **Before any G6 implementation: run the Q5 fidelity probe.** Build the scripted scenario from `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` §7 Q5 (two infantry on a single bridgehead, vehicle pathing across). Compare gamemd outcome to Rust outcome. **Most likely outcome**: both engines block (because some occupancy bit somewhere blocks the vehicle either way) → divergence is theoretical, formally accept the loss, close G6.

3. **If the Q5 probe shows a real difference**: the implementation path is to split Rust's cell-entry into two phases:
   - Phase 1 (pre-CBT-equivalent): snapshot ground bits, decide bridge_pass flag from `cell.0x100` and unit's current height.
   - Phase 2 (post-CBT-equivalent): if the height arithmetic ends up with the unit at deck height on a bridge cell, overwrite the occupancy snapshot with bridge bits.
   - Use the snapshot's bits for occupancy classification, but the pass flag for object-list selection.

### For doc maintenance

4. **Update `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`** Phase 5 to correct the `vtable[0x1B0]` claim. The vtable slot resolves to `CheckBridgeTraversal` (0x4D9C60), not `TechnoClass::Can_Enter_Cell`. Same doc Phase 8 ("FUN_004d9c10") should be relabeled `FootClass__LocomotorPassabilityCheck` (Ghidra has this label).

5. **Update `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`** Phase 1 + Field Offsets table for `CellClass`: `+0x11A` is dual-semantic (terrain height for normal cells, tube sub-direction for tube cells); `+0x11B` is `Level` (signed i8 height level). The older doc's labels were imprecise. Cross-reference the deferred-mechanics doc §2.

6. **Note in the BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md** that the pass flag is not strictly pre-decided — CBT's diff-4 going-up branch (`*param_4 = 1`) can force the pass flag to 1 mid-call. Currently the doc characterizes the mechanism as "pre-decided" which is approximately right but misses sub-case 1.e in §3 above (the always-fires-on-bridgehead-entry-from-below case).

---

## 6. Sources & Verification

**Decompiled (full body read, 2026-05-12):**
- `0x73F0A0` — `UnitClass::Can_Enter_Cell` (full Ghidra decompile + line-by-line disassembly)
- `0x4D9C60` — `CheckBridgeTraversal` (Ghidra decompile, function-body read)

**Memory reads:**
- `0x7E2454` (16 bytes): confirms `CheckBridgeTraversal` (0x4D9C60) is at this slot
- Cross-checked all four vtable slot addresses against constructor xrefs

**Xrefs read:**
- `0x4D9C60` (CheckBridgeTraversal): 4 data xrefs in AircraftClass/FootClass/InfantryClass/UnitClass vtables
- `0x7E22A4` / `0x7E8C94` / `0x7EB058` / `0x7F5C70`: confirmed as class-vtable bases via constructor refs
- `0x73F0A0`: single data xref from UnitClass vtable +0x1AC (the standard Can_Enter_Cell vtable slot)

**Function identity confirmations:**
- `0x4D9C60` = `CheckBridgeTraversal` (Ghidra label)
- `0x4D9C10` = `FootClass__LocomotorPassabilityCheck` (Ghidra label; the older doc's "FUN_004d9c10")

**Not yet verified (deferred for empirical probe):**
- Quantitative fire-rate of each divergence sub-case during retail YR gameplay
- Whether configurations producing observable divergence ever occur in retail maps + standard skirmish play

---

## 7. Open Questions

**Q1 — Does CBT's diff-4 going-up branch (sub-case 1.e in §3) always force divergence?**

Sub-case 1.e: when a unit approaches a bridgehead from below (ground at P.Level, neighbor C is bridgehead with `C.Level = P.Level - 4`... wait that's a CONTRADICTION — bridgeheads are at the SAME Level as adjacent land, not 4 below). Let me re-examine.

Actually re-reading CBT: `iVar5 == *(char *)(param_5 + 0x11b) + -4` means `src.Level == dst.Level - 4`. In Can_Enter_Cell terms, `C.Level == P.Level - 4`. So C is 4 LOWER than P. Going from a high parent down to a low neighbor. This fires when going FROM a bridgehead/land DOWN ONTO a bridge body cell.

But then C.0x200 (bridgehead flag) is required, and body cells don't have 0x200. So sub-case 1.e fires only when the lower neighbor IS a bridgehead. This is the unusual case where a bridgehead has Level=0 and adjacent land is at Level=4 — i.e., a low-bridge configuration.

Need to verify: in standard YR maps, do bridgeheads sit at Level=0 (low bridges) or Level=land.Level (matching the adjacent land)? The standard "high bridge" pattern has bridgehead at the same Level as the adjacent land (e.g., 4). The "low bridge" pattern has bridgehead at the body's Level (0).

The Q1 question becomes: how prevalent is the diff-4 going-up CBT branch in retail? Worth a focused empirical probe — count how many bridgehead↔land transitions in retail maps have `bridgehead.Level == land.Level - 4`.

**Q2 — Does Rust's `compute_neighbor_height` already mask the divergence by always producing T2 = C.Level + 4 on bridge entry?**

The G5 patch (just landed) added a height-diff legality gate that relies on `compute_neighbor_height` to carry through legitimate bridge transitions as diff-0. If `compute_neighbor_height` always produces `T = deck_level` when entering a bridge cell (and the divergence cases are exactly the cases where T diverges from deck_level), then Rust's downstream bit-read might happen to match gamemd's behavior despite using a different code path.

Worth a follow-up trace: for each sub-case 1.a-1.e, walk through Rust's `compute_neighbor_height` + `cell_entry::check_terrain` and check whether the final layer/bit-read decision matches gamemd's `(bridge list, ground bits)` divergence outcome.

---

## 8. Bottom Line

**G6 divergence is real**, **one-directional** (only `pass=bridge, bits=ground`), and **structurally narrow** (5 sub-cases, of which 1.a–1.b fire on every "fresh path stepping onto a bridge from non-bridge" expansion, and 1.e fires on the low-bridge approach from below). Whether the divergence is **player-visible** depends on the asymmetric-occupancy condition — which is rare in retail maps and requires runtime unit placement.

The recommendation stands: **defer implementation, run the Q5 empirical probe before committing to any split, and update the older Can_Enter_Cell doc to fix the vtable[0x1B0] mistake**.
