# Phase 3 House last-Building-attack frame writer

**Date:** 2026-08-25

**Binary:** active retail Yuri's Revenge `gamemd.exe` in the live
`testProsjekt` Ghidra project

**Mode:** exhaustive-slice, research only; no Rust or Ghidra metadata changes

**Scope:** the `House+0x54D8` writer at `0x0044229C`, its exact admission and
ordering, the Building vtable `+0x80` gate, and directly adjacent effects.
The responder-selection helper reached at `0x00708080` is identified but not
expanded as a complete independent mechanism.

**Verdict:** **ACTIVE, VERIFIED MISMATCH.** `BuildingClass__ReceiveDamage`
copies the raw signed 32-bit current frame to the victim owner's
`House+0x54D8` before Building immunity, the already-dead gate, and the generic
receiver. Rust already owns and serializes the House field but does not write
it from combat. The old “not cloaked” interpretation of Building vtable
`+0x80` is wrong: the slot is exactly an `UndeploysInto && 1x1 foundation`
predicate. The old “attacker type index” interpretation of adjacent
`House+0x54DC` is also wrong: it is the attacker owner's House array index.

## 1. Entry and exact ordered contract

BuildingClass's primary vtable is `0x007E3EBC`; slot `+0x16C` resolves to
`BuildingClass__ReceiveDamage @ 0x00442230`. The active area-damage dispatcher
calls the same receiver slot at `Apply_area_damage @ 0x00489AB6`.

The load-bearing entry assembly is:

```text
0x00442243  CMP ESI, EBP                    victim == attacker?
0x00442254  CALL [attacker.vtable + 0x84]   get shared TechnoType
0x0044225A  MOV CL, [type + 0xCA0]          DamageSelf
0x00442262  JZ  0x00442C06                  result-0 return

0x0044227E  CMP EBP, 0                      attacker object is null?
0x00442280  JZ  0x004422C1                  skip complete block
0x00442286  CALL [victim.vtable + 0x80]
0x0044228E  JNZ 0x004422C1                  skip when predicate true
0x00442290  MOV EAX, [victim + 0x21C]       victim Owner House
0x00442296  MOV ECX, [0x00A8ED84]           raw current frame
0x0044229C  MOV [EAX + 0x54D8], ECX
0x004422A7  CALL [attacker.vtable + 0x3C]
0x004422B0  MOV EDX, [returned_house + 0x30]
0x004422B4  MOV [victim_house + 0x54DC], EDX
0x004422BC  CALL 0x00708080                  ECX=victim, arg=attacker
```

Building-specific immunity gates begin at `0x00442358`; the `Health == 0`
branch is at `0x004423E7`; the generic `TechnoClass__ReceiveDamage` call is at
`0x00442425`. Therefore the three adjacent effects happen before any of them.
The writer block does not read the requested damage, warhead, source-House
argument, alliance state, victim health, or eventual receiver result.

Exact admission is:

```text
if attacker == victim && !victim_type.DamageSelf:
    return result 0 before all receiver work

if attacker_object != null && !victim.Is1x1WithUndeploy():
    victim.Owner.last_building_attack_frame = raw_current_frame
    victim.Owner.last_building_attacker_house_index = attacker.Owner.index
    select_building_attack_responders(victim, attacker)
```

Self with `DamageSelf=yes` proceeds normally. A null source-House argument does
not suppress a non-null attacker object, and a non-null source-House argument
cannot substitute for a null attacker object.

## 2. Building vtable `+0x80` is not cloak state

The Building vtable slot address is `0x007E3F3C`. Its dword is `0x00457620`:

```text
0x00457620  MOV ECX, [ECX + 0x520]   BuildingType
0x00457626  JMP 0x00465D40
```

`BuildingTypeClass__Is1x1WithUndeploy @ 0x00465D40` returns one only when all
three tests pass:

```text
BuildingType+0x408 UndeploysInto != null
foundation_width_table[BuildingType+0xEF0] == 1
foundation_height_table[BuildingType+0xEF0] == 1
```

It otherwise returns zero. No cloak field, method, or state participates.
`DamageSelf` is independently bound to TechnoType byte `+0xCA0` by
`TechnoTypeClass__ReadINI`; Rust already parses that key.

## 3. Retail-data activation and exclusion

The active retail data has only these `UndeploysInto` BuildingTypes:

| Building | `UndeploysInto` | art foundation | Writer skip? |
|---|---|---:|---|
| `GACNST` | `AMCV` | `4x4` | no |
| `NACNST` | `SMCV` | `4x4` | no |
| `YACNST` | `PCV` | `4x4` | no |
| `YAREFN` | `SMIN` | `2x2` | no |

Evidence: `ini/rulesmd.ini:11631,12427,13101,13289` and
`ini/artmd.ini:1601,1626,1653,1804`; base RA2 has the two construction-yard
rows at `ini/rules.ini:8501,8532` and `ini/art.ini:1021,1044`.

Thus no active retail Building takes the `1x1 && UndeploysInto` skip. The gate
remains a supported mod-data rule and is cheap to preserve exactly because Rust
already stores `ObjectType.undeploys_into` and the merged art foundation.
`NAMISL` has `DamageSelf=yes` at `ini/rulesmd.ini:13083`, proving that the
self-entry byte is data-active, although this slice does not claim a specific
ordinary call in which the silo is its own receiver.

## 4. Adjacent effects and boundary

Attacker vtable `+0x3C` resolves to `0x006F9DC0`, whose body returns
`attacker+0x21C` (the attacker Owner House). The subsequent `+0x30` load is the
House array index. Therefore `House+0x54DC` is the last building attacker's
**owner-House index**, constructor default `-1`, not a TechnoType index.

`FUN_00708080 @ 0x00708080` is a substantial responder-selection helper. Its
verified entry gates include alliance, human victim control, attacker limbo,
campaign conditions, and attacker RTTI; it then scans same-House Infantry and
Unit arrays, tests response budget/threat/reachability, consumes Scenario RNG,
and can assign responder missions, targets, and cooldowns. Those downstream
effects do not change whether the timestamp write occurred: allied and limbo
attackers are rejected only after both House stores.

This report deliberately does not turn that identification into a complete
implementation contract. `House+0x54DC` and the responder-selection helper are
an explicit separate residual; implementing the timestamp alone must not be
described as implementing the entire adjacent block.

## 5. Complete direct-reference inventory

A non-truncated `search_instructions(operand_pattern=0x54d8)` scanned
1,161,416 instructions and returned exactly five direct displacement
references:

| Address | Owner | Effect |
|---:|---|---|
| `0x0044229C` | `BuildingClass__ReceiveDamage` | sole non-constructor writer |
| `0x004F5A59` | `HouseClass__Constructor` | initializes zero |
| `0x004FD80A` | `HouseClass__AI_Building_Strategy` | state-three deadline read |
| `0x004FD82D` | `HouseClass__AI_Building_Strategy` | re-arm deadline read |
| `0x0050CBCB` | `FUN_0050CAD0` | replacement-restriction deadline read |

The two strategy reads add literal `0x384`/900 with wrapping 32-bit arithmetic
and compare as signed values. `FUN_0050CAD0` adds signed
`Rules+0xDF0 AIRestrictReplaceTime` with the same machine-width behavior.

The corresponding full-program `0x54DC` search returned only constructor
`0x004F5A5F` (initialize `-1`) and writer `0x004422B4`. Indirect raw
serialization/reflection access is not excluded by a displacement search, but
there is no other static direct reference.

## 6. Edge matrix

| Case | Timestamp result | Proof |
|---|---|---|
| attacker object null, source House null or non-null | no write | `0x0044227E..80` tests only attacker object |
| attacker is victim, `DamageSelf=no` | result-zero return; no write | `0x00442243..62` |
| attacker is victim, `DamageSelf=yes` | ordinary writer admission | same branch falls through |
| allied attacker | write | alliance check exists only inside later `0x00708080` |
| negative, zero, or positive requested damage | write | no damage read before store |
| Building immunity/type immunity eventually rejects | write first | immunity starts after `0x00442358` |
| victim Health already zero | write first | health test at `0x004423E7` |
| attacker limbo | write first | limbo check is inside later helper |
| victim limbo | no local suppression | wrapper has no pre-writer victim-limbo read; caller reachability remains unproven |
| repeated qualifying calls in one frame | repeat both stores/helper call | no deduplication or prior-value test |
| 32-bit frame wrap | raw low 32 bits copied | direct dword store; consumers use signed wrapping arithmetic |

## 7. Open Questions Log

| ID | Question | Status and evidence |
|---|---|---|
| OQ-01 | Is `0x00442230` the Building receiver? | **RESOLVED:** Building vtable `+0x16C`; active area dispatcher calls that slot. |
| OQ-02 | Does self damage reach the writer? | **RESOLVED:** only with `DamageSelf=yes`; otherwise entry returns zero. |
| OQ-03 | Can source House replace a null attacker? | **RESOLVED-NO:** writer tests only attacker object. |
| OQ-04 | What is Building vtable `+0x80`? | **RESOLVED:** `UndeploysInto && 1x1 foundation`. |
| OQ-05 | Is `+0x80` a cloak predicate? | **RESOLVED-NO:** exact slot/body contains no cloak input. |
| OQ-06 | Does alliance suppress the timestamp? | **RESOLVED-NO:** alliance is tested only after both stores. |
| OQ-07 | Does damage sign or zero suppress it? | **RESOLVED-NO:** requested damage is unread. |
| OQ-08 | Does immunity suppress it? | **RESOLVED-NO:** immunity gates are later. |
| OQ-09 | Does already-zero Health suppress it? | **RESOLVED-NO:** Health gate is later. |
| OQ-10 | Are multiple same-frame calls deduplicated? | **RESOLVED-NO:** unconditional store and helper call. |
| OQ-11 | What value is stored at `+0x54D8`? | **RESOLVED:** raw current-frame dword. |
| OQ-12 | Are there other direct `+0x54D8` writers? | **RESOLVED-NO:** exhaustive instruction inventory above. |
| OQ-13 | What is adjacent `+0x54DC`? | **RESOLVED:** attacker owner House array index. |
| OQ-14 | Is `0x00708080` inert bookkeeping? | **RESOLVED-NO:** active responder selection with RNG/mission effects. |
| OQ-15 | Does victim limbo prevent receiver entry? | **OPEN at caller boundary:** no local gate; ordinary area collection is active-object based. |
| OQ-16 | Can serialized/reflection code touch the field without a direct displacement? | **UNCHECKED:** static direct-reference closure cannot prove a generic serializer negative. |
| OQ-17 | Does any retail undeploying Building take the 1x1 skip? | **RESOLVED-NO:** complete retail data set is 4x4/4x4/4x4/2x2. |
| OQ-18 | Is this a TS-only remnant? | **RESOLVED-NO:** active YR receiver/dispatcher and Strategy readers use it. |

## 8. Adversarial and zero-add passes

1. **Could immunity make a pre-receiver store observationally irrelevant?**
   No. Strategy and replacement timing read the persistent House field later;
   the store remains visible even when HP does not change.
2. **Could `source_house` be a safer Rust proxy for attacker presence?** No.
   Native deliberately distinguishes the object pointer from source House.
3. **Could “not cloaked” be retained as a harmless descriptive alias?** No.
   It would suppress ordinary cloaked buildings and miss the actual mod-data
   exclusion.
4. **Could `+0x54DC` still be an attacker type through an unusual `+0x3C`
   override?** No for the active Techno subclasses: the slot resolves to the
   owner-House getter, followed by House `+0x30`.
5. **Could the timestamp be delayed until after `resolve_receive_damage`?** No.
   Immunity, death, and nested side effects can exit or mutate before that
   point, while native commits it first.

The zero-add pass found no further timestamp admission input after checking the
self gate, null attacker/source-House split, vtable binding, damage sign,
alliance, immunity, health, limbo, repeat calls, frame representation, all
direct field references, and retail INI/art activation. The only newly exposed
work is the adjacent owner-index/responder-selection mechanism, recorded as a
separate residual rather than silently expanded into this slice.

## 9. Rust implementation handoff

Current Rust already provides:

- `HouseStrategyEmergencyState.last_building_attack_frame` and
  `note_building_attack` in `src/sim/house_state.rs`;
- snapshot version 96 coverage and manual world hashing;
- `EntityDamageEvent.attacker_id` separately from `source_house`;
- `ObjectType.damage_self`, `undeploys_into`, and merged art `foundation`;
- the ordered receiver commit in
  `commit_damage_events_with_isolation` (`src/sim/combat/mod.rs`).

The minimum exact writer belongs immediately before `resolve_receive_damage`
for each delivered entity receiver record. Admission must be:

```text
target category is Structure
attacker_id represents a non-null attacker object (not source_house alone)
self attacker is rejected when the shared target/source type has DamageSelf=no
target type is not (UndeploysInto present && foundation dimensions == 1x1)
```

Then write `current_tick as u32 as i32` to the target owner's House strategy
state. Do not require a currently resolvable attacker entity for non-self
records: the ordered damage record already preserves whether the native source
object argument was non-null, while an earlier record may have uninitialized
that represented source. Do not gate on alliance, damage sign, receiver flags,
immunity, victim health, limbo, HP delta, or eventual result.

Focused acceptance tests must cover null/source-House-only, allied, negative
and zero damage, immunity, already-dead victim, self `DamageSelf` false/true,
modded 1x1 undeployer, stock-shaped undeployer, repeated same-frame calls, and
signed frame wrap. The implementation review must keep `House+0x54DC` and
`FUN_00708080` visibly open rather than claiming the adjacent block complete.
