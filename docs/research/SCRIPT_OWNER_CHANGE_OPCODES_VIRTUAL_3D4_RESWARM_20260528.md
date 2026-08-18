# Script Owner-Change Opcodes via Virtual +0x3D4 - Reswarm Research Report

**Address(es):** `0x006E9380` (`TeamClass__Recruit_Or_Add`), `0x006DD8B0` (`TriggerAction__Execute`), `0x006E0AA0`, `0x006E0B60`, `0x0050D290`, `0x0050D2D0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard YR team-script and trigger-action opcode paths found in this slice that reach `TechnoClass`-family virtual slot `+0x3D4` ownership transfer. This includes direct `+0x3D4` calls in team/action helpers and trigger actions that call `HouseClass` transfer/reclaim wrappers which themselves dispatch `+0x3D4`.
**Non-Scope:** base `TechnoClass::ChangeOwner` internals; subclass wrapper internals except dispatch implications; engineer capture, garrison, mind control, Psychic Dominator, slave/chrono death, unit crush/convoy side paths already covered by other reports; full retail campaign map usage frequency; editor-facing action names not embedded in the binary.
**Confidence:** High for opcode IDs, call addresses, arguments, and dispatch shape; Medium for human-readable opcode names where names are semantic labels from behavior rather than binary strings.
**Active in YR:** Conditional. The code is active in standard YR through team scripts and map triggers, but each path fires only when a loaded map/team script uses the corresponding opcode/action.

## 1. Overview

The script-facing ownership-transfer surface is not a raw owner write. The bounded static slice found one team-script opcode (`0x14`) and four trigger-action opcodes (`0x0E`, `0x24`, `0x7B`, `0x7C`) that transfer ownership through the concrete object's virtual `+0x3D4` slot, either directly or through `HouseClass` transfer helpers.

All direct calls push/pass an announce flag of `0` or `1` before dispatch. Therefore future Rust campaign/script support must route these opcodes through the same future class-dispatched owner-transfer API as engineer capture, garrison reconciliation, and mind control.

## 2. Key Offsets / Slots

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| vtable `+0x3D4` | Techno hierarchy | virtual owner-transfer dispatch | direct calls at `0x006E96A2`, `0x006E0B38`, `0x006E0C0B`, `0x006E0C72`, `0x0050D2AD`, `0x0050D2F5` | Conditional by script/trigger data |
| `Team+0x54` | TeamClass | first member pointer for team script member iteration | `TeamClass__Recruit_Or_Add` case `0x14`; assembly `0x006E9683` | Conditional |
| `Foot+0x5D8` / member `+0x176` | Foot/team member | next team member link used by team script loop | `0x006E9690`, `0x006E96AA..0x006E96AC` | Conditional |
| `Team+0x80` | TeamClass | script-step-complete byte set after opcode `0x14` | `0x006E96AE` | Conditional |
| `ActionEntry+0x2C` | Trigger action entry | trigger action opcode read by `TriggerAction__Execute` switch | decompile `0x006DD8B0` | Conditional |
| `ActionEntry+0x90` | Trigger action entry | house/country operand used by owner-transfer trigger actions | helpers `0x006E0AA0`, `0x006E0B60`, `0x006E0CA0`, `0x006E0D00` | Conditional |
| `Techno+0x34` | Techno | tag/link pointer checked by trigger-filter helpers | `0x006E0B1C`, decompile `0x006E0AA0` | Conditional |
| `Techno+0x74` | Object | `IsMarked`/on-map gate used by trigger filters | `0x006E0B0B`, decompile `0x006E0AA0` | Conditional |
| `Object+0x81` | Object | `InLimbo` gate, must be zero for trigger-selected transfer | `0x006E0B12`, decompile `0x006E0AA0` | Conditional |
| `Object+0x90` | Object | alive byte gate used by trigger-selected transfer | decompile `0x006E0AA0` | Conditional |
| `Techno+0x21C` | Techno | current owner pointer, used by action `0x24` filter | `0x006E0C2F`, decompile `0x006E0B60` | Conditional |
| `Building+0x520` | Building | building type pointer in action `0x24` special split | `0x006E0BE0`, `0x006E0C47` | Conditional |
| `BuildingType+0x1573/+0x1574` | BuildingType | special-building split in action `0x24`; second pass calls `BuildingClass__UpdateGapAndSpecialEffects` | `0x006E0BEA..0x006E0C13`, `0x006E0C51..0x006E0C7F` | Conditional |
| `House+0x6C` / `House+0x78` | HouseClass | owned techno array and count used by transfer/reclaim wrappers | assembly `0x0050D2A0`, `0x0050D299`, `0x0050D2D4`, `0x0050D2E0` | Conditional |
| `Techno+0x2E0` | Techno | source/old-house marker written by transfer, read by reclaim | `0x0050D2B4`, `0x0050D2E6` | Conditional |
| `Techno+0x2CC` | Techno | destination/new-house marker written by transfer, cleared by reclaim | `0x0050D2BA`, `0x0050D303` | Conditional |

## 3. Opcode / Caller Map

### 3.1 Team script opcode `0x14`

`TeamClass__Recruit_Or_Add @ 0x006E9380` is the team script opcode dispatcher. Case `0x14` is the only decompiled team-script case in this slice that calls virtual `+0x3D4`.

Verified order:

1. Read the current team member from `Team+0x54`.
2. For each member, preserve the next link from member `+0x5D8` before the transfer.
3. Resolve the target house from the script argument through `0x00502D30`.
4. Push announce flag `1`.
5. Push the resolved `HouseClass*`.
6. Call `member->vtable[+0x3D4](newOwner, 1)`.
7. Continue from the preserved next link.
8. Set `Team+0x80 = 1` after the loop.

Load-bearing assembly:

```text
0x006E9683  MOV EDI,[ESI+0x54]       ; first member
0x006E9690  MOV EBP,[EDI+0x5D8]      ; next member saved before transfer
0x006E9698  PUSH 0x1                 ; announce flag
0x006E969A  CALL 0x00502D30          ; resolve script house operand
0x006E969F  PUSH EAX                 ; new owner
0x006E96A0  MOV ECX,EDI              ; member this
0x006E96A2  CALL [EBX+0x3D4]         ; concrete ChangeOwner wrapper
0x006E96AE  MOV byte ptr [ESI+0x80],1
```

The decompiler prints this case as `HouseClass__Find_By_Country_Index(1)` in one view, but the assembly shows the resolver is called after loading the script operand from the stack and before pushing the returned house.

### 3.2 Trigger action `0x0E`: tagged/trigger-selected technos to resolved house

`TriggerAction__Execute @ 0x006DD8B0` case `0x0E` calls `FUN_006E0AA0 @ 0x006E0AA0`. This helper resolves a new owner from `ActionEntry+0x90`, then scans the global `TechnoClass` array.

House resolution:

- If `ActionEntry+0x90 == 0x2325`, call `0x00726910`, which returns `param+0x2C`.
- If `ActionEntry+0x90 == -1`, return `0`.
- If `ActionEntry+0x90` is one of `0x117B..0x1182`, call `0x00510ED0`, which maps those eight sentinel IDs through `FUN_0068C030(0..7)` into `g_HouseClass_Array`.
- Otherwise call `0x00502D30` / house-by-country resolver.
- If resolution returns null, no object is changed.

Object filter before dispatch:

- global `g_TechnoClass_Count` loop;
- object alive byte is nonzero;
- object marked/on-map byte is nonzero;
- `Object+0x81` (`InLimbo`) is zero;
- `Techno+0x34` tag/link pointer is non-null;
- `FUN_006E5380(param_4, Techno+0x34)` returns true, meaning the object's tag/link is in the trigger context chain.

Dispatch evidence:

```text
0x006E0B23  MOV EDX,[ESP+0x1C]
0x006E0B27  PUSH EDX                 ; resolved new owner
0x006E0B28  CALL 0x006E5380          ; tag/context predicate
0x006E0B33  PUSH 0x0                 ; announce flag
0x006E0B35  PUSH EDI                 ; new owner
0x006E0B36  MOV ECX,ESI              ; techno this
0x006E0B38  CALL [EAX+0x3D4]
```

The helper returns `1` if at least one object was transferred, otherwise `0`.

### 3.3 Trigger action `0x24`: all technos owned by trigger house to resolved house

`TriggerAction__Execute` case `0x24` calls `FUN_006E0B60 @ 0x006E0B60`. This helper resolves a new owner from `ActionEntry+0x90`, then scans `g_TechnoClass_Array` twice for objects whose current owner equals the trigger owner argument.

Pass 1 transfers all matching objects except a special building subset:

- require `Techno+0x21C == triggerOwner`;
- if object is not RTTI `6` (building), transfer in pass 1;
- if object is building but has no type pointer, transfer in pass 1;
- if building type bytes `+0x1573` and `+0x1574` are both zero, transfer in pass 1;
- call `+0x3D4(newOwner, 0)`.

Pass 2 transfers only the special building subset:

- require same current-owner match;
- require RTTI `6`;
- require non-null building type pointer;
- require `BuildingType+0x1573 != 0 || BuildingType+0x1574 != 0`;
- call `+0x3D4(newOwner, 0)`;
- then call `BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0` on that building.

Dispatch evidence:

```text
; pass 1
0x006E0C03  PUSH 0x0
0x006E0C05  PUSH EBP                 ; resolved new owner
0x006E0C06  MOV ECX,[EAX+EDI*4]      ; object
0x006E0C0B  CALL [EDX+0x3D4]

; pass 2
0x006E0C6A  PUSH 0x0
0x006E0C6C  PUSH EBP                 ; resolved new owner
0x006E0C6D  MOV ECX,[EAX+EDI*4]      ; object
0x006E0C72  CALL [EDX+0x3D4]
0x006E0C78  MOV ECX,ESI
0x006E0C7A  CALL 0x004549B0          ; building special-effect refresh
```

The two-pass split is load-bearing: Rust must not bulk-rewrite all matching owners in one unordered scan if it implements this action.

### 3.4 Trigger action `0x7B`: transfer source house's owned units to trigger owner

`TriggerAction__Execute` case `0x7B` calls `FUN_006E0CA0 @ 0x006E0CA0`, which resolves a house from `ActionEntry+0x90` and then calls the helper at `0x0050D290`.

Call setup:

```text
0x006E0CEA  PUSH EAX                 ; resolved source house
0x006E0CE6  MOV ECX,[ESP+0x8]        ; trigger owner/destination house
0x006E0CEB  CALL 0x0050D290
```

`0x0050D290` iterates the source house's owned array in reverse and dispatches each owned object through virtual `+0x3D4(destination, 0)`:

```text
0x0050D292  MOV EBP,[ESP+0x0C]       ; source house arg
0x0050D297  MOV EBX,ECX              ; destination house
0x0050D299  MOV EDI,[EBP+0x78]       ; source owned count
0x0050D2A0  MOV EAX,[EBP+0x6C]       ; source owned array
0x0050D2A3  PUSH 0x0
0x0050D2A5  PUSH EBX                 ; destination/new owner
0x0050D2A9  MOV ECX,ESI              ; owned object
0x0050D2AD  CALL [EDX+0x3D4]
0x0050D2B4  MOV [ESI+0x2E0],EBP      ; source marker
0x0050D2BA  MOV [ESI+0x2CC],EBX      ; destination marker
```

The helper writes the source/destination markers after each owner transfer.

### 3.5 Trigger action `0x7C`: reclaim previously transferred units

`TriggerAction__Execute` case `0x7C` calls `FUN_006E0D00 @ 0x006E0D00`, which resolves a house from `ActionEntry+0x90` and then calls `0x0050D2D0`.

Call setup:

```text
0x006E0D4A  PUSH EAX                 ; resolved original/source house
0x006E0D46  MOV ECX,[ESP+0x8]        ; current trigger owner/destination house
0x006E0D4B  CALL 0x0050D2D0
```

`0x0050D2D0` iterates the current house's owned array in reverse. It only reclaims objects whose `Techno+0x2E0` marker equals the resolved house argument:

```text
0x0050D2D4  MOV EBX,[EBP+0x78]       ; current house owned count
0x0050D2E0  MOV EAX,[EBP+0x6C]       ; current house owned array
0x0050D2E6  CMP [ESI+0x2E0],EDI      ; old/source marker matches?
0x0050D2F0  PUSH 0x0
0x0050D2F2  PUSH EDI                 ; restore owner
0x0050D2F3  MOV ECX,ESI
0x0050D2F5  CALL [EDX+0x3D4]
0x0050D2FD  MOV [ESI+0x2E0],0
0x0050D303  MOV [ESI+0x2CC],0
```

This is not a generic "all units to house" action. It is paired with action `0x7B` through the `+0x2E0/+0x2CC` markers.

## 4. Negative Pattern Sweep

The full byte sweep for `CALL [reg+0x3D4]` found the following direct virtual-call sites:

| Pattern / address | Classification for this slot |
|---|---|
| `0x006E96A2` | in scope: team script opcode `0x14` |
| `0x006E0B38` | in scope: trigger action `0x0E` helper |
| `0x006E0C0B`, `0x006E0C72` | in scope: trigger action `0x24` helper |
| `0x0050D2AD`, `0x0050D2F5` | in scope via trigger actions `0x7B` / `0x7C` house transfer helpers |
| `0x00519A2C`, `0x00519F7E`, `0x00520451` | out of scope: engineer capture / infantry per-cell capture family |
| `0x004582EB`, `0x00458323` | out of scope: civilian garrison transfer/revert |
| `0x004720DA`, `0x0053B298` | out of scope: mind-control release / Psychic Dominator |
| `0x007463C9`, `0x0074187B` | out of scope: UnitClass convoy wrapper / unit contact owner-transfer branch |
| `0x00448663` | out of scope: BuildingClass associated-unit transfer inside `BuildingClass::ChangeOwner` |
| `0x006B0BBB` | out of scope: slave/chrono-death credit/liberation helper |
| `0x006ECF79` | false positive / not a valid instruction; it lands inside `TeamClass__Convoy_Script_Follow_Target`, whose decompile has no `+0x3D4` call |

No additional valid `CALL [reg+0x3D4]` instruction in the searched executable patterns belonged to another team/script/trigger opcode in this slice.

## 5. Integration Points

Team script integration:

- `TeamClass__Recruit_Or_Add @ 0x006E9380` dispatches script actions `0x00..0x40`.
- Case `0x14` performs the owner transfer over the current team member list.
- The path is active when a map/team script reaches script action `0x14`.

Trigger action integration:

- `TriggerAction__Execute @ 0x006DD8B0` switches on `ActionEntry+0x2C`.
- Cases `0x0E`, `0x24`, `0x7B`, and `0x7C` are the ownership-transfer cases found here.
- These paths are active for map/campaign triggers that contain the corresponding action entries.

House resolution shared details:

- Sentinel `0x2325` resolves from the runtime trigger context through `0x00726910`.
- Operand `-1` returns null/failure in helper paths.
- Operands `0x117B..0x1182` map to side slots `0..7` via `0x00510ED0`.
- Other operands route through `0x00502D30`.

## 6. Current Rust Implementation Status

Rust has a partial map-trigger runtime, but none of the owner-transfer action IDs from this report are implemented.

| Rust surface | Current behavior observed | Delta |
|---|---|---|
| `src/map/actions.rs:12` | parses action kind and preserves raw params | parser surface is sufficient to carry these action IDs |
| `src/sim/trigger_runtime.rs:25..36` | implements only actions `22`, `28`, `29`, `48`, `53`, `54`, `56`, `57`, `67`, `68`, `69`, `112` | missing action `14`, `36`, `123`, `124` decimal (`0x0E`, `0x24`, `0x7B`, `0x7C`) |
| `src/sim/trigger_runtime.rs:214` | unmatched actions are ignored | owner-transfer trigger actions currently no-op |
| `src/sim/world/mod.rs:591` | trigger runtime runs through `Simulation::advance_triggers` and returns effects only | no mutation surface for owner transfers in trigger actions |
| `src/sim/ai.rs` | high-level AI exists, but no native TeamClass script interpreter found in this scan | team-script opcode `0x14` missing/unchecked |
| `src/sim/world/world_orders.rs:233` | engineer capture still direct-writes owner | same future owner-transfer API is needed, but engineer capture is out of this report's opcode scope |
| `src/sim/passenger.rs:597`, `src/sim/passenger.rs:608` | garrison owner reconciliation direct-writes owner | same future owner-transfer API is needed, but garrison is out of this report's opcode scope |
| `src/sim/entity_store.rs:142`, `src/sim/world/mod.rs:1204` | owner index rebuilds after mutation | native transfer happens inside `+0x3D4` with old/new side effects, not only by later global rebuild |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TeamClass__Recruit_Or_Add @ 0x006E9380` script switch | verified for `+0x3D4` ownership case | decompile and assembly `0x006E9683..0x006E96AE`; prior docs enumerate `0x00..0x40` switch | editor-facing opcode name not binary-embedded |
| Team script opcode `0x14` | verified | call `0x006E96A2`, resolver `0x00502D30`, `Team+0x80` write `0x006E96AE` | retail map usage frequency not counted |
| `TriggerAction__Execute @ 0x006DD8B0` switch | verified for owner-transfer cases | decompile cases `0x0E`, `0x24`, `0x7B`, `0x7C`; xrefs to helpers | exact editor action names not binary-verified |
| Trigger action `0x0E` -> `FUN_006E0AA0` | verified | xref `0x006DE069`; direct call `0x006E0B38`; decompile filter | retail map usage frequency not counted |
| Trigger action `0x24` -> `FUN_006E0B60` | verified | xref `0x006DE096`; calls `0x006E0C0B`, `0x006E0C72`; decompile two-pass split | exact names for type bytes `+0x1573/+0x1574` |
| Trigger action `0x7B` -> `FUN_006E0CA0` -> `0x0050D290` | verified | xref `0x006DFA26`; call setup `0x006E0CE6..0x006E0CEB`; transfer dispatch `0x0050D2AD` | editor-facing name not binary-verified |
| Trigger action `0x7C` -> `FUN_006E0D00` -> `0x0050D2D0` | verified | xref `0x006DFA53`; call setup `0x006E0D46..0x006E0D4B`; reclaim dispatch `0x0050D2F5` | editor-facing name not binary-verified |
| House operand sentinel handling | verified | helper decompile `0x006E0AA0`, `0x006E0B60`, `0x006E0CA0`, `0x006E0D00`; `0x00510F60`, `0x00510ED0`, `0x00726910` | human-readable meaning of `0x2325` beyond context-house resolver |
| Direct `CALL [reg+0x3D4]` sweep | verified | byte patterns `FF 90/92/93 D4 03 00 00`; context for every valid script/trigger hit | unusual register encodings not in this exact pattern family, if any |
| Rust trigger runtime comparison | verified | `src/sim/trigger_runtime.rs:25..36`, `src/sim/trigger_runtime.rs:214` | no Rust edited |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which team-script opcode directly dispatches virtual +0x3D4? -> Opcode 0x14 in TeamClass__Recruit_Or_Add.` (evidence: `0x006E9380`, call `0x006E96A2`)
- `[RESOLVED] OQ-02 - What arguments does team opcode 0x14 pass? -> Resolved house from script operand and announce flag 1.` (evidence: `0x006E9698..0x006E96A2`)
- `[RESOLVED] OQ-03 - Does team opcode 0x14 preserve the next member before transfer? -> Yes, it reads member +0x5D8 into EBP before calling +0x3D4.` (evidence: `0x006E9690`)
- `[RESOLVED] OQ-04 - Does team opcode 0x14 mark the script step complete? -> Yes, writes Team+0x80 = 1 after the loop.` (evidence: `0x006E96AE`)
- `[RESOLVED] OQ-05 - Which TriggerAction cases directly loop technos and call +0x3D4? -> Cases 0x0E and 0x24.` (evidence: `TriggerAction__Execute @ 0x006DD8B0`; helpers `0x006E0AA0`, `0x006E0B60`)
- `[RESOLVED] OQ-06 - What does trigger action 0x0E filter on? -> alive, marked, non-limbo, non-null Techno+0x34, and membership in the trigger context via FUN_006E5380.` (evidence: `0x006E0AA0`, assembly `0x006E0B0B..0x006E0B38`)
- `[RESOLVED] OQ-07 - What announce flag does trigger action 0x0E pass? -> 0.` (evidence: `0x006E0B33`)
- `[RESOLVED] OQ-08 - What does trigger action 0x24 filter on? -> current owner equals trigger owner, then a two-pass building/non-building split by RTTI and BuildingType+0x1573/+0x1574.` (evidence: `0x006E0B60`, assembly `0x006E0C0B`, `0x006E0C72`)
- `[RESOLVED] OQ-09 - What announce flag does trigger action 0x24 pass? -> 0 in both passes.` (evidence: `0x006E0C03`, `0x006E0C6A`)
- `[RESOLVED] OQ-10 - Does trigger action 0x24 have a post-transfer side effect for special buildings? -> Yes, pass 2 calls BuildingClass__UpdateGapAndSpecialEffects after +0x3D4.` (evidence: `0x006E0C7A`)
- `[RESOLVED] OQ-11 - Which trigger actions enter HouseClass transfer/reclaim wrappers? -> Cases 0x7B and 0x7C.` (evidence: xrefs `0x006DFA26`, `0x006DFA53`; helpers `0x006E0CA0`, `0x006E0D00`)
- `[RESOLVED] OQ-12 - What does action 0x7B transfer? -> It calls 0x0050D290 with ECX = trigger owner/destination and arg = resolved source house; that helper loops the source house owned array and calls each object +0x3D4(destination,0).` (evidence: `0x006E0CE6..0x006E0CEB`, `0x0050D290..0x0050D2BA`)
- `[RESOLVED] OQ-13 - What does action 0x7C reclaim? -> It loops the current trigger owner's owned array, selects objects whose Techno+0x2E0 marker equals the resolved house, calls +0x3D4(resolvedHouse,0), then clears +0x2E0/+0x2CC.` (evidence: `0x006E0D46..0x006E0D4B`, `0x0050D2D0..0x0050D303`)
- `[RESOLVED] OQ-14 - Are additional valid CALL [reg+0x3D4] script/team sites present in the byte sweep? -> No additional valid script/team/trigger ownership sites were found; other hits are known non-script systems or a false positive.` (evidence: byte-pattern sweep and context table in section 4)
- `[RESOLVED] OQ-15 - Is current Rust trigger runtime implementing these action IDs? -> No; action constants omit 14, 36, 123, and 124 decimal and unknown actions no-op.` (evidence: `src/sim/trigger_runtime.rs:25..36`, `src/sim/trigger_runtime.rs:214`)
- `[RESOLVED] OQ-16 - Is current Rust team-script support implementing opcode 0x14? -> No native TeamClass script interpreter was found in the focused Rust scan.` (evidence: `rg` over `src/sim`, `src/sim/ai.rs` high-level AI only)
- `[DEFERRED] OQ-17 - What are the exact FinalAlert/editor-facing names for action IDs 0x0E, 0x24, 0x7B, and 0x7C?` (category: `requires-different-system-context`; reason: names are not embedded in the verified binary switch; next-step-if-pursued: inspect editor resources or map authoring docs, but do not use names as mechanism evidence)
- `[DEFERRED] OQ-18 - Which stock campaign maps use each action and team opcode?` (category: `requires-different-system-context`; reason: this report verifies active engine paths, not MIX map extraction/frequency; next-step-if-pursued: extract retail campaign maps and scan `[Actions]` / `[ScriptTypes]`)
- `[DEFERRED] OQ-19 - Exact semantic names for BuildingType+0x1573/+0x1574 in trigger action 0x24 split.` (category: `bounded-cost-too-high`; reason: not needed to prove owner-transfer order; next-step-if-pursued: trace BuildingTypeClass::ReadINI fields for these bytes)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Team script opcode `0x14` changes every current team member through concrete virtual `+0x3D4(newOwner,1)`, preserving the next member before transfer and then marking the script step complete. | `0x006E9683..0x006E96AE`; call `0x006E96A2` | missing/unchecked: no native TeamClass script interpreter found | future team-script runtime; future owner-transfer API | Implement opcode `0x14` as class-dispatched owner transfer over current team members, with announce flag true and next-link preservation semantics. | A scripted team containing infantry and vehicles changes owner; vehicles use Unit wrapper, infantry use Foot wrapper, and all continue iteration even if transfer mutates team membership. | Do not bulk-set `entity.owner` over a sorted entity list. |
| Trigger action `0x0E` transfers only active, marked, non-limbo technos whose `Techno+0x34` tag/link is in the trigger context chain, through `+0x3D4(newOwner,0)`. | helper `0x006E0AA0`; dispatch `0x006E0B38`; predicate `0x006E5380` | missing: Rust trigger runtime ignores action 14 decimal | `src/sim/trigger_runtime.rs`; map tag/trigger context surfaces; future owner-transfer API | Add action support only when the runtime can provide native-equivalent trigger context filtering and concrete owner dispatch. | Map trigger action 14 targets a tagged unit and an untagged nearby unit; only the tagged unit transfers, with announce flag false. | Do not approximate by transferring every entity at the tagged cell or every entity of a house. |
| Trigger action `0x24` transfers all technos owned by the trigger owner to the resolved house in two passes: ordinary/non-special first, special buildings second, then special buildings refresh gap/special effects. | helper `0x006E0B60`; calls `0x006E0C0B`, `0x006E0C72`; post-call `0x004549B0` | missing: Rust trigger runtime ignores action 36 decimal | `src/sim/trigger_runtime.rs`; owner-transfer API; future building special-effect/gap surface | Preserve the native two-pass order and building post-transfer refresh when implementing this action. | Trigger action 36 on a house with units plus a gap/radar-like building transfers units first, then the special building and refreshes its effects. | Do not do one global direct owner rewrite plus one index rebuild. |
| Trigger action `0x7B` transfers every object in the resolved source house's owned array to the trigger owner/destination via `+0x3D4(destination,0)` and writes `Techno+0x2E0/+0x2CC` markers after each transfer. | call setup `0x006E0CE6..0x006E0CEB`; helper `0x0050D290`; dispatch `0x0050D2AD`; marker writes `0x0050D2B4..0x0050D2BA` | missing: Rust trigger runtime ignores action 123 decimal | `src/sim/trigger_runtime.rs`; house owned-array/order model; owner-transfer API | Implement as reverse iteration over the source house's native owned-order list, not entity sorted order, and preserve transfer markers if action `0x7C` is supported. | A map trigger temporarily hands all units from House A to House B; later reclaim can identify only the transferred objects. | Do not implement without marker state if reclaim action is in scope. |
| Trigger action `0x7C` reclaims only objects currently owned by the trigger owner whose `Techno+0x2E0` marker matches the resolved house, calls `+0x3D4(resolvedHouse,0)`, then clears `+0x2E0/+0x2CC`. | call setup `0x006E0D46..0x006E0D4B`; helper `0x0050D2D0`; dispatch `0x0050D2F5`; marker clear `0x0050D2FD..0x0050D303` | missing: Rust trigger runtime ignores action 124 decimal | `src/sim/trigger_runtime.rs`; owner-transfer API; temporary-transfer marker state | Pair with action `0x7B` state so reclaim affects only previously transferred units. | After two transfer actions, reclaim from one source restores only units whose stored source marker matches that house. | Do not reclaim all current-owner units indiscriminately. |
| House operands use special sentinels before ordinary country-index resolution: `0x2325`, `-1`, and `0x117B..0x1182`. | `0x006E0AA0`, `0x006E0B60`, `0x006E0CA0`, `0x006E0D00`; `0x00510F60`, `0x00510ED0`, `0x00726910` | missing/unchecked in Rust trigger params | `src/map/actions.rs`; `src/sim/trigger_runtime.rs` | Preserve raw integer operands and resolve them at execution time with native sentinel semantics. | Action with operand `-1` does nothing; action with side sentinel resolves through the active house array instead of parsing as a literal house name. | Do not convert all operands to strings/names during parsing and lose sentinel identity. |

### Stale Docs / Follow-up Docs

- `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md` deferred the scenario/team-script slice. Replacement framing: "Team script opcode `0x14` and trigger actions `0x0E`, `0x24`, `0x7B`, and `0x7C` are verified script-facing owner-transfer paths. They all reach concrete virtual `+0x3D4`; actions `0x7B/0x7C` do so through `HouseClass` transfer/reclaim helpers and marker fields `Techno+0x2E0/+0x2CC`."
- Older broad references that say "trigger actions / mission scripts call ChangeOwner" should be refined to the opcode map above rather than leaving the action IDs unnamed.

## Sources

- Ghidra read-only decompile: `TeamClass__Recruit_Or_Add @ 0x006E9380`, `TriggerAction__Execute @ 0x006DD8B0`, `FUN_006E0AA0`, `FUN_006E0B60`, `FUN_006E0CA0`, `FUN_006E0D00`, `FUN_006E5380`, `FUN_00510F60`, `FUN_00510ED0`, `FUN_00726910`.
- Ghidra read-only assembly/context: `0x006E9683..0x006E96AE`, `0x006E0B0B..0x006E0B40`, `0x006E0BE0..0x006E0C13`, `0x006E0C3D..0x006E0C7F`, `0x006E0CE6..0x006E0CEB`, `0x006E0D46..0x006E0D4B`, `0x0050D290..0x0050D2C6`, `0x0050D2D0..0x0050D310`.
- Ghidra byte-pattern sweep: `FF 90 D4 03 00 00`, `FF 91 D4 03 00 00`, `FF 92 D4 03 00 00`, `FF 93 D4 03 00 00`, `FF 96 D4 03 00 00`, `FF 97 D4 03 00 00`.
- Prior docs used as maps/checks: `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md`, `TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md`, `AI_BRIDGE_INTERACTION_GHIDRA_REPORT.md`, `HOUSECLASS_GHIDRA_REPORT.md`, `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md`.
- Rust scanned/read: `src/map/actions.rs`, `src/map/triggers.rs`, `src/map/trigger_graph.rs`, `src/sim/trigger_runtime.rs`, `src/sim/ai.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/sim/passenger.rs`, `src/sim/entity_store.rs`.

## Status

COMPLETE for the requested bounded script/team/scenario opcode caller map that reaches virtual `+0x3D4` in this slice. Retail campaign map usage frequency, editor-facing action names, and exact semantic names for `BuildingType+0x1573/+0x1574` are explicitly deferred because they do not change the verified owner-transfer call contract.
