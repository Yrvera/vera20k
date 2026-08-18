# Building Docking System — Bounded Re-investigation

**Program:** Yuri's Revenge `gamemd.exe`  
**Investigation date:** 2026-07-28  
**Mode:** exhaustive-slice  
**Verdict:** **CORRECTED** — the prior report's central claims about `QueueingCell`,
free-unit creation, `ClearBib`, unloading, and forced undocking were contradicted by
the live binary.

## Scope

This report is the canonical result for five previously RED docking claims:

1. `QueueingCell` storage and consumers.
2. `FreeUnit` creation, direction globals, and fallback placement.
3. `BuildingClass::ClearBibArea` ordering and callers.
4. Harvester storage-drain granularity.
5. `BuildingClass::UndockUnit` coordinate framing and track semantics.

The investigation deliberately does not claim a complete model of multi-pad aircraft
docking, every radio message, hospital/armory/bunker behavior, the complete credit and
ore-purifier calculation, or visual animation timing. Those are separate slices.

## Verdict first

| Prior claim | Live-binary result |
|---|---|
| `QueueingCell` is two 16-bit fields | **Wrong.** It is stored as two 32-bit integers at `BuildingTypeClass + 0x1618/+0x161C`; the active consumer reads their low 16-bit words. |
| Refinery exit logic consumes `QueueingCell` | **Wrong.** The only gameplay consumer found in a program-wide instruction sweep is `UnitClass::Mission_Harvest`. |
| `NumberOfDocks` defaults to zero | **Wrong.** The constructor default is one. |
| `ExitObject` creates the refinery's `FreeUnit` | **Wrong.** `BuildingClass::OnConstructionComplete` owns that creation path. |
| `ClearBibArea` retries the same cell eight times | **Wrong.** It checks the base cell once, then visits eight distinct neighboring cells in compass order. |
| A dump pulse removes one bale | **Wrong.** It removes one entire non-empty native storage slot. |
| `0x89F698` is a scalar X offset | **Wrong.** It is one packed `{i16 dx, i16 dy}` direction entry: south `(0,+1)`. |
| Forced undock moves one cell southeast and `0x47` is a facing | **Wrong.** The head point is half a cell southwest; `0x47` is Drive track index 71. |

## Relevant native layout

### `BuildingTypeClass`

| Offset | Native representation | Meaning |
|---:|---|---|
| `+0x0EA0` | pointer | Resolved `FreeUnit` `UnitTypeClass`, or null |
| `+0x1618` | `i32` | Parsed `QueueingCell` X |
| `+0x161C` | `i32` | Parsed `QueueingCell` Y |
| `+0x16BD` | byte/bool | `WeaponsFactory` |
| `+0x1780` | `i32` | `NumberOfDocks` |

`BuildingTypeClass::constructor` at `0x45DD90` initializes both queueing components to
zero and `NumberOfDocks` to one:

- `0x45DFE6`: write dword `0` to `+0x1618`
- `0x45E096`: write dword `0` to `+0x161C`
- `0x45E28A`: write dword `1` to `+0x1780`

### Direction table

The startup initializer at `0x49F2F0` populates packed direction pairs at
`0x89F688..0x89F6A4`:

| Index | Address | Direction | Packed pair |
|---:|---:|---|---|
| 0 | `0x89F688` | N | `(0,-1)` |
| 1 | `0x89F68C` | NE | `(+1,-1)` |
| 2 | `0x89F690` | E | `(+1,0)` |
| 3 | `0x89F694` | SE | `(+1,+1)` |
| 4 | `0x89F698` | S | `(0,+1)` |
| 5 | `0x89F69C` | SW | `(-1,+1)` |
| 6 | `0x89F6A0` | W | `(-1,0)` |
| 7 | `0x89F6A4` | NW | `(-1,-1)` |

The static file image contains zeroes at these addresses because the table is populated
at runtime. The startup table references the initializer at `0x812BAC`.

## 1. `QueueingCell`

### Parsing and representation

`BuildingTypeClass::ReadINI` at `0x45FE50` reads `QueueingCell` through
`CCINIClass::ReadMinMax` at `0x529880`. That helper uses the signed decimal format
`"%d,%d"` and writes two dwords. Missing or empty input retains the supplied `(0,0)`
default.

The resulting dwords are written at:

- `0x461520` → `BuildingTypeClass + 0x1618`
- `0x461526` → `BuildingTypeClass + 0x161C`

`NumberOfDocks` is separately read as an integer at `0x464938`, using the existing
constructor value as its default. The native parser does not clamp it to at least one
or narrow it to eight bits.

### Active consumer

A program-wide sweep of 1,152,096 instructions found:

- `+0x1618`: constructor write, INI write, one unrelated immediate comparison, and
  `UnitClass::Mission_Harvest` at `0x73ED25`.
- `+0x161C`: constructor write, INI write, and `UnitClass::Mission_Harvest` at
  `0x73ED34`.

No building exit or radio routine reads either field.

In the state-2 fallback of `UnitClass::Mission_Harvest` at `0x73E5E0`, the unit:

1. Resolves the refinery after its fallback dock search.
2. Derives the building's north-west foundation cell from world coordinates.
3. Reads the **low word** of `+0x1618` into `DX` and adds it to cell X.
4. Reads the **low word** of `+0x161C` into `AX` and adds it to cell Y.
5. Calls `Find_Nearby_Passable_Cell` at `0x56DC20`.
6. Sets that result as the destination.

The distinction matters: the parser and object layout are two signed 32-bit values, but
this consumer performs 16-bit arithmetic. Negative and out-of-range mod values therefore
have low-word wrapping semantics in this path.

### Role in docking

`QueueingCell` is a harvester fallback/waiting destination. It is not the normal
accepted radio dock cell and is not an exit-cell offset. For the stock refinery
geometry, the normal accepted cell is north-west foundation cell `+(3,1)`, while stock
art supplies `QueueingCell=4,1`.

## 2. Refinery `FreeUnit` creation

### Owner and timing

`FreeUnit` is parsed in `BuildingTypeClass::ReadINI` at `0x460540`; the resolved
`UnitTypeClass` pointer is stored at `BuildingTypeClass + 0xEA0`.

Creation belongs to `BuildingClass::OnConstructionComplete` at `0x445F80`, called from
the completed construction mission path at `0x449AD4`. It is not owned by
`BuildingClass::ExitObject_Main`.

The path is skipped when any relevant gate fails, including:

- no `FreeUnit` type;
- map-editor mode;
- the completion call's suppressing parameter;
- the player/control/build-count eligibility checks.

### Primary coordinate

The verified `BuildingClass` vtable has its complete-object locator at slot `+0x48`,
`BuildingClass::GetCoords` (`0x447AC0`). That function returns:

```text
center.x = location.x + foundation_width  * 128 - 128
center.y = location.y + foundation_height * 128 - 128
center.z = location.z
```

The primary free-unit placement converts that center to a cell and adds direction-table
entry `0x89F698`, south `(0,+1)`. It attempts `Unlimbo` at the resulting cell center
with facing byte `0xC0`.

For a stock 4×3 refinery foundation:

```text
building center cell = north-west + (2,1)
primary free-unit cell = north-west + (2,2)
```

### Fallback and failure

If primary `Unlimbo` fails, the native code makes two
`Find_Nearby_Passable_Cell` attempts seeded from the building location. Both placement
attempts use facing `0xA0`. The calls differ by one still-unnamed boolean option; their
raw post-zone option groups are:

```text
first:  0,1,1,1,1,0,0, scratch,0,0
second: 0,1,1,0,1,0,0, scratch,0,0
```

The helper-option name remains deliberately unspecified. The raw values are sufficient
to prevent an invented semantic label.

After successful placement, the unit queues mission 10 and commences it. If allocation
or all placement attempts fail, the building owner receives a refund equal to the free
unit's build cost; a constructed but unplaceable object is destroyed/uninitialized.

### `ExitObject_Main` is separate

`BuildingClass::ExitObject_Main` at `0x443C60` contains a produced-object branch using
both packed direction globals:

```text
building center cell + SW(-1,+1) + S(0,+1)
```

That is center `+(-1,+2)`, with facing `0xA0`. It neither creates the refinery
construction bonus nor reads `QueueingCell`.

## 3. `BuildingClass::ClearBibArea`

`BuildingClass::ClearBibArea` at `0x449540` is active only when
`BuildingTypeClass + 0x16BD` (`WeaponsFactory`) is true.

Its exact ordering is:

1. Read `ExitList[10]` as a packed `{i16 dx, i16 dy}`.
2. Obtain the building's north-west foundation cell through virtual slot `+0x1B8`,
   verified as `ObjectClass::Get_Cell_Packed` at `0x41BEA0`.
3. Form `base = north_west + ExitList[10]`.
4. Shift `base.x -= 1`.
5. Find the nearest object in `base`, excluding the building itself.
6. If the base has no blocker, return `false` immediately.
7. Scatter the base blocker once.
8. For directions `0..7`, visit one distinct neighboring cell through
   `MapCoord_StepByDir_GetCell` at `0x481810`, using order
   N, NE, E, SE, S, SW, W, NW.
9. Scatter a blocker in each occupied neighbor.
10. Return `true`.

This is one base scatter plus at most one scatter invocation for each of eight distinct
neighbors. It is not eight retries against the base cell.

The early return has a non-obvious consequence: if the base is empty but only a neighbor
is occupied, the routine returns `false` without examining or scattering that neighbor.

### Callers and return handling

- Building deployed-state mission caller at `0x4496B0`: invokes the routine for a
  weapons factory and ignores its return.
- Vehicle-eject mission owner at `0x44D880`: in state 1, a `false` result advances to
  state 2; a `true` result stays in state 1 and retries on a later mission update.

This behavior is active for the stock Allied, Soviet, and Yuri war factories through
`WeaponsFactory=yes`. It is not gated by `Bib=yes`.

## 4. Harvester unload granularity

The unloading state is in `UnitClass::Mission_Deploy_Building` at `0x73D630`.

### Timing gate

In state 3, the routine compares the unit accumulator at `+0xF8` against:

```text
Rules.HarvesterDumpRate * 900.0
```

Exact equality passes. With the binary/default rules value `0.016`, the mathematical
threshold is `14.4`; an integer-like accumulator first satisfies it at 15.

### Drain operation

At an eligible crossing:

1. `FindFirstNonEmptySlot` (`0x6C9820`) scans native storage slots `0..3`.
2. The first slot whose float amount is greater than zero is selected.
3. `GetAmount` (`0x6C9680`) returns that slot's complete amount.
4. `RemoveAmount` (`0x6C96B0`) is called with that exact amount.
5. The removed amount is credited and the accumulator resets to zero.
6. State remains 3 for another crossing.

Therefore each productive crossing drains **one complete resource slot**, not one bale
or one fixed-size fragment. Mixed cargo drains in ascending native slot order over
separate crossings.

If no slot remains, or removal is non-positive, the crossing performs no credit and
does not reset the accumulator. It optionally starts `ProductionAnim`, changes the
mission state to 4, clears `SpecialAnim`, and returns. State 4 is processed on a later
mission call.

## 5. Forced undocking

### Callers

`BuildingClass::UndockUnit` at `0x4593A0` has three verified callers:

- `BuildingClass::ReceiveDamage` at `0x442230`
- `BuildingClass::Sell` at `0x449C30`
- `TemporalClass::Update` at `0x71A760`

It is an interrupt-ejection path, not the stock refinery's ordinary unload-completion
path.

### Gates and command sequence

If the reciprocal object link at `BuildingClass + 0x2E4` is null, the function is a
no-op. It also returns without clearing the link if the linked object's virtual kind
check at slot `+0x2C` does not return the Drive-compatible value `1`.

For a valid linked Drive unit, it:

1. Asserts that the active locomotor at unit `+0x674` is non-null.
2. Calls locomotor virtual slot `+0x58` (`Power_On`).
3. Gets the building center coordinate.
4. Calls locomotor virtual slot `+0x70` with track `0x47` and head coordinate:

   ```text
   (center.x - 128, center.y + 128, center.z)
   ```

5. Calls the unit virtual at `+0x544` with `1.0`.
6. Clears the unit's reciprocal `+0x2E4` link, then the building's link.
7. Sends radio argument `3` through building virtual slot `+0x274`,
   verified as `RadioClass::Transmit_Radio_ToFirst`.

Using the engine's `+X=east`, `+Y=south` frame, the head coordinate is half a cell west
and half a cell south: **half-cell southwest**. It is neither a full cell nor southeast.

### Meaning of `0x47`

The Drive locomotor vtable was identified through its Complete Object Locator:

- vtable `0x7E7EB0`
- COL `0x7FFDE8`, subobject offset 4
- TypeDescriptor `0x820248`: `.?AVDriveLocomotionClass@@`
- slot `+0x70` → `DriveLocomotionClass::Force_Track` at `0x4B0C40`

Thus `0x47` is track index 71, not a body-facing byte. A cold raw read of
`TurnTrack[71]` at `0x7E7E7C` returned:

```text
0f 0f 00 00 c0 00 00 00 00 00 00 00
```

This selects `RawTrack 15`, target facing `0xC0`, with zero flags. `Force_Track` records
the track and head/destination coordinate; it does not assign body facing `0x47`.

## Stock INI surface

Verified stock Yuri's Revenge values include:

- `GAREFN`: `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `FreeUnit=CMIN`
- `NAREFN`: `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `FreeUnit=HARV`
- `NADEPT`: `NumberOfDocks=1`
- `artmd.ini` `GAREFN` and `NAREFN`: `QueueingCell=4,1`
- No stock `Weeder=` assignment was found in `rules.ini` or `rulesmd.ini`
- No stock merged-INI `HarvesterDumpRate` assignment was found; the native/default
  value is `0.016`

The native storage container has four indexed slots. Stock ore-miner behavior exercises
the ordinary ore/gem subset; this report does not promote the TS `Weeder` branch to
active stock YR behavior.

## Integration and state ownership

| Mechanism | Native owner/timing | Normal or exceptional |
|---|---|---|
| Queueing fallback | `UnitClass::Mission_Harvest`, state 2 | Normal fallback |
| Free refinery unit | `BuildingClass::OnConstructionComplete` | Once per eligible completion |
| Clear producer exit | Building deployed/eject mission paths | Situational congestion |
| Storage drain | `UnitClass::Mission_Deploy_Building`, state 3 | Normal unload loop |
| Forced track 71 | Damage, sale, or temporal undock interrupt | Exceptional |

The busy-refinery redirect established in this slice is the
`UnitClass::Mission_Harvest` fallback. This report does not claim a separate
building-radio redirect without a verified caller path.

## Rust parity status

### Correct or stock-equivalent

- `NumberOfDocks` defaults to one for ordinary stock definitions.
- Stock `QueueingCell=4,1` is parsed and used as a waiting/fallback coordinate.
- The current miner dock sequence drains a complete Ore slot, then a complete Gem slot,
  with a later empty crossing before departure. That matches the stock-visible core
  granularity and terminal-pulse shape.
- Normal stock departure does not force track `0x47`.
- The interrupt path uses track index `0x47`.

### Divergences

| Severity and trigger | Current Rust mismatch | Primary location |
|---|---|---|
| **High; every eligible refinery completion** | Free unit is spawned when the building is placed rather than on construction completion. For a 4×3 refinery it uses north-west `+(2,3)` instead of native `+(2,2)`. Its radius fallback and no-refund failure behavior also differ. | `src/sim/production/production_refinery.rs` |
| **High when a weapons-factory exit strip is occupied** | No native-equivalent `ClearBibArea` call chain exists. The generic scatter helper has no caller, so a blocked exit is held rather than cleared with the native base-then-neighbors behavior. | `src/sim/production/production_spawn.rs`, `src/sim/movement/scatter.rs` |
| **Medium/rare; only linked-unit sale/damage/temporal interrupts** | Rust's forced-undock head offset is `(0,+256)` rather than `(-128,+128)`. The helper is wired to sale, but matching damage and temporal call paths were not found. | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/production/production_sell.rs` |
| **Mod-only** | Rust parses queue components as `u16`, rejecting negative values instead of accepting signed dwords whose low words are consumed. | `src/rules/art_data.rs`, `src/rules/object_type.rs` |
| **Mod-only** | Rust clamps `NumberOfDocks` to at least one and narrows it to `u8`; native storage and parsing are unclamped `i32`. | `src/rules/object_type.rs` |
| **Legacy/mod cargo** | Rust models two cargo categories, while the native container scans four storage slots. | `src/sim/miner/miner_dock_sequence.rs` |

## Adversarial cases

1. **Negative or large `QueueingCell`:** native parsing accepts signed dwords, while
   `Mission_Harvest` consumes low words with 16-bit arithmetic. Rust's unsigned parser
   does not reproduce this mod behavior.
2. **Empty `ClearBib` base with occupied neighbor:** native returns `false` immediately,
   does not scan the neighbor, and the eject state advances. A broad 3×3 scatter would
   be observably wrong.
3. **Free-unit placement failure:** native tries the primary cell, then two nearby-cell
   passes, and refunds the type's build cost if creation cannot be completed. Logging
   and silently losing the bonus is not equivalent.
4. **Exact unload threshold and mixed cargo:** equality passes; only the first ascending
   non-empty slot drains; the later empty crossing terminates without credit or timer
   reset.
5. **Invalid forced-undock link:** null or non-Drive links cause a no-op and remain
   uncleared. A valid link is cleared only after the locomotor/unit commands.

## Coverage ledger

| Target | Evidence acquired | Result |
|---|---|---|
| `BuildingTypeClass::constructor` `0x45DD90` | decompile + assembly | Field widths and defaults verified |
| `BuildingTypeClass::ReadINI` `0x45FE50` | decompile + assembly + strings | Queueing, docks, and free-unit parsing verified |
| `CCINIClass::ReadMinMax` `0x529880` | decompile | Signed pair parsing verified |
| Program-wide `+0x1618/+0x161C` references | 1,152,096-instruction sweep, repeated cold | Sole gameplay consumer verified |
| `UnitClass::Mission_Harvest` `0x73E5E0` | decompile + assembly | Queue fallback behavior verified |
| Direction initializer `0x49F2F0` | decompile + startup xref | Packed direction table verified |
| `BuildingClass::OnConstructionComplete` `0x445F80` | decompile + callers | Free-unit owner, placement, refund verified |
| `BuildingClass::GetCoords` `0x447AC0` | decompile + vtable/COL | Center formula verified |
| `BuildingClass::ExitObject_Main` `0x443C60` | decompile + field exclusion | Separate produced-object path verified |
| `BuildingClass::ClearBibArea` `0x449540` | decompile + assembly + callers | Base/neighbor order and return use verified |
| `MapCoord_StepByDir_GetCell` `0x481810` | decompile | Neighbor order verified |
| `UnitClass::Mission_Deploy_Building` `0x73D630` | decompile + assembly | Gate, whole-slot drain, terminal crossing verified |
| Storage helpers `0x6C9820/0x6C9680/0x6C96B0` | decompile | Slot selection and exact amount verified |
| `BuildingClass::UndockUnit` `0x4593A0` | decompile + assembly + callers | Gates, coordinate, ordering verified |
| Drive vtable and COL | raw memory + RTTI walk | `Force_Track` slot identity verified |
| `TurnTrack[71]` `0x7E7E7C` | independent cold raw read | RawTrack/facing/flags verified |
| Stock INI and art | direct text search | Active values verified |
| Rust implementation | direct source inspection | Matches and divergences recorded |

## Open-question log

All questions were seeded before the Ghidra pass.

| ID | Question | Final status |
|---|---|---|
| Q01 | Exact `QueueingCell` layout? | **Resolved:** two dwords; low words consumed. |
| Q02 | Which parser and signedness? | **Resolved:** `ReadMinMax`, signed `%d,%d`. |
| Q03 | All active readers? | **Resolved:** only `Mission_Harvest` gameplay reads found. |
| Q04 | Exact harvest state usage? | **Resolved:** state-2 nearby-passable fallback. |
| Q05 | Is it the accepted dock cell? | **Resolved:** no; accepted cell is distinct. |
| Q06 | Who owns free-unit creation? | **Resolved:** `OnConstructionComplete`. |
| Q07 | What are `0x89F698/0x89F69C`? | **Resolved:** packed S and SW pairs. |
| Q08 | How is `FreeUnit` parsed and used in stock? | **Resolved.** |
| Q09 | What is creation ordering? | **Resolved:** completion callback, primary then two fallbacks. |
| Q10 | How is the `ClearBib` base formed? | **Resolved:** NW + ExitList[10], then X−1. |
| Q11 | What is neighbor order? | **Resolved:** N through NW, clockwise table order. |
| Q12 | Who calls `ClearBib`, and how is return used? | **Resolved.** |
| Q13 | What is the dump threshold? | **Resolved:** accumulator ≥ rate×900. |
| Q14 | What amount is removed? | **Resolved:** the complete first non-empty slot. |
| Q15 | How does unloading terminate? | **Resolved:** a later empty/non-positive crossing enters state 4. |
| Q16 | Who calls forced undock? | **Resolved:** damage, sale, temporal update. |
| Q17 | What coordinate and track are forced? | **Resolved:** center `(−128,+128)`, track 71. |
| Q18 | When are reciprocal links cleared? | **Resolved:** after valid Drive commands, unit then building. |
| Q19 | Native `NumberOfDocks` default and range? | **Resolved:** default 1, parsed/stored `i32`, no observed clamp. |
| Q20 | Which behavior is stock YR versus legacy? | **Resolved for this slice:** no stock `Weeder=` assignment. |
| Q21 | Rust queueing parity? | **Resolved:** stock value works; signed/mod edges differ. |
| Q22 | Rust free-unit parity? | **Resolved:** timing, coordinate, fallback, and refund differ. |
| Q23 | Rust `ClearBib` parity? | **Resolved:** active producer-exit behavior is missing. |
| Q24 | Rust unload parity? | **Resolved:** stock core matches; native has four slots. |
| Q25 | Rust forced-undock parity? | **Resolved:** track matches; offset and caller coverage differ. |
| Q26 | Which mission/tick owners run each path? | **Resolved for all five mechanisms.** |
| Q27 | Free-unit failure edges? | **Resolved:** gated skip, two fallbacks, refund/destruction. |
| Q28 | Negative queue values? | **Resolved mechanically:** signed parse plus low-word use. |
| Q29 | Unload equality, mixed, and empty edges? | **Resolved.** |
| Q30 | `ClearBib` empty-base edge? | **Resolved:** early false, no neighbor scan. |
| Q31 | Destroyed/sold/interrupted docking? | **Resolved for forced-undock callers; broader lifecycle is out of scope.** |
| Q32 | Exact save/load serialization of these transient states? | **Deferred:** outside this bounded contradiction slice. |

## Implementation handoff

### 1. Move and correct refinery `FreeUnit` creation

- **Required behavior:** create on the construction-complete callback; for stock 4×3
  use north-west `+(2,2)` for the primary cell; preserve `0xC0` primary and `0xA0`
  fallback facings; reproduce the two nearby-passable attempts and refund failure.
- **Rust touchpoint:** `src/sim/production/production_refinery.rs` and the building
  construction-completion owner.
- **Acceptance:** a normal completed Allied/Soviet refinery creates its bonus miner once
  at the native cell; blocked primary exercises ordered fallback; total failure refunds.
- **Risk:** moving timing can expose assumptions in placement/build-up ordering. Keep the
  change owned by the completion transition rather than adding a second spawn hook.

### 2. Add native producer-exit clearing

- **Required behavior:** only for `WeaponsFactory`; derive the exact base, require a base
  blocker before neighbor scans, scatter base then N/NE/E/SE/S/SW/W/NW, and preserve
  the caller's boolean retry semantics.
- **Rust touchpoint:** `src/sim/production/production_spawn.rs` and the existing scatter
  service in `src/sim/movement/scatter.rs`.
- **Acceptance:** empty base plus occupied neighbor does not scatter; occupied base
  scatters it and occupied neighbors once each; the eject state retries only after a
  `true` result.
- **Risk:** do not substitute an unconditional 3×3 scatter.

### 3. Correct and connect forced undock

- **Required behavior:** force track `0x47` toward building-center
  `(-128,+128)` only for a valid reciprocal Drive link; add equivalent damage and
  temporal interruption ownership where those systems exist.
- **Rust touchpoint:** `src/sim/miner/miner_dock_sequence.rs`,
  `src/sim/production/production_sell.rs`, damage/temporal integration.
- **Acceptance:** valid linked interruptions get the half-cell-southwest head point and
  clear both links after commands; invalid links remain untouched.
- **Risk:** this is not normal refinery departure and must not be called from its ordinary
  unload completion.

### 4. Preserve lower-priority mod semantics

- Consider a signed/raw representation for `QueueingCell` if mod parity is in scope.
- Preserve native `i32` `NumberOfDocks` parsing before deciding how unsafe values should
  be represented internally.
- Generalize cargo slots only if TS/extended resource behavior becomes an active target.

No unload-granularity fix is warranted for the stock Ore/Gem loop: the current whole-slot
model is the verified mechanism.

## Remaining uncertainty

- The semantic name of the single differing `Find_Nearby_Passable_Cell` boolean in the
  two free-unit fallback calls remains unknown. The exact raw arguments are recorded;
  assigning a friendly name requires a focused helper-contract investigation.
- Save/load serialization and restoration of transient docking states were not audited.
- This slice proves the three `UndockUnit` callers but does not model every surrounding
  damage or temporal state transition.

## Primary evidence

### Live `gamemd.exe`

- `BuildingTypeClass::constructor` — `0x45DD90`
- `BuildingTypeClass::ReadINI` — `0x45FE50`
- `CCINIClass::ReadMinMax` — `0x529880`
- `UnitClass::Mission_Harvest` — `0x73E5E0`
- Direction-table initializer — `0x49F2F0`
- `BuildingClass::GetCoords` — `0x447AC0`
- `BuildingClass::OnConstructionComplete` — `0x445F80`
- `BuildingClass::ExitObject_Main` — `0x443C60`
- `BuildingClass::ClearBibArea` — `0x449540`
- `MapCoord_StepByDir_GetCell` — `0x481810`
- `UnitClass::Mission_Deploy_Building` — `0x73D630`
- `FindFirstNonEmptySlot` — `0x6C9820`
- `GetAmount` — `0x6C9680`
- `RemoveAmount` — `0x6C96B0`
- `BuildingClass::UndockUnit` — `0x4593A0`
- `DriveLocomotionClass::Force_Track` — `0x4B0C40`

### Project evidence

- Stock Yuri's Revenge `rules.ini`, `rulesmd.ini`, and `artmd.ini` data
- `src/rules/art_data.rs`
- `src/rules/object_type.rs`
- `src/rules/ruleset.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/production/production_refinery.rs`
- `src/sim/production/production_spawn.rs`
- `src/sim/production/production_sell.rs`
- `src/sim/movement/scatter.rs`
