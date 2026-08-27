# Phase 3 active-retail crate runtime re-investigation

**Status:** COMPLETE for the active-retail crate mechanism: slot lifecycle,
pickup transaction, all stock-reachable effects, trigger and object ingress,
movement callers, presentation order, and installed-retail reachability.

**Confidence:** HIGH. Every active-stock branch below is grounded in live
decompilation/disassembly plus the installed `rulesmd.ini` and all 184 named
retail maps. The only remaining label uncertainty is the semantic naming of
some Unit-effect side/BaseUnit subvectors; their executed predicates and order
are fully observed and are an implementation contract, not an open behavior.

**Verdict:** the existing Rust scenario-start crate scatter is materially wrong
and the runtime mechanism is otherwise absent. The row remains **OPEN** until a
builder replaces the divergent behavior, all focused validation passes, and
fresh implementation critics return zero findings.

**Player impact:** high and ordinary. Retail skirmish enables `Crates=yes` by
default, stock random selection reaches eight live effects, and installed
skirmish maps additionally use action 108 and `CrateBeneath` ingress.
`CarriesCrate` exists in stock types but is disabled by every installed map's
scenario flags. Wrong slot, RNG, Mark, pickup, or effect order shifts every
later Scenario RNG consumer.

**Scope:** active `gamemd.exe`, stock `rulesmd.ini`, 184 named installed retail
maps, current Rust, and evidence-backed exclusions. Unsafe allocator failure,
native memory corruption, and unreachable malformed zero-weight data are not
deliberately emulated.

## Executive corrections

The re-investigation overturns these stale hypotheses:

- Initial count uses connected human/session nodes only, not human plus AI.
  Current Rust's human-only count is correct, but its unsigned rule parsing is
  not.
- A crate placement accepted by the pre-validator owns a slot and timer even
  when Overlay allocation, construction, Unlimbo, or Mark fails. Such a
  placement is a **ghost**, and it stops the random retry loop.
- Water random placement is a timed ghost in stock rules: FNPC uses Float, but
  crate Mark runs the ground/passability path and fails. Rust's visible water
  crate assertion is wrong.
- Placement does not directly stamp `OverlayGrid`; it goes through native
  Overlay construction and Mark.
- Radius effects iterate the live Ground display layer in exact 3D and do not
  filter to the picker owner.
- HealBase iterates owner-matching Logic objects; it is not an explicit
  building-only sweep.
- `[Powerups]` field three is the over-water eligibility byte, not a generic
  sound flag.
- `[CrateRules] FreeMCV` is parsed and retained, but the live pickup override
  reads the multiplayer session `Bases` option instead. Treating `FreeMCV` as
  the live gate is stale.
- `CrateBeneath` and trigger action 108 do not honor the global Crates option.
  `CarriesCrate` alone checks it, in addition to `TruckCrate`/`TrainCrate`.
- CrateTrigger has two distinct events: synchronous collector AttachedTag event
  49 and a separate next-Logic-rung global event-50 latch.
- Pickup's boolean return controls its locomotor caller; it is not a consumed
  flag. Unit-crate success and an event-49 collector death return zero, while
  most consumed outcomes return one.

## Retail data authority

Stock `[CrateRules]` is:

```ini
CrateMaximum=255
CrateMinimum=1
CrateRadius=3.0
CrateRegen=3
SilverCrate=HealBase
SoloCrateMoney=5000
UnitCrateType=none
WoodCrate=Money
WaterCrate=Money
HealCrateSound=HealCrate
WoodCrateImg=CRATE
CrateImg=CRATE
WaterCrateImg=WCRATE
FreeMCV=yes
```

Canonical powerup indices are fixed independently of INI declaration order:

| Index | Name | Retail weight | Retail anim | Over water | Retail data |
|---:|---|---:|---|---|---:|
| 0 | Money | 20 | MONEY | yes | 2000 |
| 1 | Unit | 20 | none | no | 0 |
| 2 | HealBase | 10 | HEALALL | yes | 0 |
| 3 | Cloak | 0 | CLOAK | yes | 0 |
| 4 | Explosion | 0 | none | yes | 500 |
| 5 | Napalm | 0 | none | no | 600 |
| 6 | Squad | 0 | none | no | 0 |
| 7 | Darkness | 0 | SHROUDX | yes | 0 |
| 8 | Reveal | 10 | REVEAL | yes | 0 |
| 9 | Armor | 10 | ARMOR | yes | 1.5 |
| 10 | Speed | 10 | SPEED | yes | 1.2 |
| 11 | Firepower | 10 | FIREPOWR | yes | 2.0 |
| 12 | ICBM | 0 | CHEMISLE | yes | 0 |
| 13 | Invulnerability | 0 | ARMOR | yes | 1.0 |
| 14 | Veteran | 20 | VETERAN | yes | 1 |
| 15 | IonStorm | 0 | none | yes | 0 |
| 16 | Gas | 0 | none | yes | 100 |
| 17 | Tiberium | 0 | none | no | 0 |
| 18 | Pod | 0 | none | no | 0 |

The active weighted total is 110. The reachable random stock set is exactly
Money, Unit, HealBase, Reveal, Armor, Speed, Firepower, and Veteran.

`RulesClass__ReadPowerups @ 0x00673E80` owns the canonical table regardless of
INI declaration order. It parses weight through signed `atoi`, animation by
name (`<none>` or an unknown name becomes `-1`), field three as a
case-insensitive yes/no water byte, and data through `atof`; a token containing
`%` is additionally multiplied by binary64 `0.01`. Missing whole keys preserve
the initialized slot, missing or unknown water tokens preserve the prior byte,
and extra tokens are ignored. The native arrays are weights `0x0081DA8C`,
animation indices `0x0081DAD8`, data doubles `0x0089EC28`, and water bytes
`0x0089ECC0`.

`RulesClass__ReadCrateRules @ 0x0066B900` parses signed minimum, maximum, and
solo money; double regen; cell-range-scaled radius; the three image identities;
the three solo fixed-type mappings; heal sound; `UnitCrateType`; and
`FreeMCV`. Missing values preserve prior state. An unknown supplied fixed-type
name resolves to Money. `FreeMCV` is real parsed state, but the live
multiplayer pickup override does not read it: `0x00481B99..0x00481BFF` reads
the session `Bases` option byte at `0x00A8B258`.

The installed-map census found:

- zero CRATE/WCRATE OverlayPack identities in all 184 named maps;
- 13 ordinary-skirmish action-108 calls: two in `xarena.map` and eleven in
  `xxmas.map`, all selecting positive-weight stock effects;
- zero ordinary-skirmish event-49 or event-50 conditions;
- zero ordinary-skirmish `TeamType Tag=` rows; all 144 retail occurrences are
  confined to eleven campaign maps;
- 72 `CrateBeneath` structure instances across 23 installed maps, of which 58
  across sixteen maps are in the strict ordinary-skirmish subset;
- four ordinary-map `TRUCKB CarriesCrate=yes` instances, but both scenario
  flags default false and every explicit installed `TruckCrate`/`TrainCrate`
  value is `no`, so none can produce a crate in retail data.

These facts exclude weight-zero handlers, CrateTrigger actions, and
`CarriesCrate` drops from the ordinary stock-skirmish closure, but not their
native parsing or synthetic regression coverage. Action 108 and
`CrateBeneath` remain active and required.

## Persistent slot lifecycle

`MapClass` owns `0x1000` inline bytes at `Map+0x158`: 256 ordered 16-byte
slots.

| Offset | Type | Meaning |
|---:|---|---|
| `+0x00` | `i32` | placement start frame |
| `+0x04` | `u32` | persisted timer auxiliary word |
| `+0x08` | `i32` | duration frames |
| `+0x0C` | packed `i16 x, i16 y` | cell; `(0,0)` means empty |

`MapClass__constructor @ 0x00565090` zeroes coordinates.
`MapClass::Init_Clear @ 0x005659F0` establishes the authoritative fresh/reset
tuple `{start=-1, aux=0, duration=0, x=0, y=0}` for every slot. Coordinate is
the sole occupied/empty discriminator.

`MouseClass__Save @ 0x005BE6D0` raw-writes the singleton body containing every
slot word. `MouseClass__Load @ 0x005BDF70` restores them verbatim, including
ghosts and paused timers. No post-load timer normalization was found.

The quick retail sync checksum at `0x0064DAB0` does not directly fold the Map
slot table. Rust snapshots must serialize it, and Rust's broader future-state
hash must include it, but `compute_retail_multiplayer_checksum` must not add it.

## Bootstrap and signed count

`ScenarioClass__Post_Map_Init @ 0x00686890` runs bootstrap whenever the global
Crates option is enabled; it has no game-mode gate. The signed count is:

```text
min(CrateMaximum, max(CrateMinimum, pregame_human_node_count))
```

Negative and inverted mod values are not pre-clamped. Each requested iteration
calls random placement exactly once; a failed call is not topped up.

`FUN_005E7460 @ 0x005E7460` copies the connected player-node count at
`0x005E7471..0x005E748B`. AI seats live in a separate global and are consumed
separately by House/start construction. A one-human/seven-AI skirmish therefore
requests one crate before `CrateMinimum`, not eight.

## Random and specific-cell placement

### Random placement

`MapClass__PlaceCrateAtRandomCell @ 0x0056BD40`:

1. Finds the first empty slot in ascending order. A full table returns false
   without RNG.
2. Retains that slot through at most 1,000 attempts.
3. Each attempt draws Scenario RNG X then Y within the active Map rectangle:
   `left + RandomRanged(0,width-1)`, then the analogous Y expression.
4. Water origins call FNPC with Float speed type 5; all others use Track 1.
5. The accepted destination runs the hard validator. A hard rejection retries
   and spends no timer RNG.
6. A precheck acceptance stops retries even if later Overlay/Mark work fails.
7. Acceptance consumes `RandomRanged(0,0x7ffffffe)` for the timer.

The verified FNPC tuple is Normal movement zone, no required zone, no bridge-
aware zone check, 1x1 footprint, no overlay/height/current-occupant/final
occupancy check, bridges allowed, zero target sentinel, and radius
`min(SizeW+SizeH,32)`. Preferred-candidate selection uses the current frame
modulo and consumes no Scenario RNG.

### Specific-cell placement

The helper at `0x0056BEC0` snaps the supplied origin once before scanning for
the first empty slot. Invalid snap or a full table returns false. It never
overwrites a live slot and does not deduplicate coordinates.

The full dword data parameter matters:

- exactly `0x14` performs no post-write: a visible crate keeps `0xff`, and a
  ghost preserves prior cell data;
- every other value writes its low byte after accepted placement, even for a
  ghost; `0x114` therefore writes `0x14` and is not the sentinel.

Pickup treats unsigned data below 19 as a fixed type. Data 19 and above uses
weighted random selection.

### Validator, Mark, and ghost acceptance

`CrateSlot__PlaceOverlayAndInitTimer @ 0x004A17C0` calls
`CrateSlot__ValidateCellAndCreateOverlay @ 0x004A18F0`.

Hard rejection is limited to:

- `MapClass__Is_Cell_In_Playfield(cell,1)` false; or
- a pre-existing overlay identity (`Cell+0x44 != -1`).

After those checks, the validator chooses `WaterCrateImg` or `WoodCrateImg`,
allocates an OverlayClass, and invokes its constructor. Terrain occupation can
skip Unlimbo. `OverlayClass::Mark @ 0x005FC570` can reject slope, crate
passability, override, or occupation. The outer validator propagates none of
allocation, construction, Unlimbo, or Mark failure; it reports accepted,
claims the slot, stores the timer, and dirties presentation.

Therefore an allocation, terrain, slope, passability, occupation, Unlimbo, or
Mark failure is an accepted timed ghost. Only the two hard prechecks leave the
slot empty. A visible Mark writes data `0xff`; a ghost preserves the cell byte.

Native additionally registers a failed Overlay object/UniqueID in some Mark-
failure cases, but not in Logic, and the slot system never reads that identity.
This object-graph artifact is excluded from crate gameplay state; ghost slot,
timer, RNG, and cell-byte behavior are required.

## Exact timer and regeneration

On accepted placement:

```text
lower = CrateRegen * 450.0
upper = CrateRegen * 1800.0
draw = Scenario.RandomRanged(0, 0x7ffffffe)
value = lower + draw / 2147483646.0 * (upper - lower)
duration = x87 truncate-toward-zero(value)
start = current pre-increment frame
aux = high dword of upper's stored double
```

Retail `CrateRegen=3` yields 1350 at draw zero, 5400 at the inclusive maximum,
and aux `0x40B51800`. The direction of interpolation is load-bearing even
though reversing it preserves the distribution.

`MapClass__UpdateCrateRegenTimers @ 0x0056BBE0` runs only in nonzero game mode
with Crates enabled and scans all 256 slots ascending. Empty slots skip.
`start==-1` expires only when duration is zero. Otherwise signed/wrapping
`current-start >= duration` expires. Expiration clears the slot and calls
random placement once. First-free reinsertion can land at or before the
current index (not revisited) or above it (visited later in the same scan), so
modded zero/negative timers can cascade.

The sole caller is `LogicClass__PerTickUpdate @ 0x0055AFB0`. Order is live
objects, `FUN_0053D310`, AlphaShape purge, crate regeneration, Tactical, then
Factory and House arrays. The Rust insertion point is immediately before the
existing first factory sweep, after live-object/combat/effect work, using the
pre-increment absolute `binary_frame`.

## Clear and removal

`CrateSlot__ClearAndPreserveTimer @ 0x004A1750` returns false without mutation
for an empty coordinate. Otherwise it attempts overlay removal, clears the
coordinate regardless, preserves remaining duration using signed/wrapping
elapsed arithmetic, then sets start to `-1`. A pre-existing `start==-1`
preserves duration.

The overlay helper at `0x004A1AA0` removes only an identity exactly equal to
the current Rules `CrateImg`, `WoodCrateImg`, or `WaterCrateImg`. A missing or
mismatched overlay can survive while the slot becomes free.

`MapClass__RemoveCrateAtCell @ 0x0056C020`:

- mode zero removes any live `Crate=yes` overlay directly and uses no slots;
- nonzero mode clears only the first ascending occupied slot with the packed
  coordinate, visible or ghost.

Pickup ignores removal failure. In nonzero mode with Crates enabled, it calls
one immediate random replacement before the selected effect.

## Pickup transaction

`CrateClass__PickupDispatch @ 0x00481A00` receives `ECX=CellClass*` and one
collector argument. Exact prefix:

1. null collector, no overlay, non-crate overlay, or nonzero-mode passive
   owner returns one;
2. if `CrateTrigger=yes` and collector AttachedTag is non-null, synchronously
   call `TagClass__ProcessTriggerEvent @ 0x006E53A0` with event 49;
3. ignore callback return and re-read collector native-alive;
4. a dead collector returns zero immediately, leaving crate/latch/RNG intact;
5. otherwise set `Scenario+0x34BE=1`, even with no AttachedTag;
6. select, apply MP guards/remaps, remove, immediately replace, then dispatch
   the effect.

The latch is independent. At the next leading Logic rung,
`LogicClass__PerTickUpdate` walks global Tags in order with event 50, then
clears it. Ordinary stock maps contain no event-49/50 conditions, so the
latch remains future-affecting state only for custom/campaign data. The
ordinary stock implementation may evidence-exclude trigger actions, but must
not invent a different RNG/removal boundary.

## Selection and guard order

Data below 19 selects directly with no selection RNG. Otherwise native sums
the nineteen signed weights, draws inclusive `RandomRanged(1,total)`, and
chooses the first cumulative sum `>= roll`.

Mode zero bypasses MP guards. Only signed data zero enters the image override:
`CrateImg -> SilverCrate`, then `WoodCrateImg -> WoodCrate`, then
`WaterCrateImg -> WaterCrate`. Since retail CrateImg and WoodCrateImg both name
CRATE, Wood/Money overwrites Silver/HealBase. Mode-zero Money is fixed
`SoloCrateMoney` and spends no amount draw.

Nonzero-mode order is:

1. resolve side-compatible BaseUnit;
2. FreeMCV overrides to Unit when OwnedBuildings is zero, available funds are
   strictly above 1500, no side BaseUnit is owned, and the session `Bases`
   option is enabled; parsed `[CrateRules] FreeMCV` is not read here;
3. anti-stack remaps: Unit above 50 units, Squad above 100 infantry, existing
   Cloak/Armor/Speed/Firepower modifier, Aircraft Speed, non-firepower Techno,
   non-trainable/already-elite Veteran, and Unit/Squad on Water or Beach;
4. Water land plus the selected powerup's over-water byte false remaps Money;
5. mode-four bookkeeping;
6. remove and immediate replacement;
7. Squad remaps to Money;
8. effect dispatch.

## Eight active-retail effects

All presentation follows mutation. The common tail at `0x004832F5` resolves
the animation from the final selected/remapped type and, when it is not `-1`,
allocates an Anim at crate-center ground Z plus 200 with constructor arguments
`(0,1,0x600,0,0)`. Allocation failure is silent. Unit placement success is the
only active-stock outcome that skips this common tail.

### Shared radius contract

Armor, Speed, Firepower, Veteran, and the stock-disabled Cloak handler iterate
the live Ground display-layer buffer in its current order. They do not iterate
the entity store and do not owner-filter: enemy candidates in range are
modified. Distance is exact 3D from crate cell-center coordinates at computed
ground Z to candidate virtual coordinates:

```text
Math__ftol(Sqrt_Approx(dx^2 + dy^2 + dz^2)) < Rules.CrateRadius
```

The boundary is strict, so stock distance 768 is rejected. Armor accepts
Techno with exact multiplier 1.0; Speed accepts Foot with exact multiplier 1.0
and excludes Aircraft; Firepower accepts Techno with exact multiplier 1.0 and
does not repeat the picker's firepower-capability test; Veteran requires the
marked/active byte, Techno lineage, a trainable type, and positive data.

### Money — `0x00482463`

Multiplayer converts data with `Math__ftol`, consumes one inclusive Scenario
RNG draw `[base, base+900]`, then calls `HouseClass__Add_Credits @ 0x004F9950`.
Stock adds 2000..2900. Addition is wrapping signed i32, with no cap or
saturation. The mode-zero fixed-money path uses `SoloCrateMoney` and consumes
no amount draw. Credits mutate first; picker-owner-human gates spatial
`CrateMoneySound`; MONEY animation follows.

### Unit — `0x00482041`

`UnitCrateType`, when non-null, overrides selection. Otherwise native repeats
`RandomRanged(0, UnitTypeCount-1)` until a type has `CrateGoodie=yes` and passes
BaseUnit eligibility; every rejected candidate still consumes its draw and
there is no retry cap. A BaseUnit is rejected when session Bases is off. With
Bases on, it is admitted only for a human-controlled house or the free-MCV
override. The chosen Unit constructor receives the picker owner.

Native first attempts Unlimbo at crate-center ground coordinates. On failure
it runs Foot FNPC with the chosen unit movement type and makes exactly one
nearby Unlimbo attempt. Success plays `CrateUnitSound` only for a human owner,
returns zero, and skips the common animation. Type/allocation failure consumes
the crate and reaches the common Unit tail (stock animation none). If both
placements fail, native destroys the candidate, remaps to Money, executes its
amount RNG/credit/sound, then creates the MONEY animation.

### HealBase — `0x00482B8F`

Picker-owner-human plays `HealCrateSound` before healing. Native then walks the
live Logic vector in exact order. Each non-null candidate whose owner equals
the picker owner receives virtual damage
`candidate.Health - candidate.Type.Strength`, distance zero, Rules warhead
`+0xFA8`, and flags `(0,1,1)`. Negative damage heals and the receiver clamps to
Strength; zero still calls the receiver. The IsTechno test is on the picker,
not each candidate, so this is not an explicit building-only sweep. HEALALL
animation follows the sweep.

### Reveal — `0x00481F9D`

`MapClass__BlackoutShroud @ 0x00577D90` runs first with the picker owner. It
marks that House revealed, but performs full-map cell mutation only when that
House is the local `g_PlayerPtr`, preserving native per-peer ownership.
`CrateRevealSound` then plays without a picker-human gate; REVEAL animation is
last.

### Armor — `0x00482D56`

Each shared-radius Armor candidate multiplies its native binary64 field by the
parsed 1.5. If any affected candidate owner satisfies
`HouseClass__IsHumanPlayer`, native emits `EVA_UnitArmorUpgraded` after the
loop. Independently, picker-owner-human gates spatial `CrateArmourSound`.
ARMOR animation follows. Order is mutation loop, EVA, spatial sound,
animation.

### Speed — `0x00482F36`

Each eligible non-Aircraft Foot multiplies native `Foot+0x580` by parsed 1.2.
If any affected owner has the native PlayerControl byte, native emits
`EVA_UnitSpeedUpgraded` after the loop. Picker-owner-human then gates
`CrateSpeedSound`; SPEED animation follows. The multiplier is persistent
binary64 state consumed directly by `FootClass__GetCurrentSpeed`; Rust's
locomotor `SimFixed` speed is not an equivalent owner.

### Firepower — `0x00483125`

Each eligible Techno multiplies native `Techno+0x160` by parsed 2.0. If any
affected owner satisfies `IsHumanPlayer`, native emits
`EVA_UnitFirePowerUpgraded` after the loop. Picker-owner-human then gates
`CrateFireSound`; FIREPOWR animation follows. The persistent binary64 value
feeds damage and weapon timing; radius candidates do not undergo the picker's
capability-vslot gate.

### Veteran — `0x00482972`

For each eligible candidate, native loops integer `i=0` while
`(double)i < data`. Each iteration applies the ordered native helpers:
Veteran-to-Elite, standard-Rookie-to-Veteran, then negative-Rookie reset.
Stock data 1 advances exactly one tier. No EVA is emitted.
Picker-owner-human gates `CratePromoteSound` after the loop; VETERAN animation
follows.

RNG order is selection, replacement X/Y attempts, successful replacement
timer, then effect-specific draws. Predetermined content omits selection.

## Active ingress

### Trigger action 108

`TriggerAction__Execute @ 0x006DD8B0`, case `0x6C` at
`0x006DF69B..0x006DF6BE`, resolves waypoint `TAction+0x44`, passes the full
dword `+0x90` to specific placement, has no mode/Crates gate, and returns the
helper boolean including true for a ghost.

Retail `xarena.map` uses Event 1 on two tagged `CATECH01` structures to place
fixed HealBase and Reveal crates. Retail `xxmas.map` uses unconditional Event
8 and eleven action-108 calls selecting Money, Speed, Armor, Veteran, and
Firepower. Event 8 is unconditionally true in
`TriggerCondition__Evaluate @ 0x0071E940`. Event 1 is object-raised,
owner-filtered, writes the triggering owner, forces the evaluator persistence
byte, and is intentionally non-latching.

Both map tags use repeat mode zero and are one-shot. Repeat ownership is
`[Tags]` field zero, not `[Triggers]` field seven. `[Triggers]` field three is
the native disabled byte: stock zero means enabled. Current Rust reverses that
field, which disables these retail triggers before its already-missing Event
and Action implementations can run.

Current Rust polls only a small trigger subset, lacks Events 1/8 and action
108, and does not retain map-object tag columns. Exact retail ingress therefore
requires object tags plus an object-raised Event-1 seam, and Event-8/action-108
support in the existing pre-object trigger rung.

### Building `CrateBeneath`

`BuildingClass__Place_OccupyMap @ 0x00441F60`, block
`0x004421A4..0x00442221`, invokes specific placement at the building center
when `BuildingType+0x1767 CrateBeneath`. `CrateBeneathIsMoney` passes zero;
otherwise it passes `0x14`. There is no Crates or mode gate.

### Unit `CarriesCrate`

`UnitClass__ReceiveDamage @ 0x00737C90`, death block
`0x0073838A..0x00738452`, requires `CarriesCrate`, global Crates, and the
scenario `TruckCrate` flag for non-trains or `TrainCrate` for trains. Both map
flags default false. It performs an outer FNPC/invalid-cell check, then the
specific helper snaps again, passing exactly `0x14`.

Only stock `TRUCKB` has `CarriesCrate=yes`. Four ordinary maps instantiate it,
but every explicit installed `TruckCrate`/`TrainCrate` value is `no`; neither
producer gate is active in retail data. This ingress is required parsing and
synthetic regression surface, but evidence-excluded from ordinary-stock
runtime closure.

## Complete pickup caller closure

Thirteen calls in eleven bodies are live:

| Family | Function/callsites | Native continuation |
|---|---|---|
| Hover | `0x00514F70` / `0x5153E9` | zero, alive, limbo, and status byte choose stop/status-7 versus continue |
| Jumpjet | `0x005B17B0` / `0x5B1894` | zero or limbo clears destination if alive; otherwise continues destination install |
| Jumpjet descend | `0x0054C550` / `0x54C9F6` | ignores return/liveness, then writes descent completion fields |
| Drive ForceTrack | `0x004B0C40` / `0x4B0D1B` | same return/alive/limbo contract as Ship ForceTrack |
| Drive track | `0x004B0F20` / `0x4B1DBE` | zero/limbo clears only if alive; success advances track |
| Drive move | `0x004B2630` / `0x4B405D`, `0x4B46E6` | candidate and final-stage branches re-read alive/limbo |
| Ship ForceTrack | `0x006A0310` / `0x6A03EB` | one+unlimbo applies requested XYZ; otherwise alive clears, dead retains |
| Ship track | `0x006A05F0` / `0x6A1401` | zero/limbo alive clears; success advances; dead retains |
| Ship move | `0x006A1C80` / `0x6A3689`, `0x6A3D15` | candidate/final branches re-read alive/limbo |
| Teleport | arrival / `0x71972E` | ignores return/liveness, then continues warp cleanup |
| Walk | `0x0075C240` / `0x75C56C` | zero+unlimbo alive clears; dead returns; otherwise sets destination |

The shared Rust transaction must release entity borrows, retain a stable ID,
and re-fetch after every callback/effect. A deferred queue cannot preserve
same-stack continuation.

Ship ForceTrack is load-bearing for SQD. `ParasiteClass::Attach @ 0x0062A980`
snapshots victim XYZ, calls Ship ForceTrack selector `-1`, ignores its result,
then writes victim backlink followed by manager victim even if pickup killed or
uninitialized the limboed SQD. Rust must not sanitize that deferred-lifetime
state.

## Return-value matrix

- Prefix rejection/no crate: one.
- Event-49 collector death before selection: zero, crate intact.
- Unit exact or nearby placement success: zero for human and AI; human only
  adds sound; common animation is skipped.
- Unit type/create-null: common Unit animation tail, one.
- Unit created but both placements fail: destroy candidate, remap to Money,
  execute Money, common tail, one.
- Every other consumed effect, even Explosion after collector death: common
  tail, one.

## Rust divergence

Current `src/sim/crates.rs`:

- owns a discarded local `[bool;256]` rather than persistent slots;
- parses unsigned/clamped min/max and lacks regen/full rules/Powerups;
- draws within the wrong abstraction instead of exact active Map bounds;
- directly stamps overlays and asserts that occupation/Mark is bypassed;
- expects visible water crates instead of ghost acceptance;
- discards timer state after spending its draw;
- has no specific placement, clear, pickup, replacement, regeneration,
  effects, ingress, caller barrier, save, or hash ownership.

Current object and overlay rules omit `CrateGoodie`, `CarriesCrate`,
`CrateBeneath`, `CrateBeneathIsMoney`, and `CrateTrigger`. Current scenario
data omits `TruckCrate`/`TrainCrate`. Current trigger runtime lacks the active
retail action/event/tag ingress. `GameEntity` has Armor multiplier only; stock
crate Firepower and Foot Speed need independent persistent native-bit fields
and consumer integration.

Current trigger parsing also interprets `[Triggers]` field three as an enabled
bit rather than native disabled, and derives repetition from Trigger field
seven instead of `[Tags]` field zero. Both must be corrected as part of the
active retail action-108 ingress.

## Evidence-backed exclusions

- No installed ordinary map authors a CRATE/WCRATE OverlayPack identity.
  Nonzero-mode loader filtering plus independent OverlayData decoding must be
  preserved, but authored overlay-to-slot bootstrap is excluded because native
  deliberately does not do it.
- No ordinary stock map defines events 49/50 or TeamType tags. The native latch
  state and ordering are verified, but full campaign/custom trigger action
  behavior is outside ordinary stock-skirmish implementation closure.
- `CarriesCrate` is parsed and its native death gate is verified, but every
  installed retail `TruckCrate`/`TrainCrate` flag is false. No ordinary stock
  unit-death crate is active.
- Weight-zero handlers are not selected randomly and no ordinary stock action
  108 supplies them. They remain parsed/indexed and testable but are excluded
  from the active ordinary effect implementation.
- Failed Overlay allocation's orphan object-array identity and UniqueID are not
  read by crate gameplay or the quick checksum. Ghost state and RNG are
  required; exact native orphan graph identity is not.
- Native allocator OOM, zero-total malformed Powerups, invalid pointer graph,
  and memory corruption are invalid-domain behavior.

## Required builder validation

The implementation design carries the exhaustive executable matrix. At
minimum it must cover exact slot defaults/serde/hash; signed count; no-RNG full
table; hard rejection versus accepted ghost; land/water Mark behavior; timer
goldens; clear/pause/wrapping; ascending regen reinsertion; predetermined and
weighted selection boundaries; replacement/effect RNG order; all strict MP
guard thresholds; eight active effects; Unit success/fallback; action 108;
CrateBeneath; CarriesCrate; Event-1/Event-8 retail ingress; all caller
continuations; SQD ForceTrack; and the scheduler rung immediately before
factories.

## Primary evidence

- Active retail `gamemd.exe`, image base `0x00400000`, live Ghidra
  decompilation/disassembly/xrefs at every address cited above.
- Installed retail `rulesmd.ini` extracted from the active VFS.
- 184 named retail maps extracted from the active installation and scanned by
  decoded INI/OverlayPack content.
- Current Rust `src/sim/crates.rs`, `src/rules/ruleset.rs`,
  `src/sim/trigger_runtime.rs`, `src/map/entities.rs`, movement families,
  `src/sim/game_entity.rs`, snapshot, and world-hash owners.

No Ghidra metadata was modified during this investigation.
