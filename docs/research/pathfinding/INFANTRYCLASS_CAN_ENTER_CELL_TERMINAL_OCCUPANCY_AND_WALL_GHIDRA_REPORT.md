# InfantryClass::Can_Enter_Cell Terminal Occupancy and Wall Branch -- Ghidra Research Report

**Address(es):** `0x0051BF90` (`InfantryClass::Can_Enter_Cell`, body `0x0051BF90..0x0051C882`), terminal slice `0x0051C78B..0x0051C880`, overlay/wall slice `0x0051C17C..0x0051C225`, `0x00772AC0` (weapon warhead `Wall` predicate), `0x004F9A10` (`HouseClass::Is_Ally_ByIndex`), `0x006F3970` (`TechnoClass::GetWeaponRange`), `0x0075AB30` (WalkLocomotion `Is_Moving`)

**Investigation Mode:** exhaustive-slice

**Claimed Scope:** close the two deferred blockers in `INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`: (1) the exact terminal occupancy/subcell zero-versus-nonzero ladder and (2) Infantry wall/overlay return-code production, especially code `4`. The fixed failed-A* retry tuple from `PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md` is used where it changes layer selection.

**Non-Scope:** re-documenting the complete object/building classifier before the terminal slice, UnitClass parity, implementing Rust, runtime trigger frequency, or changing the earlier reports.

**Confidence:** High. Both target slices were checked in decompile and exact disassembly, relevant callees were decompiled, the Infantry vtable identity was re-proven from RTTI, and a second cold decompile of `0x0051BF90` was byte-for-byte identical to the first tool result.

**Active in YR:** Yes. Infantry pathfinding calls this virtual slot. Stock YR defines walls (`GAWALL`, `NAWALL`, `GASAND`) and wall-capable warheads, so code `4` is reachable in ordinary data rather than dormant TS-only code.

## 0. Investigation Contract

The target was deliberately narrow: prove the exact final result ladder after the object scan and prove whether Infantry can produce code `4` from overlays. The recent parent report explicitly left those as OQ-8 and OQ-9, so no settled bridge, tube, or building work was repeated.

Hypotheses tested:

1. A free functional subcell is enough for code `0`. **Refuted.** Enemy ownership can still produce code `5` or `7`, and the separately sampled bit 5 can produce code `2`.
2. Full functional subcells always hard-block. **Refuted.** Allied full occupancy maps to code `6` only at exactly three stationary allied Infantry occupants and otherwise code `2`; an earlier nonzero result can also survive unchanged.
3. Infantry never produces code `4`. **Refuted.** An allied wall that the mover can fire on with a `Wall=yes` warhead produces code `4`.
4. The failed-retry fixed tuple can select bridge occupancy bits. **Refuted for the ordinary retry tuple.** Its path height is the candidate cell's signed `Level`, while bridge occupancy resampling requires `Level + 4`.

## 1. Identity and Active Call Path

The class binding was re-proven rather than accepted from labels:

- `read_memory(0x007EB054, 8)` exposed the Complete Object Locator pointer immediately before the vtable.
- The COL at `0x008033B8` points to TypeDescriptor `0x00825508`; `inspect_memory_content(0x00825510)` returned `.?AVInfantryClass@@`.
- `read_memory(0x007EB204, 8)` returned `0x0051BF90` at vtable `+0x1AC` and `0x004D9C60` at `+0x1B0`.
- `get_function_by_address(0x0051BF90)` returned body `0x0051BF90..0x0051C882`.

The fixed failed-A* retry call is already verified at `0x005840C0`: it invokes mover vtable `+0x1AC` synchronously with candidate cell, direction `0..7`, candidate signed `Level`, null parent, and final argument `1`. The result consumer tests only zero versus nonzero. See `PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md`.

## 2. State Captured for the Terminal Slice

Exact disassembly of `0x0051BF90` establishes these values:

| Value | Source | Meaning in this slice |
|---|---|---|
| low occupancy byte | ground `CellClass+0x124`; bridge `+0x128` after bridge resnapshot | bits `2..4` are the three functional Infantry subcells; terminal mask is exactly `0x1C` |
| sampled bit 5 | `(occupancy_dword >> 5) & 1` | separately retained non-Infantry unit occupancy predicate |
| occupancy owner | ground `CellClass+0x54`; bridge `+0x58` after bridge resnapshot | house index used by the final ownership branch; `-1` means no owner |
| object-list layer byte | ground `0`, bridge `1` | selects `CellClass+0xE4` or `+0xE8`; it also gates the pre-terminal speed-table rejection |
| accumulated result | `EBX`, initialized to `0` | earlier overlay/object/building classifications survive unless a later branch explicitly replaces them |
| stationary allied Infantry count | local initialized to `0`, held in `EDI` after the object loop | increments for allied Infantry whose locomotor `Is_Moving` result is false |

Ground capture is at `0x0051BFD2..0x0051BFEE`. Bridge resampling is at `0x0051C0FB..0x0051C136` and requires all of: non-null parent/path context, the structural bridge flag, and `path_height == signed(cell.Level) + 4`.

For the failed-retry fixed tuple, `path_height == signed(candidate.Level)`, so the low occupancy byte, sampled bit 5, and owner remain the **ground** values. A rare null-parent bridgehead/difference-four path can still select the bridge object list; in that case the object scan is bridge-layer while these terminal occupancy/owner values remain ground-layer. Rust's existing split `CanEnterLayerContext` is therefore conceptually necessary.

One earlier object-loop branch can clear only the retained bit-5 predicate: when the mover's slave-manager condition and `SlaveManagerClass::IsSlaveAtCell` succeed at `0x0051C2C5`, the sampled bit is cleared while the low occupancy byte is unchanged.

## 3. Exact Overlay and Wall Branch

The branch at `0x0051C17C..0x0051C225` is:

```text
if cell.overlay_index != -1:
    overlay_type = OverlayTypes[cell.overlay_index]

    if overlay_type.Crate and !mover.owner.IsPlayerControl():
        return 7

    if overlay_type.Wall
       and ((cell.overlay_state >> 4) != overlay_type.DamageLevels):
        if !mover.CanFireOrActOnCell():
            return 7

        weapon = mover.GetWeapon(0).weapon_type
        if weapon is null or !weapon.warhead.Wall:
            return 7

        if mover.owner.Is_Ally_ByIndex(cell.wall_owner):
            result = 4
        else:
            result = 5
```

Load-bearing details:

- No overlay (`CellClass+0x44 == -1`) skips the entire branch.
- `OverlayType+0x2AA` is `Crate`; `OverlayType+0x2A8` is `Wall`; `OverlayType+0x2A0` is `DamageLevels`. `OverlayTypeClass::ReadINI @ 0x005FE770` reads all three.
- Equality between the upper nibble of `CellClass+0x11E` and `DamageLevels` skips dynamic wall classification and preserves the current result. The comparison is intentionally `!=`, not `<` or `>=`.
- The mover is checked through virtual slot `+0x2AC`, then primary weapon slot `GetWeapon(0)` at virtual `+0x3F8`.
- `0x00772AC0` returns true exactly when `WeaponType+0xAC` has a non-null warhead and `WarheadType+0x144 != 0`. `WarheadTypeClass::ReadINI @ 0x0075D3A0` maps that field to the `Wall` key.
- `HouseClass::Is_Ally_ByIndex @ 0x004F9A10` returns true for the house's own index, false for `-1`, and otherwise tests the corresponding bit in the alliance mask.
- Exact code production is `NEG AL; SBB EAX,EAX; ADD EAX,5`: ally/own wall yields **code `4`**; nonally or owner `-1` yields **code `5`**.
- Code `4` or `5` is stored in the accumulator and then survives the terminal block because it is nonzero.

This corrects the earlier parent report's statement that Infantry code `4` was unverified.

## 4. Exact Terminal Result Ladder

Before the occupancy ladder, `0x0051C78B..0x0051C7D0` applies one speed-table hard block:

```text
if mover.TechnoClass+0x418 == 0
   and object_list_layer == ground
   and SpeedType/LandType cost == 0.0:
    return 7
```

The object-list layer byte matters independently of the occupancy-bit layer. The rare bridge-object/ground-occupancy split therefore bypasses this ground speed check.

The exact remainder at `0x0051C7DF..0x0051C880` is:

```text
full = (occupancy_low_byte & 0x1C) == 0x1C

if result == 0 and sampled_bit5 != 0:
    return 2

if occupancy_owner != -1:
    if mover.owner.Is_Ally_ByIndex(occupancy_owner):
        if full and result < 2:
            return 6 if stationary_allied_infantry_count == 3 else 2
    else:
        if mover.GetWeaponRange(-1) <= 0:
            return 7
        if result < 5:
            return 5

if result != 0:
    return result
if full:
    return 7
return 0
```

The equality test for code `6` is exact, not `>= 3`. Assembly at `0x0051C826..0x0051C830` subtracts three and converts equality into `6`; every non-equal count, including counts greater than three, becomes `2`.

### 4.1 Stationary allied Infantry count

The object-family jump table at `0x0051C884` routes abstract type `0x0F` to the Infantry branch at `0x0051C6E7`. In normal play, only allied Infantry reach the count branch. For each one, the function calls the occupant locomotor's vtable `+0x10` and increments the count when it returns false.

Fresh vtable inspection placed WalkLocomotion's `+0x10` slot at `0x0075AB30`; its decompile returns the byte at locomotor `+0x30`. Thus, for ordinary walking Infantry, the count is the number of allied Infantry occupants whose `Is_Moving` byte is zero. Map-editor special handling is outside standard gameplay and does not change the stock-play statement.

### 4.2 Adversarial corner cases

| Input state | Exact result | Why |
|---|---:|---|
| full `0x1C`, allied owner, exactly 3 stationary allied Infantry, prior result `0` or `1` | `6` | equality-to-three branch |
| full `0x1C`, allied owner, 2 stationary plus 1 moving Infantry, prior result `0` | `2` | stationary count is 2, not occupant count 3 |
| free functional subcell, enemy owner, no effective weapon range | `7` | hostile owner branch runs even though `full` is false |
| free functional subcell, enemy owner, positive weapon range | `5` | hostile owner upgrades a result below 5 |
| full `0x1C`, owner `-1`, prior result `0` | `7` | final anonymous-full hard block |
| allied wall produced code `4`, then full `0x1C` | `4` | allied full override applies only when prior result is below 2; final nonzero wins |
| sampled bit 5 set, prior result `0`, any owner | `2` | bit-5 return precedes ownership and full-mask logic |
| wall overlay state nibble equals `DamageLevels` | prior result unchanged | wall classifier is skipped on equality |
| bridge object list selected but retry occupancy remains ground | result uses bridge occupants plus ground mask/owner | the two layer decisions are separate |

## 5. Tiny-Detail Ledger

1. Functional subcell fullness is exactly `(low_byte & 0x1C) == 0x1C`; bits 0 and 1 are ignored by the terminal fullness test.
2. Bit 5 is sampled separately from the same occupancy dword before the object scan.
3. The slave-at-cell branch can clear the retained bit-5 value without clearing the packed occupancy byte.
4. Bit-5 code `2` fires only when the accumulated result is still zero.
5. Occupancy owner `-1` skips both ally and hostile-owner branches.
6. Enemy ownership is evaluated even when at least one functional subcell is free.
7. Hostile ownership calls `GetWeaponRange(mover, -1)` and hard-blocks at `<= 0`.
8. A hostile owner upgrades only results below `5`; larger prior codes remain intact.
9. Allied-full handling runs only when the prior result is below `2`.
10. Allied-full code `6` requires stationary count exactly equal to three.
11. A no-owner full mask with result zero becomes code `7` only at the final fallback.
12. Any nonzero result surviving to the final fallback wins over the full-mask check.
13. Wall classification requires overlay state upper nibble **not equal** to `DamageLevels`.
14. The wall branch always selects weapon index zero.
15. Missing weapon, missing warhead, or `Warhead.Wall == false` each hard-blocks with code `7`.
16. Wall owner `-1` is nonallied and therefore yields code `5`, not `4`.
17. The fixed failed-retry tuple keeps terminal occupancy byte, bit 5, and owner on the ground layer.
18. Object-list selection and terminal occupancy selection can diverge in the null-parent bridgehead edge case.
19. The pre-terminal zero-speed rejection is disabled when the selected object-list layer is bridge.
20. The target slices consume no RNG and perform no world-state mutation; they synchronously derive a result from the captured cell, mover, occupant, house, weapon, and locomotor state.

## 6. INI and Stock-YR Activation

Fresh parser and stock-data checks prove the wall branch is live:

| Data | Evidence | Effect |
|---|---|---|
| `OverlayType.Wall` | `OverlayTypeClass::ReadINI @ 0x005FE770`; `rulesmd.ini:12031`, `12827`, `16388` | stock `GAWALL`, `NAWALL`, and `GASAND` enter the wall branch |
| `OverlayType.DamageLevels` | same parser; `artmd.ini:4055`, `4126`, `4140` | stock sandbag/wall overlays provide the exact nibble comparison value |
| `WarheadType.Wall` | `WarheadTypeClass::ReadINI @ 0x0075D3A0`; e.g. `rulesmd.ini:27335` | stock wall-capable weapons can pass `0x00772AC0` |
| wall ownership/alliance | `CellClass+0x50`, `HouseClass::Is_Ally_ByIndex @ 0x004F9A10` | chooses code `4` versus `5` |

The branch is conditional on the particular Infantry's primary weapon and wall state, but it is part of standard YR semantics.

## 7. Current Rust Status

Read-only inspection found these concrete disparities; no Rust files were changed.

| Rust surface | Current behavior | Exact gap |
|---|---|---|
| `src/sim/pathfinding/cell_entry.rs:415..424` | returns `Clear` for Infantry when a subcell is available and the selected list has no non-Infantry blocker | bypasses sampled bit 5 and enemy owner/range results; a free subcell can still be native code `2`, `5`, or `7` |
| `cell_entry.rs:449..577` | explicitly approximate crush/primary-blocker/friendship classifier | does not preserve the exact accumulated-result ordering or stationary-allied count ladder |
| `src/sim/pathfinding/core.rs:1859` | collapses `overlay_blocks` to static unwalkable terrain | cannot produce dynamic Infantry wall codes `4`, `5`, or `7` from alliance, weapon, warhead, and overlay damage state |
| `CanEnterLayerContext` | already separates object-list and occupancy-bit layers | suitable shape for the verified rare split; it must also carry the matching owner source |
| occupancy/world snapshot | ordered occupants and subcells exist, but the retry classifier lacks the exact packed-bit/owner/weapon/locomotor inputs | production retry activation remains blocked until these inputs and exact ordering reach the Infantry oracle |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | Remaining work in claimed scope |
|---|---|---|---|
| Infantry vtable identity and `+0x1AC` binding | verified | fresh COL/TypeDescriptor/vtable memory reads | none |
| fixed retry call and zero/nonzero consumer | verified by parent adapter report | `0x0058424D..0x00584286` | none |
| ground/bridge terminal input capture | verified | disassembly `0x0051BFD2..0x0051BFEE`, `0x0051C0FB..0x0051C136` | none |
| wall/crate overlay branch | verified | decompile + disassembly `0x0051C17C..0x0051C225` | none |
| warhead `Wall` predicate | verified | decompile `0x00772AC0`, parser `0x0075D3A0` | none |
| alliance mapping to codes `4/5` | verified | decompile `0x004F9A10`, assembly arithmetic at wall branch | none |
| terminal speed gate | verified | disassembly `0x0051C78B..0x0051C7D0` | none |
| bit-5, owner, full-mask final ladder | verified | disassembly `0x0051C7DF..0x0051C880` | none |
| stationary allied Infantry count | verified | object jump table, locomotor virtual call, WalkLocomotion `0x0075AB30` | none |
| stock-YR activation | verified | parser decompiles plus repo `rulesmd.ini`/`artmd.ini` | none |
| Rust disparity | verified read-only | direct source inspection | implementation is outside this report |
| zero-add pass | complete | second full decompile matched the first exactly; cold re-check added no new question | none |

## 9. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is the target really InfantryClass vtable +0x1AC? -> Yes; fresh RTTI/COL walk and slot read bind 0x007EB204 to 0x0051BF90.`
- `[RESOLVED] OQ-2 -- Which occupancy layer does the fixed retry tuple use? -> Ground byte, bit 5, and owner; resampling requires Level+4.`
- `[RESOLVED] OQ-3 -- What is the functional-full mask? -> Exactly 0x1C.`
- `[RESOLVED] OQ-4 -- Does sampled bit 5 precede ownership handling? -> Yes, when prior result is zero it immediately returns 2.`
- `[RESOLVED] OQ-5 -- What does owner -1 do? -> It skips ownership handling; zero/full later becomes 7 and zero/not-full becomes 0.`
- `[RESOLVED] OQ-6 -- What is allied-full mapping? -> Prior result below 2 becomes 6 only at stationary count 3, otherwise 2.`
- `[RESOLVED] OQ-7 -- What is counted as stationary allied Infantry? -> Allied abstract type 0x0F occupant whose locomotor +0x10 Is_Moving call returns false.`
- `[RESOLVED] OQ-8 -- Is the threshold exactly three or at least three? -> Exactly equal to three.`
- `[RESOLVED] OQ-9 -- How does enemy ownership behave with free subcells? -> Range <=0 returns 7; positive range upgrades results below 5 to 5.`
- `[RESOLVED] OQ-10 -- Can an allied non-full zero-result cell clear? -> Yes, final result 0.`
- `[RESOLVED] OQ-11 -- What does full occupancy with no owner do? -> Code 7 when no prior result exists.`
- `[RESOLVED] OQ-12 -- Does Infantry produce code 4? -> Yes, for an allied/own wall after fire and Wall-warhead gates pass.`
- `[RESOLVED] OQ-13 -- What proves the wall-capable weapon gate? -> 0x00772AC0 tests non-null WeaponType+0xAC warhead and WarheadType+0x144, parsed from Wall.`
- `[RESOLVED] OQ-14 -- When is wall classification skipped? -> When the overlay-state upper nibble equals OverlayType.DamageLevels.`
- `[RESOLVED] OQ-15 -- Does Crate share the wall result? -> No; non-player-controlled crate is an earlier immediate code-7 branch, while a player-controlled crate continues.`
- `[RESOLVED] OQ-16 -- Is Rust's free-subcell fast path exact? -> No; it omits bit-5 and hostile-owner/range outcomes.`
- `[RESOLVED] OQ-17 -- Is code 4 stock-active or TS legacy? -> Stock-active; stock walls and Wall=yes warheads exercise the data gates.`
- `[RESOLVED] OQ-18 -- Do the target slices mutate state or consume RNG? -> No; they are synchronous read-only classification in this call path.`

No open or deferred question remains inside the claimed slice.

## 10. Implementation Handoff

### Ready-to-implement behavior

1. Preserve distinct object-list layer, occupancy-bit layer, and occupancy-owner source. Under the failed-retry tuple, use ground occupancy byte/bit 5/owner even if the rare bridgehead rule selects the bridge object list.
2. Replace the Infantry free-subcell shortcut with the exact ordered terminal ladder in section 4. Do not reduce the input to `has_free_subcell`.
3. Count stationary allied Infantry from the ordered selected-layer occupant scan using each occupant's actual locomotor moving state; compare the final count to `3` by equality.
4. Add dynamic wall classification before the terminal ladder: overlay identity/state, `Wall`, `DamageLevels`, mover fire/action ability, primary weapon, warhead `Wall`, wall owner, and alliance decide codes `7`, `4`, or `5`.
5. Preserve the accumulated result ordering. In particular, sampled bit 5 acts only at result zero; allied-full override acts only below 2; hostile ownership upgrades only below 5; final nonzero wins over full occupancy.
6. Keep the retry oracle pure and synchronous. It should consume a native-shaped immutable snapshot and return the exact class code; the flood caller may continue reducing this to zero/nonzero.

### Minimum acceptance fixtures

- free subcell + enemy owner + unarmed Infantry -> `7`;
- free subcell + enemy owner + armed Infantry -> `5`;
- sampled bit 5 + prior zero -> `2`;
- allied full mask + exactly three stationary allied Infantry -> `6`;
- allied full mask + any other stationary count -> `2`;
- no owner + full mask + prior zero -> `7`;
- allied wall + usable primary `Wall=yes` warhead -> `4`;
- hostile/unowned wall + same weapon -> `5`;
- wall with unusable/no/non-Wall primary weapon -> `7`;
- wall state nibble equal to `DamageLevels` -> wall branch skipped;
- prior code `4` plus full occupancy -> `4`;
- rare bridge-object/ground-occupancy context -> bridge occupants with ground terminal byte/owner.

### Stop condition for implementation

The narrow parity gap is closed only when the exact class code for every fixture above is produced from actual simulation state and the failed-retry flood consumes that oracle result. A terrain-only or boolean-only approximation is not sufficient.

## 11. Required Follow-up to Existing Documents

The earlier `INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md` is now stale in two places:

- section 3.4's terminal subcell conclusion is superseded by section 4 of this report;
- section 3.5 and OQ-9 must no longer say code `4` is unverified for Infantry.

The failed-A* retry implementation contract may now promote the Infantry terminal and overlay/wall blockers from unknown to verified, but changing that contract is intentionally outside this research-only investigation.

## 12. Evidence Sources

- Live Ghidra MCP against `gamemd.exe`: `read_memory`, `inspect_memory_content`, `get_function_by_address`, full `decompile_function(0x0051BF90)` twice, exact function disassembly, and helper decompiles for `0x004F9A10`, `0x0050B730`, `0x005FE770`, `0x006F3970`, `0x00717880`, `0x00772AC0`, `0x0075D3A0`, `0x0075AB30`, `0x00481130`, `0x00481180`, and `0x0040DD70`.
- `docs/research/pathfinding/INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`
- `docs/research/pathfinding/PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `src/sim/pathfinding/cell_entry.rs`
- `src/sim/pathfinding/core.rs`
