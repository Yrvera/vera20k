# Phase 3 Building `SpawnSurvivors` Cargo Ejection — Active-Retail Ghidra Report

**Research date:** 2026-08-29  
**Binary:** active retail Yuri's Revenge `gamemd.exe`, image base `0x00400000`  
**Program:** `/gamemd.exe` (`C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`)  
**Primary function:** `BuildingClass::SpawnSurvivors @ 0x00442D90`  
**Mode:** read-only Ghidra, retail INI/map census, and current-Rust inspection  
**Claimed scope:** inherited Building Cargo ejection performed by `SpawnSurvivors`, with particular focus on non-explosive `UnitAbsorb`/`InfantryAbsorb` Buildings, ordinary immediate destruction, and a non-explosive Building fatally damaged while already on mission Selling  
**Non-scope:** ordinary command-sale refund/crew work, full `CanBeOccupied` garrison release internals beyond proving that it is a separate earlier mechanism, generic crew/smudge formulas already closed by `PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md`, generic `RecordKill` score formulas, and mod-only content not reachable from the mounted active-retail data  
**Status:** **IMPLEMENTATION-READY for active retail**  
**Confidence:** **HIGH**. All active gates, order, state changes, failure behavior, RNG calls, cleanup consequences, call sites, and stock reach were closed. There is no active `UNKNOWN`, `UNCHECKED`, or approximate result in this report.

## 1. Verdict

The current Rust destruction-garrison helper is not a native implementation of the absorber Cargo arm.

Native `SpawnSurvivors` does **not** treat `CanBeOccupied`, `UnitAbsorb`, and `InfantryAbsorb` as one garrison family. Its Cargo arm admits only:

```text
(BuildingType.UnitAbsorb || BuildingType.InfantryAbsorb)
&& signed Building.Cargo.count > 0
```

It pops the inherited Cargo head until empty, so the absorber's already-correct native-head boarding order becomes newest-boarded-first ejection. For each popped object it performs a footprint-coordinate probe whose result is discarded, increments `g_MapEditorMode`, copies the Building's bridge byte, and tries to Unlimbo the passenger at the Building's **raw Location X/Y** with the occupied-cell ground Z and the Building's rounded facing. For the active stock `YAPOWR`, that final X/Y is the north-west/origin foundation cell, not a perimeter or exit cell.

Successful Infantry restore normal House tracking, clear the absorber flag and destination, retain owner/health/target/Team state, and receive Hunt only when the **Building owner** is AI-controlled. Failure calls `RecordKill` with the retained source only when that source has `AbstractFlags.IsTechno`, then `UnInit`; it does not reinsert the passenger. The loop continues after failure.

`YAPOWR` is a 2x2 Building with `Passengers=5`. Its foundation list contains four offsets followed by `0x7FFF,0x7FFF`, but the Cargo loop never tests the terminator. Passenger five therefore performs its discarded preliminary probe against the shared dummy Cell. All five still try final placement at the same origin cell. That cell has only three functional Infantry subcells, so an otherwise empty, full five-passenger `YAPOWR` deterministically releases the first three popped Infantry and kills/UnInits the last two. In the ordinary uncontested case it makes exactly ten Scenario `RandomRanged(0,3)` API calls: one discarded preliminary draw and one final draw per passenger.

The cargo arm runs before the crew-budget early return. A zero crew budget never suppresses it. A non-explosive, nonselling Building calls it once and then is immediately UnInitialized. A non-explosive Building already on Selling uses duration zero: call one drains all cargo while the Building is still mapped; its later own Update calls `Limbo -> SpawnSurvivors -> UnInit`, but call two sees empty Cargo and cannot rerun a passenger. A Type-`Explodes` Building is different again: the generic fatal Techno receiver recursively destroys Cargo before either `SpawnSurvivors` call.

## 2. Address and field ledger

### 2.1 Functions and globals

| Address | Function / datum | Cargo relevance |
|---|---|---|
| `0x00442D90` | `BuildingClass::SpawnSurvivors` | complete Cargo loop, then crew/smudge continuation |
| `0x00441F1B` | first caller in `BuildingClass::DestructionEffects @ 0x004415F0` | first call before `EMPPassengers` |
| `0x004400D4` | second caller in `BuildingClass::Update @ 0x0043FB20` | after Building Limbo and before UnInit |
| `0x00473430` | Cargo pop helper | remove head, clear popped `+0x30`, decrement count |
| `0x004733A0` | `CargoClass::AddPassenger` | Limbo plus head insertion; establishes reverse boarding order |
| `0x00481180` | `CellClass::PlaceInfantryInCell` | discarded preliminary and authoritative final subcell selection |
| `0x0051DFF0` | `InfantryClass::Unlimbo` | final placement, priority override under `g_MapEditorMode` |
| `0x005217C0` | `InfantryClass::MarkCellOccupancy` | sets selected ground/deck subcell bit after success |
| `0x004D7170` | `FootClass::Unlimbo` | active object/location/lifecycle restoration |
| `0x006F6CA0` | `TechnoClass::Unlimbo` | Techno/House restoration in the successful chain |
| `0x005F4EC0` | `ObjectClass::Unlimbo` | map/display/Logic reveal path |
| `0x00702D40` | `TechnoClass::RecordKill` | failure's source-aware death record |
| `0x004DE5D0` | `FootClass::UnInit` | failure cleanup, including Team removal |
| `0x0050B730` | `HouseClass::IsControlledByHuman` | Building-owner control test for Hunt |
| `0x004C93D0` | `FacingClass::Current` | source for rounded byte facing |
| `0x005F6960` | `ObjectClass::GetOccupiedCell` | supplies authoritative final ground Z |
| `0x00565730` | `MapClass::Get_CellClass_At_Coord` | preliminary/final cell lookup, dummy on out-of-map coord |
| `0x0044262B..0x0044263F` | fatal `CanBeOccupied` test and `SellBuilding` call | proves separate pre-DestructionEffects garrison path |
| `0x00457DE0` | `BuildingClass::SellBuilding` | separate `CanBeOccupied` vector/edge-release mechanism |
| `0x0043C2D0` | `BuildingClass::Receive_Radio` | absorber entry admission, not ejection admission |
| `0x004DFB70` | capture-fate absorber selector | scans House absorber vector, initial radio `0x0F` |
| `0x00739EC0` | `UnitClass::PerCellProcess` | Unit absorber final entry/recheck |
| `0x00519630` | `InfantryClass::PerCellProcess` | active YAPOWR final entry/boarding |
| `0x00A8E7AC` | `g_MapEditorMode` | incremented/decremented once around every final attempt |
| `0x00ABDC50` | shared dummy `CellClass` | receives YAPOWR passenger-five preliminary lookup |

The Ghidra xref result for `0x00442D90` is exhaustive: only unconditional calls from `0x00441F1B` and `0x004400D4` exist.

### 2.2 Relevant object fields

| Object | Offset | Meaning in this mechanism | Evidence |
|---|---:|---|---|
| Building | `+0x114` | signed Cargo count / embedded Cargo base | `0x00442E0C..0x00442E20`; pop helper uses base `+0` |
| Building | `+0x118` | Cargo head | pop helper sees Cargo base `+4` |
| passenger | `+0x30` | Cargo next link | `0x00473438..0x0047343E` |
| Building | `+0x520` | `BuildingTypeClass*` | Cargo gate |
| BuildingType | `+0x16AE` | `UnitAbsorb` | Cargo gate and parsed key |
| BuildingType | `+0x16AF` | `InfantryAbsorb` | Cargo gate and parsed key |
| BuildingType | `+0x157B` | `CanBeOccupied` | fatal receiver's separate `SellBuilding` branch |
| Building | `+0x684` and following | `CanBeOccupied` occupant vector family | distinct from inherited Cargo `+0x114/+0x118` |
| Building | `+0x540` | retained damage/C4 source passed into both calls | both callers load it immediately before call |
| Building | `+0x6E0` | survivor-suppression latch set from fatal `ignore_defenses` | checked after preliminary probe, before final Unlimbo |
| Building | `+0x388` | facing object | read at `0x00442F40` |
| Building | `+0x8C` | `OnBridge` | copied to passenger at `0x00442EF1..0x00442EF7` |
| Building | `+0x9C/+0xA0/+0xA4` | raw Location | raw X/Y become final passenger X/Y |
| Building | `+0x21C` | owner House | human/AI mission decision |
| passenger | `+0x21C` | owner House | normal Infantry tracking count restoration |
| passenger | `+0x438` | ordinary Infantry House-tracking membership | restored on successful ejection when clear |
| passenger | `+0x439` | absorber/incoming no-normal-count flag | cleared on successful ejection |
| source | `+0x14` bit `0x01` | `AbstractFlags.IsTechno` | gates whether failure passes source to `RecordKill` |

## 3. Native storage and admission are three separate mechanisms

### 3.1 `CanBeOccupied` is not the Cargo gate

At fatal Building damage, `0x00442625` loads the BuildingType; `0x0044262B` reads Type `+0x157B`; when nonzero, `0x00442635..0x0044263B` calls `BuildingClass::SellBuilding(0,0)`. Only afterward does the receiver proceed to the Building death virtual at `0x00442665`.

That `SellBuilding` work owns the `CanBeOccupied` occupant vector and its edge-cell release behavior. `SpawnSurvivors` never reads `CanBeOccupied`, never reads the occupant vector, and never calls the edge-cell helper. Treating this vector and inherited Cargo as one passenger list is an architecture mismatch.

`YAPOWR` makes the distinction concrete: `rulesmd.ini:13157` contains only the comment `;CanBeOccupied=yes`, so its effective value is false. Its live passengers are inherited Cargo admitted by `InfantryAbsorb`, not `CanBeOccupied` garrison occupants.

### 3.2 Absorber entry gates do not become ejection gates

The full initial absorber admission is the radio-`0x0F` branch of `BuildingClass::Receive_Radio @ 0x0043C2D0`, reached by the House-vector selector at `0x004DFB70`. Its relevant ordered gates are:

1. base Radio/Techno handling;
2. directional alliance from Building to sender;
3. reject Building missions raw `0x12` and `0x13` (Selling);
4. reject `Building+0x534 == 0`;
5. unless the movement zone is Amphibious, require Building/victim naval identity parity;
6. reject `BalloonHover`;
7. require `Building::HasPower`;
8. require category-specific `UnitAbsorb` or `InfantryAbsorb`;
9. reject a victim whose own `CaptureManager` is full;
10. require `occupant_count + 1 <= Passengers` and `victim.Size <= SizeLimit`.

Unit arrival at `0x00739EC0` repeats the full `0x0F` admission before boarding. Infantry arrival at `0x00519630` uses the established contact and radio `0x15`; it rechecks the absorb kind and rejects current Selling but deliberately does not repeat all alliance, power, capacity, and size gates. On accepted active-YAPOWR Infantry arrival, native sets `+0x439`, Limbos, conditionally removes `+0x438`/House tracking, and `CargoClass::AddPassenger` head-links the Infantry.

None of alliance, mission, BState, naval identity, BalloonHover, power, CaptureManager, `Passengers`, `SizeLimit`, `Crewed`, or `CanBeOccupied` is rechecked by the ejection arm. Ejection tests only the two absorb flags and signed Cargo count. These entry predicates are therefore provenance for how stock Cargo was formed, not additional destruction gates.

### 3.3 Exact ejection gate

The admission assembly is direct:

```asm
00442DF2  MOV EAX,[EDI+520h]          ; BuildingType
00442DF8  CMP byte ptr [EAX+16AEh],BL ; UnitAbsorb
00442DFE  JNZ 00442E0C
00442E00  CMP byte ptr [EAX+16AFh],BL ; InfantryAbsorb
00442E06  JZ  00443017               ; skip Cargo arm
00442E0C  MOV EAX,[EDI+114h]          ; signed Cargo count
00442E12  LEA ECX,[EDI+114h]
00442E18  CMP EAX,EBX
00442E1A  JLE 00443017
00442E20  CALL 00473430               ; pop head
00442E27  CMP ESI,EBX
00442E29  JZ  00443017
```

The gate is an OR between `UnitAbsorb` and `InfantryAbsorb`; it does not verify that each carried object's category matches the flag that originally admitted it. A corrupt/mixed Cargo chain is still popped object by object and dispatched by each passenger's runtime `WhatAmI` result.

If count is positive but head is null, the first pop returns null and the Cargo loop stops. The pop helper does not repair the inconsistent count. This is exact malformed-state behavior; ordinary native entry keeps count/head consistent.

## 4. Call order and lifecycle

### 4.1 First call: inside `DestructionEffects`

The first call site is:

```asm
00441F07  TEST AL,AL                  ; fatal ignore_defenses
00441F09  JZ   00441F12
00441F0B  MOV byte ptr [ESI+6E0h],1  ; sticky suppression latch
00441F12  MOV EAX,[ESI+540h]          ; current retained source
00441F1A  PUSH EAX
00441F1B  CALL 00442D90               ; SpawnSurvivors
00441F20  MOV ECX,[ESP+6Ch]           ; attacker/source for next call
00441F27  CALL 00707CB0               ; EMPPassengers
```

The larger `DestructionEffects` order, already independently closed in the Explodes report, is center smudge, per-foundation `Explosion=`, active-zero FIRE3 seam, storage/callback work, timer/source installation, `DestroyAnim=`, particle selection, a direct second Health-zero write, suppression latch, first `SpawnSurvivors`, then `EMPPassengers`.

Within `SpawnSurvivors`, side/crew inputs are computed before or around this region, but the Cargo arm reaches `0x00443017` before any zero-crew early exit. Cargo is therefore completely processed first. Only after it ends does the function enter crew/smudge work. Cargo RNG calls precede every crew/smudge RNG call.

### 4.2 Second call: retained own Update

The only second call site is exact:

```asm
004400C1  MOV EAX,[ESI]
004400C5  CALL dword ptr [EAX+0D4h]   ; Building Limbo
004400CB  MOV ECX,[ESI+540h]
004400D1  PUSH ECX
004400D4  CALL 00442D90               ; fresh SpawnSurvivors call
004400D9  MOV EDX,[ESI]
004400DD  CALL dword ptr [EDX+0F8h]   ; UnInit
004400E5  CALL 00441F60               ; post-UnInit Place_OccupyMap equivalent
```

The second call recomputes its local inputs. It is not a continuation cursor from call one. For a non-explosive absorber, however, call one has already popped every Cargo node, including nodes whose placement failed, so call two's signed count is zero and it performs no Cargo lookup, placement, kill, House, Logic, occupancy, or RNG work.

### 4.3 Lifecycle matrix

| Fatal Building state | Generic fatal Cargo admission | Destruction duration | `SpawnSurvivors` Cargo observations | Final removal |
|---|---|---:|---|---|
| non-Explodes, not Selling, ordinary absorber | no explosive purge | 8 | first call sees and drains Cargo; no second call | fatal wrapper immediately UnInits after effects and runs post-UnInit cell commit |
| non-Explodes, already Selling, ordinary absorber | no explosive purge | 0 | first call drains Cargo while Building remains mapped; later second call after Limbo sees empty Cargo | own `BuildingClass::Update`: Limbo, empty second call, UnInit, cell commit |
| Type `Explodes` absorber/carrying Building | recursive head-stable fatal purge before death weapon/effects | 0 | both calls see empty Cargo | retained own Update after effects complete |
| non-Type-Explodes but effective veteran/elite `Explodes` ability or current Suicide weapon | recursive fatal purge | duration remains 8 unless current mission Selling | first call sees empty Cargo | immediate unless already Selling |
| `CanBeOccupied` Building | separate `SellBuilding` vector release before death effects | determined independently | Cargo arm runs only if an absorb flag and inherited Cargo also exist | according to duration arm |

“Selling” here means a Building fatally damaged while its current raw mission is `0x13`. Ordinary player command-sale is a different transaction and is not evidence that a fatal retained Building has Cargo on call two.

## 5. Cargo order and per-passenger algorithm

### 5.1 Stable head pop

`CargoClass::AddPassenger @ 0x004733A0` Limbos the incoming object and inserts it before the prior Cargo head. For ordinary one-object arrivals, the most recently absorbed passenger is the head.

The pop helper is only:

```asm
00473430  MOV EAX,[ECX+4]      ; head
00473433  TEST EAX,EAX
00473435  JNZ 00473438
00473437  RET                  ; null: no count repair
00473438  MOV EDX,[EAX+30h]    ; next
0047343B  MOV [ECX+4],EDX      ; new head
0047343E  MOV [EAX+30h],0      ; detached node
00473445  DEC dword ptr [ECX]  ; count--
00473447  RET
```

Thus active YAPOWR Cargo boards newest-first and ejects newest-first. Every node is permanently removed before placement is attempted; failure never restores it. `0x00443002..0x00443011` pops the next head and loops to `0x00442E2F` after both success and failure.

The Cargo link is the only parentage used by this absorber transaction. The Infantry absorber branch does not install the ordinary transport-parent pointer, and `SpawnSurvivors` does not write one. It clears the popped object's `+0x30` chain link only. Success leaves any Team membership intact; failure's `FootClass::UnInit` removes the member from its Team when `Foot+0x5D4` is non-null.

### 5.2 Exact high-level loop

The Cargo portion is equivalent to the following, preserving native order:

```text
offset_ptr = BuildingType.GetCellFootprintOffsets()

if (Type.UnitAbsorb || Type.InfantryAbsorb) && Cargo.count > 0:
    passenger = Cargo.PopHead()
    while passenger != null:
        offset = *offset_ptr
        offset_ptr += 1

        preliminary = signed16_wrap(origin_cell + offset)
        preliminary_coord = (preliminary.x * 256 + 0x80,
                             preliminary.y * 256 + 0xA4,
                             0)

        if passenger.WhatAmI() == Unit:
            preliminary_coord = CellAt(preliminary_coord).virtual_0x48()
        else:
            preliminary_coord = CellAt(preliminary_coord)
                .PlaceInfantryInCell(preliminary_coord, 0, 0, 0)

        g_MapEditorMode += 1
        passenger.OnBridge = Building.OnBridge

        final = Building.raw_Location
        final.z = Building.GetOccupiedCell().virtual_0x48().z

        if !Building.survivors_suppressed:
            facing = ((((Building.Facing.Current() >> 7) + 1) >> 1) & 0xFF)
            success = passenger.Unlimbo(final, facing)
        else:
            success = false

        if success:
            if !passenger.in_normal_infantry_tracking:
                passenger.owner.normal_infantry_count += 1
                passenger.in_normal_infantry_tracking = true
            passenger.absorber_occupant = false
            passenger.SetDestination(global_invalid_coord, 1, 0)
            if !Building.owner.IsControlledByHuman():
                passenger.QueueMission(Hunt /* raw 0x0F */, 0)
        else:
            killer = source if source != null && source.AbstractFlags.IsTechno
                     else null
            passenger.RecordKill(killer)
            passenger.UnInit()

        g_MapEditorMode -= 1
        passenger = Cargo.PopHead()
```

The preliminary result is dead data. Assembly `0x00442EDE..0x00442F32` overwrites final X/Y from Building `+0x9C/+0xA0`, then overwrites only final Z from the occupied cell. Neither the Unit cell-coordinate result nor Infantry subcell result influences final X, final Y, final Z, facing, success, or later passenger selection. Its only active effects are the lookup and, for Infantry, possible Scenario RNG consumption inside `PlaceInfantryInCell`.

## 6. Foundation pointer, YAPOWR passenger five, and final location

### 6.1 Native 2x2 list

The Foundation-2x2 list begins at `0x0089CA68`. Its initializer writes:

| Write | Entry |
|---|---|
| `0x0045B2B3` | `(0,0)` |
| `0x0045B2C6` | `(1,0)` |
| `0x0045B2DA` | `(0,1)` |
| `0x0045B2ED` | `(1,1)` |
| `0x0045B2F3` | `(0x7FFF,0x7FFF)` terminator |
| `0x0045B2FF..0x0045B309` | zero-fill the remaining table tail |

The Cargo loop advances `EBP` by four at `0x00442E38` before using each entry and contains no terminator compare. YAPOWR's `Passengers=5` therefore makes the fifth legitimate stock passenger consume the terminator as if it were an offset.

The loop adds coordinates as 16-bit values and sign-extends them. On ordinary positive map cells, adding `0x7FFF` wraps into a large negative coordinate. `MapClass::Get_CellClass_At_Coord` returns shared dummy `CellClass @ 0x00ABDC50`. Its constructor-zeroed ground/deck occupation bytes do not early-reject the Infantry probe, so passenger five still executes the preliminary quadrant-zero `RandomRanged(0,3)` call. The returned dummy-cell subcell coordinate is then discarded like all other preliminary results.

There is no sixth well-formed stock YAPOWR passenger because `Passengers=5`. A corrupt/custom over-capacity chain would next read the zero-filled table tail; that state is not produced by active retail admission and is not an implementation requirement for this row.

### 6.2 Final X/Y are the origin cell

The Building raw Location used here is the origin/north-west foundation-cell location also used to derive the packed origin cell at function entry. It is not the visual footprint center and not an edge/exit result. Final Z is refreshed from `Building.GetOccupiedCell()->GetCoord().z`, and `OnBridge` is copied independently.

Consequences for active YAPOWR:

- all passengers, including passenger five, request the same final origin-cell X/Y;
- foundation offsets affect only the discarded preliminary work and its RNG;
- no edge search, radius search, scatter, parachute, or fallback cell search occurs;
- the Building is still alive, mapped, and occupying its footprint during the first call;
- `g_MapEditorMode` makes final Infantry placement use priority override despite the still-present Building;
- the second retained call occurs after Building Limbo, but Cargo is already empty.

## 7. Infantry placement, occupancy, and exact RNG

### 7.1 `PlaceInfantryInCell`

`CellClass::PlaceInfantryInCell @ 0x00481180` derives a requested subcell from the coordinate's within-cell fraction. Both preliminary `(0x80,0xA4)` and final `(0x80,0x80)` are within 60 leptons of center, so both request subcell zero.

For subcell zero, after applicable rejection gates, the function calls Scenario `RandomRanged(0,3)` once and selects one of four rotated rows. Every row scans functional subcells 2, 3, and 4; subcells 0 and 1 are never accepted. If all three are occupied, the function returns the global invalid coordinate, but the draw has already occurred.

Without priority override:

- selected-plane occupation bit `0x20` rejects before the draw;
- ground bit `0x40` rejects before the draw unless the occupying Building has the damaged-door/garrison allowance and `CanGarrison` succeeds;
- Building occupation bit `0x80` alone does not reject.

Normal Building footprint marking uses bit `0x80`, so ordinary uncontested YAPOWR footprint cells do not block preliminary draws. The final Infantry call is stronger: `SpawnSurvivors` increments `g_MapEditorMode`, then `InfantryClass::Unlimbo` passes priority override 1 and skips the normal playfield gate. Final placement ignores the `0x20/0x40` hard rejection checks but still respects occupied functional subcells 2..4.

After a successful Foot/Object Unlimbo, `InfantryClass::Unlimbo @ 0x0051DFF0` calls vtable `+0xF0`; active Infantry binds this to `InfantryClass::MarkCellOccupancy @ 0x005217C0`, which ORs `1 << GetSubCell(coord)` into Cell ground `+0x124` or deck `+0x128`. Each success is therefore visible to the next passenger's final placement.

### 7.2 Active YAPOWR call counts and survivor count

For an ordinary ground YAPOWR with no external `0x20/0x40` preliminary blocker and no preexisting Infantry subcell occupancy:

| Cargo size | Preliminary `Ranged(0,3)` calls | Final `Ranged(0,3)` calls | Successful releases | Failed `RecordKill+UnInit` |
|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 |
| 1 | 1 | 1 | 1 | 0 |
| 2 | 2 | 2 | 2 | 0 |
| 3 | 3 | 3 | 3 | 0 |
| 4 | 4 | 4 | 3 | 1 |
| 5 | 5 | 5 | 3 | 2 |

For full Cargo, the exact API trace is ten `Scenario.RandomRanged(0,3)` calls, interleaved passenger by passenger as preliminary then final. Passengers 1-3 in stable pop order take the three functional final slots. Passenger 4 draws, scans a full cell, and fails. Passenger 5 performs the terminator/dummy preliminary draw, then its final draw, scans the same full origin cell, and fails. Failure does not free a functional slot because that passenger was never placed.

If `k` of the three functional origin-cell slots are already occupied, successful releases are `min(cargo_count, 3-k)` and the earliest popped passengers get those successes. Every unsuppressed final attempt still takes one final draw. A dynamic `0x20/0x40` blocker on a preliminary footprint cell can remove that passenger's preliminary draw, exactly because the rejection precedes the quadrant-zero call; it cannot remove the unsuppressed final draw.

`RandomRanged` is an API-level call. Its own rejection-sampling implementation can consume more than one raw PRNG word for a particular state. Therefore the parity assertion is the ordered API-call trace plus identical final Scenario RNG state, not the incorrect assumption “one raw word per call.” This is resolved behavior, not an unknown count.

### 7.3 Suppression and Unit exclusion

`Building+0x6E0` is tested only at `0x00442F2A`, after the discarded preliminary work and after incrementing `g_MapEditorMode`. With suppression set:

- Cargo is still popped completely;
- each Infantry still performs its preliminary lookup and any preliminary draw allowed by the cell flags;
- final `Unlimbo` is never called, so there is no final Infantry draw;
- every passenger runs `RecordKill+UnInit`;
- the loop and global decrement continue normally.

The Unit preliminary branch uses Cell virtual `+0x48` and takes no placement RNG. Final `UnitClass::Unlimbo @ 0x00737BA0` has no ordinary placement RNG; only UnitType animation flags `+0xE18/+0xE19` can request their documented `RandomRanged(0,0x1D)` setup draws. Active retail has zero `UnitAbsorb=yes` Building authors and zero map overrides, so no stock execution reaches this Unit cargo result. The compiled Unit gate must remain accepted, but no active-retail acceptance test should invent a stock Unit RNG case.

Cargo processing completes before crew/smudge processing at `0x00443017` and following. Thus the full-YAPOWR ten-call sequence precedes any Crewed YAPOWR survivor/smudge draws. A crew budget of zero changes none of these Cargo calls.

## 8. Success, failure, and external state

### 8.1 Successful passenger

After `passenger.Unlimbo(final,facing)` returns nonzero, native performs exactly these Cargo-specific post-actions:

1. if passenger `+0x438` is clear, increment **passenger owner** House `+0x2F4` and set `+0x438=1`;
2. clear passenger `+0x439=0`;
3. call passenger virtual `+0x174` with the global invalid destination, arguments `1,0`, clearing destination/NavCom;
4. test **Building owner** `HouseClass::IsControlledByHuman`;
5. if false, queue raw mission `0x0F` (`Hunt`) with argument zero; if true, queue no mission.

The successful chain also has these proven negative properties:

- passenger owner is not changed to the Building owner;
- passenger health is not written;
- target/archive target is not assigned or cleared by this Cargo postlude;
- no Scatter or move-away destination is requested;
- no Team removal or re-add occurs;
- no transport-parent link is installed;
- Building facing is copied only through the Unlimbo facing argument, using `((((Current >> 7)+1)>>1)&0xFF)`;
- `OnBridge` is copied from Building before the attempt;
- Cargo count/head are already updated before any of this work.

Object/Techno/Foot/Infantry Unlimbo supplies the ordinary placement consequences: `InLimbo=false`, final Location/facing, map and display membership, Logic reveal/tail registration under the native eligibility gates, Cell occupation/subcell marking, and House Added-To-Game behavior. Successful passengers become live in stable pop order during the first call, before the Building's eventual removal. The explicit `+0x438/+0x439` tail restores the normal Infantry count that absorber entry deliberately removed.

No explicit power-dirty callback occurs in this Cargo loop. The authoritative Building Cargo count nevertheless decrements before each attempt, so subsequent `GetPowerOutput` sees `ExtraPower * count` fall immediately; after call one YAPOWR contributes only its base output. The Building itself remains in its owner House/Logic/occupancy until the appropriate later UnInit.

### 8.2 Failed passenger

Failure is either `Building+0x6E0 != 0` or a false passenger `Unlimbo` result. Native then computes:

```text
killer = null                         if source == null
killer = null                         if !(source.AbstractFlags & IsTechno)
killer = source                       otherwise
passenger.RecordKill(killer)          ; virtual +0xE0
passenger.UnInit()                    ; virtual +0xF8
```

Assembly `0x00442FC5..0x00442FDE` implements the null/bit gate, `0x00442FE5` calls `+0xE0`, and `0x00442FEF` calls `+0xF8`. For Infantry the slots resolve to `TechnoClass::RecordKill @ 0x00702D40` and `FootClass::UnInit @ 0x004DE5D0`.

The popped object is already detached from Cargo. It remains concealed/limbo until UnInit, receives the valid retained Techno source for native death attribution when one exists, and is removed from its Team during Foot UnInit when linked. Capture/chrono and ordinary Object cleanup also run through that UnInit chain. There is no placement fallback, owner transfer, health repair, target rewrite, Cargo restore, or early loop termination. The next Cargo head is always attempted.

### 8.3 First-call versus second-call House/Logic/occupancy view

For the active non-explosive Selling-retained case:

1. fatal subtraction and `DestructionEffects` have Health zero;
2. first Cargo pop/placement runs while the Building is alive by Object lifecycle byte, nonlimbo, in Logic, and occupying its footprint;
3. successes enter Logic/occupancy and normal passenger-owner tracking immediately; failures are UnInitialized immediately;
4. call one ends with Cargo empty;
5. fatal receiver returns without Building UnInit because duration is zero;
6. the Building's own later live Logic visit runs the ordinary Update prefix, then Building Limbo removes map/occupancy/Logic presence;
7. second `SpawnSurvivors` sees empty Cargo;
8. Building UnInit and post-UnInit cell commit finish removal.

There is no window in which the retained Building still owns a passenger that call two can rediscover. Save/load of the retained window preserves an empty Cargo and pending Building cleanup, not a deferred passenger release.

## 9. Active-retail census

### 9.1 Corpus and reproducibility

The read-only census used:

- merged `ini/rules.ini` then `ini/rulesmd.ini` by INI section/key authority;
- merged `ini/art.ini` then `ini/artmd.ini` for art fields;
- all 184 extracted `.map`/`.mpr` files under `target/phase3-retail-census/extract`;
- all 11,992 `[Structures]` rows;
- every map-local type section, applied as a later Rules pass for relevant keys.

Hashes:

| File | SHA-256 |
|---|---|
| `ini/rulesmd.ini` | `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF` |
| `ini/artmd.ini` | `E1F0378394313C04EBBD5073F47785EE3E46F1B3C62D65724E8F3C310EE7BA31` |

Effective `[BuildingTypes]` has 403 numbered entries and 402 unique IDs because `NAPSYA` appears at keys 185 and 241. The only canonical types that explicitly author either absorb key are:

| Type | `UnitAbsorb` | `InfantryAbsorb` | `Passengers` | `SizeLimit` | `Crewed` | Active result |
|---|---|---|---:|---:|---|---|
| `YAPOWR` | no | yes | 5 | 15 | yes | active stock Cargo ejection proof |
| `YAROCK` | No | absent/false | absent/default | absent/default | No | no positive absorb gate; excluded |

No active stock Building authors `UnitAbsorb=yes`. No mounted map overrides `UnitAbsorb` or `InfantryAbsorb`. No map-local YAPOWR section overrides `Explodes`, `CanBeOccupied`, `Passengers`, `SizeLimit`, or `Crewed`. `all02umd.map` has a YAPOWR section, but its authored fields are ordinary name/owner/build/cost/strength/armor/power data; none changes this mechanism.

### 9.2 YAPOWR authoritative fields

`rulesmd.ini:13125..13163` and `artmd.ini:3228..3233` establish:

```ini
[YAPOWR]
Strength=700
Power=150
Crewed=yes
UnitAbsorb=no
InfantryAbsorb=yes
;CanBeOccupied=yes       ; comment, not authority
Passengers=5
SizeLimit=15
ExtraPower=100

[YAPOWR]                 ; artmd.ini
Foundation=2x2
```

YAPOWR has no active authored `Explodes=yes`; the BuildingType constructor/default therefore leaves Type `Explodes=false`. It is the ordinary stock non-explosive trigger for this report, available both as a player-built Yuri power structure and as a preplaced map Structure.

### 9.3 Preplaced stock reach

There are 204 preplaced YAPOWR Structures across 13 maps:

| Map | Count |
|---|---:|
| `all02umd.map` | 20 |
| `all03umd.map` | 3 |
| `all04dmd.map` | 19 |
| `all05umd.map` | 16 |
| `all06umd.map` | 9 |
| `all07smd.map` | 41 |
| `c1a02md.map` | 5 |
| `c2s02md.map` | 4 |
| `sov03umd.map` | 14 |
| `sov04dmd.map` | 15 |
| `sov05umd.map` | 19 |
| `sov06lmd.map` | 24 |
| `sov07tmd.map` | 15 |

Owner census, independently summing to 204:

| Owner | Count | Owner | Count |
|---|---:|---|---:|
| Africans | 9 | Alliance | 13 |
| Americans | 6 | Arabs | 8 |
| BadGuy1 House | 27 | BadGuy2 | 12 |
| BadGuy2 House | 4 | Civie3 | 1 |
| Neutral | 9 | Yuri2 | 13 |
| Yuri2Country | 2 | Yuri3 | 6 |
| Yuri4 | 1 | YuriCountry | 93 |

The Cargo mechanism is not restricted to these preplacements. Ordinary skirmish Yuri construction plus Infantry entry is the frequent player-visible case.

## 10. Current Rust comparison

### 10.1 Correct pieces to preserve

| Native requirement | Current Rust evidence | Verdict |
|---|---|---|
| parse `InfantryAbsorb` / `UnitAbsorb` | `src/rules/object_type.rs:1715..1716` | match |
| absorber arrival sets represented special occupant state and removes normal Infantry tracking | `src/sim/capture_fate_facility.rs:397..425` | match |
| absorber uses native-head insertion | `src/sim/capture_fate_facility.rs:431..437`, `src/sim/passenger.rs:104..115` | match |
| head pop can preserve that stable order and decrements total size | `src/sim/passenger.rs:129..138` | reusable match |
| raw mission 15 maps to Hunt; raw 19 maps to Selling | `src/rules/mission_data.rs:77,138,184`, Selling tests/enum | match |
| passenger owner remains authoritative during absorber handling | current absorber entry avoids rehoming | preserve |

### 10.2 Active mismatches

1. `src/sim/world/mod.rs:1999..2015` forms one destruction-garrison event when **any** of `can_be_occupied || infantry_absorb || unit_absorb` is true. Native has separate `CanBeOccupied` vector/SellBuilding work and inherited Cargo/SpawnSurvivors work.
2. `src/sim/world/mod.rs:2030..2038` executes that ejection in `BeforeDeathEffects`, before the Building's native `DestructionEffects` call point. Native non-explosive absorber Cargo is owned by the first `SpawnSurvivors` near the end of DestructionEffects.
3. `src/sim/production/production_sell.rs:644..650` detaches all Cargo in bulk through `take_for_uninit`; native pops one head at a time, performs state/RNG/lifecycle work, then pops the next.
4. `src/sim/production/production_sell.rs:656..675` recognizes an absorber but still calls `eject_garrison_passengers_at_edges`. Suppressing owner rehome does not make edge placement native. Active YAPOWR passengers must all attempt the raw Building origin cell.
5. The edge helper does not reproduce the discarded foundation probe, the 2x2 terminator/dummy fifth probe, per-passenger `g_MapEditorMode` priority, exact two-draw trace, three-functional-subcell cap, Building facing rounding, `OnBridge` copy, success tracking bytes/counter, destination clear, Building-owner AI Hunt, or source-aware failure `RecordKill+UnInit`.
6. Bulk event snapshots make it easy to preserve passengers that native already killed or to lose same-loop occupancy feedback. Native passenger 4/5 observe the subcell bits marked by passengers 1-3.
7. `src/sim/world/mod.rs:2041..2056` immediately UnInits every fatal Structure. It cannot represent the non-explosive-current-Selling duration-zero case or its later empty second call.
8. Current tests around `production_sell.rs:1378` and following encode absorber edge-ejection behavior. Those are regression tests for the approximation, not native evidence; they must be replaced or narrowed to the distinct `CanBeOccupied` path.

The revised Phase 3 Explodes design also needs a wording correction: “preserve existing non-Explodes occupied/absorber ejection” is too coarse. Preserve native `CanBeOccupied` edge release; replace absorber destruction ejection with this exact Cargo arm.

## 11. Rust implementation handoff

### 11.1 Required ownership split

Implement one Building lifecycle survivor owner, consistent with the revised design's `src/sim/building_survivors.rs`, and put the Cargo arm at the beginning of each exact `SpawnSurvivors` transaction. Do not route it through production command-sale code.

The owner must accept/read live Building identity, current source identity, suppression latch, foundation/origin facts, map/occupancy, Scenario RNG, House control/tracking, and lifecycle services. It must run synchronously so every passenger sees prior passenger placement and cleanup.

Keep these mechanisms separate:

- fatal `CanBeOccupied`: native `SellBuilding` occupant vector and edge search;
- fatal explosive admission: recursive Cargo destruction before death weapon/effects;
- non-explosive absorber: `SpawnSurvivors` head-pop loop in this report;
- generic non-absorber carried Cargo: ordinary UnInit recursion unless another verified owner applies.

### 11.2 Exact Cargo transaction

For each call:

1. test only effective `unit_absorb || infantry_absorb` and nonempty Cargo;
2. obtain the native foundation-offset list/cursor even though the placement result is discarded;
3. pop one head and immediately update count/size/link state;
4. consume one offset without a terminator test;
5. perform category-specific preliminary lookup/Infantry subcell probe with native flags and Scenario RNG;
6. enter the map-editor-priority-equivalent scope;
7. copy Building `OnBridge`, compose final raw Building Location X/Y plus occupied-cell Z, and round Building facing exactly;
8. when unsuppressed, try category-native Unlimbo/placement at that final coordinate;
9. on success, restore Infantry tracking only when absent, clear absorber flag and destination, preserve owner/health/target/Team, and queue Hunt only for AI-controlled Building owner;
10. on failure, pass the non-null retained source only when its represented Abstract category flag is Techno—there is no additional `IsAlive +0x90` test—then fully UnInit the passenger, including Team/links;
11. leave the priority scope and pop the next head regardless of result;
12. only after Cargo empties, evaluate/continue crew and smudge work.

Do not calculate an edge, exit cell, footprint center, nearest free cell, scatter cell, or fallback. Do not put failed passengers back into Cargo. Do not cap the loop by foundation length. Do not repeat call-one Cargo at retained cleanup.

### 11.3 Lifecycle integration

- Non-Explodes, nonselling: call one drains Cargo, then the existing immediate Building UnInit/post-cell transaction runs.
- Non-Explodes, current Selling: install the same duration-zero pending cleanup required by the Explodes design. Call one drains Cargo. Preserve the empty Cargo through save/load. Own Update performs Limbo, fresh call two (empty Cargo), UnInit, and post-cell work.
- Type/effective explosive admission: preserve recursive Cargo annihilation before death weapon; both survivor calls must see no Cargo.
- `ignore_defenses`: set the sticky suppression latch before call one. Do not skip the Cargo arm or its preliminary work.

## 12. Acceptance tests

The following tests are required to close this mechanism. They should assert state and ordered RNG/lifecycle traces, not just returned survivor counts.

1. **Gate truth table:** Cargo present but neither absorb flag -> no Cargo arm; either flag alone -> arm; both -> one arm, not two; signed-zero count -> skip.
2. **Entry gates are not ejection gates:** after boarding, change alliance, power, mission, capacity, SizeLimit, and BState; destruction still ejects because only absorb flag/count are rechecked.
3. **CanBeOccupied separation:** a pure `CanBeOccupied` Building uses its native vector/edge path and never executes absorber preliminary/final RNG; a pure YAPOWR-shaped absorber never uses an edge.
4. **Native head order:** board IDs 10, 20, 30 through absorber native-head insertion; destruction attempts 30, 20, 10.
5. **One-by-one feedback:** with five Infantry and empty origin subcells, attempts 1-3 succeed and mark distinct functional slots; attempts 4-5 fail and UnInit.
6. **YAPOWR exact RNG:** full ordinary 2x2 YAPOWR emits `prelim Ranged(0,3), final Ranged(0,3)` five times, ten calls total, before crew/smudge RNG; assert identical final Scenario RNG state.
7. **Fifth terminator:** passenger five uses offset `(0x7FFF,0x7FFF)`, resolves dummy Cell for preliminary probe, consumes its preliminary draw, then still attempts final raw origin.
8. **No foundation terminator stop:** full Cargo processes passenger five; it is not left in Cargo for call two or UnInit recursion.
9. **Origin, not edge:** all final X/Y requests equal Building raw Location; foundation offsets and preliminary subcell outputs never affect final X/Y/Z.
10. **Facing:** cover values around both rounding boundaries and assert `((((Current >> 7)+1)>>1)&0xFF)` passed to every unsuppressed Unlimbo.
11. **Bridge:** copied `OnBridge` plus occupied-cell Z are visible to every attempt; no preliminary Z survives.
12. **Human success:** successful passenger owner/health/target/Team/mission remain unchanged except destination clear and absorber/tracking state.
13. **AI success:** Building owner AI queues Hunt raw 15 on successful passenger even if passenger owner/control differs; no target is assigned.
14. **Tracking idempotence:** `+0x438=false/+0x439=true` increments passenger-owner Infantry count once, sets/clears bytes; already-`+0x438=true` does not double increment.
15. **Failure source:** null source and non-Techno source call RecordKill(null); a non-null Techno-category source is passed even without an additional IsAlive test; then UnInit runs.
16. **Failure Team cleanup:** successful passenger stays in Team; failed passenger is removed exactly once by Foot UnInit.
17. **Failure continues:** force passenger one placement failure with later free state for passenger two; passenger two is still popped and attempted.
18. **Preexisting origin occupants:** with one/two/three functional slots occupied, successes are two/one/zero in pop order; every unsuppressed final attempt still draws once.
19. **Preliminary blocker:** set applicable `0x20` or disallowed `0x40` on one footprint probe cell; that preliminary attempt consumes no draw, while final priority placement still draws/attempts.
20. **Suppression:** `+0x6E0` full YAPOWR pops and kills all five, performs each allowed preliminary draw, performs zero final Unlimbo/final draws, balances priority scope, leaves Cargo empty.
21. **Crew-zero independence:** force crew budget zero and prove the full Cargo transaction still occurs; no crew/smudge continuation draw follows.
22. **Immediate lifecycle:** non-Explodes nonselling YAPOWR drains Cargo once, then Building UnInit/post-cell occurs; no pending retained cleanup exists.
23. **Selling retained lifecycle:** non-Explodes YAPOWR already on Selling drains Cargo in first call while mapped, returns Health-zero/alive/nonlimbo/in-Logic with empty Cargo, then own Update performs Limbo -> empty second call -> UnInit -> cell commit.
24. **Selling save/load:** save in that retained window and reload; Cargo remains empty, passenger successes/failures remain committed, RNG state remains advanced, and cleanup does not rerun them.
25. **Explosive control:** Type-Explodes Cargo is recursively destroyed before death weapon/effects and both Spawn calls; no placement RNG or success tracking occurs.
26. **Ability/Suicide control:** explosive fatal admission without Type Explodes purges Cargo, but duration remains immediate unless current mission is Selling.
27. **Power observation:** after each pop, active Building Cargo count/size reflects the removal; after call one YAPOWR ExtraPower occupancy bonus is zero even though retained Building base power/lifecycle remains until cleanup.
28. **No transport relink:** success clears Cargo next and leaves no absorber transport parent; failure does not leave dangling Cargo/transport/Team links.

## 13. Coverage ledger and evidence-backed exclusions

| Question | Result | Evidence |
|---|---|---|
| all call sites | closed | exhaustive xrefs: `0x00441F1B`, `0x004400D4` only |
| ejection flags/count gate | closed | `0x00442DF2..0x00442E20` |
| `CanBeOccupied` relation | closed/separate | `0x0044262B..0x0044263B`, distinct fields/helper |
| entry admission and final rechecks | closed | `0x0043C2D0`, `0x004DFB70`, `0x00739EC0`, `0x00519630` |
| stable pop/link/count | closed | `0x00473430..0x00473447`, AddPassenger head insert |
| offset order/terminator | closed | `0x00442E2F..0x00442E5C`, `0x0045B2B3..0x0045B309` |
| preliminary result fate | closed/discarded | overwritten at `0x00442EDE..0x00442F32` |
| final coordinate/facing/bridge | closed | `0x00442EF1..0x00442F63` |
| success writes and non-writes | closed | `0x00442F6D..0x00442FC3` plus call-target inspection |
| failure/kill/UnInit | closed | `0x00442FC5..0x00442FF5`; vtable resolution |
| loop continuation | closed | `0x00443002..0x00443011` |
| Infantry slots/occupation | closed | `0x00481180`, `0x0051DFF0`, `0x005217C0` |
| RNG call order/count | closed | both PlaceInfantry call sites plus final priority path |
| House/Logic/occupancy | closed | Unlimbo/Reveal/Mark and explicit tracking tail |
| second-call Cargo | closed/empty | call-one exhaustive pop plus only later call site |
| nonretained/retained/explosive split | closed | fatal receiver, timer, both call sites, own Update |
| active stock authors | closed | effective INI and 184-map census |
| ordinary stock trigger | closed | YAPOWR rules/art plus 204 preplacements/player construction |
| current Rust delta | closed | direct read of world, production, cargo, absorber entry |

Evidence-backed exclusions:

- **Stock UnitAbsorb execution:** zero positive canonical authors and zero map overrides. The compiled OR gate remains required, but its stock trigger frequency is zero.
- **YAROCK:** explicitly `UnitAbsorb=No` and no positive InfantryAbsorb; cannot enter Cargo arm from active type data.
- **CanBeOccupied as a YAPOWR gate:** commented line has no INI authority; binary Cargo arm does not read the field anyway.
- **Explodes YAPOWR:** no canonical or relevant map `Explodes=yes`; ordinary YAPOWR is the non-explosive control. Synthetic type/ability/Suicide explosive admission remains a control path, not YAPOWR's default.
- **Second-pass passenger release:** impossible for well-formed active Cargo because call one pops every node before any result and never restores failures.
- **Edge/perimeter search:** no such call or coordinate survives in the Cargo loop.
- **Crew-budget suppression of Cargo:** budget exit is after Cargo arm.
- **Passenger owner transfer:** no writer in the loop; Building owner is read only for human/AI mission choice.
- **Mission target assignment:** no target/archive-target setter in Cargo success; destination clear and optional Hunt are the only navigation/mission tail.
- **Health/facing-state approximation:** health is untouched; facing is the exact rounded Building current facing passed to native Unlimbo.
- **Mod over-capacity after YAPOWR passenger five:** active admission caps at five. The zero-filled post-terminator tail is compiled but not an active-retail requirement.
- **Raw PRNG word equals API-call count:** excluded as false. Exact parity uses ordered `RandomRanged` calls and final stream state.

## 14. Resolved question log

| Question | Resolution |
|---|---|
| Does `CanBeOccupied` admit SpawnSurvivors Cargo? | **RESOLVED:** no; fatal `SellBuilding` owns a distinct vector before death effects. |
| Must both absorb flags match passenger category at ejection? | **RESOLVED:** no; either flag plus positive count admits the loop, then runtime `WhatAmI` dispatches each object. |
| Are `Passengers` and `SizeLimit` ejection gates? | **RESOLVED:** no; entry only. |
| Can zero crew budget leave Cargo inside? | **RESOLVED:** no; Cargo drains first. |
| What is the pop order? | **RESOLVED:** Cargo head order, newest ordinary absorber arrival first. |
| Is a failed passenger restored to Cargo? | **RESOLVED:** no; it was detached/count-decremented before attempt and is UnInitialized. |
| Does failure stop later passengers? | **RESOLVED:** no; next head is always popped. |
| Are foundation coordinates the final positions? | **RESOLVED:** no; preliminary result is overwritten. |
| Where is final X/Y? | **RESOLVED:** Building raw Location, the origin/NW foundation cell. |
| Where is final Z? | **RESOLVED:** current occupied-cell coordinate Z. |
| What happens to YAPOWR passenger five? | **RESOLVED:** consumes the 2x2 terminator, probes dummy Cell, then attempts the same final origin cell. |
| How many Infantry can leave an empty full YAPOWR? | **RESOLVED:** exactly three succeed, two fail. |
| How many ordinary Cargo RNG calls occur? | **RESOLVED:** ten `Ranged(0,3)` API calls for five Infantry, before crew/smudge. |
| Does suppression skip Cargo? | **RESOLVED:** no; it skips final Unlimbo only after preliminary work, then kills every passenger. |
| Does Building occupancy block final Infantry placement? | **RESOLVED:** no; `g_MapEditorMode` forces priority override, while functional Infantry slots still constrain placement. |
| Which owner controls Hunt? | **RESOLVED:** Building owner control; passenger owner is preserved. |
| What mission is raw `0x0F`? | **RESOLVED:** Hunt, not Sleep. |
| Are target, owner, Team, or health rewritten on success? | **RESOLVED:** no; destination, tracking/absorber flags, and optional mission are the Cargo postlude. |
| What source reaches failure RecordKill? | **RESOLVED:** non-null source only when its AbstractFlags bit0 marks a Techno; otherwise null. |
| Does success clear Team? | **RESOLVED:** no. Failure UnInit removes Team membership. |
| Does a non-explosive nonselling Building get call two? | **RESOLVED:** no; duration eight is immediately UnInitialized by fatal wrapper. |
| Does a non-explosive Selling-retained Building get call two? | **RESOLVED:** yes, after Limbo, but call one has emptied Cargo. |
| Does Type Explodes reach this Cargo ejection? | **RESOLVED:** calls occur but generic fatal admission already destroyed Cargo. |
| Is stock UnitAbsorb active? | **RESOLVED/excluded:** compiled support exists; active retail authors/overrides are zero. |
| Is current Rust edge release equivalent? | **RESOLVED:** no; it differs in coordinate, RNG, capacity failure, order, state, and lifecycle. |

## 15. Adversarial checks and zero-add pass

The following independent attacks were applied before declaring the report ready:

1. **Gate inversion:** disassembled the exact Type/count block and verified the OR/positive-signed-count shape; no `CanBeOccupied`, `Crewed`, mission, or capacity read is hidden in it.
2. **Caller census:** exhaustive function xrefs found two calls only, ruling out a sell-command or third cleanup invocation.
3. **Coordinate-liveness attack:** followed both Unit and Infantry preliminary results forward and proved raw Building X/Y overwrite them before Unlimbo.
4. **Terminator attack:** disassembled the 2x2 table initializer and the loop pointer increment; the fifth entry is terminator and no comparison exists.
5. **Dummy-RNG attack:** followed the signed-16 wrap to out-of-map lookup and dummy-cell zero occupation, proving passenger five's preliminary draw survives.
6. **Occupancy-feedback attack:** independently resolved Infantry vtable `+0xF0` to `0x005217C0`, proving each successful final placement marks a bit observed by later attempts.
7. **Building-blocker attack:** decompiled `PlaceInfantryInCell`; bit `0x80` alone does not reject, while final priority override bypasses `0x20/0x40` rejection.
8. **Mission-number attack:** checked current mission enum/table; raw 15 is Hunt and raw 19 is Selling.
9. **Failure-source attack:** inspected the raw `+0x14 & 1` assembly and cross-checked AbstractFlags bit0 as IsTechno; it is not Object `IsAlive +0x90`.
10. **Team-link attack:** traced success past the Cargo postlude and failure into `FootClass::UnInit`; only failure removes Team membership.
11. **Second-call attack:** combined exhaustive head drain, absence of restore, only two call sites, and Limbo-before-call-two order. No retained passenger path remains.
12. **Lifecycle inversion:** rechecked duration eight versus duration zero in the already-verified fatal wrapper/own Update: eight removes immediately; zero retains for own Update.
13. **Storage-conflation attack:** independently disassembled fatal `CanBeOccupied` -> `SellBuilding` before the Building death virtual; this is not the Cargo list.
14. **Active-data attack:** parsed all effective BuildingTypes and map-local type overrides, not just canonical text search; only YAPOWR is positive.
15. **Map-frequency attack:** counted only `[Structures]` rows, avoiding `[AITriggerTypes]`, task forces, or map type-list references that also mention YAPOWR.
16. **Rust freshness attack:** reread the active feature worktree after the native trace; `world/mod.rs` still merges three mechanisms and `production_sell.rs` still uses edge ejection.

Two cold spot-checks were deliberately separated from the main trace:

- a fresh `0x00481180`/`0x0051DFF0` decompile reproduced the preliminary/final draw and priority-override interpretation;
- a fresh vtable-memory read and `0x005217C0` decompile reproduced functional-subcell marking, independently supporting the three-success result.

The final zero-add pass searched for additional `SpawnSurvivors` callers, another Cargo restore/reinsert, a terminator check, any owner/target/health/Team writer on success, a placement fallback, a second-call Cargo producer, stock positive UnitAbsorb, or a relevant map override. It found none. The active mechanism is closed.

## 16. Documentation corrections and annotation candidates

Two older reports should not be used as implementation authority for this arm:

- `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` describes Cargo release as exit-cell work and labels raw `0x0F` as Sleep in this context. Fresh assembly proves discarded foundation probes, final raw Building origin, and Hunt.
- `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` describes the Spawn gate as `CanBeOccupied`. Fresh assembly proves Type `UnitAbsorb || InfantryAbsorb` plus positive Cargo count.

No Ghidra annotations were applied. Candidate metadata for a later certainty-gated sync:

- name/plate `0x00442D90` as Building SpawnSurvivors with explicit Cargo-first arm;
- plate `0x00473430` as Cargo pop-head/count-decrement;
- comment `0x00442EDE` that preliminary placement is discarded but effectful for RNG;
- comment `0x00442F2A` that suppression is tested after preliminary work;
- comment `0x00443011` that loop is governed by Cargo head, not foundation terminator.

## 17. Sources inspected

- live active `/gamemd.exe` Ghidra program and exact functions/assembly listed in §2;
- `docs/research/PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md` for the already-closed enclosing fatal timer/retention and generic explosive-admission order;
- `docs/research/CAPTURE_MANAGER_FATE_GRINDER_ABSORBER_CONTINUATIONS_GHIDRA_REPORT.md` for absorber selector/entry admission, revalidated where load-bearing against live functions;
- `docs/plans/2026-08-29-phase3-building-explodes-lifecycle-design.md` for the current implementation boundary needing correction;
- mounted `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`;
- 184 extracted retail maps under `target/phase3-retail-census/extract`;
- current Rust `src/sim/world/mod.rs`, `src/sim/production/production_sell.rs`, `src/sim/passenger.rs`, `src/sim/capture_fate_facility.rs`, `src/rules/object_type.rs`, and `src/rules/mission_data.rs`.

No Rust was edited, no Cargo command was run, no commit was created, and no Ghidra metadata was mutated during this investigation.
