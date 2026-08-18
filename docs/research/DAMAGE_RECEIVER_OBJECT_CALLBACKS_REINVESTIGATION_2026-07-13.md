# ObjectClass ReceiveDamage and callback reinvestigation — 2026-07-13

| Sequence | Native owner | Predicate inputs | pDamage before/after |
|---|---|---|---|
| 1. Entry rejection | ObjectClass::ReceiveDamage 0x005F5390 | signed Health <= 0; requested damage == 0; or !ignoreDefenses and ObjectTypeClass+0x233 Immune | Unchanged; returns result 0 |
| 2. Armor/distance normalization | Damage kernel 0x00489180, called by ObjectClass | !ignoreDefenses; requested signed damage, WarheadType, armor at Type+0x9C, integer distance | Replaced in place by the kernel result |
| 3. CanC4 floor | ObjectClass | WhatAmI == 6 and BuildingType+0x1577 CanC4 == 0 | max(normalized damage, 1); this can convert healing into 1 damage |
| 4. Post-normalization rejection | ObjectClass | normalized damage == 0 | Unchanged at 0; returns result 0 |
| 5. Healing | ObjectClass | normalized damage < 0 | Remains negative even if Health is capped at Strength; returns result 0 |
| 6. Positive classification and HP write | ObjectClass | old Health, signed Strength, hardcoded half-Strength crossing, Rules+0x1708 red ratio | Inclusive overkill is written back as old Health; otherwise unchanged; Health becomes old Health - pDamage |
| 7. Cyborg one-time survival | ObjectClass plus Infantry virtual callback | new Health <= 0, WhatAmI == 0x0F, !ignoreDefenses, InfantryType+0xEAC Cyborg, Infantry+0x6DB == 0 | pDamage is not restored; Health is replaced with max(ftol(Strength * 0.25), 1), result becomes 3 |
| 8. Receiver trigger chain | AttachedTag dispatcher 0x006E53A0, synchronously invoked by ObjectClass | result 2/3, attacker, current tag/alive state, and first change from full | pDamage is no longer read or written; callbacks may mutate current Health/tag/alive state seen by later receiver reads |
| 9. Death/credit routing | ObjectClass then derived +0xE0/+0xE4 callbacks | fresh Health == 0; sourceHouse; attacker and attacker owner | Unchanged; result becomes 4, credit callback precedes +0xDC destruction |
| 10. Surviving attacked events and refresh | ObjectClass | current alive, result != 4, attacker/tag, then result != 0 and selected | Unchanged; events 0x06 and 0x2C are synchronous, then vtable+0x124(2) is the final receiver-side refresh |

| Sequence | Side effect/callback | Arguments | Can mutate a later read? |
|---|---|---|---|
| A | Damage kernel 0x00489180 | signed damage in ECX, WarheadType in EDX, stack armor and distance | Yes. Its return replaces pDamage before every later classification. |
| B | Healing callback vtable+0x148 | 7 | No later receiver read: the function returns result 0 immediately after the callback. The caller can observe its side effects. |
| C | Cyborg animation constructor 0x00421EA0 | Rules+0x9C animation, receiver coordinates, flags including 0x600 | Yes. Allocation/construction completes before the rescue HP/flag writes and later triggers. |
| D | Cyborg state callback vtable+0x558 | 6, 1, 0 | Yes. Current Health, AttachedTag, and Alive are read after this synchronous call. |
| E | Trigger 0x27 | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. Alive is read immediately afterward; tag and Health are read again later. |
| F | Trigger 0x2A | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. Alive, Health, and tag have later fresh reads. |
| G | Trigger 0x28 | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. Alive is read immediately afterward; tag and Health are read again later. |
| H | Trigger 0x2B | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. Alive, Health, and tag have later fresh reads. |
| I | Trigger 0x26 | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. Both 0x29 gates and the outer Alive/Health gates run later. |
| J | First trigger 0x29 | receiver, DAT_00AC1360, 0, sourceObject 0 | Yes. AttachedTag and Alive are re-read before the second 0x29. |
| K | Second trigger 0x29 | receiver, DAT_00AC1360, 0, sourceObject attacker | Yes. Alive and Health are read after it before death routing. |
| L | Object kill callback vtable+0xE0 | attacker, including null when sourceHouse is null | Yes. It is synchronous; +0xDC still follows, then Alive is read. Techno-derived dispatch reaches 0x00702D40. |
| M | House kill callback vtable+0xE4 | sourceHouse | Yes. It is synchronous; +0xDC still follows, then Alive is read. Techno-derived dispatch reaches 0x00703230. |
| N | Destruction callback vtable+0xDC | 1 | Yes. Alive is read immediately after return; result 4 is already committed. |
| O | Trigger 0x06 | receiver, DAT_00AC1360, 0, sourceObject attacker | Yes. Alive is read immediately afterward; the 0x2C gate uses current state. |
| P | Trigger 0x2C | receiver, DAT_00AC1360, 0, sourceObject attacker | Yes. Alive is read immediately afterward before refresh. |
| Q | Final refresh/mark vtable+0x124 | 2 | No later receiver read. It is the final side effect before returning the current result. |

## Verdict

Status: COMPLETE for the bounded ObjectClass receiver/callback slice.

The active gamemd.exe body at 0x005F5390 was checked in both decompiler and raw assembly. Every branch, HP/pDamage write, trigger dispatch, liveness checkpoint, kill-routing choice, and receiver-owned callback in 0x005F5390–0x005F584C is decided. Derived wrapper behavior is not claimed here; only direct reachability and the kill/refresh callback boundaries needed to understand later mutation are included.

The core implementation consequence is that this is not a single subtraction followed by deferred death cleanup. It is an ordered, re-entrant transaction. Trigger actions run synchronously, can detach the receiver's tag, and can change or destroy the receiver before the next explicit reread.

## Scope and evidence discipline

- Program checked: active /gamemd.exe in Ghidra project testProsjekt-12.1.2-test, x86 32-bit.
- Primary evidence: decompile_function and disassemble_function for 0x005F5390. Assembly is authoritative where the decompiler flattened stack aliases or virtual-call arguments.
- Narrow callees checked: 0x00489180, 0x005F92D0, 0x00523340, 0x006E53A0, 0x007265C0, 0x006DD8B0, 0x006F9DD0, 0x00702D40, 0x00703230, and 0x0074FF50.
- Vtable targets were established from raw vtable bytes/data xrefs before decompiling the target bodies. Local Ghidra names were treated as hints only.
- Research-index validation was current at preflight. The broad system-filtered query returned no focused document, so the exact 0x005F5390 evidence graph and the cited existing reports were used.
- Repo evidence checked: ini/rules.ini, ini/rulesmd.ini, src/sim/combat/mod.rs, and src/sim/components.rs.
- Excluded: concrete Techno/Building/Terrain wrapper policy before or after their call into ObjectClass, exhaustive trigger-action semantics, and the full tail of type-specific score counters inside RecordKill.

## Verified signature, storage, and result values

Raw stack use and RET 0x1C establish seven 32-bit stack slots:

| Slot | Verified use in this body |
|---|---|
| +0x04 | int32 pointer pDamage |
| +0x08 | signed/integer distance passed to 0x00489180 |
| +0x0C | WarheadType pointer |
| +0x10 | attacker TechnoClass pointer |
| +0x14 | ignoreDefenses, consumed as a byte |
| +0x18 | not consumed by this body |
| +0x1C | sourceHouse pointer |

Health at ObjectClass+0x6C and Strength at the current type+0xA0 are signed 32-bit values in this mechanism. pDamage is a signed 32-bit in/out value. This matters: negative requests are healing, the CanC4 floor can turn a negative into +1, and inclusive overkill rewrites the caller's value.

Verified return values from this body:

| Value | Exact receiver condition |
|---|---|
| 0 | Entry rejection, post-normalization zero, or healing |
| 1 | Positive HP change without the verified yellow/red crossing |
| 2 | Yellow crossing survives as the final classification |
| 3 | Red crossing, or Cyborg one-time survival |
| 4 | Fresh Health == 0 reaches credit routing and destruction |
| 5 | A pre-death yellow/red/first-change Alive check, or the positive-post-base-HP guard, aborts the receiver sequence. Later Alive failures after +0xDC/0x06/0x2C return the already-current result instead. |

The numeric values are load-bearing. Human enum display names are not used as proof in this report.

## Entry, normalization, healing, and exact classification

### Early exits and normalization

At entry the function snapshots current Health. It returns 0 when Health <= 0 or requested pDamage == 0. If defenses are not ignored, ObjectTypeClass+0x233 rejects the hit.

ObjectTypeClass::ReadINI at 0x005F92D0 proves that +0x233 is Immune. The adjacent +0x232 is Insignificant. The receiver gate is therefore Immune, not Insignificant.

When defenses are active, 0x00489180 replaces pDamage using the WarheadType, type armor at +0x9C, and distance. That kernel also decides whether negative damage reaches the healing branch: its negative path preserves the negative value for distance < 8 and returns zero at distance >= 8. The receiver then applies the Building/CanC4 rule.

For WhatAmI == 6 and BuildingType+0x1577 CanC4 == 0:

    pDamage = max(pDamage, 1)

This rule is after armor/distance normalization and is not guarded by ignoreDefenses. It turns zero or negative values into +1. Stock data makes the branch live: CAMISC01, CAMISC02, CAMISC06, and AMMOCRAT have CanC4=no in both the base and md rule sets.

### Healing

For normalized pDamage < 0:

1. Snapshot old Health.
2. Compute signed Health = old Health - pDamage.
3. If the result exceeds fresh type Strength, replace Health with Strength.
4. If final Health differs from old Health, call vtable+0x148(7).
5. Return 0.

pDamage is not changed to the actual capped heal. For example, a request of -100 can restore only five HP while the caller still sees -100.

The representative Techno-derived +0x148 target is 0x006F9DD0. Its verified body performs virtual visibility/guard calls and conditionally writes argument 7 to Techno+0xF0. Base Object's target at 0x005F4370 is a stub. Calling this merely NotifyHealthChanged would hide the verified derived behavior.

### Positive damage and thresholds

Let old be the entry Health snapshot, d be the normalized positive pDamage, strength be the signed type Strength, and new be old - min(d, old).

1. Start with result 1.
2. Yellow candidate: d < old, old >= arithmetic (strength >> 1), and old - d < arithmetic (strength >> 1). If true, set result 2.
3. If d >= old, write old back through pDamage. Equality is included.
4. Compute redThreshold in x87 as double(strength) * double(Rules+0x1708).
5. If double(old) > redThreshold and double(new) < redThreshold, set result 3. Both comparisons are strict; equality on either side does not cross.
6. Write signed Health = new.

Red classification overwrites yellow. Yellow uses the hardcoded signed half-Strength expression; ObjectClass::ReceiveDamage does not read Rules ConditionYellow at +0x1700.

### One-time Cyborg survival

After the HP write, a nonpositive Health enters a special branch only when all of these are true:

- WhatAmI returns 0x0F.
- ignoreDefenses is false.
- InfantryType+0xEAC Cyborg is set.
- Infantry+0x6DB is clear.

InfantryClass::WhatAmI at 0x00523340 returns 0x0F, so this is Infantry/Cyborg behavior, not a Building or VeinholeMonster rule. The body optionally constructs the Rules+0x9C animation at the receiver coordinates, writes Health = max(ftol(Strength * 0.25), 1), sets +0x6DB to 1, calls vtable+0x558(6, 1, 0), and forces result 3.

No uncommented Cyborg=yes was found in the stock repo INIs. The compiled path remains relevant to supported custom data, but it is not a normal stock-YR activation.

## Complete ordered read/write/call ledger

The table is in machine order. A fresh read means a prior synchronous callback can affect it.

| Order/address | Operation | Predicate or arguments | Later mutation visibility |
|---|---|---|---|
| 1 / 0x5F5390 | Snapshot signed Health | Object+0x6C | Baseline retained for full-health test and arithmetic |
| 2 / 0x5F53A1–0x5F53D5 | Return 0 gates | Health <= 0; pDamage == 0; or !ignoreDefenses and Type+0x233 | No callbacks |
| 3 / 0x5F53F3–0x5F5414 | Call 0x00489180 and write pDamage | damage, warhead, armor, distance | All later pDamage reads see the replacement |
| 4 / 0x5F5416–0x5F5454 | WhatAmI/type checks and pDamage floor | Building and !CanC4 | All later pDamage reads see at least 1 |
| 5 / 0x5F545A–0x5F5466 | Return 0 gate | pDamage == 0 | No callbacks |
| 6 / 0x5F5468–0x5F548C | Healing HP write/cap | pDamage < 0 | Fresh type Strength is used for cap |
| 7 / healing tail | vtable+0x148(7), conditional | final Health != old Health | Function returns immediately afterward |
| 8 / 0x5F5498 onward | Initialize result 1 | positive pDamage | May become 2/3/4 |
| 9 / yellow branch | Set result 2 | surviving strict half-Strength downward crossing | Red can overwrite it |
| 10 / overkill branch | Write pDamage = old Health | pDamage >= old Health | HP write consumes capped value |
| 11 / 0x5F54DE–0x5F5503 | Red x87 comparisons, set result 3 | old > threshold and new < threshold | No callback between classification and HP write |
| 12 / 0x5F5505–0x5F550D | Write Health = old - pDamage | signed int32 | Cyborg branch and triggers see it |
| 13 / 0x5F5516–0x5F55D9 | Cyborg checks, animation, HP/flag writes, +0x558 call | exact predicates above; callback args 6,1,0 | The call can affect subsequent fresh reads |
| 14 / 0x5F55EE–0x5F55F8 | Snapshot post-base Health | after HP write/Cyborg callback, before receiver triggers | Used later only by the positive-snapshot gate |
| 15 / 0x5F55FE–0x5F5617 | Trigger 0x27 | result 2, attacker nonnull, current tag nonnull; sourceObject 0 | Fresh Alive follows |
| 16 / 0x5F561C | Fresh Alive | false returns 5 | Sees 0x27 mutation |
| 17 / 0x5F562A–0x5F563E | Trigger 0x2A | result 2, current tag nonnull; attacker not required; sourceObject 0 | Fresh Alive follows at 0x5F5643 |
| 18 / 0x5F5643 | Fresh Alive | false returns 5 | Sees 0x2A mutation |
| 19 / 0x5F5656–0x5F566F | Trigger 0x28 | result 3, attacker nonnull, current tag nonnull; sourceObject 0 | Fresh Alive follows |
| 20 / 0x5F5674 | Fresh Alive | false returns 5 | Sees 0x28 mutation |
| 21 / 0x5F5682–0x5F5696 | Trigger 0x2B | result 3, current tag nonnull; attacker not required; sourceObject 0 | Fresh Alive follows at 0x5F569B |
| 22 / 0x5F569B | Fresh Alive | false returns 5 | Sees 0x2B mutation |
| 23 / 0x5F56A9–0x5F56D6 | First-change-from-full gate | fresh Health != entry Health; fresh GetType nonnull; entry Health == fresh Strength | Previous triggers can make or erase the Health-difference test |
| 24 / 0x5F56D8–0x5F56F1 | Trigger 0x26 | first-change gate, attacker nonnull, current tag nonnull; sourceObject 0 | Later tag/alive reads see mutation |
| 25 / 0x5F56F6–0x5F5714 | First trigger 0x29 | first-change gate, current tag nonnull and current Alive; sourceObject 0 | Second 0x29 re-reads tag/alive |
| 26 / 0x5F5719–0x5F573F | Second trigger 0x29 | first-change gate; current tag must exist, current Alive false returns 5, attacker must exist; sourceObject attacker | Outer Alive and HP gates see mutation |
| 27 / 0x5F5744 | Fresh Alive | false returns 5 | Sees all first-change callbacks |
| 28 / 0x5F5752–0x5F575F | Positive-snapshot guard | if post-base snapshot > 0, fresh Health must remain > 0 else return 5 | A callback-induced death from positive base HP does not enter credit routing here |
| 29 / 0x5F5765–0x5F576C | Fresh death test | Health == 0 exactly | Trigger healing can rescue a zero snapshot; callback-created negative HP is not equal to zero |
| 30 / 0x5F576E–0x5F579A | Route kill callback | sourceHouse null or equals nonnull attacker owner -> +0xE0(attacker); otherwise +0xE4(sourceHouse) | Callback precedes destruction and may mutate state |
| 31 / 0x5F57A0–0x5F57AF | Commit result 4; call vtable+0xDC(1) | exact zero-Health death route | Fresh Alive follows |
| 32 / 0x5F57B5 | Fresh Alive | false returns current result | Sees kill and destroy callbacks |
| 33 / 0x5F57BF–0x5F57DB | Trigger 0x06 | attacker and current tag, result != 4; sourceObject attacker | Fresh Alive follows |
| 34 / 0x5F57E0 | Fresh Alive | false returns current result | Sees 0x06 mutation |
| 35 / 0x5F57EA–0x5F5807 | Trigger 0x2C | attacker and current tag, result != 4; sourceObject attacker | Fresh Alive follows |
| 36 / 0x5F580C | Fresh Alive | false returns current result | Sees 0x2C mutation |
| 37 / 0x5F5816–0x5F582A | Final vtable+0x124(2) refresh/mark | result != 0 and selected byte +0x83 != 0 | Final receiver operation; caller observes effects |
| 38 / 0x5F5830 or 0x5F583E | Return | current result or forced 5 | RET 0x1C |

The two 0x29 dispatches are therefore not interchangeable and are not an any-damage pair. They are adjacent calls inside one first-change-from-full gate, with a new tag/alive check between them and different sourceObject arguments.

## Trigger dispatch, synchronous re-entry, and callback mutation

At every receiver event call, ObjectClass loads AttachedTag from receiver+0x34 into ECX and calls 0x006E53A0 with five explicit stack arguments:

    event, receiver, DAT_00AC1360, 0, sourceObject

The local name TechnoClass__ProcessCellAction is misleading for this use. The ECX receiver is the current Tag object.

0x006E53A0 checks the tag disabled/busy bytes, evaluates its trigger list, and synchronously invokes the action-list runner at 0x007265C0. That runner loops TriggerAction entries and calls TriggerAction::Execute at 0x006DD8B0 before returning to ObjectClass. It can also detach the receiver's tag through 0x005F5B50 when the receiver still owns that tag.

Concrete bounded proof that these calls cannot be modeled as queued notifications:

- An action path through case 0x2A reaches area damage at 0x00489280.
- An action path through case 0x3F reaches area damage with the C4 warhead.
- An action path through case 0x6F synchronously sells a Building.

The per-tag busy byte prevents a simple same-tag recursive evaluation, but it does not make actions side-effect-free or defer changes. Other objects/tags and nested damage paths remain reachable. Thus each explicit tag, Alive, and Health reread in the ledger is semantically required.

## Object-versus-house kill routing, scoring, and XP boundary

The death route is based on fresh Health == 0 after trigger processing:

| sourceHouse / attacker relation | Receiver virtual call |
|---|---|
| sourceHouse is null, regardless of attacker nullability | vtable+0xE0(attacker) |
| sourceHouse nonnull, attacker nonnull, and sourceHouse == attacker+0x21C Owner | vtable+0xE0(attacker) |
| sourceHouse nonnull and attacker is null or owned by another House | vtable+0xE4(sourceHouse) |

Base Object targets 0x005F42F0 (+0xE0) and 0x005F4300 (+0xE4) are stubs. Representative Techno-derived vtables resolve +0xE0 to TechnoClass::RecordKill at 0x00702D40 and +0xE4 to 0x00703230.

Both Techno-derived paths synchronously run the victim's own tag-event sequence before their score work, with fresh Alive checks between events. That callback chain can mutate or detach the victim before ObjectClass proceeds to +0xDC. ObjectClass does not cancel +0xDC based on a new Health read; it calls destruction after the selected credit callback, then checks Alive.

The verified boundary between object credit and house credit is:

- Object path 0x00702D40 can resolve an XP recipient. Its priority chain includes the linked eligible object, the killer itself, a MissileSpawn parent, or the live occupant of an occupied Building. DontScore on the victim type exits before XP/scoring.
- The object path calls VeterancyClass::AddExperience at 0x0074FF50 with the recipient veterancy state, recipient cost, and victim value scaled by victim veterancy tier (rookie x1, veteran x2, elite x3). Allied kills yield zero scaled value.
- 0x0074FF50 adds scaledVictimValue / (recipientCost * Rules.VeteranRatio), using its verified Rules offsets, then clamps to VeteranCap.
- House path 0x00703230 has no killer object and does not call AddExperience. It still performs house/stat attribution using sourceHouse.
- The object path writes victim-owner House+0x548C from the attacker type identity and increments attacker-owner House+0x54E8 by scaled value. The house path writes +0x548C from sourceHouse+0x30 and increments sourceHouse+0x54E8.

This report does not claim that the tail of every type-specific house counter is exhaustively modeled. It does establish the receiver's routing predicate, callback order, which path owns XP, and the state mutations visible before destruction.

## Active-YR reachability

Direct code xrefs to 0x005F5390 establish:

| Caller | Evidence and relevance |
|---|---|
| TechnoClass::ReceiveDamage 0x00701900, call at 0x00701DF8 | Normal live unit/building damage pipeline |
| TerrainClass::Take_Damage 0x0071B920, call at 0x0071B986 | Live terrain/tree damage path |
| Function 0x0074D5D0, call at 0x0074D5FC | Vtable data at 0x007F6814 identifies the VeinholeMonster wrapper; TS/vein legacy is dormant in stock YR |

Additional vtable data refs place 0x005F5390 in Object-derived virtual tables, so virtual damage sites can reach it through slot +0x16C. Concrete wrapper-specific preprocessing/postprocessing is outside this report.

## Rust comparison

Current Rust does not preserve this receiver transaction:

- src/sim/components.rs:83–87 stores current/max HP as u16, while the native receiver uses signed 32-bit Health, Strength, and pDamage semantics.
- src/sim/combat/mod.rs:1849–1884 applies queued u16 damage with saturating_sub, refreshes a building damage gate, applies fear, records zero-HP IDs, and writes last_attacker_id. It has no pDamage in/out value, negative healing branch, inclusive overkill writeback, Immune gate at this layer, CanC4 floor, result 0–5, or ordered trigger callback chain.
- src/sim/combat/mod.rs:1896–1909 defers deaths to handle_entity_deaths. Native ObjectClass credit/destruction happens inside the receiver after synchronous trigger callbacks and fresh Health/Alive checks.
- src/sim/components.rs:580–597 derives display DamageState from fixed 50%/25% u16 ratios. That is not the receiver transition classifier: yellow is a crossing event using signed Strength >> 1, red is a strict crossing against Rules ConditionRed, and both depend on old and new HP.
- No current combat surface was found for receiver events 0x26–0x2C, the two distinct 0x29 dispatches, object-versus-house kill callbacks, RecordKill recipient routing, or synchronous trigger mutation.

These are DRIFT findings, not an assertion that the current display DamageState helper itself must be deleted. Display classification and native receiver transition/result semantics are distinct consumers.

## Contradictions and stale wording

| Existing claim | Binary result | Disposition |
|---|---|---|
| RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md:203 and :756 call Type+0x233 Insignificant | ObjectTypeClass::ReadINI 0x005F92D0 maps Immune to +0x233 and Insignificant to +0x232 | WRONG; receiver early gate is Immune |
| RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md:272–281 and DAMAGE_MATH_GHIDRA_REPORT.md:448–460 treat WhatAmI 0x0F as Building/NUKE/Veinhole uncertainty | InfantryClass::WhatAmI 0x00523340 returns 0x0F; the branch reads InfantryType+0xEAC Cyborg and Infantry+0x6DB | WRONG/MISLEADING; it is Cyborg Infantry survival |
| DAMAGE_MATH_GHIDRA_REPORT.md:480 says event 0x29 means any damage | Both calls sit under fresh Health != entry Health and entry Health == fresh Strength | WRONG; both are first change from full |
| RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md:306 says the attacker-bearing 0x29 is first damaged “any” | It shares the same outer first-change-from-full block as the first 0x29 | MISLEADING; only its sourceObject differs |
| DAMAGE_MATH_GHIDRA_REPORT.md:481–482 describes 0x2A/0x2B as with-attacker variants | Assembly pushes sourceObject 0 and does not require attacker; 0x27/0x28 require attacker but also push sourceObject 0 | WRONG argument/predicate description |
| Older flattened summaries imply trigger handling follows death | Assembly dispatches yellow/red/first-change triggers before the fresh death test and credit routing | WRONG order; callbacks can rescue, kill, or detach first |
| Any prose saying ObjectClass yellow uses Rules ConditionYellow | The receiver uses signed Strength >> 1; only red reads Rules+0x1708 | WRONG for this function |

This new report records corrections but does not edit the older documents.

## Adversarial fixtures that distinguish the mechanism

1. Healing a normal object: Health 95, Strength 100, pDamage -20, distance < 8, defenses active. Final Health is 100, result 0, +0x148(7) runs, and pDamage remains -20.
2. Healing a CanC4=no Building: same negative request. The post-kernel floor changes pDamage to +1, so the receiver damages rather than heals.
3. Inclusive overkill: Health 10, pDamage 10 or 100. Both write pDamage = 10, Health = 0, then run triggers/credit/destruction in native order.
4. Yellow boundary equality: Strength 101 gives arithmetic Strength >> 1 == 50. A move 50 -> 49 does not cross because old >= 50 is true and new < 50 is true; a move 51 -> 50 does not cross because new equals 50. This catches <= substitutions and non-arithmetic-half implementations.
5. Red equality: if old or new equals the floating red threshold, the strict crossing is false.
6. First change from full without attacker: only the first 0x29 can fire, and it carries sourceObject 0; 0x26 and the second 0x29 require attacker.
7. Tag detachment during 0x26: both later 0x29 calls can disappear because each reads current AttachedTag rather than a saved pointer.
8. Trigger kills a receiver whose post-base HP was positive: the positive-snapshot guard returns 5; this invocation does not enter normal zero-HP credit/destruction routing.
9. Trigger heals a receiver whose post-base HP was zero: the fresh exact-zero test can be bypassed, so kill routing is skipped.
10. sourceHouse differs from attacker Owner: +0xE4(sourceHouse) runs, with no object XP callback, even though attacker is nonnull.

## Implementation handoff

Each row gives the required behavior-to-test chain. This is a contract input, not permission to write Rust.

| Behavior | Binary/retail evidence | Current Rust delta | Rust surface | Acceptance | Proposed test name | Risk if omitted |
|---|---|---|---|---|---|---|
| Signed pDamage normalization, zero/Immune gates, CanC4 floor, healing/cap, inclusive overkill writeback | 0x005F53A1–0x005F550D; parser 0x005F92D0; stock CanC4=no INIs | u16 saturating subtraction has no signed in/out request or heal path | combat damage request/receiver API; Health representation decision | Fixture table reproduces final HP, result, and returned pDamage for zero, heal, floor, and overkill cases | object_receive_damage_signed_request_matrix | Healing, modded warheads, and caller-visible damage drift |
| Exact result 0–5 and yellow/red crossings | 0x005F5498–0x005F550D, x87 red comparison, forced 5 exits | Display ratio helper is not a transition result; combat returns no native result | receiver result enum and condition-transition code | Exhaustive boundary tests around odd/even Strength and red equality | object_receive_damage_condition_crossing_boundaries | Wrong trigger cadence, wrapper branch, and UI/audio consequences |
| Ordered synchronous event sequence including both 0x29 calls | 0x005F55EE–0x005F5765; dispatcher 0x006E53A0 | No receiver event pipeline; death is batched | sim-owned trigger dispatcher integration and receiver transaction | Event log exactly matches predicate/order/arguments; callbacks can detach tag or alter HP and later gates observe it | object_receive_damage_callback_order_and_mutation | Re-entry, campaign triggers, and same-tick state diverge |
| Exact death gate and object-vs-house routing | 0x005F5752–0x005F57AF | Zero HP is merely queued for later generic death handling | receiver/death lifecycle and source attribution | Positive-snapshot callback death returns 5; native zero route selects +E0 or +E4 by exact predicate before destruction | object_receive_damage_death_credit_route | Wrong killer, missing house credit, and reordered teardown |
| Object RecordKill XP boundary versus house-only credit | 0x00702D40, 0x00703230, AddExperience 0x0074FF50 | No equivalent receiver callback/XP path found | veterancy, House stats, garrison/missile parent attribution | Object path can award routed XP; house path never calls AddExperience; DontScore/allied cases match | object_receive_damage_record_kill_xp_boundary | Veterancy and score economy drift |
| Cyborg one-time survival | 0x005F5516–0x005F55D9; Infantry WhatAmI 0x00523340; InfantryType parser | No equivalent branch identified | infantry damage-state/lifecycle callback | Exact predicate writes 25%-ftol min 1, +0x6DB, result 3, args 6/1/0; second lethal hit cannot repeat | object_receive_damage_cyborg_one_time_survival | Custom-data and legacy-compatible infantry behavior drift |
| Final surviving attacked events and selected refresh | 0x005F57B5–0x005F582A | last_attacker write and building gate refresh are differently ordered/mechanized | receiver tail, selection/render invalidation boundary | 0x06 then Alive, 0x2C then Alive, then selected +0x124(2); result 4 skips both events | object_receive_damage_survivor_tail_refresh_order | Retaliation/trigger/UI refresh timing drift |

Recommended implementation shape under the project architecture is a sim-owned receiver transaction with explicit callbacks/lifecycle authority. It must permit synchronous trigger effects and commit in native order. A pure “calculate damage then emit events later” helper cannot satisfy the verified reread semantics.

## Coverage ledger

| In-scope item | State | Evidence |
|---|---|---|
| Active callsites | RESOLVED | Direct xrefs and vtable refs; Techno and Terrain live, Veinhole legacy contextualized |
| HP/Strength/pDamage widths and writes | RESOLVED | Raw 32-bit MOV/FILD/SUB/CMP operations in 0x005F5390 |
| Early exits and Immune identity | RESOLVED | Receiver assembly plus ObjectTypeClass::ReadINI 0x005F92D0 |
| Armor/distance handoff | RESOLVED | Callsite plus 0x00489180 body |
| Healing and Strength cap | RESOLVED | Receiver assembly and +0x148 target boundary |
| Inclusive overkill | RESOLVED | Jcc and pDamage store before HP subtraction |
| Result classification | RESOLVED | Result local/EBP flow and x87 threshold assembly |
| Cyborg survival | RESOLVED | Receiver body, Infantry WhatAmI, parser/INI evidence |
| Events 0x26–0x2C, including two 0x29 calls | RESOLVED | Exact caller assembly, predicates, sourceObject pushes, and order |
| Callback mutation visibility | RESOLVED | Every callback paired with its next fresh tag/Alive/Health read |
| Object-vs-house kill routing | RESOLVED | 0x005F576E–0x005F579A |
| Kill-credit/stat/XP boundary | RESOLVED | Techno-derived vtable targets, RecordKill/house callback/AddExperience bodies |
| Destruction and final refresh checkpoints | RESOLVED | +0xDC, Alive reread, events 0x06/0x2C, selected +0x124 |
| Trigger synchronous re-entry | RESOLVED | Dispatcher/action runner/action examples and receiver rereads |
| Rust delta and acceptance chain | RESOLVED | Direct current-code comparison and named tests above |

## Negative facts and do-not-do list

- Do not label ObjectType+0x233 Insignificant; it is Immune.
- Do not model pDamage as unsigned or input-only.
- Do not let CanC4=no Building healing pass through as healing; the native post-kernel floor produces +1.
- Do not use Rules ConditionYellow for ObjectClass transition event classification.
- Do not merge the two 0x29 calls or call either one an unconditional any-damage event.
- Do not attach the attacker as sourceObject to 0x27, 0x28, 0x2A, 0x2B, or 0x26; all push zero. Attacker is a predicate only for 0x27, 0x28, and 0x26.
- Do not queue trigger actions past the receiver. Their mutations are visible to the next explicit read.
- Do not decide death solely from the HP written before callbacks. Native code uses both the saved post-base snapshot and fresh HP.
- Do not award object XP on the house-only +0xE4 route.
- Do not treat WhatAmI 0x0F survival as Veinhole/Building behavior.
- Do not replace credit-before-destroy with generic deferred despawn without proving identical callback and state order.

## Remaining uncertainty and final open questions

No G1 blocker remains for the bounded receiver/callback acceptance rows.

| Question | State | Why it does not block this report |
|---|---|---|
| What human-facing enum names correspond to every numeric trigger code? | DEFERRED, non-behavioral | Numeric codes, predicates, order, and exact arguments are verified; names are not needed to implement dispatch faithfully |
| What does every concrete subclass do in its +0x124 refresh override? | OUT OF SCOPE | The receiver's predicate, argument, ordering, and no-later-read boundary are verified; wrapper/subclass rendering policy needs its own slice |
| What does every tail counter in 0x00702D40/0x00703230 represent? | OUT OF SCOPE | The receiver routing, XP/no-XP split, principal House writes, and callback mutation boundary are decided |
| Can stock retail runtime create Cyborg=yes through a source other than the stock INIs? | UNCHECKED outside normal stock data | It does not alter the verified compiled predicate or the implementation requirement for supported custom data |

Final answer for implementation planning: preserve signed in/out damage, exact classification, both first-full-health 0x29 dispatches, synchronous callback mutation, fresh death checks, object/house credit split, credit-before-destroy, and the final selected refresh as one ordered sim transaction.
