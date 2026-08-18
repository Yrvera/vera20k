# AnimClass Bouncer Water Splash Branch - Ghidra Research Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, water branch `0x00423CD5..0x00423DE2`, parent destroy `0x00424294..0x00424298`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact active-YR bouncer/meteor water/splash branch after `AnimClass` embedded bounce update returns `1` or `2`, including branch predicates, RulesClass assets, constructor row order/args, meteor split, z offsets, skipped `ExpireAnim`/AI area damage, and parent cleanup order.
**Non-Scope:** accepted ground impact, `BounceAnim=` row, `Apply_area_damage` internals, full `BounceClass::Update` physics, full renderer draw composition, and `VoxelAnimClass::AI` water behavior.
**Confidence:** High
**Active in YR:** Conditional. The branch is live for `AnimClass` instances whose type has `Bouncer=yes` or `IsMeteor=true`, when the embedded bounce path returns `1` or `2`, the current cell is water, and the anim is below the ground-height gate.

Working notes:
- `Target question`: What exact active-YR water/splash behavior runs around `AnimClass::AI @ 0x00423CD5..0x00423DE2` for bouncer/meteor impacts?
- `Non-goals`: Do not expand into accepted ground impact, normal destroy, `BounceAnim=`, full `Apply_area_damage`, or `VoxelAnimClass` beyond avoiding cross-system confusion.
- `Evidence needed to mark COMPLETE`: decompile plus assembly context for branch predicates, `IsMeteor` split, RulesClass offsets, constructor push order/args, z offsets, skip of `ExpireAnim`/AI damage, parent destroy order, INI defaults, Rust surface scan, and stale-doc handoff.
- `Stop conditions`: stop when water branch row order and skipped side effects are resolved or explicitly deferred, and when the report names concrete implementation/test handoff.

## 1. Overview

The bouncer/meteor water branch is the rejected-water-impact half of `AnimClass::AI`. After the embedded bounce path returns `1` or `2`, the AI branch accepts normal `ExpireAnim=` impact handling only when the cell is not water or the anim is at/above the ground-height gate. If the cell is water and the anim is below that gate, `AnimClass::AI` skips `ExpireAnim=`, skips the accepted-impact `Apply_area_damage`/helper block, emits water splash rows from `RulesClass`, then destroys the parent anim.

For non-meteor bouncers, the water branch emits two rows: `Rules.Wake` at the parent coords, then the first `Rules.SplashList` entry at `Z+3`. For meteors, it emits one row: the last `Rules.SplashList` entry at `Z+3`.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `AnimClass.Type` | `+0x0C8` | current `AnimTypeClass*`; source for `IsMeteor` | decompile `0x00423CD5`; assembly `MOV EDX,[ESI+0xC8]` | Yes |
| `AnimClass.IsBouncerInstance` | `+0x194` | earlier AI gate for embedded bounce path | prior report; AI call gate `0x00423C24..0x00423C44` | Conditional |
| `AnimClass.Coords` | `+0x09C` | parent integer coords used for water rows | decompile uses `param_1+0x27`; assembly around `0x00423C4A` | Yes |
| `AnimClass.BounceCoords` | `+0x140` | float impact coords for accepted ground branch, not water rows | decompile accepted branch `0x00423DE7..0x00423E70` | Conditional; not used by water rows |
| `AnimType.IsMeteor` | `+0x356` | selects meteor single-splash vs non-meteor wake+first-splash path | `0x00423CE0..0x00423CE8` | Conditional |
| `Rules.Wake` | `RulesClass+0x94` | non-meteor first water row type | `0x00423D7A..0x00423D89`; `ini/rulesmd.ini:525` | Conditional |
| `Rules.SplashList.buffer` | `RulesClass+0xBC4` | dynamic vector backing pointer for splash anim types | `0x00423D2F`, `0x00423DD2` | Conditional |
| `Rules.SplashList.count` | `RulesClass+0xBD0` | used by meteor to pick last splash entry | `0x00423D29..0x00423D35` | Conditional |
| `CellClass.LandType` | `Cell+0xEC` | compared to `2` for water | `0x00423C70..0x00423C7D` | Yes |

## 3. Core Logic

### 3.1 Entry and water predicate

Active in YR: Conditional.

This branch is reached only after `AnimClass+0x194` is set and vtable `+0x1E8` returns `1` or `2`. That activation was settled in the parent bouncer report; this report starts at the post-return gate.

`AnimClass::AI` computes:

- `is_water = (CellClass+0xEC == 2)`.
- `above_or_at_ground_gate = (AnimZ >= CellGroundHeight(coords) + DAT_0089A1B4)`.

Assembly evidence:

- `0x00423C70` calls `CellClass::Get_Cell_At`; `0x00423C75` reads `[EAX+0xEC]`; `0x00423C7D` compares it to `2`; `0x00423CAA` stores `SETZ BL`.
- `0x00423CB1` calls `CellClass::GetGroundHeight`; `0x00423CB6` adds `DAT_0089A1B4`; `0x00423CBC` compares current `AnimZ`; `0x00423CBE` stores `SETGE AL`.
- `0x00423CC1..0x00423CCF` routes accepted cases to `0x00423DE7`: `!is_water` jumps to accepted impact, and `is_water && above_or_at_ground_gate` also jumps to accepted impact.
- Therefore the water/splash branch at `0x00423CD5` runs only for `is_water && !above_or_at_ground_gate`.

This is not a TS-only dead path. Retail YR has active `Bouncer=yes` debris and `IsMeteor=true` meteor anim types in `ini/artmd.ini`; whether the branch fires is conditional on those anim instances landing in water below the gate.

### 3.2 Meteor water branch

Active in YR: Conditional on `AnimType+0x356 IsMeteor != 0`.

When `IsMeteor` is true, `0x00423CE0..0x00423CE8` takes the meteor branch. The branch allocates one `AnimClass` and constructs the last entry of `Rules.SplashList`:

`AnimClass(type=Rules.SplashList.buffer[Rules.SplashList.count - 1], coords=(parent.x, parent.y, parent.z + 3), delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

Assembly evidence:

- `0x00423CDB` pushes allocation size `0x1C8`; `0x00423CEA` calls allocation; `0x00423CF2..0x00423CF4` skips to `0x00423EFD` if allocation fails.
- `0x00423CFA..0x00423D06` copies parent `x`, `y`, `z`; `0x00423D15` adds `3` to the local z value.
- `0x00423D10`, `0x00423D18`, and `0x00423D1E` push `drawFlags=0x600`, `loop=1`, and `delay=0`; `0x00423D02`/`0x00423D04` push `reverse=0` and `zAdjust=0`.
- `0x00423D29` reads `RulesClass+0xBD0` count; `0x00423D2F` reads `RulesClass+0xBC4` buffer; `0x00423D35` reads `[buffer + count*4 - 4]`, the last entry.
- `0x00423D3C` calls `AnimClass::Constructor`; `0x00423D41` jumps to `0x00423EFD`.

With retail `rulesmd.ini` `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`, the meteor branch picks `H2O_EXP1`, the last listed entry. `artmd.ini` labels that section as the large water explosion.

### 3.3 Non-meteor water branch

Active in YR: Conditional on `AnimType+0x356 IsMeteor == 0`.

When `IsMeteor` is false, the branch emits up to two rows. The first allocation constructs `Rules.Wake` at unchanged parent coords:

`AnimClass(type=Rules.Wake, coords=(parent.x, parent.y, parent.z), delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

Assembly evidence:

- `0x00423D46` allocates; allocation failure at `0x00423D50` skips only this first row and continues to the second allocation at `0x00423D8E`.
- `0x00423D52..0x00423D76` copies parent x/y/z with no z add.
- `0x00423D54`, `0x00423D56`, `0x00423D58`, `0x00423D5F`, and `0x00423D65` push `reverse=0`, `zAdjust=0`, `drawFlags=0x600`, `loop=1`, and `delay=0`.
- `0x00423D7A..0x00423D86` reads `RulesClass+0x94`; `0x00423D89` calls `AnimClass::Constructor`.

The second allocation constructs the first `Rules.SplashList` entry at `Z+3`:

`AnimClass(type=*Rules.SplashList.buffer, coords=(parent.x, parent.y, parent.z + 3), delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

Assembly evidence:

- `0x00423D8E..0x00423D9D` allocates; allocation failure skips to `0x00423EFD`.
- `0x00423DA3..0x00423DBE` copies parent x/y/z and adds `3` to z.
- `0x00423DB1`, `0x00423DB3`, `0x00423DB9`, `0x00423DC1`, and `0x00423DC7` push `reverse=0`, `zAdjust=0`, `drawFlags=0x600`, `loop=1`, and `delay=0`.
- `0x00423DD2` reads `RulesClass+0xBC4`; `0x00423DD8` reads `[buffer]`, the first entry; `0x00423DDD` calls `AnimClass::Constructor`.

With retail `rulesmd.ini` `Wake=WAKE1` and `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`, this means non-meteor water impacts emit `WAKE1`, then `H2O_EXP3`. `artmd.ini` labels `H2O_EXP3` as the small water explosion.

### 3.4 Skipped accepted-impact side effects

Active in YR: Conditional.

The water branch does not construct `ExpireAnim=` and does not run the accepted-impact `Apply_area_damage` / `FUN_0048A620` block owned by `0x00423DE7..0x00423EF8`. Both water sub-branches jump to `0x00423EFD` after their water rows, bypassing the `AnimType+0x304` read at `0x00423DED`, the `ExpireAnim` constructor at `0x00423E70`, `Apply_area_damage @ 0x00423EAB`, and helper call `0x00423EF8`.

Important boundary: if the embedded bounce call returned `1`, `ProcessBounceResult` may already have performed its return-1 `BounceAnim=` and direct same-cell damage before `AnimClass::AI` reaches this water branch. That settled return-1 behavior is outside this report. This report only proves that the AI accepted-impact `ExpireAnim`/area-damage block is skipped by the water/splash path.

### 3.5 Parent cleanup order

Active in YR: Conditional.

Water branch cleanup order is:

1. embedded bounce path returns `1` or `2` to `AnimClass::AI`;
2. AI proves `is_water && !above_or_at_ground_gate`;
3. AI emits meteor or non-meteor water row(s), subject to allocation success;
4. AI jumps to `0x00423EFD`;
5. because this branch still has `is_water == true` and `above_or_at_ground_gate == false`, `0x00423EFD..0x00423F07` jumps over the accepted-ground post-impact branch to `0x00424294`;
6. `0x00424294..0x00424298` calls vtable `+0xF8`, destroying the parent.

Assembly evidence:

- Meteor and non-meteor water rows both jump to `0x00423EFD` after row work (`0x00423D41`, `0x00423DE2`).
- `0x00423EFD` tests the cached water flag; `0x00423F01..0x00423F07` tests the cached ground-gate flag and jumps to `0x00424294` for water-below-gate.
- `0x00424294..0x00424298` calls parent vtable `+0xF8`.

Allocation failure does not prevent parent cleanup. Meteor allocation failure jumps directly to `0x00423EFD`. Non-meteor first allocation failure skips only the `Wake` row and still attempts the splash row; second allocation failure jumps to `0x00423EFD`.

## 4. INI Keys

| Key | File/source | Binary field/use | Retail YR value | Effect in this slice | Active in YR |
|---|---|---:|---|---|---|
| `Wake=` | `[General]`, `ini/rulesmd.ini:525` | `RulesClass+0x94` read at `0x00423D7A..0x00423D86` | `WAKE1` | non-meteor water row 1 | Conditional |
| `SplashList=` | `[CombatDamage]`, `ini/rulesmd.ini:902` | buffer `RulesClass+0xBC4`, count `+0xBD0` | `H2O_EXP3,H2O_EXP2,H2O_EXP1` | non-meteor uses first; meteor uses last | Conditional |
| `IsMeteor=` | `ini/artmd.ini`, parsed into `AnimType+0x356` | branch read at `0x00423CE0` | true for `METLARGE`, `METSMALL`; false/default otherwise | selects meteor single-row branch | Conditional |
| `Bouncer=` | `ini/artmd.ini`, parsed into `AnimType+0x35A` | constructor sets instance `+0x194` in prior report | true for many `DBRIS*` and `METDEBRI` sections | enables embedded bounce path | Conditional |
| `ExpireAnim=` | `ini/artmd.ini`, `AnimType+0x304` | not read by water branch | present on stock bouncers/meteors | skipped for water-below-gate | Conditional |

Asset role from retail YR:

- `WAKE1`: `Rules.Wake`; emitted only by non-meteor water branch.
- `H2O_EXP3`: first `SplashList` entry; emitted by non-meteor water branch.
- `H2O_EXP1`: last `SplashList` entry; emitted by meteor water branch.
- `H2O_EXP2`: middle `SplashList` entry; not emitted by this branch for the default three-entry list.

## 5. Integration Points

This branch runs inside the active `AnimClass::AI` object tick. The call chain is:

`AnimClass::Constructor` sets per-instance bouncer byte from `Bouncer=yes` or `IsMeteor=true` -> object logic tick calls `AnimClass::AI` -> `AnimClass::AI @ 0x00423C24..0x00423C44` calls vtable `+0x1E8` -> return `1` or `2` enters the impact gate -> water-below-ground branch emits water rows -> `AnimClass::AI` destroys the parent via vtable `+0xF8`.

`VoxelAnimClass::AI` has a related but separate water splash branch. Its older report mentions different z offsets for voxel anim water landings. Those values must not be copied into this `AnimClass` branch; the verified `AnimClass` water rows here use `Z+3` for splash rows and unchanged z for `Wake`.

## 6. Current Rust Implementation Status

Current Rust does not implement this branch.

- `src/rules/art_data.rs` parses `bounce_anim`, `expire_anim`, `trailer_anim`, and `trailer_seperation` in `AnimTypeRuntimeConfig`, but it does not expose `Bouncer`, `IsMeteor`, `Damage`, `DamageRadius`, `Warhead`, or water-impact-specific branch metadata.
- `src/rules/ruleset.rs` parses `[General] Wake=`, but no Rust parse/use of `SplashList` was found by `rg "SplashList|splash_list" src`.
- `src/sim/components.rs` has `AnimClassSpawnDescriptor` capable of carrying constructor row fields, but there is no generic bouncer/meteor `AnimClass` runtime that calls embedded bounce physics or emits this water row sequence.

Rust-facing delta: missing bouncer/meteor runtime, missing SplashList rules metadata, missing water-below-ground impact gate, and missing row-order preservation for `WAKE1`/first splash vs last splash.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass::AI` post-bounce impact gate | verified | decompile `0x00423AC0`; assembly `0x00423C70..0x00423CCF` | none for water predicate |
| Cell water predicate `Cell+0xEC == 2` | verified | `0x00423C75..0x00423CAA` | broader `LandType` enum proof remains in terrain docs |
| Ground-height predicate | verified | `0x00423CB1..0x00423CBE` | exact global meaning of `DAT_0089A1B4` deferred |
| Meteor water branch | verified | decompile plus assembly `0x00423CD5..0x00423D41` | none for row args/order |
| Non-meteor water branch | verified | decompile plus assembly `0x00423D46..0x00423DE2` | none for row args/order |
| Skip of `ExpireAnim`/accepted AI damage | verified | branch jumps to `0x00423EFD`; accepted block `0x00423DE7..0x00423EF8` bypassed | prior return-1 direct damage remains parent report scope |
| Parent cleanup order | verified | `0x00423EFD..0x00423F07`; `0x00424294..0x00424298` | none for this slice |
| `Rules.Wake` and `Rules.SplashList` defaults | verified from INI plus binary offsets | `ini/rulesmd.ini:525`, `ini/rulesmd.ini:902`; assembly offsets `+0x94`, `+0xBC4`, `+0xBD0` | parser storage offsets beyond these reads not rederived |
| Visual draw composition of spawned water rows | deferred | constructor rows verified only | out-of-scope renderer/blitter investigation |
| `Apply_area_damage` internals | deferred | water branch bypass verified | slot 3 owns call-row/internals |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What is the target branch? -> `AnimClass::AI @ 0x00423CD5..0x00423DE2`, reached after embedded bounce update returns `1` or `2`.` (evidence: `0x00423C24..0x00423CD5`)
- `[RESOLVED] OQ-02 - What is out of scope? -> accepted ground impact, `BounceAnim`, `Apply_area_damage` internals, full `BounceClass` physics, and `VoxelAnimClass` water behavior are not claimed here.` (evidence: user slot scope; prior reports)
- `[RESOLVED] OQ-03 - Is the branch active in YR? -> Conditional; retail YR has `Bouncer=yes` and `IsMeteor=true` anim types and the branch is inside active `AnimClass::AI`.` (evidence: `ini/artmd.ini` bouncer/meteor sections; `0x00423AC0`)
- `[RESOLVED] OQ-04 - What exact predicate selects water/splash over accepted impact? -> `Cell+0xEC == 2` and `AnimZ < CellGroundHeight + DAT_0089A1B4`.` (evidence: `0x00423C70..0x00423CCF`)
- `[RESOLVED] OQ-05 - What selects meteor vs non-meteor branch? -> `AnimType+0x356 IsMeteor`; nonzero takes meteor path, zero takes non-meteor path.` (evidence: `0x00423CE0..0x00423CE8`)
- `[RESOLVED] OQ-06 - What does meteor water branch spawn? -> last `SplashList` entry at parent coords with `z+3`, args `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0`.` (evidence: `0x00423CFA..0x00423D3C`)
- `[RESOLVED] OQ-07 - What does non-meteor water branch spawn first? -> `Rules.Wake` at unchanged parent coords, args `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0`.` (evidence: `0x00423D46..0x00423D89`)
- `[RESOLVED] OQ-08 - What does non-meteor water branch spawn second? -> first `SplashList` entry at parent coords with `z+3`, same constructor args.` (evidence: `0x00423D8E..0x00423DDD`)
- `[RESOLVED] OQ-09 - Does water branch spawn `ExpireAnim=`? -> No; both water sub-branches jump past the `AnimType+0x304` read and constructor.` (evidence: `0x00423D41`, `0x00423DE2`, `0x00423DE7..0x00423E70`)
- `[RESOLVED] OQ-10 - Does water branch call accepted-impact `Apply_area_damage`? -> No; it bypasses `0x00423E75..0x00423EAB`.` (evidence: `0x00423EFD` jump target after water rows)
- `[RESOLVED] OQ-11 - Does allocation failure affect parent cleanup? -> No; allocation failure skips the missing row and still reaches final parent destroy.` (evidence: `0x00423CF4`, `0x00423D50`, `0x00423D9D`, `0x00424294..0x00424298`)
- `[RESOLVED] OQ-12 - Does first non-meteor allocation failure skip the second splash? -> No; it jumps to `0x00423D8E` and still attempts the splash row.` (evidence: `0x00423D50`)
- `[RESOLVED] OQ-13 - Which retail splash list entries are used? -> With `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`, non-meteor uses `H2O_EXP3`, meteor uses `H2O_EXP1`.` (evidence: `ini/rulesmd.ini:902`; `0x00423D35`; `0x00423DD8`)
- `[RESOLVED] OQ-14 - What current Rust surface exists? -> parser has child anim refs and Wake, but no SplashList parse or bouncer water runtime.` (evidence: `src/rules/art_data.rs`; `src/rules/ruleset.rs`; `rg "SplashList|splash_list" src`)
- `[DEFERRED] OQ-15 - Exact renderer/blitter output for `WAKE1`/`H2O_EXP*` rows.` (category: out-of-scope; reason: this slice verifies constructor rows, not draw composition; next-step-if-pursued: run an AnimClass draw report for water splash rows)
- `[DEFERRED] OQ-16 - Exact value/semantic name of `DAT_0089A1B4`.` (category: requires-different-system-context; reason: branch formula is verified, but global origin was not needed for row-order proof; next-step-if-pursued: trace ground-height tolerance global readers/writers)
- `[DEFERRED] OQ-17 - Full `RulesClass` ReadINI vector layout for `SplashList`.` (category: bounded-cost-too-high; reason: branch read offsets and retail INI default are enough for this slice; next-step-if-pursued: decompile RulesClass parser for SplashList dynamic-vector storage)

## 9. Visual/UI Composition Ledger

This report verifies visual object creation rows, not final draw composition.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `AnimClass::AI @ 0x00423D46..0x00423D89` | water below gate, `IsMeteor=false`, allocation success | `Rules.Wake` (`WAKE1`) | parent coords, unchanged z | handled later by `AnimClass::DrawIt` | Conditional | non-meteor water wake |
| 2 | `AnimClass::AI @ 0x00423D8E..0x00423DDD` | water below gate, `IsMeteor=false`, allocation success | `Rules.SplashList[0]` (`H2O_EXP3`) | parent coords, z+3 | handled later by `AnimClass::DrawIt` | Conditional | non-meteor water splash |
| 1 | `AnimClass::AI @ 0x00423CD5..0x00423D3C` | water below gate, `IsMeteor=true`, allocation success | `Rules.SplashList[count-1]` (`H2O_EXP1`) | parent coords, z+3 | handled later by `AnimClass::DrawIt` | Conditional | meteor water splash |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `WAKE1` | yes via `Wake=` | conditional | conditional | no | no | yes | no | no | `ini/rulesmd.ini:525`; `0x00423D7A..0x00423D89` |
| `H2O_EXP3` | yes via `SplashList[0]` | conditional | conditional | no | no | yes | no | no | `ini/rulesmd.ini:902`; `0x00423DD2..0x00423DDD` |
| `H2O_EXP1` | yes via `SplashList[count-1]` | conditional | conditional | no | no | yes | no | no | `ini/rulesmd.ini:902`; `0x00423D29..0x00423D3C` |
| `H2O_EXP2` | yes via `SplashList[1]` | no in this default branch | no for this branch | no | no | no | no | yes for this branch | no middle-index read in `0x00423CD5..0x00423DE2` |
| `ExpireAnim=` target | yes on many bouncer types | no in water-below-gate path | no for this path | no | no | no | no | yes for this path | water rows jump past `0x00423DE7..0x00423E70` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Water-below-ground impact skips accepted `ExpireAnim=` and accepted AI area damage/helper block | `0x00423CC1..0x00423CCF`, water rows jump to `0x00423EFD`, accepted block `0x00423DE7..0x00423EF8` bypassed | missing bouncer runtime | future generic `AnimClass` runtime; terrain/water query; effect queue | gate accepted impact before reading/spawning `ExpireAnim` or applying accepted-impact area damage | force the same bouncer to impact accepted ground and water-below-gate; water emits no `ExpireAnim` and no accepted-impact area damage | Do not treat water impact as ordinary `ExpireAnim` impact |
| Non-meteor water branch emits `Rules.Wake` then first `Rules.SplashList` entry | `0x00423D46..0x00423D89`; `0x00423D8E..0x00423DDD`; `ini/rulesmd.ini:525,902` | no SplashList parse/use; no branch | `src/rules/ruleset.rs`; future `AnimClass` runtime and spawn queue | preserve row order: `WAKE1` at z unchanged, then `H2O_EXP3` at z+3 for retail defaults | `anim_bouncer_water_nonmeteor_spawns_wake_then_first_splash` | Do not use only one generic splash for all water impacts |
| Meteor water branch emits last `Rules.SplashList` entry only | `0x00423CE0..0x00423D3C`; `ini/rulesmd.ini:902` | no `IsMeteor` runtime branch | `src/rules/art_data.rs`; future `AnimClass` runtime; rules splash vector metadata | with retail defaults, spawn `H2O_EXP1` at z+3 and no `Wake` row | `anim_bouncer_water_meteor_spawns_last_splash_only` | Do not copy non-meteor `Wake` behavior onto meteors |
| All water rows use constructor args `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0` | push sequences `0x00423D02..0x00423D3C`, `0x00423D54..0x00423D89`, `0x00423DB1..0x00423DDD` | descriptor can carry args, but no emitter exists | `AnimClassSpawnDescriptor`; future bouncer emitter | emit native constructor descriptors, not `WorldEffect` defaults | assert all water branch spawn descriptors preserve exact row fields | Do not collapse to type defaults or renderer-only effects |
| Allocation failures skip only their row and still destroy parent | `0x00423CF4`, `0x00423D50`, `0x00423D9D`, `0x00424294..0x00424298` | Rust allocation infallibility currently unchecked | future runtime test harness if effect allocation can be disabled | side effects must not depend on successful visual row allocation except for that row existing | simulate failed wake allocation but successful splash allocation; parent still destroys after splash attempt | Do not let failed visual allocation strand the parent bouncer |

Proposed Rust test names:

- `anim_bouncer_water_nonmeteor_spawns_wake_then_first_splash`
- `anim_bouncer_water_meteor_spawns_last_splash_only`
- `anim_bouncer_water_skips_expireanim_and_area_damage`
- `anim_bouncer_water_rows_preserve_constructor_args`
- `anim_bouncer_water_allocation_failure_still_destroys_parent`

### Negative Facts / Do Not Do

- Do not spawn `ExpireAnim=` for water-below-ground bouncer/meteor impacts.
- Do not call the accepted-impact `Apply_area_damage`/`FUN_0048A620` block from the water branch.
- Do not use the middle `SplashList` entry for the default three-entry list in this branch.
- Do not emit `Wake` for meteor water impacts.
- Do not copy `VoxelAnimClass::AI` water z offsets into this `AnimClass::AI` branch; verified `AnimClass` splash rows use `Z+3`.
- Do not use `drawFlags=0x2600` or `zAdjust=-30` for water rows; those belong to accepted `ExpireAnim=`, not water splashes.

### Stale Docs / Follow-up Docs

- `docs/research/ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md` can replace its water uncertainty with:
  `Water-below-ground impacts (`Cell+0xEC == 2` and `AnimZ < GroundHeight + DAT_0089A1B4`) skip the accepted `ExpireAnim=`/AI area-damage block. Non-meteor bouncers emit `Rules.Wake` at unchanged parent coords, then the first `Rules.SplashList` entry at `Z+3`, both with `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0`. Meteors emit only the last `Rules.SplashList` entry at `Z+3` with the same args, then the parent is destroyed.`
- Any doc that treats water rejected impact as equivalent to accepted `ExpireAnim=` impact should be replaced with:
  `The water-below-ground branch is separate from accepted impact: it uses Rules wake/splash assets and bypasses `AnimType+0x304 ExpireAnim`, `Apply_area_damage`, and the helper side-effect block.`
- Any synthesis that imports `VoxelAnimClass` water offsets into `AnimClass` should mark that as system-specific drift: `AnimClass::AI` uses `Z+3` for its water splash rows.

## Sources

- Ghidra read-only decompile: `AnimClass::AI @ 0x00423AC0`; `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra assembly contexts: `0x00423C70..0x00423CCF`, `0x00423CD5..0x00423D41`, `0x00423D46..0x00423D89`, `0x00423D8E..0x00423DE2`, `0x00423EFD..0x00423F07`, `0x00424294..0x00424298`.
- Prior reports checked: `docs/research/ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md`, `docs/research/ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`, `docs/research/VOXELANIMCLASS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan: `src/rules/art_data.rs`, `src/rules/ruleset.rs`, `src/sim/components.rs`.
