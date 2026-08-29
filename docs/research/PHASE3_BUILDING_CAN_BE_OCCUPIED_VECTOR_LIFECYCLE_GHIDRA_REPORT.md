# Phase 3 Building `CanBeOccupied` Occupant-Vector Lifecycle — Active-Retail Ghidra Report

**Addresses:** `BuildingClass` constructor `0x0043B340`, destructor `0x0043BF50`, `Update @ 0x0043F9A0`, `ReceiveDamage @ 0x00442230`, `ChangeOwner @ 0x00448260`, `PointerExpired @ 0x0044E8F0`, `Load @ 0x00453E20`, `Save @ 0x00454190`, `Save_ChecksumFields @ 0x00454260`, `GetWeapon @ 0x004526F0`, `SellBuilding @ 0x00457DE0`, `GetOccupantCount @ 0x004581F0`, `CheckAutoSellOrCivilian @ 0x00458200`, `SpawnUnitsWithParachute @ 0x004585C0`; `InfantryClass::Can_Enter_Cell @ 0x0051BF90`, `AddGarrisonOccupant @ 0x00522910`, `Unlimbo @ 0x0051DFF0`, `Scatter @ 0x0051D0D0`; `TechnoClass::ReceiveDamage @ 0x00701900`, death-weapon helper `0x0070D690`, `RecordKill @ 0x00702D40`; `TemporalClass::Update @ 0x0071A760`

**Investigation mode:** exhaustive-slice, live read-only active-retail `gamemd.exe` plus mounted retail rules/art/maps and direct current-Rust inspection

**Claimed scope:** the distinct Building-owned `CanBeOccupied` Infantry pointer vector, every active lifecycle producer and consumer needed to represent it independently of inherited `TechnoClass` Cargo, and the fatal Type-`Explodes` ordering that makes the distinction load-bearing

**Non-scope:** implementing Rust, changing Ghidra metadata, reimplementing the whole generic Cargo/absorber survivor mechanism, or changing unrelated Infantry pathfinding outside the exact `SellBuilding` predicate call

**Verdict:** **COMPLETE — native requirement is implementation-ready; current Rust is not parity-closed.** Native Buildings always contain a separate ordered `CanBeOccupied` Infantry vector at `Building+0x684..+0x698` and a separate fire cursor at `+0x69C`. Inherited Cargo remains at `Techno+0x114/+0x118`. The two stores can coexist and no fatal, sell, unload, save/load, pointer-expiry, or destruction consumer is permitted to reinterpret one as the other.

The fatal contradiction is resolved. An occupied Type-`Explodes` `YAPPPT` does **not** lose its garrison vector in generic explosive Cargo purge. `TechnoClass::ReceiveDamage` drains only inherited Cargo, then detonates the death weapon. After the parent returns, `BuildingClass::ReceiveDamage` calls `SellBuilding(0,0)`, which releases each garrison occupant at one selected perimeter cell or UnInits it on placement/no-exit failure, then calls `BuildingClass::DestructionEffects`. Thus an open-edge garrison occupant survives the parent death weapon and exits afterward; a blocked/no-exit occupant is removed by the later Building wrapper with no attacker/source kill attribution.

The player-visible severity is high when triggered but the `YAPPPT` fatal case is absent from the shipped effective map state. The mounted data contains 4,394 effective `CanBeOccupied` placements across 117 of 184 extracted maps, so ordinary garrison storage/fire/sell/destruction is common. Canonical `YAPPPT` is the only base-rules `CanBeOccupied && Explodes` type, but its sole shipped preplacement (`sov01umd`, cell `39,69`) is map-overridden to `CanBeOccupied=no`; `all01umd` likewise disables occupancy. `all01umd` can route `YAPPPT` through its campaign BasePlan despite `TechLevel=-1`, but the same override still makes any resulting instance non-occupiable. No effective shipped placement combines `CanBeOccupied` with `Explodes`, `Passengers>0`, `UnitAbsorb`, or `InfantryAbsorb`. The occupied explosive case is therefore synthetic/custom-map/save reachable, not an ordinary shipped-map event, while dual-store representation remains mandatory binary architecture.

## 1. Evidence basis and zero-add method

The live Ghidra MCP session was connected to project `testProsjekt`, program `/gamemd.exe`. All inspection was read-only. Fresh whole-program instruction censuses completed without truncation:

| displacement | matches / instructions scanned | classified occupant-vector owners |
|---|---:|---|
| `+0x684` | 79 / 1,163,336 | constructor/destructor vector object, plus unrelated class fields removed by function/class context |
| `+0x688` | 50 / 1,163,336 | occupant item pointer in load/save/fire/sell/add/kill/UI consumers |
| `+0x694` | 69 / 1,163,336 | occupant count in the same consumers |
| `+0x69C` | 48 / 1,163,336 | Building garrison fire cursor in constructor/fire/sell |

After classifying every raw match, a second zero-add pass searched the complete program again for each four-byte displacement and every discovered helper/caller. It added no unclassified active Building occupant-vector reader or writer. All addresses below were then decompiled fresh; branch-sensitive claims were checked against assembly.

Mounted content authority:

- `target/asset/extract/rulesmd.ini`, SHA-256 `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF`;
- `target/asset/extract/extract/artmd.ini`, SHA-256 `E1F0378394313C04EBBD5073F47785EE3E46F1B3C62D65724E8F3C310EE7BA31`;
- 184 extracted `.map` files under `target/phase3-retail-census/extract`.

## 2. The two owners are structurally distinct

### 2.1 Building occupant vector

The first occupant-vector field is an embedded `DynamicVectorClass<InfantryClass*>`:

| Building offset | native meaning | evidence |
|---|---|---|
| `+0x684` | vector vtable | constructor `0x0043B6F1`; destructor `0x0043C04B` |
| `+0x688` | ordered `InfantryClass**` items | append `0x00522941..0x00522992`; save/load and all consumers below |
| `+0x68C` | allocated capacity | vector resize/clear calls in Sell/destructor |
| `+0x690/+0x691` | DynamicVector allocation metadata | constructor/destructor |
| `+0x694` | signed occupant count | `GetOccupantCount @ 0x004581F0` returns it directly |
| `+0x698` | growth step | constructor initialization |
| `+0x69C` | 32-bit current garrison fire index | weapon/muzzle/fire/sell accesses |

`BuildingClass` construction initializes this vector and cursor independently of base-class construction. `BuildingClass` destruction clears/frees this vector tail independently after `TechnoClass` destruction has handled its own state.

### 2.2 Inherited Cargo

Inherited Cargo is the `TechnoClass` store:

| Techno offset | native meaning |
|---|---|
| `+0x114` | signed Cargo count / Cargo base |
| `+0x118` | Cargo linked-list head |
| passenger `+0x30` | next Cargo link |

`CargoClass::AddPassenger @ 0x004733A0` Limbos and head-inserts; `0x00473430` pops the head. Nothing in the occupant-vector append path writes these fields. Nothing in `SellBuilding` reads them. Conversely, explosive purge and `SpawnSurvivors` Cargo work do not read `+0x688/+0x694/+0x69C`.

This is not merely a rules distinction. The object layout and constructor/destructor make simultaneous nonempty stores representable. Stock data happens not to author a dual-capability placed type, but native code contains no union or exclusivity invariant.

## 3. Construction, entry, and owner reconciliation

### 3.1 Construction and capability

Every native Building constructs an empty occupant vector; `CanBeOccupied` gates admission and consumers rather than memory existence. `MaxNumberOccupants` is not vector allocation capacity. The vector grows through its DynamicVector append operation.

### 3.2 Entry gate

`BuildingClass::CanDock @ 0x00457CE0` admits an Infantry garrison entry only when all applicable gates pass:

1. candidate pointer is non-null;
2. `BuildingType+0x157B CanBeOccupied` is true;
3. current raw mission is neither `0x12` nor `0x13`;
4. Building coordinates are in the playfield;
5. the Building virtual at `+0x1D4` returns false;
6. for `Occupier=yes`, Infantry owner equals Building owner **or** the Building owner's country is `MultiplayPassive`;
7. vector count is **not equal** to `MaxNumberOccupants` — native does not use `<`;
8. Building Health is not in the red condition;
9. Building is not mind-controlled.

The alternate `Assaulter=yes` branch requires an enemy and nonempty vector. Retail census finds no active `Assaulter=yes` Infantry type in base rules or map overrides, so this branch is evidence-backed inactive in mounted content.

Base rules contain 65 InfantryTypes. Exactly `E1`, `E2`, and `INIT` have active `Occupier=yes`; these are live/common infantry types.

### 3.3 Append transaction

`InfantryClass::PerCellProcess @ 0x00519630` calls the dock predicate and then `InfantryClass::AddGarrisonOccupant @ 0x00522910`.

For the Occupier path, `AddGarrisonOccupant`:

- calls Infantry Limbo through vtable `+0xD4`;
- appends the Infantry pointer at `items[count]`, preserving admission order;
- increments `Building+0x694`;
- recomputes threat;
- for the first occupant, queues Building mission raw `2` and emits the native sound/EVA side effects;
- when the occupant owner House's raw human-control byte `+0x1EC` is true, clears Infantry bytes `+0x690/+0x691`.

It does **not** touch inherited Cargo, assign the Building owner, assign the occupant owner, or install `Foot+0x5D4` Team/parent state. Existing Team state can therefore survive entry.

### 3.4 Civilian/neutral owner reconciliation

`BuildingClass::Update` calls `CheckAutoSellOrCivilian @ 0x00458200` unconditionally at `0x004401AF`. That helper only applies when signed `BuildingType+0x634 TechLevel == -1`:

1. if red Health, call `SellBuilding(0,0)` first;
2. if vector count is zero and owner is not the Civilian House, call `ChangeOwner(Civilian,0)`;
3. re-read count;
4. if count is positive and current owner is Civilian, call `ChangeOwner(items[0]->owner,0)`.

`AddGarrisonOccupant` itself never changes the Building owner. The transfer occurs at the next invocation of this Building Update helper; whether that is later in the same global frame or in the following frame depends on live object-order scheduling. `BuildingClass::ChangeOwner @ 0x00448260` does not traverse or mutate the vector and never changes occupant owners. First-vector-element ownership remains the authority.

## 4. Read-side consumers while occupied

| consumer | exact vector behavior |
|---|---|
| `GetOccupantCount @ 0x004581F0` | returns `Building+0x694` |
| `GetWeapon @ 0x004526F0` | when the occupied-building predicate admits and `count > fire_index`, reads `items[fire_index]`, selects that Infantry's veterancy-qualified `OccupyWeapon`; otherwise falls back to Building weapon |
| muzzle/fire-coordinate helpers | `0x00453840` and `0x00453A70` use `MuzzleFlash[fire_index]` when `CanBeOccupied && count>0` |
| successful fire | `TechnoClass::FireAt/SpawnsBullet` advances `Building+0x69C` modulo current count at `0x006FF065..0x006FF085` |
| kill credit | `TechnoClass::RecordKill @ 0x00702D40`, `0x00702F98..0x00702FF0`, redirects veterancy to `items[current fire_index]` for an occupied Building |
| pips | `DrawPipScalePips @ 0x00709A90`, access `0x00709C84`, reads vector count/items |

The `RecordKill` redirect deliberately reads the current cursor at callback time. It does not receive or preserve a per-shot shooter ID and performs no fresh count/bounds/liveness validation in that branch. A projectile that kills later can therefore credit whichever vector element the cursor selects then, including the post-shot/after-subsequent-shots cursor. Rust must not “correct” this by remembering the actual occupant that fired.

No active `PenetratesBunker` removal exists for this vector. That flag operates on the separate bunker/shelter occupant at `Techno+0x2E4`, not `CanBeOccupied` entries. Do not compact the Building vector on generic Infantry damage merely because a similarly named bunker mechanic exists.

## 5. Fatal Type-`Explodes` ordering and the `YAPPPT` correction

### 5.1 Parent Techno stage

`BuildingClass::ReceiveDamage @ 0x00442230` calls `TechnoClass::ReceiveDamage @ 0x00701900` at `0x00442425`. Inside the admitted fatal Explodes/Suicide path, the only passenger loop is:

- test inherited Cargo head `Techno+0x118` at `0x00702603`;
- pop through `0x00473430` at `0x00702614..0x0070263C`;
- remove Team membership where applicable, perform the fatal helper/source callback, and UnInit;
- continue until Cargo head is null at `0x0070265F..0x00702667`;
- call death-weapon helper `0x0070D690` at `0x00702669..0x0070266D`.

`0x0070D690` resolves explicit/current/default death weapon, creates the bullet, and detonates it at the dying Techno coordinate. The parent contains no occupant-vector displacement access. Vector occupants are already Limboed and absent from ordinary Logic/occupancy, so the death-weapon AoE cannot hit them as world targets.

### 5.2 Building wrapper stage

After the Techno parent returns, the Building wrapper performs its docked/contact cleanup, then:

- reads `BuildingType+0x157B CanBeOccupied` at `0x00442625..0x00442633`;
- calls `SellBuilding(0,0)` at `0x00442635..0x0044263B`;
- performs the light cleanup;
- calls virtual `+0x4EC` at `0x00442665`, bound here to `BuildingClass::DestructionEffects @ 0x004415F0`.

Exact order:

```text
Techno fatal processing
  -> admitted inherited-Cargo purge
  -> death-weapon detonation
  -> return to Building wrapper
  -> Building dock/contact cleanup
  -> CanBeOccupied SellBuilding(0,0) vector release/removal
  -> BuildingClass::DestructionEffects
```

`DestructionEffects` later performs its Health-zero/destruction duration/survivor work. At entry, `SellBuilding` has already emptied the occupant vector; inherited Cargo follows its own Explodes or SpawnSurvivors lifecycle.

### 5.3 Fatal `YAPPPT` outcome

For a synthetic/custom/save `YAPPPT` with vector occupants:

- inherited Cargo, if independently populated, is destroyed before the death weapon;
- vector occupants remain intact and Limboed through the death weapon;
- `SellBuilding(0,0)` then processes the vector in reverse order;
- an accepted exit plus successful `Unlimbo` releases an occupant alive, retaining owner and Health;
- no accepted exit invokes `SpawnUnitsWithParachute(0)`, which reverse-UnInits every vector occupant;
- an accepted exit followed by an individual `Unlimbo` failure UnInits only that occupant;
- neither removal branch calls `RecordKill` or passes the fatal attacker/source.

This explicitly supersedes the claim in `PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md` that Explodes “annihilates” `YAPPPT` garrison occupants. That report followed the inherited Cargo loop correctly but assigned the `CanBeOccupied` vector to the wrong owner. `PHASE3_BUILDING_SPAWN_SURVIVORS_CARGO_GHIDRA_REPORT.md` correctly identifies the separate vector and is the authoritative older separation evidence.

## 6. `SellBuilding` ABI, caller matrix, and exact vector release

### 6.1 Two hidden explicit flags

`BuildingClass::SellBuilding @ 0x00457DE0` ends with `RET 0x8`; after its prologue, formal flag 1 is `[ESP+0x40]` and flag 2 is `[ESP+0x44]`.

| caller | effective flags `(flag1, flag2)` | active meaning |
|---|---:|---|
| fatal `BuildingClass::ReceiveDamage` | `(0,0)` | no Team/Hunt postlude; no-edge kill-all |
| red-Health `CheckAutoSellOrCivilian` | `(0,0)` | same |
| Building mission-unload `0x0044D880`, call `0x0044D89C` | `(0,0)` | same, then separate Cargo handling |
| TriggerAction case `111`, call `0x006DF779..0x006DF77F` | `(0,0)` | same |
| ordinary player Sell, `0x0044A5C4` | `(0,1)` | no Team/Hunt; no-edge inside-foundation fallback |
| `HouseClass::All-To-Hunt @ 0x00501400`, call `0x00501503..0x00501505` | `(1,0)` | Team removal and queued Hunt; no-edge kill-all |

Flag 1 gates only the successful-occupant Team removal plus mission `0x0F` postlude. Flag 2 selects the no-cell fallback: false calls `SpawnUnitsWithParachute(0)`; true uses the inside-foundation cell.

### 6.2 Cursor reset and first-occupant scan authority

The function writes `Building+0x69C = 0` before testing count. If count is zero it returns.

For a nonempty vector, every perimeter candidate is validated by **only vector element zero**:

```text
items[0]->InfantryClass::Can_Enter_Cell(
    MapClass::Get_CellClass(candidate),
    direction = -1,
    path_height = -1,
    parent/current = 0,
    arg5 = 1
) == 0
```

The vtable binding is proven live: Infantry primary vtable `0x007EB058`, slot `+0x1AC` at `0x007EB204`, points to `InfantryClass::Can_Enter_Cell @ 0x0051BF90`. The call pushes `1,0,-1,-1` before resolving the Cell. No scan RNG is called.

Return `0` is the only accepted result. The full native Infantry classifier remains active for this tuple: bridge/tube and layer selection, playfield, overlay/wall, land/speed passability, object-list ownership/motion/building/weapon policy, and functional Infantry subcell occupancy can all make the result nonzero. Return codes `1..7` are all rejection here. Fresh decompile closes the formerly deferred terminal subcell rule: ground/deck occupation bits `0x1C` mean all functional subcells `2,3,4` are occupied; when no earlier soft result exists, that produces hard `7`. Current Rust's `check_terrain`-only stand-in is not this predicate.

The exact tuple also means implementation must preserve signed CellStruct/map lookup behavior. Native does not saturate, clamp, convert to unsigned, prefilter negative candidates, or sort/deduplicate candidate cells before `MapClass::Get_CellClass`.

### 6.3 Stable perimeter order

For Building origin `(ox,oy)` and foundation width/height `(W,H)`, native probes in this exact order until the first return-zero cell:

1. east column: `(ox+W, oy+H)` down through `(ox+W, oy-1)`;
2. south row: `(ox+W, oy+H)` west through `(ox-1, oy+H)`;
3. north row: `(ox, oy-1)` east through `(ox+W, oy-1)`;
4. west column: `(ox-1, oy)` south through `(ox-1, oy+H)`.

SE, NE, and SW can be probed twice. NW `(ox-1,oy-1)` is never probed. One accepted Cell is converted once through its Cell coordinate virtual and reused for every occupant. There is no distance selection, randomization, per-occupant edge retry, or candidate deduplication.

### 6.4 No accepted cell

- flag 2 false: `SpawnUnitsWithParachute(0)` reverse-UnInits every occupant and clears the vector; no placement or Scatter RNG is consumed;
- flag 2 true: choose inside-foundation `(ox+W-1, oy+H-1)` and continue through normal placement.

Despite the helper name, the null branch creates no parachute Anim. Its non-null Assaulter branch does create one Anim per occupant and removes them, but mounted retail has no Assaulter type and that branch is excluded.

### 6.5 One-coordinate reverse placement

After choosing/falling back to a cell, native obtains the cell-center coordinate, increments `g_MapEditorMode`, and iterates `items[count-1]` down to `items[0]`:

1. call occupant `Unlimbo(chosen_coord,0)`;
2. on failure, call occupant vtable `+0xF8` (`UnInit`) and continue;
3. on success, conditionally clear Infantry bytes `+0x690/+0x691` for the raw human-owner gate;
4. clear archive target through occupant vtable `+0x3C8(0)`;
5. call occupant Scatter virtual `+0x174(building_center,true,true)`;
6. only if flag 1 is true: remove from `Foot+0x5D4` Team when non-null, then queue mission raw `0x0F` with argument zero.

After all occupants, native decrements `g_MapEditorMode`, clears/resizes the entire vector, and recomputes threat. Owner and Health are never overwritten on a successful occupant. A failed occupant receives no attacker/source `RecordKill` call; normal UnInit owns its cleanup/loss bookkeeping and Team unlink.

### 6.6 Exact placement and RNG correction

The chosen Cell virtual supplies its center coordinate. `InfantryClass::Unlimbo @ 0x0051DFF0` calls `CellClass::PlaceInfantryInCell @ 0x00481180` on the ground-height path. Because the coordinate is cell-centered, requested subcell resolves to zero and every attempted occupant calls shared Scenario `RandomRanged(0,3)` once, then scans the selected rotated row for free functional subcells `2..4`. The `g_MapEditorMode` increment supplies the priority override for bit-`0x20/0x40` rejection; it does not create more than the three functional subcells.

Therefore, with one selected cell and more than three occupants, later occupants can consume their placement draw and still fail/UnInit after the earlier successful occupants fill subcells. This corrects older garrison reports that said ejection placement consumed no RNG. What those reports correctly excluded was a raw `%8` ejection-direction draw.

After successful Unlimbo, `InfantryClass::Scatter @ 0x0051D0D0` can independently consume `RandomRanged(0,4)` after its mission/state/locomotor/type-table gates. If it finds a destination, it queues mission `2` before setting the destination. If flag 1 is true, SellBuilding then removes Team membership and queues mission `0x0F` after Scatter returns. Exact successful order is:

```text
Unlimbo -> placement RandomRanged(0,3)
-> clear tracking bytes if gated
-> clear archive target
-> Scatter(building center,true,true)
   -> optional RandomRanged(0,4)
   -> optional queue mission 2
   -> optional destination set
-> optional flag1 Team remove
-> optional flag1 queue mission 0x0F
```

Failed Unlimbo consumes no **Scatter** RNG and receives no `0x0F`; its preceding placement draw is retained.

## 7. Other active exit/removal paths

| path | vector outcome | Cargo relationship |
|---|---|---|
| player Sell | `SellBuilding(0,1)` vector release; inside fallback | later player-sell Cargo/survivor work is separate |
| red-Health auto civilian helper | `SellBuilding(0,0)`, then empty-vector owner reconciliation | none |
| fatal Building wrapper | `SellBuilding(0,0)` after death weapon, before DestructionEffects | admitted Cargo already purged independently |
| mission Unload `0x0044D880` | vector `SellBuilding(0,0)` first | then independently unloads inherited Cargo for slave/absorber types |
| TriggerAction 111 | `SellBuilding(0,0)` | none |
| All-To-Hunt | `SellBuilding(1,0)` | none |
| Temporal erase | `TemporalClass::Update @ 0x0071A760` calls `SpawnUnitsWithParachute(0)` when Building vector count is positive, before inherited Cargo cleanup and Building UnInit | Cargo cleanup follows separately |
| Assaulter entry | non-null `SpawnUnitsWithParachute(attacker)` Anim/removal branch | inactive in mounted retail because no Assaulter type |

TriggerAction 111 occurs five times in mounted retail data: four actions in `all01umd`, one in `sov03umd`. Direct Trigger Action 6 (`All-To-Hunt`) occurs 22 times across eight maps: `all02umd` 3, `all04dmd` 3, `all05umd` 2, `all06umd` 2, `all07smd` 4, `sov02smd` 2, `sov03umd` 2, `sov05umd` 4. These actions are active content, but the garrison arm is conditional on a qualifying owned Building being occupied at execution time.

## 8. Pointer expiry, Limbo, UnInit, and destruction

`BuildingClass::PointerExpired @ 0x0044E8F0` first chains `TechnoClass::PointerExpired`, which owns Cargo/reference cleanup. It then removes a matching occupant pointer from the Building vector **only when `g_MapEditorMode != 0`**. Removal compacts the vector and decrements count; it does not normalize `+0x69C`.

This conditional is active in `SellBuilding`: failed occupant UnInit occurs inside the incremented scope, so pointer-expiry notification removes the element while reverse iteration remains safe. Ordinary pointer expiry outside that scope does not compact the vector. Normal active gameplay keeps vector occupants Limboed and reaches one of the explicit vector drains; do not add a generic “remove every expired ID from every occupant vector” policy.

`BuildingClass::Limbo @ 0x00445880` does not traverse or drain the occupant vector. `BuildingClass` destructor `0x0043BF50` clears/resizes/frees vector storage without calling occupant UnInit. Correct active call paths therefore drain at Sell/Unload/Temporal time before Building UnInit. Rust's generic carrier-UnInit recursion must remain Cargo-only; applying it to this vector double-kills occupants already released and invents kills on native raw-clear paths.

## 9. Save, load, and checksum

`AbstractClass::Save @ 0x00410320` writes the receiver pointer and raw receiver block using the class virtual size, so scalar `Building+0x69C` persists in native saves.

`BuildingClass::Save @ 0x00454190`, after base `TechnoClass::Save`, explicitly writes:

- occupant count from `+0x694` at `0x004541FA`;
- ordered pointer slots from `+0x688` at `0x00454221` and following.

`BuildingClass::Load @ 0x00453E20` reads the count at `0x004540AD`, grows/appends slots in stream order, and registers every slot with the SwizzleManager through `0x00454101`. Cargo is saved/loaded through the base mechanism, not this vector stream.

`BuildingClass::Save_ChecksumFields @ 0x00454260` calls the Techno checksum and hashes its documented scalar fields but does not directly hash `+0x684/+0x688/+0x694/+0x69C`. That native omission is not permission for Rust to omit future-deterministic state: project `world_hash` is deliberately stricter than native direct checksum and must include occupant order/count and fire cursor independently of Cargo.

## 10. Retail census and activation verdict

### 10.1 Base rules

- 164 BuildingTypes have active `CanBeOccupied=yes`.
- All 164 also have active `CanOccupyFire=yes`.
- None has `MaxNumberOccupants=0`.
- Max distribution: `1` (1 type), `3` (3), `4` (3), `5` (15), `6` (7), `8` (9), `10` (126).
- Foundation distribution: `1x1` 11, `1x2` 2, `2x1` 3, `2x2` 23, `2x3` 12, `2x5` 2, `2x6` 1, `3x2` 11, `3x3` 41, `3x4` 6, `3x5` 2, `4x2` 4, `4x3` 4, `4x4` 30, `5x3` 2, `5x4` 1, `6x4` 9.
- No `CanBeOccupied` type has missing/zero foundation.
- Sole base-rules `CanBeOccupied && Explodes`: `YAPPPT` (`MaxNumberOccupants=10`, `Foundation=3x3`).
- Base-rules intersections with `Passengers>0`, `InfantryAbsorb=yes`, or `UnitAbsorb=yes`: zero.
- `YAPOWR` has `Passengers=5` and `InfantryAbsorb=yes`; its `CanBeOccupied=yes` line is commented and inactive.

### 10.2 Placed retail state

- 11,992 placed Structures across 184 maps.
- 4,489 placements use a base-rules `CanBeOccupied` type.
- After per-map rule overrides: 4,394 effective `CanBeOccupied` placements on 117 maps, 155 distinct effective placed types.
- Average on maps that contain one: 37.56; min 1; max 227.
- frequent types include `CAPARS06` 132, `CANEWY12` 109, `CANEWY21` 101, `CANEWY20` 101, `CAGAS01` 91.
- effective placed `CanBeOccupied` intersections with `Explodes`, `Passengers>0`, or either absorber flag: zero.

### 10.3 `YAPPPT` reachability

Base `YAPPPT` sets `TechLevel=-1`, `Explodes=yes`, `CanBeOccupied=yes`, `MaxNumberOccupants=10`, `CanOccupyFire=yes`, `DeathWeapon=BlimpBombEffect`, `DeathWeaponDamageModifier=.01`, `Foundation=3x3`.

Mounted map findings:

- `sov01umd` preplaces one `YAPPPT` for `YuriCountry` at `39,69`, Health byte 256, but its map section sets `CanBeOccupied=no; CanOccupyFire=no`.
- `all01umd` sets `CanBeOccupied=no; MaxNumberOccupants=0; CanOccupyFire=no`. It contains a `Confederation` BasePlan node `YAPPPT,46,69`, at the same site as a placed `YAPPET`. Campaign BasePlan wildcard selection returns the first unsatisfied node without generic TechLevel-lower-bound rejection, so `TechLevel=-1` alone does not prove this scenario-production route dead. Regardless of whether the node is currently satisfied/blocked or later produced, the map override makes the resulting `YAPPPT` non-occupiable.

Therefore canonical `YAPPPT` is not available through ordinary player/sidebar build eligibility, can participate in active campaign BasePlan state, and occurs as one shipped preplacement, but **no shipped effective instance can enter the occupant-vector fatal case**. Mods, synthetic tests, or a map/save that retains base `CanBeOccupied=yes` activate it.

## 11. Current Rust mismatch inventory

### 11.1 One store cannot represent native state

Current `src/sim/passenger.rs:37..219` defines one `PassengerCargo` containing passenger IDs, sizes/capacity, and the incorrectly colocated `garrison_fire_index`, wrapped by one `PassengerRole::Transport`. `GameEntity` has only `passenger_role` at `src/sim/game_entity.rs:889`.

All three spawn paths are mutually exclusive:

- `src/sim/world/world_spawn.rs:253..266`;
- `:505..515`;
- `:664..674`.

They use `if Passengers > 0 { Cargo } else if CanBeOccupied { same Cargo }`. A dual-capability Building cannot hold both native stores.

### 11.2 Vector consumers currently routed to Cargo

The following must move to the new occupant owner:

- `CanBeOccupied` command admission, boarding arrival, empty/full checks, first-occupant owner reconciliation, and unload in `src/sim/passenger.rs`;
- garrison fire gate, passive targeting, current occupant/veterancy/OccupyWeapon selection, muzzle index, cursor advance in `src/sim/combat/combat_fire_gate.rs` and `src/sim/combat/mod.rs`;
- kill-credit redirect, which is currently absent because `award_kill_experience` pays the Building attacker ID;
- player sell, red-Health release, fatal release, mission unload, Action 111, All-To-Hunt, temporal erase;
- SHP occupied frames/anims in `src/app/presentation/instances/shp.rs`;
- garrison pips in `src/app/presentation/ui_overlays.rs`;
- cursor/context-order/dispatch unload and occupied checks in `src/app/input/{cursor,context_order,dispatch}.rs`;
- snapshot reciprocal references and world hash.

### 11.3 Cargo-only consumers that must not move

Keep these on inherited Cargo:

- ordinary transport/aircraft/paradrop/drop payload and unload;
- absorber admission and capture-fate facility state;
- `YAPOWR` ExtraPower occupant count;
- open-topped/capture-manager Cargo relations;
- generic admitted fatal Cargo purge;
- `SpawnSurvivors` absorber Cargo release;
- generic carrier pointer expiry and recursive UnInit;
- bunker Cargo-like paths whose native evidence points to Cargo rather than `CanBeOccupied` vector.

### 11.4 Fatal and sell mismatches

`src/sim/world/mod.rs:1990..2045` currently treats `CanBeOccupied || InfantryAbsorb || UnitAbsorb` as one family, takes the one Cargo object, and ejects it in `BeforeDeathEffects`; otherwise it purges the same Cargo. This runs before Rust death effects and cannot express native Cargo-before-death-weapon plus vector-after-death-weapon.

`src/sim/production/production_sell.rs:641..663` detaches that same Cargo for fatal release. It also:

- uses a documented terrain-only stand-in at `:287..323`, not full `InfantryClass::Can_Enter_Cell`;
- uses unsigned/saturating cell construction rather than native signed lookup;
- overrides CanBeOccupied occupant owner with Building owner on destruction, which native never does;
- supplies unconditional `PlacementEvidence::MarkSucceeded`, preventing real per-occupant Unlimbo failure;
- returns false after failed reveal without native UnInit;
- writes Health zero before UnInit on no-exit failure, which is not the native Sell helper transaction;
- approximates Scatter with an immediate move and unconditional draw instead of native gates, `RandomRanged(0,4)`, mission 2, destination, and optional later mission `0x0F`;
- has no flag-1 All-To-Hunt Team/Hunt mode.

`src/sim/world/lifecycle.rs:2110..2143` recursively UnInits the one carried store; `:2353..2357` generically disembarks every expired passenger from it. Both need Cargo/vector separation and the vector's priority-scoped expiry rule.

### 11.5 Serialization/hash

Current snapshot version is `121` (`src/sim/snapshot.rs:355`) with strict version rejection. Cargo validation is at `:968` and reciprocal role validation at `:1153`/`:1189`. `world_hash.rs` hashes Cargo capacity/order/sizes/total size but not current `garrison_fire_index`.

## 12. Minimal Rust representation and migration

Use a distinct serialized owner. The smallest native-shaped form is:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildingOccupants {
    pub occupant_ids: Vec<u64>, // native append order
    pub fire_index: u32,        // native Building+0x69C width
}

pub struct GameEntity {
    pub passenger_role: PassengerRole,       // inherited Cargo / passenger backlink
    pub building_occupants: BuildingOccupants,
}
```

An `Option<BuildingOccupants>` is acceptable only if every live Structure receives `Some`, because native embeds the vector in every Building, not only types currently marked `CanBeOccupied`. An unconditional default-empty field is smaller in behavioral complexity. Capacity is derived from the effective type's `MaxNumberOccupants`; do not copy Cargo size/capacity/total-size fields into the vector.

Initialize Cargo and Building occupants independently; remove every `else if`. Continue using a passenger-side `PassengerRole::Inside { transport_id }` concealment/backlink if desired, but snapshot validation must accept that backlink only when the carrier contains the passenger in exactly one corresponding native owner. Do not interpret it as native `Foot+0x5D4` Team state.

Snapshot policy is straightforward: bump `121 -> 122` and continue strict rejection of old snapshots, matching the existing project policy. There is no lossless general migration for a synthetic old snapshot whose one store came from a dual-capability type. If an importer is later required, it may type-route nonintersection stock states, but it must reject ambiguous dual-capability old state rather than guess.

Hash `occupant_ids` length/order and `fire_index` independently of Cargo. Validate every occupant reference, uniqueness within the vector, Building category, effective capacity invariant for produced well-formed state, and reciprocal passenger backlink. Preserve native equality admission (`count != Max`) at the behavioral gate rather than silently changing it to a representation assertion.

## 13. Implementation handoff and acceptance tests

### 13.1 Required implementation effects

1. Add the separate serialized/hash-covered Building occupant owner and initialize it independently of Cargo.
2. Route every `CanBeOccupied` producer/consumer in sections 3, 4, 6, 7, 8, and presentation/input through it.
3. Keep every Cargo-only consumer in section 11.3 on `PassengerRole` Cargo.
4. Split fatal ordering at the native seam: admitted Cargo purge, death weapon, vector `SellBuilding(0,0)`, DestructionEffects.
5. Implement one SellBuilding vector operation with both flags and exact caller modes.
6. Bind its candidate scan to the complete Infantry `Can_Enter_Cell(cell,-1,-1,0,1)==0` service; do not retain the terrain-only stand-in.
7. Use real Infantry Unlimbo/placement under a balanced priority scope, including one `RandomRanged(0,3)` per cell-centered attempt and immediate live subcell feedback.
8. Preserve successful owner/Health/Team/tracking/target/mission effects and failure source-less UnInit.
9. Implement current-cursor kill-credit redirect rather than remembered-shot attribution.
10. Add priority-scoped vector pointer-expiry cleanup and keep generic expiry/UnInit Cargo-only.

### 13.2 Acceptance matrix

All tests must exercise the real owner and real lifecycle services, not a parallel pure expected-value helper.

1. `building_constructor_initializes_cargo_and_occupants_independently`.
2. `dual_store_building_can_hold_cargo_and_ordered_garrison_simultaneously`.
3. `dual_store_sellbuilding_never_drains_inherited_cargo`.
4. `dual_store_explosive_purge_never_drains_building_occupants`.
5. synthetic `YAPPPT`: Cargo is empty before death weapon; death-weapon observer sees vector unchanged; open-edge occupants are released only after the weapon; DestructionEffects sees vector empty.
6. synthetic `YAPPPT` no-edge: Cargo dies before weapon; vector reverse-UnInits after weapon with no placement/Scatter RNG and no killer credit.
7. synthetic `YAPPPT` accepted edge plus four occupants: four `RandomRanged(0,3)` placement calls occur in reverse order; three can occupy functional subcells and the failed fourth UnInits without Scatter or source credit.
8. `YAPOWR` Type-Explodes fixture purges inherited Cargo only and never enters vector release.
9. vector capacity gate uses `count != Max`, including an explicit corrupt/changed-rule `count > Max` characterization.
10. append order, first-occupant owner authority, and first-occupant CanEnter probe remain stable.
11. owner does not change during AddGarrison; next CheckAuto invocation changes Civilian Building to first occupant owner.
12. red Health calls vector release first, then reverts empty TechLevel-`-1` Building to Civilian.
13. Building ChangeOwner never changes occupant owners/order.
14. GetWeapon/muzzle use current vector cursor and occupant `OccupyWeapon`/veterancy.
15. successful shot advances cursor modulo count.
16. delayed kill credits vector element at **callback-time current cursor**, not actual saved shooter or Building.
17. pips and occupied SHP frames read vector, not Cargo; dual-store Cargo count cannot change them.
18. exact east/south/north/west perimeter sequence preserves duplicates and skipped NW.
19. signed off-map candidates reach native map lookup semantics without unsigned saturation/prefilter.
20. candidate return `0` accepts; each nonzero CanEnter code rejects.
21. first occupant alone selects one coordinate; all reverse occupants reuse it.
22. player Sell `(0,1)` uses inside-foundation fallback and leaves Team/owner unchanged.
23. fatal, red-Health, mission unload, and Action 111 `(0,0)` use no-edge kill-all.
24. All-To-Hunt `(1,0)` performs Scatter first, then Team removal and mission `0x0F`.
25. successful release clears archive target; Scatter queues mission `2` before destination; flag-1 mission `0x0F` follows.
26. Scatter pre-RNG table/state gate consumes no `0..4` draw while successful placement's earlier `0..3` draw remains committed.
27. failed Unlimbo UnInits only that occupant, keeps owner/source rules, and continues reverse loop.
28. ordinary pointer expiry does not compact vector; Sell-scoped failed-Unlimbo expiry does; fire cursor is not normalized by that removal.
29. Building Limbo/destructor do not recursively UnInit vector occupants; explicit active drain paths run first.
30. Temporal erase reverse-UnInits vector occupants before separate Cargo/Building cleanup.
31. save/load round-trip preserves Cargo independently, vector order, and 32-bit fire cursor.
32. vector order or cursor changes world hash; Cargo-only changes remain independent.
33. snapshot rejects missing/non-Infantry/duplicate vector IDs and accepts a correct reciprocal inside backlink.
34. retail census regression: zero effective shipped placed vector+Explodes/Cargo/absorber intersections; sole `sov01umd` YAPPPT override stays non-occupiable; `all01umd` BasePlan YAPPPT override stays non-occupiable.
35. existing transport, absorber, YAPOWR ExtraPower, paradrop, and SpawnSurvivors Cargo tests remain unchanged and pass.

## 14. Coverage ledger

| required area | status | evidence |
|---|---|---|
| construction/init | VERIFIED | `0x0043B6F1..0x0043B709`, base Techno constructor separation |
| storage/access census | VERIFIED, zero-add | complete displacement scans listed in section 1 |
| entry/append | VERIFIED | `0x00457CE0`, `0x00519630`, `0x00522910` |
| owner change/reconcile | VERIFIED | `0x004401AF`, `0x00458200`, `0x00448260` |
| firing/targeting/muzzle/cursor | VERIFIED | `0x004526F0`, `0x00453840`, `0x00453A70`, `0x006FF065..85` |
| kill credit and pips | VERIFIED | `0x00702F98..FF0`, `0x00709C84` |
| generic Explodes purge | VERIFIED Cargo-only | `0x00702603..67` |
| death-weapon order | VERIFIED | `0x00702669`, `0x0070D690` |
| Building fatal wrapper order | VERIFIED | `0x00442425`, `0x00442625..65` |
| Sell ABI/caller census | VERIFIED | `RET 0x8`, all six caller families in section 6.1 |
| exact scan/order/predicate | VERIFIED | `0x00457E35..0x00458060`, fresh `0x0051BF90` full decompile |
| placement/failure/RNG | VERIFIED | `0x00458060..0x0045819E`, `0x0051DFF0`, `0x00481180` |
| Scatter/mission/Team order | VERIFIED | `0x004580E9..0x00458138`, `0x0051D0D0` |
| unload/Action111/AllToHunt/Temporal | VERIFIED | `0x0044D880`, `0x006DF779`, `0x00501400`, `0x0071A760` |
| pointer expiry | VERIFIED | `0x0044E8F0` |
| Limbo/UnInit/destructor | VERIFIED | `0x00445880`, `0x0043BF50` |
| save/load/checksum | VERIFIED | `0x00453E20`, `0x00454190`, `0x00454260`, `0x00410320` |
| retail data/type/map census | VERIFIED | mounted rules/art/184-map corpus |
| current Rust owner/consumer census | VERIFIED | direct source scan summarized in section 11 |
| minimal representation/migration | CLOSED | section 12 |

## 15. Adversarial and cold spot checks

1. **Could `+0x688/+0x694` be Cargo under a second view?** No. Constructor/destructor initialize a separate DynamicVector after base Techno state; Cargo uses `+0x114/+0x118`; save/load serializes both independently.
2. **Could the parent Explodes loop reach vector occupants indirectly through passenger backlinks?** No. Assembly walks only the Cargo head link and count. Vector occupants have no Cargo next/head membership from AddGarrison.
3. **Could the death weapon kill Limboed vector occupants before SellBuilding?** No. They are removed from mapped Logic/occupancy by entry Limbo; parent has no direct vector dispatch.
4. **Could `SellBuilding` choose another edge after a later occupant fails?** No. Cell conversion precedes the reverse loop and every Unlimbo receives the same coordinate.
5. **Could no-edge mean parachute survival?** Not for null argument. `SpawnUnitsWithParachute(0)` reverse-UnInits and clears; Anim creation is confined to the non-null Assaulter branch.
6. **Cold check — save/load:** re-decompile confirmed vector count/items are explicit after base save/load and swizzled in stored order; cursor persists in raw object bytes.
7. **Cold check — fatal wrapper:** re-decompile confirmed parent call, later CanBeOccupied test/SellBuilding, then virtual DestructionEffects, with no vector access inside parent.

## 16. Open questions — final state

- `[RESOLVED]` Are `CanBeOccupied` occupants inherited Cargo? **No; separate Building DynamicVector.**
- `[RESOLVED]` Can both stores coexist? **Yes; independent embedded owners and no exclusivity in native initialization.**
- `[RESOLVED]` Does Type Explodes purge the vector? **No; Cargo only.**
- `[RESOLVED]` Do `YAPPPT` vector occupants survive the death weapon? **Yes while Limboed; later SellBuilding releases or UnInits them.**
- `[RESOLVED]` Is exit placement terrain-only? **No; full Infantry CanEnter tuple, return zero only.**
- `[RESOLVED]` Does successful ejection placement consume RNG? **Yes, cell-centered Infantry placement calls Scenario `RandomRanged(0,3)` per attempted occupant; Scatter may later call `0..4`.**
- `[RESOLVED]` Does destruction transfer occupant ownership? **No.**
- `[RESOLVED]` When is Team removed/mission `0x0F` queued? **Only flag-1 All-To-Hunt mode after Scatter.**
- `[RESOLVED]` Is canonical occupied YAPPPT an ordinary shipped trigger? **No; both shipped map contexts disable occupancy.**
- `[RESOLVED]` Can YAPPPT appear through shipped campaign state despite `TechLevel=-1`? **Yes as an active `all01umd` BasePlan type and as one `sov01umd` preplacement, but neither context permits occupancy.**

There are no OPEN, UNKNOWN, UNCHECKED, or approximate native results in this claimed scope.

## 17. Ghidra annotation candidates

Read-only candidates; none were applied:

- name `Building+0x684` family `Occupants` / `DynamicVectorClass<InfantryClass*>`;
- name `Building+0x69C` `CurrentGarrisonFireIndex`;
- prototype `BuildingClass::SellBuilding(bool queue_hunt_and_remove_team, bool use_inside_fallback)`;
- comment `0x00702603`: inherited Cargo only; not Building occupant vector;
- comment `0x00442635`: fatal `CanBeOccupied` vector release after parent death weapon;
- comment `0x00458060`: cell-center conversion then priority-scoped reverse placement;
- comment `0x004580BD`: Infantry Unlimbo includes `RandomRanged(0,3)` placement for centered coordinate;
- comment `0x00458110`: flag 1 gates Team removal and mission `0x0F`;
- comment `0x00458140`: flag 2 selects null-kill vs inside-foundation no-cell fallback.

## Sources

- live active-retail `/gamemd.exe` in Ghidra project `testProsjekt`, functions/addresses cited inline
- `docs/research/PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md` — corrected for vector/Cargo ownership
- `docs/research/PHASE3_BUILDING_SPAWN_SURVIVORS_CARGO_GHIDRA_REPORT.md`
- `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`
- `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`
- `docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`
- `docs/research/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`
- `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`
- `docs/research/CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`
- `docs/research/GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`
- `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`
- `docs/research/pathfinding/INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`, with its formerly deferred terminal branch closed fresh here
- `docs/research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md`
- `docs/plans/2026-08-29-phase3-building-explodes-lifecycle-design.md`
- mounted retail rules/art/maps paths and hashes in section 1
- current Rust files and lines cited in section 11
