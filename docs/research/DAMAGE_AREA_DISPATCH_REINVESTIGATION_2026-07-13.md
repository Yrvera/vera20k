# Damage Area Dispatcher Reinvestigation — 2026-07-13

**Task:** Damage authoritative cutover plan, bounded unit 3A  
**Investigation mode:** exhaustive slice  
**Primary function:** `Apply_area_damage @ 0x00489280`  
**Program:** active Ghidra program `/gamemd.exe`, x86 32-bit, image base `0x00400000`  
**Retail file checked:** `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, SHA-256 `1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`  
**Scope:** collection order, filters, layers, fixed records, target lifetime, receiver arguments, Cartesian world-lepton conversion, distance adjustments, non-HP ordering, all static xrefs, and focused Rust handoff  
**Non-scope:** implementing Rust, running the game/debugger, driving an Oracle, changing another research report, or re-deriving the concrete receiver internals owned by Tasks 1/2

## 1. Verdict

**COMPLETE for Task 3A's dispatcher contract and worked fixtures.**

The native transaction is a synchronous two-phase operation:

1. collect fixed 8-byte records in exact order — airborne spatial candidates first, then `CellSpread` table cells, then each selected `CellClass` linked list head-to-tail;
2. walk those records in insertion order and call each still-eligible target's concrete receiver immediately.

Each record freezes only `{ raw target pointer, signed i32 distance_leptons }`. It does not freeze target health, limbo/alive state, coordinates, armor, owner, or receiver result. An earlier receiver can therefore change a later target before its turn. Standard `ObjectClass::UnInit` does not free that later pointer inline: it clears `Object+0x90` and appends the object to the pending-delete vector, so the outer dispatcher safely reaches the later record and skips it at the alive check. Evidence: live `decompile_function(0x005F65F0)`, `disassemble_bytes(0x005F660C..0x005F6684)`, and pending-delete drain `decompile_function(0x00725C70)`.

The current Rust path is **DRIFT**, not an equivalent implementation. It returns no splash targets for `CellSpread=0`, scans cells before air, explicitly deduplicates, forces a cell-center impact, ignores Z in splash distance, uses exact integer square root, quantizes distance to whole cells, precomputes `u16` damage, and applies damage later in a batch.

One non-blocking heritage fact remains unnamed: `FUN_0048A700` contains an area-damage call but has no code/data xref, caller, vtable binding, or PE export. It is classified dormant/unreferenced for active-YR reachability; whether it originated in TS or an abandoned YR helper is `UNKNOWN`. This does not leave an active dispatcher route unclassified.

## 2. Prior State and Evidence Preflight

The requested output did not exist at preflight. Older reports did exist, so this pass rechecked gaps and contradictions rather than duplicating their prose. The research-index brief for `Apply_area_damage`/`0x00489280` resolved the existing damage, CellSpread, bridge, lifecycle, and Rust touchpoints. The selected local sources were then read directly before live verification:

- `docs/research/DAMAGE_MATH_GHIDRA_REPORT.md`
- `docs/research/APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW_GHIDRA_REPORT.md`
- `docs/research/TARGETDEATH_APPLY_AREA_DAMAGE_LIVE_VECTOR_ITERATION_RESWARM_20260528.md`
- `docs/research/CELLSPREAD_OFFSET_TABLE_DUMP_GHIDRA_REPORT.md`
- `docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md`
- `docs/research/CELL_REFERENCE_POINTS_GHIDRA_REPORT.md`
- `docs/research/OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`
- `docs/research/PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`
- the current Task 1/2 receiver reports dated 2026-07-13

Primary live checks in this pass:

- `decompile_function(0x00489280)` for the complete dispatcher;
- `disassemble_bytes` across `0x00489280..0x00489B0F`, with cold spot-checks at `0x004893E8..0x00489454`, `0x00489555..0x004895B1`, `0x00489700..0x00489767`, and `0x00489A70..0x00489AC4`;
- `decompile_function(0x00412B40)` / `decompile_function(0x004137A0)` for airborne spatial enumeration;
- `decompile_function(0x0041C380)` / `decompile_function(0x004CAC40)` for distance and `Sqrt_Approx`;
- `decompile_function(0x00561910)` plus table-memory checks for the CellSpread initializer;
- `get_xrefs_to(0x00489280)` and caller/callsite decompiles for all 33 references;
- `decompile_function(0x005F65F0)` / `decompile_function(0x00725C70)` for record lifetime;
- direct reads of current Rust files named in Section 12.

No game, debugger, input automation, Cargo command, or Rust write occurred.

## 3. Entry Contract and Early Returns

The effective x86 contract is:

```text
int __fastcall Apply_area_damage(
    CoordStruct* impact,          // ECX: three signed i32 Cartesian world leptons
    i32 base_damage,              // EDX
    ObjectClass* source,          // [EBP+0x08], nullable
    WarheadTypeClass* warhead,    // [EBP+0x0C], nullable
    bool affect_resource,         // [EBP+0x10]
    HouseClass* source_house      // [EBP+0x14], nullable
)
```

Evidence: live entry decompile and assembly `0x00489280..0x00489347`; receiver push sequence `0x00489A97..0x00489AB6`.

The dispatcher returns `1` without collecting when any of these entry conditions holds:

- `base_damage == 0`;
- the scenario flag byte has bit `0x20` set;
- `warhead == null`.

For normal completion it returns the negation of the near-center invulnerability-isolation flag described below. The special warhead pointer equal to `RulesClass+0xFAC` returns `2` after target dispatch and before the late rocker/bridge/overlay-chain/particle tail. The exact public name of `RulesClass+0xFAC` is not required here and is not guessed.

Two radius values are intentionally different:

- fine damage radius: `Math__ftol((float)CellSpread * 256.0f)` signed leptons (`0x004892DD..0x004892EE`);
- airborne spatial-query radius: `Math__ftol((float)CellSpread)`, passed to `FUN_00412B40`, whose helper clamps values below `2` to its minimum bucket radius.

## 4. Exact Ordered Transaction

### 4.1 Phase A — airborne records first

The dispatcher computes terrain ground height at the exact impact XY and enters the airborne query only when:

```text
ground_height(impact.x, impact.y) < impact.z
```

The comparison is strict. Evidence: live dispatcher assembly `0x00489377..0x004893BD`.

It queries the global spatial index with the impact `CellClass` and `ftol(CellSpread)`. `FUN_00412B40` appends the center bucket and then surrounding bucket perimeters; bucket vectors are copied in their own ascending order. `FUN_004137A0` pops the first scratch entry by shifting later entries left. Therefore airborne order is spatial-bucket enumeration order, not `EntityStore` order, cell-list order, stable ID, or distance order.

For each popped pointer, the collector checks:

1. `target+0x90 != 0`;
2. `target+0x74 != 0`;
3. signed health `target+0x6C > 0`;
4. exact 3D impact-to-`target+0x9C` distance `<= max_radius_leptons`.

Evidence: live assembly `0x004893E8..0x00489454` and the following distance/append block through `0x0048955E`.

An accepted airborne target gets a newly allocated 8-byte record. The code recomputes the distance for the stored record rather than reusing the first filter result. This redundant recomputation is part of the native mechanism.

### 4.2 Phase B — choose one object-list layer

The impact cell is selected from signed cell coordinates. If the impact cell lacks structural flag `Cell+0x140 & 0x100`, the chosen object layer is ground. If it has that flag, deck/alternate layer is selected only when:

```text
impact.z > GetGroundHeight(impact) + trunc_toward_zero(DAT_0089E864 / 2)
```

The boundary is strict `>`. Assembly `0x0048955E..0x0048958D` implements signed divide-by-two with `CDQ; SUB; SAR` and then compares impact Z.

Fresh init-site assembly corrects an older cross-document error. At `0x00489101..0x00489120`, the writer loads `H = DAT_0089E870`, computes integer `4*H`, converts it to x87, **adds** `0.5`, calls `Math__ftol`, and stores `DAT_0089E864`. It does not multiply by `0.5`. Thus for integer `H`:

```text
DAT_0089E864 = ftol(4*H + 0.5) = 4*H
selector threshold contribution = DAT_0089E864 / 2 = 2*H
```

The same one-bit ground/deck result is reused for every spread cell. The dispatcher never recomputes the object layer from a neighboring cell's bridge state. Ground uses `Cell+0xE4`; deck uses `Cell+0xE8` (`0x004896BF..0x004896D5`).

### 4.3 Phase C — CellSpread table, then linked lists

The number of cells is:

```text
band  = Math__ftol((double)(float)CellSpread + 0.99)
count = COUNT_TABLE[band]
```

Fresh assembly `0x00489592..0x004895AA` shows `FLD float [warhead+0x124]`, `FADD double [0x007E5160]`, `Math__ftol`, then indexed read from `0x007ED3D0`. This is not simply `ftol(CellSpread)` and is not mathematically identical to `ceil` for every modded float.

The verified count table is:

```text
band:   0  1   2   3   4   5    6    7    8    9    10   11
count:  1  9  21  37  61  89  121  161  205  253  309  369
```

The dispatcher walks the packed signed-i16 offset table at `0x00ABD490` in exact initializer order. `(0,0)` is first. For each cell it resolves a `CellClass` (the map helper returns its dummy cell for invalid coordinates), performs the per-cell non-HP overlay/resource work, then follows the one selected list head-to-tail via `Object+0x30`.

`CellClass::AddContent @ 0x0047E8A0` prepends non-buildings and appends `WhatAmI()==6` buildings to the selected list. Consequently, recent mobile entrants normally precede older occupants while buildings remain at the tail. The dispatcher does not sort this order.

### 4.4 CellSpread zero

`CellSpread=0` is not an empty splash:

- fine radius is `0` leptons;
- band is `ftol(0.99)=0`;
- count is `1`;
- the exact impact cell `(0,0)` is scanned;
- a candidate whose stored distance is exactly `0` passes the inclusive final radius check.

This is load-bearing for direct-hit, force-fire, stacked occupancy, and exact-center mod cases. Current Rust's early `return Vec::new()` is wrong.

### 4.5 No explicit deduplication

There is no target-pointer lookup, set, or sort before record append. A repeated pointer yields repeated records and repeated receiver calls if it remains eligible.

The startup initializer has a verified table defect in band 11:

- index `319` = `(-3, 11)`;
- index `322` = `(-3, 11)` again;
- `(3, -11)` is absent.

Evidence: live `decompile_function(0x00561910)` around writes to `0x00ABD98C`/`0x00ABD998`; current exact Rust transcription also shows the duplicate. Therefore a modded spread that reaches band 11 scans that cell twice, including its pre-list overlay/resource effects and target list. Stock maximum `CellSpread=10` stops at 309 entries, so the defect is dormant in stock content but active for compatible mods.

There is no native bounds clamp on the 12-entry count table. Behavior beyond its valid input domain is not a safe, specified extension. Rust's clamp to band 11 is deterministic but is not native mechanism parity.

## 5. Candidate Filters and Isolation Gate

### 5.1 Per-cell collection filters

In linked-list order, a candidate is appended only after these collection-stage tests:

1. source/self rule: accept if `candidate != source`, or the source type byte at `source_type+0xCA0` is nonzero, or `warhead == RulesClass+0xFAC`;
2. `candidate+0x90 != 0`;
3. if `WhatAmI()==1` and scenario option bit `0x800` is set, compare candidate type against the `RulesClass+0xB40/+0xB4C` vector/count and skip matches.

Fresh assembly `0x0048971C..0x00489759` proves the last rule. `rules.ini:303` / `rulesmd.ini:393` define `HarvesterUnit=HARV,CMIN`, and the verified scenario flag mapping is `HarvesterImmune`. Older prose calling this vector `ProtectedFromAOE` is wrong.

The cell collector does **not** require positive health, `+0x74`, non-limbo state, or in-radius distance yet. Those are dispatch-stage tests. It also performs no house/alliance test; receiver logic owns those semantics.

### 5.2 Near-center invulnerability isolation

The collector sets a transaction-wide flag when an eligible near-center candidate meets all of these conditions:

- `CellSpread <= 0.5`;
- distance `< 85` leptons;
- object flags at `+0x14` include bit 0;
- virtual slot `+0x160` reports active invulnerability;
- candidate field `+0x1C4 == 0`.

The exact semantic name of `+0x1C4` remains `UNKNOWN`; it is not renamed here. Once the flag is set, dispatch skips every record that does not have object flag bit 0 and active `+0x160`. This is a global isolation mode for that detonation, not a per-target damage modifier.

### 5.3 Dispatch-stage filters, in order

For each record, the dispatcher reads the stored raw pointer and checks:

1. `target+0x90 != 0`;
2. if `WhatAmI()==6`, skip when `BuildingType+0x1701 != 0` (`InvisibleInGame`);
3. when isolation mode is active, require object flag bit 0 and active virtual `+0x160`;
4. if `WhatAmI()==2` and virtual `+0x54` is true, halve stored distance with signed truncation toward zero;
5. signed health `target+0x6C > 0`;
6. `target+0x74 != 0`;
7. `target+0x81 == 0` (not limbo);
8. final signed distance `<= max_radius_leptons` (inclusive).

Evidence: live dispatcher decompile and assembly `0x004899D8..0x00489AB6`; the aircraft halving and final gates were cold-checked at `0x00489A70..0x00489A95`.

## 6. Fixed Records, Re-entry, and Lifetime

Each accepted candidate is represented by a separately allocated record:

```text
offset +0: raw ObjectClass* target
offset +4: signed i32 captured_distance_leptons
size:      8 bytes
```

The outer vector stores pointers to these records. All collection finishes before the first receiver call. Records are freed after dispatch.

The record boundary freezes target identity and distance only. Consequences:

- if an earlier receiver moves a later target, the later record keeps its old distance;
- if an earlier receiver changes the later target's health, limbo state, `+0x74`, type-visible state, or invulnerability, the later dispatch sees the new state;
- if an earlier receiver or a nested death weapon kills/uninitializes the later target, standard `ObjectClass::UnInit` calls detach/limbo, writes `+0x90=0`, and queues the same allocation for deferred deletion (`0x005F6616..0x005F667D`); the outer record is still pointer-valid and skips it at test 1;
- nested `Apply_area_damage` calls can run completely before the outer dispatcher resumes. The outer record list is not rebuilt.

`get_xrefs_to(0x00725C70)` places the pending-delete drain in `Main_Tick`, scenario/map initialization, and a bulk cleanup helper, not in the area receiver call chain. The standard damage/removal path therefore does not reclaim a recorded allocation inline.

Rust should use a safe stable/generational handle rather than emulate a raw pointer, but it must preserve the native observation: repeated records remain repeated, captured distance remains frozen, and a now-dead/uninitialized target is skipped at its later turn.

## 7. Cartesian World-Lepton Conversion and Distance

### 7.1 Reference frames

`CoordStruct` is three signed i32 **Cartesian world-lepton** components:

```text
x: west/east world axis in leptons
y: north/south world axis in leptons
z: world height in leptons
256 leptons = 1 map cell along x or y
```

It is not a packed map cell, an isometric screen vector, a pixel vector, a terrain level, or a facing/track index.

The impact cell conversion for each axis is signed truncation toward zero:

```text
cell = (value + ((value >> 31) & 255)) >> 8
```

The result is stored as signed i16 cell coordinates. Evidence: assembly `0x00489309..0x00489330`.

Concrete signed examples:

```text
world lepton   native cell
-257           -1
-256           -1
-255            0
-1              0
 0              0
 255            0
 256            1
```

This differs from arithmetic-floor division for negative subcell coordinates.

Cell centers used by the building/cell branches are `cell*256 + 128` in X/Y, with Z supplied by the cell/ground coordinate helper. Exact non-building and airborne targets use their actual object coordinates, not a cell center.

### 7.2 Native distance primitive

The dispatcher forms signed i32 deltas, promotes them to the distance helper's floating calculation, and computes:

```text
distance = Math__ftol(Sqrt_Approx(dx*dx + dy*dy + dz*dz))
```

`Sqrt_Approx @ 0x004CAC40` converts the magnitude to float32, uses a mantissa lookup table at `0x008650BC`, reconstructs a float result, and returns it for `Math__ftol`. It is not host `sqrt`, `f64::sqrt`, or an exact integer square root.

The stored/final distance unit is a signed i32 lepton count. No division by 256 occurs before the receiver call.

### 7.3 Target-specific reference points

| Target path | Coordinate used | Adjustment |
|---|---|---|
| airborne collection | direct `target+0x9C` CoordStruct | at dispatch, `WhatAmI()==2 && vslot+0x54` halves distance toward zero |
| non-building cell-list target | target virtual `+0xA4` exact CoordStruct | none |
| building in spread offset index `>0` | center CoordStruct of the **currently enumerated cell** | none |
| building in offset index `0` | impact cell center CoordStruct | if `impact.z-center.z <= 2*H`, distance is forced to 0; otherwise compute 3D approximate distance and subtract `2*H`, where `H=DAT_0089E870` |

The building test is on the spread-offset index, not a building foundation index. Building exact object coordinates are not used by these branches. Evidence: live assembly/decompile `0x0048971C..0x0048987F`.

## 8. Worked Fixtures

These fixtures name every frame and keep arithmetic in world leptons until the receiver boundary.

### 8.1 Ground non-building fixture

Inputs:

```text
CellSpread = 1.0
impact world coord = (2624, 5312, 0)
                    = (cell 10, sub_x 64; cell 20, sub_y 192; z 0)
target world coord = (2752, 5312, 0)
                    = (cell 10, sub_x 192; cell 20, sub_y 192; z 0)
```

Native steps:

1. impact cell conversion gives `(10,20)`;
2. band `ftol(1.0+0.99)=1`, so 9 table cells are scanned, center first;
3. target is reached in the selected center-cell list;
4. Cartesian delta is `(-128,0,0)`;
5. squared magnitude is `16384`; `Sqrt_Approx` uses lookup entry 0 and yields float `128`, then `ftol` yields `128` leptons;
6. max radius is `ftol(1.0*256)=256` leptons;
7. record is `{target,128}` and passes `128 <= 256`.

This fixture disproves the old cell-centered description: a center-cell mobile can receive nonzero distance because exact subcell XY is used.

### 8.2 Airborne aircraft fixture

Inputs:

```text
CellSpread = 1.0
ground height at impact XY < 300
impact world coord   = (2624, 5312, 300)
aircraft world coord = (2752, 5312, 300)
aircraft WhatAmI()==2 and vslot+0x54 == true
```

Native steps:

1. strict height gate admits the airborne spatial query;
2. the aircraft is collected before every cell-list target;
3. exact delta `(-128,0,0)` produces stored distance `128` leptons;
4. dispatch halves it with `CDQ; SUB; SAR` to `64` leptons;
5. receiver sees distance `64`, not `128`, and the record remains before all ground records.

### 8.3 Building fixture

Let the initialized per-level lepton step be `H=DAT_0089E870`. Use the center of cell `(10,20)`:

```text
center world XY = (10*256+128, 20*256+128) = (2688, 5248)
cell-center Z = Z0
impact = (2688, 5248, Z0 + 2*H + 256)
CellSpread = 1.0
building is encountered in offset index 0
```

The center branch sees vertical difference greater than `2*H`, computes the 3D approximate distance, then subtracts `2*H`.

For the concrete fixture input `H=104`, delta Z is `464`. The `Sqrt_Approx` lookup entry read at `0x008722FC` reconstructs float `464`, so:

```text
stored distance = 464 - 2*104 = 256 leptons
max radius      = 256 leptons
result          = included at the exact inclusive boundary
```

If the same center building has `impact.z-center.z <= 2*H`, its stored distance is exactly `0`. If a building is instead encountered at noncenter offset `(1,0)` with the same Z, the branch uses that enumerated cell's center and does not apply the center vertical exemption/subtraction.

The `H=104` number above is an explicit fixture input consistent with the project's nominal geometry; this static pass did not read post-init process memory. The symbolic formula is the verified contract.

## 9. Exact Receiver Call

For every accepted record, the dispatcher resets a local signed i32 damage variable to the original `base_damage`, then calls target vtable `+0x16C`:

```text
target->ReceiveDamage(
    &local_damage,          // fresh original incoming i32 for each target
    distance_leptons,       // captured i32, possibly aircraft-halved
    warhead,                // dispatcher argument, unchanged
    source,                 // dispatcher argument, nullable
    false,                  // ignore_defenses
    false,                  // second receiver flag/argument
    source_house            // dispatcher argument, nullable
)
```

Assembly `0x00489A97..0x00489AB6` pushes, right-to-left: source house, `0`, `0`, source, warhead, distance, and `&local_damage`, then calls `[target_vtable+0x16C]`. The receiver return value is ignored. The possibly mutated `local_damage` is not carried to the next target.

The dispatcher sets an `any_receiver_called` byte after the call; the later rocker effect consumes that fact. `affect_resource` does not enter the receiver; it gates the per-cell tiberium/resource path.

## 10. Non-HP Effects and Their Ordering

These effects are inside the synchronous dispatcher transaction but outside the entity receiver arithmetic contract.

### 10.1 Before each cell's object-list capture

For every enumerated spread cell, before reading its selected object-list head:

1. inspect the cell overlay when `Cell+0x44 != -1`;
2. when the overlay/type and warhead gates permit and `affect_resource` is true, call `Reduce_Tiberium(base_damage / 10)` using native signed truncation;
3. when wall/wood/absolute-destroyer gates permit, destroy the overlay, using `-1` for the absolute-destroyer path or base damage otherwise;
4. when overlay destruction clears it, call the global targeting cleanup.

Evidence: live dispatcher `0x004895B2..0x004896BA`. These mutations can change what a later duplicate table entry observes.

### 10.2 After all entity receivers

Normal non-special completion continues in this order:

1. optional `Rocker` impulse scan over a 7x7 cell square, using the same selected layer and `any_receiver_called`-derived magnitude;
2. bridge/wood-bridge damage blocks and their scenario RNG draws/Z windows;
3. impact-cell explodable-overlay removal, map/zone/target refresh, explosion animation, and recursive `Apply_area_damage` with the configured rules warhead;
4. debris/particle-side effects for the overlay chain;
5. warhead `Particle` system creation at the impact coordinate.

The special `RulesClass+0xFAC` return occurs before this late tail. Exact bridge damage RNG/Z-window details remain owned by `APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW_GHIDRA_REPORT.md`; this report verifies their position relative to collection and receivers.

An implementation must not move the pre-cell effects after target damage or move the late tail into a later global world phase.

## 11. Complete Static Xref Inventory

Live `get_xrefs_to(0x00489280)` returned 33 callsites. Legend: **A** = ordinary active YR core route; **C** = conditionally active YR route; **D-TS** = proven dormant TS legacy in standard YR; **D-U** = statically unreferenced/dormant, historical origin unknown.

| Callsite(s) | Class | Route bucket | Owner / gate evidence |
|---|---|---|---|
| `0x00424ED1` | C | excluded special | `AnimClass::Middle`; `TiberiumChainReaction` gate, stock `TWLT070T` activation |
| `0x0048A371` | C | recursive area | dispatcher explodable-overlay tail; direct self-recursion with rules warhead |
| `0x0053A5D0` | C | lightning | `LightningStorm::GroundStrike`; standard Weather Storm strike path |
| `0x0053CDB5`, `0x0053CDD4` | C | excluded special | `Wave_splash_forces`; frame/state-zero base hit plus conditional deck-Z hit |
| `0x0053B16B` | C | excluded special | `PsychicDominator::MindControlArea`; standard YR Psychic Dominator path |
| `0x004387A3` | C | weapon AoE | `BombClass::Detonate`; live Ivan/attached-bomb target and state gates |
| `0x006E04DD`, `0x006E0545`, `0x006E05AD`, `0x006E062F`, `0x006E0697` | C | excluded trigger | `FUN_006E0490`, sole caller `TriggerAction::Execute` case `0x3F`; center plus four `+/-0x55` XY hits; public action label not guessed |
| `0x006E250B` | C | excluded trigger | `FUN_006E2390`, sole caller `TriggerAction::Execute` case `0x2A`; weapon-selected strike with bridge-height adjustment |
| `0x006CD90C` | C | excluded superweapon | `SuperClass::Launch` case 9, Genetic Mutator `MutateExplosion` branch |
| `0x00469A83` | A | projectile / weapon AoE core | `WarheadTypeClass::Detonate`; ordinary projectile/warhead detonation route. Task 3C must separately prove death-weapon producer convergence/timing |
| `0x00425237` | C | weapon AoE | `NukeGroundZero::ApplyDamage`; standard NukeWarhead route |
| `0x00423EAB` | C | excluded special | `AnimClass::AI` bouncer/meteor impact branch with configured expire animation/warhead |
| `0x00424647` | C | excluded special | `AnimClass::AI` per-frame damaging animation branch |
| `0x0048A88B` | D-U | excluded dormant | `FUN_0048A700` square/radius helper; zero code/data refs, zero callers, no vtable binding, and not exported (`list_exports` returns only entrypoint) |
| `0x004A76AF` | C | weapon AoE | `DiskLaserClass::AI` terminal Floating Disc beam; vtable-bound, stock `DISK`/DiskLaser weapons |
| `0x004CD9BB` | C | excluded crash | `FlyLocomotionClass::Process` aircraft ground-crash explosion |
| `0x006632C7` | C | projectile / weapon AoE | `RocketLocomotion::Detonate`; stock V3ROCKET/DMISL/CMISL normal/elite warhead selection |
| `0x004B5D28`, `0x004B5FC7` | D-TS | excluded dormant | `DropPodLocomotion`; COM CLSID exists but stock YR INIs have no locomotor binding/producer; verified TS holdover |
| `0x0051A6C1`, `0x0051A79E`, `0x0051A7D3` | C | excluded C4 | `InfantryClass::PerCellProcess`; mission `0x11`, C4-capable infantry, nav-target/cell gates |
| `0x0071BABF` | C | excluded terrain | `TerrainClass::Take_Damage`; lethal special-destruction branch |
| `0x0074A1E1` | C | excluded voxel impact | `VoxelAnimClass::AI`; terminal land/above-water impact with configured ExpireAnim/warhead |
| `0x00481E33`, `0x00481E89` | C | excluded crate | `CrateClass::PickupDispatch` poison-gas center and eight-neighbor hits |
| `0x0048266D` | C | excluded crate | crate Explosion handler; five randomized-offset rules-warhead hits |
| `0x00482836` | C | excluded crate | crate Napalm handler; direct picker damage plus rules-warhead AoE |

The crate handlers are compiled active mechanisms, but stock `[Powerups]` sets `Gas=0`, `Explosion=0`, and `Napalm=0` in both `rules.ini:22496` and `rulesmd.ini:30345`. They remain conditionally reachable from forced/map-selected crate types and are not TS-dormant.

No radiation callsite directly references `0x00489280`; periodic radiation is a direct-receiver producer owned by Task 3C. Ordinary projectile/warhead impact reaches the dispatcher through `WarheadTypeClass::Detonate`. Whether every death-weapon path converges there, and its exact provenance/tick position, remains for Task 3C rather than being inferred from this xref inventory.

## 12. Focused Current-Rust Disparity Scan

### 12.1 Preserved substrate

- `src/sim/combat/cell_spread.rs:25` has the exact count table.
- `src/sim/combat/cell_spread.rs:34-76` preserves the exact 369 offsets and the band-11 duplicate.
- `src/sim/occupancy.rs` preserves the selected-list ordering class: non-structures prepend, structures append.
- `src/sim/combat/combat_aoe.rs:103-105` chooses one impact-derived layer and reuses it across the spread walk.

These facts are necessary but do not make the dispatcher equivalent.

### 12.2 Verified drifts

| Native contract | Current Rust | Verdict |
|---|---|---|
| `CellSpread=0` scans center cell and can hit distance-zero targets | `combat_aoe.rs:93-95` returns empty | **DRIFT** |
| airborne records precede every cell record in spatial-index order | `combat_aoe.rs:109-136` scans cells first; `:138-157` then scans `EntityStore::values()` fallback | **DRIFT** |
| no dedup; repeated records can call repeatedly | `combat_aoe.rs:106`, `:120` use `BTreeSet` | **DRIFT** |
| exact impact subcell XY/Z | API accepts only cell RX/RY plus coarse impact Z; `:276-285` forces `(128,128)` and ignores Z in distance | **DRIFT** |
| `Sqrt_Approx` then `ftol`, signed i32 leptons | `:296-297` uses exact `isqrt_i64`, divides by 256, creates fixed cell distance | **DRIFT** |
| receiver receives fresh signed i32 base damage plus warhead/distance | `:310-322` precomputes final `u16` damage/Verses/prone result | **DRIFT** |
| air prefilters, height gate, bucket order, aircraft half-distance | absent from fallback path | **DRIFT** |
| source/self, harvester immunity, InvisibleInGame, isolation, exact alive/limbo gates | absent or represented by different coarse gates | **DRIFT** |
| 8-byte identity+distance snapshot; synchronous ordered receivers | Rust returns `(stable_id,u16 damage)` and applies batched `damage_events` at `combat/mod.rs:1849-1885` | **DRIFT** |
| receiver side effects/death can change later records during same dispatch | Rust applies all HP in a later phase and handles deaths at Phase 6 | **DRIFT** |
| pre-cell overlay/resource work before capture; late bridge/wall/tail afterward | world applies bridge/wall/tiberium/terrain events later at `world/mod.rs:2425-2460` | **DRIFT** |
| native count table has no clamp | `cell_spread.rs:96-98` clamps to band 11 | **DRIFT** for out-of-domain modded input |
| selector compares world-lepton impact Z to ground height plus `2*H` | Rust selector comments/inputs operate in terrain level units (`combat_aoe.rs:223-243`) | **DRIFT / frame mismatch** |

`src/sim/occupancy.rs:145-163` registers every structure foundation cell. Whether every one of those entries is native `CellClass::AddContent` membership was not re-proved in this slice; the current AoE `BTreeSet` hides any repeated Rust structure encounter. Task 3S or the implementation contract must not assume the dedup makes this harmless.

## 13. Rust / G2 Implementation Handoff

### 13.1 Required area-dispatch adapter

The Rust-native representation should preserve native semantics with a transient ordered record such as:

```text
AreaTargetRecord {
    target_id_or_generation_handle,
    distance_leptons: i32,
}
```

Required behavior:

1. accept a `WorldLeptonCoord { x_leptons:i32, y_leptons:i32, z_leptons:i32 }`, not only a cell coordinate;
2. compute signed truncation-to-zero cell conversion exactly;
3. collect airborne candidates first in a proven equivalent of native spatial-index order;
4. choose ground/deck once from the impact cell in world-lepton units;
5. walk exact table cells and exact selected-list order without sorting or deduplication;
6. preserve CellSpread zero and the valid-domain table defect;
7. store signed i32 lepton distance using a bit/procedure-equivalent `Sqrt_Approx`/`ftol` path;
8. dispatch records synchronously and sequentially, re-resolving live target state at each turn;
9. call the complete concrete receiver with fresh incoming damage and exact arguments;
10. keep pre-cell and post-receiver non-HP effects at their verified positions.

If Rust lacks a native-order airborne spatial index, substituting stable-ID/BTreeMap order is not acceptable. That is an explicit G2 implementation prerequisite.

### 13.2 `ProjectileImpactDamageCall` field provenance at the dispatcher boundary

| Planned field | Native provenance |
|---|---|
| `target_id` | each fixed record's target pointer, translated to a safe stable/generational handle without dedup |
| `source_object_id` | dispatcher stack argument `[EBP+0x08]`, passed unchanged to every receiver |
| `source_house` | dispatcher stack argument `[EBP+0x14]`, passed unchanged |
| `warhead_id` | dispatcher stack argument `[EBP+0x0C]`, passed unchanged |
| `incoming_damage` | dispatcher EDX argument copied fresh to a local i32 before every receiver |
| `impact_coord` | dispatcher ECX pointer to exact signed Cartesian world-lepton CoordStruct |
| `ignore_defenses` | literal `false` at `0x00489AA8` |

The scheduler owner is not the current late `damage_events` phase. Area resolution is a synchronous callee of the producer's detonation/impact position. Task 3B must name the live Logic-vector projectile scheduler owner; Task 3A proves that once the producer calls the dispatcher, all collection, receiver calls, recursion, and tail effects finish before the producer resumes.

### 13.3 Acceptance fixtures

At minimum, the implementation contract should require executable checks equivalent to:

- `area_cellspread_zero_hits_only_exact_center_distance`
- `area_air_records_precede_table_and_cell_list_records`
- `area_cell_list_preserves_native_head_to_tail_order`
- `area_band11_duplicate_calls_receiver_twice`
- `area_removed_later_record_is_skipped_after_alive_clear`
- `area_moved_later_record_keeps_captured_distance`
- `area_aircraft_distance_halves_toward_zero`
- `area_building_center_vertical_exemption_and_subtraction`
- `area_bridge_layer_boundary_is_strict_world_lepton_gt`
- the three numeric fixtures in Section 8 with raw incoming receiver observations

Rust-vs-Rust tests are regression checks. Final parity still requires retail-derived ordered observations at the collector/receiver boundary.

## 14. Contradiction and Correction Ledger

| Prior/current claim | Live finding | Classification |
|---|---|---|
| area targets are sorted by distance | records remain append order: air, table, linked list | **WRONG** |
| area damage is generally cell-centered | only building branches use cell centers; air/mobiles use exact CoordStruct | **WRONG / misleading** |
| spread band is `ftol(CellSpread)` | cell loop uses `ftol(CellSpread + 0.99)`; air query separately uses `ftol(CellSpread)` | **WRONG** |
| Rules `+0xB40/+0xB4C` is `ProtectedFromAOE` | it is `HarvesterUnit` vector/count under `HarvesterImmune` scenario bit `0x800` | **WRONG** |
| band-11 duplicate should be corrected | exact parity preserves duplicate `(-3,11)` and absent `(3,-11)` | **WRONG recommendation** |
| `FUN_00663030` is Disk Laser | live owner/callers prove `RocketLocomotion::Detonate` for V3/DMISL/CMISL | **WRONG label** |
| `DAT_0089E864 = 2*DAT_0089E870` due multiply by 0.5 | bytes show `FADD 0.5` after `4*H`; result is `4*H`, then selector halves to `2*H` | **WRONG decode** |
| raw records may be freed by an earlier ordinary receiver | `ObjectClass::UnInit` clears alive and queues deferred delete; later record remains valid and skips | **RESOLVED stale uncertainty** |
| Rust has no target dedup | current Rust explicitly uses `BTreeSet` | **STALE** |
| Rust uses a generated/symmetric CellSpread table | current Rust preserves exact table and defect | **STALE** |

## 15. Adversarial Review and Coverage Ledger

### 15.1 Five adversarial questions

1. **Could `CellSpread=0` still be empty because max radius is zero?** No. Center cell is scanned and distance-zero candidates pass the inclusive comparison.
2. **Could an implicit vector/list invariant make dedup harmless?** No general dedup exists, and the verified band-11 duplicate repeats an entire cell. At minimum, compatible modded input can repeat effects and receivers.
3. **Could an earlier lethal receiver invalidate a later raw record?** Standard removal clears alive and defers allocation destruction. The pointer remains valid; later record skips. Nested effects may still change the later target before that check.
4. **Could the dispatcher recompute distance after a target moves?** No. It stores distance during collection and never recomputes it in phase B, except the dispatch-time aircraft divide-by-two of the stored value.
5. **Could each neighboring bridge cell select its own layer?** No. One impact-derived selector byte chooses `+0xE4` or `+0xE8` for every table cell.

### 15.2 Coverage

| Area | Status | Evidence / residual |
|---|---|---|
| entry signature and early returns | VERIFIED | live entry decompile/assembly |
| signed lepton-to-cell conversion | VERIFIED | `0x00489309..0x00489330`, negative fixtures |
| airborne gate/order/prefilters | VERIFIED | dispatcher plus `0x00412B40`/`0x004137A0` |
| layer selector and constant formula | VERIFIED | `0x0048955E..0x0048958D`, `0x00489101..0x00489120` |
| CellSpread zero/count/order | VERIFIED | `0x00489592..0x004895AA`, count/offset init |
| duplicate/no-dedup behavior | VERIFIED | initializer indices 319/322 plus no search/set in dispatcher |
| per-cell overlay/resource ordering | VERIFIED | full dispatcher decompile |
| all collector/dispatch filters | VERIFIED | assembly ranges in Sections 5/9 |
| ground/air/building distances | VERIFIED | distance helpers, dispatcher branches, fixtures |
| fixed record boundary | VERIFIED | 8-byte allocations/appends and phase split |
| standard prior-removal lifetime | VERIFIED | `0x005F65F0`, pending-delete drain `0x00725C70` |
| exact receiver arguments | VERIFIED | `0x00489A97..0x00489AB6` |
| late non-HP relative order | VERIFIED | full dispatcher body; bridge inner detail delegated to prior focused report |
| static xrefs | 33/33 INVENTORIED | live `get_xrefs_to`; one dormant orphan has unknown historical ancestry only |
| current Rust comparison | VERIFIED DRIFT | direct reads listed in Section 12 |
| retail runtime observation | NOT RUN | prohibited by bounded static/read-only scope; required later for parity certification |

The zero-add pass found no additional unresolved in-scope branch, filter, ordering rule, distance branch, or call argument.

## 16. Open Questions — Final State

- `[RESOLVED] OQ-3A-01 — What is the fixed capture boundary?` After all air/table/list collection and before any receiver; each record is target pointer plus i32 lepton distance.
- `[RESOLVED] OQ-3A-02 — What happens at CellSpread zero?` Center cell only, zero radius, exact-distance-zero targets eligible.
- `[RESOLVED] OQ-3A-03 — Is there target dedup?` No; repeated records are preserved.
- `[RESOLVED] OQ-3A-04 — What is exact order?` Air spatial order, then offset-table order, then selected cell list head-to-tail.
- `[RESOLVED] OQ-3A-05 — Is layer per cell?` No; one impact-derived layer is reused.
- `[RESOLVED] OQ-3A-06 — What happens when an earlier receiver removes a later target?` Standard UnInit clears alive and defers deletion; later record pointer remains valid and skips.
- `[RESOLVED] OQ-3A-07 — Does movement/state change alter the record?` State gates are live; target identity/distance stay captured.
- `[RESOLVED] OQ-3A-08 — What reaches the receiver?` Fresh original i32 damage, stored/adjusted i32 lepton distance, unchanged warhead/source/source house, and two false flags including `ignore_defenses=false`.
- `[RESOLVED] OQ-3A-09 — Are non-HP effects outside the transaction?` No; pre-cell and post-receiver effects are synchronous at fixed positions, though they belong to adjacent implementation subsystems.
- `[RESOLVED] OQ-3A-10 — Which xrefs are active?` All 33 are inventoried; ordinary detonation is core active, most others are gated active routes, DropPod is TS-dormant, and `FUN_0048A700` is dormant/unreferenced.
- `[DEFERRED / non-blocking historical label] OQ-3A-11 — Was orphan `FUN_0048A700` originally TS or abandoned YR code?` No static reference/export identifies ancestry. Active reachability is already resolved absent.
- `[DEFERRED / out-of-valid-domain] OQ-3A-12 — What exact crash/read behavior follows a modded band index beyond 11?` Native indexes past the fixed count table with no clamp. This is not a specified safe extension and does not affect stock `CellSpread<=10`.
- `[DEFERRED / runtime oracle] OQ-3A-13 — What ordered retail trace certifies the contract?` Task 4/Oracle must capture raw collector order, distance, receiver arguments, and alive-state skips; static proof is not parity certification.

## 17. Status and Handoff

**Task 3A status: COMPLETE.**

G1 area-dispatch mechanism rows are resolved for static implementation planning. G2 remains open at the broader producer/scheduler boundary until Tasks 3B/3C/3S reconcile projectile, death-weapon, radiation, and lightning timing and name the authoritative scheduler owner. The current late batched Rust damage phase cannot be promoted to authority from this report.

Because this worker's sole authorized output was this report, it did not rebuild the repository research index; the parent reconciliation owner should reindex/validate after all 3A-3C reports exist so concurrent document outputs are incorporated once.
