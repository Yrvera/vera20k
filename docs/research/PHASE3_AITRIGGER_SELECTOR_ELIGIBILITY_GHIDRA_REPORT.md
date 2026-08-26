# Phase 3 AITrigger Selector and Eligibility Coverage Map

**Date:** 2026-08-26
**Investigation mode:** coverage-map
**Binary:** active retail Yuri's Revenge gamemd.exe
**Primary addresses:** 0x006F0AB0, 0x0041E720, 0x004F8440, 0x004F70D0, 0x005010D0, 0x00509610, 0x005F7900, 0x0041FD60, 0x0041FE20, 0x006E8DE0
**Confidence:** High for the scoped call graph, branch predicates, ordering, arithmetic, persistence, and retail-data census.
**Active in YR:** Yes. HouseClass::Update is the sole selector caller, retail AIMD supplies 165 AITriggerTypes, and stock TeamDelays repeatedly arms the path for ordinary nonhuman Houses.

## 1. Verdict

The Stage-B ordinary-skirmish AITrigger selector is now mechanism-closed for research. The active path is not an Autocreate TeamType picker. It is a House-timed, two-draw maximum, registry-ordered weighted selector with a live base-defense/cap prepass, a complete eligibility pipeline, an exact priority tier at truncated weight 5000, a same-Team cancellation pass, and adaptive per-trigger weights updated by Team destruction.

This report resolves the Stage-B evidence gaps recorded as OQ-10, OQ-11, OQ-14, OQ-15, and the Stage-B portion of OQ-29 in PHASE3_TEAM_PRODUCTION_REACHABILITY_GHIDRA_REPORT.md. It does not claim that Rust implements the selector. It also does not close Stage-C Team recruitment or the end-to-end production-reachability row.

The most load-bearing implementation constraints are:

- the probability draw occurs before the ratio and active-latch checks, so even an inactive selector spends one scenario RNG draw on every expired House timer;
- fixed AIMD and map AITrigger rows are one ordered registry with in-place map override, source provenance, and separate enable-section behavior;
- eligibility conditions 0 through 7 have distinct bodies; condition payload comparison applies only to 0, 1, 4, and 7;
- object counting uses type-class WhatAmI values Aircraft=3, Building=7, Infantry=0x10, Unit=0x28, not runtime object-class values;
- a first eligible truncated weight of 5000 discards every previously accumulated non-5000 candidate, later 5000 rows join the tier, and later non-5000 rows are ignored;
- adaptive success/failure updates affect every trigger sharing the destroyed Team's primary TeamType, before that Team is removed;
- the full 0x110-byte AITrigger object, including current weight and both history counters, is saved and loaded, but the native CRC callback does not directly include the counters;
- selected primary and secondary TeamTypes are output in that order, may be the same pointer, and are each rechecked against Max by the actual Team creator.

## 2. Scope and evidence boundary

### 2.1 Included

- ordinary House selector cadence and House admission;
- fixed AIMD versus map AITrigger provenance and enable semantics;
- ratio and active-latch writers;
- total/base-defense caps and the eviction branch;
- every eligibility gate reachable from 0x0041E720;
- every condition enum body from -1 through 7 and unknown-enum rejection;
- zone, TaskForce factory availability, and TeamType Max gates;
- weighted selection, primary/secondary output, duplicate suppression, and Autocreate writes;
- adaptive weight feedback, Team success coupling, save/load, and CRC projection;
- retail AIMD/rules census, stock-live mechanisms, and evidence-backed stock exclusions;
- current Rust ownership gaps and an exact implementation handoff.

### 2.2 Excluded

- Stage-C candidate recruitment at 0x006EA610 and its callees;
- ScriptType opcodes other than the already-required action 49 success writer;
- campaign forced-team and reinforcement creators;
- generic strategic AI outside this selector corridor;
- Railgun, LaserDraw, Sonic Wave, destroyable-cliff presentation, and TS legacy.

These exclusions do not leave a question inside the Stage-B mechanism. They identify separate owners which must not be approximated through this selector.

### 2.3 Evidence provenance

Live Ghidra was connected to project testProsjekt, program gamemd.exe, on 2026-08-26. Function callers/xrefs, decompilation, disassembly, raw vtable bytes, and full-program instruction searches were read from that session. Cold assembly checks also decoded bytes directly from:

C:\Users\enok\Documents\Command and Conquer Red Alert II\gamemd.exe

Retail data was read from:

- C:\Users\enok\Documents\vera20k-docs-backup\ini\aimd.ini
- C:\Users\enok\Documents\vera20k-docs-backup\ini\rulesmd.ini

SHA-256:

- aimd.ini: 5DF41EAEC00A78D0760EF5EECDF27D65AE1CD537309C7EAC973318266986F89D
- rulesmd.ini: 3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF

Prior verified zone behavior is incorporated from PHASE3_TEAMTYPE_POSTLOAD_ZONE_DERIVATION_GHIDRA_REPORT.md. The available-wallet binding at 0x004F6990 is cross-checked against PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md. Binary evidence in this report overrides stale semantic labels in older navigation documents.

## 3. Registry provenance and enable state

### 3.1 Fixed then map, with in-place identity reuse

ScenarioClass::Full_Init calls the AITrigger loader twice near 0x006879DA and 0x006879E3:

1. fixed AIMD pass with source argument 1;
2. map pass with source argument 0.

AITrigger loader 0x0041F2E0 walks [AITriggerTypes] in INI entry order. An existing identifier is reused and re-read in place; a new identifier allocates a 0x110-byte object and appends to the global registry. After each successful read it stores the current pass source at AITrigger+0x9C. Therefore a map override of a fixed identifier becomes source 0 without moving its registry position.

The fixed pass forces AITrigger+0xA4 enabled to true. The map pass does not blanket-enable rows. It separately walks [AITriggerTypesEnable]:

- in g_GameMode==0, a listed false value clears +0xA4 and a listed true value sets it;
- in g_GameMode!=0, any listed key sets +0xA4 regardless of the parsed boolean value;
- an unlisted fixed row retains its forced enabled state;
- an unlisted map-new row retains the constructor default false;
- an unlisted map override retains the pre-existing row's enabled byte.

AITrigger+0x9C==1 is also subject to Scenario+0x34B4 IgnoreGlobalAITriggers. Source 0 rows bypass that rejection, including map overrides of fixed IDs.

### 3.2 Tokens 11 through 14

The 18-token AITrigger row stores:

| 1-based token | Native field/result | Scoped meaning |
|---|---:|---|
| 11 | +0xD0 byte | required only when g_GameMode!=0; ordinary-mode enable |
| 12 | consumed, no retained destination | parser padding/inert token in this binary |
| 13 | +0xAC signed int | acting-side restriction: 1 Allied, 2 Soviet, 3 Yuri, other unrestricted |
| 14 | +0xD1 byte | serialized and CRC-fed, but no gameplay reader exists |

A full-program instruction scan for AITrigger+0xD1 found only constructor initialization at 0x0041E3DD, parser write at 0x0041F988, CRC read at 0x0041E6E7, and serialization/export-style read at 0x0041FBE5. It has no selector, eligibility, Team, House, or other gameplay consumer. This is an active-binary inert serialized field, not an unverified semantic flag.

Retail values are token11=1 on all 165 rows, token12=0 on all rows, token14=0 on all rows, and token13 distribution 1:68, 2:52, 3:45.

## 4. House state, writers, and cadence

### 4.1 Ratio gate

House+0x565C is the signed RatioAITriggerTeam value:

- constructor default 100 at 0x004F5BDD;
- scenario INI RatioAITriggerTeam= read/write at 0x00500D0D..0x00500D25;
- TriggerAction case 76 writes an arbitrary action value at 0x006DF364;
- selector compares the first RandomRanged(1,100) result against it at 0x006F0B06.

The selector always performs that draw before checking the ratio or active latch. Stock generated skirmish Houses therefore consume the draw and pass the default 100 ratio gate.

### 4.2 Active latch

House+0x1F2 is the selector-active byte:

- constructor initializes zero at 0x004F570A;
- TriggerAction case 74 AITriggersBegin writes one at 0x006DF2FA;
- TriggerAction case 75 AITriggersStop writes zero at 0x006DF339;
- HouseClass::ComputerTakeover writes one at 0x0050A7F6;
- successful UnitClass MCV deploy writes one at 0x0073990C;
- selector reads it at 0x006F0B12.

The instruction scan found exactly these constructor/read/writer sites plus an unrelated read elsewhere. Rust must not infer this latch from is_human, current Team count, or Autocreate metadata.

### 4.3 Timer seed and repeat

House+0x5798/+0x579C/+0x57A0 is the selector timer. HouseClass::SetDifficulty at 0x004F70D0 and HouseClass::Read_Scenario_INI near 0x005010FF seed:

- start = current frame;
- duration = Rules.TeamDelays[difficulty] + House.ArrayIndex * 175.

Only the first interval has the House-index stagger. At expiry, HouseClass::Update 0x004F8A00:

1. checks whether the House is admitted;
2. calls selector 0x006F0AB0;
3. forwards output TeamTypes in order to 0x006F09C0;
4. resets start to current frame and duration to Rules.TeamDelays[difficulty], with no repeat stagger.

The reset occurs even when selection produces no output.

Stock Rules.TeamDelays is 2000,2500,3500 in native Hard/Normal/Easy index order. The INI comment says easy/medium/hard, but House difficulty indexing and the binary table resolve the stored order as hardest first.

### 4.4 House admission and scheduler order

At timer expiry:

- g_GameMode==0 skips House+0x1EC IsHuman or House+0x1ED PlayerControl;
- g_GameMode!=0 skips only House+0x1EC IsHuman;
- all modes skip HouseType+0x1A6 MultiplayPassive.

The global TeamClass pass precedes HouseClass updates. A Team created by this House tail cannot execute Team AI or recruit until the next game frame.

## 5. Selector prepass and cap behavior at 0x006F0AB0

### 5.1 Live counts

After ratio/latch admission the selector scans the global Team registry in current order and counts:

- every live Team whose Team+0x2C owner equals the acting House;
- among those, every Team whose TeamType+0xF6 IsBaseDefense is true.

Rules arrays are indexed by House+0x184 difficulty:

- Rules+0x13CC points to TotalAITeamCap;
- Rules+0x13B0 points to MaximumAIDefensiveTeams;
- Rules+0x1394 points to MinimumAIDefensiveTeams;
- Rules+0x17F3 is UseMinDefenseRule.

House+0x566C is the maintained base-defense Team count. Team construction increments it at 0x006E8D14 and Team destruction decrements it at 0x006E8E58.

### 5.2 Exact limit and eviction branches

Let total be owned live Team count and defense be owned base-defense Team count.

If total < TotalAITeamCap OR defense < trunc(total / 2):

- no Team is evicted;
- suppressDefense becomes true only when MaximumAIDefensiveTeams < defense;
- equality with MaximumAIDefensiveTeams still permits one more base-defense selection.

Otherwise:

- scan owned base-defense Teams forward;
- choose the Team with strictly smallest signed Team+0x50;
- equal priorities retain the earlier global Team;
- invoke vtable+0x20 with argument 1, which resolves to the Team scalar-deleting destructor;
- decrement the selector's local total before destruction;
- set suppressDefense true only when a candidate existed.

Eligibility is entered only if the resulting local total remains below TotalAITeamCap.

The deletion is synchronous. Team destruction updates matching AITrigger weights before member removal, per-TeamType count decrement, base-defense count decrement, and global Team registry removal. The same selector invocation therefore sees the new trigger weights and the removed Team in its later eligibility walk.

### 5.3 Stock exclusion for the eviction branch

Retail has TotalAITeamCap=30, MinimumAIDefensiveTeams=1, MaximumAIDefensiveTeams=2, and UseMinDefenseRule=yes at all three difficulties.

The strict Maximum < defense test allows selector-produced defense count to reach 3, then suppresses further base-defense triggers. With defense at most 3 and total at least 30, defense < trunc(total/2) is always true. Therefore ordinary unmodified selector-only retail play cannot enter the eviction branch.

The branch remains active for map/custom limits, preplaced or separately created Teams, and other Team creators. It is an evidence-backed stock exclusion, not dead code.

## 6. Eligibility pipeline at 0x0041E720

The only gameplay caller of 0x0041E720 is selector 0x006F0AB0 at 0x006F0C58. Arguments are the trigger receiver, acting House, target/enemy House, and suppressDefense byte.

### 6.1 Base-defense preamble

hasBaseDefense is true when either primary TeamType+0xF6 or secondary TeamType+0xF6 is true. Primary TeamType+0xDC must be non-null.

If target is null, or if UseMinDefenseRule is true and House+0x566C is below MinimumAIDefensiveTeams[difficulty]:

- require hasBaseDefense;
- reject when suppressDefense is true.

Otherwise, a base-defense-containing trigger is rejected only when suppressDefense is true. A mixed trigger whose primary is base defense and secondary is not still qualifies as a base-defense trigger.

### 6.2 Source, enabled, session, and difficulty

The trigger is rejected when:

- source +0x9C is fixed and Scenario.IgnoreGlobalAITriggers is true;
- enabled +0xA4 is false;
- g_GameMode!=0 and token11/+0xD0 is false;
- the selected difficulty byte is false.

Difficulty mapping is deliberately asymmetric:

- g_GameMode!=0 uses House difficulty 0/1/2 -> +0xD4/+0xD3/+0xD2, corresponding to Hard/Normal/Easy tokens 18/17/16;
- g_GameMode==0 uses Scenario difficulty 0/1/2 -> +0xD2/+0xD3/+0xD4, corresponding to Easy/Normal/Hard.

### 6.3 Owner, side, and tech

AITrigger+0xA0 owner mode:

- 0 rejects;
- 2 accepts every acting country;
- 1 requires acting CountryType+0xB8 country index == trigger+0xA8;
- other nonzero values accept.

AITrigger+0xAC side restriction:

- 1 requires House+0x1E8 == 0 Allied;
- 2 requires House+0x1E8 == 1 Soviet;
- 3 requires House+0x1E8 == 2 Yuri;
- other values are unrestricted.

Acting House+0x1D4 TechLevel must be at least trigger+0xB0, the Stage-A TaskForce-derived threshold.

### 6.4 Complete condition truth table

| Condition +0x98 | Target null | Target exists |
|---:|---|---|
| -1 | pass | pass |
| 0 | fail | count target-owned instances of trigger object; compare payload |
| 1 | calls acting-count helper with null target and returns false | count acting-owned instances; compare payload |
| 2 | fail | target PowerOutput - PowerDrain < 100 |
| 3 | fail | target PowerOutput - PowerDrain < 0 |
| 4 | fail | compare target available wallet against payload |
| 5 | fail | acting first Iron Curtain instance is at least AIMinorSuperReadyPercent charged |
| 6 | fail | acting first Chronosphere instance is at least AIMinorSuperReadyPercent charged |
| 7 | fail | count instances owned by first Civilian-side House; compare payload |
| other | fail | fail |

Retail condition distribution is 0:50, 1:49, 4:53, 5:2, 6:3, 7:8. No retail row uses -1, 2, or 3. Consequently a null enemy House produces no stock output, but -1 remains an active map/custom escape.

### 6.5 Payload and object family

AITrigger+0xE4 is a little-endian signed amount from the first four bytes of the 32-byte comparison payload. AITrigger+0xE8 is a little-endian signed comparator from the next four bytes:

| Comparator | Result |
|---:|---|
| 0 | actual < amount |
| 1 | actual <= amount |
| 2 | actual == amount |
| 3 | actual >= amount |
| 4 | actual > amount |
| 5 | actual != amount |
| other | false |

Payload comparison is used by conditions 0, 1, 4, and 7. Conditions 2, 3, 5, and 6 ignore it.

For count conditions, trigger+0xD8 is a type object. Its type-class vtable+0x2C WhatAmI selects the House IndexClass:

| WhatAmI | Family | House count field |
|---:|---|---:|
| 3 | AircraftTypeClass | +0x558C |
| 7 | BuildingTypeClass | +0x5550 |
| 0x10 | InfantryTypeClass | +0x5578 |
| 0x28 | UnitTypeClass | +0x5564 |

Any other value or null type counts as zero. HouseClass::CountOwnedInstances 0x0049FAE0 is an IndexClass lookup by the type registry index returned through vtable+0x40. It auto-expands missing slots as zero; it is not a live-object scan.

The family constants were independently cold-checked through primary type vtables:

- AircraftType vtable 0x007E2868 +0x2C -> 0x0041CFB0 -> MOV EAX,3;
- BuildingType vtable 0x007E4570 +0x2C -> 0x00465D90 -> MOV EAX,7;
- InfantryType vtable 0x007EB610 +0x2C -> 0x00524D40 -> MOV EAX,0x10;
- UnitType vtable 0x007F6218 +0x2C -> 0x00748170 -> MOV EAX,0x28.

These are type-class values and must not be confused with runtime object values Aircraft=2, Building=6, Infantry=0xF, Unit=1.

The two middle House IndexClass labels were independently resolved from the
live lifecycle switch rather than inherited from an older field map:
HouseClass::Added_To_Game 0x00502A80 dispatches runtime Unit=1 to +0x5564,
Aircraft=2 to +0x558C, Building=6 to +0x5550, and Infantry=0xF to +0x5578.
HouseClass::Removed_From_Game applies the matching decrements at
0x0050291D, 0x005029A7, 0x005027C1, and 0x00502A41. Therefore +0x5564 is
the Unit IndexClass and +0x5578 is the Infantry IndexClass; older documentation
which swaps those two labels is stale.

### 6.6 Condition 4 available wallet

Condition 4 calls the target House embedded economy object's vtable+0x18, bound to 0x004F6990. The exact signed result is:

ftol(StorageClass::GetTotalValue(House+0x2FC) * HouseType.IncomeMult(+0x148) + House.Balance(+0x30C))

This is then compared with +0xE4 using +0xE8. It is not raw credits-only.

### 6.7 Conditions 5 and 6 superweapon readiness

The acting House's SuperClass vector at +0x258/count +0x264 is scanned forward. The first SuperWeaponType+0xB4 matching kind 1 Iron Curtain or kind 3 Chronosphere is decisive. The scan stops at the first match even if that instance is inactive or not ready; later same-kind instances are not considered.

The matching instance must have +0x6D active. Remaining charge is:

- duration +0x38 when start +0x30 is -1;
- otherwise duration minus elapsed current frames, clamped to zero once elapsed >= duration.

Recharge is SuperClass::GetRechargeTime 0x006CC260. Eligibility is:

remaining / recharge <= 1.0 - Rules.AIMinorSuperReadyPercent(+0xD70)

Stock AIMinorSuperReadyPercent=.7, so the weapon qualifies at least 70% through recharge. Positive/zero yields infinity and zero/zero yields NaN; both fail the comparison.

### 6.8 Condition 7 Civilian owner

0x0041EC90 resolves the literal Civilian side through 0x006A46D0, scans the global House registry forward, and chooses the first House whose CountryType+0xBC side index matches. No matching House or a null selected House returns false. Otherwise it performs the same family count and payload comparison as condition 0.

### 6.9 Zone gate

0x0041FEE0 runs primary then secondary:

- TeamType+0xF0 false bypasses the relation check;
- null target passes;
- otherwise the helper compares acting/target base-zone IDs under TeamType+0xEC combined movement row;
- TeamType+0xF1 selects either the ordinary same-zone requirement or the combined-row-different plus Amphibious-row-same transport-crossing form.

The exact post-load +0xEC/+0xF0/+0xF1 derivation and lookup behavior are verified in PHASE3_TEAMTYPE_POSTLOAD_ZONE_DERIVATION_GHIDRA_REPORT.md. Failure of primary or secondary rejects the trigger with no retry.

### 6.10 TaskForce factory availability

0x00509610 runs primary then secondary. A null TeamType or null TeamType+0xE4 TaskForce rejects. Compact TaskForce entries are visited forward.

For every member type it calls the member type vtable+0x94 with arguments 1,1,0,actingHouse. All four relevant type-family vtables point this slot to 0x005F7900.

In g_GameMode!=0, a null factory immediately rejects. Only g_GameMode==0 gets the reverse global-Building fallback requiring acting owner, exact member type, and 0x006F1E20 true. An empty TaskForce returns true.

For the exact 1,1,0 call, FindFactory 0x005F7900 scans the acting House's Building vector forward and:

1. skips Building+0x81 limbo;
2. requires BuildingType+0xEB8 Factory enum == member type WhatAmI;
3. requires Building+0x660 HasPower/online;
4. rejects current mission or queued mission 0x13 Selling;
5. skips the optional HouseClass::CanBuild gate because the third boolean is zero;
6. requires member type owner mask from vtable+0x70 to intersect factory BuildingType owner mask;
7. skips the special Aircraft-pad/path check because the first boolean is one;
8. for a naval UnitType, requires both member type+0xCCE and factory type+0xCCE Naval; otherwise requires factory type Naval=false.

It retains the last eligible fallback but returns immediately on Building+0x3D3 IsPrimaryFactory. Eligibility only consumes the non-null result.

Older navigation that labels Building+0x660 as primary factory or type vtable+0x70 as naval flags is wrong for this call. The live meanings are HasPower at +0x660, IsPrimaryFactory at +0x3D3, owner mask at virtual +0x70, and Naval at type+0xCCE.

### 6.11 TeamType Max

After zones and factories, 0x0041E720 checks primary then secondary. If TeamType+0xB8 Max is nonnegative, 0x005095D0 counts global live Teams with exact acting House and TeamType; count >= Max rejects.

This is a pre-selection snapshot. Actual creation 0x006F09C0 checks Max again immediately before each Team allocation. Thus primary and secondary may be the same TeamType:

- Max=1: primary creation succeeds, secondary creation is refused;
- Max=2 or greater: one selector result may create two identical Teams.

Retail has 11 same-primary/secondary rows. Four have Max 2 or 3 and can create two identical Teams from one selector event when below the cap.

## 7. Weight construction and output

### 7.1 Registry walk and truncation

Eligible AITriggers are scanned in global registry order. Current weight at +0xB8 is converted by Math::ftol at 0x006F0C67. The result is a signed 32-bit integer used as that row's weight.

### 7.2 Exact 5000 priority tier

When an eligible row truncates to 5000:

- the first such row clears all previously accumulated entries and total weight;
- it sets a priority-present latch;
- that row and subsequent 5000 rows are added.

Once the latch is set, later eligible rows whose truncated weight is not 5000 are ignored. The test is exact integer equality after ftol, not >=5000 and not a double comparison.

Retail has nine current-weight-5000 rows. Seven are locked by min=max=5000. Two have min=10 and max=5000, so their first failure update drops current weight to 4950 and removes them from the priority tier. This makes adaptive priority membership stock-live.

### 7.3 Weighted draw and unsigned accumulation

All admitted signed weights are added with 32-bit wrapping. A second RandomRanged(1,total) occurs only when total is nonzero and the candidate vector count is nonzero. Zero total or no entries produces no second draw and no output.

Candidate weights are cumulatively added in registry order with 32-bit wrapping. Assembly at 0x006F0D08..0x006F0D0C uses JAE, so draw <= cumulative is an unsigned comparison. With a positive total, a zero-weight row cannot win.

Random::RandomRanged 0x0065C7E0 normalizes reversed signed endpoints. Therefore a malformed negative total requests a range between total and 1, while cumulative matching remains unsigned. A normalized span at least 0x80000000 does not terminate in the helper. Retail excludes this: every current/min/max weight is positive, with minimum 10 and maximum 5000, and native adaptive clamps stay within positive stock bounds.

### 7.4 Output, cancellation, and Autocreate

The chosen trigger appends primary +0xDC, then optional secondary +0xE0. There is one total-cap precheck before selection, so a two-TeamType result can overshoot TotalAITeamCap by one.

The selector then scans every live Team owned by the acting House. If any output TeamType equals that Team's TeamType and:

Team+0x7B != 0 OR Team+0x7F == 0

the entire output vector is cleared. There is no retry, no replacement candidate, and no further RNG. The bytes should remain literal or neutrally described as regroup-incomplete/not-yet-formed state until their separate owner is proved.

If not cancelled, TeamType+0xA9 Autocreate is set to one for every output entry. All 163 retail TeamTypes already declare Autocreate=yes, so the write is stock-inert but remains active for map/custom data.

## 8. Adaptive weight feedback

### 8.1 State

Each AITrigger stores:

- current weight double +0xB8;
- minimum weight double +0xC0;
- maximum weight double +0xC8;
- success counter signed int +0x104;
- attempt counter signed int +0x108.

Rules values are:

- AITriggerSuccessWeightDelta double at Rules+0xC0, stock 20;
- AITriggerFailureWeightDelta double at Rules+0xC8, stock -50;
- AITriggerTrackRecordCoefficient double at Rules+0xD0, stock 1.

### 8.2 Success update at 0x0041FD60

Using pre-increment successes S and attempts A:

- adjustment = 0 when A <= 0;
- otherwise adjustment = (S/A - 0.5) * A;
- clamp adjustment to zero when negative;
- current = clamp(current + SuccessDelta + adjustment, minimum, maximum);
- increment successes;
- increment attempts.

### 8.3 Failure update at 0x0041FE20

Using pre-increment S and A:

- adjustment = 0 when A <= 0;
- otherwise adjustment = (S/A - 0.5) * TrackRecordCoefficient * A;
- clamp adjustment to zero when positive;
- current = clamp(current + FailureDelta + adjustment, minimum, maximum);
- increment attempts only.

### 8.4 Destruction fan-out and ordering

The sole gameplay callers of both weight helpers are in TeamClass destructor 0x006E8DE0. The destructor scans every AITrigger forward. Every trigger whose primary +0xDC equals the destroyed Team's TeamType receives feedback:

- Team+0x84 false -> failure;
- Team+0x84 true -> success.

Secondary matches alone receive no direct feedback. All triggers sharing a primary TeamType update, not merely the trigger which selected the Team.

The update loop runs before base-defense count decrement, member removal, TeamType live-count decrement, and global Team removal.

Team+0x84 initializes false. TeamClass::AI Script action 49 sets it true and advances. Current Rust already preserves this success byte and excludes it from Team CRC, but it does not fan destruction into AITrigger weights.

## 9. Save/load and CRC boundary

AITrigger primary vtable is 0x007E2A50:

- +0x14 -> load 0x0041E540;
- +0x18 -> save 0x0041E5C0;
- +0x30 -> raw size 0x0041FFE0, which returns 0x110.

Save delegates to AbstractClass::Save 0x00410320, which writes the receiver block using virtual size 0x110. Load delegates to AbstractClass::Load 0x00410380, restores class vtables, and queues +0xD8/+0xDC/+0xE0 pointer swizzles. The entire object block includes current weight and both +0x104/+0x108 counters. All three must survive save/load.

The AITrigger CRC callback raw body 0x0041E5E0..0x0041E6F7 feeds:

- inherited/base identity through 0x00410BE0;
- condition +0x98, source +0x9C, owner mode +0xA0, country +0xA8, threshold +0xB0;
- payload amount +0xE4 and comparator +0xE8 only for conditions 0, 1, 4, and 7;
- current/min/max doubles +0xB8/+0xC0/+0xC8;
- token11/+0xD0;
- Easy/Normal/Hard bytes +0xD2/+0xD3/+0xD4;
- side restriction +0xAC;
- inert serialized token14/+0xD1.

The callback does not directly feed success or attempt counters +0x104/+0x108. It also does not directly feed primary/secondary/object pointers, enabled +0xA4, or token12. Rust must snapshot both counters, hash the native CRC projection including dynamic current weight, and avoid directly hashing the counters merely because they are serialized.

## 10. Retail liveness and exclusions

### 10.1 Registry census

- AITriggerTypes: 165
- TeamTypes: 163
- TeamTypes with Autocreate=yes: 163
- TeamTypes with IsBaseDefense=yes: 12
- AITriggers with secondary TeamType: 49
- AITriggers with identical primary and secondary: 11

Output shapes:

| Shape | Count |
|---|---:|
| no secondary, primary non-defense | 112 |
| no secondary, primary defense | 4 |
| secondary, neither defense | 42 |
| secondary, primary defense and secondary non-defense | 7 |
| secondary defense | 0 |

The seven mixed rows qualify as base-defense triggers during the minimum-defense corridor and can also output a non-defense secondary.

### 10.2 Condition/object/comparator census

Object family:

- Building 87
- none 56
- Unit 11
- Infantry 7
- Aircraft 4

Comparator:

- 0: 6
- 1: 4
- 2: 5
- 3: 148
- 4: 2
- 5: 0

Condition/comparator pairs:

- condition 0: comparator 1 x2, 3 x46, 4 x2
- condition 1: comparator 0 x1, 1 x2, 2 x5, 3 x41
- condition 4: comparator 3 x53
- condition 5: comparator 0 x2, payload inert
- condition 6: comparator 0 x3, payload inert
- condition 7: comparator 3 x8

Amount distribution is 0:61, 1:88, 2:5, 3:2, 4:1, 5:5, 6:2, 8:1.

### 10.3 Weight census

Current:

- 10 x2, 20 x4, 30 x4, 40 x23, 50 x35, 60 x10, 70 x53, 500 x25, 5000 x9

Minimum:

- 10 x134, 30 x10, 40 x3, 50 x2, 60 x5, 70 x4, 5000 x7

Maximum:

- 30 x2, 40 x10, 50 x10, 60 x3, 70 x106, 500 x25, 5000 x9

All retail weight bounds are positive. Malformed negative/overflow behavior is custom-data reachable but excluded from fixed retail.

### 10.4 Evidence-backed exclusions

| Mechanism | Fixed retail verdict | Why |
|---|---|---|
| selector eviction branch | excluded from ordinary selector-only stock path | MaxDefense 2 permits at most 3, so defense < floor(total/2) remains true at cap |
| condition -1 | absent | no retail trigger uses it |
| conditions 2 and 3 | absent | no retail trigger uses them |
| comparator 5 | absent | no retail trigger uses it |
| token12 behavior | inert in binary | consumed, discarded, no retained destination |
| token14 gameplay effect | inert in binary | stored/serialized/CRC-fed, no gameplay reader |
| Autocreate side-effect visibility | inert for fixed retail | all 163 TeamTypes already yes |
| negative/huge weighted total | excluded | all fixed weights and adaptive bounds are positive and small |
| secondary direct feedback | absent by mechanism | destructor compares only trigger primary |

## 11. Current Rust disparity

Read-only inspection at HEAD 9d3be282b05ca314aeed357b3bf36764b910211d found:

### 11.1 Preserved correct foundations

- src/sim/team_script_vm.rs retains ScriptType, TaskForce, TeamType, and AITrigger registry order.
- TeamAiTriggerDefinition retains source, enable, owner, threshold, condition, category-distinct object identity, payload, three weights, tokens 11/13/14 storage, primary/secondary, and difficulty flags.
- TeamType metadata retains Max and Autocreate.
- Team action 49 sets Team success, snapshots it, and deliberately excludes it from Team CRC.
- TeamType zone derivation fields exist.
- HouseState already has side, country, IsHuman, PlayerControl, difficulty, MultiplayPassive, credits, TechLevel, and enemy House.

### 11.2 Missing or wrong for this mechanism

- no House AITrigger ratio, active latch, selector timer, or maintained/derived base-defense Team count;
- no selector or complete eligibility pipeline;
- no Rules fields for TeamDelays, minimum/maximum defense Teams, total cap, UseMinDefenseRule, success/failure deltas, track coefficient, or AIMinorSuperReadyPercent;
- no scenario ingress for RatioAITriggerTeam or IgnoreGlobalAITriggers and no TriggerAction 74/75/76 writers;
- successful MCV deploy does not write a selector-active latch;
- AITrigger weights are static registry data with no success/attempt history or destruction feedback;
- TeamScriptVm::hash_state intentionally excludes the AITrigger registry, while native CRC includes the dynamic AITrigger projection;
- current superweapon storage is keyed rather than explicitly preserving native per-House SuperClass order, yet conditions 5/6 require first matching instance semantics;
- current House fields have only aggregate object counts, not the four category-distinct per-type count tables used by conditions 0/1/7;
- no exact acting-House Building-order FindFactory(1,1,0) eligibility seam;
- create_team_from_type immediately admits candidates, while native selector output calls an empty Team constructor and recruitment starts next frame;
- Team deletion paths remove Rust state without first updating every AITrigger sharing the primary TeamType.

### 11.3 Scheduler placement

Rust already runs TeamScriptVm before the Logic object vector. The selector belongs at the late House update boundary, after object/production consequences and before late frame commit. It must not be folded into the existing generic ai::tick_ai random-wave choice. The native output creation occurs in the House rung and the new Team remains unavailable to the already-completed Team pass.

## 12. Implementation handoff

### 12.1 Data and ingress

1. Add the exact selector Rules values with Hard/Normal/Easy table order and native defaults/INI parsing.
2. Add House ratio, active latch, and three-part timer state; seed with TeamDelays[difficulty] + House order index*175 and reset without stagger.
3. Add scenario IgnoreGlobalAITriggers and RatioAITriggerTeam ingress.
4. Wire TriggerAction 74/75/76 and successful MCV deploy to the exact House fields.
5. Retain current/min/max as native doubles plus signed success/attempt counters on each ordered AITrigger.
6. Preserve native House order, per-House superweapon instance order, and authoritative owned-Building order for the first-match scans.

### 12.2 Pure selector/eligibility owner

Implement one selector owner in or beside src/sim/team_script_vm.rs which receives explicit facts rather than reaching through unrelated app state:

- acting and target House facts;
- ordered Team registry and TeamType metadata;
- ordered AITrigger registry;
- family/type owned counts;
- base-zone connectivity;
- ordered House Buildings and factory facts;
- ordered superweapon instances;
- scenario mode/provenance flags;
- the scenario RNG.

The owner must preserve every branch and order in sections 5 through 7, including signed ftol, exact 5000 equality, wrapping totals, unsigned cumulative comparison, primary-before-secondary output, cancellation without retry, and Autocreate writes.

### 12.3 Creation boundary

Do not connect selector output to the current immediate-admission create_team_from_type path. Introduce or use an exact empty TeamType constructor which:

- rechecks Max separately for each output;
- permits same-primary/secondary double creation when Max allows;
- updates per-TeamType and base-defense counts at construction;
- leaves recruitment for the later Team pass.

Stage-C recruitment remains a separate mechanism and reviewer round.

### 12.4 Feedback and destruction

Centralize Team destruction so every removal reason first:

1. scans ordered AITriggers;
2. updates every primary TeamType match with the exact success/failure formula;
3. then performs base-defense/member/type/global removal in native order.

Eviction in the selector must call this same path synchronously. Recheck that action 49 success remains serialized but absent from Team CRC.

### 12.5 Snapshot and hash

- snapshot current/min/max and both counters;
- snapshot House ratio/latch/timer and any new order authorities;
- add the exact AITrigger CRC projection in registry order;
- include dynamic current weight;
- include payload only for conditions 0,1,4,7;
- do not directly hash success/attempt counters;
- preserve the already-correct Team success non-CRC rule;
- bump the snapshot version once for the coherent slice.

### 12.6 Required focused acceptance tests

At minimum:

1. expired inactive House spends exactly the probability draw and no weighted draw;
2. ratio 0/100 and TriggerAction 74/75/76 boundaries;
3. first seed includes HouseIndex*175, repeat does not, empty output still resets;
4. ordinary House admission skips IsHuman and MultiplayPassive but not PlayerControl alone;
5. fixed/map source override and IgnoreGlobalAITriggers;
6. map-new unlisted disabled, fixed unlisted enabled, ordinary listed false still enables;
7. every difficulty/session mapping;
8. owner mode 0/1/2/other and side 1/2/3/other;
9. null target with -1 versus every retail condition;
10. conditions 0/1/4/7 with all six comparators and family-duplicate object IDs;
11. raw available-wallet condition rather than credits-only;
12. conditions 2/3 exact strict thresholds;
13. first matching inactive superweapon blocks later matching ready instance;
14. zero recharge fails and 70% stock threshold passes at equality;
15. primary/secondary zone and factory rejection order;
16. empty TaskForce passes and one unavailable member rejects;
17. powered primary factory versus unpowered/selling/naval/owner-mask mismatches;
18. Max primary then secondary, including same ID Max1 and Max2;
19. strict MaximumDefense equality allows one extra, then suppresses;
20. eviction chooses earliest smallest signed priority and feeds weight before removal;
21. first 5000 clears earlier candidates, later 5000 joins, later non-5000 is ignored;
22. zero total spends no second draw; positive total uses unsigned cumulative registry order;
23. output cancellation clears both entries with no retry or extra draw;
24. two outputs may overshoot total cap by one;
25. success/failure formulas at attempts zero, positive/negative adjustment clamp, and min/max bounds;
26. all triggers sharing primary update, secondary-only does not;
27. save/load retains counters/current weight but hash changes only for the native CRC-fed projection;
28. Team pass before House creation proves no same-frame Team execution;
29. all prior Stage-A registry identity/order tests and base-defense timer/order tests remain green.

No bare Cargo command is authorized by this report; project Cargo rules still require process ownership checks and --lib.

## 13. Coverage ledger

| Mechanism | Native evidence | Status |
|---|---|---|
| sole selector caller | caller/xref to 0x006F0AB0 | RESOLVED: HouseClass::Update only |
| fixed/map source order | 0x006879DA/0x006879E3, 0x0041F2E0 | RESOLVED |
| enable-section semantics | 0x0041F3A9 onward | RESOLVED |
| IgnoreGlobalAITriggers | 0x0041E7B0 region | RESOLVED |
| tokens 11-14 | parser, consumer, full-program field searches | RESOLVED; token12 and token14 gameplay are inert |
| ratio state and writers | +0x565C instruction census | RESOLVED |
| active latch and writers | +0x1F2 instruction census | RESOLVED |
| initial/repeat cadence | 0x004F70D0, 0x005010D0, 0x004F8A00 | RESOLVED |
| House admission | 0x004F8A27..0x004F8A5D | RESOLVED |
| count/cap prepass | 0x006F0B20 onward | RESOLVED |
| eviction choice/destruction | 0x006F0B80 corridor, Team vtable | RESOLVED |
| fixed-retail eviction reachability | retail caps plus exact inequalities | RESOLVED EXCLUSION |
| base-defense preamble | 0x0041E720 | RESOLVED |
| mode/difficulty | 0x0041E7D0..0x0041E855 | RESOLVED |
| owner/side/tech | 0x0041E855 onward | RESOLVED |
| condition -1 and null target | 0x0041E8D0 corridor | RESOLVED |
| conditions 0/1 counts | 0x0041EAF0, 0x0041EE90 | RESOLVED |
| conditions 2/3 power | 0x0041E92B..0x0041E999 | RESOLVED |
| condition 4 wallet | 0x0041F230 -> 0x004F6990 | RESOLVED |
| conditions 5/6 super readiness | 0x0041F0D0, 0x0041F180 | RESOLVED |
| condition 7 Civilian | 0x0041EC90 | RESOLVED |
| payload comparators | 0x0041EAF0 family | RESOLVED |
| type WhatAmI family mapping | four primary type vtables | RESOLVED |
| zone gate | 0x0041FEE0 plus prior verified report | RESOLVED |
| factory gate | 0x00509610, 0x005F7900 | RESOLVED |
| TeamType Max | 0x005095D0, 0x006F09C0 | RESOLVED |
| 5000 priority tier | 0x006F0C61..0x006F0CB6 | RESOLVED |
| weighted draw/order | 0x006F0CC4..0x006F0D20 | RESOLVED |
| malformed range behavior | 0x0065C7E0 | RESOLVED; excluded from fixed retail |
| output/cancellation/Autocreate | 0x006F0D26..0x006F0E64 | RESOLVED |
| adaptive success/failure | 0x0041FD60, 0x0041FE20 | RESOLVED |
| feedback owner/order | Team destructor 0x006E8DE0 | RESOLVED |
| save/load extent | vtable 0x007E2A50, raw size 0x110 | RESOLVED |
| CRC projection | 0x0041E5E0..0x0041E6F7 | RESOLVED |
| retail liveness and census | hash-pinned AIMD/rules | RESOLVED |
| Rust selector parity | read-only source inspection | RESOLVED GAP |
| Stage-C recruitment | separate owner | RESOLVED OUT OF SCOPE |
| excluded presentation/TS systems | user boundary | RESOLVED OUT OF SCOPE |

## 14. Adversarial questions

### Q1. Could Autocreate=yes alone be used to seed Teams?

No. All 163 retail TeamTypes already set Autocreate=yes, but House update selects only through eligible AITriggers, weighted registry order, cap state, and two RNG gates. Autocreate is a post-selection write, not the selection predicate.

### Q2. Could weights be kept as immutable INI values because stock 5000 rows dominate?

No. Team destruction updates every matching-primary trigger. Two stock 5000 rows have minimum 10 and fall to 4950 on failure, changing priority-tier membership. Current weight and counters survive save/load.

### Q3. Could a BTreeMap order substitute for all native order dependencies?

No. The mechanism separately depends on AITrigger registry order, Team registry order, House registry order, House-owned Building order, and per-House SuperClass order. Key sort is not evidence-equivalent to any of them.

### Q4. Could TeamType Max be checked only once during eligibility?

No. Primary and secondary can be identical. Eligibility observes both against the same pre-create count, then 0x006F09C0 rechecks each actual creation. Max=1 suppresses the second; Max>=2 can permit both.

### Q5. Could a failed chosen trigger retry another weighted candidate?

No. Same-Team cancellation clears the entire output after the weighted choice and returns. Zone, factory, and Max failures reject during eligibility, but post-choice cancellation never redraws or retries.

## 15. Cold spot checks and zero-add pass

### 15.1 Cold spot 1: conditions 2 and 3

Raw assembly 0x0041E92B..0x0041E999 independently confirms two virtual reads from target House+0x24, subtraction PowerOutput-PowerDrain, x87 signed conversion, and strict comparisons against 100.0 and 0.0. The branches use TEST AH,1 after FCOMP and do not read payload amount/comparator.

### 15.2 Cold spot 2: save/load and raw size

Raw assembly confirms:

- load 0x0041E540 calls AbstractClass::Load, restores four vtables, and swizzles +0xD8/+0xDC/+0xE0;
- save 0x0041E5C0 calls AbstractClass::Save;
- vtable +0x30 target 0x0041FFE0 returns 0x110.

This independently establishes persistence of +0x104/+0x108 counters even though the CRC callback omits them.

### 15.3 Additional cold check: type-class IDs

Raw bytes at the four type vtable +0x2C targets return 3,7,0x10,0x28. This prevented a false mapping from runtime object RTTI values from entering the handoff.

### 15.4 Zero-add pass

After the full branch map was assembled, a final pass checked:

- callers and xrefs of selector, eligibility, factory helper, and both weight helpers;
- full-program instruction references to +0xD1, +0x1F2, +0x565C, +0x566C, and timer fields;
- AITrigger vtable save/load/CRC slots;
- fixed/map loader call order;
- retail rows for every condition, comparator, weight tier, secondary shape, and same-primary/secondary case;
- current Rust owners for House, TeamScriptVm, Rules, superweapons, production factories, snapshots, hashing, and scheduler order.

No additional Stage-B mechanism, writer, consumer, or retail-active row was found. The remaining Team recruitment questions belong to Stage C and were not silently promoted into this report.

## 16. Final open-question log

| ID | Question | Final status |
|---|---|---|
| OQ-SB-01 | What owns ordinary selector cadence? | RESOLVED: HouseClass timer on TeamDelays |
| OQ-SB-02 | Does an inactive selector consume RNG? | RESOLVED: yes, probability draw first |
| OQ-SB-03 | What do tokens 11-14 do? | RESOLVED: session enable, discarded padding, side restriction, serialized inert byte |
| OQ-SB-04 | How do fixed and map rows interact? | RESOLVED: in-place override, source replacement, separate enable section |
| OQ-SB-05 | Are all condition enums understood? | RESOLVED |
| OQ-SB-06 | Is object family resolution exact? | RESOLVED: type-class RTTI 3/7/0x10/0x28 |
| OQ-SB-07 | Is the wallet check raw credits? | RESOLVED: no, storage value*IncomeMult + Balance |
| OQ-SB-08 | Which superweapon instance is tested? | RESOLVED: first matching type, even when unusable |
| OQ-SB-09 | What exact factory predicate is used? | RESOLVED: FindFactory(1,1,0) in House Building order |
| OQ-SB-10 | What is the exact priority tier? | RESOLVED: ftol(current)==5000 |
| OQ-SB-11 | Can output retry after cancellation? | RESOLVED: no |
| OQ-SB-12 | Can selection exceed caps? | RESOLVED: strict defense equality permits +1; two outputs can exceed total by one |
| OQ-SB-13 | How do weight histories update? | RESOLVED |
| OQ-SB-14 | Do histories persist and hash? | RESOLVED: persist; counters not directly CRC-fed |
| OQ-SB-15 | Is the eviction branch stock-live? | RESOLVED EXCLUSION for selector-only fixed retail; active for map/custom/other creators |
| OQ-SB-16 | Is Stage-C recruitment part of this mechanism? | RESOLVED OUT OF SCOPE: separate Team-tick owner |
| OQ-SB-17 | Are presentation or TS systems needed here? | RESOLVED OUT OF SCOPE |

## 17. Stop condition

Research may hand off to a Stage-B builder only if the builder treats every section above as required behavior and keeps the row open until focused tests, save/load/hash checks, prior-fix rechecks, and a fresh critic pass all succeed. Any shortcut through generic ai::tick_ai, Autocreate-only selection, immutable weights, unordered maps, raw credits, or immediate candidate admission is non-parity.
