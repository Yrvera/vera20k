# FootClass::Find_Docking_Bay (0x004DF040) Internals — Ghidra RE Report

Swarm: 2026-07-28T12:25, slot-4. Target: `0x004DF040` and its direct helpers only
(`FUN_004DEE80` @ vtable+0x52C, `FUN_0065ADF0`, `FUN_005F6500`). Non-goals: the
Mission_Harvest state machine itself, the dock-vector layout at TypeClass+0x3E8/+0x3EC/+0x3F8,
and the RepairBay list identity at rules+0x850 (all already settled in
`docs/scans/trace-swarm-20260728/mission-harvest-cadence.md`). Evidence-needed-for-COMPLETE:
decompile+asm of 0x004DF040 and 0x004DEE80, vtable-slot reads, COL walk for vtable ownership,
and resolution of the 0x00A8E7AC global's real role. Stop condition: reached once all of the
above were verified or shown to require tracing outside scope (recorded as open questions
below).

---

## 1. Function identity and vtable ownership

`FootClass__Find_Docking_Bay` at `0x004DF040`, `__thiscall`, body `0x004DF040`-`0x004DF0BE`.
Verified via `decompile_function 0x004DF040` and `disassemble_function 0x004DF040`.

**Vtable ownership (COL walk, independent of any label):** `read_memory 0x007F5C6C` (4 bytes
before the candidate `vtable__UnitClass` base `0x007F5C70`) → COL pointer `0x0080CC68`.
`read_memory 0x0080CC68` (20 bytes) → RTTICompleteObjectLocator with `pTypeDescriptor =
0x00842D80`. `read_memory 0x00842D80` (40 bytes) → type-descriptor name bytes decode to
`.?AVUnitClass@@` (mangled `class UnitClass`). This independently confirms the vtable at
`0x007F5C70` belongs to **UnitClass**, corroborated by `get_xrefs_to 0x007F5C70` → written only
from `UnitClass__Constructor` (`0x0073543A`), `UnitClass__Destructor` (`0x00735794`), and
`UnitClass__Load` (`0x00744521`) — the classic MSVC vtable-assignment sites for that exact class.

**Slot reads**, `read_memory 0x007F6198` (16 bytes, little-endian dwords):
- `+0x528` = `0x004DF040` = `FootClass__Find_Docking_Bay` itself
- `+0x52C` = `0x004DEE80` = the per-type building scanner (target of the internal vcall)
- `+0x530` = `0x004DEE50` = thin wrapper (not decoded this session)

Active in YR: **Yes.** `UnitClass::Mission_Harvest` (`0x0073E5E0`) calls slot `0x528` three
times via `CALL dword ptr [EDX+0x528]` (confirmed in its own disassembly, see §3); this is the
live harvester dock-search path. `AircraftClass__FindBuildingToDock` (`0x0041BBD0`) also calls
`0x004DF040` directly (`get_function_callers 0x004DF040` → one static caller) — out of scope
per this target's boundary (a different `this` object, aircraft).

---

## 2. Find_Docking_Bay body: parameter roles and receiver identity

```c
int __thiscall FootClass__Find_Docking_Bay(int *param_1,int param_2,undefined4 param_3,undefined4 param_4)
{
  iVar1 = param_2;  iVar3 = 0;  iVar4 = 0;  local_4 = -1;
  if (0 < *(int *)(param_2 + 0x10)) {
    do {
      param_2 = -1;
      iVar2 = (**(code **)(*param_1 + 0x52c))(
                  *(undefined4 *)(*(int *)(iVar1 + 4) + iVar4 * 4), param_3, param_4, &param_2);
      if ((iVar2 != 0) &&
         ((((iVar3 == 0 || (param_2 < local_4)) || (local_4 == -1)) ||
          (*(char *)(iVar2 + 0x3d3) != '\0')))) {
        local_4 = param_2;  iVar3 = iVar2;
      }
      iVar4 = iVar4 + 1;
    } while (iVar4 < *(int *)(iVar1 + 0x10));
  }
  return iVar3;
}
```
Verified via `decompile_function 0x004DF040`.

**Vcall receiver identity (critical, asm-only fact):** at the callsite (`0x004DF07E: MOV
ECX,ESI` immediately before `0x004DF080: CALL dword ptr [EAX+0x52c]`, where `ESI` was loaded
from `ECX` at function entry `0x004DF052: MOV ESI,ECX`), the `this` pointer for the `+0x52C`
vcall is **the same FootClass/UnitClass instance passed into Find_Docking_Bay itself** (the
harvester), **not** the per-type entry. The per-type entry (`typePtr`, a `BuildingTypeClass*`)
is passed as the first **explicit** stack argument instead. Verified via
`disassemble_function 0x004DF040` (push order at `0x004DF068`-`0x004DF07E`: `&distOut`,
`param_4`, `param_3`, `typePtr`, then `MOV ECX,ESI`, then `CALL`).

**Parameter semantics** (numbering `param_2`/`param_3`/`param_4` as the task's "1st/2nd/3rd"
explicit args, i.e. excluding `this`):
- **1st (`param_2`)**: pointer to a `DynamicVectorClass`-shaped list: count at `+0x10`, array-of-pointers base at `+0x04`. This is the caller-supplied dock-type vector (already-settled TypeClass+0x3E8/RepairBay layout — out of scope here).
- **2nd (`param_3`)**: passed through unchanged to the `+0x52C` vcall. **Verified NEVER read inside `FUN_004DEE80`** (see §3) — confirmed by both its decompile (no reference to `param_3` anywhere in the body) and its disassembly (no stack-slot fetch corresponding to that argument's position). In every observed callsite it is the literal `0`. Its role could not be determined from any live call path in this call chain — it is dead weight here (Active in YR: **Conditional/No observable effect** — dead on this path; may be consumed by a different caller not traced).
- **3rd (`param_4`)**: the "wide-pass" flag (`0` normal, `1` wide). This is the parameter that flips between the two Mission_Harvest state-2 calls and is `1` for the state-4 RepairBay call. **This is the one parameter that is actually read** inside `FUN_004DEE80`, gating the reservation-list bypass (§3). Active in YR: **Yes**.

**Tie-break flag `*(char *)(iVar2 + 0x3d3)`:** this is `TechnoClass::IsPrimaryFactory` (bool),
confirmed via `get_struct_layout TechnoClass` → offset `979` decimal `= 0x3D3`, field name
`IsPrimaryFactory`. **This corrects the existing doc
`docs/research/chrono-miner-mission-decision/fn-find-docking-bay.md`, which calls this a
"Veteran status byte" (its own §Unverified flags this as unconfirmed/inferred).** It is not
veterancy (`TechnoClass.Veterancy` is a separate `int` field at offset `336`/`0x150`) — it is
the player's right-click "Primary" building designation flag.

**Selection/tie-break logic (asm-verified, `0x004DF08A`-`0x004DF0AF`):** a new per-type
candidate replaces the running best if: no best yet, OR its distance is strictly less than the
current best, OR (redundant sentinel) best distance is still `-1`, OR **the candidate's own
`IsPrimaryFactory` flag is set — this last condition overrides distance unconditionally**. The
check only inspects the *new* candidate's flag, not the *current best's* flag, so a
Primary-designated building accepted early can still be displaced by a later, closer,
non-Primary candidate of a different dock type; only the flag on the type processed *last* is
final. Iteration order is therefore load-bearing whenever more than one dock type in the vector
has Primary-designated instances. **This same two-argument OR-chain (with the identical
`IsPrimaryFactory` override) also gates the inner per-type nearest-instance selection inside
`FUN_004DEE80`** — the override applies at both the intra-type and cross-type levels.

**Return contract:** the function returns a **`BuildingClass*` pointer** (or `0`/NULL if no
eligible building was found across all types) — not a bay index. Confirmed via
`0x004DF0B8: MOV EAX,EBP` (EBP accumulates the best building pointer) and the `return iVar3;`
decompile line.

---

## 3. Per-type nearest-building scanner: `FUN_004DEE80` (vtable+0x52C)

```c
int * __thiscall FUN_004dee80(int *param_1,int param_2,undefined4 param_3,int *param_4,int *param_5)
{
  local_28 = 0;
  iVar3 = HouseClass__CountOwnedInstances(*(undefined4 *)(param_2 + 0xdf8));
  if (iVar3 != 0) {
    local_1c = *(int *)(param_1[0x87] + 0x78);         // this->Owner->BuildingCount
    for (iVar3 = 0; iVar3 < local_1c; iVar3++) {
      piVar1 = *(int **)(*(int *)(param_1[0x87] + 0x6c) + iVar3*4);  // this->Owner->Buildings[i]
      if (piVar1 != 0 && *(char*)((int)piVar1+0x81)==0 && piVar1[0x148]==param_2 &&
          (param_4==(int*)1 || FUN_0065adf0(piVar1 /*this*/, param_1 /*miner*/) != 0)) {
        if (this->GetMission() != 2) {
          // GetCoords(building), GetCell-ish(miner), MapClass::Can_Reach_Zone(...)
          if (!reachable) continue;
        }
        if ((**vtable_param_1)(0xf, piVar1 /* Receive_Radio(0xF, building) */) == 1) {
          iVar4 = FUN_005f6500(this=piVar1 is arg /* squared 2D distance */);
          if (*param_5==-1 || iVar4<*param_5 || piVar1[0x3d3]!=0) { *param_5=iVar4; local_28=piVar1; }
        }
      }
    }
  }
  return local_28;
}
```
(Reconstructed/paraphrased from `decompile_function 0x004DEE80` + `disassemble_function
0x004DEE80` for readability; exact wording verified against both outputs.)

### Eligibility filters, in exact order

1. **Owner-count gate**: `HouseClass__CountOwnedInstances(*(param_2+0xdf8))` — if the house
   owns zero instances of this BuildingType, return `0` immediately without scanning. Called on
   the miner's own house context (`ESI+0x21C` → `+0x5500` sub-object at
   `0x004DEE93`-`0x004DEEA5`).
2. **Iteration scope is single-house, not alliance-scoped**: the building array iterated is
   `*(int*)(param_1[0x87]+0x6c)` with count `*(int*)(param_1[0x87]+0x78)`, where `param_1[0x87]`
   (`TechnoClass+0x21C`) is `this->Owner` (`get_struct_layout TechnoClass` → offset `540 =
   0x21C` = `Owner` pointer). **No alliance/ally house loop or `Is_Enemy`/`Is_Ally` call exists
   anywhere in this function's body** — verified by full decompile+disassembly read; only the
   miner's own house's `BuildingInstances` array is ever touched. Confirmed via
   `decompile_function 0x004DEE80` and `disassemble_function 0x004DEE80`.
3. **Non-null + alive**: `piVar1 != 0 && *(char*)(piVar1+0x81) == 0` (byte at `+0x81`; treated
   as a dead/limbo flag by prior docs, but this exact offset does not appear in the current
   `get_struct_layout BuildingClass`/`TechnoClass` dumps under any named field — **treat the
   specific semantic name as unverified**, only the zero-test behavior is confirmed by asm).
4. **Type match**: `piVar1[0x148] == param_2`, i.e. `BuildingClass+0x520 == typePtr`. Confirmed
   `get_struct_layout BuildingClass` → offset `1312 = 0x520` = `Type` pointer.
5. **Reservation/contact-list check, `FUN_0065ADF0`, bypassed when `param_4==1`**: asm at
   `0x004DEF02`-`0x004DEF13`: `CMP dword ptr [ESP+0x48],0x1; JZ skip; PUSH ESI(miner); MOV
   ECX,EDI(building); CALL FUN_0065ADF0; TEST AL,AL; JZ reject`. **Receiver/argument mapping
   verified from raw asm** (not the ambiguous decompiled variable names, which coincidentally
   both use `param_1`): `ECX` (this, for `FUN_0065ADF0`) = the **candidate building** (`EDI`);
   the pushed explicit argument = the **miner** (`ESI`). `decompile_function 0x0065ADF0` shows
   it scans an array at `building+0xE4` (count `building+0xE8`) and returns true if any slot is
   `0` (free) or equals the miner pointer (already reserved for this miner); false only if every
   slot is occupied by a *different* unit. **This is a per-building docking-reservation/contact
   list check, not a "zone" check** — this corrects `fn-find-docking-bay.md`'s description of
   `FUN_0065ADF0` as a "zone-available check," which conflates it with the separate check in
   step 6.
6. **Zone reachability, gated by `GetMission() != 2`, independent of `param_4`**: asm/decompile
   `0x004DEF19`-`0x004DEFDD`: `GetMission()` (vtable `+0x2c`) is called; if it returns `2` this
   entire block (and hence the `MapClass::Can_Reach_Zone` call at `0x004DEFD1`, static call
   `0x0056D100`) is **skipped outright**, independent of `param_4`. If `GetMission() != 2`, the
   building's and miner's coordinates/zone data are fetched (vtable `+0x48`, `+0x4c`, `+0xbc`,
   `+0x84`) and `MapClass::Can_Reach_Zone` is called; a `false` result rejects the candidate.
   **This gate is entirely separate from the `param_4`-gated reservation check in step 5** —
   the two are independent conditions in the source, not one combined "editor bypass" as
   `fn-find-docking-bay.md` implies. `GetMission()==2`'s symbolic meaning (which top-level
   Mission enum value this is) was not resolved this session — out of scope.
7. **Radio dock-clearance**: `Receive_Radio(0xF, building)` (vtable `+0x278`) must return
   exactly `1`. The dispatch target (BuildingClass's own Receive_Radio handler) was **not**
   decompiled this session — any further eligibility gating (health threshold,
   under-construction/sold state, dock-capacity accounting) that the existing doc
   `FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md` attributes to a direct
   `BuildingClass__CanDock (0x00457CE0)` call **does not appear anywhere in `FUN_004DEE80`'s own
   decompiled body or disassembly** (no `CALL 0x00457CE0` instruction exists in this function).
   If such a check happens, it must be internal to the `Receive_Radio(0xF)` virtual dispatch —
   unverified, out of scope, flagged below as a doc discrepancy.
8. **Distance scoring, `FUN_005F6500`**: `decompile_function 0x005F6500` +
   `disassemble_function 0x005F6500` show it computes **`(Ymine-Ybldg)² + (Xmine-Xbldg)²` only**
   — the Z (height) component is fetched from `GetCoords` but never subtracted, squared, or
   added into the result (confirmed by both the decompile's return expression and the asm, which
   never re-touches the stored Z dword after `MOV dword ptr [ESP+0xc],EDX`). **Distance metric
   is 2D (ground-plane X/Y only), squared, in raw lepton² units — no lepton→cell conversion is
   applied** (no shift/divide before the multiply). No explicit health/damage or
   under-construction filter exists in `FUN_004DEE80` or `FUN_005F6500` themselves.

Active in YR for all of the above: **Yes** — `FootClass::Find_Docking_Bay` fires on every
harvester return-to-refinery decision (`UnitClass__Mission_Harvest` state 2, confirmed live via
existing settled trace-swarm doc) and every idle-repair-bay lookup (state 4).

---

## 4. The scan-override global `0x00A8E7AC` ("g_MapEditorMode" label)

**Where it is actually touched relative to Find_Docking_Bay's callsites** — full
`disassemble_function 0x0073E5E0` (`UnitClass::Mission_Harvest`) read:

- State 2, **first** call (`0x0073EB47`: `PUSH 0,0; ADD EAX,0x3e8; CALL [EDX+0x528]`) — args
  `(vector, 0, 0)` — **no** touch of `0x00A8E7AC` around it.
- State 2, **second** call (`0x0073EB7C`) — args `(vector, 0, 0)` again — **no** touch of
  `0x00A8E7AC` around it either.
- State 2, **third/wide-pass** call (`0x0073EC1F`-`0x0073EC52`) — args `(vector, 0, 1)` —
  **is** bracketed: `MOV EDX,[0x00A8E7AC]; PUSH 1; INC EDX; PUSH 0; MOV [0x00A8E7AC],EDX; ...
  PUSH vector; CALL [EDX+0x528] ... MOV ECX,[0x00A8E7AC]; DEC ECX; MOV [0x00A8E7AC],ECX`.
- State 4 (`0x0073EEB0`-`0x0073EEC6`): args `(RulesClass+0x850, 0, 1)` — **same `param_4=1`
  value as the wide-pass call, but this callsite does NOT touch `0x00A8E7AC` at all.**

This asymmetry (the global brackets only ONE of the two `param_4==1` callsites) is itself
evidence that the global's effect is not a property of `param_4==1` in general.

**Verified: the global is never read inside the target or its direct helpers.** Full
`decompile_function`/`disassemble_function` passes over `0x004DF040` (Find_Docking_Bay),
`0x004DEE80` (the vcall target), and `0x0065ADF0` (the reservation check) show **zero**
references to address `0x00A8E7AC` in any of the three. Its effect, if any, on this call chain
is entirely indirect.

**Verified downstream consumer**: `decompile_function 0x00501540` (`HouseClass__Is_Enemy`) —
`if (g_MapEditorMode != 0) { return true; }` (reads the same address; independently confirmed
by this session, not just taken from the prior doc). While this global is nonzero, **every
house is treated as a mutual enemy of every other house**, overriding the normal
alliance/team-array computation.

**What the label "g_MapEditorMode" actually gates — verified vs. inferred:**
- Verified: forces `HouseClass::Is_Enemy` to unconditionally return `true`.
- Verified: `get_xrefs_to 0x00A8E7AC` shows dozens of READ/WRITE sites scattered across
  unrelated systems — `HouseClass__MakeAlly`, `HouseClass__BreakAlliance`,
  `HouseClass__ComputerTakeover`, `FactoryClass__AbandonProduction`,
  `ScenarioClass__Read_Scenario`, `RandomMapGenerator__Generate`/
  `InitMapFromSyntheticINI`, `MapClass__Resize`, `CrateSlot__ValidateCellAndCreateOverlay`,
  `CellClass__DestroyOverlay`, `ObjectClass__Reveal`, plus several un-named `FUN_*` sites — none
  of which are docking-bay-specific or "map editor" specific in any obvious sense.
- **The "g_MapEditorMode" label is very likely drift**, per the task's own warning: nothing in
  the verified behavior (forcing global mutual-enmity) or in the breadth of unrelated callers
  supports a literal "in the map editor" semantic; it reads far more like a general-purpose
  "suppress/override alliance and relationship checks during a bracketed nested operation"
  counter that happens to also be (re)used by an actual map/scenario editor code path among many
  others. **This session did not decompile the other ~15 consumer functions**, so a definitive
  replacement name is not proposed — only that the current name should not be trusted at face
  value. This matches the task's own framing of "known drift."
- **Practical effect on Find_Docking_Bay's wide-pass call is unconfirmed/likely inert on the
  common path**: since `FUN_004DEE80`'s own building scan never calls `Is_Enemy` or any
  alliance-aware function directly (§3, filter 2), and since neither `Receive_Radio(0xF)`'s
  target nor `MapClass::Can_Reach_Zone`'s body were decompiled this session, it cannot be ruled
  out that one of those (out-of-scope) callees consults `Is_Enemy`. But nothing inside
  `0x004DF040`'s direct call graph, as verified, reads or reacts to this global — the bracket in
  `Mission_Harvest` around only the wide-pass call looks like **defensive bookkeeping carried
  over from a shared code path, not a mechanism that changes Find_Docking_Bay's own selection
  logic**. Active in YR: **Conditional** — the increment/decrement executes on every wide-pass
  dock search (state 2, when the reserved/first-pass search fails), but its only verified
  effect (`Is_Enemy` override) is not demonstrably reachable from this call chain.

---

## 5. Iteration order (both levels)

- **Outer (Find_Docking_Bay)**: over the caller-supplied dock-type vector in stored array
  order, index `0..count`, one vcall per type entry. No re-sorting.
- **Inner (`FUN_004DEE80`)**: over `this->Owner->BuildingInstances[]` in stored array order,
  index `0..count`, filtered to the current type; tracks the nearest instance passing all
  filters for that type only (with the same `IsPrimaryFactory` override as the outer loop, see
  §2).

Both orders are simple linear scans in storage order — no spatial pre-sort, no
distance-bucketing.

---

## 6. Struct fields touched (byte offsets, direct pointer arithmetic)

| Object | Offset | Field | Evidence |
|---|---|---|---|
| UnitClass/FootClass (`this`) | `+0x000` | vtable ptr | dispatch of `+0x52c` |
| TechnoClass (`this`) | `+0x21C` | `Owner` (HouseClass*) | `get_struct_layout TechnoClass` offset 540; used at `0x004DEE93`/`0x004DEEB2` |
| HouseClass | `+0x6C` | BuildingInstances array ptr | asm `0x004DEED4` |
| HouseClass | `+0x78` | BuildingInstances count | asm `0x004DEEBA` |
| BuildingTypeClass (`typePtr`) | `+0xDF8` | arg to `HouseClass__CountOwnedInstances` | asm `0x004DEE8B` |
| BuildingClass | `+0x081` | zero-tested liveness byte (name unverified) | asm `0x004DEEE2` |
| BuildingClass | `+0x520` | `Type` (BuildingTypeClass*) | `get_struct_layout BuildingClass` offset 1312 |
| TechnoClass | `+0x3D3` | `IsPrimaryFactory` (bool) | `get_struct_layout TechnoClass` offset 979 |
| BuildingClass (candidate) | `+0xE4`/`+0xE8` | reservation/contact-list array ptr / count | `decompile_function 0x0065ADF0` |
| Global | `0x00A8E7AC` | scan-override counter (label "g_MapEditorMode", drift-suspect) | asm `0x0073EC1F`-`0x0073EC52`; `decompile_function 0x00501540` |

---

## 7. Implementation Handoff

- **Verified behavior → Rust delta**: `FUN_004DEE80` only scans the miner's **own house's**
  `BuildingInstances` (no ally/alliance loop, verified §3 filter 2) → the parent's stated
  current Rust shape (`src/sim/miner/miner_system.rs::find_nearest_refinery`, "alliance + health
  + building_up filters") applies an alliance filter and a health filter that **do not exist in
  gamemd's `Find_Docking_Bay`/`FUN_004DEE80` at all**. **Affected surface**: `miner_system.rs`
  refinery-selection logic. **Acceptance scenario**: two allied houses each with their own
  refinery; a harvester belonging to house A must never select house B's (allied) refinery,
  even if closer/less-damaged — verify by placing an allied refinery closer than the owner's own
  refinery and confirming the miner still returns to its own house's refinery. **Proposed test
  name**: `miner_docking_never_selects_allied_house_refinery`. **Risk**: medium — this is a
  behavioral divergence from retail that is currently silently masked in single-house test
  setups; multiplayer/allied scenarios would visibly differ.
- **Verified behavior → Rust delta**: the tie-break/override at candidate-building offset
  `+0x3D3` is `IsPrimaryFactory`, not veterancy or health-based, and it overrides distance
  **unconditionally and only when checked on the newly-considered candidate** (not "sticky" once
  a Primary is picked — a later, closer, non-Primary type can still displace it). **Affected
  surface**: `find_nearest_refinery`'s tie-break rule (currently "2D cell distance ... strict-less
  tie-break" per parent context, with no Primary-designation concept). **Acceptance scenario**:
  player right-clicks "Primary" on a farther refinery of dock-type B while dock-type A's closer
  refinery is not designated Primary, and type B is processed **after** type A in the dock list —
  the harvester should select the farther Primary refinery. **Proposed test name**:
  `miner_docking_prefers_primary_designated_refinery_regardless_of_distance`. **Risk**: low
  currently (feature likely simply absent in Rust) but changes player-visible destination choice
  once "Primary" building designation is implemented.
- **Verified behavior → Rust delta**: distance metric is 2D squared (X/Y only, Z/height ignored)
  in raw lepton² units, computed independently at two levels (intra-type nearest, then cross-type
  best-of-bests) — both using the same squared-2D metric, never converted to cells or made 3D.
  **Affected surface**: any refinery/repair-bay distance comparison in the miner docking path.
  **Acceptance scenario**: two same-type refineries at equal X/Y-cell distance but different
  elevation (Z) must be treated as tied by the selection logic. **Proposed test name**:
  `miner_docking_distance_ignores_height_delta`. **Risk**: low (elevation differences between
  candidate refineries are rare on typical maps) but a straightforward, cheap parity fix.

---

## 8. Negative Facts / Do Not Do

- Do **not** treat `FUN_0065ADF0` as a "zone reachability" check — it is a per-building
  reservation/contact-list check (`building+0xE4`/`+0xE8`, testing for a free slot or a slot
  already held by the calling miner). Evidence: `decompile_function 0x0065ADF0` +
  `disassemble_function 0x004DEE80` receiver mapping (`0x004DEF09`-`0x004DEF0C`). The actual
  zone-reachability check is the separate, `GetMission()!=2`-gated `MapClass::Can_Reach_Zone`
  call later in the same function — do not conflate the two gates or assume both are controlled
  by the same parameter.
- Do **not** attribute a direct `BuildingClass__CanDock (0x00457CE0)` call to `FUN_004DEE80` —
  no such `CALL 0x00457CE0` instruction exists anywhere in its disassembly (verified this
  session). If `CanDock`-style filtering (state/sold/capacity) exists on this path, it is inside
  the un-decoded `Receive_Radio(0xF)` dispatch target, not in this function.
- Do **not** label BuildingClass/TechnoClass offset `+0x3D3` as "Veteran status" — it is
  `TechnoClass::IsPrimaryFactory` (`get_struct_layout TechnoClass` offset 979). `Veterancy` is a
  separate `int` field at offset `336`/`0x150`.
- Do **not** assume `param_3` (the always-`0` second explicit argument) does anything on this
  call path — it is read nowhere inside `FUN_004DEE80`, confirmed by decompile and
  disassembly. Do not invent a "editor mode = param_3" semantic (an earlier draft of this
  investigation's own working notes made exactly this mistake before checking the asm).
- Do **not** assume the `0x00A8E7AC` global changes `Find_Docking_Bay`'s own selection logic —
  it is read in none of `0x004DF040`, `0x004DEE80`, or `0x0065ADF0`. Any effect is indirect via
  `HouseClass::Is_Enemy` and is not demonstrated to be reachable from this call chain.

---

## 9. Remaining Uncertainty

- Exact semantic name/enum meaning of `GetMission() == 2` (the value that skips the zone-reachability
  block) was not resolved — would require decoding the Mission enum, out of scope for this target.
- Whether `Receive_Radio(0xF, building)`'s target function (BuildingClass's own radio handler,
  not decoded this session) performs additional filtering (health threshold, under-construction/sold
  state, dock-capacity/free-bay accounting) that the existing
  `FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md` attributes (likely incorrectly, at the wrong
  call depth) to a direct `BuildingClass__CanDock` call.
- Exact name/semantics of `BuildingClass`/liveness byte at `+0x081` — the zero-test behavior is
  confirmed, the specific field name is not (does not appear in the current `BuildingClass`/
  `TechnoClass` struct dumps).
- The true intended semantic of global `0x00A8E7AC` beyond "forces `Is_Enemy`==true" — would
  require decompiling its ~15+ other consumer functions (scenario load, random map generation,
  alliance/diplomacy, crate/overlay code), which is well outside this target's scope.
- Whether `param_3` has any effect in a different, unobserved call context (e.g. if some other
  caller passes a non-zero value) — no such call site was found; only the three Mission_Harvest
  sites and the one AircraftClass site (itself out of scope) were checked.
- `vtable+0x530` (`FUN_004DEE50`) was identified as a sibling slot (thin wrapper per the prior
  doc) but not decompiled this session — out of scope, not needed to answer the assigned
  questions.

---

## 10. Stale-doc replacement wording

- `docs/research/chrono-miner-mission-decision/fn-find-docking-bay.md`, §Struct field
  accesses/BuildingClass table, row `*(char *)(iVar2 + 0x3d3)`: replace "Veteran status byte" /
  "Non-zero = Veteran; triggers dock preference override" with "`TechnoClass::IsPrimaryFactory`
  (bool, offset 0x3D3 per `get_struct_layout TechnoClass`); non-zero = player-designated Primary
  building for this type; triggers unconditional dock-preference override regardless of
  distance." Also its §Behavioral analysis / §Unverified should drop the "Veteran" framing
  entirely — it is fully resolved, not a YELLOW item.
- `docs/research/chrono-miner-mission-decision/fn-find-docking-bay.md`, step 4 ("Zone
  reachability: unless `param_4 == 1`... calls `FUN_0065adf0`... and `MapClass::Can_Reach_Zone`
  to confirm... zone can reach... If `GetMission() == 2`, skips the zone check"): replace with
  two separate bullet points — (a) `FUN_0065ADF0`, bypassed when `param_4==1`, is the
  per-building **reservation/contact-list** check, not zone-related; (b) the
  `MapClass::Can_Reach_Zone` zone-reachability check is a **separate**, always-attempted step
  gated only by `GetMission() != 2`, unrelated to `param_4`.
- `docs/research/miner/FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md`, §4 point 5 and §6
  ("CanDock check — calls `BuildingClass__CanDock` (0x457CE0)... Address: 0x00457CE0. Called by
  evaluator regardless of arg3"): no `CALL 0x00457CE0` exists inside `FUN_004DEE80`
  (`0x004DEE80`) per this session's disassembly — downgrade this claim to "unverified; not
  present in the evaluator itself; if it happens at all it must be inside the un-decoded
  `Receive_Radio(0xF)` dispatch target" rather than presenting it as a directly-observed,
  ordered filter step of the evaluator.
