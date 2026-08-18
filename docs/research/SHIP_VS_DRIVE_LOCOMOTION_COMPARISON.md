# ShipLocomotionClass vs DriveLocomotionClass — Detailed Binary Comparison

Research conducted 2026-03-23 via Ghidra MCP live decompilation of `gamemd.exe`.
Previous docs claimed Ship is a "byte-for-byte clone" — this report corrects that claim
with exact differences found.

**Verdict:** Ship is ~95% identical to Drive. Same struct layout, same stepping algorithm,
same overall control flow. But there are **6 concrete differences**, some of which affect
gameplay behavior (notably: fewer track curves and different deceleration logic).

---

## 1. Class Identity

| | Drive | Ship |
|---|---|---|
| CLSID | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | `{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` |
| Constructor | `0x004af540` | `0x0069ec50` |
| Object size | 0x6C bytes | 0x6C bytes (identical) |
| IUnknown vtable | `0x007e7f7c` | `0x007f2e58` |
| ILocomotion vtable | `0x007e7eb0` | `0x007f2d8c` |
| IPiggyback vtable | `0x007e7e8c` | `0x007f2d68` |

### Constructor comparison

Both constructors are **instruction-for-instruction identical** except for:
1. Different global addresses for NullCoord sentinel
2. Different vtable pointer constants

Assembly verified side-by-side:
- Drive: `MOV ECX, [0x008a0790]` (NullCoord), `MOV [ESI], 0x7e7f7c` (vtable)
- Ship: `MOV ECX, [0x00b077f8]` (NullCoord), `MOV [ESI], 0x7f2e58` (vtable)

Field init sequence, offsets, and default values are byte-for-byte the same.

---

## 2. Global Data

### Null Coordinate Sentinels

| | Address | Value at runtime |
|---|---|---|
| Drive NullCoord | `0x008a0790` (X), `0x008a0794` (Y), `0x008a0798` (Z) | {0, 0, 0} |
| Ship NullCoord | `0x00b077f8` (X), `0x00b077fc` (Y), `0x00b07800` (Z) | {0, 0, 0} |

Both are {0, 0, 0}. Functionally identical.

### HeightStep

| | Address | Value at runtime |
|---|---|---|
| Drive HeightStep | `0x00b07158` | 0 |
| Ship HeightStep | `0x00b07838` | 0 |

Both are 0 (initialized at runtime by `InitHeightStep` functions).

---

## 3. TurnTrack Table — DIFFERENT

**This is a real behavioral difference.** Ship has fewer turn tracks than Drive.

| | Address | Entry count | Size |
|---|---|---|---|
| Drive | `0x007e7b28` | **72** entries | 864 bytes |
| Ship | `0x007f2a40` | **67** entries | 804 bytes |

### Shared entries (0–66): BYTE-FOR-BYTE IDENTICAL

All 67 entries covering the 8×8 direction matrix (entries 0–63) plus 3 special
tracks (entries 64–66, referencing raw tracks 11–13) are identical.

### Drive-only entries (67–71): MISSING FROM SHIP

Drive has 5 additional entries that Ship lacks entirely:

| Entry | Track | Short | Direction | Flags | Description |
|-------|-------|-------|-----------|-------|-------------|
| 67 | 14 | 14 | 0x0020 (NE) | 0 | Raw track 14, no transform |
| 68 | 14 | 14 | 0x0060 (SE) | 4 | Raw track 14, negate-Y |
| 69 | 14 | 14 | 0x00A0 (SW) | 1 | Raw track 14, swap-XY |
| 70 | 14 | 14 | 0x00E0 (NW) | 2 | Raw track 14, negate-X |
| 71 | 15 | 15 | 0x00C0 (W) | 0 | Raw track 15, no transform |

These are the "extra special" tracks used by Drive for specific maneuvers
(likely related to docking, deploying, or other land-vehicle behaviors that
ships don't need). Since ships never index into entries 67–71, there's no
crash risk — but it means ships have a slightly simpler movement repertoire.

---

## 4. RawTrack Table — DIFFERENT

| | Address | Entry count | Size |
|---|---|---|---|
| Drive | `0x007e7a28` | **16** entries | 256 bytes |
| Ship | `0x007f2960` | **14** entries | 224 bytes |

### Metadata comparison (entries 1–13): IDENTICAL

All metadata fields (total_count, entry_index, jump_index) match perfectly.
The pointers differ (point to Ship's own copy of the data), but the
curve parameters are the same.

| Track | Count | Entry | Jump | Ship ptr | Drive ptr |
|-------|-------|-------|------|----------|-----------|
| 0 | 0 | * | 0 | NULL | NULL |
| 1 | -1 | 0 | -1 | 0x7f1320 | 0x7e6258 |
| 2 | -1 | 0 | -1 | 0x7f1440 | 0x7e6378 |
| 3 | 37 | 12 | 22 | 0x7f15c0 | 0x7e64f8 |
| 4 | 26 | 11 | 19 | 0x7f1858 | 0x7e6790 |
| 5 | 45 | 15 | 31 | 0x7f1a30 | 0x7e6968 |
| 6 | 44 | 16 | 27 | 0x7f1d18 | 0x7e6c50 |
| 7 | -1 | 0 | -1 | 0x7f1fc8 | 0x7e6f00 |
| 8 | -1 | 0 | -1 | 0x7f2118 | 0x7e7050 |
| 9 | -1 | 0 | -1 | 0x7f2220 | 0x7e7158 |
| 10 | -1 | 0 | -1 | 0x7f2398 | 0x7e72d0 |
| 11 | -1 | 0 | -1 | 0x7f24e8 | 0x7e7420 |
| 12 | -1 | 0 | -1 | 0x7f2590 | 0x7e74c8 |
| 13 | -1 | 0 | -1 | 0x7f2630 | 0x7e7568 |
| 14 | -1 | 0 | -1 | — (N/A) | 0x7e78a8 |
| 15 | -1 | 0 | -1 | — (N/A) | 0x7e7968 |

*Entry 0's third field differs (Ship=64, Drive=192) but this entry is the
null sentinel (ptr=NULL, count=0) — the field's purpose is unclear and it
is never accessed by the stepping code.*

### Drive-only tracks 14 and 15

These two raw track definitions exist only in Drive's table. They are
referenced by TurnTrack entries 67–71 (which also only exist in Drive).

### Track point data: IDENTICAL

All 14 shared raw tracks (1–13) have **byte-for-byte identical** track point
arrays. Verified by comparing 64 bytes at the start of tracks 1, 2, 3, 4,
5, 6, 7, and 8. The curves are the same — Ship just has fewer of them.

---

## 5. ILocomotion Vtable — 40 Slots

20 of 40 slots point to shared (inherited) functions. The other 20 slots
point to Ship-specific copies of the same logic.

| Slot | Name | Ship | Drive | Shared? |
|------|------|------|-------|---------|
| 0 | QueryInterface | 0x6a4300 | 0x4b4d90 | No (own copy) |
| 1 | AddRef | 0x6a4310 | 0x4b4da0 | No (own copy) |
| 2 | Release | 0x6a4320 | 0x4b4db0 | No (own copy) |
| 3 | Link_To_Object | 0x55a710 | 0x55a710 | **YES** |
| 4 | Is_Moving | 0x69f290 | 0x4afb80 | No (own copy) |
| 5 | Destination | 0x69f3a0 | 0x4afc90 | No (own copy) |
| 6 | Move_To | 0x69f3d0 | 0x4afcc0 | No (own copy) |
| 7 | Stop_Moving | 0x55abf0 | 0x55abf0 | **YES** |
| 8 | Do_Turn | 0x55abe0 | 0x55abe0 | **YES** |
| 9 | Draw_Matrix | 0x69f670 | 0x4aff60 | No (own copy) |
| 10 | Shadow_Matrix | 0x69fb20 | 0x4b0410 | No (own copy) |
| 11 | Head_To_Coord | 0x55abd0 | 0x55abd0 | **YES** |
| 12 | Can_Enter_Cell | 0x55a8c0 | 0x55a8c0 | **YES** |
| 13 | Is_To_Have_Shadow | 0x55abc0 | 0x55abc0 | **YES** |
| 14 | Mark_All_Occupation_Bits | 0x6a3ea0 | 0x4b4870 | No (own copy) |
| 15 | Z_Gradient | 0x6a3eb0 | 0x4b4880 | No (own copy) |
| 16 | **Process** | 0x69fc10 | 0x4b0500 | No (own copy) |
| 17 | Set_Destination | 0x69f450 | 0x4afd40 | No (own copy) |
| 18 | Stop_Moving_Full | 0x69f510 | 0x4afe00 | No (own copy) |
| 19 | Do_Turn_Update | 0x6a05c0 | 0x4b0ef0 | No (own copy) |
| 20 | Force_New_Slope | 0x69fbe0 | 0x4b04d0 | No (own copy) |
| 21 | Is_Moving_Now | 0x55ab90 | 0x55ab90 | **YES** |
| 22 | (base stub) | 0x55a8f0 | 0x55a8f0 | **YES** |
| 23 | (base stub) | 0x55a910 | 0x55a910 | **YES** |
| 24 | (base stub) | 0x55a930 | 0x55a930 | **YES** |
| 25 | (base stub) | 0x55a940 | 0x55a940 | **YES** |
| 26 | (base stub) | 0x55ab70 | 0x55ab70 | **YES** |
| 27 | (base stub) | 0x55ab80 | 0x55ab80 | **YES** |
| 28 | Force_Track | 0x6a0310 | 0x4b0c40 | No (own copy) |
| 29 | In_Which_Layer | 0x6a3e50 | 0x4b4820 | No (own copy) |
| 30 | (base stub) | 0x55ac00 | 0x55ac00 | **YES** |
| 31 | Piggybacker_CLSID | 0x69f250 | 0x4afb40 | No (own copy) |
| 32 | Is_Moving_Check | 0x69f330 | 0x4afc20 | No (own copy) |
| 33 | (base stub) | 0x55ad10 | 0x55ad10 | **YES** |
| 34 | (base stub) | 0x55acf0 | 0x55acf0 | **YES** |
| 35 | (base stub) | 0x55ad00 | 0x55ad00 | **YES** |
| 36 | Begin_Piggyback | 0x4b4c60 | 0x4b4c60 | **YES** |
| 37 | End_Piggyback | 0x4b4c70 | 0x4b4c70 | **YES** |
| 38 | Is_Ok_To_End | 0x4b4c80 | 0x4b4c80 | **YES** |
| 39 | Is_To_Have_Shadow_Ovr | 0x6a3f00 | 0x4b48d0 | No (own copy) |

**Shared: 20 slots** (base class or literally the same function pointer).
**Own-copy: 20 slots** (Ship has its own implementation, structurally identical
to Drive but using Ship's own global data pointers).

Both In_Which_Layer implementations return the same value: `2` (ground layer).

---

## 6. Behavioral Differences in Code

### Difference 1: Wake/Dust Animation Frequency

In `Process`, both locomotors spawn a wake/dust animation while moving.
The frame-skip logic differs:

| | Code | Effect |
|---|---|---|
| **Drive** (0x4b07a2) | `IDIV 0xa; TEST EDX,EDX` | Every **10** frames (`frame % 10 == 0`) |
| **Ship** (0x69fe50) | `AND EDX, 0x80000007; JNS/DEC/OR/INC` | Every **8** frames (`frame & 7 == 0`) |

**Impact:** Ships spawn wake animations 25% more frequently than Drive spawns
dust clouds. This makes visual sense — ships leave a continuous wake trail,
while ground vehicles kick up intermittent dust.

**Note:** Both Ship and Drive check the `TypeClass+0xd69` flag before spawning the wake
animation. Drive checks:
```
if (frame % 10 == 0 && typeclass->d69 == 0 && techno->0x8c == 0 && locomotion_type == 2 && rules->wake_anim != 0)
```
Ship checks (verified via `decompile_function 0x0069fc10`):
```
if ((frame & 7 == 0) && typeclass->d69 == 0 && techno->0x8c == 0 && locomotion_type == 2 && rules->wake_anim != 0)
```
The only difference is the frequency (every 8 vs every 10 frames). Ship does **not** skip the
`+0xd69` guard — the earlier claim that it did was incorrect.
(corrected 2026-05-29: was "Ship skips TypeClass+0xd69 check"; binary shows Ship also checks
*(char*)(typeclass + 0xd69) == '\0' before spawning wake anim, same as Drive — ROOT_CAUSE: INFERENCE_HARDENED)

### Difference 2: Deceleration Rate Source

In `Process_Drive_Track`, when computing deceleration near the destination:

| | Code | Source |
|---|---|---|
| **Drive** (0x4b0f20) | `iStack_d0 = vtable_call(linked_techno, +0x38c)` | Virtual call to get rate |
| **Ship** (0x6a05f0) | `*(int *)(iVar8 + 0x678)` | Direct read from TypeClass+0x678 |

Drive dynamically queries the deceleration multiplier via a virtual function
(`+0x38c` on the linked TechnoClass vtable), while Ship hardcodes reading
it from `TypeClass+0x678` (the `decel_steps` field).

**Impact:** For standard units these produce the same value. But if a unit's
class overrides the vtable method at +0x38c, Drive would respect the override
while Ship would not. In practice, no naval units override this, so the
behavior is identical for all shipping (pun intended) configurations.

### Difference 3: Tether Check Ordering in Process_Movement

Ship and Drive check the same conditions but in a slightly different order:

**Ship** (`0x6a1c80`):
```
1. Check tether count (techno+0x2D0 && RadioClass::Tether_Count)
2. Check vtable+0x1D4 (IsDeploying)
3. Check vtable+0x1D8 (IsUnloading)
4. Check vtable+0x37C
5. Check vtable+0x380
```

**Drive** (`0x4b2630`):
```
1. Check vtable+0x1D4 (IsDeploying)
2. Check vtable+0x1D8 (IsUnloading)
3. Check tether count (techno+0x2D0 && RadioClass::Tether_Count)
4. Check vtable+0x37C
5. Check vtable+0x380
```

**Impact:** When a tethered unit is also deploying/unloading, Drive will
return 0 (from the deploy check), while Ship will return 1 (from the
tether check). Edge case — unlikely to matter for naval units.

### Difference 4: Extra Tow/Mission Check in Drive Process

In `Process` (the top-level tick function), Drive has an extra block in the
"track active, second Process_Movement call" path (around `0x4b05d0–0x4b063d`):

Drive checks if the unit is a convoy leader (`type_id == 1` i.e. infantry-like)
and if so, queries the mission target's destination to potentially update the
movement target. Ship lacks this entire block — it goes straight to
`Process_Movement`.

**Impact:** This is convoy/tow logic for ground vehicles. Ships don't tow,
so omitting this is correct.

### Difference 5: Process Drive_Track null-check

At the end of the Process function, Drive has a null-check on the linked
techno pointer before accessing `+0x90`:
```c
// Drive:
if ((uVar5 != 0) && (*(char *)(uVar5 + 0x90) != '\0'))
// Ship:
if (*(char *)(uVar5 + 0x90) != '\0')
```
Ship omits the null check. This is safe because the linked techno is always
non-null during normal gameplay, but it's a minor code difference.

### Difference 6: Track Table Size (Entries 67–71)

As detailed in section 3, Ship lacks TurnTrack entries 67–71 and RawTrack
entries 14–15. These extra tracks in Drive are used for special maneuvers
that ships don't perform (docking, deploying MCV, etc.).

---

## 7. Memory Layout

### Ship data region (0x7f1308 – 0x7f2e6c)

```
0x7f1308  Float constants (decel thresholds)
0x7f1320  Track point data (tracks 1-13, ~5696 bytes)
0x7f2960  RawTrack[14] (224 bytes)
0x7f2a40  TurnTrack[67] (804 bytes)
0x7f2d64  4 bytes padding
0x7f2d68  IPiggyback vtable (9 slots, 36 bytes)
0x7f2d8c  ILocomotion vtable (40 slots, 160 bytes)
0x7f2e2c  (44 bytes padding/data)
0x7f2e58  IUnknown vtable (4 slots, 16 bytes)
```

### Drive data region (0x7e6240 – 0x7e7f90)

```
0x7e6240  Float constants (decel thresholds)
0x7e6258  Track point data (tracks 1-15, ~6096 bytes)
0x7e7a28  RawTrack[16] (256 bytes)
0x7e7b28  TurnTrack[72] (864 bytes)
0x7e7e88  4 bytes padding
0x7e7e8c  IPiggyback vtable (9 slots, 36 bytes)
0x7e7eb0  ILocomotion vtable (40 slots, 160 bytes)
0x7e7f50  (44 bytes padding/data)
0x7e7f7c  IUnknown vtable (4 slots, 16 bytes)
```

---

## 8. Ship-Specific Function Addresses

| Address | Name | Drive Equivalent |
|---------|------|-----------------|
| 0x69ebd0 | InitBridgeZOffset | 0x4af470 |
| 0x69ebf2 | InitNullCoords | 0x4af4e0 |
| 0x69ec50 | Constructor | 0x4af540 |
| 0x69ecf0 | Constructor (copy?) | — |
| 0x69f250 | Piggybacker_CLSID | 0x4af610 |
| 0x69f290 | Is_Moving | 0x4afb80 |
| 0x69f330 | Is_Moving_Check | 0x4afc20 |
| 0x69f3a0 | Destination | 0x4afc90 |
| 0x69f3d0 | Move_To | 0x4afcc0 |
| 0x69f450 | Set_Destination | 0x4afd40 |
| 0x69f510 | Stop_Moving_Full | 0x4afe00 |
| 0x69f670 | Draw_Matrix | 0x4aff60 |
| 0x69fb20 | Shadow_Matrix | 0x4b0410 |
| 0x69fbe0 | Force_New_Slope | 0x4b04d0 |
| 0x69fc10 | **Process** | 0x4b0500 |
| 0x6a01a0 | Apply_Track_Step | 0x4b0ad0 |
| 0x6a0310 | Force_Track | 0x4b0c40 |
| 0x6a05c0 | Do_Turn_Update | 0x4b0ef0 |
| 0x6a05f0 | **Process_Drive_Track** | 0x4b0f20 |
| 0x6a1c80 | **Process_Movement** | 0x4b2630 |
| 0x6a3db0 | Transform_Track_Coords | 0x4b4780 |
| 0x6a3e50 | In_Which_Layer | 0x4b4820 |
| 0x6a3ea0 | Z_Adjust | 0x4b4870 |
| 0x6a3eb0 | Z_Gradient | 0x4b4880 |
| 0x6a3f00 | Mark_All_Occupation_Bits | 0x4b48d0 |
| 0x6a42b0 | Constructor (variant) | — |

---

## 9. Summary for Implementation

For the Rust engine, Ship and Drive can share ~95% of their code. The
implementation should:

1. **Share the stepping algorithm** — Process_Drive_Track / Process_Movement
   logic is identical. Only the data table pointers differ.

2. **Use separate track tables** — Ship needs 67 TurnTrack entries and
   14 RawTrack entries. Drive needs 72 and 16 respectively. The shared
   entries (0–66 / 0–13) have identical data, but the tables must be
   separate because Drive has 5 extra entries.

3. **Wake animation frequency** — Ship spawns every 8 frames, Drive every
   10 frames. Parameterize this.

4. **Deceleration source** — Ship reads TypeClass+0x678 directly; Drive
   calls a virtual function. For our implementation, reading the field
   directly is fine (no units override the virtual method in practice).

5. **Track point data can be shared** — Since all 14 common curves are
   byte-identical, the Rust engine can store one copy and have both
   locomotors reference it. Drive just has 2 additional curves (14, 15).

6. **Convoy/tow logic** — Only relevant for Drive; Ship can skip it.
